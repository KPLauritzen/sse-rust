use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use sse_core::endpoint_local_parity::{
    endpoint_local_parity_action, mass_support_signature, trimmed_active_window_signature,
};
use sse_core::matrix::DynMatrix;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    if cli.help {
        println!("{}", usage());
        return Ok(());
    }

    let samples = selected_samples();
    let sample_reports = samples.iter().map(build_sample_report).collect::<Vec<_>>();
    let pairs = selected_pairs()
        .iter()
        .map(|pair| build_pair_report(pair, &sample_reports))
        .collect::<Result<Vec<_>, _>>()?;
    let report = SupportIncidenceReport {
        model: ModelDescription {
            name: "active_block_support_incidence".to_string(),
            description: "Delete all-zero rows and columns, forget entry weights, and treat the active support as a bipartite row/column incidence graph. Descriptors summarize row-support and column-support nerves, Hall deficits, maximum matching deficiency, connected components, pairwise overlap, and an exact small biclique edge-cover hint.".to_string(),
            max_selected_active_shape: "4x4".to_string(),
            max_biclique_cover_edges: 16,
        },
        samples: sample_reports,
        pairs,
    };

    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize report: {err}"))?;
    if let Some(path) = cli.json_out {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
        }
        fs::write(&path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("wrote {}", path.display());
    } else {
        println!("{json}");
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    json_out: Option<PathBuf>,
    help: bool,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut json_out = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--json-out" => {
                json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--help" | "-h" => {
                return Ok(Cli {
                    json_out: None,
                    help: true,
                });
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    Ok(Cli {
        json_out,
        help: false,
    })
}

fn usage() -> String {
    "usage: diagnose_support_incidence_descriptors [--json-out PATH]".to_string()
}

#[derive(Serialize)]
struct SupportIncidenceReport {
    model: ModelDescription,
    samples: Vec<SampleReport>,
    pairs: Vec<PairReport>,
}

#[derive(Serialize)]
struct ModelDescription {
    name: String,
    description: String,
    max_selected_active_shape: String,
    max_biclique_cover_edges: usize,
}

#[derive(Clone)]
struct Sample {
    id: &'static str,
    label: &'static str,
    source: &'static str,
    matrix: DynMatrix,
}

#[derive(Clone)]
struct Pair {
    id: &'static str,
    label: &'static str,
    pair_kind: &'static str,
    left_id: &'static str,
    right_id: &'static str,
}

#[derive(Clone, Serialize)]
struct SampleReport {
    id: String,
    label: String,
    source: String,
    original_shape: String,
    active_shape: String,
    matrix: DynMatrix,
    active_block: DynMatrix,
    coarse_signature: String,
    trimmed_active_window_signature: String,
    support_incidence_signature: String,
    support_incidence: SupportIncidenceDescriptor,
}

