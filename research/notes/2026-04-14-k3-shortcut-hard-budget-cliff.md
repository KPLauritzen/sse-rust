# k=3 hard shortcut-search budget cliff at dim5 (2026-04-14)

## Question

Can a slightly tighter hard surface (`max_entry=5`) support larger shortcut attempt budgets at `max_intermediate_dim=5` without losing k=3 progress?

## Config

Base config for this sweep:

- endpoints: `A=[[1,3],[2,1]]`, `B=[[1,6],[1,1]]`
- stage: `shortcut-search`
- guides: `research/guide_artifacts/k3_normalized_guide_pool.json`
- `max_intermediate_dim=5`
- guided: `max_shortcut_lag=4`, `min_gap=2`, `max_gap=6`, `rounds=2`
- shortcut: `max_guides=12`, `rounds=2`
- outer command timeout: `180s`

## Results

### `max_entry=5`

- attempts `64`: timeout (`124`), empty JSON
- attempts `48`: timeout (`124`), empty JSON
- attempts `40`: timeout (`124`), empty JSON
- attempts `36`: timeout (`124`), empty JSON
- attempts `32`: completed (`tmp/loop10_dim5_entry5_a32.json`)
  - lag `7`
  - guided segments improved `1`
  - promoted guides `1`
  - frontier nodes expanded `3562`
  - total visited `188970`

Additional probe at attempts `48` with `max_guides=4` also timed out (`124`).

### `max_entry=6` comparison at attempts `32`

- completed (`tmp/loop10_dim5_entry6_a32_notimeout_180.json`)
  - lag `7`
  - guided segments improved `1`
  - promoted guides `1`
  - frontier nodes expanded `4426`
  - total visited `277582`

## Interpretation

- The hard dim5 surface has a sharp tractability cliff between attempts `32` and `36+` under this runtime budget.
- Lowering `max_entry` from `6` to `5` reduces work at attempts `32`, but does not improve witness quality (still lag `7`, same local improvement count).
- Guide-count reduction alone (`max_guides=4`) did not rescue attempts `48`.

## Conclusion

This parameter-only retune does not move Goal 2. It provides a slightly cheaper completed operating point (`attempts=32`), but no lag `<7` progress and no budget-extension breakthrough.

## Next hypothesis

Focus on segment admission quality (cheap prefilter/ranking) rather than global bound retuning, because the current hard surface behavior is dominated by a steep cost cliff.

## Post-hoc process caveat (added 2026-04-14)

Later on 2026-04-14, we identified lingering `search` processes from earlier `timeout cargo run ...` probes. Runtime/elapsed comparisons in this note may therefore include contention noise; treat timing deltas as provisional and revalidate critical runtime claims with `timeout -k ... target/dist/search`. Lag/outcome classifications are still the hard signal.
