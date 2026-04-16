# Matrix Characterization Classification

## Question

For bead `sse-rust-dhe`, which extra matrix-characterization ideas are actually
useful in the current repo, and where should each one live?

## Context

- `src/invariants.rs` already owns exact endpoint-level arithmetic and spectrum
  screens: trace, determinant, Bowen-Franks, generalized Bowen-Franks, generic
  square power traces, and the `2x2` Eilers-Kiming obstruction.
- `src/path_scoring.rs` and `src/search/beam.rs` already use a
  structure-first score built around duplicate-row/column and quotient-like
  summaries rather than raw endpoint distance.
- `src/graph_moves.rs` and `src/bin/compare_graph_move_proposals.rs` already
  expose a proposal-dossier seam scored by same-future/same-past quotient gap.
- `src/quadratic.rs`, `src/invariants.rs`, and
  `src/bin/profile_gl2z_similarity_2x2.rs` already give the repo a meaningful
  `2x2` arithmetic dossier.
- `src/concrete_shift.rs` and `src/search/shortcut.rs` already provide a small
  certified concrete-shift proof surface for `2x2`.
- `src/conjugacy.rs` is explicitly a proposal/evidence surface, not a proof or
  prune surface.

The practical classification question is therefore not "which ideas sound
mathematically deep?" It is "which ideas improve an exact gate, a pairwise
ranking seam, or only the evidence corpus?"

## Classification

### Keep For Exact Or Theorem-Backed Pruning

- Bounded aligned or concrete-shift witness existence or nonexistence.
  Keep only as a pairwise remaining-lag gate or certified late fallback, not as
  a frontier score. The exact surface already lives in `src/concrete_shift.rs`
  and is consumed from `src/search/shortcut.rs`. The useful repo question is:
  "can this pair admit lag `<= L` on a small certified surface?" not "how much
  aligned flavor does this single node have?"

- Endpoint-only `2x2` arithmetic obstructions and exact positive territory.
  Keep this in the endpoint dossier lane: `src/invariants.rs`,
  `src/quadratic.rs`, `src/search.rs`, and
  `src/bin/profile_gl2z_similarity_2x2.rs`. The negative part is already a hard
  reject through `check_invariants_2x2(...)`. The positive part
  (`GL(2,Z)`-similarity plus Baker or Choe-Shin determinant territory) is worth
  keeping as exact classification or shortcut logic for narrow `2x2` cases, but
  it does not belong in beam ranking.

### Keep For Beam-Ranking Or Proposal-Dossier Experiments

- Same-future or same-past quotient mismatch.
  This is the main "keep" lane. It already matches both the literature note and
  the local evidence. The repo touchpoints are `src/path_scoring.rs`,
  `src/search/beam.rs`, `src/graph_moves.rs`,
  `src/bin/analyze_path_signal_corpus.rs`, and
  `src/bin/compare_graph_move_proposals.rs`. Any stronger characterization that
  is really just a better quotient signature should extend this lane rather than
  create a separate scoring family.

- Partition-refined versions of the current quotient signature.
  Keep, but only as a comparison score or proposal shortlisting experiment. The
  natural landing zones are the same as above: `src/path_scoring.rs` for
  analyzer-only score specs and `src/graph_moves.rs` for proposal gaps. This is
  the cleanest place to spend one more classification round because it respects
  the already-winning structure-first lane instead of replacing it.

- Positive-conjugacy path proximity.
  Keep only in the proposal-dossier lane. The code already says this clearly:
  `src/conjugacy.rs` is an experimental proposal source, and
  `src/structured_surface.rs` labels it that way. If it is used at all, it
  should score local seeds or waypoints against sampled path material in
  `src/bin/evaluate_positive_conjugacy_usefulness.rs`, not alter exact pruning
  and not become a generic endpoint-distance beam term.

### Keep As Evidence-Only Clustering Or Corpus Profiling

- Extra `2x2` arithmetic dossier fields beyond the current hard reject.
  Conductor, maximal-order status, principal-class status, determinant band, and
  the exact `GL(2,Z)` similarity explanation are useful for literature controls,
  case labeling, and hardness clustering. They belong in
  `src/quadratic.rs`, `src/invariants.rs`, and
  `src/bin/profile_gl2z_similarity_2x2.rs`, not in `src/path_scoring.rs`.

- Path-quotient and triangle-collapse structure.
  Keep this as corpus and guide-pool profiling only. It is useful, but it is
  not a matrix characterization in the sense needed for frontier ranking. The
  right home is the already-existing `src/path_quotient.rs`,
  `src/bin/analyze_triangle_path_telemetry.rs`, and
  `src/bin/analyze_guide_pool_quotient.rs` lane.

### Defer Or Drop

- Exact SSE or SE invariants as default beam scores.
  Drop. Inside one fixed positive instance they are constant across any
  genuinely reachable frontier, so they do not rank useful branches. The repo
  should keep using them for endpoint rejection only.

- Raw entrywise endpoint distance, generic matrix norms, NMF-style distances,
  and entropy-like profile scores.
  Drop for now. The existing local evidence already points the other way, and
  the current `src/path_scoring.rs` comparison scores (`endpoint_sig_low`,
  `entry_sum_low`, `max_entry_low`) are already enough negative evidence.

- Relaxed aligned-deficit residuals as a new default beam family.
  Defer. The exact fixed-lag witness question is real, but the current repo only
  has a small certified `2x2` concrete-shift surface. Until there is a cheap
  generic square residual, this is too expensive and too narrow to beat the
  existing structure-first lane.

- Standalone restricted move-graph distances without a specific current move
  family or landmark set.
  Defer. The useful current graph surface is the proposal shortlist in
  `src/graph_moves.rs`, not an abstract new distance metric.

## Durable Placement Summary

- If a characterization is exact and theorem-backed, it belongs in
  `src/invariants.rs`, `src/search.rs`, or the small certified
  `src/concrete_shift.rs` fallback surface.
- If it is pairwise and heuristic, it belongs in `src/path_scoring.rs`,
  `src/graph_moves.rs`, and the existing analyzer binaries first.
- If it explains cases better than it guides search, it belongs in the dossier
  and corpus tools around `src/quadratic.rs`, `src/conjugacy.rs`, and
  `src/path_quotient.rs`.

## Bounded First Experiments

1. Add one analysis-only partition-refined quotient score.
   Wire it into `candidate_score_specs()` and the graph-proposal gap comparison
   surfaces only, then rerun the existing signal-corpus and proposal-shortlist
   analyzers. Do not change `score_node()` or default beam behavior in the same
   round.

2. Add one proposal-dossier export for positive-conjugacy proximity.
   Extend `src/bin/evaluate_positive_conjugacy_usefulness.rs` to report how
   often realized local seeds are close to sampled positive-conjugacy waypoints,
   and compare that against the existing target-distance baselines. Keep it out
   of main search until it wins as a sidecar measure first.

## Conclusion

The repo should keep spending characterization effort on pairwise compatibility
signals, not on more orbit invariants and not on raw endpoint distance.

Concretely:

- exact arithmetic and concrete-shift data stay endpoint-side or certified;
- quotient-style structural mismatch stays the executor-adjacent ranking lane;
- `2x2` arithmetic detail and path-quotient structure stay dossier or corpus
  lanes unless they earn a narrower solver role later.
