# RFC: Integrate Refinement And Shortcut Search Into The Main Solver

## Status

Proposed

## Summary

Integrate the current refinement and shortcutting workflows from the research
binaries into the main search stack and CLI, so the project has one primary
solver with multiple search strategies instead of a small main solver plus a
growing sidecar lab.

In practice, this means:

- keeping `search` as the primary entry point,
- moving graph-path guidance and shortcut compression behind explicit strategy
  stages and budgets,
- treating `k = 3` and `k >= 4` Brix-Ruiz cases as first-class search targets,
- and reserving sidecar binaries for temporary diagnostics, profiling, and
  one-off experiments rather than core solver capability.

## Context

The project has now crossed an important threshold:

1. `k = 3` has a known graph-only path.
2. The remaining research goals are not "find any witness at all" in the
   abstract, but:
   - find a shorter `k = 3` path, ideally below lag `7`,
   - and find any path for `k = 4` or above.

At the same time, the codebase has split into two realities:

- the main solver path in [`src/search.rs`](../src/search.rs) and the `search`
  CLI,
- and a collection of research binaries in [`src/bin/`](../src/bin/) that now
  contain some of the most relevant search machinery for the hard cases.

Examples of the current split:

- `search` supports `mixed` and `graph-only` modes and is the public-facing
  solver.
- sidecar binaries such as `find_brix_ruiz_path_shortcuts`,
  `compare_brix_ruiz_graph_paths`, `check_lind_marcus_path`, and the waypoint
  and graph-path tools contain search logic that is directly relevant to the
  hard benchmark family.

This split was useful while the project was still answering questions like:

- can graph-only search recover any known `k = 3` witness at all,
- can waypoint-guided compression beat blind endpoint search,
- and which move families are worth keeping?

But the split is now starting to hurt both correctness and iteration:

- the main CLI no longer reflects the strongest search capability in the repo,
- profiling the "main solver" does not necessarily profile the full search
  workflow that matters for `k = 3` and `k = 4`,
- budget management is fragmented across binaries,
- and expensive long-running refinement/shortcut experiments are hard to
  compare, resume, or integrate with the normal telemetry surface.

## Problem

The current architecture makes it too easy for important search ideas to remain
outside the main solver path.

That has three concrete costs.

### 1. Product Capability Drift

The project presents `search` as the main solver, but meaningful progress on
hard cases increasingly comes from sidecar flows rather than from the main CLI.

This creates an avoidable mismatch between:

- what the repo can actually do,
- what the main CLI exposes,
- and what the harness can score directly.

### 2. Research Turnaround Cost

Raw compute is already a bottleneck:

- a graph-only `k = 3` solution is available in seconds,
- but end-to-end shortcut compression can require tens of minutes.

If the refinement machinery remains outside the main search stack, there is no
single place to express:

- fast inner-loop budgets,
- slower guided compression budgets,
- resumable long-running campaigns,
- or shared telemetry and persistence across those stages.

### 3. Strategy Fragmentation

The repo now contains multiple search substrates:

- blind graph search,
- mixed factorisation search,
- structured graph moves,
- waypoint-guided graph search,
- shortcut/refinement search seeded by known guide paths,
- and concrete-shift sidecars.

These should be organized as strategies inside one solver pipeline, not treated
as separate disconnected programs.

## Goals

- Make the main CLI the canonical interface to the strongest search workflows in
  the repo.
- Treat refinement and shortcutting as solver strategies, not sidecar-only
  experiments.
- Preserve fast iteration by separating cheap inner-loop stages from expensive
  long-running stages.
- Reuse existing persistence and telemetry surfaces where possible.
- Keep default behavior stable until the integrated strategies are mature.
- Make `k = 3` lag improvement and `k = 4+` path discovery first-class search
  objectives.

## Non-Goals

- Do not merge every experimental binary into the main CLI wholesale.
- Do not remove one-off diagnostics that are genuinely useful for temporary
  inspection or paper reproduction.
- Do not force the default `search` mode to immediately run the most expensive
  shortcut pipeline.
- Do not change correctness semantics to accept heuristic witnesses as proofs.

## Proposal

### 1. Introduce A Strategy Layer In The Main Solver

Keep the low-level search primitives where they are, but add an explicit
strategy layer that can orchestrate the current search substrates.

The main solver should support staged execution such as:

- `graph-only`
- `mixed`
- `guided-shortcut`
- `campaign`

These names are illustrative; the important part is that they are explicit and
budgeted.

Each strategy stage should be able to:

- receive a common search request,
- emit telemetry in a common format,
- optionally consume persisted guide data,
- and either return a final witness or hand off to the next stage.

### 2. Make Shortcutting A First-Class Solver Stage

The current shortcut/refinement workflow should be promoted from sidecar logic
to an internal strategy that can be invoked by the main CLI.

That stage should:

- accept an optional guide path or waypoint set,
- run bounded segment-shortening or refinement search,
- persist improved segment results,
- and return a shorter path if found.

The key point is that this should not be wired in as an unstructured fallback.
It should be an explicit stage with its own budgets and telemetry.

### 3. Separate Fast And Slow Search Loops

The integrated solver should acknowledge that the project now has multiple
iteration cadences.

Recommended structure:

- Fast loop:
  `graph-only`, `mixed`, and cheap proposal generation on `k = 3` and `k = 4`
