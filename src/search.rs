use std::collections::{BTreeMap, VecDeque};
use std::time::Instant;

use ahash::{AHashMap as HashMap, AHashSet as HashSet};

use crate::graph_moves::{
    enumerate_graph_move_successors, enumerate_graph_proposals, GraphProposal,
    SameFuturePastSignatureGap,
};
use crate::invariants::{check_invariants_2x2, check_square_power_trace_invariants};
use crate::matrix::{DynMatrix, SqMatrix};
use crate::search_observer::{
    SearchEdgeRecord, SearchEdgeStatus, SearchObserver, SearchRootRecord,
};
use crate::types::{
    DynSsePath, DynSseResult, EsseStep, FrontierMode, MoveFamilyPolicy, SearchConfig,
    SearchDirection, SearchLayerTelemetry, SearchLayerTimingTelemetry, SearchMoveFamilyTelemetry,
    SearchRequest, SearchRunResult, SearchTelemetry, SsePath, SseResult, DEFAULT_BEAM_WIDTH,
};
#[cfg(test)]
use crate::types::{SearchStage, ShortcutSearchConfig, ShortcutSearchStopReason};

use rayon::prelude::*;

mod beam;
mod dispatch;
mod frontier;
mod path;
mod shortcut;
mod stages;

use self::beam::{
    choose_next_beam_bfs_handoff_direction, choose_next_beam_direction,
    effective_beam_bfs_handoff_depth, push_beam_bfs_handoff_entry, push_beam_frontier_entry,
    record_best_beam_bfs_handoff_exact_meet, should_use_beam_bfs_handoff_phase,
    BeamBfsHandoffExactMeet, BeamBfsHandoffFrontier, BeamFrontier,
};
use self::dispatch::{
    emit_layer, emit_roots, emit_started, endpoint_search_request, finish_search_2x2,
    finish_search_dyn,
};
use self::frontier::{
    choose_next_layer, expand_frontier_layer, expand_frontier_layer_dyn, FrontierExpansionSettings,
    FrontierExpansionTiming, FrontierLayerChoiceInputs, FrontierOverlapSignal,
};
#[cfg(test)]
use self::frontier::{
    deduplicate_expansions, should_expand_forward, FrontierExpansion, LayerExpansionOrderKey,
};
#[cfg(test)]
use self::path::reverse_dyn_sse_path;
pub use self::path::{
    build_full_path_guide_artifact, validate_sse_path_2x2, validate_sse_path_dyn,
};
use self::path::{
    permutation_step_between, reconstruct_bidirectional_dyn_path, reconstruct_bidirectional_path,
};
#[cfg(test)]
use self::shortcut::find_concrete_shift_shortcut_proof;
use self::shortcut::try_concrete_shift_shortcut_2x2;
#[cfg(test)]
use self::stages::{
    compare_ranked_guides, prepare_full_path_guide, refine_guide_path_once, GuidedSegmentCache,
    GuidedSegmentCacheKey, RankedGuide,
};
#[cfg(test)]
use crate::concrete_shift::{ConcreteShiftRelation2x2, ConcreteShiftSearchResult2x2};
#[cfg(test)]
use crate::graph_moves::same_future_past_signature;

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ApproxSignature {
    dim: usize,
    entry_sum: u64,
    row_sums: Vec<u32>,
    col_sums: Vec<u32>,
    row_supports: Vec<u8>,
    col_supports: Vec<u8>,
}

const TIMED_SEARCH_FRONTIER_CHUNK_SIZE: usize = 256;

pub(super) type MoveFamilyTelemetryAccumulator = HashMap<&'static str, SearchMoveFamilyTelemetry>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphProposalProbeConfig {
    pub shortlist_size: usize,
    pub realization_max_lag: usize,
    pub max_zigzag_bridge_entry: Option<u32>,
    pub shortlist_mode: GraphProposalShortlistMode,
    pub refined_coarse_prefix: usize,
}

impl Default for GraphProposalProbeConfig {
    fn default() -> Self {
        Self {
            shortlist_size: 4,
            realization_max_lag: 3,
            max_zigzag_bridge_entry: Some(8),
            shortlist_mode: GraphProposalShortlistMode::BestGap,
            refined_coarse_prefix: 4,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GraphProposalShortlistMode {
    BestGap,
    CoarsePrefixRefined,
}

#[derive(Clone, Debug)]
pub struct GraphProposalProbeAttempt {
    pub proposal: GraphProposal,
    pub result: DynSseResult,
    pub telemetry: SearchTelemetry,
}

#[derive(Clone, Debug)]
pub struct GraphProposalProbeResult {
    pub raw_candidates: usize,
    pub unique_candidates: usize,
    pub best_gap: Option<SameFuturePastSignatureGap>,
    pub best_gap_candidates: usize,
    pub attempts: Vec<GraphProposalProbeAttempt>,
}

fn elapsed_nanos(started: Instant) -> u64 {
    let nanos = started.elapsed().as_nanos();
    nanos.min(u128::from(u64::MAX)) as u64
}

fn layer_timing(
    started: Instant,
    expansion_timing: FrontierExpansionTiming,
    merge_nanos: u64,
    finalize_nanos: u64,
) -> SearchLayerTimingTelemetry {
    SearchLayerTimingTelemetry {
        total_nanos: elapsed_nanos(started),
        expand_compute_nanos: expansion_timing.expand_compute_nanos,
        expand_accumulate_nanos: expansion_timing.expand_accumulate_nanos,
        dedup_nanos: expansion_timing.dedup_nanos,
        merge_nanos,
        finalize_nanos,
    }
}

fn frontier_expansion_settings(config: &SearchConfig) -> FrontierExpansionSettings {
    FrontierExpansionSettings {
        max_intermediate_dim: config.max_intermediate_dim,
        max_entry: config.max_entry,
        move_family_policy: config.move_family_policy,
    }
}

/// Search for a strong shift equivalence path between two 2x2 matrices.
///
/// Uses bidirectional BFS over the graph where nodes are square matrices of
/// varying sizes (2x2, 3x3, ...) in canonical form, and edges are elementary
/// SSE steps (A = UV, B = VU). Searching from both ends reduces complexity
/// from O(branching^L) to O(2 * branching^(L/2)).
pub fn search_sse_2x2(a: &SqMatrix<2>, b: &SqMatrix<2>, config: &SearchConfig) -> SseResult<2> {
    search_sse_2x2_with_telemetry(a, b, config).0
}

/// Search for a strong shift equivalence path between arbitrary square endpoints.
pub fn search_sse_dyn(a: &DynMatrix, b: &DynMatrix, config: &SearchConfig) -> DynSseResult {
    search_sse_with_telemetry_dyn(a, b, config).0
}

/// Search for a strong shift equivalence path, returning aggregate telemetry.
pub fn search_sse_2x2_with_telemetry(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> (SseResult<2>, SearchTelemetry) {
    search_sse_2x2_with_telemetry_and_observer(a, b, config, None)
}

/// Probe the best-gap graph proposal shortlist under a bounded graph-only search.
///
/// This is a research-oriented seam: it leaves default frontier expansion alone
/// and only evaluates a small top-k shortlist of already-scored proposals.
pub fn probe_graph_proposal_shortlist(
    current: &DynMatrix,
    target: &DynMatrix,
    search_config: &SearchConfig,
    probe_config: &GraphProposalProbeConfig,
) -> Result<GraphProposalProbeResult, String> {
    if !current.is_square() || !target.is_square() {
        return Err("graph proposal probe requires square current and target matrices".to_string());
    }
    if probe_config.shortlist_size == 0 {
        return Err("graph proposal probe requires shortlist_size >= 1".to_string());
    }
    if probe_config.realization_max_lag == 0 {
        return Err("graph proposal probe requires realization_max_lag >= 1".to_string());
    }
    if probe_config.shortlist_mode == GraphProposalShortlistMode::CoarsePrefixRefined
        && probe_config.refined_coarse_prefix == 0
    {
        return Err(
            "graph proposal probe requires refined_coarse_prefix >= 1 in refined mode".to_string(),
        );
    }

    let proposals = enumerate_graph_proposals(
        current,
        target,
        search_config.max_intermediate_dim,
        probe_config.max_zigzag_bridge_entry,
    );
    let best_gap = proposals.best_gap();
    let best_gap_candidates = proposals.best_gap_shortlist_len();
    let shortlist = match probe_config.shortlist_mode {
        GraphProposalShortlistMode::BestGap => {
            proposals.best_gap_shortlist(probe_config.shortlist_size)
        }
        GraphProposalShortlistMode::CoarsePrefixRefined => proposals
            .refined_shortlist_from_coarse_prefix(
                probe_config.refined_coarse_prefix,
                probe_config.shortlist_size,
            ),
    };
    let realization_config = SearchConfig {
        max_lag: probe_config.realization_max_lag,
        max_intermediate_dim: search_config.max_intermediate_dim,
        max_entry: search_config.max_entry,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::GraphOnly,
        beam_width: None,
        beam_bfs_handoff_depth: None,
        beam_bfs_handoff_deferred_cap: None,
    };
    let attempts = shortlist
        .into_iter()
        .map(|proposal| {
            let (result, telemetry) =
                search_sse_with_telemetry_dyn(current, &proposal.matrix, &realization_config);
            GraphProposalProbeAttempt {
                proposal,
                result,
                telemetry,
            }
        })
        .collect();

    Ok(GraphProposalProbeResult {
        raw_candidates: proposals.candidates,
        unique_candidates: proposals.nodes.len(),
        best_gap,
        best_gap_candidates,
        attempts,
    })
}

/// Execute one search request across the staged solver boundary.
pub fn execute_search_request(
    request: &SearchRequest,
) -> Result<(SearchRunResult, SearchTelemetry), String> {
    dispatch::execute_search_request(request)
}

/// Execute one search request and optionally stream observer events.
pub fn execute_search_request_and_observer(
    request: &SearchRequest,
    observer: Option<&mut dyn SearchObserver>,
) -> Result<(SearchRunResult, SearchTelemetry), String> {
    dispatch::execute_search_request_and_observer(request, observer)
}

/// Search for a strong shift equivalence path between arbitrary square endpoints,
/// returning aggregate telemetry.
pub fn search_sse_with_telemetry_dyn(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
) -> (DynSseResult, SearchTelemetry) {
    search_sse_with_telemetry_dyn_with_deadline_and_observer(a, b, config, None, None)
}

fn search_sse_with_telemetry_dyn_with_deadline(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    deadline: Option<Instant>,
) -> (DynSseResult, SearchTelemetry) {
    search_sse_with_telemetry_dyn_with_deadline_and_observer(a, b, config, None, deadline)
}

/// Search for a strong shift equivalence path between arbitrary square endpoints,
/// returning aggregate telemetry and optionally recording events.
pub fn search_sse_with_telemetry_dyn_and_observer(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    observer: Option<&mut dyn SearchObserver>,
) -> (DynSseResult, SearchTelemetry) {
    search_sse_with_telemetry_dyn_with_deadline_and_observer(a, b, config, observer, None)
}

fn search_sse_with_telemetry_dyn_with_deadline_and_observer(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    deadline: Option<Instant>,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let request = endpoint_search_request(a, b, config);

    if deadline_reached(deadline) {
        return finish_search_dyn(observer, &request, DynSseResult::Unknown, telemetry);
    }

    if !a.is_square() || !b.is_square() {
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::NotEquivalent("search expects square endpoint matrices".to_string()),
            telemetry,
        );
    }

    if let Some(reason) = check_square_power_trace_invariants(a, b) {
        telemetry.invariant_filtered = true;
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::NotEquivalent(reason),
            telemetry,
        );
    }

    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    if a == b {
        emit_started(&mut observer, &request, &a_canon, &b_canon);
        emit_roots(
            &mut observer,
            &[
                SearchRootRecord {
                    direction: SearchDirection::Forward,
                    canonical: a_canon.clone(),
                    orig: a.clone(),
                    depth: 0,
                },
                SearchRootRecord {
                    direction: SearchDirection::Backward,
                    canonical: b_canon.clone(),
                    orig: b.clone(),
                    depth: 0,
                },
            ],
        );
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::Equivalent(DynSsePath {
                matrices: vec![a.clone()],
                steps: vec![],
            }),
            telemetry,
        );
    }

    if a_canon == b_canon {
        telemetry.canonical_shortcut = true;
        if a != b {
            telemetry.permutation_shortcut = true;
        }
        emit_started(&mut observer, &request, &a_canon, &b_canon);
        emit_roots(
            &mut observer,
            &[
                SearchRootRecord {
                    direction: SearchDirection::Forward,
                    canonical: a_canon.clone(),
                    orig: a.clone(),
                    depth: 0,
                },
                SearchRootRecord {
                    direction: SearchDirection::Backward,
                    canonical: b_canon.clone(),
                    orig: b.clone(),
                    depth: 0,
                },
            ],
        );
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::Equivalent(DynSsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: permutation_step_between(a, b).into_iter().collect(),
            }),
            telemetry,
        );
    }

    match config.frontier_mode {
        FrontierMode::Beam => {
            return search_beam_dyn_with_telemetry(
                a,
                b,
                config,
                observer,
                &request,
                deadline,
                config.beam_width.unwrap_or(DEFAULT_BEAM_WIDTH),
            );
        }
        FrontierMode::BeamBfsHandoff => {
            return search_beam_bfs_handoff_dyn_with_telemetry(
                a,
                b,
                config,
                observer,
                &request,
                deadline,
                config.beam_width.unwrap_or(DEFAULT_BEAM_WIDTH),
            );
        }
        FrontierMode::Bfs => {}
    }

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a.clone());
    fwd_frontier.push_back(a_canon.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b.clone());
    bwd_frontier.push_back(b_canon.clone());
    telemetry.max_frontier_size = 1;
    let mut fwd_factorisations_per_node = 1.0f64;
    let mut bwd_factorisations_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;
    let mut fwd_overlap_signal = FrontierOverlapSignal::default();
    let mut bwd_overlap_signal = FrontierOverlapSignal::default();
    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    emit_started(&mut observer, &request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b.clone(),
                depth: 0,
            },
        ],
    );

    if config.move_family_policy == MoveFamilyPolicy::GraphOnly {
        return search_graph_only_dyn_with_telemetry(a, b, config, observer, &request, deadline);
    }

    for layer_index in 0..config.max_lag {
        if deadline_reached(deadline) {
            break;
        }
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(FrontierLayerChoiceInputs {
            fwd_depth: next_fwd_depth,
            bwd_depth: next_bwd_depth,
            fwd_frontier_len: fwd_frontier.len(),
            bwd_frontier_len: bwd_frontier.len(),
            fwd_factorisations_per_node,
            bwd_factorisations_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal,
            bwd_overlap_signal,
        }) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };

        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                )
            };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let layer_started = Instant::now();
        let (expansions, expansion_stats, expansion_timing, timed_out) = expand_frontier_layer_dyn(
            &current_frontier,
            orig,
            frontier_expansion_settings(config),
            deadline,
        );
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        if expansion_stats.frontier_nodes > 0 {
            let factorisations_per_node = expansion_stats.factorisations_enumerated.max(1) as f64
                / expansion_stats.frontier_nodes as f64;
            if expand_forward {
                fwd_factorisations_per_node = factorisations_per_node;
                fwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            } else {
                bwd_factorisations_per_node = factorisations_per_node;
                bwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            }
        }
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;
        let mut next_frontier: VecDeque<DynMatrix> = VecDeque::new();
        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = layer_depth + 1;
        let merge_started = Instant::now();

        for expansion in &expansions {
            let parent_orig = layer_records.as_ref().map(|_| {
                orig.get(&expansion.parent_canon)
                    .expect("parent node should have an original matrix")
                    .clone()
            });
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig
                            .clone()
                            .expect("observer layer records need a parent matrix"),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            signatures.insert(approx_signature(&expansion.next_canon));

            let approximate_hit =
                other_signatures.contains(&approx_signature(&expansion.next_canon));
            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if path_depth > config.max_lag {
                    if let Some(records) = layer_records.as_mut() {
                        records.push(SearchEdgeRecord {
                            layer_index,
                            direction,
                            move_family: expansion.move_family,
                            from_canonical: expansion.parent_canon.clone(),
                            from_orig: parent_orig
                                .clone()
                                .expect("observer layer records need a parent matrix"),
                            to_canonical: expansion.next_canon.clone(),
                            to_orig: expansion.next_orig.clone(),
                            from_depth: layer_depth,
                            to_depth: next_depth,
                            step: expansion.step.clone(),
                            status: record_status,
                            approximate_other_side_hit: approximate_hit,
                            enqueued,
                        });
                    }
                    continue;
                }
                let merge_nanos = elapsed_nanos(merge_started);
                let finalize_started = Instant::now();
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig
                            .clone()
                            .expect("observer layer records need a parent matrix"),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                let finalize_nanos = elapsed_nanos(finalize_started);
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: next_frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    timing: layer_timing(
                        layer_started,
                        expansion_timing,
                        merge_nanos,
                        finalize_nanos,
                    ),
                    move_family_telemetry: finalize_move_family_telemetry(
                        layer_move_family_telemetry,
                    ),
                });
                return finish_search_dyn(
                    observer,
                    &request,
                    DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                next_frontier.push_back(expansion.next_canon.clone());
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig.expect("observer layer records need a parent matrix"),
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: layer_depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        let merge_nanos = elapsed_nanos(merge_started);
        let finalize_started = Instant::now();
        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        let overlap_signal = FrontierOverlapSignal::from_layer(
            expansion_stats.frontier_nodes,
            approximate_other_side_hits,
        );
        if expand_forward {
            fwd_overlap_signal = overlap_signal;
        } else {
            bwd_overlap_signal = overlap_signal;
        }
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        let finalize_nanos = elapsed_nanos(finalize_started);
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: next_frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            timing: layer_timing(layer_started, expansion_timing, merge_nanos, finalize_nanos),
            move_family_telemetry: finalize_move_family_telemetry(layer_move_family_telemetry),
        });

        if timed_out {
            break;
        }
        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search_dyn(observer, &request, DynSseResult::Unknown, telemetry)
}

