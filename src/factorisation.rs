use crate::matrix::{DynMatrix, SqMatrix};

/// Enumerate all 2x2 nonneg integer factorisations A = UV where U and V are 2x2
/// with entries in 0..=max_entry and det(U) > 0.
///
/// Algorithm: iterate over all 2x2 nonneg U with positive determinant,
/// compute V = adj(U) * A / det(U), check that all entries are nonneg integers
/// within the entry bound.
pub fn enumerate_square_factorisations_2x2(
    a: &SqMatrix<2>,
    max_entry: u32,
) -> Vec<(DynMatrix, DynMatrix)> {
    let mut results = Vec::new();
    let [[a00, a01], [a10, a11]] = a.data;

    let me = max_entry as i64;

    for u00 in 0..=max_entry {
        for u01 in 0..=max_entry {
            for u10 in 0..=max_entry {
                for u11 in 0..=max_entry {
                    let det = u00 as i64 * u11 as i64 - u01 as i64 * u10 as i64;
                    if det <= 0 {
                        continue;
                    }

                    // adj(U) = [[u11, -u01], [-u10, u00]]
                    // V = adj(U) * A / det
                    // v00 = (u11*a00 - u01*a10) / det
                    // v01 = (u11*a01 - u01*a11) / det
                    // v10 = (-u10*a00 + u00*a10) / det
                    // v11 = (-u10*a01 + u00*a11) / det

                    let v00_num = u11 as i64 * a00 as i64 - u01 as i64 * a10 as i64;
                    if v00_num < 0 || v00_num % det != 0 {
                        continue;
                    }
                    let v00 = v00_num / det;
                    if v00 > me {
                        continue;
                    }

                    let v01_num = u11 as i64 * a01 as i64 - u01 as i64 * a11 as i64;
                    if v01_num < 0 || v01_num % det != 0 {
                        continue;
                    }
                    let v01 = v01_num / det;
                    if v01 > me {
                        continue;
                    }

                    let v10_num = -(u10 as i64) * a00 as i64 + u00 as i64 * a10 as i64;
                    if v10_num < 0 || v10_num % det != 0 {
                        continue;
                    }
                    let v10 = v10_num / det;
                    if v10 > me {
                        continue;
                    }

                    let v11_num = -(u10 as i64) * a01 as i64 + u00 as i64 * a11 as i64;
                    if v11_num < 0 || v11_num % det != 0 {
                        continue;
                    }
                    let v11 = v11_num / det;
                    if v11 > me {
                        continue;
                    }

                    let u = DynMatrix::new(2, 2, vec![u00, u01, u10, u11]);
                    let v = DynMatrix::new(
                        2,
                        2,
                        vec![v00 as u32, v01 as u32, v10 as u32, v11 as u32],
                    );
                    results.push((u, v));
                }
            }
        }
    }
    results
}

/// Compute VU as a SqMatrix<2> given DynMatrix factors.
pub fn vu_product_2x2(v: &DynMatrix, u: &DynMatrix) -> SqMatrix<2> {
    let vu = v.mul(u);
    vu.to_sq::<2>().expect("VU product should be 2x2")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_factorisations_identity() {
        let id = SqMatrix::new([[1, 0], [0, 1]]);
        let facts = enumerate_square_factorisations_2x2(&id, 5);
        // Every factorisation should satisfy UV = I
        for (u, v) in &facts {
            let uv = u.mul(v);
            assert_eq!(uv, DynMatrix::from_sq(&id));
        }
        // Identity = UV means V = U^{-1}, so U must be a nonneg integer matrix
        // with nonneg integer inverse. These are exactly the permutation matrices
        // and the matrices [[1,k],[0,1]], [[1,0],[k,1]] with k >= 0, plus
        // their permutation conjugates. With max_entry=5, there should be many.
        assert!(!facts.is_empty());
    }

    #[test]
    fn test_factorisations_verify_product() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let facts = enumerate_square_factorisations_2x2(&a, 5);
        assert!(!facts.is_empty());
        for (u, v) in &facts {
            let uv = u.mul(v);
            assert_eq!(uv, DynMatrix::from_sq(&a), "UV != A for U={:?}, V={:?}", u, v);
        }
    }

    #[test]
    fn test_known_factorisation() {
        // [[2,1],[1,1]] = [[1,1],[0,1]] * [[1,0],[1,1]]
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let facts = enumerate_square_factorisations_2x2(&a, 5);
        let u_expected = DynMatrix::new(2, 2, vec![1, 1, 0, 1]);
        let v_expected = DynMatrix::new(2, 2, vec![1, 0, 1, 1]);
        assert!(
            facts.contains(&(u_expected, v_expected)),
            "Expected factorisation not found"
        );
    }

    #[test]
    fn test_vu_product() {
        let u = DynMatrix::new(2, 2, vec![1, 1, 0, 1]);
        let v = DynMatrix::new(2, 2, vec![1, 0, 1, 1]);
        let vu = vu_product_2x2(&v, &u);
        assert_eq!(vu, SqMatrix::new([[1, 1], [1, 2]]));
    }
}
