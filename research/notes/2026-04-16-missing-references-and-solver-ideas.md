# Missing references and fresh solver ideas (2026-04-16)

## Question

For bead `sse-rust-9ls.6`, which missing references outside the current
`references/` inventory are worth keeping, and what concrete solver ideas do
they add beyond the repo's existing Brix/Boyle/Bilich/Eilers-heavy literature
set?

## Context

- The repo already has the main Brix/Boyle/Bilich-Dor-On-Ruiz/Eilers-Ruiz
  surfaces plus several literature notes built from them.
- This slice stayed intentionally bounded: a few sources only, chosen for
  solver-design relevance rather than bibliography coverage.
- I targeted three gaps:
  - old primary `2x2` positive criteria that could become exact shortcut logic;
  - a path-space reformulation suggesting search-path quotienting rather than
    more state widening;
  - one adjacent algorithmic pointer for stronger partition-based signatures.

New artifacts added in this pass:

- [references/baker-1983-s0143385700002091.pdf](../../references/baker-1983-s0143385700002091.pdf)
- [references/wagoner-1990-triangle-identities-and-symmetries-of-a-subshift-of-finite-type.pdf](../../references/wagoner-1990-triangle-identities-and-symmetries-of-a-subshift-of-finite-type.pdf)
- [references/choe-shin-1997-s0025579300012614/paper.html](../../references/choe-shin-1997-s0025579300012614/paper.html)

## Evidence

### 1. Baker (1983): exact `2x2` positive shortcut territory

Source:

- [references/baker-1983-s0143385700002091.pdf](../../references/baker-1983-s0143385700002091.pdf)

What is new relative to the repo:

- The repo already uses Baker-family material indirectly through later sources,
  but it did not have the older primary `2x2` paper itself in `references/`.
- That matters because this is the source line behind the classical
  "similar-over-`GL(2,Z)` implies SSE in a determinant band" fact that later
  `2x2` papers extend.

Why it seems actionable here:

- This suggests a **positive-only exact classifier** for a narrow `2x2` slice:
  first test whether the endpoints are similar over `GL(2,Z)`, then test
  whether they land in the known determinant window.
- That would not replace search, but it could:
  - short-circuit some `2x2` positives exactly;
  - produce source-backed control cases for harness/bench work;
  - give the invariant layer a mathematically grounded "known-positive" dossier
    field, complementary to the existing negative arithmetic screens.

Why I would keep it narrow:

- This looks like a theorem-backed shortcut for `2x2`, not a general move
  family for `3x3`/`4x4`.
- Promoting it into broad move enumeration would likely recreate the repo's
  earlier "theorem-shaped widening without frontier relief" failures.

### 2. Choe-Shin (1997): extends the Baker band into composite negative determinant territory

Source:

- [references/choe-shin-1997-s0025579300012614/paper.html](../../references/choe-shin-1997-s0025579300012614/paper.html)
- [references/choe-shin-1997-s0025579300012614/00README.json](../../references/choe-shin-1997-s0025579300012614/00README.json)

What is new relative to the repo:

- The repo had no reference for the immediate extension of the Baker-style
  `2x2` positive criterion into a negative-determinant window.
- The saved Cambridge landing page includes the citation abstract:
  if `A, B` are similar over the integers and
  `-2 tr(A) <= det(A) < -tr(A)`, then `A, B` are strongly shift equivalent when
  `|det(A)|` is composite.

Why it seems actionable here:

- It enlarges the same **exact positive-only `2x2` shortcut** idea rather than
  adding another negative obstruction.
- This is useful locally because the repo already spends time around
  determinant-sensitive `2x2` arithmetic and hard `2x2` intermediates.
- A small follow-up could:
  - add a `gl2z_similarity_profile_2x2`;
  - report whether a pair lands in the Baker band, Choe-Shin band, or neither;
  - harvest explicit control cases for `sse-rust-9ls.7`.

Why I would not over-invest yet:

- The gain is still confined to `2x2` positives.
- It helps case classification and benchmark construction more obviously than
  it helps Goal 2's hard `k=3` lag search directly.

### 3. Wagoner (1990): triangle identities point at path quotienting, not more state widening

