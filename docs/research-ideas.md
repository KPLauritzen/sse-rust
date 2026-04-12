# Research Ideas

These notes come from reading `README.md`, `docs/TODO.md`, the current search code, and all papers in `references/`.

The goal is not to filter too aggressively. If an idea looked even somewhat plausible as a way to improve the search, it goes here. But the document is no longer flat: the papers now support a clearer priority order than before.

This file is an idea bank, not the active backlog. Use `bd` for actionable work
and `research/notes/` for longer evolving dossiers.

## Major Corrections To The Local Picture

- The repo now has matrix-level concrete-shift validators and bounded search
  machinery in [`src/aligned.rs`](../src/aligned.rs), even though some public
  names still preserve the older local `module` terminology.
- Bilich-Dor-On-Ruiz 2024 defines matrix-level aligned, balanced, and
  compatible shift equivalence for finite essential matrices and proves that
  they coincide with each other and with SSE.
- So aligned/balanced/compatible search is no longer a speculative sidecar. It
  is a legitimate search substrate for the main problem.
- The remaining gap is not source acquisition. It is search-space reduction,
  clearer naming, and better integration into the main solver.
- Carlsen-Dor-On-Eilers 2024 and the Matsumoto line of papers make the operator-algebraic witness spaces much less "remote" than they first looked. In several finite-essential settings they are exactly the same equivalence relation seen through better-structured data.

## What The Code Already Has

- Bounded bidirectional integer SSE search in [`src/search.rs`](../src/search.rs).
- Factorization enumeration in [`src/factorisation.rs`](../src/factorisation.rs).
- `2x2` invariants in [`src/invariants.rs`](../src/invariants.rs), including Bowen-Franks, generalized Bowen-Franks, and an Eilers-Kiming ideal-class test.
- Concrete-shift validation and bounded aligned search in [`src/aligned.rs`](../src/aligned.rs).
- Balanced one-step search in [`src/balanced.rs`](../src/balanced.rs).
- Positive-conjugacy search in [`src/conjugacy.rs`](../src/conjugacy.rs).
- Graph-move search experiments in [`src/graph_moves.rs`](../src/graph_moves.rs).

That means the best next ideas are mostly about:

- replacing generic expansion with more structured moves,
- turning existing sidecars into proposal generators,
- and adding stronger screens before expensive search.

## Current Priority Order

### Tier 1: Best Near-Term Bets

- Implement fixed-lag matrix-level aligned or compatible witness search.
  Why: this is now mathematically first-class, not heuristic. Bilich-Dor-On-Ruiz proves aligned, balanced, compatible, and SSE coincide on finite essential matrices. A lag-bounded solver in one of these formulations is therefore a direct SSE solver, not merely evidence.

- Choose the easiest of aligned, balanced, and compatible formulations and treat the others as equivalent frontends.
  Why: the papers prove equivalence, so there is no reason to commit to the hardest encoding. If compatible witnesses or balanced witnesses are easier to enumerate than aligned ones, use them.

- Add explicit row-splitting, column-splitting, and diagonal-refactorization moves as first-class search edges.
  Why: Boyle-Kim-Roush 2013 repeatedly reduce constructive SSE arguments to exactly these move families. The current search is much closer to generic factorization enumeration than to that structured decomposition.

- Turn positive conjugacy into a proposal engine rather than a standalone experiment.
  Why: the current code already finds simple positive conjugators in small Brix-Ruiz cases. Boyle-Kim-Roush suggest using positive-conjugacy paths as a bridge toward SSE. The productive workflow looks like:
  - find a small conjugator or path,
  - sample intermediate positive matrices,
  - extract candidate splits or refactorizations,
  - feed those candidates into the integer search with high priority.

- Replace pure bounded BFS with best-first search guided by paper-driven structure.
  Good ranking signals look like:
  - closeness to a sampled positive-conjugacy path,
  - closeness to a small aligned or compatible witness,
  - lower total entry sum,
  - lower intermediate dimension,
  - fewer distinct row and column types,
  - cheaper arithmetic profile in the quadratic-order data,
  - explicit similarity-form proximity on the Brix-Ruiz family.
  Why: the hard cases increasingly look mis-guided rather than merely deep.

- Make the Brix-Ruiz family the default benchmark family.
  Use `A_k = [[1, k], [k-1, 1]]` and `B_k = [[1, k(k-1)], [1, 1]]`.
  Why: the family is explicit, hard, and comes with known similarities and unital shift-equivalence structure. It is the right place to test whether a new heuristic is genuinely helping.

### Tier 2: Strong Secondary Directions

- Upgrade balanced search from one-step witness detection to short balanced zig-zag search.
  Why: Brix 2022 makes balanced SSE central to eventual conjugacy, and Bilich-Dor-On-Ruiz now shows balanced shift equivalence is actually equivalent to SSE in the finite-essential setting. The current same-size one-step search is too narrow.

