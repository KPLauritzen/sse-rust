# Brix-Ruiz `k=4` graph-plus-structured staged weighted `4x4 -> 3` fallback still regresses the retained cap (2026-04-20)

## Question

Can the rejected weighted-row `4x4 -> 3` family be narrowed enough to keep its
continuity signal without reopening the retained
`beam256 + lag40 + dim4 + entry12` Brix-Ruiz `k=4` timeout?

This round stayed within the requested bounded slice:

- one minimal staging seam only;
- no beam-order retune;
- no generic weighted framework;
- no reopening of determinant-only or row-relation gates; and
- one retained-lane measurement on the kept `graph_plus_structured` surface.

## Hypothesis

The restricted seam tested here was:

- keep the existing
  `binary_sparse_rectangular_factorisation_4x3_to_3` family first;
- add one staged weighted sibling only on `MoveFamilyPolicy::GraphPlusStructured`;
- defer that sibling until after the current source matrix produces **zero**
  binary-sparse `4x4 -> 3` witnesses; and
- keep the weighted vocabulary itself minimal by allowing only the solved
  fourth row of the `4x3` factor to be weighted/non-binary.

This is stricter than the rejected unconditional weighted family because the
weighted vocabulary never fires on sources that the retained sparse family
already covers.

## Temporary Slice

Temporary implementation under test:

- add `visit_weighted_binary_sparse_factorisations_4x4_to_3` as the narrow
  weighted sibling of the existing `4x4 -> 3` sparse family;
- wire it into `visit_factorisations_with_family_for_policy` only for
  `MoveFamilyPolicy::GraphPlusStructured`;
- emit it under its own telemetry label:
  `staged_weighted_binary_sparse_rectangular_factorisation_4x3_to_3`; and
- only invoke that label if the current source emitted no
  `binary_sparse_rectangular_factorisation_4x3_to_3` witnesses.

Files touched temporarily during the probe:

- `src/factorisation.rs`

This code was measured and then reverted. The final worktree keeps only this
note and the measurement artifacts.

## Measurement Surface

Retained lane and fixed control:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- retained surface: `beam256 + lag40 + dim4 + entry12`

Comparison baseline from
[2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-4x4-to-3x3-amalgamation-cut-keep.md](2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-4x4-to-3x3-amalgamation-cut-keep.md):

- `actual_outcome = unknown`
- `elapsed_ms = 23546`
- `frontier_nodes_expanded = 19970`
- `factorisations_enumerated = 487699`
- `candidates_after_pruning = 271803`
- `discovered_nodes = 176662`
- `approximate_other_side_hits = 184`
- `total_visited_nodes = 176664`
- `focus_progress_score = 87050000`
- `directed_progress_score = 19783000`

Local artifacts from this probe:

- single-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_staged_weighted_4x4_to_3_fallback.json`
- retained-case run at the native `24000 ms` cap:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_staged_weighted_4x4_to_3_fallback_2026-04-20.json`
- diagnostic rerun at `32000 ms`:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_staged_weighted_4x4_to_3_fallback_timeout32s.json`
- diagnostic result:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_staged_weighted_4x4_to_3_fallback_timeout32s_2026-04-20.json`

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
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_staged_weighted_4x4_to_3_fallback.json')
path.write_text(json.dumps(out, indent=2) + '\n')
print(path)
PY

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 60s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_staged_weighted_4x4_to_3_fallback.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_staged_weighted_4x4_to_3_fallback_2026-04-20.json

python - <<'PY'
import json
from pathlib import Path
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_staged_weighted_4x4_to_3_fallback.json')
obj = json.loads(path.read_text())
obj['cases'][0]['timeout_ms'] = 32000
out = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_staged_weighted_4x4_to_3_fallback_timeout32s.json')
out.write_text(json.dumps(obj, indent=2) + '\n')
print(out)
PY

timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_staged_weighted_4x4_to_3_fallback_timeout32s.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_staged_weighted_4x4_to_3_fallback_timeout32s_2026-04-20.json
```

## Results

### Primary retained-case measurement

At the actual retained `24000 ms` cap, the staged fallback still regressed the
lane from `unknown` to `timeout`:

| Field | Baseline | Staged weighted fallback at retained cap |
| --- | --- | --- |
| actual outcome | `unknown` | `timeout` |
| elapsed | `23546 ms` | `24023 ms` |
| timeout cap | `24000 ms` | `24000 ms` |
| reason | none | `worker exceeded 24000 ms` |

Because the run timed out, the primary artifact did not preserve the useful
search telemetry fields.

### Diagnostic rerun at `32000 ms`

The single diagnostic rerun finished and showed that the staged seam is real,
but still too expensive for the native retained cap:

| Field | Baseline | Staged weighted fallback at `32000 ms` |
| --- | --- | --- |
| actual outcome | `unknown` | `unknown` |
| elapsed | `23546 ms` | `24753 ms` |
| frontier expanded | `19970` | `19970` |
| factorisations | `487699` | `565195` |
| candidates after pruning | `271803` | `296070` |
| discovered nodes | `176662` | `191224` |
| approximate hits | `184` | `187` |
| visited | `176664` | `191226` |
| terminal bottleneck | `factorisation_volume` | `factorisation_volume` |
| focus progress score | `87050000` | `87050000` |
| directed progress score | `19783000` | `20077000` |

Family-local telemetry on that diagnostic:

- `binary_sparse_rectangular_factorisation_4x3_to_3`:
  `23784 generated`, `2207 kept`, `6 discovered`
- `staged_weighted_binary_sparse_rectangular_factorisation_4x3_to_3`:
  `79056 generated`, `23431 kept`, `13168 discovered`

Read against the earlier rejected unconditional weighted family:

- the staged seam did substantially reduce weighted-family volume versus the
  prior unconditional run (`79056` staged generated vs `224685` unconditional);
- it still produced the same bounded approximate-hit count (`187`) and a
  slightly stronger directed progress score (`20077000` vs `20038000` on the
  earlier `32000 ms` run); but
- the retained `24000 ms` control still regressed to timeout, so the improved
  spend shape was not strong enough to earn a keep.

## Validation

Focused temporary checks run before the retained-lane measurement and the later
revert:

```bash
timeout -k 20s 180s cargo test -p sse-core --lib \
  test_weighted_binary_sparse_factorisations_reach_weighted_last_row_bridge \
  -- --test-threads=1

timeout -k 20s 180s cargo test -p sse-core --lib \
  test_graph_plus_structured_staged_weighted_4x4_to_3_fallback_only_fires_without_binary_sparse \
  -- --test-threads=1

timeout -k 20s 180s cargo test -p sse-core --lib \
  test_graph_plus_structured_staged_weighted_4x4_to_3_fallback_exposes_weighted_only_source \
  -- --test-threads=1

timeout -k 20s 180s cargo test -p sse-core --lib \
  test_selected_family_labels_for_graph_plus_structured_4x4_skip_explicit_amalgamation_families \
  -- --test-threads=1
```

Observed result:

- all four focused tests passed

Formatter for the final reverted worktree:

```bash
timeout -k 20s 120s cargo fmt --all
```

Observed result:

- in this workmux setup the formatter invocation did not return within the
  bounded wait after the temporary code had already been reverted, so the final
  diff remained markdown-only

## Decision

Decision: **reject**

Reason:

- the staged seam is a real narrowing of the weighted `4x4 -> 3` family;
- it materially lowers weighted-family volume compared with the rejected
  unconditional widening; but
- on the actual retained `beam256 + lag40 + dim4 + entry12` cap it still
  regresses the hard case from `unknown` to `timeout`.

Durable conclusion:

- do not keep the staged weighted `4x4 -> 3` fallback on the retained
  Brix-Ruiz `k=4` `graph_plus_structured` lane;
- if this family is revisited later, it needs a stronger seam than
  "binary-sparse miss -> weighted fallback" to fit inside the native retained
  cap.
