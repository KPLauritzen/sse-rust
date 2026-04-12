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

Printed path from `cargo run --release --features research-tools --bin find_lind_marcus_graph_waypoints -- --max-depth 6 --max-dim 5 --max-states 1000000`:

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

Optimization follow-up:

- the first endpoint search enumerated every child-label assignment for one-step splits, even though the sidecar canonicalizes states up to vertex permutation
- [`src/graph_moves.rs`](../src/graph_moves.rs) now has a representative-only graph successor path for canonical graph search: for a one-step split, it chooses one child-label assignment for each split parent and orders the two split child rows
- the full witness enumerator remains available for callers that need explicit division/edge matrices

Same bounded probe after representative split generation:

- forward depth `2` dropped from `1,852,176` raw split candidates to `16,054` representative candidates while discovering the same `10,489` new canonical states
- a `60s` run with `--max-candidates 1000000` reached forward depth `4`, visited `47,636` states, and generated `131,503` representative candidates before the time cap
- no endpoint meet yet under that cap

Interpretation:

- the largest avoidable duplication was child-label redundancy inside split witnesses, and eliminating it is a real speedup
- the remaining cost now looks more like canonicalization and large-frontier bookkeeping than raw child-label witness explosion

Second optimization follow-up:

- [`src/bin/find_brix_ruiz_graph_path.rs`](../src/bin/find_brix_ruiz_graph_path.rs) now computes missing layer successors in parallel with Rayon, stores successor lists in an in-memory cache, and uses recent candidate-per-node cost when choosing between equal-depth forward/backward layers
- the in-memory successor cache showed `0` hits on a single fresh blind BFS run, because each side expands newly discovered canonical states and an overlap would terminate the search; this cache is still useful groundwork for iterative schedules or future persistent successor caches

Probe with `--max-seconds 45`, `--max-candidates 1000000`:

- no endpoint meet before the time cap
- reached forward depth `4`
- visited states: `43,225`
- generated representative candidates: `125,397`
- elapsed time: `45.195s`

Interpretation:

- parallel expansion lets the capped run process further into the same blind search, but the improvement is not as dramatic as the representative split generation
- the next likely bottleneck is still canonicalization and large-frontier bookkeeping; caching becomes more interesting if we persist successors across repeated runs rather than only within one fresh run

Third optimization follow-up:

- first attempt was misdirected: specialized 4x4 and 5x5 `canonical_perm` fast paths in [`src/matrix.rs`](../src/matrix.rs) (stack arrays for 4x4, and partition-by-`(diag, row_sum, col_sum)` refinement for 5x5 so only invariant-preserving permutations are tried) moved the 60-second run from `60.476s` to `60.094s` — basically no change, meaning canonicalization was not the real bottleneck
- adding per-phase timing to `expand_layer` (`prep`, `cache_check`, `compute`, `seq`) made the real hot spot obvious: at backward depth `2`, `total = 24.423s` of which `compute = 0.106s` and `seq = 24.317s`. Forward depth `2` over comparable work spent only `0.340s` in `seq`. Per discovered state, backward `seq` was about `992μs` versus forward `32μs` — a ~`31x` asymmetry that pointed at a structure-dependent cost inside the successor-insertion loop, not at successor enumeration itself
- root cause: `visited_union_size(seen, other_seen)` iterated one of the two hash maps and probed the other for each key — `O(min(|seen|, |other_seen|))` per call — and it was called inside the inner loop for every newly discovered successor (to evaluate the `max_states` cap, as well as at every summary and early return). At backward depth `2` that was roughly `24,521` new states × `10,824` forward-seen entries ≈ `265M` hash probes per layer, which matches the observed `~24s`. The asymmetry came from `seen` being the smaller map on the backward side, so the cheaper `.len()` branch was rarely the one chosen
- fix: [`src/bin/find_brix_ruiz_graph_path.rs`](../src/bin/find_brix_ruiz_graph_path.rs) now maintains an incremental `total_visited` counter updated in `O(1)` per insertion. The `other_seen.get(&successor.matrix)` lookup already computed for meeting-point detection is reused to decide whether the new state is a fresh contribution to the union, so there is no extra hashing
- result on the hot layer: backward depth `2` `seq` phase went from `24.317s` to `0.033s` (~`737x`), and the blind BFS that previously hit the time cap at forward depth `4` with `~46k` visited states now reaches `500k` states in about `1.7s` and finds a graph-only endpoint path for `brix_ruiz_k3` in about `5s`
- the 5x5 canonical fast path and [`src/bin/profile_graph_moves.rs`](../src/bin/profile_graph_moves.rs) were kept; they become relevant once the quadratic visited-union cost is out of the way
- generalisable lesson: "hot enough to matter" in bookkeeping is dominated by `O(|seen|)`-per-state operations, not by per-successor work; before optimising a canonical form, check whether any frontier bookkeeping inside the inner loop is non-constant in the size of the already-visited set

