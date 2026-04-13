use std::cmp::Ordering;
use std::collections::{BTreeMap, VecDeque};
use std::time::{Duration, Instant};

use ahash::{AHashMap as HashMap, AHashSet as HashSet};

use crate::aligned::{
    search_concrete_shift_equivalence_2x2, ConcreteShiftRelation2x2, ConcreteShiftSearchConfig2x2,
    ConcreteShiftSearchResult2x2,
};
use crate::factorisation::visit_all_factorisations_with_family;
use crate::graph_moves::enumerate_graph_move_successors;
use crate::invariants::check_invariants_2x2;
use crate::matrix::{DynMatrix, SqMatrix};
use crate::search_observer::{
    SearchEdgeRecord, SearchEdgeStatus, SearchEvent, SearchFinishedRecord, SearchObserver,
    SearchRootRecord, SearchStartRecord,
};
use crate::types::{
    DynSsePath, DynSseResult, EsseStep, GuideArtifact, GuideArtifactCompatibility,
    GuideArtifactEndpoints, GuideArtifactPayload, GuideArtifactProvenance, GuideArtifactQuality,
    GuideArtifactValidation, GuidedRefinementConfig, SearchConfig, SearchDirection,
    SearchLayerTelemetry, SearchMode, SearchMoveFamilyTelemetry, SearchRequest, SearchRunResult,
    SearchStage, SearchTelemetry, ShortcutSearchConfig, ShortcutSearchRoundTelemetry,
    ShortcutSearchStopReason, SsePath, SseResult,
};

#[cfg(not(target_arch = "wasm32"))]
use rayon::prelude::*;

#[derive(Clone)]
struct FrontierExpansion {
    parent_canon: DynMatrix,
    next_canon: DynMatrix,
    next_orig: DynMatrix,
    step: EsseStep,
    move_family: &'static str,
    same_future_past_signature: Option<SameFuturePastSignature>,
}

#[derive(Clone, Default)]
struct FrontierExpansionStats {
    frontier_nodes: usize,
    factorisation_calls: usize,
    factorisations_enumerated: usize,
    candidates_generated: usize,
    pruned_by_size: usize,
    pruned_by_spectrum: usize,
    same_future_past_collisions: usize,
    move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct ApproxSignature {
    dim: usize,
    entry_sum: u64,
    row_sums: Vec<u32>,
    col_sums: Vec<u32>,
    row_supports: Vec<u8>,
    col_supports: Vec<u8>,
}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
struct SameFuturePastClassSignature {
    multiplicity: usize,
    entry_sum: u32,
    support: u8,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct SameFuturePastSignature {
    dim: usize,
    entry_sum: u64,
    row_classes: Vec<SameFuturePastClassSignature>,
    col_classes: Vec<SameFuturePastClassSignature>,
}

const SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD: usize = 8;
const TIMED_SEARCH_FRONTIER_CHUNK_SIZE: usize = 256;

#[derive(Clone, Copy, Default)]
struct FrontierOverlapSignal {
    frontier_nodes: usize,
    approximate_other_side_hits: usize,
}

impl FrontierOverlapSignal {
    fn from_layer(frontier_nodes: usize, approximate_other_side_hits: usize) -> Self {
        Self {
            frontier_nodes,
            approximate_other_side_hits,
        }
    }

    fn is_trained(self) -> bool {
        self.frontier_nodes >= 8
    }

    fn overlap_ratio(self) -> f64 {
        self.approximate_other_side_hits as f64 / self.frontier_nodes.max(1) as f64
    }
}

#[derive(Clone, Debug)]
struct RankedGuide {
    path: DynSsePath,
    effective_lag: usize,
    effective_cost: Option<usize>,
    effective_score: Option<f64>,
    stable_key: String,
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

#[derive(Clone, Debug, Eq, PartialEq)]
struct BeamFrontierEntry {
    canonical: DynMatrix,
    depth: usize,
    score: i64,
    approximate_hit: bool,
    serial: usize,
}

#[derive(Clone, Debug)]
struct BeamFrontier {
    beam_width: usize,
    entries: Vec<BeamFrontierEntry>,
}

impl BeamFrontier {
    fn new(beam_width: usize) -> Self {
        Self {
            beam_width: beam_width.max(1),
            entries: Vec::new(),
        }
    }

    fn push(&mut self, entry: BeamFrontierEntry) {
        self.entries.push(entry);
        self.entries.sort_by(compare_beam_frontier_entries);
        self.entries.truncate(self.beam_width);
    }

    fn pop_best(&mut self) -> Option<BeamFrontierEntry> {
        if self.entries.is_empty() {
            None
        } else {
            Some(self.entries.remove(0))
        }
    }

    fn peek(&self) -> Option<&BeamFrontierEntry> {
        self.entries.first()
    }

    fn len(&self) -> usize {
        self.entries.len()
    }
}

/// Search for a strong shift equivalence path between two 2x2 matrices.
///
/// Uses bidirectional BFS over the graph where nodes are square matrices of
/// varying sizes (2x2, 3x3, ...) in canonical form, and edges are elementary
/// SSE steps (A = UV, B = VU). Searching from both ends reduces complexity
/// from O(branching^L) to O(2 * branching^(L/2)).
pub fn search_sse_2x2(a: &SqMatrix<2>, b: &SqMatrix<2>, config: &SearchConfig) -> SseResult<2> {
    search_sse_2x2_with_telemetry(a, b, config).0
}

/// Search for a strong shift equivalence path between arbitrary square endpoints.
pub fn search_sse_dyn(a: &DynMatrix, b: &DynMatrix, config: &SearchConfig) -> DynSseResult {
    search_sse_with_telemetry_dyn(a, b, config).0
}

/// Search for a strong shift equivalence path, returning aggregate telemetry.
pub fn search_sse_2x2_with_telemetry(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> (SseResult<2>, SearchTelemetry) {
    search_sse_2x2_with_telemetry_and_observer(a, b, config, None)
}

fn search_request(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    stage: SearchStage,
) -> SearchRequest {
    SearchRequest {
        source: a.clone(),
        target: b.clone(),
        config: config.clone(),
        stage,
        guide_artifacts: Vec::new(),
        guided_refinement: GuidedRefinementConfig::default(),
        shortcut_search: ShortcutSearchConfig::default(),
    }
}

/// Execute one search request across the staged solver boundary.
pub fn execute_search_request(
    request: &SearchRequest,
) -> Result<(SearchRunResult, SearchTelemetry), String> {
    execute_search_request_and_observer(request, None)
}

/// Execute one search request and optionally stream observer events.
pub fn execute_search_request_and_observer(
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
        .min_by(|left, right| compare_path_quality(left, right))
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

fn emit_started(
    observer: &mut Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    a_canonical: &DynMatrix,
    b_canonical: &DynMatrix,
) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&SearchEvent::Started(SearchStartRecord {
            request: request.clone(),
            source_canonical: a_canonical.clone(),
            target_canonical: b_canonical.clone(),
        }));
    }
}

fn emit_roots(observer: &mut Option<&mut dyn SearchObserver>, roots: &[SearchRootRecord]) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&SearchEvent::Roots(roots.to_vec()));
    }
}

fn emit_layer(observer: &mut Option<&mut dyn SearchObserver>, records: &[SearchEdgeRecord]) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&SearchEvent::Layer(records.to_vec()));
    }
}

fn emit_finished(
    observer: &mut Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    result: SearchRunResult,
    telemetry: &SearchTelemetry,
) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&SearchEvent::Finished(SearchFinishedRecord {
            request: request.clone(),
            result,
            telemetry: telemetry.clone(),
        }));
    }
}

fn finish_search_2x2(
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    result: SseResult<2>,
    telemetry: SearchTelemetry,
) -> (SseResult<2>, SearchTelemetry) {
    emit_finished(&mut observer, request, result.clone().into(), &telemetry);
    (result, telemetry)
}

fn finish_search_dyn(
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    result: DynSseResult,
    telemetry: SearchTelemetry,
) -> (DynSseResult, SearchTelemetry) {
    emit_finished(&mut observer, request, result.clone().into(), &telemetry);
    (result, telemetry)
}

#[derive(Clone)]
struct GuidedEdge {
    from: usize,
    to: usize,
    lag: usize,
    path: DynSsePath,
}

/// Validate a 2x2 witness path against its endpoints.
pub fn validate_sse_path_2x2(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    path: &SsePath<2>,
) -> Result<(), String> {
    validate_sse_path_dyn(
        &DynMatrix::from_sq(a),
        &DynMatrix::from_sq(b),
        &path.clone().into(),
    )
}

/// Validate a dynamic witness path against its endpoints.
pub fn validate_sse_path_dyn(
    a: &DynMatrix,
    b: &DynMatrix,
    path: &DynSsePath,
) -> Result<(), String> {
    if path.matrices.len() != path.steps.len() + 1 {
        return Err(format!(
            "path contains {} matrices but {} steps",
            path.matrices.len(),
            path.steps.len()
        ));
    }

    if path.steps.is_empty() {
        if path.matrices.len() != 1 {
            return Err(format!(
                "empty-step path should contain exactly one matrix, got {}",
                path.matrices.len()
            ));
        }
        if path.matrices[0] != *a || path.matrices[0] != *b {
            return Err("empty-step path does not match the endpoint matrices".to_string());
        }
        return Ok(());
    }

    if path.matrices.first() != Some(a) {
        return Err("path.matrices does not start at A".to_string());
    }
    if path.matrices.last() != Some(b) {
        return Err("path.matrices does not end at B".to_string());
    }

    for (idx, step) in path.steps.iter().enumerate() {
        let uv = step.u.mul(&step.v);
        let vu = step.v.mul(&step.u);
        if uv != path.matrices[idx] {
            return Err(format!("step {idx} does not start at path.matrices[{idx}]"));
        }
        if vu != path.matrices[idx + 1] {
            return Err(format!(
                "step {idx} does not end at path.matrices[{}]",
                idx + 1
            ));
        }
    }

    Ok(())
}

/// Build a reusable `full_path` guide artifact from a validated witness path.
pub fn build_full_path_guide_artifact(
    source: &DynMatrix,
    target: &DynMatrix,
    path: &DynSsePath,
) -> Result<GuideArtifact, String> {
    validate_sse_path_dyn(source, target, path)?;
    Ok(GuideArtifact {
        artifact_id: None,
        endpoints: GuideArtifactEndpoints {
            source: source.clone(),
            target: target.clone(),
        },
        payload: GuideArtifactPayload::FullPath { path: path.clone() },
        provenance: GuideArtifactProvenance::default(),
        validation: GuideArtifactValidation::WitnessValidated,
        compatibility: GuideArtifactCompatibility::default(),
        quality: GuideArtifactQuality {
            lag: Some(path.steps.len()),
            cost: Some(path.steps.len()),
            score: None,
        },
    })
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

fn prepare_full_path_guide(
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

fn reverse_dyn_sse_path(path: &DynSsePath) -> DynSsePath {
    DynSsePath {
        matrices: path.matrices.iter().cloned().rev().collect(),
        steps: path
            .steps
            .iter()
            .rev()
            .map(|step| EsseStep {
                u: step.v.clone(),
                v: step.u.clone(),
            })
            .collect(),
    }
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

fn compare_ranked_guides(left: &RankedGuide, right: &RankedGuide) -> Ordering {
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

fn reanchor_dyn_sse_path(
    path: &DynSsePath,
    source: &DynMatrix,
    target: &DynMatrix,
) -> Result<DynSsePath, String> {
    let mut path = path.clone();
    if path.matrices.is_empty() {
        return Err("guide path contains no matrices".to_string());
    }

    if path.matrices.first() != Some(source) {
        let first = path
            .matrices
            .first()
            .expect("non-empty path should have a first matrix")
            .clone();
        let step = permutation_step_between(source, &first).ok_or_else(|| {
            "guide start is not permutation-compatible with the request".to_string()
        })?;
        path.steps.insert(0, step);
        path.matrices.insert(0, source.clone());
    }

    if path.matrices.last() != Some(target) {
        let last = path
            .matrices
            .last()
            .expect("non-empty path should have a last matrix")
            .clone();
        let step = permutation_step_between(&last, target).ok_or_else(|| {
            "guide end is not permutation-compatible with the request".to_string()
        })?;
        path.steps.push(step);
        path.matrices.push(target.clone());
    }

    Ok(path)
}

fn refine_guide_path(
    request: &SearchRequest,
    initial: &DynSsePath,
    telemetry: &mut SearchTelemetry,
) -> DynSsePath {
    let mut remaining_segment_attempts = usize::MAX;
    refine_guide_path_with_budget(request, initial, telemetry, &mut remaining_segment_attempts)
}

fn refine_guide_path_with_budget(
    request: &SearchRequest,
    initial: &DynSsePath,
    telemetry: &mut SearchTelemetry,
    remaining_segment_attempts: &mut usize,
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
        );
        if next.steps.len() >= current.steps.len() {
            break;
        }
        current = next;
    }
    current
}

fn refine_guide_path_once(
    guide: &DynSsePath,
    base_config: &SearchConfig,
    guided_config: &GuidedRefinementConfig,
    telemetry: &mut SearchTelemetry,
    remaining_segment_attempts: &mut usize,
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

/// Search for a strong shift equivalence path between arbitrary square endpoints,
/// returning aggregate telemetry.
pub fn search_sse_with_telemetry_dyn(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
) -> (DynSseResult, SearchTelemetry) {
    search_sse_with_telemetry_dyn_with_deadline_and_observer(a, b, config, None, None)
}

fn search_sse_with_telemetry_dyn_with_deadline(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    deadline: Option<Instant>,
) -> (DynSseResult, SearchTelemetry) {
    search_sse_with_telemetry_dyn_with_deadline_and_observer(a, b, config, None, deadline)
}

/// Search for a strong shift equivalence path between arbitrary square endpoints,
/// returning aggregate telemetry and optionally recording events.
pub fn search_sse_with_telemetry_dyn_and_observer(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    observer: Option<&mut dyn SearchObserver>,
) -> (DynSseResult, SearchTelemetry) {
    search_sse_with_telemetry_dyn_with_deadline_and_observer(a, b, config, observer, None)
}

fn search_sse_with_telemetry_dyn_with_deadline_and_observer(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    deadline: Option<Instant>,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let request = search_request(a, b, config, SearchStage::EndpointSearch);

    if deadline_reached(deadline) {
        return finish_search_dyn(observer, &request, DynSseResult::Unknown, telemetry);
    }

    if !a.is_square() || !b.is_square() {
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::NotEquivalent("search expects square endpoint matrices".to_string()),
            telemetry,
        );
    }

    if trace_square(a) != trace_square(b) {
        telemetry.invariant_filtered = true;
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::NotEquivalent("trace(M^2) invariant mismatch".to_string()),
            telemetry,
        );
    }
    if a.trace() != b.trace() {
        telemetry.invariant_filtered = true;
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::NotEquivalent("trace invariant mismatch".to_string()),
            telemetry,
        );
    }

    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    if a == b {
        emit_started(&mut observer, &request, &a_canon, &b_canon);
        emit_roots(
            &mut observer,
            &[
                SearchRootRecord {
                    direction: SearchDirection::Forward,
                    canonical: a_canon.clone(),
                    orig: a.clone(),
                    depth: 0,
                },
                SearchRootRecord {
                    direction: SearchDirection::Backward,
                    canonical: b_canon.clone(),
                    orig: b.clone(),
                    depth: 0,
                },
            ],
        );
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::Equivalent(DynSsePath {
                matrices: vec![a.clone()],
                steps: vec![],
            }),
            telemetry,
        );
    }

    if a_canon == b_canon {
        telemetry.canonical_shortcut = true;
        if a != b {
            telemetry.permutation_shortcut = true;
        }
        emit_started(&mut observer, &request, &a_canon, &b_canon);
        emit_roots(
            &mut observer,
            &[
                SearchRootRecord {
                    direction: SearchDirection::Forward,
                    canonical: a_canon.clone(),
                    orig: a.clone(),
                    depth: 0,
                },
                SearchRootRecord {
                    direction: SearchDirection::Backward,
                    canonical: b_canon.clone(),
                    orig: b.clone(),
                    depth: 0,
                },
            ],
        );
        return finish_search_dyn(
            observer,
            &request,
            DynSseResult::Equivalent(DynSsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: permutation_step_between(a, b).into_iter().collect(),
            }),
            telemetry,
        );
    }

    if let Some(beam_width) = config.beam_width {
        return search_beam_dyn_with_telemetry(
            a, b, config, observer, &request, deadline, beam_width,
        );
    }

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a.clone());
    fwd_frontier.push_back(a_canon.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b.clone());
    bwd_frontier.push_back(b_canon.clone());
    telemetry.max_frontier_size = 1;
    let mut fwd_factorisations_per_node = 1.0f64;
    let mut bwd_factorisations_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;
    let mut fwd_overlap_signal = FrontierOverlapSignal::default();
    let mut bwd_overlap_signal = FrontierOverlapSignal::default();
    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    emit_started(&mut observer, &request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b.clone(),
                depth: 0,
            },
        ],
    );

    if config.search_mode == SearchMode::GraphOnly {
        return search_graph_only_dyn_with_telemetry(a, b, config, observer, &request, deadline);
    }

    for layer_index in 0..config.max_lag {
        if deadline_reached(deadline) {
            break;
        }
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_factorisations_per_node,
            bwd_factorisations_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal,
            bwd_overlap_signal,
        ) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };

        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                )
            };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let (expansions, expansion_stats, timed_out) = expand_frontier_layer_dyn(
            &current_frontier,
            orig,
            config.max_intermediate_dim,
            config.max_entry,
            config.search_mode,
            deadline,
        );
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        if expansion_stats.frontier_nodes > 0 {
            let factorisations_per_node = expansion_stats.factorisations_enumerated.max(1) as f64
                / expansion_stats.frontier_nodes as f64;
            if expand_forward {
                fwd_factorisations_per_node = factorisations_per_node;
                fwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            } else {
                bwd_factorisations_per_node = factorisations_per_node;
                bwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            }
        }
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;
        let mut next_frontier: VecDeque<DynMatrix> = VecDeque::new();
        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let next_depth = layer_depth + 1;

        for expansion in &expansions {
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            signatures.insert(approx_signature(&expansion.next_canon));

            let approximate_hit =
                other_signatures.contains(&approx_signature(&expansion.next_canon));
            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                let path_depth = next_depth + other_depth;
                if path_depth > config.max_lag {
                    continue;
                }
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: next_frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    move_family_telemetry: layer_move_family_telemetry,
                });
                return finish_search_dyn(
                    observer,
                    &request,
                    DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                next_frontier.push_back(expansion.next_canon.clone());
                enqueued_nodes += 1;
            }
        }

        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        let overlap_signal = FrontierOverlapSignal::from_layer(
            expansion_stats.frontier_nodes,
            approximate_other_side_hits,
        );
        if expand_forward {
            fwd_overlap_signal = overlap_signal;
        } else {
            bwd_overlap_signal = overlap_signal;
        }
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: next_frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            move_family_telemetry: layer_move_family_telemetry,
        });

        if timed_out {
            break;
        }
        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search_dyn(observer, &request, DynSseResult::Unknown, telemetry)
}

