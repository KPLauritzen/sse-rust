# Strong Shift Equivalence Explorer

## Project Goals

This project is currently optimizing for four concrete search goals:

1. Find any path for `k = 3`. Solved.
2. Find a new shortest path with lag `< 7` for `k = 3`.
3. Find any path for `k = 4` or above.
4. Make the main solver endpoint-agnostic for square matrices up to dimension 4, possibly higher later.

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
- **Boyle, Kim & Roush (2013)** — "Path methods for strong shift equivalence of positive matrices" ([arXiv:1209.5096](references/boyle-kim-roush-2013-1209.5096/paper.pdf)). Develops path-based techniques for establishing SSE over dense subrings of ℝ.

### Recent theoretical advances (Kevin Aguyar Brix and collaborators)

- **Brix (2022)** — ["Balanced strong shift equivalence, balanced in-splits, and eventual conjugacy."](references/brix-2022-1912.05212/paper.pdf) Ergodic Theory and Dynamical Systems. Introduces balanced SSE, connecting it to one-sided eventual conjugacy via graph operations (out-splits and balanced in-splits).
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

## Research Surfaces

This README is not the live roadmap. Use `bd` for active priorities, owners,
statuses, and next actions.

Long-lived project context currently sits in a few places:

- [`docs/TODO.md`](docs/TODO.md) keeps durable solver/search context and the
  current stack shape without trying to be a task list.
- [`docs/aligned-shift-equivalence.md`](docs/aligned-shift-equivalence.md)
  records the current concrete-shift surface in
  [`src/concrete_shift.rs`](src/concrete_shift.rs) and the remaining
  terminology caveats around older aligned-oriented API names.
- Small-case cataloguing and path visualisation remain useful long-horizon
  directions, but they are not maintained here as a checklist.

Native builds expand each BFS frontier layer in parallel with `rayon`, using a
collect-then-merge pass so collision detection and parent-map updates stay
deterministic.

## Documentation Map

- [`docs/README.md`](docs/README.md) explains how repo docs are split between
  project overview, roadmap context, and research workflow.
- [`docs/TODO.md`](docs/TODO.md) is roadmap context only; use `bd` for live
  actionable work.
- [`research/README.md`](research/README.md) explains where to put terse log
  entries, longer research notes, and local run artifacts.

---

## Implementation

Rust library crate plus native CLI entry points. Native builds use `rayon` for
frontier-level BFS parallelism.

**What it can do:**

- BFS search for SSE paths between 2×2 matrices, including through 3×3 intermediate matrices (rectangular factorisations).
- Run the main solver in either `mixed` or `graph-only` mode.
- Disprove SSE via a chain of invariants: trace, determinant, Bowen-Franks group, generalised Bowen-Franks groups (18 polynomials from Eilers & Kiming 2008), and the Eilers-Kiming ideal class invariant.
- Search for bounded concrete-shift witnesses for small `2x2` cases.
- Use a concrete-shift witness as a bounded fallback proof path from
  `search_sse_2x2` on finite essential pairs.

**Key source files:**

- [`src/concrete_shift.rs`](src/concrete_shift.rs) — Fixed-lag SE witnesses,
  concrete-shift witness verification, and bounded concrete-shift search. This
  is the current concrete-shift surface for aligned concrete shift, balanced
  concrete shift, and compatible concrete-shift witnesses.
- [`src/search.rs`](src/search.rs) — BFS search engine. Entry point: `search_sse_2x2`.
- [`src/invariants.rs`](src/invariants.rs) — All invariant checks, called as pre-filters before search. Entry point: `check_invariants_2x2`.
- [`src/quadratic.rs`](src/quadratic.rs) — Quadratic field arithmetic for the Eilers-Kiming ideal class invariant (binary quadratic form reduction, eigenvector ideal class computation).
- [`src/factorisation.rs`](src/factorisation.rs) — Exhaustive enumeration of nonneg integer factorisations A = UV (square and rectangular).
- [`src/matrix.rs`](src/matrix.rs) — Fixed-size `SqMatrix<N>` and dynamic `DynMatrix` types.

**Native build targets:**

Default `cargo build` now builds the library plus the main `search` CLI. The
research helper programs in `src/bin/` are available behind the
`research-tools` feature so iterative builds do not relink every helper binary
by default.

```sh
cargo build --release
cargo build --release --features research-tools --bins
cargo run --release --bin search -- --help
cargo run --profile dist --features research-tools --bin research_harness -- --cases research/cases.json --format pretty
```

Supported workflows now go through those two front doors:

- use `search` for direct endpoint runs, including the generic
  `guided-refinement` stage,
- use `research_harness` for benchmark-family fixtures, staged comparisons, and
  campaign-style scoring,
- and treat the remaining `research-tools` binaries as targeted diagnostics or
  paper-reproduction helpers rather than alternate solver entry points.

