# k=3 shortcut-search segment-query cache (2026-04-14)

## Question

Can `shortcut_search` reduce heavy repeated segment-search work by memoizing segment endpoint queries within one run, without changing required-case correctness?

## Context

The active `k=3` shortcut plateau remains lag `7`, and recent A/B runs showed high segment-search cost under fixed attempt caps.

Profiling-first evidence (bounded run):

- command used `search --stage shortcut-search ... --shortcut-max-total-segment-attempts 32 --pprof`
- hot path remained factorisation-heavy (`visit_all_factorisations_with_family`, `visit_binary_sparse_factorisations_3x3_to_4`, `solve_nonneg_3x3/4x4`)

I then checked duplication potential in the initial top-12 guide working set for `min_gap=2,max_gap=6`:

- candidate segment queries enumerated: `595`
- unique `(source,target,lag_cap)` queries: `514`
- duplicates: `81`

That suggested runtime query memoization could be worthwhile.

## Change

Implemented per-run guided segment cache in `shortcut_search`:

- cache key: `(source matrix, target matrix, lag_cap)`
- cache value: prior `DynSseResult`
- cache scope: one `shortcut_search` request across all guides/rounds
- unknown results are cached only when `guided.segment_timeout_secs` is unset
- added telemetry fields:
  - `shortcut_search.segment_cache_hits`
  - `shortcut_search.segment_cache_misses`

Also added unit coverage:

- `test_refine_guide_path_once_reuses_cached_segment_result`

## Evidence

### Baseline gate

- `cargo test -q`: pass (`201` tests + bin tests)
- `just research-json-save baseline`
- `just research-json-save 2026-04-14-loop3-segment-cache`

Harness fitness comparison:

- required passed: `24 -> 24`
- target hits: `21 -> 21`
- total points: `3645 -> 3645`
- telemetry-focus score: `45802619 -> 45802619`

### Targeted k=3 shortcut measurements

Control config:

- `guided-min-gap=2`, `guided-max-gap=6`
- attempts: `128`

Pre-cache (`tmp/k3_shortcut_loop2_control.json`):

- lag `7`
- guided segments improved `11`
- promoted guides `2`
- frontier nodes expanded `20088`
- total visited nodes `1263782`

Post-cache (`tmp/k3_shortcut_loop3_cache128_control.json`):

- lag `7`
- guided segments improved `11`
- promoted guides `2`
- cache hits `22`, misses `106`
- frontier nodes expanded `17918`
- total visited nodes `1121478`

Delta (post - pre):

- lag `0`
- guided improvements `0`
- frontier nodes expanded `-2170`
- total visited nodes `-142304`

Focused config:

- `guided-min-gap=3`, `guided-max-gap=7`
- attempts: `128`

Pre-cache run timed out at `300s` and produced no valid JSON.

Post-cache (`tmp/k3_shortcut_loop3_cache128_gapfocused.json`) completed under the same cap:

- lag `7`
- guided segments improved `15`
- promoted guides `3`
- cache hits `38`, misses `90`
- frontier nodes expanded `23424`
- total visited nodes `1483243`

## Conclusion

The segment-query cache is a net-positive runtime optimization for hard `k=3` shortcut-search workloads. It does not yet break the lag-7 plateau, but it materially reduces work on control-128 and allows a previously timeout-prone focused-128 run to finish.

## Next Steps

Use the saved budget to push more effective segment attempts rather than fixed-count attempts, e.g. add a wall-clock budget mode for `shortcut_search` or a stronger admission policy that spends attempts on segments with better payoff signal.
