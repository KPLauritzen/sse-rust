# Retained `k = 4` interior `3x3 -> 3x3` bridge: `max_entry = 4` vs `5` threshold (2026-04-18)

## Goal

Classify the retained graph-only interior obstruction from
[`2026-04-18-riedel-graph-only-rectangular-endpoint-promotion.md`](./2026-04-18-riedel-graph-only-rectangular-endpoint-promotion.md)
that stays `unknown` at `max_entry = 4` but flips to `equivalent` at
`max_entry = 5`, and keep the smallest durable explanation.

Exact obstruction:

```text
A = [[1,3,1],    B = [[4,4,4],
     [1,3,0],         [1,1,1],
     [2,6,4]]         [0,1,3]]
```

## Reproduce

Direct retained probes:

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

timeout -k 10s 60s target/release/search \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:4,4,4,1,1,1,0,1,3 \
  --max-lag 3 \
  --max-intermediate-dim 3 \
  --max-entry 5 \
  --move-policy graph-only \
  --json \
  --write-guide-artifact \
  research/riedel_k4_retained_interior_bridge_entry5_threshold_guide_2026-04-18.json
# outcome: equivalent
```

Focused one-hop probes that isolate the threshold:

```bash
timeout -k 10s 30s target/release/search \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:1,2,1,1,3,0,3,5,4 \
  --max-lag 1 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-only \
  --json
# outcome: unknown

timeout -k 10s 30s target/release/search \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:1,2,1,1,3,0,3,5,4 \
  --max-lag 1 \
  --max-intermediate-dim 3 \
  --max-entry 5 \
  --move-policy graph-only \
  --json
# outcome: equivalent

timeout -k 10s 30s target/release/search \
  3x3:4,3,5,1,1,2,0,1,3 \
  3x3:4,4,4,1,1,1,0,1,3 \
  --max-lag 1 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-only \
  --json
# outcome: equivalent
```

## Observed threshold

At `max_entry = 5`, the retained bridge is exactly the previously retained
three-step interior chain

```text
A
-> M1 = [[1,2,1],
         [1,3,0],
         [3,5,4]]
-> M2 = [[4,3,5],
         [1,1,2],
         [0,1,3]]
-> B
```

The committed sidecar guide is:

- [`research/riedel_k4_retained_interior_bridge_entry5_threshold_guide_2026-04-18.json`](../riedel_k4_retained_interior_bridge_entry5_threshold_guide_2026-04-18.json)

Step classification under the same bounded `3x3` surface:

- `A -> M1`: direct lag-1 `graph_only` move at `max_entry = 5`, matched by
  `elementary_conjugation_3x3` on the structured surface
- `M1 -> M2`: lag-1 permutation relabeling
- `M2 -> B`: direct lag-1 `graph_only` move already available at `max_entry = 4`
  and also matched by `elementary_conjugation_3x3`

So the full `4` vs `5` threshold is carried entirely by the **first** hop
`A -> M1`.

## Smallest explanation

The minimal blocker is **not** a factor entry that first reaches `5`.

For the critical hop `A -> M1`, the retained lag-1 witness recovered at
`max_entry = 5` is:

```text
U = [[1,2,1],
     [1,3,0],
     [2,2,4]]

V = [[1,0,0],
     [0,1,0],
     [0,1,1]]
```

Both factor matrices stay within entry `4`.

What crosses the threshold is the realized successor matrix itself:

```text
M1 = VU = [[1,2,1],
           [1,3,0],
           [3,5,4]]
```

and `M1` has `max_entry = 5`.

That matches the main search pruning seam in [`src/search.rs`](../../src/search.rs):

- graph-only successor expansion admits the candidate step first; then
- the search discards successors with
  `successor.orig_matrix.max_entry() > config.max_entry`.

So under `max_entry = 4`, the first retained interior successor is generated
but not admitted to the frontier, which means the later permutation and final
`M2 -> B` hop never become reachable.

## Keep / Reject

Keep:

- the bounded explanation "`max_entry = 5` is needed because the first retained
  intermediate matrix `M1` itself has entry `5`"

Reject:

- "the threshold comes from a new `4x4` detour"
- "the threshold comes from a factor entry first reaching `5`"
- "the threshold needs a broad graph-only rewrite"

## Smallest next step

Treat this as **evidence-only** unless later work explicitly wants a retained
code slice.

If a code-facing follow-up is needed, the smallest safe slice is:

- consume the committed retained guide artifact for this exact bridge in a
  retained-only research path/guide seam; and
- do **not** change general `max_entry` pruning or widen graph-only family
  policy just to admit `M1` under the committed `max_entry = 4` bound.

That keeps the result merge-safe and targeted to the single retained
obstruction.
