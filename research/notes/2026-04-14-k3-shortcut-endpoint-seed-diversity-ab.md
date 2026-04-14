# k=3 shortcut: endpoint-seed diversity A/B into dim5 stage-2 (2026-04-14)

## Question
If we add a genuinely distinct guide (not collapsed by re-anchored dedup) from endpoint search, does hard dim5 stage-2 shortcutting improve the lag-7 plateau?

## Feeder sweep (dim4 shortcut stage)

Tried multiple dim4 shortcut feeder configs (all with normalized pool input, attempts 512):

- lagcap3 gap6
- lagcap4 gap6
- lagcap5 gap5
- lagcap5 gap6

All exported the same lag-7 path hash:

- `e8a5f6a9b4fea2e02bc0754e7301808ca30e7b33448015152e82e332bc4e5b7f`

This hash is not one of the current normalized-pool hashes, but these feeder variants did not diversify beyond that single path.

## Alternative source: endpoint search seed

Ran endpoint BFS mixed search (`dim4`, `entry6`, `max_lag=7`) and exported:

- artifact: `tmp/loop17_endpoint_bfs_mixed_dim4e6_l7.json`
- witness lag: `8`
- path hash: `71486936905c079c9374bce718d1773846cee89111822c0aa1c70d2c567f38b0`

This hash is distinct from both the normalized pool and the feeder lag-7 hash above.

## Stage-2 A/B on hard dim5 lag-cap 5

Common config:

- stage `shortcut-search`, `max_intermediate_dim=5`, `max_entry=6`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=6`
- `guided_segment_timeout=5`, `guided_rounds=2`
- `shortcut_max_guides=16`, `shortcut_rounds=2`, attempts `64`
- bounded with `timeout -k 10s 240s`

### Pool-only control

Artifact: `tmp/loop15_stage2_dim5_lagcap5_a64_pool_only.json`

- guides loaded/accepted/unique: `12 / 12 / 12`
- lag `7`
- guided improvements/promoted: `3 / 1`
- factorisations `10,357,960`
- frontier `9,431`
- visited `581,304`

### Pool + distinct lag-8 endpoint seed

Artifact: `tmp/loop17_stage2_dim5_pool_plus_lag8guide_a64.json`

- guides loaded/accepted/unique: `13 / 13 / 13`
- lag `7`
- guided improvements/promoted: `3 / 1`
- factorisations `11,921,204`
- frontier `11,491`
- visited `751,631`

## Higher-attempt follow-up

Pool + lag-8 seed at attempts `128`:

- artifact: `tmp/loop17_stage2_dim5_pool_plus_lag8guide_a128.json`
- timed out at outer cap `240s` (empty JSON)

## Interpretation

- This run confirms we can raise `unique_guides` (`12 -> 13`) with a truly distinct endpoint-derived seed.
- Even with higher unique-guide count, dim5 stage-2 did not improve lag at attempts 64 and became more expensive.
- Increasing attempts to 128 with this extra seed pushed the run back into timeout.

## Next hypothesis

Diversity alone is not sufficient; prioritize *quality-aware admission* of extra guides (e.g., segment-level usefulness) before adding them to hard dim5 stage-2 working sets.
