# k=3 shortcut-search gap-focus A/B (2026-04-14)

## Question

Does tightening guided segment selection toward larger gaps improve progress on the hard Brix-Ruiz `k=3` shortcut-search plateau (best known lag `7`)?

## Context

The prior loop showed that simple larger-gap-first ordering did not help and increased work. This loop tested a stronger gap-focus policy through existing CLI knobs:

- control: `guided-min-gap=2`, `guided-max-gap=6`
- focused: `guided-min-gap=3`, `guided-max-gap=7`

Common settings:

- endpoints: `A=[[1,3],[2,1]]`, `B=[[1,6],[1,1]]`
- stage: `shortcut-search`
- guide pool: `research/guide_artifacts/k3_normalized_guide_pool.json`
- bounds: `max_intermediate_dim=5`, `max_entry=6`
- guided: `max_shortcut_lag=4`, `rounds=2`
- shortcut: `max_guides=12`, `rounds=2`

## Evidence

Primary run pair (`max_total_segment_attempts=128`):

- control run completed (`tmp/k3_shortcut_loop2_control.json`)
- focused run timed out at 300s (`tmp/k3_shortcut_loop2_gapfocused.json` invalid/partial JSON)

Control (`128 attempts`) metrics:

- outcome: `equivalent`
- lag: `7`
- guided segments improved: `11`
- promoted guides: `2`
- stop reason: `max_segment_attempts_reached`
- frontier nodes expanded: `20088`
- total visited nodes: `1263782`

Secondary comparable pair (`max_total_segment_attempts=64`), both completed:

- control64 (`tmp/k3_shortcut_loop2_control64.json`)
  - lag `7`, improved `3`, promoted `1`
  - frontier nodes expanded `10882`
  - total visited nodes `712040`
- focused64 (`tmp/k3_shortcut_loop2_gapfocused64.json`)
  - lag `7`, improved `4`, promoted `2`
  - frontier nodes expanded `16592`
  - total visited nodes `1135613`

`64-attempt` interpretation:

- focused gap policy increased local shortcut activity counts,
- but did not improve best lag,
- and increased overall search work substantially (`+5710` expanded nodes, `+423573` visited nodes).

Hard-gate harness check for the loop:

- `just research-json-save baseline`
- `just research-json-save 2026-04-14-loop2-gapfocus-ab`
- required cases: `24/24 -> 24/24`
- target hits: `21 -> 21`
- total points: `3645 -> 3645`
- telemetry-focus score: `45802619 -> 45802619`

## Conclusion

Gap tightening (`min_gap=3,max_gap=7`) is not a useful direction for the current k=3 shortcut plateau. It preserves lag 7 and tends to increase work; at the 128-attempt budget it did not complete within 300 seconds.

## Next Steps

Try the opposite direction: keep broader gap coverage, but add lightweight per-segment admission based on endpoint invariants or coarse signatures before invoking expensive segment search, so attempts are filtered by plausibility rather than by gap size alone.
