# Exact-endpoint multi-meet lag-7 diversity probe on hard Brix-Ruiz `k = 3` (2026-04-19)

## Question

For the hard exact endpoints

- `A = [[1,3],[2,1]]`
- `B = [[1,6],[1,1]]`

does the new bounded endpoint multi-meet surface expose more useful diversity
than the old rank-1 exact witness, and if those retained exact meets are
replayed back onto the same exact endpoints, do they still collapse to the
existing Baker lag-7 witness?

This slice stayed bounded to one exact-endpoint probe:

1. retain multiple exact meets on the exact endpoint surface;
2. materialize only those retained exact witnesses as guides;
3. replay only that retained exact pool on the same exact endpoints.

## Exact retained surface

Command:

```sh
timeout -k 10s 60s target/release/search 1,3,2,1 1,6,1,1 \
  --stage endpoint-search \
  --frontier-mode bfs \
  --move-policy graph-plus-structured \
  --max-lag 8 --max-intermediate-dim 4 --max-entry 5 \
  --endpoint-multi-meet-cap 12 \
  --json --telemetry \
  > tmp/sse-rust-28u1_exact_endpoint_multi_meet_gps_l8_dim4_entry5_2026-04-19.result.json
```

Observed result:

- outcome: `equivalent`
- emitted witness lag: `8`
- retained exact meets: `4 / 12`
- each retained exact meet had `path_lag = 7`
- each retained reconstructed to a `9`-matrix / `8`-step exact-endpoint path

Telemetry:

- `frontier_nodes_expanded = 84,875`
- `total_visited_nodes = 235,450`
- `max_frontier_size = 127,212`
- `layers = 7`

Durable retained guide pool:

- [research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json](/home/kasper/dev/sse-rust__worktrees/28u1-multi-meet-lag7-probe/research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json)

Retained exact meet signatures:

### Retained 1

Meeting canonical:

```text
4x4:0,0,1,1,1,0,1,2,1,1,2,2,1,1,0,0
```

Path signature:

```text
2x2:1,3,2,1
-> 3x3:0,1,0,2,1,2,1,2,1
-> 4x4:1,0,1,1,2,1,0,2,2,1,0,1,2,1,0,0
-> 4x4:0,0,1,1,0,1,0,2,1,1,0,1,2,1,1,1
-> 4x4:0,0,1,1,2,2,1,1,1,1,0,0,2,1,1,0
-> 4x4:0,1,1,0,2,0,1,1,1,0,0,1,2,1,1,2
-> 3x3:0,1,1,3,0,3,2,0,2
-> 2x2:0,5,1,2
-> 2x2:1,6,1,1
```

### Retained 2

Meeting canonical:

```text
4x4:0,0,1,1,2,1,0,2,1,0,0,2,1,1,1,1
```

Path signature:

```text
2x2:1,3,2,1
-> 3x3:0,1,0,2,1,2,1,2,1
-> 4x4:1,0,1,1,2,1,0,2,2,1,0,1,2,1,0,0
-> 4x4:0,0,1,1,0,1,0,2,1,1,0,1,2,1,1,1
-> 4x4:0,0,2,1,0,1,2,2,1,1,1,1,1,0,1,0
-> 4x4:1,1,1,1,2,1,0,2,2,0,0,1,1,0,1,0
-> 3x3:0,2,3,1,1,1,1,1,1
-> 3x3:1,2,4,1,0,1,1,0,1
-> 2x2:1,6,1,1
```

### Retained 3

Meeting canonical:

```text
4x4:0,0,1,1,1,0,1,1,1,1,2,0,2,1,2,0
```

Path signature:

```text
2x2:1,3,2,1
-> 3x3:1,1,2,2,0,1,2,0,1
-> 4x4:1,1,0,1,2,0,1,1,2,2,1,2,0,0,1,0
-> 4x4:1,1,0,1,2,1,1,1,0,2,0,1,0,1,1,0
-> 4x4:2,1,0,1,1,0,1,0,2,2,0,1,1,1,1,0
-> 4x4:2,1,0,1,1,0,1,1,2,1,0,2,1,0,1,0
-> 3x3:0,0,1,1,2,2,1,2,0
-> 2x2:0,1,5,2
-> 2x2:1,6,1,1
```

