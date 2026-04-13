use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sse_core::matrix::DynMatrix;
use sse_core::matrix::SqMatrix;
use sse_core::search::{search_sse_2x2_with_telemetry, search_sse_with_telemetry_dyn};
use sse_core::types::{
    DynSsePath, DynSseResult, SearchConfig, SearchMode, SearchTelemetry, SsePath, SseResult,
};

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
    a: Vec<Vec<u32>>,
    b: Vec<Vec<u32>>,
    config: JsonSearchConfig,
    timeout_ms: u64,
    allowed_outcomes: Vec<String>,
    target_outcome: Option<String>,
    points: OutcomePoints,
    #[serde(default)]
    tags: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JsonSearchConfig {
    max_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    #[serde(default = "default_search_mode")]
    search_mode: SearchMode,
}

fn default_search_mode() -> SearchMode {
    SearchMode::Mixed
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
    result_model: ResultModel,
    telemetry: SearchTelemetry,
}

#[derive(Debug, Serialize)]
struct HarnessSummary {
    schema_version: u32,
    cases_path: String,
    fitness: FitnessSummary,
    comparisons: Vec<ComparisonSummary>,
    cases: Vec<CaseSummary>,
}

#[derive(Debug, Serialize)]
struct FitnessSummary {
    required_cases: usize,
    passed_required_cases: usize,
    target_hits: usize,
    total_points: i64,
    total_elapsed_ms: u128,
    generalized_cases: usize,
    comparison_groups: usize,
    telemetry_focus_cases: usize,
    telemetry_focus_score: u64,
    telemetry_focus_reach_score: u64,
    telemetry_focus_directed_score: i64,
}