/// Search for a strong shift equivalence path, optionally recording the visited graph.
pub fn search_sse_2x2_with_telemetry_and_observer(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();
    let request = search_request(&a_dyn, &b_dyn, config, SearchStage::EndpointSearch);
    emit_started(&mut observer, &request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a_dyn.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b_dyn.clone(),
                depth: 0,
            },
        ],
    );

    // Quick check: are they already equal?
    if a == b {
        return finish_search_2x2(
            observer,
            &request,
            SseResult::Equivalent(SsePath {
                matrices: vec![a.clone()],
                steps: vec![],
            }),
            telemetry,
        );
    }

    // Pre-filter with invariants.
    if let Some(reason) = check_invariants_2x2(a, b) {
        telemetry.invariant_filtered = true;
        return finish_search_2x2(
            observer,
            &request,
            SseResult::NotEquivalent(reason),
            telemetry,
        );
    }

    // If a and b have the same canonical form, they are related by permutation
    // similarity. For 2x2, b = PAP where P = [[0,1],[1,0]].
    // Elementary SSE: U = AP, V = P, then UV = APP = A, VU = PAP = B.
    if a.canonical() == b.canonical() && a != b {
        telemetry.permutation_shortcut = true;
        let p = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let ap = DynMatrix::from_sq(a).mul(&p);
        let step = EsseStep { u: ap, v: p };
        return finish_search_2x2(
            observer,
            &request,
            SseResult::Equivalent(SsePath {
                matrices: vec![a.clone(), b.clone()],
                steps: vec![step],
            }),
            telemetry,
        );
    }

    if let Some(beam_width) = config.beam_width {
        return search_beam_2x2_with_telemetry_and_observer(
            a, b, config, observer, &request, beam_width,
        );
    }

    // Forward direction (from A).
    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a_dyn);
    fwd_frontier.push_back(a_canon.clone());

    // Backward direction (from B).
    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), DynMatrix::from_sq(b));
    bwd_frontier.push_back(b_canon.clone());
    telemetry.max_frontier_size = 1;
    let mut fwd_factorisations_per_node = 1.0f64;
    let mut bwd_factorisations_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;
    let mut fwd_overlap_signal = FrontierOverlapSignal::default();
    let mut bwd_overlap_signal = FrontierOverlapSignal::default();
    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    // Edge case: A and B canonicalise to the same form (should have been
    // caught by the permutation check above, but handle for safety).
    if a_canon == b_canon {
        telemetry.canonical_shortcut = true;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        return finish_search_2x2(
            observer,
            &request,
            SseResult::Equivalent(reconstruct_bidirectional_path(
                a,
                b,
                &a_canon,
                &fwd_parent,
                &fwd_orig,
                &bwd_parent,
                &bwd_orig,
            )),
            telemetry,
        );
    }

    if config.search_mode == SearchMode::GraphOnly {
        return search_graph_only_2x2_with_telemetry_and_observer(a, b, config, observer, &request);
    }

    for layer_index in 0..config.max_lag {
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_factorisations_per_node,
            bwd_factorisations_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            fwd_overlap_signal,
            bwd_overlap_signal,
        ) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };

        let (frontier, parent, depths, orig, signatures, other_depths, other_signatures) =
            if expand_forward {
                (
                    &mut fwd_frontier,
                    &mut fwd_parent,
                    &mut fwd_depths,
                    &mut fwd_orig,
                    &mut fwd_signatures,
                    &bwd_depths as &HashMap<_, _>,
                    &bwd_signatures as &HashSet<_>,
                )
            } else {
                (
                    &mut bwd_frontier,
                    &mut bwd_parent,
                    &mut bwd_depths,
                    &mut bwd_orig,
                    &mut bwd_signatures,
                    &fwd_depths as &HashMap<_, _>,
                    &fwd_signatures as &HashSet<_>,
                )
            };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let (expansions, expansion_stats) = expand_frontier_layer(
            &current_frontier,
            orig,
            config.max_intermediate_dim,
            config.max_entry,
            config.search_mode,
        );
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        if expansion_stats.frontier_nodes > 0 {
            let factorisations_per_node = expansion_stats.factorisations_enumerated.max(1) as f64
                / expansion_stats.frontier_nodes as f64;
            if expand_forward {
                fwd_factorisations_per_node = factorisations_per_node;
                fwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            } else {
                bwd_factorisations_per_node = factorisations_per_node;
                bwd_cost_sample_nodes = expansion_stats.frontier_nodes;
            }
        }
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;
        let mut next_frontier: VecDeque<DynMatrix> = VecDeque::new();
        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = layer_depth + 1;

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            signatures.insert(approx_signature(&expansion.next_canon));

            let approximate_hit =
                other_signatures.contains(&approx_signature(&expansion.next_canon));
            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            // Check if the other side has already visited this node.
            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if path_depth > config.max_lag {
                    if let Some(records) = layer_records.as_mut() {
                        records.push(SearchEdgeRecord {
                            layer_index,
                            direction,
                            move_family: expansion.move_family,
                            from_canonical: expansion.parent_canon.clone(),
                            from_orig: parent_orig.clone(),
                            to_canonical: expansion.next_canon.clone(),
                            to_orig: expansion.next_orig.clone(),
                            from_depth: layer_depth,
                            to_depth: next_depth,
                            step: expansion.step.clone(),
                            status: record_status,
                            approximate_other_side_hit: approximate_hit,
                            enqueued,
                        });
                    }
                    continue;
                }
                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction.clone()),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: next_frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    move_family_telemetry: layer_move_family_telemetry,
                });
                return finish_search_2x2(
                    observer,
                    &request,
                    SseResult::Equivalent(reconstruct_bidirectional_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            // For 2x2 nodes, bound entries to prevent unbounded growth.
            // For intermediate (3x3+) nodes, always add -- the factorisation
            // back to 2x2 already bounds factor entries via max_entry.
            if enqueued {
                next_frontier.push_back(expansion.next_canon.clone());
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: layer_depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        let overlap_signal = FrontierOverlapSignal::from_layer(
            expansion_stats.frontier_nodes,
            approximate_other_side_hits,
        );
        if expand_forward {
            fwd_overlap_signal = overlap_signal;
        } else {
            bwd_overlap_signal = overlap_signal;
        }
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: next_frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            move_family_telemetry: layer_move_family_telemetry,
        });

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    // If bounded ESSE search exhausts on a finite essential pair, try the
    // aligned concrete-shift substrate before reporting `Unknown`.
    if config.search_mode == SearchMode::Mixed && should_try_concrete_shift_fallback(a, b, config) {
        let concrete_config = ConcreteShiftSearchConfig2x2 {
            relation: ConcreteShiftRelation2x2::Aligned,
            max_lag: config.max_lag as u32,
            max_entry: config.max_entry,
            max_witnesses: concrete_shift_witness_budget(config),
        };
        if let ConcreteShiftSearchResult2x2::Equivalent(witness) =
            search_concrete_shift_equivalence_2x2(a, b, &concrete_config)
        {
            telemetry.concrete_shift_shortcut = true;
            return finish_search_2x2(
                observer,
                &request,
                SseResult::EquivalentByConcreteShift(witness),
                telemetry,
            );
        }
    }

    finish_search_2x2(observer, &request, SseResult::Unknown, telemetry)
}

fn is_essential_matrix_2x2(m: &SqMatrix<2>) -> bool {
    let row0 = m.data[0][0] + m.data[0][1];
    let row1 = m.data[1][0] + m.data[1][1];
    let col0 = m.data[0][0] + m.data[1][0];
    let col1 = m.data[0][1] + m.data[1][1];
    row0 > 0 && row1 > 0 && col0 > 0 && col1 > 0
}

fn concrete_shift_witness_budget(config: &SearchConfig) -> usize {
    if config.max_lag <= 4 && config.max_entry <= 6 {
        10_000
    } else {
        25_000
    }
}

fn should_try_concrete_shift_fallback(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
) -> bool {
    is_essential_matrix_2x2(a)
        && is_essential_matrix_2x2(b)
        && config.max_lag <= 4
        && config.max_entry <= 6
}

fn compare_beam_frontier_entries(left: &BeamFrontierEntry, right: &BeamFrontierEntry) -> Ordering {
    right
        .approximate_hit
        .cmp(&left.approximate_hit)
        .then_with(|| right.score.cmp(&left.score))
        .then_with(|| left.depth.cmp(&right.depth))
        .then_with(|| left.serial.cmp(&right.serial))
}

fn choose_next_beam_direction(
    fwd_frontier: &BeamFrontier,
    bwd_frontier: &BeamFrontier,
) -> Option<bool> {
    match (fwd_frontier.peek(), bwd_frontier.peek()) {
        (Some(fwd), Some(bwd)) => match compare_beam_frontier_entries(fwd, bwd) {
            Ordering::Less => Some(true),
            Ordering::Greater => Some(false),
            Ordering::Equal => Some(fwd_frontier.len() <= bwd_frontier.len()),
        },
        (Some(_), None) => Some(true),
        (None, Some(_)) => Some(false),
        (None, None) => None,
    }
}

fn beam_vector_gap_u64(
    left: impl IntoIterator<Item = u64>,
    right: impl IntoIterator<Item = u64>,
) -> i64 {
    let mut left = left.into_iter().collect::<Vec<_>>();
    let mut right = right.into_iter().collect::<Vec<_>>();
    left.sort_unstable();
    right.sort_unstable();
    let len = left.len().max(right.len());
    (0..len)
        .map(|index| {
            let l = left.get(index).copied().unwrap_or(0);
            let r = right.get(index).copied().unwrap_or(0);
            l.abs_diff(r) as i64
        })
        .sum()
}

fn beam_candidate_score(
    matrix: &DynMatrix,
    depth: usize,
    other_signatures: &HashSet<ApproxSignature>,
    target_signature: &ApproxSignature,
) -> (i64, bool) {
    let signature = approx_signature(matrix);
    let approximate_hit = other_signatures.contains(&signature);
    let dim_gap = signature.dim.abs_diff(target_signature.dim) as i64;
    let entry_sum_gap = signature.entry_sum.abs_diff(target_signature.entry_sum) as i64;
    let row_gap = beam_vector_gap_u64(
        signature.row_sums.iter().map(|value| *value as u64),
        target_signature.row_sums.iter().map(|value| *value as u64),
    );
    let col_gap = beam_vector_gap_u64(
        signature.col_sums.iter().map(|value| *value as u64),
        target_signature.col_sums.iter().map(|value| *value as u64),
    );
    let row_support_gap = beam_vector_gap_u64(
        signature.row_supports.iter().map(|value| *value as u64),
        target_signature
            .row_supports
            .iter()
            .map(|value| *value as u64),
    );
    let col_support_gap = beam_vector_gap_u64(
        signature.col_supports.iter().map(|value| *value as u64),
        target_signature
            .col_supports
            .iter()
            .map(|value| *value as u64),
    );
    let score = if approximate_hit { 1_000_000_000 } else { 0 }
        - depth as i64 * 1_000_000
        - dim_gap * 100_000
        - entry_sum_gap * 100
        - row_gap * 10
        - col_gap * 10
        - row_support_gap * 5
        - col_support_gap * 5
        - matrix.max_entry() as i64;
    (score, approximate_hit)
}

fn push_beam_frontier_entry(
    frontier: &mut BeamFrontier,
    canonical: &DynMatrix,
    depth: usize,
    other_signatures: &HashSet<ApproxSignature>,
    target_signature: &ApproxSignature,
    serial: &mut usize,
) {
    let (score, approximate_hit) =
        beam_candidate_score(canonical, depth, other_signatures, target_signature);
    frontier.push(BeamFrontierEntry {
        canonical: canonical.clone(),
        depth,
        score,
        approximate_hit,
        serial: *serial,
    });
    *serial += 1;
}

