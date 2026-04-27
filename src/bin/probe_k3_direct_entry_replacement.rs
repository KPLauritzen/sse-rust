use std::env;

use serde::Serialize;
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::types::GuideArtifactPayload;

const SOURCE: [u32; 4] = [1, 3, 2, 1];

const BAKER_INTERMEDIATE: [u32; 9] = [1, 2, 2, 2, 1, 1, 1, 0, 0];
const BAKER_TARGET: [u32; 16] = [1, 2, 2, 0, 1, 0, 2, 0, 0, 1, 1, 1, 1, 1, 2, 0];

const NON_BAKER_INTERMEDIATE: [u32; 9] = [0, 1, 0, 2, 1, 2, 1, 2, 1];
const NON_BAKER_TARGET: [u32; 16] = [1, 0, 1, 1, 2, 1, 0, 2, 2, 1, 0, 1, 2, 1, 0, 0];

const FACTOR_RANK_BOUND: usize = 2;
const ORIGINAL_ENTRY_CORRIDOR_LAG: usize = 2;
const DIRECT_REPLACEMENT_LAG: usize = 1;

#[derive(Debug, Serialize)]
struct Report {
    config: ProbeConfig,
    controls: Vec<ControlReport>,
}

#[derive(Debug, Serialize)]
struct ProbeConfig {
    source: DynMatrix,
    u_shape: [usize; 2],
    v_shape: [usize; 2],
    max_entry: u32,
    max_attempts_per_target: u64,
    target_controls: usize,
    method: &'static str,
}

#[derive(Debug, Serialize)]
struct ControlReport {
    label: &'static str,
    artifact_path: String,
    artifact_id: Option<String>,
    artifact_lag: Option<usize>,
    original_path_lag: usize,
    original_entry_corridor_lag: usize,
    suffix_lag_after_entry_target: usize,
    stitched_lag_if_direct_hit: usize,
    source: DynMatrix,
    intermediate: DynMatrix,
    target: DynMatrix,
    source_rank: usize,
    target_rank: usize,
    factor_rank_bound: usize,
    early_pruning: Vec<String>,
    candidate_factor_pair_attempts: u64,
    capped: bool,
    exhausted_under_bounds: bool,
    hit: bool,
    decision: &'static str,
}

struct ControlSpec {
    label: &'static str,
    artifact_path: String,
    expected_intermediate: DynMatrix,
    expected_target: DynMatrix,
}

struct Cli {
    baker_guide: String,
    non_baker_guide: String,
    max_entry: u32,
    max_attempts_per_target: u64,
}

fn main() -> Result<(), String> {
    let cli = parse_cli(env::args().skip(1))?;
    let controls = vec![
        load_control(ControlSpec {
            label: "baker",
            artifact_path: cli.baker_guide.clone(),
            expected_intermediate: DynMatrix::new(3, 3, BAKER_INTERMEDIATE.to_vec()),
            expected_target: DynMatrix::new(4, 4, BAKER_TARGET.to_vec()),
        })?,
        load_control(ControlSpec {
            label: "non_baker",
            artifact_path: cli.non_baker_guide.clone(),
            expected_intermediate: DynMatrix::new(3, 3, NON_BAKER_INTERMEDIATE.to_vec()),
            expected_target: DynMatrix::new(4, 4, NON_BAKER_TARGET.to_vec()),
        })?,
    ];

    let report = Report {
        config: ProbeConfig {
            source: DynMatrix::new(2, 2, SOURCE.to_vec()),
            u_shape: [2, 4],
            v_shape: [4, 2],
            max_entry: cli.max_entry,
            max_attempts_per_target: cli.max_attempts_per_target,
            target_controls: controls.len(),
            method: "exact rational rank precheck for VU rank <= 2; enumerate no candidate pairs when target rank is already too high",
        },
        controls,
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|err| format!("failed to serialize probe report: {err}"))?
    );
    Ok(())
}

fn load_control(spec: ControlSpec) -> Result<ControlReport, String> {
    let artifacts = load_guide_artifacts_from_path(&spec.artifact_path)?;
    if artifacts.len() != 1 {
        return Err(format!(
            "{} guide path {} yielded {} artifacts; expected exactly one",
            spec.label,
            spec.artifact_path,
            artifacts.len()
        ));
    }

    let artifact = artifacts
        .into_iter()
        .next()
        .expect("artifact length checked above");
    let GuideArtifactPayload::FullPath { path } = artifact.payload;
    if path.matrices.len() < 3 {
        return Err(format!(
            "{} guide path {} has only {} matrices; need positions 0..2",
            spec.label,
            spec.artifact_path,
            path.matrices.len()
        ));
    }
    if path.matrices.len() != path.steps.len() + 1 {
        return Err(format!(
            "{} guide path {} has {} matrices but {} steps",
            spec.label,
            spec.artifact_path,
            path.matrices.len(),
            path.steps.len()
        ));
    }

    let source = path.matrices[0].clone();
    let intermediate = path.matrices[1].clone();
    let target = path.matrices[2].clone();
    let expected_source = DynMatrix::new(2, 2, SOURCE.to_vec());

    require_matrix(
        spec.label,
        "position 0 source",
        &source,
        &expected_source,
        &spec.artifact_path,
    )?;
    require_matrix(
        spec.label,
        "position 1 intermediate",
        &intermediate,
        &spec.expected_intermediate,
        &spec.artifact_path,
    )?;
    require_matrix(
        spec.label,
        "position 2 target",
        &target,
        &spec.expected_target,
        &spec.artifact_path,
    )?;

    let original_path_lag = path.steps.len();
    if original_path_lag < ORIGINAL_ENTRY_CORRIDOR_LAG {
        return Err(format!(
            "{} guide path {} has lag {}; need at least {}",
            spec.label, spec.artifact_path, original_path_lag, ORIGINAL_ENTRY_CORRIDOR_LAG
        ));
    }
    let suffix_lag_after_entry_target = original_path_lag - ORIGINAL_ENTRY_CORRIDOR_LAG;
    let stitched_lag_if_direct_hit = DIRECT_REPLACEMENT_LAG + suffix_lag_after_entry_target;
    let source_rank = rational_rank(&source);
    let target_rank = rational_rank(&target);
    let rank_blocks_factorisation = target_rank > FACTOR_RANK_BOUND;

    Ok(ControlReport {
        label: spec.label,
        artifact_path: spec.artifact_path,
        artifact_id: artifact.artifact_id,
        artifact_lag: artifact.quality.lag,
        original_path_lag,
        original_entry_corridor_lag: ORIGINAL_ENTRY_CORRIDOR_LAG,
        suffix_lag_after_entry_target,
        stitched_lag_if_direct_hit,
        source,
        intermediate,
        target,
        source_rank,
        target_rank,
        factor_rank_bound: FACTOR_RANK_BOUND,
        early_pruning: if rank_blocks_factorisation {
            vec![format!(
                "target rank {target_rank} exceeds rank(VU) <= {FACTOR_RANK_BOUND} for V:4x2 and U:2x4"
            )]
        } else {
            vec!["rank precheck did not block; no fallback enumeration is implemented for this pinned-control probe".to_string()]
        },
        candidate_factor_pair_attempts: 0,
        capped: false,
        exhausted_under_bounds: rank_blocks_factorisation,
        hit: false,
        decision: if rank_blocks_factorisation {
            "no direct lag-1 entry replacement exists for this pinned target"
        } else {
            "inconclusive"
        },
    })
}

