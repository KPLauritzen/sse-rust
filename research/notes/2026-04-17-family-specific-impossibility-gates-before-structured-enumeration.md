# Exact family-specific impossibility gates before structured enumeration (2026-04-17)

## Question

For bead `sse-rust-ilu.1`, is there a small exact source-node gate that can
reject an entire structured move family before its enumerator runs, especially
near the current `3x3` hotspot around `square_factorisation_3x3` and the newer
bounded structured families?

This round stayed research-first and bounded. The target was not a broad search
refactor. The target was one exact family gate worth keeping, one rejected
candidate, or a durable next-step note.

## Sources examined

- literature direction note:
  [2026-04-17-exact-pruning-obstruction-literature-survey.md](2026-04-17-exact-pruning-obstruction-literature-survey.md)
- current `square_factorisation_3x3` hotspot/profile notes:
  [2026-04-17-early-canonical-dedup-square-factorisation-orbits.md](2026-04-17-early-canonical-dedup-square-factorisation-orbits.md)
- current structured-family definitions in
  [src/factorisation.rs](../../src/factorisation.rs)

Bounded local evidence came from:

```bash
cargo run --features research-tools --bin profile_square_factorisation_sources --quiet
```

plus one local brute-force computation of the determinant set for `3x3`
cap-`4` factor matrices.

## Families examined

- `square_factorisation_3x3`
- `single_row_split_3x3_to_4x4`
- `single_column_split_3x3_to_4x4`
- `diagonal_refactorization_3x3`
- `diagonal_refactorization_4x4`

## Candidate 1: determinant-factor gate for `square_factorisation_3x3`

### Candidate

The `square_factorisation_3x3` family only emits witnesses

```text
A = U V
```

with `U, V in {0, ..., 4}^{3x3}` because the family hard-caps the factor entry
bound at `4`. Therefore any emitted witness satisfies

```text
det(A) = det(U) det(V).
```

So an exact source-node gate is:

- precompute `D_4 = { det(M) : M in {0, ..., 4}^{3x3} }`;
- reject the whole family for `A` if `det(A)` is not in `D_4 * D_4`.

### Why it is sound

This is a direct necessary condition. If the family emits `A = U V`, then
`det(A)` must factor through determinants of admissible family witnesses.

Local brute force shows:

- `D_4` has `179` values, all inside `[-128, 128]`;
- the product set `D_4 * D_4` has `4483` values, inside `[-16384, 16384]`;
- small determinants such as `±83`, `±89`, `±97`, `±101`, `±103`, `±107`,
  `±109`, `±113`, and `±127` are excluded.

So this is a real exact gate, not a heuristic.

### Why it is too weak on the current hotspot

On the current mixed `k=3` hard control from
`profile_square_factorisation_sources`, the heavy `square_factorisation_3x3`
sources are all singular. For the top raw sources, `det(A) = 0` every time, so
the determinant gate rejects none of them.

Representative hotspot rows from the local control:

- `[0,1,0] [2,2,3] [0,1,0]`: `raw_sq=3737`, `det=0`
- `[0,1,0] [1,1,1] [1,5,1]`: `raw_sq=1279`, `det=0`
- `[1,1,5] [1,0,1] [1,0,1]`: `raw_sq=1331`, `det=0`

That is the decisive bounded evidence: the exact determinant gate is valid, but
it misses the actual expensive seam that motivated the bead.

### Related profile/support idea and why it is unsound as a hard gate

The natural follow-up idea was to promote duplicate-row, duplicate-column, or
partition-refined support profiles into a hard no-go gate for
`square_factorisation_3x3`.

That does **not** survive as an exact family impossibility test.

Reason:

- duplicated-row or duplicated-column sources can still have many raw square
  witnesses;
- the current profile control explicitly shows this.

Counterexamples from the same bounded control:

- unproductive after later pruning but still family-admissible:
  `[0,1,0] [2,2,3] [0,1,0]` with `raw_sq=3737`
- productive under the same broad duplicate-row/column surface:
  `[1,1,5] [1,0,1] [1,0,1]` with `raw_sq=1331`, discovered square successors
  `96`
- another productive duplicate-column source:
  `[0,1,0] [1,1,1] [1,5,1]` with `raw_sq=1279`, discovered square successors
  `97`

