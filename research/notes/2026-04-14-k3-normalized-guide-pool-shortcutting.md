# k=3 normalized guide pool + iterative shortcutting (2026-04-14)

## Question

Can the current generic staged solver (`search --stage shortcut-search`) beat lag 7 on the hard Brix-Ruiz `k=3` pair when seeded from a normalized in-repo guide pool?

Endpoints:

- `A = [[1,3],[2,1]]`
- `B = [[1,6],[1,1]]`

## Guide-pool assembly

I added and ran a new helper:

- `cargo run --release --features research-tools --bin assemble_k3_guide_pool -- --out research/guide_artifacts/k3_normalized_guide_pool.json`

Source ingestion:

- seeded fixture guide from `research/fixtures/brix_ruiz_family.json#brix_ruiz_k3`
- sqlite shortcut paths from `research/k3-graph-paths.sqlite` (`shortcut_path_results`)
- sqlite graph paths from `research/k3-graph-paths.sqlite` (`graph_path_results`)
- explicit in-repo Lind-Marcus/Baker lag-7 witness from `src/bin/check_lind_marcus_path.rs`

Materialization method:

- for matrix-only path sources, reconstruct each consecutive segment with endpoint search at `max_lag=1` using recorded sqlite run bounds (`max_dim`, `max_entry`) and mixed move policy
- stitch all segment witnesses into validated `full_path` artifacts
- normalize into one artifact envelope

Assembly outcome (from `tmp/k3_guide_pool_assemble.log`):

- loaded candidates: `15`
- accepted candidates: `12`
- reconstruction failures: `3` (`unknown` on lag-1 segment replay)
- unique guides written: `12`
- best lag in pool: `7`

Resulting pool file:

- `research/guide_artifacts/k3_normalized_guide_pool.json`

Lag distribution in the normalized pool:

- lag `7`: `2` guides (includes Lind-Marcus/Baker reference)
- lag `8`: `1`
- lag `9`: `1`
- lag `10`: `3`
- lag `11`: `3`
- lag `30`: `1`
- lag `31`: `1`

## Iterative shortcut-search runs

### Round 1 (small bound)

Command:

```bash
cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 4 --max-entry 5 \
  --guided-max-shortcut-lag 3 --guided-min-gap 2 --guided-max-gap 4 --guided-rounds 1 \
  --shortcut-max-guides 6 --shortcut-rounds 1 --shortcut-max-total-segment-attempts 48 \
  --json --telemetry \
  --write-guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  > tmp/k3_shortcut_round1.json
```

Outcome:

- `equivalent`, lag `7`
- shortcut telemetry: `best_lag_start=7`, `best_lag_end=7`
- stop reason: `max_segment_attempts_reached`
- segment attempts: `48`

### Round 2 (moderate increase)

Command:

```bash
cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 4 --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 --shortcut-max-total-segment-attempts 128 \
  --json --telemetry \
  --write-guide-artifact research/guide_artifacts/k3_shortcut_round2.json \
  > tmp/k3_shortcut_round2.json
```

Outcome:

- `equivalent`, lag `7`
- shortcut telemetry: `best_lag_start=7`, `best_lag_end=7`
- stop reason: `max_segment_attempts_reached`
- segment attempts: `128`

### Round 3 attempt (wider attempt budget)

Command (timed):

```bash
timeout 420 cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 4 --guided-min-gap 2 --guided-max-gap 6 --guided-rounds 2 \
  --shortcut-max-guides 12 --shortcut-rounds 2 --shortcut-max-total-segment-attempts 256 \
  --json --telemetry \
  --write-guide-artifact research/guide_artifacts/k3_shortcut_round3.json \
  > tmp/k3_shortcut_round3.json
```

Outcome:

- timed out at `420s` (exit `124`)
- no completed JSON/artifact produced for this run

## Distinct witness check

I compared matrix-sequence signatures across:

- the 12 normalized pool guides
- `k3_shortcut_round1.json` artifact output
- `k3_shortcut_round2.json` artifact output

Result:

- `12` unique witness signatures total
- both completed shortcut runs produced the same lag-7 witness signature as the pool's Lind-Marcus/Baker reference guide
- no new distinct witness shape appeared in these two completed iterative passes

## Current best-known result in this branch

For exact endpoints `[[1,3],[2,1]] -> [[1,6],[1,1]]` under the current staged shortcut workflow:

- best lag reached in completed runs: **7**
- no `<7` witness found in this session

