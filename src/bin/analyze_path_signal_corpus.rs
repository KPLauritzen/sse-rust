use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::path_scoring::{candidate_score_specs, new_summaries, rank_target, ScoreSummary};
use sse_core::search::execute_search_request_and_observer;
use sse_core::search_observer::{SearchEdgeStatus, SearchEvent, SearchObserver};
use sse_core::types::{
    FrontierMode, GuideArtifactPayload, GuidedRefinementConfig, MoveFamilyPolicy, SearchConfig,
    SearchDirection, SearchRequest, SearchRunResult, SearchStage, ShortcutSearchConfig,
};

fn main() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    let pair_catalog = load_pair_catalog(&cli)?;
    let source_paths = load_source_paths(&cli, &pair_catalog)?;
    let path_cases = derive_path_cases(&source_paths, &cli);
    let path_case_count = path_cases.len();
    let endpoint_cases = load_research_cases(&cli, &pair_catalog)?;
    let endpoint_case_count = endpoint_cases.len();
    let mut cases = path_cases;
    cases.extend(endpoint_cases);
    if cases.is_empty() {
        return Err(
            "no analysis cases were loaded; pass --guide-artifacts, --path-db, or --cases"
                .to_string(),
        );
    }

    let specs = candidate_score_specs();
    let mut summaries = new_summaries(&specs);
    let mut analyzed_cases = Vec::new();
    let mut solved_cases = 0usize;
    let mut unmatched_cases = 0usize;
    let mut total_solution_nodes = 0usize;
    let mut total_ranked_nodes = 0usize;

    for case in cases {
        match analyze_case(&case, &specs, &mut summaries)? {
            Some(analysis) => {
                solved_cases += 1;
                total_solution_nodes += analysis.solution_nodes;
                total_ranked_nodes += analysis.ranked_nodes;
                analyzed_cases.push(analysis);
            }
            None => {
                unmatched_cases += 1;
            }
        }
    }
    let unranked_solved_cases = analyzed_cases
        .iter()
        .filter(|analysis| analysis.solution_nodes > 0 && analysis.ranked_nodes == 0)
        .count();

    println!("Signal corpus analysis");
    println!(
        "  source_paths={} path_segment_cases={} endpoint_cases={} solved_cases={} unsolved_cases={} unranked_solved_cases={} ranked_solution_nodes={}/{}",
        source_paths.len(),
        path_case_count,
        endpoint_case_count,
        solved_cases,
        unmatched_cases,
        unranked_solved_cases,
        total_ranked_nodes,
        total_solution_nodes
    );
    println!(
        "  config: min_gap={} max_gap={} max_cases={} max_endpoint_dim={} max_intermediate_dim={} max_entry={} search_mode={:?}",
        cli.min_gap,
        cli.max_gap,
        cli.max_cases,
        cli.max_endpoint_dim,
        cli.max_intermediate_dim,
        cli.max_entry,
        cli.search_mode
    );
    println!(
        "          explicit_search_mode_override={}",
        cli.search_mode_explicit
    );
    if !cli.benchmark_roles.is_empty() {
        println!(
            "          benchmark_roles={}",
            cli.benchmark_roles.join(",")
        );
    }
    println!("  summary:");
    for (name, summary) in &summaries {
        println!(
            "    {:<24} n={:<3} mean_pct={:>6.2}% worst_pct={:>6.2}% top1={:<3} top5%={:<3} top10%={:<3}",
            name,
            summary.seen,
            100.0 * summary.mean_percentile(),
            100.0 * summary.worst_percentile,
            summary.top_1,
            summary.top_5_pct,
            summary.top_10_pct
        );
    }

    println!("  cases:");
    for analysis in analyzed_cases.iter().take(12) {
        println!(
            "    {} budget_lag={} solved_lag={} ranked={}/{} layers={}",
            analysis.label,
            analysis.budget_lag,
            analysis.solved_lag,
            analysis.ranked_nodes,
            analysis.solution_nodes,
            analysis.layer_count
        );
    }
    if analyzed_cases.len() > 12 {
        println!("    ... {} more case(s)", analyzed_cases.len() - 12);
    }

    write_layer_contrast_artifact(
        &cli,
        source_paths.len(),
        path_case_count,
        endpoint_case_count,
        solved_cases,
        unmatched_cases,
        unranked_solved_cases,
        total_ranked_nodes,
        total_solution_nodes,
        &analyzed_cases,
    )?;

    Ok(())
}

#[derive(Debug)]
struct Cli {
    guide_artifact_paths: Vec<PathBuf>,
    path_dbs: Vec<PathBuf>,
    cases_paths: Vec<PathBuf>,
    case_ids: Vec<String>,
    campaign_ids: Vec<String>,
    benchmark_roles: Vec<String>,
    min_gap: usize,
    max_gap: usize,
    max_cases: usize,
    max_endpoint_dim: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    search_mode: MoveFamilyPolicy,
    search_mode_explicit: bool,
    witness_manifest_path: Option<PathBuf>,
    family_benchmark_path: Option<PathBuf>,
    emit_layer_contrasts_path: Option<PathBuf>,
}

#[derive(Clone)]
struct SourcePath {
    label: String,
    matrices: Vec<DynMatrix>,
    pair_metadata: PairMetadata,
}

#[derive(Clone)]
struct SegmentCase {
    label: String,
    budget_lag: usize,
    source: DynMatrix,
    target: DynMatrix,
    config: SearchConfig,
    stage: SearchStage,
    guided_refinement: GuidedRefinementConfig,
    shortcut_search: ShortcutSearchConfig,
    pair_metadata: PairMetadata,
    contrast_source_kind: ContrastSourceKind,
}

#[derive(Clone, Debug, Serialize)]
struct CaseAnalysis {
    label: String,
    budget_lag: usize,
    solved_lag: usize,
    solution_nodes: usize,
    ranked_nodes: usize,
    layer_count: usize,
    pair_metadata: PairMetadata,
    contrast_source_kind: ContrastSourceKind,
    rankable_layers: Vec<LayerContrast>,
}

#[derive(Clone, Debug, Default)]
struct PairCatalog {
    path_pairs_by_endpoints: HashMap<String, String>,
    family_by_pair_id: HashMap<String, FamilyMetadata>,
    benchmark_pair_by_case_id: HashMap<String, String>,
    benchmark_case_ids_by_role: HashMap<String, BTreeSet<String>>,
    manifest_endpoint_cases_by_case_id: HashMap<String, ManifestEndpointCase>,
}

#[derive(Clone, Debug, Default, Serialize)]
struct PairMetadata {
    #[serde(skip_serializing_if = "Option::is_none")]
    pair_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    evaluation_family_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    benchmark_role: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    benchmark_case_id: Option<String>,
}

#[derive(Clone, Debug)]
struct FamilyMetadata {
    evaluation_family_id: String,
    benchmark_role: String,
}

