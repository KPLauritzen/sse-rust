# Same-future/past diversity layer correlation (2026-04-25)

## Question

For bead `sse-rust-pucb`, check whether retained `graph_plus_structured`
beam lanes show useful alignment between per-layer same-future/past diversity
saturation and approximate-hit layers / family-local source signals.

This follows the opt-in telemetry from `sse-rust-ywq`. The goal was
report-first; no default `beam` or `graph_plus_structured` behavior was
changed.

## Telemetry addition

The previous telemetry had peak and final same-future/past diversity counters,
but not enough layer alignment detail. I added a compact opt-in field under
`telemetry.same_future_past_diversity.layer_samples` for
`same_future_past_diversity_beam` runs.

Each sample records:

- `layer_index`;
- expansion `direction`;
- retained active `frontier_nodes` across both sides after the layer;
- `unique_buckets`;
- same-side `saturated_buckets`;
- `max_bucket_size`; and
- `cross_frontier_overlap_buckets`.

The existing `telemetry.layers[]` entries already provide approximate hits,
frontier size, visited count, elapsed layer timing, and per-move-family source
counts. Joining by `layer_index` gives the requested comparison without dumping
node-level data.

## Corpus

Scratch inputs/results:

- `tmp/pucb_same_future_past_diversity_layer_ab_cases.json`
- `tmp/pucb_same_future_past_diversity_layer_ab_results.json`
- `tmp/pucb_same_future_past_diversity_layer_summary.json`

Cases:

| Lane | Mode | Timeout |
| --- | --- | ---: |
| `brix_ruiz_k3_graph_plus_structured_beam_probe` | `beam` | 6s |
| `brix_ruiz_k3_graph_plus_structured_beam_probe` | `same_future_past_diversity_beam` | 6s |
| `brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12` | `beam` | 30s |
| `brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12` | `same_future_past_diversity_beam` | 30s |

## A/B totals

| Lane | Mode | Outcome | Time | Layers | Approx hits | Max frontier | Visited | Expanded | Kept | Max diversity nodes | Max buckets | Max saturated | Max bucket | Cross overlap |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| k3 probe | `beam` | `unknown` | 490 ms | 16 | 10 | 10 | 2,631 | 142 | 3,501 | 0 | 0 | 0 | 0 | 0 |
| k3 probe | `same_future_past_diversity_beam` | `unknown` | 494 ms | 16 | 10 | 10 | 2,631 | 142 | 3,501 | 20 | 20 | 0 | 1 | 0 |
| k4 retained | `beam` | `unknown` | 23,548 ms | 80 | 184 | 256 | 176,664 | 19,970 | 271,803 | 0 | 0 | 0 | 0 | 0 |
| k4 retained | `same_future_past_diversity_beam` | `unknown` | 24,041 ms | 80 | 184 | 256 | 176,664 | 19,970 | 271,803 | 512 | 509 | 85 | 6 | 0 |

The report-only diversity mode preserved the baseline work counts and
approximate-hit counts. For k4, per-layer `frontier_nodes` and
`approximate_other_side_hits` matched the baseline at every layer.

## Layer correlation

### k3 probe

The k3 control lane has no retained same-future/past saturation at all:

- saturated layers: `0 / 16`;
- approximate-hit layers: `6 / 16`;
- total approximate hits: `10`;
- all approximate hits occurred with `saturated_buckets = 0`.

Approximate-hit source signal:

| Family | Discovered | Approx hits |
| --- | ---: | ---: |
| `elementary_conjugation` | 323 | 5 |
| `rectangular_factorisation_2x3` | 653 | 4 |
| `binary_sparse_rectangular_factorisation_3x3_to_4` | 349 | 1 |

This is a direct negative control: approximate hits do not require saturation
in the retained beam frontier.

### k4 retained lane

The k4 retained lane has saturation on every sampled layer, but the strongest
saturation does not line up with approximate hits:

