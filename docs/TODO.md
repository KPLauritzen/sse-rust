# Search Improvements

The BFS search currently returns `Unknown` for known-SSE pairs like Brix-Ruiz k=3 (`[[1,3],[2,1]]` ↔ `[[1,6],[1,1]]`) because the search space explodes at practical bounds. These are approaches to improve search without parallelism.

## Iterative deepening on max_entry

Instead of committing to a single large `max_entry`, search with max_entry=2, then 3, then 4, etc. Each round is much cheaper than one large bound, and most SSE paths use small entries.

## Spectral pruning of intermediates

Nonzero eigenvalues are preserved by SSE. Skip intermediate matrices whose trace is inconsistent with the target's spectral radius. Cheap check that could trim the BFS frontier.

## Aligned shift equivalence

Carlsen, Dor-On & Eilers (2024) showed that fixed-lag aligned SE algorithms outperform naive factorisation search. This reformulates the problem algebraically and could complement or replace brute-force BFS for certain cases.
