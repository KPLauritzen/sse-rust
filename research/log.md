# Research Log

## 2026-04-05

- `7dea2f2` Baseline run.
  Established the initial harness score and runtime floor for the autoresearch loop.

- `7dea2f2` Allowed negative-determinant square `2x2` factors.
  Failed. It widened the search without producing better target behavior and only slowed the baseline.

- `7dea2f2` Deduplicated canonical successors within each frontier-node expansion.
  Kept. This removed obvious duplicate work without changing search behavior.

- `7dea2f2` Continued exploring after one bidirectional frontier emptied.
  Failed. It spent more time in unproductive tail search and did not improve any hard case.

- `7dea2f2` Combined deeper frontier exploration with negative-determinant square factors.
  Failed. It stacked two widening ideas that both increased work without creating a target hit.

- `7dea2f2` Revalidated the canonical-successor dedup on a fresh run.
  Kept. Same score, slightly lower runtime.

- `6d60073` Deduplicated canonical successors across the full frontier layer.
  Kept. This cut duplicate cross-node work and improved warm runtime.

- `e16edbf` Added bounded `3x3` same-size moves via cached identity splits.
  Failed. It caused a large factorisation blow-up without opening new useful Brix-Ruiz states.

- `e16edbf` Deduplicated factorisation outputs by `VU` in the dispatcher.
  Failed. The extra checking cost outweighed the reduced duplicate outputs.

- `1672ada` Specialized small permutation canonicalization in the hot loop.
  Kept. Pure speedup with no change to search behavior.

- `1672ada` Optimized small-matrix multiplication with row slicing and zero-skips.
  Kept. Another small runtime win with unchanged results.

- `c86b908` Added restricted `3x3` same-size shear factorizations.
  Failed. It explored deeper but not in a way that improved targets, so runtime got worse.

- `c86b908` Combined shear moves with explicit forward/backward depth budgeting.
  Failed. Better exploration shape was not enough; it still missed the hard cases and slowed down.

- `c86b908` Removed heap allocation from specialized canonicalization paths.
  Kept. Small runtime win, no behavioral change.

- `6070a45` Added explicit tiny-shape multiplication specializations.
  Failed. The extra specialization overhead did not beat the current fast path.

- `6070a45` Parallelized the expensive `2x3` rectangular factorisation enumerator.
  Kept. Straight runtime win on native builds.

- `1eb69df` Parallelized the `3x3 -> 2` rectangular factorisation enumerator.
  Kept. Another runtime improvement with identical correctness.

- `4e568d7` Parallelized the square `2x2` factorisation enumerator too.
  Failed. The unit of work was too small, so parallel overhead dominated.

- `4e568d7` Skipped outer rayon frontier scheduling for single-node layers.
  Failed. It was neutral logically but slightly slower in practice.

- `67f679a` Streamed factorisation visitation directly into BFS expansion.
  Kept. Reduced intermediate allocation and improved runtime.

## 2026-04-05 Sidecar Split Work

- `763c18e` Added bounded balanced-elementary witness search as a sidecar.
  Kept as evidence. It did not solve the hard cases, but it ruled out cheap direct balanced-elementary witnesses.

- `1c83ead` Added explicit out-split sidecar probes.
  Kept as evidence. No common one-step `3x3` refinement or simple bridge appeared for the Brix-Ruiz pairs.

- `0e933f2` Generalized to two-step out-split refinements and bridge zig-zags.
  Kept as evidence. The widened split universe still stayed disconnected across the two sides.

- `563101a` Added bounded `3x3 -> 2x2 -> 3x3` zig-zag sidecar probes.
  Kept as evidence. The canonical `3x3` start sets looked closed under this move, so it did not create a meeting path.

- `f1268a3` Added dual in-split sidecar probes via transposition.
  Kept as evidence. This genuinely enlarged the move family, but still found no common refinement.

- `e8076c2` Added two-step mixed out/in split refinement probes.
  Kept as evidence. Even the enlarged mixed split sets remained disjoint.

## 2026-04-05 Structured `3x3` Moves

- `80ac7ec` Added explicit diagonal `3x3` conjugation moves above the square-factorisation cap.
  Failed. It stayed correct but pushed the search in a less informative direction and hurt focus telemetry.

- `e1be8ea` Replaced smaller-frontier alternation with representative-sample factorisation-cost balancing.
  Kept. It improved the search schedule on the wide Brix-Ruiz probe.

- `abc9eb5` Added opposite-shear `3x3` conjugation moves.
  Kept. This opened a slightly better `3x3` search surface and improved focus telemetry.

- `20af1b7` Added paired-shear conjugation moves with a shared pivot.
  Kept. It enlarged the useful `3x3` surface again and improved focus telemetry.

- `20af1b7` Added the complementary shared-target paired-shear family.
  Kept. This was the best `3x3` structured-move improvement in that sequence.

## 2026-04-05 to 2026-04-06 Matrix-Level / Proposal Work

- `857637a` Integrated concrete-shift fallback into the main search.
  Kept. It made matrix-level concrete-shift available as a direct proof fallback, even though it did not fire on the benchmark targets.

- `857637a` Added a bounded concrete-shift-guided successor ordering pass.
  Failed. It only changed local ordering inside a layer-synchronous BFS, so it added cost without changing the hard frontier shape.

- `857637a` Added exact `3x3 -> 2x2 -> 3x3` out-split zig-zag proposal edges to the main search.
  Failed. It mostly repackaged paths the solver could already simulate, so telemetry stayed flat and runtime regressed.

- `857637a` Added explicit same-future in-split proposals to the main search.
  Kept. This was the first refined split family that produced a small but real telemetry improvement on the hard family.

- `4879cff` Added the same-past dual as explicit out-split proposals.
  Kept. Another small improvement, again mainly on the `k=4` family probe.

- `2c87aaa` Added a tightly bounded visible-coincidence `3x3 -> 3x3` refined split family.
  Failed. It preserved correctness but mostly duplicated existing `3x3` successors, so the hard-case telemetry stayed flat.

## 2026-04-16 Balanced Sidecar Follow-up

- `bc39307` Added a bounded `3x3 -> 2x2 <-balanced-> 2x2 -> 3x3` return-seam probe.
  Kept as evidence. It turns the toy bridge hit into a genuine bounded `3x3`
  move, but both Brix-Ruiz controls still stay disconnected at the same return
  seam.

- `47462de` Added a bounded `3x3(out) -> 2x2 <-balanced-> 2x2 -> 3x3(in)` return seam.
  Kept as evidence. The mixed in-split return family is real on the toy control
  with the same bounded bridge hop, but bounded `brix_ruiz_k3` and `k4` still
  stay disconnected at the same caps.

