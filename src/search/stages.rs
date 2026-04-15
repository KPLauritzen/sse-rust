use std::cmp::Ordering;
use std::time::{Duration, Instant};

use ahash::AHashMap as HashMap;

use super::path::{reanchor_dyn_sse_path, reverse_dyn_sse_path, validate_sse_path_dyn};
use super::*;
use crate::types::{GuideArtifact, GuideArtifactPayload};

#[derive(Clone, Debug)]
pub(super) struct RankedGuide {
    pub(super) path: DynSsePath,
    pub(super) effective_lag: usize,
    pub(super) effective_cost: Option<usize>,
    pub(super) effective_score: Option<f64>,
    pub(super) stable_key: String,
}

#[derive(Debug)]
struct PreparedShortcutGuidePool {
    guides: Vec<RankedGuide>,
    accepted_guides: usize,
    unique_guides: usize,
}

#[derive(Clone, Debug)]
struct ShortcutGuidePoolEntry {
    guide: RankedGuide,
    processed: bool,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct GuidedSegmentCacheKey {
    pub(super) source: DynMatrix,
    pub(super) target: DynMatrix,
    pub(super) max_lag: usize,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct GuidedSegmentEndpointKey {
    source: DynMatrix,
    target: DynMatrix,
}

#[derive(Default)]
pub(super) struct GuidedSegmentCache {
    exact_results: HashMap<GuidedSegmentCacheKey, DynSseResult>,
    shortest_equivalent_paths: HashMap<GuidedSegmentEndpointKey, DynSsePath>,
}

impl GuidedSegmentCache {
    fn get(&self, key: &GuidedSegmentCacheKey) -> Option<DynSseResult> {
        if let Some(result) = self.exact_results.get(key) {
            return Some(result.clone());
        }
        let endpoint_key = GuidedSegmentEndpointKey {
            source: key.source.clone(),
            target: key.target.clone(),
        };
        self.shortest_equivalent_paths
            .get(&endpoint_key)
            .and_then(|path| {
                if path.steps.len() <= key.max_lag {
                    Some(DynSseResult::Equivalent(path.clone()))
                } else {
                    None
                }
            })
    }

    pub(super) fn insert(&mut self, key: GuidedSegmentCacheKey, result: DynSseResult) {
        if let DynSseResult::Equivalent(path) = &result {
            let endpoint_key = GuidedSegmentEndpointKey {
                source: key.source.clone(),
                target: key.target.clone(),
            };
            match self.shortest_equivalent_paths.entry(endpoint_key) {
                std::collections::hash_map::Entry::Occupied(mut entry) => {
                    if path.steps.len() < entry.get().steps.len() {
                        entry.insert(path.clone());
                    }
                }
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(path.clone());
                }
            }
        }
        self.exact_results.insert(key, result);
    }
}

#[derive(Default)]
struct ShortcutGuidePool {
    guides: HashMap<Vec<DynMatrix>, ShortcutGuidePoolEntry>,
}

impl ShortcutGuidePool {
    fn new(guides: Vec<RankedGuide>) -> Self {
        let guides = guides
            .into_iter()
            .map(|guide| {
                (
                    canonical_guide_identity(&guide.path),
                    ShortcutGuidePoolEntry {
                        guide,
                        processed: false,
                    },
                )
            })
            .collect();
        Self { guides }
    }

    fn take_working_set(&mut self, max_guides: usize) -> Vec<RankedGuide> {
        let mut pending = self
            .guides
            .iter()
            .filter(|(_, entry)| !entry.processed)
            .map(|(identity, entry)| (identity.clone(), entry.guide.clone()))
            .collect::<Vec<_>>();
        pending.sort_by(|(_, left), (_, right)| compare_ranked_guides(left, right));
        pending.truncate(max_guides);

        for (identity, _) in &pending {
            if let Some(entry) = self.guides.get_mut(identity) {
                entry.processed = true;
            }
        }

        pending.into_iter().map(|(_, guide)| guide).collect()
    }

