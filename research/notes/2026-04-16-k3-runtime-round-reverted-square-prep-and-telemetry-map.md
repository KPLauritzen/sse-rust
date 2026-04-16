# k=3 runtime round (reverted): square `2x3` preparation and telemetry-map hit fast path (2026-04-16)

## Question
After the kept `4x4` cofactor and structured `3x3` adjugate wins, is there one more bounded low-level runtime or bookkeeping cut worth keeping on the current solver stack?

## Fresh profile evidence

Rebuilt-binary `pprof` reruns on current `HEAD` (`2d76902`) still pointed at two nearby low-level candidates:

- Mixed endpoint-search control (`1,3,2,1 -> 1,6,1,1`, `--search-mode mixed`, `max_lag=6`, `max_intermediate_dim=3`, `max_entry=6`) remained dominated by `square_factorisation_3x3`:
  - factorisations enumerated `354,093`
  - candidates generated `354,221`
  - visited `5,141`
  - sampled stacks again concentrated in `enumerate_square_factorisation_3x3_family`, especially `solve_nonneg_2x3_into`
- Bounded hard shortcut control (`shortcut-search`, attempts `4`, `dim=5`, `entry=5`, `lag=7`, normalized guide pool) still split between `square_factorisation_3x3` and structured sparse families:
  - factorisations enumerated `336,161`
  - candidates generated `350,249`
  - visited `14,669`
  - top generated families: `square_factorisation_3x3` `148,778`, `binary_sparse_rectangular_factorisation_4x4_to_5` `119,640`, `binary_sparse_rectangular_factorisation_3x3_to_4` `49,176`
  - sampled stacks included `move_family_telemetry_mut` `124` times and `solve_nonneg_2x3_into` `169` times

## Attempt 1

Localized the earlier prepared-`2x3` idea to the square `3x3` family only:

- added a prepared `2x3` pivot or null-space helper in `src/factorisation.rs`
- used it only inside `enumerate_sq3_from_row0` and the research-only square breakdown helper
- left the generic `solve_nonneg_2x3_into` path unchanged

Direct controls looked strong:

- mixed control: wall `0.47s` with identical telemetry to current `HEAD`
- hard attempts-8 control:
  - baseline from the current kept round: wall `6.49s`, lag `7`, guide improvements/promotions `0/0`
  - localized prepared path: wall `6.28s`, lag `7`, guide improvements/promotions `0/0`

Aggregate confirmation still failed:

- current kept harness baseline: `research/runs/sse-rust-t8r-3x3-adjugate-r2.json`
  - required `23/23`, target hits `22`, points `3795`, telemetry-focus `69,496,257`, elapsed `22,951 ms`
- localized-prepared harness reruns:
  - `tmp/sse-rust-2ve-local-prep-harness.json`: elapsed `23,197 ms`
  - `tmp/sse-rust-2ve-local-prep-harness.r2.json`: elapsed `23,242 ms`

The regressions concentrated in broad mixed-search cases rather than the targeted hard shortcut surface, including `brix_ruiz_k3_graph_only` and `riedel_baker_k10`.

## Attempt 2

Tried a pure bookkeeping fast path in `src/search.rs`:

- changed `move_family_telemetry_mut` to check `contains_key` / `get_mut` before falling back to `entry(family.to_string())`
- goal: avoid allocating a fresh `String` on hot telemetry-map hits while keeping the serialized output shape unchanged

Direct controls again stayed favorable:

- mixed control: wall `0.45s`, telemetry unchanged
- hard attempts-8 control: wall `6.33s`, lag `7`, telemetry unchanged from the current kept round

Aggregate harness stayed slightly above the current kept baseline across three reruns:

- `tmp/sse-rust-2ve-telemetry-map-harness.json`: elapsed `23,076 ms`
- `tmp/sse-rust-2ve-telemetry-map-harness.r2.json`: elapsed `23,004 ms`
- `tmp/sse-rust-2ve-telemetry-map-harness.r3.json`: elapsed `23,082 ms`

The gap was small, but it never crossed below the retained `22,951 ms` reference, so this still did not satisfy the keep gate.

## Decision

Reverted both attempts.

- Attempt 1 improved the targeted square-family control but made the aggregate harness measurably worse.
- Attempt 2 looked like a real bookkeeping cut on the hard surface, but the full harness stayed slightly slower on every rerun.

No code was kept from this round.

## Follow-up

The real remaining low-level lead is a broader internal telemetry accumulator refactor:

- keep public JSON telemetry keyed by `String`
- use a borrowed or indexed family accumulator internally during search expansion so hot-path telemetry updates never allocate or clone family labels

That is a separate bounded task and should be tracked independently instead of broadening this reverted round.
