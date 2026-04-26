use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::types::{GuideArtifact, GuideArtifactPayload};

const DEFAULT_ENDPOINT_RADIUS: usize = 3;
const DEFAULT_TOP_STUCK: usize = 8;

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
    let mut samples = Vec::new();

    for guide_artifact in &cli.guide_artifacts {
        let guide_tag = guide_artifact
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("guide");
        let guide_identity = normalized_path_identity(guide_artifact);
        let guide_display = guide_artifact.display().to_string();
        for (artifact_index, artifact) in load_guide_artifacts_from_path(guide_artifact)?
            .into_iter()
            .enumerate()
        {
            samples.extend(extract_endpoint_samples(
                &artifact,
                guide_tag,
                &guide_identity,
                &guide_display,
                artifact_index,
                cli.endpoint_radius,
            ));
        }
    }

    if let Some(stuck_report) = &cli.stuck_report {
        let report = read_json::<StuckStateReport>(stuck_report)?;
        samples.extend(extract_stuck_samples(&report, cli.top_stuck));
    }

    if samples.is_empty() {
        return Err("no samples found".to_string());
    }

    let parity_pair_reports = build_parity_pair_reports(&samples);
    let report = EndpointNeighborhoodReport {
        endpoint_radius: cli.endpoint_radius,
        top_stuck: cli.top_stuck,
        guide_artifacts: cli
            .guide_artifacts
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        stuck_report: cli.stuck_report.map(|path| path.display().to_string()),
        sample_summary: build_sample_summary(&samples),
        parity_pair_summary: build_parity_pair_summary(&parity_pair_reports),
        parity_pair_reports,
        candidate_results: vec![
            analyze_candidate(
                "mass_support_signature",
                "dim + entry sum + sorted row/col sums + sorted row/col supports",
                &samples,
                mass_support_signature,
            ),
            analyze_candidate(
                "trimmed_active_window",
                "canonical square state, then trim zero rows/cols and keep the active rectangular block exactly",
                &samples,
                trimmed_active_window_signature,
            ),
            analyze_candidate(
                "trimmed_entry_bag_signature",
                "trimmed active window shape + sorted row/col sums/supports + sorted positive-entry multiset",
                &samples,
                trimmed_entry_bag_signature,
            ),
        ],
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
    guide_artifacts: Vec<PathBuf>,
    stuck_report: Option<PathBuf>,
    endpoint_radius: usize,
    top_stuck: usize,
    json_out: Option<PathBuf>,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut guide_artifacts = Vec::new();
    let mut stuck_report = None;
    let mut endpoint_radius = DEFAULT_ENDPOINT_RADIUS;
    let mut top_stuck = DEFAULT_TOP_STUCK;
    let mut json_out = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--guide-artifact" => guide_artifacts.push(PathBuf::from(
                args.next().ok_or("--guide-artifact requires a path")?,
            )),
            "--stuck-report" => {
                stuck_report = Some(PathBuf::from(
                    args.next().ok_or("--stuck-report requires a path")?,
                ));
            }
            "--endpoint-radius" => {
                endpoint_radius = args
                    .next()
                    .ok_or("--endpoint-radius requires a value")?
                    .parse()
                    .map_err(|_| "invalid --endpoint-radius".to_string())?;
            }
            "--top-stuck" => {
                top_stuck = args
                    .next()
                    .ok_or("--top-stuck requires a value")?
                    .parse()
                    .map_err(|_| "invalid --top-stuck".to_string())?;
            }
            "--json-out" => {
                json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if guide_artifacts.is_empty() && stuck_report.is_none() {
        return Err("provide at least one --guide-artifact or --stuck-report".to_string());
    }

    Ok(Cli {
        guide_artifacts,
        stuck_report,
        endpoint_radius,
        top_stuck,
        json_out,
    })
}

fn usage() -> String {
    format!(
        "usage: diagnose_endpoint_neighborhood_normal_forms [--guide-artifact PATH ...] [--stuck-report PATH] [--endpoint-radius N] [--top-stuck N] [--json-out PATH]\n\
defaults: endpoint-radius={DEFAULT_ENDPOINT_RADIUS}, top-stuck={DEFAULT_TOP_STUCK}"
    )
}

fn read_json<T>(path: &PathBuf) -> Result<T, String>
where
    T: for<'de> Deserialize<'de>,
{
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("failed to read {}: {err}", path.display()))?;
    serde_json::from_str(&raw).map_err(|err| format!("failed to parse {}: {err}", path.display()))
}

#[derive(Deserialize)]
struct StuckStateReport {
    ranked_approximate_hits: Vec<ApproximateHit>,
}

#[derive(Deserialize)]
struct ApproximateHit {
    rank: usize,
    move_family: String,
    to_matrix: DynMatrix,
    counterpart_matrix: Option<DynMatrix>,
}

#[derive(Clone)]
struct SampleState {
    label: String,
    sample_kind: String,
    dim: usize,
    endpoint_side: String,
    matrix: DynMatrix,
    origin: SampleOrigin,
}