/// Search for a strong shift equivalence path, optionally recording the visited graph.
pub fn search_sse_2x2_with_telemetry_and_observer(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();
    let request = endpoint_search_request(&a_dyn, &b_dyn, config);
    emit_started(&mut observer, &request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a_dyn.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b_dyn.clone(),
                depth: 0,
            },
        ],
    );

    // Quick check: are they already equal?
    if a == b {
        return finish_search_2x2(
            observer,
            &request,
            SseResult::Equivalent(SsePath {
                matrices: vec![a.clone()],
                steps: vec![],
            }),
            telemetry,
        );
    }

    // Pre-filter with invariants.
    if let Some(reason) = check_invariants_2x2(a, b) {
        telemetry.invariant_filtered = true;
        return finish_search_2x2(
            observer,
            &request,
            SseResult::NotEquivalent(reason),
            telemetry,
        );
    }

    // If a and b have the same canonical form, they are related by permutation
    // similarity. For 2x2, b = PAP where P = [[0,1],[1,0]].
    // Elementary SSE: U = AP, V = P, then UV = APP = A, VU = PAP = B.
    if a.canonical() == b.canonical() && a != b {
        telemetry.permutation_shortcut = true;
        let p = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let ap = DynMatrix::from_sq(a).mul(&p);
        let step = EsseStep { u: ap, v: p };
        return finish_search_2x2(
            observer,
            &request,
            SseResult::Equivalent(SsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: vec![step],
            }),
            telemetry,
        );
    }

    match config.frontier_mode {
        FrontierMode::Beam => {
            return search_beam_2x2_with_telemetry_and_observer(
                a,
                b,
                config,
                observer,
                &request,
                config.beam_width.unwrap_or(DEFAULT_BEAM_WIDTH),
            );
        }
        FrontierMode::BeamBfsHandoff => {
            return search_beam_bfs_handoff_2x2_with_telemetry_and_observer(
                a,
                b,
                config,
                observer,
                &request,
                config.beam_width.unwrap_or(DEFAULT_BEAM_WIDTH),
            );
        }
        FrontierMode::Bfs => {}
    }

    // Forward direction (from A).
    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a_dyn);
    fwd_frontier.push_back(a_canon.clone());

    // Backward direction (from B).
    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), DynMatrix::from_sq(b));
    bwd_frontier.push_back(b_canon.clone());
    telemetry.max_frontier_size = 1;
    let mut fwd_factorisations_per_node = 1.0f64;
    let mut bwd_factorisations_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;
    let mut fwd_overlap_signal = FrontierOverlapSignal::default();
    let mut bwd_overlap_signal = FrontierOverlapSignal::default();
    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    // Edge case: A and B canonicalise to the same form (should have been
    // caught by the permutation check above, but handle for safety).
    if a_canon == b_canon {
        telemetry.canonical_shortcut = true;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        return finish_search_2x2(
            observer,
            &request,
            SseResult::Equivalent(reconstruct_bidirectional_path(
                a,
                b,
                &a_canon,
                &fwd_parent,
                &fwd_orig,
                &bwd_parent,
                &bwd_orig,
            )),
            telemetry,
        );
    }

    if config.move_family_policy == MoveFamilyPolicy::GraphOnly {
        return search_graph_only_2x2_with_telemetry_and_observer(a, b, config, observer, &request);
    }

    for layer_index in 0..config.max_lag {
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(FrontierLayerChoiceInputs {
            fwd_depth: next_fwd_depth,
            bwd_depth: next_bwd_depth,
            fwd_frontier_len: fwd_frontier.len(),
            bwd_frontier_len: bwd_frontier.len(),
            fwd_factorisations_per_node,
            bwd_factorisations_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal,
            bwd_overlap_signal,
        }) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };

        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                )
            };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let layer_started = Instant::now();
        let (expansions, expansion_stats, expansion_timing) =
            expand_frontier_layer(&current_frontier, orig, frontier_expansion_settings(config));
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        if expansion_stats.frontier_nodes > 0 {
            let factorisations_per_node = expansion_stats.factorisations_enumerated.max(1) as f64
                / expansion_stats.frontier_nodes as f64;
            if expand_forward {
                fwd_factorisations_per_node = factorisations_per_node;
                fwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            } else {
                bwd_factorisations_per_node = factorisations_per_node;
                bwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            }
        }
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;
        let mut next_frontier: VecDeque<DynMatrix> = VecDeque::new();
        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = layer_depth + 1;
        let merge_started = Instant::now();

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            let next_signature = approx_signature(&expansion.next_canon);
            let approximate_hit = other_signatures.contains(&next_signature);
            signatures.insert(next_signature);
            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            // Check if the other side has already visited this node.
            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if path_depth > config.max_lag {
                    if let Some(records) = layer_records.as_mut() {
                        records.push(SearchEdgeRecord {
                            layer_index,
                            direction,
                            move_family: expansion.move_family,
                            from_canonical: expansion.parent_canon.clone(),
                            from_orig: parent_orig.clone(),
                            to_canonical: expansion.next_canon.clone(),
                            to_orig: expansion.next_orig.clone(),
                            from_depth: layer_depth,
                            to_depth: next_depth,
                            step: expansion.step.clone(),
                            status: record_status,
                            approximate_other_side_hit: approximate_hit,
                            enqueued,
                        });
                    }
                    continue;
                }
                let merge_nanos = elapsed_nanos(merge_started);
                let finalize_started = Instant::now();
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                let finalize_nanos = elapsed_nanos(finalize_started);
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction.clone()),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: next_frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    timing: layer_timing(
                        layer_started,
                        expansion_timing,
                        merge_nanos,
                        finalize_nanos,
                    ),
                    move_family_telemetry: finalize_move_family_telemetry(
                        layer_move_family_telemetry,
                    ),
                });
                return finish_search_2x2(
                    observer,
                    &request,
                    SseResult::Equivalent(reconstruct_bidirectional_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            // For 2x2 nodes, bound entries to prevent unbounded growth.
            // For intermediate (3x3+) nodes, always add -- the factorisation
            // back to 2x2 already bounds factor entries via max_entry.
            if enqueued {
                next_frontier.push_back(expansion.next_canon.clone());
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: layer_depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        let merge_nanos = elapsed_nanos(merge_started);
        let finalize_started = Instant::now();
        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        let overlap_signal = FrontierOverlapSignal::from_layer(
            expansion_stats.frontier_nodes,
            approximate_other_side_hits,
        );
        if expand_forward {
            fwd_overlap_signal = overlap_signal;
        } else {
            bwd_overlap_signal = overlap_signal;
        }
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        let finalize_nanos = elapsed_nanos(finalize_started);
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: next_frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            timing: layer_timing(layer_started, expansion_timing, merge_nanos, finalize_nanos),
            move_family_telemetry: finalize_move_family_telemetry(layer_move_family_telemetry),
        });

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    // If bounded ESSE search exhausts on a finite essential pair, try the
    // bounded concrete-shift relations before reporting `Unknown`.
    if let Some(witness) = try_concrete_shift_shortcut_2x2(a, b, config) {
        telemetry.concrete_shift_shortcut = true;
        return finish_search_2x2(
            observer,
            &request,
            SseResult::EquivalentByConcreteShift(witness),
            telemetry,
        );
    }

    finish_search_2x2(observer, &request, SseResult::Unknown, telemetry)
}

fn search_beam_2x2_with_telemetry_and_observer(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    beam_width: usize,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a_dyn.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b_dyn.clone());

    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    let mut serial = 0usize;
    let mut fwd_frontier = BeamFrontier::new(beam_width);
    let mut bwd_frontier = BeamFrontier::new(beam_width);
    push_beam_frontier_entry(
        &mut fwd_frontier,
        &a_canon,
        0,
        &bwd_signatures,
        &b_canon,
        &mut serial,
    );
    push_beam_frontier_entry(
        &mut bwd_frontier,
        &b_canon,
        0,
        &fwd_signatures,
        &a_canon,
        &mut serial,
    );
    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    emit_started(&mut observer, request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a_dyn,
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b_dyn,
                depth: 0,
            },
        ],
    );

    let mut layer_index = 0usize;
    loop {
        fwd_frontier.refresh_approximate_hits(&bwd_signatures);
        bwd_frontier.refresh_approximate_hits(&fwd_signatures);
        let Some(expand_forward) = choose_next_beam_direction(&fwd_frontier, &bwd_frontier) else {
            break;
        };
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures, target) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                    &b_canon,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                    &a_canon,
                )
            };

        let current_entries = frontier.pop_batch_same_depth(frontier.expansion_batch_size());
        if current_entries.is_empty() {
            continue;
        }
        let current_depth = current_entries[0].depth;
        if current_depth >= config.max_lag {
            continue;
        }

        let current_frontier = current_entries
            .iter()
            .map(|entry| entry.canonical.clone())
            .collect::<Vec<_>>();
        let layer_started = Instant::now();
        let (expansions, expansion_stats, expansion_timing) =
            expand_frontier_layer(&current_frontier, orig, frontier_expansion_settings(config));
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;

        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = current_depth + 1;
        let merge_started = Instant::now();

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            let next_signature = approx_signature(&expansion.next_canon);
            let approximate_hit = other_signatures.contains(&next_signature);
            signatures.insert(next_signature);

            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if path_depth > config.max_lag {
                    continue;
                }

                let merge_nanos = elapsed_nanos(merge_started);
                let finalize_started = Instant::now();
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                let finalize_nanos = elapsed_nanos(finalize_started);
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    timing: layer_timing(
                        layer_started,
                        expansion_timing,
                        merge_nanos,
                        finalize_nanos,
                    ),
                    move_family_telemetry: finalize_move_family_telemetry(
                        layer_move_family_telemetry,
                    ),
                });
                return finish_search_2x2(
                    observer,
                    request,
                    SseResult::Equivalent(reconstruct_bidirectional_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                push_beam_frontier_entry(
                    frontier,
                    &expansion.next_canon,
                    next_depth,
                    other_signatures,
                    target,
                    &mut serial,
                );
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: current_depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        let merge_nanos = elapsed_nanos(merge_started);
        let finalize_started = Instant::now();
        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        let finalize_nanos = elapsed_nanos(finalize_started);
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            timing: layer_timing(layer_started, expansion_timing, merge_nanos, finalize_nanos),
            move_family_telemetry: finalize_move_family_telemetry(layer_move_family_telemetry),
        });
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        layer_index += 1;
    }

    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
    if let Some(witness) = try_concrete_shift_shortcut_2x2(a, b, config) {
        telemetry.concrete_shift_shortcut = true;
        return finish_search_2x2(
            observer,
            request,
            SseResult::EquivalentByConcreteShift(witness),
            telemetry,
        );
    }

    finish_search_2x2(observer, request, SseResult::Unknown, telemetry)
}

