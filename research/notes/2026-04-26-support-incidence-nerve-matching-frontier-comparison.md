# Support-incidence nerve and matching descriptors for frontier comparison (2026-04-26)

## Question

For bead `sse-rust-nw7.12`, test whether support-incidence nerve or
matching-style descriptors add useful matrix near-miss signal beyond existing
mass/support signatures, `trimmed_active_window`, endpoint-local parity, the
rejected active-block orbit/stabilizer profile, and the rejected weighted-WL
descriptor.

This is diagnostics-only:

- no solver scoring, pruning, canonicalization, or move generation changes;
- no generic `4x4` factorisation enumeration;
- no spectral, WL, or orbit/stabilizer follow-through; and
- no claim that support-incidence descriptors are SSE invariants.

## Descriptor definitions

Research-only helper:

- `src/bin/diagnose_support_incidence_descriptors.rs`

Reproducible command:

```bash
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_support_incidence_descriptors -- \
  --json-out tmp/sse-rust-nw7-12-support-incidence-descriptors.json
```

Model: `active_block_support_incidence`.

For each selected nonnegative matrix:

1. delete all-zero rows and all-zero columns;
2. forget entry weights;
3. treat every positive entry as an edge in a bipartite row/column incidence
   graph; and
4. summarize only the resulting support set system.

The tested descriptor is the JSON signature of these fields:

- active shape, support edge count, sorted row-support sizes, and sorted
  column-support sizes;
- row-support nerve: for row supports `R_i` as subsets of active columns, a
  row subset is a simplex when the supports have nonempty common intersection;
  the summary records simplex counts by size, maximal face sizes, and
  one-skeleton component sizes;
- column-support nerve, defined dually over column supports as subsets of
  active rows;
- Hall-deficit profiles on both sides: for every nonempty subset `S`, record
  `|S| - |N(S)|`, grouped by subset size, plus positive-deficit counts;
- maximum bipartite matching size, row deficiency, and column deficiency;
- connected components of the active incidence graph, summarized by row count,
  column count, and edge count;
- pairwise row-support and column-support overlap-size multisets; and
- a tiny biclique-cover hint: enumerate all maximal complete bipartite support
  subgraphs and branch-and-bound the exact minimum edge biclique cover. The
  selected controls are at most `4x4`, so this covers at most `16` support
  edge positions.

These summaries are invariant under independent active-row and active-column
renaming, but they deliberately discard all positive entry weights.

## Controls

The controls intentionally match the previous orbit-profile and weighted-WL
slices:

- retained Brix-Ruiz `k = 4` rank-4 sparse `4x4` near-hit pair from
  `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`;
- retained Brix-Ruiz `k = 4` rank-6 sparse `4x4` near-hit pair carried from the
  active-block switch rank-6 fixture;
- Baker/Lind-Marcus `A4 -> A5` same-size `4x4` control from
  `research/guide_artifacts/k3_shortcut_round1.json`; and
