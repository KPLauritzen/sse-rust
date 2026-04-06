use std::collections::{HashMap, HashSet, VecDeque};
use std::time::{Duration, Instant};

use crate::graph_moves::{enumerate_one_step_insplits, enumerate_one_step_outsplits};
use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::{EsseStep, SsePath};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

/// Configuration for the balanced SSE BFS.
pub struct BalancedBfsConfig {
    /// Maximum intermediate matrix dimension (e.g. 4 means up to 4x4).
    pub max_intermediate_dim: usize,
    /// Maximum BFS layers to expand.
    pub max_layers: usize,
    /// Maximum frontier size before bailing out.
    pub max_frontier_size: usize,
    /// Time limit for the search.
    pub time_limit: Duration,
}

/// Result of the balanced SSE BFS.
pub enum BalancedBfsResult {
    Equivalent(SsePath<2>),
    Exhausted,
}

/// Bidirectional BFS over the balanced SSE move graph (outsplits, insplits,
/// and their reverses = amalgamations). This restricts to 0-1 division
/// matrices, which is a much smaller move family than general factorisation.
pub fn balanced_sse_bfs_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &BalancedBfsConfig,
) -> BalancedBfsResult {
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();

    let source_trace = a.trace();

    // Forward BFS state (from A).
    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_orig.insert(a_canon.clone(), a_dyn.clone());
    fwd_frontier.push_back(a_canon.clone());

    // Backward BFS state (from B).
    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_orig.insert(b_canon.clone(), b_dyn.clone());
    bwd_frontier.push_back(b_canon.clone());

    if a_canon == b_canon {
        return BalancedBfsResult::Equivalent(reconstruct_balanced_path(
            a,
            b,
            &a_canon,
            &fwd_parent,
            &fwd_orig,
            &bwd_parent,
            &bwd_orig,
        ));
    }

    let deadline = Instant::now() + config.time_limit;

    for _layer in 0..config.max_layers {
        if Instant::now() >= deadline {
            break;
        }

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

        let current: Vec<DynMatrix> = frontier.drain(..).collect();
        if current.is_empty() {
            break;
        }

        // Expand all frontier nodes in parallel.
        let orig_snapshot = &*orig;
        let max_dim = config.max_intermediate_dim;

        #[cfg(not(target_arch = "wasm32"))]
        let expansions: Vec<Vec<(DynMatrix, DynMatrix, EsseStep)>> = current
            .par_iter()
            .map(|current_canon| {
                let current_orig = match orig_snapshot.get(current_canon) {
                    Some(m) => m,
                    None => return Vec::new(),
                };
                enumerate_balanced_neighbors(current_orig, max_dim, source_trace)
                    .into_iter()
                    .map(|(neighbor_orig, step)| {
                        let neighbor_canon = neighbor_orig.canonical_perm();
                        (neighbor_canon, neighbor_orig, step)
                    })
                    .collect()
            })
            .collect();

        #[cfg(target_arch = "wasm32")]
        let expansions: Vec<Vec<(DynMatrix, DynMatrix, EsseStep)>> = current
            .iter()
            .map(|current_canon| {
                let current_orig = match orig_snapshot.get(current_canon) {
                    Some(m) => m,
                    None => return Vec::new(),
                };
                enumerate_balanced_neighbors(current_orig, max_dim, source_trace)
                    .into_iter()
                    .map(|(neighbor_orig, step)| {
                        let neighbor_canon = neighbor_orig.canonical_perm();
                        (neighbor_canon, neighbor_orig, step)
                    })
                    .collect()
            })
            .collect();

        // Process expansions sequentially for dedup and meeting detection.
        for (idx, node_expansions) in expansions.into_iter().enumerate() {
            let current_canon = &current[idx];
            for (neighbor_canon, neighbor_orig, step) in node_expansions {
                if parent.contains_key(&neighbor_canon) {
                    continue;
                }

                parent.insert(neighbor_canon.clone(), Some((current_canon.clone(), step)));
                orig.insert(neighbor_canon.clone(), neighbor_orig);

                if other_parent.contains_key(&neighbor_canon) {
                    return BalancedBfsResult::Equivalent(reconstruct_balanced_path(
                        a,
                        b,
                        &neighbor_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    ));
                }

                frontier.push_back(neighbor_canon);
            }
        }

        if frontier.len() > config.max_frontier_size {
            break;
        }
    }

    BalancedBfsResult::Exhausted
}