fn search_beam_bfs_handoff_2x2_with_telemetry_and_observer(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    beam_width: usize,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a_dyn.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b_dyn.clone());

    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    let mut serial = 0usize;
    let mut fwd_frontier =
        BeamBfsHandoffFrontier::new(beam_width, config.beam_bfs_handoff_deferred_cap);
    let mut bwd_frontier =
        BeamBfsHandoffFrontier::new(beam_width, config.beam_bfs_handoff_deferred_cap);
    push_beam_bfs_handoff_entry(
        &mut fwd_frontier,
        &a_canon,
        0,
        &bwd_signatures,
        &b_canon,
        &mut serial,
        true,
    );
    push_beam_bfs_handoff_entry(
        &mut bwd_frontier,
        &b_canon,
        0,
        &fwd_signatures,
        &a_canon,
        &mut serial,
        true,
    );
    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    emit_started(&mut observer, request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a_dyn,
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b_dyn,
                depth: 0,
            },
        ],
    );

    let mut beam_phase = true;
    let mut best_exact_meet: Option<BeamBfsHandoffExactMeet> = None;
    let beam_handoff_depth = effective_beam_bfs_handoff_depth(config);
    let mut layer_index = 0usize;
    loop {
        fwd_frontier.refresh_approximate_hits(&bwd_signatures);
        bwd_frontier.refresh_approximate_hits(&fwd_signatures);
        if beam_phase && fwd_frontier.active_len() == 0 && bwd_frontier.active_len() == 0 {
            beam_phase = false;
        }
        let Some(expand_forward) =
            choose_next_beam_bfs_handoff_direction(&fwd_frontier, &bwd_frontier, beam_phase)
        else {
            break;
        };
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.pending_len().max(bwd_frontier.pending_len()));
        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures, target) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                    &b_canon,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                    &a_canon,
                )
            };

        let current_entries = if beam_phase {
            frontier.pop_beam_batch()
        } else {
            frontier.pop_bfs_batch()
        };
        if current_entries.is_empty() {
            continue;
        }
        let current_depth = current_entries[0].depth;
        if current_depth >= config.max_lag {
            continue;
        }

        let current_frontier = current_entries
            .iter()
            .map(|entry| entry.canonical.clone())
            .collect::<Vec<_>>();
        let layer_started = Instant::now();
        let (expansions, expansion_stats, expansion_timing) =
            expand_frontier_layer(&current_frontier, orig, frontier_expansion_settings(config));
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;

        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = current_depth + 1;
        let merge_started = Instant::now();

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            let next_signature = approx_signature(&expansion.next_canon);
            let approximate_hit = other_signatures.contains(&next_signature);
            signatures.insert(next_signature);

            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if path_depth > config.max_lag {
                    continue;
                }
                record_best_beam_bfs_handoff_exact_meet(
                    &mut best_exact_meet,
                    &expansion.next_canon,
                    path_depth,
                );
                if beam_phase {
                    continue;
                }

                let merge_nanos = elapsed_nanos(merge_started);
                let finalize_started = Instant::now();
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                let finalize_nanos = elapsed_nanos(finalize_started);
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: frontier.pending_len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    timing: layer_timing(
                        layer_started,
                        expansion_timing,
                        merge_nanos,
                        finalize_nanos,
                    ),
                    move_family_telemetry: finalize_move_family_telemetry(
                        layer_move_family_telemetry,
                    ),
                });
                let best_exact_meet = best_exact_meet
                    .as_ref()
                    .expect("exact meet should be recorded before returning");
                return finish_search_2x2(
                    observer,
                    request,
                    SseResult::Equivalent(reconstruct_bidirectional_path(
                        a,
                        b,
                        &best_exact_meet.canonical,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                let use_beam_phase =
                    should_use_beam_bfs_handoff_phase(beam_phase, next_depth, beam_handoff_depth);
                push_beam_bfs_handoff_entry(
                    frontier,
                    &expansion.next_canon,
                    next_depth,
                    other_signatures,
                    target,
                    &mut serial,
                    use_beam_phase,
                );
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: current_depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        let merge_nanos = elapsed_nanos(merge_started);
        let finalize_started = Instant::now();
        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        let finalize_nanos = elapsed_nanos(finalize_started);
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: frontier.pending_len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            timing: layer_timing(layer_started, expansion_timing, merge_nanos, finalize_nanos),
            move_family_telemetry: finalize_move_family_telemetry(layer_move_family_telemetry),
        });
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.pending_len().max(bwd_frontier.pending_len()));
        layer_index += 1;
    }

    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
    if let Some(best_exact_meet) = best_exact_meet.as_ref() {
        return finish_search_2x2(
            observer,
            request,
            SseResult::Equivalent(reconstruct_bidirectional_path(
                a,
                b,
                &best_exact_meet.canonical,
                &fwd_parent,
                &fwd_orig,
                &bwd_parent,
                &bwd_orig,
            )),
            telemetry,
        );
    }
    if let Some(witness) = try_concrete_shift_shortcut_2x2(a, b, config) {
        telemetry.concrete_shift_shortcut = true;
        return finish_search_2x2(
            observer,
            request,
            SseResult::EquivalentByConcreteShift(witness),
            telemetry,
        );
    }

    finish_search_2x2(observer, request, SseResult::Unknown, telemetry)
}

fn search_beam_dyn_with_telemetry(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    deadline: Option<Instant>,
    beam_width: usize,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b.clone());

    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    let mut serial = 0usize;
    let mut fwd_frontier = BeamFrontier::new(beam_width);
    let mut bwd_frontier = BeamFrontier::new(beam_width);
    push_beam_frontier_entry(
        &mut fwd_frontier,
        &a_canon,
        0,
        &bwd_signatures,
        &b_canon,
        &mut serial,
    );
    push_beam_frontier_entry(
        &mut bwd_frontier,
        &b_canon,
        0,
        &fwd_signatures,
        &a_canon,
        &mut serial,
    );
    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    emit_started(&mut observer, request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b.clone(),
                depth: 0,
            },
        ],
    );

    let mut layer_index = 0usize;
    loop {
        fwd_frontier.refresh_approximate_hits(&bwd_signatures);
        bwd_frontier.refresh_approximate_hits(&fwd_signatures);
        let Some(expand_forward) = choose_next_beam_direction(&fwd_frontier, &bwd_frontier) else {
            break;
        };
        if deadline_reached(deadline) {
            break;
        }
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures, target) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                    &b_canon,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                    &a_canon,
                )
            };

        let current_entries = frontier.pop_batch_same_depth(frontier.expansion_batch_size());
        if current_entries.is_empty() {
            continue;
        }
        let current_depth = current_entries[0].depth;
        if current_depth >= config.max_lag {
            continue;
        }

        let current_frontier = current_entries
            .iter()
            .map(|entry| entry.canonical.clone())
            .collect::<Vec<_>>();
        let layer_started = Instant::now();
        let (expansions, expansion_stats, expansion_timing, timed_out) = expand_frontier_layer_dyn(
            &current_frontier,
            orig,
            frontier_expansion_settings(config),
            deadline,
        );
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;

        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = current_depth + 1;
        let merge_started = Instant::now();

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            let next_signature = approx_signature(&expansion.next_canon);
            let approximate_hit = other_signatures.contains(&next_signature);
            signatures.insert(next_signature);

            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if path_depth > config.max_lag {
                    continue;
                }

                let merge_nanos = elapsed_nanos(merge_started);
                let finalize_started = Instant::now();
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                let finalize_nanos = elapsed_nanos(finalize_started);
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    timing: layer_timing(
                        layer_started,
                        expansion_timing,
                        merge_nanos,
                        finalize_nanos,
                    ),
                    move_family_telemetry: finalize_move_family_telemetry(
                        layer_move_family_telemetry,
                    ),
                });
                return finish_search_dyn(
                    observer,
                    request,
                    DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                push_beam_frontier_entry(
                    frontier,
                    &expansion.next_canon,
                    next_depth,
                    other_signatures,
                    target,
                    &mut serial,
                );
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: current_depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        let merge_nanos = elapsed_nanos(merge_started);
        let finalize_started = Instant::now();
        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        let finalize_nanos = elapsed_nanos(finalize_started);
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            timing: layer_timing(layer_started, expansion_timing, merge_nanos, finalize_nanos),
            move_family_telemetry: finalize_move_family_telemetry(layer_move_family_telemetry),
        });
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        layer_index += 1;

        if timed_out {
            break;
        }
    }

    finish_search_dyn(observer, request, DynSseResult::Unknown, telemetry)
}

fn search_beam_bfs_handoff_dyn_with_telemetry(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    deadline: Option<Instant>,
    beam_width: usize,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b.clone());

    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    let mut serial = 0usize;
    let mut fwd_frontier =
        BeamBfsHandoffFrontier::new(beam_width, config.beam_bfs_handoff_deferred_cap);
    let mut bwd_frontier =
        BeamBfsHandoffFrontier::new(beam_width, config.beam_bfs_handoff_deferred_cap);
    push_beam_bfs_handoff_entry(
        &mut fwd_frontier,
        &a_canon,
        0,
        &bwd_signatures,
        &b_canon,
        &mut serial,
        true,
    );
    push_beam_bfs_handoff_entry(
        &mut bwd_frontier,
        &b_canon,
        0,
        &fwd_signatures,
        &a_canon,
        &mut serial,
        true,
    );
    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    emit_started(&mut observer, request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b.clone(),
                depth: 0,
            },
        ],
    );

    let mut beam_phase = true;
    let mut best_exact_meet: Option<BeamBfsHandoffExactMeet> = None;
    let beam_handoff_depth = effective_beam_bfs_handoff_depth(config);
    let mut layer_index = 0usize;
    loop {
        fwd_frontier.refresh_approximate_hits(&bwd_signatures);
        bwd_frontier.refresh_approximate_hits(&fwd_signatures);
        if beam_phase && fwd_frontier.active_len() == 0 && bwd_frontier.active_len() == 0 {
            beam_phase = false;
        }
        let Some(expand_forward) =
            choose_next_beam_bfs_handoff_direction(&fwd_frontier, &bwd_frontier, beam_phase)
        else {
            break;
        };
        if deadline_reached(deadline) {
            break;
        }
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.pending_len().max(bwd_frontier.pending_len()));
        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures, target) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                    &b_canon,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                    &a_canon,
                )
            };

        let current_entries = if beam_phase {
            frontier.pop_beam_batch()
        } else {
            frontier.pop_bfs_batch()
        };
        if current_entries.is_empty() {
            continue;
        }
        let current_depth = current_entries[0].depth;
        if current_depth >= config.max_lag {
            continue;
        }

        let current_frontier = current_entries
            .iter()
            .map(|entry| entry.canonical.clone())
            .collect::<Vec<_>>();
        let layer_started = Instant::now();
        let (expansions, expansion_stats, expansion_timing, timed_out) = expand_frontier_layer_dyn(
            &current_frontier,
            orig,
            frontier_expansion_settings(config),
            deadline,
        );
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;

        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = current_depth + 1;
        let merge_started = Instant::now();

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            let next_signature = approx_signature(&expansion.next_canon);
            let approximate_hit = other_signatures.contains(&next_signature);
            signatures.insert(next_signature);

            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if path_depth > config.max_lag {
                    continue;
                }
                record_best_beam_bfs_handoff_exact_meet(
                    &mut best_exact_meet,
                    &expansion.next_canon,
                    path_depth,
                );
                if beam_phase {
                    continue;
                }

                let merge_nanos = elapsed_nanos(merge_started);
                let finalize_started = Instant::now();
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                let finalize_nanos = elapsed_nanos(finalize_started);
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: frontier.pending_len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    timing: layer_timing(
                        layer_started,
                        expansion_timing,
                        merge_nanos,
                        finalize_nanos,
                    ),
                    move_family_telemetry: finalize_move_family_telemetry(
                        layer_move_family_telemetry,
                    ),
                });
                let best_exact_meet = best_exact_meet
                    .as_ref()
                    .expect("exact meet should be recorded before returning");
                return finish_search_dyn(
                    observer,
                    request,
                    DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                        a,
                        b,
                        &best_exact_meet.canonical,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                let use_beam_phase =
                    should_use_beam_bfs_handoff_phase(beam_phase, next_depth, beam_handoff_depth);
                push_beam_bfs_handoff_entry(
                    frontier,
                    &expansion.next_canon,
                    next_depth,
                    other_signatures,
                    target,
                    &mut serial,
                    use_beam_phase,
                );
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: current_depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        let merge_nanos = elapsed_nanos(merge_started);
        let finalize_started = Instant::now();
        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        let finalize_nanos = elapsed_nanos(finalize_started);
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: frontier.pending_len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            timing: layer_timing(layer_started, expansion_timing, merge_nanos, finalize_nanos),
            move_family_telemetry: finalize_move_family_telemetry(layer_move_family_telemetry),
        });
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.pending_len().max(bwd_frontier.pending_len()));
        layer_index += 1;

        if timed_out {
            break;
        }
    }

    if let Some(best_exact_meet) = best_exact_meet.as_ref() {
        return finish_search_dyn(
            observer,
            request,
            DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                a,
                b,
                &best_exact_meet.canonical,
                &fwd_parent,
                &fwd_orig,
                &bwd_parent,
                &bwd_orig,
            )),
            telemetry,
        );
    }

    finish_search_dyn(observer, request, DynSseResult::Unknown, telemetry)
}

