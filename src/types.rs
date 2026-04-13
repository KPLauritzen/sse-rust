use crate::aligned::ConcreteShiftWitness2x2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::matrix::{DynMatrix, SqMatrix};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    Mixed,
    #[serde(alias = "graph-only")]
    GraphOnly,
}

impl Default for SearchMode {
    fn default() -> Self {
        Self::Mixed
    }
}

/// High-level solver stage terminology. This is intentionally separate from
/// [`SearchMode`], which only selects the low-level search substrate.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchStage {
    EndpointSearch,
    GuidedRefinement,
    ShortcutSearch,
}

impl Default for SearchStage {
    fn default() -> Self {
        Self::EndpointSearch
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GuideArtifactValidation {
    Unchecked,
    WitnessValidated,
}

impl Default for GuideArtifactValidation {
    fn default() -> Self {
        Self::Unchecked
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuideArtifactEndpoints {
    pub source: DynMatrix,
    pub target: DynMatrix,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuideArtifactProvenance {
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub source_ref: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuideArtifactCompatibility {
    #[serde(default)]
    pub supported_stages: Vec<SearchStage>,
    #[serde(default)]
    pub max_endpoint_dim: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct GuideArtifactQuality {
    #[serde(default)]
    pub lag: Option<usize>,
    #[serde(default)]
    pub cost: Option<usize>,
    #[serde(default)]
    pub score: Option<f64>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum GuideArtifactPayload {
    FullPath { path: DynSsePath },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GuideArtifact {
    #[serde(default)]
    pub artifact_id: Option<String>,
    pub endpoints: GuideArtifactEndpoints,
    #[serde(flatten)]
    pub payload: GuideArtifactPayload,
    #[serde(default)]
    pub provenance: GuideArtifactProvenance,
    #[serde(default)]
    pub validation: GuideArtifactValidation,
    #[serde(default)]
    pub compatibility: GuideArtifactCompatibility,
    #[serde(default)]
    pub quality: GuideArtifactQuality,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct GuidedRefinementConfig {
    pub max_shortcut_lag: usize,
    pub min_gap: usize,
    pub max_gap: Option<usize>,
    pub rounds: usize,
}

impl Default for GuidedRefinementConfig {
    fn default() -> Self {
        Self {
            max_shortcut_lag: 3,
            min_gap: 2,
            max_gap: None,
            rounds: 1,
        }
    }
}

/// Configuration for the SSE search.
#[derive(Clone, Debug)]
pub struct SearchConfig {
    /// Maximum number of elementary SSE steps to search.
    pub max_lag: usize,
    /// Maximum intermediate dimension for factorisations (m in n×m × m×n).
    /// Current search supports 2x2 square steps and 2x2 <-> 3x3 rectangular steps.
    pub max_intermediate_dim: usize,
    /// Maximum entry value in intermediate matrices U, V.
    pub max_entry: u32,
    /// Search move mode.
    pub search_mode: SearchMode,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_lag: 4,
            max_intermediate_dim: 2,
            max_entry: 25,
            search_mode: SearchMode::Mixed,
        }
    }
}

/// Generic request boundary for square-endpoint search orchestration.
#[derive(Clone, Debug)]
pub struct SearchRequest {
    pub source: DynMatrix,
    pub target: DynMatrix,
    pub config: SearchConfig,
    pub stage: SearchStage,
    pub guide_artifacts: Vec<GuideArtifact>,
    pub guided_refinement: GuidedRefinementConfig,
}

/// One elementary SSE step: A = UV, B = VU.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct EsseStep {
    pub u: DynMatrix,
    pub v: DynMatrix,
}

/// A chain of elementary SSE steps connecting A to B.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SsePath<const N: usize> {
    /// The sequence of matrices: A = matrices[0], B = matrices[last].
    pub matrices: Vec<SqMatrix<N>>,
    /// The elementary steps: matrices[i] = steps[i].u * steps[i].v,
    /// matrices[i+1] = steps[i].v * steps[i].u.
    pub steps: Vec<EsseStep>,
}

/// A chain of elementary SSE steps connecting arbitrary square endpoints.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct DynSsePath {
    /// The sequence of matrices: A = matrices[0], B = matrices[last].
    pub matrices: Vec<DynMatrix>,
    /// The elementary steps: matrices[i] = steps[i].u * steps[i].v,
    /// matrices[i+1] = steps[i].v * steps[i].u.
    pub steps: Vec<EsseStep>,
}

impl From<SsePath<2>> for DynSsePath {
    fn from(path: SsePath<2>) -> Self {
        let SsePath { matrices, steps } = path;
        if steps.is_empty() {
            return Self {
                matrices: matrices
                    .into_iter()
                    .map(|matrix| DynMatrix::from_sq(&matrix))
                    .collect(),
                steps,
            };
        }

        let start = matrices
            .first()
            .expect("non-empty-step SsePath should contain a start matrix");
        let mut dyn_matrices = Vec::with_capacity(steps.len() + 1);
        dyn_matrices.push(DynMatrix::from_sq(start));

        for step in &steps {
            let current = step.u.mul(&step.v);
            debug_assert_eq!(
                current,
                *dyn_matrices
                    .last()
                    .expect("reconstructed path should have a current matrix"),
                "SsePath<2> step chain should start from the previously reconstructed matrix"
            );
            dyn_matrices.push(step.v.mul(&step.u));
        }

        if let Some(end) = matrices.last() {
            debug_assert_eq!(
                dyn_matrices.last(),
                Some(&DynMatrix::from_sq(end)),
                "SsePath<2> stored endpoint should match the reconstructed endpoint"
            );
        }

        Self {
            matrices: dyn_matrices,
            steps,
        }
    }
}

/// Result of an SSE search.
#[derive(Clone, Debug)]
pub enum SseResult<const N: usize> {
    /// Found a path proving SSE.
    Equivalent(SsePath<N>),
    /// Found a direct aligned/balanced/compatible concrete-shift witness.
    EquivalentByConcreteShift(ConcreteShiftWitness2x2),
    /// Proved not SSE by an invariant mismatch.
    NotEquivalent(String),
    /// Search exhausted without finding a path or proving non-equivalence.
    Unknown,
}

/// Result of an SSE search between arbitrary square endpoints.
#[derive(Clone, Debug)]
pub enum DynSseResult {
    /// Found a path proving SSE.
    Equivalent(DynSsePath),
    /// Proved not SSE by an invariant mismatch.
    NotEquivalent(String),
    /// Search exhausted without finding a path or proving non-equivalence.
    Unknown,
}

/// Generic result boundary shared by request/result/event/persistence layers.
#[derive(Clone, Debug)]
pub enum SearchRunResult {
    Equivalent(DynSsePath),
    EquivalentByConcreteShift(ConcreteShiftWitness2x2),
    NotEquivalent(String),
    Unknown,
}

impl From<SseResult<2>> for SearchRunResult {
    fn from(result: SseResult<2>) -> Self {
        match result {
            SseResult::Equivalent(path) => Self::Equivalent(path.into()),
            SseResult::EquivalentByConcreteShift(witness) => {
                Self::EquivalentByConcreteShift(witness)
            }
            SseResult::NotEquivalent(reason) => Self::NotEquivalent(reason),
            SseResult::Unknown => Self::Unknown,
        }
    }
}

impl From<DynSseResult> for SearchRunResult {
    fn from(result: DynSseResult) -> Self {
        match result {
            DynSseResult::Equivalent(path) => Self::Equivalent(path),
            DynSseResult::NotEquivalent(reason) => Self::NotEquivalent(reason),
            DynSseResult::Unknown => Self::Unknown,
        }
    }
}

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
    pub move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
}

/// Aggregate telemetry for a full `search_sse_2x2` invocation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchTelemetry {
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
    pub move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
    pub layers: Vec<SearchLayerTelemetry>,
}

#[cfg(test)]
mod tests {
    use super::{
        DynMatrix, DynSsePath, EsseStep, GuideArtifact, GuideArtifactCompatibility,
        GuideArtifactEndpoints, GuideArtifactPayload, GuideArtifactProvenance,
        GuideArtifactQuality, GuideArtifactValidation, GuidedRefinementConfig, SearchMode,
        SearchStage, SsePath,
    };
    use crate::matrix::SqMatrix;

    #[test]
    fn test_search_mode_deserializes_snake_and_kebab_case_graph_only() {
        let snake: SearchMode = serde_json::from_str("\"graph_only\"").unwrap();
        let kebab: SearchMode = serde_json::from_str("\"graph-only\"").unwrap();

        assert_eq!(snake, SearchMode::GraphOnly);
        assert_eq!(kebab, SearchMode::GraphOnly);
    }

    #[test]
    fn test_guide_artifact_round_trips_as_full_path() {
        let artifact = GuideArtifact {
            artifact_id: Some("artifact-1".to_string()),
            endpoints: GuideArtifactEndpoints {
                source: DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
                target: DynMatrix::new(2, 2, vec![0, 1, 1, 0]),
            },
            payload: GuideArtifactPayload::FullPath {
                path: DynSsePath {
                    matrices: vec![
                        DynMatrix::new(2, 2, vec![1, 0, 0, 1]),
                        DynMatrix::new(2, 2, vec![0, 1, 1, 0]),
                    ],
                    steps: vec![EsseStep {
                        u: DynMatrix::new(2, 2, vec![0, 1, 1, 0]),
                        v: DynMatrix::new(2, 2, vec![0, 1, 1, 0]),
                    }],
                },
            },
            provenance: GuideArtifactProvenance {
                source_kind: Some("fixture".to_string()),
                label: Some("swap".to_string()),
                source_ref: Some("unit-test".to_string()),
            },
            validation: GuideArtifactValidation::WitnessValidated,
            compatibility: GuideArtifactCompatibility {
                supported_stages: vec![SearchStage::GuidedRefinement],
                max_endpoint_dim: Some(4),
            },
            quality: GuideArtifactQuality {
                lag: Some(1),
                cost: Some(1),
                score: Some(1.0),
            },
        };

        let json = serde_json::to_string(&artifact).unwrap();
        let decoded: GuideArtifact = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, artifact);
    }

    #[test]
    fn test_sse_path_2x2_conversion_reconstructs_rectangular_intermediates() {
        let a = SqMatrix::new([[2, 1], [1, 1]]);
        let step1 = EsseStep {
            u: DynMatrix::new(2, 3, vec![1, 0, 1, 0, 1, 0]),
            v: DynMatrix::new(3, 2, vec![1, 0, 1, 1, 1, 1]),
        };
        let mid = step1.v.mul(&step1.u);
        let step2 = EsseStep {
            u: DynMatrix::new(3, 2, vec![1, 0, 0, 1, 0, 1]),
            v: DynMatrix::new(2, 3, vec![1, 0, 1, 1, 1, 1]),
        };
        let b = step2.v.mul(&step2.u).to_sq::<2>().unwrap();
        let path = SsePath {
            matrices: vec![a, b],
            steps: vec![step1, step2],
        };

        let dyn_path: DynSsePath = path.into();
        assert_eq!(dyn_path.matrices.len(), 3);
        assert_eq!(dyn_path.matrices[0].rows, 2);
        assert_eq!(dyn_path.matrices[1], mid);
        assert_eq!(dyn_path.matrices[2].rows, 2);
    }

    #[test]
    fn test_guided_refinement_config_defaults_to_single_round() {
        let config = GuidedRefinementConfig::default();
        assert_eq!(config.max_shortcut_lag, 3);
        assert_eq!(config.min_gap, 2);
        assert_eq!(config.max_gap, None);
        assert_eq!(config.rounds, 1);
    }
}