Source:

- [references/wagoner-1990-triangle-identities-and-symmetries-of-a-subshift-of-finite-type.pdf](../../references/wagoner-1990-triangle-identities-and-symmetries-of-a-subshift-of-finite-type.pdf)

What is new relative to the repo:

- The current code and notes already do a lot of **state** dedup:
  canonical permutation, layer dedup, approximate signatures, and
  same-future/same-past collisions.
- What is mostly missing is a **path** quotient viewpoint. Wagoner explicitly
  builds strong-shift-equivalence space from RS triangle identities coming from
  Markov-partition triangles, i.e. local relations between different short
  chains of elementary SSE data.

Why it seems actionable here:

- The repo already stores parent chains and guide artifacts, but treats
  different short witness chains as distinct unless they land on the same final
  canonical matrix.
- Wagoner's viewpoint suggests a narrower experiment:
  - look for short graph-move triangles or commuting squares already present in
    local witness paths;
  - canonicalize a short path suffix modulo these local rewrites;
  - count "triangle-collapsible" path redundancy separately from ordinary state
    collisions.

That could help in two places:

- guide artifacts: avoid keeping several locally equivalent short chains;
- telemetry/ranking: penalize successors that only differ by a short
  triangle-identity reshuffle from already-seen work.

Why I would not chase the full theory right now:

- Full RS-space / homotopy / symmetry computations are much farther from the
  current bounded search problems than the local triangle-rewrite idea.
- This looks promising as a **research-only telemetry and canonicalization
  seam**, not as a near-term theorem engine.

### 4. Paige-Tarjan (1987): adjacent pointer for stronger partition-based signatures

Source considered:

- Robert Paige and Robert Tarjan, *Three Partition Refinement Algorithms*,
  SIAM Journal on Computing, 1987.

What is new relative to the repo:

- The public abstract page is enough to identify the key algorithmic fit:
  relational coarsest partition and double lexical ordering.
- The current `same_future_past_signature` only records exact duplicate
  rows/columns and simple class statistics. It does **not** iteratively refine
  row/column classes by how they interact with other classes.

Why it seems actionable here:

- A coarsest-partition style refinement could yield a stronger square-matrix
  signature for:
  - graph-proposal ranking;
  - within-layer dedup/order;
  - seeding `canonical_perm` block structure before trying permutations.

Why I did not add it to `references/` in this slice:

- The accessible mirror exposed the abstract page cleanly but blocked a direct
  artifact fetch in this environment.
- It is a useful adjacent algorithmic pointer, but not as central to this repo
  as the two symbolic-dynamics primaries above.

## Conclusion

What was actually new relative to the current repo:

- a missing primary source for narrow exact-positive `2x2` SSE territory
  (Baker 1983);
- a missing extension of that territory into a composite negative-determinant
  band (Choe-Shin 1997);
- a missing path-space / local-rewrite formulation of SSE search
  (Wagoner 1990);
- an adjacent partition-refinement algorithm pointer for stronger signatures
  (Paige-Tarjan 1987).

The ideas that look most actionable here are:

1. Add a `2x2` positive-only dossier/shortcut around `GL(2,Z)` similarity plus
   the Baker and Choe-Shin determinant bands.
2. Add a research-only metric for short path chains that collapse under local
   triangle-style rewrites, instead of measuring only state collisions.
3. If same-future/same-past quotienting keeps paying off, strengthen it with an
   iterated partition-refinement signature rather than only duplicate-row and
   duplicate-column classes.

The ideas that are probably not worth pursuing immediately are:

1. full Wagoner complex or periodic-point obstruction machinery;
2. turning the Baker/Choe-Shin theorems into broad main-search move families;
3. doing a much larger paywalled `2x2` theorem crawl before `sse-rust-9ls.7`
   has consumed the current positive-case literature.

## Next Steps

- Add a small `gl2z_similarity_profile_2x2` surface first as reporting, not
  pruning.
- If that profile cleanly explains selected literature positives, promote a few
  cases into `sse-rust-9ls.7`.
- Prototype triangle-collapsible-path telemetry on existing graph-only witness
  paths before touching the main frontier logic.
