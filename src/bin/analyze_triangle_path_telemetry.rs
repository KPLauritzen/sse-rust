use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::path_quotient::{analyze_path_quotient, NamedPath, PathQuotientConfig};
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
        return Err("no path sources were loaded; pass --guide-artifacts or --path-db".to_string());
    }

    let analysis = analyze_path_quotient(
        &paths,
        &PathQuotientConfig {
            max_suffix_lag: cli.max_suffix_lag,
            max_rewrite_states: cli.max_rewrite_states,
            max_samples: cli.max_samples,
        },
    );

    println!("Triangle path-quotient telemetry");
    println!(
        "  source_paths={} suffix_window_occurrences={} unique_suffix_windows={} max_suffix_lag={} max_rewrite_states={}",
        analysis.corpus.source_paths,
        analysis.corpus.suffix_window_occurrences,
        analysis.corpus.unique_suffix_windows,
        analysis.config.max_suffix_lag,
        analysis.config.max_rewrite_states
    );
    println!(
        "  state-collision analogue: terminal_groups={} endpoint_groups={}",
        analysis.corpus.terminal_state_collision_groups, analysis.corpus.endpoint_collision_groups
    );
    println!(
        "  local rewrites: triangle_pairs={} triangle_two_step_windows={} commuting_square_pairs={} commuting_square_two_step_windows={}",
        analysis.local_rewrites.triangle_endpoint_pairs,
        analysis.local_rewrites.triangle_two_step_windows,
        analysis.local_rewrites.commuting_square_endpoint_pairs,
        analysis.local_rewrites.commuting_square_two_step_windows
    );
    println!(
        "  endpoint groups explained by local rewrites: {} with, {} without",
        analysis
            .local_rewrites
            .endpoint_collision_groups_with_local_rewrites,
        analysis
            .local_rewrites
            .endpoint_collision_groups_without_local_rewrites
    );
    println!(
        "  canonicalization: collapsed_occurrences={} lag_reduced_occurrences={} triangle_touched={} square_touched={} truncated_occurrences={} unique_windows={} -> {}",
        analysis.canonicalization.collapsed_window_occurrences,
        analysis.canonicalization.lag_reduced_window_occurrences,
        analysis
            .canonicalization
            .triangle_rewritten_window_occurrences,
        analysis
            .canonicalization
            .commuting_square_rewritten_window_occurrences,
        analysis
            .canonicalization
            .exploration_truncated_window_occurrences,
        analysis.canonicalization.unique_raw_windows,
        analysis.canonicalization.unique_canonical_windows
    );

    if analysis.samples.is_empty() {
        println!("  samples: none");
    } else {
        println!("  samples:");
        for sample in &analysis.samples {
            println!(
                "    {} [{}..{}] occurrences={} lag {} -> {} via {:?}",
                sample.source_label,
                sample.start_index,
                sample.end_index,
                sample.occurrence_count,
                sample.original_lag,
                sample.canonical_lag,
                sample.rewrite_kinds
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
    path_dbs: Vec<PathBuf>,
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
        path_dbs: Vec::new(),
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
            "--path-db" => {
                cli.path_dbs.push(PathBuf::from(
                    args.next().ok_or("--path-db requires a path")?,
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
                    "usage: analyze_triangle_path_telemetry [options]\n\n\
                     Options:\n\
                       --guide-artifacts PATH    load full-path guide artifact(s) from PATH (repeatable)\n\
                       --path-db PATH            load legacy sqlite path corpus from PATH (repeatable)\n\
                       --max-suffix-lag N        analyze suffix windows up to lag N (default: 4)\n\
                       --max-rewrite-states N    cap local rewrite exploration states per unique window (default: 1024)\n\
                       --max-samples N           cap printed/json collapse samples (default: 12)\n\
                       --json-out PATH           write the full analysis as pretty JSON\n\
                     \n\
                     If no explicit inputs are given, the tool loads any existing default\n\
                     sources from research/k3-graph-paths.sqlite and\n\
                     research/guide_artifacts/k3_normalized_guide_pool.json."
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

    if cli.guide_artifact_paths.is_empty() && cli.path_dbs.is_empty() {
        let default_db = PathBuf::from("research/k3-graph-paths.sqlite");
        if default_db.exists() {
            cli.path_dbs.push(default_db);
        }
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
    for path in &cli.path_dbs {
        paths.extend(load_paths_from_sqlite(path)?);
    }

    let mut seen = BTreeSet::new();
    paths.retain(|path| seen.insert((path.label.clone(), path.matrices.clone())));
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

fn load_paths_from_sqlite(path: &Path) -> Result<Vec<NamedPath>, String> {
    let conn = Connection::open(path)
        .map_err(|err| format!("failed to open {}: {err}", path.display()))?;
    let mut stmt = conn
        .prepare(
            "SELECT r.id, r.guide_label, m.step_index, x.rows, x.cols, x.data_json
             FROM shortcut_path_results r
             JOIN shortcut_path_matrices m ON m.result_id = r.id
             JOIN matrices x ON x.id = m.matrix_id
             ORDER BY r.id, m.step_index",
        )
        .map_err(|err| format!("failed to query {}: {err}", path.display()))?;
    let mut rows = stmt
        .query([])
        .map_err(|err| format!("failed to iterate {}: {err}", path.display()))?;

    let mut paths = Vec::new();
    let mut current_id = None;
    let mut current_label = String::new();
    let mut current_matrices = Vec::new();

    while let Some(row) = rows
        .next()
        .map_err(|err| format!("failed to read sqlite row from {}: {err}", path.display()))?
    {
        let result_id: i64 = row.get(0).map_err(sqlite_value_err)?;
        let guide_label: String = row.get(1).map_err(sqlite_value_err)?;
        let rows_count: i64 = row.get(3).map_err(sqlite_value_err)?;
        let cols_count: i64 = row.get(4).map_err(sqlite_value_err)?;
        let data_json: String = row.get(5).map_err(sqlite_value_err)?;
        let rows = serde_json::from_str::<Vec<Vec<u32>>>(&data_json)
            .map_err(|err| format!("failed to parse matrix JSON in {}: {err}", path.display()))?;
        let data = rows.into_iter().flatten().collect::<Vec<_>>();
        let matrix =
            DynMatrix::new(rows_count as usize, cols_count as usize, data).canonical_perm();

        if current_id != Some(result_id) {
            if let Some(previous_id) = current_id {
                paths.push(NamedPath {
                    label: format!("sqlite:{previous_id}:{current_label}"),
                    matrices: std::mem::take(&mut current_matrices),
                });
            }
            current_id = Some(result_id);
            current_label = guide_label;
        }
        current_matrices.push(matrix);
    }

    if let Some(result_id) = current_id {
        paths.push(NamedPath {
            label: format!("sqlite:{result_id}:{current_label}"),
            matrices: current_matrices,
        });
    }

    Ok(paths)
}

fn canonicalize_path(path: &[DynMatrix]) -> Vec<DynMatrix> {
    path.iter().map(DynMatrix::canonical_perm).collect()
}

fn sqlite_value_err(err: rusqlite::Error) -> String {
    err.to_string()
}
