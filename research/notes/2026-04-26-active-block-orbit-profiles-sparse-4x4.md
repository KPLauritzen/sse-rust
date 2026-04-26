# Sparse `4x4` active-block orbit profiles (2026-04-26)

## Question

For bead `sse-rust-nw7.11`, test whether sparse active-block
automorphism/orbit/stabilizer profiles distinguish the retained Brix-Ruiz
`k = 4` near-hit layouts from true local reuse controls, beyond entrywise
distance, row/column sums, support profiles, trimmed active windows, and the
existing endpoint-local parity descriptors.

This is diagnostic-only:

- no solver scoring, pruning, canonicalization, or move generation changes;
- no generic `4x4` factorisation enumeration;
- no weighted WL/color-refinement follow-through; and
- no spectral or nerve descriptors.

## Active-block graph model

Model: `weighted_bipartite_active_block`.

For each selected square matrix:

1. delete all-zero rows and all-zero columns;
2. create row vertices `R_i` and column vertices `C_j` with row/column vertex
   colors kept distinct;
3. add one edge `R_i -> C_j` for every nonzero active-block entry;
4. label each edge by the exact entry value; and
5. brute-force row/column permutations in `S_r x S_c` to compute the
   stabilizer, row orbits, column orbits, nonzero-edge orbits, and pair
   transporters.

The largest selected active block is `4x4`, so each stabilizer/transporter scan
checks at most `4! * 4! = 576` row/column permutation pairs.

The report also includes a support-shadow quotient of the same model, replacing
each positive edge label by `1`. This is not a second graph-invariant proposal;
it is a calibration field to show when the exact-weight profile is only
refining the already-known support descriptor.

Research-only helper:

- `src/bin/diagnose_active_block_orbit_profiles.rs`

Reproducible command:

```bash
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_active_block_orbit_profiles -- \
  --json-out tmp/sse-rust-nw7-11-active-block-orbit-profiles.json
```

## Exact controls used

Retained Brix-Ruiz `k = 4` rank-4 pair:

- source surface:
  `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`
- frontier active block:
  `[[1,4,2,7],[3,1,0,6]]`
- counterpart active block:
  `[[1,12,0,1],[1,1,4,4]]`

Retained Brix-Ruiz `k = 4` rank-6 pair:

- source surface:
  `src/bin/diagnose_brix_ruiz_k4_active_block_switches.rs` rank-6 cluster
  fixture, carried from the retained stuck-state inventory;
- frontier active block:
  `[[2,3],[2,1],[11,0],[2,2]]`
- counterpart active block:
  `[[2,1],[1,4],[3,1],[11,0]]`

Baker/Lind-Marcus same-size `4x4` control:

- source artifact:
  `research/guide_artifacts/k3_shortcut_round1.json`
- control:
  path `matrices[4] -> matrices[5]`, the hard Baker `A4 -> A5` step;
- active blocks are full `4x4`.

Cheap true-reuse calibration:

