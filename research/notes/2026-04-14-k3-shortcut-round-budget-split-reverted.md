# k=3 shortcut-search round-budget split (reverted) — 2026-04-14

## Question

Would splitting `max_total_segment_attempts` across shortcut rounds (instead of allowing round 0 to consume all attempts) improve lag-search behavior by forcing promoted-guide follow-up rounds?

## Change tested

In `search_shortcut_search_with_observer`, allocate a per-round attempt budget:

- `round_budget = ceil(remaining_attempts / rounds_remaining)`
- consume segment attempts against `round_budget`
- carry unused budget to later rounds

This was tested as a temporary code change in `src/search.rs` and then evaluated.

## Evidence

### Correctness gate

Harness run with the patch:

- `required_cases`: `24/24`
- `target_hits`: `21`
- `total_points`: `3645`
- `telemetry_focus_score`: `45802619`

So the hard gate did not regress.

### Targeted stable probes (dim4/entry5)

Config shared across probes:

- `max_intermediate_dim=4`, `max_entry=5`
- `guided_max_shortcut_lag=3`, `guided_min_gap=2`, `guided_max_gap=4`, `guided_rounds=1`
- `shortcut_max_guides=12`, `shortcut_rounds=2`

#### Attempts = 96

Before patch (`tmp/loop7_guides12_a96_dim4e5.json`):

- lag `7`
- segments considered/improved: `96/14`
- promoted guides: `2`
- rounds completed: `1` (budget exhausted in round 0)
- frontier/visited: `623` / `32705`

With patch (`tmp/loop8_post_roundbudget_guides12_a96_dim4e5.json`):

- lag `7`
- segments considered/improved: `66/2`
- promoted guides: `1`
- rounds completed: `2`
- stop reason: `guide_pool_exhausted`
- frontier/visited: `570` / `30217`

#### Attempts = 128

Before patch (`tmp/loop7_guides12_a128_dim4e5.json`):

- lag `7`
- segments considered/improved: `128/22`
- promoted guides: `3`
- rounds completed: `1`
- frontier/visited: `707` / `36079`

With patch (`tmp/loop8_post_roundbudget_guides12_a128_dim4e5.json`):

- lag `7`
- segments considered/improved: `100/7`
- promoted guides: `2`
- rounds completed: `2`
- stop reason: `guide_pool_exhausted`
- frontier/visited: `599` / `31833`

Interpretation:

- The split does force later rounds, but it reduces local improvement throughput and does not improve best lag.

### Hard-surface timeout check

Hard probe with patch:

- `max_intermediate_dim=5`, `max_entry=6`, `guided_max_shortcut_lag=4`, `attempts=64`, `max_guides=12`
- command timed out at `180s` (exit `124`), same practical outcome as pre-patch hard probes.

## Decision

Reverted. The patch adds policy complexity without moving Goal 2:

- no lag `<7` witness,
- no hard-surface completion improvement,
- lower local improvement throughput on the stable probe surface.

## Next hypothesis

Keep round semantics unchanged and focus on segment admission quality (cheap prefilter/ranking before expensive segment searches) so the hard `dim5/entry6` surface can complete more reliably.
