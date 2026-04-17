# Graph-only round: prune impossible 5x5 canonical permutation scans (2026-04-17)

## Question

On the durable graph-only baseline set, is there one more profiler-first local
change worth keeping inside graph-only successor canonicalization before
rerunning the bounded `k=4` graph-only campaign?

This round stayed intentionally narrow:

- profile first on the hard graph-only solve baseline;
- keep only graph-only-local low-level work;
- do not mix in campaign writeup changes, frontier-policy changes, or broader
  solver rewrites.

## Baseline surfaces

This round stayed on the durable graph-only baseline set from
`research/notes/2026-04-17-graph-only-harness-baselines.md`:

- `brix_ruiz_k3_graph_only`
- `brix_ruiz_k3_graph_only_beam_probe`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`

The before side of the keep/revert gate uses the current durable baseline note:

- `brix_ruiz_k3_graph_only`: `8991 ms`
- `brix_ruiz_k3_graph_only_beam_probe`: `111 ms`
- `k4` lag `20 / 30 / 40`: `2578 / 4359 / 5923 ms`

## Profiling-first evidence

Profile command on the hard exact graph-only surface:

```bash
target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --pprof
```

Pre-change sampled stacks stayed centered in graph successor generation. The
main hotspot families were:

- `DynMatrix::canonical_perm` inside `push_canonical_graph_successor`
- `next_permutation` inside the `5x5` canonicalization path
- outsplit row splitting in `append_representative_outsplit_successors` /
  `recurse_split_row_columns`
- a smaller residual hash-table path during successor dedup

Counted sampled frames from the saved profile in `tmp/2uy26-pre-pprof.txt`:

- `DynMatrix::canonical_perm`: `646`
- `next_permutation`: `254`
- `push_canonical_graph_successor`: `748`
- `append_representative_outsplit_successors`: `698`
- `recurse_split_row_columns`: `257`

The actionable lead was specific: the `5x5` canonical path was still scanning
all `5!` permutations and filtering almost all of them away even when the
invariant grouping already proved that only within-group swaps could matter.

Post-change sampled frames from `tmp/2uy26-post-pprof.txt`:

- `DynMatrix::canonical_perm`: `412`
- `next_permutation`: `84`
- `push_canonical_graph_successor`: `528`
- `append_representative_outsplit_successors`: `545`
- `recurse_split_row_columns`: `396`

Interpretation:

- the `next_permutation` hotspot family dropped sharply;
- canonicalization is still hot, but less dominant;
- the remaining work is now more concentrated in genuine successor generation
  rather than wasted cross-group permutation scans.

## Change

In `src/matrix.rs`:

- kept the existing `5x5` invariant grouping rule
- replaced the `5!` full-permutation scan plus `perm_respects_groups` filter
  with direct enumeration of only the contiguous invariant-group runs
- added an early return when every grouped vertex is already singleton-valued

No pruning rule, search policy, frontier ranking, move-family policy, or public
result surface changed.

## Validation

Correctness checks after the change:

- `cargo test -q test_dyn_canonical_`
- `cargo build --release --features research-tools --bin research_harness --bin search`

Focused graph-only measurement reruns after the change:

- `brix_ruiz_k3_graph_only`: `3` same-worktree runs
- `brix_ruiz_k3_graph_only_beam_probe`: one-case harness corpus with
  `measurement.repeat_runs = 5`
- each `k4` boundary-ramp deepening point: one focused rerun

All accepted graph-only baselines preserved outcome and counters.

## Before / After

### `brix_ruiz_k3_graph_only`

Before durable baseline:

- `8991 ms`
- outcome `equivalent`
- witness lag `17`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`

After same-worktree reruns:

- `8855 / 8840 / 8829 ms`, median `8840 ms`
- delta vs baseline note: `-151 ms` (`-1.7%`)
- outcome stayed `equivalent`
- witness lag stayed `17`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`
- `max_frontier_size = 717,764`

### `brix_ruiz_k3_graph_only_beam_probe`

Before durable baseline:

- median `111 ms`
- outcome `unknown`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5,372`
- `max_frontier_size = 10`

After same-worktree reruns:

- samples `61 / 67 / 69 / 71 / 77 ms`, median `69 ms`
- delta vs baseline note: `-42 ms` (`-37.8%`)
- outcome stayed `unknown`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5,372`
- `max_frontier_size = 10`

### `brix_ruiz_k4_graph_only_boundary_ramp`

Before durable baseline:

- lag `20`: `2578 ms`
- lag `30`: `4359 ms`
- lag `40`: `5923 ms`

After focused reruns:

- lag `20`: `1769 ms` (`-31.4%`)
- lag `30`: `3185 ms` (`-26.9%`)
- lag `40`: `4553 ms` (`-23.1%`)

The reach surfaces stayed identical in search shape:

- lag `20`: outcome `unknown`,
  `frontier_nodes_expanded = 2,354`,
  `total_visited_nodes = 128,118`,
  `approximate_other_side_hits = 18`
- lag `30`: outcome `unknown`,
  `frontier_nodes_expanded = 3,634`,
  `total_visited_nodes = 219,284`,
  `approximate_other_side_hits = 86`
- lag `40`: outcome `unknown`,
  `frontier_nodes_expanded = 4,914`,
  `total_visited_nodes = 305,954`,
  `approximate_other_side_hits = 232`

All three kept `factorisations_enumerated = 0`.

## Kept / Reverted

Kept:

- direct within-group permutation enumeration for `5x5`
  `DynMatrix::canonical_perm`

Reverted:

- none

Not changed on purpose:

- no graph frontier policy changes
- no ranking or pruning changes
- no campaign-writeup changes
- no work on `sse-rust-2uy.21` itself beyond the recommendation below

## Decision

Keep.

This is a bounded graph-only-local optimization backed by profile evidence and
accepted by the full graph-only baseline set:

- pre-change profiling showed wasted time in impossible `5x5` permutation scans
- post-change profiling showed that hotspot family materially reduced
- every accepted graph-only baseline preserved outcome, lag, and counters
- timing improved across the exact `k=3` and `k=4` surfaces that the next
  graph-only campaign depends on

`sse-rust-2uy.21` should now be rerun on this optimized graph-only baseline
rather than on the earlier pre-round baseline.
