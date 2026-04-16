use std::fs;
use std::path::{Path, PathBuf};

use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::path_quotient::{
    analyze_guide_pool_quotient, GuidePoolQuotientAnalysis, NamedPath, PathQuotientConfig,
};
use sse_core::types::{DynSsePath, GuideArtifact, GuideArtifactPayload};

#[derive(Clone, Debug)]
struct LoadedGuide {
    label: String,
    artifact: GuideArtifact,
    canonical_matrices: Vec<DynMatrix>,
}

#[derive(Debug, serde::Serialize)]
struct RetainedGuideArtifactEnvelope {
    artifacts: Vec<GuideArtifact>,
    quotient_materialization: RetainedGuideMaterializationMetadata,
}

#[derive(Debug, serde::Serialize)]
struct RetainedGuideMaterializationMetadata {
    source_guide_artifact_paths: Vec<String>,
    selection_policy: String,
    analysis: GuidePoolQuotientAnalysis,
    retained_classes: Vec<RetainedGuideClassMaterialization>,
}

#[derive(Debug, serde::Serialize)]
struct RetainedGuideClassMaterialization {
    retained_label: String,
    retained_artifact_id: Option<String>,
    retained_lag: usize,
    canonical_lag: usize,
    source_labels: Vec<String>,
    canonical_matrices: Vec<DynMatrix>,
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    let guides = load_guides(&cli)?;
    if guides.is_empty() {
        return Err("no guide artifacts were loaded; pass --guide-artifacts".to_string());
    }
    let paths = guides
        .iter()
        .map(|guide| NamedPath {
            label: guide.label.clone(),
            matrices: guide.canonical_matrices.clone(),
        })
        .collect::<Vec<_>>();

    let analysis = analyze_guide_pool_quotient(
        &paths,
        &PathQuotientConfig {
            max_suffix_lag: cli.max_suffix_lag,
            max_rewrite_states: cli.max_rewrite_states,
            max_samples: cli.max_samples,
        },
    );

    let retained_envelope = cli
        .retained_guide_artifacts_out
        .as_ref()
        .map(|_| build_retained_guide_artifact_envelope(&cli, &guides, &analysis))
        .transpose()?;

    println!("Guide-pool quotient shrinkage");
    println!(
        "  guides: source={} unique_raw={} retained={} removed_by_dedup={} collision_groups={} raw_guides_in_collision_groups={} changed={} lag_reduced={} truncated={}",
        analysis.guide_pool.source_guides,
        analysis.guide_pool.unique_raw_guides,
        analysis.guide_pool.quotient_retained_guides,
        analysis.guide_pool.raw_guides_removed_by_dedup,
        analysis.guide_pool.canonical_collision_groups,
        analysis.guide_pool.raw_guides_in_collision_groups,
        analysis.guide_pool.guides_changed,
        analysis.guide_pool.guides_lag_reduced,
        analysis.guide_pool.exploration_truncated_guides
    );
    println!(
        "  guide size: total_lag={} -> {} retained_lag={} total_matrices={} -> {} retained_matrices={}",
        analysis.guide_pool.raw_total_lag,
        analysis.guide_pool.quotient_total_lag,
        analysis.guide_pool.quotient_retained_total_lag,
        analysis.guide_pool.raw_total_matrices,
        analysis.guide_pool.quotient_total_matrices,
        analysis.guide_pool.quotient_retained_total_matrices
    );
    println!(
        "  suffix windows: occurrences={} -> {} unique={} -> {} duplicate_occurrences={} -> {} removed={}",
        analysis.raw_window_analysis.corpus.suffix_window_occurrences,
        analysis.quotient_window_analysis.corpus.suffix_window_occurrences,
        analysis.raw_window_analysis.corpus.unique_suffix_windows,
        analysis.quotient_window_analysis.corpus.unique_suffix_windows,
        analysis
            .local_suffix_redundancy
            .raw_duplicate_suffix_window_occurrences,
        analysis
            .local_suffix_redundancy
            .quotient_duplicate_suffix_window_occurrences,
        analysis
            .local_suffix_redundancy
            .duplicate_suffix_window_occurrences_removed
    );
    println!(
        "  raw local rewrites: triangle_pairs={} triangle_two_step_windows={} commuting_square_pairs={} commuting_square_two_step_windows={}",
        analysis
            .raw_window_analysis
            .local_rewrites
            .triangle_endpoint_pairs,
        analysis
            .raw_window_analysis
            .local_rewrites
            .triangle_two_step_windows,
        analysis
            .raw_window_analysis
            .local_rewrites
            .commuting_square_endpoint_pairs,
        analysis
            .raw_window_analysis
            .local_rewrites
            .commuting_square_two_step_windows
    );