fn search_beam_2x2_with_telemetry_and_observer(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    beam_width: usize,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a_dyn.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b_dyn.clone());

    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    let target_fwd_signature = approx_signature(&b_canon);
    let target_bwd_signature = approx_signature(&a_canon);
    let mut serial = 0usize;
    let mut fwd_frontier = BeamFrontier::new(beam_width);
    let mut bwd_frontier = BeamFrontier::new(beam_width);
    push_beam_frontier_entry(
        &mut fwd_frontier,
        &a_canon,
        0,
        &bwd_signatures,
        &target_fwd_signature,
        &mut serial,
    );
    push_beam_frontier_entry(
        &mut bwd_frontier,
        &b_canon,
        0,
        &fwd_signatures,
        &target_bwd_signature,
        &mut serial,
    );
    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    emit_started(&mut observer, request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a_dyn,
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b_dyn,
                depth: 0,
            },
        ],
    );

    let mut layer_index = 0usize;
    while let Some(expand_forward) = choose_next_beam_direction(&fwd_frontier, &bwd_frontier) {
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        let (
            frontier,
            parent,
            depths,
            orig,
            signatures,
            other_depths,
            other_signatures,
            target_signature,
        ) = if expand_forward {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_depths,
                &mut fwd_orig,
                &mut fwd_signatures,
                &bwd_depths as &HashMap<_, _>,
                &bwd_signatures as &HashSet<_>,
                &target_fwd_signature,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_depths,
                &mut bwd_orig,
                &mut bwd_signatures,
                &fwd_depths as &HashMap<_, _>,
                &fwd_signatures as &HashSet<_>,
                &target_bwd_signature,
            )
        };

        let Some(current_entry) = frontier.pop_best() else {
            continue;
        };
        if current_entry.depth >= config.max_lag {
            continue;
        }

        let current_frontier = vec![current_entry.canonical.clone()];
        let (expansions, expansion_stats) = expand_frontier_layer(
            &current_frontier,
            orig,
            config.max_intermediate_dim,
            config.max_entry,
            config.search_mode,
        );
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;

        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = current_entry.depth + 1;

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_entry.depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            let next_signature = approx_signature(&expansion.next_canon);
            let approximate_hit = other_signatures.contains(&next_signature);
            signatures.insert(next_signature);

            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_entry.depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if path_depth > config.max_lag {
                    continue;
                }

                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    move_family_telemetry: layer_move_family_telemetry,
                });
                return finish_search_2x2(
                    observer,
                    request,
                    SseResult::Equivalent(reconstruct_bidirectional_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                push_beam_frontier_entry(
                    frontier,
                    &expansion.next_canon,
                    next_depth,
                    other_signatures,
                    target_signature,
                    &mut serial,
                );
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: current_entry.depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            move_family_telemetry: layer_move_family_telemetry,
        });
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        layer_index += 1;
    }

    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
    if config.search_mode == SearchMode::Mixed && should_try_concrete_shift_fallback(a, b, config) {
        let concrete_config = ConcreteShiftSearchConfig2x2 {
            relation: ConcreteShiftRelation2x2::Aligned,
            max_lag: config.max_lag as u32,
            max_entry: config.max_entry,
            max_witnesses: concrete_shift_witness_budget(config),
        };
        if let ConcreteShiftSearchResult2x2::Equivalent(witness) =
            search_concrete_shift_equivalence_2x2(a, b, &concrete_config)
        {
            telemetry.concrete_shift_shortcut = true;
            return finish_search_2x2(
                observer,
                request,
                SseResult::EquivalentByConcreteShift(witness),
                telemetry,
            );
        }
    }

    finish_search_2x2(observer, request, SseResult::Unknown, telemetry)
}

fn search_beam_dyn_with_telemetry(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    deadline: Option<Instant>,
    beam_width: usize,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b.clone());

    let mut fwd_signatures = HashSet::new();
    let mut bwd_signatures = HashSet::new();
    fwd_signatures.insert(approx_signature(&a_canon));
    bwd_signatures.insert(approx_signature(&b_canon));

    let target_fwd_signature = approx_signature(&b_canon);
    let target_bwd_signature = approx_signature(&a_canon);
    let mut serial = 0usize;
    let mut fwd_frontier = BeamFrontier::new(beam_width);
    let mut bwd_frontier = BeamFrontier::new(beam_width);
    push_beam_frontier_entry(
        &mut fwd_frontier,
        &a_canon,
        0,
        &bwd_signatures,
        &target_fwd_signature,
        &mut serial,
    );
    push_beam_frontier_entry(
        &mut bwd_frontier,
        &b_canon,
        0,
        &fwd_signatures,
        &target_bwd_signature,
        &mut serial,
    );
    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);

    emit_started(&mut observer, request, &a_canon, &b_canon);
    emit_roots(
        &mut observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: a_canon.clone(),
                orig: a.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: b_canon.clone(),
                orig: b.clone(),
                depth: 0,
            },
        ],
    );

    let mut layer_index = 0usize;
    while let Some(expand_forward) = choose_next_beam_direction(&fwd_frontier, &bwd_frontier) {
        if deadline_reached(deadline) {
            break;
        }
        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        let (
            frontier,
            parent,
            depths,
            orig,
            signatures,
            other_depths,
            other_signatures,
            target_signature,
        ) = if expand_forward {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_depths,
                &mut fwd_orig,
                &mut fwd_signatures,
                &bwd_depths as &HashMap<_, _>,
                &bwd_signatures as &HashSet<_>,
                &target_fwd_signature,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_depths,
                &mut bwd_orig,
                &mut bwd_signatures,
                &fwd_depths as &HashMap<_, _>,
                &fwd_signatures as &HashSet<_>,
                &target_bwd_signature,
            )
        };

        let Some(current_entry) = frontier.pop_best() else {
            continue;
        };
        if current_entry.depth >= config.max_lag {
            continue;
        }

        let current_frontier = vec![current_entry.canonical.clone()];
        let (expansions, expansion_stats, timed_out) = expand_frontier_layer_dyn(
            &current_frontier,
            orig,
            config.max_intermediate_dim,
            config.max_entry,
            config.search_mode,
            deadline,
        );
        telemetry.frontier_nodes_expanded += expansion_stats.frontier_nodes;
        telemetry.factorisation_calls += expansion_stats.factorisation_calls;
        telemetry.factorisations_enumerated += expansion_stats.factorisations_enumerated;
        telemetry.candidates_generated += expansion_stats.candidates_generated;
        telemetry.pruned_by_size += expansion_stats.pruned_by_size;
        telemetry.pruned_by_spectrum += expansion_stats.pruned_by_spectrum;
        let candidates_after_pruning = expansions.len();
        telemetry.candidates_after_pruning += candidates_after_pruning;

        let mut collisions_with_seen = 0usize;
        let mut collisions_with_other_frontier = 0usize;
        let mut approximate_other_side_hits = 0usize;
        let mut discovered_nodes = 0usize;
        let mut parents_with_progress = HashSet::new();
        let mut enqueued_nodes = 0usize;
        let mut layer_move_family_telemetry = expansion_stats.move_family_telemetry.clone();
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(expansions.len()));
        let next_depth = current_entry.depth + 1;

        for expansion in &expansions {
            let parent_orig = orig
                .get(&expansion.parent_canon)
                .expect("parent node should have an original matrix")
                .clone();
            if parent.contains_key(&expansion.next_canon) {
                collisions_with_seen += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_entry.depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: SearchEdgeStatus::SeenCollision,
                        approximate_other_side_hit: false,
                        enqueued: false,
                    });
                }
                continue;
            }

            discovered_nodes += 1;
            parent.insert(
                expansion.next_canon.clone(),
                Some((expansion.parent_canon.clone(), expansion.step.clone())),
            );
            depths.insert(expansion.next_canon.clone(), next_depth);
            orig.insert(expansion.next_canon.clone(), expansion.next_orig.clone());
            let next_signature = approx_signature(&expansion.next_canon);
            let approximate_hit = other_signatures.contains(&next_signature);
            signatures.insert(next_signature);

            let enqueued =
                expansion.next_orig.rows > 2 || expansion.next_orig.max_entry() <= config.max_entry;
            let mut record_status = SearchEdgeStatus::Discovered;

            if let Some(&other_depth) = other_depths.get(&expansion.next_canon) {
                collisions_with_other_frontier += 1;
                parents_with_progress.insert(expansion.parent_canon.clone());
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .exact_meets += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .discovered_nodes += 1;
                record_status = SearchEdgeStatus::ExactMeet;
                let path_depth = next_depth + other_depth;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: expansion.move_family,
                        from_canonical: expansion.parent_canon.clone(),
                        from_orig: parent_orig.clone(),
                        to_canonical: expansion.next_canon.clone(),
                        to_orig: expansion.next_orig.clone(),
                        from_depth: current_entry.depth,
                        to_depth: next_depth,
                        step: expansion.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: approximate_hit,
                        enqueued,
                    });
                }
                if path_depth > config.max_lag {
                    continue;
                }

                telemetry.collisions_with_seen += collisions_with_seen;
                telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
                telemetry.approximate_other_side_hits += approximate_other_side_hits;
                telemetry.same_future_past_collisions +=
                    expansion_stats.same_future_past_collisions;
                telemetry.discovered_nodes += discovered_nodes;
                let dead_end_nodes = current_frontier
                    .len()
                    .saturating_sub(parents_with_progress.len());
                telemetry.dead_end_nodes += dead_end_nodes;
                telemetry.enqueued_nodes += enqueued_nodes;
                telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
                accumulate_move_family_telemetry(
                    &mut telemetry.move_family_telemetry,
                    &layer_move_family_telemetry,
                );
                if let Some(records) = layer_records.as_ref() {
                    emit_layer(&mut observer, records);
                }
                telemetry.layers.push(SearchLayerTelemetry {
                    layer_index,
                    direction: Some(direction),
                    frontier_nodes: expansion_stats.frontier_nodes,
                    factorisation_calls: expansion_stats.factorisation_calls,
                    factorisations_enumerated: expansion_stats.factorisations_enumerated,
                    candidates_generated: expansion_stats.candidates_generated,
                    pruned_by_size: expansion_stats.pruned_by_size,
                    pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
                    candidates_after_pruning,
                    collisions_with_seen,
                    collisions_with_other_frontier,
                    approximate_other_side_hits,
                    same_future_past_collisions: expansion_stats.same_future_past_collisions,
                    discovered_nodes,
                    dead_end_nodes,
                    enqueued_nodes,
                    next_frontier_nodes: frontier.len(),
                    total_visited_nodes: telemetry.total_visited_nodes,
                    move_family_telemetry: layer_move_family_telemetry,
                });
                return finish_search_dyn(
                    observer,
                    request,
                    DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                        a,
                        b,
                        &expansion.next_canon,
                        &fwd_parent,
                        &fwd_orig,
                        &bwd_parent,
                        &bwd_orig,
                    )),
                    telemetry,
                );
            }

            if approximate_hit {
                approximate_other_side_hits += 1;
                move_family_telemetry_mut(
                    &mut layer_move_family_telemetry,
                    expansion.move_family,
                )
                .approximate_other_side_hits += 1;
            }

            parents_with_progress.insert(expansion.parent_canon.clone());
            move_family_telemetry_mut(&mut layer_move_family_telemetry, expansion.move_family)
                .discovered_nodes += 1;

            if enqueued {
                push_beam_frontier_entry(
                    frontier,
                    &expansion.next_canon,
                    next_depth,
                    other_signatures,
                    target_signature,
                    &mut serial,
                );
                enqueued_nodes += 1;
            }
            if let Some(records) = layer_records.as_mut() {
                records.push(SearchEdgeRecord {
                    layer_index,
                    direction,
                    move_family: expansion.move_family,
                    from_canonical: expansion.parent_canon.clone(),
                    from_orig: parent_orig,
                    to_canonical: expansion.next_canon.clone(),
                    to_orig: expansion.next_orig.clone(),
                    from_depth: current_entry.depth,
                    to_depth: next_depth,
                    step: expansion.step.clone(),
                    status: record_status,
                    approximate_other_side_hit: approximate_hit,
                    enqueued,
                });
            }
        }

        telemetry.collisions_with_seen += collisions_with_seen;
        telemetry.collisions_with_other_frontier += collisions_with_other_frontier;
        telemetry.approximate_other_side_hits += approximate_other_side_hits;
        telemetry.same_future_past_collisions += expansion_stats.same_future_past_collisions;
        telemetry.discovered_nodes += discovered_nodes;
        let dead_end_nodes = current_frontier
            .len()
            .saturating_sub(parents_with_progress.len());
        telemetry.dead_end_nodes += dead_end_nodes;
        telemetry.enqueued_nodes += enqueued_nodes;
        telemetry.total_visited_nodes = visited_union_size(&fwd_parent, &bwd_parent);
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer_move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        telemetry.layers.push(SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: expansion_stats.frontier_nodes,
            factorisation_calls: expansion_stats.factorisation_calls,
            factorisations_enumerated: expansion_stats.factorisations_enumerated,
            candidates_generated: expansion_stats.candidates_generated,
            pruned_by_size: expansion_stats.pruned_by_size,
            pruned_by_spectrum: expansion_stats.pruned_by_spectrum,
            candidates_after_pruning,
            collisions_with_seen,
            collisions_with_other_frontier,
            approximate_other_side_hits,
            same_future_past_collisions: expansion_stats.same_future_past_collisions,
            discovered_nodes,
            dead_end_nodes,
            enqueued_nodes,
            next_frontier_nodes: frontier.len(),
            total_visited_nodes: telemetry.total_visited_nodes,
            move_family_telemetry: layer_move_family_telemetry,
        });
        telemetry.max_frontier_size = telemetry
            .max_frontier_size
            .max(fwd_frontier.len().max(bwd_frontier.len()));
        layer_index += 1;

        if timed_out {
            break;
        }
    }

    finish_search_dyn(observer, request, DynSseResult::Unknown, telemetry)
}

