# Lean evaluation for narrow repo claims (2026-04-16)

## Question

For `sse-rust-9ls.4`, could Lean realistically help this repo now, and if so
on which bounded seams?

## Context

- Repo-local baseline:
  - no `*.lean` files,
  - no `lean-toolchain`,
  - no `lakefile.toml` / `lakefile.lean`,
  - local environment currently has no `lean`, `lake`, or `elan`,
  - CI is currently a single Rust-only job in `.github/workflows/ci.yml`.
- The recent durable seams are mostly narrow reporting/evidence surfaces:
  - measurement-only corpus auditing,
  - triangle-path quotient telemetry,
  - quotient-retained guide-pool A/B.
- Those seams are useful because they make bounded claims explicit, but they do
  **not** imply that the repo is ready for a broad formal-methods layer. Most
  of the recent value came from better evidence and cleaner contracts, not from
  theorem-heavy machinery.
- Lean's current official workflow is a project-local `lean-toolchain` managed
  by `elan`, with `Lake` as the standard build tool. `Lake` handles dependency
  fetching, tests, linters, and custom tasks, and the Lean toolchain includes a
  kernel re-checker for compiled proof artifacts. Mathlib already has useful
  infrastructure for finite matrices and Smith-normal-form style algebra.

## Good formalization candidates

### 1. Witness-path soundness and reanchoring

Rust surface:

- `src/search/path.rs`
- `validate_sse_path_dyn`
- `reverse_dyn_sse_path`
- `reanchor_dyn_sse_path`
- `permutation_step_between`

Candidate claim:

- If each step satisfies `A_i = U_i V_i` and `A_{i+1} = V_i U_i`, then the
  stored path is a valid SSE witness.
- Reversing a witness path preserves validity.
- Reanchoring by a permutation-similarity step preserves validity.

Why this is a good target:

- It is finite and algebraic.
- It directly matches the repo's trust question around validated guide
  artifacts and witness reuse.
- It does not depend on frontier heuristics, timeouts, or search ranking.

Expected payoff: medium.

Expected proof cost: low to medium.

### 2. Graph-move witness constructors

Rust surface:

- `src/graph_moves.rs`
- `enumerate_one_step_outsplits`
- `enumerate_one_step_insplits`
- same-future / same-past specialized families

Candidate claim:

- The constructed witnesses really satisfy the advertised algebra
  `A = D E`, `C = E D`.
- In-split soundness is the transpose dual of out-split soundness.
- Amalgamations reverse the same witness data.

Why this is a good target:

- The code comments and `TERMINOLOGY.md` already state these contracts
  explicitly.
- If the repo keeps adding structured move families, this proof style would
  remain reusable.
- This is closer to "trust small transformations" than to "verify the whole
  solver."

Expected payoff: medium.

Expected proof cost: medium.

### 3. Small invariant lemmas behind existing pruning and reporting

Rust surface:

- `src/invariants.rs`
- `check_square_power_trace_invariants`
- the Newton-recurrence justification inside `check_invariants_2x2`
- simple `2x2` Smith-normal-form helpers used for Bowen-Franks reporting

Candidate claim:

- ESSE and SSE preserve the nonzero spectrum, so `trace(M^k)` is invariant
  across square endpoints even when dimensions differ.
- For `2x2` matrices, equal trace and determinant imply equal power traces by
  Newton recurrence.
- The small closed-form Smith-normal-form formulas used in the `2x2` invariant
  layer are sound.

Why this is a good target:

- These are narrow "why this filter/report is legitimate" lemmas.
- They are durable even if the search frontier logic changes.
- They are easier to prove than the repo's deeper arithmetic code.

Expected payoff: low to medium.

Expected proof cost:

- low for the Newton/power-trace slice,
- medium for the Smith-normal-form framing.

### 4. `GL(2,Z)` dossier internals, but only partially

Rust surface:

- `src/invariants.rs`
- `gl2z_similarity_profile_2x2`
- `split_similarity_content_2x2`
- determinant-band classification
- `src/bin/profile_gl2z_similarity_2x2.rs`

Good subtargets:

- determinant-band classification,
- split-case `gcd(A - \lambda I)` content formulas,
- small permutation/conjugation invariance lemmas.

Bad first target:

- the irreducible quadratic-order ideal-class path behind the current
  arithmetic dossier.

Why this is mixed:

- Mathlib already has relevant finite-matrix and `SL(2, Z)`-adjacent
  infrastructure.
