use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use sse_core::endpoint_local_parity::{
    endpoint_local_parity_action, mass_support_signature, trimmed_active_window_signature,
};
use sse_core::matrix::DynMatrix;

const FULL_DIRECTED_TRACE_MAX_POWER: usize = 6;
const FULL_DIRECTED_TOTAL_WALK_MAX_POWER: usize = 4;
const ACTIVE_GRAM_TRACE_MAX_POWER: usize = 4;

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
    let sample_reports = samples
        .iter()
        .map(build_sample_report)
        .collect::<Result<Vec<_>, _>>()?;
    let pairs = selected_pairs()
        .iter()
        .map(|pair| build_pair_report(pair, &sample_reports))
        .collect::<Result<Vec<_>, _>>()?;
    let report = SpectralWalkReport {
        models: vec![
            ModelDescription {
                name: "full_directed_exact_walk_moments".to_string(),
                description: "For each square weighted matrix M, report exact closed directed walk counts trace(M^p) for p=1..6 and total weighted directed walks 1^T M^p 1 for p=1..4. The p=1..4 traces are the same power-trace invariant surface already used by the solver for square endpoints up to dimension 4; p>4 is determined by the 4x4 characteristic polynomial on these controls.".to_string(),
            },
            ModelDescription {
                name: "full_directed_charpoly_and_bowen_franks".to_string(),
                description: "Report the exact characteristic polynomial of the full square weighted adjacency matrix and the Smith-normal-form invariants of I-M. The characteristic polynomial is a spectral surrogate, not a floating equality claim; for 4x4 it overlaps the existing trace(M^1..M^4) data by Newton identities.".to_string(),
            },
            ModelDescription {
                name: "active_weighted_gram_spectrum".to_string(),
                description: "Delete all-zero rows and columns, form B B^T for the active block B, and report exact trace((B B^T)^p), p=1..4, plus the exact characteristic polynomial. This is a singular-value-adjacent descriptor using squared singular values without adding a floating eigensolver dependency.".to_string(),
            },
            ModelDescription {
                name: "active_bipartite_laplacian_spectrum".to_string(),
                description: "Delete all-zero rows and columns, build the undirected bipartite row/column graph, and report exact support and weighted Laplacian characteristic polynomials. The support version intentionally forgets weights; the weighted version uses entry values as edge weights.".to_string(),
            },
        ],
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
    "usage: diagnose_spectral_walk_descriptors [--json-out PATH]".to_string()
}

#[derive(Serialize)]
struct SpectralWalkReport {
    models: Vec<ModelDescription>,
    samples: Vec<SampleReport>,
    pairs: Vec<PairReport>,
}

