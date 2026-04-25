# Endpoint layer winner contract closeout (2026-04-25)

Bead: `sse-rust-132`

Conclusion:

- the endpoint frontier layer winner contract was already implemented in code;
- no search-behavior change was required to satisfy the bead;
- the only code change in this closeout was tighter acceptance-style tests.

Current contract in `src/search/frontier.rs`:

- `FrontierExpansion` carries an explicit `LayerExpansionOrderKey` with
  `(frontier_index, successor_index)`;
- `expand_frontier_node` assigns that key from frontier position and accepted
  per-parent successor order;
- `deduplicate_expansions` sorts by `order_key` if candidates arrive out of
  order, then applies canonical dedup and same-future/past representative
  selection in ascending key order.

Implication for later staging or parallelism work:

- preserve winner selection as "lowest explicit `order_key` wins";
- preserve dedup output order as ascending `order_key`;
- do not let chunk order, shard order, or scheduler timing define
  representative choice.

Acceptance evidence added here:

- canonical dedup keeps the `order_key = (0, 0)` winner even when candidate
  input is reversed;
- same-future/past representative pruning keeps the `order_key = (0, 0)`
  winner and returns deduped output in ascending `order_key` order even when
  candidate input is reversed.
