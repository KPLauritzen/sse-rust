# Explicit graph-only decomposition for the retained Riedel `k = 4` witness (2026-04-18)

## Goal

Recover a first explicit graph-only decomposition on the Riedel ladder at the
smallest practical rung `k = 4`, using the retained
`graph_plus_structured` witness as the anchor and keeping the result as
sidecar/decomposition evidence rather than solver work.

## Kept anchor witness

Keep the previously retained `graph_plus_structured` `k = 4` witness from
[`2026-04-18-riedel-witness-classification-k4-k6.md`](./2026-04-18-riedel-witness-classification-k4-k6.md)
as the constructive anchor:

```text
W0 = [[4,2],     W1 = [[1,3,1],   W2 = [[1,2,1],
      [1,4]]           [1,3,0],         [1,3,0],
                       [2,6,4]]         [3,5,4]]

W3 = [[4,3,5],   W4 = [[4,4,4],   W5 = [[3,1],
      [1,1,2],         [1,1,1],         [1,5]]
      [0,1,3]]         [0,1,3]]
```

This is the kept five-step witness:

```text
W0 -> W1 -> W2 -> W3 -> W4 -> W5
```

## Step replacements

Each non-graph step in the kept witness now has an explicit bounded
graph-only expansion.

| Anchor step | Replacement artifact | Graph-only lag | Interpretation |
| --- | --- | --- | --- |
| `W0 -> W1` | [`research/riedel_k4_anchor_step0_graph_decomposition_2026-04-18.json`](../riedel_k4_anchor_step0_graph_decomposition_2026-04-18.json) | `6` | `outsplit -> outsplit -> outsplit -> permutation -> in_amalgamation -> in_amalgamation` |
| `W1 -> W2` | [`research/riedel_k4_retained_step_decomposition_2026-04-18.json`](../riedel_k4_retained_step_decomposition_2026-04-18.json) | `3` | `insplit -> permutation -> out_amalgamation` |
| `W2 -> W3` | [`research/riedel_k4_anchor_step2_graph_decomposition_2026-04-18.json`](../riedel_k4_anchor_step2_graph_decomposition_2026-04-18.json) | `1` | permutation relabeling |
| `W3 -> W4` | [`research/riedel_k4_anchor_step3_graph_decomposition_2026-04-18.json`](../riedel_k4_anchor_step3_graph_decomposition_2026-04-18.json) | `3` | `insplit -> permutation -> out_amalgamation` |
| `W4 -> W5` | [`research/riedel_k4_anchor_step4_graph_decomposition_2026-04-18.json`](../riedel_k4_anchor_step4_graph_decomposition_2026-04-18.json) | `6` | `outsplit -> in_amalgamation -> permutation -> insplit -> out_amalgamation -> in_amalgamation` |

So the kept anchor witness now expands to a full explicit graph-only
decomposition of total lag

```text
6 + 3 + 1 + 3 + 6 = 19.
```

No witness step from the kept anchor remains unresolved in this bounded slice.

## Full graph-only artifact

In addition to the anchored stepwise replacement above, a direct bounded
`graph_only` search now produces a full explicit `k = 4` witness:

- retained JSON run output:
  [`research/riedel_k4_graph_only_full_decomposition_2026-04-18.json`](../riedel_k4_graph_only_full_decomposition_2026-04-18.json)
- retained reusable guide artifact:
  [`research/riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json`](../riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json)

That direct witness is:

- `outcome = equivalent`
- witness lag `15`
- found under the bounded envelope `max_lag = 19`, `max_intermediate_dim = 5`,
  `max_entry = 12`

This note keeps the anchor-based `lag 19` expansion because it records exactly
which retained witness steps were replaced, but the direct lag-15 artifact is
the stronger full graph-only witness to reuse later.

I did **not** retain a minimality claim for the lag-15 witness: direct reruns
with `max_lag = 14` and `max_lag = 15` both timed out inside a `30 s` cap, so
this slice only establishes existence, not shortestness.

## Reproduce

Build the helper binaries:

```bash
cargo build --profile dist --features research-tools --bin explain_witness_step --bin search
```

Regenerate the anchor-step artifacts:

```bash
target/dist/explain_witness_step \
  --from 2x2:4,2,1,4 \
  --to 3x3:1,3,1,1,3,0,2,6,4 \
  --graph-max-lag 6 \
  --graph-max-intermediate-dim 5 \
  --graph-max-entry 12 \
  --factorisation-max-entry 12 \
  --write-json research/riedel_k4_anchor_step0_graph_decomposition_2026-04-18.json \
  > tmp/riedel_k4_anchor_step0_graph_decomposition_stdout_2026-04-18.json

target/dist/explain_witness_step \
  --from 3x3:1,2,1,1,3,0,3,5,4 \
  --to 3x3:4,3,5,1,1,2,0,1,3 \
  --graph-max-lag 2 \
  --graph-max-intermediate-dim 3 \
  --graph-max-entry 12 \
  --factorisation-max-entry 12 \
  --write-json research/riedel_k4_anchor_step2_graph_decomposition_2026-04-18.json \
  > tmp/riedel_k4_anchor_step2_graph_decomposition_stdout_2026-04-18.json

target/dist/explain_witness_step \
  --from 3x3:4,3,5,1,1,2,0,1,3 \
  --to 3x3:4,4,4,1,1,1,0,1,3 \
  --graph-max-lag 3 \
  --graph-max-intermediate-dim 4 \
  --graph-max-entry 12 \
  --factorisation-max-entry 12 \
  --write-json research/riedel_k4_anchor_step3_graph_decomposition_2026-04-18.json \
  > tmp/riedel_k4_anchor_step3_graph_decomposition_stdout_2026-04-18.json

target/dist/explain_witness_step \
  --from 3x3:4,4,4,1,1,1,0,1,3 \
  --to 2x2:3,1,1,5 \
  --graph-max-lag 6 \
  --graph-max-intermediate-dim 5 \
  --graph-max-entry 12 \
  --factorisation-max-entry 12 \
  --write-json research/riedel_k4_anchor_step4_graph_decomposition_2026-04-18.json \
  > tmp/riedel_k4_anchor_step4_graph_decomposition_stdout_2026-04-18.json
```

Regenerate the full direct graph-only witness:

```bash
timeout -k 5s 45s target/dist/search \
  4,2,1,4 \
  3,1,1,5 \
  --max-lag 19 \
  --max-intermediate-dim 5 \
  --max-entry 12 \
  --move-policy graph-only \
  --json \
  --write-guide-artifact research/riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json \
  > research/riedel_k4_graph_only_full_decomposition_2026-04-18.json
```

Focused validation used for this slice:

```bash
cargo test --features research-tools --bin explain_witness_step
```
