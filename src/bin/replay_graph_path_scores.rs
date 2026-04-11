use std::collections::{BTreeMap, HashMap, HashSet};

use sse_core::graph_moves::enumerate_graph_move_successors;
use sse_core::matrix::DynMatrix;

fn main() {
    let max_dim = 5;
    let max_entry = 6;

    let endpoint = KnownPath {
        label: "blind endpoint 16-move path",
        path: endpoint_16_path(),
        segment_ends: vec![8, 16],
    };
    let baker = KnownPath {
        label: "Baker waypoint-expanded 22-move path",
        path: baker_22_path(),
        segment_ends: vec![1, 6, 8, 10, 16, 19, 22],
    };

    for known in [&endpoint, &baker] {
        println!();
        println!("=== {} ===", known.label);
        println!(
            "moves={}, max_dim={}, max_entry={}",
            known.path.len() - 1,
            max_dim,
            max_entry
        );
        analyze_local_steps(known, max_dim, max_entry);
        analyze_bfs_segments(known, max_dim, max_entry);
    }
}

struct KnownPath {
    label: &'static str,
    path: Vec<DynMatrix>,
    segment_ends: Vec<usize>,
}

#[derive(Clone, Copy)]
struct ScoreSpec {
    name: &'static str,
    score: fn(&DynMatrix, &DynMatrix, &DynMatrix) -> i64,
}

#[derive(Default)]
struct ScoreSummary {
    seen: usize,
    top_1: usize,
    top_5_pct: usize,
    top_10_pct: usize,
    percentile_sum: f64,
    worst_percentile: f64,
}

impl ScoreSummary {
    fn add(&mut self, rank: Rank) {
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

    fn mean_percentile(&self) -> f64 {
        if self.seen == 0 {
            0.0
        } else {
            self.percentile_sum / self.seen as f64
        }
    }
}

#[derive(Clone, Copy)]
struct Rank {
    rank: usize,
    total: usize,
    ties: usize,
}

fn analyze_local_steps(known: &KnownPath, max_dim: usize, max_entry: u32) {
    println!();
    println!("Local successor ranking:");
    let specs = score_specs();
    let mut summaries = new_summaries(&specs);
    let final_target = known.path.last().expect("path should not be empty");

    let mut segment_start = 0usize;
    for &segment_end in &known.segment_ends {
        let segment_goal = &known.path[segment_end];
        for step in segment_start..segment_end {
            let current = &known.path[step];
            let target = &known.path[step + 1];
            let successors = successor_set(current, max_dim, max_entry);
            print_step_header(step, current, target, successors.len());
            for spec in &specs {
                match rank_target(&successors, target, final_target, segment_goal, *spec) {
                    Some(rank) => {
                        summaries
                            .get_mut(spec.name)
                            .expect("summary exists")
                            .add(rank);
                        print_rank(spec.name, rank);
                    }
                    None => println!("    {:<24} missing", spec.name),
                }
            }
        }
        segment_start = segment_end;
    }

    print_summaries(&summaries);
}

fn analyze_bfs_segments(known: &KnownPath, max_dim: usize, max_entry: u32) {
    println!();
    println!("BFS segment next-frontier ranking:");
    let specs = score_specs();
    let mut summaries = new_summaries(&specs);
    let final_target = known.path.last().expect("path should not be empty");

    let mut segment_start = 0usize;
    for &segment_end in &known.segment_ends {
        println!("  segment {segment_start}->{segment_end}:");
        let segment_goal = &known.path[segment_end];
        let mut seen = HashMap::<DynMatrix, usize>::new();
        let mut frontier = vec![known.path[segment_start].clone()];
        seen.insert(known.path[segment_start].clone(), 0);

        for step in segment_start..segment_end {
            let rel_depth = step - segment_start;
            let target = &known.path[step + 1];
            let mut candidates = HashSet::<DynMatrix>::new();
            let mut next_frontier = Vec::new();

            for current in &frontier {
                for successor in successor_set(current, max_dim, max_entry) {
                    candidates.insert(successor.clone());
                    if !seen.contains_key(&successor) {
                        seen.insert(successor.clone(), rel_depth + 1);
                        next_frontier.push(successor);
                    }
                }
            }

            let target_seen_depth = seen.get(target).copied();
            print!(
                "    step {:>2} depth {:>2}: frontier={}, candidates={}, new={}",
                step,
                rel_depth,
                frontier.len(),
                candidates.len(),
                next_frontier.len()
            );
            if let Some(depth) = target_seen_depth {
                if depth < rel_depth + 1 {
                    println!("; known next was already seen at relative depth {depth}");
                } else {
                    println!();
                    for spec in &specs {
                        match rank_target(&next_frontier, target, final_target, segment_goal, *spec)
                        {
                            Some(rank) => {
                                summaries
                                    .get_mut(spec.name)
                                    .expect("summary exists")
                                    .add(rank);
                                print_rank(spec.name, rank);
                            }
                            None => println!("      {:<24} missing from new frontier", spec.name),
                        }
                    }
                }
            } else {
                println!("; known next not reached from this BFS layer");
            }

            frontier = next_frontier;
        }
        segment_start = segment_end;
    }

    print_summaries(&summaries);
}

fn score_specs() -> Vec<ScoreSpec> {
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
    ]
}

