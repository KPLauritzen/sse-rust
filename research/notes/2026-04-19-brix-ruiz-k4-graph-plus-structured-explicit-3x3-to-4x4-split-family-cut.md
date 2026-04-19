# Brix-Ruiz `k=4` graph-plus-structured cut: drop explicit `3x3 -> 4x4` split families (2026-04-19)

## Question

On the retained open Brix-Ruiz `k = 4` `graph_plus_structured` lane, do the
explicit `single_row_split_3x3_to_4x4` and
`single_column_split_3x3_to_4x4` families earn their keep, or are they only
adding duplicate factorisation volume on the retained
`beam256 + dim4 + entry12` surface?

## Change

One bounded family-level adjustment was tested:

- for `MoveFamilyPolicy::GraphPlusStructured`, disable the explicit
  `single_row_split_3x3_to_4x4` and `single_column_split_3x3_to_4x4` families
- keep the broader `binary_sparse_rectangular_factorisation_3x3_to_4` family
  enabled, so the lane still has a structured `3x3 -> 4x4` lift path
- leave `MoveFamilyPolicy::Mixed` unchanged

Why this slice was chosen:

- on the baseline retained lane, both explicit split families generated large
  raw volume and retained zero successors after pruning
- baseline retained-lane telemetry:
  - `beam256 + lag20 + dim4 + entry12`:
    `single_row_split_3x3_to_4x4 = 35,077 -> 0 kept`,
    `single_column_split_3x3_to_4x4 = 34,209 -> 0 kept`
  - `beam256 + lag40 + dim4 + entry12`:
    `single_row_split_3x3_to_4x4 = 48,498 -> 0 kept`,
    `single_column_split_3x3_to_4x4 = 50,196 -> 0 kept`

## Measurement Surface

Main measurement lane:

- open Brix-Ruiz `k = 4`
- `graph_plus_structured`
- retained surface: `beam256 + dim4 + entry12`

Measured corpus:

- committed corpus:
  `research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json`
- retained-lane cases used for the keep/reject decision:
  `beam256 + lag20/30/40 + dim4 + entry12`

Local run artifacts:

- baseline:
  `tmp/nw71_baseline_brix_ruiz_k4_graph_plus_structured_broad_beam.json`
- after family cut:
  `tmp/nw71_after_split_family_cut_brix_ruiz_k4_graph_plus_structured_broad_beam.json`

Reproduce:

```bash
cargo build --bin research_harness --features research-tools

timeout -k 20s 190s target/debug/research_harness \
  --cases research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  --format json \
  > tmp/nw71_baseline_brix_ruiz_k4_graph_plus_structured_broad_beam.json

timeout -k 20s 190s target/debug/research_harness \
  --cases research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  --format json \
  > tmp/nw71_after_split_family_cut_brix_ruiz_k4_graph_plus_structured_broad_beam.json
```

## Results

Retained `beam256 + dim4 + entry12` surface:

| Case | Outcome | Frontier expanded | Factorisations | Avg factorisations / expanded node | Focus progress | Directed progress |
| --- | --- | --- | --- | --- | --- | --- |
| `beam256 + lag20 + dim4 + entry12` | `unknown -> unknown` | `9,730 -> 9,730` | `441,907 -> 372,621` (`-69,286`, `-15.68%`) | `45.42 -> 38.30` | `43,050,000 -> 43,050,000` | `13,655,000 -> 13,655,000` |
| `beam256 + lag30 + dim4 + entry12` | `timeout -> timeout` | `0 -> 0` | `0 -> 0` | `0.0 -> 0.0` | `0 -> 0` | `0 -> 0` |
| `beam256 + lag40 + dim4 + entry12` | `unknown -> unknown` | `19,970 -> 19,970` | `592,101 -> 493,407` (`-98,694`, `-16.67%`) | `29.65 -> 24.71` | `87,050,000 -> 87,050,000` | `19,783,000 -> 19,783,000` |

What moved:

- witness status: no change
- frontier / visited-volume proxy: no change on the retained lane
  (`frontier_nodes_expanded` stayed identical on the non-timeout cases)
- ranking-quality / reach proxy: no change
  (`focus_progress_score`, `directed_progress_score`, and
  `last_layer_candidates_after_pruning` stayed identical)
- factorisation cost: improved materially
  (`-167,980` factorisations, `-16.25%` across the retained beam256 surface)

Broader corpus sanity check:

- all non-timeout dim4 cases in the committed broad-beam corpus dropped from
  `1,918,349` to `1,620,822` factorisations (`-297,527`, `-15.51%`)
- no case changed witness status or frontier-expanded volume

Elapsed time did not move enough to matter:

- small wins on `beam128` and `beam256 + lag20`
- small loss on `beam256 + lag40`
- no timeout flipped on the retained lane

## Validation

Focused tests:

```bash
timeout -k 20s 120s cargo test -p sse-core --lib single_row_split -- --test-threads=1
timeout -k 20s 120s cargo test -p sse-core --lib single_column_split -- --test-threads=1
timeout -k 20s 120s cargo test -p sse-core --lib \
  test_selected_family_labels_for_graph_plus_structured_3x3_skip_square_family \
  -- --test-threads=1
```

Observed result:

- all focused tests passed

Formatter:

```bash
timeout -k 20s 120s cargo fmt --all
```

Observed result:

- completed successfully in this session

## Decision

Decision: **keep**

Reason:

- on the retained open Brix-Ruiz `k = 4` `beam256 + dim4 + entry12` lane, the
  explicit `3x3 -> 4x4` split families were pure duplicate generation volume
- removing them preserved witness status, frontier-expanded volume, and
  progress/ranking proxies while cutting factorisation cost by about `16%`
- that is enough same-budget improvement to keep the family cut in
  `graph_plus_structured`