- source artifacts:
  `research/guide_artifacts/k3_shortcut_round1.json` and
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`
- control:
  Baker/non-Baker replay-overlap step `2`, full `4x4`.

## Sample profiles

Orbit notation reports orbit sizes. Stabilizer sizes are within `S_r x S_c`.

| Sample | Active shape | Support stabilizer | Weighted stabilizer | Support row/col orbits | Weighted row/col orbits |
| --- | ---: | ---: | ---: | --- | --- |
| Brix rank-4 frontier | `2x4` | `6` | `1` | rows `1/1`, cols `3/1` | rows `1/1`, cols `1/1/1/1` |
| Brix rank-4 counterpart | `2x4` | `6` | `1` | rows `1/1`, cols `3/1` | rows `1/1`, cols `1/1/1/1` |
| Brix rank-6 frontier | `4x2` | `6` | `1` | rows `3/1`, cols `1/1` | rows `1/1/1/1`, cols `1/1` |
| Brix rank-6 counterpart | `4x2` | `6` | `1` | rows `3/1`, cols `1/1` | rows `1/1/1/1`, cols `1/1` |
| Baker `A4` | `4x4` | `1` | `1` | rows `1/1/1/1`, cols `1/1/1/1` | rows `1/1/1/1`, cols `1/1/1/1` |
| Baker `A5` | `4x4` | `2` | `2` | rows `1/1/1/1`, cols `1/1/2` | rows `1/1/1/1`, cols `1/1/2` |
| k3 Baker step-2 replay | `4x4` | `2` | `1` | rows `2/1/1`, cols `1/1/1/1` | rows `1/1/1/1`, cols `1/1/1/1` |
| k3 non-Baker step-2 replay | `4x4` | `2` | `1` | rows `1/2/1`, cols `1/1/1/1` | rows `1/1/1/1`, cols `1/1/1/1` |

## Pair transporters

`support transporters` count row/column permutations matching only support.
`weighted transporters` count exact active-block matches under the weighted
model. `min L1` is the best exact weighted L1 distance after independent
active-row/active-column permutation; it is reported only to interpret the
transporter result, not promoted as a new descriptor.

| Pair | Kind | Same coarse signature | Support transporters | Weighted transporters | min L1 over all row/col perms | min L1 over support transporters | Reading |
| --- | --- | --- | ---: | ---: | ---: | ---: | --- |
| Brix rank-4 frontier/counterpart | coarse-only near miss | yes | `6` | `0` | `16` | `16` | support orbit matches exactly, but exact weights shatter the support symmetry |
| Brix rank-6 frontier/counterpart | coarse-only near miss | yes | `6` | `0` | `4` | `4` | support orbit matches exactly; exact-weight profile again sees only a near miss |
| Baker `A4 -> A5` | known local transfer, not one current family | no | `0` | `0` | `5` | n/a | known same-size local transfer is invisible to this orbit profile |
| k3 replay step-2 overlap | known reuse calibration | yes | `2` | `1` | `0` | `0` | weighted transporter recognizes literal row/column reuse |

## Reading

The profile has two failure modes.

First, the support-shadow profile collapses the retained Brix rank-4 and rank-6
near-hit pairs exactly where the existing support/coarse descriptors already
collapse them. The nontrivial stabilizer is real but not new signal: it is just
the `2x4` or `4x2` active block with three interchangeable full-support columns
or rows. It cannot distinguish the retained near-hit from its opposite-side
counterpart.

Second, the exact-weight profile distinguishes the near-hits, but only by
destroying nearly all orbit structure. Both retained Brix pairs become
identity-stabilizer profiles with singleton weighted row/column/edge orbits.
That is too close to saying "the active windows differ entrywise" to justify a
new proposal bead.

The Baker control is the key negative calibration. `A4 -> A5` is the known hard
same-size `4x4` local transfer, but the active-block orbit profile has no
support or weighted transporter between the two full `4x4` blocks. So this
profile does not recognize the true local transfer that motivated the
comparison.

The only positive calibration is the cheap k3 replay-overlap pair: exact
weighted transporters detect literal row/column reuse. That is useful as a
sanity check, but it is not the missing Brix/Baker signal.

## Decision

Reject promotion. Do not open a follow-up bead from this slice.

Orbit/stabilizer structure does not provide useful diagnostic signal beyond the
existing descriptors on these controls:

- support-shadow orbit profiles collapse to the already-known support profile;
- weighted orbit profiles mostly collapse to identity stabilizers and singleton
  orbits;
- the profile misses the Baker `A4 -> A5` true local-transfer control; and
- the Brix rank-6 low row/column-permuted L1 value is a distance/layout fact,
  not an orbit/stabilizer fact, and overlaps the already-rejected active-block
  switch diagnostics.

If this area is revisited, it should be through an existing-word or
bridge-replay hypothesis, not through another orbit-profile descriptor.

## Validation

Focused validation run for this slice:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools \
  --bin diagnose_active_block_orbit_profiles
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_active_block_orbit_profiles -- \
  --json-out tmp/sse-rust-nw7-11-active-block-orbit-profiles.json
```

Observed result:

- formatting passed;
- helper tests passed (`4` tests);
- the bounded diagnostic emitted
  `tmp/sse-rust-nw7-11-active-block-orbit-profiles.json`; and
- the pair table above is reproduced from that JSON.
