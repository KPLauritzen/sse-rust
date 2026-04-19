# Endpoint Multi-Meet CLI Surface (2026-04-19)

## What changed

Endpoint search now has one explicit bounded opt-in surface for retaining more
than one exact meet:

- CLI flag: `--endpoint-multi-meet-cap N`
- output surface: top-level JSON field `endpoint_exact_meets`
- current availability: `--stage endpoint-search` with `--frontier-mode bfs`

Without that flag, endpoint search keeps the existing behavior and returns on
the first admissible exact meet exactly as before.

## What “multiple exact meets” means on this surface

The retained surface is **not** an unbounded witness enumerator.

It means:

- the search reached at least one admissible exact cross-frontier meet in a BFS
  merge layer;
- with the flag enabled, the search finishes that current merge layer instead
  of returning immediately on the first admissible meet;
- it retains at most `N` admissible exact meets from that layer;
- each retained item includes:
  - the canonical meeting state;
  - the meet lag used for ranking (`next_depth + other_depth`);
  - a reconstructed reusable full witness path from the original source to the
    original target.

This is a **bounded retained layer surface**, not a global enumeration over all
later layers.

## Ranking and cap semantics

Retained exact meets are sorted and truncated by:

1. smaller meet lag first;
2. then earlier discovery order in the current serial merge loop.

The CLI primary result is the rank-1 retained witness when the surface is
enabled. With no opt-in flag, the old immediate-return path remains unchanged.

Cap semantics:

- `N` is a hard cap on retained exact meets for that merge layer;
- the retained JSON array is already sorted in ranking order;
- if more than `N` admissible meets are observed, later lower-ranked meets are
  dropped.

## Caveat: meet lag vs reconstructed path length

The retained `path_lag` is the exact meet depth used for ranking. The rebuilt
path can be longer than `path_lag` because path reconstruction may need
permutation bridges around the stored canonical representatives.

So the surface should be read as:

- `path_lag`: ranking depth of the exact meet;
- `path.steps.len()`: reusable witness length after representative replay.

Do not assume those two numbers are equal.

## Focused validation case

One small bounded case that already exhibits multiple exact meets is:

- source: `[[0,0],[0,1]]`
- target: `[[0,1],[0,1]]`
- config: endpoint search, BFS, `--endpoint-multi-meet-cap 4`

That case retained 4 exact meets on the new surface.
