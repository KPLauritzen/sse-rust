use std::time::Instant;

use ahash::{AHashMap as HashMap, AHashSet as HashSet};

use crate::factorisation::visit_factorisations_with_family_for_policy;
use crate::graph_moves::{
    enumerate_graph_move_successors, same_future_past_signature, SameFuturePastSignature,
};
use crate::matrix::DynMatrix;
use crate::types::{EsseStep, MoveFamilyPolicy};

use rayon::prelude::*;

use super::{
    accumulate_move_family_telemetry_accumulator, deadline_reached, elapsed_nanos,
    move_family_telemetry_mut, MoveFamilyTelemetryAccumulator,
};

const SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD: usize = 8;
const TIMED_SEARCH_FRONTIER_CHUNK_SIZE: usize = 256;

#[derive(Clone)]
pub(super) struct FrontierExpansion {
    pub(super) order_key: LayerExpansionOrderKey,
    pub(super) parent_canon: DynMatrix,
    pub(super) next_canon: DynMatrix,
    pub(super) next_orig: DynMatrix,
    pub(super) step: EsseStep,
    pub(super) move_family: &'static str,
    pub(super) same_future_past_signature: Option<SameFuturePastSignature>,
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub(super) struct LayerExpansionOrderKey {
    pub(super) frontier_index: usize,
    pub(super) successor_index: usize,
}

impl LayerExpansionOrderKey {
    pub(super) const fn new(frontier_index: usize, successor_index: usize) -> Self {
        Self {
            frontier_index,
            successor_index,
        }
    }
}

#[derive(Clone, Default)]
pub(super) struct FrontierExpansionStats {
    pub(super) frontier_nodes: usize,
    pub(super) factorisation_calls: usize,
    pub(super) factorisations_enumerated: usize,
    pub(super) candidates_generated: usize,
    pub(super) pruned_by_size: usize,
    pub(super) pruned_by_spectrum: usize,
    pub(super) same_future_past_collisions: usize,
    pub(super) move_family_telemetry: MoveFamilyTelemetryAccumulator,
}

#[derive(Clone, Copy, Default)]
pub(super) struct FrontierExpansionTiming {
    pub(super) expand_compute_nanos: u64,
    pub(super) expand_accumulate_nanos: u64,
    pub(super) dedup_nanos: u64,
}

#[derive(Clone, Copy)]
pub(super) struct FrontierExpansionSettings {
    pub(super) max_intermediate_dim: usize,
    pub(super) max_entry: u32,
    pub(super) move_family_policy: MoveFamilyPolicy,
}

#[derive(Clone, Copy, Default)]
pub(super) struct FrontierOverlapSignal {
    pub(super) frontier_nodes: usize,
    pub(super) approximate_other_side_hits: usize,
}

impl FrontierOverlapSignal {
    pub(super) fn from_layer(frontier_nodes: usize, approximate_other_side_hits: usize) -> Self {
        Self {
            frontier_nodes,
            approximate_other_side_hits,
        }
    }

    fn is_trained(self) -> bool {
        self.frontier_nodes >= 8
    }

