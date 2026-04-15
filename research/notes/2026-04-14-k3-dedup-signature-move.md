# k=3 runtime micro-cut: move same-future signatures instead of cloning during dedup (2026-04-14)

## Question
After loop31's cofactor reuse, can we trim `deduplicate_expansions` overhead by removing `SameFuturePastSignature` clone churn seen in profile stacks?

## Change

In `src/search.rs` `deduplicate_expansions`:

- switched loop binding to `for mut expansion in expansions`,
- replaced borrowed signature + `clone()` insertion with ownership transfer via `expansion.same_future_past_signature.take()`,
- inserted the moved signature directly into `same_future_past_seen`.

Behavioral intent is unchanged: same-future representative filtering still applies only when enabled and dim `>=3`; signatures are not used after dedup.

## Correctness + score gate

- `cargo test -q`: pass.
- `cargo test -q -- --test-threads=1`: pass.
- Harness A/B:
  - loop31 (`research/runs/loop31-cofactor-reuse.json`): required `24/24`, hits `21`, points `3645`, telemetry-focus `45,802,619`, elapsed `13581 ms`
  - loop32 (`research/runs/loop32-dedup-signature-move.json`): required `24/24`, hits `21`, points `3645`, telemetry-focus `45,802,619`, elapsed `13535 ms`

## Hard-surface probe

Config: `1,3,2,1 -> 1,6,1,1`, stage `shortcut-search`, `mixed`, dim5, entry5, lagcap5, gap2..5, timeout3, guides8, attempts168, cap260.

- `tmp/loop32_dim5_lagcap5_gap5_a168_mixed_entry5_g8_t3.json`
  - outcome equivalent lag `7`
  - factorisations `12,789,994`, visited `477,752`
  - same-future collisions `141,757`
  - improvements/promoted `20/3`
  - wall `228.88s`

Compared with loop31 reference (same counters, `227.65s`), this surface is effectively neutral.

## Decision

Keep as a small runtime win: correctness and objective metrics are unchanged, harness elapsed improved modestly, and the change reduces clone churn on a measured path.

## Next hypothesis

Profile again on the hard dim5 shortcut surface and target the remaining solver hot stacks (`solve_overdetermined_5x4` and structured sparse factorisation loops), then retest strict-cap boundary (`attempts176` at cap240).
