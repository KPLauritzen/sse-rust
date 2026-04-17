# Non-Brix layer-contrast rerun after mixed observer fix (2026-04-17)

## Question

After the dynamic mixed observer layer-event fix, do the bounded non-Brix
endpoint cases now contribute useful rankable layer-contrast evidence under the
current observer and ranking-signal tooling?

Bounded cases in scope:

- `lind_marcus_a_to_c`
- `full_2_shift_higher_block_1x1_to_4x4`

Policies compared:

- `mixed`
- `graph_only`
- `graph_plus_structured`

This stays inside the existing observer, ranking-signal, and layer-contrast
tooling. No new ranking pipeline or sqlite store was added.

## Rebuild and validation

Rebuilt only the targeted test and the analyzer binary:

```bash
cargo test -q test_dyn_mixed_search_observer_emits_layers_for_lind_marcus_case
cargo build --quiet --features research-tools --bin analyze_path_signal_corpus
```

The targeted regression test passed.

## Tooling caveat

`analyze_path_signal_corpus --search-mode ...` only overrides the move-family
policy for **derived path-segment cases**. Endpoint cases loaded from
`research/cases.json` keep `case.config.move_family_policy`.

So a direct command like:

```bash
target/debug/analyze_path_signal_corpus \
  --cases research/cases.json \
  --case-id lind_marcus_a_to_c \
  --search-mode graph-only \
  ...
```

changes the artifact metadata but does **not** actually rerun that endpoint
case under `graph_only`.

For this rerun, the real policy comparison used one-case corpora in `tmp/`
with explicit `config.move_family_policy` overrides.

## Exact commands

### Build one-case corpora

```bash
jq '{schema_version, cases: [.cases[] | select(.id=="lind_marcus_a_to_c") | .config.move_family_policy = "mixed"]}' \
  research/cases.json > tmp/cases_lind_marcus_a_to_c_mixed_2026-04-17.json
jq '{schema_version, cases: [.cases[] | select(.id=="lind_marcus_a_to_c") | .config.move_family_policy = "graph-only"]}' \
  research/cases.json > tmp/cases_lind_marcus_a_to_c_graph_only_2026-04-17.json
jq '{schema_version, cases: [.cases[] | select(.id=="lind_marcus_a_to_c") | .config.move_family_policy = "graph_plus_structured"]}' \
  research/cases.json > tmp/cases_lind_marcus_a_to_c_graph_plus_structured_2026-04-17.json

jq '{schema_version, cases: [.cases[] | select(.id=="full_2_shift_higher_block_1x1_to_4x4") | .config.move_family_policy = "mixed"]}' \
  research/cases.json > tmp/cases_full_2_shift_higher_block_1x1_to_4x4_mixed_2026-04-17.json
jq '{schema_version, cases: [.cases[] | select(.id=="full_2_shift_higher_block_1x1_to_4x4") | .config.move_family_policy = "graph-only"]}' \
  research/cases.json > tmp/cases_full_2_shift_higher_block_1x1_to_4x4_graph_only_2026-04-17.json
jq '{schema_version, cases: [.cases[] | select(.id=="full_2_shift_higher_block_1x1_to_4x4") | .config.move_family_policy = "graph_plus_structured"]}' \
  research/cases.json > tmp/cases_full_2_shift_higher_block_1x1_to_4x4_graph_plus_structured_2026-04-17.json
```

### Run the bounded analyzer sweeps

```bash
timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases tmp/cases_lind_marcus_a_to_c_mixed_2026-04-17.json \
  --case-id lind_marcus_a_to_c \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/rerun_layer_contrast_lind_marcus_a_to_c_mixed_2026-04-17.json \
  > tmp/rerun_layer_contrast_lind_marcus_a_to_c_mixed_2026-04-17.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases tmp/cases_lind_marcus_a_to_c_graph_only_2026-04-17.json \
  --case-id lind_marcus_a_to_c \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_only_2026-04-17.json \
  > tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_only_2026-04-17.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases tmp/cases_lind_marcus_a_to_c_graph_plus_structured_2026-04-17.json \
  --case-id lind_marcus_a_to_c \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_plus_structured_2026-04-17.json \
  > tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_plus_structured_2026-04-17.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases tmp/cases_full_2_shift_higher_block_1x1_to_4x4_mixed_2026-04-17.json \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_mixed_2026-04-17.json \
  > tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_mixed_2026-04-17.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases tmp/cases_full_2_shift_higher_block_1x1_to_4x4_graph_only_2026-04-17.json \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_graph_only_2026-04-17.json \
  > tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_graph_only_2026-04-17.stdout.txt

timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases tmp/cases_full_2_shift_higher_block_1x1_to_4x4_graph_plus_structured_2026-04-17.json \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_graph_plus_structured_2026-04-17.json \
  > tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_graph_plus_structured_2026-04-17.stdout.txt
```

