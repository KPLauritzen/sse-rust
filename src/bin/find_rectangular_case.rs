use std::collections::{HashMap, HashSet};

use sse_core::factorisation::enumerate_all_factorisations;
use sse_core::matrix::{DynMatrix, SqMatrix};
use sse_core::search::search_sse_2x2_with_telemetry;
use sse_core::types::{SearchConfig, SseResult};

fn main() {
    let mut generate_max_entry = 3u32;
    let mut search_max_entry = 6u32;
    let mut max_lag = 4usize;

    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--generate-max-entry" => {
                generate_max_entry = args
                    .next()
                    .expect("--generate-max-entry requires a value")
                    .parse()
                    .expect("invalid generate max entry");
            }
            "--search-max-entry" => {
                search_max_entry = args
                    .next()
                    .expect("--search-max-entry requires a value")
                    .parse()
                    .expect("invalid search max entry");
            }
            "--max-lag" => {
                max_lag = args
                    .next()
                    .expect("--max-lag requires a value")
                    .parse()
                    .expect("invalid max lag");
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_rectangular_case [--generate-max-entry N] [--search-max-entry N] [--max-lag N]"
                );
                return;
            }
            _ => panic!("unknown argument: {arg}"),
        }
    }

    let dim2_config = SearchConfig {
        max_lag,
        max_intermediate_dim: 2,
        max_entry: search_max_entry,
        ..SearchConfig::default()
    };
    let dim3_config = SearchConfig {
        max_lag,
        max_intermediate_dim: 3,
        max_entry: search_max_entry,
        ..SearchConfig::default()
    };

    let mut rectangular_buckets: HashMap<DynMatrix, HashSet<SqMatrix<2>>> = HashMap::new();

    for a in enumerate_candidate_matrices(generate_max_entry) {
        let a_dyn = DynMatrix::from_sq(&a);
        for (u, v) in enumerate_all_factorisations(&a_dyn, 3, generate_max_entry) {
            if u.rows == 2 && u.cols == 3 && v.rows == 3 && v.cols == 2 {
                let intermediate = v.mul(&u);
                rectangular_buckets
                    .entry(intermediate)
                    .or_default()
                    .insert(a.clone());
            }
        }
    }

    let mut tested_pairs = 0usize;
    for endpoints in rectangular_buckets.values() {
        if endpoints.len() < 2 {
            continue;
        }

        let mut matrices: Vec<SqMatrix<2>> = endpoints.iter().cloned().collect();
        matrices.sort();

        for i in 0..matrices.len() {
            for j in i + 1..matrices.len() {
                let a = &matrices[i];
                let b = &matrices[j];
                if a.canonical() == b.canonical() {
                    continue;
                }

                let (dim2_result, dim2_telemetry) =
                    search_sse_2x2_with_telemetry(a, b, &dim2_config);
                if matches!(dim2_result, SseResult::Equivalent(_)) {
                    continue;
                }

                let (dim3_result, dim3_telemetry) =
                    search_sse_2x2_with_telemetry(a, b, &dim3_config);
                tested_pairs += 1;

                if let SseResult::Equivalent(path) = dim3_result {
                    println!("Found rectangular-only candidate after testing {tested_pairs} pairs");
                    println!("A = {:?}", a);
                    println!("B = {:?}", b);
                    println!("dim2 = {}", describe_result(&dim2_result));
                    println!("dim3 = Equivalent(steps={})", path.steps.len());
                    println!(
                        "dim2 telemetry: expanded={} factorisations={} collisions={} frontier={} visited={}",
                        dim2_telemetry.frontier_nodes_expanded,
                        dim2_telemetry.factorisations_enumerated,
                        dim2_telemetry.collisions_with_seen,
                        dim2_telemetry.max_frontier_size,
                        dim2_telemetry.total_visited_nodes,
                    );
                    println!(
                        "dim3 telemetry: expanded={} factorisations={} collisions={} frontier={} visited={}",
                        dim3_telemetry.frontier_nodes_expanded,
                        dim3_telemetry.factorisations_enumerated,
                        dim3_telemetry.collisions_with_seen,
                        dim3_telemetry.max_frontier_size,
                        dim3_telemetry.total_visited_nodes,
                    );
                    return;
                }
            }
        }
    }

    println!("No rectangular-only candidate found");
}

fn enumerate_candidate_matrices(max_entry: u32) -> Vec<SqMatrix<2>> {
    let mut matrices = Vec::new();
    for a00 in 0..=max_entry {
        for a01 in 0..=max_entry {
            for a10 in 0..=max_entry {
                for a11 in 0..=max_entry {
                    let matrix = SqMatrix::new([[a00, a01], [a10, a11]]);
                    if matrix.entry_sum() < 4 {
                        continue;
                    }
                    if !matrix.is_irreducible() {
                        continue;
                    }
                    matrices.push(matrix);
                }
            }
        }
    }
    matrices
}

fn describe_result(result: &SseResult<2>) -> &'static str {
    match result {
        SseResult::Equivalent(_) => "Equivalent",
        SseResult::NotEquivalent(_) => "NotEquivalent",
        SseResult::Unknown => "Unknown",
    }
}
