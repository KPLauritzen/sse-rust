# Brix-Ruiz `k=4` graph-plus-structured `4x4 -> 3` singular-admission gate was a no-op (2026-04-20)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, does the
surviving `binary_sparse_rectangular_factorisation_4x3_to_3` family still waste
work on nonsingular `4x4` source matrices, so that a singular-only admission
gate would improve the retained `beam256 + dim4 + entry12` surface?

This stayed within the requested bounded slice:

- one concrete `4x4 -> 3` admission hypothesis;
- one temporary lane-local policy change to test it;
- one retained-lane measurement pass on the kept hard case; and
- no beam-order retune, no generic solver rewrite, and no revisit of the
  already-kept explicit `4x4 -> 3x3` amalgamation cut.

## Hypothesis

The hypothesis was:

- any `4x4 -> 3` factorisation has rank at most `3`, so the source `4x4`
  matrix must be singular;
- therefore, on `MoveFamilyPolicy::GraphPlusStructured`, the surviving
  `binary_sparse_rectangular_factorisation_4x3_to_3` family could be admitted
  only when the current `4x4` source has determinant `0`;
- if the retained lane still visits many nonsingular `4x4` sources that would
  otherwise probe this family, that gate should reduce factorisation work
  without changing frontier behavior.

Why this looked plausible:

- after the explicit `4x4 -> 3x3` amalgamation siblings were cut, the retained
  lane still spent nontrivial budget on the broader sparse `4x4 -> 3` family:
  `23617 generated`, `2282 kept`, `7 discovered` on the current kept
  `beam256 + lag40 + dim4 + entry12` surface.

## Temporary Slice

Temporary implementation under test:

- add a source-side admission gate for
  `binary_sparse_rectangular_factorisation_4x3_to_3`;
- apply it only on `MoveFamilyPolicy::GraphPlusStructured`;
- keep `MoveFamilyPolicy::Mixed` unchanged;
- reject the family before enumeration when the current `4x4` source is
  nonsingular.

This code was measured and then reverted. The final worktree keeps only this
note and the measurement artifacts.

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
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20.json`
- measured run after the temporary singular-only admission gate:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_singular_only_4x4_to_3_gate_2026-04-20.json`

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
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20.json')
path.write_text(json.dumps(out, indent=2) + '\n')
print(path)
PY

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 60s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_singular_only_4x4_to_3_gate_2026-04-20.json
```

## Results

The retained-lane measurement was an exact no-op against the current kept
baseline.

| Field | Baseline | After temporary singular-only gate |
| --- | --- | --- |
| outcome | `unknown` | `unknown` |
| elapsed | `23546 ms` | `23508 ms` |
| frontier expanded | `19970` | `19970` |
| factorisations | `487699` | `487699` |
| candidates after pruning | `271803` | `271803` |
| discovered nodes | `176662` | `176662` |
| approximate hits | `184` | `184` |
| visited | `176664` | `176664` |
| terminal bottleneck | `factorisation_volume` | `factorisation_volume` |
| focus progress score | `87050000` | `87050000` |
| directed progress score | `19783000` | `19783000` |

The `4x4 -> 3` family telemetry itself was also unchanged:

- `binary_sparse_rectangular_factorisation_4x3_to_3`:
  `23617 generated`, `2282 kept`, `7 discovered`, `0 approximate hits`

Interpretation:

- the temporary singular-only gate did not fire on the retained hard case in
  any way that changed search telemetry;
- the unchanged family counts strongly suggest that the retained-lane sources
  that actually reach this family are already singular; and
- even if the determinant criterion is mathematically correct, it is not the
  next useful spend-better admission seam on this lane.

## Validation

Focused temporary checks run before the retained-lane measurement:

```bash
timeout -k 20s 120s cargo test -p sse-core --lib \
  test_binary_sparse_factorisation_4x4_to_3_source_gate_rejects_nonsingular_4x4 \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_binary_sparse_factorisation_4x4_to_3_source_gate_keeps_known_singular_source \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_graph_plus_structured_keeps_binary_sparse_4x4_to_3_on_known_singular_source \
  -- --test-threads=1
```

Observed result:

- all three focused temporary tests passed before the code was reverted

Formatter for the final reverted worktree:

```bash
timeout -k 20s 120s cargo fmt --all
```

## Decision

Decision: **reject**

Reason:

- on the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, the
  singular-only admission rule for the surviving `4x4 -> 3` sparse family is a
  strict no-op;
- the retained-lane telemetry and the family's own generated/kept counts are
  unchanged; and
- that means this exact determinant-based admission seam is not where the
  retained lane is leaking budget.

Durable conclusion:

- do not keep a singular-only `4x4 -> 3` admission gate in
  `GraphPlusStructured`;
- the next fresh `4x4 -> 3` seam should look for a stronger family-local
  certificate than plain determinant singularity.
