# Layer-contrast held-out family policy probe (2026-04-16)

## Question

Can one narrow endpoint-case broadening pass make either remaining held-out
literature family rankable for the layer-contrast corpus without changing the
general analyzer pipeline?

Target families in scope:

- `lind_marcus`
- `higher_block`

## Slice

Probe exactly one bounded broadening idea:

- keep the same endpoint pairs and family/dedup manifests;
- vary only the endpoint-case `move_family_policy`;
- stop if the current analyzer surface still emits zero rankable layers.

This stays inside the existing layer-contrast extraction lane rather than
adding a new benchmark surface or broader fitting pipeline.

## Probe setup

Used `target/debug/analyze_path_signal_corpus` after a local
`cargo build --quiet --features research-tools --bin analyze_path_signal_corpus`
build, with one-case temporary corpora derived from `research/cases.json`.

For each family case, ran:

- existing `mixed` policy from `research/cases.json`;
- `graph_only`;
- `graph_plus_structured`.

Shared command shape:

```bash
target/debug/analyze_path_signal_corpus \
  --cases <one-case temp corpus>.json \
  --case-id <case id> \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts tmp/<probe>.json
```

## Results

### `lind_marcus_a_to_c`

- `mixed`: solved lag `2`, `solution_nodes = 1`, exported rankable layers `0`
- `graph_only`: solved lag `2`, `solution_nodes = 1`, exported rankable layers `0`
- `graph_plus_structured`: solved lag `2`, `solution_nodes = 1`, exported rankable layers `0`

Under the current analyzer collector, all three runs retained `layer_count = 0`.

### `full_2_shift_higher_block_1x1_to_4x4`

- `mixed`: solved lag `4`, `solution_nodes = 2`, exported rankable layers `0`
- `graph_only`: solved lag `3`, `solution_nodes = 2`, exported rankable layers `0`
- `graph_plus_structured`: solved lag `4`, `solution_nodes = 2`, exported rankable layers `0`

Under the current analyzer collector, all three runs retained `layer_count = 0`.

## Interpretation

This bounded policy sweep did **not** make either additional held-out family
rankable.

The important negative signal is not just "still zero labels"; it is that the
current `analyze_path_signal_corpus` layer collector still retained no usable
observer layers for either family under any tested policy variant. The current
collector only keeps enqueued or `ExactMeet` edges from each observed layer, so
these endpoint solves do not presently expose a sibling set that the
layer-contrast export can intersect with the returned witness path.

## Decision

Do not update the durable layer-contrast artifact for this round.

Reason:

- the probe produced no new rankable cases,
- no new held-out family coverage,
- and no nontrivial `supporting_continuation` labels.

The next useful follow-up, if this lane is revisited, should target the
observer/extraction seam directly rather than repeating more endpoint-family
policy sweeps on the same current collector.
