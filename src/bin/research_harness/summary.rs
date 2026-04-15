use std::collections::BTreeMap;
use std::env;
use std::path::Path;

use sse_core::types::{SearchLayerTelemetry, SearchStage, SearchTelemetry};

use super::execution::{execute_case_for_harness, run_case_in_subprocess};
use super::{
    endpoint_identity_key, merge_best_known_witness, resolve_case, BestKnownWitness,
    CampaignCaseSummary, CampaignSummary, CaseCorpus, CaseSummary, ComparisonSummary,
    ComparisonVariantSummary, DerivedTelemetrySummary, FitnessSummary, HarnessSummary,
    JsonSearchConfig, ResearchCase, ReusedResults, StrategySummary,
};

pub(crate) fn run_harness(
    cases_path: &Path,
    corpus: &CaseCorpus,
    reused_results: &ReusedResults,
) -> Result<HarnessSummary, String> {
    let current_exe =
        env::current_exe().map_err(|err| format!("failed to resolve current executable: {err}"))?;
    let mut accumulator = HarnessAccumulator::new(cases_path, corpus, reused_results);

    for case in scheduled_cases(corpus) {
        let case_summary = summarize_case(&current_exe, cases_path, reused_results, case)?;
        accumulator.record_case(case_summary);
    }

    Ok(accumulator.finish())
}

pub(crate) fn build_comparison_summaries(cases: &[CaseSummary]) -> Vec<ComparisonSummary> {
    let mut groups = Vec::<ComparisonSummary>::new();
    let mut group_indices = BTreeMap::<String, usize>::new();

    for case in cases {
        let key = endpoint_identity_key(&case.endpoint);

        let variant = ComparisonVariantSummary {
            case_id: case.id.clone(),
            description: case.description.clone(),
            campaign: case.campaign.clone(),
            measurement: case.measurement.clone(),
            stage_combination: stage_combination_label(&case.config),
            config: case.config.clone(),
            actual_outcome: case.actual_outcome.clone(),
            result_model: case.result_model.clone(),
            best_known_witness: case.best_known_witness.clone(),
            improved_best_known_witness: case.improved_best_known_witness,
            passed: case.passed,
            hit_target: case.hit_target,
            points: case.points,
            elapsed_ms: case.elapsed_ms,
        };

        match group_indices.get(&key).copied() {
            Some(index) => groups[index].variants.push(variant),
            None => {
                group_indices.insert(key, groups.len());
                groups.push(ComparisonSummary {
                    endpoint: case.endpoint.clone(),
                    variants: vec![variant],
                });
            }
        }
    }

    groups
        .into_iter()
        .filter(|group| group.variants.len() > 1)
        .collect()
}

pub(crate) fn build_campaign_summaries(cases: &[CaseSummary]) -> Vec<CampaignSummary> {
    let mut groups = BTreeMap::<String, Vec<&CaseSummary>>::new();
    for case in cases {
        let Some(campaign) = case.campaign.as_ref() else {
            continue;
        };
        groups.entry(campaign.id.clone()).or_default().push(case);
    }

    groups
        .into_iter()
        .map(|(id, mut group_cases)| {
            group_cases.sort_by_key(|case| {
                let campaign = case
                    .campaign
                    .as_ref()
                    .expect("campaign grouping only contains campaign cases");
                (
                    campaign.schedule_order,
                    campaign.strategy.clone(),
                    case.id.clone(),
                )
            });

            let mut current_witness_cases = 0usize;
            let mut current_witness_lag_total = 0usize;
            let mut best_known_witness_cases = 0usize;
            let mut best_known_witness_lag_total = 0usize;
            let mut total_points = 0i64;
            let mut total_elapsed_ms = 0u128;
            let mut scheduled_cases = Vec::with_capacity(group_cases.len());

            for case in group_cases {
                total_points += case.points;
                total_elapsed_ms += case.elapsed_ms;
                if let Some(lag) = case.result_model.witness_lag {
                    current_witness_cases += 1;
                    current_witness_lag_total += lag;
                }
                if let Some(best_known) = &case.best_known_witness {
                    best_known_witness_cases += 1;
                    best_known_witness_lag_total += best_known.lag;
                }

                let campaign = case
                    .campaign
                    .as_ref()
                    .expect("campaign grouping only contains campaign cases");
                scheduled_cases.push(CampaignCaseSummary {
                    case_id: case.id.clone(),
                    strategy: campaign.strategy.clone(),
                    schedule_order: campaign.schedule_order,
                    measurement: case.measurement.clone(),
                    actual_outcome: case.actual_outcome.clone(),
                    current_witness_lag: case.result_model.witness_lag,
                    best_known_witness: case.best_known_witness.clone(),
                    improved_best_known_witness: case.improved_best_known_witness,
                    points: case.points,
                    elapsed_ms: case.elapsed_ms,
                });
            }

            CampaignSummary {
                id,
                cases: scheduled_cases.len(),
                current_witness_cases,
                current_witness_lag_total,
                current_lag_score: lag_score(current_witness_cases, current_witness_lag_total),
                best_known_witness_cases,
                best_known_witness_lag_total,
                best_known_lag_score: lag_score(
                    best_known_witness_cases,
                    best_known_witness_lag_total,
                ),
                total_points,
                total_elapsed_ms,
                scheduled_cases,
            }
        })
        .collect()
}

