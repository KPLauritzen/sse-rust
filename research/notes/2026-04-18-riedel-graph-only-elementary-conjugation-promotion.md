# Retained Riedel graph-only promotion: `elementary_conjugation_3x3` only (2026-04-18)

## Goal

Promote exactly one smallest theorem-backed slice from the retained low-rung
Riedel decomposition evidence into the main `graph_only` solver, then rerun the
retained benchmark lane and record whether the promotion solves any rung or
improves bounded reach.

This note stays narrow:

- one promoted slice only;
- no broad graph-only rewrite;
- one retained-lane rerun;
- keep the negative result if the lane still does not move.

## Chosen slice

Promoted slice:

- `elementary_conjugation_3x3` as the **only** structured family exposed to the
  dedicated `graph_only` endpoint search on the retained `max_intermediate_dim = 3`
  surface

Code seam:

- graph-only successor enumeration now admits this retained one-step family in
  addition to the existing graph moves
- graph-only witness reconstruction now replays exact one-step witnesses through
  the same bounded policy surface, so promoted edges still validate as ordinary
  ESSE steps
- after the merge-safety follow-up, the promotion is intentionally gated off for
  broader graph-only searches with `max_intermediate_dim > 3`, so the retained
  low-rung lane keeps the promotion while the repo merge check does not pay the
  wider branching cost

Why this slice was chosen over the alternatives:

- the retained note
  [`2026-04-18-riedel-k4-retained-step-decomposition.md`](./2026-04-18-riedel-k4-retained-step-decomposition.md)
  isolates one exact `3x3 -> 3x3` obstruction and matches it directly to
  `elementary_conjugation_3x3`
- this is a bona fide one-step SSE family, so it can be promoted without
  inventing macro-edge witness semantics
- it is smaller than promoting the `2x2 <-> 3x3` rectangular endpoint families,
  and much smaller than promoting target-directed proposal/ranking machinery
- the retained step note already rules out the tempting but wrong narrower read
  that the low-rung gap is "really diagonal refactorization"

Rejected for this pass:

- `rectangular_factorisation_2x3` / `rectangular_factorisation_3x3_to_2`
  endpoint promotion
- graph-only proposal or shortcut-search promotion

Reason:

- both are broader than this same-dimension retained step lift, and the latter
  would also require wider search-stage integration than this bounded pass.

## Focused validation

Focused code validation used for the promotion:

- `cargo test test_selected_family_labels_for_graph_only_3x3_keep_retained_conjugation_only --lib`
- `cargo test test_graph_only_dyn_promotes_retained_elementary_conjugation_step --lib`
- `cargo test test_graph_only_dyn_reconstructs_deferred_witness_on_direct_successor --lib`
- `cargo test test_expand_frontier_layer_graph_only_skips_factorisations --lib`

Formatter note:

- `cargo fmt` and `cargo fmt -p sse-core` were both attempted before commit, but
  each hung under this repo's workmux host-exec path and had to be bounded with
  `timeout 20s`

## Retained-lane rerun

Bounded rerun command:

```bash
cargo build --profile dist --features research-tools --bin research_harness

timeout -k 10s 45s target/dist/research_harness \
  --cases research/riedel_gap_benchmark_lane_2026-04-18.json \
  --format json \
  > tmp/riedel_gap_benchmark_lane_run_2026-04-18_graph_only_promoted_elementary_conjugation.json
```

Retained-lane outcome:

- `graph_only` still solves `0/6` retained rungs
- `graph_plus_structured` still solves `6/6`
- `mixed` still solves `6/6`

`graph_only` rerun details:

| Rung | Outcome | Elapsed |
| --- | --- | --- |
| `k = 4` | `unknown` | `2 ms` |
| `k = 6` | `unknown` | `1 ms` |
| `k = 8` | `unknown` | `1 ms` |
| `k = 10` | `unknown` | `2 ms` |
| `k = 12` | `unknown` | `2 ms` |
| `k = 14` | `unknown` | `2 ms` |

So the retained lane itself did **not** move.

## What Changed Locally But Still Failed Globally

The promotion is active on the retained lane:

- `k = 4`: promoted family generated `166` raw candidates and discovered `65`
  canonical successors
- `k = 6`: generated `492`, discovered `103`
- `k = 8`: generated `676`, discovered `135`
- `k = 10`: generated `850`, discovered `169`
- `k = 12`: generated `1156`, discovered `213`
- `k = 14`: generated `1204`, discovered `227`

But on every retained rung:

- promoted-family exact meets stayed at `0`

That is the durable negative result to keep: the one-step retained
`3x3 -> 3x3` lift increases bounded low-rung graph-only exploration, but it
does **not** by itself bridge the retained lane.

## Exact Remaining Blockage

What stayed blocked after the promotion:

- the retained lane still has no direct `graph_only` lift for the
  `2x2 -> 3x3` and `3x3 -> 2x2` rectangular endpoint steps identified in
  [`2026-04-18-riedel-witness-classification-k4-k6.md`](./2026-04-18-riedel-witness-classification-k4-k6.md)
- the promoted same-dimension `3x3` motion therefore fires inside the bounded
  `3x3` surface, but the retained bidirectional search still never reaches a
  matching bridge across the endpoint obstruction within the fixed lane caps
  `lag <= 5,6,8,11,12,14` and `max_intermediate_dim = 3`

So the right follow-up reading is:

- this pass confirmed that the retained low-rung interior `3x3` conjugation
  slice is real and promotable; but
- the retained lane remains blocked first by the unresolved rectangular
  endpoint lifts, not by the absence of this interior same-dimension step.
