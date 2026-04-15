# Goal 3 probe: k=4 beam envelope sweep on Brix-Ruiz endpoint (2026-04-14)

## Question
Can frontier-policy retuning produce any k=4 path signal (`equivalent`) on the open Brix-Ruiz k4 endpoint while remaining tractable?

## Endpoint

- `A=1,4,3,1`
- `B=1,12,1,1`

## Baseline observations

Mixed BFS probes:

- `dim=3, entry=8, lag=8` (`tmp/loop28_k4_d3e8l8t60.json`): unknown, completed quickly.
- `dim=4, entry=10, lag=8` (`tmp/loop28_k4_d4e10l8t120.json`): unknown, high frontier churn.
- `dim=5, entry=10, lag=8` (`tmp/loop28_k4_d5e10l8t120.json`): timeout at 120s.

## Frontier A/B

At `dim=4, entry=10, lag=8`:

- `mixed + beam(32)` (`tmp/loop28_k4_mixed_beam32.json`): unknown, very low frontier/visited, completes immediately.
- `mixed + beam-bfs-handoff(32)` (`tmp/loop28_k4_mixed_handoff32.json`): timeout 120s.
- `graph-only + bfs` (`tmp/loop28_k4_graph_bfs.json`): unknown, quick, no factorisations.

## Beam depth/width sweeps

`dim=4, entry=10`, mixed beam:

- beam32 with lag `8/10/12/14/16` all returned unknown quickly.
- widening beam at lag12 (`32/64/128`) increased work but remained unknown.

`dim=5, entry=10`, mixed beam:

- beam64, lag12 (`tmp/loop28_k4_mixed_beam64_lag12_dim5e10.json`): unknown in 101s.
- beam64, lag14 (`tmp/loop28_k4_mixed_beam64_lag14_dim5e10.json`): unknown in 119s.
- beam64, lag16 (`tmp/loop28_k4_mixed_beam64_lag16_dim5e10.json`): timeout 120s (empty JSON).
- beam128, lag12 (`tmp/loop28_k4_mixed_beam128_lag12_dim5e10.json`): timeout 120s.

## Interpretation

- No `equivalent` witness found for k=4 in this sweep.
- A new tractable Goal-3 envelope exists at `mixed + beam64 + dim5 + entry10 + lag<=14` (returns unknown before timeout).
- The next cliff is around lag16 at this width/cap.

## Next hypothesis

If pursuing Goal 3 next, target this tractable envelope and improve frontier ranking quality within beam (or add guided proposal surfaces for k4), rather than widening width aggressively.
