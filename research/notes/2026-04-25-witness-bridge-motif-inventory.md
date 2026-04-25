# Witness bridge motif inventory for retained `3x3` and `4x4` surfaces (2026-04-25)

## Question

Mine the already successful or near-successful witness surfaces for reusable
bridge motifs that could inform proposal generators, ranking features, or
narrow structured-family hypotheses, without reopening broad path enumeration.

The goal is not to restate the Baker-only step coverage result. The question is
which motifs repeat across witness classes, where the repetition stops, and
whether any retained `k = 4` stuck evidence suggests a concrete next family.

## Artifact List

Successful `k = 3` exact-endpoint surfaces:

- [`research/guide_artifacts/k3_shortcut_round1.json`](../guide_artifacts/k3_shortcut_round1.json):
  Baker/Lind-Marcus lag-7 witness on
  `[[1,3],[2,1]] -> [[1,6],[1,1]]`.
- [`research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`](../guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json):
  non-Baker exact-endpoint lag-7 replay.
- [`research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json`](../guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json):
  four retained exact multi-meet lag-8 paths, each locally reducible to lag 7.

Successful or near-successful `k = 4` surfaces:

- [`research/riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json`](../riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json):
  retained Riedel/Baker full graph-only guide, lag 15 under the wider
  `dim <= 5`, `entry <= 12` envelope.
- [`research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`](2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md)
  and regenerated local extractor output
  `tmp/sse-rust-0ot-brix-ruiz-k4-stuck-states.json`: Brix-Ruiz `k = 4`
  retained graph-plus-structured approximate-hit/stuck-state surface.

Comparison controls:

- [`src/bin/classify_witness_steps.rs`](../../src/bin/classify_witness_steps.rs)
- [`src/bin/compare_brix_ruiz_graph_paths.rs`](../../src/bin/compare_brix_ruiz_graph_paths.rs)
- [`src/bin/verify_lind_marcus_reconstruction.rs`](../../src/bin/verify_lind_marcus_reconstruction.rs)

## Motif Definitions

Support profile notation is:

```text
support_count; sorted row supports; sorted column supports
```

The inventory below deliberately keeps only four high-signal motifs.

| Motif | Dimension pattern | Support-profile transition | Examples | Move-family coverage | Frequency / repetition | Keep / reject |
| --- | --- | --- | --- | --- | --- | --- |
| M1. Sparse `k = 3` entry corridor | `2x2 -> 3x3 -> 4x4` | `4; 2/2; 2/2 -> 7; 1/3/3; 2/2/3 -> 11; 2/3/3/3; 1/3/3/4`, plus the transposed retained variant `7; 2/2/3; 1/3/3 -> 11; 1/3/3/4; 2/3/3/3` | Baker `search-shortcut_search-lag-7:0..1`; non-Baker replay `search-shortcut_search-lag-7:0..1`; retained exact meets 1 and 2 steps `0..1`; retained exact meets 3 and 4 steps `0..1` use the transposed profile | Covered by `rectangular_factorisation_2x3` then `binary_sparse_rectangular_factorisation_3x3_to_4`; Baker first lift also has graph-probe lag 2 | Present in every inspected `k = 3` exact path class: Baker, non-Baker replay, and all four retained exact meets | Keep as a ranking/proposal prefix feature for hard `k = 3` endpoints. Reject as a new move family because current families already cover it, and the motif does not transfer to the Riedel `k = 4` profile without changing support scale. |
| M2. Support-11 `4x4` plateau shuffle | mostly `4x4 -> 4x4`, sometimes followed by `4x4 -> 3x3` | Common same-profile core: `11; 2/2/3/4; 2/2/3/4 -> 11; 2/2/3/4; 2/2/3/4`; entry into the core from `11; 2/3/3/3; 1/3/3/4` repeats, and exit to `8; 2/3/3; 2/3/3` repeats | Baker steps `2..4`; non-Baker steps `2..4`; retained exact meet 2 steps `3..5`; retained exact meet 4 steps `3..5`; retained exact meets 1 and 3 carry asymmetric support-11 variants | The first plateau moves are usually `elementary_conjugation`; the Baker central refactorization remains not represented by current structured families at `dim <= 4`, and its useful explanation is a heterogeneous bridge. Retained same-size middle moves can be graph-probe lag 1 while still not forming a named structured family. | The balanced same-profile transition appears 5 times in the inspected exact `k = 3` paths; entry into the balanced profile appears 3 times; balanced exit to support-8 `3x3` appears 3 times | Keep as the main `k = 3` plateau motif for guide ranking and segment hypotheses. Reject adding a default same-size `4x4` family from it: the uncovered cases are heterogeneous layout changes, not one clean reusable factorisation class. |
| M3. `3x3`/`4x4` boundary bounce | `4x4 -> 3x3`, `3x3 -> 4x4`, and Riedel `4x4 -> 5x5 -> 4x4` graph detours | In `k = 3`, repeated exit is `11; 2/2/3/4; 2/2/3/4 -> 8; 2/3/3; 2/3/3`; in Riedel `k = 4`, repeated bounce is `8; 2/3/3; 2/3/3 <-> 12/14; 4x4 support profiles`, with early detours through support-18/20 `5x5` states | Baker step `5`; non-Baker step `4`; retained exact meet 2 step `5`; retained exact meet 4 step `5`; Riedel full graph guide steps `1,6,8,9,10,11,12,13` | `k = 3` exits are often covered by `binary_sparse_rectangular_factorisation_4x3_to_3`; Riedel is graph-only split/amalgamation/permutation evidence under `dim <= 5`, not a native `dim <= 4` structured family | Repeats across successful `k = 3` exact witnesses and the solved Riedel/Baker `k = 4` graph-only control, but not as the same support scale or same family | Keep only as a ranking feature: boundary-bounce guides are credible in solved controls. Reject it as the next Brix-Ruiz `k = 4` family because the retained Brix-Ruiz stuck extraction gives `binary_sparse_rectangular_factorisation_4x3_to_3` only 7 discoveries and 0 approximate hits, while `3x3_to_4` has only 2 shallow approximate hits. |
| M4. Sparse active-block `4x4` layout miss | `4x4 -> 4x4` inside two zero rows or zero columns | Brix-Ruiz rank-4 example keeps support profile `0/0/3/4; 1/2/2/2` and aggregate signature rows `0/0/10/14`, cols `2/4/5/13`, but misses the active `2 x 4` layout | Rank 4: from `[[1,4,1,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]`, diagonal step to `[[1,4,2,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]`, closest opposite-side state `[[1,12,0,1],[1,1,4,4],[0,0,0,0],[0,0,0,0]]`. Rank 6 is the same phenomenon with rows `3/4/5/11`, cols `0/0/6/17` | Parent move is already `diagonal_refactorization_4x4`; the missing piece is not diagonal scaling itself but a local active-block transfer/layout correction. Existing graph moves also produce approximate hits, but the best structured evidence is diagonal | Near-success only, not an exact witness: `diagonal_refactorization_4x4` has 37 approximate hits, including rank 4, 6, and 9 near-cap pairs; `elementary_conjugation` has 90 mostly graph-family hits | Keep as the one promising `k = 4` proposal/ranking slice. It does not generalize back to the exact `k = 3` witnesses, but it is the strongest retained Brix-Ruiz `k = 4` structured signal. Do not open a new bead here because `sse-rust-nw7.7` already covers the concrete active-block proposal experiment. |