    fn overlap_ratio(self) -> f64 {
        self.approximate_other_side_hits as f64 / self.frontier_nodes.max(1) as f64
    }
}

#[derive(Clone, Copy)]
pub(super) struct FrontierLayerChoiceInputs {
    pub(super) fwd_depth: Option<usize>,
    pub(super) bwd_depth: Option<usize>,
    pub(super) fwd_frontier_len: usize,
    pub(super) bwd_frontier_len: usize,
    pub(super) fwd_factorisations_per_node: f64,
    pub(super) bwd_factorisations_per_node: f64,
    pub(super) fwd_cost_sample_nodes: usize,
    pub(super) bwd_cost_sample_nodes: usize,
    pub(super) fwd_overlap_signal: FrontierOverlapSignal,
    pub(super) bwd_overlap_signal: FrontierOverlapSignal,
}

pub(super) fn expand_frontier_layer_dyn(
    current_frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    settings: FrontierExpansionSettings,
    deadline: Option<Instant>,
) -> (
    Vec<FrontierExpansion>,
    FrontierExpansionStats,
    FrontierExpansionTiming,
    bool,
) {
    let mut expansions = Vec::new();
    let mut stats = FrontierExpansionStats::default();
    let mut timing = FrontierExpansionTiming::default();
    let mut timed_out = false;
    let chunk_size = frontier_chunk_size(current_frontier.len(), deadline);
    for (chunk_index, chunk) in current_frontier.chunks(chunk_size).enumerate() {
        if deadline_reached(deadline) {
            timed_out = true;
            break;
        }
        let frontier_offset = chunk_index * chunk_size;

        let compute_started = Instant::now();
        let per_node: Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)> = chunk
            .par_iter()
            .enumerate()
            .map(|(node_index, current_canon)| {
                expand_frontier_node(frontier_offset + node_index, current_canon, orig, settings)
            })
            .collect();
        timing.expand_compute_nanos += elapsed_nanos(compute_started);

        let accumulate_started = Instant::now();
        for (node_expansions, node_stats) in per_node {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
        timing.expand_accumulate_nanos += elapsed_nanos(accumulate_started);
    }

    let dedup_started = Instant::now();
    let (deduped, same_future_past_collisions) = deduplicate_expansions(
        expansions,
        current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
    );
    timing.dedup_nanos = elapsed_nanos(dedup_started);
    stats.same_future_past_collisions = same_future_past_collisions;
    record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
    (deduped, stats, timing, timed_out)
}

pub(super) fn expand_frontier_layer(
    current_frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    settings: FrontierExpansionSettings,
) -> (
    Vec<FrontierExpansion>,
    FrontierExpansionStats,
    FrontierExpansionTiming,
) {
    let compute_started = Instant::now();
    let per_node: Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)> = current_frontier
        .par_iter()
        .enumerate()
        .map(|(frontier_index, current_canon)| {
            expand_frontier_node(frontier_index, current_canon, orig, settings)
        })
        .collect();
    let expand_compute_nanos = elapsed_nanos(compute_started);
    let mut expansions = Vec::new();
    let mut stats = FrontierExpansionStats::default();
    let accumulate_started = Instant::now();
    for (node_expansions, node_stats) in per_node {
        expansions.extend(node_expansions);
        accumulate_frontier_stats(&mut stats, &node_stats);
    }
    let expand_accumulate_nanos = elapsed_nanos(accumulate_started);
    let dedup_started = Instant::now();
    let (deduped, same_future_past_collisions) = deduplicate_expansions(
        expansions,
        current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
    );
    let dedup_nanos = elapsed_nanos(dedup_started);
    stats.same_future_past_collisions = same_future_past_collisions;
    record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
    (
        deduped,
        stats,
        FrontierExpansionTiming {
            expand_compute_nanos,
            expand_accumulate_nanos,
            dedup_nanos,
        },
    )
}

pub(super) fn deduplicate_expansions(
    mut expansions: Vec<FrontierExpansion>,
    enable_same_future_past_representatives: bool,
) -> (Vec<FrontierExpansion>, usize) {
    let needs_sort = expansions
        .windows(2)
        .any(|window| window[0].order_key > window[1].order_key);
    #[cfg(not(test))]
    debug_assert!(
        !needs_sort,
        "frontier expansions should already be emitted in nondecreasing order_key order"
    );
    if needs_sort {
        expansions.sort_unstable_by_key(|expansion| expansion.order_key);
    }
    let mut seen = HashSet::new();
    let mut same_future_past_seen = HashSet::new();
    let mut deduped = Vec::with_capacity(expansions.len());
    let mut same_future_past_collisions = 0usize;
    for mut expansion in expansions {
        if seen.contains(&expansion.next_canon) {
            continue;
        }
        if enable_same_future_past_representatives && expansion.next_canon.rows >= 3 {
            if let Some(signature) = expansion.same_future_past_signature.take() {
                if !same_future_past_seen.insert(signature) {
                    same_future_past_collisions += 1;
                    continue;
                }
            }
        }
        seen.insert(expansion.next_canon.clone());
        deduped.push(expansion);
    }
    (deduped, same_future_past_collisions)
}

pub(super) fn choose_next_layer(inputs: FrontierLayerChoiceInputs) -> Option<(bool, usize)> {
    match (inputs.fwd_depth, inputs.bwd_depth) {
        (Some(fwd), Some(bwd)) => {
            if fwd < bwd {
                Some((true, fwd))
            } else if bwd < fwd {
                Some((false, bwd))
            } else {
                Some((should_expand_forward(inputs), fwd))
            }
        }
        (Some(fwd), None) => Some((true, fwd)),
        (None, Some(bwd)) => Some((false, bwd)),
        (None, None) => None,
    }
}

