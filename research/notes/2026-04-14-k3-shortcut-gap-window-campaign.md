# k=3 shortcut-search gap-window campaign on hard dim5 surface (2026-04-14)

## Question

Can gap-window policy (`guided_min_gap`, `guided_max_gap`) improve hard-surface tractability enough to move lag below 7?

## Context

Hard config baseline for this campaign:

- endpoints: `A=[[1,3],[2,1]]`, `B=[[1,6],[1,1]]`
- stage: `shortcut-search`
- guides: `research/guide_artifacts/k3_normalized_guide_pool.json`
- `max_intermediate_dim=5`, `max_entry=6`
- guided: `max_shortcut_lag=4`, `rounds=2`
- shortcut: `max_guides=12`, `rounds=2`

A temporary local telemetry patch (reverted after measurement) was used to break out segment attempts/improvements by gap and lag-cap.

## Measurement summary

### Baseline gap window (`min_gap=2`, `max_gap=6`), attempts=32

Artifact: `tmp/loop12_hist_dim5e6_a32.json`

- lag `7`
- guided improvements `1`
- frontier `4426`
- visited `277582`
- attempt distribution by gap: `2:9, 3:8, 4:6, 5:5, 6:4`
- only observed improvement: gap `5`

### Wider-gap concentration (`min_gap=4`, `max_gap=6`), attempts=32

Artifact: `tmp/loop12_hard_gapfocus_min4_a32.json`

- lag `7`
- guided improvements `2`
- frontier `9828`
- visited `634765`

Result: more local improvements, but much higher work.

### Narrower-gap concentration (`min_gap=2`, `max_gap=4`), attempts=32

Artifact: `tmp/loop12_hard_gapfocus_max4_a32.json`

- lag `7`
- guided improvements `0`
- frontier `968`
- visited `30089`

Result: major cost drop, but no immediate local improvements.

## High-budget narrow-gap campaign (`max_gap=4`)

Despite weaker per-attempt local progress at low budget, `max_gap=4` unlocked much larger attempt budgets without exploding runtime:

- attempts `128`: `tmp/loop12_hard_gapmax4_a128.json`
  - lag `7`, improvements `14`, frontier `1957`, visited `56746`
- attempts `256`: `tmp/loop12_hard_gapmax4_a256_t360.json`
  - lag `7`, improvements `35`, frontier `3742`, visited `87781`
- attempts `512`: `tmp/loop12_hard_gapmax4_a512_t600.json`
  - lag `7`, improvements `106`, frontier `6276`, visited `143160`

All completed, all remained lag `7`.

## Staged refinement follow-up from gap-4 best path

Stage 1 best guide was exported from the `max_gap=4`, attempts-128 run:

- guide artifact: `tmp/loop13_stage1_best_gap4_a128.json` (lag 7)

Stage 2, full gap window (`max_gap=6`) on this single best guide:

- artifact: `tmp/loop13_stage2_from_gap4best_a64_gap6.json`
- lag `7`
- guided improvements `0`
- stop reason `guide_pool_exhausted`

Increasing per-segment lag cap to 5 on the same stage-2 setup timed out at 240s for attempts 32 and 64:

- `tmp/loop13_stage2_from_gap4best_a32_gap6_lagcap5.json` (empty)
- `tmp/loop13_stage2_from_gap4best_a64_gap6_lagcap5.json` (empty)

## Conclusion

Gap-window tuning can dramatically change runtime profile, and `max_gap=4` gives a tractable high-attempt operating point on the hard dim5 surface. However, even with very large attempt budgets (up to 512), this campaign did not break the lag-7 plateau.

## Next hypothesis

Use the tractable `max_gap=4` campaign as a first pass, but add a targeted second-pass admission rule that selectively reintroduces expensive long-gap segments only for high-payoff candidates (rather than globally restoring `max_gap=6`).

## Post-hoc process caveat (added 2026-04-14)

Later on 2026-04-14, lingering `search` processes were discovered from earlier `timeout cargo run ...` probes. Treat runtime/elapsed and throughput comparisons in this note as potentially noisy under contention; rerun key timing claims using strict `timeout -k ... target/dist/search` execution. Witness/outcome and lag classifications remain the primary evidence.
