# k=3 shortcut: timeout=3 guide-breadth boundary check (2026-04-14)

## Question
With the improved hard-surface baseline (`guided_segment_timeout=3`, lag-cap 5), does changing `shortcut_max_guides` at the attempts-168 boundary convert budget into additional progress?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`, `max_entry=5`
- `move_policy=mixed`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=5`
- `guided_segment_timeout=3`, `guided_rounds=2`
- `shortcut_rounds=2`, `shortcut_max_total_segment_attempts=168`
- outer cap `timeout -k 10s 240s`

## Results

- `max_guides=12` (`tmp/loop28_dim5_lagcap5_gap5_a168_mixed_entry5_g12_t3.json`):
  - lag `7`, improvements `20`, promoted `3`
  - rounds `1` (working set `12`)
  - factorisations `12,789,994`, visited `477,752`
  - wall `237s`
- `max_guides=8` baseline (`tmp/loop27_dim5_lagcap5_gap5_a168_mixed_entry5_g8_t3.json`):
  - identical metrics to guides=12
- `max_guides=4` (`tmp/loop28_dim5_lagcap5_gap5_a168_mixed_entry5_g4_t3.json`):
  - lag `7`, improvements `20`, promoted `3`
  - rounds `2` (working sets `4,4`)
  - higher cost: factorisations `13,205,484`, visited `502,698`
  - wall `240s`

## Interpretation

- Increasing guide breadth from 8 to 12 is inert at this boundary (identical outcome/work).
- Narrowing to 4 is regressive due extra round churn.
- Guide-count tuning appears exhausted on this hard surface.

## Decision

Keep `shortcut_max_guides=8` (or 12; equivalent here) and avoid `4` on this boundary.
