# Research Ideas

These notes come from reading `README.md`, `docs/TODO.md`, the current search code, and all papers in `references/` via `pdftotext -layout`.

The goal here is not to filter aggressively. If an idea looked even mildly plausible as a way to improve the search, it goes here.

## Reprioritization From Sidecar Evidence

The local Brix-Ruiz sidecar experiments sharpen the picture.

- Positive conjugacy is now a higher-priority direction, not just a paper-driven hunch. The sidecar log finds simple diagonal conjugacies for `k = 3` and `k = 4`, with sampled affine paths staying positive.
- Blind widening of small split-sidecar graphs is lower priority than this document originally suggested. One-step and two-step out-split refinements fail, mixed in/out refinements fail, and the bounded `3x3 -> 2x2 -> 3x3` zig-zag sidecar appears to preserve small isolated components.
- Direct same-size balanced-elementary attacks also look less promising on the Brix-Ruiz family than they did at first glance.

So, after the sidecar pass, graph refinements should be thought of mainly as:

- heuristic signals,
- pruning or component-detection tools,
- and sources of candidate moves for the main solver,

not as something to keep widening blindly in isolation.

## What The Code Already Has

- Bounded bidirectional integer SSE search in [`src/search.rs`](/home/kasper/dev/sse-rust/src/search.rs).
- Invariant filtering in [`src/invariants.rs`](/home/kasper/dev/sse-rust/src/invariants.rs), including Bowen-Franks, generalized Bowen-Franks, and the Eilers-Kiming ideal-class invariant for `2x2`.
- Graph-move search experiments in [`src/graph_moves.rs`](/home/kasper/dev/sse-rust/src/graph_moves.rs).
- A bounded balanced-elementary witness search in [`src/balanced.rs`](/home/kasper/dev/sse-rust/src/balanced.rs).
- A bounded positive-conjugacy search in [`src/conjugacy.rs`](/home/kasper/dev/sse-rust/src/conjugacy.rs).

So the next ideas should mostly do one of three things:

- turn those side searches into better move generators for the main search,
- add new bounded search substrates,
- or add stronger obstructions so we stop wasting time earlier.

## Near-Term Search Ideas

- Use positive conjugacy as a proposal engine, not just a yes/no experiment.
  Why: Boyle-Kim-Roush show that paths of positive conjugate matrices are strongly tied to SSE over `R_+`. We already have a small positive-conjugacy search, but right now it only reports a witness. A better use would be:
  - find a small conjugator or a short path in conjugacy space,
  - sample intermediate positive matrices,
  - round or factor those samples into candidate row/column splits or rectangular factorizations,
  - inject those candidates into the integer search as prioritized moves.

- Add explicit row-split, column-split, and diagonal-refactorization moves as first-class edges.
  Why: Boyle-Kim-Roush repeatedly reduce arguments to row splittings, column splittings, and diagonal refactorizations. The current search is mostly generic factorization plus graph moves. A more structured move set may be much easier to guide than generic `RS/SR` enumeration.

- Search over small shear/diagonal conjugators before generic BFS expansion.
  Why: the Brix-Ruiz family already comes with concrete similarity matrices, and the current code finds simple positive conjugators like `diag(1, 2)` and `diag(1, 3)` in small cases. There may be a productive restricted family:
  - diagonal scalings,
  - unitriangular shears,
  - short products of those.
  If one of these almost works over `Z_+`, its failure pattern might tell us which split to try next.

- Turn balanced elementary equivalence into a layered search, not a one-shot bounded witness check.
  Why: Brix's eventual-conjugacy paper makes balanced SSE central, and the repo already has a bounded search for one balanced elementary step. But the sidecar evidence suggests direct same-size balanced witnesses are structurally unlikely for the Brix-Ruiz family, so this should be treated as a secondary direction unless it is feeding the main solver. The next move would be to:
  - search for short balanced zig-zags,
  - alternate balanced steps with ordinary SSE steps,
  - cache common intermediates `S`,
  - canonicalize by permuting columns of `S` to avoid duplicate states.

- Search for canonical common refinements instead of arbitrary graph-move sequences.
  Why: several papers suggest that repeated split operations can often be compressed into a better common refinement picture. However, the current sidecar log already rules out the smallest obvious common-refinement attacks around the Brix-Ruiz family, so this idea now looks more useful as a negative probe or ranking feature than as a mainline search program.

