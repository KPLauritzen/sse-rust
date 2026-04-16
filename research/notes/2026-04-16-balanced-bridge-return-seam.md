# Balanced `3x3 -> 2x2 <-balanced-> 2x2 -> 3x3` return seam (2026-04-16)

## Question

After the bounded balanced bridge-state seam
`A -> 3x3 -> 2x2 <-balanced-> 2x2 <- 3x3 <- B` lands with toy hits but no
Brix-Ruiz hits, does one bounded return step back to `3x3` expose a richer
same-size seam?

## Slice

This round keeps the balanced work sidecar-only and chooses one concrete
follow-up:

- start from canonical one-step `3x3` outsplit states of the original `2x2`
  endpoints;
- factor each `3x3` state down by bounded `3x3 -> 2x2` factorizations;
- from each resulting canonical `2x2` bridge state, take one bounded balanced
  neighbor hop;
- outsplit that balanced target bridge back up to canonical `3x3` states;
- record a hit when that returned `3x3` state is already in the other side's
  canonical one-step outsplit set.

The new reusable helper is:

- `enumerate_balanced_bridge_return_neighbors_3x3` in `src/balanced.rs`.

The probe surface is:

- `find_balanced --bridge-return-seam` in `src/bin/find_balanced.rs`.

This deliberately does not reopen endpoint-local `2x2` entry-cap ramps. The
balanced hop still only happens after a bounded `3x3 -> 2x2` sidecar step.

## Exact shape

For a canonical `3x3` start state `C`, bridge cap `E_bridge`, and bounded
balanced config `(m_max, E_bal)`, the move family is:

```text
C -> 2x2 bridge --balanced-> 2x2 bridge -> 3x3 canonical return state
```

More explicitly:

1. enumerate canonical one-step `3x3` outsplit start states on each side;
2. for one start state `C`, enumerate bounded `3x3 -> 2x2` factorizations with
   bridge-entry cap `E_bridge`;
3. canonicalize the resulting `2x2` bridge states;
4. from each such bridge state, enumerate bounded same-size balanced neighbors
   using `BalancedSearchConfig2x2`;
5. outsplit each balanced target bridge back to canonical `3x3` states;
6. record a directional hit when the returned `3x3` state lies in the other
   side's canonical outsplit-start set.

The actual probe reports both directions:

- `A`-side `3x3` starts returning into the `B`-side start set;
- `B`-side `3x3` starts returning into the `A`-side start set.

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
  --bridge-max-entry 1 --bridge-return-seam
```

Result:

- the direct bounded balanced witness still exists on the original `2x2`
  endpoints;
- each side has the same `2` canonical one-step `3x3` outsplit states;
- there are `4` directional `A -> B` return hits and `4` directional
  `B -> A` return hits;
- every directional hit uses the same bounded balanced bridge hop
  `[[0,1],[0,1]] --balanced(S = [1, 1]^T)-> [[1,0],[1,0]]`.

Interpretation:

- once the balanced bridge hop is allowed to return to `3x3`, the toy control
  gets a genuine same-dimension move family rather than only a `2x2` bridge-set
  observation;
- the seam is still tightly bounded and completely explicit.

### Brix-Ruiz `k = 3`

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k3 --max-common-dim 2 --max-entry 8 \
  --bridge-max-entry 8 --bridge-return-seam
```

Result:

- there is still no direct bounded balanced witness on the endpoints;
- the canonical one-step `3x3` outsplit start sets are still small:
  `7` states on the `A` side and `9` on the `B` side;
- the directional return-hit count is `0` in both directions.

Interpretation:

- allowing one bounded balanced bridge hop and then returning to `3x3` still
  does not connect the two bounded `3x3` outsplit families;
- the earlier negative bridge-state result was not just missing an obvious
  return step.

### Brix-Ruiz `k = 4`

Command:

```sh
cargo run --quiet --features research-tools --bin find_balanced -- \
  --case brix_k4 --max-common-dim 2 --max-entry 8 \
  --bridge-max-entry 8 --bridge-return-seam
```

Result:

- again there is no direct bounded balanced witness on the endpoints;
- the canonical one-step `3x3` outsplit start sets are `9` states on the
  `A` side and `15` on the `B` side;
- the directional return-hit count is `0` in both directions.

## Conclusion

This round lands one bounded richer balanced seam beyond the earlier
bridge-state-only probe:

- the toy control shows the seam is real at the `3x3` level;
- both Brix-Ruiz controls still stay empty at the stated bounds.

That is enough to rule out the next obvious balanced follow-up without going
back to endpoint-local `2x2` sweeps.

If balanced sidecar work continues from here, the next small options look more
like:

- a different bounded source of `2x2` bridge states before the balanced hop; or
- a different bounded return family from the post-balanced `2x2` bridge.
