# Sampled positive-conjugacy follow-up: invariant-aware reprojection before local seed scoring (2026-04-17)

## Question

Can the sampled positive-conjugacy proposal surface become a better bounded
local-seed engine if, before local seed scoring, we reproject each sampled
matrix onto an exact integer `2x2` candidate that already matches the
endpoints' cheap arithmetic invariants?

This slice stayed strictly outside default `search_sse` behavior. It only
changes the sidecar probe.

## Chosen slice

Added one invariant-aware proposal variant in `src/conjugacy.rs` and compared it
against the existing rounded-sample proposals inside
`src/bin/probe_positive_conjugacy_seeds.rs`.

The new variant:

- still starts from the sampled positive-conjugacy witness;
- but instead of keeping only floor/ceil integer shadows of sampled matrices,
  it snaps each sampled real matrix to the nearest positive integer `2x2`
  matrix that already matches the endpoints' exact trace/determinant data;
- when the endpoints share both diagonal entries, the reprojection keeps that
  diagonal pair exactly and only solves the off-diagonal product constraint;
- otherwise it reprojects into the full positive trace/determinant family.

The probe now prints both proposal surfaces side by side:

- `rounded sampled proposals`
- `invariant-compatible reprojections`

and scores the same exact local seed family against each.

## Validation

Focused checks:

```bash
cargo test -q conjugacy::tests --lib

cargo test -q --features research-tools --bin probe_positive_conjugacy_seeds

cargo run --features research-tools --bin probe_positive_conjugacy_seeds -- \
  --case brix_k3 \
  --proposal-top-k 4 \
  --seed-top-k 4 \
  --local-seed-lag 2 \
  --max-lag 4 \
  --max-dim 4 \
  --max-entry 6

cargo run --features research-tools --bin probe_positive_conjugacy_seeds -- \
  --case brix_k4 \
  --proposal-top-k 4 \
  --seed-top-k 4 \
  --local-seed-lag 2 \
  --max-lag 4 \
  --max-dim 4 \
  --max-entry 6

cargo run --features research-tools --bin probe_positive_conjugacy_seeds -- \
  --case riedel_baker_k3 \
  --proposal-top-k 4 \
  --seed-top-k 4 \
  --local-seed-lag 2 \
  --max-lag 4 \
  --max-dim 3 \
  --max-entry 4
```

## Results

### `brix_k3`

Rounded proposals stayed as before:

- `[[1, 5], [1, 1]]`
- `[[1, 4], [2, 1]]`
- `[[1, 4], [1, 1]]`
- `[[1, 5], [2, 1]]`

Invariant-aware reprojection collapsed that surface to one exact interior
candidate:

- `[[1, 2], [3, 1]]`

Effect on the seeded shortlist:

- rounded ranking again preferred the target-local factorisation seeds first;
- invariant reprojection promoted the source-side permutation seed
  `[[1, 2], [3, 1]]` to rank `1`, with the target-side permutation
  `[[1, 1], [6, 1]]` next.

Outcome:

- no seeded residual search solved under `lag<=4`;
- no blind control solved either;
- bounded improvement: none.

Reading:

- the reprojection does what it should structurally: it removes invariant-bad
  rounded shadows and points directly at the exact permutation-compatible
  candidate family;
- but for `brix_k3`, that cleaner exact surface still does not make the
  proposal engine useful under the bounded suffix search.

### `brix_k4`

Rounded proposals:

- still centered on the exact interior shadow `[[1, 6], [2, 1]]` plus nearby
  endpoint-heavy rounded samples.

Invariant-aware reprojection produced just:

- `[[1, 6], [2, 1]]`
- `[[1, 3], [4, 1]]`

Outcome:

- there is still only one exact local seed under the bound,
  `[[1, 3], [4, 1]]`;
- both rounded and reprojected variants pick the same seed;
- residual search stays `unknown`.

Reading:

- reprojection trims noise from the proposal surface;
- but the real bottleneck here is exact local seed scarcity, not proposal
  ranking quality.

### `riedel_baker_k3`

Rounded proposals again stayed near the sampled path but were not themselves
exact invariant-compatible waypoints.

Invariant-aware reprojection collapsed the surface to:

- `[[3, 1], [2, 3]]`

which is exactly the source-side permutation seed already known to realize a
bounded suffix.

Outcome:

- both rounded and reprojected variants shortlist the same two exact local
  permutation seeds;
- both realize bounded suffixes with best total lag `4`;
- blind target-nearest matches them exactly.

Reading:

- the reprojection is directionally correct: it recovers the exact useful seed
  rather than nearby rounded shadows;
- but it still does not beat the existing blind control on this bounded case.

## Conclusion

This is a useful but negative sidecar result.

What improved:

- the probe can now compare raw rounded proposals against an exact
  invariant-compatible reprojection of the same sampled witness;
- on `brix_k3`, the invariant-aware variant does reorder the shortlist toward
  the exact permutation seed that the rounded ranking had placed behind target
  factorisation seeds;
- on `brix_k4` and `riedel_baker_k3`, the variant strips away rounded noise and
  exposes the exact small invariant-compatible family directly.

What did **not** improve:

- on the bounded evidence cases, invariant-aware reprojection still does not
  make proposal-guided shortlists beat the blind target-nearest shortlist;
- it does not resolve the remaining “proposal engine” gap for
  `sse-rust-2uy.3`.

Current read:

- the proposal surface benefits from exact invariant-aware cleanup;
- but the remaining gap is no longer “rounded proposals violate obvious
  invariants”;
- the remaining problem is that even the cleaned exact shortlist is not yet
  predictive enough to outperform very simple blind controls on the hard bounded
  cases.

So `sse-rust-2uy.3` should stay open. The next bounded follow-up, if this line
continues, should focus on how to score or prioritize the surviving exact local
seed family better, not on widening the raw rounded proposal family further.