pub(super) fn should_expand_forward(inputs: FrontierLayerChoiceInputs) -> bool {
    if inputs.fwd_frontier_len == 0 || inputs.bwd_frontier_len == 0 {
        return inputs.fwd_frontier_len <= inputs.bwd_frontier_len;
    }
    if inputs.fwd_cost_sample_nodes < 8 || inputs.bwd_cost_sample_nodes < 8 {
        return inputs.fwd_frontier_len <= inputs.bwd_frontier_len;
    }

    if inputs.fwd_overlap_signal.is_trained() && inputs.bwd_overlap_signal.is_trained() {
        if inputs.fwd_overlap_signal.approximate_other_side_hits > 0
            && inputs.bwd_overlap_signal.approximate_other_side_hits == 0
        {
            return true;
        }
        if inputs.bwd_overlap_signal.approximate_other_side_hits > 0
            && inputs.fwd_overlap_signal.approximate_other_side_hits == 0
        {
            return false;
        }

        let fwd_overlap_ratio = inputs.fwd_overlap_signal.overlap_ratio();
        let bwd_overlap_ratio = inputs.bwd_overlap_signal.overlap_ratio();
        if inputs.fwd_overlap_signal.approximate_other_side_hits >= 2
            && fwd_overlap_ratio > bwd_overlap_ratio * 2.0
        {
            return true;
        }
        if inputs.bwd_overlap_signal.approximate_other_side_hits >= 2
            && bwd_overlap_ratio > fwd_overlap_ratio * 2.0
        {
            return false;
        }
    }

    let fwd_estimated_work =
        inputs.fwd_frontier_len as f64 * inputs.fwd_factorisations_per_node.max(1.0);
    let bwd_estimated_work =
        inputs.bwd_frontier_len as f64 * inputs.bwd_factorisations_per_node.max(1.0);
    fwd_estimated_work <= bwd_estimated_work
}

fn frontier_chunk_size(frontier_len: usize, deadline: Option<Instant>) -> usize {
    if deadline.is_some() {
        frontier_len.min(TIMED_SEARCH_FRONTIER_CHUNK_SIZE).max(1)
    } else {
        frontier_len.max(1)
    }
}

fn expand_frontier_node(
    frontier_index: usize,
    current_canon: &DynMatrix,
    orig: &HashMap<DynMatrix, DynMatrix>,
    settings: FrontierExpansionSettings,
) -> (Vec<FrontierExpansion>, FrontierExpansionStats) {
    let current = orig
        .get(current_canon)
        .expect("frontier node should have an original matrix");
    let mut expansions = Vec::new();
    let mut seen_successors = HashSet::new();
    let mut stats = FrontierExpansionStats {
        frontier_nodes: 1,
        factorisation_calls: 1,
        ..FrontierExpansionStats::default()
    };

    let graph_successors = enumerate_graph_move_successors(current, settings.max_intermediate_dim);
    stats.candidates_generated += graph_successors.candidates;
    for (family, count) in graph_successors.family_candidates {
        move_family_telemetry_mut(&mut stats.move_family_telemetry, family).candidates_generated +=
            count;
    }

    for successor in graph_successors.nodes {
        let next = successor.orig_matrix;
        let next_canon = successor.matrix;
        if !seen_successors.insert(next_canon.clone()) {
            continue;
        }
        let same_future_past_signature = same_future_past_signature(&next_canon);
        expansions.push(FrontierExpansion {
            order_key: LayerExpansionOrderKey::new(frontier_index, expansions.len()),
            parent_canon: current_canon.clone(),
            next_canon,
            next_orig: next,
            step: successor.step,
            move_family: successor.family,
            same_future_past_signature,
        });
    }

    if settings.move_family_policy.permits_factorisations() {
        visit_factorisations_with_family_for_policy(
            current,
            settings.max_intermediate_dim,
            settings.max_entry,
            settings.move_family_policy,
            |move_family, u, v| {
                move_family_telemetry_mut(&mut stats.move_family_telemetry, move_family)
                    .candidates_generated += 1;
                stats.factorisations_enumerated += 1;
                stats.candidates_generated += 1;
                let next = v.mul(&u);

                if next.rows > settings.max_intermediate_dim {
                    stats.pruned_by_size += 1;
                    return;
                }

                let next_canon = next.canonical_perm();
                if !seen_successors.insert(next_canon.clone()) {
                    return;
                }
                let step = EsseStep { u, v };
                expansions.push(FrontierExpansion {
                    order_key: LayerExpansionOrderKey::new(frontier_index, expansions.len()),
                    parent_canon: current_canon.clone(),
                    next_canon,
                    next_orig: next,
                    step,
                    move_family,
                    same_future_past_signature: None,
                });
            },
        );
    }

    (expansions, stats)
}

fn accumulate_frontier_stats(total: &mut FrontierExpansionStats, delta: &FrontierExpansionStats) {
    total.frontier_nodes += delta.frontier_nodes;
    total.factorisation_calls += delta.factorisation_calls;
    total.factorisations_enumerated += delta.factorisations_enumerated;
    total.candidates_generated += delta.candidates_generated;
    total.pruned_by_size += delta.pruned_by_size;
    total.pruned_by_spectrum += delta.pruned_by_spectrum;
    accumulate_move_family_telemetry_accumulator(
        &mut total.move_family_telemetry,
        &delta.move_family_telemetry,
    );
}

fn record_candidates_after_pruning_by_family(
    expansions: &[FrontierExpansion],
    telemetry: &mut MoveFamilyTelemetryAccumulator,
) {
    for expansion in expansions {
        move_family_telemetry_mut(telemetry, expansion.move_family).candidates_after_pruning += 1;
    }
}