    fn promote(&mut self, guide: RankedGuide) -> bool {
        let identity = canonical_guide_identity(&guide.path);
        match self.guides.entry(identity) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if compare_ranked_guides(&guide, &entry.get().guide) == Ordering::Less {
                    let processed = entry.get().processed;
                    entry.insert(ShortcutGuidePoolEntry { guide, processed });
                }
                false
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(ShortcutGuidePoolEntry {
                    guide,
                    processed: false,
                });
                true
            }
        }
    }

    fn has_unprocessed(&self) -> bool {
        self.guides.values().any(|entry| !entry.processed)
    }
}

#[derive(Clone)]
struct GuidedEdge {
    from: usize,
    to: usize,
    lag: usize,
    path: DynSsePath,
}

pub(super) fn execute_search_request(
    request: &SearchRequest,
) -> Result<(SearchRunResult, SearchTelemetry), String> {
    execute_search_request_and_observer(request, None)
}

pub(super) fn execute_search_request_and_observer(
    request: &SearchRequest,
    observer: Option<&mut dyn SearchObserver>,
) -> Result<(SearchRunResult, SearchTelemetry), String> {
    match request.stage {
        SearchStage::EndpointSearch => {
            let a_sq = request.source.to_sq::<2>();
            let b_sq = request.target.to_sq::<2>();
            if let (Some(a), Some(b)) = (a_sq.as_ref(), b_sq.as_ref()) {
                let (result, telemetry) =
                    search_sse_2x2_with_telemetry_and_observer(a, b, &request.config, observer);
                Ok((result.into(), telemetry))
            } else {
                let (result, telemetry) = search_sse_with_telemetry_dyn_and_observer(
                    &request.source,
                    &request.target,
                    &request.config,
                    observer,
                );
                Ok((result.into(), telemetry))
            }
        }
        SearchStage::GuidedRefinement => search_guided_refinement_with_observer(request, observer),
        SearchStage::ShortcutSearch => search_shortcut_search_with_observer(request, observer),
    }
}