## Exported artifacts

Rankable-layer JSON exports:

- `tmp/rerun_layer_contrast_lind_marcus_a_to_c_mixed_2026-04-17.json`
- `tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_only_2026-04-17.json`
- `tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_plus_structured_2026-04-17.json`
- `tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_mixed_2026-04-17.json`
- `tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_graph_only_2026-04-17.json`
- `tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_graph_plus_structured_2026-04-17.json`

Matching analyzer stdout logs:

- `tmp/rerun_layer_contrast_lind_marcus_a_to_c_*.stdout.txt`
- `tmp/rerun_layer_contrast_full_2_shift_higher_block_1x1_to_4x4_*.stdout.txt`

## Results

### `lind_marcus_a_to_c`

- `mixed`
  - solved lag `2`
  - `ranked_solution_nodes = 1 / 1`
  - `layer_count = 2`
  - exported rankable layers: `2`
- `graph_only`
  - solved lag `2`
  - `ranked_solution_nodes = 0 / 1`
  - `layer_count = 0`
  - exported rankable layers: `0`
- `graph_plus_structured`
  - solved lag `2`
  - `ranked_solution_nodes = 1 / 1`
  - `layer_count = 2`
  - exported rankable layers: `2`

Rankable witness candidates under the useful policies:

- layer `0`, `forward`
  - candidate `2x2:1,1,1,1`
  - label `best_continuation`
  - `remaining_witness_lag = 1`
- layer `1`, `backward`
  - candidate `2x2:1,1,1,1`
  - label `best_continuation`
  - `remaining_witness_lag = 1`

### `full_2_shift_higher_block_1x1_to_4x4`

- `mixed`
  - solved lag `4`
  - `ranked_solution_nodes = 2 / 2`
  - `layer_count = 3`
  - exported rankable layers: `3`
- `graph_only`
  - solved lag `3`
  - `ranked_solution_nodes = 0 / 2`
  - `layer_count = 0`
  - exported rankable layers: `0`
- `graph_plus_structured`
  - solved lag `4`
  - `ranked_solution_nodes = 2 / 2`
  - `layer_count = 3`
  - exported rankable layers: `3`

Rankable witness candidates under the useful policies:

- layer `0`, `forward`
  - candidate `2x2:0,0,2,2`
  - label `best_continuation`
  - `remaining_witness_lag = 3`
- layer `1`, `backward`
  - candidate `3x3:0,0,0,0,1,1,2,1,1`
  - label `best_continuation`
  - `remaining_witness_lag = 2`
- layer `2`, `forward`
  - candidate `3x3:0,0,0,0,1,1,2,1,1`
  - label `best_continuation`
  - `remaining_witness_lag = 1`

## Held-out-family reading

This rerun materially changes the family-aware picture for the default mixed
benchmark surface.

The earlier held-out endpoint artifact in
`research/notes/2026-04-16-layer-contrast-ranking-labels.md` already supplied:

- `riedel_baker`: `5` rankable held-out pairs
- `40` rankable held-out layers / matched witness observations

Adding the new mixed reruns from this note gives:

- `lind_marcus`: `1` rankable held-out pair, `2` rankable observations
- `higher_block`: `1` rankable held-out pair, `3` rankable observations

So the current mixed held-out endpoint picture is now:

- held-out families present: `3 / 3`
- held-out families rankable: `3 / 3`
- rankable held-out pairs: `7`
- ranked held-out observations: `45`

Against `research/ranking_signal_family_benchmark_v1.json`, that clears the
minimum meaningful coverage gate:

- at least `3` held-out families present
- at least `2` held-out families rankable
- at least `4` rankable held-out pairs
- at least `20` ranked held-out observations

So for the mixed endpoint surface, the non-Brix held-out-family result is no
longer exploratory-only.

## Conclusion

`lind_marcus` now contributes useful layer-contrast evidence under the current
tooling, but only for `mixed` or `graph_plus_structured`; `graph_only` still
solves the endpoint while exporting no observer layers.

`higher_block` also now contributes useful rankable evidence under the current
tooling, again for `mixed` or `graph_plus_structured`; `graph_only` still
solves the endpoint but exports no rankable layers.

The broader held-out-family picture therefore changes in a meaningful way for
the mixed benchmark lane: with `riedel_baker` already rankable, these two new
families lift the current held-out endpoint coverage above the family-aware
benchmark threshold.

## Recommendation

The next useful bounded follow-up is:

1. Refresh the durable mixed held-out endpoint layer-contrast artifact so the
   benchmark-facing corpus reflects all three non-Brix held-out families.
2. Add an analyzer follow-up so endpoint-case policy sweeps can honor an
   explicit move-family override without rewriting temporary case files.

Do **not** treat `graph_only` as benchmark-ready for held-out family claims:
on both newly probed families it remains zero-signal under the current
observer/layer-contrast surface.
