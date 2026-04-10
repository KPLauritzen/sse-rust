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

## Lind-Marcus/Baker lag-7 witness

New reference:

- [references/Lind-Marcus2021.pdf](../references/Lind-Marcus2021.pdf), Example 7.3.12, gives K. Baker's explicit strong shift equivalence of lag `7` between
  `A = [[1, 3], [2, 1]]` and `B = [[1, 6], [1, 1]]`.

Useful bounds from the displayed witness:

- the largest factor entry is `5`, occurring in the final `3x3 -> 2x2` step
- the largest intermediate matrix size is `4x4`
- therefore `max_lag = 7`, `max_intermediate_dim = 4`, `max_entry = 5` is already enough to contain the displayed elementary SSE witness if the move generator can generate the relevant factorisations

Main-solver probes with `max_intermediate_dim = 4`:

- `max_lag = 5`, `max_entry = 6`: `unknown`, elapsed `10.939s`
- `max_lag = 6`, `max_entry = 6`: `unknown`, elapsed `21.129s`
- `max_lag = 7`, `max_entry = 6`: `unknown`, elapsed `50.812s`
- `max_lag = 7`, `max_entry = 5`: `unknown`, elapsed `19.630s`

The `max_entry = 5` failure is the important result: the search is not missing the path because the entry bound is too small.

A diagnostic binary now exists at [`src/bin/check_lind_marcus_path.rs`](../src/bin/check_lind_marcus_path.rs). It encodes the seven displayed `(U, V)` factors, checks each `UV -> VU` transition, and asks whether the current one-step generator can produce each transition up to permutation with `max_intermediate_dim = 4`, `max_entry = 5`.

Diagnostic output:

- step `1`, `2x2 -> 3x3`: covered by `rectangular_factorisation_2x3` and `same_past_outsplit_2x2_to_3x3`
- step `2`, `3x3 -> 4x4`: missing
- step `3`, `4x4 -> 4x4`: covered by `elementary_conjugation`
- step `4`, `4x4 -> 4x4`: covered by `elementary_conjugation`
- step `5`, `4x4 -> 4x4`: missing
- step `6`, `4x4 -> 3x3`: missing
- step `7`, `3x3 -> 2x2`: covered by `rectangular_factorisation_3x3_to_2`

Interpretation:

- the missing steps are elementary SSE factorisations in the Baker witness, but they are not one-step graph state splits or one-step graph state amalgamations in the matrix sense currently implemented
- step `2` is not a one-step in/out split: the target `4x4` matrix has no duplicate row or duplicate column pair, even up to permutation
- step `5` is same-size, so it is not a one-step state split or amalgamation
- step `6` contracts from `4x4` to `3x3`, but the displayed factor is not a division/amalgamation matrix; one row has two `1`s
- this does not contradict the Decomposition Theorem: an elementary SSE factorisation can require a longer decomposition into graph splitting and amalgamation codes

Next reasonable experiments:

- search for graph-only paths between the missing waypoints `A1 -> A2`, `A4 -> A5`, and `A5 -> A6`, allowing `max_intermediate_dim = 5` before widening the full `A -> B` search
- add a graph-only search mode that disables generic factorisation and conjugation moves, then try iterative bounds such as `max_dim = 5`, lag `8..12`
- if the goal is to reproduce Baker's displayed lag-7 witness rather than a graph-only proof, add targeted factorisation families for the missing dimensions instead of general bounded `4x4` factorisation
- avoid naive full `4x4` factorisation: enumerating all `4x4` factor candidates with entries `0..=5` is about `6^16`, which is not a viable BFS move family

## Graph-only waypoint search

Follow-up:

- added [`src/bin/find_lind_marcus_graph_waypoints.rs`](../src/bin/find_lind_marcus_graph_waypoints.rs), a bounded bidirectional graph-only waypoint search
- the move set is only one-step out-splits, one-step in-splits, out-amalgamations, and in-amalgamations
- the first version targeted only the three Baker waypoint transitions that were missing from the main one-step generator, rather than widening the full `A -> B` search
- the current version searches all seven consecutive Baker waypoints and prints the reconstructed graph-only subpaths

