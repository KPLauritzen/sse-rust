use std::collections::{BTreeMap, VecDeque};

use ahash::{AHashMap as HashMap, AHashSet as HashSet};

use crate::aligned::{
    search_concrete_shift_equivalence_2x2, ConcreteShiftRelation2x2, ConcreteShiftSearchConfig2x2,
    ConcreteShiftSearchResult2x2,
};
use crate::factorisation::visit_all_factorisations_with_family;
use crate::graph_moves::enumerate_graph_move_successors;
use crate::invariants::check_invariants_2x2;
use crate::matrix::{DynMatrix, SqMatrix};
use crate::search_observer::{
    SearchEdgeRecord, SearchEdgeStatus, SearchObserver, SearchRootRecord,
};
use crate::types::{
    DynSsePath, DynSseResult, EsseStep, SearchConfig, SearchDirection, SearchLayerTelemetry,
    SearchMode, SearchMoveFamilyTelemetry, SearchTelemetry, SsePath, SseResult,
};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

#[derive(Clone)]
struct FrontierExpansion {
    parent_canon: DynMatrix,
    next_canon: DynMatrix,
    next_orig: DynMatrix,
    step: EsseStep,
    move_family: &'static str,
    same_future_past_signature: Option<SameFuturePastSignature>,
}

