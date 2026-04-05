use crate::matrix::{DynMatrix, SqMatrix};

/// A balanced elementary equivalence witness for a pair of 2x2 matrices.
///
/// The matrices satisfy
/// `A = S R_A`, `B = S R_B`, and `R_A S = R_B S`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedElementaryWitness2x2 {
    pub s: DynMatrix,
    pub r_a: DynMatrix,
    pub r_b: DynMatrix,
}

/// Configuration for bounded balanced-elementary search on 2x2 matrices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BalancedSearchConfig2x2 {
    /// Maximum common intermediate size `m` in `S (2xm)` and `R_A, R_B (mx2)`.
    pub max_common_dim: usize,
    /// Maximum entry allowed in `S`, `R_A`, and `R_B`.
    pub max_entry: u32,
}

impl Default for BalancedSearchConfig2x2 {
    fn default() -> Self {
        Self {
            max_common_dim: 2,
            max_entry: 10,
        }
    }
}

/// Search result for bounded balanced-elementary search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BalancedSearchResult2x2 {
    Equivalent(BalancedElementaryWitness2x2),
    Exhausted,
}

/// Verify the balanced-elementary equations for a proposed witness.
pub fn verify_balanced_elementary_witness_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &BalancedElementaryWitness2x2,
) -> Result<(), String> {
    if witness.s.rows != 2 {
        return Err("S must have 2 rows".into());
    }
    if witness.r_a.rows != witness.s.cols || witness.r_b.rows != witness.s.cols {
        return Err("R_A and R_B must have as many rows as S has columns".into());
    }
    if witness.r_a.cols != 2 || witness.r_b.cols != 2 {
        return Err("R_A and R_B must have 2 columns".into());
    }

    let lhs_a = witness.s.mul(&witness.r_a);
    let lhs_b = witness.s.mul(&witness.r_b);
    if lhs_a != DynMatrix::from_sq(a) {
        return Err(format!(
            "A != S R_A: {:?} vs {:?}",
            DynMatrix::from_sq(a),
            lhs_a
        ));
    }
    if lhs_b != DynMatrix::from_sq(b) {
        return Err(format!(
            "B != S R_B: {:?} vs {:?}",
            DynMatrix::from_sq(b),
            lhs_b
        ));
    }

    let ras = witness.r_a.mul(&witness.s);
    let rbs = witness.r_b.mul(&witness.s);
    if ras != rbs {
        return Err(format!("R_A S != R_B S: {:?} vs {:?}", ras, rbs));
    }

    Ok(())
}

/// Search for a bounded balanced-elementary equivalence witness between two 2x2 matrices.
pub fn find_balanced_elementary_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &BalancedSearchConfig2x2,
) -> BalancedSearchResult2x2 {
    for common_dim in 1..=config.max_common_dim {
        if let Some(witness) = find_balanced_elementary_equivalence_with_common_dim_2x2(
            a,
            b,
            common_dim,
            config.max_entry,
        ) {
            return BalancedSearchResult2x2::Equivalent(witness);
        }
    }

    BalancedSearchResult2x2::Exhausted
}

/// Search for a balanced-elementary equivalence witness with a fixed common dimension.
pub fn find_balanced_elementary_equivalence_with_common_dim_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    common_dim: usize,
    max_entry: u32,
) -> Option<BalancedElementaryWitness2x2> {
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);

    for s_data in enumerate_matrix_data(2 * common_dim, max_entry) {
        let s = DynMatrix::new(2, common_dim, s_data);
        let r_a_candidates = enumerate_right_factorisations_2x2(&s, &a_dyn, max_entry);
        if r_a_candidates.is_empty() {
            continue;
        }
        let r_b_candidates = enumerate_right_factorisations_2x2(&s, &b_dyn, max_entry);
        if r_b_candidates.is_empty() {
            continue;
        }

        for r_a in &r_a_candidates {
            let ras = r_a.mul(&s);
            for r_b in &r_b_candidates {
                if ras == r_b.mul(&s) {
                    let witness = BalancedElementaryWitness2x2 {
                        s: s.clone(),
                        r_a: r_a.clone(),
                        r_b: r_b.clone(),
                    };
                    debug_assert!(verify_balanced_elementary_witness_2x2(a, b, &witness).is_ok());
                    return Some(witness);
                }
            }
        }
    }

    None
}