Endpoint-only result:

```text
target/release/find_brix_ruiz_graph_path \
  --max-depth 22 --max-dim 5 --max-entry 6 \
  --max-states 5000000 --max-candidates 50000000 --max-seconds 60
```

- found a graph-only path at depth `16`
- no Lind-Marcus/Baker waypoints are encoded in this endpoint search; the only fixed endpoints are the Brix-Ruiz `k = 3` matrices
- meeting state: `4x4 [0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 2, 1, 2, 1, 1]`
- visited states: `1,404,317`
- candidates generated: `3,200,150`
- elapsed time: `4.794s`

Printed path:

```text
1. outsplit: 2x2 [1, 2, 3, 1] -> 3x3 [0, 0, 2, 1, 1, 1, 2, 2, 1]
2. insplit: 3x3 [0, 0, 2, 1, 1, 1, 2, 2, 1] -> 4x4 [0, 0, 1, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 2, 0, 1]
3. outsplit: 4x4 [0, 0, 1, 1, 1, 1, 0, 1, 2, 2, 0, 1, 2, 2, 0, 1] -> 5x5 [0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 2, 2, 1]
4. in_amalgamation: 5x5 [0, 0, 1, 1, 0, 0, 0, 1, 1, 1, 1, 1, 0, 0, 1, 0, 0, 1, 1, 1, 0, 0, 2, 2, 1] -> 4x4 [0, 0, 1, 1, 0, 1, 2, 2, 0, 1, 1, 1, 1, 1, 1, 0]
5. outsplit: 4x4 [0, 0, 1, 1, 0, 1, 2, 2, 0, 1, 1, 1, 1, 1, 1, 0] -> 5x5 [0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 0, 2, 0, 2, 2, 1]
6. in_amalgamation: 5x5 [0, 0, 0, 0, 1, 1, 0, 1, 1, 0, 1, 1, 0, 1, 1, 1, 0, 1, 1, 0, 2, 0, 2, 2, 1] -> 4x4 [0, 0, 0, 1, 1, 0, 2, 1, 1, 1, 1, 0, 2, 2, 2, 1]
7. outsplit: 4x4 [0, 0, 0, 1, 1, 0, 2, 1, 1, 1, 1, 0, 2, 2, 2, 1] -> 5x5 [0, 1, 0, 0, 1, 1, 0, 2, 0, 0, 1, 1, 0, 2, 1, 1, 0, 1, 1, 0, 1, 1, 0, 2, 1]
8. in_amalgamation: 5x5 [0, 1, 0, 0, 1, 1, 0, 2, 0, 0, 1, 1, 0, 2, 1, 1, 0, 1, 1, 0, 1, 1, 0, 2, 1] -> 4x4 [0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 2, 1, 2, 1, 1]
9. insplit: 4x4 [0, 0, 1, 1, 1, 1, 0, 1, 1, 0, 0, 2, 1, 2, 1, 1] -> 5x5 [0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1, 0, 2, 1, 1, 0, 0, 1, 1, 1, 1, 0, 2, 1]
10. out_amalgamation: 5x5 [0, 1, 1, 0, 0, 1, 0, 1, 0, 1, 1, 1, 0, 2, 1, 1, 0, 0, 1, 1, 1, 1, 0, 2, 1] -> 4x4 [0, 0, 1, 1, 0, 1, 0, 1, 1, 2, 0, 1, 2, 2, 1, 1]
11. insplit: 4x4 [0, 0, 1, 1, 0, 1, 0, 1, 1, 2, 0, 1, 2, 2, 1, 1] -> 5x5 [0, 1, 0, 0, 1, 1, 0, 1, 2, 0, 2, 1, 0, 2, 1, 0, 0, 1, 1, 0, 2, 1, 0, 2, 1]
12. out_amalgamation: 5x5 [0, 1, 0, 0, 1, 1, 0, 1, 2, 0, 2, 1, 0, 2, 1, 0, 0, 1, 1, 0, 2, 1, 0, 2, 1] -> 4x4 [0, 0, 0, 1, 0, 1, 1, 0, 2, 2, 0, 1, 3, 4, 1, 1]
13. outsplit: 4x4 [0, 0, 0, 1, 0, 1, 1, 0, 2, 2, 0, 1, 3, 4, 1, 1] -> 5x5 [0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 0, 1, 0, 1, 1, 0, 1, 3, 1, 4, 1]
14. in_amalgamation: 5x5 [0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 2, 0, 2, 0, 1, 0, 1, 1, 0, 1, 3, 1, 4, 1] -> 4x4 [0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 0, 4, 4, 1, 1]
15. out_amalgamation: 4x4 [0, 0, 0, 1, 1, 1, 1, 0, 2, 2, 0, 0, 4, 4, 1, 1] -> 3x3 [0, 0, 2, 1, 1, 4, 1, 1, 1]
16. out_amalgamation: 3x3 [0, 0, 2, 1, 1, 4, 1, 1, 1] -> 2x2 [1, 1, 6, 1]
```