## Coverage And Repetition Analysis

The main generalization is across `k = 3` witness classes, not across all
surfaces. Baker, non-Baker, and the retained exact multi-meet paths use the same
low-dimensional grammar:

```text
2x2 source
-> sparse 3x3
-> sparse support-11 4x4
-> support-11 4x4 plateau
-> 3x3 or 2x2 contraction
-> 2x2 target
```

This grammar is stronger than a Baker-only conclusion because it survives the
non-Baker replay and four retained exact-meet variants. It is weaker than a new
family proposal because the repeated pieces are mostly already covered:
rectangular endpoint moves, binary-sparse `3x3 -> 4x4` / `4x4 -> 3x3`, and
elementary conjugation.

The Baker and blind graph-only `k = 3` graph paths do not share hidden canonical
intermediates beyond endpoints:

```text
Total shared (canonical) matrices: 2
Baker-only intermediates: 21
Blind-only intermediates: 15
```

So the useful motif is a support/dimension envelope, not a shared waypoint
bottleneck.

The Riedel/Baker `k = 4` graph-only guide repeats the idea of boundary bouncing,
but at a larger support scale and with `5x5` detours:

```text
3x3 support 8 -> 4x4 support 14 -> 5x5 support 20 -> 4x4 support 14
...
3x3 support 8 -> 4x4 support 12/14 -> 3x3 support 8
```

This is a solved-control motif, not direct evidence for Brix-Ruiz Goal 3. The
Brix-Ruiz retained stuck-state extractor is negative on the native boundary
family (`4x3_to_3` has 0 approximate hits), while the active sparse `4x4`
diagonal/layout misses are positive. That rejects the broad generalization
"successful `k = 3`/Riedel boundary bounces should become the next Brix-Ruiz
`k = 4` move family".

## Recommendations

Keep:

- M1 as a guide ranking prefix for hard `k = 3` exact-endpoint proposals.
- M2 as the main `k = 3` plateau segment feature, especially when ranking
  candidate guide fragments.
- M3 as a replay/ranking signal for solved controls, but not as a new Brix-Ruiz
  `k = 4` proposal family.
- M4 as the only promising proposal-facing motif from this slice: sparse
  `4x4` active-block transfer around diagonal approximate hits.

Reject:

- adding a Baker-specific default family from M2;
- reopening weighted or generic `4x4 -> 3x3` work from M3;
- claiming a single hidden canonical bottleneck across the exact `k = 3`
  witnesses; and
- claiming that any motif here generalizes cleanly across Baker, non-Baker,
  Riedel/Baker `k = 4`, and Brix-Ruiz `k = 4`.