- Add a higher-block / power-graph move family.
  Why: Brix 2022 and Brix-Mundey-Rennie suggest that iterated in-splits can often be packaged as a single in-split on a higher-power graph. That suggests a bounded move:
  - build a small higher-block presentation,
  - perform one larger canonical split there,
  - project back down.
  This may dominate many tiny split moves, but only if it helps the main solver escape the isolated components already seen in the sidecar graph.

- Treat the complete in-split or dual graph as a "maximal refinement" waypoint.
  Why: Brix-Mundey-Rennie describe complete in-splits as a largest or universal-looking in-split. The sidecar evidence says we should be cautious here: this is more plausible as a restart heuristic or canonical probe than as a standalone refinement search.

- Interleave matrix search and graph search deliberately.
  Why: right now these are mostly separate experiments. A better strategy may be:
  - matrix BFS until stuck,
  - graph refinement burst used to score, prune, or propose moves,
  - matrix reduction burst,
  - repeat.
  The papers keep moving between these viewpoints; the implementation probably should too. But the sidecar log suggests the graph part should support the main solver, not become a separate widening project.

- Add best-first search with paper-driven heuristics instead of pure depth-bounded BFS.
  Candidate priorities:
  - lower total entry sum,
  - lower intermediate dimension,
  - fewer distinct row/column types,
  - closer to a known positive-conjugacy path sample,
  - closer to a common balanced witness `S`,
  - closer to an explicit similarity form from the Brix-Ruiz family.
  Why: the hard cases look less like "deep" instances than "badly guided" ones.

- Normalize states more aggressively after each move.
  Candidates:
  - simultaneous row/column permutations,
  - remove duplicate rows/columns when a move creates obvious redundancy,
  - sort by row/column signatures,
  - canonical labeling of the associated graph.
  Why: graph-move searches can explode on isomorphic duplicates.

## Brix-Ruiz Family Ideas

- Special-case the family `A_k = [[1, k], [k-1, 1]]`, `B_k = [[1, k(k-1)], [1, 1]]` as a template-search benchmark.
  Why: the family is explicit, the conjugator `P_k` is explicit, and the open cases are exactly the ones we care about.

- Search near the known similarity `P_k` instead of from scratch.
  Concrete idea:
  - factor `P_k` into short products of shear and diagonal matrices,
  - ask whether each factor can be simulated by a bounded sequence of positive elementary moves,
  - stitch those local simulations together.
  Local evidence already strengthens this idea: for `k = 3, 4`, much simpler diagonal conjugators exist than the generic `P_k`.

- Use family induction experiments.
  Half-baked idea:
  - compute witnesses for small `k`,
  - look for recurring split patterns,
  - extrapolate a parametric move schema in `k`.
  Even a wrong pattern could still give a strong heuristic for larger search.

- Search for "repair moves" that convert the explicit similarity into a positive integer zig-zag.
  Why: the obstruction may be narrow. The sidecar log suggests the conjugacy side is simple while the small split/refinement side is stubborn, which is exactly the pattern that makes "repair the similarity" look more plausible than "discover a common refinement from scratch".

## Alternate Witness Spaces

- Compatible shift equivalence as a shadow search space.
  Why: Carlsen-Dor-On-Eilers prove compatible/representable/strong Morita shift equivalence collapse back to SSE for finite essential matrices. That does not give a shortcut by itself, but it suggests searching for a compatible witness first, then trying to discretize it into an SSE path.

- Representable shift equivalence as a scoring heuristic.
  Half-baked idea:
  - use the operator-theoretic compatibility equations to define a real-valued "distance to representability",
  - prioritize matrix states that improve that score,
  - ignore correctness at the heuristic level.
  This is speculative, but it may be better than blind breadth-first expansion.

- Aligned module shift equivalence as a bridge generator.
  Why: the current `aligned.rs` machinery is already a sidecar. The paper does not yet give the missing matrix-level equivalence, so this is not a proof route. But aligned witnesses may still suggest useful intermediate rectangular factorizations or graph correspondences to try.

- Block-map search instead of only matrix search.
  Why: Brix 2022 ties eventual conjugacy to finite block-code data after suitable splits. Maybe the right search state is not just a matrix, but a bounded block map plus a refinement state. Then matrices are derived from that state, not the other way around.