### Retained 4

Meeting canonical:

```text
4x4:0,0,1,1,0,1,0,1,1,2,0,1,2,2,1,1
```

Path signature:

```text
2x2:1,3,2,1
-> 3x3:1,1,2,2,0,1,2,0,1
-> 4x4:1,1,0,1,2,0,1,1,2,2,1,2,0,0,1,0
-> 4x4:1,1,0,1,2,1,1,1,0,2,0,1,0,1,1,0
-> 4x4:1,0,0,1,2,0,1,1,0,1,0,1,2,1,2,1
-> 4x4:0,1,0,1,1,0,2,1,0,0,1,1,2,1,2,1
-> 3x3:1,2,1,1,0,1,1,3,1
-> 3x3:0,2,0,1,1,1,1,4,1
-> 2x2:1,6,1,1
```

## Replay on the same exact endpoints

Command:

```sh
timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json \
  --max-intermediate-dim 4 --max-entry 5 \
  --guided-max-shortcut-lag 4 --guided-min-gap 2 --guided-max-gap 6 \
  --guided-segment-timeout 5 --guided-rounds 2 \
  --shortcut-max-guides 4 --shortcut-rounds 2 \
  --shortcut-max-total-segment-attempts 64 \
  --json --telemetry \
  --write-guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  > tmp/sse-rust-28u1_exact_endpoint_multi_meet_replay_2026-04-19.result.json
```

Observed result:

- outcome: `equivalent`
- lag: `7`
- guide artifacts considered / accepted: `4 / 4`
- shortcut guides loaded / accepted / unique: `4 / 4 / 4`
- best lag start / end: `8 / 7`
- segment attempts: `64`
- stop reason: `max_segment_attempts_reached`

Durable promoted artifact:

- [research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json](/home/kasper/dev/sse-rust__worktrees/28u1-multi-meet-lag7-probe/research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json)

New exact-endpoint lag-7 signature:

```text
2x2:1,3,2,1
-> 3x3:0,1,0,2,1,2,1,2,1
-> 4x4:1,0,1,1,2,1,0,2,2,1,0,1,2,1,0,0
-> 4x4:0,0,1,1,0,1,0,2,1,1,0,1,2,1,1,1
-> 4x4:0,0,2,1,0,1,2,2,1,1,1,1,1,0,1,0
-> 3x3:0,2,3,1,1,1,1,1,1
-> 2x2:0,5,1,2
-> 2x2:1,6,1,1
```

For contrast, the previously committed exact-endpoint Baker artifact
`research/guide_artifacts/k3_normalized_guide_pool.json#k3-lind-marcus-baker-lag7`
has signature:

```text
2x2:1,3,2,1
-> 3x3:1,2,2,2,1,1,1,0,0
-> 4x4:1,2,2,0,1,0,2,0,0,1,1,1,1,1,2,0
-> 4x4:1,2,1,1,1,0,1,0,1,1,0,1,2,0,0,1
-> 4x4:1,2,2,0,1,1,1,1,0,1,0,1,0,2,1,0
-> 4x4:1,1,1,1,3,0,2,2,1,0,0,0,0,1,1,1
-> 3x3:1,1,1,5,0,5,1,0,1
-> 2x2:1,6,1,1
```

These are not the same exact-endpoint lag-7 path.

## Conclusion

This probe produced a positive diversity result.

- the bounded exact-endpoint multi-meet surface retained four distinct exact
  meets on the hard endpoints instead of only exposing one rank-1 witness;
- replaying only that retained exact-endpoint pool on the same exact endpoints
  produced a second explicit lag-7 witness, not the committed Baker witness;
- for this slice, the surface does **not** collapse at exact-meet retention,
  and the bounded replay/promotion step also does **not** collapse back to
  Baker.

The new explicit exact-endpoint lag-7 artifact is
`research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`.