- `f2032dc` Added a bounded `3x3(in) -> 2x2 <-balanced-> 2x2 -> 3x3(out)` source seam.
  Kept as evidence. Swapping the bounded bridge-source family from out-splits to
  in-splits stays toy-positive with the same tiny balanced bridge hop, but
  bounded `brix_ruiz_k3` and `k4` still stay disconnected at the same caps.

## 2026-04-16 Arithmetic Follow-up

- `working tree` Checked the deferred `2x2` ideal-quotient / colon-ideal follow-up.
  Kept as evidence. Eilers-Kiming Theorem 1 reduces the colon-ideal equation to
  class congruence only modulo primes dividing `lambda`, so it is weaker than
  the repo's current exact quadratic-order ideal-class comparison and did not
  justify a new hard rejection.

## 2026-04-15

- `working tree` Added an explicit graph-proposal research seam in `src/graph_moves.rs`.
  Kept as evidence. Raw same-future/same-past/zig-zag proposal families are broader than blind one-step graph expansion, but quotient-signature scoring collapses them to tiny shortlists; on the `brix_ruiz_k3` `guide:1 -> guide:15` probe, the best shortlisted zig-zag proposal beats every blind one-step successor on the same quotient score.

- `2c87aaa` Added a global factorisation-result cache keyed by matrix and bound.
  Failed. It reused exact enumerations but the synchronization and cloning cost regressed harness runtime without changing the hard cases.

## 2026-04-06

- `c2e8b68` Penalized dead-end-heavy frontier alternation.
  Failed. It cut the hard-family search off a layer earlier, lowered focus telemetry, and still produced zero overlap hits.

- `386fe26` Coarsened overlap signatures away from exact support masks.
  Kept. It preserved the reach score while producing the first nonzero approximate overlaps on the `k=3` hard probes.

- `74683df` Chased approximate overlaps from the opposite frontier.
  Failed. It improved directed telemetry and runtime, but it exhausted the `k=3` probes a layer earlier and lost too much reach.

- `e773b86` Probed `3x3 -> 2x2 <- 3x3` rectangular bridges after overlap hits.
  Failed. None of the coarse-overlap pairs admitted the hoped-for bounded bridge, so behavior stayed flat and runtime ticked up.

- `c87a786` Added scale-insensitive overlap signatures.
  Failed. Normalizing the row and column profiles by scale did not produce any new hard-case continuity signal.

- `071f236` Added histogram-based overlap signatures.
  Failed. The signature was too coarse, produced misleading overlap pressure, and collapsed the hard probes a layer earlier.

## 2026-04-12

- `worktree` Re-ran the endpoint-guided shortcut search at `max_dim=4`, `max_entry=5`.
  Kept as evidence. The existing mixed shortcut search compresses the blind 16-move graph path to an 11-step SSE path, so the current bottleneck is no longer "can we beat graph-only at all?" but "how do we recover the missing short structured moves?"

- `worktree` Checked Lind-Marcus/Baker step coverage against the current move families.
  Kept as evidence. Steps 2, 5, and 6 are still missing; a failed generic `4x4` shear extension confirmed that the real gap is not same-size conjugation but the missing `3x4` / `4x3` structured vocabulary.

- `worktree` Added a binary-sparse `4x4 -> 3x3` rectangular factor family.
  Kept. It now recovers Baker step 6 and the hidden `3x3` bridge inside Baker step 5, but step 2 is still missing and the default shortcut search still bottoms out at total lag 11.

- `worktree` Added the dual structured `3x3 -> 4x4` rectangular factor family.
  Kept. It now recovers Baker step 2 directly while keeping the default shortcut search flat at total lag 11, so the remaining literal gap is step 5 itself rather than the `3x4`/`4x3` boundary moves around it.

- `search-k3-graph-paths` Wired the shortcut search to read/write sqlite guide paths.
  The `find_brix_ruiz_path_shortcuts` binary now accepts `--paths-db` to load graph-move
  paths and prior shortcut results from the sqlite database, and writes improved shortcut
  paths back. This creates a feedback loop: graph-path explorer → shortcut search → next
  shortcut search.

- `search-k3-graph-paths` Added `--segment-timeout` to prevent OOM in shortcut search.
  The BFS can consume 27+ GB on large-gap segments (2x2→2x2 with gap ≥ 8–9). A per-segment
  timeout with 256-node frontier chunking now aborts before memory pressure becomes critical.

- `search-k3-graph-paths` First sqlite-seeded shortcut run (`--segment-timeout 10`, 7 guides).
  Kept. The feedback loop works: starting from 2 graph paths (16 moves each) and 5 prior
  shortcut results, the search found two independent 7-move SSE paths for `brix_ruiz_k3`.
  This is a significant improvement over the 11-step result from the endpoint-only search
  and matches the known Baker lag-7 witness length. The best routes pass through 4x4 and 3x3
  intermediates with max entry 6. All improved results were persisted to the sqlite database
  (9 total result rows, 2 at lag 7).

- `new-moves-3` Added binary-sparse 4x4↔5x5 rectangular factorisation families.
  Two new families enable short visits to dimension 5 from 4x4 nodes and back:
  5x5→4x4 (U has binary-sparse rows, 10K outer iterations per node) and 4x4→5x5
  (U has 4 binary-sparse columns + 1 distinguished weighted column). Both gated on
  `max_intermediate_dim ≥ 5`. Supporting infrastructure: `solve_nonneg_4x4` via
  Cramer's rule with rank-3 fallback, `solve_overdetermined_5x4`, and length-4
  binary-sparse/weighted row helpers. Quick shortcut search confirms the families
  fire and produce shortcuts (e.g. 2x2→5x5 gap=5 solved in 262 visited nodes).

- `2026-04-12-1646-factor-solver-memo` Cached full `2x3` and `3x2` solver outputs.
  Failed. The added lookup and cloning cost pushed all three telemetry-focus mixed probes into timeout.

- `2026-04-12-1650-unsat-solver-memo` Cached only repeated unsatisfiable `2x3` and `3x2` solver calls.
  Failed. Lighter than the full cache, but still regressed enough to timeout `brix_ruiz_k3_wide_probe`.

- `2026-04-12-1656-row-candidate-cache` Cached valid `U` row candidates by target row and reused them for both outer and inner rectangular loops.
  Kept. It preserved the full baseline score while cutting saved-artifact runtime from `26522 ms` to `20277 ms`.

## 2026-04-13

