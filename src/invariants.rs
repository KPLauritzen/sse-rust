use crate::matrix::SqMatrix;

/// Check whether two 2x2 matrices pass all known SSE invariants.
/// Returns `None` if all invariants match, `Some(reason)` on first mismatch.
pub fn check_invariants_2x2(a: &SqMatrix<2>, b: &SqMatrix<2>) -> Option<String> {
    // 1. Trace
    if a.trace() != b.trace() {
        return Some(format!("trace mismatch: {} vs {}", a.trace(), b.trace()));
    }

    // 2. Determinant
    let det_a = a.det();
    let det_b = b.det();
    if det_a != det_b {
        return Some(format!("determinant mismatch: {} vs {}", det_a, det_b));
    }

    // 3. Trace sequences (k=2..10)
    // Since trace and det match, the Newton recurrence guarantees all trace
    // powers match for 2x2 matrices. The characteristic polynomial is
    // t^2 - tr(A)*t + det(A), and if tr and det match, the nonzero eigenvalues
    // match, so all tr(A^k) match. Skip this check for 2x2.

    // 4. Bowen-Franks group: Smith normal form of (I - A)
    let bf_a = bowen_franks_2x2(a);
    let bf_b = bowen_franks_2x2(b);
    if bf_a != bf_b {
        return Some(format!(
            "Bowen-Franks group mismatch: {:?} vs {:?}",
            bf_a, bf_b
        ));
    }

    // 5. Generalized Bowen-Franks groups: Z^2 / p(A)Z^2 for various polynomials
    if let Some(reason) = check_generalized_bowen_franks_2x2(a, b) {
        return Some(reason);
    }

    // 6. Eilers-Kiming ideal class invariant
    if let Some(reason) = check_eilers_kiming_2x2(a, b) {
        return Some(reason);
    }

    None
}