#[derive(Clone)]
enum SampleOrigin {
    Witness {
        guide_identity: String,
        artifact_identity: String,
        artifact_index: usize,
        step_index: usize,
    },
    Stuck {
        hit_index: usize,
        rank: usize,
        move_family: String,
        role: StuckRole,
    },
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum StuckRole {
    Frontier,
    Counterpart,
}

fn extract_endpoint_samples(
    artifact: &GuideArtifact,
    guide_tag: &str,
    guide_identity: &str,
    guide_display: &str,
    artifact_index: usize,
    endpoint_radius: usize,
) -> Vec<SampleState> {
    let GuideArtifactPayload::FullPath { path } = &artifact.payload;
    let last = path.matrices.len().saturating_sub(1);
    path.matrices
        .iter()
        .enumerate()
        .filter(|(_, matrix)| matrix.rows == matrix.cols && matches!(matrix.rows, 3 | 4))
        .filter_map(|(idx, matrix)| {
            let depth_from_start = idx;
            let depth_from_end = last.saturating_sub(idx);
            let near_start = depth_from_start <= endpoint_radius;
            let near_end = depth_from_end <= endpoint_radius;
            if !near_start && !near_end {
                return None;
            }
            let artifact_identity = artifact
                .artifact_id
                .clone()
                .unwrap_or_else(|| format!("{guide_identity}#artifact{}", artifact_index));
            let artifact_id = artifact
                .artifact_id
                .clone()
                .unwrap_or_else(|| fallback_artifact_id(guide_tag, guide_identity, artifact_index));
            Some(SampleState {
                label: format!(
                    "{guide_display}:artifact{}:{}:step{}",
                    artifact_index, artifact_id, idx
                ),
                sample_kind: format!("k3_witness:{guide_tag}"),
                dim: matrix.rows,
                endpoint_side: match (near_start, near_end) {
                    (true, true) => "both".to_string(),
                    (true, false) => "source".to_string(),
                    (false, true) => "target".to_string(),
                    (false, false) => unreachable!(),
                },
                matrix: matrix.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: guide_identity.to_string(),
                    artifact_identity,
                    artifact_index,
                    step_index: idx,
                },
            })
        })
        .collect()
}

fn extract_stuck_samples(report: &StuckStateReport, top_stuck: usize) -> Vec<SampleState> {
    if top_stuck == 0 {
        return Vec::new();
    }
    let mut samples = Vec::new();
    for (hit_index, hit) in report.ranked_approximate_hits.iter().enumerate() {
        if hit.to_matrix.rows != hit.to_matrix.cols || !matches!(hit.to_matrix.rows, 3 | 4) {
            continue;
        }
        if !hit.counterpart_matrix.as_ref().is_some_and(|counterpart| {
            counterpart.rows == counterpart.cols
                && counterpart.rows == hit.to_matrix.rows
                && counterpart.cols == hit.to_matrix.cols
                && matches!(counterpart.rows, 3 | 4)
        }) {
            continue;
        }
        samples.push(SampleState {
            label: format!("k4_stuck_rank{}_to", hit.rank),
            sample_kind: format!("k4_stuck:{}", hit.move_family),
            dim: hit.to_matrix.rows,
            endpoint_side: "frontier".to_string(),
            matrix: hit.to_matrix.clone(),
            origin: SampleOrigin::Stuck {
                hit_index,
                rank: hit.rank,
                move_family: hit.move_family.clone(),
                role: StuckRole::Frontier,
            },
        });
        if let Some(counterpart) = &hit.counterpart_matrix {
            samples.push(SampleState {
                label: format!("k4_stuck_rank{}_counterpart", hit.rank),
                sample_kind: format!("k4_counterpart:{}", hit.move_family),
                dim: counterpart.rows,
                endpoint_side: "opposite_frontier".to_string(),
                matrix: counterpart.clone(),
                origin: SampleOrigin::Stuck {
                    hit_index,
                    rank: hit.rank,
                    move_family: hit.move_family.clone(),
                    role: StuckRole::Counterpart,
                },
            });
        }
        if samples.len() / 2 >= top_stuck {
            break;
        }
    }
    samples
}

#[derive(Serialize)]
struct EndpointNeighborhoodReport {
    endpoint_radius: usize,
    top_stuck: usize,
    guide_artifacts: Vec<String>,
    stuck_report: Option<String>,
    sample_summary: SampleSummary,
    parity_pair_summary: ParityPairSummary,
    parity_pair_reports: Vec<ParityPairReport>,
    candidate_results: Vec<CandidateReport>,
}

#[derive(Serialize)]
struct SampleSummary {
    total_samples: usize,
    by_kind: BTreeMap<String, usize>,
    by_dim: BTreeMap<usize, usize>,
}

#[derive(Serialize)]
struct ParityPairSummary {
    total_pairs: usize,
    by_kind: BTreeMap<String, usize>,
    by_signal: BTreeMap<String, usize>,
    by_action: BTreeMap<String, usize>,
}

#[derive(Serialize)]
struct ParityPairReport {
    pair_id: String,
    pair_kind: String,
    dimension: usize,
    endpoint_context: String,
    coarse_signature_match: bool,
    trimmed_active_window_match: bool,
    parity_signal: String,
    recommended_action: String,
    left: ParitySampleReport,
    right: ParitySampleReport,
}

#[derive(Serialize)]
struct ParitySampleReport {
    label: String,
    sample_kind: String,
    endpoint_side: String,
    matrix: DynMatrix,
    canonical_matrix: DynMatrix,
    coarse_signature: String,
    trimmed_active_window: DynMatrix,
    trimmed_active_window_signature: String,
}

