// Diagnostic: do the 22-move Baker waypoint-expanded graph path and the
// 16-move blind endpoint graph path for brix_ruiz_k3 share any canonical
// intermediate matrices, or are they fully disjoint apart from the
// hardcoded endpoints?

use std::collections::HashMap;

use sse_core::matrix::DynMatrix;

fn m(rows: usize, cols: usize, data: &[u32]) -> DynMatrix {
    DynMatrix::new(rows, cols, data.to_vec())
}

fn baker_path() -> Vec<DynMatrix> {
    vec![
        m(2, 2, &[1, 2, 3, 1]),
        m(3, 3, &[0, 0, 1, 1, 1, 2, 2, 2, 1]),
        m(4, 4, &[0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 2, 1, 1, 2, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0,
            ],
        ),
        m(4, 4, &[0, 0, 0, 1, 0, 1, 1, 1, 1, 2, 1, 1, 1, 2, 1, 0]),
        m(
            5,
            5,
            &[
                0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 0,
            ],
        ),
        m(4, 4, &[0, 0, 1, 2, 1, 0, 1, 2, 2, 0, 1, 2, 1, 1, 0, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 1, 2, 1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 2, 0, 2, 1, 0, 1, 1, 1, 0, 0,
            ],
        ),
        m(4, 4, &[0, 0, 1, 1, 0, 1, 0, 2, 1, 1, 0, 1, 2, 1, 1, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 1, 1, 1, 1, 1, 0, 1, 2, 2, 1, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0,
            ],
        ),
        m(4, 4, &[0, 0, 1, 1, 2, 1, 0, 2, 1, 0, 0, 2, 1, 1, 1, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 2, 2, 1, 0, 0, 1, 0, 1, 1, 1, 1, 2, 0, 0, 0,
            ],
        ),
        m(4, 4, &[0, 0, 0, 1, 1, 0, 1, 1, 2, 2, 1, 0, 2, 2, 1, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 2, 1, 1, 1, 0, 2, 1, 1, 1, 1,
            ],
        ),
        m(4, 4, &[0, 0, 0, 1, 2, 0, 2, 2, 2, 1, 1, 0, 2, 1, 1, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 2, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1,
            ],
        ),
        m(4, 4, &[0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 3, 1, 1, 1, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 2, 2, 0, 3, 3, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0,
            ],
        ),
        m(4, 4, &[0, 0, 1, 1, 2, 0, 3, 5, 0, 0, 1, 1, 1, 1, 0, 1]),
        m(3, 3, &[0, 5, 5, 0, 1, 1, 1, 1, 1]),
        m(2, 2, &[0, 5, 1, 2]),
        m(3, 3, &[0, 0, 5, 1, 1, 1, 1, 1, 1]),
        m(2, 2, &[1, 1, 6, 1]),
    ]
}

