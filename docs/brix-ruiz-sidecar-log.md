# Brix-Ruiz Sidecar Log

This file holds the detailed experiment log for the Brix-Ruiz family so that [TODO.md](TODO.md) can stay focused on current roadmap items.

## Positive conjugacy paths for similar matrices

The Brix-Ruiz family in [references/brix-ruiz-2025-2504.09889.pdf](../references/brix-ruiz-2025-2504.09889.pdf) is not just shift equivalent: Example 3.8 states that

- `A_k = [[1, k], [k-1, 1]]` and `B_k = [[1, k(k-1)], [1, 1]]` are similar over `Z`
- the explicit conjugating matrix is `P_k = [[k-1, k], [1, 1]]`
- only `k = 2, 3` are currently confirmed SSE

That makes `brix_ruiz_k3` a good candidate for a different attack than bounded ESSE BFS. The relevant local theory is Boyle-Kim-Roush's path-method paper [references/boyle-kim-roush-2013-1209.5096.pdf](../references/boyle-kim-roush-2013-1209.5096.pdf), which treats positive paths inside a conjugacy class as an SSE substrate.

Concrete experiment:

- build an experimental search for short positive conjugacy paths `G_t^-1 A G_t` between similar `2x2` matrices, seeded by the explicit `P_k`
- parameterize candidate conjugacies by short products of elementary shears or diagonal scalings instead of ESSE factorizations
- test first on `k = 3`, where SSE is known, before spending effort on `k = 4`

Outcome:

- the experimental positive-conjugacy search finds a much simpler witness than the generic `P_k` similarity for the Brix-Ruiz family
- for `k = 3`, `diag(1, 2)^-1 A diag(1, 2) = B`
- for `k = 4`, `diag(1, 3)^-1 A diag(1, 3) = B`
- the sampled affine paths `((1-t)I + tD)^-1 A ((1-t)I + tD)` stay strictly positive for both cases

This does not prove SSE over `Z_+`, and therefore should not be wired into `search_sse` as a correctness shortcut. It does, however, suggest constructive searches built from diagonal refactorizations or balanced witnesses rather than generic BFS expansion.

## Balanced elementary search

Follow-up experiment:

- implement a bounded search for balanced elementary equivalence witnesses `(R_A, S, R_B)` from Brix (2022), Definition 5.4
- test whether `brix_ruiz_k3` is reachable by a short balanced chain, and whether the same bounded witness family sees useful structure on `k = 4`

Outcome:

- a bounded `2x2` balanced-elementary search exists locally and is validated on a nontrivial toy example
- it exhausts on both `brix_ruiz_k3` and `brix_ruiz_k4` for `max_common_dim = 2`, `max_entry = 8`
- this is consistent with the matrix-size bound in Brix (2022): for `2x2` matrices, a direct balanced elementary witness can only factor through a common graph of size at most `2`
- since the Brix-Ruiz matrices are invertible and distinct, a direct same-size balanced elementary witness is structurally unlikely to be the right attack

## One-step out-split refinements

Next move:

- implement explicit small graph moves, starting with `2x2 -> 3x3` out-splits via division matrices and edge-matrix factorizations
- search whether out-splits of the Brix-Ruiz `2x2` pair admit a short balanced chain or another highly structured common refinement in size `3`

Outcome:

- explicit `2x2 -> 3x3` out-split enumeration exists locally
- for `brix_ruiz_k3`, the search finds `42` out-splits on the `A` side and `54` on the `B` side
- for `brix_ruiz_k4`, the counts are `54` and `90`, respectively
- there is no common one-step `3x3` out-split refinement up to permutation for either pair
- after factoring those `3x3` out-splits back down with the existing bounded `3x3 -> 2x2` search, there is still no shared `2x2` bridge under `max_entry = 8`

This rules out the smallest obvious graph-move patterns around the Brix-Ruiz family.

## Two-step out-split chains

Follow-up:

- search two-step split chains rather than one-step refinements
- either `2x2 -> 3x3 -> 4x4` out-splits on both sides looking for a common canonical refinement, or bounded search among `3x3` out-splits for a short `3x3 <-> 2x2 <-> 3x3` zig-zag

Outcome:

- the graph-move sidecar has a generic one-step out-split enumerator for square matrices, so it can probe both `2x2 -> 3x3` and `3x3 -> 4x4`
- for `brix_ruiz_k3`, the two-step search explores `17856` second-step out-splits from the `A` side and `29304` from the `B` side, collapsing to `98` and `161` canonical `4x4` refinements respectively
- for `brix_ruiz_k4`, the corresponding counts are `34920` and `90000`, collapsing to `200` and `509` canonical `4x4` refinements
- there is still no common two-step out-split refinement up to permutation for either pair