fn search_shortcut_search_with_observer(
    request: &SearchRequest,
    mut observer: Option<&mut dyn SearchObserver>,
) -> Result<(SearchRunResult, SearchTelemetry), String> {
    if !request.source.is_square() || !request.target.is_square() {
        return Err("shortcut_search requires square source and target matrices".to_string());
    }
    if request.shortcut_search.max_guides == 0 {
        return Err("shortcut_search requires max_guides >= 1".to_string());
    }
    if request.shortcut_search.rounds == 0 {
        return Err("shortcut_search requires rounds >= 1".to_string());
    }
    if request.shortcut_search.max_total_segment_attempts == 0 {
        return Err("shortcut_search requires max_total_segment_attempts >= 1".to_string());
    }

    let prepared = prepare_shortcut_guide_pool(request)?;
    if prepared.guides.is_empty() {
        return Err(
            "shortcut_search requires at least one compatible full_path guide artifact".to_string(),
        );
    }

    let source_canonical = request.source.canonical_perm();
    let target_canonical = request.target.canonical_perm();
    emit_started(&mut observer, request, &source_canonical, &target_canonical);

    let initial_working_set_guides = prepared
        .guides
        .len()
        .min(request.shortcut_search.max_guides);
    let mut best = prepared
        .guides
        .iter()
        .take(initial_working_set_guides)
        .map(|guide| guide.path.clone())
        .min_by(compare_path_quality)
        .expect("non-empty prepared shortcut guide pool should have an initial working set");
    let best_lag = Some(best.steps.len());
    let mut telemetry = SearchTelemetry {
        guide_artifacts_considered: request.guide_artifacts.len(),
        guide_artifacts_accepted: prepared.accepted_guides,
        shortcut_search: crate::types::ShortcutSearchTelemetry {
            guide_artifacts_loaded: request.guide_artifacts.len(),
            guide_artifacts_accepted: prepared.accepted_guides,
            unique_guides: prepared.unique_guides,
            initial_working_set_guides,
            best_lag_start: best_lag,
            best_lag_end: best_lag,
            ..crate::types::ShortcutSearchTelemetry::default()
        },
        ..SearchTelemetry::default()
    };
    let mut guide_pool = ShortcutGuidePool::new(prepared.guides);
    let mut remaining_segment_attempts = request.shortcut_search.max_total_segment_attempts;
    let mut segment_cache = GuidedSegmentCache::default();
    let mut promoted_serial = 0usize;
    let mut stop_reason = ShortcutSearchStopReason::MaxRoundsReached;

    for round_index in 0..request.shortcut_search.rounds {
        if remaining_segment_attempts == 0 {
            stop_reason = ShortcutSearchStopReason::MaxSegmentAttemptsReached;
            break;
        }

        let working_set = guide_pool.take_working_set(request.shortcut_search.max_guides);
        if working_set.is_empty() {
            stop_reason = ShortcutSearchStopReason::GuidePoolExhausted;
            break;
        }

        let mut round = ShortcutSearchRoundTelemetry {
            round_index,
            working_set_guides: working_set.len(),
            starting_best_lag: Some(best.steps.len()),
            ending_best_lag: Some(best.steps.len()),
            ..ShortcutSearchRoundTelemetry::default()
        };

        for guide in working_set {
            let attempts_before = telemetry.guided_segments_considered;
            let improvements_before = telemetry.guided_segments_improved;
            let refined = refine_guide_path_with_budget(
                request,
                &guide.path,
                &mut telemetry,
                &mut remaining_segment_attempts,
                &mut segment_cache,
            );
            round.segment_attempts += telemetry.guided_segments_considered - attempts_before;
            round.segment_improvements += telemetry.guided_segments_improved - improvements_before;

            if compare_path_quality(&refined, &guide.path) == Ordering::Less {
                let promoted = promoted_ranked_guide(&refined, promoted_serial);
                promoted_serial += 1;
                if guide_pool.promote(promoted) {
                    round.promoted_guides += 1;
                }
            }

            if compare_path_quality(&refined, &best) == Ordering::Less {
                best = refined;
            }

            if remaining_segment_attempts == 0 {
                break;
            }
        }

        round.ending_best_lag = Some(best.steps.len());
        let round_promoted_guides = round.promoted_guides;
        telemetry.shortcut_search.rounds.push(round);
        telemetry.shortcut_search.rounds_completed += 1;
        telemetry.shortcut_search.segment_attempts = telemetry.guided_segments_considered;
        telemetry.shortcut_search.segment_improvements = telemetry.guided_segments_improved;
        telemetry.shortcut_search.promoted_guides += round_promoted_guides;
        telemetry.shortcut_search.best_lag_end = Some(best.steps.len());

        if remaining_segment_attempts == 0 {
            stop_reason = ShortcutSearchStopReason::MaxSegmentAttemptsReached;
            break;
        }
        if !guide_pool.has_unprocessed() {
            stop_reason = ShortcutSearchStopReason::GuidePoolExhausted;
            break;
        }
        if round_promoted_guides == 0 {
            stop_reason = ShortcutSearchStopReason::NoImprovementRound;
            break;
        }
    }

    telemetry.shortcut_search.stop_reason = Some(stop_reason);
    let result = SearchRunResult::Equivalent(best);
    emit_finished(&mut observer, request, result.clone(), &telemetry);
    Ok((result, telemetry))
}

