# RFC: Generalize The Main Solver And Integrate Guided Search Stages

## Status

Proposed

## Summary

Evolve the project toward one primary solver for arbitrary square endpoint
matrices, with explicit guided search stages and a stronger experiment harness.

In practice, this means:

- keeping `search` as the canonical solver entry point,
- making the solver core endpoint-agnostic for square matrices rather than
  implicitly optimized around `2x2` endpoints,
- integrating guide-aware refinement and shortcut search as generic solver
  stages rather than family-specific sidecars,
- keeping benchmark-family logic such as Brix-Ruiz `k = 3` and `k = 4` in
  helpers, fixtures, seeded databases, and the harness rather than hardcoding
  it into the solver,
- and treating campaign scheduling as a harness concern, not as a blurrier
  version of solver mode.

The immediate target is square endpoints up to dimension `4`, with room to
raise that bound later as the search stack improves.

## Context

The project has crossed two thresholds at once.

First, the hard benchmark family is clearer than before:

1. `k = 3` has a known path.
2. The near-term search goals are:
   - find a shorter `k = 3` path, ideally with lag `< 7`,
   - find any path for `k = 4` or above,
   - and make the main solver itself less special-cased around `2x2`.

Second, the codebase now has an architectural split:

- the main solver path in [`src/search.rs`](../src/search.rs) and the `search`
  CLI,
- and a growing collection of research binaries in [`src/bin/`](../src/bin/)
  that contain guide search, shortcut refinement, waypoint logic, and
  family-specific orchestration.

That split was useful while the project was still proving out ideas, but it now
creates two problems:

- the generic solver surface is weaker than the best search workflows in the
  repo,
- and the strongest search workflows are still expressed through binaries that
  are narrow in scope, hard to compare, and awkward to resume.

There is also a deeper design issue underneath the current split:

- the dynamic search path already accepts arbitrary square endpoints,
- but solver behavior is still materially split between the `2x2` and dynamic
  paths,
- persistence and observer integration are still effectively centered on
  `2x2` endpoint search,
- and some of the most productive refinement logic is currently tailored to the
  Brix-Ruiz `k = 3` case rather than written as generic guided search
  machinery.

If the project keeps building outward from those local assumptions, the solver
will become harder to generalize later.

## Problem

The current architecture is too specialized in the wrong places.

### 1. The Solver Is Not Yet Cleanly Generalized

The project wants a main solver that works for arbitrary square matrices, but
the current solver surface still has `2x2`-specific seams.

Those seams are not only about persistence.
They also include richer `2x2` invariant filtering, observer support, and
proof shortcuts that do not yet have dynamic-path equivalents.

That is tolerable for local progress on the current hard examples, but it is
the wrong direction for the architecture.

### 2. Family-Specific Workflows Are Drifting Into Core Capability

The Brix-Ruiz `k = 3` and `k = 4` cases are the best current benchmarks, but
they should remain benchmark families, not become hidden assumptions of the
solver.

The solver should search square endpoints.
The harness should decide which endpoint families, seeds, schedules, and
budgets to run.

### 3. "Mode", "Strategy", And "Campaign" Are Blurry

The repo currently mixes together several different concepts:

- low-level substrate selection such as `mixed` versus `graph-only`,
- higher-level stages such as guide generation or route refinement,
- and long-running experiment schedules that vary bounds, reuse persistence,
  and compare runs.

Those are distinct concerns and should not share one overloaded vocabulary.

### 4. The Harness Is Too Weak For The Next Phase

A lot of future progress will come from running more disciplined experiments:

- repeated bound schedules,
- A/B comparisons across strategies,
- persistent result reuse,
- and scoring based on both outcome and search quality.

That requires a stronger harness and result model than the project currently
has.

## Goals

- Make the main solver endpoint-agnostic for square matrices, with the current
  practical target being dimensions up to `4`.
- Make the solver orchestration boundary endpoint-agnostic even where some
  proof shortcuts remain intentionally `2x2`-specific for a while, provided
  those special cases are explicit.
- Keep `search` as the canonical solver interface.
- Integrate guide-aware refinement and shortcutting as generic solver stages,
  not family-specific sidecars.
- Keep benchmark-family logic such as `k = 3` and `k = 4` out of the solver
  core.
