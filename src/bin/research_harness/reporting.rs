use sse_core::types::SearchStage;

use super::{CaseSummary, ComparisonSummary, HarnessSummary, MeasurementSummary};

pub(crate) fn format_pretty_summary(summary: &HarnessSummary) -> String {
    let mut out = String::new();
    push_overview(&mut out, summary);
    push_comparisons(&mut out, &summary.comparisons);
    push_campaigns(&mut out, summary);
    push_strategies(&mut out, summary);

    for case in &summary.cases {
        push_case(&mut out, case);
    }

    out
}

fn push_overview(out: &mut String, summary: &HarnessSummary) {
    out.push_str("Research Harness\n");
    out.push_str(&format!("cases: {}\n", summary.cases_path));
    out.push_str(&format!(
        "reused_history_sources: {}\n",
        summary.reused_history_sources
    ));
    out.push_str(&format!(
        "required_passes: {}/{}\n",
        summary.fitness.passed_required_cases, summary.fitness.required_cases
    ));
    out.push_str(&format!(
        "non_required_cases: {}\n",
        summary.fitness.non_required_cases
    ));
    out.push_str(&format!("target_hits: {}\n", summary.fitness.target_hits));
    out.push_str(&format!("total_points: {}\n", summary.fitness.total_points));
    out.push_str(&format!(
        "current_lag_score: {} across {} witness case(s), total lag {}\n",
        summary.fitness.current_lag_score,
        summary.fitness.current_witness_cases,
        summary.fitness.current_witness_lag_total
    ));
    out.push_str(&format!(
        "best_known_lag_score: {} across {} witness case(s), total lag {}, improvements {}\n",
        summary.fitness.best_known_lag_score,
        summary.fitness.best_known_witness_cases,
        summary.fitness.best_known_witness_lag_total,
        summary.fitness.best_known_improvements
    ));
    out.push_str(&format!(
        "generalized_cases: {}\n",
        summary.fitness.generalized_cases
    ));
    out.push_str(&format!(
        "comparison_groups: {}\n",
        summary.fitness.comparison_groups
    ));
    out.push_str(&format!(
        "campaign_groups: {}\n",
        summary.fitness.campaign_groups
    ));
    out.push_str(&format!(
        "strategy_groups: {}\n",
        summary.fitness.strategy_groups
    ));
    out.push_str(&format!(
        "telemetry_focus_score: {} across {} case(s)\n",
        summary.fitness.telemetry_focus_score, summary.fitness.telemetry_focus_cases
    ));
    out.push_str(&format!(
        "telemetry_focus_directed_score: {}\n",
        summary.fitness.telemetry_focus_directed_score
    ));
    out.push_str(&format!(
        "total_elapsed_ms: {}\n\n",
        summary.fitness.total_elapsed_ms
    ));
}

fn push_comparisons(out: &mut String, comparisons: &[ComparisonSummary]) {
    if comparisons.is_empty() {
        return;
    }

    out.push_str("Comparisons\n");
    for comparison in comparisons {
        out.push_str(&format!(
            "- endpoints={}x{} variants={}\n",
            comparison.endpoint.source_dim,
            comparison.endpoint.target_dim,
            comparison.variants.len()
        ));
        for variant in &comparison.variants {
            out.push_str(&format!(
                "  {}: strategy={} stage_combo={} frontier={:?} beam_width={:?} move_family_policy={:?} stage={:?} max_lag={} max_dim={} max_entry={} outcome={} resolution={:?} witness_lag={:?} best_known_lag={:?} improved_best={} points={} elapsed={}ms",
                variant.case_id,
                variant
                    .campaign
                    .as_ref()
                    .map(|campaign| campaign.strategy.as_str())
                    .unwrap_or("-"),
                variant.stage_combination,
                variant.config.frontier_mode,
                variant.config.beam_width,
                variant.config.move_family_policy,
                variant.config.stage,
                variant.config.max_lag,
                variant.config.max_intermediate_dim,
                variant.config.max_entry,
                variant.actual_outcome,
                variant.result_model.resolution_kind,
                variant.result_model.witness_lag,
                variant.best_known_witness.as_ref().map(|best| best.lag),
                variant.improved_best_known_witness,
                variant.points,
                variant.elapsed_ms,
            ));
            if let Some(measurement) = &variant.measurement {
                out.push_str(&format!(
                    " measurement=[{}]",
                    format_measurement_summary(measurement)
                ));
            }
            out.push('\n');
        }
    }
    out.push('\n');
}

