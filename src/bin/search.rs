use std::process::ExitCode;
use std::{fs, path::Path};

use sse_core::matrix::DynMatrix;
use sse_core::search::{
    build_full_path_guide_artifact, execute_search_request, execute_search_request_and_observer,
};
#[cfg(not(target_arch = "wasm32"))]
use sse_core::sqlite_graph::SqliteGraphRecorder;
use sse_core::types::{
    GuideArtifact, GuideArtifactCompatibility, GuideArtifactProvenance, GuidedRefinementConfig,
    SearchConfig, SearchMode, SearchRequest, SearchRunResult, SearchStage, SearchTelemetry,
};

#[derive(Debug)]
struct Cli {
    a: DynMatrix,
    b: DynMatrix,
    config: SearchConfig,
    stage: SearchStage,
    guide_artifact_paths: Vec<String>,
    guided_refinement: GuidedRefinementConfig,
    json: bool,
    telemetry: bool,
    visited_db: Option<String>,
    write_guide_artifact: Option<String>,
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(msg) => {
            eprintln!("error: {msg}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<ExitCode, String> {
    run_with_args(std::env::args().skip(1))
}

fn run_with_args<I>(args: I) -> Result<ExitCode, String>
where
    I: Iterator<Item = String>,
{
    let cli = parse_cli(args)?;
    let mut guide_artifacts = Vec::new();
    for path in &cli.guide_artifact_paths {
        guide_artifacts.extend(load_guide_artifacts(path)?);
    }
    if cli.stage == SearchStage::GuidedRefinement && guide_artifacts.is_empty() {
        return Err("guided_refinement requires at least one --guide-artifacts file".to_string());
    }
    let request = SearchRequest {
        source: cli.a.clone(),
        target: cli.b.clone(),
        config: cli.config.clone(),
        stage: cli.stage,
        guide_artifacts,
        guided_refinement: cli.guided_refinement.clone(),
    };

    if let Some(path) = cli.visited_db.as_deref() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut recorder = SqliteGraphRecorder::new(path)?;
            let (result, telemetry) =
                execute_search_request_and_observer(&request, Some(&mut recorder))?;
            if let Some(err) = recorder.error() {
                return Err(format!("failed to persist visited graph to {path}: {err}"));
            }
            maybe_write_guide_artifact(
                &request,
                cli.stage,
                &result,
                cli.write_guide_artifact.as_deref(),
            )?;
            if cli.json {
                print_json(
                    &cli.a,
                    &cli.b,
                    cli.stage,
                    &result,
                    &telemetry,
                    cli.telemetry,
                );
            } else {
                print_pretty(
                    &cli.a,
                    &cli.b,
                    cli.stage,
                    &result,
                    &telemetry,
                    cli.telemetry,
                );
            }
            return Ok(exit_code(&result));
        }
        #[cfg(target_arch = "wasm32")]
        {
            return Err("--visited-db is not supported on wasm32 targets".to_string());
        }
    }

    let (result, telemetry) = execute_search_request(&request)?;
    maybe_write_guide_artifact(
        &request,
        cli.stage,
        &result,
        cli.write_guide_artifact.as_deref(),
    )?;
    if cli.json {
        print_json(
            &cli.a,
            &cli.b,
            cli.stage,
            &result,
            &telemetry,
            cli.telemetry,
        );
    } else {
        print_pretty(
            &cli.a,
            &cli.b,
            cli.stage,
            &result,
            &telemetry,
            cli.telemetry,
        );
    }
    Ok(exit_code(&result))
}

fn exit_code(result: &SearchRunResult) -> ExitCode {
    match result {
        SearchRunResult::Equivalent(_) | SearchRunResult::EquivalentByConcreteShift(_) => {
            ExitCode::SUCCESS
        }
        SearchRunResult::NotEquivalent(_) => ExitCode::from(1),
        SearchRunResult::Unknown => ExitCode::from(3),
    }
}

