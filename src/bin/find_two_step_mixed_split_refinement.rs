use std::collections::{HashMap, HashSet};

use sse_core::graph_moves::{
    enumerate_insplits_2x2_to_3x3, enumerate_one_step_split_refinements,
    enumerate_outsplits_2x2_to_3x3,
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
                println!("usage: find_two_step_mixed_split_refinement [--case brix_k3|brix_k4]");
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

    let a_start = canonical_first_step_split_states(&a);
    let b_start = canonical_first_step_split_states(&b);
    println!("A first-step mixed states: {}", a_start.len());
    println!("B first-step mixed states: {}", b_start.len());

    let a_second = canonical_second_step_split_states(&a_start);
    let b_second = canonical_second_step_split_states(&b_start);
    println!("A canonical second-step mixed states: {}", a_second.len());
    println!("B canonical second-step mixed states: {}", b_second.len());

    for (canon, left_parent) in &a_second {
        if let Some(right_parent) = b_second.get(canon) {
            println!("Found common two-step mixed split refinement");
            println!("meeting 4x4 state = {:?}", canon);
            println!("A parent 3x3 state = {:?}", left_parent);
            println!("B parent 3x3 state = {:?}", right_parent);
            return;
        }
    }

    println!("No common two-step mixed split refinement found");
}

fn canonical_first_step_split_states(m: &SqMatrix<2>) -> Vec<DynMatrix> {
    let mut seen = HashSet::new();
    let mut states = Vec::new();
    for witness in enumerate_outsplits_2x2_to_3x3(m)
        .into_iter()
        .chain(enumerate_insplits_2x2_to_3x3(m).into_iter())
    {
        let canon = witness.outsplit.canonical_perm();
        if seen.insert(canon.clone()) {
            states.push(canon);
        }
    }
    states
}

fn canonical_second_step_split_states(start: &[DynMatrix]) -> HashMap<DynMatrix, DynMatrix> {
    let mut states = HashMap::new();
    for parent in start {
        for child in enumerate_one_step_split_refinements(parent) {
            states.entry(child).or_insert_with(|| parent.clone());
        }
    }
    states
}
