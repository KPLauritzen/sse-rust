# RFC 003: Introduce A Shared Vocabulary For Structured Witness And Proposal Surfaces

## Status

Accepted

## Summary

Introduce one explicit vocabulary above the repo's three current structured
`2x2` search surfaces:

- concrete-shift search and verification in [`src/aligned.rs`](../../src/aligned.rs),
- balanced-elementary search in [`src/balanced.rs`](../../src/balanced.rs),
- and sampled positive-conjugacy witness / proposal search in
  [`src/conjugacy.rs`](../../src/conjugacy.rs).

This RFC does **not** propose forcing all three into one proof interface.

Instead, it proposes:

- reserving **concrete shift** for the aligned / balanced / compatible family
  implemented in `src/aligned.rs`,
- distinguishing **balanced concrete shift** from
  **balanced elementary equivalence**,
- treating positive conjugacy as a proposal/evidence surface rather than as a
  proof shortcut,
- renaming `src/aligned.rs` to a broader module name such as
  `src/concrete_shift.rs` in a separate mechanical cleanup step,
- and adding a small shared vocabulary layer so the solver, harness, binaries,
  and documentation can describe these surfaces consistently.

The immediate payoff is clarity:

- which surfaces produce certified proofs,
- which surfaces are currently solver-integrated,
- which surfaces are still sidecars,
- and which surfaces should be treated as proposal generators rather than as
  interchangeable witness engines.

This RFC also assumes that [`TERMINOLOGY.md`](../../TERMINOLOGY.md) is part of
the rollout surface. If this vocabulary is adopted, `TERMINOLOGY.md` must be
updated to reflect the new distinctions explicitly.

## Context

The repo currently has three nearby but semantically different modules:

- [`src/aligned.rs`](../../src/aligned.rs)
  - verifies fixed-lag shift-equivalence witnesses,
  - verifies aligned / balanced / compatible concrete-shift witnesses,
  - and performs bounded concrete-shift search through
    `search_concrete_shift_equivalence_with_lag_2x2`.
- [`src/balanced.rs`](../../src/balanced.rs)
  - searches for bounded balanced-elementary witnesses of the form
    `A = S R_A`, `B = S R_B`, `R_A S = R_B S`.
- [`src/conjugacy.rs`](../../src/conjugacy.rs)
  - searches for bounded sampled positive-conjugacy witnesses,
  - and derives ranked proposal matrices from those sampled witnesses.

Those modules sit close to one another in the literature and in the repo's
idea bank, but they do not currently play the same role in the product.

Current integration status is asymmetrical:

- `src/aligned.rs` is actively used by the main solver through the `2x2`
  concrete-shift fallback in [`src/search.rs`](../../src/search.rs), and its
  result is modeled directly in [`src/types.rs`](../../src/types.rs) as
  `EquivalentByConcreteShift`.
- `src/balanced.rs` is currently a sidecar proof search surface, mainly used by
  [`src/bin/find_balanced.rs`](../../src/bin/find_balanced.rs).
- `src/conjugacy.rs` is currently a research/proposal surface, mainly used by
  [`src/bin/find_positive_conjugacy.rs`](../../src/bin/find_positive_conjugacy.rs)
  and the proposal/usefulness evaluators.

This asymmetry is not obvious from the module names alone.

The worst offender is `src/aligned.rs` itself.

That file now implements the broad concrete-shift family surface for:

- aligned concrete shift,
- balanced concrete shift,
- compatible concrete shift,

but its module name still sounds like one specific subtype.

So the repo currently has a mismatch between:

- the broad semantics of the module,
- and the narrow suggestion of its filename.

There is also a naming collision:

- `ConcreteShiftRelation2x2::Balanced` in `src/aligned.rs` means
  **balanced concrete shift** in the Bilich-Dor-On-Ruiz sense.
- `BalancedElementaryWitness2x2` in `src/balanced.rs` means
  **balanced elementary equivalence** in the Brix-style factor form.

Those are related but not identical ideas, and the repo should stop talking as
if a bare word like **balanced** were enough to identify the surface.

Positive conjugacy adds a second source of confusion:

- it is mathematically adjacent to structured SSE arguments,
- and it may be a useful proposal source,
- but the repo's own notes already say it should not be treated as a proof of
  SSE over `Z_+`.
- and its current witness object records a sampled affine path, not an exact
  certified positive-conjugacy proof.