- But mirroring the repo's quadratic-order and ideal-class code would be much
  heavier than the immediate trust value justifies.

Expected payoff: low near term, possibly higher later for `2x2` dossiers.

Expected proof cost: medium to high.

## What Lean would be bad at here

- The main search loop in `src/search.rs`, factorisation enumeration in
  `src/factorisation.rs`, and beam or shortcut tuning are empirical and
  performance-driven. Lean will not tell us whether a frontier policy is useful
  on `brix_ruiz_k3`.
- The recent reporting seams are mostly measurement or telemetry contracts:
  - `research/notes/2026-04-16-measurement-corpus-baseline-audit.md` is about
    baseline cost and classification policy.
  - `research/notes/2026-04-16-triangle-path-telemetry.md` explicitly treats
    the quotient as telemetry, not a theorem engine.
  - `research/notes/2026-04-16-k3-quotient-retained-shortcut-ab.md` explicitly
    avoids claiming reconstructed witness-step correctness for canonicalized
    full paths.
  Formalizing those seams now would prove the wrong thing.
- The repo's matrix layer is runtime-native (`DynMatrix`, `SqMatrix<N>`), while
  Lean's standard matrix development is indexed by finite types. Any serious
  proof effort would first need an adapter layer from runtime dimensions to
  `Fin n`-indexed matrices.
- A Lean sidecar would not automatically verify the Rust implementation.
  Based on the current Lean toolchain model plus this repo's all-Rust CI, the
  realistic role is "kernel-checked spec and lemmas", not verified solver
  runtime.
- Deep arithmetic formalization around `quadratic.rs` and the Eilers-Kiming
  path is currently too expensive for the likely payoff.

## Setup and integration cost

- Bootstrap cost: low to medium.
  The repo would need a new `lean-toolchain`, a `Lake` package config, a small
  Lean source tree, and a second CI job next to the existing Rust-only job.
- Developer setup cost: low to medium, but nonzero.
  `elan` is missing locally today, so the repo would be adding the Lean
  toolchain from zero rather than extending an existing footprint.
- Modeling cost: medium.
  The first real work is not the theorem itself; it is translating repo-native
  matrices, steps, and path contracts into Lean-friendly statements.
- Maintenance cost: medium.
  Proofs tied too closely to current helper structure would need upkeep as move
  families and reporting seams evolve.
- Deep arithmetic cost: high.
  Anything that tries to mirror the current quadratic-order implementation goes
  well past the scope of a small evaluation or a first proof-of-concept.

## Near-term payoff

Lean looks useful only in a narrow sidecar role.

The best small win would be exactly one of:

1. a proof file for witness-path validity, reversal, and permutation
   reanchoring around `src/search/path.rs`;
2. a proof file that the out-split and in-split witness constructors implement
   the advertised `A = D E`, `C = E D` algebra;
3. a short invariant proof covering `trace(M^k)` preservation and the `2x2`
   Newton-recurrence shortcut justification.

All three are:

- durable,
- narrow,
- repo-specific,
- plausibly reusable when trusting guide artifacts, structured moves, or
  invariant-backed pruning.

I would **not** add Lean tooling in this slice.

If the repo revisits this later, the right experiment is a tiny sidecar package
that proves one of the seams above and nothing else. Success should be judged
by whether the proof clarifies or de-risks a real repo contract; if it does
not, stop there.

## Conclusion

Lean could realistically help this repo a little, but only on narrow soundness
claims around witnesses, structured moves, or small invariants.

It is **not** a good fit right now for:

- main-search behavior,
- performance work,
- current measurement/reporting seams,
- or deep quadratic arithmetic.

The realistic recommendation is:

- maybe later, as a small sidecar proof package;
- not now, as a repo-wide adoption or solver-integration plan.

## Sources

- Lean build tools overview:
  https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/
- `Lake`:
  https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/Lake/
- `elan`:
  https://lean-lang.org/doc/reference/latest/Build-Tools-and-Distribution/Managing-Toolchains-with-Elan/
- Lean proof validation:
  https://lean-lang.org/doc/reference/latest/ValidatingProofs/
- mathlib Smith normal form over PIDs:
  https://leanprover-community.github.io/mathlib4_docs/Mathlib/LinearAlgebra/FreeModule/PID.html
- mathlib fixed-determinant matrices / `SL(2, Z)` surface:
  https://leanprover-community.github.io/mathlib4_docs/Mathlib/LinearAlgebra/Matrix/FixedDetMatrices.html
