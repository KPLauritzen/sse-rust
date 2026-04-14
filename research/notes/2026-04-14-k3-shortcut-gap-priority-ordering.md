# k=3 shortcut-search gap-priority segment ordering (2026-04-14)

## Question

For the hard Brix-Ruiz `k=3` endpoints, does spending the guided shortcut segment-attempt budget on larger gaps first improve lag-reduction progress within the same total-attempt cap?

## Context

Recent normalized guide-pool runs plateaued at lag 7 with stop reason `max_segment_attempts_reached`, so this probe targeted segment-attempt ordering inside guided refinement.

Endpoints:

- `A = [[1,3],[2,1]]`
- `B = [[1,6],[1,1]]`

Fixed run configuration (pre and post):

- stage: `shortcut-search`
- guide artifacts: `research/guide_artifacts/k3_normalized_guide_pool.json`
- bounds: `max_intermediate_dim=5`, `max_entry=6`
- guided config: `max_shortcut_lag=4`, `min_gap=2`, `max_gap=6`, `rounds=2`
- shortcut config: `max_guides=12`, `rounds=2`, `max_total_segment_attempts=128`

## Evidence

Pre-change run (baseline for this probe):

```bash
timeout 300 cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 4 --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 --shortcut-max-total-segment-attempts 128 \
  --json --telemetry > tmp/k3_shortcut_gaporder_pre.json
```

Post-change run (same command/config, after applying larger-gap-first attempt ordering):

```bash
timeout 300 cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 4 --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 --shortcut-max-total-segment-attempts 128 \
  --json --telemetry > tmp/k3_shortcut_gaporder_post.json
```

Observed metrics:

- outcome: `equivalent` in both runs
- witness lag: `7 -> 7`
- guided segments considered: `128 -> 128`
- guided segments improved: `11 -> 11`
- promoted guides: `2 -> 2`
- stop reason: `max_segment_attempts_reached` in both runs
- frontier nodes expanded: `20088 -> 21665` (worse)
- total visited nodes: `1263782 -> 1394505` (worse)

Hard-gate harness comparison after reverting the tweak:

- `just research-json-save baseline`
- `just research-json-save 2026-04-14-loop-gap-ordering-eval`
- required cases: `24/24 -> 24/24`
- target hits: `21 -> 21`
- total points: `3645 -> 3645`
- telemetry-focus score: `45802619 -> 45802619`
- total elapsed ms: `13728 -> 13749` (noise-level slower)

## Conclusion

Gap-prioritized segment ordering did not improve the k=3 lag plateau and increased search work under the same attempt budget. The heuristic was reverted.

## Next Steps

Test a stricter candidate filter instead of reordering all candidates, e.g. prune low-upside segment attempts whose maximum theoretical lag gain is below a configurable threshold, so budget is reduced rather than merely reshuffled.
