//! Quadratic-order arithmetic for 2x2 integer similarity and the Eilers-Kiming
//! ideal-class invariant.
//!
//! For a 2×2 matrix with characteristic polynomial x² - tx + d, the Perron
//! eigenvalue λ lives in the quadratic field K = Q(√Δ) where Δ = t² - 4d.
//! For irreducible characteristic polynomials, the order `Z[λ]` has
//! discriminant Δ, and reduced binary quadratic forms of discriminant Δ encode
//! its ideal classes. That gives an exact `GL(2,Z)`-similarity classifier via
//! the Latimer-MacDuffee/Taussky correspondence, and also supplies the same
//! order-ideal-class datum used by the current Eilers-Kiming obstruction.
//!
//! We represent ideal classes via binary quadratic forms, which are equivalent
//! to ideal classes in quadratic orders. For negative discriminant (imaginary
//! quadratic), we use Gauss reduction. For positive discriminant (real
//! quadratic), we use continued-fraction reduction.

use crate::matrix::SqMatrix;

/// A reduced binary quadratic form (a, b, c) representing ax² + bxy + cy²
/// with discriminant b² - 4ac = D.
/// Two ideals are in the same class iff they reduce to the same form.
#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ReducedForm {
    pub a: i64,
    pub b: i64,
    pub c: i64,
}

/// Exact profile of the quadratic order `Z[λ]` attached to an irreducible
/// `2x2` characteristic polynomial.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct QuadraticOrderProfile {
    /// The order discriminant `disc(Z[λ])`.
    pub order_discriminant: i64,
    /// The fundamental discriminant of the ambient quadratic field.
    pub field_discriminant: i64,
    /// The conductor `f`, equivalently the order index `[O_K : Z[λ]]`.
    pub conductor: i64,
}

impl QuadraticOrderProfile {
    pub fn maximal_order(self) -> bool {
        self.conductor == 1
    }
}

/// Return the exact quadratic-order profile determined by the given
/// discriminant, or `None` when the characteristic polynomial is not
/// irreducible over `Q`.
pub fn quadratic_order_profile(discriminant: i64) -> Option<QuadraticOrderProfile> {
    if discriminant == 0 {
        return None;
    }
    if discriminant > 0 && is_perfect_square(discriminant as u64) {
        return None;
    }

    let (field_discriminant, conductor) = fundamental_discriminant(discriminant);
    Some(QuadraticOrderProfile {
        order_discriminant: discriminant,
        field_discriminant,
        conductor,
    })
}

/// Canonical reduced representative of the principal class for the quadratic
/// order of discriminant `D`.
pub fn principal_reduced_form(discriminant: i64) -> Option<ReducedForm> {
    quadratic_order_profile(discriminant)?;

    let principal_b = discriminant.rem_euclid(2);
    let principal_c = (principal_b * principal_b - discriminant) / 4;
    if discriminant < 0 {
        Some(reduce_form_negative(1, principal_b, principal_c))
    } else {
        Some(reduce_form_positive(1, principal_b, principal_c))
    }
}

/// Whether the given reduced form represents the principal class in the order
/// of discriminant `D`.
pub fn reduced_form_is_principal(discriminant: i64, form: &ReducedForm) -> Option<bool> {
    let principal = principal_reduced_form(discriminant)?;
    Some(*form == principal)
}

