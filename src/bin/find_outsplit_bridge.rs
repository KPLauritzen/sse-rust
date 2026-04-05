use std::collections::HashMap;

use sse_core::factorisation::enumerate_factorisations_3x3_to_2;
use sse_core::graph_moves::{enumerate_outsplits_2x2_to_3x3, OutsplitWitness2x2To3x3};
use sse_core::matrix::SqMatrix;

fn main() {
    let mut case = String::from("brix_k3");
    let mut max_entry = 8u32;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--max-entry" => {
                max_entry = args
                    .next()
                    .expect("--max-entry requires a value")
                    .parse()
                    .expect("invalid max entry");
            }
            "--help" | "-h" => {
                println!("usage: find_outsplit_bridge [--case brix_k3|brix_k4] [--max-entry N]");
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

    let a_outsplits = enumerate_outsplits_2x2_to_3x3(&a);
    let b_outsplits = enumerate_outsplits_2x2_to_3x3(&b);

    let a_bridges = compute_bridge_map(&a_outsplits, max_entry);
    let b_bridges = compute_bridge_map(&b_outsplits, max_entry);

    println!("A out-splits: {}", a_outsplits.len());
    println!("B out-splits: {}", b_outsplits.len());
    println!("A bridge states: {}", a_bridges.len());
    println!("B bridge states: {}", b_bridges.len());

    for (bridge, left_idx) in &a_bridges {
        if let Some(right_idx) = b_bridges.get(bridge) {
            println!("Found common 2x2 bridge {:?}", bridge);
            println!("A out-split = {:?}", a_outsplits[*left_idx].outsplit);
            println!("B out-split = {:?}", b_outsplits[*right_idx].outsplit);
            return;
        }
    }

    println!("No common 2x2 bridge found");
}

fn compute_bridge_map(
    outsplits: &[OutsplitWitness2x2To3x3],
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