Interpretation:

- this changes the status of the graph-move sidecar: a blind endpoint search can now find a graph-only proof for `brix_ruiz_k3` within the same `max_dim = 5` universe as the waypoint-expanded Baker witness
- the endpoint path is shorter than the waypoint-expanded Baker graph path (`16` graph moves instead of `22`)
- the search still uses bounds informed by the prior Lind-Marcus/Baker investigation, especially `max_dim = 5` and depth `22`; the important difference is that it no longer uses the displayed literature waypoints as targets

## Blind endpoint search with `max_dim = 6`

Follow-up:

- tried widening the blind endpoint search from `max_dim = 5` to `max_dim = 6`
- added a 6x6 dynamic canonical representative fast path in [`src/matrix.rs`](../src/matrix.rs), using the same invariant-partition strategy as the 5x5 path

Probes:

```text
target/release/find_brix_ruiz_graph_path \
  --max-depth 16 --max-dim 6 --max-entry 6 \
  --max-states 2000000 --max-candidates 10000000 --max-seconds 60
```

- before the 6x6 canonical fast path: state cap at `2,000,001` visited states, `3,512,611` candidates, `33.699s`
- after the 6x6 canonical fast path: same state/candidate counts, but elapsed time dropped to `7.772s`

Larger capped probe:

```text
target/release/find_brix_ruiz_graph_path \
  --max-depth 16 --max-dim 6 --max-entry 6 \
  --max-states 8000000 --max-candidates 30000000 --max-seconds 100
```

Outcome:

- no endpoint meet before the state cap
- visited states: `8,000,001`
- candidates generated: `16,940,740`
- elapsed time: `34.480s`

Interpretation:

- allowing 6x6 states is now computationally viable per successor, but the state-space blow-up is the new bottleneck
- raising caps blindly is unlikely to be the right next move; a useful `max_dim = 6` search probably needs dimension-aware scheduling, delayed 6x6 expansion, or a search mode for paths that genuinely require dimension `6`

## Follow-up experiments

Three follow-up questions are open after the depth-`16` `brix_ruiz_k3` result:

1. **Is the blind path actually different from Baker's?** Both are graph-only and stay inside `max_dim = 5`, but the blind one is `16` moves and Baker's is `22`. They could still share intermediates.
2. **Does a larger `max_entry` shorten the `k = 3` path?** The blind run used `max_entry = 6`, the smallest that contains both endpoints. If raising it lets the search visit higher-entry waypoints, maybe a shorter path exists. Also: how high can `max_entry` go inside a `60s` budget?
3. **Does a graph-only path exist for `brix_ruiz_k4`?** SSE for `k = 4` is **not** confirmed in the literature, so a hit here would be a real new result, and absence is also informative.

Outcomes 1 and 2:

