use std::collections::{HashMap, VecDeque};

use sse_core::graph_moves::enumerate_graph_move_successors;
use sse_core::matrix::DynMatrix;

fn main() {
    let mut max_depth = 6usize;
    let mut max_dim = 5usize;
    let mut max_states = 1_000_000usize;

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
            "--max-states" => {
                max_states = args
                    .next()
                    .expect("--max-states requires a value")
                    .parse()
                    .expect("invalid max states");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_lind_marcus_graph_waypoints [--max-depth N] [--max-dim N] [--max-states N]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    println!(
        "Graph-only waypoint search: max_depth={max_depth}, max_dim={max_dim}, max_states={max_states}"
    );
    println!("Paths are printed between canonical permutation representatives.");

    let matrices = lind_marcus_baker_matrices();
    let waypoints = [
        ("step_1_A0_to_A1", 0usize, 1usize),
        ("step_2_A1_to_A2", 1usize, 2usize),
        ("step_3_A2_to_A3", 2usize, 3usize),
        ("step_4_A3_to_A4", 3usize, 4usize),
        ("step_5_A4_to_A5", 4usize, 5usize),
        ("step_6_A5_to_A6", 5usize, 6usize),
        ("step_7_A6_to_A7", 6usize, 7usize),
    ];

    let mut full_path = Vec::new();

    for (label, start_idx, target_idx) in waypoints {
        println!();
        println!("--- {label} ---");
        let result = search_graph_waypoint(
            &matrices[start_idx],
            &matrices[target_idx],
            max_depth,
            max_dim,
            max_states,
        );
        match result {
            WaypointResult::Found {
                depth,
                meeting,
                path,
            } => {
                println!("FOUND graph-only path at depth {depth}");
                println!("meeting = {:?}", meeting);
                print_path(&path);
                full_path.extend(path);
            }
            WaypointResult::NotFound { visited } => {
                println!("No graph-only path found within depth {max_depth}");
                println!("visited states = {visited}");
                return;
            }
            WaypointResult::Capped { visited } => {
                println!("State cap hit before exhaustion");
                println!("visited states = {visited}");
                return;
            }
        }
    }

    println!();
    println!("=== Full graph-only k=3 path, up to permutation ===");
    println!("total graph moves = {}", full_path.len());
    print_path(&full_path);
}

#[derive(Debug)]
enum WaypointResult {
    Found {
        depth: usize,
        meeting: DynMatrix,
        path: Vec<PathStep>,
    },
    NotFound {
        visited: usize,
    },
    Capped {
        visited: usize,
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

fn search_graph_waypoint(
    start: &DynMatrix,
    target: &DynMatrix,
    max_depth: usize,
    max_dim: usize,
    max_states: usize,
) -> WaypointResult {
    let start = start.canonical_perm();
    let target = target.canonical_perm();
    if start == target {
        return WaypointResult::Found {
            depth: 0,
            meeting: start,
            path: Vec::new(),
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
            return WaypointResult::NotFound {
                visited: visited_union_size(&fwd_seen, &bwd_seen),
            };
        };

        if layer_depth >= max_depth {
            return WaypointResult::NotFound {
                visited: visited_union_size(&fwd_seen, &bwd_seen),
            };
        }

        if expand_forward {
            if let Some(result) = expand_layer(
                "forward",
                layer_depth,
                &mut fwd_frontier,
                &mut fwd_seen,
                &mut fwd_parent,
                &bwd_seen,
                &bwd_parent,
                max_depth,
                max_dim,
                max_states,
            ) {
                return result;
            }
        } else if let Some(result) = expand_layer(
            "backward",
            layer_depth,
            &mut bwd_frontier,
            &mut bwd_seen,
            &mut bwd_parent,
            &fwd_seen,
            &fwd_parent,
            max_depth,
            max_dim,
            max_states,
        ) {
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
    max_states: usize,
) -> Option<WaypointResult> {
    let current_layer_len = frontier
        .iter()
        .take_while(|node| seen.get(*node).copied() == Some(layer_depth))
        .count();

    let mut candidates = 0usize;
    let mut deduped_candidates = 0usize;
    let mut discovered = 0usize;
    let mut next_frontier = Vec::new();
    let expand_forward = side_name == "forward";

    for _ in 0..current_layer_len {
        let current = frontier.pop_front().expect("frontier length should match");
        let successors = enumerate_graph_move_successors(&current, max_dim);
        candidates += successors.candidates;
        deduped_candidates += successors.nodes.len();

        for successor in successors.nodes {
            if seen.contains_key(&successor.matrix) {
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
                    println!(
                        "{side_name} depth {layer_depth}: frontier={current_layer_len}, candidates={candidates}, deduped={deduped_candidates}, discovered={discovered}"
                    );
                    let path = if expand_forward {
                        reconstruct_path(&successor.matrix, parent, other_parent)
                    } else {
                        reconstruct_path(&successor.matrix, other_parent, parent)
                    };
                    return Some(WaypointResult::Found {
                        depth,
                        meeting: successor.matrix,
                        path,
                    });
                }
            }

            seen.insert(successor.matrix.clone(), next_depth);
            next_frontier.push(successor.matrix);
            discovered += 1;

            if visited_union_size(seen, other_seen) > max_states {
                return Some(WaypointResult::Capped {
                    visited: visited_union_size(seen, other_seen),
                });
            }
        }
    }

    println!(
        "{side_name} depth {layer_depth}: frontier={current_layer_len}, candidates={candidates}, deduped={deduped_candidates}, discovered={discovered}, visited={}",
        visited_union_size(seen, other_seen)
    );

    frontier.extend(next_frontier);
    None
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
        println!("  empty path");
        return;
    }

    for (idx, step) in path.iter().enumerate() {
        println!(
            "  {}. {}: {}x{} {:?} -> {}x{} {:?}",
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

fn lind_marcus_baker_matrices() -> Vec<DynMatrix> {
    let mut matrices = Vec::new();
    for (u, v) in lind_marcus_baker_steps() {
        if matrices.is_empty() {
            matrices.push(u.mul(&v));
        }
        matrices.push(v.mul(&u));
    }
    matrices
}

fn lind_marcus_baker_steps() -> Vec<(DynMatrix, DynMatrix)> {
    vec![
        (
            DynMatrix::new(2, 3, vec![0, 1, 1, 1, 0, 0]),
            DynMatrix::new(3, 2, vec![2, 1, 1, 2, 0, 1]),
        ),
        (
            DynMatrix::new(3, 4, vec![1, 0, 2, 0, 0, 1, 1, 1, 0, 1, 0, 0]),
            DynMatrix::new(4, 3, vec![1, 0, 2, 1, 0, 0, 0, 1, 0, 1, 0, 1]),
        ),
        (
            DynMatrix::new(4, 4, vec![2, 0, 0, 1, 0, 2, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1]),
            DynMatrix::new(4, 4, vec![0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0]),
        ),
        (
            DynMatrix::new(4, 4, vec![0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0]),
            DynMatrix::new(4, 4, vec![2, 0, 0, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0]),
        ),
        (
            DynMatrix::new(4, 4, vec![0, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0]),
            DynMatrix::new(4, 4, vec![0, 1, 0, 1, 0, 2, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0]),
        ),
        (
            DynMatrix::new(4, 3, vec![1, 0, 1, 0, 1, 0, 0, 0, 1, 1, 0, 0]),
            DynMatrix::new(3, 4, vec![0, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0]),
        ),
        (
            DynMatrix::new(3, 2, vec![1, 0, 0, 5, 0, 1]),
            DynMatrix::new(2, 3, vec![1, 1, 1, 1, 0, 1]),
        ),
    ]
}