fn search_guided_refinement_with_observer(
    request: &SearchRequest,
    mut observer: Option<&mut dyn SearchObserver>,
) -> Result<(SearchRunResult, SearchTelemetry), String> {
    if request.guided_refinement.max_shortcut_lag == 0 {
        return Err("guided_refinement requires max_shortcut_lag >= 1".to_string());
    }
    if request.guided_refinement.min_gap < 2 {
        return Err("guided_refinement requires min_gap >= 2".to_string());
    }
    if request.guided_refinement.rounds == 0 {
        return Err("guided_refinement requires rounds >= 1".to_string());
    }

    let mut prepared_guides = Vec::new();
    for artifact in &request.guide_artifacts {
        let Some(path) = prepare_full_path_guide(request, artifact)? else {
            continue;
        };
        prepared_guides.push(path);
    }

    if prepared_guides.is_empty() {
        return Err(
            "guided_refinement requires at least one compatible full_path guide artifact"
                .to_string(),
        );
    }

    let mut telemetry = SearchTelemetry {
        guide_artifacts_considered: request.guide_artifacts.len(),
        guide_artifacts_accepted: prepared_guides.len(),
        ..SearchTelemetry::default()
    };
    let source_canonical = request.source.canonical_perm();
    let target_canonical = request.target.canonical_perm();
    emit_started(&mut observer, request, &source_canonical, &target_canonical);

    let mut best: Option<DynSsePath> = None;
    for path in prepared_guides {
        let refined = refine_guide_path(request, &path, &mut telemetry);
        if refined.steps.len() < best.as_ref().map_or(usize::MAX, |path| path.steps.len())
            || (best.is_some()
                && refined.steps.len() == best.as_ref().unwrap().steps.len()
                && refined.matrices.len() < best.as_ref().unwrap().matrices.len())
        {
            best = Some(refined);
        } else if best.is_none() {
            best = Some(refined);
        }
    }

    let best = best.expect("prepared guides should produce a candidate path");
    let result = SearchRunResult::Equivalent(best);
    emit_finished(&mut observer, request, result.clone(), &telemetry);
    Ok((result, telemetry))
}

pub(super) fn prepare_full_path_guide(
    request: &SearchRequest,
    artifact: &GuideArtifact,
) -> Result<Option<DynSsePath>, String> {
    if !guide_artifact_supports_stage(artifact, request.stage) {
        return Ok(None);
    }

    if let Some(max_endpoint_dim) = artifact.compatibility.max_endpoint_dim {
        if request.source.rows > max_endpoint_dim || request.target.rows > max_endpoint_dim {
            return Ok(None);
        }
    }

    let GuideArtifactPayload::FullPath { path } = &artifact.payload;
    validate_sse_path_dyn(&artifact.endpoints.source, &artifact.endpoints.target, path).map_err(
        |err| {
            format!(
                "guide artifact {} is not a valid full-path witness: {err}",
                artifact_label(artifact)
            )
        },
    )?;

    let mut oriented_candidates = Vec::new();
    if endpoint_identity_matches(
        &artifact.endpoints.source,
        &artifact.endpoints.target,
        &request.source,
        &request.target,
    ) {
        oriented_candidates.push(path.clone());
    }
    if endpoint_identity_matches(
        &artifact.endpoints.target,
        &artifact.endpoints.source,
        &request.source,
        &request.target,
    ) {
        oriented_candidates.push(reverse_dyn_sse_path(path));
    }
    if oriented_candidates.is_empty() {
        return Ok(None);
    }

    let mut best: Option<DynSsePath> = None;
    let mut last_error = None;
    for oriented in oriented_candidates {
        let reanchored = match reanchor_dyn_sse_path(&oriented, &request.source, &request.target) {
            Ok(path) => path,
            Err(err) => {
                last_error = Some(format!(
                    "guide artifact {} cannot be re-anchored: {err}",
                    artifact_label(artifact)
                ));
                continue;
            }
        };
        if let Err(err) = validate_sse_path_dyn(&request.source, &request.target, &reanchored) {
            last_error = Some(format!(
                "guide artifact {} does not validate against the requested endpoints: {err}",
                artifact_label(artifact)
            ));
            continue;
        }

        let should_replace = best
            .as_ref()
            .map(|current| {
                reanchored.steps.len() < current.steps.len()
                    || (reanchored.steps.len() == current.steps.len()
                        && reanchored.matrices.len() < current.matrices.len())
            })
            .unwrap_or(true);
        if should_replace {
            best = Some(reanchored);
        }
    }

    match best {
        Some(best) => Ok(Some(best)),
        None => Err(last_error.unwrap_or_else(|| {
            format!(
                "guide artifact {} could not be re-anchored to the requested endpoints",
                artifact_label(artifact)
            )
        })),
    }
}

fn guide_artifact_supports_stage(artifact: &GuideArtifact, stage: SearchStage) -> bool {
    artifact.compatibility.supported_stages.is_empty()
        || artifact.compatibility.supported_stages.contains(&stage)
        || (stage == SearchStage::ShortcutSearch
            && artifact
                .compatibility
                .supported_stages
                .contains(&SearchStage::GuidedRefinement))
}

