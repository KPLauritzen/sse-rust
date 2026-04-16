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
- retained overflow admission now has an additional opt-in seam via
  `beam_bfs_handoff_deferred_cap`; when unset the deferred queue remains
  unlimited, and when set only the earliest deferred entries up to that cap are
  retained per frontier side

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

Still-deeper seeding recheck on `HEAD e349c35`:

```bash
timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --frontier-mode beam \
  --beam-width 10 \
  --json --telemetry > tmp/sse-rust-9fv-beam-control.json

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 8 \
  --json --telemetry > tmp/sse-rust-9fv-handoff-depth8.json

timeout -k 1s 5s target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --search-mode graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-depth 10 \
  --json --telemetry > tmp/sse-rust-9fv-handoff-depth10.json
```

The plain-beam control completed in `0.07s`; the depth-`8` and depth-`10`
handoff runs were each bounded by the same `5s` cap.

Bounded deferred-admission follow-up in this worktree:

```bash
/usr/bin/time -f '%e' -o tmp/sse-rust-x15-beam-control.time \
  target/debug/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam \
  --beam-width 10 \
  --json --telemetry > tmp/sse-rust-x15-beam-control.json

/usr/bin/time -f '%e' -o tmp/sse-rust-x15-handoff-cap10.time \
  timeout -k 1s 5s target/debug/search 1,3,2,1 1,6,1,1 \
  --max-lag 10 \
  --max-intermediate-dim 5 \
  --max-entry 6 \
  --move-policy graph-only \
  --frontier-mode beam-bfs-handoff \
  --beam-width 10 \
  --beam-bfs-handoff-deferred-cap 10 \
  --json --telemetry > tmp/sse-rust-x15-handoff-cap10.json
```

This keeps the existing default handoff depth (`4`) and changes only the
retained-overflow admission rule: each frontier side keeps at most one
beam-width worth of deferred entries.

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

### Still-Deeper Seeding Recheck

- plain `beam` control on `target/dist/search`:
  - outcome: `unknown`
  - wall time: `0.07s`
  - `frontier_nodes_expanded = 182`
  - `candidates_generated = 8334`
  - `total_visited_nodes = 5372`
  - `max_frontier_size = 10`
- `beam_bfs_handoff_depth = 8`: timed out under `5.05s` (`tmp/sse-rust-9fv-handoff-depth8.json` remained empty)
- `beam_bfs_handoff_depth = 10`: timed out under `5.05s` (`tmp/sse-rust-9fv-handoff-depth10.json` remained empty)

These numbers were rechecked after fixing the handoff boundary to make depth
`4` inclusive and after deferring beam-phase exact-meet returns until the BFS
phase can recover shorter deferred paths.

### Deferred-Admission Cap Probe

- plain `beam` control on `target/debug/search`:
  - outcome: `unknown`
  - wall time: `0.52s`
  - `frontier_nodes_expanded = 182`
  - `candidates_generated = 8334`
  - `total_visited_nodes = 5372`
  - `max_frontier_size = 10`
- `beam_bfs_handoff_deferred_cap = 10` with the default handoff depth:
  - outcome: `unknown`
  - wall time: `0.87s`
  - `frontier_nodes_expanded = 236`
  - `candidates_generated = 13758`
  - `total_visited_nodes = 7999`
  - `max_frontier_size = 20`

Both commands exited with solver status `3` (`unknown`), but unlike the
uncapped handoff mode the capped run emitted final JSON and stayed well under
the existing `5s` bound.

## Interpretation

- The depth-4 graph-only handoff is implemented and deterministic, but this
  first bounded control run is negative.
- For this `k=3` case, switching to BFS after only a few beam-guided layers
  causes the deferred queue to grow too aggressively; the BFS phase becomes
  more expensive than the corresponding plain beam run.
- The new depth seam did not produce a viable graph-only harness setting in
  the first bounded sweep: even pushing the handoff out to depth `6` still
  timed out where plain `beam` returned in about half a second.
- Still-deeper seeding through the same seam did not rescue the mode either:
  even with `beam_bfs_handoff_depth = 10` (the full `max_lag` cap), the
  retained-overflow BFS phase still failed to finish within `5s` while plain
  `beam` completed in `0.07s` with unchanged telemetry.
- Capping deferred retention at one beam-width per side is the first
  `beam_bfs_handoff` setting on this control that avoids the timeout without
  redesigning the frontier wholesale, so the unbounded deferred queue is the
  main source of the previous blow-up.
- The capped run is still negative for the search goal: it returned `unknown`
  and actually expanded more nodes than plain `beam`, so the seam improves
  measurability, not solution quality.
- The handoff mode is therefore a usable research surface, not yet a good
  default for the graph-only `k=3` benchmark.

## Follow-Up

- Keep the default unchanged for now; the bounded sweep does not justify moving
  away from the existing depth-`4` baseline.
- Deeper beam seeding alone is no longer the most informative next probe on
  this graph-only control; the existing seam already shows that `8` and `10`
  still time out.
- If graph-only remains the target and this surface is revisited, the next
  bounded follow-up should vary small deferred caps around the beam-width scale
  before touching `beam_bfs_handoff_depth` again; depth-only sweeps are now
  less informative than cap sizing.
