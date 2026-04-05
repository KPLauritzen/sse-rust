use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sse_core::matrix::SqMatrix;
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{SearchConfig, SearchTelemetry, SseResult};

#[derive(Debug)]
struct Cli {
    cases_path: PathBuf,
    format: OutputFormat,
    worker_case: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum OutputFormat {
    Pretty,
    Json,
}

#[derive(Clone, Debug, Deserialize)]
struct CaseCorpus {
    schema_version: u32,
    cases: Vec<ResearchCase>,
}

#[derive(Clone, Debug, Deserialize)]
struct ResearchCase {
    id: String,
    description: String,
    a: [[u32; 2]; 2],
    b: [[u32; 2]; 2],
    config: JsonSearchConfig,
    timeout_ms: u64,
    allowed_outcomes: Vec<String>,
    target_outcome: Option<String>,
    points: OutcomePoints,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct JsonSearchConfig {
    max_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
}

#[derive(Clone, Debug, Deserialize)]
struct OutcomePoints {
    equivalent: i64,
    not_equivalent: i64,
    unknown: i64,
    timeout: i64,
    panic: i64,
}

#[derive(Debug, Serialize, Deserialize)]
struct WorkerCaseResult {
    id: String,
    actual_outcome: String,
    elapsed_ms: u128,
    steps: Option<usize>,
    reason: Option<String>,
    telemetry: SearchTelemetry,
}

#[derive(Debug, Serialize)]
struct HarnessSummary {
    schema_version: u32,
    cases_path: String,
    fitness: FitnessSummary,
    cases: Vec<CaseSummary>,
}

#[derive(Debug, Serialize)]
struct FitnessSummary {
    required_cases: usize,
    passed_required_cases: usize,
    target_hits: usize,
    total_points: i64,
    total_elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
struct CaseSummary {
    id: String,
    description: String,
    actual_outcome: String,
    allowed_outcomes: Vec<String>,
    target_outcome: Option<String>,
    passed: bool,
    hit_target: bool,
    points: i64,
    elapsed_ms: u128,
    timeout_ms: u64,
    steps: Option<usize>,
    reason: Option<String>,
    telemetry: SearchTelemetry,
    tags: Vec<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(exit_code) => exit_code,
        Err(err) => {
            eprintln!("research_harness error: {err}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    let cli = parse_cli(env::args().skip(1))?;
    let corpus = load_corpus(&cli.cases_path)?;

    if let Some(case_id) = cli.worker_case.as_deref() {
        let case = corpus
            .cases
            .iter()
            .find(|case| case.id == case_id)
            .ok_or_else(|| format!("unknown worker case id: {case_id}"))?;
        let result = run_case(case);
        println!(
            "{}",
            serde_json::to_string(&result)
                .map_err(|err| format!("failed to serialise worker result: {err}"))?
        );
        return Ok(ExitCode::SUCCESS);
    }

    let summary = run_harness(&cli.cases_path, &corpus)?;
    match cli.format {
        OutputFormat::Pretty => print!("{}", format_pretty_summary(&summary)),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string_pretty(&summary)
                .map_err(|err| format!("failed to serialise summary: {err}"))?
        ),
    }

    if summary.fitness.required_cases == summary.fitness.passed_required_cases {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::from(1))
    }
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut cases_path = PathBuf::from("research/cases.json");
    let mut format = OutputFormat::Pretty;
    let mut worker_case = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--cases" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--cases requires a path".to_string())?;
                cases_path = PathBuf::from(value);
            }
            "--format" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--format requires json or pretty".to_string())?;
                format = match value.as_str() {
                    "json" => OutputFormat::Json,
                    "pretty" => OutputFormat::Pretty,
                    _ => return Err(format!("unsupported format: {value}")),
                };
            }
            "--worker-case" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--worker-case requires a case id".to_string())?;
                worker_case = Some(value);
            }
            "--help" | "-h" => {
                return Err(
                    "usage: research_harness [--cases research/cases.json] [--format pretty|json] [--worker-case CASE_ID]"
                        .to_string(),
                );
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Cli {
        cases_path,
        format,
        worker_case,
    })
}

fn load_corpus(path: &Path) -> Result<CaseCorpus, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

fn run_case(case: &ResearchCase) -> WorkerCaseResult {
    let a = SqMatrix::new(case.a);
    let b = SqMatrix::new(case.b);
    let config = SearchConfig {
        max_lag: case.config.max_lag,
        max_intermediate_dim: case.config.max_intermediate_dim,
        max_entry: case.config.max_entry,
    };

    let started = Instant::now();
    let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
    let elapsed_ms = started.elapsed().as_millis();

    match result {
        SseResult::Equivalent(path) => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "equivalent".to_string(),
            elapsed_ms,
            steps: Some(path.steps.len()),
            reason: None,
            telemetry,
        },
        SseResult::NotEquivalent(reason) => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "not_equivalent".to_string(),
            elapsed_ms,
            steps: None,
            reason: Some(reason),
            telemetry,
        },
        SseResult::Unknown => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "unknown".to_string(),
            elapsed_ms,
            steps: None,
            reason: None,
            telemetry,
        },
    }
}