/// Enumerate all balanced SSE neighbors of a square matrix:
/// outsplits (n→n+1), insplits (n→n+1), column amalgamation (n→n-1),
/// row amalgamation (n→n-1).
fn enumerate_balanced_neighbors(
    node: &DynMatrix,
    max_dim: usize,
    source_trace: u64,
) -> Vec<(DynMatrix, EsseStep)> {
    let mut results = Vec::new();
    let mut seen = HashSet::new();

    // Outsplits (n → n+1): A = D·E, C = E·D.
    // Step: u = D, v = E. node = u·v, outsplit = v·u.
    if node.rows + 1 <= max_dim {
        for witness in enumerate_one_step_outsplits(node) {
            if witness.outsplit.trace() != source_trace {
                continue;
            }
            let canon = witness.outsplit.canonical_perm();
            if seen.insert(canon) {
                let step = EsseStep {
                    u: witness.division,
                    v: witness.edge,
                };
                results.push((witness.outsplit, step));
            }
        }
    }

    // Insplits (n → n+1): A = E^T·D^T, C = D^T·E^T.
    // Step: u = E^T (edge), v = D^T (division). node = u·v, insplit = v·u.
    if node.rows + 1 <= max_dim {
        for witness in enumerate_one_step_insplits(node) {
            if witness.outsplit.trace() != source_trace {
                continue;
            }
            let canon = witness.outsplit.canonical_perm();
            if seen.insert(canon) {
                let step = EsseStep {
                    u: witness.edge,
                    v: witness.division,
                };
                results.push((witness.outsplit, step));
            }
        }
    }

    // Column amalgamation (reverse outsplit, n → n-1):
    // C has identical columns p,q. C = E·D, A = D·E.
    // Step: u = E, v = D. node(C) = u·v, amalg(A) = v·u.
    if node.rows >= 3 {
        for (amalg, division, edge) in enumerate_column_amalgamations(node) {
            if amalg.trace() != source_trace {
                continue;
            }
            let canon = amalg.canonical_perm();
            if seen.insert(canon) {
                let step = EsseStep {
                    u: edge,
                    v: division,
                };
                results.push((amalg, step));
            }
        }
    }

    // Row amalgamation (reverse insplit, n → n-1):
    // C has identical rows p,q. Via transpose: C^T has identical columns p,q.
    // C = div·edge, A = edge·div.
    // Step: u = div, v = edge. node(C) = u·v, amalg(A) = v·u.
    if node.rows >= 3 {
        for (amalg, division, edge) in enumerate_row_amalgamations(node) {
            if amalg.trace() != source_trace {
                continue;
            }
            let canon = amalg.canonical_perm();
            if seen.insert(canon) {
                let step = EsseStep {
                    u: division,
                    v: edge,
                };
                results.push((amalg, step));
            }
        }
    }

    results
}

/// Find all column amalgamations: pairs of identical columns that can be merged.
/// Returns (amalgamated_matrix, division_D, edge_E).
fn enumerate_column_amalgamations(c: &DynMatrix) -> Vec<(DynMatrix, DynMatrix, DynMatrix)> {
    let n = c.rows;
    let mut results = Vec::new();

    for p in 0..n {
        for q in (p + 1)..n {
            // Check if columns p and q are identical.
            let identical = (0..n).all(|row| c.get(row, p) == c.get(row, q));
            if !identical {
                continue;
            }

            let k = n - 1;

            // E = C with column q removed (n × k).
            let mut e_data = Vec::with_capacity(n * k);
            for row in 0..n {
                for col in 0..n {
                    if col == q {
                        continue;
                    }
                    e_data.push(c.get(row, col));
                }
            }
            let edge = DynMatrix::new(n, k, e_data);

            // D is (k × n): D[parent_of(b)][b] = 1.
            let mut d_data = vec![0u32; k * n];
            for b in 0..n {
                let parent = if b == q {
                    p
                } else if b > q {
                    b - 1
                } else {
                    b
                };
                d_data[parent * n + b] = 1;
            }
            let division = DynMatrix::new(k, n, d_data);

            // A = D · E.
            let amalg = division.mul(&edge);

            results.push((amalg, division, edge));
        }
    }

    results
}

/// Find all row amalgamations: pairs of identical rows that can be merged.
/// Implemented via transpose: row-amalgamate C = column-amalgamate C^T, then transpose back.
fn enumerate_row_amalgamations(c: &DynMatrix) -> Vec<(DynMatrix, DynMatrix, DynMatrix)> {
    let ct = c.transpose();
    enumerate_column_amalgamations(&ct)
        .into_iter()
        .map(|(amalg_t, div_t, edge_t)| {
            // For column amalgamation of C^T: C^T = edge_t · div_t, amalg_t = div_t · edge_t
            // Transposing: C = div_t^T · edge_t^T, amalg = edge_t^T · div_t^T
            // So: division = div_t^T, edge = edge_t^T
            // Step: u = division, v = edge. C = u·v, amalg = v·u.
            (amalg_t.transpose(), div_t.transpose(), edge_t.transpose())
        })
        .collect()
}

