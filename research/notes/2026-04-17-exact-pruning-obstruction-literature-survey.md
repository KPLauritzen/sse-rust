# Exact pruning and obstruction literature survey (2026-04-17)

## Question

For bead `sse-rust-ilu.4`, which existing results are actually relevant to:

- exact pruning and impossibility certificates for SSE search,
- move-family obstructions or exact family gates,
- symmetry-orbit reduction and quotient-style invariants,
- bounded-search no-go arguments that could become durable solver artifacts?

The goal is not a bibliography. The goal is a source-backed map from concrete
results to plausible solver use, caution, or rejection.

## Context

Recent local implementation slices already point at three relevant seams:

- exact early orbit dedup inside `square_factorisation_3x3`,
- BFS-only deferred witness reconstruction for exact bounded search,
- quotient/signature experiments around same-future/past structure.

So this survey prioritized papers that could strengthen one of these seams:

1. front-door exact `2x2` classification,
2. exact local orbit reduction or canonicalization,
3. structured move-family restriction before blind enumeration,
4. bounded exhaustive-search certificates,
5. cautionary results against overclaiming a cheap complete obstruction.

## Evidence

### 1. Baker (1983): narrow exact positive `2x2` territory

Source:

- [references/baker-1983-s0143385700002091.pdf](../../references/baker-1983-s0143385700002091.pdf)
- Cambridge abstract: <https://doi.org/10.1017/S0143385700002091>

Exact result:

- The paper's main theorem, as stated in the Cambridge abstract, is:
  if `A` and `B` are positive `2x2` integral matrices with non-negative
  determinant and are similar over the integers, then `A` and `B` are strongly
  shift equivalent.

Potential repo use:

- This is an exact positive-only shortcut for a narrow `2x2` slice.
- It can become:
  - a source-backed early accept for some `2x2` endpoints,
  - a control-case generator for harness calibration,
  - a positive dossier field alongside the current negative arithmetic screens.

Assumptions / danger:

- It does not give a negative obstruction.
- It is confined to a `2x2` positivity and determinant band.
- It does not directly help the hard `k=3` or `k=4` frontier unless the solver
  is already reducing a bounded subproblem to such an endpoint.

Judgment:

- `Directly usable`, but only as a narrow exact positive classifier.

### 2. Choe-Shin (1997): extends the Baker band, still only in `2x2`

Source:

- [references/choe-shin-1997-s0025579300012614/paper.html](../../references/choe-shin-1997-s0025579300012614/paper.html)
- Cambridge abstract: <https://doi.org/10.1112/S0025579300012614>

Exact result:

- The abstract states:
  if `A` and `B` are similar over the integers,
  `-2 tr(A) <= det(A) < -tr(A)`,
  and `|det(A)|` is not prime,
  then `A` and `B` are strongly shift equivalent.

Potential repo use:

- Same use pattern as Baker:
  a second exact positive-only band for `2x2`.
- Best use is a `gl2z`-similarity dossier that says:
  Baker-positive, Choe-Shin-positive, or outside known exact band.

Assumptions / danger:

- Again, not a negative obstruction.
- Again, too narrow to justify broad solver complexity.

Judgment:

- `Directly usable`, but only as another narrow `2x2` positive shortcut.

### 3. Eilers-Kiming (2008): exact `2x2` arithmetic obstruction, but the quotient follow-up is weaker than the current check

Source:

- [references/eilers-kiming-2008-0809.2713.pdf](../../references/eilers-kiming-2008-0809.2713.pdf)
- arXiv abstract: <https://arxiv.org/abs/0809.2713>
- local follow-up: [2026-04-16-2x2-ideal-quotient-followup.md](2026-04-16-2x2-ideal-quotient-followup.md)

Exact result:

- The arXiv abstract states that the paper introduces a new computable
  invariant for SSE and gives examples where it disproves SSE when the other
  invariants used there do not.
- The repo follow-up on the paper's Theorem 1 records the exact colon-ideal /
  ideal-quotient criterion:
  `A ~ B` iff there are `x in (A:B)` and `y in (B:A)` with `xy = lambda^k`
  for some `k >= 0`, and that the computable class-group consequence is an
  ideal-class comparison modulo primes dividing `lambda`.

Potential repo use:

- The exact arithmetic obstruction itself is already relevant and already partly
  implemented in the repo's quadratic-order machinery.
- The useful surviving idea is:
  keep a strong exact arithmetic dossier for `2x2` endpoints and for reduced
  `2x2` subproblems.

Assumptions / danger:

