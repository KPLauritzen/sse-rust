use sse_core::matrix::DynMatrix;
use sse_core::types::{
    EndpointExactMeetSurface, SearchDirection, SearchRunResult, SearchStage, SearchTelemetry,
};

pub(super) fn print_pretty(
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

pub(super) fn print_json(
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

pub(super) fn build_result_json(
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

    build_json_value(
        serde_json::json!(dyn_matrix_to_vecs(a)),
        serde_json::json!(dyn_matrix_to_vecs(b)),
        stage,
        outcome,
        steps,
        reason,
        relation,
        telemetry,
        show_telemetry,
    )
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
                "meet_direction": witness.meet_direction.map(search_direction_label),
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

fn search_direction_label(direction: SearchDirection) -> &'static str {
    match direction {
        SearchDirection::Forward => "forward",
        SearchDirection::Backward => "backward",
    }
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
