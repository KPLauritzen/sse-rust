# k=3/k=4 runtime cut: 4x4 cofactor reuse in structured sparse factorisation loops (2026-04-14)

## Question
Can we reduce the hard-surface factorisation cost by avoiding repeated 4x4 cofactor recomputation and per-call solve allocations in the structured `4x4<->5x5` sparse-factorisation loops?

## Change

In `src/factorisation.rs`:

- added `cofactor_matrix_and_det_4x4` to compute the 4x4 cofactor matrix and determinant once,
- added `solve_nonneg_4x4_with_cofactors` (non-allocating, unique-solution path),
- rewired the nonsingular branch of `solve_nonneg_4x4` to use that helper,
- rewired both structured hot loops
  - `visit_binary_sparse_factorisations_5x5_to_4`,
  - `visit_binary_sparse_factorisations_4x4_to_5`,
  to precompute cofactors once per candidate core matrix and solve each RHS column via the non-allocating helper.

## Correctness gate

- `cargo test -q`: pass (`202/202`, `15/15`).
- `cargo test -q -- --test-threads=1`: pass.
- Harness snapshot:
  - baseline: required `24/24`, target hits `21`, points `3645`, telemetry-focus `45,802,619`, elapsed `16046 ms`
  - `loop31-cofactor-reuse`: required `24/24`, target hits `21`, points `3645`, telemetry-focus `45,802,619`, elapsed `13581 ms`

## Hard k=3 boundary probes (kept surface)

Config: endpoint `1,3,2,1 -> 1,6,1,1`, `stage=shortcut-search`, `mixed`, `max_intermediate_dim=5`, `max_entry=5`, `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=5`, `guided_segment_timeout=3`, `shortcut_max_guides=8`, `shortcut_rounds=2`, strict direct binary execution (`timeout -k`).

- attempts `160` (`tmp/loop31_dim5_lagcap5_gap5_a160_mixed_entry5_g8_t3.json`):
  - outcome equivalent lag `7`, improvements/promoted `20/3`
  - factorisations `12,512,442`, visited `474,120`
- attempts `168` (`tmp/loop31_dim5_lagcap5_gap5_a168_mixed_entry5_g8_t3.json`):
  - outcome equivalent lag `7`, improvements/promoted `20/3`
  - factorisations `12,789,994`, visited `477,752`
  - wall `227.65s` under cap `260s`
- attempts `176`:
  - cap `260s` (`tmp/loop31_dim5_lagcap5_gap5_a176_mixed_entry5_g8_t3.json`): completed in `240.77s`, equivalent lag `7`, improvements/promoted `20/3`, factorisations `13,472,156`, visited `494,954`
  - strict cap `240s` (`tmp/loop31_dim5_lagcap5_gap5_a176_mixed_entry5_g8_t3_cap240.json`): timeout (`124`, empty JSON)

Net: no lag movement (`7`), but lower runtime pressure around the previous timeout cliff.

## Goal 3 k=4 envelope checks

Endpoint `1,4,3,1 -> 1,12,1,1`, `mixed + beam64 + dim5`:

- lag `16`, entry `10`, cap `120s` (`tmp/loop31_k4_mixed_beam64_dim5_lag16_entry10_cap120.json`): timeout (`124`, empty JSON).
- lag `14`, entry `12`, cap `120s` (`tmp/loop31_k4_mixed_beam64_dim5_lag14_entry12_cap120.json`): unknown, completed in `112.40s`, telemetry matched prior envelope (`factorisations 1,865,003`, visited `76,817`, frontier `1,666`).

No k4 witness found.

## Decision

Keep the optimization. It is correctness-neutral and improves runtime surfaces (harness elapsed drop and better near-cliff completion behavior) without regressing score gates.

## Next hypothesis

Keep pushing the same hotspot family: reduce remaining allocation churn on the structured 4x4 sparse path (currently still dominated by factorisation/solve stacks in profile), then re-test whether strict `240s` can complete attempts `176+` on the hard k=3 surface.
