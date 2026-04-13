use std::collections::BTreeMap;

use sse_core::graph_moves::find_graph_move_witnesses_between;
use sse_core::matrix::DynMatrix;

fn main() {
    let path = baker_graph_only_path();
    let baker_blocks = baker_block_ranges();

    println!("k=3 graph-only path compression probe");
    println!("path length = {} moves", path.len() - 1);
    println!();

    println!("Baker block endpoints:");
    for (idx, (start, end)) in baker_blocks.iter().copied().enumerate() {
        report_segment(&path, start, end, Some(idx + 1));
    }

    println!();
    println!("All contiguous segments with a direct one-step graph move witness:");
    let mut found_any = false;
    for start in 0..path.len() - 1 {
        for end in start + 2..path.len() {
            let witnesses = find_graph_move_witnesses_between(&path[start], &path[end]);
            if witnesses.is_empty() {
                continue;
            }

            found_any = true;
            let family_counts = family_counts(&witnesses);
            println!(
                "  [{}..{}] length {}: {}x{} -> {}x{} via {}",
                start,
                end,
                end - start,
                path[start].rows,
                path[start].cols,
                path[end].rows,
                path[end].cols,
                format_family_counts(&family_counts),
            );
        }
    }

    if !found_any {
        println!("  none");
    }
}

fn report_segment(path: &[DynMatrix], start: usize, end: usize, baker_step: Option<usize>) {
    let witnesses = find_graph_move_witnesses_between(&path[start], &path[end]);
    let label = baker_step
        .map(|idx| format!("  step {idx}:"))
        .unwrap_or_else(|| "  segment:".to_string());

    println!(
        "{label} [{}..{}] length {}: {}x{} -> {}x{}",
        start,
        end,
        end - start,
        path[start].rows,
        path[start].cols,
        path[end].rows,
        path[end].cols,
    );

    if witnesses.is_empty() {
        println!("    direct graph-move compression: no");
    } else {
        let family_counts = family_counts(&witnesses);
        println!(
            "    direct graph-move compression: yes ({})",
            format_family_counts(&family_counts)
        );
    }
}

fn family_counts(
    witnesses: &[sse_core::graph_moves::GraphMoveSuccessor],
) -> BTreeMap<&'static str, usize> {
    let mut counts = BTreeMap::new();
    for witness in witnesses {
        *counts.entry(witness.family).or_default() += 1;
    }
    counts
}

fn format_family_counts(counts: &BTreeMap<&'static str, usize>) -> String {
    counts
        .iter()
        .map(|(family, count)| format!("{family} x{count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn baker_block_ranges() -> Vec<(usize, usize)> {
    let block_lengths = [1usize, 5, 2, 2, 6, 3, 3];
    let mut start = 0usize;
    let mut ranges = Vec::with_capacity(block_lengths.len());
    for block_len in block_lengths {
        ranges.push((start, start + block_len));
        start += block_len;
    }
    ranges
}

fn baker_graph_only_path() -> Vec<DynMatrix> {
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

fn m(rows: usize, cols: usize, data: &[u32]) -> DynMatrix {
    DynMatrix::new(rows, cols, data.to_vec())
}
