use std::env;
use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use sse_core::search::{execute_search_request, validate_sse_path_dyn};
use sse_core::types::{
    FrontierMode, SearchConfig, SearchRequest, SearchRunResult, SearchTelemetry, DEFAULT_BEAM_WIDTH,
};

use super::{
    case_matrix, normalized_measurement_config, resolve_case, HarnessSolverPath, MeasurementConfig,
    MeasurementSummary, ResearchCase, ResultModel, ResultResolutionKind, WorkerCaseResult,
};

#[derive(Debug)]
pub(crate) struct ExecutedCase {
    pub(crate) representative: WorkerCaseResult,
    pub(crate) measurement: Option<MeasurementSummary>,
}

pub(crate) fn run_case(case: &ResearchCase, cases_path: &Path) -> WorkerCaseResult {
    let started = Instant::now();
    let resolved = match resolve_case(case, cases_path) {
        Ok(resolved) => resolved,
        Err(reason) => {
            return panic_result(
                &case.id,
                case.a.len(),
                case.b.len(),
                started.elapsed().as_millis(),
                reason,
            )
        }
    };

    let a = match case_matrix(&resolved.endpoint.a) {
        Ok(matrix) => matrix,
        Err(reason) => {
            return panic_result(
                &case.id,
                resolved.endpoint.source_dim,
                resolved.endpoint.target_dim,
                started.elapsed().as_millis(),
                format!("invalid matrix A in corpus: {reason}"),
            )
        }
    };
    let b = match case_matrix(&resolved.endpoint.b) {
        Ok(matrix) => matrix,
        Err(reason) => {
            return panic_result(
                &case.id,
                resolved.endpoint.source_dim,
                resolved.endpoint.target_dim,
                started.elapsed().as_millis(),
                format!("invalid matrix B in corpus: {reason}"),
            )
        }
    };

    let normalized_beam_width =
        match normalized_beam_width(case.config.frontier_mode, case.config.beam_width) {
            Ok(width) => width,
            Err(reason) => {
                return panic_result(
                    &case.id,
                    resolved.endpoint.source_dim,
                    resolved.endpoint.target_dim,
                    started.elapsed().as_millis(),
                    reason,
                )
            }
        };

    let request = SearchRequest {
        source: a.clone(),
        target: b.clone(),
        config: SearchConfig {
            max_lag: case.config.max_lag,
            max_intermediate_dim: case.config.max_intermediate_dim,
            max_entry: case.config.max_entry,
            frontier_mode: case.config.frontier_mode,
            move_family_policy: case.config.move_family_policy,
            beam_width: normalized_beam_width,
        },
        stage: case.config.stage,
        guide_artifacts: resolved.guide_artifacts,
        guided_refinement: case.config.guided_refinement.clone(),
        shortcut_search: case.config.shortcut_search.clone(),
    };

    let (result, telemetry) = match execute_search_request(&request) {
        Ok(outcome) => outcome,
        Err(reason) => {
            return panic_result(
                &case.id,
                a.rows,
                b.rows,
                started.elapsed().as_millis(),
                reason,
            )
        }
    };

    match result {
        SearchRunResult::Equivalent(path) => match validate_sse_path_dyn(&a, &b, &path) {
            Ok(()) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "equivalent".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: Some(path.steps.len()),
                reason: None,
                result_model: equivalent_result_model(
                    solver_path_for_dims(a.rows, b.rows),
                    a.rows,
                    b.rows,
                    &telemetry,
                    path.steps.len(),
                    path.matrices.len(),
                ),
                telemetry,
            },
            Err(reason) => WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "panic".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: Some(path.steps.len()),
                reason: Some(format!("invalid equivalent path: {reason}")),
                result_model: result_model(
                    solver_path_for_dims(a.rows, b.rows),
                    a.rows,
                    b.rows,
                    ResultResolutionKind::InvalidPath,
                    Some(path.steps.len()),
                    Some(path.matrices.len()),
                    &telemetry,
                ),
                telemetry,
            },
        },
        SearchRunResult::EquivalentByConcreteShift(proof) => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "equivalent".to_string(),
            elapsed_ms: started.elapsed().as_millis(),
            steps: None,
            reason: Some(proof.description()),
            result_model: result_model(
                solver_path_for_dims(a.rows, b.rows),
                a.rows,
                b.rows,
                ResultResolutionKind::ConcreteShiftWitness,
                Some(proof.witness.shift.lag as usize),
                None,
                &telemetry,
            ),
            telemetry,
        },
        SearchRunResult::NotEquivalent(reason) => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "not_equivalent".to_string(),
            elapsed_ms: started.elapsed().as_millis(),
            steps: None,
            reason: Some(reason),
            result_model: not_equivalent_result_model(
                solver_path_for_dims(a.rows, b.rows),
                a.rows,
                b.rows,
                &telemetry,
            ),
            telemetry,
        },
        SearchRunResult::Unknown => WorkerCaseResult {
            id: case.id.clone(),
            actual_outcome: "unknown".to_string(),
            elapsed_ms: started.elapsed().as_millis(),
            steps: None,
            reason: None,
            result_model: result_model(
                solver_path_for_dims(a.rows, b.rows),
                a.rows,
                b.rows,
                ResultResolutionKind::SearchExhausted,
                None,
                None,
                &telemetry,
            ),
            telemetry,
        },
    }
}