- Separate low-level search mode, higher-level stage orchestration, and harness
  campaign scheduling.
- Strengthen the harness so it becomes the main loop for iterative search
  improvement.
- Keep default solver behavior stable until the generalized orchestration is in
  place and tested.

## Non-Goals

- Do not hardcode Brix-Ruiz `k = 3`, `k = 4`, literature waypoints, or other
  benchmark-family endpoints into the main solver.
- Do not merge every research binary into `search` wholesale.
- Do not force expensive guided refinement into the default search path.
- Do not change correctness semantics to treat heuristic guides as proofs.
- Do not pretend the current persistence model is already generic enough when it
  is not.

## Design Principles

### Solver Core: Generic Endpoints

The solver core should operate on arbitrary square endpoints plus an explicit
request configuration.

It should not know why a given pair matters.
It should not know that a run belongs to `k = 3` or `k = 4`.
It should not assume literature waypoints, seeded paths, or benchmark-family
fixtures unless those are provided through generic interfaces.

### Harness: Families, Schedules, And Scoring

The harness should own:

- benchmark families,
- run schedules,
- seeded guide databases,
- persistence reuse policy across runs,
- and scoring/reporting.

This is where Brix-Ruiz-specific workflows belong.

### Helpers And Fixtures: Reproducibility

Case-specific helpers remain useful for:

- starting a known benchmark run,
- seeding a database with known paths,
- reproducing a literature example,
- or checking a narrow family-specific claim.

Those are valid support tools, but they should not define the solver
architecture.

## Proposal

### 1. Generalize The Solver Request Surface First

Before integrating more guided search logic, make the orchestration boundary
generic for square endpoints.

This phase should explicitly reconcile the current semantic split between the
`2x2` and dynamic solver paths.
The project should decide which behaviors become generic, which remain
intentionally `2x2`-specialized for now, and how that distinction is expressed
in the API, telemetry, and CLI.

The solver should accept a request that can describe:

- arbitrary square endpoints,
- low-level substrate selection,
- optional guide artifacts,
- stage budgets,
- and persistence hooks.

This is the architectural prerequisite for integrating refinement in a way that
does not keep inheriting `2x2` assumptions.

This request surface should be paired with a correspondingly generic result and
observer boundary, so dynamic endpoint search can participate in the same
telemetry and persistence pipeline rather than remaining a parallel path.

### 2. Separate Three Layers Explicitly

The project should distinguish these terms consistently.

#### Search Mode

A low-level substrate choice used inside one search stage.

Examples:

- `mixed`
- `graph-only`

This is close to what `SearchMode` already means today.

#### Strategy Stage

A bounded solver step that consumes a request and either returns a witness or
produces artifacts for the next step.

Examples:

- endpoint search
- guide generation
- guide refinement
- segment-shortening

Stages are part of the solver-side orchestration layer.

#### Campaign

A harness-level run plan that selects cases, stages, schedules, budgets, and
persistence reuse across repeated runs.

Examples:

- iterative deepening over bounds,
- compare `mixed` versus `graph-only`,
- use yesterday's guide store as seeds for today's refinement run.

Campaign is not a synonym for search mode.

### 3. Make Guided Refinement Generic Before Making It Mainline

The current shortcut/refinement workflow should not be moved into `search`
unchanged, because it is still too tied to one benchmark family.

Instead, first extract generic concepts:

- guide path input,
- waypoint or segment selection,
- bounded segment search,
- route recomposition,
- improved-route persistence,
- and stage-level telemetry.

Only after that extraction should a guided refinement stage become part of the
main solver stack.

### 4. Generalize Persistence Instead Of "Reusing" It Blindly

The current persistence story is promising but incomplete.

The integrated design should converge on two generic persistence surfaces:

- a solver-run store for endpoint search telemetry, visited nodes, and solver
  artifacts,
- and a guide/result store for reusable paths, route improvements, and other
  guided-search outputs.

These stores should be keyed by generic endpoint/config identity, not by one
benchmark family.

The important point is not to claim that persistence is already unified.
It is to make unification an explicit design goal.

### 4a. Define Guide Artifacts Before Wiring Them Into The CLI

The project should not introduce `--stage`, `--guide-db`, or equivalent solver
flags until guide artifacts are defined in generic terms.

