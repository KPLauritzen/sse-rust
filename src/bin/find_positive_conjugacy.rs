use sse_core::conjugacy::{
    find_positive_conjugacy_2x2, PositiveConjugacySearchConfig2x2, PositiveConjugacySearchResult2x2,
};
use sse_core::matrix::SqMatrix;

fn main() {
    let mut case = String::from("brix_k3");
    let mut max_conjugator_entry = 8u32;
    let mut sample_points = 64usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--max-conjugator-entry" => {
                max_conjugator_entry = args
                    .next()
                    .expect("--max-conjugator-entry requires a value")
                    .parse()
                    .expect("invalid max conjugator entry");
            }
            "--sample-points" => {
                sample_points = args
                    .next()
                    .expect("--sample-points requires a value")
                    .parse()
                    .expect("invalid sample point count");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_positive_conjugacy [--case brix_k3|brix_k4] [--max-conjugator-entry N] [--sample-points N]"
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

    let config = PositiveConjugacySearchConfig2x2 {
        max_conjugator_entry,
        sample_points,
    };

    match find_positive_conjugacy_2x2(&a, &b, &config) {
        PositiveConjugacySearchResult2x2::Equivalent(witness) => {
            println!("Found positive conjugacy witness");
            println!("A = {:?}", a);
            println!("B = {:?}", b);
            println!("G = {:?}", witness.conjugator);
            let min_entry = witness
                .sampled_path
                .iter()
                .map(|m| {
                    m.data
                        .iter()
                        .flat_map(|row| row.iter())
                        .copied()
                        .fold(f64::INFINITY, f64::min)
                })
                .fold(f64::INFINITY, f64::min);
            println!("sampled matrices = {}", witness.sampled_path.len());
            println!("minimum sampled entry = {:.6}", min_entry);
            println!(
                "final sample = {:?}",
                witness
                    .sampled_path
                    .last()
                    .expect("path should be non-empty")
                    .data
            );
        }
        PositiveConjugacySearchResult2x2::Exhausted => {
            println!("No positive conjugacy witness found");
        }
    }
}
