use std::collections::{BTreeMap, BTreeSet};

use sse_core::factorisation::{
    binary_sparse_factorisation_4x4_to_3_permutation_orbit_key,
    visit_all_factorisations_with_family,
};
use sse_core::matrix::DynMatrix;
use sse_core::search::execute_search_request_and_observer;
use sse_core::search_observer::{SearchEdgeStatus, SearchEvent, SearchObserver};
use sse_core::types::{FrontierMode, MoveFamilyPolicy, SearchConfig, SearchRequest, SearchStage};

const BINARY_SPARSE_3X3_TO_4: &str = "binary_sparse_rectangular_factorisation_3x3_to_4";
const BINARY_SPARSE_4X3_TO_3: &str = "binary_sparse_rectangular_factorisation_4x3_to_3";
const SINGLE_ROW_SPLIT_3X3_TO_4: &str = "single_row_split_3x3_to_4x4";

#[derive(Default)]
struct FamilySourceObserver {
    family_counts: BTreeMap<&'static str, BTreeMap<DynMatrix, SourceStats>>,
}

#[derive(Clone, Default)]
struct SourceStats {
    total_edges: usize,
    discovered_edges: usize,
    exact_meets: usize,
    seen_collisions: usize,
}

#[derive(Default)]
struct BinarySparseSummary {
    callbacks: usize,
    orbit_callbacks: usize,
    exact_successors: usize,
    canonical_successors: usize,
}

#[derive(Default)]
struct RowSplitSummary {
    kept_callbacks: usize,
    raw_unquotiented_callbacks: usize,
    twin_orbits: usize,
    exact_successors: usize,
    canonical_successors: usize,
}

impl SearchObserver for FamilySourceObserver {
    fn on_event(&mut self, event: &SearchEvent) {
        let SearchEvent::Layer(edges) = event else {
            return;
        };

        for edge in edges {
            if !matches!(
                edge.move_family,
                BINARY_SPARSE_3X3_TO_4 | BINARY_SPARSE_4X3_TO_3 | SINGLE_ROW_SPLIT_3X3_TO_4
            ) {
                continue;
            }
            let stats = self
                .family_counts
                .entry(edge.move_family)
                .or_default()
                .entry(edge.from_orig.clone())
                .or_default();
            stats.total_edges += 1;
            match edge.status {
                SearchEdgeStatus::Discovered => stats.discovered_edges += 1,
                SearchEdgeStatus::ExactMeet => stats.exact_meets += 1,
                SearchEdgeStatus::SeenCollision => stats.seen_collisions += 1,
            }
        }
    }
}

