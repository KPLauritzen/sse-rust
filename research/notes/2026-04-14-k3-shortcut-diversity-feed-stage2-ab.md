# k=3 shortcut: dim4 feeder guide diversity into dim5 stage-2 A/B (2026-04-14)

## Question
If lag-7 feeder guides from the cheap dim4 surface are injected into the hard dim5 lag-cap-5 stage, does extra guide diversity improve the lag-7 plateau?

## Setup

### Feeder guide generation

Generated feeder guides from dim4 shortcut runs with `--write-guide-artifact`:

- `tmp/loop15_feed1_dim4_lagcap5_gap4_a2048.json`
- `tmp/loop15_feed2_dim4_lagcap5_gap4_a2048_r4.json`
- `tmp/loop15_feed3_dim4_lagcap4_gap6_a512.json`

All three exported the same path hash (same best witness under these settings):

- `e8a5f6a9b4fea2e02bc0754e7301808ca30e7b33448015152e82e332bc4e5b7f`

That hash is not a byte-identical path in the normalized pool JSON, so it is a candidate diversity addition at artifact level.

### Stage-2 A/B (hard dim5, lag-cap 5)

Common config:

- endpoint: `1,3,2,1 -> 1,6,1,1`
- stage: `shortcut-search`
- `max_intermediate_dim=5`, `max_entry=6`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=6`
- `guided_segment_timeout=5`, `guided_rounds=2`
- `shortcut_max_guides=16`, `shortcut_rounds=2`, attempts `64`
- outer cap: `timeout -k 10s 240s`

Variants:

1. pool-only (`research/guide_artifacts/k3_normalized_guide_pool.json`)
2. pool+newguide (pool + `tmp/loop15_feed1_dim4_lagcap5_gap4_a2048.json`)

## Results

Pool-only (`tmp/loop15_stage2_dim5_lagcap5_a64_pool_only.json`):

- outcome `equivalent`, lag `7`
- guides loaded/accepted/unique: `12 / 12 / 12`
- factorisations `10,357,960`
- frontier `9,431`, visited `581,304`
- guided improvements `3`, promoted guides `1`
- stop `max_segment_attempts_reached`

Pool+newguide (`tmp/loop15_stage2_dim5_lagcap5_a64_pool_plus_newguide.json`):

- outcome `equivalent`, lag `7`
- guides loaded/accepted/unique: `13 / 13 / 12`
- factorisations `10,415,093`
- frontier `9,591`, visited `592,942`
- guided improvements `3`, promoted guides `1`
- stop `max_segment_attempts_reached`

## Interpretation

- The injected feeder artifact was accepted syntactically but did not increase the unique working guide count (`unique_guides` stayed `12`), so runtime behavior was effectively unchanged.
- No lag improvement and slight work increase suggest canonical re-anchoring/dedup still collapses this feeder path to existing pool structure.

## Next hypothesis

To test true diversity effects, generate feeder artifacts that are *ranking-distinct after re-anchoring* (not just byte-distinct paths), then rerun dim5 stage-2 with expanded `max_guides` and compare `unique_guides`/`initial_working_set_guides` before evaluating lag impact.
