# Repo activity summary: 2026-04-17 to 2026-04-18

## Question

What were the highest-signal repo changes over roughly the last 24 hours, and
which of them materially changed current solver direction versus merely adding
diagnostic evidence?

## Scope

This note reviews activity in roughly the window from `2026-04-17 04:15 UTC`
through `2026-04-18 04:15 UTC`, continuing the cadence of
`research/notes/2026-04-17-last-24h-repo-summary.md`.

Primary sources:

- `git log main --since='2026-04-17 04:15 UTC'`
- focused `git log` slices over `src/`, `research/`, and `docs/`
- `research/log.md`
- durable notes added on `2026-04-17`
- `bd list --all --updated-after '2026-04-17T04:15:00Z' --json`
- `bd show sse-rust-2uy.2`
- `bd show sse-rust-2uy.3`
- `bd show sse-rust-2uy.8`
- `bd show sse-rust-2uy.10`

## Highest-signal changes

### 1. The repo promoted stable baseline surfaces before chasing more solver novelty

- The most durable change in this window was not a new witness. It was the
  promotion of graph-only and graph-plus-structured harness baselines into
  explicit first-class controls (`8dcd98e`, `e2a7242`).
- That matters because the repo now has a shared language for:
  - the hard exact `k=3` graph-only control,
  - the cheap graph-only beam probe,
  - the bounded graph-only `k=4` reach ramp,
  - the hard exact `k=3` graph-plus-structured control,
  - and the cheap graph-plus-structured beam probe.
- This materially changed direction. Recent runtime work, campaign notes, and
  keep/revert decisions are now anchored to these retained surfaces instead of
  ad hoc one-off runs.
- The current cheap Goal 3 measurement default is therefore clearer:
  graph-only remains the reusable open-family control, while
  graph-plus-structured is now a measured intermediate lane rather than a vague
  mixed-search subset.

### 2. The explicit structured-family seam stopped being a narrow experiment and became a real main-search layer

- The search-side change with the strongest directional effect was the closure
  of the remaining explicit-family backlog on `sse-rust-2uy.2`.
- Earlier explicit row/column split, diagonal-refactorization, and `5x5 -> 4x4`
  amalgamation work was completed by landing explicit `4x4 -> 3x3` row and
  column amalgamation families (`78394f6`, `678aa65`).
- The durable effect is that `GraphPlusStructured` now has a substantially more
  complete explicit `3x3/4x4/5x5` corridor, with first-class family labels and
  telemetry, instead of depending so heavily on broad rectangular or square
  factorisation sweeps.
- This also changed how current reach evidence should be read:
  - on the open Brix-Ruiz `k=4` pair, graph-plus-structured still does not
    produce a witness, but a widened `dim4` beam exposes real bounded overlap
    signal while `dim5` still collapses into factorisation volume;
  - on the Riedel/Baker ladder, graph-plus-structured is no longer just
    "cheaper mixed". It now reaches a real but timeout-tight `k = 16` edge
    where the bounded mixed control times out.
- So the repo's search direction shifted further toward bounded explicit
  structure, not back toward broader fully mixed widening.

### 3. Runtime work stayed disciplined: a few low-level keeps survived, but several tempting follow-ups were explicitly rejected

- The kept runtime wins were narrow and evidence-backed:
  - graph-plus-structured `3x3` core adjugate/determinant gating
    (`0265697`);
  - graph-only successor dedup hashing (`609ab4c`);
  - graph-only `5x5` canonical-permutation pruning (`136cf28`).
- Those changes matter because they improved the retained baseline surfaces
  without changing witness lag or search counters. That gave the repo more room
  to extend the graph-only `k=4` lag map and to treat graph-plus-structured
  exact search as a repeatable performance control.
- But the window was equally important for what it rejected:
  - compact graph-only "canonical-only" replay handles were ruled out as
    incompatible with exact edge replay and observer contracts;
  - broadening deferred witness reconstruction as a generic strategy did not
    earn a keep;
  - only a tightly measured exact graph-plus-structured BFS deferred-parent
    slice survived, and even that remained scoped as a local keep rather than a
    template for mixed or dynamic search.
- The durable change in repo behavior is therefore methodological: runtime work
  now has to clear retained baseline gates and a real maintenance-cost test,
  not just a local hotspot argument.

### 4. Exact family-specific gates gained ground, while coarse quotient/support ideas were pushed back into diagnostic status

- Another important direction shift came from the exact-pruning work.
- The repo now has or explicitly favors exact family-local gates such as:
  - admissible-diagonal emptiness for diagonal refactorization (`0f92107`);
  - exact row/column mass gates for bounded `3x3 -> 4x4` split families
    (`290946b`);
  - bounded no-go certificates as a real harness-side reporting surface
    (`638d92d`).
