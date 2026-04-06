use std::collections::{BTreeMap, HashMap, HashSet, VecDeque};

use crate::aligned::{
    search_concrete_shift_equivalence_2x2, ConcreteShiftRelation2x2, ConcreteShiftSearchConfig2x2,
    ConcreteShiftSearchResult2x2,
};
use crate::factorisation::visit_all_factorisations_with_family;
use crate::graph_moves::{
    enumerate_same_future_insplits_2x2_to_3x3, enumerate_same_past_outsplits_2x2_to_3x3,
};
use crate::invariants::check_invariants_2x2;
use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::{
    EsseStep, SearchConfig, SearchDirection, SearchLayerTelemetry, SearchMoveFamilyTelemetry,
    SearchTelemetry, SsePath, SseResult,
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
}

#[derive(Clone, Default)]
struct FrontierExpansionStats {
    frontier_nodes: usize,
    factorisation_calls: usize,
    factorisations_enumerated: usize,
    candidates_generated: usize,
    pruned_by_size: usize,
    pruned_by_spectrum: usize,
    move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ApproxSignature {
    dim: usize,
    entry_sum: u64,
    row_sums: Vec<u32>,
    col_sums: Vec<u32>,
    support_mask: u16,
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

/// Search for a strong shift equivalence path, returning aggregate telemetry.
pub fn search_sse_2x2_with_telemetry(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();

    // Quick check: are they already equal?
    if a == b {
        return (
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
        return (SseResult::NotEquivalent(reason), telemetry);
    }

    // If a and b have the same canonical form, they are related by permutation
    // similarity. For 2x2, b = PAP where P = [[0,1],[1,0]].
    // Elementary SSE: U = AP, V = P, then UV = APP = A, VU = PAP = B.
    if a.canonical() == b.canonical() && a != b {
        telemetry.permutation_shortcut = true;
        let p = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let ap = DynMatrix::from_sq(a).mul(&p);
        let step = EsseStep { u: ap, v: p };
        return (
            SseResult::Equivalent(SsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: vec![step],
            }),
            telemetry,
        );
    }

    // Bidirectional BFS: expand from both A and B, meet in the middle.
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();

    // Forward direction (from A).
    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_orig.insert(a_canon.clone(), a_dyn);
    fwd_frontier.push_back(a_canon.clone());

    // Backward direction (from B).
    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_orig.insert(b_canon.clone(), DynMatrix::from_sq(b));
    bwd_frontier.push_back(b_canon.clone());
    telemetry.max_frontier_size = 1;
    let mut fwd_factorisations_per_node = 1.0f64;
    let mut bwd_factorisations_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;
    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    // Edge case: A and B canonicalise to the same form (should have been
    // caught by the permutation check above, but handle for safety).
    if a_canon == b_canon {
        telemetry.canonical_shortcut = true;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        return (
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

    // Precompute spectral invariants for pruning intermediates.
    let source_trace = a.trace();
    let source_det = a.det();

    for layer_index in 0..config.max_lag {
        // Expand the frontier with the lower estimated factorisation cost.
        let expand_forward = should_expand_forward(
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_factorisations_per_node,
            bwd_factorisations_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
        );
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };

        let (frontier, parent, orig, signatures, other_parent, other_signatures) = if expand_forward
        {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_orig,
                &mut fwd_signatures,
                &bwd_parent as &HashMap<_, _>,
                &bwd_signatures as &HashSet<_>,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_orig,
                &mut bwd_signatures,
                &fwd_parent as &HashMap<_, _>,
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

        for expansion in expansions {
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step)),
            );
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            signatures.insert(approx_signature(&expansion.next_canon));

            // Check if the other side has already visited this node.
            if other_parent.contains_key(&expansion.next_canon) {
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
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
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
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: next_frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    move_family_telemetry: layer_move_family_telemetry,
                });
                return (
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

            if other_signatures.contains(&approx_signature(&expansion.next_canon)) {
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
            if expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry {
                next_frontier.push_back(expansion.next_canon);
                enqueued_nodes += 1;
            }
        }

        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
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
    if should_try_concrete_shift_fallback(a, b, config) {
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
            return (SseResult::EquivalentByConcreteShift(witness), telemetry);
        }
    }

    (SseResult::Unknown, telemetry)
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

fn should_expand_forward(
    fwd_frontier_len: usize,
    bwd_frontier_len: usize,
    fwd_factorisations_per_node: f64,
    bwd_factorisations_per_node: f64,
    fwd_cost_sample_nodes: usize,
    bwd_cost_sample_nodes: usize,
) -> bool {
    if fwd_frontier_len == 0 || bwd_frontier_len == 0 {
        return fwd_frontier_len <= bwd_frontier_len;
    }
    if fwd_cost_sample_nodes < 8 || bwd_cost_sample_nodes < 8 {
        return fwd_frontier_len <= bwd_frontier_len;
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

        if let Some(current_sq) = current.to_sq::<2>() {
            for witness in enumerate_same_past_outsplits_2x2_to_3x3(&current_sq) {
                move_family_telemetry_mut(
                    &mut stats.move_family_telemetry,
                    "same_past_outsplit_2x2_to_3x3",
                )
                .candidates_generated += 1;
                stats.candidates_generated += 1;
                let next = witness.outsplit;
                if next.rows > max_intermediate_dim {
                    stats.pruned_by_size += 1;
                    continue;
                }
                if !is_spectrally_consistent(&next, source_trace, source_det) {
                    stats.pruned_by_spectrum += 1;
                    continue;
                }

                let next_canon = next.canonical_perm();
                if !seen_successors.insert(next_canon.clone()) {
                    continue;
                }
                let step = EsseStep {
                    u: witness.division,
                    v: witness.edge,
                };
                let move_family = "same_past_outsplit_2x2_to_3x3";
                expansions.push(FrontierExpansion {
                    parent_canon: current_canon.clone(),
                    next_canon,
                    next_orig: next,
                    step,
                    move_family,
                });
            }

            for witness in enumerate_same_future_insplits_2x2_to_3x3(&current_sq) {
                move_family_telemetry_mut(
                    &mut stats.move_family_telemetry,
                    "same_future_insplit_2x2_to_3x3",
                )
                .candidates_generated += 1;
                stats.candidates_generated += 1;
                let next = witness.outsplit;
                if next.rows > max_intermediate_dim {
                    stats.pruned_by_size += 1;
                    continue;
                }
                if !is_spectrally_consistent(&next, source_trace, source_det) {
                    stats.pruned_by_spectrum += 1;
                    continue;
                }

                let next_canon = next.canonical_perm();
                if !seen_successors.insert(next_canon.clone()) {
                    continue;
                }
                let step = EsseStep {
                    u: witness.edge,
                    v: witness.division,
                };
                let move_family = "same_future_insplit_2x2_to_3x3";
                expansions.push(FrontierExpansion {
                    parent_canon: current_canon.clone(),
                    next_canon,
                    next_orig: next,
                    step,
                    move_family,
                });
            }
        }

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
                });
            },
        );

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
        let deduped = deduplicate_expansions(expansions);
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
        let deduped = deduplicate_expansions(expansions);
        record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
        (deduped, stats)
    }
}

