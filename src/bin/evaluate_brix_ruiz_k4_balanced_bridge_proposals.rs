use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::time::Instant;

use serde::Serialize;
use sse_core::balanced::{
    enumerate_balanced_bridge_insplit_return_neighbors_3x3,
    enumerate_balanced_bridge_return_neighbors_3x3, BalancedSearchConfig2x2,
};
use sse_core::matrix::DynMatrix;
use sse_core::search::execute_search_request_and_observer;
use sse_core::search_observer::{
    SearchEdgeRecord, SearchEdgeStatus, SearchEvent, SearchObserver, SearchRootRecord,
};
use sse_core::types::{
    EsseStep, FrontierMode, MoveFamilyPolicy, SearchConfig, SearchDirection, SearchRequest,
    SearchRunResult, SearchStage, SearchTelemetry,
};

const CASE_ID: &str = "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12";

fn main() {
    match run() {
        Ok(()) => {}
        Err(err) if err == usage() => {
            println!("{err}");
        }
        Err(err) => {
            eprintln!("{err}");
            std::process::exit(2);
        }
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        return Err(usage());
    }
    let cli = parse_cli(args.into_iter())?;
    let request = retained_request();
    let mut observer = RetainedObserver::default();
    let started = Instant::now();
    let (result, telemetry) = execute_search_request_and_observer(&request, Some(&mut observer))?;
    let elapsed_ms = started.elapsed().as_millis() as u64;

    let report = observer.into_report(&result, &telemetry, elapsed_ms, &cli);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;

    if let Some(path) = cli.json_out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
        }
        fs::write(&path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    json_out: Option<PathBuf>,
    top_hits: usize,
    keep_examples: usize,
    bridge_max_entry: u32,
    max_common_dim: usize,
    max_entry: u32,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut cli = Cli {
        json_out: None,
        top_hits: 12,
        keep_examples: 12,
        bridge_max_entry: 4,
        max_common_dim: 1,
        max_entry: 4,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json-out" => {
                cli.json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--top-hits" => {
                cli.top_hits = parse_positive_usize(args.next(), "--top-hits")?;
            }
            "--keep-examples" => {
                cli.keep_examples = parse_positive_usize(args.next(), "--keep-examples")?;
            }
            "--bridge-max-entry" => {
                cli.bridge_max_entry = parse_positive_u32(args.next(), "--bridge-max-entry")?;
            }
            "--max-common-dim" => {
                cli.max_common_dim = parse_positive_usize(args.next(), "--max-common-dim")?;
            }
            "--max-entry" => {
                cli.max_entry = parse_positive_u32(args.next(), "--max-entry")?;
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if cli.top_hits > 220 {
        return Err("--top-hits must be at most 220".to_string());
    }
    if cli.bridge_max_entry > 12 || cli.max_entry > 12 {
        return Err("--bridge-max-entry and --max-entry must be at most 12".to_string());
    }
    if cli.max_common_dim > 2 {
        return Err("--max-common-dim must be at most 2 for this bounded probe".to_string());
    }

    Ok(cli)
}

fn parse_positive_usize(value: Option<String>, label: &str) -> Result<usize, String> {
    let parsed = value
        .ok_or_else(|| format!("{label} requires a value"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {label}"))?;
    if parsed == 0 {
        return Err(format!("{label} must be at least 1"));
    }
    Ok(parsed)
}

fn parse_positive_u32(value: Option<String>, label: &str) -> Result<u32, String> {
    let parsed = value
        .ok_or_else(|| format!("{label} requires a value"))?
        .parse::<u32>()
        .map_err(|_| format!("invalid {label}"))?;
    if parsed == 0 {
        return Err(format!("{label} must be at least 1"));
    }
    Ok(parsed)
}

fn usage() -> String {
    "usage: evaluate_brix_ruiz_k4_balanced_bridge_proposals [--top-hits N<=220] [--bridge-max-entry N<=12] [--max-common-dim N<=2] [--max-entry N<=12] [--keep-examples N] [--json-out PATH]".to_string()
}

fn retained_request() -> SearchRequest {
    SearchRequest {
        source: DynMatrix::new(2, 2, vec![1, 4, 3, 1]),
        target: DynMatrix::new(2, 2, vec![1, 12, 1, 1]),
        config: SearchConfig {
            max_lag: 40,
            max_intermediate_dim: 4,
            max_entry: 12,
            frontier_mode: FrontierMode::Beam,
            move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
            beam_width: Some(256),
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
            endpoint_multi_meet_cap: None,
        },
        stage: SearchStage::EndpointSearch,
        guide_artifacts: Vec::new(),
        guided_refinement: Default::default(),
        shortcut_search: Default::default(),
    }
}

#[derive(Default)]
struct RetainedObserver {
    visits_by_direction: BTreeMap<DirectionLabel, BTreeMap<DynMatrix, StateVisit>>,
    visits_by_signature: BTreeMap<DirectionLabel, BTreeMap<ApproxSignature, Vec<StateVisit>>>,
    approximate_hits: Vec<ApproximateHitEvidence>,
}

impl SearchObserver for RetainedObserver {
    fn on_event(&mut self, event: &SearchEvent) {
        match event {
            SearchEvent::Roots(records) => {
                for record in records {
                    self.record_root(record);
                }
            }
            SearchEvent::Layer(edges) => {
                for edge in edges {
                    if should_record_retained_visit(edge.status, edge.enqueued) {
                        self.record_visit(
                            direction_label(edge.direction),
                            edge.to_depth,
                            edge.to_canonical.clone(),
                            edge.to_orig.clone(),
                        );
                    }
                }
                for edge in edges {
                    self.record_edge_before_visit(edge);
                }
            }
            _ => {}
        }
    }
}

impl RetainedObserver {
    fn record_root(&mut self, record: &SearchRootRecord) {
        self.record_visit(
            direction_label(record.direction),
            record.depth,
            record.canonical.clone(),
            record.orig.clone(),
        );
    }

    fn record_visit(
        &mut self,
        direction: DirectionLabel,
        depth: usize,
        canonical: DynMatrix,
        orig: DynMatrix,
    ) {
        let by_direction = self.visits_by_direction.entry(direction).or_default();
        if by_direction.contains_key(&canonical) {
            return;
        }

        let visit = StateVisit {
            direction,
            depth,
            canonical: canonical.clone(),
            orig,
            signature: approx_signature(&canonical),
        };
        by_direction.insert(canonical, visit.clone());
        self.visits_by_signature
            .entry(direction)
            .or_default()
            .entry(visit.signature.clone())
            .or_default()
            .push(visit);
    }

    fn record_edge_before_visit(&mut self, edge: &SearchEdgeRecord) {
        if !should_record_ranked_approximate_hit(
            edge.approximate_other_side_hit,
            edge.status,
            edge.enqueued,
        ) {
            return;
        }

        let direction = direction_label(edge.direction);
        let opposite = opposite_direction(direction);
        let signature = approx_signature(&edge.to_canonical);
        let counterpart = self
            .visits_by_signature
            .get(&opposite)
            .and_then(|by_signature| by_signature.get(&signature))
            .and_then(|visits| closest_counterpart(visits, &edge.to_canonical));

        let bridge_depth = counterpart.as_ref().map(|hit| edge.to_depth + hit.depth);
        self.approximate_hits.push(ApproximateHitEvidence {
            rank: 0,
            layer_index: edge.layer_index,
            direction,
            move_family: edge.move_family.to_string(),
            from_depth: edge.from_depth,
            to_depth: edge.to_depth,
            counterpart_depth: counterpart.as_ref().map(|hit| hit.depth),
            bridge_depth,
            bridge_slack_at_lag40: bridge_depth.map(|depth| 40isize - depth as isize),
            counterpart_l1: counterpart
                .as_ref()
                .map(|hit| matrix_l1_distance(&edge.to_canonical, &hit.canonical)),
            from_matrix: edge.from_orig.clone(),
            to_matrix: edge.to_orig.clone(),
            to_canonical: edge.to_canonical.clone(),
            counterpart_matrix: counterpart.as_ref().map(|hit| hit.orig.clone()),
            counterpart_canonical: counterpart.as_ref().map(|hit| hit.canonical.clone()),
            step: step_summary(&edge.step),
        });
    }

    fn into_report(
        self,
        result: &SearchRunResult,
        telemetry: &SearchTelemetry,
        elapsed_ms: u64,
        cli: &Cli,
    ) -> ProposalReport {
        let mut approximate_hits = self.approximate_hits;
        approximate_hits.sort_by(compare_approximate_hits);
        for (idx, hit) in approximate_hits.iter_mut().enumerate() {
            hit.rank = idx + 1;
        }

        let selected_hits = select_top_hits_for_proposals(&approximate_hits, cli.top_hits);
        let proposal_config = ProposalConfig {
            top_hits: cli.top_hits,
            bridge_max_entry: cli.bridge_max_entry,
            max_common_dim: cli.max_common_dim,
            max_entry: cli.max_entry,
            seam_families: vec![
                "principal_3x3_bridge_return_outsplit".to_string(),
                "principal_3x3_bridge_return_insplit".to_string(),
            ],
            semantics: "report_only_balanced_elementary_principal_window_overlay".to_string(),
        };
        let mut evaluation = evaluate_proposals(
            &selected_hits,
            &self.visits_by_direction,
            &self.visits_by_signature,
            &proposal_config,
            cli.keep_examples,
        );
        evaluation.max_frontier = telemetry.max_frontier_size;
        evaluation.visited = telemetry.total_visited_nodes;
        evaluation.expanded = telemetry.frontier_nodes_expanded;
        evaluation.factorisations = telemetry.factorisations_enumerated;
        evaluation.kept_candidates = telemetry.candidates_after_pruning;
        evaluation.discovered = telemetry.discovered_nodes;
        evaluation.elapsed_ms = elapsed_ms;

        ProposalReport {
            case_id: CASE_ID.to_string(),
            result: result_label(result),
            baseline: BaselineMetrics::from_result_telemetry(result, telemetry, elapsed_ms),
            proposal_config,
            selected_hit_count: selected_hits.len(),
            selected_hit_dimensions: dimension_counts(&selected_hits),
            evaluation,
            selected_hits: selected_hits.into_iter().take(cli.keep_examples).collect(),
        }
    }
}

#[derive(Clone)]
struct StateVisit {
    direction: DirectionLabel,
    depth: usize,
    canonical: DynMatrix,
    orig: DynMatrix,
    signature: ApproxSignature,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
#[serde(rename_all = "snake_case")]
enum DirectionLabel {
    Forward,
    Backward,
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Serialize)]
struct ApproxSignature {
    dim: usize,
    entry_sum: u64,
    row_sums: Vec<u32>,
    col_sums: Vec<u32>,
    row_supports: Vec<u8>,
    col_supports: Vec<u8>,
}

#[derive(Clone, Serialize)]
struct ApproximateHitEvidence {
    rank: usize,
    layer_index: usize,
    direction: DirectionLabel,
    move_family: String,
    from_depth: usize,
    to_depth: usize,
    counterpart_depth: Option<usize>,
    bridge_depth: Option<usize>,
    bridge_slack_at_lag40: Option<isize>,
    counterpart_l1: Option<u64>,
    from_matrix: DynMatrix,
    to_matrix: DynMatrix,
    to_canonical: DynMatrix,
    counterpart_matrix: Option<DynMatrix>,
    counterpart_canonical: Option<DynMatrix>,
    step: StepSummary,
}

#[derive(Clone, Serialize)]
struct StepSummary {
    u_rows: usize,
    u_cols: usize,
    v_rows: usize,
    v_cols: usize,
    u: DynMatrix,
    v: DynMatrix,
}

#[derive(Serialize)]
struct ProposalReport {
    case_id: String,
    result: String,
    baseline: BaselineMetrics,
    proposal_config: ProposalConfig,
    selected_hit_count: usize,
    selected_hit_dimensions: BTreeMap<usize, usize>,
    evaluation: ProposalEvaluation,
    selected_hits: Vec<ApproximateHitEvidence>,
}

#[derive(Clone, Serialize)]
struct ProposalConfig {
    top_hits: usize,
    bridge_max_entry: u32,
    max_common_dim: usize,
    max_entry: u32,
    seam_families: Vec<String>,
    semantics: String,
}

#[derive(Serialize)]
struct BaselineMetrics {
    outcome: String,
    exact_target_hits: usize,
    approximate_hits: usize,
    max_frontier: usize,
    visited: usize,
    expanded: usize,
    factorisations: usize,
    kept_candidates: usize,
    discovered: usize,
    elapsed_ms: u64,
}

impl BaselineMetrics {
    fn from_result_telemetry(
        result: &SearchRunResult,
        telemetry: &SearchTelemetry,
        elapsed_ms: u64,
    ) -> Self {
        Self {
            outcome: result_label(result),
            exact_target_hits: telemetry.collisions_with_other_frontier,
            approximate_hits: telemetry.approximate_other_side_hits,
            max_frontier: telemetry.max_frontier_size,
            visited: telemetry.total_visited_nodes,
            expanded: telemetry.frontier_nodes_expanded,
            factorisations: telemetry.factorisations_enumerated,
            kept_candidates: telemetry.candidates_after_pruning,
            discovered: telemetry.discovered_nodes,
            elapsed_ms,
        }
    }
}

#[derive(Default, Serialize)]
struct ProposalEvaluation {
    source_hits_considered: usize,
    principal_windows_considered: usize,
    proposal_neighbors_generated_raw: usize,
    unique_full_candidates: usize,
    endpoint_hit_scope: String,
    exact_opposite_frontier_hits: usize,
    approximate_opposite_frontier_hits: usize,
    improved_counterpart_l1: usize,
    best_counterpart_l1_before: Option<u64>,
    best_counterpart_l1_after: Option<u64>,
    max_frontier: usize,
    visited: usize,
    expanded: usize,
    factorisations: usize,
    kept_candidates: usize,
    discovered: usize,
    elapsed_ms: u64,
    examples: Vec<ProposalExample>,
}

#[derive(Clone)]
struct ProposalAggregate {
    representative: ProposalExample,
    any_exact_opposite_frontier_hit: bool,
    any_approximate_opposite_frontier_hit: bool,
    any_counterpart_l1_improved: bool,
    min_candidate_counterpart_l1: Option<u64>,
}

#[derive(Clone, Serialize)]
struct ProposalExample {
    source_rank: usize,
    source_direction: DirectionLabel,
    source_depth: usize,
    deleted_vertex: usize,
    seam_family: String,
    exact_opposite_frontier_hit: bool,
    approximate_opposite_frontier_hit: bool,
    base_counterpart_l1: Option<u64>,
    candidate_counterpart_l1: Option<u64>,
    improvement: Option<i64>,
    any_counterpart_l1_improved: bool,
    candidate: DynMatrix,
    candidate_canonical: DynMatrix,
    counterpart_canonical: Option<DynMatrix>,
}

fn evaluate_proposals(
    hits: &[ApproximateHitEvidence],
    visits_by_direction: &BTreeMap<DirectionLabel, BTreeMap<DynMatrix, StateVisit>>,
    visits_by_signature: &BTreeMap<DirectionLabel, BTreeMap<ApproxSignature, Vec<StateVisit>>>,
    config: &ProposalConfig,
    keep_examples: usize,
) -> ProposalEvaluation {
    let balanced_config = BalancedSearchConfig2x2 {
        max_common_dim: config.max_common_dim,
        max_entry: config.max_entry,
    };
    let mut eval = ProposalEvaluation {
        endpoint_hit_scope: "not_applicable_4x4_overlay_vs_2x2_endpoint".to_string(),
        ..ProposalEvaluation::default()
    };
    let mut unique = BTreeMap::<(DynMatrix, DirectionLabel), ProposalAggregate>::new();

    for hit in hits {
        if hit.to_matrix.rows != 4 || hit.to_matrix.cols != 4 {
            continue;
        }
        eval.source_hits_considered += 1;
        eval.best_counterpart_l1_before =
            min_option(eval.best_counterpart_l1_before, hit.counterpart_l1);

        for deleted_vertex in 0..4 {
            let window = principal_window_3x3(&hit.to_matrix, deleted_vertex);
            eval.principal_windows_considered += 1;
            for (seam_family, neighbors) in [
                (
                    "principal_3x3_bridge_return_outsplit",
                    enumerate_balanced_bridge_return_neighbors_3x3(
                        &window,
                        config.bridge_max_entry,
                        &balanced_config,
                    ),
                ),
                (
                    "principal_3x3_bridge_return_insplit",
                    enumerate_balanced_bridge_insplit_return_neighbors_3x3(
                        &window,
                        config.bridge_max_entry,
                        &balanced_config,
                    ),
                ),
            ] {
                eval.proposal_neighbors_generated_raw += neighbors.len();
                for neighbor in neighbors {
                    let candidate = embed_principal_window_3x3(
                        &hit.to_matrix,
                        deleted_vertex,
                        &neighbor.matrix,
                    );
                    let candidate_canonical = candidate.canonical_perm();
                    let opposite = opposite_direction(hit.direction);
                    let exact_opposite_frontier_hit = visits_by_direction
                        .get(&opposite)
                        .is_some_and(|visits| visits.contains_key(&candidate_canonical));
                    let signature_opposite_frontier_hit =
                        visits_by_signature.get(&opposite).is_some_and(|visits| {
                            visits.contains_key(&approx_signature(&candidate_canonical))
                        });
                    let approximate_opposite_frontier_hit = is_non_exact_signature_hit(
                        signature_opposite_frontier_hit,
                        exact_opposite_frontier_hit,
                    );
                    let candidate_counterpart_l1 = hit
                        .counterpart_canonical
                        .as_ref()
                        .map(|counterpart| matrix_l1_distance(&candidate_canonical, counterpart));
                    let improvement = hit
                        .counterpart_l1
                        .zip(candidate_counterpart_l1)
                        .map(|(before, after)| before as i64 - after as i64);
                    let example = ProposalExample {
                        source_rank: hit.rank,
                        source_direction: hit.direction,
                        source_depth: hit.to_depth,
                        deleted_vertex,
                        seam_family: seam_family.to_string(),
                        exact_opposite_frontier_hit,
                        approximate_opposite_frontier_hit,
                        base_counterpart_l1: hit.counterpart_l1,
                        candidate_counterpart_l1,
                        improvement,
                        any_counterpart_l1_improved: improvement
                            .is_some_and(|improvement| improvement > 0),
                        candidate,
                        candidate_canonical: candidate_canonical.clone(),
                        counterpart_canonical: hit.counterpart_canonical.clone(),
                    };
                    unique
                        .entry((candidate_canonical, hit.direction))
                        .and_modify(|existing| merge_proposal_aggregate(existing, example.clone()))
                        .or_insert_with(|| ProposalAggregate {
                            representative: example.clone(),
                            any_exact_opposite_frontier_hit: example.exact_opposite_frontier_hit,
                            any_approximate_opposite_frontier_hit: example
                                .approximate_opposite_frontier_hit,
                            any_counterpart_l1_improved: example.any_counterpart_l1_improved,
                            min_candidate_counterpart_l1: example.candidate_counterpart_l1,
                        });
                }
            }
        }
    }

    eval.unique_full_candidates = unique.len();
    let aggregates = unique.into_values().collect::<Vec<_>>();
    let mut examples = aggregates
        .iter()
        .map(|aggregate| aggregate.representative.clone())
        .collect::<Vec<_>>();
    examples.sort_by(compare_proposal_examples);
    for aggregate in &aggregates {
        eval.exact_opposite_frontier_hits += usize::from(aggregate.any_exact_opposite_frontier_hit);
        eval.approximate_opposite_frontier_hits +=
            usize::from(aggregate.any_approximate_opposite_frontier_hit);
        if aggregate.any_counterpart_l1_improved {
            eval.improved_counterpart_l1 += 1;
        }
        eval.best_counterpart_l1_after = min_option(
            eval.best_counterpart_l1_after,
            aggregate.min_candidate_counterpart_l1,
        );
    }
    eval.examples = examples.into_iter().take(keep_examples).collect();
    eval
}

fn principal_window_3x3(matrix: &DynMatrix, deleted_vertex: usize) -> DynMatrix {
    debug_assert_eq!(matrix.rows, 4);
    debug_assert_eq!(matrix.cols, 4);
    let kept = (0..4)
        .filter(|&idx| idx != deleted_vertex)
        .collect::<Vec<_>>();
    let mut data = Vec::with_capacity(9);
    for &row in &kept {
        for &col in &kept {
            data.push(matrix.get(row, col));
        }
    }
    DynMatrix::new(3, 3, data)
}

fn embed_principal_window_3x3(
    base: &DynMatrix,
    deleted_vertex: usize,
    window: &DynMatrix,
) -> DynMatrix {
    debug_assert_eq!(base.rows, 4);
    debug_assert_eq!(base.cols, 4);
    debug_assert_eq!(window.rows, 3);
    debug_assert_eq!(window.cols, 3);
    let kept = (0..4)
        .filter(|&idx| idx != deleted_vertex)
        .collect::<Vec<_>>();
    let mut candidate = base.clone();
    for (window_row, &row) in kept.iter().enumerate() {
        for (window_col, &col) in kept.iter().enumerate() {
            candidate.set(row, col, window.get(window_row, window_col));
        }
    }
    candidate
}

fn compare_proposal_examples(
    left: &ProposalExample,
    right: &ProposalExample,
) -> std::cmp::Ordering {
    right
        .exact_opposite_frontier_hit
        .cmp(&left.exact_opposite_frontier_hit)
        .then_with(|| {
            right
                .approximate_opposite_frontier_hit
                .cmp(&left.approximate_opposite_frontier_hit)
        })
        .then_with(|| {
            right
                .any_counterpart_l1_improved
                .cmp(&left.any_counterpart_l1_improved)
        })
        .then_with(|| left.source_rank.cmp(&right.source_rank))
        .then_with(|| left.source_depth.cmp(&right.source_depth))
        .then_with(|| left.deleted_vertex.cmp(&right.deleted_vertex))
        .then_with(|| left.seam_family.cmp(&right.seam_family))
        .then_with(|| {
            left.candidate_canonical
                .data
                .cmp(&right.candidate_canonical.data)
        })
}

fn is_non_exact_signature_hit(signature_hit: bool, exact_hit: bool) -> bool {
    signature_hit && !exact_hit
}

fn should_record_ranked_approximate_hit(
    approximate_other_side_hit: bool,
    status: SearchEdgeStatus,
    enqueued: bool,
) -> bool {
    approximate_other_side_hit && should_record_retained_visit(status, enqueued)
}

fn should_record_retained_visit(status: SearchEdgeStatus, enqueued: bool) -> bool {
    enqueued && matches!(status, SearchEdgeStatus::Discovered)
}

fn select_top_hits_for_proposals(
    approximate_hits: &[ApproximateHitEvidence],
    top_hits: usize,
) -> Vec<ApproximateHitEvidence> {
    approximate_hits
        .iter()
        .filter(|hit| hit.to_matrix.rows == 4 && hit.to_matrix.cols == 4)
        .take(top_hits)
        .cloned()
        .collect()
}

fn merge_proposal_aggregate(existing: &mut ProposalAggregate, incoming: ProposalExample) {
    if compare_proposal_examples(&incoming, &existing.representative).is_lt() {
        existing.representative = incoming.clone();
    }
    existing.any_exact_opposite_frontier_hit |= incoming.exact_opposite_frontier_hit;
    existing.any_approximate_opposite_frontier_hit |= incoming.approximate_opposite_frontier_hit;
    existing.any_counterpart_l1_improved |= incoming.any_counterpart_l1_improved;
    existing.min_candidate_counterpart_l1 = min_option(
        existing.min_candidate_counterpart_l1,
        incoming.candidate_counterpart_l1,
    );
}

fn compare_approximate_hits(
    left: &ApproximateHitEvidence,
    right: &ApproximateHitEvidence,
) -> std::cmp::Ordering {
    bridge_slack_rank(left)
        .cmp(&bridge_slack_rank(right))
        .then_with(|| compare_counterpart_l1_option(left.counterpart_l1, right.counterpart_l1))
        .then_with(|| right.bridge_depth.cmp(&left.bridge_depth))
        .then_with(|| right.to_depth.cmp(&left.to_depth))
        .then_with(|| left.move_family.cmp(&right.move_family))
        .then_with(|| left.to_canonical.data.cmp(&right.to_canonical.data))
}

fn compare_counterpart_l1_option(left: Option<u64>, right: Option<u64>) -> std::cmp::Ordering {
    (left.is_none(), left.unwrap_or(u64::MAX)).cmp(&(right.is_none(), right.unwrap_or(u64::MAX)))
}

fn bridge_slack_rank(hit: &ApproximateHitEvidence) -> (u8, isize) {
    match hit.bridge_slack_at_lag40 {
        Some(slack) if slack >= 0 => (0, slack),
        Some(slack) => (1, -slack),
        None => (2, isize::MAX),
    }
}

fn closest_counterpart<'a>(
    visits: &'a [StateVisit],
    canonical: &DynMatrix,
) -> Option<&'a StateVisit> {
    visits.iter().min_by(|left, right| {
        matrix_l1_distance(canonical, &left.canonical)
            .cmp(&matrix_l1_distance(canonical, &right.canonical))
            .then_with(|| left.depth.cmp(&right.depth))
            .then_with(|| left.direction.cmp(&right.direction))
            .then_with(|| left.canonical.data.cmp(&right.canonical.data))
    })
}

fn matrix_l1_distance(left: &DynMatrix, right: &DynMatrix) -> u64 {
    if left.rows != right.rows || left.cols != right.cols {
        return u64::MAX;
    }
    left.data
        .iter()
        .zip(&right.data)
        .map(|(&a, &b)| a.abs_diff(b) as u64)
        .sum()
}

fn approx_signature(m: &DynMatrix) -> ApproxSignature {
    let mut row_sums = vec![0u32; m.rows];
    let mut col_sums = vec![0u32; m.cols];
    let mut row_supports = vec![0u8; m.rows];
    let mut col_supports = vec![0u8; m.cols];
    let mut entry_sum = 0u64;

    for row in 0..m.rows {
        for col in 0..m.cols {
            let value = m.get(row, col);
            row_sums[row] += value;
            col_sums[col] += value;
            entry_sum += value as u64;
            if value > 0 {
                row_supports[row] += 1;
                col_supports[col] += 1;
            }
        }
    }

    row_sums.sort_unstable();
    col_sums.sort_unstable();
    row_supports.sort_unstable();
    col_supports.sort_unstable();

    ApproxSignature {
        dim: m.rows,
        entry_sum,
        row_sums,
        col_sums,
        row_supports,
        col_supports,
    }
}

fn dimension_counts(hits: &[ApproximateHitEvidence]) -> BTreeMap<usize, usize> {
    let mut counts = BTreeMap::new();
    for hit in hits {
        *counts.entry(hit.to_matrix.rows).or_insert(0) += 1;
    }
    counts
}

fn min_option(left: Option<u64>, right: Option<u64>) -> Option<u64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.min(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn step_summary(step: &EsseStep) -> StepSummary {
    StepSummary {
        u_rows: step.u.rows,
        u_cols: step.u.cols,
        v_rows: step.v.rows,
        v_cols: step.v.cols,
        u: step.u.clone(),
        v: step.v.clone(),
    }
}

fn direction_label(direction: SearchDirection) -> DirectionLabel {
    match direction {
        SearchDirection::Forward => DirectionLabel::Forward,
        SearchDirection::Backward => DirectionLabel::Backward,
    }
}

fn opposite_direction(direction: DirectionLabel) -> DirectionLabel {
    match direction {
        DirectionLabel::Forward => DirectionLabel::Backward,
        DirectionLabel::Backward => DirectionLabel::Forward,
    }
}

fn result_label(result: &SearchRunResult) -> String {
    match result {
        SearchRunResult::Equivalent(_) => "equivalent".to_string(),
        SearchRunResult::EquivalentByConcreteShift(_) => "equivalent_by_concrete_shift".to_string(),
        SearchRunResult::NotEquivalent(reason) => format!("not_equivalent: {reason}"),
        SearchRunResult::Unknown => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_hit(dim: usize) -> ApproximateHitEvidence {
        let matrix = DynMatrix::new(dim, dim, vec![0; dim * dim]);
        ApproximateHitEvidence {
            rank: 0,
            layer_index: 0,
            direction: DirectionLabel::Forward,
            move_family: "principal_3x3_bridge_return_outsplit".to_string(),
            from_depth: 0,
            to_depth: 1,
            counterpart_depth: None,
            bridge_depth: None,
            bridge_slack_at_lag40: None,
            counterpart_l1: None,
            from_matrix: matrix.clone(),
            to_matrix: matrix.clone(),
            to_canonical: matrix,
            counterpart_matrix: None,
            counterpart_canonical: None,
            step: StepSummary {
                u_rows: 1,
                u_cols: 1,
                v_rows: 1,
                v_cols: 1,
                u: DynMatrix::new(1, 1, vec![1]),
                v: DynMatrix::new(1, 1, vec![1]),
            },
        }
    }

    #[test]
    fn principal_window_deletes_matching_row_and_column() {
        let matrix = DynMatrix::new(4, 4, (1..=16).collect());
        let window = principal_window_3x3(&matrix, 1);
        assert_eq!(
            window,
            DynMatrix::new(3, 3, vec![1, 3, 4, 9, 11, 12, 13, 15, 16])
        );
    }

    #[test]
    fn embed_principal_window_preserves_deleted_incidence() {
        let base = DynMatrix::new(4, 4, (1..=16).collect());
        let window = DynMatrix::new(3, 3, vec![0; 9]);
        let candidate = embed_principal_window_3x3(&base, 1, &window);
        assert_eq!(candidate.get(1, 0), 5);
        assert_eq!(candidate.get(0, 1), 2);
        assert_eq!(candidate.get(1, 1), 6);
        assert_eq!(candidate.get(0, 0), 0);
        assert_eq!(candidate.get(3, 3), 0);
    }

    #[test]
    fn exact_signature_hit_is_not_counted_as_approximate() {
        assert!(!is_non_exact_signature_hit(true, true));
        assert!(is_non_exact_signature_hit(true, false));
        assert!(!is_non_exact_signature_hit(false, false));
    }

    #[test]
    fn ranked_approximate_hits_exclude_exact_meets() {
        assert!(should_record_ranked_approximate_hit(
            true,
            SearchEdgeStatus::Discovered,
            true,
        ));
        assert!(!should_record_ranked_approximate_hit(
            true,
            SearchEdgeStatus::ExactMeet,
            false,
        ));
        assert!(!should_record_ranked_approximate_hit(
            false,
            SearchEdgeStatus::Discovered,
            true,
        ));
        assert!(!should_record_ranked_approximate_hit(
            true,
            SearchEdgeStatus::Discovered,
            false,
        ));
    }

    #[test]
    fn select_top_hits_filters_to_4x4_before_truncation() {
        let selected =
            select_top_hits_for_proposals(&[test_hit(3), test_hit(4), test_hit(2), test_hit(4)], 2);

        assert_eq!(selected.len(), 2);
        assert!(selected
            .iter()
            .all(|hit| hit.to_matrix.rows == 4 && hit.to_matrix.cols == 4));
    }

    #[test]
    fn compare_counterpart_l1_orders_missing_last() {
        assert!(compare_counterpart_l1_option(Some(8), None).is_lt());
        assert!(compare_counterpart_l1_option(None, Some(8)).is_gt());
        assert!(compare_counterpart_l1_option(Some(8), Some(12)).is_lt());
    }

    #[test]
    fn retained_visits_exclude_exact_meets() {
        let mut observer = RetainedObserver::default();
        let matrix = DynMatrix::new(4, 4, vec![0, 1, 0, 0, 0, 2, 1, 0, 0, 11, 0, 0, 0, 1, 1, 0]);
        let edge = SearchEdgeRecord {
            layer_index: 7,
            direction: SearchDirection::Forward,
            move_family: "principal_3x3_bridge_return_outsplit",
            from_canonical: matrix.clone(),
            from_orig: matrix.clone(),
            to_canonical: matrix.clone(),
            to_orig: matrix.clone(),
            from_depth: 4,
            to_depth: 5,
            step: EsseStep {
                u: DynMatrix::new(1, 1, vec![1]),
                v: DynMatrix::new(1, 1, vec![1]),
            },
            status: SearchEdgeStatus::ExactMeet,
            approximate_other_side_hit: false,
            enqueued: false,
        };

        observer.on_event(&SearchEvent::Layer(vec![edge]));

        assert!(observer.visits_by_direction.is_empty());
        assert!(observer.visits_by_signature.is_empty());
    }

    #[test]
    fn layer_observer_sees_same_layer_opposite_counterpart() {
        let mut observer = RetainedObserver::default();
        let matrix = DynMatrix::new(4, 4, vec![0, 1, 0, 0, 0, 2, 1, 0, 0, 11, 0, 0, 0, 1, 1, 0]);
        let edge = |direction: SearchDirection,
                    approximate_other_side_hit: bool,
                    to_depth: usize| SearchEdgeRecord {
            layer_index: 7,
            direction,
            move_family: "principal_3x3_bridge_return_outsplit",
            from_canonical: matrix.clone(),
            from_orig: matrix.clone(),
            to_canonical: matrix.clone(),
            to_orig: matrix.clone(),
            from_depth: to_depth.saturating_sub(1),
            to_depth,
            step: EsseStep {
                u: DynMatrix::new(1, 1, vec![1]),
                v: DynMatrix::new(1, 1, vec![1]),
            },
            status: SearchEdgeStatus::Discovered,
            approximate_other_side_hit,
            enqueued: true,
        };

        observer.on_event(&SearchEvent::Layer(vec![
            edge(SearchDirection::Forward, true, 5),
            edge(SearchDirection::Backward, false, 6),
        ]));

        assert_eq!(observer.approximate_hits.len(), 1);
        assert_eq!(observer.approximate_hits[0].counterpart_depth, Some(6));
        assert_eq!(observer.approximate_hits[0].counterpart_l1, Some(0));
    }

    #[test]
    fn merge_proposal_aggregate_promotes_stronger_representative() {
        let candidate = DynMatrix::new(4, 4, vec![0; 16]);
        let mut aggregate = ProposalAggregate {
            representative: ProposalExample {
                source_rank: 5,
                source_direction: DirectionLabel::Forward,
                source_depth: 10,
                deleted_vertex: 0,
                seam_family: "principal_3x3_bridge_return_outsplit".to_string(),
                exact_opposite_frontier_hit: false,
                approximate_opposite_frontier_hit: true,
                base_counterpart_l1: Some(30),
                candidate_counterpart_l1: Some(24),
                improvement: Some(6),
                any_counterpart_l1_improved: true,
                candidate: candidate.clone(),
                candidate_canonical: candidate.clone(),
                counterpart_canonical: None,
            },
            any_exact_opposite_frontier_hit: false,
            any_approximate_opposite_frontier_hit: true,
            any_counterpart_l1_improved: true,
            min_candidate_counterpart_l1: Some(24),
        };
        let incoming = ProposalExample {
            source_rank: 2,
            source_direction: DirectionLabel::Backward,
            source_depth: 12,
            deleted_vertex: 1,
            seam_family: "principal_3x3_bridge_return_insplit".to_string(),
            exact_opposite_frontier_hit: true,
            approximate_opposite_frontier_hit: false,
            base_counterpart_l1: Some(20),
            candidate_counterpart_l1: Some(8),
            improvement: Some(12),
            any_counterpart_l1_improved: true,
            candidate: candidate.clone(),
            candidate_canonical: candidate,
            counterpart_canonical: Some(DynMatrix::new(4, 4, vec![1; 16])),
        };

        merge_proposal_aggregate(&mut aggregate, incoming);

        assert!(aggregate.representative.exact_opposite_frontier_hit);
        assert!(!aggregate.representative.approximate_opposite_frontier_hit);
        assert_eq!(aggregate.representative.base_counterpart_l1, Some(20));
        assert_eq!(aggregate.representative.candidate_counterpart_l1, Some(8));
        assert_eq!(aggregate.representative.improvement, Some(12));
        assert!(aggregate.representative.any_counterpart_l1_improved);
        assert_eq!(aggregate.representative.source_rank, 2);
        assert_eq!(
            aggregate.representative.source_direction,
            DirectionLabel::Backward
        );
        assert_eq!(
            aggregate.representative.counterpart_canonical,
            Some(DynMatrix::new(4, 4, vec![1; 16]))
        );
        assert!(aggregate.any_exact_opposite_frontier_hit);
        assert!(aggregate.any_approximate_opposite_frontier_hit);
        assert!(aggregate.any_counterpart_l1_improved);
        assert_eq!(aggregate.min_candidate_counterpart_l1, Some(8));
    }

    #[test]
    fn merge_proposal_aggregate_accumulates_nonrepresentative_metrics() {
        let candidate = DynMatrix::new(4, 4, vec![0; 16]);
        let mut aggregate = ProposalAggregate {
            representative: ProposalExample {
                source_rank: 2,
                source_direction: DirectionLabel::Backward,
                source_depth: 12,
                deleted_vertex: 1,
                seam_family: "principal_3x3_bridge_return_insplit".to_string(),
                exact_opposite_frontier_hit: true,
                approximate_opposite_frontier_hit: false,
                base_counterpart_l1: Some(20),
                candidate_counterpart_l1: Some(8),
                improvement: Some(12),
                any_counterpart_l1_improved: true,
                candidate: candidate.clone(),
                candidate_canonical: candidate.clone(),
                counterpart_canonical: Some(DynMatrix::new(4, 4, vec![1; 16])),
            },
            any_exact_opposite_frontier_hit: true,
            any_approximate_opposite_frontier_hit: false,
            any_counterpart_l1_improved: true,
            min_candidate_counterpart_l1: Some(8),
        };
        let incoming = ProposalExample {
            source_rank: 5,
            source_direction: DirectionLabel::Forward,
            source_depth: 10,
            deleted_vertex: 0,
            seam_family: "principal_3x3_bridge_return_outsplit".to_string(),
            exact_opposite_frontier_hit: false,
            approximate_opposite_frontier_hit: true,
            base_counterpart_l1: Some(30),
            candidate_counterpart_l1: Some(6),
            improvement: Some(24),
            any_counterpart_l1_improved: true,
            candidate: candidate.clone(),
            candidate_canonical: candidate,
            counterpart_canonical: Some(DynMatrix::new(4, 4, vec![2; 16])),
        };

        merge_proposal_aggregate(&mut aggregate, incoming);

        assert_eq!(aggregate.representative.source_rank, 2);
        assert_eq!(
            aggregate.representative.source_direction,
            DirectionLabel::Backward
        );
        assert!(aggregate.representative.exact_opposite_frontier_hit);
        assert!(!aggregate.representative.approximate_opposite_frontier_hit);
        assert_eq!(aggregate.representative.candidate_counterpart_l1, Some(8));
        assert_eq!(
            aggregate.representative.counterpart_canonical,
            Some(DynMatrix::new(4, 4, vec![1; 16]))
        );
        assert!(aggregate.any_exact_opposite_frontier_hit);
        assert!(aggregate.any_approximate_opposite_frontier_hit);
        assert!(aggregate.any_counterpart_l1_improved);
        assert_eq!(aggregate.min_candidate_counterpart_l1, Some(6));
    }

    #[test]
    fn direction_relative_dedup_keeps_both_directions() {
        let candidate = DynMatrix::new(4, 4, vec![0; 16]);
        let forward = ProposalExample {
            source_rank: 1,
            source_direction: DirectionLabel::Forward,
            source_depth: 10,
            deleted_vertex: 0,
            seam_family: "principal_3x3_bridge_return_outsplit".to_string(),
            exact_opposite_frontier_hit: true,
            approximate_opposite_frontier_hit: false,
            base_counterpart_l1: Some(12),
            candidate_counterpart_l1: Some(8),
            improvement: Some(4),
            any_counterpart_l1_improved: true,
            candidate: candidate.clone(),
            candidate_canonical: candidate.clone(),
            counterpart_canonical: None,
        };
        let backward = ProposalExample {
            source_rank: 2,
            source_direction: DirectionLabel::Backward,
            source_depth: 11,
            deleted_vertex: 1,
            seam_family: "principal_3x3_bridge_return_insplit".to_string(),
            exact_opposite_frontier_hit: false,
            approximate_opposite_frontier_hit: true,
            base_counterpart_l1: Some(18),
            candidate_counterpart_l1: Some(9),
            improvement: Some(9),
            any_counterpart_l1_improved: true,
            candidate: candidate.clone(),
            candidate_canonical: candidate.clone(),
            counterpart_canonical: None,
        };

        let mut unique = BTreeMap::<(DynMatrix, DirectionLabel), ProposalExample>::new();
        unique.insert((candidate.clone(), DirectionLabel::Forward), forward);
        unique.insert((candidate, DirectionLabel::Backward), backward);

        assert_eq!(unique.len(), 2);
    }
}