No new follow-up bead was opened. The only clearly promising follow-up is M4,
and the existing `sse-rust-nw7.7` bead already covers that bounded active-block
proposal/ranking slice.

## Commands Run

Focused binary build:

```bash
timeout -k 20s 180s cargo build --features research-tools \
  --bin classify_witness_steps \
  --bin compare_brix_ruiz_graph_paths \
  --bin verify_lind_marcus_reconstruction
```

`k = 3` step classifiers:

```bash
timeout -k 20s 120s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/sse-rust-0ot-classify-k3-baker-dim4.json

timeout -k 20s 120s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/sse-rust-0ot-classify-k3-non-baker-dim4.json

timeout -k 20s 120s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/sse-rust-0ot-classify-k3-retained-dim4.json
```

Support-profile and frequency extraction:

```bash
jq -r 'def mats: if has("artifacts") then .artifacts[] | .artifact_id as $id | .path.matrices as $m | range(0; $m|length) as $i | [$id,$i,$m[$i]] else .artifact_id as $id | .path.matrices as $m | range(0; $m|length) as $i | [$id,$i,$m[$i]] end; def support_count($m): [$m.data[] | select(. != 0)] | length; def row_supports($m): [range(0; $m.rows) as $r | [$m.data[($r*$m.cols):(($r+1)*$m.cols)][] | select(. != 0)] | length] | sort | join("/"); def col_supports($m): [range(0; $m.cols) as $c | [range(0; $m.rows) as $r | $m.data[$r*$m.cols+$c] | select(. != 0)] | length] | sort | join("/"); (["artifact","pos","dim","support","row_supports","col_supports","data"] | @tsv), (mats | .[0] as $id | .[1] as $i | .[2] as $m | [$id,$i,(($m.rows|tostring)+"x"+($m.cols|tostring)),support_count($m),row_supports($m),col_supports($m),($m.data|join(","))] | @tsv)' \
  research/guide_artifacts/k3_shortcut_round1.json \
  research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json \
  research/riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json \
  > tmp/sse-rust-0ot-support-profiles.tsv
```

Riedel `k = 4` transition extraction:

```bash
jq -r 'def sc($m): [$m.data[] | select(. != 0)] | length; def rs($m): [range(0; $m.rows) as $r | [$m.data[($r*$m.cols):(($r+1)*$m.cols)][] | select(. != 0)] | length] | sort | join("/"); def cs($m): [range(0; $m.cols) as $c | [range(0; $m.rows) as $r | $m.data[$r*$m.cols+$c] | select(. != 0)] | length] | sort | join("/"); def prof($m): ((sc($m)|tostring)+"; "+rs($m)+"; "+cs($m)); (["step","from_dim","to_dim","profile_transition","from","to"] | @tsv), (.path.matrices as $m | range(0; ($m|length-1)) as $i | [$i,(($m[$i].rows|tostring)+"x"+($m[$i].cols|tostring)),(($m[$i+1].rows|tostring)+"x"+($m[$i+1].cols|tostring)),(prof($m[$i])+" -> "+prof($m[$i+1])),($m[$i].data|join(",")),($m[$i+1].data|join(","))] | @tsv)' \
  research/riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json \
  > tmp/sse-rust-0ot-riedel-k4-transitions.tsv
```

Brix-Ruiz `k = 4` stuck-state extraction:

```bash
timeout -k 20s 180s cargo build --features research-tools \
  --bin extract_brix_ruiz_k4_stuck_states

timeout -k 20s 120s target/debug/extract_brix_ruiz_k4_stuck_states \
  --json-out tmp/sse-rust-0ot-brix-ruiz-k4-stuck-states.json \
  --top 40

jq -r '(["rank","layer","direction","family","from_depth","to_depth","counterpart_depth","slack","l1","row_sums","col_sums","support","from_matrix","to_matrix","counterpart_matrix"] | @tsv), (.ranked_approximate_hits[:12][] | [.rank,.layer_index,.direction,.move_family,.from_depth,.to_depth,.counterpart_depth,.bridge_slack_at_lag40,.counterpart_l1,(.signature.row_sums|join("/")),(.signature.col_sums|join("/")),((.signature.row_supports|join("/"))+";"+(.signature.col_supports|join("/"))),(.from_matrix.data|join(",")),(.to_matrix.data|join(",")),((.counterpart_matrix.data // [])|join(","))] | @tsv)' \
  tmp/sse-rust-0ot-brix-ruiz-k4-stuck-states.json \
  > tmp/sse-rust-0ot-brix-ruiz-k4-top12.tsv
```

Graph-path comparison and Baker graph-only reconstruction checks:

```bash
timeout -k 20s 120s target/debug/verify_lind_marcus_reconstruction \
  > tmp/sse-rust-0ot-verify-lind-marcus-reconstruction.txt

timeout -k 20s 120s target/debug/compare_brix_ruiz_graph_paths \
  > tmp/sse-rust-0ot-compare-brix-ruiz-graph-paths.txt
```

Final formatting validation:

```bash
timeout -k 20s 120s cargo fmt --all
```
