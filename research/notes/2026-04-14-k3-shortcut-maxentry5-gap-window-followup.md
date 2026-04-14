# k=3 shortcut: max_entry=5 gap-window follow-up on hard dim5 stage-2 (2026-04-14)

## Question
After identifying `mixed + max_entry=5` as a tractability lever, does it open previously timeout-prone wider gap windows, and can guide-count tuning buy more budget headroom in the good `gap=5` window?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`
- `move_policy=mixed`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`
- `guided_segment_timeout=5`, `guided_rounds=2`
- `shortcut_rounds=2`
- direct `target/dist/search` with `timeout -k 10s ...`

## Results

### Wider gap check under `max_entry=5`

- `gap=6`, attempts `128`, guides `12` (`tmp/loop23_dim5_lagcap5_gap6_a128_mixed_entry5.json`): timed out at 240s (`124`, empty JSON).
- `gap=6`, attempts `96`, guides `12` (`tmp/loop23_dim5_lagcap5_gap6_a96_mixed_entry5.json`): completed
  - lag `7`
  - improvements `5`, promoted `2`
  - factorisations `14,618,351`
  - visited `659,196`
  - stop `max_segment_attempts_reached`

Compared to `gap=5`, attempts `96` (`max_entry=5`), this is substantially more expensive and lower-yield (fewer improvements).

### Guide-count tuning inside the tractable `gap=5`, attempts `160` window

Baseline (`guides=12`) from prior loop:

- `tmp/loop22_dim5_lagcap5_gap5_a160_mixed_entry5.json`
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `13,684,131`
  - visited `599,474`
  - wall `237s`

Follow-ups:

- `guides=8` (`tmp/loop23_dim5_lagcap5_gap5_a160_mixed_entry5_g8.json`)
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `12,970,458`
  - visited `539,515`
  - wall `233s`
- `guides=6` (`tmp/loop23_dim5_lagcap5_gap5_a160_mixed_entry5_g6.json`)
  - lag `7`, improvements `20`, promoted `3`
  - factorisations `13,102,948`
  - visited `562,299`
  - wall `231s`

Control reminder (`max_entry=6`, attempts `160`, guides `12`): timed out at 240s (`tmp/loop22_dim5_lagcap5_gap5_a160_mixed_entry6.json`, empty JSON).

## Interpretation

- `max_entry=5` does **not** make `gap=6` a practical search window at moderate budgets; it remains near the timeout cliff and delivers weaker local yield.
- The useful operating region remains `gap=5`.
- Within `gap=5`, reducing guide count from `12` to `8` or `6` keeps outcomes identical and gives modest cost reductions, but not enough headroom to fundamentally change the budget ceiling.
- No lag `<7` witness found.

## Next hypothesis

Keep `mixed + max_entry=5 + gap=5` as the hard-surface baseline and target segment-selection quality (which segments are attempted first) rather than wider-gap expansion or further guide-count trimming.
