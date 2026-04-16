# Balanced same-size neighbor / zig-zag first slice (2026-04-16)

## Question

What is the smallest useful step beyond direct same-size balanced-elementary
`2x2` witnesses: a reusable bounded proposal surface, or a bounded same-size
zig-zag search?

## Slice

This round lands the smallest reusable balanced-sidecar surface first:

- enumerate distinct nontrivial same-size balanced-elementary `2x2` neighbors
  of one matrix under bounded `S (2xm)` and bounded entries;
- build one concrete zig-zag shape on top of that surface:
  `2x2 <-balanced-> 2x2 <-balanced-> 2x2`, where the middle `2x2` bridge is a
  shared nontrivial neighbor of both endpoints;
- keep the work sidecar-only in `src/balanced.rs` and
  `src/bin/find_balanced.rs`.

The new neighbor surface is still same-size and still bounded by the existing
`BalancedSearchConfig2x2`, but it no longer needs a pre-chosen target matrix.
That is the smallest proposal-oriented upgrade from the old direct witness
search.

## Exact shape

For a fixed source matrix `A`, common dimension `m`, and common left factor
`S`, the old direct search solved

- `A = S R_A`,
- `B = S R_B`,
- `R_A S = R_B S`,

for one specific target `B`.

The new surface keeps the same equations but fixes only the source side first:

1. enumerate all bounded `R_A` with `A = S R_A`;
2. compute `X = R_A S`;
3. enumerate all bounded `R_C` with `R_C S = X`;
4. emit `C = S R_C` as a nontrivial same-size balanced neighbor when `C != A`.

This naturally reuses the `S` side. For each fixed `S`, the implementation now
caches row-solution sets for the `r S = x_row` equations needed to reconstruct
all bounded `R_C` factors for that `S`.

The one-bridge zig-zag search is then just intersection of the bounded
neighbor sets from the two endpoints.

## Bounds used

Toy control:

- `max_common_dim = 1`
- `max_entry = 1`

Brix-Ruiz controls:

- `max_common_dim = 2`
- `max_entry = 8`
- small follow-up ramp: `max_entry = 10`, then `12`

## Evidence

### Toy control

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case toy --max-common-dim 1 --max-entry 1 --neighbors --zigzag
```

Result:

- direct balanced witness still exists;
- the new neighbor surface is non-empty but tiny:
  each side has exactly `1` nontrivial neighbor, namely the other endpoint;
- there is no nontrivial one-bridge same-size zig-zag, because the neighbor
  sets are `{B}` and `{A}` rather than a shared middle state.

Interpretation:

- the new surface is real and testable, not just a reformulation of the old
  target-specific search;
- at this smallest toy bound it behaves like a directed proposal edge set, not
  like an already-rich `2x2` component.

### Brix-Ruiz `k = 3`

Commands:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k3 --max-common-dim 2 --max-entry 8 --neighbors --zigzag

cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k3 --max-common-dim 2 --max-entry 10 --neighbors

cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k3 --max-common-dim 2 --max-entry 12 --neighbors
```

Result:

- no direct balanced witness at `max_entry = 8`;
- `0` nontrivial same-size balanced neighbors from the `A` side;
- `0` nontrivial same-size balanced neighbors from the `B` side;
- therefore no bounded one-bridge same-size zig-zag meeting;
- raising the entry cap to `10` and `12` does not change the empty-neighborhood
  result.

### Brix-Ruiz `k = 4`

Commands:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k4 --max-common-dim 2 --max-entry 8 --neighbors --zigzag

cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k4 --max-common-dim 2 --max-entry 10 --neighbors

cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k4 --max-common-dim 2 --max-entry 12 --neighbors
```

Result:

- same outcome as `k = 3`:
  no direct witness, `0` nontrivial neighbors on either side, and no bounded
  one-bridge meeting;
- the small entry-cap ramp again leaves the neighborhood empty.

## Conclusion

This is a sharper negative result than the old direct-witness-only sidecar.

Before this slice, the claim was only that the two Brix-Ruiz endpoints do not
share one bounded same-size balanced-elementary witness. After this slice, the
bounded same-size proposal surface itself is empty on both sides for the tested
controls, so a same-size balanced-elementary attack still has no local room to
start.

That makes the next balanced follow-up clearer:

- either allow a genuinely richer balanced zig-zag than one same-size bridge,
- or use balanced data only as a proposal seam once a dimension-changing bridge
  exists elsewhere.