- `worktree` Surveyed Boyle-Kim-Roush, Bilich-Dor-On-Ruiz, Eilers-Ruiz, Eilers-Kiming, Brix, and Brix-Ruiz for bounded solver ideas.
  Ranked six concrete experiments in `research/notes/2026-04-13-solver-literature-ideas.md`, with quotient-state compression and narrow diagonal-refactorization at the top because they fit the recent cache win/failure pattern and the current frontier-growth bottleneck.

- `main-search-graph-hash` Swapped the main `src/search.rs` visited/frontier maps to `AHashMap`/`AHashSet`.
  Kept. Profiling the in-harness graph-only `brix_ruiz_k3` case showed large-state bookkeeping dominating runtime; the faster hash tables cut that case from about `9.6s` to `8.75s` and reduced total harness time from `11.37s` to `10.51s` with identical outcomes.

- `2026-04-13-same-future-past-graph-reps` Added same-future/past quotient signatures and used them for layer-local graph representative selection.
  Kept. Mixed probes were unchanged, but `brix_ruiz_k3_graph_only` collapsed `431401` graph successors under the new quotient and the saved harness run dropped from `12827 ms` to `12563 ms` overall.

- `2026-04-13-0403-drop-spectrum-prune` Removed redundant mid-search spectrum checks from the main expansion loops after `pprof` showed the hot path was spending time in candidate screening that never actually pruned.
  Kept. The saved harness artifact stayed identical on score and outcomes while dropping total runtime from `14658 ms` to `10601 ms`; `brix_ruiz_k3_graph_only` fell to `9030 ms` and mixed `brix_ruiz_k3` to `458 ms`.

- `guided-segment-timeout` Added generic per-segment timeout support to guided refinement.
  Kept. `GuidedRefinementConfig` and the `search` CLI now expose an optional `segment_timeout_secs` / `--guided-segment-timeout`, and the generic guided-refinement segment search enforces it with 256-node frontier chunk checks so one hard shortcut attempt no longer monopolizes a run.

- `profile-mixed-brix-k3` Added gated `pprof` / `dhat` hooks to the `search` CLI and profiled the mixed `brix_ruiz_k3` endpoint run directly.
  `pprof` concentrated on `visit_all_factorisations_with_family`, especially `enumerate_sq3_from_row0` and `solve_nonneg_2x3`, while reduced-lag `dhat` still allocated about `200 MB` across `2.58M` blocks by layer 3, so the next wins should come from cutting factorisation-side allocation churn rather than graph heuristics.

- `2026-04-13-1455-solver-buffer-reuse` Reused `solve_nonneg_2x3` output buffers inside the hot `3x3` square-factorisation loop instead of allocating fresh vectors every call.
  Kept. The second saved harness rerun dropped total elapsed from `10858 ms` to `10593 ms` with identical scores, while the mixed `brix_ruiz_k3`, `brix_ruiz_k3_wide_probe`, and `brix_ruiz_k4_probe` cases all got faster.

- `2026-04-13-1516-solver-buffer-reuse-level2` Tried the same buffer-reuse pattern for `solve_nonneg_3x3` and the row-2 solution loop inside `enumerate_sq3_from_row0`.
  Failed. The saved run regressed total elapsed from `10593 ms` to `10886 ms`, and the mixed `brix_ruiz_k3`, `wide_probe`, and `k4_probe` cases all slowed back down, so the extra bookkeeping did not pay for itself.

- `2026-04-13-1530-canonical-self-fastpath` Returned `self.clone()` from the `canonical_perm` fast paths when the matrix was already canonical.
  Failed. The saved run regressed total elapsed from `10593 ms` to `10706 ms`; `brix_ruiz_k4` improved slightly, but the harder mixed `k=3` probes and the graph-only probe all got slower overall.

- `2026-04-13-profile-square-sources` Added `profile_square_factorisation_sources` and `profile_sq3_breakdown` to inspect which `3x3` sources drive `square_factorisation_3x3` traffic and where the local enumerator spends its effort.
  Kept as profiling support. The mixed `brix_ruiz_k3` hard case showed `square_factorisation_3x3` dominating with about `320k` generated candidates but only `~2.7k` post-pruning survivors, while the source-bucket breakdown showed duplicate-row and duplicate-column `3x3` states dominating raw callback volume and the per-source enumerator breakdown showed most row-1 candidates die on the very first `2x3` column solve.

- `2026-04-13-1455-exact-vu-dedup` Deduplicated exact `VU` factorisation outputs before canonicalization in the mixed frontier expander.
  Failed. Although the profiling helpers showed large raw-to-unique collapse for some hot `3x3` sources, the saved harness rerun stayed inside noise on the mixed probes and landed at `10624 ms` total, so the extra hash checks did not buy a real win.

- `2026-04-13-1505-sq3-symmetry-break` Tried symmetry breaking in the square `3x3` enumerator for sources with equal rows or columns.
  Failed. It reduced raw `square_factorisation_3x3` counts on the hard mixed probes, but runtime regressed badly (`12411 ms` total, with all three mixed telemetry-focus probes slower), so the added ordering checks were not worth it.

## 2026-04-14

- `worktree` Added `assemble_k3_guide_pool` and materialized a normalized k=3 guide envelope from fixture + sqlite + in-repo Lind-Marcus/Baker witness sources.
  Kept as workflow support. `research/guide_artifacts/k3_normalized_guide_pool.json` now contains 12 validated full-path guides with best lag 7, suitable for repeatable generic `shortcut_search` runs.

- `worktree` Ran iterative generic `shortcut_search` over the normalized k=3 pool with staged bound increases.
  Kept as evidence. Completed runs at 48 and 128 segment-attempt caps both held best lag at 7 and converged to the same witness signature as the Lind-Marcus/Baker guide; a 256-attempt run timed out at 420s. Details are in `research/notes/2026-04-14-k3-normalized-guide-pool-shortcutting.md`.

- `worktree` Tested a gap-prioritized segment-attempt ordering for guided shortcut refinement on the hard k=3 pair.
  Failed. Under the same 128-attempt shortcut budget, lag stayed at 7 with unchanged segment-improvement and promotion counts, while frontier work increased (`frontier_nodes_expanded 20088 -> 21665`, `total_visited_nodes 1263782 -> 1394505`), so the heuristic was reverted. Details are in `research/notes/2026-04-14-k3-shortcut-gap-priority-ordering.md`.

