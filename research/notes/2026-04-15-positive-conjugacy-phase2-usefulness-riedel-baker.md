# Sampled positive-conjugacy phase 2 follow-up: Riedel/Baker exact-waypoint usefulness (2026-04-15)

## Question

Does the negative phase-2 exact-waypoint result from the Brix-Ruiz family
persist on non-Brix `2x2` literature positives, specifically the Riedel/Baker
family?

Scope stayed narrow:

- exact offline waypoint usefulness only;
- no `search.rs` integration;
- no reinterpretation as general proposal/seed usefulness.

## Cases

Used the literature family from
[2026-04-15-non-brix-ruiz-sse-pairs.md](./2026-04-15-non-brix-ruiz-sse-pairs.md):

```text
A_k = [[k,   2],
       [1,   k]]

B_k = [[k-1, 1],
       [1,   k+1]]
```

Evaluated:

- `riedel_baker_k2`
- `riedel_baker_k3`
- `riedel_baker_k10`

## Commands

Built the evaluator once:

```bash
cargo build --features research-tools --bin evaluate_positive_conjugacy_usefulness
```

Ran bounded graph-only checks:

```bash
timeout -k 10s 20s target/debug/evaluate_positive_conjugacy_usefulness -- \
  --case riedel_baker_k2 \
  --max-conjugator-entry 4 \
  --top-k 4 \
  --controls-limit 4 \
  --graph-max-lag 3 \
  --graph-max-dim 3 \
  --graph-max-entry 3

timeout -k 10s 20s target/debug/evaluate_positive_conjugacy_usefulness -- \
  --case riedel_baker_k3 \
  --max-conjugator-entry 4 \
  --top-k 4 \
  --controls-limit 4 \
  --graph-max-lag 4 \
  --graph-max-dim 3 \
  --graph-max-entry 4

timeout -k 10s 30s target/debug/evaluate_positive_conjugacy_usefulness -- \
  --case riedel_baker_k10 \
  --max-conjugator-entry 4 \
  --top-k 4 \
  --controls-limit 4 \
  --graph-max-lag 10 \
  --graph-max-dim 3 \
  --graph-max-entry 11
```

## Results

Shared witness fact:

- all three cases produced a bounded sampled positive-conjugacy witness;
- the witness was the same unipotent shear in every run:
  `G = [[1, 1], [0, 1]]`;
- each run produced `6` unique rounded-sample proposals.

### `riedel_baker_k2`

Endpoint:

- `A = [[2, 2], [1, 2]]`
- `B = [[1, 1], [1, 3]]`

Direct baseline:

- graph-only `lag<=3`, `dim<=3`, `entry<=3`
- `A -> B` is `equivalent lag=3`

Top-ranked proposals:

- `P1 = [[1, 2], [1, 2]]`
- `P2 = [[1, 2], [1, 3]]`
- `P3 = [[2, 2], [1, 3]]`
- `P4 = [[1, 1], [1, 2]]`

Outcome:

- all top proposals fail endpoint invariants immediately;
- none survive to frontier work;
- failures are by trace or determinant mismatch.

Local controls (same trace/determinant):

- `C1 = [[2, 1], [2, 2]]`
- `C2 = [[3, 1], [1, 1]]`

Control outcome:

- both controls succeed as bounded exact waypoints;
- in each case one segment is a permutation shortcut and the residual segment is
  a short lag-`2` or lag-`3` equivalence.

Reading:

- even on an easy family member where exact offline waypoints exist nearby, the
  rounded sampled positive-conjugacy proposals miss them completely.

### `riedel_baker_k3`

Endpoint:

- `A = [[3, 2], [1, 3]]`
- `B = [[2, 1], [1, 4]]`

Direct baseline:

- graph-only `lag<=4`, `dim<=3`, `entry<=4`
- `A -> B` is `unknown`
- telemetry: `visited=38`, `frontier_expanded=18`, `candidates=52`,
  `pruned=52`, `max_frontier=20`

Top-ranked proposals:

- `P1 = [[2, 2], [1, 3]]`
- `P2 = [[2, 2], [1, 4]]`
- `P3 = [[3, 2], [1, 4]]`
- `P4 = [[2, 1], [1, 3]]`

Outcome:

- all top proposals fail endpoint invariants immediately;
- none survive to frontier work;
- again the failures are already visible at trace/determinant level.

Local controls (same trace/determinant):

- `C1 = [[3, 1], [2, 3]]`
- `C2 = [[4, 1], [1, 2]]`

Control outcome:

- `A -> C1` and `C2 -> B` are lag-`1` permutation shortcuts;
- the residual segments `C1 -> B` and `A -> C2` stay `unknown` with the same
  telemetry as the direct baseline.

Reading:

- this is the same exact-waypoint failure pattern seen on Brix-Ruiz:
  proposals die on invariants, while the tiny invariant-compatible controls only
  peel off permutation-equivalent endpoints and leave the hard residual pair.

### `riedel_baker_k10`

Endpoint:

- `A = [[10, 2], [1, 10]]`
- `B = [[9, 1], [1, 11]]`

Direct baseline:

- graph-only `lag<=10`, `dim<=3`, `entry<=11`
- `A -> B` is `unknown`
- telemetry: `visited=101`, `frontier_expanded=46`, `candidates=144`,
  `pruned=143`, `max_frontier=55`

Top-ranked proposals:

- `P1 = [[9, 2], [1, 10]]`
- `P2 = [[9, 2], [1, 11]]`
- `P3 = [[10, 2], [1, 11]]`
- `P4 = [[9, 1], [1, 10]]`

Outcome:

- all top proposals fail endpoint invariants immediately;
- none survive to frontier work.

Local controls (same trace/determinant):

- `C1 = [[10, 1], [2, 10]]`
- `C2 = [[11, 1], [1, 9]]`

Control outcome:

- `A -> C1` and `C2 -> B` are lag-`1` permutation shortcuts;
- the residual segments `C1 -> B` and `A -> C2` stay `unknown` with the same
  telemetry as the direct baseline.

Reading:

- the Brix-style negative result persists at a higher non-Brix `k` as well.

## Conclusion

Sampled positive conjugacy does produce bounded witnesses on the tested
Riedel/Baker cases, but the current rounded-sample proposal family is still not
a practical cross-family exact offline waypoint heuristic.

The important split is:

- sampled positive-conjugacy witness existence generalizes beyond Brix-Ruiz;
- exact-waypoint usefulness of the current discrete proposals does not.

On `k=3` and `k=10`, the failure mode matches the Brix-Ruiz phase-2 result:
top-ranked proposals are rejected by endpoint invariants immediately, while
nearby invariant-compatible controls only remove permutation-equivalent
endpoints and leave the hard residual pair unchanged under the current bound.

`k=2` is a useful sanity check in the opposite direction: exact offline
waypoints do exist nearby, but the sampled positive-conjugacy proposals still do not
find them. That makes the present negative result stronger than “the family is
too hard”; the proposal projection/ranking itself is the issue under the exact
waypoint interpretation.