pub(crate) fn execute_case_for_harness<F>(
    case: &ResearchCase,
    mut run_attempt: F,
) -> Result<ExecutedCase, String>
where
    F: FnMut() -> Result<WorkerCaseResult, String>,
{
    let Some(measurement) = normalized_measurement_config(case)? else {
        return Ok(ExecutedCase {
            representative: run_attempt()?,
            measurement: None,
        });
    };

    for _ in 0..measurement.warmup_runs {
        run_attempt()?;
    }

    let mut measured_results = Vec::with_capacity(measurement.repeat_runs);
    for _ in 0..measurement.repeat_runs {
        measured_results.push(run_attempt()?);
    }

    let measurement_summary = summarize_measurement_results(&measurement, &measured_results);
    let representative = measured_results
        .iter()
        .find(|result| result.elapsed_ms == measurement_summary.median_elapsed_ms)
        .cloned()
        .unwrap_or_else(|| {
            measured_results
                .first()
                .expect("measurement.repeat_runs should guarantee measured results")
                .clone()
        });

    Ok(ExecutedCase {
        representative,
        measurement: Some(measurement_summary),
    })
}

pub(crate) fn run_case_in_subprocess(
    current_exe: &Path,
    cases_path: &Path,
    case: &ResearchCase,
) -> Result<WorkerCaseResult, String> {
    let worker_output_path = env::temp_dir().join(format!(
        "research-harness-worker-{}-{}.json",
        std::process::id(),
        case.id
    ));
    let resolved = resolve_case(case, cases_path).ok();
    let source_dim = resolved
        .as_ref()
        .map(|resolved| resolved.endpoint.source_dim)
        .unwrap_or_else(|| case.a.len());
    let target_dim = resolved
        .as_ref()
        .map(|resolved| resolved.endpoint.target_dim)
        .unwrap_or_else(|| case.b.len());

    let mut child = Command::new(current_exe)
        .arg("--cases")
        .arg(cases_path)
        .arg("--worker-case")
        .arg(&case.id)
        .arg("--worker-output")
        .arg(&worker_output_path)
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|err| format!("failed to spawn worker for {}: {err}", case.id))?;

    let started = Instant::now();
    let timeout = Duration::from_millis(case.timeout_ms);

    loop {
        if started.elapsed() > timeout {
            child
                .kill()
                .map_err(|err| format!("failed to kill timed out worker {}: {err}", case.id))?;
            let output = child
                .wait_with_output()
                .map_err(|err| format!("failed to collect timed out worker {}: {err}", case.id))?;
            let _ = fs::remove_file(&worker_output_path);
            let reason = stderr_snippet(&output.stderr);
            return Ok(WorkerCaseResult {
                id: case.id.clone(),
                actual_outcome: "timeout".to_string(),
                elapsed_ms: started.elapsed().as_millis(),
                steps: None,
                reason: reason.or_else(|| Some(format!("worker exceeded {} ms", case.timeout_ms))),
                result_model: result_model(
                    solver_path_for_dims(source_dim, target_dim),
                    source_dim,
                    target_dim,
                    ResultResolutionKind::Timeout,
                    None,
                    None,
                    &SearchTelemetry::default(),
                ),
                telemetry: SearchTelemetry::default(),
            });
        }

        match child
            .try_wait()
            .map_err(|err| format!("failed to poll worker {}: {err}", case.id))?
        {
            Some(status) => {
                let output = child.wait_with_output().map_err(|err| {
                    format!("failed to collect finished worker {}: {err}", case.id)
                })?;

                if !status.success() {
                    let _ = fs::remove_file(&worker_output_path);
                    return Ok(WorkerCaseResult {
                        id: case.id.clone(),
                        actual_outcome: "panic".to_string(),
                        elapsed_ms: started.elapsed().as_millis(),
                        steps: None,
                        reason: stderr_snippet(&output.stderr)
                            .or_else(|| Some(format!("worker exited with status {status}"))),
                        result_model: result_model(
                            solver_path_for_dims(source_dim, target_dim),
                            source_dim,
                            target_dim,
                            ResultResolutionKind::Panic,
                            None,
                            None,
                            &SearchTelemetry::default(),
                        ),
                        telemetry: SearchTelemetry::default(),
                    });
                }

                let worker_output = fs::read_to_string(&worker_output_path).map_err(|err| {
                    format!(
                        "worker {} did not produce readable output {}: {err}",
                        case.id,
                        worker_output_path.display()
                    )
                })?;
                let _ = fs::remove_file(&worker_output_path);
                let parsed: WorkerCaseResult = serde_json::from_str(worker_output.trim())
                    .map_err(|err| format!("worker {} produced invalid json: {err}", case.id))?;
                return Ok(parsed);
            }
            None => thread::sleep(Duration::from_millis(10)),
        }
    }
}