- k3 Baker/non-Baker replay-overlap step `2` from
  `research/guide_artifacts/k3_shortcut_round1.json` and
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`.

Raw artifact:

- `tmp/sse-rust-nw7-12-support-incidence-descriptors.json`

## Sample descriptor table

`match` is the maximum bipartite matching size. `def` is
`row_deficiency/column_deficiency`. `biclique cover` is the exact minimum
support-edge biclique cover size.

| Sample | Active shape | Support edges | Row supports | Column supports | match | def | biclique cover | Components |
| --- | ---: | ---: | --- | --- | ---: | --- | ---: | --- |
| Brix rank-4 frontier | `2x4` | `7` | `3/4` | `1/2/2/2` | `2` | `0/2` | `2` | `2r/4c/7e` |
| Brix rank-4 counterpart | `2x4` | `7` | `3/4` | `1/2/2/2` | `2` | `0/2` | `2` | `2r/4c/7e` |
| Brix rank-6 frontier | `4x2` | `7` | `1/2/2/2` | `3/4` | `2` | `2/0` | `2` | `4r/2c/7e` |
| Brix rank-6 counterpart | `4x2` | `7` | `1/2/2/2` | `3/4` | `2` | `2/0` | `2` | `4r/2c/7e` |
| Baker `A4` | `4x4` | `11` | `2/2/3/4` | `2/2/3/4` | `4` | `0/0` | `3` | `4r/4c/11e` |
| Baker `A5` | `4x4` | `11` | `1/3/3/4` | `2/3/3/3` | `4` | `0/0` | `3` | `4r/4c/11e` |
| k3 Baker step-2 replay | `4x4` | `11` | `2/3/3/3` | `1/3/3/4` | `4` | `0/0` | `3` | `4r/4c/11e` |
| k3 non-Baker step-2 replay | `4x4` | `11` | `2/3/3/3` | `1/3/3/4` | `4` | `0/0` | `3` | `4r/4c/11e` |

## Comparison table

`incidence` is equality of the full support-incidence descriptor. For the
Brix rows, every subdescriptor also matched: nerves, Hall profiles, matching,
components, overlap, and biclique-cover hints.

| Pair | coarse | trimmed/action | orbit-profile baseline | weighted-WL baseline | incidence | Reading |
| --- | --- | --- | --- | --- | --- | --- |
| Brix rank-4 frontier/counterpart | yes | split, `rank_or_propose_inside_coarse_bucket` | support transporters match, weighted transporters split | split from round 1 | match | Pure support incidence collapses the false coarse-bucket match exactly where the support-shadow orbit profile collapsed it. |
| Brix rank-6 frontier/counterpart | yes | split, `rank_or_propose_inside_coarse_bucket` | support transporters match, weighted transporters split | split from round 1 | match | Same failure mode as rank 4: the active support set system is identical up to row/column renaming. |
| Baker `A4 -> A5` | no | split, `ignore` | no support or weighted transporter | split from round 1 | split | Incidence sees a support difference, but this is not a useful transfer-reuse signal because the known same-size step is split. |
| k3 replay-overlap step 2 | yes | match, `reuse_endpoint_local_parity` | weighted transporter match | match through round 3 | match | Incidence preserves literal replay reuse, matching the existing retained local descriptor. |

## Reading

The support-incidence descriptors pass only the easiest sanity check: they
preserve the known k3 Baker/non-Baker replay overlap at step `2`.

They fail the motivating Brix controls. The retained rank-4 and rank-6
near-hit/counterpart pairs have identical active support incidence up to
independent row/column renaming. Because the new descriptors intentionally
forget weights, the row/column nerves, Hall deficits, matching deficiencies,
connected components, overlap profiles, and biclique-cover hints all match
those false coarse-bucket pairs. That is weaker than `trimmed_active_window`
and weighted-WL, both of which split those pairs.

The Baker `A4 -> A5` control is also negative calibration. Support incidence
splits the pair, but so do `trimmed_active_window`, orbit profiles, and
weighted-WL. Since `A4 -> A5` is a known local transfer, incidence equality is
not the missing transfer-reuse signal.

The matching and component subfields are especially coarse on the selected
`4x4` controls: Baker `A4`, Baker `A5`, and the k3 replay-overlap states all
have perfect `4`-matchings and one connected `4r/4c/11e` component. Those
fields are useful context for describing the support graph but not enough for
ranking or proposal generation.

## Decision

Reject promotion. Do not open a follow-up bead from this slice.

Support-incidence nerve and matching descriptors do not add useful near-miss
signal beyond the existing descriptor surface on these controls:

- they preserve literal k3 replay reuse;
- they do not separate the retained Brix rank-4 or rank-6 coarse-only layout
  mismatches;
- their Baker split is already visible to `trimmed_active_window`, orbit
  profiles, and weighted-WL; and
- no subdescriptor sees a useful distinction missed by the retained
  `trimmed_active_window` parity surface.

If this area is revisited, it should be as a small descriptive table column
inside an existing-word or bridge-replay hypothesis, not as a standalone
ranking, pruning, canonicalization, or move-generation proposal.

## Validation

Focused validation for this slice:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools \
  --bin diagnose_support_incidence_descriptors
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_support_incidence_descriptors -- \
  --json-out tmp/sse-rust-nw7-12-support-incidence-descriptors.json
```

Observed result:

- formatting passed;
- helper tests passed (`6` tests);
- the bounded diagnostic emitted
  `tmp/sse-rust-nw7-12-support-incidence-descriptors.json`; and
- the comparison table above is reproduced from that JSON.