/// Compute the Bowen-Franks invariant for a 2x2 matrix.
/// This is the Smith normal form of (I - A), represented as the sorted
/// diagonal entries (d1, d2) where d1 | d2.
///
/// For 2x2, (I - A) = [[1-a, -b], [-c, 1-d]].
/// Smith normal form: d1 = gcd of all entries, d2 = det / d1.
fn bowen_franks_2x2(m: &SqMatrix<2>) -> (i64, i64) {
    let [[a, b], [c, d]] = m.data;
    // Entries of (I - A)
    let e00 = 1i64 - a as i64;
    let e01 = -(b as i64);
    let e10 = -(c as i64);
    let e11 = 1i64 - d as i64;

    // det(I - A)
    let det = e00 * e11 - e01 * e10;

    // gcd of all four entries
    let g = gcd(
        gcd(e00.unsigned_abs(), e01.unsigned_abs()),
        gcd(e10.unsigned_abs(), e11.unsigned_abs()),
    );

    if g == 0 {
        // All entries zero means I - A = 0, so A = I
        return (0, 0);
    }

    // Smith normal form for 2x2: d1 = g, d2 = det / g
    // We use absolute values since the Smith normal form uses nonneg entries
    // with the convention that d1 | d2 and d1 >= 0.
    let d1 = g as i64;
    let d2 = det / d1;

    // Return in canonical form (both can be signed for our comparison purposes,
    // what matters is that both matrices give the same pair)
    (d1, d2)
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

/// Evaluate a polynomial p(x) = coeffs[0] + coeffs[1]*x + coeffs[2]*x^2 + ...
/// at a 2x2 matrix A, returning a 2x2 i64 matrix.
fn eval_poly_at_matrix_2x2(coeffs: &[i64], a: &SqMatrix<2>) -> [[i64; 2]; 2] {
    let [[a00, a01], [a10, a11]] = a.data;
    let (a00, a01, a10, a11) = (a00 as i64, a01 as i64, a10 as i64, a11 as i64);

    // Build up: result = sum of coeffs[k] * A^k
    // We track A^k iteratively.
    let mut result = [[0i64; 2]; 2];
    // pow = A^k, starting at I
    let mut pow = [[1i64, 0], [0, 1i64]];

    for &c in coeffs {
        for i in 0..2 {
            for j in 0..2 {
                result[i][j] += c * pow[i][j];
            }
        }
        // pow = pow * A
        let new_pow = [
            [
                pow[0][0] * a00 + pow[0][1] * a10,
                pow[0][0] * a01 + pow[0][1] * a11,
            ],
            [
                pow[1][0] * a00 + pow[1][1] * a10,
                pow[1][0] * a01 + pow[1][1] * a11,
            ],
        ];
        pow = new_pow;
    }
    result
}

/// Smith normal form of a 2x2 integer matrix.
/// Returns (d1, d2) where d1 | d2 (using absolute values).
fn smith_normal_form_2x2_i64(m: &[[i64; 2]; 2]) -> (i64, i64) {
    let g = gcd(
        gcd(m[0][0].unsigned_abs(), m[0][1].unsigned_abs()),
        gcd(m[1][0].unsigned_abs(), m[1][1].unsigned_abs()),
    );
    if g == 0 {
        return (0, 0);
    }
    let det = m[0][0] * m[1][1] - m[0][1] * m[1][0];
    let d1 = g as i64;
    let d2 = det / d1;
    (d1, d2)
}

/// Check generalized Bowen-Franks groups Z^2 / p(A)Z^2 for a battery of
/// polynomials from Eilers-Kiming (2008), Section 3.
fn check_generalized_bowen_franks_2x2(a: &SqMatrix<2>, b: &SqMatrix<2>) -> Option<String> {
    // Polynomials from Eilers-Kiming p.7, represented as coefficient vectors
    // [c0, c1, c2, ...] for c0 + c1*x + c2*x^2 + ...
    let polynomials: &[(&str, &[i64])] = &[
        // x - 1 is already checked as standard Bowen-Franks, skip
        ("x+1", &[1, 1]),
        ("2x-1", &[-1, 2]),
        ("2x+1", &[1, 2]),
        ("x^2-x-1", &[-1, -1, 1]),
        ("x^2-x+1", &[1, -1, 1]),
        ("x^2+x-1", &[-1, 1, 1]),
        ("x^2+x+1", &[1, 1, 1]),
        ("x^2-2x+1", &[1, -2, 1]),
        ("x^2+2x+1", &[1, 2, 1]),
        ("x^2-1", &[-1, 0, 1]),
        ("x^2+1", &[1, 0, 1]),
        ("2x^2-x-1", &[-1, -1, 2]),
        ("2x^2+x-1", &[-1, 1, 2]),
        ("2x^2-3x+1", &[1, -3, 2]),
        ("2x^2+3x+1", &[1, 3, 2]),
        ("4x^2-4x+1", &[1, -4, 4]),
        ("4x^2+4x+1", &[1, 4, 4]),
        ("4x^2-1", &[-1, 0, 4]),
    ];

    for (name, coeffs) in polynomials {
        let pa = eval_poly_at_matrix_2x2(coeffs, a);
        let pb = eval_poly_at_matrix_2x2(coeffs, b);
        let snf_a = smith_normal_form_2x2_i64(&pa);
        let snf_b = smith_normal_form_2x2_i64(&pb);
        if snf_a != snf_b {
            return Some(format!(
                "generalized Bowen-Franks mismatch for p(x)={}: {:?} vs {:?}",
                name, snf_a, snf_b
            ));
        }
    }
    None
}

/// Check the Eilers-Kiming ideal class invariant (Theorem 1, part iii).
/// For irreducible 2x2 matrices over a quadratic number field, computes the
/// ideal class of the Perron eigenvector ideal in O_K and compares.
fn check_eilers_kiming_2x2(a: &SqMatrix<2>, b: &SqMatrix<2>) -> Option<String> {
    use crate::quadratic;

    let class_a = quadratic::eigenvector_ideal_class_2x2(a);
    let class_b = quadratic::eigenvector_ideal_class_2x2(b);

    match (class_a, class_b) {
        (Some(ca), Some(cb)) => {
            if ca != cb {
                Some(format!(
                    "Eilers-Kiming ideal class mismatch: {:?} vs {:?}",
                    ca, cb
                ))
            } else {
                None
            }
        }
        // If we can't compute for one or both, skip this invariant.
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_same_matrix_passes() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        assert_eq!(check_invariants_2x2(&a, &a), None);
    }

    #[test]
    fn test_conjugate_passes() {
        // Conjugate matrices are SSE, should pass all invariants.
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        assert_eq!(check_invariants_2x2(&a, &b), None);
    }

    #[test]
    fn test_different_trace_fails() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[3, 1], [1, 1]]);
        let result = check_invariants_2x2(&a, &b);
        assert!(result.is_some());
        assert!(result.unwrap().contains("trace"));
    }

    #[test]
    fn test_different_det_fails() {
        let a = SqMatrix::new([[3, 1], [1, 1]]); // det = 2
        let b = SqMatrix::new([[2, 1], [1, 2]]); // det = 3, but trace = 4 for both
                                                 // Actually trace(a) = 4, trace(b) = 4, det(a) = 2, det(b) = 3
        let result = check_invariants_2x2(&a, &b);
        assert!(result.is_some());
        assert!(result.unwrap().contains("determinant"));
    }

    #[test]
    fn test_bowen_franks_conjugate() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        assert_eq!(bowen_franks_2x2(&a), bowen_franks_2x2(&b));
    }

    #[test]
    fn test_gcd() {
        assert_eq!(gcd(12, 8), 4);
        assert_eq!(gcd(7, 0), 7);
        assert_eq!(gcd(0, 5), 5);
        assert_eq!(gcd(0, 0), 0);
    }

    #[test]
    fn test_eval_poly_identity() {
        // p(x) = 1 (constant) should give the identity matrix
        let a = SqMatrix::new([[3, 1], [2, 5]]);
        let result = eval_poly_at_matrix_2x2(&[1], &a);
        assert_eq!(result, [[1, 0], [0, 1]]);
    }

    #[test]
    fn test_eval_poly_x_minus_1() {
        // p(x) = x - 1 at A should give A - I
        let a = SqMatrix::new([[3, 1], [2, 5]]);
        let result = eval_poly_at_matrix_2x2(&[-1, 1], &a);
        assert_eq!(result, [[2, 1], [2, 4]]);
    }

    #[test]
    fn test_eval_poly_x_squared() {
        // p(x) = x^2 at A should give A^2
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let result = eval_poly_at_matrix_2x2(&[0, 0, 1], &a);
        // A^2 = [[5,3],[3,2]]
        assert_eq!(result, [[5, 3], [3, 2]]);
    }

    #[test]
    fn test_smith_normal_form_2x2() {
        // [[2, 4], [6, 8]]: gcd=2, det=2*8-4*6=-8, d2=-8/2=-4
        let m = [[2i64, 4], [6, 8]];
        let (d1, d2) = smith_normal_form_2x2_i64(&m);
        assert_eq!(d1, 2);
        assert_eq!(d2, -4);
    }

    #[test]
    fn test_generalized_bf_same_matrix() {
        let a = SqMatrix::new([[5, 13], [6, 1]]);
        assert_eq!(check_generalized_bowen_franks_2x2(&a, &a), None);
    }

    #[test]
    fn test_generalized_bf_conjugate() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        assert_eq!(check_generalized_bowen_franks_2x2(&a, &b), None);
    }
}