pub(crate) fn build_strategy_summaries(cases: &[CaseSummary]) -> Vec<StrategySummary> {
    let mut groups = BTreeMap::<String, Vec<&CaseSummary>>::new();
    for case in cases {
        let Some(campaign) = case.campaign.as_ref() else {
            continue;
        };
        groups
            .entry(campaign.strategy.clone())
            .or_default()
            .push(case);
    }

    groups
        .into_iter()
        .map(|(strategy, group_cases)| {
            let mut campaigns = BTreeMap::<String, ()>::new();
            let mut passed_required_cases = 0usize;
            let mut target_hits = 0usize;
            let mut total_points = 0i64;
            let mut total_elapsed_ms = 0u128;
            let mut current_witness_cases = 0usize;
            let mut current_witness_lag_total = 0usize;
            let mut best_known_witness_cases = 0usize;
            let mut best_known_witness_lag_total = 0usize;
            let mut best_known_improvements = 0usize;

            for case in &group_cases {
                if case.required && case.passed {
                    passed_required_cases += 1;
                }
                if case.hit_target {
                    target_hits += 1;
                }
                total_points += case.points;
                total_elapsed_ms += case.elapsed_ms;
                if let Some(lag) = case.result_model.witness_lag {
                    current_witness_cases += 1;
                    current_witness_lag_total += lag;
                }
                if let Some(best_known) = &case.best_known_witness {
                    best_known_witness_cases += 1;
                    best_known_witness_lag_total += best_known.lag;
                }
                if case.improved_best_known_witness {
                    best_known_improvements += 1;
                }
                if let Some(campaign) = case.campaign.as_ref() {
                    campaigns.insert(campaign.id.clone(), ());
                }
            }

            StrategySummary {
                strategy,
                cases: group_cases.len(),
                campaigns: campaigns.len(),
                passed_required_cases,
                target_hits,
                total_points,
                total_elapsed_ms,
                current_witness_cases,
                current_witness_lag_total,
                current_lag_score: lag_score(current_witness_cases, current_witness_lag_total),
                best_known_witness_cases,
                best_known_witness_lag_total,
                best_known_lag_score: lag_score(
                    best_known_witness_cases,
                    best_known_witness_lag_total,
                ),
                best_known_improvements,
            }
        })
        .collect()
}

pub(crate) fn lag_score(witness_cases: usize, witness_lag_total: usize) -> i64 {
    witness_cases as i64 * 1_000_000 - witness_lag_total as i64
}

