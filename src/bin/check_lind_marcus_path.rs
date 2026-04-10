use std::collections::{BTreeMap, HashSet};

use sse_core::factorisation::visit_all_factorisations_with_family;
use sse_core::graph_moves::{
    enumerate_in_amalgamations, enumerate_one_step_insplits, enumerate_one_step_outsplits,
    enumerate_out_amalgamations, enumerate_same_future_insplits_2x2_to_3x3,
    enumerate_same_past_outsplits_2x2_to_3x3,
};
use sse_core::matrix::DynMatrix;

fn main() {
    let max_intermediate_dim = 4usize;
    let max_entry = 5u32;
    let steps = lind_marcus_baker_steps();

    for (idx, (u, v)) in steps.iter().enumerate() {
        let current = u.mul(v);
        let next = v.mul(u);
        let found_families =
            generated_families_reaching(&current, &next, max_intermediate_dim, max_entry);

        println!(
            "Step {}: {}x{} -> {}x{}, factor max entry = {}",
            idx + 1,
            current.rows,
            current.cols,
            next.rows,
            next.cols,
            u.max_entry().max(v.max_entry())
        );
        println!("  current = {:?}", current.data);
        println!("  next    = {:?}", next.data);
        if found_families.is_empty() {
            println!("  generator coverage: MISSING");
        } else {
            println!("  generator coverage:");
            for (family, count) in found_families {
                println!("    {family}: {count}");
            }
        }
    }
}

fn lind_marcus_baker_steps() -> Vec<(DynMatrix, DynMatrix)> {
    vec![
        (
            DynMatrix::new(2, 3, vec![0, 1, 1, 1, 0, 0]),
            DynMatrix::new(3, 2, vec![2, 1, 1, 2, 0, 1]),
        ),
        (
            DynMatrix::new(3, 4, vec![1, 0, 2, 0, 0, 1, 1, 1, 0, 1, 0, 0]),
            DynMatrix::new(4, 3, vec![1, 0, 2, 1, 0, 0, 0, 1, 0, 1, 0, 1]),
        ),
        (
            DynMatrix::new(4, 4, vec![2, 0, 0, 1, 0, 2, 0, 1, 1, 0, 1, 0, 1, 1, 0, 1]),
            DynMatrix::new(4, 4, vec![0, 1, 1, 0, 0, 0, 1, 0, 0, 0, 0, 1, 1, 0, 0, 0]),
        ),
        (
            DynMatrix::new(4, 4, vec![0, 1, 1, 0, 0, 0, 0, 1, 0, 1, 0, 0, 1, 0, 0, 0]),
            DynMatrix::new(4, 4, vec![2, 0, 0, 1, 1, 1, 0, 1, 0, 1, 1, 0, 1, 0, 1, 0]),
        ),
        (
            DynMatrix::new(4, 4, vec![0, 1, 1, 1, 1, 0, 1, 1, 1, 0, 0, 0, 0, 1, 0, 0]),
            DynMatrix::new(4, 4, vec![0, 1, 0, 1, 0, 2, 1, 0, 0, 0, 1, 0, 1, 0, 0, 0]),
        ),
        (
            DynMatrix::new(4, 3, vec![1, 0, 1, 0, 1, 0, 0, 0, 1, 1, 0, 0]),
            DynMatrix::new(3, 4, vec![0, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0]),
        ),
        (
            DynMatrix::new(3, 2, vec![1, 0, 0, 5, 0, 1]),
            DynMatrix::new(2, 3, vec![1, 1, 1, 1, 0, 1]),
        ),
    ]
}

fn generated_families_reaching(
    current: &DynMatrix,
    target: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
) -> BTreeMap<&'static str, usize> {
    let target_canon = target.canonical_perm();
    let mut matches = BTreeMap::new();

    for representative in permutation_representatives(current) {
        for (family, successor) in
            generated_successors(&representative, max_intermediate_dim, max_entry)
        {
            if successor.canonical_perm() == target_canon {
                *matches.entry(family).or_insert(0) += 1;
            }
        }
    }

    matches
}

fn generated_successors(
    current: &DynMatrix,
    max_intermediate_dim: usize,
    max_entry: u32,
) -> Vec<(&'static str, DynMatrix)> {
    let mut successors = Vec::new();

    if let Some(current_sq) = current.to_sq::<2>() {
        for witness in enumerate_same_past_outsplits_2x2_to_3x3(&current_sq) {
            successors.push(("same_past_outsplit_2x2_to_3x3", witness.outsplit));
        }
        for witness in enumerate_same_future_insplits_2x2_to_3x3(&current_sq) {
            successors.push(("same_future_insplit_2x2_to_3x3", witness.outsplit));
        }
    }

    let dim = current.rows;

    if dim >= 3 && dim < max_intermediate_dim {
        for witness in enumerate_one_step_outsplits(current) {
            successors.push(("outsplit", witness.outsplit));
        }
        for witness in enumerate_one_step_insplits(current) {
            successors.push(("insplit", witness.outsplit));
        }
    }

    if dim > 2 {
        for witness in enumerate_out_amalgamations(current) {
            successors.push(("out_amalgamation", witness.outsplit));
        }
        for witness in enumerate_in_amalgamations(current) {
            successors.push(("in_amalgamation", witness.outsplit));
        }
    }

    visit_all_factorisations_with_family(
        current,
        max_intermediate_dim,
        max_entry,
        |family, u, v| {
            successors.push((family, v.mul(&u)));
        },
    );

    successors
}

fn permutation_representatives(matrix: &DynMatrix) -> Vec<DynMatrix> {
    let n = matrix.rows;
    let mut representatives = Vec::new();
    let mut seen = HashSet::new();
    let mut perm: Vec<usize> = (0..n).collect();

    loop {
        let representative = matrix.conjugate_by_perm(&perm);
        if seen.insert(representative.clone()) {
            representatives.push(representative);
        }
        if !next_permutation(&mut perm) {
            break;
        }
    }

    representatives
}

fn next_permutation(perm: &mut [usize]) -> bool {
    let n = perm.len();
    if n <= 1 {
        return false;
    }

    let mut i = n - 1;
    while i > 0 && perm[i - 1] >= perm[i] {
        i -= 1;
    }
    if i == 0 {
        return false;
    }

    let mut j = n - 1;
    while perm[j] <= perm[i - 1] {
        j -= 1;
    }
    perm.swap(i - 1, j);
    perm[i..].reverse();
    true
}
