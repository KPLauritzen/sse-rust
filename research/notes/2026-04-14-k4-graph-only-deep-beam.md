# Goal 3 probe: k4 graph-only deep beam sweeps (2026-04-14)

## Question
Can graph-only search (no factorisation families) find a k4 witness on the Brix-Ruiz endpoint when pushing lag much deeper under beam control?

## Endpoint

- `A=1,4,3,1`
- `B=1,12,1,1`

## Setup

Common config:

- `move_policy=graph-only`
- `frontier_mode=beam`
- `max_intermediate_dim=5`, `max_entry=12`
- outer cap `timeout -k 10s 120s`

## Results

Beam64 lag sweep:

- lag `20` (`tmp/loop30_k4_graph_beam64_dim5_lag20.json`): unknown in 2s, visited `128,118`.
- lag `30` (`tmp/loop30_k4_graph_beam64_dim5_lag30.json`): unknown in 3s, visited `219,284`.
- lag `40` (`tmp/loop30_k4_graph_beam64_dim5_lag40.json`): unknown in 5s, visited `305,954`.

Beam-width sweep at lag40:

- beam `128` (`tmp/loop30_k4_graph_beam128_dim5_lag40.json`): unknown in 10s, visited `629,645`.
- beam `256` (`tmp/loop30_k4_graph_beam256_dim5_lag40.json`): unknown in 21s, visited `1,274,769`.

Deep stress point:

- beam `256`, lag `100` (`tmp/loop30_k4_graph_beam256_dim5_lag100.json`): unknown in 78s, visited `3,983,365`.

All runs remained `unknown`; no `equivalent` witness found.

## Interpretation

- Pure graph-move exploration can scale to very high lag under beam within modest wall time.
- Even deep graph-only coverage does not produce a k4 witness for this endpoint.

## Decision

Do not prioritize further graph-only-deep sweeps for Goal 3; they appear to be a low-yield branch compared with mixed/beam tractable envelopes.
