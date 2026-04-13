# Beam Executor Retune on Bounded K=3

Retune target: keep the current beam scoring surface fixed and change only the
beam executor policy in `src/search.rs`.

## Change

- rank beam frontier entries with `path_scoring::score_node(node, target)`,
  keeping approximate-hit promotion as a separate priority;
- expand a small same-depth batch per beam step instead of a single best node,
  so width-capped frontiers retain some breadth before the next truncation.

## Validation

Exact bounded probe from `research/notes/2026-04-13-beam-k3-validation.md`:

```bash
timeout 60s target/debug/search 1,3,2,1 1,6,1,1 \
  --max-lag 7 \
  --max-intermediate-dim 4 \
  --max-entry 10 \
  --search-mode beam \
  --beam-width WIDTH \
  --json --telemetry
```

Results after the executor retune:

- beam width `50`: `unknown` in `1.72s`
  - `frontier_nodes_expanded = 684`
  - `candidates_generated = 169294`
  - `max_frontier_size = 50`
  - `total_visited_nodes = 8611`
  - `approximate_other_side_hits = 61`
  - `collisions_with_other_frontier = 0`
- beam width `200`: `unknown` in `9.30s`
  - `frontier_nodes_expanded = 3181`
  - `candidates_generated = 402692`
  - `max_frontier_size = 200`
  - `total_visited_nodes = 30147`
  - `approximate_other_side_hits = 278`
  - `collisions_with_other_frontier = 0`

## Interpretation

- Width `50` is still unsolved, but it is materially cheaper than the earlier
  `11.53s` / `25835`-node run on the same cap.
- Width `200` no longer times out under the 60-second bound; it now returns
  `unknown` in single-digit seconds.
- The executor retune therefore improved bounded beam viability on the known
  `k=3` control, but it still does not produce an exact meet under `max_lag 7`.
