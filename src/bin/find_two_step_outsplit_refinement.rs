use std::collections::HashMap;

use sse_core::graph_moves::{
    enumerate_one_step_outsplits, enumerate_outsplits_2x2_to_3x3,
    find_common_two_step_outsplit_refinement_2x2,
};
use sse_core::matrix::{DynMatrix, SqMatrix};

fn main() {
    let mut case = String::from("brix_k3");

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--help" | "-h" => {
                println!("usage: find_two_step_outsplit_refinement [--case brix_k3|brix_k4]");
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

    let a_first = enumerate_outsplits_2x2_to_3x3(&a);
    let b_first = enumerate_outsplits_2x2_to_3x3(&b);
    let (a_second_total, a_second_canonical) = two_step_counts(&a_first);
    let (b_second_total, b_second_canonical) = two_step_counts(&b_first);

    println!("A first-step out-splits: {}", a_first.len());
    println!("B first-step out-splits: {}", b_first.len());
    println!("A second-step out-splits: {}", a_second_total);
    println!("B second-step out-splits: {}", b_second_total);
    println!("A canonical 4x4 refinements: {}", a_second_canonical);
    println!("B canonical 4x4 refinements: {}", b_second_canonical);

    match find_common_two_step_outsplit_refinement_2x2(&a, &b) {
        Some((left, right)) => {
            println!("Found common two-step out-split refinement");
            println!("A 3x3 out-split = {:?}", left.first.outsplit);
            println!("A 4x4 out-split = {:?}", left.second.outsplit);
            println!("B 3x3 out-split = {:?}", right.first.outsplit);
            println!("B 4x4 out-split = {:?}", right.second.outsplit);
        }
        None => {
            println!("No common two-step out-split refinement found");
        }
    }
}

fn two_step_counts(first_step: &[sse_core::graph_moves::OutsplitWitness]) -> (usize, usize) {
    let mut total = 0usize;
    let mut canonical = HashMap::<DynMatrix, ()>::new();
    for witness in first_step {
        for second in enumerate_one_step_outsplits(&witness.outsplit) {
            total += 1;
            canonical.entry(second.outsplit.canonical_perm()).or_insert(());
        }
    }
    (total, canonical.len())
}
