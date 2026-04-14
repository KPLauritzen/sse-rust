# Search Parallelism Investigation (`5b8`)

Date: 2026-04-13

## Scope

This note records the investigation behind bead `5b8`, focused on where one
endpoint-search layer spends wall time, what additional parallelism looks safe,
and which correctness, determinism, and memory constraints matter before
changing the solver.

## Instrumentation Added

- `SearchLayerTelemetry` now carries a `timing` block with per-layer wall-clock
  timing:
  - `expand_compute_nanos`
  - `expand_accumulate_nanos`
  - `dedup_nanos`
  - `merge_nanos`
  - `finalize_nanos`
  - `total_nanos`
- The `search` CLI pretty `--telemetry` output now prints aggregate layer-timing
  totals and the split across those phases.

The timing coverage is on the main mixed BFS path and beam path used by the
generic endpoint solver and the dedicated `2x2` endpoint solver.

## Probes Run

Artifacts are in repo-local `tmp/`.

```sh
cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 4 --max-intermediate-dim 3 --max-entry 4 \
  --telemetry --json > tmp/5b8-k3-l4.json

cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 6 --max-intermediate-dim 3 --max-entry 6 \
  --telemetry --json > tmp/5b8-k3-l6.json

cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 8 --max-intermediate-dim 3 --max-entry 8 \
  --telemetry --json > tmp/5b8-k3-wide.json

cargo run --release --bin search -- \
  1,4,3,1 1,12,1,1 \
  --max-lag 8 --max-intermediate-dim 3 --max-entry 8 \
  --telemetry --json > tmp/5b8-k4-probe.json
```

## Measured Layer Breakdown

### `k=3`, `lag=4`, `max_entry=4`

- Total layer time: `63.9 ms`
- `expand_compute`: `72.5%`
- `expand_accumulate`: `8.8%`
- `dedup`: `12.5%`
- `merge`: `5.8%`
- `finalize`: `0.2%`

### `k=3`, `lag=8`, `max_entry=8` (`brix_ruiz_k3_wide_probe` shape)

- Total layer time: `564.8 ms`
- `expand_compute`: `86.5%`
- `expand_accumulate`: `3.5%`
- `dedup`: `7.0%`
- `merge`: `2.7%`
- `finalize`: `0.2%`

Largest layers:

- backward layer `4`: frontier `3793`, total `251.3 ms`, compute `225.2 ms`,
  dedup `15.2 ms`, merge `2.9 ms`
- forward layer `5`: frontier `1538`, total `129.6 ms`, compute `123.1 ms`,
  dedup `4.2 ms`, merge `0.1 ms`
- backward layer `3`: frontier `664`, total `104.7 ms`, compute `79.0 ms`,
  dedup `11.1 ms`, merge `7.8 ms`

### `k=4`, `lag=8`, `max_entry=8` (`brix_ruiz_k4_probe` shape)

- Total layer time: `420.7 ms`
- `expand_compute`: `89.7%`
- `expand_accumulate`: `3.2%`
- `dedup`: `5.2%`
- `merge`: `1.8%`
- `finalize`: `0.1%`

## What Changed In The Diagnosis

The initial hypothesis that serial merge and parent-map work dominate one layer
does not hold for these bounded mixed-search probes.

What the measurements show instead:

- the existing Rayon-backed expansion compute is already the dominant per-layer
  wall-clock cost,
- whole-layer deduplication is the largest secondary serial phase,
- parent/depth/orig insertion plus queueing is present but materially smaller
  than both compute and dedup,
- and only one direction expands at a time, so there is still inter-layer
  serialization even when intra-layer expansion is parallel.

This fits the code structure in `src/search.rs`:

- `expand_frontier_layer*` performs per-node successor generation in parallel,
- materializes a full `Vec<FrontierExpansion>`,
- deduplicates the whole layer,
- and only then serially commits discoveries into `parent`, `depths`, `orig`,
  signature sets, and the next frontier.

## Safe Parallelism Assessment

### Low-risk / plausible

- Parallel or sharded deduplication is the most credible next target inside one
  layer because it is the largest non-compute phase.
- Any such change should preserve deterministic first-writer selection, because
  dedup winner choice feeds the parent map and therefore the reconstructed path.

### Higher-risk / lower-immediate-payoff

- Parallelizing the current merge/commit loop alone is unlikely to buy much on
  these probes because it is only about `2-6%` of measured layer time.
- Concurrent forward and backward expansion would need snapshot semantics for
  `seen`, deterministic conflict resolution on meets, and a defined rule for
  which side wins when both discover the same canonical state in the same round.

## Correctness, Determinism, And Memory Risks

### Correctness / determinism

- The solver currently has deterministic "first accepted expansion wins"
  behavior because expansions are collected, deduplicated, and committed in a
  stable order.
- Parallel dedup or parallel commit can change which parent claims a canonical
  node first unless winner selection is made explicit and stable.
- Parallel bidirectional expansion can make exact-meet discovery timing-dependent
  unless both sides work against immutable snapshots and merge afterward.

### Memory

- The main mixed solver already buffers a full `Vec<FrontierExpansion>` for the
  entire layer before commit.
- Sharding dedup or expanding both directions at once will increase peak live
  candidate memory unless the staging representation is changed.
- This matches the earlier host-side concern that larger `k=4` runs may become
  memory-bound before they become fully compute-bound.

## Recommendation

1. Treat this bead as evidence that the first deeper parallelism design should
   target deterministic dedup or reduced staging memory, not just a locked
   parallel parent-map commit.
2. Keep the current one-direction-at-a-time execution until a snapshot-and-merge
   design exists with explicit winner rules and a memory budget story.
3. For `k=4+`, prioritize instrumentation and design work that reduces
   per-layer staging memory, because extra parallelism that duplicates staging
   will likely make the memory problem worse before it helps throughput.