fn push_campaigns(out: &mut String, summary: &HarnessSummary) {
    if summary.campaigns.is_empty() {
        return;
    }

    out.push_str("Campaigns\n");
    for campaign in &summary.campaigns {
        out.push_str(&format!(
            "- {}: cases={} current_lag_score={} best_known_lag_score={} points={} elapsed={}ms\n",
            campaign.id,
            campaign.cases,
            campaign.current_lag_score,
            campaign.best_known_lag_score,
            campaign.total_points,
            campaign.total_elapsed_ms,
        ));
        for scheduled_case in &campaign.scheduled_cases {
            out.push_str(&format!(
                "  order={} {} strategy={} outcome={} current_lag={:?} best_known_lag={:?} improved_best={} points={} elapsed={}ms",
                scheduled_case.schedule_order,
                scheduled_case.case_id,
                scheduled_case.strategy,
                scheduled_case.actual_outcome,
                scheduled_case.current_witness_lag,
                scheduled_case.best_known_witness.as_ref().map(|best| best.lag),
                scheduled_case.improved_best_known_witness,
                scheduled_case.points,
                scheduled_case.elapsed_ms,
            ));
            if let Some(measurement) = &scheduled_case.measurement {
                out.push_str(&format!(
                    " measurement=[{}]",
                    format_measurement_summary(measurement)
                ));
            }
            out.push('\n');
        }
    }
    out.push('\n');
}

fn push_strategies(out: &mut String, summary: &HarnessSummary) {
    if summary.strategies.is_empty() {
        return;
    }

    out.push_str("Strategies\n");
    for strategy in &summary.strategies {
        out.push_str(&format!(
            "- {}: cases={} campaigns={} passes={} target_hits={} points={} current_lag_score={} best_known_lag_score={} improvements={} elapsed={}ms\n",
            strategy.strategy,
            strategy.cases,
            strategy.campaigns,
            strategy.passed_required_cases,
            strategy.target_hits,
            strategy.total_points,
            strategy.current_lag_score,
            strategy.best_known_lag_score,
            strategy.best_known_improvements,
            strategy.total_elapsed_ms,
        ));
    }
    out.push('\n');
}

