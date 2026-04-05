# SSE Autoresearch

This repo can support autonomous search work, but only through the fixed research harness.

## Setup

1. Create a fresh branch for the run.
   Prefer `autoresearch/<tag>`.
2. Read these files before editing:
   - `README.md`
   - `docs/TODO.md`
   - `research/cases.json`
   - `src/search.rs`
   - `src/factorisation.rs`
   - `src/invariants.rs`
3. Before making any changes, capture the baseline:
   - `just research-json-save baseline`
   - record the baseline result for comparison
4. Verify the baseline harness is green:
   - `just research`
   - `cargo test -q`
5. Create `research/results.tsv` if it does not exist yet.
   Header:
   `commit	required_passes	required_total	target_hits	total_points	focus_score	total_ms	status	description	artifact_path`
6. Leave `research/results.tsv` untracked by git.
   It is a local lab notebook, not repository history.

## Frozen Files

- `research/cases.json` is the ground-truth evaluator unless the human explicitly asks for case work.
- `src/bin/research_harness.rs` is frozen during normal search experiments unless the human explicitly asks for harness work.
- `research/program.md` is also frozen during the loop unless the human explicitly asks for workflow changes.
- Do not change the expected outcomes for cases to make the score look better.
- Do not lower correctness pressure just to manufacture target hits.

## Editable Surface

Start narrow. Prefer changes in:

- `src/search.rs`
- `src/factorisation.rs`
- `src/invariants.rs`

Avoid editing wasm/deploy/docs files during the experiment loop unless the human asks.
Avoid editing the harness or cases during normal solver experiments.

## Objective

The harness score is lexicographic:

1. Pass all required cases in `research/cases.json`.
2. Increase `target_hits`.
3. Increase `total_points`.
4. Increase `telemetry_focus_score` on cases tagged `telemetry-focus`.
5. Reduce `total_ms`.

Never accept a change that breaks a required correctness case just to improve runtime.
Never accept a runtime win that leaves a telemetry-focus case strictly less informative than before.

## Command

Use:

```sh
just research
```

For machine-readable output:

```sh
just research-json
```

To save a machine-readable artifact for an experiment:

```sh
just research-json-save <stamp>
```

This writes `research/runs/<stamp>.json`.

## Experiment Loop

LOOP FOREVER

1. Inspect git state.
2. Identify the current kept commit.
   This is the commit you should return to if the experiment does not help.
3. Make one focused search improvement.
4. Run `cargo test -q`.
5. Run `just research-json-save <stamp>`.
   The `<stamp>` should be stable and sortable, for example `2026-04-05-1530-k3-best-first`.
6. If required cases regress, discard the change by resetting to the last kept commit.
7. If required cases stay green, prefer changes that improve `target_hits`, then `total_points`, then `telemetry_focus_score`, then `total_ms`.
8. Append one row to `research/results.tsv`, including the artifact path.
9. If the experiment improved the score, keep the commit and advance the branch.
10. If the experiment did not improve the score, reset back to the last kept commit.

When `brix_ruiz_k3` is still `unknown`, inspect the proxy telemetry from `just research` or `just research-json`.
Use it to identify whether the current bottleneck is frontier growth, factorisation volume, pruning quality, or collision rate.
In particular, pay attention to:

- `terminal_bottleneck`
- `productive_layers`
- `deepest_productive_layer`
- `first_stagnant_layer`
- the compact per-layer dump for cases tagged `telemetry-focus`

Do not treat raw runtime alone as the optimisation target.
`unknown` is only acceptable for `brix_ruiz_k3`; treat it as a regression for the other cases.
Zero telemetry on some easy cases is expected when they exit through a shortcut or invariant check before BFS.

The idea is that you are a completely autonomous researcher trying things out. If they work, keep. If they don't, discard. And you're advancing the branch so that you can iterate. If you feel like you're getting stuck in some way, you can rewind but you should probably do this very very sparingly (if ever).

NEVER STOP: Once the experiment loop has begun (after the initial setup), do NOT pause to ask the human if you should continue. Do NOT ask "should I keep going?" or "is this a good stopping point?". The human might be asleep, or gone from a computer and expects you to continue working indefinitely until you are manually stopped. You are autonomous. If you run out of ideas, think harder — read papers referenced in the code, re-read the in-scope files for new angles, try combining previous near-misses, try more radical architectural changes. The loop runs until the human interrupts you, period.

## Known Constraints

- `brix_ruiz_k3` is a known-SSE target and currently hard for brute-force search.
- Matrix-level aligned or compatible search is now in scope, but it is larger work than the BFS and telemetry loop.
- Optimisation work should usually focus on the existing BFS and factorisation stack unless the human explicitly changes scope.
- Preferred attack directions: structured move ordering, positive-conjugacy-guided proposals, best-first frontier ordering, factorisation memoisation, and iterative deepening on search bounds.