fn main() -> Result<(), String> {
    let request = SearchRequest {
        source: DynMatrix::new(2, 2, vec![1, 3, 2, 1]),
        target: DynMatrix::new(2, 2, vec![1, 6, 1, 1]),
        config: SearchConfig {
            max_lag: 6,
            max_intermediate_dim: 4,
            max_entry: 6,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
        },
        stage: SearchStage::EndpointSearch,
        guide_artifacts: Vec::new(),
        guided_refinement: Default::default(),
        shortcut_search: Default::default(),
    };

    let mut observer = FamilySourceObserver::default();
    let (result, telemetry) = execute_search_request_and_observer(&request, Some(&mut observer))?;
    eprintln!("result={result:?}");
    eprintln!(
        "telemetry: layers={} factorisations={} candidates_after_pruning={} discovered={}",
        telemetry.layers.len(),
        telemetry.factorisations_enumerated,
        telemetry.candidates_after_pruning,
        telemetry.discovered_nodes
    );

    println!("Control family breakdown");
    for family in [
        BINARY_SPARSE_3X3_TO_4,
        BINARY_SPARSE_4X3_TO_3,
        SINGLE_ROW_SPLIT_3X3_TO_4,
    ] {
        let stats = telemetry
            .move_family_telemetry
            .get(family)
            .cloned()
            .unwrap_or_default();
        println!(
            "{family}: candidates={} after_pruning={} discovered={} exact_meets={}",
            stats.candidates_generated,
            stats.candidates_after_pruning,
            stats.discovered_nodes,
            stats.exact_meets,
        );
    }

    for family in [
        BINARY_SPARSE_3X3_TO_4,
        BINARY_SPARSE_4X3_TO_3,
        SINGLE_ROW_SPLIT_3X3_TO_4,
    ] {
        println!();
        println!("Top sources for {family}");
        let mut rows = observer
            .family_counts
            .get(family)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .collect::<Vec<_>>();
        rows.sort_by(|(left_matrix, left_stats), (right_matrix, right_stats)| {
            right_stats
                .total_edges
                .cmp(&left_stats.total_edges)
                .then_with(|| {
                    right_stats
                        .discovered_edges
                        .cmp(&left_stats.discovered_edges)
                })
                .then_with(|| left_matrix.data.cmp(&right_matrix.data))
        });

        if rows.is_empty() {
            println!("(no hits on the fixed mixed control)");
        }

        for (idx, (matrix, stats)) in rows.into_iter().take(8).enumerate() {
            match family {
                BINARY_SPARSE_3X3_TO_4 => {
                    let raw = measure_binary_sparse_3x3_to_4(&matrix, request.config.max_entry);
                    println!(
                        "{}. total={} discovered={} seen={} meets={} raw={} orbit={} exact={} canon={} matrix={}",
                        idx + 1,
                        stats.total_edges,
                        stats.discovered_edges,
                        stats.seen_collisions,
                        stats.exact_meets,
                        raw.callbacks,
                        raw.orbit_callbacks,
                        raw.exact_successors,
                        raw.canonical_successors,
                        format_matrix(&matrix),
                    );
                }
                BINARY_SPARSE_4X3_TO_3 => {
                    let raw = measure_binary_sparse_4x4_to_3(&matrix, request.config.max_entry);
                    println!(
                        "{}. total={} discovered={} seen={} meets={} raw={} orbit={} exact={} canon={} matrix={}",
                        idx + 1,
                        stats.total_edges,
                        stats.discovered_edges,
                        stats.seen_collisions,
                        stats.exact_meets,
                        raw.callbacks,
                        raw.orbit_callbacks,
                        raw.exact_successors,
                        raw.canonical_successors,
                        format_matrix(&matrix),
                    );
                }
                SINGLE_ROW_SPLIT_3X3_TO_4 => {
                    let raw = measure_single_row_split_3x3_to_4(&matrix, request.config.max_entry);
                    println!(
                        "{}. total={} discovered={} seen={} meets={} kept={} raw_unquotiented={} twin_orbit={} exact={} canon={} matrix={}",
                        idx + 1,
                        stats.total_edges,
                        stats.discovered_edges,
                        stats.seen_collisions,
                        stats.exact_meets,
                        raw.kept_callbacks,
                        raw.raw_unquotiented_callbacks,
                        raw.twin_orbits,
                        raw.exact_successors,
                        raw.canonical_successors,
                        format_matrix(&matrix),
                    );
                }
                _ => unreachable!(),
            }
        }
    }

    println!();
    println!("Direct samples");
    let binary_sparse_up_sample = DynMatrix::new(3, 3, vec![1, 2, 2, 2, 1, 1, 1, 0, 0]);
    let binary_sparse_up_summary =
        measure_binary_sparse_3x3_to_4(&binary_sparse_up_sample, request.config.max_entry);
    println!(
        "binary_sparse_up sample: raw={} orbit={} exact={} canon={} matrix={}",
        binary_sparse_up_summary.callbacks,
        binary_sparse_up_summary.orbit_callbacks,
        binary_sparse_up_summary.exact_successors,
        binary_sparse_up_summary.canonical_successors,
        format_matrix(&binary_sparse_up_sample),
    );
    let binary_sparse_down_sample =
        DynMatrix::new(4, 4, vec![1, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0, 0, 1, 1, 1]);
    let binary_sparse_down_summary =
        measure_binary_sparse_4x4_to_3(&binary_sparse_down_sample, request.config.max_entry);
    println!(
        "binary_sparse_down sample: raw={} orbit={} exact={} canon={} matrix={}",
        binary_sparse_down_summary.callbacks,
        binary_sparse_down_summary.orbit_callbacks,
        binary_sparse_down_summary.exact_successors,
        binary_sparse_down_summary.canonical_successors,
        format_matrix(&binary_sparse_down_sample),
    );
    let row_split_sample = DynMatrix::new(3, 3, vec![2, 1, 1, 1, 0, 2, 0, 1, 1]);
    let row_split_summary = measure_single_row_split_3x3_to_4(&row_split_sample, 3);
    println!(
        "row_split sample: kept={} raw_unquotiented={} twin_orbit={} exact={} canon={} matrix={}",
        row_split_summary.kept_callbacks,
        row_split_summary.raw_unquotiented_callbacks,
        row_split_summary.twin_orbits,
        row_split_summary.exact_successors,
        row_split_summary.canonical_successors,
        format_matrix(&row_split_sample),
    );

    Ok(())
}