- Add same-future and same-past graph moves as deliberate proposal generators.
  Why: Eilers-Ruiz 2019 and Brix 2022 make the refined split moves much more specific than "try random graph refinements". In particular, the `(I+)` viewpoint of redistributing the past of vertices with the same future looks like a natural bounded move family to mine for matrix proposals.

- Search for canonical common refinements instead of arbitrary split bursts.
  Why: the graph papers repeatedly compress many local splits into more canonical refinement pictures. That said, the local sidecar evidence already says small blind refinement universes can stay trapped in isolated components, so this should support the main solver rather than replace it.

- Add higher-block or higher-power refinement moves.
  Why: Brix-Mundey-Rennie show that iterated in-splits can often be realized as a single in-split on a higher-power graph. That suggests a bounded move family which may dominate many tiny local split steps.

- Use complete in-splits or dual graphs only as canonical probes or restart waypoints.
  Why: the literature gives them a universal flavor, but the local search evidence argues against widening blindly there. They look more useful as "jump to a canonical refinement and re-evaluate" than as a standalone search space.

- Strengthen the arithmetic pre-filter beyond the current ideal-class check.
  Why: Eilers-Kiming 2008 contains more structure than the current code uses. In particular:
  - the ideal quotient equations `xy = lambda^k` in colon ideals,
  - conductor and Picard-group conditions under good hypotheses,
  - finer order-theoretic restrictions in the quadratic setting.
  Even partial implementations should prune cases before expensive search.

- Add a bounded search over small diagonal and shear conjugators before generic expansion.
  Why: the Brix-Ruiz family already carries explicit similarities, and the current code finds very small diagonal conjugators for `k = 3, 4`. Short products of diagonal scalings and unitriangular shears may be a productive restricted witness family.

### Tier 3: Plausible But More Experimental

- Search over compatible shift equivalence data directly.
  Why: Carlsen-Dor-On-Eilers prove compatible shift equivalence and SSE coincide in the finite-essential setting. The witness equations may be easier to organize than raw matrix BFS, even if they are larger.

- Search over representable shift equivalence or common operator models as a heuristic state space.
  Why: representable shift equivalence is equivalent to SSE in the same regime, but the associated data are less obviously discrete. This looks more like a heuristic relaxation than a first implementation target.

- Search over block maps rather than only adjacency matrices.
  Why: Brix 2022 frames eventual conjugacy through block-map data after suitable splits. For some instances, the right state may be "bounded block code plus refinement metadata", with matrices derived from that state.

- Carry a relaxed real or rational witness along the integer search.
  Why: several papers move through larger ambient categories and then descend back to integer SSE. A practical version would solve a relaxed real problem, then use its failure pattern to prioritize integer moves.

- Learn a move policy from solved small instances.
  Why: if we can generate many small positive examples, the successful move traces may be regular enough to train a ranking heuristic. This is not a theorem-driven idea, but it matches the "guidance matters more than raw depth" picture.

- Build a library of reusable zig-zag motifs.
  Why: once a hard instance yields a witness, the short subpatterns in that witness are likely more reusable than the full proof.

## Ideas That Should Be Downgraded

- Blind widening of split-sidecar graphs.
  Why: the local sidecar experiments already push against this. One-step and two-step split refinements do not look like a robust mainline strategy.

- Treating module-aligned search as the main aligned program.
  Why: it is no longer the right target. It should be viewed as a heuristic bridge at most.

- Expecting one more cheap invariant to settle the hard cases.
  Why: Boyle-Schmieding and Kim-Roush both point the other way. Over rings, the SE/SSE gap is genuinely subtle; over `Z_+`, deep obstructions already exist. Better guidance and better structured witness spaces look more promising than hoping for one last easy obstruction.

- Same-size balanced-elementary search as a standalone solver.
  Why: the Brix-Ruiz sidecar evidence makes this look too rigid. Balanced search becomes more interesting once it is allowed to form short zig-zags or to propose moves for the main search.

## Family-Specific Ideas For `A_k, B_k`

- Search near the known similarity rather than from scratch.
  Concrete plan:
  - factor the known similarity into short diagonal and shear pieces,
  - ask which pieces already preserve positivity,
  - convert the failure of the others into candidate split or refactorization moves.

- Run induction-style experiments in `k`.
  Why: even a partially wrong parametric pattern can still be a strong heuristic.

- Use unital shift equivalence as benchmark information, not as proof.
  Why: Brix-Ruiz 2025 shows the family is unitally shift equivalent, but that notion is weaker than SSE. It is useful as structure and as a source of candidate witnesses, not as a terminating certificate.

- Treat the family as the default regression test for any new heuristic move source.
  A new idea should answer at least one of:
  - does it find `k = 3` or `k = 4` more directly,
  - does it explain the diagonal conjugators already found,
  - does it improve the frontier on `k = 5+`,
  - does it avoid the isolated graph-sidecar components seen locally.

## Stronger Obstruction Ideas

- Extend the Eilers-Kiming arithmetic in the exact quadratic order attached to the `2x2` pair.
  Why: the current code stops early in that story. The paper suggests more necessary arithmetic compatibility than just matching an ideal class.

