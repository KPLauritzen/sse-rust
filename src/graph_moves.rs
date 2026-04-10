use std::collections::{HashMap, HashSet};

use crate::factorisation::enumerate_factorisations_3x3_to_2;
use crate::matrix::{DynMatrix, SqMatrix};

/// One explicit one-step out-split witness.
///
/// The matrices satisfy `A = D E` and `C = E D`, where `D` is a division matrix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutsplitWitness {
    pub division: DynMatrix,
    pub edge: DynMatrix,
    pub outsplit: DynMatrix,
}

pub type OutsplitWitness2x2To3x3 = OutsplitWitness;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMoveSuccessor {
    pub family: &'static str,
    pub matrix: DynMatrix,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GraphMoveSuccessors {
    pub candidates: usize,
    pub nodes: Vec<GraphMoveSuccessor>,
}

/// Two successive out-splits starting from a 2x2 matrix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TwoStepOutsplitChain2x2 {
    pub first: OutsplitWitness,
    pub second: OutsplitWitness,
}

/// Enumerate all one-step out-splits of a square nonnegative matrix.
pub fn enumerate_one_step_outsplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    assert!(a.is_square());

    let parent_count = a.rows;
    let child_count = parent_count + 1;
    let assignments = child_parent_assignments(child_count, parent_count);
    let parent_rows: Vec<Vec<u32>> = (0..parent_count)
        .map(|row| (0..parent_count).map(|col| a.get(row, col)).collect())
        .collect();

    let mut witnesses = Vec::new();
    for assignment in assignments {
        let division = division_matrix_from_assignment(&assignment, parent_count);
        let child_rows_by_parent = children_by_parent(&assignment, parent_count);
        let split_options_by_parent: Vec<Vec<Vec<Vec<u32>>>> = parent_rows
            .iter()
            .zip(&child_rows_by_parent)
            .map(|(row, children)| split_row_into_children(row, children.len()))
            .collect();
        let mut child_rows = vec![vec![0u32; parent_count]; child_count];
        recurse_outsplit_rows(
            0,
            &child_rows_by_parent,
            &split_options_by_parent,
            &mut child_rows,
            &division,
            a,
            &mut witnesses,
        );
    }

    witnesses
}

/// Enumerate all one-step 2x2 -> 3x3 out-splits of a 2x2 nonnegative matrix.
pub fn enumerate_outsplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_one_step_outsplits(&DynMatrix::from_sq(a))
}

/// Enumerate all one-step in-splits of a square nonnegative matrix.
///
/// This is implemented as out-splitting the transpose and transposing back.
pub fn enumerate_one_step_insplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_one_step_outsplits(&a.transpose())
        .into_iter()
        .map(|witness| OutsplitWitness {
            division: witness.division.transpose(),
            edge: witness.edge.transpose(),
            outsplit: witness.outsplit.transpose(),
        })
        .collect()
}

/// Enumerate all one-step 2x2 -> 3x3 in-splits of a 2x2 nonnegative matrix.
pub fn enumerate_insplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_one_step_insplits(&DynMatrix::from_sq(a))
}

/// Enumerate one-step in-splits whose output has two vertices with the same future,
/// i.e. two equal rows in the refined adjacency matrix.
pub fn enumerate_same_future_insplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_one_step_insplits(a)
        .into_iter()
        .filter(|witness| has_duplicate_rows(&witness.outsplit))
        .collect()
}

/// Enumerate all one-step 2x2 -> 3x3 same-future in-splits.
pub fn enumerate_same_future_insplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_same_future_insplits(&DynMatrix::from_sq(a))
}

/// Enumerate one-step out-splits whose output has two vertices with the same past,
/// i.e. two equal columns in the refined adjacency matrix.
pub fn enumerate_same_past_outsplits(a: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_one_step_outsplits(a)
        .into_iter()
        .filter(|witness| has_duplicate_columns(&witness.outsplit))
        .collect()
}

/// Enumerate all one-step 2x2 -> 3x3 same-past out-splits.
pub fn enumerate_same_past_outsplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    enumerate_same_past_outsplits(&DynMatrix::from_sq(a))
}

