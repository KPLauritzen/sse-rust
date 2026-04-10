use std::collections::{BTreeMap, HashMap, VecDeque};
use std::time::{Duration, Instant};

use sse_core::graph_moves::enumerate_graph_move_successors;
use sse_core::matrix::{DynMatrix, SqMatrix};

fn main() {
    let mut max_depth = 22usize;
    let mut max_dim = 5usize;
    let mut max_entry = 6u32;
    let mut max_states = 1_000_000usize;
    let mut max_candidates = 10_000_000usize;
    let mut max_seconds = 30u64;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--max-depth" => {
                max_depth = args
                    .next()
                    .expect("--max-depth requires a value")
                    .parse()
                    .expect("invalid max depth");
            }
            "--max-dim" => {
                max_dim = args
                    .next()
                    .expect("--max-dim requires a value")
                    .parse()
                    .expect("invalid max dim");
            }
            "--max-entry" => {
                max_entry = args
                    .next()
                    .expect("--max-entry requires a value")
                    .parse()
                    .expect("invalid max entry");
            }
            "--max-states" => {
                max_states = args
                    .next()
                    .expect("--max-states requires a value")
                    .parse()
                    .expect("invalid max states");
            }
            "--max-candidates" => {
                max_candidates = args
                    .next()
                    .expect("--max-candidates requires a value")
                    .parse()
                    .expect("invalid max candidates");
            }
            "--max-seconds" => {
                max_seconds = args
                    .next()
                    .expect("--max-seconds requires a value")
                    .parse()
                    .expect("invalid max seconds");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_brix_ruiz_graph_path [--max-depth N] [--max-dim N] [--max-entry N] [--max-states N] [--max-candidates N] [--max-seconds N]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let start = DynMatrix::from_sq(&SqMatrix::new([[1, 3], [2, 1]]));
    let target = DynMatrix::from_sq(&SqMatrix::new([[1, 6], [1, 1]]));

    println!("Blind graph-only Brix-Ruiz k=3 search");
    println!(
        "Config: max_depth={max_depth}, max_dim={max_dim}, max_entry={max_entry}, max_states={max_states}, max_candidates={max_candidates}, max_seconds={max_seconds}"
    );
    println!("Moves: outsplit, insplit, out_amalgamation, in_amalgamation");
    println!("States are canonicalized up to vertex permutation.");
    println!();

    let result = search_graph_path(
        &start,
        &target,
        max_depth,
        max_dim,
        max_entry,
        max_states,
        max_candidates,
        Duration::from_secs(max_seconds),
    );
    match result {
        GraphSearchResult::Found {
            depth,
            meeting,
            path,
            visited,
            candidates,
            elapsed,
        } => {
            println!();
            println!("FOUND graph-only path at depth {depth}");
            println!("meeting = {:?}", meeting.data);
            println!("visited states = {visited}");
            println!("candidates generated = {candidates}");
            println!("elapsed = {:.3}s", elapsed.as_secs_f64());
            println!();
            print_path(&path);
        }
        GraphSearchResult::NotFound {
            visited,
            candidates,
            elapsed,
        } => {
            println!();
            println!("No graph-only path found within depth {max_depth}");
            println!("visited states = {visited}");
            println!("candidates generated = {candidates}");
            println!("elapsed = {:.3}s", elapsed.as_secs_f64());
        }
        GraphSearchResult::Capped {
            reason,
            visited,
            candidates,
            elapsed,
        } => {
            println!();
            println!("{reason} before exhaustion");
            println!("visited states = {visited}");
            println!("candidates generated = {candidates}");
            println!("elapsed = {:.3}s", elapsed.as_secs_f64());
        }
    }
}

#[derive(Debug)]
enum GraphSearchResult {
    Found {
        depth: usize,
        meeting: DynMatrix,
        path: Vec<PathStep>,
        visited: usize,
        candidates: usize,
        elapsed: Duration,
    },
    NotFound {
        visited: usize,
        candidates: usize,
        elapsed: Duration,
    },
    Capped {
        reason: &'static str,
        visited: usize,
        candidates: usize,
        elapsed: Duration,
    },
}

#[derive(Clone, Debug)]
struct PathStep {
    family: &'static str,
    from: DynMatrix,
    to: DynMatrix,
}

#[derive(Clone, Debug)]
struct ParentStep {
    parent: DynMatrix,
    family: &'static str,
}

#[derive(Default)]
struct LayerStats {
    candidates: usize,
    deduped_candidates: usize,
    pruned_by_entry: usize,
    collisions_with_seen: usize,
    discovered: usize,
    family_counts: BTreeMap<&'static str, usize>,
}

