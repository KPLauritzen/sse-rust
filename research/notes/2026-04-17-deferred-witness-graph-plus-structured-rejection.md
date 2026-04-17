# Deferred witness round: graph-plus-structured exact BFS does not justify a keep (2026-04-17)

## Question

After the kept graph-only round, should deferred witness reconstruction be
extended to a bounded non-graph-only surface?

This round stayed intentionally narrow:

- exact endpoint-search BFS only
- no beam or beam-to-BFS-handoff changes
- no mixed-wide refactor
- start with measurement before code

## Surfaces measured first

### Mixed control: hard Brix-Ruiz endpoint search

Command:

```bash
target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 6 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy mixed \
  --telemetry --json
```

Observed baseline:

- outcome `unknown`
- wall time `0.440 s`
- `frontier_nodes_expanded = 4,148`
- `total_visited_nodes = 5,141`
- `factorisations_enumerated = 354,093`

Interpretation:

- mixed endpoint BFS is still witnessful, but on this accepted control the
  stored-parent seam is not obviously dominant
- this is too small to justify widening the round into generic mixed BFS first

### Graph-plus-structured exact control: hard Brix-Ruiz solve baseline

Command:

```bash
target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 8 \
  --max-intermediate-dim 4 \
  --max-entry 5 \
  --move-policy graph-plus-structured \
  --telemetry --json
```

Observed baseline:

- outcome `equivalent`
- wall time `2.217 s`
- witness lag `8`
- `frontier_nodes_expanded = 84,875`
- `total_visited_nodes = 212,170`
- `factorisations_enumerated = 470,662`
- `candidates_generated = 889,746`
- `collisions_with_other_frontier = 1`

Interpretation:

- this is the only non-graph-only surface in the round that looked plausibly
  hot enough for parent witness storage to matter
- it is still much smaller than the kept graph-only exact control from the
  prior round, and it spends substantial work in structured factorisation
  enumeration

### Dynamic control: 1x1 -> 4x4 higher-block known pair

Command:

```bash
target/dist/search 1x1:2 4x4:1,1,0,0,0,0,1,1,1,1,0,0,0,0,1,1 \
  --max-lag 4 \
  --max-intermediate-dim 4 \
  --max-entry 2 \
  --move-policy graph-plus-structured \
  --telemetry --json
```

Observed baseline:

- outcome `equivalent`
- wall time `0.006 s`
- witness lag `4`
- `frontier_nodes_expanded = 5`
- `total_visited_nodes = 15`

Interpretation:

- the generic dynamic seam is cold on the bounded accepted control
- do not widen this round into generic dynamic reconstruction work

## Prototype

Prototype only, later reverted:

- specialized exact `graph_plus_structured` `2x2` BFS to store parent canon
  only: `HashMap<DynMatrix, Option<DynMatrix>>`
- kept observer emission exact by continuing to use the live `expansion.step`
  payload when emitting `SearchEdgeRecord`s
- reconstructed final steps only after a successful meet by replaying adjacent
  matrix pairs with:
  - permutation replay
  - direct graph-move replay
  - policy-bounded factorisation replay

This preserved correctness on the prototype branch, but it required a large
specialized BFS body to separate parent-map types cleanly from the existing
generic witnessful path.

## Prototype measurements

Same exact graph-plus-structured control after the prototype:

- outcome `equivalent`
- wall time `2.213 s`
- witness lag `8`
- `frontier_nodes_expanded = 84,875`
- `total_visited_nodes = 212,170`
- `factorisations_enumerated = 470,662`
- `candidates_generated = 889,746`
- `collisions_with_other_frontier = 1`

Same dynamic control after the prototype:

- outcome `equivalent`
- wall time `0.013 s`
- witness lag `4`
- `frontier_nodes_expanded = 5`
- `total_visited_nodes = 15`

Interpretation:

- the hot `graph_plus_structured` control was functionally identical and
  runtime-flat (`2.217 s -> 2.213 s`)
- the dynamic control stayed tiny and slightly noisier, with no meaningful
  reason to widen the work
- without a measured RSS win, the maintenance cost of the specialized BFS path
  is not justified

## Decision

Reject for now. Keep the graph-only deferred-witness optimization narrow.

Why:

- mixed endpoint BFS was too small on the accepted control to justify widening
- the dynamic seam was cold
- the only plausible hot surface (`graph_plus_structured` exact BFS) showed no
  meaningful runtime improvement after a specialized deferred-parent prototype
- the required code shape duplicated too much exact-search logic for a
  measurement-flat result

## Recommendation

- keep the previously merged graph-only deferred-witness change as-is
- do not extend deferred witness reconstruction into `graph_plus_structured`,
  mixed, or dynamic BFS without direct memory evidence on a hot accepted
  surface
- if a later round wants to revisit this, it should first secure a trustworthy
  RSS measurement path for the `graph_plus_structured` exact control before
  paying the complexity cost again
