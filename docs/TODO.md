# Search Improvements

This file is intentionally non-authoritative for outstanding work.

Use `bd` as the source of truth for anything actionable:

- `bd prime`
- `bd ready`
- `bd show <id>`
- `bd list --status open`

This document only keeps durable context that is useful when reading or
re-orienting the search code.

## Current Solver Stack

The current solver stack is:

- bidirectional endpoint search in [`src/search.rs`](../src/search.rs),
- factorisation enumeration in [`src/factorisation.rs`](../src/factorisation.rs),
- invariant filtering in [`src/invariants.rs`](../src/invariants.rs),
- graph-move search in [`src/graph_moves.rs`](../src/graph_moves.rs),
- concrete-shift witness search in [`src/aligned.rs`](../src/aligned.rs),
- balanced-elementary sidecar search in [`src/balanced.rs`](../src/balanced.rs),
- positive-conjugacy sidecar search in [`src/conjugacy.rs`](../src/conjugacy.rs),
- research harness and telemetry in [`src/bin/research_harness.rs`](../src/bin/research_harness.rs).

## Current Search Reality

These points are easy to get wrong when reading older notes:

- `graph-only` is already implemented in the main solver and should be treated
  as an existing search mode, not a missing feature.
- matrix-level concrete-shift search is already implemented for bounded `2x2`
  aligned / balanced / compatible relations and is wired as a bounded fallback
  in the main solver.
- the search is no longer purely blind factorisation: structured graph moves,
  `3x3` conjugation/shear families, guided refinement, and shortcut search are
  all already present.
- the hard empirical surface is still the Brix-Ruiz family, especially `k=3`
  and above.

## Durable Research Themes

The repo has repeatedly pointed toward a few broad themes. Treat these as
orientation, not as a checklist.

- Better witness and proposal generation matters more than adding another blind
  widening pass.
- Structured move families are more promising than leaning harder on generic
  factorisation enumeration.
- Guided refinement and shortcut search are important because they let us use
  proposals and waypoints without paying for the full mixed frontier.
- Benchmarking should stay centered on the hard Brix-Ruiz family and should
  distinguish graph-only costs from mixed structured-search costs.
- Arithmetic filtering is still useful support work, but it is not the main
  route to progress on the hardest positive cases.

## Tracking Rule

If a work item has a status, priority, owner, or next action, it belongs in
`bd`, not in this file.
