# Graph-only round: use node-only amalgamation construction on the graph-only hot path (2026-04-18)

## Question

After the earlier graph-only rounds removed eager witness storage and moved the
hot path onto node-only graph successor expansion, is there one more bounded
graph-only-local cleanup worth keeping?

This round stayed intentionally narrow:

- profile first on the accepted exact graph-only control;
- keep any change inside graph-only successor generation only;
- do not change pruning, ranking, move-family coverage, or bidirectional search
  policy;
- keep the change only if accepted graph-only surfaces stay stable and at least
  one accepted runtime surface improves with same-session evidence.

## Hotspot Evidence Followed

Profile command on the hard exact graph-only surface:

```bash
timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --pprof > tmp/sse-rust-csl-pre-pprof.txt 2>&1
```

Before the change, the graph-only node-only successor path still spent hot-path
samples in the **witnessful** amalgamation builders:

- `enumerate_out_amalgamations -> enumerate_in_amalgamations -> enumerate_graph_move_successor_nodes`
- `DynMatrix::canonical_perm -> push_canonical_graph_successor_node`
- `Vec::clone` inside `append_representative_outsplit_successor_nodes`
- `HashMap::insert` on graph-only successor dedup

The exact lead followed here was specific:

- `enumerate_graph_move_successor_nodes` already avoids constructing
  `EsseStep`s for graph-only search;
- but its amalgamation branch was still calling `enumerate_out_amalgamations`
  and `enumerate_in_amalgamations`, which build full `OutsplitWitness`
  payloads (`division`, `edge`, and `outsplit`) even though graph-only search
  only needs the resulting successor matrix there.

After the change, the same profile no longer showed the witnessful amalgamation
enumerators on the hot path. The relevant frames instead shifted to the new
node-only helper:

- `append_out_amalgamation_successor_nodes -> enumerate_graph_move_successor_nodes`
- `DynMatrix::canonical_perm -> push_canonical_graph_successor_node`
- `append_representative_outsplit_successor_nodes` and split generation remain
  real hot work

Profile artifacts kept in this worktree:

- pre-change: `tmp/sse-rust-csl-pre-pprof.txt`
- post-change: `tmp/sse-rust-csl-post-pprof.txt`

## Change

Kept one bounded optimization in `src/graph_moves.rs`:

- added direct node-only amalgamation construction for
  `enumerate_graph_move_successor_nodes`
- kept the existing witnessful amalgamation enumerators unchanged for code
  paths that still need exact one-step witnesses
- added a focused equivalence test showing the node-only graph successor surface
  still matches the stepful graph successor targets on an amalgamation-bearing
  matrix

Scope intentionally not changed:

- no new move families
- no mixed or graph-plus-structured changes
- no frontier-policy rewrite
- no memory-budget or deeper parallelism redesign

## Commands Used

Hotspot and wall/RSS probes:

```bash
timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --pprof > tmp/sse-rust-csl-pre-pprof.txt 2>&1

/usr/bin/time -f 'wall=%e rss_kb=%M' \
  timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only > tmp/sse-rust-csl-pre-time.txt 2>&1

timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --pprof > tmp/sse-rust-csl-post-pprof.txt 2>&1

/usr/bin/time -f 'wall=%e rss_kb=%M' \
  timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only > tmp/sse-rust-csl-post-time.txt 2>&1
```

Focused tests:

```bash
cargo test -q test_graph_move_successor_nodes_match_stepful_targets_on_amalgamation_surface -- --test-threads=1
cargo test -q test_find_graph_move_witnesses_between_finds_direct_successor -- --test-threads=1
cargo test -q test_graph_only_canonical_only_handles_cannot_replay_all_discovered_edges -- --test-threads=1
```

Accepted baseline reruns on the kept tree:

```bash
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k3_graph_only
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k3_graph_only_beam_probe
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12
```

Same-session A/B on a reverted tree for one cheap accepted control and one
harder reach control:

```bash
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k3_graph_only_beam_probe
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12
```

Formatting and final control:

```bash
cargo fmt
cargo bench --bench search -- --noplot
```

`cargo fmt` completed normally in this worktree; the known formatter hang did
not reproduce here, so no fallback was needed.