fn parse_cli<I>(mut args: I) -> Result<Cli, String>
where
    I: Iterator<Item = String>,
{
    let mut a: Option<DynMatrix> = None;
    let mut b: Option<DynMatrix> = None;
    let mut config = SearchConfig::default();
    let mut stage = SearchStage::EndpointSearch;
    let mut guide_artifact_paths = Vec::new();
    let mut guided_refinement = GuidedRefinementConfig::default();
    let mut json = false;
    let mut telemetry = false;
    let mut visited_db = None;
    let mut write_guide_artifact = None;

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--help" | "-h" => {
                return Err("usage: search <A> <B> [options]\n\n\
                     Matrices are given either as 4 comma-separated 2x2 entries,\n\
                     or as NxN-prefixed row-major data:\n\
                     \n\
                       search 1,2,3,4 5,6,7,8\n\
                       search 3x3:0,1,0,1,0,1,0,1,0 4x4:...\n\
                     \n\
                     Options:\n\
                       --max-lag N              max elementary SSE steps (default: 4)\n\
                       --max-intermediate-dim N max intermediate dimension (default: 2)\n\
                       --max-entry N            max entry value in U,V (default: 25)\n\
                       --search-mode MODE       mixed | graph-only (default: mixed)\n\
                       --stage STAGE            endpoint-search | guided-refinement (default: endpoint-search)\n\
                       --guide-artifacts PATH   read JSON guide artifact(s) from PATH (repeatable)\n\
                       --guided-max-shortcut-lag N max lag for one guided shortcut search (default: 3)\n\
                       --guided-min-gap N       minimum guide gap to consider for refinement (default: 2)\n\
                       --guided-max-gap N       maximum guide gap to consider for refinement\n\
                       --guided-segment-timeout SECS\n\
                                               max wall-clock seconds for one guided segment search\n\
                       --guided-rounds N        number of refinement rounds per guide (default: 1)\n\
                       --visited-db PATH        write visited nodes and SSE edges to a sqlite db\n\
                       --write-guide-artifact PATH\n\
                                               write a reusable full_path guide artifact JSON file\n\
                       --json                   output JSON instead of human-readable text\n\
                       --telemetry              include search telemetry in output"
                    .to_string());
            }
            "--max-lag" => {
                config.max_lag = next_parsed(&mut args, "--max-lag")?;
            }
            "--max-intermediate-dim" => {
                config.max_intermediate_dim = next_parsed(&mut args, "--max-intermediate-dim")?;
            }
            "--max-entry" => {
                config.max_entry = next_parsed(&mut args, "--max-entry")?;
            }
            "--search-mode" => {
                let value = args.next().ok_or("--search-mode requires a value")?;
                config.search_mode = match value.as_str() {
                    "mixed" => SearchMode::Mixed,
                    "graph-only" | "graph_only" => SearchMode::GraphOnly,
                    _ => return Err(format!("unknown search mode: {value}")),
                };
            }
            "--stage" => {
                let value = args.next().ok_or("--stage requires a value")?;
                stage = match value.as_str() {
                    "endpoint-search" | "endpoint_search" => SearchStage::EndpointSearch,
                    "guided-refinement" | "guided_refinement" => SearchStage::GuidedRefinement,
                    "shortcut-search" | "shortcut_search" => SearchStage::ShortcutSearch,
                    _ => return Err(format!("unknown stage: {value}")),
                };
            }
            "--guide-artifacts" => {
                guide_artifact_paths.push(args.next().ok_or("--guide-artifacts requires a path")?);
            }
            "--guided-max-shortcut-lag" => {
                guided_refinement.max_shortcut_lag =
                    next_parsed(&mut args, "--guided-max-shortcut-lag")?;
            }
            "--guided-min-gap" => {
                guided_refinement.min_gap = next_parsed(&mut args, "--guided-min-gap")?;
            }
            "--guided-max-gap" => {
                guided_refinement.max_gap = Some(next_parsed(&mut args, "--guided-max-gap")?);
            }
            "--guided-segment-timeout" => {
                guided_refinement.segment_timeout_secs =
                    Some(next_parsed(&mut args, "--guided-segment-timeout")?);
            }
            "--guided-rounds" => {
                guided_refinement.rounds = next_parsed(&mut args, "--guided-rounds")?;
            }
            "--visited-db" => {
                visited_db = Some(args.next().ok_or("--visited-db requires a path")?);
            }
            "--write-guide-artifact" => {
                write_guide_artifact = Some(
                    args.next()
                        .ok_or("--write-guide-artifact requires a path")?,
                );
            }
            "--json" => json = true,
            "--telemetry" => telemetry = true,
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            positional => {
                let mat = parse_matrix(positional)?;
                if a.is_none() {
                    a = Some(mat);
                } else if b.is_none() {
                    b = Some(mat);
                } else {
                    return Err(
                        "too many positional arguments (expected exactly 2 matrices)".to_string(),
                    );
                }
            }
        }
    }

    let a = a.ok_or("missing matrix A (first positional argument)")?;
    let b = b.ok_or("missing matrix B (second positional argument)")?;

    Ok(Cli {
        a,
        b,
        config,
        stage,
        guide_artifact_paths,
        guided_refinement,
        json,
        telemetry,
        visited_db,
        write_guide_artifact,
    })
}

