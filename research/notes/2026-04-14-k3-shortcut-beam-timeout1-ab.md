# k=3 shortcut: beam frontier and timeout coupling A/B on hard dim5 stage-2 (2026-04-14)

## Question
Can segment searches in `shortcut_search` benefit from beam frontier mode enough to improve lag/progress or tractability versus the rebuilt BFS baseline (`mixed + max_entry=5 + gap<=5`)?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`, `max_entry=5`
- `move_policy=mixed`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=5`
- `guided_rounds=2`, `shortcut_rounds=2`, `shortcut_max_guides=12`
- direct `target/dist/search` with bounded outer timeout

BFS reference points:

- attempts `96`, `guided_segment_timeout=5`: lag `7`, improvements `10`, wall `145s`.
- attempts `160`, `guided_segment_timeout=5`: lag `7`, improvements `20`, wall `237s`.

## Results

### Beam with `guided_segment_timeout=5`

- beam width `32`, attempts `96` (`tmp/loop26_dim5_lagcap5_gap5_a96_mixed_entry5_beam32.json`): timed out at 180s.
- beam width `8`, attempts `96` (`tmp/loop26_dim5_lagcap5_gap5_a96_mixed_entry5_beam8.json`): completed
  - lag `7`, improvements `11`, promoted `2`
  - factorisations `7,948,646`, visited `133,709`
  - wall `235s`
- beam width `4`, attempts `96` (`tmp/loop26_dim5_lagcap5_gap5_a96_mixed_entry5_beam4.json`): completed
  - lag `7`, improvements `10`, promoted `2`
  - factorisations `4,447,622`, visited `90,163`
  - wall `196s`

Beam reduced work counters but did not improve lag, and wall time remained worse than BFS.

### Beam width 4 with tighter segment timeout (`guided_segment_timeout=1`)

- attempts `96` (`tmp/loop26_dim5_lagcap5_gap5_a96_mixed_entry5_beam4_t1.json`):
  - lag `7`, improvements `5`, promoted `2`
  - factorisations `2,997,864`, visited `59,366`
  - wall `113s`
- attempts `160` (`tmp/loop26_dim5_lagcap5_gap5_a160_mixed_entry5_beam4_t1.json`):
  - lag `7`, improvements `13`, promoted `2`
  - wall `178s`
- attempts `192` (`tmp/loop26_dim5_lagcap5_gap5_a192_mixed_entry5_beam4_t1.json`):
  - lag `7`, improvements `13`, promoted `2`
  - wall `219s`

The timeout-1 variant is faster, but local progress saturates early (improvements flat at 13 by attempts 160-192), still below BFS attempts-160 improvements (20).

## Interpretation

- Beam mode does not move lag below 7 on this surface.
- With timeout 5, beam cuts work counters but increases wall time versus BFS.
- With timeout 1, beam width 4 gains wall-time speed but loses improvement yield and plateaus early.

## Decision

Do not switch the active hard-dim5 baseline to beam. Keep BFS-based mixed baseline (`max_entry=5`, `gap<=5`, `min_gap=2`) for current loop objective.
