# Aligned Shift Equivalence

This note records the current aligned, balanced, and compatible concrete-shift
status in this repo.

## Current Repo Status

For square matrices `A` and `B`, a shift equivalence of lag `m >= 1` is given
by nonnegative matrices `R` and `S` such that

```text
A^m = RS
B^m = SR
AR = RB
BS = SA
```

This is the algebraic substrate that aligned, balanced, and compatible
concrete-shift relations refine.

The project now has a dedicated Rust module, [`src/aligned.rs`](../src/aligned.rs), for:

- verifying a proposed fixed-lag `2x2` shift-equivalence witness,
- verifying concrete-shift witnesses for the aligned, balanced, and compatible
  matrix-level relations from Bilich, Dor-On & Ruiz (2024),
- bounded search for such witnesses on small `2x2` cases,
- backwards-compatible wrappers that preserve the older local `module`
  terminology where the public API has not been renamed yet.

In code terms, this means the repo has explicit witness data for the four
fiberwise bijections

- `sigma_g : E^1 ⊗ G^1 -> G^1 ⊗ F^1`
- `sigma_h : F^1 ⊗ H^1 -> H^1 ⊗ E^1`
- `omega_e : G^1 ⊗ H^1 -> (E^1)^⊗m`
- `omega_f : H^1 ⊗ G^1 -> (F^1)^⊗m`

and can check the corresponding path-bijection relations directly on enumerated
edge/path bases for `2x2` examples.

Several public names still reflect the older local vocabulary:

- `ModuleShiftWitness2x2`,
- `search_aligned_module_shift_equivalence_*`,
- [`search_aligned_module`](../src/wasm.rs) as a compatibility alias for older
  frontend code.

The current WASM surface in [`src/wasm.rs`](../src/wasm.rs) exposes:

- `search_sse`, which can return equivalence via an aligned concrete-shift
  witness,
- `search_aligned_shift`, the experimental bounded aligned-concrete-shift
  search entry point,
- `search_aligned_module`, the backwards-compatible alias.

## What Changed

Earlier local notes treated aligned shift equivalence as blocked on a missing
primary source. That is no longer the right status:

- the repo now contains
  [`references/bilich-dor-on-ruiz-2024-2411.05598/paper.pdf`](../references/bilich-dor-on-ruiz-2024-2411.05598/paper.pdf),
  which defines matrix-level aligned, balanced, and compatible concrete shift
  equivalence for finite essential matrices,
- [`references/carlsen-doron-eilers-2024-2011.10320.pdf`](../references/carlsen-doron-eilers-2024-2011.10320.pdf)
  remains useful background for the intermediary relations and the
  operator-algebraic framing,
- [`references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf`](../references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf)
  is still relevant for the older module-level viewpoint and for the legacy
  naming still present in this code.

So the repo is no longer blocked on source acquisition. The remaining work is
implementation and integration.

## Remaining Gaps

- Public naming is mixed. Several types and functions still say `module` even
  though the comments and verification logic now target the concrete
  matrix-level relation.
- The bounded witness search is still experimental and small-case oriented. The
  main problem is search-space reduction, not simply raising witness limits.
- The top-level docs were written at different times, so `README.md`,
  `docs/TODO.md`, and this note need to stay consistent about what the code
  actually does.
- The main solver still needs a clear strategy for when aligned, balanced, or
  compatible search should serve as a direct proof path, a fallback, or a
  proposal generator.

## Practical rollout

1. Keep the witness validators in [`src/aligned.rs`](../src/aligned.rs) as the
   ground truth for the concrete relations.
2. Continue using bounded concrete-shift search as an experimental sidecar and
   focused proof aid for small `2x2` cases.
3. Standardize terminology so `aligned shift` refers to the concrete
   matrix-level relation and old `module` names are clearly marked as
   compatibility shims.
4. Decide which of aligned, balanced, or compatible search should be the
   mainline bounded formulation in [`src/search.rs`](../src/search.rs).
5. Only then expand the search surface or benchmark effort further.

## Benchmark Note

Any numeric comparison between BFS and aligned-concrete-shift search should be
rerun from [`benches/search.rs`](../benches/search.rs) before citing it. The
qualitative status is simpler: the current aligned code is useful as an
experimental search substrate, but the next gains are more likely to come from
better witness-space guidance than from pushing the current brute-force search
much harder.