fn build_sample_summary(samples: &[SampleState]) -> SampleSummary {
    let mut by_kind = BTreeMap::new();
    let mut by_dim = BTreeMap::new();
    for sample in samples {
        *by_kind.entry(sample.sample_kind.clone()).or_insert(0) += 1;
        *by_dim.entry(sample.dim).or_insert(0) += 1;
    }
    SampleSummary {
        total_samples: samples.len(),
        by_kind,
        by_dim,
    }
}

fn build_parity_pair_reports(samples: &[SampleState]) -> Vec<ParityPairReport> {
    let mut reports = build_witness_parity_pairs(samples);
    reports.extend(build_stuck_parity_pairs(samples));
    reports.sort_by(|left, right| left.pair_id.cmp(&right.pair_id));
    reports
}

fn build_witness_parity_pairs(samples: &[SampleState]) -> Vec<ParityPairReport> {
    let mut unique_samples =
        BTreeMap::<(String, String, usize, usize, String, usize), &SampleState>::new();
    for sample in samples {
        if let SampleOrigin::Witness {
            guide_identity,
            artifact_identity,
            artifact_index,
            step_index,
            ..
        } = &sample.origin
        {
            unique_samples
                .entry((
                    guide_identity.clone(),
                    artifact_identity.clone(),
                    *artifact_index,
                    *step_index,
                    sample.endpoint_side.clone(),
                    sample.dim,
                ))
                .or_insert(sample);
        }
    }

    let mut by_artifact_step_side_dim =
        BTreeMap::<(String, usize, String, usize), Vec<&SampleState>>::new();
    for ((_, artifact_identity, _, step_index, endpoint_side, dim), sample) in unique_samples {
        by_artifact_step_side_dim
            .entry((artifact_identity, step_index, endpoint_side, dim))
            .or_default()
            .push(sample);
    }

    let mut reports = Vec::new();
    for ((artifact_identity, step_index, endpoint_side, dim), grouped_samples) in
        by_artifact_step_side_dim
    {
        if grouped_samples.len() < 2 {
            continue;
        }
        for left_index in 0..grouped_samples.len() {
            for right_index in (left_index + 1)..grouped_samples.len() {
                let left = grouped_samples[left_index];
                let right = grouped_samples[right_index];
                if witness_guide_identity(left) == witness_guide_identity(right) {
                    continue;
                }
                reports.push(build_pair_report(
                    pair_id_for_samples(
                        &format!(
                            "k3_witness_art{}_step{}_{}_{}x{}",
                            stable_hash_hex(&artifact_identity),
                            step_index,
                            endpoint_side,
                            dim,
                            dim
                        ),
                        left,
                        right,
                    ),
                    "k3_witness_replay_overlap".to_string(),
                    format!("step {} / {}", step_index, endpoint_side),
                    left,
                    right,
                ));
            }
        }
    }
    reports
}

fn build_stuck_parity_pairs(samples: &[SampleState]) -> Vec<ParityPairReport> {
    let mut pairs = BTreeMap::<usize, (Option<&SampleState>, Option<&SampleState>)>::new();
    for sample in samples {
        let SampleOrigin::Stuck {
            hit_index, role, ..
        } = &sample.origin
        else {
            continue;
        };
        let entry = pairs.entry(*hit_index).or_default();
        match role {
            StuckRole::Frontier => {
                entry.0 = Some(sample);
            }
            StuckRole::Counterpart => {
                entry.1 = Some(sample);
            }
        }
    }

    let mut reports = Vec::new();
    for (hit_index, (left, right)) in pairs {
        let (Some(left), Some(right)) = (left, right) else {
            continue;
        };
        let SampleOrigin::Stuck {
            rank, move_family, ..
        } = &left.origin
        else {
            continue;
        };
        reports.push(build_pair_report(
            pair_id_for_samples(
                &format!("k4_stuck_hit{}_rank{}_{}", hit_index, rank, move_family),
                left,
                right,
            ),
            "k4_stuck_vs_counterpart".to_string(),
            format!("rank {} / {}", rank, move_family),
            left,
            right,
        ));
    }
    reports
}

fn build_pair_report(
    pair_id: String,
    pair_kind: String,
    endpoint_context: String,
    left: &SampleState,
    right: &SampleState,
) -> ParityPairReport {
    let left_report = build_parity_sample_report(left);
    let right_report = build_parity_sample_report(right);
    let coarse_signature_match = left_report.coarse_signature == right_report.coarse_signature;
    let trimmed_active_window_match =
        left_report.trimmed_active_window_signature == right_report.trimmed_active_window_signature;
    let (parity_signal, recommended_action) =
        classify_parity_signal(coarse_signature_match, trimmed_active_window_match);

    ParityPairReport {
        pair_id,
        pair_kind,
        dimension: left.dim,
        endpoint_context,
        coarse_signature_match,
        trimmed_active_window_match,
        parity_signal: parity_signal.to_string(),
        recommended_action: recommended_action.to_string(),
        left: left_report,
        right: right_report,
    }
}