- The `2026-04-16` follow-up matters: the ideal-quotient consequence is a
  quotient of the ideal-class data, so it is weaker than the repo's current
  exact ideal-class comparison, not stronger.
- Therefore the paper does not currently justify a new exact pairwise screen
  beyond what the repo already keeps.

Judgment:

- `Maybe usable` as dossier/context.
- `Likely dead end` for a new exact prune beyond the existing ideal-class test.

### 4. Boyle-Kim-Roush (2013): exact constructive factorization of SSE paths into row split / diagonal refactorization / column split

Source:

- [references/boyle-kim-roush-2013-1209.5096/paper.pdf](../../references/boyle-kim-roush-2013-1209.5096/paper.pdf)
- [references/boyle-kim-roush-2013-1209.5096/source.tex](../../references/boyle-kim-roush-2013-1209.5096/source.tex)
- arXiv page: <https://arxiv.org/abs/1209.5096>

Exact result:

- Lemma `diagrefactoconj` shows that a diagonal refactorization `A = DC`,
  `B = CD` can be lifted to an elementary row splitting of `A` and an
  elementary column splitting of `B` which are conjugate.
- Theorem `easystreet` states:
  if nondegenerate `A, B` are SSE over `U_+` for a subfield `U` of `R`,
  then there is a nondegenerate `C` and a nonsingular diagonal `D` such that
  `A` reaches `C` by finitely many row splittings and `B` reaches `D^{-1} C D`
  by finitely many column splittings.

Potential repo use:

- This is not an obstruction paper.
- It is the cleanest theorem-backed justification for turning
  row-split / diagonal-refactorization / column-split into explicit bounded
  move families instead of hoping generic factorisation rediscovers them.
- It supports family-specific enumeration gates such as:
  - try row/column split envelopes before broader mixed enumeration,
  - add tiny diagonal-refactorization proposal families,
  - explain why a bounded search slice may safely focus on these witnesses.

Assumptions / danger:

- The theorem is constructive-positive, not negative.
- It does not justify pruning away all non-splitting paths unless the active
  search surface is explicitly restricted to a proved family.
- Overusing it as a universal hard gate would be incorrect.

Judgment:

- `Directly usable` as justification for exact structured move families.

### 5. Eilers-Ruiz (2019) and Brix (2022): same-future / balanced in-split structure is strong for quotienting and move design, weak as a global hard prune

Sources:

- [references/eilers-ruiz-2019-1908.03714/paper.pdf](../../references/eilers-ruiz-2019-1908.03714/paper.pdf)
- [references/eilers-ruiz-2019-1908.03714/xyz-paper-3-ix.tex](../../references/eilers-ruiz-2019-1908.03714/xyz-paper-3-ix.tex)
- [references/brix-2022-1912.05212/paper.pdf](../../references/brix-2022-1912.05212/paper.pdf)
- [references/brix-2022-1912.05212/source.tex](../../references/brix-2022-1912.05212/source.tex)
- arXiv page for Brix: <https://arxiv.org/abs/1912.05212>

Exact results:

- Eilers-Ruiz explicitly interpret Move `(I+)` as:
  when vertices have exactly the same future, one may redistribute their pasts
  among them freely.
- In the regular-graph class they record `111 = <O, I+>`, i.e. the relevant
  structure-preserving relation is generated by out-splits and balanced
  same-future past-redistribution moves.
- Brix proves the one-sided eventual-conjugacy analogue:
  eventual conjugacy of finite graphs with no sinks is generated by out-splits
  and elementary balanced in-splits, and balanced strong shift equivalence of
  adjacency matrices captures the corresponding graph-side structure.

Potential repo use:

- This is the strongest literature support for quotient/support-profile work.
- Plausible exact or near-exact uses:
  - partition states by same-future / same-past classes,
  - define exact orbit keys for balanced split families,
  - restrict certain family-specific proposal generators to balanced in/out
    split envelopes before generic enumeration,
  - strengthen the current `square_factorisation_3x3` orbit reasoning with a
    literature-backed “redistribute among symmetric future classes” viewpoint.

Assumptions / danger:

- These are not generic SSE negative criteria.
- Eventual conjugacy and graph `C*` move-generation statements are nearby, but
  not the same as a universal SSE obstruction for the solver's full matrix
  search space.
- The safe first use is exact dedup/orbit reduction or proposal restriction,
  not “reject this branch because same-future data looks wrong”.

Judgment:

- `Maybe usable`, with the best payoffs in exact orbit reduction and proposal
  shaping rather than in new hard impossibility tests.

