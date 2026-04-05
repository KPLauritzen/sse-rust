# Search Improvements

The BFS search currently returns `Unknown` for known-SSE pairs like Brix-Ruiz k=3 (`[[1,3],[2,1]]` ↔ `[[1,6],[1,1]]`) because the search space explodes at practical bounds. Items are ranked by expected benchmark impact (high → low).

## 1. Rayon parallelism on frontier expansion

Factorisation enumeration for each frontier node is independent. Wrapping the frontier iteration in `rayon::par_iter()` gives near-linear speedup on multi-core machines. The merge step (collision detection) needs a concurrent map or a collect-then-merge pattern. Easy to implement, multiplicative speedup on all benchmarks.

## 2. Aligned shift equivalence

Carlsen, Dor-On & Eilers (2024) showed that fixed-lag aligned SE algorithms outperform naive factorisation search. This reformulates the problem algebraically and could complement or replace brute-force BFS for certain cases. Potentially the biggest win for hard cases like Brix-Ruiz k=3, but significant implementation effort.

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
