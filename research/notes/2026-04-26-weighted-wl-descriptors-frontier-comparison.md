# Weighted WL descriptors for frontier comparison (2026-04-26)

## Question

For bead `sse-rust-nw7.10`, test whether cheap Weisfeiler-Lehman style color
refinement on weighted matrix graphs gives useful frontier-comparison signal
beyond the existing `mass_support_signature`, `trimmed_active_window`, same
future/past telemetry, canonical permutation keys, and the rejected active-block
orbit/stabilizer profiles.

This is diagnostic-only:

- no solver scoring, pruning, canonicalization, or move generation changes;
- no generic `4x4` factorisation enumeration;
- no spectral, nerve/matching, or orbit/stabilizer follow-through; and
- no claim that WL equality or inequality is an SSE correctness invariant.

## Descriptor definitions

Research-only helper:

- `src/bin/diagnose_weighted_wl_descriptors.rs`

Reproducible command:

```bash
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_weighted_wl_descriptors -- \
  --json-out tmp/sse-rust-nw7-10-weighted-wl-descriptors.json
```

Two descriptors were tested, each at exactly `1`, `2`, and `3` rounds.

### Weighted active bipartite WL

For each matrix:

1. delete all-zero rows and all-zero columns;
2. create row vertices and column vertices with distinct initial colors `R`
   and `C`;
3. in each round, recolor each row by its previous row color plus the sorted
   multiset of `(entry weight, previous column color)` over nonzero row
   incidences;
4. recolor each column dually by `(entry weight, previous row color)`; and
5. use the sorted row-color histogram plus sorted column-color histogram as the
   descriptor for that round.

This is invariant under independent active-row and active-column permutations.
It keeps row/column structure explicit and folds exact edge weights into the
local color update.

### Directed weighted matrix WL

For square matrices only:

1. create one graph vertex per matrix index;
2. start every vertex with color `V`;
3. in each round, recolor each vertex by its previous color plus sorted
   outgoing and incoming multisets of `(entry weight, previous neighbor color)`
   over nonzero entries; and
4. use the sorted vertex-color histogram as the descriptor for that round.

This is invariant under permutation similarity, not independent row/column
permutation. It is included as a calibration descriptor because it tests whether
tying row and column identity obscures the useful active-block reading.

## Controls

The selected controls intentionally match the previous orbit-profile slice:

- retained Brix-Ruiz `k = 4` rank-4 sparse `4x4` near-hit pair from
  `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`;
- retained Brix-Ruiz `k = 4` rank-6 sparse `4x4` near-hit pair carried from the
  active-block switch rank-6 fixture;
- Baker/Lind-Marcus `A4 -> A5` same-size `4x4` control from
  `research/guide_artifacts/k3_shortcut_round1.json`; and
- k3 Baker/non-Baker replay-overlap step `2` from
  `research/guide_artifacts/k3_shortcut_round1.json` and
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`.

## Comparison table

`coarse` is equality of `mass_support_signature`. `trimmed` is equality of
`trimmed_active_window_signature`. `bip rN` is the weighted active bipartite WL
match result after `N` rounds. `dir rN` is the directed weighted matrix WL match
result after `N` rounds.

| Pair | coarse | trimmed | bip r1 | bip r2 | bip r3 | dir r1 | dir r2 | dir r3 | Orbit-profile baseline | Reading |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Brix rank-4 frontier/counterpart | yes | no | no | no | no | no | no | no | support transporters `6`; weighted transporters `0`; weighted stabilizers singleton | WL separates the false coarse-bucket match immediately, but this agrees with `trimmed_active_window` rather than adding a new keep/reject decision. |
| Brix rank-6 frontier/counterpart | yes | no | no | no | no | no | no | no | support transporters `6`; weighted transporters `0`; weighted stabilizers singleton | Same as rank-4: WL sees the active layout/weight mismatch that coarse support hides. |
| Baker `A4 -> A5` | no | no | no | no | no | no | no | no | no support or weighted transporter | WL also separates this known local-transfer control, so WL equality is not the missing Baker transfer signal. |
| k3 replay-overlap step 2 | yes | yes | yes | yes | yes | yes | yes | yes | weighted transporter `1` | WL preserves literal replay reuse through all tested rounds. |

Raw artifact:

- `tmp/sse-rust-nw7-10-weighted-wl-descriptors.json`

## Reading

The active bipartite WL descriptor passes the basic sanity checks:

- it separates the retained Brix rank-4 and rank-6 false coarse-bucket matches
  from the first refinement round;
- it preserves the true k3 replay-overlap calibration through rounds `1-3`; and
- it is cheap and more interpretable than the rejected weighted orbit profile,
  because its row/column color histograms still expose local weighted incidence
  structure instead of collapsing to identity stabilizers and singleton orbits.

However, it does not produce a new action signal beyond the existing retained
descriptor surface. On the selected controls, its pair decisions match the
existing `trimmed_active_window` decision exactly:

- Brix retained coarse-only pairs: split;
- Baker `A4 -> A5`: split; and
- k3 literal replay reuse: match.

The Baker control is the limiting negative calibration. The repo already knows
that `A4 -> A5` is a real same-size local step, but both WL descriptors separate
the pair in every tested round. That makes WL useful as a difference descriptor
inside coarse buckets, not as a detector for the missing local transfer.

The directed weighted matrix WL adds no positive signal on this slice. It agrees
with the active bipartite descriptor on every selected pair, but it is less
aligned with the row/column incidence framing that produced the retained
sparse-`4x4` hotspot. There is no reason to prefer it over the active bipartite
variant for future diagnostics.

## Decision

Reject promotion. Do not open a follow-up bead from this slice.

Weighted active bipartite WL is a reasonable diagnostic table column for future
coarse-bucket audits, but this bounded probe does not justify ranking,
proposal generation, pruning, canonicalization, or a new structured-family
experiment:

- it separates false coarse-bucket Brix matches, but `trimmed_active_window`
  already does that;
- it preserves literal k3 replay reuse, but so do the existing local parity
  descriptors;
- it misses the Baker `A4 -> A5` transfer signal in the same way the
  orbit-profile baseline does; and
- rounds `2` and `3` do not change the selected-pair decisions from round `1`.

If this area is revisited, WL should be used only as a cheap descriptive field
inside a larger existing-word or bridge-replay hypothesis, not as the main
proposal source.

## Validation

Focused validation for this slice:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools \
  --bin diagnose_weighted_wl_descriptors
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_weighted_wl_descriptors -- \
  --json-out tmp/sse-rust-nw7-10-weighted-wl-descriptors.json
```

Observed result:

- formatting passed;
- helper tests passed (`7` tests);
- the bounded diagnostic emitted
  `tmp/sse-rust-nw7-10-weighted-wl-descriptors.json`; and
- the comparison table above is reproduced from that JSON.
