use crate::matrix::DynMatrix;
use crate::search_observer::{
    SearchEdgeRecord, SearchEvent, SearchFinishedRecord, SearchObserver, SearchRootRecord,
    SearchStartRecord,
};
use crate::types::{
    DynSseResult, GuidedRefinementConfig, SearchConfig, SearchRequest, SearchRunResult,
    SearchStage, SearchTelemetry, ShortcutSearchConfig, SseResult,
};

use super::stages::{search_guided_refinement_with_observer, search_shortcut_search_with_observer};
use super::{
    search_sse_2x2_with_telemetry_and_observer, search_sse_with_telemetry_dyn_and_observer,
};

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
        SearchStage::EndpointSearch => Ok(execute_endpoint_search_request(request, observer)),
        SearchStage::GuidedRefinement => search_guided_refinement_with_observer(request, observer),
        SearchStage::ShortcutSearch => search_shortcut_search_with_observer(request, observer),
    }
}

pub(super) fn endpoint_search_request(
    source: &DynMatrix,
    target: &DynMatrix,
    config: &SearchConfig,
) -> SearchRequest {
    SearchRequest {
        source: source.clone(),
        target: target.clone(),
        config: config.clone(),
        stage: SearchStage::EndpointSearch,
        guide_artifacts: Vec::new(),
        guided_refinement: GuidedRefinementConfig::default(),
        shortcut_search: ShortcutSearchConfig::default(),
    }
}

pub(super) fn emit_started(
    observer: &mut Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    source_canonical: &DynMatrix,
    target_canonical: &DynMatrix,
) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&SearchEvent::Started(SearchStartRecord {
            request: request.clone(),
            source_canonical: source_canonical.clone(),
            target_canonical: target_canonical.clone(),
        }));
    }
}

pub(super) fn emit_roots(
    observer: &mut Option<&mut dyn SearchObserver>,
    roots: &[SearchRootRecord],
) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&SearchEvent::Roots(roots.to_vec()));
    }
}

pub(super) fn emit_layer(
    observer: &mut Option<&mut dyn SearchObserver>,
    records: &[SearchEdgeRecord],
) {
    if let Some(observer) = observer.as_deref_mut() {
        observer.on_event(&SearchEvent::Layer(records.to_vec()));
    }
}

pub(super) fn emit_finished(
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

pub(super) fn finish_search_2x2(
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    result: SseResult<2>,
    telemetry: SearchTelemetry,
) -> (SseResult<2>, SearchTelemetry) {
    emit_finished(&mut observer, request, result.clone().into(), &telemetry);
    (result, telemetry)
}

pub(super) fn finish_search_dyn(
    mut observer: Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    result: DynSseResult,
    telemetry: SearchTelemetry,
) -> (DynSseResult, SearchTelemetry) {
    emit_finished(&mut observer, request, result.clone().into(), &telemetry);
    (result, telemetry)
}

fn execute_endpoint_search_request(
    request: &SearchRequest,
    observer: Option<&mut dyn SearchObserver>,
) -> (SearchRunResult, SearchTelemetry) {
    let a_sq = request.source.to_sq::<2>();
    let b_sq = request.target.to_sq::<2>();
    if let (Some(a), Some(b)) = (a_sq.as_ref(), b_sq.as_ref()) {
        let (result, telemetry) =
            search_sse_2x2_with_telemetry_and_observer(a, b, &request.config, observer);
        (result.into(), telemetry)
    } else {
        let (result, telemetry) = search_sse_with_telemetry_dyn_and_observer(
            &request.source,
            &request.target,
            &request.config,
            observer,
        );
        (result.into(), telemetry)
    }
}
