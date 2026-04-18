# Riedel low-rung witness classification against current graph-only move families (2026-04-18)

## Goal

Classify the smallest practical solved Riedel/Baker witnesses against the
repo's current `graph_only` move vocabulary, starting with `k = 4` and
`k = 6`, and leave one durable statement of what the mismatch currently is.

This note stays narrow:

- witness analysis only;
- no main-search rewrites;
- only `k = 4` and `k = 6`;
- classify concrete witness steps against the move families that exist today.

## Current vocabulary boundary

Current `graph_only` search does **not** permit factorisations
([`src/types.rs`](../../src/types.rs), `MoveFamilyPolicy::permits_factorisations`).
Its one-step move surface is the graph-move enumerator in
[`src/graph_moves.rs`](../../src/graph_moves.rs): `outsplit`, `insplit`,
`out_amalgamation`, `in_amalgamation`.

By contrast, the broader structured surfaces include:

- `rectangular_factorisation_2x3`
- `rectangular_factorisation_3x3_to_2`
- `elementary_conjugation_3x3`
- `diagonal_refactorization_3x3`
- and the larger 3x3/4x4/5x5 split-amalgamation factorisation families

from [`src/factorisation.rs`](../../src/factorisation.rs).

## Keep / Reject decision

Keep as the primary low-rung evidence:

- the `graph_plus_structured` witnesses for `k = 4` and `k = 6`

Reason:

- they solve the retained lane with the same endpoint lag as `mixed`;
- `k = 6` `mixed` is exactly the same witness as `graph_plus_structured`; and
- `k = 4` `mixed` only muddies the picture by swapping two interior steps for
  `mixed`-only `square_factorisation_3x3` moves.

Reject as the primary interpretation for this low-rung slice:

- "the current low-rung mismatch is already an explicit diagonal-refactorization chain"

Reason:

- the kept `graph_plus_structured` witnesses at `k = 4` and `k = 6` contain
  **no** `diagonal_refactorization_3x3` step at all;
- their interior same-dimension moves are `elementary_conjugation_3x3`
  steps plus one permutation-style relabeling step.

So the current low-rung mismatch is broader and simpler:

- the witness begins with a structured `2x2 -> 3x3` rectangular factorisation;
- the witness ends with a structured `3x3 -> 2x2` rectangular factorisation;
- the interior `3x3 -> 3x3` steps are not one-step graph moves either; and
- several of those interior steps do admit a short graph-only expansion, but
  only as a **lag-3** detour through `4x4`, not as a current one-step graph
  family.

## Commands used

Build the existing tooling and the small classifier sidecar:

```bash
cargo build --profile dist --features research-tools --bin search
cargo build --profile dist --features research-tools --bin classify_witness_steps
```

Materialize the low-rung witnesses locally:

```bash
target/dist/search 4,2,1,4 3,1,1,5 \
  --max-lag 5 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-plus-structured \
  --json \
  --write-guide-artifact tmp/riedel_k4_graph_plus_structured_guide.json

target/dist/search 6,2,1,6 5,1,1,7 \
  --max-lag 7 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json \
  --write-guide-artifact tmp/riedel_k6_graph_plus_structured_guide.json

target/dist/search 4,2,1,4 3,1,1,5 \
  --max-lag 5 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy mixed \
  --json \
  --write-guide-artifact tmp/riedel_k4_mixed_guide.json

target/dist/search 6,2,1,6 5,1,1,7 \
  --max-lag 7 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy mixed \
  --json \
  --write-guide-artifact tmp/riedel_k6_mixed_guide.json
```

Classify the witness steps and retain the report:

```bash
target/dist/classify_witness_steps \
  --guide-artifact tmp/riedel_k4_graph_plus_structured_guide.json \
  --guide-artifact tmp/riedel_k6_graph_plus_structured_guide.json \
  --guide-artifact tmp/riedel_k4_mixed_guide.json \
  --guide-artifact tmp/riedel_k6_mixed_guide.json \
  --factorisation-max-entry 12 \
  --graph-probe-max-entry 12 \
  > research/riedel_witness_step_classification_k4_k6_2026-04-18.json
```

The classifier used explicit retained entry caps:

- factorisation matching `max_entry = 12`
- bounded graph-only probe `max_entry = 12`

The bounded graph-only probe also used:

- `max_lag = 3`
- `max_intermediate_dim = 4`

