use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sse_core::matrix::DynMatrix;
use sse_core::matrix::SqMatrix;
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{SearchConfig, SearchTelemetry, SsePath, SseResult};

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
    beam_width: Option<usize>,
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
    telemetry_focus_cases: usize,
    telemetry_focus_score: u64,
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
    telemetry_summary: DerivedTelemetrySummary,
    tags: Vec<String>,
}

#[derive(Debug, Serialize)]
struct DerivedTelemetrySummary {
    productive_layers: usize,
    deepest_productive_layer: Option<usize>,
    first_stagnant_layer: Option<usize>,
    exhausted_before_max_lag: bool,
    terminal_bottleneck: String,
    avg_factorisations_per_expanded_node: f64,
    avg_survivor_ratio: f64,
    avg_discovery_ratio: f64,
    last_layer_frontier_nodes: usize,
    last_layer_candidates_after_pruning: usize,
    last_layer_discovered_nodes: usize,
    last_layer_next_frontier_nodes: usize,
    focus_progress_score: u64,
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
        beam_width: case.config.beam_width,
    };

    let started = Instant::now();
    let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
    let elapsed_ms = started.elapsed().as_millis();

    match result {
        SseResult::Equivalent(path) => match validate_sse_path(&a, &b, &path) {
            Ok(()) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "equivalent".to_string(),
                elapsed_ms,
                steps: Some(path.steps.len()),
                reason: None,
                telemetry,
            },
            Err(reason) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "panic".to_string(),
                elapsed_ms,
                steps: Some(path.steps.len()),
                reason: Some(format!("invalid equivalent path: {reason}")),
                telemetry,
            },
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

fn validate_sse_path(a: &SqMatrix<2>, b: &SqMatrix<2>, path: &SsePath<2>) -> Result<(), String> {
    if path.steps.is_empty() {
        if path.matrices.len() != 1 {
            return Err(format!(
                "empty-step path should contain exactly one 2x2 matrix, got {}",
                path.matrices.len()
            ));
        }
        if path.matrices[0] != *a || path.matrices[0] != *b {
            return Err("empty-step path does not match the input matrices".to_string());
        }
        return Ok(());
    }

    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let first = &path.steps[0];
    let first_uv = first.u.mul(&first.v);
    if first_uv != a_dyn {
        return Err("first step does not start at A".to_string());
    }

    let last = path.steps.last().expect("non-empty path has a last step");
    let last_vu = last.v.mul(&last.u);
    if last_vu != b_dyn {
        return Err("last step does not end at B".to_string());
    }

    for (idx, window) in path.steps.windows(2).enumerate() {
        let left = window[0].v.mul(&window[0].u);
        let right = window[1].u.mul(&window[1].v);
        if left != right {
            return Err(format!(
                "step chain breaks between indices {} and {}",
                idx,
                idx + 1
            ));
        }
    }

    if let Some(first_matrix) = path.matrices.first() {
        if *first_matrix != *a {
            return Err("path.matrices does not start at A".to_string());
        }
    }
    if let Some(last_matrix) = path.matrices.last() {
        if *last_matrix != *b {
            return Err("path.matrices does not end at B".to_string());
        }
    }

    Ok(())
}

fn run_harness(cases_path: &Path, corpus: &CaseCorpus) -> Result<HarnessSummary, String> {
    let current_exe =
        env::current_exe().map_err(|err| format!("failed to resolve current executable: {err}"))?;

    let mut cases = Vec::with_capacity(corpus.cases.len());
    let mut passed_required_cases = 0usize;
    let mut target_hits = 0usize;
    let mut total_points = 0i64;
    let mut total_elapsed_ms = 0u128;
    let mut telemetry_focus_cases = 0usize;
    let mut telemetry_focus_score = 0u64;

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
        let telemetry_summary = derive_telemetry_summary(
            &executed.actual_outcome,
            &executed.telemetry,
            case.config.max_lag,
        );
        if case.tags.iter().any(|tag| tag == "telemetry-focus") {
            telemetry_focus_cases += 1;
            telemetry_focus_score += telemetry_summary.focus_progress_score;
        }

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
            telemetry_summary,
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
            telemetry_focus_cases,
            telemetry_focus_score,
        },
        cases,
    })
}