    if analysis.samples.is_empty() {
        println!("  guide samples: none");
    } else {
        println!("  guide samples:");
        for sample in &analysis.samples {
            println!(
                "    {} occurrences={} lag {} -> {} via {:?} truncated={}",
                sample.source_label,
                sample.occurrence_count,
                sample.original_lag,
                sample.canonical_lag,
                sample.rewrite_kinds,
                sample.truncated
            );
        }
    }

    if analysis.retained_guides.is_empty() {
        println!("  retained guides: none");
    } else {
        println!("  retained guides:");
        for guide in &analysis.retained_guides {
            println!(
                "    {} lag={} merged_sources={} source_labels={}",
                guide.label,
                guide.lag,
                guide.occurrence_count,
                guide.source_labels.join(", ")
            );
        }
    }

    if let Some(path) = &cli.json_out {
        ensure_parent_dir(path)?;
        let json = serde_json::to_string_pretty(&analysis)
            .map_err(|err| format!("failed to serialize analysis JSON: {err}"))?;
        fs::write(path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("  wrote {}", path.display());
    }

    if let (Some(path), Some(envelope)) = (&cli.retained_guide_artifacts_out, retained_envelope) {
        ensure_parent_dir(path)?;
        let json = serde_json::to_string_pretty(&envelope)
            .map_err(|err| format!("failed to serialize retained guide artifact JSON: {err}"))?;
        fs::write(path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!(
            "  wrote retained guide artifacts {} representative(s) to {}",
            envelope.artifacts.len(),
            path.display()
        );
    }

    Ok(())
}

#[derive(Debug)]
struct Cli {
    guide_artifact_paths: Vec<PathBuf>,
    max_suffix_lag: usize,
    max_rewrite_states: usize,
    max_samples: usize,
    json_out: Option<PathBuf>,
    retained_guide_artifacts_out: Option<PathBuf>,
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut cli = Cli {
        guide_artifact_paths: Vec::new(),
        max_suffix_lag: 4,
        max_rewrite_states: 1024,
        max_samples: 12,
        json_out: None,
        retained_guide_artifacts_out: None,
    };

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--guide-artifacts" => {
                cli.guide_artifact_paths.push(PathBuf::from(
                    args.next().ok_or("--guide-artifacts requires a path")?,
                ));
            }
            "--max-suffix-lag" => {
                cli.max_suffix_lag = args
                    .next()
                    .ok_or("--max-suffix-lag requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-suffix-lag".to_string())?;
            }
            "--max-rewrite-states" => {
                cli.max_rewrite_states = args
                    .next()
                    .ok_or("--max-rewrite-states requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-rewrite-states".to_string())?;
            }
            "--max-samples" => {
                cli.max_samples = args
                    .next()
                    .ok_or("--max-samples requires a value")?
                    .parse()
                    .map_err(|_| "invalid --max-samples".to_string())?;
            }
            "--json-out" => {
                cli.json_out = Some(PathBuf::from(
                    args.next().ok_or("--json-out requires a path")?,
                ));
            }
            "--retained-guide-artifacts-out" => {
                cli.retained_guide_artifacts_out = Some(PathBuf::from(
                    args.next()
                        .ok_or("--retained-guide-artifacts-out requires a path")?,
                ));
            }
            "--help" | "-h" => {
                return Err(
                    "usage: analyze_guide_pool_quotient [options]\n\n\
                     Options:\n\
                       --guide-artifacts PATH    load full-path guide artifact(s) from PATH (repeatable)\n\
                       --max-suffix-lag N        analyze suffix windows up to lag N (default: 4)\n\
                       --max-rewrite-states N    cap local rewrite exploration states per unique path/window (default: 1024)\n\
                       --max-samples N           cap printed/json guide samples (default: 12)\n\
                       --json-out PATH           write the full comparison as pretty JSON\n\
                       --retained-guide-artifacts-out PATH\n\
                                                 write one existing witness artifact per quotient class, plus quotient metadata\n\
                     \n\
                     If no explicit inputs are given, the tool loads\n\
                     research/guide_artifacts/k3_normalized_guide_pool.json when present."
                        .to_string(),
                );
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }

    if cli.max_suffix_lag == 0 {
        return Err("--max-suffix-lag must be at least 1".to_string());
    }
    if cli.max_rewrite_states == 0 {
        return Err("--max-rewrite-states must be at least 1".to_string());
    }

    if cli.guide_artifact_paths.is_empty() {
        let default_guides =
            PathBuf::from("research/guide_artifacts/k3_normalized_guide_pool.json");
        if default_guides.exists() {
            cli.guide_artifact_paths.push(default_guides);
        }
    }

    Ok(cli)
}

fn load_guides(cli: &Cli) -> Result<Vec<LoadedGuide>, String> {
    let mut artifacts = Vec::new();
    for path in &cli.guide_artifact_paths {
        artifacts.extend(load_guide_artifacts_from_path(path)?);
    }
    label_loaded_guides(artifacts)
}

fn label_loaded_guides(artifacts: Vec<GuideArtifact>) -> Result<Vec<LoadedGuide>, String> {
    let mut base_counts = std::collections::BTreeMap::<String, usize>::new();
    for (index, artifact) in artifacts.iter().enumerate() {
        let base = base_guide_label(index, artifact);
        *base_counts.entry(base).or_default() += 1;
    }

    let mut seen_counts = std::collections::BTreeMap::<String, usize>::new();
    let mut guides = Vec::new();
    let mut unsupported_labels = Vec::new();
    for (index, artifact) in artifacts.into_iter().enumerate() {
        let base = base_guide_label(index, &artifact);
        let seen = seen_counts.entry(base.clone()).or_default();
        *seen += 1;
        let label = if base_counts.get(&base).copied().unwrap_or(0) > 1 {
            format!("{base}#{}", *seen)
        } else {
            base
        };
        #[allow(unreachable_patterns)]
        let matrices = match &artifact.payload {
            GuideArtifactPayload::FullPath { path } => canonicalize_path(&path.matrices),
            _ => {
                unsupported_labels.push(label.clone());
                continue;
            }
        };
        guides.push(LoadedGuide {
            label,
            artifact,
            canonical_matrices: matrices,
        });
    }
    if !unsupported_labels.is_empty() {
        return Err(format!(
            "unsupported guide artifact payloads: {}",
            unsupported_labels.join(", ")
        ));
    }
    Ok(guides)
}

fn base_guide_label(index: usize, artifact: &GuideArtifact) -> String {
    artifact
        .artifact_id
        .clone()
        .or_else(|| artifact.provenance.label.clone())
        .unwrap_or_else(|| format!("guide_artifact_{index:03}"))
}

fn canonicalize_path(path: &[DynMatrix]) -> Vec<DynMatrix> {
    path.iter().map(DynMatrix::canonical_perm).collect()
}

fn build_retained_guide_artifact_envelope(
    cli: &Cli,
    guides: &[LoadedGuide],
    analysis: &GuidePoolQuotientAnalysis,
) -> Result<RetainedGuideArtifactEnvelope, String> {
    let by_label = guides
        .iter()
        .map(|guide| (guide.label.as_str(), guide))
        .collect::<std::collections::BTreeMap<_, _>>();

    let mut artifacts = Vec::new();
    let mut retained_classes = Vec::new();

    for retained in &analysis.retained_guides {
        let mut candidates = retained
            .source_labels
            .iter()
            .map(|label| {
                by_label
                    .get(label.as_str())
                    .copied()
                    .ok_or_else(|| format!("missing source artifact for retained label {label}"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        candidates.sort_by(|left, right| compare_loaded_guides_for_retention(left, right));
        let selected = candidates
            .into_iter()
            .next()
            .expect("retained guide should have at least one source candidate");

        artifacts.push(selected.artifact.clone());
        retained_classes.push(RetainedGuideClassMaterialization {
            retained_label: selected.label.clone(),
            retained_artifact_id: selected.artifact.artifact_id.clone(),
            retained_lag: loaded_guide_effective_lag(selected),
            canonical_lag: retained.lag,
            source_labels: retained.source_labels.clone(),
            canonical_matrices: retained.matrices.clone(),
        });
    }

    artifacts.sort_by(compare_artifacts_for_output);
    retained_classes.sort_by(|left, right| {
        left.canonical_lag
            .cmp(&right.canonical_lag)
            .then(left.retained_lag.cmp(&right.retained_lag))
            .then(left.retained_label.cmp(&right.retained_label))
    });

    Ok(RetainedGuideArtifactEnvelope {
        artifacts,
        quotient_materialization: RetainedGuideMaterializationMetadata {
            source_guide_artifact_paths: cli
                .guide_artifact_paths
                .iter()
                .map(|path| path.display().to_string())
                .collect(),
            selection_policy: "select the shortest existing witness artifact in each quotient class; ties break by artifact_id/provenance label".to_string(),
            analysis: analysis.clone(),
            retained_classes,
        },
    })
}

fn compare_loaded_guides_for_retention(
    left: &LoadedGuide,
    right: &LoadedGuide,
) -> std::cmp::Ordering {
    loaded_guide_effective_lag(left)
        .cmp(&loaded_guide_effective_lag(right))
        .then(left_path_matrix_count(left).cmp(&left_path_matrix_count(right)))
        .then(artifact_sort_key(&left.artifact).cmp(&artifact_sort_key(&right.artifact)))
        .then(left.label.cmp(&right.label))
}

fn compare_artifacts_for_output(left: &GuideArtifact, right: &GuideArtifact) -> std::cmp::Ordering {
    artifact_effective_lag(left)
        .cmp(&artifact_effective_lag(right))
        .then(
            left_path_matrix_count_from_artifact(left)
                .cmp(&left_path_matrix_count_from_artifact(right)),
        )
        .then(artifact_sort_key(left).cmp(&artifact_sort_key(right)))
}

fn loaded_guide_effective_lag(guide: &LoadedGuide) -> usize {
    artifact_effective_lag(&guide.artifact)
}

fn artifact_effective_lag(artifact: &GuideArtifact) -> usize {
    artifact
        .quality
        .lag
        .unwrap_or_else(|| artifact_path(artifact).steps.len())
}

fn left_path_matrix_count(guide: &LoadedGuide) -> usize {
    artifact_path(&guide.artifact).matrices.len()
}

fn left_path_matrix_count_from_artifact(artifact: &GuideArtifact) -> usize {
    artifact_path(artifact).matrices.len()
}

fn artifact_sort_key(artifact: &GuideArtifact) -> (&str, &str, &str) {
    (
        artifact.artifact_id.as_deref().unwrap_or(""),
        artifact.provenance.source_ref.as_deref().unwrap_or(""),
        artifact.provenance.label.as_deref().unwrap_or(""),
    )
}

fn artifact_path(artifact: &GuideArtifact) -> &DynSsePath {
    let GuideArtifactPayload::FullPath { path } = &artifact.payload;
    path
}

fn ensure_parent_dir(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        artifact_effective_lag, artifact_path, base_guide_label,
        build_retained_guide_artifact_envelope, canonicalize_path, label_loaded_guides, Cli,
    };
    use sse_core::matrix::DynMatrix;
    use sse_core::path_quotient::{analyze_guide_pool_quotient, NamedPath, PathQuotientConfig};
    use sse_core::types::{
        DynSsePath, GuideArtifact, GuideArtifactCompatibility, GuideArtifactEndpoints,
        GuideArtifactPayload, GuideArtifactProvenance, GuideArtifactQuality,
        GuideArtifactValidation, SearchStage,
    };
    use std::path::PathBuf;

    #[test]
    fn retained_guide_materialization_chooses_shortest_witness_per_quotient_class() {
        let artifacts = vec![
            fixture_artifact("direct", vec![matrix(1), matrix(3)], 1),
            fixture_artifact("two-hop", vec![matrix(1), matrix(2), matrix(3)], 2),
            fixture_artifact("other", vec![matrix(4), matrix(5)], 1),
        ];
        let guides = label_loaded_guides(artifacts).unwrap();
        let paths = guides
            .iter()
            .map(|guide| NamedPath {
                label: guide.label.clone(),
                matrices: guide.canonical_matrices.clone(),
            })
            .collect::<Vec<_>>();
        let analysis = analyze_guide_pool_quotient(
            &paths,
            &PathQuotientConfig {
                max_suffix_lag: 3,
                max_rewrite_states: 32,
                max_samples: 8,
            },
        );

        let cli = Cli {
            guide_artifact_paths: vec![PathBuf::from("input.json")],
            max_suffix_lag: 3,
            max_rewrite_states: 32,
            max_samples: 8,
            json_out: None,
            retained_guide_artifacts_out: Some(PathBuf::from("retained.json")),
        };
        let envelope = build_retained_guide_artifact_envelope(&cli, &guides, &analysis).unwrap();

        let retained_ids = envelope
            .artifacts
            .iter()
            .map(|artifact| artifact.artifact_id.as_deref().unwrap_or(""))
            .collect::<Vec<_>>();
        assert_eq!(retained_ids, vec!["direct", "other"]);
        assert_eq!(envelope.quotient_materialization.retained_classes.len(), 2);
        assert_eq!(
            envelope.quotient_materialization.retained_classes[0].source_labels,
            vec!["direct".to_string(), "two-hop".to_string()]
        );
    }

    #[test]
    fn label_loaded_guides_disambiguates_duplicate_labels() {
        let guides = label_loaded_guides(vec![
            fixture_artifact_with_label(None, Some("dup"), vec![matrix(1), matrix(2)], 1),
            fixture_artifact_with_label(None, Some("dup"), vec![matrix(2), matrix(3)], 1),
        ])
        .unwrap();

        assert_eq!(guides[0].label, "dup#1");
        assert_eq!(guides[1].label, "dup#2");
    }

    #[test]
    fn helpers_preserve_existing_effective_lag_and_canonicalize() {
        let artifact = fixture_artifact("lagged", vec![matrix(7), matrix(8), matrix(9)], 2);
        assert_eq!(base_guide_label(0, &artifact), "lagged");
        assert_eq!(artifact_effective_lag(&artifact), 2);
        assert_eq!(
            canonicalize_path(&artifact_path(&artifact).matrices),
            artifact_path(&artifact)
                .matrices
                .iter()
                .map(DynMatrix::canonical_perm)
                .collect::<Vec<_>>()
        );
    }

    fn fixture_artifact(id: &str, matrices: Vec<DynMatrix>, lag: usize) -> GuideArtifact {
        fixture_artifact_with_label(Some(id), Some(id), matrices, lag)
    }

    fn fixture_artifact_with_label(
        artifact_id: Option<&str>,
        provenance_label: Option<&str>,
        matrices: Vec<DynMatrix>,
        lag: usize,
    ) -> GuideArtifact {
        GuideArtifact {
            artifact_id: artifact_id.map(str::to_string),
            endpoints: GuideArtifactEndpoints {
                source: matrices.first().cloned().unwrap(),
                target: matrices.last().cloned().unwrap(),
            },
            payload: GuideArtifactPayload::FullPath {
                path: DynSsePath {
                    matrices,
                    steps: vec![],
                },
            },
            provenance: GuideArtifactProvenance {
                source_kind: Some("test".to_string()),
                label: provenance_label.map(str::to_string),
                source_ref: Some("test:fixture".to_string()),
            },
            validation: GuideArtifactValidation::Unchecked,
            compatibility: GuideArtifactCompatibility {
                supported_stages: vec![SearchStage::ShortcutSearch],
                max_endpoint_dim: None,
            },
            quality: GuideArtifactQuality {
                lag: Some(lag),
                cost: Some(lag),
                score: None,
            },
        }
    }

    fn matrix(value: u32) -> DynMatrix {
        DynMatrix::new(1, 1, vec![value])
    }
}
