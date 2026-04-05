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
   - `just research-json`
   - record the baseline result for comparison
4. Verify the baseline harness is green:
   - `just research`
   - `cargo test -q`
5. Create `research/results.tsv` if it does not exist yet.
   Header:
   `commit	required_passes	required_total	target_hits	total_points	total_ms	status	description`

## Frozen Files

- `research/cases.json` is the ground-truth evaluator.
- Do not modify `research_harness.rs` during experiments unless the human explicitly asks for harness work.
- Do not change the expected outcomes for cases to make the score look better.

## Editable Surface

Start narrow. Prefer changes in:

- `src/search.rs`
- `src/factorisation.rs`
- `src/invariants.rs`

Avoid editing wasm/deploy/docs files during the experiment loop unless the human asks.

## Objective

The harness score is lexicographic:

1. Pass all required cases in `research/cases.json`.
2. Increase `target_hits`.
3. Increase `total_points`.
4. Reduce `total_ms`.

Never accept a change that breaks a required correctness case just to improve runtime.

## Command

Use:

```sh
just research
```

For machine-readable output:

```sh
just research-json
```

## Experiment Loop

LOOP FOREVER

1. Inspect git state.
2. Make one focused search improvement.
3. Run `cargo test -q`.
4. Run `just research-json`.
5. If required cases regress, discard the change.
6. If required cases stay green, prefer changes that improve `target_hits`, then `total_points`, then `total_ms`.
7. Append one row to `research/results.tsv`.

When `brix_ruiz_k3` is still `unknown`, inspect the proxy telemetry from `just research` or `just research-json`.
Use it to identify whether the current bottleneck is frontier growth, factorisation volume, pruning quality, or collision rate.
Do not treat raw runtime alone as the optimisation target.
`unknown` is only acceptable for `brix_ruiz_k3`; treat it as a regression for the other cases.
Zero telemetry on some easy cases is expected when they exit through a shortcut or invariant check before BFS.

Make sure to commit on changes that you want to keep. 

The idea is that you are a completely autonomous researcher trying things out. If they work, keep. If they don't, discard. And you're advancing the branch so that you can iterate. If you feel like you're getting stuck in some way, you can rewind but you should probably do this very very sparingly (if ever).

NEVER STOP: Once the experiment loop has begun (after the initial setup), do NOT pause to ask the human if you should continue. Do NOT ask "should I keep going?" or "is this a good stopping point?". The human might be asleep, or gone from a computer and expects you to continue working indefinitely until you are manually stopped. You are autonomous. If you run out of ideas, think harder — read papers referenced in the code, re-read the in-scope files for new angles, try combining previous near-misses, try more radical architectural changes. The loop runs until the human interrupts you, period.

## Known Constraints

- `brix_ruiz_k3` is a known-SSE target and currently hard for brute-force search.
- Matrix-level aligned shift equivalence is blocked on a missing primary source.
  Do not invent that definition.
- Optimisation work should focus on the existing BFS and factorisation stack unless the human explicitly changes scope.
- Preferred attack directions: factorisation memoisation, best-first or heuristic frontier ordering, and iterative deepening on search bounds.
