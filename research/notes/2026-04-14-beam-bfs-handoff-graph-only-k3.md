# Beam-to-BFS Handoff Validation on Graph-Only `k=3`

## Goal

Evaluate an explicit beam-seeding followed by BFS handoff on the known
Brix-Ruiz `k=3` graph-only control:

- source: `[[1,3],[2,1]]`
- target: `[[1,6],[1,1]]`

The implementation under test keeps plain `beam` unchanged and adds an
explicit `beam_bfs_handoff` frontier mode. In that mode:

- beam overflow is retained instead of destroyed
- parent/depth/orig tracking remains discovery-time, so reconstruction is
  unchanged
- insertion order is deterministic via `(depth, serial)` in the deferred queue
- beam seeding now has an explicit config seam via
  `beam_bfs_handoff_depth`; when unset it still defaults to depth `4`
  inclusive, so depth-`5` and later discoveries go directly to the BFS queue

This probe is intentionally **graph-only**.

## Probe

Bounded commands:

```bash
target/debug/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --beam-width 10 \
  --frontier-mode beam \
  --json --telemetry

target/debug/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --beam-width 10 \
  --frontier-mode beam-bfs-handoff \
  --json --telemetry
```

The plain-beam run completed directly. The handoff run was bounded with
`timeout 20s`.

Follow-up harness sweep after adding the config seam:

- plain `beam` control, `beam_width = 10`, `timeout_ms = 5000`
- `beam_bfs_handoff` at depths `0`, `2`, `4`, and `6`
- same endpoint pair and graph-only bounds as the original probe

## Results

### Plain Beam

- outcome: `unknown`
- wall time: `0.61s`
- `frontier_nodes_expanded = 182`
- `candidates_generated = 8334`
- `total_visited_nodes = 5372`
- `max_frontier_size = 10`

### Beam Then BFS Handoff

- timed out under `20s`
- did not emit final JSON before the cap
- same control also timed out under the same `20s` cap for widths `1`, `2`,
  `4`, and `10`

### Depth Sweep Under The Harness

- plain `beam` control:
  - outcome: `unknown`
  - wall time: `0.51s`
  - `frontier_nodes_expanded = 182`
  - `candidates_generated = 8334`
  - `total_visited_nodes = 5372`
  - `max_frontier_size = 10`
- `beam_bfs_handoff_depth = 0`: timed out under `5.0s`
- `beam_bfs_handoff_depth = 2`: timed out under `5.0s`
- `beam_bfs_handoff_depth = 4`: timed out under `5.0s`
- `beam_bfs_handoff_depth = 6`: timed out under `5.0s`

These numbers were rechecked after fixing the handoff boundary to make depth
`4` inclusive and after deferring beam-phase exact-meet returns until the BFS
phase can recover shorter deferred paths.

## Interpretation

- The depth-4 graph-only handoff is implemented and deterministic, but this
  first bounded control run is negative.
- For this `k=3` case, switching to BFS after only a few beam-guided layers
  causes the deferred queue to grow too aggressively; the BFS phase becomes
  more expensive than the corresponding plain beam run.
- The new depth seam did not produce a viable graph-only harness setting in
  the first bounded sweep: even pushing the handoff out to depth `6` still
  timed out where plain `beam` returned in about half a second.
- The handoff mode is therefore a usable research surface, not yet a good
  default for the graph-only `k=3` benchmark.

## Follow-Up

- Keep the default unchanged for now; the bounded sweep does not justify moving
  away from the existing depth-`4` baseline.
- If graph-only remains the target, try either still-deeper beam seeding
  (`8+`) or a narrower deferred admission policy before the BFS phase begins.
