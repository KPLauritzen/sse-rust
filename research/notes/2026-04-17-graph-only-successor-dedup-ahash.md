# Graph-only round: use `ahash` for successor dedup tables (2026-04-17)

## Question

On the bounded graph-only baseline set, is there one profiler-first low-level
change worth keeping in graph move generation or frontier bookkeeping without
changing pruning, ranking, or default search behavior?

## Baseline surfaces

This round stayed on the durable graph-only baseline set from
`research/notes/2026-04-17-graph-only-harness-baselines.md`:

- `brix_ruiz_k3_graph_only`
- `brix_ruiz_k3_graph_only_beam_probe`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`

Reference baseline numbers already recorded before this round:

- durable note baseline:
  - `brix_ruiz_k3_graph_only`: `8991 ms`
  - `brix_ruiz_k3_graph_only_beam_probe`: `111 ms`
  - `k4` lag `20 / 30 / 40`: `2578 / 4359 / 5923 ms`
- coordinator post-merge artifact summary:
  - `brix_ruiz_k3_graph_only`: about `8584 ms`
  - `brix_ruiz_k3_graph_only_beam_probe`: median about `62 ms`
  - `k4` lag `40`: about `4180 ms`

The saved post-merge artifact path from the dispatch was not present in this
checkout, so the keep or revert gate below uses fresh same-worktree reruns plus
the durable note as context.

## Profiling-first evidence

Direct profile command on the hard solve baseline:

```bash
target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --pprof
```

Pre-change sampled stacks stayed concentrated in graph successor generation:

- `append_representative_outsplit_successors`
- `push_canonical_graph_successor`
- `DynMatrix::canonical_perm`
- split-composition recursion inside `split_row_into_children`

The reusable low-level lead was specific and narrow: the local dedup tables in
`src/graph_moves.rs` were still using the standard SipHash path, and the
profile showed sampled `core::hash::sip::Hasher::write` frames under
`push_canonical_graph_successor`.

After the change, the same profile no longer showed those SipHash frames on the
graph successor dedup path. The hot work stayed in graph expansion and
canonicalization, but the local dedup tables were now on the `ahash` path.

Profile artifacts kept in this worktree:

- pre-change: `tmp/2uy19-graph-only-pprof.txt`
- post-change: `tmp/2uy19-after-graph-only-pprof.txt`

## Change

In `src/graph_moves.rs`:

- switched the internal `HashMap` and `HashSet` imports from `std` to
  `ahash::{AHashMap, AHashSet}`

That changes only the ephemeral dedup and lookup tables used while building
graph successors and related graph-move helpers. No search-policy logic,
pruning thresholds, ranking, or public output shape changed.

## Validation

Focused graph-only reruns after the change:

- `target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k3_graph_only`
- `target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`
- `target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`
- `target/release/research_harness --cases research/cases.json --format json --worker-case brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`

For the beam surface, `--worker-case` only gives a single run, so the
apples-to-apples keep gate used a one-case corpus:

```bash
jq '{schema_version, cases: [.cases[] | select(.id == "brix_ruiz_k3_graph_only_beam_probe")]}' \
  research/cases.json > tmp/2uy19-beam-case.json

target/release/research_harness --cases tmp/2uy19-beam-case.json --format json
```

To avoid comparing against a stale artifact on that cheap surface, I also did a
same-session A/B by temporarily flipping the one-line import change off and on.

## Before / After

### `brix_ruiz_k3_graph_only`

Same-session pre-change local rerun:

- before: `8440 ms`
- outcome `equivalent`
- witness lag `17`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`

Post-change sequential reruns:

- after: `8295 / 8183 / 8133 ms`, median `8183 ms`
- delta vs same-session pre-change: `-257 ms` (`-3.0%`)
- delta vs coordinator post-merge reference `8584 ms`: `-401 ms` (`-4.7%`)

Search shape stayed identical on every rerun:

- outcome `equivalent`
- witness lag `17`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`
- `max_frontier_size = 717,764`

### `brix_ruiz_k3_graph_only_beam_probe`

Same-session one-case harness A/B:

- before samples: `66 / 70 / 72 / 74 / 74 ms`, median `72 ms`
- after samples: `66 / 70 / 72 / 75 / 75 ms`, median `72 ms`
- delta: neutral

Outcome and counters stayed identical:

- outcome `unknown`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5,372`
- `max_frontier_size = 10`

Interpretation:

- the durable artifact summary reported a faster historical median (`~62 ms`),
  but on this same-session worktree A/B the import change is neutral on the beam
  control rather than regressive

### `brix_ruiz_k4_graph_only_boundary_ramp`

Post-change reruns versus the durable baseline note:

- lag `20`: `2578 -> 1988 ms` (`-22.9%`)
- lag `30`: `4359 -> 3519 ms` (`-19.3%`)
- lag `40`: `5923 -> 5005 ms` (`-15.5%`)

All three reach surfaces stayed identical in search shape:

- outcome `unknown`
- lag `20`: `frontier_nodes_expanded = 2,354`, `total_visited_nodes = 128,118`,
  `approximate_other_side_hits = 18`
- lag `30`: `frontier_nodes_expanded = 3,634`, `total_visited_nodes = 219,284`,
  `approximate_other_side_hits = 86`
- lag `40`: `frontier_nodes_expanded = 4,914`, `total_visited_nodes = 305,954`,
  `approximate_other_side_hits = 232`

## Decision

Keep.

This is a bounded graph-only-local change backed by profiler evidence. On this
worktree it is:

- clearly faster on the hard exact graph-only solve surface
- clearly faster on the three graph-only `k=4` reach surfaces
- neutral on the cheap beam control under same-session A/B
- identical on graph-only outcomes, lag, and frontier counters

That satisfies the keep gate for this round without broadening into new search
policy changes.