fn maybe_write_guide_artifact(
    request: &SearchRequest,
    stage: SearchStage,
    result: &SearchRunResult,
    output_path: Option<&str>,
) -> Result<(), String> {
    let Some(output_path) = output_path else {
        return Ok(());
    };

    let path = match result {
        SearchRunResult::Equivalent(path) => path,
        SearchRunResult::EquivalentByConcreteShift(_) => {
            return Err(
                "--write-guide-artifact only supports path witnesses; concrete shift witnesses \
                 cannot be exported as full_path guide artifacts"
                    .to_string(),
            );
        }
        SearchRunResult::NotEquivalent(_) | SearchRunResult::Unknown => {
            return Err(
                "--write-guide-artifact requires a successful search result with a path witness"
                    .to_string(),
            );
        }
    };

    let mut artifact = build_full_path_guide_artifact(&request.source, &request.target, path)
        .map_err(|err| format!("failed to build guide artifact from search witness: {err}"))?;
    artifact.artifact_id = Some(format!(
        "search-{}-lag-{}",
        search_stage_label(stage),
        path.steps.len()
    ));
    artifact.provenance = GuideArtifactProvenance {
        source_kind: Some("search_cli".to_string()),
        label: Some(format!("search-{}-witness", search_stage_label(stage))),
        source_ref: Some(format!("search:{}", search_stage_label(stage))),
    };
    artifact.compatibility = GuideArtifactCompatibility {
        supported_stages: vec![SearchStage::GuidedRefinement],
        max_endpoint_dim: Some(request.source.rows.max(request.target.rows)),
    };

    let json = serde_json::to_string_pretty(&artifact)
        .map_err(|err| format!("failed to serialize guide artifact JSON: {err}"))?;
    fs::write(output_path, format!("{json}\n"))
        .map_err(|err| format!("failed to write guide artifact to {output_path}: {err}"))
}

fn search_stage_label(stage: SearchStage) -> &'static str {
    match stage {
        SearchStage::EndpointSearch => "endpoint_search",
        SearchStage::GuidedRefinement => "guided_refinement",
        SearchStage::ShortcutSearch => "shortcut_search",
    }
}

fn next_parsed<I, T>(args: &mut I, flag: &str) -> Result<T, String>
where
    I: Iterator<Item = String>,
    T: std::str::FromStr,
    T::Err: std::fmt::Display,
{
    let value = args.next().ok_or(format!("{flag} requires a value"))?;
    value
        .parse()
        .map_err(|err| format!("invalid value for {flag}: {err}"))
}

fn parse_matrix(s: &str) -> Result<DynMatrix, String> {
    if let Some((dims, entries)) = s.split_once(':') {
        let (rows, cols) = parse_dims(dims)?;
        if rows != cols {
            return Err(format!("matrix must be square, got {rows}x{cols}"));
        }
        let nums = parse_entries(entries)?;
        if nums.len() != rows * cols {
            return Err(format!(
                "expected {} comma-separated entries for a {}x{} matrix, got {}",
                rows * cols,
                rows,
                cols,
                nums.len()
            ));
        }
        return Ok(DynMatrix::new(rows, cols, nums));
    }

    let nums = parse_entries(s)?;
    if nums.len() != 4 {
        return Err(format!(
            "expected 4 entries for a bare 2x2 matrix, got {}; use NxN:... for larger endpoints",
            nums.len()
        ));
    }
    Ok(DynMatrix::new(2, 2, nums))
}

fn parse_dims(s: &str) -> Result<(usize, usize), String> {
    let (rows, cols) = s
        .split_once('x')
        .ok_or_else(|| format!("invalid matrix prefix '{s}' (expected NxN)"))?;
    let rows: usize = rows
        .parse()
        .map_err(|err| format!("invalid row count in '{s}': {err}"))?;
    let cols: usize = cols
        .parse()
        .map_err(|err| format!("invalid column count in '{s}': {err}"))?;
    Ok((rows, cols))
}

fn parse_entries(s: &str) -> Result<Vec<u32>, String> {
    s.split(',')
        .map(|part| {
            part.trim()
                .parse::<u32>()
                .map_err(|err| format!("invalid matrix entry '{part}': {err}"))
        })
        .collect()
}

