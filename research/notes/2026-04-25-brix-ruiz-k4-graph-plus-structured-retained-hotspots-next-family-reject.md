# Brix-Ruiz `k=4` retained graph-plus-structured hotspots do not expose a clean next non-weighted family (2026-04-25)

## Question

For bead `sse-rust-nw7.2`, does the retained open Brix-Ruiz `k=4`
`graph_plus_structured` lane still expose one concrete non-weighted
structured-family hypothesis worth implementing, or should this lane be
rejected for now as lacking a clean next family?

This round stayed on the requested surface:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- retained case: `beam256 + lag40 + dim4 + entry12`
- no weighted `4x4 -> 3` reopening
- no beam direction/order retune
- no partition/dimension tie-break replay
- no broad mixed search or generic path enumeration

## Scorecard

Round type: retained-lane hotspot mining / next-family selection.

Fixed control:

- same single retained case as the kept 2026-04-19 amalgamation-cut baseline;
- native case timeout `24000 ms`;
- same `beam256 + lag40 + dim4 + entry12` bounds.

Useful-reach fields:

- `actual_outcome`
- `frontier_nodes_expanded`
- `discovered_nodes`
- `approximate_other_side_hits`
- `total_visited_nodes`
- `focus_progress_score`
- `directed_progress_score`
- family-local `discovered_nodes` and `approximate_other_side_hits`

Budget fields:

- `elapsed_ms`
- `factorisations_enumerated`
- `candidates_after_pruning`
- family-local `candidates_generated` and `candidates_after_pruning`

Keep threshold:

- one concrete non-weighted family or family-local seam with unspent retained
  signal and a small enough implementation slice to test next.

Reject threshold:

- all high-signal retained families are already covered by a kept cut, no-op,
  or reject note, and remaining candidates would amount to retuning, replaying
  closed surfaces, or generic broadening.

## Fresh Retained-Lane Telemetry

Local artifacts:

- single-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_2.json`
- retained-lane run:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_run_2026-04-25_nw7_2.json`
- family summary:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_family_summary_2026-04-25_nw7_2.tsv`
- per-layer `diagonal_refactorization_4x4` summary:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_layer_diag4_2026-04-25_nw7_2.tsv`

Commands:

```bash
python - <<'PY'
import json
from pathlib import Path
src = json.loads(Path('research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json').read_text())
keep = 'brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12'
cases = [case for case in src['cases'] if case['id'] == keep]
if len(cases) != 1:
    raise SystemExit(f'expected one case, found {len(cases)}')
out = {'schema_version': src.get('schema_version', 1), 'cases': cases}
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_2.json')
path.write_text(json.dumps(out, indent=2) + '\n')
print(path)
PY

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-25_nw7_2.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_run_2026-04-25_nw7_2.json

jq -r '(["family","generated","kept","discovered","approx_hits","exact_meets","kept_per_generated","discovered_per_kept"] | @tsv), (.cases[0].telemetry.move_family_telemetry | to_entries | sort_by(-.value.candidates_generated)[] | [.key, .value.candidates_generated, .value.candidates_after_pruning, .value.discovered_nodes, .value.approximate_other_side_hits, .value.exact_meets, (if .value.candidates_generated == 0 then 0 else (.value.candidates_after_pruning/.value.candidates_generated) end), (if .value.candidates_after_pruning == 0 then 0 else (.value.discovered_nodes/.value.candidates_after_pruning) end)] | @tsv)' \
  tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_run_2026-04-25_nw7_2.json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_family_summary_2026-04-25_nw7_2.tsv

jq -r '.cases[0].telemetry.layers[] | [.layer_index,.direction,.factorisations_enumerated,.candidates_after_pruning,.discovered_nodes,.approximate_other_side_hits,((.move_family_telemetry.diagonal_refactorization_4x4 // {candidates_generated:0,candidates_after_pruning:0,discovered_nodes:0,approximate_other_side_hits:0}) | [.candidates_generated,.candidates_after_pruning,.discovered_nodes,.approximate_other_side_hits] | @tsv)] | @tsv' \
  tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_retained_run_2026-04-25_nw7_2.json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_layer_diag4_2026-04-25_nw7_2.tsv
```

Fresh retained-lane result:

| Field | 2026-04-19 kept baseline | 2026-04-25 fresh run |
| --- | ---: | ---: |
| actual outcome | `unknown` | `unknown` |
| elapsed | `23546 ms` | `23334 ms` |
| frontier expanded | `19970` | `19970` |
| factorisations | `487699` | `487699` |
| candidates after pruning | `271803` | `271803` |
| discovered nodes | `176662` | `176662` |
| approximate hits | `184` | `184` |
| visited | `176664` | `176664` |
| terminal bottleneck | `factorisation_volume` | `factorisation_volume` |
| focus progress score | `87050000` | `87050000` |
| directed progress score | `19783000` | `19783000` |