pub(crate) fn stage_combination_label(config: &JsonSearchConfig) -> String {
    match config.stage {
        SearchStage::EndpointSearch => format!(
            "endpoint_search/frontier={:?}/beam_width={:?}/move_family_policy={:?}",
            config.frontier_mode, config.beam_width, config.move_family_policy
        ),
        SearchStage::GuidedRefinement => format!(
            "guided_refinement/frontier={:?}/beam_width={:?}/move_family_policy={:?}/shortcut_lag={}/min_gap={}/max_gap={:?}/rounds={}/segment_timeout_secs={:?}",
            config.frontier_mode,
            config.beam_width,
            config.move_family_policy,
            config.guided_refinement.max_shortcut_lag,
            config.guided_refinement.min_gap,
            config.guided_refinement.max_gap,
            config.guided_refinement.rounds,
            config.guided_refinement.segment_timeout_secs,
        ),
        SearchStage::ShortcutSearch => format!(
            "shortcut_search/frontier={:?}/beam_width={:?}/move_family_policy={:?}/max_guides={}/rounds={}/max_total_segment_attempts={}/emit_promoted_guides={}/guided_shortcut_lag={}/guided_min_gap={}/guided_max_gap={:?}/guided_rounds={}/guided_segment_timeout_secs={:?}",
            config.frontier_mode,
            config.beam_width,
            config.move_family_policy,
            config.shortcut_search.max_guides,
            config.shortcut_search.rounds,
            config.shortcut_search.max_total_segment_attempts,
            config.shortcut_search.artifacts.emit_promoted_guides,
            config.guided_refinement.max_shortcut_lag,
            config.guided_refinement.min_gap,
            config.guided_refinement.max_gap,
            config.guided_refinement.rounds,
            config.guided_refinement.segment_timeout_secs,
        ),
    }
}

pub(crate) fn derive_telemetry_summary(
    actual_outcome: &str,
    telemetry: &SearchTelemetry,
    configured_max_lag: usize,
) -> DerivedTelemetrySummary {
    if telemetry.invariant_filtered {
        return DerivedTelemetrySummary {
            productive_layers: 0,
            deepest_productive_layer: None,
            first_stagnant_layer: None,
            exhausted_before_max_lag: false,
            terminal_bottleneck: "invariant_filter".to_string(),
            avg_factorisations_per_expanded_node: 0.0,
            avg_survivor_ratio: 0.0,
            avg_discovery_ratio: 0.0,
            avg_dead_end_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
            directed_progress_score: 0,
        };
    }

    if telemetry.permutation_shortcut
        || telemetry.canonical_shortcut
        || telemetry.concrete_shift_shortcut
    {
        return DerivedTelemetrySummary {
            productive_layers: 0,
            deepest_productive_layer: None,
            first_stagnant_layer: None,
            exhausted_before_max_lag: false,
            terminal_bottleneck: "shortcut".to_string(),
            avg_factorisations_per_expanded_node: 0.0,
            avg_survivor_ratio: 0.0,
            avg_discovery_ratio: 0.0,
            avg_dead_end_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
            directed_progress_score: 0,
        };
    }

    if telemetry.layers.is_empty() {
        return DerivedTelemetrySummary {
            productive_layers: 0,
            deepest_productive_layer: None,
            first_stagnant_layer: None,
            exhausted_before_max_lag: false,
            terminal_bottleneck: "no_search".to_string(),
            avg_factorisations_per_expanded_node: 0.0,
            avg_survivor_ratio: 0.0,
            avg_discovery_ratio: 0.0,
            avg_dead_end_ratio: 0.0,
            last_layer_frontier_nodes: 0,
            last_layer_candidates_after_pruning: 0,
            last_layer_discovered_nodes: 0,
            last_layer_next_frontier_nodes: 0,
            focus_progress_score: 0,
            directed_progress_score: 0,
        };
    }

    let productive_layers_vec: Vec<&_> = telemetry
        .layers
        .iter()
        .filter(|layer| layer.discovered_nodes > 0 || layer.collisions_with_other_frontier > 0)
        .collect();
    let productive_layers = productive_layers_vec.len();
    let deepest_productive_layer = productive_layers_vec
        .iter()
        .map(|layer| layer.layer_index)
        .max();
    let first_stagnant_layer = telemetry
        .layers
        .iter()
        .find(|layer| {
            layer.frontier_nodes > 0
                && layer.discovered_nodes == 0
                && layer.collisions_with_other_frontier == 0
                && layer.next_frontier_nodes == 0
        })
        .map(|layer| layer.layer_index);

    let expanded_nodes = telemetry.frontier_nodes_expanded.max(1) as f64;
    let avg_factorisations_per_expanded_node =
        telemetry.factorisations_enumerated as f64 / expanded_nodes;
    let avg_survivor_ratio = if telemetry.factorisations_enumerated == 0 {
        0.0
    } else {
        telemetry.candidates_after_pruning as f64 / telemetry.factorisations_enumerated as f64
    };
    let avg_discovery_ratio = if telemetry.candidates_after_pruning == 0 {
        0.0
    } else {
        telemetry.discovered_nodes as f64 / telemetry.candidates_after_pruning as f64
    };
    let avg_dead_end_ratio = telemetry.dead_end_nodes as f64 / expanded_nodes;

    let last_layer = telemetry
        .layers
        .last()
        .expect("non-empty layers already checked");
    let exhausted_before_max_lag = actual_outcome == "unknown"
        && last_layer.next_frontier_nodes == 0
        && telemetry.layers.len() < configured_max_lag;

    let terminal_bottleneck = classify_bottleneck(actual_outcome, telemetry, last_layer);
    let focus_progress_score =
        compute_reach_progress_score(telemetry, productive_layers, deepest_productive_layer);
    let directed_progress_score = compute_directed_progress_score(
        telemetry,
        deepest_productive_layer,
        avg_discovery_ratio,
        avg_dead_end_ratio,
        exhausted_before_max_lag,
    );

    DerivedTelemetrySummary {
        productive_layers,
        deepest_productive_layer,
        first_stagnant_layer,
        exhausted_before_max_lag,
        terminal_bottleneck,
        avg_factorisations_per_expanded_node,
        avg_survivor_ratio,
        avg_discovery_ratio,
        avg_dead_end_ratio,
        last_layer_frontier_nodes: last_layer.frontier_nodes,
        last_layer_candidates_after_pruning: last_layer.candidates_after_pruning,
        last_layer_discovered_nodes: last_layer.discovered_nodes,
        last_layer_next_frontier_nodes: last_layer.next_frontier_nodes,
        focus_progress_score,
        directed_progress_score,
    }
}

