use wasm_bindgen::prelude::*;

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

fn dynmatrix_to_vecs(m: &DynMatrix) -> Vec<Vec<u32>> {
    (0..m.rows)
        .map(|i| (0..m.cols).map(|j| m.get(i, j)).collect())
        .collect()
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
