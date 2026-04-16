use crate::matrix::{DynMatrix, SqMatrix};
use crate::quadratic::ReducedForm;

const GENERIC_SQUARE_TRACE_INVARIANT_MAX_POWER: usize = 4;

/// Determinant-band classification for the narrow 2x2 positive literature
/// territory around Baker (1983) and Choe-Shin (1997).
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DeterminantBand2x2 {
    Baker,
    ChoeShin,
    Neither,
}

impl DeterminantBand2x2 {
    pub fn label(self) -> &'static str {
        match self {
            Self::Baker => "baker",
            Self::ChoeShin => "choe_shin",
            Self::Neither => "neither",
        }
    }
}

/// Cheap arithmetic dossier for one 2x2 endpoint matrix.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ArithmeticProfile2x2 {
    pub trace: i64,
    pub determinant: i64,
    pub discriminant: i64,
    pub determinant_band: DeterminantBand2x2,
}

/// Exact GL(2,Z)-similarity analysis used by [`gl2z_similarity_profile_2x2`].
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Gl2zSimilarityAnalysis2x2 {
    CharacteristicPolynomialMismatch,
    Scalar {
        eigenvalue: i64,
    },
    Split {
        low_eigenvalue: i64,
        high_eigenvalue: i64,
        source_content: i64,
        target_content: i64,
    },
    Irreducible {
        source_order_ideal_class: ReducedForm,
        target_order_ideal_class: ReducedForm,
    },
}

/// Reporting-oriented exact 2x2 dossier for integer similarity and the
/// Baker/Choe-Shin determinant bands.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Gl2zSimilarityProfile2x2 {
    pub source: ArithmeticProfile2x2,
    pub target: ArithmeticProfile2x2,
    pub pair_determinant_band: Option<DeterminantBand2x2>,
    pub gl2z_similar: bool,
    pub analysis: Gl2zSimilarityAnalysis2x2,
}

impl Gl2zSimilarityProfile2x2 {
    pub fn shares_characteristic_polynomial(&self) -> bool {
        self.pair_determinant_band.is_some()
    }
}

/// Arithmetic-only 2x2 dossier used by the GL(2,Z)-similarity profile.
pub fn arithmetic_profile_2x2(matrix: &SqMatrix<2>) -> ArithmeticProfile2x2 {
    let trace = matrix.trace() as i64;
    let determinant = matrix.det();
    let discriminant = trace * trace - 4 * determinant;
    ArithmeticProfile2x2 {
        trace,
        determinant,
        discriminant,
        determinant_band: determinant_band_2x2(trace, determinant),
    }
}

/// Classify the Baker/Choe-Shin determinant territory for one 2x2 endpoint.
pub fn determinant_band_2x2(trace: i64, determinant: i64) -> DeterminantBand2x2 {
    if determinant >= -trace {
        DeterminantBand2x2::Baker
    } else if determinant >= -2 * trace
        && determinant < -trace
        && is_composite(determinant.unsigned_abs())
    {
        DeterminantBand2x2::ChoeShin
    } else {
        DeterminantBand2x2::Neither
    }
}

/// Exact 2x2 integer-similarity profile.
///
/// For irreducible characteristic polynomials, this uses the quadratic-order
/// ideal class in `Z[λ]` (Latimer-MacDuffee/Taussky). For split cases, it uses
/// the standard divisor/content normal form given by `gcd(A - λI)`.
pub fn gl2z_similarity_profile_2x2(
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
) -> Gl2zSimilarityProfile2x2 {
    let source_profile = arithmetic_profile_2x2(source);
    let target_profile = arithmetic_profile_2x2(target);

    if source_profile.trace != target_profile.trace
        || source_profile.determinant != target_profile.determinant
    {
        return Gl2zSimilarityProfile2x2 {
            source: source_profile,
            target: target_profile,
            pair_determinant_band: None,
            gl2z_similar: false,
            analysis: Gl2zSimilarityAnalysis2x2::CharacteristicPolynomialMismatch,
        };
    }

    let pair_determinant_band = Some(source_profile.determinant_band);
    let discriminant = source_profile.discriminant;

    if let Some(sqrt_discriminant) = exact_square_root(discriminant) {
        debug_assert_eq!((source_profile.trace + sqrt_discriminant) % 2, 0);
        debug_assert_eq!((source_profile.trace - sqrt_discriminant) % 2, 0);

        let low_eigenvalue = (source_profile.trace - sqrt_discriminant) / 2;
        let high_eigenvalue = (source_profile.trace + sqrt_discriminant) / 2;
        let source_content = split_similarity_content_2x2(source, low_eigenvalue);
        let target_content = split_similarity_content_2x2(target, low_eigenvalue);
        let gl2z_similar = source_content == target_content;

        let analysis = if source_content == 0 && target_content == 0 {
            Gl2zSimilarityAnalysis2x2::Scalar {
                eigenvalue: low_eigenvalue,
            }
        } else {
            Gl2zSimilarityAnalysis2x2::Split {
                low_eigenvalue,
                high_eigenvalue,
                source_content,
                target_content,
            }
        };

        return Gl2zSimilarityProfile2x2 {
            source: source_profile,
            target: target_profile,
            pair_determinant_band,
            gl2z_similar,
            analysis,
        };
    }

    let source_class = crate::quadratic::eigenvector_ideal_class_2x2(source)
        .expect("irreducible 2x2 endpoint should yield a quadratic-order ideal class");
    let target_class = crate::quadratic::eigenvector_ideal_class_2x2(target)
        .expect("irreducible 2x2 endpoint should yield a quadratic-order ideal class");
    let gl2z_similar = source_class == target_class;

    Gl2zSimilarityProfile2x2 {
        source: source_profile,
        target: target_profile,
        pair_determinant_band,
        gl2z_similar,
        analysis: Gl2zSimilarityAnalysis2x2::Irreducible {
            source_order_ideal_class: source_class,
            target_order_ideal_class: target_class,
        },
    }
}

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

