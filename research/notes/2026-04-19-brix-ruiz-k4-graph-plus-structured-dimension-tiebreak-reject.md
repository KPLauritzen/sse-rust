# Brix-Ruiz `k=4` graph-plus-structured equal-score dimension tie-break regressed the retained lane (2026-04-19)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane, can the same
`beam256 + dim4 + entry12` budget be spent better by changing only one
ranking-quality seam inside beam ordering:

- keep the current `beam_default_low` score unchanged;
- keep approximate-hit priority unchanged; but
- when two beam entries tie on the current score, prefer the smaller endpoint
  dimension gap before falling back to insertion order.

This stays inside the requested lane-local scope:

- no move-family widening or pruning rewrite;
- no generic solver rewrite;
- no same-depth beam-direction retune; and
- no binary-sparse orbit-dedup revisit.

## Hypothesis

The retained dim4 lane already uses a structure-first beam score, but equal
score buckets currently fall back to serial order. A small tie-break in favor of
smaller endpoint dimension gap looked plausible because earlier ranking notes
showed dimension pressure can be informative without needing a full beam-score
replacement.

## Attempted Slice

Temporary code change, measured and then reverted:

- in `src/search/beam.rs`, after `approximate_hit` and `score`, compare beam
  entries by endpoint dimension gap before using `serial`

The final worktree does **not** keep this beam-order change.

## Measurement Surface

Retained lane:

- endpoint: open Brix-Ruiz `k=4`
- policy: `graph_plus_structured`
- bounds: `beam256 + dim4 + entry12`
- retained cases: `lag20`, `lag30`, `lag40`

Baseline for comparison:

- `research/notes/2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-3x3-to-4x4-split-family-cut.md`

Local artifacts:

- retained three-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_corpus_2026-04-19.json`
- measured temporary run:
  `tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_dim_tiebreak_run_2026-04-19.json`

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

# Run while the temporary equal-score dimension-gap tie-break patch is applied.
timeout -k 20s 120s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_corpus_2026-04-19.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_retained_beam256_dim4_entry12_dim_tiebreak_run_2026-04-19.json
```

## Results

Compared with the kept split-family baseline on the same retained lane:

| Case | Baseline outcome | Temporary tie-break outcome | Delta |
| --- | --- | --- | --- |
| `beam256 + lag20 + dim4 + entry12` | `unknown` | `unknown` | no material change |
| `beam256 + lag30 + dim4 + entry12` | `timeout` | `timeout` | no change |
| `beam256 + lag40 + dim4 + entry12` | `unknown` | `timeout` | regression |

What stayed flat at `lag20`:

- `frontier_nodes_expanded = 9,730`
- `factorisations_enumerated = 372,621`
- `approximate_other_side_hits = 126`
- `total_visited_nodes = 112,521`
- `focus_progress_score = 43,050,000`
- `directed_progress_score = 13,655,000`
- `terminal_bottleneck = factorisation_volume`

What changed:

- the tie-break did **not** improve the already-rankable retained case
  `beam256 + lag20 + dim4 + entry12`
- the harder retained case
  `beam256 + lag40 + dim4 + entry12` regressed from the baseline's bounded
  `unknown` to a full `timeout`

Important measurement detail:

- timeout cases in this harness surface do not publish partial frontier
  telemetry, so the `lag40` regression is visible as an outcome flip rather
  than as a partial before/after layer table

## Validation

Focused validation before measurement:

```bash
timeout -k 20s 120s cargo test -p sse-core --lib \
  test_beam_frontier_enforces_width_cap -- --test-threads=1

timeout -k 20s 120s cargo test -p sse-core --lib \
  test_beam_direction_prefers_approximate_hits -- --test-threads=1
```

Tooling compatibility fix needed for the bounded scoring probe:

```bash
timeout -k 20s 180s cargo build --quiet --features research-tools --bin analyze_path_signal_corpus
timeout -k 20s 120s cargo test --quiet --features research-tools --bin analyze_path_signal_corpus
```

Formatter before commit:

```bash
timeout -k 20s 120s cargo fmt --all
```

## Decision

Decision: **reject**

Reason:

- the equal-score dimension-gap tie-break does not improve the retained lane's
  already-bounded `lag20` surface at all; and
- it regresses the key retained hard case from `unknown` to `timeout` at
  `beam256 + lag40 + dim4 + entry12`

Durable conclusion:

- do **not** spend the retained open Brix-Ruiz `k=4` `graph_plus_structured`
  budget on this specific equal-score dimension-pressure seam
- if another ranking-quality slice is tried on this lane, it should look at a
  different tie-break or beam-quality signal rather than this dimension-gap
  fallback