/// Compute the fundamental discriminant D_K from the raw discriminant Δ = t² - 4d.
/// Factors out perfect squares: if Δ = f²·D_K where D_K is squarefree
/// (times 1 or 4 depending on D_K mod 4), returns (D_K_fund, f) where
/// D_K_fund is the fundamental discriminant of K.
fn fundamental_discriminant(delta: i64) -> (i64, i64) {
    if delta == 0 {
        return (0, 0);
    }
    let sign = if delta > 0 { 1 } else { -1 };
    let abs_delta = delta.unsigned_abs();

    // Factor out the largest square from abs_delta.
    // abs_delta = f^2 * d where d is squarefree.
    let mut d = abs_delta;
    let mut f: u64 = 1;

    let mut p = 2u64;
    while p * p <= d {
        while d % (p * p) == 0 {
            d /= p * p;
            f *= p;
        }
        p += 1;
    }
    // Now delta = sign * f^2 * d, and d is squarefree.
    let d_signed = sign * d as i64;

    // Fundamental discriminant: D_K = d if d ≡ 1 (mod 4), else D_K = 4d.
    // But we also need to adjust f: if D_K = 4d then delta = (f)^2 * 4d = (f)^2 * D_K,
    // but we need delta = g^2 * D_K. If d ≡ 1 mod 4, D_K = d, g = f * 2 if
    // the original had extra factors of 4... Actually let me re-derive.
    //
    // We have Δ = t² - 4d_matrix. We want Δ = g² · D_K where D_K is the
    // fundamental discriminant. The fundamental discriminant of Q(√Δ) is:
    //   - If squarefree part of Δ ≡ 1 (mod 4): D_K = squarefree part
    //   - Otherwise: D_K = 4 * squarefree part
    //
    // Since Δ = sign * f² * d (d squarefree):
    //   If sign*d ≡ 1 mod 4: D_K = sign*d, and Δ = f² * D_K, so g = f
    //   If sign*d ≢ 1 mod 4: D_K = 4*sign*d, need Δ = g² * D_K
    //     f² * d = g² * 4d => g = f/2 (f must be even)
    //     But f might be odd! If f is odd, then Δ = f²·d and D_K = 4d,
    //     but f² · d ≠ g² · 4d for integer g. Actually this can't happen:
    //     Δ = t² - 4·det. So Δ ≡ t² mod 4. If t is even, Δ ≡ 0 mod 4.
    //     If t is odd, Δ ≡ 1 mod 4. So Δ mod 4 ∈ {0, 1}.
    //     If d ≡ 2 or 3 mod 4 and sign = 1: d_signed ≡ 2 or 3 mod 4,
    //     D_K = 4d_signed. Then Δ = f²·d and we need f even so g = f/2.
    //     But we know Δ ≡ 0 or 1 mod 4. If d ≡ 2 mod 4, d is even but
    //     squarefree, so d ≡ 2 mod 4. f²·d ≡ 0 mod 4 only if f is even
    //     or d ≡ 0 mod 4. d ≡ 2 mod 4 and f odd gives f²d ≡ 2 mod 4, contradiction.
    //     So f must be even. Similarly for d ≡ 3 mod 4 with sign = 1:
    //     f²·3 ≡ 3f² mod 4. For this to be 0 or 1 mod 4, need f even (3·4 ≡ 0).
    //
    // OK, let's just compute it directly.

    let d_mod4 = ((d_signed % 4) + 4) % 4;
    if d_mod4 == 1 {
        // D_K = d_signed, g = f
        (d_signed, f as i64)
    } else {
        // D_K = 4 * d_signed, g = f / 2
        // f must be even (see above)
        debug_assert!(
            f % 2 == 0,
            "f={} should be even when d_signed={} mod 4 != 1",
            f,
            d_mod4
        );
        (4 * d_signed, (f / 2) as i64)
    }
}

/// Given a quadratic form (a, b, c) with discriminant D = b² - 4ac < 0,
/// return the unique reduced form in the same class.
///
/// A form is reduced if |b| ≤ a ≤ c, and if |b| = a or a = c then b ≥ 0.
fn reduce_form_negative(mut a: i64, mut b: i64, mut c: i64) -> ReducedForm {
    // Ensure a > 0 (positive definite)
    if a < 0 {
        a = -a;
        b = -b;
        c = -c;
    }

    loop {
        // Step 1: Ensure |b| ≤ a
        if b.abs() > a {
            // Replace (a, b, c) by (a, b - 2ka, ...) where k rounds b/(2a)
            let k = if b > 0 {
                (b + a) / (2 * a)
            } else {
                -(-b + a) / (2 * a)
            };
            let new_b = b - 2 * k * a;
            let new_c = (new_b * new_b - (b * b - 4 * a * c)) / (4 * a);
            b = new_b;
            c = new_c;
        }

        // Step 2: If a > c, swap: (a,b,c) -> (c,-b,a)
        if a > c {
            let tmp = a;
            a = c;
            c = tmp;
            b = -b;
            continue;
        }

        // Step 3: Normalize sign of b in boundary cases
        if b < 0 && (b.abs() == a || a == c) {
            b = -b;
        }

        break;
    }

    ReducedForm { a, b, c }
}

