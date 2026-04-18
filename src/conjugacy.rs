use std::collections::BTreeMap;

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

/// Configuration for extracting bounded discrete proposals from a sampled
/// positive-conjugacy witness.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositiveConjugacyProposalConfig2x2 {
    /// Maximum number of ranked proposals to return after deduplication.
    pub max_proposals: usize,
    /// Whether to retain the endpoint matrices in the proposal set.
    pub include_endpoints: bool,
}

impl Default for PositiveConjugacyProposalConfig2x2 {
    fn default() -> Self {
        Self {
            max_proposals: 8,
            include_endpoints: false,
        }
    }
}

/// Configuration for ranking actual local move candidates against sampled
/// positive-conjugacy proposals.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositiveConjugacySeedConfig2x2 {
    /// Maximum number of ranked local seeds to return after deduplication.
    pub max_candidates: usize,
}

impl Default for PositiveConjugacySeedConfig2x2 {
    fn default() -> Self {
        Self { max_candidates: 4 }
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

    fn entrywise_l1_to_sq(&self, other: &SqMatrix<2>) -> f64 {
        let mut total = 0.0;
        for i in 0..2 {
            for j in 0..2 {
                total += (self.data[i][j] - other.data[i][j] as f64).abs();
            }
        }
        total
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

/// Current discrete proposal family derived from a sampled positive-conjugacy
/// witness. Phase 1 uses entrywise floor/ceil shadows of sampled positive
/// matrices as small nearby waypoint candidates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PositiveConjugacyProposalKind2x2 {
    RoundedSampleWaypoint,
    InvariantCompatibleReprojection,
}

/// Ranked discrete proposal derived from a sampled positive-conjugacy witness.
#[derive(Clone, Debug, PartialEq)]
pub struct PositiveConjugacyProposal2x2 {
    pub matrix: SqMatrix<2>,
    pub kind: PositiveConjugacyProposalKind2x2,
    pub nearest_sample_index: usize,
    pub nearest_sample_t: f64,
    pub shadow_l1_distance: f64,
    pub endpoint_l1_distance: u32,
    pub preserves_endpoint_diagonal: bool,
    pub stays_within_endpoint_box: bool,
}

/// Ranked actual local move candidate scored against the sampled
/// positive-conjugacy proposal surface.
///
/// Unlike [`PositiveConjugacyProposal2x2`], this is intended for genuine
/// search-side candidates such as one-step SSE successors, not for treating the
/// rounded sampled matrices as exact waypoint targets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PositiveConjugacySeedCandidate2x2 {
    pub matrix: SqMatrix<2>,
    pub nearest_proposal_rank: usize,
    pub proposal_l1_distance: u32,
    pub target_l1_distance: u32,
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

/// Turn a sampled positive-conjugacy witness into a small ranked set of nearby
/// discrete waypoint proposals.
///
/// Phase 1 keeps the proposal object intentionally narrow: we round each
/// sampled positive matrix entrywise via floor/ceil, keep the positive integer
/// shadows, deduplicate them, and rank them by nearest-sample `L1` distance.
/// This treats positive conjugacy as a proposal source rather than as a proof.
pub fn derive_positive_conjugacy_proposals_2x2(
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
    witness: &PositiveConjugacyWitness2x2,
    config: &PositiveConjugacyProposalConfig2x2,
) -> Vec<PositiveConjugacyProposal2x2> {
    if config.max_proposals == 0 || witness.sampled_path.is_empty() {
        return Vec::new();
    }

    #[derive(Clone, Copy)]
    struct BestShadow {
        nearest_sample_index: usize,
        shadow_l1_distance: f64,
    }

    let mut best_by_matrix: BTreeMap<SqMatrix<2>, BestShadow> = BTreeMap::new();

    for (sample_index, sample) in witness.sampled_path.iter().enumerate() {
        let choices = [
            entry_rounding_choices(sample.data[0][0]),
            entry_rounding_choices(sample.data[0][1]),
            entry_rounding_choices(sample.data[1][0]),
            entry_rounding_choices(sample.data[1][1]),
        ];

        for &m00 in &choices[0] {
            for &m01 in &choices[1] {
                for &m10 in &choices[2] {
                    for &m11 in &choices[3] {
                        let candidate = SqMatrix::new([[m00, m01], [m10, m11]]);
                        if !candidate.data.iter().flatten().all(|entry| *entry > 0) {
                            continue;
                        }
                        if !config.include_endpoints
                            && (&candidate == source || &candidate == target)
                        {
                            continue;
                        }

                        let shadow_l1_distance = sample.entrywise_l1_to_sq(&candidate);
                        match best_by_matrix.get_mut(&candidate) {
                            Some(best) if shadow_l1_distance < best.shadow_l1_distance => {
                                *best = BestShadow {
                                    nearest_sample_index: sample_index,
                                    shadow_l1_distance,
                                };
                            }
                            None => {
                                best_by_matrix.insert(
                                    candidate,
                                    BestShadow {
                                        nearest_sample_index: sample_index,
                                        shadow_l1_distance,
                                    },
                                );
                            }
                            _ => {}
                        }
                    }
                }
            }
        }
    }

    let sample_count = witness.sampled_path.len();
    let mut proposals = best_by_matrix
        .into_iter()
        .map(|(matrix, best)| PositiveConjugacyProposal2x2 {
            endpoint_l1_distance: l1_distance_to_nearest_endpoint(&matrix, source, target),
            preserves_endpoint_diagonal: matrix.data[0][0] == source.data[0][0]
                && matrix.data[0][0] == target.data[0][0]
                && matrix.data[1][1] == source.data[1][1]
                && matrix.data[1][1] == target.data[1][1],
            stays_within_endpoint_box: stays_within_endpoint_box(&matrix, source, target),
            nearest_sample_index: best.nearest_sample_index,
            nearest_sample_t: sample_parameter(best.nearest_sample_index, sample_count),
            shadow_l1_distance: best.shadow_l1_distance,
            kind: PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
            matrix,
        })
        .collect::<Vec<_>>();

    proposals.sort_by(|left, right| {
        left.shadow_l1_distance
            .total_cmp(&right.shadow_l1_distance)
            .then(left.endpoint_l1_distance.cmp(&right.endpoint_l1_distance))
            .then(left.matrix.max_entry().cmp(&right.matrix.max_entry()))
            .then(left.matrix.cmp(&right.matrix))
    });
    proposals.truncate(config.max_proposals);
    proposals
}

/// Reproject a sampled positive-conjugacy witness onto exact `2x2` integer
/// candidates that already match the endpoints' cheap arithmetic invariants.
///
/// This stays in the evidence lane: we still use sampled positive conjugacy as a
/// proposal source, but instead of retaining raw floor/ceil shadows we snap
/// each sampled real matrix to the nearest positive integer matrix with the
/// endpoints' shared trace/determinant data. When the endpoints share both
/// diagonal entries, the reprojection keeps that diagonal pair exactly; in the
/// general `2x2` case it keeps the shared trace and determinant.
pub fn derive_invariant_compatible_positive_conjugacy_proposals_2x2(
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
    witness: &PositiveConjugacyWitness2x2,
    config: &PositiveConjugacyProposalConfig2x2,
) -> Vec<PositiveConjugacyProposal2x2> {
    if config.max_proposals == 0 || witness.sampled_path.is_empty() {
        return Vec::new();
    }

    let invariant_candidates = enumerate_invariant_compatible_candidates_2x2(source, target)
        .into_iter()
        .filter(|candidate| {
            config.include_endpoints || (candidate != source && candidate != target)
        })
        .collect::<Vec<_>>();
    if invariant_candidates.is_empty() {
        return Vec::new();
    }

    #[derive(Clone, Copy)]
    struct BestProjection {
        nearest_sample_index: usize,
        shadow_l1_distance: f64,
    }

    let mut best_by_matrix: BTreeMap<SqMatrix<2>, BestProjection> = BTreeMap::new();
    for (sample_index, sample) in witness.sampled_path.iter().enumerate() {
        let Some((candidate, shadow_l1_distance)) = invariant_candidates
            .iter()
            .map(|candidate| (candidate, sample.entrywise_l1_to_sq(candidate)))
            .min_by(|left, right| {
                left.1
                    .total_cmp(&right.1)
                    .then(
                        l1_distance_to_nearest_endpoint(left.0, source, target)
                            .cmp(&l1_distance_to_nearest_endpoint(right.0, source, target)),
                    )
                    .then(left.0.max_entry().cmp(&right.0.max_entry()))
                    .then(left.0.cmp(right.0))
            })
        else {
            continue;
        };

        match best_by_matrix.get_mut(candidate) {
            Some(best) if shadow_l1_distance < best.shadow_l1_distance => {
                *best = BestProjection {
                    nearest_sample_index: sample_index,
                    shadow_l1_distance,
                };
            }
            None => {
                best_by_matrix.insert(
                    candidate.clone(),
                    BestProjection {
                        nearest_sample_index: sample_index,
                        shadow_l1_distance,
                    },
                );
            }
            _ => {}
        }
    }

    let sample_count = witness.sampled_path.len();
    let mut proposals = best_by_matrix
        .into_iter()
        .map(|(matrix, best)| PositiveConjugacyProposal2x2 {
            endpoint_l1_distance: l1_distance_to_nearest_endpoint(&matrix, source, target),
            preserves_endpoint_diagonal: matrix.data[0][0] == source.data[0][0]
                && matrix.data[0][0] == target.data[0][0]
                && matrix.data[1][1] == source.data[1][1]
                && matrix.data[1][1] == target.data[1][1],
            stays_within_endpoint_box: stays_within_endpoint_box(&matrix, source, target),
            nearest_sample_index: best.nearest_sample_index,
            nearest_sample_t: sample_parameter(best.nearest_sample_index, sample_count),
            shadow_l1_distance: best.shadow_l1_distance,
            kind: PositiveConjugacyProposalKind2x2::InvariantCompatibleReprojection,
            matrix,
        })
        .collect::<Vec<_>>();

    proposals.sort_by(|left, right| {
        left.shadow_l1_distance
            .total_cmp(&right.shadow_l1_distance)
            .then(left.endpoint_l1_distance.cmp(&right.endpoint_l1_distance))
            .then(left.matrix.max_entry().cmp(&right.matrix.max_entry()))
            .then(left.matrix.cmp(&right.matrix))
    });
    proposals.truncate(config.max_proposals);
    proposals
}

/// Produce a tiny ranked exact seed-hint list for the current `2x2` endpoints.
///
/// This is intentionally a search-ordering seam, not a proof surface: it runs a
/// bounded sampled positive-conjugacy search, reprojects the sampled path onto
/// invariant-compatible exact proposals, and then ranks exact candidate
/// matrices against those proposals.
pub fn derive_invariant_compatible_positive_conjugacy_seed_hints_2x2(
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
    candidates: &[SqMatrix<2>],
    witness_config: &PositiveConjugacySearchConfig2x2,
    proposal_config: &PositiveConjugacyProposalConfig2x2,
    seed_config: &PositiveConjugacySeedConfig2x2,
) -> Vec<PositiveConjugacySeedCandidate2x2> {
    if candidates.is_empty()
        || proposal_config.max_proposals == 0
        || seed_config.max_candidates == 0
    {
        return Vec::new();
    }

    let PositiveConjugacySearchResult2x2::Equivalent(witness) =
        find_positive_conjugacy_2x2(source, target, witness_config)
    else {
        return Vec::new();
    };

    let proposals = derive_invariant_compatible_positive_conjugacy_proposals_2x2(
        source,
        target,
        &witness,
        proposal_config,
    );
    if proposals.is_empty() {
        return Vec::new();
    }

    rank_positive_conjugacy_seed_candidates_2x2(target, &proposals, candidates, seed_config)
}

/// Rank actual local 2x2 move candidates by proximity to sampled
/// positive-conjugacy proposals.
///
/// This is the intended phase-3 reinterpretation of the proposal surface:
/// proposals remain approximate evidence, while real one-step candidates are
/// scored by how well they align with that surface.
pub fn rank_positive_conjugacy_seed_candidates_2x2(
    target: &SqMatrix<2>,
    proposals: &[PositiveConjugacyProposal2x2],
    candidates: &[SqMatrix<2>],
    config: &PositiveConjugacySeedConfig2x2,
) -> Vec<PositiveConjugacySeedCandidate2x2> {
    if config.max_candidates == 0 || proposals.is_empty() || candidates.is_empty() {
        return Vec::new();
    }

    #[derive(Clone, Copy)]
    struct BestSeed {
        nearest_proposal_rank: usize,
        proposal_l1_distance: u32,
    }

    let mut best_by_matrix: BTreeMap<SqMatrix<2>, BestSeed> = BTreeMap::new();
    for candidate in candidates {
        let Some((nearest_proposal_rank, proposal_l1_distance)) = proposals
            .iter()
            .enumerate()
            .map(|(index, proposal)| (index + 1, matrix_l1_distance(candidate, &proposal.matrix)))
            .min_by(|left, right| left.1.cmp(&right.1).then(left.0.cmp(&right.0)))
        else {
            continue;
        };

        best_by_matrix.entry(candidate.clone()).or_insert(BestSeed {
            nearest_proposal_rank,
            proposal_l1_distance,
        });
    }

    let mut ranked = best_by_matrix
        .into_iter()
        .map(|(matrix, best)| PositiveConjugacySeedCandidate2x2 {
            target_l1_distance: matrix_l1_distance(&matrix, target),
            matrix,
            nearest_proposal_rank: best.nearest_proposal_rank,
            proposal_l1_distance: best.proposal_l1_distance,
        })
        .collect::<Vec<_>>();

    ranked.sort_by(|left, right| {
        left.proposal_l1_distance
            .cmp(&right.proposal_l1_distance)
            .then(left.nearest_proposal_rank.cmp(&right.nearest_proposal_rank))
            .then(left.target_l1_distance.cmp(&right.target_l1_distance))
            .then(left.matrix.max_entry().cmp(&right.matrix.max_entry()))
            .then(left.matrix.cmp(&right.matrix))
    });
    ranked.truncate(config.max_candidates);
    ranked
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

fn entry_rounding_choices(value: f64) -> Vec<u32> {
    assert!(value.is_finite(), "sampled entries must be finite");
    let floor = value.floor().max(0.0) as u32;
    let ceil = value.ceil().max(0.0) as u32;
    if floor == ceil {
        vec![floor]
    } else {
        vec![floor, ceil]
    }
}

fn sample_parameter(sample_index: usize, sample_count: usize) -> f64 {
    if sample_count <= 1 {
        0.0
    } else {
        sample_index as f64 / (sample_count - 1) as f64
    }
}

fn l1_distance_to_nearest_endpoint(
    matrix: &SqMatrix<2>,
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
) -> u32 {
    matrix_l1_distance(matrix, source).min(matrix_l1_distance(matrix, target))
}

fn matrix_l1_distance(left: &SqMatrix<2>, right: &SqMatrix<2>) -> u32 {
    let mut total = 0u32;
    for i in 0..2 {
        for j in 0..2 {
            total += left.data[i][j].abs_diff(right.data[i][j]);
        }
    }
    total
}

fn stays_within_endpoint_box(
    matrix: &SqMatrix<2>,
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
) -> bool {
    for i in 0..2 {
        for j in 0..2 {
            let min_entry = source.data[i][j].min(target.data[i][j]);
            let max_entry = source.data[i][j].max(target.data[i][j]);
            if matrix.data[i][j] < min_entry || matrix.data[i][j] > max_entry {
                return false;
            }
        }
    }
    true
}

fn enumerate_invariant_compatible_candidates_2x2(
    source: &SqMatrix<2>,
    target: &SqMatrix<2>,
) -> Vec<SqMatrix<2>> {
    if source.trace() != target.trace() || source.det() != target.det() {
        return Vec::new();
    }

    let mut candidates = BTreeMap::<SqMatrix<2>, ()>::new();

    if source.data[0][0] == target.data[0][0] && source.data[1][1] == target.data[1][1] {
        let diag00 = source.data[0][0];
        let diag11 = source.data[1][1];
        let product = i128::from(diag00) * i128::from(diag11) - i128::from(source.det());
        if let Ok(product) = u64::try_from(product) {
            let mut upper_right = 1u64;
            while upper_right * upper_right <= product {
                if product % upper_right == 0 {
                    let lower_left = product / upper_right;
                    for (upper_right, lower_left) in
                        [(upper_right, lower_left), (lower_left, upper_right)]
                    {
                        let (Ok(upper_right), Ok(lower_left)) =
                            (u32::try_from(upper_right), u32::try_from(lower_left))
                        else {
                            continue;
                        };
                        candidates.insert(
                            SqMatrix::new([[diag00, upper_right], [lower_left, diag11]]),
                            (),
                        );
                    }
                }
                upper_right += 1;
            }
        }
    } else {
        let trace = source.trace();
        let det = source.det();
        for upper_left in 1..trace {
            let lower_right = trace - upper_left;
            let product = i128::from(upper_left) * i128::from(lower_right) - i128::from(det);
            let Ok(product) = u64::try_from(product) else {
                continue;
            };

            let mut upper_right = 1u64;
            while upper_right * upper_right <= product {
                if product % upper_right == 0 {
                    let lower_left = product / upper_right;
                    for (upper_right, lower_left) in
                        [(upper_right, lower_left), (lower_left, upper_right)]
                    {
                        let (Ok(upper_left), Ok(upper_right), Ok(lower_left), Ok(lower_right)) = (
                            u32::try_from(upper_left),
                            u32::try_from(upper_right),
                            u32::try_from(lower_left),
                            u32::try_from(lower_right),
                        ) else {
                            continue;
                        };
                        candidates.insert(
                            SqMatrix::new([[upper_left, upper_right], [lower_left, lower_right]]),
                            (),
                        );
                    }
                }
                upper_right += 1;
            }
        }
    }

    candidates.into_keys().collect()
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

    #[test]
    fn test_brix_ruiz_k3_generates_ranked_waypoint_proposals() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let PositiveConjugacySearchResult2x2::Equivalent(witness) = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 4,
                sample_points: 64,
            },
        ) else {
            panic!("expected a witness for the k=3 pair");
        };

        let proposals = derive_positive_conjugacy_proposals_2x2(
            &a,
            &b,
            &witness,
            &PositiveConjugacyProposalConfig2x2 {
                max_proposals: 4,
                include_endpoints: false,
            },
        );

        let proposal_matrices = proposals
            .iter()
            .map(|proposal| proposal.matrix.clone())
            .collect::<Vec<_>>();
        assert_eq!(
            proposal_matrices,
            vec![
                SqMatrix::new([[1, 5], [1, 1]]),
                SqMatrix::new([[1, 4], [2, 1]]),
                SqMatrix::new([[1, 4], [1, 1]]),
                SqMatrix::new([[1, 5], [2, 1]]),
            ]
        );
        assert!(proposals
            .iter()
            .all(|proposal| proposal.preserves_endpoint_diagonal));
        assert!(proposals
            .iter()
            .all(|proposal| proposal.stays_within_endpoint_box));
        assert!(proposals[0].shadow_l1_distance > 0.0);
    }