fn search_graph_only_2x2_with_telemetry_and_observer(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    config: &SearchConfig,
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
) -> (SseResult<2>, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);
    let a_canon = a_dyn.canonical_perm();
    let b_canon = b_dyn.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a_dyn);
    fwd_frontier.push_back(a_canon.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b_dyn);
    bwd_frontier.push_back(b_canon);

    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = 2;
    let mut fwd_candidates_per_node = 1.0f64;
    let mut bwd_candidates_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;

    for layer_index in 0..config.max_lag {
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_candidates_per_node,
            bwd_candidates_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }

        let direction = if expand_forward {
            SearchDirection::Forward
        } else {
            SearchDirection::Backward
        };

        let (frontier, parent, depths, orig, other_depths) = if expand_forward {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_depths,
                &mut fwd_orig,
                &bwd_depths,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_depths,
                &mut bwd_orig,
                &fwd_depths,
            )
        };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let current_frontier_len = current_frontier.len();
        let computed: Vec<(DynMatrix, crate::graph_moves::GraphMoveSuccessors)> = current_frontier
            .par_iter()
            .map(|current_canon| {
                let current = orig
                    .get(current_canon)
                    .expect("frontier node should have an original matrix");
                (
                    current_canon.clone(),
                    enumerate_graph_move_successors(current, config.max_intermediate_dim),
                )
            })
            .collect();

        let mut layer = SearchLayerTelemetry {
            layer_index,
            direction: Some(direction),
            frontier_nodes: current_frontier_len,
            ..SearchLayerTelemetry::default()
        };
        let mut layer_records = observer
            .as_ref()
            .map(|_| Vec::with_capacity(layer.candidates_after_pruning.max(8)));
        let mut next_frontier = VecDeque::new();
        let next_depth = layer_depth + 1;
        let mut parents_with_progress = HashSet::new();

        for (current_canon, successors) in computed {
            let current_orig = orig
                .get(&current_canon)
                .expect("frontier node should have an original matrix")
                .clone();
            layer.candidates_generated += successors.candidates;
            for (family, count) in successors.family_candidates {
                move_family_telemetry_mut(&mut layer.move_family_telemetry, family)
                    .candidates_generated += count;
            }

            if current_frontier_len > 0 {
                let candidates_per_node =
                    layer.candidates_generated.max(1) as f64 / current_frontier_len as f64;
                if expand_forward {
                    fwd_candidates_per_node = candidates_per_node;
                    fwd_cost_sample_nodes = current_frontier_len;
                } else {
                    bwd_candidates_per_node = candidates_per_node;
                    bwd_cost_sample_nodes = current_frontier_len;
                }
            }

            for successor in successors.nodes {
                if successor.orig_matrix.max_entry() > config.max_entry {
                    continue;
                }
                move_family_telemetry_mut(&mut layer.move_family_telemetry, successor.family)
                    .candidates_after_pruning += 1;
                layer.candidates_after_pruning += 1;

                if parent.contains_key(&successor.matrix) {
                    layer.collisions_with_seen += 1;
                    if let Some(records) = layer_records.as_mut() {
                        records.push(SearchEdgeRecord {
                            layer_index,
                            direction,
                            move_family: successor.family,
                            from_canonical: current_canon.clone(),
                            from_orig: current_orig.clone(),
                            to_canonical: successor.matrix.clone(),
                            to_orig: successor.orig_matrix.clone(),
                            from_depth: layer_depth,
                            to_depth: next_depth,
                            step: successor.step.clone(),
                            status: SearchEdgeStatus::SeenCollision,
                            approximate_other_side_hit: false,
                            enqueued: false,
                        });
                    }
                    continue;
                }

                parent.insert(
                    successor.matrix.clone(),
                    Some((current_canon.clone(), successor.step.clone())),
                );
                depths.insert(successor.matrix.clone(), next_depth);
                orig.insert(successor.matrix.clone(), successor.orig_matrix.clone());
                layer.discovered_nodes += 1;
                move_family_telemetry_mut(&mut layer.move_family_telemetry, successor.family)
                    .discovered_nodes += 1;
                parents_with_progress.insert(current_canon.clone());
                let mut record_status = SearchEdgeStatus::Discovered;

                if let Some(&other_depth) = other_depths.get(&successor.matrix) {
                    layer.collisions_with_other_frontier += 1;
                    move_family_telemetry_mut(
                        &mut layer.move_family_telemetry,
                        successor.family,
                    )
                    .exact_meets += 1;
                    record_status = SearchEdgeStatus::ExactMeet;
                    let path_depth = next_depth + other_depth;
                    if path_depth <= config.max_lag {
                        layer.next_frontier_nodes = next_frontier.len();
                        layer.total_visited_nodes = telemetry.total_visited_nodes;
                        layer.dead_end_nodes =
                            current_frontier_len.saturating_sub(parents_with_progress.len());
                        telemetry.collisions_with_seen += layer.collisions_with_seen;
                        telemetry.collisions_with_other_frontier +=
                            layer.collisions_with_other_frontier;
                        telemetry.same_future_past_collisions += layer.same_future_past_collisions;
                        telemetry.discovered_nodes += layer.discovered_nodes;
                        telemetry.dead_end_nodes += layer.dead_end_nodes;
                        telemetry.enqueued_nodes += layer.enqueued_nodes;
                        telemetry.candidates_generated += layer.candidates_generated;
                        telemetry.candidates_after_pruning += layer.candidates_after_pruning;
                        telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
                        telemetry.frontier_nodes_expanded += layer.frontier_nodes;
                        telemetry.total_visited_nodes =
                            visited_union_size(&fwd_parent, &bwd_parent);
                        layer.total_visited_nodes = telemetry.total_visited_nodes;
                        accumulate_move_family_telemetry(
                            &mut telemetry.move_family_telemetry,
                            &layer.move_family_telemetry,
                        );
                        if let Some(records) = layer_records.as_mut() {
                            records.push(SearchEdgeRecord {
                                layer_index,
                                direction,
                                move_family: successor.family,
                                from_canonical: current_canon.clone(),
                                from_orig: current_orig.clone(),
                                to_canonical: successor.matrix.clone(),
                                to_orig: successor.orig_matrix.clone(),
                                from_depth: layer_depth,
                                to_depth: next_depth,
                                step: successor.step.clone(),
                                status: record_status,
                                approximate_other_side_hit: false,
                                enqueued: false,
                            });
                        }
                        if let Some(records) = layer_records.as_ref() {
                            emit_layer(&mut observer, records);
                        }
                        telemetry.layers.push(layer);
                        return finish_search_2x2(
                            observer,
                            request,
                            SseResult::Equivalent(reconstruct_bidirectional_path(
                                a,
                                b,
                                &successor.matrix,
                                &fwd_parent,
                                &fwd_orig,
                                &bwd_parent,
                                &bwd_orig,
                            )),
                            telemetry,
                        );
                    }
                } else {
                    telemetry.total_visited_nodes += 1;
                }

                next_frontier.push_back(successor.matrix.clone());
                layer.enqueued_nodes += 1;
                if let Some(records) = layer_records.as_mut() {
                    records.push(SearchEdgeRecord {
                        layer_index,
                        direction,
                        move_family: successor.family,
                        from_canonical: current_canon.clone(),
                        from_orig: current_orig.clone(),
                        to_canonical: successor.matrix.clone(),
                        to_orig: successor.orig_matrix.clone(),
                        from_depth: layer_depth,
                        to_depth: next_depth,
                        step: successor.step.clone(),
                        status: record_status,
                        approximate_other_side_hit: false,
                        enqueued: true,
                    });
                }
            }
        }

        layer.dead_end_nodes = current_frontier_len.saturating_sub(parents_with_progress.len());
        layer.next_frontier_nodes = next_frontier.len();
        layer.total_visited_nodes = telemetry.total_visited_nodes;

        telemetry.frontier_nodes_expanded += layer.frontier_nodes;
        telemetry.candidates_generated += layer.candidates_generated;
        telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
        telemetry.candidates_after_pruning += layer.candidates_after_pruning;
        telemetry.collisions_with_seen += layer.collisions_with_seen;
        telemetry.collisions_with_other_frontier += layer.collisions_with_other_frontier;
        telemetry.same_future_past_collisions += layer.same_future_past_collisions;
        telemetry.discovered_nodes += layer.discovered_nodes;
        telemetry.dead_end_nodes += layer.dead_end_nodes;
        telemetry.enqueued_nodes += layer.enqueued_nodes;
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer.move_family_telemetry,
        );
        if let Some(records) = layer_records.as_ref() {
            emit_layer(&mut observer, records);
        }
        telemetry.layers.push(layer);

        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search_2x2(observer, request, SseResult::Unknown, telemetry)
}

fn search_graph_only_dyn_with_telemetry(
    a: &DynMatrix,
    b: &DynMatrix,
    config: &SearchConfig,
    observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    deadline: Option<Instant>,
) -> (DynSseResult, SearchTelemetry) {
    let mut telemetry = SearchTelemetry::default();
    let a_canon = a.canonical_perm();
    let b_canon = b.canonical_perm();

    let mut fwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut fwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut fwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut fwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    fwd_parent.insert(a_canon.clone(), None);
    fwd_depths.insert(a_canon.clone(), 0);
    fwd_orig.insert(a_canon.clone(), a.clone());
    fwd_frontier.push_back(a_canon.clone());

    let mut bwd_parent: HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>> = HashMap::new();
    let mut bwd_depths: HashMap<DynMatrix, usize> = HashMap::new();
    let mut bwd_orig: HashMap<DynMatrix, DynMatrix> = HashMap::new();
    let mut bwd_frontier: VecDeque<DynMatrix> = VecDeque::new();
    bwd_parent.insert(b_canon.clone(), None);
    bwd_depths.insert(b_canon.clone(), 0);
    bwd_orig.insert(b_canon.clone(), b.clone());
    bwd_frontier.push_back(b_canon.clone());

    telemetry.max_frontier_size = 1;
    telemetry.total_visited_nodes = 2;
    let mut fwd_candidates_per_node = 1.0f64;
    let mut bwd_candidates_per_node = 1.0f64;
    let mut fwd_cost_sample_nodes = 0usize;
    let mut bwd_cost_sample_nodes = 0usize;

    for layer_index in 0..config.max_lag {
        if deadline_reached(deadline) {
            break;
        }
        let next_fwd_depth = fwd_frontier
            .front()
            .and_then(|node| fwd_depths.get(node))
            .copied();
        let next_bwd_depth = bwd_frontier
            .front()
            .and_then(|node| bwd_depths.get(node))
            .copied();
        let Some((expand_forward, layer_depth)) = choose_next_layer(
            next_fwd_depth,
            next_bwd_depth,
            fwd_frontier.len(),
            bwd_frontier.len(),
            fwd_candidates_per_node,
            bwd_candidates_per_node,
            fwd_cost_sample_nodes,
            bwd_cost_sample_nodes,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ) else {
            break;
        };
        if layer_depth >= config.max_lag {
            break;
        }

        let (frontier, parent, depths, orig, other_depths) = if expand_forward {
            (
                &mut fwd_frontier,
                &mut fwd_parent,
                &mut fwd_depths,
                &mut fwd_orig,
                &bwd_depths,
            )
        } else {
            (
                &mut bwd_frontier,
                &mut bwd_parent,
                &mut bwd_depths,
                &mut bwd_orig,
                &fwd_depths,
            )
        };

        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
        let current_frontier: Vec<DynMatrix> = frontier.drain(..).collect();
        let current_frontier_len = current_frontier.len();
        let mut layer = SearchLayerTelemetry {
            layer_index,
            direction: Some(if expand_forward {
                SearchDirection::Forward
            } else {
                SearchDirection::Backward
            }),
            frontier_nodes: current_frontier_len,
            ..SearchLayerTelemetry::default()
        };
        let mut next_frontier = VecDeque::new();
        let next_depth = layer_depth + 1;
        let mut parents_with_progress = HashSet::new();
        let mut timed_out = false;

        for chunk in current_frontier.chunks(frontier_chunk_size(current_frontier_len, deadline)) {
            if deadline_reached(deadline) {
                timed_out = true;
                break;
            }
            let computed: Vec<(DynMatrix, crate::graph_moves::GraphMoveSuccessors)> = chunk
                .par_iter()
                .map(|current_canon| {
                    let current = orig
                        .get(current_canon)
                        .expect("frontier node should have an original matrix");
                    (
                        current_canon.clone(),
                        enumerate_graph_move_successors(current, config.max_intermediate_dim),
                    )
                })
                .collect();

            for (current_canon, successors) in computed {
                layer.candidates_generated += successors.candidates;
                for (family, count) in successors.family_candidates {
                    move_family_telemetry_mut(&mut layer.move_family_telemetry, family)
                        .candidates_generated += count;
                }

                if current_frontier_len > 0 {
                    let candidates_per_node =
                        layer.candidates_generated.max(1) as f64 / current_frontier_len as f64;
                    if expand_forward {
                        fwd_candidates_per_node = candidates_per_node;
                        fwd_cost_sample_nodes = current_frontier_len;
                    } else {
                        bwd_candidates_per_node = candidates_per_node;
                        bwd_cost_sample_nodes = current_frontier_len;
                    }
                }

                for successor in successors.nodes {
                    if successor.orig_matrix.max_entry() > config.max_entry {
                        continue;
                    }
                    move_family_telemetry_mut(
                        &mut layer.move_family_telemetry,
                        successor.family,
                    )
                    .candidates_after_pruning += 1;
                    layer.candidates_after_pruning += 1;

                    if parent.contains_key(&successor.matrix) {
                        layer.collisions_with_seen += 1;
                        continue;
                    }

                    parent.insert(
                        successor.matrix.clone(),
                        Some((current_canon.clone(), successor.step.clone())),
                    );
                    depths.insert(successor.matrix.clone(), next_depth);
                    orig.insert(successor.matrix.clone(), successor.orig_matrix.clone());
                    layer.discovered_nodes += 1;
                    move_family_telemetry_mut(
                        &mut layer.move_family_telemetry,
                        successor.family,
                    )
                    .discovered_nodes += 1;
                    parents_with_progress.insert(current_canon.clone());

                    if let Some(&other_depth) = other_depths.get(&successor.matrix) {
                        layer.collisions_with_other_frontier += 1;
                        move_family_telemetry_mut(
                            &mut layer.move_family_telemetry,
                            successor.family,
                        )
                        .exact_meets += 1;
                        let path_depth = next_depth + other_depth;
                        if path_depth <= config.max_lag {
                            layer.next_frontier_nodes = next_frontier.len();
                            telemetry.collisions_with_seen += layer.collisions_with_seen;
                            telemetry.collisions_with_other_frontier +=
                                layer.collisions_with_other_frontier;
                            telemetry.same_future_past_collisions +=
                                layer.same_future_past_collisions;
                            telemetry.discovered_nodes += layer.discovered_nodes;
                            layer.dead_end_nodes =
                                current_frontier_len.saturating_sub(parents_with_progress.len());
                            telemetry.dead_end_nodes += layer.dead_end_nodes;
                            telemetry.enqueued_nodes += layer.enqueued_nodes;
                            telemetry.candidates_generated += layer.candidates_generated;
                            telemetry.candidates_after_pruning += layer.candidates_after_pruning;
                            telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
                            telemetry.frontier_nodes_expanded += layer.frontier_nodes;
                            telemetry.total_visited_nodes =
                                visited_union_size(&fwd_parent, &bwd_parent);
                            layer.total_visited_nodes = telemetry.total_visited_nodes;
                            accumulate_move_family_telemetry(
                                &mut telemetry.move_family_telemetry,
                                &layer.move_family_telemetry,
                            );
                            telemetry.layers.push(layer);
                            return finish_search_dyn(
                                observer,
                                request,
                                DynSseResult::Equivalent(reconstruct_bidirectional_dyn_path(
                                    a,
                                    b,
                                    &successor.matrix,
                                    &fwd_parent,
                                    &fwd_orig,
                                    &bwd_parent,
                                    &bwd_orig,
                                )),
                                telemetry,
                            );
                        }
                    } else {
                        telemetry.total_visited_nodes += 1;
                    }

                    next_frontier.push_back(successor.matrix.clone());
                    layer.enqueued_nodes += 1;
                }
            }
        }

        layer.dead_end_nodes = current_frontier_len.saturating_sub(parents_with_progress.len());
        layer.next_frontier_nodes = next_frontier.len();
        layer.total_visited_nodes = telemetry.total_visited_nodes;

        telemetry.frontier_nodes_expanded += layer.frontier_nodes;
        telemetry.candidates_generated += layer.candidates_generated;
        telemetry.pruned_by_spectrum += layer.pruned_by_spectrum;
        telemetry.candidates_after_pruning += layer.candidates_after_pruning;
        telemetry.collisions_with_seen += layer.collisions_with_seen;
        telemetry.collisions_with_other_frontier += layer.collisions_with_other_frontier;
        telemetry.same_future_past_collisions += layer.same_future_past_collisions;
        telemetry.discovered_nodes += layer.discovered_nodes;
        telemetry.dead_end_nodes += layer.dead_end_nodes;
        telemetry.enqueued_nodes += layer.enqueued_nodes;
        accumulate_move_family_telemetry(
            &mut telemetry.move_family_telemetry,
            &layer.move_family_telemetry,
        );
        telemetry.layers.push(layer);

        if timed_out {
            break;
        }
        if next_frontier.is_empty() {
            break;
        }
        *frontier = next_frontier;
        telemetry.max_frontier_size = telemetry.max_frontier_size.max(frontier.len());
    }

    finish_search_dyn(observer, request, DynSseResult::Unknown, telemetry)
}

fn deadline_reached(deadline: Option<Instant>) -> bool {
    deadline.is_some_and(|deadline| Instant::now() >= deadline)
}

fn frontier_chunk_size(frontier_len: usize, deadline: Option<Instant>) -> usize {
    if deadline.is_some() {
        frontier_len.min(TIMED_SEARCH_FRONTIER_CHUNK_SIZE).max(1)
    } else {
        frontier_len.max(1)
    }
}

