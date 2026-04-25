# Same-future/past frontier diversity metric (2026-04-25)

## Question

For bead `sse-rust-ywq`, define and test one frontier diversity metric for
`graph_plus_structured` beam search that goes beyond exact canonical matrix
equality, without changing default search semantics.

## Metric

The diversity bucket is the existing same-future/past signature from
`src/graph_moves.rs`.

For a square matrix, the bucket contains:

- dimension;
- total entry sum;
- row classes, where equal row vectors are grouped and each class records
  multiplicity, row entry sum, and row support size; and
- column classes with the analogous multiplicity, entry sum, and support size.

This is coarser than canonical matrix equality and catches retained-frontier
near-duplicates with the same repeated future/past row/column structure even
when the active block layout differs.

## Implementation

Added opt-in frontier mode:

`frontier_mode = "same_future_past_diversity_beam"`

The retained implementation is report-only: it uses the normal beam scorer,
beam comparator, beam width, and expansion semantics, then records same-future
/past bucket saturation telemetry for the retained active frontiers. It does
not prune or reorder states.

An earlier admission variant that preferred unsaturated buckets before the beam
score was rejected during the slice: the retained `k=4` lane timed out at the
same 30s budget and still timed out with a 70s diagnostic timeout before the
harness could return telemetry. The committed mode therefore keeps the metric as
opt-in telemetry only.

Telemetry is emitted under `telemetry.same_future_past_diversity`:

- `max_frontier_nodes`;
- `max_unique_buckets`;
- `max_saturated_buckets`, counted within each side's active frontier and then
  summed across sides;
- `max_bucket_size`;
- `max_cross_frontier_overlap_buckets`, counted separately from saturation;
- terminal `final_*` bucket counts; and
- admission/replacement fields, currently zero for the report-only mode.

## Commands

Build:

```bash
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
```

A/B corpus:

```bash
jq -n --slurpfile base research/cases.json \
  --slurpfile k4 research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  '{schema_version:5, cases: [
    ($base[0].cases[] | select(.id == "brix_ruiz_k3_graph_plus_structured_beam_probe")
      | .id="brix_ruiz_k3_graph_plus_structured_beam_probe__baseline"
      | del(.measurement) | .timeout_ms=6000),
    ($base[0].cases[] | select(.id == "brix_ruiz_k3_graph_plus_structured_beam_probe")
      | .id="brix_ruiz_k3_graph_plus_structured_beam_probe__same_future_past_diversity"
      | .description="A/B same-future/past diversity beam variant for sse-rust-ywq."
      | del(.measurement) | .timeout_ms=6000
      | .config.frontier_mode="same_future_past_diversity_beam"),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__baseline"
      | .timeout_ms=30000),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__same_future_past_diversity"
      | .description="A/B same-future/past diversity beam variant for sse-rust-ywq."
      | .timeout_ms=30000
      | .config.frontier_mode="same_future_past_diversity_beam")
  ]}' > tmp/ywq_same_future_past_diversity_ab_cases.json

timeout -k 20s 150s target/debug/research_harness \
  --cases tmp/ywq_same_future_past_diversity_ab_cases.json \
  --format json \
  > tmp/ywq_same_future_past_diversity_ab_results.json
```

Metric extraction:

```bash
jq -r '(["id","outcome","steps","elapsed_ms","exact_meets","approx_hits",
  "max_frontier","total_visited","expanded","factorisations","kept_candidates",
  "discovered","max_div_nodes","max_buckets","max_sat_buckets","max_bucket_size",
  "max_cross_overlap"] | @tsv),
  (.cases[] | [.id,.actual_outcome,(.steps//""),.elapsed_ms,
  .telemetry.collisions_with_other_frontier,
  .telemetry.approximate_other_side_hits,
  .telemetry.max_frontier_size,
  .telemetry.total_visited_nodes,
  .telemetry.frontier_nodes_expanded,
  .telemetry.factorisations_enumerated,
  .telemetry.candidates_after_pruning,
  .telemetry.discovered_nodes,
  (.telemetry.same_future_past_diversity.max_frontier_nodes//0),
  (.telemetry.same_future_past_diversity.max_unique_buckets//0),
  (.telemetry.same_future_past_diversity.max_saturated_buckets//0),
  (.telemetry.same_future_past_diversity.max_bucket_size//0),
  (.telemetry.same_future_past_diversity.max_cross_frontier_overlap_buckets//0)] | @tsv)' \
  tmp/ywq_same_future_past_diversity_ab_results.json
```

## A/B Metrics

### `k=3` control

Case: `brix_ruiz_k3_graph_plus_structured_beam_probe`, `beam_width=10`.

| Mode | Outcome | Witness lag | Time | Exact meets | Approx. hits | Max frontier | Total visited | Expanded | Factorisations | Kept candidates | Discovered | Max diversity nodes | Max buckets | Saturated buckets | Max bucket | Cross overlap |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `beam` | `unknown` | none | `523 ms` | `0` | `10` | `10` | `2631` | `142` | `21653` | `3501` | `2629` | `0` | `0` | `0` | `0` | `0` |
| `same_future_past_diversity_beam` | `unknown` | none | `491 ms` | `0` | `10` | `10` | `2631` | `142` | `21653` | `3501` | `2629` | `20` | `20` | `0` | `1` | `0` |

The control retained frontiers had no same-future/past bucket saturation.

### Retained Brix-Ruiz `k=4` lane

Case: `brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`.

| Mode | Outcome | Witness lag | Time | Exact meets | Approx. hits | Max frontier | Total visited | Expanded | Factorisations | Kept candidates | Discovered | Max diversity nodes | Max buckets | Saturated buckets | Max bucket | Cross overlap |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `beam` | `unknown` | none | `23894 ms` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` | `0` | `0` | `0` | `0` | `0` |
| `same_future_past_diversity_beam` | `unknown` | none | `24621 ms` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` | `512` | `509` | `85` | `6` | `0` |

The retained lane did not change outcome, witness lag, exact meets,
approximate hits, or work counts. The metric reports real but modest retained
frontier saturation: at peak, 512 active frontier nodes across both sides occupy
509 same-future/past buckets; 85 same-side buckets were saturated at least on
one observed frontier summary, the largest observed same-side bucket held 6
active nodes, and no cross-frontier bucket overlap was observed at the sampled
peaks.

## Decision

Keep the metric as opt-in report-only telemetry. Do not promote an admission or
ranking variant from this slice.

The retained `k=4` lane does contain same-future/past bucket saturation, but the
observed saturation is not large enough to justify hard pruning, and the
attempted aggressive admission ordering was too expensive / disruptive for the
same-budget retained comparison.

## Follow-up

A concrete follow-up bead is justified only if it stays report-first:

> Compare per-layer same-future/past bucket saturation against approximate-hit
> layers and family-local sources, then test a much narrower admission variant
> only for approximate-hit buckets or late-depth saturated buckets.

Do not reopen broad active-block switches or add a move family from this result.

## Validation

```bash
timeout -k 20s 180s cargo test --features research-tools same_future_past_diversity
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
timeout -k 20s 150s target/debug/research_harness \
  --cases tmp/ywq_same_future_past_diversity_ab_cases.json \
  --format json \
  > tmp/ywq_same_future_past_diversity_ab_results.json
```

`cargo bench --bench search -- --noplot` was not run for this slice because the
committed mode is opt-in report-only telemetry and the default beam hot path is
not changed.
