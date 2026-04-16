# Triangle-collapsible witness-path telemetry (2026-04-16)

## Question

On the existing graph-path and guide-artifact corpora, how much short
witness-path redundancy collapses under a bounded Wagoner-style local rewrite
quotient, if the quotient only knows about:

- direct-vs-two-hop triangles already present in the corpus;
- distinct two-hop paths with the same endpoints, treated as a commuting-square
  rewrite family;
- no new search moves, no RS-space machinery, and no theorem-engine claims.

## Tooling added

- `src/path_quotient.rs`
  Small research-only helper that:
  - extracts short suffix windows from existing full paths;
  - mines lag-1 and lag-2 endpoint-preserving alternatives;
  - canonicalizes each short window by bounded local rewrite exploration.
- `src/bin/analyze_triangle_path_telemetry.rs`
  Research-facing binary that loads full paths from guide artifacts and/or the
  legacy `research/k3-graph-paths.sqlite` corpus, runs the quotient analysis,
  prints a compact summary, and can write JSON artifacts.

## What counts as collapse here

All paths are first canonicalized matrix-by-matrix with `canonical_perm()`.

The analysis then treats every contiguous lag-`<= 4` subpath as a short suffix
window of its enclosing prefix and applies only these local rules:

1. **Triangle rewrite**
   If the corpus contains both `A -> C` and `A -> B -> C`, rewrite the two-hop
   window to the direct lag-1 representative.
2. **Commuting-square rewrite**
   If the corpus contains two distinct two-hop windows
   `A -> B1 -> C` and `A -> B2 -> C`, rewrite either two-hop window to the
   lexicographically smaller two-hop representative for that endpoint pair.

For each unique short window, the helper explores the reachable local rewrite
graph with a per-window cap of `256` states and chooses the canonical
representative by `(lag, matrix-sequence)` order.

A window counts as **collapsed** if that canonical representative differs from
the original window. A window counts as **lag-reduced** if the canonical lag is
strictly smaller.

## Commands

Fixture sanity check:

```bash
cargo run --quiet --features research-tools --bin analyze_triangle_path_telemetry -- \
  --guide-artifacts research/guide_artifacts/generic_shortcut_permutation_3x3_pool.json \
  --max-suffix-lag 3 --max-rewrite-states 64 --max-samples 8
```

Bounded corpus runs:

```bash
cargo run --quiet --features research-tools --bin analyze_triangle_path_telemetry -- \
  --path-db research/k3-graph-paths.sqlite \
  --max-suffix-lag 4 --max-rewrite-states 256 --max-samples 12 \
  --json-out research/runs/2026-04-16-triangle-path-telemetry-graph.json

cargo run --quiet --features research-tools --bin analyze_triangle_path_telemetry -- \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-suffix-lag 4 --max-rewrite-states 256 --max-samples 12 \
  --json-out research/runs/2026-04-16-triangle-path-telemetry-guides.json

cargo run --quiet --features research-tools --bin analyze_triangle_path_telemetry -- \
  --path-db research/k3-graph-paths.sqlite \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-suffix-lag 4 --max-rewrite-states 256 --max-samples 12 \
  --json-out research/runs/2026-04-16-triangle-path-telemetry-combined.json
```

## Results

### Fixture sanity check

- The direct-vs-two-hop permutation fixture behaved as expected.
- The two-hop guide `generic_guided_permutation_3x3_two_hop [0..2]` collapsed
  from lag `2` to lag `1` via a single triangle rewrite.

### Graph-path corpus only

Artifact:

- `research/runs/2026-04-16-triangle-path-telemetry-graph.json`

Summary:

- source paths: `11`
- suffix-window occurrences: `286`
- unique suffix windows: `159`
- terminal-state collision groups: `18`
- endpoint collision groups: `46`
- triangle endpoint pairs: `16`
- triangle two-step windows: `18`
- commuting-square endpoint pairs: `4`
- commuting-square two-step windows: `8`
- endpoint groups explained by local rewrites: `18`
- endpoint groups not explained by local rewrites: `28`
- collapsed window occurrences: `77`
- lag-reduced window occurrences: `72`
- windows touching a triangle rewrite on the canonical path: `72`
- windows touching a commuting-square rewrite on the canonical path: `10`
- truncated windows: `0`
- unique windows after quotient: `159 -> 89` (`44.0%` reduction)

