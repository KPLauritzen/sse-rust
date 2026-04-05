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

## What is still blocked

The exact matrix-level alignment constraints are not yet encoded locally.

The repo README points to:

- Carlsen, Dor-On & Eilers (2024), which establishes intermediary relations between SE and SSE
- Brix, Dor-On, Hazrat & Ruiz (2025), which states that a forthcoming work defines matrix-level aligned shift equivalence and proves it equivalent to SSE

The 2025 paper gives the graph/module aligned condition as shift equivalence plus associator relations, but that does not by itself provide a ready-to-code matrix-only verifier. Until that definition is transcribed precisely, an implementation that claims to decide aligned shift equivalence would be guessing.

## Practical rollout

1. Keep aligned work separate from the existing ESSE BFS in `src/search.rs`.
2. Use fixed-lag SE witnesses as the reusable base layer.
3. Add the matrix-level alignment constraints once the primary definition is transcribed from the source.
4. Only then integrate an aligned fixed-lag solver into the top-level SSE search.

## Immediate next coding step

Implement the exact matrix-level alignment witness and validator in `src/aligned.rs`, then add a bounded `find_aligned_shift_equivalence_with_lag_2x2` solver before wiring it into `search_sse_2x2`.
