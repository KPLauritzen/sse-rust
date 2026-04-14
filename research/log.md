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
  Kept as evidence. It did not solve the hard cases, but it ruled out cheap direct balanced witnesses.

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
