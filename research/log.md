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
