# Goal 4 dynamic endpoint parity audit (2026-04-25)

## Question

Audit the remaining `2x2`-specific solver behavior that still matters for Goal 4
("main solver endpoint-agnostic for square matrices up to dimension 4"), then
pick one concrete bounded parity slice rather than widening the architecture
discussion again.

## Current read of the seam

Dynamic square endpoint search is no longer missing the basic generic lane:

- `SearchRequest`, `SearchStage`, `SearchTelemetry`, observer events, CLI
  endpoint parsing, harness endpoint fixtures, and retained endpoint exact-meet
  inventories already operate on `DynMatrix` surfaces.
- dynamic square search now gets bounded power-trace filtering plus
  same-dimension Bowen-Franks screening before frontier expansion.
- dynamic mixed and dynamic `graph_only` observer `Layer` parity have already
  been restored in the recent April notes.

The remaining blockers are therefore not "dynamic endpoint search does not
exist" blockers. They are narrower `2x2` seams spread across filters, proof
surfaces, result modeling, and some reporting/persistence edges.

## Remaining `2x2`-only behavior by risk

### High risk: shared result/event boundary still carries a `2x2` proof type

Owner area:

- `src/types.rs`
- `src/search_observer.rs`
- `src/search/dispatch.rs`
- `src/bin/search.rs`
- `src/sqlite_graph.rs`

Current behavior:

- the generic finished-result surface is `SearchRunResult`, but one variant is
  still `EquivalentByConcreteShift(ConcreteShiftProof2x2)`;
- `SearchFinishedRecord` therefore carries a `2x2` proof type through the
  shared observer event boundary;
- CLI pretty/JSON output and sqlite result persistence serialize that
  `2x2`-specific proof variant directly.

Why this matters:

- this is the cleanest remaining place where the "generic square endpoint"
  boundary is not actually generic;
- any future same-dimension square structured proof family would currently need
  another hardcoded result variant or more special-case serialization.

Why it is higher value than a proof rewrite:

- it can be fixed as modeling/plumbing only;
- it does not require claiming new theorem semantics outside `2x2`;
- it directly matches the RFC's requirement that result/event surfaces become
  generic before persistence is treated as solved.

### Medium risk: `2x2` filter dossier is still richer than dynamic square filters

Owner area:

- `src/invariants.rs`
- `src/search.rs`

Current behavior:

- `check_invariants_2x2` still has the strongest early rejection surface:
  trace, determinant, standard Bowen-Franks, generalized Bowen-Franks, and the
  Eilers-Kiming ideal-class invariant;
- dynamic square search only uses bounded power traces plus same-dimension
  standard Bowen-Franks for dimensions up to `4`.

Why this matters:

- the dynamic square lane still accepts some pairs that the `2x2` lane can
  reject exactly;
- the remaining gap is real, but the missing checks are not generic square
  facts. They are theorem-backed `2x2` arithmetic.

Why this is not the chosen next slice:

- the remaining gap is mathematically narrow, not just plumbing;
- trying to "generalize" it now would either stay `2x2`-specialized anyway or
  force a broader arithmetic rewrite that is too large for this audit turn.

### Medium risk: structured proof and proposal surfaces remain explicitly `2x2`

Owner area:

- `src/search/shortcut.rs`
- `src/structured_surface.rs`
- `src/concrete_shift.rs`
- `src/balanced.rs`
- `src/conjugacy.rs`
- `src/search.rs`

Current behavior:

- bounded concrete-shift fallback is `2x2`-only;
- balanced elementary search is `2x2`-only;
- positive conjugacy seed hints and proposal surfaces are `2x2`-only;
- the structured-surface descriptor vocabulary is intentionally
  `StructuredSurfaceDescriptor2x2`.

Why this matters:

- these are the most visible "solver special cases" when reading the code;
- they are still legitimate special cases, but they are not generic square
  surfaces yet.

Why this is intentionally `2x2` for now:

- the semantics are theorem- or literature-scoped;
- Goal 4 does not require pretending those proofs are already generic.

