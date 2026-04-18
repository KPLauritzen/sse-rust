# Sampled positive-conjugacy main-search integration: bounded root seed hint for `search_sse_2x2` (2026-04-18)

## Question

Can the existing sampled positive-conjugacy witness/proposal/seed machinery land
as a first real seam in the main `2x2` search path without widening into a
general proposal engine or changing exact search semantics?

## Chosen slice

Kept the integration deliberately narrow.

- `2x2` endpoints only.
- Main `search_sse_2x2_with_telemetry_and_observer` BFS path only.
- Root expansion layer only.
- Exact same-dimension successors only.
- No dynamic search changes.
- No hard prune or admission gate.

Implementation shape:

- added a small `src/conjugacy.rs` helper that turns:
  - bounded positive-conjugacy witness,
  - invariant-compatible exact reprojections,
  - existing exact seed ranking,
  into a tiny ranked exact seed-hint list;
- threaded that list into `src/search.rs` in one place:
  - after root-layer `expand_frontier_layer`,
  - before normal merge/enqueue,
  - by reordering only the exact `2x2` successors that match the tiny hint set;
- left all non-hinted expansions in their prior stable order.

Bounded positive-conjugacy settings used by the main-search seam:

- `max_conjugator_entry <= 4` (also capped by `SearchConfig.max_entry`)
- `sample_points = 64`
- `max_proposals = 4`
- `max_candidates = 4`
- proposal source = invariant-compatible reprojections only

Meaning:

- this is a candidate ordering hint;
- it is not treated as a proof object;
- it does not prune non-hinted branches.

## Validation

Focused checks run:

```bash
timeout -k 10s 60s cargo fmt

timeout -k 10s 60s rustfmt src/conjugacy.rs src/search.rs

cargo test -p sse-core conjugacy::tests --lib

cargo test -p sse-core search::tests::test_search_2x2_root_layer_applies_positive_conjugacy_seed_hint_ordering --lib

cargo test -p sse-core search::tests::test_brix_ruiz_k3_search --lib

timeout -k 10s 60s cargo run -p sse-core --bin search -- 1,3,2,1 1,6,1,1 --max-lag 1 --max-intermediate-dim 4 --max-entry 8 --telemetry
```

Formatter note:

- `cargo fmt` timed out in this workmux setup without producing output;
- `rustfmt src/conjugacy.rs src/search.rs` completed cleanly and the focused
  tests above were rerun afterward.

Bounded observer calibration surface used by the new search seam test:

- `max_lag = 1`
- `max_intermediate_dim = 4`
- `max_entry = 8`
- fixed calibration pool:
  - forward/reverse Brix-Ruiz `k=3`
  - forward/reverse Brix-Ruiz `k=4`
  - forward/reverse Riedel-Baker `k=3`

## Result

This is a **kept main-search integration**, but the bounded outcome for this
round is **neutral**.

What changed:

- the observer-based search test confirms that at least one fixed calibration
  root in the bounded pool now gets a different first-layer same-dimension
  expansion order from the positive-conjugacy seed hint;
- that change is realized in the actual main `search_sse_2x2` path, not only in
  sidecar ranking code.

What did not change in the bounded control:

- the normal `search` binary on Brix-Ruiz `k=3`
  - `A=[[1,3],[2,1]]`
  - `B=[[1,6],[1,1]]`
  - `lag<=1`, `dim<=4`, `entry<=8`
  still returns `Unknown`;
- telemetry remained a normal one-layer bounded search:
  - `frontier nodes expanded = 1`
  - `candidates after pruning = 324`
  - `discovered nodes = 323`
  - `max frontier size = 323`

Reading:

- the seed signal is now genuinely wired into main-search behavior;
- under this bounded control it changes realized order, but not the bounded
  search outcome;
- that is acceptable for this first seam because the integration is explicit,
  exact, and still narrowly scoped.

## Conclusion

Keep this integration.

Current status:

- first real sampled positive-conjugacy seam is now in the main `2x2` search;
- it should stay limited to root-layer ordering for now;
- this round is neutral on bounded solve quality, but positive on infrastructure:
  the signal now affects real search order without turning into an unsafe prune.
