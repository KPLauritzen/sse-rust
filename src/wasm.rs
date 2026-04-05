use wasm_bindgen::prelude::*;

use crate::aligned::{
    search_aligned_module_shift_equivalence_2x2, AlignedModuleSearchConfig2x2,
    AlignedModuleSearchResult2x2, ModuleShiftWitness2x2,
};
use crate::matrix::{DynMatrix, SqMatrix};
use crate::search::search_sse_2x2;
use crate::types::SearchConfig;

#[derive(serde::Serialize)]
struct WasmSseResult {
    status: String,
    reason: Option<String>,
    /// Each step is a (U, V) pair: the current matrix equals UV, the next equals VU.
    steps: Option<Vec<WasmSseStep>>,
}

#[derive(serde::Serialize)]
struct WasmSseStep {
    u: Vec<Vec<u32>>,
    v: Vec<Vec<u32>>,
}

#[derive(serde::Serialize)]
struct WasmAlignedModuleResult {
    status: String,
    witness: Option<WasmAlignedModuleWitness>,
}

#[derive(serde::Serialize)]
struct WasmAlignedModuleWitness {
    lag: u32,
    r: Vec<Vec<u32>>,
    s: Vec<Vec<u32>>,
    sigma_g: Vec<Vec<usize>>,
    sigma_h: Vec<Vec<usize>>,
    omega_e: Vec<Vec<usize>>,
    omega_f: Vec<Vec<usize>>,
}

fn dynmatrix_to_vecs(m: &DynMatrix) -> Vec<Vec<u32>> {
    (0..m.rows)
        .map(|i| (0..m.cols).map(|j| m.get(i, j)).collect())
        .collect()
}

fn sqmatrix_to_vecs<const N: usize>(m: &SqMatrix<N>) -> Vec<Vec<u32>> {
    (0..N)
        .map(|i| (0..N).map(|j| m.data[i][j]).collect())
        .collect()
}

fn module_witness_to_wasm(witness: &ModuleShiftWitness2x2) -> WasmAlignedModuleWitness {
    WasmAlignedModuleWitness {
        lag: witness.shift.lag,
        r: sqmatrix_to_vecs(&witness.shift.r),
        s: sqmatrix_to_vecs(&witness.shift.s),
        sigma_g: witness.sigma_g.mapping.to_vec(),
        sigma_h: witness.sigma_h.mapping.to_vec(),
        omega_e: witness.omega_e.mapping.to_vec(),
        omega_f: witness.omega_f.mapping.to_vec(),
    }
}

/// Search for strong shift equivalence between two 2x2 nonneg integer matrices.
///
/// Returns a JSON string with the result.
#[wasm_bindgen]
pub fn search_sse(
    a00: u32,
    a01: u32,
    a10: u32,
    a11: u32,
    b00: u32,
    b01: u32,
    b10: u32,
    b11: u32,
    max_lag: usize,
    max_intermediate_dim: usize,
    max_entry: u32,
) -> String {
    let a = SqMatrix::new([[a00, a01], [a10, a11]]);
    let b = SqMatrix::new([[b00, b01], [b10, b11]]);
    let config = SearchConfig {
        max_lag,
        max_intermediate_dim,
        max_entry,
        ..SearchConfig::default()
    };

    let result = search_sse_2x2(&a, &b, &config);

    let wasm_result = match result {
        crate::types::SseResult::Equivalent(path) => {
            let steps: Vec<WasmSseStep> = path
                .steps
                .iter()
                .map(|s| WasmSseStep {
                    u: dynmatrix_to_vecs(&s.u),
                    v: dynmatrix_to_vecs(&s.v),
                })
                .collect();
            WasmSseResult {
                status: "equivalent".into(),
                reason: None,
                steps: Some(steps),
            }
        }
        crate::types::SseResult::NotEquivalent(reason) => WasmSseResult {
            status: "not_equivalent".into(),
            reason: Some(reason),
            steps: None,
        },
        crate::types::SseResult::Unknown => WasmSseResult {
            status: "unknown".into(),
            reason: None,
            steps: None,
        },
    };

    serde_json::to_string(&wasm_result).unwrap()
}

/// Search for a bounded aligned module shift-equivalence witness between two 2x2 matrices.
///
/// This is experimental and should not be interpreted as an SSE proof procedure:
/// the current implementation searches for the graph/module aligned witnesses
/// from Brix, Dor-On, Hazrat & Ruiz (2025), not the forthcoming matrix-level
/// aligned shift equivalence relation.
#[wasm_bindgen]
pub fn search_aligned_module(
    a00: u32,
    a01: u32,
    a10: u32,
    a11: u32,
    b00: u32,
    b01: u32,
    b10: u32,
    b11: u32,
    max_lag: u32,
    max_entry: u32,
    max_module_witnesses: usize,
) -> String {
    let a = SqMatrix::new([[a00, a01], [a10, a11]]);
    let b = SqMatrix::new([[b00, b01], [b10, b11]]);
    let config = AlignedModuleSearchConfig2x2 {
        max_lag,
        max_entry,
        max_module_witnesses,
    };

    let result = search_aligned_module_shift_equivalence_2x2(&a, &b, &config);

    let wasm_result = match result {
        AlignedModuleSearchResult2x2::Equivalent(witness) => WasmAlignedModuleResult {
            status: "equivalent".into(),
            witness: Some(module_witness_to_wasm(&witness)),
        },
        AlignedModuleSearchResult2x2::Exhausted => WasmAlignedModuleResult {
            status: "exhausted".into(),
            witness: None,
        },
        AlignedModuleSearchResult2x2::SearchLimitReached => WasmAlignedModuleResult {
            status: "search_limit_reached".into(),
            witness: None,
        },
    };

    serde_json::to_string(&wasm_result).unwrap()
}
