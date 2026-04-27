use super::exact_meets::{
    best_retained_exact_meet_path, maybe_store_endpoint_exact_meets, ExactMeetRetention,
};
use super::test_fixtures::default_config;
use super::{
    execute_search_request, search_sse_2x2_with_telemetry,
    search_sse_2x2_with_telemetry_and_observer, search_sse_with_telemetry_dyn,
    validate_sse_path_dyn,
};
use crate::matrix::{DynMatrix, SqMatrix};
use crate::types::{
    DynSsePath, DynSseResult, FrontierMode, GuidedRefinementConfig, SearchConfig, SearchDirection,
    SearchRequest, SearchStage, SearchTelemetry, ShortcutSearchConfig, SseResult,
};

fn small_2x2_scan_space(max_entry: u32) -> Vec<SqMatrix<2>> {
    let mut matrices = Vec::new();
    for a00 in 0..=max_entry {
        for a01 in 0..=max_entry {
            for a10 in 0..=max_entry {
                for a11 in 0..=max_entry {
                    matrices.push(SqMatrix::new([[a00, a01], [a10, a11]]));
                }
            }
        }
    }
    matrices
}

#[test]
fn test_endpoint_multi_meet_surface_retains_multiple_exact_meets_when_available() {
    let mut config = default_config();
    config.max_lag = 4;
    config.max_entry = 6;
    config.endpoint_multi_meet_cap = Some(4);

    let matrices = small_2x2_scan_space(2);
    let mut found_case = None;

    'outer: for source in &matrices {
        for target in &matrices {
            if source == target {
                continue;
            }
            let (result, telemetry) =
                search_sse_2x2_with_telemetry_and_observer(source, target, &config, None);
            let Some(surface) = telemetry.endpoint_exact_meets else {
                continue;
            };
            if surface.retained.len() <= 1 {
                continue;
            }
            found_case = Some((source.clone(), target.clone(), result, surface));
            break 'outer;
        }
    }

    let Some((source, target, result, surface)) = found_case else {
        panic!("expected bounded 2x2 scan to find a case with multiple retained exact meets");
    };
    eprintln!(
        "bounded endpoint multi-meet case: {:?} -> {:?} ({} retained)",
        source,
        target,
        surface.retained.len()
    );

    assert_eq!(surface.requested_cap, 4);
    assert!(surface.retained.len() > 1);
    assert!(surface
        .retained
        .windows(2)
        .all(|window| window[0].path_lag <= window[1].path_lag));
    assert!(
        surface
            .retained
            .iter()
            .all(|witness| witness.meet_direction.is_some()),
        "retained endpoint exact meets should record the producing frontier direction",
    );
    assert_eq!(
        surface
            .retained
            .iter()
            .map(|witness| witness.meeting_canonical.clone())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        surface.retained.len(),
        "retained endpoint exact meets should be canonical-unique",
    );

    let SseResult::Equivalent(primary_path) = result else {
        panic!("expected equivalent result for retained multi-meet case");
    };
    assert_eq!(
        primary_path.matrices.first(),
        Some(&source),
        "primary witness should start at the requested source"
    );
    assert_eq!(
        primary_path.matrices.last(),
        Some(&target),
        "primary witness should end at the requested target"
    );

    for witness in &surface.retained {
        assert!(
            witness.path.steps.len() >= witness.path_lag,
            "reconstructed witness may add permutation bridges but should not be shorter than the meet lag"
        );
        assert_eq!(
            witness.path.matrices.first(),
            Some(&DynMatrix::from_sq(&source)),
            "retained witness should start at the requested source"
        );
        assert_eq!(
            witness.path.matrices.last(),
            Some(&DynMatrix::from_sq(&target)),
            "retained witness should end at the requested target"
        );
        validate_sse_path_dyn(
            &DynMatrix::from_sq(&source),
            &DynMatrix::from_sq(&target),
            &witness.path,
        )
        .expect("retained witness path should validate");
    }
}

#[test]
fn test_direct_2x2_search_surfaces_invalid_endpoint_multi_meet_config_in_telemetry() {
    let source = SqMatrix::new([[0, 0], [0, 1]]);
    let target = SqMatrix::new([[0, 1], [0, 1]]);
    let config = SearchConfig {
        frontier_mode: FrontierMode::Beam,
        beam_width: Some(2),
        endpoint_multi_meet_cap: Some(2),
        ..default_config()
    };

    let (result, telemetry) = search_sse_2x2_with_telemetry(&source, &target, &config);

    assert!(matches!(result, SseResult::Unknown));
    assert_eq!(
        telemetry.invalid_config.as_deref(),
        Some("endpoint_multi_meet_cap currently only supports --frontier-mode bfs")
    );
    assert!(telemetry.endpoint_exact_meets.is_none());
    assert!(telemetry.layers.is_empty());
}

