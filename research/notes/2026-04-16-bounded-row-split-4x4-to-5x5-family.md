# Bounded explicit row-splitting family: `4x4 -> 5x5` (2026-04-16)

## Question

What is the smallest explicit `4x4 -> 5x5` row-splitting family worth adding to
the main factorisation/search seam without reopening broad `4x5` widening or a
generic multi-row split framework?

## Chosen slice

Add exactly one new family:

- label: `single_row_split_4x4_to_5x5`
- source dimension: `4x4`
- target dimension: `5x5`
- move shape: split one chosen source row into two contiguous clones, leave the
  other three rows unchanged

This is the direct `4x4` rectangular sibling of
`single_row_split_3x3_to_4x4`, and it deliberately stops there:

- only one row may split;
- the split clones must stay contiguous;
- no column-splitting dual ships in this round;
- no generic `4x4 -> 5x5` row-split framework or broader `4x5` widening ships
  here.

## Algebra

For a chosen split row `i`, use the fixed contiguous duplication matrix

```text
U_i : 4 x 5
```

whose row-block sizes are `[1, 1, 1, 1]` except the chosen row gets block size
`2`.

Then write the chosen source row

```text
r_i = p + q
```

with nonzero `p, q in Z_+^4`, and build `V` by replacing row `i` with the two
contiguous rows `p, q`. This gives:

- `A = U_i V`
- `B = V U_i`

so `B` is a literal one-step row split where the matching column block is
duplicated in the same contiguous position.

The family stays bounded in the same explicit way as the landed `3x3 -> 4x4`
slice:

1. only one row may split;
2. mirrored `(p, q)` / `(q, p)` duplicates are suppressed by keeping only the
   lexicographically ordered representative.

## Why this seam

- It continues the explicit structured-family lane under `sse-rust-2uy.2`
  without broadening the solver to generic `4x5` rectangular moves.
- It gives `4x4 -> 5x5` row-splitting vocabulary its own stable telemetry label
  instead of relying on accidental recovery through the broader sparse family.
- It fits naturally ahead of
  `binary_sparse_rectangular_factorisation_4x4_to_5` in the descriptor order,
  while still staying inside the existing family-selection seam.

## Validation

Focused checks:

- `cargo test -q single_row_split_4x4_to_5x5`
- `cargo test -q selected_family_labels_for_4x4_keep_specific_before_generic`
- `cargo test -q expand_frontier_layer_graph_plus_structured_exposes_single_row_split_4x4_to_5x5`

Coverage added:

- direct `4x4 -> 5x5` witness test at the factorisation layer
- family-order test keeping the new explicit row-split family ahead of the
  broader sparse `4x4 -> 5x5` family
- dispatcher-label test for `single_row_split_4x4_to_5x5`
- `GraphPlusStructured` factorisation-policy witness coverage for the new label
- frontier telemetry test showing the family participates in
  `GraphPlusStructured` expansion
