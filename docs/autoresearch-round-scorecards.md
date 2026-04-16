# Autoresearch Round Scorecards

## Status

Use this note as the default useful-reach and budget template for common
autoresearch round types.

- It complements [`../research/program.md`](../research/program.md).
- If a round already has a dedicated contract, use that first. Current example:
  [`graph-proposal-shortlist-rounds.md`](graph-proposal-shortlist-rounds.md).
- Do **not** collapse keep/revert decisions into one scalar. Read the goal
  ledger first, then the round scorecard.

## Round Card Template

Before running a new round family, record these fields in the `bd` issue or the
round note:

- round type and lane;
- fixed bound or control being held constant;
- useful-reach fields;
- budget fields;
- keep threshold;
- revert threshold;
- vanity counters to ignore.

A direct project win still outranks the scorecard:

- new witness;
- lower best lag;
- broader bounded completion region;
- new exact positive or negative classification.

## Default Scorecards

### Throughput / Control Rounds

Use when the point is "same search shape, cheaper execution" rather than a new
search surface.

Useful reach:

- `collisions_with_other_frontier`
- `approximate_other_side_hits`
- `discovered_nodes`
- `guided_segments_improved` / `shortcut_search.promoted_guides` when the
  control is guided

Budget:

- `elapsed_ms`
- `frontier_nodes_expanded`
- `total_visited_nodes`
- `factorisations_enumerated`
- `max_frontier_size`

Keep when useful reach stays flat and budget improves.

Revert when runtime falls only because the round explores less productive
search.

### Pruning Rounds

Use when adding a new theorem-backed prune, invariant gate, or candidate
filter.

Useful reach:

- direct goal-ledger wins
- `collisions_with_other_frontier`
- `approximate_other_side_hits`
- `discovered_nodes`
- `guided_segments_improved` / `shortcut_search.promoted_guides` on guided
  surfaces

Budget:

- `elapsed_ms`
- `frontier_nodes_expanded`
- `total_visited_nodes`
- `factorisations_enumerated`
- `candidates_after_pruning`
- `pruned_by_size` / `pruned_by_spectrum` as diagnosis only

Keep when useful reach improves, or stays flat while budget drops.

Revert when the only win is higher prune rate, fewer candidates, or lower
factorisation count.

If the prune is heuristic rather than exact, score it as a ranking or admission
round first.

### Widening Rounds

Use when broadening move families, dimension caps, frontier shape, or admission
width while holding the outer cap fixed.

Useful reach:

- direct goal-ledger wins
- `collisions_with_other_frontier`
- `approximate_other_side_hits`
- `discovered_nodes`
- `guided_segments_improved` / `shortcut_search.promoted_guides` if widening a
  guided stage
- broader bounded completion region when the round is explicitly capped

Budget:

- `elapsed_ms`
- `frontier_nodes_expanded`
- `total_visited_nodes`
- `factorisations_enumerated`
- `max_frontier_size`

Keep when useful reach improves under the same cap, even if raw work rises.

Revert when budget rises without new useful reach, or when any apparent win
depends on silently loosening the cap.

If the main question is "where is the new cliff?", use the boundary-mapping
scorecard instead.

### Ranking / Proposal Probe Rounds

Use when reordering candidate admission, scoring guides, or evaluating bounded
proposal shortlists.

Useful reach:

- better top-ranked candidates rather than more candidates
- tighter or lexicographically better shortlist signals such as
  `best target signature gap` or `best-gap shortlist`
- invariant-compatible or realizable top proposals at the same bound
- `guided_segments_improved`
- `shortcut_search.promoted_guides`
- `collisions_with_other_frontier` / `approximate_other_side_hits` when the
  ranking is inside main search rather than an offline probe

Budget:

- `elapsed_ms`
- shortlisted or probed candidate count, such as `probed proposals`
- `frontier_nodes_expanded` or per-probe `frontier_nodes`
- `total_visited_nodes` / `visited`
- `factorisations_enumerated`

Keep when productive continuity improves under the same shortlist cap or search
bound.

Revert when only raw proposal volume, ranking churn, or blind-overlap counts
move without better gap, realizability, or downstream guided progress.

### Guided / Shortcut Rounds

Use when tuning `guided_refinement`, `shortcut_search`, guide-pool admission,
or segment budgets.

Useful reach:

- lower best lag
- `guided_segments_improved`
- `shortcut_search.segment_improvements`
- `shortcut_search.promoted_guides`
- `shortcut_search.best_lag_end` versus `shortcut_search.best_lag_start`

Budget:

- `elapsed_ms`
- `shortcut_search.segment_attempts`
- `frontier_nodes_expanded`
- `total_visited_nodes`
- `factorisations_enumerated`
- `max_frontier_size`

Keep when lag falls, or when the same lag yields more improved segments or
promoted guides at equal or lower budget.

Revert when the round spends more segment attempts, guides, or rounds without
moving lag or productive segment counts.

Treat `rounds_completed`, `guide_artifacts_loaded`, and cache counters as
diagnostics, not success metrics by themselves.

### Boundary-Mapping Rounds

Use when the deliverable is a reproducible lag, entry, dimension, or timeout
cliff, usually through `deepening_schedule` or an ordered explicit ramp.

Useful reach:

- broader bounded completion region
- a sharper reproducible boundary such as "completes through `lag8`, times out
  by `lag9`"
- new exact positive or negative classification at a schedule step
- monotone schedule evidence that survives rerun on the same worker or binary

Budget:

- per-step `elapsed_ms`
- timeout rate
- `factorisations_enumerated`
- `total_visited_nodes`
- `max_frontier_size`

Keep as evidence when the boundary itself moves outward under the same timeout
or cap, or when the same boundary becomes materially cheaper without shrinking
the completion region.

Do not claim a keepable solver win when only one interior point got faster but
the boundary stayed flat.

If the map is heavy, keep it in a dedicated local corpus or round note rather
than the shared default corpus.

## Citation Rule

When opening a new autoresearch round, cite this note plus any dedicated round
note, then state which useful-reach and budget fields are authoritative for
that run.

If a round needs a different scorecard, define the exception explicitly before
using it.
