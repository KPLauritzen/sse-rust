# k=3 shortcut: rebuild-validated mixed max_entry A/B on hard dim5 surface (2026-04-14)

## Question
After discovering a stale `target/dist/search` confounder, what does a rebuilt-binary A/B show on the hard dim5 lag-cap-5 shortcut surface, and is there a tractability lever that preserves local progress?

## Setup

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- stage `shortcut-search`
- `max_intermediate_dim=5`
- move policy `mixed` unless stated otherwise
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=5`
- `guided_segment_timeout=5`, `guided_rounds=2`
- `shortcut_rounds=2`, `shortcut_max_guides=12`
- outer cap `timeout -k 10s ...`
- execution via direct `target/dist/search`

Rebuild hygiene:

- Rebuilt binary before measurement: `cargo build --profile dist --features research-tools --bin search`.
- Pre-rebuild guide sweep artifacts (`loop21_dim5_lagcap5_gap5_*`) are treated as provisional; they were produced before this rebuild and showed inconsistent working-set telemetry.

## Results

### Rebuild validation and guide-count recheck

Rebuilt-binary attempts-96 guide sweep (`max_entry=6`, `timeout 180s`):

- `max_guides=3` (`tmp/loop21r_dim5_lagcap5_gap5_a96_g3.json`): lag `7`, improvements `10`, promoted `2`, factorisations `9,037,775`.
- `max_guides=6` (`tmp/loop21r_dim5_lagcap5_gap5_a96_g6.json`): lag `7`, improvements `10`, promoted `2`, factorisations `9,069,737`.
- `max_guides=12` (`tmp/loop21r_dim5_lagcap5_gap5_a96_g12.json`): lag `7`, improvements `10`, promoted `2`, factorisations `8,913,471`.

No lag movement; guide-count is not the active bottleneck on this surface.

### Move-policy check (rebuilt binary)

At attempts-96 (`max_entry=6`):

- `mixed` (`tmp/loop22_dim5_lagcap5_gap5_a96_mixed.json`): lag `7`, improvements `10`, factorisations `8,704,022`.
- `graph-only` (`tmp/loop22_dim5_lagcap5_gap5_a96_graph-only.json`): lag `7`, improvements `1`, factorisations `0`.

Graph-only is dramatically cheaper but under-produces local shortcut improvements at this budget.

### High-budget graph-only stress (for depth ceiling)

- `max_gap=5`, attempts `2048` (`tmp/loop22_dim5_lagcap5_gap5_a2048_graph-only.json`): lag `7`, improvements `169`, promoted `10`, stop `guide_pool_exhausted`, factorisations `0`.
- `max_gap=6`, attempts `2048` (`tmp/loop22_dim5_lagcap5_gap6_a2048_graph-only.json`): lag `7`, improvements `236`, promoted `10`, stop `guide_pool_exhausted`, factorisations `0`.
- `max_gap=7`, `guided_max_shortcut_lag=6`, attempts `4096` (`tmp/loop22_dim5_lagcap6_gap7_a4096_graph-only.json`): lag `7`, improvements `291`, promoted `10`, stop `guide_pool_exhausted`, factorisations `0`.

Even with cheap deep graph-only refinement, lag remains `7`.

### Mixed `max_entry` A/B (rebuilt binary)

Attempts-128 (`timeout 240s`):

- `max_entry=5` (`tmp/loop22_dim5_lagcap5_gap5_a128_mixed_entry5.json`): lag `7`, improvements `11`, promoted `3`, factorisations `11,227,660`, visited `487,095`.
- `max_entry=6` (`tmp/loop22_dim5_lagcap5_gap5_a128_mixed_entry6.json`): lag `7`, improvements `11`, promoted `3`, factorisations `12,240,338`, visited `700,884`.

Attempts-160 (`timeout 240s`):

- `max_entry=5` (`tmp/loop22_dim5_lagcap5_gap5_a160_mixed_entry5.json`): **completed**; lag `7`, improvements `20`, promoted `3`, factorisations `13,684,131`, visited `599,474`.
- `max_entry=6` (`tmp/loop22_dim5_lagcap5_gap5_a160_mixed_entry6.json`): **timed out** (`124`, empty JSON).

## Interpretation

- Rebuilt-binary runs confirm the key tractability lever on this surface is `max_entry=5` under mixed policy.
- At equal budget (`attempts=128`), `max_entry=5` preserves local progress while reducing work.
- At higher budget (`attempts=160`), `max_entry=5` stays tractable and increases local improvements, while `max_entry=6` times out under the same cap.
- Despite this tractability gain, best lag remains `7`.

## Next hypothesis

Use `mixed + max_entry=5` as the active hard-dim5 stage-2 baseline and spend the recovered budget on segment-selection quality (not raw guide-count). Candidate next step: targeted segment ordering/admission that prioritizes historically productive gap/endpoint bands under this cheaper bound.
