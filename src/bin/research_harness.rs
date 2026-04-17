use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::{Deserialize, Serialize};
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::search::{execute_search_request, validate_sse_path_dyn};
use sse_core::types::{
    DynSsePath, FrontierMode, GuideArtifact, GuideArtifactCompatibility, GuideArtifactEndpoints,
    GuideArtifactPayload, GuideArtifactProvenance, GuideArtifactQuality, GuideArtifactValidation,
    GuidedRefinementConfig, MoveFamilyPolicy, SearchConfig, SearchRequest, SearchRunResult,
    SearchStage, SearchTelemetry, ShortcutSearchConfig,
};

#[path = "research_harness/execution.rs"]
mod execution;
#[path = "research_harness/reporting.rs"]
mod reporting;
#[path = "research_harness/summary.rs"]
mod summary;

use self::execution::run_case;
use self::reporting::format_pretty_summary;
use self::summary::run_harness;
#[cfg(test)]
use self::{
    execution::execute_case_for_harness,
    summary::{
        build_campaign_summaries, build_comparison_summaries, build_deepening_schedule_summaries,
        build_strategy_summaries, derive_telemetry_summary, lag_score, scheduled_cases,
        stage_combination_label,
    },
};

#[derive(Debug)]
struct Cli {
    cases_path: PathBuf,
    format: OutputFormat,
    worker_case: Option<String>,
    worker_output: Option<PathBuf>,
    reuse_runs: Vec<PathBuf>,
    reuse_dirs: Vec<PathBuf>,
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
struct RawCaseCorpus {
    schema_version: u32,
    cases: Vec<RawResearchCase>,
}

#[derive(Clone, Debug, Deserialize)]
struct ResearchCase {
    id: String,
    description: String,
    #[serde(default)]
    a: Vec<Vec<u32>>,
    #[serde(default)]
    b: Vec<Vec<u32>>,
    #[serde(default)]
    endpoint_fixture: Option<String>,
    #[serde(default)]
    seeded_guide_ids: Vec<String>,
    #[serde(default)]
    guide_artifact_paths: Vec<String>,
    #[serde(default = "default_case_required")]
    required: bool,
    config: JsonSearchConfig,
    timeout_ms: u64,
    allowed_outcomes: Vec<String>,
    target_outcome: Option<String>,
    points: OutcomePoints,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    campaign: Option<CampaignConfig>,
    #[serde(default)]
    measurement: Option<MeasurementConfig>,
    #[serde(default)]
    deepening: Option<DeepeningMetadata>,
}

#[derive(Clone, Debug, Deserialize)]
struct RawResearchCase {
    id: String,
    description: String,
    #[serde(default)]
    a: Vec<Vec<u32>>,
    #[serde(default)]
    b: Vec<Vec<u32>>,
    #[serde(default)]
    endpoint_fixture: Option<String>,
    #[serde(default)]
    seeded_guide_ids: Vec<String>,
    #[serde(default)]
    guide_artifact_paths: Vec<String>,
    #[serde(default = "default_case_required")]
    required: bool,
    config: JsonSearchConfig,
    timeout_ms: u64,
    allowed_outcomes: Vec<String>,
    target_outcome: Option<String>,
    points: OutcomePoints,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    campaign: Option<CampaignConfig>,
    #[serde(default)]
    measurement: Option<MeasurementConfig>,
    #[serde(default)]
    deepening_schedule: Option<DeepeningSchedule>,
}

#[derive(Clone, Debug, Deserialize)]
struct DeepeningSchedule {
    attempts: Vec<DeepeningAttempt>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
struct DeepeningMetadata {
    base_case_id: String,
    attempt_number: usize,
    attempt_count: usize,
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
struct DeepeningAttempt {
    #[serde(default)]
    max_lag: Option<usize>,
    #[serde(default)]
    max_intermediate_dim: Option<usize>,
    #[serde(default)]
    max_entry: Option<u32>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CampaignConfig {
    id: String,
    strategy: String,
    #[serde(default)]
    schedule_order: usize,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
struct MeasurementConfig {
    #[serde(default)]
    warmup_runs: usize,
    #[serde(default = "default_measurement_repeat_runs")]
    repeat_runs: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JsonSearchConfig {
    max_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    #[serde(default)]
    frontier_mode: FrontierMode,
    #[serde(default)]
    beam_width: Option<usize>,
    #[serde(default)]
    beam_bfs_handoff_depth: Option<usize>,
    #[serde(default)]
    beam_bfs_handoff_deferred_cap: Option<usize>,
    #[serde(
        default = "default_move_family_policy",
        alias = "search_mode",
        alias = "move_policy"
    )]
    move_family_policy: MoveFamilyPolicy,
    #[serde(default)]
    stage: SearchStage,
    #[serde(default)]
    guided_refinement: GuidedRefinementConfig,
    #[serde(default)]
    shortcut_search: ShortcutSearchConfig,
}

#[derive(Clone, Debug, Deserialize)]
struct EndpointFixtureCollection {
    #[allow(dead_code)]
    schema_version: u32,
    fixtures: Vec<EndpointFixture>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
enum EndpointFixtureFile {
    Collection(EndpointFixtureCollection),
    Single(EndpointFixture),
}

#[derive(Clone, Debug, Deserialize)]
struct EndpointFixture {
    id: String,
    #[serde(default)]
    a: Vec<Vec<u32>>,
    #[serde(default)]
    b: Vec<Vec<u32>>,
    #[serde(default)]
    seeded_guides: Vec<SeededGuideFixture>,
}

#[derive(Clone, Debug, Deserialize)]
struct SeededGuideFixture {
    id: String,
    matrices: Vec<Vec<Vec<u32>>>,
    #[serde(default)]
    label: Option<String>,
    #[serde(default)]
    source_kind: Option<String>,
    #[serde(default)]
    source_ref: Option<String>,
    #[serde(default)]
    supported_stages: Vec<SearchStage>,
    #[serde(default)]
    max_endpoint_dim: Option<usize>,
}

#[derive(Clone, Debug)]
struct ResolvedCase {
    endpoint: EndpointSummary,
    endpoint_fixture: Option<String>,
    seeded_guide_ids: Vec<String>,
    guide_artifact_paths: Vec<String>,
    guide_artifacts: Vec<GuideArtifact>,
}

fn default_case_required() -> bool {
    true
}

fn default_measurement_repeat_runs() -> usize {
    1
}

fn default_move_family_policy() -> MoveFamilyPolicy {
    MoveFamilyPolicy::Mixed
}

#[derive(Clone, Debug, Deserialize)]
struct OutcomePoints {
    equivalent: i64,
    not_equivalent: i64,
    unknown: i64,
    timeout: i64,
    panic: i64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
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
    reused_history_sources: usize,
    fitness: FitnessSummary,
    comparisons: Vec<ComparisonSummary>,
    campaigns: Vec<CampaignSummary>,
    deepening_schedules: Vec<DeepeningScheduleSummary>,
    strategies: Vec<StrategySummary>,
    cases: Vec<CaseSummary>,
}

#[derive(Debug, Serialize)]
struct FitnessSummary {
    required_cases: usize,
    passed_required_cases: usize,
    non_required_cases: usize,
    target_hits: usize,
    total_points: i64,
    total_elapsed_ms: u128,
    current_witness_cases: usize,
    current_witness_lag_total: usize,
    current_lag_score: i64,
    best_known_witness_cases: usize,
    best_known_witness_lag_total: usize,
    best_known_lag_score: i64,
    best_known_improvements: usize,
    generalized_cases: usize,
    comparison_groups: usize,
    campaign_groups: usize,
    deepening_schedule_groups: usize,
    strategy_groups: usize,
    telemetry_focus_cases: usize,
    telemetry_focus_score: u64,
    telemetry_focus_reach_score: u64,
    telemetry_focus_directed_score: i64,
}

#[derive(Debug, Serialize)]
struct CaseSummary {
    id: String,
    description: String,
    campaign: Option<CampaignConfig>,
    measurement: Option<MeasurementSummary>,
    deepening: Option<DeepeningMetadata>,
    endpoint_fixture: Option<String>,
    seeded_guide_ids: Vec<String>,
    guide_artifact_paths: Vec<String>,
    endpoint: EndpointSummary,
    config: JsonSearchConfig,
    actual_outcome: String,
    allowed_outcomes: Vec<String>,
    target_outcome: Option<String>,
    required: bool,
    passed: bool,
    hit_target: bool,
    points: i64,
    elapsed_ms: u128,
    timeout_ms: u64,
    steps: Option<usize>,
    reason: Option<String>,
    result_model: ResultModel,
    best_known_witness: Option<BestKnownWitness>,
    improved_best_known_witness: bool,
    telemetry: SearchTelemetry,
    telemetry_summary: DerivedTelemetrySummary,
    tags: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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
    campaign: Option<CampaignConfig>,
    measurement: Option<MeasurementSummary>,
    stage_combination: String,
    config: JsonSearchConfig,
    actual_outcome: String,
    result_model: ResultModel,
    best_known_witness: Option<BestKnownWitness>,
    improved_best_known_witness: bool,
    passed: bool,
    hit_target: bool,
    points: i64,
    elapsed_ms: u128,
}

#[derive(Clone, Debug, Serialize)]
struct BestKnownWitness {
    lag: usize,
    elapsed_ms: u128,
    source: String,
}

#[derive(Debug, Serialize)]
struct CampaignSummary {
    id: String,
    cases: usize,
    current_witness_cases: usize,
    current_witness_lag_total: usize,
    current_lag_score: i64,
    best_known_witness_cases: usize,
    best_known_witness_lag_total: usize,
    best_known_lag_score: i64,
    total_points: i64,
    total_elapsed_ms: u128,
    scheduled_cases: Vec<CampaignCaseSummary>,
}

#[derive(Debug, Serialize)]
struct CampaignCaseSummary {
    case_id: String,
    strategy: String,
    schedule_order: usize,
    measurement: Option<MeasurementSummary>,
    deepening: Option<DeepeningMetadata>,
    actual_outcome: String,
    current_witness_lag: Option<usize>,
    best_known_witness: Option<BestKnownWitness>,
    improved_best_known_witness: bool,
    points: i64,
    elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
struct DeepeningScheduleSummary {
    base_case_id: String,
    description: String,
    campaign: Option<CampaignConfig>,
    attempts: usize,
    solved_at_attempt: Option<usize>,
    target_hit_at_attempt: Option<usize>,
    best_witness_attempt: Option<usize>,
    total_elapsed_ms: u128,
    scheduled_cases: Vec<DeepeningScheduleCaseSummary>,
}

#[derive(Debug, Serialize)]
struct DeepeningScheduleCaseSummary {
    case_id: String,
    attempt_number: usize,
    attempt_count: usize,
    schedule_order: Option<usize>,
    measurement: Option<MeasurementSummary>,
    config: JsonSearchConfig,
    actual_outcome: String,
    hit_target: bool,
    current_witness_lag: Option<usize>,
    best_known_witness: Option<BestKnownWitness>,
    improved_best_known_witness: bool,
    elapsed_ms: u128,
}

#[derive(Debug, Serialize)]
struct StrategySummary {
    strategy: String,
    cases: usize,
    campaigns: usize,
    passed_required_cases: usize,
    target_hits: usize,
    total_points: i64,
    total_elapsed_ms: u128,
    current_witness_cases: usize,
    current_witness_lag_total: usize,
    current_lag_score: i64,
    best_known_witness_cases: usize,
    best_known_witness_lag_total: usize,
    best_known_lag_score: i64,
    best_known_improvements: usize,
}

#[derive(Clone, Debug, Serialize)]
struct MeasurementSummary {
    warmup_runs: usize,
    repeat_runs: usize,
    elapsed_samples_ms: Vec<u128>,
    min_elapsed_ms: u128,
    median_elapsed_ms: u128,
    p90_elapsed_ms: u128,
    max_elapsed_ms: u128,
    outcome_counts: BTreeMap<String, usize>,
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
    let reused_results = load_reused_results(&cli.reuse_runs, &cli.reuse_dirs)?;