#[derive(Debug, Serialize)]
struct CaseSummary {
    id: String,
    description: String,
    endpoint: EndpointSummary,
    config: JsonSearchConfig,
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
    result_model: ResultModel,
    telemetry: SearchTelemetry,
    telemetry_summary: DerivedTelemetrySummary,
    tags: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct EndpointSummary {
    source_dim: usize,
    target_dim: usize,
    a: Vec<Vec<u32>>,
    b: Vec<Vec<u32>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum HarnessSolverPath {
    TwoByTwo,
    SquareEndpoint,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum ResultResolutionKind {
    Identity,
    PermutationShortcut,
    CanonicalShortcut,
    FrontierPath,
    ConcreteShiftWitness,
    InvariantFilterNotEquivalent,
    SearchNotEquivalent,
    SearchExhausted,
    Timeout,
    Panic,
    InvalidPath,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResultModel {
    solver_path: HarnessSolverPath,
    source_dim: usize,
    target_dim: usize,
    resolution_kind: ResultResolutionKind,
    witness_lag: Option<usize>,
    path_matrix_count: Option<usize>,
    frontier_layers: usize,
}

#[derive(Debug, Serialize)]
struct ComparisonSummary {
    endpoint: EndpointSummary,
    variants: Vec<ComparisonVariantSummary>,
}

#[derive(Debug, Serialize)]
struct ComparisonVariantSummary {
    case_id: String,
    description: String,
    config: JsonSearchConfig,
    actual_outcome: String,
    result_model: ResultModel,
    passed: bool,
    hit_target: bool,
    points: i64,
    elapsed_ms: u128,
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
    avg_dead_end_ratio: f64,
    last_layer_frontier_nodes: usize,
    last_layer_candidates_after_pruning: usize,
    last_layer_discovered_nodes: usize,
    last_layer_next_frontier_nodes: usize,
    focus_progress_score: u64,
    directed_progress_score: i64,
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
    let a = case_matrix(&case.a).expect("invalid matrix A in corpus");
    let b = case_matrix(&case.b).expect("invalid matrix B in corpus");
    let config = SearchConfig {
        max_lag: case.config.max_lag,
        max_intermediate_dim: case.config.max_intermediate_dim,
        max_entry: case.config.max_entry,
        search_mode: case.config.search_mode,
    };

    let started = Instant::now();
    if a.rows == 2 && b.rows == 2 {
        let a_sq = a
            .to_sq::<2>()
            .expect("matrix A should be 2x2 when using the 2x2 solver path");
        let b_sq = b
            .to_sq::<2>()
            .expect("matrix B should be 2x2 when using the 2x2 solver path");
        let (result, telemetry) = search_sse_2x2_with_telemetry(&a_sq, &b_sq, &config);
        match result {
            SseResult::Equivalent(path) => match validate_sse_path_2x2(&a_sq, &b_sq, &path) {
                Ok(()) => WorkerCaseResult {
                    id: case.id.clone(),
                    actual_outcome: "equivalent".to_string(),
                    elapsed_ms: started.elapsed().as_millis(),
                    steps: Some(path.steps.len()),
                    reason: None,
                    result_model: equivalent_result_model(
                        HarnessSolverPath::TwoByTwo,
                        a.rows,
                        b.rows,
                        &telemetry,
                        path.steps.len(),
                        path.matrices.len(),
                    ),
                    telemetry,
                },
                Err(reason) => WorkerCaseResult {
                    id: case.id.clone(),
                    actual_outcome: "panic".to_string(),
                    elapsed_ms: started.elapsed().as_millis(),
                    steps: Some(path.steps.len()),
                    reason: Some(format!("invalid equivalent path: {reason}")),
                    result_model: result_model(
                        HarnessSolverPath::TwoByTwo,
                        a.rows,
                        b.rows,
                        ResultResolutionKind::InvalidPath,
                        Some(path.steps.len()),
                        Some(path.matrices.len()),
                        &telemetry,
                    ),
                    telemetry,
                },
            },
            SseResult::EquivalentByConcreteShift(witness) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "equivalent".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: Some("aligned concrete-shift witness".to_string()),
                result_model: result_model(
                    HarnessSolverPath::TwoByTwo,
                    a.rows,
                    b.rows,
                    ResultResolutionKind::ConcreteShiftWitness,
                    Some(witness.shift.lag as usize),
                    None,
                    &telemetry,
                ),
                telemetry,
            },
            SseResult::NotEquivalent(reason) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "not_equivalent".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: Some(reason),
                result_model: not_equivalent_result_model(
                    HarnessSolverPath::TwoByTwo,
                    a.rows,
                    b.rows,
                    &telemetry,
                ),
                telemetry,
            },
            SseResult::Unknown => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "unknown".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: None,
                result_model: result_model(
                    HarnessSolverPath::TwoByTwo,
                    a.rows,
                    b.rows,
                    ResultResolutionKind::SearchExhausted,
                    None,
                    None,
                    &telemetry,
                ),
                telemetry,
            },
        }
    } else {
        let (result, telemetry) = search_sse_with_telemetry_dyn(&a, &b, &config);
        match result {
            DynSseResult::Equivalent(path) => match validate_sse_path_dyn(&a, &b, &path) {
                Ok(()) => WorkerCaseResult {
                    id: case.id.clone(),
                    actual_outcome: "equivalent".to_string(),
                    elapsed_ms: started.elapsed().as_millis(),
                    steps: Some(path.steps.len()),
                    reason: None,
                    result_model: equivalent_result_model(
                        HarnessSolverPath::SquareEndpoint,
                        a.rows,
                        b.rows,
                        &telemetry,
                        path.steps.len(),
                        path.matrices.len(),
                    ),
                    telemetry,
                },
                Err(reason) => WorkerCaseResult {
                    id: case.id.clone(),
                    actual_outcome: "panic".to_string(),
                    elapsed_ms: started.elapsed().as_millis(),
                    steps: Some(path.steps.len()),
                    reason: Some(format!("invalid equivalent path: {reason}")),
                    result_model: result_model(
                        HarnessSolverPath::SquareEndpoint,
                        a.rows,
                        b.rows,
                        ResultResolutionKind::InvalidPath,
                        Some(path.steps.len()),
                        Some(path.matrices.len()),
                        &telemetry,
                    ),
                    telemetry,
                },
            },
            DynSseResult::NotEquivalent(reason) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "not_equivalent".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: Some(reason),
                result_model: not_equivalent_result_model(
                    HarnessSolverPath::SquareEndpoint,
                    a.rows,
                    b.rows,
                    &telemetry,
                ),
                telemetry,
            },
            DynSseResult::Unknown => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "unknown".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: None,
                result_model: result_model(
                    HarnessSolverPath::SquareEndpoint,
                    a.rows,
                    b.rows,
                    ResultResolutionKind::SearchExhausted,
                    None,
                    None,
                    &telemetry,
                ),
                telemetry,
            },
        }
    }
}

