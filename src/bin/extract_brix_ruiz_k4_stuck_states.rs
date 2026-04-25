use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;
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
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    let request = retained_request();
    let mut observer = StuckStateObserver::default();
    let (result, telemetry) = execute_search_request_and_observer(&request, Some(&mut observer))?;
    let report = observer.into_report(&request, &result, &telemetry, cli.top_limit);
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
    top_limit: usize,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut cli = Cli {
        json_out: None,
        top_limit: 32,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json-out" => {
                cli.json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--top" => {
                cli.top_limit = args
                    .next()
                    .ok_or("--top requires a value")?
                    .parse()
                    .map_err(|_| "invalid --top".to_string())?;
            }
            "--help" | "-h" => {
                return Err(
                    "usage: extract_brix_ruiz_k4_stuck_states [--json-out PATH] [--top N]"
                        .to_string(),
                );
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if cli.top_limit == 0 {
        return Err("--top must be at least 1".to_string());
    }

    Ok(cli)
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
struct StuckStateObserver {
    visits_by_direction: BTreeMap<DirectionLabel, BTreeMap<DynMatrix, StateVisit>>,
    visits_by_signature: BTreeMap<DirectionLabel, BTreeMap<ApproxSignature, Vec<StateVisit>>>,
    approximate_hits: Vec<ApproximateHitEvidence>,
    parent_stats: BTreeMap<ParentKey, ParentStats>,
    family_stats: BTreeMap<&'static str, FamilyEvidence>,
    layer_stats: BTreeMap<LayerKey, LayerEvidence>,
}

impl SearchObserver for StuckStateObserver {
    fn on_event(&mut self, event: &SearchEvent) {
        match event {
            SearchEvent::Roots(records) => {
                for record in records {
                    self.record_root(record);
                }
            }
            SearchEvent::Layer(edges) => {
                for edge in edges {
                    self.record_edge_before_visit(edge);
                }
                for edge in edges {
                    if !matches!(edge.status, SearchEdgeStatus::SeenCollision) {
                        self.record_visit(
                            direction_label(edge.direction),
                            edge.to_depth,
                            edge.to_canonical.clone(),
                            edge.to_orig.clone(),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

impl StuckStateObserver {
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
        let direction = direction_label(edge.direction);
        let opposite = opposite_direction(direction);
        let signature = approx_signature(&edge.to_canonical);
        let counterpart = self
            .visits_by_signature
            .get(&opposite)
            .and_then(|by_signature| by_signature.get(&signature))
            .and_then(|visits| closest_counterpart(visits, &edge.to_canonical));

        let parent_key = ParentKey {
            direction,
            depth: edge.from_depth,
            family: edge.move_family,
            matrix: edge.from_orig.clone(),
        };
        let parent_stats = self.parent_stats.entry(parent_key.clone()).or_default();
        parent_stats.total_edges += 1;
        parent_stats.enqueued_edges += usize::from(edge.enqueued);
        parent_stats.approximate_hits += usize::from(edge.approximate_other_side_hit);
        match edge.status {
            SearchEdgeStatus::Discovered => parent_stats.discovered_edges += 1,
            SearchEdgeStatus::ExactMeet => parent_stats.exact_meets += 1,
            SearchEdgeStatus::SeenCollision => parent_stats.seen_collisions += 1,
        }

        let family_stats = self.family_stats.entry(edge.move_family).or_default();
        family_stats.total_edges += 1;
        family_stats.enqueued_edges += usize::from(edge.enqueued);
        family_stats.approximate_hits += usize::from(edge.approximate_other_side_hit);
        match edge.status {
            SearchEdgeStatus::Discovered => family_stats.discovered_edges += 1,
            SearchEdgeStatus::ExactMeet => family_stats.exact_meets += 1,
            SearchEdgeStatus::SeenCollision => family_stats.seen_collisions += 1,
        }

        let layer_stats = self
            .layer_stats
            .entry(LayerKey {
                layer_index: edge.layer_index,
                direction,
            })
            .or_default();
        layer_stats.total_edges += 1;
        layer_stats.enqueued_edges += usize::from(edge.enqueued);
        layer_stats.approximate_hits += usize::from(edge.approximate_other_side_hit);
        match edge.status {
            SearchEdgeStatus::Discovered => layer_stats.discovered_edges += 1,
            SearchEdgeStatus::ExactMeet => layer_stats.exact_meets += 1,
            SearchEdgeStatus::SeenCollision => layer_stats.seen_collisions += 1,
        }

        if edge.approximate_other_side_hit {
            let bridge_depth = counterpart.as_ref().map(|hit| edge.to_depth + hit.depth);
            let counterpart_l1 = counterpart
                .as_ref()
                .map(|hit| matrix_l1_distance(&edge.to_canonical, &hit.canonical));
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
                counterpart_l1,
                enqueued: edge.enqueued,
                status: status_label(edge.status).to_string(),
                signature: signature.clone(),
                from_matrix: edge.from_orig.clone(),
                to_matrix: edge.to_orig.clone(),
                to_canonical: edge.to_canonical.clone(),
                counterpart_matrix: counterpart.as_ref().map(|hit| hit.orig.clone()),
                counterpart_canonical: counterpart.as_ref().map(|hit| hit.canonical.clone()),
                step: step_summary(&edge.step),
                parent_family_total_edges: parent_stats.total_edges,
                parent_family_discovered_edges: parent_stats.discovered_edges,
                parent_family_approximate_hits: parent_stats.approximate_hits,
            });
        }
    }

    fn into_report(
        self,
        request: &SearchRequest,
        result: &SearchRunResult,
        telemetry: &SearchTelemetry,
        top_limit: usize,
    ) -> StuckStateReport {
        let mut approximate_hits = self.approximate_hits;
        approximate_hits.sort_by(compare_approximate_hits);
        for (idx, hit) in approximate_hits.iter_mut().enumerate() {
            hit.rank = idx + 1;
        }

        let mut family_evidence = self
            .family_stats
            .into_iter()
            .map(|(family, stats)| FamilyEvidenceRow {
                family: family.to_string(),
                total_edges: stats.total_edges,
                discovered_edges: stats.discovered_edges,
                seen_collisions: stats.seen_collisions,
                exact_meets: stats.exact_meets,
                approximate_hits: stats.approximate_hits,
                enqueued_edges: stats.enqueued_edges,
            })
            .collect::<Vec<_>>();
        family_evidence.sort_by(|left, right| {
            right
                .approximate_hits
                .cmp(&left.approximate_hits)
                .then_with(|| right.discovered_edges.cmp(&left.discovered_edges))
                .then_with(|| right.total_edges.cmp(&left.total_edges))
                .then_with(|| left.family.cmp(&right.family))
        });

        let mut family_sources = self
            .parent_stats
            .iter()
            .filter(|(_, stats)| stats.approximate_hits > 0)
            .map(|(key, stats)| ParentEvidenceRow::from_key_stats(key, stats))
            .collect::<Vec<_>>();
        family_sources.sort_by(compare_parent_sources);
        family_sources.truncate(top_limit);

        let mut low_yield_parents = self
            .parent_stats
            .into_iter()
            .filter(|(_, stats)| stats.discovered_edges == 0 && stats.exact_meets == 0)
            .map(|(key, stats)| ParentEvidenceRow::from_key_stats(&key, &stats))
            .collect::<Vec<_>>();
        low_yield_parents.sort_by(|left, right| {
            right
                .total_edges
                .cmp(&left.total_edges)
                .then_with(|| left.direction.cmp(&right.direction))
                .then_with(|| left.depth.cmp(&right.depth))
                .then_with(|| left.family.cmp(&right.family))
                .then_with(|| left.matrix.data.cmp(&right.matrix.data))
        });
        low_yield_parents.truncate(top_limit);

        let mut layers = self
            .layer_stats
            .into_iter()
            .map(|(key, stats)| LayerEvidenceRow {
                layer_index: key.layer_index,
                direction: key.direction.as_str().to_string(),
                total_edges: stats.total_edges,
                discovered_edges: stats.discovered_edges,
                seen_collisions: stats.seen_collisions,
                exact_meets: stats.exact_meets,
                approximate_hits: stats.approximate_hits,
                enqueued_edges: stats.enqueued_edges,
            })
            .collect::<Vec<_>>();
        layers.sort_by(|left, right| {
            left.layer_index
                .cmp(&right.layer_index)
                .then_with(|| left.direction.cmp(&right.direction))
        });

        StuckStateReport {
            case_id: CASE_ID.to_string(),
            source: request.source.clone(),
            target: request.target.clone(),
            config: request.config.clone(),
            result: result_label(result),
            telemetry: telemetry.clone(),
            family_evidence,
            layer_evidence: layers,
            ranked_approximate_hits: approximate_hits.into_iter().take(top_limit).collect(),
            ranked_family_approximate_sources: family_sources,
            low_yield_parents,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct ParentKey {
    direction: DirectionLabel,
    depth: usize,
    family: &'static str,
    matrix: DynMatrix,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
struct LayerKey {
    layer_index: usize,
    direction: DirectionLabel,
}

#[derive(Clone, Default)]
struct ParentStats {
    total_edges: usize,
    discovered_edges: usize,
    seen_collisions: usize,
    exact_meets: usize,
    approximate_hits: usize,
    enqueued_edges: usize,
}

type FamilyEvidence = ParentStats;
type LayerEvidence = ParentStats;

#[derive(Clone)]
struct StateVisit {
    direction: DirectionLabel,
    depth: usize,
    canonical: DynMatrix,
    orig: DynMatrix,
    signature: ApproxSignature,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
enum DirectionLabel {
    Forward,
    Backward,
}

impl DirectionLabel {
    fn as_str(self) -> &'static str {
        match self {
            Self::Forward => "forward",
            Self::Backward => "backward",
        }
    }
}

#[derive(Serialize)]
struct StuckStateReport {
    case_id: String,
    source: DynMatrix,
    target: DynMatrix,
    config: SearchConfig,
    result: String,
    telemetry: SearchTelemetry,
    family_evidence: Vec<FamilyEvidenceRow>,
    layer_evidence: Vec<LayerEvidenceRow>,
    ranked_approximate_hits: Vec<ApproximateHitEvidence>,
    ranked_family_approximate_sources: Vec<ParentEvidenceRow>,
    low_yield_parents: Vec<ParentEvidenceRow>,
}

#[derive(Serialize)]
struct FamilyEvidenceRow {
    family: String,
    total_edges: usize,
    discovered_edges: usize,
    seen_collisions: usize,
    exact_meets: usize,
    approximate_hits: usize,
    enqueued_edges: usize,
}

#[derive(Serialize)]
struct LayerEvidenceRow {
    layer_index: usize,
    direction: String,
    total_edges: usize,
    discovered_edges: usize,
    seen_collisions: usize,
    exact_meets: usize,
    approximate_hits: usize,
    enqueued_edges: usize,
}

#[derive(Clone, Serialize)]
struct ParentEvidenceRow {
    direction: String,
    depth: usize,
    family: String,
    total_edges: usize,
    discovered_edges: usize,
    seen_collisions: usize,
    exact_meets: usize,
    approximate_hits: usize,
    enqueued_edges: usize,
    matrix: DynMatrix,
    signature: ApproxSignature,
}

impl ParentEvidenceRow {
    fn from_key_stats(key: &ParentKey, stats: &ParentStats) -> Self {
        Self {
            direction: key.direction.as_str().to_string(),
            depth: key.depth,
            family: key.family.to_string(),
            total_edges: stats.total_edges,
            discovered_edges: stats.discovered_edges,
            seen_collisions: stats.seen_collisions,
            exact_meets: stats.exact_meets,
            approximate_hits: stats.approximate_hits,
            enqueued_edges: stats.enqueued_edges,
            matrix: key.matrix.clone(),
            signature: approx_signature(&key.matrix.canonical_perm()),
        }
    }
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
    enqueued: bool,
    status: String,
    signature: ApproxSignature,
    from_matrix: DynMatrix,
    to_matrix: DynMatrix,
    to_canonical: DynMatrix,
    counterpart_matrix: Option<DynMatrix>,
    counterpart_canonical: Option<DynMatrix>,
    step: StepSummary,
    parent_family_total_edges: usize,
    parent_family_discovered_edges: usize,
    parent_family_approximate_hits: usize,
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

impl Serialize for DirectionLabel {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

fn compare_approximate_hits(
    left: &ApproximateHitEvidence,
    right: &ApproximateHitEvidence,
) -> std::cmp::Ordering {
    bridge_slack_rank(left)
        .cmp(&bridge_slack_rank(right))
        .then_with(|| left.counterpart_l1.cmp(&right.counterpart_l1))
        .then_with(|| {
            right
                .parent_family_approximate_hits
                .cmp(&left.parent_family_approximate_hits)
        })
        .then_with(|| right.bridge_depth.cmp(&left.bridge_depth))
        .then_with(|| right.to_depth.cmp(&left.to_depth))
        .then_with(|| left.move_family.cmp(&right.move_family))
        .then_with(|| left.to_canonical.data.cmp(&right.to_canonical.data))
}

fn bridge_slack_rank(hit: &ApproximateHitEvidence) -> (u8, isize) {
    match hit.bridge_slack_at_lag40 {
        Some(slack) if slack >= 0 => (0, slack),
        Some(slack) => (1, -slack),
        None => (2, isize::MAX),
    }
}

fn compare_parent_sources(
    left: &ParentEvidenceRow,
    right: &ParentEvidenceRow,
) -> std::cmp::Ordering {
    right
        .approximate_hits
        .cmp(&left.approximate_hits)
        .then_with(|| right.discovered_edges.cmp(&left.discovered_edges))
        .then_with(|| right.total_edges.cmp(&left.total_edges))
        .then_with(|| right.depth.cmp(&left.depth))
        .then_with(|| left.family.cmp(&right.family))
        .then_with(|| left.matrix.data.cmp(&right.matrix.data))
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

fn status_label(status: SearchEdgeStatus) -> &'static str {
    match status {
        SearchEdgeStatus::SeenCollision => "seen_collision",
        SearchEdgeStatus::Discovered => "discovered",
        SearchEdgeStatus::ExactMeet => "exact_meet",
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