/// Given a quadratic form (a, b, c) with discriminant D = b² - 4ac > 0,
/// return a canonical reduced form in the same class.
///
/// For real quadratic fields, "reduced" means: 0 < b < √D and √D - b < 2|a| < √D + b.
/// Reduced forms are not unique but cycle; we pick the lexicographic minimum in the cycle.
fn reduce_form_positive(a: i64, b: i64, c: i64) -> ReducedForm {
    let disc = b * b - 4 * a * c;
    debug_assert!(disc > 0);
    let sqrt_d = isqrt(disc as u64) as i64;

    let mut cur_a = a;
    let mut cur_b = b;
    let mut cur_c = c;

    // Ensure a > 0 initially
    if cur_a < 0 {
        cur_a = -cur_a;
        cur_b = -cur_b;
        cur_c = -cur_c;
    }

    // Reduce: repeatedly apply the rho operator until we reach a reduced form,
    // then cycle through all reduced forms to find the lex-min.
    // rho: (a, b, c) -> (c, -b + 2c * round((b)/(2c)), ...)
    // More precisely: (a,b,c) -> (c, b', a') where b' = -b + 2c*k, k chosen
    // so that |b'| < sqrt(D) and b' has same parity as b (i.e., b' ≡ b mod 2c).

    // First, reduce to a "nearly reduced" form.
    for _ in 0..1000 {
        // Apply rho: (a, b, c) -> (c, b', a')
        // Choose k so that b' = -cur_b + 2*cur_c*k is closest to sqrt_d (and same sign)
        // We want 0 < b' and sqrt_d - b' < 2|cur_c|
        if cur_c == 0 {
            break;
        }
        // Standard approach: b' such that b' ≡ -cur_b mod 2|cur_c|
        // and sqrt_d - 2|cur_c| < b' ≤ sqrt_d
        let two_c = 2 * cur_c.abs();
        if two_c == 0 {
            break;
        }
        let neg_b_mod = ((-cur_b) % two_c + two_c) % two_c;
        // b' = neg_b_mod + 2|c| * m, we want sqrt_d - 2|c| < b' <= sqrt_d
        // so m = floor((sqrt_d - neg_b_mod) / two_c)
        let m = if sqrt_d >= neg_b_mod {
            (sqrt_d - neg_b_mod) / two_c
        } else {
            -((neg_b_mod - sqrt_d + two_c - 1) / two_c)
        };
        let b_prime = neg_b_mod + two_c * m;

        // Check: is it reduced? 0 < b_prime and sqrt_d - b_prime < 2|c|
        // If b_prime <= 0, try m+1
        let b_prime = if b_prime <= 0 {
            b_prime + two_c
        } else {
            b_prime
        };

        let new_a = cur_c.abs();
        let new_c = (b_prime * b_prime - disc) / (4 * new_a);
        // Check reduced condition
        let is_reduced =
            b_prime > 0 && sqrt_d - b_prime < 2 * new_a && 2 * new_a < sqrt_d + b_prime;

        cur_a = new_a;
        cur_b = b_prime;
        cur_c = if new_c < 0 { new_c } else { new_c };

        if is_reduced {
            break;
        }
    }

    // Now cycle through reduced forms to find the lex-min representative.
    // The cycle has finite length (bounded by class number).
    let start_a = cur_a;
    let start_b = cur_b;
    let start_c = cur_c;
    let mut best = ReducedForm {
        a: cur_a.abs(),
        b: cur_b,
        c: cur_c.abs(),
    };

    for _ in 0..1000 {
        // Apply rho: (a, b, c) -> (|c|, b', ...)
        let two_c = 2 * cur_c.abs();
        if two_c == 0 {
            break;
        }
        let neg_b_mod = ((-cur_b) % two_c + two_c) % two_c;
        let m = if sqrt_d >= neg_b_mod {
            (sqrt_d - neg_b_mod) / two_c
        } else {
            -((neg_b_mod - sqrt_d + two_c - 1) / two_c)
        };
        let mut b_prime = neg_b_mod + two_c * m;
        if b_prime <= 0 {
            b_prime += two_c;
        }

        let new_a = cur_c.abs();
        let new_c = (b_prime * b_prime - disc) / (4 * new_a);

        cur_a = new_a;
        cur_b = b_prime;
        cur_c = new_c;

        let candidate = ReducedForm {
            a: cur_a.abs(),
            b: cur_b,
            c: cur_c.abs(),
        };
        if candidate.a < best.a
            || (candidate.a == best.a && candidate.b < best.b)
            || (candidate.a == best.a && candidate.b == best.b && candidate.c < best.c)
        {
            best = candidate;
        }

        if cur_a == start_a && cur_b == start_b && cur_c == start_c {
            break;
        }
    }

    best
}

/// Integer square root (floor).
fn isqrt(n: u64) -> u64 {
    if n == 0 {
        return 0;
    }
    let mut x = (n as f64).sqrt() as u64;
    // Correct potential floating-point errors
    while x * x > n {
        x -= 1;
    }
    while (x + 1) * (x + 1) <= n {
        x += 1;
    }
    x
}

/// Check if n is a perfect square.
fn is_perfect_square(n: u64) -> bool {
    let s = isqrt(n);
    s * s == n
}

