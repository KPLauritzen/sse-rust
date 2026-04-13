# Beam Search K=3 Validation

## Goal

Validate the new beam executor against the known Brix-Ruiz `k=3` pair
`[[1,3],[2,1]] -> [[1,6],[1,1]]` and compare it with the existing mixed BFS.

## Implementation Under Test

- `19fe65d` introduced the initial beam frontier executor from the worker branch.
- `9065c66` tightened the control surface so beam is explicit via
  `--search-mode beam` / `SearchMode::Beam`, and `--beam-width` no longer
  silently overrides other modes.

## Probe Configuration

Bounded comparison run used:

```bash
target/debug/search 1,3,2,1 1,6,1,1 \
  --max-lag 7 \
  --max-intermediate-dim 4 \
  --max-entry 10 \
  --json --telemetry
```

Variants:

- mixed BFS
- beam width `50`
- beam width `200`
- beam width `1000`

Each run was wrapped in `timeout 60s` and `/usr/bin/time`.

## Results

### Mixed BFS

- timed out at `60s`
- did not emit final JSON under the cap

### Beam width 50

- outcome: `unknown`
- wall time: `11.53s`
- `frontier_nodes_expanded = 637`
- `candidates_generated = 229013`
- `max_frontier_size = 50`
- `total_visited_nodes = 25835`
- `collisions_with_other_frontier = 0`

### Beam width 200

- timed out at `60s`
- did not emit final JSON under the cap

### Beam width 1000

- timed out at `60s`
- did not emit final JSON under the cap

## Interpretation

- The current beam implementation does enforce the width cap and can return much
  faster than mixed BFS on this bounded lag-7 probe when the width is small.
- That speedup does **not** yet translate into solving the known `k=3` case:
  width `50` returns `unknown`, and wider beams lose the speed advantage and
  time out under the same 60-second cap.
- For this case, the present score/frontier policy is therefore not yet a
  competitive replacement for the existing search; it is an implementation
  milestone, not a successful benchmark result.

## Next Step

Retune the ranking/frontier policy before trying `fl1.6`:

- consider hybridizing the default score with the stronger `dimension_low`
  signal observed in replay,
- consider depth scheduling or alternation changes so the beam does not churn
  for hundreds of single-node expansions without a meet,
- rerun this exact bounded comparison before escalating to wider campaigns.