That means the repo needs a shared vocabulary more than it needs one shared
algorithm trait.

The repo already has a project-wide terminology file,
[`TERMINOLOGY.md`](../../TERMINOLOGY.md), but this particular distinction is
not yet expressed there with enough precision. Any adoption of this RFC should
be treated as a required terminology-file update, not just an RFC-local idea.

## Problem

The current code and docs are too loose about what kind of surface each module
actually is.

### 1. The Repo Conflates Family Names

Today, a phrase like **balanced search** can mean at least two different things:

- balanced concrete shift inside `src/aligned.rs`,
- or balanced-elementary witnesses inside `src/balanced.rs`.

That is bad for:

- code review,
- task scoping,
- harness reporting,
- and future CLI/config naming.

### 2. Proof Surfaces And Proposal Surfaces Are Not Distinguished Clearly

The repo currently mixes together:

- certified proof-producing surfaces,
- bounded proof-search sidecars,
- and proposal/evidence generators.

`src/conjugacy.rs` should not look interchangeable with `src/aligned.rs`, but
there is no shared vocabulary making that distinction explicit.

### 3. The Main Solver Uses One Surface But The Docs Talk About Three

In practice, the main solver currently knows about concrete-shift proofs.

It does **not** currently consume:

- balanced-elementary witnesses,
- or sampled positive-conjugacy witnesses/proposals,

as first-class solver stages.

Without a shared vocabulary, that difference is easy to blur in roadmap
discussion and future refactors.

### 4. Future Integration Work Needs Shared Reporting Before Shared Search Code

The repo is more likely to benefit from:

- shared descriptors,
- shared reporting vocabulary,
- shared harness/result metadata,

than from immediately trying to deduplicate the algorithms in those modules.

The missing abstraction is first a semantic one, not an implementation one.

## Goals

- Make the repo explicit about which surfaces are:
  - certified proof surfaces,
  - bounded sidecar proof searches,
  - or proposal/evidence sources.
- Reserve **concrete shift** for the aligned / balanced / compatible family in
  `src/aligned.rs`.
- Disambiguate **balanced concrete shift** from
  **balanced elementary equivalence** everywhere the repo reports or documents
  them.
- Give the solver, harness, and research binaries a shared vocabulary for these
  surfaces without pretending they already share one execution model.
- Prepare a clean path for future integration work, especially if balanced or
  conjugacy-driven methods become solver-facing later.

## Non-Goals

- Do not merge `src/aligned.rs`, `src/balanced.rs`, and `src/conjugacy.rs`
  into one module.
- Do not claim that positive conjugacy is a proof shortcut.
- Do not force balanced-elementary search into the main solver in this RFC.
- Do not rename every public API immediately if compatibility shims are still
  useful.
- Do not require one umbrella trait that every structured search surface must
  implement.

## Design Principles

### Semantics First

The first abstraction should describe what a surface means:

- proof,
- bounded proof search,
- proposal source,

before it describes how a surface is executed.

### One Family Name Per Mathematical Family

The repo should use:

- **concrete shift** only for aligned / balanced / compatible concrete-shift
  relations,
- **balanced elementary** for the Brix-style `S, R_A, R_B` witness surface,
- and **positive conjugacy** for the sampled conjugacy-path / proposal surface.

The repo should also prefer family-shaped names for modules and docs when a
surface has already grown beyond one subtype.

That means the codebase should talk about `src/aligned.rs` as the
**concrete-shift surface** even before any mechanical rename lands.

### Shared Vocabulary, Not Forced Uniformity

The right first step is:

- common descriptors,
- common reporting labels,
- and optional capability-specific interfaces,

not one trait that collapses proof and proposal surfaces into the same shape.

### Solver Integration Must Track Correctness Semantics

If a surface is solver-facing, its proof semantics must be explicit.

Today that means:

- concrete-shift results can participate in the solver's result surface,
- balanced-elementary search remains a sidecar proof search,
- positive conjugacy remains a proposal/evidence source.

## Proposal

### 1. Define A Shared Descriptor Vocabulary

Add one small descriptive layer, preferably in a new module such as
`src/structured_witness.rs` or `src/structured_search.rs`.

At minimum, define:

