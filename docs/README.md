# Documentation Map

This repo now uses three documentation layers with different jobs:

- [`../README.md`](../README.md) is the entry point for the project itself:
  problem statement, research context, capabilities, and build/deploy notes.
- [`TODO.md`](TODO.md) is roadmap context for solver and search work.
  It is not the live task list; use `bd` for actionable backlog items.
- [`../research/README.md`](../research/README.md) describes the experiment
  workflow, logs, notes, and local run artifacts.

## Topic Notes

- [`../TERMINOLOGY.md`](../TERMINOLOGY.md) is the repo-wide vocabulary file.
  RFCs and topic notes should update it when they intentionally change or
  sharpen shared language.
- [`aligned-shift-equivalence.md`](aligned-shift-equivalence.md) records the
  current concrete-shift surface in `src/concrete_shift.rs`, including
  aligned, balanced concrete shift, and compatible concrete-shift witnesses,
  plus the terminology caveats around older aligned-oriented public names. It
  is not a rollout checklist.
- [`2x2-positive-classifier.md`](2x2-positive-classifier.md) records the
  narrow exact Baker/Choe-Shin positive `2x2` dossier slice and the difference
  between determinant territory and the stricter theorem-backed pair class.
- [`search-parallelism-5b8.md`](search-parallelism-5b8.md) records the measured
  layer-timing breakdown for endpoint search and why dedup, not merge/commit,
  is the first plausible deeper-parallelism target.
- [`search-parallelism-8h4.md`](search-parallelism-8h4.md) turns that timing
  evidence into a concrete determinism, memory, and correctness risk assessment
  for future deeper parallelism work in `src/search.rs`.
- [`graph-proposal-shortlist-rounds.md`](graph-proposal-shortlist-rounds.md)
  standardizes the default bounded graph-proposal shortlist evidence round,
  including the runner, output fields, and keep/revert interpretation.
- [`autoresearch-round-scorecards.md`](autoresearch-round-scorecards.md)
  standardizes the default useful-reach and budget scorecards for common
  autoresearch round types so workers can cite a reusable keep/revert contract.
- [`research-ideas.md`](research-ideas.md) is the long-horizon idea bank from
  paper reading and code review. It should collect plausible directions and
  synthesis, not a ranked backlog.
- [`brix-ruiz-sidecar-log.md`](brix-ruiz-sidecar-log.md) is the family-specific
  experimental record for Brix-Ruiz sidecar work.
- [`rfcs/rfc-001-main-search-shortcut-integration.md`](rfcs/rfc-001-main-search-shortcut-integration.md)
  proposes promoting refinement and shortcutting into the main solver/CLI so
  the hard `k = 3` and `k >= 4` search cases become first-class product
  behavior rather than sidecar-only workflows.
- [`rfcs/rfc-002-shortcut-search-stage.md`](rfcs/rfc-002-shortcut-search-stage.md)
  proposes the missing generic `shortcut_search` stage as an artifact-driven
  outer loop built on top of `guided_refinement`.
- [`rfcs/rfc-003-structured-witness-vocabulary.md`](rfcs/rfc-003-structured-witness-vocabulary.md)
  records the accepted shared vocabulary for the repo's concrete-shift,
  balanced elementary equivalence, and sampled positive-conjugacy surfaces
  while keeping their proof semantics distinct.
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
