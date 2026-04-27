use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::matrix::DynMatrix;
use crate::types::DynSsePath;

/// Direction of a BFS layer expansion in bidirectional search.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchDirection {
    Forward,
    Backward,
}

/// Telemetry captured for one frontier expansion layer.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchMoveFamilyTelemetry {
    pub candidates_generated: usize,
    pub candidates_after_pruning: usize,
    pub discovered_nodes: usize,
    pub exact_meets: usize,
    pub approximate_other_side_hits: usize,
}

/// Wall-clock timing breakdown for one frontier expansion layer.
#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct SearchLayerTimingTelemetry {
    pub total_nanos: u64,
    pub expand_compute_nanos: u64,
    pub expand_accumulate_nanos: u64,
    pub dedup_nanos: u64,
    pub merge_nanos: u64,
    pub finalize_nanos: u64,
}

/// Telemetry captured for one frontier expansion layer.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchLayerTelemetry {
    pub layer_index: usize,
    pub direction: Option<SearchDirection>,
    pub frontier_nodes: usize,
    pub factorisation_calls: usize,
    pub factorisations_enumerated: usize,
    pub candidates_generated: usize,
    pub pruned_by_size: usize,
    pub pruned_by_spectrum: usize,
    pub candidates_after_pruning: usize,
    pub collisions_with_seen: usize,
    pub collisions_with_other_frontier: usize,
    pub approximate_other_side_hits: usize,
    pub same_future_past_collisions: usize,
    pub discovered_nodes: usize,
    pub dead_end_nodes: usize,
    pub enqueued_nodes: usize,
    pub next_frontier_nodes: usize,
    pub total_visited_nodes: usize,
    pub timing: SearchLayerTimingTelemetry,
    pub move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct StratifiedBeamRefillTelemetry {
    pub active_admissions: usize,
    pub deferred_admissions: usize,
    pub drops_by_bucket_cap: usize,
    pub drops_by_global_cap: usize,
    pub refill_count: usize,
    pub refill_exhausted: usize,
    pub refill_below_threshold: usize,
    pub refill_admissions: usize,
    pub final_active_frontier_nodes: usize,
    pub final_deferred_frontier_nodes: usize,
}

