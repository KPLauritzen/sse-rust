# Deeper Search Parallelism Risk Assessment (`8h4`)

Date: 2026-04-15

This note sharpens the earlier timing investigation in
[`search-parallelism-5b8.md`](search-parallelism-5b8.md) into a concrete design
gate for bead `sse-rust-8h4`: what exactly is order-sensitive in the current
search layering, where deeper parallelism would increase peak memory, what
correctness hazards concurrent bidirectional execution introduces, and what
should be built first if we still want more parallelism later.

## Existing Evidence

- [`search-parallelism-5b8.md`](search-parallelism-5b8.md) already established
  that per-layer compute dominates, whole-layer dedup is the largest secondary
  serial phase, and merge/commit is materially smaller.
- `src/search.rs` confirms that the current endpoint search is structured as:
  1. pick one direction,
  2. drain that whole frontier,
  3. materialize a full `Vec<FrontierExpansion>`,
  4. deduplicate the full layer,
  5. serially claim `parent`, `depths`, `orig`, signatures, and next-frontier
     entries,
  6. return immediately on the first admissible exact meet.
- Earlier sidecar work on the hard `k = 4` graph family already showed that
  extra per-state bookkeeping can become a first-order memory limit:
  [`brix-ruiz-sidecar-log.md`](brix-ruiz-sidecar-log.md) recorded roughly
  `~540` bytes for `seen + parent` and another `~500-700` bytes per state for a
  cache that turned out not to help fresh bidirectional runs. The exact numbers
  are from a different search surface, but the lesson transfers: duplicated
  staging has to justify its memory cost.

## Current Order-Sensitive Contracts

The relevant code paths are:

- dynamic endpoint BFS:
  `search_sse_with_telemetry_dyn_with_deadline_and_observer`
  (`src/search.rs`, around `1614-1858`)
- `2x2` endpoint BFS:
  `search_sse_with_telemetry_and_observer` (`src/search.rs`, around
  `2049-2388`)
- layer staging:
  `expand_frontier_layer_dyn` and `expand_frontier_layer`
  (`src/search.rs`, around `5030-5066` and `5183-5299`)
- layer dedup:
  `deduplicate_expansions` (`src/search.rs`, around `5069-5092`)
- witness reconstruction:
  `walk_parent_chain`, `reconstruct_bidirectional_path`, and
  `reconstruct_bidirectional_dyn_path` (`src/search.rs`, around `5522-5677`)

Those functions imply several exact determinism hazards.

### 1. Dedup currently means "first expansion in layer order wins"

`deduplicate_expansions` walks the layer linearly and keeps the first
representative for each `next_canon`. It also keeps the first
same-future/past representative when that pruning mode is enabled for graph
expansions on dimension `>= 3`.

That means current behavior depends on the implicit layer order produced by:

- frontier order in `current_frontier`,
- per-parent successor enumeration order,
- accumulation order when per-node Rayon results are flattened,
- then the final left-to-right `deduplicate_expansions` scan.

Parallel or sharded dedup without an explicit stable winner key would change:

- which parent/step claims a canonical state,
- which `orig` representative is stored for that canonical state,
- and, for same-future/past pruning, which non-identical graph representative
  survives to the next frontier at all.

The last point is stricter than a witness-format issue: the representative
choice changes the future search surface because same-future/past pruning is not
canonical-equivalence dedup.

### 2. Parent-map claiming is also first-writer-wins

After dedup, the merge loop still applies a serial claim rule:

- if `parent.contains_key(next_canon)`, the later expansion is discarded as a
  seen collision;
- otherwise the winner writes `parent`, `depths`, `orig`, and the signature
  set.

So there are two stacked order-sensitive winner rules:

- dedup winner within the current layer,
- then first committed discovery against previously seen nodes.

If parallel commit is added without preserving a total order, the stored parent
chain can change even when the accepted canonical set does not.

### 3. Witness reconstruction depends on the stored `orig` representative

The parent maps are keyed by canonical matrices, but witness reconstruction
walks `orig` chains, not just canonical keys. If forward and backward stored
different concrete representatives for the same canonical meeting node,
reconstruction inserts a permutation bridge between them.

That is already valid today, but it means winner choice affects:

- the exact matrix sequence in the returned witness,
- whether an extra permutation step is inserted at the meeting point,
- and which parent path is surfaced to downstream tooling and tests.

So "determinism" here is not only about set membership. It reaches the exact
returned witness.

### 4. Exact-meet resolution currently depends on commit order

The merge loop returns immediately on the first admissible exact meet it sees.
If multiple meets are present in the same layer, the current answer is the
first one in deduped expansion order whose `next_depth + other_depth` is within
`max_lag`.

Parallel exact-meet handling therefore needs an explicit policy, not just a
lock:

- minimum total depth first,
- then a stable tie-break among equal-depth candidates,
- and a deterministic rule for which representative pair defines the meeting
  witness.

Without that, returned witnesses become schedule-dependent.

## Memory Hazards Under The Current `FrontierExpansion` Model

The main memory issue is not abstract. It is already baked into the staging
shape.

### Current peak shape

Both `expand_frontier_layer` and `expand_frontier_layer_dyn` do all of the
following before commit:

- build per-node `Vec<FrontierExpansion>` results,
- store the whole `Vec<(Vec<FrontierExpansion>, FrontierExpansionStats)>`
  collection from Rayon,
- flatten that into one layer-sized `Vec<FrontierExpansion>`,
- build a second `Vec<FrontierExpansion>` for the deduped result,
- only then enter the commit loop.

Each `FrontierExpansion` carries cloned `parent_canon`, `next_canon`,
`next_orig`, and a full `EsseStep` (`u`, `v`), plus optional same-future/past
signature metadata. On harder layers, that is already the dominant transient
allocation structure.