fn artifact_label(artifact: &GuideArtifact) -> &str {
    artifact
        .artifact_id
        .as_deref()
        .or(artifact.provenance.label.as_deref())
        .unwrap_or("<unnamed>")
}

fn endpoint_identity_matches(
    source_a: &DynMatrix,
    target_a: &DynMatrix,
    source_b: &DynMatrix,
    target_b: &DynMatrix,
) -> bool {
    matrices_share_endpoint_identity(source_a, source_b)
        && matrices_share_endpoint_identity(target_a, target_b)
}

fn matrices_share_endpoint_identity(left: &DynMatrix, right: &DynMatrix) -> bool {
    left.rows == right.rows
        && left.cols == right.cols
        && left.is_square()
        && right.is_square()
        && left.canonical_perm() == right.canonical_perm()
}

fn prepare_shortcut_guide_pool(
    request: &SearchRequest,
) -> Result<PreparedShortcutGuidePool, String> {
    let mut accepted_guides = Vec::new();
    for (index, artifact) in request.guide_artifacts.iter().enumerate() {
        let Some(path) = prepare_full_path_guide(request, artifact)? else {
            continue;
        };
        accepted_guides.push(RankedGuide {
            effective_lag: artifact.quality.lag.unwrap_or(path.steps.len()),
            effective_cost: artifact.quality.cost,
            effective_score: artifact.quality.score,
            stable_key: guide_stable_key(artifact, index),
            path,
        });
    }

    let accepted_count = accepted_guides.len();
    let mut deduped: HashMap<Vec<DynMatrix>, RankedGuide> = HashMap::default();
    for guide in accepted_guides {
        let key = canonical_guide_identity(&guide.path);
        match deduped.entry(key) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if compare_ranked_guides(&guide, entry.get()) == Ordering::Less {
                    entry.insert(guide);
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(guide);
            }
        }
    }

    let unique_guides = deduped.len();
    let mut guides = deduped.into_values().collect::<Vec<_>>();
    guides.sort_by(compare_ranked_guides);

    Ok(PreparedShortcutGuidePool {
        guides,
        accepted_guides: accepted_count,
        unique_guides,
    })
}

fn canonical_guide_identity(path: &DynSsePath) -> Vec<DynMatrix> {
    path.matrices
        .iter()
        .map(DynMatrix::canonical_perm)
        .collect::<Vec<_>>()
}

fn guide_stable_key(artifact: &GuideArtifact, index: usize) -> String {
    format!(
        "{}|{}|{}|{:08}",
        artifact.artifact_id.as_deref().unwrap_or(""),
        artifact.provenance.source_ref.as_deref().unwrap_or(""),
        artifact.provenance.label.as_deref().unwrap_or(""),
        index,
    )
}

pub(super) fn compare_ranked_guides(left: &RankedGuide, right: &RankedGuide) -> Ordering {
    left.effective_lag
        .cmp(&right.effective_lag)
        .then_with(|| compare_optional_usize(left.effective_cost, right.effective_cost))
        .then_with(|| compare_optional_score_desc(left.effective_score, right.effective_score))
        .then_with(|| left.stable_key.cmp(&right.stable_key))
}

fn compare_path_quality(left: &DynSsePath, right: &DynSsePath) -> Ordering {
    left.steps
        .len()
        .cmp(&right.steps.len())
        .then_with(|| left.matrices.len().cmp(&right.matrices.len()))
}

fn promoted_ranked_guide(path: &DynSsePath, serial: usize) -> RankedGuide {
    RankedGuide {
        path: path.clone(),
        effective_lag: path.steps.len(),
        effective_cost: Some(path.steps.len()),
        effective_score: None,
        stable_key: format!("promoted|{:08}", serial),
    }
}

fn compare_optional_usize(left: Option<usize>, right: Option<usize>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => left.cmp(&right),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn compare_optional_score_desc(left: Option<f64>, right: Option<f64>) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) => right.total_cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

