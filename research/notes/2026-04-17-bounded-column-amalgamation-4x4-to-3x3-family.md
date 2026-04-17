# Bounded explicit column-amalgamation family: `4x4 -> 3x3` (2026-04-17)

## Question

What is the smallest explicit `4x4 -> 3x3` column-amalgamation sibling worth
adding after the landed row slice, without broadening into generic
multi-column amalgamation or touching search policy?

## Chosen slice

Add exactly one new family:

- label: `single_column_amalgamation_4x4_to_3x3`
- source dimension: `4x4`
- target dimension: `3x3`
- move shape: amalgamate one chosen contiguous source-column pair into one
  target column, leave the other two target columns unchanged

This is the transpose-dual of the landed
`single_row_amalgamation_4x4_to_3x3` seam, and it deliberately stops there:

- only one contiguous source-column pair may amalgamate;
- the matching contiguous source-row pair must already be duplicated;
- no generic multi-column or generic `4x4 -> 3x3` rectangular framework ships
  here;
- no search ranking or policy changes ship here.

## Algebra

For a chosen amalgamation slot `i`, reuse the fixed contiguous duplication
matrix

```text
D_i^T : 4 x 3
```

from the `3x3 -> 4x4` column-splitting sibling.

Apply the transpose-dual of the bounded row-amalgamation construction: run the
existing `4x4 -> 3x3` row-amalgamation enumerator on `A^T`, then transpose the
resulting witness pair back. With factors `A = U V` and `B = V U`, this keeps
the explicit column vocabulary bounded:

1. only one contiguous source-column pair may amalgamate;
2. the matching duplicated source-row block must already be present;
3. the recovered `4x3` / `3x4` witness pair is still expressed through the
   fixed duplication matrix rather than a broader `4x3` solver.

## Why this seam

- It closes the remaining explicit-family gap on the bounded `4x4/3x3`
  corridor without reopening row work or adding a generic framework.
- It gives the column sibling its own stable telemetry/export label:
  `single_column_amalgamation_4x4_to_3x3`.
- It fits naturally in the same factorisation-family descriptor seam as the
  landed row sibling.
- For `(4,3)` exporter fallback metadata, the explicit row/column labels now
  both stay ahead of `binary_sparse_rectangular_factorisation_4x3_to_3`.

## Validation

Focused checks for this slice:

- `cargo test -q single_column_amalgamation_4x4_to_3x3`
- `cargo test -q selected_family_labels_for_4x4_keep_specific_before_generic`
- `cargo test -q --features research-tools --bin export_k3_paths_neo4j fallback_factorisation_families_keep_explicit_4x4_to_3x3_labels_ahead_of_sparse`

Coverage added:

- direct `4x4 -> 3x3` witness coverage at the factorisation layer
- dispatcher-label coverage for `single_column_amalgamation_4x4_to_3x3`
- `GraphPlusStructured` frontier telemetry coverage for the new label
- exporter fallback-order coverage keeping both explicit `(4,3)` labels ahead
  of the broader sparse fallback
