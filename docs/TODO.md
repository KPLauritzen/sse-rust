# Search Improvements

The BFS search currently returns `Unknown` for known-SSE pairs like Brix-Ruiz k=3 (`[[1,3],[2,1]]` ↔ `[[1,6],[1,1]]`) because the search space explodes at practical bounds. Items are ranked by expected benchmark impact (high → low).

## Status note on aligned work

Aligned shift equivalence is currently blocked on specification, not on engineering.

What is implemented locally today is the graph/module-aligned condition from Definition 5.2 of Brix, Dor-On, Hazrat & Ruiz (2025); see [docs/aligned-shift-equivalence.md](aligned-shift-equivalence.md). What is still missing is the exact matrix-level relation that is claimed to be equivalent to SSE.

The local reference [references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf](../references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf) says in Remark 5.3 that a forthcoming work by Bilich, Dor-On and Ruiz defines aligned shift equivalence for finite essential matrices and proves it is equivalent to strong shift equivalence. Until that source is available, implementing a matrix-level aligned solver here would be guesswork.

So for now:

- keep the current aligned-module code separate from `search_sse`
- treat item 2 below as blocked on obtaining the primary matrix-level definition
- prefer improvements to the existing BFS/search stack for immediate solver gains

## 1. Implemented: Rayon parallelism on frontier expansion

Factorisation enumeration for each frontier node is now parallelised on native targets with `rayon::par_iter()`, using a collect-then-merge pass for collision detection and parent-map updates. The `wasm32` target keeps a serial fallback so the browser build does not depend on threading support.

## 2. Aligned shift equivalence

Carlsen, Dor-On & Eilers (2024) showed that fixed-lag aligned SE algorithms outperform naive factorisation search. This reformulates the problem algebraically and could complement or replace brute-force BFS for certain cases. Potentially the biggest win for hard cases like Brix-Ruiz k=3, but significant implementation effort.

Status: blocked pending the exact matrix-level definition of aligned shift equivalence. The repo already implements the module-level aligned witness search from Brix, Dor-On, Hazrat & Ruiz (2025), but that paper explicitly does not establish that aligned module shift equivalence implies SSE. The missing input is the forthcoming Bilich-Dor-On-Ruiz matrix-level formulation cited in Remark 5.3 of the 2025 paper.

Unblock condition: obtain the primary source defining matrix-level aligned shift equivalence, then implement the matrix-level witness, validator, and fixed-lag bounded solver in `src/aligned.rs` before integrating it into `search_sse`.

## 3. Best-first / A* search

Replace pure BFS with a priority queue. Rank frontier nodes by heuristic distance to the target — e.g. Frobenius norm difference `||M - B||_F`, or spectral distance (trace/det difference). Nodes "closer" to the target get expanded first. Turns blind BFS into directed search without changing correctness.

## 4. Iterative deepening on max_entry and max_intermediate_dim

Instead of committing to a single large `max_entry`, search with `(max_entry=2, max_dim=2)`, then `(3, 2)`, then `(3, 3)`, etc. Each round is much cheaper than one large bound, and most SSE paths use small entries. Visited sets from earlier rounds can be reused.

## 5. Constraint propagation in factorisation

When enumerating U for A = UV, once the first row of U is fixed, the space of valid V columns is tightly constrained. Propagating these constraints column-by-column (rather than enumerating all of U then solving for V) could prune the inner loops earlier. Turns factorisation enumeration into a constraint satisfaction problem.

## 6. Factorisation memoisation

The same intermediate matrix can appear in many BFS branches. Caching `(matrix, max_entry) → Vec<(U, V)>` avoids re-enumerating factorisations for repeated nodes. Memory cost is bounded by the visited set size (already stored). Helps most when the BFS frontier has many collisions.

## 7. Higher trace-sequence filtering on intermediates

Currently spectral pruning checks trace and determinant. Also checking `tr(M²)` (one matrix multiply, gives `Σλᵢ²`) provides a tighter filter. Cheap per-node cost, incremental improvement over existing spectral pruning.

## 8. Symmetry exploitation

If A and B are both symmetric, or if A = PBP⁻¹ for a known permutation P, the search can be restricted to paths respecting that symmetry. Reduces branching factor for the subset of inputs with exploitable structure.

## 9. Faster canonical form for ≥3×3

`DynMatrix::canonical_perm` tries all n! permutations. For n=3 (6 permutations) this is fine, but it's in the hot loop. A sorting-based heuristic (sort rows by row-sum, break ties lexicographically) could serve as a fast pre-hash, with full canonicalisation only on collisions. Minor improvement since n=3 is the practical max today.
