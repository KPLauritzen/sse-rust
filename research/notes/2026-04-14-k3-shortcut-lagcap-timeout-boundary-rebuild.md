# k=3 shortcut: lag-cap and segment-timeout boundary on rebuilt hard baseline (2026-04-14)

## Question
On the rebuilt hard dim5 baseline (`mixed + max_entry=5 + min_gap=2 + max_gap=5`), which lag-cap/segment-timeout setting gives the best tractable progress under a 240s outer cap?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`, `max_entry=5`
- `move_policy=mixed`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_min_gap=2`, `guided_max_gap=5`, `guided_rounds=2`
- `shortcut_rounds=2`, `shortcut_max_guides=8`
- outer cap `timeout -k 10s 240s`
- direct `target/dist/search`

## Results

### Lag-cap check at attempts=160 (`guided_segment_timeout=5`)

- lag-cap `4` (`tmp/loop27_dim5_lagcap4_gap5_a160_mixed_entry5_g8.json`):
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `13,559,905`, visited `579,677`
  - wall `237s`
- lag-cap `5` baseline (`tmp/loop23_dim5_lagcap5_gap5_a160_mixed_entry5_g8.json`):
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `12,970,458`, visited `539,515`
  - wall `233s`

Lag-cap 5 dominates lag-cap 4 on this surface.

### Segment-timeout retune for lag-cap=5

At attempts=160:

- timeout `5` baseline:
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `12,970,458`, visited `539,515`
  - wall `233s`
- timeout `3` (`tmp/loop27_dim5_lagcap5_gap5_a160_mixed_entry5_g8_t3.json`):
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `12,512,442`, visited `474,120`
  - wall `230s`

Timeout 3 keeps progress and reduces work.

### Attempts boundary under timeout=3

- attempts `168` (`tmp/loop27_dim5_lagcap5_gap5_a168_mixed_entry5_g8_t3.json`): completed
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `12,789,994`, visited `477,752`
  - wall `236s`
- attempts `176` (`tmp/loop27_dim5_lagcap5_gap5_a176_mixed_entry5_g8_t3.json`): timeout (`124`, empty JSON)
- attempts `192` (`tmp/loop27_dim5_lagcap5_gap5_a192_mixed_entry5_g8_t3.json`): timeout (`124`, empty JSON)

## Interpretation

- On this rebuilt hard surface, lag-cap `5` remains better than lag-cap `4`.
- Reducing segment timeout from 5 to 3 is a valid tractability improvement at fixed progress.
- Timeout 3 extends the feasible attempt boundary slightly (up to `168`), but no new lag reduction or additional local improvements beyond the attempts-160 plateau was observed.

## Next hypothesis

Keep lag-cap `5`; adopt timeout `3` as the working hard-surface baseline and explore whether guide-pool breadth at attempts `168` can convert the recovered budget into additional improvements.