fn print_pretty(
    a: &DynMatrix,
    b: &DynMatrix,
    stage: SearchStage,
    result: &SearchRunResult,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) {
    println!("Stage = {:?}", stage);
    println!("A = {}", format_dyn_matrix(a));
    println!("B = {}", format_dyn_matrix(b));
    println!();

    match result {
        SearchRunResult::Equivalent(path) => {
            println!("Result: EQUIVALENT ({} step(s))", path.steps.len());
            println!();
            for (i, step) in path.steps.iter().enumerate() {
                println!("Step {}:", i + 1);
                println!("  U = {}", format_dyn_matrix(&step.u));
                println!("  V = {}", format_dyn_matrix(&step.v));
            }
        }
        SearchRunResult::EquivalentByConcreteShift(_witness) => {
            println!("Result: EQUIVALENT (concrete shift witness)");
        }
        SearchRunResult::NotEquivalent(reason) => {
            println!("Result: NOT EQUIVALENT");
            println!("Reason: {reason}");
        }
        SearchRunResult::Unknown => {
            println!("Result: UNKNOWN (search exhausted)");
        }
    }

    if show_telemetry {
        print_telemetry(telemetry);
    }
}

fn print_telemetry(telemetry: &SearchTelemetry) {
    println!();
    println!("Telemetry:");
    println!("  layers: {}", telemetry.layers.len());
    println!(
        "  frontier nodes expanded: {}",
        telemetry.frontier_nodes_expanded
    );
    println!(
        "  factorisations enumerated: {}",
        telemetry.factorisations_enumerated
    );
    println!(
        "  candidates after pruning: {}",
        telemetry.candidates_after_pruning
    );
    println!("  discovered nodes: {}", telemetry.discovered_nodes);
    println!("  total visited nodes: {}", telemetry.total_visited_nodes);
    println!("  max frontier size: {}", telemetry.max_frontier_size);
    println!(
        "  guide artifacts considered: {}",
        telemetry.guide_artifacts_considered
    );
    println!(
        "  guide artifacts accepted: {}",
        telemetry.guide_artifacts_accepted
    );
    println!(
        "  guided segments considered: {}",
        telemetry.guided_segments_considered
    );
    println!(
        "  guided segments improved: {}",
        telemetry.guided_segments_improved
    );
    println!(
        "  guided refinement rounds: {}",
        telemetry.guided_refinement_rounds
    );
}

fn print_json(
    a: &DynMatrix,
    b: &DynMatrix,
    stage: SearchStage,
    result: &SearchRunResult,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) {
    let (outcome, steps, reason) = match result {
        SearchRunResult::Equivalent(path) => (
            "equivalent",
            Some(
                path.steps
                    .iter()
                    .map(step_json)
                    .collect::<Vec<serde_json::Value>>(),
            ),
            None,
        ),
        SearchRunResult::EquivalentByConcreteShift(_) => {
            ("equivalent_by_concrete_shift", None, None)
        }
        SearchRunResult::NotEquivalent(reason) => ("not_equivalent", None, Some(reason.clone())),
        SearchRunResult::Unknown => ("unknown", None, None),
    };

    print_json_value(
        serde_json::json!(dyn_matrix_to_vecs(a)),
        serde_json::json!(dyn_matrix_to_vecs(b)),
        stage,
        outcome,
        steps,
        reason,
        telemetry,
        show_telemetry,
    );
}

fn print_json_value(
    a: serde_json::Value,
    b: serde_json::Value,
    stage: SearchStage,
    outcome: &str,
    steps: Option<Vec<serde_json::Value>>,
    reason: Option<String>,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) {
    let mut obj = serde_json::json!({
        "a": a,
        "b": b,
        "stage": stage,
        "outcome": outcome,
    });

    if let Some(steps) = steps {
        obj["steps"] = serde_json::json!(steps);
    }
    if let Some(reason) = reason {
        obj["reason"] = serde_json::json!(reason);
    }
    if show_telemetry {
        obj["telemetry"] = serde_json::to_value(telemetry).unwrap_or_default();
    }

    println!(
        "{}",
        serde_json::to_string_pretty(&obj).expect("json serialization")
    );
}

fn step_json(step: &sse_core::types::EsseStep) -> serde_json::Value {
    serde_json::json!({
        "u": dyn_matrix_to_vecs(&step.u),
        "v": dyn_matrix_to_vecs(&step.v),
    })
}

