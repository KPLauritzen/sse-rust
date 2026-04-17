# Bounded explicit row-amalgamation family: `4x4 -> 3x3` (2026-04-17)

## Question

What is the smallest explicit `4x4 -> 3x3` reverse rectangular sibling worth
adding to the main factorisation/search seam without broadening into the column
dual or a generic `4x4 -> 3x3` amalgamation framework?

## Chosen slice

Add exactly one new family:

- label: `single_row_amalgamation_4x4_to_3x3`
- source dimension: `4x4`
- target dimension: `3x3`
- move shape: amalgamate one chosen contiguous source-row pair into one target
  row, leave the other two target rows unchanged

This is the direct reverse sibling of the landed
`single_row_split_3x3_to_4x4` seam, and it deliberately stops there:

- only one contiguous source-row pair may amalgamate;
- the matching contiguous source-column pair must already be duplicated;
- no column-amalgamation sibling ships in this round;
- no generic multi-row or generic `4x4 -> 3x3` rectangular framework ships
  here.

## Algebra

For a chosen amalgamation slot `i`, reuse the fixed contiguous duplication
matrix

```text
D_i : 3 x 4
```

from the `3x3 -> 4x4` row-splitting sibling.

Then require the current `4x4` source matrix to already have duplicated columns
`i` and `i+1`. Recover the `4x3` factor `U` by deleting one copy of that
duplicated column block. With `V = D_i`, this gives:

- `A = U V`
- `B = V U`

so `B` is a literal one-step row amalgamation: rows `i` and `i+1` of `U` sum
into one target row, while the other rows pass through unchanged.

The bounded family keeps the reverse seam explicit:

1. only one contiguous source-row pair may amalgamate;
2. the chosen source pair must stay nonzero, matching the nondegenerate split
   pieces required by the forward sibling;
3. the matching duplicated source-column block must already be present, so no
   broad `4x3` solve is introduced.

## Why this seam

- It fills the obvious missing explicit corridor in the current `4x4/3x3`
  lane without changing search policy or ranking.
- It gives `4x4 -> 3x3` row-amalgamation vocabulary its own stable telemetry
  label instead of relying only on the broader sparse family.
- It fits naturally ahead of
  `binary_sparse_rectangular_factorisation_4x3_to_3` in both the main
  factorisation-family descriptor list and the Neo4j exporter fallback list for
  `(4,3)`.
- The remaining explicit-family gap after this slice is the column-amalgamation
  sibling, not a generic framework change.

## Validation

Focused checks for this slice:

- `cargo test -q single_row_amalgamation_4x4_to_3x3`
- `cargo test -q selected_family_labels_for_4x4_keep_specific_before_generic`
- `cargo test -q --features research-tools --bin export_k3_paths_neo4j fallback_factorisation_families_keep_explicit_4x4_to_3x3_labels_ahead_of_sparse`

Coverage added:

- direct `4x4 -> 3x3` witness coverage at the factorisation layer
- dispatcher-label coverage for `single_row_amalgamation_4x4_to_3x3`
- `GraphPlusStructured` frontier telemetry coverage for the new label
- exporter fallback-order coverage keeping the explicit `(4,3)` label ahead of
  the broader sparse fallback