- `worktree` Ran a gap-focus A/B for k=3 shortcut search (`min_gap=2,max_gap=6` control vs `min_gap=3,max_gap=7` focused).
  Failed. The focused policy did not improve lag (still 7), timed out at 128 attempts, and at 64 attempts increased search work despite slightly higher local improvement counts (`frontier_nodes_expanded 10882 -> 16592`, `total_visited_nodes 712040 -> 1135613`). Details are in `research/notes/2026-04-14-k3-shortcut-gap-focus-ab.md`.

- `loop3-segment-cache` Added per-run segment-query memoization for `shortcut_search` guided segments with cache hit/miss telemetry.
  Kept. Profiling still shows factorisation-dominated segment cost, and memoization produced concrete reuse on hard `k=3` runs (`22` hits at control-128, `38` hits at focused-128). Control-128 kept lag 7 but reduced work (`frontier_nodes_expanded 20088 -> 17918`, `total_visited_nodes 1263782 -> 1121478`), and the previously timeout-prone focused-128 run completed under the same 300s cap (still lag 7). Details are in `research/notes/2026-04-14-k3-shortcut-segment-cache.md`.

- `loop4-budget-ramp` Ramped cache-enabled shortcut attempt budgets from `128` to `192` and `256`, then probed `guided_max_shortcut_lag=5`.
  Mixed. Larger budgets now complete and increase local segment improvements (`11 -> 24 -> 41`) with substantial cache reuse, but best lag stayed at 7. The lag-cap-5 probe timed out at 300s without a completed artifact, so broadening per-segment depth appears too costly. Details are in `research/notes/2026-04-14-k3-shortcut-cache-budget-ramp.md`.

- `loop5-cache-followups` Tested two post-cache semantic tweaks: miss-only attempt budgeting and endpoint-level lag-dominance cache reuse.
  Failed. Both variants caused the core control run (attempts 128, min_gap=2/max_gap=6) to time out at 300s and were reverted. Details are in `research/notes/2026-04-14-k3-shortcut-cache-followups-reverted.md`.

- `loop6-fair-share-budgeting` Tried deterministic per-guide fair-share segment-attempt allocation within each shortcut round.
  Failed and reverted. It preserved required-case correctness on rerun but regressed harness telemetry-focus score (`45802619 -> 42689216`) with no lag improvement, so the code change was dropped.

- `loop7-guide-count-sweep` Ran a measurement-first `shortcut_max_guides` sweep to test budget concentration.
  Kept as evidence only. On the hard `dim5/entry6` surface, attempts-64 timed out for guides `4/8/12` (no completed artifacts). On a stable `dim4/entry5` surface, guides=4 was slightly cheaper at attempts=96, while guides=12 produced more local improvements at attempts=128; all variants stayed at lag 7. Details are in `research/notes/2026-04-14-k3-shortcut-guide-count-sweep.md`.

- `loop8-round-budget-split` Tried splitting `shortcut_search` segment attempts across rounds to force promoted-guide follow-up.
  Failed and reverted. The patch preserved required-case correctness but on targeted `dim4/entry5` probes reduced local shortcut-improvement throughput without improving lag, and it still timed out on the hard `dim5/entry6` attempts-64 probe. Details are in `research/notes/2026-04-14-k3-shortcut-round-budget-split-reverted.md`.

- `loop9-guided-segment-timeout-sweep` Swept `guided_segment_timeout_secs` on the hard `dim5/entry6` shortcut surface.
  Kept as evidence only. Attempts-64 still timed out for timeout values `1/2/3/5` (no completed JSON). At lower budgets (`8/16/32`), segment timeouts improved tractability and reduced work but yielded zero segment improvements and no lag gain (stayed at 7). Details are in `research/notes/2026-04-14-k3-shortcut-guided-segment-timeout-sweep.md`.

- `loop10-hard-budget-cliff` Swept hard dim5 shortcut attempts under `max_entry=5` and compared against `max_entry=6` at the stable attempt budget.
  Kept as evidence only. Under a 180s cap, `max_entry=5` still timed out at attempts `36+` and only attempts `32` completed (lag 7, one local improvement). The `max_entry=6` attempts-32 comparison also stayed at lag 7 with one improvement, so this bound retune reduces work but does not move the plateau. Details are in `research/notes/2026-04-14-k3-shortcut-hard-budget-cliff.md`.

- `loop11-trace-cube-invariant` Tested and reverted a dynamic `trace(M^3)` prefilter in endpoint search.
  Failed (neutral/negative). Targeted dim4 and hard dim5 shortcut probes were unchanged on lag/work where completed, and the first hard probe at the prior 180s outer cap timed out with no artifact. The patch was reverted to avoid stacking complexity without bottleneck movement. Details are in `research/notes/2026-04-14-k3-shortcut-trace-cube-invariant-reverted.md`.

- `loop12-gap-window-campaign` Measured hard dim5 shortcut behavior under alternative gap windows and pushed a high-budget narrow-gap campaign.
  Kept as evidence only. Narrowing to `max_gap=4` massively improved tractability and enabled attempts up to `512` with low work growth, but all runs still converged to lag `7`. Widening to `min_gap=4` raised local improvements but was far more expensive at the same budget. Details are in `research/notes/2026-04-14-k3-shortcut-gap-window-campaign.md`.

- `loop13-staged-refinement-followup` Tested whether a `max_gap=4` stage-1 best path can be compressed further by a stage-2 full-gap pass.
  Failed. The stage-2 run on the exported lag-7 stage-1 best guide (`max_gap=6`, lag-cap 4) produced no improvements before pool exhaustion, and raising lag-cap to 5 timed out under 240s for attempts 32 and 64. Logged in `research/notes/2026-04-14-k3-shortcut-gap-window-campaign.md`.

- `loop14-profiled-dim4-lagcap5-campaign` Re-profiled the hard shortcut surface and ran a dim4/lag-cap-5 attempt expansion campaign with bounded binary-timeout execution.
  Kept as evidence only. `pprof` on the hard dim5 control remained factorisation-dominated (notably `visit_binary_sparse_factorisations_4x4_to_5` / `solve_nonneg_4x4`). Switching to dim4 made large lag-cap-5 runs tractable (including attempts 512 and 2048) and produced many local guide improvements, but every run still converged to lag 7. Narrow gap (`max_gap=4`) was extremely cheap and saturated to `guide_pool_exhausted` without global lag gain. Details are in `research/notes/2026-04-14-k3-shortcut-dim4-lagcap5-profiled-campaign.md`.

- `process-hygiene-caveat` Added post-hoc caveat notes to April 14 shortcut experiment writeups after discovering lingering `search` processes from earlier `timeout cargo run ...` probes.
  Kept as documentation correction. Timing/throughput comparisons in affected notes are now marked provisional unless rerun under strict `timeout -k ... target/dist/search` execution; lag/outcome classifications remain the primary correctness signal.

