# Bounded explicit column-splitting family: `4x4 -> 5x5` (2026-04-16)

## Question

What is the smallest explicit `4x4 -> 5x5` column-splitting family worth adding
to the main factorisation/search seam without reopening broad generic `4x5`
widening or a multi-column framework?

## Chosen slice

Add exactly one new family:

- label: `single_column_split_4x4_to_5x5`
- source dimension: `4x4`
- target dimension: `5x5`
- move shape: split one chosen source column into two contiguous clones, leave
  the other three columns unchanged

This is the direct transpose-dual sibling of the landed
`single_row_split_4x4_to_5x5` slice, and it deliberately stops there:

- only one column may split;
- the split clones must stay contiguous;
- no row-splitting follow-up ships in this round because that sibling already
  landed;
- no generic `4x4 -> 5x5` column-split framework or broader `4x5` widening
  ships here.

## Algebra

For a chosen split column `j`, reuse the bounded row-splitting construction on
the transpose.

Let

```text
D_j : 4 x 5
```

be the fixed contiguous row-duplication matrix for clone counts
`[2,1,1,1]`, `[1,2,1,1]`, `[1,1,2,1]`, or `[1,1,1,2]` depending on the chosen
slot. Then the column-splitting family uses

```text
V_j = D_j^T : 5 x 4.
```

Write the chosen source column

```text
c_j = p + q
```

with nonzero `p, q in Z_+^4`, and build `U` by replacing column `j` with the
two contiguous columns `p, q`. Then:

- `A = U V_j`
- `B = V_j U`

so `B` is a literal one-step column split where the matching row block is
duplicated in the same contiguous position.

The implementation stays bounded exactly like the row-splitting sibling:

1. only one column may split;
2. mirrored `(p, q)` / `(q, p)` duplicates are suppressed by inheriting the
   lexicographically ordered representative from the transpose-dual row-split
   enumerator.

## Why this seam

- It continues the explicit structured-family lane under `sse-rust-2uy.2`
  without broadening the solver to generic `4x5` rectangular moves.
- It gives `4x4 -> 5x5` column-splitting vocabulary its own stable telemetry
  label instead of hoping the move is only recovered through the broader sparse
  family.
- It fits naturally after `single_row_split_4x4_to_5x5` and ahead of
  `binary_sparse_rectangular_factorisation_4x4_to_5` in the descriptor order.
- The small `(4,5)` exporter fallback list should name the same explicit
  families in the same order so downstream telemetry stays consistent outside
  the main dispatcher seam.

## Validation

Focused checks:

- `cargo test -q single_column_split_4x4_to_5x5`
- `cargo test -q selected_family_labels_for_4x4_keep_specific_before_generic`
- `cargo test -q expand_frontier_layer_graph_plus_structured_exposes_single_column_split_4x4_to_5x5`
- `cargo test -q --bin export_k3_paths_neo4j fallback_factorisation_families_keep_explicit_4x4_to_5x5_labels_ahead_of_sparse`

Coverage added:

- direct `4x4 -> 5x5` witness test at the factorisation layer
- family-order test keeping the new explicit column-split family ahead of the
  broader sparse `4x4 -> 5x5` family
- dispatcher-label test for `single_column_split_4x4_to_5x5`
- `GraphPlusStructured` factorisation-policy witness coverage for the new label
- frontier telemetry test showing the family participates in
  `GraphPlusStructured` expansion
- exporter fallback-list coverage keeping the explicit `(4,5)` labels stable