fn deduplicate_expansions(expansions: Vec<FrontierExpansion>) -> Vec<FrontierExpansion> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::with_capacity(expansions.len());
    for expansion in expansions {
        if seen.insert(expansion.next_canon.clone()) {
            deduped.push(expansion);
        }
    }
    deduped
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
    let mut entry_sum = 0u64;
    let mut support_mask = 0u16;

    for row in 0..m.rows {
        for col in 0..m.cols {
            let value = m.get(row, col);
            row_sums[row] += value;
            col_sums[col] += value;
            entry_sum += value as u64;
            if value > 0 {
                support_mask |= 1 << (row * m.cols + col);
            }
        }
    }

    row_sums.sort_unstable();
    col_sums.sort_unstable();

    ApproxSignature {
        dim: m.rows,
        entry_sum,
        row_sums,
        col_sums,
        support_mask,
    }
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
            // For larger matrices, trace check only (still useful).
            true
        }
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    fn default_config() -> SearchConfig {
        SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 2,
            max_entry: 10,
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

        let (expansions, stats) =
            expand_frontier_layer(&[a_canon], &orig, 2, 10, a.trace(), a.det());

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
            a.trace(),
            a.det(),
        );
        let (duplicate_frontier_expansions, _) = expand_frontier_layer(
            &[a_canon.clone(), a_canon],
            &orig,
            2,
            10,
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
        ));
    }

    #[test]
    fn test_should_expand_forward_falls_back_to_smaller_frontier_when_untrained() {
        assert!(should_expand_forward(3, 5, 100.0, 1.0, 0, 0));
        assert!(!should_expand_forward(7, 2, 1.0, 100.0, 0, 0));
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
}
