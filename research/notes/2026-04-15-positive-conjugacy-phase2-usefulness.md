# Positive-conjugacy phase 2 usefulness check (2026-04-15)

## Question

Are the top-ranked phase-1 positive-conjugacy proposals actually useful as bounded offline waypoint or seed candidates?

## Setup

Added a standalone research binary:

- `cargo run --features research-tools --bin evaluate_positive_conjugacy_usefulness -- ...`

The evaluator reuses the merged phase-1 code in `src/conjugacy.rs` and does three things:

1. derive the ranked discrete proposals from the positive-conjugacy witness;
2. run bounded endpoint searches on `A -> M` and `M -> B` for each selected proposal;
3. compare them against a tiny same-diagonal determinant-matched control family.

Controls are intentionally local and narrow:

- same diagonal as the endpoints;
- same determinant as the endpoints;
- exclude the endpoints themselves;
- exclude any matrices already proposed by the phase-1 ranking.

That gives a small set of “nearby but not proposal-ranked” matrices without widening scope into broad random search.

## Commands

Primary `k=3` graph-only run:

```bash
cargo run --features research-tools --bin evaluate_positive_conjugacy_usefulness -- \
  --case brix_k3 \
  --top-k 4 \
  --controls-limit 4
```

Tight mixed sanity check on `k=3`:

```bash
cargo run --features research-tools --bin evaluate_positive_conjugacy_usefulness -- \
  --case brix_k3 \
  --top-k 1 \
  --controls-limit 2 \
  --include-mixed \
  --mixed-max-lag 4 \
  --mixed-max-dim 3 \
  --mixed-max-entry 6 \
  --mixed-beam-width 32
```

Secondary `k=4` graph-only spot-check:

```bash
cargo run --features research-tools --bin evaluate_positive_conjugacy_usefulness -- \
  --case brix_k4 \
  --top-k 2 \
  --controls-limit 3 \
  --graph-max-lag 8 \
  --graph-max-dim 4 \
  --graph-max-entry 6
```

## Results: `brix_k3`

Endpoint:

- `A = [[1, 3], [2, 1]]`
- `B = [[1, 6], [1, 1]]`

Phase-1 witness/proposals:

- witness conjugator `G = [[1, 0], [0, 2]]`
- `6` unique proposal candidates
- top `4` ranked proposals:
  - `P1 = [[1, 5], [1, 1]]`
  - `P2 = [[1, 4], [2, 1]]`
  - `P3 = [[1, 4], [1, 1]]`
  - `P4 = [[1, 5], [2, 1]]`

Graph-only direct baseline:

- config: `lag<=10`, `dim<=5`, `entry<=6`
- outcome: `unknown`
- telemetry: `visited=318244`, `frontier_expanded=40707`, `candidates=491744`, `pruned=479326`, `max_frontier=205297`

Top-ranked proposals:

- `P1` fails immediately: determinant mismatch `-5 vs -4`
- `P2` fails immediately: determinant mismatch `-5 vs -7`
- `P3` fails immediately: determinant mismatch `-5 vs -3`
- `P4` fails immediately: determinant mismatch `-5 vs -9`
- all proposal segment checks return `not_equivalent` with `visited=0`

Local controls:

- `C1 = [[1, 2], [3, 1]]`
- `C2 = [[1, 1], [6, 1]]`

Graph-only control behavior:

- `A -> C1` is `equivalent lag=1` via permutation shortcut
- `C1 -> B` is `unknown` with the same telemetry as the direct baseline:
  - `visited=318244`, `frontier_expanded=40707`, `candidates=491744`, `pruned=479326`, `max_frontier=205297`
- `A -> C2` is `unknown` with the same telemetry as the direct baseline
- `C2 -> B` is `equivalent lag=1` via permutation shortcut

Interpretation:

- top-ranked proposals do not survive basic endpoint invariants, so they are not exact waypoint candidates at all;
- the nearest determinant-matched controls only expose endpoint permutations and leave the full direct-pair difficulty in the residual segment;
- top-ranked proposals do **not** beat controls.

## Mixed sanity check: `brix_k3`

Config:

- graph-only baseline kept as above;
- mixed beam sanity profile: `lag<=4`, `dim<=3`, `entry<=6`, `beam=32`

Direct mixed baseline:

- outcome: `unknown`
- telemetry: `visited=2654`, `frontier_expanded=162`, `candidates=109794`, `pruned=4402`, `factorisations=109669`, `max_frontier=32`

Mixed outcome on the top proposal/control slice:

- `P1 = [[1, 5], [1, 1]]` is still rejected immediately by determinant mismatch on both segments
- `A -> C1` and `C2 -> B` are still trivial permutation shortcuts
- the residual control segments remain `unknown`:
  - `C1 -> B`: `visited=2679`, `frontier_expanded=162`, `candidates=110165`, `pruned=4428`, `factorisations=110040`
  - `A -> C2`: `visited=2636`, `frontier_expanded=162`, `candidates=107619`, `pruned=4259`, `factorisations=107494`

Interpretation:

- even a tight mixed profile does not rescue the phase-1 `k=3` proposal ranking;
- the proposal family is still losing before real search starts, while the controls again only peel off permutation endpoints.

## Secondary spot-check: `brix_k4`

Endpoint:

- `A = [[1, 4], [3, 1]]`
- `B = [[1, 12], [1, 1]]`

Graph-only direct baseline:

- config: `lag<=8`, `dim<=4`, `entry<=6`
- outcome: `unknown`
- telemetry: `visited=2860`, `frontier_expanded=823`, `candidates=4224`, `pruned=3997`, `max_frontier=1505`

Top-ranked proposals checked:

- `P1 = [[1, 6], [2, 1]]`
- `P2 = [[1, 11], [1, 1]]`

Outcome:

- `P1` is an exact integer shadow and matches determinant, but still fails immediately by Bowen-Franks group mismatch:
  - `A -> P1`: `(1, -12) vs (2, -6)`
  - `P1 -> B`: `(2, -6) vs (1, -12)`
- `P2` fails immediately by determinant mismatch

Controls checked:

- `C1 = [[1, 3], [4, 1]]`
- `C2 = [[1, 2], [6, 1]]`
- `C3 = [[1, 1], [12, 1]]`

Outcome:

- `A -> C1` and `C3 -> B` are `equivalent lag=1` via permutation shortcut
- `C1 -> B` and `A -> C3` are `unknown` with the same telemetry as the direct baseline
- `C2` also fails immediately by Bowen-Franks mismatch

Interpretation:

- even when the top proposal lands exactly on an integer sample, the current ranking still does not predict invariant-compatible offline waypoint usefulness.

## Conclusion

Negative result:

- for `brix_k3`, the top-ranked phase-1 positive-conjugacy proposals are **not** measurably useful as exact offline waypoint candidates;
- they are rejected immediately by endpoint invariants, so they do not even reach bounded graph-only or mixed frontier work;
- nearby determinant-matched controls do not beat the direct pair either, because they only strip off permutation-equivalent endpoints and leave the hard segment unchanged;
- the `k=4` spot-check points in the same direction: even the exact integer-shadow proposal dies on a stronger invariant.

Current reading:

- the phase-1 proposal family/ranking is **not yet predictive** for exact offline waypoint usefulness;
- if this direction is pursued further, it likely needs invariant-aware reprojection or reinterpretation as local move seeds rather than literal intermediate endpoint targets.
