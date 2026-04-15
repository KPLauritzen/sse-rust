# Concrete Next Step For Search Parallelism (`ejw`)

Date: 2026-04-15

This note turns the evidence in
[`search-parallelism-5b8.md`](search-parallelism-5b8.md) and
[`search-parallelism-8h4.md`](search-parallelism-8h4.md) into a concrete first
implementation target for bead `sse-rust-ejw`.

## Decision

The first keepable implementation target should be:

1. make layer winner selection explicit and testable in `src/search.rs`;
2. keep the current one-direction-at-a-time merge/commit model unchanged;
3. treat budgeted staging as the immediate follow-on once the winner contract is
   explicit;
4. leave deterministic parallel dedup and dual-direction execution for later.

In practice, that means introducing an explicit per-layer winner key for
`FrontierExpansion` and defining dedup/commit semantics in terms of that key,
instead of in terms of incidental vector order.

## Why This Comes First

### Option A: explicit stable winner keys

This is the smallest change that removes the main correctness ambiguity called
out in `8h4`.

- `expand_frontier_layer_dyn` and `expand_frontier_layer` already impose an
  implicit total order: frontier order first, then per-parent successor
  enumeration order.
- `deduplicate_expansions` currently preserves whichever candidate appears first
  in that order.
- the merge loop in both bidirectional searches then applies another
  first-writer-wins rule when it commits `parent`, `depths`, `orig`, and exact
  meets.

Making that order explicit does not solve memory pressure by itself, but it
gives future staging and parallel dedup work a concrete contract to preserve.

### Option B: lower-memory or budgeted staging

This should come next, not first.

- `5b8` and `8h4` already showed that whole-layer `Vec<FrontierExpansion>`
  staging is the main structural memory hazard.
- but a chunked or streamed staging design still needs to know which candidate
  wins when multiple chunks produce the same `next_canon` or
  same-future/past representative.
- without an explicit winner key, a memory reduction change risks changing
  witnesses and frontier shape at the same time.

So staging should be the next implementation target after the winner contract
lands, not before.

### Option C: deterministic parallel dedup

This remains the most plausible later performance target, but it is not the
first target.

- `5b8` showed dedup is the largest remaining serial phase.
- `8h4` showed that sharded dedup without an explicit winner rule would change
  `parent` claims, `orig` representatives, and same-future/past frontier shape.

Parallel dedup only becomes safe once every shard can reduce onto the same
explicit winner key and the merged winners can be replayed in that order.

### Option D: concurrent forward/backward expansion

This is still the wrong next step.

- it duplicates staging memory on both sides of the search;
- it needs round snapshots and deterministic same-round meet resolution;
- and the measured payoff is less justified than fixing the dedup path first.

Nothing in the current evidence is strong enough to move this ahead of winner
selection plus staging control.

## First-Step Target In Code

The first implementation target touches only the layer-expansion and
deduplication seams.

### Target seams

- `expand_frontier_layer_dyn`
- `expand_frontier_layer`
- `deduplicate_expansions`
- existing dedup-focused tests in `src/search.rs`

The bidirectional merge loops and witness reconstruction should stay behaviorally
unchanged for this step. They are downstream consumers of the winner contract,
not the place to redesign it yet.

### Proposed data-shape change

Add an explicit layer-local ordering key to `FrontierExpansion`:

```rust
struct LayerExpansionOrderKey {
    frontier_index: usize,
    successor_index: usize,
}

struct FrontierExpansion {
    order_key: LayerExpansionOrderKey,
    // existing fields...
}
```

Meaning:

- `frontier_index`: the index of the parent node inside the drained
  `current_frontier` for that layer;
- `successor_index`: the index of the accepted successor within that parent's
  deterministic successor enumeration order after per-parent duplicate pruning.

Lower `order_key` wins.

That preserves the current semantics exactly:

- smaller `frontier_index` beats larger `frontier_index`;
- for the same parent, earlier successor enumeration beats later enumeration;
- the surviving deduped layer can still be committed serially in winner order.

### Immediate contract change

`deduplicate_expansions` should be defined in terms of `order_key`, not in
terms of input vector order.

That means:

- canonical duplicates keep the smallest `order_key`;
- same-future/past representative pruning also keeps the smallest `order_key`;
- the returned deduped layer stays ordered by ascending `order_key`.

This is enough to make the current serial implementation and a future sharded
implementation aim at the same winners.

## What This Enables Next

Once `order_key` is explicit, the next keepable design should be budgeted
staging rather than immediate parallel dedup.

The follow-on target should look like this:

1. expand a bounded chunk of frontier parents;
2. produce `FrontierExpansion` candidates with explicit `order_key`;
3. reduce that chunk into chunk-local winners;
4. merge chunk winners into a layer-global winner set by `order_key`;
5. commit winners in ascending `order_key`;
6. stop or spill when a staging budget is hit.

That keeps memory proportional to the budgeted winner set instead of to two
whole-layer `Vec<FrontierExpansion>` allocations, while still preserving the
same winner semantics.

Only after that should `ejw` try to parallelize intra-layer dedup itself.

## Invariants To Preserve

### Dedup invariants

- Equal `next_canon` candidates must keep the smallest `order_key`.
- Same-future/past representative pruning must keep the smallest `order_key`.
- Dedup output order must be ascending `order_key`, regardless of the order
  candidates are presented to the reducer.

### Merge/commit invariants

- The merge loop must continue to commit discoveries in ascending `order_key`.
- `parent`, `depths`, and `orig` remain first-writer-wins with respect to that
  explicit order.
- if a future refactor batches exact meets, the tie-break must be
  `min(path_depth, order_key)` rather than first scheduler-visible meet.

### Witness invariants

- The stored `orig` representative for a canonical node must come from the
  winning `order_key`.
- rerunning the same search on the same binary must still produce the same
  witness path and the same meeting representative pair.
- permutation bridges during reconstruction remain valid, but they must reflect
  the stable winners chosen upstream.

## Required Regression Tests

The first-step implementation should keep the existing behavior while tightening
the contract with focused tests:

1. canonical dedup keeps the lowest `order_key` even if candidates arrive in a
   different vector order;
2. same-future/past representative pruning keeps the lowest `order_key` even if
   candidates arrive in a different vector order;
3. duplicate frontier parents still collapse onto the earliest parent index,
   showing that cross-parent winner selection remains stable.

Those tests are sufficient for the first step. Broader staging-budget tests
should come with the staging redesign, not with this note.