#[test]
fn test_direct_2x2_stratified_refill_rejects_non_2x2_intermediate_config() {
    let source = SqMatrix::new([[0, 0], [0, 1]]);
    let target = SqMatrix::new([[0, 1], [0, 1]]);
    let config = SearchConfig {
        frontier_mode: FrontierMode::StratifiedBeamRefill,
        beam_width: Some(2),
        max_intermediate_dim: 3,
        ..default_config()
    };

    let (result, telemetry) = search_sse_2x2_with_telemetry(&source, &target, &config);

    assert!(matches!(result, SseResult::Unknown));
    assert_eq!(
        telemetry.invalid_config.as_deref(),
        Some(
            "stratified_beam_refill with max_intermediate_dim > 2 requires the dynamic search API"
        )
    );
    assert!(telemetry.layers.is_empty());
}

#[test]
fn test_direct_dynamic_search_surfaces_invalid_endpoint_multi_meet_config_in_telemetry() {
    let source = DynMatrix::new(2, 2, vec![0, 0, 0, 1]);
    let target = DynMatrix::new(2, 2, vec![0, 1, 0, 1]);
    let config = SearchConfig {
        frontier_mode: FrontierMode::Beam,
        beam_width: Some(2),
        endpoint_multi_meet_cap: Some(2),
        ..default_config()
    };

    let (result, telemetry) = search_sse_with_telemetry_dyn(&source, &target, &config);

    assert!(matches!(result, DynSseResult::Unknown));
    assert_eq!(
        telemetry.invalid_config.as_deref(),
        Some("endpoint_multi_meet_cap currently only supports --frontier-mode bfs")
    );
    assert!(telemetry.endpoint_exact_meets.is_none());
    assert!(telemetry.layers.is_empty());
}

#[test]
fn test_request_dispatch_rejects_invalid_endpoint_multi_meet_config() {
    let request = SearchRequest {
        source: DynMatrix::new(2, 2, vec![0, 0, 0, 1]),
        target: DynMatrix::new(2, 2, vec![0, 1, 0, 1]),
        config: SearchConfig {
            frontier_mode: FrontierMode::Beam,
            beam_width: Some(2),
            endpoint_multi_meet_cap: Some(2),
            ..default_config()
        },
        stage: SearchStage::EndpointSearch,
        guide_artifacts: Vec::new(),
        guided_refinement: GuidedRefinementConfig::default(),
        shortcut_search: ShortcutSearchConfig::default(),
    };

    let err = execute_search_request(&request).unwrap_err();
    assert!(err.contains("--frontier-mode bfs"));
}

#[test]
fn test_timed_out_dynamic_multi_meet_keeps_best_path_but_omits_surface() {
    let meet_a = DynMatrix::new(2, 2, vec![0, 0, 0, 1]);
    let meet_b = DynMatrix::new(2, 2, vec![0, 0, 1, 1]);
    let config = SearchConfig {
        endpoint_multi_meet_cap: Some(4),
        ..default_config()
    };
    let retention =
        ExactMeetRetention::from_config(&config).expect("valid config should retain meets");
    let retention = {
        let mut retention = retention;
        retention.retain(&meet_b, 2, SearchDirection::Backward);
        retention.retain(&meet_a, 1, SearchDirection::Forward);
        retention
    };

    let build_path = |canonical: &DynMatrix| DynSsePath {
        matrices: vec![canonical.clone()],
        steps: vec![],
    };

    let best_path = best_retained_exact_meet_path(&retention, build_path)
        .expect("retained exact meet should still provide a best path under timeout");
    assert_eq!(best_path.matrices, vec![meet_a.clone()]);

    let mut timed_out_telemetry = SearchTelemetry::default();
    maybe_store_endpoint_exact_meets(
        &mut timed_out_telemetry,
        &retention,
        true,
        build_path,
        Clone::clone,
    );
    assert!(
        timed_out_telemetry.endpoint_exact_meets.is_none(),
        "partial timed-out layers should not publish the ranked multi-meet surface"
    );

    let mut complete_telemetry = SearchTelemetry::default();
    maybe_store_endpoint_exact_meets(
        &mut complete_telemetry,
        &retention,
        false,
        build_path,
        Clone::clone,
    );
    assert_eq!(
        complete_telemetry
            .endpoint_exact_meets
            .as_ref()
            .map(|surface| surface.retained.len()),
        Some(2)
    );
    assert_eq!(
        complete_telemetry
            .endpoint_exact_meets
            .as_ref()
            .map(|surface| {
                surface
                    .retained
                    .iter()
                    .map(|witness| witness.meet_direction)
                    .collect::<Vec<_>>()
            }),
        Some(vec![
            Some(SearchDirection::Forward),
            Some(SearchDirection::Backward)
        ])
    );
}
