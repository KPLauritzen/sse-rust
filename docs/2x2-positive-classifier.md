## Exact `2x2` positive classifier

This repo now keeps two distinct `2x2` dossier notions separate:

- `determinant_territory` is only the determinant-side interval classification.
- `exact_positive_class` is the narrower theorem-backed pair classification.

The implemented exact positive classes are intentionally small:

- `Baker1983`:
  both endpoints are strictly positive, the shared determinant is nonnegative,
  and the pair is exactly `GL(2,Z)`-similar.
- `ChoeShin1997`:
  the shared determinant lies in `-2 tr(A) <= det(A) < -tr(A)`,
  `|det(A)|` is composite, and the pair is exactly `GL(2,Z)`-similar.

Interpretation rules:

- `determinant_territory = baker` does not by itself certify Baker's theorem.
  Zero entries still leave the pair outside the implemented exact Baker class.
- `exact_positive_class = none` is not a negative obstruction. It only means
  the pair falls outside this narrow literature-backed positive slice.
- Existing exact negative arithmetic checks, especially the Eilers-Kiming path
  in [`src/invariants.rs`](../src/invariants.rs), remain unchanged and stronger
  than this positive classifier for disproof work.
