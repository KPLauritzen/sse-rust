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
2. Frontier growth on hard probes:
   - mixed `k=3`/`k=4` telemetry-focus probes still spend most effort in broad
     candidate generation and pruning churn.
3. Frontier strategy quality:
   - beam and beam-to-BFS-handoff are now implemented surfaces, but ranking and
     handoff policies are still unstable on hard graph-only controls.
4. Missing short structured vocabulary around the `4x4` family region:
   - the remaining short-path gap is structural, not just deeper brute force.

## Harness Scoring Contract

The harness remains lexicographic, with required cases as the correctness gate:

1. Pass all **required** cases.
2. Increase `target_hits`.
3. Increase `total_points`.
4. Improve telemetry-focus progress metrics.
5. Reduce `total_elapsed_ms`.

Never accept a change that regresses required-case correctness for runtime gain.

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
- use campaign IDs/strategies that clearly label them as measurement probes.

Microbench-level timing of isolated hot loops still belongs in dedicated bench
surfaces, not in harness pass/fail logic.

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

For shortcut plateau work, include:

- `research/notes/2026-04-12-baker-k3-factor-shape.md`
- `research/notes/2026-04-14-k3-normalized-guide-pool-shortcutting.md`

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
