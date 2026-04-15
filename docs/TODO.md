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