#[derive(Serialize)]
struct PairReport {
    id: String,
    label: String,
    pair_kind: String,
    left_id: String,
    right_id: String,
    same_coarse_signature: bool,
    same_trimmed_active_window_signature: bool,
    endpoint_local_parity_action: String,
    same_support_incidence_signature: bool,
    same_nerve_summary: bool,
    same_hall_deficit_profile: bool,
    same_matching_summary: bool,
    same_component_summary: bool,
    same_overlap_profile: bool,
    same_biclique_cover_hint: bool,
    prior_orbit_profile_decision: String,
    prior_weighted_wl_decision: String,
    diagnostic_reading: String,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct SupportIncidenceDescriptor {
    active_shape: [usize; 2],
    edge_count: usize,
    row_support_sizes: Vec<usize>,
    column_support_sizes: Vec<usize>,
    row_support_nerve: NerveSummary,
    column_support_nerve: NerveSummary,
    row_hall_deficit: HallProfile,
    column_hall_deficit: HallProfile,
    matching: MatchingSummary,
    components: Vec<ComponentSummary>,
    overlap: OverlapSummary,
    biclique_cover: BicliqueCoverSummary,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct NerveSummary {
    family_size: usize,
    universe_size: usize,
    simplex_counts_by_size: Vec<usize>,
    maximal_face_sizes: Vec<usize>,
    one_skeleton_component_sizes: Vec<usize>,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct HallProfile {
    side_size: usize,
    neighbor_size: usize,
    max_deficit: i32,
    positive_deficit_count: usize,
    positive_deficit_counts_by_subset_size: Vec<usize>,
    buckets_by_subset_size: Vec<HallSubsetBucket>,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct HallSubsetBucket {
    subset_size: usize,
    sorted_deficits: Vec<i32>,
    sorted_neighborhood_sizes: Vec<usize>,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct MatchingSummary {
    maximum_matching_size: usize,
    row_deficiency: usize,
    column_deficiency: usize,
    covers_all_rows: bool,
    covers_all_columns: bool,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct ComponentSummary {
    rows: usize,
    columns: usize,
    edges: usize,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct OverlapSummary {
    row_pair_intersection_sizes: Vec<usize>,
    column_pair_intersection_sizes: Vec<usize>,
}

#[derive(Clone, Eq, PartialEq, Serialize)]
struct BicliqueCoverSummary {
    maximal_biclique_count: usize,
    maximal_biclique_edge_size_histogram: Vec<SizeCount>,
    largest_biclique_edges: usize,
    minimum_edge_biclique_cover_size: usize,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd, Serialize)]
struct SizeCount {
    size: usize,
    count: usize,
}

#[derive(Clone)]
struct ActiveBlock {
    matrix: DynMatrix,
    row_masks: Vec<u64>,
    column_masks: Vec<u64>,
}

fn selected_samples() -> Vec<Sample> {
    vec![
        Sample {
            id: "brix_rank4_frontier",
            label: "Brix-Ruiz k=4 retained rank-4 diagonal near-hit frontier",
            source: "research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md rank 4 to_matrix",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        },
        Sample {
            id: "brix_rank4_counterpart",
            label: "Brix-Ruiz k=4 retained rank-4 closest opposite-side counterpart",
            source: "research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md rank 4 counterpart_matrix",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0],
            ),
        },
        Sample {
            id: "brix_rank6_frontier",
            label: "Brix-Ruiz k=4 retained rank-6 diagonal near-hit frontier",
            source: "src/bin/diagnose_brix_ruiz_k4_active_block_switches.rs rank-6 cluster fixture",
            matrix: DynMatrix::new(
                4,
                4,
                vec![0, 2, 3, 0, 0, 2, 1, 0, 0, 11, 0, 0, 0, 2, 2, 0],
            ),
        },
        Sample {
            id: "brix_rank6_counterpart",
            label: "Brix-Ruiz k=4 retained rank-6 closest opposite-side counterpart",
            source: "src/bin/diagnose_brix_ruiz_k4_active_block_switches.rs rank-6 cluster fixture",
            matrix: DynMatrix::new(
                4,
                4,
                vec![0, 2, 1, 0, 0, 1, 4, 0, 0, 3, 1, 0, 0, 11, 0, 0],
            ),
        },
        Sample {
            id: "baker_a4",
            label: "Baker/Lind-Marcus k=3 hard same-size control A4",
            source: "research/guide_artifacts/k3_shortcut_round1.json path.matrices[4]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 2, 2, 0, 1, 1, 1, 1, 0, 1, 0, 1, 0, 2, 1, 0],
            ),
        },
        Sample {
            id: "baker_a5",
            label: "Baker/Lind-Marcus k=3 hard same-size control A5",
            source: "research/guide_artifacts/k3_shortcut_round1.json path.matrices[5]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 1, 1, 1, 3, 0, 2, 2, 1, 0, 0, 0, 0, 1, 1, 1],
            ),
        },
        Sample {
            id: "k3_baker_step2",
            label: "Baker/Lind-Marcus replay overlap calibration step 2",
            source: "research/guide_artifacts/k3_shortcut_round1.json path.matrices[2]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 2, 2, 0, 1, 0, 2, 0, 0, 1, 1, 1, 1, 1, 2, 0],
            ),
        },
        Sample {
            id: "k3_non_baker_step2",
            label: "Non-Baker exact replay overlap calibration step 2",
            source: "research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json path.matrices[2]",
            matrix: DynMatrix::new(
                4,
                4,
                vec![1, 0, 1, 1, 2, 1, 0, 2, 2, 1, 0, 1, 2, 1, 0, 0],
            ),
        },
    ]
}

fn selected_pairs() -> Vec<Pair> {
    vec![
        Pair {
            id: "brix_rank4_frontier_vs_counterpart",
            label: "retained Brix-Ruiz k=4 rank-4 frontier vs closest counterpart",
            pair_kind: "coarse_only_near_miss",
            left_id: "brix_rank4_frontier",
            right_id: "brix_rank4_counterpart",
        },
        Pair {
            id: "brix_rank6_frontier_vs_counterpart",
            label: "retained Brix-Ruiz k=4 rank-6 frontier vs closest counterpart",
            pair_kind: "coarse_only_near_miss",
            left_id: "brix_rank6_frontier",
            right_id: "brix_rank6_counterpart",
        },
        Pair {
            id: "baker_a4_to_a5",
            label: "Baker/Lind-Marcus hard same-size 4x4 A4 -> A5 control",
            pair_kind: "known_local_transfer_not_one_current_family",
            left_id: "baker_a4",
            right_id: "baker_a5",
        },
        Pair {
            id: "k3_replay_overlap_step2",
            label: "known k=3 Baker/non-Baker replay overlap step 2",
            pair_kind: "known_reuse_calibration",
            left_id: "k3_baker_step2",
            right_id: "k3_non_baker_step2",
        },
    ]
}

fn build_sample_report(sample: &Sample) -> SampleReport {
    let active = active_block(&sample.matrix);
    let support_incidence = support_incidence_descriptor(&active);
    let support_incidence_signature = canonical_json(&support_incidence);

    SampleReport {
        id: sample.id.to_string(),
        label: sample.label.to_string(),
        source: sample.source.to_string(),
        original_shape: shape_label(&sample.matrix),
        active_shape: shape_label(&active.matrix),
        matrix: sample.matrix.clone(),
        active_block: active.matrix,
        coarse_signature: mass_support_signature(&sample.matrix),
        trimmed_active_window_signature: trimmed_active_window_signature(&sample.matrix),
        support_incidence_signature,
        support_incidence,
    }
}

fn build_pair_report(pair: &Pair, samples: &[SampleReport]) -> Result<PairReport, String> {
    let left = samples
        .iter()
        .find(|sample| sample.id == pair.left_id)
        .ok_or_else(|| format!("unknown sample id {}", pair.left_id))?;
    let right = samples
        .iter()
        .find(|sample| sample.id == pair.right_id)
        .ok_or_else(|| format!("unknown sample id {}", pair.right_id))?;

    let same_coarse_signature = left.coarse_signature == right.coarse_signature;
    let same_trimmed_active_window_signature =
        left.trimmed_active_window_signature == right.trimmed_active_window_signature;
    let same_support_incidence_signature =
        left.support_incidence_signature == right.support_incidence_signature;
    let same_nerve_summary = left.support_incidence.row_support_nerve
        == right.support_incidence.row_support_nerve
        && left.support_incidence.column_support_nerve
            == right.support_incidence.column_support_nerve;
    let same_hall_deficit_profile = left.support_incidence.row_hall_deficit
        == right.support_incidence.row_hall_deficit
        && left.support_incidence.column_hall_deficit
            == right.support_incidence.column_hall_deficit;
    let same_matching_summary = left.support_incidence.matching == right.support_incidence.matching;
    let same_component_summary =
        left.support_incidence.components == right.support_incidence.components;
    let same_overlap_profile = left.support_incidence.overlap == right.support_incidence.overlap;
    let same_biclique_cover_hint =
        left.support_incidence.biclique_cover == right.support_incidence.biclique_cover;
    let action = endpoint_local_parity_action(&left.matrix, &right.matrix);

    Ok(PairReport {
        id: pair.id.to_string(),
        label: pair.label.to_string(),
        pair_kind: pair.pair_kind.to_string(),
        left_id: pair.left_id.to_string(),
        right_id: pair.right_id.to_string(),
        same_coarse_signature,
        same_trimmed_active_window_signature,
        endpoint_local_parity_action: action.as_str().to_string(),
        same_support_incidence_signature,
        same_nerve_summary,
        same_hall_deficit_profile,
        same_matching_summary,
        same_component_summary,
        same_overlap_profile,
        same_biclique_cover_hint,
        prior_orbit_profile_decision: prior_orbit_profile_decision(pair.id).to_string(),
        prior_weighted_wl_decision: prior_weighted_wl_decision(pair.id).to_string(),
        diagnostic_reading: diagnostic_reading(
            pair.pair_kind,
            same_coarse_signature,
            same_trimmed_active_window_signature,
            same_support_incidence_signature,
        ),
    })
}

fn prior_orbit_profile_decision(pair_id: &str) -> &'static str {
    match pair_id {
        "brix_rank4_frontier_vs_counterpart" | "brix_rank6_frontier_vs_counterpart" => {
            "support_transporters_match_weighted_transporters_split"
        }
        "baker_a4_to_a5" => "no_support_or_weighted_transporter",
        "k3_replay_overlap_step2" => "weighted_transporter_match",
        _ => "unknown",
    }
}