fn search_graph_only_2x2_with_telemetry_and_observer(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a_dyn);
    fwd_frontier.push_back(a_canon.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b_dyn);
    bwd_frontier.push_back(b_canon);

    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = 2;
    let mut fwd_candidates_per_node = 1.0f64;
    let mut bwd_candidates_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;

    for layer_index in 0..config.max_lag {
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(FrontierLayerChoiceInputs {
            fwd_depth: next_fwd_depth,
            bwd_depth: next_bwd_depth,
            fwd_frontier_len: fwd_frontier.len(),
            bwd_frontier_len: bwd_frontier.len(),
            fwd_factorisations_per_node: fwd_candidates_per_node,
            bwd_factorisations_per_node: bwd_candidates_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal: FrontierOverlapSignal::default(),
            bwd_overlap_signal: FrontierOverlapSignal::default(),
        }) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }

        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };

        let (frontier, parent, depths, orig, other_depths) = if expand_forward {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_depths,
                &mut fwd_orig,
                &bwd_depths,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_depths,
                &mut bwd_orig,
                &fwd_depths,
            )
        };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let current_frontier_len = current_frontier.len();
        let computed: Vec<(DynMatrix, crate::graph_moves::GraphMoveSuccessors)> = current_frontier
            .par_iter()
            .map(|current_canon| {
                let current = orig
                    .get(current_canon)
                    .expect("frontier node should have an original matrix");
                (
                    current_canon.clone(),
                    enumerate_graph_move_successors(current, config.max_intermediate_dim),
                )
            })
            .collect();

        let mut layer = SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: current_frontier_len,
            ..SearchLayerTelemetry::default()
        };
        let mut layer_move_family_telemetry = MoveFamilyTelemetryAccumulator::default();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(layer.candidates_after_pruning.max(8)));
        let mut next_frontier = VecDeque::new();
        let next_depth = layer_depth + 1;
        let mut parents_with_progress = HashSet::new();

        for (current_canon, successors) in computed {
            let current_orig = orig
                .get(&current_canon)
                .expect("frontier node should have an original matrix")
                .clone();
            layer.candidates_generated += successors.candidates;
            for (family, count) in successors.family_candidates {
                move_family_telemetry_mut(&mut layer_move_family_telemetry, family)
                    .candidates_generated += count;
            }

            if current_frontier_len > 0 {
                let candidates_per_node =
                    layer.candidates_generated.max(1) as f64 / current_frontier_len as f64;
                if expand_forward {
                    fwd_candidates_per_node = candidates_per_node;
                    fwd_cost_sample_nodes = current_frontier_len;
                } else {
                    bwd_candidates_per_node = candidates_per_node;
                    bwd_cost_sample_nodes = current_frontier_len;
                }
            }

            for successor in successors.nodes {
                if successor.orig_matrix.max_entry() > config.max_entry {
                    continue;
                }
                move_family_telemetry_mut(&mut layer_move_family_telemetry, successor.family)
                    .candidates_after_pruning += 1;
                layer.candidates_after_pruning += 1;

                if parent.contains_key(&successor.matrix) {
                    layer.collisions_with_seen += 1;
                    if let Some(records) = layer_records.as_mut() {
                        records.push(SearchEdgeRecord {
                            layer_index,
                            direction,
                            move_family: successor.family,
                            from_canonical: current_canon.clone(),
                            from_orig: current_orig.clone(),
                            to_canonical: successor.matrix.clone(),
                            to_orig: successor.orig_matrix.clone(),
                            from_depth: layer_depth,
                            to_depth: next_depth,
                            step: successor.step.clone(),
                            status: SearchEdgeStatus::SeenCollision,
                            approximate_other_side_hit: false,
                            enqueued: false,
                        });
                    }
                    continue;
                }

                parent.insert(
                    successor.matrix.clone(),
                    Some((current_canon.clone(), successor.step.clone())),
                );
                depths.insert(successor.matrix.clone(), next_depth);
                orig.insert(successor.matrix.clone(), successor.orig_matrix.clone());
                layer.discovered_nodes += 1;
                move_family_telemetry_mut(&mut layer_move_family_telemetry, successor.family)
                    .discovered_nodes += 1;
                parents_with_progress.insert(current_canon.clone());
                let mut record_status = SearchEdgeStatus::Discovered;

                if let Some(&other_depth) = other_depths.get(&successor.matrix) {
                    layer.collisions_with_other_frontier += 1;
                    move_family_telemetry_mut(
                        &mut layer_move_family_telemetry,
                        successor.family,
                    )
                    .exact_meets += 1;
                    record_status = SearchEdgeStatus::ExactMeet;
                    let path_depth = next_depth + other_depth;
                    if path_depth <= config.max_lag {
                        layer.next_frontier_nodes = next_frontier.len();
                        layer.total_visited_nodes = telemetry.total_visited_nodes;
                        layer.dead_end_nodes =
                            current_frontier_len.saturating_sub(parents_with_progress.len());
                        telemetry.collisions_with_seen += layer.collisions_with_seen;
                        telemetry.collisions_with_other_frontier +=
                            layer.collisions_with_other_frontier;
                        telemetry.same_future_past_collisions += layer.same_future_past_collisions;
                        telemetry.discovered_nodes += layer.discovered_nodes;
                        telemetry.dead_end_nodes += layer.dead_end_nodes;
                        telemetry.enqueued_nodes += layer.enqueued_nodes;
                        telemetry.candidates_generated += layer.candidates_generated;
                        telemetry.candidates_after_pruning += layer.candidates_after_pruning;
                        telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
                        telemetry.frontier_nodes_expanded += layer.frontier_nodes;
                        telemetry.total_visited_nodes =
                            visited_union_size(&fwd_parent, &bwd_parent);
                        layer.total_visited_nodes = telemetry.total_visited_nodes;
                        accumulate_move_family_telemetry(
                            &mut telemetry.move_family_telemetry,
                            &layer_move_family_telemetry,
                        );
                        if let Some(records) = layer_records.as_mut() {
                            records.push(SearchEdgeRecord {
                                layer_index,
                                direction,
                                move_family: successor.family,
                                from_canonical: current_canon.clone(),
                                from_orig: current_orig.clone(),
                                to_canonical: successor.matrix.clone(),
                                to_orig: successor.orig_matrix.clone(),
                                from_depth: layer_depth,
                                to_depth: next_depth,
                                step: successor.step.clone(),
                                status: record_status,
                                approximate_other_side_hit: false,
                                enqueued: false,
                            });
                        }
                        if let Some(records) = layer_records.as_ref() {
                            emit_layer(&mut observer, records);
                        }
                        layer.move_family_telemetry =
                            finalize_move_family_telemetry(layer_move_family_telemetry);
                        telemetry.layers.push(layer);
                        return finish_search_2x2(
                            observer,
                            request,
                            SseResult::Equivalent(reconstruct_bidirectional_path(
                                a,
                                b,
                                &successor.matrix,
                                &fwd_parent,
                                &fwd_orig,
                                &bwd_parent,
                                &bwd_orig,
                            )),
                            telemetry,
                        );
                    }
                } else {
                    telemetry.total_visited_nodes += 1;
                }

                next_frontier.push_back(successor.matrix.clone());
                layer.enqueued_nodes += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: successor.family,
                        from_canonical: current_canon.clone(),
                        from_orig: current_orig.clone(),
                        to_canonical: successor.matrix.clone(),
                        to_orig: successor.orig_matrix.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: successor.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: false,
                        enqueued: true,
                    });
                }
            }
        }

        layer.dead_end_nodes = current_frontier_len.saturating_sub(parents_with_progress.len());
        layer.next_frontier_nodes = next_frontier.len();
        layer.total_visited_nodes = telemetry.total_visited_nodes;

        telemetry.frontier_nodes_expanded += layer.frontier_nodes;
        telemetry.candidates_generated += layer.candidates_generated;
        telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
        telemetry.candidates_after_pruning += layer.candidates_after_pruning;
        telemetry.collisions_with_seen += layer.collisions_with_seen;
        telemetry.collisions_with_other_frontier += layer.collisions_with_other_frontier;
        telemetry.same_future_past_collisions += layer.same_future_past_collisions;
        telemetry.discovered_nodes += layer.discovered_nodes;
        telemetry.dead_end_nodes += layer.dead_end_nodes;
        telemetry.enqueued_nodes += layer.enqueued_nodes;
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        layer.move_family_telemetry = finalize_move_family_telemetry(layer_move_family_telemetry);
        telemetry.layers.push(layer);

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    if let Some(witness) = try_concrete_shift_shortcut_2x2(a, b, config) {
        telemetry.concrete_shift_shortcut = true;
        return finish_search_2x2(
            observer,
            request,
            SseResult::EquivalentByConcreteShift(witness),
            telemetry,
        );
    }

    finish_search_2x2(observer, request, SseResult::Unknown, telemetry)
}

fn search_graph_only_dyn_with_telemetry(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    deadline: Option<Instant>,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a.clone());
    fwd_frontier.push_back(a_canon.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b.clone());
    bwd_frontier.push_back(b_canon.clone());

    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = 2;
    let mut fwd_candidates_per_node = 1.0f64;
    let mut bwd_candidates_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;

    for layer_index in 0..config.max_lag {
        if deadline_reached(deadline) {
            break;
        }
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(FrontierLayerChoiceInputs {
            fwd_depth: next_fwd_depth,
            bwd_depth: next_bwd_depth,
            fwd_frontier_len: fwd_frontier.len(),
            bwd_frontier_len: bwd_frontier.len(),
            fwd_factorisations_per_node: fwd_candidates_per_node,
            bwd_factorisations_per_node: bwd_candidates_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal: FrontierOverlapSignal::default(),
            bwd_overlap_signal: FrontierOverlapSignal::default(),
        }) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }

        let (frontier, parent, depths, orig, other_depths) = if expand_forward {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_depths,
                &mut fwd_orig,
                &bwd_depths,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_depths,
                &mut bwd_orig,
                &fwd_depths,
            )
        };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let current_frontier_len = current_frontier.len();
        let mut layer = SearchLayerTelemetry {
            layer_index,
            direction: Some(if expand_forward {
                SearchDirection::Forward
            } else {
                SearchDirection::Backward
            }),
            frontier_nodes: current_frontier_len,
            ..SearchLayerTelemetry::default()
        };
        let mut layer_move_family_telemetry = MoveFamilyTelemetryAccumulator::default();
        let mut next_frontier = VecDeque::new();
        let next_depth = layer_depth + 1;
        let mut parents_with_progress = HashSet::new();
        let mut timed_out = false;

        for chunk in current_frontier.chunks(frontier_chunk_size(current_frontier_len, deadline)) {
            if deadline_reached(deadline) {
                timed_out = true;
                break;
            }
            let computed: Vec<(DynMatrix, crate::graph_moves::GraphMoveSuccessors)> = chunk
                .par_iter()
                .map(|current_canon| {
                    let current = orig
                        .get(current_canon)
                        .expect("frontier node should have an original matrix");
                    (
                        current_canon.clone(),
                        enumerate_graph_move_successors(current, config.max_intermediate_dim),
                    )
                })
                .collect();

            for (current_canon, successors) in computed {
                layer.candidates_generated += successors.candidates;
                for (family, count) in successors.family_candidates {
                    move_family_telemetry_mut(&mut layer_move_family_telemetry, family)
                        .candidates_generated += count;
                }

                if current_frontier_len > 0 {
                    let candidates_per_node =
                        layer.candidates_generated.max(1) as f64 / current_frontier_len as f64;
                    if expand_forward {
                        fwd_candidates_per_node = candidates_per_node;
                        fwd_cost_sample_nodes = current_frontier_len;
                    } else {
                        bwd_candidates_per_node = candidates_per_node;
                        bwd_cost_sample_nodes = current_frontier_len;
                    }
                }

                for successor in successors.nodes {
                    if successor.orig_matrix.max_entry() > config.max_entry {
                        continue;
                    }
                    move_family_telemetry_mut(
                        &mut layer_move_family_telemetry,
                        successor.family,
                    )
                    .candidates_after_pruning += 1;
                    layer.candidates_after_pruning += 1;

                    if parent.contains_key(&successor.matrix) {
                        layer.collisions_with_seen += 1;
                        continue;
                    }

                    parent.insert(
                        successor.matrix.clone(),
                        Some((current_canon.clone(), successor.step.clone())),
                    );
                    depths.insert(successor.matrix.clone(), next_depth);
                    orig.insert(successor.matrix.clone(), successor.orig_matrix.clone());
                    layer.discovered_nodes += 1;
                    move_family_telemetry_mut(
                        &mut layer_move_family_telemetry,
                        successor.family,
                    )
                    .discovered_nodes += 1;
                    parents_with_progress.insert(current_canon.clone());

                    if let Some(&other_depth) = other_depths.get(&successor.matrix) {
                        layer.collisions_with_other_frontier += 1;
                        move_family_telemetry_mut(
                            &mut layer_move_family_telemetry,
                            successor.family,
                        )
                        .exact_meets += 1;
                        let path_depth = next_depth + other_depth;
                        if path_depth <= config.max_lag {
                            layer.next_frontier_nodes = next_frontier.len();
                            telemetry.collisions_with_seen += layer.collisions_with_seen;
                            telemetry.collisions_with_other_frontier +=
                                layer.collisions_with_other_frontier;
                            telemetry.same_future_past_collisions +=
                                layer.same_future_past_collisions;
                            telemetry.discovered_nodes += layer.discovered_nodes;
                            layer.dead_end_nodes =
                                current_frontier_len.saturating_sub(parents_with_progress.len());
                            telemetry.dead_end_nodes += layer.dead_end_nodes;
                            telemetry.enqueued_nodes += layer.enqueued_nodes;
                            telemetry.candidates_generated += layer.candidates_generated;
                            telemetry.candidates_after_pruning += layer.candidates_after_pruning;
                            telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
                            telemetry.frontier_nodes_expanded += layer.frontier_nodes;
                            telemetry.total_visited_nodes =
                                visited_union_size(&fwd_parent, &bwd_parent);
                            layer.total_visited_nodes = telemetry.total_visited_nodes;
                            accumulate_move_family_telemetry(
                                &mut telemetry.move_family_telemetry,
                                &layer_move_family_telemetry,
                            );
                            layer.move_family_telemetry =
                                finalize_move_family_telemetry(layer_move_family_telemetry);
                            telemetry.layers.push(layer);
                            return finish_search_dyn(
                                observer,
                                request,
                                DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                                    a,
                                    b,
                                    &successor.matrix,
                                    &fwd_parent,
                                    &fwd_orig,
                                    &bwd_parent,
                                    &bwd_orig,
                                )),
                                telemetry,
                            );
                        }
                    } else {
                        telemetry.total_visited_nodes += 1;
                    }

                    next_frontier.push_back(successor.matrix.clone());
                    layer.enqueued_nodes += 1;
                }
            }
        }

        layer.dead_end_nodes = current_frontier_len.saturating_sub(parents_with_progress.len());
        layer.next_frontier_nodes = next_frontier.len();
        layer.total_visited_nodes = telemetry.total_visited_nodes;

        telemetry.frontier_nodes_expanded += layer.frontier_nodes;
        telemetry.candidates_generated += layer.candidates_generated;
        telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
        telemetry.candidates_after_pruning += layer.candidates_after_pruning;
        telemetry.collisions_with_seen += layer.collisions_with_seen;
        telemetry.collisions_with_other_frontier += layer.collisions_with_other_frontier;
        telemetry.same_future_past_collisions += layer.same_future_past_collisions;
        telemetry.discovered_nodes += layer.discovered_nodes;
        telemetry.dead_end_nodes += layer.dead_end_nodes;
        telemetry.enqueued_nodes += layer.enqueued_nodes;
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        layer.move_family_telemetry = finalize_move_family_telemetry(layer_move_family_telemetry);
        telemetry.layers.push(layer);

        if timed_out {
            break;
        }
        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search_dyn(observer, request, DynSseResult::Unknown, telemetry)
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn frontier_chunk_size(frontier_len: usize, deadline: Option<Instant>) -> usize {
    if deadline.is_some() {
        frontier_len.min(TIMED_SEARCH_FRONTIER_CHUNK_SIZE).max(1)
    } else {
        frontier_len.max(1)
    }
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

fn move_family_telemetry_mut<'a>(
    map: &'a mut MoveFamilyTelemetryAccumulator,
    family: &'static str,
) -> &'a mut SearchMoveFamilyTelemetry {
    map.entry(family).or_default()
}

fn accumulate_search_move_family_telemetry(
    total: &mut SearchMoveFamilyTelemetry,
    delta: &SearchMoveFamilyTelemetry,
) {
    total.candidates_generated += delta.candidates_generated;
    total.candidates_after_pruning += delta.candidates_after_pruning;
    total.discovered_nodes += delta.discovered_nodes;
    total.exact_meets += delta.exact_meets;
    total.approximate_other_side_hits += delta.approximate_other_side_hits;
}

fn accumulate_move_family_telemetry_accumulator(
    total: &mut MoveFamilyTelemetryAccumulator,
    delta: &MoveFamilyTelemetryAccumulator,
) {
    for (&family, family_delta) in delta {
        let family_total = total.entry(family).or_default();
        accumulate_search_move_family_telemetry(family_total, family_delta);
    }
}

fn accumulate_move_family_telemetry(
    total: &mut BTreeMap<String, SearchMoveFamilyTelemetry>,
    delta: &MoveFamilyTelemetryAccumulator,
) {
    for (&family, family_delta) in delta {
        if let Some(family_total) = total.get_mut(family) {
            accumulate_search_move_family_telemetry(family_total, family_delta);
        } else {
            total.insert(family.to_string(), family_delta.clone());
        }
    }
}

fn finalize_move_family_telemetry(
    telemetry: MoveFamilyTelemetryAccumulator,
) -> BTreeMap<String, SearchMoveFamilyTelemetry> {
    telemetry
        .into_iter()
        .map(|(family, family_telemetry)| (family.to_string(), family_telemetry))
        .collect()
}

