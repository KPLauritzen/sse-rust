# k=3 shortcut: equivalent lag-dominance segment-cache reuse (2026-04-14)

## Question
Can shortcut segment-cache reuse be improved by reusing known equivalent segment results across higher lag caps (safe dominance direction: lower lag proof implies higher lag proof)?

## Change

Runtime change in `src/search.rs`:

- replaced the plain exact-key segment cache with a small cache wrapper that keeps:
  - exact `(source,target,max_lag)` results,
  - the shortest known equivalent path per `(source,target)`.
- cache lookup now checks exact first, then reuses the shortest equivalent path when `path_lag <= requested_max_lag`.
- no Unknown/not-equivalent dominance reuse was added.

Validation coverage:

- existing cache-hit test still passes,
- added `test_refine_guide_path_once_reuses_equivalent_cache_result_across_lag_caps` to verify reuse of a cached lag-1 equivalent result for a lag-2 query.

## Measurement

All probes run via `timeout -k ... target/dist/search`.

### Baseline (before change, prior artifact)

Config: `dim4`, `lagcap=5`, `min_gap=2`, `max_gap=6`, attempts `512`, rounds `2`.

Artifact: `tmp/loop14_dim4_lagcap5_a512.json`

- outcome `equivalent`, lag `7`
- factorisations `33,957,312`
- frontier `856,001`
- visited `6,486,917`
- guided improvements `112`
- cache hits/misses `151 / 361`

### After change

Same config, two deterministic reruns:

- `tmp/loop16_after_dim4_lagcap5_gap6_a512.json`
- `tmp/loop16_after_dim4_lagcap5_gap6_a512_repeat.json`

Both produced identical telemetry:

- outcome `equivalent`, lag `7`
- factorisations `33,499,328`
- frontier `855,116`
- visited `6,472,560`
- guided improvements `112`
- cache hits/misses `164 / 348`

### Control scenario where no cross-lag reuse was expected

Config: `dim4`, `lagcap=5`, `min_gap=2`, `max_gap=4`, attempts `2048`, rounds `2`.

- before: `tmp/loop16_baseline_dim4_lagcap5_gap4_a2048.json`
- after: `tmp/loop16_after_dim4_lagcap5_gap4_a2048.json`

Telemetry was identical, indicating the change only helps when cross-lag repeated segments exist.

## Interpretation

- The change is correctness-neutral on measured outcomes (lag remains `7`) and required harness cases.
- It improved cache reuse on the wider-gap dim4 lagcap5 probe (`hits +13`, `misses -13`) with a small deterministic reduction in factorisation/visited work.
- This is a modest but real runtime win on the active shortcut plateau path.

## Harness gate

`just research-json-save loop16-lag-dominance-cache`:

- required cases: `24/24` passed,
- `target_hits`: `21` (unchanged),
- `total_points`: `3645` (unchanged),
- `telemetry_focus_score`: `45802619` (unchanged),
- `total_elapsed_ms`: `13554 -> 13471` vs `loop14-dim4-lagcap5`.

## Next hypothesis

Combine this safe cache dominance with guide admission that actually increases `unique_guides` after re-anchoring, then retest hard dim5 stage-2 where current feeder injections still collapse to existing guide identities.
