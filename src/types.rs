use crate::concrete_shift::{ConcreteShiftRelation2x2, ConcreteShiftWitness2x2};
use serde::{Deserialize, Serialize};

use crate::matrix::{DynMatrix, SqMatrix};
use crate::structured_surface::StructuredSurfaceDescriptor2x2;

pub use crate::telemetry::{
    EndpointExactMeetSurface, EndpointExactMeetWitness, SameFuturePastDiversityLayerSample,
    SameFuturePastDiversityTelemetry, SearchDirection, SearchLayerTelemetry,
    SearchLayerTimingTelemetry, SearchMoveFamilyTelemetry, SearchTelemetry,
    ShortcutSearchRoundTelemetry, ShortcutSearchStopReason, ShortcutSearchTelemetry,
    StratifiedBeamRefillTelemetry,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FrontierMode {
    Bfs,
    Beam,
    ConcreteShiftProfileBeam,
    WitnessBridgeProfileBeam,
    SparseK4BridgeProfileBeam,
    SameFuturePastDiversityBeam,
    BeamBfsHandoff,
    StratifiedBeamRefill,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MoveFamilyPolicy {
    Mixed,
    #[serde(alias = "graph-plus-structured")]
    GraphPlusStructured,
    #[serde(alias = "graph-only")]
    GraphOnly,
}

pub const DEFAULT_BEAM_WIDTH: usize = 64;

impl Default for FrontierMode {
    fn default() -> Self {
        Self::Bfs
    }
}

impl FrontierMode {
    pub fn uses_beam_width(self) -> bool {
        matches!(
            self,
            Self::Beam
                | Self::ConcreteShiftProfileBeam
                | Self::WitnessBridgeProfileBeam
                | Self::SparseK4BridgeProfileBeam
                | Self::SameFuturePastDiversityBeam
                | Self::BeamBfsHandoff
                | Self::StratifiedBeamRefill
        )
    }
}

impl Default for MoveFamilyPolicy {
    fn default() -> Self {
        Self::Mixed
    }
}

impl MoveFamilyPolicy {
    pub fn permits_factorisations(self) -> bool {
        !matches!(self, Self::GraphOnly)
    }

    pub fn includes_square_factorisation_3x3(self) -> bool {
        matches!(self, Self::Mixed)
    }

    pub fn snake_case_label(self) -> &'static str {
        match self {
            Self::Mixed => "mixed",
            Self::GraphPlusStructured => "graph_plus_structured",
            Self::GraphOnly => "graph_only",
        }
    }

    pub fn kebab_case_label(self) -> &'static str {
        match self {
            Self::Mixed => "mixed",
            Self::GraphPlusStructured => "graph-plus-structured",
            Self::GraphOnly => "graph-only",
        }
    }
}

/// High-level solver stage terminology. This is intentionally separate from
/// [`FrontierMode`] and [`MoveFamilyPolicy`], which select the low-level search
/// substrate and allowed move families.
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
#[serde(default)]
pub struct GuidedRefinementConfig {
    pub max_shortcut_lag: usize,
    pub min_gap: usize,
    pub max_gap: Option<usize>,
    pub rounds: usize,
    pub segment_timeout_secs: Option<u64>,
}

