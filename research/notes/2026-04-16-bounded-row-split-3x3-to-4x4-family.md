# Bounded explicit row-splitting family: `3x3 -> 4x4` (2026-04-16)

## Question

What is the smallest explicit row-splitting family worth adding to the main
factorisation/search seam without recreating the broad `3x4` widening that the
solver already gets from generic rectangular enumeration?

## Chosen slice

Add exactly one new family:

- label: `single_row_split_3x3_to_4x4`
- source dimension: `3x3`
- target dimension: `4x4`
- move shape: split one chosen source row into two contiguous clones, leave the
  other two rows unchanged

This keeps the round aligned with the current `3x3/4x4` gap and avoids bundling
the larger `4x4 -> 5x5` dual or a generic multi-row split framework.

## Algebra

For a chosen split row `i`, use the fixed contiguous duplication matrix

```text
U_i : 3 x 4
```

whose row-block sizes are `[1, 1, 1]` except the chosen row gets block size
`2`. Concretely:

- `i = 0`: clone counts `[2, 1, 1]`
- `i = 1`: clone counts `[1, 2, 1]`
- `i = 2`: clone counts `[1, 1, 2]`

Then write the chosen source row

```text
r_i = p + q
```

with nonzero `p, q in Z_+^3`, and build `V` by replacing row `i` with the two
contiguous rows `p, q`. This gives:

- `A = U_i V`
- `B = V U_i`

so `B` is a literal one-step row split where the matching column block is
duplicated in the same contiguous position.

The implementation keeps the family bounded in two ways:

1. only one row may split;
2. mirrored `(p, q)` / `(q, p)` duplicates are suppressed by keeping only the
   lexicographically ordered representative.

## Why this seam

- It is explicit row-splitting vocabulary in the main search, not accidental
  recovery through generic factorisation.
- It is still small enough to sit ahead of
  `binary_sparse_rectangular_factorisation_3x3_to_4` in the descriptor order.
- The helper duplication-matrix construction matches the same contiguous block
  algebra as the existing literature sanity anchor in `src/search.rs`: the
  `2x2 -> 5x5` fixture corresponds to clone counts `[3, 2]`.

## Validation

Focused checks:

- `cargo test -q single_row_split`
- `cargo test -q selected_family_labels_for_`
- `cargo test -q graph_plus_structured_policy_exposes_`

Coverage added:

- family-order tests for `MoveFamilyPolicy::Mixed` and
  `MoveFamilyPolicy::GraphPlusStructured`
- helper test matching the literature-style `[3, 2]` duplication matrix
- explicit `3x3 -> 4x4` witness test at the factorisation layer
- dispatcher-label test for `single_row_split_3x3_to_4x4`
- frontier telemetry test showing the family participates in
  `GraphPlusStructured` expansion even when canonical dedup retains an
  equivalent successor from another family
