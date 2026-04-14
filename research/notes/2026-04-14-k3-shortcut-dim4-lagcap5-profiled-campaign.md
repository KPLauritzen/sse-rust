# k=3 shortcut: profiled dim4 vs dim5 and lag-cap-5 campaign (2026-04-14)

## Question
Can we use profiling-guided bound changes to convert the hard `k=3` shortcut plateau (lag `7`) into deeper usable search without timeout blowups?

## Profiling-first baseline (hard dim5 control)

Command (prebuilt binary, bounded timeout):

```bash
timeout -k 10s 180s target/dist/search \
  1,3,2,1 1,6,1,1 \
  --max-lag 7 \
  --stage shortcut-search \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --guided-max-shortcut-lag 4 \
  --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 \
  --shortcut-max-total-segment-attempts 32 \
  --json --telemetry --pprof
```

Result (`tmp/loop14_profile_run.json`):

- outcome `equivalent`, lag `7`
- factorisations enumerated `4,871,620`
- frontier nodes expanded `4,426`
- visited nodes `277,582`
- guided segments improved `1`

`pprof` (`tmp/loop14_profile.txt`) remained factorisation-dominated, with hot stacks repeatedly passing through `visit_all_factorisations_with_family`, `visit_binary_sparse_factorisations_4x4_to_5`, and `solve_nonneg_4x4`.

## Dimensionality probe (same endpoint/config family)

### Lag-cap 4

- `dim4, attempts=32` (`tmp/loop14_dim4_a32.json`)
  - lag `7`, improved `1`
  - factorisations `2,417,141`, frontier `3,668`, visited `248,886`
- `dim4, attempts=128` (`tmp/loop14_dim4_a128.json`)
  - lag `7`, improved `11`, promoted guides `2`
  - factorisations `9,203,481`, frontier `15,449`, visited `1,044,306`
- `dim4, attempts=512` (`tmp/loop14_dim4_a512.json`)
  - lag `7`, improved `112`, promoted guides `5`
  - factorisations `21,726,337`, frontier `36,878`, visited `1,855,340`

Observation: dim4 gives much better tractability than dim5 at the same attempt caps, but still does not break lag `7`.

### Lag-cap 5 (enabled by dim4 tractability)

- full gap window (`min_gap=2,max_gap=6`)
  - `attempts=256` (`tmp/loop14_dim4_lagcap5_a256.json`): lag `7`, improved `41`, frontier `514,335`, visited `4,925,090`
  - `attempts=512` (`tmp/loop14_dim4_lagcap5_a512.json`): lag `7`, improved `112`, frontier `856,001`, visited `6,486,917`
- narrow gap window (`min_gap=2,max_gap=4`)
  - `attempts=512` (`tmp/loop14_dim4_lagcap5_gap4_a512.json`): lag `7`, improved `106`, promoted `7`, very low cost (frontier `1,443`, visited `78,568`)
  - `attempts=2048, rounds=2` (`tmp/loop14_dim4_lagcap5_gap4_a2048.json`): lag `7`, improved `205`, promoted `10`, stop `max_rounds_reached`
  - `attempts=2048, rounds=4` (`tmp/loop14_dim4_lagcap5_gap4_a2048_r4.json`): lag `7`, same misses/work, stop `guide_pool_exhausted` after round `3`

## Interpretation

- Profiling signal was confirmed: the dim5 surface is bottlenecked by high-dimensional factorisation families.
- Lowering to `max_intermediate_dim=4` buys major runtime headroom and allows aggressive lag-cap-5 campaigns that were previously timeout-prone.
- Despite that, all runs still converge to lag `7`; extra lag-cap depth mainly increases work, not witness quality.
- Under the cheap `gap<=4` surface, search quickly saturates the useful guide pool (`guide_pool_exhausted`) without lag improvement.

## Next hypothesis

Keep the cheap dim4 staging surface as a feeder, then apply a selective (not broad) dim5/full-gap follow-up on a *diverse promoted subset* rather than a single best guide or full-pool brute force. The current evidence suggests bottleneck is structural guide diversity/segment selection, not raw attempt budget alone.
