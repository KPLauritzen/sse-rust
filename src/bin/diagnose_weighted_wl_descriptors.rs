use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::Serialize;
use sse_core::endpoint_local_parity::{mass_support_signature, trimmed_active_window_signature};
use sse_core::matrix::DynMatrix;

const ROUNDS: [usize; 3] = [1, 2, 3];

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
    let report = WlReport {
        models: vec![
            ModelDescription {
                name: "weighted_active_bipartite_wl".to_string(),
                rounds: ROUNDS.to_vec(),
                description: "Delete all-zero rows and columns. Start row vertices with color R and column vertices with color C. For each round, recolor each row by its previous row color plus the sorted multiset of (entry weight, previous column color) over nonzero row incidences; recolor columns dually. The descriptor is the sorted row-color histogram plus sorted column-color histogram after the selected round.".to_string(),
            },
            ModelDescription {
                name: "directed_weighted_matrix_wl".to_string(),
                rounds: ROUNDS.to_vec(),
                description: "Use the square matrix as a directed weighted graph on one vertex set. Start all vertices with color V. For each round, recolor each vertex by its previous color plus sorted outgoing and incoming multisets of (entry weight, previous neighbor color) over nonzero entries. The descriptor is the sorted vertex-color histogram after the selected round.".to_string(),
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
    "usage: diagnose_weighted_wl_descriptors [--json-out PATH]".to_string()
}

#[derive(Serialize)]
struct WlReport {
    models: Vec<ModelDescription>,
    samples: Vec<SampleReport>,
    pairs: Vec<PairReport>,
}

#[derive(Serialize)]
struct ModelDescription {
    name: String,
    rounds: Vec<usize>,
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
    weighted_active_bipartite_wl: Vec<BipartiteWlRoundReport>,
    directed_weighted_matrix_wl: Vec<DirectedWlRoundReport>,
}

#[derive(Clone, Serialize)]
struct BipartiteWlRoundReport {
    round: usize,
    signature: String,
    row_color_histogram: Vec<ColorBucket>,
    column_color_histogram: Vec<ColorBucket>,
}

#[derive(Clone, Serialize)]
struct DirectedWlRoundReport {
    round: usize,
    signature: String,
    vertex_color_histogram: Vec<ColorBucket>,
}

#[derive(Clone, Serialize)]
struct ColorBucket {
    color: String,
    count: usize,
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
    weighted_active_bipartite_wl_matches: Vec<RoundMatch>,
    directed_weighted_matrix_wl_matches: Vec<RoundMatch>,
    diagnostic_reading: String,
}

#[derive(Serialize)]
struct RoundMatch {
    round: usize,
    same_signature: bool,
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
    SampleReport {
        id: sample.id.to_string(),
        label: sample.label.to_string(),
        source: sample.source.to_string(),
        original_shape: shape_label(&sample.matrix),
        active_shape: shape_label(&active),
        matrix: sample.matrix.clone(),
        active_block: active.clone(),
        coarse_signature: mass_support_signature(&sample.matrix),
        trimmed_active_window_signature: trimmed_active_window_signature(&sample.matrix),
        weighted_active_bipartite_wl: ROUNDS
            .iter()
            .map(|&round| weighted_active_bipartite_wl(&sample.matrix, round))
            .collect(),
        directed_weighted_matrix_wl: ROUNDS
            .iter()
            .map(|&round| directed_weighted_matrix_wl(&sample.matrix, round))
            .collect(),
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
    let bipartite_matches = round_matches(
        &left.weighted_active_bipartite_wl,
        &right.weighted_active_bipartite_wl,
        |round| round.round,
        |round| &round.signature,
    );
    let directed_matches = round_matches(
        &left.directed_weighted_matrix_wl,
        &right.directed_weighted_matrix_wl,
        |round| round.round,
        |round| &round.signature,
    );
    let same_coarse_signature = left.coarse_signature == right.coarse_signature;
    let same_trimmed_active_window_signature =
        left.trimmed_active_window_signature == right.trimmed_active_window_signature;
    let diagnostic_reading = diagnostic_reading(
        pair.pair_kind,
        same_coarse_signature,
        same_trimmed_active_window_signature,
        &bipartite_matches,
        &directed_matches,
    );

    Ok(PairReport {
        id: pair.id.to_string(),
        label: pair.label.to_string(),
        pair_kind: pair.pair_kind.to_string(),
        left_id: pair.left_id.to_string(),
        right_id: pair.right_id.to_string(),
        same_coarse_signature,
        same_trimmed_active_window_signature,
        weighted_active_bipartite_wl_matches: bipartite_matches,
        directed_weighted_matrix_wl_matches: directed_matches,
        diagnostic_reading,
    })
}

fn round_matches<T, FRound, FSignature>(
    left: &[T],
    right: &[T],
    round_of: FRound,
    signature_of: FSignature,
) -> Vec<RoundMatch>
where
    FRound: Fn(&T) -> usize,
    FSignature: Fn(&T) -> &str,
{
    left.iter()
        .map(|left_round| {
            let round = round_of(left_round);
            let same_signature = right
                .iter()
                .find(|right_round| round_of(right_round) == round)
                .is_some_and(|right_round| signature_of(left_round) == signature_of(right_round));
            RoundMatch {
                round,
                same_signature,
            }
        })
        .collect()
}

fn diagnostic_reading(
    pair_kind: &str,
    same_coarse_signature: bool,
    same_trimmed_active_window_signature: bool,
    bipartite_matches: &[RoundMatch],
    directed_matches: &[RoundMatch],
) -> String {
    let bipartite_all_match = bipartite_matches.iter().all(|round| round.same_signature);
    let bipartite_any_match = bipartite_matches.iter().any(|round| round.same_signature);
    let directed_all_match = directed_matches.iter().all(|round| round.same_signature);

    match pair_kind {
        "coarse_only_near_miss" if same_coarse_signature && !bipartite_any_match => {
            "weighted active bipartite WL separates this false coarse-bucket match from round 1; this agrees with trimmed_active_window separation and is more graded than the rejected singleton weighted-orbit profile".to_string()
        }
        "known_reuse_calibration" if bipartite_all_match && directed_all_match => {
            "both WL descriptors preserve this literal k3 replay overlap for rounds 1-3".to_string()
        }
        "known_local_transfer_not_one_current_family" if !bipartite_any_match => {
            "WL separates the Baker A4 -> A5 local-transfer control; useful as a difference descriptor, not as a transfer-invariance signal".to_string()
        }
        _ if same_trimmed_active_window_signature && bipartite_all_match => {
            "WL agrees with existing trimmed_active_window reuse on this control".to_string()
        }
        _ => "mixed WL result; inspect round-level matches before promotion".to_string(),
    }
}

fn weighted_active_bipartite_wl(matrix: &DynMatrix, rounds: usize) -> BipartiteWlRoundReport {
    let active = active_block(matrix);
    let mut row_colors = vec!["R".to_string(); active.rows];
    let mut col_colors = vec!["C".to_string(); active.cols];

    for _ in 0..rounds {
        let next_row_colors = (0..active.rows)
            .map(|row| {
                let mut incidents = Vec::new();
                for (col, col_color) in col_colors.iter().enumerate() {
                    let value = active.get(row, col);
                    if value != 0 {
                        incidents.push(format!("{value}:{col_color}"));
                    }
                }
                incidents.sort();
                format!("R({})[{}]", row_colors[row], join_strings(&incidents))
            })
            .collect::<Vec<_>>();
        let next_col_colors = (0..active.cols)
            .map(|col| {
                let mut incidents = Vec::new();
                for (row, row_color) in row_colors.iter().enumerate() {
                    let value = active.get(row, col);
                    if value != 0 {
                        incidents.push(format!("{value}:{row_color}"));
                    }
                }
                incidents.sort();
                format!("C({})[{}]", col_colors[col], join_strings(&incidents))
            })
            .collect::<Vec<_>>();
        row_colors = next_row_colors;
        col_colors = next_col_colors;
    }

    let row_color_histogram = color_histogram(&row_colors);
    let column_color_histogram = color_histogram(&col_colors);
    BipartiteWlRoundReport {
        round: rounds,
        signature: format!(
            "active_shape={}|rows={}|cols={}",
            shape_label(&active),
            histogram_signature(&row_color_histogram),
            histogram_signature(&column_color_histogram)
        ),
        row_color_histogram,
        column_color_histogram,
    }
}

fn directed_weighted_matrix_wl(matrix: &DynMatrix, rounds: usize) -> DirectedWlRoundReport {
    assert!(matrix.is_square());
    let mut colors = vec!["V".to_string(); matrix.rows];

    for _ in 0..rounds {
        let next_colors = (0..matrix.rows)
            .map(|vertex| {
                let mut outgoing = Vec::new();
                let mut incoming = Vec::new();
                for (other, other_color) in colors.iter().enumerate() {
                    let out_value = matrix.get(vertex, other);
                    if out_value != 0 {
                        outgoing.push(format!("{out_value}:{other_color}"));
                    }
                    let in_value = matrix.get(other, vertex);
                    if in_value != 0 {
                        incoming.push(format!("{in_value}:{other_color}"));
                    }
                }
                outgoing.sort();
                incoming.sort();
                format!(
                    "V({})|out[{}]|in[{}]",
                    colors[vertex],
                    join_strings(&outgoing),
                    join_strings(&incoming)
                )
            })
            .collect::<Vec<_>>();
        colors = next_colors;
    }

    let vertex_color_histogram = color_histogram(&colors);
    DirectedWlRoundReport {
        round: rounds,
        signature: format!(
            "dim={}|vertices={}",
            matrix.rows,
            histogram_signature(&vertex_color_histogram)
        ),
        vertex_color_histogram,
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

fn color_histogram(colors: &[String]) -> Vec<ColorBucket> {
    let mut counts = BTreeMap::<String, usize>::new();
    for color in colors {
        *counts.entry(color.clone()).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(color, count)| ColorBucket { color, count })
        .collect()
}

fn histogram_signature(histogram: &[ColorBucket]) -> String {
    histogram
        .iter()
        .map(|bucket| format!("{}*{}", bucket.count, bucket.color))
        .collect::<Vec<_>>()
        .join(";")
}

fn join_strings(values: &[String]) -> String {
    values.join(",")
}

fn shape_label(matrix: &DynMatrix) -> String {
    format!("{}x{}", matrix.rows, matrix.cols)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn active_bipartite_wl_separates_brix_coarse_only_pairs() {
        let samples = sample_map();
        for (left_id, right_id) in [
            ("brix_rank4_frontier", "brix_rank4_counterpart"),
            ("brix_rank6_frontier", "brix_rank6_counterpart"),
        ] {
            let left = &samples[left_id].matrix;
            let right = &samples[right_id].matrix;
            assert_eq!(mass_support_signature(left), mass_support_signature(right));

            for round in ROUNDS {
                assert_ne!(
                    weighted_active_bipartite_wl(left, round).signature,
                    weighted_active_bipartite_wl(right, round).signature
                );
            }
        }
    }

    #[test]
    fn active_bipartite_wl_preserves_k3_replay_overlap() {
        let samples = sample_map();
        let left = &samples["k3_baker_step2"].matrix;
        let right = &samples["k3_non_baker_step2"].matrix;

        for round in ROUNDS {
            assert_eq!(
                weighted_active_bipartite_wl(left, round).signature,
                weighted_active_bipartite_wl(right, round).signature
            );
        }
    }

    #[test]
    fn directed_matrix_wl_preserves_k3_replay_overlap() {
        let samples = sample_map();
        let left = &samples["k3_baker_step2"].matrix;
        let right = &samples["k3_non_baker_step2"].matrix;

        for round in ROUNDS {
            assert_eq!(
                directed_weighted_matrix_wl(left, round).signature,
                directed_weighted_matrix_wl(right, round).signature
            );
        }
    }

    #[test]
    fn weighted_wl_separates_baker_same_size_control() {
        let samples = sample_map();
        let left = &samples["baker_a4"].matrix;
        let right = &samples["baker_a5"].matrix;

        for round in ROUNDS {
            assert_ne!(
                weighted_active_bipartite_wl(left, round).signature,
                weighted_active_bipartite_wl(right, round).signature
            );
            assert_ne!(
                directed_weighted_matrix_wl(left, round).signature,
                directed_weighted_matrix_wl(right, round).signature
            );
        }
    }

    fn sample_map() -> BTreeMap<&'static str, Sample> {
        selected_samples()
            .into_iter()
            .map(|sample| (sample.id, sample))
            .collect()
    }
}
