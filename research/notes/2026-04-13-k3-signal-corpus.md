# K=3 Signal Corpus Probe

## Goal

Add a repeatable way to score solution-path nodes against cheap ranking signals on
top of the existing search stack, using actual observed search layers where
possible and path-replay analysis as a broader fallback.

## Tooling Added

- `src/path_scoring.rs`
  Shared candidate-score definitions and rank-summary helpers used by both
  offline path replay and the new corpus analyzer.
- `src/bin/analyze_path_signal_corpus.rs`
  Loads full paths from guide artifacts or `research/k3-graph-paths.sqlite`,
  derives bounded segment cases, reruns endpoint search with a `SearchObserver`,
  and ranks any encountered solution-path nodes within their discovered layer.
- `src/bin/replay_graph_path_scores.rs`
  Now imports the shared scoring module instead of carrying its own copy of the
  score definitions.

## Initial Bounded Probe

Command:

```bash
cargo run --features research-tools --bin analyze_path_signal_corpus -- \
  --search-mode graph-only \
  --max-endpoint-dim 4 \
  --max-gap 2 \
  --max-cases 8
```

Observed result:

- `source_paths=11`
- `solved_cases=2`, `unsolved_cases=6`
- only one solved case produced nontrivial observer layers:
  `sqlite:2:graph_path_result_2_ordinal_1 [6..8]`
- for that case, the single interior solution node ranked:
  - top-1 for `dimension_low`
  - top-1 for `row_col_types_low`
  - top-1 for `support_types_low`
  - top-1 for `duplicates_high`
  - top-1 for `types_plus_sig_low`
  - much weaker for `endpoint_sig_low` (`37.50%`)
  - much weaker for `entry_sum_low` (`25.00%`)
  - worst for `max_entry_low` (`56.25%`)

Interpretation:

- On the first nontrivial observed-layer sample, coarse structural signals
  (dimension/type-count/duplicate structure) look better than endpoint-shape
  distance or raw entry-size signals.
- Many very short mixed-mode segment cases terminate through fast paths before
  any observer layer is emitted, so observed-layer corpus growth needs either:
  - larger endpoint dimensions,
  - graph-only forcing,
  - or a richer source corpus of solved paths.

## Offline Replay Sanity Check

Command:

```bash
cargo run --features research-tools --bin replay_graph_path_scores
```

On the existing blind endpoint 16-move path, the local-successor summary favored
the same family of coarse structural signals over endpoint-distance heuristics:

- `dimension_low`: mean percentile `19.84%`
- `row_col_types_low`: mean percentile `24.97%`
- `support_types_low`: mean percentile `31.19%`
- `max_entry_low`: mean percentile `32.51%`
- `segment_goal_sig_low`: mean percentile `53.04%`
- `entry_sum_low`: mean percentile `53.14%`
- `endpoint_sig_low`: mean percentile `57.14%`
- `entry_plus_sig_low`: mean percentile `57.77%`

This is still replay analysis rather than actual search-layer telemetry, but it
points in the same direction as the bounded observer-backed run: structure-first
signals are currently more promising than naive target-distance scores.
