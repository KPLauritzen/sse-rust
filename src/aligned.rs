use crate::matrix::SqMatrix;

/// Fixed-lag shift equivalence witness for a pair of 2x2 matrices.
///
/// This is the algebraic substrate needed for aligned shift equivalence:
/// once the matrix-level alignment constraints are encoded, they will refine
/// this witness rather than replace it.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ShiftEquivalenceWitness2x2 {
    pub lag: u32,
    pub r: SqMatrix<2>,
    pub s: SqMatrix<2>,
}

/// Verify the classical shift equivalence relations for a proposed witness.
pub fn verify_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    witness: &ShiftEquivalenceWitness2x2,
) -> Result<(), String> {
    if witness.lag == 0 {
        return Err("shift equivalence lag must be positive".into());
    }

    let a_pow = a.pow(witness.lag);
    let b_pow = b.pow(witness.lag);
    let rs = witness.r.mul_u32(&witness.s);
    let sr = witness.s.mul_u32(&witness.r);
    let ar = a.mul_u32(&witness.r);
    let rb = witness.r.mul_u32(b);
    let bs = b.mul_u32(&witness.s);
    let sa = witness.s.mul_u32(a);

    if rs != a_pow {
        return Err(format!("A^lag != RS: {:?} vs {:?}", a_pow, rs));
    }
    if sr != b_pow {
        return Err(format!("B^lag != SR: {:?} vs {:?}", b_pow, sr));
    }
    if ar != rb {
        return Err(format!("AR != RB: {:?} vs {:?}", ar, rb));
    }
    if bs != sa {
        return Err(format!("BS != SA: {:?} vs {:?}", bs, sa));
    }

    Ok(())
}

/// Search for a bounded 2x2 shift equivalence witness.
///
/// This does not yet impose the extra alignment constraints, but it provides
/// the fixed-lag witness search that an aligned solver will build on.
pub fn find_shift_equivalence_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    max_lag: u32,
    max_entry: u32,
) -> Option<ShiftEquivalenceWitness2x2> {
    for lag in 1..=max_lag {
        if let Some(witness) = find_shift_equivalence_with_lag_2x2(a, b, lag, max_entry) {
            return Some(witness);
        }
    }
    None
}

/// Search for a bounded 2x2 shift equivalence witness with a fixed lag.
pub fn find_shift_equivalence_with_lag_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    lag: u32,
    max_entry: u32,
) -> Option<ShiftEquivalenceWitness2x2> {
    if lag == 0 {
        return None;
    }

    let a_pow = a.pow(lag);
    let b_pow = b.pow(lag);
    let r_candidates = enumerate_intertwiners_2x2(a, b, max_entry);

    for r in r_candidates {
        let s_candidates = solve_left_product_2x2(&r, &a_pow, max_entry);
        for s in s_candidates {
            let witness = ShiftEquivalenceWitness2x2 {
                lag,
                r: r.clone(),
                s,
            };

            if witness.s.mul_u32(&witness.r) != b_pow {
                continue;
            }

            if verify_shift_equivalence_2x2(a, b, &witness).is_ok() {
                return Some(witness);
            }
        }
    }

    None
}

fn enumerate_intertwiners_2x2(
    left: &SqMatrix<2>,
    right: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<SqMatrix<2>> {
    let mut candidates = Vec::new();

    for x00 in 0..=max_entry {
        for x01 in 0..=max_entry {
            for x10 in 0..=max_entry {
                for x11 in 0..=max_entry {
                    let x = SqMatrix::new([[x00, x01], [x10, x11]]);
                    if left.mul_u32(&x) == x.mul_u32(right) {
                        candidates.push(x);
                    }
                }
            }
        }
    }

    candidates
}

fn solve_left_product_2x2(
    left: &SqMatrix<2>,
    target: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<SqMatrix<2>> {
    let first_col =
        solve_left_product_column_2x2(left, [target.data[0][0], target.data[1][0]], max_entry);
    if first_col.is_empty() {
        return Vec::new();
    }

    let second_col =
        solve_left_product_column_2x2(left, [target.data[0][1], target.data[1][1]], max_entry);
    if second_col.is_empty() {
        return Vec::new();
    }

    let mut solutions = Vec::new();
    for c0 in &first_col {
        for c1 in &second_col {
            solutions.push(SqMatrix::new([[c0[0], c1[0]], [c0[1], c1[1]]]));
        }
    }
    solutions
}

fn solve_left_product_column_2x2(
    left: &SqMatrix<2>,
    target_col: [u32; 2],
    max_entry: u32,
) -> Vec<[u32; 2]> {
    let [[a, b], [c, d]] = left.data;
    let [t0, t1] = target_col;
    let det = a as i64 * d as i64 - b as i64 * c as i64;

    if det != 0 {
        let x_num = d as i64 * t0 as i64 - b as i64 * t1 as i64;
        let y_num = a as i64 * t1 as i64 - c as i64 * t0 as i64;

        if x_num % det != 0 || y_num % det != 0 {
            return Vec::new();
        }

        let x = x_num / det;
        let y = y_num / det;
        if x < 0 || y < 0 || x > max_entry as i64 || y > max_entry as i64 {
            return Vec::new();
        }

        return vec![[x as u32, y as u32]];
    }

    let mut solutions = Vec::new();
    for x in 0..=max_entry {
        for y in 0..=max_entry {
            let lhs0 = a as u64 * x as u64 + b as u64 * y as u64;
            let lhs1 = c as u64 * x as u64 + d as u64 * y as u64;
            if lhs0 == t0 as u64 && lhs1 == t1 as u64 {
                solutions.push([x, y]);
            }
        }
    }
    solutions
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_shift_equivalence_identity_witness() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let witness = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: a.clone(),
        };

        assert!(verify_shift_equivalence_2x2(&a, &a, &witness).is_ok());
    }

    #[test]
    fn test_verify_shift_equivalence_rejects_bad_witness() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let witness = ShiftEquivalenceWitness2x2 {
            lag: 1,
            r: SqMatrix::identity(),
            s: SqMatrix::identity(),
        };

        assert!(verify_shift_equivalence_2x2(&a, &b, &witness).is_err());
    }

    #[test]
    fn test_find_shift_equivalence_identity_case() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let witness = find_shift_equivalence_2x2(&a, &a, 2, 3).expect("expected witness");
        assert!(verify_shift_equivalence_2x2(&a, &a, &witness).is_ok());
    }

    #[test]
    fn test_find_shift_equivalence_permutation_conjugate_case() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let witness = find_shift_equivalence_2x2(&a, &b, 1, 3).expect("expected witness");
        assert_eq!(witness.lag, 1);
        assert!(verify_shift_equivalence_2x2(&a, &b, &witness).is_ok());
    }

    #[test]
    fn test_find_shift_equivalence_zero_matrix_singular_case() {
        let z = SqMatrix::new([[0, 0], [0, 0]]);
        let witness = find_shift_equivalence_2x2(&z, &z, 1, 0).expect("expected witness");
        assert!(verify_shift_equivalence_2x2(&z, &z, &witness).is_ok());
    }
}
