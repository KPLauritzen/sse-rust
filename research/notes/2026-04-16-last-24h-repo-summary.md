# Repo activity summary: 2026-04-15 to 2026-04-16

## Question

What were the highest-signal repo changes over roughly the last 24 hours,
looking at both `git` activity and `bd` updates, and which of those changes
materially shifted solver work or autoresearch context?

## Scope

This note reviews activity in roughly the window from `2026-04-15 04:15 UTC`
through `2026-04-16 04:15 UTC`.

Primary sources:

- `git log main --since='24 hours ago'`
- focused `git log` slices over `src/`, `research/`, `docs/`, and `benches/`
- `bd list --all --updated-after '2026-04-15T04:15:00Z' --json`
- `bd list --all --created-after '2026-04-15T04:15:00Z' --json`
- `research/program.md`
- `research/cases.json`
- the new notes and log entries landed during the same window

## Highest-signal changes

### 1. Harness and benchmark surfaces were substantially reshaped

- `benches/search.rs` was rebuilt around stable micro and throughput probes
  instead of heavier scenario-style Criterion runs (`a0abb33`, `bc2ef78`,
  `59c327b`, `1bc4e0f`, `dc85dc1`). The explicit repo decision is now:
  scenario-level comparisons belong in `research_harness`, while Criterion stays
  focused on fast endpoint checks plus `expand_next_n` throughput.
- `research_harness` gained repeated measurement support for non-gating cases
  (`e7cb7c6`) and then a first deepening surface via per-case
  `deepening_schedule` expansion (`e349c35`). The important nuance is that the
  harness can now express repeated timing and deterministic ramped attempts, but
  the corpus has not yet started using deepening schedules.
- The harness/corpus public configuration surface now prefers
  `move_family_policy` over the older `search_mode` wording (`8606fcd`), which
  matters because recent solver work added a real intermediate policy between
  graph-only and fully mixed search.

### 2. The main search gained new bounded policy seams, but not a new Goal 2 or Goal 3 win

- `MoveFamilyPolicy::GraphPlusStructured` landed as a real intermediate preset
  (`a4235a6`). It keeps graph moves plus bounded structured families while
  excluding the broad `square_factorisation_3x3` sweep. This is a genuine new
  experiment surface for workers that want more structure than graph-only
  without paying for the full mixed policy.
- A first explicit `3x3` diagonal-refactorization family landed in the main
  factorisation selection seam (`a74a6a6`). This advances the long-running
  "explicit structured families" backlog, but only as a narrow first slice.
- Dynamic square-endpoint search now rejects mismatched endpoints using power
  traces up through `trace(M^4)` when dimensions permit (`b90bf13`). This is
  the first concrete generic-square parity improvement after Goal 4 moved into
  active backlog.
- The large maintainability tranche also landed: `src/search.rs`,
  `src/factorisation.rs`, and `src/bin/research_harness.rs` were split into
  smaller seams (`ae18a69`, `1164c2a`, `bf4db90`, `ac8737c`, `c4553a9`,
  `76c61ad`, `0219291`). This does not change solver results directly, but it
  lowers the cost of future frontier, proposal, and harness-policy work.

### 3. Several research seams became explicit, and most of the evidence was diagnostic rather than victorious

- The graph-move proposal slice (`d8b2c3c`, `761426b`) is currently the
  strongest "new research-only seam" result from the day. Raw proposal
  generation is broader than blind one-step graph expansion, but the existing
  quotient-signature score can collapse it to a tiny shortlist. On the
  `guide:1 -> guide:15` waypoint probe, the best shortlist had size `1`, beat
  every blind one-step successor on the same score, and was realizable in a
  bounded `3`-step graph-only probe.
- The sampled positive-conjugacy line became much clearer, but mostly in the
  negative direction (`eeccc8b`, `73c69e8`, `8a54fa9`). The new usefulness and
  seed probes showed that the current top-ranked sampled proposals are poor
  literal waypoint candidates: on `brix_k3` and `brix_k4`, they usually die on
  determinant or Bowen-Franks checks before any real bounded search begins.
  The plausible follow-up is local seed or invariant-aware reprojection work,
  not "use the current proposal ranking as an exact intermediate target."
- Beam-to-BFS handoff got a proper config seam and bounded validation
  (`4c0fb4b`, `0b84660`, `7a24a61`, `520d29a`). The result is currently
  negative on the graph-only `k=3` control:
  - plain beam remains cheap and returns `unknown`,
  - depth-only sweeps through `0/2/4/6/8/10` do not rescue the mode,
  - a deferred cap of `10` avoids timeout but still does more work than plain
    beam and remains `unknown`.
  This turns `beam_bfs_handoff` into a measurable research surface, but not a
  recommended default.

### 4. Corpus and research notes broadened beyond Brix-Ruiz, which matters for future evaluation quality