fn enumerate_right_factorisations_2x2(
    s: &DynMatrix,
    target: &DynMatrix,
    max_entry: u32,
) -> Vec<DynMatrix> {
    debug_assert_eq!(s.rows, 2);
    debug_assert_eq!(target.rows, 2);
    debug_assert_eq!(target.cols, 2);

    let col0 = enumerate_column_solutions_2xm(s, [target.get(0, 0), target.get(1, 0)], max_entry);
    if col0.is_empty() {
        return Vec::new();
    }

    let col1 = enumerate_column_solutions_2xm(s, [target.get(0, 1), target.get(1, 1)], max_entry);
    if col1.is_empty() {
        return Vec::new();
    }

    let mut factors = Vec::new();
    for left in &col0 {
        for right in &col1 {
            let mut data = Vec::with_capacity(2 * s.cols);
            for row in 0..s.cols {
                data.push(left[row]);
                data.push(right[row]);
            }
            factors.push(DynMatrix::new(s.cols, 2, data));
        }
    }
    factors
}

fn enumerate_column_solutions_2xm(
    s: &DynMatrix,
    target: [u32; 2],
    max_entry: u32,
) -> Vec<Vec<u32>> {
    let mut solutions = Vec::new();
    let mut current = vec![0u32; s.cols];
    recurse_column_solution(0, s, target, max_entry, &mut current, &mut solutions);
    solutions
}

fn recurse_column_solution(
    idx: usize,
    s: &DynMatrix,
    target: [u32; 2],
    max_entry: u32,
    current: &mut [u32],
    solutions: &mut Vec<Vec<u32>>,
) {
    if idx == s.cols {
        let sum0: u32 = (0..s.cols).map(|j| s.get(0, j) * current[j]).sum();
        let sum1: u32 = (0..s.cols).map(|j| s.get(1, j) * current[j]).sum();
        if sum0 == target[0] && sum1 == target[1] {
            solutions.push(current.to_vec());
        }
        return;
    }

    for value in 0..=max_entry {
        current[idx] = value;

        let partial0: u32 = (0..=idx).map(|j| s.get(0, j) * current[j]).sum();
        let partial1: u32 = (0..=idx).map(|j| s.get(1, j) * current[j]).sum();
        if partial0 > target[0] || partial1 > target[1] {
            continue;
        }

        recurse_column_solution(idx + 1, s, target, max_entry, current, solutions);
    }
}

fn enumerate_matrix_data(len: usize, max_entry: u32) -> MatrixDataIter {
    MatrixDataIter::new(len, max_entry)
}

struct MatrixDataIter {
    data: Vec<u32>,
    max_entry: u32,
    done: bool,
}

impl MatrixDataIter {
    fn new(len: usize, max_entry: u32) -> Self {
        Self {
            data: vec![0; len],
            max_entry,
            done: false,
        }
    }
}

impl Iterator for MatrixDataIter {
    type Item = Vec<u32>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.done {
            return None;
        }

        let current = self.data.clone();
        for idx in (0..self.data.len()).rev() {
            if self.data[idx] < self.max_entry {
                self.data[idx] += 1;
                for reset_idx in idx + 1..self.data.len() {
                    self.data[reset_idx] = 0;
                }
                return Some(current);
            }
        }

        self.done = true;
        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_balanced_elementary_positive_example() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let witness = BalancedElementaryWitness2x2 {
            s: DynMatrix::new(2, 1, vec![1, 1]),
            r_a: DynMatrix::new(1, 2, vec![1, 0]),
            r_b: DynMatrix::new(1, 2, vec![0, 1]),
        };
        assert!(verify_balanced_elementary_witness_2x2(&a, &b, &witness).is_ok());
    }

    #[test]
    fn test_search_balanced_elementary_positive_example() {
        let a = SqMatrix::new([[1, 0], [1, 0]]);
        let b = SqMatrix::new([[0, 1], [0, 1]]);
        let result = find_balanced_elementary_equivalence_2x2(
            &a,
            &b,
            &BalancedSearchConfig2x2 {
                max_common_dim: 1,
                max_entry: 1,
            },
        );
        match result {
            BalancedSearchResult2x2::Equivalent(witness) => {
                assert!(verify_balanced_elementary_witness_2x2(&a, &b, &witness).is_ok());
            }
            BalancedSearchResult2x2::Exhausted => panic!("expected a balanced witness"),
        }
    }

    #[test]
    fn test_brix_ruiz_k3_balanced_elementary_exhausted() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let result = find_balanced_elementary_equivalence_2x2(
            &a,
            &b,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );
        assert_eq!(result, BalancedSearchResult2x2::Exhausted);
    }

    #[test]
    fn test_brix_ruiz_k4_balanced_elementary_exhausted() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        let result = find_balanced_elementary_equivalence_2x2(
            &a,
            &b,
            &BalancedSearchConfig2x2 {
                max_common_dim: 2,
                max_entry: 8,
            },
        );
        assert_eq!(result, BalancedSearchResult2x2::Exhausted);
    }
}