fn build_parity_sample_report(sample: &SampleState) -> ParitySampleReport {
    let canonical_matrix = sample.matrix.canonical_perm();
    let trimmed_active_window = trim_zero_rows_and_cols(&canonical_matrix);
    ParitySampleReport {
        label: sample.label.clone(),
        sample_kind: sample.sample_kind.clone(),
        endpoint_side: sample.endpoint_side.clone(),
        matrix: sample.matrix.clone(),
        canonical_matrix,
        coarse_signature: mass_support_signature(&sample.matrix),
        trimmed_active_window_signature: format!(
            "{}x{}|{}",
            trimmed_active_window.rows,
            trimmed_active_window.cols,
            join_u32(&trimmed_active_window.data)
        ),
        trimmed_active_window,
    }
}

fn build_parity_pair_summary(reports: &[ParityPairReport]) -> ParityPairSummary {
    let mut by_kind = BTreeMap::new();
    let mut by_signal = BTreeMap::new();
    let mut by_action = BTreeMap::new();
    for report in reports {
        *by_kind.entry(report.pair_kind.clone()).or_insert(0) += 1;
        *by_signal.entry(report.parity_signal.clone()).or_insert(0) += 1;
        *by_action
            .entry(report.recommended_action.clone())
            .or_insert(0) += 1;
    }

    ParityPairSummary {
        total_pairs: reports.len(),
        by_kind,
        by_signal,
        by_action,
    }
}

fn classify_parity_signal(
    coarse_signature_match: bool,
    trimmed_active_window_match: bool,
) -> (&'static str, &'static str) {
    match (coarse_signature_match, trimmed_active_window_match) {
        (true, true) => ("exact_trimmed_window_match", "reuse_endpoint_local_parity"),
        (true, false) => (
            "coarse_only_layout_mismatch",
            "rank_or_propose_inside_coarse_bucket",
        ),
        (false, true) => ("trimmed_match_without_coarse_match", "diagnose_only"),
        (false, false) => ("no_endpoint_local_overlap", "ignore"),
    }
}

fn witness_guide_identity(sample: &SampleState) -> &str {
    match &sample.origin {
        SampleOrigin::Witness { guide_identity, .. } => guide_identity,
        SampleOrigin::Stuck { .. } => "",
    }
}

fn pair_id_for_samples(prefix: &str, left: &SampleState, right: &SampleState) -> String {
    let mut member_ids = [sample_identity_token(left), sample_identity_token(right)];
    member_ids.sort_unstable();
    format!("{prefix}:{}:{}", member_ids[0], member_ids[1])
}

fn normalized_path_identity(path: &PathBuf) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.clone())
        .display()
        .to_string()
}

fn sample_identity_token(sample: &SampleState) -> String {
    match &sample.origin {
        SampleOrigin::Witness {
            guide_identity,
            artifact_identity,
            artifact_index,
            step_index,
            ..
        } => format!(
            "w{}i{}a{}s{}e{}d{}",
            stable_hash_hex(guide_identity),
            stable_hash_hex(artifact_identity),
            artifact_index,
            step_index,
            stable_hash_hex(&sample.endpoint_side),
            sample.dim
        ),
        SampleOrigin::Stuck {
            hit_index,
            rank,
            move_family,
            role,
        } => format!(
            "h{}r{}f{}{}e{}d{}",
            hit_index,
            rank,
            stable_hash_hex(move_family),
            match role {
                StuckRole::Frontier => "f",
                StuckRole::Counterpart => "c",
            },
            stable_hash_hex(&sample.endpoint_side),
            sample.dim
        ),
    }
}

