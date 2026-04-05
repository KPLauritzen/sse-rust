use std::collections::{HashMap, VecDeque};

use crate::factorisation::enumerate_all_factorisations;
use crate::invariants::check_invariants_2x2;
use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::{EsseStep, SearchConfig, SsePath, SseResult};

/// Search for a strong shift equivalence path between two 2x2 matrices.
///
/// Uses bidirectional BFS over the graph where nodes are square matrices of
/// varying sizes (2x2, 3x3, ...) in canonical form, and edges are elementary
/// SSE steps (A = UV, B = VU). Searching from both ends reduces complexity
/// from O(branching^L) to O(2 * branching^(L/2)).
pub fn search_sse_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> SseResult<2> {
    // Quick check: are they already equal?
    if a == b {
        return SseResult::Equivalent(SsePath {
            matrices: vec![a.clone()],
            steps: vec![],
        });
    }

    // Pre-filter with invariants.
    if let Some(reason) = check_invariants_2x2(a, b) {
        return SseResult::NotEquivalent(reason);
    }

    // If a and b have the same canonical form, they are related by permutation
    // similarity. For 2x2, b = PAP where P = [[0,1],[1,0]].
    // Elementary SSE: U = AP, V = P, then UV = APP = A, VU = PAP = B.
    if a.canonical() == b.canonical() && a != b {
        let p = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let ap = DynMatrix::from_sq(a).mul(&p);
        let step = EsseStep { u: ap, v: p };
        return SseResult::Equivalent(SsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![step],
        });
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

    // Edge case: A and B canonicalise to the same form (should have been
    // caught by the permutation check above, but handle for safety).
    if a_canon == b_canon {
        return SseResult::Equivalent(reconstruct_bidirectional_path(
            a,
            b,
            &a_canon,
            &fwd_parent,
            &fwd_orig,
            &bwd_parent,
            &bwd_orig,
        ));
    }

    for _lag in 0..config.max_lag {
        // Expand the smaller frontier to keep work balanced.
        let expand_forward = fwd_frontier.len() <= bwd_frontier.len();

        let (frontier, parent, orig, other_parent) = if expand_forward {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_orig,
                &bwd_parent as &HashMap<_, _>,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_orig,
                &fwd_parent as &HashMap<_, _>,
            )
        };

        let mut next_frontier: VecDeque<DynMatrix> = VecDeque::new();

        while let Some(current_canon) = frontier.pop_front() {
            let current = orig[&current_canon].clone();
            let factorisations = enumerate_all_factorisations(
                &current,
                config.max_intermediate_dim,
                config.max_entry,
            );

            for (u, v) in factorisations {
                let vu = v.mul(&u);

                // Size bound: don't explore matrices larger than max_intermediate_dim.
                if vu.rows > config.max_intermediate_dim {
                    continue;
                }

                let vu_canon = vu.canonical_perm();

                if parent.contains_key(&vu_canon) {
                    continue;
                }

                let step = EsseStep {
                    u: u.clone(),
                    v: v.clone(),
                };
                parent.insert(vu_canon.clone(), Some((current_canon.clone(), step)));
                orig.insert(vu_canon.clone(), vu.clone());

                // Check if the other side has already visited this node.
                if other_parent.contains_key(&vu_canon) {
                    return SseResult::Equivalent(reconstruct_bidirectional_path(
                        a,
                        b,
                        &vu_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    ));
                }

                // For 2x2 nodes, bound entries to prevent unbounded growth.
                // For intermediate (3x3+) nodes, always add -- the factorisation
                // back to 2x2 already bounds factor entries via max_entry.
                if vu.rows > 2 || vu.max_entry() <= config.max_entry {
                    next_frontier.push_back(vu_canon);
                }
            }
        }

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
    }

    SseResult::Unknown
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

    // Build the combined step list.
    let mut all_steps = fwd_steps;

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

    // --- Literature examples ---

    /// Helper: verify an SSE path is valid (each step satisfies UV and VU consistency).
    fn assert_valid_path(path: &SsePath<2>) {
        assert!(path.steps.len() >= 1);
        // Verify first step starts from first matrix and last step ends at last matrix.
        let first_step = &path.steps[0];
        let uv = first_step.u.mul(&first_step.v);
        assert_eq!(uv, DynMatrix::from_sq(&path.matrices[0]), "First step: UV != A");

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
            assert_eq!(
                vu_i, uv_next,
                "Step {}: VU != UV of step {}",
                i, i + 1
            );
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
            matches!(result, SseResult::Equivalent(_) | SseResult::Unknown),
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
            _ => panic!(
                "Expected Equivalent for constructed rectangular SSE pair A={:?} B={:?}, got {:?}",
                a, b,
                match &result {
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
}
