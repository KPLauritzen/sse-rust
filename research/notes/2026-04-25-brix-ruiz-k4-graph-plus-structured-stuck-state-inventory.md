# Brix-Ruiz `k=4` retained graph-plus-structured stuck-state inventory (2026-04-25)

## Question

For bead `sse-rust-nw7.6`, extract concrete high-progress stuck states from the
retained open Brix-Ruiz `k=4` `graph_plus_structured` case:

`brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`

This is an evidence pass only:

- no new move family;
- no beam retune;
- no broad mixed search;
- no generic `4x4` factorisation enumeration revival; and
- no reopening of rejected weighted `4x4 -> 3` variants.

## Sources And Artifacts

Read first:

- `research/notes/2026-04-17-brix-ruiz-k4-graph-plus-structured-broad-beam.md`
- `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-retained-hotspots-next-family-reject.md`
- `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-4x4-to-3-row-relation-admission-gate-no-op.md`
- `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-staged-weighted-4x4-to-3-fallback-reject.md`
- `docs/autoresearch-round-scorecards.md`
- `src/search_observer.rs`
- `src/bin/profile_square_factorisation_sources.rs`
- `research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json`

New diagnostic:

- `src/bin/extract_brix_ruiz_k4_stuck_states.rs`

Local extraction artifacts:

- full JSON report:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_6.json`
- top ranked approximate pairs:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_top24_2026-04-25_nw7_6.tsv`
- family evidence:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_family_evidence_2026-04-25_nw7_6.tsv`
- family-local approximate-hit sources:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_sources_top40_2026-04-25_nw7_6.tsv`
- layers with approximate hits:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_approx_layers_2026-04-25_nw7_6.tsv`
- materialized single-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_6.json`
- independent retained-case harness replay:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_run_2026-04-25_nw7_6.json`

Commands:

```bash
timeout -k 20s 180s cargo build --features research-tools --bin extract_brix_ruiz_k4_stuck_states

timeout -k 20s 120s target/debug/extract_brix_ruiz_k4_stuck_states \
  --json-out tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_6.json \
  --top 220

jq '{schema_version: (.schema_version // 1), cases: [.cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")]}' \
  research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_6.json

timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_6.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_run_2026-04-25_nw7_6.json

jq -r '(["rank","layer","direction","family","from_depth","to_depth","counterpart_depth","bridge_depth","slack","l1","signature","from_matrix","to_matrix","counterpart_matrix"] | @tsv), (.ranked_approximate_hits[:24][] | [.rank,.layer_index,.direction,.move_family,.from_depth,.to_depth,.counterpart_depth,.bridge_depth,.bridge_slack_at_lag40,.counterpart_l1,((.signature.row_sums|join("/"))+" x "+(.signature.col_sums|join("/"))),(.from_matrix.data|join(",")),(.to_matrix.data|join(",")),((.counterpart_matrix.data // [])|join(","))] | @tsv)' \
  tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_6.json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_top24_2026-04-25_nw7_6.tsv
