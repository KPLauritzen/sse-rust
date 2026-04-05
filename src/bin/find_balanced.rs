use sse_core::balanced::{
    find_balanced_elementary_equivalence_2x2, BalancedSearchConfig2x2, BalancedSearchResult2x2,
};
use sse_core::matrix::SqMatrix;

fn main() {
    let mut case = String::from("brix_k3");
    let mut max_common_dim = 2usize;
    let mut max_entry = 8u32;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--case" => {
                case = args.next().expect("--case requires a value");
            }
            "--max-common-dim" => {
                max_common_dim = args
                    .next()
                    .expect("--max-common-dim requires a value")
                    .parse()
                    .expect("invalid max common dim");
            }
            "--max-entry" => {
                max_entry = args
                    .next()
                    .expect("--max-entry requires a value")
                    .parse()
                    .expect("invalid max entry");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_balanced [--case brix_k3|brix_k4|toy] [--max-common-dim N] [--max-entry N]"
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
        "toy" => (
            SqMatrix::new([[1, 0], [1, 0]]),
            SqMatrix::new([[0, 1], [0, 1]]),
        ),
        _ => panic!("unsupported case: {case}"),
    };

    let config = BalancedSearchConfig2x2 {
        max_common_dim,
        max_entry,
    };

    match find_balanced_elementary_equivalence_2x2(&a, &b, &config) {
        BalancedSearchResult2x2::Equivalent(witness) => {
            println!("Found balanced elementary witness");
            println!("A = {:?}", a);
            println!("B = {:?}", b);
            println!("S = {:?}", witness.s);
            println!("R_A = {:?}", witness.r_a);
            println!("R_B = {:?}", witness.r_b);
            println!("R_A S = {:?}", witness.r_a.mul(&witness.s));
        }
        BalancedSearchResult2x2::Exhausted => {
            println!("No balanced elementary witness found");
            println!("A = {:?}", a);
            println!("B = {:?}", b);
            println!(
                "search bounds: max_common_dim={}, max_entry={}",
                max_common_dim, max_entry
            );
        }
    }
}
