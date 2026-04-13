use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use sse_core::matrix::DynMatrix;
use sse_core::search::{execute_search_request, validate_sse_path_dyn};
use sse_core::types::{
    DynSsePath, GuideArtifact, GuideArtifactCompatibility, GuideArtifactEndpoints,
    GuideArtifactPayload, GuideArtifactProvenance, GuideArtifactQuality, GuideArtifactValidation,
    GuidedRefinementConfig, SearchConfig, SearchMode, SearchRequest, SearchRunResult, SearchStage,
    SearchTelemetry,
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
    config: JsonSearchConfig,
    timeout_ms: u64,
    allowed_outcomes: Vec<String>,
    target_outcome: Option<String>,
    points: OutcomePoints,
    #[serde(default)]
    tags: Vec<String>,
    #[serde(default)]
    campaign: Option<CampaignConfig>,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
struct CampaignConfig {
    id: String,
    strategy: String,
    #[serde(default)]
    schedule_order: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct JsonSearchConfig {
    max_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    #[serde(default = "default_search_mode")]
    search_mode: SearchMode,
    #[serde(default)]
    stage: SearchStage,
    #[serde(default)]
    guided_refinement: GuidedRefinementConfig,
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
    reused_history_sources: usize,
    fitness: FitnessSummary,
    comparisons: Vec<ComparisonSummary>,
    campaigns: Vec<CampaignSummary>,
    strategies: Vec<StrategySummary>,
    cases: Vec<CaseSummary>,
}

#[derive(Debug, Serialize)]
struct FitnessSummary {
    required_cases: usize,
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
    generalized_cases: usize,
    comparison_groups: usize,
    campaign_groups: usize,
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
    endpoint_fixture: Option<String>,
    seeded_guide_ids: Vec<String>,
    guide_artifact_paths: Vec<String>,
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
    actual_outcome: String,
    current_witness_lag: Option<usize>,
    best_known_witness: Option<BestKnownWitness>,
    improved_best_known_witness: bool,
    points: i64,
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
    serde_json::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
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
        guide_artifacts.extend(load_guide_artifacts(&resolved_path)?);
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

#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum GuideArtifactFile {
    Artifact(GuideArtifact),
    Artifacts(Vec<GuideArtifact>),
    Envelope { artifacts: Vec<GuideArtifact> },
}

fn load_guide_artifacts(path: impl AsRef<Path>) -> Result<Vec<GuideArtifact>, String> {
    let path = path.as_ref();
    let json = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read guide artifacts from {}: {err}",
            path.display()
        )
    })?;
    let parsed: GuideArtifactFile = serde_json::from_str(&json).map_err(|err| {
        format!(
            "failed to parse guide artifacts from {} as JSON: {err}",
            path.display()
        )
    })?;
    Ok(match parsed {
        GuideArtifactFile::Artifact(artifact) => vec![artifact],
        GuideArtifactFile::Artifacts(artifacts) => artifacts,
        GuideArtifactFile::Envelope { artifacts } => artifacts,
    })
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
                search_mode: SearchMode::GraphOnly,
            },
            stage: SearchStage::EndpointSearch,
            guide_artifacts: Vec::new(),
            guided_refinement: GuidedRefinementConfig::default(),
        };
        let (result, _telemetry) = execute_search_request(&request).map_err(|err| {
            format!(
                "failed to reconstruct seeded guide {} segment {}x{} -> {}x{}: {err}",
                guide.id, window[0].rows, window[0].cols, window[1].rows, window[1].cols
            )
        })?;
        let SearchRunResult::Equivalent(segment) = result else {
            return Err(format!(
                "seeded guide {} segment {}x{} -> {}x{} did not produce a direct path",
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
                "seeded guide {} reconstruction failed to stitch consecutive segments",
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
            .expect("seeded guide path should have a source matrix"),
        full_path
            .matrices
            .last()
            .expect("seeded guide path should have a target matrix"),
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
        .expect("seeded guide path should have a source matrix")
        .clone();
    let target = full_path
        .matrices
        .last()
        .expect("seeded guide path should have a target matrix")
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

fn run_case(case: &ResearchCase, cases_path: &Path) -> WorkerCaseResult {
    let started = Instant::now();
    let resolved = match resolve_case(case, cases_path) {
        Ok(resolved) => resolved,
        Err(reason) => {
            return panic_result(
                &case.id,
                case.a.len(),
                case.b.len(),
                started.elapsed().as_millis(),
                reason,
            )
        }
    };

    let a = match case_matrix(&resolved.endpoint.a) {
        Ok(matrix) => matrix,
        Err(reason) => {
            return panic_result(
                &case.id,
                resolved.endpoint.source_dim,
                resolved.endpoint.target_dim,
                started.elapsed().as_millis(),
                format!("invalid matrix A in corpus: {reason}"),
            )
        }
    };
    let b = match case_matrix(&resolved.endpoint.b) {
        Ok(matrix) => matrix,
        Err(reason) => {
            return panic_result(
                &case.id,
                resolved.endpoint.source_dim,
                resolved.endpoint.target_dim,
                started.elapsed().as_millis(),
                format!("invalid matrix B in corpus: {reason}"),
            )
        }
    };

    let request = SearchRequest {
        source: a.clone(),
        target: b.clone(),
        config: SearchConfig {
            max_lag: case.config.max_lag,
            max_intermediate_dim: case.config.max_intermediate_dim,
            max_entry: case.config.max_entry,
            search_mode: case.config.search_mode,
        },
        stage: case.config.stage,
        guide_artifacts: resolved.guide_artifacts,
        guided_refinement: case.config.guided_refinement.clone(),
    };

    let (result, telemetry) = match execute_search_request(&request) {
        Ok(outcome) => outcome,
        Err(reason) => {
            return panic_result(
                &case.id,
                a.rows,
                b.rows,
                started.elapsed().as_millis(),
                reason,
            )
        }
    };

    match result {
        SearchRunResult::Equivalent(path) => match validate_sse_path_dyn(&a, &b, &path) {
            Ok(()) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "equivalent".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: Some(path.steps.len()),
                reason: None,
                result_model: equivalent_result_model(
                    solver_path_for_dims(a.rows, b.rows),
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
                    solver_path_for_dims(a.rows, b.rows),
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
        SearchRunResult::EquivalentByConcreteShift(witness) => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "equivalent".to_string(),
            elapsed_ms: started.elapsed().as_millis(),
            steps: None,
            reason: Some("aligned concrete-shift witness".to_string()),
            result_model: result_model(
                solver_path_for_dims(a.rows, b.rows),
                a.rows,
                b.rows,
                ResultResolutionKind::ConcreteShiftWitness,
                Some(witness.shift.lag as usize),
                None,
                &telemetry,
            ),
            telemetry,
        },
        SearchRunResult::NotEquivalent(reason) => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "not_equivalent".to_string(),
            elapsed_ms: started.elapsed().as_millis(),
            steps: None,
            reason: Some(reason),
            result_model: not_equivalent_result_model(
                solver_path_for_dims(a.rows, b.rows),
                a.rows,
                b.rows,
                &telemetry,
            ),
            telemetry,
        },
        SearchRunResult::Unknown => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "unknown".to_string(),
            elapsed_ms: started.elapsed().as_millis(),
            steps: None,
            reason: None,
            result_model: result_model(
                solver_path_for_dims(a.rows, b.rows),
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

fn panic_result(
    case_id: &str,
    source_dim: usize,
    target_dim: usize,
    elapsed_ms: u128,
    reason: String,
) -> WorkerCaseResult {
    WorkerCaseResult {
        id: case_id.to_string(),
        actual_outcome: "panic".to_string(),
        elapsed_ms,
        steps: None,
        reason: Some(reason),
        result_model: result_model(
            solver_path_for_dims(source_dim, target_dim),
            source_dim,
            target_dim,
            ResultResolutionKind::Panic,
            None,
            None,
            &SearchTelemetry::default(),
        ),
        telemetry: SearchTelemetry::default(),
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

fn lag_score(witness_cases: usize, witness_lag_total: usize) -> i64 {
    witness_cases as i64 * 1_000_000 - witness_lag_total as i64
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

fn stage_combination_label(config: &JsonSearchConfig) -> String {
    match config.stage {
        SearchStage::EndpointSearch => format!("endpoint_search/{:?}", config.search_mode),
        SearchStage::GuidedRefinement => format!(
            "guided_refinement/{:?}/shortcut_lag={}/min_gap={}/max_gap={:?}/rounds={}",
            config.search_mode,
            config.guided_refinement.max_shortcut_lag,
            config.guided_refinement.min_gap,
            config.guided_refinement.max_gap,
            config.guided_refinement.rounds
        ),
        SearchStage::ShortcutSearch => format!("shortcut_search/{:?}", config.search_mode),
    }
}

fn run_harness(
    cases_path: &Path,
    corpus: &CaseCorpus,
    reused_results: &ReusedResults,
) -> Result<HarnessSummary, String> {
    let current_exe =
        env::current_exe().map_err(|err| format!("failed to resolve current executable: {err}"))?;

    let mut cases = Vec::with_capacity(corpus.cases.len());
    let mut scheduled_cases = corpus.cases.iter().collect::<Vec<_>>();
    scheduled_cases.sort_by_key(|case| {
        case.campaign
            .as_ref()
            .map(|campaign| {
                (
                    0usize,
                    campaign.id.clone(),
                    campaign.schedule_order,
                    case.id.clone(),
                )
            })
            .unwrap_or_else(|| (1usize, String::new(), 0usize, case.id.clone()))
    });

    let mut passed_required_cases = 0usize;
    let mut target_hits = 0usize;
    let mut total_points = 0i64;
    let mut total_elapsed_ms = 0u128;
    let mut current_witness_cases = 0usize;
    let mut current_witness_lag_total = 0usize;
    let mut best_known_witness_cases = 0usize;
    let mut best_known_witness_lag_total = 0usize;
    let mut best_known_improvements = 0usize;
    let mut generalized_cases = 0usize;
    let mut telemetry_focus_cases = 0usize;
    let mut telemetry_focus_score = 0u64;
    let mut telemetry_focus_directed_score = 0i64;

    for case in scheduled_cases {
        let resolved = resolve_case(case, cases_path)?;
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
        if let Some(lag) = executed.result_model.witness_lag {
            current_witness_cases += 1;
            current_witness_lag_total += lag;
        }
        let endpoint = resolved.endpoint.clone();
        let endpoint_key = endpoint_identity_key(&endpoint);
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

        let (best_known_witness, improved_best_known_witness) = merge_best_known_witness(
            executed
                .result_model
                .witness_lag
                .map(|lag| BestKnownWitness {
                    lag,
                    elapsed_ms: executed.elapsed_ms,
                    source: "current-run".to_string(),
                }),
            reused_results.endpoint_best_witness.get(&endpoint_key),
        );
        if let Some(best_known) = &best_known_witness {
            best_known_witness_cases += 1;
            best_known_witness_lag_total += best_known.lag;
        }
        if improved_best_known_witness {
            best_known_improvements += 1;
        }

        cases.push(CaseSummary {
            id: case.id.clone(),
            description: case.description.clone(),
            campaign: case.campaign.clone(),
            endpoint_fixture: resolved.endpoint_fixture,
            seeded_guide_ids: resolved.seeded_guide_ids,
            guide_artifact_paths: resolved.guide_artifact_paths,
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
            best_known_witness,
            improved_best_known_witness,
            telemetry: executed.telemetry,
            telemetry_summary,
            tags: case.tags.clone(),
        });
    }

    let comparisons = build_comparison_summaries(&cases);
    let campaigns = build_campaign_summaries(&cases);
    let strategies = build_strategy_summaries(&cases);

    Ok(HarnessSummary {
        schema_version: corpus.schema_version,
        cases_path: cases_path.display().to_string(),
        reused_history_sources: reused_results.sources.len(),
        fitness: FitnessSummary {
            required_cases: corpus.cases.len(),
            passed_required_cases,
            target_hits,
            total_points,
            total_elapsed_ms,
            current_witness_cases,
            current_witness_lag_total,
            current_lag_score: lag_score(current_witness_cases, current_witness_lag_total),
            best_known_witness_cases,
            best_known_witness_lag_total,
            best_known_lag_score: lag_score(best_known_witness_cases, best_known_witness_lag_total),
            best_known_improvements,
            generalized_cases,
            comparison_groups: comparisons.len(),
            campaign_groups: campaigns.len(),
            strategy_groups: strategies.len(),
            telemetry_focus_cases,
            telemetry_focus_score,
            telemetry_focus_reach_score: telemetry_focus_score,
            telemetry_focus_directed_score,
        },
        comparisons,
        campaigns,
        strategies,
        cases,
    })
}

fn build_comparison_summaries(cases: &[CaseSummary]) -> Vec<ComparisonSummary> {
    let mut groups = Vec::<ComparisonSummary>::new();
    let mut group_indices = BTreeMap::<String, usize>::new();

    for case in cases {
        let key = endpoint_identity_key(&case.endpoint);

        let variant = ComparisonVariantSummary {
            case_id: case.id.clone(),
            description: case.description.clone(),
            campaign: case.campaign.clone(),
            stage_combination: stage_combination_label(&case.config),
            config: case.config.clone(),
            actual_outcome: case.actual_outcome.clone(),
            result_model: case.result_model.clone(),
            best_known_witness: case.best_known_witness.clone(),
            improved_best_known_witness: case.improved_best_known_witness,
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

fn build_campaign_summaries(cases: &[CaseSummary]) -> Vec<CampaignSummary> {
    let mut groups = BTreeMap::<String, Vec<&CaseSummary>>::new();
    for case in cases {
        let Some(campaign) = case.campaign.as_ref() else {
            continue;
        };
        groups.entry(campaign.id.clone()).or_default().push(case);
    }

    groups
        .into_iter()
        .map(|(id, mut group_cases)| {
            group_cases.sort_by_key(|case| {
                let campaign = case
                    .campaign
                    .as_ref()
                    .expect("campaign grouping only contains campaign cases");
                (
                    campaign.schedule_order,
                    campaign.strategy.clone(),
                    case.id.clone(),
                )
            });

            let mut current_witness_cases = 0usize;
            let mut current_witness_lag_total = 0usize;
            let mut best_known_witness_cases = 0usize;
            let mut best_known_witness_lag_total = 0usize;
            let mut total_points = 0i64;
            let mut total_elapsed_ms = 0u128;
            let mut scheduled_cases = Vec::with_capacity(group_cases.len());

            for case in group_cases {
                total_points += case.points;
                total_elapsed_ms += case.elapsed_ms;
                if let Some(lag) = case.result_model.witness_lag {
                    current_witness_cases += 1;
                    current_witness_lag_total += lag;
                }
                if let Some(best_known) = &case.best_known_witness {
                    best_known_witness_cases += 1;
                    best_known_witness_lag_total += best_known.lag;
                }

                let campaign = case
                    .campaign
                    .as_ref()
                    .expect("campaign grouping only contains campaign cases");
                scheduled_cases.push(CampaignCaseSummary {
                    case_id: case.id.clone(),
                    strategy: campaign.strategy.clone(),
                    schedule_order: campaign.schedule_order,
                    actual_outcome: case.actual_outcome.clone(),
                    current_witness_lag: case.result_model.witness_lag,
                    best_known_witness: case.best_known_witness.clone(),
                    improved_best_known_witness: case.improved_best_known_witness,
                    points: case.points,
                    elapsed_ms: case.elapsed_ms,
                });
            }

            CampaignSummary {
                id,
                cases: scheduled_cases.len(),
                current_witness_cases,
                current_witness_lag_total,
                current_lag_score: lag_score(current_witness_cases, current_witness_lag_total),
                best_known_witness_cases,
                best_known_witness_lag_total,
                best_known_lag_score: lag_score(
                    best_known_witness_cases,
                    best_known_witness_lag_total,
                ),
                total_points,
                total_elapsed_ms,
                scheduled_cases,
            }
        })
        .collect()
}

fn build_strategy_summaries(cases: &[CaseSummary]) -> Vec<StrategySummary> {
    let mut groups = BTreeMap::<String, Vec<&CaseSummary>>::new();
    for case in cases {
        let Some(campaign) = case.campaign.as_ref() else {
            continue;
        };
        groups
            .entry(campaign.strategy.clone())
            .or_default()
            .push(case);
    }

    groups
        .into_iter()
        .map(|(strategy, group_cases)| {
            let mut campaigns = BTreeMap::<String, ()>::new();
            let mut passed_required_cases = 0usize;
            let mut target_hits = 0usize;
            let mut total_points = 0i64;
            let mut total_elapsed_ms = 0u128;
            let mut current_witness_cases = 0usize;
            let mut current_witness_lag_total = 0usize;
            let mut best_known_witness_cases = 0usize;
            let mut best_known_witness_lag_total = 0usize;
            let mut best_known_improvements = 0usize;

            for case in &group_cases {
                if case.passed {
                    passed_required_cases += 1;
                }
                if case.hit_target {
                    target_hits += 1;
                }
                total_points += case.points;
                total_elapsed_ms += case.elapsed_ms;
                if let Some(lag) = case.result_model.witness_lag {
                    current_witness_cases += 1;
                    current_witness_lag_total += lag;
                }
                if let Some(best_known) = &case.best_known_witness {
                    best_known_witness_cases += 1;
                    best_known_witness_lag_total += best_known.lag;
                }
                if case.improved_best_known_witness {
                    best_known_improvements += 1;
                }
                if let Some(campaign) = case.campaign.as_ref() {
                    campaigns.insert(campaign.id.clone(), ());
                }
            }

            StrategySummary {
                strategy,
                cases: group_cases.len(),
                campaigns: campaigns.len(),
                passed_required_cases,
                target_hits,
                total_points,
                total_elapsed_ms,
                current_witness_cases,
                current_witness_lag_total,
                current_lag_score: lag_score(current_witness_cases, current_witness_lag_total),
                best_known_witness_cases,
                best_known_witness_lag_total,
                best_known_lag_score: lag_score(
                    best_known_witness_cases,
                    best_known_witness_lag_total,
                ),
                best_known_improvements,
            }
        })
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
    let worker_output_path = env::temp_dir().join(format!(
        "research-harness-worker-{}-{}.json",
        std::process::id(),
        case.id
    ));
    let resolved = resolve_case(case, cases_path).ok();
    let source_dim = resolved
        .as_ref()
        .map(|resolved| resolved.endpoint.source_dim)
        .unwrap_or_else(|| case.a.len());
    let target_dim = resolved
        .as_ref()
        .map(|resolved| resolved.endpoint.target_dim)
        .unwrap_or_else(|| case.b.len());

    let mut child = Command::new(current_exe)
        .arg("--cases")
        .arg(cases_path)
        .arg("--worker-case")
        .arg(&case.id)
        .arg("--worker-output")
        .arg(&worker_output_path)
        .stdout(Stdio::null())
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
            let _ = fs::remove_file(&worker_output_path);
            let reason = stderr_snippet(&output.stderr);
            return Ok(WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "timeout".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: reason.or_else(|| Some(format!("worker exceeded {} ms", case.timeout_ms))),
                result_model: result_model(
                    solver_path_for_dims(source_dim, target_dim),
                    source_dim,
                    target_dim,
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
                    let _ = fs::remove_file(&worker_output_path);
                    return Ok(WorkerCaseResult {
                        id: case.id.clone(),
                        actual_outcome: "panic".to_string(),
                        elapsed_ms: started.elapsed().as_millis(),
                        steps: None,
                        reason: stderr_snippet(&output.stderr)
                            .or_else(|| Some(format!("worker exited with status {status}"))),
                        result_model: result_model(
                            solver_path_for_dims(source_dim, target_dim),
                            source_dim,
                            target_dim,
                            ResultResolutionKind::Panic,
                            None,
                            None,
                            &SearchTelemetry::default(),
                        ),
                        telemetry: SearchTelemetry::default(),
                    });
                }

                let worker_output = fs::read_to_string(&worker_output_path).map_err(|err| {
                    format!(
                        "worker {} did not produce readable output {}: {err}",
                        case.id,
                        worker_output_path.display()
                    )
                })?;
                let _ = fs::remove_file(&worker_output_path);
                let parsed: WorkerCaseResult = serde_json::from_str(worker_output.trim())
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
        "reused_history_sources: {}\n",
        summary.reused_history_sources
    ));
    out.push_str(&format!(
        "required_passes: {}/{}\n",
        summary.fitness.passed_required_cases, summary.fitness.required_cases
    ));
    out.push_str(&format!("target_hits: {}\n", summary.fitness.target_hits));
    out.push_str(&format!("total_points: {}\n", summary.fitness.total_points));
    out.push_str(&format!(
        "current_lag_score: {} across {} witness case(s), total lag {}\n",
        summary.fitness.current_lag_score,
        summary.fitness.current_witness_cases,
        summary.fitness.current_witness_lag_total
    ));
    out.push_str(&format!(
        "best_known_lag_score: {} across {} witness case(s), total lag {}, improvements {}\n",
        summary.fitness.best_known_lag_score,
        summary.fitness.best_known_witness_cases,
        summary.fitness.best_known_witness_lag_total,
        summary.fitness.best_known_improvements
    ));
    out.push_str(&format!(
        "generalized_cases: {}\n",
        summary.fitness.generalized_cases
    ));
    out.push_str(&format!(
        "comparison_groups: {}\n",
        summary.fitness.comparison_groups
    ));
    out.push_str(&format!(
        "campaign_groups: {}\n",
        summary.fitness.campaign_groups
    ));
    out.push_str(&format!(
        "strategy_groups: {}\n",
        summary.fitness.strategy_groups
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
                    "  {}: strategy={} stage_combo={} mode={:?} stage={:?} max_lag={} max_dim={} max_entry={} outcome={} resolution={:?} witness_lag={:?} best_known_lag={:?} improved_best={} points={} elapsed={}ms\n",
                    variant.case_id,
                    variant
                        .campaign
                        .as_ref()
                        .map(|campaign| campaign.strategy.as_str())
                        .unwrap_or("-"),
                    variant.stage_combination,
                    variant.config.search_mode,
                    variant.config.stage,
                    variant.config.max_lag,
                    variant.config.max_intermediate_dim,
                    variant.config.max_entry,
                    variant.actual_outcome,
                    variant.result_model.resolution_kind,
                    variant.result_model.witness_lag,
                    variant.best_known_witness.as_ref().map(|best| best.lag),
                    variant.improved_best_known_witness,
                    variant.points,
                    variant.elapsed_ms,
                ));
            }
        }
        out.push('\n');
    }

    if !summary.campaigns.is_empty() {
        out.push_str("Campaigns\n");
        for campaign in &summary.campaigns {
            out.push_str(&format!(
                "- {}: cases={} current_lag_score={} best_known_lag_score={} points={} elapsed={}ms\n",
                campaign.id,
                campaign.cases,
                campaign.current_lag_score,
                campaign.best_known_lag_score,
                campaign.total_points,
                campaign.total_elapsed_ms,
            ));
            for scheduled_case in &campaign.scheduled_cases {
                out.push_str(&format!(
                    "  order={} {} strategy={} outcome={} current_lag={:?} best_known_lag={:?} improved_best={} points={} elapsed={}ms\n",
                    scheduled_case.schedule_order,
                    scheduled_case.case_id,
                    scheduled_case.strategy,
                    scheduled_case.actual_outcome,
                    scheduled_case.current_witness_lag,
                    scheduled_case.best_known_witness.as_ref().map(|best| best.lag),
                    scheduled_case.improved_best_known_witness,
                    scheduled_case.points,
                    scheduled_case.elapsed_ms,
                ));
            }
        }
        out.push('\n');
    }

    if !summary.strategies.is_empty() {
        out.push_str("Strategies\n");
        for strategy in &summary.strategies {
            out.push_str(&format!(
                "- {}: cases={} campaigns={} passes={} target_hits={} points={} current_lag_score={} best_known_lag_score={} improvements={} elapsed={}ms\n",
                strategy.strategy,
                strategy.cases,
                strategy.campaigns,
                strategy.passed_required_cases,
                strategy.target_hits,
                strategy.total_points,
                strategy.current_lag_score,
                strategy.best_known_lag_score,
                strategy.best_known_improvements,
                strategy.total_elapsed_ms,
            ));
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
            "  endpoints: {}x{} config: mode={:?} stage={:?} max_lag={} max_dim={} max_entry={} timeout={}ms\n",
            case.endpoint.source_dim,
            case.endpoint.target_dim,
            case.config.search_mode,
            case.config.stage,
            case.config.max_lag,
            case.config.max_intermediate_dim,
            case.config.max_entry,
            case.timeout_ms,
        ));
        if case.config.stage == SearchStage::GuidedRefinement {
            out.push_str(&format!(
                "  guided_refinement: max_shortcut_lag={} min_gap={} max_gap={:?} rounds={}\n",
                case.config.guided_refinement.max_shortcut_lag,
                case.config.guided_refinement.min_gap,
                case.config.guided_refinement.max_gap,
                case.config.guided_refinement.rounds,
            ));
        }
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
        if let Some(campaign) = &case.campaign {
            out.push_str(&format!(
                "  campaign: id={} strategy={} order={}\n",
                campaign.id, campaign.strategy, campaign.schedule_order
            ));
        }
        if let Some(endpoint_fixture) = &case.endpoint_fixture {
            out.push_str(&format!("  endpoint_fixture: {}\n", endpoint_fixture));
        }
        if !case.seeded_guide_ids.is_empty() || !case.guide_artifact_paths.is_empty() {
            out.push_str(&format!(
                "  guide_inputs: seeded={:?} artifacts={:?}\n",
                case.seeded_guide_ids, case.guide_artifact_paths
            ));
        }
        if let Some(best_known) = &case.best_known_witness {
            out.push_str(&format!(
                "  best_known_witness: lag={} elapsed={}ms source={} improved_best={}\n",
                best_known.lag,
                best_known.elapsed_ms,
                best_known.source,
                case.improved_best_known_witness,
            ));
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
    use std::path::Path;
    use std::time::{SystemTime, UNIX_EPOCH};

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
                search_mode: SearchMode::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
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
                search_mode: SearchMode::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
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
                search_mode: SearchMode::GraphOnly,
                stage: SearchStage::GuidedRefinement,
                guided_refinement: GuidedRefinementConfig {
                    max_shortcut_lag: 1,
                    min_gap: 2,
                    max_gap: Some(2),
                    rounds: 1,
                },
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
                search_mode: SearchMode::GraphOnly,
                stage: SearchStage::GuidedRefinement,
                guided_refinement: GuidedRefinementConfig {
                    max_shortcut_lag: 1,
                    min_gap: 2,
                    max_gap: Some(2),
                    rounds: 1,
                },
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
        };

        let result = run_case(&case, Path::new("research/cases.json"));
        assert_eq!(result.actual_outcome, "equivalent");
        assert_eq!(result.steps, Some(1));
        assert_eq!(result.telemetry.guide_artifacts_accepted, 1);
        assert_eq!(result.telemetry.guided_segments_improved, 1);
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
                endpoint_fixture: None,
                seeded_guide_ids: vec![],
                guide_artifact_paths: vec![],
                endpoint: endpoint.clone(),
                config: JsonSearchConfig {
                    max_lag: 1,
                    max_intermediate_dim: 2,
                    max_entry: 1,
                    search_mode: SearchMode::Mixed,
                    stage: SearchStage::EndpointSearch,
                    guided_refinement: GuidedRefinementConfig::default(),
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
            },
            CaseSummary {
                id: "case-b".to_string(),
                description: "B".to_string(),
                campaign: Some(CampaignConfig {
                    id: "identity".to_string(),
                    strategy: "graph-only".to_string(),
                    schedule_order: 20,
                }),
                endpoint_fixture: None,
                seeded_guide_ids: vec![],
                guide_artifact_paths: vec![],
                endpoint,
                config: JsonSearchConfig {
                    max_lag: 2,
                    max_intermediate_dim: 2,
                    max_entry: 2,
                    search_mode: SearchMode::GraphOnly,
                    stage: SearchStage::EndpointSearch,
                    guided_refinement: GuidedRefinementConfig::default(),
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
            },
        ];

        let comparisons = build_comparison_summaries(&cases);
        assert_eq!(comparisons.len(), 1);
        assert_eq!(comparisons[0].variants.len(), 2);
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
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            endpoint: endpoint.clone(),
            config: JsonSearchConfig {
                max_lag: 6,
                max_intermediate_dim: 3,
                max_entry: 6,
                search_mode: SearchMode::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
            },
            actual_outcome: "unknown".to_string(),
            allowed_outcomes: vec!["equivalent".to_string(), "unknown".to_string()],
            target_outcome: Some("equivalent".to_string()),
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
            endpoint_fixture: None,
            seeded_guide_ids: vec![],
            guide_artifact_paths: vec![],
            endpoint: endpoint.clone(),
            config: JsonSearchConfig {
                max_lag: 4,
                max_intermediate_dim: 2,
                max_entry: 4,
                search_mode: SearchMode::Mixed,
                stage: SearchStage::EndpointSearch,
                guided_refinement: GuidedRefinementConfig::default(),
            },
            actual_outcome: "equivalent".to_string(),
            allowed_outcomes: vec!["equivalent".to_string()],
            target_outcome: Some("equivalent".to_string()),
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
