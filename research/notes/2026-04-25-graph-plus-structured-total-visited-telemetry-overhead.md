# Graph-plus-structured retained telemetry overhead cut (2026-04-25)

## Question

For bead `sse-rust-nw7.5`, profile one CPU/memory cost in the
`graph_plus_structured` move generation path on the retained open Brix-Ruiz
`k=4` lane and make at most one focused optimization without changing default
search semantics.

Retained case:

`brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`

Hard bounds kept:

- no move-family additions or removals;
- no beam/ranking policy change;
- no envelope broadening; and
- no search outcome or witness semantics change.

## Baseline

Single-case corpus:

`tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw75.json`

Command:

```bash
jq '{schema_version: (.schema_version // 1), cases: [.cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")]}' \
  research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw75.json

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw75.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_baseline_2026-04-25_nw75.json
```

Baseline metrics:

| Field | Value |
| --- | ---: |
| outcome | `unknown` |
| elapsed | `23186 ms` |
| frontier nodes expanded | `19970` |
| factorisation calls | `19970` |
| factorisations enumerated | `487699` |
| candidates generated | `653742` |
| candidates after pruning | `271803` |
| discovered nodes | `176662` |
| approximate other-side hits | `184` |
| total visited nodes | `176664` |

Family-count bottleneck remains factorisation volume. Top generators:

| Family | Generated | Kept | Discovered | Approx. hits |
| --- | ---: | ---: | ---: | ---: |
| `binary_sparse_rectangular_factorisation_3x3_to_4` | `202296` | `2915` | `1930` | `2` |
| `elementary_conjugation` | `120478` | `89222` | `28904` | `90` |
| `insplit` | `73424` | `47849` | `47023` | `28` |
| `outsplit` | `70250` | `46917` | `45716` | `27` |
| `diagonal_refactorization_4x4` | `57585` | `51031` | `35778` | `37` |

Layer timing showed a second-order telemetry overhead target:

| Timing bucket | Baseline |
| --- | ---: |
| summed layer timing | `22676.472 ms` |
| expand compute | `8102.842 ms` |
| expand accumulate | `80.753 ms` |
| dedup | `800.664 ms` |
| merge | `10447.493 ms` |
| finalize | `3163.909 ms` |

## Bottleneck Analysis

The dominant retained cost is still materializing and merging the large
candidate stream. The family telemetry does not expose a fresh safe family cut:
the largest raw generator, `3x3 -> 4`, keeps only about `1.4%` of generated
candidates and has only `2` approximate hits, but prior notes already spent the
obvious family/orbit cuts.

The focused implementation target here was therefore layer telemetry overhead,
not move semantics. In the graph-plus-structured `2x2` BFS endpoint path,
`total_visited_nodes` was recomputed at exact-return and normal layer
finalization by scanning the full union of forward and backward parent maps.
For the retained run this happened across `80` layers while the maps grew to
`176664` total visited nodes.

## Optimization

Kept: incremental `total_visited_nodes` accounting in
`search_graph_plus_structured_2x2_with_telemetry_and_observer`.

The path now initializes `total_visited_nodes` to the two distinct roots after
the canonical shortcut check, increments it only for non-exact discoveries, and
does not rescan the parent-map union during normal layer finalization or an
immediate exact return. Exact collisions are not counted as new union nodes,
matching the previous union-size semantics.

This changes telemetry accounting cost only. It does not change generated
families, pruning, frontier order, enqueued nodes, exact-meet handling, or
reconstructed paths.

## After Metrics

Command:

```bash
timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw75.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_after_incremental_total_visited_2026-04-25_nw75.json
```

| Field | Before | After |
| --- | ---: | ---: |
| outcome | `unknown` | `unknown` |
| elapsed | `23186 ms` | `23094 ms` |
| frontier nodes expanded | `19970` | `19970` |
| factorisation calls | `19970` | `19970` |
| factorisations enumerated | `487699` | `487699` |
| candidates generated | `653742` | `653742` |
| candidates after pruning | `271803` | `271803` |
| discovered nodes | `176662` | `176662` |
| approximate other-side hits | `184` | `184` |
| total visited nodes | `176664` | `176664` |

| Timing bucket | Before | After |
| --- | ---: | ---: |
| summed layer timing | `22676.472 ms` | `22594.354 ms` |
| expand compute | `8102.842 ms` | `8048.704 ms` |
| expand accumulate | `80.753 ms` | `81.860 ms` |
| dedup | `800.664 ms` | `801.097 ms` |
| merge | `10447.493 ms` | `10443.483 ms` |
| finalize | `3163.909 ms` | `3138.648 ms` |

The retained telemetry is work-count identical. The elapsed and finalization
movement is small and within single-run noise, but the removed parent-map union
scan is deterministic overhead and the accounting invariant is validated by the
unchanged `total_visited_nodes` value.

## Decision

Keep the incremental telemetry accounting cut. It is a narrow CPU/memory
overhead reduction on the measured path and preserves default search semantics.

Do not open a follow-up bead from this slice. The next meaningful bottleneck is
still candidate materialization and merge cost, but this pass did not expose a
fresh safe family or canonicalization cut beyond the already-spent retained-lane
surfaces.

## Validation

Focused retained-case validation:

```bash
timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness
timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw75.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_after_incremental_total_visited_2026-04-25_nw75.json
```

Required quality gates:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools search
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
cargo bench --bench search -- --noplot
```

Results:

- `cargo fmt --all`: passed.
- `cargo test --features research-tools search`: passed, `133` library tests
  passed, `1` ignored, plus matching filtered bin tests.
- `cargo build --features research-tools --bin research_harness`: passed.
- `cargo bench --bench search -- --noplot`: passed. Criterion samples:
  `endpoint_equivalent_fast` `2.6086..2.6319 us`,
  `endpoint_invariant_reject_fast` `3.6752..3.7012 us`,
  `mixed_k3_lag3_dim3_n2048` `512.50..524.75 ms`, and
  `graph_only_k3_lag8_dim4_n8192` `75.935..77.339 ms`.
