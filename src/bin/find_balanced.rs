use sse_core::balanced::{
    enumerate_balanced_bridge_return_neighbors_3x3, enumerate_balanced_elementary_neighbors_2x2,
    enumerate_balanced_neighbor_set_hits_2x2, enumerate_outsplit_bridge_states_2x2,
    find_balanced_elementary_equivalence_2x2, find_balanced_elementary_zigzag_meeting_2x2,
    BalancedBridgeReturnNeighbor3x3, BalancedSearchConfig2x2, BalancedSearchResult2x2,
};
use sse_core::graph_moves::enumerate_outsplits_2x2_to_3x3;
use sse_core::matrix::{DynMatrix, SqMatrix};
use sse_core::structured_surface::StructuredSurfaceDescriptor2x2;
use std::collections::BTreeSet;

fn main() {
    let descriptor = StructuredSurfaceDescriptor2x2::balanced_elementary_equivalence();
    let mut case = String::from("brix_k3");
    let mut max_common_dim = 2usize;
    let mut max_entry = 8u32;
    let mut print_neighbors = false;
    let mut search_zigzag = false;
    let mut bridge_max_entry = 8u32;
    let mut search_bridge_neighbor_seam = false;
    let mut search_bridge_return_seam = false;

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
            "--neighbors" => {
                print_neighbors = true;
            }
            "--zigzag" => {
                search_zigzag = true;
            }
            "--bridge-max-entry" => {
                bridge_max_entry = args
                    .next()
                    .expect("--bridge-max-entry requires a value")
                    .parse()
                    .expect("invalid bridge max entry");
            }
            "--bridge-neighbor-seam" => {
                search_bridge_neighbor_seam = true;
            }
            "--bridge-return-seam" => {
                search_bridge_return_seam = true;
            }
            "--help" | "-h" => {
                println!(
                    "usage: find_balanced [--case brix_k3|brix_k4|toy] [--max-common-dim N] [--max-entry N] [--neighbors] [--zigzag] [--bridge-max-entry N] [--bridge-neighbor-seam] [--bridge-return-seam]"
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

    println!(
        "search bounds: max_common_dim={}, max_entry={}",
        max_common_dim, max_entry
    );
    match find_balanced_elementary_equivalence_2x2(&a, &b, &config) {
        BalancedSearchResult2x2::Equivalent(witness) => {
            println!("Found {}", descriptor.reporting_label());
            println!("A = {:?}", a);
            println!("B = {:?}", b);
            println!("S = {:?}", witness.s);
            println!("R_A = {:?}", witness.r_a);
            println!("R_B = {:?}", witness.r_b);
            println!("R_A S = {:?}", witness.r_a.mul(&witness.s));
        }
        BalancedSearchResult2x2::Exhausted => {
            println!("No {} found", descriptor.reporting_label());
            println!("A = {:?}", a);
            println!("B = {:?}", b);
        }
    }

    if print_neighbors {
        println!();
        print_neighbors_for_side("A", &a, &config);
        print_neighbors_for_side("B", &b, &config);
    }

    if search_zigzag {
        println!();
        match find_balanced_elementary_zigzag_meeting_2x2(&a, &b, &config) {
            Some(result) => {
                println!("Found bounded balanced zig-zag meeting");
                println!("bridge = {:?}", result.bridge);
                println!("left S = {:?}", result.left_witness.s);
                println!("left R_A = {:?}", result.left_witness.r_a);
                println!("left R_bridge = {:?}", result.left_witness.r_b);
                println!("right S = {:?}", result.right_witness.s);
                println!("right R_B = {:?}", result.right_witness.r_a);
                println!("right R_bridge = {:?}", result.right_witness.r_b);
            }
            None => {
                println!("No bounded balanced zig-zag meeting found");
            }
        }
    }

    if search_bridge_neighbor_seam {
        println!();
        let a_bridges = enumerate_outsplit_bridge_states_2x2(&a, bridge_max_entry);
        let b_bridges = enumerate_outsplit_bridge_states_2x2(&b, bridge_max_entry);
        println!(
            "A-side canonical outsplit bridge states ({}): {:?}",
            a_bridges.len(),
            a_bridges
        );
        println!(
            "B-side canonical outsplit bridge states ({}): {:?}",
            b_bridges.len(),
            b_bridges
        );

        let hits = enumerate_balanced_neighbor_set_hits_2x2(&a_bridges, &b_bridges, &config);
        println!("A->B balanced bridge-neighbor hits: {}", hits.len());
        if hits.is_empty() {
            println!("No bounded balanced bridge-neighbor seam found");
        } else {
            for hit in hits {
                println!("  {:?} -> {:?}", hit.source, hit.target);
                println!("    via S = {:?}", hit.witness.s);
            }
        }
    }

    if search_bridge_return_seam {
        println!();
        let a_states = canonical_outsplit_states(&a);
        let b_states = canonical_outsplit_states(&b);
        println!(
            "A-side canonical 3x3 outsplit states ({}): {:?}",
            a_states.len(),
            a_states
        );
        println!(
            "B-side canonical 3x3 outsplit states ({}): {:?}",
            b_states.len(),
            b_states
        );

        let a_hits =
            collect_balanced_bridge_return_hits(&a_states, &b_states, bridge_max_entry, &config);
        println!("A->B balanced bridge-return hits: {}", a_hits.len());
        if a_hits.is_empty() {
            println!("No bounded A->B balanced bridge-return seam found");
        } else {
            print_bridge_return_hits(&a_hits);
        }

        let b_hits =
            collect_balanced_bridge_return_hits(&b_states, &a_states, bridge_max_entry, &config);
        println!("B->A balanced bridge-return hits: {}", b_hits.len());
        if b_hits.is_empty() {
            println!("No bounded B->A balanced bridge-return seam found");
        } else {
            print_bridge_return_hits(&b_hits);
        }
    }
}

fn print_neighbors_for_side(label: &str, matrix: &SqMatrix<2>, config: &BalancedSearchConfig2x2) {
    let neighbors = enumerate_balanced_elementary_neighbors_2x2(matrix, config);
    println!(
        "{}-side nontrivial balanced neighbors: {}",
        label,
        neighbors.len()
    );
    for neighbor in neighbors {
        println!("  {:?} via S = {:?}", neighbor.matrix, neighbor.witness.s);
    }
}

fn canonical_outsplit_states(m: &SqMatrix<2>) -> Vec<DynMatrix> {
    let mut seen = BTreeSet::new();
    let mut states = Vec::new();
    for witness in enumerate_outsplits_2x2_to_3x3(m) {
        let canon = witness.outsplit.canonical_perm();
        if seen.insert(canon.clone()) {
            states.push(canon);
        }
    }
    states
}

fn collect_balanced_bridge_return_hits(
    source_candidates: &[DynMatrix],
    target_candidates: &[DynMatrix],
    bridge_max_entry: u32,
    config: &BalancedSearchConfig2x2,
) -> Vec<(DynMatrix, BalancedBridgeReturnNeighbor3x3)> {
    let target_set = target_candidates.iter().cloned().collect::<BTreeSet<_>>();
    let mut hits = Vec::new();

    for source in source_candidates {
        for neighbor in
            enumerate_balanced_bridge_return_neighbors_3x3(source, bridge_max_entry, config)
        {
            if target_set.contains(&neighbor.matrix) {
                hits.push((source.clone(), neighbor));
            }
        }
    }

    hits
}

fn print_bridge_return_hits(hits: &[(DynMatrix, BalancedBridgeReturnNeighbor3x3)]) {
    for (source, hit) in hits {
        println!("  {:?} -> {:?}", source, hit.matrix);
        println!(
            "    via {:?} --balanced(S = {:?})-> {:?}",
            hit.source_bridge, hit.witness.s, hit.target_bridge
        );
    }
}
