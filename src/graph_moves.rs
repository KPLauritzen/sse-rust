use crate::matrix::{DynMatrix, SqMatrix};

/// One explicit 2x2 -> 3x3 out-split witness.
///
/// The matrices satisfy `A = D E` and `C = E D`, where `D` is a division matrix.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OutsplitWitness2x2To3x3 {
    pub division: DynMatrix,
    pub edge: DynMatrix,
    pub outsplit: DynMatrix,
}

/// Enumerate all one-step 2x2 -> 3x3 out-splits of a 2x2 nonnegative matrix.
pub fn enumerate_outsplits_2x2_to_3x3(a: &SqMatrix<2>) -> Vec<OutsplitWitness2x2To3x3> {
    let mut witnesses = Vec::new();

    for assignment in child_parent_assignments_3_over_2() {
        let division = division_matrix_from_assignment(&assignment);
        let row0_children: Vec<usize> = assignment
            .iter()
            .enumerate()
            .filter_map(|(idx, &parent)| (parent == 0).then_some(idx))
            .collect();
        let row1_children: Vec<usize> = assignment
            .iter()
            .enumerate()
            .filter_map(|(idx, &parent)| (parent == 1).then_some(idx))
            .collect();

        let row0_splits = split_row_2_into_children(a.data[0], row0_children.len());
        let row1_splits = split_row_2_into_children(a.data[1], row1_children.len());

        for split0 in &row0_splits {
            for split1 in &row1_splits {
                let mut rows = [[0u32; 2]; 3];
                for (child_idx, row) in row0_children.iter().copied().zip(split0.iter().copied()) {
                    rows[child_idx] = row;
                }
                for (child_idx, row) in row1_children.iter().copied().zip(split1.iter().copied()) {
                    rows[child_idx] = row;
                }

                let edge = DynMatrix::new(
                    3,
                    2,
                    rows.iter().flat_map(|row| row.iter()).copied().collect(),
                );
                let outsplit = edge.mul(&division);
                debug_assert_eq!(division.mul(&edge), DynMatrix::from_sq(a));

                witnesses.push(OutsplitWitness2x2To3x3 {
                    division: division.clone(),
                    edge,
                    outsplit,
                });
            }
        }
    }

    witnesses
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

fn child_parent_assignments_3_over_2() -> Vec<[usize; 3]> {
    let mut assignments = Vec::new();
    for a0 in 0..=1 {
        for a1 in 0..=1 {
            for a2 in 0..=1 {
                let assignment = [a0, a1, a2];
                if assignment.contains(&0) && assignment.contains(&1) {
                    assignments.push(assignment);
                }
            }
        }
    }
    assignments
}

fn division_matrix_from_assignment(assignment: &[usize; 3]) -> DynMatrix {
    let mut data = vec![0u32; 6];
    for (child, &parent) in assignment.iter().enumerate() {
        data[parent * 3 + child] = 1;
    }
    DynMatrix::new(2, 3, data)
}

fn split_row_2_into_children(row: [u32; 2], children: usize) -> Vec<Vec<[u32; 2]>> {
    if children == 0 {
        return Vec::new();
    }
    let mut results = Vec::new();
    let mut current = vec![[0u32; 2]; children];
    recurse_split_row_2_into_children(0, row, &mut current, &mut results);
    results
}

fn recurse_split_row_2_into_children(
    idx: usize,
    remaining: [u32; 2],
    current: &mut [[u32; 2]],
    results: &mut Vec<Vec<[u32; 2]>>,
) {
    if idx + 1 == current.len() {
        current[idx] = remaining;
        results.push(current.to_vec());
        return;
    }

    for left0 in 0..=remaining[0] {
        for left1 in 0..=remaining[1] {
            current[idx] = [left0, left1];
            recurse_split_row_2_into_children(
                idx + 1,
                [remaining[0] - left0, remaining[1] - left1],
                current,
                results,
            );
        }
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
}
