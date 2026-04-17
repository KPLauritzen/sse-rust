# Exact orbit representatives for `binary_sparse_rectangular_factorisation_3x3_to_4` (2026-04-17)

## Question

For bead `sse-rust-veu`, is there a neighboring structured factorisation family
that can enumerate one exact symmetry representative per orbit earlier than the
current canonical-successor dedup, without changing reachable search behavior or
witness reconstruction?

The bounded target was one of:

- one additional exact orbit seam worth keeping,
- one durable rejection with a concrete reason,
- or a note explaining why the landed early-dedup seam for
  `square_factorisation_3x3` does not generalize cleanly.

## Control

I started from the same fixed mixed-endpoint profiling surface as
`profile_square_factorisation_sources`, but lifted `max_intermediate_dim` from
`3` to `4` so the `3x3 -> 4x4` families are actually present:

- source `[[1,3],[2,1]]`
- target `[[1,6],[1,1]]`
- `max_lag=6`, `max_intermediate_dim=4`, `max_entry=6`,
  `move_family_policy=mixed`

Research command:

```bash
cargo run --features research-tools --bin profile_structured_factorisation_orbits --quiet > tmp/2026-04-17-veu-profile-structured-factorisation-orbits.txt
```

Saved output:

- [tmp/2026-04-17-veu-profile-structured-factorisation-orbits.txt](../../tmp/2026-04-17-veu-profile-structured-factorisation-orbits.txt)

## Observation 1: `binary_sparse_rectangular_factorisation_3x3_to_4` has a real exact orbit

This family is not symmetric under all `S4` permutations of the intermediate
basis. It has:

- one distinguished slot whose column in `U` may be weighted and whose row in
  `V` must stay binary-sparse;
- three core slots whose columns in `U` must stay binary-sparse and whose rows
  in `V` stay in the weighted sparse vocabulary with at most one non-binary
  core row.

So the exact orbit is:

- simultaneous intermediate-basis renamings `(U', V') = (U P, P^{-1} V)`
- **only when** the renamed pair still lies in the same binary-sparse family.

That is narrower than the plain `square_factorisation_3x3` permutation action,
but it is still exact:

- `U'V' = UV`
- `V'U' = P^{-1}(VU)P`
- so `V'U'` has the same canonical successor as `VU`

## Measurement

On the fixed mixed control, the family is active enough to matter:

- `candidates=257736`
- `after_pruning=1790`
- `discovered=1543`

Representative hotspot rows from
[tmp/2026-04-17-veu-profile-structured-factorisation-orbits.txt](../../tmp/2026-04-17-veu-profile-structured-factorisation-orbits.txt):

- `[0,6,1] [0,1,1] [0,6,1]`: `raw=2448`, `orbit=87`, `canon=65`
- `[1,1,5] [1,0,1] [1,0,1]`: `raw=1440`, `orbit=48`, `canon=48`
- `[1,0,6] [1,0,1] [1,0,1]`: `raw=2712`, `orbit=87`, `canon=80`
- `[0,1,0] [1,1,1] [1,5,1]`: `raw=1368`, `orbit=38`, `canon=36`

The direct family sample from the existing Baker-step-2 witness source also
shows a large exact collapse:

- `[1,2,2] [2,1,1] [1,0,0]`: `raw=768`, `orbit=22`, `canon=20`

So the family emits many raw callbacks that differ only by provably irrelevant
intermediate-basis renaming inside the same structured vocabulary.

## Kept change

- Added `binary_sparse_factorisation_3x3_to_4_orbit_key(u, v, max_entry)` in
  [src/factorisation.rs](../../src/factorisation.rs).
- The key minimizes the witness pair over all intermediate-slot permutations
  whose renamed pair still satisfies the exact
  `binary_sparse_rectangular_factorisation_3x3_to_4` family constraints.
- [src/search/frontier.rs](../../src/search/frontier.rs) now keeps a per-source
  `HashSet` of these orbit keys and drops repeated orbit-equivalent raw
  callbacks before `v.mul(&u)`.

This is the same kind of local exact seam as the landed
`square_factorisation_3x3` dedup:

- it does not change the later canonical-successor dedup,
- it does not change witness reconstruction,
- it only avoids materializing repeated raw family witnesses that already live
  in one exact orbit.

Bounded validation on the fixed mixed control kept the same top-level search
totals after landing the seam:

- `layers=6`
- `factorisations=998208`
- `candidates_after_pruning=629050`
- `discovered=608543`

## Observation 2: the obvious row-split symmetry is already consumed locally

For `single_row_split_3x3_to_4x4`, the only clear exact symmetry is swapping the
two clone rows of the split pair. The enumerator already quotients that orbit by
keeping only `split <= twin`.

Direct sample:

- `[2,1,1] [1,0,2] [0,1,1]`
- `raw_unquotiented=16`
- `twin_orbit=8`
- `kept=8`
- `exact=8`
- `canon=8`

So this family already emits one representative per mirror-orbit of the split
pair. There is no comparable extra plain permutation orbit left to harvest in
the frontier without moving to a more family-specific analysis.

The fixed mixed control produced no `single_row_split_3x3_to_4x4` hits on this
endpoint pair, so this stays a structural note rather than a search-hotspot
optimization claim.

## Why the `square_factorisation_3x3` seam does not generalize cleanly

The landed `square_factorisation_3x3` seam works because every middle-basis
permutation is admissible and exact, so the orbit key is a plain family-wide
`S3` quotient.

The neighboring structured families split into two different cases:

- `binary_sparse_rectangular_factorisation_3x3_to_4` does have an exact orbit,
  but only under **family-preserving** permutations that respect the
  distinguished-slot vocabulary.
- `single_row_split_3x3_to_4x4` already consumes its only obvious exact orbit
  inside the enumerator itself via `split <= twin`.

So there is no one clean "apply the `square_factorisation_3x3` permutation seam
everywhere" rule. The surviving generalization is family-specific exact orbit
recognition, not a broad generic support/profile quotient.

## Validation

```bash
cargo test -q binary_sparse_factorisation_3x3_to_4_orbit_key -- --test-threads=1
cargo test -q test_binary_sparse_factorisations_reach_baker_step_2 -- --test-threads=1
cargo test -q test_expand_frontier_layer_deduplicates_canonical_successors -- --test-threads=1
cargo test -q test_search_telemetry_mixed_expand_case_repeats_cleanly -- --test-threads=1
```

All four passed.

## Verdict

Keep the `binary_sparse_rectangular_factorisation_3x3_to_4` early orbit-dedup
seam.

Also keep the row-split note: its obvious exact mirror symmetry is already
handled locally, so it does not supply a second analogous frontier seam.