fn new_summaries(specs: &[ScoreSpec]) -> BTreeMap<&'static str, ScoreSummary> {
    specs
        .iter()
        .map(|spec| (spec.name, ScoreSummary::default()))
        .collect()
}

fn successor_set(current: &DynMatrix, max_dim: usize, max_entry: u32) -> Vec<DynMatrix> {
    let mut seen = HashSet::new();
    let mut successors = Vec::new();
    for successor in enumerate_graph_move_successors(current, max_dim).nodes {
        if successor.matrix.max_entry() <= max_entry && seen.insert(successor.matrix.clone()) {
            successors.push(successor.matrix);
        }
    }
    successors
}

fn rank_target(
    candidates: &[DynMatrix],
    target: &DynMatrix,
    final_target: &DynMatrix,
    segment_goal: &DynMatrix,
    spec: ScoreSpec,
) -> Option<Rank> {
    let target_score = (spec.score)(target, final_target, segment_goal);
    if !candidates.iter().any(|candidate| candidate == target) {
        return None;
    }

    let mut better = 0usize;
    let mut ties = 0usize;
    for candidate in candidates {
        let candidate_score = (spec.score)(candidate, final_target, segment_goal);
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

fn print_step_header(step: usize, current: &DynMatrix, target: &DynMatrix, candidates: usize) {
    println!(
        "  step {:>2}: {}x{} -> {}x{}, candidates={}",
        step, current.rows, current.cols, target.rows, target.cols, candidates
    );
}

fn print_rank(label: &str, rank: Rank) {
    println!(
        "    {:<24} rank {:>6}/{:<6} pct={:>6.2}% ties={}",
        label,
        rank.rank,
        rank.total,
        100.0 * rank.rank as f64 / rank.total as f64,
        rank.ties
    );
}

fn print_summaries(summaries: &BTreeMap<&'static str, ScoreSummary>) {
    println!("  summary:");
    for (name, summary) in summaries {
        println!(
            "    {:<24} n={:<3} mean_pct={:>6.2}% worst_pct={:>6.2}% top1={:<3} top5%={:<3} top10%={:<3}",
            name,
            summary.seen,
            100.0 * summary.mean_percentile(),
            100.0 * summary.worst_percentile,
            summary.top_1,
            summary.top_5_pct,
            summary.top_10_pct
        );
    }
}

fn entry_sum(m: &DynMatrix) -> u64 {
    m.data.iter().map(|&value| value as u64).sum()
}

fn row_type_count(m: &DynMatrix) -> usize {
    let mut rows = (0..m.rows)
        .map(|row| (0..m.cols).map(|col| m.get(row, col)).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    rows.sort();
    rows.dedup();
    rows.len()
}

fn col_type_count(m: &DynMatrix) -> usize {
    let mut cols = (0..m.cols)
        .map(|col| (0..m.rows).map(|row| m.get(row, col)).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    cols.sort();
    cols.dedup();
    cols.len()
}

fn row_support_type_count(m: &DynMatrix) -> usize {
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

fn col_support_type_count(m: &DynMatrix) -> usize {
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

fn duplicate_row_pairs(m: &DynMatrix) -> usize {
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

fn duplicate_col_pairs(m: &DynMatrix) -> usize {
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

fn signature_distance(left: &DynMatrix, right: &DynMatrix) -> u64 {
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

fn mat(dim: usize, data: Vec<u32>) -> DynMatrix {
    DynMatrix::new(dim, dim, data).canonical_perm()
}

fn endpoint_16_path() -> Vec<DynMatrix> {
    vec![
        mat(2, vec![1, 2, 3, 1]),
        mat(3, vec![0, 0, 2, 1, 1, 1, 2, 2, 1]),
        mat(4, vec![0, 0, 1, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 2, 0, 1]),
        mat(
            5,
            vec![
                0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 2, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 0, 1, 2, 2, 0, 1, 1, 1, 1, 1, 1, 0]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 0, 2, 0, 2, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 1, 0, 2, 1, 1, 1, 1, 0, 2, 2, 2, 1]),
        mat(
            5,
            vec![
                0, 1, 0, 0, 1, 1, 0, 2, 0, 0, 1, 1, 0, 2, 1, 1, 0, 1, 1, 0, 1, 1, 0, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 2, 1, 2, 1, 1]),
        mat(
            5,
            vec![
                0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1, 0, 2, 1, 1, 0, 0, 1, 1, 1, 1, 0, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 0, 1, 0, 1, 1, 2, 0, 1, 2, 2, 1, 1]),
        mat(
            5,
            vec![
                0, 1, 0, 0, 1, 1, 0, 1, 2, 0, 2, 1, 0, 2, 1, 0, 0, 1, 1, 0, 2, 1, 0, 2, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 0, 1, 1, 0, 2, 2, 0, 1, 3, 4, 1, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 0, 1, 0, 1, 1, 0, 1, 3, 1, 4, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 0, 4, 4, 1, 1]),
        mat(3, vec![0, 0, 2, 1, 1, 4, 1, 1, 1]),
        mat(2, vec![1, 1, 6, 1]),
    ]
}

fn baker_22_path() -> Vec<DynMatrix> {
    vec![
        mat(2, vec![1, 2, 3, 1]),
        mat(3, vec![0, 0, 1, 1, 1, 2, 2, 2, 1]),
        mat(4, vec![0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 2, 1, 1, 2, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 0, 1, 1, 1, 1, 2, 1, 1, 1, 2, 1, 0]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 0,
            ],
        ),
        mat(4, vec![0, 0, 1, 2, 1, 0, 1, 2, 2, 0, 1, 2, 1, 1, 0, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 1, 2, 1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 2, 0, 2, 1, 0, 1, 1, 1, 0, 0,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 0, 1, 0, 2, 1, 1, 0, 1, 2, 1, 1, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 1, 1, 1, 1, 1, 0, 1, 2, 2, 1, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 2, 1, 0, 2, 1, 0, 0, 2, 1, 1, 1, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 2, 2, 1, 0, 0, 1, 0, 1, 1, 1, 1, 2, 0, 0, 0,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 1, 0, 1, 1, 2, 2, 1, 0, 2, 2, 1, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 2, 1, 1, 1, 0, 2, 1, 1, 1, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 2, 0, 2, 2, 2, 1, 1, 0, 2, 1, 1, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 2, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1,
            ],
        ),
        mat(4, vec![0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 3, 1, 1, 1, 1]),
        mat(
            5,
            vec![
                0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 2, 2, 0, 3, 3, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0,
            ],
        ),
        mat(4, vec![0, 0, 1, 1, 2, 0, 3, 5, 0, 0, 1, 1, 1, 1, 0, 1]),
        mat(3, vec![0, 5, 5, 0, 1, 1, 1, 1, 1]),
        mat(2, vec![0, 5, 1, 2]),
        mat(3, vec![0, 0, 5, 1, 1, 1, 1, 1, 1]),
        mat(2, vec![1, 1, 6, 1]),
    ]
}
