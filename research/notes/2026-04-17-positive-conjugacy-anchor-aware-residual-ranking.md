# Sampled positive-conjugacy follow-up: anchor-aware residual arithmetic ranking for exact local seeds (2026-04-17)

## Question

Can the surviving exact local seed family be ranked better than the current
blind target-nearest control if the sidecar scores each seed by the residual
segment it actually induces, rather than by target distance alone?

This stayed strictly outside default `search_sse`. The only code change is in
the sidecar probe.

## Chosen slice

Updated `src/bin/probe_positive_conjugacy_seeds.rs` with one additional seed
ordering:

- keep the existing proposal-guided shortlist unchanged;
- keep the existing blind target-nearest shortlist unchanged as the control;
- add an `anchor-aware residual arithmetic` shortlist on the same exact local
  seed family.

The new ordering is explicit and bounded:

1. prefer residual pairs that fall into an implemented exact positive class;
2. then prefer residual pairs that stay strictly positive;
3. then prefer residual pairs that are `GL(2,Z)`-similar;
4. then prefer smaller anchor-aware residual endpoint distance;
5. only then use proposal proximity and proposal rank as tie-breaks.

Here “anchor-aware residual endpoint distance” means:

- for a source-anchored seed, rank by `d(seed, B)`;
- for a target-anchored seed, rank by `d(A, seed)`.

That fixes a real blind spot in the old control: target-anchored seeds induce
the residual search `A -> seed`, but the old blind score still ranked them by
distance to `B`.

The probe now prints the residual arithmetic profile for each shortlisted seed:

- `residual_exact`
- `residual_gl2z`
- `residual_positive`
- `residual_l1`

## Validation

Focused checks:

```bash
cargo test -q conjugacy::tests --lib

cargo test -q --features research-tools --bin probe_positive_conjugacy_seeds

cargo build -q --features research-tools --bin probe_positive_conjugacy_seeds

timeout -k 10s 30s target/debug/probe_positive_conjugacy_seeds -- \
  --case brix_k3 \
  --proposal-top-k 4 \
  --seed-top-k 4 \
  --local-seed-lag 2 \
  --max-lag 4 \
  --max-dim 4 \
  --max-entry 6

timeout -k 10s 30s target/debug/probe_positive_conjugacy_seeds -- \
  --case brix_k4 \
  --proposal-top-k 4 \
  --seed-top-k 4 \
  --local-seed-lag 2 \
  --max-lag 4 \
  --max-dim 4 \
  --max-entry 6

timeout -k 10s 30s target/debug/probe_positive_conjugacy_seeds -- \
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

This is the only case where the new heuristic materially changes the order.

Rounded proposals:

- current proposal-guided shortlist starts with the target-side factorisation
  seeds `[[0, 5], [1, 2]]` and `[[2, 5], [1, 0]]`;
- the new residual-arithmetic shortlist moves the positive permutation seeds to
  the front:
  - `[[1, 2], [3, 1]]` (`source`)
  - `[[1, 1], [6, 1]]` (`target`)

Why the reorder happened:

- every surviving residual pair is already `GL(2,Z)`-similar;
- none land in an implemented exact Baker/Choe-Shin class;
- the main separating signal is therefore strict positivity of the residual
  pair, which demotes the zero-entry factorisation seeds.

Invariant-compatible reprojection shows the same pattern:

- the arithmetic shortlist keeps the two positive permutation seeds first;
- the blind target-nearest control still keeps the closer zero-entry
  factorisation seeds first.

Outcome:

- no shortlisted residual search solved under `lag<=4`;
- proposal-guided, anchor-aware residual arithmetic, and blind target-nearest
  all stay negative.

Reading:

- anchor-aware residual scoring is directionally more coherent than the old
  blind target metric;
- but for `brix_k3`, the residual arithmetic surface is still too flat to
  predict a useful bounded suffix.

### `brix_k4`

No real change.

- there is still only one exact local seed under the bound:
  `[[1, 3], [4, 1]]`;
- all three shortlists are identical;
- residual search stays `unknown`.

Reading:

- this case is still bottlenecked by exact local seed scarcity, not by ranking.

### `riedel_baker_k3`

The new arithmetic profile is informative but not decisive.

- both surviving residual pairs are in the exact positive class
  `baker_1983`;
- both are strictly positive and `GL(2,Z)`-similar;
- both shortlists therefore remain the same as the existing order.

Outcome:

- both shortlisted residual segments still solve;
- best realized total lag remains `4`;
- the new ordering does not beat blind target-nearest.

Reading:

- the residual arithmetic profile confirms why these seeds are healthy;
- but it does not create additional separation beyond what the current blind
  control already had.

## Conclusion

This is another useful negative sidecar result.

What improved:

- the probe now exposes one explicit anchor-aware residual ranking rather than
  only proposal proximity and blind target-nearest;
- the residual profile is now visible per shortlisted exact local seed;
- the old anchor-blind control issue is fixed for analysis purposes.

What did **not** improve:

- on `brix_k3`, the new heuristic reorders the shortlist toward the positive
  permutation seeds, but still finds no bounded suffix;
- on `brix_k4`, there is still only one exact local seed to rank;
- on `riedel_baker_k3`, residual arithmetic agrees with the already-good order
  and does not beat the blind control.

So `sse-rust-2uy.3` should remain open.

Current read:

- the remaining proposal-engine gap is not just “choose seeds using the right
  endpoint distance”;
- even after anchor-aware residual arithmetic, the surviving exact local seed
  family is still not separated strongly enough to beat the blind target-nearest
  control on the bounded evidence cases;
- the next bounded slice, if this line continues, likely needs richer residual
  difficulty signals than positivity / exact positive class / `GL(2,Z)`
  similarity alone.
