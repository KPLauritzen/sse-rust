use sse_core::matrix::DynMatrix;

fn main() {
    let baker_waypoints = lind_marcus_baker_matrices();
    let graph_path = baker_graph_only_path();
    let block_lengths = [1usize, 5, 2, 2, 6, 3, 3];

    assert_eq!(graph_path.len(), 23, "22 moves should give 23 matrices");
    assert_eq!(
        block_lengths.iter().sum::<usize>(),
        graph_path.len() - 1,
        "block lengths should cover the full graph path"
    );
    assert_eq!(
        baker_waypoints.len(),
        block_lengths.len() + 1,
        "need one more waypoint than Baker steps"
    );

    println!("Lind-Marcus/Baker reconstruction from 22 graph moves");
    println!("graph moves = {}", graph_path.len() - 1);
    println!("Baker steps = {}", block_lengths.len());
    println!();

    let mut offset = 0usize;
    for (idx, &block_len) in block_lengths.iter().enumerate() {
        let start = &graph_path[offset];
        let end = &graph_path[offset + block_len];
        let baker_start = &baker_waypoints[idx];
        let baker_end = &baker_waypoints[idx + 1];

        let start_matches = start.canonical_perm() == baker_start.canonical_perm();
        let end_matches = end.canonical_perm() == baker_end.canonical_perm();

        println!(
            "Baker step {}: graph block [{}..{}] length {}",
            idx + 1,
            offset,
            offset + block_len,
            block_len
        );
        println!(
            "  start matches waypoint A{}: {}",
            idx,
            yes_no(start_matches)
        );
        println!(
            "  end matches waypoint A{}:   {}",
            idx + 1,
            yes_no(end_matches)
        );
        println!(
            "  graph block: {}x{} {} -> {}x{} {}",
            start.rows,
            start.cols,
            format_matrix(start),
            end.rows,
            end.cols,
            format_matrix(end)
        );
        println!(
            "  Baker step:  {}x{} {} -> {}x{} {}",
            baker_start.rows,
            baker_start.cols,
            format_matrix(baker_start),
            baker_end.rows,
            baker_end.cols,
            format_matrix(baker_end)
        );
        println!();

        assert!(start_matches, "block start does not match Baker waypoint");
        assert!(end_matches, "block end does not match Baker waypoint");
        offset += block_len;
    }

    assert_eq!(offset, graph_path.len() - 1);
    println!("All 7 Baker waypoint transitions are recovered by the 22 graph-only path.");
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

fn format_matrix(matrix: &DynMatrix) -> String {
    let rows: Vec<String> = (0..matrix.rows)
        .map(|row| {
            let entries: Vec<String> = (0..matrix.cols)
                .map(|col| matrix.get(row, col).to_string())
                .collect();
            format!("[{}]", entries.join(","))
        })
        .collect();
    rows.join(" ")
}

fn lind_marcus_baker_matrices() -> Vec<DynMatrix> {
    let mut matrices = Vec::new();
    for (u, v) in lind_marcus_baker_steps() {
        if matrices.is_empty() {
            matrices.push(u.mul(&v));
        }
        matrices.push(v.mul(&u));
    }
    matrices
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
