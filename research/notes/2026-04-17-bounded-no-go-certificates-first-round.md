# Exact bounded no-go certificates: first explicit envelope round (2026-04-17)

## Question

For bead `sse-rust-ilu.3`, can we keep one **exact** bounded-envelope no-go
certificate direction without falling back to heuristic “search failed”
claims?

This round stayed narrow on purpose. Instead of another broad survey, it asked
two explicit envelope questions:

1. can a strict intermediate-dimension cap become a real exact no-go
   certificate on a durable harness surface?
2. can a tiny lag-capped surface on the canonical hard pair become an exact
   no-go statement worth keeping, or is it only local evidence?

The round also rechecked whether the previously rejected
`square_factorisation_3x3` quotient/support ideas somehow become exact once the
envelope is phrased more tightly.

## Exactness standard used here

The repo’s bounded endpoint-search surface distinguishes:

- `not_equivalent`: a global exact rejection from hard invariants, and
- `unknown`: bounded search exhausted without finding a witness.

For this bead, `unknown` is still useful when the claim is explicitly bounded:

- “no witness exists under this cap”.

That interpretation is consistent with the harness model in
[src/bin/research_harness/execution.rs](../../src/bin/research_harness/execution.rs),
which records `SearchRunResult::Unknown` as `SearchExhausted`, and with the
bounded layer loops in [src/search.rs](../../src/search.rs).

So the certificate form in this note is:

- exact **within the stated bounded envelope**,
- not a global non-SSE claim.

## Probe 1: strict intermediate-dimension cap on `rectangular_positive_pair`

This is the cleanest exact bounded-envelope result from the round because the
repo treated this pair as the durable example where the dim-`2` envelope failed
but the dim-`3` envelope succeeded:

- source: `[[3,4],[3,4]]`
- target: `[[4,4],[3,3]]`
- baseline bound: `lag4 / entry6`

Commands:

```bash
cargo run --quiet --bin search -- \
  3,4,3,4 4,4,3,3 \
  --max-lag 4 \
  --max-intermediate-dim 2 \
  --max-entry 6 \
  --json --telemetry

cargo run --quiet --bin search -- \
  3,4,3,4 4,4,3,3 \
  --max-lag 4 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --json --telemetry
```

### Result under `max_intermediate_dim = 2`

Outcome:

- `unknown`

Telemetry summary:

- expanded frontier nodes: `1`
- factorisations enumerated: `43`
- only active family: `square_factorisation_2x2`
- exact meets: `0`

This is an exact bounded no-go certificate for the envelope

- `lag <= 4`
- `intermediate dimension <= 2`
- `entry <= 6`

on this pair.

### Result under `max_intermediate_dim = 3`

Outcome:

- `equivalent`

Witness:

- a `3`-step path was found immediately once `2x3` rectangular witnesses were
  allowed.

Telemetry summary:

- factorisations enumerated: `51,637`
- dominant family: `rectangular_factorisation_2x3`
- one exact meet appears from the backward side `insplit` expansion

### Why this matters

This is better than a family-local emptiness claim:

- it is a real exact certificate on a durable harness surface;
- the excluded resource is explicit and interpretable: `dimension 3`;
- the positive control proves the cap, not the pair, is what blocks the
  witness.

So the first keepable bounded-envelope certificate from this round is:

- **strict intermediate-dimension caps can be exact no-go certificates when
  the known witness family necessarily passes through the excluded dimension.**

This is mathematically clean and operationally durable.

### 2026-04-19 update

This probe's **dim-2** conclusion still holds on current `main`:

- `[[3,4],[3,4]] -> [[4,4],[3,3]]` remains `unknown` under
  `lag4 / max_intermediate_dim=2 / entry6`

But the **positive-side interpretation above is now stale**.

Rechecking the same pair on current `main` with
`lag4 / max_intermediate_dim=3 / entry6` now yields a direct `1`-step witness:

```text
U = [[1,1],[1,1]]
V = [[2,2],[1,2]]
```

with `A = UV` and `B = VU`.

So this pair no longer supports the stronger explanatory claim that the current
positive witness necessarily passes through a `2x3` rectangular intermediate or
that "dimension `3` is what blocks the witness" on the positive side. What
survives is narrower:

- the dim-`2` bounded no-go certificate is still valid for this exact envelope;
- the dim-`3` companion case is still a valid positive control under the wider
  envelope;
