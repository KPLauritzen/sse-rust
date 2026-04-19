# Brix-Ruiz `k=4` graph-plus-structured partition-refined tie-break regressed the retained lane (2026-04-19)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, can the same
`beam256 + dim4 + entry12` budget be spent better by changing only one
non-dimension beam-quality seam inside beam ordering:

- keep approximate-hit priority unchanged;
- keep the existing `beam_default_low` score unchanged; but
- when two beam entries tie on the current score, prefer the smaller
  partition-refined same-future/same-past quotient gap before falling back to
  depth and insertion order.

This stays inside the requested lane-local scope:

- no move-family broadening;
- no generic solver rewrite;
- no beam-direction retune; and
- no revisit of the rejected binary-sparse orbit-dedup or dimension-gap
  tie-break hypotheses.

## Hypothesis

This looked like the strongest fresh non-dimension ranking seam available:

- prior analysis-only notes already showed the partition-refined quotient signal
  was directionally useful as a bounded sidecar signal;
- unlike `signature_distance`, it is not a disguised dimension-pressure tie-break;
- the beam executor rounds `score_node()` into coarse `i64` buckets, so an
  equal-score tie-break can matter without replacing the existing score.

## Attempted Slice

Temporary code change, measured and then reverted:

- in [`src/search/beam.rs`](../../src/search/beam.rs), extend
  `BeamFrontierEntry` with the partition-refined quotient gap to the endpoint;
- keep `approximate_hit` and `score` ordering unchanged;
- after score ties, compare entries by the smaller partition-refined gap before
  depth and serial order.

The final worktree does **not** keep this beam-order change.

## Measurement Surface

Retained lane:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- bounds: `beam256 + dim4 + entry12`
- retained cases: `lag20`, `lag30`, `lag40`

Baseline for comparison:

- kept split-family baseline note:
  [`2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-3x3-to-4x4-split-family-cut.md`](./2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-3x3-to-4x4-split-family-cut.md)
- baseline retained outcomes there:
  `lag20 = unknown`, `lag30 = timeout`, `lag40 = unknown`

Local artifacts:

- retained three-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_corpus_2026-04-19.json`
- measured temporary run:
  `tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_partition_refined_tiebreak_run_2026-04-19.json`

Exact commands:

```bash
python - <<'PY'
import json
from pathlib import Path
src = json.loads(Path('research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json').read_text())
keep = {
    'brix_ruiz_k4__graph_plus_structured__beam256_lag20_dim4_entry12',
    'brix_ruiz_k4__graph_plus_structured__beam256_lag30_dim4_entry12',
    'brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12',
}
out = {
    'schema_version': src.get('schema_version', 1),
    'cases': [case for case in src['cases'] if case['id'] in keep],
}
Path('tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_corpus_2026-04-19.json').write_text(
    json.dumps(out, indent=2) + '\n'
)
PY

timeout -k 20s 180s cargo build --quiet --features research-tools --bin research_harness

# Run while the temporary partition-refined tie-break patch is applied.
timeout -k 20s 120s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_corpus_2026-04-19.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_partition_refined_tiebreak_run_2026-04-19.json
```

## Results

Compared with the kept retained-lane baseline:

| Case | Baseline outcome | Temporary tie-break outcome | Delta |
| --- | --- | --- | --- |
| `beam256 + lag20 + dim4 + entry12` | `unknown` | `timeout` | regression |
| `beam256 + lag30 + dim4 + entry12` | `timeout` | `timeout` | no improvement |
| `beam256 + lag40 + dim4 + entry12` | `unknown` | `timeout` | regression |

Observed measurement detail:

- all three retained cases exited via the harness worker timeout
  (`15000 / 18000 / 24000 ms`);
- the timeout results therefore surfaced with `telemetry.layers = []` and
  `terminal_bottleneck = no_search`, so there is no useful partial frontier
  breakdown inside this run artifact;
- even with that limitation, the retained-lane decision is still clear because
  the temporary tie-break loses the two cases that were previously bounded as
  `unknown`.

Interpretation:

- the partition-refined quotient tie-break does not buy usable same-budget beam
  quality on the retained open Brix-Ruiz `k=4` surface;
- on this lane it is more likely to destabilize or slow equal-score ordering
  than to improve witness reach.

## Validation

Focused validation before measurement:

```bash
timeout -k 20s 120s cargo test -p sse-core --lib \
  test_beam_frontier_prefers_lower_refined_gap_on_score_ties -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_beam_direction_prefers_approximate_hits -- --test-threads=1
```

Observed result:

- both focused tests passed

Formatter before commit:

```bash
timeout -k 20s 120s cargo fmt --all
```

## Decision

Decision: **reject**

Reason:

- the one bounded retained-lane measurement pass regressed the retained surface
  from `unknown/timeout/unknown` to `timeout/timeout/timeout`; and
- that is enough evidence to keep this partition-refined quotient signal out of
  default beam ordering on the open Brix-Ruiz `k=4` lane.

Durable conclusion:

- do **not** spend the retained `beam256 + dim4 + entry12`
  `graph_plus_structured` budget on this specific non-dimension tie-break;
- if another beam-quality seam is tested here, it should be a different
  non-dimension ordering signal rather than this partition-refined quotient
  fallback.
