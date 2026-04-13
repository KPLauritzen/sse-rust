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

/// One elementary SSE step: A = UV, B = VU.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EsseStep {
    pub u: DynMatrix,
    pub v: DynMatrix,
}

/// A chain of elementary SSE steps connecting A to B.
#[derive(Clone, Debug)]
pub struct SsePath<const N: usize> {
    /// The sequence of matrices: A = matrices[0], B = matrices[last].
    pub matrices: Vec<SqMatrix<N>>,
    /// The elementary steps: matrices[i] = steps[i].u * steps[i].v,
    /// matrices[i+1] = steps[i].v * steps[i].u.
    pub steps: Vec<EsseStep>,
}

/// A chain of elementary SSE steps connecting arbitrary square endpoints.
#[derive(Clone, Debug)]
pub struct DynSsePath {
    /// The sequence of matrices: A = matrices[0], B = matrices[last].
    pub matrices: Vec<DynMatrix>,
    /// The elementary steps: matrices[i] = steps[i].u * steps[i].v,
    /// matrices[i+1] = steps[i].v * steps[i].u.
    pub steps: Vec<EsseStep>,
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
    pub move_family_telemetry: BTreeMap<String, SearchMoveFamilyTelemetry>,
    pub layers: Vec<SearchLayerTelemetry>,
}

#[cfg(test)]
mod tests {
    use super::SearchMode;

    #[test]
    fn test_search_mode_deserializes_snake_and_kebab_case_graph_only() {
        let snake: SearchMode = serde_json::from_str("\"graph_only\"").unwrap();
        let kebab: SearchMode = serde_json::from_str("\"graph-only\"").unwrap();

        assert_eq!(snake, SearchMode::GraphOnly);
        assert_eq!(kebab, SearchMode::GraphOnly);
    }
}