## Bridge-zig-zag probes

Outcome:

- the first-step `3x3` out-splits still factor back down to only a handful of canonical `2x2` bridge states:
  `1` versus `3` for `k = 3`, and `1` versus `2` for `k = 4`
- running the main bounded `2x2` solver on every bridge-state pair with `max_lag = 6`, `max_intermediate_dim = 3`, `max_entry = 25` does not produce a connection
- all of those bridge-pair searches stay `unknown`, with about `2555` frontier expansions per pair for `k = 3` and `2643` per pair for `k = 4`

This makes the obstruction sharper: the missing witness is not a tiny common refinement and not a tiny bridge pair inside the existing `2x2` BFS universe.

## Direct `3x3` zig-zag search

Next move:

- probe richer `3x3`-level move families directly rather than forcing them through exact common refinements or exact `2x2` bridge states
- search on the `3x3` out-split states themselves, prioritising short zig-zags that alternate `3x3 -> 2x2` amalgamations and `2x2 -> 3x3` out-splits

Outcome:

- there is a reusable sidecar move generator for one `3x3 -> 2x2 -> 3x3` zig-zag step, built from the existing bounded `3x3 -> 2x2` factorisations followed by explicit `2x2 -> 3x3` out-splits
- after canonicalising the one-step out-splits, the initial `3x3` state sets are already small:
  `7` states on the `A` side and `9` on the `B` side for `k = 3`,
  `9` and `15` for `k = 4`
- at `bridge_max_entry = 8`, each canonical `A`-side `3x3` state has exactly `7` zig-zag neighbors for `k = 3`, and those neighbors stay inside the same `7`-state set
- similarly, each canonical `A`-side `3x3` state has exactly `9` zig-zag neighbors for `k = 4`, again staying inside the same `9`-state set
- so the `A`-side start sets are already closed under this bounded `3x3 -> 2x2 -> 3x3` move, and since they are disjoint from the `B`-side start sets there is no bounded zig-zag meeting at this level
- raising the bridge bound from `8` to `10` and `12` does not change that closure behavior

This is more informative than the earlier negative probes: the current bounded `3x3` graph move is not merely failing to connect the two sides, it appears to preserve small isolated components around each family.

## Mixed split directions

Next move:

- enlarge the bounded `3x3` move generator itself
- add a second kind of `3x3` graph move beyond out-splits so the sidecar graph is not trapped in those tiny closed components

Outcome:

- a dual sidecar move now enumerates one-step in-splits by out-splitting the transpose and transposing back
- for both Brix-Ruiz pairs, the one-step in-split counts match the one-step out-split counts:
  `42/42` on the `A/B` sides for `k = 3`, and `54/90` for `k = 4`
- after canonicalisation, the in-split state sets are genuinely different from the out-split state sets on each side, so this is a real enlargement of the move family rather than a tautological reparameterisation
- nevertheless, there is still no common one-step `3x3` refinement in any of the four combinations:
  out/out, in/in, out/in, or in/out

This rules out another obvious escape hatch: the obstruction is not simply that we were only splitting on the outgoing side.

## Two-step mixed split refinements

Final sidecar widening in this branch:

- test whether two-step `3x3` refinements built from either out-splits or in-splits admit a common `4x4` state

Outcome:

- allowing either split direction doubles the first-step `3x3` refinement universe, but it still stays small:
  `14` versus `18` canonical first-step states for `k = 3`, and `18` versus `30` for `k = 4`
- pushing one more mixed split step produces much larger canonical `4x4` refinement sets:
  `320` versus `518` for `k = 3`, and `613` versus `1681` for `k = 4`
- even with that enlarged move family, the two sides remain disjoint: there is still no common two-step mixed split refinement

## Current conclusion

The sidecar evidence is now consistent across every small structured move family tried so far:

- direct balanced witnesses fail
- one-step out-split refinements fail
- two-step out-split refinements fail
- bounded `3x3 -> 2x2 -> 3x3` zig-zag components stay isolated
- one-step mixed out/in refinements fail
- two-step mixed out/in refinements fail

That is enough reason to stop widening the split-sidecar graph blindly for now and move back to the main solver, using this structure as guidance for a search heuristic or pruning signal.
