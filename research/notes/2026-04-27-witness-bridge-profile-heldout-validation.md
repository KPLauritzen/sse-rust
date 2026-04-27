# Witness bridge profile held-out validation (2026-04-27)

## Question

Does the frozen opt-in `witness_bridge_profile_beam` mode from
`sse-rust-ejj` replicate on held-out `graph_plus_structured` controls without
retuning?

This is bead `sse-rust-tk0l`.

## Source result being checked

`sse-rust-ejj` kept the mode opt-in only. On the retained open Brix-Ruiz `k = 4`
graph-plus-structured lane, it improved approximate other-side hits from `184`
to `198` and lowered visited nodes and elapsed time, but it did not find exact
hits. The `k = 3` control was mixed, so the rule was not promoted to any
default behavior.

This held-out slice does not retune the rule.

## Held-out cases

Required cases:

- `riedel_baker_k4__graph_plus_structured__benchmark_lane`
- `brix_ruiz_k4__graph_plus_structured__beam128_lag40_dim4_entry12`

Optional cheap rung:

- `riedel_baker_k6__graph_plus_structured__benchmark_lane`

The Riedel/Baker retained lanes are not beam cases in the source corpus. For
the A/B comparison, both variants were converted to a single fixed
`frontier_mode`/beam envelope:

```text
frontier_mode = beam or witness_bridge_profile_beam
beam_width = 256
```

This was a fixed conversion choice, not a width sweep. The Brix-Ruiz held-out
case kept its source envelope unchanged except for the witness variant's
`frontier_mode`:

```text
max_lag = 40
max_intermediate_dim = 4
max_entry = 12
beam_width = 128
move_family_policy = graph_plus_structured
```

## Commands

Build:

```bash
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
```

Required held-out corpus:

```bash
jq -n \
  --slurpfile riedel research/riedel_gap_benchmark_lane_2026-04-18.json \
  --slurpfile k4 research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  '{schema_version:5, cases: [
    ($riedel[0].cases[] | select(.id == "riedel_baker_k4__graph_plus_structured__benchmark_lane")
      | .id="riedel_baker_k4__graph_plus_structured__benchmark_lane__beam_baseline"
      | .description="Held-out baseline beam variant for sse-rust-tk0l."
      | .config.frontier_mode="beam"
      | .config.beam_width=256
      | .timeout_ms=8000),
    ($riedel[0].cases[] | select(.id == "riedel_baker_k4__graph_plus_structured__benchmark_lane")
      | .id="riedel_baker_k4__graph_plus_structured__benchmark_lane__witness_bridge_profile"
      | .description="Held-out witness-bridge profile beam variant for sse-rust-tk0l."
      | .config.frontier_mode="witness_bridge_profile_beam"
      | .config.beam_width=256
      | .timeout_ms=8000),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam128_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam128_lag40_dim4_entry12__beam_baseline"
      | .description="Held-out open Brix-Ruiz k4 baseline beam variant for sse-rust-tk0l."
      | .timeout_ms=30000),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam128_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam128_lag40_dim4_entry12__witness_bridge_profile"
      | .description="Held-out open Brix-Ruiz k4 witness-bridge profile beam variant for sse-rust-tk0l."
      | .config.frontier_mode="witness_bridge_profile_beam"
      | .timeout_ms=30000)
  ]}' > tmp/tk0l_witness_bridge_profile_heldout_cases.json
```

Required held-out run:

```bash
timeout -k 20s 120s target/debug/research_harness \
  --cases tmp/tk0l_witness_bridge_profile_heldout_cases.json \
  --format json \
  > tmp/tk0l_witness_bridge_profile_heldout_results.json
```

Optional `k = 6` Riedel/Baker rung:

```bash
jq -n \
  --slurpfile riedel research/riedel_gap_benchmark_lane_2026-04-18.json \
  '{schema_version:5, cases: [
    ($riedel[0].cases[] | select(.id == "riedel_baker_k6__graph_plus_structured__benchmark_lane")
      | .id="riedel_baker_k6__graph_plus_structured__benchmark_lane__beam_baseline"
      | .description="Optional held-out baseline beam variant for sse-rust-tk0l."
      | .config.frontier_mode="beam"
      | .config.beam_width=256
      | .timeout_ms=8000),
    ($riedel[0].cases[] | select(.id == "riedel_baker_k6__graph_plus_structured__benchmark_lane")
      | .id="riedel_baker_k6__graph_plus_structured__benchmark_lane__witness_bridge_profile"
      | .description="Optional held-out witness-bridge profile beam variant for sse-rust-tk0l."
      | .config.frontier_mode="witness_bridge_profile_beam"
      | .config.beam_width=256
      | .timeout_ms=8000)
  ]}' > tmp/tk0l_witness_bridge_profile_heldout_k6_cases.json

timeout -k 20s 30s target/debug/research_harness \
  --cases tmp/tk0l_witness_bridge_profile_heldout_k6_cases.json \
  --format json \
  > tmp/tk0l_witness_bridge_profile_heldout_k6_results.json
```