- Medium loop:
  reduced-budget guided shortcutting on selected segments or guide paths
- Long loop:
  full campaign runs that attempt route compression or `k = 4+` discovery with
  resume/persistence

The main CLI should let the user select these explicitly rather than embedding
all of them in one opaque default run.

### 4. Reuse The Existing Persistence Surface

The current persisted visited-graph support in the main `search` CLI and the
existing sqlite-backed guide-path work already point in the right direction.

The integrated design should converge on:

- one canonical visited-graph/persistence story for ordinary search,
- one canonical guide/result store for shortcut campaigns,
- and explicit CLI options for reading and writing both.

This avoids re-solving the same expensive segments repeatedly and reduces the
cost of long-running refinement work.

### 5. Keep Research Binaries, But Demote Their Role

After integration, sidecar binaries should mostly serve one of three jobs:

- profiling and instrumentation,
- paper-specific reproduction checks,
- or targeted diagnostics while a new idea is still unproven.

They should no longer be the only place where the strongest search logic lives.

## Proposed CLI Direction

The current `search` CLI already has the right shape for this evolution:

- it accepts endpoints,
- it has explicit search-mode flags,
- it supports telemetry,
- and it can persist visited search graphs.

The next step is to extend it with explicit strategy controls rather than adding
more separate binaries.

Illustrative options:

```text
search A B \
  --strategy mixed \
  --strategy graph-only \
  --strategy guided-shortcut \
  --guide-db k3-guides.sqlite \
  --segment-timeout-ms 10000 \
  --campaign-budget-ms 60000
```

This example is intentionally schematic. The exact flag names can change, but
the core principle should remain:

- one CLI,
- multiple named stages,
- explicit budgets,
- explicit persistence.

## Architecture Sketch

### Core Layer

Unchanged responsibilities:

- move generation,
- factorisation enumeration,
- invariants,
- graph moves,
- witness validation,
- path reconstruction.

### Strategy Layer

New responsibilities:

- stage selection and ordering,
- waypoint/guide consumption,
- segment-shortening orchestration,
- budget enforcement,
- resume and persistence policy.

This is the layer that should absorb the current shortcut/refinement orchestration.

### CLI/Harness Layer

Responsibilities:

- expose strategies and budgets cleanly,
- present results and telemetry,
- allow campaign resumption,
- and align the harness with real project objectives.

## Why This Is Better Than More Sidecars

### It Makes Profiling Honest

Once shortcutting and refinement live inside the main solver path, profiling the
main CLI actually profiles the real search stack.

### It Improves Research Throughput

A single staged solver makes it easier to:

- short-circuit expensive stages when cheap stages already fail,
- reuse persisted results,
- and compare strategies under the same telemetry surface.

### It Clarifies Project Identity

The project is now fundamentally about hard search:

- shortening `k = 3`,
- and solving `k = 4+`.

Those are not sidecar activities. They are the main content of the solver.

## Risks

### 1. The Main CLI Becomes Too Complex

Risk:
integrating too many experimental ideas at once could turn `search` into a
large, hard-to-reason-about pile of fallbacks.

Mitigation:

- integrate one strategy at a time,
- keep default behavior stable,
- and require each new strategy to have explicit budgets and telemetry.

### 2. Expensive Search Becomes The Default

Risk:
integrating shortcutting could accidentally make ordinary runs much slower.

Mitigation:

- keep long-running stages opt-in at first,
- and treat campaign search as a named mode rather than implicit behavior.

### 3. Strategy Logic Becomes Hard To Test

Risk:
multi-stage orchestration is harder to regression-test than one bounded BFS.

Mitigation:

- add strategy-level tests for stage selection and resume behavior,
- and promote `k = 3` / `k = 4` benchmark cases into explicit solver targets.

## Rollout Plan

### Phase 1

- Define the strategy interface inside the main solver.
- Keep existing `mixed` and `graph-only` behavior intact.
- Do not change default CLI behavior yet.

### Phase 2

- Integrate one existing shortcut/refinement path as an internal strategy.
- Reuse current persistence mechanisms where possible.
- Expose it as an explicit non-default CLI mode.

### Phase 3

- Add reduced-budget campaign support and resumable guide usage.
- Extend the research harness so it can score solver stages that actually target
  shorter `k = 3` witnesses and `k = 4+` discovery.

### Phase 4

- Retire or demote the overlapping research binaries whose logic is now fully
  covered by the main solver path.

## Alternatives Considered

### Keep The Current Split

Rejected because it keeps the strongest hard-case search logic outside the main
solver and makes runtime/persistence policy increasingly fragmented.

### Merge Everything Into The Default Search Path Immediately

Rejected because the runtime cost is too high and the research cadence now
depends on having distinct fast and slow loops.

### Create A Separate "Campaign" CLI Instead Of Extending `search`

Possible, but weaker than extending the main CLI. It preserves the same
architectural split under a different name and does not solve the capability
drift problem as directly.

## Recommendation

Adopt this RFC.

The repo should evolve toward one primary solver with explicit staged search
strategies, not toward a growing cluster of disconnected research binaries.

That matches the current state of the project:

- the main value is the search,
- `k = 3` and `k = 4+` are the hard benchmark family,
- and refinement/shortcutting is no longer "extra tooling" but part of the
  solver itself.