fn push_case(out: &mut String, case: &CaseSummary) {
    out.push_str(&format!(
        "- {}: required={} outcome={} passed={} target={} points={} elapsed={}ms\n",
        case.id,
        case.required,
        case.actual_outcome,
        case.passed,
        case.hit_target,
        case.points,
        case.elapsed_ms
    ));
    out.push_str(&format!(
        "  endpoints: {}x{} config: frontier={:?} beam_width={:?} move_family_policy={:?} stage={:?} max_lag={} max_dim={} max_entry={} timeout={}ms\n",
        case.endpoint.source_dim,
        case.endpoint.target_dim,
        case.config.frontier_mode,
        case.config.beam_width,
        case.config.move_family_policy,
        case.config.stage,
        case.config.max_lag,
        case.config.max_intermediate_dim,
        case.config.max_entry,
        case.timeout_ms,
    ));
    if case.config.stage == SearchStage::GuidedRefinement {
        out.push_str(&format!(
            "  guided_refinement: max_shortcut_lag={} min_gap={} max_gap={:?} rounds={} segment_timeout_secs={:?}\n",
            case.config.guided_refinement.max_shortcut_lag,
            case.config.guided_refinement.min_gap,
            case.config.guided_refinement.max_gap,
            case.config.guided_refinement.rounds,
            case.config.guided_refinement.segment_timeout_secs,
        ));
    } else if case.config.stage == SearchStage::ShortcutSearch {
        out.push_str(&format!(
            "  shortcut_search: max_guides={} rounds={} max_total_segment_attempts={} emit_promoted_guides={} emitted_supported_stages={:?}\n",
            case.config.shortcut_search.max_guides,
            case.config.shortcut_search.rounds,
            case.config.shortcut_search.max_total_segment_attempts,
            case.config.shortcut_search.artifacts.emit_promoted_guides,
            case.config.shortcut_search.artifacts.supported_stages,
        ));
    }
    out.push_str(&format!(
        "  result: solver={:?} resolution={:?} witness_lag={:?} path_matrix_count={:?} frontier_layers={}\n",
        case.result_model.solver_path,
        case.result_model.resolution_kind,
        case.result_model.witness_lag,
        case.result_model.path_matrix_count,
        case.result_model.frontier_layers,
    ));
    if let Some(reason) = &case.reason {
        out.push_str(&format!("  reason: {}\n", reason));
    }
    if let Some(campaign) = &case.campaign {
        out.push_str(&format!(
            "  campaign: id={} strategy={} order={}\n",
            campaign.id, campaign.strategy, campaign.schedule_order
        ));
    }
    if let Some(endpoint_fixture) = &case.endpoint_fixture {
        out.push_str(&format!("  endpoint_fixture: {}\n", endpoint_fixture));
    }
    if !case.seeded_guide_ids.is_empty() || !case.guide_artifact_paths.is_empty() {
        out.push_str(&format!(
            "  guide_inputs: seeded={:?} artifacts={:?}\n",
            case.seeded_guide_ids, case.guide_artifact_paths
        ));
    }
    if let Some(best_known) = &case.best_known_witness {
        out.push_str(&format!(
            "  best_known_witness: lag={} elapsed={}ms source={} improved_best={}\n",
            best_known.lag,
            best_known.elapsed_ms,
            best_known.source,
            case.improved_best_known_witness,
        ));
    }
    if let Some(measurement) = &case.measurement {
        out.push_str(&format!(
            "  measurement: {}\n",
            format_measurement_summary(measurement)
        ));
    }
    out.push_str(&format!(
        "  telemetry: layers={} expanded={} factorisations={} kept={} pruned_size={} pruned_spectrum={} discovered={} approx_hits={} dead_ends={} seen_collisions={} max_frontier={} visited={}\n",
        case.telemetry.layers.len(),
        case.telemetry.frontier_nodes_expanded,
        case.telemetry.factorisations_enumerated,
        case.telemetry.candidates_after_pruning,
        case.telemetry.pruned_by_size,
        case.telemetry.pruned_by_spectrum,
        case.telemetry.discovered_nodes,
        case.telemetry.approximate_other_side_hits,
        case.telemetry.dead_end_nodes,
        case.telemetry.collisions_with_seen,
        case.telemetry.max_frontier_size,
        case.telemetry.total_visited_nodes,
    ));
    out.push_str(&format!(
        "  telemetry_summary: bottleneck={} productive_layers={} deepest_productive_layer={:?} stagnant_layer={:?} exhausted_early={} focus_progress_score={} directed_progress_score={} dead_end_ratio={:.3}\n",
        case.telemetry_summary.terminal_bottleneck,
        case.telemetry_summary.productive_layers,
        case.telemetry_summary.deepest_productive_layer,
        case.telemetry_summary.first_stagnant_layer,
        case.telemetry_summary.exhausted_before_max_lag,
        case.telemetry_summary.focus_progress_score,
        case.telemetry_summary.directed_progress_score,
        case.telemetry_summary.avg_dead_end_ratio,
    ));
    if case.tags.iter().any(|tag| tag == "telemetry-focus") {
        for layer in &case.telemetry.layers {
            out.push_str(&format!(
                "    layer {} {:?}: frontier={} factors={} kept={} discovered={} approx_hits={} dead_ends={} next={} meet={} seen_collisions={}\n",
                layer.layer_index,
                layer.direction,
                layer.frontier_nodes,
                layer.factorisations_enumerated,
                layer.candidates_after_pruning,
                layer.discovered_nodes,
                layer.approximate_other_side_hits,
                layer.dead_end_nodes,
                layer.next_frontier_nodes,
                layer.collisions_with_other_frontier,
                layer.collisions_with_seen,
            ));
        }
    }
}

fn format_measurement_summary(summary: &MeasurementSummary) -> String {
    format!(
        "warmups={} repeats={} min={}ms median={}ms p90={}ms max={}ms outcomes={:?} samples={:?}",
        summary.warmup_runs,
        summary.repeat_runs,
        summary.min_elapsed_ms,
        summary.median_elapsed_ms,
        summary.p90_elapsed_ms,
        summary.max_elapsed_ms,
        summary.outcome_counts,
        summary.elapsed_samples_ms,
    )
}
