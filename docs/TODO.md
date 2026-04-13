# Search Improvements

This file tracks outstanding search work after reconciling the roadmap with the
actual codebase. Items below are either still open or only partially complete.
Fully completed items have been removed.

The live actionable backlog is tracked in `bd`.
Use this document as roadmap context and status notes; use Beads for active
task tracking and prioritization.

The current solver stack is:

- bidirectional BFS in [`src/search.rs`](../src/search.rs),
- factorisation enumeration in [`src/factorisation.rs`](../src/factorisation.rs),
- invariant filtering in [`src/invariants.rs`](../src/invariants.rs),
- graph-move search in [`src/graph_moves.rs`](../src/graph_moves.rs),
- concrete-shift witness search in [`src/aligned.rs`](../src/aligned.rs),
- balanced-elementary sidecar search in [`src/balanced.rs`](../src/balanced.rs),
- positive-conjugacy sidecar search in [`src/conjugacy.rs`](../src/conjugacy.rs),
- harness and telemetry in [`src/bin/research_harness.rs`](../src/bin/research_harness.rs).

The hard benchmark remains the Brix-Ruiz family, especially `k=3` and above.

## Status Corrections

- `graph-only` mode is already implemented in the main solver.
  `SearchMode` exists, `search_sse_2x2` dispatches to a graph-only path, the
  `search` CLI exposes `--search-mode`, and the harness carries explicit
  `brix_ruiz_k3_graph_only` coverage.

- Matrix-level concrete-shift work is no longer "missing".
  [`src/aligned.rs`](../src/aligned.rs) now contains aligned, balanced, and
  compatible concrete-shift validators plus bounded search, and the main solver
  already tries an aligned concrete-shift fallback on bounded finite-essential
  cases.

- The Brix-Ruiz family is already part of the regression surface.
  [`research/cases.json`](../research/cases.json) includes `brix_ruiz_k3`, a
  wider `k=3` probe, and a `k=4` family probe, and
  [`benches/search.rs`](../benches/search.rs) includes hard-family Criterion
  benches.

- The search is not purely blind factorisation anymore.
  The factorisation layer already includes several structured `3x3`
  conjugation/shear families, and the graph layer already includes same-future,
  same-past, and bounded zig-zag generators. The remaining work is to promote
  the right structured families into the mainline search strategy.

## Graph-Only Follow-Up

Status: partial

What is already done:

- a real search-mode switch exists,
- graph-only search runs through the main solver,
- the harness already compares `mixed` and `graph-only` Brix-Ruiz cases in the
  normal research workflow,
- Lind-Marcus waypoint reproduction exists as a dedicated graph-only tool,
- an ignored regression asserts graph-only search finds the known
  `brix_ruiz_k3` path.

What is still missing:

- decide whether graph-only remains a diagnostic mode or becomes a first-class
  benchmark target.

## Current Priority Order

### 1. Broaden concrete-shift integration in the main search

Status: partial

Already present:

- matrix-level aligned, balanced, and compatible witness verification,
- bounded concrete-shift search for those relations,
- aligned concrete-shift fallback from `search_sse`.

Remaining work:

- decide whether aligned, balanced, or compatible is the easiest mainline
  formulation and standardize around it,
- use more than the aligned relation in the main solver,
- integrate concrete-shift search as either:
  - a direct proof path used more aggressively, or
  - a proposal generator instead of a last-chance fallback only.

### 2. Add explicit structured move families to the main search

Status: partial

Already present:

- graph split/amalgamation moves,
- `3x3` elementary conjugation and several shear families,
- per-move-family telemetry.

Still missing as first-class move families:

- row-splitting,
- column-splitting,
- diagonal refactorization.

Why this still matters:

- the current search still leans too heavily on generic factorisation
  enumeration,
- the papers point toward a more structured constructive move vocabulary than
  the solver currently exposes directly.

### 3. Turn positive conjugacy into a proposal engine

Status: not started in the main solver

Already present:

- [`src/conjugacy.rs`](../src/conjugacy.rs) finds positive conjugacy witnesses
  for the Brix-Ruiz `k=3` and `k=4` cases.

Still missing:

