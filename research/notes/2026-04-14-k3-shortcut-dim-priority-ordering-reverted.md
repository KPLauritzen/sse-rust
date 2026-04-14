# k=3 shortcut: dim-priority segment ordering (reverted) (2026-04-14)

## Question
Can timeout-bounded `shortcut_search` improve hard-surface tractability by prioritizing segment attempts with smaller endpoint dimensions before higher-dimensional segments?

## Code experiment

Temporary runtime patch in `src/search.rs` (reverted):

- in `refine_guide_path_once`, precompute segment candidates,
- when `guided_segment_timeout_secs` is set, sort by `(max_endpoint_dim, gap, start, end)`,
- then execute segment searches in that order.

## Validation

- `cargo test -q -- --test-threads=1` passed.
- Harness artifact (`just research-json-save loop25-dim-priority-segment-ordering`) kept hard gate and score:
  - required `24/24`, target hits `21`, points `3645`, telemetry focus `45,802,619`
  - total elapsed improved slightly (`13,628 -> 13,332 ms`).

## Hard-surface check (decision metric)

Config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- `stage=shortcut-search`
- `move_policy=mixed`
- `max_intermediate_dim=5`, `max_entry=5`
- `guided_min_gap=2`, `guided_max_gap=5`, `guided_max_shortcut_lag=5`
- `guided_segment_timeout=5`, `guided_rounds=2`
- `shortcut_max_guides=8`, `shortcut_max_total_segment_attempts=160`

Result vs pre-patch baseline (`tmp/loop23_dim5_lagcap5_gap5_a160_mixed_entry5_g8.json`):

- outcome unchanged: lag `7`, improvements `20`, promoted `3`
- but work regressed:
  - factorisations `12,970,458 -> 13,435,679`
  - visited `539,515 -> 559,880`
  - frontier `11,896 -> 12,200`

## Decision

Reverted the patch. The hard target metric regressed despite neutral harness score.