- `loop15-dim4-feeder-diversity-ab` Tested whether adding a dim4-feeder lag-7 artifact changes hard dim5 lag-cap-5 shortcut behavior.
  Failed (neutral). In a controlled dim5 stage-2 A/B (`attempts=64`, `guided_segment_timeout=5`), adding the feeder artifact increased loaded guides (`12 -> 13`) but not unique guides (`12 -> 12`), and lag/work outcomes remained effectively unchanged (still lag 7, same guided improvements/promotions). Details are in `research/notes/2026-04-14-k3-shortcut-diversity-feed-stage2-ab.md`.

- `loop16-equivalent-lag-dominance-cache` Added safe segment-cache dominance reuse for equivalent results across lag caps in `shortcut_search`.
  Kept. The cache now reuses shortest known equivalent `(source,target)` paths when `path_lag <= requested_max_lag` (exact-key cache still checked first; no Unknown dominance reuse). On the dim4 lagcap5 full-gap probe (`attempts=512`), this deterministically improved cache reuse (`hits 151 -> 164`, `misses 361 -> 348`) and reduced work (`factorisations 33,957,312 -> 33,499,328`, `visited 6,486,917 -> 6,472,560`) with unchanged lag (`7`). Harness gate stayed stable at `24/24` required cases. Details are in `research/notes/2026-04-14-k3-shortcut-equivalent-lag-dominance-cache.md`.

- `loop17-endpoint-seed-diversity-ab` Tested true guide diversity (unique guides increased) by injecting a distinct lag-8 endpoint-derived seed into hard dim5 stage-2.
  Failed (negative). At attempts `64`, the pool+seed run increased `unique_guides` (`12 -> 13`) but kept lag/improvement outcomes unchanged (still lag 7, improvements/promotions 3/1) while increasing work substantially. Raising attempts to `128` with this seed timed out at the 240s outer cap. Details are in `research/notes/2026-04-14-k3-shortcut-endpoint-seed-diversity-ab.md`.

- `loop18-admission-lagband` Tested and reverted a lag-band guide-admission heuristic in `shortcut_search` (`effective_lag <= best_lag + slack`).
  Reverted. With `slack=0`, hard dim5 stage-2 probes became cheaper and one previously timing-out plus-seed attempts-128 probe completed, but harness telemetry-focus metrics regressed (`45,802,619 -> 43,380,841`) despite unchanged required correctness/targets. Retuning to `slack=1` lost the tractability gain (attempts-128 timed out again). Details are in `research/notes/2026-04-14-k3-shortcut-admission-lagband-reverted.md`.

- `loop19-dim5-lagcap5-timeout-boundary` Mapped the hard dim5 lag-cap-5 timeout cliff on the kept codebase (post-loop16 cache change).
  Kept as evidence only. Attempts `96` completed (lag 7, improvements 5, promoted 2), while `104/112/128` all timed out at 240s. At attempts `128`, reducing `max_guides` (`12 -> 8 -> 4`) and reducing `shortcut_rounds` (`2 -> 1`) still timed out. This confirms a steep segment-mix cost cliff not fixed by coarse outer knobs. Details are in `research/notes/2026-04-14-k3-shortcut-dim5-lagcap5-timeout-boundary.md`.

- `loop20-gap5-attempt-saturation` Mapped the tractable dim5 lag-cap-5 gap window and measured attempt saturation on `max_gap=5`.
  Kept as evidence only. `max_gap=6` timed out at attempts 128, while `max_gap=5` completed and saturated by attempts 160/192 with unchanged lag/improvement outcomes (still lag 7, improvements 10, promoted 2) and higher work (`factorisations 12.08M -> 13.71M -> 14.29M`). This supports focusing on segment-quality/cost pruning instead of larger attempt budgets. Details are in `research/notes/2026-04-14-k3-shortcut-dim5-lagcap5-gap5-attempt-saturation.md`.

- `loop21r/22-rebuild-validated-maxentry5` Revalidated hard dim5 shortcut measurements on a freshly rebuilt dist `search` binary and mapped a tractability lever.
  Kept as evidence only. Rebuild-validated guide-count sweep at attempts 96 showed no lag movement (all lag 7, improvements/promotions 10/2). Mixed vs graph-only at attempts 96 confirmed graph-only is much cheaper but under-produces local improvements. The strongest actionable result was mixed `max_entry` A/B: at attempts 128, `max_entry=5` matched lag/improvement outcomes while reducing work (`factorisations 12,240,338 -> 11,227,660`, `visited 700,884 -> 487,095`), and at attempts 160 `max_entry=5` completed (lag 7, improvements 20) while `max_entry=6` timed out at 240s. Details in `research/notes/2026-04-14-k3-shortcut-rebuild-validated-maxentry5-mixed.md`.

- `loop23-maxentry5-gap-followup` Checked whether the `max_entry=5` lever opens wider gap windows and tuned guide count in the tractable window.
  Kept as evidence only. `gap=6` remained poor under `max_entry=5` (attempts 128 timed out; attempts 96 completed but with lower yield and higher cost than `gap=5`). In `gap=5`, attempts 160 remained lag 7 with identical improvement outcomes across guides 12/8/6, with only modest runtime/work reductions for guides 8/6. This confirms the active baseline as `mixed + max_entry=5 + gap=5`, and points next work toward segment-order/admission quality rather than broader gaps or guide-count tweaks. Details in `research/notes/2026-04-14-k3-shortcut-maxentry5-gap-window-followup.md`.

- `loop24-min-gap3-timeout` Tested segment-selection policy `guided_min_gap=3` on the rebuilt `mixed + max_entry=5` hard-dim5 baseline.
  Failed. Both attempts 128 and 160 timed out at 240s (empty JSON), while the corresponding min-gap-2 baseline at attempts 160 completed with lag 7 and improvements 20. Decision: keep `guided_min_gap=2`; min-gap-3 is regressive on this surface. Details in `research/notes/2026-04-14-k3-shortcut-maxentry5-min-gap3-timeout.md`.

- `loop25-dim-priority-ordering` Tested and reverted dimension-priority segment ordering for timeout-bounded shortcut refinement.
  Reverted. Harness gate stayed stable (24/24 required; target/points/telemetry-focus unchanged, slight total-elapsed improvement), but the hard dim5 decision probe (`mixed + max_entry=5 + gap<=5 + attempts160 + guides8`) regressed work with no lag/progress gain (`factorisations 12,970,458 -> 13,435,679`, `visited 539,515 -> 559,880`, lag stayed 7). Details in `research/notes/2026-04-14-k3-shortcut-dim-priority-ordering-reverted.md`.

