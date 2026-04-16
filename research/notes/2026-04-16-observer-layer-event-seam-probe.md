# Observer layer-event seam probe (2026-04-16)

## Question

Can one small observer/extraction change make at least one currently
non-rankable held-out family usable for layer-contrast labels?

Families in scope:

- `lind_marcus`
- `higher_block`

## Slice

Probe exactly one bounded seam:

- keep the family-aware manifest and endpoint-case surface unchanged;
- inspect whether solved endpoint searches already produce usable observed
  sibling layers for the current analyzer/export lane;
- stop if the blocker is below `analyze_path_signal_corpus`.

This follows the previous held-out-family policy probe and stays on the
layer-contrast extraction lane.

## Probe setup

Used the existing endpoint cases from `research/cases.json`:

- `lind_marcus_a_to_c`
- `full_2_shift_higher_block_1x1_to_4x4`

Three bounded checks were run.

1. Search telemetry surface:

```bash
cargo run --quiet --bin search -- \
  3x3:1,1,0,0,0,1,1,1,1 1x1:2 \
  --max-lag 2 --max-intermediate-dim 2 --max-entry 2 \
  --move-policy mixed --telemetry --json

cargo run --quiet --bin search -- \
  1x1:2 4x4:1,1,0,0,0,0,1,1,1,1,0,0,0,0,1,1 \
  --max-lag 4 --max-intermediate-dim 4 --max-entry 2 \
  --move-policy mixed --telemetry --json
```

2. Current analyzer/export surface:

```bash
cargo run --quiet --features research-tools --bin analyze_path_signal_corpus -- \
  --cases research/cases.json \
  --case-id lind_marcus_a_to_c \
  --case-id full_2_shift_higher_block_1x1_to_4x4 \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/observer_probe.json
```

3. Observer persistence surface via sqlite:

```bash
cargo run --quiet --bin search -- \
  3x3:1,1,0,0,0,1,1,1,1 1x1:2 \
  --max-lag 2 --max-intermediate-dim 2 --max-entry 2 \
  --move-policy mixed --visited-db tmp/lind_marcus_layers.sqlite --json >/dev/null

cargo run --quiet --bin search -- \
  1x1:2 4x4:1,1,0,0,0,0,1,1,1,1,0,0,0,0,1,1 \
  --max-lag 4 --max-intermediate-dim 4 --max-entry 2 \
  --move-policy mixed --visited-db tmp/higher_block_layers.sqlite --json >/dev/null
```

The durable summary of the observed counts is stored in
`research/observer_layer_event_probe_2026-04-16.json`.

## Results

### Search telemetry says the endpoint solves do have layers

- `lind_marcus_a_to_c`
  - solved lag `2`
  - telemetry layers `2`
  - telemetry `candidates_after_pruning = 4`
  - telemetry `discovered_nodes = 3`
- `full_2_shift_higher_block_1x1_to_4x4`
  - solved lag `4`
  - telemetry layers `3`
  - telemetry `candidates_after_pruning = 246`
  - telemetry `discovered_nodes = 14`

So this round is **not** blocked by the absence of search-layer work.

### The observer/export seam still exposes zero usable layers

- `analyze_path_signal_corpus` still reported:
  - `lind_marcus_a_to_c`: `layer_count = 0`, `ranked = 0/1`
  - `full_2_shift_higher_block_1x1_to_4x4`: `layer_count = 0`, `ranked = 0/2`
- the sqlite observer surface persisted:
  - one `search_runs` row per case;
  - two `run_nodes` root rows per case;
  - zero `run_edges` rows for both cases.

In a narrow local observer counter probe, both runs delivered:

- `Started = 1`
- `Roots = 1`
- `Finished = 1`
- `Layer = 0`

That matched both the analyzer result and the sqlite persistence result.

### A collector-only widening attempt did not change the outcome

A temporary local widening of `LayerCollector` to retain all
non-`SeenCollision` successors, not just `enqueued` / `ExactMeet`, still left
both cases at `layer_count = 0`.

So the blocker is not the current collector predicate alone.

## Interpretation

The current layer-contrast seam is blocked **below**
`src/bin/analyze_path_signal_corpus.rs`.

The current search surface for these held-out endpoint cases provides:

- search telemetry layers;
- a solved path;
- start/root/finish observer events;

but it does **not** provide any `SearchEvent::Layer` payloads to the observer
surface that the analyzer and sqlite recorder depend on.

Without those layer events, the analyzer cannot recover a sibling set for
within-layer continuation labels, regardless of family manifests, dedup rules,
or export-side filtering.

Source inspection also shows that the dynamic `graph_only` path currently builds
telemetry layers without corresponding observer-layer emission. That reinforces
the same conclusion: this seam needs a core search-observer fix, not another
family-policy or export-only tweak.

## Decision

Treat this round as a durable negative result.

Do **not** claim that one small analyzer/export change made a non-Brix
held-out family rankable.

The next useful follow-up, if this lane is revisited, should be a dedicated
search-core observer fix that makes dynamic endpoint searches emit
`SearchEvent::Layer` payloads consistently with their telemetry layers.