/// Enumerate all canonical one-step split refinements, allowing either split direction.
pub fn enumerate_one_step_split_refinements(a: &DynMatrix) -> Vec<DynMatrix> {
    let mut seen = HashSet::new();
    let mut refinements = Vec::new();

    for witness in enumerate_one_step_outsplits(a)
        .into_iter()
        .chain(enumerate_one_step_insplits(a).into_iter())
    {
        let canon = witness.outsplit.canonical_perm();
        if seen.insert(canon.clone()) {
            refinements.push(canon);
        }
    }

    refinements
}

/// Enumerate canonical successors reached by one graph split or amalgamation.
pub fn enumerate_graph_move_successors(current: &DynMatrix, max_dim: usize) -> GraphMoveSuccessors {
    assert!(current.is_square());

    let mut candidates = 0usize;
    let mut seen = HashSet::new();
    let mut nodes = Vec::new();

    if current.rows < max_dim {
        append_representative_outsplit_successors(
            current,
            "outsplit",
            false,
            &mut candidates,
            &mut seen,
            &mut nodes,
        );

        append_representative_outsplit_successors(
            &current.transpose(),
            "insplit",
            true,
            &mut candidates,
            &mut seen,
            &mut nodes,
        );
    }

    if current.rows > 2 {
        for witness in enumerate_out_amalgamations(current) {
            candidates += 1;
            push_canonical_graph_successor(
                "out_amalgamation",
                witness.outsplit,
                &mut seen,
                &mut nodes,
            );
        }
        for witness in enumerate_in_amalgamations(current) {
            candidates += 1;
            push_canonical_graph_successor(
                "in_amalgamation",
                witness.outsplit,
                &mut seen,
                &mut nodes,
            );
        }
    }

    GraphMoveSuccessors { candidates, nodes }
}

fn append_representative_outsplit_successors(
    a: &DynMatrix,
    family: &'static str,
    transpose_result: bool,
    candidates: &mut usize,
    seen: &mut HashSet<DynMatrix>,
    nodes: &mut Vec<GraphMoveSuccessor>,
) {
    debug_assert!(a.is_square());

    let parent_count = a.rows;
    let child_count = parent_count + 1;
    let parent_rows: Vec<Vec<u32>> = (0..parent_count)
        .map(|row| (0..parent_count).map(|col| a.get(row, col)).collect())
        .collect();

    for split_parent in 0..parent_count {
        let assignment: Vec<usize> = (0..child_count)
            .map(|child| {
                if child < parent_count {
                    child
                } else {
                    split_parent
                }
            })
            .collect();

        for split in split_row_into_children(&parent_rows[split_parent], 2) {
            // The two split children are interchangeable up to permutation.
            if split[1] < split[0] {
                continue;
            }

            *candidates += 1;

            let mut child_rows = parent_rows.clone();
            child_rows[split_parent] = split[0].clone();
            child_rows.push(split[1].clone());

            let mut data = Vec::with_capacity(child_count * child_count);
            for row in &child_rows {
                for &parent in &assignment {
                    data.push(row[parent]);
                }
            }

            let mut outsplit = DynMatrix::new(child_count, child_count, data);
            if transpose_result {
                outsplit = outsplit.transpose();
            }
            push_canonical_graph_successor(family, outsplit, seen, nodes);
        }
    }
}

fn push_canonical_graph_successor(
    family: &'static str,
    matrix: DynMatrix,
    seen: &mut HashSet<DynMatrix>,
    nodes: &mut Vec<GraphMoveSuccessor>,
) {
    let canon = matrix.canonical_perm();
    if seen.insert(canon.clone()) {
        nodes.push(GraphMoveSuccessor {
            family,
            matrix: canon,
        });
    }
}

/// Search for a common one-step 3x3 out-split refinement up to permutation.
pub fn find_common_outsplit_refinement_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
) -> Option<(OutsplitWitness2x2To3x3, OutsplitWitness2x2To3x3)> {
    let a_outsplits = enumerate_outsplits_2x2_to_3x3(a);
    let b_outsplits = enumerate_outsplits_2x2_to_3x3(b);

    for left in &a_outsplits {
        let left_canon = left.outsplit.canonical_perm();
        for right in &b_outsplits {
            if left_canon == right.outsplit.canonical_perm() {
                return Some((left.clone(), right.clone()));
            }
        }
    }

    None
}