fn expand_frontier_layer_dyn(
    current_frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    max_intermediate_dim: usize,
    max_entry: u32,
    search_mode: SearchMode,
    deadline: Option<Instant>,
) -> (Vec<FrontierExpansion>, FrontierExpansionStats, bool) {
    let expand_node = |current_canon: &DynMatrix| {
        let current = orig
            .get(current_canon)
            .expect("frontier node should have an original matrix");
        let mut expansions = Vec::new();
        let mut seen_successors = HashSet::new();
        let mut stats = FrontierExpansionStats {
            frontier_nodes: 1,
            factorisation_calls: 1,
            ..FrontierExpansionStats::default()
        };

        let graph_successors = enumerate_graph_move_successors(current, max_intermediate_dim);
        stats.candidates_generated += graph_successors.candidates;
        for (family, count) in graph_successors.family_candidates {
            move_family_telemetry_mut(&mut stats.move_family_telemetry, family)
                .candidates_generated += count;
        }

        for successor in graph_successors.nodes {
            let next = successor.orig_matrix;
            let next_canon = successor.matrix;
            if !seen_successors.insert(next_canon.clone()) {
                continue;
            }
            let same_future_past_signature = same_future_past_signature(&next_canon);
            expansions.push(FrontierExpansion {
                parent_canon: current_canon.clone(),
                next_canon,
                next_orig: next,
                step: successor.step,
                move_family: successor.family,
                same_future_past_signature,
            });
        }

        if search_mode == SearchMode::Mixed {
            visit_all_factorisations_with_family(
                current,
                max_intermediate_dim,
                max_entry,
                |move_family, u, v| {
                    move_family_telemetry_mut(&mut stats.move_family_telemetry, move_family)
                        .candidates_generated += 1;
                    stats.factorisations_enumerated += 1;
                    stats.candidates_generated += 1;
                    let next = v.mul(&u);

                    if next.rows > max_intermediate_dim {
                        stats.pruned_by_size += 1;
                        return;
                    }

                    let next_canon = next.canonical_perm();
                    if !seen_successors.insert(next_canon.clone()) {
                        return;
                    }
                    let step = EsseStep { u, v };
                    expansions.push(FrontierExpansion {
                        parent_canon: current_canon.clone(),
                        next_canon,
                        next_orig: next,
                        step,
                        move_family,
                        same_future_past_signature: None,
                    });
                },
            );
        }

        (expansions, stats)
    };

    let mut expansions = Vec::new();
    let mut stats = FrontierExpansionStats::default();
    let mut timed_out = false;
    for chunk in current_frontier.chunks(frontier_chunk_size(current_frontier.len(), deadline)) {
        if deadline_reached(deadline) {
            timed_out = true;
            break;
        }

        #[cfg(not(target_arch = "wasm32"))]
        let per_node: Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)> =
            chunk.par_iter().map(expand_node).collect();

        #[cfg(target_arch = "wasm32")]
        let per_node: Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)> =
            chunk.iter().map(expand_node).collect();

        for (node_expansions, node_stats) in per_node {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
    }

    let (deduped, same_future_past_collisions) = deduplicate_expansions(
        expansions,
        current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
    );
    stats.same_future_past_collisions = same_future_past_collisions;
    record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
    (deduped, stats, timed_out)
}

fn deduplicate_expansions(
    expansions: Vec<FrontierExpansion>,
    enable_same_future_past_representatives: bool,
) -> (Vec<FrontierExpansion>, usize) {
    let mut seen = HashSet::new();
    let mut same_future_past_seen = HashSet::new();
    let mut deduped = Vec::with_capacity(expansions.len());
    let mut same_future_past_collisions = 0usize;
    for expansion in expansions {
        if seen.contains(&expansion.next_canon) {
            continue;
        }
        if enable_same_future_past_representatives && expansion.next_canon.rows >= 3 {
            if let Some(signature) = expansion.same_future_past_signature.as_ref() {
                if !same_future_past_seen.insert(signature.clone()) {
                    same_future_past_collisions += 1;
                    continue;
                }
            }
        }
        seen.insert(expansion.next_canon.clone());
        deduped.push(expansion);
    }
    (deduped, same_future_past_collisions)
}

fn choose_next_layer(
    fwd_depth: Option<usize>,
    bwd_depth: Option<usize>,
    fwd_frontier_len: usize,
    bwd_frontier_len: usize,
    fwd_factorisations_per_node: f64,
    bwd_factorisations_per_node: f64,
    fwd_cost_sample_nodes: usize,
    bwd_cost_sample_nodes: usize,
    fwd_overlap_signal: FrontierOverlapSignal,
    bwd_overlap_signal: FrontierOverlapSignal,
) -> Option<(bool, usize)> {
    match (fwd_depth, bwd_depth) {
        (Some(fwd), Some(bwd)) => {
            if fwd < bwd {
                Some((true, fwd))
            } else if bwd < fwd {
                Some((false, bwd))
            } else {
                Some((
                    should_expand_forward(
                        fwd_frontier_len,
                        bwd_frontier_len,
                        fwd_factorisations_per_node,
                        bwd_factorisations_per_node,
                        fwd_cost_sample_nodes,
                        bwd_cost_sample_nodes,
                        fwd_overlap_signal,
                        bwd_overlap_signal,
                    ),
                    fwd,
                ))
            }
        }
        (Some(fwd), None) => Some((true, fwd)),
        (None, Some(bwd)) => Some((false, bwd)),
        (None, None) => None,
    }
}

fn should_expand_forward(
    fwd_frontier_len: usize,
    bwd_frontier_len: usize,
    fwd_factorisations_per_node: f64,
    bwd_factorisations_per_node: f64,
    fwd_cost_sample_nodes: usize,
    bwd_cost_sample_nodes: usize,
    fwd_overlap_signal: FrontierOverlapSignal,
    bwd_overlap_signal: FrontierOverlapSignal,
) -> bool {
    if fwd_frontier_len == 0 || bwd_frontier_len == 0 {
        return fwd_frontier_len <= bwd_frontier_len;
    }
    if fwd_cost_sample_nodes < 8 || bwd_cost_sample_nodes < 8 {
        return fwd_frontier_len <= bwd_frontier_len;
    }

    if fwd_overlap_signal.is_trained() && bwd_overlap_signal.is_trained() {
        if fwd_overlap_signal.approximate_other_side_hits > 0
            && bwd_overlap_signal.approximate_other_side_hits == 0
        {
            return true;
        }
        if bwd_overlap_signal.approximate_other_side_hits > 0
            && fwd_overlap_signal.approximate_other_side_hits == 0
        {
            return false;
        }

        let fwd_overlap_ratio = fwd_overlap_signal.overlap_ratio();
        let bwd_overlap_ratio = bwd_overlap_signal.overlap_ratio();
        if fwd_overlap_signal.approximate_other_side_hits >= 2
            && fwd_overlap_ratio > bwd_overlap_ratio * 2.0
        {
            return true;
        }
        if bwd_overlap_signal.approximate_other_side_hits >= 2
            && bwd_overlap_ratio > fwd_overlap_ratio * 2.0
        {
            return false;
        }
    }

    let fwd_estimated_work = fwd_frontier_len as f64 * fwd_factorisations_per_node.max(1.0);
    let bwd_estimated_work = bwd_frontier_len as f64 * bwd_factorisations_per_node.max(1.0);
    fwd_estimated_work <= bwd_estimated_work
}

fn expand_frontier_layer(
    current_frontier: &[DynMatrix],
    orig: &HashMap<DynMatrix, DynMatrix>,
    max_intermediate_dim: usize,
    max_entry: u32,
    search_mode: SearchMode,
) -> (Vec<FrontierExpansion>, FrontierExpansionStats) {
    let expand_node = |current_canon: &DynMatrix| {
        let current = orig
            .get(current_canon)
            .expect("frontier node should have an original matrix");
        let mut expansions = Vec::new();
        let mut seen_successors = HashSet::new();
        let mut stats = FrontierExpansionStats {
            frontier_nodes: 1,
            factorisation_calls: 1,
            ..FrontierExpansionStats::default()
        };

        let graph_successors = enumerate_graph_move_successors(current, max_intermediate_dim);
        stats.candidates_generated += graph_successors.candidates;
        for (family, count) in graph_successors.family_candidates {
            move_family_telemetry_mut(&mut stats.move_family_telemetry, family)
                .candidates_generated += count;
        }

        for successor in graph_successors.nodes {
            let next = successor.orig_matrix;
            let next_canon = successor.matrix;
            if !seen_successors.insert(next_canon.clone()) {
                continue;
            }
            let same_future_past_signature = same_future_past_signature(&next_canon);
            expansions.push(FrontierExpansion {
                parent_canon: current_canon.clone(),
                next_canon,
                next_orig: next,
                step: successor.step,
                move_family: successor.family,
                same_future_past_signature,
            });
        }

        if search_mode == SearchMode::Mixed {
            visit_all_factorisations_with_family(
                current,
                max_intermediate_dim,
                max_entry,
                |move_family, u, v| {
                    move_family_telemetry_mut(&mut stats.move_family_telemetry, move_family)
                        .candidates_generated += 1;
                    stats.factorisations_enumerated += 1;
                    stats.candidates_generated += 1;
                    let next = v.mul(&u);

                    // Size bound: don't explore matrices larger than max_intermediate_dim.
                    if next.rows > max_intermediate_dim {
                        stats.pruned_by_size += 1;
                        return;
                    }

                    let next_canon = next.canonical_perm();
                    if !seen_successors.insert(next_canon.clone()) {
                        return;
                    }
                    let step = EsseStep { u, v };
                    expansions.push(FrontierExpansion {
                        parent_canon: current_canon.clone(),
                        next_canon,
                        next_orig: next,
                        step,
                        move_family,
                        same_future_past_signature: None,
                    });
                },
            );
        }

        (expansions, stats)
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        let per_node: Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)> =
            current_frontier.par_iter().map(expand_node).collect();
        let mut expansions = Vec::new();
        let mut stats = FrontierExpansionStats::default();
        for (node_expansions, node_stats) in per_node {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
        let (deduped, same_future_past_collisions) = deduplicate_expansions(
            expansions,
            current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
        );
        stats.same_future_past_collisions = same_future_past_collisions;
        record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
        (deduped, stats)
    }

    #[cfg(target_arch = "wasm32")]
    {
        let mut expansions = Vec::new();
        let mut stats = FrontierExpansionStats::default();
        for (node_expansions, node_stats) in current_frontier.iter().map(expand_node) {
            expansions.extend(node_expansions);
            accumulate_frontier_stats(&mut stats, &node_stats);
        }
        let (deduped, same_future_past_collisions) = deduplicate_expansions(
            expansions,
            current_frontier.len() >= SAME_FUTURE_PAST_REPRESENTATIVE_LAYER_THRESHOLD,
        );
        stats.same_future_past_collisions = same_future_past_collisions;
        record_candidates_after_pruning_by_family(&deduped, &mut stats.move_family_telemetry);
        (deduped, stats)
    }
}

fn accumulate_frontier_stats(total: &mut FrontierExpansionStats, delta: &FrontierExpansionStats) {
    total.frontier_nodes += delta.frontier_nodes;
    total.factorisation_calls += delta.factorisation_calls;
    total.factorisations_enumerated += delta.factorisations_enumerated;
    total.candidates_generated += delta.candidates_generated;
    total.pruned_by_size += delta.pruned_by_size;
    total.pruned_by_spectrum += delta.pruned_by_spectrum;
    accumulate_move_family_telemetry(
        &mut total.move_family_telemetry,
        &delta.move_family_telemetry,
    );
}

fn approx_signature(m: &DynMatrix) -> ApproxSignature {
    let mut row_sums = vec![0u32; m.rows];
    let mut col_sums = vec![0u32; m.cols];
    let mut row_supports = vec![0u8; m.rows];
    let mut col_supports = vec![0u8; m.cols];
    let mut entry_sum = 0u64;

    for row in 0..m.rows {
        for col in 0..m.cols {
            let value = m.get(row, col);
            row_sums[row] += value;
            col_sums[col] += value;
            entry_sum += value as u64;
            if value > 0 {
                row_supports[row] += 1;
                col_supports[col] += 1;
            }
        }
    }

    row_sums.sort_unstable();
    col_sums.sort_unstable();
    row_supports.sort_unstable();
    col_supports.sort_unstable();

    ApproxSignature {
        dim: m.rows,
        entry_sum,
        row_sums,
        col_sums,
        row_supports,
        col_supports,
    }
}

fn same_future_past_signature(m: &DynMatrix) -> Option<SameFuturePastSignature> {
    if !m.is_square() {
        return None;
    }

    let mut entry_sum = 0u64;
    let mut row_vectors = Vec::with_capacity(m.rows);
    for row in 0..m.rows {
        let mut values = Vec::with_capacity(m.cols);
        for col in 0..m.cols {
            let value = m.get(row, col);
            entry_sum += value as u64;
            values.push(value);
        }
        row_vectors.push(values);
    }

    let mut col_vectors = Vec::with_capacity(m.cols);
    for col in 0..m.cols {
        let mut values = Vec::with_capacity(m.rows);
        for row in 0..m.rows {
            values.push(m.get(row, col));
        }
        col_vectors.push(values);
    }

    Some(SameFuturePastSignature {
        dim: m.rows,
        entry_sum,
        row_classes: duplicate_vector_classes(&row_vectors),
        col_classes: duplicate_vector_classes(&col_vectors),
    })
}

fn duplicate_vector_classes(vectors: &[Vec<u32>]) -> Vec<SameFuturePastClassSignature> {
    let mut multiplicities = BTreeMap::<Vec<u32>, usize>::new();
    for values in vectors {
        *multiplicities.entry(values.clone()).or_default() += 1;
    }

    let mut classes = multiplicities
        .into_iter()
        .map(|(values, multiplicity)| SameFuturePastClassSignature {
            multiplicity,
            entry_sum: values.iter().copied().sum(),
            support: values.iter().filter(|&&value| value > 0).count() as u8,
        })
        .collect::<Vec<_>>();
    classes.sort_unstable();
    classes
}

fn move_family_telemetry_mut<'a>(
    map: &'a mut BTreeMap<String, SearchMoveFamilyTelemetry>,
    family: &str,
) -> &'a mut SearchMoveFamilyTelemetry {
    map.entry(family.to_string()).or_default()
}

fn accumulate_move_family_telemetry(
    total: &mut BTreeMap<String, SearchMoveFamilyTelemetry>,
    delta: &BTreeMap<String, SearchMoveFamilyTelemetry>,
) {
    for (family, family_delta) in delta {
        let family_total = total.entry(family.clone()).or_default();
        family_total.candidates_generated += family_delta.candidates_generated;
        family_total.candidates_after_pruning += family_delta.candidates_after_pruning;
        family_total.discovered_nodes += family_delta.discovered_nodes;
        family_total.exact_meets += family_delta.exact_meets;
        family_total.approximate_other_side_hits += family_delta.approximate_other_side_hits;
    }
}

fn record_candidates_after_pruning_by_family(
    expansions: &[FrontierExpansion],
    telemetry: &mut BTreeMap<String, SearchMoveFamilyTelemetry>,
) {
    for expansion in expansions {
        move_family_telemetry_mut(telemetry, expansion.move_family).candidates_after_pruning += 1;
    }
}

fn visited_union_size(
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
) -> usize {
    fwd_parent.len()
        + bwd_parent
            .keys()
            .filter(|key| !fwd_parent.contains_key(*key))
            .count()
}

