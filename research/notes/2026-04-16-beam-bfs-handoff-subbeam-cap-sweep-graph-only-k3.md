# Bounded sub-beam deferred-cap sweep on graph-only `brix_ruiz_k3`

## Question

After the beam-width-scale deferred-cap sweep stayed negative, does fixing the
same graph-only `brix_ruiz_k3` handoff surface at `beam_width = 10` and
`beam_bfs_handoff_depth = 4` but moving the retained-overflow cap *below* beam
width produce a meaningfully better measurement surface than:

- plain `beam`, and
- the historical depth-`4` `beam_bfs_handoff` baseline?

Fixed control:

- endpoint fixture: `research/fixtures/brix_ruiz_family.json#brix_ruiz_k3`
- source: `[[1,3],[2,1]]`
- target: `[[1,6],[1,1]]`
- move family: `graph-only`
- bounds: `max_lag = 10`, `max_intermediate_dim = 5`, `max_entry = 6`
- frontier beam width: `10`
- handoff depth: `4`

Round type and lane:

- frontier comparison in the measurement/evidence lane
- authoritative useful-reach fields:
  `collisions_with_other_frontier`, `approximate_other_side_hits`,
  `discovered_nodes`
- authoritative budget fields:
  `elapsed_ms`, `frontier_nodes_expanded`, `total_visited_nodes`,
  `max_frontier_size`

## Probe

Rebuilt binary before measurement:

```bash
cargo build --profile dist --features research-tools --bin search
```

Bounded first pass on `target/dist/search`, using the same `5s` budget as the
existing frontier probes:

```bash
timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
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
  --beam-bfs-handoff-deferred-cap 0 \
  --json --telemetry

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 4 \
  --beam-bfs-handoff-deferred-cap 1 \
  --json --telemetry

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 4 \
  --beam-bfs-handoff-deferred-cap 2 \
  --json --telemetry

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 4 \
  --beam-bfs-handoff-deferred-cap 3 \
  --json --telemetry
```

Artifacts:

- `tmp/4hp_dist_beam_control.json`
- `tmp/4hp_dist_handoff_depth4.json`
- `tmp/4hp_dist_handoff_cap0.json`
- `tmp/4hp_dist_handoff_cap1.json`
- `tmp/4hp_dist_handoff_cap2.json`
- `tmp/4hp_dist_handoff_cap3.json`

Matching wall-time/status sidecars:

- `tmp/4hp_dist_beam_control.{time,status}`
- `tmp/4hp_dist_handoff_depth4.{time,status}`
- `tmp/4hp_dist_handoff_cap0.{time,status}`
- `tmp/4hp_dist_handoff_cap1.{time,status}`
- `tmp/4hp_dist_handoff_cap2.{time,status}`
- `tmp/4hp_dist_handoff_cap3.{time,status}`

## Results

### Plain beam control

- outcome: `unknown` (solver exit `3`)
- wall time: `0.07s`
- `collisions_with_other_frontier = 0`
- `approximate_other_side_hits = 0`
- `discovered_nodes = 5370`
- `frontier_nodes_expanded = 182`
- `total_visited_nodes = 5372`
- `max_frontier_size = 10`

### Historical handoff baseline and sub-beam cap variants

- historical depth-`4` handoff: timed out at `5.05s`; exit `124`; no JSON
- `deferred_cap = 0`: timed out at `5.06s`; exit `124`; no JSON
- `deferred_cap = 1`: timed out at `5.06s`; exit `124`; no JSON
- `deferred_cap = 2`: timed out at `5.04s`; exit `124`; no JSON
- `deferred_cap = 3`: timed out at `5.04s`; exit `124`; no JSON

No sub-beam cap emitted final JSON inside the shared bound, so there is still
no useful-reach ledger to compare against plain beam beyond the negative fact
that every handoff variant remained outside the budget where plain beam stayed
cheap and stable.

The strongest new negative result is `deferred_cap = 0`: even dropping all
retained overflow did not rescue the depth-`4` handoff shape on this control.

## Decision

No sub-beam deferred cap produces a meaningfully better handoff measurement
surface on graph-only `brix_ruiz_k3`.

Why:

- against plain beam, every handoff variant still lost the budget ledger by
  timing out at the same `5s` cap where the control finished in `0.07s`;
- against the historical depth-`4` handoff baseline, none of the sub-beam caps
  improved the outcome class or emitted final JSON, so there is no useful-reach
  evidence that any cap rescues the surface;
- raw semantics got stricter all the way down to `deferred_cap = 0`, but the
  measurement surface did not improve, so this is not a keepable frontier-case
  change.

## Follow-up

- Do not change `research/cases.json` from this sweep.
- Keep the existing plain-beam control and historical depth-`4` handoff
  baseline as the shared corpus frontier comparison.
- Do not escalate to repeated harness measurement: no sub-beam cap produced
  stable final JSON within the bounded first-pass budget.
- If this handoff line is revisited, the next bounded follow-up should not be
  another cap-only corpus probe. First inspect or instrument why the depth-`4`
  handoff still times out even at `deferred_cap = 0`, then decide whether a new
  measurement surface is justified.
