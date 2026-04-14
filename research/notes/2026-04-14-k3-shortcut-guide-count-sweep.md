# k=3 shortcut-search guide-count sweep and budget concentration (2026-04-14)

## Question

Does concentrating the shortcut working set (`shortcut_max_guides`) improve k=3 progress under fixed segment-attempt budgets?

## Context

The active hard configuration (`max_intermediate_dim=5`, `max_entry=6`, `guided_max_shortcut_lag=4`) has become timeout-prone in this environment. I first bounded aggressively, then switched to a stable measurement surface (`max_intermediate_dim=4`, `max_entry=5`, `guided_max_shortcut_lag=3`) for a controlled sweep.

## Hard-config timeout check (measurement only)

Config:

- `max_intermediate_dim=5`, `max_entry=6`
- `guided_max_shortcut_lag=4`, `guided_min_gap=2`, `guided_max_gap=6`, `guided_rounds=2`
- `shortcut_rounds=2`, attempts `64`
- guides in `{4,8,12}`

All three runs timed out at `180s` with no completed JSON artifacts.

Interpretation:

- The current hard surface is still dominated by expensive segment searches.
- Guide-count policy tuning alone is not enough on this surface without stronger admission/prefiltering.

## Stable sweep (dim4/entry5)

Config:

- `max_intermediate_dim=4`, `max_entry=5`
- `guided_max_shortcut_lag=3`, `guided_min_gap=2`, `guided_max_gap=4`, `guided_rounds=1`
- `shortcut_rounds=2`
- guides in `{4,8,12}`

### Attempts = 96

- guides `4`: lag `7`, improved `14`, promoted `2`, cache hits/misses `28/68`, frontier `605`, visited `31890`, rounds completed `2`
- guides `8`: lag `7`, improved `14`, promoted `2`, cache hits/misses `25/71`, frontier `623`, visited `32705`, rounds completed `1`
- guides `12`: same as guides `8`

Signal:

- Concentrating to 4 guides improved efficiency slightly at equal lag and equal local improvements.

### Attempts = 128 (A/B: guides 4 vs 12)

- guides `4`: lag `7`, improved `14`, promoted `2`, cache hits/misses `57/71`, frontier `614`, visited `32522`, rounds completed `2`
- guides `12`: lag `7`, improved `22`, promoted `3`, cache hits/misses `31/97`, frontier `707`, visited `36079`, rounds completed `1`

Signal:

- Wider working set (12) buys more local shortcut improvements but still no lag `<7` witness.
- Narrower working set (4) is cheaper but appears too conservative at this budget.

## Conclusion

No guide-count setting in this sweep broke the lag-7 plateau. The best immediate tradeoff depends on objective:

- efficiency: favor lower `max_guides` (4),
- local-improvement throughput: favor higher `max_guides` (12).

For Goal 2, guide-count tuning alone is insufficient.

## Next Hypothesis

Use a staged policy instead of a fixed `max_guides`: keep a wide first pass (to harvest high-yield improvements), then narrow later passes to exploit cache-heavy refinement. Pair that with a cheap segment admission prefilter to make the hard (`dim5/entry6`) surface complete reliably.