    if let Some(case_id) = cli.worker_case.as_deref() {
        let case = corpus
            .cases
            .iter()
            .find(|case| case.id == case_id)
            .ok_or_else(|| format!("unknown worker case id: {case_id}"))?;
        let result = run_case(case, &cli.cases_path);
        let encoded = serde_json::to_string(&result)
            .map_err(|err| format!("failed to serialise worker result: {err}"))?;
        if let Some(path) = cli.worker_output {
            fs::write(&path, encoded).map_err(|err| {
                format!("failed to write worker result {}: {err}", path.display())
            })?;
        } else {
            println!("{encoded}");
        }
        return Ok(ExitCode::SUCCESS);
    }

    let summary = run_harness(&cli.cases_path, &corpus, &reused_results)?;
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
    let mut worker_output = None;
    let mut reuse_runs = Vec::new();
    let mut reuse_dirs = Vec::new();

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
            "--worker-output" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--worker-output requires a path".to_string())?;
                worker_output = Some(PathBuf::from(value));
            }
            "--reuse-run" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--reuse-run requires a path".to_string())?;
                reuse_runs.push(PathBuf::from(value));
            }
            "--reuse-dir" => {
                let value = args
                    .next()
                    .ok_or_else(|| "--reuse-dir requires a directory".to_string())?;
                reuse_dirs.push(PathBuf::from(value));
            }
            "--help" | "-h" => {
                return Err(
                    "usage: research_harness [--cases research/cases.json] [--format pretty|json] [--worker-case CASE_ID] [--reuse-run PATH]... [--reuse-dir DIR]..."
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
        worker_output,
        reuse_runs,
        reuse_dirs,
    })
}

fn load_corpus(path: &Path) -> Result<CaseCorpus, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    let parsed: RawCaseCorpus = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    let corpus = expand_corpus(parsed)?;
    validate_corpus(&corpus)?;
    Ok(corpus)
}

fn expand_corpus(corpus: RawCaseCorpus) -> Result<CaseCorpus, String> {
    let mut cases = Vec::new();
    for case in corpus.cases {
        cases.extend(expand_case(case)?);
    }
    Ok(CaseCorpus {
        schema_version: corpus.schema_version,
        cases,
    })
}

fn expand_case(case: RawResearchCase) -> Result<Vec<ResearchCase>, String> {
    let schedule = case.deepening_schedule.clone();
    let base_case = case.into_research_case();

    let Some(schedule) = schedule else {
        return Ok(vec![base_case]);
    };

    if schedule.attempts.is_empty() {
        return Err(format!(
            "case {} deepening_schedule.attempts must not be empty",
            base_case.id
        ));
    }

    let attempt_digits = schedule.attempts.len().to_string().len();
    let attempt_count = schedule.attempts.len();
    schedule
        .attempts
        .into_iter()
        .enumerate()
        .map(|(index, attempt)| {
            if attempt.max_lag.is_none()
                && attempt.max_intermediate_dim.is_none()
                && attempt.max_entry.is_none()
            {
                return Err(format!(
                    "case {} deepening_schedule attempt {} must override at least one bound",
                    base_case.id,
                    index + 1
                ));
            }

            let mut derived_case = base_case.clone();
            derived_case.config.max_lag = attempt.max_lag.unwrap_or(base_case.config.max_lag);
            derived_case.config.max_intermediate_dim = attempt
                .max_intermediate_dim
                .unwrap_or(base_case.config.max_intermediate_dim);
            derived_case.config.max_entry = attempt.max_entry.unwrap_or(base_case.config.max_entry);
            derived_case.id = deepening_case_id(
                &base_case.id,
                index + 1,
                attempt_digits,
                &derived_case.config,
            );
            derived_case.deepening = Some(DeepeningMetadata {
                base_case_id: base_case.id.clone(),
                attempt_number: index + 1,
                attempt_count,
            });
            if let Some(campaign) = derived_case.campaign.as_mut() {
                campaign.schedule_order = campaign
                    .schedule_order
                    .checked_add(index)
                    .ok_or_else(|| {
                        format!(
                            "case {} deepening_schedule attempt {} overflowed campaign schedule_order",
                            base_case.id,
                            index + 1
                        )
                    })?;
            }
            Ok(derived_case)
        })
        .collect()
}

fn deepening_case_id(
    base_id: &str,
    attempt_number: usize,
    attempt_digits: usize,
    config: &JsonSearchConfig,
) -> String {
    format!(
        "{base_id}__deepening_{attempt_number:0attempt_digits$}_lag{}_dim{}_entry{}",
        config.max_lag, config.max_intermediate_dim, config.max_entry
    )
}

fn validate_corpus(corpus: &CaseCorpus) -> Result<(), String> {
    for case in &corpus.cases {
        normalized_measurement_config(case)?;
    }
    Ok(())
}

impl RawResearchCase {
    fn into_research_case(self) -> ResearchCase {
        ResearchCase {
            id: self.id,
            description: self.description,
            a: self.a,
            b: self.b,
            endpoint_fixture: self.endpoint_fixture,
            seeded_guide_ids: self.seeded_guide_ids,
            guide_artifact_paths: self.guide_artifact_paths,
            required: self.required,
            config: self.config,
            timeout_ms: self.timeout_ms,
            allowed_outcomes: self.allowed_outcomes,
            target_outcome: self.target_outcome,
            points: self.points,
            tags: self.tags,
            campaign: self.campaign,
            measurement: self.measurement,
            deepening: None,
        }
    }
}

fn normalized_measurement_config(case: &ResearchCase) -> Result<Option<MeasurementConfig>, String> {
    let Some(measurement) = case.measurement.clone() else {
        return Ok(None);
    };

    if case.required {
        return Err(format!(
            "case {} cannot define measurement for required=true cases",
            case.id
        ));
    }
    if measurement.repeat_runs == 0 {
        return Err(format!(
            "case {} measurement.repeat_runs must be at least 1",
            case.id
        ));
    }

    Ok(Some(measurement))
}

fn resolve_case(case: &ResearchCase, cases_path: &Path) -> Result<ResolvedCase, String> {
    let cases_root = cases_path.parent().unwrap_or_else(|| Path::new("."));
    let (endpoint, fixture, seeded_guides) = if let Some(fixture_ref) = &case.endpoint_fixture {
        let fixture = load_endpoint_fixture(fixture_ref, cases_root)?;
        (
            EndpointSummary {
                source_dim: fixture.a.len(),
                target_dim: fixture.b.len(),
                a: fixture.a.clone(),
                b: fixture.b.clone(),
            },
            Some(fixture_ref.clone()),
            fixture.seeded_guides,
        )
    } else {
        if case.a.is_empty() || case.b.is_empty() {
            return Err(format!(
                "case {} must define inline endpoints or endpoint_fixture",
                case.id
            ));
        }
        (
            EndpointSummary {
                source_dim: case.a.len(),
                target_dim: case.b.len(),
                a: case.a.clone(),
                b: case.b.clone(),
            },
            None,
            Vec::new(),
        )
    };

    let mut guide_artifacts = Vec::new();
    let mut guide_artifact_paths = Vec::with_capacity(case.guide_artifact_paths.len());
    for artifact_path in &case.guide_artifact_paths {
        let resolved_path = resolve_relative_path(cases_root, artifact_path);
        guide_artifact_paths.push(resolved_path.display().to_string());
        guide_artifacts.extend(load_guide_artifacts_from_path(&resolved_path)?);
    }

    for guide_id in &case.seeded_guide_ids {
        let guide = seeded_guides
            .iter()
            .find(|guide| guide.id == *guide_id)
            .ok_or_else(|| {
                format!(
                    "case {} requested unknown seeded guide {}",
                    case.id, guide_id
                )
            })?;
        guide_artifacts.push(materialize_seeded_guide_artifact(
            &endpoint,
            fixture.as_deref(),
            guide,
        )?);
    }

    Ok(ResolvedCase {
        endpoint,
        endpoint_fixture: fixture,
        seeded_guide_ids: case.seeded_guide_ids.clone(),
        guide_artifact_paths,
        guide_artifacts,
    })
}

fn resolve_relative_path(root: &Path, path: impl AsRef<Path>) -> PathBuf {
    let path = path.as_ref();
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        let joined = root.join(path);
        if joined.exists() {
            joined
        } else if let Some(parent) = root.parent() {
            let from_parent = parent.join(path);
            if from_parent.exists() {
                from_parent
            } else {
                joined
            }
        } else {
            joined
        }
    }
}

fn load_endpoint_fixture(fixture_ref: &str, cases_root: &Path) -> Result<EndpointFixture, String> {
    let (path, fixture_id) = split_fixture_ref(fixture_ref);
    let path = resolve_relative_path(cases_root, &path);
    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("failed to read endpoint fixture {}: {err}", path.display()))?;
    let parsed: EndpointFixtureFile = serde_json::from_str(&raw)
        .map_err(|err| format!("failed to parse endpoint fixture {}: {err}", path.display()))?;