/// Check a bounded generic power-trace surface for square endpoint matrices.
///
/// SSE preserves the nonzero spectrum, so `trace(M^k)` is invariant even when
/// endpoint dimensions differ because extra zero eigenvalues contribute nothing.
/// We compare the first few powers up to the current bounded square endpoint
/// dimensions, keeping the check cheap and dimension-agnostic.
pub fn check_square_power_trace_invariants(a: &DynMatrix, b: &DynMatrix) -> Option<String> {
    debug_assert!(a.is_square());
    debug_assert!(b.is_square());

    let max_power = a
        .rows
        .max(b.rows)
        .min(GENERIC_SQUARE_TRACE_INVARIANT_MAX_POWER);
    if max_power == 0 {
        return None;
    }

    let a_traces = square_power_traces(a, max_power);
    let b_traces = square_power_traces(b, max_power);

    for power in [2usize, 1, 3, 4] {
        if power > max_power {
            continue;
        }
        if a_traces[power - 1] != b_traces[power - 1] {
            return Some(power_trace_mismatch_reason(power));
        }
    }

    None
}

fn square_power_traces(m: &DynMatrix, max_power: usize) -> Vec<u64> {
    debug_assert!(m.is_square());
    let mut traces = Vec::with_capacity(max_power);
    let mut power = m.clone();
    for exponent in 1..=max_power {
        traces.push(power.trace());
        if exponent < max_power {
            power = power.mul(m);
        }
    }
    traces
}

