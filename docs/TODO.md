# Search Improvements

The BFS search currently returns `Unknown` for known-SSE pairs like Brix-Ruiz k=3 (`[[1,3],[2,1]]` ↔ `[[1,6],[1,1]]`) because the search space explodes at practical bounds. These are approaches to improve search without parallelism.

## Bidirectional BFS

Search from both A and B simultaneously to depth L/2 and check for a meeting point in the canonical-form hash map. Turns O(branching^L) into O(2 · branching^(L/2)) — an exponential reduction. The existing canonical forms make the meet-in-the-middle check a cheap hash lookup.

## Iterative deepening on max_entry

Instead of committing to a single large `max_entry`, search with max_entry=2, then 3, then 4, etc. Each round is much cheaper than one large bound, and most SSE paths use small entries.

## Smarter factorisation pruning

The rectangular (2×3) factorisation enumerator is 6 nested loops — O(max_entry^6). Possible improvements:

- **Determinant divisibility**: for square factorisations, det(U) must divide det(A), which eliminates most U candidates.
- **Row-sum bounds**: row sums of U are bounded by the max row sum of A (since V has nonneg entries).
- **Early column feasibility**: after choosing the first row of U in the rectangular case, check column solvability before iterating the second row.

## Spectral pruning of intermediates

Nonzero eigenvalues are preserved by SSE. Skip intermediate matrices whose trace is inconsistent with the target's spectral radius. Cheap check that could trim the BFS frontier.

## Aligned shift equivalence

Carlsen, Dor-On & Eilers (2024) showed that fixed-lag aligned SE algorithms outperform naive factorisation search. This reformulates the problem algebraically and could complement or replace brute-force BFS for certain cases.
