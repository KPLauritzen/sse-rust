# Bounded orbit exhaustion for `binary_sparse_rectangular_factorisation_4x3_to_3` (2026-04-18)

## Question

For bead `sse-rust-7qn`, can the new exact `S3` orbit representatives for
`binary_sparse_rectangular_factorisation_4x3_to_3` support one bounded
exhaustive/no-go step worth keeping, while staying entirely local to that
family?

The target was deliberately narrow:

- no generic certificate framework,
- no broader frontier rewrite,
- no family widening,
- just one exact local bounded probe on the same mixed control used to land the
  orbit seam.

## Bounded surface

Fixed mixed endpoint control, unchanged from
[2026-04-18-binary-sparse-4x4-to-3-orbit-representative-seam.md](2026-04-18-binary-sparse-4x4-to-3-orbit-representative-seam.md):

- source `[[1,3],[2,1]]`
- target `[[1,6],[1,1]]`
- `max_lag=6`
- `max_intermediate_dim=4`
- `max_entry=6`
- `move_family_policy=mixed`

The kept probe stays local to
`binary_sparse_rectangular_factorisation_4x3_to_3`:

1. run the fixed control once;
2. collect every `4x4` source matrix that the search actually expands through
   this family, together with side and depth;
3. for each such source, exhaust **all** one-step witnesses in this family;
4. quotient those witnesses by the landed exact
   `binary_sparse_factorisation_4x4_to_3_permutation_orbit_key`;
5. compare the resulting `3x3` canonical successors against the opposite
   frontier's final canonical depth map, using the exact remaining-lag budget
   `max_lag - (source_depth + 1)`.

Because the control exhausts its BFS envelope exactly, this gives an exact
bounded local no-go statement:

- within this fixed `lag<=6 / dim<=4 / entry<=6` surface,
- none of these one-step family-local successors can close against the opposite
  frontier if the counted lag-feasible hit set is empty.

## Kept tooling

Extended
[src/bin/profile_structured_factorisation_orbits.rs](../../src/bin/profile_structured_factorisation_orbits.rs)
with one additional sidecar section for
`binary_sparse_rectangular_factorisation_4x3_to_3`:

- record source side/depth for structured-family edges on the fixed control;
- record final visited canonical depth maps for forward/backward search;
- exhaust the one-step local family surface from every observed `4x4` source;
- report raw callbacks, exact orbit representatives, canonical successors, and
  lag-feasible opposite-frontier hits.

Saved output:

- [tmp/2026-04-18-7qn-profile-structured-factorisation-orbits.txt](../../tmp/2026-04-18-7qn-profile-structured-factorisation-orbits.txt)

## Result

All observed `binary_sparse_rectangular_factorisation_4x3_to_3` sources on the
fixed control lie at depth `2`, so the exact remaining opposite-side budget is
`3`.

The bounded surface contains:

- observed family-local `4x4` sources: `102`
- raw one-step family witnesses: `1409`
- exact orbit representatives: `428`
- canonical successor classes: `426`
- lag-feasible opposite-frontier hits: `0`

So the quotient cuts the bounded local work from `1409` raw witnesses to `428`
exact representatives, a `3.29x` reduction, while staying extremely close to
the eventual canonical-successor surface:

- `orbit = canon` on `101 / 102` observed sources;
- one exceptional source had `orbit=4`, `canon=2`:
  - `[0,0,0,0] [0,1,0,3] [5,2,0,1] [5,2,0,1]`

Representative bounded rows from the saved run:

- `[0,0,1,0] [5,0,5,1] [5,0,1,1] [5,0,1,1]`:
  `raw=29`, `orbit=6`, `canon=6`, `lag_feasible_hits=0`
- `[0,0,1,0] [5,0,6,1] [5,0,1,1] [5,0,1,1]`:
  `raw=29`, `orbit=6`, `canon=6`, `lag_feasible_hits=0`
- `[0,0,1,1] [4,0,5,6] [4,0,1,2] [0,0,1,1]`:
  `raw=22`, `orbit=6`, `canon=6`, `lag_feasible_hits=0`
- `[0,1,0,0] [0,1,1,1] [0,5,1,1] [0,1,0,0]`:
  `raw=22`, `orbit=6`, `canon=6`, `lag_feasible_hits=0`

## Read

This is the first bounded local no-go surface on this family that is worth
keeping.

Why:

- it is exact inside a clearly stated envelope;
- it uses the landed exact orbit key directly, not a heuristic signature;
- the quotient materially reduces the bounded work (`1409 -> 428`);
- on almost every observed source the orbit quotient is already as strong as
  the retained canonical surface (`101 / 102` sources with `orbit = canon`);
- the bounded certificate is concrete:
  across all `102` observed `4x4` sources on the fixed control, exhausting the
  family modulo the exact orbit produces **zero** lag-feasible hits into the
  opposite frontier.

This is still only a local bounded statement. It is **not** a general no-go for
the family or for the endpoint pair outside the stated envelope. But it is a
durable exact probe showing that the quotient is strong enough to support
bounded exhaustive/no-go reasoning on this family.

## Validation

Commands used:

```bash
cargo test -q test_expand_frontier_layer_deduplicates_binary_sparse_4x4_to_3_permutation_orbits -- --test-threads=1

timeout -k 10s 120s cargo run --features research-tools --bin profile_structured_factorisation_orbits --quiet > tmp/2026-04-18-7qn-profile-structured-factorisation-orbits.txt
```

The focused test passed, and the bounded profile completed with the same
top-level search totals as the prior seam note:

- `layers=6`
- `factorisations=1003272`
- `candidates_after_pruning=629050`
- `discovered=608543`

## Decision

Keep the tooling addition.

This round ends as a merge-ready local sidecar/profiling change, not a rejected
evidence-only note.
