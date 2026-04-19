# Retained `k = 4` interior bridge: guided replay control on the narrow lane (2026-04-19)

## Goal

Classify the retained Riedel/Baker interior `3x3 -> 3x3` obstruction on the
committed narrow lane:

- `lag <= 3`
- `max_intermediate_dim <= 3`
- `max_entry <= 4`

and decide whether it is intrinsically blocked there, or whether the committed
`max_entry = 5` guide artifact can already be consumed on a retained-only
replay/control surface without changing plain graph-only policy.

## Exact obstruction

The retained interior bridge is the same exact pair isolated on 2026-04-18:

```text
A = [[1,3,1],    B = [[4,4,4],
     [1,3,0],         [1,1,1],
     [2,6,4]]         [0,1,3]]
```

The committed guide artifact remains:

- [`research/riedel_k4_retained_interior_bridge_entry5_threshold_guide_2026-04-18.json`](../riedel_k4_retained_interior_bridge_entry5_threshold_guide_2026-04-18.json)

## What the current surfaces already do

Plain narrow-lane endpoint search remains blocked:

```bash
timeout -k 10s 60s target/release/search \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:4,4,4,1,1,1,0,1,3 \
  --max-lag 3 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-only \
  --json
# outcome: unknown
```

The same CLI already consumes the committed guide artifact on the retained-only
guided stage without changing any pruning rule:

```bash
timeout -k 10s 60s target/release/search \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:4,4,4,1,1,1,0,1,3 \
  --stage guided-refinement \
  --guide-artifacts research/riedel_k4_retained_interior_bridge_entry5_threshold_guide_2026-04-18.json \
  --max-lag 3 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-only \
  --guided-max-shortcut-lag 1 \
  --guided-min-gap 2 \
  --guided-max-gap 3 \
  --guided-rounds 1 \
  --json
# outcome: equivalent
```

Why this works:

- [`src/search/stages.rs`](../../src/search/stages.rs) accepts compatible
  `full_path` artifacts through `prepare_full_path_guide`;
- the artifact is re-anchored to the requested endpoints and validated exactly
  as an SSE path;
- `guided_refinement` starts from that exact path and only tries to shorten it;
  if no shorter narrow-lane shortcut exists, it still returns the original
  retained path; and
- no change to plain `endpoint_search`, general `max_entry` pruning, or graph-only
  family policy is required.

So the artifact is already consumable on a retained-only replay surface; there
is no missing implementation seam in the guided stages themselves.

## Durable control added

To keep that reading reusable in the research harness, this slice adds two
focused cases in [`research/cases.json`](../cases.json):

- `riedel_k4_retained_interior_bridge_narrow_lane`
  - exact retained bridge
  - plain `graph-only` `endpoint_search`
  - expected outcome `unknown`
- `riedel_k4_retained_interior_bridge_guided_replay`
  - same endpoints and same narrow-lane config
  - `guided_refinement` seeded from the committed retained guide artifact
  - expected outcome `equivalent`

This is the retained-only control surface to keep for later work.

## Keep / Reject

Keep:

- plain graph-only narrow-lane search is still blocked on this bridge under
  `max_entry <= 4`;
- the committed `max_entry = 5` artifact is already replayable on an exact
  retained-only guided surface; and
- the right durable control is a narrow baseline case plus a guide-backed replay
  case, not a policy rewrite.

Reject:

- widening general graph-only family policy;
- changing general `max_entry` pruning semantics so the first `entry = 5`
  intermediate is admitted to ordinary narrow-lane search; and
- treating the guide artifact as evidence that plain narrow-lane graph-only
  endpoint search is no longer blocked.

## Final reading

Final reading: **guide/replay explainable on a retained-only surface**.

That reading includes an important negative boundary:

- the obstruction remains **intrinsically blocked for plain narrow-lane
  endpoint_search** under `lag <= 3`, `max_intermediate_dim <= 3`, and
  `max_entry <= 4`; but
- it is **not** intrinsically blocked for retained explanation/replay, because
  the existing guided stage can already consume the committed artifact exactly.
