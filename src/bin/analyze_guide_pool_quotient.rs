use std::fs;
use std::path::{Path, PathBuf};

use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::path_quotient::{analyze_guide_pool_quotient, NamedPath, PathQuotientConfig};
use sse_core::types::GuideArtifactPayload;

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let cli = parse_cli(std::env::args().skip(1))?;
    let paths = load_paths(&cli)?;
    if paths.is_empty() {
        return Err("no guide artifacts were loaded; pass --guide-artifacts".to_string());
    }

    let analysis = analyze_guide_pool_quotient(
        &paths,
        &PathQuotientConfig {
            max_suffix_lag: cli.max_suffix_lag,
            max_rewrite_states: cli.max_rewrite_states,
            max_samples: cli.max_samples,
        },
    );

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
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                fs::create_dir_all(parent)
                    .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
            }
        }
        let json = serde_json::to_string_pretty(&analysis)
            .map_err(|err| format!("failed to serialize analysis JSON: {err}"))?;
        fs::write(path, format!("{json}\n"))
            .map_err(|err| format!("failed to write {}: {err}", path.display()))?;
        println!("  wrote {}", path.display());
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
            "--help" | "-h" => {
                return Err(
                    "usage: analyze_guide_pool_quotient [options]\n\n\
                     Options:\n\
                       --guide-artifacts PATH    load full-path guide artifact(s) from PATH (repeatable)\n\
                       --max-suffix-lag N        analyze suffix windows up to lag N (default: 4)\n\
                       --max-rewrite-states N    cap local rewrite exploration states per unique path/window (default: 1024)\n\
                       --max-samples N           cap printed/json guide samples (default: 12)\n\
                       --json-out PATH           write the full comparison as pretty JSON\n\
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

fn load_paths(cli: &Cli) -> Result<Vec<NamedPath>, String> {
    let mut paths = Vec::new();
    for path in &cli.guide_artifact_paths {
        paths.extend(load_paths_from_guide_artifacts(path)?);
    }
    Ok(paths)
}

fn load_paths_from_guide_artifacts(path: &Path) -> Result<Vec<NamedPath>, String> {
    let mut paths = Vec::new();
    let mut unsupported_labels = Vec::new();
    for artifact in load_guide_artifacts_from_path(path)? {
        let label = artifact
            .artifact_id
            .or(artifact.provenance.label)
            .unwrap_or_else(|| "guide_artifact_path".to_string());
        #[allow(unreachable_patterns)]
        let matrices = match artifact.payload {
            GuideArtifactPayload::FullPath { path } => canonicalize_path(&path.matrices),
            _ => {
                unsupported_labels.push(label);
                continue;
            }
        };
        paths.push(NamedPath { label, matrices });
    }
    if !unsupported_labels.is_empty() {
        return Err(format!(
            "unsupported guide artifact payloads in {}: {}",
            path.display(),
            unsupported_labels.join(", ")
        ));
    }
    Ok(paths)
}

fn canonicalize_path(path: &[DynMatrix]) -> Vec<DynMatrix> {
    path.iter().map(DynMatrix::canonical_perm).collect()
}