fn refine_guide_path(
    request: &SearchRequest,
    initial: &DynSsePath,
    telemetry: &mut SearchTelemetry,
) -> DynSsePath {
    let mut remaining_segment_attempts = usize::MAX;
    let mut segment_cache = GuidedSegmentCache::default();
    refine_guide_path_with_budget(
        request,
        initial,
        telemetry,
        &mut remaining_segment_attempts,
        &mut segment_cache,
    )
}

fn refine_guide_path_with_budget(
    request: &SearchRequest,
    initial: &DynSsePath,
    telemetry: &mut SearchTelemetry,
    remaining_segment_attempts: &mut usize,
    segment_cache: &mut GuidedSegmentCache,
) -> DynSsePath {
    let mut current = initial.clone();
    for _ in 0..request.guided_refinement.rounds {
        if *remaining_segment_attempts == 0 {
            break;
        }
        telemetry.guided_refinement_rounds += 1;
        let next = refine_guide_path_once(
            &current,
            &request.config,
            &request.guided_refinement,
            telemetry,
            remaining_segment_attempts,
            segment_cache,
        );
        if next.steps.len() >= current.steps.len() {
            break;
        }
        current = next;
    }
    current
}

pub(super) fn refine_guide_path_once(
    guide: &DynSsePath,
    base_config: &SearchConfig,
    guided_config: &GuidedRefinementConfig,
    telemetry: &mut SearchTelemetry,
    remaining_segment_attempts: &mut usize,
    segment_cache: &mut GuidedSegmentCache,
) -> DynSsePath {
    if guide.steps.is_empty() {
        return guide.clone();
    }

    let mut edges = Vec::with_capacity(guide.steps.len());
    for idx in 0..guide.steps.len() {
        edges.push(GuidedEdge {
            from: idx,
            to: idx + 1,
            lag: 1,
            path: DynSsePath {
                matrices: vec![guide.matrices[idx].clone(), guide.matrices[idx + 1].clone()],
                steps: vec![guide.steps[idx].clone()],
            },
        });
    }

    let max_gap = guided_config.max_gap.unwrap_or(guide.steps.len());
    'gap_search: for start in 0..guide.steps.len() {
        let min_end = start + guided_config.min_gap;
        if min_end >= guide.matrices.len() {
            continue;
        }
        let max_end = (start + max_gap).min(guide.steps.len());
        for end in min_end..=max_end {
            if *remaining_segment_attempts == 0 {
                break 'gap_search;
            }
            let gap = end - start;
            let lag_cap = guided_config.max_shortcut_lag.min(gap - 1);
            if lag_cap == 0 {
                continue;
            }

            *remaining_segment_attempts -= 1;
            telemetry.guided_segments_considered += 1;
            let cache_key = GuidedSegmentCacheKey {
                source: guide.matrices[start].clone(),
                target: guide.matrices[end].clone(),
                max_lag: lag_cap,
            };
            let result = if let Some(cached_result) = segment_cache.get(&cache_key) {
                telemetry.shortcut_search.segment_cache_hits += 1;
                cached_result.clone()
            } else {
                telemetry.shortcut_search.segment_cache_misses += 1;
                let mut config = base_config.clone();
                config.max_lag = lag_cap;
                let deadline = guided_config
                    .segment_timeout_secs
                    .map(Duration::from_secs)
                    .map(|timeout| Instant::now() + timeout);
                let (result, segment_telemetry) = search_sse_with_telemetry_dyn_with_deadline(
                    &guide.matrices[start],
                    &guide.matrices[end],
                    &config,
                    deadline,
                );
                merge_search_telemetry(telemetry, &segment_telemetry);
                let can_cache_unknown = guided_config.segment_timeout_secs.is_none();
                if !matches!(result, DynSseResult::Unknown) || can_cache_unknown {
                    segment_cache.insert(cache_key, result.clone());
                }
                result
            };
            if let DynSseResult::Equivalent(path) = result {
                if path.steps.len() < gap {
                    telemetry.guided_segments_improved += 1;
                    edges.push(GuidedEdge {
                        from: start,
                        to: end,
                        lag: path.steps.len(),
                        path,
                    });
                }
            }
        }
    }

    let Some(best_route) = shortest_guided_path(guide.matrices.len(), &edges) else {
        return guide.clone();
    };
    stitch_guided_route(&best_route)
}

