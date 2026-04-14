use std::fs;
use std::process::ExitCode;

#[cfg(feature = "dhat-profile")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::search::{
    build_full_path_guide_artifact, execute_search_request, execute_search_request_and_observer,
};
#[cfg(not(target_arch = "wasm32"))]
use sse_core::sqlite_graph::SqliteGraphRecorder;
use sse_core::types::{
    FrontierMode, GuideArtifactCompatibility, GuideArtifactProvenance, GuidedRefinementConfig,
    MoveFamilyPolicy, SearchConfig, SearchRequest, SearchRunResult, SearchStage, SearchTelemetry,
    ShortcutSearchConfig, DEFAULT_BEAM_WIDTH,
};

#[derive(Debug)]
struct Cli {
    a: DynMatrix,
    b: DynMatrix,
    config: SearchConfig,
    stage: SearchStage,
    guide_artifact_paths: Vec<String>,
    guide_artifact_dirs: Vec<String>,
    guided_refinement: GuidedRefinementConfig,
    shortcut_search: ShortcutSearchConfig,
    json: bool,
    telemetry: bool,
    pprof: bool,
    dhat: bool,
    visited_db: Option<String>,
    write_guide_artifact: Option<String>,
}

#[cfg(feature = "pprof-profile")]
type CpuProfileGuard = pprof::ProfilerGuard<'static>;

#[cfg(not(feature = "pprof-profile"))]
struct CpuProfileGuard;

#[cfg(feature = "dhat-profile")]
type HeapProfiler = dhat::Profiler;

