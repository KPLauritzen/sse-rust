use std::collections::BTreeMap;

use crate::matrix::DynMatrix;

#[derive(Clone, Copy)]
pub struct ScoreSpec {
    pub name: &'static str,
    pub score: fn(&DynMatrix, &DynMatrix, &DynMatrix) -> i64,
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

pub fn candidate_score_specs() -> Vec<ScoreSpec> {
    vec![
        ScoreSpec {
            name: "dimension_low",
            score: |m, _, _| m.rows as i64,
        },
        ScoreSpec {
            name: "entry_sum_low",
            score: |m, _, _| entry_sum(m) as i64,
        },
        ScoreSpec {
            name: "max_entry_low",
            score: |m, _, _| m.max_entry() as i64,
        },
        ScoreSpec {
            name: "row_col_types_low",
            score: |m, _, _| (row_type_count(m) + col_type_count(m)) as i64,
        },
        ScoreSpec {
            name: "support_types_low",
            score: |m, _, _| (row_support_type_count(m) + col_support_type_count(m)) as i64,
        },
        ScoreSpec {
            name: "duplicates_high",
            score: |m, _, _| -(duplicate_row_pairs(m) as i64 + duplicate_col_pairs(m) as i64),
        },
        ScoreSpec {
            name: "endpoint_sig_low",
            score: |m, endpoint, _| signature_distance(m, endpoint) as i64,
        },
        ScoreSpec {
            name: "segment_goal_sig_low",
            score: |m, _, segment_goal| signature_distance(m, segment_goal) as i64,
        },
        ScoreSpec {
            name: "entry_plus_sig_low",
            score: |m, endpoint, _| entry_sum(m) as i64 + signature_distance(m, endpoint) as i64,
        },
        ScoreSpec {
            name: "types_plus_sig_low",
            score: |m, endpoint, _| {
                signature_distance(m, endpoint) as i64
                    + 8 * (row_type_count(m) + col_type_count(m)) as i64
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
        if candidate_score < target_score {
            better += 1;
        } else if candidate_score == target_score {
            ties += 1;
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
    use super::{candidate_score_specs, rank_target, signature_distance};
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
}
