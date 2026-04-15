# k=3 runtime experiment (reverted): overdetermined 5x4 fast path via 4x4 cofactors (2026-04-14)

## Question
After loop32, does bypassing `solve_nonneg_4x4` allocation overhead in `solve_overdetermined_5x4` improve hard-surface and harness runtime?

## Attempted change

Temporary patch in `src/factorisation.rs`:

- in `solve_overdetermined_5x4`, precomputed 4x4 cofactors/determinant per subset,
- used `solve_nonneg_4x4_with_cofactors` directly for nonsingular subsets,
- kept the original `solve_nonneg_4x4` path for singular subsets.

## Results

Correctness remained stable (`cargo test -q` pass), but performance did not improve:

- Hard k3 surface (`attempts=168`, dim5/entry5/lagcap5/gap<=5/timeout3/guides8):
  - telemetry unchanged (lag `7`, improvements/promoted `20/3`, factorisations `12,789,994`, visited `477,752`),
  - wall moved the wrong way: `228.88s` (loop32 ref) -> `230.78s`.

- Harness A/B:
  - loop32 (`research/runs/loop32-dedup-signature-move.json`): elapsed `13535 ms`
  - loop33 (`research/runs/loop33-overdet-fastpath.json`): elapsed `13563 ms`
  - required/target/points/telemetry-focus unchanged (`24/24`, `21`, `3645`, `45,802,619`).

## Decision

Reverted. The patch added complexity without moving Goal 2 or Goal 3 evidence and regressed runtime on both the hard probe and harness aggregate.

## Next hypothesis

Return to profile-led cuts in the still-dominant solver stacks (`solve_nonneg_4x4` and structured sparse factorisation paths), but only keep changes that improve both targeted hard-surface wall time and harness aggregate.