#[cfg(not(feature = "dhat-profile"))]
struct HeapProfiler;

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
        guide_artifacts.extend(load_guide_artifacts_from_path(path)?);
    }
    for dir in &cli.guide_artifact_dirs {
        guide_artifacts.extend(load_guide_artifacts_from_path(dir)?);
    }
    if cli.stage == SearchStage::GuidedRefinement && guide_artifacts.is_empty() {
        return Err(
            "guided_refinement requires at least one --guide-artifacts file or --guide-artifact-dir"
                .to_string(),
        );
    }
    let request = SearchRequest {
        source: cli.a.clone(),
        target: cli.b.clone(),
        config: cli.config.clone(),
        stage: cli.stage,
        guide_artifacts,
        guided_refinement: cli.guided_refinement.clone(),
        shortcut_search: cli.shortcut_search.clone(),
    };
    let cpu_profile = start_cpu_profile(cli.pprof)?;
    let _heap_profile = start_heap_profile(cli.dhat)?;

    let (result, telemetry) = if let Some(path) = cli.visited_db.as_deref() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            let mut recorder = SqliteGraphRecorder::new(path)?;
            let profiled_result =
                execute_search_request_and_observer(&request, Some(&mut recorder))?;
            if let Some(err) = recorder.error() {
                return Err(format!("failed to persist visited graph to {path}: {err}"));
            }
            profiled_result
        }
        #[cfg(target_arch = "wasm32")]
        {
            return Err("--visited-db is not supported on wasm32 targets".to_string());
        }
    } else {
        execute_search_request(&request)?
    };
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
    let code = exit_code(&result);
    finish_cpu_profile(cpu_profile);
    Ok(code)
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
    let mut guide_artifact_dirs = Vec::new();
    let mut guided_refinement = GuidedRefinementConfig::default();
    let mut shortcut_search = ShortcutSearchConfig::default();
    let mut json = false;
    let mut telemetry = false;
    let mut pprof = false;
    let mut dhat = false;
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
                       --frontier-mode MODE     bfs | beam | beam-bfs-handoff (default: bfs)\n\
                       --move-policy POLICY     mixed | graph-only (default: mixed)\n\
                       --search-mode MODE       legacy shortcut: mixed | graph-only | beam\n\
                       --beam-width N           cap each beam frontier (default when beam is selected: 64)\n\
                       --stage STAGE            endpoint-search | guided-refinement | shortcut-search\n\
                                              (shortcut-search runs iterative bounded refinement over a reusable guide pool; default: endpoint-search)\n\
                       --guide-artifacts PATH   read JSON guide artifact(s) from PATH (repeatable)\n\
                       --guide-artifact-dir DIR read all JSON guide artifact(s) from DIR (repeatable)\n\
                       --guided-max-shortcut-lag N max lag for one guided shortcut search (default: 3)\n\
                       --guided-min-gap N       minimum guide gap to consider for refinement (default: 2)\n\
                       --guided-max-gap N       maximum guide gap to consider for refinement\n\
                       --guided-segment-timeout SECS\n\
                                               max wall-clock seconds for one guided segment search\n\
                       --guided-rounds N        number of refinement rounds per guide (default: 1)\n\
                       --shortcut-max-guides N  cap the initial shortcut guide working set (default: 32)\n\
                       --shortcut-rounds N      cap outer shortcut rounds (default: 5)\n\
                       --shortcut-max-total-segment-attempts N\n\
                                              cap total segment attempts across the stage (default: 128)\n\
                       --shortcut-emit-promoted-guides\n\
                                              request promoted guide artifacts on the generic output surface\n\
                       --visited-db PATH        write visited nodes and SSE edges to a sqlite db\n\
                       --write-guide-artifact PATH\n\
                                               write a reusable full_path guide artifact JSON file\n\
                       --json                   output JSON instead of human-readable text\n\
                       --telemetry              include search telemetry in output\n\
                       --pprof                  print a terminal CPU profile (requires pprof-profile feature)\n\
                       --dhat                   print a heap profile summary on exit (requires dhat-profile feature)"
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
            "--frontier-mode" => {
                let value = args.next().ok_or("--frontier-mode requires a value")?;
                config.frontier_mode = parse_frontier_mode(&value)?;
            }
            "--move-policy" | "--move-family-policy" => {
                let value = args.next().ok_or(format!("{arg} requires a value"))?;
                config.move_family_policy = parse_move_policy(&value)?;
            }
            "--search-mode" => {
                let value = args.next().ok_or("--search-mode requires a value")?;
                apply_legacy_search_mode(&mut config, &value)?;
            }
            "--beam-width" => {
                let width: usize = next_parsed(&mut args, "--beam-width")?;
                if width == 0 {
                    return Err("--beam-width must be at least 1".to_string());
                }
                config.beam_width = Some(width);
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
            "--guide-artifact-dir" => {
                guide_artifact_dirs
                    .push(args.next().ok_or("--guide-artifact-dir requires a path")?);
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
            "--shortcut-max-guides" => {
                shortcut_search.max_guides = next_parsed(&mut args, "--shortcut-max-guides")?;
            }
            "--shortcut-rounds" => {
                shortcut_search.rounds = next_parsed(&mut args, "--shortcut-rounds")?;
            }
            "--shortcut-max-total-segment-attempts" => {
                shortcut_search.max_total_segment_attempts =
                    next_parsed(&mut args, "--shortcut-max-total-segment-attempts")?;
            }
            "--shortcut-emit-promoted-guides" => {
                shortcut_search.artifacts.emit_promoted_guides = true;
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
            "--pprof" => pprof = true,
            "--dhat" => dhat = true,
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
    if config.frontier_mode.uses_beam_width() && config.beam_width.is_none() {
        config.beam_width = Some(DEFAULT_BEAM_WIDTH);
    }
    if !config.frontier_mode.uses_beam_width() && config.beam_width.is_some() {
        return Err("--beam-width requires --frontier-mode beam or beam-bfs-handoff".to_string());
    }

    Ok(Cli {
        a,
        b,
        config,
        stage,
        guide_artifact_paths,
        guide_artifact_dirs,
        guided_refinement,
        shortcut_search,
        json,
        telemetry,
        pprof,
        dhat,
        visited_db,
        write_guide_artifact,
    })
}

fn parse_frontier_mode(value: &str) -> Result<FrontierMode, String> {
    match value {
        "bfs" => Ok(FrontierMode::Bfs),
        "beam" => Ok(FrontierMode::Beam),
        "beam-bfs-handoff" | "beam_bfs_handoff" => Ok(FrontierMode::BeamBfsHandoff),
        _ => Err(format!("unknown frontier mode: {value}")),
    }
}

fn parse_move_policy(value: &str) -> Result<MoveFamilyPolicy, String> {
    match value {
        "mixed" => Ok(MoveFamilyPolicy::Mixed),
        "graph-only" | "graph_only" => Ok(MoveFamilyPolicy::GraphOnly),
        _ => Err(format!("unknown move policy: {value}")),
    }
}

fn apply_legacy_search_mode(config: &mut SearchConfig, value: &str) -> Result<(), String> {
    match value {
        "mixed" => {
            config.frontier_mode = FrontierMode::Bfs;
            config.move_family_policy = MoveFamilyPolicy::Mixed;
        }
        "graph-only" | "graph_only" => {
            config.frontier_mode = FrontierMode::Bfs;
            config.move_family_policy = MoveFamilyPolicy::GraphOnly;
        }
        "beam" => {
            config.frontier_mode = FrontierMode::Beam;
            config.move_family_policy = MoveFamilyPolicy::Mixed;
        }
        _ => return Err(format!("unknown search mode: {value}")),
    }
    Ok(())
}

fn start_cpu_profile(enabled: bool) -> Result<Option<CpuProfileGuard>, String> {
    if !enabled {
        return Ok(None);
    }

    #[cfg(feature = "pprof-profile")]
    {
        let guard = pprof::ProfilerGuardBuilder::default()
            .frequency(1000)
            .blocklist(&["libc", "libgcc", "pthread", "vdso"])
            .build()
            .map_err(|err| format!("failed to start pprof profiler: {err}"))?;
        Ok(Some(guard))
    }

    #[cfg(not(feature = "pprof-profile"))]
    {
        Err("--pprof requires building with --features pprof-profile".to_string())
    }
}

fn finish_cpu_profile(_guard: Option<CpuProfileGuard>) {
    #[cfg(feature = "pprof-profile")]
    if let Some(guard) = _guard {
        match guard.report().build() {
            Ok(report) => {
                eprintln!("--- CPU profile ---");
                eprintln!("{report:?}");
            }
            Err(err) => eprintln!("--- CPU profile build failed: {err}"),
        }
    }
}

fn start_heap_profile(enabled: bool) -> Result<Option<HeapProfiler>, String> {
    if !enabled {
        return Ok(None);
    }

    #[cfg(feature = "dhat-profile")]
    {
        Ok(Some(dhat::Profiler::new_heap()))
    }

    #[cfg(not(feature = "dhat-profile"))]
    {
        Err("--dhat requires building with --features dhat-profile".to_string())
    }
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
        supported_stages: if request
            .shortcut_search
            .artifacts
            .supported_stages
            .is_empty()
        {
            vec![SearchStage::GuidedRefinement, SearchStage::ShortcutSearch]
        } else {
            request.shortcut_search.artifacts.supported_stages.clone()
        },
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
    let total_layer_nanos: u64 = telemetry
        .layers
        .iter()
        .map(|layer| layer.timing.total_nanos)
        .sum();
    let expand_compute_nanos: u64 = telemetry
        .layers
        .iter()
        .map(|layer| layer.timing.expand_compute_nanos)
        .sum();
    let expand_accumulate_nanos: u64 = telemetry
        .layers
        .iter()
        .map(|layer| layer.timing.expand_accumulate_nanos)
        .sum();
    let dedup_nanos: u64 = telemetry
        .layers
        .iter()
        .map(|layer| layer.timing.dedup_nanos)
        .sum();
    let merge_nanos: u64 = telemetry
        .layers
        .iter()
        .map(|layer| layer.timing.merge_nanos)
        .sum();
    let finalize_nanos: u64 = telemetry
        .layers
        .iter()
        .map(|layer| layer.timing.finalize_nanos)
        .sum();

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
        "  layer timing total: {:.3} ms",
        total_layer_nanos as f64 / 1_000_000.0
    );
    println!(
        "  layer timing split: compute={:.3} ms, accumulate={:.3} ms, dedup={:.3} ms, merge={:.3} ms, finalize={:.3} ms",
        expand_compute_nanos as f64 / 1_000_000.0,
        expand_accumulate_nanos as f64 / 1_000_000.0,
        dedup_nanos as f64 / 1_000_000.0,
        merge_nanos as f64 / 1_000_000.0,
        finalize_nanos as f64 / 1_000_000.0,
    );
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
    println!(
        "  shortcut guides loaded: {}",
        telemetry.shortcut_search.guide_artifacts_loaded
    );
    println!(
        "  shortcut guides accepted: {}",
        telemetry.shortcut_search.guide_artifacts_accepted
    );
    println!(
        "  shortcut unique guides: {}",
        telemetry.shortcut_search.unique_guides
    );
    println!(
        "  shortcut working set guides: {}",
        telemetry.shortcut_search.initial_working_set_guides
    );
    println!(
        "  shortcut segment attempts: {}",
        telemetry.shortcut_search.segment_attempts
    );
    println!(
        "  shortcut segment improvements: {}",
        telemetry.shortcut_search.segment_improvements
    );
    println!(
        "  shortcut promoted guides: {}",
        telemetry.shortcut_search.promoted_guides
    );
    println!(
        "  shortcut emitted guides: {}",
        telemetry.shortcut_search.emitted_guide_artifacts
    );
    println!(
        "  shortcut rounds completed: {}",
        telemetry.shortcut_search.rounds_completed
    );
    println!(
        "  shortcut best lag: {:?} -> {:?}",
        telemetry.shortcut_search.best_lag_start, telemetry.shortcut_search.best_lag_end
    );
    println!(
        "  shortcut stop reason: {:?}",
        telemetry.shortcut_search.stop_reason
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

#[cfg(test)]
mod tests {
    use super::{parse_cli, parse_matrix, run_with_args};
    use rusqlite::Connection;
    use sse_core::types::{
        FrontierMode, GuideArtifact, GuideArtifactPayload, MoveFamilyPolicy, SearchStage,
    };
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
    fn parse_cli_accepts_guide_artifact_dir_flag() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--guide-artifact-dir".to_string(),
                "guides".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.guide_artifact_dirs, vec!["guides".to_string()]);
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
    fn parse_cli_accepts_shortcut_boundary_flags() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--stage".to_string(),
                "shortcut-search".to_string(),
                "--shortcut-max-guides".to_string(),
                "8".to_string(),
                "--shortcut-rounds".to_string(),
                "2".to_string(),
                "--shortcut-max-total-segment-attempts".to_string(),
                "16".to_string(),
                "--shortcut-emit-promoted-guides".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.stage, SearchStage::ShortcutSearch);
        assert_eq!(cli.shortcut_search.max_guides, 8);
        assert_eq!(cli.shortcut_search.rounds, 2);
        assert_eq!(cli.shortcut_search.max_total_segment_attempts, 16);
        assert!(cli.shortcut_search.artifacts.emit_promoted_guides);
    }

    #[test]
    fn parse_cli_supports_all_frontier_and_move_policy_combinations() {
        let cases = [
            (
                "bfs",
                "mixed",
                FrontierMode::Bfs,
                MoveFamilyPolicy::Mixed,
                None,
            ),
            (
                "bfs",
                "graph-only",
                FrontierMode::Bfs,
                MoveFamilyPolicy::GraphOnly,
                None,
            ),
            (
                "beam",
                "mixed",
                FrontierMode::Beam,
                MoveFamilyPolicy::Mixed,
                Some("7"),
            ),
            (
                "beam",
                "graph-only",
                FrontierMode::Beam,
                MoveFamilyPolicy::GraphOnly,
                Some("9"),
            ),
            (
                "beam-bfs-handoff",
                "mixed",
                FrontierMode::BeamBfsHandoff,
                MoveFamilyPolicy::Mixed,
                Some("11"),
            ),
        ];

        for (frontier, move_policy, expected_frontier, expected_move_policy, beam_width) in cases {
            let mut args = vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--frontier-mode".to_string(),
                frontier.to_string(),
                "--move-policy".to_string(),
                move_policy.to_string(),
            ];
            if let Some(width) = beam_width {
                args.push("--beam-width".to_string());
                args.push(width.to_string());
            }
            let cli = parse_cli(args.into_iter()).unwrap();

            assert_eq!(cli.config.frontier_mode, expected_frontier);
            assert_eq!(cli.config.move_family_policy, expected_move_policy);
            assert_eq!(
                cli.config.beam_width,
                beam_width.map(|value| value.parse().unwrap())
            );
        }
    }

    #[test]
    fn parse_cli_accepts_legacy_search_mode_beam() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--search-mode".to_string(),
                "beam".to_string(),
                "--beam-width".to_string(),
                "7".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.config.frontier_mode, FrontierMode::Beam);
        assert_eq!(cli.config.move_family_policy, MoveFamilyPolicy::Mixed);
        assert_eq!(cli.config.beam_width, Some(7));
    }

    #[test]
    fn parse_cli_rejects_zero_beam_width() {
        let err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--beam-width".to_string(),
                "0".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert_eq!(err, "--beam-width must be at least 1");
    }

    #[test]
    fn parse_cli_rejects_beam_width_without_beam_mode() {
        let err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--beam-width".to_string(),
                "7".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            "--beam-width requires --frontier-mode beam or beam-bfs-handoff"
        );
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
            vec![SearchStage::GuidedRefinement, SearchStage::ShortcutSearch]
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

    #[test]
    fn run_with_args_shortcut_search_accepts_guide_artifact_directory() {
        let dir = temp_output_path("guide-artifact-dir");
        fs::create_dir_all(&dir).unwrap();
        let guide_path = dir.join("guide.json");
        fs::write(
            &guide_path,
            r#"{
  "artifact_id": "identity",
  "endpoints": {
    "source": {"rows": 2, "cols": 2, "data": [1, 0, 0, 1]},
    "target": {"rows": 2, "cols": 2, "data": [1, 0, 0, 1]}
  },
  "kind": "full_path",
  "path": {
    "matrices": [{"rows": 2, "cols": 2, "data": [1, 0, 0, 1]}],
    "steps": []
  },
  "compatibility": {
    "supported_stages": ["guided_refinement"]
  },
  "quality": {
    "cost": 0
  }
}
"#,
        )
        .unwrap();

        let exit_code = run_with_args(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--stage".to_string(),
                "shortcut-search".to_string(),
                "--guide-artifact-dir".to_string(),
                dir.display().to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn run_with_args_writes_visited_db() {
        let output_path = temp_sqlite_path("visited-db");

        let exit_code = run_with_args(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--visited-db".to_string(),
                output_path.display().to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(exit_code, std::process::ExitCode::SUCCESS);
        assert!(output_path.exists());

        let conn = Connection::open(&output_path).unwrap();
        let run_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM search_runs", [], |row| row.get(0))
            .unwrap();
        let node_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM run_nodes", [], |row| row.get(0))
            .unwrap();

        assert_eq!(run_count, 1);
        assert_eq!(node_count, 1);

        drop(conn);
        cleanup_sqlite_artifacts(&output_path);
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

    fn temp_sqlite_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::current_dir().unwrap().join(format!(
            "sse-core-search-{label}-{}-{nonce}.sqlite",
            std::process::id()
        ))
    }

    fn cleanup_sqlite_artifacts(path: &std::path::Path) {
        let _ = fs::remove_file(path);
        let _ = fs::remove_file(format!("{}-wal", path.display()));
        let _ = fs::remove_file(format!("{}-shm", path.display()));
    }
}
