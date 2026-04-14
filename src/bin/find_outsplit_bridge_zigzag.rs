use std::collections::HashMap;

use sse_core::factorisation::enumerate_factorisations_3x3_to_2;
use sse_core::graph_moves::{enumerate_outsplits_2x2_to_3x3, OutsplitWitness};
use sse_core::matrix::SqMatrix;
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{FrontierMode, MoveFamilyPolicy, SearchConfig, SseResult};

fn main() {
    let mut case = String::from("brix_k3");
    let mut bridge_max_entry = 8u32;
    let mut search_max_lag = 6usize;
    let mut search_max_dim = 3usize;
    let mut search_max_entry = 25u32;

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
            "--search-max-lag" => {
                search_max_lag = args
                    .next()
                    .expect("--search-max-lag requires a value")
                    .parse()
                    .expect("invalid search max lag");
            }
            "--search-max-dim" => {
                search_max_dim = args
                    .next()
                    .expect("--search-max-dim requires a value")
                    .parse()
                    .expect("invalid search max dim");
            }
            "--search-max-entry" => {
                search_max_entry = args
                    .next()
                    .expect("--search-max-entry requires a value")
                    .parse()
                    .expect("invalid search max entry");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_outsplit_bridge_zigzag [--case brix_k3|brix_k4] [--bridge-max-entry N] [--search-max-lag N] [--search-max-dim N] [--search-max-entry N]"
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

    let config = SearchConfig {
        max_lag: search_max_lag,
        max_intermediate_dim: search_max_dim,
        max_entry: search_max_entry,
        frontier_mode: FrontierMode::Bfs,
        move_family_policy: MoveFamilyPolicy::Mixed,
        beam_width: None,
    };

    let a_bridges = compute_bridge_map(&enumerate_outsplits_2x2_to_3x3(&a), bridge_max_entry);
    let b_bridges = compute_bridge_map(&enumerate_outsplits_2x2_to_3x3(&b), bridge_max_entry);

    let mut a_states: Vec<SqMatrix<2>> = a_bridges.keys().cloned().collect();
    let mut b_states: Vec<SqMatrix<2>> = b_bridges.keys().cloned().collect();
    a_states.sort();
    b_states.sort();

    println!("A bridge states ({}): {:?}", a_states.len(), a_states);
    println!("B bridge states ({}): {:?}", b_states.len(), b_states);

    for left in &a_states {
        for right in &b_states {
            let (result, telemetry) = search_sse_2x2_with_telemetry(left, right, &config);
            match result {
                SseResult::Equivalent(path) => {
                    println!("Found bridge zig-zag");
                    println!("A bridge = {:?}", left);
                    println!("B bridge = {:?}", right);
                    println!("lag = {}", path.steps.len());
                    println!(
                        "frontier_nodes_expanded = {}",
                        telemetry.frontier_nodes_expanded
                    );
                    return;
                }
                SseResult::EquivalentByConcreteShift(_witness) => {
                    println!("Found bridge zig-zag via concrete-shift witness");
                    println!("A bridge = {:?}", left);
                    println!("B bridge = {:?}", right);
                    return;
                }
                SseResult::NotEquivalent(reason) => {
                    println!(
                        "Bridge pair ruled out by invariants: {:?} vs {:?}: {}",
                        left, right, reason
                    );
                }
                SseResult::Unknown => {
                    println!(
                        "Bridge pair stayed unknown: {:?} vs {:?} (expanded {})",
                        left, right, telemetry.frontier_nodes_expanded
                    );
                }
            }
        }
    }

    println!("No bridge zig-zag found");
}

fn compute_bridge_map(
    outsplits: &[OutsplitWitness],
    max_entry: u32,
) -> HashMap<SqMatrix<2>, usize> {
    let mut map = HashMap::new();
    for (idx, witness) in outsplits.iter().enumerate() {
        for (u, v) in enumerate_factorisations_3x3_to_2(&witness.outsplit, max_entry) {
            let bridge = v
                .mul(&u)
                .to_sq::<2>()
                .expect("3x3-to-2 factorisation should produce a 2x2 bridge");
            map.entry(bridge.canonical()).or_insert(idx);
        }
    }
    map
}
