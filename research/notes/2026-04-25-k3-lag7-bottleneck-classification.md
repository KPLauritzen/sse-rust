# Hard Brix-Ruiz `k = 3` lag-7 bottleneck classification (2026-04-25)

## Question

Classify why the hard Brix-Ruiz `k = 3` exact endpoints

```text
[[1,3],[2,1]] -> [[1,6],[1,1]]
```

keep producing lag-7 witnesses, and decide whether the current evidence exposes
one credible bounded next experiment for lag `< 7`.

This is a classification slice only. It does not reopen endpoint multi-meet
search, guide-pool assembly, broad shortcut search, or new move-family work.

## Sources

- [`research/notes/2026-04-25-baker-lag7-structured-classification.md`](2026-04-25-baker-lag7-structured-classification.md)
- [`research/notes/2026-04-19-exact-endpoint-multi-meet-lag7-diversity.md`](2026-04-19-exact-endpoint-multi-meet-lag7-diversity.md)
- [`research/notes/2026-04-20-k3-non-baker-exact-endpoint-lag7-guided-replay-control.md`](2026-04-20-k3-non-baker-exact-endpoint-lag7-guided-replay-control.md)
- [`research/notes/2026-04-19-k3-lag7-retained-diversity-collapse.md`](2026-04-19-k3-lag7-retained-diversity-collapse.md)
- [`research/notes/2026-04-14-k3-normalized-guide-pool-shortcutting.md`](2026-04-14-k3-normalized-guide-pool-shortcutting.md)
- [`research/notes/2026-04-14-k3-shortcut-lagcap-timeout-boundary-rebuild.md`](2026-04-14-k3-shortcut-lagcap-timeout-boundary-rebuild.md)
- [`docs/brix-ruiz-sidecar-log.md`](../../docs/brix-ruiz-sidecar-log.md)
- [`src/bin/compare_brix_ruiz_graph_paths.rs`](../../src/bin/compare_brix_ruiz_graph_paths.rs)
- [`src/bin/assemble_k3_guide_pool.rs`](../../src/bin/assemble_k3_guide_pool.rs)
- [`src/bin/search.rs`](../../src/bin/search.rs)

## Comparison table

The two main exact-endpoint lag-7 classes are:

- Baker/Lind-Marcus:
  [`research/guide_artifacts/k3_shortcut_round1.json`](../guide_artifacts/k3_shortcut_round1.json)
- non-Baker exact replay:
  [`research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`](../guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json)

Support profile notation below is `support_count; sorted row supports; sorted
column supports`.

| Position | Baker dim/profile | Non-Baker dim/profile | Reading |
| --- | --- | --- | --- |
| 0 | `2x2`, `4; 2/2; 2/2` | `2x2`, `4; 2/2; 2/2` | same exact source |
| 1 | `3x3`, `7; 1/3/3; 2/2/3` | `3x3`, `7; 1/3/3; 2/2/3` | same sparse first-lift profile, different matrix |
| 2 | `4x4`, `11; 2/3/3/3; 1/3/3/4` | `4x4`, `11; 2/3/3/3; 1/3/3/4` | same 4x4 support-11 entry profile |
| 3 | `4x4`, `11; 2/2/3/4; 2/2/3/4` | `4x4`, `11; 2/2/3/4; 2/2/3/4` | same balanced 4x4 plateau profile |
| 4 | `4x4`, `11; 2/2/3/4; 2/2/3/4` | `4x4`, `11; 2/2/3/4; 2/2/3/4` | same balanced 4x4 plateau profile |
| 5 | `4x4`, `11; 1/3/3/4; 2/3/3/3` | `3x3`, `8; 2/3/3; 2/3/3` | divergence: Baker keeps one more 4x4 step; non-Baker drops earlier |
| 6 | `3x3`, `7; 2/2/3; 1/3/3` | `2x2`, `3; 1/2; 1/2` | divergence: non-Baker uses a sparse 2x2 bridge |
| 7 | `2x2`, `4; 2/2; 2/2` | `2x2`, `4; 2/2; 2/2` | same exact target |

Move-family classification with `max_intermediate_dim = 4`, `max_entry = 5`,
and permutation-aware factorisation matching:

