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
    recurse_child_parent_assignments(
        0,
        child_count,
        parent_count,
        &mut current,
        &mut assignments,
    );
    assignments
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
}
