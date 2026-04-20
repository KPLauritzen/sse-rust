# Brix-Ruiz `k=4` graph-plus-structured `4x4 -> 3` row-relation admission gate was a no-op (2026-04-20)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, does the
surviving `binary_sparse_rectangular_factorisation_4x3_to_3` family still spend
work on `4x4` sources that are singular but do not satisfy the stronger
row-relation vocabulary forced by an actual binary-sparse `4x3` witness?

This stayed inside the requested bounded slice:

- one concrete stronger `4x4 -> 3` family-local certificate beyond determinant
  singularity;
- one minimal lane-local admission change on the retained
  `graph_plus_structured` surface;
- one retained-lane measurement pass on the kept hard case; and
- no revisit of the already-kept explicit `4x4 -> 3x3` amalgamation cut, no
  beam-order retune, and no broad solver rewrite.

## Hypothesis

The stronger exact source-side criterion was:

- if a `4x4` source admits a `binary_sparse_rectangular_factorisation_4x3_to_3`
  witness, then after some row ordering one source row must equal one of a
  small finite set of rational combinations of the other three rows;
- for the binary-sparse `4x3` family used here, that finite set collapses to
  `27` normalized row-relation profiles with denominator `1` or `2`;
- therefore, on `MoveFamilyPolicy::GraphPlusStructured`, the surviving
  `4x4 -> 3` family can be skipped unless the current source satisfies at least
  one of those exact family-induced row relations.

Why this is strictly stronger than plain determinant singularity:

- determinant singularity only says the four rows are linearly dependent;
- this gate requires the dependence to match one of the specific rational
  coefficient patterns that can actually arise from an invertible binary-sparse
  `4x3` witness.

## Temporary Slice

Temporary implementation under test:

- add an exact source-side helper that checks the `27` admissible
  `4x4 -> 3` row-relation profiles on the current source matrix;
- apply that helper only when visiting
  `binary_sparse_rectangular_factorisation_4x3_to_3` under
  `MoveFamilyPolicy::GraphPlusStructured`;
- keep `MoveFamilyPolicy::Mixed` unchanged; and
- keep the broader retained `4x4 -> 3` family itself intact when the source
  satisfies the certificate.

Files touched:

- `src/factorisation.rs`

## Measurement Surface

Retained lane and bounds:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- retained surface: `beam256 + lag40 + dim4 + entry12`

Comparison baseline:

- the currently kept retained-lane baseline from
  [2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-4x4-to-3x3-amalgamation-cut-keep.md](2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-4x4-to-3x3-amalgamation-cut-keep.md)
- that note records the same lane after the earlier explicit `4x4 -> 3x3`
  amalgamation cut, with:
  `outcome = unknown`, `elapsed = 23546 ms`,
  `frontier_nodes_expanded = 19970`,
  `factorisations_enumerated = 487699`,
  `candidates_after_pruning = 271803`,
  `discovered_nodes = 176662`,
  `approximate_other_side_hits = 184`,
  `total_visited_nodes = 176664`,
  `focus_progress_score = 87050000`,
  `directed_progress_score = 19783000`

Local artifacts from this probe:

- single-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_row_relation_gate.json`
- measured run after the temporary row-relation admission gate:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_row_relation_gate_2026-04-20.json`

Exact commands:

```bash
python - <<'PY'
import json
from pathlib import Path
src = json.loads(Path('research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json').read_text())
keep = 'brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12'
out = {
    'schema_version': src.get('schema_version', 1),
    'cases': [case for case in src['cases'] if case['id'] == keep],
}
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_row_relation_gate.json')
path.write_text(json.dumps(out, indent=2) + '\n')
print(path)
PY

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 60s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_row_relation_gate.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_row_relation_gate_2026-04-20.json
```

## Results

Compared with the current kept retained-lane baseline:

| Field | Baseline | After row-relation gate |
| --- | --- | --- |
| outcome | `unknown` | `unknown` |
| elapsed | `23546 ms` | `23850 ms` |
| frontier expanded | `19970` | `19970` |
| factorisations | `487699` | `487699` |
| candidates after pruning | `271803` | `271803` |
| discovered nodes | `176662` | `176662` |
| approximate hits | `184` | `184` |
| visited | `176664` | `176664` |
| terminal bottleneck | `factorisation_volume` | `factorisation_volume` |
| focus progress score | `87050000` | `87050000` |
| directed progress score | `19783000` | `19783000` |

The surviving `4x4 -> 3` family telemetry was also unchanged:

- `binary_sparse_rectangular_factorisation_4x3_to_3`:
  `23617 generated`, `2282 kept`, `7 discovered`, `0 approximate hits`

Interpretation:

- on the retained hard case, every source that actually reaches the surviving
  `4x4 -> 3` family already satisfies the stronger row-relation certificate;
- the new family-local gate therefore never fires in a way that changes
  retained-lane telemetry; and
- this exact source-side certificate is mathematically sharper than
  determinant singularity, but it is still not the next useful spend-better
  seam on this lane.

## Validation

Focused checks run before the retained-lane measurement:

```bash
timeout -k 20s 120s cargo test -p sse-core --lib \
  test_binary_sparse_factorisation_4x4_to_3_row_relation_gate_keeps_known_witness_source \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_binary_sparse_factorisation_4x4_to_3_row_relation_gate_rejects_singular_nonfamily_source \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_graph_plus_structured_keeps_binary_sparse_4x4_to_3_on_row_relation_compatible_source \
  -- --test-threads=1
```

Observed result:

- all three focused tests passed

Formatter attempted before commit:

```bash
timeout -k 20s 60s cargo fmt --all
```

Observed result:

- timed out without output in this workmux setup after the temporary gate had
  already been reverted, so the final committed diff remained markdown-only

## Decision

Decision: **reject**

Reason:

- the stronger row-relation certificate is a real exact family-local criterion,
  not another determinant restatement;
- on the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane it is still
  an exact no-op, with unchanged global telemetry and unchanged
  `binary_sparse_rectangular_factorisation_4x3_to_3` family counts; and
- that means the next fresh `4x4 -> 3` seam must be stronger in a different
  way than either determinant singularity or this finite row-relation
  compatibility check.

Durable conclusion:

- do not keep the row-relation admission gate for
  `binary_sparse_rectangular_factorisation_4x3_to_3` on the retained lane;
- the measured retained hard case already lives entirely inside that stronger
  certificate surface.
