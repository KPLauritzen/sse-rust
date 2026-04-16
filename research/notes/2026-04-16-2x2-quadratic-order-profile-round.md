# 2x2 quadratic-order profile round (2026-04-16)

## Question

What is the smallest defensible extension of the current `2x2` arithmetic
dossier beyond the existing Eilers-Kiming ideal-class slice, while keeping this
round theorem-backed and reporting-focused rather than turning it into a broad
new pruning policy?

## Added

The repo now exposes one bounded second-stage arithmetic layer for irreducible
`2x2` endpoint matrices:

- exact quadratic-order data for `Z[lambda]`:
  - order discriminant,
  - field discriminant,
  - conductor, equivalently the order index in the maximal order,
  - maximal-vs-nonmaximal status;
- a principal-vs-nonprincipal summary for the Perron eigenvector ideal class,
  computed against the canonical reduced principal form of the same order when
  the current ideal-class routine can classify the endpoint.

Concrete touchpoints:

- [`src/quadratic.rs`](../../src/quadratic.rs) now exports
  `QuadraticOrderProfile`, `quadratic_order_profile`, and principal-class
  helpers;
- [`src/invariants.rs`](../../src/invariants.rs) now attaches this data to the
  per-endpoint `ArithmeticProfile2x2`;
- [`src/bin/profile_gl2z_similarity_2x2.rs`](../../src/bin/profile_gl2z_similarity_2x2.rs)
  prints the added dossier lines for calibration pairs.

## Calibration

### Eilers-Kiming `[[14,2],[1,0]]` vs `[[13,5],[3,1]]`

- both endpoints lie in the maximal order with discriminant `204`;
- the source ideal class is principal;
- the target ideal class is nonprincipal.

This matches the paper-level story and is consistent with the existing ideal
class mismatch already used as a hard rejection.

### Brix-Ruiz `k = 3`

- both endpoints have order discriminant `24`;
- field discriminant `24`, conductor `1`, so the attached order is maximal;
- both endpoint ideal classes are principal.

### Brix-Ruiz `k = 4`

- both endpoints have order discriminant `48`;
- field discriminant `12`, conductor `2`, so the attached order is nonmaximal;
- both endpoint ideal classes are principal.

This makes `k = 4` a useful calibration example for the new conductor /
order-index reporting even though it does not add a new rejection.

## Why This Stays Reporting-Only

No new hard rejection was added in this round.

Reason:

- once trace and determinant match, the discriminant is already fixed for the
  pair, so the attached order and its conductor are also fixed;
- principal-vs-nonprincipal status is a coarser summary of the ideal class, so
  any mismatch there is already subsumed by the existing ideal-class
  obstruction.

So this round strengthens the arithmetic dossier and makes the order structure
explicit, but it does not create a genuinely new theorem-backed pairwise screen
beyond the current ideal-class comparison.

## Deferred

The next plausible arithmetic layer is still one of:

- an exact ideal-quotient / colon-ideal necessary check, or
- another bounded order-theoretic condition from Eilers-Kiming that is not
  already implied by the ideal class.

That work was deferred here because this round deliberately preferred one small
defensible addition over a batch of partially justified arithmetic heuristics.

Follow-up result:

- [`2026-04-16-2x2-ideal-quotient-followup.md`](2026-04-16-2x2-ideal-quotient-followup.md)
  records the negative result for that next step: the theorem-backed
  ideal-quotient condition only yields class congruence modulo primes dividing
  `lambda`, so it did not justify a new exact pairwise screen beyond the
  current ideal-class comparison.
