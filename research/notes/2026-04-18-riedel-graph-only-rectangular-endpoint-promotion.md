# Retained Riedel graph-only promotion: rectangular endpoint lifts on the retained lane (2026-04-18)

## Goal

Promote only the retained `2x2 <-> 3x3` endpoint-lift families implicated by
the kept low-rung Riedel witnesses into `graph_only` on the retained
`max_intermediate_dim = 3` lane, then rerun the durable Riedel benchmark lane
and record whether the lane moves.

This slice stays narrow:

- no broad graph-only rewrite;
- no new speculative structured families beyond the retained endpoint lifts;
- preserve merge-safe behavior for `just check-k3-graph-merge`; and
- keep the outcome attached to the committed retained lane.

## Chosen promotion

Promoted families:

- `rectangular_factorisation_2x3`
- `rectangular_factorisation_3x3_to_2`

Policy seam:

- [`src/factorisation.rs`](../../src/factorisation.rs) now exposes those two
  families to `MoveFamilyPolicy::GraphOnly`;
- like the earlier retained `elementary_conjugation_3x3` lift already on
  `main`, the endpoint lifts are gated to the retained dim-3 surface only:
  `max_intermediate_dim == 3`; and
- broader `graph_only` searches with `max_intermediate_dim > 3` still skip the
  promotion so the merge check does not inherit the extra branching.

Why this exact slice:

- the retained witness-classification note
  [`2026-04-18-riedel-witness-classification-k4-k6.md`](./2026-04-18-riedel-witness-classification-k4-k6.md)
  identifies those endpoint lifts as the next unresolved one-step families
  after the retained conjugation-only pass;
- they are exact one-step SSE families, so the existing graph-only path replay
  seam can still validate witnesses exactly; and
- the gate stays bounded to the retained lane instead of widening graph-only
  factorisation policy in general.

## Focused validation

Focused code validation used for the promotion:

- `cargo test --lib test_selected_family_labels_for_graph_only_`
- `cargo test --lib test_graph_only_dyn_promotes_retained_`
- `cargo test --lib test_graph_only_dyn_reconstructs_deferred_witness_on_direct_successor`
- `cargo test --lib test_expand_frontier_layer_graph_only_skips_factorisations`
- `just check-k3-graph-merge`

Formatter:

- `cargo fmt`

## Retained-lane rerun

Retained rerun command:

```bash
cargo build --profile dist --features research-tools --bin research_harness

timeout -k 10s 45s target/dist/research_harness \
  --cases research/riedel_gap_benchmark_lane_2026-04-18.json \
  --format json \
  > tmp/riedel_gap_benchmark_lane_run_2026-04-18_graph_only_retained_rectangular_endpoints.json
```

Retained-lane outcome:

- `graph_only` still solves `0/6` retained rungs
- `graph_plus_structured` still solves `6/6`
- `mixed` still solves `6/6`

`graph_only` rerun details:

| Rung | Outcome | Elapsed |
| --- | --- | --- |
| `k = 4` | `unknown` | `2 ms` |
| `k = 6` | `unknown` | `5 ms` |
| `k = 8` | `unknown` | `10 ms` |
| `k = 10` | `unknown` | `23 ms` |
| `k = 12` | `unknown` | `51 ms` |
| `k = 14` | `unknown` | `111 ms` |

Totals across the retained lane:

- `graph_only`: `0/6 equivalent`, `6/6 unknown`, total elapsed `202 ms`
- `graph_plus_structured`: `6/6 equivalent`, total elapsed `9060 ms`
- `mixed`: `6/6 equivalent`, total elapsed `10097 ms`

The promotion definitely changed bounded reach:

- `rectangular_factorisation_2x3` generated `29566` raw candidates across the
  retained graph-only lane and discovered `1680` canonical successors;
- `rectangular_factorisation_3x3_to_2` generated `6350` raw candidates and
  survived pruning `1547` times; but
- both promoted endpoint families still recorded `0` exact meets on the
  retained lane.

So the retained lane itself still did **not** move.

## Merge-safe result

`just check-k3-graph-merge` still passed after the promotion.

That is the explicit merge-budget result to keep:

- the dim-3 gate avoided the previous failure mode of widening graph-only
  branching into the broader `max_intermediate_dim = 5` merge probe; and
- no narrowing/follow-up fix beyond the retained dim-3 gate was needed.

## Exact remaining blocker

The retained endpoint lifts are live, but the retained lane is still blocked by
the interior dim-3 bridge, not by missing endpoint coverage.

Smallest reproducible blocker:

- retained `k = 4` middle `3x3 -> 3x3` segment

```text
[[1,3,1],    [[4,4,4],
 [1,3,0], ->  [1,1,1],
 [2,6,4]]     [0,1,3]]
```

Direct bounded probes:

```bash
timeout -k 10s 60s target/release/search \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:4,4,4,1,1,1,0,1,3 \
  --max-lag 3 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-only \
  --json
# outcome: unknown

timeout -k 10s 60s target/release/search \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:4,4,4,1,1,1,0,1,3 \
  --max-lag 3 \
  --max-intermediate-dim 3 \
  --max-entry 5 \
  --move-policy graph-only \
  --json
# outcome: equivalent
```

The same threshold shows up on the full retained `k = 4` endpoint pair:

```bash
timeout -k 10s 60s target/release/search \
  4,2,1,4 \
  3,1,1,5 \
  --max-lag 5 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-only \
  --json
# outcome: unknown

timeout -k 10s 60s target/release/search \
  4,2,1,4 \
  3,1,1,5 \
  --max-lag 5 \
  --max-intermediate-dim 3 \
  --max-entry 5 \
  --move-policy graph-only \
  --json
# outcome: equivalent
```

So the durable reading to keep is:

- the retained endpoint-lift promotion is real and active on the retained
  graph-only lane;
- it does not solve any retained rung under the committed benchmark bounds;
- the smallest exact remaining blocker is the retained `max_entry = 4` cap on
  the interior dim-3 `k = 4` bridge, not the absence of the endpoint lifts; and
- the next follow-up, if any, should stay focused on that retained bounded
  interior dim-3 obstruction rather than widening graph-only policy further.