Representative collapses:

- `sqlite:2:graph_path_result_2_ordinal_1 [6..8]`: lag `2 -> 1` via triangle.
- `sqlite:1:hardcoded endpoint 16-path [1..4]`: lag `3 -> 1` via triangle.
- `sqlite:3:hardcoded endpoint 16-path [0..4]`: lag `4 -> 2` via triangle plus
  a commuting-square normalization.

### Normalized guide-pool corpus only

Artifact:

- `research/runs/2026-04-16-triangle-path-telemetry-guides.json`

Summary:

- source paths: `12`
- suffix-window occurrences: `548`
- unique suffix windows: `387`
- terminal-state collision groups: `39`
- endpoint collision groups: `99`
- triangle endpoint pairs: `46`
- triangle two-step windows: `79`
- commuting-square endpoint pairs: `33`
- commuting-square two-step windows: `66`
- endpoint groups explained by local rewrites: `46`
- endpoint groups not explained by local rewrites: `53`
- collapsed window occurrences: `296`
- lag-reduced window occurrences: `296`
- windows touching a triangle rewrite on the canonical path: `296`
- windows touching a commuting-square rewrite on the canonical path: `2`
- truncated windows: `0`
- unique windows after quotient: `387 -> 160` (`58.7%` reduction)

Representative collapses:

- `k3-sqlite-shortcut-9 [6..8]`: lag `2 -> 1` via triangle, repeated `6` times.
- `k3-sqlite-shortcut-12 [7..10]`: lag `3 -> 1` via triangle, repeated `3`
  times.
- `k3-sqlite-shortcut-12 [6..10]`: lag `4 -> 2` via triangle, repeated `3`
  times.

### Combined graph + guide corpus

Artifact:

- `research/runs/2026-04-16-triangle-path-telemetry-combined.json`

Summary:

- source paths: `22`
- suffix-window occurrences: `812`
- unique suffix windows: `444`
- terminal-state collision groups: `42`
- endpoint collision groups: `110`
- triangle endpoint pairs: `51`
- triangle two-step windows: `85`
- commuting-square endpoint pairs: `36`
- commuting-square two-step windows: `72`
- endpoint groups explained by local rewrites: `53`
- endpoint groups not explained by local rewrites: `57`
- collapsed window occurrences: `373`
- lag-reduced window occurrences: `368`
- windows touching a triangle rewrite on the canonical path: `368`
- windows touching a commuting-square rewrite on the canonical path: `12`
- truncated windows: `0`
- unique windows after quotient: `444 -> 187` (`57.9%` reduction)

## Interpretation

1. **Local triangle collapse is common in the current witness/guide material.**
   The guide pool especially contains many short suffixes that canonically drop
   by one or two steps once direct-vs-two-hop alternatives are identified.
2. **Commuting-square relations are present, but mostly secondary.**
   They show up in the mined catalog (`36` endpoint pairs in the combined run),
   yet only `12` combined-window canonicalizations used a square rewrite at all,
   and most of the actual lag reduction still came from triangles.
3. **The quotient explains a meaningful subset, not all, of path redundancy.**
   In the combined run, `53` endpoint-collision groups had a local rewrite
   explanation, but `57` endpoint-collision groups did not. So this seam is
   useful telemetry, not a full local proof calculus.
4. **This is squarely a path-canonicalization surface, not a move-family change.**
   The analysis only rewrites already-stored witness windows from the existing
   corpora. It does not introduce new frontier successors or claim any new
   mathematical implication beyond those observed corpus relations.

## Conclusion

The bounded Wagoner-style experiment is worth keeping as a research-only seam.
On the present `k=3` graph/guide corpora, short local quotienting does detect
substantial triangle-collapsible redundancy, especially in guide artifacts, and
it does so without requiring any change to the main search expansion logic.

The next sensible follow-up, if needed, is still narrow:

- report this quotient alongside guide-path quality/lag metadata;
- measure whether quotient-normalized guide pools are materially smaller or less
  duplicative before shortcut-search consumes them;
- do **not** promote the local rewrites into default search moves without
  separate frontier-benefit evidence.
