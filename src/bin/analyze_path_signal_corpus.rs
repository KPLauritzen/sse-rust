use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::path::{Path, PathBuf};

#[cfg(not(target_arch = "wasm32"))]
use rusqlite::Connection;
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::path_scoring::{candidate_score_specs, new_summaries, rank_target, ScoreSummary};
use sse_core::search::execute_search_request_and_observer;
use sse_core::search_observer::{SearchEdgeStatus, SearchEvent, SearchObserver};
use sse_core::types::{
    GuideArtifactPayload, GuidedRefinementConfig, SearchConfig, SearchDirection, SearchMode,
    SearchRequest, SearchRunResult, SearchStage, ShortcutSearchConfig,
};

fn main() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    let source_paths = load_source_paths(&cli)?;
    if source_paths.is_empty() {
        return Err("no full paths were loaded; pass --guide-artifacts or --path-db".to_string());
    }

    let cases = derive_cases(&source_paths, &cli);
    if cases.is_empty() {
        return Err("no segment cases were derived from the loaded paths".to_string());
    }

    let specs = candidate_score_specs();
    let mut summaries = new_summaries(&specs);
    let mut analyzed_cases = Vec::new();
    let mut solved_cases = 0usize;
    let mut unmatched_cases = 0usize;
    let mut total_solution_nodes = 0usize;
    let mut total_ranked_nodes = 0usize;

    for case in cases {
        match analyze_case(&case, &cli, &specs, &mut summaries)? {
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

    println!("Signal corpus analysis");
    println!(
        "  source_paths={} solved_cases={} unsolved_cases={} ranked_solution_nodes={}/{}",
        source_paths.len(),
        solved_cases,
        unmatched_cases,
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
            "    {} gap={} solved_lag={} ranked={}/{} layers={}",
            analysis.label,
            analysis.gap,
            analysis.solved_lag,
            analysis.ranked_nodes,
            analysis.solution_nodes,
            analysis.layer_count
        );
    }
    if analyzed_cases.len() > 12 {
        println!("    ... {} more case(s)", analyzed_cases.len() - 12);
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    guide_artifact_paths: Vec<PathBuf>,
    path_dbs: Vec<PathBuf>,
    min_gap: usize,
    max_gap: usize,
    max_cases: usize,
    max_endpoint_dim: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
    search_mode: SearchMode,
}

#[derive(Clone)]
struct SourcePath {
    label: String,
    matrices: Vec<DynMatrix>,
}

#[derive(Clone)]
struct SegmentCase {
    label: String,
    gap: usize,
    source: DynMatrix,
    target: DynMatrix,
}

struct CaseAnalysis {
    label: String,
    gap: usize,
    solved_lag: usize,
    solution_nodes: usize,
    ranked_nodes: usize,
    layer_count: usize,
}

#[derive(Default)]
struct LayerCollector {
    layers: Vec<ObservedLayer>,
}

struct ObservedLayer {
    direction: SearchDirection,
    candidates: Vec<DynMatrix>,
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

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut cli = Cli {
        guide_artifact_paths: Vec::new(),
        path_dbs: Vec::new(),
        min_gap: 2,
        max_gap: 4,
        max_cases: 48,
        max_endpoint_dim: 3,
        max_intermediate_dim: 5,
        max_entry: 6,
        search_mode: SearchMode::Mixed,
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
                    "mixed" => SearchMode::Mixed,
                    "graph-only" | "graph_only" => SearchMode::GraphOnly,
                    _ => return Err(format!("unknown search mode: {value}")),
                };
            }
            "--help" | "-h" => {
                return Err(
                    "usage: analyze_path_signal_corpus [options]\n\n\
                     Options:\n\
                       --guide-artifacts PATH   load full-path guide artifacts (repeatable)\n\
                       --path-db PATH           load shortcut paths from a legacy sqlite db (repeatable)\n\
                       --min-gap N             minimum segment gap to analyze (default: 2)\n\
                       --max-gap N             maximum segment gap to analyze (default: 4)\n\
                       --max-cases N           cap derived segment cases (default: 48)\n\
                       --max-endpoint-dim N    ignore cases with larger endpoints (default: 3)\n\
                       --max-intermediate-dim N\n\
                                              search config bound for derived cases (default: 5)\n\
                       --max-entry N           search config entry bound (default: 6)\n\
                       --search-mode MODE      mixed | graph-only (default: mixed)\n\
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

    if cli.guide_artifact_paths.is_empty() && cli.path_dbs.is_empty() {
        let default_db = PathBuf::from("research/k3-graph-paths.sqlite");
        if default_db.exists() {
            cli.path_dbs.push(default_db);
        }
    }

    Ok(cli)
}

fn load_source_paths(cli: &Cli) -> Result<Vec<SourcePath>, String> {
    let mut paths = Vec::new();
    for path in &cli.guide_artifact_paths {
        paths.extend(load_paths_from_guide_artifacts(path)?);
    }
    for path in &cli.path_dbs {
        paths.extend(load_paths_from_sqlite(path)?);
    }

    let mut seen = HashSet::new();
    paths.retain(|path| seen.insert(path_signature(&path.matrices)));
    Ok(paths)
}

fn load_paths_from_guide_artifacts(path: &Path) -> Result<Vec<SourcePath>, String> {
    let mut paths = Vec::new();
    for artifact in load_guide_artifacts_from_path(path)? {
        let GuideArtifactPayload::FullPath { path } = artifact.payload;
        let label = artifact
            .artifact_id
            .or(artifact.provenance.label)
            .unwrap_or_else(|| "guide_artifact_path".to_string());
        paths.push(SourcePath {
            label,
            matrices: canonicalize_path(&path.matrices),
        });
    }
    Ok(paths)
}

#[cfg(not(target_arch = "wasm32"))]
fn load_paths_from_sqlite(path: &Path) -> Result<Vec<SourcePath>, String> {
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
                paths.push(SourcePath {
                    label: format!("sqlite:{}:{}", prev_id, current_label),
                    matrices: std::mem::take(&mut current_matrices),
                });
            }
            current_id = Some(result_id);
            current_label = guide_label;
        }
        current_matrices.push(matrix);
    }

    if let Some(result_id) = current_id {
        paths.push(SourcePath {
            label: format!("sqlite:{}:{}", result_id, current_label),
            matrices: current_matrices,
        });
    }

    Ok(paths)
}