So support/profile structure is still useful for orbit reduction and proposal
ordering, but not as a theorem-level source-only rejection rule for this family.

### Decision

Keep as a mathematically correct dossier item.

Reject as the next implementation candidate for the current hotspot.

## Candidate 2: exact row/column mass gates for the bounded split families

### Candidate

For `single_row_split_3x3_to_4x4`, the family algebra is:

- choose a source row `r_i`;
- write `r_i = p + q` with nonzero `p, q in Z_+^3`;
- duplicate the matching block contiguously.

So the family is nonempty **iff** some source row has sum at least `2`.

Likewise, `single_column_split_3x3_to_4x4` is nonempty **iff** some source
column has sum at least `2`.

### Why it is sound

Necessity:

- if `r_i = p + q` with nonzero `p, q`, then `sum(r_i) >= 2`.

Sufficiency:

- if `sum(r_i) >= 2`, choose one positive unit from `r_i` as `p` and let
  `q = r_i - p`; then `p, q` are nonnegative and nonzero, so the bounded family
  has a valid split witness.

The same argument works columnwise for the column-split family.

### Why it is low leverage here

This is an exact gate, but it does not bite on the current `3x3` hotspot
surfaces. The expensive sources already have row and column mass far above
`1`, so the family would still be admitted.

### Decision

Keep as a correct exact gate.

Do not prioritize it ahead of the better diagonal-family gate below, because it
is unlikely to cut the current expensive surfaces.

## Candidate 3: exact admissible-divisor gate for diagonal refactorization

### Candidate

For `diagonal_refactorization_3x3` and `diagonal_refactorization_4x4`, the
family emits

```text
A = D X -> B = X D
```

or

```text
A = X D -> B = D X
```

with tiny positive diagonal `D`:

- `3x3`: `d_i in {1, 2, 3}`
- `4x4`: `d_i in {1, 2}`

and scalar diagonals skipped.

An exact source-node gate is therefore:

- for each row, compute which allowed diagonal entries divide that entire row
  while leaving the quotient inside the family cap;
- for each column, compute the analogous allowed divisors;
- reject the whole family if there is no non-scalar admissible row divisor
  profile and no non-scalar admissible column divisor profile.

### Why it is sound

This is just the family definition rewritten as a source-only test.

- `A = D X` is possible exactly when every row `i` is divisible by `d_i`, with
  quotient row entries still inside the cap.
- `A = X D` is possible exactly when every column `j` is divisible by `d_j`,
  with quotient column entries still inside the cap.
- the family is nontrivial exactly when the admissible diagonal is non-scalar.

So the admissible-divisor-profile test is not a heuristic. It is an exact
emptiness test for the current bounded diagonal families.

### Why this is the best next implementation candidate

- It is exact.
- It is local to one family.
- It is a very small code slice.
- It matches the current literature-backed direction: bounded structured-family
  certificates rather than global invariants.
- Unlike the determinant gate, it does not depend on the current square-family
  hotspot staying nonsingular to be useful.

This still will not solve the `square_factorisation_3x3` hotspot by itself. But
it is the cleanest exact family gate discovered in this bounded round.

## Overall verdict

### Keep

- `diagonal_refactorization_3x3` / `4x4` admissible-divisor-profile gate:
  best next exact implementation candidate.
- row/column mass gates for the `3x3 -> 4x4` split families:
  exact but low leverage.

### Reject

- duplicate-row, duplicate-column, or support/profile-only no-go claims for
  `square_factorisation_3x3`:
  unsound as hard family impossibility gates.
- determinant-factor gate for `square_factorisation_3x3` as the next practical
  implementation target:
  sound, but too weak on the current singular hotspot.

## Best next step

If this bead turns into code, the best bounded prototype is:

1. add an exact `has_diagonal_refactorization_candidate_{3x3,4x4}` precheck;
2. gate the diagonal-family enumeration on that predicate;
3. leave `square_factorisation_3x3` on the current exact orbit-reduction path,
   not on a new support-profile hard gate.

If a later bead returns to `square_factorisation_3x3`, the next exact research
direction should probably be a singular-family certificate stronger than plain
determinant factorization, because the current hotspot is overwhelmingly
`det=0`.
