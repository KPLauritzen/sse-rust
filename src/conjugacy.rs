use crate::matrix::SqMatrix;

/// Configuration for bounded positive-conjugacy search on 2x2 matrices.
///
/// This is an experimental search substrate inspired by path methods for
/// positive matrices. It is intentionally separate from the integer SSE solver:
/// finding a positive conjugacy path is evidence and a source of candidate moves,
/// not a proof of SSE over `Z_+`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositiveConjugacySearchConfig2x2 {
    /// Maximum entry allowed in an integer conjugator candidate `G`.
    pub max_conjugator_entry: u32,
    /// Number of evenly spaced interior samples used to validate the affine path
    /// `H(t) = (1-t)I + tG`.
    pub sample_points: usize,
}

impl Default for PositiveConjugacySearchConfig2x2 {
    fn default() -> Self {
        Self {
            max_conjugator_entry: 8,
            sample_points: 64,
        }
    }
}

/// A 2x2 real matrix used to record sampled matrices along a positive path.
#[derive(Clone, Debug, PartialEq)]
pub struct RealMatrix2x2 {
    pub data: [[f64; 2]; 2],
}

impl RealMatrix2x2 {
    fn identity() -> Self {
        Self {
            data: [[1.0, 0.0], [0.0, 1.0]],
        }
    }

    fn determinant(&self) -> f64 {
        self.data[0][0] * self.data[1][1] - self.data[0][1] * self.data[1][0]
    }

    fn min_entry(&self) -> f64 {
        self.data
            .iter()
            .flat_map(|row| row.iter())
            .copied()
            .fold(f64::INFINITY, f64::min)
    }

    fn inverse(&self) -> Option<Self> {
        let det = self.determinant();
        if det.abs() <= 1e-12 {
            return None;
        }
        Some(Self {
            data: [
                [self.data[1][1] / det, -self.data[0][1] / det],
                [-self.data[1][0] / det, self.data[0][0] / det],
            ],
        })
    }

    fn mul(&self, other: &Self) -> Self {
        let a = self.data;
        let b = other.data;
        Self {
            data: [
                [
                    a[0][0] * b[0][0] + a[0][1] * b[1][0],
                    a[0][0] * b[0][1] + a[0][1] * b[1][1],
                ],
                [
                    a[1][0] * b[0][0] + a[1][1] * b[1][0],
                    a[1][0] * b[0][1] + a[1][1] * b[1][1],
                ],
            ],
        }
    }

    fn from_sq(m: &SqMatrix<2>) -> Self {
        let [[a, b], [c, d]] = m.data;
        Self {
            data: [[a as f64, b as f64], [c as f64, d as f64]],
        }
    }
}

/// Witness data for a sampled positive conjugacy path.
#[derive(Clone, Debug, PartialEq)]
pub struct PositiveConjugacyWitness2x2 {
    pub conjugator: SqMatrix<2>,
    pub sampled_path: Vec<RealMatrix2x2>,
}

/// Result of bounded positive-conjugacy search.
#[derive(Clone, Debug, PartialEq)]
pub enum PositiveConjugacySearchResult2x2 {
    Equivalent(PositiveConjugacyWitness2x2),
    Exhausted,
}