At minimum, a reusable guide artifact should describe:

- the endpoint identity it applies to,
- the artifact kind, such as full path, segment, waypoint set, or proposal
  batch,
- the matrix payload or references needed to replay it,
- provenance, such as literature, seeded fixture, prior solver run, or manual
  import,
- validation status,
- compatibility constraints for stages that consume it,
- and quality metadata such as lag, cost, or score.

Guide artifacts should be generic solver inputs, not family labels wrapped in a
database row.

### 5. Keep Benchmark Families Out Of Core Search Logic

The Brix-Ruiz family should remain central in the harness and regression
surface, but not in the solver core.

That means:

- no hardcoded `k = 3` or `k = 4` endpoint assumptions inside the main solver,
- no solver stages that require literature-specific waypoints to exist,
- and no persistence schema that encodes one family as the primary use case.

It is fine to ship helper binaries or scripts that:

- launch Brix-Ruiz runs,
- seed guide databases from known paths,
- or replay literature witnesses.

Those belong next to the solver, not inside it.

### 6. Invest In The Harness As A First-Class Product Surface

The harness is likely to be the main multiplier for future search improvement.

The next harness should support:

- endpoint corpora for arbitrary square cases,
- repeated schedules over bounds and stages,
- result reuse across runs,
- comparison across commits or configurations,
- and scoring that tracks both correctness outcome and search quality.

The minimum viable version of that harness support is a prerequisite for solver
generalization work, not a later convenience.
If the project cannot express non-`2x2` cases in the corpus and run them
through the normal comparison loop, then endpoint-agnostic solver changes will
not have an adequate regression surface.

For current goals, "search quality" should include at least:

- witness found or not,
- best lag found so far,
- cost of the best witness search,
- and useful stage-level telemetry.

## Proposed CLI And Interface Direction

The main `search` CLI should stay focused on solving one endpoint pair.

Illustrative direction:

```text
search A B \
  --search-mode mixed \
  --stage endpoint-search \
  --stage guided-refinement \
  --guide-db guides.sqlite \
  --segment-timeout-ms 10000
```

This is intentionally schematic.
The important part is the separation of concepts:

- `--search-mode` selects a low-level substrate,
- `--stage` selects bounded solver stages,
- and repeated schedules belong to the harness rather than being overloaded into
  the solver CLI.

The harness can then build on top with commands or configs of the form:

```text
research-harness run family/brix-ruiz-k3.json \
  --schedule iterative-deepening \
  --reuse-db guides.sqlite \
  --compare baseline,new-strategy
```

Again, the exact interface can change.
The architectural boundary is the key point.

However, the CLI should only expose guide-oriented flags once the underlying
guide artifact model is generic enough that those flags do not silently mean
"Brix-Ruiz sidecar inputs, but under a new name".

## Architecture Sketch

### Core Layer

Responsibilities:

- move generation,
- factorisation enumeration,
- invariants,
- graph moves,
- witness validation,
- path reconstruction,
- and endpoint-agnostic square-matrix search.

### Orchestration Layer

Responsibilities:

- stage selection and ordering,
- guide and artifact consumption,
- stage budget enforcement,
- generic persistence hooks,
- and stage-level telemetry aggregation.

This is the layer that should absorb guided refinement once the stage interface
is generic enough.

### Harness Layer

Responsibilities:

- benchmark families,
- campaign scheduling,
- persistence reuse across runs,
- scoring and comparison,
- and experiment reporting.

This is the layer that should encode Brix-Ruiz-focused workflows.

## Why This Is Better Than More Sidecars

### It Generalizes In The Right Direction

The solver grows toward arbitrary square endpoints, not toward a larger pile of
case-specific binaries.

### It Keeps Benchmark Logic Where It Belongs

The project can stay heavily focused on `k = 3` and `k = 4` without teaching
the solver that those are special endpoint families.

### It Clarifies The Experiment Loop

The harness becomes the place to iterate on schedules, bounds, persistence
reuse, and comparisons, which is where a lot of future progress is likely to
come from.

### It Makes Profiling More Honest

Once guided stages are integrated through generic orchestration, profiling the
main solver path measures the actual solver machinery rather than a subset that
excludes the useful guided logic.

## Risks