    let mut fixtures = match parsed {
        EndpointFixtureFile::Single(fixture) => vec![fixture],
        EndpointFixtureFile::Collection(collection) => collection.fixtures,
    };

    match fixture_id {
        Some(id) => fixtures
            .into_iter()
            .find(|fixture| fixture.id == id)
            .ok_or_else(|| format!("fixture {} not found in {}", id, path.display())),
        None => {
            if fixtures.len() != 1 {
                return Err(format!(
                    "fixture file {} contains {} fixtures; use path#fixture_id",
                    path.display(),
                    fixtures.len()
                ));
            }
            Ok(fixtures.remove(0))
        }
    }
}

fn split_fixture_ref(fixture_ref: &str) -> (String, Option<String>) {
    match fixture_ref.split_once('#') {
        Some((path, fixture_id)) if !fixture_id.is_empty() => {
            (path.to_string(), Some(fixture_id.to_string()))
        }
        _ => (fixture_ref.to_string(), None),
    }
}

fn materialize_seeded_guide_artifact(
    endpoint: &EndpointSummary,
    fixture_ref: Option<&str>,
    guide: &SeededGuideFixture,
) -> Result<GuideArtifact, String> {
    if guide.matrices.len() < 2 {
        return Err(format!(
            "seeded guide {} must contain at least two matrices",
            guide.id
        ));
    }

    let matrices = guide
        .matrices
        .iter()
        .map(|rows| case_matrix(rows))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|err| {
            format!(
                "seeded guide {} contains invalid matrix data: {err}",
                guide.id
            )
        })?;

    let max_entry = matrices
        .iter()
        .flat_map(|matrix| matrix.data.iter().copied())
        .max()
        .unwrap_or(0);
    let max_dim = matrices.iter().map(|matrix| matrix.rows).max().unwrap_or(0);

    let mut full_path = DynSsePath {
        matrices: vec![matrices[0].clone()],
        steps: Vec::new(),
    };

    for window in matrices.windows(2) {
        let request = SearchRequest {
            source: window[0].clone(),
            target: window[1].clone(),
            config: SearchConfig {
                max_lag: 1,
                max_intermediate_dim: max_dim,
                max_entry,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
            stage: SearchStage::EndpointSearch,
            guide_artifacts: Vec::new(),
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };
        let (result, _telemetry) = execute_search_request(&request).map_err(|err| {
            format!(
                "failed to reconstruct seeded guide {} segment {}x{} -> {}x{}: {err}",
                guide.id, window[0].rows, window[0].cols, window[1].rows, window[1].cols
            )
        })?;
        let SearchRunResult::Equivalent(segment) = result else {
            return Err(format!(
                "seeded guide {} segment {}x{} -> {}x{} did not produce a direct witness",
                guide.id, window[0].rows, window[0].cols, window[1].rows, window[1].cols
            ));
        };

        if segment.matrices.first() != Some(&window[0])
            || segment.matrices.last() != Some(&window[1])
        {
            return Err(format!(
                "seeded guide {} segment endpoints do not match the requested matrices",
                guide.id
            ));
        }

        if full_path.matrices.last() != segment.matrices.first() {
            return Err(format!(
                "seeded guide {} reconstruction failed to stitch consecutive witness segments",
                guide.id
            ));
        }
        full_path.steps.extend(segment.steps);
        full_path
            .matrices
            .extend(segment.matrices.into_iter().skip(1));
    }

    validate_sse_path_dyn(
        full_path
            .matrices
            .first()
            .expect("seeded guide matrix sequence should have a source matrix"),
        full_path
            .matrices
            .last()
            .expect("seeded guide matrix sequence should have a target matrix"),
        &full_path,
    )
    .map_err(|err| {
        format!(
            "seeded guide {} did not reconstruct cleanly: {err}",
            guide.id
        )
    })?;

    let source = full_path
        .matrices
        .first()
        .expect("seeded guide matrix sequence should have a source matrix")
        .clone();
    let target = full_path
        .matrices
        .last()
        .expect("seeded guide matrix sequence should have a target matrix")
        .clone();
    let endpoint_label = format!("{}->{}", endpoint.source_dim, endpoint.target_dim);
    let label = guide
        .label
        .clone()
        .unwrap_or_else(|| format!("seeded-guide-{}", guide.id));

    Ok(GuideArtifact {
        artifact_id: Some(match fixture_ref {
            Some(fixture_ref) => format!("{fixture_ref}#{}", guide.id),
            None => guide.id.clone(),
        }),
        endpoints: GuideArtifactEndpoints { source, target },
        payload: GuideArtifactPayload::FullPath {
            path: full_path.clone(),
        },
        provenance: GuideArtifactProvenance {
            source_kind: Some(
                guide
                    .source_kind
                    .clone()
                    .unwrap_or_else(|| "seeded_fixture".to_string()),
            ),
            label: Some(label),
            source_ref: Some(guide.source_ref.clone().unwrap_or_else(|| {
                fixture_ref
                    .map(|fixture_ref| format!("{fixture_ref}:{endpoint_label}"))
                    .unwrap_or_else(|| endpoint_label.clone())
            })),
        },
        validation: GuideArtifactValidation::WitnessValidated,
        compatibility: GuideArtifactCompatibility {
            supported_stages: if guide.supported_stages.is_empty() {
                vec![SearchStage::GuidedRefinement]
            } else {
                guide.supported_stages.clone()
            },
            max_endpoint_dim: guide
                .max_endpoint_dim
                .or(Some(endpoint.source_dim.max(endpoint.target_dim))),
        },
        quality: GuideArtifactQuality {
            lag: Some(full_path.steps.len()),
            cost: Some(full_path.steps.len()),
            score: None,
        },
    })
}

#[derive(Debug, Default)]
struct ReusedResults {
    endpoint_best_witness: BTreeMap<String, BestKnownWitness>,
    sources: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PersistedHarnessSummary {
    #[serde(default)]
    cases: Vec<PersistedCaseSummary>,
}

#[derive(Debug, Deserialize)]
struct PersistedCaseSummary {
    endpoint: EndpointSummary,
    elapsed_ms: u128,
    result_model: PersistedResultModel,
}

#[derive(Debug, Deserialize)]
struct PersistedResultModel {
    witness_lag: Option<usize>,
}

fn load_reused_results(
    reuse_runs: &[PathBuf],
    reuse_dirs: &[PathBuf],
) -> Result<ReusedResults, String> {
    let mut sources = reuse_runs.to_vec();
    for dir in reuse_dirs {
        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => continue,
            Err(err) => {
                return Err(format!(
                    "failed to read reuse directory {}: {err}",
                    dir.display()
                ))
            }
        };
        let mut dir_sources = Vec::new();
        for entry in entries {
            let entry =
                entry.map_err(|err| format!("failed to read entry in {}: {err}", dir.display()))?;
            let path = entry.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "json")
            {
                dir_sources.push(path);
            }
        }
        dir_sources.sort();
        sources.extend(dir_sources);
    }

    let mut reused = ReusedResults::default();
    for source in sources {
        let raw = fs::read_to_string(&source)
            .map_err(|err| format!("failed to read reuse artifact {}: {err}", source.display()))?;
        let parsed: PersistedHarnessSummary = serde_json::from_str(&raw)
            .map_err(|err| format!("failed to parse reuse artifact {}: {err}", source.display()))?;
        let source_label = source.display().to_string();
        reused.sources.push(source_label.clone());

        for case in parsed.cases {
            let Some(lag) = case.result_model.witness_lag else {
                continue;
            };
            let endpoint_key = endpoint_identity_key(&case.endpoint);
            let candidate = BestKnownWitness {
                lag,
                elapsed_ms: case.elapsed_ms,
                source: source_label.clone(),
            };
            match reused.endpoint_best_witness.get(&endpoint_key) {
                Some(existing) if !best_known_witness_beats(&candidate, existing) => {}
                _ => {
                    reused.endpoint_best_witness.insert(endpoint_key, candidate);
                }
            }
        }
    }

    Ok(reused)
}