pub(crate) fn scheduled_cases(corpus: &CaseCorpus) -> Vec<&ResearchCase> {
    let mut scheduled_cases = corpus.cases.iter().collect::<Vec<_>>();
    scheduled_cases.sort_by_key(|case| {
        case.campaign
            .as_ref()
            .map(|campaign| {
                (
                    0usize,
                    campaign.id.clone(),
                    campaign.schedule_order,
                    case.id.clone(),
                )
            })
            .unwrap_or_else(|| (1usize, String::new(), 0usize, case.id.clone()))
    });
    scheduled_cases
}

fn summarize_case(
    current_exe: &Path,
    cases_path: &Path,
    reused_results: &ReusedResults,
    case: &ResearchCase,
) -> Result<CaseSummary, String> {
    let resolved = resolve_case(case, cases_path)?;
    let executed = execute_case_for_harness(case, || {
        run_case_in_subprocess(current_exe, cases_path, case)
    })?;
    let passed = case
        .allowed_outcomes
        .iter()
        .any(|allowed| allowed == &executed.representative.actual_outcome);
    let hit_target = case
        .target_outcome
        .as_ref()
        .is_some_and(|target| target == &executed.representative.actual_outcome);
    let points = case
        .points
        .for_outcome(&executed.representative.actual_outcome);

    let endpoint = resolved.endpoint.clone();
    let endpoint_key = endpoint_identity_key(&endpoint);
    let telemetry_summary = derive_telemetry_summary(
        &executed.representative.actual_outcome,
        &executed.representative.telemetry,
        case.config.max_lag,
    );

    let (best_known_witness, improved_best_known_witness) = merge_best_known_witness(
        executed
            .representative
            .result_model
            .witness_lag
            .map(|lag| BestKnownWitness {
                lag,
                elapsed_ms: executed.representative.elapsed_ms,
                source: "current-run".to_string(),
            }),
        reused_results.endpoint_best_witness.get(&endpoint_key),
    );

    Ok(CaseSummary {
        id: case.id.clone(),
        description: case.description.clone(),
        campaign: case.campaign.clone(),
        measurement: executed.measurement.clone(),
        endpoint_fixture: resolved.endpoint_fixture,
        seeded_guide_ids: resolved.seeded_guide_ids,
        guide_artifact_paths: resolved.guide_artifact_paths,
        endpoint,
        config: case.config.clone(),
        actual_outcome: executed.representative.actual_outcome,
        allowed_outcomes: case.allowed_outcomes.clone(),
        target_outcome: case.target_outcome.clone(),
        required: case.required,
        passed,
        hit_target,
        points,
        elapsed_ms: executed.representative.elapsed_ms,
        timeout_ms: case.timeout_ms,
        steps: executed.representative.steps,
        reason: executed.representative.reason,
        result_model: executed.representative.result_model,
        best_known_witness,
        improved_best_known_witness,
        telemetry: executed.representative.telemetry,
        telemetry_summary,
        tags: case.tags.clone(),
    })
}

