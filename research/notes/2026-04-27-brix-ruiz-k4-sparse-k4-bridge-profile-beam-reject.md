# Brix-Ruiz `k=4` sparse bridge-profile beam scout (2026-04-27)

## Question

For bead `sse-rust-nw7`, test one fresh retained-lane ranking hypothesis on
the open Brix-Ruiz `k=4` `graph_plus_structured` surface:

```text
brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12
```

The hypothesis is that the retained diagonal-refactorization stuck states point
to a useful sparse active-block bridge profile. This is not another active-block
switch proposal and not a validity claim for a new SSE move. It only changes
beam ranking in an opt-in frontier mode.

## Hypothesis

Add opt-in frontier mode:

```text
sparse_k4_bridge_profile_beam
```

The mode uses the normal beam executor and default beam score, then applies a
`-48.0` score bonus to square `4x4` candidates with the retained sparse profile:

```text
support = 7
row supports x column supports =
  0/0/3/4 x 1/2/2/2
  or the transpose
```

This profile matches the concrete rank-4 retained stuck pair in
`research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`
and its transposed rank-6 shape. It is materially different from the frozen
`witness_bridge_profile_beam`, which uses solved-witness support profiles and
does not include this sparse `4x4` retained-k4 shape.

Implementation files:

- `src/path_scoring.rs`
- `src/search/beam.rs`
- `src/search.rs`
- `src/types.rs`
- CLI / label plumbing in `src/bin/search.rs`, `src/bin/brix_ruiz_k3.rs`,
  `src/bin/research_harness/execution.rs`, `src/bin/evaluate_positive_conjugacy_usefulness.rs`,
  and `src/sqlite_graph.rs`

The default `beam`, `bfs`, and `graph_plus_structured` behavior is unchanged.

## Commands And Artifacts

Focused validation:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test -p sse-core --lib sparse_k4_bridge_profile --features research-tools -- --test-threads=1
timeout -k 20s 180s cargo test -p sse-core --lib test_frontier_mode_deserializes_bfs_and_beam --features research-tools -- --test-threads=1
timeout -k 20s 180s cargo test -p sse-core --bin search parse_cli_accepts_sparse_k4_bridge_profile_beam_mode --features research-tools -- --test-threads=1
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
```

A/B corpus:

```bash
timeout 30 jq -n \
  --slurpfile k4 research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  '{schema_version:5, cases: [
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__beam_baseline"
      | .description="Retained open Brix-Ruiz k4 baseline beam variant for sse-rust-nw7 sparse-k4 bridge profile scout."),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__sparse_k4_bridge_profile"
      | .description="Retained open Brix-Ruiz k4 sparse-k4 bridge profile beam variant for sse-rust-nw7."
      | .config.frontier_mode="sparse_k4_bridge_profile_beam")
  ]}' > tmp/nw7_sparse_k4_bridge_profile_ab_cases.json
```

One lane-local measurement attempt:

```bash
timeout -k 20s 120s target/debug/research_harness \
  --cases tmp/nw7_sparse_k4_bridge_profile_ab_cases.json \
  --format json \
  > tmp/nw7_sparse_k4_bridge_profile_ab_results.json
```

Artifacts:

- `tmp/nw7_sparse_k4_bridge_profile_ab_cases.json`
- `tmp/nw7_sparse_k4_bridge_profile_ab_results.json`

## Results

| Case | Outcome | Elapsed ms | Exact hits | Approx hits | Max frontier | Visited | Expanded | Factorisations | Kept candidates | Discovered | Focus progress | Directed progress |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline `beam` | `unknown` | `23061` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` | `87050000` | `19783000` |
| `sparse_k4_bridge_profile_beam` | `unknown` | `16247` | `0` | `133` | `256` | `125765` | `19943` | `446892` | `213410` | `125763` | `87050000` | `14550000` |

Family-local approximate-hit comparison:

| Family | Baseline approx hits | Sparse-profile approx hits | Reading |
| --- | ---: | ---: | --- |
| `elementary_conjugation` | `90` | `65` | large loss in graph-family overlap |
| `diagonal_refactorization_4x4` | `37` | `31` | targeted structured family still loses hits |
| `insplit` | `28` | `17` | graph split continuity worsens |
| `outsplit` | `27` | `18` | graph split continuity worsens |
| `binary_sparse_rectangular_factorisation_3x3_to_4` | `2` | `2` | shallow boundary signal unchanged |

The sparse profile reduced work and elapsed time, but this is not useful search
per unit budget: approximate overlap fell `184 -> 133`, directed progress fell
`19783000 -> 14550000`, and there were no exact meets.

## Decision

Decision: **reject as a retained Brix-Ruiz k4 ranking hypothesis.**

The retained sparse active-block profile is real stuck-state evidence, but
promoting it into beam ranking over-focuses the search and cuts away productive
graph-family and diagonal-refactorization continuity. The lower elapsed time
and lower candidate volume are vanity wins here because useful reach and
directed progress both regress.

Keep `sparse_k4_bridge_profile_beam` only as an opt-in reproduction hook for
this negative result. Do not promote it to default `beam`, do not combine it
with `witness_bridge_profile_beam`, and do not open a follow-up bead from this
slice.