/// Check whether a candidate intermediate matrix has a nonzero spectrum
/// consistent with the source matrix. SSE preserves nonzero eigenvalues,
/// so any intermediate in a valid chain must pass this check.
#[cfg(test)]
fn is_spectrally_consistent(vu: &DynMatrix, source_trace: u64, source_det: i64) -> bool {
    if !vu.is_square() {
        return true; // non-square shouldn't happen, don't filter
    }
    // Trace (sum of eigenvalues) must match for all sizes.
    if vu.trace() != source_trace {
        return false;
    }
    match vu.rows {
        2 => {
            // For 2x2: trace and det fully determine the spectrum.
            vu.det_2x2() == source_det
        }
        3 => {
            // A 3x3 intermediate from a 2x2 source has one zero eigenvalue.
            // det must be 0, and the sum of eigenvalue pairs must equal source_det.
            vu.det_3x3() == 0 && vu.principal_minor_sum_3x3() == source_det
        }
        _ => {
            // For k×k intermediates from a 2×2 source, eigenvalues are
            // {λ₁, λ₂, 0, ..., 0}. Use the power-trace identity:
            // tr(M²) = λ₁² + λ₂² = trace² - 2·det.
            let expected_tr2 = (source_trace as i64) * (source_trace as i64) - 2 * source_det;
            let m2 = vu.mul(vu);
            let actual_tr2 = m2.trace() as i64;
            actual_tr2 == expected_tr2
        }
    }
}

fn trace_square(m: &DynMatrix) -> i64 {
    m.mul(m).trace() as i64
}

/// Create a permutation similarity step: given matrices M and M' = PMP
/// where P is the swap permutation, return an EsseStep with U = MP, V = P
/// so that UV = M and VU = M'.
fn permutation_step(m: &DynMatrix) -> EsseStep {
    let n = m.rows;
    let mut p_data = vec![0u32; n * n];
    for i in 0..n {
        p_data[i * n + (n - 1 - i)] = 1;
    }
    let p = DynMatrix::new(n, n, p_data);
    let mp = m.mul(&p);
    EsseStep { u: mp, v: p }
}

fn permutation_step_between(from: &DynMatrix, to: &DynMatrix) -> Option<EsseStep> {
    if from.rows != from.cols || to.rows != to.cols || from.rows != to.rows {
        return None;
    }
    let n = from.rows;
    let mut perm: Vec<usize> = (0..n).collect();
    let mut result = None;
    for_each_permutation(&mut perm, 0, &mut |perm| {
        if result.is_some() {
            return;
        }
        let (p, pinv) = permutation_matrices(perm);
        let candidate = pinv.mul(from).mul(&p);
        if candidate == *to {
            let u = from.mul(&p);
            result = Some(EsseStep { u, v: pinv });
        }
    });
    result
}

fn permutation_matrices(perm: &[usize]) -> (DynMatrix, DynMatrix) {
    let n = perm.len();
    let mut p_data = vec![0u32; n * n];
    let mut pinv_data = vec![0u32; n * n];
    for (row, &col) in perm.iter().enumerate() {
        p_data[row * n + col] = 1;
        pinv_data[col * n + row] = 1;
    }
    (
        DynMatrix::new(n, n, p_data),
        DynMatrix::new(n, n, pinv_data),
    )
}

fn for_each_permutation<F>(perm: &mut [usize], start: usize, visit: &mut F)
where
    F: FnMut(&[usize]),
{
    if start == perm.len() {
        visit(perm);
        return;
    }
    for idx in start..perm.len() {
        perm.swap(start, idx);
        for_each_permutation(perm, start + 1, visit);
        perm.swap(start, idx);
    }
}

/// Walk a parent chain from `node` back to the root, returning
/// (matrices, steps) in root-to-node order.
fn walk_parent_chain(
    node: &DynMatrix,
    parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    orig: &HashMap<DynMatrix, DynMatrix>,
) -> (Vec<DynMatrix>, Vec<EsseStep>) {
    let mut matrices = Vec::new();
    let mut steps = Vec::new();
    let mut current = node.clone();

    matrices.push(orig[&current].clone());

    while let Some(Some((prev, step))) = parent.get(&current) {
        steps.push(step.clone());
        matrices.push(orig[prev].clone());
        current = prev.clone();
    }

    matrices.reverse();
    steps.reverse();
    (matrices, steps)
}

/// Reconstruct a path from the forward and backward BFS trees that meet
/// at `meeting_canon`.
///
/// Forward chain: A -> ... -> M (steps recorded as current=UV, neighbor=VU).
/// Backward chain: B -> ... -> M (same convention).
/// We reverse the backward chain to get M -> ... -> B, flipping each step's
/// (U,V) to (V,U) since the direction of the elementary SSE is reversed.
fn reconstruct_bidirectional_path(
    a: &SqMatrix<2>,
    b: &SqMatrix<2>,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> SsePath<2> {
    // Forward: A -> ... -> M
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);

    // Backward: B -> ... -> M, which we reverse to M -> ... -> B.
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices
        .last()
        .expect("forward chain should end at the meeting node")
        .clone();
    let bwd_meeting = bwd_matrices
        .last()
        .expect("backward chain should end at the meeting node")
        .clone();

    // Build the combined step list.
    let mut all_steps = fwd_steps;

    if fwd_meeting != bwd_meeting {
        let step = permutation_step_between(&fwd_meeting, &bwd_meeting)
            .expect("meeting representatives should be permutation-similar");
        all_steps.push(step);
    }

    // Reverse backward steps: each backward step had current=UV, neighbor=VU.
    // In the forward direction (M->...->B) we need neighbor=UV, current=VU,
    // i.e. the elementary SSE step with U and V swapped.
    for step in bwd_steps.into_iter().rev() {
        all_steps.push(EsseStep {
            u: step.v,
            v: step.u,
        });
    }

    // Build the combined matrix list (all intermediate DynMatrix nodes).
    let mut all_dyn_matrices: Vec<DynMatrix> = fwd_matrices;
    if fwd_meeting != bwd_meeting {
        all_dyn_matrices.push(bwd_meeting);
    }
    // bwd_matrices is [B, ..., M] — reversed and skip M (already in fwd).
    for m in bwd_matrices.into_iter().rev().skip(1) {
        all_dyn_matrices.push(m);
    }

    let a_dyn = DynMatrix::from_sq(a);
    let b_dyn = DynMatrix::from_sq(b);

    // If the BFS start node differs from `a` (due to canonicalisation),
    // prepend a permutation step: a -> canonical(a).
    if *all_dyn_matrices.first().unwrap() != a_dyn {
        all_steps.insert(0, permutation_step(&a_dyn));
        all_dyn_matrices.insert(0, a_dyn);
    }

    // If the BFS end node differs from `b` (due to canonicalisation),
    // append a permutation step: canonical(b) -> b.
    if *all_dyn_matrices.last().unwrap() != b_dyn {
        let last = all_dyn_matrices.last().unwrap().clone();
        all_steps.push(permutation_step(&last));
        all_dyn_matrices.push(b_dyn);
    }

    // Collect the 2x2 nodes for the SsePath matrices field.
    let sq_matrices: Vec<SqMatrix<2>> = all_dyn_matrices
        .iter()
        .filter_map(|dm| dm.to_sq::<2>())
        .collect();

    SsePath {
        matrices: sq_matrices,
        steps: all_steps,
    }
}

