# RFC 002: Make `shortcut_search` An Artifact-Driven Iterative Solver Stage

## Status

Proposed

## Summary

Implement `shortcut_search` as the generic solver stage that repeatedly
improves a pool of reusable guide artifacts under bounded budgets.

`guided_refinement` should remain the inner primitive: given one guide path and
one bounded configuration, try to replace subsegments with shorter witnesses.

`shortcut_search` should sit one level above that primitive:

- load and validate a compatible guide pool,
- rank and deduplicate guides,
- run bounded refinement/search rounds across that pool,
- retain improved witnesses as new guide artifacts,
- and report the best witness found for the requested endpoints.

This stage should be endpoint-agnostic for square matrices and should not
reintroduce benchmark-family-specific sidecar logic into the solver.

## Context

[`rfc-001-main-search-shortcut-integration.md`](rfc-001-main-search-shortcut-integration.md)
explicitly left one gap after the six-phase rollout: `shortcut_search` was
named as a generic stage, but only `guided_refinement` was actually delivered.

That gap is now clearer.

The current branch has landed:

- generic guide artifacts,
- generic `guided_refinement`,
- guide artifact export from `search`,
- harness support for guide-aware stages,
- and a generic per-segment timeout for guided segment searches.

That work is enough to test the intended architecture directly.

Recent hand-held `k = 3` exploration on the Brix-Ruiz endpoints showed:

- a fresh graph-only witness exported as a guide artifact had lag `17`,
- one bounded mixed `guided_refinement` pass improved it to `13`,
- a further tight-bounds run with `--guided-rounds 3` improved `13 -> 9`,
- and another three tight rounds from the `9`-step guide produced no further
  improvement.

Those results matter for design.

They show that:

- the generic guide-artifact path works,
- iterative refinement matters,
- per-segment timeout matters,
- but a single-guide local refinement loop is not yet the same thing as the
  older shortcut-search workflow that historically reached lag `7`.

The old sidecar reached stronger results by doing more than one-guide
compression:

- it reused prior shortcut outputs,
- it operated on a growing guide pool,
- it used tight segment budgets such as roughly `10s` per attempted shortcut,
- and it accumulated improved paths across repeated rounds.

That workflow is the missing generic `shortcut_search`.

## Problem

The current solver surface is still missing the artifact-driven outer loop that
made shortcut search productive.

### 1. `guided_refinement` Is Only The Inner Primitive

Today `guided_refinement` can:

- accept one or more guide artifacts,
- run bounded subsegment replacement,
- and iterate for a fixed number of rounds within one invocation.

But it is still fundamentally a local path-compression stage.

It does not itself define a reusable guide-pool workflow, artifact promotion
policy, or repeated search loop over accumulated results.

### 2. `shortcut_search` Exists In Name Only

The CLI and type surface already name `shortcut_search`, but the solver still
returns an "unintegrated" error for that stage.

That is now misleading.

The architecture already has enough of the required primitives that the missing
piece is no longer "unknown future research". It is a concrete stage design
problem.

### 3. The Best Known Generic Workflow Is Too Manual

Right now, recovering the strongest generic behavior requires manual steps:

1. run endpoint search,
2. export a guide artifact,
3. run bounded refinement,
4. export the improved guide,
5. repeat on the improved artifact,
6. compare results by hand.

That is good enough for exploration, but too weak for disciplined search and
too awkward for the harness to compare honestly.

### 4. Persistence And Reuse Are Still Underpowered

The generic solver can now consume and emit guide artifacts, but it does not
yet have a first-class story for:

- loading a guide pool from a directory or manifest,
- deduplicating competing witnesses,
- writing improved artifacts back into that pool,
- or preferring promising guides under bounded runtime.

Without that, `shortcut_search` cannot match the useful behavior of the old
shortcut sidecar, even if `guided_refinement` itself keeps improving.

## Goals

- Implement `shortcut_search` as a real generic solver stage.
- Keep benchmark families out of solver logic; operate only on endpoint pairs
  plus generic guide artifacts and budgets.
- Treat `guided_refinement` as the inner primitive used by
  `shortcut_search`, not as the entire shortcut workflow.
- Support bounded runtime per attempted shortcut segment.
- Support bounded work over a pool of guides rather than only one guide.
- Provide enough telemetry that long runs are inspectable and comparable.
- Give the harness a stable stage it can schedule, reuse, and compare.

