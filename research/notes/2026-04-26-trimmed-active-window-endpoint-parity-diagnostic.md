# Trimmed active-window endpoint parity diagnostic for square frontier states (2026-04-26)

## Question

For bead `sse-rust-w7e4`, take the narrow-kept
`trimmed_active_window` descriptor from `sse-rust-1j1` and test whether it
helps **Goal 4 endpoint-agnostic parity diagnostics** on square `3x3` / `4x4`
frontier states, without changing production canonicalization or search policy.

This slice stays bounded:

- no production canonicalization rewrite;
- no beam or move-family retune;
- no broad search;
- no new solver successor family; and
- no claim that the descriptor is an SSE invariant.

## Diagnostic Surface

I extended the existing research-only helper:

- `src/bin/diagnose_endpoint_neighborhood_normal_forms.rs`

The helper still computes the three local descriptors from `sse-rust-1j1`, but
it now also emits a **paired parity report** for square endpoint-near samples.

For each paired comparison it records:

1. the coarse approximate signature already used in `src/search.rs`
   (`mass_support_signature`);
2. the trimmed active-window descriptor after `canonical_perm()`;
3. whether the two sides match on the coarse bucket;
4. whether they also match on the trimmed active window; and
5. the recommended future-layer action.

The helper does not touch solver behavior. It only writes JSON diagnostics.

## Sample Set

The reproducible report uses `27` samples:

- `11` endpoint-near `k = 3` witness/replay states from:
  - `research/guide_artifacts/k3_shortcut_round1.json`
  - `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`
- `16` retained `k = 4` stuck/counterpart states from the top `8` approximate
  hits in a fresh extractor replay.

Dimension split:

- `4` states of dimension `3`
- `23` states of dimension `4`

The new paired parity section produces `12` direct forward/backward comparisons:

- `4` `k = 3` witness/replay overlap pairs
- `8` retained `k = 4` stuck-vs-counterpart pairs

## Reproducible Commands

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms
timeout -k 20s 180s cargo run --features research-tools --bin extract_brix_ruiz_k4_stuck_states -- \
  --json-out tmp/sse-rust-w7e4-k4-stuck-top16.json \
  --top 16
timeout -k 20s 180s cargo run --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms -- \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --stuck-report tmp/sse-rust-w7e4-k4-stuck-top16.json \
  --endpoint-radius 3 \
  --top-stuck 8 \
  --json-out tmp/sse-rust-w7e4-trimmed-active-window-parity.json
