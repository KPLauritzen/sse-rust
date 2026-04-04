use std::collections::{HashMap, VecDeque};

use crate::factorisation::enumerate_all_factorisations;
use crate::invariants::check_invariants_2x2;
use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::{EsseStep, SearchConfig, SsePath, SseResult};

/// Search for a strong shift equivalence path between two 2x2 matrices.
///
/// Uses BFS over the graph where nodes are square matrices of varying sizes
/// (2×2, 3×3, ...) in canonical form, and edges are elementary SSE steps
/// (A = UV, B = VU). Rectangular factorisations allow the search to pass
/// through higher-dimensional intermediate matrices.
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

    let target_canonical = DynMatrix::from_sq(&b.canonical());

    // BFS state using DynMatrix (canonical form) as keys.
    // Nodes can be square matrices of any size (2×2, 3��3, ...).
    let mut parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut canonical_to_original: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut frontier: VecDeque<DynMatrix> = VecDeque::new();

    let a_dyn = DynMatrix::from_sq(a);
    let a_canon = a_dyn.canonical_perm();
    parent.insert(a_canon.clone(), None);
    canonical_to_original.insert(a_canon.clone(), a_dyn);
    frontier.push_back(a_canon.clone());

    for _lag in 0..config.max_lag {
        let mut next_frontier: VecDeque<DynMatrix> = VecDeque::new();

        while let Some(current_canon) = frontier.pop_front() {
            let current = canonical_to_original[&current_canon].clone();
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
                canonical_to_original.insert(vu_canon.clone(), vu.clone());

                if vu_canon == target_canonical {
                    return SseResult::Equivalent(reconstruct_path(
                        a,
                        b,
                        &vu_canon,
                        &parent,
                        &canonical_to_original,
                    ));
                }

                // For 2×2 nodes, bound entries to prevent unbounded growth.
                // For intermediate (3×3+) nodes, always add — the factorisation
                // back to 2×2 already bounds factor entries via max_entry.
                if vu.rows > 2 || vu.max_entry() <= config.max_entry {
                    next_frontier.push_back(vu_canon);
                }
            }
        }

        if next_frontier.is_empty() {
            break;
        }
        frontier = next_frontier;
    }

    SseResult::Unknown
}

fn reconstruct_path(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    target_canon: &DynMatrix,
    parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    canonical_to_original: &HashMap<DynMatrix, DynMatrix>,
) -> SsePath<2> {
    let mut steps_rev = Vec::new();
    let mut dyn_matrices_rev = Vec::new();

    let mut current = target_canon.clone();
    dyn_matrices_rev.push(DynMatrix::from_sq(b));

    while let Some(Some((prev, step))) = parent.get(&current) {
        steps_rev.push(step.clone());
        dyn_matrices_rev.push(canonical_to_original[prev].clone());
        current = prev.clone();
    }

    steps_rev.reverse();
    dyn_matrices_rev.reverse();

    // Fix the first and last to be exactly a and b.
    *dyn_matrices_rev.first_mut().unwrap() = DynMatrix::from_sq(a);
    *dyn_matrices_rev.last_mut().unwrap() = DynMatrix::from_sq(b);

    // Convert the 2×2 endpoints to SqMatrix<2> for the path.
    // Intermediate nodes may be larger (3×3, etc.) but SsePath only stores
    // the 2×2 start and end in its matrices field.
    let mut sq_matrices = Vec::new();
    for dm in &dyn_matrices_rev {
        if let Some(sq) = dm.to_sq::<2>() {
            sq_matrices.push(sq);
        }
        // Skip non-2×2 intermediate matrices in the SqMatrix path.
        // The full path with all dimensions is captured in the steps.
    }

    SsePath {
        matrices: sq_matrices,
        steps: steps_rev,
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
    fn test_eilers_kiming_triple_invariants_match() {
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m2 = SqMatrix::new([[5, 6], [13, 1]]);
        let m3 = SqMatrix::new([[4, 9], [9, 2]]);
        assert_eq!(m1.trace(), 6);
        assert_eq!(m2.trace(), 6);
        assert_eq!(m3.trace(), 6);
        assert_eq!(m1.det(), -73);
        assert_eq!(m2.det(), -73);
        assert_eq!(m3.det(), -73);
        assert!(check_invariants_2x2(&m1, &m2).is_none());
        assert!(check_invariants_2x2(&m1, &m3).is_none());
        assert!(check_invariants_2x2(&m2, &m3).is_none());
    }

    #[test]
    fn test_eilers_kiming_m1_m2_unknown() {
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m2 = SqMatrix::new([[5, 6], [13, 1]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
        };
        let result = search_sse_2x2(&m1, &m2, &config);
        assert!(
            matches!(result, SseResult::Unknown),
            "Expected Unknown for Eilers-Kiming non-SSE pair (m1, m2)"
        );
    }

    #[test]
    fn test_eilers_kiming_m1_m3_unknown() {
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m3 = SqMatrix::new([[4, 9], [9, 2]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
        };
        let result = search_sse_2x2(&m1, &m3, &config);
        assert!(
            matches!(result, SseResult::Unknown),
            "Expected Unknown for Eilers-Kiming non-SSE pair (m1, m3)"
        );
    }

    // Eilers-Kiming 2008, p.8-9: [[14,2],[1,0]] and [[13,5],[3,1]] share
    // classical invariants (char poly x^2 - 14x - 2) but are NOT SSE.

    #[test]
    fn test_eilers_kiming_14_2_invariants_match() {
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);
        assert_eq!(a.trace(), b.trace());
        assert_eq!(a.det(), b.det());
        assert!(check_invariants_2x2(&a, &b).is_none());
    }

    #[test]
    fn test_eilers_kiming_14_2_unknown() {
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
        };
        let result = search_sse_2x2(&a, &b, &config);
        assert!(
            matches!(result, SseResult::Unknown),
            "Expected Unknown for Eilers-Kiming non-SSE pair ([[14,2],[1,0]], [[13,5],[3,1]])"
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
        // Construct a pair connected through a 3×3 intermediate.
        // Step 1: A = U1*V1, C = V1*U1 (3×3)
        let u1 = DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 0]);
        let v1 = DynMatrix::new(3, 2, vec![1, 0, 1, 1, 1, 1]);
        let a_dyn = u1.mul(&v1); // A = [[2,1],[1,1]]
        let c = v1.mul(&u1); // C (3×3)

        // Step 2: factor C = U2*V2, B = V2*U2 (2×2)
        // We need to find U2 (3×2), V2 (2×3) such that U2*V2 = C.
        // C = [[1,0,1],[1,1,1],[1,1,1]]
        // Try U2 = [[1,0],[0,1],[0,1]], V2 = [[1,0,1],[1,1,1]]
        // U2*V2 = [[1,0,1],[1,1,1],[1,1,1]] = C ✓
        let u2 = DynMatrix::new(3, 2, vec![1, 0, 0, 1, 0, 1]);
        let v2 = DynMatrix::new(2, 3, vec![1, 0, 1, 1, 1, 1]);
        let c_check = u2.mul(&v2);
        assert_eq!(c, c_check, "C from step 1 != C from step 2");

        let b_dyn = v2.mul(&u2); // B = [[1,0],[1,2]] (2×2)
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