- The repo added a non-Brix literature lane in `research/cases.json`
  (`a6eae3c`, `a0786a9`, `f9570f8`, `bac20ab`):
  - `riedel_baker_k4/k6/k8/k10/k12`
  - `lind_marcus_a_to_c`
  - `full_2_shift_higher_block_1x1_to_4x4`
- The explicit benchmark decision was to keep the Riedel/Baker family
  harness-only rather than promote it into Criterion (`1bc4e0f`). That keeps
  `benches/search.rs` on low-noise micro/throughput terrain while still giving
  the harness a lag-sensitive non-Brix literature ladder.
- New research notes also clarified the current Goal 3 boundary. On the present
  head, the `k=4` mixed-beam surface completes with `unknown` through `lag 8`
  and times out at `lag 9` under the tested `beam64 + dim5 + entry10` release
  configuration (`563e341`, `1f84b62`). This is useful boundary evidence, but
  not a new witness.

### 5. The literature/context refresh produced two concrete new directions, and one of them already landed

- The missing-reference pass (`e0108b8`, `46c9dec`) added Baker 1983,
  Wagoner 1990, and a Choe-Shin 1997 landing-page reference. The actionable
  conclusions were:
  - a narrow exact-positive `2x2` dossier around `GL(2,Z)` similarity and the
    Baker/Choe-Shin determinant bands,
  - research-only triangle-collapsible path telemetry inspired by Wagoner's
    local triangle identities,
  - possibly stronger partition-refinement signatures later.
- The first of those follow-ups already landed as a reporting-only
  `gl2z_similarity_profile_2x2` surface plus a research-facing binary
  (`d90afd9`). That is an invariant/reporting addition, not a pruning or search
  change.
- The second follow-up is already represented in the backlog as
  `sse-rust-t4p`, which asks for triangle-collapsible witness-path telemetry
  rather than a full RS-space or theorem-engine attempt.

### 6. Terminology and durable docs were intentionally tightened before more solver work

- RFC 003 was accepted and rolled out (`dabf227`, `9f1c69c`, `72bd1d7`,
  `242a17f`, `bc1fd42`, `e9dd0b1`, `d54527e`).
- The important durable effect is that the repo now distinguishes:
  - concrete-shift work,
  - balanced-elementary work,
  - sampled positive-conjugacy proposal work.
- `src/aligned.rs` became `src/concrete_shift.rs`, structured-surface
  descriptors were added, and open bead wording was updated to stop overstating
  positive-conjugacy semantics.
- Separate docs cleanup (`518e0b0`) also removed stale "live roadmap" material
  from `README.md` and other durable docs in favor of `bd` and note-backed
  context.

## Backlog and priority shifts

- `sse-rust-9ls.3`, `sse-rust-9ls.5`, `sse-rust-9ls.7`, `sse-rust-2uy.11`,
  `sse-rust-2uy.14`, `sse-rust-2uy.15`, `sse-rust-aa8`, `sse-rust-x15`, and
  `sse-rust-sty` all closed during the window. That means the repo spent much
  of the day turning earlier backlog ideas into durable harness, search-seam,
  terminology, and literature artifacts rather than running open-ended solver
  loops.
- `sse-rust-2uy.2`, `sse-rust-2uy.3`, `sse-rust-2uy.8`, and `sse-rust-2uy.10`
  remain in progress, and their current notes now match the codebase better:
  diagonal refactorization is only the first structured-family slice, sampled
  positive conjugacy is still a research-side proposal surface, iterative
  deepening currently lives in the harness rather than the solver, and generic
  square parity has only reached power-trace prefiltering so far.
- New follow-up beads opened during the same window point to the current live
  frontier:
  - `sse-rust-oaj`: small deferred-cap sweeps around beam width for
    `beam_bfs_handoff`,
  - `sse-rust-t4p`: triangle-collapsible witness-path telemetry,
  - `sse-rust-a2n`: this review/documentation task itself.
- There was one bookkeeping glitch worth remembering: `sse-rust-x53` duplicated
  the same GL(2,Z)-similarity reporting slice already tracked canonically by
  `sse-rust-sty`.

## Conclusion

The biggest change over this window was not a new witness. It was a broad
improvement in research infrastructure and context quality:

- the harness and benchmark surfaces are more deliberate,
- the search code is much easier to experiment on,
- the corpus is less Brix-Ruiz-only,
- the repo's terminology is cleaner,
- and the newest literature ideas have already produced one concrete reporting
  tool plus one explicit next-step telemetry bead.

For solver progress, the strongest new positive signal is the graph-proposal
shortlist seam. The strongest negative signals are the current
`beam_bfs_handoff` graph-only settings, the current positive-conjugacy proposal
ranking as exact waypoint guidance, and the rechecked `k=4` mixed-beam boundary
above `lag 8`.