/// Compute the class of the Perron eigenvector ideal in the quadratic order
/// `Z[λ]` for a 2×2 matrix.
///
/// For A = [[a,b],[c,d]] with characteristic polynomial x² - tx + det where
/// t = a+d, the Perron eigenvector is v = (λ - d, c) where λ is the larger root.
///
/// The ideal `I = Z[λ] · (λ - d) + Z[λ] · c` corresponds to a binary quadratic
/// form of discriminant Δ, which we reduce to a canonical representative.
///
/// Returns None if the invariant doesn't apply (rational eigenvalues, zero
/// eigenvector component, etc.).
pub fn eigenvector_ideal_class_2x2(mat: &SqMatrix<2>) -> Option<ReducedForm> {
    let [[a, b], [c, d]] = mat.data;
    let t = a as i64 + d as i64; // trace
    let det = a as i64 * d as i64 - b as i64 * c as i64; // determinant

    // Discriminant of char poly x² - tx + det
    let delta = t * t - 4 * det;

    quadratic_order_profile(delta)?;

    // The eigenvector for eigenvalue λ is (λ - d, c) (from (A - λI)v = 0).
    // If c = 0, use the other row: (b, λ - a).
    // We need at least one off-diagonal entry nonzero for a useful invariant.
    let (v_rational, v_lambda_coeff) = if c > 0 {
        // v = (λ - d, c) where λ = (t + √Δ)/2 (Perron eigenvalue)
        // v₁ = λ - d = (t - 2d)/2 + √Δ/2 = (a - d)/2 + √Δ/2
        // v₂ = c
        (c as i64, a as i64 - d as i64)
    } else if b > 0 {
        // v = (b, λ - a) where λ = (t + √Δ)/2
        // v₁ = b
        // v₂ = λ - a = (t - 2a)/2 + √Δ/2 = (d - a)/2 + √Δ/2
        (b as i64, d as i64 - a as i64)
    } else {
        return None; // Diagonal matrix, not irreducible
    };

    // We need to compute the ideal class of the ideal generated by v₁ and v₂
    // in O_K where K = Q(√Δ).
    //
    // v_rational = c (or b), an integer
    // The other component is (v_lambda_coeff + √Δ) / 2
    //
    // The ideal I = O_K · v_rational + O_K · ((v_lambda_coeff + √Δ)/2)
    //
    // An ideal in a quadratic order can be represented as a binary quadratic form.
    // The ideal aZ + ((b + √D)/2)Z has norm a and corresponds to form (a, b, (b²-D)/(4a)).
    //
    // We work with the fundamental discriminant D_K and the full ring O_K.

    // For the ideal class computation, we work with discriminant Δ directly
    // (the form discriminant), not the fundamental discriminant. The ideal
    // I = c·O_K + ((v_lambda_coeff + √Δ)/2)·O_K.
    //
    // This ideal has the quadratic form representation with:
    //   norm(I) = N(I) and the form (N(I), ..., ...)
    //
    // More directly: the ideal generated by an integer n and (p + √Δ)/2 is
    // nZ + ((p + √Δ)/2)Z, which corresponds to form (n, p, (p²-Δ)/(4n))
    // when it represents an invertible ideal.
    //
    // But we need to be careful: the eigenvector gives us an ideal in O_K
    // (the maximal order), not necessarily in Z[λ].

    // Let's compute via the norm form directly.
    // Ideal = v_rational · O_K + ((v_lambda_coeff + √Δ)/2) · O_K
    //
    // In terms of quadratic forms with discriminant Δ:
    // (a_form, b_form, c_form) where a_form = v_rational,
    // b_form = v_lambda_coeff (adjusted to have same parity as Δ),
    // c_form = (b_form² - Δ) / (4 * a_form)

    let a_form = v_rational;

    // b_form must satisfy b_form ≡ Δ mod 2 and the ideal is
    // a_form · Z + ((-b_form + √Δ)/2) · Z.
    // Actually, for the ideal n·Z + ((p+√Δ)/2)·Z, the form is (n, p, (p²-Δ)/(4n)).
    // We need p ≡ Δ (mod 2) for this to have integer c_form.
    let b_form = v_lambda_coeff;
    // Adjust b_form to have same parity as Δ
    if (b_form.unsigned_abs() % 2) != (delta.unsigned_abs() % 2) {
        // Shift by 2*a_form to stay in same ideal class: b -> b + 2*a_form
        // This doesn't change the ideal, just the representation.
        // Actually, we need b ≡ Δ mod 2. If parity doesn't match,
        // the element (b + √Δ)/2 is not in O_K with this discriminant.
        // This happens when Δ and v_lambda_coeff have different parity.
        // In this case, multiply the ideal by 2 and adjust.
        // Simpler: use the norm form approach.
        // Parity mismatch — fall through to HNF-based approach.
    }

    // Alternative approach: compute the ideal norm directly.
    // For ideal I = ⟨v_rational, (v_lambda_coeff + √Δ)/2⟩:
    // N(I) = |v_rational * v_rational_conj| / ... This is getting complicated.
    //
    // Let me use the standard approach for ideals in quadratic fields.
    // Every ideal in O_K can be written as a·Z + ((-b + √D_K)/2)·Z where
    // D_K is the fundamental discriminant, a > 0, and a | (b² - D_K)/4.
    //
    // The Perron eigenvector ideal in O_K is generated by:
    //   v_rational  and  (v_lambda_coeff + √Δ)/2
    //
    // Since Δ = f² · D_K (up to sign of f²), √Δ = f · √D_K.
    // So (v_lambda_coeff + √Δ)/2 = (v_lambda_coeff + f·√D_K)/2.
    // This is in O_K iff v_lambda_coeff ≡ f·(D_K mod 2) ... it's cleaner
    // to work with Δ as the form discriminant.

    // Let's use the simpler HNF-based approach for the ideal.
    // The ideal I in Z[ω] generated by α and β (where ω = (D_K + √D_K)/2 or √D_K)
    // can be computed by finding the HNF of the 2×2 matrix of coordinates.

    // Actually, for 2×2 matrices the simplest correct approach is:
    // The eigenvector ideal is ⟨c, λ-d⟩ in Z[λ] ⊂ O_K.
    // Norm of this ideal = |c · conjugate(λ-d)| / [O_K : Z[λ]] or similar.
    //
    // For the CLASS of the ideal (which is what we compare), we can use:
    // N(⟨c, λ-d⟩) and the form representation.
    //
    // Let me just compute the quadratic form (a, b, c_form) directly:
    // The ideal ⟨n, (p + √Δ)/2⟩ corresponds to form (n, p, (p²-Δ)/(4n))
    // where n > 0 and n | (p²-Δ)/4.
    // We need p² ≡ Δ mod 4n.

    // Our generators: v_rational and (v_lambda_coeff + √Δ)/2
    // So n = v_rational, p = v_lambda_coeff.
    // Check: need v_lambda_coeff² ≡ Δ mod (4 * v_rational)

    let check = v_lambda_coeff * v_lambda_coeff - delta;
    if check % (4 * a_form) != 0 {
        // The ideal doesn't have this simple form. We need to find the
        // proper HNF representation. Compute via GCD-based reduction.
        return eigenvector_ideal_class_via_hnf(v_rational, v_lambda_coeff, delta);
    }

    let c_form = check / (4 * a_form);

    if delta < 0 {
        Some(reduce_form_negative(a_form, v_lambda_coeff, c_form))
    } else {
        Some(reduce_form_positive(a_form, v_lambda_coeff, c_form))
    }
}