Outcome with `max_dim = 5`, `max_states = 1_000_000`:

- `A1 -> A2` is found at graph-only depth `5`
- `A4 -> A5` is found at graph-only depth `6`
- `A5 -> A6` is found at graph-only depth `3`
- when all seven Baker waypoint transitions are required to use only graph moves, the tool prints a full path of `22` graph moves, up to permutation of vertices

This is the key result: the missing Baker elementary SSE steps are not one-step graph moves, but they are reachable by short graph-only paths if the waypoint search is allowed to pass through `5x5` matrices.

The first estimate of `18` graph moves only replaced the three missing Baker elementary steps and kept the other covered steps as one-step non-graph moves. Requiring every step to be an in/out split or amalgamation gives the printed `22`-move path:

- Baker step `1` becomes `1` graph move
- replace Baker step `2` by `5` graph moves
- Baker step `3` becomes `2` graph moves
- Baker step `4` becomes `2` graph moves
- Baker step `5` becomes `6` graph moves
- Baker step `6` becomes `3` graph moves
- Baker step `7` becomes `3` graph moves

Printed path from `cargo run --release --bin find_lind_marcus_graph_waypoints -- --max-depth 6 --max-dim 5 --max-states 1000000`:

```text
1. outsplit: 2x2 [1, 2, 3, 1] -> 3x3 [0, 0, 1, 1, 1, 2, 2, 2, 1]
2. insplit: 3x3 [0, 0, 1, 1, 1, 2, 2, 2, 1] -> 4x4 [0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 2, 1, 1, 2, 1]
3. insplit: 4x4 [0, 0, 0, 1, 0, 0, 0, 1, 0, 1, 1, 2, 1, 1, 2, 1] -> 5x5 [0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0]
4. out_amalgamation: 5x5 [0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 1, 1, 1, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0] -> 4x4 [0, 0, 0, 1, 0, 1, 1, 1, 1, 2, 1, 1, 1, 2, 1, 0]
5. insplit: 4x4 [0, 0, 0, 1, 0, 1, 1, 1, 1, 2, 1, 1, 1, 2, 1, 0] -> 5x5 [0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 0]
6. out_amalgamation: 5x5 [0, 0, 0, 0, 1, 0, 1, 0, 1, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 1, 1, 2, 1, 0, 0] -> 4x4 [0, 0, 1, 2, 1, 0, 1, 2, 2, 0, 1, 2, 1, 1, 0, 1]
7. insplit: 4x4 [0, 0, 1, 2, 1, 0, 1, 2, 2, 0, 1, 2, 1, 1, 0, 1] -> 5x5 [0, 0, 0, 1, 2, 1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 2, 0, 2, 1, 0, 1, 1, 1, 0, 0]
8. out_amalgamation: 5x5 [0, 0, 0, 1, 2, 1, 0, 1, 1, 1, 1, 1, 1, 0, 0, 2, 0, 2, 1, 0, 1, 1, 1, 0, 0] -> 4x4 [0, 0, 1, 1, 0, 1, 0, 2, 1, 1, 0, 1, 2, 1, 1, 1]
9. outsplit: 4x4 [0, 0, 1, 1, 0, 1, 0, 2, 1, 1, 0, 1, 2, 1, 1, 1] -> 5x5 [0, 0, 0, 1, 1, 1, 1, 1, 0, 1, 2, 2, 1, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0]
10. in_amalgamation: 5x5 [0, 0, 0, 1, 1, 1, 1, 1, 0, 1, 2, 2, 1, 0, 0, 1, 1, 1, 0, 1, 1, 1, 0, 1, 0] -> 4x4 [0, 0, 1, 1, 2, 1, 0, 2, 1, 0, 0, 2, 1, 1, 1, 1]
11. insplit: 4x4 [0, 0, 1, 1, 2, 1, 0, 2, 1, 0, 0, 2, 1, 1, 1, 1] -> 5x5 [0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 2, 2, 1, 0, 0, 1, 0, 1, 1, 1, 1, 2, 0, 0, 0]
12. out_amalgamation: 5x5 [0, 0, 0, 1, 1, 1, 0, 1, 1, 1, 2, 2, 1, 0, 0, 1, 0, 1, 1, 1, 1, 2, 0, 0, 0] -> 4x4 [0, 0, 0, 1, 1, 0, 1, 1, 2, 2, 1, 0, 2, 2, 1, 1]
13. insplit: 4x4 [0, 0, 0, 1, 1, 0, 1, 1, 2, 2, 1, 0, 2, 2, 1, 1] -> 5x5 [0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 2, 1, 1, 1, 0, 2, 1, 1, 1, 1]
14. out_amalgamation: 5x5 [0, 0, 0, 0, 1, 1, 0, 0, 1, 1, 1, 0, 0, 1, 1, 2, 1, 1, 1, 0, 2, 1, 1, 1, 1] -> 4x4 [0, 0, 0, 1, 2, 0, 2, 2, 2, 1, 1, 0, 2, 1, 1, 1]
15. insplit: 4x4 [0, 0, 0, 1, 2, 0, 2, 2, 2, 1, 1, 0, 2, 1, 1, 1] -> 5x5 [0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 2, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1]
16. out_amalgamation: 5x5 [0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 2, 1, 1, 1, 1, 0, 1, 1, 1, 1, 1] -> 4x4 [0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 3, 1, 1, 1, 1]
17. outsplit: 4x4 [0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 3, 1, 1, 1, 1] -> 5x5 [0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 2, 2, 0, 3, 3, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0]
18. in_amalgamation: 5x5 [0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 2, 2, 0, 3, 3, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0] -> 4x4 [0, 0, 1, 1, 2, 0, 3, 5, 0, 0, 1, 1, 1, 1, 0, 1]
19. in_amalgamation: 4x4 [0, 0, 1, 1, 2, 0, 3, 5, 0, 0, 1, 1, 1, 1, 0, 1] -> 3x3 [0, 5, 5, 0, 1, 1, 1, 1, 1]
20. out_amalgamation: 3x3 [0, 5, 5, 0, 1, 1, 1, 1, 1] -> 2x2 [0, 5, 1, 2]
21. insplit: 2x2 [0, 5, 1, 2] -> 3x3 [0, 0, 5, 1, 1, 1, 1, 1, 1]
22. out_amalgamation: 3x3 [0, 0, 5, 1, 1, 1, 1, 1, 1] -> 2x2 [1, 1, 6, 1]
```

