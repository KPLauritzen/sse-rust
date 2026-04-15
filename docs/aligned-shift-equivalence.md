# Concrete Shift Surface

This note lives at `aligned-shift-equivalence.md` for historical reasons. It
records the current concrete-shift surface in this repo: aligned concrete
shift, balanced concrete shift, and compatible concrete shift, all currently
implemented in [`src/concrete_shift.rs`](../src/concrete_shift.rs).

## Current Repo Status

For square matrices `A` and `B`, a shift equivalence of lag `m >= 1` is given
by nonnegative matrices `R` and `S` such that

```text
A^m = RS
B^m = SR
AR = RB
BS = SA
```

This is the algebraic substrate that aligned concrete shift, balanced concrete
shift, and compatible concrete shift refine.

The project now has a dedicated Rust module, [`src/concrete_shift.rs`](../src/concrete_shift.rs), for:

- verifying a proposed fixed-lag `2x2` shift-equivalence witness,
- verifying concrete-shift witnesses for the aligned concrete shift, balanced
  concrete shift, and compatible concrete shift relations from Bilich, Dor-On
  & Ruiz (2024),
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
- and several comments that still mention the older module-level viewpoint.

Repo-facing docs should describe this file as the **concrete-shift surface**.

## What Changed

Earlier local notes treated aligned shift equivalence as blocked on a missing
primary source. That is no longer the right status:

- the repo now contains
  [`references/bilich-dor-on-ruiz-2024-2411.05598/paper.pdf`](../references/bilich-dor-on-ruiz-2024-2411.05598/paper.pdf),
  which defines matrix-level aligned concrete shift, balanced concrete shift,
  and compatible concrete shift for finite essential matrices,
- [`references/carlsen-doron-eilers-2024-2011.10320.pdf`](../references/carlsen-doron-eilers-2024-2011.10320.pdf)
  remains useful background for the intermediary relations and the
  operator-algebraic framing,
- [`references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf`](../references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf)
  is still relevant for the older module-level viewpoint and for the legacy
  naming still present in this code.

So the repo is no longer blocked on source acquisition. This note is about the
current surface and terminology; implementation sequencing belongs in `bd`.

## Durable Caveats

- Public naming is mixed. Several types and functions still say `module` even
  though the comments and verification logic now target the concrete
  matrix-level relation.
- The bounded witness search is still experimental and small-case oriented. The
  main problem is search-space reduction, not simply raising witness limits.
- The top-level docs were written at different times, so `README.md`,
  `docs/TODO.md`, and this note need to stay consistent about what the code
  actually does.
- The main solver's structured proof/fallback surface here is **concrete
  shift**. Sampled positive conjugacy is a separate proposal/evidence surface
  in [`src/conjugacy.rs`](../src/conjugacy.rs), not part of this proof note.
- Today the shipped `2x2` endpoint search uses bounded concrete-shift search as
  a late fallback after either mixed or graph-only frontier search exhausts on
  an essential pair within the small witness bounds.

## Practical Reading

- Treat the witness validators in [`src/concrete_shift.rs`](../src/concrete_shift.rs) as the
  ground truth for the concrete relations.
- Treat bounded concrete-shift search as an implemented experimental surface
  for small `2x2` cases, not as a complete main-solver rollout plan.
- Treat older `module` names in the public API as compatibility shims until
  the terminology cleanup tracked in `bd` lands.

## Benchmark Note

Any numeric comparison between BFS and the current concrete-shift search
surface should be rerun from [`benches/search.rs`](../benches/search.rs)
before citing it. The qualitative status is simpler: the current
`src/concrete_shift.rs` code is useful as an experimental search substrate, but the
next gains are more likely to come from better witness-space guidance than from
pushing the current brute-force search much harder.
