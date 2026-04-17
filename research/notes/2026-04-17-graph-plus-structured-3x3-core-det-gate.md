# Graph-plus-structured round: gate `3x3` core adjugates behind determinant checks (2026-04-17)

## Question

On the new graph-plus-structured baseline surfaces, is there one small
profiler-led low-level change worth keeping without touching pruning, ranking,
or default search behavior?

## Baselines

Kept this round strictly on the two new graph-plus-structured baseline
surfaces from `sse-rust-2uy.22`:

- `brix_ruiz_k3_graph_plus_structured`
- `brix_ruiz_k3_graph_plus_structured_beam_probe`

Local A/B commands in this worktree:

```bash
target/dist/research_harness --cases research/cases.json --format json \
  --worker-case brix_ruiz_k3_graph_plus_structured

target/dist/research_harness --cases tmp/2uy23-beam-case.json --format json
```

`tmp/2uy23-beam-case.json` is the one-case corpus extracted from
`research/cases.json` with `schema_version` preserved.

## Profiling-first evidence

I profiled the solve baseline directly on the matching endpoint-search
configuration:

```bash
target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 8 \
  --max-intermediate-dim 4 \
  --max-entry 5 \
  --move-policy graph-plus-structured \
  --json --telemetry --pprof
```

The hot samples stayed concentrated in structured sparse factorisation work:

- `expand_frontier_node`: `808` sampled stacks
- `enumerate_binary_sparse_factorisation_3x3_to_4_family`: `536`
- `adjugate_matrix_and_det_3x3`: `173`
- `solve_nonneg_3x3_with_adjugate`: `158`
- `enumerate_binary_sparse_factorisation_4x4_to_3_family`: `101`

The solve baseline telemetry also showed that this round is not dominated by
graph-only bookkeeping:

- total expand-compute time across layers: about `1066 ms`
- total dedup time across layers: about `521 ms`
- total merge time across layers: about `596 ms`

For the hot structured `3x3 -> 4x4` family, the reusable core-space is very
small: only `216` binary `3x3` cores are possible, and `114` of them
(`52.78%`) are singular. That made one bounded low-level lead clear: do not
pay for a full adjugate on singular cores, and do not recompute the same
determinant when the caller already has it.

## Change

In `src/factorisation.rs`:

- split `adjugate_matrix_3x3` from `adjugate_matrix_and_det_3x3`
- in `visit_binary_sparse_factorisations_4x4_to_3`, keep the existing cheap
  determinant guard and then compute only the adjugate
- in `visit_binary_sparse_factorisations_3x3_to_4`, add an explicit `det3x3`
  gate before computing the adjugate

No pruning, ranking, move-family selection, or frontier policy logic changed.

## Correctness gate

Focused tests on the touched path:

- `cargo test -q solve_nonneg_3x3_with_adjugate_matches_solver`
- `cargo test -q graph_plus_structured`

Both passed.

## Before / After

### Solve baseline

Three local reruns of
`brix_ruiz_k3_graph_plus_structured` on the same worktree:

- before: `2139 / 2175 / 2168 ms`, median `2168 ms`
- after: `2067 / 2046 / 2066 ms`, median `2066 ms`
- delta: `-102 ms` (`-4.7%`)

Outcome and search shape stayed identical:

- outcome `equivalent`
- witness lag `8`
- `frontier_nodes_expanded = 84,875`
- `total_visited_nodes = 212,170`
- `factorisations_enumerated = 470,662`

### Beam baseline

Repeated one-case harness measurement for
`brix_ruiz_k3_graph_plus_structured_beam_probe`:

- before samples: `43 / 43 / 45 / 54 / 56 ms`, median `45 ms`
- after samples: `34 / 36 / 36 / 42 / 53 ms`, median `36 ms`
- delta: `-9 ms` (`-20.0%`)

Outcome and search shape stayed identical:

- outcome `unknown`
- `frontier_nodes_expanded = 142`
- `total_visited_nodes = 2,631`
- `factorisations_enumerated = 22,301`

## Decision

Keep.

This is a bounded profiler-first win on the exact new graph-plus-structured
baseline surfaces. It preserves outcomes and counters, trims redundant
structured-family `3x3` arithmetic, and improves both the exact solve lane and
the cheap beam control.
