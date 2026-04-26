use crate::matrix::DynMatrix;
use crate::search_observer::{
    SearchEdgeRecord, SearchEvent, SearchFinishedRecord, SearchObserver, SearchRootRecord,
    SearchStartRecord,
};
use crate::types::{
    DynSseResult, FrontierMode, GuidedRefinementConfig, SearchConfig, SearchDirection,
    SearchRequest, SearchRunResult, SearchStage, SearchTelemetry, ShortcutSearchConfig, SseResult,
};

use super::stages::{search_guided_refinement_with_observer, search_shortcut_search_with_observer};
use super::{
    search_sse_2x2_with_telemetry_and_observer, search_sse_with_telemetry_dyn_and_observer,
    validate_endpoint_multi_meet_config,
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
    validate_request(request)?;
    match request.stage {
        SearchStage::EndpointSearch => Ok(execute_endpoint_search_request(request, observer)),
        SearchStage::GuidedRefinement => search_guided_refinement_with_observer(request, observer),
        SearchStage::ShortcutSearch => search_shortcut_search_with_observer(request, observer),
    }
}

fn validate_request(request: &SearchRequest) -> Result<(), String> {
    let Some(cap) = request.config.endpoint_multi_meet_cap else {
        return Ok(());
    };
    if cap == 0 {
        return Err("endpoint_multi_meet_cap must be at least 1 when requested".to_string());
    }
    if request.stage != SearchStage::EndpointSearch {
        return Err("endpoint_multi_meet_cap only supports --stage endpoint-search".to_string());
    }
    validate_endpoint_multi_meet_config(&request.config)
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

pub(super) fn emit_started_and_roots(
    observer: &mut Option<&mut dyn SearchObserver>,
    request: &SearchRequest,
    source_canonical: &DynMatrix,
    source_orig: &DynMatrix,
    target_canonical: &DynMatrix,
    target_orig: &DynMatrix,
) {
    emit_started(observer, request, source_canonical, target_canonical);
    emit_roots(
        observer,
        &[
            SearchRootRecord {
                direction: SearchDirection::Forward,
                canonical: source_canonical.clone(),
                orig: source_orig.clone(),
                depth: 0,
            },
            SearchRootRecord {
                direction: SearchDirection::Backward,
                canonical: target_canonical.clone(),
                orig: target_orig.clone(),
                depth: 0,
            },
        ],
    );
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
    if request.config.frontier_mode != FrontierMode::StratifiedBeamRefill {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct EventProbe {
        events: Vec<SearchEvent>,
    }

    impl SearchObserver for EventProbe {
        fn on_event(&mut self, event: &SearchEvent) {
            self.events.push(event.clone());
        }
    }

    #[test]
    fn emit_started_and_roots_emits_endpoint_start_bundle() {
        let source = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let target = DynMatrix::new(2, 2, vec![0, 1, 1, 0]);
        let request = endpoint_search_request(&source, &target, &SearchConfig::default());
        let source_canonical = source.canonical_perm();
        let target_canonical = target.canonical_perm();
        let mut probe = EventProbe::default();
        let mut observer: Option<&mut dyn SearchObserver> = Some(&mut probe);

        emit_started_and_roots(
            &mut observer,
            &request,
            &source_canonical,
            &source,
            &target_canonical,
            &target,
        );

        assert_eq!(probe.events.len(), 2);
        let SearchEvent::Started(started) = &probe.events[0] else {
            panic!("expected first event to be Started");
        };
        assert_eq!(started.request.source, request.source);
        assert_eq!(started.request.target, request.target);
        assert_eq!(started.request.config, request.config);
        assert_eq!(started.request.stage, request.stage);
        assert_eq!(started.source_canonical, source_canonical);
        assert_eq!(started.target_canonical, target_canonical);

        let SearchEvent::Roots(roots) = &probe.events[1] else {
            panic!("expected second event to be Roots");
        };
        assert_eq!(roots.len(), 2);
        assert_eq!(roots[0].direction, SearchDirection::Forward);
        assert_eq!(roots[0].canonical, source_canonical);
        assert_eq!(roots[0].orig, source);
        assert_eq!(roots[1].direction, SearchDirection::Backward);
        assert_eq!(roots[1].canonical, target_canonical);
        assert_eq!(roots[1].orig, target);
    }
}