fn case_matrix(rows: &[Vec<u32>]) -> Result<DynMatrix, String> {
    if rows.is_empty() {
        return Err("matrix must have at least one row".to_string());
    }
    let dim = rows.len();
    if rows.iter().any(|row| row.len() != dim) {
        return Err("matrix must be square".to_string());
    }
    Ok(DynMatrix::new(
        dim,
        dim,
        rows.iter().flat_map(|row| row.iter().copied()).collect(),
    ))
}

fn validate_sse_path_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    path: &SsePath<2>,
) -> Result<(), String> {
    validate_sse_path_dyn(
        &DynMatrix::from_sq(a),
        &DynMatrix::from_sq(b),
        &path.clone().into(),
    )
}

fn validate_sse_path_dyn(a: &DynMatrix, b: &DynMatrix, path: &DynSsePath) -> Result<(), String> {
    if path.steps.is_empty() {
        if path.matrices.len() != 1 {
            return Err(format!(
                "empty-step path should contain exactly one square matrix, got {}",
                path.matrices.len()
            ));
        }
        if path.matrices[0] != *a || path.matrices[0] != *b {
            return Err("empty-step path does not match the input matrices".to_string());
        }
        return Ok(());
    }

    let first = &path.steps[0];
    let first_uv = first.u.mul(&first.v);
    if first_uv != *a {
        return Err("first step does not start at A".to_string());
    }

    let last = path.steps.last().expect("non-empty path has a last step");
    let last_vu = last.v.mul(&last.u);
    if last_vu != *b {
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

fn result_model(
    solver_path: HarnessSolverPath,
    source_dim: usize,
    target_dim: usize,
    resolution_kind: ResultResolutionKind,
    witness_lag: Option<usize>,
    path_matrix_count: Option<usize>,
    telemetry: &SearchTelemetry,
) -> ResultModel {
    ResultModel {
        solver_path,
        source_dim,
        target_dim,
        resolution_kind,
        witness_lag,
        path_matrix_count,
        frontier_layers: telemetry.layers.len(),
    }
}

fn solver_path_for_dims(source_dim: usize, target_dim: usize) -> HarnessSolverPath {
    if source_dim == 2 && target_dim == 2 {
        HarnessSolverPath::TwoByTwo
    } else {
        HarnessSolverPath::SquareEndpoint
    }
}

fn equivalent_result_model(
    solver_path: HarnessSolverPath,
    source_dim: usize,
    target_dim: usize,
    telemetry: &SearchTelemetry,
    witness_lag: usize,
    path_matrix_count: usize,
) -> ResultModel {
    let resolution_kind = if witness_lag == 0 {
        ResultResolutionKind::Identity
    } else if telemetry.permutation_shortcut {
        ResultResolutionKind::PermutationShortcut
    } else if telemetry.canonical_shortcut {
        ResultResolutionKind::CanonicalShortcut
    } else {
        ResultResolutionKind::FrontierPath
    };
    result_model(
        solver_path,
        source_dim,
        target_dim,
        resolution_kind,
        Some(witness_lag),
        Some(path_matrix_count),
        telemetry,
    )
}

fn not_equivalent_result_model(
    solver_path: HarnessSolverPath,
    source_dim: usize,
    target_dim: usize,
    telemetry: &SearchTelemetry,
) -> ResultModel {
    let resolution_kind = if telemetry.invariant_filtered {
        ResultResolutionKind::InvariantFilterNotEquivalent
    } else {
        ResultResolutionKind::SearchNotEquivalent
    };
    result_model(
        solver_path,
        source_dim,
        target_dim,
        resolution_kind,
        None,
        None,
        telemetry,
    )
}

fn run_harness(cases_path: &Path, corpus: &CaseCorpus) -> Result<HarnessSummary, String> {
    let current_exe =
        env::current_exe().map_err(|err| format!("failed to resolve current executable: {err}"))?;

    let mut cases = Vec::with_capacity(corpus.cases.len());
    let mut passed_required_cases = 0usize;
    let mut target_hits = 0usize;
    let mut total_points = 0i64;
    let mut total_elapsed_ms = 0u128;
    let mut generalized_cases = 0usize;
    let mut telemetry_focus_cases = 0usize;
    let mut telemetry_focus_score = 0u64;
    let mut telemetry_focus_directed_score = 0i64;

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
        let endpoint = EndpointSummary {
            source_dim: case.a.len(),
            target_dim: case.b.len(),
            a: case.a.clone(),
            b: case.b.clone(),
        };
        if endpoint.source_dim != 2 || endpoint.target_dim != 2 {
            generalized_cases += 1;
        }
        let telemetry_summary = derive_telemetry_summary(
            &executed.actual_outcome,
            &executed.telemetry,
            case.config.max_lag,
        );
        if case.tags.iter().any(|tag| tag == "telemetry-focus") {
            telemetry_focus_cases += 1;
            telemetry_focus_score += telemetry_summary.focus_progress_score;
            telemetry_focus_directed_score += telemetry_summary.directed_progress_score;
        }

        cases.push(CaseSummary {
            id: case.id.clone(),
            description: case.description.clone(),
            endpoint,
            config: case.config.clone(),
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
            result_model: executed.result_model,
            telemetry: executed.telemetry,
            telemetry_summary,
            tags: case.tags.clone(),
        });
    }

    let comparisons = build_comparison_summaries(&cases);

    Ok(HarnessSummary {
        schema_version: corpus.schema_version,
        cases_path: cases_path.display().to_string(),
        fitness: FitnessSummary {
            required_cases: corpus.cases.len(),
            passed_required_cases,
            target_hits,
            total_points,
            total_elapsed_ms,
            generalized_cases,
            comparison_groups: comparisons.len(),
            telemetry_focus_cases,
            telemetry_focus_score,
            telemetry_focus_reach_score: telemetry_focus_score,
            telemetry_focus_directed_score,
        },
        comparisons,
        cases,
    })
}

fn build_comparison_summaries(cases: &[CaseSummary]) -> Vec<ComparisonSummary> {
    let mut groups = Vec::<ComparisonSummary>::new();
    let mut group_indices = std::collections::BTreeMap::<String, usize>::new();

    for case in cases {
        let key = serde_json::to_string(&(
            case.endpoint.source_dim,
            case.endpoint.target_dim,
            &case.endpoint.a,
            &case.endpoint.b,
        ))
        .expect("endpoint comparison key should serialise");

        let variant = ComparisonVariantSummary {
            case_id: case.id.clone(),
            description: case.description.clone(),
            config: case.config.clone(),
            actual_outcome: case.actual_outcome.clone(),
            result_model: case.result_model.clone(),
            passed: case.passed,
            hit_target: case.hit_target,
            points: case.points,
            elapsed_ms: case.elapsed_ms,
        };

        match group_indices.get(&key).copied() {
            Some(index) => groups[index].variants.push(variant),
            None => {
                group_indices.insert(key, groups.len());
                groups.push(ComparisonSummary {
                    endpoint: case.endpoint.clone(),
                    variants: vec![variant],
                });
            }
        }
    }

    groups
        .into_iter()
        .filter(|group| group.variants.len() > 1)
        .collect()
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
            avg_dead_end_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
            directed_progress_score: 0,
        };
    }

    if telemetry.permutation_shortcut
        || telemetry.canonical_shortcut
        || telemetry.concrete_shift_shortcut
    {
        return DerivedTelemetrySummary {
            productive_layers: 0,
            deepest_productive_layer: None,
            first_stagnant_layer: None,
            exhausted_before_max_lag: false,
            terminal_bottleneck: "shortcut".to_string(),
            avg_factorisations_per_expanded_node: 0.0,
            avg_survivor_ratio: 0.0,
            avg_discovery_ratio: 0.0,
            avg_dead_end_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
            directed_progress_score: 0,
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
            avg_dead_end_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
            directed_progress_score: 0,
        };
    }

    let productive_layers_vec: Vec<&_> = telemetry
        .layers
        .iter()
        .filter(|layer| layer.discovered_nodes > 0 || layer.collisions_with_other_frontier > 0)
        .collect();
    let productive_layers = productive_layers_vec.len();
    let deepest_productive_layer = productive_layers_vec
        .iter()
        .map(|layer| layer.layer_index)
        .max();
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
    let avg_dead_end_ratio = telemetry.dead_end_nodes as f64 / expanded_nodes;

    let last_layer = telemetry
        .layers
        .last()
        .expect("non-empty layers already checked");
    let exhausted_before_max_lag = actual_outcome == "unknown"
        && last_layer.next_frontier_nodes == 0
        && telemetry.layers.len() < configured_max_lag;

    let terminal_bottleneck = classify_bottleneck(actual_outcome, telemetry, last_layer);
    let focus_progress_score =
        compute_reach_progress_score(telemetry, productive_layers, deepest_productive_layer);
    let directed_progress_score = compute_directed_progress_score(
        telemetry,
        deepest_productive_layer,
        avg_discovery_ratio,
        avg_dead_end_ratio,
        exhausted_before_max_lag,
    );

    DerivedTelemetrySummary {
        productive_layers,
        deepest_productive_layer,
        first_stagnant_layer,
        exhausted_before_max_lag,
        terminal_bottleneck,
        avg_factorisations_per_expanded_node,
        avg_survivor_ratio,
        avg_discovery_ratio,
        avg_dead_end_ratio,
        last_layer_frontier_nodes: last_layer.frontier_nodes,
        last_layer_candidates_after_pruning: last_layer.candidates_after_pruning,
        last_layer_discovered_nodes: last_layer.discovered_nodes,
        last_layer_next_frontier_nodes: last_layer.next_frontier_nodes,
        focus_progress_score,
        directed_progress_score,
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
        && telemetry.pruned_by_spectrum.saturating_mul(5)
            >= telemetry.candidates_generated.saturating_mul(4)
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

fn compute_reach_progress_score(
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

fn compute_directed_progress_score(
    telemetry: &SearchTelemetry,
    deepest_productive_layer: Option<usize>,
    avg_discovery_ratio: f64,
    avg_dead_end_ratio: f64,
    exhausted_before_max_lag: bool,
) -> i64 {
    let depth = deepest_productive_layer.unwrap_or(0) as i64;
    let exact_meets = telemetry.collisions_with_other_frontier as i64;
    let approximate_hits = telemetry.approximate_other_side_hits as i64;
    let discovery_per_mille = (avg_discovery_ratio * 1000.0).round() as i64;
    let dead_end_per_mille = (avg_dead_end_ratio * 1000.0).round() as i64;
    let exhaustion_penalty = if exhausted_before_max_lag { 10_000 } else { 0 };

    exact_meets * 1_000_000
        + approximate_hits * 100_000
        + depth * 10_000
        + discovery_per_mille * 1_000
        - dead_end_per_mille * 1_000
        - exhaustion_penalty
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
                result_model: result_model(
                    solver_path_for_dims(case.a.len(), case.b.len()),
                    case.a.len(),
                    case.b.len(),
                    ResultResolutionKind::Timeout,
                    None,
                    None,
                    &SearchTelemetry::default(),
                ),
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
                        result_model: result_model(
                            solver_path_for_dims(case.a.len(), case.b.len()),
                            case.a.len(),
                            case.b.len(),
                            ResultResolutionKind::Panic,
                            None,
                            None,
                            &SearchTelemetry::default(),
                        ),
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
        "generalized_cases: {}\n",
        summary.fitness.generalized_cases
    ));
    out.push_str(&format!(
        "comparison_groups: {}\n",
        summary.fitness.comparison_groups
    ));
    out.push_str(&format!(
        "telemetry_focus_score: {} across {} case(s)\n",
        summary.fitness.telemetry_focus_score, summary.fitness.telemetry_focus_cases
    ));
    out.push_str(&format!(
        "telemetry_focus_directed_score: {}\n",
        summary.fitness.telemetry_focus_directed_score
    ));
    out.push_str(&format!(
        "total_elapsed_ms: {}\n\n",
        summary.fitness.total_elapsed_ms
    ));

    if !summary.comparisons.is_empty() {
        out.push_str("Comparisons\n");
        for comparison in &summary.comparisons {
            out.push_str(&format!(
                "- endpoints={}x{} variants={}\n",
                comparison.endpoint.source_dim,
                comparison.endpoint.target_dim,
                comparison.variants.len()
            ));
            for variant in &comparison.variants {
                out.push_str(&format!(
                    "  {}: mode={:?} max_lag={} max_dim={} max_entry={} outcome={} resolution={:?} witness_lag={:?} points={} elapsed={}ms\n",
                    variant.case_id,
                    variant.config.search_mode,
                    variant.config.max_lag,
                    variant.config.max_intermediate_dim,
                    variant.config.max_entry,
                    variant.actual_outcome,
                    variant.result_model.resolution_kind,
                    variant.result_model.witness_lag,
                    variant.points,
                    variant.elapsed_ms,
                ));
            }
        }
        out.push('\n');
    }

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
        out.push_str(&format!(
            "  endpoints: {}x{} config: mode={:?} max_lag={} max_dim={} max_entry={} timeout={}ms\n",
            case.endpoint.source_dim,
            case.endpoint.target_dim,
            case.config.search_mode,
            case.config.max_lag,
            case.config.max_intermediate_dim,
            case.config.max_entry,
            case.timeout_ms,
        ));
        out.push_str(&format!(
            "  result: solver={:?} resolution={:?} witness_lag={:?} path_matrices={:?} frontier_layers={}\n",
            case.result_model.solver_path,
            case.result_model.resolution_kind,
            case.result_model.witness_lag,
            case.result_model.path_matrix_count,
            case.result_model.frontier_layers,
        ));
        if let Some(reason) = &case.reason {
            out.push_str(&format!("  reason: {}\n", reason));
        }
        out.push_str(&format!(
            "  telemetry: layers={} expanded={} factorisations={} kept={} pruned_size={} pruned_spectrum={} discovered={} approx_hits={} dead_ends={} seen_collisions={} max_frontier={} visited={}\n",
            case.telemetry.layers.len(),
            case.telemetry.frontier_nodes_expanded,
            case.telemetry.factorisations_enumerated,
            case.telemetry.candidates_after_pruning,
            case.telemetry.pruned_by_size,
            case.telemetry.pruned_by_spectrum,
            case.telemetry.discovered_nodes,
            case.telemetry.approximate_other_side_hits,
            case.telemetry.dead_end_nodes,
            case.telemetry.collisions_with_seen,
            case.telemetry.max_frontier_size,
            case.telemetry.total_visited_nodes,
        ));
        out.push_str(&format!(
            "  telemetry_summary: bottleneck={} productive_layers={} deepest_productive_layer={:?} stagnant_layer={:?} exhausted_early={} focus_progress_score={} directed_progress_score={} dead_end_ratio={:.3}\n",
            case.telemetry_summary.terminal_bottleneck,
            case.telemetry_summary.productive_layers,
            case.telemetry_summary.deepest_productive_layer,
            case.telemetry_summary.first_stagnant_layer,
            case.telemetry_summary.exhausted_before_max_lag,
            case.telemetry_summary.focus_progress_score,
            case.telemetry_summary.directed_progress_score,
            case.telemetry_summary.avg_dead_end_ratio,
        ));
        if case.tags.iter().any(|tag| tag == "telemetry-focus") {
            for layer in &case.telemetry.layers {
                out.push_str(&format!(
                    "    layer {} {:?}: frontier={} factors={} kept={} discovered={} approx_hits={} dead_ends={} next={} meet={} seen_collisions={}\n",
                    layer.layer_index,
                    layer.direction,
                    layer.frontier_nodes,
                    layer.factorisations_enumerated,
                    layer.candidates_after_pruning,
                    layer.discovered_nodes,
                    layer.approximate_other_side_hits,
                    layer.dead_end_nodes,
                    layer.next_frontier_nodes,
                    layer.collisions_with_other_frontier,
                    layer.collisions_with_seen,
                ));
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_case_handles_non_2x2_square_endpoints() {
        let case = ResearchCase {
            id: "dyn-3x3-identity".to_string(),
            description: "identity 3x3 case".to_string(),
            a: vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]],
            b: vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]],
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 3,
                max_entry: 1,
                search_mode: SearchMode::Mixed,
            },
            timeout_ms: 1_000,
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            points: OutcomePoints {
                equivalent: 1,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: 0,
            },
            tags: vec![],
        };

        let result = run_case(&case);
        assert_eq!(result.actual_outcome, "equivalent");
        assert_eq!(result.steps, Some(0));
        assert_eq!(
            result.result_model.solver_path,
            HarnessSolverPath::SquareEndpoint
        );
        assert_eq!(
            result.result_model.resolution_kind,
            ResultResolutionKind::Identity
        );
    }

    #[test]
    fn run_case_handles_mixed_square_endpoint_dimensions() {
        let case = ResearchCase {
            id: "dyn-2x2-to-3x3".to_string(),
            description: "one-step 2x2 to 3x3 case".to_string(),
            a: vec![vec![2, 1], vec![1, 1]],
            b: vec![vec![1, 0, 1], vec![1, 1, 1], vec![1, 1, 1]],
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 3,
                max_entry: 1,
                search_mode: SearchMode::Mixed,
            },
            timeout_ms: 1_000,
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            points: OutcomePoints {
                equivalent: 1,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: 0,
            },
            tags: vec![],
        };

        let result = run_case(&case);
        assert_eq!(result.actual_outcome, "equivalent");
        assert_eq!(result.steps, Some(1));
        assert_eq!(
            result.result_model.solver_path,
            HarnessSolverPath::SquareEndpoint
        );
        assert_eq!(
            result.result_model.resolution_kind,
            ResultResolutionKind::FrontierPath
        );
        assert_eq!(result.result_model.source_dim, 2);
        assert_eq!(result.result_model.target_dim, 3);
        assert_eq!(result.result_model.witness_lag, Some(1));
    }

    #[test]
    fn comparison_groups_collect_same_endpoint_variants() {
        let endpoint = EndpointSummary {
            source_dim: 2,
            target_dim: 2,
            a: vec![vec![1, 0], vec![0, 1]],
            b: vec![vec![1, 0], vec![0, 1]],
        };
        let result_model = ResultModel {
            solver_path: HarnessSolverPath::TwoByTwo,
            source_dim: 2,
            target_dim: 2,
            resolution_kind: ResultResolutionKind::Identity,
            witness_lag: Some(0),
            path_matrix_count: Some(1),
            frontier_layers: 0,
        };
        let cases = vec![
            CaseSummary {
                id: "case-a".to_string(),
                description: "A".to_string(),
                endpoint: endpoint.clone(),
                config: JsonSearchConfig {
                    max_lag: 1,
                    max_intermediate_dim: 2,
                    max_entry: 1,
                    search_mode: SearchMode::Mixed,
                },
                actual_outcome: "equivalent".to_string(),
                allowed_outcomes: vec!["equivalent".to_string()],
                target_outcome: Some("equivalent".to_string()),
                passed: true,
                hit_target: true,
                points: 1,
                elapsed_ms: 1,
                timeout_ms: 10,
                steps: Some(0),
                reason: None,
                result_model: result_model.clone(),
                telemetry: SearchTelemetry::default(),
                telemetry_summary: derive_telemetry_summary(
                    "equivalent",
                    &SearchTelemetry::default(),
                    1,
                ),
                tags: vec![],
            },
            CaseSummary {
                id: "case-b".to_string(),
                description: "B".to_string(),
                endpoint,
                config: JsonSearchConfig {
                    max_lag: 2,
                    max_intermediate_dim: 2,
                    max_entry: 2,
                    search_mode: SearchMode::GraphOnly,
                },
                actual_outcome: "equivalent".to_string(),
                allowed_outcomes: vec!["equivalent".to_string()],
                target_outcome: Some("equivalent".to_string()),
                passed: true,
                hit_target: true,
                points: 1,
                elapsed_ms: 1,
                timeout_ms: 10,
                steps: Some(0),
                reason: None,
                result_model,
                telemetry: SearchTelemetry::default(),
                telemetry_summary: derive_telemetry_summary(
                    "equivalent",
                    &SearchTelemetry::default(),
                    1,
                ),
                tags: vec![],
            },
        ];

        let comparisons = build_comparison_summaries(&cases);
        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].variants.len(), 2);
    }
}