fn format_matrix(matrix: &DynMatrix) -> String {
    let rows = (0..matrix.rows)
        .map(|row| {
            let values = (0..matrix.cols)
                .map(|col| matrix.get(row, col).to_string())
                .collect::<Vec<_>>()
                .join(",");
            format!("[{values}]")
        })
        .collect::<Vec<_>>()
        .join(" ");
    format!("{rows}")
}

fn measure_binary_sparse_3x3_to_4(matrix: &DynMatrix, max_entry: u32) -> BinarySparseSummary {
    let mut callbacks = 0usize;
    let mut orbit_callbacks = BTreeSet::new();
    let mut exact_successors = BTreeSet::new();
    let mut canonical_successors = BTreeSet::new();

    visit_all_factorisations_with_family(matrix, 4, max_entry, |family, u, v| {
        if family != BINARY_SPARSE_3X3_TO_4 {
            return;
        }
        callbacks += 1;
        if let Some(key) = binary_sparse_factorisation_3x3_to_4_orbit_key(&u, &v, max_entry) {
            orbit_callbacks.insert(key);
        }
        let next = v.mul(&u);
        exact_successors.insert(next.data.clone());
        canonical_successors.insert(next.canonical_perm().data);
    });

    BinarySparseSummary {
        callbacks,
        orbit_callbacks: orbit_callbacks.len(),
        exact_successors: exact_successors.len(),
        canonical_successors: canonical_successors.len(),
    }
}

fn measure_binary_sparse_4x4_to_3(matrix: &DynMatrix, max_entry: u32) -> BinarySparseSummary {
    let mut callbacks = 0usize;
    let mut orbit_callbacks = BTreeSet::new();
    let mut exact_successors = BTreeSet::new();
    let mut canonical_successors = BTreeSet::new();

    visit_all_factorisations_with_family(matrix, 4, max_entry, |family, u, v| {
        if family != BINARY_SPARSE_4X3_TO_3 {
            return;
        }
        callbacks += 1;
        if let Some(key) = binary_sparse_factorisation_4x4_to_3_permutation_orbit_key(&u, &v) {
            orbit_callbacks.insert(key);
        }
        let next = v.mul(&u);
        exact_successors.insert(next.data.clone());
        canonical_successors.insert(next.canonical_perm().data);
    });

    BinarySparseSummary {
        callbacks,
        orbit_callbacks: orbit_callbacks.len(),
        exact_successors: exact_successors.len(),
        canonical_successors: canonical_successors.len(),
    }
}

fn binary_sparse_factorisation_3x3_to_4_orbit_key(
    u: &DynMatrix,
    v: &DynMatrix,
    max_entry: u32,
) -> Option<[u32; 24]> {
    if u.rows != 3 || u.cols != 4 || v.rows != 4 || v.cols != 3 {
        return None;
    }

    let perms = permutations4();
    let mut best = None;
    for perm in perms {
        let candidate = permuted_pair_data_3x4_4x3(u, v, &perm);
        let (permuted_u, permuted_v) = permuted_pair_3x4_4x3(u, v, &perm);
        if !is_binary_sparse_factorisation_3x3_to_4_witness(&permuted_u, &permuted_v, max_entry) {
            continue;
        }
        if best.map_or(true, |best_candidate| candidate < best_candidate) {
            best = Some(candidate);
        }
    }
    best
}