/// Search for a common two-step 4x4 out-split refinement up to permutation.
pub fn find_common_two_step_outsplit_refinement_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
) -> Option<(TwoStepOutsplitChain2x2, TwoStepOutsplitChain2x2)> {
    let mut left_refinements = HashMap::new();
    for first in enumerate_outsplits_2x2_to_3x3(a) {
        for second in enumerate_one_step_outsplits(&first.outsplit) {
            left_refinements
                .entry(second.outsplit.canonical_perm())
                .or_insert_with(|| TwoStepOutsplitChain2x2 {
                    first: first.clone(),
                    second,
                });
        }
    }

    for first in enumerate_outsplits_2x2_to_3x3(b) {
        for second in enumerate_one_step_outsplits(&first.outsplit) {
            let canon = second.outsplit.canonical_perm();
            if let Some(left) = left_refinements.get(&canon) {
                return Some((
                    left.clone(),
                    TwoStepOutsplitChain2x2 {
                        first: first.clone(),
                        second,
                    },
                ));
            }
        }
    }

    None
}

/// Enumerate canonical 3x3 neighbors reached by one `3x3 -> 2x2 -> 3x3` zig-zag.
pub fn enumerate_3x3_outsplit_zigzag_neighbors(
    c: &DynMatrix,
    max_bridge_entry: u32,
) -> Vec<DynMatrix> {
    assert_eq!(c.rows, 3);
    assert_eq!(c.cols, 3);

    let mut seen = HashSet::new();
    let mut neighbors = Vec::new();

    for (u, v) in enumerate_factorisations_3x3_to_2(c, max_bridge_entry) {
        let bridge = v
            .mul(&u)
            .to_sq::<2>()
            .expect("3x3-to-2 factorisation should produce a 2x2 bridge")
            .canonical();
        for witness in enumerate_outsplits_2x2_to_3x3(&bridge) {
            let canon = witness.outsplit.canonical_perm();
            if seen.insert(canon.clone()) {
                neighbors.push(canon);
            }
        }
    }

    neighbors
}

fn child_parent_assignments(child_count: usize, parent_count: usize) -> Vec<Vec<usize>> {
    let mut assignments = Vec::new();
    let mut current = vec![0usize; child_count];
    recurse_child_parent_assignments(0, child_count, parent_count, &mut current, &mut assignments);
    assignments
}

fn has_duplicate_rows(a: &DynMatrix) -> bool {
    for i in 0..a.rows {
        for j in i + 1..a.rows {
            let equal = (0..a.cols).all(|col| a.get(i, col) == a.get(j, col));
            if equal {
                return true;
            }
        }
    }
    false
}

fn has_duplicate_columns(a: &DynMatrix) -> bool {
    for i in 0..a.cols {
        for j in i + 1..a.cols {
            let equal = (0..a.rows).all(|row| a.get(row, i) == a.get(row, j));
            if equal {
                return true;
            }
        }
    }
    false
}

fn recurse_child_parent_assignments(
    idx: usize,
    child_count: usize,
    parent_count: usize,
    current: &mut [usize],
    assignments: &mut Vec<Vec<usize>>,
) {
    if idx == child_count {
        let mut seen = vec![false; parent_count];
        for &parent in current.iter() {
            seen[parent] = true;
        }
        if seen.iter().all(|used| *used) {
            assignments.push(current.to_vec());
        }
        return;
    }

    for parent in 0..parent_count {
        current[idx] = parent;
        recurse_child_parent_assignments(idx + 1, child_count, parent_count, current, assignments);
    }
}

fn division_matrix_from_assignment(assignment: &[usize], parent_count: usize) -> DynMatrix {
    let child_count = assignment.len();
    let mut data = vec![0u32; parent_count * child_count];
    for (child, &parent) in assignment.iter().enumerate() {
        data[parent * child_count + child] = 1;
    }
    DynMatrix::new(parent_count, child_count, data)
}

fn children_by_parent(assignment: &[usize], parent_count: usize) -> Vec<Vec<usize>> {
    let mut children = vec![Vec::new(); parent_count];
    for (child, &parent) in assignment.iter().enumerate() {
        children[parent].push(child);
    }
    children
}

fn split_row_into_children(row: &[u32], children: usize) -> Vec<Vec<Vec<u32>>> {
    if children == 0 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut current = vec![vec![0u32; row.len()]; children];
    recurse_split_row_columns(0, row, &mut current, &mut results);
    results
}

