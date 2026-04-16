# SSE Autoresearch Program

This program defines how to run autonomous solver work against the current
`research_harness` surface.

## Scope

Use this workflow for normal solver iteration (`src/search.rs`,
`src/factorisation.rs`, `src/invariants.rs`, and related runtime plumbing).

For harness or corpus schema work (`src/bin/research_harness.rs`,
`research/cases.json`, fixture wiring), open or claim a dedicated `bd` item
first and keep the change set explicitly scoped.

## Ground Rules

- `bd` is the active backlog. Do not track active work in ad hoc markdown TODOs.
- `research/cases.json` is the canonical evaluation corpus.
- Required-case correctness is the hard gate; runtime is secondary.
- Keep `research/log.md` terse; put longer evidence in `research/notes/`.

## Experiment Lanes

Treat harness work as three distinct lanes:

- required correctness lane:
  - required cases are the hard gate;
  - these should stay cheap, stable, and source-backed where possible.
- measurement lane:
  - non-required cases used to compare frontier policy, move-family policy,
    staged search settings, or other scenario-level performance surfaces;
  - use `measurement` blocks when repeat timing matters.
- evidence lane:
  - non-required cases or campaigns that improve bounded understanding of a
    family without belonging in the correctness gate;
  - use this for literature ladders, diagnostic ramps, and other
    decision-support probes.

When a round wants more than graph-only but less than fully mixed search,
prefer `graph_plus_structured` before inventing a new widened baseline.

## Baseline Setup

1. Check backlog and claim/update your issue:
   - `bd ready --json`
   - `bd show <id> --json`
   - `bd update <id> --status in_progress --notes "..." --json`
2. Capture a baseline artifact:
   - `just research-json-save baseline`
3. Run fast correctness checks:
   - `cargo test -q`

## Current Bottlenecks (2026-04 Refresh)

Prioritize work that moves one of these weak spots:

1. `k = 3` shortcut plateau:
   - generic staged shortcut search repeatedly converges to lag `7` on
     `brix_ruiz_k3` when seeded from the normalized guide pool.
   - open objective remains lag `< 7`.
2. Waypoint quality on structured research seams:
   - graph-proposal shortlists now look promising on bounded waypoint probes,
     but sampled positive-conjugacy proposals do not yet survive exact
     invariants as literal intermediate targets.
   - current positive-conjugacy work should be treated as seed or reprojection
     material, not exact waypoint guidance.
3. Search-shape quality under fixed budget:
   - mixed telemetry-focus probes still spend most effort in broad candidate
     generation and pruning churn.
   - the main question is not "can we widen more?" but "can we spend the same
     budget on better-ranked or better-targeted work?"
4. Frontier diagnostics are ahead of frontier defaults:
   - plain beam is still the cheap graph-only control on the hard `k=3` pair.
   - `beam_bfs_handoff` is useful as a measurement surface, but should remain
     research-only until deferred-cap sizing beats plain beam on the same
     bounded control.
5. Missing short structured vocabulary around the `4x4` family region:
   - the remaining short-path gap is structural, not just deeper brute force.
6. Generic square-endpoint parity beyond `2x2`:
   - dynamic square endpoints now get power-trace screening through `trace(M^4)`
     when available, but Goal 4 still lacks fuller same-dimension parity in
     filters, shortcut surfaces, and endpoint handling.

## Harness Scoring Contract

The harness remains lexicographic, with required cases as the correctness gate:

1. Pass all **required** cases.
2. Increase `target_hits`.
3. Increase `total_points`.
4. Improve telemetry-focus progress metrics.
5. Reduce `total_elapsed_ms`.

Never accept a change that regresses required-case correctness for runtime gain.

## Keep Or Revert Policy

Do **not** collapse keep/revert decisions into one scalar beyond the existing
required-case gate. The right question is:

- does this change buy more **useful search per unit budget**?

Read the evidence in three ledgers:

- goal ledger:
  - new witness,
  - lower best lag,
  - broader bounded completion region,
  - new exact positive or negative classification.
- useful-reach ledger:
  - `collisions_with_other_frontier`,
  - `approximate_other_side_hits`,
  - `discovered_nodes`,
  - `guided_segments_improved`,
  - `promoted_guides`,
  - other bounded continuity signals tied to the active round.
- budget ledger:
  - `elapsed_ms`,
  - memory and frontier size,
  - `factorisations_enumerated`,
  - other work counters.

Raw work counters by themselves are **not** success metrics. Fewer candidates,
more pruning, or lower factorisation counts are only good if they preserve or
improve useful reach.

Use this decision order:

1. Hard gate:
   - never keep a change that regresses required-case correctness.
