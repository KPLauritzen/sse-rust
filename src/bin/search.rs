use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;
use std::process::ExitCode;

#[cfg(feature = "dhat-profile")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

use sse_core::endpoint_local_parity::{
    endpoint_local_parity_action, mass_support_signature, supports_square_endpoint_local_parity,
    trimmed_active_window_signature, EndpointLocalParityAction,
};
use sse_core::guide_artifacts::load_guide_artifacts_from_path;
use sse_core::matrix::DynMatrix;
use sse_core::search::{
    build_full_path_guide_artifact, execute_search_request, execute_search_request_and_observer,
};
use sse_core::search_observer::{
    SearchEdgeRecord, SearchEdgeStatus, SearchEvent, SearchObserver, SearchRootRecord,
};
use sse_core::sqlite_graph::SqliteGraphRecorder;
use sse_core::types::{
    DynSsePath, EndpointExactMeetSurface, EndpointExactMeetWitness, FrontierMode,
    GuideArtifactCompatibility, GuideArtifactPayload, GuideArtifactProvenance,
    GuidedRefinementConfig, MoveFamilyPolicy, SearchConfig, SearchRequest, SearchRunResult,
    SearchStage, SearchTelemetry, ShortcutSearchConfig, DEFAULT_BEAM_WIDTH,
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
    approximate_hit_parity_report: Option<String>,
    endpoint_witness_inventory: Option<String>,
    endpoint_witness_control_guides: Vec<ControlGuideSpec>,
    endpoint_witness_guide_dir: Option<String>,
    endpoint_witness_guide_ranks: Option<Vec<usize>>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ControlGuideSpec {
    class: String,
    path: String,
    artifact_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct EndpointWitnessInventory {
    artifact_kind: &'static str,
    source: Vec<Vec<u32>>,
    target: Vec<Vec<u32>>,
    requested_cap: usize,
    retained_count: usize,
    orientation_status: &'static str,
    orientation_note: &'static str,
    controls_loaded: Vec<EndpointWitnessControlSummary>,
    rows: Vec<EndpointWitnessInventoryRow>,
    emitted_guide_artifacts: Vec<EndpointWitnessGuideArtifactOutput>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct EndpointWitnessControlSummary {
    class: String,
    artifact_id: Option<String>,
    label: Option<String>,
    source_ref: String,
    reconstructed_path_length: usize,
    full_path_hash: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct EndpointWitnessInventoryRow {
    retained_rank: usize,
    retained_index: usize,
    meet_lag: usize,
    reconstructed_path_length: usize,
    endpoint_orientation: &'static str,
    meeting_state_signature: String,
    full_path_signature: String,
    full_path_hash: String,
    control_matches: Vec<EndpointWitnessControlMatch>,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct EndpointWitnessControlMatch {
    class: String,
    artifact_id: Option<String>,
    label: Option<String>,
    source_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
struct EndpointWitnessGuideArtifactOutput {
    retained_rank: usize,
    path: String,
    artifact_id: String,
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

    let mut recorder = if let Some(path) = cli.visited_db.as_deref() {
        Some(SqliteGraphRecorder::new(path)?)
    } else {
        None
    };
    let mut approximate_hit_parity_observer = cli
        .approximate_hit_parity_report
        .as_ref()
        .map(|_| ApproximateHitParityObserver::default());
    let (result, telemetry) = if recorder.is_none() && approximate_hit_parity_observer.is_none() {
        execute_search_request(&request)?
    } else {
        let mut combined_observer = CombinedSearchObserver {
            recorder: recorder.as_mut(),
            approximate_hit_parity: approximate_hit_parity_observer.as_mut(),
        };
        execute_search_request_and_observer(&request, Some(&mut combined_observer))?
    };
    if let Some(path) = cli.visited_db.as_deref() {
        if let Some(recorder) = recorder.as_ref() {
            if let Some(err) = recorder.error() {
                return Err(format!("failed to persist visited graph to {path}: {err}"));
            }
        }
    }
    maybe_write_guide_artifact(
        &request,
        cli.stage,
        &result,
        cli.write_guide_artifact.as_deref(),
    )?;
    maybe_write_approximate_hit_parity_report(
        &request,
        &result,
        &telemetry,
        approximate_hit_parity_observer.as_ref(),
        cli.approximate_hit_parity_report.as_deref(),
    )?;
    maybe_write_endpoint_witness_inventory(&request, &telemetry, &cli)?;
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
        SearchRunResult::Equivalent(_) | SearchRunResult::EquivalentByStructuredProof(_) => {
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
    let mut approximate_hit_parity_report = None;
    let mut endpoint_witness_inventory = None;
    let mut endpoint_witness_control_guides = Vec::new();
    let mut endpoint_witness_guide_dir = None;
    let mut endpoint_witness_guide_ranks = None;

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
                       --frontier-mode MODE     bfs | beam | concrete-shift-profile-beam | witness-bridge-profile-beam | same-future-past-diversity-beam | beam-bfs-handoff | stratified-beam-refill (default: bfs)\n\
                       --move-policy POLICY     mixed | graph-plus-structured | graph-only (default: mixed)\n\
                       --search-mode MODE       legacy shortcut: mixed | graph-plus-structured | graph-only | beam\n\
                       --beam-width N           cap each beam frontier (default when beam is selected: 64)\n\
                       --beam-bfs-handoff-depth N\n\
                                              inclusive beam depth before handing discoveries to BFS (default: 4)\n\
                       --beam-bfs-handoff-deferred-cap N\n\
                                              cap retained deferred overflow entries before BFS, or global deferred cap for stratified-beam-refill\n\
                       --endpoint-multi-meet-cap N\n\
                                              retain up to N admissible exact endpoint meets on the CLI output surface (endpoint-search + bfs only)\n\
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
                       --approximate-hit-parity-report PATH\n\
                                              write an opt-in report that annotates approximate_other_side_hits with endpoint-local parity actions\n\
                       --endpoint-witness-inventory PATH\n\
                                               write a compact retained exact-meet witness inventory JSON file\n\
                       --endpoint-witness-control-guide CLASS=PATH[#ARTIFACT_ID]\n\
                                               load pinned full_path control guide(s) for exact signature matching (repeatable)\n\
                       --endpoint-witness-guide-dir DIR\n\
                                               write selected retained exact-meet witnesses as full_path guide artifacts\n\
                       --endpoint-witness-guide-ranks RANKS\n\
                                               comma-separated 1-based retained ranks to write with --endpoint-witness-guide-dir\n\
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
            "--beam-bfs-handoff-depth" => {
                config.beam_bfs_handoff_depth =
                    Some(next_parsed(&mut args, "--beam-bfs-handoff-depth")?);
            }
            "--beam-bfs-handoff-deferred-cap" => {
                config.beam_bfs_handoff_deferred_cap =
                    Some(next_parsed(&mut args, "--beam-bfs-handoff-deferred-cap")?);
            }
            "--endpoint-multi-meet-cap" => {
                let cap: usize = next_parsed(&mut args, "--endpoint-multi-meet-cap")?;
                if cap == 0 {
                    return Err("--endpoint-multi-meet-cap must be at least 1".to_string());
                }
                config.endpoint_multi_meet_cap = Some(cap);
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
            "--approximate-hit-parity-report" => {
                approximate_hit_parity_report = Some(
                    args.next()
                        .ok_or("--approximate-hit-parity-report requires a path")?,
                );
            }
            "--endpoint-witness-inventory" => {
                endpoint_witness_inventory = Some(
                    args.next()
                        .ok_or("--endpoint-witness-inventory requires a path")?,
                );
            }
            "--endpoint-witness-control-guide" => {
                endpoint_witness_control_guides.push(parse_control_guide_spec(
                    &args.next().ok_or(
                        "--endpoint-witness-control-guide requires CLASS=PATH[#ARTIFACT_ID]",
                    )?,
                )?);
            }
            "--endpoint-witness-guide-dir" => {
                endpoint_witness_guide_dir = Some(
                    args.next()
                        .ok_or("--endpoint-witness-guide-dir requires a path")?,
                );
            }
            "--endpoint-witness-guide-ranks" => {
                endpoint_witness_guide_ranks =
                    Some(parse_rank_list(&args.next().ok_or(
                        "--endpoint-witness-guide-ranks requires a comma-separated list",
                    )?)?);
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
        return Err(
            "--beam-width requires --frontier-mode beam, concrete-shift-profile-beam, witness-bridge-profile-beam, same-future-past-diversity-beam, beam-bfs-handoff, or stratified-beam-refill"
                .to_string(),
        );
    }
    if config.frontier_mode != FrontierMode::BeamBfsHandoff
        && config.beam_bfs_handoff_depth.is_some()
    {
        return Err(
            "--beam-bfs-handoff-depth requires --frontier-mode beam-bfs-handoff".to_string(),
        );
    }
    if !matches!(
        config.frontier_mode,
        FrontierMode::BeamBfsHandoff | FrontierMode::StratifiedBeamRefill
    ) && config.beam_bfs_handoff_deferred_cap.is_some()
    {
        return Err(
            "--beam-bfs-handoff-deferred-cap requires --frontier-mode beam-bfs-handoff or stratified-beam-refill".to_string(),
        );
    }
    if config.endpoint_multi_meet_cap.is_some() && stage != SearchStage::EndpointSearch {
        return Err("--endpoint-multi-meet-cap only supports --stage endpoint-search".to_string());
    }
    if config.endpoint_multi_meet_cap.is_some() && config.frontier_mode != FrontierMode::Bfs {
        return Err(
            "--endpoint-multi-meet-cap currently only supports --frontier-mode bfs".to_string(),
        );
    }
    if (endpoint_witness_inventory.is_some()
        || endpoint_witness_guide_dir.is_some()
        || !endpoint_witness_control_guides.is_empty())
        && config.endpoint_multi_meet_cap.is_none()
    {
        return Err(
            "endpoint witness inventory options require --endpoint-multi-meet-cap".to_string(),
        );
    }
    if endpoint_witness_guide_ranks.is_some() && endpoint_witness_guide_dir.is_none() {
        return Err(
            "--endpoint-witness-guide-ranks requires --endpoint-witness-guide-dir".to_string(),
        );
    }
    if !endpoint_witness_control_guides.is_empty() && endpoint_witness_inventory.is_none() {
        return Err(
            "--endpoint-witness-control-guide requires --endpoint-witness-inventory".to_string(),
        );
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
        approximate_hit_parity_report,
        endpoint_witness_inventory,
        endpoint_witness_control_guides,
        endpoint_witness_guide_dir,
        endpoint_witness_guide_ranks,
    })
}

fn parse_control_guide_spec(value: &str) -> Result<ControlGuideSpec, String> {
    let (class, path) = value
        .split_once('=')
        .ok_or("--endpoint-witness-control-guide expects CLASS=PATH[#ARTIFACT_ID]")?;
    let class = class.trim();
    let path = path.trim();
    if class.is_empty() {
        return Err("--endpoint-witness-control-guide class must not be empty".to_string());
    }
    if path.is_empty() {
        return Err("--endpoint-witness-control-guide path must not be empty".to_string());
    }
    let (path, artifact_id) = match path.rsplit_once('#') {
        Some((_path, artifact_id)) if artifact_id.trim().is_empty() => {
            return Err(
                "--endpoint-witness-control-guide artifact id after # must not be empty"
                    .to_string(),
            );
        }
        Some((path, artifact_id)) => (
            path.trim().to_string(),
            Some(artifact_id.trim().to_string()),
        ),
        None => (path.to_string(), None),
    };
    if path.is_empty() {
        return Err("--endpoint-witness-control-guide path must not be empty".to_string());
    }
    Ok(ControlGuideSpec {
        class: class.to_string(),
        path,
        artifact_id,
    })
}

fn parse_rank_list(value: &str) -> Result<Vec<usize>, String> {
    let mut ranks = Vec::new();
    for part in value.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let rank = part
            .parse::<usize>()
            .map_err(|err| format!("invalid retained rank '{part}': {err}"))?;
        if rank == 0 {
            return Err("retained ranks are 1-based and must be at least 1".to_string());
        }
        ranks.push(rank);
    }
    if ranks.is_empty() {
        return Err("--endpoint-witness-guide-ranks must include at least one rank".to_string());
    }
    ranks.sort_unstable();
    ranks.dedup();
    Ok(ranks)
}

fn parse_frontier_mode(value: &str) -> Result<FrontierMode, String> {
    match value {
        "bfs" => Ok(FrontierMode::Bfs),
        "beam" => Ok(FrontierMode::Beam),
        "concrete-shift-profile-beam" | "concrete_shift_profile_beam" => {
            Ok(FrontierMode::ConcreteShiftProfileBeam)
        }
        "witness-bridge-profile-beam" | "witness_bridge_profile_beam" => {
            Ok(FrontierMode::WitnessBridgeProfileBeam)
        }
        "same-future-past-diversity-beam" | "same_future_past_diversity_beam" => {
            Ok(FrontierMode::SameFuturePastDiversityBeam)
        }
        "beam-bfs-handoff" | "beam_bfs_handoff" => Ok(FrontierMode::BeamBfsHandoff),
        "stratified-beam-refill" | "stratified_beam_refill" => {
            Ok(FrontierMode::StratifiedBeamRefill)
        }
        _ => Err(format!("unknown frontier mode: {value}")),
    }
}

fn parse_move_policy(value: &str) -> Result<MoveFamilyPolicy, String> {
    match value {
        "mixed" => Ok(MoveFamilyPolicy::Mixed),
        "graph-plus-structured" | "graph_plus_structured" => {
            Ok(MoveFamilyPolicy::GraphPlusStructured)
        }
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
        "graph-plus-structured" | "graph_plus_structured" => {
            config.frontier_mode = FrontierMode::Bfs;
            config.move_family_policy = MoveFamilyPolicy::GraphPlusStructured;
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

struct CombinedSearchObserver<'a> {
    recorder: Option<&'a mut SqliteGraphRecorder>,
    approximate_hit_parity: Option<&'a mut ApproximateHitParityObserver>,
}

impl SearchObserver for CombinedSearchObserver<'_> {
    fn on_event(&mut self, event: &SearchEvent) {
        if let Some(recorder) = self.recorder.as_mut() {
            recorder.on_event(event);
        }
        if let Some(observer) = self.approximate_hit_parity.as_mut() {
            observer.on_event(event);
        }
    }
}

#[derive(Clone)]
struct ApproximateHitVisit {
    depth: usize,
    canonical: DynMatrix,
    orig: DynMatrix,
}

#[derive(Default)]
struct ApproximateHitParityObserver {
    visits_by_direction: BTreeMap<DirectionLabel, BTreeMap<DynMatrix, ApproximateHitVisit>>,
    visits_by_signature: BTreeMap<DirectionLabel, BTreeMap<String, Vec<ApproximateHitVisit>>>,
    annotated_hits: Vec<ApproximateHitParityAnnotatedHit>,
}

impl SearchObserver for ApproximateHitParityObserver {
    fn on_event(&mut self, event: &SearchEvent) {
        match event {
            SearchEvent::Roots(records) => {
                for record in records {
                    self.record_root(record);
                }
            }
            SearchEvent::Layer(edges) => {
                for edge in edges {
                    self.record_edge(edge);
                }
                for edge in edges {
                    if !matches!(edge.status, SearchEdgeStatus::SeenCollision) {
                        self.record_visit(
                            direction_label(edge.direction),
                            edge.to_depth,
                            edge.to_canonical.clone(),
                            edge.to_orig.clone(),
                        );
                    }
                }
            }
            _ => {}
        }
    }
}

impl ApproximateHitParityObserver {
    fn record_root(&mut self, record: &SearchRootRecord) {
        self.record_visit(
            direction_label(record.direction),
            record.depth,
            record.canonical.clone(),
            record.orig.clone(),
        );
    }

    fn record_visit(
        &mut self,
        direction: DirectionLabel,
        depth: usize,
        canonical: DynMatrix,
        orig: DynMatrix,
    ) {
        let by_direction = self.visits_by_direction.entry(direction).or_default();
        if by_direction.contains_key(&canonical) {
            return;
        }

        let signature = mass_support_signature(&canonical);
        let visit = ApproximateHitVisit {
            depth,
            canonical: canonical.clone(),
            orig,
        };
        by_direction.insert(canonical, visit.clone());
        self.visits_by_signature
            .entry(direction)
            .or_default()
            .entry(signature)
            .or_default()
            .push(visit);
    }

    fn record_edge(&mut self, edge: &SearchEdgeRecord) {
        if !edge.approximate_other_side_hit || !matches!(edge.status, SearchEdgeStatus::Discovered)
        {
            return;
        }

        let direction = direction_label(edge.direction);
        let opposite = opposite_direction(direction);
        let coarse_signature = mass_support_signature(&edge.to_canonical);
        let mut bucket_candidates = self
            .visits_by_signature
            .get(&opposite)
            .and_then(|by_signature| by_signature.get(&coarse_signature))
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .map(|candidate| annotate_bucket_candidate(&edge.to_canonical, candidate))
            .collect::<Vec<_>>();
        bucket_candidates.sort_by(compare_bucket_candidates);
        let best_action = bucket_candidates
            .first()
            .map(|candidate| candidate.action)
            .unwrap_or(EndpointLocalParityAction::Ignore);

        let supported_square_state = supports_square_endpoint_local_parity(&edge.to_canonical);
        let trimmed_signature =
            supported_square_state.then(|| trimmed_active_window_signature(&edge.to_canonical));

        self.annotated_hits.push(ApproximateHitParityAnnotatedHit {
            rank: 0,
            layer_index: edge.layer_index,
            direction,
            move_family: edge.move_family.to_string(),
            from_depth: edge.from_depth,
            to_depth: edge.to_depth,
            enqueued: edge.enqueued,
            status: status_label(edge.status).to_string(),
            supported_square_state,
            coarse_signature,
            trimmed_active_window_signature: trimmed_signature,
            from_matrix: edge.from_orig.clone(),
            to_matrix: edge.to_orig.clone(),
            to_canonical: edge.to_canonical.clone(),
            bucket_candidate_count: bucket_candidates.len(),
            best_action,
            bucket_candidates,
        });
    }

    fn build_report(
        &self,
        request: &SearchRequest,
        result: &SearchRunResult,
        telemetry: &SearchTelemetry,
    ) -> ApproximateHitParityReport {
        let mut annotated_hits = self.annotated_hits.clone();
        annotated_hits.sort_by(compare_annotated_hits);
        for (idx, hit) in annotated_hits.iter_mut().enumerate() {
            hit.rank = idx + 1;
        }

        let mut hits_by_best_action = BTreeMap::new();
        let mut candidate_actions = BTreeMap::new();
        let mut supported_square_hits = 0usize;
        let mut unsupported_hits = 0usize;
        let mut multi_candidate_buckets = 0usize;
        let unattributed_approximate_hits = telemetry
            .approximate_other_side_hits
            .saturating_sub(annotated_hits.len());

        for hit in &annotated_hits {
            *hits_by_best_action
                .entry(hit.best_action.as_str().to_string())
                .or_insert(0usize) += 1;
            if hit.supported_square_state {
                supported_square_hits += 1;
            } else {
                unsupported_hits += 1;
            }
            if hit.bucket_candidate_count > 1 {
                multi_candidate_buckets += 1;
            }
            for candidate in &hit.bucket_candidates {
                *candidate_actions
                    .entry(candidate.action.as_str().to_string())
                    .or_insert(0usize) += 1;
            }
        }

        ApproximateHitParityReport {
            source: request.source.clone(),
            target: request.target.clone(),
            config: request.config.clone(),
            stage: request.stage,
            result: result_label(result).to_string(),
            telemetry_approximate_other_side_hits: telemetry.approximate_other_side_hits,
            summary: ApproximateHitParitySummary {
                discovered_approximate_hit_records: annotated_hits.len(),
                unattributed_approximate_hits,
                report_is_complete: unattributed_approximate_hits == 0,
                completeness_note: (unattributed_approximate_hits > 0).then_some(
                    "top-level observer records only the current request surface; nested guided or shortcut segment searches can contribute additional approximate hits to telemetry",
                ),
                supported_square_hits,
                unsupported_hits,
                multi_candidate_buckets,
                hits_by_best_action,
                candidate_actions,
            },
            annotated_hits,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum DirectionLabel {
    Forward,
    Backward,
}

#[derive(Clone, serde::Serialize)]
struct ApproximateHitParityReport {
    source: DynMatrix,
    target: DynMatrix,
    config: SearchConfig,
    stage: SearchStage,
    result: String,
    telemetry_approximate_other_side_hits: usize,
    summary: ApproximateHitParitySummary,
    annotated_hits: Vec<ApproximateHitParityAnnotatedHit>,
}

#[derive(Clone, serde::Serialize)]
struct ApproximateHitParitySummary {
    discovered_approximate_hit_records: usize,
    unattributed_approximate_hits: usize,
    report_is_complete: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    completeness_note: Option<&'static str>,
    supported_square_hits: usize,
    unsupported_hits: usize,
    multi_candidate_buckets: usize,
    hits_by_best_action: BTreeMap<String, usize>,
    candidate_actions: BTreeMap<String, usize>,
}

#[derive(Clone, serde::Serialize)]
struct ApproximateHitParityAnnotatedHit {
    rank: usize,
    layer_index: usize,
    direction: DirectionLabel,
    move_family: String,
    from_depth: usize,
    to_depth: usize,
    enqueued: bool,
    status: String,
    supported_square_state: bool,
    coarse_signature: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    trimmed_active_window_signature: Option<String>,
    from_matrix: DynMatrix,
    to_matrix: DynMatrix,
    to_canonical: DynMatrix,
    bucket_candidate_count: usize,
    best_action: EndpointLocalParityAction,
    bucket_candidates: Vec<ApproximateHitParityCandidate>,
}

#[derive(Clone, serde::Serialize)]
struct ApproximateHitParityCandidate {
    action: EndpointLocalParityAction,
    counterpart_depth: usize,
    l1_distance: u64,
    trimmed_active_window_signature: Option<String>,
    counterpart_matrix: DynMatrix,
    counterpart_canonical: DynMatrix,
}

fn annotate_bucket_candidate(
    anchor: &DynMatrix,
    candidate: ApproximateHitVisit,
) -> ApproximateHitParityCandidate {
    let action = endpoint_local_parity_action(anchor, &candidate.canonical);
    let trimmed_signature = supports_square_endpoint_local_parity(&candidate.canonical)
        .then(|| trimmed_active_window_signature(&candidate.canonical));
    ApproximateHitParityCandidate {
        action,
        counterpart_depth: candidate.depth,
        l1_distance: matrix_l1_distance(anchor, &candidate.canonical),
        trimmed_active_window_signature: trimmed_signature,
        counterpart_matrix: candidate.orig,
        counterpart_canonical: candidate.canonical,
    }
}

fn compare_bucket_candidates(
    left: &ApproximateHitParityCandidate,
    right: &ApproximateHitParityCandidate,
) -> std::cmp::Ordering {
    action_priority(left.action)
        .cmp(&action_priority(right.action))
        .then_with(|| left.l1_distance.cmp(&right.l1_distance))
        .then_with(|| left.counterpart_depth.cmp(&right.counterpart_depth))
        .then_with(|| {
            left.counterpart_canonical
                .data
                .cmp(&right.counterpart_canonical.data)
        })
}

fn compare_annotated_hits(
    left: &ApproximateHitParityAnnotatedHit,
    right: &ApproximateHitParityAnnotatedHit,
) -> std::cmp::Ordering {
    action_priority(left.best_action)
        .cmp(&action_priority(right.best_action))
        .then_with(|| {
            right
                .bucket_candidate_count
                .cmp(&left.bucket_candidate_count)
        })
        .then_with(|| left.layer_index.cmp(&right.layer_index))
        .then_with(|| left.direction.cmp(&right.direction))
        .then_with(|| left.to_depth.cmp(&right.to_depth))
        .then_with(|| left.to_canonical.data.cmp(&right.to_canonical.data))
}

fn action_priority(action: EndpointLocalParityAction) -> usize {
    match action {
        EndpointLocalParityAction::ReuseEndpointLocalParity => 0,
        EndpointLocalParityAction::RankOrProposeInsideCoarseBucket => 1,
        EndpointLocalParityAction::Ignore => 2,
    }
}

fn matrix_l1_distance(left: &DynMatrix, right: &DynMatrix) -> u64 {
    if left.rows != right.rows || left.cols != right.cols {
        return u64::MAX;
    }

    left.data
        .iter()
        .zip(&right.data)
        .map(|(left, right)| left.abs_diff(*right) as u64)
        .sum()
}

fn direction_label(direction: sse_core::types::SearchDirection) -> DirectionLabel {
    match direction {
        sse_core::types::SearchDirection::Forward => DirectionLabel::Forward,
        sse_core::types::SearchDirection::Backward => DirectionLabel::Backward,
    }
}

fn opposite_direction(direction: DirectionLabel) -> DirectionLabel {
    match direction {
        DirectionLabel::Forward => DirectionLabel::Backward,
        DirectionLabel::Backward => DirectionLabel::Forward,
    }
}

fn maybe_write_approximate_hit_parity_report(
    request: &SearchRequest,
    result: &SearchRunResult,
    telemetry: &SearchTelemetry,
    observer: Option<&ApproximateHitParityObserver>,
    output_path: Option<&str>,
) -> Result<(), String> {
    let Some(output_path) = output_path else {
        return Ok(());
    };
    let observer = observer.ok_or_else(|| {
        "approximate hit parity report requested, but the observer was not initialized".to_string()
    })?;
    let report = observer.build_report(request, result, telemetry);
    let json = serde_json::to_string_pretty(&report)
        .map_err(|err| format!("failed to serialize approximate hit parity JSON: {err}"))?;
    write_string_with_parent_dirs(output_path, &format!("{json}\n"), "approximate hit parity")
}

fn write_string_with_parent_dirs(
    output_path: &str,
    contents: &str,
    label: &str,
) -> Result<(), String> {
    if let Some(parent) = Path::new(output_path).parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("failed to create {}: {err}", parent.display()))?;
        }
    }
    fs::write(output_path, contents)
        .map_err(|err| format!("failed to write {label} to {output_path}: {err}"))
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
        SearchRunResult::EquivalentByStructuredProof(_) => {
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

fn maybe_write_endpoint_witness_inventory(
    request: &SearchRequest,
    telemetry: &SearchTelemetry,
    cli: &Cli,
) -> Result<(), String> {
    if cli.endpoint_witness_inventory.is_none() && cli.endpoint_witness_guide_dir.is_none() {
        return Ok(());
    }
    let surface = telemetry.endpoint_exact_meets.as_ref().ok_or_else(|| {
        "endpoint witness inventory requested, but no retained endpoint_exact_meets surface was produced"
            .to_string()
    })?;
    let controls = load_endpoint_witness_controls(&cli.endpoint_witness_control_guides)?;
    let mut emitted_guide_artifacts = Vec::new();
    if let Some(dir) = cli.endpoint_witness_guide_dir.as_deref() {
        emitted_guide_artifacts = write_endpoint_witness_guide_artifacts(
            request,
            surface,
            dir,
            cli.endpoint_witness_guide_ranks.as_deref(),
        )?;
    }
    let inventory =
        build_endpoint_witness_inventory(request, surface, &controls, emitted_guide_artifacts);
    if let Some(path) = cli.endpoint_witness_inventory.as_deref() {
        let json = serde_json::to_string_pretty(&inventory)
            .map_err(|err| format!("failed to serialize endpoint witness inventory: {err}"))?;
        fs::write(path, format!("{json}\n")).map_err(|err| {
            format!("failed to write endpoint witness inventory to {path}: {err}")
        })?;
    }
    Ok(())
}

fn load_endpoint_witness_controls(
    specs: &[ControlGuideSpec],
) -> Result<Vec<EndpointWitnessLoadedControl>, String> {
    let mut controls = Vec::new();
    for spec in specs {
        let artifacts = load_guide_artifacts_from_path(&spec.path)?;
        let mut matched = 0usize;
        for artifact in artifacts {
            if let Some(expected_id) = spec.artifact_id.as_deref() {
                if artifact.artifact_id.as_deref() != Some(expected_id) {
                    continue;
                }
            }
            let GuideArtifactPayload::FullPath { path } = &artifact.payload;
            matched += 1;
            controls.push(EndpointWitnessLoadedControl {
                class: spec.class.clone(),
                artifact_id: artifact.artifact_id.clone(),
                label: artifact.provenance.label.clone(),
                source_ref: control_source_ref(spec),
                reconstructed_path_length: path.steps.len(),
                full_path_signature: witness_matrix_signature(path),
                full_path_hash: stable_path_hash(path),
            });
        }
        if spec.artifact_id.is_some() && matched == 0 {
            return Err(format!(
                "control guide {} did not contain artifact_id {}",
                spec.path,
                spec.artifact_id.as_deref().unwrap_or("")
            ));
        }
    }
    Ok(controls)
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct EndpointWitnessLoadedControl {
    class: String,
    artifact_id: Option<String>,
    label: Option<String>,
    source_ref: String,
    reconstructed_path_length: usize,
    full_path_signature: String,
    full_path_hash: String,
}

fn control_source_ref(spec: &ControlGuideSpec) -> String {
    match spec.artifact_id.as_deref() {
        Some(artifact_id) => format!("{}#{}", spec.path, artifact_id),
        None => spec.path.clone(),
    }
}

fn write_endpoint_witness_guide_artifacts(
    request: &SearchRequest,
    surface: &EndpointExactMeetSurface,
    dir: &str,
    selected_ranks: Option<&[usize]>,
) -> Result<Vec<EndpointWitnessGuideArtifactOutput>, String> {
    fs::create_dir_all(dir)
        .map_err(|err| format!("failed to create endpoint witness guide dir {dir}: {err}"))?;
    let selected = selected_ranks
        .map(|ranks| ranks.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_else(|| (1..=surface.retained.len()).collect::<BTreeSet<_>>());
    for rank in &selected {
        if *rank > surface.retained.len() {
            return Err(format!(
                "requested retained rank {rank}, but only {} exact meets were retained",
                surface.retained.len()
            ));
        }
    }
    let mut outputs = Vec::new();
    for (index, witness) in surface.retained.iter().enumerate() {
        let rank = index + 1;
        if !selected.contains(&rank) {
            continue;
        }
        let path_hash = stable_path_hash(&witness.path);
        let artifact_id = format!(
            "endpoint-exact-meet-rank-{rank}-meet-lag-{}-reconstructed-lag-{}-path-{}",
            witness.path_lag,
            witness.path.steps.len(),
            path_hash.trim_start_matches("fnv1a64:")
        );
        let mut artifact =
            build_full_path_guide_artifact(&request.source, &request.target, &witness.path)
                .map_err(|err| {
                    format!("failed to build retained exact-meet guide artifact rank {rank}: {err}")
                })?;
        artifact.artifact_id = Some(artifact_id.clone());
        artifact.provenance = GuideArtifactProvenance {
            source_kind: Some("endpoint_exact_meet_inventory".to_string()),
            label: Some(format!("retained endpoint exact meet rank {rank}")),
            source_ref: Some(format!("endpoint_exact_meet_rank:{rank}")),
        };
        artifact.compatibility = GuideArtifactCompatibility {
            supported_stages: vec![SearchStage::GuidedRefinement, SearchStage::ShortcutSearch],
            max_endpoint_dim: Some(request.source.rows.max(request.target.rows)),
        };
        let output_path = Path::new(dir).join(format!("rank-{rank:03}-{artifact_id}.json"));
        let json = serde_json::to_string_pretty(&artifact).map_err(|err| {
            format!("failed to serialize retained exact-meet guide artifact rank {rank}: {err}")
        })?;
        fs::write(&output_path, format!("{json}\n")).map_err(|err| {
            format!(
                "failed to write retained exact-meet guide artifact rank {rank} to {}: {err}",
                output_path.display()
            )
        })?;
        outputs.push(EndpointWitnessGuideArtifactOutput {
            retained_rank: rank,
            path: output_path.display().to_string(),
            artifact_id,
        });
    }
    Ok(outputs)
}

fn build_endpoint_witness_inventory(
    request: &SearchRequest,
    surface: &EndpointExactMeetSurface,
    controls: &[EndpointWitnessLoadedControl],
    emitted_guide_artifacts: Vec<EndpointWitnessGuideArtifactOutput>,
) -> EndpointWitnessInventory {
    EndpointWitnessInventory {
        artifact_kind: "endpoint_exact_meet_witness_inventory",
        source: dyn_matrix_to_vecs(&request.source),
        target: dyn_matrix_to_vecs(&request.target),
        requested_cap: surface.requested_cap,
        retained_count: surface.retained.len(),
        orientation_status: "not_recorded",
        orientation_note: "retained exact-meet telemetry stores the canonical meeting state and reconstructed source-to-target path, but not the frontier side/orientation that produced the meet",
        controls_loaded: controls
            .iter()
            .map(|control| EndpointWitnessControlSummary {
                class: control.class.clone(),
                artifact_id: control.artifact_id.clone(),
                label: control.label.clone(),
                source_ref: control.source_ref.clone(),
                reconstructed_path_length: control.reconstructed_path_length,
                full_path_hash: control.full_path_hash.clone(),
            })
            .collect(),
        rows: surface
            .retained
            .iter()
            .enumerate()
            .map(|(index, witness)| endpoint_witness_inventory_row(index, witness, controls))
            .collect(),
        emitted_guide_artifacts,
    }
}

fn endpoint_witness_inventory_row(
    index: usize,
    witness: &EndpointExactMeetWitness,
    controls: &[EndpointWitnessLoadedControl],
) -> EndpointWitnessInventoryRow {
    let full_path_signature = witness_matrix_signature(&witness.path);
    let full_path_hash = stable_signature_hash(&full_path_signature);
    EndpointWitnessInventoryRow {
        retained_rank: index + 1,
        retained_index: index,
        meet_lag: witness.path_lag,
        reconstructed_path_length: witness.path.steps.len(),
        endpoint_orientation: "not_recorded",
        meeting_state_signature: matrix_signature(&witness.meeting_canonical),
        control_matches: controls
            .iter()
            .filter(|control| control.full_path_signature == full_path_signature)
            .map(|control| EndpointWitnessControlMatch {
                class: control.class.clone(),
                artifact_id: control.artifact_id.clone(),
                label: control.label.clone(),
                source_ref: control.source_ref.clone(),
            })
            .collect(),
        full_path_signature,
        full_path_hash,
    }
}

fn witness_matrix_signature(path: &DynSsePath) -> String {
    path.matrices
        .iter()
        .map(matrix_signature)
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn matrix_signature(matrix: &DynMatrix) -> String {
    let data = matrix
        .data
        .iter()
        .map(u32::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!("{}x{}:{data}", matrix.rows, matrix.cols)
}

fn stable_path_hash(path: &DynSsePath) -> String {
    stable_signature_hash(&witness_matrix_signature(path))
}

fn stable_signature_hash(signature: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in signature.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv1a64:{hash:016x}")
}

fn result_label(result: &SearchRunResult) -> &'static str {
    match result {
        SearchRunResult::Equivalent(_) => "equivalent",
        SearchRunResult::EquivalentByStructuredProof(proof) => proof.outcome_label(),
        SearchRunResult::NotEquivalent(_) => "not_equivalent",
        SearchRunResult::Unknown => "unknown",
    }
}

fn status_label(status: SearchEdgeStatus) -> &'static str {
    match status {
        SearchEdgeStatus::SeenCollision => "seen_collision",
        SearchEdgeStatus::Discovered => "discovered",
        SearchEdgeStatus::ExactMeet => "exact_meet",
    }
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
        SearchRunResult::EquivalentByStructuredProof(proof) => {
            println!("Result: EQUIVALENT ({})", proof.description());
        }
        SearchRunResult::NotEquivalent(reason) => {
            println!("Result: NOT EQUIVALENT");
            println!("Reason: {reason}");
        }
        SearchRunResult::Unknown => {
            println!("Result: UNKNOWN (search exhausted)");
        }
    }

    if let Some(surface) = telemetry.endpoint_exact_meets.as_ref() {
        println!();
        println!(
            "Retained endpoint exact meets: {} (cap {})",
            surface.retained.len(),
            surface.requested_cap
        );
        for (index, witness) in surface.retained.iter().enumerate() {
            println!(
                "  {}. lag {} via {}",
                index + 1,
                witness.path_lag,
                format_dyn_matrix(&witness.meeting_canonical)
            );
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
        "  shortcut segment cache hits: {}",
        telemetry.shortcut_search.segment_cache_hits
    );
    println!(
        "  shortcut segment cache misses: {}",
        telemetry.shortcut_search.segment_cache_misses
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
    let obj = build_result_json(a, b, stage, result, telemetry, show_telemetry);

    println!(
        "{}",
        serde_json::to_string_pretty(&obj).expect("json serialization")
    );
}

fn build_result_json(
    a: &DynMatrix,
    b: &DynMatrix,
    stage: SearchStage,
    result: &SearchRunResult,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) -> serde_json::Value {
    let (outcome, steps, reason, relation) = match result {
        SearchRunResult::Equivalent(path) => (
            "equivalent",
            Some(
                path.steps
                    .iter()
                    .map(step_json)
                    .collect::<Vec<serde_json::Value>>(),
            ),
            None,
            None,
        ),
        SearchRunResult::EquivalentByStructuredProof(proof) => (
            proof.outcome_label(),
            None,
            Some(proof.description()),
            proof.relation_label().map(str::to_string),
        ),
        SearchRunResult::NotEquivalent(reason) => {
            ("not_equivalent", None, Some(reason.clone()), None)
        }
        SearchRunResult::Unknown => ("unknown", None, None, None),
    };

    let obj = build_json_value(
        serde_json::json!(dyn_matrix_to_vecs(a)),
        serde_json::json!(dyn_matrix_to_vecs(b)),
        stage,
        outcome,
        steps,
        reason,
        relation,
        telemetry,
        show_telemetry,
    );

    obj
}

fn build_json_value(
    a: serde_json::Value,
    b: serde_json::Value,
    stage: SearchStage,
    outcome: &str,
    steps: Option<Vec<serde_json::Value>>,
    reason: Option<String>,
    relation: Option<String>,
    telemetry: &SearchTelemetry,
    show_telemetry: bool,
) -> serde_json::Value {
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
    if let Some(relation) = relation {
        obj["relation"] = serde_json::json!(relation);
    }
    if let Some(surface) = telemetry.endpoint_exact_meets.as_ref() {
        obj["endpoint_exact_meets"] = endpoint_exact_meets_json(surface);
    }
    if show_telemetry {
        obj["telemetry"] = serde_json::to_value(telemetry).unwrap_or_default();
    }

    obj
}

fn endpoint_exact_meets_json(surface: &EndpointExactMeetSurface) -> serde_json::Value {
    serde_json::json!({
        "requested_cap": surface.requested_cap,
        "retained": surface
            .retained
            .iter()
            .map(|witness| serde_json::json!({
                "path_lag": witness.path_lag,
                "meeting_canonical": dyn_matrix_to_vecs(&witness.meeting_canonical),
                "path": {
                    "matrices": witness
                        .path
                        .matrices
                        .iter()
                        .map(dyn_matrix_to_vecs)
                        .collect::<Vec<_>>(),
                    "steps": witness
                        .path
                        .steps
                        .iter()
                        .map(step_json)
                        .collect::<Vec<_>>(),
                },
            }))
            .collect::<Vec<_>>(),
    })
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
    use super::{
        build_endpoint_witness_inventory, build_result_json, parse_cli, parse_matrix,
        run_with_args, stable_path_hash, witness_matrix_signature,
        write_endpoint_witness_guide_artifacts, ApproximateHitParityObserver,
        EndpointWitnessLoadedControl,
    };
    use rusqlite::Connection;
    use sse_core::concrete_shift::{
        canonical_module_shift_witness_2x2, ConcreteShiftRelation2x2, ShiftEquivalenceWitness2x2,
    };
    use sse_core::endpoint_local_parity::EndpointLocalParityAction;
    use sse_core::guide_artifacts::load_guide_artifacts_from_path;
    use sse_core::matrix::{DynMatrix, SqMatrix};
    use sse_core::search_observer::{
        SearchEdgeRecord, SearchEdgeStatus, SearchEvent, SearchObserver, SearchRootRecord,
    };
    use sse_core::types::{
        ConcreteShiftProof2x2, DynSsePath, EndpointExactMeetSurface, EndpointExactMeetWitness,
        EsseStep, FrontierMode, GuideArtifact, GuideArtifactPayload, GuidedRefinementConfig,
        MoveFamilyPolicy, SearchConfig, SearchRequest, SearchRunResult, SearchStage,
        SearchTelemetry, ShortcutSearchConfig,
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
    fn parse_cli_accepts_approximate_hit_parity_report_flag() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--approximate-hit-parity-report".to_string(),
                "approximate-hit-parity.json".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            cli.approximate_hit_parity_report.as_deref(),
            Some("approximate-hit-parity.json")
        );
    }

    #[test]
    fn parse_cli_accepts_endpoint_multi_meet_cap_for_endpoint_bfs() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "0,1,1,0".to_string(),
                "--endpoint-multi-meet-cap".to_string(),
                "3".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.config.endpoint_multi_meet_cap, Some(3));
    }

    #[test]
    fn parse_cli_accepts_endpoint_witness_inventory_flags() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--endpoint-multi-meet-cap".to_string(),
                "2".to_string(),
                "--endpoint-witness-inventory".to_string(),
                "inventory.json".to_string(),
                "--endpoint-witness-control-guide".to_string(),
                "baker=research/guide_artifacts/k3_normalized_guide_pool.json#k3-lind-marcus-baker-lag7".to_string(),
                "--endpoint-witness-guide-dir".to_string(),
                "guides".to_string(),
                "--endpoint-witness-guide-ranks".to_string(),
                "2,1,1".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            cli.endpoint_witness_inventory.as_deref(),
            Some("inventory.json")
        );
        assert_eq!(cli.endpoint_witness_control_guides.len(), 1);
        assert_eq!(cli.endpoint_witness_control_guides[0].class, "baker");
        assert_eq!(
            cli.endpoint_witness_control_guides[0]
                .artifact_id
                .as_deref(),
            Some("k3-lind-marcus-baker-lag7")
        );
        assert_eq!(cli.endpoint_witness_guide_dir.as_deref(), Some("guides"));
        assert_eq!(cli.endpoint_witness_guide_ranks, Some(vec![1, 2]));
    }

    #[test]
    fn parse_cli_rejects_empty_endpoint_witness_control_path() {
        let err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--endpoint-multi-meet-cap".to_string(),
                "1".to_string(),
                "--endpoint-witness-control-guide".to_string(),
                "baker=#control".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert!(err.contains("path must not be empty"));
    }

    #[test]
    fn parse_cli_rejects_empty_endpoint_witness_control_artifact_id() {
        let err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--endpoint-multi-meet-cap".to_string(),
                "1".to_string(),
                "--endpoint-witness-inventory".to_string(),
                "inventory.json".to_string(),
                "--endpoint-witness-control-guide".to_string(),
                "baker=control.json#".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert!(err.contains("artifact id after # must not be empty"));
    }

    #[test]
    fn parse_cli_rejects_endpoint_witness_control_without_inventory() {
        let err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--endpoint-multi-meet-cap".to_string(),
                "1".to_string(),
                "--endpoint-witness-guide-dir".to_string(),
                "guides".to_string(),
                "--endpoint-witness-control-guide".to_string(),
                "baker=control.json".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert!(err.contains("requires --endpoint-witness-inventory"));
    }

    #[test]
    fn parse_cli_rejects_endpoint_multi_meet_cap_outside_endpoint_bfs() {
        let stage_err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "0,1,1,0".to_string(),
                "--endpoint-multi-meet-cap".to_string(),
                "2".to_string(),
                "--stage".to_string(),
                "shortcut-search".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();
        assert!(stage_err.contains("--stage endpoint-search"));

        let frontier_err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "0,1,1,0".to_string(),
                "--endpoint-multi-meet-cap".to_string(),
                "2".to_string(),
                "--frontier-mode".to_string(),
                "beam".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();
        assert!(frontier_err.contains("--frontier-mode bfs"));
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
    fn concrete_shift_json_includes_relation_field() {
        let a = SqMatrix::identity();
        let witness = canonical_module_shift_witness_2x2(
            &a,
            &a,
            ShiftEquivalenceWitness2x2 {
                lag: 1,
                r: a.clone(),
                s: a.clone(),
            },
        )
        .expect("identity should admit canonical witness");
        let proof = ConcreteShiftProof2x2 {
            relation: ConcreteShiftRelation2x2::Balanced,
            witness,
        };
        let dyn_a = DynMatrix::from_sq(&a);
        let json = build_result_json(
            &dyn_a,
            &dyn_a,
            SearchStage::EndpointSearch,
            &SearchRunResult::EquivalentByStructuredProof(proof.into()),
            &SearchTelemetry::default(),
            false,
        );

        assert_eq!(json["outcome"], "equivalent_by_concrete_shift");
        assert_eq!(json["reason"], "balanced concrete-shift witness");
        assert_eq!(json["relation"], "balanced");
    }

    #[test]
    fn endpoint_multi_meet_json_surface_uses_cli_friendly_shapes() {
        let a = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let b = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let mut telemetry = SearchTelemetry::default();
        telemetry.endpoint_exact_meets = Some(EndpointExactMeetSurface {
            requested_cap: 2,
            retained: vec![EndpointExactMeetWitness {
                path_lag: 1,
                meeting_canonical: DynMatrix::new(2, 2, vec![1, 1, 1, 1]),
                path: DynSsePath {
                    matrices: vec![a.clone(), b.clone()],
                    steps: vec![sse_core::types::EsseStep {
                        u: a.clone(),
                        v: b.clone(),
                    }],
                },
            }],
        });

        let json = build_result_json(
            &a,
            &b,
            SearchStage::EndpointSearch,
            &SearchRunResult::Equivalent(DynSsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: vec![],
            }),
            &telemetry,
            false,
        );

        assert_eq!(json["endpoint_exact_meets"]["requested_cap"], 2);
        assert_eq!(
            json["endpoint_exact_meets"]["retained"][0]["meeting_canonical"],
            serde_json::json!([[1, 1], [1, 1]])
        );
        assert_eq!(
            json["endpoint_exact_meets"]["retained"][0]["path"]["matrices"][0],
            serde_json::json!([[1, 0], [0, 1]])
        );
    }

    #[test]
    fn approximate_hit_parity_report_marks_trimmed_match_as_reuse() {
        let request = identity_request();
        let result = SearchRunResult::Unknown;
        let telemetry = SearchTelemetry {
            approximate_other_side_hits: 1,
            ..Default::default()
        };
        let root = SearchRootRecord {
            direction: sse_core::types::SearchDirection::Backward,
            canonical: k3_overlap_square().canonical_perm(),
            orig: k3_overlap_square(),
            depth: 2,
        };
        let edge = SearchEdgeRecord {
            layer_index: 1,
            direction: sse_core::types::SearchDirection::Forward,
            move_family: "square_factorisation_3x3",
            from_canonical: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            from_orig: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            to_canonical: k3_overlap_square().canonical_perm(),
            to_orig: k3_overlap_square(),
            from_depth: 1,
            to_depth: 2,
            step: EsseStep {
                u: DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]),
                v: DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]),
            },
            status: SearchEdgeStatus::Discovered,
            approximate_other_side_hit: true,
            enqueued: true,
        };

        let mut observer = ApproximateHitParityObserver::default();
        observer.on_event(&SearchEvent::Roots(vec![root]));
        observer.on_event(&SearchEvent::Layer(vec![edge]));

        let report = observer.build_report(&request, &result, &telemetry);
        assert_eq!(report.summary.supported_square_hits, 1);
        assert!(report.summary.report_is_complete);
        assert_eq!(report.summary.unattributed_approximate_hits, 0);
        assert_eq!(report.annotated_hits.len(), 1);
        assert_eq!(
            report.annotated_hits[0].best_action,
            EndpointLocalParityAction::ReuseEndpointLocalParity
        );
        assert_eq!(
            report.annotated_hits[0].bucket_candidates[0].action,
            EndpointLocalParityAction::ReuseEndpointLocalParity
        );
    }

    #[test]
    fn approximate_hit_parity_report_marks_coarse_only_square_match_as_rank() {
        let request = identity_request();
        let result = SearchRunResult::Unknown;
        let telemetry = SearchTelemetry {
            approximate_other_side_hits: 1,
            ..Default::default()
        };
        let root = SearchRootRecord {
            direction: sse_core::types::SearchDirection::Backward,
            canonical: rank4_counterpart_matrix().canonical_perm(),
            orig: rank4_counterpart_matrix(),
            depth: 3,
        };
        let edge = SearchEdgeRecord {
            layer_index: 2,
            direction: sse_core::types::SearchDirection::Forward,
            move_family: "diagonal_refactorization_4x4",
            from_canonical: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            from_orig: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            to_canonical: rank4_to_matrix().canonical_perm(),
            to_orig: rank4_to_matrix(),
            from_depth: 2,
            to_depth: 3,
            step: EsseStep {
                u: DynMatrix::new(4, 4, vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
                v: DynMatrix::new(4, 4, vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
            },
            status: SearchEdgeStatus::Discovered,
            approximate_other_side_hit: true,
            enqueued: true,
        };

        let mut observer = ApproximateHitParityObserver::default();
        observer.on_event(&SearchEvent::Roots(vec![root]));
        observer.on_event(&SearchEvent::Layer(vec![edge]));

        let report = observer.build_report(&request, &result, &telemetry);
        assert_eq!(report.summary.supported_square_hits, 1);
        assert!(report.summary.report_is_complete);
        assert_eq!(report.summary.unattributed_approximate_hits, 0);
        assert_eq!(report.annotated_hits.len(), 1);
        assert_eq!(
            report.annotated_hits[0].best_action,
            EndpointLocalParityAction::RankOrProposeInsideCoarseBucket
        );
        assert_eq!(
            report.annotated_hits[0].bucket_candidates[0].action,
            EndpointLocalParityAction::RankOrProposeInsideCoarseBucket
        );
    }

    #[test]
    fn approximate_hit_parity_report_flags_unattributed_hits() {
        let request = identity_request();
        let result = SearchRunResult::Unknown;
        let telemetry = SearchTelemetry {
            approximate_other_side_hits: 2,
            ..Default::default()
        };
        let root = SearchRootRecord {
            direction: sse_core::types::SearchDirection::Backward,
            canonical: rank4_counterpart_matrix().canonical_perm(),
            orig: rank4_counterpart_matrix(),
            depth: 3,
        };
        let edge = SearchEdgeRecord {
            layer_index: 2,
            direction: sse_core::types::SearchDirection::Forward,
            move_family: "diagonal_refactorization_4x4",
            from_canonical: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            from_orig: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            to_canonical: rank4_to_matrix().canonical_perm(),
            to_orig: rank4_to_matrix(),
            from_depth: 2,
            to_depth: 3,
            step: EsseStep {
                u: DynMatrix::new(4, 4, vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
                v: DynMatrix::new(4, 4, vec![1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1]),
            },
            status: SearchEdgeStatus::Discovered,
            approximate_other_side_hit: true,
            enqueued: true,
        };

        let mut observer = ApproximateHitParityObserver::default();
        observer.on_event(&SearchEvent::Roots(vec![root]));
        observer.on_event(&SearchEvent::Layer(vec![edge]));

        let report = observer.build_report(&request, &result, &telemetry);
        assert!(!report.summary.report_is_complete);
        assert_eq!(report.summary.unattributed_approximate_hits, 1);
        assert!(report.summary.completeness_note.is_some());
    }

    #[test]
    fn endpoint_witness_inventory_rows_include_hashes_and_control_matches() {
        let request = identity_request();
        let path = DynSsePath {
            matrices: vec![request.source.clone()],
            steps: vec![],
        };
        let surface = EndpointExactMeetSurface {
            requested_cap: 1,
            retained: vec![EndpointExactMeetWitness {
                path_lag: 3,
                meeting_canonical: request.source.clone(),
                path: path.clone(),
            }],
        };
        let controls = vec![EndpointWitnessLoadedControl {
            class: "baker".to_string(),
            artifact_id: Some("control".to_string()),
            label: Some("Baker control".to_string()),
            source_ref: "control.json#control".to_string(),
            reconstructed_path_length: 0,
            full_path_signature: witness_matrix_signature(&path),
            full_path_hash: stable_path_hash(&path),
        }];

        let inventory = build_endpoint_witness_inventory(&request, &surface, &controls, vec![]);

        assert_eq!(inventory.retained_count, 1);
        assert_eq!(inventory.orientation_status, "not_recorded");
        assert_eq!(inventory.rows[0].retained_rank, 1);
        assert_eq!(inventory.rows[0].retained_index, 0);
        assert_eq!(inventory.rows[0].meet_lag, 3);
        assert_eq!(inventory.rows[0].reconstructed_path_length, 0);
        assert_eq!(inventory.rows[0].meeting_state_signature, "2x2:1,0,0,1");
        assert!(inventory.rows[0].full_path_hash.starts_with("fnv1a64:"));
        assert_eq!(inventory.rows[0].control_matches[0].class, "baker");
    }

    #[test]
    fn endpoint_witness_guide_artifact_output_round_trips() {
        let request = identity_request();
        let surface = EndpointExactMeetSurface {
            requested_cap: 1,
            retained: vec![EndpointExactMeetWitness {
                path_lag: 0,
                meeting_canonical: request.source.clone(),
                path: DynSsePath {
                    matrices: vec![request.source.clone()],
                    steps: vec![],
                },
            }],
        };
        let dir = temp_output_path("endpoint-witness-guides");
        let outputs = write_endpoint_witness_guide_artifacts(
            &request,
            &surface,
            &dir.display().to_string(),
            Some(&[1]),
        )
        .unwrap();

        assert_eq!(outputs.len(), 1);
        let artifacts = load_guide_artifacts_from_path(&dir).unwrap();
        assert_eq!(artifacts.len(), 1);
        assert_eq!(
            artifacts[0].provenance.source_kind.as_deref(),
            Some("endpoint_exact_meet_inventory")
        );
        assert!(matches!(
            &artifacts[0].payload,
            GuideArtifactPayload::FullPath { path } if path.steps.is_empty()
        ));

        let _ = fs::remove_dir_all(dir);
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
                "bfs",
                "graph-plus-structured",
                FrontierMode::Bfs,
                MoveFamilyPolicy::GraphPlusStructured,
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
                "graph-plus-structured",
                FrontierMode::Beam,
                MoveFamilyPolicy::GraphPlusStructured,
                Some("8"),
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
    fn parse_cli_accepts_legacy_search_mode_graph_plus_structured() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--search-mode".to_string(),
                "graph-plus-structured".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.config.frontier_mode, FrontierMode::Bfs);
        assert_eq!(
            cli.config.move_family_policy,
            MoveFamilyPolicy::GraphPlusStructured
        );
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
            "--beam-width requires --frontier-mode beam, concrete-shift-profile-beam, witness-bridge-profile-beam, same-future-past-diversity-beam, beam-bfs-handoff, or stratified-beam-refill"
        );
    }

    #[test]
    fn parse_cli_accepts_beam_bfs_handoff_depth_with_handoff_mode() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--frontier-mode".to_string(),
                "beam-bfs-handoff".to_string(),
                "--beam-bfs-handoff-depth".to_string(),
                "6".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.config.frontier_mode, FrontierMode::BeamBfsHandoff);
        assert_eq!(cli.config.beam_bfs_handoff_depth, Some(6));
    }

    #[test]
    fn parse_cli_accepts_concrete_shift_profile_beam_mode() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--frontier-mode".to_string(),
                "concrete-shift-profile-beam".to_string(),
                "--beam-width".to_string(),
                "5".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            cli.config.frontier_mode,
            FrontierMode::ConcreteShiftProfileBeam
        );
        assert_eq!(cli.config.beam_width, Some(5));
    }

    #[test]
    fn parse_cli_accepts_same_future_past_diversity_beam_mode() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--frontier-mode".to_string(),
                "same-future-past-diversity-beam".to_string(),
                "--beam-width".to_string(),
                "5".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            cli.config.frontier_mode,
            FrontierMode::SameFuturePastDiversityBeam
        );
        assert_eq!(cli.config.beam_width, Some(5));
    }

    #[test]
    fn parse_cli_accepts_witness_bridge_profile_beam_mode() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--frontier-mode".to_string(),
                "witness-bridge-profile-beam".to_string(),
                "--beam-width".to_string(),
                "5".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(
            cli.config.frontier_mode,
            FrontierMode::WitnessBridgeProfileBeam
        );
        assert_eq!(cli.config.beam_width, Some(5));
    }

    #[test]
    fn parse_cli_accepts_beam_bfs_handoff_deferred_cap_with_handoff_mode() {
        let cli = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--frontier-mode".to_string(),
                "beam-bfs-handoff".to_string(),
                "--beam-bfs-handoff-deferred-cap".to_string(),
                "24".to_string(),
            ]
            .into_iter(),
        )
        .unwrap();

        assert_eq!(cli.config.frontier_mode, FrontierMode::BeamBfsHandoff);
        assert_eq!(cli.config.beam_bfs_handoff_deferred_cap, Some(24));
    }

    #[test]
    fn parse_cli_rejects_beam_bfs_handoff_depth_without_handoff_mode() {
        let err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--beam-bfs-handoff-depth".to_string(),
                "6".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            "--beam-bfs-handoff-depth requires --frontier-mode beam-bfs-handoff"
        );
    }

    #[test]
    fn parse_cli_rejects_beam_bfs_handoff_deferred_cap_without_handoff_mode() {
        let err = parse_cli(
            vec![
                "1,0,0,1".to_string(),
                "1,0,0,1".to_string(),
                "--beam-bfs-handoff-deferred-cap".to_string(),
                "24".to_string(),
            ]
            .into_iter(),
        )
        .unwrap_err();

        assert_eq!(
            err,
            "--beam-bfs-handoff-deferred-cap requires --frontier-mode beam-bfs-handoff or stratified-beam-refill"
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

    fn identity_request() -> SearchRequest {
        let matrix = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        SearchRequest {
            source: matrix.clone(),
            target: matrix,
            config: SearchConfig::default(),
            stage: SearchStage::EndpointSearch,
            guide_artifacts: Vec::new(),
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        }
    }

    fn k3_overlap_square() -> DynMatrix {
        DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0])
    }

    fn rank4_to_matrix() -> DynMatrix {
        DynMatrix::new(4, 4, vec![1, 4, 2, 7, 3, 1, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0])
    }

    fn rank4_counterpart_matrix() -> DynMatrix {
        DynMatrix::new(4, 4, vec![1, 12, 0, 1, 1, 1, 4, 4, 0, 0, 0, 0, 0, 0, 0, 0])
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