- In the same window, the obvious coarser alternatives were explicitly
  weakened:
  - support-profile and duplicate-row summaries were shown to be too weak as
    hard `square_factorisation_3x3` gates;
  - coarse same-future/past and partition-refined quotient signatures also
    failed as exact source-only emptiness certificates on singular `3x3`
    sources.
- This materially changed current direction. The repo is now more clearly
  oriented toward exact bounded family certificates and orbit reduction, not
  toward promoting quotient/support summaries into hard pruning.

### 5. Reporting, ranking, and endpoint parity all became more durable, and two long-running infrastructure beads effectively moved off the critical path

- The main reporting change was that witness-manifest-backed ranking analysis
  became active rather than passive (`c349118`, `753c005`, `7ce1630`,
  `e95b1fb`, `bb42e61`, `a452e39`).
- The important durable consequence is that held-out ranking work is now tied
  to validated manifest endpoints and benchmark roles, with ambiguity around
  shared canonical endpoint pairs handled explicitly rather than silently.
- Analyzer-side move-family overrides (`cf9c71b`) and the refreshed non-Brix
  mixed held-out layer-contrast artifact (`e4c1d25`, `d2fd32d`, `6d8d2e0`)
  also matter because all three held-out non-Brix families are now rankable on
  the retained artifact surface.
- The harness deepening slice (`9cdf857`, `3dcddeb`) is similarly important,
  but in a limited way: deepening schedules are now first-class reporting and
  summary objects, not yet an execution strategy with reuse or stop-on-success.
- The generic square endpoint path also advanced materially:
  same-dimension square search now applies a Bowen-Franks reject beyond the
  earlier power-trace gate (`5f830e4`, plus the arithmetic hardening follow-up).
- Together those changes closed or effectively downgraded two infrastructure
  blockers:
  - `sse-rust-2uy.8` moved from "missing deepening surface" to "reporting
    present, execution reuse still open";
  - `sse-rust-2uy.10` moved from "generic square path missing obvious parity
    checks" to "generic square path improved, but still behind the specialized
    `2x2` lane."

## Direction-changing keeps vs. diagnostic or reverted work

- Kept and direction-changing:
  explicit graph-only and graph-plus-structured baseline surfaces; completion
  of the explicit structured-family corridor; narrowly accepted runtime wins on
  retained controls; exact family-local gating over coarse quotient summaries;
  manifest-backed held-out ranking and generic Bowen-Franks parity.
- Kept but still diagnostic:
  the widened graph-only `k=4` lag map; graph-plus-structured `dim4` broad-beam
  overlap growth; Riedel/Baker reach extension; bounded no-go certificates;
  observer-emission probes.
- Explicitly not promoted:
  graph-proposal shortlist success beyond the original one seam; compact
  representative retention in graph-only BFS; quotient/support signatures as
  exact `square_factorisation_3x3` gates; generic deferred-witness widening
  outside the tightly measured exact graph-plus-structured seam.

## Active seams and priorities now

- `sse-rust-2uy.2` is closed, which changes the main search backlog. The live
  question is no longer "add the missing explicit family labels" but "how much
  useful reach does the new explicit structured middle layer actually buy on
  hard open families?"
- `sse-rust-2uy.3` remains open, but the active seam narrowed again. Richer
  exact seed families, invariant-aware reprojection, and anchor-aware residual
  arithmetic all stayed negative on the bounded evidence cases. The next step
  is therefore richer residual-difficulty scoring on the surviving exact local
  seed family, still as a sidecar, not default search behavior.
- `sse-rust-2uy.8` is no longer an "add any deepening surface" task. The
  remaining priority is execution-side scheduling or reuse, because the summary
  and reporting surface now exists.
- `sse-rust-2uy.10` also changed shape. Generic square search now has power
  traces plus Bowen-Franks as same-dimension rejects, so the remaining parity
  gap is about which specialized `2x2` shortcuts still deserve generic analogs.

## Conclusion

The biggest durable change over this window was the repo becoming much more
explicit about its middle layer.

It now has:

- retained graph-only and graph-plus-structured baselines,
- a substantially completed explicit structured-family corridor,
- sharper family-local exact gates,
- more durable held-out ranking/reporting surfaces,
- and a stronger generic square reject path.

What it did **not** get was a new Goal 2 or Goal 3 witness, or a proposal side
channel ready to enter default search.

So the repo ends this day pointed toward a clearer bounded strategy:

- measure against retained baselines,
- keep explicit structured search as the main non-graph widening lane,
- prefer exact family-local certificates over coarse quotient summaries,
- and treat sampled positive-conjugacy and other proposal probes as diagnostic
  until they beat simple controls on the bounded cases that matter.