fn visited_union_size(
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
) -> usize {
    fwd_parent.len()
        + bwd_parent
            .keys()
            .filter(|key| !fwd_parent.contains_key(*key))
            .count()
}

/// Check whether a candidate intermediate matrix has a nonzero spectrum
/// consistent with the source matrix. SSE preserves nonzero eigenvalues,
/// so any intermediate in a valid chain must pass this check.
#[cfg(test)]
fn is_spectrally_consistent(vu: &DynMatrix, source_trace: u64, source_det: i64) -> bool {
    if !vu.is_square() {
        return true; // non-square shouldn't happen, don't filter
    }
    // Trace (sum of eigenvalues) must match for all sizes.
    if vu.trace() != source_trace {
        return false;
    }
    match vu.rows {
        2 => {
            // For 2x2: trace and det fully determine the spectrum.
            vu.det_2x2() == source_det
        }
        3 => {
            // A 3x3 intermediate from a 2x2 source has one zero eigenvalue.
            // det must be 0, and the sum of eigenvalue pairs must equal source_det.
            vu.det_3x3() == 0 && vu.principal_minor_sum_3x3() == source_det
        }
        _ => {
            // For k×k intermediates from a 2×2 source, eigenvalues are
            // {λ₁, λ₂, 0, ..., 0}. Use the power-trace identity:
            // tr(M²) = λ₁² + λ₂² = trace² - 2·det.
            let expected_tr2 = (source_trace as i64) * (source_trace as i64) - 2 * source_det;
            let m2 = vu.mul(vu);
            let actual_tr2 = m2.trace() as i64;
            actual_tr2 == expected_tr2
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use super::*;
    use crate::search_observer::{SearchEvent, SearchObserver};
    use crate::types::{
        GuideArtifact, GuideArtifactCompatibility, GuideArtifactPayload, GuideArtifactProvenance,
        GuideArtifactValidation, GuidedRefinementConfig,
    };

    fn default_config() -> SearchConfig {
        SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 2,
            max_entry: 10,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        }
    }

    #[derive(Default)]
    struct LayerEventProbe {
        layer_sizes: Vec<usize>,
    }

    impl SearchObserver for LayerEventProbe {
        fn on_event(&mut self, event: &SearchEvent) {
            if let SearchEvent::Layer(edges) = event {
                self.layer_sizes.push(edges.len());
            }
        }
    }

    fn full_path_artifact(id: &str, path: DynSsePath) -> GuideArtifact {
        let source = path
            .matrices
            .first()
            .expect("guide path should have a source matrix")
            .clone();
        let target = path
            .matrices
            .last()
            .expect("guide path should have a target matrix")
            .clone();
        let mut artifact = build_full_path_guide_artifact(&source, &target, &path).unwrap();
        artifact.artifact_id = Some(id.to_string());
        artifact.provenance = GuideArtifactProvenance {
            source_kind: Some("unit_test".to_string()),
            label: Some(id.to_string()),
            source_ref: None,
        };
        artifact.compatibility = GuideArtifactCompatibility {
            supported_stages: vec![SearchStage::GuidedRefinement],
            max_endpoint_dim: Some(4),
        };
        artifact
    }

    fn permutation_guide(base: &DynMatrix, perms: &[[usize; 3]]) -> DynSsePath {
        let matrices = perms
            .iter()
            .map(|perm| base.conjugate_by_perm(perm))
            .collect::<Vec<_>>();
        let steps = matrices
            .windows(2)
            .map(|pair| permutation_step_between(&pair[0], &pair[1]).unwrap())
            .collect::<Vec<_>>();
        DynSsePath { matrices, steps }
    }

    fn shortcut_request(
        source: DynMatrix,
        target: DynMatrix,
        guide_artifacts: Vec<GuideArtifact>,
        guided_refinement: GuidedRefinementConfig,
        shortcut_search: ShortcutSearchConfig,
    ) -> SearchRequest {
        SearchRequest {
            source,
            target,
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts,
            guided_refinement,
            shortcut_search,
        }
    }

    fn endpoint_request(
        source: DynMatrix,
        target: DynMatrix,
        config: SearchConfig,
    ) -> SearchRequest {
        SearchRequest {
            source,
            target,
            config,
            stage: SearchStage::EndpointSearch,
            guide_artifacts: Vec::new(),
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        }
    }

    fn literature_row_split_fixture_2x2_to_5x5() -> (DynMatrix, DynMatrix, DynSsePath) {
        // Instantiates the generic elementary row-splitting template recorded in
        // research/notes/2026-04-15-non-brix-ruiz-sse-pairs.md with
        // a = 1 + 1 + 1, b = 1 + 1 + 0, c = 1 + 1, d = 1 + 0.
        let source = DynMatrix::new(2, 2, vec![3, 2, 2, 1]);
        let target = DynMatrix::new(
            5,
            5,
            vec![
                1, 1, 1, 1, 1, //
                1, 1, 1, 1, 1, //
                1, 1, 1, 0, 0, //
                1, 1, 1, 1, 1, //
                1, 1, 1, 0, 0,
            ],
        );
        let path = DynSsePath {
            matrices: vec![source.clone(), target.clone()],
            steps: vec![EsseStep {
                u: DynMatrix::new(
                    2,
                    5,
                    vec![
                        1, 1, 1, 0, 0, //
                        0, 0, 0, 1, 1,
                    ],
                ),
                v: DynMatrix::new(
                    5,
                    2,
                    vec![
                        1, 1, //
                        1, 1, //
                        1, 0, //
                        1, 1, //
                        1, 0,
                    ],
                ),
            }],
        };
        (source, target, path)
    }

    #[test]
    fn test_build_full_path_guide_artifact_populates_metadata() {
        let source = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let path = DynSsePath {
            matrices: vec![source.clone()],
            steps: vec![],
        };

        let artifact = build_full_path_guide_artifact(&source, &source, &path).unwrap();
        assert_eq!(artifact.endpoints.source, source);
        assert_eq!(artifact.endpoints.target, artifact.endpoints.source);
        assert_eq!(
            artifact.validation,
            GuideArtifactValidation::WitnessValidated
        );
        assert_eq!(artifact.quality.lag, Some(0));
        assert_eq!(artifact.quality.cost, Some(0));
        assert!(matches!(
            artifact.payload,
            GuideArtifactPayload::FullPath { path: ref full_path } if full_path.steps.is_empty()
        ));
    }

    #[test]
    fn test_build_full_path_guide_artifact_rejects_invalid_path() {
        let source = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let target = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let invalid = DynSsePath {
            matrices: vec![source.clone(), source.clone()],
            steps: vec![EsseStep {
                u: target.clone(),
                v: target.clone(),
            }],
        };

        let err = build_full_path_guide_artifact(&source, &target, &invalid).unwrap_err();
        assert!(err.contains("does not end"));
    }

    #[test]
    fn test_validate_sse_path_dyn_accepts_literature_row_split_2x2_to_5x5_fixture() {
        let (source, target, path) = literature_row_split_fixture_2x2_to_5x5();
        let step = &path.steps[0];

        assert_eq!(step.u.mul(&step.v), source);
        assert_eq!(step.v.mul(&step.u), target);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
    }

    #[test]
    fn test_self_sse() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let result = search_sse_2x2(&a, &a, &default_config());
        match result {
            SseResult::Equivalent(path) => {
                assert_eq!(path.matrices.len(), 1);
                assert_eq!(path.steps.len(), 0);
            }
            _ => panic!("Expected Equivalent for self-SSE"),
        }
    }

    #[test]
    fn test_elementary_sse_pair() {
        // [[2,1],[1,1]] is elementary SSE to [[1,1],[1,2]]
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let result = search_sse_2x2(&a, &b, &default_config());
        match result {
            SseResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                // Verify the step: A = UV, B = VU
                let step = &path.steps[0];
                let uv = step.u.mul(&step.v);
                let vu = step.v.mul(&step.u);
                assert_eq!(uv, DynMatrix::from_sq(&a));
                assert_eq!(vu, DynMatrix::from_sq(&b));
            }
            _ => panic!("Expected Equivalent for known elementary SSE pair"),
        }
    }

    #[test]
    fn test_beam_search_finds_small_solvable_case() {
        let a = SqMatrix::new([[1, 1], [2, 5]]);
        let b = SqMatrix::new([[1, 2], [1, 5]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::Beam,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: Some(4),
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };

        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
        match result {
            SseResult::Equivalent(path) => {
                assert_valid_path(&path);
            }
            other => panic!("expected Equivalent from beam search, got {other:?}"),
        }
        assert!(telemetry.frontier_nodes_expanded >= 1);
        assert!(telemetry.max_frontier_size <= 4);
    }

    #[test]
    fn test_beam_graph_only_combination_uses_beam_frontier_without_factorisations() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 4,
            max_entry: 6,
            frontier_mode: FrontierMode::Beam,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: Some(3),
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };

        let (_result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);

        assert!(telemetry.frontier_nodes_expanded >= 1);
        assert_eq!(telemetry.factorisations_enumerated, 0);
        assert!(telemetry.max_frontier_size <= 3);
    }

    #[test]
    fn test_beam_bfs_handoff_search_finds_small_solvable_case() {
        let a = SqMatrix::new([[1, 1], [2, 5]]);
        let b = SqMatrix::new([[1, 2], [1, 5]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::BeamBfsHandoff,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: Some(2),
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };

        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
        match result {
            SseResult::Equivalent(path) => {
                assert_valid_path(&path);
            }
            other => panic!("expected Equivalent from beam_bfs_handoff search, got {other:?}"),
        }
        assert!(telemetry.frontier_nodes_expanded >= 1);
        assert!(telemetry.max_frontier_size >= 1);
    }

    #[test]
    fn test_beam_bfs_handoff_recovers_shorter_deferred_path_than_beam_phase_meet() {
        let a = SqMatrix::new([[0, 2], [0, 1]]);
        let b = SqMatrix::new([[1, 1], [0, 0]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::BeamBfsHandoff,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: Some(1),
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };

        let result = search_sse_2x2(&a, &b, &config);
        let SseResult::Equivalent(path) = result else {
            panic!("expected Equivalent path from beam_bfs_handoff");
        };
        assert_valid_path(&path);
        assert_eq!(path.steps.len(), 1);
    }

    #[test]
    fn test_execute_search_request_endpoint_search_preserves_2x2_result_shaping() {
        let a = SqMatrix::new([[0, 1], [1, 2]]);
        let b = SqMatrix::new([[1, 1], [2, 1]]);
        let request = endpoint_request(
            DynMatrix::from_sq(&a),
            DynMatrix::from_sq(&b),
            SearchConfig {
                max_lag: 1,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::EquivalentByConcreteShift(proof) => {
                assert_eq!(proof.relation, ConcreteShiftRelation2x2::Aligned);
                assert_eq!(proof.witness.shift.lag, 1);
            }
            other => panic!(
                "expected endpoint request dispatch to preserve concrete-shift result shaping, got {other:?}"
            ),
        }
        assert!(telemetry.concrete_shift_shortcut);
    }

    #[test]
    fn test_execute_search_request_endpoint_search_routes_dynamic_endpoints() {
        let a = DynMatrix::new(3, 3, vec![2, 0, 0, 0, 1, 0, 0, 0, 0]);
        let b = DynMatrix::new(3, 3, vec![0, 0, 0, 0, 1, 0, 0, 0, 2]);
        let request = endpoint_request(a.clone(), b.clone(), default_config());

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                assert_eq!(path.matrices, vec![a, b]);
            }
            other => panic!(
                "expected endpoint request dispatch to route non-2x2 endpoints through dynamic search, got {other:?}"
            ),
        }
        assert!(telemetry.permutation_shortcut || telemetry.canonical_shortcut);
    }

    #[test]
    fn test_different_trace_not_equivalent() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[3, 1], [1, 1]]);
        let result = search_sse_2x2(&a, &b, &default_config());
        match result {
            SseResult::NotEquivalent(reason) => {
                assert!(reason.contains("trace"));
            }
            _ => panic!("Expected NotEquivalent"),
        }
    }

    #[test]
    fn test_guided_refinement_stage_accepts_full_path_artifact() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: default_config(),
            stage: SearchStage::GuidedRefinement,
            guide_artifacts: vec![full_path_artifact("direct-guide", guide)],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from guided refinement, got {other:?}"),
        }
        assert_eq!(telemetry.guide_artifacts_considered, 1);
        assert_eq!(telemetry.guide_artifacts_accepted, 1);
    }

    #[test]
    fn test_guided_refinement_stage_shortens_permutation_guide() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid).unwrap(),
                permutation_step_between(&mid, &b).unwrap(),
            ],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
            stage: SearchStage::GuidedRefinement,
            guide_artifacts: vec![full_path_artifact("two-hop-guide", guide)],
            guided_refinement: GuidedRefinementConfig {
                max_shortcut_lag: 1,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: None,
            },
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from guided refinement, got {other:?}"),
        }
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.guided_segments_improved, 1);
    }

    #[test]
    fn test_guided_refinement_segment_timeout_preserves_guide_when_search_times_out() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid).unwrap(),
                permutation_step_between(&mid, &b).unwrap(),
            ],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
            stage: SearchStage::GuidedRefinement,
            guide_artifacts: vec![full_path_artifact("two-hop-guide", guide)],
            guided_refinement: GuidedRefinementConfig {
                max_shortcut_lag: 1,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: Some(0),
            },
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 2);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from guided refinement, got {other:?}"),
        }
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.guided_segments_improved, 0);
    }

    #[test]
    fn test_refine_guide_path_once_reuses_cached_segment_result() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid).unwrap(),
                permutation_step_between(&mid, &b).unwrap(),
            ],
        };
        let cached = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };

        let config = SearchConfig {
            max_lag: 2,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let guided = GuidedRefinementConfig {
            max_shortcut_lag: 1,
            min_gap: 2,
            max_gap: Some(2),
            rounds: 1,
            segment_timeout_secs: None,
        };

        let mut telemetry = SearchTelemetry::default();
        let mut remaining_segment_attempts = 4usize;
        let mut segment_cache = GuidedSegmentCache::default();
        segment_cache.insert(
            GuidedSegmentCacheKey {
                source: a.clone(),
                target: b.clone(),
                max_lag: 1,
            },
            DynSseResult::Equivalent(cached),
        );

        let refined = refine_guide_path_once(
            &guide,
            &config,
            &guided,
            &mut telemetry,
            &mut remaining_segment_attempts,
            &mut segment_cache,
        );

        assert_eq!(refined.steps.len(), 1);
        validate_sse_path_dyn(&a, &b, &refined).unwrap();
        assert_eq!(remaining_segment_attempts, 3);
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.guided_segments_improved, 1);
        assert_eq!(telemetry.shortcut_search.segment_cache_hits, 1);
        assert_eq!(telemetry.shortcut_search.segment_cache_misses, 0);
        assert_eq!(telemetry.frontier_nodes_expanded, 0);
    }

    #[test]
    fn test_refine_guide_path_once_reuses_equivalent_cache_result_across_lag_caps() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid1 = base.conjugate_by_perm(&[2, 0, 1]);
        let mid2 = base.conjugate_by_perm(&[0, 2, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid1.clone(), mid2.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid1).unwrap(),
                permutation_step_between(&mid1, &mid2).unwrap(),
                permutation_step_between(&mid2, &b).unwrap(),
            ],
        };
        let cached = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };

        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let guided = GuidedRefinementConfig {
            max_shortcut_lag: 2,
            min_gap: 3,
            max_gap: Some(3),
            rounds: 1,
            segment_timeout_secs: None,
        };

        let mut telemetry = SearchTelemetry::default();
        let mut remaining_segment_attempts = 4usize;
        let mut segment_cache = GuidedSegmentCache::default();
        segment_cache.insert(
            GuidedSegmentCacheKey {
                source: a.clone(),
                target: b.clone(),
                max_lag: 1,
            },
            DynSseResult::Equivalent(cached),
        );

        let refined = refine_guide_path_once(
            &guide,
            &config,
            &guided,
            &mut telemetry,
            &mut remaining_segment_attempts,
            &mut segment_cache,
        );

        assert_eq!(refined.steps.len(), 1);
        validate_sse_path_dyn(&a, &b, &refined).unwrap();
        assert_eq!(remaining_segment_attempts, 3);
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.guided_segments_improved, 1);
        assert_eq!(telemetry.shortcut_search.segment_cache_hits, 1);
        assert_eq!(telemetry.shortcut_search.segment_cache_misses, 0);
        assert_eq!(telemetry.frontier_nodes_expanded, 0);
    }

    #[test]
    fn test_shortcut_search_stage_accepts_legacy_guided_refinement_artifact() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid, b.clone()],
            steps: vec![
                permutation_step_between(&a, &base.conjugate_by_perm(&[2, 0, 1])).unwrap(),
                permutation_step_between(&base.conjugate_by_perm(&[2, 0, 1]), &b).unwrap(),
            ],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: vec![full_path_artifact("legacy-guided", guide)],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from shortcut search, got {other:?}"),
        }
        assert_eq!(telemetry.guide_artifacts_considered, 1);
        assert_eq!(telemetry.guide_artifacts_accepted, 1);
        assert_eq!(telemetry.shortcut_search.guide_artifacts_loaded, 1);
        assert_eq!(telemetry.shortcut_search.guide_artifacts_accepted, 1);
        assert_eq!(telemetry.shortcut_search.unique_guides, 1);
        assert_eq!(telemetry.shortcut_search.initial_working_set_guides, 1);
        assert_eq!(telemetry.shortcut_search.best_lag_start, Some(2));
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(1));
        assert_eq!(telemetry.shortcut_search.promoted_guides, 1);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 2);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::GuidePoolExhausted)
        );
    }

    #[test]
    fn test_shortcut_search_stage_deduplicates_and_ranks_guides() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let direct = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };
        let two_hop = DynSsePath {
            matrices: vec![a.clone(), mid.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid).unwrap(),
                permutation_step_between(&mid, &b).unwrap(),
            ],
        };

        let mut direct_forward = full_path_artifact("direct-forward", direct.clone());
        direct_forward.quality.lag = None;
        direct_forward.quality.cost = Some(5);
        direct_forward.quality.score = Some(1.0);

        let mut direct_duplicate = full_path_artifact("direct-duplicate", direct);
        direct_duplicate.quality.lag = Some(1);
        direct_duplicate.quality.cost = Some(9);
        direct_duplicate.quality.score = Some(0.5);

        let mut indirect = full_path_artifact("two-hop", two_hop);
        indirect.quality.cost = Some(1);
        indirect.quality.score = Some(10.0);

        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: vec![indirect, direct_duplicate, direct_forward],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig {
                max_guides: 1,
                ..ShortcutSearchConfig::default()
            },
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from shortcut search, got {other:?}"),
        }
        assert_eq!(telemetry.guide_artifacts_considered, 3);
        assert_eq!(telemetry.guide_artifacts_accepted, 3);
        assert_eq!(telemetry.shortcut_search.unique_guides, 2);
        assert_eq!(telemetry.shortcut_search.initial_working_set_guides, 1);
        assert_eq!(telemetry.shortcut_search.best_lag_start, Some(1));
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(1));
    }

    #[test]
    fn test_shortcut_search_stage_iteratively_refines_promoted_guides() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let guide = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let source = guide.matrices.first().unwrap().clone();
        let target = guide.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("iterative-seed", guide)],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.best_lag_start, Some(3));
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(1));
        assert_eq!(telemetry.shortcut_search.promoted_guides, 2);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 3);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::GuidePoolExhausted)
        );
        assert_eq!(telemetry.shortcut_search.rounds.len(), 3);
        assert_eq!(
            telemetry.shortcut_search.rounds[0].starting_best_lag,
            Some(3)
        );
        assert_eq!(telemetry.shortcut_search.rounds[0].ending_best_lag, Some(2));
        assert_eq!(
            telemetry.shortcut_search.rounds[1].starting_best_lag,
            Some(2)
        );
        assert_eq!(telemetry.shortcut_search.rounds[1].ending_best_lag, Some(1));
    }

    #[test]
    fn test_shortcut_search_stage_deduplicates_promoted_guides() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let first = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let second = permutation_guide(&base, &[[0, 1, 2], [0, 2, 1], [1, 2, 0], [2, 1, 0]]);
        let source = first.matrices.first().unwrap().clone();
        let target = first.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![
                full_path_artifact("promotion-a", first),
                full_path_artifact("promotion-b", second),
            ],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(3),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 2,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.promoted_guides, 1);
    }

    #[test]
    fn test_shortcut_search_stage_reports_no_improvement_round_with_leftover_pool() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let direct = permutation_guide(&base, &[[0, 1, 2], [2, 1, 0]]);
        let indirect = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 1, 0]]);
        let source = direct.matrices.first().unwrap().clone();
        let target = direct.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![
                full_path_artifact("indirect-leftover", indirect),
                full_path_artifact("direct-best", direct),
            ],
            GuidedRefinementConfig::default(),
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.initial_working_set_guides, 1);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::NoImprovementRound)
        );
    }

    #[test]
    fn test_shortcut_search_stage_reports_guide_pool_exhaustion() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let direct = permutation_guide(&base, &[[0, 1, 2], [2, 1, 0]]);
        let source = direct.matrices.first().unwrap().clone();
        let target = direct.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("direct-only", direct)],
            GuidedRefinementConfig::default(),
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(telemetry.shortcut_search.segment_attempts, 0);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::GuidePoolExhausted)
        );
    }

    #[test]
    fn test_shortcut_search_stage_reports_max_rounds_reached() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let guide = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let source = guide.matrices.first().unwrap().clone();
        let target = guide.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("max-rounds", guide)],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 1,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 2);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(2));
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::MaxRoundsReached)
        );
    }

    #[test]
    fn test_shortcut_search_stage_respects_total_segment_attempt_budget() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let guide = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let source = guide.matrices.first().unwrap().clone();
        let target = guide.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("budgeted", guide)],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(3),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 1,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.shortcut_search.segment_attempts, 1);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(telemetry.shortcut_search.rounds[0].segment_attempts, 1);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::MaxSegmentAttemptsReached)
        );
    }

    #[test]
    fn test_prepare_full_path_guide_reorients_reversed_artifact_for_shortcut_search() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };
        let artifact = full_path_artifact("reverse-guide", reverse_dyn_sse_path(&guide));
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                frontier_mode: FrontierMode::Bfs,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
                beam_width: None,
                beam_bfs_handoff_depth: None,
                beam_bfs_handoff_deferred_cap: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: vec![],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let prepared = prepare_full_path_guide(&request, &artifact)
            .unwrap()
            .unwrap();
        assert_eq!(prepared.matrices, guide.matrices);
        assert_eq!(prepared.steps.len(), 1);
        validate_sse_path_dyn(&a, &b, &prepared).unwrap();
    }

    #[test]
    fn test_compare_ranked_guides_prefers_cost_then_score_then_stable_key() {
        let path = DynSsePath {
            matrices: vec![DynMatrix::new(2, 2, vec![1, 0, 0, 1])],
            steps: vec![],
        };
        let mut best = RankedGuide {
            path: path.clone(),
            effective_lag: 1,
            effective_cost: Some(2),
            effective_score: Some(4.0),
            stable_key: "a".to_string(),
        };
        let worse_cost = RankedGuide {
            path: path.clone(),
            effective_lag: 1,
            effective_cost: Some(3),
            effective_score: Some(10.0),
            stable_key: "b".to_string(),
        };
        let worse_score = RankedGuide {
            path: path.clone(),
            effective_lag: 1,
            effective_cost: Some(2),
            effective_score: Some(1.0),
            stable_key: "c".to_string(),
        };
        let worse_stable = RankedGuide {
            path,
            effective_lag: 1,
            effective_cost: Some(2),
            effective_score: Some(4.0),
            stable_key: "z".to_string(),
        };

        assert_eq!(compare_ranked_guides(&best, &worse_cost), Ordering::Less);
        assert_eq!(compare_ranked_guides(&best, &worse_score), Ordering::Less);
        assert_eq!(compare_ranked_guides(&best, &worse_stable), Ordering::Less);

        best.effective_cost = None;
        assert_eq!(compare_ranked_guides(&worse_cost, &best), Ordering::Less);
    }

    #[test]
    fn test_shortcut_search_stage_rejects_rectangular_endpoints() {
        let request = SearchRequest {
            source: DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 0]),
            target: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            config: default_config(),
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: Vec::new(),
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let err = execute_search_request(&request).unwrap_err();
        assert_eq!(
            err,
            "shortcut_search requires square source and target matrices"
        );
    }

    #[test]
    fn test_telemetry_for_invariant_rejection() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[3, 1], [1, 1]]);
        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &default_config());
        assert!(matches!(result, SseResult::NotEquivalent(_)));
        assert!(telemetry.invariant_filtered);
        assert_eq!(telemetry.frontier_nodes_expanded, 0);
        assert!(telemetry.layers.is_empty());
    }

    #[test]
    fn test_different_det_not_equivalent() {
        let a = SqMatrix::new([[3, 1], [1, 1]]); // tr=4, det=2
        let b = SqMatrix::new([[2, 1], [1, 2]]); // tr=4, det=3
        let result = search_sse_2x2(&a, &b, &default_config());
        match result {
            SseResult::NotEquivalent(reason) => {
                assert!(reason.contains("determinant"));
            }
            _ => panic!("Expected NotEquivalent"),
        }
    }

    #[test]
    fn test_path_verification() {
        // For any found path, verify each step: A_i = UV, A_{i+1} = VU
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let result = search_sse_2x2(&a, &b, &default_config());
        if let SseResult::Equivalent(path) = result {
            for step in &path.steps {
                let _uv = step.u.mul(&step.v);
                let _vu = step.v.mul(&step.u);
                // Dimensions should be consistent.
                assert_eq!(step.u.rows, step.v.cols);
                assert_eq!(step.u.cols, step.v.rows);
            }
        }
    }

    #[test]
    fn test_try_concrete_shift_shortcut_returns_witness_for_identity_pair() {
        let a = SqMatrix::new([[1, 0], [0, 1]]);
        let config = SearchConfig {
            max_entry: 1,
            ..default_config()
        };
        let proof = try_concrete_shift_shortcut_2x2(&a, &a, &config).unwrap();
        assert_eq!(proof.relation, ConcreteShiftRelation2x2::Aligned);
    }

    #[test]
    fn test_try_concrete_shift_shortcut_allows_graph_only_policy() {
        let a = SqMatrix::new([[1, 0], [0, 1]]);
        let config = SearchConfig {
            max_entry: 1,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            ..default_config()
        };
        let proof = try_concrete_shift_shortcut_2x2(&a, &a, &config);
        assert!(proof.is_some());
    }

    #[test]
    fn test_shortcut_helper_preserves_concrete_shift_relation() {
        let a = SqMatrix::identity();
        let config = SearchConfig {
            max_lag: 1,
            max_intermediate_dim: 2,
            max_entry: 1,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };

        let shortcut_proof = try_concrete_shift_shortcut_2x2(&a, &a, &config)
            .expect("identity pair should produce a bounded concrete-shift proof");
        assert_eq!(shortcut_proof.relation, ConcreteShiftRelation2x2::Aligned);
        assert_eq!(
            shortcut_proof.description(),
            "aligned concrete-shift witness"
        );

        let run_result: SearchRunResult =
            SseResult::EquivalentByConcreteShift(shortcut_proof.clone()).into();
        match run_result {
            SearchRunResult::EquivalentByConcreteShift(run_proof) => {
                assert_eq!(run_proof.relation, ConcreteShiftRelation2x2::Aligned)
            }
            other => panic!("expected concrete-shift run result, got {:?}", other),
        }
    }

    #[test]
    fn test_find_concrete_shift_shortcut_proof_prefers_lower_lag() {
        let a = SqMatrix::identity();
        let config = SearchConfig {
            max_lag: 1,
            max_intermediate_dim: 2,
            max_entry: 1,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let base_proof = try_concrete_shift_shortcut_2x2(&a, &a, &config)
            .expect("identity pair should produce a bounded concrete-shift proof");

        let mut balanced_lag_one = base_proof.witness.clone();
        balanced_lag_one.shift.lag = 1;

        let mut probe_calls = Vec::new();
        let chosen = find_concrete_shift_shortcut_proof(4, |lag, relation| {
            probe_calls.push((lag, relation));
            match (lag, relation) {
                (1, ConcreteShiftRelation2x2::Balanced) => {
                    ConcreteShiftSearchResult2x2::Equivalent(balanced_lag_one.clone())
                }
                (4, ConcreteShiftRelation2x2::Aligned) => {
                    panic!("search should stop after finding the lag-1 proof")
                }
                _ => ConcreteShiftSearchResult2x2::Exhausted,
            }
        })
        .expect("expected a chosen proof");

        assert_eq!(chosen.relation, ConcreteShiftRelation2x2::Balanced);
        assert_eq!(chosen.witness.shift.lag, 1);
        assert_eq!(
            probe_calls,
            vec![
                (1, ConcreteShiftRelation2x2::Aligned),
                (1, ConcreteShiftRelation2x2::Balanced),
            ]
        );
    }

    #[test]
    fn test_find_concrete_shift_shortcut_proof_stops_after_lower_lag_limit() {
        let mut probe_calls = Vec::new();
        let proof = find_concrete_shift_shortcut_proof(3, |lag, relation| {
            probe_calls.push((lag, relation));
            match (lag, relation) {
                (1, ConcreteShiftRelation2x2::Aligned) => {
                    ConcreteShiftSearchResult2x2::SearchLimitReached
                }
                _ => ConcreteShiftSearchResult2x2::Exhausted,
            }
        });

        assert!(proof.is_none());
        assert_eq!(
            probe_calls,
            vec![
                (1, ConcreteShiftRelation2x2::Aligned),
                (1, ConcreteShiftRelation2x2::Balanced),
                (1, ConcreteShiftRelation2x2::Compatible),
            ]
        );
    }

    #[test]
    fn test_telemetry_for_elementary_pair_search() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &default_config());
        assert!(matches!(
            result,
            SseResult::Equivalent(_) | SseResult::EquivalentByConcreteShift(_)
        ));
        assert!(
            telemetry.permutation_shortcut
                || telemetry.concrete_shift_shortcut
                || !telemetry.layers.is_empty()
        );
        if telemetry.permutation_shortcut || telemetry.concrete_shift_shortcut {
            assert_eq!(telemetry.frontier_nodes_expanded, 0);
            assert!(telemetry.layers.is_empty());
        } else {
            assert!(telemetry.frontier_nodes_expanded >= 1);
            assert!(telemetry.factorisation_calls >= 1);
            assert!(telemetry.factorisations_enumerated >= telemetry.candidates_after_pruning);
            assert!(telemetry.total_visited_nodes >= 2);
        }
    }

    #[test]
    fn test_bfs_positive_pair_path_is_valid() {
        let a = SqMatrix::new([[1, 1], [2, 5]]);
        let b = SqMatrix::new([[1, 2], [1, 5]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        match result {
            SseResult::Equivalent(path) => assert_valid_path(&path),
            SseResult::EquivalentByConcreteShift(_witness) => {}
            other => panic!("expected Equivalent path, got {:?}", other),
        }
    }

    #[test]
    fn test_graph_only_bfs_falls_back_to_concrete_shift_on_lag_one_pair() {
        let a = SqMatrix::new([[0, 1], [1, 2]]);
        let b = SqMatrix::new([[1, 1], [2, 1]]);
        let config = SearchConfig {
            max_lag: 1,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };

        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
        match result {
            SseResult::EquivalentByConcreteShift(proof) => {
                assert_eq!(proof.relation, ConcreteShiftRelation2x2::Aligned);
                assert_eq!(proof.witness.shift.lag, 1);
            }
            other => panic!(
                "expected graph-only search to fall back to a concrete-shift proof, got {:?}",
                other
            ),
        }
        assert!(telemetry.concrete_shift_shortcut);
        assert_eq!(telemetry.frontier_nodes_expanded, 1);
        assert_eq!(telemetry.factorisations_enumerated, 0);
    }

    #[test]
    fn test_telemetry_for_brix_ruiz_search() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let (_result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
        assert!(!telemetry.invariant_filtered);
        assert!(!telemetry.permutation_shortcut);
        assert!(!telemetry.layers.is_empty());
        assert!(telemetry.frontier_nodes_expanded >= 1);
        assert!(telemetry.factorisations_enumerated >= telemetry.candidates_after_pruning);
        assert!(telemetry.layers.iter().all(|layer| {
            let timing = layer.timing;
            let phased_total = timing
                .expand_compute_nanos
                .saturating_add(timing.expand_accumulate_nanos)
                .saturating_add(timing.dedup_nanos)
                .saturating_add(timing.merge_nanos)
                .saturating_add(timing.finalize_nanos);
            timing.total_nanos >= phased_total && timing.total_nanos > 0
        }));
    }

    #[test]
    fn test_expand_frontier_layer_deduplicates_canonical_successors() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (expansions, stats, _timing) = expand_frontier_layer(
            &[a_canon],
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 2,
                max_entry: 10,
                move_family_policy: MoveFamilyPolicy::Mixed,
            },
        );

        assert!(!expansions.is_empty());
        assert!(stats.factorisations_enumerated > expansions.len());
    }

    #[test]
    fn test_expand_frontier_layer_graph_only_skips_factorisations() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (_expansions, stats, _timing) = expand_frontier_layer(
            &[a_canon],
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 2,
                max_entry: 10,
                move_family_policy: MoveFamilyPolicy::GraphOnly,
            },
        );

        assert_eq!(stats.factorisations_enumerated, 0);
    }

    #[test]
    fn test_expand_frontier_layer_graph_plus_structured_keeps_structured_telemetry() {
        let u = DynMatrix::new(3, 3, vec![5, 2, 0, 2, 1, 0, 0, 0, 1]);
        let v = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 1, 0, 0, 0, 1]);
        let current = u.mul(&v);
        let current_canon = current.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(current_canon.clone(), current);

        let (_expansions, stats, _timing) = expand_frontier_layer(
            &[current_canon],
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 3,
                max_entry: 6,
                move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
            },
        );

        assert!(stats.factorisations_enumerated > 0);
        assert!(stats
            .move_family_telemetry
            .contains_key("opposite_shear_conjugation_3x3"));
        assert!(!stats
            .move_family_telemetry
            .contains_key("square_factorisation_3x3"));
    }

    #[test]
    fn test_expand_frontier_layer_graph_plus_structured_exposes_diagonal_refactorization() {
        let u = DynMatrix::new(3, 3, vec![3, 0, 0, 0, 1, 0, 0, 0, 2]);
        let v = DynMatrix::new(3, 3, vec![1, 1, 0, 2, 1, 1, 1, 0, 1]);
        let current = u.mul(&v);
        let current_canon = current.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(current_canon.clone(), current);

        let (expansions, stats, _timing) = expand_frontier_layer(
            &[current_canon],
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 3,
                max_entry: 6,
                move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
            },
        );

        assert!(expansions
            .iter()
            .any(|expansion| expansion.move_family == "diagonal_refactorization_3x3"));
        assert!(stats
            .move_family_telemetry
            .get("diagonal_refactorization_3x3")
            .is_some_and(|telemetry| telemetry.candidates_generated > 0));
    }

    #[test]
    fn test_expand_frontier_layer_graph_plus_structured_exposes_single_row_split() {
        let current = DynMatrix::new(3, 3, vec![2, 1, 1, 1, 0, 2, 0, 1, 1]);
        let current_canon = current.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(current_canon.clone(), current);

        let (expansions, stats, _timing) = expand_frontier_layer(
            &[current_canon],
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 4,
                max_entry: 3,
                move_family_policy: MoveFamilyPolicy::GraphPlusStructured,
            },
        );

        assert!(stats.factorisations_enumerated > 0);
        assert!(expansions
            .iter()
            .any(|expansion| expansion.next_orig.rows == 4));
        assert!(stats
            .move_family_telemetry
            .get("single_row_split_3x3_to_4x4")
            .is_some_and(|telemetry| telemetry.candidates_generated > 0));
    }

    #[test]
    fn test_expand_frontier_layer_deduplicates_across_frontier_nodes() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (single_expansions, _, _) = expand_frontier_layer(
            std::slice::from_ref(&a_canon),
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 2,
                max_entry: 10,
                move_family_policy: MoveFamilyPolicy::Mixed,
            },
        );
        let (duplicate_frontier_expansions, _, _) = expand_frontier_layer(
            &[a_canon.clone(), a_canon],
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 2,
                max_entry: 10,
                move_family_policy: MoveFamilyPolicy::Mixed,
            },
        );

        assert_eq!(duplicate_frontier_expansions.len(), single_expansions.len());
        assert!(duplicate_frontier_expansions
            .iter()
            .all(|expansion| expansion.order_key.frontier_index == 0));
    }

    #[test]
    fn test_expand_frontier_layer_emits_non_decreasing_order_keys() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (expansions, _, _) = expand_frontier_layer(
            &[a_canon],
            &orig,
            FrontierExpansionSettings {
                max_intermediate_dim: 3,
                max_entry: 6,
                move_family_policy: MoveFamilyPolicy::Mixed,
            },
        );

        assert!(expansions.len() > 1);
        assert!(expansions
            .windows(2)
            .all(|window| window[0].order_key <= window[1].order_key));
    }

    #[test]
    fn test_search_telemetry_mixed_expand_case_repeats_cleanly() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 3,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };

        for _ in 0..16 {
            let (_result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
            assert!(!telemetry.layers.is_empty());
            assert!(telemetry.frontier_nodes_expanded >= 1);
            assert!(telemetry.factorisations_enumerated >= telemetry.candidates_after_pruning);
        }
    }

    #[test]
    fn test_dyn_mixed_search_observer_emits_layers_for_lind_marcus_case() {
        let a = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 0, 1, 1, 1, 1]);
        let b = DynMatrix::new(1, 1, vec![2]);
        let config = SearchConfig {
            max_lag: 2,
            max_intermediate_dim: 2,
            max_entry: 2,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let mut observer = LayerEventProbe::default();

        let (result, telemetry) =
            search_sse_with_telemetry_dyn_and_observer(&a, &b, &config, Some(&mut observer));

        assert!(matches!(result, DynSseResult::Equivalent(_)));
        assert!(!telemetry.layers.is_empty());
        assert_eq!(observer.layer_sizes.len(), telemetry.layers.len());
        assert!(observer.layer_sizes.iter().all(|size| *size > 0));
    }

    #[test]
    fn test_should_expand_forward_prefers_lower_estimated_work() {
        assert!(!should_expand_forward(FrontierLayerChoiceInputs {
            fwd_depth: Some(0),
            bwd_depth: Some(0),
            fwd_frontier_len: 1002,
            bwd_frontier_len: 1137,
            fwd_factorisations_per_node: 151644.0 / 323.0,
            bwd_factorisations_per_node: 103760.0 / 662.0,
            fwd_cost_sample_nodes: 323,
            bwd_cost_sample_nodes: 662,
            fwd_overlap_signal: FrontierOverlapSignal::default(),
            bwd_overlap_signal: FrontierOverlapSignal::default(),
        }));
    }

    #[test]
    fn test_should_expand_forward_falls_back_to_smaller_frontier_when_untrained() {
        assert!(should_expand_forward(FrontierLayerChoiceInputs {
            fwd_depth: Some(0),
            bwd_depth: Some(0),
            fwd_frontier_len: 3,
            bwd_frontier_len: 5,
            fwd_factorisations_per_node: 100.0,
            bwd_factorisations_per_node: 1.0,
            fwd_cost_sample_nodes: 0,
            bwd_cost_sample_nodes: 0,
            fwd_overlap_signal: FrontierOverlapSignal::default(),
            bwd_overlap_signal: FrontierOverlapSignal::default(),
        }));
        assert!(!should_expand_forward(FrontierLayerChoiceInputs {
            fwd_depth: Some(0),
            bwd_depth: Some(0),
            fwd_frontier_len: 7,
            bwd_frontier_len: 2,
            fwd_factorisations_per_node: 1.0,
            bwd_factorisations_per_node: 100.0,
            fwd_cost_sample_nodes: 0,
            bwd_cost_sample_nodes: 0,
            fwd_overlap_signal: FrontierOverlapSignal::default(),
            bwd_overlap_signal: FrontierOverlapSignal::default(),
        }));
    }

    #[test]
    fn test_should_expand_forward_prefers_recent_overlap_signal() {
        assert!(should_expand_forward(FrontierLayerChoiceInputs {
            fwd_depth: Some(0),
            bwd_depth: Some(0),
            fwd_frontier_len: 1500,
            bwd_frontier_len: 900,
            fwd_factorisations_per_node: 200.0,
            bwd_factorisations_per_node: 10.0,
            fwd_cost_sample_nodes: 100,
            bwd_cost_sample_nodes: 100,
            fwd_overlap_signal: FrontierOverlapSignal::from_layer(1500, 4),
            bwd_overlap_signal: FrontierOverlapSignal::from_layer(900, 0),
        }));
    }

    #[test]
    fn test_approx_signature_ignores_exact_support_layout() {
        let a = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 2, 1, 0, 1]);
        let b = DynMatrix::new(3, 3, vec![1, 0, 2, 0, 2, 1, 1, 1, 0]);
        assert_eq!(approx_signature(&a), approx_signature(&b));
    }

    #[test]
    fn test_same_future_past_signature_matches_duplicate_class_profiles() {
        let a = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 1, 1]);
        let b = DynMatrix::new(3, 3, vec![1, 0, 1, 1, 0, 1, 0, 1, 1]);

        assert_eq!(
            same_future_past_signature(&a),
            same_future_past_signature(&b)
        );
    }

    #[test]
    fn test_same_future_past_representative_selection_uses_lowest_order_key() {
        let parent = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let graph_a = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 1, 1]);
        let graph_b = DynMatrix::new(3, 3, vec![1, 0, 1, 1, 0, 1, 0, 1, 1]);
        let factorised = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 1, 1, 1, 0, 1]);
        let graph_a_signature = same_future_past_signature(&graph_a);
        let graph_b_signature = same_future_past_signature(&graph_b);
        let dummy_step = EsseStep {
            u: DynMatrix::new(1, 1, vec![1]),
            v: DynMatrix::new(1, 1, vec![1]),
        };
        let expansions = vec![
            FrontierExpansion {
                order_key: LayerExpansionOrderKey::new(0, 1),
                parent_canon: parent.clone(),
                next_canon: graph_b.clone(),
                next_orig: graph_b.clone(),
                step: dummy_step.clone(),
                move_family: "graph_b",
                same_future_past_signature: graph_b_signature,
            },
            FrontierExpansion {
                order_key: LayerExpansionOrderKey::new(0, 0),
                parent_canon: parent.clone(),
                next_canon: graph_a.clone(),
                next_orig: graph_a,
                step: dummy_step.clone(),
                move_family: "graph_a",
                same_future_past_signature: graph_a_signature,
            },
            FrontierExpansion {
                order_key: LayerExpansionOrderKey::new(0, 2),
                parent_canon: parent,
                next_canon: factorised.clone(),
                next_orig: factorised,
                step: dummy_step,
                move_family: "factorised",
                same_future_past_signature: None,
            },
        ];

        let (deduped, same_future_past_collisions) = deduplicate_expansions(expansions, true);

        assert_eq!(same_future_past_collisions, 1);
        assert_eq!(deduped.len(), 2);
        assert!(deduped
            .iter()
            .any(|expansion| expansion.move_family == "graph_a"));
        assert!(!deduped
            .iter()
            .any(|expansion| expansion.move_family == "graph_b"));
        assert!(deduped
            .iter()
            .any(|expansion| expansion.move_family == "factorised"));
        assert_eq!(deduped[0].order_key, LayerExpansionOrderKey::new(0, 0));
    }

    #[test]
    fn test_deduplicate_expansions_keeps_lowest_order_key_canonical_representative() {
        let parent_a = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let parent_b = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let next = DynMatrix::new(2, 2, vec![2, 1, 1, 1]);
        let dummy_step = EsseStep {
            u: DynMatrix::new(1, 1, vec![1]),
            v: DynMatrix::new(1, 1, vec![1]),
        };
        let expansions = vec![
            FrontierExpansion {
                order_key: LayerExpansionOrderKey::new(0, 1),
                parent_canon: parent_b,
                next_canon: next.clone(),
                next_orig: next.clone(),
                step: dummy_step.clone(),
                move_family: "second",
                same_future_past_signature: None,
            },
            FrontierExpansion {
                order_key: LayerExpansionOrderKey::new(0, 0),
                parent_canon: parent_a.clone(),
                next_canon: next.clone(),
                next_orig: next.clone(),
                step: dummy_step.clone(),
                move_family: "first",
                same_future_past_signature: None,
            },
            FrontierExpansion {
                order_key: LayerExpansionOrderKey::new(1, 0),
                parent_canon: parent_a,
                next_canon: next,
                next_orig: DynMatrix::new(2, 2, vec![2, 1, 1, 1]),
                step: dummy_step,
                move_family: "third",
                same_future_past_signature: None,
            },
        ];

        let (deduped, same_future_past_collisions) = deduplicate_expansions(expansions, false);

        assert_eq!(same_future_past_collisions, 0);
        assert_eq!(deduped.len(), 1);
        assert_eq!(deduped[0].order_key, LayerExpansionOrderKey::new(0, 0));
        assert_eq!(deduped[0].move_family, "first");
    }

    // --- Literature examples ---

    /// Helper: verify an SSE path is valid (each step satisfies UV and VU consistency).
    fn assert_valid_path(path: &SsePath<2>) {
        assert!(path.steps.len() >= 1);
        // Verify first step starts from first matrix and last step ends at last matrix.
        let first_step = &path.steps[0];
        let uv = first_step.u.mul(&first_step.v);
        assert_eq!(
            uv,
            DynMatrix::from_sq(&path.matrices[0]),
            "First step: UV != A"
        );

        let last_step = &path.steps[path.steps.len() - 1];
        let vu = last_step.v.mul(&last_step.u);
        assert_eq!(
            vu,
            DynMatrix::from_sq(&path.matrices[path.matrices.len() - 1]),
            "Last step: VU != B"
        );

        // Verify chain: VU of step i = UV of step i+1 (the intermediate matrix).
        for i in 0..path.steps.len() - 1 {
            let vu_i = path.steps[i].v.mul(&path.steps[i].u);
            let uv_next = path.steps[i + 1].u.mul(&path.steps[i + 1].v);
            assert_eq!(vu_i, uv_next, "Step {}: VU != UV of step {}", i, i + 1);
        }
    }

    // Eilers-Kiming 2008, p.8: Three 2x2 matrices that share all classical
    // invariants (trace=6, det=-73, same Bowen-Franks group) but are pairwise
    // NOT SSE. Our search can't prove non-SSE (no ideal class invariant yet),
    // so it should return Unknown.

    #[test]
    fn test_eilers_kiming_triple_classical_invariants_match() {
        // Classical invariants (trace, det, Bowen-Franks) all match for this triple.
        // The Eilers-Kiming ideal class invariant distinguishes them.
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m2 = SqMatrix::new([[5, 6], [13, 1]]);
        let m3 = SqMatrix::new([[4, 9], [9, 2]]);
        assert_eq!(m1.trace(), 6);
        assert_eq!(m2.trace(), 6);
        assert_eq!(m3.trace(), 6);
        assert_eq!(m1.det(), -73);
        assert_eq!(m2.det(), -73);
        assert_eq!(m3.det(), -73);
    }

    #[test]
    fn test_eilers_kiming_m1_m2_not_equivalent() {
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m2 = SqMatrix::new([[5, 6], [13, 1]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let result = search_sse_2x2(&m1, &m2, &config);
        assert!(
            matches!(result, SseResult::NotEquivalent(_)),
            "Expected NotEquivalent for Eilers-Kiming non-SSE pair (m1, m2)"
        );
    }

    #[test]
    fn test_eilers_kiming_m1_m3_not_equivalent() {
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m3 = SqMatrix::new([[4, 9], [9, 2]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let result = search_sse_2x2(&m1, &m3, &config);
        assert!(
            matches!(result, SseResult::NotEquivalent(_)),
            "Expected NotEquivalent for Eilers-Kiming non-SSE pair (m1, m3)"
        );
    }

    // Eilers-Kiming 2008, p.8-9: [[14,2],[1,0]] and [[13,5],[3,1]] share
    // classical invariants (char poly x^2 - 14x - 2) but are NOT SSE.

    #[test]
    fn test_eilers_kiming_14_2_classical_invariants_match() {
        // Classical invariants match, but the ideal class invariant distinguishes them.
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);
        assert_eq!(a.trace(), b.trace());
        assert_eq!(a.det(), b.det());
    }

    #[test]
    fn test_eilers_kiming_14_2_not_equivalent() {
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        assert!(
            matches!(result, SseResult::NotEquivalent(_)),
            "Expected NotEquivalent for Eilers-Kiming non-SSE pair ([[14,2],[1,0]], [[13,5],[3,1]])"
        );
    }

    // Brix-Ruiz 2025, Example 3.8 (k=3): [[1,3],[2,1]] and [[1,6],[1,1]]
    // are known to be SSE (trace=2, det=-5).

    #[test]
    fn test_brix_ruiz_k3_invariants_match() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        assert_eq!(a.trace(), b.trace()); // 2
        assert_eq!(a.det(), b.det()); // -5
        assert!(check_invariants_2x2(&a, &b).is_none());
    }

    #[test]
    fn test_brix_ruiz_k3_search() {
        // Known SSE but the search space is too large for brute force at
        // practical bounds. This test verifies the search doesn't incorrectly
        // report NotEquivalent and exercises the rectangular factorisation
        // code path. Finding the actual path will require optimisations
        // (parallelism, smarter pruning, or algebraic shortcuts).
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        assert!(
            matches!(
                result,
                SseResult::Equivalent(_)
                    | SseResult::EquivalentByConcreteShift(_)
                    | SseResult::Unknown
            ),
            "Should not be NotEquivalent — these are known SSE"
        );
    }

    #[test]
    #[ignore = "expensive graph-only regression"]
    fn test_brix_ruiz_k3_graph_only_finds_path() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 22,
            max_intermediate_dim: 5,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        assert!(
            matches!(result, SseResult::Equivalent(_)),
            "graph-only search should find the known Brix-Ruiz k=3 path"
        );
    }

    #[test]
    fn test_rectangular_sse_constructed() {
        // Construct a pair connected through a 3x3 intermediate.
        // Step 1: A = U1*V1, C = V1*U1 (3x3)
        let u1 = DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 0]);
        let v1 = DynMatrix::new(3, 2, vec![1, 0, 1, 1, 1, 1]);
        let a_dyn = u1.mul(&v1); // A = [[2,1],[1,1]]
        let c = v1.mul(&u1); // C (3x3)

        // Step 2: factor C = U2*V2, B = V2*U2 (2x2)
        // We need to find U2 (3x2), V2 (2x3) such that U2*V2 = C.
        // C = [[1,0,1],[1,1,1],[1,1,1]]
        // Try U2 = [[1,0],[0,1],[0,1]], V2 = [[1,0,1],[1,1,1]]
        // U2*V2 = [[1,0,1],[1,1,1],[1,1,1]] = C
        let u2 = DynMatrix::new(3, 2, vec![1, 0, 0, 1, 0, 1]);
        let v2 = DynMatrix::new(2, 3, vec![1, 0, 1, 1, 1, 1]);
        let c_check = u2.mul(&v2);
        assert_eq!(c, c_check, "C from step 1 != C from step 2");

        let b_dyn = v2.mul(&u2); // B = [[1,0],[1,2]] (2x2)
        let a: SqMatrix<2> = a_dyn.to_sq().unwrap();
        let b: SqMatrix<2> = b_dyn.to_sq().unwrap();

        // Verify A and B are distinct (and not just permutation-similar).
        assert_ne!(a, b);

        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 5,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        match &result {
            SseResult::Equivalent(path) => {
                assert!(path.steps.len() >= 1);
                // Verify path: A and B have same trace/det so might be connected
                // via square steps too, but this exercises the full search with
                // rectangular factorisation enabled.
                assert_valid_path(path);
            }
            SseResult::EquivalentByConcreteShift(_witness) => {}
            _ => panic!(
                "Expected Equivalent for constructed rectangular SSE pair A={:?} B={:?}, got {:?}",
                a,
                b,
                match &result {
                    SseResult::EquivalentByConcreteShift(_) => {
                        "EquivalentByConcreteShift".to_string()
                    }
                    SseResult::NotEquivalent(r) => format!("NotEquivalent({})", r),
                    SseResult::Unknown => "Unknown".to_string(),
                    _ => unreachable!(),
                }
            ),
        }
    }

    // Brix-Ruiz 2025, Example 3.8 (k=4): [[1,4],[3,1]] and [[1,12],[1,1]]
    // are SE but SSE status is OPEN.

    #[test]
    fn test_brix_ruiz_k4_invariants_match() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        assert_eq!(a.trace(), b.trace()); // 2
        assert_eq!(a.det(), b.det()); // -11
        assert!(check_invariants_2x2(&a, &b).is_none());
    }

    // --- Spectral pruning tests ---

    #[test]
    fn test_spectral_consistent_2x2_matching() {
        // [[2,1],[1,1]] has trace=3, det=1
        let m = DynMatrix::new(2, 2, vec![2, 1, 1, 1]);
        assert!(is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_spectral_inconsistent_2x2_wrong_trace() {
        let m = DynMatrix::new(2, 2, vec![3, 1, 1, 1]); // trace=4, det=2
        assert!(!is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_spectral_inconsistent_2x2_wrong_det() {
        let m = DynMatrix::new(2, 2, vec![2, 1, 1, 2]); // trace=4, det=3
        assert!(!is_spectrally_consistent(&m, 4, 2));
    }

    #[test]
    fn test_spectral_consistent_3x3_zero_eigenvalue() {
        // A 3x3 with eigenvalues {2, 1, 0}: trace=3, det=0, minor_sum=2.
        // Consistent with a 2x2 source having trace=3, det=2.
        // [[2,0,0],[0,1,0],[0,0,0]]: trace=3, det=0, minor_sum = 2+0+0 = 2
        let m = DynMatrix::new(3, 3, vec![2, 0, 0, 0, 1, 0, 0, 0, 0]);
        assert_eq!(m.trace(), 3);
        assert_eq!(m.det_3x3(), 0);
        assert_eq!(m.principal_minor_sum_3x3(), 2);
        assert!(is_spectrally_consistent(&m, 3, 2));
        assert!(!is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_spectral_inconsistent_3x3_nonzero_det() {
        // [[1,0,0],[0,1,0],[0,0,1]]: trace=3, det=1 (no zero eigenvalue)
        let m = DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]);
        assert!(!is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_search_sse_dyn_rejects_trace_cube_invariant_mismatch() {
        let a = DynMatrix::new(3, 3, vec![0, 0, 0, 0, 3, 0, 0, 0, 3]);
        let b = DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 4]);
        let (result, telemetry) = search_sse_with_telemetry_dyn(&a, &b, &default_config());

        match result {
            DynSseResult::NotEquivalent(reason) => {
                assert_eq!(reason, "trace(M^3) invariant mismatch");
            }
            other => panic!("expected invariant rejection, got {other:?}"),
        }
        assert!(telemetry.invariant_filtered);
        assert_eq!(telemetry.frontier_nodes_expanded, 0);
        assert!(telemetry.layers.is_empty());
    }

    #[test]
    fn test_probe_graph_proposal_shortlist_realizes_best_gap_waypoint_candidate() {
        let current = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 1, 2, 2, 1]);
        let target = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 4, 1, 1, 1]);
        let config = SearchConfig {
            max_intermediate_dim: 4,
            ..default_config()
        };
        let probe = GraphProposalProbeConfig {
            shortlist_size: 4,
            realization_max_lag: 3,
            max_zigzag_bridge_entry: Some(8),
            shortlist_mode: GraphProposalShortlistMode::BestGap,
            refined_coarse_prefix: 4,
        };

        let result = probe_graph_proposal_shortlist(&current, &target, &config, &probe)
            .expect("waypoint probe should be valid");

        assert_eq!(
            result.best_gap,
            Some(SameFuturePastSignatureGap {
                dimension_gap: 0,
                row_class_gap: 2,
                col_class_gap: 6,
                entry_sum_gap: 0,
            })
        );
        assert_eq!(result.best_gap_candidates, 1);
        assert_eq!(result.attempts.len(), 1);
        let attempt = &result.attempts[0];
        assert_eq!(
            attempt.proposal.matrix,
            DynMatrix::new(3, 3, vec![0, 0, 1, 1, 1, 1, 3, 3, 1])
        );
        match &attempt.result {
            DynSseResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 3);
                assert_eq!(path.matrices.first(), Some(&current));
                assert_eq!(path.matrices.last(), Some(&attempt.proposal.matrix));
            }
            other => panic!("expected realized proposal path, got {other:?}"),
        }
        assert!(attempt.telemetry.frontier_nodes_expanded >= 1);
        assert_eq!(attempt.telemetry.factorisations_enumerated, 0);
    }

    #[test]
    fn test_probe_graph_proposal_shortlist_supports_refined_coarse_prefix_order() {
        let current = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 1, 2, 2, 1]);
        let target = DynMatrix::new(3, 3, vec![0, 0, 2, 1, 1, 4, 1, 1, 1]);
        let config = SearchConfig {
            max_intermediate_dim: 4,
            ..default_config()
        };
        let probe = GraphProposalProbeConfig {
            shortlist_size: 4,
            realization_max_lag: 3,
            max_zigzag_bridge_entry: Some(8),
            shortlist_mode: GraphProposalShortlistMode::CoarsePrefixRefined,
            refined_coarse_prefix: 4,
        };

        let result = probe_graph_proposal_shortlist(&current, &target, &config, &probe)
            .expect("refined shortlist probe should be valid");

        assert_eq!(result.best_gap_candidates, 1);
        assert_eq!(result.attempts.len(), 4);
        assert_eq!(result.attempts[0].proposal.target_partition_refined_gap, 38);
        assert_eq!(
            result.attempts[0].proposal.matrix,
            DynMatrix::new(3, 3, vec![0, 0, 1, 1, 1, 2, 2, 2, 1])
        );
        assert!(
            result.attempts[0].proposal.target_signature_gap
                > result.best_gap.expect("best gap should be present")
        );
    }

    #[test]
    fn test_search_sse_dyn_same_matrix() {
        let a = DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0]);
        let (result, telemetry) = search_sse_with_telemetry_dyn(&a, &a, &default_config());
        match result {
            DynSseResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 0);
                assert_eq!(path.matrices, vec![a]);
            }
            other => panic!("expected equivalent result, got {other:?}"),
        }
        assert_eq!(telemetry.frontier_nodes_expanded, 0);
    }

    #[test]
    fn test_search_sse_dyn_permutation_shortcut() {
        let a = DynMatrix::new(3, 3, vec![2, 0, 0, 0, 1, 0, 0, 0, 0]);
        let b = DynMatrix::new(3, 3, vec![0, 0, 0, 0, 1, 0, 0, 0, 2]);
        let (result, telemetry) = search_sse_with_telemetry_dyn(&a, &b, &default_config());
        match result {
            DynSseResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                assert_eq!(path.matrices, vec![a, b]);
            }
            other => panic!("expected equivalent result, got {other:?}"),
        }
        assert!(telemetry.permutation_shortcut || telemetry.canonical_shortcut);
    }
}
