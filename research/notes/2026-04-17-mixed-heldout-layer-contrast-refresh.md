# Mixed held-out layer-contrast artifact refresh (2026-04-17)

## Question

Refresh the durable mixed endpoint layer-contrast artifact so it covers all
three non-Brix held-out families under the existing analyzer, witness-manifest,
and family-benchmark surfaces, then record whether the result is
benchmark-meaningful under
`research/ranking_signal_family_benchmark_v1.json`.

## Build and exact commands

Rebuilt only the analyzer binary needed for the refresh:

```bash
cargo build --quiet --features research-tools --bin analyze_path_signal_corpus
```

Refreshed the durable mixed held-out endpoint artifact directly from
`research/cases.json`. No temporary one-case corpus was needed here because the
seven held-out endpoint cases already use the real mixed endpoint policy by
default.

```bash
timeout -k 10s 60s target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id riedel_baker_k4 \
  --case-id riedel_baker_k6 \
  --case-id riedel_baker_k8 \
  --case-id riedel_baker_k10 \
  --case-id riedel_baker_k12 \
  --case-id lind_marcus_a_to_c \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts research/layer_contrast_signal_corpus_non_brix_mixed_heldout_2026-04-17.json \
  > tmp/layer_contrast_signal_corpus_non_brix_mixed_heldout_2026-04-17.stdout.txt
```

## Durable outputs

- Artifact path:
  `research/layer_contrast_signal_corpus_non_brix_mixed_heldout_2026-04-17.json`
- Companion stdout log:
  `tmp/layer_contrast_signal_corpus_non_brix_mixed_heldout_2026-04-17.stdout.txt`

## Coverage

Artifact summary:

- held-out families present: `3 / 3`
  - `riedel_baker`
  - `lind_marcus`
  - `higher_block`
- held-out families rankable: `3 / 3`
- rankable held-out pairs: `7`
- ranked held-out observations: `45`
- unranked solved held-out pairs: `0`

Per-family rankable coverage:

- `riedel_baker`
  - held-out pairs present: `5`
  - held-out pairs rankable: `5`
  - rankable held-out observations: `40`
- `lind_marcus`
  - held-out pairs present: `1`
  - held-out pairs rankable: `1`
  - rankable held-out observations: `2`
- `higher_block`
  - held-out pairs present: `1`
  - held-out pairs rankable: `1`
  - rankable held-out observations: `3`

Case-level rankable layers in the refreshed artifact:

- `riedel_baker_k4`: `4`
- `riedel_baker_k6`: `6`
- `riedel_baker_k8`: `8`
- `riedel_baker_k10`: `10`
- `riedel_baker_k12`: `12`
- `lind_marcus_a_to_c`: `2`
- `full_2_shift_higher_block_1x1_to_4x4`: `3`

## Benchmark meaning

Under `research/ranking_signal_family_benchmark_v1.json`, this refreshed mixed
held-out endpoint artifact is **benchmark-meaningful**.

It clears every minimum coverage gate:

- all three held-out families are present;
- at least two held-out families are rankable;
- at least four held-out pairs are rankable;
- at least twenty held-out observations are rankable.

Measured totals:

- `heldout_families_present = 3`
- `heldout_families_rankable = 3`
- `rankable_heldout_pairs = 7`
- `ranked_heldout_observations = 45`

## Follow-up

No new bead was filed from this refresh. The concrete remaining tooling
follow-up is already tracked by `sse-rust-eiu`, which covers analyzer-side
endpoint move-family overrides for future policy sweeps without temporary
corpus rewriting.