The dynamic timed variant chunks compute work, but it still appends each chunk's
results into one layer-wide `expansions` buffer before dedup. Chunking helps
deadline control, not peak candidate memory.

### What deeper parallelism would worsen

Under the current representation, deeper parallelism increases peak live memory
in predictable ways:

- sharded dedup adds shard-local winner maps or buffers before the global merge;
- parallel commit usually requires retaining candidate batches until conflict
  resolution finishes;
- concurrent forward and backward expansion doubles whole-layer staging,
  dedup scratch, and next-frontier buffers for the overlapping round;
- snapshot-based dual-direction execution also needs two immutable "other side"
  views to merge against, rather than one already-committed side and one active
  side.

So a design that adds more concurrency without changing the staging model will
push the solver toward higher peak memory before it removes much serial work.
That is the wrong trade under the current evidence, especially for `k = 4+`.

## Correctness Hazards In Concurrent Bidirectional Execution

Today only one direction expands at a time, and it checks exact meets against
the other side's already committed `depths`/`orig` snapshot. That serial rule
avoids several race-shaped correctness problems.

If both directions expand concurrently, these hazards appear immediately:

### 1. Same-round meet discovery becomes ambiguous

Two sides can discover the same canonical node in the same round.

If they only check against pre-round snapshots, they can miss each other until a
later merge phase.

If they check a shared mutable `other_depths`, then the answer depends on
timing:

- which side publishes first,
- which concrete representative gets observed,
- and whether a lag-admissible meet is seen before or after another winner
  overwrites the effective candidate set.

### 2. "Seen" semantics stop being a simple hash-map lookup

The current `parent.contains_key(next_canon)` rule is only valid because one
side is the sole writer for the active layer and the other side is immutable for
that turn.

With concurrent bidirectional execution, the implementation needs a round model:

- expand from immutable snapshots,
- choose winners within each side deterministically,
- then resolve cross-side overlaps deterministically,
- then publish the merged state.

Any design that lets active-layer writers race on shared `parent`/`depths` maps
changes semantics, not just performance.

### 3. Heuristic scheduling state becomes timing-sensitive

`choose_next_layer` uses per-side cost and overlap telemetry from prior committed
layers. In the current model that data is stable for the duration of a layer.

If both directions run at once and update approximate-overlap state while the
other side is still computing, the scheduling heuristic becomes sensitive to
when counters are sampled. That is mainly a determinism/debuggability hazard,
but it also makes it much harder to reason about neutral-vs-negative complexity
changes, which `research/program.md` says should be reverted.

## Recommended Implementation Order

The evidence supports this order.

1. Make winner selection explicit before adding more concurrency.
   The current solver has an implicit total order. A future parallel design
   should not rely on incidental Rayon/shard ordering. Introduce an explicit
   stable layer serial or comparable tie-break key before attempting sharded
   dedup or parallel commit.
2. Reduce staging memory or make it budgetable.
   The current whole-layer `FrontierExpansion` buffering is the main structural
   memory risk. If we cannot stream, chunk-commit, or otherwise cap staging,
   deeper parallelism will amplify the wrong bottleneck.
3. Only then try deterministic intra-layer dedup parallelism.
   This is still the most plausible next performance target from `5b8`, but it
   should be built as a deterministic reduction over explicit winner keys, not
   as opportunistic concurrent insertion.
4. Leave dual-direction expansion for last.
   It requires a round/snapshot design, deterministic cross-side meet
   resolution, and a memory story. The current evidence does not justify doing
   that before deterministic dedup and staging control exist.

## Required Invariants And Tests Before Any Parallel Dedup Or Dual-Direction Work

At minimum, future work should lock down the following invariants.

### Dedup invariants

- canonical dedup must preserve a stable winner rule for equal `next_canon`
  candidates;
- same-future/past representative pruning must also preserve a stable winner
  rule, because it changes the explored frontier rather than only the surfaced
  witness;
- rerunning the same search with the same config must return the same witness
  and telemetry on the same binary.

### Parent / witness invariants

- for any kept candidate, the stored `parent`, `depth`, and `orig`
  representative must correspond to the same stable winner;
- equal-depth competing exact meets must resolve with a documented deterministic
  policy;
- witness reconstruction for a chosen meeting node must be invariant under
  thread scheduling.

### Memory / staging invariants

- per-layer candidate staging must have a measurable upper bound or telemetry
  surface before adding sharded buffers;
- dual-direction rounds must account for the simultaneous live footprint of both
  sides' staged candidates, winner maps, and next frontiers.

### Suggested tests

- unit tests asserting first-writer retention for canonical dedup and
  same-future/past representative selection;
- observer-backed regression tests that repeated runs return the same exact
  witness for a case with multiple same-layer duplicate candidates;
- a targeted exact-meet tie test that forces multiple admissible meet
  candidates in one layer and asserts the documented winner;
- if staging is refactored, a telemetry test that proves the reported candidate
  budget stays within the intended envelope.

## Conclusion

The current code already has a coherent serial contract:

- one side expands,
- the whole layer is staged,
- first representatives win dedup,
- first committed discoveries win parent/orig claims,
- and the first admissible exact meet ends the search.

That contract is why the solver is deterministic today.

Given the `5b8` timing data and the current `FrontierExpansion` memory shape,
the justified next step is not "more parallelism everywhere". It is:

1. make winner selection explicit and testable,
2. reduce or bound staging memory,
3. then try deterministic parallel dedup inside one layer,
4. and defer true concurrent forward/backward execution until snapshot semantics
   and meet-resolution rules are designed up front.