fn permuted_pair_3x4_4x3(
    u: &DynMatrix,
    v: &DynMatrix,
    perm: &[usize; 4],
) -> (DynMatrix, DynMatrix) {
    let mut u_data = Vec::with_capacity(12);
    for row in 0..3 {
        for &slot in perm {
            u_data.push(u.get(row, slot));
        }
    }

    let mut v_data = Vec::with_capacity(12);
    for &slot in perm {
        for col in 0..3 {
            v_data.push(v.get(slot, col));
        }
    }

    (DynMatrix::new(3, 4, u_data), DynMatrix::new(4, 3, v_data))
}

fn permuted_pair_data_3x4_4x3(u: &DynMatrix, v: &DynMatrix, perm: &[usize; 4]) -> [u32; 24] {
    let mut data = [0u32; 24];

    for row in 0..3 {
        let base = row * 4;
        for (offset, &slot) in perm.iter().enumerate() {
            data[base + offset] = u.get(row, slot);
        }
    }

    for (row, &slot) in perm.iter().enumerate() {
        let base = 12 + row * 3;
        for col in 0..3 {
            data[base + col] = v.get(slot, col);
        }
    }

    data
}

fn is_binary_sparse_factorisation_3x3_to_4_witness(
    u: &DynMatrix,
    v: &DynMatrix,
    max_entry: u32,
) -> bool {
    if u.rows != 3 || u.cols != 4 || v.rows != 4 || v.cols != 3 {
        return false;
    }

    let cols = [
        [u.get(0, 0), u.get(1, 0), u.get(2, 0)],
        [u.get(0, 1), u.get(1, 1), u.get(2, 1)],
        [u.get(0, 2), u.get(1, 2), u.get(2, 2)],
        [u.get(0, 3), u.get(1, 3), u.get(2, 3)],
    ];
    let rows = [
        [v.get(0, 0), v.get(0, 1), v.get(0, 2)],
        [v.get(1, 0), v.get(1, 1), v.get(1, 2)],
        [v.get(2, 0), v.get(2, 1), v.get(2, 2)],
        [v.get(3, 0), v.get(3, 1), v.get(3, 2)],
    ];
    for distinguished_slot in 0..4 {
        if !is_weighted_sparse_row_len3(cols[distinguished_slot], max_entry)
            || !is_binary_sparse_row_len3(rows[distinguished_slot])
        {
            continue;
        }

        let mut weighted_core_rows = 0usize;
        let mut ok = true;
        for slot in 0..4 {
            if slot == distinguished_slot {
                continue;
            }
            if !is_binary_sparse_row_len3(cols[slot])
                || !is_weighted_sparse_row_len3(rows[slot], max_entry)
            {
                ok = false;
                break;
            }
            if !is_binary_sparse_row_len3(rows[slot]) {
                weighted_core_rows += 1;
            }
        }
        if ok && weighted_core_rows <= 1 {
            return true;
        }
    }

    false
}

fn permutations4() -> Vec<[usize; 4]> {
    let mut perms = Vec::with_capacity(24);
    for a in 0..4 {
        for b in 0..4 {
            if b == a {
                continue;
            }
            for c in 0..4 {
                if c == a || c == b {
                    continue;
                }
                for d in 0..4 {
                    if d == a || d == b || d == c {
                        continue;
                    }
                    perms.push([a, b, c, d]);
                }
            }
        }
    }
    perms
}

fn is_binary_sparse_row_len3(row: [u32; 3]) -> bool {
    matches!(
        row,
        [1, 0, 0] | [0, 1, 0] | [0, 0, 1] | [1, 1, 0] | [1, 0, 1] | [0, 1, 1]
    )
}

fn is_weighted_sparse_row_len3(row: [u32; 3], max_entry: u32) -> bool {
    let mut first = None;
    let mut second = None;
    for value in row {
        if value == 0 {
            continue;
        }
        if value > max_entry {
            return false;
        }
        if first.is_none() {
            first = Some(value);
        } else if second.is_none() {
            second = Some(value);
        } else {
            return false;
        }
    }
    let Some(first) = first else {
        return false;
    };
    let Some(second) = second else {
        return true;
    };
    first == 1 || second == 1 || first == second
}