fn recurse_split_row_columns(
    col_idx: usize,
    row: &[u32],
    current: &mut [Vec<u32>],
    results: &mut Vec<Vec<Vec<u32>>>,
) {
    if col_idx == row.len() {
        results.push(current.to_vec());
        return;
    }

    let compositions = compositions(row[col_idx], current.len());
    for composition in compositions {
        for (child_idx, value) in composition.into_iter().enumerate() {
            current[child_idx][col_idx] = value;
        }
        recurse_split_row_columns(col_idx + 1, row, current, results);
    }
}

fn compositions(total: u32, parts: usize) -> Vec<Vec<u32>> {
    if parts == 0 {
        return Vec::new();
    }

    let mut results = Vec::new();
    let mut current = vec![0u32; parts];
    recurse_compositions(0, total, &mut current, &mut results);
    results
}

fn recurse_compositions(
    idx: usize,
    remaining: u32,
    current: &mut [u32],
    results: &mut Vec<Vec<u32>>,
) {
    if idx + 1 == current.len() {
        current[idx] = remaining;
        results.push(current.to_vec());
        return;
    }

    for value in 0..=remaining {
        current[idx] = value;
        recurse_compositions(idx + 1, remaining - value, current, results);
    }
}

fn recurse_outsplit_rows(
    parent_idx: usize,
    child_rows_by_parent: &[Vec<usize>],
    split_options_by_parent: &[Vec<Vec<Vec<u32>>>],
    child_rows: &mut [Vec<u32>],
    division: &DynMatrix,
    a: &DynMatrix,
    witnesses: &mut Vec<OutsplitWitness>,
) {
    if parent_idx == child_rows_by_parent.len() {
        let edge = DynMatrix::new(
            child_rows.len(),
            a.cols,
            child_rows
                .iter()
                .flat_map(|row| row.iter())
                .copied()
                .collect(),
        );
        let outsplit = edge.mul(division);
        debug_assert_eq!(division.mul(&edge), *a);
        witnesses.push(OutsplitWitness {
            division: division.clone(),
            edge,
            outsplit,
        });
        return;
    }

    for split in &split_options_by_parent[parent_idx] {
        for (child_idx, row) in child_rows_by_parent[parent_idx]
            .iter()
            .copied()
            .zip(split.iter())
        {
            child_rows[child_idx].clone_from(row);
        }
        recurse_outsplit_rows(
            parent_idx + 1,
            child_rows_by_parent,
            split_options_by_parent,
            child_rows,
            division,
            a,
            witnesses,
        );
    }
}

/// Enumerate all one-step out-amalgamations of a square matrix.
///
/// An out-amalgamation merges two states whose columns are identical
/// ("same past"). This is the reverse of an out-split. For each pair of
/// identical columns (p, q), the merged matrix has dimension m-1.
///
/// The SSE step satisfies C = E·D and result = D·E, where D is a 0-1
/// division matrix.
pub fn enumerate_out_amalgamations(c: &DynMatrix) -> Vec<OutsplitWitness> {
    assert!(c.is_square());
    let m = c.rows;
    if m <= 1 {
        return vec![];
    }

    let mut witnesses = Vec::new();

    for p in 0..m {
        for q in (p + 1)..m {
            // Check if columns p and q are identical.
            let cols_equal = (0..m).all(|row| c.get(row, p) == c.get(row, q));
            if !cols_equal {
                continue;
            }

            let n = m - 1;

            // Build mapping: old state k -> new state f[k].
            // State q merges into state p; states after q shift down by 1.
            let f: Vec<usize> = (0..m)
                .map(|k| {
                    if k < q {
                        k
                    } else if k == q {
                        p
                    } else {
                        k - 1
                    }
                })
                .collect();

            // D (n×m): division matrix, D[f[k], k] = 1.
            let mut d_data = vec![0u32; n * m];
            for k in 0..m {
                d_data[f[k] * m + k] = 1;
            }
            let division = DynMatrix::new(n, m, d_data);

            // E (m×n): E[i, g] = C[i, representative(g)].
            // Representative: for group g, pick the first old state mapping to g.
            let mut e_data = vec![0u32; m * n];
            for i in 0..m {
                for g in 0..n {
                    let repr = if g < q { g } else { g + 1 };
                    e_data[i * n + g] = c.get(i, repr);
                }
            }
            let edge = DynMatrix::new(m, n, e_data);

            debug_assert_eq!(edge.mul(&division), *c);

            let result = division.mul(&edge);

            witnesses.push(OutsplitWitness {
                division,
                edge,
                outsplit: result,
            });
        }
    }

    witnesses
}