fn classify_bottleneck(
    actual_outcome: &str,
    telemetry: &SearchTelemetry,
    last_layer: &SearchLayerTelemetry,
) -> String {
    if actual_outcome == "equivalent" {
        return "solved".to_string();
    }
    if actual_outcome == "timeout" {
        return "timeout".to_string();
    }
    if actual_outcome == "panic" {
        return "panic".to_string();
    }
    if last_layer.frontier_nodes >= 16
        && last_layer.candidates_after_pruning <= 1
        && last_layer.discovered_nodes == 0
        && last_layer.next_frontier_nodes == 0
    {
        return "state_space_collapse".to_string();
    }
    if telemetry.collisions_with_seen > telemetry.discovered_nodes
        && telemetry.collisions_with_seen >= 16
    {
        return "duplicate_dominated".to_string();
    }
    if telemetry.candidates_generated > 0
        && telemetry.pruned_by_spectrum.saturating_mul(5)
            >= telemetry.candidates_generated.saturating_mul(4)
    {
        return "spectral_pruning_dominated".to_string();
    }
    if telemetry.max_frontier_size >= 1000 {
        return "frontier_growth".to_string();
    }
    if telemetry.factorisations_enumerated >= 10_000 {
        return "factorisation_volume".to_string();
    }
    "mixed".to_string()
}

fn compute_reach_progress_score(
    telemetry: &SearchTelemetry,
    productive_layers: usize,
    deepest_productive_layer: Option<usize>,
) -> u64 {
    let depth = deepest_productive_layer.unwrap_or(0) as u64;
    let productive = productive_layers as u64;
    let meets = telemetry.collisions_with_other_frontier as u64;
    let visited = telemetry.total_visited_nodes.min(50_000) as u64;
    depth * 1_000_000 + productive * 100_000 + meets * 10_000 + visited
}

fn compute_directed_progress_score(
    telemetry: &SearchTelemetry,
    deepest_productive_layer: Option<usize>,
    avg_discovery_ratio: f64,
    avg_dead_end_ratio: f64,
    exhausted_before_max_lag: bool,
) -> i64 {
    let depth = deepest_productive_layer.unwrap_or(0) as i64;
    let exact_meets = telemetry.collisions_with_other_frontier as i64;
    let approximate_hits = telemetry.approximate_other_side_hits as i64;
    let discovery_per_mille = (avg_discovery_ratio * 1000.0).round() as i64;
    let dead_end_per_mille = (avg_dead_end_ratio * 1000.0).round() as i64;
    let exhaustion_penalty = if exhausted_before_max_lag { 10_000 } else { 0 };

    exact_meets * 1_000_000
        + approximate_hits * 100_000
        + depth * 10_000
        + discovery_per_mille * 1_000
        - dead_end_per_mille * 1_000
        - exhaustion_penalty
}

struct HarnessAccumulator {
    schema_version: u32,
    cases_path: String,
    reused_history_sources: usize,
    cases: Vec<CaseSummary>,
    required_cases: usize,
    passed_required_cases: usize,
    non_required_cases: usize,
    target_hits: usize,
    total_points: i64,
    total_elapsed_ms: u128,
    current_witness_cases: usize,
    current_witness_lag_total: usize,
    best_known_witness_cases: usize,
    best_known_witness_lag_total: usize,
    best_known_improvements: usize,
    generalized_cases: usize,
    telemetry_focus_cases: usize,
    telemetry_focus_score: u64,
    telemetry_focus_directed_score: i64,
}

