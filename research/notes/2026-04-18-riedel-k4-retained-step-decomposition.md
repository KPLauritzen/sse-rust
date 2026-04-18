# Retained `k = 4` interior step: bounded graph decomposition sidecar (2026-04-18)

## Goal

Keep one durable, explicit decomposition record for the retained low-rung
Riedel witness step singled out in
[`2026-04-18-riedel-witness-classification-k4-k6.md`](./2026-04-18-riedel-witness-classification-k4-k6.md).

This slice stays narrow:

- one concrete retained `3x3 -> 3x3` interior step from the kept `k = 4`
  witness;
- no solver rewrite;
- no generic decomposition framework; and
- evidence first: exact matrices, exact bounded path, exact remaining failure
  point.

## Step studied

The retained step is the first same-dimension interior step of the kept
`graph_plus_structured` `k = 4` witness:

```text
A =
[[1,3,1],
 [1,3,0],
 [2,6,4]]

B =
[[1,2,1],
 [1,3,0],
 [3,5,4]]
```

The bounded sidecar report for this exact pair is committed as:

- [`research/riedel_k4_retained_step_decomposition_2026-04-18.json`](../riedel_k4_retained_step_decomposition_2026-04-18.json)

and is produced by the research-only helper:

- [`src/bin/explain_witness_step.rs`](../../src/bin/explain_witness_step.rs)

## Direct interpretation

For this exact pair, the sidecar report shows:

- no exact one-step graph family match;
- `graph_plus_structured` matches it as `elementary_conjugation_3x3`;
- no `diagonal_refactorization_3x3` match appears at the same bounded exact
  factorisation surface; and
- under `mixed`, the same exact pair also admits `square_factorisation_3x3`,
  which is another reason to keep the retained reading anchored on the
  `graph_plus_structured` witness rather than the noisier mixed surface.

So the right bounded reading for this concrete retained step is still:

- it is **not** already a one-step graph move;
- it is **not** explained here by a direct diagonal-refactorization lift; but
- it **is** explained by a short graph-only detour through `4x4`.

## Explicit bounded graph decomposition

Within the same bounded probe envelope used in the witness-classification note
(`lag <= 3`, `max_intermediate_dim = 4`, `max_entry = 12`), the helper recovers
the explicit graph-only path

```text
A
-> C1
-> C2
-> B
```

with

```text
C1 =
[[1,1,1,2],
 [1,0,0,3],
 [2,4,4,2],
 [1,0,0,3]]

C2 =
[[1,2,1,1],
 [1,3,0,0],
 [1,3,0,0],
 [2,2,4,4]]
```

and the hop-by-hop interpretation:

1. `A -> C1`: `insplit`
   This is an elementary column split.
   Local evidence: `C1` has duplicate rows `2` and `4`.
2. `C1 -> C2`: `permutation_relabeling`
   This is the graph-isomorphism middle hop, with one-based permutation
   `[1,3,4,2]`.
3. `C2 -> B`: `out_amalgamation`
   This is an elementary row amalgamation.
   Local evidence: `C2` has duplicate columns `3` and `4`, and those are the
   columns merged on the final hop.

So the bounded explicit explanation is:

- column split to `4x4`;
- graph relabeling inside `4x4`; then
- row amalgamation back to `3x3`.

This is the concrete sidecar evidence to carry forward, not a theorem-only
placeholder.

## Failure boundary kept for later work

This note also records the local negative result that still matters for later
`5yo.4 / 5yo.5` work:

- there is no exact one-step graph move from `A` to `B`;
- there is no direct `diagonal_refactorization_3x3` match at the bounded exact
  factorisation surface used here; and
- the bounded explanation currently needs the short `4x4` detour rather than
  collapsing directly inside `3x3`.

That is the precise local obstruction still left after this sidecar pass.

## Reproduce

Build the helper:

```bash
cargo build --profile dist --features research-tools --bin explain_witness_step
```

Validate the helper logic only:

```bash
cargo test --features research-tools --bin explain_witness_step
```

Regenerate the committed decomposition artifact:

```bash
target/dist/explain_witness_step \
  --from 3x3:1,3,1,1,3,0,2,6,4 \
  --to 3x3:1,2,1,1,3,0,3,5,4 \
  --graph-max-lag 3 \
  --graph-max-intermediate-dim 4 \
  --graph-max-entry 12 \
  --factorisation-max-entry 12 \
  --write-json research/riedel_k4_retained_step_decomposition_2026-04-18.json \
  > tmp/riedel_k4_retained_step_decomposition_stdout.json
```

The retained output should report:

- `graph_plus_structured_families = ["elementary_conjugation_3x3"]`
- `has_diagonal_refactorization_match = false`
- `graph_only_explanation.outcome = "equivalent"`
- `graph_only_explanation.lag = 3`
- hop families
  `["insplit", "permutation_relabeling", "out_amalgamation"]`
