# Brix-Ruiz `k=4` graph-plus-structured weighted `4x4 -> 3` family regresses the retained cap (2026-04-20)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, does one
genuinely new structured-family widening help where the earlier `4x4 -> 3`
admission gates did not?

This round stayed within the requested bounded slice:

- one explicit new `4x4 -> 3` family only;
- no beam-order retune;
- no broad policy rewrite;
- no reopening of the rejected determinant or row-relation gates; and
- one retained-lane measurement on the kept `beam256 + lag40 + dim4 + entry12`
  surface.

## Hypothesis

The tested family was a transpose-dual-inspired weighted sibling of the
surviving `binary_sparse_rectangular_factorisation_4x3_to_3` family:

- keep the existing binary-sparse `3x3` core used to solve the `3x4` factor;
- allow exactly one distinguished non-binary weighted row on the `4x3` factor;
- keep the extension explicit, family-local, and concrete; and
- expose it as its own telemetry label:
  `weighted_binary_sparse_rectangular_factorisation_4x3_to_3`.

Why this was worth probing:

- it is a real structured-family widening rather than another source-side
  admission gate;
- it adds one singular weighted-row vocabulary on the `4x4 -> 3` side; and
- it is the closest bounded sibling to the already-existing weighted
  `3x3 -> 4` family that fits the `4x4 -> 3` geometry.

## Temporary Slice

Temporary implementation under test:

- add `weighted_binary_sparse_rectangular_factorisation_4x3_to_3` to the
  `4x4` family table;
- enable it wherever factorisations are permitted, including
  `MoveFamilyPolicy::GraphPlusStructured`;
- require three binary-sparse core rows plus exactly one weighted non-binary
  row on the `4x3` factor; and
- add focused factorisation and frontier-policy tests for the new family.

This code was measured and then reverted. The final worktree keeps only this
note and the local measurement artifacts.

Files touched temporarily during the probe:

- `src/factorisation.rs`
- `src/search.rs`

## Measurement Surface

Retained lane and fixed control:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- retained surface: `beam256 + lag40 + dim4 + entry12`

Comparison baseline:

- the currently kept retained-lane baseline from
  [2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-4x4-to-3x3-amalgamation-cut-keep.md](2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-4x4-to-3x3-amalgamation-cut-keep.md)
- baseline values on that same surface:
  `actual_outcome = unknown`, `elapsed_ms = 23546`,
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
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_weighted_row_family.json`
- retained-case run at the native `24000 ms` cap:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_weighted_row_family_2026-04-20.json`
- extended diagnostic rerun at `32000 ms` only to inspect telemetry:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_weighted_row_family_timeout32s_2026-04-20.json`

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
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_weighted_row_family.json')
path.write_text(json.dumps(out, indent=2) + '\n')
print(path)
PY

timeout -k 20s 240s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 60s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_weighted_row_family.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_weighted_row_family_2026-04-20.json

python - <<'PY'
import json
from pathlib import Path
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_weighted_row_family.json')
obj = json.loads(path.read_text())
obj['cases'][0]['timeout_ms'] = 32000
out = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_weighted_row_family_timeout32s.json')
out.write_text(json.dumps(obj, indent=2) + '\n')
print(out)
PY

timeout -k 20s 80s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-20_weighted_row_family_timeout32s.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_weighted_row_family_timeout32s_2026-04-20.json
```

## Results

### Primary retained-case measurement

At the actual retained `24000 ms` case cap, the new family regressed the lane
from the kept baseline's `unknown` to `timeout`:

| Field | Baseline | Weighted family at retained cap |
| --- | --- | --- |
| actual outcome | `unknown` | `timeout` |
| elapsed | `23546 ms` | `24023 ms` |
| timeout cap | `24000 ms` | `24000 ms` |
| reason | none | `worker exceeded 24000 ms` |

Because the run timed out, no productive search telemetry was recorded on that
primary measurement artifact.

### Extended diagnostic rerun

To see whether the timeout was near-miss noise or real widening cost, the same
surface was rerun with only the case timeout extended to `32000 ms`. That
diagnostic finished, but the apparent gains depended entirely on the looser
cap:

| Field | Baseline | Weighted family at `32000 ms` |
| --- | --- | --- |
| actual outcome | `unknown` | `unknown` |
| elapsed | `23546 ms` | `25636 ms` |
| frontier expanded | `19970` | `19970` |
| factorisations | `487699` | `708410` |
| candidates after pruning | `271803` | `327262` |
| discovered nodes | `176662` | `198539` |
| approximate hits | `184` | `187` |
| visited | `176664` | `198541` |
| terminal bottleneck | `factorisation_volume` | `factorisation_volume` |
| focus progress score | `87050000` | `87050000` |
| directed progress score | `19783000` | `20038000` |

Family-local telemetry on that extended diagnostic:

- `binary_sparse_rectangular_factorisation_4x3_to_3`:
  `22686 generated`, `2144 kept`, `1 discovered`
- `weighted_binary_sparse_rectangular_factorisation_4x3_to_3`:
  `224685 generated`, `60816 kept`, `24963 discovered`

Interpretation:

- the new family is not a no-op; it materially widens the retained lane;
- that widening does increase bounded continuity signals once extra time is
  granted; but
- on the actual retained cap, the added family volume is too expensive and
  causes the lane to miss the baseline finish line entirely.

Under the repo's widening-round scorecard, this is a reject:

- the fixed outer cap is part of the control;
- the primary retained-case measurement regressed from `unknown` to `timeout`;
- and the apparent continuity gain only appears after silently loosening that
  cap.

## Validation

Focused checks run before the retained-lane measurement:

```bash
timeout -k 20s 180s cargo test -p sse-core --lib weighted_binary_sparse -- --test-threads=1

timeout -k 20s 180s cargo test -p sse-core --lib \
  selected_family_labels_for_graph_plus_structured_4x4_skip_explicit_amalgamation_families \
  -- --test-threads=1

timeout -k 20s 180s cargo test -p sse-core --lib \
  selected_family_labels_for_4x4_keep_specific_before_generic \
  -- --test-threads=1
```

Observed result:

- all focused tests passed before the temporary code was reverted

Formatter for the final reverted worktree:

```bash
timeout -k 20s 120s cargo fmt --all
```

## Decision

Decision: **reject**

Reason:

- this is a real new structured-family widening on the retained
  `graph_plus_structured` lane;
- at the actual retained `beam256 + lag40 + dim4 + entry12` cap it regresses
  from `unknown` to `timeout`; and
- the only positive signal appears after extending the timeout beyond the kept
  control, which does not satisfy the widening-round keep rule.

Durable conclusion:

- do not keep `weighted_binary_sparse_rectangular_factorisation_4x3_to_3` on
  the retained Brix-Ruiz `k=4` `graph_plus_structured` lane;
- if this weighted-row vocabulary is revisited later, it should return as a
  narrower admission, ordering, or staging idea rather than as an unconditional
  family widening on the current retained cap.