fn require_matrix(
    label: &str,
    role: &str,
    actual: &DynMatrix,
    expected: &DynMatrix,
    artifact_path: &str,
) -> Result<(), String> {
    if actual != expected {
        return Err(format!(
            "{label} guide path {artifact_path} has unexpected {role}: got {:?}, expected {:?}",
            actual, expected
        ));
    }
    Ok(())
}

fn rational_rank(matrix: &DynMatrix) -> usize {
    let mut work = (0..matrix.rows)
        .map(|row| {
            (0..matrix.cols)
                .map(|col| matrix.get(row, col) as i128)
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut rank = 0usize;

    for col in 0..matrix.cols {
        let Some(pivot_row) = (rank..matrix.rows).find(|&row| work[row][col] != 0) else {
            continue;
        };
        work.swap(rank, pivot_row);
        let pivot = work[rank][col];

        for row in 0..matrix.rows {
            if row == rank || work[row][col] == 0 {
                continue;
            }
            let factor = work[row][col];
            for update_col in col..matrix.cols {
                work[row][update_col] =
                    work[row][update_col] * pivot - work[rank][update_col] * factor;
            }
        }

        rank += 1;
        if rank == matrix.rows {
            break;
        }
    }

    rank
}

fn parse_cli(args: impl Iterator<Item = String>) -> Result<Cli, String> {
    let mut baker_guide = None;
    let mut non_baker_guide = None;
    let mut max_entry = 5;
    let mut max_attempts_per_target = 500_000;
    let mut args = args.peekable();

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--baker-guide" => {
                baker_guide = Some(
                    args.next()
                        .ok_or("--baker-guide requires a path".to_string())?,
                );
            }
            "--non-baker-guide" => {
                non_baker_guide = Some(
                    args.next()
                        .ok_or("--non-baker-guide requires a path".to_string())?,
                );
            }
            "--max-entry" => {
                max_entry = parse_u32_arg(&mut args, "--max-entry")?;
            }
            "--max-attempts-per-target" => {
                max_attempts_per_target = parse_u64_arg(&mut args, "--max-attempts-per-target")?;
            }
            "--help" | "-h" => {
                return Err(
                    "Usage: probe_k3_direct_entry_replacement \\\n       --baker-guide PATH --non-baker-guide PATH \\\n       [--max-entry N] [--max-attempts-per-target N]"
                        .to_string(),
                );
            }
            other => return Err(format!("unrecognized argument: {other}")),
        }
    }

    let baker_guide = baker_guide.ok_or("probe requires --baker-guide PATH".to_string())?;
    let non_baker_guide =
        non_baker_guide.ok_or("probe requires --non-baker-guide PATH".to_string())?;
    if max_attempts_per_target == 0 {
        return Err("--max-attempts-per-target must be at least 1".to_string());
    }

    Ok(Cli {
        baker_guide,
        non_baker_guide,
        max_entry,
        max_attempts_per_target,
    })
}

fn parse_u32_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<u32, String> {
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse::<u32>()
        .map_err(|err| format!("failed to parse {flag} value {value:?}: {err}"))
}

fn parse_u64_arg(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<u64, String> {
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse::<u64>()
        .map_err(|err| format!("failed to parse {flag} value {value:?}: {err}"))
}

#[cfg(test)]
mod tests {
    use super::rational_rank;
    use sse_core::matrix::DynMatrix;

    #[test]
    fn rational_rank_detects_rank_three_pinned_entry_targets() {
        let baker = DynMatrix::new(4, 4, vec![1, 2, 2, 0, 1, 0, 2, 0, 0, 1, 1, 1, 1, 1, 2, 0]);
        let non_baker = DynMatrix::new(4, 4, vec![1, 0, 1, 1, 2, 1, 0, 2, 2, 1, 0, 1, 2, 1, 0, 0]);

        assert_eq!(rational_rank(&baker), 3);
        assert_eq!(rational_rank(&non_baker), 3);
    }
}
