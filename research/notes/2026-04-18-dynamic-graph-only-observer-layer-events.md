# Dynamic graph-only endpoint layer events (2026-04-18)

## Question

Does adding observer `SearchEvent::Layer` parity to the dynamic endpoint BFS
path with `move_family_policy = graph_only` change the bounded
layer-contrast export outcome for `lind_marcus_a_to_c`?

Bounded case in scope:

- `lind_marcus_a_to_c`

This follows the mixed-only fix in
`research/notes/2026-04-16-dynamic-mixed-endpoint-layer-events.md` and the
policy-comparison rerun in
`research/notes/2026-04-17-non-brix-layer-contrast-rerun-after-mixed-observer-fix.md`,
which had left dynamic `graph_only` observer parity out of scope and measured
`lind_marcus_a_to_c` `graph_only` as solved-but-unrankable with `layers = 0`.

## Slice

Kept the seam.

Patched exactly the dynamic endpoint BFS path with:

- `move_family_policy = graph_only`
- no search-policy changes
- no move-family broadening
- no mixed or graph-plus-structured changes beyond shared observer parity

Implementation point:

- `src/search.rs`
  - `search_graph_only_dyn_with_telemetry`

The change mirrors the existing observer/event behavior already used by the
dynamic mixed and `2x2` graph-only paths:

- retain graph-only steps only when an observer is present
- collect per-layer `SearchEdgeRecord`s for:
  - `SeenCollision`
  - `Discovered`
  - `ExactMeet`
- call `emit_layer(&mut observer, records)`:
  - when the layer telemetry is committed
  - immediately before returning on an exact meet

## Focused validation

Targeted tests:

```bash
cargo test -q test_dyn_mixed_search_observer_emits_layers_for_lind_marcus_case
cargo test -q test_dyn_graph_only_search_observer_emits_layers_for_lind_marcus_case
```

Both passed.

Analyzer build:

```bash
cargo build --quiet --features research-tools --bin analyze_path_signal_corpus
```

One-case endpoint corpus with explicit `graph-only` policy override:

```bash
jq '{schema_version, cases: [.cases[] | select(.id=="lind_marcus_a_to_c") | .config.move_family_policy = "graph-only"]}' \
  research/cases.json > tmp/cases_lind_marcus_a_to_c_graph_only_2026-04-18.json
```

Bounded rerun:

```bash
timeout -k 10s 30s target/debug/analyze_path_signal_corpus \
  --cases tmp/cases_lind_marcus_a_to_c_graph_only_2026-04-18.json \
  --case-id lind_marcus_a_to_c \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_only_2026-04-18.json \
  > tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_only_2026-04-18.stdout.txt
```

Artifacts:

- one-case corpus:
  - `tmp/cases_lind_marcus_a_to_c_graph_only_2026-04-18.json`
- analyzer stdout:
  - `tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_only_2026-04-18.stdout.txt`
- exported layer-contrast artifact:
  - `tmp/rerun_layer_contrast_lind_marcus_a_to_c_graph_only_2026-04-18.json`

## Results

`lind_marcus_a_to_c` `graph_only` becomes rankable.

Analyzer stdout reports:

- `solved_cases = 1`
- `unranked_solved_cases = 0`
- `ranked_solution_nodes = 1 / 1`
- case line:
  - `lind_marcus_a_to_c budget_lag=2 solved_lag=2 ranked=1/1 layers=2`

Exported artifact summary:

- `exported_cases = 1`
- `exported_rankable_cases = 1`
- `exported_rankable_layers = 2`
- `exported_matched_candidates = 2`
- `exported_families = ["lind_marcus"]`

Rankable layers in the export:

- layer `0`, `forward`
  - `layer_size = 1`
  - matched witness candidate: `2x2:1,1,1,1`
  - label: `best_continuation`
  - `remaining_witness_lag = 1`
- layer `1`, `backward`
  - `layer_size = 2`
  - matched witness candidate: `2x2:1,1,1,1`
  - label: `best_continuation`
  - `remaining_witness_lag = 1`

## Interpretation

The dynamic `graph_only` observer parity seam is worth keeping.

For this bounded endpoint case, the observer-layer gap was the missing export
surface: after restoring parity, `lind_marcus_a_to_c` no longer remains
unrankable under `graph_only`; it exports `2` rankable layers from the solved
path at lag `2`.

The analyzer still reports `config.search_mode = "mixed"` in artifact metadata.
As established in the 2026-04-17 note, endpoint cases loaded from
`research/cases.json` follow the per-case `config.move_family_policy`, so the
real policy override here comes from the one-case corpus written to `tmp/`,
not from analyzer `--search-mode`.
