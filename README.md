# Strong Shift Equivalence Explorer

## Problem Statement

Two square matrices **A** and **B** over the nonneg integers ℤ₊ are **elementary strong shift equivalent (ESSE)** if there exist (possibly rectangular) matrices **U** and **V** over ℤ₊ such that:

```
A = UV    and    B = VU
```

They are **strong shift equivalent (SSE)** if there is a finite chain of elementary strong shift equivalences connecting them:

```
A = A₀ ~ₑ A₁ ~ₑ A₂ ~ₑ ... ~ₑ Aₗ = B
```

where each `~ₑ` is an elementary SSE. The number of steps `ℓ` is called the **lag**.

Note that the intermediate matrices in the chain need not be the same size as A or B — the factorisation A = UV can go through any intermediate dimension m (U is n×m, V is m×n).

### Why does this matter?

SSE was introduced by R.F. Williams in 1973 to classify **shifts of finite type (SFTs)** — a fundamental class of dynamical systems in symbolic dynamics. Williams proved that two SFTs are **topologically conjugate** (i.e. "the same system up to relabelling") if and only if their adjacency matrices are strong shift equivalent. This reduces a deep dynamical systems question to matrix algebra.

### The weaker cousin: Shift Equivalence (SE)

Two matrices A, B are **shift equivalent** with lag k if there exist matrices R, S over ℤ₊ such that:

```
Aᵏ = RS,  Bᵏ = SR,  AR = RB,  SA = BS
```