#[cfg(target_arch = "wasm32")]
fn load_paths_from_sqlite(path: &Path) -> Result<Vec<SourcePath>, String> {
    let _ = path;
    Err("sqlite path loading is not supported on wasm32".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn sqlite_value_err(err: rusqlite::Error) -> String {
    err.to_string()
}

fn canonicalize_path(path: &[DynMatrix]) -> Vec<DynMatrix> {
    path.iter().map(DynMatrix::canonical_perm).collect()
}

fn derive_cases(paths: &[SourcePath], cli: &Cli) -> Vec<SegmentCase> {
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
                    gap: end - start,
                    source,
                    target,
                });
            }
        }
    }

    cases.sort_by(|left, right| {
        left.gap
            .cmp(&right.gap)
            .then(left.source.rows.cmp(&right.source.rows))
            .then(left.target.rows.cmp(&right.target.rows))
            .then(left.label.cmp(&right.label))
    });
    cases.truncate(cli.max_cases);

    cases
}

fn analyze_case(
    case: &SegmentCase,
    cli: &Cli,
    specs: &[sse_core::path_scoring::ScoreSpec],
    summaries: &mut BTreeMap<&'static str, ScoreSummary>,
) -> Result<Option<CaseAnalysis>, String> {
    let request = SearchRequest {
        source: case.source.clone(),
        target: case.target.clone(),
        config: SearchConfig {
            max_lag: case.gap,
            max_intermediate_dim: cli.max_intermediate_dim,
            max_entry: cli.max_entry,
            search_mode: cli.search_mode,
        },
        stage: SearchStage::EndpointSearch,
        guide_artifacts: Vec::new(),
        guided_refinement: GuidedRefinementConfig::default(),
        shortcut_search: ShortcutSearchConfig::default(),
    };

    let mut observer = LayerCollector::default();
    println!("  running {}", case.label);
    let (result, _) = execute_search_request_and_observer(&request, Some(&mut observer))?;
    let SearchRunResult::Equivalent(path) = result else {
        return Ok(None);
    };

    let mut remaining = path
        .matrices
        .iter()
        .skip(1)
        .take(path.matrices.len().saturating_sub(2))
        .map(DynMatrix::canonical_perm)
        .collect::<HashSet<_>>();

    let solution_nodes = remaining.len();
    let mut ranked_nodes = 0usize;

    for layer in &observer.layers {
        if remaining.is_empty() {
            break;
        }
        let endpoint_target = match layer.direction {
            SearchDirection::Forward => &case.target,
            SearchDirection::Backward => &case.source,
        };
        let matched = layer
            .candidates
            .iter()
            .filter(|candidate| remaining.contains(*candidate))
            .cloned()
            .collect::<Vec<_>>();
        for candidate in matched {
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
            remaining.remove(&candidate);
            ranked_nodes += 1;
        }
    }

    Ok(Some(CaseAnalysis {
        label: case.label.clone(),
        gap: case.gap,
        solved_lag: path.steps.len(),
        solution_nodes,
        ranked_nodes,
        layer_count: observer.layers.len(),
    }))
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
