# Graph-only round: defer exact witness reconstruction until after a meet (2026-04-17)

## Question

Can `graph_only` search avoid storing full per-node `EsseStep` payloads during
deep search, then recover the exact witness only after a successful meet?

This round stayed intentionally narrow:

- graph-only search only
- no mixed or graph-plus-structured changes
- measurement-first, with the accepted graph-only baseline set as the guardrail

## Current witness-storage shape

Before this prototype, graph-only BFS already reconstructed its final path from
parent chains, but it still carried full edge witnesses all the way through the
search:

- `src/search/path.rs` already had `walk_parent_chain`,
  `reconstruct_bidirectional_path`, and `reconstruct_bidirectional_dyn_path`
  for final path assembly.
- graph-only BFS parent maps in `src/search.rs` stored
  `HashMap<DynMatrix, Option<(DynMatrix, EsseStep)>>`, so every discovered node
  kept a cloned `EsseStep`.
- `src/graph_moves.rs` built `EsseStep` eagerly in
  `enumerate_graph_move_successors` and `push_canonical_graph_successor`, even
  though graph-only search only needs the canonical target and one
  representative matrix until it actually succeeds.

On the hard `brix_ruiz_k3_graph_only` exact positive, that meant carrying step
payloads across `1,399,061` visited nodes while the final witness lag was only
`17`.

## Prototype

Kept on this branch:

- added a lightweight graph-only successor surface in `src/graph_moves.rs`:
  `GraphMoveNode`, `GraphMoveNodes`, and
  `enumerate_graph_move_successor_nodes`
- changed graph-only BFS parent maps in `src/search.rs` to keep only
  `parent_canon`:
  `HashMap<DynMatrix, Option<DynMatrix>>`
- left the existing `orig` representative-matrix maps in place
- added deferred graph-only reconstruction in `src/search/path.rs`:
  `reconstruct_graph_only_bidirectional_path` and
  `reconstruct_graph_only_bidirectional_dyn_path`
- rebuilt exact one-step witnesses only after a meet by replaying adjacent
  matrix pairs:
  - permutation steps still use `permutation_step_between`
  - graph moves use `find_exact_graph_move_witness_between`
  - backward-side edges replay the opposite stored graph move and flip `(U,V)`
    to `(V,U)`, matching the old stepful reconstruction logic

Boundaries kept on purpose:

- mixed BFS, beam, and beam-to-BFS-handoff search still use the old witnessful
  edge storage
- observer mode stays correct by reconstructing an exact per-edge witness only
  when it needs to emit a `SearchEdgeRecord`

## Tradeoffs

Expected upside:

- graph-only BFS stops cloning and storing `EsseStep` per discovered node
- the hot graph-only exact-search path stops constructing witnesses before
  canonical dedup
- the success-only replay cost scales with witness lag instead of visited-node
  count

Costs:

- one more graph-move replay pass on the successful path
- graph-only observer mode is more expensive when enabled, because it now
  materializes edge witnesses on demand
- the graph-only successor enumeration now has a parallel lightweight path and a
  witnessful path, which is a maintenance cost

## Validation

Targeted correctness checks on this branch:

- `cargo test -q graph_only_dyn_reconstructs_deferred_witness_on_direct_successor -- --test-threads=1`
- `cargo test -q graph_only_bfs_falls_back_to_concrete_shift_on_lag_one_pair -- --test-threads=1`
- `cargo test -q find_graph_move_witnesses_between_finds_direct_successor -- --test-threads=1`

Accepted graph-only baseline reruns:

- `brix_ruiz_k3_graph_only`
- `brix_ruiz_k3_graph_only_beam_probe`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`

All accepted surfaces preserved outcome and counters.

## Measurements

### Same-worktree exact-solve before / after

Direct `search` binary on the hard exact graph-only solve:

```bash
target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only
```

Before the prototype:

- wall time `8.58 s`
- max RSS `3,132,960 KB`

After the prototype:

- wall time `7.07 s`
- max RSS `2,255,212 KB`

Delta:

- wall time `-1.51 s` (`-17.6%`)
- max RSS `-877,748 KB` (`-28.0%`)

The final witness stayed `17` steps.

### Accepted baseline set after the prototype

`brix_ruiz_k3_graph_only`:

- `equivalent` in `6724 ms`
- witness lag `17`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`
- `collisions_with_other_frontier = 1`
- `factorisations_enumerated = 0`

`brix_ruiz_k3_graph_only_beam_probe`:

- `unknown` in `76 ms`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5,372`
- `max_frontier_size = 10`
- `factorisations_enumerated = 0`

`brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`:

- `unknown` in `1923 ms`
- `frontier_nodes_expanded = 2,354`
- `total_visited_nodes = 128,118`
- `approximate_other_side_hits = 18`
- `factorisations_enumerated = 0`

`brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`:

- `unknown` in `3418 ms`
- `frontier_nodes_expanded = 3,634`
- `total_visited_nodes = 219,284`
- `approximate_other_side_hits = 86`
- `factorisations_enumerated = 0`

`brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`:

- `unknown` in `5011 ms`
- `frontier_nodes_expanded = 4,914`
- `total_visited_nodes = 305,954`
- `approximate_other_side_hits = 232`
- `factorisations_enumerated = 0`

Interpretation:

- the exact graph-only BFS surface shows a real memory win and a clear runtime
  win
- the beam-only control surfaces are unchanged in search shape and stay within
  the expected graph-only baseline envelope
- the `k4` reach controls are preserved exactly in counters; their single-run
  timings were slightly noisier than the earlier durable baseline, but those
  cases do not exercise the changed BFS reconstruction/storage path

## Decision

Keep, but scope the claim correctly.

This is a good bounded optimization for exact graph-only BFS:

- it materially reduces peak memory on the hard exact positive
- it also improved the exact `k=3` solve wall time in the same worktree
- it preserved the accepted graph-only baseline outcomes and counters
- it does not broaden into mixed-search behavior

What this is **not**:

- evidence that all graph-only modes get uniformly faster
- a reason to refactor mixed search around deferred witnesses yet

Recommendation:

- keep the graph-only BFS deferred-reconstruction change
- describe it primarily as a memory reduction for exact graph-only witness
  search, with a secondary exact-solve runtime win
- do not generalize the optimization claim beyond graph-only BFS until a later
  round measures beam and mixed modes directly