### 6. Bilich-Dor-On-Ruiz (2024): exact alternative witness relations, promising for bounded positive search slices but not yet a negative certificate

Source:

- [references/bilich-dor-on-ruiz-2024-2411.05598/paper.pdf](../../references/bilich-dor-on-ruiz-2024-2411.05598/paper.pdf)
- [references/bilich-dor-on-ruiz-2024-2411.05598/source.tex](../../references/bilich-dor-on-ruiz-2024-2411.05598/source.tex)
- arXiv page: <https://arxiv.org/abs/2411.05598>

Exact result:

- The introduction and Corollary `cor-matrices-equivalent-relations` state that
  for finite essential matrices with entries in `N`, balanced, aligned, and
  compatible shift equivalence with lag `m` coincide, and consequently these
  relations coincide with SSE.
- The paper also notes that fixed-lag aligned implementations behaved better
  experimentally than some more complicated alternatives.

Potential repo use:

- Strong support for keeping aligned / balanced / compatible witness searches as
  exact structured surfaces rather than “heuristic sidecars”.
- Plausible uses:
  - more bounded exact structured enumeration instead of generic factorisation,
  - lag-bounded witness profiles as ranking features,
  - stronger exact positive certificates inside small-dimensional subproblems.

Assumptions / danger:

- “Coincides with SSE” is an existence theorem across all lags, not a theorem
  that a small fixed lag failure is an impossibility certificate.
- So a failed lag-`m` search is not by itself an exact prune unless the branch
  invariantly forces that lag bound.

Judgment:

- `Maybe usable`.
- Best interpreted as positive-structure guidance, not a new generic hard prune.

### 7. Wagoner (1990): RS triangle identities support path quotienting and bounded no-go ledgers more than matrix-state pruning

Source:

- [references/wagoner-1990-triangle-identities-and-symmetries-of-a-subshift-of-finite-type.pdf](../../references/wagoner-1990-triangle-identities-and-symmetries-of-a-subshift-of-finite-type.pdf)
- Pacific Journal PDF/abstract page: <https://msp.org/pjm/1990/144-1/p11.xhtml>

Exact result:

- The paper proves that `Aut(sigma_A)` is isomorphic to the fundamental group
  of the space `RS(sigma_A)` of strong shift equivalences built from algebraic
  RS triangle identities, and that the higher homotopy groups of `RS(sigma_A)`
  vanish.

Potential repo use:

- The concrete transferable part is not the full topology.
- The transferable part is that short witness chains satisfy local triangle /
  commuting-square relations, so many distinct chains are equivalent already at
  the path level.
- This suggests:
  - path-suffix quotienting for stored guides,
  - “triangle-collapsible” redundancy metrics,
  - bounded no-go certificates that record exhaustive search modulo both state
    symmetry and local path rewrites.

Assumptions / danger:

- Full RS-space machinery is much farther from the repo's practical search loop
  than local orbit keys or partition refinement.
- A direct theorem-backed matrix prune is not visible here.

Judgment:

- `Maybe usable` for path-canonicalization research.
- `Likely dead end` for near-term state-level exact pruning.

### 8. McKay (1998): canonical augmentation is the best nearby exact model for orbit pruning and bounded-envelope no-go certificates

Source:

- ANU author PDF: <https://users.cecs.anu.edu.au/~bdm/papers/orderly.pdf>
- DOI landing page: <https://doi.org/10.1006/jagm.1997.0898>

Exact result:

- The paper develops generation by canonical construction path.
- Theorem 1 states that, given the parent/augmentation axioms, `scan(X0, n)`
  outputs exactly one labelled object from each unlabelled object of order at
  most `n` descended from the root object.

Potential repo use:

- This is the best adjacent literature match for the repo's current exact orbit
  dedup work.
- Immediate implications:
  - the `square_factorisation_3x3` orbit-key seam is the right kind of exact
    reduction, not a heuristic shortcut;
  - if a move family is put into canonical-augmentation form, a failed bounded
    search becomes an auditable exact no-go certificate for that envelope;
  - the new BFS-only deferred witness idea aligns with this: canonical parent
    storage is exactly what makes an envelope-exhaustion certificate and witness
    reconstruction coexist cleanly.

Assumptions / danger:

- You only get the theorem if the augmentation relation, parent choice, and
  canonical acceptance test are exact.
- A weak or ad hoc quotient can silently destroy completeness.

Judgment:

- `Directly usable`.
- This is the strongest nearby template for exact symmetry reduction and
  bounded-search certificates in this repo.

