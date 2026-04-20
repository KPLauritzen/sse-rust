# Brix-Ruiz `k=4` graph-plus-structured single-doubled `diagonal_refactorization_4x4` variant regresses the retained cap (2026-04-20)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, is there one
family-local diagonal restriction that keeps `diagonal_refactorization_4x4`
alive enough to justify staying on the `nw7` track without reopening broader
same-dimension work?

This round stayed inside the requested slice:

- one diagonal-focused hypothesis only;
- no beam-order retune;
- no weighted `4x4 -> 3` reopening;
- no generic orbit/certificate work; and
- one retained-lane measurement on the kept
  `beam256 + lag40 + dim4 + entry12` surface.

## Hypothesis

Restrict `diagonal_refactorization_4x4` to the narrowest nontrivial binary
diagonals:

- keep only single-doubled diagonals `[2,1,1,1]` up to permutation;
- drop the other binary non-scalar diagonal patterns such as
  `[2,2,1,1]`, `[2,2,2,1]`, and `[2,2,2,2]`;
- keep both row-divide and column-divide witnesses; and
- leave the rest of the retained lane unchanged.

Why this was chosen now:

- the earlier note
  [2026-04-20-no-clean-next-exact-family-after-bounded-orbit-seams.md](2026-04-20-no-clean-next-exact-family-after-bounded-orbit-seams.md)
  identified `diagonal_refactorization_4x4` as the remaining tempting
  same-dimension family on this lane;
- a fresh retained-case baseline run on the current branch showed that the
  family is genuinely active rather than inert:
  `57585` generated, `51031` kept after pruning, `35778` discovered,
  `37` approximate hits; and
- among the allowed diagonal-focused seams, "single doubled lane only" is the
  cheapest family-local restriction: it keeps the diagonal idea intact while
  shrinking the binary diagonal vocabulary from `14` non-scalar patterns to `4`.

## Temporary Slice

Temporary implementation under test:

- narrow the `4x4` diagonal-family gate to accept only diagonals with exactly
  one doubled entry;
- narrow the enumerator to emit only those same witnesses;
- update the diagonal-family focused tests to use a single-doubled witness; and
- keep all other families and ordering unchanged.

Files touched temporarily during the probe:

- `src/factorisation.rs`
- `src/search.rs`

This code was measured and then reverted. The final worktree keeps only this
note and the local measurement artifacts.

## Measurement Surface

Retained lane and fixed control:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- retained surface: `beam256 + lag40 + dim4 + entry12`

Local artifacts:

- single-case corpus:
  `tmp/sse_rust_6u1_single_case_baseline_2026-04-20.json`
- fresh baseline run on the current branch:
  `tmp/sse_rust_6u1_baseline_result_2026-04-20.json`
- retained-case run with the temporary single-doubled restriction:
  `tmp/sse_rust_6u1_after_single_doubled_diagonal_variant_2026-04-20.json`

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
path = Path('tmp/sse_rust_6u1_single_case_baseline_2026-04-20.json')
path.write_text(json.dumps(out, indent=2) + '\n')
print(path)
PY

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

timeout -k 20s 60s target/debug/research_harness \
  --cases tmp/sse_rust_6u1_single_case_baseline_2026-04-20.json \
  --format json \
  > tmp/sse_rust_6u1_baseline_result_2026-04-20.json

timeout -k 20s 180s cargo test -p sse-core --lib \
  test_graph_plus_structured_policy_exposes_diagonal_refactorization_4x4_witness \
  -- --test-threads=1

timeout -k 20s 180s cargo test -p sse-core --lib \
  test_diagonal_refactorizations_4x4_reach_expected_target \
  -- --test-threads=1

timeout -k 20s 180s cargo test -p sse-core --lib \
  test_expand_frontier_layer_graph_plus_structured_exposes_diagonal_refactorization_4x4 \
  -- --test-threads=1

timeout -k 20s 60s target/debug/research_harness \
  --cases tmp/sse_rust_6u1_single_case_baseline_2026-04-20.json \
  --format json \
  > tmp/sse_rust_6u1_after_single_doubled_diagonal_variant_2026-04-20.json

cargo fmt --all
```

## Results

### Fresh retained-case baseline

Current-branch retained baseline on the fixed surface:

| Field | Baseline |
| --- | --- |
| actual outcome | `unknown` |
| elapsed | `23604 ms` |
| frontier expanded | `19970` |
| factorisations | `487699` |
| candidates after pruning | `271803` |
| discovered nodes | `176662` |
| approximate hits | `184` |
| visited | `176664` |
| terminal bottleneck | `factorisation_volume` |
| focus progress score | `87050000` |
| directed progress score | `19783000` |

Family-local baseline telemetry for `diagonal_refactorization_4x4`:

- `candidates_generated = 57585`
- `candidates_after_pruning = 51031`
- `discovered_nodes = 35778`
- `approximate_other_side_hits = 37`

So the family is active enough that a family-local restriction is a real probe,
not a no-op.

### After the single-doubled restriction

The temporary restriction regressed the retained case from `unknown` to
`timeout` at the native cap:

| Field | Baseline | After single-doubled restriction |
| --- | --- | --- |
| actual outcome | `unknown` | `timeout` |
| elapsed | `23604 ms` | `24039 ms` |
| timeout cap | `24000 ms` | `24000 ms` |
| reason | none | `worker exceeded 24000 ms` |

Because the run timed out at the case cap, the post-change artifact did not
preserve productive search telemetry:

- `frontier_layers = 0`
- `frontier_nodes_expanded = 0`
- `factorisations_enumerated = 0`
- `focus_progress_score = 0`
- `directed_progress_score = 0`

Interpretation:

- despite shrinking the diagonal vocabulary, this variant did not behave like a
  harmless cost cut on the retained hard case;
- under the only measurement that matters here, it regressed the lane from a
  bounded `unknown` finish to a hard timeout; and
- because the request was for exactly one retained-lane probe, there is no
  second diagonal hypothesis in this round.

## Validation

Focused temporary checks run before the retained-lane measurement:

- `test_graph_plus_structured_policy_exposes_diagonal_refactorization_4x4_witness`
- `test_diagonal_refactorizations_4x4_reach_expected_target`
- `test_expand_frontier_layer_graph_plus_structured_exposes_diagonal_refactorization_4x4`

Observed result:

- all three focused tests passed under the temporary code before it was
  reverted

Formatter for the final reverted worktree:

```bash
cargo fmt --all
```

## Decision

Decision: **reject**

Reason:

- this is a concrete diagonal-focused family-local restriction on the retained
  `graph_plus_structured` lane;
- the fresh baseline confirmed that `diagonal_refactorization_4x4` is active,
  so the probe was meaningful; but
- at the actual retained `beam256 + lag40 + dim4 + entry12` cap the restricted
  variant regressed the case from `unknown` to `timeout`.

Durable conclusion:

- do not keep the single-doubled-only `diagonal_refactorization_4x4` variant
  on the retained Brix-Ruiz `k=4` `graph_plus_structured` lane; and
- this family does not currently clear the bar for a kept `nw7` diagonal seam
  under the native retained cap.
