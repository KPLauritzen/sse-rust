# Sampled positive-conjugacy local seed follow-up: richer exact seed family (2026-04-17)

## Question

Can sampled positive-conjugacy become a stronger bounded local seed engine if
the probe scores against a slightly richer exact local seed family, while still
staying outside default `search_sse` behavior?

This round kept the existing probe shape for comparability:

- same sampled positive-conjugacy witness search;
- same proposal ranking from `src/conjugacy.rs`;
- same blind target-nearest control shortlist;
- same bounded residual exact search after choosing a local seed.

The only slice widened here was the exact local seed family exposed to the
probe.

## Chosen slice

Updated `src/bin/probe_positive_conjugacy_seeds.rs` so the local seed pool is
no longer only:

- source-local same-dimension `2x2` factorisation successors.

It is now the deduplicated union of:

- source-local same-dimension `2x2` factorisation seeds;
- target-local same-dimension `2x2` factorisation seeds;
- source-side `2x2` permutation-conjugate seeds;
- target-side `2x2` permutation-conjugate seeds.

Deduplication is orientation-aware:

- keep both `(matrix, source)` and `(matrix, target)` when the same exact seed
  appears on both sides, because they induce different residual searches.

Each seed now carries an anchor:

- `source`: residual exact search is `seed -> B`;
- `target`: residual exact search is `A -> seed`.

This keeps the probe exact-search compatible while recovering endpoint-local
exact seeds that the earlier source-only factorisation family missed.

## Validation

Focused checks:

```bash
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

Baseline context from the previous probe:

- source-local factorisation-only family had `1` exact seed:
  `[[1, 2], [3, 1]]`.

Updated family:

- `6` exact local seeds under the same bound;
- anchor breakdown: `source:1`, `target:5`.

Top seeded shortlist:

- `[[0, 5], [1, 2]]` (`target`, lag `1`)
- `[[2, 5], [1, 0]]` (`target`, lag `2`)
- `[[1, 2], [3, 1]]` (`source`, lag `1`)
- `[[1, 1], [6, 1]]` (`target`, lag `1`)

Outcome:

- no shortlisted residual segment solved under `lag<=4`;
- proposal-guided and blind target-nearest shortlists were identical on the top
  `4`;
- bounded improvement: none.

Reading:

- the old seed family was genuinely too narrow for this pair;
- richer exact local seeds alone still do not make the sampled proposal surface
  beat the blind shortlist.

### `brix_k4`

Updated family:

- still only `1` exact local seed under the bound:
  `[[1, 3], [4, 1]]` from the source-side permutation.

Outcome:

- proposal-guided and blind shortlists remain identical;
- residual search stays `unknown`;
- bounded improvement: none.

Reading:

- this case is still bottlenecked by exact local seed scarcity, even after
  adding the cheap endpoint-local family.

### `riedel_baker_k3`

Updated family:

- `2` exact local seeds:
  - `[[3, 1], [2, 3]]` (`source`, lag `1`)
  - `[[4, 1], [1, 2]]` (`target`, lag `1`)

Outcome:

- both shortlisted residual segments solve;
- best realized total lag is `4` from the source-side permutation seed;
- proposal-guided and blind target-nearest again choose the same ordering;
- bounded improvement: none.

Reading:

- the richer exact family recovers the exact local controls already seen in the
  earlier waypoint note;
- but the current sampled proposal score still does not separate them better
  than the blind target-nearest control.

## Conclusion

This round produced a bounded but useful negative result.

What improved:

- the local seed probe now covers exact endpoint-local seeds that were missing
  from the previous source-only same-dimension family;
- the probe can compare source-anchored and target-anchored exact residuals
  without changing main search.

What did **not** improve:

- on the tested bounded cases, the richer exact local seed family did not make
  proposal-guided shortlists beat the current blind target-nearest shortlist.

Current read:

- sampled positive conjugacy can support a better exact local-seed *surface*
  than the old probe exposed;
- but seed-family widening alone is not enough to make the proposal ranking
  materially stronger;
- the next bounded follow-up, if this line continues, should be
  invariant-aware reprojection or filtering of proposal candidates before local
  seed scoring, not more blind widening of the exact seed family.
