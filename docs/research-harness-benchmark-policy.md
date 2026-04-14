# Research Harness Benchmark Policy

## Decision (2026-04-14)

Benchmark-style measurements should be expressible through
`research_harness`, but only as **non-gating** cases.

Concretely:

- use harness cases for scenario-level measurements tied to real endpoint
  families, stage configs, and guide artifacts,
- mark those cases with `"required": false` so they do not affect required-pass
  correctness gates,
- keep their scoring neutral (`target_outcome: null`, points typically `0`).

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

## Boundary

Use `research_harness` for scenario-level measurements (search policies,
move-policy/frontier-policy combinations, staged shortcut settings).

Use dedicated bench surfaces for microbench/per-function timing where repeated,
low-noise measurement and statistical summaries are the main goal.

## Follow-Up Plan

Deferred implementation work:

1. add optional harness-level repeat/warmup controls for non-required cases,
2. add summary stats (median/p90) for repeated measurement cases,
3. optionally add a `measurement` block in case schema to formalize these knobs.

Until then, keep measurement cases bounded and explicit, and rely on saved JSON
artifacts for trend comparison.
