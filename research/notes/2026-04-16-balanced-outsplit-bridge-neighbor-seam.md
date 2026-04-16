# Balanced outsplit-bridge neighbor seam (2026-04-16)

## Question

What is the smallest richer balanced follow-up after the same-size `2x2`
balanced-neighbor surface turns out to be empty on the Brix-Ruiz controls?

## Slice

This round keeps the balanced work sidecar-only and chooses one concrete,
proposal-oriented seam:

- first build bounded canonical `2x2` bridge states by one
  `2x2 -> 3x3 -> 2x2` out-split/factorisation step;
- then use the existing bounded same-size balanced-neighbor surface only on
  those bridge states;
- stop there, without returning to `3x3` or wiring anything into the main
  solver.

The new reusable helpers are:

- `enumerate_outsplit_bridge_states_2x2` in `src/balanced.rs`;
- `enumerate_balanced_neighbor_set_hits_2x2` in `src/balanced.rs`;
- `find_balanced --bridge-neighbor-seam` in `src/bin/find_balanced.rs`.

This deliberately avoids reopening the old endpoint-local same-size `2x2`
entry-cap ramps. The only same-size balanced search now happens on bridge
states that already arose from one dimension-changing sidecar seam.

## Exact shape

For endpoint matrices `A` and `B`, bridge bound `E_bridge`, and bounded
balanced config `(m_max, E_bal)`, the seam is:

```text
A -> 3x3 out-split -> 2x2 bridge --balanced-neighbor-> 2x2 bridge <- 3x3 out-split <- B
```

More explicitly:

1. enumerate all one-step `2x2 -> 3x3` out-splits of `A` and `B`;
2. factor each `3x3` back down by bounded `3x3 -> 2x2` factorizations with
   bridge-entry cap `E_bridge`;
3. canonicalize the resulting `2x2` bridge states on each side;
4. from each `A`-side bridge state, enumerate bounded same-size balanced
   neighbors using `BalancedSearchConfig2x2`;
5. record a hit when one of those balanced neighbors is already in the
   `B`-side bridge-state set.

This is not a direct target-specific balanced-witness search on the original
endpoints. Balanced data is only used after a bounded dimension-changing bridge
state already exists.

## Bounds used

Toy control:

- `bridge_max_entry = 1`
- `max_common_dim = 1`
- `max_entry = 1`

Brix-Ruiz `k = 3` and `k = 4` controls:

- `bridge_max_entry = 8`
- `max_common_dim = 2`
- `max_entry = 8`

## Evidence

### Toy control

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case toy --max-common-dim 1 --max-entry 1 \
  --bridge-max-entry 1 --bridge-neighbor-seam
```

Result:

- the direct balanced witness still exists;
- both sides have the same `4` canonical bridge states:
  `[[0,0],[0,1]]`, `[[0,0],[1,1]]`, `[[0,0],[2,1]]`, `[[0,1],[0,1]]`;
- there are exactly `2` bounded balanced bridge-neighbor hits:
  `[[0,0],[0,1]] -> [[0,0],[1,1]]` and the reverse hit back;
- both hits use the same tiny common factor `S = [0, 1]^T`.

Interpretation:

- once a dimension-changing bridge state exists, the balanced sidecar is no
  longer forced to act only on the original endpoints;
- on the toy control this richer seam is genuinely non-empty, so it is more
  than a restatement of the earlier endpoint-local same-size negative result.

### Brix-Ruiz `k = 3`

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k3 --max-common-dim 2 --max-entry 8 \
  --bridge-max-entry 8 --bridge-neighbor-seam
```

Result:

- there is still no direct bounded balanced witness on the endpoints;
- the `A` side has `1` canonical bridge state: `[[1,2],[3,1]]`;
- the `B` side has `3` canonical bridge states:
  `[[0,1],[5,2]]`, `[[0,5],[1,2]]`, `[[1,1],[6,1]]`;
- the balanced bridge-neighbor hit count is `0`.

Interpretation:

- the earlier same-size endpoint-local emptiness was not just missing one
  nearby outsplit bridge;
- even after allowing one bounded dimension-changing bridge step first, the
  resulting `2x2` bridge states still do not admit a bounded one-hop balanced
  proposal seam across the two sides.

### Brix-Ruiz `k = 4`

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k4 --max-common-dim 2 --max-entry 8 \
  --bridge-max-entry 8 --bridge-neighbor-seam
```

Result:

- again no direct bounded endpoint witness;
- the `A` side has `1` canonical bridge state: `[[1,3],[4,1]]`;
- the `B` side has `2` canonical bridge states:
  `[[0,1],[11,2]]`, `[[1,1],[12,1]]`;
- the balanced bridge-neighbor hit count is `0`.

## Conclusion

This round lands one bounded richer balanced seam beyond same-size endpoint
neighbors, and it stays small enough to support with tests and direct sidecar
commands.

The conclusion is mixed but useful:

- toy shows the seam is real: balanced proposals can move between bounded
  outsplit bridge states;
- both Brix-Ruiz controls stay empty on that seam at the stated bounds.

So the next balanced follow-up, if any, should not go back to the old same-size
endpoint sweeps. The remaining obvious options are either:

- return from the balanced bridge state back up to `3x3`, i.e.
  `3x3 -> 2x2 <-balanced-> 2x2 -> 3x3`;
- or wait for a different small bridge source and use the same bounded
  bridge-state balanced proposal surface there.
