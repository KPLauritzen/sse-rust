# Bounded orbit exhaustion for `binary_sparse_rectangular_factorisation_3x3_to_4` (2026-04-19)

## Question

For bead `sse-rust-e3g`, is there one additional hot family on the same bounded
mixed endpoint control where an already-known exact family-preserving orbit can
support a local bounded no-go surface worth keeping?

The slice stayed narrow:

- no generic certificate framework;
- no new competing family in the same round;
- no broad frontier rewrite;
- just one bounded follow-up using the existing
  `binary_sparse_factorisation_3x3_to_4_orbit_key`.

## Why this family

I first re-ran the fixed profiler surface used by the 2026-04-17 and
2026-04-18 orbit notes.

Current control:

- source `[[1,3],[2,1]]`
- target `[[1,6],[1,1]]`
- `max_lag=6`
- `max_intermediate_dim=4`
- `max_entry=6`
- `move_family_policy=mixed`

On this control the plausible follow-up families split cleanly:

- `single_row_split_3x3_to_4x4` is inactive here:
  `candidates=41674`, `after_pruning=0`, `discovered=0`
- `binary_sparse_rectangular_factorisation_3x3_to_4` is still the live hotspot:
  `candidates=257736`, `after_pruning=2015`, `discovered=1585`

So this round keeps the family choice local and justified: the next bounded
certificate probe should stay on
`binary_sparse_rectangular_factorisation_3x3_to_4`.

## Exact surface

The exact orbit is the one already established in
[2026-04-17-binary-sparse-3x3-to-4-orbit-representative-seam.md](2026-04-17-binary-sparse-3x3-to-4-orbit-representative-seam.md):

- simultaneous intermediate-basis renamings `(U', V') = (U P, P^{-1} V)`;
- only permutations `P` whose renamed pair still lies in the same exact
  `binary_sparse_rectangular_factorisation_3x3_to_4` family are admitted.

That orbit is exact because:

- `U'V' = UV`
- `V'U' = P^{-1}(VU)P`

So quotienting one-step witnesses by
`binary_sparse_factorisation_3x3_to_4_orbit_key` is an exact family-local
reduction before canonical successor comparison.

## Bounded surface

I extended the existing
[src/bin/profile_structured_factorisation_orbits.rs](../../src/bin/profile_structured_factorisation_orbits.rs)
sidecar with one more bounded pass for this family only:

1. run the same fixed control once;
2. collect every `3x3` source matrix that the search actually expands through
   `binary_sparse_rectangular_factorisation_3x3_to_4`;
3. exhaust all one-step witnesses in that family from each observed source;
4. quotient those witnesses by the exact family-preserving orbit key;
5. compare the resulting `4x4` canonical successors against the opposite
   frontier's final canonical depth map, using the exact remaining-lag budget
   `max_lag - (source_depth + 1)`.

This gives an exact bounded local statement inside the stated envelope:

- if the lag-feasible hit count is zero, then none of the observed one-step
  family-local successors can close against the opposite frontier within the
  fixed `lag<=6 / dim<=4 / entry<=6` surface.

## Result

Saved output:

- [tmp/2026-04-19-e3g-profile-structured-factorisation-orbits.txt](../../tmp/2026-04-19-e3g-profile-structured-factorisation-orbits.txt)

Bounded exhaustion summary from that run:

- observed family-local `3x3` sources: `298`
- raw one-step family witnesses: `199104`
- exact orbit representatives: `7351`
- canonical successor classes: `6406`
- lag-feasible opposite-frontier hits: `0`

So the exact quotient cuts the bounded local witness surface by about `27.09x`
before canonical comparison.

Representative bounded rows:

- `[1,0,6] [1,0,1] [1,0,1]`:
  `raw=2712`, `orbit=87`, `canon=80`, `lag_feasible_hits=0`
- `[0,5,1] [0,1,1] [0,6,1]`:
  `raw=2568`, `orbit=98`, `canon=76`, `lag_feasible_hits=0`
- `[0,6,1] [0,1,1] [0,6,1]`:
  `raw=2448`, `orbit=87`, `canon=65`, `lag_feasible_hits=0`
- `[0,1,7] [0,1,6] [0,1,1]`:
  `raw=2064`, `orbit=77`, `canon=55`, `lag_feasible_hits=0`

## Read

This surface is worth keeping, but for a narrower reason than the `4x3 -> 3`
bounded exhaustion from 2026-04-18.

What is strong:

- the orbit is exact, not heuristic;
- the family is actually hot on the fixed control;
- the bounded local hit set is empty across all `298` observed sources;
- the raw-to-orbit collapse is large enough to make exhaustive local checking
  tractable inside this envelope.

What is weaker than the earlier `4x3 -> 3` case:

- the orbit quotient is not nearly as close to the retained canonical surface;
- only `112 / 298` observed sources satisfy `orbit = canon`;
- total orbit classes still exceed canonical successors by `945`.

So the right interpretation is:

- this is a durable exact bounded no-go surface for the stated control;
- it is not evidence that the orbit quotient alone almost matches canonical
  collapse on this family.

## Validation

Commands used:

```bash
cargo test -q binary_sparse_factorisation_3x3_to_4_orbit_key -- --test-threads=1

timeout -k 10s 180s cargo run --features research-tools \
  --bin profile_structured_factorisation_orbits --quiet \
  > tmp/2026-04-19-e3g-profile-structured-factorisation-orbits.txt

cargo fmt --all
```

Focused test passed. The bounded profile completed with:

- `result=Unknown`
- `layers=6`
- `factorisations=1003521`
- `candidates_after_pruning=629057`
- `discovered=608550`

## Decision

Keep it.

The kept result is a bounded local certificate surface, not a broad solver
claim: within this exact fixed mixed envelope, exhausting
`binary_sparse_rectangular_factorisation_3x3_to_4` modulo its exact
family-preserving orbit produces zero lag-feasible opposite-frontier hits.
