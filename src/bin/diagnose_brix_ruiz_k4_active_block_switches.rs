use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sse_core::matrix::DynMatrix;

const DEFAULT_RANK: usize = 4;
const DEFAULT_MAX_DELTA: u32 = 12;
const MAX_DELTA_CAP: u32 = 64;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("{}", usage());
        return Ok(());
    }

    let cli = parse_cli(args.into_iter())?;
    let raw = fs::read_to_string(&cli.input)
        .map_err(|err| format!("failed to read {}: {err}", cli.input.display()))?;
    let report: StuckStateReport =
        serde_json::from_str(&raw).map_err(|err| format!("failed to parse input JSON: {err}"))?;
    let hit = report
        .ranked_approximate_hits
        .iter()
        .find(|hit| hit.rank == cli.rank)
        .ok_or_else(|| format!("rank {} not present in input report", cli.rank))?;
    let analysis = analyze_hit(hit, cli.max_delta)?;
    let json = serde_json::to_string_pretty(&analysis)
        .map_err(|err| format!("failed to serialize analysis: {err}"))?;

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
    input: PathBuf,
    json_out: Option<PathBuf>,
    rank: usize,
    max_delta: u32,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut input = None;
    let mut json_out = None;
    let mut rank = DEFAULT_RANK;
    let mut max_delta = DEFAULT_MAX_DELTA;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                input = Some(PathBuf::from(args.next().ok_or("--input requires a path")?));
            }
            "--json-out" => {
                json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--rank" => {
                rank = args
                    .next()
                    .ok_or("--rank requires a value")?
                    .parse()
                    .map_err(|_| "invalid --rank".to_string())?;
            }
            "--max-delta" => {
                max_delta = args
                    .next()
                    .ok_or("--max-delta requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-delta".to_string())?;
            }
            "--help" | "-h" => {
                return Err(usage());
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    let input = input.ok_or("--input is required")?;
    if rank == 0 {
        return Err("--rank must be at least 1".to_string());
    }
    if max_delta == 0 {
        return Err("--max-delta must be at least 1".to_string());
    }
    if max_delta > MAX_DELTA_CAP {
        return Err(format!("--max-delta must be at most {MAX_DELTA_CAP}"));
    }

    Ok(Cli {
        input,
        json_out,
        rank,
        max_delta,
    })
}

fn usage() -> String {
    format!(
        "usage: diagnose_brix_ruiz_k4_active_block_switches --input PATH [--rank N] [--max-delta N <= {MAX_DELTA_CAP}] [--json-out PATH]"
    )
}

#[derive(Deserialize)]
struct StuckStateReport {
    ranked_approximate_hits: Vec<ApproximateHit>,
}

#[derive(Deserialize)]
struct ApproximateHit {
    rank: usize,
    layer_index: usize,
    direction: String,
    move_family: String,
    from_depth: usize,
    to_depth: usize,
    counterpart_depth: Option<usize>,
    bridge_slack_at_lag40: Option<isize>,
    counterpart_l1: Option<u64>,
    from_matrix: DynMatrix,
    to_matrix: DynMatrix,
    counterpart_matrix: Option<DynMatrix>,
}

#[derive(Serialize)]
struct SwitchAnalysis {
    proposal: &'static str,
    decision_scope: &'static str,
    rank: usize,
    layer_index: usize,
    direction: String,
    move_family: String,
    depths: DepthSummary,
    active_rows: Vec<usize>,
    active_cols: Vec<usize>,
    max_delta: u32,
    base_canonical_distance: u64,
    best_canonical_distance: u64,
    exact_canonical_distance_improved: bool,
    exact_canonical_match_found: bool,
    total_proposals: usize,
    accepted_signature_preserving: usize,
    rejected_signature_changing: usize,
    improved_signature_preserving: usize,
    best_proposals: Vec<SwitchProposal>,
    from_matrix: DynMatrix,
    to_matrix: DynMatrix,
    counterpart_matrix: DynMatrix,
}

#[derive(Serialize)]
struct DepthSummary {
    from_depth: usize,
    to_depth: usize,
    counterpart_depth: Option<usize>,
    bridge_slack_at_lag40: Option<isize>,
    retained_counterpart_l1: Option<u64>,
}