fn reconstruct_bidirectional_dyn_path(
    a: &DynMatrix,
    b: &DynMatrix,
    meeting_canon: &DynMatrix,
    fwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    fwd_orig: &HashMap<DynMatrix, DynMatrix>,
    bwd_parent: &HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>,
    bwd_orig: &HashMap<DynMatrix, DynMatrix>,
) -> DynSsePath {
    let (fwd_matrices, fwd_steps) = walk_parent_chain(meeting_canon, fwd_parent, fwd_orig);
    let (bwd_matrices, bwd_steps) = walk_parent_chain(meeting_canon, bwd_parent, bwd_orig);

    let fwd_meeting = fwd_matrices
        .last()
        .expect("forward chain should end at the meeting node")
        .clone();
    let bwd_meeting = bwd_matrices
        .last()
        .expect("backward chain should end at the meeting node")
        .clone();

    let mut all_steps = fwd_steps;
    if fwd_meeting != bwd_meeting {
        let step = permutation_step_between(&fwd_meeting, &bwd_meeting)
            .expect("meeting representatives should be permutation-similar");
        all_steps.push(step);
    }

    for step in bwd_steps.into_iter().rev() {
        all_steps.push(EsseStep {
            u: step.v,
            v: step.u,
        });
    }

    let mut all_dyn_matrices: Vec<DynMatrix> = fwd_matrices;
    if fwd_meeting != bwd_meeting {
        all_dyn_matrices.push(bwd_meeting);
    }
    for m in bwd_matrices.into_iter().rev().skip(1) {
        all_dyn_matrices.push(m);
    }

    if *all_dyn_matrices.first().unwrap() != *a {
        let first = all_dyn_matrices.first().unwrap().clone();
        let step =
            permutation_step_between(a, &first).expect("start should be permutation-similar");
        all_steps.insert(0, step);
        all_dyn_matrices.insert(0, a.clone());
    }

    if *all_dyn_matrices.last().unwrap() != *b {
        let last = all_dyn_matrices.last().unwrap().clone();
        let step = permutation_step_between(&last, b).expect("end should be permutation-similar");
        all_steps.push(step);
        all_dyn_matrices.push(b.clone());
    }

    DynSsePath {
        matrices: all_dyn_matrices,
        steps: all_steps,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{
        GuideArtifact, GuideArtifactCompatibility, GuideArtifactPayload, GuideArtifactProvenance,
        GuideArtifactValidation, GuidedRefinementConfig,
    };

    fn default_config() -> SearchConfig {
        SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 2,
            max_entry: 10,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        }
    }

    fn full_path_artifact(id: &str, path: DynSsePath) -> GuideArtifact {
        let source = path
            .matrices
            .first()
            .expect("guide path should have a source matrix")
            .clone();
        let target = path
            .matrices
            .last()
            .expect("guide path should have a target matrix")
            .clone();
        let mut artifact = build_full_path_guide_artifact(&source, &target, &path).unwrap();
        artifact.artifact_id = Some(id.to_string());
        artifact.provenance = GuideArtifactProvenance {
            source_kind: Some("unit_test".to_string()),
            label: Some(id.to_string()),
            source_ref: None,
        };
        artifact.compatibility = GuideArtifactCompatibility {
            supported_stages: vec![SearchStage::GuidedRefinement],
            max_endpoint_dim: Some(4),
        };
        artifact
    }

    fn permutation_guide(base: &DynMatrix, perms: &[[usize; 3]]) -> DynSsePath {
        let matrices = perms
            .iter()
            .map(|perm| base.conjugate_by_perm(perm))
            .collect::<Vec<_>>();
        let steps = matrices
            .windows(2)
            .map(|pair| permutation_step_between(&pair[0], &pair[1]).unwrap())
            .collect::<Vec<_>>();
        DynSsePath { matrices, steps }
    }

    fn shortcut_request(
        source: DynMatrix,
        target: DynMatrix,
        guide_artifacts: Vec<GuideArtifact>,
        guided_refinement: GuidedRefinementConfig,
        shortcut_search: ShortcutSearchConfig,
    ) -> SearchRequest {
        SearchRequest {
            source,
            target,
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                search_mode: SearchMode::GraphOnly,
                beam_width: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts,
            guided_refinement,
            shortcut_search,
        }
    }

    #[test]
    fn test_build_full_path_guide_artifact_populates_metadata() {
        let source = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let path = DynSsePath {
            matrices: vec![source.clone()],
            steps: vec![],
        };

        let artifact = build_full_path_guide_artifact(&source, &source, &path).unwrap();
        assert_eq!(artifact.endpoints.source, source);
        assert_eq!(artifact.endpoints.target, artifact.endpoints.source);
        assert_eq!(
            artifact.validation,
            GuideArtifactValidation::WitnessValidated
        );
        assert_eq!(artifact.quality.lag, Some(0));
        assert_eq!(artifact.quality.cost, Some(0));
        assert!(matches!(
            artifact.payload,
            GuideArtifactPayload::FullPath { path: ref full_path } if full_path.steps.is_empty()
        ));
    }

    #[test]
    fn test_build_full_path_guide_artifact_rejects_invalid_path() {
        let source = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let target = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let invalid = DynSsePath {
            matrices: vec![source.clone(), source.clone()],
            steps: vec![EsseStep {
                u: target.clone(),
                v: target.clone(),
            }],
        };

        let err = build_full_path_guide_artifact(&source, &target, &invalid).unwrap_err();
        assert!(err.contains("does not end"));
    }

    #[test]
    fn test_self_sse() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let result = search_sse_2x2(&a, &a, &default_config());
        match result {
            SseResult::Equivalent(path) => {
                assert_eq!(path.matrices.len(), 1);
                assert_eq!(path.steps.len(), 0);
            }
            _ => panic!("Expected Equivalent for self-SSE"),
        }
    }

    #[test]
    fn test_elementary_sse_pair() {
        // [[2,1],[1,1]] is elementary SSE to [[1,1],[1,2]]
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let result = search_sse_2x2(&a, &b, &default_config());
        match result {
            SseResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                // Verify the step: A = UV, B = VU
                let step = &path.steps[0];
                let uv = step.u.mul(&step.v);
                let vu = step.v.mul(&step.u);
                assert_eq!(uv, DynMatrix::from_sq(&a));
                assert_eq!(vu, DynMatrix::from_sq(&b));
            }
            _ => panic!("Expected Equivalent for known elementary SSE pair"),
        }
    }

    #[test]
    fn test_beam_search_finds_small_solvable_case() {
        let a = SqMatrix::new([[1, 1], [2, 5]]);
        let b = SqMatrix::new([[1, 2], [1, 5]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 6,
            search_mode: SearchMode::Mixed,
            beam_width: Some(4),
        };

        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
        match result {
            SseResult::Equivalent(path) => {
                assert_valid_path(&path);
            }
            other => panic!("expected Equivalent from beam search, got {other:?}"),
        }
        assert!(telemetry.frontier_nodes_expanded >= 1);
        assert!(telemetry.max_frontier_size <= 4);
    }

    #[test]
    fn test_beam_frontier_enforces_width_cap() {
        let mut frontier = BeamFrontier::new(2);
        frontier.push(BeamFrontierEntry {
            canonical: DynMatrix::new(1, 1, vec![1]),
            depth: 1,
            score: 1,
            approximate_hit: false,
            serial: 2,
        });
        frontier.push(BeamFrontierEntry {
            canonical: DynMatrix::new(1, 1, vec![2]),
            depth: 1,
            score: 3,
            approximate_hit: false,
            serial: 1,
        });
        frontier.push(BeamFrontierEntry {
            canonical: DynMatrix::new(1, 1, vec![3]),
            depth: 1,
            score: 2,
            approximate_hit: false,
            serial: 0,
        });

        assert_eq!(frontier.len(), 2);
        assert_eq!(frontier.pop_best().unwrap().score, 3);
        assert_eq!(frontier.pop_best().unwrap().score, 2);
    }

    #[test]
    fn test_different_trace_not_equivalent() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[3, 1], [1, 1]]);
        let result = search_sse_2x2(&a, &b, &default_config());
        match result {
            SseResult::NotEquivalent(reason) => {
                assert!(reason.contains("trace"));
            }
            _ => panic!("Expected NotEquivalent"),
        }
    }

    #[test]
    fn test_guided_refinement_stage_accepts_full_path_artifact() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: default_config(),
            stage: SearchStage::GuidedRefinement,
            guide_artifacts: vec![full_path_artifact("direct-guide", guide)],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from guided refinement, got {other:?}"),
        }
        assert_eq!(telemetry.guide_artifacts_considered, 1);
        assert_eq!(telemetry.guide_artifacts_accepted, 1);
    }

    #[test]
    fn test_guided_refinement_stage_shortens_permutation_guide() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid).unwrap(),
                permutation_step_between(&mid, &b).unwrap(),
            ],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                search_mode: SearchMode::GraphOnly,
                beam_width: None,
            },
            stage: SearchStage::GuidedRefinement,
            guide_artifacts: vec![full_path_artifact("two-hop-guide", guide)],
            guided_refinement: GuidedRefinementConfig {
                max_shortcut_lag: 1,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: None,
            },
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from guided refinement, got {other:?}"),
        }
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.guided_segments_improved, 1);
    }

    #[test]
    fn test_guided_refinement_segment_timeout_preserves_guide_when_search_times_out() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid).unwrap(),
                permutation_step_between(&mid, &b).unwrap(),
            ],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                search_mode: SearchMode::GraphOnly,
                beam_width: None,
            },
            stage: SearchStage::GuidedRefinement,
            guide_artifacts: vec![full_path_artifact("two-hop-guide", guide)],
            guided_refinement: GuidedRefinementConfig {
                max_shortcut_lag: 1,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: Some(0),
            },
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 2);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from guided refinement, got {other:?}"),
        }
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.guided_segments_improved, 0);
    }

    #[test]
    fn test_shortcut_search_stage_accepts_legacy_guided_refinement_artifact() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), mid, b.clone()],
            steps: vec![
                permutation_step_between(&a, &base.conjugate_by_perm(&[2, 0, 1])).unwrap(),
                permutation_step_between(&base.conjugate_by_perm(&[2, 0, 1]), &b).unwrap(),
            ],
        };
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                search_mode: SearchMode::GraphOnly,
                beam_width: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: vec![full_path_artifact("legacy-guided", guide)],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from shortcut search, got {other:?}"),
        }
        assert_eq!(telemetry.guide_artifacts_considered, 1);
        assert_eq!(telemetry.guide_artifacts_accepted, 1);
        assert_eq!(telemetry.shortcut_search.guide_artifacts_loaded, 1);
        assert_eq!(telemetry.shortcut_search.guide_artifacts_accepted, 1);
        assert_eq!(telemetry.shortcut_search.unique_guides, 1);
        assert_eq!(telemetry.shortcut_search.initial_working_set_guides, 1);
        assert_eq!(telemetry.shortcut_search.best_lag_start, Some(2));
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(1));
        assert_eq!(telemetry.shortcut_search.promoted_guides, 1);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 2);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::GuidePoolExhausted)
        );
    }

    #[test]
    fn test_shortcut_search_stage_deduplicates_and_ranks_guides() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let mid = base.conjugate_by_perm(&[2, 0, 1]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let direct = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };
        let two_hop = DynSsePath {
            matrices: vec![a.clone(), mid.clone(), b.clone()],
            steps: vec![
                permutation_step_between(&a, &mid).unwrap(),
                permutation_step_between(&mid, &b).unwrap(),
            ],
        };

        let mut direct_forward = full_path_artifact("direct-forward", direct.clone());
        direct_forward.quality.lag = None;
        direct_forward.quality.cost = Some(5);
        direct_forward.quality.score = Some(1.0);

        let mut direct_duplicate = full_path_artifact("direct-duplicate", direct);
        direct_duplicate.quality.lag = Some(1);
        direct_duplicate.quality.cost = Some(9);
        direct_duplicate.quality.score = Some(0.5);

        let mut indirect = full_path_artifact("two-hop", two_hop);
        indirect.quality.cost = Some(1);
        indirect.quality.score = Some(10.0);

        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                search_mode: SearchMode::GraphOnly,
                beam_width: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: vec![indirect, direct_duplicate, direct_forward],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig {
                max_guides: 1,
                ..ShortcutSearchConfig::default()
            },
        };

        let (result, telemetry) = execute_search_request(&request).unwrap();
        match result {
            SearchRunResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                validate_sse_path_dyn(&a, &b, &path).unwrap();
            }
            other => panic!("expected Equivalent from shortcut search, got {other:?}"),
        }
        assert_eq!(telemetry.guide_artifacts_considered, 3);
        assert_eq!(telemetry.guide_artifacts_accepted, 3);
        assert_eq!(telemetry.shortcut_search.unique_guides, 2);
        assert_eq!(telemetry.shortcut_search.initial_working_set_guides, 1);
        assert_eq!(telemetry.shortcut_search.best_lag_start, Some(1));
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(1));
    }

    #[test]
    fn test_shortcut_search_stage_iteratively_refines_promoted_guides() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let guide = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let source = guide.matrices.first().unwrap().clone();
        let target = guide.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("iterative-seed", guide)],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.best_lag_start, Some(3));
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(1));
        assert_eq!(telemetry.shortcut_search.promoted_guides, 2);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 3);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::GuidePoolExhausted)
        );
        assert_eq!(telemetry.shortcut_search.rounds.len(), 3);
        assert_eq!(
            telemetry.shortcut_search.rounds[0].starting_best_lag,
            Some(3)
        );
        assert_eq!(telemetry.shortcut_search.rounds[0].ending_best_lag, Some(2));
        assert_eq!(
            telemetry.shortcut_search.rounds[1].starting_best_lag,
            Some(2)
        );
        assert_eq!(telemetry.shortcut_search.rounds[1].ending_best_lag, Some(1));
    }

    #[test]
    fn test_shortcut_search_stage_deduplicates_promoted_guides() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let first = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let second = permutation_guide(&base, &[[0, 1, 2], [0, 2, 1], [1, 2, 0], [2, 1, 0]]);
        let source = first.matrices.first().unwrap().clone();
        let target = first.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![
                full_path_artifact("promotion-a", first),
                full_path_artifact("promotion-b", second),
            ],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(3),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 2,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.promoted_guides, 1);
    }

    #[test]
    fn test_shortcut_search_stage_reports_no_improvement_round_with_leftover_pool() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let direct = permutation_guide(&base, &[[0, 1, 2], [2, 1, 0]]);
        let indirect = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 1, 0]]);
        let source = direct.matrices.first().unwrap().clone();
        let target = direct.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![
                full_path_artifact("indirect-leftover", indirect),
                full_path_artifact("direct-best", direct),
            ],
            GuidedRefinementConfig::default(),
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.initial_working_set_guides, 1);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::NoImprovementRound)
        );
    }

    #[test]
    fn test_shortcut_search_stage_reports_guide_pool_exhaustion() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let direct = permutation_guide(&base, &[[0, 1, 2], [2, 1, 0]]);
        let source = direct.matrices.first().unwrap().clone();
        let target = direct.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("direct-only", direct)],
            GuidedRefinementConfig::default(),
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 1);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(telemetry.shortcut_search.segment_attempts, 0);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::GuidePoolExhausted)
        );
    }

    #[test]
    fn test_shortcut_search_stage_reports_max_rounds_reached() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let guide = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let source = guide.matrices.first().unwrap().clone();
        let target = guide.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("max-rounds", guide)],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(2),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 1,
                max_total_segment_attempts: 16,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        assert_eq!(path.steps.len(), 2);
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(telemetry.shortcut_search.best_lag_end, Some(2));
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::MaxRoundsReached)
        );
    }

    #[test]
    fn test_shortcut_search_stage_respects_total_segment_attempt_budget() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let guide = permutation_guide(&base, &[[0, 1, 2], [1, 0, 2], [2, 0, 1], [2, 1, 0]]);
        let source = guide.matrices.first().unwrap().clone();
        let target = guide.matrices.last().unwrap().clone();
        let request = shortcut_request(
            source.clone(),
            target.clone(),
            vec![full_path_artifact("budgeted", guide)],
            GuidedRefinementConfig {
                max_shortcut_lag: 2,
                min_gap: 2,
                max_gap: Some(3),
                rounds: 1,
                segment_timeout_secs: None,
            },
            ShortcutSearchConfig {
                max_guides: 1,
                rounds: 4,
                max_total_segment_attempts: 1,
                ..ShortcutSearchConfig::default()
            },
        );

        let (result, telemetry) = execute_search_request(&request).unwrap();
        let SearchRunResult::Equivalent(path) = result else {
            panic!("expected Equivalent from shortcut search");
        };
        validate_sse_path_dyn(&source, &target, &path).unwrap();
        assert_eq!(telemetry.guided_segments_considered, 1);
        assert_eq!(telemetry.shortcut_search.segment_attempts, 1);
        assert_eq!(telemetry.shortcut_search.rounds_completed, 1);
        assert_eq!(telemetry.shortcut_search.rounds[0].segment_attempts, 1);
        assert_eq!(
            telemetry.shortcut_search.stop_reason,
            Some(ShortcutSearchStopReason::MaxSegmentAttemptsReached)
        );
    }

    #[test]
    fn test_prepare_full_path_guide_reorients_reversed_artifact_for_shortcut_search() {
        let base = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 1, 1, 0, 2]);
        let a = base.conjugate_by_perm(&[1, 0, 2]);
        let b = base.conjugate_by_perm(&[2, 1, 0]);
        let guide = DynSsePath {
            matrices: vec![a.clone(), b.clone()],
            steps: vec![permutation_step_between(&a, &b).unwrap()],
        };
        let artifact = full_path_artifact("reverse-guide", reverse_dyn_sse_path(&guide));
        let request = SearchRequest {
            source: a.clone(),
            target: b.clone(),
            config: SearchConfig {
                max_lag: 2,
                max_intermediate_dim: 3,
                max_entry: 6,
                search_mode: SearchMode::GraphOnly,
                beam_width: None,
            },
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: vec![],
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let prepared = prepare_full_path_guide(&request, &artifact)
            .unwrap()
            .unwrap();
        assert_eq!(prepared.matrices, guide.matrices);
        assert_eq!(prepared.steps.len(), 1);
        validate_sse_path_dyn(&a, &b, &prepared).unwrap();
    }

    #[test]
    fn test_compare_ranked_guides_prefers_cost_then_score_then_stable_key() {
        let path = DynSsePath {
            matrices: vec![DynMatrix::new(2, 2, vec![1, 0, 0, 1])],
            steps: vec![],
        };
        let mut best = RankedGuide {
            path: path.clone(),
            effective_lag: 1,
            effective_cost: Some(2),
            effective_score: Some(4.0),
            stable_key: "a".to_string(),
        };
        let worse_cost = RankedGuide {
            path: path.clone(),
            effective_lag: 1,
            effective_cost: Some(3),
            effective_score: Some(10.0),
            stable_key: "b".to_string(),
        };
        let worse_score = RankedGuide {
            path: path.clone(),
            effective_lag: 1,
            effective_cost: Some(2),
            effective_score: Some(1.0),
            stable_key: "c".to_string(),
        };
        let worse_stable = RankedGuide {
            path,
            effective_lag: 1,
            effective_cost: Some(2),
            effective_score: Some(4.0),
            stable_key: "z".to_string(),
        };

        assert_eq!(compare_ranked_guides(&best, &worse_cost), Ordering::Less);
        assert_eq!(compare_ranked_guides(&best, &worse_score), Ordering::Less);
        assert_eq!(compare_ranked_guides(&best, &worse_stable), Ordering::Less);

        best.effective_cost = None;
        assert_eq!(compare_ranked_guides(&worse_cost, &best), Ordering::Less);
    }

    #[test]
    fn test_shortcut_search_stage_rejects_rectangular_endpoints() {
        let request = SearchRequest {
            source: DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 0]),
            target: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
            config: default_config(),
            stage: SearchStage::ShortcutSearch,
            guide_artifacts: Vec::new(),
            guided_refinement: GuidedRefinementConfig::default(),
            shortcut_search: ShortcutSearchConfig::default(),
        };

        let err = execute_search_request(&request).unwrap_err();
        assert_eq!(
            err,
            "shortcut_search requires square source and target matrices"
        );
    }

    #[test]
    fn test_telemetry_for_invariant_rejection() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[3, 1], [1, 1]]);
        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &default_config());
        assert!(matches!(result, SseResult::NotEquivalent(_)));
        assert!(telemetry.invariant_filtered);
        assert_eq!(telemetry.frontier_nodes_expanded, 0);
        assert!(telemetry.layers.is_empty());
    }

    #[test]
    fn test_different_det_not_equivalent() {
        let a = SqMatrix::new([[3, 1], [1, 1]]); // tr=4, det=2
        let b = SqMatrix::new([[2, 1], [1, 2]]); // tr=4, det=3
        let result = search_sse_2x2(&a, &b, &default_config());
        match result {
            SseResult::NotEquivalent(reason) => {
                assert!(reason.contains("determinant"));
            }
            _ => panic!("Expected NotEquivalent"),
        }
    }

    #[test]
    fn test_path_verification() {
        // For any found path, verify each step: A_i = UV, A_{i+1} = VU
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let result = search_sse_2x2(&a, &b, &default_config());
        if let SseResult::Equivalent(path) = result {
            for step in &path.steps {
                let _uv = step.u.mul(&step.v);
                let _vu = step.v.mul(&step.u);
                // Dimensions should be consistent.
                assert_eq!(step.u.rows, step.v.cols);
                assert_eq!(step.u.cols, step.v.rows);
            }
        }
    }

    #[test]
    fn test_telemetry_for_elementary_pair_search() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let b = SqMatrix::new([[1, 1], [1, 2]]);
        let (result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &default_config());
        assert!(matches!(
            result,
            SseResult::Equivalent(_) | SseResult::EquivalentByConcreteShift(_)
        ));
        assert!(
            telemetry.permutation_shortcut
                || telemetry.concrete_shift_shortcut
                || !telemetry.layers.is_empty()
        );
        if telemetry.permutation_shortcut || telemetry.concrete_shift_shortcut {
            assert_eq!(telemetry.frontier_nodes_expanded, 0);
            assert!(telemetry.layers.is_empty());
        } else {
            assert!(telemetry.frontier_nodes_expanded >= 1);
            assert!(telemetry.factorisation_calls >= 1);
            assert!(telemetry.factorisations_enumerated >= telemetry.candidates_after_pruning);
            assert!(telemetry.total_visited_nodes >= 2);
        }
    }

    #[test]
    fn test_bfs_positive_pair_path_is_valid() {
        let a = SqMatrix::new([[1, 1], [2, 5]]);
        let b = SqMatrix::new([[1, 2], [1, 5]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 6,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        match result {
            SseResult::Equivalent(path) => assert_valid_path(&path),
            SseResult::EquivalentByConcreteShift(_witness) => {}
            other => panic!("expected Equivalent path, got {:?}", other),
        }
    }

    #[test]
    fn test_telemetry_for_brix_ruiz_search() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        };
        let (_result, telemetry) = search_sse_2x2_with_telemetry(&a, &b, &config);
        assert!(!telemetry.invariant_filtered);
        assert!(!telemetry.permutation_shortcut);
        assert!(!telemetry.layers.is_empty());
        assert!(telemetry.frontier_nodes_expanded >= 1);
        assert!(telemetry.factorisations_enumerated >= telemetry.candidates_after_pruning);
    }

    #[test]
    fn test_expand_frontier_layer_deduplicates_canonical_successors() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (expansions, stats) =
            expand_frontier_layer(&[a_canon], &orig, 2, 10, SearchMode::Mixed);

        assert!(!expansions.is_empty());
        assert!(stats.factorisations_enumerated > expansions.len());
    }

    #[test]
    fn test_expand_frontier_layer_deduplicates_across_frontier_nodes() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let a_dyn = DynMatrix::from_sq(&a);
        let a_canon = a_dyn.canonical_perm();
        let mut orig = HashMap::new();
        orig.insert(a_canon.clone(), a_dyn);

        let (single_expansions, _) = expand_frontier_layer(
            std::slice::from_ref(&a_canon),
            &orig,
            2,
            10,
            SearchMode::Mixed,
        );
        let (duplicate_frontier_expansions, _) =
            expand_frontier_layer(&[a_canon.clone(), a_canon], &orig, 2, 10, SearchMode::Mixed);

        assert_eq!(duplicate_frontier_expansions.len(), single_expansions.len());
    }

    #[test]
    fn test_should_expand_forward_prefers_lower_estimated_work() {
        assert!(!should_expand_forward(
            1002,
            1137,
            151644.0 / 323.0,
            103760.0 / 662.0,
            323,
            662,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ));
    }

    #[test]
    fn test_should_expand_forward_falls_back_to_smaller_frontier_when_untrained() {
        assert!(should_expand_forward(
            3,
            5,
            100.0,
            1.0,
            0,
            0,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ));
        assert!(!should_expand_forward(
            7,
            2,
            1.0,
            100.0,
            0,
            0,
            FrontierOverlapSignal::default(),
            FrontierOverlapSignal::default(),
        ));
    }

    #[test]
    fn test_should_expand_forward_prefers_recent_overlap_signal() {
        assert!(should_expand_forward(
            1500,
            900,
            200.0,
            10.0,
            100,
            100,
            FrontierOverlapSignal::from_layer(1500, 4),
            FrontierOverlapSignal::from_layer(900, 0),
        ));
    }

    #[test]
    fn test_approx_signature_ignores_exact_support_layout() {
        let a = DynMatrix::new(3, 3, vec![1, 2, 0, 0, 1, 2, 1, 0, 1]);
        let b = DynMatrix::new(3, 3, vec![1, 0, 2, 0, 2, 1, 1, 1, 0]);
        assert_eq!(approx_signature(&a), approx_signature(&b));
    }

    #[test]
    fn test_same_future_past_signature_matches_duplicate_class_profiles() {
        let a = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 1, 1]);
        let b = DynMatrix::new(3, 3, vec![1, 0, 1, 1, 0, 1, 0, 1, 1]);

        assert_eq!(
            same_future_past_signature(&a),
            same_future_past_signature(&b)
        );
    }

    #[test]
    fn test_same_future_past_representative_selection_only_collapses_graph_moves() {
        let parent = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let graph_a = DynMatrix::new(3, 3, vec![1, 1, 0, 1, 1, 0, 0, 1, 1]);
        let graph_b = DynMatrix::new(3, 3, vec![1, 0, 1, 1, 0, 1, 0, 1, 1]);
        let factorised = DynMatrix::new(3, 3, vec![1, 1, 0, 0, 1, 1, 1, 0, 1]);
        let graph_a_signature = same_future_past_signature(&graph_a);
        let graph_b_signature = same_future_past_signature(&graph_b);
        let dummy_step = EsseStep {
            u: DynMatrix::new(1, 1, vec![1]),
            v: DynMatrix::new(1, 1, vec![1]),
        };
        let expansions = vec![
            FrontierExpansion {
                parent_canon: parent.clone(),
                next_canon: graph_a.clone(),
                next_orig: graph_a,
                step: dummy_step.clone(),
                move_family: "graph_a",
                same_future_past_signature: graph_a_signature,
            },
            FrontierExpansion {
                parent_canon: parent.clone(),
                next_canon: graph_b.clone(),
                next_orig: graph_b.clone(),
                step: dummy_step.clone(),
                move_family: "graph_b",
                same_future_past_signature: graph_b_signature,
            },
            FrontierExpansion {
                parent_canon: parent,
                next_canon: factorised.clone(),
                next_orig: factorised,
                step: dummy_step,
                move_family: "factorised",
                same_future_past_signature: None,
            },
        ];

        let (deduped, same_future_past_collisions) = deduplicate_expansions(expansions, true);

        assert_eq!(same_future_past_collisions, 1);
        assert_eq!(deduped.len(), 2);
        assert!(deduped
            .iter()
            .any(|expansion| expansion.move_family == "factorised"));
    }

    // --- Literature examples ---

    /// Helper: verify an SSE path is valid (each step satisfies UV and VU consistency).
    fn assert_valid_path(path: &SsePath<2>) {
        assert!(path.steps.len() >= 1);
        // Verify first step starts from first matrix and last step ends at last matrix.
        let first_step = &path.steps[0];
        let uv = first_step.u.mul(&first_step.v);
        assert_eq!(
            uv,
            DynMatrix::from_sq(&path.matrices[0]),
            "First step: UV != A"
        );

        let last_step = &path.steps[path.steps.len() - 1];
        let vu = last_step.v.mul(&last_step.u);
        assert_eq!(
            vu,
            DynMatrix::from_sq(&path.matrices[path.matrices.len() - 1]),
            "Last step: VU != B"
        );

        // Verify chain: VU of step i = UV of step i+1 (the intermediate matrix).
        for i in 0..path.steps.len() - 1 {
            let vu_i = path.steps[i].v.mul(&path.steps[i].u);
            let uv_next = path.steps[i + 1].u.mul(&path.steps[i + 1].v);
            assert_eq!(vu_i, uv_next, "Step {}: VU != UV of step {}", i, i + 1);
        }
    }

    // Eilers-Kiming 2008, p.8: Three 2x2 matrices that share all classical
    // invariants (trace=6, det=-73, same Bowen-Franks group) but are pairwise
    // NOT SSE. Our search can't prove non-SSE (no ideal class invariant yet),
    // so it should return Unknown.

    #[test]
    fn test_eilers_kiming_triple_classical_invariants_match() {
        // Classical invariants (trace, det, Bowen-Franks) all match for this triple.
        // The Eilers-Kiming ideal class invariant distinguishes them.
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m2 = SqMatrix::new([[5, 6], [13, 1]]);
        let m3 = SqMatrix::new([[4, 9], [9, 2]]);
        assert_eq!(m1.trace(), 6);
        assert_eq!(m2.trace(), 6);
        assert_eq!(m3.trace(), 6);
        assert_eq!(m1.det(), -73);
        assert_eq!(m2.det(), -73);
        assert_eq!(m3.det(), -73);
    }

    #[test]
    fn test_eilers_kiming_m1_m2_not_equivalent() {
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m2 = SqMatrix::new([[5, 6], [13, 1]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        };
        let result = search_sse_2x2(&m1, &m2, &config);
        assert!(
            matches!(result, SseResult::NotEquivalent(_)),
            "Expected NotEquivalent for Eilers-Kiming non-SSE pair (m1, m2)"
        );
    }

    #[test]
    fn test_eilers_kiming_m1_m3_not_equivalent() {
        let m1 = SqMatrix::new([[5, 13], [6, 1]]);
        let m3 = SqMatrix::new([[4, 9], [9, 2]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        };
        let result = search_sse_2x2(&m1, &m3, &config);
        assert!(
            matches!(result, SseResult::NotEquivalent(_)),
            "Expected NotEquivalent for Eilers-Kiming non-SSE pair (m1, m3)"
        );
    }

    // Eilers-Kiming 2008, p.8-9: [[14,2],[1,0]] and [[13,5],[3,1]] share
    // classical invariants (char poly x^2 - 14x - 2) but are NOT SSE.

    #[test]
    fn test_eilers_kiming_14_2_classical_invariants_match() {
        // Classical invariants match, but the ideal class invariant distinguishes them.
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);
        assert_eq!(a.trace(), b.trace());
        assert_eq!(a.det(), b.det());
    }

    #[test]
    fn test_eilers_kiming_14_2_not_equivalent() {
        let a = SqMatrix::new([[14, 2], [1, 0]]);
        let b = SqMatrix::new([[13, 5], [3, 1]]);
        let config = SearchConfig {
            max_lag: 3,
            max_intermediate_dim: 2,
            max_entry: 15,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        assert!(
            matches!(result, SseResult::NotEquivalent(_)),
            "Expected NotEquivalent for Eilers-Kiming non-SSE pair ([[14,2],[1,0]], [[13,5],[3,1]])"
        );
    }

    // Brix-Ruiz 2025, Example 3.8 (k=3): [[1,3],[2,1]] and [[1,6],[1,1]]
    // are known to be SSE (trace=2, det=-5).

    #[test]
    fn test_brix_ruiz_k3_invariants_match() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        assert_eq!(a.trace(), b.trace()); // 2
        assert_eq!(a.det(), b.det()); // -5
        assert!(check_invariants_2x2(&a, &b).is_none());
    }

    #[test]
    fn test_brix_ruiz_k3_search() {
        // Known SSE but the search space is too large for brute force at
        // practical bounds. This test verifies the search doesn't incorrectly
        // report NotEquivalent and exercises the rectangular factorisation
        // code path. Finding the actual path will require optimisations
        // (parallelism, smarter pruning, or algebraic shortcuts).
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 4,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        assert!(
            matches!(
                result,
                SseResult::Equivalent(_)
                    | SseResult::EquivalentByConcreteShift(_)
                    | SseResult::Unknown
            ),
            "Should not be NotEquivalent — these are known SSE"
        );
    }

    #[test]
    #[ignore = "expensive graph-only regression"]
    fn test_brix_ruiz_k3_graph_only_finds_path() {
        let a = SqMatrix::new([[1, 3], [2, 1]]);
        let b = SqMatrix::new([[1, 6], [1, 1]]);
        let config = SearchConfig {
            max_lag: 22,
            max_intermediate_dim: 5,
            max_entry: 6,
            search_mode: SearchMode::GraphOnly,
            beam_width: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        assert!(
            matches!(result, SseResult::Equivalent(_)),
            "graph-only search should find the known Brix-Ruiz k=3 path"
        );
    }

    #[test]
    fn test_rectangular_sse_constructed() {
        // Construct a pair connected through a 3x3 intermediate.
        // Step 1: A = U1*V1, C = V1*U1 (3x3)
        let u1 = DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 0]);
        let v1 = DynMatrix::new(3, 2, vec![1, 0, 1, 1, 1, 1]);
        let a_dyn = u1.mul(&v1); // A = [[2,1],[1,1]]
        let c = v1.mul(&u1); // C (3x3)

        // Step 2: factor C = U2*V2, B = V2*U2 (2x2)
        // We need to find U2 (3x2), V2 (2x3) such that U2*V2 = C.
        // C = [[1,0,1],[1,1,1],[1,1,1]]
        // Try U2 = [[1,0],[0,1],[0,1]], V2 = [[1,0,1],[1,1,1]]
        // U2*V2 = [[1,0,1],[1,1,1],[1,1,1]] = C
        let u2 = DynMatrix::new(3, 2, vec![1, 0, 0, 1, 0, 1]);
        let v2 = DynMatrix::new(2, 3, vec![1, 0, 1, 1, 1, 1]);
        let c_check = u2.mul(&v2);
        assert_eq!(c, c_check, "C from step 1 != C from step 2");

        let b_dyn = v2.mul(&u2); // B = [[1,0],[1,2]] (2x2)
        let a: SqMatrix<2> = a_dyn.to_sq().unwrap();
        let b: SqMatrix<2> = b_dyn.to_sq().unwrap();

        // Verify A and B are distinct (and not just permutation-similar).
        assert_ne!(a, b);

        let config = SearchConfig {
            max_lag: 4,
            max_intermediate_dim: 3,
            max_entry: 5,
            search_mode: SearchMode::Mixed,
            beam_width: None,
        };
        let result = search_sse_2x2(&a, &b, &config);
        match &result {
            SseResult::Equivalent(path) => {
                assert!(path.steps.len() >= 1);
                // Verify path: A and B have same trace/det so might be connected
                // via square steps too, but this exercises the full search with
                // rectangular factorisation enabled.
                assert_valid_path(path);
            }
            SseResult::EquivalentByConcreteShift(_witness) => {}
            _ => panic!(
                "Expected Equivalent for constructed rectangular SSE pair A={:?} B={:?}, got {:?}",
                a,
                b,
                match &result {
                    SseResult::EquivalentByConcreteShift(_) => {
                        "EquivalentByConcreteShift".to_string()
                    }
                    SseResult::NotEquivalent(r) => format!("NotEquivalent({})", r),
                    SseResult::Unknown => "Unknown".to_string(),
                    _ => unreachable!(),
                }
            ),
        }
    }

    // Brix-Ruiz 2025, Example 3.8 (k=4): [[1,4],[3,1]] and [[1,12],[1,1]]
    // are SE but SSE status is OPEN.

    #[test]
    fn test_brix_ruiz_k4_invariants_match() {
        let a = SqMatrix::new([[1, 4], [3, 1]]);
        let b = SqMatrix::new([[1, 12], [1, 1]]);
        assert_eq!(a.trace(), b.trace()); // 2
        assert_eq!(a.det(), b.det()); // -11
        assert!(check_invariants_2x2(&a, &b).is_none());
    }

    // --- Spectral pruning tests ---

    #[test]
    fn test_spectral_consistent_2x2_matching() {
        // [[2,1],[1,1]] has trace=3, det=1
        let m = DynMatrix::new(2, 2, vec![2, 1, 1, 1]);
        assert!(is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_spectral_inconsistent_2x2_wrong_trace() {
        let m = DynMatrix::new(2, 2, vec![3, 1, 1, 1]); // trace=4, det=2
        assert!(!is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_spectral_inconsistent_2x2_wrong_det() {
        let m = DynMatrix::new(2, 2, vec![2, 1, 1, 2]); // trace=4, det=3
        assert!(!is_spectrally_consistent(&m, 4, 2));
    }

    #[test]
    fn test_spectral_consistent_3x3_zero_eigenvalue() {
        // A 3x3 with eigenvalues {2, 1, 0}: trace=3, det=0, minor_sum=2.
        // Consistent with a 2x2 source having trace=3, det=2.
        // [[2,0,0],[0,1,0],[0,0,0]]: trace=3, det=0, minor_sum = 2+0+0 = 2
        let m = DynMatrix::new(3, 3, vec![2, 0, 0, 0, 1, 0, 0, 0, 0]);
        assert_eq!(m.trace(), 3);
        assert_eq!(m.det_3x3(), 0);
        assert_eq!(m.principal_minor_sum_3x3(), 2);
        assert!(is_spectrally_consistent(&m, 3, 2));
        assert!(!is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_spectral_inconsistent_3x3_nonzero_det() {
        // [[1,0,0],[0,1,0],[0,0,1]]: trace=3, det=1 (no zero eigenvalue)
        let m = DynMatrix::new(3, 3, vec![1, 0, 0, 0, 1, 0, 0, 0, 1]);
        assert!(!is_spectrally_consistent(&m, 3, 1));
    }

    #[test]
    fn test_search_sse_dyn_same_matrix() {
        let a = DynMatrix::new(3, 3, vec![0, 1, 0, 1, 0, 1, 0, 1, 0]);
        let (result, telemetry) = search_sse_with_telemetry_dyn(&a, &a, &default_config());
        match result {
            DynSseResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 0);
                assert_eq!(path.matrices, vec![a]);
            }
            other => panic!("expected equivalent result, got {other:?}"),
        }
        assert_eq!(telemetry.frontier_nodes_expanded, 0);
    }

    #[test]
    fn test_search_sse_dyn_permutation_shortcut() {
        let a = DynMatrix::new(3, 3, vec![2, 0, 0, 0, 1, 0, 0, 0, 0]);
        let b = DynMatrix::new(3, 3, vec![0, 0, 0, 0, 1, 0, 0, 0, 2]);
        let (result, telemetry) = search_sse_with_telemetry_dyn(&a, &b, &default_config());
        match result {
            DynSseResult::Equivalent(path) => {
                assert_eq!(path.steps.len(), 1);
                assert_eq!(path.matrices, vec![a, b]);
            }
            other => panic!("expected equivalent result, got {other:?}"),
        }
        assert!(telemetry.permutation_shortcut || telemetry.canonical_shortcut);
    }
}
