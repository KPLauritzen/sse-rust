# k=3 quotient-retained guide-pool A/B for shortcut-search input (2026-04-16)

## Question

The previous quotient report showed that the current normalized `k=3` pool
collapses from `12` stored guides to `5` quotient classes. The next narrow
question was whether a quotient-retained pool is actually a better
`shortcut_search` input surface, not just a smaller report.

This note stays bounded to guide-pool preparation/reporting:

- no default search behavior changes;
- no promotion of local quotient rewrites into solver search;
- use the existing bounded `shortcut_search` probe on
  `brix_ruiz_k3` as the comparison surface.

## Materialization Surface

I extended `analyze_guide_pool_quotient` with
`--retained-guide-artifacts-out PATH`.

The materialized output is intentionally conservative:

- it does **not** emit canonicalized full-path rewrites as reusable
  `full_path` artifacts, because the current quotient seam only proves matrix
  sequence equivalence, not reconstructed witness steps;
- instead it writes one existing validated witness artifact per quotient class,
  choosing the shortest available source witness in that class;
- the output envelope keeps normal `artifacts` for search consumption and adds
  `quotient_materialization` metadata with the quotient analysis and canonical
  matrix representatives.

Durable outputs:

- retained guide-pool artifact:
  `research/guide_artifacts/k3_quotient_retained_guide_pool.json`
- quotient analysis backing the materialization:
  `research/runs/2026-04-16-k3-quotient-retained-guide-pool-analysis.json`
- bounded shortcut-search A/B summary:
  `research/runs/2026-04-16-k3-quotient-retained-shortcut-ab.json`

Retained source witnesses written to the pool:

- `k3-lind-marcus-baker-lag7`
- `k3-sqlite-shortcut-7`
- `k3-sqlite-shortcut-9`
- `k3-sqlite-graph-2`
- `k3-seeded-endpoint-16-path`

Important nuance:

- two quotient classes still require long retained witnesses (`lag 30` and
  `lag 31`) because the quotient seam shortens their canonical matrix paths to
  `27` and `28`, but we do not yet reconstruct new validated step witnesses;
- one merged shortcut class canonicalizes to lag `7`, but its shortest existing
  witness is still `k3-sqlite-shortcut-9` at lag `8`.

## Commands

Materialize the retained pool and write the backing analysis:

```bash
timeout 180 cargo run --quiet --features research-tools --bin analyze_guide_pool_quotient -- \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-suffix-lag 4 --max-rewrite-states 1024 --max-samples 12 \
  --json-out research/runs/2026-04-16-k3-quotient-retained-guide-pool-analysis.json \
  --retained-guide-artifacts-out research/guide_artifacts/k3_quotient_retained_guide_pool.json
```

Run the bounded normalized-vs-quotient shortcut-search A/B:

```bash
python - <<'PY'
import json
from pathlib import Path
src = Path("research/cases.json")
out = Path("tmp/2026-04-16-k3-quotient-ab-cases.json")
want = {
    "brix_ruiz_k3_shortcut_normalized_pool_probe",
    "brix_ruiz_k3_shortcut_quotient_retained_pool_probe",
}
corpus = json.loads(src.read_text())
selected = [case for case in corpus["cases"] if case["id"] in want]
out.write_text(json.dumps({"schema_version": corpus["schema_version"], "cases": selected}, indent=2) + "\n")
PY

timeout 120 cargo run --quiet --features research-tools --bin research_harness -- \
  --cases tmp/2026-04-16-k3-quotient-ab-cases.json --format json \
  > research/runs/2026-04-16-k3-quotient-retained-shortcut-ab.json
```

## Results

### Pool preparation

From `research/runs/2026-04-16-k3-quotient-retained-guide-pool-analysis.json`:

- source guides: `12`
- quotient-retained classes: `5`
- removed by quotient retention: `7`
- retained canonical lag total: `76`
- retained canonical matrix total: `81`

Shortcut-search input surface after materialization:

- normalized pool accepted by `shortcut_search`: `12` guides
- quotient-retained pool accepted by `shortcut_search`: `5` guides
- initial working set under `max_guides=6`: `6 -> 5`

### Bounded A/B on `brix_ruiz_k3`

Both cases used the same existing measurement probe:

- `max_lag=6`
- `max_intermediate_dim=4`
- `max_entry=5`
- `move_family_policy=mixed`
- stage `shortcut_search`
- `max_guides=6`
- `rounds=1`
- `max_total_segment_attempts=48`
- measurement `warmup_runs=1`, `repeat_runs=5`

Normalized pool (`brix_ruiz_k3_shortcut_normalized_pool_probe`):

- outcomes: `5 / 5 equivalent`
- elapsed samples ms: `2560, 2565, 2568, 2594, 2609`
- median elapsed: `2568 ms`
- accepted guides / unique guides / initial working set: `12 / 12 / 6`
- segment attempts / improvements / promoted guides: `48 / 2 / 1`
- best lag: `7 -> 7`
- stop reason: `max_segment_attempts_reached`

Quotient-retained pool (`brix_ruiz_k3_shortcut_quotient_retained_pool_probe`):

- outcomes: `5 / 5 equivalent`
- elapsed samples ms: `2570, 2582, 2589, 2594, 2601`
- median elapsed: `2589 ms`
- accepted guides / unique guides / initial working set: `5 / 5 / 5`
- segment attempts / improvements / promoted guides: `48 / 2 / 1`
- best lag: `7 -> 7`
- stop reason: `max_segment_attempts_reached`

Delta on this bounded surface:

- median elapsed worsened slightly: `2568 -> 2589 ms` (`+21 ms`, about `+0.8%`)
- the search outcome stayed identical (`lag 7`)
- round/segment telemetry stayed identical apart from the smaller prepared pool
  counts

## Conclusion

The quotient-retained pool is useful as a **research artifact** and as a
cleaner input-preparation report, but it does **not** improve the current
bounded `shortcut_search` surface.

On the established `brix_ruiz_k3` shortcut probe, reducing the pool from
`12` guides to `5` representatives kept the exact same lag-7 outcome and
segment-level behavior, while running slightly slower on the median measured
time. On this evidence, the quotient-retained pool should stay research-only
for preparation/reporting and should not replace the current normalized pool as
the default shortcut-search input.
