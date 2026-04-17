# Endpoint move-family override in `analyze_path_signal_corpus` (2026-04-17)

## Question

Can `analyze_path_signal_corpus` honor an explicit analyzer-side move-family
override for endpoint cases loaded from `research/cases.json`, so endpoint
policy sweeps no longer need one-case temporary corpora in `tmp/`?

## Exact seam

Before this change, the analyzer had two different move-family paths:

- derived path-segment cases in `derive_path_cases()` always used the CLI
  `--search-mode`;
- endpoint cases in `load_research_cases()` always kept
  `case.config.move_family_policy`.

That meant `--search-mode graph-only` changed metadata and derived-case search
behavior, but did not actually rerun endpoint cases under `graph_only`.

## Decision

Keep `--search-mode` as the explicit override surface.

Implementation contract:

- derived path-segment cases still default to CLI `mixed`;
- endpoint cases keep `research/cases.json` policy by default;
- if `--search-mode` is supplied explicitly, that override now applies to both
  derived and endpoint cases.

The analyzer stdout now also prints the effective per-case
`move_family_policy`, so bounded validation can prove which endpoint policy was
actually executed.

## Validation

Build and targeted tests:

```bash
cargo test -q --features research-tools --bin analyze_path_signal_corpus
cargo build --quiet --features research-tools --bin analyze_path_signal_corpus
```

Bounded endpoint sweeps directly from `research/cases.json`:

```bash
timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id lind_marcus_a_to_c \
  --search-mode mixed \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/lind_marcus_a_to_c_mixed_endpoint_override.json \
  > tmp/lind_marcus_a_to_c_mixed_endpoint_override.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id lind_marcus_a_to_c \
  --search-mode graph-only \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/lind_marcus_a_to_c_graph_only_endpoint_override.json \
  > tmp/lind_marcus_a_to_c_graph_only_endpoint_override.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id lind_marcus_a_to_c \
  --search-mode graph_plus_structured \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/lind_marcus_a_to_c_graph_plus_structured_endpoint_override.json \
  > tmp/lind_marcus_a_to_c_graph_plus_structured_endpoint_override.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --max-endpoint-dim 4 \
  --search-mode mixed \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/full_2_shift_higher_block_1x1_to_4x4_mixed_endpoint_override.json \
  > tmp/full_2_shift_higher_block_1x1_to_4x4_mixed_endpoint_override.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --max-endpoint-dim 4 \
  --search-mode graph-only \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/full_2_shift_higher_block_1x1_to_4x4_graph_only_endpoint_override.json \
  > tmp/full_2_shift_higher_block_1x1_to_4x4_graph_only_endpoint_override.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --max-endpoint-dim 4 \
  --search-mode graph_plus_structured \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/full_2_shift_higher_block_1x1_to_4x4_graph_plus_structured_endpoint_override.json \
  > tmp/full_2_shift_higher_block_1x1_to_4x4_graph_plus_structured_endpoint_override.stdout.txt
```

## Results

Effective endpoint policies reported in stdout:

- `lind_marcus_a_to_c`
  - `move_family_policy=Mixed`
  - `move_family_policy=GraphOnly`
  - `move_family_policy=GraphPlusStructured`
- `full_2_shift_higher_block_1x1_to_4x4`
  - `move_family_policy=Mixed`
  - `move_family_policy=GraphOnly`
  - `move_family_policy=GraphPlusStructured`

Case behavior:

- `lind_marcus_a_to_c`
  - `mixed`: solved lag `2`, ranked `1/1`, layers `2`
  - `graph_only`: solved lag `2`, ranked `0/1`, layers `0`
  - `graph_plus_structured`: solved lag `2`, ranked `1/1`, layers `2`
- `full_2_shift_higher_block_1x1_to_4x4`
  - `mixed`: solved lag `4`, ranked `2/2`, layers `3`
  - `graph_only`: solved lag `3`, ranked `0/2`, layers `0`
  - `graph_plus_structured`: solved lag `4`, ranked `2/2`, layers `3`

This is enough to show that endpoint sweeps now execute the requested policy
directly through the analyzer, without rewriting one-case corpora in `tmp/`.

## Follow-up

No new bead was filed from this fix. The bounded inconsistency in scope was the
endpoint-case override seam, and this change closes that gap without changing
the witness-manifest or ranking-label contract.