That is intentional: it is large enough to catch the obvious
`3x3 -> 4x4 -> 4x4 -> 3x3` split/amalgamation detours without widening into a
general decomposition search.

## Primary classification: kept `graph_plus_structured` witnesses

### `k = 4` kept witness

| Step | Endpoints | Structured family match | Graph-only classification |
| --- | --- | --- | --- |
| 0 | `2x2 -> 3x3` | `rectangular_factorisation_2x3` | not represented by current one-step graph move families |
| 1 | `3x3 -> 3x3` | `elementary_conjugation_3x3` | needs longer split/amalgamation expansion |
| 2 | `3x3 -> 3x3` | none; graph-only lag-1 permutation-style relabeling only | not represented by current one-step graph move families |
| 3 | `3x3 -> 3x3` | `elementary_conjugation_3x3` | needs longer split/amalgamation expansion |
| 4 | `3x3 -> 2x2` | `rectangular_factorisation_3x3_to_2` | not represented by current one-step graph move families |

Count summary for the kept `k = 4` witness:

- `already graph-coded`: `0`
- `diagonal-refactorization-like`: `0`
- `needs longer split/amalgamation expansion`: `2`
- `not represented by the current one-step graph move families`: `3`

### `k = 6` kept witness

| Step | Endpoints | Structured family match | Graph-only classification |
| --- | --- | --- | --- |
| 0 | `2x2 -> 3x3` | `rectangular_factorisation_2x3` | not represented by current one-step graph move families |
| 1 | `3x3 -> 3x3` | `elementary_conjugation_3x3` | needs longer split/amalgamation expansion |
| 2 | `3x3 -> 3x3` | `elementary_conjugation_3x3` | needs longer split/amalgamation expansion |
| 3 | `3x3 -> 3x3` | none; graph-only lag-1 permutation-style relabeling only | not represented by current one-step graph move families |
| 4 | `3x3 -> 3x3` | `elementary_conjugation_3x3` | needs longer split/amalgamation expansion |
| 5 | `3x3 -> 3x3` | `elementary_conjugation_3x3` | needs longer split/amalgamation expansion |
| 6 | `3x3 -> 2x2` | `rectangular_factorisation_3x3_to_2` | not represented by current one-step graph move families |

Count summary for the kept `k = 6` witness:

- `already graph-coded`: `0`
- `diagonal-refactorization-like`: `0`
- `needs longer split/amalgamation expansion`: `4`
- `not represented by the current one-step graph move families`: `3`

## Concrete mismatch example

The first interior `k = 4` same-dimension step

```text
[[1,3,1],    [[1,2,1],
 [1,3,0], ->  [1,3,0],
 [2,6,4]]     [3,5,4]]
```

is matched directly by `elementary_conjugation_3x3`, not by a one-step graph
family. The bounded graph-only probe does recover it, but only as the lag-3
detour

```text
3x3
[[1,3,1],[1,3,0],[2,6,4]]

-> 4x4
[[1,1,1,2],[1,0,0,3],[2,4,4,2],[1,0,0,3]]

-> 4x4
[[1,2,1,1],[1,3,0,0],[1,3,0,0],[2,2,4,4]]

-> 3x3
[[1,2,1],[1,3,0],[3,5,4]]
```

This is the cleanest low-rung mismatch story to keep:

- the witness segment is real and small;
- it is not one step in today's graph-only vocabulary; but
- it *is* reachable by a short split/amalgamation expansion once `4x4`
  intermediates are allowed.

## Mixed cross-check

`k = 6`:

- the `mixed` witness is identical to the kept `graph_plus_structured` witness,
  so the same mismatch statement applies directly.

`k = 4`:

- the `mixed` witness keeps the same lag and the same endpoint rectangular
  steps;
- its two interior "needs longer expansion" steps are matched by
  `mixed`-only `square_factorisation_3x3` instead of
  `graph_plus_structured`'s `elementary_conjugation_3x3`; and
- that does **not** improve the graph-only story, because those same segments
  still only collapse to graph-only after a lag-3 `4x4` detour.

So for future `5yo.3` / `5yo.4` work, the right durable reading is:

- use the kept `graph_plus_structured` witnesses as the primary low-rung
  mismatch surface;
- do not assume the low-rung obstruction is already an explicit
  diagonal-refactorization chain;
- expect the concrete gap at `k = 4` and `k = 6` to be
  "structured rectangular endpoints plus interior same-dimension steps that
  need short graph-only expansions", not "missing one existing graph-coded
  family edge".