#[derive(Clone, Debug)]
struct ManifestEndpointCase {
    pair_id: String,
    source: DynMatrix,
    target: DynMatrix,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ContrastSourceKind {
    PathSegment,
    EndpointCase,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
enum ContinuationLabel {
    BestContinuation,
    SupportingContinuation,
}

#[derive(Clone, Debug, Serialize)]
struct LayerContrast {
    layer_index: usize,
    direction: SearchDirection,
    layer_size: usize,
    matched_witness_candidates: usize,
    best_remaining_witness_lag: usize,
    dedup_scope_key: String,
    matched_candidates: Vec<CandidateLabel>,
}

#[derive(Clone, Debug, Serialize)]
struct CandidateLabel {
    candidate_key: String,
    continuation_label: ContinuationLabel,
    remaining_witness_lag: Option<usize>,
    solution_path_index: Option<usize>,
}

#[derive(Clone, Debug)]
struct SolutionStateInfo {
    solution_path_index: usize,
    remaining_witness_lag: usize,
}

#[derive(Debug, Serialize)]
struct LayerContrastArtifact {
    schema_version: usize,
    artifact_kind: &'static str,
    label_contract: &'static str,
    witness_manifest_path: Option<PathBuf>,
    family_benchmark_path: Option<PathBuf>,
    config: ArtifactConfig,
    summary: ArtifactSummary,
    cases: Vec<ArtifactCaseAnalysis>,
}

#[derive(Debug, Serialize)]
struct ArtifactConfig {
    guide_artifact_paths: Vec<PathBuf>,
    path_dbs: Vec<PathBuf>,
    cases_paths: Vec<PathBuf>,
    case_ids: Vec<String>,
    campaign_ids: Vec<String>,
    benchmark_roles: Vec<String>,
    min_gap: usize,
    max_gap: usize,
    max_cases: usize,
    max_endpoint_dim: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    search_mode: MoveFamilyPolicy,
}

#[derive(Debug, Serialize)]
struct ArtifactSummary {
    source_paths: usize,
    path_segment_cases: usize,
    endpoint_cases: usize,
    solved_cases: usize,
    unsolved_cases: usize,
    unranked_solved_cases: usize,
    ranked_solution_nodes: usize,
    solution_nodes: usize,
    exported_cases: usize,
    exported_rankable_cases: usize,
    exported_rankable_layers: usize,
    exported_matched_candidates: usize,
    exported_families: Vec<String>,
}

#[derive(Debug, Serialize)]
struct ArtifactCaseAnalysis {
    label: String,
    budget_lag: usize,
    solved_lag: usize,
    solution_nodes: usize,
    ranked_nodes: usize,
    layer_count: usize,
    pair_metadata: PairMetadata,
    contrast_source_kind: ContrastSourceKind,
    rankable_layers: Vec<ArtifactLayerContrast>,
}

#[derive(Debug, Serialize)]
struct ArtifactLayerContrast {
    layer_index: usize,
    direction: SearchDirection,
    layer_size: usize,
    matched_witness_candidates: usize,
    best_remaining_witness_lag: usize,
    dedup_scope_key: String,
    matched_candidates: Vec<ArtifactMatchedCandidate>,
}

#[derive(Debug, Serialize)]
struct ArtifactMatchedCandidate {
    candidate_key: String,
    continuation_label: ContinuationLabel,
    remaining_witness_lag: usize,
    solution_path_index: usize,
}

#[derive(Debug, Deserialize)]
struct WitnessCorpusManifest {
    first_ingestion_slice: WitnessCorpusFirstSlice,
}

#[derive(Debug, Deserialize)]
struct WitnessCorpusFirstSlice {
    #[serde(default)]
    validated_pairs: Vec<WitnessCorpusPair>,
    #[serde(default)]
    endpoint_case_only_pairs: Vec<WitnessCorpusEndpointCasePair>,
}

#[derive(Debug, Deserialize)]
struct WitnessCorpusPair {
    pair_id: String,
    source: ManifestMatrix,
    target: ManifestMatrix,
}

#[derive(Debug, Deserialize)]
struct WitnessCorpusEndpointCasePair {
    pair_id: String,
    case_id: String,
    source: ManifestMatrix,
    target: ManifestMatrix,
}

#[derive(Debug, Deserialize)]
struct ManifestMatrix {
    rows: usize,
    cols: usize,
    data: Vec<u32>,
}

#[derive(Debug, Deserialize)]
struct RankingSignalFamilyBenchmark {
    families: Vec<RankingSignalFamily>,
}

#[derive(Debug, Deserialize)]
struct RankingSignalFamily {
    evaluation_family_id: String,
    benchmark_role: String,
    pairs: Vec<RankingSignalPair>,
}

#[derive(Debug, Deserialize)]
struct RankingSignalPair {
    pair_id: String,
    benchmark_case_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ResearchCaseCorpus {
    cases: Vec<ResearchCase>,
}

#[derive(Clone, Debug, Deserialize)]
struct ResearchCase {
    id: String,
    #[serde(default)]
    a: Vec<Vec<u32>>,
    #[serde(default)]
    b: Vec<Vec<u32>>,
    config: JsonSearchConfig,
    #[serde(default)]
    campaign: Option<CampaignConfig>,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct CampaignConfig {
    id: String,
}

#[derive(Clone, Debug, Deserialize)]
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

#[derive(Default)]
struct LayerCollector {
    layers: Vec<ObservedLayer>,
}

struct ObservedLayer {
    direction: SearchDirection,
    candidates: Vec<DynMatrix>,
}

impl PairCatalog {
    fn resolve_path(&self, source: Option<&DynMatrix>, target: Option<&DynMatrix>) -> PairMetadata {
        let Some(source) = source else {
            return PairMetadata::default();
        };
        let Some(target) = target else {
            return PairMetadata::default();
        };
        let Some(pair_id) = self
            .path_pairs_by_endpoints
            .get(&endpoint_pair_key(source, target))
            .cloned()
        else {
            return PairMetadata::default();
        };
        self.build_pair_metadata(pair_id, None)
    }

    fn resolve_case(&self, pair_id: &str) -> PairMetadata {
        self.build_pair_metadata(pair_id.to_string(), Some(pair_id.to_string()))
    }

    fn resolve_research_case(&self, case_id: &str) -> PairMetadata {
        if let Some(pair_id) = self.benchmark_pair_by_case_id.get(case_id) {
            return self.build_pair_metadata(pair_id.clone(), Some(case_id.to_string()));
        }
        self.resolve_case(case_id)
    }

    fn build_pair_metadata(
        &self,
        pair_id: String,
        benchmark_case_id: Option<String>,
    ) -> PairMetadata {
        let family = self.family_by_pair_id.get(&pair_id);
        PairMetadata {
            pair_id: Some(pair_id),
            evaluation_family_id: family.map(|family| family.evaluation_family_id.clone()),
            benchmark_role: family.map(|family| family.benchmark_role.clone()),
            benchmark_case_id,
        }
    }
}

impl SearchObserver for LayerCollector {
    fn on_event(&mut self, event: &SearchEvent) {
        let SearchEvent::Layer(edges) = event else {
            return;
        };
        if edges.is_empty() {
            return;
        }

        let direction = edges[0].direction;
        let mut seen = HashSet::new();
        let mut candidates = Vec::new();
        for edge in edges {
            if edge.status == SearchEdgeStatus::SeenCollision {
                continue;
            }
            if !edge.enqueued && edge.status != SearchEdgeStatus::ExactMeet {
                continue;
            }
            if seen.insert(edge.to_canonical.clone()) {
                candidates.push(edge.to_canonical.clone());
            }
        }
        if !candidates.is_empty() {
            self.layers.push(ObservedLayer {
                direction,
                candidates,
            });
        }
    }
}

fn default_move_family_policy() -> MoveFamilyPolicy {
    MoveFamilyPolicy::Mixed
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut cli = Cli {
        guide_artifact_paths: Vec::new(),
        path_dbs: Vec::new(),
        cases_paths: Vec::new(),
        case_ids: Vec::new(),
        campaign_ids: Vec::new(),
        benchmark_roles: Vec::new(),
        min_gap: 2,
        max_gap: 4,
        max_cases: 48,
        max_endpoint_dim: 3,
        max_intermediate_dim: 5,
        max_entry: 6,
        search_mode: MoveFamilyPolicy::Mixed,
        search_mode_explicit: false,
        witness_manifest_path: None,
        family_benchmark_path: None,
        emit_layer_contrasts_path: None,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--guide-artifacts" => {
                cli.guide_artifact_paths.push(PathBuf::from(
                    args.next().ok_or("--guide-artifacts requires a path")?,
                ));
            }
            "--path-db" => {
                cli.path_dbs.push(PathBuf::from(
                    args.next().ok_or("--path-db requires a path")?,
                ));
            }
            "--cases" => {
                cli.cases_paths
                    .push(PathBuf::from(args.next().ok_or("--cases requires a path")?));
            }
            "--case-id" => {
                cli.case_ids
                    .push(args.next().ok_or("--case-id requires a value")?);
            }
            "--campaign-id" => {
                cli.campaign_ids
                    .push(args.next().ok_or("--campaign-id requires a value")?);
            }
            "--benchmark-role" => {
                cli.benchmark_roles
                    .push(args.next().ok_or("--benchmark-role requires a value")?);
            }
            "--min-gap" => {
                cli.min_gap = args
                    .next()
                    .ok_or("--min-gap requires a value")?
                    .parse()
                    .map_err(|_| "invalid --min-gap".to_string())?;
            }
            "--max-gap" => {
                cli.max_gap = args
                    .next()
                    .ok_or("--max-gap requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-gap".to_string())?;
            }
            "--max-cases" => {
                cli.max_cases = args
                    .next()
                    .ok_or("--max-cases requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-cases".to_string())?;
            }
            "--max-endpoint-dim" => {
                cli.max_endpoint_dim = args
                    .next()
                    .ok_or("--max-endpoint-dim requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-endpoint-dim".to_string())?;
            }
            "--max-intermediate-dim" => {
                cli.max_intermediate_dim = args
                    .next()
                    .ok_or("--max-intermediate-dim requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-intermediate-dim".to_string())?;
            }
            "--max-entry" => {
                cli.max_entry = args
                    .next()
                    .ok_or("--max-entry requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-entry".to_string())?;
            }
            "--search-mode" => {
                let value = args.next().ok_or("--search-mode requires a value")?;
                cli.search_mode = match value.as_str() {
                    "mixed" => MoveFamilyPolicy::Mixed,
                    "graph-plus-structured" | "graph_plus_structured" => {
                        MoveFamilyPolicy::GraphPlusStructured
                    }
                    "graph-only" | "graph_only" => MoveFamilyPolicy::GraphOnly,
                    _ => return Err(format!("unknown search mode: {value}")),
                };
                cli.search_mode_explicit = true;
            }
            "--witness-manifest" => {
                cli.witness_manifest_path = Some(PathBuf::from(
                    args.next().ok_or("--witness-manifest requires a path")?,
                ));
            }
            "--family-benchmark" => {
                cli.family_benchmark_path = Some(PathBuf::from(
                    args.next().ok_or("--family-benchmark requires a path")?,
                ));
            }
            "--emit-layer-contrasts" => {
                cli.emit_layer_contrasts_path = Some(PathBuf::from(
                    args.next()
                        .ok_or("--emit-layer-contrasts requires a path")?,
                ));
            }
            "--help" | "-h" => {
                return Err(
                    "usage: analyze_path_signal_corpus [options]\n\n\
                     Options:\n\
                       --guide-artifacts PATH   load full-path guide artifacts (repeatable)\n\
                       --path-db PATH           load shortcut paths from a legacy sqlite db (repeatable)\n\
                       --cases PATH             load inline endpoint cases from a research cases json (repeatable)\n\
                       --case-id ID            limit loaded research cases to this case id (repeatable)\n\
                       --campaign-id ID        limit loaded research cases to this campaign id (repeatable)\n\
                       --benchmark-role ROLE   limit loaded research cases to family-benchmark entries\n\
                                             with this benchmark_role and a benchmark_case_id\n\
                       --min-gap N             minimum segment gap to analyze (default: 2)\n\
                       --max-gap N             maximum segment gap to analyze (default: 4)\n\
                       --max-cases N           cap derived segment cases (default: 48)\n\
                       --max-endpoint-dim N    ignore cases with larger endpoints (default: 3)\n\
                       --max-intermediate-dim N\n\
                                              search config bound for derived cases (default: 5)\n\
                       --max-entry N           search config entry bound (default: 6)\n\
                       --search-mode MODE      override move-family policy for both derived and endpoint cases:\n\
                                             mixed | graph-plus-structured | graph-only\n\
                                             default: derived cases use mixed; endpoint cases keep case config\n\
                       --witness-manifest PATH resolve full-path sources to durable pair ids\n\
                       --family-benchmark PATH resolve pair ids to evaluation families / roles\n\
                       --emit-layer-contrasts PATH\n\
                                             write rankable within-layer continuation labels as JSON\n\
                     \n\
                     If no inputs are supplied and research/k3-graph-paths.sqlite exists,\n\
                     it is used as the default path source."
                        .to_string(),
                );
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if cli.max_gap < cli.min_gap {
        return Err("--max-gap must be >= --min-gap".to_string());
    }
    if !cli.benchmark_roles.is_empty() && cli.family_benchmark_path.is_none() {
        return Err("--benchmark-role requires --family-benchmark".to_string());
    }
    if cli.emit_layer_contrasts_path.is_some() {
        if cli.witness_manifest_path.is_none() {
            return Err("--emit-layer-contrasts requires --witness-manifest".to_string());
        }
        if cli.family_benchmark_path.is_none() {
            return Err("--emit-layer-contrasts requires --family-benchmark".to_string());
        }
    }

    if cli.guide_artifact_paths.is_empty() && cli.path_dbs.is_empty() && cli.cases_paths.is_empty()
    {
        let default_db = PathBuf::from("research/k3-graph-paths.sqlite");
        if default_db.exists() {
            cli.path_dbs.push(default_db);
        }
    }

    Ok(cli)
}

fn load_pair_catalog(cli: &Cli) -> Result<PairCatalog, String> {
    let mut catalog = PairCatalog::default();
    if let Some(path) = &cli.witness_manifest_path {
        let raw = fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let manifest: WitnessCorpusManifest = serde_json::from_str(&raw)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
        for pair in manifest.first_ingestion_slice.validated_pairs {
            let source = manifest_matrix(&pair.source)?.canonical_perm();
            let target = manifest_matrix(&pair.target)?.canonical_perm();
            catalog
                .path_pairs_by_endpoints
                .insert(endpoint_pair_key(&source, &target), pair.pair_id);
        }
        for pair in manifest.first_ingestion_slice.endpoint_case_only_pairs {
            let source = manifest_matrix(&pair.source)?.canonical_perm();
            let target = manifest_matrix(&pair.target)?.canonical_perm();
            let case_id = pair.case_id;
            let pair_id = pair.pair_id;
            if let Some(existing) = catalog.manifest_endpoint_cases_by_case_id.insert(
                case_id.clone(),
                ManifestEndpointCase {
                    pair_id: pair_id.clone(),
                    source,
                    target,
                },
            ) {
                return Err(format!(
                    "duplicate witness manifest endpoint_case_only_pairs case_id {case_id}: {} and {pair_id}",
                    existing.pair_id
                ));
            }
        }
    }
    if let Some(path) = &cli.family_benchmark_path {
        let raw = fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let manifest: RankingSignalFamilyBenchmark = serde_json::from_str(&raw)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
        for family in manifest.families {
            let metadata = FamilyMetadata {
                evaluation_family_id: family.evaluation_family_id,
                benchmark_role: family.benchmark_role,
            };
            for pair in family.pairs {
                catalog
                    .family_by_pair_id
                    .insert(pair.pair_id.clone(), metadata.clone());
                if let Some(benchmark_case_id) = pair.benchmark_case_id {
                    if let Some(existing_pair_id) = catalog
                        .benchmark_pair_by_case_id
                        .insert(benchmark_case_id.clone(), pair.pair_id.clone())
                    {
                        return Err(format!(
                            "duplicate family benchmark benchmark_case_id {benchmark_case_id}: {existing_pair_id} and {}",
                            pair.pair_id
                        ));
                    }
                    catalog
                        .benchmark_case_ids_by_role
                        .entry(metadata.benchmark_role.clone())
                        .or_default()
                        .insert(benchmark_case_id);
                }
            }
        }
    }

    Ok(catalog)
}

fn load_source_paths(cli: &Cli, pair_catalog: &PairCatalog) -> Result<Vec<SourcePath>, String> {
    let mut paths = Vec::new();
    for path in &cli.guide_artifact_paths {
        paths.extend(load_paths_from_guide_artifacts(path, pair_catalog)?);
    }
    for path in &cli.path_dbs {
        paths.extend(load_paths_from_sqlite(path, pair_catalog)?);
    }

    let mut seen = HashSet::new();
    paths.retain(|path| seen.insert(path_signature(&path.matrices)));
    Ok(paths)
}

fn load_paths_from_guide_artifacts(
    path: &Path,
    pair_catalog: &PairCatalog,
) -> Result<Vec<SourcePath>, String> {
    let mut paths = Vec::new();
    for artifact in load_guide_artifacts_from_path(path)? {
        let GuideArtifactPayload::FullPath { path } = artifact.payload;
        let label = artifact
            .artifact_id
            .or(artifact.provenance.label)
            .unwrap_or_else(|| "guide_artifact_path".to_string());
        let matrices = canonicalize_path(&path.matrices);
        paths.push(SourcePath {
            label,
            pair_metadata: pair_catalog.resolve_path(matrices.first(), matrices.last()),
            matrices,
        });
    }
    Ok(paths)
}

fn load_paths_from_sqlite(
    path: &Path,
    pair_catalog: &PairCatalog,
) -> Result<Vec<SourcePath>, String> {
    let conn = Connection::open(path)
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.guide_label, m.step_index, x.rows, x.cols, x.data_json
             FROM shortcut_path_results r
             JOIN shortcut_path_matrices m ON m.result_id = r.id
             JOIN matrices x ON x.id = m.matrix_id
             ORDER BY r.id, m.step_index",
        )
        .map_err(|err| format!("failed to query {}: {err}", path.display()))?;
    let mut rows = stmt
        .query([])
        .map_err(|err| format!("failed to iterate {}: {err}", path.display()))?;

    let mut paths = Vec::new();
    let mut current_id = None;
    let mut current_label = String::new();
    let mut current_matrices = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read sqlite row from {}: {err}", path.display()))?
    {
        let result_id: i64 = row.get(0).map_err(sqlite_value_err)?;
        let guide_label: String = row.get(1).map_err(sqlite_value_err)?;
        let rows_count: i64 = row.get(3).map_err(sqlite_value_err)?;
        let cols_count: i64 = row.get(4).map_err(sqlite_value_err)?;
        let data_json: String = row.get(5).map_err(sqlite_value_err)?;
        let rows = serde_json::from_str::<Vec<Vec<u32>>>(&data_json)
            .map_err(|err| format!("failed to parse matrix JSON in {}: {err}", path.display()))?;
        let data = rows.into_iter().flatten().collect::<Vec<_>>();
        let matrix =
            DynMatrix::new(rows_count as usize, cols_count as usize, data).canonical_perm();

        if current_id != Some(result_id) {
            if let Some(prev_id) = current_id {
                let matrices = std::mem::take(&mut current_matrices);
                paths.push(SourcePath {
                    label: format!("sqlite:{}:{}", prev_id, current_label),
                    pair_metadata: pair_catalog.resolve_path(matrices.first(), matrices.last()),
                    matrices,
                });
            }
            current_id = Some(result_id);
            current_label = guide_label;
        }
        current_matrices.push(matrix);
    }

    if let Some(result_id) = current_id {
        let matrices = current_matrices;
        paths.push(SourcePath {
            label: format!("sqlite:{}:{}", result_id, current_label),
            pair_metadata: pair_catalog.resolve_path(matrices.first(), matrices.last()),
            matrices,
        });
    }

    Ok(paths)
}

fn sqlite_value_err(err: rusqlite::Error) -> String {
    err.to_string()
}

fn canonicalize_path(path: &[DynMatrix]) -> Vec<DynMatrix> {
    path.iter().map(DynMatrix::canonical_perm).collect()
}

fn load_research_cases(cli: &Cli, pair_catalog: &PairCatalog) -> Result<Vec<SegmentCase>, String> {
    let mut loaded = Vec::new();
    let requested_case_ids = cli.case_ids.iter().cloned().collect::<BTreeSet<_>>();
    let requested_campaign_ids = cli.campaign_ids.iter().cloned().collect::<BTreeSet<_>>();
    let requested_benchmark_case_ids = selected_benchmark_case_ids(
        &cli.benchmark_roles,
        &pair_catalog.benchmark_case_ids_by_role,
    )?;
    validate_requested_manifest_catalog_consistency(
        pair_catalog,
        requested_benchmark_case_ids.as_ref(),
    )?;
    let mut loaded_case_ids = BTreeSet::new();

    for path in &cli.cases_paths {
        let raw = fs::read_to_string(path)
            .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
        let corpus: ResearchCaseCorpus = serde_json::from_str(&raw)
            .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;

        for case in corpus.cases {
            if !matches_research_case_filters(
                &case,
                &requested_case_ids,
                &requested_campaign_ids,
                requested_benchmark_case_ids.as_ref(),
            ) {
                continue;
            }
            if case.a.is_empty() || case.b.is_empty() {
                return Err(format!(
                    "research case {} in {} must define inline square endpoints",
                    case.id,
                    path.display()
                ));
            }

            let source = case_matrix(&case.a)?.canonical_perm();
            let target = case_matrix(&case.b)?.canonical_perm();
            validate_manifest_endpoint_case(pair_catalog, &case.id, &source, &target)?;
            loaded.push(SegmentCase {
                label: case.id.clone(),
                budget_lag: case.config.max_lag,
                source,
                target,
                config: SearchConfig {
                    max_lag: case.config.max_lag,
                    max_intermediate_dim: case.config.max_intermediate_dim,
                    max_entry: case.config.max_entry,
                    frontier_mode: case.config.frontier_mode,
                    move_family_policy: effective_endpoint_move_family_policy(
                        case.config.move_family_policy,
                        cli,
                    ),
                    beam_width: case.config.beam_width,
                    beam_bfs_handoff_depth: case.config.beam_bfs_handoff_depth,
                    beam_bfs_handoff_deferred_cap: case.config.beam_bfs_handoff_deferred_cap,
                },
                stage: case.config.stage,
                guided_refinement: case.config.guided_refinement,
                shortcut_search: case.config.shortcut_search,
                pair_metadata: pair_catalog.resolve_research_case(&case.id),
                contrast_source_kind: ContrastSourceKind::EndpointCase,
            });
            loaded_case_ids.insert(case.id);
        }
    }

    if let Some(requested_benchmark_case_ids) = requested_benchmark_case_ids {
        let missing = requested_benchmark_case_ids
            .difference(&loaded_case_ids)
            .cloned()
            .collect::<Vec<_>>();
        if !missing.is_empty() {
            return Err(format!(
                "requested benchmark cases were not found in the loaded research corpora: {}",
                missing.join(", ")
            ));
        }
    }

    Ok(loaded)
}

fn effective_endpoint_move_family_policy(
    case_policy: MoveFamilyPolicy,
    cli: &Cli,
) -> MoveFamilyPolicy {
    if cli.search_mode_explicit {
        cli.search_mode
    } else {
        case_policy
    }
}

fn matches_research_case_filters(
    case: &ResearchCase,
    requested_case_ids: &BTreeSet<String>,
    requested_campaign_ids: &BTreeSet<String>,
    requested_benchmark_case_ids: Option<&BTreeSet<String>>,
) -> bool {
    if !requested_case_ids.is_empty() && !requested_case_ids.contains(&case.id) {
        return false;
    }
    if let Some(requested_benchmark_case_ids) = requested_benchmark_case_ids {
        if !requested_benchmark_case_ids.contains(&case.id) {
            return false;
        }
    }
    if requested_campaign_ids.is_empty() {
        return true;
    }
    case.campaign
        .as_ref()
        .map(|campaign| requested_campaign_ids.contains(&campaign.id))
        .unwrap_or(false)
}

fn selected_benchmark_case_ids(
    requested_roles: &[String],
    benchmark_case_ids_by_role: &HashMap<String, BTreeSet<String>>,
) -> Result<Option<BTreeSet<String>>, String> {
    if requested_roles.is_empty() {
        return Ok(None);
    }

    let mut selected = BTreeSet::new();
    for role in requested_roles {
        let Some(case_ids) = benchmark_case_ids_by_role.get(role) else {
            let mut known_roles = benchmark_case_ids_by_role
                .keys()
                .cloned()
                .collect::<Vec<_>>();
            known_roles.sort();
            return Err(format!(
                "unknown --benchmark-role {role}; known roles with benchmark_case_id entries: {}",
                known_roles.join(", ")
            ));
        };
        selected.extend(case_ids.iter().cloned());
    }

    Ok(Some(selected))
}

fn validate_requested_manifest_catalog_consistency(
    catalog: &PairCatalog,
    requested_benchmark_case_ids: Option<&BTreeSet<String>>,
) -> Result<(), String> {
    let Some(requested_benchmark_case_ids) = requested_benchmark_case_ids else {
        return Ok(());
    };
    for case_id in requested_benchmark_case_ids {
        let Some(pair_id) = catalog.benchmark_pair_by_case_id.get(case_id) else {
            return Err(format!(
                "requested benchmark case {case_id} is missing from the family benchmark catalog"
            ));
        };
        let Some(manifest_case) = catalog.manifest_endpoint_cases_by_case_id.get(case_id) else {
            return Err(format!(
                "family benchmark case {case_id} for pair {pair_id} is missing from witness manifest endpoint_case_only_pairs"
            ));
        };
        if manifest_case.pair_id != *pair_id {
            return Err(format!(
                "family benchmark case {case_id} points to pair {pair_id}, but witness manifest maps it to {}",
                manifest_case.pair_id
            ));
        }
    }
    Ok(())
}

fn validate_manifest_endpoint_case(
    pair_catalog: &PairCatalog,
    case_id: &str,
    source: &DynMatrix,
    target: &DynMatrix,
) -> Result<(), String> {
    let Some(manifest_case) = pair_catalog.manifest_endpoint_cases_by_case_id.get(case_id) else {
        return Ok(());
    };
    if endpoint_pair_key(source, target)
        != endpoint_pair_key(&manifest_case.source, &manifest_case.target)
    {
        return Err(format!(
            "research case {case_id} endpoints do not match witness manifest pair {}",
            manifest_case.pair_id
        ));
    }
    Ok(())
}

fn derive_path_cases(paths: &[SourcePath], cli: &Cli) -> Vec<SegmentCase> {
    let mut cases = Vec::new();
    let mut seen = BTreeSet::new();

    for path in paths {
        for start in 0..path.matrices.len().saturating_sub(1) {
            let upper = (start + cli.max_gap).min(path.matrices.len() - 1);
            for end in start + cli.min_gap..=upper {
                let source = path.matrices[start].clone();
                let target = path.matrices[end].clone();
                if source.rows > cli.max_endpoint_dim || target.rows > cli.max_endpoint_dim {
                    continue;
                }
                let key = format!("{}=>{}", matrix_key(&source), matrix_key(&target));
                if !seen.insert(key) {
                    continue;
                }
                cases.push(SegmentCase {
                    label: format!("{} [{}..{}]", path.label, start, end),
                    budget_lag: end - start,
                    source,
                    target,
                    config: SearchConfig {
                        max_lag: end - start,
                        max_intermediate_dim: cli.max_intermediate_dim,
                        max_entry: cli.max_entry,
                        frontier_mode: FrontierMode::Bfs,
                        move_family_policy: cli.search_mode,
                        beam_width: None,
                        beam_bfs_handoff_depth: None,
                        beam_bfs_handoff_deferred_cap: None,
                    },
                    stage: SearchStage::EndpointSearch,
                    guided_refinement: GuidedRefinementConfig::default(),
                    shortcut_search: ShortcutSearchConfig::default(),
                    pair_metadata: path.pair_metadata.clone(),
                    contrast_source_kind: ContrastSourceKind::PathSegment,
                });
            }
        }
    }

    cases.sort_by(|left, right| {
        left.budget_lag
            .cmp(&right.budget_lag)
            .then(left.source.rows.cmp(&right.source.rows))
            .then(left.target.rows.cmp(&right.target.rows))
            .then(left.label.cmp(&right.label))
    });
    cases.truncate(cli.max_cases);

    cases
}

fn analyze_case(
    case: &SegmentCase,
    specs: &[sse_core::path_scoring::ScoreSpec],
    summaries: &mut BTreeMap<&'static str, ScoreSummary>,
) -> Result<Option<CaseAnalysis>, String> {
    let request = SearchRequest {
        source: case.source.clone(),
        target: case.target.clone(),
        config: case.config.clone(),
        stage: case.stage.clone(),
        guide_artifacts: Vec::new(),
        guided_refinement: case.guided_refinement.clone(),
        shortcut_search: case.shortcut_search.clone(),
    };

    let mut observer = LayerCollector::default();
    println!(
        "  running {} move_family_policy={:?}",
        case.label, case.config.move_family_policy
    );
    let (result, _) = execute_search_request_and_observer(&request, Some(&mut observer))?;
    let SearchRunResult::Equivalent(path) = result else {
        return Ok(None);
    };

    let mut remaining_summary = path
        .matrices
        .iter()
        .skip(1)
        .take(path.matrices.len().saturating_sub(2))
        .map(DynMatrix::canonical_perm)
        .collect::<HashSet<_>>();

    let solution_nodes = remaining_summary.len();
    let mut ranked_nodes = 0usize;
    let mut forward_remaining =
        solution_lookup_by_direction(&path.matrices, SearchDirection::Forward);
    let mut backward_remaining =
        solution_lookup_by_direction(&path.matrices, SearchDirection::Backward);
    let mut rankable_layers = Vec::new();

    for (layer_index, layer) in observer.layers.iter().enumerate() {
        if remaining_summary.is_empty()
            && forward_remaining.is_empty()
            && backward_remaining.is_empty()
        {
            break;
        }
        let endpoint_target = match layer.direction {
            SearchDirection::Forward => &case.target,
            SearchDirection::Backward => &case.source,
        };
        let matched_for_summary = layer
            .candidates
            .iter()
            .filter(|candidate| remaining_summary.contains(*candidate))
            .cloned()
            .collect::<Vec<_>>();
        for candidate in matched_for_summary {
            for spec in specs {
                if let Some(rank) = rank_target(
                    &layer.candidates,
                    &candidate,
                    endpoint_target,
                    endpoint_target,
                    *spec,
                ) {
                    summaries
                        .get_mut(spec.name)
                        .expect("summary exists for every spec")
                        .add(rank);
                }
            }
            remaining_summary.remove(&candidate);
            ranked_nodes += 1;
        }

        let remaining_labels = match layer.direction {
            SearchDirection::Forward => &mut forward_remaining,
            SearchDirection::Backward => &mut backward_remaining,
        };
        let matched_for_labels = layer
            .candidates
            .iter()
            .filter_map(|candidate| {
                remaining_labels
                    .get(&matrix_key(candidate))
                    .cloned()
                    .map(|info| (candidate.clone(), info))
            })
            .collect::<Vec<_>>();
        if matched_for_labels.is_empty() {
            continue;
        }

        let best_remaining_witness_lag = matched_for_labels
            .iter()
            .map(|(_, info)| info.remaining_witness_lag)
            .min()
            .expect("matched layer has at least one witness candidate");
        let dedup_scope_key = case_dedup_scope_key(case, layer.direction);
        let matched_candidates = matched_for_labels
            .iter()
            .map(|(candidate, info)| CandidateLabel {
                candidate_key: matrix_key(candidate),
                continuation_label: if info.remaining_witness_lag == best_remaining_witness_lag {
                    ContinuationLabel::BestContinuation
                } else {
                    ContinuationLabel::SupportingContinuation
                },
                remaining_witness_lag: Some(info.remaining_witness_lag),
                solution_path_index: Some(info.solution_path_index),
            })
            .collect::<Vec<_>>();
        rankable_layers.push(LayerContrast {
            layer_index,
            direction: layer.direction,
            layer_size: layer.candidates.len(),
            matched_witness_candidates: matched_for_labels.len(),
            best_remaining_witness_lag,
            dedup_scope_key,
            matched_candidates,
        });
        for (candidate, _) in matched_for_labels {
            remaining_labels.remove(&matrix_key(&candidate));
        }
    }

    Ok(Some(CaseAnalysis {
        label: case.label.clone(),
        budget_lag: case.budget_lag,
        solved_lag: path.steps.len(),
        solution_nodes,
        ranked_nodes,
        layer_count: observer.layers.len(),
        pair_metadata: case.pair_metadata.clone(),
        contrast_source_kind: case.contrast_source_kind,
        rankable_layers,
    }))
}

fn write_layer_contrast_artifact(
    cli: &Cli,
    source_paths: usize,
    path_segment_cases: usize,
    endpoint_cases: usize,
    solved_cases: usize,
    unsolved_cases: usize,
    unranked_solved_cases: usize,
    ranked_solution_nodes: usize,
    solution_nodes: usize,
    analyzed_cases: &[CaseAnalysis],
) -> Result<(), String> {
    let Some(path) = &cli.emit_layer_contrasts_path else {
        return Ok(());
    };

    let artifact_cases = analyzed_cases
        .iter()
        .map(compact_case_analysis)
        .collect::<Vec<_>>();
    let exported_rankable_cases = analyzed_cases
        .iter()
        .filter(|case| !case.rankable_layers.is_empty())
        .count();
    let exported_rankable_layers = analyzed_cases
        .iter()
        .map(|case| case.rankable_layers.len())
        .sum::<usize>();
    let exported_matched_candidates = artifact_cases
        .iter()
        .flat_map(|case| case.rankable_layers.iter())
        .map(|layer| layer.matched_candidates.len())
        .sum::<usize>();
    let exported_families = analyzed_cases
        .iter()
        .filter_map(|case| case.pair_metadata.evaluation_family_id.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let artifact = LayerContrastArtifact {
        schema_version: 1,
        artifact_kind: "layer_contrast_signal_corpus",
        label_contract: "within_layer_continuation_quality_v1",
        witness_manifest_path: cli.witness_manifest_path.clone(),
        family_benchmark_path: cli.family_benchmark_path.clone(),
        config: ArtifactConfig {
            guide_artifact_paths: cli.guide_artifact_paths.clone(),
            path_dbs: cli.path_dbs.clone(),
            cases_paths: cli.cases_paths.clone(),
            case_ids: cli.case_ids.clone(),
            campaign_ids: cli.campaign_ids.clone(),
            benchmark_roles: cli.benchmark_roles.clone(),
            min_gap: cli.min_gap,
            max_gap: cli.max_gap,
            max_cases: cli.max_cases,
            max_endpoint_dim: cli.max_endpoint_dim,
            max_intermediate_dim: cli.max_intermediate_dim,
            max_entry: cli.max_entry,
            search_mode: cli.search_mode,
        },
        summary: ArtifactSummary {
            source_paths,
            path_segment_cases,
            endpoint_cases,
            solved_cases,
            unsolved_cases,
            unranked_solved_cases,
            ranked_solution_nodes,
            solution_nodes,
            exported_cases: analyzed_cases.len(),
            exported_rankable_cases,
            exported_rankable_layers,
            exported_matched_candidates,
            exported_families,
        },
        cases: artifact_cases,
    };
    let json = serde_json::to_string_pretty(&artifact)
        .map_err(|err| format!("failed to serialize {}: {err}", path.display()))?;
    fs::write(path, format!("{json}\n"))
        .map_err(|err| format!("failed to write {}: {err}", path.display()))
}

fn compact_case_analysis(case: &CaseAnalysis) -> ArtifactCaseAnalysis {
    ArtifactCaseAnalysis {
        label: case.label.clone(),
        budget_lag: case.budget_lag,
        solved_lag: case.solved_lag,
        solution_nodes: case.solution_nodes,
        ranked_nodes: case.ranked_nodes,
        layer_count: case.layer_count,
        pair_metadata: case.pair_metadata.clone(),
        contrast_source_kind: case.contrast_source_kind,
        rankable_layers: case
            .rankable_layers
            .iter()
            .map(compact_layer_contrast)
            .collect(),
    }
}

fn compact_layer_contrast(layer: &LayerContrast) -> ArtifactLayerContrast {
    ArtifactLayerContrast {
        layer_index: layer.layer_index,
        direction: layer.direction,
        layer_size: layer.layer_size,
        matched_witness_candidates: layer.matched_witness_candidates,
        best_remaining_witness_lag: layer.best_remaining_witness_lag,
        dedup_scope_key: layer.dedup_scope_key.clone(),
        matched_candidates: layer
            .matched_candidates
            .iter()
            .map(|candidate| ArtifactMatchedCandidate {
                candidate_key: candidate.candidate_key.clone(),
                continuation_label: candidate.continuation_label,
                remaining_witness_lag: candidate
                    .remaining_witness_lag
                    .expect("matched candidate should carry remaining lag"),
                solution_path_index: candidate
                    .solution_path_index
                    .expect("matched candidate should carry a solution-path index"),
            })
            .collect(),
    }
}

fn manifest_matrix(matrix: &ManifestMatrix) -> Result<DynMatrix, String> {
    if matrix.rows == 0 || matrix.cols == 0 {
        return Err("manifest matrix must have positive dimensions".to_string());
    }
    if matrix.data.len() != matrix.rows * matrix.cols {
        return Err("manifest matrix data length does not match dimensions".to_string());
    }
    Ok(DynMatrix::new(
        matrix.rows,
        matrix.cols,
        matrix.data.clone(),
    ))
}

fn solution_lookup_by_direction(
    path: &[DynMatrix],
    direction: SearchDirection,
) -> HashMap<String, SolutionStateInfo> {
    let mut lookup = HashMap::new();
    for (index, matrix) in path
        .iter()
        .enumerate()
        .skip(1)
        .take(path.len().saturating_sub(2))
    {
        let canonical = matrix.canonical_perm();
        let remaining_witness_lag = match direction {
            SearchDirection::Forward => path.len() - 1 - index,
            SearchDirection::Backward => index,
        };
        let key = matrix_key(&canonical);
        let info = SolutionStateInfo {
            solution_path_index: index,
            remaining_witness_lag,
        };
        lookup
            .entry(key)
            .and_modify(|existing: &mut SolutionStateInfo| {
                if info.remaining_witness_lag < existing.remaining_witness_lag {
                    *existing = info.clone();
                }
            })
            .or_insert(info);
    }
    lookup
}

fn endpoint_pair_key(source: &DynMatrix, target: &DynMatrix) -> String {
    format!("{}=>{}", matrix_key(source), matrix_key(target))
}

fn case_dedup_scope_key(case: &SegmentCase, direction: SearchDirection) -> String {
    let pair_scope = case
        .pair_metadata
        .pair_id
        .clone()
        .unwrap_or_else(|| endpoint_pair_key(&case.source, &case.target));
    let case_scope = case
        .pair_metadata
        .benchmark_case_id
        .clone()
        .unwrap_or_else(|| case.label.clone());
    format!("{pair_scope}|{case_scope}|{}", direction_label(direction))
}

fn direction_label(direction: SearchDirection) -> &'static str {
    match direction {
        SearchDirection::Forward => "forward",
        SearchDirection::Backward => "backward",
    }
}

fn path_signature(path: &[DynMatrix]) -> String {
    path.iter().map(matrix_key).collect::<Vec<_>>().join("|")
}

fn matrix_key(matrix: &DynMatrix) -> String {
    format!(
        "{}x{}:{}",
        matrix.rows,
        matrix.cols,
        matrix
            .data
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
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

#[cfg(test)]
mod tests {
    use super::{
        case_matrix, effective_endpoint_move_family_policy, matches_research_case_filters,
        parse_cli, selected_benchmark_case_ids, validate_requested_manifest_catalog_consistency,
        CampaignConfig, Cli, FamilyMetadata, JsonSearchConfig, ManifestEndpointCase, PairCatalog,
        ResearchCase,
    };
    use sse_core::matrix::DynMatrix;
    use sse_core::types::MoveFamilyPolicy;
    use std::collections::{BTreeSet, HashMap};

    #[test]
    fn case_matrix_rejects_non_square_input() {
        let err = case_matrix(&[vec![1, 2], vec![3]]).expect_err("matrix should be rejected");
        assert_eq!(err, "matrix must be square");
    }

    #[test]
    fn research_case_filters_apply_case_and_campaign_ids() {
        let requested_case_ids = BTreeSet::from(["keep_me".to_string()]);
        let requested_campaign_ids = BTreeSet::from(["non_brix".to_string()]);
        let matching = ResearchCase {
            id: "keep_me".to_string(),
            a: vec![vec![1]],
            b: vec![vec![1]],
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 1,
                max_entry: 1,
                frontier_mode: Default::default(),
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: super::default_move_family_policy(),
                stage: Default::default(),
                guided_refinement: Default::default(),
                shortcut_search: Default::default(),
            },
            campaign: Some(CampaignConfig {
                id: "non_brix".to_string(),
            }),
        };
        let wrong_campaign = ResearchCase {
            campaign: Some(CampaignConfig {
                id: "other".to_string(),
            }),
            ..matching.clone()
        };
        let wrong_id = ResearchCase {
            id: "skip_me".to_string(),
            ..matching.clone()
        };

        assert!(matches_research_case_filters(
            &matching,
            &requested_case_ids,
            &requested_campaign_ids,
            None,
        ));
        assert!(!matches_research_case_filters(
            &wrong_campaign,
            &requested_case_ids,
            &requested_campaign_ids,
            None,
        ));
        assert!(!matches_research_case_filters(
            &wrong_id,
            &requested_case_ids,
            &requested_campaign_ids,
            None,
        ));
    }

    #[test]
    fn research_case_filters_apply_benchmark_case_ids() {
        let requested_case_ids = BTreeSet::new();
        let requested_campaign_ids = BTreeSet::new();
        let requested_benchmark_case_ids = BTreeSet::from(["keep_me".to_string()]);
        let matching = ResearchCase {
            id: "keep_me".to_string(),
            a: vec![vec![1]],
            b: vec![vec![1]],
            config: JsonSearchConfig {
                max_lag: 1,
                max_intermediate_dim: 1,
                max_entry: 1,
                frontier_mode: Default::default(),
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
                move_family_policy: super::default_move_family_policy(),
                stage: Default::default(),
                guided_refinement: Default::default(),
                shortcut_search: Default::default(),
            },
            campaign: None,
        };
        let wrong_id = ResearchCase {
            id: "skip_me".to_string(),
            ..matching.clone()
        };

        assert!(matches_research_case_filters(
            &matching,
            &requested_case_ids,
            &requested_campaign_ids,
            Some(&requested_benchmark_case_ids),
        ));
        assert!(!matches_research_case_filters(
            &wrong_id,
            &requested_case_ids,
            &requested_campaign_ids,
            Some(&requested_benchmark_case_ids),
        ));
    }

    #[test]
    fn selected_benchmark_case_ids_combines_roles() {
        let by_role = HashMap::from([
            (
                "heldout_benchmark".to_string(),
                BTreeSet::from(["riedel_baker_k4".to_string(), "riedel_baker_k6".to_string()]),
            ),
            (
                "sanity_only".to_string(),
                BTreeSet::from(["fixture_case".to_string()]),
            ),
        ]);

        let selected = selected_benchmark_case_ids(
            &["heldout_benchmark".to_string(), "sanity_only".to_string()],
            &by_role,
        )
        .expect("known roles should resolve")
        .expect("selected case ids should be returned");

        assert_eq!(
            selected,
            BTreeSet::from([
                "fixture_case".to_string(),
                "riedel_baker_k4".to_string(),
                "riedel_baker_k6".to_string(),
            ])
        );
    }

    #[test]
    fn selected_benchmark_case_ids_rejects_unknown_role() {
        let err = selected_benchmark_case_ids(&["missing".to_string()], &HashMap::new())
            .expect_err("unknown role should fail");
        assert!(err.contains("unknown --benchmark-role missing"));
    }

    #[test]
    fn requested_manifest_catalog_consistency_is_scoped_to_selected_cases() {
        let mut catalog = PairCatalog {
            benchmark_pair_by_case_id: HashMap::from([
                ("keep_me".to_string(), "pair_keep".to_string()),
                ("skip_me".to_string(), "pair_skip".to_string()),
            ]),
            family_by_pair_id: HashMap::from([
                (
                    "pair_keep".to_string(),
                    FamilyMetadata {
                        evaluation_family_id: "heldout".to_string(),
                        benchmark_role: "heldout_benchmark".to_string(),
                    },
                ),
                (
                    "pair_skip".to_string(),
                    FamilyMetadata {
                        evaluation_family_id: "heldout".to_string(),
                        benchmark_role: "heldout_benchmark".to_string(),
                    },
                ),
            ]),
            ..PairCatalog::default()
        };
        catalog.manifest_endpoint_cases_by_case_id.insert(
            "keep_me".to_string(),
            ManifestEndpointCase {
                pair_id: "pair_keep".to_string(),
                source: DynMatrix::new(1, 1, vec![1]),
                target: DynMatrix::new(1, 1, vec![1]),
            },
        );

        validate_requested_manifest_catalog_consistency(
            &catalog,
            Some(&BTreeSet::from(["keep_me".to_string()])),
        )
        .expect("selected case should validate");

        let err = validate_requested_manifest_catalog_consistency(
            &catalog,
            Some(&BTreeSet::from(["skip_me".to_string()])),
        )
        .expect_err("missing selected case should fail");
        assert!(err.contains("family benchmark case skip_me"));
    }

    #[test]
    fn endpoint_cases_keep_case_policy_without_explicit_override() {
        let cli = Cli {
            search_mode: MoveFamilyPolicy::Mixed,
            search_mode_explicit: false,
            ..parse_cli(std::iter::empty()).expect("default cli should parse")
        };

        assert_eq!(
            effective_endpoint_move_family_policy(MoveFamilyPolicy::GraphOnly, &cli),
            MoveFamilyPolicy::GraphOnly
        );
    }

    #[test]
    fn endpoint_cases_honor_explicit_search_mode_override() {
        let cli = parse_cli(["--search-mode".to_string(), "graph-only".to_string()].into_iter())
            .expect("cli should accept graph-only");

        assert_eq!(
            effective_endpoint_move_family_policy(MoveFamilyPolicy::Mixed, &cli),
            MoveFamilyPolicy::GraphOnly
        );
    }
}