fn summarize_measurement_results(
    measurement: &MeasurementConfig,
    results: &[WorkerCaseResult],
) -> MeasurementSummary {
    let mut elapsed_samples_ms = results
        .iter()
        .map(|result| result.elapsed_ms)
        .collect::<Vec<_>>();
    elapsed_samples_ms.sort_unstable();

    let mut outcome_counts = std::collections::BTreeMap::<String, usize>::new();
    for result in results {
        *outcome_counts
            .entry(result.actual_outcome.clone())
            .or_default() += 1;
    }

    MeasurementSummary {
        warmup_runs: measurement.warmup_runs,
        repeat_runs: measurement.repeat_runs,
        min_elapsed_ms: *elapsed_samples_ms
            .first()
            .expect("measurement.repeat_runs should guarantee elapsed samples"),
        median_elapsed_ms: nearest_rank_value(&elapsed_samples_ms, 1, 2),
        p90_elapsed_ms: nearest_rank_value(&elapsed_samples_ms, 9, 10),
        max_elapsed_ms: *elapsed_samples_ms
            .last()
            .expect("measurement.repeat_runs should guarantee elapsed samples"),
        elapsed_samples_ms,
        outcome_counts,
    }
}

fn nearest_rank_value(sorted_values: &[u128], numerator: usize, denominator: usize) -> u128 {
    let rank = (sorted_values.len() * numerator)
        .div_ceil(denominator)
        .max(1);
    sorted_values[rank - 1]
}

fn normalized_beam_width(
    frontier_mode: FrontierMode,
    beam_width: Option<usize>,
) -> Result<Option<usize>, String> {
    if !frontier_mode.uses_beam_width() {
        if beam_width.is_some() {
            return Err(
                "beam_width requires frontier_mode to be beam or beam_bfs_handoff".to_string(),
            );
        }
        return Ok(None);
    }
    match beam_width {
        Some(0) => Err("beam_width must be at least 1".to_string()),
        Some(width) => Ok(Some(width)),
        None => Ok(Some(DEFAULT_BEAM_WIDTH)),
    }
}

fn panic_result(
    case_id: &str,
    source_dim: usize,
    target_dim: usize,
    elapsed_ms: u128,
    reason: String,
) -> WorkerCaseResult {
    WorkerCaseResult {
        id: case_id.to_string(),
        actual_outcome: "panic".to_string(),
        elapsed_ms,
        steps: None,
        reason: Some(reason),
        result_model: result_model(
            solver_path_for_dims(source_dim, target_dim),
            source_dim,
            target_dim,
            ResultResolutionKind::Panic,
            None,
            None,
            &SearchTelemetry::default(),
        ),
        telemetry: SearchTelemetry::default(),
    }
}

fn result_model(
    solver_path: HarnessSolverPath,
    source_dim: usize,
    target_dim: usize,
    resolution_kind: ResultResolutionKind,
    witness_lag: Option<usize>,
    path_matrix_count: Option<usize>,
    telemetry: &SearchTelemetry,
) -> ResultModel {
    ResultModel {
        solver_path,
        source_dim,
        target_dim,
        resolution_kind,
        witness_lag,
        path_matrix_count,
        frontier_layers: telemetry.layers.len(),
    }
}

fn solver_path_for_dims(source_dim: usize, target_dim: usize) -> HarnessSolverPath {
    if source_dim == 2 && target_dim == 2 {
        HarnessSolverPath::TwoByTwo
    } else {
        HarnessSolverPath::SquareEndpoint
    }
}

fn equivalent_result_model(
    solver_path: HarnessSolverPath,
    source_dim: usize,
    target_dim: usize,
    telemetry: &SearchTelemetry,
    witness_lag: usize,
    path_matrix_count: usize,
) -> ResultModel {
    let resolution_kind = if witness_lag == 0 {
        ResultResolutionKind::Identity
    } else if telemetry.permutation_shortcut {
        ResultResolutionKind::PermutationShortcut
    } else if telemetry.canonical_shortcut {
        ResultResolutionKind::CanonicalShortcut
    } else {
        ResultResolutionKind::FrontierPath
    };
    result_model(
        solver_path,
        source_dim,
        target_dim,
        resolution_kind,
        Some(witness_lag),
        Some(path_matrix_count),
        telemetry,
    )
}

fn not_equivalent_result_model(
    solver_path: HarnessSolverPath,
    source_dim: usize,
    target_dim: usize,
    telemetry: &SearchTelemetry,
) -> ResultModel {
    let resolution_kind = if telemetry.invariant_filtered {
        ResultResolutionKind::InvariantFilterNotEquivalent
    } else {
        ResultResolutionKind::SearchNotEquivalent
    };
    result_model(
        solver_path,
        source_dim,
        target_dim,
        resolution_kind,
        None,
        None,
        telemetry,
    )
}

fn stderr_snippet(stderr: &[u8]) -> Option<String> {
    let text = String::from_utf8_lossy(stderr);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.lines().take(6).collect::<Vec<_>>().join(" | "))
    }
}
