# No clean next bounded exact family after the binary-sparse orbit seams (2026-04-20)

## Question

For bead `sse-rust-igo`, is there exactly one additional hot structured family
on the same bounded mixed endpoint control that admits a comparably clean exact
family-preserving symmetry or certificate action, so that one more local
orbit/certificate seam is worth keeping?

This slice stayed narrow:

- no generic certificate/orbit framework;
- no frontier rewrite or solver-policy change;
- no multiple-family round;
- no weighted `4x4 -> 3` reopening;
- just one fixed-control check and a keep/reject decision.

## Decision

Reject.

No remaining family on this bounded surface clears the bar cleanly enough to
justify another kept seam.

## Why no family qualified

### 1. `single_row_split_3x3_to_4x4` is the only nearby family with a clean local exact action, but that action is already fully spent

This family does admit an exact local symmetry:

- for a fixed split row, the witness has the form `(U, V)` where `U` is the
  fixed duplication matrix for that row and `V` contains the two adjacent split
  pieces `split` and `twin`;
- swapping those two adjacent clone slots gives another witness in the same
  exact family;
- the source matrix stays the same, and the successor changes only by swapping
  the two cloned states, so the two successors are permutation-similar.

But the existing enumerator already quotients exactly by that action:

- [src/factorisation.rs](../../src/factorisation.rs) emits only witnesses with
  `split <= twin`;
- the profiler sidecar confirms the whole raw `16`-callback sample collapses to
  `8` twin-swap representatives, and that `kept = exact = canon = 8`.

So there is no new seam to land here. The exact local action exists, but it is
already encoded at witness generation time.

### 2. `diagonal_refactorization_4x4` does not admit a comparably clean exact family-preserving action

The tempting remaining `4x4` same-dimension family is structurally different
from the landed binary-sparse seams:

- witnesses come in two heterogeneous shapes, `(D, X)` from row-division and
  `(X, D)` from column-division;
- the distinguished factor `D` must stay diagonal;
- under a generic simultaneous middle-basis renaming
  `(U', V') = (U P, P^{-1} V)`, the diagonal factor becomes `P^{-1} D P`, which
  is generally not diagonal unless `P` stabilizes the equal-diagonal blocks of
  that specific witness.

That means there is no uniform exact `S4`-style family action analogous to the
landed square and binary-sparse orbit keys. Any admissible permutation set
would depend on the individual diagonal multiplicities and on whether the
current witness arose from the row-divided or column-divided branch. That is
not the same kind of clean local seam, and forcing it here would violate the
“exactness argument is explicit and local” constraint for this bead.

## Bounded control surface

Fixed mixed endpoint control, unchanged from the earlier exact-orbit notes:

- source `[[1,3],[2,1]]`
- target `[[1,6],[1,1]]`
- `max_lag=6`
- `max_intermediate_dim=4`
- `max_entry=6`
- `move_family_policy=mixed`

## Measured result

Saved bounded profile output:

- [tmp/2026-04-20-igo-profile-structured-factorisation-orbits.txt](../../tmp/2026-04-20-igo-profile-structured-factorisation-orbits.txt)

The fixed-control profile says:

- `binary_sparse_rectangular_factorisation_3x3_to_4` is still the live hot
  family: `candidates=257736`, `after_pruning=2015`, `discovered=1585`
- `binary_sparse_rectangular_factorisation_4x3_to_3` is still active but
  already covered by the landed seam: `candidates=105446`, `after_pruning=98`,
  `discovered=8`
- `single_row_split_3x3_to_4x4` is not live on this control:
  `candidates=41674`, `after_pruning=0`, `discovered=0`

The direct row-split sample in the same saved run shows the only clean exact
symmetry is already exhausted inside the current enumerator:

- `kept=8`
- `raw_unquotiented=16`
- `twin_orbit=8`
- `exact=8`
- `canon=8`

So the measured bounded read is:

- the only additional family with an explicit local exact action is inactive on
  the fixed control and already internally quotiented by that action;
- the other plausible remaining same-dimension family does not support a clean
  exact family-preserving group action of the kind used by the kept orbit seams.

## Validation

Commands used:

```bash
timeout -k 10s 180s cargo run --features research-tools \
  --bin profile_structured_factorisation_orbits --quiet \
  > tmp/2026-04-20-igo-profile-structured-factorisation-orbits.txt

cargo fmt --all
```

No focused test was needed because no solver code changed; this session ends in
a durable reject note rather than a code seam.