- Add a second-stage "hard case arithmetic dossier" after the easy invariants pass.
  Candidates:
  - the quadratic order and conductor,
  - class-group and Picard-group data when computable,
  - bounded searches for the colon-ideal product equations from Eilers-Kiming.

- Use Boyle-Schmieding as a boundary marker, not as an implementation blueprint.
  Why: its main value here is to warn that the SE/SSE gap can hide in nilpotent or `NK_1` data over general rings. That is a reason not to over-promise from weak invariants.

- Revisit Kim-Roush style periodic-point or sign-gyration data only if a cheap computable fragment exists.
  Why: this is conceptually relevant obstruction theory, but it is far from an obvious practical prune in the current repo.

## How The Papers Change The Search Strategy

- Boyle-Kim-Roush 2013 pushes strongly toward structured move families:
  - row splits,
  - column splits,
  - diagonal refactorizations,
  - positive-conjugacy paths.

- Bilich-Dor-On-Ruiz 2024 upgrades aligned, balanced, and compatible shift equivalence from side ideas to direct SSE formulations.

- Carlsen-Dor-On-Eilers 2024 says compatible and representable shift equivalence are not merely suggestive analogies. In the finite-essential setting they are equivalent to SSE.

- Brix 2022 and Eilers-Ruiz 2019 make refined split moves much more targeted than the current graph sidecar code.

- Brix-Mundey-Rennie 2024 argues for compressed refinement moves via higher powers and complete in-splits.

- Eilers-Kiming 2008 says the current arithmetic pruning is only a first slice of the available `2x2` theory.

- Boyle-Schmieding 2019 and Kim-Roush 1999 both argue against magical thinking. There may not be one short missing invariant that makes the hard cases trivial.

## Concrete Implementation Candidates

- Add a new matrix-level aligned or compatible search module and wire it into [`src/search.rs`](../src/search.rs) as both:
  - a direct bounded witness search,
  - and a proposal source for the generic search.

- Promote [`src/conjugacy.rs`](../src/conjugacy.rs) into a move generator that emits prioritized candidate factorizations or structured split moves.

- Extend [`src/balanced.rs`](../src/balanced.rs) from one-step search to short balanced zig-zag search with caching of common intermediates.

- Refactor [`src/graph_moves.rs`](../src/graph_moves.rs) around targeted move families from the papers:
  - out-splits,
  - refined in-splits of the `(I+)` flavor,
  - higher-block refinements,
  - canonical probe moves.

- Upgrade [`src/invariants.rs`](../src/invariants.rs) with a second arithmetic stage specialized to the quadratic-order data of the `2x2` case.

- Add a unified experimental driver which can alternate:
  - ordinary factorization moves,
  - aligned or compatible witness search,
  - balanced side information,
  - conjugacy-guided proposals,
  - graph-refinement probes.

- Build benchmark scripts centered on the Brix-Ruiz family and record:
  - witness length,
  - maximum intermediate size,
  - number of expanded states,
  - which heuristic generated the successful move.

## Short Per-Paper Takeaways

- Boyle-Kim-Roush 2013: constructive SSE arguments like row splits, column splits, diagonal refactorizations, and positive-conjugacy paths should become actual move families.
- Boyle-Schmieding 2019: the SE/SSE gap is structurally real; do not expect one cheap invariant to close it.
- Bilich-Dor-On-Ruiz 2024: matrix-level aligned, balanced, and compatible shift equivalence are defined and are equivalent to SSE for finite essential matrices.
- Brix 2022: balanced SSE and refined split moves are central, especially through the block-map and eventual-conjugacy viewpoints.
- Brix-Dor-On-Hazrat-Ruiz 2025: module-aligned search still has heuristic value, but matrix-level aligned search is now the more relevant target.
- Brix-Mundey-Rennie 2024: iterated in-splits can often be compressed through higher-power constructions; complete in-splits are natural canonical probes.
- Brix-Ruiz 2025: the explicit `A_k, B_k` family is the right benchmark family, and unital shift equivalence provides structure but not proof.
- Carlsen-Dor-On-Eilers 2024: compatible and representable shift equivalence collapse to SSE in the finite-essential setting.
- Eilers-Kiming 2008: there is more `2x2` arithmetic available than the repo currently uses.
- Eilers-Ruiz 2019: refined graph moves, especially the "same future" in-split picture, suggest more targeted proposal moves than blind graph widening.
- Kim-Roush 1999: sophisticated periodic-point obstructions exist, but converting them into practical computation is nontrivial.
- Matsumoto 2007: diagonal-preserving orbit-equivalence data come with explicit cocycle structure and topological full-group witnesses.
- Matsumoto 2015: continuous orbit equivalence and eventual one-sided conjugacy can be expressed through gauge and diagonal preserving isomorphisms with cocycle control.
- Matsumoto-Matui 2013: continuous orbit equivalence forces flow equivalence and determinant compatibility, reinforcing the usefulness of ordered cohomology and groupoid data as screens.
