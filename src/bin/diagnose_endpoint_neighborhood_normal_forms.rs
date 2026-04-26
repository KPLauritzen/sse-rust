use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use sse_core::matrix::DynMatrix;

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
        let artifact = read_json::<GuideArtifact>(guide_artifact)?;
        samples.extend(extract_endpoint_samples(
            &artifact,
            guide_artifact
                .file_stem()
                .and_then(|stem| stem.to_str())
                .unwrap_or("guide"),
            cli.endpoint_radius,
        ));
    }

    if let Some(stuck_report) = &cli.stuck_report {
        let report = read_json::<StuckStateReport>(stuck_report)?;
        samples.extend(extract_stuck_samples(&report, cli.top_stuck));
    }

    if samples.is_empty() {
        return Err("no samples found".to_string());
    }

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
struct GuideArtifact {
    #[serde(default)]
    artifact_id: Option<String>,
    path: GuidePath,
}

#[derive(Deserialize)]
struct GuidePath {
    matrices: Vec<DynMatrix>,
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

#[derive(Clone, Serialize)]
struct SampleState {
    label: String,
    sample_kind: String,
    dim: usize,
    endpoint_side: String,
    matrix: DynMatrix,
}

fn extract_endpoint_samples(
    artifact: &GuideArtifact,
    guide_tag: &str,
    endpoint_radius: usize,
) -> Vec<SampleState> {
    let last = artifact.path.matrices.len().saturating_sub(1);
    artifact
        .path
        .matrices
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
            let artifact_id = artifact
                .artifact_id
                .as_deref()
                .unwrap_or_else(|| fallback_artifact_id(guide_tag));
            Some(SampleState {
                label: format!("{guide_tag}:{artifact_id}:step{}", idx),
                sample_kind: format!("k3_witness:{guide_tag}"),
                dim: matrix.rows,
                endpoint_side: match (near_start, near_end) {
                    (true, true) => "both".to_string(),
                    (true, false) => "source".to_string(),
                    (false, true) => "target".to_string(),
                    (false, false) => unreachable!(),
                },
                matrix: matrix.clone(),
            })
        })
        .collect()
}

fn extract_stuck_samples(report: &StuckStateReport, top_stuck: usize) -> Vec<SampleState> {
    if top_stuck == 0 {
        return Vec::new();
    }
    let mut samples = Vec::new();
    for hit in report.ranked_approximate_hits.iter().filter(|hit| {
        hit.to_matrix.rows == hit.to_matrix.cols
            && matches!(hit.to_matrix.rows, 3 | 4)
            && hit.counterpart_matrix.as_ref().is_some_and(|counterpart| {
                counterpart.rows == counterpart.cols && matches!(counterpart.rows, 3 | 4)
            })
    }) {
        samples.push(SampleState {
            label: format!("k4_stuck_rank{}_to", hit.rank),
            sample_kind: format!("k4_stuck:{}", hit.move_family),
            dim: hit.to_matrix.rows,
            endpoint_side: "frontier".to_string(),
            matrix: hit.to_matrix.clone(),
        });
        if let Some(counterpart) = &hit.counterpart_matrix {
            samples.push(SampleState {
                label: format!("k4_stuck_rank{}_counterpart", hit.rank),
                sample_kind: format!("k4_counterpart:{}", hit.move_family),
                dim: counterpart.rows,
                endpoint_side: "opposite_frontier".to_string(),
                matrix: counterpart.clone(),
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
    candidate_results: Vec<CandidateReport>,
}

#[derive(Serialize)]
struct SampleSummary {
    total_samples: usize,
    by_kind: BTreeMap<String, usize>,
    by_dim: BTreeMap<usize, usize>,
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
    let active_rows = (0..canonical.rows)
        .filter(|&row| (0..canonical.cols).any(|col| canonical.get(row, col) != 0))
        .collect::<Vec<_>>();
    let active_cols = (0..canonical.cols)
        .filter(|&col| (0..canonical.rows).any(|row| canonical.get(row, col) != 0))
        .collect::<Vec<_>>();

    let mut data = Vec::with_capacity(active_rows.len() * active_cols.len());
    for &row in &active_rows {
        for &col in &active_cols {
            data.push(canonical.get(row, col));
        }
    }

    DynMatrix::new(active_rows.len(), active_cols.len(), data)
}

fn fallback_artifact_id(guide_tag: &str) -> &str {
    guide_tag
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

    #[test]
    fn guide_artifact_deserializes_with_present_artifact_id() {
        let artifact = serde_json::from_str::<GuideArtifact>(
            r#"{
                "artifact_id": "demo",
                "path": {
                    "matrices": [
                        {"rows": 3, "cols": 3, "data": [0,1,0,1,0,1,0,1,0]}
                    ]
                }
            }"#,
        )
        .expect("artifact with id should deserialize");

        assert_eq!(artifact.artifact_id.as_deref(), Some("demo"));
        assert_eq!(artifact.path.matrices.len(), 1);
    }

    #[test]
    fn guide_artifact_deserializes_with_missing_artifact_id() {
        let artifact = serde_json::from_str::<GuideArtifact>(
            r#"{
                "path": {
                    "matrices": [
                        {"rows": 4, "cols": 4, "data": [0,1,0,0,1,0,1,0,0,1,0,1,0,0,1,0]}
                    ]
                }
            }"#,
        )
        .expect("artifact without id should deserialize");

        assert_eq!(artifact.artifact_id, None);
        assert_eq!(artifact.path.matrices.len(), 1);
    }

    #[test]
    fn extract_endpoint_samples_uses_guide_specific_fallback_artifact_id() {
        let artifact = GuideArtifact {
            artifact_id: None,
            path: GuidePath {
                matrices: vec![DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0])],
            },
        };

        let samples = extract_endpoint_samples(&artifact, "guide_alpha", 3);

        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].label, "guide_alpha:guide_alpha:step0");
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
}
