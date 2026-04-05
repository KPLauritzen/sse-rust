use std::collections::HashMap;

use sse_core::graph_moves::{
    enumerate_insplits_2x2_to_3x3, enumerate_outsplits_2x2_to_3x3, OutsplitWitness,
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
                println!("usage: find_mixed_split_refinement [--case brix_k3|brix_k4]");
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

    let a_out = enumerate_outsplits_2x2_to_3x3(&a);
    let b_out = enumerate_outsplits_2x2_to_3x3(&b);
    let a_in = enumerate_insplits_2x2_to_3x3(&a);
    let b_in = enumerate_insplits_2x2_to_3x3(&b);

    println!(
        "A out/in counts: {}/{}",
        a_out.len(),
        a_in.len()
    );
    println!(
        "B out/in counts: {}/{}",
        b_out.len(),
        b_in.len()
    );
    let a_out_canon = canonical_map(&a_out);
    let a_in_canon = canonical_map(&a_in);
    let b_out_canon = canonical_map(&b_out);
    let b_in_canon = canonical_map(&b_in);
    println!("A out/in canonical sets equal: {}", a_out_canon == a_in_canon);
    println!("B out/in canonical sets equal: {}", b_out_canon == b_in_canon);

    report_overlap("out/out", &a_out, &b_out, &a_out_canon, &b_out_canon);
    report_overlap("in/in", &a_in, &b_in, &a_in_canon, &b_in_canon);
    report_overlap("out/in", &a_out, &b_in, &a_out_canon, &b_in_canon);
    report_overlap("in/out", &a_in, &b_out, &a_in_canon, &b_out_canon);
}

fn report_overlap(
    label: &str,
    left: &[OutsplitWitness],
    right: &[OutsplitWitness],
    left_map: &HashMap<DynMatrix, usize>,
    right_map: &HashMap<DynMatrix, usize>,
) {
    println!(
        "{} canonical counts: {}/{}",
        label,
        left_map.len(),
        right_map.len()
    );
    for (canon, left_idx) in left_map {
        if let Some(right_idx) = right_map.get(canon) {
            println!("Found {} common refinement", label);
            println!("left = {:?}", left[*left_idx].outsplit);
            println!("right = {:?}", right[*right_idx].outsplit);
            return;
        }
    }
    println!("No {} common refinement", label);
}

fn canonical_map(witnesses: &[OutsplitWitness]) -> HashMap<DynMatrix, usize> {
    let mut map = HashMap::new();
    for (idx, witness) in witnesses.iter().enumerate() {
        map.entry(witness.outsplit.canonical_perm()).or_insert(idx);
    }
    map
}
