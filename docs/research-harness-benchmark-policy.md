# Research Harness Benchmark Policy

## Decision (2026-04-14)

Benchmark-style measurements should be expressible through
`research_harness`, but only as **non-gating** cases.

Concretely:

- use harness cases for scenario-level measurements tied to real endpoint
  families, stage configs, and guide artifacts,
- mark those cases with `"required": false` so they do not affect required-pass
  correctness gates,
- keep their scoring neutral (`target_outcome: null`, points typically `0`),
- optionally add a per-case `measurement` block when warmup/repeat timing is
  needed:

```json
"measurement": {
  "warmup_runs": 1,
  "repeat_runs": 5
}
```

## Why

Pros:

- single orchestration surface for correctness and measurement probes,
- shared telemetry schema, campaign grouping, and artifact reuse,
- easier A/B comparisons on the exact same endpoint fixtures.

Risks if unmanaged:

- noisy runtime probes can destabilize pass/fail expectations,
- "benchmark" pressure can bias loop decisions away from correctness,
- repeated timing runs can inflate harness runtime significantly.

The `required=false` split keeps correctness gates stable while preserving a
standard measurement lane in the same harness report.

For repeated measurement cases, `elapsed_ms` in the case summary is the
representative median repeat sample rather than the total wall time spent
running warmups/repeats. The detailed measurement block in JSON/pretty output
includes the repeat samples plus `median` and `p90` so noise can be compared
without distorting harness fitness tie-break semantics.

## Boundary

Use `research_harness` for scenario-level measurements (search policies,
move-policy/frontier-policy combinations, staged shortcut settings).

Use dedicated bench surfaces for microbench/per-function timing where repeated,
low-noise measurement and statistical summaries are the main goal.

Current `benches/search.rs` coverage is intentionally micro/throughput oriented:
fast endpoint sanity checks plus telemetry-driven `expand_next_n` expansion
throughput probes. Keep heavier scenario-family runs in `research_harness`.

For Criterion runs in `benches/search.rs`, keep baseline usage explicit:

- create baseline: `just bench-search-save-baseline <name>` (or `cargo bench --bench search -- --save-baseline <name>`),
- compare baseline: `just bench-search-compare-baseline <name>` (or `cargo bench --bench search -- --baseline <name>`).

Use plain `cargo bench` / `just bench-search` only for local sanity runs where
no benchmark delta is being claimed. If a result is used for a keep/revert
decision or reported as a regression/speedup, compare against a named baseline.

## Guardrails

- `measurement` is only valid on `required=false` cases. Required cases remain
  single-run correctness gates.
- Warmup runs are excluded from reported elapsed samples.
- Repeated measurement is for scenario-level harness probes; keep dedicated
  bench surfaces for microbench-style statistical work.
