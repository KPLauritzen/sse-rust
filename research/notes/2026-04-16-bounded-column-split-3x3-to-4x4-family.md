# Bounded explicit column-splitting family: `3x3 -> 4x4` (2026-04-16)

## Question

What is the smallest explicit column-splitting family worth adding to the main
factorisation/search seam without reopening the broad generic `3x4`
rectangular enumeration that the solver already has elsewhere?

## Chosen slice

Add exactly one new family:

- label: `single_column_split_3x3_to_4x4`
- source dimension: `3x3`
- target dimension: `4x4`
- move shape: split one chosen source column into two contiguous clones, leave
  the other two columns unchanged

This is the concrete sibling of the landed bounded
`single_row_split_3x3_to_4x4` slice. It stays focused on the same `3x3/4x4`
gap and does not introduce a broader rectangular framework.

## Algebra

For a chosen split column `j`, use the same contiguous duplication pattern as
the row-splitting slice, but transposed.

Let

```text
D_j : 3 x 4
```

be the fixed row-splitting duplication matrix for clone counts `[2,1,1]`,
`[1,2,1]`, or `[1,1,2]` depending on the chosen slot. Then the column-splitting
family uses

```text
V_j = D_j^T : 4 x 3.
```

Write the chosen source column

```text
c_j = p + q
```

with nonzero `p, q in Z_+^3`, and build `U` by replacing column `j` with the
two contiguous columns `p, q`. Then:

- `A = U V_j`
- `B = V_j U`

so `B` is a literal one-step column split where the matching row block is
duplicated in the same contiguous position.

The implementation keeps the family bounded in two ways:

1. only one column may split;
2. mirrored `(p, q)` / `(q, p)` duplicates are suppressed by inheriting the
   lexicographically ordered representative from the transpose-dual row-split
   enumerator.

## Why this seam

- It keeps column splitting explicit in the main search instead of hoping the
  move appears accidentally inside generic factorisation enumeration.
- It matches the same bounded `3x3 -> 4x4` scope as the row split, so the two
  structured slices now cover both rectangular orientations without widening
  beyond the current target surface.
- The transpose-dual implementation is algebraically clean and keeps the new
  family tightly coupled to the already documented contiguous-duplication model.

## Validation

Focused checks:

- `cargo test -q single_column_split`
- `cargo test -q selected_family_labels_for_`

Coverage added:

- family-order tests for `MoveFamilyPolicy::Mixed` and
  `MoveFamilyPolicy::GraphPlusStructured`
- explicit `3x3 -> 4x4` witness test at the factorisation layer
- dispatcher-label test for `single_column_split_3x3_to_4x4`
- frontier telemetry test showing the family participates in
  `GraphPlusStructured` expansion