```

The extractor uses only the existing `SearchObserver` event surface. It records
per-edge approximate hits, the closest already-seen opposite-side state with
the same approximate signature, parent-family edge counts, and per-layer
context. Matrix data below is row-major.

## Control Reproduction

Fresh extractor replay:

| Field | Value |
| --- | ---: |
| outcome | `unknown` |
| frontier nodes expanded | `19970` |
| factorisations enumerated | `487699` |
| candidates after pruning | `271803` |
| discovered nodes | `176662` |
| approximate hits | `184` |
| total visited nodes | `176664` |
| terminal bottleneck | `factorisation_volume` |
| focus progress score | `87050000` |
| directed progress score | `19783000` |

This matches the retained baseline useful-reach/work-count telemetry from the
2026-04-25 hotspot note.

## Family-Local Evidence

| Family | Edges | Discovered | Seen collisions | Approx. hits | Reading |
| --- | ---: | ---: | ---: | ---: | --- |
| `elementary_conjugation` | `89222` | `28904` | `60318` | `90` | dominant near-cap stuck-pair source, but graph-family |
| `diagonal_refactorization_4x4` | `51031` | `35778` | `15253` | `37` | best structured high-progress signal |
| `insplit` | `47849` | `47023` | `826` | `28` | graph-family continuity, mostly shallow/front-loaded |
| `outsplit` | `46917` | `45716` | `1201` | `27` | graph-family continuity, mostly shallow/front-loaded |
| `binary_sparse_rectangular_factorisation_3x3_to_4` | `2915` | `1930` | `985` | `2` | real but shallow boundary signal |
| `binary_sparse_rectangular_factorisation_4x3_to_3` | `2282` | `7` | `2275` | `0` | no approximate-hit support |

The full retained run still has no exact meets. The useful state-level signal is
therefore approximate-signature overlap, not a hidden exact bridge.

## Ranked Stuck States And Pairs

The ranking prioritizes approximate pairs whose two endpoint depths almost
consume the retained `lag40` budget. `slack = 40 - (to_depth +
counterpart_depth)`. `l1` is the entrywise distance between canonical
representatives of the approximate-hit state and the closest opposite-side
same-signature state.

| Rank | Family | Layer / side | Depths | Slack | l1 | Signature | State pair read |
| ---: | --- | --- | --- | ---: | ---: | --- | --- |
| 1 | `elementary_conjugation` | `65` / backward | `38 + 2 = 40` | `0` | `28` | rows `1/4/5/13`, cols `0/0/10/13` | exact-budget 4x4 approximate pair; graph move reaches the signature surface but not the layout |
| 2 | `elementary_conjugation` | `60` / backward | `33 + 6 = 39` | `1` | `16` | rows `3/4/5/11`, cols `0/0/6/17` | closest near-cap pair by l1 among slack-1 hits |
| 3 | `elementary_conjugation` | `46` / forward | `18 + 21 = 39` | `1` | `34` | rows `1/4/5/13`, cols `0/0/7/16` | independent near-cap graph-family layout miss |
| 4 | `diagonal_refactorization_4x4` | `75` / forward | `36 + 2 = 38` | `2` | `22` | rows `0/0/10/14`, cols `2/4/5/13` | strongest structured near-cap pair |
| 6 | `diagonal_refactorization_4x4` | `58` / backward | `31 + 6 = 37` | `3` | `14` | rows `3/4/5/11`, cols `0/0/6/17` | best structured pair by l1 among high-progress hits |
| 9 | `diagonal_refactorization_4x4` | `61` / backward | `34 + 2 = 36` | `4` | `18` | rows `3/4/5/11`, cols `0/0/7/16` | repeat of the same active-block signature shape |
| 20 | `outsplit` | `40` / forward | `12 + 21 = 33` | `7` | `14` | rows `3/4/5/11`, cols `0/0/5/18` | graph split can also land near the same sparse 4x4 signature surface |
| 133 | `binary_sparse_rectangular_factorisation_3x3_to_4` | `5` / forward | `2 + 4 = 6` | `34` | `10` | rows `1/1/4/13`, cols `0/4/5/10` | shallow 3x3-to-4 boundary hit, not high-progress |
| 157 | `binary_sparse_rectangular_factorisation_3x3_to_4` | `5` / forward | `2 + 2 = 4` | `36` | `18` | rows `1/3/4/13`, cols `0/3/6/12` | second shallow 3x3-to-4 boundary hit |

Concrete rank-4 structured pair:

- from depth `35` forward:
  `[[1,4,1,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]`
- diagonal step `U = diag(1,1,2,1)` produces:
  `[[1,4,2,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]`
- closest opposite-side same-signature state at depth `2`:
  `[[1,12,0,1],[1,1,4,4],[0,0,0,0],[0,0,0,0]]`

Both children have the same sorted row sums `0/0/10/14`, sorted column sums
`2/4/5/13`, and the same row/column support profile. The miss is not aggregate
mass; it is the active `2 x 4` layout inside a sparse `4 x 4` boundary state.

## Frontier And Layer Context

Approximate hits are not only early noise. The top structured hit appears at
layer `75`, where the beam is still saturated:

| Layer | Side | Frontier | Candidates | Discovered | Dead-end nodes | Approx. hits |
| ---: | --- | ---: | ---: | ---: | ---: | ---: |
| `5` | forward | `256` | `19797` | `19218` | `0` | `29` |
| `7` | forward | `256` | `2739` | `1779` | `36` | `13` |
| `46` | forward | `256` | `2145` | `1277` | `3` | `10` |
| `58` | backward | `256` | `3415` | `2055` | `8` | `2` |
| `60` | backward | `256` | `3550` | `2204` | `12` | `4` |
| `61` | backward | `256` | `3352` | `2037` | `5` | `1` |
| `65` | backward | `256` | `3367` | `2179` | `3` | `1` |
| `75` | forward | `256` | `2532` | `1077` | `23` | `2` |

Layer `75` family-local `diagonal_refactorization_4x4` telemetry:

- `806` candidates generated;
- `749` after pruning;
- `465` discovered; and
- `1` approximate hit.

So the structured rank-4 state is not a global dead end. It is a productive
4x4 diagonal-refactorization parent that exposes the missing local bridge only
after substantial endpoint progress.

## 3x3/4x4 Boundary Reading

The `3x3 -> 4` binary-sparse family produced exactly two approximate hits, both
at layer `5`, from forward depth `1` to depth `2`. Their bridge slack values are
`34` and `36`, so they are evidence that the boundary can touch the opposite
signature surface, but not evidence for a high-progress bridge.

The high-work low-yield boundary parents are instead target-side collision
sources. Examples from `low_yield_parents`:

| Side | Depth | Family | Edges | Discovered | Seen | Matrix |
| --- | ---: | --- | ---: | ---: | ---: | --- |
| backward | `28` | `binary_sparse_rectangular_factorisation_3x3_to_4` | `28` | `0` | `28` | `[[1,1,0],[12,1,0],[18,1,0]]` |
| backward | `34` | `binary_sparse_rectangular_factorisation_3x3_to_4` | `28` | `0` | `28` | `[[1,1,0],[12,1,0],[20,1,0]]` |
| backward | `10` | `binary_sparse_rectangular_factorisation_3x3_to_4` | `27` | `0` | `27` | `[[0,17,1],[0,1,1],[0,12,1]]` |
| backward | `22` | `binary_sparse_rectangular_factorisation_3x3_to_4` | `27` | `0` | `27` | `[[0,1,19],[0,1,12],[0,1,1]]` |

This supports the earlier rejection: the live retained `4x4 -> 3` surface is
not where approximate progress is hiding, and the opposite `3x3 -> 4` boundary
mostly replays already-seen states once it reaches high depth.

## Keep/Reject Reading

Keep as durable evidence:

- the new extractor, because it turns aggregate approximate-hit telemetry into
  concrete state/state-pair evidence without changing solver behavior;
- the rank-4 and rank-6 diagonal `4x4` stuck pairs as the best structured
  evidence for a bounded next proposal; and
- the low-yield backward `3x3 -> 4` parents as negative boundary context.

Reject as next-family basis:

- weighted `4x4 -> 3` reopening, already rejected and still unsupported by this
  extraction;
- another `4x4 -> 3` row/determinant admission gate, because the retained family
  has `7` discoveries and `0` approximate hits;
- generic `4x4` factorisation enumeration; and
- a beam/order retune, because this bead needed state evidence rather than more
  aggregate reach.

## Missing-Local-Transformation Hypothesis For `sse-rust-nw7.7`

The best bounded hypothesis is not "add more `4x4 -> 3`." It is:

> Test a small proposal slice for sparse `4x4` states with two zero rows/columns
> where a diagonal refactorization reaches the same sorted row sums, column
> sums, and support profile as an opposite-side state, but the active `2 x 4`
> layout differs. The local move to investigate is a row/column-sum-preserving
> active-block transfer, such as a bounded `2 x 2` contingency switch inside
> the nonzero rows/columns, seeded only from concrete approximate-hit pairs like
> rank 4 and rank 6.

This should be treated as a counterexample-guided proposal experiment, not as a
claim that such switches are valid SSE moves. The bound is concrete: seed from
the retained extractor's diagonal `4x4` approximate pairs, probe only the
active sparse block, and compare whether the proposal can reduce the exact
canonical distance to the recorded counterpart states.

No follow-up bead was opened; `sse-rust-nw7.7` already covers the next bounded
move/proposal design step.

## Validation

Focused validation for this note:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
timeout -k 20s 180s cargo build --features research-tools --bin profile_square_factorisation_sources
timeout -k 20s 180s cargo build --features research-tools --bin extract_brix_ruiz_k4_stuck_states
timeout -k 20s 120s target/debug/extract_brix_ruiz_k4_stuck_states --json-out tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_6.json --top 220
```

Observed result:

- all five commands passed;
- the harness replay produced `unknown` in `23320 ms`; and
- the extractor replay preserved the retained telemetry counts listed above.