## Before / After

### Direct exact graph-only solve (`brix_ruiz_k3` endpoint)

Same-worktree `/usr/bin/time` before / after:

- before: `wall=7.40`, `rss_kb=7716`
- after: `wall=7.21`, `rss_kb=7744`
- delta: `-0.19 s` (`-2.6%`) wall, RSS effectively neutral

The exact witness stayed `17` steps.

### Same-session A/B on accepted graph-only controls

`brix_ruiz_k3_graph_only_beam_probe`:

- reverted tree: `75 ms`
- kept tree after format: `72 ms`
- delta: `-3 ms` (`-4.0%`)

Search shape stayed identical:

- outcome `unknown`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5,372`
- `max_frontier_size = 10`
- `factorisations_enumerated = 0`

`brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`:

- reverted tree: `5021 ms`
- kept tree after format: `4949 ms`
- delta: `-72 ms` (`-1.4%`)

Search shape stayed identical:

- outcome `unknown`
- `frontier_nodes_expanded = 4,914`
- `total_visited_nodes = 305,954`
- `approximate_other_side_hits = 232`
- `factorisations_enumerated = 0`

Interpretation:

- the hard `k4` reach surface moved only slightly and I do **not** treat that
  as a strong runtime claim;
- the exact graph-only solve and the cheap beam probe both moved in the right
  direction on same-session measurements;
- the hotspot evidence also shows the intended work was actually removed from
  the hot path rather than shifted accidentally into mixed-search code.

### Accepted graph-only baseline set on the kept tree

Post-format accepted reruns:

`brix_ruiz_k3_graph_only`:

- `equivalent` in `6685 ms`
- witness lag `17`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`
- `max_frontier_size = 717,764`
- `factorisations_enumerated = 0`

`brix_ruiz_k3_graph_only_beam_probe`:

- `unknown` in `72 ms`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5,372`
- `max_frontier_size = 10`
- `factorisations_enumerated = 0`

`brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`:

- `unknown` in `1939 ms`
- `frontier_nodes_expanded = 2,354`
- `total_visited_nodes = 128,118`
- `approximate_other_side_hits = 18`
- `factorisations_enumerated = 0`

`brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`:

- `unknown` in `3524 ms`
- `frontier_nodes_expanded = 3,634`
- `total_visited_nodes = 219,284`
- `approximate_other_side_hits = 86`
- `factorisations_enumerated = 0`

`brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`:

- `unknown` in `4949 ms`
- `frontier_nodes_expanded = 4,914`
- `total_visited_nodes = 305,954`
- `approximate_other_side_hits = 232`
- `factorisations_enumerated = 0`

## Criterion Control

Final validation run:

```bash
cargo bench --bench search -- --noplot
```

Observed ranges:

- `endpoint_equivalent_fast`: `2.6766 µs .. 2.7063 µs`
- `endpoint_invariant_reject_fast`: `3.8107 µs .. 3.8362 µs`
- `expand_next_n/frontier_expansion/mixed_k3_lag3_dim3_n2048`:
  `522.79 ms .. 526.30 ms`
- `expand_next_n/frontier_expansion/graph_only_k3_lag8_dim4_n8192`:
  `73.347 ms .. 75.000 ms`

For this round the Criterion suite was a final control, not the keep gate. The
keep gate stayed on the graph-only hotspot evidence plus the accepted graph-only
surfaces above.

## Decision

Keep, but scope the claim narrowly.

This is a small graph-only-local runtime cleanup:

- the before/after profiles show graph-only search no longer paying for the
  witnessful amalgamation builders on the node-only hot path;
- the accepted graph-only surfaces preserved outcomes and counters exactly;
- the hard exact graph-only solve improved modestly on the direct wall path;
- the cheap accepted beam probe also improved modestly in same-session A/B.

What this change is **not**:

- evidence of a material memory/headroom improvement;
- evidence that the harder `k4` reach surfaces improved in a strong way;
- a reason to broaden into search-policy or staging redesign from this round.

So the right claim for this slice is:

- keep the node-only amalgamation construction because it removes unnecessary
  witness construction from graph-only successor generation and yields a small,
  measurable runtime win on accepted graph-only controls without changing
  search shape.