- layer samples: `80`;
- total approximate hits: `184`;
- sum of saturated buckets across samples: `1,723`;
- saturation-vs-approximate-hit Pearson check: `-0.164`;
- `58 / 80` layers had at least one approximate hit;
- the `22` no-approx layers still held `657` saturated buckets;
- no cross-frontier same-future/past bucket overlap was observed.

Most saturated sampled layers:

| Layer | Direction | Approx hits | Saturated buckets | Max bucket |
| ---: | --- | ---: | ---: | ---: |
| 3 | backward | 0 | 85 | 2 |
| 4 | backward | 0 | 84 | 2 |
| 0 | forward | 0 | 81 | 2 |
| 1 | backward | 0 | 81 | 2 |
| 2 | backward | 0 | 81 | 2 |
| 49 | forward | 3 | 48 | 4 |
| 45 | forward | 4 | 38 | 3 |
| 44 | forward | 2 | 36 | 4 |
| 47 | forward | 5 | 36 | 6 |
| 43 | forward | 1 | 35 | 3 |
| 13 | backward | 6 | 34 | 4 |
| 51 | forward | 1 | 33 | 3 |

Most approximate-hit-heavy layers:

| Layer | Direction | Approx hits | Saturated buckets | Max bucket |
| ---: | --- | ---: | ---: | ---: |
| 5 | forward | 29 | 3 | 2 |
| 7 | forward | 13 | 10 | 2 |
| 46 | forward | 10 | 29 | 3 |
| 40 | forward | 7 | 23 | 2 |
| 9 | forward | 6 | 4 | 2 |
| 13 | backward | 6 | 34 | 4 |
| 27 | backward | 5 | 18 | 2 |
| 47 | forward | 5 | 36 | 6 |
| 19 | backward | 4 | 15 | 2 |
| 23 | backward | 4 | 12 | 2 |
| 34 | backward | 4 | 11 | 2 |
| 45 | forward | 4 | 38 | 3 |

Approximate-hit source signal:

| Family | Discovered | Approx hits |
| --- | ---: | ---: |
| `elementary_conjugation` | 28,904 | 90 |
| `diagonal_refactorization_4x4` | 35,778 | 37 |
| `insplit` | 47,023 | 28 |
| `outsplit` | 45,716 | 27 |
| `binary_sparse_rectangular_factorisation_3x3_to_4` | 1,930 | 2 |

The strongest family-local approximate-hit sources are ordinary retained graph
or 4x4 refactorization families, not a clear signature of high
same-future/past bucket saturation.

## Decision

Do not run a narrower admission/ranking experiment from this evidence.

The correlation is negative/inconclusive for the retained k4 lane and absent
for the k3 control. The early k4 saturated layers are especially poor
admission targets: layers `0..4` have `81..85` saturated buckets but `0`
approximate hits. The highest approximate-hit layer, layer `5`, has only `3`
saturated buckets.

Keep `same_future_past_diversity_beam` as opt-in report-only telemetry. Do not
promote it to a default mode. No follow-up bead is justified from this slice.

## Commands

Build and tests:

```bash
cargo fmt
timeout -k 20s 180s cargo test --features research-tools same_future_past_diversity
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
      | del(.measurement) | .timeout_ms=6000
      | .config.frontier_mode="same_future_past_diversity_beam"),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__baseline"
      | .timeout_ms=30000),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__same_future_past_diversity"
      | .timeout_ms=30000
      | .config.frontier_mode="same_future_past_diversity_beam")
  ]}' > tmp/pucb_same_future_past_diversity_layer_ab_cases.json

timeout -k 20s 180s target/debug/research_harness \
  --cases tmp/pucb_same_future_past_diversity_layer_ab_cases.json \
  --format json \
  > tmp/pucb_same_future_past_diversity_layer_ab_results.json
```

`cargo bench --bench search -- --noplot` was not run because this change adds
serialization-visible samples only to an opt-in report-only frontier mode. The
default `beam` and `graph_plus_structured` hot path semantics are unchanged.