#[derive(Serialize)]
struct ModelDescription {
    name: String,
    description: String,
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
    endpoint_local_parity_self_supported: bool,
    full_directed_closed_walk_traces_1_to_6: Vec<i64>,
    existing_power_trace_prefix_1_to_4: Vec<i64>,
    full_directed_total_walks_1_to_4: Vec<i64>,
    full_directed_adjacency_charpoly: Vec<i64>,
    bowen_franks_i_minus_m: Vec<i64>,
    active_weighted_gram_traces_1_to_4: Vec<i64>,
    active_weighted_gram_charpoly: Vec<i64>,
    active_support_laplacian_charpoly: Vec<i64>,
    active_weighted_laplacian_charpoly: Vec<i64>,
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
    same_existing_power_trace_prefix_1_to_4: bool,
    same_full_directed_closed_walk_traces_1_to_6: bool,
    same_full_directed_total_walks_1_to_4: bool,
    same_full_directed_adjacency_charpoly: bool,
    same_bowen_franks_i_minus_m: bool,
    same_active_weighted_gram_spectrum: bool,
    same_active_support_laplacian_spectrum: bool,
    same_active_weighted_laplacian_spectrum: bool,
    prior_orbit_profile_decision: String,
    prior_weighted_wl_decision: String,
    prior_support_incidence_decision: String,
    diagnostic_reading: String,
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

fn build_sample_report(sample: &Sample) -> Result<SampleReport, String> {
    let active = active_block(&sample.matrix);
    let full_matrix = dyn_to_i128(&sample.matrix);
    let active_gram = gram_matrix(&active);
    let support_laplacian = bipartite_laplacian(&active, EdgeWeightPolicy::Support);
    let weighted_laplacian = bipartite_laplacian(&active, EdgeWeightPolicy::Weighted);
    let full_directed_closed_walk_traces_1_to_6 = power_traces(
        &full_matrix,
        FULL_DIRECTED_TRACE_MAX_POWER,
        "full directed walk traces",
    )?;

    Ok(SampleReport {
        id: sample.id.to_string(),
        label: sample.label.to_string(),
        source: sample.source.to_string(),
        original_shape: shape_label(&sample.matrix),
        active_shape: shape_label(&active),
        matrix: sample.matrix.clone(),
        active_block: active.clone(),
        coarse_signature: mass_support_signature(&sample.matrix),
        trimmed_active_window_signature: trimmed_active_window_signature(&sample.matrix),
        endpoint_local_parity_self_supported: sample.matrix.is_square()
            && matches!(sample.matrix.rows, 3 | 4),
        existing_power_trace_prefix_1_to_4: full_directed_closed_walk_traces_1_to_6[..4].to_vec(),
        full_directed_closed_walk_traces_1_to_6,
        full_directed_total_walks_1_to_4: total_walk_counts(
            &full_matrix,
            FULL_DIRECTED_TOTAL_WALK_MAX_POWER,
            "full directed total walk counts",
        )?,
        full_directed_adjacency_charpoly: charpoly_coefficients(
            &full_matrix,
            "full directed adjacency characteristic polynomial",
        )?,
        bowen_franks_i_minus_m: bowen_franks_i_minus_m(&sample.matrix)?,
        active_weighted_gram_traces_1_to_4: power_traces(
            &active_gram,
            ACTIVE_GRAM_TRACE_MAX_POWER,
            "active weighted gram traces",
        )?,
        active_weighted_gram_charpoly: charpoly_coefficients(
            &active_gram,
            "active weighted gram characteristic polynomial",
        )?,
        active_support_laplacian_charpoly: charpoly_coefficients(
            &support_laplacian,
            "active support laplacian characteristic polynomial",
        )?,
        active_weighted_laplacian_charpoly: charpoly_coefficients(
            &weighted_laplacian,
            "active weighted laplacian characteristic polynomial",
        )?,
    })
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
    let endpoint_local_parity_action = endpoint_local_parity_action(&left.matrix, &right.matrix)
        .as_str()
        .to_string();
    let same_existing_power_trace_prefix_1_to_4 =
        left.existing_power_trace_prefix_1_to_4 == right.existing_power_trace_prefix_1_to_4;
    let same_full_directed_closed_walk_traces_1_to_6 = left.full_directed_closed_walk_traces_1_to_6
        == right.full_directed_closed_walk_traces_1_to_6;
    let same_full_directed_total_walks_1_to_4 =
        left.full_directed_total_walks_1_to_4 == right.full_directed_total_walks_1_to_4;
    let same_full_directed_adjacency_charpoly =
        left.full_directed_adjacency_charpoly == right.full_directed_adjacency_charpoly;
    let same_bowen_franks_i_minus_m = left.bowen_franks_i_minus_m == right.bowen_franks_i_minus_m;
    let same_active_weighted_gram_spectrum =
        left.active_weighted_gram_charpoly == right.active_weighted_gram_charpoly;
    let same_active_support_laplacian_spectrum =
        left.active_support_laplacian_charpoly == right.active_support_laplacian_charpoly;
    let same_active_weighted_laplacian_spectrum =
        left.active_weighted_laplacian_charpoly == right.active_weighted_laplacian_charpoly;

    Ok(PairReport {
        id: pair.id.to_string(),
        label: pair.label.to_string(),
        pair_kind: pair.pair_kind.to_string(),
        left_id: pair.left_id.to_string(),
        right_id: pair.right_id.to_string(),
        same_coarse_signature,
        same_trimmed_active_window_signature,
        endpoint_local_parity_action,
        same_existing_power_trace_prefix_1_to_4,
        same_full_directed_closed_walk_traces_1_to_6,
        same_full_directed_total_walks_1_to_4,
        same_full_directed_adjacency_charpoly,
        same_bowen_franks_i_minus_m,
        same_active_weighted_gram_spectrum,
        same_active_support_laplacian_spectrum,
        same_active_weighted_laplacian_spectrum,
        prior_orbit_profile_decision: prior_orbit_profile_decision(pair.id).to_string(),
        prior_weighted_wl_decision: prior_weighted_wl_decision(pair.id).to_string(),
        prior_support_incidence_decision: prior_support_incidence_decision(pair.id).to_string(),
        diagnostic_reading: diagnostic_reading(
            pair.pair_kind,
            same_coarse_signature,
            same_trimmed_active_window_signature,
            same_existing_power_trace_prefix_1_to_4,
            same_full_directed_closed_walk_traces_1_to_6,
            same_full_directed_total_walks_1_to_4,
            same_full_directed_adjacency_charpoly,
            same_bowen_franks_i_minus_m,
            same_active_weighted_gram_spectrum,
            same_active_support_laplacian_spectrum,
            same_active_weighted_laplacian_spectrum,
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

fn prior_support_incidence_decision(pair_id: &str) -> &'static str {
    match pair_id {
        "brix_rank4_frontier_vs_counterpart" | "brix_rank6_frontier_vs_counterpart" => {
            "match_false_coarse_bucket_pair"
        }
        "baker_a4_to_a5" => "split_known_transfer_control",
        "k3_replay_overlap_step2" => "match_literal_replay_reuse",
        _ => "unknown",
    }
}

fn diagnostic_reading(
    pair_kind: &str,
    same_coarse_signature: bool,
    same_trimmed_active_window_signature: bool,
    same_existing_power_trace_prefix_1_to_4: bool,
    same_full_directed_closed_walk_traces_1_to_6: bool,
    same_full_directed_total_walks_1_to_4: bool,
    same_full_directed_adjacency_charpoly: bool,
    same_bowen_franks_i_minus_m: bool,
    same_active_weighted_gram_spectrum: bool,
    same_active_support_laplacian_spectrum: bool,
    same_active_weighted_laplacian_spectrum: bool,
) -> String {
    match pair_kind {
        "coarse_only_near_miss"
            if same_coarse_signature
                && same_existing_power_trace_prefix_1_to_4
                && same_full_directed_adjacency_charpoly
                && !same_trimmed_active_window_signature =>
        {
            format!(
                "full directed spectral data is exactly the existing power-trace surface on this 4x4 control and does not split the false coarse bucket; active weighted gram {} and weighted Laplacian {} only restate active-block weight/layout differences already split by trimmed_active_window/WL; support Laplacian {}",
                if same_active_weighted_gram_spectrum {
                    "matches"
                } else {
                    "splits"
                },
                if same_active_weighted_laplacian_spectrum {
                    "matches"
                } else {
                    "splits"
                },
                if same_active_support_laplacian_spectrum {
                    "matches the support-only prior descriptor"
                } else {
                    "splits despite forgetting weights"
                }
            )
        }
        "known_local_transfer_not_one_current_family"
            if same_existing_power_trace_prefix_1_to_4
                && same_full_directed_closed_walk_traces_1_to_6
                && same_full_directed_adjacency_charpoly
                && same_bowen_franks_i_minus_m
                && !same_full_directed_total_walks_1_to_4
                && !same_active_weighted_gram_spectrum
                && !same_active_support_laplacian_spectrum
                && !same_active_weighted_laplacian_spectrum =>
        {
            "full directed spectral data preserves the Baker A4 -> A5 transfer because it overlaps existing SSE power-trace invariants; active-block gram/laplacian descriptors split it, so they are not transfer-reuse signals".to_string()
        }
        "known_reuse_calibration"
            if same_trimmed_active_window_signature
                && same_existing_power_trace_prefix_1_to_4
                && same_full_directed_closed_walk_traces_1_to_6
                && same_full_directed_total_walks_1_to_4
                && same_full_directed_adjacency_charpoly
                && same_bowen_franks_i_minus_m
                && same_active_weighted_gram_spectrum
                && same_active_support_laplacian_spectrum
                && same_active_weighted_laplacian_spectrum =>
        {
            "all tested exact spectral/walk descriptors preserve this literal k3 replay-overlap calibration".to_string()
        }
        _ => "mixed spectral/walk result; inspect exact descriptor matches before promotion"
            .to_string(),
    }
}

fn active_block(matrix: &DynMatrix) -> DynMatrix {
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
    DynMatrix::new(rows.len(), cols.len(), data)
}

fn dyn_to_i128(matrix: &DynMatrix) -> Vec<Vec<i128>> {
    (0..matrix.rows)
        .map(|row| {
            (0..matrix.cols)
                .map(|col| matrix.get(row, col) as i128)
                .collect()
        })
        .collect()
}

fn gram_matrix(active: &DynMatrix) -> Vec<Vec<i128>> {
    let mut gram = vec![vec![0i128; active.rows]; active.rows];
    for left_row in 0..active.rows {
        for right_row in 0..active.rows {
            let mut value = 0i128;
            for col in 0..active.cols {
                value += active.get(left_row, col) as i128 * active.get(right_row, col) as i128;
            }
            gram[left_row][right_row] = value;
        }
    }
    gram
}

#[derive(Clone, Copy)]
enum EdgeWeightPolicy {
    Support,
    Weighted,
}

fn bipartite_laplacian(active: &DynMatrix, policy: EdgeWeightPolicy) -> Vec<Vec<i128>> {
    let rows = active.rows;
    let cols = active.cols;
    let dimension = rows + cols;
    let mut laplacian = vec![vec![0i128; dimension]; dimension];

    for row in 0..rows {
        for col in 0..cols {
            let entry = active.get(row, col);
            if entry == 0 {
                continue;
            }
            let weight = match policy {
                EdgeWeightPolicy::Support => 1i128,
                EdgeWeightPolicy::Weighted => entry as i128,
            };
            let row_vertex = row;
            let col_vertex = rows + col;
            laplacian[row_vertex][row_vertex] += weight;
            laplacian[col_vertex][col_vertex] += weight;
            laplacian[row_vertex][col_vertex] -= weight;
            laplacian[col_vertex][row_vertex] -= weight;
        }
    }

    laplacian
}

fn power_traces(matrix: &[Vec<i128>], max_power: usize, context: &str) -> Result<Vec<i64>, String> {
    ensure_square(matrix, context)?;
    let mut traces = Vec::with_capacity(max_power);
    let mut power = matrix.to_vec();
    for exponent in 1..=max_power {
        traces.push(checked_i64(trace_i128(&power), context)?);
        if exponent < max_power {
            power = checked_mul_i128(&power, matrix, context)?;
        }
    }
    Ok(traces)
}

fn total_walk_counts(
    matrix: &[Vec<i128>],
    max_power: usize,
    context: &str,
) -> Result<Vec<i64>, String> {
    ensure_square(matrix, context)?;
    let mut totals = Vec::with_capacity(max_power);
    let mut power = matrix.to_vec();
    for exponent in 1..=max_power {
        let total = power
            .iter()
            .flatten()
            .try_fold(0i128, |acc, &value| acc.checked_add(value))
            .ok_or_else(|| format!("overflow summing {context} at power {exponent}"))?;
        totals.push(checked_i64(total, context)?);
        if exponent < max_power {
            power = checked_mul_i128(&power, matrix, context)?;
        }
    }
    Ok(totals)
}

fn charpoly_coefficients(matrix: &[Vec<i128>], context: &str) -> Result<Vec<i64>, String> {
    ensure_square(matrix, context)?;
    let dimension = matrix.len();
    if dimension == 0 {
        return Ok(vec![1]);
    }

    let traces = power_traces_i128(matrix, dimension, context)?;
    let mut coeffs = vec![0i128; dimension + 1];
    coeffs[0] = 1;
    for k in 1..=dimension {
        let mut sum = 0i128;
        for i in 1..=k {
            let term = coeffs[k - i]
                .checked_mul(traces[i - 1])
                .ok_or_else(|| format!("overflow computing {context} coefficient c{k}"))?;
            sum = sum
                .checked_add(term)
                .ok_or_else(|| format!("overflow summing {context} coefficient c{k}"))?;
        }
        let divisor = k as i128;
        if sum % divisor != 0 {
            return Err(format!(
                "non-integral Newton identity while computing {context} coefficient c{k}: {sum}/{divisor}"
            ));
        }
        coeffs[k] = -sum / divisor;
    }

    coeffs
        .into_iter()
        .map(|coeff| checked_i64(coeff, context))
        .collect()
}

fn power_traces_i128(
    matrix: &[Vec<i128>],
    max_power: usize,
    context: &str,
) -> Result<Vec<i128>, String> {
    ensure_square(matrix, context)?;
    let mut traces = Vec::with_capacity(max_power);
    let mut power = matrix.to_vec();
    for exponent in 1..=max_power {
        traces.push(trace_i128(&power));
        if exponent < max_power {
            power = checked_mul_i128(&power, matrix, context)?;
        }
    }
    Ok(traces)
}

fn checked_mul_i128(
    left: &[Vec<i128>],
    right: &[Vec<i128>],
    context: &str,
) -> Result<Vec<Vec<i128>>, String> {
    let rows = left.len();
    let inner = left.first().map_or(0, Vec::len);
    if right.len() != inner {
        return Err(format!(
            "dimension mismatch multiplying {context}: {}x{} by {}x{}",
            rows,
            inner,
            right.len(),
            right.first().map_or(0, Vec::len)
        ));
    }
    let cols = right.first().map_or(0, Vec::len);
    let mut result = vec![vec![0i128; cols]; rows];

    for row in 0..rows {
        for (mid, &left_value) in left[row].iter().enumerate() {
            if left_value == 0 {
                continue;
            }
            for col in 0..cols {
                let product = left_value
                    .checked_mul(right[mid][col])
                    .ok_or_else(|| format!("overflow multiplying {context}"))?;
                result[row][col] = result[row][col]
                    .checked_add(product)
                    .ok_or_else(|| format!("overflow accumulating {context}"))?;
            }
        }
    }

    Ok(result)
}

fn trace_i128(matrix: &[Vec<i128>]) -> i128 {
    matrix.iter().enumerate().map(|(idx, row)| row[idx]).sum()
}

fn ensure_square(matrix: &[Vec<i128>], context: &str) -> Result<(), String> {
    let dimension = matrix.len();
    if matrix.iter().any(|row| row.len() != dimension) {
        return Err(format!("{context} requires a square matrix"));
    }
    Ok(())
}

fn checked_i64(value: i128, context: &str) -> Result<i64, String> {
    i64::try_from(value).map_err(|_| format!("{context} value does not fit in i64: {value}"))
}

fn bowen_franks_i_minus_m(matrix: &DynMatrix) -> Result<Vec<i64>, String> {
    if !matrix.is_square() {
        return Err(format!(
            "Bowen-Franks descriptor requires a square matrix, got {}",
            shape_label(matrix)
        ));
    }
    let identity_minus = (0..matrix.rows)
        .map(|row| {
            (0..matrix.cols)
                .map(|col| {
                    let diagonal = if row == col { 1i64 } else { 0i64 };
                    diagonal - matrix.get(row, col) as i64
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    smith_normal_form_invariants_i64(&identity_minus)
}

fn smith_normal_form_invariants_i64(matrix: &[Vec<i64>]) -> Result<Vec<i64>, String> {
    let n = matrix.len();
    if matrix.iter().any(|row| row.len() != n) {
        return Err("Smith normal form helper requires a square matrix".to_string());
    }

    let mut deltas = Vec::with_capacity(n + 1);
    deltas.push(1u128);
    let mut rank = 0usize;
    for minor_size in 1..=n {
        let delta = gcd_of_minors_i64(matrix, minor_size)?;
        if delta != 0 {
            rank = minor_size;
        }
        deltas.push(delta);
    }

    let mut invariants = Vec::with_capacity(n);
    let mut previous_delta = 1u128;
    for &delta in deltas.iter().take(rank + 1).skip(1) {
        let invariant = delta / previous_delta;
        invariants.push(i64::try_from(invariant).map_err(|_| {
            format!("Smith normal form invariant does not fit in i64: {invariant}")
        })?);
        previous_delta = delta;
    }
    invariants.resize(n, 0);
    Ok(invariants)
}

fn gcd_of_minors_i64(matrix: &[Vec<i64>], minor_size: usize) -> Result<u128, String> {
    let combinations = index_combinations(matrix.len(), minor_size);
    let mut gcd_acc = 0u128;

    for row_indices in &combinations {
        for col_indices in &combinations {
            let minor = row_indices
                .iter()
                .map(|&row| col_indices.iter().map(|&col| matrix[row][col]).collect())
                .collect::<Vec<Vec<i64>>>();
            let determinant = determinant_i128_checked(&minor)
                .ok_or_else(|| "overflow computing Smith normal form minor".to_string())?;
            gcd_acc = gcd_u128(gcd_acc, determinant.unsigned_abs());
        }
    }

    Ok(gcd_acc)
}

fn index_combinations(n: usize, choose: usize) -> Vec<Vec<usize>> {
    if choose == 0 {
        return vec![Vec::new()];
    }
    if choose > n {
        return Vec::new();
    }

    let mut current = (0..choose).collect::<Vec<_>>();
    let mut combinations = Vec::new();

    loop {
        combinations.push(current.clone());

        let Some(pivot) = (0..choose)
            .rev()
            .find(|&idx| current[idx] != idx + n - choose)
        else {
            break;
        };
        current[pivot] += 1;
        for idx in (pivot + 1)..choose {
            current[idx] = current[idx - 1] + 1;
        }
    }

    combinations
}

fn determinant_i128_checked(matrix: &[Vec<i64>]) -> Option<i128> {
    match matrix.len() {
        0 => Some(1),
        1 => Some(matrix[0][0] as i128),
        2 => {
            let a = matrix[0][0] as i128;
            let b = matrix[0][1] as i128;
            let c = matrix[1][0] as i128;
            let d = matrix[1][1] as i128;
            a.checked_mul(d)?.checked_sub(b.checked_mul(c)?)
        }
        _ => matrix[0]
            .iter()
            .enumerate()
            .filter(|(_, entry)| **entry != 0)
            .try_fold(0i128, |acc, (col, &entry)| {
                let minor = matrix[1..]
                    .iter()
                    .map(|row| {
                        row.iter()
                            .enumerate()
                            .filter_map(|(idx, &value)| (idx != col).then_some(value))
                            .collect::<Vec<_>>()
                    })
                    .collect::<Vec<_>>();
                let sign = if col % 2 == 0 { 1i128 } else { -1i128 };
                let term = sign
                    .checked_mul(entry as i128)?
                    .checked_mul(determinant_i128_checked(&minor)?)?;
                acc.checked_add(term)
            }),
    }
}

fn gcd_u128(mut a: u128, mut b: u128) -> u128 {
    while b != 0 {
        let t = b;
        b = a % b;
        a = t;
    }
    a
}

fn shape_label(matrix: &DynMatrix) -> String {
    format!("{}x{}", matrix.rows, matrix.cols)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn full_directed_spectral_data_matches_existing_trace_prefix_on_all_pairs() {
        let reports = sample_report_map();
        for pair in selected_pairs() {
            let left = &reports[pair.left_id];
            let right = &reports[pair.right_id];
            assert_eq!(
                left.existing_power_trace_prefix_1_to_4, right.existing_power_trace_prefix_1_to_4,
                "{} should match existing square power traces",
                pair.id
            );
            assert_eq!(
                left.full_directed_adjacency_charpoly, right.full_directed_adjacency_charpoly,
                "{} should match full directed charpoly",
                pair.id
            );
        }
    }

    #[test]
    fn bowen_franks_matches_all_pairs_but_total_walk_counts_split_non_reuse_controls() {
        let reports = sample_report_map();
        for pair in selected_pairs() {
            let left = &reports[pair.left_id];
            let right = &reports[pair.right_id];
            assert_eq!(
                left.bowen_franks_i_minus_m, right.bowen_franks_i_minus_m,
                "{} should match the existing Bowen-Franks descriptor",
                pair.id
            );

            match pair.id {
                "k3_replay_overlap_step2" => assert_eq!(
                    left.full_directed_total_walks_1_to_4, right.full_directed_total_walks_1_to_4,
                    "{} should preserve total-walk counts",
                    pair.id
                ),
                "brix_rank4_frontier_vs_counterpart"
                | "brix_rank6_frontier_vs_counterpart"
                | "baker_a4_to_a5" => assert_ne!(
                    left.full_directed_total_walks_1_to_4, right.full_directed_total_walks_1_to_4,
                    "{} should split total-walk counts",
                    pair.id
                ),
                _ => panic!("unexpected pair id {}", pair.id),
            }
        }
    }

    #[test]
    fn active_weighted_gram_spectrum_separates_brix_coarse_only_pairs() {
        let reports = sample_report_map();
        for (left_id, right_id) in [
            ("brix_rank4_frontier", "brix_rank4_counterpart"),
            ("brix_rank6_frontier", "brix_rank6_counterpart"),
        ] {
            let left = &reports[left_id];
            let right = &reports[right_id];
            assert_eq!(left.coarse_signature, right.coarse_signature);
            assert_ne!(
                left.trimmed_active_window_signature,
                right.trimmed_active_window_signature
            );
            assert_ne!(
                left.active_weighted_gram_charpoly,
                right.active_weighted_gram_charpoly
            );
        }
    }

    #[test]
    fn support_laplacian_matches_brix_pairs_but_weighted_laplacian_splits_them() {
        let reports = sample_report_map();
        for (left_id, right_id) in [
            ("brix_rank4_frontier", "brix_rank4_counterpart"),
            ("brix_rank6_frontier", "brix_rank6_counterpart"),
        ] {
            let left = &reports[left_id];
            let right = &reports[right_id];
            assert_eq!(
                left.active_support_laplacian_charpoly,
                right.active_support_laplacian_charpoly
            );
            assert_ne!(
                left.active_weighted_laplacian_charpoly,
                right.active_weighted_laplacian_charpoly
            );
        }
    }

    #[test]
    fn active_block_spectral_descriptors_preserve_k3_replay_overlap() {
        let reports = sample_report_map();
        let left = &reports["k3_baker_step2"];
        let right = &reports["k3_non_baker_step2"];
        assert_eq!(
            left.active_weighted_gram_charpoly,
            right.active_weighted_gram_charpoly
        );
        assert_eq!(
            left.active_support_laplacian_charpoly,
            right.active_support_laplacian_charpoly
        );
        assert_eq!(
            left.active_weighted_laplacian_charpoly,
            right.active_weighted_laplacian_charpoly
        );
    }

    #[test]
    fn active_block_spectral_descriptors_split_baker_transfer_control() {
        let reports = sample_report_map();
        let left = &reports["baker_a4"];
        let right = &reports["baker_a5"];
        assert_eq!(
            left.full_directed_adjacency_charpoly,
            right.full_directed_adjacency_charpoly
        );
        assert_ne!(
            left.active_weighted_gram_charpoly,
            right.active_weighted_gram_charpoly
        );
        assert_ne!(
            left.active_support_laplacian_charpoly,
            right.active_support_laplacian_charpoly
        );
        assert_ne!(
            left.active_weighted_laplacian_charpoly,
            right.active_weighted_laplacian_charpoly
        );
    }

    #[test]
    fn charpoly_newton_identity_for_diagonal_matrix() {
        let matrix = vec![vec![2, 0, 0], vec![0, 3, 0], vec![0, 0, 5]];
        assert_eq!(
            charpoly_coefficients(&matrix, "diagonal charpoly").unwrap(),
            vec![1, -10, 31, -30]
        );
    }

    fn sample_report_map() -> BTreeMap<&'static str, SampleReport> {
        selected_samples()
            .iter()
            .map(|sample| {
                (
                    sample.id,
                    build_sample_report(sample).expect("sample report"),
                )
            })
            .collect()
    }
}