| Step | Baker family | Non-Baker family | Reading |
| --- | --- | --- | --- |
| 0 | `rectangular_factorisation_2x3`; graph probe lag `2` | exact `outsplit`; `rectangular_factorisation_2x3` | endpoint lift is not the bottleneck |
| 1 | `binary_sparse_rectangular_factorisation_3x3_to_4`; no dim-4 graph probe | `binary_sparse_rectangular_factorisation_3x3_to_4`; no dim-4 graph probe | both enter the hard 4x4 envelope through the same structured family |
| 2 | `elementary_conjugation`; no dim-4 graph probe | `elementary_conjugation`; no dim-4 graph probe | same same-size family |
| 3 | `elementary_conjugation`; no dim-4 graph probe | `elementary_conjugation`; no dim-4 graph probe | same same-size family |
| 4 | no current structured family; no dim-4 graph probe | `binary_sparse_rectangular_factorisation_4x3_to_3`; no dim-4 graph probe | Baker has the known hard same-size refactorization; non-Baker exits the plateau here |
| 5 | `binary_sparse_rectangular_factorisation_4x3_to_3`; graph probe lag `3` | exact `in_amalgamation`; `rectangular_factorisation_3x3_to_2` | both contract after the 4x4 plateau, but through different visible waypoints |
| 6 | `rectangular_factorisation_3x3_to_2`; graph probe lag `4` | `square_factorisation_2x2`; graph probe lag `3` | final return differs; neither exposes a one-step shortening |

With `max_intermediate_dim = 5`, the graph probes explain more individual
structured steps but do not make a shorter endpoint witness:

- Baker step `4` becomes a graph-only bridge at lag `7`, so the hard same-size
  Baker refactorization is not a dim-4 graph move.
- Non-Baker steps `1`, `2`, `3`, and `4` get graph-only explanations of lags
  `6`, `3`, `2`, and `6`, respectively.
- The graph-only sidecar has an independent blind endpoint path of depth `16`,
  while Baker's waypoint-expanded graph path has depth `22`.
- Those two graph-only paths share only the source and target canonical
  matrices. All `21` Baker graph intermediates and all `15` blind graph
  intermediates are canonically distinct.

## Bottleneck motifs

The exact lag-7 witnesses do **not** share a mandatory-looking canonical
intermediate. The best evidence against a single hidden canonical bottleneck is:

```text
Guide-pool quotient shrinkage over Baker + non-Baker:
  source=2 unique_raw=2 retained=2 removed_by_dedup=0 collision_groups=0
```

and for the four exact multi-meet retained paths:

```text
source=4 unique_raw=4 retained=4 removed_by_dedup=0 collision_groups=0
lag 8 -> 7 via local Triangle rewrite for each retained path
```

So the retained exact paths are not just relabelings or quotient-collapsed
variants of the same path.

They do share a structural envelope:

- all exact retained paths start with a `2 -> 3 -> 4` lift into sparse
  support-11 `4x4` matrices;
- the retained exact multi-meet paths keep four consecutive `4x4` matrices
  before contracting;
- the Baker and non-Baker lag-7 paths both use the same first `4x4` support
  profiles and the same two elementary-conjugation-shaped middle moves;
- graph-only explanations for the hard structured moves need dimension `5`;
  dimension `4` graph probes miss the central `3x3 -> 4x4`, same-size `4x4`,
  and `4x4 -> 3x3` bridges.

The right current classification is therefore: **shared support/dimension
envelope bottleneck, no shared canonical waypoint bottleneck**.

## What would have to change for lag `< 7`

A shorter exact-endpoint witness would need one of these to become real:

1. A missing shorter bridge through the 4x4 plateau.
   Current evidence is negative: Baker has one hard same-size `4x4 -> 4x4`
   refactorization, while non-Baker exits the plateau earlier through
   `4x4 -> 3x3` and still lands at lag `7`.
2. A lower-dimensional shortcut.
   Current evidence is negative: the hard transitions are exactly the ones
   whose graph-only explanations disappear at `max_intermediate_dim = 4`.
3. A different endpoint orientation.
   Current evidence is weak: the broader normalized guide pool has non-exact
   endpoint classes, but exact replay has repeatedly reanchored to Baker or to
   another lag-7 path, not below lag `7`.
4. A new move family.
   This slice gives no clean candidate. The prior Baker structured
   classification already rejected adding a default family for the remaining
   Baker step because it decomposes as a heterogeneous bridge, not a simple
   reusable one-step family.
