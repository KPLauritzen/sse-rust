# Missed or underweighted future-work avenues from the 2026-04-16/17 research notes

## Question

For bead `sse-rust-qdv`, which future-work avenues look plausibly worthwhile
after reading the concrete durable research notes from `2026-04-16` and
`2026-04-17`, but are not clearly being driven by the current live bead set?

This note is intentionally grounded in the underlying research notes first.
The two `last-24h-repo-summary` notes are used only as a secondary cross-check.

## Scope and sources

Primary sources:

- the concrete research notes under `research/notes/2026-04-16-*.md`
- the concrete research notes under `research/notes/2026-04-17-*.md`
- current `bd` state, especially `sse-rust-2uy.*`, `sse-rust-2sp`, and recent
  research follow-up beads tied to those notes

Secondary cross-check only:

- `research/notes/2026-04-16-last-24h-repo-summary.md`
- `research/notes/2026-04-17-last-24h-repo-summary.md`

No `2026-04-18` research notes appeared in this worktree while this survey was
running.

Relevant live bead state while writing:

- `sse-rust-2uy` is still open as the parent backlog, but most recent concrete
  children tied to the surveyed notes are now closed
- `sse-rust-2uy.3` is in progress and still owns the positive-conjugacy follow-up
- `sse-rust-2sp` is in progress and still owns graph-only observer parity for
  dynamic endpoint search
- the recent exact-pruning/orbit-reduction/k4 campaign beads tied to the note
  clusters below are closed rather than active

## Strongest underweighted or missing avenues

### 1. Exact family-specific gates before structured enumeration

This is the clearest promising direction that is visible in the notes but not
clearly owned by a live bead now.

Why it still looks promising:

- [2026-04-17-family-specific-impossibility-gates-before-structured-enumeration.md](2026-04-17-family-specific-impossibility-gates-before-structured-enumeration.md)
  identifies one exact implementation candidate that is both small and local:
  an admissible-divisor emptiness gate for
  `diagonal_refactorization_3x3`/`4x4`
- the same note explicitly says this is the "best next exact implementation
  candidate", while the exact row/column mass gates for `3x3 -> 4x4` split
  families are correct but lower leverage
- [2026-04-17-bounded-no-go-certificates-first-round.md](2026-04-17-bounded-no-go-certificates-first-round.md)
  independently points to explicit structured-family envelopes as one of the
  strongest exact bounded-certificate directions
- [2026-04-17-exact-pruning-obstruction-literature-survey.md](2026-04-17-exact-pruning-obstruction-literature-survey.md)
  reaches the same conclusion from the literature side: exact structured
  move-family restriction is one of the three most defensible exact-pruning
  directions

Why I think this is underweighted rather than already covered:

- the research beads that established this direction, `sse-rust-ilu.1`,
  `sse-rust-ilu.3`, and `sse-rust-ilu.4`, are all closed
- I did not find a live follow-up bead explicitly owning "implement the exact
  diagonal-refactorization admissible-divisor gate" or "promote exact
  family-local gates into the search"
- the live `2uy` children are centered elsewhere: positive conjugacy,
  previously landed structured families, deepening, and already-landed generic
  square endpoint parity

Recommended bead action:

- create or reframe a bead around **exact family-local pre-enumeration gates**,
  with the first slice narrowly scoped to the diagonal-refactorization
  admissible-divisor gate
- keep row/column mass gates as optional secondary follow-ups rather than the
  headline task

### 2. Systematic exact orbit reduction, not just one-off local seams

The notes make a stronger case for exact orbit reduction than the current bead
set suggests.

Why it still looks promising:

- [2026-04-17-early-canonical-dedup-square-factorisation-orbits.md](2026-04-17-early-canonical-dedup-square-factorisation-orbits.md)
  kept an exact local seam where heavy `square_factorisation_3x3` hotspot rows
  collapse sharply before canonical successor materialization, for example
  `raw_sq=3737 -> raw_orbit=802 -> raw_canon=450`
- [2026-04-17-binary-sparse-3x3-to-4-orbit-representative-seam.md](2026-04-17-binary-sparse-3x3-to-4-orbit-representative-seam.md)
  found a second exact family-specific orbit seam, with similar collapse such
  as `raw=2448 -> orbit=87 -> canon=65`, and makes the general point that the
  real reusable idea is **family-specific exact orbit recognition**
- [2026-04-17-exact-pruning-obstruction-literature-survey.md](2026-04-17-exact-pruning-obstruction-literature-survey.md)
  names exact orbit reduction and canonical augmentation as one of the three
  most promising literature-backed directions