### Criterion baseline workflow (`benches/search.rs`)

Use named baselines for any performance comparison so we do not compare against
stale or implicit prior runs.

```sh
just bench-search-save-baseline <name>
just bench-search-compare-baseline <name>
```

Equivalent raw commands:

```sh
cargo bench --bench search -- --save-baseline <name>
cargo bench --bench search -- --baseline <name>
```

Suggested baseline naming: `<YYYY-MM-DD>-<short-label>`.

Use `cargo bench` or `just bench-search` without a baseline only for local
sanity checks where no performance claim is being made.

The Criterion suite in `benches/search.rs` now focuses on stable micro surfaces:
fast endpoint sanity checks plus telemetry-driven `expand_next_n` throughput
cases that run until a fixed expanded-node budget is reached.

Keep heavy scenario-family evaluation and campaign comparisons in
`research_harness` rather than moving those fixtures into Criterion.

Baseline comparison is required when:

- reporting a speedup or regression,
- deciding whether to keep a solver or search-policy change based on runtime,
- writing notes/PR summaries that cite benchmark deltas.

The older Brix-Ruiz-specific search sidecars (`brix_ruiz_k3`,
`find_brix_ruiz_graph_path`, and `find_brix_ruiz_path_shortcuts`) are retired
from the supported Cargo targets. Their source files remain in-tree as
historical references for the research log and notes.

### Persisting the visited search graph

The main `search` CLI can optionally persist the visited graph to a local
SQLite database:

```sh
cargo run --release --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 4 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --visited-db visited.sqlite \
  --telemetry
```

The persisted schema is graph-shaped even though it uses SQLite tables:

- `matrices` stores each unique square or rectangular matrix once.
- `search_runs` stores one row per invocation, including config, outcome, and
  serialized telemetry/result payloads.
- `run_nodes` stores the canonical nodes discovered from each side.
- `run_edges` stores each explored post-pruning SSE edge, including move family,
  BFS direction, layer/depth, `U`, and `V`.

The recorder is disabled by default and uses batched per-layer transactions with
WAL mode, so normal searches do not pay any storage overhead unless
`--visited-db` is enabled.

### Exporting reusable guide artifacts

The generic `search` CLI can also write a successful path witness as a reusable
`full_path` guide artifact:

```sh
cargo run --release --bin search -- \
  1,0,0,1 1,0,0,1 \
  --write-guide-artifact guide.json
```

This export only succeeds for results that include an explicit SSE path witness.
Concrete-shift-only witnesses, `not_equivalent`, and `unknown` results are not
serializable as `full_path` guide artifacts. The written JSON matches the
generic guide-artifact schema in `src/types.rs` and can be fed back into later
guided searches with `--guide-artifacts`.

---

## References

- Lind, D. & Marcus, B. (2021). *An Introduction to Symbolic Dynamics and Coding* (2nd ed.). Cambridge University Press.
- Williams, R.F. (1973). Classification of subshifts of finite type. *Annals of Mathematics*.
- Kim, K.H. & Roush, F.W. (1999). Williams's conjecture is false for irreducible subshifts. *Annals of Mathematics*. [(pdf)](references/kim-roush-1999-math9907095.pdf)
- Wagoner, J. (1999). Strong shift equivalence theory and the shift equivalence problem. *Bulletin of the AMS*.
- Boyle, M. & Schmieding, S. (2019). Strong shift equivalence and algebraic K-theory. *J. Reine Angew. Math.* [(pdf)](references/boyle-schmieding-2019-1501.04695.pdf)
- Eilers, S. & Kiming, I. (2008). On some new invariants for strong shift equivalence for shifts of finite type. arXiv:0809.2713. [(pdf)](references/eilers-kiming-2008-0809.2713.pdf)
- Carlsen, T.M., Dor-On, A. & Eilers, S. (2024). Shift equivalences through the lens of Cuntz-Krieger algebras. *Analysis & PDE*. [(pdf)](references/carlsen-doron-eilers-2024-2011.10320.pdf)
- Brix, K.A. (2022). Balanced strong shift equivalence, balanced in-splits, and eventual conjugacy. *Ergodic Theory and Dynamical Systems*. [(pdf)](references/brix-2022-1912.05212/paper.pdf)
- Brix, K.A., Dor-On, A., Hazrat, R. & Ruiz, E. (2025). Unital aligned shift equivalence and the graded classification conjecture. *Mathematische Zeitschrift*. [(pdf)](references/brix-doron-hazrat-ruiz-2025-2409.03950.pdf)
- Boyle, M., Kim, K.H. & Roush, F.W. (2013). Path methods for strong shift equivalence of positive matrices. *Acta Applicandae Mathematicae*. [(pdf)](references/boyle-kim-roush-2013-1209.5096/paper.pdf)
