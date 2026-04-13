use std::collections::{BTreeMap, HashMap, HashSet};

use sse_core::graph_moves::enumerate_graph_move_successors;
use sse_core::matrix::DynMatrix;
use sse_core::path_scoring::{
    candidate_score_specs, new_summaries, rank_target, Rank, ScoreSummary,
};

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

fn analyze_local_steps(known: &KnownPath, max_dim: usize, max_entry: u32) {
    println!();
    println!("Local successor ranking:");
    let specs = candidate_score_specs();
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
    let specs = candidate_score_specs();
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