fn stable_hash_hex(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[derive(Serialize)]
struct CandidateReport {
    name: &'static str,
    description: &'static str,
    unique_forms: usize,
    collision_bucket_count: usize,
    largest_bucket_size: usize,
    collision_buckets: Vec<FormBucket>,
}

#[derive(Serialize)]
struct FormBucket {
    key: String,
    sample_labels: Vec<String>,
    sample_kinds: Vec<String>,
}

fn analyze_candidate<F>(
    name: &'static str,
    description: &'static str,
    samples: &[SampleState],
    mut key_fn: F,
) -> CandidateReport
where
    F: FnMut(&DynMatrix) -> String,
{
    let mut buckets = BTreeMap::<String, Vec<&SampleState>>::new();
    for sample in samples {
        buckets
            .entry(key_fn(&sample.matrix))
            .or_default()
            .push(sample);
    }

    let collision_buckets = buckets
        .iter()
        .filter(|(_, samples)| samples.len() > 1)
        .map(|(key, samples)| FormBucket {
            key: key.clone(),
            sample_labels: samples.iter().map(|sample| sample.label.clone()).collect(),
            sample_kinds: samples
                .iter()
                .map(|sample| sample.sample_kind.clone())
                .collect(),
        })
        .collect::<Vec<_>>();

    CandidateReport {
        name,
        description,
        unique_forms: buckets.len(),
        collision_bucket_count: collision_buckets.len(),
        largest_bucket_size: buckets
            .values()
            .map(|samples| samples.len())
            .max()
            .unwrap_or(0),
        collision_buckets,
    }
}

fn mass_support_signature(matrix: &DynMatrix) -> String {
    let mut row_sums = vec![0u64; matrix.rows];
    let mut col_sums = vec![0u64; matrix.cols];
    let mut row_supports = vec![0u8; matrix.rows];
    let mut col_supports = vec![0u8; matrix.cols];
    let mut entry_sum = 0u64;

    for row in 0..matrix.rows {
        for col in 0..matrix.cols {
            let value = matrix.get(row, col);
            row_sums[row] += value as u64;
            col_sums[col] += value as u64;
            entry_sum += value as u64;
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

    format!(
        "d{}|sum{}|rs{}|cs{}|rS{}|cS{}",
        matrix.rows,
        entry_sum,
        join_u64(&row_sums),
        join_u64(&col_sums),
        join_u8(&row_supports),
        join_u8(&col_supports),
    )
}

fn trimmed_active_window_signature(matrix: &DynMatrix) -> String {
    let trimmed = trimmed_active_window(matrix);
    format!(
        "{}x{}|{}",
        trimmed.rows,
        trimmed.cols,
        join_u32(&trimmed.data)
    )
}

fn trimmed_entry_bag_signature(matrix: &DynMatrix) -> String {
    let trimmed = trimmed_active_window(matrix);
    let mut row_sums = vec![0u64; trimmed.rows];
    let mut col_sums = vec![0u64; trimmed.cols];
    let mut row_supports = vec![0u8; trimmed.rows];
    let mut col_supports = vec![0u8; trimmed.cols];
    let mut positive_entries = Vec::new();

    for row in 0..trimmed.rows {
        for col in 0..trimmed.cols {
            let value = trimmed.get(row, col);
            row_sums[row] += value as u64;
            col_sums[col] += value as u64;
            if value != 0 {
                row_supports[row] += 1;
                col_supports[col] += 1;
                positive_entries.push(value);
            }
        }
    }

    row_sums.sort_unstable();
    col_sums.sort_unstable();
    row_supports.sort_unstable();
    col_supports.sort_unstable();
    positive_entries.sort_unstable();

    format!(
        "{}x{}|rs{}|cs{}|rS{}|cS{}|bag{}",
        trimmed.rows,
        trimmed.cols,
        join_u64(&row_sums),
        join_u64(&col_sums),
        join_u8(&row_supports),
        join_u8(&col_supports),
        join_u32(&positive_entries),
    )
}

fn trimmed_active_window(matrix: &DynMatrix) -> DynMatrix {
    let canonical = matrix.canonical_perm();
    trim_zero_rows_and_cols(&canonical)
}

fn trim_zero_rows_and_cols(matrix: &DynMatrix) -> DynMatrix {
    let active_rows = (0..matrix.rows)
        .filter(|&row| (0..matrix.cols).any(|col| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();
    let active_cols = (0..matrix.cols)
        .filter(|&col| (0..matrix.rows).any(|row| matrix.get(row, col) != 0))
        .collect::<Vec<_>>();

    let mut data = Vec::with_capacity(active_rows.len() * active_cols.len());
    for &row in &active_rows {
        for &col in &active_cols {
            data.push(matrix.get(row, col));
        }
    }

    DynMatrix::new(active_rows.len(), active_cols.len(), data)
}

fn fallback_artifact_id(guide_tag: &str, guide_label: &str, artifact_index: usize) -> String {
    format!("{guide_tag}@{guide_label}#{}", artifact_index)
}

fn join_u8(values: &[u8]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn join_u32(values: &[u32]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

fn join_u64(values: &[u64]) -> String {
    values
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sse_core::search::build_full_path_guide_artifact;
    use sse_core::types::DynSsePath;

    fn rank4_to_matrix() -> DynMatrix {
        DynMatrix::new(4, 4, vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    fn rank4_counterpart_matrix() -> DynMatrix {
        DynMatrix::new(4, 4, vec![1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    #[test]
    fn mass_support_signature_collapses_rank4_pair() {
        assert_eq!(
            mass_support_signature(&rank4_to_matrix()),
            mass_support_signature(&rank4_counterpart_matrix())
        );
    }

    #[test]
    fn trimmed_active_window_separates_rank4_pair() {
        assert_ne!(
            trimmed_active_window_signature(&rank4_to_matrix()),
            trimmed_active_window_signature(&rank4_counterpart_matrix())
        );
    }

    #[test]
    fn trimmed_entry_bag_signature_separates_rank4_pair() {
        assert_ne!(
            trimmed_entry_bag_signature(&rank4_to_matrix()),
            trimmed_entry_bag_signature(&rank4_counterpart_matrix())
        );
    }

    fn guide_artifact_fixture(
        artifact_id: Option<&str>,
        matrices: Vec<DynMatrix>,
    ) -> GuideArtifact {
        let source = matrices
            .first()
            .cloned()
            .expect("fixture needs at least one matrix");
        let target = matrices
            .last()
            .cloned()
            .expect("fixture needs at least one matrix");
        let path = DynSsePath {
            matrices,
            steps: Vec::new(),
        };
        let mut artifact =
            build_full_path_guide_artifact(&source, &target, &path).expect("fixture should build");
        artifact.artifact_id = artifact_id.map(str::to_string);
        artifact
    }

    #[test]
    fn extract_endpoint_samples_uses_guide_specific_fallback_artifact_id() {
        let artifact = guide_artifact_fixture(
            None,
            vec![DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0])],
        );

        let samples = extract_endpoint_samples(
            &artifact,
            "guide_alpha",
            "fixtures/a.json",
            "fixtures/a.json",
            2,
            3,
        );

        assert_eq!(samples.len(), 1);
        assert_eq!(
            samples[0].label,
            "fixtures/a.json:artifact2:guide_alpha@fixtures/a.json#2:step0"
        );
    }

    #[test]
    fn extract_endpoint_samples_uses_present_artifact_id() {
        let artifact = guide_artifact_fixture(
            Some("demo"),
            vec![DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0])],
        );

        let samples = extract_endpoint_samples(
            &artifact,
            "guide_alpha",
            "fixtures/a.json",
            "fixtures/a.json",
            0,
            3,
        );

        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].label, "fixtures/a.json:artifact0:demo:step0");
    }

    #[test]
    fn extract_stuck_samples_counts_eligible_hits_before_stopping() {
        let report = StuckStateReport {
            ranked_approximate_hits: vec![
                ApproximateHit {
                    rank: 1,
                    move_family: "skip".to_string(),
                    to_matrix: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
                    counterpart_matrix: Some(DynMatrix::new(2, 2, vec![1, 0, 0, 1])),
                },
                ApproximateHit {
                    rank: 2,
                    move_family: "keep".to_string(),
                    to_matrix: DynMatrix::new(
                        4,
                        4,
                        vec![0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0],
                    ),
                    counterpart_matrix: Some(DynMatrix::new(
                        4,
                        4,
                        vec![0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0],
                    )),
                },
                ApproximateHit {
                    rank: 3,
                    move_family: "keep".to_string(),
                    to_matrix: DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0]),
                    counterpart_matrix: Some(DynMatrix::new(3, 3, vec![0, 0, 1, 1, 0, 1, 0, 1, 0])),
                },
            ],
        };

        let samples = extract_stuck_samples(&report, 2);

        assert_eq!(samples.len(), 4);
        assert_eq!(samples[0].label, "k4_stuck_rank2_to");
        assert_eq!(samples[2].label, "k4_stuck_rank3_to");
    }

    #[test]
    fn extract_stuck_samples_respects_zero_top_stuck() {
        let report = StuckStateReport {
            ranked_approximate_hits: vec![ApproximateHit {
                rank: 2,
                move_family: "keep".to_string(),
                to_matrix: DynMatrix::new(
                    4,
                    4,
                    vec![0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0],
                ),
                counterpart_matrix: Some(DynMatrix::new(
                    4,
                    4,
                    vec![0, 0, 1, 0, 0, 1, 0, 1, 1, 0, 0, 1, 0, 1, 0, 0],
                )),
            }],
        };

        let samples = extract_stuck_samples(&report, 0);

        assert!(samples.is_empty());
    }

    #[test]
    fn extract_stuck_samples_skips_mixed_dimension_counterparts() {
        let report = StuckStateReport {
            ranked_approximate_hits: vec![ApproximateHit {
                rank: 2,
                move_family: "keep".to_string(),
                to_matrix: DynMatrix::new(
                    4,
                    4,
                    vec![0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0, 1, 0, 0, 1, 0],
                ),
                counterpart_matrix: Some(DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0])),
            }],
        };

        let samples = extract_stuck_samples(&report, 8);

        assert!(samples.is_empty());
    }

    #[test]
    fn normalized_path_identity_collapses_relative_aliases() {
        let direct = normalized_path_identity(&PathBuf::from("Cargo.toml"));
        let dotted = normalized_path_identity(&PathBuf::from("./Cargo.toml"));

        assert_eq!(direct, dotted);
    }

    #[test]
    fn witness_pair_report_marks_exact_trimmed_window_match() {
        let shared = DynMatrix::new(3, 3, vec![1, 0, 1, 0, 1, 0, 1, 0, 0]);
        let left = SampleState {
            label: "left".to_string(),
            sample_kind: "k3_witness:left".to_string(),
            dim: 3,
            endpoint_side: "source".to_string(),
            matrix: shared.clone(),
            origin: SampleOrigin::Witness {
                guide_identity: "left/path.json".to_string(),
                artifact_identity: "demo".to_string(),
                artifact_index: 0,
                step_index: 1,
            },
        };
        let right = SampleState {
            label: "right".to_string(),
            sample_kind: "k3_witness:right".to_string(),
            dim: 3,
            endpoint_side: "source".to_string(),
            matrix: shared,
            origin: SampleOrigin::Witness {
                guide_identity: "right/path.json".to_string(),
                artifact_identity: "demo".to_string(),
                artifact_index: 0,
                step_index: 1,
            },
        };

        let report = build_pair_report(
            "pair".to_string(),
            "k3_witness_replay_overlap".to_string(),
            "step 1 / source".to_string(),
            &left,
            &right,
        );

        assert_eq!(report.parity_signal, "exact_trimmed_window_match");
        assert_eq!(report.recommended_action, "reuse_endpoint_local_parity");
    }

    #[test]
    fn stuck_pair_report_marks_coarse_only_layout_mismatch() {
        let left = SampleState {
            label: "rank4_to".to_string(),
            sample_kind: "k4_stuck:diag".to_string(),
            dim: 4,
            endpoint_side: "frontier".to_string(),
            matrix: rank4_to_matrix(),
            origin: SampleOrigin::Stuck {
                hit_index: 0,
                rank: 4,
                move_family: "diag".to_string(),
                role: StuckRole::Frontier,
            },
        };
        let right = SampleState {
            label: "rank4_counterpart".to_string(),
            sample_kind: "k4_counterpart:diag".to_string(),
            dim: 4,
            endpoint_side: "opposite_frontier".to_string(),
            matrix: rank4_counterpart_matrix(),
            origin: SampleOrigin::Stuck {
                hit_index: 0,
                rank: 4,
                move_family: "diag".to_string(),
                role: StuckRole::Counterpart,
            },
        };

        let report = build_pair_report(
            "pair".to_string(),
            "k4_stuck_vs_counterpart".to_string(),
            "rank 4 / diag".to_string(),
            &left,
            &right,
        );

        assert!(report.coarse_signature_match);
        assert!(!report.trimmed_active_window_match);
        assert_eq!(report.parity_signal, "coarse_only_layout_mismatch");
        assert_eq!(
            report.recommended_action,
            "rank_or_propose_inside_coarse_bucket"
        );
    }

    #[test]
    fn witness_pair_builder_emits_all_cross_guide_pairs_with_unique_ids() {
        let shared = DynMatrix::new(3, 3, vec![1, 0, 1, 0, 1, 0, 1, 0, 0]);
        let samples = vec![
            SampleState {
                label: "a".to_string(),
                sample_kind: "k3_witness:a".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_a.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "b".to_string(),
                sample_kind: "k3_witness:b".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_b.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "c".to_string(),
                sample_kind: "k3_witness:c".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared,
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_c.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
        ];

        let reports = build_witness_parity_pairs(&samples);
        let ids = reports
            .iter()
            .map(|report| report.pair_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(reports.len(), 3);
        assert_eq!(ids.len(), 3);
    }

    #[test]
    fn witness_pair_builder_allows_distinct_guides_with_same_basename() {
        let shared = DynMatrix::new(3, 3, vec![1, 0, 1, 0, 1, 0, 1, 0, 0]);
        let samples = vec![
            SampleState {
                label: "dir_a/shared.json:demo:step2".to_string(),
                sample_kind: "k3_witness:shared".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "dir_a/shared.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "dir_b/shared.json:demo:step2".to_string(),
                sample_kind: "k3_witness:shared".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared,
                origin: SampleOrigin::Witness {
                    guide_identity: "dir_b/shared.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
        ];

        let reports = build_witness_parity_pairs(&samples);

        assert_eq!(reports.len(), 1);
        assert_eq!(reports[0].parity_signal, "exact_trimmed_window_match");
    }

    #[test]
    fn witness_pair_builder_does_not_cross_pair_different_artifacts() {
        let shared = DynMatrix::new(3, 3, vec![1, 0, 1, 0, 1, 0, 1, 0, 0]);
        let samples = vec![
            SampleState {
                label: "guide_a.json:alpha:step2".to_string(),
                sample_kind: "k3_witness:a".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_a.json".to_string(),
                    artifact_identity: "alpha".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "guide_a.json:beta:step2".to_string(),
                sample_kind: "k3_witness:a".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_a.json".to_string(),
                    artifact_identity: "beta".to_string(),
                    artifact_index: 1,
                    step_index: 2,
                },
            },
            SampleState {
                label: "guide_b.json:alpha:step2".to_string(),
                sample_kind: "k3_witness:b".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_b.json".to_string(),
                    artifact_identity: "alpha".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "guide_b.json:beta:step2".to_string(),
                sample_kind: "k3_witness:b".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared,
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_b.json".to_string(),
                    artifact_identity: "beta".to_string(),
                    artifact_index: 1,
                    step_index: 2,
                },
            },
        ];

        let reports = build_witness_parity_pairs(&samples);

        assert_eq!(reports.len(), 2);
        assert!(reports
            .iter()
            .all(|report| report.parity_signal == "exact_trimmed_window_match"));
    }

    #[test]
    fn witness_pair_builder_keeps_duplicate_artifact_ids_separate_by_index() {
        let shared = DynMatrix::new(3, 3, vec![1, 0, 1, 0, 1, 0, 1, 0, 0]);
        let samples = vec![
            SampleState {
                label: "guide_a.json:artifact0:dup:step2".to_string(),
                sample_kind: "k3_witness:a".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_a.json".to_string(),
                    artifact_identity: "dup".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "guide_a.json:artifact1:dup:step2".to_string(),
                sample_kind: "k3_witness:a".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_a.json".to_string(),
                    artifact_identity: "dup".to_string(),
                    artifact_index: 1,
                    step_index: 2,
                },
            },
            SampleState {
                label: "guide_b.json:artifact0:dup:step2".to_string(),
                sample_kind: "k3_witness:b".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared,
                origin: SampleOrigin::Witness {
                    guide_identity: "guide_b.json".to_string(),
                    artifact_identity: "dup".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
        ];

        let reports = build_witness_parity_pairs(&samples);

        assert_eq!(reports.len(), 2);
    }

    #[test]
    fn witness_pair_builder_pair_ids_survive_old_slug_collision() {
        let shared = DynMatrix::new(3, 3, vec![1, 0, 1, 0, 1, 0, 1, 0, 0]);
        let samples = vec![
            SampleState {
                label: "dir/a.json:demo:step2".to_string(),
                sample_kind: "k3_witness:a".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "dir/a.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "dir_a.json:demo:step2".to_string(),
                sample_kind: "k3_witness:b".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared.clone(),
                origin: SampleOrigin::Witness {
                    guide_identity: "dir_a.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
            SampleState {
                label: "peer.json:demo:step2".to_string(),
                sample_kind: "k3_witness:c".to_string(),
                dim: 3,
                endpoint_side: "source".to_string(),
                matrix: shared,
                origin: SampleOrigin::Witness {
                    guide_identity: "peer.json".to_string(),
                    artifact_identity: "demo".to_string(),
                    artifact_index: 0,
                    step_index: 2,
                },
            },
        ];

        let reports = build_witness_parity_pairs(&samples);
        let ids = reports
            .iter()
            .map(|report| report.pair_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(reports.len(), 3);
        assert_eq!(ids.len(), 3);
        let collision_reports = reports
            .iter()
            .filter(|report| {
                report.left.label == "peer.json:demo:step2"
                    || report.right.label == "peer.json:demo:step2"
            })
            .collect::<Vec<_>>();
        assert_eq!(collision_reports.len(), 2);
        assert_ne!(collision_reports[0].pair_id, collision_reports[1].pair_id);
    }

    #[test]
    fn stuck_pair_builder_keys_by_per_hit_identity() {
        let samples = vec![
            SampleState {
                label: "rank4_diag_to".to_string(),
                sample_kind: "k4_stuck:diag".to_string(),
                dim: 4,
                endpoint_side: "frontier".to_string(),
                matrix: rank4_to_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 0,
                    rank: 4,
                    move_family: "diag".to_string(),
                    role: StuckRole::Frontier,
                },
            },
            SampleState {
                label: "rank4_diag_counterpart".to_string(),
                sample_kind: "k4_counterpart:diag".to_string(),
                dim: 4,
                endpoint_side: "opposite_frontier".to_string(),
                matrix: rank4_counterpart_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 0,
                    rank: 4,
                    move_family: "diag".to_string(),
                    role: StuckRole::Counterpart,
                },
            },
            SampleState {
                label: "rank4_conj_to".to_string(),
                sample_kind: "k4_stuck:conj".to_string(),
                dim: 4,
                endpoint_side: "frontier".to_string(),
                matrix: rank4_to_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 1,
                    rank: 4,
                    move_family: "conj".to_string(),
                    role: StuckRole::Frontier,
                },
            },
            SampleState {
                label: "rank4_conj_counterpart".to_string(),
                sample_kind: "k4_counterpart:conj".to_string(),
                dim: 4,
                endpoint_side: "opposite_frontier".to_string(),
                matrix: rank4_counterpart_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 1,
                    rank: 4,
                    move_family: "conj".to_string(),
                    role: StuckRole::Counterpart,
                },
            },
        ];

        let reports = build_stuck_parity_pairs(&samples);
        let ids = reports
            .iter()
            .map(|report| report.pair_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(reports.len(), 2);
        assert!(ids
            .iter()
            .any(|id| id.starts_with("k4_stuck_hit1_rank4_conj:")));
        assert!(ids
            .iter()
            .any(|id| id.starts_with("k4_stuck_hit0_rank4_diag:")));
    }

    #[test]
    fn stuck_pair_builder_avoids_cartesian_pairing_for_shared_rank_and_family() {
        let samples = vec![
            SampleState {
                label: "hit0_to".to_string(),
                sample_kind: "k4_stuck:diag".to_string(),
                dim: 4,
                endpoint_side: "frontier".to_string(),
                matrix: rank4_to_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 0,
                    rank: 4,
                    move_family: "diag".to_string(),
                    role: StuckRole::Frontier,
                },
            },
            SampleState {
                label: "hit0_counterpart".to_string(),
                sample_kind: "k4_counterpart:diag".to_string(),
                dim: 4,
                endpoint_side: "opposite_frontier".to_string(),
                matrix: rank4_counterpart_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 0,
                    rank: 4,
                    move_family: "diag".to_string(),
                    role: StuckRole::Counterpart,
                },
            },
            SampleState {
                label: "hit1_to".to_string(),
                sample_kind: "k4_stuck:diag".to_string(),
                dim: 4,
                endpoint_side: "frontier".to_string(),
                matrix: rank4_to_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 1,
                    rank: 4,
                    move_family: "diag".to_string(),
                    role: StuckRole::Frontier,
                },
            },
            SampleState {
                label: "hit1_counterpart".to_string(),
                sample_kind: "k4_counterpart:diag".to_string(),
                dim: 4,
                endpoint_side: "opposite_frontier".to_string(),
                matrix: rank4_counterpart_matrix(),
                origin: SampleOrigin::Stuck {
                    hit_index: 1,
                    rank: 4,
                    move_family: "diag".to_string(),
                    role: StuckRole::Counterpart,
                },
            },
        ];

        let reports = build_stuck_parity_pairs(&samples);
        let ids = reports
            .iter()
            .map(|report| report.pair_id.as_str())
            .collect::<std::collections::BTreeSet<_>>();

        assert_eq!(reports.len(), 2);
        assert!(ids
            .iter()
            .any(|id| id.starts_with("k4_stuck_hit0_rank4_diag:")));
        assert!(ids
            .iter()
            .any(|id| id.starts_with("k4_stuck_hit1_rank4_diag:")));
    }
}
