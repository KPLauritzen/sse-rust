# Search Improvements

This file tracks outstanding search work only. Completed items have been pruned.

The current solver stack is:

- bidirectional BFS in [`src/search.rs`](../src/search.rs),
- factorisation enumeration in [`src/factorisation.rs`](../src/factorisation.rs),
- invariant filtering in [`src/invariants.rs`](../src/invariants.rs),
- sidecar experiments in [`src/aligned.rs`](../src/aligned.rs), [`src/balanced.rs`](../src/balanced.rs), [`src/conjugacy.rs`](../src/conjugacy.rs), and [`src/graph_moves.rs`](../src/graph_moves.rs).

The hard benchmark remains the Brix-Ruiz family, especially `k=3` and above.

## Current Priority Order

### 1. Implement matrix-level aligned or compatible search

This is now the highest-priority missing search substrate.

Why:

- the old "aligned work is blocked on a missing definition" status is no longer true
- Bilich-Dor-On-Ruiz defines matrix-level aligned, balanced, and compatible shift equivalence for finite essential matrices
- in that setting, these notions coincide with SSE
- the repo currently only has the older module-level aligned sidecar in [`src/aligned.rs`](../src/aligned.rs)

Concrete work:

- add matrix-level witness types and validators
- implement fixed-lag bounded search for at least one of:
  - aligned shift equivalence
  - compatible shift equivalence
  - balanced shift equivalence
- integrate the bounded solver into `search_sse` either as:
  - a direct proof path, or
  - a high-priority move generator

### 2. Add structured move families to the main search

The current solver is still too close to generic factorisation BFS.

Highest-value missing move families:

- row-splitting
- column-splitting
- diagonal refactorization

Why:

- Boyle-Kim-Roush repeatedly reduce constructive SSE arguments to these move types
- they are more structured than blind `UV/VU` enumeration
- they match the current evidence that search guidance matters more than just raising bounds

Concrete work:

- encode these as explicit expansions in [`src/search.rs`](../src/search.rs)
- canonicalize their outputs aggressively
- record per-move-family telemetry to see which ones actually help on Brix-Ruiz cases

### 3. Turn positive conjugacy into a proposal engine

[`src/conjugacy.rs`](../src/conjugacy.rs) already finds simple positive conjugators for the Brix-Ruiz `k=3` and `k=4` cases. That should stop being a standalone experiment.

Concrete work:

- expose a proposal API returning candidate intermediate matrices or move suggestions
- sample along positive-conjugacy paths
- convert sampled matrices into candidate:
  - row splits
  - column splits
  - diagonal refactorizations
  - bounded factorizations
- feed those candidates into the main search with priority

### 4. Replace pure BFS ordering with best-first search

The current search is bidirectional, but still FIFO within each frontier.

Concrete work:

- add a best-first or A* style frontier
- compare against FIFO on the research harness
- keep correctness identical by changing only expansion order

Good initial ranking signals:

- total entry sum
- closeness to target under a simple matrix norm
- fewer distinct row and column types
- closeness to a positive-conjugacy sample
- closeness to an aligned or compatible witness candidate

### 5. Strengthen arithmetic filtering for `2x2`

[`src/invariants.rs`](../src/invariants.rs) already uses a useful slice of Eilers-Kiming, but not the full arithmetic picture.

Concrete work:

- add a second-stage arithmetic screen after the current easy invariants
- work in the actual quadratic order attached to the pair
- test more necessary conditions coming from:
  - colon ideals
  - equations of the form `xy = lambda^k`
  - conductor or Picard-group data when cheap enough

Goal:

- rule out more hard non-SSE pairs before launching expensive search

### 6. Upgrade balanced search from one-step witnesses to short zig-zags

[`src/balanced.rs`](../src/balanced.rs) currently searches for one balanced elementary step. That is too narrow.

Concrete work:

- search for short balanced zig-zags
- cache common intermediate `S` matrices
- allow balanced search to act as:
  - a direct bounded witness search
  - a proposal source for ordinary SSE search

Why:

- balanced search is now more important after the matrix-level equivalence results
- but same-size one-step balanced search should not remain a standalone dead-end experiment

### 7. Refactor graph moves around targeted proposals, not blind widening

The local sidecar evidence says blind split expansion is not the mainline strategy.

What to keep using graph moves for:

- proposal generation
- canonical probes
- restart waypoints
- component detection

Concrete work:

- refocus [`src/graph_moves.rs`](../src/graph_moves.rs) around targeted move families from the papers
- prioritize refined in-split ideas of the `(I+)` flavor
- add higher-block or higher-power refinement moves
- treat complete in-splits or dual graphs as bounded probes, not a separate widening universe

### 8. Add iterative deepening over search bounds

We still need a better schedule for `max_entry`, `max_lag`, and `max_intermediate_dim`.

Concrete work:

- run small bound schedules first, for example:
  - smaller `max_entry`
  - then larger `max_entry`
  - then larger intermediate dimension
- reuse visited or cached data where safe
- expose the schedule in the research harness

This should work especially well once the main search has better move ordering.

### 9. Add memoization and cheaper canonical pre-hashing

These are still worth doing, but they are below the search-structure changes above.

Concrete work:

- cache repeated factorisation work for recurring intermediate matrices
- add a cheap pre-hash before full permutation canonicalization on `3x3`
- measure whether the hot loop is actually dominated by factorisation or canonicalization before spending too much time here

### 10. Make the Brix-Ruiz family the default regression target

The repo needs a more explicit benchmark discipline.

Concrete work:

- add or expand benchmark cases for `k=3,4,5,...`
- log, for each run:
  - witness found or not
  - layers expanded
  - factorisations enumerated
  - max frontier
  - which heuristic generated the winning move
- use the research harness to compare:
  - FIFO vs best-first
  - generic factorisation vs structured moves
  - plain search vs conjugacy-guided proposals
  - plain search vs aligned or compatible sidecars

## Lower-Priority Or Deprioritized Work

- Blind widening of split-sidecar searches.
  Current evidence says this should not drive the roadmap.

- Treating module-aligned search as the main aligned program.
  Keep it only as a heuristic sidecar until matrix-level aligned or compatible search lands.

- Expecting one more cheap invariant to settle the hard cases.
  The papers point toward better witness spaces and better search guidance instead.
