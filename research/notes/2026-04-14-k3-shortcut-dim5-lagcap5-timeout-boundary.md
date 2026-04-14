# k=3 shortcut: dim5 lag-cap-5 timeout boundary map (2026-04-14)

## Question
After loop16 cache improvements, can hard dim5 lag-cap-5 stage-2 runs push beyond previous timeout limits by tuning attempt budget and simple admission controls?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`, `max_entry=6`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=6`
- `guided_segment_timeout=5`, `guided_rounds=2`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- outer cap `timeout -k 10s 240s`

## Results

### Attempts sweep (`shortcut_max_total_segment_attempts`)

- `96` (`tmp/loop19_dim5_pool_only_lagcap5_timeout5_a96.json`): completed
  - lag `7`
  - guided improvements `5`
  - promoted guides `2`
  - factorisations `16,202,168`
  - frontier `17,009`
  - visited `1,013,560`
  - stop `max_segment_attempts_reached`
- `104` (`tmp/loop19_dim5_pool_only_lagcap5_timeout5_a104.json`): timeout (`124`, empty JSON)
- `112` (`tmp/loop19_dim5_pool_only_lagcap5_timeout5_a112.json`): timeout (`124`, empty JSON)
- `128` (`tmp/loop19_dim5_pool_only_lagcap5_timeout5_a128.json`): timeout (`124`, empty JSON)

### Attempted rescue knobs at attempts=128

All still timed out at 240s:

- `max_guides=8` (`tmp/loop19_dim5_pool_only_lagcap5_timeout5_a128_guides8.json`)
- `max_guides=4` (`tmp/loop19_dim5_pool_only_lagcap5_timeout5_a128_guides4.json`)
- `shortcut_rounds=1` (`tmp/loop19_dim5_pool_only_lagcap5_timeout5_a128_rounds1.json`)

## Interpretation

- The hard dim5 lag-cap-5 surface still has a steep runtime cliff between attempts `96` and `104+` under this timeout policy.
- Simple knob reductions (`max_guides`, `shortcut_rounds`) do not move the `128` timeout boundary.
- No lag<7 progress observed.

## Next hypothesis

The boundary is driven by expensive segment mix, not coarse outer budget knobs; next step should target segment-level admission quality or per-gap selective scheduling rather than global guide/round counts.