fn derive_telemetry_summary(
    actual_outcome: &str,
    telemetry: &SearchTelemetry,
    configured_max_lag: usize,
) -> DerivedTelemetrySummary {
    if telemetry.invariant_filtered {
        return DerivedTelemetrySummary {
            productive_layers: 0,
            deepest_productive_layer: None,
            first_stagnant_layer: None,
            exhausted_before_max_lag: false,
            terminal_bottleneck: "invariant_filter".to_string(),
            avg_factorisations_per_expanded_node: 0.0,
            avg_survivor_ratio: 0.0,
            avg_discovery_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
        };
    }

    if telemetry.permutation_shortcut || telemetry.canonical_shortcut {
        return DerivedTelemetrySummary {
            productive_layers: 0,
            deepest_productive_layer: None,
            first_stagnant_layer: None,
            exhausted_before_max_lag: false,
            terminal_bottleneck: "shortcut".to_string(),
            avg_factorisations_per_expanded_node: 0.0,
            avg_survivor_ratio: 0.0,
            avg_discovery_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
        };
    }

    if telemetry.layers.is_empty() {
        return DerivedTelemetrySummary {
            productive_layers: 0,
            deepest_productive_layer: None,
            first_stagnant_layer: None,
            exhausted_before_max_lag: false,
            terminal_bottleneck: "no_search".to_string(),
            avg_factorisations_per_expanded_node: 0.0,
            avg_survivor_ratio: 0.0,
            avg_discovery_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
        };
    }

    let productive_layers_vec: Vec<&_> = telemetry
        .layers
        .iter()
        .filter(|layer| layer.discovered_nodes > 0 || layer.collisions_with_other_frontier > 0)
        .collect();
    let productive_layers = productive_layers_vec.len();
    let deepest_productive_layer = productive_layers_vec.iter().map(|layer| layer.layer_index).max();
    let first_stagnant_layer = telemetry
        .layers
        .iter()
        .find(|layer| {
            layer.frontier_nodes > 0
                && layer.discovered_nodes == 0
                && layer.collisions_with_other_frontier == 0
                && layer.next_frontier_nodes == 0
        })
        .map(|layer| layer.layer_index);

    let expanded_nodes = telemetry.frontier_nodes_expanded.max(1) as f64;
    let avg_factorisations_per_expanded_node =
        telemetry.factorisations_enumerated as f64 / expanded_nodes;
    let avg_survivor_ratio = if telemetry.factorisations_enumerated == 0 {
        0.0
    } else {
        telemetry.candidates_after_pruning as f64 / telemetry.factorisations_enumerated as f64
    };
    let avg_discovery_ratio = if telemetry.candidates_after_pruning == 0 {
        0.0
    } else {
        telemetry.discovered_nodes as f64 / telemetry.candidates_after_pruning as f64
    };

    let last_layer = telemetry
        .layers
        .last()
        .expect("non-empty layers already checked");
    let exhausted_before_max_lag =
        actual_outcome == "unknown" && last_layer.next_frontier_nodes == 0 && telemetry.layers.len() < configured_max_lag;

    let terminal_bottleneck = classify_bottleneck(actual_outcome, telemetry, last_layer);
    let focus_progress_score = compute_focus_progress_score(telemetry, productive_layers, deepest_productive_layer);

    DerivedTelemetrySummary {
        productive_layers,
        deepest_productive_layer,
        first_stagnant_layer,
        exhausted_before_max_lag,
        terminal_bottleneck,
        avg_factorisations_per_expanded_node,
        avg_survivor_ratio,
        avg_discovery_ratio,
        last_layer_frontier_nodes: last_layer.frontier_nodes,
        last_layer_candidates_after_pruning: last_layer.candidates_after_pruning,
        last_layer_discovered_nodes: last_layer.discovered_nodes,
        last_layer_next_frontier_nodes: last_layer.next_frontier_nodes,
        focus_progress_score,
    }
}

fn classify_bottleneck(
    actual_outcome: &str,
    telemetry: &SearchTelemetry,
    last_layer: &sse_core::types::SearchLayerTelemetry,
) -> String {
    if actual_outcome == "equivalent" {
        return "solved".to_string();
    }
    if actual_outcome == "timeout" {
        return "timeout".to_string();
    }
    if actual_outcome == "panic" {
        return "panic".to_string();
    }
    if last_layer.frontier_nodes >= 16
        && last_layer.candidates_after_pruning <= 1
        && last_layer.discovered_nodes == 0
        && last_layer.next_frontier_nodes == 0
    {
        return "state_space_collapse".to_string();
    }
    if telemetry.collisions_with_seen > telemetry.discovered_nodes
        && telemetry.collisions_with_seen >= 16
    {
        return "duplicate_dominated".to_string();
    }
    if telemetry.candidates_generated > 0
        && telemetry.pruned_by_spectrum.saturating_mul(5) >= telemetry.candidates_generated.saturating_mul(4)
    {
        return "spectral_pruning_dominated".to_string();
    }
    if telemetry.max_frontier_size >= 1000 {
        return "frontier_growth".to_string();
    }
    if telemetry.factorisations_enumerated >= 10_000 {
        return "factorisation_volume".to_string();
    }
    "mixed".to_string()
}

fn compute_focus_progress_score(
    telemetry: &SearchTelemetry,
    productive_layers: usize,
    deepest_productive_layer: Option<usize>,
) -> u64 {
    let depth = deepest_productive_layer.unwrap_or(0) as u64;
    let productive = productive_layers as u64;
    let meets = telemetry.collisions_with_other_frontier as u64;
    let visited = telemetry.total_visited_nodes.min(50_000) as u64;
    depth * 1_000_000 + productive * 100_000 + meets * 10_000 + visited
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
        "telemetry_focus_score: {} across {} case(s)\n",
        summary.fitness.telemetry_focus_score, summary.fitness.telemetry_focus_cases
    ));
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
        out.push_str(&format!(
            "  telemetry_summary: bottleneck={} productive_layers={} deepest_productive_layer={:?} stagnant_layer={:?} exhausted_early={} focus_progress_score={}\n",
            case.telemetry_summary.terminal_bottleneck,
            case.telemetry_summary.productive_layers,
            case.telemetry_summary.deepest_productive_layer,
            case.telemetry_summary.first_stagnant_layer,
            case.telemetry_summary.exhausted_before_max_lag,
            case.telemetry_summary.focus_progress_score,
        ));
        if case.tags.iter().any(|tag| tag == "telemetry-focus") {
            for layer in &case.telemetry.layers {
                out.push_str(&format!(
                    "    layer {} {:?}: frontier={} factors={} kept={} discovered={} next={} meet={} seen_collisions={}\n",
                    layer.layer_index,
                    layer.direction,
                    layer.frontier_nodes,
                    layer.factorisations_enumerated,
                    layer.candidates_after_pruning,
                    layer.discovered_nodes,
                    layer.next_frontier_nodes,
                    layer.collisions_with_other_frontier,
                    layer.collisions_with_seen,
                ));
            }
        }
    }

    out
}
