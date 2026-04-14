# k=3 shortcut-search budget ramp after segment caching (2026-04-14)

## Question

After landing per-run segment-query caching, does spending higher shortcut attempt budgets convert into lag `<7` progress on the hard `k=3` pair?

## Context

Base config for these probes:

- endpoints: `A=[[1,3],[2,1]]`, `B=[[1,6],[1,1]]`
- stage: `shortcut-search`
- guides: `research/guide_artifacts/k3_normalized_guide_pool.json`
- bounds: `max_intermediate_dim=5`, `max_entry=6`
- guided: `max_shortcut_lag=4`, `min_gap=2`, `max_gap=6`, `rounds=2`
- shortcut: `max_guides=12`, `rounds=2`

Reference (post-cache, 128 attempts):

- `tmp/k3_shortcut_loop3_cache128_control.json`
- lag `7`
- guided improvements `11`
- cache hits/misses `22/106`
- frontier nodes expanded `17918`
- total visited nodes `1121478`

## Evidence

### Attempt budget = 192

Command:

```bash
timeout 300 cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 4 --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 --shortcut-max-total-segment-attempts 192 \
  --json --telemetry > tmp/k3_shortcut_loop4_cache192_control.json
```

Outcome:

- `equivalent`, lag `7`
- guided improvements `24`
- cache hits/misses `70/122`
- promoted guides `3`
- frontier nodes expanded `20449`
- total visited nodes `1182674`

### Attempt budget = 256

Command:

```bash
timeout 420 cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 4 --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 --shortcut-max-total-segment-attempts 256 \
  --json --telemetry > tmp/k3_shortcut_loop4_cache256_control.json
```

Outcome:

- `equivalent`, lag `7`
- guided improvements `41`
- cache hits/misses `85/171`
- promoted guides `3`
- frontier nodes expanded `26045`
- total visited nodes `1347191`

### Higher per-segment lag cap probe

Command:

```bash
timeout 300 cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 5 --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 --shortcut-max-total-segment-attempts 128 \
  --json --telemetry > tmp/k3_shortcut_loop4_cache128_lagcap5.json
```

Outcome:

- timed out at 300s (exit `124`)
- no completed JSON artifact

## Conclusion

Caching allows larger attempt budgets to complete (192/256), and local improvement counts rise with budget, but best lag remains stuck at `7`. Increasing per-segment lag cap to `5` at 128 attempts is currently too expensive and did not yield completed evidence.

## Next Steps

Favor selective admission over broader per-segment search depth: use a cheap prefilter/ranking for segment attempts so the higher budgets prioritize likely-compressible segments instead of uniformly expanding expensive searches.