impl HarnessAccumulator {
    fn new(cases_path: &Path, corpus: &CaseCorpus, reused_results: &ReusedResults) -> Self {
        Self {
            schema_version: corpus.schema_version,
            cases_path: cases_path.display().to_string(),
            reused_history_sources: reused_results.sources.len(),
            cases: Vec::with_capacity(corpus.cases.len()),
            required_cases: 0,
            passed_required_cases: 0,
            non_required_cases: 0,
            target_hits: 0,
            total_points: 0,
            total_elapsed_ms: 0,
            current_witness_cases: 0,
            current_witness_lag_total: 0,
            best_known_witness_cases: 0,
            best_known_witness_lag_total: 0,
            best_known_improvements: 0,
            generalized_cases: 0,
            telemetry_focus_cases: 0,
            telemetry_focus_score: 0,
            telemetry_focus_directed_score: 0,
        }
    }

    fn record_case(&mut self, case: CaseSummary) {
        if case.required {
            self.required_cases += 1;
            if case.passed {
                self.passed_required_cases += 1;
            }
        } else {
            self.non_required_cases += 1;
        }
        if case.hit_target {
            self.target_hits += 1;
        }

        self.total_points += case.points;
        self.total_elapsed_ms += case.elapsed_ms;

        if let Some(lag) = case.result_model.witness_lag {
            self.current_witness_cases += 1;
            self.current_witness_lag_total += lag;
        }
        if let Some(best_known) = &case.best_known_witness {
            self.best_known_witness_cases += 1;
            self.best_known_witness_lag_total += best_known.lag;
        }
        if case.improved_best_known_witness {
            self.best_known_improvements += 1;
        }
        if case.endpoint.source_dim != 2 || case.endpoint.target_dim != 2 {
            self.generalized_cases += 1;
        }
        if case.tags.iter().any(|tag| tag == "telemetry-focus") {
            self.telemetry_focus_cases += 1;
            self.telemetry_focus_score += case.telemetry_summary.focus_progress_score;
            self.telemetry_focus_directed_score += case.telemetry_summary.directed_progress_score;
        }

        self.cases.push(case);
    }

    fn finish(self) -> HarnessSummary {
        let comparisons = build_comparison_summaries(&self.cases);
        let campaigns = build_campaign_summaries(&self.cases);
        let strategies = build_strategy_summaries(&self.cases);

        HarnessSummary {
            schema_version: self.schema_version,
            cases_path: self.cases_path,
            reused_history_sources: self.reused_history_sources,
            fitness: FitnessSummary {
                required_cases: self.required_cases,
                passed_required_cases: self.passed_required_cases,
                non_required_cases: self.non_required_cases,
                target_hits: self.target_hits,
                total_points: self.total_points,
                total_elapsed_ms: self.total_elapsed_ms,
                current_witness_cases: self.current_witness_cases,
                current_witness_lag_total: self.current_witness_lag_total,
                current_lag_score: lag_score(
                    self.current_witness_cases,
                    self.current_witness_lag_total,
                ),
                best_known_witness_cases: self.best_known_witness_cases,
                best_known_witness_lag_total: self.best_known_witness_lag_total,
                best_known_lag_score: lag_score(
                    self.best_known_witness_cases,
                    self.best_known_witness_lag_total,
                ),
                best_known_improvements: self.best_known_improvements,
                generalized_cases: self.generalized_cases,
                comparison_groups: comparisons.len(),
                campaign_groups: campaigns.len(),
                strategy_groups: strategies.len(),
                telemetry_focus_cases: self.telemetry_focus_cases,
                telemetry_focus_score: self.telemetry_focus_score,
                telemetry_focus_reach_score: self.telemetry_focus_score,
                telemetry_focus_directed_score: self.telemetry_focus_directed_score,
            },
            comparisons,
            campaigns,
            strategies,
            cases: self.cases,
        }
    }
}
