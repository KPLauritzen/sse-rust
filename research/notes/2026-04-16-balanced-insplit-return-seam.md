# Balanced `3x3(out) -> 2x2 <-balanced-> 2x2 -> 3x3(in)` return seam (2026-04-16)

## Question

After the bounded balanced bridge-return seam
`3x3(out) -> 2x2 <-balanced-> 2x2 -> 3x3(out)` lands with toy hits but no
Brix-Ruiz hits, does swapping only the bounded `2x2 -> 3x3` return family from
out-splits to in-splits expose a mixed bounded seam?

## Slice

This round keeps the balanced work sidecar-only and changes exactly one piece
of the previous return seam:

- start from canonical one-step `3x3` out-split states of the original `2x2`
  endpoints;
- factor each `3x3` state down by bounded `3x3 -> 2x2` factorizations;
- from each resulting canonical `2x2` bridge state, take one bounded balanced
  neighbor hop;
- return the post-balanced bridge to canonical `3x3` states using one-step
  `2x2 -> 3x3` in-splits instead of out-splits;
- record a hit when that returned `3x3` state is already in the other side's
  canonical one-step in-split set.

The new reusable helper is:

- `enumerate_balanced_bridge_insplit_return_neighbors_3x3` in
  `src/balanced.rs`.

The probe surface is:

- `find_balanced --bridge-insplit-return-seam` in `src/bin/find_balanced.rs`.

This deliberately does not reopen endpoint-local `2x2` entry-cap ramps and
does not widen the main solver.

## Exact shape

For a canonical `3x3` out-split source state `C`, bridge cap `E_bridge`, and
bounded balanced config `(m_max, E_bal)`, the move family is:

```text
C(out) -> 2x2 bridge --balanced-> 2x2 bridge -> 3x3 canonical in-split state
```

More explicitly:

1. enumerate canonical one-step `3x3` out-split start states on each side;
2. for one start state `C`, enumerate bounded `3x3 -> 2x2` factorizations with
   bridge-entry cap `E_bridge`;
3. canonicalize the resulting `2x2` bridge states;
4. from each bridge state, enumerate bounded same-size balanced neighbors using
   `BalancedSearchConfig2x2`;
5. in-split each balanced target bridge back to canonical `3x3` states;
6. record a directional hit when the returned `3x3` state lies in the other
   side's canonical one-step in-split start set.

The actual probe reports both directions:

- `A(out) -> B(in)`;
- `B(out) -> A(in)`.

## Bounds used

Toy control:

- `bridge_max_entry = 1`
- `max_common_dim = 1`
- `max_entry = 1`

Brix-Ruiz `k = 3` control:

- `bridge_max_entry = 8`
- `max_common_dim = 2`
- `max_entry = 8`

Focused unit coverage also checks `k = 4` at the same bounded Brix cap and
stays empty there as well.

## Evidence

### Toy control

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case toy --max-common-dim 1 --max-entry 1 \
  --bridge-max-entry 1 --bridge-insplit-return-seam
```

Result:

- each side has `2` canonical `3x3` out-split source states and `2` canonical
  `3x3` in-split target states;
- there are `4` directional `A(out) -> B(in)` hits and `4` directional
  `B(out) -> A(in)` hits;
- every directional hit uses the same bounded balanced bridge hop
  `[[0,1],[0,1]] --balanced(S = [1, 1]^T)-> [[1,0],[1,0]]`.

Interpretation:

- the mixed return seam is real on the toy control, not just the earlier
  out-split-only return seam;
- changing the return family does not destroy the bounded constructive picture
  on the toy example.

### Brix-Ruiz `k = 3`

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k3 --max-common-dim 2 --max-entry 8 \
  --bridge-max-entry 8 --bridge-insplit-return-seam
```

Result:

- there is still no direct bounded balanced witness on the endpoints;
- the canonical one-step `3x3` state sets remain small:
  `7` out-split source states on the `A` side, `9` out-split source states on
  the `B` side, `7` in-split target states on the `A` side, and `9` in-split
  target states on the `B` side;
- the directional hit count is `0` in both directions.

Interpretation:

- changing only the bounded return family from out-split to in-split still
  does not connect the two bounded `3x3` state families on `brix_k3`;
- the previous negative result was not just an artifact of returning through
  the out-split family.

## Conclusion

This round lands one alternate bounded continuation after the merged out-split
return seam:

- the toy control again shows a genuine bounded seam;
- bounded `brix_k3` still stays empty;
- focused unit coverage also keeps bounded `brix_k4` empty.

That is enough to rule out the next obvious return-family variant without
reopening endpoint-local `2x2` sweeps or widening the main solver.

If balanced sidecar work continues from here, the next small option is more
likely a different bounded source of `2x2` bridge states before the balanced
hop than another return-family tweak.
