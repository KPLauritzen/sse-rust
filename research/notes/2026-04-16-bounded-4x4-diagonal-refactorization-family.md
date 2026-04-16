# Bounded explicit diagonal-refactorization family: `4x4 -> 4x4` (2026-04-16)

## Question

What is the smallest same-size `4x4` diagonal-refactorization family worth
adding through the factorisation-family descriptor seam without reopening broad
generic `4x4` same-dimension enumeration?

## Chosen slice

Add exactly one new family:

- label: `diagonal_refactorization_4x4`
- source dimension: `4x4`
- target dimension: `4x4`
- move shape: `A = D X -> B = X D` or `A = X D -> B = D X`

The family is intentionally tighter than the landed `3x3` slice:

- `D` is positive diagonal with entries only in `{1, 2}`;
- scalar diagonals are skipped;
- factors still have to stay within the normal `max_entry` cap;
- only nontrivial same-size moves are emitted.

So the whole seam is just the non-scalar binary diagonals on `4x4`, not a
general bounded-`k` same-dimension sweep.

## Algebra

For a non-scalar binary diagonal

```text
D = diag(d_0, d_1, d_2, d_3),  d_i in {1, 2},
```

emit a move when either:

- `A = D X`, so the next state is `B = X D`, or
- `A = X D`, so the next state is `B = D X`.

Operationally this means:

- row-divide `A` by `D` and re-scale on columns, or
- column-divide `A` by `D` and re-scale on rows,

subject to exact divisibility and the existing factor entry cap.

## Why this seam

- It matches the literature note's open bounded same-size `4x4`
  diagonal-refactorization follow-up.
- It stays in the same explicit-family descriptor seam as the landed
  `diagonal_refactorization_3x3`, row split, and column split slices.
- It is tiny enough to place after the specific `4x4 -> 3x3` and `4x4 -> 5x5`
  rectangular families and before generic same-dimension conjugation.

## Negative boundary

A local sanity check against the published Baker/Lind-Marcus step-5 `4x4`
source/target pair found no direct witness of the form above under tiny
diagonal bounds up to `3`.

So this slice should be read as:

- explicit same-size `4x4` vocabulary coverage;
- not a claim that the missing literal Baker step-5 move is now recovered.

That keeps the family honest and bounded.

## Validation

Focused checks:

- `cargo test -q diagonal_refactorization_4x4`
- `cargo test -q selected_family_labels_for_4x4_keep_specific_before_generic`
- `cargo test -q graph_plus_structured_policy_exposes_diagonal_refactorization_4x4`

Coverage added:

- direct `4x4 -> 4x4` witness test at the factorisation layer
- dispatcher-label test for `diagonal_refactorization_4x4`
- family-order test keeping the new same-size slice between the specific
  `4x4` rectangular families and generic same-dimension conjugation
- frontier telemetry test showing the family participates in
  `GraphPlusStructured` expansion