- [2026-04-17-bounded-no-go-certificates-first-round.md](2026-04-17-bounded-no-go-certificates-first-round.md)
  explicitly lists bounded exhaustive-search certificates modulo exact orbit
  reduction as a promising exact direction

Why I think this is underweighted rather than already covered:

- `sse-rust-8le` and `sse-rust-veu` landed two concrete orbit seams, but both
  are closed and local
- I did not find an active umbrella bead whose job is to identify the next hot
  structured families with exact orbit quotients, or to connect orbit
  reduction to bounded no-go certificates
- the underlying notes suggest a broader pattern than the current bead layout
  reflects

Recommended bead action:

- create or reframe a bead around **exact orbit reduction for structured
  families**, with two explicit goals:
  1. frontier-local savings on hot structured enumerators
  2. exact bounded-exhaustion or no-go arguments modulo those same orbit keys

### 3. Goal 3 follow-up should pivot to `graph_plus_structured` dim-4 profiling/ranking, not more generic `k=4` widening

This is the strongest solver-facing avenue that looks underowned right now.

Why it still looks promising:

- [2026-04-17-brix-ruiz-k4-graph-plus-structured-campaign.md](2026-04-17-brix-ruiz-k4-graph-plus-structured-campaign.md)
  says the only justified next step is a narrow profiling or ranking-quality
  round on the bounded `dim4` surface
- [2026-04-17-brix-ruiz-k4-graph-plus-structured-broad-beam.md](2026-04-17-brix-ruiz-k4-graph-plus-structured-broad-beam.md)
  materially strengthens that case: `beam64` was too narrow on the `dim4`
  lane, while `beam256 + dim4 + entry12` remains bounded and shows much larger
  approximate-hit growth
- the same broad-beam note is also clear about what **not** to do:
  the first richer `dim5` surface is still a factorisation-volume cliff
- this makes the next rational Goal 3 slice much narrower than "run another
  big k=4 campaign"

Why I think this is underweighted rather than already covered:

- the directly relevant beads, `sse-rust-2uy.24` and `sse-rust-dqc`, are both
  closed
- `sse-rust-2uy.21` closed with "evidence only" for graph-only and explicitly
  did not open a follow-up
- I did not find a live bead that owns the narrower recommendation from the raw
  notes: **profile or improve ranking quality on the `beam256 + dim4 + entry12`
  graph-plus-structured surface**

Recommended bead action:

- if Goal 3 is reopened soon, create a bead explicitly scoped to
  `graph_plus_structured` `k=4` **dim-4** profiling/ranking work using the
  `beam256 + dim4 + entry12` surface
- do not frame it as another generic reach campaign, and do not send it back
  to graph-only deep-beam or graph-plus-structured dim-5 widening first

## Lower-priority but still somewhat buried

### Narrow exact `2x2` positive shortcut promotion

This direction is not missing, but it may be slightly under-scoped.

Evidence:

- [2026-04-16-missing-references-and-solver-ideas.md](2026-04-16-missing-references-and-solver-ideas.md)
  proposed a positive-only `2x2` dossier/shortcut around `GL(2,Z)` similarity
  plus Baker and Choe-Shin determinant bands
- [2026-04-17-exact-pruning-obstruction-literature-survey.md](2026-04-17-exact-pruning-obstruction-literature-survey.md)
  still treats narrow exact `2x2` arithmetic/similarity dossiers as one of the
  strongest defensible exact directions
- `sse-rust-sty` landed the reporting surface
- `sse-rust-vn0` then closed the stronger ideal-quotient follow-up as a dead
  end

Current read:

- this lane is not missing in the way the three stronger avenues above are
- but it is still only at the reporting/dossier stage, with no live bead that
  would test a careful positive-only shortcut promotion

Recommended bead action:

- only reopen this if a worker specifically wants a narrow exact-positive
  `2x2` acceptor or better source-backed positive control cases
- do not treat it as a main Goal 2 or Goal 3 lever

## Already covered by live or clearly owned work

### Sampled positive conjugacy remains actively covered

The note evidence does not support calling this missed.

Evidence:

- [2026-04-17-positive-conjugacy-local-seed-family-followup.md](2026-04-17-positive-conjugacy-local-seed-family-followup.md)
- [2026-04-17-positive-conjugacy-invariant-reprojection-followup.md](2026-04-17-positive-conjugacy-invariant-reprojection-followup.md)
- [2026-04-17-positive-conjugacy-anchor-aware-residual-ranking.md](2026-04-17-positive-conjugacy-anchor-aware-residual-ranking.md)

