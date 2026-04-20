# Retained Riedel/Baker `k = 4` wider-envelope full graph-only replay control (2026-04-20)

## Goal

Freeze exactly one durable reusable control for the already-solved
Riedel/Baker `k = 4` wider `graph_only` existence result from
[`2026-04-18-riedel-k4-full-graph-decomposition.md`](./2026-04-18-riedel-k4-full-graph-decomposition.md),
without rebuilding the decomposition ladder and without presenting the result
as progress on the still-open Brix-Ruiz Goal 3 lane.

## Chosen surface

Keep one committed harness worker-case only:

- `riedel_k4_graph_only_full_decomposition_guided_replay`

This surface is intentionally guide-backed rather than plain endpoint search.
The purpose is to preserve the known wider-envelope existence witness in a form
that later workers can replay directly, compare against retained obstructions,
and cite without reconstructing the sidecar decomposition from scratch.

## Frozen control

The control reuses the existing retained full-path artifact:

- [`research/riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json`](../riedel_k4_graph_only_full_decomposition_guide_2026-04-18.json)

on the exact wider bounded envelope already recorded on 2026-04-18:

- endpoints `[[4,2],[1,4]] -> [[3,1],[1,5]]`
- `move_family_policy = graph-only`
- `stage = guided_refinement`
- `max_lag = 19`
- `max_intermediate_dim = 5`
- `max_entry = 12`

The retained artifact itself already carries the stronger explicit witness:

- `validation = witness_validated`
- guide quality lag `15`

The control keeps the envelope at `lag <= 19` because that is the bounded
existence surface established in the original note. This avoids overstating the
result as a new shortest-path claim while still freezing the stronger retained
guide for direct replay.

## Why this surface

Keep:

- one durable solved-control surface for known wider-envelope graph-only
  existence on Riedel/Baker `k = 4`;
- the existing committed full graph-only guide artifact as the witness anchor;
- a harness-facing worker-case so later workers can validate the control with a
  single command.

Reject:

- broad graph-only ladder remaps;
- new search-policy or tooling framework work; and
- any framing that treats this solved Riedel/Baker control as Brix-Ruiz Goal 3
  progress.

## Reproduce / Validate

Build the focused binary:

```bash
cargo build --profile dist --features research-tools --bin research_harness
```

Replay and validate the kept control directly:

```bash
timeout -k 5s 20s target/dist/research_harness \
  --cases research/cases.json \
  --worker-case riedel_k4_graph_only_full_decomposition_guided_replay
```

Observed reading on current head:

- outcome `equivalent`
- the run accepts the committed `2026-04-18` full-path guide artifact and
  replays the solved witness on the wider retained envelope

## Reuse guidance

Use this control when you need one stable answer to:

- "Is there already a retained wider-envelope `graph_only` existence witness
  for the solved Riedel/Baker `k = 4` pair?"

Do not use it to claim:

- a new plain-search breakthrough on the open Brix-Ruiz `k = 4` lane; or
- a new shortestness/minimality result for the Riedel/Baker witness.