fn format_dyn_matrix(m: &DynMatrix) -> String {
    let rows: Vec<String> = (0..m.rows)
        .map(|r| {
            let entries: Vec<String> = (0..m.cols)
                .map(|c| m.data[r * m.cols + c].to_string())
                .collect();
            format!("[{}]", entries.join(", "))
        })
        .collect();
    format!("[{}]", rows.join(", "))
}

fn dyn_matrix_to_vecs(m: &DynMatrix) -> Vec<Vec<u32>> {
    (0..m.rows)
        .map(|r| (0..m.cols).map(|c| m.data[r * m.cols + c]).collect())
        .collect()
}

#[derive(serde::Deserialize)]
#[serde(untagged)]
enum GuideArtifactFile {
    Artifact(GuideArtifact),
    Artifacts(Vec<GuideArtifact>),
    Envelope { artifacts: Vec<GuideArtifact> },
}

fn load_guide_artifacts(path: impl AsRef<Path>) -> Result<Vec<GuideArtifact>, String> {
    let path = path.as_ref();
    let json = fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read guide artifacts from {}: {err}",
            path.display()
        )
    })?;
    let parsed: GuideArtifactFile = serde_json::from_str(&json).map_err(|err| {
        format!(
            "failed to parse guide artifacts from {} as JSON: {err}",
            path.display()
        )
    })?;
    Ok(match parsed {
        GuideArtifactFile::Artifact(artifact) => vec![artifact],
        GuideArtifactFile::Artifacts(artifacts) => artifacts,
        GuideArtifactFile::Envelope { artifacts } => artifacts,
    })
}

#[cfg(test)]
mod tests {
    use super::{parse_cli, parse_matrix, run_with_args};
    use sse_core::types::{GuideArtifact, GuideArtifactPayload, SearchStage};
    use std::fs;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn parse_bare_2x2_matrix() {
        let matrix = parse_matrix("1,2,3,4").unwrap();
        assert_eq!(matrix.rows, 2);
        assert_eq!(matrix.cols, 2);
        assert_eq!(matrix.data, vec![1, 2, 3, 4]);
    }

    #[test]
    fn parse_prefixed_square_matrix() {
        let matrix = parse_matrix("3x3:0,1,0,1,0,1,0,1,0").unwrap();
        assert_eq!(matrix.rows, 3);
        assert_eq!(matrix.cols, 3);
        assert_eq!(matrix.data, vec![0, 1, 0, 1, 0, 1, 0, 1, 0]);
    }

    #[test]
    fn parse_cli_accepts_write_guide_artifact_flag() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--write-guide-artifact".to_string(),
                "guide.json".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.write_guide_artifact.as_deref(), Some("guide.json"));
    }

    #[test]
    fn parse_cli_accepts_guided_segment_timeout_flag() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--guided-segment-timeout".to_string(),
                "10".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.guided_refinement.segment_timeout_secs, Some(10));
    }

    #[test]
    fn run_with_args_writes_guide_artifact_for_path_witness() {
        let output_path = temp_output_path("guide-artifact");

        let exit_code = run_with_args(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--write-guide-artifact".to_string(),
                output_path.display().to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);

        let json = fs::read_to_string(&output_path).unwrap();
        let artifact: GuideArtifact = serde_json::from_str(&json).unwrap();
        assert_eq!(
            artifact.provenance.source_kind.as_deref(),
            Some("search_cli")
        );
        assert_eq!(
            artifact.compatibility.supported_stages,
            vec![SearchStage::GuidedRefinement]
        );
        assert_eq!(artifact.quality.lag, Some(0));
        assert!(matches!(
            artifact.payload,
            GuideArtifactPayload::FullPath { path } if path.steps.is_empty()
        ));

        let _ = fs::remove_file(output_path);
    }

    #[test]
    fn run_with_args_rejects_guide_artifact_export_without_path_witness() {
        let output_path = temp_output_path("guide-artifact-error");

        let err = run_with_args(
            vec![
                "2,1,1,1".to_string(),
                "3,1,1,1".to_string(),
                "--write-guide-artifact".to_string(),
                output_path.display().to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert!(err.contains("requires a successful search result with a path witness"));
        assert!(!output_path.exists());
    }

    fn temp_output_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!(
            "sse-core-search-{label}-{}-{nonce}.json",
            std::process::id()
        ))
    }
}
