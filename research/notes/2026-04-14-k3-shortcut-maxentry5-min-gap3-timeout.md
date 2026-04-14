# k=3 shortcut: max_entry=5 min-gap-3 segment-selection check (2026-04-14)

## Question
On the rebuilt hard-dim5 baseline (`mixed + max_entry=5 + gap<=5`), does excluding gap-2 segments (`guided_min_gap=3`) reduce churn and improve tractability/progress?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`, `max_entry=5`
- `move_policy=mixed`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_max_shortcut_lag=5`, `guided_max_gap=5`
- `guided_segment_timeout=5`, `guided_rounds=2`
- `shortcut_rounds=2`, `shortcut_max_guides=8`
- outer cap `timeout -k 10s 240s`

Variant under test:

- `guided_min_gap=3` (baseline is `guided_min_gap=2`)

## Results

- `attempts=160` (`tmp/loop24_dim5_lagcap5_gap3to5_a160_mixed_entry5_g8.json`): timed out at 240s (`124`, empty JSON).
- `attempts=128` (`tmp/loop24_dim5_lagcap5_gap3to5_a128_mixed_entry5_g8.json`): timed out at 240s (`124`, empty JSON).

Baseline comparison (`guided_min_gap=2` on same bound family):

- `attempts=160`, guides 8 completed in 233s with lag `7`, improvements `20`, promoted `3` (`tmp/loop23_dim5_lagcap5_gap5_a160_mixed_entry5_g8.json`).

## Interpretation

- Raising minimum gap to 3 is a clear regression on this surface: it turns previously tractable budgets into timeouts.
- Gap-2 segments appear operationally important for keeping this mixed search tractable and/or productive.

## Decision

Do not keep min-gap-3 for the active hard-dim5 baseline.
