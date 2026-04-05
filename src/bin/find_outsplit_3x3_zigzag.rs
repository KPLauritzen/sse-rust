use std::collections::{HashMap, HashSet, VecDeque};

use sse_core::graph_moves::{
    enumerate_3x3_outsplit_zigzag_neighbors, enumerate_outsplits_2x2_to_3x3,
};
use sse_core::matrix::{DynMatrix, SqMatrix};

fn main() {
    let mut case = String::from("brix_k3");
    let mut bridge_max_entry = 8u32;
    let mut max_depth = 2usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--bridge-max-entry" => {
                bridge_max_entry = args
                    .next()
                    .expect("--bridge-max-entry requires a value")
                    .parse()
                    .expect("invalid bridge max entry");
            }
            "--max-depth" => {
                max_depth = args
                    .next()
                    .expect("--max-depth requires a value")
                    .parse()
                    .expect("invalid max depth");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_outsplit_3x3_zigzag [--case brix_k3|brix_k4] [--bridge-max-entry N] [--max-depth N]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let (a, b) = match case.as_str() {
        "brix_k3" => (
            SqMatrix::new([[1, 3], [2, 1]]),
            SqMatrix::new([[1, 6], [1, 1]]),
        ),
        "brix_k4" => (
            SqMatrix::new([[1, 4], [3, 1]]),
            SqMatrix::new([[1, 12], [1, 1]]),
        ),
        _ => panic!("unsupported case: {case}"),
    };

    let left_start = canonical_outsplit_states(&a);
    let right_start = canonical_outsplit_states(&b);
    println!("A start states: {}", left_start.len());
    println!("B start states: {}", right_start.len());

    let mut cache = HashMap::<DynMatrix, Vec<DynMatrix>>::new();
    match find_bounded_zigzag_meeting(
        &left_start,
        &right_start,
        max_depth,
        bridge_max_entry,
        &mut cache,
    ) {
        Some(result) => {
            println!("Found 3x3 zig-zag meeting");
            println!("meeting state = {:?}", result.meeting_state);
            println!("left depth = {}", result.left_depth);
            println!("right depth = {}", result.right_depth);
        }
        None => {
            println!("No bounded 3x3 zig-zag meeting found");
        }
    }
}

fn canonical_outsplit_states(m: &SqMatrix<2>) -> Vec<DynMatrix> {
    let mut seen = HashSet::new();
    let mut states = Vec::new();
    for witness in enumerate_outsplits_2x2_to_3x3(m) {
        let canon = witness.outsplit.canonical_perm();
        if seen.insert(canon.clone()) {
            states.push(canon);
        }
    }
    states
}

#[derive(Debug)]
struct MeetingResult {
    meeting_state: DynMatrix,
    left_depth: usize,
    right_depth: usize,
}

fn find_bounded_zigzag_meeting(
    left_start: &[DynMatrix],
    right_start: &[DynMatrix],
    max_depth: usize,
    bridge_max_entry: u32,
    cache: &mut HashMap<DynMatrix, Vec<DynMatrix>>,
) -> Option<MeetingResult> {
    let mut left_seen = HashMap::<DynMatrix, usize>::new();
    let mut right_seen = HashMap::<DynMatrix, usize>::new();
    let mut left_frontier = VecDeque::<DynMatrix>::new();
    let mut right_frontier = VecDeque::<DynMatrix>::new();

    for state in left_start {
        left_seen.insert(state.clone(), 0);
        left_frontier.push_back(state.clone());
    }
    for state in right_start {
        if let Some(&left_depth) = left_seen.get(state) {
            return Some(MeetingResult {
                meeting_state: state.clone(),
                left_depth,
                right_depth: 0,
            });
        }
        right_seen.insert(state.clone(), 0);
        right_frontier.push_back(state.clone());
    }

    for layer in 0..max_depth {
        let expand_left = left_frontier.len() <= right_frontier.len();
        let (frontier, seen, other_seen, side_name) = if expand_left {
            (&mut left_frontier, &mut left_seen, &right_seen, "A")
        } else {
            (&mut right_frontier, &mut right_seen, &left_seen, "B")
        };

        let current_len = frontier.len();
        let current_depth = frontier
            .front()
            .and_then(|state| seen.get(state))
            .copied()
            .unwrap_or(layer);
        println!(
            "{}-side layer {}: frontier {} states at depth {}",
            side_name, layer, current_len, current_depth
        );

        for _ in 0..current_len {
            let current = frontier.pop_front().expect("frontier length should match");
            let current_depth = seen[&current];
            if current_depth >= max_depth {
                continue;
            }

            let neighbors = cache
                .entry(current.clone())
                .or_insert_with(|| {
                    enumerate_3x3_outsplit_zigzag_neighbors(&current, bridge_max_entry)
                })
                .clone();
            println!(
                "{}-side state {:?} has {} zig-zag neighbors",
                side_name,
                current,
                neighbors.len()
            );

            for neighbor in neighbors {
                if seen.contains_key(&neighbor) {
                    continue;
                }
                let next_depth = current_depth + 1;
                if let Some(&other_depth) = other_seen.get(&neighbor) {
                    return Some(MeetingResult {
                        meeting_state: neighbor,
                        left_depth: if expand_left { next_depth } else { other_depth },
                        right_depth: if expand_left { other_depth } else { next_depth },
                    });
                }
                seen.insert(neighbor.clone(), next_depth);
                frontier.push_back(neighbor);
            }
        }
    }

    println!("A visited states: {}", left_seen.len());
    println!("B visited states: {}", right_seen.len());
    None
}