- `loop26-beam-timeout-ab` Measured beam frontier variants for timeout-bounded shortcut segment searches on the rebuilt hard-dim5 baseline.
  Failed to improve the active objective. Beam with timeout 5 reduced work counters but was slower on wall time than BFS and did not improve lag. Beam width 4 with timeout 1 was faster but under-produced and saturated local improvements (13 at attempts 160-192) versus BFS attempts-160 (20), with lag unchanged at 7. Decision: keep BFS baseline. Details in `research/notes/2026-04-14-k3-shortcut-beam-timeout1-ab.md`.

- `loop27-lagcap-timeout-boundary` Re-measured lag-cap and segment-timeout tradeoffs on the rebuilt hard dim5 baseline.
  Kept as evidence only. Lag-cap 4 was strictly worse than lag-cap 5 at attempts 160 (same lag/improvements, higher cost). Segment-timeout 3 improved tractability at fixed progress (attempts160 kept lag7/improved20/promoted3 while reducing work), and raised the feasible boundary to attempts168 (attempts176+ timed out). No lag<7 movement. Details in `research/notes/2026-04-14-k3-shortcut-lagcap-timeout-boundary-rebuild.md`.

- `loop28-timeout3-guide-breadth` Checked guide-count effects at the improved timeout=3 hard-surface boundary.
  Kept as evidence only. At attempts168, guides=12 was identical to guides=8 (same lag7/improvements/promotions/work), while guides=4 was strictly worse due two-round churn and higher cost. Guide-count tuning appears exhausted here.

- `loop28-k4-beam-envelope` Ran Goal-3-focused k4 endpoint sweeps and mapped a tractable beam envelope.
  Kept as evidence only. No k4 witness found (`equivalent` absent), but mixed-beam gives a tractable dim5 region up to about lag14 under 120s (`beam64, dim5, entry10` unknown in 101-119s), with a timeout cliff at lag16. Mixed beam-bfs-handoff timed out and did not improve reach in this setup.

- `loop29-k4-entry-ramp` Extended Goal-3 k4 beam-envelope sweeps with higher entry bounds and wider beam checks.
  Kept as evidence only. On `mixed + beam64 + dim5 + lag14`, entry 11/12 both remained unknown near the 120s cap (no witness). Beam width 96 was not tractable on dim5 entry12 (timed out at lag14 and lag12). The practical bounded envelope remains beam64 with lag up to about 14.

- `loop30-k4-graph-only-deep` Ran deep graph-only beam sweeps for Goal 3 on the k4 endpoint.
  Kept as evidence only. Graph-only scales to high lag quickly (including lag100 with beam256 in 78s and ~3.98M visited nodes), but every run remained unknown with no witness. This branch appears low-yield for Goal 3 compared with mixed-beam envelopes.

- `loop31-cofactor-reuse-4x4` Added reusable 4x4 cofactor/determinant solving path and removed repeated solve allocations in structured `4x4<->5x5` sparse-factorisation loops.
  Kept. Correctness and score gates were unchanged (`24/24` required, hits `21`, points `3645`, telemetry-focus `45,802,619`), while harness elapsed improved (`16046 -> 13581 ms`). Hard k=3 plateau stayed at lag `7`, but attempts-176 now completed under a relaxed `260s` cap (still timed out under strict `240s`). Goal-3 k4 mixed-beam lag16 remained timeout at `120s`.

- `loop32-dedup-signature-move` Removed `SameFuturePastSignature` clone churn in `deduplicate_expansions` by moving signatures into the representative set (`take()`), leaving behavior unchanged.
  Kept as a micro-optimization. Gates/score were unchanged (`24/24` required, hits `21`, points `3645`, telemetry-focus `45,802,619`), harness elapsed improved slightly (`13581 -> 13535 ms`), and the hard k=3 attempts-168 surface stayed effectively neutral on lag/work.

- `loop33-overdet-fastpath` Tested and reverted a `solve_overdetermined_5x4` nonsingular fast path that bypassed `solve_nonneg_4x4` allocations via precomputed cofactors.
  Reverted. Required correctness and score gates stayed unchanged, but runtime regressed slightly on both the hard dim5 attempts-168 probe (`228.88s -> 230.78s`) and harness aggregate (`13535 -> 13563 ms`).

- `loop34-singular-alloc-trim` Reduced singular-solver allocation churn by filtering `solve_nonneg_3x3` fallback candidates in place and replacing per-iteration 4x4 column-subset vector allocations with static arrays.
  Kept. Required and objective metrics were unchanged (`24/24`, hits `21`, points `3645`, telemetry-focus `45,802,619`), while harness elapsed improved (`13535 -> 13461 ms`). Hard k=3 attempts-168 remained neutral on lag/work (`lag 7`, improvements/promoted `20/3`).

- `loop35-attempt176-boundary-retest` Rechecked hard-surface strict boundary after loop34 runtime trims.
  Evidence only. Attempts-176 still timed out under strict cap240 (`240.01s`) and completed only under cap260 (`240.87s`) with unchanged lag/work telemetry, so no Goal-2 boundary movement despite keeping loop34 for harness runtime gains.

- `loop36-k4-mixed-boundary-recheck` Re-ran the refreshed k4 mixed-beam branch from the previously logged dim5 envelope and then rebuilt with `--release` to confirm the result.
  Evidence only. On current `HEAD 8606fcd`, the historical `mixed + beam64 + dim5` k4 envelope did not reproduce: `lag14/13/12` at `entry10-12` timed out under `target/dist/search`, and `lag12` plus even `lag10` at `entry10` still timed out under `target/release/search` (`120.02s`, empty JSON, no witness). Treat the practical cliff on this worker as at or below `lag10` for `beam64 + dim5 + entry10`; no keepable Goal-3 signal. Details in `research/notes/2026-04-15-k4-mixed-beam-boundary-recheck.md`.

- `loop37-k4-low-lag-reramp` Re-ramped the same release-binary k4 mixed-beam surface upward from low lag to recover the first completing points.
  Kept as boundary evidence. On `target/release/search` with `mixed + beam64 + dim5 + entry10`, lag `4/6/8` all returned `unknown` in `35.93s / 61.36s / 103.68s`, while lag `9` timed out at `120.02s` (matching the earlier lag10 timeout). The current keepable envelope on this worker is therefore completion through lag8 with a timeout cliff between lag8 and lag9; still no k4 witness. Details in `research/notes/2026-04-15-k4-mixed-beam-low-lag-ramp.md`.