#[derive(Clone, Serialize)]
struct SwitchProposal {
    row_pair: [usize; 2],
    col_pair: [usize; 2],
    orientation: SwitchOrientation,
    delta: u32,
    signature_preserving: bool,
    canonical_distance: u64,
    improvement: i64,
    candidate_matrix: DynMatrix,
    candidate_canonical: DynMatrix,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
enum SwitchOrientation {
    AddMainDiagonal,
    AddAntiDiagonal,
}

fn analyze_hit(hit: &ApproximateHit, max_delta: u32) -> Result<SwitchAnalysis, String> {
    if hit.move_family != "diagonal_refactorization_4x4" {
        return Err(format!(
            "rank {} is {}, expected diagonal_refactorization_4x4",
            hit.rank, hit.move_family
        ));
    }
    validate_square_4x4(&hit.from_matrix, "from_matrix")?;
    validate_square_4x4(&hit.to_matrix, "to_matrix")?;
    let counterpart = hit
        .counterpart_matrix
        .clone()
        .ok_or_else(|| format!("rank {} has no counterpart_matrix", hit.rank))?;
    validate_square_4x4(&counterpart, "counterpart_matrix")?;

    let active_rows = active_rows(&hit.to_matrix);
    let active_cols = active_cols(&hit.to_matrix);
    if active_rows.len() != 2 || active_cols.len() != 4 {
        return Err(format!(
            "rank {} active block is {}x{}, expected 2x4",
            hit.rank,
            active_rows.len(),
            active_cols.len()
        ));
    }

    let base_signature = approximate_signature(&hit.to_matrix.canonical_perm());
    let counterpart_canonical = counterpart.canonical_perm();
    let base_canonical = hit.to_matrix.canonical_perm();
    let base_distance = matrix_l1_distance(&base_canonical, &counterpart_canonical);
    let mut proposals = enumerate_switches(&hit.to_matrix, &active_rows, &active_cols, max_delta)
        .into_iter()
        .map(|mut proposal| {
            proposal.signature_preserving =
                approximate_signature(&proposal.candidate_matrix.canonical_perm())
                    == base_signature;
            proposal.candidate_canonical = proposal.candidate_matrix.canonical_perm();
            proposal.canonical_distance =
                matrix_l1_distance(&proposal.candidate_canonical, &counterpart_canonical);
            proposal.improvement = base_distance as i64 - proposal.canonical_distance as i64;
            proposal
        })
        .collect::<Vec<_>>();

    proposals.sort_by(|left, right| {
        left.canonical_distance
            .cmp(&right.canonical_distance)
            .then_with(|| right.signature_preserving.cmp(&left.signature_preserving))
            .then_with(|| left.row_pair.cmp(&right.row_pair))
            .then_with(|| left.col_pair.cmp(&right.col_pair))
            .then_with(|| left.delta.cmp(&right.delta))
    });

    let total_proposals = proposals.len();
    let accepted_signature_preserving = proposals
        .iter()
        .filter(|proposal| proposal.signature_preserving)
        .count();
    let rejected_signature_changing = total_proposals - accepted_signature_preserving;
    let improved_signature_preserving = proposals
        .iter()
        .filter(|proposal| {
            proposal.signature_preserving && proposal.canonical_distance < base_distance
        })
        .count();
    let accepted_proposals = proposals
        .iter()
        .filter(|proposal| proposal.signature_preserving)
        .cloned()
        .collect::<Vec<_>>();
    let best_canonical_distance = accepted_proposals
        .first()
        .map(|proposal| proposal.canonical_distance)
        .unwrap_or(base_distance);
    let exact_canonical_match_found = accepted_proposals
        .iter()
        .any(|proposal| proposal.canonical_distance == 0);

    Ok(SwitchAnalysis {
        proposal: "bounded_2x2_active_block_contingency_switch",
        decision_scope: "diagnostic_only_not_an_sse_family",
        rank: hit.rank,
        layer_index: hit.layer_index,
        direction: hit.direction.clone(),
        move_family: hit.move_family.clone(),
        depths: DepthSummary {
            from_depth: hit.from_depth,
            to_depth: hit.to_depth,
            counterpart_depth: hit.counterpart_depth,
            bridge_slack_at_lag40: hit.bridge_slack_at_lag40,
            retained_counterpart_l1: hit.counterpart_l1,
        },
        active_rows,
        active_cols,
        max_delta,
        base_canonical_distance: base_distance,
        best_canonical_distance,
        exact_canonical_distance_improved: best_canonical_distance < base_distance,
        exact_canonical_match_found,
        total_proposals,
        accepted_signature_preserving,
        rejected_signature_changing,
        improved_signature_preserving,
        best_proposals: accepted_proposals.into_iter().take(8).collect(),
        from_matrix: hit.from_matrix.clone(),
        to_matrix: hit.to_matrix.clone(),
        counterpart_matrix: counterpart,
    })
}

fn validate_square_4x4(matrix: &DynMatrix, label: &str) -> Result<(), String> {
    if matrix.rows == 4 && matrix.cols == 4 && matrix.data.len() == 16 {
        Ok(())
    } else {
        Err(format!(
            "{label} is {}x{} with {} entries, expected 4x4",
            matrix.rows,
            matrix.cols,
            matrix.data.len()
        ))
    }
}

fn enumerate_switches(
    matrix: &DynMatrix,
    active_rows: &[usize],
    active_cols: &[usize],
    max_delta: u32,
) -> Vec<SwitchProposal> {
    let mut proposals = Vec::new();
    for row_left_idx in 0..active_rows.len() {
        for row_right_idx in row_left_idx + 1..active_rows.len() {
            let row_pair = [active_rows[row_left_idx], active_rows[row_right_idx]];
            for col_left_idx in 0..active_cols.len() {
                for col_right_idx in col_left_idx + 1..active_cols.len() {
                    let col_pair = [active_cols[col_left_idx], active_cols[col_right_idx]];
                    push_switches(
                        matrix,
                        row_pair,
                        col_pair,
                        SwitchOrientation::AddMainDiagonal,
                        max_delta,
                        &mut proposals,
                    );
                    push_switches(
                        matrix,
                        row_pair,
                        col_pair,
                        SwitchOrientation::AddAntiDiagonal,
                        max_delta,
                        &mut proposals,
                    );
                }
            }
        }
    }
    proposals
}

fn push_switches(
    matrix: &DynMatrix,
    row_pair: [usize; 2],
    col_pair: [usize; 2],
    orientation: SwitchOrientation,
    delta_cap: u32,
    proposals: &mut Vec<SwitchProposal>,
) {
    let [r0, r1] = row_pair;
    let [c0, c1] = col_pair;
    let max_delta = match orientation {
        SwitchOrientation::AddMainDiagonal => matrix.get(r0, c1).min(matrix.get(r1, c0)),
        SwitchOrientation::AddAntiDiagonal => matrix.get(r0, c0).min(matrix.get(r1, c1)),
    }
    .min(delta_cap);
    for delta in 1..=max_delta {
        let mut candidate = matrix.clone();
        match orientation {
            SwitchOrientation::AddMainDiagonal => {
                let Some(next_r0_c0) = candidate.get(r0, c0).checked_add(delta) else {
                    continue;
                };
                let Some(next_r1_c1) = candidate.get(r1, c1).checked_add(delta) else {
                    continue;
                };
                candidate.set(r0, c0, next_r0_c0);
                candidate.set(r0, c1, candidate.get(r0, c1) - delta);
                candidate.set(r1, c0, candidate.get(r1, c0) - delta);
                candidate.set(r1, c1, next_r1_c1);
            }
            SwitchOrientation::AddAntiDiagonal => {
                let Some(next_r0_c1) = candidate.get(r0, c1).checked_add(delta) else {
                    continue;
                };
                let Some(next_r1_c0) = candidate.get(r1, c0).checked_add(delta) else {
                    continue;
                };
                candidate.set(r0, c0, candidate.get(r0, c0) - delta);
                candidate.set(r0, c1, next_r0_c1);
                candidate.set(r1, c0, next_r1_c0);
                candidate.set(r1, c1, candidate.get(r1, c1) - delta);
            }
        }
        proposals.push(SwitchProposal {
            row_pair,
            col_pair,
            orientation,
            delta,
            signature_preserving: false,
            canonical_distance: u64::MAX,
            improvement: i64::MIN,
            candidate_matrix: candidate,
            candidate_canonical: DynMatrix::new(4, 4, vec![0; 16]),
        });
    }
}

fn active_rows(matrix: &DynMatrix) -> Vec<usize> {
    (0..matrix.rows)
        .filter(|&row| (0..matrix.cols).any(|col| matrix.get(row, col) != 0))
        .collect()
}

fn active_cols(matrix: &DynMatrix) -> Vec<usize> {
    (0..matrix.cols)
        .filter(|&col| (0..matrix.rows).any(|row| matrix.get(row, col) != 0))
        .collect()
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ApproxSignature {
    row_sums: Vec<u64>,
    col_sums: Vec<u64>,
    row_supports: Vec<u8>,
    col_supports: Vec<u8>,
}

fn approximate_signature(matrix: &DynMatrix) -> ApproxSignature {
    let mut row_sums = vec![0u64; matrix.rows];
    let mut col_sums = vec![0u64; matrix.cols];
    let mut row_supports = vec![0u8; matrix.rows];
    let mut col_supports = vec![0u8; matrix.cols];

    for row in 0..matrix.rows {
        for col in 0..matrix.cols {
            let value = matrix.get(row, col);
            row_sums[row] += value as u64;
            col_sums[col] += value as u64;
            if value != 0 {
                row_supports[row] += 1;
                col_supports[col] += 1;
            }
        }
    }

    row_sums.sort_unstable();
    col_sums.sort_unstable();
    row_supports.sort_unstable();
    col_supports.sort_unstable();

    ApproxSignature {
        row_sums,
        col_sums,
        row_supports,
        col_supports,
    }
}

fn matrix_l1_distance(left: &DynMatrix, right: &DynMatrix) -> u64 {
    if left.rows != right.rows || left.cols != right.cols {
        return u64::MAX;
    }
    left.data
        .iter()
        .zip(&right.data)
        .map(|(&a, &b)| a.abs_diff(b) as u64)
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rank4_fixture_counts_signature_preserving_switches() {
        let hit = ApproximateHit {
            rank: 4,
            layer_index: 75,
            direction: "forward".to_string(),
            move_family: "diagonal_refactorization_4x4".to_string(),
            from_depth: 35,
            to_depth: 36,
            counterpart_depth: Some(2),
            bridge_slack_at_lag40: Some(2),
            counterpart_l1: Some(22),
            from_matrix: DynMatrix::new(4, 4, vec![1, 4, 1, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]),
            to_matrix: DynMatrix::new(4, 4, vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0]),
            counterpart_matrix: Some(DynMatrix::new(
                4,
                4,
                vec![1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0],
            )),
        };

        let analysis = analyze_hit(&hit, DEFAULT_MAX_DELTA).expect("rank-4 fixture should analyze");

        assert_eq!(analysis.max_delta, DEFAULT_MAX_DELTA);
        assert_eq!(analysis.total_proposals, 18);
        assert_eq!(analysis.accepted_signature_preserving, 10);
        assert_eq!(analysis.rejected_signature_changing, 8);
        assert_eq!(analysis.improved_signature_preserving, 2);
        assert_eq!(analysis.base_canonical_distance, 22);
        assert_eq!(analysis.best_canonical_distance, 20);
        assert!(!analysis.exact_canonical_match_found);
        assert!(analysis
            .best_proposals
            .iter()
            .all(|proposal| proposal.signature_preserving));
    }

    #[test]
    fn switch_enumeration_skips_overflowing_additions() {
        let matrix = DynMatrix::new(
            4,
            4,
            vec![u32::MAX, 1, 0, 0, 1, u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        let mut proposals = Vec::new();

        push_switches(
            &matrix,
            [0, 1],
            [0, 1],
            SwitchOrientation::AddMainDiagonal,
            DEFAULT_MAX_DELTA,
            &mut proposals,
        );

        assert!(proposals.is_empty());
    }

    #[test]
    fn anti_diagonal_switches_are_capped_for_large_entries() {
        let matrix = DynMatrix::new(
            4,
            4,
            vec![u32::MAX, 0, 0, 0, 0, u32::MAX, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0],
        );
        let mut proposals = Vec::new();

        push_switches(
            &matrix,
            [0, 1],
            [0, 1],
            SwitchOrientation::AddAntiDiagonal,
            3,
            &mut proposals,
        );

        assert_eq!(proposals.len(), 3);
        assert_eq!(proposals.last().map(|proposal| proposal.delta), Some(3));
    }

    #[test]
    fn cli_rejects_unbounded_delta_caps() {
        let args = vec![
            "--input".to_string(),
            "tmp/input.json".to_string(),
            "--max-delta".to_string(),
            (MAX_DELTA_CAP + 1).to_string(),
        ];

        let err = parse_cli(args.into_iter()).expect_err("large cap should be rejected");

        assert!(err.contains("--max-delta must be at most"));
    }
}
