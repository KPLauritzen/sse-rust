# Dynamic mixed endpoint layer events (2026-04-16)

## Question

Can one small search-core observer fix make at least one currently
non-rankable held-out family usable for layer-contrast labels?

Bounded family in scope:

- `lind_marcus`
  - endpoint case: `lind_marcus_a_to_c`

This follows
`research/notes/2026-04-16-observer-layer-event-seam-probe.md`, which showed
that dynamic endpoint solves already populated telemetry layers but still
delivered zero `SearchEvent::Layer` payloads to the observer surface.

## Slice

Patch exactly one seam:

- dynamic endpoint BFS searches with `move_family_policy = mixed`;
- leave the analyzer label contract unchanged;
- leave family-aware provenance and dedup unchanged;
- leave dynamic `graph_only` observer parity out of scope for this round.

The target implementation point is
`search_sse_with_telemetry_dyn_with_deadline_and_observer` in `src/search.rs`.

## Change

Mirror the existing `2x2` observer behavior in the dynamic mixed endpoint path.

Added:

- per-layer `SearchEdgeRecord` collection when an observer is present;
- explicit records for:
  - `SeenCollision`
  - `Discovered`
  - `ExactMeet`
- `emit_layer(&mut observer, records)` at the same points where the dynamic
  mixed path already commits a telemetry layer or returns on an exact meet.

Not changed:

- frontier alternation;
- move-family policy;
- ranking-label extraction;
- witness manifest lookup;
- family split semantics.

So this is an observer/event parity fix, not a search-policy change.

## Validation

Targeted regression test:

```bash
cargo test -q test_dyn_mixed_search_observer_emits_layers_for_lind_marcus_case
```

Bounded analyzer rerun:

```bash
cargo run --quiet --features research-tools --bin analyze_path_signal_corpus -- \
  --cases research/cases.json \
  --case-id lind_marcus_a_to_c \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts \
  research/layer_contrast_signal_corpus_lind_marcus_2026-04-16.json
```

## Results

The new targeted test passed and confirmed the intended seam behavior:

- the dynamic mixed `lind_marcus` case now emits observer layer events;
- the observer sees the same number of layer events as the search telemetry
  layer count for that case;
- all emitted layer payloads on that regression case are non-empty.

The bounded analyzer rerun changed the held-out endpoint case from unrankable to
rankable:

- `solved_cases = 1`
- `unranked_solved_cases = 0`
- `ranked_solution_nodes = 1 / 1`
- `lind_marcus_a_to_c`
  - `budget_lag = 2`
  - `solved_lag = 2`
  - `ranked = 1 / 1`
  - `layers = 2`

The emitted artifact
`research/layer_contrast_signal_corpus_lind_marcus_2026-04-16.json` records:

- `exported_rankable_cases = 1`
- `exported_rankable_layers = 2`
- `exported_matched_candidates = 2`
- `exported_families = ["lind_marcus"]`

Rankable layers in that artifact:

- layer `0`, `forward`
  - `layer_size = 1`
  - matched witness candidate: `2x2:1,1,1,1`
  - label: `best_continuation`
  - `remaining_witness_lag = 1`
- layer `1`, `backward`
  - `layer_size = 2`
  - matched witness candidate: `2x2:1,1,1,1`
  - label: `best_continuation`
  - `remaining_witness_lag = 1`

## Interpretation

Yes: one small search-core observer fix is enough to make at least one
previously non-rankable held-out family usable for layer-contrast labels.

For this round, `lind_marcus` now has durable layer-contrast evidence without
changing analyzer-side provenance or dedup semantics.

By inspection, the same `SearchEvent::Layer` stream also feeds sqlite
persistence, so the observer recorder should now receive these dynamic mixed
layers too. That is an inference from the shared observer surface; sqlite was
not rerun in this bounded validation round.

Dynamic `graph_only` parity and broader held-out-family reruns remain separate
follow-up slices if this lane is continued.