fn search_graph_path(
    start: &DynMatrix,
    target: &DynMatrix,
    max_depth: usize,
    max_dim: usize,
    max_entry: u32,
    max_states: usize,
    max_candidates: usize,
    max_runtime: Duration,
) -> GraphSearchResult {
    let started = Instant::now();
    let mut total_candidates = 0usize;
    let start = start.canonical_perm();
    let target = target.canonical_perm();
    if start == target {
        return GraphSearchResult::Found {
            depth: 0,
            meeting: start,
            path: Vec::new(),
            visited: 1,
            candidates: 0,
            elapsed: started.elapsed(),
        };
    }

    let mut fwd_seen = HashMap::<DynMatrix, usize>::new();
    let mut bwd_seen = HashMap::<DynMatrix, usize>::new();
    let mut fwd_parent = HashMap::<DynMatrix, ParentStep>::new();
    let mut bwd_parent = HashMap::<DynMatrix, ParentStep>::new();
    let mut fwd_frontier = VecDeque::<DynMatrix>::new();
    let mut bwd_frontier = VecDeque::<DynMatrix>::new();

    fwd_seen.insert(start.clone(), 0);
    bwd_seen.insert(target.clone(), 0);
    fwd_frontier.push_back(start);
    bwd_frontier.push_back(target);

    loop {
        let next_fwd_depth = fwd_frontier.front().and_then(|m| fwd_seen.get(m)).copied();
        let next_bwd_depth = bwd_frontier.front().and_then(|m| bwd_seen.get(m)).copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
        ) else {
            return GraphSearchResult::NotFound {
                visited: visited_union_size(&fwd_seen, &bwd_seen),
                candidates: total_candidates,
                elapsed: started.elapsed(),
            };
        };

        if layer_depth >= max_depth {
            return GraphSearchResult::NotFound {
                visited: visited_union_size(&fwd_seen, &bwd_seen),
                candidates: total_candidates,
                elapsed: started.elapsed(),
            };
        }

        let result = if expand_forward {
            expand_layer(
                "forward",
                layer_depth,
                &mut fwd_frontier,
                &mut fwd_seen,
                &mut fwd_parent,
                &bwd_seen,
                &bwd_parent,
                max_depth,
                max_dim,
                max_entry,
                max_states,
                max_candidates,
                max_runtime,
                started,
                &mut total_candidates,
            )
        } else {
            expand_layer(
                "backward",
                layer_depth,
                &mut bwd_frontier,
                &mut bwd_seen,
                &mut bwd_parent,
                &fwd_seen,
                &fwd_parent,
                max_depth,
                max_dim,
                max_entry,
                max_states,
                max_candidates,
                max_runtime,
                started,
                &mut total_candidates,
            )
        };

        if let Some(result) = result {
            return result;
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn expand_layer(
    side_name: &'static str,
    layer_depth: usize,
    frontier: &mut VecDeque<DynMatrix>,
    seen: &mut HashMap<DynMatrix, usize>,
    parent: &mut HashMap<DynMatrix, ParentStep>,
    other_seen: &HashMap<DynMatrix, usize>,
    other_parent: &HashMap<DynMatrix, ParentStep>,
    max_depth: usize,
    max_dim: usize,
    max_entry: u32,
    max_states: usize,
    max_candidates: usize,
    max_runtime: Duration,
    started: Instant,
    total_candidates: &mut usize,
) -> Option<GraphSearchResult> {
    let current_layer_len = frontier
        .iter()
        .take_while(|node| seen.get(*node).copied() == Some(layer_depth))
        .count();

    let expand_forward = side_name == "forward";
    let mut stats = LayerStats::default();
    let mut next_frontier = Vec::new();

    for _ in 0..current_layer_len {
        if started.elapsed() >= max_runtime {
            print_layer_summary(
                side_name,
                layer_depth,
                current_layer_len,
                &stats,
                visited_union_size(seen, other_seen),
            );
            return Some(GraphSearchResult::Capped {
                reason: "Time cap hit",
                visited: visited_union_size(seen, other_seen),
                candidates: *total_candidates,
                elapsed: started.elapsed(),
            });
        }

        let current = frontier.pop_front().expect("frontier length should match");
        let successors = enumerate_graph_move_successors(&current, max_dim);
        stats.candidates += successors.candidates;
        stats.deduped_candidates += successors.nodes.len();
        *total_candidates += successors.candidates;

        if *total_candidates > max_candidates {
            print_layer_summary(
                side_name,
                layer_depth,
                current_layer_len,
                &stats,
                visited_union_size(seen, other_seen),
            );
            return Some(GraphSearchResult::Capped {
                reason: "Candidate cap hit",
                visited: visited_union_size(seen, other_seen),
                candidates: *total_candidates,
                elapsed: started.elapsed(),
            });
        }

        for successor in successors.nodes {
            *stats.family_counts.entry(successor.family).or_default() += 1;

            if successor.matrix.max_entry() > max_entry {
                stats.pruned_by_entry += 1;
                continue;
            }
            if seen.contains_key(&successor.matrix) {
                stats.collisions_with_seen += 1;
                continue;
            }

            let next_depth = layer_depth + 1;
            parent.insert(
                successor.matrix.clone(),
                ParentStep {
                    parent: current.clone(),
                    family: successor.family,
                },
            );

            if let Some(&other_depth) = other_seen.get(&successor.matrix) {
                let depth = next_depth + other_depth;
                if depth <= max_depth {
                    print_layer_summary(
                        side_name,
                        layer_depth,
                        current_layer_len,
                        &stats,
                        visited_union_size(seen, other_seen),
                    );
                    let path = if expand_forward {
                        reconstruct_path(&successor.matrix, parent, other_parent)
                    } else {
                        reconstruct_path(&successor.matrix, other_parent, parent)
                    };
                    return Some(GraphSearchResult::Found {
                        depth,
                        meeting: successor.matrix,
                        path,
                        visited: visited_union_size(seen, other_seen),
                        candidates: *total_candidates,
                        elapsed: started.elapsed(),
                    });
                }
            }

            seen.insert(successor.matrix.clone(), next_depth);
            next_frontier.push(successor.matrix);
            stats.discovered += 1;

            if visited_union_size(seen, other_seen) > max_states {
                return Some(GraphSearchResult::Capped {
                    reason: "State cap hit",
                    visited: visited_union_size(seen, other_seen),
                    candidates: *total_candidates,
                    elapsed: started.elapsed(),
                });
            }
        }
    }

    print_layer_summary(
        side_name,
        layer_depth,
        current_layer_len,
        &stats,
        visited_union_size(seen, other_seen),
    );

    frontier.extend(next_frontier);
    None
}

fn print_layer_summary(
    side_name: &str,
    layer_depth: usize,
    frontier: usize,
    stats: &LayerStats,
    visited: usize,
) {
    println!(
        "{side_name} depth {layer_depth}: frontier={frontier}, candidates={}, deduped={}, pruned_by_entry={}, collisions={}, discovered={}, visited={}",
        stats.candidates,
        stats.deduped_candidates,
        stats.pruned_by_entry,
        stats.collisions_with_seen,
        stats.discovered,
        visited,
    );

    if !stats.family_counts.is_empty() {
        let families = stats
            .family_counts
            .iter()
            .map(|(family, count)| format!("{family}={count}"))
            .collect::<Vec<_>>()
            .join(", ");
        println!("  families: {families}");
    }
}

fn choose_next_layer(
    fwd_depth: Option<usize>,
    bwd_depth: Option<usize>,
    fwd_frontier_len: usize,
    bwd_frontier_len: usize,
) -> Option<(bool, usize)> {
    match (fwd_depth, bwd_depth) {
        (Some(fwd), Some(bwd)) => {
            if fwd < bwd {
                Some((true, fwd))
            } else if bwd < fwd {
                Some((false, bwd))
            } else {
                Some((fwd_frontier_len <= bwd_frontier_len, fwd))
            }
        }
        (Some(fwd), None) => Some((true, fwd)),
        (None, Some(bwd)) => Some((false, bwd)),
        (None, None) => None,
    }
}

fn reconstruct_path(
    meeting: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, ParentStep>,
    bwd_parent: &HashMap<DynMatrix, ParentStep>,
) -> Vec<PathStep> {
    let mut left = Vec::new();
    let mut current = meeting.clone();
    while let Some(step) = fwd_parent.get(&current) {
        left.push(PathStep {
            family: step.family,
            from: step.parent.clone(),
            to: current.clone(),
        });
        current = step.parent.clone();
    }
    left.reverse();

    let mut right = Vec::new();
    let mut current = meeting.clone();
    while let Some(step) = bwd_parent.get(&current) {
        right.push(PathStep {
            family: reverse_family(step.family),
            from: current.clone(),
            to: step.parent.clone(),
        });
        current = step.parent.clone();
    }

    left.extend(right);
    left
}

fn reverse_family(family: &'static str) -> &'static str {
    match family {
        "outsplit" => "out_amalgamation",
        "insplit" => "in_amalgamation",
        "out_amalgamation" => "outsplit",
        "in_amalgamation" => "insplit",
        _ => family,
    }
}

fn print_path(path: &[PathStep]) {
    if path.is_empty() {
        println!("empty path");
        return;
    }

    for (idx, step) in path.iter().enumerate() {
        println!(
            "{}. {}: {}x{} {:?} -> {}x{} {:?}",
            idx + 1,
            step.family,
            step.from.rows,
            step.from.cols,
            step.from.data,
            step.to.rows,
            step.to.cols,
            step.to.data
        );
    }
}

fn visited_union_size(
    left: &HashMap<DynMatrix, usize>,
    right: &HashMap<DynMatrix, usize>,
) -> usize {
    left.len()
        + right
            .keys()
            .filter(|node| !left.contains_key(*node))
            .count()
}
