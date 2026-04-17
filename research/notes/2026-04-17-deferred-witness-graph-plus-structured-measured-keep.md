# Deferred witness round: exact graph-plus-structured BFS is now a measured keep (2026-04-17)

## Question

After the earlier rejected widening round, does deferred witness reconstruction
become defensible on the exact `graph-plus-structured` Brix-Ruiz `k=3` control
once runtime and memory are measured credibly from the worktree?

This retry stayed narrow on purpose:

- exact `2x2` endpoint-search BFS only
- `graph-plus-structured` only
- no mixed, beam, beam-to-BFS-handoff, or dynamic widening
- measurement path first, then a single focused prototype

## Trustworthy measurement path

Added repo-local measurement wrapper:

- `scripts/measure-search-runtime-rss.sh`

Why this is trustworthy enough for this workmux setup:

- runs inside the repo worktree with `WORKMUX_SANDBOX=container`
- writes all artifacts under repo-local `tmp/`
- times the already-built `target/dist/search` binary directly, so build noise
  is excluded
- records both wall seconds and max RSS KB from `/usr/bin/time`
- keeps all raw per-run stdout/stderr/metrics files for later inspection

Baseline artifact directory:

- `tmp/search-measure-20260417T113351Z-brix-ruiz-k3-gps-exact-baseline`

Prototype artifact directory:

- `tmp/search-measure-20260417T114019Z-brix-ruiz-k3-gps-exact-prototype`

Measured command in both cases:

```bash
target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 8 \
  --max-intermediate-dim 4 \
  --max-entry 5 \
  --move-policy graph-plus-structured \
  --telemetry --json
```

Each measurement set used `5` direct runs.

## Baseline before the prototype

From `tmp/search-measure-20260417T113351Z-brix-ruiz-k3-gps-exact-baseline`:

- median wall time `2.18 s`
- wall samples `2.15, 2.14, 2.21, 2.18, 2.21`
- median max RSS `649,116 KB`
- RSS samples `645,136, 648,120, 652,072, 649,116, 649,308`

Representative solver output:

- outcome `equivalent`
- witness lag `8`
- `frontier_nodes_expanded = 84,875`
- `total_visited_nodes = 212,170`
- `factorisations_enumerated = 470,662`
- `candidates_generated = 889,746`
- `collisions_with_other_frontier = 1`

## Prototype

Kept narrow to the exact `graph-plus-structured` `2x2` BFS seam in
`src/search.rs` and `src/search/path.rs`:

- added a specialized exact BFS path that stores parent canon only:
  `HashMap<DynMatrix, Option<DynMatrix>>`
- left observer emission exact by continuing to use live `expansion.step`
  payloads for `SearchEdgeRecord`
- rebuilt the final witness only after a successful meet by replaying adjacent
  stored matrix pairs with the same bounded policy surface:
  - direct permutation replay
  - direct graph-move replay
  - one-step structured replay through `expand_frontier_layer` on a single
    stored representative

This still has a maintenance cost because the exact `graph-plus-structured`
loop remains specialized rather than fully unified with mixed BFS.

## After the prototype

From `tmp/search-measure-20260417T114019Z-brix-ruiz-k3-gps-exact-prototype`:

- median wall time `2.07 s`
- wall samples `2.07, 2.09, 2.07, 2.03, 2.08`
- median max RSS `587,204 KB`
- RSS samples `587,248, 584,880, 587,204, 585,504, 587,296`

Representative solver output stayed identical:

- outcome `equivalent`
- witness lag `8`
- `frontier_nodes_expanded = 84,875`
- `total_visited_nodes = 212,170`
- `factorisations_enumerated = 470,662`
- `candidates_generated = 889,746`
- `collisions_with_other_frontier = 1`

## Delta

Median-to-median on the same accepted control:

- wall time `2.18 s -> 2.07 s` (`-0.11 s`, about `-5.0%`)
- max RSS `649,116 KB -> 587,204 KB` (`-61,912 KB`, about `-9.5%`)

The before/after timing bands are also separated enough to look credible on
this control:

- baseline wall range `2.14 s .. 2.21 s`
- prototype wall range `2.03 s .. 2.09 s`

## Correctness checks

Targeted validation on this branch:

- `cargo test -q test_rectangular_sse_constructed -- --test-threads=1`
- `cargo test -q test_graph_plus_structured_exact_reconstructs_deferred_witness -- --test-threads=1`

The measured control itself preserved outcome, witness lag, and search-shape
counters across the five-run before/after comparison.

## Decision

Keep, but scope the claim tightly.

This retry differs from the earlier rejected round because the memory path is
now credible and the focused control shows a real win in both dimensions:

- runtime improved modestly but consistently on the accepted exact control
- max RSS improved materially on the same control
- search shape and final witness stayed unchanged

What this is not:

- evidence for mixed BFS
- evidence for beam or dynamic surfaces
- permission to widen the deferred-witness idea generically

## Recommendation

- keep deferred witness reconstruction for exact `graph-plus-structured` `2x2`
  BFS
- keep the new measurement wrapper as the repeatable wall/RSS path for later
  bounded solver rounds
- do not widen this seam into mixed or dynamic work unless a later round first
  shows a similarly strong measured signal on those exact surfaces