fn run_harness(cases_path: &Path, corpus: &CaseCorpus) -> Result<HarnessSummary, String> {
    let current_exe =
        env::current_exe().map_err(|err| format!("failed to resolve current executable: {err}"))?;

    let mut cases = Vec::with_capacity(corpus.cases.len());
    let mut passed_required_cases = 0usize;
    let mut target_hits = 0usize;
    let mut total_points = 0i64;
    let mut total_elapsed_ms = 0u128;

    for case in &corpus.cases {
        let executed = run_case_in_subprocess(&current_exe, cases_path, case)?;
        let passed = case
            .allowed_outcomes
            .iter()
            .any(|allowed| allowed == &executed.actual_outcome);
        let hit_target = case
            .target_outcome
            .as_ref()
            .is_some_and(|target| target == &executed.actual_outcome);
        let points = case.points.for_outcome(&executed.actual_outcome);

        if passed {
            passed_required_cases += 1;
        }
        if hit_target {
            target_hits += 1;
        }

        total_points += points;
        total_elapsed_ms += executed.elapsed_ms;

        cases.push(CaseSummary {
            id: case.id.clone(),
            description: case.description.clone(),
            actual_outcome: executed.actual_outcome,
            allowed_outcomes: case.allowed_outcomes.clone(),
            target_outcome: case.target_outcome.clone(),
            passed,
            hit_target,
            points,
            elapsed_ms: executed.elapsed_ms,
            timeout_ms: case.timeout_ms,
            steps: executed.steps,
            reason: executed.reason,
            telemetry: executed.telemetry,
            tags: case.tags.clone(),
        });
    }

    Ok(HarnessSummary {
        schema_version: corpus.schema_version,
        cases_path: cases_path.display().to_string(),
        fitness: FitnessSummary {
            required_cases: corpus.cases.len(),
            passed_required_cases,
            target_hits,
            total_points,
            total_elapsed_ms,
        },
        cases,
    })
}

fn run_case_in_subprocess(
    current_exe: &Path,
    cases_path: &Path,
    case: &ResearchCase,
) -> Result<WorkerCaseResult, String> {
    let mut child = Command::new(current_exe)
        .arg("--cases")
        .arg(cases_path)
        .arg("--worker-case")
        .arg(&case.id)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to spawn worker for {}: {err}", case.id))?;

    let started = Instant::now();
    let timeout = Duration::from_millis(case.timeout_ms);

    loop {
        if started.elapsed() > timeout {
            child
                .kill()
                .map_err(|err| format!("failed to kill timed out worker {}: {err}", case.id))?;
            let output = child
                .wait_with_output()
                .map_err(|err| format!("failed to collect timed out worker {}: {err}", case.id))?;
            let reason = stderr_snippet(&output.stderr);
            return Ok(WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "timeout".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: reason.or_else(|| Some(format!("worker exceeded {} ms", case.timeout_ms))),
                telemetry: SearchTelemetry::default(),
            });
        }

        match child
            .try_wait()
            .map_err(|err| format!("failed to poll worker {}: {err}", case.id))?
        {
            Some(status) => {
                let output = child.wait_with_output().map_err(|err| {
                    format!("failed to collect finished worker {}: {err}", case.id)
                })?;

                if !status.success() {
                    return Ok(WorkerCaseResult {
                        id: case.id.clone(),
                        actual_outcome: "panic".to_string(),
                        elapsed_ms: started.elapsed().as_millis(),
                        steps: None,
                        reason: stderr_snippet(&output.stderr)
                            .or_else(|| Some(format!("worker exited with status {status}"))),
                        telemetry: SearchTelemetry::default(),
                    });
                }

                let stdout = String::from_utf8(output.stdout)
                    .map_err(|err| format!("worker {} produced non-utf8 stdout: {err}", case.id))?;
                let parsed: WorkerCaseResult = serde_json::from_str(stdout.trim())
                    .map_err(|err| format!("worker {} produced invalid json: {err}", case.id))?;
                return Ok(parsed);
            }
            None => thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn stderr_snippet(stderr: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stderr);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.lines().take(6).collect::<Vec<_>>().join(" | "))
    }
}

impl OutcomePoints {
    fn for_outcome(&self, outcome: &str) -> i64 {
        match outcome {
            "equivalent" => self.equivalent,
            "not_equivalent" => self.not_equivalent,
            "unknown" => self.unknown,
            "timeout" => self.timeout,
            "panic" => self.panic,
            _ => self.panic,
        }
    }
}

fn format_pretty_summary(summary: &HarnessSummary) -> String {
    let mut out = String::new();
    out.push_str("Research Harness\n");
    out.push_str(&format!("cases: {}\n", summary.cases_path));
    out.push_str(&format!(
        "required_passes: {}/{}\n",
        summary.fitness.passed_required_cases, summary.fitness.required_cases
    ));
    out.push_str(&format!("target_hits: {}\n", summary.fitness.target_hits));
    out.push_str(&format!("total_points: {}\n", summary.fitness.total_points));
    out.push_str(&format!(
        "total_elapsed_ms: {}\n\n",
        summary.fitness.total_elapsed_ms
    ));

    for case in &summary.cases {
        out.push_str(&format!(
            "- {}: outcome={} passed={} target={} points={} elapsed={}ms\n",
            case.id,
            case.actual_outcome,
            case.passed,
            case.hit_target,
            case.points,
            case.elapsed_ms
        ));
        if let Some(reason) = &case.reason {
            out.push_str(&format!("  reason: {}\n", reason));
        }
        out.push_str(&format!(
            "  telemetry: layers={} expanded={} factorisations={} kept={} pruned_size={} pruned_spectrum={} discovered={} seen_collisions={} max_frontier={} visited={}\n",
            case.telemetry.layers.len(),
            case.telemetry.frontier_nodes_expanded,
            case.telemetry.factorisations_enumerated,
            case.telemetry.candidates_after_pruning,
            case.telemetry.pruned_by_size,
            case.telemetry.pruned_by_spectrum,
            case.telemetry.discovered_nodes,
            case.telemetry.collisions_with_seen,
            case.telemetry.max_frontier_size,
            case.telemetry.total_visited_nodes,
        ));
    }

    out
}
