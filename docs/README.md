# Documentation Map

This repo now uses three documentation layers with different jobs:

- [`../README.md`](../README.md) is the entry point for the project itself:
  problem statement, research context, capabilities, and build/deploy notes.
- [`TODO.md`](TODO.md) is roadmap context for solver and search work.
  It is not the live task list; use `bd` for actionable backlog items.
- [`../research/README.md`](../research/README.md) describes the experiment
  workflow, logs, notes, and local run artifacts.

## Topic Notes

- [`aligned-shift-equivalence.md`](aligned-shift-equivalence.md) tracks the
  current status of aligned, balanced, and compatible concrete-shift work.
- [`research-ideas.md`](research-ideas.md) is the long-horizon idea bank from
  paper reading and code review. It should collect plausible directions, not
  act as a checklist.
- [`brix-ruiz-sidecar-log.md`](brix-ruiz-sidecar-log.md) is the family-specific
  experimental record for Brix-Ruiz sidecar work.
- [`rfcs/rfc-001-main-search-shortcut-integration.md`](rfcs/rfc-001-main-search-shortcut-integration.md)
  proposes promoting refinement and shortcutting into the main solver/CLI so
  the hard `k = 3` and `k >= 4` search cases become first-class product
  behavior rather than sidecar-only workflows.
- [`rfcs/rfc-002-shortcut-search-stage.md`](rfcs/rfc-002-shortcut-search-stage.md)
  proposes the missing generic `shortcut_search` stage as an artifact-driven
  outer loop built on top of `guided_refinement`.
- [`research-harness-benchmark-policy.md`](research-harness-benchmark-policy.md)
  defines how benchmark-style measurement probes should be represented through
  `research_harness` without weakening required-case correctness gates.

## Rules Of Thumb

- Put active work items in `bd`, not in new markdown TODO lists.
- Put short chronological experiment entries in
  [`../research/log.md`](../research/log.md).
- Put longer literature notes, experiment dossiers, and synthesis writeups in
  `research/notes/`.
- Keep generated run artifacts local in `research/runs/` and the local score
  table in `research/results.tsv`.