fn power_trace_mismatch_reason(power: usize) -> String {
    match power {
        1 => "trace invariant mismatch".to_string(),
        2 => "trace(M^2) invariant mismatch".to_string(),
        _ => format!("trace(M^{power}) invariant mismatch"),
    }
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

fn exact_square_root(n: i64) -> Option<i64> {
    if n < 0 {
        return None;
    }
    let mut root = (n as f64).sqrt() as u64;
    let target = n as u64;
    while root * root > target {
        root -= 1;
    }
    while (root + 1) * (root + 1) <= target {
        root += 1;
    }
    if root * root == target {
        Some(root as i64)
    } else {
        None
    }
}

fn split_similarity_content_2x2(matrix: &SqMatrix<2>, eigenvalue: i64) -> i64 {
    let [[a, b], [c, d]] = matrix.data;
    gcd(
        gcd((a as i64 - eigenvalue).unsigned_abs(), b as u64),
        gcd(c as u64, (d as i64 - eigenvalue).unsigned_abs()),
    ) as i64
}

fn is_composite(n: u64) -> bool {
    if n < 4 {
        return false;
    }
    let mut factor = 2u64;
    while factor * factor <= n {
        if n % factor == 0 {
            return true;
        }
        factor += 1;
    }
    false
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
    fn test_square_power_trace_invariants_allow_zero_extended_spectrum() {
        let a = DynMatrix::new(2, 2, vec![2, 0, 0, 1]);
        let b = DynMatrix::new(3, 3, vec![2, 0, 0, 0, 1, 0, 0, 0, 0]);
        assert_eq!(check_square_power_trace_invariants(&a, &b), None);
    }

    #[test]
    fn test_square_power_trace_invariants_reject_trace_cube_mismatch() {
        let a = DynMatrix::new(3, 3, vec![0, 0, 0, 0, 3, 0, 0, 0, 3]);
        let b = DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 4]);
        assert_eq!(
            check_square_power_trace_invariants(&a, &b),
            Some("trace(M^3) invariant mismatch".to_string())
        );
    }

    #[test]
    fn test_same_matrix_passes() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        assert_eq!(check_invariants_2x2(&a, &a), None);
    }

    #[test]
    fn test_determinant_band_classification() {
        assert_eq!(determinant_band_2x2(6, 7), DeterminantBand2x2::Baker);
        assert_eq!(determinant_band_2x2(4, -6), DeterminantBand2x2::ChoeShin);
        assert_eq!(determinant_band_2x2(4, -5), DeterminantBand2x2::Neither);
    }

    #[test]
    fn test_gl2z_similarity_profile_rejects_characteristic_polynomial_mismatch() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[3, 1], [1, 1]]);
        let profile = gl2z_similarity_profile_2x2(&a, &b);

        assert!(!profile.gl2z_similar);
        assert_eq!(profile.pair_determinant_band, None);
        assert_eq!(
            profile.analysis,
            Gl2zSimilarityAnalysis2x2::CharacteristicPolynomialMismatch
        );
    }

    #[test]
    fn test_gl2z_similarity_profile_irreducible_baker_case() {
        let a = SqMatrix::new([[3, 2], [1, 3]]);
        let b = SqMatrix::new([[2, 1], [1, 4]]);
        let profile = gl2z_similarity_profile_2x2(&a, &b);

        assert!(profile.gl2z_similar);
        assert_eq!(
            profile.pair_determinant_band,
            Some(DeterminantBand2x2::Baker)
        );
        match profile.analysis {
            Gl2zSimilarityAnalysis2x2::Irreducible {
                source_order_ideal_class,
                target_order_ideal_class,
            } => assert_eq!(source_order_ideal_class, target_order_ideal_class),
            other => panic!("expected irreducible similarity analysis, got {other:?}"),
        }
    }

    #[test]
    fn test_gl2z_similarity_profile_irreducible_non_similar_pair() {
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);
        let profile = gl2z_similarity_profile_2x2(&a, &b);

        assert!(!profile.gl2z_similar);
        assert_eq!(
            profile.pair_determinant_band,
            Some(DeterminantBand2x2::Baker)
        );
        match profile.analysis {
            Gl2zSimilarityAnalysis2x2::Irreducible {
                source_order_ideal_class,
                target_order_ideal_class,
            } => assert_ne!(source_order_ideal_class, target_order_ideal_class),
            other => panic!("expected irreducible similarity analysis, got {other:?}"),
        }
    }

    #[test]
    fn test_gl2z_similarity_profile_split_cases_use_content_invariant() {
        let similar_a = SqMatrix::new([[1, 1], [0, 3]]);
        let similar_b = SqMatrix::new([[2, 1], [1, 2]]);
        let similar_profile = gl2z_similarity_profile_2x2(&similar_a, &similar_b);
        assert!(similar_profile.gl2z_similar);
        match similar_profile.analysis {
            Gl2zSimilarityAnalysis2x2::Split {
                low_eigenvalue,
                high_eigenvalue,
                source_content,
                target_content,
            } => {
                assert_eq!((low_eigenvalue, high_eigenvalue), (1, 3));
                assert_eq!(source_content, 1);
                assert_eq!(target_content, 1);
            }
            other => panic!("expected split similarity analysis, got {other:?}"),
        }

        let not_similar_a = SqMatrix::new([[1, 1], [0, 3]]);
        let not_similar_b = SqMatrix::new([[1, 2], [0, 3]]);
        let not_similar_profile = gl2z_similarity_profile_2x2(&not_similar_a, &not_similar_b);
        assert!(!not_similar_profile.gl2z_similar);
        match not_similar_profile.analysis {
            Gl2zSimilarityAnalysis2x2::Split {
                source_content,
                target_content,
                ..
            } => {
                assert_eq!(source_content, 1);
                assert_eq!(target_content, 2);
            }
            other => panic!("expected split similarity analysis, got {other:?}"),
        }
    }

    #[test]
    fn test_gl2z_similarity_profile_repeated_eigenvalue_distinguishes_scalar_case() {
        let scalar = SqMatrix::new([[2, 0], [0, 2]]);
        let same_scalar = SqMatrix::new([[2, 0], [0, 2]]);
        let scalar_profile = gl2z_similarity_profile_2x2(&scalar, &same_scalar);
        assert!(scalar_profile.gl2z_similar);
        assert_eq!(
            scalar_profile.analysis,
            Gl2zSimilarityAnalysis2x2::Scalar { eigenvalue: 2 }
        );

        let jordan = SqMatrix::new([[2, 1], [0, 2]]);
        let larger_jordan = SqMatrix::new([[2, 2], [0, 2]]);
        let repeated_profile = gl2z_similarity_profile_2x2(&jordan, &larger_jordan);
        assert!(!repeated_profile.gl2z_similar);
        match repeated_profile.analysis {
            Gl2zSimilarityAnalysis2x2::Split {
                low_eigenvalue,
                high_eigenvalue,
                source_content,
                target_content,
            } => {
                assert_eq!((low_eigenvalue, high_eigenvalue), (2, 2));
                assert_eq!(source_content, 1);
                assert_eq!(target_content, 2);
            }
            other => panic!("expected repeated-eigenvalue split analysis, got {other:?}"),
        }
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
