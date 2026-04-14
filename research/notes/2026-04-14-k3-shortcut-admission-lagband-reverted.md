# k=3 shortcut: lag-band guide-admission heuristic (reverted, 2026-04-14)

## Question
Can shortcut stage-2 tractability improve by prioritizing guides near current best lag (admission lag band), especially when extra higher-lag guides are injected?

## Patch tested (reverted)

In `src/search.rs`, `ShortcutGuidePool::take_working_set` was changed to prefer unprocessed guides with:

- `effective_lag <= best_lag + SHORTCUT_WORKING_SET_LAG_SLACK`

with fallback to all unprocessed guides if the filtered set was empty.

Two settings were tested:

- `SHORTCUT_WORKING_SET_LAG_SLACK = 0`
- `SHORTCUT_WORKING_SET_LAG_SLACK = 1`

## Measurement

### A) `slack=0`

Hard dim5 stage-2 with distinct lag-8 seed (previously from `loop17`):

- config: lagcap5, attempts64, timeout5, max_guides16
- artifact: `tmp/loop18_pool_plus_lag8_after_admission_a64.json`
- result: lag `7`, guided improvements/promoted `3/1`
- round working set shrank from `13` to `2`
- work dropped versus pre-patch pool+lag8 run:
  - factorisations `11,921,204 -> 10,539,397`
  - frontier `11,491 -> 9,847`
  - visited `751,631 -> 616,833`

The same plus-seed config at attempts128 now completed (where pre-patch timed out at 240s):

- artifact: `tmp/loop18_pool_plus_lag8_after_admission_a128.json`
- lag `7`, stop `no_improvement_round`, no lag gain.

### B) Harness gate impact for `slack=0`

`just research-json-save loop18-admission-lagband`:

- required cases: `24/24` passed (no correctness regression)
- `target_hits`: `21` (unchanged)
- `total_points`: `3645` (unchanged)
- **telemetry_focus_score regressed**: `45,802,619 -> 43,380,841`
- directed score also regressed: `8,920,000 -> 7,706,000`

### C) `slack=1`

Retest plus-seed attempts128 config:

- artifact: `tmp/loop18b_pool_plus_lag8_slack1_a128.json`
- timed out at 240s (empty JSON), so the tractability win from `slack=0` did not carry over.

## Decision

Reverted the admission-lagband patch.

Reason: despite local tractability improvements on one dim5 stage-2 slice, `slack=0` regressed harness telemetry-focus metrics, and `slack=1` lost the observed tractability benefit.

## Next hypothesis

Keep admission behavior unchanged for now and focus on less aggressive guide-quality signals that do not collapse working sets enough to hurt telemetry-focus progress.
