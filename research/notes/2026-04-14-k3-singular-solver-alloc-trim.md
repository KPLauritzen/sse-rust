# k=3 runtime trim: singular 3x3/4x4 solver allocation cleanup (2026-04-14)

## Question
Can we reduce solver overhead in singular fallback paths (`solve_nonneg_3x3` / `solve_nonneg_4x4`) without changing witness behavior?

## Change

In `src/factorisation.rs`:

- `solve_nonneg_3x3` (det=0 branch): reused a single vector by calling `solve_nonneg_2x3_into` and filtering in place (`retain`) instead of allocating a second results vector.
- `solve_nonneg_4x4` (det=0 branch): replaced per-iteration `Vec<usize>` column-subset construction with static `COL_SUBSETS` arrays.

No search semantics were intentionally changed.

## Correctness + score gate

- `cargo test -q`: pass.
- Harness A/B:
  - loop32 baseline (`research/runs/loop32-dedup-signature-move.json`): required `24/24`, hits `21`, points `3645`, telemetry-focus `45,802,619`, elapsed `13535 ms`
  - loop34 (`research/runs/loop34-singular-alloc-trim.json`): required `24/24`, hits `21`, points `3645`, telemetry-focus `45,802,619`, elapsed `13461 ms`

## Hard-surface probe

Config: `1,3,2,1 -> 1,6,1,1`, `shortcut-search`, mixed, dim5, entry5, lagcap5, gap2..5, timeout3, guides8, attempts168, cap260.

- `tmp/loop34_dim5_lagcap5_gap5_a168_mixed_entry5_g8_t3.json`
  - outcome equivalent lag `7`
  - factorisations `12,789,994`, visited `477,752`
  - improvements/promoted `20/3`
  - wall `228.94s`

Compared with loop32 reference (`228.88s`), this surface is effectively neutral.

## Decision

Keep. This is a correctness-neutral runtime win on harness aggregate with neutral hard-surface objective telemetry.

## Next hypothesis

Re-profile the hard dim5 shortcut surface and target the remaining dominant solver stacks (`solve_nonneg_4x4` + structured sparse enumeration), then retest strict-cap (`240s`) boundary behavior for attempts `176`.
