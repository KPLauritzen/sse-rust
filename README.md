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

### Core tool: SSE path search

A BFS/DFS over nonneg integer matrix factorisations. Given A and B:
- Enumerate all ways to write A = UV where U ∈ ℤ₊^{n×m}, V ∈ ℤ₊^{m×n} for intermediate dimensions m = 1, 2, ..., m_max.
- Compute C = VU.
- If C = B, we found a lag-1 SSE.
- Otherwise, add C to the search frontier (if not already visited).
- Continue BFS up to a maximum lag.

Key parameters to bound:
- **max_lag**: maximum chain length
- **max_dim**: maximum intermediate matrix dimension
- **max_entry**: maximum entry value in intermediate matrices (needed to keep factorisation enumeration finite)

### Aligned shift equivalence

Carlsen, Dor-On & Eilers showed that aligned shift equivalence characterises SSE and that fixed-lag aligned SE algorithms perform better. Implementing this reformulation alongside naive factorisation search would be a concrete contribution.

### Visualisation of SSE paths

When a path is found, visualise the chain:
```
A = U₁V₁ → V₁U₁ = A₁ = U₂V₂ → V₂U₂ = A₂ → ... → B
```
Show the intermediate matrices, their dimensions, and the associated directed graphs. The graph perspective (in-splits, out-splits) could make the transformations more intuitive.

### Database / catalogue

Systematically enumerate SSE classes for small matrices (e.g. all 2×2 with entries ≤ N, then 3×3). Compare with Eilers & Kiming's results and extend them.

### Interactive web tool

No public tool exists for exploring SSE. A web-based explorer where users input two matrices and get back either a proof (the path) or evidence against (which invariants fail) would be genuinely useful to the symbolic dynamics community.

---

## Implementation Approach

### Language: Rust

The core computation is backtracking search with integer arithmetic and early pruning — exactly where Rust excels. Representing small matrices as fixed-size arrays on the stack gives good cache locality. The factorisation enumeration is CPU-bound with no need for external libraries.

Python would work for 2×2 with small entries but is ~50-100x slower for the enumeration inner loop. Since the goal is to push the boundary of what's computationally reachable, Rust is the right choice.

### Architecture

```
sse-explorer/
├── crates/
│   ├── sse-core/          # Matrix types, factorisation, invariants
│   ├── sse-search/        # BFS/DFS path search engine
│   └── sse-cli/           # CLI interface
├── sse-web/               # Optional: WASM web frontend
└── results/               # Computed SSE classes, counterexamples
```

### Core data structures

```rust
// Matrices with compile-time or runtime dimensions
// For 2×2 exploration, fixed-size arrays are ideal
struct Matrix<const N: usize> {
    entries: [[u32; N]; N],
}

// For variable-dimension intermediaries
struct DynMatrix {
    rows: usize,
    cols: usize,
    entries: Vec<u32>,
}

// An elementary SSE step
struct ESSEStep {
    u: DynMatrix,
    v: DynMatrix,
    // u * v = source, v * u = target
}

// A complete SSE path
struct SSEPath {
    matrices: Vec<DynMatrix>,  // A₀, A₁, ..., Aₗ
    steps: Vec<ESSEStep>,
}
```

### Search strategy

1. **Invariant pre-check**: Run cheap invariant checks first. Bail immediately if any mismatch.
2. **Iterative deepening**: Search lag-1 first, then lag-2, etc. Within each lag, iterate over max intermediate dimension.
3. **Canonical forms**: Use some canonical ordering on matrices to avoid revisiting equivalent states.
4. **Parallelism**: The search at each frontier node is independent — this parallelises trivially with rayon.

### Visualisation

- CLI: Print the chain of matrices and factorisations.
- Web (stretch goal): Interactive SVG/canvas showing the directed graphs at each step in the chain. Highlight the in-split / out-split structure of each elementary SSE. Use WASM to run the Rust search engine in-browser.

### Testing

Verify against known results:
- Baker's example: A₃ is SSE to B₃ over ℤ₊ via a chain of 7 elementary SSEs (some through 4×4 matrices). See Lind & Marcus p. 238.
- Kim & Roush counterexamples: matrices that are SE but not SSE.
- Eilers & Kiming's 2×2 catalogue.

---

## Getting Started

1. Implement `Matrix` types and basic operations (multiply, trace, characteristic polynomial).
2. Implement the factorisation enumerator for a given matrix and target intermediate dimension.
3. Implement lag-1 SSE check between two matrices.
4. Extend to BFS over chains.
5. Add invariant checks as pre-filters.
6. Run against known examples to validate.
7. Systematically explore 2×2 matrices and build a catalogue.

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
