# Graph-only round: use node-only expansion when exact edge witnesses are not needed (2026-04-18)

## Question

After the landed graph-only rounds for successor-dedup hashing, `5x5`
canonical-permutation pruning, and deferred exact witness reconstruction, is
there one more bounded graph-only hot-path optimization worth keeping?

This round stayed intentionally narrow:

- profile first on the accepted graph-only exact control;
- keep the change graph-only-local;
- do not change search policy, move-family coverage, or campaign scope;
- keep only one real optimization if the accepted graph-only surfaces justify it.

## Hotspot Followed

Profile command on the hard exact graph-only surface:

```bash
target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --pprof
```

Before the change, the graph-only search hot path was still spending samples in
the **stepful** graph successor enumerator even though exact witnesses are
already reconstructed only after a meet:

- `enumerate_graph_move_successors`: `1366`
- `append_representative_outsplit_successors`: `716`
- `DynMatrix::canonical_perm`: `432`
- `HashMap<K,V,S,A>::insert`: `141`

The exact lead followed in this round was:

- graph-only BFS and dynamic graph-only search were still expanding through
  `enumerate_graph_move_successors`, materializing `EsseStep` payloads and
  witness-side matrices for every explored edge;
- those step payloads are only needed for the 2x2 observer edge stream, not for
  ordinary graph-only search or final witness reconstruction.

After the change, the same profile no longer showed the stepful graph-only
enumerator on the hot path in practice:

- `enumerate_graph_move_successors`: `1`
- `enumerate_graph_move_successor_nodes`: `1515`
- `append_representative_outsplit_successors`: `1`
- `append_representative_outsplit_successor_nodes`: `736`

Interpretation:

- the graph-only hot path now expands through the lighter node-only successor
  surface when exact per-edge witnesses are not needed;
- the remaining time is still real graph-only work: outsplit generation,
  canonicalization, and dedup;
- observer-capable 2x2 graph-only search keeps the old stepful path so emitted
  edge records stay exact.

Profile artifacts kept in this worktree:

- pre-change: `tmp/0y7-graph-only-pre-pprof.txt`
- post-change: `tmp/0y7-graph-only-post-pprof.txt`

## Change

Kept one bounded optimization:

- `src/search.rs`: graph-only search now uses a node-only successor expansion
  path when exact edge witnesses are not needed, instead of always materializing
  `EsseStep`s during search
- `src/graph_moves.rs`: the node-only outsplit helper now reuses the split
  parent’s division matrix instead of rebuilding it for every split

Scope intentionally not changed:

- no pruning, ranking, or frontier-policy rewrite
- no new move families
- no graph-plus-structured or mixed-search retuning
- no Riedel or campaign widening

## Commands Used

Before/after graph-only surface measurements:

```bash
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k3_graph_only
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k3_graph_only_beam_probe
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12
target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12
```

Focused tests:

```bash
cargo test -q graph_only_dyn_reconstructs_deferred_witness_on_direct_successor -- --test-threads=1
cargo test -q test_graph_only_bfs_falls_back_to_concrete_shift_on_lag_one_pair -- --test-threads=1
cargo test -q test_graph_only_canonical_only_handles_cannot_replay_all_discovered_edges -- --test-threads=1
```

Formatting and bench control:

```bash
cargo fmt
cargo bench --bench search -- --noplot
```

## Before / After

These keep/reject numbers use same-session isolated reruns for the accepted
graph-only surfaces.

### `brix_ruiz_k3_graph_only`

- before: `8108 ms`
- after: `6747 ms`
- delta: `-1361 ms` (`-16.8%`)

Search shape stayed identical:

- outcome `equivalent`
- witness lag `17`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`
- `max_frontier_size = 717,764`
- `factorisations_enumerated = 0`

### `brix_ruiz_k3_graph_only_beam_probe`

- before: `83 ms`
- after: `69 ms`
- delta: `-14 ms` (`-16.9%`)

Search shape stayed identical:

- outcome `unknown`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5,372`
- `max_frontier_size = 10`
- `factorisations_enumerated = 0`

### `brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`

- before: `2573 ms`
- after: `1936 ms`
- delta: `-637 ms` (`-24.8%`)

Search shape stayed identical:

- outcome `unknown`
- `frontier_nodes_expanded = 2,354`
- `total_visited_nodes = 128,118`
- `approximate_other_side_hits = 18`
- `factorisations_enumerated = 0`

### `brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`

- before: `4509 ms`
- after: `3430 ms`
- delta: `-1079 ms` (`-23.9%`)

Search shape stayed identical:

- outcome `unknown`
- `frontier_nodes_expanded = 3,634`
- `total_visited_nodes = 219,284`
- `approximate_other_side_hits = 86`
- `factorisations_enumerated = 0`

### `brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`

- before: `6102 ms`
- after: `4939 ms`
- delta: `-1163 ms` (`-19.1%`)

Search shape stayed identical:

- outcome `unknown`
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

- `endpoint_equivalent_fast`: `2.5958 µs .. 2.6142 µs`
- `endpoint_invariant_reject_fast`: `3.7112 µs .. 3.7389 µs`
- `expand_next_n/frontier_expansion/mixed_k3_lag3_dim3_n2048`:
  `513.37 ms .. 526.18 ms`
- `expand_next_n/frontier_expansion/graph_only_k3_lag8_dim4_n8192`:
  `73.866 ms .. 74.525 ms`

For this round the Criterion suite was a final control run, not the keep/reject
gate. The keep gate stayed on the accepted graph-only harness surfaces above.

## Decision

Keep.

This change is small, graph-only-local, and backed by measurement:

- the pre-change profile showed graph-only search still paying for stepful
  successor materialization on the hot path;
- the post-change profile moved that hot path onto the existing node-only
  successor surface;
- every accepted graph-only control kept the same outcome and counters;
- the isolated before/after reruns showed clear wins on the exact `k=3` solve,
  the beam probe, and all three accepted `k=4` reach surfaces.