impl StratifiedBeamRefillTelemetry {
    pub fn is_empty(&self) -> bool {
        self.active_admissions == 0
            && self.deferred_admissions == 0
            && self.drops_by_bucket_cap == 0
            && self.drops_by_global_cap == 0
            && self.refill_count == 0
            && self.refill_exhausted == 0
            && self.refill_below_threshold == 0
            && self.refill_admissions == 0
            && self.final_active_frontier_nodes == 0
            && self.final_deferred_frontier_nodes == 0
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SameFuturePastDiversityLayerSample {
    pub layer_index: usize,
    pub direction: SearchDirection,
    pub frontier_nodes: usize,
    pub unique_buckets: usize,
    pub saturated_buckets: usize,
    pub max_bucket_size: usize,
    pub cross_frontier_overlap_buckets: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SameFuturePastDiversityTelemetry {
    pub active_admissions: usize,
    pub rejected_admissions: usize,
    pub unique_bucket_admissions: usize,
    pub duplicate_bucket_admissions: usize,
    pub replacements_from_saturated_bucket: usize,
    pub final_frontier_nodes: usize,
    pub final_unique_buckets: usize,
    pub final_saturated_buckets: usize,
    pub final_max_bucket_size: usize,
    pub final_cross_frontier_overlap_buckets: usize,
    pub max_frontier_nodes: usize,
    pub max_unique_buckets: usize,
    pub max_saturated_buckets: usize,
    pub max_bucket_size: usize,
    pub max_cross_frontier_overlap_buckets: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub layer_samples: Vec<SameFuturePastDiversityLayerSample>,
}

impl SameFuturePastDiversityTelemetry {
    pub fn is_empty(&self) -> bool {
        self.active_admissions == 0
            && self.rejected_admissions == 0
            && self.unique_bucket_admissions == 0
            && self.duplicate_bucket_admissions == 0
            && self.replacements_from_saturated_bucket == 0
            && self.final_frontier_nodes == 0
            && self.final_unique_buckets == 0
            && self.final_saturated_buckets == 0
            && self.final_max_bucket_size == 0
            && self.final_cross_frontier_overlap_buckets == 0
            && self.max_frontier_nodes == 0
            && self.max_unique_buckets == 0
            && self.max_saturated_buckets == 0
            && self.max_bucket_size == 0
            && self.max_cross_frontier_overlap_buckets == 0
            && self.layer_samples.is_empty()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointExactMeetWitness {
    pub path_lag: usize,
    pub meet_direction: Option<SearchDirection>,
    pub meeting_canonical: DynMatrix,
    pub path: DynSsePath,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EndpointExactMeetSurface {
    pub requested_cap: usize,
    pub retained: Vec<EndpointExactMeetWitness>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutSearchStopReason {
    GuidePoolExhausted,
    NoImprovementRound,
    MaxRoundsReached,
    MaxSegmentAttemptsReached,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShortcutSearchRoundTelemetry {
    pub round_index: usize,
    pub working_set_guides: usize,
    pub starting_best_lag: Option<usize>,
    pub ending_best_lag: Option<usize>,
    pub segment_attempts: usize,
    pub segment_improvements: usize,
    pub promoted_guides: usize,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ShortcutSearchTelemetry {
    pub guide_artifacts_loaded: usize,
    pub guide_artifacts_accepted: usize,
    pub unique_guides: usize,
    pub initial_working_set_guides: usize,
    pub segment_attempts: usize,
    #[serde(default)]
    pub segment_cache_hits: usize,
    #[serde(default)]
    pub segment_cache_misses: usize,
    pub segment_improvements: usize,
    pub promoted_guides: usize,
    pub emitted_guide_artifacts: usize,
    pub rounds_completed: usize,
    pub best_lag_start: Option<usize>,
    pub best_lag_end: Option<usize>,
    pub stop_reason: Option<ShortcutSearchStopReason>,
    pub rounds: Vec<ShortcutSearchRoundTelemetry>,
}

/// Aggregate telemetry for a full `search_sse_2x2` invocation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchTelemetry {
    #[serde(default)]
    pub invalid_config: Option<String>,
    pub invariant_filtered: bool,
    pub permutation_shortcut: bool,
    pub canonical_shortcut: bool,
    pub concrete_shift_shortcut: bool,
    pub frontier_nodes_expanded: usize,
    pub factorisation_calls: usize,
    pub factorisations_enumerated: usize,
    pub candidates_generated: usize,
    pub pruned_by_size: usize,
    pub pruned_by_spectrum: usize,
    pub candidates_after_pruning: usize,
    pub collisions_with_seen: usize,
    pub collisions_with_other_frontier: usize,
    pub approximate_other_side_hits: usize,
    pub same_future_past_collisions: usize,
    pub discovered_nodes: usize,
    pub dead_end_nodes: usize,
    pub enqueued_nodes: usize,
    pub max_frontier_size: usize,
    pub total_visited_nodes: usize,
    pub guide_artifacts_considered: usize,
    pub guide_artifacts_accepted: usize,
    pub guided_segments_considered: usize,
    pub guided_segments_improved: usize,
    pub guided_refinement_rounds: usize,
    pub shortcut_search: ShortcutSearchTelemetry,
    pub move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
    pub layers: Vec<SearchLayerTelemetry>,
    #[serde(
        default,
        skip_serializing_if = "StratifiedBeamRefillTelemetry::is_empty"
    )]
    pub stratified_beam_refill: StratifiedBeamRefillTelemetry,
    #[serde(
        default,
        skip_serializing_if = "SameFuturePastDiversityTelemetry::is_empty"
    )]
    pub same_future_past_diversity: SameFuturePastDiversityTelemetry,
    #[serde(skip)]
    pub endpoint_exact_meets: Option<EndpointExactMeetSurface>,
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::matrix::DynMatrix;
    use crate::types::{
        DynSsePath, EndpointExactMeetSurface, EndpointExactMeetWitness, SearchDirection,
        SearchLayerTelemetry, SearchTelemetry,
    };

    #[test]
    fn search_telemetry_reexports_preserve_json_shape() {
        let matrix = DynMatrix::new(2, 2, vec![1, 0, 0, 1]);
        let telemetry = SearchTelemetry {
            frontier_nodes_expanded: 7,
            layers: vec![SearchLayerTelemetry {
                layer_index: 2,
                direction: Some(SearchDirection::Backward),
                frontier_nodes: 3,
                ..SearchLayerTelemetry::default()
            }],
            endpoint_exact_meets: Some(EndpointExactMeetSurface {
                requested_cap: 1,
                retained: vec![EndpointExactMeetWitness {
                    path_lag: 0,
                    meet_direction: Some(SearchDirection::Forward),
                    meeting_canonical: matrix.clone(),
                    path: DynSsePath {
                        matrices: vec![matrix],
                        steps: Vec::new(),
                    },
                }],
            }),
            ..SearchTelemetry::default()
        };

        let encoded = serde_json::to_value(&telemetry).unwrap();
        let object = encoded.as_object().unwrap();

        assert_eq!(object["frontier_nodes_expanded"], json!(7));
        assert_eq!(object["layers"][0]["direction"], json!("backward"));
        assert!(object.get("endpoint_exact_meets").is_none());
        assert!(object.get("stratified_beam_refill").is_none());
        assert!(object.get("same_future_past_diversity").is_none());
    }
}