// --- Path reconstruction ---

fn reconstruct_balanced_path(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> SsePath<2> {
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices.last().unwrap().clone();
    let bwd_meeting = bwd_matrices.last().unwrap().clone();

    let mut all_steps = fwd_steps;

    // Bridge the meeting point if the two sides have different originals.
    if fwd_meeting != bwd_meeting {
        if let Some(step) = permutation_step_between(&fwd_meeting, &bwd_meeting) {
            all_steps.push(step);
        }
    }

    // Reverse backward steps.
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

    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);

    if *all_dyn_matrices.first().unwrap() != a_dyn {
        all_steps.insert(0, permutation_step_dyn(&a_dyn));
        all_dyn_matrices.insert(0, a_dyn);
    }
    if *all_dyn_matrices.last().unwrap() != b_dyn {
        let last = all_dyn_matrices.last().unwrap().clone();
        all_steps.push(permutation_step_dyn(&last));
        all_dyn_matrices.push(b_dyn);
    }

    let sq_matrices: Vec<SqMatrix<2>> = all_dyn_matrices
        .iter()
        .filter_map(|dm| dm.to_sq::<2>())
        .collect();

    SsePath {
        matrices: sq_matrices,
        steps: all_steps,
    }
}

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

fn permutation_step_dyn(m: &DynMatrix) -> EsseStep {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_column_amalgamation_basic() {
        // C with identical columns 0 and 1.
        let c = DynMatrix::new(3, 3, vec![1, 1, 2, 3, 3, 4, 5, 5, 6]);
        let results = enumerate_column_amalgamations(&c);
        assert!(!results.is_empty());
        for (amalg, div, edge) in &results {
            assert_eq!(amalg.rows, 2);
            assert_eq!(amalg.cols, 2);
            // Verify C = E · D.
            assert_eq!(edge.mul(div), c);
            // Verify A = D · E.
            assert_eq!(div.mul(edge), *amalg);
        }
    }

    #[test]
    fn test_row_amalgamation_basic() {
        // C with identical rows 0 and 1.
        let c = DynMatrix::new(3, 3, vec![1, 2, 3, 1, 2, 3, 4, 5, 6]);
        let results = enumerate_row_amalgamations(&c);
        assert!(!results.is_empty());
        for (amalg, div, edge) in &results {
            assert_eq!(amalg.rows, 2);
            assert_eq!(amalg.cols, 2);
            // Verify C = div · edge.
            assert_eq!(div.mul(edge), c);
            // Verify amalg = edge · div.
            assert_eq!(edge.mul(div), *amalg);
        }
    }

    #[test]
    fn test_outsplit_then_amalgamate_roundtrip() {
        let a = DynMatrix::from_sq(&SqMatrix::new([[1, 3], [2, 1]]));
        let outsplits = enumerate_one_step_outsplits(&a);
        assert!(!outsplits.is_empty());

        // Every outsplit should have at least one pair of identical columns,
        // and amalgamating should recover the original matrix (up to permutation).
        for witness in &outsplits {
            let c = &witness.outsplit;
            let col_amalgs = enumerate_column_amalgamations(c);
            // At least one amalgamation should recover the original.
            let a_canon = a.canonical_perm();
            let found = col_amalgs
                .iter()
                .any(|(amalg, _, _)| amalg.canonical_perm() == a_canon);
            assert!(
                found,
                "outsplit of A should be amalgamable back to A: outsplit={:?}",
                c
            );
        }
    }

    #[test]
    fn test_balanced_bfs_brix_ruiz_k3_dim3_exhausted() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = BalancedBfsConfig {
            max_intermediate_dim: 3,
            max_layers: 20,
            max_frontier_size: 50_000,
            time_limit: Duration::from_secs(5),
        };
        assert!(matches!(
            balanced_sse_bfs_2x2(&a, &b, &config),
            BalancedBfsResult::Exhausted
        ));
    }

    #[test]
    fn test_balanced_bfs_trivial_pair() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let config = BalancedBfsConfig {
            max_intermediate_dim: 4,
            max_layers: 10,
            max_frontier_size: 10_000,
            time_limit: Duration::from_secs(5),
        };
        match balanced_sse_bfs_2x2(&a, &b, &config) {
            BalancedBfsResult::Equivalent(path) => {
                assert!(!path.steps.is_empty());
            }
            BalancedBfsResult::Exhausted => {
                panic!("should find equivalence for this trivial pair");
            }
        }
    }
}
