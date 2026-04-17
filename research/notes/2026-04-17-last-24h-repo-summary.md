# Repo activity summary: 2026-04-16 to 2026-04-17

## Question

What were the highest-signal repo changes over roughly the last 24 hours, and
which of them materially changed the current solver direction?

## Scope

This note reviews activity in roughly the window from `2026-04-16 04:15 UTC`
through `2026-04-17 04:15 UTC`, continuing the cadence of
`research/notes/2026-04-16-last-24h-repo-summary.md`.

Primary sources:

- `git log main --since='2026-04-16 04:15 UTC'`
- focused `git log` slices over `src/`, `research/`, and `docs/`
- `research/log.md`
- recent durable notes in `research/notes/`
- `bd list --all --updated-after '2026-04-16T04:15:00Z' --json`
- `bd ready --json`
- `bd show sse-rust-2uy.2`
- `bd show sse-rust-2uy.3`
- `bd show sse-rust-2uy.8`
- `bd show sse-rust-2uy.10`

## Highest-signal changes

### 1. The repo's operating discipline was tightened before more solver work

- The window opened with a documentation and program refresh rather than an
  immediate search rewrite (`1fe9cfc`, `a9af373`, `4f90e86`, `81dc77b`,
  `f6f7c6e`).
- The durable effect is that the repo now speaks more plainly about keep vs
  revert decisions, separates required, measurement, and evidence lanes more
  clearly, and treats some previously optimistic surfaces as diagnostic-only.
- In particular, the measurement corpus audit and follow-up program guidance
  make two conclusions explicit:
  - plain graph-only beam remains the cheap `k=3` control,
  - `beam_bfs_handoff` is still mainly a losing baseline to measure against,
    not a frontier strategy that has earned more default emphasis.

### 2. Ranking and research instrumentation improved, but mostly as analysis seams

- Triangle-path quotient telemetry became a real reporting surface
  (`766b175`, `9b94746`, `65a3a19`, `78df1e4`). The important result is that
  quotient normalization materially compresses the current `k=3` guide pool,
  but the quotient-retained pool was neutral-to-slightly-worse in bounded
  shortcut-search A/B, so it remains research-only rather than a promoted
  default input.
- Graph-proposal shortlist rounds were also operationalized more explicitly
  (`7fc78d4`), which matters because the repo now has a durable way to keep
  waypoint-local proposal probes bounded and visible without pretending they
  are already stable harness baselines.
- The repo now has a more deliberate ranking-evaluation scaffold
  (`05fed09`, `7c95a8a`, `a7d28f5`, `5b7ed6f`, `5fe440d`, `c07783b`,
  `33392b6`, `7ec3256`). The key durable shift is that dynamic mixed endpoint
  search now emits observer layer events, which turned at least one held-out
  non-Brix family slice (`lind_marcus_a_to_c`) from unrankable to rankable.
- Partition-refined quotient scoring and shortlist reordering also landed
  (`aed841c`, `dc087a8`), but the repo's current conclusion is still cautious:
  they are useful bounded proposal-analysis tools, not evidence for changing
  default beam ranking or promoting refined shortlists into the main harness.

### 3. Runtime work stayed disciplined: two clear keeps, one explicit revert

- Two runtime slices landed with genuine direct-control wins:
  `4x4` cofactor unrolling (`716f78c`) and `3x3` adjugate reuse in structured
  sparse loops (`2d76902`).
- Those wins mattered because they reduced heavy structured-family hot spots
  without changing required outcomes, and the follow-up harness checks were at
  least flat-to-slightly-positive rather than merely locally faster.
- The next low-level round then documented a real reject decision
  (`c70d511`): a square-family prepared `2x3` solver path and a telemetry-map
  fast path both helped targeted probes but failed the aggregate keep gate.
- One telemetry follow-up was kept (`ed7124c`): hot move-family accumulation
  now uses borrowed family labels internally before converting back to the
  public map shape.

### 4. Arithmetic and balanced sidecars became clearer, mostly in diagnostic ways