- but the pair is no longer the right durable example if we want a control that
  still isolates "dim `2` fails, dim `3` succeeds for genuinely dim-`3`
  reasons."

Follow-up:

- `sse-rust-mlv` was opened to find a replacement positive-control case that
  still serves that function cleanly.

## Probe 2: tiny lag caps on the canonical `brix_ruiz_k3` pair

I then checked whether a tiny lag cap on the canonical hard pair gives a
durable exact no-go surface rather than just a local curiosity.

Endpoints:

- source: `[[1,3],[2,1]]`
- target: `[[1,6],[1,1]]`

Commands:

```bash
cargo run --quiet --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 1 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json --telemetry

cargo run --quiet --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 2 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json --telemetry
```

Results:

- `lag1 / dim3 / entry6`: `unknown`
- `lag2 / dim3 / entry6`: `unknown`

Lag-`2` telemetry still shows:

- exact meets: `0`
- dominant family: `rectangular_factorisation_2x3`
- discovered nodes: `619`

So these are also exact bounded no-go certificates:

- no witness exists under the stated lag caps on this surface.

### Why this is weaker than Probe 1

This certificate is real, but it is less useful as a durable research artifact:

- it excludes only very small lag bands;
- it does not expose a crisp structural reason like “dimension `3` is
  necessary”;
- the active families are still broad enough that a source-only arithmetic
  summary is not obviously complete.

Verdict:

- keep as local exact evidence;
- do not prioritize tiny lag-cap certificates as the main bounded-envelope
  direction.

## Rejected candidate: promote `square_factorisation_3x3` quotient/support summaries into bounded source-only certificates

This still does not survive as an exact bounded-envelope direction.

The relevant bounded rejection is already established in
[2026-04-17-square-factorisation-3x3-quotient-support-obstruction-followup.md](2026-04-17-square-factorisation-3x3-quotient-support-obstruction-followup.md):

- fixed support plus duplicate-row structure still splits into factorable and
  unfactorable exact cases;
- the bounded singular-grid scan gives explicit collisions where the coarse
  same-future/past signature agrees but exact family emptiness disagrees;
- the stronger partition-refined quotient signature also collides.

Concrete counterexamples from that note:

```text
[0,0,0] [0,0,0] [2,1,0]   emitted_factorisations = 0
[2,1,0] [0,0,0] [0,0,0]   emitted_factorisations = 74700
```

and

```text
[0,0,0] [0,0,0] [0,1,2]   emitted_factorisations = 0
[0,0,0] [0,1,2] [0,0,0]   emitted_factorisations = 74700
```

So even in a bounded research framing, the coarse quotient/support route is
still too weak to certify `square_factorisation_3x3` emptiness exactly.

## Where the bounded-envelope directions now look strongest

### Promising

- exact intermediate-dimension certificates on pairs whose known witnesses
  necessarily pass through one excluded dimension; `rectangular_positive_pair`
  was the motivating example for this note, but it now needs a replacement
  control case on the positive side;
- explicit structured-family envelopes, especially the bounded diagonal
  refactorization slice from `ilu.1`, where family emptiness is equivalent to a
  tiny divisibility/admissible-diagonal test;
- exact `2x2` arithmetic slices when the reduced search really stays inside
  that arithmetic regime, rather than merely touching it occasionally;
- bounded exhaustive-search certificates modulo **exact** orbit reduction, in
  the McKay/canonical-augmentation sense from the literature note.

### Weak

- coarse source-only quotient/support signatures for
  `square_factorisation_3x3`;
- determinant-only source gates on singular `3x3` hotspot surfaces;
- broad lag-cap claims without either a reduced family proof or a genuinely
  exact orbit-complete bounded exhaustion argument.

## Decision

Keep two durable conclusions from this first round:

1. a strict intermediate-dimension cap can already serve as an exact bounded
   no-go certificate on a real harness surface;
2. the best source-side certificate family still looks like explicit bounded
   structured families, especially diagonal refactorization, not
   `square_factorisation_3x3` quotient/support summaries.

If a later bead wants one implementation-target certificate rather than a note,
the best next exact candidate is still:

- implement the diagonal admissible-divisor gate from `ilu.1`, and
- attach it only to the explicit bounded diagonal family where completeness is
  clear.