```rust
pub enum StructuredSurfaceFamily2x2 {
    ConcreteShift(ConcreteShiftRelation2x2),
    BalancedElementary,
    PositiveConjugacy,
}

pub enum StructuredSurfaceSemantics {
    CertifiedProof,
    CertifiedProofSearch,
    ProposalSource,
}

pub enum StructuredSurfaceUsage {
    MainSolverFallback,
    SidecarProofSearch,
    SidecarProposalSearch,
}

pub struct StructuredSurfaceDescriptor2x2 {
    pub family: StructuredSurfaceFamily2x2,
    pub semantics: StructuredSurfaceSemantics,
    pub usage: StructuredSurfaceUsage,
    pub user_label: &'static str,
}
```

The exact type names are not important.
The important part is that the repo gets one canonical way to say:

- what family a surface belongs to,
- whether its outputs are proof-grade or proposal-grade,
- and how the current product uses it.

### 2. Make `src/aligned.rs` The Explicit Concrete-Shift Surface

Treat `src/aligned.rs` as the concrete-shift implementation surface, even if
the file name remains in place for compatibility for a while.

The preferred end state is to rename it to something like:

- `src/concrete_shift.rs`

so the module name matches the family it actually implements.

This rename should be treated as a mechanical terminology cleanup, not as a
semantic redesign.

Until that rename happens, docs, RFCs, beads, and review comments should avoid
using the bare module name `aligned` as the family label.

Prefer phrasing like:

- concrete-shift surface in `src/aligned.rs`
- concrete-shift fallback
- concrete-shift proof search

This keeps the vocabulary stable before and after the file rename.

Its descriptor mapping should be explicit:

- `ConcreteShift(Aligned)` -> certified proof search, currently solver-facing
- `ConcreteShift(Balanced)` -> certified proof search, currently searched
  through the same bounded engine
- `ConcreteShift(Compatible)` -> certified proof search, currently searched
  through the same bounded engine

In other words, `src/aligned.rs` is already the common interface for the
concrete-shift family.
The repo should acknowledge that directly instead of implying that
`src/balanced.rs` is an equal peer inside the same family.

If the rename is deferred, the repo should still document clearly that
`aligned.rs` is a historical file name for the broader concrete-shift surface,
not the name of one isolated relation.

### 3. Distinguish `BalancedElementary` From `BalancedConcreteShift`

The repo should stop using bare **balanced** in new user-facing descriptions.

Use one of:

- **balanced concrete shift**
- **balanced elementary equivalence**

depending on which module/surface is meant.

This distinction should be reflected in:

- doc phrasing,
- bead descriptions,
- harness/report summaries if these surfaces are exposed there,
- and any future CLI surface.

This is the most important vocabulary cleanup in the proposal.

### 4. Treat Positive Conjugacy As A Proposal Surface

`src/conjugacy.rs` should be described as:

- a sampled positive-conjugacy witness search,
- with proposal derivation,
- currently used as a proposal/evidence source rather than as an exact proof
  path.

The current implementation validates the affine path at fixed sample points,
so "sampled positive-conjugacy witness" is the accurate repo-facing term today.

That means its descriptor should be explicit:

- `PositiveConjugacy` -> proposal source -> sidecar proposal search

If a future RFC promotes conjugacy-derived proposals into the main solver, that
solver integration should consume proposals, not pretend that the conjugacy
search itself is a proof-producing surface.

### 5. Add Capability-Specific Interfaces Only If Needed

If the code later wants interfaces, prefer two narrow traits over one umbrella
trait:

```rust
pub trait StructuredProofSearch2x2 {
    type Config;
    type Witness;
    type Result;

    fn descriptor() -> StructuredSurfaceDescriptor2x2;
}

pub trait StructuredProposalSource2x2 {
    type Config;
    type Proposal;
    type Output;

    fn descriptor() -> StructuredSurfaceDescriptor2x2;
}
```

The repo should avoid a trait shape that forces:

- concrete-shift proofs,
- balanced-elementary witnesses,
- and conjugacy proposals

into one result enum just for uniformity.

### 6. Keep Solver Result Modeling Honest

The current solver result type already has a specific branch for concrete-shift
proofs in [`src/types.rs`](../../src/types.rs).

That is the correct current state.

This RFC recommends:

- keeping solver-grade proof results explicit,
- not widening `EquivalentByConcreteShift` into a vague
  `EquivalentByStructuredWitness`,
- and only generalizing the solver result model if another proof-grade surface
  actually becomes solver-integrated.

In other words, shared vocabulary should arrive before any attempt to
generalize the main solver result enum.

### 7. Reflect The Vocabulary In Docs And Research Notes

