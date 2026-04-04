use crate::matrix::SqMatrix;

/// Check whether two 2x2 matrices pass all known SSE invariants.
/// Returns `None` if all invariants match, `Some(reason)` on first mismatch.
pub fn check_invariants_2x2(a: &SqMatrix<2>, b: &SqMatrix<2>) -> Option<String> {
    // 1. Trace
    if a.trace() != b.trace() {
        return Some(format!(
            "trace mismatch: {} vs {}",
            a.trace(),
            b.trace()
        ));
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
    let g = gcd(gcd(e00.unsigned_abs(), e01.unsigned_abs()),
                gcd(e10.unsigned_abs(), e11.unsigned_abs()));

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
}