2. Direct project win:
   - keep immediately if it moves a project goal directly.
3. Otherwise judge by round type:
   - throughput round:
     - keep if useful reach stays flat and budget improves.
   - pruning round:
     - keep if useful reach improves, or stays flat while budget improves.
   - widening round:
     - keep if useful reach improves under the same cap, even if raw work rises.
   - ranking or admission round:
     - keep if productive continuity signals improve under the same cap.
4. Revert if the change only improves vanity counters:
   - fewer candidates,
   - more pruning,
   - lower factorisation count,
   - or lower runtime achieved by obviously cutting away productive search.

### Exact Vs Heuristic Pruning

Treat pruning changes differently depending on what they claim.

- exact prune:
  - theorem-backed necessary condition;
  - allowed to reduce raw exploration aggressively;
  - keep it if required positives still pass and fixed-budget runs improve
    overall.
- heuristic prune:
  - ranking guess, admission filter, or "this looks bad" signal;
  - do **not** default it to a hard prune;
  - first use it as a score term, penalty, ordering feature, or shortlist
    preference;
  - only upgrade it to a hard prune after repeated evidence that it does not
    kill productive branches.

When an expensive prune helps only part of the search, gate it narrowly by
dimension, frontier policy, or stage rather than paying its overhead on every
path.

## Benchmark-Style Measurements Through Harness

Decision:

- Scenario-level benchmark probes **should** be expressible through
  `research_harness` so they share the same endpoint fixtures, stage configs,
  and telemetry schema as normal research cases.
- They must be marked as non-gating corpus entries (`"required": false`) to
  avoid distorting correctness gates.

Usage rules for measurement cases:

- set `required` to `false`,
- keep `target_outcome` as `null`,
- keep points neutral (typically `0` across normal outcomes),
- use an optional `measurement` block (`warmup_runs`, `repeat_runs`) only on
  those non-required cases when repeat timing is needed,
- use `deepening_schedule` on non-required cases when the point is to map a
  lag/dimension/entry boundary with one ordered harness surface instead of a
  pile of unrelated one-off commands,
- treat repeated-case `elapsed_ms` / `total_elapsed_ms` as representative median
  timing, not accumulated warmup/repeat wall time,
- use campaign IDs/strategies that clearly label them as measurement probes.

Microbench-level timing of isolated hot loops still belongs in dedicated bench
surfaces, not in harness pass/fail logic.

Keep heavyweight boundary ramps out of the shared canonical corpus unless they
are cheap enough to remain a normal full-corpus run. If the realistic surface
is much heavier, encode the policy in the program/docs and run it from a
dedicated local corpus or explicit worker-case branch instead.

See `docs/research-harness-benchmark-policy.md` for tradeoffs and follow-up.

## Experiment Loop

Repeat:

1. Pick one bottleneck hypothesis.
2. Make one focused code change.
3. Run `cargo test -q`.
4. Run `just research-json-save <stamp>`.
5. Compare against baseline and latest kept artifact.
6. Commit with a descriptive message.
7. Update `bd` notes with evidence and decision.

If a change regresses required-case correctness, revert it promptly.

If a change is neutral or negative on the active bottleneck, prefer reverting
instead of stacking speculative complexity.

## What To Read Before Deep Changes

- `research/README.md`
- `research/cases.json`
- `research/log.md`
- relevant notes in `research/notes/`
- `src/search.rs`
- `src/factorisation.rs`
- `src/invariants.rs`

For frontier-strategy work, include:

- `research/notes/2026-04-13-beam-k3-executor-retune.md`
- `research/notes/2026-04-14-beam-bfs-handoff-graph-only-k3.md`
- `research/notes/2026-04-15-k4-mixed-beam-low-lag-ramp.md`

For waypoint-quality and proposal work, include:

- `research/notes/2026-04-15-graph-move-proposal-slice.md`
- `research/notes/2026-04-15-positive-conjugacy-phase2-usefulness.md`

For shortcut plateau work, include:

- `research/notes/2026-04-12-baker-k3-factor-shape.md`
- `research/notes/2026-04-14-k3-normalized-guide-pool-shortcutting.md`

For literature-driven research refresh work, include:

- `research/notes/2026-04-16-missing-references-and-solver-ideas.md`

## Profiling Guidance

The sandbox does not provide system profilers. Use Rust-level profiling hooks
(`pprof`, `dhat`) only when needed, and remove or gate instrumentation before
finalizing commits.

## Session Close

Before pausing or handing off:

1. ensure `bd` status/notes reflect reality,
2. keep or revert the final experiment commit based on evidence,
3. log results in `research/log.md` and `research/notes/` as needed,
4. keep run artifacts in `research/runs/` (local unless explicitly requested).