fn prior_weighted_wl_decision(pair_id: &str) -> &'static str {
    match pair_id {
        "brix_rank4_frontier_vs_counterpart"
        | "brix_rank6_frontier_vs_counterpart"
        | "baker_a4_to_a5" => "split_from_round_1",
        "k3_replay_overlap_step2" => "match_through_round_3",
        _ => "unknown",
    }
}

fn diagnostic_reading(
    pair_kind: &str,
    same_coarse_signature: bool,
    same_trimmed_active_window_signature: bool,
    same_support_incidence_signature: bool,
) -> String {
    match pair_kind {
        "coarse_only_near_miss"
            if same_coarse_signature
                && !same_trimmed_active_window_signature
                && same_support_incidence_signature =>
        {
            "support-incidence descriptors collapse this coarse-only active-layout mismatch; they add no separation beyond the existing support profile and are weaker than trimmed_active_window/WL on this control".to_string()
        }
        "known_reuse_calibration" if same_support_incidence_signature => {
            "support-incidence descriptors preserve this literal k3 replay-overlap calibration".to_string()
        }
        "known_local_transfer_not_one_current_family" if !same_support_incidence_signature => {
            "support-incidence descriptors split the Baker A4 -> A5 local-transfer control, so equality is not a transfer-reuse signal".to_string()
        }
        _ if same_support_incidence_signature => {
            "support-incidence descriptors match; inspect the coarser controls before assigning reuse meaning".to_string()
        }
        _ => "support-incidence descriptors split this pair".to_string(),
    }
}

