use std::collections::BTreeMap;

use crate::concrete_shift::{
    concrete_shift_profile_2x2, ConcreteShiftProfileConfig2x2, ConcreteShiftProfileStatus2x2,
};
use crate::graph_moves::partition_refined_same_future_past_gap_total;
use crate::matrix::DynMatrix;

#[derive(Clone, Copy)]
pub struct ScoreSpec {
    pub name: &'static str,
    pub score: fn(&DynMatrix, &DynMatrix, &DynMatrix) -> f64,
}

#[derive(Clone, Copy)]
struct ScoreWeights {
    dim_gap: f64,
    row_col_types: f64,
    support_types: f64,
    duplicates: f64,
    quotient_gap: f64,
    endpoint_gap: f64,
}

#[derive(Default)]
pub struct ScoreSummary {
    pub seen: usize,
    pub top_1: usize,
    pub top_5_pct: usize,
    pub top_10_pct: usize,
    pub percentile_sum: f64,
    pub worst_percentile: f64,
}

impl ScoreSummary {
    pub fn add(&mut self, rank: Rank) {
        self.seen += 1;
        let pct = rank.rank as f64 / rank.total as f64;
        self.percentile_sum += pct;
        self.worst_percentile = self.worst_percentile.max(pct);
        if rank.rank == 1 {
            self.top_1 += 1;
        }
        if pct <= 0.05 {
            self.top_5_pct += 1;
        }
        if pct <= 0.10 {
            self.top_10_pct += 1;
        }
    }

    pub fn mean_percentile(&self) -> f64 {
        if self.seen == 0 {
            0.0
        } else {
            self.percentile_sum / self.seen as f64
        }
    }
}

#[derive(Clone, Copy)]
pub struct Rank {
    pub rank: usize,
    pub total: usize,
    pub ties: usize,
}

pub const DEFAULT_BEAM_SCORE_NAME: &str = "beam_default_low";

const DEFAULT_BEAM_SCORE_WEIGHTS: ScoreWeights = ScoreWeights {
    dim_gap: 0.0,
    row_col_types: 12.0,
    support_types: 6.0,
    duplicates: -6.0,
    quotient_gap: 0.5,
    endpoint_gap: 0.25,
};

const BEAM_DIMENSION_STRICT_SCORE_WEIGHTS: ScoreWeights = ScoreWeights {
    dim_gap: 128.0,
    row_col_types: 6.0,
    support_types: 2.0,
    duplicates: -2.0,
    quotient_gap: 0.25,
    endpoint_gap: 0.75,
};

pub fn candidate_score_specs() -> Vec<ScoreSpec> {
    vec![
        ScoreSpec {
            name: DEFAULT_BEAM_SCORE_NAME,
            score: |m, endpoint, _| score_node(m, endpoint),
        },
        ScoreSpec {
            name: "beam_dim_strict_low",
            score: |m, endpoint, _| {
                score_node_with_weights(m, endpoint, BEAM_DIMENSION_STRICT_SCORE_WEIGHTS)
            },
        },
        ScoreSpec {
            name: "dimension_low",
            score: |m, _, _| m.rows as f64,
        },
        ScoreSpec {
            name: "entry_sum_low",
            score: |m, _, _| entry_sum(m) as f64,
        },
        ScoreSpec {
            name: "max_entry_low",
            score: |m, _, _| m.max_entry() as f64,
        },
        ScoreSpec {
            name: "row_col_types_low",
            score: |m, _, _| (row_type_count(m) + col_type_count(m)) as f64,
        },
        ScoreSpec {
            name: "support_types_low",
            score: |m, _, _| (row_support_type_count(m) + col_support_type_count(m)) as f64,
        },
        ScoreSpec {
            name: "duplicates_high",
            score: |m, _, _| -((duplicate_row_pairs(m) + duplicate_col_pairs(m)) as f64),
        },
        ScoreSpec {
            name: "partition_refined_quotient_low",
            score: |m, endpoint, _| {
                partition_refined_same_future_past_gap_total(m, endpoint) as f64
            },
        },
        ScoreSpec {
            name: "endpoint_sig_low",
            score: |m, endpoint, _| signature_distance(m, endpoint) as f64,
        },
        ScoreSpec {
            name: "segment_goal_sig_low",
            score: |m, _, segment_goal| signature_distance(m, segment_goal) as f64,
        },
        ScoreSpec {
            name: "entry_plus_sig_low",
            score: |m, endpoint, _| entry_sum(m) as f64 + signature_distance(m, endpoint) as f64,
        },
        ScoreSpec {
            name: "types_plus_sig_low",
            score: |m, endpoint, _| {
                signature_distance(m, endpoint) as f64
                    + 8.0 * (row_type_count(m) + col_type_count(m)) as f64
            },
        },
    ]
}