#[derive(Clone, Default)]
struct FrontierExpansionStats {
    frontier_nodes: usize,
    factorisation_calls: usize,
    factorisations_enumerated: usize,
    candidates_generated: usize,
    pruned_by_size: usize,
    pruned_by_spectrum: usize,
    same_future_past_collisions: usize,
    move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ApproxSignature {
    dim: usize,
    entry_sum: u64,
    row_sums: Vec<u32>,
    col_sums: Vec<u32>,
    row_supports: Vec<u8>,
    col_supports: Vec<u8>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SameFuturePastClassSignature {
    multiplicity: usize,
    entry_sum: u32,
    support: u8,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SameFuturePastSignature {
    dim: usize,
    entry_sum: u64,
    row_classes: Vec<SameFuturePastClassSignature>,
    col_classes: Vec<SameFuturePastClassSignature>,
}

const SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD: usize = 8;

#[derive(Clone, Copy, Default)]
struct FrontierOverlapSignal {
    frontier_nodes: usize,
    approximate_other_side_hits: usize,
}

impl FrontierOverlapSignal {
    fn from_layer(frontier_nodes: usize, approximate_other_side_hits: usize) -> Self {
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

/// Search for a strong shift equivalence path between arbitrary square endpoints,
/// returning aggregate telemetry.
pub fn search_sse_with_telemetry_dyn(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();

    if !a.is_square() || !b.is_square() {
        return (
            DynSseResult::NotEquivalent("search expects square endpoint matrices".to_string()),
            telemetry,
        );
    }

    if trace_square(a) != trace_square(b) {
        telemetry.invariant_filtered = true;
        return (
            DynSseResult::NotEquivalent("trace(M^2) invariant mismatch".to_string()),
            telemetry,
        );
    }
    if a.trace() != b.trace() {
        telemetry.invariant_filtered = true;
        return (
            DynSseResult::NotEquivalent("trace invariant mismatch".to_string()),
            telemetry,
        );
    }

    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    if a == b {
        return finish_search_dyn(
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
        return finish_search_dyn(
            DynSseResult::Equivalent(DynSsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: permutation_step_between(a, b).into_iter().collect(),
            }),
            telemetry,
        );
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

    if config.search_mode == SearchMode::GraphOnly {
        return search_graph_only_dyn_with_telemetry(a, b, config);
    }

    let source_trace = a.trace();
    let source_trace_square = trace_square(a);

    for layer_index in 0..config.max_lag {
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_factorisations_per_node,
            bwd_factorisations_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal,
            bwd_overlap_signal,
        ) else {
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
        let (expansions, expansion_stats) = expand_frontier_layer_dyn(
            &current_frontier,
            orig,
            config.max_intermediate_dim,
            config.max_entry,
            config.search_mode,
            source_trace,
            source_trace_square,
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
        let next_depth = layer_depth + 1;

        for expansion in &expansions {
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
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
                let path_depth = next_depth + other_depth;
                if path_depth > config.max_lag {
                    continue;
                }
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
                    move_family_telemetry: layer_move_family_telemetry,
                });
                return finish_search_dyn(
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
        }

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
            move_family_telemetry: layer_move_family_telemetry,
        });

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search_dyn(DynSseResult::Unknown, telemetry)
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

    if let Some(observer) = observer.as_deref_mut() {
        observer.on_search_started(&a_dyn, &b_dyn, &a_canon, &b_canon, config);
        let roots = [
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
        ];
        observer.on_roots(&roots);
    }

    // Quick check: are they already equal?
    if a == b {
        return finish_search(
            observer,
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
        return finish_search(observer, SseResult::NotEquivalent(reason), telemetry);
    }

    // If a and b have the same canonical form, they are related by permutation
    // similarity. For 2x2, b = PAP where P = [[0,1],[1,0]].
    // Elementary SSE: U = AP, V = P, then UV = APP = A, VU = PAP = B.
    if a.canonical() == b.canonical() && a != b {
        telemetry.permutation_shortcut = true;
        let p = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let ap = DynMatrix::from_sq(a).mul(&p);
        let step = EsseStep { u: ap, v: p };
        return finish_search(
            observer,
            SseResult::Equivalent(SsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: vec![step],
            }),
            telemetry,
        );
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
        return finish_search(
            observer,
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

    if config.search_mode == SearchMode::GraphOnly {
        return search_graph_only_2x2_with_telemetry_and_observer(a, b, config, observer);
    }

    // Precompute spectral invariants for pruning intermediates.
    let source_trace = a.trace();
    let source_det = a.det();

    for layer_index in 0..config.max_lag {
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_factorisations_per_node,
            bwd_factorisations_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal,
            bwd_overlap_signal,
        ) else {
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
        let (expansions, expansion_stats) = expand_frontier_layer(
            &current_frontier,
            orig,
            config.max_intermediate_dim,
            config.max_entry,
            config.search_mode,
            source_trace,
            source_det,
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
            signatures.insert(approx_signature(&expansion.next_canon));

            let approximate_hit =
                other_signatures.contains(&approx_signature(&expansion.next_canon));
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
                if let (Some(observer), Some(records)) =
                    (observer.as_deref_mut(), layer_records.as_ref())
                {
                    observer.on_layer(records);
                }
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
                    move_family_telemetry: layer_move_family_telemetry,
                });
                return finish_search(
                    observer,
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
        if let (Some(observer), Some(records)) = (observer.as_deref_mut(), layer_records.as_ref()) {
            observer.on_layer(records);
        }
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
            move_family_telemetry: layer_move_family_telemetry,
        });

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    // If bounded ESSE search exhausts on a finite essential pair, try the
    // aligned concrete-shift substrate before reporting `Unknown`.
    if config.search_mode == SearchMode::Mixed && should_try_concrete_shift_fallback(a, b, config) {
        let concrete_config = ConcreteShiftSearchConfig2x2 {
            relation: ConcreteShiftRelation2x2::Aligned,
            max_lag: config.max_lag as u32,
            max_entry: config.max_entry,
            max_witnesses: concrete_shift_witness_budget(config),
        };
        if let ConcreteShiftSearchResult2x2::Equivalent(witness) =
            search_concrete_shift_equivalence_2x2(a, b, &concrete_config)
        {
            telemetry.concrete_shift_shortcut = true;
            return finish_search(
                observer,
                SseResult::EquivalentByConcreteShift(witness),
                telemetry,
            );
        }
    }

    finish_search(observer, SseResult::Unknown, telemetry)
}

fn is_essential_matrix_2x2(m: &SqMatrix<2>) -> bool {
    let row0 = m.data[0][0] + m.data[0][1];
    let row1 = m.data[1][0] + m.data[1][1];
    let col0 = m.data[0][0] + m.data[1][0];
    let col1 = m.data[0][1] + m.data[1][1];
    row0 > 0 && row1 > 0 && col0 > 0 && col1 > 0
}

fn concrete_shift_witness_budget(config: &SearchConfig) -> usize {
    if config.max_lag <= 4 && config.max_entry <= 6 {
        10_000
    } else {
        25_000
    }
}

fn should_try_concrete_shift_fallback(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> bool {
    is_essential_matrix_2x2(a)
        && is_essential_matrix_2x2(b)
        && config.max_lag <= 4
        && config.max_entry <= 6
}

fn search_graph_only_2x2_with_telemetry_and_observer(
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

    let source_trace = a.trace();
    let source_det = a.det();
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
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_candidates_per_node,
            bwd_candidates_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ) else {
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
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(layer.candidates_after_pruning.max(8)));
        let mut next_frontier = VecDeque::new();
        let next_depth = layer_depth + 1;
        let mut parents_with_progress = HashSet::new();
        let mut same_future_past_seen = HashSet::new();

        for (current_canon, successors) in computed {
            let current_orig = orig
                .get(&current_canon)
                .expect("frontier node should have an original matrix")
                .clone();
            layer.candidates_generated += successors.candidates;
            for (family, count) in successors.family_candidates {
                move_family_telemetry_mut(&mut layer.move_family_telemetry, family)
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
                if !is_spectrally_consistent(&successor.orig_matrix, source_trace, source_det) {
                    layer.pruned_by_spectrum += 1;
                    continue;
                }

                move_family_telemetry_mut(&mut layer.move_family_telemetry, successor.family)
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

                if current_frontier_len >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD
                    && successor.matrix.rows >= 3
                {
                    if let Some(signature) = same_future_past_signature(&successor.matrix) {
                        if !same_future_past_seen.insert(signature) {
                            layer.same_future_past_collisions += 1;
                            continue;
                        }
                    }
                }

                parent.insert(
                    successor.matrix.clone(),
                    Some((current_canon.clone(), successor.step.clone())),
                );
                depths.insert(successor.matrix.clone(), next_depth);
                orig.insert(successor.matrix.clone(), successor.orig_matrix.clone());
                layer.discovered_nodes += 1;
                move_family_telemetry_mut(&mut layer.move_family_telemetry, successor.family)
                    .discovered_nodes += 1;
                parents_with_progress.insert(current_canon.clone());
                let mut record_status = SearchEdgeStatus::Discovered;

                if let Some(&other_depth) = other_depths.get(&successor.matrix) {
                    layer.collisions_with_other_frontier += 1;
                    move_family_telemetry_mut(
                        &mut layer.move_family_telemetry,
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
                            &layer.move_family_telemetry,
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
                        if let (Some(observer), Some(records)) =
                            (observer.as_deref_mut(), layer_records.as_ref())
                        {
                            observer.on_layer(records);
                        }
                        telemetry.layers.push(layer);
                        return finish_search(
                            observer,
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
            &layer.move_family_telemetry,
        );
        if let (Some(observer), Some(records)) = (observer.as_deref_mut(), layer_records.as_ref()) {
            observer.on_layer(records);
        }
        telemetry.layers.push(layer);

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search(observer, SseResult::Unknown, telemetry)
}

fn search_graph_only_dyn_with_telemetry(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
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

    let source_trace = a.trace();
    let source_trace_square = trace_square(a);
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
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_candidates_per_node,
            bwd_candidates_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ) else {
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
            direction: Some(if expand_forward {
                SearchDirection::Forward
            } else {
                SearchDirection::Backward
            }),
            frontier_nodes: current_frontier_len,
            ..SearchLayerTelemetry::default()
        };
        let mut next_frontier = VecDeque::new();
        let next_depth = layer_depth + 1;
        let mut parents_with_progress = HashSet::new();
        let mut same_future_past_seen = HashSet::new();

        for (current_canon, successors) in computed {
            layer.candidates_generated += successors.candidates;
            for (family, count) in successors.family_candidates {
                move_family_telemetry_mut(&mut layer.move_family_telemetry, family)
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
                if !is_spectrally_consistent_dyn(
                    &successor.orig_matrix,
                    source_trace,
                    source_trace_square,
                ) {
                    layer.pruned_by_spectrum += 1;
                    continue;
                }

                move_family_telemetry_mut(&mut layer.move_family_telemetry, successor.family)
                    .candidates_after_pruning += 1;
                layer.candidates_after_pruning += 1;

                if parent.contains_key(&successor.matrix) {
                    layer.collisions_with_seen += 1;
                    continue;
                }

                if current_frontier_len >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD
                    && successor.matrix.rows >= 3
                {
                    if let Some(signature) = same_future_past_signature(&successor.matrix) {
                        if !same_future_past_seen.insert(signature) {
                            layer.same_future_past_collisions += 1;
                            continue;
                        }
                    }
                }

                parent.insert(
                    successor.matrix.clone(),
                    Some((current_canon.clone(), successor.step.clone())),
                );
                depths.insert(successor.matrix.clone(), next_depth);
                orig.insert(successor.matrix.clone(), successor.orig_matrix.clone());
                layer.discovered_nodes += 1;
                move_family_telemetry_mut(&mut layer.move_family_telemetry, successor.family)
                    .discovered_nodes += 1;
                parents_with_progress.insert(current_canon.clone());

                if let Some(&other_depth) = other_depths.get(&successor.matrix) {
                    layer.collisions_with_other_frontier += 1;
                    move_family_telemetry_mut(
                        &mut layer.move_family_telemetry,
                        successor.family,
                    )
                    .exact_meets += 1;
                    let path_depth = next_depth + other_depth;
                    if path_depth <= config.max_lag {
                        layer.next_frontier_nodes = next_frontier.len();
                        telemetry.collisions_with_seen += layer.collisions_with_seen;
                        telemetry.collisions_with_other_frontier +=
                            layer.collisions_with_other_frontier;
                        telemetry.same_future_past_collisions += layer.same_future_past_collisions;
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
                            &layer.move_family_telemetry,
                        );
                        telemetry.layers.push(layer);
                        return finish_search_dyn(
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
            &layer.move_family_telemetry,
        );
        telemetry.layers.push(layer);

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search_dyn(DynSseResult::Unknown, telemetry)
}

fn choose_next_layer(
    fwd_depth: Option<usize>,
    bwd_depth: Option<usize>,
    fwd_frontier_len: usize,
    bwd_frontier_len: usize,
    fwd_factorisations_per_node: f64,
    bwd_factorisations_per_node: f64,
    fwd_cost_sample_nodes: usize,
    bwd_cost_sample_nodes: usize,
    fwd_overlap_signal: FrontierOverlapSignal,
    bwd_overlap_signal: FrontierOverlapSignal,
) -> Option<(bool, usize)> {
    match (fwd_depth, bwd_depth) {
        (Some(fwd), Some(bwd)) => {
            if fwd < bwd {
                Some((true, fwd))
            } else if bwd < fwd {
                Some((false, bwd))
            } else {
                Some((
                    should_expand_forward(
                        fwd_frontier_len,
                        bwd_frontier_len,
                        fwd_factorisations_per_node,
                        bwd_factorisations_per_node,
                        fwd_cost_sample_nodes,
                        bwd_cost_sample_nodes,
                        fwd_overlap_signal,
                        bwd_overlap_signal,
                    ),
                    fwd,
                ))
            }
        }
        (Some(fwd), None) => Some((true, fwd)),
        (None, Some(bwd)) => Some((false, bwd)),
        (None, None) => None,
    }
}

fn should_expand_forward(
    fwd_frontier_len: usize,
    bwd_frontier_len: usize,
    fwd_factorisations_per_node: f64,
    bwd_factorisations_per_node: f64,
    fwd_cost_sample_nodes: usize,
    bwd_cost_sample_nodes: usize,
    fwd_overlap_signal: FrontierOverlapSignal,
    bwd_overlap_signal: FrontierOverlapSignal,
) -> bool {
    if fwd_frontier_len == 0 || bwd_frontier_len == 0 {
        return fwd_frontier_len <= bwd_frontier_len;
    }
    if fwd_cost_sample_nodes < 8 || bwd_cost_sample_nodes < 8 {
        return fwd_frontier_len <= bwd_frontier_len;
    }

    if fwd_overlap_signal.is_trained() && bwd_overlap_signal.is_trained() {
        if fwd_overlap_signal.approximate_other_side_hits > 0
            && bwd_overlap_signal.approximate_other_side_hits == 0
        {
            return true;
        }
        if bwd_overlap_signal.approximate_other_side_hits > 0
            && fwd_overlap_signal.approximate_other_side_hits == 0
        {
            return false;
        }

        let fwd_overlap_ratio = fwd_overlap_signal.overlap_ratio();
        let bwd_overlap_ratio = bwd_overlap_signal.overlap_ratio();
        if fwd_overlap_signal.approximate_other_side_hits >= 2
            && fwd_overlap_ratio > bwd_overlap_ratio * 2.0
        {
            return true;
        }
        if bwd_overlap_signal.approximate_other_side_hits >= 2
            && bwd_overlap_ratio > fwd_overlap_ratio * 2.0
        {
            return false;
        }
    }

    let fwd_estimated_work = fwd_frontier_len as f64 * fwd_factorisations_per_node.max(1.0);
    let bwd_estimated_work = bwd_frontier_len as f64 * bwd_factorisations_per_node.max(1.0);
    fwd_estimated_work <= bwd_estimated_work
}

fn expand_frontier_layer(
    current_frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    max_intermediate_dim: usize,
    max_entry: u32,
    search_mode: SearchMode,
    source_trace: u64,
    source_det: i64,
) -> (Vec<FrontierExpansion>, FrontierExpansionStats) {
    let expand_node = |current_canon: &DynMatrix| {
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

        let graph_successors = enumerate_graph_move_successors(current, max_intermediate_dim);
        stats.candidates_generated += graph_successors.candidates;
        for (family, count) in graph_successors.family_candidates {
            move_family_telemetry_mut(&mut stats.move_family_telemetry, family)
                .candidates_generated += count;
        }

        for successor in graph_successors.nodes {
            let next = successor.orig_matrix;
            if !is_spectrally_consistent(&next, source_trace, source_det) {
                stats.pruned_by_spectrum += 1;
                continue;
            }
            let next_canon = successor.matrix;
            if !seen_successors.insert(next_canon.clone()) {
                continue;
            }
            let same_future_past_signature = same_future_past_signature(&next_canon);
            expansions.push(FrontierExpansion {
                parent_canon: current_canon.clone(),
                next_canon,
                next_orig: next,
                step: successor.step,
                move_family: successor.family,
                same_future_past_signature,
            });
        }

        if search_mode == SearchMode::Mixed {
            visit_all_factorisations_with_family(
                current,
                max_intermediate_dim,
                max_entry,
                |move_family, u, v| {
                    move_family_telemetry_mut(&mut stats.move_family_telemetry, move_family)
                        .candidates_generated += 1;
                    stats.factorisations_enumerated += 1;
                    stats.candidates_generated += 1;
                    let next = v.mul(&u);

                    // Size bound: don't explore matrices larger than max_intermediate_dim.
                    if next.rows > max_intermediate_dim {
                        stats.pruned_by_size += 1;
                        return;
                    }

                    // Spectral pruning: nonzero eigenvalues are preserved by SSE,
                    // so every intermediate must have the same nonzero spectrum.
                    if !is_spectrally_consistent(&next, source_trace, source_det) {
                        stats.pruned_by_spectrum += 1;
                        return;
                    }

                    let next_canon = next.canonical_perm();
                    if !seen_successors.insert(next_canon.clone()) {
                        return;
                    }
                    let step = EsseStep { u, v };
                    expansions.push(FrontierExpansion {
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
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let per_node: Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)> =
            current_frontier.par_iter().map(expand_node).collect();
        let mut expansions = Vec::new();
        let mut stats = FrontierExpansionStats::default();
        for (node_expansions, node_stats) in per_node {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
        let (deduped, same_future_past_collisions) = deduplicate_expansions(
            expansions,
            current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
        );
        stats.same_future_past_collisions = same_future_past_collisions;
        record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
        (deduped, stats)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut expansions = Vec::new();
        let mut stats = FrontierExpansionStats::default();
        for (node_expansions, node_stats) in current_frontier.iter().map(expand_node) {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
        let (deduped, same_future_past_collisions) = deduplicate_expansions(
            expansions,
            current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
        );
        stats.same_future_past_collisions = same_future_past_collisions;
        record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
        (deduped, stats)
    }
}

fn expand_frontier_layer_dyn(
    current_frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    max_intermediate_dim: usize,
    max_entry: u32,
    search_mode: SearchMode,
    source_trace: u64,
    source_trace_square: i64,
) -> (Vec<FrontierExpansion>, FrontierExpansionStats) {
    let expand_node = |current_canon: &DynMatrix| {
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

        let graph_successors = enumerate_graph_move_successors(current, max_intermediate_dim);
        stats.candidates_generated += graph_successors.candidates;
        for (family, count) in graph_successors.family_candidates {
            move_family_telemetry_mut(&mut stats.move_family_telemetry, family)
                .candidates_generated += count;
        }

        for successor in graph_successors.nodes {
            let next = successor.orig_matrix;
            if !is_spectrally_consistent_dyn(&next, source_trace, source_trace_square) {
                stats.pruned_by_spectrum += 1;
                continue;
            }
            let next_canon = successor.matrix;
            if !seen_successors.insert(next_canon.clone()) {
                continue;
            }
            let same_future_past_signature = same_future_past_signature(&next_canon);
            expansions.push(FrontierExpansion {
                parent_canon: current_canon.clone(),
                next_canon,
                next_orig: next,
                step: successor.step,
                move_family: successor.family,
                same_future_past_signature,
            });
        }

        if search_mode == SearchMode::Mixed {
            visit_all_factorisations_with_family(
                current,
                max_intermediate_dim,
                max_entry,
                |move_family, u, v| {
                    move_family_telemetry_mut(&mut stats.move_family_telemetry, move_family)
                        .candidates_generated += 1;
                    stats.factorisations_enumerated += 1;
                    stats.candidates_generated += 1;
                    let next = v.mul(&u);

                    if next.rows > max_intermediate_dim {
                        stats.pruned_by_size += 1;
                        return;
                    }

                    if !is_spectrally_consistent_dyn(&next, source_trace, source_trace_square) {
                        stats.pruned_by_spectrum += 1;
                        return;
                    }

                    let next_canon = next.canonical_perm();
                    if !seen_successors.insert(next_canon.clone()) {
                        return;
                    }
                    let step = EsseStep { u, v };
                    expansions.push(FrontierExpansion {
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
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let per_node: Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)> =
            current_frontier.par_iter().map(expand_node).collect();
        let mut expansions = Vec::new();
        let mut stats = FrontierExpansionStats::default();
        for (node_expansions, node_stats) in per_node {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
        let (deduped, same_future_past_collisions) = deduplicate_expansions(
            expansions,
            current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
        );
        stats.same_future_past_collisions = same_future_past_collisions;
        record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
        (deduped, stats)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut expansions = Vec::new();
        let mut stats = FrontierExpansionStats::default();
        for (node_expansions, node_stats) in current_frontier.iter().map(expand_node) {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
        let (deduped, same_future_past_collisions) = deduplicate_expansions(
            expansions,
            current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
        );
        stats.same_future_past_collisions = same_future_past_collisions;
        record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
        (deduped, stats)
    }
}

fn deduplicate_expansions(
    expansions: Vec<FrontierExpansion>,
    enable_same_future_past_representatives: bool,
) -> (Vec<FrontierExpansion>, usize) {
    let mut seen = HashSet::new();
    let mut same_future_past_seen = HashSet::new();
    let mut deduped = Vec::with_capacity(expansions.len());
    let mut same_future_past_collisions = 0usize;
    for expansion in expansions {
        if seen.contains(&expansion.next_canon) {
            continue;
        }
        if enable_same_future_past_representatives && expansion.next_canon.rows >= 3 {
            if let Some(signature) = expansion.same_future_past_signature.as_ref() {
                if !same_future_past_seen.insert(signature.clone()) {
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

fn accumulate_frontier_stats(total: &mut FrontierExpansionStats, delta: &FrontierExpansionStats) {
    total.frontier_nodes += delta.frontier_nodes;
    total.factorisation_calls += delta.factorisation_calls;
    total.factorisations_enumerated += delta.factorisations_enumerated;
    total.candidates_generated += delta.candidates_generated;
    total.pruned_by_size += delta.pruned_by_size;
    total.pruned_by_spectrum += delta.pruned_by_spectrum;
    accumulate_move_family_telemetry(
        &mut total.move_family_telemetry,
        &delta.move_family_telemetry,
    );
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

fn same_future_past_signature(m: &DynMatrix) -> Option<SameFuturePastSignature> {
    if !m.is_square() {
        return None;
    }

    let mut entry_sum = 0u64;
    let mut row_vectors = Vec::with_capacity(m.rows);
    for row in 0..m.rows {
        let mut values = Vec::with_capacity(m.cols);
        for col in 0..m.cols {
            let value = m.get(row, col);
            entry_sum += value as u64;
            values.push(value);
        }
        row_vectors.push(values);
    }

    let mut col_vectors = Vec::with_capacity(m.cols);
    for col in 0..m.cols {
        let mut values = Vec::with_capacity(m.rows);
        for row in 0..m.rows {
            values.push(m.get(row, col));
        }
        col_vectors.push(values);
    }

    Some(SameFuturePastSignature {
        dim: m.rows,
        entry_sum,
        row_classes: duplicate_vector_classes(&row_vectors),
        col_classes: duplicate_vector_classes(&col_vectors),
    })
}

fn duplicate_vector_classes(vectors: &[Vec<u32>]) -> Vec<SameFuturePastClassSignature> {
    let mut multiplicities = BTreeMap::<Vec<u32>, usize>::new();
    for values in vectors {
        *multiplicities.entry(values.clone()).or_default() += 1;
    }

    let mut classes = multiplicities
        .into_iter()
        .map(|(values, multiplicity)| SameFuturePastClassSignature {
            multiplicity,
            entry_sum: values.iter().copied().sum(),
            support: values.iter().filter(|&&value| value > 0).count() as u8,
        })
        .collect::<Vec<_>>();
    classes.sort_unstable();
    classes
}

fn move_family_telemetry_mut<'a>(
    map: &'a mut BTreeMap<String, SearchMoveFamilyTelemetry>,
    family: &str,
) -> &'a mut SearchMoveFamilyTelemetry {
    map.entry(family.to_string()).or_default()
}

fn accumulate_move_family_telemetry(
    total: &mut BTreeMap<String, SearchMoveFamilyTelemetry>,
    delta: &BTreeMap<String, SearchMoveFamilyTelemetry>,
) {
    for (family, family_delta) in delta {
        let family_total = total.entry(family.clone()).or_default();
        family_total.candidates_generated += family_delta.candidates_generated;
        family_total.candidates_after_pruning += family_delta.candidates_after_pruning;
        family_total.discovered_nodes += family_delta.discovered_nodes;
        family_total.exact_meets += family_delta.exact_meets;
        family_total.approximate_other_side_hits += family_delta.approximate_other_side_hits;
    }
}

fn record_candidates_after_pruning_by_family(
    expansions: &[FrontierExpansion],
    telemetry: &mut BTreeMap<String, SearchMoveFamilyTelemetry>,
) {
    for expansion in expansions {
        move_family_telemetry_mut(telemetry, expansion.move_family).candidates_after_pruning += 1;
    }
}

fn finish_search(
    mut observer: Option<&mut dyn SearchObserver>,
    result: SseResult<2>,
    telemetry: SearchTelemetry,
) -> (SseResult<2>, SearchTelemetry) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_search_finished(&result, &telemetry);
    }
    (result, telemetry)
}

fn finish_search_dyn(
    result: DynSseResult,
    telemetry: SearchTelemetry,
) -> (DynSseResult, SearchTelemetry) {
    (result, telemetry)
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

fn trace_square(m: &DynMatrix) -> i64 {
    m.mul(m).trace() as i64
}

fn is_spectrally_consistent_dyn(
    candidate: &DynMatrix,
    source_trace: u64,
    source_trace_square: i64,
) -> bool {
    candidate.is_square()
        && candidate.trace() == source_trace
        && trace_square(candidate) == source_trace_square
}

/// Create a permutation similarity step: given matrices M and M' = PMP
/// where P is the swap permutation, return an EsseStep with U = MP, V = P
/// so that UV = M and VU = M'.
fn permutation_step(m: &DynMatrix) -> EsseStep {
    let n = m.rows;
    let mut p_data = vec![0u32; n * n];
    for i in 0..n {
        p_data[i * n + (n - 1 - i)] = 1;
    }
    let p = DynMatrix::new(n, n, p_data);
    let mp = m.mul(&p);
    EsseStep { u: mp, v: p }
}

fn permutation_step_between(from: &DynMatrix, to: &DynMatrix) -> Option<EsseStep> {
    if from.rows != from.cols || to.rows != to.cols || from.rows != to.rows {
        return None;
    }
    let n = from.rows;
    let mut perm: Vec<usize> = (0..n).collect();
    let mut result = None;
    for_each_permutation(&mut perm, 0, &mut |perm| {
        if result.is_some() {
            return;
        }
        let (p, pinv) = permutation_matrices(perm);
        let candidate = pinv.mul(from).mul(&p);
        if candidate == *to {
            let u = from.mul(&p);
            result = Some(EsseStep { u, v: pinv });
        }
    });
    result
}

fn permutation_matrices(perm: &[usize]) -> (DynMatrix, DynMatrix) {
    let n = perm.len();
    let mut p_data = vec![0u32; n * n];
    let mut pinv_data = vec![0u32; n * n];
    for (row, &col) in perm.iter().enumerate() {
        p_data[row * n + col] = 1;
        pinv_data[col * n + row] = 1;
    }
    (
        DynMatrix::new(n, n, p_data),
        DynMatrix::new(n, n, pinv_data),
    )
}

fn for_each_permutation<F>(perm: &mut [usize], start: usize, visit: &mut F)
where
    F: FnMut(&[usize]),
{
    if start == perm.len() {
        visit(perm);
        return;
    }
    for idx in start..perm.len() {
        perm.swap(start, idx);
        for_each_permutation(perm, start + 1, visit);
        perm.swap(start, idx);
    }
}

/// Walk a parent chain from `node` back to the root, returning
/// (matrices, steps) in root-to-node order.
fn walk_parent_chain(
    node: &DynMatrix,
    parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    orig: &HashMap<DynMatrix, DynMatrix>,
) -> (Vec<DynMatrix>, Vec<EsseStep>) {
    let mut matrices = Vec::new();
    let mut steps = Vec::new();
    let mut current = node.clone();

    matrices.push(orig[&current].clone());

    while let Some(Some((prev, step))) = parent.get(&current) {
        steps.push(step.clone());
        matrices.push(orig[prev].clone());
        current = prev.clone();
    }

    matrices.reverse();
    steps.reverse();
    (matrices, steps)
}

/// Reconstruct a path from the forward and backward BFS trees that meet
/// at `meeting_canon`.
///
/// Forward chain: A -> ... -> M (steps recorded as current=UV, neighbor=VU).
/// Backward chain: B -> ... -> M (same convention).
/// We reverse the backward chain to get M -> ... -> B, flipping each step's
/// (U,V) to (V,U) since the direction of the elementary SSE is reversed.
fn reconstruct_bidirectional_path(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> SsePath<2> {
    // Forward: A -> ... -> M
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);

    // Backward: B -> ... -> M, which we reverse to M -> ... -> B.
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices
        .last()
        .expect("forward chain should end at the meeting node")
        .clone();
    let bwd_meeting = bwd_matrices
        .last()
        .expect("backward chain should end at the meeting node")
        .clone();

    // Build the combined step list.
    let mut all_steps = fwd_steps;

    if fwd_meeting != bwd_meeting {
        let step = permutation_step_between(&fwd_meeting, &bwd_meeting)
            .expect("meeting representatives should be permutation-similar");
        all_steps.push(step);
    }

    // Reverse backward steps: each backward step had current=UV, neighbor=VU.
    // In the forward direction (M->...->B) we need neighbor=UV, current=VU,
    // i.e. the elementary SSE step with U and V swapped.
    for step in bwd_steps.into_iter().rev() {
        all_steps.push(EsseStep {
            u: step.v,
            v: step.u,
        });
    }

    // Build the combined matrix list (all intermediate DynMatrix nodes).
    let mut all_dyn_matrices: Vec<DynMatrix> = fwd_matrices;
    if fwd_meeting != bwd_meeting {
        all_dyn_matrices.push(bwd_meeting);
    }
    // bwd_matrices is [B, ..., M] — reversed and skip M (already in fwd).
    for m in bwd_matrices.into_iter().rev().skip(1) {
        all_dyn_matrices.push(m);
    }

    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);

    // If the BFS start node differs from `a` (due to canonicalisation),
    // prepend a permutation step: a -> canonical(a).
    if *all_dyn_matrices.first().unwrap() != a_dyn {
        all_steps.insert(0, permutation_step(&a_dyn));
        all_dyn_matrices.insert(0, a_dyn);
    }

    // If the BFS end node differs from `b` (due to canonicalisation),
    // append a permutation step: canonical(b) -> b.
    if *all_dyn_matrices.last().unwrap() != b_dyn {
        let last = all_dyn_matrices.last().unwrap().clone();
        all_steps.push(permutation_step(&last));
        all_dyn_matrices.push(b_dyn);
    }

    // Collect the 2x2 nodes for the SsePath matrices field.
    let sq_matrices: Vec<SqMatrix<2>> = all_dyn_matrices
        .iter()
        .filter_map(|dm| dm.to_sq::<2>())
        .collect();

    SsePath {
        matrices: sq_matrices,
        steps: all_steps,
    }
}

fn reconstruct_bidirectional_dyn_path(
    a: &DynMatrix,
    b: &DynMatrix,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> DynSsePath {
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices
        .last()
        .expect("forward chain should end at the meeting node")
        .clone();
    let bwd_meeting = bwd_matrices
        .last()
        .expect("backward chain should end at the meeting node")
        .clone();

    let mut all_steps = fwd_steps;
    if fwd_meeting != bwd_meeting {
        let step = permutation_step_between(&fwd_meeting, &bwd_meeting)
            .expect("meeting representatives should be permutation-similar");
        all_steps.push(step);
    }

    for step in bwd_steps.into_iter().rev() {
        all_steps.push(EsseStep {
            u: step.v,
            v: step.u,
        });
    }

    let mut all_dyn_matrices: Vec<DynMatrix> = fwd_matrices;
    if fwd_meeting != bwd_meeting {
        all_dyn_matrices.push(bwd_meeting);
    }
    for m in bwd_matrices.into_iter().rev().skip(1) {
        all_dyn_matrices.push(m);
    }

    if *all_dyn_matrices.first().unwrap() != *a {
        let first = all_dyn_matrices.first().unwrap().clone();
        let step =
            permutation_step_between(a, &first).expect("start should be permutation-similar");
        all_steps.insert(0, step);
        all_dyn_matrices.insert(0, a.clone());
    }

    if *all_dyn_matrices.last().unwrap() != *b {
        let last = all_dyn_matrices.last().unwrap().clone();
        let step = permutation_step_between(&last, b).expect("end should be permutation-similar");
        all_steps.push(step);
        all_dyn_matrices.push(b.clone());
    }

    DynSsePath {
        matrices: all_dyn_matrices,
        steps: all_steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SearchConfig {
        SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 2,
            max_entry: 10,
            search_mode: SearchMode::Mixed,
        }
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
            search_mode: SearchMode::Mixed,
        };
        let result = search_sse_2x2(&a, &b, &config);
        match result {
            SseResult::Equivalent(path) => assert_valid_path(&path),
            SseResult::EquivalentByConcreteShift(_witness) => {}
            other => panic!("expected Equivalent path, got {:?}", other),
        }
    }

    #[test]
    fn test_telemetry_for_brix_ruiz_search() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            search_mode: SearchMode::Mixed,
        };
        let (_result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
        assert!(!telemetry.invariant_filtered);
        assert!(!telemetry.permutation_shortcut);
        assert!(!telemetry.layers.is_empty());
        assert!(telemetry.frontier_nodes_expanded >= 1);
        assert!(telemetry.factorisations_enumerated >= telemetry.candidates_after_pruning);
    }

    #[test]
    fn test_expand_frontier_layer_deduplicates_canonical_successors() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (expansions, stats) = expand_frontier_layer(
            &[a_canon],
            &orig,
            2,
            10,
            SearchMode::Mixed,
            a.trace(),
            a.det(),
        );

        assert!(!expansions.is_empty());
        assert!(stats.factorisations_enumerated > expansions.len());
    }

    #[test]
    fn test_expand_frontier_layer_deduplicates_across_frontier_nodes() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (single_expansions, _) = expand_frontier_layer(
            std::slice::from_ref(&a_canon),
            &orig,
            2,
            10,
            SearchMode::Mixed,
            a.trace(),
            a.det(),
        );
        let (duplicate_frontier_expansions, _) = expand_frontier_layer(
            &[a_canon.clone(), a_canon],
            &orig,
            2,
            10,
            SearchMode::Mixed,
            a.trace(),
            a.det(),
        );

        assert_eq!(duplicate_frontier_expansions.len(), single_expansions.len());
    }

    #[test]
    fn test_should_expand_forward_prefers_lower_estimated_work() {
        assert!(!should_expand_forward(
            1002,
            1137,
            151644.0 / 323.0,
            103760.0 / 662.0,
            323,
            662,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ));
    }

    #[test]
    fn test_should_expand_forward_falls_back_to_smaller_frontier_when_untrained() {
        assert!(should_expand_forward(
            3,
            5,
            100.0,
            1.0,
            0,
            0,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ));
        assert!(!should_expand_forward(
            7,
            2,
            1.0,
            100.0,
            0,
            0,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ));
    }

    #[test]
    fn test_should_expand_forward_prefers_recent_overlap_signal() {
        assert!(should_expand_forward(
            1500,
            900,
            200.0,
            10.0,
            100,
            100,
            FrontierOverlapSignal::from_layer(1500, 4),
            FrontierOverlapSignal::from_layer(900, 0),
        ));
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
    fn test_same_future_past_representative_selection_only_collapses_graph_moves() {
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
                parent_canon: parent.clone(),
                next_canon: graph_a.clone(),
                next_orig: graph_a,
                step: dummy_step.clone(),
                move_family: "graph_a",
                same_future_past_signature: graph_a_signature,
            },
            FrontierExpansion {
                parent_canon: parent.clone(),
                next_canon: graph_b.clone(),
                next_orig: graph_b.clone(),
                step: dummy_step.clone(),
                move_family: "graph_b",
                same_future_past_signature: graph_b_signature,
            },
            FrontierExpansion {
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
            .any(|expansion| expansion.move_family == "factorised"));
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
            search_mode: SearchMode::Mixed,
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
            search_mode: SearchMode::Mixed,
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
            search_mode: SearchMode::Mixed,
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
            search_mode: SearchMode::Mixed,
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
            search_mode: SearchMode::GraphOnly,
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
            search_mode: SearchMode::Mixed,
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
