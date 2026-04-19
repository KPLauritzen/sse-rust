# Replacement positive control for the bounded dim-2 vs dim-3 slice (2026-04-19)

## Question

For bead `sse-rust-mlv`, can we replace the stale positive-side control for the
bounded dim-`2` vs dim-`3` certificate slice without widening into a generic
hunt?

The old positive-side control was:

- `[[3,4],[3,4]] -> [[4,4],[3,3]]`

That pair is no longer suitable on current `main` because the dim-`3` envelope
now has a direct lag-`1` witness, so it no longer demonstrates a genuinely
dim-`3` success.

## Scope

This pass stayed intentionally narrow:

- keep the old rectangular dim-`2` bounded no-go guard unless a replacement
  absolutely required changing it;
- test one nearby literature-backed positive family already present in the repo
  before considering any broader search;
- accept one durable replacement only if the dim-`3` success is clearly
  nontrivial and the matching dim-`2` envelope still returns `unknown`.

## Candidate kept: `riedel_baker_k4`

Chosen endpoints:

- `A = [[4,2],[1,4]]`
- `B = [[3,1],[1,5]]`

These are already a durable literature-backed positive pair in
[research/cases.json](../cases.json), so the question here was only whether
they also form a good bounded dim-split control.

Focused validation commands:

```bash
timeout -k 5s 20s cargo run --quiet --bin search -- \
  4,2,1,4 3,1,1,5 \
  --max-lag 5 \
  --max-intermediate-dim 2 \
  --max-entry 4 \
  --json --telemetry

timeout -k 5s 20s cargo run --quiet --bin search -- \
  4,2,1,4 3,1,1,5 \
  --max-lag 1 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --json --telemetry

timeout -k 5s 20s cargo run --quiet --bin search -- \
  4,2,1,4 3,1,1,5 \
  --max-lag 5 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --json --telemetry
```

Results:

- `lag5 / dim2 / entry4`: `unknown`
- `lag1 / dim3 / entry4`: `unknown`
- `lag5 / dim3 / entry4`: `equivalent`

Why this is a good replacement positive control:

- the positive-side success is not a trivial direct witness: the lag-`1`
  dim-`3` probe still fails;
- the successful bounded witness is genuinely higher-dimensional: the found path
  is lag `5`, starts with a `2x2 -> 3x3` rectangular lift, spends interior work
  in the `3x3` search surface, and ends with a `3x3 -> 2x2` drop;
- the dim-`2` envelope remains exact-and-bounded `unknown` under a tight,
  inexpensive probe.

Telemetry summary:

- dim-`2` probe expanded only the source node and enumerated `4`
  `square_factorisation_2x2` candidates before exhausting;
- dim-`3` lag-`1` probe reached `199` dim-`3` candidates but no exact meet;
- dim-`3` lag-`5` probe succeeded with one exact meet after exploring the
  retained `3x3` surface, dominated by `square_factorisation_3x3`.

## Decision

Keep the replacement same-endpoint control pair:

- `riedel_baker_k4_dim2_bounded_no_go`
- `riedel_baker_k4_dim3_positive_control`

and keep the old rectangular dim-`2` bounded no-go guard as-is.

So the durable regression surface now has:

- the old rectangular case as a standalone exact dim-`2` bounded no-go guard;
- a new `riedel_baker_k4` same-endpoint dim split that directly asserts
  `dim2 -> unknown` versus `dim3 -> equivalent`.

This is still a bounded change: one candidate family, one kept replacement
control pair, and no wider hunt beyond the first viable literature-backed
probe.