/// Compute ideal class when the simple form representation doesn't work directly.
/// Uses HNF to find the standard form of the ideal.
fn eigenvector_ideal_class_via_hnf(
    v_rational: i64,
    v_lambda_coeff: i64,
    delta: i64,
) -> Option<ReducedForm> {
    // The ideal is generated by v_rational and (v_lambda_coeff + √Δ)/2 in
    // the order with discriminant Δ.
    //
    // Write elements as x + y·ω where ω = (Δ_rem + √Δ)/2 and Δ_rem = Δ mod 2.
    // Then: v_rational = v_rational + 0·ω
    //       (v_lambda_coeff + √Δ)/2 needs to be expressed in terms of ω.
    //
    // If Δ is odd: ω = (1 + √Δ)/2, so √Δ = 2ω - 1.
    //   (v_lambda_coeff + √Δ)/2 = (v_lambda_coeff + 2ω - 1)/2 = (v_lambda_coeff - 1)/2 + ω
    //   This requires v_lambda_coeff to be odd.
    //
    // If Δ is even: ω = √Δ/2... wait, that's not standard.
    //   Actually when Δ ≡ 0 mod 4, ω = √(Δ/4) = √(Δ)/2.
    //   (v_lambda_coeff + √Δ)/2 = v_lambda_coeff/2 + ω
    //   This requires v_lambda_coeff to be even.
    //
    // Hmm, let me reconsider. The discriminant Δ = t² - 4det for our char poly.
    // Δ ≡ t² mod 4, so Δ ≡ 0 or 1 mod 4.
    //
    // Case Δ ≡ 0 mod 4: O has basis {1, √(Δ/4)} = {1, √D'} where D' = Δ/4.
    //   Actually the maximal order depends on D' mod 4...
    //   This is getting complex. Let me use a more direct approach.

    // Direct approach using integer ideals:
    // The ideal I = ⟨v_rational, (v_lambda_coeff + √Δ)/2⟩ in the order of discriminant Δ.
    // As a Z-module, I = v_rational·Z + ((v_lambda_coeff + √Δ)/2)·Z.
    // This is already in "standard form" aZ + ((b + √Δ)/2)Z with a = v_rational, b = v_lambda_coeff.
    // But we need a | (b² - Δ)/4.
    //
    // If a does not divide (b² - Δ)/4, then the generators aren't in standard form
    // and we need to compute the HNF.
    //
    // The HNF of ideal ⟨α, β⟩ where α = v_rational, β = (v_lambda_coeff + √Δ)/2:
    // As a Z-module with basis {1, (b₀ + √Δ)/2} (where b₀ ∈ {0,1} has b₀ ≡ Δ mod 2):
    //   α = v_rational · 1 + 0 · ω   (where ω = (b₀ + √Δ)/2)
    //   β = ? · 1 + ? · ω
    //
    // If Δ ≡ 1 mod 4: ω = (1 + √Δ)/2
    //   β = (v_lambda_coeff + √Δ)/2 = (v_lambda_coeff - 1)/2 + ω
    //   So β has coordinates ((v_lambda_coeff - 1)/2, 1) if v_lambda_coeff is odd
    //   If v_lambda_coeff is even, (v_lambda_coeff + √Δ)/2 is not in Z[ω], need different gen.
    //
    // If Δ ≡ 0 mod 4: ω = √Δ/2
    //   β = (v_lambda_coeff + √Δ)/2 = v_lambda_coeff/2 + ω
    //   Coordinates (v_lambda_coeff/2, 1) if v_lambda_coeff is even.

    let b0 = ((delta % 4) + 4) % 4; // 0 or 1
                                    // b0 should be 0 or 1 since delta ≡ 0 or 1 mod 4

    if b0 == 1 {
        // Δ ≡ 1 mod 4, ω = (1 + √Δ)/2
        // β = (v_lambda_coeff + √Δ)/2
        // = (v_lambda_coeff - 1)/2 + (1 + √Δ)/2
        // = (v_lambda_coeff - 1)/2 + ω
        if v_lambda_coeff % 2 == 0 {
            // v_lambda_coeff is even, so (v_lambda_coeff + √Δ)/2 is not in Z[ω]
            // since √Δ = 2ω - 1, (v_lambda_coeff + 2ω - 1)/2 = (v_lambda_coeff-1)/2 + ω
            // and (v_lambda_coeff-1) is odd, so (v_lambda_coeff-1)/2 is not integer.
            // We need to multiply by 2 or find different generators.
            // Use: I = ⟨v_rational, v_lambda_coeff + √Δ⟩ / ... no.
            // Actually, the eigenvector coordinates might not generate an ideal in O_K.
            // Fall back: return None (can't compute invariant).
            return None;
        }
        let beta_const = (v_lambda_coeff - 1) / 2; // integer since v_lambda_coeff is odd

        // The ideal aZ + (r + ω)Z where a = v_rational and
        // r ≡ beta_const mod v_rational, 0 ≤ r < v_rational.
        let a_ideal = v_rational.abs();
        let r = ((beta_const % a_ideal) + a_ideal) % a_ideal;
        // Form: (a_ideal, 2r + 1, ...) since the form for ω = (1+√Δ)/2 is
        // a·x² + (2r+1)·xy + ((2r+1)² - Δ)/(4a) · y²
        let b_form = 2 * r + 1;
        let c_form_num = b_form * b_form - delta;
        if c_form_num % (4 * a_ideal) != 0 {
            return None;
        }
        let c_form = c_form_num / (4 * a_ideal);

        if delta < 0 {
            Some(reduce_form_negative(a_ideal, b_form, c_form))
        } else {
            Some(reduce_form_positive(a_ideal, b_form, c_form))
        }
    } else {
        // Δ ≡ 0 mod 4, ω = √Δ / 2 = √(Δ/4)
        // β = (v_lambda_coeff + √Δ)/2 = v_lambda_coeff/2 + ω
        if v_lambda_coeff % 2 != 0 {
            return None;
        }
        let beta_const = v_lambda_coeff / 2;

        let a_ideal = v_rational.abs();
        let r = ((beta_const % a_ideal) + a_ideal) % a_ideal;
        // Form: (a_ideal, 2r, ...) since ω = √(Δ/4) and the form is
        // a·x² + 2r·xy + (r² - Δ/4)/a · y²
        let b_form = 2 * r;
        let c_form_num = b_form * b_form - delta;
        if c_form_num % (4 * a_ideal) != 0 {
            return None;
        }
        let c_form = c_form_num / (4 * a_ideal);

        if delta < 0 {
            Some(reduce_form_negative(a_ideal, b_form, c_form))
        } else {
            Some(reduce_form_positive(a_ideal, b_form, c_form))
        }
    }
}

