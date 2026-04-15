# k=3 boundary retest after loop34 runtime trims (2026-04-14)

## Question
Did loop34 runtime trims move the strict-cap hard-surface boundary enough for attempts `176` to complete under cap `240s`?

## Setup

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `mixed`, `max_intermediate_dim=5`, `max_entry=5`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=5`
- `guided_segment_timeout=3`, `guided_rounds=2`
- `shortcut_max_guides=8`, `shortcut_rounds=2`, `shortcut_max_total_segment_attempts=176`
- direct binary runs via `timeout -k`

## Results

- strict cap `240s` (`tmp/loop34_dim5_lagcap5_gap5_a176_mixed_entry5_g8_t3_cap240.json`): timeout (`124`, empty JSON), elapsed `240.01s`.
- calibration cap `260s` (`tmp/loop34_dim5_lagcap5_gap5_a176_mixed_entry5_g8_t3_cap260.json`): completed in `240.87s`.

Completed run telemetry remained unchanged from earlier attempts-176 completions:

- outcome equivalent lag `7`
- factorisations `13,472,156`
- visited `494,954`
- improvements/promoted `20/3`

## Interpretation

Loop34 did not shift the strict 240s boundary; attempts-176 still sits just beyond the cap.

## Decision

Keep loop34 runtime trims for aggregate harness win, but do not treat them as a boundary breakthrough on Goal 2.