### 1. The Solver Boundary Remains Too Vague

Risk:
the project could keep mixing substrate choice, stage orchestration, and
campaign scheduling.

Mitigation:

- define the vocabulary explicitly,
- reflect that vocabulary in types and CLI flags,
- and keep campaign logic in the harness.

### 2. Guided Refinement Gets Integrated Too Early

Risk:
a family-specific shortcut workflow could be imported into the main solver
before its abstractions are generic.

Mitigation:

- genericize guide ingestion, segment search, and persistence first,
- and only then promote the stage into the solver pipeline.

### 3. Persistence Design Stays Accidentally `2x2`-Centric

Risk:
new persistence features could keep inheriting current endpoint assumptions.

Mitigation:

- make arbitrary square endpoints the explicit design target,
- test persistence on non-`2x2` cases,
- and avoid CLI features that only work on one endpoint size unless clearly
  marked as temporary.

### 4. Harness Work Is Deferred Too Long

Risk:
the project could keep adding search ideas without improving the experiment
loop, which would slow down evaluation and comparison.

Mitigation:

- treat harness improvements as part of the main rollout,
- and make scoring and run comparison first-class deliverables.

### 5. Generic Interfaces Become Vague Before The First Real Stage Exists

Risk:
the project could spend time designing abstract request, artifact, and stage
types that are too detached from the first guided stage that actually needs to
ship.

Mitigation:

- design around one concrete guided-stage extraction target,
- keep the first generic artifact schema narrow and extensible,
- and require each new abstraction to be exercised by a real non-sidecar
  integration step.

## Rollout Plan

### Phase 1: Generalize The Boundaries

- audit the semantic differences between the `2x2` and dynamic solver paths,
- define generic request, result, observer, stage, and persistence interfaces
  for square endpoints,
- decide which behaviors remain intentionally `2x2`-specific for now and mark
  them explicitly,
- keep existing `mixed` and `graph-only` endpoint search behavior intact,
- separate `SearchMode` from stage orchestration terminology,
- and do not change default solver behavior yet.

### Phase 2: Minimum Viable Harness Generalization

- extend the case corpus and runner so non-`2x2` square endpoints can be
  expressed and executed in the normal harness flow,
- add enough result modeling to compare generalized solver runs honestly,
- preserve existing `2x2` scoring while expanding the regression surface,
- and keep the harness capable of A/B comparison across configurations.

### Phase 3: Strengthen The Harness

- add campaign scheduling concepts to the harness,
- add scoring for best lag found, not just terminal outcome,
- add reuse of persisted guides/results across runs,
- and make it easy to compare strategies across the benchmark corpus.

### Phase 4: Integrate One Guided Stage Generically

- define the first generic guide artifact format,
- extract one existing guide-aware refinement path into a generic stage,
- ensure it works for arbitrary square endpoints within current practical
  bounds,
- and expose it as an explicit non-default solver stage.

### Phase 5: Expand The Generic Experiment Surface

- promote benchmark families such as Brix-Ruiz into harness fixtures and seeded
  artifacts,
- add broader non-`2x2` regression cases,
- and compare stage combinations under the improved harness.

### Phase 6: Demote Overlapping Sidecars

- retire or demote research binaries whose logic is now covered by generic
  solver stages plus harness workflows,
- while keeping targeted diagnostics and paper-reproduction tools where they are
  still useful.

## Alternatives Considered

### Keep The Current Split

Rejected because it keeps the useful guided logic outside the main solver path
and leaves the experiment loop weaker than it needs to be.

### Integrate The Current Shortcut Binary As-Is

Rejected because that would import a still family-specific workflow into the
solver before the abstractions are generic enough.

### Put Everything Under A New Campaign CLI

Possible, but weaker than clarifying the boundary between solver and harness.
It risks renaming the confusion rather than fixing it.

## Recommendation

Adopt this RFC with the generalized boundary as the main design constraint.

The project should evolve toward:

- a main solver for arbitrary square endpoints,
- explicit guided stages with generic interfaces,
- benchmark families managed by the harness rather than hardcoded into the
  solver,
- and a stronger experiment loop that can drive future search improvements.

That is a better fit for the project's current state than continuing to grow a
`2x2`-leaning solver plus a sidecar-heavy research layer.