## Non-Goals

- Do not make shortcut search part of the default endpoint-search path.
- Do not reintroduce Brix-Ruiz-specific sqlite schemas or family-specific
  control flow into the solver.
- Do not force the solver to own campaign scheduling across many invocations.
- Do not require a database-backed guide store for the first generic version.
- Do not collapse `guided_refinement` and `shortcut_search` into one vague
  stage name.

## Design Principles

### `guided_refinement` Is The Local Optimizer

`guided_refinement` should stay small and legible:

- one request,
- one bounded configuration,
- one current guide at a time,
- and local route recomposition from bounded segment searches.

That makes it a reusable building block.

### `shortcut_search` Is The Pool Manager

`shortcut_search` should own:

- guide-pool loading,
- ranking and deduplication,
- repeated refinement over multiple candidate guides,
- promotion of improved witnesses back into the pool,
- and best-witness selection for the current request.

This stage is still solver orchestration, not harness campaign scheduling,
because it is one bounded solver invocation for one endpoint pair.

### Filesystem Artifacts First, Richer Stores Later

The first generic version should work with guide artifacts on disk:

- one or more explicit files,
- and optionally one or more directories or manifests.

That is enough to make reuse real without prematurely locking the design to a
database schema.

### Tight Budgets Must Be First-Class

The old shortcut workflow only stayed useful because each attempted shortcut
had a clear budget.

The generic design should keep that lesson explicit:

- segment budgets are a core control surface,
- guide-pool size is a core control surface,
- total segment-attempt budget is a core control surface,
- and per-round stopping conditions are a core control surface.

## Proposal

### 1. Define `shortcut_search` As A Multi-Guide Iterative Stage

`shortcut_search` should accept the same endpoint request shape as other solver
stages, plus a shortcut-specific configuration that controls:

- maximum guides considered,
- guide ranking policy,
- number of shortcut rounds,
- per-guide rounds,
- per-segment timeout,
- maximum total segment attempts,
- per-round promotion policy,
- and optional artifact output targets.

At a minimum, one invocation should be able to:

1. load compatible guide artifacts,
2. normalize and deduplicate them,
3. select an initial working set,
4. run bounded refinement/search over that set,
5. add improved witnesses back into the candidate pool,
6. stop when no round improves the best witness or budgets are exhausted,
7. return the best witness plus stage telemetry.

### 2. Keep `guided_refinement` As The Inner Engine

The first implementation should avoid duplicating the segment-shortening logic.

Instead, `shortcut_search` should use the existing guide-aware machinery:

- validate and reanchor guide artifacts,
- attempt bounded segment replacements,
- stitch improved routes,
- and measure guide/segment telemetry.

The main new work is not the local refinement logic itself.
It is the artifact-pool loop around it.

### 3. Introduce A Generic Guide-Pool Surface

The stage should support guide reuse through generic artifact inputs.

The first version should support:

- repeated `--guide-artifacts PATH`,
- a `--guide-artifact-dir DIR` style input for loading many artifacts,
- and one or more output paths or directories for improved artifacts.

Compatibility should be explicit for the first version.
`shortcut_search` should accept:

- stage-agnostic full-path artifacts,
- artifacts tagged for `shortcut_search`,
- and legacy full-path artifacts tagged for `guided_refinement`, because the
  current exported guide pool is still produced in that form.

The first implementation should not require a pre-migration step before the
existing guide-artifact pool can seed `shortcut_search`.
Once `shortcut_search` can emit artifacts itself, newly written artifacts
should either be stage-agnostic or explicitly tagged for both
`guided_refinement` and `shortcut_search`.

Guide identity should also be concrete for the first version.
At minimum the implementation should:

- treat endpoint identity as exact requested endpoint identity after
  orientation and re-anchoring,
- deduplicate guides by the re-anchored full-path matrix sequence, so reversed
  or otherwise equivalent imports collapse to one canonical witness,
- rank guides first by effective lag, then by any explicit cost/score metadata,
  then by a stable deterministic tie-breaker such as artifact ID or input path,
- and treat missing lag metadata as "compute lag from the witness" rather than
  as an undefined ranking case.

### 4. Add Stage-Level Budgets And Stopping Conditions

The stage needs bounded work beyond the inner search configuration.

At minimum:

