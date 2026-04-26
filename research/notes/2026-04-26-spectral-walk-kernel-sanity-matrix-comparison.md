# Spectral and walk-kernel sanity check for matrix comparison (2026-04-26)

## Question

For bead `sse-rust-nw7.13`, test whether cheap spectral or walk-count
descriptors add useful matrix-comparison signal beyond existing exact power
traces, Bowen-Franks checks, `same_future_past`/coarse buckets,
`trimmed_active_window`, endpoint-local parity, and the recently rejected
active-block orbit/WL/support-incidence descriptors.

This is diagnostics-only:

- no solver scoring, pruning, canonicalization, or move generation changes;
- no generic `4x4` factorisation enumeration;
- no floating spectral equality claim; and
- no numeric eigensolver dependency.

## Descriptor Definitions

Research-only helper:

- `src/bin/diagnose_spectral_walk_descriptors.rs`

Reproducible command:

```bash
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_spectral_walk_descriptors -- \
  --json-out tmp/sse-rust-nw7-13-spectral-walk-descriptors.json
```

The helper uses exact integer arithmetic for small selected matrices:

- `full_directed_closed_walk_traces_1_to_6`: exact `trace(M^p)` for
  `p = 1..6` on the full square weighted adjacency matrix. The prefix
  `p = 1..4` is the existing square power-trace invariant surface already used
  by the solver for dimensions up to `4`.
- `full_directed_adjacency_charpoly`: exact characteristic polynomial of the
  full square weighted adjacency matrix. For these `4x4` controls, this is
  equivalent to the existing trace prefix by Newton identities, not new
  spectral signal.
- `bowen_franks_i_minus_m`: Smith-normal-form invariants of `I - M`, matching
  the same-dimension Bowen-Franks check already present for square endpoints.
- `full_directed_total_walks_1_to_4`: exact total weighted directed walk counts
  `1^T M^p 1` for `p = 1..4`. This is a walk-count descriptor but not an SSE
  invariant.
- `active_weighted_gram_charpoly`: delete all-zero rows/columns to get active
  block `B`; report the exact characteristic polynomial of `B B^T`. This is a
  singular-value-adjacent descriptor using squared singular values.
- `active_support_laplacian_charpoly`: exact characteristic polynomial of the
  undirected bipartite row/column support Laplacian for the active block.
- `active_weighted_laplacian_charpoly`: same bipartite Laplacian, using entry
  values as edge weights.

## Controls

The controls intentionally match the previous descriptor slices:

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

- `tmp/sse-rust-nw7-13-spectral-walk-descriptors.json`

## Comparison Table

`coarse` is `mass_support_signature` equality. `trimmed` is
`trimmed_active_window_signature` equality. `trace1-4` is the existing exact
power-trace prefix. `trace1-6`, `charpoly`, and `BF` are full directed spectral
or exact invariant-adjacent fields. `total walks` is `1^T M^p 1` for `p=1..4`.
`gram`, `support Lap`, and `weighted Lap` are active-block spectral-adjacent
fields.

| Pair | coarse | trimmed/action | trace1-4 | trace1-6 | charpoly | BF | total walks | gram | support Lap | weighted Lap | Reading |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Brix rank-4 frontier/counterpart | yes | split, `rank_or_propose_inside_coarse_bucket` | match | match | match | match | split | split | match | split | Full directed spectral data is only existing trace/BF overlap. Active weighted descriptors split, but only where `trimmed_active_window` and WL already split. |
| Brix rank-6 frontier/counterpart | yes | split, `rank_or_propose_inside_coarse_bucket` | match | match | match | match | split | split | match | split | Same as rank 4. Support Laplacian collapses exactly like support-incidence descriptors. |
| Baker `A4 -> A5` | no | split, `ignore` | match | match | match | match | split | split | split | split | Full directed spectral data preserves this known same-size transfer because it is the existing invariant surface. Active/walk-total descriptors split it, so they are not reuse signals. |
| k3 replay-overlap step 2 | yes | match, `reuse_endpoint_local_parity` | match | match | match | match | match | match | match | match | All tested descriptors preserve literal replay reuse. |

## Exact Sample Values

Full directed traces and characteristic polynomials show the overlap directly:

