# Brix-Ruiz `k=4` active-block switch proposal diagnostic (2026-04-25)

## Question

For bead `sse-rust-nw7.7`, take one retained
`graph_plus_structured` hotspot from the nw7.6 stuck-state inventory and test
the smallest counterexample-guided structured move proposal: a bounded `2x2`
contingency switch inside the active block of the sparse `4x4` state.

This is a proposal diagnostic only. It does not change default solver behavior,
does not add a selected factorisation family, does not enumerate generic `4x4`
factorisations, and does not reopen the weighted `4x4 -> 3` surfaces.

## Hotspot

Selected retained approximate-hit pair:

- case:
  `brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`
- extractor rank: `4`
- layer / side: `75` / forward
- family that exposed the stuck pair: `diagonal_refactorization_4x4`
- depths: from `35`, to `36`, counterpart `2`, slack at lag40 `2`
- retained canonical L1 to counterpart: `22`

Matrix data is row-major.

Source-side parent:

```text
[[1,4,1,7],
 [3,1,0,6],
 [0,0,0,0],
 [0,0,0,0]]
```

The retained diagonal refactorization uses `D = diag(1,1,2,1)` and reaches:

```text
[[1,4,2,7],
 [3,1,0,6],
 [0,0,0,0],
 [0,0,0,0]]
```

Closest opposite-side same-signature state:

```text
[[1,12,0,1],
 [1, 1,4,4],
 [0, 0,0,0],
 [0, 0,0,0]]
```

The two states have the same sorted row sums, sorted column sums, and sorted
row/column support profile after canonicalization:

- row sums: `0/0/10/14`
- column sums: `2/4/5/13`
- row supports: `0/0/3/4`
- column supports: `1/2/2/2`

So the obstruction is not aggregate mass. It is the active `2x4` layout.

## Why Existing Families Miss This Local Transition

The selected family already reaches the signature surface, but its local move is
diagonal refactorization: `A = D X -> X D` or `A = X D -> D X` with binary
diagonal entries. In this hotspot that only moves the effect of a single
diagonal multiplier into the active block. It does not redistribute mass while
holding both active row sums and active column sums fixed.

The other selected `graph_plus_structured` families also do not describe this
local transition:

- graph split/amalgamation/conjugation families can change support and layout,
  but the retained run still records only approximate overlap, no exact meet;
- the selected dimension-changing `4x4 -> 3` rectangular family has `0`
  approximate hits on the retained run and is the wrong surface for this
  same-dimension sparse block;
- explicit `4x4 -> 3` row/column amalgamation families are not enabled under
  `GraphPlusStructured`; and
- generic `4x4` factorisation enumeration is outside this bead.

The proposal below is therefore distinct from the selected structured-family
labels, but it is not proven to be an SSE-valid move.

## Proposal Diagnostic

New research-only binary:

```bash
target/debug/diagnose_brix_ruiz_k4_active_block_switches \
  --input tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_7.json \
  --rank 4 \
  --json-out tmp/brix_ruiz_k4_active_block_switch_rank4_2026-04-25_nw7_7.json
```

Definition:

1. Read the retained stuck-state extractor JSON.
2. Select one `diagonal_refactorization_4x4` approximate-hit rank.
3. Require a sparse `4x4` state with exactly two nonzero rows and four active
   columns.
4. Enumerate every nonzero nonnegative `2x2` switch over the two active rows
   and any two active columns, with `delta <= 12` by default and a hard CLI
   ceiling of `64`:

```text
[a b] -> [a+d b-d]   or   [a b] -> [a-d b+d]
[c e]    [c-d e+d]        [c+d e-d]
```

5. Mark a proposal as accepted by the diagnostic only if it preserves the same
   sorted row sums, sorted column sums, and sorted support profile as the
   retained approximate-hit state.
6. Measure exact canonical L1 distance to the recorded counterpart.

This is deliberately a proposal screen. It does not emit solver successors.

## Proposal Counts

Rank-4 result:

| Field | Value |
| --- | ---: |
| total nonnegative switches | `18` |
| accepted, signature-preserving | `10` |
| rejected, signature-changing | `8` |
| accepted switches improving canonical L1 | `2` |
| maximum delta cap | `12` |
| base canonical L1 | `22` |
| best canonical L1 | `20` |
| exact canonical match found | `false` |