fn measure_single_row_split_3x3_to_4(matrix: &DynMatrix, max_entry: u32) -> RowSplitSummary {
    let mut kept_callbacks = 0usize;
    let mut exact_successors = BTreeSet::new();
    let mut canonical_successors = BTreeSet::new();

    visit_all_factorisations_with_family(matrix, 4, max_entry, |family, u, v| {
        if family != SINGLE_ROW_SPLIT_3X3_TO_4 {
            return;
        }
        kept_callbacks += 1;
        let next = v.mul(&u);
        exact_successors.insert(next.data.clone());
        canonical_successors.insert(next.canonical_perm().data);
    });

    let (raw_unquotiented_callbacks, twin_orbits) =
        enumerate_single_row_split_3x3_raw_orbits(matrix, max_entry);

    RowSplitSummary {
        kept_callbacks,
        raw_unquotiented_callbacks,
        twin_orbits,
        exact_successors: exact_successors.len(),
        canonical_successors: canonical_successors.len(),
    }
}

fn enumerate_single_row_split_3x3_raw_orbits(matrix: &DynMatrix, max_entry: u32) -> (usize, usize) {
    let rows = [
        [matrix.get(0, 0), matrix.get(0, 1), matrix.get(0, 2)],
        [matrix.get(1, 0), matrix.get(1, 1), matrix.get(1, 2)],
        [matrix.get(2, 0), matrix.get(2, 1), matrix.get(2, 2)],
    ];

    let mut callbacks = 0usize;
    let mut orbit_keys = BTreeSet::new();

    for split_row in 0..3 {
        let original = rows[split_row];
        let lower0 = original[0].saturating_sub(max_entry);
        let upper0 = original[0].min(max_entry);
        let lower1 = original[1].saturating_sub(max_entry);
        let upper1 = original[1].min(max_entry);
        let lower2 = original[2].saturating_sub(max_entry);
        let upper2 = original[2].min(max_entry);

        for split0 in lower0..=upper0 {
            for split1 in lower1..=upper1 {
                for split2 in lower2..=upper2 {
                    let split = [split0, split1, split2];
                    let twin = [
                        original[0] - split0,
                        original[1] - split1,
                        original[2] - split2,
                    ];
                    if split == [0, 0, 0] || twin == [0, 0, 0] {
                        continue;
                    }
                    callbacks += 1;
                    orbit_keys.insert(single_row_split_pair_key(split_row, &rows, &split, &twin));
                }
            }
        }
    }

    (callbacks, orbit_keys.len())
}

fn single_row_split_pair_key(
    split_row: usize,
    rows: &[[u32; 3]; 3],
    split: &[u32; 3],
    twin: &[u32; 3],
) -> [u32; 24] {
    let original = build_single_row_split_pair_data(split_row, rows, split, twin);
    let swapped = build_single_row_split_pair_data(split_row, rows, twin, split);
    original.min(swapped)
}

fn build_single_row_split_pair_data(
    split_row: usize,
    rows: &[[u32; 3]; 3],
    split: &[u32; 3],
    twin: &[u32; 3],
) -> [u32; 24] {
    let mut data = [0u32; 24];
    let clone_cols = match split_row {
        0 => [0usize, 1, 2, 3],
        1 => [0usize, 1, 2, 3],
        2 => [0usize, 1, 2, 3],
        _ => unreachable!(),
    };

    let u = match split_row {
        0 => [[1, 1, 0, 0], [0, 0, 1, 0], [0, 0, 0, 1]],
        1 => [[1, 0, 0, 0], [0, 1, 1, 0], [0, 0, 0, 1]],
        2 => [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 1]],
        _ => unreachable!(),
    };

    for row in 0..3 {
        for col in 0..4 {
            data[row * 4 + col] = u[row][clone_cols[col]];
        }
    }

    let mut v_rows = Vec::with_capacity(4);
    for row in 0..split_row {
        v_rows.push(rows[row]);
    }
    v_rows.push([split[0], split[1], split[2]]);
    v_rows.push([twin[0], twin[1], twin[2]]);
    for row in (split_row + 1)..3 {
        v_rows.push(rows[row]);
    }

    for (row, values) in v_rows.iter().enumerate() {
        let base = 12 + row * 3;
        data[base] = values[0];
        data[base + 1] = values[1];
        data[base + 2] = values[2];
    }

    data
}
