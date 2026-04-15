# Literature-Backed Solver Ideas For Frontier-Growth Cases

## Question

Which literature results suggest the most plausible next solver improvements for
the current `sse-rust` search stack, given that:

- exact memoisation of tiny linear-solver outputs regressed,
- reusable row-candidate caches helped materially,
- the hard Brix-Ruiz probes still bottleneck on frontier growth more than on
  immediate proof discovery?

## Context

The current hot loop is still centered on:

- generic factorisation enumeration in `src/factorisation.rs`,
- bidirectional search and coarse quotienting in `src/search.rs`,
- `2x2` arithmetic screening in `src/invariants.rs`.

Recent local evidence matters for ranking:

- The best recent cache win reused *shape-level* row candidates, not exact
  solver outputs.
- The failed concrete-shift-guided ordering pass only reordered successors
  *inside* a layer-synchronous BFS, so it did not change the hard frontier
  shape.
- Split-sidecar work shows that blind local refinement universes can stay
  trapped in isolated components around the Brix-Ruiz family.

So the best next ideas are the ones that either:

- compress many equivalent frontier states into one cacheable representative,
- replace blind factorisation with structured move families from the papers, or
- cut cases off before the frontier grows.

## Evidence

### 1. Same-future / same-past quotient states

Paper/result:
Eilers-Ruiz (2019) defines the `(I+)` move and explicitly interprets it as
redistributing the past among vertices with the same future. Brix (2022) then
puts out-splits plus balanced in-splits at the center of eventual conjugacy.

Specific actionable idea:
Add a frontier quotient finer than the current `ApproxSignature`, based on
duplicate-row / duplicate-column classes and their multiplicities. Use it for:

- layer-local dedup of graph-style successors,
- cache keys for structured successor generation,
- best-first novelty scoring.

Why it seems relevant here:
This is the literature version of the cache pattern that already worked locally:
reuse information keyed by a reusable local shape, not by an exact solved
subproblem. It directly attacks frontier growth instead of speeding up a single
tiny solve.

Expected upside/risk:
Likely upside is a real reduction in frontier width and repeated structured-move
work on the hard `3x3` / `4x4` surfaces. The risk is over-coarsening. So the
first use should be dedup/order only, not hard pruning.

Smallest experiment first:
In `src/search.rs`, add a `same_future_past_signature` for square intermediates,
log how often multiple exact states collapse to one signature on
`brix_ruiz_k3_wide_probe`, and use the signature only for within-layer
representative selection on graph-derived successors.

### 2. Explicit row-split / diagonal-refactor / column-split moves

Paper/result:
Boyle-Kim-Roush (2013), Proposition 2.10 and Theorem 3.8, show that
nondegenerate SSE data can be reorganized through finitely many row splittings,
a diagonal refactorization, and finitely many column splittings. Lemma 3.7 shows
the diagonal refactorization can be lifted across splittings.

Specific actionable idea:
Promote these three move types from “implicit inside generic factorisation” to
first-class move families. In particular, add a bounded diagonal-refactorization
family instead of waiting for it to appear accidentally inside exhaustive
enumeration.

Why it seems relevant here:
The recent Baker-step note already points at a missing literal refactorization
vocabulary, not just “more search.” Structured `3x4` / `4x3` families already
helped recover missing Baker coverage. The next gap is still structured.

Expected upside/risk:
High upside if the hard path is genuinely hidden inside sparse split/refactor
chains. The risk is adding another move family that is too broad and recreates
the failed generic widening experiments.

Smallest experiment first:
In `src/factorisation.rs`, add a narrow same-size diagonal-refactorization
enumerator for `3x3` and `4x4` using tiny diagonal entries only, and gate
success on either:

- recovering Baker step 5 or an equivalent shortcut, or
- improving `brix_ruiz_k3_wide_probe` focus telemetry without widening runtime
  catastrophically.

### 3. Best-first search that breaks the layer barrier

Paper/result:
Bilich-Dor-On-Ruiz (2024) prove aligned concrete shift, balanced concrete
shift, and compatible concrete shift with a fixed lag coincide on finite
essential matrices, and explicitly note that fixed-lag aligned algorithms
performed better in experiments. Boyle-Kim-Roush (2013) also push toward
guided constructive paths rather than blind enumeration.

Specific actionable idea:
Replace FIFO frontier order with a best-first order that can act *across*
layers, using a score such as:

- lower dimension,
- fewer same-future / same-past classes,
- lower entry sum,
- structured-move-family bonus,
- closeness to a sampled positive-conjugacy waypoint.

Why it seems relevant here:
This addresses a failure mode the log already isolates: concrete-shift-guided
ordering failed because it only permuted work within a layer-synchronous BFS.
That cannot fix frontier-growth bottlenecks.

Expected upside/risk:
Potentially high leverage, because it changes which region of the search space
gets explored before the frontier blows up. The risk is heuristic noise. Unlike
hard pruning, though, it is easy to measure and back out.

Smallest experiment first:
In `src/search.rs`, replace one `VecDeque` frontier with a `BinaryHeap` keyed by
`(depth, heuristic)` only on telemetry-focus cases, and compare:

- exact meets,
- approximate-overlap hits,
- max frontier,
- terminal bottleneck label.

### 4. Aligned / compatible witness profiles as cacheable side data

