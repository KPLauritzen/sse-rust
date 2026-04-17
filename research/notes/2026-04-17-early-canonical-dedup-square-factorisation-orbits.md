# Early canonical dedup via `square_factorisation_3x3` orbit keys (2026-04-17)

Question: can we keep one exact earlier dedup seam inside frontier expansion so repeated raw square-factorisation callbacks that only differ by middle-basis renaming do not pay the full retained frontier payload cost more than once?

## Control

- Fixed bounded mixed endpoint control from `src/bin/profile_square_factorisation_sources.rs`:
  - source `[[1,3],[2,1]]`
  - target `[[1,6],[1,1]]`
  - `max_lag=6`, `max_intermediate_dim=3`, `max_entry=6`, `move_family_policy=mixed`
- Command:

```bash
cargo run --features research-tools --bin profile_square_factorisation_sources --quiet > tmp/sse-rust-8le-profile-square-factorisation-sources.txt
```

- Search-level telemetry stayed unchanged after the frontier change:
  - `layers=6`
  - `factorisations=354093`
  - `candidates_after_pruning=7478`
  - `discovered=5139`

## Observation

The heavy-collapse sources are exactly the duplicated-row or duplicated-column `3x3` surfaces where `square_factorisation_3x3` can emit many witnesses related by a permutation of the intermediate basis.

Representative rows from `tmp/sse-rust-8le-profile-square-factorisation-sources.txt`:

- `[0,1,0] [2,2,3] [0,1,0]`: `raw_sq=3737`, `raw_orbit=802`, `raw_exact=1993`, `raw_canon=450`
- `[0,1,0] [1,1,1] [1,5,1]`: `raw_sq=1279`, `raw_orbit=279`, `raw_exact=913`, `raw_canon=178`
- `[1,1,5] [1,0,1] [1,0,1]`: `raw_sq=1331`, `raw_orbit=279`, `raw_exact=798`, `raw_canon=178`

Bucket aggregate:

- `sum=11,dup_rows=0,dup_cols=1`: `raw_callbacks=17589`, `raw_orbit=3784`, `raw_canon=2462`
- `sum=11,dup_rows=1,dup_cols=0`: `raw_callbacks=15189`, `raw_orbit=3240`, `raw_canon=2091`

So the bounded exact seam removes a large chunk of raw repeated witnesses before `VU` materialization, while still leaving the later canonical dedup responsible for non-permutation collisions.

## Kept change

- Added `square_factorisation_3x3_permutation_orbit_key(u, v)` in `src/factorisation.rs`.
- The key canonicalizes the witness pair under simultaneous intermediate-basis renaming:
  - `U' = U P`
  - `V' = P^{-1} V`
- This is exact because:
  - `U'V' = UV`
  - `V'U' = P^{-1}(VU)P`
  - therefore `V'U'` has the same canonical successor as `VU`
- `src/search/frontier.rs` now keeps a per-source `HashSet` of these orbit keys only for `square_factorisation_3x3`, and drops repeated orbit-equivalent raw callbacks before `v.mul(&u)`.
- Later `next_canon` dedup still runs unchanged, so reachable canonical successors, exact meets, witness reconstruction, and observer layer output stay on the existing exact path.

## Validation

```bash
cargo test -q square_factorisation_3x3_permutation_orbit_key -- --test-threads=1
cargo test -q expand_frontier_layer_deduplicates -- --test-threads=1
cargo test -q test_search_telemetry_mixed_expand_case_repeats_cleanly -- --test-threads=1
```

Results:

- the new orbit-key exactness test passed;
- the frontier dedup regression tests passed;
- the mixed search telemetry regression passed;
- the bounded control kept the same search-level totals while exposing the pre-materialization orbit collapse.

## Verdict

Keep it.

This is a small exact local reduction, not a broad frontier refactor. It is worth keeping because the adversarial mixed control shows substantial repeated raw `square_factorisation_3x3` witnesses collapsing to a much smaller orbit count before canonical successors, and the retained search behavior stayed unchanged on the bounded validation surface.