Best accepted proposals:

```text
row_pair=[0,1], col_pair=[0,2], add_main_diagonal, delta=2
candidate =
[[3,4,0,7],
 [1,1,2,6],
 [0,0,0,0],
 [0,0,0,0]]
canonical L1: 20
```

```text
row_pair=[0,1], col_pair=[2,3], add_anti_diagonal, delta=2
candidate =
[[1,4,0,9],
 [3,1,2,4],
 [0,0,0,0],
 [0,0,0,0]]
canonical L1: 20
```

The screen therefore finds a real local direction signal, but only a small L1
improvement and no exact bridge.

## Same-Budget Baseline

The diagnostic is not integrated as a move family, so there is no solver A/B
with proposal successors. The same-budget retained replay is included as the
baseline context for the hotspot.

| Field | Retained baseline / replay |
| --- | ---: |
| outcome | `unknown` |
| elapsed | `23478 ms` |
| frontier nodes expanded | `19970` |
| factorisations enumerated | `487699` |
| candidates after pruning | `271803` |
| discovered nodes | `176662` |
| approximate hits | `184` |
| visited | `176664` |
| terminal bottleneck | `factorisation_volume` |
| focus progress score | `87050000` |
| directed progress score | `19783000` |
| `diagonal_refactorization_4x4` generated | `57585` |
| `diagonal_refactorization_4x4` kept | `51031` |
| `diagonal_refactorization_4x4` discovered | `35778` |
| `diagonal_refactorization_4x4` approximate hits | `37` |

## Decision

Recommendation: **reject promotion of the `2x2` active-block switch as an SSE
family from this slice. Keep the diagnostic as a narrow proposal probe.**

Reason:

- the proposal is distinct from the selected structured families, because it is
  a row/column-sum-preserving active-block redistribution rather than a
  diagonal refactorization or dimension-changing factorisation;
- the best accepted switches improve exact canonical L1 from `22` to `20`, so
  the motif is not random noise;
- no accepted switch reaches the counterpart exactly;
- no SSE-valid factorisation proof is available for the switch itself; and
- integrating it as a solver move would be a new semantic claim, not just an
  implementation of an already-valid family.

At most one justified next bounded experiment: run this same diagnostic over
only the rank-4/rank-6 diagonal-refactorization retained cluster and require a
repeated exact-canonical-distance improvement before spending another bead on a
validity proof. Do not add a solver family from this single pair.

## Commands Run

```bash
timeout -k 20s 180s cargo build --features research-tools --bin extract_brix_ruiz_k4_stuck_states

timeout -k 20s 120s target/debug/extract_brix_ruiz_k4_stuck_states \
  --json-out tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_7.json \
  --top 220

timeout -k 20s 180s cargo build --features research-tools --bin diagnose_brix_ruiz_k4_active_block_switches

timeout -k 20s 60s target/debug/diagnose_brix_ruiz_k4_active_block_switches \
  --input tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_7.json \
  --rank 4 \
  --json-out tmp/brix_ruiz_k4_active_block_switch_rank4_2026-04-25_nw7_7.json

jq '{schema_version: (.schema_version // 1), cases: [.cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")]}' \
  research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_7.json

timeout -k 20s 180s cargo build --features research-tools --bin research_harness

timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_7.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_run_2026-04-25_nw7_7.json
```

## Validation

Final validation after adding the diagnostic:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
timeout -k 20s 180s cargo build --features research-tools --bin extract_brix_ruiz_k4_stuck_states
timeout -k 20s 180s cargo build --features research-tools --bin diagnose_brix_ruiz_k4_active_block_switches
timeout -k 20s 180s cargo test --features research-tools --bin diagnose_brix_ruiz_k4_active_block_switches
timeout -k 20s 60s target/debug/diagnose_brix_ruiz_k4_active_block_switches \
  --input tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_7.json \
  --rank 4 \
  --json-out tmp/brix_ruiz_k4_active_block_switch_rank4_2026-04-25_nw7_7.json
```

All commands passed.