fn blind_path() -> Vec<DynMatrix> {
    vec![
        m(2, 2, &[1, 2, 3, 1]),
        m(3, 3, &[0, 0, 2, 1, 1, 1, 2, 2, 1]),
        m(4, 4, &[0, 0, 1, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 2, 0, 1]),
        m(
            5,
            5,
            &[
                0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 2, 2, 1,
            ],
        ),
        m(4, 4, &[0, 0, 1, 1, 0, 1, 2, 2, 0, 1, 1, 1, 1, 1, 1, 0]),
        m(
            5,
            5,
            &[
                0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 0, 2, 0, 2, 2, 1,
            ],
        ),
        m(4, 4, &[0, 0, 0, 1, 1, 0, 2, 1, 1, 1, 1, 0, 2, 2, 2, 1]),
        m(
            5,
            5,
            &[
                0, 1, 0, 0, 1, 1, 0, 2, 0, 0, 1, 1, 0, 2, 1, 1, 0, 1, 1, 0, 1, 1, 0, 2, 1,
            ],
        ),
        m(4, 4, &[0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 2, 1, 2, 1, 1]),
        m(
            5,
            5,
            &[
                0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1, 0, 2, 1, 1, 0, 0, 1, 1, 1, 1, 0, 2, 1,
            ],
        ),
        m(4, 4, &[0, 0, 1, 1, 0, 1, 0, 1, 1, 2, 0, 1, 2, 2, 1, 1]),
        m(
            5,
            5,
            &[
                0, 1, 0, 0, 1, 1, 0, 1, 2, 0, 2, 1, 0, 2, 1, 0, 0, 1, 1, 0, 2, 1, 0, 2, 1,
            ],
        ),
        m(4, 4, &[0, 0, 0, 1, 0, 1, 1, 0, 2, 2, 0, 1, 3, 4, 1, 1]),
        m(
            5,
            5,
            &[
                0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 0, 1, 0, 1, 1, 0, 1, 3, 1, 4, 1,
            ],
        ),
        m(4, 4, &[0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 0, 4, 4, 1, 1]),
        m(3, 3, &[0, 0, 2, 1, 1, 4, 1, 1, 1]),
        m(2, 2, &[1, 1, 6, 1]),
    ]
}

fn main() {
    let baker = baker_path();
    let blind = blind_path();

    println!("Baker waypoint-expanded path: {} matrices", baker.len());
    println!("Blind endpoint path:           {} matrices", blind.len());

    let baker_canon: Vec<DynMatrix> = baker.iter().map(|m| m.canonical_perm()).collect();
    let blind_canon: Vec<DynMatrix> = blind.iter().map(|m| m.canonical_perm()).collect();

    // Index baker canonical matrices for fast lookup.
    let mut baker_index: HashMap<DynMatrix, Vec<usize>> = HashMap::new();
    for (i, c) in baker_canon.iter().enumerate() {
        baker_index.entry(c.clone()).or_default().push(i);
    }

    println!();
    println!("Shared canonical matrices (Baker index -> Blind index):");
    let mut shared_pairs = 0usize;
    for (j, c) in blind_canon.iter().enumerate() {
        if let Some(idxs) = baker_index.get(c) {
            for &i in idxs {
                println!(
                    "  Baker[{i:>2}] {}x{} = Blind[{j:>2}] {}x{}",
                    baker[i].rows, baker[i].cols, blind[j].rows, blind[j].cols
                );
                shared_pairs += 1;
            }
        }
    }
    println!();
    println!("Total shared (canonical) matrices: {shared_pairs}");

    // Sanity: report which matrices are unique to each path.
    let blind_set: std::collections::HashSet<&DynMatrix> = blind_canon.iter().collect();
    let baker_only: Vec<usize> = baker_canon
        .iter()
        .enumerate()
        .filter(|(_, c)| !blind_set.contains(c))
        .map(|(i, _)| i)
        .collect();
    let baker_set: std::collections::HashSet<&DynMatrix> = baker_canon.iter().collect();
    let blind_only: Vec<usize> = blind_canon
        .iter()
        .enumerate()
        .filter(|(_, c)| !baker_set.contains(c))
        .map(|(i, _)| i)
        .collect();
    println!(
        "Baker-only intermediates: {} (positions {:?})",
        baker_only.len(),
        baker_only
    );
    println!(
        "Blind-only intermediates: {} (positions {:?})",
        blind_only.len(),
        blind_only
    );

    // Per-dimension intersection counts.
    println!();
    println!("Dimension breakdown of shared canonical matrices:");
    let mut by_dim: std::collections::BTreeMap<usize, usize> = std::collections::BTreeMap::new();
    for (j, c) in blind_canon.iter().enumerate() {
        if baker_index.contains_key(c) {
            *by_dim.entry(blind[j].rows).or_default() += 1;
        }
    }
    for (d, n) in by_dim {
        println!("  {d}x{d}: {n}");
    }
}