## Stronger Obstruction Ideas

- Push the Eilers-Kiming arithmetic further.
  Why: the current code already uses their ideal-class invariant, and the paper suggests a deeper arithmetic structure around quadratic orders and equation solving. Even an incomplete implementation could rule out more candidates before the expensive search starts.

- Add a second-stage arithmetic screen specialized to the discriminant / order of the `2x2` pair.
  Half-baked idea:
  - once trace and determinant match,
  - compute the associated quadratic order,
  - test cheap necessary conditions for the relevant ideal-class equations,
  - only then launch the expensive search.

- Use Boyle-Schmieding only as a "do not over-promise" boundary.
  Why: the `NK_1` classification says the gap between SE and SSE is real and subtle. This does not look computationally useful for the current code, but it warns against assuming a simple invariant is still missing. In practice that argues for better search guidance rather than hoping for one easy obstruction.

- Revisit Kim-Roush style sign-gyration or periodic-point data only if it can be made algorithmic.
  Why: this looks more like obstruction theory than a direct search method, but if a cheap computable fragment exists, it could still be a useful prune.

## Wild Ideas

- Learn a move policy from solved toy instances.
  Not a theorem, just an experiment:
  - generate many small equivalent pairs,
  - log successful move sequences,
  - train a ranking heuristic for next moves.
  The search space may be too combinatorial for hand-written heuristics alone.

- Maintain a "cloud" of nearby real positive matrices while doing integer search.
  Why: the real path methods seem smoother than the integer problem. Maybe each integer state should carry a few nearby real conjugacy samples, and the integer search should prefer moves that keep that cloud connected.

- Search for a canonical over-approximate witness first, then tighten it.
  Example:
  - allow rational or real factorizations,
  - solve an easier relaxed problem,
  - project the relaxed witness back to integer candidates.
  This could fail badly, but it matches the way several papers move through larger ambient categories before returning to integer SSE.

- Use complete in-splits as a restart mechanism.
  When BFS stalls:
  - replace the current frontier by bounded complete in-split refinements,
  - canonicalize,
  - restart the matrix search from there.
  This should be treated as a cautious experiment, not a default strategy, because the current sidecar evidence shows small refinement universes can stay trapped in isolated components.

- Build a library of reusable "motifs" for successful zig-zags.
  If a hard case eventually yields a witness, store short subpatterns:
  - split shape,
  - rectangular factor shape,
  - reduction pattern.
  Then reuse those motifs in future searches.

## Implementation Candidates

- Promote [`src/conjugacy.rs`](/home/kasper/dev/sse-rust/src/conjugacy.rs) from a standalone experiment into a heuristic move generator for [`src/search.rs`](/home/kasper/dev/sse-rust/src/search.rs).
- Promote [`src/balanced.rs`](/home/kasper/dev/sse-rust/src/balanced.rs) from one-step witness search into a side-information module for ranking, proposal generation, and selective bounded zig-zag search.
- Add a unified experimental driver that can alternate:
  - ordinary factorization moves,
  - graph refinement probes,
  - balanced side-information,
  - conjugacy-guided proposal moves.
- Build benchmark scripts specifically around the Brix-Ruiz `k=3,4,5,...` family so each new heuristic can be measured against the same hard instances, especially whether it escapes the isolated sidecar components already observed locally.

## Papers Behind These Notes

- Boyle-Kim-Roush 2013: path methods, conjugacy paths, row/column splittings, diagonal refactorizations.
- Boyle-Schmieding 2019: `NK_1` and the algebraic gap between SE and SSE.
- Brix 2022: balanced SSE, eventual conjugacy, out-splits plus balanced in-splits, block-map viewpoint.
- Brix-Dor-On-Hazrat-Ruiz 2025: aligned module shift equivalence as a promising but not yet matrix-complete sidecar.
- Brix-Mundey-Rennie 2024: iterated in-splits compressed via higher-power graphs; complete in-splits as canonical refinements.
- Brix-Ruiz 2025: the explicit hard family and explicit similarities.
- Carlsen-Dor-On-Eilers 2024: compatible/representable shift equivalence as alternate witness spaces collapsing to SSE.
- Eilers-Kiming 2008: stronger arithmetic obstructions for `2x2`.
- Kim-Roush 1999: deeper obstruction phenomena that do not automatically yield constructive searches.
