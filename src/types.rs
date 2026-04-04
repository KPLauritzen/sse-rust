use crate::matrix::{DynMatrix, SqMatrix};

/// Configuration for the SSE search.
#[derive(Clone, Debug)]
pub struct SearchConfig {
    /// Maximum number of elementary SSE steps to search.
    pub max_lag: usize,
    /// Maximum intermediate dimension for factorisations (m in n×m × m×n).
    /// For now, only m = n (square factorisations) is implemented.
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
