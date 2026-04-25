# Brix-Ruiz `k=4` active-block switch cluster sweep reject (2026-04-25)

## Question

For bead `sse-rust-nw7.9`, sweep the research-only active-block `2x2`
contingency switch diagnostic over the retained Brix-Ruiz `k=4`
rank-4/rank-6 `diagonal_refactorization_4x4` hotspot cluster from the nw7.6
stuck-state artifact.

This remains a proposal diagnostic only. It does not change default solver
behavior, does not emit solver successors, does not add a selected family, does
not enumerate generic `4x4` factorisations, and does not reopen weighted
`4x4 -> 3` or generic `4x4 -> 3` gates.

## Inputs

Case:

`brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`

Fresh local retained stuck-state artifact for this bead:

`tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_9.json`

Cluster sweep output:

`tmp/brix_ruiz_k4_active_block_switch_cluster_rank4_rank6_2026-04-25_nw7_9.json`

The extractor replay used the same retained case and reported the same high
level telemetry as the nw7.6/nw7.7 slices: outcome `unknown`, `19970`
frontier nodes expanded, `487699` factorisations enumerated, `271803`
candidates after pruning, `176662` discovered nodes, `184` approximate hits,
and `176664` total visited nodes.

## Selection Rule

Select only retained approximate-hit pairs whose extractor rank is in `{4, 6}`
and whose move family is `diagonal_refactorization_4x4`.

Accept sparse active blocks shaped either `2x4` or `4x2`. Rank 4 is the original
`2x4` active-row shape; rank 6 is the transposed `4x2` active-column shape.
For each retained pair, enumerate every nonzero nonnegative `2x2` switch inside
the active rows/columns with `max_delta = 12`, then count only signature
preserving switches as accepted by the diagnostic. Distances are exact canonical
L1 distances to the recorded counterpart state.

## Results

No selected pair was skipped.

| Rank | Shape | Base L1 | Best L1 | Nonnegative switches | Signature preserving | Signature changing | Improving signature-preserving switches | Best improvement | Median improvement | Exact canonical matches |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `4` | `2x4` | `22` | `20` | `18` | `10` | `8` | `2` | `2` | `2.0` | `0` |
| `6` | `4x2` | `14` | `12` | `16` | `7` | `9` | `1` | `2` | `2.0` | `0` |

Aggregate:

| Field | Value |
| --- | ---: |
| retained pairs considered | `2` |
| skipped pairs | `0` |
| total nonnegative switches | `34` |
| signature-preserving switches | `17` |
| signature-changing switches | `17` |
| exact canonical L1 improvements | `3` |
| best improvement magnitude | `2` |
| median improvement magnitude | `2.0` |
| exact canonical matches | `0` |

Best improving rank-6 proposal:

```text
row_pair=[0,1], col_pair=[1,2], add_main_diagonal, delta=1
candidate =
[[0,3,2,0],
 [0,1,2,0],
 [0,11,0,0],
 [0,2,2,0]]
canonical L1: 12
```

## Reading

The signal repeats, but weakly. Both retained diagonal hotspot pairs have at
least one signature-preserving active-block switch that improves exact
canonical L1 by `2`, so the rank-4 result was not a one-off artifact of that
single layout. However, the improvement magnitude stays small, only `3` of
`34` total switches improve, and no exact canonical counterpart appears.

Recommendation: reject opening a validity-proof bead from this slice. Keep the
diagnostic as evidence that a local row/column-sum-preserving redistribution
can point in the right direction, but do not promote it to a solver family or
spend a proof bead without a stronger exact-match or larger-distance signal.

No follow-up bead was opened.

## Commands Run

```bash
timeout -k 20s 180s cargo build --features research-tools --bin extract_brix_ruiz_k4_stuck_states --bin diagnose_brix_ruiz_k4_active_block_switches

timeout -k 20s 120s target/debug/extract_brix_ruiz_k4_stuck_states \
  --json-out tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_9.json \
  --top 220

timeout -k 20s 180s cargo build --features research-tools --bin diagnose_brix_ruiz_k4_active_block_switches

timeout -k 20s 60s target/debug/diagnose_brix_ruiz_k4_active_block_switches \
  --input tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_9.json \
  --sweep-retained-diagonal-hotspot-cluster \
  --json-out tmp/brix_ruiz_k4_active_block_switch_cluster_rank4_rank6_2026-04-25_nw7_9.json
```

Validation:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo build --features research-tools --bin diagnose_brix_ruiz_k4_active_block_switches
timeout -k 20s 180s cargo test --features research-tools --bin diagnose_brix_ruiz_k4_active_block_switches
timeout -k 20s 180s cargo build --features research-tools --bin extract_brix_ruiz_k4_stuck_states
```

All validation commands passed, and the final sweep command wrote the cluster
JSON listed above.
