# Strong Shift Equivalence Explorer

This repo explores strong shift equivalence (SSE) over nonnegative integer
matrices, with current emphasis on hard small-matrix search cases and solver
surfaces that can later generalize beyond fixed `2x2` endpoints.

## Start Here

- Read this file for the project frame, current solver surfaces, and supported
  entry points.
- Read [`docs/README.md`](docs/README.md) for the durable docs map.
- Use `bd` for active work (`bd prime`, `bd ready`, `bd show <id>`). Do not
  treat markdown docs as the live plan.
- Read [`research/README.md`](research/README.md) for experiment workflow,
  logs, notes, and local run artifacts.

## Project Goals

This project is currently optimizing for four concrete search goals:

1. Find any path for `k = 3`. Solved.
2. Find a new shortest path with lag `< 7` for `k = 3`, or find new paths with lag `= 7`.
3. Find any path for `k = 4` or above.
4. Make the main solver endpoint-agnostic for square matrices up to dimension 4, possibly higher later.

## Problem At A Glance

Two square matrices **A** and **B** over the nonneg integers ℤ₊ are
**elementary strong shift equivalent (ESSE)** if there exist (possibly
rectangular) matrices **U** and **V** over ℤ₊ such that:

```text
A = UV    and    B = VU
```

They are **strong shift equivalent (SSE)** if there is a finite chain of
elementary strong shift equivalences connecting them:

```text
A = A₀ ~ₑ A₁ ~ₑ A₂ ~ₑ ... ~ₑ Aₗ = B
```

where each `~ₑ` is an elementary SSE. The number of steps `ℓ` is called the
**lag**.

The intermediate matrices in the chain do not have to match the endpoint size:
the factorization `A = UV` can go through any intermediate dimension `m`
(`U` is `n×m`, `V` is `m×n`).

Why this matters:

- SSE classifies shifts of finite type up to conjugacy.
- Shift equivalence (SE) is a weaker, decidable relation, but SE and SSE are
  not the same in general.
- Decidability of SSE over `Z_+` remains open, so the repo is built around
  strong filters, bounded search, and structured witness surfaces rather than
  a guaranteed decision procedure.

## Current Solver Surfaces

The supported solver front doors are:

- `cargo run --release --bin search -- ...` for direct endpoint runs,
  including staged search.
- `cargo run --profile dist --features research-tools --bin research_harness -- ...`
  for fixture-backed comparisons, campaigns, and measurement probes.

The main implemented search surfaces are:

- invariant filtering in [`src/invariants.rs`](src/invariants.rs),
- bidirectional endpoint search in [`src/search.rs`](src/search.rs),
- factorisation enumeration in [`src/factorisation.rs`](src/factorisation.rs),
- concrete-shift witness search in [`src/concrete_shift.rs`](src/concrete_shift.rs),
- balanced elementary-equivalence search in [`src/balanced.rs`](src/balanced.rs),
- graph-move experiments in [`src/graph_moves.rs`](src/graph_moves.rs),
- sampled positive-conjugacy proposal search in [`src/conjugacy.rs`](src/conjugacy.rs).

Native builds expand each BFS frontier layer in parallel with `rayon`, using a
collect-then-merge pass so collision detection and parent-map updates stay
deterministic.

## Documentation Split

This README is the repo entrypoint. It should stay high level.

- [`docs/README.md`](docs/README.md) is the durable docs map.
- [`docs/TODO.md`](docs/TODO.md) keeps durable solver/search context under a
  historical filename. It is not the live task tracker.
- [`docs/research-ideas.md`](docs/research-ideas.md) is the long-horizon idea
  bank tied to the current solver surfaces, not a ranked roadmap.
- [`research/README.md`](research/README.md) covers experiment workflow, logs,
  notes, and local run artifacts.
- `bd` owns active priorities, statuses, and next actions.

---

## Implementation

Rust library crate plus native CLI entry points. Native builds use `rayon` for
frontier-level BFS parallelism.

**What it can do:**

- BFS search for SSE paths between `2x2` matrices, including through `3x3`
  intermediate matrices (rectangular factorisations).
- Run the main solver in either `mixed` or `graph-only` mode.
- Disprove SSE via a chain of invariants: trace, determinant, Bowen-Franks
  group, generalised Bowen-Franks groups (18 polynomials from Eilers & Kiming
  2008), and the Eilers-Kiming ideal class invariant.
- Search for bounded concrete-shift witnesses for small `2x2` cases.
- Use a concrete-shift witness as a bounded fallback proof path from
  `search_sse_2x2` on finite essential pairs.

**Key source files:**

- [`src/concrete_shift.rs`](src/concrete_shift.rs) — Fixed-lag SE witnesses,
  concrete-shift witness verification, and bounded concrete-shift search. This
  is the current concrete-shift surface for aligned concrete shift, balanced
  concrete shift, and compatible concrete-shift witnesses.
- [`src/search.rs`](src/search.rs) — BFS search engine. Entry point:
  `search_sse_2x2`.
- [`src/invariants.rs`](src/invariants.rs) — All invariant checks, called as
  pre-filters before search. Entry point: `check_invariants_2x2`.
- [`src/quadratic.rs`](src/quadratic.rs) — Quadratic field arithmetic for the
  Eilers-Kiming ideal class invariant (binary quadratic form reduction,
  eigenvector ideal class computation).
- [`src/factorisation.rs`](src/factorisation.rs) — Exhaustive enumeration of
  nonneg integer factorisations `A = UV` (square and rectangular).
- [`src/matrix.rs`](src/matrix.rs) — Fixed-size `SqMatrix<N>` and dynamic
  `DynMatrix` types.

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
