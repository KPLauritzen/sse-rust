# Mixed `k3` frontier benchmark regression follow-up after 5x5 graph-only canonicalization change (2026-04-17)

## Question

Did `sse-rust-2uy.26` introduce a real mixed-search regression on
`expand_next_n/frontier_expansion/mixed_k3_lag3_dim3_n2048`, or was the
post-merge Criterion report an incidental benchmark fluctuation unrelated to
the new `5x5` canonicalization path?

This follow-up stayed intentionally bounded:

- confirm the mixed benchmark signal in isolation;
- profile the matching mixed search configuration;
- determine whether `canonical_perm_5x5` can be reached on this surface;
- keep or revert `sse-rust-2uy.26` based on that evidence only.

## Surfaces investigated

Benchmark surface from `benches/search.rs`:

- `expand_next_n/frontier_expansion/mixed_k3_lag3_dim3_n2048`
- endpoints `[[1, 3], [2, 1]] -> [[1, 6], [1, 1]]`
- `max_lag = 3`
- `max_intermediate_dim = 3`
- `max_entry = 6`
- `move_family_policy = mixed`

The original concern was the post-merge report of an apparent slowdown around
`+9.7%` to `+23.7%` on this single Criterion surface after the `5x5`
graph-only permutation-scan optimization in `src/matrix.rs`.

## Code-path check first

Before profiling or reverting anything, check whether this surface can even hit
the new `5x5` canonicalization path.

Relevant code:

- `benches/search.rs` fixes `max_intermediate_dim = 3` for this benchmark
- `src/graph_moves.rs:482` only appends outsplit successors when
  `current.rows < max_dim`
- `src/graph_moves.rs:493` only appends amalgamations when `current.rows > 2`

Implication:

- on this surface, graph successors can only move between `2x2` and `3x3`
- no graph successor can grow to `4x4` or `5x5`
- the benchmark therefore cannot reach `DynMatrix::canonical_perm_5x5` through
  its graph-move lane

That does not by itself rule out some broader mixed-search regression, but it
does rule out the most direct hypothesis that the new `5x5` code path is being
exercised here.

## Isolated benchmark reruns

### Current branch (`136cf28`, `sse-rust-2uy.26` merged)

Command:

```bash
cargo bench --bench search -- --noplot mixed_k3_lag3_dim3_n2048
```

Observed reruns:

- run 1: `time: [502.26 ms 504.79 ms 508.34 ms]`
- run 2: `time: [513.09 ms 515.56 ms 517.85 ms]`

These two same-commit reruns already show drift of roughly `2%` without any
code changes in between.

### Pre-change comparison commit (`9c5c81a`, parent of `sse-rust-2uy.26`)

Same command from a separate worktree at `9c5c81a`:

- `time: [505.36 ms 507.10 ms 509.31 ms]`
- Criterion reported
  `change: [+0.1614% +0.6959% +1.2626%] (p = 0.03 < 0.05)`
  followed by `Change within noise threshold.`

Interpretation:

- the pre-change commit was not faster in any meaningful way
- the measured difference versus the immediately preceding current-branch run
  stayed around `0.2%` to `1.3%`
- that is an order of magnitude smaller than the originally reported
  `+9.7%` to `+23.7%` slowdown

Across all isolated reruns in this follow-up, the mixed surface stayed in a
rough `503 ms` to `516 ms` band. This is consistent with normal Criterion noise
on a sub-second parallel benchmark, not a durable double-digit regression tied
to the `sse-rust-2uy.26` code change.

## Matching mixed-search profile

Profile command:

```bash
cargo run --release --features=pprof-profile --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 3 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-family-policy mixed \
  --pprof \
  --telemetry
```

Saved artifact:

- `tmp/mixed-k3-search-pprof.txt`

Observed telemetry on this surface:

- outcome `UNKNOWN`
- `frontier nodes expanded = 205`
- `factorisations enumerated = 153,972`
- `candidates after pruning = 2,019`
- `total visited nodes = 1,815`
- layer timing split:
  `compute ~= 42-44 ms`,
  `dedup ~= 6.9-7.8 ms`,
  everything else smaller

The sampled profile is dominated by mixed frontier expansion and `3x3`
factorisation work. Grepping the saved profile output gives:

- `expand_frontier_node`: `50`
- `enumerate_square_factorisation_3x3_family`: `45`
- `solve_nonneg_2x3_into`: `24`
- `solve_nonneg_3x3`: `9`
- `canonical_perm_5x5`: `0`
- `canonical_perm_4x4`: `0`
- `canonical_perm_3x3`: `0`

Interpretation:

- the hotspot on this mixed surface is the structured-factorisation lane, not
  `5x5` graph canonicalization
- no sampled evidence showed `canonical_perm_5x5` participating in the profiled
  run
- the original regression report therefore does not have a plausible profiler
  path back to the `sse-rust-2uy.26` `5x5` change

## Keep / revert decision

Keep `sse-rust-2uy.26`.

Reasons:

- this follow-up did not reproduce any double-digit slowdown on the mixed
  `k3` frontier surface
- the pre-change commit did not benchmark materially faster than the merged
  version
- code-path inspection shows the benchmark is dimension-bounded away from `5x5`
- the direct mixed-search profile points at `3x3` factorisation work, not the
  new `5x5` canonicalization routine

## Recommendation

Recommendation: keep, no revert.

If this benchmark regresses again later, treat it as a fresh benchmark hygiene
question rather than as fallout from `sse-rust-2uy.26`. The next bounded step,
if needed, should target mixed `3x3` factorisation throughput or reduce
Criterion variance on this surface, not revert the graph-only `5x5`
canonicalization optimization.
