# Exact orbit representatives for `binary_sparse_rectangular_factorisation_4x3_to_3` (2026-04-18)

## Question

For bead `sse-rust-2uy.30`, what is the next hot structured family on the
bounded mixed endpoint control where an **exact**, family-preserving orbit
quotient can drop repeated raw callbacks before `VU` materialization, and is
that seam worth keeping?

The slice stayed narrow:

- no broad frontier refactor,
- no generic orbit support layer,
- no move-family widening,
- just one family-local orbit check plus bounded measurement.

## Chosen family

I chose `binary_sparse_rectangular_factorisation_4x3_to_3`, the bounded
`4x4 -> 3x3` down-family exposed from `src/factorisation.rs`.

Why this family, not the hotter `diagonal_refactorization_4x4` lane:

- on the same fixed mixed control, `diagonal_refactorization_4x4` is active,
  but its witness form does **not** admit the clean middle-basis permutation
  action used by the landed seams;
- `binary_sparse_rectangular_factorisation_4x3_to_3` is still hot enough to
  matter and **does** admit the same exact orbit argument:
  `candidates=105446`, `after_pruning=133`, `discovered=8`.

## Exact symmetry argument

For this family the raw witnesses have shape

- `U : 4x3`, with every row binary-sparse of support size `1` or `2`;
- `V : 3x4`, with nonnegative entries bounded by `max_entry`.

So every permutation `P` of the `3`-dimensional intermediate basis preserves
the family:

- `U' = U P` still has binary-sparse rows, because permuting coordinates does
  not change row support size;
- `V' = P^{-1} V` just permutes rows of `V`, so nonnegativity and the entry
  bound are preserved.

This gives the same exact orbit relation as the landed square seam:

- `U'V' = UPP^{-1}V = UV`,
- `V'U' = P^{-1}(VU)P`.

Therefore orbit-equivalent raw callbacks have the same source matrix and a
permutation-similar successor, so dropping all but one representative per orbit
before `v.mul(&u)` is exact.

## Control

Fixed bounded mixed endpoint control, matching the earlier orbit notes:

- source `[[1,3],[2,1]]`
- target `[[1,6],[1,1]]`
- `max_lag=6`
- `max_intermediate_dim=4`
- `max_entry=6`
- `move_family_policy=mixed`

Research command:

```bash
cargo run --features research-tools --bin profile_structured_factorisation_orbits --quiet > tmp/2026-04-18-2uy30-profile-structured-factorisation-orbits.txt
```

Saved output:

- [tmp/2026-04-18-2uy30-profile-structured-factorisation-orbits.txt](../../tmp/2026-04-18-2uy30-profile-structured-factorisation-orbits.txt)

## Measurement

Control family totals from the saved run:

- `binary_sparse_rectangular_factorisation_4x3_to_3`:
  `candidates=105446`, `after_pruning=133`, `discovered=8`, `exact_meets=0`

Representative hotspot rows:

- `[0,0,1,0] [5,0,6,1] [5,0,1,1] [5,0,1,1]`:
  `raw=29`, `orbit=6`, `exact=29`, `canon=6`
- `[0,0,0,0] [1,0,1,1] [1,1,1,5] [1,0,1,1]`:
  `raw=18`, `orbit=6`, `exact=18`, `canon=6`
- `[0,0,0,0] [1,1,1,1] [0,5,0,0] [1,1,1,1]`:
  `raw=12`, `orbit=4`, `exact=12`, `canon=4`
- `[0,1,0,1] [5,0,1,1] [5,0,1,1] [0,1,0,1]`:
  `raw=18`, `orbit=5`, `exact=18`, `canon=5`

Direct family sample from the existing Baker-step-6 witness source:

- `[1,1,1,1] [3,0,2,2] [1,0,0,0] [0,1,1,1]`:
  `raw=12`, `orbit=2`, `exact=12`, `canon=2`

Read of those numbers:

- the raw callback volume really does collapse sharply under the exact orbit,
  typically by about `3x` to `6x` on the bounded hotspots;
- on the measured hotspots the orbit key is as strong as the eventual
  canonical-successor collapse: `orbit = canon` in every representative row
  above;
- later exact dedup still matters elsewhere, because `exact` can remain larger
  than `canon`, but the local orbit seam removes a substantial amount of
  repeated pre-materialization work.

## Kept change

- added
  `binary_sparse_factorisation_4x4_to_3_permutation_orbit_key(u, v)` in
  [src/factorisation.rs](../../src/factorisation.rs);
- [src/search/frontier.rs](../../src/search/frontier.rs) now keeps a per-source
  `HashSet` of these orbit keys for
  `binary_sparse_rectangular_factorisation_4x3_to_3` and skips repeated
  orbit-equivalent raw callbacks before `v.mul(&u)`;
- [src/bin/profile_structured_factorisation_orbits.rs](../../src/bin/profile_structured_factorisation_orbits.rs)
  now profiles this family on the same bounded control.

This stays inside the same exact local-seam pattern as the landed
`square_factorisation_3x3` and `binary_sparse_rectangular_factorisation_3x3_to_4`
changes:

- no generic orbit framework,
- no search-policy rewrite,
- no change to later canonical dedup or witness reconstruction.

## Search-level validation on the bounded control

The bounded control kept the same top-level search totals after landing the new
seam:

- `layers=6`
- `factorisations=1003272`
- `candidates_after_pruning=629050`
- `discovered=608543`

So the seam changes only the raw local work inside this family; it does not
change reachable canonical behavior on the fixed control.

## Bounded exhaustive / no-go read

This orbit key looks **promising** for bounded exhaustive-search or no-go work
on the same family.

Reason:

- the orbit is exact and finite (`S3`);
- the key is family-complete, not a heuristic signature;
- on the bounded hotspots above, `orbit = canon`, so quotienting by this orbit
  already matches the retained canonical collapse very closely.

That is not yet a certificate implementation, but it is a defensible base for
a later bounded “exhaust this family modulo exact orbit representatives” step.

## Decision

Keep it.

This is the next structured family on the bounded control that is both:

1. hot enough to matter locally, and
2. cleanly closed under an exact family-preserving orbit action.

The measured raw-to-orbit collapse is real, the exactness argument is simple,
and the bounded control kept the same search totals after the seam landed.