fn merge_search_telemetry(into: &mut SearchTelemetry, from: &SearchTelemetry) {
    into.invariant_filtered |= from.invariant_filtered;
    into.permutation_shortcut |= from.permutation_shortcut;
    into.canonical_shortcut |= from.canonical_shortcut;
    into.concrete_shift_shortcut |= from.concrete_shift_shortcut;
    into.frontier_nodes_expanded += from.frontier_nodes_expanded;
    into.factorisation_calls += from.factorisation_calls;
    into.factorisations_enumerated += from.factorisations_enumerated;
    into.candidates_generated += from.candidates_generated;
    into.pruned_by_size += from.pruned_by_size;
    into.pruned_by_spectrum += from.pruned_by_spectrum;
    into.candidates_after_pruning += from.candidates_after_pruning;
    into.collisions_with_seen += from.collisions_with_seen;
    into.collisions_with_other_frontier += from.collisions_with_other_frontier;
    into.approximate_other_side_hits += from.approximate_other_side_hits;
    into.same_future_past_collisions += from.same_future_past_collisions;
    into.discovered_nodes += from.discovered_nodes;
    into.dead_end_nodes += from.dead_end_nodes;
    into.enqueued_nodes += from.enqueued_nodes;
    into.max_frontier_size = into.max_frontier_size.max(from.max_frontier_size);
    into.total_visited_nodes += from.total_visited_nodes;
    for (family, family_telemetry) in &from.move_family_telemetry {
        let entry = into
            .move_family_telemetry
            .entry(family.clone())
            .or_default();
        entry.candidates_generated += family_telemetry.candidates_generated;
        entry.candidates_after_pruning += family_telemetry.candidates_after_pruning;
        entry.discovered_nodes += family_telemetry.discovered_nodes;
        entry.exact_meets += family_telemetry.exact_meets;
        entry.approximate_other_side_hits += family_telemetry.approximate_other_side_hits;
    }
    into.layers.extend(from.layers.clone());
}

fn shortest_guided_path(node_count: usize, edges: &[GuidedEdge]) -> Option<Vec<GuidedEdge>> {
    if node_count == 0 {
        return None;
    }
    if node_count == 1 {
        return Some(Vec::new());
    }

    let mut best_cost = vec![usize::MAX; node_count];
    let mut best_prev: Vec<Option<usize>> = vec![None; node_count];
    let mut best_edge: Vec<Option<usize>> = vec![None; node_count];
    best_cost[0] = 0;

    for node in 0..node_count {
        if best_cost[node] == usize::MAX {
            continue;
        }
        for (idx, edge) in edges
            .iter()
            .enumerate()
            .filter(|(_, edge)| edge.from == node)
        {
            let candidate = best_cost[node] + edge.lag;
            if candidate < best_cost[edge.to] {
                best_cost[edge.to] = candidate;
                best_prev[edge.to] = Some(node);
                best_edge[edge.to] = Some(idx);
            }
        }
    }

    if best_cost[node_count - 1] == usize::MAX {
        return None;
    }

    let mut route = Vec::new();
    let mut current = node_count - 1;
    while current != 0 {
        let edge_idx = best_edge[current].expect("reachable node should have an incoming edge");
        route.push(edges[edge_idx].clone());
        current = best_prev[current].expect("reachable node should have a predecessor");
    }
    route.reverse();
    Some(route)
}

fn stitch_guided_route(route: &[GuidedEdge]) -> DynSsePath {
    let mut matrices = Vec::new();
    let mut steps = Vec::new();
    for (idx, edge) in route.iter().enumerate() {
        if idx == 0 {
            matrices.extend(edge.path.matrices.iter().cloned());
        } else {
            matrices.extend(edge.path.matrices.iter().skip(1).cloned());
        }
        steps.extend(edge.path.steps.iter().cloned());
    }
    DynSsePath { matrices, steps }
}