/// Enumerate all one-step in-amalgamations of a square matrix.
///
/// An in-amalgamation merges two states whose rows are identical
/// ("same future"). This is the reverse of an in-split.
/// Implemented by transposing, out-amalgamating, and transposing back.
pub fn enumerate_in_amalgamations(c: &DynMatrix) -> Vec<OutsplitWitness> {
    enumerate_out_amalgamations(&c.transpose())
        .into_iter()
        .map(|witness| OutsplitWitness {
            division: witness.division.transpose(),
            edge: witness.edge.transpose(),
            outsplit: witness.outsplit.transpose(),
        })
        .collect()
}

#[doc(hidden)]
pub mod profiling_helpers {
    pub fn split_row_into_children(row: &[u32], children: usize) -> Vec<Vec<Vec<u32>>> {
        super::split_row_into_children(row, children)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enumerate_outsplits_2x2_to_3x3_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let witnesses = enumerate_outsplits_2x2_to_3x3(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.division.rows, 2);
            assert_eq!(witness.division.cols, 3);
            assert_eq!(witness.edge.rows, 3);
            assert_eq!(witness.edge.cols, 2);
            assert_eq!(witness.outsplit.rows, 3);
            assert_eq!(witness.outsplit.cols, 3);
            assert_eq!(witness.division.mul(&witness.edge), DynMatrix::from_sq(&a));
        }
    }

    #[test]
    fn test_enumerate_one_step_outsplits_3x3_nonempty() {
        let a = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 3, 2, 0, 1]);
        let witnesses = enumerate_one_step_outsplits(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.division.rows, 3);
            assert_eq!(witness.division.cols, 4);
            assert_eq!(witness.edge.rows, 4);
            assert_eq!(witness.edge.cols, 3);
            assert_eq!(witness.outsplit.rows, 4);
            assert_eq!(witness.outsplit.cols, 4);
            assert_eq!(witness.division.mul(&witness.edge), a);
        }
    }

    #[test]
    fn test_enumerate_one_step_insplits_3x3_nonempty() {
        let a = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 3, 2, 0, 1]);
        let witnesses = enumerate_one_step_insplits(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.edge.rows, 3);
            assert_eq!(witness.edge.cols, 4);
            assert_eq!(witness.division.rows, 4);
            assert_eq!(witness.division.cols, 3);
            assert_eq!(witness.outsplit.rows, 4);
            assert_eq!(witness.outsplit.cols, 4);
            assert_eq!(witness.edge.mul(&witness.division), a);
        }
    }

    #[test]
    fn test_representative_outsplit_successors_match_full_enumeration() {
        let matrices = [
            DynMatrix::from_sq(&SqMatrix::new([[1, 3], [2, 1]])),
            DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 3, 2, 0, 1]),
        ];

        for a in matrices {
            let expected = enumerate_one_step_outsplits(&a)
                .into_iter()
                .map(|witness| witness.outsplit.canonical_perm())
                .collect::<HashSet<_>>();
            let mut seen = HashSet::new();
            let mut nodes = Vec::new();
            let mut candidates = 0;
            append_representative_outsplit_successors(
                &a,
                "outsplit",
                false,
                &mut candidates,
                &mut seen,
                &mut nodes,
            );
            let actual = nodes
                .into_iter()
                .map(|node| node.matrix)
                .collect::<HashSet<_>>();
            assert_eq!(actual, expected);
            assert!(candidates <= expected.len() * 2);
        }
    }

    #[test]
    fn test_enumerate_same_future_insplits_2x2_to_3x3_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let witnesses = enumerate_same_future_insplits_2x2_to_3x3(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.edge.mul(&witness.division), DynMatrix::from_sq(&a));
            assert!(has_duplicate_rows(&witness.outsplit));
        }
    }

    #[test]
    fn test_enumerate_same_past_outsplits_2x2_to_3x3_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let witnesses = enumerate_same_past_outsplits_2x2_to_3x3(&a);
        assert!(!witnesses.is_empty());
        for witness in &witnesses {
            assert_eq!(witness.division.mul(&witness.edge), DynMatrix::from_sq(&a));
            assert!(has_duplicate_columns(&witness.outsplit));
        }
    }

    #[test]
    fn test_brix_ruiz_k3_has_no_common_one_step_outsplit_refinement() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        assert!(find_common_outsplit_refinement_2x2(&a, &b).is_none());
    }

    #[test]
    fn test_brix_ruiz_k4_has_no_common_one_step_outsplit_refinement() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        assert!(find_common_outsplit_refinement_2x2(&a, &b).is_none());
    }

    #[test]
    fn test_enumerate_3x3_outsplit_zigzag_neighbors_nonempty() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let first = enumerate_outsplits_2x2_to_3x3(&a);
        let neighbors = enumerate_3x3_outsplit_zigzag_neighbors(&first[0].outsplit, 8);
        assert!(!neighbors.is_empty());
        assert!(neighbors.iter().all(|m| m.rows == 3 && m.cols == 3));
    }

    #[test]
    fn test_out_amalgamation_roundtrips_same_past_outsplit() {
        // A same-past outsplit produces duplicate columns, so its output
        // should be out-amalgamable back to the original matrix.
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let outsplits = enumerate_same_past_outsplits_2x2_to_3x3(&a);
        assert!(!outsplits.is_empty());

        for outsplit_witness in &outsplits {
            let c = &outsplit_witness.outsplit;
            assert!(has_duplicate_columns(c));

            let amalgamations = enumerate_out_amalgamations(c);
            assert!(
                !amalgamations.is_empty(),
                "Same-past outsplit should be out-amalgamable"
            );

            // At least one amalgamation should recover the original matrix
            // (up to permutation).
            let a_dyn = DynMatrix::from_sq(&a);
            let a_canon = a_dyn.canonical_perm();
            let recovered = amalgamations
                .iter()
                .any(|w| w.outsplit.canonical_perm() == a_canon);
            assert!(recovered, "Out-amalgamation should recover original matrix");
        }
    }

    #[test]
    fn test_in_amalgamation_roundtrips_same_future_insplit() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let insplits = enumerate_same_future_insplits_2x2_to_3x3(&a);
        assert!(!insplits.is_empty());

        for insplit_witness in &insplits {
            let c = &insplit_witness.outsplit;
            assert!(has_duplicate_rows(c));

            let amalgamations = enumerate_in_amalgamations(c);
            assert!(
                !amalgamations.is_empty(),
                "Same-future insplit should be in-amalgamable"
            );

            let a_dyn = DynMatrix::from_sq(&a);
            let a_canon = a_dyn.canonical_perm();
            let recovered = amalgamations
                .iter()
                .any(|w| w.outsplit.canonical_perm() == a_canon);
            assert!(recovered, "In-amalgamation should recover original matrix");
        }
    }

    #[test]
    fn test_out_amalgamation_of_4x4_outsplit() {
        // Out-split a 3x3 to 4x4, then out-amalgamate back.
        let a3 = DynMatrix::new(3, 3, vec![1, 2, 2, 2, 1, 1, 1, 0, 0]);
        let outsplits = enumerate_same_past_outsplits(&a3);
        if outsplits.is_empty() {
            return; // skip if no same-past outsplits
        }

        for witness in &outsplits {
            let c4 = &witness.outsplit;
            assert_eq!(c4.rows, 4);
            let amalgamations = enumerate_out_amalgamations(c4);
            assert!(!amalgamations.is_empty());

            let a3_canon = a3.canonical_perm();
            let recovered = amalgamations
                .iter()
                .any(|w| w.outsplit.canonical_perm() == a3_canon);
            assert!(recovered, "4x4 out-amalgamation should recover the 3x3");
        }
    }

    #[test]
    fn test_amalgamation_sse_step_valid() {
        // Verify the SSE step: C = E·D and result = D·E.
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let outsplits = enumerate_same_past_outsplits_2x2_to_3x3(&a);
        for outsplit_w in &outsplits {
            let c = &outsplit_w.outsplit;
            for amal_w in &enumerate_out_amalgamations(c) {
                assert_eq!(amal_w.edge.mul(&amal_w.division), *c, "C should equal E·D");
                assert_eq!(
                    amal_w.division.mul(&amal_w.edge),
                    amal_w.outsplit,
                    "result should equal D·E"
                );
            }
        }
    }
}