```

Artifacts:

- stuck report: `tmp/sse-rust-w7e4-k4-stuck-top16.json`
- parity diagnostic: `tmp/sse-rust-w7e4-trimmed-active-window-parity.json`

## Observed Parity Signals

### Pair summary

The `12` paired comparisons split cleanly:

| Pair kind | Count | Coarse bucket match | Trimmed active-window match | Recommended action |
| --- | ---: | ---: | ---: | --- |
| `k3_witness_replay_overlap` | `4` | `4/4` | `4/4` | `reuse_endpoint_local_parity` |
| `k4_stuck_vs_counterpart` | `8` | `8/8` | `0/8` | `rank_or_propose_inside_coarse_bucket` |

So the descriptor is not just "finer in the abstract". It separates the exact
two cases Goal 4 cares about:

- exact local parity reuse across two known witness surfaces; and
- coarse-only near-miss overlap where the missing structure is active-block
  layout, not aggregate mass/support.

### Positive control: known `k = 3` witness/replay overlap

The helper finds `4` endpoint-near replay overlaps between the Baker and
non-Baker lag-`7` witnesses:

- step `1`, source side, `3x3`
- step `2`, source side, `4x4`
- step `3`, source side, `4x4`
- step `4`, target side, `4x4`

All four pairs satisfy:

- same coarse signature; and
- same trimmed active-window descriptor after `canonical_perm()`.

Example:

- pair:
  `k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19:search-shortcut_search-lag-7:step2`
  vs
  `k3_shortcut_round1:search-shortcut_search-lag-7:step2`
- trimmed window:
  `4x4|0,0,1,2,1,0,1,2,2,0,1,2,1,1,0,1`
- signal:
  `exact_trimmed_window_match`
- action:
  `reuse_endpoint_local_parity`

This is the useful positive control: candidate B recognizes that the two
forward/backward witness surfaces are genuinely the same endpoint-local square
shape, not merely in the same coarse bucket.

### Retained `k = 4` signal: coarse overlap but trimmed layout mismatch

All top `8` retained `k = 4` stuck/counterpart pairs behave the opposite way:

- same coarse signature in every pair; but
- different trimmed active windows in every pair.

Representative retained pair:

- pair id: `k4_stuck_rank4_diagonal_refactorization_4x4`
- coarse signal:
  rows `0/0/10/14`, cols `2/4/5/13`, same support profile on both sides
- frontier trimmed window:
  `2x4|0,6,1,3,2,7,4,1`
- counterpart trimmed window:
  `2x4|0,1,1,12,4,4,1,1`
- signal:
  `coarse_only_layout_mismatch`
- action:
  `rank_or_propose_inside_coarse_bucket`

This repeats across all `8` retained pairs, including both
`diagonal_refactorization_4x4` entries (`rank 4`, `rank 6`) and the sampled
`elementary_conjugation` hotspots. The descriptor therefore generalizes beyond
one hand-picked counterexample inside this bounded report.

## Comparison To The Earlier Candidate Table

The earlier `sse-rust-1j1` note already showed:

- `mass_support_signature` is too coarse as a hard local normal form; and
- `trimmed_active_window` is the cleanest narrow-kept descriptor.

This follow-up adds the missing parity-specific reading:

- candidate B is **not** a good hard duplicate-reduction key for the main
  frontier, because its collision buckets stay small; but
- candidate B **is** a useful endpoint-parity discriminator inside the existing
  coarse approximate bucket for square `3x3` / `4x4` states.

That is exactly the Goal 4 use case.

## Exact Future-Layer Signal

The signal to consume in a future proposal/ranking layer should be:

1. only on square `3x3` / `4x4` frontier states;
2. only after `canonical_perm()`;
3. only after the existing coarse approximate signature already matches; and
4. as a three-way endpoint-local relation:

| Condition | Signal | Meaning | Use |
| --- | --- | --- | --- |
| coarse match + trimmed match | `reuse_endpoint_local_parity` | the two sides share the same local square endpoint surface | safe parity/ranking boost |
| coarse match + trimmed mismatch | `rank_or_propose_inside_coarse_bucket` | the two sides have the same coarse mass/support envelope, but a different active-block layout | proposal/ranking cue only; not a hard parity equivalence |
| coarse mismatch | `ignore` | no local endpoint overlap on this surface | no use from this descriptor |

The important boundary is the second row. The retained `k = 4` evidence says
that coarse overlap without trimmed equality is exactly where the interesting
near-miss structure lives. So candidate B should **not** be upgraded to a hard
parity filter. It should be consumed as a ranking/proposal discriminator inside
the coarse bucket.

## Decision

Decision: **keep, narrowly, for Goal 4 endpoint parity diagnostics and future
proposal/ranking surfaces. Do not promote it into production canonicalization or
hard frontier dedup.**

Why:

- positive control passes:
  the descriptor recovers exact witness/replay endpoint-local reuse on all `4`
  observed `k = 3` overlap pairs;
- retained `k = 4` signal is consistent:
  all `8` top stuck/counterpart pairs remain coarse-equal but trimmed-unequal;
- the resulting action is concrete:
  use candidate B as a secondary discriminator *within* the existing coarse
  approximate bucket, not as a replacement for that bucket; and
- this stays within the Goal 4 diagnostic boundary.

## Validation

Observed results:

- `cargo fmt --all` passed;
- `cargo test --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms`
  passed (`9` tests);
- the fresh stuck-state extractor wrote
  `tmp/sse-rust-w7e4-k4-stuck-top16.json`; and
- the bounded parity diagnostic wrote
  `tmp/sse-rust-w7e4-trimmed-active-window-parity.json`.