## 2026-04-16

- `working tree` Added a bounded same-size balanced-neighbor surface and a
  one-bridge `2x2` zig-zag probe for `sse-rust-2uy.6`.
  Kept as evidence. `src/balanced.rs` can now enumerate distinct nontrivial
  same-size balanced-elementary neighbors of a `2x2` matrix and search for a
  bounded `2x2 <-balanced-> 2x2 <-balanced-> 2x2` meeting, while
  `find_balanced` exposes that surface for toy and Brix controls. The toy pair
  has exactly one nontrivial neighbor each way, but both Brix-Ruiz controls
  have empty same-size balanced neighborhoods at `max_common_dim=2` for
  `max_entry=8`, `10`, and `12`, so the first bounded zig-zag step still has
  no local room to start. Details in
  `research/notes/2026-04-16-balanced-neighbor-zigzag-first-slice.md`.

- `working tree` Added a bounded balanced bridge-neighbor seam on outsplit
  bridge states for `sse-rust-ckj`.
  Kept as evidence. `src/balanced.rs` now exposes canonical `2x2` outsplit
  bridge-state enumeration plus bounded balanced-neighbor hits between candidate
  bridge sets, and `find_balanced --bridge-neighbor-seam` reports the seam on
  named controls. The toy pair has a tiny positive bridge-state seam, but the
  Brix-Ruiz `k=3` and `k=4` controls still have zero bounded hits at
  `bridge_max_entry=8`, `max_common_dim=2`, `max_entry=8`, so even one
  dimension-changing bridge step does not yet create a balanced proposal path
  between the two sides. Details in
  `research/notes/2026-04-16-balanced-outsplit-bridge-neighbor-seam.md`.

- `working tree` Audited the measurement-only corpus baselines after the lane
  split.
  Retired the non-reproducing shared `beam_bfs_handoff` `cap10` probe,
  reworded the retained `k=3` frontier and staged-search controls, and moved
  the mixed `brix_ruiz_k3_wide_probe` / `brix_ruiz_k4_probe` surfaces into the
  explicit non-required evidence lane with neutral timeout scoring. Details in
  `research/notes/2026-04-16-measurement-corpus-baseline-audit.md`.

- `e0108b8` Added a bounded missing-reference literature slice for `sse-rust-9ls.6`.
  Kept as durable context. Added Baker 1983, Wagoner 1990, and a Choe-Shin
  1997 abstract-page reference; the resulting note says the best concrete
  follow-ups are a narrow `GL(2,Z)`-similarity-backed positive `2x2` dossier
  and research-only triangle-collapsible path telemetry, not a broader move-set
  widening. Details in
  `research/notes/2026-04-16-missing-references-and-solver-ideas.md`.

- `x53` Added a reporting-only `gl2z_similarity_profile_2x2` seam and a bounded
  research binary to inspect it.
  Kept. `src/invariants.rs` now reports exact `GL(2,Z)` similarity for `2x2`
  pairs via quadratic-order ideal classes in the irreducible case and the
  `gcd(A-\lambda I)` split invariant in the rational-eigenvalue case, while
  also classifying the Baker and Choe-Shin determinant bands. The new
  `profile_gl2z_similarity_2x2` binary exposes this as a research-facing dossier
  for named or explicit pairs without touching the main search. Focused tests
  cover irreducible, split, repeated-eigenvalue, and band-classification cases.

- `working tree` Wrote a durable last-day repo summary note.
  Added `research/notes/2026-04-16-last-24h-repo-summary.md` to summarize the
  highest-signal `git` and `bd` changes across solver behavior, harness and
  benchmark surfaces, literature findings, terminology rollout, and backlog
  shifts over roughly `2026-04-15 04:15 UTC` through `2026-04-16 04:15 UTC`.

- `working tree` Wrote autoresearch-round recommendations from the same review.
  Added `research/notes/2026-04-16-autoresearch-round-recommendations.md` with
  concrete suggestions for `research/program.md`, `research/cases.json`,
  harness-case tuning, supporting doc wording, and which fresh ideas should be
  elevated into program-level guidance.

- `working tree` Updated the autoresearch program and harness docs to match the
  current repo state.
  Refreshed `research/program.md`, `research/README.md`, and
  `docs/research-harness-benchmark-policy.md` so they explicitly distinguish
  required, measurement, and evidence lanes; point workers at
  `graph_plus_structured`, `deepening_schedule`, and the current frontier and
  proposal notes; and document why heavyweight boundary ramps should stay out of
  the shared default corpus unless they are cheap enough for normal runs.

- `working tree` Applied the first obvious corpus updates from the
  autoresearch recommendations.
  Promoted `lind_marcus_a_to_c` into the required correctness lane, added
  `brix_ruiz_k3_graph_plus_structured_probe`, added a deferred-cap-10
  `beam_bfs_handoff` diagnostic probe, reworded the depth-4 handoff case as a
  diagnostic baseline, and added a lightweight `k=4` deepening ramp case to
  exercise `deepening_schedule` without putting the heavyweight beam64+dim5
  boundary map into the shared corpus.

- `working tree` Added explicit keep/revert policy guidance for autoresearch
  rounds.
  Updated `research/program.md` to say that keep/revert decisions should be
  based on useful search per unit budget rather than one scalar, with separate
  goal, useful-reach, and budget ledgers plus different treatment for exact
  versus heuristic pruning. Added matching wording in
  `docs/research-harness-benchmark-policy.md` so repeated timing is not read in
  isolation from the reach signals.

- `sse-rust-t4p` Added bounded triangle-path quotient telemetry on existing
  witness/guide corpora.
  Kept as research-only telemetry. Added `src/path_quotient.rs` plus the
  `analyze_triangle_path_telemetry` research binary to mine lag-1/lag-2 local
  rewrite families from stored full paths, canonicalize short suffix windows,
  and report triangle-collapsible redundancy separately from state-collision
  style counts. On the bounded `lag <= 4` runs, the combined graph+guide corpus
  shrank from `444` unique windows to `187` under the local quotient, with
  `373/812` window occurrences collapsing and no rewrite-state truncation.
  Details are in `research/notes/2026-04-16-triangle-path-telemetry.md` and
  `research/runs/2026-04-16-triangle-path-telemetry-{graph,guides,combined}.json`.