The fresh run is telemetry-identical to the kept baseline on all useful-reach
and work-count fields apart from normal elapsed-time noise.

Top retained family telemetry:

| Family | Generated | Kept | Discovered | Approx. hits | Read |
| --- | ---: | ---: | ---: | ---: | --- |
| `binary_sparse_rectangular_factorisation_3x3_to_4` | `202296` | `2915` | `1930` | `2` | largest raw generator, weak retained-direction signal |
| `elementary_conjugation` | `120478` | `89222` | `28904` | `90` | graph/same-dim baseline family, not a new structured-family slice |
| `insplit` | `73424` | `47849` | `47023` | `28` | graph family, not a new structured-family slice |
| `outsplit` | `70250` | `46917` | `45716` | `27` | graph family, not a new structured-family slice |
| `diagonal_refactorization_4x4` | `57585` | `51031` | `35778` | `37` | active, but already probed as the clean diagonal follow-up |
| `binary_sparse_rectangular_factorisation_4x3_to_3` | `23617` | `2282` | `7` | `0` | retained but no approximate-hit signal |

## Hotspot Read

### `binary_sparse_rectangular_factorisation_3x3_to_4`

This remains the largest retained generator, but it does not expose a clean
next family:

- the explicit `3x3 -> 4x4` split families were already cut from
  `GraphPlusStructured` while preserving reach;
- internal orbit dedup on this exact hotspot was already measured as a strict
  retained-lane no-op; and
- the fresh run still shows only `2` approximate hits from `202296` generated
  candidates, so the current signal is raw volume rather than a new concrete
  family direction.

Reopening this seam now would likely replay duplicate-witness or explicit-split
work that already has retained-lane evidence.

### `binary_sparse_rectangular_factorisation_4x3_to_3`

This is still the retained `4x4 -> 3` structured surface after the kept
amalgamation cut, but the fresh telemetry is not enough to justify another
family:

- only `7` discovered nodes and `0` approximate hits on the retained run;
- determinant/singular admission was already a no-op;
- the stronger row-relation admission gate was already a no-op;
- unconditional weighted widening regressed the retained cap; and
- staged weighted fallback also regressed the retained cap.

The hard boundary explicitly excludes reopening the weighted variants. The
remaining unweighted source-side certificates already measured as no-ops on
this retained case.

### `diagonal_refactorization_4x4`

This is the only active non-weighted structured family with substantial
retained approximate-hit signal:

- `57585` generated
- `51031` kept
- `35778` discovered
- `37` approximate hits

However, the clean diagonal follow-up has already been tried. The
single-doubled-only variant restricted the binary diagonal vocabulary from all
non-scalar `{1,2}` patterns to the smallest nontrivial pattern class, but it
regressed the retained case from `unknown` to `timeout` at the native cap.

The exact-family note also rejected `diagonal_refactorization_4x4` as a clean
family-preserving orbit/certificate seam because row-divide and column-divide
witnesses are heterogeneous and any admissible permutation set depends on the
individual diagonal multiplicities. Splitting this family again by row-vs-column
branch or diagonal Hamming weight would be a ranking/admission replay around an
already-rejected diagonal restriction, not a new clean structured-family
hypothesis.

### Graph Families And 3x3 Same-Dimension Shears

The remaining high-signal approximate hits come from graph families
(`elementary_conjugation`, `insplit`, `outsplit`) and not from a new
non-weighted structured family. Retuning their order or beam behavior is
outside this bead.

The `3x3` shear/conjugation families are active but do not show retained
approximate-hit signal in the fresh family telemetry, so they do not justify a
new `k=4` structured-family implementation slice from this run.

## Decision

Decision: **reject this retained lane for now as lacking a clean next
non-weighted structured-family hypothesis.**

Reason:

- the fresh retained run reproduces the kept baseline exactly on the useful
  reach and work-count fields;
- the largest raw structured hotspot,
  `binary_sparse_rectangular_factorisation_3x3_to_4`, already has its obvious
  explicit-family and orbit-dedup seams spent;
- the surviving `4x4 -> 3` family has no retained approximate-hit signal and
  its unweighted admission gates were already no-ops;
- the only strong active non-weighted structured hotspot,
  `diagonal_refactorization_4x4`, already had the clean concrete restriction
  rejected under the native retained cap; and
- the remaining apparent ideas are either graph-family retunes, closed
  weighted variants, partition/dimension replay, or broad mixed search.

No follow-up bead was opened because there is no concrete implementation slice
from this retained-lane read that clears the bar.

## Validation

No Rust code changed in this round.

Validation command before commit:

```bash
timeout -k 20s 120s cargo fmt --all
```
