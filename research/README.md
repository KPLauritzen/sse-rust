# Research Workspace

This directory holds the research workflow and lab notebook material for the
repo. It is not the active backlog; use `bd` for actionable tasks.

## What Lives Here

- [`program.md`](program.md) defines the autonomous experiment loop and the
  constraints around the harness.
- [`cases.json`](cases.json) is the ground-truth benchmark set for
  `research_harness`.
  Required cases (`"required": true`, default) gate correctness; non-required
  cases (`"required": false`) are for benchmark-style measurement probes or
  evidence-only diagnostic campaigns.
- [`log.md`](log.md) is the terse chronological ledger. Keep entries short.
- [`notes/README.md`](notes/README.md) defines the structure for longer-form
  notes that do not fit cleanly in the log.

Case metadata conventions:

- use `measurement` on non-required cases when repeated timing matters;
- use `deepening_schedule` on non-required cases when a bounded lag/dimension
  or entry ramp is more informative than isolated one-off probes;
- keep the distinction clear between required gates, measurement probes, and
  evidence campaigns.

Local artifacts:

- `research/runs/` is for machine-readable harness outputs such as
  `just research-json-save <stamp>`.
- `research/results.tsv` is the local score table for experiment history.

The harness can reuse prior saved JSON runs when comparing best-known witness
lags and strategy summaries:

```sh
cargo run --profile dist --features research-tools --bin research_harness -- \
  --cases research/cases.json \
  --reuse-dir research/runs \
  --format pretty
```

Those local artifacts should stay untracked unless the human explicitly asks
for repository changes to that policy.

Benchmark-style case policy is documented in
[`../docs/research-harness-benchmark-policy.md`](../docs/research-harness-benchmark-policy.md).

When a run emits JSON, each case result includes a case-level summary plus
`result_model`, aggregate telemetry, and per-layer telemetry. Use that shape to
separate correctness, runtime, and search-shape effects when deciding whether a
change is worth keeping.

## Supported Entry Points

For active work, use:

- `cargo run --bin search -- ...` for direct solver runs and the generic
  staged solver surface,
- `cargo run --features research-tools --bin research_harness -- ...` for
  fixture-backed benchmark and campaign runs.

The remaining `research-tools` binaries are intentionally narrower. Keep them
for targeted diagnostics or paper reproduction, not as alternate front doors
for the same Brix-Ruiz search flows. In Phase 6, the older
`brix_ruiz_k3`, `find_brix_ruiz_graph_path`, and
`find_brix_ruiz_path_shortcuts` sidecars were retired from Cargo targets after
their overlap was absorbed by `search` plus `research_harness`.

## Note-Taking Convention

When work produces new evidence:

1. Add a short entry to [`log.md`](log.md) with the commit hash or run stamp.
2. If the reasoning needs more than a few lines, create or update a note in
   `research/notes/`.
3. In longer notes, record the question, the evidence, the conclusion, and the
   next steps so later work can reuse the result without replaying the whole
   investigation.