All three say the current variants are still negative but leave a concrete next
slice: richer residual difficulty signals on the surviving exact local seed
family.

Matching bead state:

- `sse-rust-2uy.3` is in progress and already says essentially that

### Graph-only observer parity for dynamic endpoint analysis is actively covered

Evidence:

- [2026-04-16-dynamic-mixed-endpoint-layer-events.md](2026-04-16-dynamic-mixed-endpoint-layer-events.md)
- [2026-04-17-non-brix-layer-contrast-rerun-after-mixed-observer-fix.md](2026-04-17-non-brix-layer-contrast-rerun-after-mixed-observer-fix.md)

The open gap is now specifically graph-only parity for dynamic endpoint search,
not a missing planning idea.

Matching bead state:

- `sse-rust-2sp` is in progress and already owns that follow-up

## Directions the recent notes already rejected or sharply de-prioritized

### 1. More cap-only `beam_bfs_handoff` tuning on the graph-only `k=3` control

Rejected by:

- [2026-04-16-beam-bfs-handoff-cap-sweep-graph-only-k3.md](2026-04-16-beam-bfs-handoff-cap-sweep-graph-only-k3.md)
- [2026-04-16-beam-bfs-handoff-subbeam-cap-sweep-graph-only-k3.md](2026-04-16-beam-bfs-handoff-subbeam-cap-sweep-graph-only-k3.md)

Both cap scales stayed worse than plain beam, and even `deferred_cap = 0` did
not rescue the surface.

### 2. Treating the current graph-proposal shortlist seam as a general frontier win

Rejected by:

- [2026-04-17-graph-proposal-shortlist-beyond-one-seam.md](2026-04-17-graph-proposal-shortlist-beyond-one-seam.md)

The one good seam remained a one-off curiosity; the broader controls collapsed
back to blind one-step behavior.

### 3. Promoting quotient/support signatures into exact `square_factorisation_3x3` hard gates

Rejected by:

- [2026-04-17-square-factorisation-3x3-quotient-support-obstruction-followup.md](2026-04-17-square-factorisation-3x3-quotient-support-obstruction-followup.md)
- [2026-04-17-family-specific-impossibility-gates-before-structured-enumeration.md](2026-04-17-family-specific-impossibility-gates-before-structured-enumeration.md)

These signatures remain useful for ordering and orbit reduction, but not as
exact source-only emptiness certificates.

### 4. Reopening the recent balanced local variants at the same bounded caps

Rejected or sharply de-prioritized by the sequence:

- [2026-04-16-balanced-neighbor-zigzag-first-slice.md](2026-04-16-balanced-neighbor-zigzag-first-slice.md)
- [2026-04-16-balanced-outsplit-bridge-neighbor-seam.md](2026-04-16-balanced-outsplit-bridge-neighbor-seam.md)
- [2026-04-16-balanced-bridge-return-seam.md](2026-04-16-balanced-bridge-return-seam.md)
- [2026-04-16-balanced-insplit-return-seam.md](2026-04-16-balanced-insplit-return-seam.md)
- [2026-04-16-balanced-insplit-source-return-seam.md](2026-04-16-balanced-insplit-source-return-seam.md)

These notes repeatedly say the current bounded local balanced surfaces stay
empty on `brix_k3`/`k4`. If balanced work is reopened later, it should be on a
materially different bridge source or bridge-return shape, not as another near
copy of these caps.

### 5. The Eilers-Kiming ideal-quotient follow-up as a stronger exact pairwise screen

Rejected by:

- [2026-04-16-2x2-ideal-quotient-followup.md](2026-04-16-2x2-ideal-quotient-followup.md)

The quotient consequence is weaker than the current exact ideal-class
comparison, not stronger.

## Bottom line

The strongest missed or underweighted avenues are not broad new theories.
They are three narrow follow-ups that the raw notes support repeatedly:

1. exact family-specific gates before structured enumeration, especially the
   diagonal-refactorization admissible-divisor gate
2. systematic exact orbit reduction across structured families, and possibly
   bounded no-go certificates modulo those orbit quotients
3. a narrow `graph_plus_structured` `k=4` dim-4 profiling/ranking round on the
   `beam256 + dim4 + entry12` surface

By contrast, the recent notes already make several tempting alternatives look
spent: more `beam_bfs_handoff` cap tuning, treating the current proposal
shortlist as broadly predictive, quotient/support hard gates for
`square_factorisation_3x3`, repeated local balanced seam tweaks, and the
ideal-quotient arithmetic follow-up.