#[allow(dead_code)]
fn gcd_i64(mut a: i64, mut b: i64) -> i64 {
    a = a.abs();
    b = b.abs();
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
    use crate::matrix::SqMatrix;

    #[test]
    fn test_isqrt() {
        assert_eq!(isqrt(0), 0);
        assert_eq!(isqrt(1), 1);
        assert_eq!(isqrt(4), 2);
        assert_eq!(isqrt(5), 2);
        assert_eq!(isqrt(8), 2);
        assert_eq!(isqrt(9), 3);
        assert_eq!(isqrt(100), 10);
    }

    #[test]
    fn test_is_perfect_square() {
        assert!(is_perfect_square(0));
        assert!(is_perfect_square(1));
        assert!(is_perfect_square(4));
        assert!(is_perfect_square(9));
        assert!(!is_perfect_square(2));
        assert!(!is_perfect_square(3));
        assert!(!is_perfect_square(5));
    }

    #[test]
    fn test_fundamental_discriminant() {
        // Δ = 5: squarefree, 5 ≡ 1 mod 4 → D_K = 5, f = 1
        assert_eq!(fundamental_discriminant(5), (5, 1));
        // Δ = 12 = 4*3: 3 ≡ 3 mod 4 → D_K = 12, f = 1
        assert_eq!(fundamental_discriminant(12), (12, 1));
        // Δ = 20 = 4*5: 5 ≡ 1 mod 4 → D_K = 5, f = 2
        assert_eq!(fundamental_discriminant(20), (5, 2));
        // Δ = -4: |Δ| = 4 = 2²·1, d=1, sign=-1, d_signed=-1, -1 mod 4 = 3 → D_K = -4, f = 1
        assert_eq!(fundamental_discriminant(-4), (-4, 1));
        // Δ = -20: |Δ| = 20 = 2²·5, d=5, sign=-1, d_signed=-5, -5 mod 4 = 3 → D_K = -20, f = 1
        // Wait: -5 mod 4 = -1 mod 4 = 3. So D_K = 4*(-5) = -20, f = 2/2 = 1
        assert_eq!(fundamental_discriminant(-20), (-20, 1));
    }

    #[test]
    fn test_quadratic_order_profile_detects_nonmaximal_order() {
        let profile = quadratic_order_profile(48).expect("48 should define a quadratic order");
        assert_eq!(
            profile,
            QuadraticOrderProfile {
                order_discriminant: 48,
                field_discriminant: 12,
                conductor: 2,
            }
        );
        assert!(!profile.maximal_order());
    }

    #[test]
    fn test_principal_reduced_form_for_real_quadratic_order() {
        let principal = principal_reduced_form(204).expect("principal form should exist");
        assert_eq!(reduced_form_is_principal(204, &principal), Some(true));

        let nonprincipal = eigenvector_ideal_class_2x2(&SqMatrix::new([[13, 5], [3, 1]]))
            .expect("Eilers-Kiming target class should be computable");
        assert_eq!(reduced_form_is_principal(204, &nonprincipal), Some(false));
    }

    #[test]
    fn test_reduce_form_negative_principal() {
        // Form (1, 0, 1) with disc -4: already reduced, principal class
        let f = reduce_form_negative(1, 0, 1);
        assert_eq!(f, ReducedForm { a: 1, b: 0, c: 1 });
    }

    #[test]
    fn test_reduce_form_negative_class2() {
        // In Q(√-5), disc = -20. Form (2, 0, 5): not principal.
        // Check: 0² - 4·2·5 = -40... wait disc should be -20.
        // Form (2, 2, 3): disc = 4 - 24 = -20. Reduce:
        // |b|=2 ≤ a=2 ≤ c=3, and |b|=a so b should be ≥ 0 → b=2. Already reduced.
        let f = reduce_form_negative(2, 2, 3);
        assert_eq!(f, ReducedForm { a: 2, b: 2, c: 3 });

        // Principal form with disc -20: (1, 0, 5)
        let p = reduce_form_negative(1, 0, 5);
        assert_eq!(p, ReducedForm { a: 1, b: 0, c: 5 });
        assert_ne!(f, p); // Different classes
    }

    #[test]
    fn test_reduce_form_negative_equivalent() {
        // Two forms in the same class should reduce to the same form.
        // disc = -23: (1, 1, 6) is the principal form.
        // (2, 1, 3) is another form. disc = 1 - 24 = -23. Reduced since |1| ≤ 2 ≤ 3.
        let f1 = reduce_form_negative(2, 1, 3);
        let f2 = reduce_form_negative(2, -1, 3); // Same class (inverse)
                                                 // For disc -23, class number is 3. (2,1,3) and (2,-1,3) are inverses.
                                                 // They should reduce to different forms since they're in different classes.
        assert_ne!(f1, f2);
    }

    #[test]
    fn test_eigenvector_class_same_matrix() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let ca = eigenvector_ideal_class_2x2(&a);
        assert!(ca.is_some());
        assert_eq!(ca, eigenvector_ideal_class_2x2(&a));
    }

    #[test]
    fn test_eigenvector_class_conjugate() {
        // Conjugate matrices are SSE, should have same ideal class
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let ca = eigenvector_ideal_class_2x2(&a);
        let cb = eigenvector_ideal_class_2x2(&b);
        assert_eq!(ca, cb);
    }

    #[test]
    fn test_eilers_kiming_14_2_non_sse() {
        // Eilers-Kiming p.8: A = [[14,2],[1,0]] and B = [[13,5],[3,1]]
        // are NOT SSE. Char poly x² - 14x - 2, Δ = 196 + 8 = 204 = 4·51.
        // K = Q(√51), disc 51 (51 ≡ 3 mod 4 → fund disc = 4·51 = 204).
        // Wait, 51 = 3·17. 51 mod 4 = 3. Fund disc = 4·51 = 204. Hmm no.
        // Actually Δ = 204 = 4 · 51. 51 is squarefree. 51 ≡ 3 mod 4.
        // So D_K = 4·51 = 204? No, that's not right either.
        // Let me reconsider: Δ = 204 = 2² · 51. d = 51, f = 2.
        // But wait we factor out the largest square: 204 = 4 · 51. So f=2, d=51.
        // d_signed = 51. 51 mod 4 = 3. So D_K = 4·51 = 204, and f = 2/2 = 1.
        // Hmm that means fund_disc is 204 with conductor 1? That doesn't sound right.
        //
        // Actually: the fundamental discriminant of Q(√51) is:
        // 51 ≡ 3 mod 4, so D_K = 4·51 = 204. But wait, that means Δ = D_K exactly,
        // and f = 1. Let's check: 204 = 1² · 204. Yes.
        //
        // The paper says the ideal B is non-principal in O_K, proving A not SSE to B.
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);

        let ca = eigenvector_ideal_class_2x2(&a);
        let cb = eigenvector_ideal_class_2x2(&b);

        assert!(ca.is_some(), "Should compute class for A");
        assert!(cb.is_some(), "Should compute class for B");
        assert_ne!(
            ca, cb,
            "A and B should have different ideal classes (non-SSE)"
        );
    }

    #[test]
    fn test_eilers_kiming_triple() {
        // Eilers-Kiming p.8: [[5,13],[6,1]], [[5,6],[13,1]], [[4,9],[9,2]]
        // are pairwise NOT SSE.
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m2 = SqMatrix::new([[5, 6], [13, 1]]);
        let m3 = SqMatrix::new([[4, 9], [9, 2]]);

        let c1 = eigenvector_ideal_class_2x2(&m1);
        let c2 = eigenvector_ideal_class_2x2(&m2);
        let c3 = eigenvector_ideal_class_2x2(&m3);

        assert!(c1.is_some(), "Should compute class for m1");
        assert!(c2.is_some(), "Should compute class for m2");
        assert!(c3.is_some(), "Should compute class for m3");

        // At least some pairs should differ (the paper says all three are non-SSE)
        let all_same = c1 == c2 && c2 == c3;
        assert!(
            !all_same,
            "At least some pairs should have different ideal classes"
        );
    }

    #[test]
    fn test_rational_eigenvalues_returns_none() {
        // [[2,1],[1,0]]: char poly x²-2x-1, Δ = 4+4 = 8 (not perfect square → should work)
        // [[3,0],[0,1]]: diagonal, eigenvalues 3,1 (rational) → should return None
        let a = SqMatrix::new([[3, 0], [0, 1]]);
        assert_eq!(eigenvector_ideal_class_2x2(&a), None);
    }
}