### 9. Paige-Tarjan (1987): exact partition refinement is a strong next step for quotient/support-profile signatures

Source:

- Princeton abstract page: <https://collaborate.princeton.edu/en/publications/three-partition-refinement-algorithms/>
- DOI: <https://doi.org/10.1137/0216062>

Exact result:

- The abstract states that the paper gives improved partition-refinement
  algorithms for lexicographic sorting, relational coarsest partition, and
  double lexical ordering.

Potential repo use:

- The solver's current quotient ideas are mostly one-shot duplicate-row or
  duplicate-column signatures.
- Paige-Tarjan points to the next exact step:
  iteratively refine row/column partitions by how blocks interact with other
  blocks until reaching a coarsest stable partition.
- Plausible uses:
  - stronger exact support-profile signatures,
  - better canonical block structure before permutation search,
  - exact representative choice inside a family orbit.

Assumptions / danger:

- A refined partition is still only a quotient unless we prove completeness for
  the move family being deduped.
- So it should start as orbit reduction, ordering, or certificate compression,
  not a universal hard prune.

Judgment:

- `Maybe usable`, with the clearest value as a stronger exact signature layer.

### 10. Kim-Roush (1999): strongest caution against expecting one cheap complete obstruction

Source:

- [references/kim-roush-1999-math9907095.pdf](../../references/kim-roush-1999-math9907095.pdf)
- arXiv page: <https://arxiv.org/abs/math/9907095>
- Annals abstract page: <https://annals.math.princeton.edu/articles/12529>

Exact result:

- The paper proves that Williams' conjecture is false for irreducible shifts of
  finite type.

Potential repo use:

- This is not a pruning recipe.
- It is a hard caution on research direction:
  quotient-style invariants, shift-equivalence data, or “almost complete”
  arithmetic screens should not be expected to collapse SSE globally.

Assumptions / danger:

- The wrong lesson would be “obstructions are hopeless”.
- The right lesson is narrower:
  exact negatives are likely to be family-specific, dimension-specific, or
  envelope-specific, not one universal cheap invariant.

Judgment:

- `Directly usable` as a cautionary filter on which ideas deserve engineering.

## Conclusion

The most promising literature does not point to one magical new invariant.
It points to three narrower, more defensible directions:

1. exact orbit reduction and canonical augmentation,
2. exact structured move-family restriction,
3. narrow exact `2x2` arithmetic / similarity dossiers.

The least promising direction is spending large effort on quotient-style
arithmetic variants that only weaken existing exact data, or on hoping a single
global invariant will settle the hard cases.

## Ranked follow-up ideas

1. **Lift current orbit dedup toward canonical augmentation for bounded move families.**
   Use McKay as the model. Start where the repo already has signal:
   `square_factorisation_3x3` orbit keys and small structured families. The
   concrete goal is an exact bounded-envelope certificate saying “every
   canonical representative in this family/lag/size budget was exhausted”.

2. **Implement partition-refined exact support-profile signatures.**
   Use Paige-Tarjan style refinement to strengthen same-future/past and
   duplicate-row/column signatures. First use: exact representative selection,
   orbit compression, and certificate compression. Do not promote to universal
   hard pruning without a proof.

3. **Keep pushing explicit split / diagonal-refactorization / column-split families.**
   Boyle-Kim-Roush remains the cleanest theorem-backed justification for this.
   This is especially plausible for bounded `3x3` / `4x4` search where the hard
   gap looks structural rather than purely budgetary.

4. **Add a narrow `2x2` positive-classifier dossier around integer similarity.**
   Baker and Choe-Shin justify exact positive shortcuts in specific determinant
   bands. This will not solve Goal 2, but it can simplify subproblems and make
   `2x2` endpoint handling more source-backed.

5. **Treat balanced/aligned concrete-shift searches as exact bounded witness surfaces, not generic negative tests.**
   Bilich-Dor-On-Ruiz supports using them more confidently for exact positive
   witness search, but not for declaring impossibility after a failed small-lag
   run.

## Best next implementation candidate

The best next candidate for this repo is:

- an exact canonical-augmentation style bounded search slice for one existing
  structured family, with
  - a stronger partition-refined orbit key,
  - exact early orbit dedup,
  - deferred witness reconstruction from canonical parents,
  - and a durable “bounded no-go certificate” artifact when the envelope is
    exhausted.

That combines the strongest transferable literature result (McKay), the
strongest current local signal (exact orbit dedup and deferred-parent search),
and avoids overclaiming a new universal obstruction.