SE is much easier to check (it's decidable), and Williams originally conjectured that SE and SSE are the same relation. **This conjecture is false** — Kim and Roush (1999) gave counterexamples for irreducible matrices, settling a major open problem. The gap between SE and SSE remains poorly understood and is still an active area of research.

### Decidability

It is unknown whether SSE is decidable in general. There is no known algorithm that, given two matrices, will always terminate and correctly report whether they are SSE. This is part of what makes computational exploration interesting: we can build tools that try hard within a computational budget, even if they can't always give a definitive answer.

---

## Relevant Research

### Foundational

- **R.F. Williams (1973)** — "Classification of subshifts of finite type." Introduced SSE and proved it characterises conjugacy of SFTs.
- **Kim & Roush (1999)** — ["Williams's conjecture is false for irreducible subshifts."](references/kim-roush-1999-math9907095.pdf) Proved SE ≠ SSE, disproving the Williams conjecture.
- **Wagoner (1999)** — "Strong shift equivalence theory and the shift equivalence problem." Survey connecting SSE to algebraic K-theory. Notes that machine computations of the Φ₂ invariant detected counterexamples.
- **Lind & Marcus (2021)** — *An Introduction to Symbolic Dynamics and Coding* (2nd ed., Cambridge). The standard textbook. Chapter 7 covers conjugacy, SSE, and SE in detail with worked examples.

### Computational

- **Eilers & Kiming (2008)** — "On some new invariants for strong shift equivalence for shifts of finite type" ([arXiv:0809.2713](references/eilers-kiming-2008-0809.2713.pdf)). **Key paper for this project.** Introduces a new computable invariant for SSE based on earlier work of Trow, Boyle, and Marcus. Includes a large-scale numerical experiment on all irreducible 2×2 matrices with entry sum < 25. Demonstrates cases where the new invariant disproves SSE where other invariants fail.
- **Kim & Roush (1990)** — "An algorithm for sofic shift equivalence." Published in Ergodic Theory and Dynamical Systems.
- **Boyle, Kim & Roush (2013)** — "Path methods for strong shift equivalence of positive matrices" ([arXiv:1209.5096](references/boyle-kim-roush-2013-1209.5096.pdf)). Develops path-based techniques for establishing SSE over dense subrings of ℝ.

### Recent theoretical advances (Kevin Aguyar Brix and collaborators)

- **Brix (2022)** — ["Balanced strong shift equivalence, balanced in-splits, and eventual conjugacy."](references/brix-2022-1912.05212.pdf) Ergodic Theory and Dynamical Systems. Introduces balanced SSE, connecting it to one-sided eventual conjugacy via graph operations (out-splits and balanced in-splits).
- **Carlsen, Dor-On & Eilers (2024)** — ["Shift equivalences through the lens of Cuntz-Krieger algebras."](references/carlsen-doron-eilers-2024-2011.10320.pdf) Introduces compatible, aligned, and balanced shift equivalence as intermediary relations between SE and SSE. **Notably, algorithms implementing aligned shift equivalence for a fixed lag perform better than naive SSE search** — this is a potential avenue for the explorer.
- **Brix, Dor-On, Hazrat & Ruiz (2025)** — ["Unital aligned shift equivalence and the graded classification conjecture."](references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf) Accepted in Mathematische Zeitschrift. Connects shift equivalence to Leavitt path algebras.
- **Brix & Ruiz (2025)** — "Unital shift equivalence" ([arXiv:2504.09889](references/brix-ruiz-2025-2504.09889.pdf)). Shows unital SE does not imply (balanced) SSE.
- **Brix, Mundey & Rennie (2024)** — ["Splittings for C*-correspondences and strong shift equivalence."](references/brix-mundey-rennie-2024-2305.01917.pdf) Extends in-splits to C*-correspondences.

### Algebraic K-theory connection

- **Boyle & Schmieding (2019)** — ["Strong shift equivalence and algebraic K-theory."](references/boyle-schmieding-2019-1501.04695.pdf) Shows the refinement of SE by SSE is captured by NK₁(R)/E(A,R), connecting this concrete matrix problem to deep algebra. The group E(A,R) depends on the SE class of A.

---

## Invariants for Disproving SSE

Before attempting the expensive search for an SSE path, we can check necessary conditions. If any invariant differs between A and B, they are **not** SSE. Ordered roughly by computational cost:

1. **Size of nonzero Jordan blocks** — SSE preserves the nonzero spectrum.
2. **Characteristic polynomial** (on nonzero eigenvalues) — must match.
3. **Trace sequences** — Tr(Aⁿ) = Tr(Bⁿ) for all n (counts periodic orbits). Check for small n.
4. **Determinant** — det(I - tA) = det(I - tB) as formal power series.
5. **Bowen-Franks group** — The cokernel of (I - A), an abelian group invariant of SE (and hence SSE).
6. **Dimension group / shift equivalence class** — checks whether A and B are SE. This is decidable and necessary for SSE.
7. **Eilers-Kiming invariant** — a refinement based on Trow-Boyle-Marcus invariants, specifically designed to separate SSE classes within an SE class.

---

## Avenues for Exploration

### Aligned shift equivalence

Carlsen, Dor-On & Eilers showed that aligned shift equivalence characterises SSE and that fixed-lag aligned SE algorithms perform better. Implementing this reformulation alongside naive factorisation search would be a concrete contribution.

### Visualisation of SSE paths

When a path is found, visualise the chain as directed graphs at each step, highlighting the in-split / out-split structure of each elementary SSE.

### Database / catalogue

Systematically enumerate SSE classes for small matrices (e.g. all irreducible 2×2 with entry sum ≤ 25, reproducing and extending Eilers & Kiming's experiment).

### Interactive web tool

The WASM bindings exist but there is no frontend yet. A web-based explorer where users input two matrices and get back either a proof (the SSE path) or evidence against (which invariants fail) would be useful to the symbolic dynamics community.

### Search improvements

See [docs/TODO.md](docs/TODO.md) for concrete approaches: bidirectional BFS, iterative deepening, smarter factorisation pruning, spectral pruning, and aligned shift equivalence.

### Parallelism

Native builds now expand each BFS frontier layer in parallel with `rayon`, using a collect-then-merge pass so collision detection and parent-map updates stay deterministic. The `wasm32` build keeps the same serial expansion path.

---

## Implementation

Rust library crate (`sse-core`) with WASM bindings. Native builds use `rayon` for frontier-level BFS parallelism; browser-targeted `wasm32` builds fall back to the serial search path.

**What it can do:**

- BFS search for SSE paths between 2×2 matrices, including through 3×3 intermediate matrices (rectangular factorisations).
- Disprove SSE via a chain of invariants: trace, determinant, Bowen-Franks group, generalised Bowen-Franks groups (18 polynomials from Eilers & Kiming 2008), and the Eilers-Kiming ideal class invariant.
- Experimentally search for aligned module shift-equivalence witnesses for small 2×2 cases.
  This is exposed separately from SSE search and is not currently used as an SSE proof method.
- Compile to WASM for in-browser use.

**Key source files:**

- [`src/aligned.rs`](src/aligned.rs) — Fixed-lag SE witnesses, aligned module witness verification, and bounded aligned module search.
- [`src/search.rs`](src/search.rs) — BFS search engine. Entry point: `search_sse_2x2`.
- [`src/invariants.rs`](src/invariants.rs) — All invariant checks, called as pre-filters before search. Entry point: `check_invariants_2x2`.
- [`src/quadratic.rs`](src/quadratic.rs) — Quadratic field arithmetic for the Eilers-Kiming ideal class invariant (binary quadratic form reduction, eigenvector ideal class computation).
- [`src/factorisation.rs`](src/factorisation.rs) — Exhaustive enumeration of nonneg integer factorisations A = UV (square and rectangular).
- [`src/matrix.rs`](src/matrix.rs) — Fixed-size `SqMatrix<N>` and dynamic `DynMatrix` types.
- [`src/wasm.rs`](src/wasm.rs) — WASM bindings exposing `search_sse` and the experimental `search_aligned_module` as JSON-returning functions.

**Building WASM:**

```sh
wasm-pack build --target web
```

This produces a `pkg/` directory with `sse_core.js` and `sse_core_bg.wasm`.

**Deployment:**

The WASM output is used by the [SSE Explorer](https://kplauritzen.dk/sse-explorer/) frontend, hosted on [kplauritzen.github.io](https://github.com/KPLauritzen/kplauritzen.github.io). The built WASM files (`sse_core.js` and `sse_core_bg.wasm`) are committed directly into that repo under `docs/wasm/`. After rebuilding, copy the files manually:

```sh
wasm-pack build --target web
cp pkg/sse_core.js pkg/sse_core_bg.wasm ../kplauritzen.github.io/docs/wasm/
```

---

## References

- Lind, D. & Marcus, B. (2021). *An Introduction to Symbolic Dynamics and Coding* (2nd ed.). Cambridge University Press.
- Williams, R.F. (1973). Classification of subshifts of finite type. *Annals of Mathematics*.
- Kim, K.H. & Roush, F.W. (1999). Williams's conjecture is false for irreducible subshifts. *Annals of Mathematics*. [(pdf)](references/kim-roush-1999-math9907095.pdf)
- Wagoner, J. (1999). Strong shift equivalence theory and the shift equivalence problem. *Bulletin of the AMS*.
- Boyle, M. & Schmieding, S. (2019). Strong shift equivalence and algebraic K-theory. *J. Reine Angew. Math.* [(pdf)](references/boyle-schmieding-2019-1501.04695.pdf)
- Eilers, S. & Kiming, I. (2008). On some new invariants for strong shift equivalence for shifts of finite type. arXiv:0809.2713. [(pdf)](references/eilers-kiming-2008-0809.2713.pdf)
- Carlsen, T.M., Dor-On, A. & Eilers, S. (2024). Shift equivalences through the lens of Cuntz-Krieger algebras. *Analysis & PDE*. [(pdf)](references/carlsen-doron-eilers-2024-2011.10320.pdf)
- Brix, K.A. (2022). Balanced strong shift equivalence, balanced in-splits, and eventual conjugacy. *Ergodic Theory and Dynamical Systems*. [(pdf)](references/brix-2022-1912.05212.pdf)
- Brix, K.A., Dor-On, A., Hazrat, R. & Ruiz, E. (2025). Unital aligned shift equivalence and the graded classification conjecture. *Mathematische Zeitschrift*. [(pdf)](references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf)
- Boyle, M., Kim, K.H. & Roush, F.W. (2013). Path methods for strong shift equivalence of positive matrices. *Acta Applicandae Mathematicae*. [(pdf)](references/boyle-kim-roush-2013-1209.5096.pdf)