pub fn new_summaries(specs: &[ScoreSpec]) -> BTreeMap<&'static str, ScoreSummary> {
    specs
        .iter()
        .map(|spec| (spec.name, ScoreSummary::default()))
        .collect()
}

pub fn rank_target(
    candidates: &[DynMatrix],
    target: &DynMatrix,
    endpoint: &DynMatrix,
    segment_goal: &DynMatrix,
    spec: ScoreSpec,
) -> Option<Rank> {
    let target_score = (spec.score)(target, endpoint, segment_goal);
    if !candidates.iter().any(|candidate| candidate == target) {
        return None;
    }

    let mut better = 0usize;
    let mut ties = 0usize;
    for candidate in candidates {
        let candidate_score = (spec.score)(candidate, endpoint, segment_goal);
        match candidate_score.total_cmp(&target_score) {
            std::cmp::Ordering::Less => better += 1,
            std::cmp::Ordering::Equal => ties += 1,
            std::cmp::Ordering::Greater => {}
        }
    }

    Some(Rank {
        rank: better + 1,
        total: candidates.len(),
        ties,
    })
}

pub fn entry_sum(m: &DynMatrix) -> u64 {
    m.data.iter().map(|&value| value as u64).sum()
}

pub fn row_type_count(m: &DynMatrix) -> usize {
    let mut rows = (0..m.rows)
        .map(|row| (0..m.cols).map(|col| m.get(row, col)).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    rows.sort();
    rows.dedup();
    rows.len()
}

pub fn col_type_count(m: &DynMatrix) -> usize {
    let mut cols = (0..m.cols)
        .map(|col| (0..m.rows).map(|row| m.get(row, col)).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    cols.sort();
    cols.dedup();
    cols.len()
}

pub fn row_support_type_count(m: &DynMatrix) -> usize {
    let mut rows = (0..m.rows)
        .map(|row| {
            (0..m.cols)
                .map(|col| u8::from(m.get(row, col) > 0))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows.dedup();
    rows.len()
}

pub fn col_support_type_count(m: &DynMatrix) -> usize {
    let mut cols = (0..m.cols)
        .map(|col| {
            (0..m.rows)
                .map(|row| u8::from(m.get(row, col) > 0))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    cols.sort();
    cols.dedup();
    cols.len()
}

pub fn duplicate_row_pairs(m: &DynMatrix) -> usize {
    let mut count = 0;
    for left in 0..m.rows {
        for right in left + 1..m.rows {
            if (0..m.cols).all(|col| m.get(left, col) == m.get(right, col)) {
                count += 1;
            }
        }
    }
    count
}

pub fn duplicate_col_pairs(m: &DynMatrix) -> usize {
    let mut count = 0;
    for left in 0..m.cols {
        for right in left + 1..m.cols {
            if (0..m.rows).all(|row| m.get(row, left) == m.get(row, right)) {
                count += 1;
            }
        }
    }
    count
}

pub fn score_node(node: &DynMatrix, target: &DynMatrix) -> f64 {
    score_node_with_weights(node, target, DEFAULT_BEAM_SCORE_WEIGHTS)
}

pub fn score_node_with_concrete_shift_profile(node: &DynMatrix, target: &DynMatrix) -> f64 {
    let base = score_node(node, target);
    let Some(profile_score) = concrete_shift_profile_score(node, target) else {
        return base;
    };
    base + profile_score
}

pub fn score_node_with_witness_bridge_profile(node: &DynMatrix, target: &DynMatrix) -> f64 {
    score_node(node, target) + witness_bridge_profile_score(node)
}

pub fn score_node_with_sparse_k4_bridge_profile(node: &DynMatrix, target: &DynMatrix) -> f64 {
    score_node(node, target) + sparse_k4_bridge_profile_score(node)
}

fn witness_bridge_profile_score(node: &DynMatrix) -> f64 {
    if matches_witness_bridge_profile(node) {
        -48.0
    } else {
        0.0
    }
}

fn matches_witness_bridge_profile(node: &DynMatrix) -> bool {
    if !matches!(node.rows, 3 | 4) || !node.is_square() {
        return false;
    }

    let (support, row_supports, col_supports) = support_profile(node);
    match node.rows {
        3 => {
            (support == 7 && row_supports == [1, 3, 3] && col_supports == [2, 2, 3])
                || (support == 7 && row_supports == [2, 2, 3] && col_supports == [1, 3, 3])
                || (support == 8 && row_supports == [2, 3, 3] && col_supports == [2, 3, 3])
        }
        4 => {
            (support == 11 && row_supports == [2, 2, 3, 4] && col_supports == [2, 2, 3, 4])
                || (support == 11 && row_supports == [2, 3, 3, 3] && col_supports == [1, 3, 3, 4])
                || (support == 11 && row_supports == [1, 3, 3, 4] && col_supports == [2, 3, 3, 3])
        }
        _ => false,
    }
}

fn sparse_k4_bridge_profile_score(node: &DynMatrix) -> f64 {
    if matches_sparse_k4_bridge_profile(node) {
        -48.0
    } else {
        0.0
    }
}

fn matches_sparse_k4_bridge_profile(node: &DynMatrix) -> bool {
    if node.rows != 4 || !node.is_square() {
        return false;
    }

    // Retained Brix-Ruiz k4 diagonal-refactorization near-hit shape.
    let (support, row_supports, col_supports) = support_profile(node);
    support == 7
        && ((row_supports == [0, 0, 3, 4] && col_supports == [1, 2, 2, 2])
            || (row_supports == [1, 2, 2, 2] && col_supports == [0, 0, 3, 4]))
}

fn support_profile(node: &DynMatrix) -> (usize, Vec<usize>, Vec<usize>) {
    let mut support = 0usize;
    let mut row_supports = vec![0usize; node.rows];
    let mut col_supports = vec![0usize; node.cols];

    for row in 0..node.rows {
        for col in 0..node.cols {
            if node.get(row, col) > 0 {
                support += 1;
                row_supports[row] += 1;
                col_supports[col] += 1;
            }
        }
    }

    row_supports.sort_unstable();
    col_supports.sort_unstable();
    (support, row_supports, col_supports)
}

fn concrete_shift_profile_score(node: &DynMatrix, target: &DynMatrix) -> Option<f64> {
    let node = node.to_sq::<2>()?;
    let target = target.to_sq::<2>()?;
    let profile =
        concrete_shift_profile_2x2(&node, &target, &ConcreteShiftProfileConfig2x2::default());
    Some(concrete_shift_profile_status_score(
        profile.status,
        profile.limit_reached,
        profile.shift_witnesses,
        profile.concrete_witness_lag,
        profile.max_lag,
    ))
}

fn concrete_shift_profile_status_score(
    status: ConcreteShiftProfileStatus2x2,
    limit_reached: bool,
    shift_witnesses: usize,
    concrete_witness_lag: Option<u32>,
    max_lag: u32,
) -> f64 {
    let lag = concrete_witness_lag.unwrap_or(max_lag + 1) as f64;
    let shift_signal = shift_witnesses.min(8) as f64;
    match status {
        ConcreteShiftProfileStatus2x2::Equivalent => -96.0 + lag,
        ConcreteShiftProfileStatus2x2::ShiftWitnessOnly if limit_reached => -4.0 - shift_signal,
        ConcreteShiftProfileStatus2x2::ShiftWitnessOnly => -16.0 - shift_signal,
        ConcreteShiftProfileStatus2x2::SearchLimitReached => 12.0,
        ConcreteShiftProfileStatus2x2::Exhausted => 32.0,
    }
}

fn score_node_with_weights(node: &DynMatrix, target: &DynMatrix, weights: ScoreWeights) -> f64 {
    let dim_gap = node.rows.abs_diff(target.rows) as f64;
    let row_col_types = (row_type_count(node) + col_type_count(node)) as f64;
    let support_types = (row_support_type_count(node) + col_support_type_count(node)) as f64;
    let duplicates = (duplicate_row_pairs(node) + duplicate_col_pairs(node)) as f64;
    let quotient_gap = same_future_past_signature_gap(node, target) as f64;
    let endpoint_gap = signature_distance(node, target) as f64;

    weights.dim_gap * dim_gap
        + weights.row_col_types * row_col_types
        + weights.support_types * support_types
        + weights.duplicates * duplicates
        + weights.quotient_gap * quotient_gap
        + weights.endpoint_gap * endpoint_gap
}

pub fn signature_distance(left: &DynMatrix, right: &DynMatrix) -> u64 {
    let left_sig = Signature::new(left);
    let right_sig = Signature::new(right);
    let dim_distance = left_sig.dim.abs_diff(right_sig.dim) as u64;
    10 * dim_distance
        + left_sig.entry_sum.abs_diff(right_sig.entry_sum)
        + left_sig.max_entry.abs_diff(right_sig.max_entry) as u64
        + sorted_l1(&left_sig.row_sums, &right_sig.row_sums)
        + sorted_l1(&left_sig.col_sums, &right_sig.col_sums)
        + sorted_l1_u8(&left_sig.row_supports, &right_sig.row_supports)
        + sorted_l1_u8(&left_sig.col_supports, &right_sig.col_supports)
}

fn same_future_past_signature_gap(left: &DynMatrix, right: &DynMatrix) -> u64 {
    let left_sig = StructureSignature::new(left);
    let right_sig = StructureSignature::new(right);

    10 * left_sig.dim.abs_diff(right_sig.dim) as u64
        + left_sig.entry_sum.abs_diff(right_sig.entry_sum)
        + class_gap(&left_sig.row_classes, &right_sig.row_classes)
        + class_gap(&left_sig.col_classes, &right_sig.col_classes)
}

struct Signature {
    dim: usize,
    entry_sum: u64,
    max_entry: u32,
    row_sums: Vec<u32>,
    col_sums: Vec<u32>,
    row_supports: Vec<u8>,
    col_supports: Vec<u8>,
}

impl Signature {
    fn new(m: &DynMatrix) -> Self {
        let mut row_sums = vec![0u32; m.rows];
        let mut col_sums = vec![0u32; m.cols];
        let mut row_supports = vec![0u8; m.rows];
        let mut col_supports = vec![0u8; m.cols];

        for row in 0..m.rows {
            for col in 0..m.cols {
                let value = m.get(row, col);
                row_sums[row] += value;
                col_sums[col] += value;
                if value > 0 {
                    row_supports[row] += 1;
                    col_supports[col] += 1;
                }
            }
        }

        row_sums.sort_unstable();
        col_sums.sort_unstable();
        row_supports.sort_unstable();
        col_supports.sort_unstable();

        Self {
            dim: m.rows,
            entry_sum: entry_sum(m),
            max_entry: m.max_entry(),
            row_sums,
            col_sums,
            row_supports,
            col_supports,
        }
    }
}

struct StructureSignature {
    dim: usize,
    entry_sum: u64,
    row_classes: Vec<DuplicateVectorClassSignature>,
    col_classes: Vec<DuplicateVectorClassSignature>,
}

impl StructureSignature {
    fn new(m: &DynMatrix) -> Self {
        let row_vectors = (0..m.rows)
            .map(|row| (0..m.cols).map(|col| m.get(row, col)).collect::<Vec<_>>())
            .collect::<Vec<_>>();
        let col_vectors = (0..m.cols)
            .map(|col| (0..m.rows).map(|row| m.get(row, col)).collect::<Vec<_>>())
            .collect::<Vec<_>>();

        Self {
            dim: m.rows,
            entry_sum: entry_sum(m),
            row_classes: duplicate_vector_classes(&row_vectors),
            col_classes: duplicate_vector_classes(&col_vectors),
        }
    }
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct DuplicateVectorClassSignature {
    multiplicity: usize,
    entry_sum: u64,
    support: u8,
}

fn duplicate_vector_classes(vectors: &[Vec<u32>]) -> Vec<DuplicateVectorClassSignature> {
    let mut multiplicities = BTreeMap::<Vec<u32>, usize>::new();
    for values in vectors {
        *multiplicities.entry(values.clone()).or_default() += 1;
    }

    let mut classes = multiplicities
        .into_iter()
        .map(|(values, multiplicity)| DuplicateVectorClassSignature {
            multiplicity,
            entry_sum: values.iter().map(|&value| value as u64).sum(),
            support: values.iter().filter(|&&value| value > 0).count() as u8,
        })
        .collect::<Vec<_>>();
    classes.sort_unstable();
    classes
}

fn class_gap(
    left: &[DuplicateVectorClassSignature],
    right: &[DuplicateVectorClassSignature],
) -> u64 {
    let len = left.len().max(right.len());
    let mut total = 0u64;
    for idx in 0..len {
        let left_class = left.get(idx);
        let right_class = right.get(idx);
        total += left_class
            .map(|class| class.multiplicity)
            .unwrap_or(0)
            .abs_diff(right_class.map(|class| class.multiplicity).unwrap_or(0))
            as u64;
        total += left_class
            .map(|class| class.entry_sum)
            .unwrap_or(0)
            .abs_diff(right_class.map(|class| class.entry_sum).unwrap_or(0));
        total += left_class
            .map(|class| class.support)
            .unwrap_or(0)
            .abs_diff(right_class.map(|class| class.support).unwrap_or(0)) as u64;
    }
    total
}

fn sorted_l1(left: &[u32], right: &[u32]) -> u64 {
    let len = left.len().max(right.len());
    let mut total = 0u64;
    for idx in 0..len {
        let left_value = left.get(idx).copied().unwrap_or(0);
        let right_value = right.get(idx).copied().unwrap_or(0);
        total += left_value.abs_diff(right_value) as u64;
    }
    total
}

fn sorted_l1_u8(left: &[u8], right: &[u8]) -> u64 {
    let len = left.len().max(right.len());
    let mut total = 0u64;
    for idx in 0..len {
        let left_value = left.get(idx).copied().unwrap_or(0);
        let right_value = right.get(idx).copied().unwrap_or(0);
        total += left_value.abs_diff(right_value) as u64;
    }
    total
}

#[cfg(test)]
mod tests {
    use super::{
        candidate_score_specs, concrete_shift_profile_status_score, rank_target, score_node,
        score_node_with_concrete_shift_profile, score_node_with_sparse_k4_bridge_profile,
        score_node_with_weights, score_node_with_witness_bridge_profile, signature_distance,
        BEAM_DIMENSION_STRICT_SCORE_WEIGHTS, DEFAULT_BEAM_SCORE_NAME,
    };
    use crate::concrete_shift::ConcreteShiftProfileStatus2x2;
    use crate::matrix::DynMatrix;

    #[test]
    fn rank_target_counts_ties_and_rank() {
        let target = DynMatrix::new(2, 2, vec![1, 2, 2, 1]);
        let candidates = vec![
            DynMatrix::new(2, 2, vec![1, 1, 1, 1]),
            target.clone(),
            DynMatrix::new(2, 2, vec![2, 1, 1, 2]),
        ];
        let spec = candidate_score_specs()
            .into_iter()
            .find(|spec| spec.name == "entry_sum_low")
            .expect("entry_sum_low spec should exist");

        let rank = rank_target(&candidates, &target, &target, &target, spec)
            .expect("target should be present");
        assert_eq!(rank.rank, 2);
        assert_eq!(rank.total, 3);
        assert_eq!(rank.ties, 2);
    }

    #[test]
    fn signature_distance_is_zero_for_identical_matrices() {
        let matrix = DynMatrix::new(3, 3, vec![0, 1, 2, 1, 0, 1, 2, 1, 0]);
        assert_eq!(signature_distance(&matrix, &matrix), 0);
    }

    #[test]
    fn default_beam_score_is_registered() {
        assert!(candidate_score_specs()
            .into_iter()
            .any(|spec| spec.name == DEFAULT_BEAM_SCORE_NAME));
    }

    #[test]
    fn partition_refined_quotient_score_is_registered() {
        assert!(candidate_score_specs()
            .into_iter()
            .any(|spec| spec.name == "partition_refined_quotient_low"));
    }

    #[test]
    fn score_node_prefers_matching_duplicate_class_structure() {
        let target = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 1, 1]);
        let structured = DynMatrix::new(3, 3, vec![1, 0, 1, 1, 0, 1, 0, 1, 1]);
        let unstructured = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 1, 1, 1, 0, 1]);

        assert!(score_node(&structured, &target) < score_node(&unstructured, &target));
    }

    #[test]
    fn score_node_prefers_target_over_same_weight_but_less_structured_candidate() {
        let target = DynMatrix::new(4, 4, vec![0, 0, 1, 1, 0, 1, 2, 2, 0, 1, 1, 1, 1, 1, 1, 0]);
        let near_miss = DynMatrix::new(4, 4, vec![0, 1, 0, 1, 1, 0, 2, 1, 0, 1, 1, 1, 1, 1, 1, 0]);

        assert!(score_node(&target, &target) < score_node(&near_miss, &target));
    }

    #[test]
    fn concrete_shift_profile_score_prefers_low_lag_profile_hit() {
        let target = DynMatrix::new(2, 2, vec![1, 1, 1, 0]);
        let profile_hit = DynMatrix::new(2, 2, vec![1, 1, 1, 0]);
        let profile_miss = DynMatrix::new(2, 2, vec![3, 0, 0, 0]);

        assert!(
            score_node_with_concrete_shift_profile(&profile_hit, &target)
                < score_node_with_concrete_shift_profile(&profile_miss, &target)
        );
    }

    #[test]
    fn concrete_shift_profile_status_score_keeps_limited_shift_signal() {
        let limited_shift = concrete_shift_profile_status_score(
            ConcreteShiftProfileStatus2x2::ShiftWitnessOnly,
            true,
            3,
            None,
            1,
        );
        let pure_limit = concrete_shift_profile_status_score(
            ConcreteShiftProfileStatus2x2::SearchLimitReached,
            true,
            0,
            None,
            1,
        );
        let exhausted = concrete_shift_profile_status_score(
            ConcreteShiftProfileStatus2x2::Exhausted,
            false,
            0,
            None,
            1,
        );

        assert!(limited_shift < pure_limit);
        assert!(pure_limit < exhausted);
    }

    #[test]
    fn witness_bridge_profile_score_prefers_frozen_support_profile() {
        let target = DynMatrix::new(2, 2, vec![1, 3, 2, 1]);
        let bridge = DynMatrix::new(3, 3, vec![1, 0, 0, 1, 1, 1, 1, 1, 1]);
        let same_dim_miss = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 1, 1]);

        assert!(
            score_node_with_witness_bridge_profile(&bridge, &target)
                < score_node_with_witness_bridge_profile(&same_dim_miss, &target)
        );
    }

    #[test]
    fn witness_bridge_profile_score_covers_frozen_profile_table() {
        let target = DynMatrix::new(2, 2, vec![1, 3, 2, 1]);
        let matching_profiles = [
            DynMatrix::new(3, 3, vec![1, 0, 0, 1, 1, 1, 1, 1, 1]),
            DynMatrix::new(3, 3, vec![1, 1, 1, 0, 1, 1, 0, 1, 1]),
            DynMatrix::new(3, 3, vec![0, 1, 1, 1, 1, 1, 1, 1, 1]),
            DynMatrix::new(4, 4, vec![1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 1, 0, 1]),
            DynMatrix::new(4, 4, vec![1, 1, 1, 0, 1, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 0]),
            DynMatrix::new(4, 4, vec![1, 1, 1, 1, 1, 1, 1, 0, 1, 1, 0, 1, 0, 0, 1, 0]),
        ];

        for profile in matching_profiles {
            let bonus = score_node_with_witness_bridge_profile(&profile, &target)
                - score_node(&profile, &target);
            assert_eq!(bonus, -48.0);
        }

        let outside_frozen_dims = DynMatrix::new(5, 5, vec![1; 25]);
        let bonus = score_node_with_witness_bridge_profile(&outside_frozen_dims, &target)
            - score_node(&outside_frozen_dims, &target);
        assert_eq!(bonus, 0.0);
    }

    #[test]
    fn sparse_k4_bridge_profile_score_targets_retained_sparse_active_blocks() {
        let target = DynMatrix::new(2, 2, vec![1, 12, 1, 1]);
        let sparse_rows =
            DynMatrix::new(4, 4, vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]);
        let sparse_cols =
            DynMatrix::new(4, 4, vec![1, 3, 0, 0, 4, 1, 0, 0, 2, 0, 0, 0, 7, 6, 0, 0]);
        let row_permuted_sparse =
            DynMatrix::new(4, 4, vec![0, 0, 0, 0, 1, 4, 2, 7, 0, 0, 0, 0, 3, 1, 0, 6]);
        let col_permuted_sparse =
            DynMatrix::new(4, 4, vec![2, 1, 7, 4, 0, 3, 6, 1, 0, 0, 0, 0, 0, 0, 0, 0]);
        let dense_witness_profile =
            DynMatrix::new(4, 4, vec![1, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 1, 0, 1, 0, 1]);
        let same_dim_miss =
            DynMatrix::new(4, 4, vec![1, 4, 2, 7, 3, 1, 1, 5, 0, 0, 0, 0, 0, 0, 0, 0]);

        for profile in [
            sparse_rows,
            sparse_cols,
            row_permuted_sparse,
            col_permuted_sparse,
        ] {
            let bonus = score_node_with_sparse_k4_bridge_profile(&profile, &target)
                - score_node(&profile, &target);
            assert_eq!(bonus, -48.0);
        }

        for profile in [dense_witness_profile, same_dim_miss] {
            let bonus = score_node_with_sparse_k4_bridge_profile(&profile, &target)
                - score_node(&profile, &target);
            assert_eq!(bonus, 0.0);
        }
    }

    #[test]
    fn dimension_hybrid_penalizes_dimension_gap_even_for_structured_candidates() {
        let target = DynMatrix::new(2, 2, vec![1, 1, 1, 0]);
        let matching_dim = DynMatrix::new(2, 2, vec![1, 0, 1, 1]);
        let farther_dim = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 0, 1]);

        assert!(
            score_node_with_weights(&matching_dim, &target, BEAM_DIMENSION_STRICT_SCORE_WEIGHTS)
                < score_node_with_weights(
                    &farther_dim,
                    &target,
                    BEAM_DIMENSION_STRICT_SCORE_WEIGHTS,
                )
        );
    }
}
