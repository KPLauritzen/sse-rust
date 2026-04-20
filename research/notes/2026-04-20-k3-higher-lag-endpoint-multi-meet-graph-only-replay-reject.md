# Higher-lag endpoint multi-meet retention plus shortcut replay on hard Brix-Ruiz `k = 3` (reject) (2026-04-20)

## Question

For the hard exact endpoints

- `A = [[1,3],[2,1]]`
- `B = [[1,6],[1,1]]`

can a higher-lag exact-endpoint multi-meet surface produce additional explicit
non-Baker lag-7 witnesses when replayed back onto the same exact endpoints
through bounded shortcut search?

This slice stayed bounded to the one hard exact pair only. It did not widen
generic tooling, broaden endpoint families, or refactor the general guide-pool
surface.

## Screening before materialization

I used two bounded screening probes to avoid materializing a larger retained
pool unless the endpoint surface actually broadened.

### Graph-plus-structured `lag9 / dim4 / entry6 / cap24`

Command:

```sh
timeout -k 10s 60s target/release/search 1,3,2,1 1,6,1,1 \
  --stage endpoint-search \
  --frontier-mode bfs \
  --move-policy graph-plus-structured \
  --max-lag 9 --max-intermediate-dim 4 --max-entry 6 \
  --endpoint-multi-meet-cap 24 \
  --json --telemetry \
  > tmp/sse-rust-36b_exact_endpoint_multi_meet_gps_l9_dim4_entry6_cap24_2026-04-20.result.json
```

Observed result:

- outcome: `equivalent`
- retained exact meets: `4 / 24`
- retained meet lag(s): all `7`
- reconstructed witness lag(s): all `8`
- telemetry:
  - `frontier_nodes_expanded = 161,340`
  - `total_visited_nodes = 558,689`
  - `max_frontier_size = 358,619`
  - `layers = 7`

This did **not** broaden the retained surface beyond the prior
`lag8 / dim4 / entry5 / cap12` probe from 2026-04-19. It still retained only
four exact meets, so I did not materialize this pool.

### Graph-plus-structured `lag9 / dim5 / entry5 / cap24`

Command:

```sh
timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --stage endpoint-search \
  --frontier-mode bfs \
  --move-policy graph-plus-structured \
  --max-lag 9 --max-intermediate-dim 5 --max-entry 5 \
  --endpoint-multi-meet-cap 24 \
  --json --telemetry \
  > tmp/sse-rust-36b_exact_endpoint_multi_meet_gps_l9_dim5_entry5_cap24_2026-04-20.result.json
```

Observed result:

- timed out before the merge layer finished
- no retained `endpoint_exact_meets` JSON surface was published

This axis was too expensive for a bounded exact-endpoint retention pass, so I
rejected it and pivoted to graph-only.

## Materialized higher-lag retained pool

The viable higher-lag surface came from the existing fast graph-only exact
endpoint lane.

Command:

```sh
timeout -k 10s 30s target/release/search 1,3,2,1 1,6,1,1 \
  --stage endpoint-search \
  --frontier-mode bfs \
  --move-policy graph-only \
  --max-lag 22 --max-intermediate-dim 5 --max-entry 6 \
  --endpoint-multi-meet-cap 24 \
  --json --telemetry \
  > tmp/sse-rust-36b_exact_endpoint_multi_meet_graph_only_l22_dim5_entry6_cap24_2026-04-20.result.json
```

Observed result:

- outcome: `equivalent`
- retained exact meets: `2 / 24`
- retained meet lag(s): both `16`
- reconstructed witness lag(s): both `17`
- telemetry:
  - `frontier_nodes_expanded = 1,382,998`
  - `total_visited_nodes = 1,410,460`
  - `max_frontier_size = 717,764`
  - `layers = 16`

Durable retained guide pool:

- `research/guide_artifacts/k3_exact_endpoint_multi_meet_graph_only_retained_pool_2026-04-20.json`

This retained pool contains exactly the two graph-only exact-endpoint witnesses
emitted on that higher-lag surface and nothing else.

## Replay from that retained pool only

Command:

```sh
timeout -k 10s 120s target/release/search 1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_exact_endpoint_multi_meet_graph_only_retained_pool_2026-04-20.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 5 --guided-min-gap 2 --guided-max-gap 8 \
  --guided-segment-timeout 5 --guided-rounds 2 \
  --shortcut-max-guides 2 --shortcut-rounds 2 \
  --shortcut-max-total-segment-attempts 128 \
  --json --telemetry \
  --write-guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_graph_only_replay_lag9_2026-04-20.json \
  > tmp/sse-rust-36b_exact_endpoint_multi_meet_graph_only_replay_lag9_2026-04-20.result.json
```

Observed result:

- outcome: `equivalent`
- promoted witness lag: `9`
- guide artifacts considered / accepted: `2 / 2`
- shortcut guides loaded / accepted / unique: `2 / 2 / 2`
- best lag start / end: `17 / 9`
- segment attempts: `128`
- segment improvements: `72`
- promoted guides: `1`
- stop reason: `max_segment_attempts_reached`

Durable replay artifact:

- `research/guide_artifacts/k3_exact_endpoint_multi_meet_graph_only_replay_lag9_2026-04-20.json`

Promoted exact-endpoint lag-9 signature:

```text
2x2:1,3,2,1
-> 3x3:1,1,2,2,0,1,2,0,1
-> 4x4:1,1,0,1,2,0,1,1,2,2,1,2,0,0,1,0
-> 4x4:0,0,1,0,1,1,0,1,2,2,1,2,1,2,1,0
-> 4x4:0,0,1,1,1,1,0,1,1,0,0,2,1,2,1,1
-> 4x4:0,1,1,1,1,2,0,1,1,2,0,2,0,1,1,0
-> 3x3:0,1,0,1,0,2,1,2,2
-> 3x3:0,0,1,1,2,2,1,2,0
-> 2x2:0,1,5,2
-> 2x2:1,6,1,1
```

## Conclusion

Reject for the stated lag-7 objective.

This round did **not** promote any additional explicit non-Baker lag-7 witness
on the hard exact endpoints.

What the bounded evidence shows instead:

- raising the graph-plus-structured endpoint surface to
  `lag9 / dim4 / entry6 / cap24` did not broaden the retained exact pool at
  all;
- the more aggressive graph-plus-structured `dim5` surface did not finish a
  merge layer under the bounded timeout, so it yielded no usable retained pool;
- the graph-only higher-lag exact surface did produce a real retained exact
  pool, but replaying that pool only improved from lag `17` to lag `9`, not to
  lag `7`.

Current exact lag-7 inventory on these exact endpoints is therefore unchanged:

- the committed Baker family in
  `research/guide_artifacts/k3_shortcut_round1.json`
- the previously promoted exact non-Baker family in
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`

This slice adds durable evidence that higher-lag exact-endpoint meets can
shortcut materially, but on the hard Brix-Ruiz `k=3` pair this bounded replay
still does **not** uncover another explicit lag-7 family.
