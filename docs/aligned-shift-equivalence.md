# Aligned Shift Equivalence

This note records the current implementation target for aligned shift equivalence in this repo.

## What is fixed already

For square matrices `A` and `B`, a shift equivalence of lag `m >= 1` is given by nonnegative matrices `R` and `S` such that

```text
A^m = RS
B^m = SR
AR = RB
BS = SA
```

This is the algebraic substrate that aligned shift equivalence refines.

The project now has a dedicated Rust module, [src/aligned.rs](/home/kasper/dev/sse-rust/src/aligned.rs), for:

- verifying a proposed fixed-lag 2x2 shift equivalence witness
- bounded search for such witnesses
- verifying the graph/module shift-equivalence witness of Definition 5.1 in Brix, Dor-On, Hazrat & Ruiz (2025)
- verifying the graph/module alignment diagrams of Definition 5.2 from the same paper
- bounded brute-force search for aligned module witnesses for small 2x2 cases

In code terms, this means the repo now has explicit witness data for the four fiberwise bijections

- `sigma_g : E^1 ⊗ G^1 -> G^1 ⊗ F^1`
- `sigma_h : F^1 ⊗ H^1 -> H^1 ⊗ E^1`
- `omega_e : G^1 ⊗ H^1 -> (E^1)^⊗m`
- `omega_f : H^1 ⊗ G^1 -> (F^1)^⊗m`

and can check the associator relations `(5.3)` and `(5.4)` directly on enumerated edge/path bases for 2x2 examples.

The bounded search is intentionally exposed as module-level search only. It should not be used as a proof of SSE, because Remark 5.5 in the 2025 paper explicitly says it is not currently known whether aligned module shift equivalence implies strong shift equivalence.

There is also a separate WASM entry point, `search_aligned_module`, exposed from [src/wasm.rs](/home/kasper/dev/sse-rust/src/wasm.rs). It returns witness data only for the experimental module-level search and is intentionally separate from `search_sse`.

## What is still blocked

The exact matrix-level alignment constraints are still not encoded locally, and this is because the primary matrix-level source is not yet present in the repo.

The repo README points to:

- Carlsen, Dor-On & Eilers (2024), which establishes intermediary relations between SE and SSE
- Brix, Dor-On, Hazrat & Ruiz (2025), which states that a forthcoming work defines matrix-level aligned shift equivalence and proves it equivalent to SSE

Concretely, after checking the local references:

- [references/carlsen-doron-eilers-2024-2011.10320.pdf](/home/kasper/dev/sse-rust/references/carlsen-doron-eilers-2024-2011.10320.pdf) proves several intermediary relations and shows they collapse back to SSE in the Cuntz-Krieger setting, but it does not give the matrix-level aligned witness format used by this repo's search code.
- [references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf](/home/kasper/dev/sse-rust/references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf) defines module shift equivalence in Definition 5.1 and aligned module shift equivalence in Definition 5.2. That is exactly the level now implemented in `src/aligned.rs`.
- The same 2025 paper says in Remark 5.3 that a forthcoming work by Bilich, Dor-On and Ruiz defines aligned shift equivalence for finite essential matrices and proves it equivalent to strong shift equivalence.
- Remark 5.5 in the 2025 paper also states that it is not currently known whether aligned module shift equivalence implies strong shift equivalence.

So the missing ingredient is not "more brute force" over the current module witness space. The missing ingredient is the exact matrix-level relation itself:

- what the matrix-level witness data is
- what equations/compatibility constraints that witness must satisfy
- what the fixed-lag algorithmic search space should be
- and the precise theorem connecting that relation back to SSE

Until that definition is transcribed precisely from the primary source, an implementation that claims to decide aligned shift equivalence of matrices would still be guessing.

## Practical rollout

1. Keep aligned work separate from the existing ESSE BFS in `src/search.rs`.
2. Use fixed-lag SE witnesses as the reusable base layer.
3. Use the current graph/module validator as a correctness target for any matrix-level reformulation.
4. Add the matrix-level alignment constraints once the primary definition is transcribed from the source.
5. Only then integrate an aligned fixed-lag solver into the top-level SSE search.

## Current benchmark snapshot

Using the Criterion benchmarks in [benches/search.rs](/home/kasper/dev/sse-rust/benches/search.rs):

- For the elementary pair `[[2,1],[1,1]]` ↔ `[[1,1],[1,2]]`, the current BFS search runs in about `1.76 µs`, while the current aligned-module brute-force search takes about `19 ms` and hits its witness limit under the benchmark configuration.
- For the Brix-Ruiz `k=3` pair `[[1,3],[2,1]]` ↔ `[[1,6],[1,1]]`, the current BFS search takes about `342 ms`, while the aligned-module search returns in about `206 µs` but exhausts the bounded search without finding a witness.

This means the present aligned-module implementation is useful as an experimental search substrate, but not yet as a competitive solver. The immediate need is search-space reduction, not more exhaustive benchmarking.

## Immediate next coding step

This work is blocked until the primary matrix-level source is available.

Once unblocked, the next coding step is:

1. Implement the exact matrix-level aligned witness and validator in `src/aligned.rs`.
2. Add a bounded `find_aligned_shift_equivalence_with_lag_2x2` solver.
3. Cross-check that matrix-level witnesses imply the currently implemented module-level validator on 2x2 examples.
4. Only then wire the aligned fixed-lag solver into `search_sse_2x2`.
