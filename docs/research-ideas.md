# Research Ideas

These notes come from reading `README.md`, `docs/TODO.md`, the current search code, and all papers in `references/`.

The goal is not to filter too aggressively. If an idea looked even somewhat
plausible as a way to improve the search, it goes here. But the document is no
longer flat: the papers now support clearer thematic clusters than before.

This file is an idea bank, not the active backlog. Use `bd` for actionable work
and `research/notes/` for longer evolving dossiers.

## Major Corrections To The Local Picture

- The repo now has matrix-level concrete-shift validators and bounded search
  machinery in [`src/concrete_shift.rs`](../src/concrete_shift.rs), even
  though some public
  names still preserve the older local `module` terminology.
- Bilich-Dor-On-Ruiz 2024 defines matrix-level aligned concrete shift,
  balanced concrete shift, and compatible concrete shift for finite essential
  matrices and proves that they coincide with each other and with SSE.
- So the concrete-shift family is no longer a speculative sidecar. It is a
  legitimate search substrate for the main problem.
- The remaining gap is not source acquisition. It is search-space reduction,
  clearer naming, and better integration into the main solver.
- Carlsen-Dor-On-Eilers 2024 and the Matsumoto line of papers make the operator-algebraic witness spaces much less "remote" than they first looked. In several finite-essential settings they are exactly the same equivalence relation seen through better-structured data.

## What The Code Already Has

- Bounded bidirectional integer SSE search in [`src/search.rs`](../src/search.rs).
- Factorization enumeration in [`src/factorisation.rs`](../src/factorisation.rs).
- `2x2` invariants in [`src/invariants.rs`](../src/invariants.rs), including Bowen-Franks, generalized Bowen-Franks, and an Eilers-Kiming ideal-class test.
- Concrete-shift validation and bounded search in [`src/concrete_shift.rs`](../src/concrete_shift.rs).
- Balanced elementary-equivalence one-step search in [`src/balanced.rs`](../src/balanced.rs).
- Sampled positive-conjugacy proposal search in [`src/conjugacy.rs`](../src/conjugacy.rs).
- Graph-move search experiments in [`src/graph_moves.rs`](../src/graph_moves.rs).

That means the durable takeaways are mostly about structured moves, proposal
generation, and stronger filters before expensive search. This file no longer
tries to rank near-term bets; active prioritization and ownership belong in
`bd`.

## Durable Direction Clusters

- Structured move families matter more than another blind widening pass.
  Boyle-Kim-Roush, Eilers-Ruiz, and the Brix line all point toward explicit
  row/column splits, diagonal refactorizations, graph refinements, and related
  move vocabularies as the productive search substrate.

- Concrete-shift witnesses are first-class formulations, not side curiosities.
  Bilich-Dor-On-Ruiz and Carlsen-Dor-On-Eilers justify using aligned concrete
  shift, balanced concrete shift, and compatible concrete shift as direct
  bounded search surfaces for the main solver.

- The Brix-Ruiz family should stay the main hard regression surface.
  `A_k = [[1, k], [k-1, 1]]` and `B_k = [[1, k(k-1)], [1, 1]]` remain useful
  because they are explicit, difficult, and already expose where guidance
  quality matters more than raw frontier growth.

- Arithmetic pruning is still worthwhile, but it looks complementary rather
  than mainline. The current ideal-class check only uses part of the available
  Eilers-Kiming structure; stronger arithmetic dossiers are best viewed as
  support for search and diagnosis.

- Longer-horizon relaxations remain plausible research directions. Compatible
  or representable shift-equivalence data, block-map viewpoints, relaxed
  witnesses, and learned move ranking all still make sense as exploratory
  families, but not as standing roadmap commitments in this document.

## Ideas That Should Be Downgraded

- Blind widening of split-sidecar graphs.
  Why: the local sidecar experiments already push against this. One-step and two-step split refinements do not look like a robust mainline strategy.

- Treating module-aligned search as the main aligned program.
  Why: it is no longer the right target. It should be viewed as a heuristic bridge at most.

- Expecting one more cheap invariant to settle the hard cases.
  Why: Boyle-Schmieding and Kim-Roush both point the other way. Over rings, the SE/SSE gap is genuinely subtle; over `Z_+`, deep obstructions already exist. Better guidance and better structured witness spaces look more promising than hoping for one last easy obstruction.

- Same-size balanced elementary-equivalence search as a standalone solver.
  Why: the Brix-Ruiz sidecar evidence makes this look too rigid. Balanced elementary-equivalence search becomes more interesting once it is allowed to form short zig-zags or to propose moves for the main search.

## How The Papers Change The Search Strategy

- Boyle-Kim-Roush 2013 pushes strongly toward structured move families:
  - row splits,
  - column splits,
  - diagonal refactorizations,
  - positive-conjugacy paths.

- Bilich-Dor-On-Ruiz 2024 upgrades aligned concrete shift, balanced concrete shift, and compatible concrete shift from side ideas to direct SSE formulations.

- Carlsen-Dor-On-Eilers 2024 says compatible and representable shift equivalence are not merely suggestive analogies. In the finite-essential setting they are equivalent to SSE.

- Brix 2022 and Eilers-Ruiz 2019 make refined split moves much more targeted than the current graph sidecar code.

- Brix-Mundey-Rennie 2024 argues for compressed refinement moves via higher powers and complete in-splits.

- Eilers-Kiming 2008 says the current arithmetic pruning is only a first slice of the available `2x2` theory.

- Boyle-Schmieding 2019 and Kim-Roush 1999 both argue against magical thinking. There may not be one short missing invariant that makes the hard cases trivial.

## Relevant Code Surfaces

If one of these themes becomes active implementation work, track the concrete
sequencing in `bd`. The main repo surfaces implicated by the literature are:

- [`src/search.rs`](../src/search.rs) for the generic frontier engine and any
  integration of structured proposal sources.
- [`src/graph_moves.rs`](../src/graph_moves.rs) for explicit move-family work
  such as splits, refinements, and canonical probes.
- [`src/concrete_shift.rs`](../src/concrete_shift.rs) and [`src/balanced.rs`](../src/balanced.rs)
  for concrete-shift or balanced elementary-equivalence witness formulations.
- [`src/conjugacy.rs`](../src/conjugacy.rs) for proposal-generation ideas
  driven by sampled positive-conjugacy data.
- [`src/invariants.rs`](../src/invariants.rs) for stronger arithmetic or other
  pre-search screens.

## Short Per-Paper Takeaways

- Boyle-Kim-Roush 2013: constructive SSE arguments like row splits, column splits, diagonal refactorizations, and positive-conjugacy paths should become actual move families.
- Boyle-Schmieding 2019: the SE/SSE gap is structurally real; do not expect one cheap invariant to close it.
- Bilich-Dor-On-Ruiz 2024: matrix-level aligned concrete shift, balanced concrete shift, and compatible concrete shift are defined and are equivalent to SSE for finite essential matrices.
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