- a proposal API returning candidate intermediate matrices or move suggestions,
- sampling along conjugacy paths to generate candidate row/column splits,
  diagonal refactorizations, or bounded factorisations,
- wiring those proposals into `search_sse` with priority.

### 4. Replace pure BFS ordering with best-first search

Status: not started

The current search chooses which side to expand next, but each side still uses
layer-synchronous FIFO frontiers.

Concrete work:

- add a best-first or A* style frontier,
- compare against FIFO on the research harness,
- keep correctness identical by changing only expansion order.

Good initial ranking signals:

- total entry sum,
- closeness to target under a simple matrix norm,
- fewer distinct row and column types,
- closeness to a positive-conjugacy sample,
- closeness to a concrete-shift witness candidate.

### 5. Strengthen arithmetic filtering for `2x2`

Status: partial

Already present:

- trace,
- determinant,
- Bowen-Franks,
- generalized Bowen-Franks,
- Eilers-Kiming ideal class.

Still missing:

- a second-stage arithmetic screen after the current easy invariants,
- work in the actual quadratic order attached to the pair,
- more necessary conditions involving colon ideals, `xy = lambda^k` style
  equations, and cheap conductor/Picard-style data when available.

Goal:

- rule out more hard non-SSE pairs before expensive search starts.

### 6. Upgrade balanced search from one-step witnesses to short zig-zags

Status: not started

Already present:

- bounded same-size balanced-elementary witness search in
  [`src/balanced.rs`](../src/balanced.rs).

Still missing:

- short balanced zig-zag search,
- caching of common intermediate `S` matrices,
- integration as either:
  - a direct bounded witness search, or
  - a proposal source for ordinary SSE search.

### 7. Refactor graph moves around targeted proposals, not blind widening

Status: partial

Already present:

- same-future in-split generators,
- same-past out-split generators,
- bounded `3x3 -> 2x2 -> 3x3` zig-zag neighbors,
- blind graph-only endpoint and waypoint tools.

Still missing:

- re-center the mainline graph strategy around proposal generation,
  restart waypoints, and canonical probes,
- add higher-block or higher-power refinement moves,
- prioritize refined in-split ideas of the `(I+)` flavor,
- avoid treating blind widening as the default graph roadmap.

### 8. Add iterative deepening over search bounds

Status: not started

Concrete work:

- run small bound schedules first,
- increase `max_entry`, then lag, then intermediate dimension in a controlled
  schedule,
- reuse visited or cached data where safe,
- expose the schedule in the research harness.

This should matter more once move ordering improves.

### 9. Promote memoization and cheaper canonical pre-filtering into the main solver

Status: partial

Already present:

- sidecar graph-only search has an optional in-memory successor cache,
- the main solver already deduplicates expansions and records move-family
  telemetry.

Still missing in the main solver:

- repeated factorisation caching for recurring intermediates,
- a cheap pre-filter before full permutation canonicalization on `3x3`,
- measurement to confirm where the hot loop actually spends time before adding
  more caching complexity.

### 10. Expand the Brix-Ruiz regression discipline

Status: partial

Already present:

- `brix_ruiz_k3` target case,
- wider `k=3` telemetry probe,
- `k=4` family probe,
- Criterion benches centered on the hard family.

Still missing:

- a broader `k=3,4,5,...` benchmark ladder,
- routine logging of:
  - witness found or not,
  - layers expanded,
  - factorisations enumerated,
  - max frontier,
  - which heuristic generated the winning move,
- explicit research-harness comparisons for:
  - FIFO vs best-first,
  - generic factorisation vs structured moves,
  - plain search vs conjugacy-guided proposals,
  - plain search vs concrete-shift sidecars,
  - mixed vs graph-only.

## Lower-Priority Or Deprioritized Work

- Blind widening of split-sidecar searches.
  Current evidence says this should not drive the roadmap.

- Treating module-aligned search as the main aligned program.
  Keep it only as a heuristic sidecar; the concrete-shift matrix-level framing is
  now the more relevant mainline target.

- Expecting one more cheap invariant to settle the hard cases.
  The papers and the local experiments point more toward better witness spaces
  and better search guidance.
