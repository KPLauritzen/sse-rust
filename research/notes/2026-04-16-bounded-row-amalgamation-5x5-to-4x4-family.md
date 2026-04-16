# Bounded explicit row-amalgamation family: `5x5 -> 4x4` (2026-04-16)

## Question

What is the smallest explicit `5x5 -> 4x4` reverse rectangular sibling worth
adding to the main factorisation/search seam without reopening broad generic
`5x4` handling or a multi-row amalgamation framework?

## Chosen slice

Add exactly one new family:

- label: `single_row_amalgamation_5x5_to_4x4`
- source dimension: `5x5`
- target dimension: `4x4`
- move shape: amalgamate one chosen contiguous source-row pair into one target
  row, leave the other three target rows unchanged

This is the direct reverse sibling of the landed
`single_row_split_4x4_to_5x5` slice, and it deliberately stops there:

- only one contiguous source-row pair may amalgamate;
- the matching contiguous source-column pair must already be duplicated;
- no column-amalgamation dual ships in this round;
- no generic `5x5 -> 4x4` row-amalgamation framework or broader `5x4`
  widening ships here.

## Algebra

For a chosen amalgamation slot `i`, reuse the same fixed contiguous duplication
matrix

```text
D_i : 4 x 5
```

from the `4x4 -> 5x5` row-splitting sibling.

Then require the current `5x5` source matrix to already have duplicated columns
`i` and `i+1`. Recover the `5x4` factor `U` by deleting one copy of that
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
   broad `5x4` solve is introduced.

## Why this seam

- It continues the explicit structured-family lane under `sse-rust-2uy.2`
  without broadening the solver to generic `5x4` rectangular moves.
- It gives `5x5 -> 4x4` row-amalgamation vocabulary its own stable telemetry
  label instead of relying only on recovery through the broader sparse family.
- It fits naturally ahead of
  `binary_sparse_rectangular_factorisation_5x5_to_4` in the descriptor order.
- The small `(5,4)` exporter fallback list should name the same explicit label
  ahead of the sparse fallback so downstream telemetry stays consistent outside
  the main dispatcher seam.

## Validation

Focused checks:

- `cargo test -q single_row_amalgamation_5x5_to_4x4`
- `cargo test -q selected_family_labels_for_5x5_keep_specific_before_generic`
- `cargo test -q expand_frontier_layer_graph_plus_structured_exposes_single_row_amalgamation_5x5_to_4x4`
- `cargo test -q --features research-tools --bin export_k3_paths_neo4j fallback_factorisation_families_keep_explicit_5x5_to_4x4_labels_ahead_of_sparse`

Coverage added:

- direct `5x5 -> 4x4` witness test at the factorisation layer
- family-order test keeping the new explicit row-amalgamation family ahead of
  the broader sparse `5x5 -> 4x4` family
- dispatcher-label test for `single_row_amalgamation_5x5_to_4x4`
- `GraphPlusStructured` factorisation-policy witness coverage for the new label
- frontier telemetry test showing the family participates in
  `GraphPlusStructured` expansion
- exporter fallback-list coverage keeping the explicit `(5,4)` labels stable
