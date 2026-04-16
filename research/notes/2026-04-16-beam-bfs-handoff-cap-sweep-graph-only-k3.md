# Bounded deferred-cap sweep on graph-only `brix_ruiz_k3`

## Question

On the existing graph-only `brix_ruiz_k3` frontier measurement surface, do
small `beam_bfs_handoff_deferred_cap` settings around the current beam width
produce a meaningfully better handoff measurement case than the historical
depth-`4` baseline, judged against plain beam with the useful-reach/budget
contract from `research/program.md`?

Fixed control:

- endpoint fixture: `research/fixtures/brix_ruiz_family.json#brix_ruiz_k3`
- source: `[[1,3],[2,1]]`
- target: `[[1,6],[1,1]]`
- move family: `graph-only`
- bounds: `max_lag = 10`, `max_intermediate_dim = 5`, `max_entry = 6`
- frontier beam width: `10`
- handoff depth: `4`

Round type and lane:

- widening/control probe in the measurement lane
- authoritative useful-reach fields:
  `collisions_with_other_frontier`, `approximate_other_side_hits`,
  `discovered_nodes`
- authoritative budget fields:
  `elapsed_ms`, `frontier_nodes_expanded`, `total_visited_nodes`,
  `max_frontier_size`

## Probe

First pass on the rebuilt `dist` binary, with the same `5s` budget used by the
current frontier probes:

```bash
target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam \
  --beam-width 10 \
  --json --telemetry

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 4 \
  --json --telemetry

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 4 \
  --beam-bfs-handoff-deferred-cap 5 \
  --json --telemetry

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 4 \
  --beam-bfs-handoff-deferred-cap 10 \
  --json --telemetry

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 4 \
  --beam-bfs-handoff-deferred-cap 20 \
  --json --telemetry
```

Artifacts were saved under `tmp/`:

- `tmp/oaj-dist-beam-control.json`
- `tmp/oaj-dist-handoff-depth4.json`
- `tmp/oaj-dist-handoff-cap5.json`
- `tmp/oaj-dist-handoff-cap10.json`
- `tmp/oaj-dist-handoff-cap20.json`

## Results

### Plain beam control

- outcome: `unknown`
- wall time: `0.07s`
- `collisions_with_other_frontier = 0`
- `approximate_other_side_hits = 0`
- `discovered_nodes = 5370`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5372`
- `max_frontier_size = 10`

### Historical handoff baseline and cap variants

- historical depth-`4` handoff: timed out at `5.06s`; no JSON emitted
- `deferred_cap = 5`: timed out at `5.05s`; no JSON emitted
- `deferred_cap = 10`: timed out at `5.04s`; no JSON emitted
- `deferred_cap = 20`: timed out at `5.04s`; no JSON emitted

None of the capped runs produced final telemetry, so there is no useful-reach
ledger to compare against plain beam beyond the negative fact that they failed
to stay inside the same bounded budget where plain beam remained cheap and
stable.

## Decision

No deferred-cap setting in this sweep is a meaningfully better measurement
surface than the current historical handoff baseline.

Why:

- against the budget ledger, all three cap variants were still as bad as the
  historical losing control at the shared `5s` bound;
- against the useful-reach ledger, none of them emitted final telemetry, so
  they do not yet support a stable apples-to-apples frontier comparison;
- against plain beam, the control remains dramatically cheaper while preserving
  the only comparable reach numbers on this surface.

This reproduces the negative direction already flagged in
`research/notes/2026-04-16-measurement-corpus-baseline-audit.md` on a freshly
rebuilt `dist` binary. There is still no evidence that beam-width-scale
deferred caps rescue the graph-only handoff surface.

## Follow-up

- Do not change `research/cases.json` from this sweep.
- Keep the existing plain-beam control and historical depth-`4` handoff case as
  the durable shared-corpus frontier comparison.
- If this line is revisited, do not touch `beam_bfs_handoff_depth` again first.
  The next bounded probe should move below beam width instead: try sub-beam
  caps such as `0`, `1`, `2`, and `3` at the same depth-`4` control, and only
  escalate to repeated harness measurement if one of those settings emits final
  JSON within the existing `5s` budget.