impl Default for GuidedRefinementConfig {
    fn default() -> Self {
        Self {
            max_shortcut_lag: 3,
            min_gap: 2,
            max_gap: None,
            rounds: 1,
            segment_timeout_secs: None,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutGuideRankingPolicy {
    #[default]
    LagCostScoreThenStable,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ShortcutPromotionPolicy {
    #[default]
    ImprovedOnly,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutSearchArtifactOutputConfig {
    /// Request that improved full-path guides be emitted on the generic surface.
    pub emit_promoted_guides: bool,
    /// Compatibility tags to attach to emitted guides. Empty means stage-agnostic.
    pub supported_stages: Vec<SearchStage>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ShortcutSearchConfig {
    /// Hard cap on guides admitted to the initial ranked working set.
    pub max_guides: usize,
    /// Stable ranking policy for guide-pool admission.
    pub ranking: ShortcutGuideRankingPolicy,
    /// Maximum outer shortcut-search rounds.
    pub rounds: usize,
    /// Hard cap across all segment attempts in one stage invocation.
    pub max_total_segment_attempts: usize,
    /// Promotion rule for improved guides between rounds.
    pub promotion: ShortcutPromotionPolicy,
    /// Output policy for promoted guides produced by the stage.
    pub artifacts: ShortcutSearchArtifactOutputConfig,
}

impl Default for ShortcutSearchConfig {
    fn default() -> Self {
        Self {
            max_guides: 32,
            ranking: ShortcutGuideRankingPolicy::LagCostScoreThenStable,
            rounds: 5,
            max_total_segment_attempts: 128,
            promotion: ShortcutPromotionPolicy::ImprovedOnly,
            artifacts: ShortcutSearchArtifactOutputConfig::default(),
        }
    }
}

/// Configuration for the SSE search.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchConfig {
    /// Maximum number of elementary SSE steps to search.
    pub max_lag: usize,
    /// Maximum intermediate dimension for factorisations (m in n×m × m×n).
    /// Current search supports 2x2 square steps and 2x2 <-> 3x3 rectangular steps.
    pub max_intermediate_dim: usize,
    /// Maximum entry value in intermediate matrices U, V.
    pub max_entry: u32,
    /// Frontier expansion style.
    pub frontier_mode: FrontierMode,
    /// Allowed move families during frontier expansion.
    pub move_family_policy: MoveFamilyPolicy,
    /// Optional best-first beam frontier cap. `None` preserves layer-synchronous BFS.
    pub beam_width: Option<usize>,
    /// Optional inclusive depth cutoff for handing beam discoveries over to BFS.
    ///
    /// `None` preserves the existing built-in default handoff depth.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beam_bfs_handoff_depth: Option<usize>,
    /// Optional cap on retained deferred overflow entries for `beam_bfs_handoff`.
    ///
    /// `None` preserves the existing unlimited deferred queue.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub beam_bfs_handoff_deferred_cap: Option<usize>,
    /// Optional cap on retained exact endpoint meets for the CLI multi-meet surface.
    ///
    /// `None` preserves the existing single-result early-return behavior.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub endpoint_multi_meet_cap: Option<usize>,
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_lag: 4,
            max_intermediate_dim: 2,
            max_entry: 25,
            frontier_mode: FrontierMode::Bfs,
            move_family_policy: MoveFamilyPolicy::Mixed,
            beam_width: None,
            beam_bfs_handoff_depth: None,
            beam_bfs_handoff_deferred_cap: None,
            endpoint_multi_meet_cap: None,
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
    pub shortcut_search: ShortcutSearchConfig,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConcreteShiftProof2x2 {
    pub relation: ConcreteShiftRelation2x2,
    pub witness: ConcreteShiftWitness2x2,
}

impl ConcreteShiftProof2x2 {
    pub fn descriptor(&self) -> StructuredSurfaceDescriptor2x2 {
        StructuredSurfaceDescriptor2x2::concrete_shift(self.relation)
    }

    pub fn description(&self) -> String {
        self.descriptor().reporting_label().to_string()
    }
}

/// Result of an SSE search.
#[derive(Clone, Debug)]
pub enum SseResult<const N: usize> {
    /// Found a path proving SSE.
    Equivalent(SsePath<N>),
    /// Found a direct aligned/balanced/compatible concrete-shift witness.
    EquivalentByConcreteShift(ConcreteShiftProof2x2),
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

/// Generic structured proof payload shared by result/event/persistence layers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum StructuredProofResult {
    ConcreteShift2x2(ConcreteShiftProof2x2),
}

impl StructuredProofResult {
    pub fn outcome_label(&self) -> &'static str {
        match self {
            Self::ConcreteShift2x2(_) => "equivalent_by_concrete_shift",
        }
    }

    pub fn description(&self) -> String {
        match self {
            Self::ConcreteShift2x2(proof) => proof.description(),
        }
    }

    pub fn relation_label(&self) -> Option<&'static str> {
        match self {
            Self::ConcreteShift2x2(proof) => Some(proof.relation.as_str()),
        }
    }

    pub fn as_concrete_shift_2x2(&self) -> Option<&ConcreteShiftProof2x2> {
        match self {
            Self::ConcreteShift2x2(proof) => Some(proof),
        }
    }
}

impl From<ConcreteShiftProof2x2> for StructuredProofResult {
    fn from(proof: ConcreteShiftProof2x2) -> Self {
        Self::ConcreteShift2x2(proof)
    }
}

/// Generic result boundary shared by request/result/event/persistence layers.
#[derive(Clone, Debug)]
pub enum SearchRunResult {
    Equivalent(DynSsePath),
    EquivalentByStructuredProof(StructuredProofResult),
    NotEquivalent(String),
    Unknown,
}

impl SearchRunResult {
    pub fn outcome_label(&self) -> &'static str {
        match self {
            Self::Equivalent(_) => "equivalent",
            Self::EquivalentByStructuredProof(proof) => proof.outcome_label(),
            Self::NotEquivalent(_) => "not_equivalent",
            Self::Unknown => "unknown",
        }
    }
}

impl From<SseResult<2>> for SearchRunResult {
    fn from(result: SseResult<2>) -> Self {
        match result {
            SseResult::Equivalent(path) => Self::Equivalent(path.into()),
            SseResult::EquivalentByConcreteShift(proof) => {
                Self::EquivalentByStructuredProof(proof.into())
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

#[cfg(test)]
mod tests {
    use super::{
        ConcreteShiftProof2x2, DynMatrix, DynSsePath, EsseStep, FrontierMode, GuideArtifact,
        GuideArtifactCompatibility, GuideArtifactEndpoints, GuideArtifactPayload,
        GuideArtifactProvenance, GuideArtifactQuality, GuideArtifactValidation,
        GuidedRefinementConfig, MoveFamilyPolicy, SearchConfig, SearchStage,
        ShortcutGuideRankingPolicy, ShortcutPromotionPolicy, ShortcutSearchConfig, SsePath,
        DEFAULT_BEAM_WIDTH,
    };
    use crate::concrete_shift::{
        canonical_module_shift_witness_2x2, ConcreteShiftRelation2x2, ShiftEquivalenceWitness2x2,
    };
    use crate::matrix::SqMatrix;

    #[test]
    fn test_move_family_policy_deserializes_supported_labels() {
        let snake: MoveFamilyPolicy = serde_json::from_str("\"graph_only\"").unwrap();
        let kebab: MoveFamilyPolicy = serde_json::from_str("\"graph-only\"").unwrap();
        let structured_snake: MoveFamilyPolicy =
            serde_json::from_str("\"graph_plus_structured\"").unwrap();
        let structured_kebab: MoveFamilyPolicy =
            serde_json::from_str("\"graph-plus-structured\"").unwrap();
        let mixed: MoveFamilyPolicy = serde_json::from_str("\"mixed\"").unwrap();

        assert_eq!(snake, MoveFamilyPolicy::GraphOnly);
        assert_eq!(kebab, MoveFamilyPolicy::GraphOnly);
        assert_eq!(structured_snake, MoveFamilyPolicy::GraphPlusStructured);
        assert_eq!(structured_kebab, MoveFamilyPolicy::GraphPlusStructured);
        assert_eq!(mixed, MoveFamilyPolicy::Mixed);
    }

    #[test]
    fn test_frontier_mode_deserializes_bfs_and_beam() {
        let bfs: FrontierMode = serde_json::from_str("\"bfs\"").unwrap();
        let beam: FrontierMode = serde_json::from_str("\"beam\"").unwrap();
        let concrete_shift_profile_beam: FrontierMode =
            serde_json::from_str("\"concrete_shift_profile_beam\"").unwrap();
        let witness_bridge_profile_beam: FrontierMode =
            serde_json::from_str("\"witness_bridge_profile_beam\"").unwrap();
        let sparse_k4_bridge_profile_beam: FrontierMode =
            serde_json::from_str("\"sparse_k4_bridge_profile_beam\"").unwrap();
        let same_future_past_diversity_beam: FrontierMode =
            serde_json::from_str("\"same_future_past_diversity_beam\"").unwrap();
        let beam_bfs_handoff: FrontierMode = serde_json::from_str("\"beam_bfs_handoff\"").unwrap();
        let stratified_beam_refill: FrontierMode =
            serde_json::from_str("\"stratified_beam_refill\"").unwrap();

        assert_eq!(bfs, FrontierMode::Bfs);
        assert_eq!(beam, FrontierMode::Beam);
        assert_eq!(
            concrete_shift_profile_beam,
            FrontierMode::ConcreteShiftProfileBeam
        );
        assert_eq!(
            witness_bridge_profile_beam,
            FrontierMode::WitnessBridgeProfileBeam
        );
        assert_eq!(
            sparse_k4_bridge_profile_beam,
            FrontierMode::SparseK4BridgeProfileBeam
        );
        assert_eq!(
            same_future_past_diversity_beam,
            FrontierMode::SameFuturePastDiversityBeam
        );
        assert_eq!(beam_bfs_handoff, FrontierMode::BeamBfsHandoff);
        assert_eq!(stratified_beam_refill, FrontierMode::StratifiedBeamRefill);
    }

    #[test]
    fn test_search_config_defaults_disable_beam() {
        let config = SearchConfig::default();

        assert_eq!(config.frontier_mode, FrontierMode::Bfs);
        assert_eq!(config.move_family_policy, MoveFamilyPolicy::Mixed);
        assert_eq!(config.beam_width, None);
        assert_eq!(config.beam_bfs_handoff_depth, None);
        assert_eq!(config.beam_bfs_handoff_deferred_cap, None);
        assert_eq!(config.endpoint_multi_meet_cap, None);
        assert_eq!(DEFAULT_BEAM_WIDTH, 64);
    }

    #[test]
    fn test_search_config_deserializes_missing_handoff_depth_as_none() {
        let config: SearchConfig =
            serde_json::from_str(r#"{"max_lag":2,"max_intermediate_dim":3,"max_entry":4}"#)
                .unwrap();

        assert_eq!(config.max_lag, 2);
        assert_eq!(config.max_intermediate_dim, 3);
        assert_eq!(config.max_entry, 4);
        assert_eq!(config.beam_bfs_handoff_depth, None);
        assert_eq!(config.beam_bfs_handoff_deferred_cap, None);
        assert_eq!(config.endpoint_multi_meet_cap, None);
    }

    #[test]
    fn test_search_config_serializes_handoff_depth_when_present() {
        let config = SearchConfig {
            max_lag: 2,
            max_intermediate_dim: 3,
            max_entry: 4,
            frontier_mode: FrontierMode::BeamBfsHandoff,
            move_family_policy: MoveFamilyPolicy::GraphOnly,
            beam_width: Some(8),
            beam_bfs_handoff_depth: Some(6),
            beam_bfs_handoff_deferred_cap: Some(24),
            endpoint_multi_meet_cap: Some(3),
        };

        let encoded = serde_json::to_value(&config).unwrap();
        let object = encoded.as_object().unwrap();
        assert_eq!(
            object
                .get("beam_bfs_handoff_depth")
                .and_then(serde_json::Value::as_u64),
            Some(6)
        );
        assert_eq!(
            object
                .get("beam_bfs_handoff_deferred_cap")
                .and_then(serde_json::Value::as_u64),
            Some(24)
        );
        assert_eq!(
            object
                .get("endpoint_multi_meet_cap")
                .and_then(serde_json::Value::as_u64),
            Some(3)
        );
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
        assert_eq!(config.segment_timeout_secs, None);
    }

    #[test]
    fn test_guided_refinement_config_deserializes_missing_timeout_as_none() {
        let config: GuidedRefinementConfig =
            serde_json::from_str(r#"{"max_shortcut_lag":1,"min_gap":2,"max_gap":2,"rounds":1}"#)
                .unwrap();

        assert_eq!(config.max_shortcut_lag, 1);
        assert_eq!(config.min_gap, 2);
        assert_eq!(config.max_gap, Some(2));
        assert_eq!(config.rounds, 1);
        assert_eq!(config.segment_timeout_secs, None);
    }

    #[test]
    fn test_shortcut_search_config_defaults() {
        let config = ShortcutSearchConfig::default();
        assert_eq!(config.max_guides, 32);
        assert_eq!(
            config.ranking,
            ShortcutGuideRankingPolicy::LagCostScoreThenStable
        );
        assert_eq!(config.rounds, 5);
        assert_eq!(config.max_total_segment_attempts, 128);
        assert_eq!(config.promotion, ShortcutPromotionPolicy::ImprovedOnly);
        assert!(!config.artifacts.emit_promoted_guides);
        assert!(config.artifacts.supported_stages.is_empty());
    }

    #[test]
    fn test_shortcut_search_config_deserializes_missing_artifact_stage_list() {
        let config: ShortcutSearchConfig = serde_json::from_str(
            r#"{
                "max_guides": 8,
                "ranking": "lag_cost_score_then_stable",
                "rounds": 2,
                "max_total_segment_attempts": 16,
                "promotion": "improved_only",
                "artifacts": {
                    "emit_promoted_guides": true
                }
            }"#,
        )
        .unwrap();

        assert_eq!(config.max_guides, 8);
        assert_eq!(config.rounds, 2);
        assert_eq!(config.max_total_segment_attempts, 16);
        assert!(config.artifacts.emit_promoted_guides);
        assert!(config.artifacts.supported_stages.is_empty());
    }

    #[test]
    fn test_concrete_shift_proof_description_includes_relation() {
        let a = SqMatrix::identity();
        let witness = canonical_module_shift_witness_2x2(
            &a,
            &a,
            ShiftEquivalenceWitness2x2 {
                lag: 1,
                r: SqMatrix::identity(),
                s: SqMatrix::identity(),
            },
        )
        .unwrap();
        let proof = ConcreteShiftProof2x2 {
            relation: ConcreteShiftRelation2x2::Compatible,
            witness,
        };

        assert_eq!(proof.description(), "compatible concrete-shift witness");
    }
}
