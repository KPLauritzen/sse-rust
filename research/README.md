# Research Workspace

This directory holds the research workflow and lab notebook material for the
repo. It is not the active backlog; use `bd` for actionable tasks.

## What Lives Here

- [`program.md`](program.md) defines the autonomous experiment loop and the
  constraints around the harness.
- [`cases.json`](cases.json) is the ground-truth benchmark set for
  `research_harness`.
- [`log.md`](log.md) is the terse chronological ledger. Keep entries short.
- [`notes/README.md`](notes/README.md) defines the structure for longer-form
  notes that do not fit cleanly in the log.

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

## Note-Taking Convention

When work produces new evidence:

1. Add a short entry to [`log.md`](log.md) with the commit hash or run stamp.
2. If the reasoning needs more than a few lines, create or update a note in
   `research/notes/`.
3. In longer notes, record the question, the evidence, the conclusion, and the
   next steps so later work can reuse the result without replaying the whole
   investigation.
