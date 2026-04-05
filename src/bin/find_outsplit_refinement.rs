use sse_core::graph_moves::{enumerate_outsplits_2x2_to_3x3, find_common_outsplit_refinement_2x2};
use sse_core::matrix::SqMatrix;

fn main() {
    let mut case = String::from("brix_k3");

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--help" | "-h" => {
                println!("usage: find_outsplit_refinement [--case brix_k3|brix_k4]");
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
    println!("A out-splits: {}", a_outsplits.len());
    println!("B out-splits: {}", b_outsplits.len());

    match find_common_outsplit_refinement_2x2(&a, &b) {
        Some((left, right)) => {
            println!("Found common one-step out-split refinement");
            println!("A out-split = {:?}", left.outsplit);
            println!("B out-split = {:?}", right.outsplit);
        }
        None => {
            println!("No common one-step out-split refinement found");
        }
    }
}