5. Evidence that the current envelopes impose a real lag-7 bottleneck.
   This is the strongest reading so far. Multiple canonically distinct exact
   routes all enter the same sparse `4x4` envelope, local quotient rewrites
   reduce retained lag-8 paths only to lag `7`, and prior shortcut searches
   under `dim <= 5`, `entry <= 6`, and larger segment budgets did not improve
   past lag `7`.

## Decision

No credible shorter-witness direction emerged from this classification.

Do not open a new bead mechanically. A justified follow-up would need a
specific segment-level hypothesis, for example "replace this exact consecutive
subpath by lag `N < current` under bound `B`". The current read does not isolate
such a segment. Running another broad multi-meet, guide-pool, or shortcut pass
would only repeat the existing lag-7 plateau evidence.

## Commands run

Focused builds:

```text
timeout -k 20s 180s cargo build --features research-tools \
  --bin classify_witness_steps --bin compare_brix_ruiz_graph_paths

timeout -k 20s 180s cargo build --features research-tools \
  --bin analyze_guide_pool_quotient
```

Baker/non-Baker support extraction:

```text
jq -r 'def support_count: [.data[] | select(. != 0)] | length;
  def row_supports: [. as $m | range(0; $m.rows) as $r |
    [$m.data[($r*$m.cols):(($r+1)*$m.cols)][] | select(. != 0)] | length]
    | sort | join("/");
  def col_supports: [. as $m | range(0; $m.cols) as $c |
    [range(0; $m.rows) as $r | $m.data[$r*$m.cols+$c] | select(. != 0)]
    | length] | sort | join("/");
  .path.matrices as $m | range(0; ($m|length)) as $i |
  [$i, ($m[$i].rows|tostring)+"x"+($m[$i].cols|tostring),
   ($m[$i]|support_count), ($m[$i]|row_supports), ($m[$i]|col_supports)]
  | @tsv' \
  research/guide_artifacts/k3_shortcut_round1.json \
  > tmp/sse-rust-a75-baker-support-table.tsv

jq -r 'def support_count: [.data[] | select(. != 0)] | length;
  def row_supports: [. as $m | range(0; $m.rows) as $r |
    [$m.data[($r*$m.cols):(($r+1)*$m.cols)][] | select(. != 0)] | length]
    | sort | join("/");
  def col_supports: [. as $m | range(0; $m.cols) as $c |
    [range(0; $m.rows) as $r | $m.data[$r*$m.cols+$c] | select(. != 0)]
    | length] | sort | join("/");
  .path.matrices as $m | range(0; ($m|length)) as $i |
  [$i, ($m[$i].rows|tostring)+"x"+($m[$i].cols|tostring),
   ($m[$i]|support_count), ($m[$i]|row_supports), ($m[$i]|col_supports)]
  | @tsv' \
  research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  > tmp/sse-rust-a75-non-baker-support-table.tsv
```

Witness-step classification:

```text
timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/sse-rust-a75-classify-baker-dim4.json

timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/sse-rust-a75-classify-non-baker-dim4.json

timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 5 \
  --graph-probe-max-entry 5 \
  > tmp/sse-rust-a75-classify-baker-dim5.json

timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 5 \
  --graph-probe-max-entry 5 \
  > tmp/sse-rust-a75-classify-non-baker-dim5.json
```

Quotient and graph-path comparison:

```text
timeout -k 20s 120s target/debug/analyze_guide_pool_quotient \
  --guide-artifacts research/guide_artifacts/k3_shortcut_round1.json \
  --guide-artifacts research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --max-suffix-lag 4 \
  --max-rewrite-states 1024 \
  --max-samples 12 \
  --json-out tmp/sse-rust-a75-baker-non-baker-quotient.json \
  > tmp/sse-rust-a75-baker-non-baker-quotient.txt

timeout -k 20s 120s target/debug/analyze_guide_pool_quotient \
  --guide-artifacts research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json \
  --max-suffix-lag 4 \
  --max-rewrite-states 1024 \
  --max-samples 12 \
  --json-out tmp/sse-rust-a75-retained-exact-quotient.json \
  > tmp/sse-rust-a75-retained-exact-quotient.txt

timeout -k 20s 120s target/debug/compare_brix_ruiz_graph_paths \
  > tmp/sse-rust-a75-compare-graph-paths.txt
```

Validation:

```text
timeout -k 20s 120s cargo fmt --all
```

Elapsed under `1s`, exit `0`. No code was added; the focused classifier,
quotient, graph-comparison, and support-extraction commands above validate the
artifact-derived tables used in this note.