### Medium risk: `2x2` endpoints still dispatch into a dedicated executor path

Owner area:

- `src/search/dispatch.rs`
- `src/search.rs`

Current behavior:

- `execute_endpoint_search_request` still sends ordinary `2x2` endpoint-search
  requests through `search_sse_2x2_with_telemetry_and_observer`;
- larger square endpoints use the dynamic executor;
- only `stratified_beam_refill` forces the dynamic path unconditionally.

Why this matters:

- parity work still has to be checked twice whenever a seam exists in both the
  `2x2` executor and the dynamic executor;
- however, changing dispatch policy now would be a broader solver rewrite and
  would risk default-search behavior.

### Low risk: CLI/reporting/persistence edges still expose path-vs-proof asymmetry

Owner area:

- `src/bin/search.rs`
- `src/sqlite_graph.rs`
- `src/bin/research_harness.rs`

Current behavior:

- guide-artifact export only supports path witnesses;
- concrete-shift results have dedicated outcome/reporting labels;
- endpoint exact-meet inventory is path-oriented, not structured-proof-oriented.

Why this matters:

- these edges are downstream of the result-modeling seam above;
- on their own they are not the main architectural blocker, but they will stay
  special-cased until the shared result surface is normalized.

## Intentionally `2x2` for now

These seams should stay explicit rather than being half-generalized:

- concrete-shift proof search and its relation taxonomy;
- balanced elementary proof search;
- positive conjugacy proposal search and root-seed hint ordering;
- generalized Bowen-Franks and Eilers-Kiming filtering;
- `StructuredSurfaceDescriptor2x2` as the current theorem/proposal vocabulary.

That is consistent with the RFC boundary: the solver orchestration should be
generic even while some proof shortcuts remain explicitly `2x2`-specialized.

## Chosen next parity slice

Chosen slice:

- generalize the shared search result surface for structured proof outcomes,
  while keeping concrete-shift as the only implemented structured proof.

Rationale:

- it removes a real `2x2` leak from the generic request/result/event boundary;
- it touches plumbing and type ownership, not theorem semantics;
- it makes observer, CLI, and sqlite surfaces ready for future non-path proof
  families without claiming that those families already exist;
- it is smaller and safer than changing endpoint dispatch policy or attempting a
  broader arithmetic generalization.

Not chosen:

- filter parity beyond the current dynamic square checks:
  worthwhile, but it is more theorem-specific and less cleanly generic;
- proof-shortcut generalization:
  explicitly out of scope until semantics are clear;
- endpoint-dispatch unification:
  valuable later, but too close to a broad rewrite for this slice.

## Follow-up bead

Opened:

- `sse-rust-cpd5` - `Generalize shared search result surface for structured proof outcomes`

Acceptance summary:

- remove direct `ConcreteShiftProof2x2` hardcoding from the generic outcome
  boundary;
- preserve current concrete-shift CLI/sqlite behavior, including the
  `equivalent_by_concrete_shift` label and relation details;
- add focused tests on one observer/persistence path and one CLI/JSON path.

## Validation run

Audit-only turn. No Rust code changed, so no `cargo test` was required.

Commands run during the audit:

- `bd show sse-rust-84v`
- `bd prime`
- `bd ready --json`
- `bd search "Goal 4 parity dynamic endpoint invariant filter 2x2"`
- `bd search "concrete shift dynamic endpoint parity"`
- `bd search "observer parity endpoint exact meet dynamic square"`
- targeted `sed`, `nl`, and `rg` reads over:
  - `research/program.md`
  - `docs/rfcs/rfc-001-main-search-shortcut-integration.md`
  - `research/notes/2026-04-18-dynamic-graph-only-observer-layer-events.md`
  - `src/search.rs`
  - `src/types.rs`
  - `src/structured_surface.rs`
  - `src/search/dispatch.rs`
  - `src/search_observer.rs`
  - `src/sqlite_graph.rs`
  - `src/bin/search.rs`
  - `src/bin/research_harness.rs`
  - `src/invariants.rs`