/// Search for a bounded positive-conjugacy witness between two 2x2 matrices.
///
/// The search enumerates small integer matrices `G` with positive determinant,
/// checks the exact intertwining equation `AG = GB`, and then validates that the
/// affine path `H(t) = (1-t)I + tG` stays inside `GL^+(2,R)` and yields positive
/// conjugates `H(t)^-1 A H(t)` at a fixed sampling resolution.
pub fn find_positive_conjugacy_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &PositiveConjugacySearchConfig2x2,
) -> PositiveConjugacySearchResult2x2 {
    let a_real = RealMatrix2x2::from_sq(a);
    let b_real = RealMatrix2x2::from_sq(b);

    for g00 in 0..=config.max_conjugator_entry {
        for g01 in 0..=config.max_conjugator_entry {
            for g10 in 0..=config.max_conjugator_entry {
                for g11 in 0..=config.max_conjugator_entry {
                    let g = SqMatrix::new([[g00, g01], [g10, g11]]);
                    let det = g.det();
                    if det <= 0 {
                        continue;
                    }
                    if a.mul_u32(&g) != g.mul_u32(b) {
                        continue;
                    }

                    let sampled_path = match sample_affine_positive_path(&a_real, &g, config) {
                        Some(path) => path,
                        None => continue,
                    };

                    if sampled_path
                        .last()
                        .map(|last| max_abs_diff(last, &b_real) <= 1e-9)
                        .unwrap_or(false)
                    {
                        return PositiveConjugacySearchResult2x2::Equivalent(
                            PositiveConjugacyWitness2x2 {
                                conjugator: g,
                                sampled_path,
                            },
                        );
                    }
                }
            }
        }
    }

    PositiveConjugacySearchResult2x2::Exhausted
}

fn sample_affine_positive_path(
    a: &RealMatrix2x2,
    conjugator: &SqMatrix<2>,
    config: &PositiveConjugacySearchConfig2x2,
) -> Option<Vec<RealMatrix2x2>> {
    let g = RealMatrix2x2::from_sq(conjugator);
    let i = RealMatrix2x2::identity();
    let mut path = Vec::with_capacity(config.sample_points + 1);

    for sample_idx in 0..=config.sample_points {
        let t = if config.sample_points == 0 {
            1.0
        } else {
            sample_idx as f64 / config.sample_points as f64
        };
        let h = RealMatrix2x2 {
            data: [
                [
                    (1.0 - t) * i.data[0][0] + t * g.data[0][0],
                    (1.0 - t) * i.data[0][1] + t * g.data[0][1],
                ],
                [
                    (1.0 - t) * i.data[1][0] + t * g.data[1][0],
                    (1.0 - t) * i.data[1][1] + t * g.data[1][1],
                ],
            ],
        };
        if h.determinant() <= 1e-12 {
            return None;
        }
        let h_inv = h.inverse()?;
        let conjugated = h_inv.mul(a).mul(&h);
        if conjugated.min_entry() <= 1e-9 {
            return None;
        }
        path.push(conjugated);
    }

    Some(path)
}

fn max_abs_diff(a: &RealMatrix2x2, b: &RealMatrix2x2) -> f64 {
    let mut max = 0.0f64;
    for i in 0..2 {
        for j in 0..2 {
            max = max.max((a.data[i][j] - b.data[i][j]).abs());
        }
    }
    max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_brix_ruiz_k3_positive_conjugacy_search() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let result = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 4,
                sample_points: 64,
            },
        );

        match result {
            PositiveConjugacySearchResult2x2::Equivalent(witness) => {
                assert_eq!(witness.conjugator, SqMatrix::new([[1, 0], [0, 2]]));
                assert!(!witness.sampled_path.is_empty());
                assert!(witness.sampled_path.iter().all(|m| m.min_entry() > 0.0));
            }
            PositiveConjugacySearchResult2x2::Exhausted => {
                panic!("expected a positive conjugacy witness for the k=3 pair")
            }
        }
    }

    #[test]
    fn test_brix_ruiz_k4_positive_conjugacy_search() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        let result = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 5,
                sample_points: 64,
            },
        );

        match result {
            PositiveConjugacySearchResult2x2::Equivalent(witness) => {
                assert_eq!(witness.conjugator, SqMatrix::new([[1, 0], [0, 3]]));
            }
            PositiveConjugacySearchResult2x2::Exhausted => {
                panic!("expected a positive conjugacy witness for the k=4 pair")
            }
        }
    }

    #[test]
    fn test_positive_conjugacy_search_exhausts_on_non_similar_pair() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[3, 1], [1, 1]]);
        let result = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 4,
                sample_points: 32,
            },
        );
        assert_eq!(result, PositiveConjugacySearchResult2x2::Exhausted);
    }
}
