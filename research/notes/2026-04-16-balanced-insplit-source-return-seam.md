# Balanced `3x3(in) -> 2x2 <-balanced-> 2x2 -> 3x3(out)` source seam (2026-04-16)

## Question

After both bounded return-family variants stay toy-positive but Brix-empty, does
changing only the bounded source of `2x2` bridge states before the balanced hop
change the picture?

## Slice

This round keeps the balanced hop and the bounded `2x2 -> 3x3` out-split return
family from the earlier return seam, but swaps the pre-hop source family:

- start from canonical one-step `3x3` in-split states of the original `2x2`
  endpoints;
- factor each source state down by bounded `3x3 -> 2x2` factorizations;
- from each resulting canonical `2x2` bridge state, take one bounded balanced
  neighbor hop;
- return the post-balanced bridge to canonical `3x3` states using one-step
  `2x2 -> 3x3` out-splits;
- record a hit when that returned `3x3` state is already in the other side's
  canonical one-step out-split set.

No new library helper was needed. The existing
`enumerate_balanced_bridge_return_neighbors_3x3` helper already works for an
arbitrary canonical `3x3` source, so this round only adds:

- `find_balanced --bridge-insplit-source-return-seam` in
  `src/bin/find_balanced.rs`;
- focused seam coverage in `src/balanced.rs`.

This keeps the round sidecar-only, does not widen the main solver, and does not
reopen endpoint-local `2x2` cap ramps.

## Exact shape

For a canonical `3x3` in-split source state `C`, bridge cap `E_bridge`, and
bounded balanced config `(m_max, E_bal)`, the move family is:

```text
C(in) -> 2x2 bridge --balanced-> 2x2 bridge -> 3x3 canonical out-split state
```

More explicitly:

1. enumerate canonical one-step `3x3` in-split source states on each side;
2. for one source state `C`, enumerate bounded `3x3 -> 2x2` factorizations
   with bridge-entry cap `E_bridge`;
3. canonicalize the resulting `2x2` bridge states;
4. from each bridge state, enumerate bounded same-size balanced neighbors using
   `BalancedSearchConfig2x2`;
5. out-split each balanced target bridge back to canonical `3x3` states;
6. record a directional hit when the returned `3x3` state lies in the other
   side's canonical out-split target set.

The probe reports both directions:

- `A(in) -> B(out)`;
- `B(in) -> A(out)`.

## Bounds Used

Toy control:

- `bridge_max_entry = 1`
- `max_common_dim = 1`
- `max_entry = 1`

Brix-Ruiz `k = 3` control:

- `bridge_max_entry = 8`
- `max_common_dim = 2`
- `max_entry = 8`

Focused unit coverage also checks bounded `k = 4` at the same caps.

## Evidence

### Toy control

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case toy --max-common-dim 1 --max-entry 1 \
  --bridge-max-entry 1 --bridge-insplit-source-return-seam
```

Result:

- each side has `2` canonical `3x3` in-split source states and `2` canonical
  `3x3` out-split target states;
- there are `4` directional `A(in) -> B(out)` hits and `4` directional
  `B(in) -> A(out)` hits;
- every directional hit again uses the same bounded balanced bridge hop
  `[[0,1],[0,1]] --balanced(S = [1, 1]^T)-> [[1,0],[1,0]]`.

Interpretation:

- swapping the source family from out-splits to in-splits does not destroy the
  bounded toy seam;
- the alternate source seam is real, not just an artifact of the earlier
  out-split source choice.

### Brix-Ruiz `k = 3`

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k3 --max-common-dim 2 --max-entry 8 \
  --bridge-max-entry 8 --bridge-insplit-source-return-seam
```

Result:

- there is still no direct bounded balanced witness on the endpoints;
- the canonical `3x3` state sets remain small:
  `7` in-split source states on the `A` side, `9` in-split source states on the
  `B` side, `7` out-split target states on the `A` side, and `9` out-split
  target states on the `B` side;
- the directional hit count is `0` in both directions.

Interpretation:

- changing only the bounded source family before the balanced hop still does not
  connect the two bounded `3x3` state families on `brix_k3`;
- the earlier negative picture was not just an artifact of using out-split
  source states.

### Brix-Ruiz `k = 4`

Focused unit coverage keeps the same seam empty at bounded `k = 4` as well:

- `9` in-split source states on the `A` side versus `15` out-split target
  states on the `B` side;
- `15` in-split source states on the `B` side versus `9` out-split target
  states on the `A` side;
- the directional hit count stays `0` in both directions.

## Conclusion

This round answers the next small balanced follow-up cleanly:

- the alternate insplit-source seam is genuinely non-empty on the toy control;
- bounded `brix_k3` still stays empty;
- focused bounded `k4` coverage stays empty too.

So changing the bounded source of `2x2` bridge states before the balanced hop
does not change the picture at these caps. That is enough to close out this
source-side variant without widening the main solver or reopening broader
endpoint-local `2x2` searches.
