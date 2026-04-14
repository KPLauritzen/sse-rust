# k=3 shortcut: dim5 lag-cap-5 gap-window saturation at attempts 128-192 (2026-04-14)

## Question
On the kept solver (post loop16 cache reuse), does increasing `shortcut_max_total_segment_attempts` beyond 128 help on the tractable hard-dim5 `guided_max_gap=5` surface, or does the run saturate earlier?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`, `max_entry=6`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_segment_timeout=5`, `guided_rounds=2`
- outer cap `timeout -k 10s 240s`
- execution style for process hygiene: direct `target/dist/search` under `timeout` (no `cargo run` wrapper)

## Results

### Gap window sweep at attempts=128

- `max_gap=4` (`tmp/loop20_dim5_lagcap5_gap4_a128.json`): completed
  - lag `7`
  - guided improvements `7`
  - promoted guides `2`
  - factorisations `3,949,840`
  - frontier `4,031`
  - visited `136,332`
  - stop `max_segment_attempts_reached`
- `max_gap=5` (`tmp/loop20_dim5_lagcap5_gap5_a128.json`): completed
  - lag `7`
  - guided improvements `10`
  - promoted guides `2`
  - factorisations `12,077,069`
  - frontier `12,659`
  - visited `689,054`
  - stop `max_segment_attempts_reached`
- `max_gap=6` (`tmp/loop20_dim5_lagcap5_gap6_a128.json`): timeout (`124`, empty JSON)

### Attempts ramp on the tractable middle window (`max_gap=5`)

- `attempts=160` (`tmp/loop20_dim5_lagcap5_gap5_a160.json`): completed
  - lag `7`
  - guided improvements `10`
  - promoted guides `2`
  - factorisations `13,710,929`
  - frontier `14,262`
  - visited `787,378`
  - stop `no_improvement_round`
- `attempts=192` (`tmp/loop20_dim5_lagcap5_gap5_a192.json`): completed
  - lag `7`
  - guided improvements `10`
  - promoted guides `2`
  - factorisations `14,286,045`
  - frontier `14,930`
  - visited `904,304`
  - stop `no_improvement_round`

## Interpretation

- `max_gap=5` is the highest tractable window under this timeout policy; `max_gap=6` remains timeout-prone.
- On `max_gap=5`, progress saturates before the larger attempt budgets: attempts `160` and `192` add work but do not increase improvements/promotions or improve lag.
- The run effectively saturates around `146` considered segments (`stop_reason=no_improvement_round`) on this surface.

## Next hypothesis

Given saturation on the current segment schedule, prioritize segment quality over raw attempt count: use profiling to target high-cost/low-yield segment families (currently square-factorisation-heavy) and cut expensive no-gain segment searches rather than raising global attempt budgets.

## Correction (rebuild validation)

These loop20 measurements were produced before an explicit `cargo build --profile dist --features research-tools --bin search` refresh in a later loop. A subsequent rebuild-validated rerun campaign is logged in `research/notes/2026-04-14-k3-shortcut-rebuild-validated-maxentry5-mixed.md`. Treat loop20 runtime/throughput deltas as provisional where they conflict with rebuild-validated artifacts.