fn active_block(matrix: &DynMatrix) -> ActiveBlock {
    let rows = (0..matrix.rows)
        .filter(|&row| (0..matrix.cols).any(|col| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();
    let cols = (0..matrix.cols)
        .filter(|&col| (0..matrix.rows).any(|row| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();
    let mut data = Vec::with_capacity(rows.len() * cols.len());
    for &row in &rows {
        for &col in &cols {
            data.push(matrix.get(row, col));
        }
    }

    let active_matrix = DynMatrix::new(rows.len(), cols.len(), data);
    let row_masks = row_support_masks(&active_matrix);
    let column_masks = column_support_masks(&active_matrix);

    ActiveBlock {
        matrix: active_matrix,
        row_masks,
        column_masks,
    }
}

fn support_incidence_descriptor(active: &ActiveBlock) -> SupportIncidenceDescriptor {
    let mut row_support_sizes = active
        .row_masks
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .collect::<Vec<_>>();
    let mut column_support_sizes = active
        .column_masks
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .collect::<Vec<_>>();
    row_support_sizes.sort_unstable();
    column_support_sizes.sort_unstable();

    SupportIncidenceDescriptor {
        active_shape: [active.matrix.rows, active.matrix.cols],
        edge_count: active
            .matrix
            .data
            .iter()
            .filter(|&&value| value != 0)
            .count(),
        row_support_sizes,
        column_support_sizes,
        row_support_nerve: nerve_summary(&active.row_masks, active.matrix.cols),
        column_support_nerve: nerve_summary(&active.column_masks, active.matrix.rows),
        row_hall_deficit: hall_profile(
            active.row_masks.len(),
            active.matrix.cols,
            &active.row_masks,
        ),
        column_hall_deficit: hall_profile(
            active.column_masks.len(),
            active.matrix.rows,
            &active.column_masks,
        ),
        matching: matching_summary(&active.row_masks, active.matrix.cols),
        components: component_summary(active),
        overlap: overlap_summary(&active.row_masks, &active.column_masks),
        biclique_cover: biclique_cover_summary(active),
    }
}

fn row_support_masks(matrix: &DynMatrix) -> Vec<u64> {
    assert!(matrix.cols <= 64);
    (0..matrix.rows)
        .map(|row| {
            let mut mask = 0u64;
            for col in 0..matrix.cols {
                if matrix.get(row, col) != 0 {
                    mask |= 1u64 << col;
                }
            }
            mask
        })
        .collect()
}

fn column_support_masks(matrix: &DynMatrix) -> Vec<u64> {
    assert!(matrix.rows <= 64);
    (0..matrix.cols)
        .map(|col| {
            let mut mask = 0u64;
            for row in 0..matrix.rows {
                if matrix.get(row, col) != 0 {
                    mask |= 1u64 << row;
                }
            }
            mask
        })
        .collect()
}

fn nerve_summary(family_masks: &[u64], universe_size: usize) -> NerveSummary {
    let family_size = family_masks.len();
    let mut simplex_counts_by_size = vec![0usize; family_size];
    let mut valid_subsets = Vec::new();

    for subset in nonempty_subsets(family_size) {
        let intersection = subset_intersection(subset, family_masks, universe_size);
        if intersection != 0 {
            let size = subset.count_ones() as usize;
            simplex_counts_by_size[size - 1] += 1;
            valid_subsets.push((subset, size));
        }
    }

    let mut maximal_face_sizes = valid_subsets
        .iter()
        .filter(|&&(subset, _)| {
            !valid_subsets
                .iter()
                .any(|&(other, _)| other != subset && subset & !other == 0)
        })
        .map(|&(_, size)| size)
        .collect::<Vec<_>>();
    maximal_face_sizes.sort_unstable();

    let mut uf = UnionFind::new(family_size);
    for left in 0..family_size {
        for right in (left + 1)..family_size {
            if family_masks[left] & family_masks[right] != 0 {
                uf.union(left, right);
            }
        }
    }
    let mut one_skeleton_component_sizes = uf
        .partitions()
        .into_iter()
        .map(|component| component.len())
        .collect::<Vec<_>>();
    one_skeleton_component_sizes.sort_unstable();

    NerveSummary {
        family_size,
        universe_size,
        simplex_counts_by_size,
        maximal_face_sizes,
        one_skeleton_component_sizes,
    }
}

fn subset_intersection(subset: u64, family_masks: &[u64], universe_size: usize) -> u64 {
    let mut intersection = universe_mask(universe_size);
    for (idx, mask) in family_masks.iter().enumerate() {
        if subset & (1u64 << idx) != 0 {
            intersection &= *mask;
        }
    }
    intersection
}

fn hall_profile(side_size: usize, neighbor_size: usize, side_masks: &[u64]) -> HallProfile {
    let mut buckets = (0..side_size)
        .map(|idx| HallSubsetBucket {
            subset_size: idx + 1,
            sorted_deficits: Vec::new(),
            sorted_neighborhood_sizes: Vec::new(),
        })
        .collect::<Vec<_>>();
    let mut positive_deficit_counts_by_subset_size = vec![0usize; side_size];
    let mut max_deficit = i32::MIN;
    let mut positive_deficit_count = 0usize;

    for subset in nonempty_subsets(side_size) {
        let subset_size = subset.count_ones() as usize;
        let mut neighbors = 0u64;
        for (idx, mask) in side_masks.iter().enumerate() {
            if subset & (1u64 << idx) != 0 {
                neighbors |= *mask;
            }
        }
        let neighborhood_size = neighbors.count_ones() as usize;
        debug_assert!(neighborhood_size <= neighbor_size);
        let deficit = subset_size as i32 - neighborhood_size as i32;
        max_deficit = max_deficit.max(deficit);
        if deficit > 0 {
            positive_deficit_count += 1;
            positive_deficit_counts_by_subset_size[subset_size - 1] += 1;
        }
        buckets[subset_size - 1].sorted_deficits.push(deficit);
        buckets[subset_size - 1]
            .sorted_neighborhood_sizes
            .push(neighborhood_size);
    }

    for bucket in &mut buckets {
        bucket.sorted_deficits.sort_unstable();
        bucket.sorted_neighborhood_sizes.sort_unstable();
    }

    HallProfile {
        side_size,
        neighbor_size,
        max_deficit: if max_deficit == i32::MIN {
            0
        } else {
            max_deficit
        },
        positive_deficit_count,
        positive_deficit_counts_by_subset_size,
        buckets_by_subset_size: buckets,
    }
}

fn matching_summary(row_masks: &[u64], column_count: usize) -> MatchingSummary {
    let maximum_matching_size = maximum_bipartite_matching(row_masks, column_count);
    MatchingSummary {
        maximum_matching_size,
        row_deficiency: row_masks.len().saturating_sub(maximum_matching_size),
        column_deficiency: column_count.saturating_sub(maximum_matching_size),
        covers_all_rows: maximum_matching_size == row_masks.len(),
        covers_all_columns: maximum_matching_size == column_count,
    }
}

fn maximum_bipartite_matching(row_masks: &[u64], column_count: usize) -> usize {
    let mut column_match = vec![None; column_count];
    let mut matched = 0usize;
    for row in 0..row_masks.len() {
        let mut seen = vec![false; column_count];
        if augment_matching(row, row_masks, &mut seen, &mut column_match) {
            matched += 1;
        }
    }
    matched
}

fn augment_matching(
    row: usize,
    row_masks: &[u64],
    seen: &mut [bool],
    column_match: &mut [Option<usize>],
) -> bool {
    for col in 0..seen.len() {
        if row_masks[row] & (1u64 << col) == 0 || seen[col] {
            continue;
        }
        seen[col] = true;
        if column_match[col]
            .map(|matched_row| augment_matching(matched_row, row_masks, seen, column_match))
            .unwrap_or(true)
        {
            column_match[col] = Some(row);
            return true;
        }
    }
    false
}

fn component_summary(active: &ActiveBlock) -> Vec<ComponentSummary> {
    let rows = active.matrix.rows;
    let cols = active.matrix.cols;
    let mut uf = UnionFind::new(rows + cols);
    for row in 0..rows {
        for col in 0..cols {
            if active.matrix.get(row, col) != 0 {
                uf.union(row, rows + col);
            }
        }
    }

    let mut by_root = BTreeMap::<usize, ComponentSummary>::new();
    for row in 0..rows {
        let root = uf.find(row);
        by_root
            .entry(root)
            .or_insert(ComponentSummary {
                rows: 0,
                columns: 0,
                edges: 0,
            })
            .rows += 1;
    }
    for col in 0..cols {
        let root = uf.find(rows + col);
        by_root
            .entry(root)
            .or_insert(ComponentSummary {
                rows: 0,
                columns: 0,
                edges: 0,
            })
            .columns += 1;
    }
    for row in 0..rows {
        for col in 0..cols {
            if active.matrix.get(row, col) != 0 {
                let root = uf.find(row);
                by_root
                    .entry(root)
                    .or_insert(ComponentSummary {
                        rows: 0,
                        columns: 0,
                        edges: 0,
                    })
                    .edges += 1;
            }
        }
    }

    let mut components = by_root.into_values().collect::<Vec<_>>();
    components.sort_unstable();
    components
}

fn overlap_summary(row_masks: &[u64], column_masks: &[u64]) -> OverlapSummary {
    OverlapSummary {
        row_pair_intersection_sizes: pairwise_intersection_sizes(row_masks),
        column_pair_intersection_sizes: pairwise_intersection_sizes(column_masks),
    }
}

fn pairwise_intersection_sizes(masks: &[u64]) -> Vec<usize> {
    let mut sizes = Vec::new();
    for left in 0..masks.len() {
        for right in (left + 1)..masks.len() {
            sizes.push((masks[left] & masks[right]).count_ones() as usize);
        }
    }
    sizes.sort_unstable();
    sizes
}

fn biclique_cover_summary(active: &ActiveBlock) -> BicliqueCoverSummary {
    let full_edge_mask = support_edge_mask(active);
    let maximal_bicliques = maximal_biclique_edge_masks(active);
    let largest_biclique_edges = maximal_bicliques
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .max()
        .unwrap_or(0);
    let minimum_edge_biclique_cover_size =
        minimum_biclique_cover_size(full_edge_mask, &maximal_bicliques);

    BicliqueCoverSummary {
        maximal_biclique_count: maximal_bicliques.len(),
        maximal_biclique_edge_size_histogram: size_histogram(
            maximal_bicliques
                .iter()
                .map(|mask| mask.count_ones() as usize),
        ),
        largest_biclique_edges,
        minimum_edge_biclique_cover_size,
    }
}

fn maximal_biclique_edge_masks(active: &ActiveBlock) -> Vec<u64> {
    let rows = active.matrix.rows;
    let cols = active.matrix.cols;
    assert!(rows * cols <= 64);

    let mut masks = BTreeSet::<u64>::new();
    for row_subset in nonempty_subsets(rows) {
        for col_subset in nonempty_subsets(cols) {
            if is_complete_biclique(row_subset, col_subset, &active.row_masks) {
                masks.insert(biclique_edge_mask(row_subset, col_subset, cols));
            }
        }
    }

    let masks = masks.into_iter().collect::<Vec<_>>();
    let mut maximal = masks
        .iter()
        .copied()
        .filter(|&mask| {
            !masks
                .iter()
                .any(|&other| other != mask && mask & !other == 0)
        })
        .collect::<Vec<_>>();
    maximal.sort_by(|left, right| {
        right
            .count_ones()
            .cmp(&left.count_ones())
            .then_with(|| left.cmp(right))
    });
    maximal
}

fn is_complete_biclique(row_subset: u64, col_subset: u64, row_masks: &[u64]) -> bool {
    for row in 0..row_masks.len() {
        if row_subset & (1u64 << row) != 0 && row_masks[row] & col_subset != col_subset {
            return false;
        }
    }
    true
}

fn biclique_edge_mask(row_subset: u64, col_subset: u64, column_count: usize) -> u64 {
    let mut mask = 0u64;
    for row in 0..64 {
        if row_subset & (1u64 << row) == 0 {
            continue;
        }
        for col in 0..column_count {
            if col_subset & (1u64 << col) != 0 {
                let edge_idx = row * column_count + col;
                assert!(edge_idx < 64);
                mask |= 1u64 << edge_idx;
            }
        }
    }
    mask
}

fn support_edge_mask(active: &ActiveBlock) -> u64 {
    let rows = active.matrix.rows;
    let cols = active.matrix.cols;
    assert!(rows * cols <= 64);
    let mut mask = 0u64;
    for row in 0..rows {
        for col in 0..cols {
            if active.matrix.get(row, col) != 0 {
                mask |= 1u64 << (row * cols + col);
            }
        }
    }
    mask
}

fn minimum_biclique_cover_size(full_edge_mask: u64, bicliques: &[u64]) -> usize {
    if full_edge_mask == 0 {
        return 0;
    }
    let largest = bicliques
        .iter()
        .map(|mask| mask.count_ones() as usize)
        .max()
        .unwrap_or(1);
    let mut best = full_edge_mask.count_ones() as usize;
    search_biclique_cover(full_edge_mask, 0, &mut best, largest, bicliques);
    best
}

fn search_biclique_cover(
    remaining: u64,
    used: usize,
    best: &mut usize,
    largest_biclique_edges: usize,
    bicliques: &[u64],
) {
    if remaining == 0 {
        *best = (*best).min(used);
        return;
    }
    if used >= *best {
        return;
    }

    let remaining_edges = remaining.count_ones() as usize;
    let lower_bound = remaining_edges.div_ceil(largest_biclique_edges.max(1));
    if used + lower_bound >= *best {
        return;
    }

    let uncovered_bit = 1u64 << remaining.trailing_zeros();
    for &biclique in bicliques {
        if biclique & uncovered_bit == 0 {
            continue;
        }
        let next_remaining = remaining & !biclique;
        if next_remaining != remaining {
            search_biclique_cover(
                next_remaining,
                used + 1,
                best,
                largest_biclique_edges,
                bicliques,
            );
        }
    }
}

fn size_histogram<I>(sizes: I) -> Vec<SizeCount>
where
    I: IntoIterator<Item = usize>,
{
    let mut counts = BTreeMap::<usize, usize>::new();
    for size in sizes {
        *counts.entry(size).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(size, count)| SizeCount { size, count })
        .collect()
}

fn nonempty_subsets(size: usize) -> std::ops::Range<u64> {
    assert!(size < 64);
    1..(1u64 << size)
}

fn universe_mask(size: usize) -> u64 {
    assert!(size <= 64);
    if size == 64 {
        u64::MAX
    } else {
        (1u64 << size) - 1
    }
}

fn canonical_json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("support-incidence descriptor should serialize")
}

fn shape_label(matrix: &DynMatrix) -> String {
    format!("{}x{}", matrix.rows, matrix.cols)
}

struct UnionFind {
    parent: Vec<usize>,
}

impl UnionFind {
    fn new(n: usize) -> Self {
        Self {
            parent: (0..n).collect(),
        }
    }

    fn find(&mut self, value: usize) -> usize {
        if self.parent[value] != value {
            let root = self.find(self.parent[value]);
            self.parent[value] = root;
        }
        self.parent[value]
    }

    fn union(&mut self, left: usize, right: usize) {
        let left_root = self.find(left);
        let right_root = self.find(right);
        if left_root != right_root {
            self.parent[right_root] = left_root;
        }
    }

    fn partitions(mut self) -> Vec<Vec<usize>> {
        let mut by_root = BTreeMap::<usize, Vec<usize>>::new();
        for idx in 0..self.parent.len() {
            by_root.entry(self.find(idx)).or_default().push(idx);
        }
        by_root.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn support_incidence_collapses_brix_coarse_only_pairs() {
        let samples = sample_reports();
        for (left_id, right_id) in [
            ("brix_rank4_frontier", "brix_rank4_counterpart"),
            ("brix_rank6_frontier", "brix_rank6_counterpart"),
        ] {
            let left = &samples[left_id];
            let right = &samples[right_id];

            assert_eq!(left.coarse_signature, right.coarse_signature);
            assert_ne!(
                left.trimmed_active_window_signature,
                right.trimmed_active_window_signature
            );
            assert_eq!(
                left.support_incidence_signature,
                right.support_incidence_signature
            );
        }
    }

    #[test]
    fn support_incidence_preserves_k3_replay_overlap() {
        let samples = sample_reports();
        let left = &samples["k3_baker_step2"];
        let right = &samples["k3_non_baker_step2"];

        assert_eq!(left.coarse_signature, right.coarse_signature);
        assert_eq!(
            left.trimmed_active_window_signature,
            right.trimmed_active_window_signature
        );
        assert_eq!(
            left.support_incidence_signature,
            right.support_incidence_signature
        );
    }

    #[test]
    fn support_incidence_splits_baker_same_size_control() {
        let samples = sample_reports();
        let left = &samples["baker_a4"];
        let right = &samples["baker_a5"];

        assert_ne!(
            left.support_incidence_signature,
            right.support_incidence_signature
        );
    }

    #[test]
    fn hall_and_matching_record_side_deficiencies() {
        let active = active_block(&DynMatrix::new(2, 3, vec![1, 1, 1, 1, 1, 1]));
        let descriptor = support_incidence_descriptor(&active);

        assert_eq!(descriptor.matching.maximum_matching_size, 2);
        assert_eq!(descriptor.matching.row_deficiency, 0);
        assert_eq!(descriptor.matching.column_deficiency, 1);
        assert_eq!(descriptor.row_hall_deficit.max_deficit, -1);
        assert_eq!(descriptor.column_hall_deficit.max_deficit, 1);
        assert_eq!(
            descriptor
                .column_hall_deficit
                .positive_deficit_counts_by_subset_size,
            vec![0, 0, 1]
        );
    }

    #[test]
    fn nerve_summary_detects_shared_intersections() {
        let active = active_block(&DynMatrix::new(3, 3, vec![1, 1, 0, 1, 0, 0, 0, 0, 1]));
        let descriptor = support_incidence_descriptor(&active);

        assert_eq!(
            descriptor.row_support_nerve.simplex_counts_by_size,
            vec![3, 1, 0]
        );
        assert_eq!(
            descriptor.row_support_nerve.one_skeleton_component_sizes,
            vec![1, 2]
        );
    }

    #[test]
    fn biclique_cover_counts_near_complete_support() {
        let active = active_block(&DynMatrix::new(2, 4, vec![1, 1, 1, 1, 1, 1, 0, 1]));
        let cover = biclique_cover_summary(&active);

        assert_eq!(cover.largest_biclique_edges, 6);
        assert_eq!(cover.minimum_edge_biclique_cover_size, 2);
    }

    fn sample_reports() -> BTreeMap<&'static str, SampleReport> {
        selected_samples()
            .into_iter()
            .map(|sample| {
                let id = sample.id;
                (id, build_sample_report(&sample))
            })
            .collect()
    }
}