- The `2x2` arithmetic dossier became more exact and better classified
  (`91b576a`, `b98530e`, `7fb7248`, `741a51c`, `9d17a99`, `c95d062`,
  `fc9a6d5`, `8c57e41`).
- The important durable conclusion is that narrow `2x2` arithmetic remains
  valuable for exact classification and reporting, but the deferred
  ideal-quotient follow-up did not justify a new hard rejection, and the repo
  is now explicitly separating exact pairwise classification from default
  ranking signals.
- The balanced sidecar lane grew substantially (`3960f71`, `6dd227d`,
  `bc39307`, `0bda4a4`, `47462de`, `f2032dc`, `12bbacf`, `1164d13`), adding
  same-size neighbor, bridge-return, and insplit-return seams while preserving
  concrete bridges in the return path.
- This is still mostly evidence work rather than a solver breakthrough: the
  day made the balanced reachability picture richer and less lossy, but it did
  not produce a new Goal 2 or Goal 3 witness.

### 5. The biggest search-side change was a broad explicit structured-family expansion

- `GraphPlusStructured` and the factorisation-family seam gained a much larger
  explicit vocabulary over the window:
  - bounded `3x3 -> 4x4` row split (`0117d22`),
  - bounded `3x3 -> 4x4` column split (`bdade12`),
  - bounded same-size `4x4` diagonal refactorization (`b19bca0`),
  - bounded `4x4 -> 5x5` row split (`3ee3329`),
  - bounded `4x4 -> 5x5` column split (`e8f315a`),
  - bounded `5x5 -> 4x4` row amalgamation (`51e1b65`).
- The durable effect is not a new witness; it is that the main search now has
  first-class, telemetry-visible structured families across the current
  `3x3/4x4/5x5` corridor instead of relying as heavily on broad sparse
  rectangular enumeration.
- This materially advances `sse-rust-2uy.2`: the explicit-family backlog is no
  longer just a narrow diagonal `3x3` seam. It is becoming a real middle layer
  between graph-only moves and the most expensive mixed search surfaces.

## Kept vs. reverted conclusions that changed direction

- Kept: quotient telemetry and quotient-retained guide artifacts as reporting
  and preparation tools.
- Not kept as default behavior: quotient-retained guide pools for bounded
  shortcut search.
- Kept: `4x4` cofactor unrolling, `3x3` adjugate reuse, and borrowed-label
  move-family telemetry accumulation.
- Reverted: the next prepared-solver and telemetry fast-path round after those
  wins, because aggregate harness results did not clear the keep gate.
- Kept as evidence only: the balanced return-family expansion, the ideal-
  quotient follow-up, and the partition-refined quotient proposal work.
- Kept as a losing baseline rather than an optimization target:
  `beam_bfs_handoff` around the current graph-only `k=3` control.

## Current direction and open seams

- The mainline direction now looks more like "better bounded structure plus
  better measurement/ranking surfaces" than "wider generic frontier search."
- `sse-rust-2uy.2` remains the clearest active search seam: the explicit
  structured-family lane is now substantial, but it is still incomplete.
- `sse-rust-2uy.3` remains open, but the day's evidence pushes it toward local
  seed or reprojection experiments rather than literal exact waypoint targets.
- `sse-rust-2uy.8` is still only partially solved: deepening exists in the
  harness/corpus surface, but not yet as a first-class execution strategy.
- `sse-rust-2uy.10` is still open even after the recent generic square
  improvements. Dynamic square endpoint search is better instrumented and
  better filtered than before, but still materially behind the specialized
  `2x2` path.

## Conclusion

The last day did not produce a new shortest `k=3` path or a new `k=4` witness.
Its biggest durable effects were:

- a stricter keep/revert and measurement discipline,
- better ranking and observer surfaces for non-Brix evidence,
- sharper arithmetic and balanced-sidecar diagnostics,
- and a broad explicit structured-family expansion through the main search.

That leaves the repo pointed toward bounded, telemetry-visible structure and
better proposal-quality evidence, not toward another broad generic widening
pass.