Update the documentation language over time so it says:

- `src/aligned.rs` -> concrete shift,
- `src/balanced.rs` -> balanced elementary,
- `src/conjugacy.rs` -> positive conjugacy proposal surface.

The aligned-shift note, research-ideas note, and future RFCs should all use the
same terms.

In particular, [`TERMINOLOGY.md`](../../TERMINOLOGY.md) should be updated as
part of the first rollout step so the repo's canonical vocabulary file
distinguishes:

- balanced concrete shift,
- balanced elementary equivalence,
- and positive conjugacy as a proposal surface.

## Current Mapping

Under this RFC, the repo's current surfaces would map like this:

| Module | Family | Semantics | Current usage |
| --- | --- | --- | --- |
| `src/aligned.rs` | `ConcreteShift(Aligned/Balanced/Compatible)` | certified proof search | main-solver fallback plus direct bounded search |
| `src/balanced.rs` | `BalancedElementary` | certified proof search | sidecar proof search |
| `src/conjugacy.rs` | `PositiveConjugacy` | proposal source | sidecar proposal/evidence search |

That table is the core of the proposal.

## Why This Is Better

### It Matches The Current Code Reality

The repo already treats `src/aligned.rs` differently from the other two
modules.
The new vocabulary would make that explicit.

### It Resolves The Worst Naming Collision

The word **balanced** is currently overloaded.
This RFC gives the repo a durable way to separate the two meanings.

### It Gives Future Integration Work A Clean Starting Point

If balanced-elementary search or conjugacy proposals become more central later,
the repo will already have:

- a family name,
- a semantics label,
- and a current-usage label

to hang those changes on.

### It Avoids A Fake Abstraction

The repo does not currently need one trait that says every structured surface
is "the same thing".
This RFC keeps the common layer descriptive until real execution-model reuse
exists.

## Rollout Plan

### Phase 1: Land The Vocabulary And Doc Cleanup

- add the RFC,
- add the shared descriptor types or at least settle their naming,
- update [`TERMINOLOGY.md`](../../TERMINOLOGY.md) so the canonical repo
  vocabulary reflects the new distinctions,
- update docs and notes to describe `src/aligned.rs` as the concrete-shift
  surface even before any file rename happens,
- and update affected docs and bead phrasing over time.

### Phase 2: Add A Concrete-Shift Facade If It Helps

Prefer a direct rename of `src/aligned.rs` to `src/concrete_shift.rs` once the
terminology is accepted.

If that rename is too disruptive in the short term, add a small facade or
re-export layer such as `src/concrete_shift.rs` that fronts `src/aligned.rs`
until the old module name can be retired.

The important point is that the repo should move toward a module name that
matches the broad concrete-shift family rather than preserving `aligned.rs` as
the durable public concept.

### Phase 3: Use The Descriptor In Reporting Surfaces

If the harness, sqlite export, or research binaries start reporting these
surfaces side by side, use the shared descriptor vocabulary instead of
ad-hoc strings.

### Phase 4: Revisit Solver Integration Case By Case

Only after the vocabulary is stable should the repo decide whether:

- balanced-elementary search deserves solver-facing integration,
- or conjugacy proposals deserve a generic proposal-source hook.

Those are separate RFCs or beads, not part of this terminology RFC.

## Alternatives Considered

### Treat All Three As "Concrete Shift"

Rejected.

This would flatten a real semantic difference:

- concrete shift is already a specific proof family,
- balanced-elementary search is a different witness family,
- and positive conjugacy is not currently proof-grade.

### Put Everything Behind One `WitnessSearch2x2` Trait

Rejected for now.

That would force proposal surfaces and proof surfaces into one execution shape
before the repo has demonstrated that such a trait buys anything.

### Leave The Naming As-Is

Rejected.

The current naming is already confusing enough to cause design discussions to
slide between:

- balanced concrete shift,
- balanced elementary search,
- and conjugacy-guided proposals

without a stable shared vocabulary.

## Recommendation

Adopt this RFC as a terminology and interface policy:

- `src/aligned.rs` is the concrete-shift family surface,
- `src/balanced.rs` is the balanced-elementary sidecar surface,
- `src/conjugacy.rs` is the positive-conjugacy proposal surface,
- and the repo should add shared descriptors and reporting vocabulary before it
  tries to unify their execution models.

That gives the project a cleaner language for future solver integration work
without pretending that these three modules already want the same interface.
