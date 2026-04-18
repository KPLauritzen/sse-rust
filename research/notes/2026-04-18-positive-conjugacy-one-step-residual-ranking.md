# Sampled positive-conjugacy follow-up: residual one-step continuation ranking for exact local seeds (2026-04-18)

## Question

Can the surviving exact local seed family be ranked better than the current
blind target-nearest control if each seed is scored by a tiny residual
continuation signal instead of only by residual arithmetic or endpoint
distance?

This stayed strictly outside default `search_sse`. The only code change is in
the sidecar probe.

## Chosen slice

Updated `src/bin/probe_positive_conjugacy_seeds.rs` with one additional
ordering on the same exact local seed pool:

- keep the existing `proposal-guided` ordering;
- keep the existing `anchor-aware residual arithmetic` ordering;
- keep the existing blind `target-nearest` control;
- add a `residual one-step continuation` ordering.

The new score is deliberately tiny and explicit. For the residual segment
induced by each seed:

- source-anchored seed: residual segment is `seed -> B`;
- target-anchored seed: residual segment is `A -> seed`.

From that residual source, enumerate only exact one-step same-dimension
successors under the current bounded search settings and record:

1. whether any successor hits the residual target exactly;
2. the best residual endpoint `L1` distance among one-step successors;
3. how many one-step successors strictly improve that residual distance;
4. how many move families contribute improving successors;
5. the total one-step successor count.

The ranking then prefers:

1. exact one-step hits;
2. smaller best one-step residual distance;
3. more improving successors;
4. more improving move families;
5. more one-step successors;
6. only then the existing residual arithmetic and proposal-distance tie-breaks.

The probe now prints this profile for each shortlisted seed:

- `one_step_hit`
- `one_step_best_l1`
- `one_step_improving`
- `one_step_families`
- `one_step_successors`

## Validation

Focused checks:

```bash
cargo test -p sse-core conjugacy::tests --lib

cargo test -p sse-core --features research-tools --bin probe_positive_conjugacy_seeds

cargo build -p sse-core --features research-tools --bin probe_positive_conjugacy_seeds

timeout -k 10s 60s target/debug/probe_positive_conjugacy_seeds --case brix_k3 --proposal-top-k 4 --seed-top-k 4 --local-seed-lag 2 --max-lag 4 --max-dim 4 --max-entry 6

timeout -k 10s 30s target/debug/probe_positive_conjugacy_seeds --case brix_k4 --proposal-top-k 4 --seed-top-k 4 --local-seed-lag 2 --max-lag 4 --max-dim 4 --max-entry 6

timeout -k 10s 30s target/debug/probe_positive_conjugacy_seeds --case riedel_baker_k3 --proposal-top-k 4 --seed-top-k 4 --local-seed-lag 2 --max-lag 4 --max-dim 3 --max-entry 4
```

`cargo fmt -p sse-core` was attempted repeatedly but hung in this workmux setup
via the shimmed `cargo` path without producing formatter output, so the code was
validated with compile/test gates and diff inspection instead.

## Results

### `brix_k3`

This is the only case where the new signal materially changes the shortlist.

Rounded proposals:

- blind target-nearest still starts with the nearer zero-entry target-local
  seeds:
  - `[[0, 5], [1, 2]]`
  - `[[2, 5], [1, 0]]`
- residual one-step continuation instead promotes the positive permutation
  seeds first:
  - `[[1, 2], [3, 1]]`
  - `[[1, 1], [6, 1]]`

Why:

- the positive permutation seeds have better one-step residual continuation:
  - `one_step_best_l1=4`
  - `one_step_improving=1`
  - `one_step_families=1`
- the nearer zero-entry seeds are locally flatter:
  - `one_step_best_l1=5`
  - `one_step_improving=0`
  - `one_step_families=0`

Invariant-compatible reprojection shows the same ordering shift.

Outcome:

- no residual-one-step shortlisted suffix solved under the bounded lag;
- no variant beat blind target-nearest;
- bounded improvement: none.

Reading:

- the new signal is directionally coherent and richer than raw residual
  arithmetic alone;
- but even the seeds with the best one-step continuation still do not realize a
  bounded suffix here.

### `brix_k4`

No real change.

- there is still only one exact local seed under the bound:
  `[[1, 3], [4, 1]]`;
- all orderings are identical;
- residual search stays `Unknown`.

Reading:

- this case is still bottlenecked by local seed scarcity, not by ranking.

### `riedel_baker_k3`

The new signal is uninformative here.

- both surviving residual segments have:
  - `one_step_hit=no`
  - `one_step_best_l1=3`
  - `one_step_improving=0`
  - `one_step_families=0`
  - `one_step_successors=0`
- both residual segments still solve under the bounded suffix search;
- best realized total lag remains `4`;
- the new ranking stays tied with blind.

Reading:

- on this case, one-step same-dimension continuation is too weak a proxy for
  actual bounded suffix usefulness.

## Conclusion

This is another useful negative sidecar result.

What improved:

- the probe now exposes one additional explicit residual-difficulty signal that
  is not just proposal proximity or residual arithmetic;
- on `brix_k3`, the signal correctly demotes the locally flat zero-entry seeds
  and promotes the positive permutation seeds.

What did **not** improve:

- on the bounded evidence cases, the new ranking still does not beat blind
  target-nearest;
- `brix_k3` remains unsolved under the bounded suffix probe even after the
  more coherent reorder;
- `brix_k4` remains seed-scarce;
- `riedel_baker_k3` remains tied with blind.

So `sse-rust-2uy.3` should stay open.

Current read:

- the remaining proposal-engine gap is not just residual arithmetic, and not
  just “take one improving next move”;
- one-step same-dimension continuation is informative on `brix_k3`, but still
  not predictive enough to convert the better-looking seeds into a bounded
  suffix;
- the next bounded slice, if this line continues, likely needs richer residual
  difficulty than one-step local continuation alone, perhaps a tiny multi-step
  residual probe or another bounded signal that sees beyond the immediate
  successor layer.