- per-segment timeout,
- maximum guides processed,
- maximum rounds,
- maximum total segment attempts across the whole invocation,
- and a stop condition when one full round yields no promoted improvements.

The design should assume these budgets are part of normal operation, not only
emergency guardrails.

For the first generic version, the total segment-attempt bound should be the
hard global cap.
Combined with per-segment timeout, that gives one invocation a predictable
worst-case cost even when guide promotion grows the pool.

### 5. Expose Progress And Outcome Telemetry

The current generic refinement runs are too opaque during long searches.

`shortcut_search` should report enough telemetry to answer:

- how many guide artifacts were loaded,
- how many were accepted,
- how many unique guides entered the working set,
- how many segment attempts ran,
- how many segment attempts improved,
- how many improved guides were promoted,
- what the best lag was at the start and end of each round,
- and why the stage stopped.

This is necessary both for interactive use and for harness comparisons.

## Proposed CLI Direction

Illustrative, not final:

```text
search A B \
  --search-mode mixed \
  --stage shortcut-search \
  --guide-artifact-dir research/guide_artifacts/brix-k3 \
  --shortcut-max-guides 32 \
  --shortcut-rounds 5 \
  --guided-rounds 1 \
  --guided-max-shortcut-lag 3 \
  --guided-min-gap 2 \
  --guided-max-gap 4 \
  --guided-segment-timeout 10 \
  --write-guide-artifact-dir research/runs/shortcut-output
```

The important part is the separation of concerns:

- `guided_*` flags continue to describe the local segment-shortening primitive,
- `shortcut_*` flags describe the outer multi-guide iterative stage,
- and cross-run schedules still belong to the harness.

## Harness Integration

The harness should treat `shortcut_search` as one stage option among others.

That lets it compare, for example:

- endpoint search only,
- one-shot guided refinement from one seeded guide,
- shortcut search over a small guide pool,
- and shortcut search over a reused artifact directory from prior runs.

The harness should own:

- which endpoint families are used,
- which artifact directories are reused,
- and which schedules are compared.

The solver stage should only own one bounded invocation.

## Rollout Plan

### Phase 1: Define The Stage Boundary

- add `ShortcutSearchConfig` to the generic request surface,
- define stage-level telemetry fields,
- and settle the input/output artifact interfaces.

### Phase 2: Implement Filesystem Guide-Pool Reuse

- load guide artifacts from repeated files plus directories,
- accept legacy `guided_refinement` full-path artifacts as valid
  `shortcut_search` seeds,
- normalize and deduplicate compatible guides,
- rank them by simple quality rules,
- and emit improved artifacts generically.

### Phase 3: Implement The Iterative Pool Loop

- run bounded refinement over the working set,
- promote improved witnesses,
- stop on exhaustion or no-improvement rounds,
- and return the best witness.

### Phase 4: Integrate Into The Harness

- add fixture-driven shortcut-search cases,
- compare guide-pool sizes and reuse policies,
- require at least one non-`2x2` square case in the normal harness comparison
  flow so endpoint-agnostic behavior has a real regression surface,
- and reproduce the old bounded Brix-Ruiz shortcut workflow through generic
  stage inputs rather than a family-specific sidecar.

## Alternatives Considered

### Keep Extending `guided_refinement`

Rejected as the main public design because it makes one stage name carry both:

- local path compression,
- and multi-guide artifact-pool search.

That would blur a useful boundary.

### Reintroduce A Shortcut-Specific Database Stage

Rejected for the first generic version because it would pull the design back
toward family-specific persistence before the generic artifact model is fully
settled.

### Leave The Outer Loop Entirely To The Harness

Rejected because one bounded multi-guide iterative search over one endpoint
pair is still solver orchestration, not only campaign scheduling.

The harness should schedule invocations of `shortcut_search`, not be forced to
reimplement its inner loop.

## Recommendation

Adopt this RFC as the next follow-up to
[`rfc-001-main-search-shortcut-integration.md`](rfc-001-main-search-shortcut-integration.md).

The project now has enough evidence to make `shortcut_search` concrete:

- generic guide artifacts work,
- generic guided refinement works,
- export/import works,
- per-segment timeout is now available,
- and the remaining missing value is the artifact-driven outer loop.

That outer loop should become a real solver stage rather than remain a manual
workflow assembled around `guided_refinement`.