- new diagnostic [`src/bin/compare_brix_ruiz_graph_paths.rs`](../src/bin/compare_brix_ruiz_graph_paths.rs) canonicalises every intermediate matrix from both paths and reports overlap. The two paths share **only the start `[1, 2, 3, 1]` and the target `[1, 1, 6, 1]`**: all `21` Baker intermediates and all `15` blind intermediates are canonically distinct. The blind search found a genuinely independent witness, not a subpath of Baker's path
- the `max_entry` sweep on `k = 3` (`max_dim = 5`, `60s` cap, all six values rerun blind):

  | `max_entry` | path depth | visited     | elapsed |
  | ----------- | ---------- | ----------- | ------- |
  | `6`         | `16`       | `1,404,317` | `~5s`   |
  | `7`         | `16`       | `1,686,748` | `7.0s`  |
  | `8`         | `16`       | `1,884,139` | `8.2s`  |
  | `10`        | `16`       | `2,195,632` | `9.1s`  |
  | `12`        | `16`       | `2,644,022` | `11.9s` |
  | `16`        | `16`       | `2,858,621` | `12.6s` |
  | `24`        | `16`       | `3,025,339` | `13.2s` |
  | `50`        | `16`       | `3,027,021` | `13.4s` |
  | `200`       | `16`       | `3,027,021` | `13.4s` |

- the visited count saturates at `~3.03M` for `max_entry >= 24`, so the entire reachable `max_dim = 5` graph-move universe within `~7` layers of either side is bounded by entry `24` and the search has effectively explored all of it. **Depth `16` is the global minimum graph-only path length on `max_dim = 5`**, and it is unaffected by `max_entry`. The `60s` budget is far from binding (largest run finishes in `13.4s`), so `max_entry` is not the constraint to relax

Plan for outcome 3 (the `k = 4` search):

- the literal target `B_4 = [[1, 12], [1, 1]]` has entry `12`, so any straightforward search needs `max_entry >= 12`. A first probe with `--k 4 --max-dim 5 --max-entry 12 --max-states 30000000 --max-seconds 60` hit the time cap at `63s` with `13,530,828` visited states and no meeting; backward depth `6` exploded from a `42,919`-node frontier to `9,452,297` discovered states, which is the dominant cost
- two ideas to make the search actually finish within budget:
  1. **Drop the dead in-run successor cache**. Profiling on `k = 4` shows the cache is `~500–700` bytes per visited state on top of the `~540` bytes of `seen + parent`, roughly doubling per-state memory. The cache provably never hits during a fresh bidirectional run (each side dedupes on insert and a meet terminates the search), so it is pure overhead. A `--use-cache` flag is now wired in, defaulted off; this should roughly double the state cap that fits in `~31 GB` RAM
  2. **Pre-expand the target without entry pruning**. Instead of seeding the backward side with just `B_4`, run an unbounded BFS from `B_4` for a few graph-move steps, accept whatever entries appear, and use those states as the backward frontier. The main bidirectional loop then runs the rest of the search at a small `max_entry` (e.g. `6`, the same bound that worked for `k = 3`). The reconstructed path's first few backward steps will have entries `> max_entry`, but the bulk of the search lives inside the same low-entry universe that `k = 3` could traverse in `~5s`. A `--seed-depth N` flag is now wired in
- both flags are off by default and should not affect the existing `k = 3` results when unused
- intended runs:
  - sanity check: `--k 3` with default flags reproduces the `5–7s` depth-`16` result
  - cache off vs on, `--k 4 --max-dim 5 --max-entry 12`, same caps as the previous probe — measure RSS and visited-state count at the time cap
  - `--k 4 --max-dim 5 --max-entry 6 --seed-depth {1, 2, 3}`, `60s` cap each, looking for a meet
- predictions:
  - the `--use-cache off` run should let `k = 4 / max_entry = 12` reach roughly twice the visited-state count in the same time budget but probably still not meet, because the explosion at backward depth `6+` is structural, not bookkeeping
  - `--seed-depth 1` is unlikely to reach into the `max_entry = 6` universe at all because `[1, 12]` only splits cleanly (max child entry `<= 6`) into a small number of configurations, all close to `B_4`. **`seed-depth >= 2` is the more likely value to actually start producing low-entry seeds**
  - if no `seed-depth` value finds a meet within `60s` at `max_entry = 6`, that is itself evidence that the `max_dim = 5 / max_entry = 6` universe does not connect `A_4` to even a small neighborhood of `B_4` — a non-trivial structural result, even though it is still consistent with `k = 4` SSE
