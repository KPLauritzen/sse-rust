# K3 direct entry replacement probe (2026-04-27)

## Question

Test one bounded Goal 2 segment-replacement hypothesis: can either pinned
lag-7 entry corridor

```text
2x2 source -> 3x3 intermediate -> first 4x4 envelope target
```

be replaced by one direct lag-1 rectangular ESSE step with `U: 2x4`,
`V: 4x2`, `source = U * V`, and `target = V * U`?

The common source was:

```text
2x2: [1,3,2,1]
```

## Controls checked

Baker positions `0..2` from
`research/guide_artifacts/k3_shortcut_round1.json`:

```text
source:       2x2 [1,3,2,1]
intermediate: 3x3 [1,2,2,2,1,1,1,0,0]
target:       4x4 [1,2,2,0,1,0,2,0,0,1,1,1,1,1,2,0]
```

Non-Baker positions `0..2` from
`research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`:

```text
source:       2x2 [1,3,2,1]
intermediate: 3x3 [0,1,0,2,1,2,1,2,1]
target:       4x4 [1,0,1,1,2,1,0,2,2,1,0,1,2,1,0,0]
```

Both original guide paths have lag `7`. A successful direct entry replacement
would replace the original two-edge entry corridor by one edge, then stitch to
the unchanged suffix after position `2`, for stitched lag `1 + 5 = 6`.

## Method

I added an opt-in research binary:

```text
src/bin/probe_k3_direct_entry_replacement.rs
```

The probe loads exactly the two guide artifacts above, validates that positions
`0..2` match the pinned matrices, and applies an exact rational-rank precheck.

For any candidate direct ESSE step with `U: 2x4` and `V: 4x2`, the matrix
`V * U` has rank at most `2` over the rationals. Therefore any pinned `4x4`
target with rational rank greater than `2` cannot be `V * U` for any real,
integer, nonnegative, or entry-bounded choice of `U,V`. This precheck is
stronger than the requested `U,V <= 5` and generated-entry `<= 5` bounds.

No dimension `3` search, dimension `>4` search, guide-pool rebuilding,
endpoint multi-meet replay, generic shortcut replay, beam/ranking/pruning/
dedup/canonicalization change, or default solver move-generation change was
performed.

## Commands and artifacts

Focused test:

```bash
timeout -k 10s 180s cargo test --features research-tools --bin probe_k3_direct_entry_replacement
```

Build:

```bash
timeout -k 10s 180s cargo build --features research-tools --bin probe_k3_direct_entry_replacement
```

Bounded probe:

```bash
timeout -k 10s 180s target/debug/probe_k3_direct_entry_replacement \
  --baker-guide research/guide_artifacts/k3_shortcut_round1.json \
  --non-baker-guide research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --max-entry 5 \
  --max-attempts-per-target 500000 \
  > tmp/k3_direct_entry_replacement_probe_2026-04-27.json
```

Summary extraction:

```bash
jq '.controls[] | {label, original_path_lag, suffix_lag_after_entry_target,
  stitched_lag_if_direct_hit, source_rank, target_rank,
  candidate_factor_pair_attempts, capped, exhausted_under_bounds, hit,
  decision, early_pruning}' \
  tmp/k3_direct_entry_replacement_probe_2026-04-27.json
```

## Results

| control | source rank | target rank | attempts | capped | exhausted | hit |
| --- | ---: | ---: | ---: | --- | --- | --- |
| Baker | 2 | 3 | 0 | no | yes | no |
| Non-Baker | 2 | 3 | 0 | no | yes | no |

Early pruning criterion for both controls:

```text
target rank 3 exceeds rank(VU) <= 2 for V:4x2 and U:2x4
```

The attempt count is `0` for both controls because the exact rank obstruction
fires before any candidate factor pair is materialized. This is an exhausted
no-hit under the stated bounds, not an attempt-cap stop.

No `U,V` matrices were found. No stitched lag-6 source-to-target path exists
through this direct entry replacement for either pinned control.

## Decision

Reject the special direct `2x2 -> 4x4` entry replacement for the two pinned
Goal 2 controls.

Keep only the opt-in research probe as an auditable diagnostic. Do not add a
default solver move family and do not open another follow-up from this result:
the obstruction is complete for this exact hypothesis and does not expose a
smaller subproblem.