    #[test]
    fn test_brix_ruiz_k4_generates_exact_interior_shadow() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        let PositiveConjugacySearchResult2x2::Equivalent(witness) = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 5,
                sample_points: 64,
            },
        ) else {
            panic!("expected a witness for the k=4 pair");
        };

        let proposals = derive_positive_conjugacy_proposals_2x2(
            &a,
            &b,
            &witness,
            &PositiveConjugacyProposalConfig2x2 {
                max_proposals: 4,
                include_endpoints: false,
            },
        );

        assert_eq!(proposals[0].matrix, SqMatrix::new([[1, 6], [2, 1]]));
        assert_eq!(proposals[0].shadow_l1_distance, 0.0);
        assert!(proposals
            .iter()
            .all(|proposal| proposal.preserves_endpoint_diagonal));
    }

    #[test]
    fn test_brix_ruiz_k3_invariant_reprojection_recovers_nearest_exact_interior_candidate() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let PositiveConjugacySearchResult2x2::Equivalent(witness) = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 4,
                sample_points: 64,
            },
        ) else {
            panic!("expected a witness for the k=3 pair");
        };

        let proposals = derive_invariant_compatible_positive_conjugacy_proposals_2x2(
            &a,
            &b,
            &witness,
            &PositiveConjugacyProposalConfig2x2 {
                max_proposals: 4,
                include_endpoints: false,
            },
        );

        let proposal_matrices = proposals
            .iter()
            .map(|proposal| proposal.matrix.clone())
            .collect::<Vec<_>>();
        assert_eq!(proposal_matrices, vec![SqMatrix::new([[1, 2], [3, 1]])]);
        assert!(proposals.iter().all(|proposal| {
            proposal.kind == PositiveConjugacyProposalKind2x2::InvariantCompatibleReprojection
        }));
        assert!(proposals
            .iter()
            .all(|proposal| proposal.matrix.det() == a.det()));
    }

    #[test]
    fn test_riedel_baker_k3_invariant_reprojection_uses_trace_determinant_family() {
        let a = SqMatrix::new([[3, 2], [1, 3]]);
        let b = SqMatrix::new([[2, 1], [1, 4]]);
        let PositiveConjugacySearchResult2x2::Equivalent(witness) = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 4,
                sample_points: 64,
            },
        ) else {
            panic!("expected a witness for the Riedel/Baker k=3 pair");
        };

        let proposals = derive_invariant_compatible_positive_conjugacy_proposals_2x2(
            &a,
            &b,
            &witness,
            &PositiveConjugacyProposalConfig2x2 {
                max_proposals: 4,
                include_endpoints: false,
            },
        );

        assert_eq!(proposals.len(), 1);
        assert!(matches!(
            proposals[0].matrix,
            SqMatrix {
                data: [[3, 1], [2, 3]]
            } | SqMatrix {
                data: [[4, 1], [1, 2]]
            }
        ));
        assert_eq!(proposals[0].matrix.trace(), a.trace());
        assert_eq!(proposals[0].matrix.det(), a.det());
        assert!(!proposals[0].preserves_endpoint_diagonal);
    }

    #[test]
    fn test_constant_positive_witness_has_no_interior_waypoint_proposals() {
        let a = SqMatrix::new([[1, 2], [2, 1]]);
        let PositiveConjugacySearchResult2x2::Equivalent(witness) = find_positive_conjugacy_2x2(
            &a,
            &a,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 1,
                sample_points: 16,
            },
        ) else {
            panic!("expected a witness for the constant positive pair");
        };

        let proposals = derive_positive_conjugacy_proposals_2x2(
            &a,
            &a,
            &witness,
            &PositiveConjugacyProposalConfig2x2::default(),
        );
        assert!(proposals.is_empty());

        let reprojected = derive_invariant_compatible_positive_conjugacy_proposals_2x2(
            &a,
            &a,
            &witness,
            &PositiveConjugacyProposalConfig2x2::default(),
        );
        assert!(!reprojected.is_empty());
        assert!(reprojected.iter().all(|proposal| proposal.matrix != a));
        assert!(reprojected
            .iter()
            .all(|proposal| proposal.matrix.trace() == a.trace()));
        assert!(reprojected
            .iter()
            .all(|proposal| proposal.matrix.det() == a.det()));
    }

    #[test]
    fn test_rank_positive_conjugacy_seed_candidates_prefers_nearest_top_proposals() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let PositiveConjugacySearchResult2x2::Equivalent(witness) = find_positive_conjugacy_2x2(
            &a,
            &b,
            &PositiveConjugacySearchConfig2x2 {
                max_conjugator_entry: 4,
                sample_points: 64,
            },
        ) else {
            panic!("expected a witness for the k=3 pair");
        };

        let proposals = derive_positive_conjugacy_proposals_2x2(
            &a,
            &b,
            &witness,
            &PositiveConjugacyProposalConfig2x2 {
                max_proposals: 4,
                include_endpoints: false,
            },
        );
        let candidates = vec![
            SqMatrix::new([[1, 4], [1, 1]]),
            SqMatrix::new([[1, 5], [1, 1]]),
            SqMatrix::new([[1, 4], [2, 1]]),
            SqMatrix::new([[1, 6], [1, 1]]),
            SqMatrix::new([[1, 5], [1, 1]]),
        ];

        let ranked = rank_positive_conjugacy_seed_candidates_2x2(
            &b,
            &proposals,
            &candidates,
            &PositiveConjugacySeedConfig2x2 { max_candidates: 4 },
        );

        assert_eq!(
            ranked,
            vec![
                PositiveConjugacySeedCandidate2x2 {
                    matrix: SqMatrix::new([[1, 5], [1, 1]]),
                    nearest_proposal_rank: 1,
                    proposal_l1_distance: 0,
                    target_l1_distance: 1,
                },
                PositiveConjugacySeedCandidate2x2 {
                    matrix: SqMatrix::new([[1, 4], [2, 1]]),
                    nearest_proposal_rank: 2,
                    proposal_l1_distance: 0,
                    target_l1_distance: 3,
                },
                PositiveConjugacySeedCandidate2x2 {
                    matrix: SqMatrix::new([[1, 4], [1, 1]]),
                    nearest_proposal_rank: 3,
                    proposal_l1_distance: 0,
                    target_l1_distance: 2,
                },
                PositiveConjugacySeedCandidate2x2 {
                    matrix: SqMatrix::new([[1, 6], [1, 1]]),
                    nearest_proposal_rank: 1,
                    proposal_l1_distance: 1,
                    target_l1_distance: 0,
                },
            ]
        );
    }

    #[test]
    fn test_derive_invariant_compatible_positive_conjugacy_seed_hints_matches_manual_pipeline() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let candidates = vec![
            SqMatrix::new([[1, 5], [1, 1]]),
            SqMatrix::new([[1, 4], [2, 1]]),
            SqMatrix::new([[0, 5], [1, 2]]),
            SqMatrix::new([[1, 2], [3, 1]]),
        ];
        let witness_config = PositiveConjugacySearchConfig2x2 {
            max_conjugator_entry: 4,
            sample_points: 64,
        };
        let proposal_config = PositiveConjugacyProposalConfig2x2 {
            max_proposals: 4,
            include_endpoints: false,
        };
        let seed_config = PositiveConjugacySeedConfig2x2 { max_candidates: 4 };

        let PositiveConjugacySearchResult2x2::Equivalent(witness) =
            find_positive_conjugacy_2x2(&a, &b, &witness_config)
        else {
            panic!("expected a witness for the k=3 pair");
        };
        let proposals = derive_invariant_compatible_positive_conjugacy_proposals_2x2(
            &a,
            &b,
            &witness,
            &proposal_config,
        );
        let manual =
            rank_positive_conjugacy_seed_candidates_2x2(&b, &proposals, &candidates, &seed_config);

        let via_helper = derive_invariant_compatible_positive_conjugacy_seed_hints_2x2(
            &a,
            &b,
            &candidates,
            &witness_config,
            &proposal_config,
            &seed_config,
        );

        assert_eq!(via_helper, manual);
        assert!(!via_helper.is_empty());
    }

    #[test]
    fn test_rank_positive_conjugacy_seed_candidates_handles_empty_inputs() {
        let target = SqMatrix::new([[1, 6], [1, 1]]);
        assert!(rank_positive_conjugacy_seed_candidates_2x2(
            &target,
            &[],
            &[SqMatrix::new([[1, 5], [1, 1]])],
            &PositiveConjugacySeedConfig2x2::default(),
        )
        .is_empty());
        assert!(rank_positive_conjugacy_seed_candidates_2x2(
            &target,
            &[PositiveConjugacyProposal2x2 {
                matrix: SqMatrix::new([[1, 5], [1, 1]]),
                kind: PositiveConjugacyProposalKind2x2::RoundedSampleWaypoint,
                nearest_sample_index: 0,
                nearest_sample_t: 0.0,
                shadow_l1_distance: 0.0,
                endpoint_l1_distance: 1,
                preserves_endpoint_diagonal: true,
                stays_within_endpoint_box: true,
            }],
            &[],
            &PositiveConjugacySeedConfig2x2::default(),
        )
        .is_empty());
    }
}