Metric extraction:

```bash
jq -r '.cases[] |
  [.id,.actual_outcome,(.steps//""),.elapsed_ms,
   (.telemetry.collisions_with_other_frontier // ""),
   (.telemetry.approximate_other_side_hits // ""),
   (.telemetry.max_frontier_size // ""),
   (.telemetry.total_visited_nodes // ""),
   (.telemetry.frontier_nodes_expanded // ""),
   (.telemetry.factorisations_enumerated // ""),
   (.telemetry.candidates_after_pruning // ""),
   (.telemetry.discovered_nodes // ""),
   (.telemetry.enqueued_nodes // "")] | @tsv' \
  tmp/tk0l_witness_bridge_profile_heldout_results.json
```

## Artifacts

- `tmp/tk0l_witness_bridge_profile_heldout_cases.json`
- `tmp/tk0l_witness_bridge_profile_heldout_results.json`
- `tmp/tk0l_witness_bridge_profile_heldout_k6_cases.json`
- `tmp/tk0l_witness_bridge_profile_heldout_k6_results.json`

## Metrics

| Case | Mode | Outcome | Steps | Elapsed ms | Exact hits | Approx hits | Max frontier | Visited | Expanded | Factorisations | Kept candidates | Discovered | Enqueued |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| Brix-Ruiz k4 held-out beam128 | `beam` | `unknown` | | `12937` | `0` | `95` | `128` | `97548` | `9986` | `272751` | `144897` | `97546` | `97546` |
| Brix-Ruiz k4 held-out beam128 | `witness_bridge_profile_beam` | `unknown` | | `12565` | `0` | `118` | `128` | `92405` | `9986` | `262229` | `139229` | `92403` | `92403` |
| Riedel/Baker k4 beam256 | `beam` | `equivalent` | `5` | `48` | `1` | `2` | `199` | `328` | `243` | `3275` | `514` | `327` | `326` |
| Riedel/Baker k4 beam256 | `witness_bridge_profile_beam` | `equivalent` | `5` | `36` | `1` | `2` | `199` | `321` | `243` | `3275` | `514` | `320` | `319` |
| Riedel/Baker k6 beam256 | `beam` | `equivalent` | `7` | `227` | `1` | `2` | `256` | `2370` | `687` | `8552` | `3042` | `2369` | `2368` |
| Riedel/Baker k6 beam256 | `witness_bridge_profile_beam` | `equivalent` | `7` | `257` | `1` | `2` | `256` | `2370` | `687` | `8552` | `3042` | `2369` | `2368` |

## Reading

Held-out Brix-Ruiz `k = 4` directionally replicates the retained `sse-rust-ejj`
result:

- retained/source open k4: approximate hits improved `184 -> 198`, visited/time
  dropped, no exact hit;
- held-out open k4: approximate hits improved `95 -> 118`, visited/time dropped,
  factorisations and kept candidates also dropped, no exact hit.

The Riedel/Baker controls did not show a behavioral regression:

- k4 still solved in `5` steps with the same exact and approximate hit counts;
- k6 still solved in `7` steps with identical search counters, with only
  elapsed-time noise.

## Decision

Keep `witness_bridge_profile_beam` as an opt-in research mode. Do not promote it
to default `beam`, `bfs`, or `graph_plus_structured` behavior.

The held-out open-k4 result is a useful directional replication of the
retained-k4 signal, and the Riedel/Baker controls are neutral. However, this is
still not an exact witness and the control surface is too small to justify
default behavior changes or new tuning. No follow-up bead is opened from this
slice because the result does not point to a single concrete bounded next
experiment beyond broader validation.

## Validation

```bash
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
timeout -k 20s 120s target/debug/research_harness \
  --cases tmp/tk0l_witness_bridge_profile_heldout_cases.json \
  --format json \
  > tmp/tk0l_witness_bridge_profile_heldout_results.json
timeout -k 20s 30s target/debug/research_harness \
  --cases tmp/tk0l_witness_bridge_profile_heldout_k6_cases.json \
  --format json \
  > tmp/tk0l_witness_bridge_profile_heldout_k6_results.json
```

No focused tests were run because no code changed.
