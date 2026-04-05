use serde::{Deserialize, Serialize};

use crate::matrix::{DynMatrix, SqMatrix};

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
}

impl Default for SearchConfig {
    fn default() -> Self {
        Self {
            max_lag: 4,
            max_intermediate_dim: 2,
            max_entry: 25,
        }
    }
}

/// One elementary SSE step: A = UV, B = VU.
#[derive(Clone, Debug)]
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

/// Result of an SSE search.
#[derive(Clone, Debug)]
pub enum SseResult<const N: usize> {
    /// Found a path proving SSE.
    Equivalent(SsePath<N>),
    /// Proved not SSE by an invariant mismatch.
    NotEquivalent(String),
    /// Search exhausted without finding a path or proving non-equivalence.
    Unknown,
}

/// Direction of a BFS layer expansion in bidirectional search.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchDirection {
    Forward,
    Backward,
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
    pub discovered_nodes: usize,
    pub enqueued_nodes: usize,
    pub next_frontier_nodes: usize,
    pub total_visited_nodes: usize,
}

/// Aggregate telemetry for a full `search_sse_2x2` invocation.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct SearchTelemetry {
    pub invariant_filtered: bool,
    pub permutation_shortcut: bool,
    pub canonical_shortcut: bool,
    pub frontier_nodes_expanded: usize,
    pub factorisation_calls: usize,
    pub factorisations_enumerated: usize,
    pub candidates_generated: usize,
    pub pruned_by_size: usize,
    pub pruned_by_spectrum: usize,
    pub candidates_after_pruning: usize,
    pub collisions_with_seen: usize,
    pub collisions_with_other_frontier: usize,
    pub discovered_nodes: usize,
    pub enqueued_nodes: usize,
    pub max_frontier_size: usize,
    pub total_visited_nodes: usize,
    pub layers: Vec<SearchLayerTelemetry>,
}
