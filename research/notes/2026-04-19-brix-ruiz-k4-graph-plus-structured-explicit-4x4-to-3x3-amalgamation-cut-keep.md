# Brix-Ruiz `k=4` graph-plus-structured cut: drop explicit `4x4 -> 3x3` amalgamation families (2026-04-19)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, do the
explicit `single_row_amalgamation_4x4_to_3x3` and
`single_column_amalgamation_4x4_to_3x3` families earn their keep on the
retained `beam256 + dim4 + entry12` surface, or are they only duplicate
presentations beneath the broader
`binary_sparse_rectangular_factorisation_4x3_to_3` family?

This was chosen as one fresh structured-family hypothesis:

- it stays on the retained open Brix-Ruiz `k=4` Goal 3 lane;
- it is a family admission cut, not another beam-order seam;
- it does not revisit the rejected `3x3 -> 4` orbit-dedup or tie-break ideas;
- and it mirrors the already-kept explicit `3x3 -> 4x4` split-family cut.

## Change

One minimal lane-local policy change was tested:

- for `MoveFamilyPolicy::GraphPlusStructured`, disable
  `single_row_amalgamation_4x4_to_3x3` and
  `single_column_amalgamation_4x4_to_3x3`
- keep `MoveFamilyPolicy::Mixed` unchanged
- keep the broader `binary_sparse_rectangular_factorisation_4x3_to_3` family
  enabled on the retained lane

Files touched:

- `src/factorisation.rs`
- `src/search.rs`

## Measurement Surface

Retained lane and bounds:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- retained surface: `beam256 + lag40 + dim4 + entry12`

Comparison baseline:

- the currently kept retained-lane baseline from
  `2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-3x3-to-4x4-split-family-cut.md`
- that note already records the same lane after the earlier explicit
  `3x3 -> 4x4` split-family cut:
  `outcome = unknown`, `elapsed = 23866 ms`,
  `frontier_nodes_expanded = 19970`,
  `factorisations_enumerated = 493407`,
  `candidates_after_pruning = 271803`,
  `discovered_nodes = 176662`,
  `approximate_other_side_hits = 184`,
  `total_visited_nodes = 176664`,
  `focus_progress_score = 87050000`,
  `directed_progress_score = 19783000`

Local artifacts from this bounded probe:

- single-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-19.json`
- measured run after the explicit amalgamation cut:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_explicit_amalgamation_cut_2026-04-19.json`

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
path = Path('tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-19.json')
path.write_text(json.dumps(out, indent=2) + '\n')
print(path)
PY

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 40s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case_2026-04-19.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_explicit_amalgamation_cut_2026-04-19.json
```

## Results

Compared with the current kept retained-lane baseline:

| Field | Baseline | After explicit amalgamation cut |
| --- | --- | --- |
| outcome | `unknown` | `unknown` |
| elapsed | `23866 ms` | `23546 ms` |
| frontier expanded | `19970` | `19970` |
| factorisations | `493407` | `487699` |
| candidates after pruning | `271803` | `271803` |
| discovered nodes | `176662` | `176662` |
| approximate hits | `184` | `184` |
| visited | `176664` | `176664` |
| terminal bottleneck | `factorisation_volume` | `factorisation_volume` |
| focus progress score | `87050000` | `87050000` |
| directed progress score | `19783000` | `19783000` |

Net effect:

- no witness or frontier/ranking regression on the retained hard case
- `5708` fewer factorisations (`-1.16%`)
- a small elapsed improvement (`-320 ms`) on the measured case

Family telemetry after the cut confirms that the lane still has the broader
`4x4 -> 3` sparse family while the explicit amalgamation siblings disappeared:

- `binary_sparse_rectangular_factorisation_4x3_to_3` remained active with
  `23617 generated`, `2282 kept`, `7 discovered`
- neither `single_row_amalgamation_4x4_to_3x3` nor
  `single_column_amalgamation_4x4_to_3x3` appeared in retained-lane telemetry

Interpretation:

- on this retained surface, the explicit `4x4 -> 3x3` amalgamation siblings are
  not buying unique frontier behaviour;
- the broader sparse `4x4 -> 3` family appears sufficient to preserve the
  lane's reach while the cut saves a modest amount of factorisation work.

## Validation

Focused policy-boundary tests:

```bash
timeout -k 20s 120s cargo test -p sse-core --lib \
  test_selected_family_labels_for_graph_plus_structured_4x4_skip_explicit_amalgamation_families \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_mixed_policy_exposes_single_row_amalgamation_4x4_to_3x3_witness \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_mixed_policy_exposes_single_column_amalgamation_4x4_to_3x3_witness \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_expand_frontier_layer_graph_plus_structured_skips_single_row_amalgamation_4x4_to_3x3 \
  -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_expand_frontier_layer_graph_plus_structured_skips_single_column_amalgamation_4x4_to_3x3 \
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

- this is a genuinely new structured-family cut on the retained open
  Brix-Ruiz `k=4` `graph_plus_structured` lane;
- the bounded retained-lane measurement preserved outcome, frontier-expanded
  volume, pruning volume, discovery volume, and progress scores; and
- it reduced factorisation cost without reopening any rejected beam-order seam
  or generic solver rewrite.

Durable conclusion:

- keep the explicit `4x4 -> 3x3` row/column amalgamation families out of
  `GraphPlusStructured`
- keep the broader `binary_sparse_rectangular_factorisation_4x3_to_3` family
  as the retained `4x4 -> 3` structured surface on this lane