This does not mean full graph-only BFS with `max_dim = 5`, `max_lag = 22` is immediately practical. The waypoint searches are much smaller because their targets are fixed. The practical next step is to expose waypoint/proposal guidance or reconstruct the graph-only subpaths inside the main solver, not to blindly widen the global frontier.

## Blind graph-only endpoint search

Follow-up:

- added [`src/bin/find_brix_ruiz_graph_path.rs`](../src/bin/find_brix_ruiz_graph_path.rs), a blind bidirectional graph-only endpoint search from Brix-Ruiz `k = 3` `A` to `B`
- extracted the canonical graph successor move set into [`src/graph_moves.rs`](../src/graph_moves.rs), so the blind endpoint search and Lind-Marcus waypoint search use the same graph moves
- the blind search uses all one-step out-splits, one-step in-splits, out-amalgamations, and in-amalgamations, including from `2x2` endpoints
- added runtime guardrails: `--max-states`, `--max-candidates`, and `--max-seconds`; external shell timeouts are still useful because a single large successor enumeration cannot be interrupted until it returns

Probe:

```text
timeout 35s target/release/find_brix_ruiz_graph_path \
  --max-depth 22 --max-dim 5 --max-entry 6 \
  --max-states 100000 --max-candidates 2000000 --max-seconds 30
```

Outcome:

- no endpoint meet before the candidate cap
- candidate cap hit at `2,012,826` generated candidates
- visited states: `11,950`
- elapsed time: `19.893s`
- the cap hit during backward depth `2`; forward depth `2` alone generated `1,852,176` candidates and discovered `10,489` new canonical states

Interpretation:

- this confirms that the waypoint result is not representative of blind endpoint BFS cost
- `max_dim = 5` graph-only search is viable as a capped diagnostic, but a full blind `max_depth = 22` run is not the next sensible step without stronger proposal guidance or additional pruning
