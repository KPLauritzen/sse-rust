use std::collections::{HashMap, VecDeque};

use crate::factorisation::{enumerate_square_factorisations_2x2, vu_product_2x2};
use crate::invariants::check_invariants_2x2;
use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::{EsseStep, SearchConfig, SsePath, SseResult};

/// Search for a strong shift equivalence path between two 2x2 matrices.
///
/// Uses BFS over the graph where nodes are matrices (in canonical form)
/// and edges are elementary SSE steps (A = UV, B = VU).
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

    let target_canonical = b.canonical();

    // BFS state: map from canonical matrix -> (parent canonical, step that produced it)
    let mut parent: HashMap<SqMatrix<2>, Option<(SqMatrix<2>, EsseStep)>> = HashMap::new();
    // Also track original (non-canonical) matrices so we can reconstruct the path
    // with the actual matrices used.
    let mut canonical_to_original: HashMap<SqMatrix<2>, SqMatrix<2>> = HashMap::new();
    let mut frontier: VecDeque<SqMatrix<2>> = VecDeque::new();

    let a_canon = a.canonical();
    parent.insert(a_canon.clone(), None);
    canonical_to_original.insert(a_canon.clone(), a.clone());
    frontier.push_back(a_canon.clone());

    for _lag in 0..config.max_lag {
        let mut next_frontier: VecDeque<SqMatrix<2>> = VecDeque::new();

        while let Some(current_canon) = frontier.pop_front() {
            let current = canonical_to_original[&current_canon].clone();
            let factorisations = enumerate_square_factorisations_2x2(&current, config.max_entry);

            for (u, v) in factorisations {
                let vu = vu_product_2x2(&v, &u);
                let vu_canon = vu.canonical();

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

                // Only add to next frontier if the matrix is within bounds.
                if vu.max_entry() <= config.max_entry {
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
    target_canon: &SqMatrix<2>,
    parent: &HashMap<SqMatrix<2>, Option<(SqMatrix<2>, EsseStep)>>,
    canonical_to_original: &HashMap<SqMatrix<2>, SqMatrix<2>>,
) -> SsePath<2> {
    let mut steps_rev = Vec::new();
    let mut matrices_rev = Vec::new();

    let mut current = target_canon.clone();
    matrices_rev.push(b.clone());

    while let Some(Some((prev, step))) = parent.get(&current) {
        steps_rev.push(step.clone());
        matrices_rev.push(canonical_to_original[prev].clone());
        current = prev.clone();
    }

    steps_rev.reverse();
    matrices_rev.reverse();

    // Fix the first and last to be exactly a and b.
    *matrices_rev.first_mut().unwrap() = a.clone();
    *matrices_rev.last_mut().unwrap() = b.clone();

    SsePath {
        matrices: matrices_rev,
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
                assert_eq!(path.matrices.len(), 2);
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
        // For any found path, verify each step: matrices[i] = UV, matrices[i+1] = VU
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let result = search_sse_2x2(&a, &b, &default_config());
        if let SseResult::Equivalent(path) = result {
            assert_eq!(path.matrices.len(), path.steps.len() + 1);
            for i in 0..path.steps.len() {
                let step = &path.steps[i];
                let uv = step.u.mul(&step.v);
                let vu = step.v.mul(&step.u);
                assert_eq!(uv, DynMatrix::from_sq(&path.matrices[i]));
                assert_eq!(vu, DynMatrix::from_sq(&path.matrices[i + 1]));
            }
        }
    }
}