| Sample group | `trace(M^1..M^6)` | `charpoly(M)` | `BF(I-M)` |
| --- | --- | --- | --- |
| Brix rank-4/rank-6 samples | `2/26/74/434/1682/8138` | `1/-2/-11/0/0` | `1/1/1/12` |
| Baker `A4`, Baker `A5`, k3 replay step 2 samples | `2/14/38/146/482/1694` | `1/-2/-5/0/0` | `1/1/1/6` |

Active-block spectral fields expose only already-known active-layout facts:

| Sample | active shape | gram charpoly | support Lap charpoly | weighted Lap charpoly |
| --- | ---: | --- | --- | --- |
| Brix rank-4 frontier | `2x4` | `1/-116/819` | `1/-14/72/-170/184/-72/0` | `1/-48/781/-5348/15405/-14916/0` |
| Brix rank-4 counterpart | `2x4` | `1/-180/4675` | `1/-14/72/-170/184/-72/0` | `1/-48/717/-4020/9041/-6936/0` |
| Brix rank-6 frontier | `4x2` | `1/-147/1718/0/0` | `1/-14/72/-170/184/-72/0` | `1/-46/663/-4066/11178/-11352/0` |
| Brix rank-6 counterpart | `4x2` | `1/-153/2349/0/0` | `1/-14/72/-170/184/-72/0` | `1/-46/657/-3876/9828/-8778/0` |
| Baker `A4` | `4x4` | `1/-20/55/-40/0` | `1/-22/198/-944/2567/-3962/3194/-1032/0` | `1/-28/315/-1848/6095/-11256/10716/-4032/0` |
| Baker `A5` | `4x4` | `1/-25/71/-24/0` | `1/-22/198/-942/2541/-3838/2938/-840/0` | `1/-30/357/-2194/7496/-14088/13216/-4608/0` |
| k3 Baker step-2 replay | `4x4` | `1/-23/61/-36/0` | `1/-22/198/-942/2541/-3838/2938/-840/0` | `1/-30/360/-2244/7813/-14998/14266/-4808/0` |
| k3 non-Baker step-2 replay | `4x4` | `1/-23/61/-36/0` | `1/-22/198/-942/2541/-3838/2938/-840/0` | `1/-30/360/-2244/7813/-14998/14266/-4808/0` |

## Reading

The full directed spectral lane is redundant. On every selected pair,
`trace(M^1..M^4)`, `trace(M^1..M^6)`, the exact characteristic polynomial, and
Bowen-Franks all match. The extra traces `M^5` and `M^6` do not add information
for these `4x4` samples because the characteristic polynomial already
determines the recurrence. This is useful negative confirmation: the cheap
spectral equality one would expect from SSE is already covered by exact
power-trace/Bowen-Franks screens.

The total-walk vector and active weighted spectral fields can separate the two
Brix false coarse-bucket pairs. That is not new value:

- `trimmed_active_window` and weighted WL already split both pairs;
- the descriptors also split Baker `A4 -> A5`, a known same-size local-transfer
  control; and
- the support Laplacian collapses the Brix pairs exactly like the previously
  rejected support-incidence descriptors.

The singular-value-adjacent Gram descriptor is therefore only a compact way to
say that the active weighted rows/columns have different second-order mass
geometry. It does not see a reuse or transfer signal missed by
`trimmed_active_window`, orbit profiles, WL, or support incidence.

## Decision

Reject promotion. Do not open a follow-up bead from this slice.

No tested spectral or walk-kernel descriptor justifies ranking, proposal
generation, pruning, canonicalization, or move-generation changes:

- full directed spectral descriptors overlap existing exact trace/BF data;
- active weighted Gram and weighted Laplacian descriptors split false Brix
  coarse-bucket matches but also split the Baker transfer control;
- support Laplacian repeats the already-rejected support-only collapse; and
- the only all-descriptor positive match is literal k3 replay reuse, already
  handled by endpoint-local parity.

If spectral data is revisited, it should remain an explanatory table column for
bounded audits, not a standalone proposal source.

## Validation

Focused validation for this slice:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools \
  --bin diagnose_spectral_walk_descriptors
timeout -k 20s 60s cargo run -q --features research-tools \
  --bin diagnose_spectral_walk_descriptors -- \
  --json-out tmp/sse-rust-nw7-13-spectral-walk-descriptors.json
```

Observed result:

- formatting passed;
- helper tests passed (`7` tests);
- the bounded diagnostic emitted
  `tmp/sse-rust-nw7-13-spectral-walk-descriptors.json`; and
- the comparison table above is reproduced from that JSON.