- `working tree` Evaluated Lean as a bounded formal-methods sidecar for this
  repo.
  Added `research/notes/2026-04-16-lean-evaluation.md`. The conclusion is that
  Lean is only plausible for narrow witness, structured-move, or invariant
  soundness lemmas around `src/search/path.rs`, `src/graph_moves.rs`, and
  `src/invariants.rs`; it is not a good fit for the main solver, performance
  tuning, or the current measurement/reporting seams.

- `sse-rust-oaj` Rechecked the graph-only `brix_ruiz_k3` depth-`4`
  `beam_bfs_handoff` surface on rebuilt `target/dist/search`.
  Plain beam still returned `unknown` in `0.07s`, while the historical handoff
  and deferred caps `5`, `10`, and `20` all timed out at `5s` without final
  JSON. No cap variant earned a corpus change; details are in
  `research/notes/2026-04-16-beam-bfs-handoff-cap-sweep-graph-only-k3.md`.

- `sse-rust-4hp` Tested sub-beam deferred caps on the same graph-only
  `brix_ruiz_k3` depth-`4` handoff surface.
  On rebuilt `target/dist/search`, plain beam still returned `unknown` in
  `0.07s`, while the historical handoff and deferred caps `0`, `1`, `2`, and
  `3` all timed out at `5s` without final JSON. `deferred_cap = 0` still timed
  out, so the losing control is not rescued just by dropping retained overflow.
  No sub-beam cap earned a corpus change; details are in
  `research/notes/2026-04-16-beam-bfs-handoff-subbeam-cap-sweep-graph-only-k3.md`.

- `sse-rust-5d9` Scoped the first durable witness-corpus slice for
  ranking-signal analysis.
  Added `research/notes/2026-04-16-witness-corpus-first-slice.md` plus
  `research/witness_corpus_manifest.json`. The recommendation is to treat
  solved endpoint pairs as the corpus unit, reuse guide-artifact full paths as
  the first witness payload surface, keep `research/cases.json` as the
  endpoint-only literature manifest, and defer a wider sqlite witness-corpus
  surface until non-path proof kinds or stronger cross-source queries require
  it.

- `sse-rust-byw` Defined the family-aware evaluation contract for
  ranking-signal experiments.
  Added
  `research/notes/2026-04-16-family-aware-ranking-signal-eval-contract.md`
  plus `research/ranking_signal_family_benchmark_v1.json`. The contract fixes
  `evaluation_family_id` and canonical `pair_id` as the split atoms, treats
  path-segment expansion as development-only rather than a headline benchmark,
  pins down leakage and dedup rules around canonical endpoints and witness
  states, and sets the first meaningful held-out gate at the three non-Brix
  literature families with at least two rankable families.

- `sse-rust-oci` Re-profiled the current hard shortcut control and trimmed the
  hot `4x4` cofactor path in structured sparse factorisations.
  Kept. A bounded hard-control `pprof` sample on rebuilt `target/dist/search`
  shifted the hotspot to `cofactor_matrix_and_det_4x4` inside the structured
  `4x4 -> 5x5` sparse family, so `src/factorisation.rs` now computes those
  cofactors via reused `2x2` minors instead of rebuilding every `3x3` minor.
  On the current attempts-8 hard control, lag and guide outcomes stayed flat
  while wall time dropped from `39.27s` to `6.32s`; the full harness fitness
  stayed identical except for a small elapsed improvement (`23041 ms` ->
  `22969 ms`). Details are in
  `research/notes/2026-04-16-k3-hard-control-4x4-cofactor-unroll.md`.

- `sse-rust-t8r` Re-profiled after the `4x4` cofactor win and reused `3x3`
  adjugates in the next structured sparse hotspot.
  Kept. Fresh rebuilt-binary profiles still showed `square_factorisation_3x3`
  dominating raw mixed-search volume, but the bounded current hard shortcut
  control over-weighted `solve_nonneg_3x3` inside the structured
  `3x3 -> 4x4` sparse family. `src/factorisation.rs` now computes the `3x3`
  adjugate/determinant once per nonsingular core and reuses it across repeated
  RHS solves in the `4x4 -> 3x3` and `3x3 -> 4x4` structured sparse loops. On
  direct controls, mixed endpoint-search kept identical telemetry while wall
  dropped from `2.00s` to `0.48s`, and the hard attempts-8 shortcut control
  stayed at lag `7` with no guide improvements/promotions while wall dropped
  from `11.62s` to `6.49s`. Full harness fitness stayed unchanged; reruns
  landed at `23036 ms` and `22951 ms` versus the current-`HEAD` baseline
  `22969 ms`, so aggregate impact is effectively flat-to-slightly-positive.
  Details are in
  `research/notes/2026-04-16-k3-structured-3x3-adjugate-reuse.md`.

- `sse-rust-2ve` Re-profiled after the structured `3x3` adjugate win and
  tested two more bounded runtime ideas.
  Reverted. Fresh rebuilt-binary profiles still pointed at the square `3x3`
  `solve_nonneg_2x3` path and at move-family telemetry bookkeeping, so two
  low-level candidates were measured: a square-family-only prepared `2x3`
  solver path in `enumerate_sq3_from_row0`, and a hit fast path for
  `move_family_telemetry_mut`. Both helped the targeted direct controls, but
  neither cleared the aggregate keep gate: the localized prepared-solver path
  regressed harness reruns to `23197 ms` and `23242 ms`, and the telemetry-map
  fast path stayed slightly above the current kept baseline across three reruns
  (`23076 ms`, `23004 ms`, `23082 ms` vs `22951 ms`). Details are in
  `research/notes/2026-04-16-k3-runtime-round-reverted-square-prep-and-telemetry-map.md`.

- `sse-rust-5qq` Kept the telemetry follow-up by moving hot move-family
  accumulation onto borrowed family labels.
  Kept. Fresh rebuilt-binary profiles still sampled `move_family_telemetry_mut`
  on the bounded hard shortcut control, including a `BTreeMap::entry` frame on
  the pre-change tree. `src/search.rs` and `src/search/frontier.rs` now keep
  hot expansion telemetry in an internal `AHashMap<&'static str, ...>` and
  convert back to the public `BTreeMap<String, ...>` only when finalizing layer
  or aggregate telemetry. On the same attempts-8 hard control, lag and guide
  outcomes stayed flat while wall dropped from `6.83s` to `6.55s`. Because the
  historical `research/runs/` baselines were absent locally, aggregate
  confirmation used a clean `c70d511` snapshot harness rerun: baseline
  `22722 ms` and `22712 ms` versus patched `22631 ms` and `22686 ms`, with
  required hits, total points, and telemetry-focus score unchanged. Details are
  in
  `research/notes/2026-04-16-k3-telemetry-accumulator-borrowed-family-keys.md`.