fn endpoint_identity_key(endpoint: &EndpointSummary) -> String {
    serde_json::to_string(&(
        endpoint.source_dim,
        endpoint.target_dim,
        &endpoint.a,
        &endpoint.b,
    ))
    .expect("endpoint identity key should serialise")
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

fn best_known_witness_beats(candidate: &BestKnownWitness, existing: &BestKnownWitness) -> bool {
    candidate.lag < existing.lag
        || (candidate.lag == existing.lag && candidate.elapsed_ms < existing.elapsed_ms)
        || (candidate.lag == existing.lag
            && candidate.elapsed_ms == existing.elapsed_ms
            && candidate.source < existing.source)
}

fn merge_best_known_witness(
    current: Option<BestKnownWitness>,
    historical: Option<&BestKnownWitness>,
) -> (Option<BestKnownWitness>, bool) {
    match (current, historical) {
        (Some(current), Some(historical)) => {
            if best_known_witness_beats(&current, historical) {
                (Some(current), true)
            } else {
                (Some(historical.clone()), false)
            }
        }
        (Some(current), None) => (Some(current), true),
        (None, Some(historical)) => (Some(historical.clone()), false),
        (None, None) => (None, false),
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use std::path::{Path, PathBuf};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn write_temp_corpus(prefix: &str, contents: &str) -> (PathBuf, PathBuf) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!("{prefix}-{}-{timestamp}", std::process::id()));
        fs::create_dir_all(&temp_dir).expect("temporary corpus directory should exist");
        let corpus_path = temp_dir.join("cases.json");
        fs::write(&corpus_path, contents).expect("temporary corpus file should be written");
        (temp_dir, corpus_path)
    }

    fn summary_case(case: &ResearchCase, actual_outcome: &str, elapsed_ms: u128) -> CaseSummary {
        let source_dim = case.a.len();
        let target_dim = case.b.len();
        let passed = case
            .allowed_outcomes
            .iter()
            .any(|allowed| allowed == actual_outcome);
        let hit_target = case
            .target_outcome
            .as_ref()
            .is_some_and(|target| target == actual_outcome);

        CaseSummary {
            id: case.id.clone(),
            description: case.description.clone(),
            campaign: case.campaign.clone(),
            measurement: None,
            deepening: case.deepening.clone(),
            endpoint_fixture: case.endpoint_fixture.clone(),
            seeded_guide_ids: case.seeded_guide_ids.clone(),
            guide_artifact_paths: case.guide_artifact_paths.clone(),
            endpoint: EndpointSummary {
                source_dim,
                target_dim,
                a: case.a.clone(),
                b: case.b.clone(),
            },
            config: case.config.clone(),
            actual_outcome: actual_outcome.to_string(),
            allowed_outcomes: case.allowed_outcomes.clone(),
            target_outcome: case.target_outcome.clone(),
            required: case.required,
            passed,
            hit_target,
            points: case.points.for_outcome(actual_outcome),
            elapsed_ms,
            timeout_ms: case.timeout_ms,
            steps: None,
            reason: None,
            result_model: ResultModel {
                solver_path: if source_dim == 2 && target_dim == 2 {
                    HarnessSolverPath::TwoByTwo
                } else {
                    HarnessSolverPath::SquareEndpoint
                },
                source_dim,
                target_dim,
                resolution_kind: ResultResolutionKind::SearchExhausted,
                witness_lag: None,
                path_matrix_count: None,
                frontier_layers: 0,
            },
            best_known_witness: None,
            improved_best_known_witness: false,
            telemetry: SearchTelemetry::default(),
            telemetry_summary: derive_telemetry_summary(
                actual_outcome,
                &SearchTelemetry::default(),
                case.config.max_lag,
            ),
            tags: case.tags.clone(),
        }
    }

    #[test]
    fn run_case_handles_non_2x2_square_endpoints() {
        let case = ResearchCase {
            id: "dyn-3x3-identity".to_string(),
            description: "identity 3x3 case".to_string(),
            a: vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]],
            b: vec![vec![1, 0, 0], vec![0, 1, 0], vec![0, 0, 1]],
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 3,
                max_entry: 1,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
                shortcut_search: ShortcutSearchConfig::default(),
            },
            timeout_ms: 1_000,
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            required: true,
            points: OutcomePoints {
                equivalent: 1,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: 0,
            },
            tags: vec![],
            campaign: None,
            measurement: None,
            deepening: None,
        };

        let result = run_case(&case, Path::new("research/cases.json"));
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
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 3,
                max_entry: 1,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
                shortcut_search: ShortcutSearchConfig::default(),
            },
            timeout_ms: 1_000,
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            required: true,
            points: OutcomePoints {
                equivalent: 1,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: 0,
            },
            tags: vec![],
            campaign: None,
            measurement: None,
            deepening: None,
        };

        let result = run_case(&case, Path::new("research/cases.json"));
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
    fn run_case_rejects_beam_width_without_beam_frontier() {
        let case = ResearchCase {
            id: "invalid-beam-width".to_string(),
            description: "beam width requires beam frontier".to_string(),
            a: vec![vec![1, 0], vec![0, 1]],
            b: vec![vec![1, 0], vec![0, 1]],
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            required: true,
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 2,
                max_entry: 1,
                frontier_mode: FrontierMode::Bfs,
                beam_width: Some(4),
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
                shortcut_search: ShortcutSearchConfig::default(),
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
            campaign: None,
            measurement: None,
            deepening: None,
        };

        let result = run_case(&case, Path::new("research/cases.json"));
        assert_eq!(result.actual_outcome, "panic");
        assert!(result
            .reason
            .as_deref()
            .is_some_and(|reason| reason.contains("beam_width requires frontier_mode")));
    }

    #[test]
    fn run_case_rejects_handoff_depth_without_handoff_frontier() {
        let case = ResearchCase {
            id: "invalid-handoff-depth".to_string(),
            description: "handoff depth requires handoff frontier".to_string(),
            a: vec![vec![1, 0], vec![0, 1]],
            b: vec![vec![1, 0], vec![0, 1]],
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            required: true,
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 2,
                max_entry: 1,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: Some(4),
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
                shortcut_search: ShortcutSearchConfig::default(),
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
            campaign: None,
            measurement: None,
            deepening: None,
        };

        let result = run_case(&case, Path::new("research/cases.json"));
        assert_eq!(result.actual_outcome, "panic");
        assert!(result.reason.as_deref().is_some_and(|reason| {
            reason.contains("beam_bfs_handoff_depth requires frontier_mode")
        }));
    }

    #[test]
    fn resolve_case_loads_seeded_fixture_guides() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!(
            "research-harness-fixture-{}-{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&temp_dir).expect("temporary fixture directory should be created");
        let fixture_path = temp_dir.join("fixtures.json");
        fs::write(
            &fixture_path,
            r#"{
  "schema_version": 1,
  "fixtures": [
    {
      "id": "guided",
      "a": [[1, 0, 1], [2, 1, 0], [0, 1, 2]],
      "b": [[2, 0, 1], [1, 1, 0], [0, 2, 1]],
      "seeded_guides": [
        {
          "id": "two-hop",
          "matrices": [
            [[1, 0, 1], [2, 1, 0], [0, 1, 2]],
            [[2, 1, 0], [0, 1, 2], [1, 0, 1]],
            [[2, 0, 1], [1, 1, 0], [0, 2, 1]]
          ]
        }
      ]
    }
  ]
}"#,
        )
        .expect("fixture file should be written");

        let case = ResearchCase {
            id: "guided-fixture".to_string(),
            description: "fixture-backed guide".to_string(),
            a: vec![],
            b: vec![],
            endpoint_fixture: Some(format!("{}#guided", fixture_path.display())),
            seeded_guide_ids: vec!["two-hop".to_string()],
            guide_artifact_paths: vec![],
            config: JsonSearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 2,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                stage: SearchStage::GuidedRefinement,
                guided_refinement: GuidedRefinementConfig {
                    max_shortcut_lag: 1,
                    min_gap: 2,
                    max_gap: Some(2),
                    rounds: 1,
                    segment_timeout_secs: None,
                },
                shortcut_search: ShortcutSearchConfig::default(),
            },
            timeout_ms: 1_000,
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            required: true,
            points: OutcomePoints {
                equivalent: 1,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: 0,
            },
            tags: vec![],
            campaign: None,
            measurement: None,
            deepening: None,
        };

        let resolved = resolve_case(&case, &temp_dir.join("cases.json"))
            .expect("fixture-backed case should resolve");
        assert_eq!(resolved.endpoint.source_dim, 3);
        assert_eq!(resolved.guide_artifacts.len(), 1);
        assert_eq!(resolved.guide_artifacts[0].quality.lag, Some(2));

        fs::remove_dir_all(temp_dir).expect("temporary fixture directory should be removed");
    }

    #[test]
    fn run_case_guided_refinement_uses_guide_artifact_inputs() {
        let case = ResearchCase {
            id: "guided-artifact".to_string(),
            description: "guided 3x3 artifact case".to_string(),
            a: vec![],
            b: vec![],
            endpoint_fixture: Some(
                "research/fixtures/generic_guides.json#guided_permutation_3x3".to_string(),
            ),
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![
                "research/guide_artifacts/generic_guided_permutation_3x3.json".to_string(),
            ],
            config: JsonSearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 2,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                stage: SearchStage::GuidedRefinement,
                guided_refinement: GuidedRefinementConfig {
                    max_shortcut_lag: 1,
                    min_gap: 2,
                    max_gap: Some(2),
                    rounds: 1,
                    segment_timeout_secs: None,
                },
                shortcut_search: ShortcutSearchConfig::default(),
            },
            timeout_ms: 1_000,
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            required: true,
            points: OutcomePoints {
                equivalent: 1,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: 0,
            },
            tags: vec![],
            campaign: None,
            measurement: None,
            deepening: None,
        };

        let result = run_case(&case, Path::new("research/cases.json"));
        assert_eq!(result.actual_outcome, "equivalent");
        assert_eq!(result.steps, Some(1));
        assert_eq!(result.telemetry.guide_artifacts_accepted, 1);
        assert_eq!(result.telemetry.guided_segments_improved, 1);
    }

    #[test]
    fn run_case_shortcut_search_uses_guide_pool_inputs() {
        let case = ResearchCase {
            id: "shortcut-artifact-pool".to_string(),
            description: "shortcut 3x3 artifact pool case".to_string(),
            a: vec![],
            b: vec![],
            endpoint_fixture: Some(
                "research/fixtures/generic_guides.json#guided_permutation_3x3".to_string(),
            ),
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![
                "research/guide_artifacts/generic_shortcut_permutation_3x3_pool.json".to_string(),
            ],
            config: JsonSearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 2,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                stage: SearchStage::ShortcutSearch,
                guided_refinement: GuidedRefinementConfig {
                    max_shortcut_lag: 1,
                    min_gap: 2,
                    max_gap: Some(2),
                    rounds: 1,
                    segment_timeout_secs: None,
                },
                shortcut_search: ShortcutSearchConfig {
                    max_guides: 2,
                    rounds: 1,
                    max_total_segment_attempts: 16,
                    ..ShortcutSearchConfig::default()
                },
            },
            timeout_ms: 1_000,
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            required: true,
            points: OutcomePoints {
                equivalent: 1,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: 0,
            },
            tags: vec![],
            campaign: None,
            measurement: None,
            deepening: None,
        };

        let result = run_case(&case, Path::new("research/cases.json"));
        assert_eq!(result.actual_outcome, "equivalent");
        assert_eq!(result.steps, Some(1));
        assert_eq!(result.telemetry.shortcut_search.guide_artifacts_loaded, 2);
        assert_eq!(result.telemetry.shortcut_search.guide_artifacts_accepted, 2);
        assert_eq!(result.telemetry.shortcut_search.unique_guides, 2);
        assert_eq!(
            result.telemetry.shortcut_search.initial_working_set_guides,
            2
        );
    }

    #[test]
    fn load_corpus_parses_measurement_block_for_non_required_cases() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!(
            "research-harness-measurement-corpus-{}-{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&temp_dir).expect("temporary measurement corpus directory should exist");
        let corpus_path = temp_dir.join("cases.json");
        fs::write(
            &corpus_path,
            r#"{
  "schema_version": 5,
  "cases": [
    {
      "id": "measurement-probe",
      "description": "non-required measurement probe",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "required": false,
      "measurement": {
        "warmup_runs": 1,
        "repeat_runs": 5
      },
      "config": {
        "max_lag": 1,
        "max_intermediate_dim": 2,
        "max_entry": 1
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["equivalent"],
      "target_outcome": null,
      "points": {
        "equivalent": 0,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": -100
      }
    }
  ]
}"#,
        )
        .expect("measurement corpus file should be written");

        let corpus = load_corpus(&corpus_path).expect("measurement corpus should load");
        assert_eq!(corpus.cases.len(), 1);
        assert_eq!(
            corpus.cases[0].measurement,
            Some(MeasurementConfig {
                warmup_runs: 1,
                repeat_runs: 5,
            })
        );

        fs::remove_dir_all(temp_dir)
            .expect("temporary measurement corpus directory should go away");
    }

    #[test]
    fn load_corpus_expands_deepening_schedule_into_derived_cases() {
        let (temp_dir, corpus_path) = write_temp_corpus(
            "research-harness-deepening-corpus",
            r#"{
  "schema_version": 5,
  "cases": [
    {
      "id": "deepening-probe",
      "description": "ordered deepening probe",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "required": false,
      "measurement": {
        "repeat_runs": 2
      },
      "campaign": {
        "id": "iterative-bounds",
        "strategy": "mixed",
        "schedule_order": 10
      },
      "config": {
        "max_lag": 6,
        "max_intermediate_dim": 2,
        "max_entry": 3
      },
      "deepening_schedule": {
        "attempts": [
          { "max_lag": 1 },
          { "max_lag": 2, "max_intermediate_dim": 3 },
          { "max_entry": 5 }
        ]
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["unknown"],
      "target_outcome": null,
      "points": {
        "equivalent": 0,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": -100
      },
      "tags": ["deepening", "probe"]
    }
  ]
}"#,
        );

        let corpus = load_corpus(&corpus_path).expect("deepening corpus should load");
        assert_eq!(corpus.cases.len(), 3);
        assert_eq!(
            corpus
                .cases
                .iter()
                .map(|case| case.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "deepening-probe__deepening_1_lag1_dim2_entry3",
                "deepening-probe__deepening_2_lag2_dim3_entry3",
                "deepening-probe__deepening_3_lag6_dim2_entry5",
            ]
        );
        assert_eq!(corpus.cases[0].config.max_lag, 1);
        assert_eq!(corpus.cases[0].config.max_intermediate_dim, 2);
        assert_eq!(corpus.cases[0].config.max_entry, 3);
        assert_eq!(corpus.cases[1].config.max_lag, 2);
        assert_eq!(corpus.cases[1].config.max_intermediate_dim, 3);
        assert_eq!(corpus.cases[1].config.max_entry, 3);
        assert_eq!(corpus.cases[2].config.max_lag, 6);
        assert_eq!(corpus.cases[2].config.max_intermediate_dim, 2);
        assert_eq!(corpus.cases[2].config.max_entry, 5);
        assert_eq!(
            corpus.cases[0].measurement,
            Some(MeasurementConfig {
                warmup_runs: 0,
                repeat_runs: 2,
            })
        );
        assert_eq!(
            corpus
                .cases
                .iter()
                .map(|case| {
                    case.campaign
                        .as_ref()
                        .expect("deepening cases should retain campaign")
                        .schedule_order
                })
                .collect::<Vec<_>>(),
            vec![10, 11, 12]
        );
        assert_eq!(corpus.cases[2].tags, vec!["deepening", "probe"]);

        fs::remove_dir_all(temp_dir).expect("temporary deepening corpus directory should go away");
    }

    #[test]
    fn load_corpus_rejects_deepening_schedule_attempt_without_overrides() {
        let (temp_dir, corpus_path) = write_temp_corpus(
            "research-harness-invalid-deepening-corpus",
            r#"{
  "schema_version": 5,
  "cases": [
    {
      "id": "invalid-deepening",
      "description": "deepening attempt must set at least one bound",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "config": {
        "max_lag": 2,
        "max_intermediate_dim": 2,
        "max_entry": 2
      },
      "deepening_schedule": {
        "attempts": [
          {}
        ]
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["equivalent"],
      "target_outcome": "equivalent",
      "points": {
        "equivalent": 1,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": 0
      }
    }
  ]
}"#,
        );

        let err =
            load_corpus(&corpus_path).expect_err("deepening attempt without bounds should fail");
        assert!(err.contains("must override at least one bound"));

        fs::remove_dir_all(temp_dir)
            .expect("temporary invalid deepening corpus directory should go away");
    }

    #[test]
    fn load_corpus_rejects_measurement_on_required_case() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!(
            "research-harness-invalid-measurement-corpus-{}-{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&temp_dir).expect("temporary invalid corpus directory should exist");
        let corpus_path = temp_dir.join("cases.json");
        fs::write(
            &corpus_path,
            r#"{
  "schema_version": 5,
  "cases": [
    {
      "id": "required-measurement",
      "description": "required case should not accept measurement",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "measurement": {
        "repeat_runs": 3
      },
      "config": {
        "max_lag": 1,
        "max_intermediate_dim": 2,
        "max_entry": 1
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["equivalent"],
      "target_outcome": "equivalent",
      "points": {
        "equivalent": 1,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": 0
      }
    }
  ]
}"#,
        )
        .expect("invalid measurement corpus file should be written");

        let err = load_corpus(&corpus_path).expect_err("required case measurement should fail");
        assert!(err.contains("cannot define measurement"));

        fs::remove_dir_all(temp_dir).expect("temporary invalid corpus directory should go away");
    }

    #[test]
    fn execute_case_for_harness_uses_warmup_repeat_measurement_summary() {
        let case = ResearchCase {
            id: "measurement-probe".to_string(),
            description: "repeat timing probe".to_string(),
            a: vec![vec![1, 0], vec![0, 1]],
            b: vec![vec![1, 0], vec![0, 1]],
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            required: false,
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 2,
                max_entry: 1,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
                shortcut_search: ShortcutSearchConfig::default(),
            },
            timeout_ms: 100,
            allowed_outcomes: vec!["equivalent".to_string(), "unknown".to_string()],
            target_outcome: None,
            points: OutcomePoints {
                equivalent: 0,
                not_equivalent: 0,
                unknown: 0,
                timeout: 0,
                panic: -100,
            },
            tags: vec![],
            campaign: None,
            measurement: Some(MeasurementConfig {
                warmup_runs: 1,
                repeat_runs: 5,
            }),
            deepening: None,
        };
        let result = |elapsed_ms: u128, actual_outcome: &str| WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: actual_outcome.to_string(),
            elapsed_ms,
            steps: None,
            reason: None,
            result_model: ResultModel {
                solver_path: HarnessSolverPath::TwoByTwo,
                source_dim: 2,
                target_dim: 2,
                resolution_kind: ResultResolutionKind::SearchExhausted,
                witness_lag: None,
                path_matrix_count: None,
                frontier_layers: 0,
            },
            telemetry: SearchTelemetry::default(),
        };
        let mut results = VecDeque::from(vec![
            result(90, "unknown"),
            result(22, "equivalent"),
            result(11, "unknown"),
            result(19, "unknown"),
            result(16, "unknown"),
            result(27, "timeout"),
        ]);
        let mut attempts = 0usize;

        let executed = execute_case_for_harness(&case, || {
            attempts += 1;
            Ok(results
                .pop_front()
                .expect("measurement test should have a queued worker result"))
        })
        .expect("measurement execution should succeed");

        assert_eq!(attempts, 6);
        assert_eq!(executed.representative.elapsed_ms, 19);
        assert_eq!(executed.representative.actual_outcome, "unknown");

        let measurement = executed
            .measurement
            .expect("measurement summary should exist for repeated case");
        assert_eq!(measurement.warmup_runs, 1);
        assert_eq!(measurement.repeat_runs, 5);
        assert_eq!(measurement.elapsed_samples_ms, vec![11, 16, 19, 22, 27]);
        assert_eq!(measurement.min_elapsed_ms, 11);
        assert_eq!(measurement.median_elapsed_ms, 19);
        assert_eq!(measurement.p90_elapsed_ms, 27);
        assert_eq!(measurement.max_elapsed_ms, 27);
        assert_eq!(measurement.outcome_counts.get("equivalent"), Some(&1));
        assert_eq!(measurement.outcome_counts.get("unknown"), Some(&3));
        assert_eq!(measurement.outcome_counts.get("timeout"), Some(&1));
    }

    #[test]
    fn summary_serialization_and_pretty_output_include_measurement_stats() {
        let measurement = MeasurementSummary {
            warmup_runs: 1,
            repeat_runs: 5,
            elapsed_samples_ms: vec![11, 16, 19, 22, 27],
            min_elapsed_ms: 11,
            median_elapsed_ms: 19,
            p90_elapsed_ms: 27,
            max_elapsed_ms: 27,
            outcome_counts: BTreeMap::from([
                ("equivalent".to_string(), 1usize),
                ("timeout".to_string(), 1usize),
                ("unknown".to_string(), 3usize),
            ]),
        };
        let summary = HarnessSummary {
            schema_version: 5,
            cases_path: "tmp/measurement-cases.json".to_string(),
            reused_history_sources: 0,
            fitness: FitnessSummary {
                required_cases: 0,
                passed_required_cases: 0,
                non_required_cases: 1,
                target_hits: 0,
                total_points: 0,
                total_elapsed_ms: 19,
                current_witness_cases: 0,
                current_witness_lag_total: 0,
                current_lag_score: 0,
                best_known_witness_cases: 0,
                best_known_witness_lag_total: 0,
                best_known_lag_score: 0,
                best_known_improvements: 0,
                generalized_cases: 0,
                comparison_groups: 0,
                campaign_groups: 0,
                deepening_schedule_groups: 0,
                strategy_groups: 0,
                telemetry_focus_cases: 0,
                telemetry_focus_score: 0,
                telemetry_focus_reach_score: 0,
                telemetry_focus_directed_score: 0,
            },
            comparisons: vec![],
            campaigns: vec![],
            deepening_schedules: vec![],
            strategies: vec![],
            cases: vec![CaseSummary {
                id: "measurement-probe".to_string(),
                description: "repeat timing probe".to_string(),
                campaign: None,
                measurement: Some(measurement.clone()),
                deepening: None,
                endpoint_fixture: None,
                seeded_guide_ids: vec![],
                guide_artifact_paths: vec![],
                endpoint: EndpointSummary {
                    source_dim: 2,
                    target_dim: 2,
                    a: vec![vec![1, 0], vec![0, 1]],
                    b: vec![vec![1, 0], vec![0, 1]],
                },
                config: JsonSearchConfig {
                    max_lag: 1,
                    max_intermediate_dim: 2,
                    max_entry: 1,
                    frontier_mode: FrontierMode::Bfs,
                    beam_width: None,
                    beam_bfs_handoff_depth: None,
                    beam_bfs_handoff_deferred_cap: None,
                    move_family_policy: MoveFamilyPolicy::Mixed,
                    stage: SearchStage::EndpointSearch,
                    guided_refinement: GuidedRefinementConfig::default(),
                    shortcut_search: ShortcutSearchConfig::default(),
                },
                actual_outcome: "unknown".to_string(),
                allowed_outcomes: vec!["equivalent".to_string(), "unknown".to_string()],
                target_outcome: None,
                required: false,
                passed: true,
                hit_target: false,
                points: 0,
                elapsed_ms: 19,
                timeout_ms: 100,
                steps: None,
                reason: None,
                result_model: ResultModel {
                    solver_path: HarnessSolverPath::TwoByTwo,
                    source_dim: 2,
                    target_dim: 2,
                    resolution_kind: ResultResolutionKind::SearchExhausted,
                    witness_lag: None,
                    path_matrix_count: None,
                    frontier_layers: 0,
                },
                best_known_witness: None,
                improved_best_known_witness: false,
                telemetry: SearchTelemetry::default(),
                telemetry_summary: derive_telemetry_summary(
                    "unknown",
                    &SearchTelemetry::default(),
                    1,
                ),
                tags: vec!["benchmark-measurement".to_string()],
            }],
        };

        let encoded = serde_json::to_value(&summary).expect("summary should serialize");
        assert_eq!(
            encoded["cases"][0]["measurement"]["median_elapsed_ms"]
                .as_u64()
                .map(u128::from),
            Some(19)
        );
        assert_eq!(
            encoded["cases"][0]["measurement"]["p90_elapsed_ms"]
                .as_u64()
                .map(u128::from),
            Some(27)
        );

        let pretty = format_pretty_summary(&summary);
        assert!(pretty.contains("measurement: warmups=1 repeats=5"));
        assert!(pretty.contains("median=19ms"));
        assert!(pretty.contains("p90=27ms"));
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
                campaign: Some(CampaignConfig {
                    id: "identity".to_string(),
                    strategy: "mixed".to_string(),
                    schedule_order: 10,
                }),
                measurement: None,
                endpoint_fixture: None,
                seeded_guide_ids: vec![],
                guide_artifact_paths: vec![],
                endpoint: endpoint.clone(),
                config: JsonSearchConfig {
                    max_lag: 1,
                    max_intermediate_dim: 2,
                    max_entry: 1,
                    frontier_mode: FrontierMode::Bfs,
                    beam_width: None,
                    beam_bfs_handoff_depth: None,
                    beam_bfs_handoff_deferred_cap: None,
                    move_family_policy: MoveFamilyPolicy::Mixed,
                    stage: SearchStage::EndpointSearch,
                    guided_refinement: GuidedRefinementConfig::default(),
                    shortcut_search: ShortcutSearchConfig::default(),
                },
                actual_outcome: "equivalent".to_string(),
                allowed_outcomes: vec!["equivalent".to_string()],
                target_outcome: Some("equivalent".to_string()),
                required: true,
                passed: true,
                hit_target: true,
                points: 1,
                elapsed_ms: 1,
                timeout_ms: 10,
                steps: Some(0),
                reason: None,
                result_model: result_model.clone(),
                best_known_witness: Some(BestKnownWitness {
                    lag: 0,
                    elapsed_ms: 1,
                    source: "current-run".to_string(),
                }),
                improved_best_known_witness: true,
                telemetry: SearchTelemetry::default(),
                telemetry_summary: derive_telemetry_summary(
                    "equivalent",
                    &SearchTelemetry::default(),
                    1,
                ),
                tags: vec![],
                deepening: None,
            },
            CaseSummary {
                id: "case-b".to_string(),
                description: "B".to_string(),
                campaign: Some(CampaignConfig {
                    id: "identity".to_string(),
                    strategy: "graph-only".to_string(),
                    schedule_order: 20,
                }),
                measurement: None,
                endpoint_fixture: None,
                seeded_guide_ids: vec![],
                guide_artifact_paths: vec![],
                endpoint,
                config: JsonSearchConfig {
                    max_lag: 2,
                    max_intermediate_dim: 2,
                    max_entry: 2,
                    frontier_mode: FrontierMode::Bfs,
                    beam_width: None,
                    beam_bfs_handoff_depth: None,
                    beam_bfs_handoff_deferred_cap: None,
                    move_family_policy: MoveFamilyPolicy::GraphOnly,
                    stage: SearchStage::EndpointSearch,
                    guided_refinement: GuidedRefinementConfig::default(),
                    shortcut_search: ShortcutSearchConfig::default(),
                },
                actual_outcome: "equivalent".to_string(),
                allowed_outcomes: vec!["equivalent".to_string()],
                target_outcome: Some("equivalent".to_string()),
                required: true,
                passed: true,
                hit_target: true,
                points: 1,
                elapsed_ms: 1,
                timeout_ms: 10,
                steps: Some(0),
                reason: None,
                result_model,
                best_known_witness: Some(BestKnownWitness {
                    lag: 0,
                    elapsed_ms: 1,
                    source: "current-run".to_string(),
                }),
                improved_best_known_witness: true,
                telemetry: SearchTelemetry::default(),
                telemetry_summary: derive_telemetry_summary(
                    "equivalent",
                    &SearchTelemetry::default(),
                    1,
                ),
                tags: vec![],
                deepening: None,
            },
        ];

        let comparisons = build_comparison_summaries(&cases);
        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].variants.len(), 2);
    }

    #[test]
    fn deepening_schedule_order_flows_through_campaign_summary_and_reporting() {
        let (temp_dir, corpus_path) = write_temp_corpus(
            "research-harness-deepening-reporting-corpus",
            r#"{
  "schema_version": 5,
  "cases": [
    {
      "id": "deepening-probe",
      "description": "ordered deepening probe",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "campaign": {
        "id": "iterative-bounds",
        "strategy": "mixed",
        "schedule_order": 10
      },
      "config": {
        "max_lag": 3,
        "max_intermediate_dim": 2,
        "max_entry": 3
      },
      "deepening_schedule": {
        "attempts": [
          { "max_lag": 1 },
          { "max_lag": 2 },
          { "max_entry": 4 }
        ]
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["unknown"],
      "target_outcome": null,
      "points": {
        "equivalent": 0,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": 0
      }
    },
    {
      "id": "fixed-followup",
      "description": "fixed case after deepening schedule",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "campaign": {
        "id": "iterative-bounds",
        "strategy": "mixed",
        "schedule_order": 20
      },
      "config": {
        "max_lag": 4,
        "max_intermediate_dim": 2,
        "max_entry": 4
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["unknown"],
      "target_outcome": null,
      "points": {
        "equivalent": 0,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": 0
      }
    }
  ]
}"#,
        );

        let corpus = load_corpus(&corpus_path).expect("deepening reporting corpus should load");
        let ordered = scheduled_cases(&corpus);
        assert_eq!(
            ordered
                .iter()
                .map(|case| case.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "deepening-probe__deepening_1_lag1_dim2_entry3",
                "deepening-probe__deepening_2_lag2_dim2_entry3",
                "deepening-probe__deepening_3_lag3_dim2_entry4",
                "fixed-followup",
            ]
        );

        let cases = ordered
            .iter()
            .enumerate()
            .map(|(index, case)| summary_case(case, "unknown", (index + 1) as u128))
            .collect::<Vec<_>>();
        let campaigns = build_campaign_summaries(&cases);
        assert_eq!(campaigns.len(), 1);
        assert_eq!(
            campaigns[0]
                .scheduled_cases
                .iter()
                .map(|case| (case.schedule_order, case.case_id.as_str()))
                .collect::<Vec<_>>(),
            vec![
                (10, "deepening-probe__deepening_1_lag1_dim2_entry3"),
                (11, "deepening-probe__deepening_2_lag2_dim2_entry3"),
                (12, "deepening-probe__deepening_3_lag3_dim2_entry4"),
                (20, "fixed-followup"),
            ]
        );
        assert_eq!(
            campaigns[0]
                .scheduled_cases
                .iter()
                .map(|case| {
                    case.deepening.as_ref().map(|deepening| {
                        (
                            deepening.base_case_id.as_str(),
                            deepening.attempt_number,
                            deepening.attempt_count,
                        )
                    })
                })
                .collect::<Vec<_>>(),
            vec![
                Some(("deepening-probe", 1, 3)),
                Some(("deepening-probe", 2, 3)),
                Some(("deepening-probe", 3, 3)),
                None,
            ]
        );

        let deepening_schedules = build_deepening_schedule_summaries(&cases);
        assert_eq!(deepening_schedules.len(), 1);
        assert_eq!(deepening_schedules[0].base_case_id, "deepening-probe");
        assert_eq!(deepening_schedules[0].attempts, 3);
        assert_eq!(deepening_schedules[0].total_elapsed_ms, 6);
        assert_eq!(deepening_schedules[0].scheduled_cases.len(), 3);
        assert_eq!(
            deepening_schedules[0]
                .scheduled_cases
                .iter()
                .map(|case| (
                    case.attempt_number,
                    case.schedule_order,
                    case.case_id.as_str()
                ))
                .collect::<Vec<_>>(),
            vec![
                (1, Some(10), "deepening-probe__deepening_1_lag1_dim2_entry3",),
                (2, Some(11), "deepening-probe__deepening_2_lag2_dim2_entry3",),
                (3, Some(12), "deepening-probe__deepening_3_lag3_dim2_entry4",),
            ]
        );

        let summary = HarnessSummary {
            schema_version: 5,
            cases_path: corpus_path.display().to_string(),
            reused_history_sources: 0,
            fitness: FitnessSummary {
                required_cases: 0,
                passed_required_cases: 0,
                non_required_cases: cases.len(),
                target_hits: 0,
                total_points: 0,
                total_elapsed_ms: 10,
                current_witness_cases: 0,
                current_witness_lag_total: 0,
                current_lag_score: 0,
                best_known_witness_cases: 0,
                best_known_witness_lag_total: 0,
                best_known_lag_score: 0,
                best_known_improvements: 0,
                generalized_cases: 0,
                comparison_groups: 0,
                campaign_groups: campaigns.len(),
                deepening_schedule_groups: 1,
                strategy_groups: 0,
                telemetry_focus_cases: 0,
                telemetry_focus_score: 0,
                telemetry_focus_reach_score: 0,
                telemetry_focus_directed_score: 0,
            },
            comparisons: vec![],
            campaigns,
            deepening_schedules,
            strategies: vec![],
            cases,
        };

        let pretty = format_pretty_summary(&summary);
        assert!(pretty.contains("Campaigns"));
        assert!(pretty.contains("deepening_schedule_groups: 1"));
        assert!(pretty.contains("deepening=1/3 base=deepening-probe"));
        assert!(pretty.contains("Deepening Schedules"));
        assert!(pretty.contains("- deepening-probe: attempts=3"));
        assert!(pretty
            .contains("attempt=1/3 order=Some(10) deepening-probe__deepening_1_lag1_dim2_entry3"));
        assert!(pretty.contains("order=10 deepening-probe__deepening_1_lag1_dim2_entry3"));
        assert!(pretty.contains("order=11 deepening-probe__deepening_2_lag2_dim2_entry3"));
        assert!(pretty.contains("order=12 deepening-probe__deepening_3_lag3_dim2_entry4"));
        assert!(pretty.contains("order=20 fixed-followup"));

        fs::remove_dir_all(temp_dir)
            .expect("temporary deepening reporting corpus directory should go away");
    }

    #[test]
    fn research_corpus_keeps_non_2x2_square_comparison_coverage() {
        let cases_path = Path::new("research/cases.json");
        let corpus = load_corpus(cases_path).expect("research corpus should load");
        let mut groups = BTreeMap::<String, (usize, usize, usize)>::new();

        for case in &corpus.cases {
            let resolved = resolve_case(case, cases_path).expect("case should resolve");
            let endpoint = resolved.endpoint;
            let key = endpoint_identity_key(&endpoint);
            let entry = groups
                .entry(key)
                .or_insert((endpoint.source_dim, endpoint.target_dim, 0));
            entry.2 += 1;
        }

        assert!(groups.values().any(|(source_dim, target_dim, count)| {
            *source_dim == *target_dim && *source_dim > 2 && *count > 1
        }));
    }

    #[test]
    fn research_corpus_keeps_rectangular_positive_pair_bounded_certificate_split() {
        let cases_path = Path::new("research/cases.json");
        let corpus = load_corpus(cases_path).expect("research corpus should load");
        let mut rectangular_cases = corpus
            .cases
            .iter()
            .filter(|case| case.id.starts_with("rectangular_positive_pair"))
            .cloned()
            .collect::<Vec<_>>();

        rectangular_cases.sort_by_key(|case| case.config.max_intermediate_dim);

        assert_eq!(rectangular_cases.len(), 2);
        assert_eq!(
            rectangular_cases
                .iter()
                .map(|case| case.id.as_str())
                .collect::<Vec<_>>(),
            vec![
                "rectangular_positive_pair_dim2_bounded_no_go",
                "rectangular_positive_pair",
            ]
        );
        assert_eq!(
            rectangular_cases[0].allowed_outcomes,
            vec!["unknown".to_string()]
        );
        assert_eq!(
            rectangular_cases[0].target_outcome.as_deref(),
            Some("unknown")
        );
        assert_eq!(
            rectangular_cases[1].allowed_outcomes,
            vec!["equivalent".to_string()]
        );
        assert_eq!(
            rectangular_cases[1].target_outcome.as_deref(),
            Some("equivalent")
        );

        let cases = rectangular_cases
            .iter()
            .map(|case| {
                let actual_outcome = if case.id == "rectangular_positive_pair_dim2_bounded_no_go" {
                    "unknown"
                } else {
                    "equivalent"
                };
                summary_case(case, actual_outcome, 1)
            })
            .collect::<Vec<_>>();
        let comparisons = build_comparison_summaries(&cases);

        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].variants.len(), 2);
        assert_eq!(
            comparisons[0]
                .variants
                .iter()
                .map(|variant| {
                    (
                        variant.case_id.as_str(),
                        variant.config.max_intermediate_dim,
                        variant.actual_outcome.as_str(),
                    )
                })
                .collect::<Vec<_>>(),
            vec![
                ("rectangular_positive_pair_dim2_bounded_no_go", 2, "unknown",),
                ("rectangular_positive_pair", 3, "equivalent"),
            ]
        );

        let summary = HarnessSummary {
            schema_version: 5,
            cases_path: cases_path.display().to_string(),
            reused_history_sources: 0,
            fitness: FitnessSummary {
                required_cases: 2,
                passed_required_cases: 2,
                non_required_cases: 0,
                target_hits: 2,
                total_points: 500,
                total_elapsed_ms: 2,
                current_witness_cases: 0,
                current_witness_lag_total: 0,
                current_lag_score: 0,
                best_known_witness_cases: 0,
                best_known_witness_lag_total: 0,
                best_known_lag_score: 0,
                best_known_improvements: 0,
                generalized_cases: 0,
                comparison_groups: comparisons.len(),
                campaign_groups: 0,
                deepening_schedule_groups: 0,
                strategy_groups: 0,
                telemetry_focus_cases: 0,
                telemetry_focus_score: 0,
                telemetry_focus_reach_score: 0,
                telemetry_focus_directed_score: 0,
            },
            comparisons,
            campaigns: vec![],
            deepening_schedules: vec![],
            strategies: vec![],
            cases,
        };

        let pretty = format_pretty_summary(&summary);
        assert!(pretty.contains("rectangular_positive_pair_dim2_bounded_no_go"));
        assert!(pretty.contains("outcome=unknown"));
        assert!(pretty.contains("max_dim=2"));
        assert!(pretty
            .contains("description: Exact bounded-envelope guard on rectangular_positive_pair"));
    }

    #[test]
    fn json_search_config_serializes_public_move_family_policy_field() {
        let config = JsonSearchConfig {
            max_lag: 2,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::BeamBfsHandoff,
            beam_width: Some(16),
            beam_bfs_handoff_depth: Some(6),
            beam_bfs_handoff_deferred_cap: None,
            move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
            stage: SearchStage::GuidedRefinement,
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let encoded = serde_json::to_value(&config).expect("config should serialize to JSON value");
        let object = encoded
            .as_object()
            .expect("serialized config should be a JSON object");
        assert_eq!(
            object
                .get("move_family_policy")
                .and_then(serde_json::Value::as_str),
            Some("graph_plus_structured")
        );
        assert_eq!(
            object
                .get("beam_bfs_handoff_depth")
                .and_then(serde_json::Value::as_u64),
            Some(6)
        );
        assert!(!object.contains_key("search_mode"));
        assert!(!object.contains_key("move_policy"));
    }

    #[test]
    fn load_corpus_accepts_beam_bfs_handoff_depth_field() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!(
            "research-harness-handoff-depth-corpus-{}-{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&temp_dir).expect("temporary corpus directory should exist");
        let corpus_path = temp_dir.join("cases.json");
        fs::write(
            &corpus_path,
            r#"{
  "schema_version": 5,
  "cases": [
    {
      "id": "handoff-depth-field",
      "description": "beam_bfs_handoff_depth field should deserialize",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "config": {
        "max_lag": 1,
        "max_intermediate_dim": 2,
        "max_entry": 1,
        "frontier_mode": "beam_bfs_handoff",
        "beam_width": 4,
        "beam_bfs_handoff_depth": 7
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["equivalent"],
      "target_outcome": "equivalent",
      "points": {
        "equivalent": 1,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": 0
      }
    }
  ]
}"#,
        )
        .expect("handoff depth corpus file should be written");

        let corpus = load_corpus(&corpus_path).expect("handoff depth corpus should load");
        assert_eq!(corpus.cases.len(), 1);
        assert_eq!(corpus.cases[0].config.beam_bfs_handoff_depth, Some(7));

        fs::remove_dir_all(temp_dir).expect("temporary corpus directory should be removed");
    }

    #[test]
    fn load_corpus_accepts_legacy_search_mode_alias() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!(
            "research-harness-legacy-corpus-{}-{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&temp_dir).expect("temporary legacy corpus directory should exist");
        let corpus_path = temp_dir.join("cases.json");
        fs::write(
            &corpus_path,
            r#"{
  "schema_version": 4,
  "cases": [
    {
      "id": "legacy-search-mode",
      "description": "legacy search_mode alias still loads",
      "a": [[1, 0], [0, 1]],
      "b": [[1, 0], [0, 1]],
      "config": {
        "max_lag": 1,
        "max_intermediate_dim": 2,
        "max_entry": 1,
        "search_mode": "graph-plus-structured"
      },
      "timeout_ms": 100,
      "allowed_outcomes": ["equivalent"],
      "target_outcome": "equivalent",
      "points": {
        "equivalent": 1,
        "not_equivalent": 0,
        "unknown": 0,
        "timeout": 0,
        "panic": 0
      }
    }
  ]
}"#,
        )
        .expect("legacy corpus file should be written");

        let corpus = load_corpus(&corpus_path).expect("legacy corpus should load");
        assert_eq!(corpus.cases.len(), 1);
        assert_eq!(
            corpus.cases[0].config.move_family_policy,
            MoveFamilyPolicy::GraphPlusStructured
        );

        fs::remove_dir_all(temp_dir).expect("temporary legacy corpus directory should be removed");
    }

    #[test]
    fn stage_combination_label_uses_move_family_policy_terminology() {
        let label = stage_combination_label(&JsonSearchConfig {
            max_lag: 2,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::BeamBfsHandoff,
            beam_width: Some(8),
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            stage: SearchStage::ShortcutSearch,
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        });

        assert!(label.contains("move_family_policy=GraphOnly"));
        assert!(!label.contains("move_policy="));
    }

    #[test]
    fn load_reused_results_shares_endpoint_history_across_case_ids() {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after epoch")
            .as_nanos();
        let temp_dir = env::temp_dir().join(format!(
            "research-harness-reuse-{}-{}",
            std::process::id(),
            timestamp
        ));
        fs::create_dir_all(&temp_dir).expect("temporary reuse directory should be created");

        let first = temp_dir.join("first.json");
        let second = temp_dir.join("second.json");
        fs::write(
            &first,
            r#"{
  "cases": [
    {
      "id": "brix_ruiz_k3_graph_only",
      "endpoint": {
        "source_dim": 2,
        "target_dim": 2,
        "a": [[1, 3], [2, 1]],
        "b": [[1, 6], [1, 1]]
      },
      "elapsed_ms": 50,
      "result_model": { "witness_lag": 11 }
    }
  ]
}"#,
        )
        .expect("first reuse artifact should be written");
        fs::write(
            &second,
            r#"{
  "cases": [
    {
      "id": "brix_ruiz_k3_graph_only",
      "endpoint": {
        "source_dim": 2,
        "target_dim": 2,
        "a": [[1, 3], [2, 1]],
        "b": [[1, 6], [1, 1]]
      },
      "elapsed_ms": 40,
      "result_model": { "witness_lag": 7 }
    },
    {
      "id": "identity",
      "endpoint": {
        "source_dim": 2,
        "target_dim": 2,
        "a": [[1, 0], [0, 1]],
        "b": [[1, 0], [0, 1]]
      },
      "elapsed_ms": 2,
      "result_model": { "witness_lag": 0 }
    }
  ]
}"#,
        )
        .expect("second reuse artifact should be written");

        let reused =
            load_reused_results(&[], &[temp_dir.clone()]).expect("reuse artifacts should load");
        let endpoint_key = endpoint_identity_key(&EndpointSummary {
            source_dim: 2,
            target_dim: 2,
            a: vec![vec![1, 3], vec![2, 1]],
            b: vec![vec![1, 6], vec![1, 1]],
        });
        let brix = reused
            .endpoint_best_witness
            .get(&endpoint_key)
            .expect("shared endpoint witness should exist");
        assert_eq!(brix.lag, 7);
        assert_eq!(brix.elapsed_ms, 40);
        assert_eq!(reused.sources.len(), 2);

        let merged_for_other_case =
            merge_best_known_witness(None, reused.endpoint_best_witness.get(&endpoint_key));
        assert_eq!(
            merged_for_other_case.0.as_ref().map(|witness| witness.lag),
            Some(7)
        );
        assert!(!merged_for_other_case.1);

        let endpoint = EndpointSummary {
            source_dim: 2,
            target_dim: 2,
            a: vec![vec![1, 3], vec![2, 1]],
            b: vec![vec![1, 6], vec![1, 1]],
        };
        let case = |id: &str, strategy: &str| CaseSummary {
            id: id.to_string(),
            description: id.to_string(),
            campaign: Some(CampaignConfig {
                id: "brix_ruiz_k3".to_string(),
                strategy: strategy.to_string(),
                schedule_order: 10,
            }),
            measurement: None,
            deepening: None,
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            endpoint: endpoint.clone(),
            config: JsonSearchConfig {
                max_lag: 6,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
                shortcut_search: ShortcutSearchConfig::default(),
            },
            actual_outcome: "unknown".to_string(),
            allowed_outcomes: vec!["equivalent".to_string(), "unknown".to_string()],
            target_outcome: Some("equivalent".to_string()),
            required: true,
            passed: true,
            hit_target: false,
            points: 0,
            elapsed_ms: 10,
            timeout_ms: 100,
            steps: None,
            reason: None,
            result_model: ResultModel {
                solver_path: HarnessSolverPath::TwoByTwo,
                source_dim: 2,
                target_dim: 2,
                resolution_kind: ResultResolutionKind::SearchExhausted,
                witness_lag: None,
                path_matrix_count: None,
                frontier_layers: 0,
            },
            best_known_witness: merged_for_other_case.0.clone(),
            improved_best_known_witness: false,
            telemetry: SearchTelemetry::default(),
            telemetry_summary: derive_telemetry_summary("unknown", &SearchTelemetry::default(), 6),
            tags: vec![],
        };

        let strategies = build_strategy_summaries(&[
            case("brix_ruiz_k3", "mixed_baseline"),
            case("brix_ruiz_k3_wide_probe", "mixed_wide_probe"),
        ]);
        assert_eq!(strategies.len(), 2);
        for strategy in strategies {
            assert_eq!(strategy.best_known_witness_cases, 1);
            assert_eq!(strategy.best_known_witness_lag_total, 7);
        }

        fs::remove_dir_all(temp_dir).expect("temporary reuse directory should be removed");
    }

    #[test]
    fn strategy_summaries_aggregate_campaign_cases() {
        let endpoint = EndpointSummary {
            source_dim: 2,
            target_dim: 2,
            a: vec![vec![1, 0], vec![0, 1]],
            b: vec![vec![1, 0], vec![0, 1]],
        };
        let case = |id: &str,
                    campaign_id: &str,
                    strategy: &str,
                    schedule_order: usize,
                    witness_lag: Option<usize>,
                    best_known_lag: Option<usize>,
                    elapsed_ms: u128| CaseSummary {
            id: id.to_string(),
            description: id.to_string(),
            campaign: Some(CampaignConfig {
                id: campaign_id.to_string(),
                strategy: strategy.to_string(),
                schedule_order,
            }),
            measurement: None,
            deepening: None,
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            endpoint: endpoint.clone(),
            config: JsonSearchConfig {
                max_lag: 4,
                max_intermediate_dim: 2,
                max_entry: 4,
                frontier_mode: FrontierMode::Bfs,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: MoveFamilyPolicy::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
                shortcut_search: ShortcutSearchConfig::default(),
            },
            actual_outcome: "equivalent".to_string(),
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
            required: true,
            passed: true,
            hit_target: true,
            points: 10,
            elapsed_ms,
            timeout_ms: 10,
            steps: witness_lag,
            reason: None,
            result_model: ResultModel {
                solver_path: HarnessSolverPath::TwoByTwo,
                source_dim: 2,
                target_dim: 2,
                resolution_kind: ResultResolutionKind::FrontierPath,
                witness_lag,
                path_matrix_count: witness_lag.map(|lag| lag + 1),
                frontier_layers: witness_lag.unwrap_or(0),
            },
            best_known_witness: best_known_lag.map(|lag| BestKnownWitness {
                lag,
                elapsed_ms,
                source: "current-run".to_string(),
            }),
            improved_best_known_witness: best_known_lag.is_some(),
            telemetry: SearchTelemetry::default(),
            telemetry_summary: derive_telemetry_summary(
                "equivalent",
                &SearchTelemetry::default(),
                4,
            ),
            tags: vec![],
        };

        let cases = vec![
            case("mixed-a", "baseline", "mixed", 10, Some(6), Some(6), 20),
            case("mixed-b", "baseline", "mixed", 20, Some(4), Some(4), 15),
            case(
                "graph-a",
                "baseline",
                "graph-only",
                30,
                Some(8),
                Some(8),
                35,
            ),
        ];

        let campaigns = build_campaign_summaries(&cases);
        assert_eq!(campaigns.len(), 1);
        assert_eq!(campaigns[0].scheduled_cases.len(), 3);
        assert_eq!(campaigns[0].current_witness_lag_total, 18);

        let strategies = build_strategy_summaries(&cases);
        assert_eq!(strategies.len(), 2);
        let mixed = strategies
            .iter()
            .find(|strategy| strategy.strategy == "mixed")
            .expect("mixed strategy summary should exist");
        assert_eq!(mixed.cases, 2);
        assert_eq!(mixed.current_witness_lag_total, 10);
        assert_eq!(mixed.best_known_lag_score, lag_score(2, 10));
    }
}
