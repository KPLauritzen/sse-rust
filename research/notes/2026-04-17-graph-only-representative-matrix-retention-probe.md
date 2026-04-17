# Graph-only round: canonical-only replay handles are a rejection (2026-04-17)

## Question

Can exact `graph-only` BFS keep less than one full representative matrix per
visited node without breaking:

- exact witness replay
- the current canonical-adjacency assumptions
- observer `SearchEdgeRecord` output

This round stayed intentionally narrow:

- exact `graph-only` BFS only
- no mixed, beam, or graph-plus-structured widening
- measurement-first on the existing retained deferred-witness seam

## Surface inspected

The tested surface is `search_graph_only_2x2_with_telemetry_and_observer` in
`src/search.rs`.

Representative matrices are still required in three distinct places on this
surface:

1. Search-time expansion.
   `orig` is read before every graph-only expansion so the search can enumerate
   successors from the kept concrete representative, not just from the
   canonical node id (`src/search.rs:3612-3623` and `3639-3643`).
2. Observer emission.
   `SearchEdgeRecord` currently carries `from_orig`, `to_orig`, and an exact
   `step`, and the exact step is rebuilt from the concrete representative pair
   (`src/search.rs:3673-3692`, `3739-3758`, `3788-3807`).
3. Final path replay.
   Path reconstruction walks canon parents but still materializes the stored
   representative matrix at every node (`src/search/path.rs:243-260`,
   `554-592`), then replays each adjacent matrix pair via exact one-step graph
   witness recovery (`src/search/path.rs:278-301`).

## Minimum replay handle

The minimum safe replay handle on this surface is **not** just the canonical
matrix key.

To preserve exact replay, the handle must retain enough information to recover
the same concrete representative that was chosen during canonical dedup. In
practice that means:

- either the full representative `DynMatrix`
- or a smaller reconstruction handle equivalent to “canonical matrix plus the
  exact permutation back to the kept representative”

Anything weaker loses information that the current replay and observer seams
assume is still available.

## Concrete counterexample

The new regression test
`test_graph_only_canonical_only_handles_cannot_replay_all_discovered_edges`
records a bounded failure at depth `0` from the Brix-Ruiz `k=3` source.

Current stored representative:

```text
current_orig = [[1, 3],
                [2, 1]]
current_canon = [[1, 2],
                 [3, 1]]
```

One kept discovered successor:

```text
next_orig = [[0, 0, 0],
             [2, 1, 2],
             [1, 3, 1]]
next_canon = [[0, 0, 0],
              [1, 1, 3],
              [2, 2, 1]]
```

What holds:

- `enumerate_graph_move_successor_nodes(current_orig, 5)` does keep
  `(next_canon, next_orig)` as a discovered graph-only edge
- `find_exact_graph_move_witness_between(current_orig, next_orig)` succeeds

What fails:

- `find_exact_graph_move_witness_between(current_canon, next_canon)` returns
  `None`

So a canonical-only per-node handle already loses exact one-edge replay on the
first expansion layer.

## Failure mode

This is a durable rejection of the “keep only the canonical matrix” idea for
exact graph-only BFS.

Why it fails:

- search deduplicates by canonical target, but the chosen representative edge is
  witnessed between concrete matrices, not between canonical matrices
- replacing the stored representative with the canonical matrix changes the
  exact adjacent pair that replay sees
- recovering the path from canonical matrices alone would require inserting
  extra permutation steps around affected edges

Those extra permutations break the current contract in two ways:

- witness lag is no longer the same one-edge-per-search-edge lag that the exact
  graph-only search reports
- observer `SearchEdgeRecord` can no longer emit the discovered edge as a
  single exact step with the current `from_orig` / `to_orig` payload shape

## Decision

Reject canonical-only replay handles on exact graph-only BFS.

On the tested surface, full representative-matrix retention is still the right
tradeoff unless a later bounded round also stores an explicit permutation
reconstruction handle per node and threads that through both deferred replay and
observer emission.

## Validation

Targeted checks run on this branch:

- `cargo test -q test_graph_only_canonical_only_handles_cannot_replay_all_discovered_edges -- --test-threads=1`
- `cargo test -q test_graph_only_dyn_reconstructs_deferred_witness_on_direct_successor -- --test-threads=1`