Paper/result:
Bilich-Dor-On-Ruiz (2024), Definition 3.3 and Theorem A, recast SSE in terms of
concrete lag-`m` path-isomorphism data and report that aligned fixed-lag
implementations behaved better than some more complicated alternatives.

Specific actionable idea:
Do not try to memoize exact tiny linear solves again. Instead, memoize coarse
“partial witness profile” data attached to a state:

- small-lag path-count deficits,
- fiber-size patterns,
- compatible/aligned mismatch counts.

Use that profile only for ranking and telemetry at first.

Why it seems relevant here:
This is another coarse reusable state summary, analogous to the row-candidate
cache that already paid off. It also gives a mathematically justified guidance
signal instead of a purely ad hoc heuristic.

Expected upside/risk:
If the profile is informative, it could guide the search toward witness-like
regions without paying the cloning/synchronization cost that hurt the exact
solver caches. The risk is that the profile is too expensive unless kept very
small.

Smallest experiment first:
In `src/search.rs`, compute a cheap lag-`2` or lag-`3` aligned-profile summary
for `2x2` endpoints and the encountered `2x2` / `3x3` states, cache it by
canonical state, and use it only as a tie-breaker in successor ordering.

### 5. Second-stage quadratic-order arithmetic

Paper/result:
Eilers-Kiming (2008) did not stop at “ideal class yes/no”; the arithmetic lives
in the quadratic order attached to the `2x2` pair and includes order/Picard/
ideal-quotient structure. The repo currently implements only the ideal-class
slice.

Specific actionable idea:
Add a second-stage arithmetic dossier in `src/invariants.rs` / `src/quadratic.rs`
recording, at minimum:

- the discriminant and conductor/index of `Z[lambda]`,
- principal/non-principal status in the relevant order,
- bounded checks for the paper’s `xy = lambda^k` ideal-quotient style equations.

Why it seems relevant here:
When memoising exact subproblems regresses, stronger front-door filtering gets
more attractive. This is especially true for `2x2` negative cases, where the
paper already showed arithmetic can separate pairs that cheaper invariants miss.

Expected upside/risk:
Good upside for non-SSE negatives and for heuristic ranking on hard positives.
The risk is limited immediate benefit on the Brix-Ruiz family if those pairs are
actually SSE; so this should begin as profile/ranking data, not an aggressive
prune.

Smallest experiment first:
Expose a `QuadraticOrderProfile` in `src/invariants.rs`, print it for the
Eilers-Kiming benchmark negatives and the Brix-Ruiz family, and only then
promote theoretically justified mismatches into hard rejections.

### 6. Family-specific conjugacy-guided restricted vocabularies

Paper/result:
Brix-Ruiz (2025) gives the explicit `A_k, B_k` family and the similarity
`P_k`. Boyle-Kim-Roush (2013) connect SSE construction to guided positive-matrix
path methods. Locally, the repo already finds very small positive conjugators
for `k = 3, 4`.

Specific actionable idea:
Treat the known similarity not as a certificate, but as a proposal generator:
factor it into short products of diagonal scalings and shears, then translate
those pieces into candidate split/refactor moves and put them ahead of generic
factorisation.

Why it seems relevant here:
The hard family is the one place where the literature gives explicit structure.
The local sidecar log already says this structure points more toward diagonal
refactorizations and balanced elementary-equivalence witnesses than toward
blind BFS expansion.

Expected upside/risk:
Potentially very high leverage on the benchmark family. The risk is
overspecializing. This is acceptable if the first pass is gated behind
telemetry-only or move-ordering use.

Smallest experiment first:
In `src/factorisation.rs`, add one tiny proposal family derived from bounded
products of a diagonal scaling and one shear for the Brix-Ruiz `2x2` family, and
measure whether it improves the first productive layer or approximate-overlap
rate before making it a default move source.

## Conclusion

The strongest near-term literature signal is not “find a magical new
obstruction.” It is:

1. compress equivalent frontier states more aggressively,
2. make split/refactor families explicit instead of implicit,
3. let guidance act before frontier growth becomes irreversible.

The two ideas I would try first are:

1. same-future / same-past quotient signatures in `src/search.rs`,
2. a narrow diagonal-refactorization family in `src/factorisation.rs`.

Both match the current telemetry better than exact tiny-solver memoisation.
Both are bounded, measurable, and easy to revert.

## Next Steps

- Implement the quotient-signature experiment first if the next task is
  frontier control.
- Implement the narrow diagonal-refactorization family first if the next task is
  literal Baker-step / structured-vocabulary recovery.
- Keep larger higher-power / complete in-split ideas in reserve; they still look
  mathematically plausible, but they seem one step too large before the cheaper
  quotienting and move-ordering ideas are tested.

## Sources Consulted

- Boyle, Kim, Roush (2013), especially Definition 2.4-2.7, Proposition 2.10,
  Lemma 3.7, Theorem 3.8.
- Bilich, Dor-On, Ruiz (2024), especially Definition 3.3 and Theorem A.
- Eilers, Ruiz (2019), especially Move `(I+)` and the “same future / redistribute
  past” interpretation.
- Brix (2022), abstract/main theorem on eventual conjugacy via out-splits and
  balanced in-splits.
- Eilers, Kiming (2008), arithmetic invariants for irreducible `2x2` matrices.
- Brix, Ruiz (2025), the explicit `A_k, B_k` family and similarity structure.
