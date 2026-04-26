# Repo activity summary: 2026-04-25 to 2026-04-26

## Question

What were the highest-signal repo changes over roughly the last 24 hours, and
which of them materially changed current solver direction versus mainly adding
evidence, instrumentation, or diagnostics?

## Scope

This note reviews activity in roughly the window from `2026-04-25 05:00:50 UTC`
through `2026-04-26 05:00:50 UTC`, continuing the durable summary cadence from
`research/notes/2026-04-18-last-24h-repo-summary.md`.

Primary sources:

- `git log main --since='2026-04-25 05:00:50 UTC' --stat --oneline`
- `git log --since='2026-04-25T07:00:50+02:00' --stat --oneline -- research/notes src docs`
- `research/log.md`
- durable notes in or about the window, especially:
  - `research/notes/2026-04-25-k3-lag7-bottleneck-classification.md`
  - `research/notes/2026-04-25-exact-endpoint-witness-inventory-surface.md`
  - `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-retained-hotspots-next-family-reject.md`
  - `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`
  - `research/notes/2026-04-25-brix-ruiz-k4-active-block-switch-proposal-reject.md`
  - `research/notes/2026-04-25-brix-ruiz-k4-active-block-switch-cluster-sweep-reject.md`
  - `research/notes/2026-04-25-brix-ruiz-k4-balanced-bridge-proposals-reject.md`
  - `research/notes/2026-04-25-brix-ruiz-k4-higher-power-insplit-proposal-scout-reject.md`
  - `research/notes/2026-04-25-concrete-shift-profile-beam-prototype.md`
  - `research/notes/2026-04-25-transfer-search-policy-witness-bridge-profile.md`
  - `research/notes/2026-04-25-same-future-past-frontier-diversity-metric.md`
  - `research/notes/2026-04-25-same-future-past-diversity-layer-correlation.md`
  - `research/notes/2026-04-25-concrete-shift-proposal-data.md`
  - `research/notes/2026-04-25-boolean-bridge-concrete-shift-family.md`
  - `research/notes/2026-04-25-goal4-dynamic-endpoint-parity-audit.md`
  - `research/notes/2026-04-25-structured-proof-result-surface-keep.md`
  - `research/notes/2026-04-25-endpoint-start-root-dispatch-seam.md`
  - `research/notes/2026-04-26-endpoint-exact-meet-retention-seam.md`
  - `research/notes/2026-04-26-factorisation-family-registry-seam.md`
  - `research/notes/2026-04-26-move-language-map-elementary-factorisations.md`
  - `research/notes/2026-04-26-endpoint-neighborhood-normal-forms-square-frontiers.md`
  - `research/notes/2026-04-26-trimmed-active-window-endpoint-parity-diagnostic.md`
  - `research/notes/2026-04-26-trimmed-active-window-coarse-bucket-ranking-experiment.md`
  - `research/notes/2026-04-26-approximate-hit-parity-report-surface.md`
- `bd list --all --updated-after '2026-04-25T07:00:50+02:00' --json`
- `bd ready --json`
- `bd list --status=open --json`
- `bd show sse-rust-nw7`
- `bd show sse-rust-mvst`
- `bd show sse-rust-tk0l`
- `bd show sse-rust-srl`
- `bd show sse-rust-379`

`research/log.md` only records the early concrete-shift proposal-report
addition from this window, so later work had to be reconstructed mainly from
durable notes, git history, and `bd` updates.

## Highest-signal changes

### 1. Goal 3 search direction narrowed further: many plausible `k=4` follow-ups were closed, and the surviving theme became "sparse same-profile `4x4` layout transfer"

- The retained open Brix-Ruiz `k=4` `graph_plus_structured` lane was reread
  several times from different angles and kept reproducing the same answer:
  no new exact meet, no changed baseline counters, and no clean next
  non-weighted family (`ea7c8fb`, `836db9b`, `d2f2c26`, `8a2e206`).
- The strongest retained structured signal stayed the rank-4/rank-6 sparse
  `4x4` diagonal near-hit pair. That mattered because the new stuck-state
  extractor turned aggregate approximate-hit counts into a concrete local
  obstruction, and the move-language note then connected that obstruction to
  the hard Baker `A4 -> A5` same-size step.
- That is a real direction change. The repo is now less justified in spending
  time on:
  - another generic `4x4 -> 3` admission gate,
  - another weighted reopening,
  - broad factorisation enumeration,
  - or another broad beam/refill widening pass.
- What remains alive is much narrower: a missing short word or local transfer
  inside sparse same-profile `4x4` boundary states. The active-block switch
  diagnostics were useful because they showed that this theme is not pure
  noise, but they still failed the promotion test: repeated `L1` improvement
  of only `2`, no exact counterpart, and no solver-family proof.

### 2. Goal 4 gained a concrete endpoint-local parity surface and a search-facing report path

- The most important positive new seam was the sequence:
  `goal4 dynamic parity audit -> normal-form audit -> trimmed active-window
  parity diagnostic -> coarse-bucket experiment -> approximate-hit parity
  report surface` (`41d58d4`, `0176649`, `4264060`, `ad9141d`, `4eff85c`,
  `7f1fe65`, `8072593`).
- This materially changed current Goal 4 direction. The repo no longer has
  only a vague "make square endpoint search more generic" target. It now has:
  - one retained local descriptor, `trimmed_active_window`;
  - one explicit three-way action vocabulary,
    `reuse_endpoint_local_parity` vs
    `rank_or_propose_inside_coarse_bucket` vs `ignore`;
  - and one opt-in `search` surface that can emit those annotations on real
    approximate hits.
- The kept boundary is disciplined:
  - reporting only;
  - no default beam ordering;
  - no hard pruning;
  - no hard dedup;
  - no canonicalization rewrite;
  - and no SSE-invariant claim.
- The in-progress follow-up `sse-rust-mvst` is important because the first
  search-facing report exposed its main missing edge immediately: top-level
  search can annotate outer approximate hits, but nested guided or shortcut
  segment hits still go unattributed.

### 3. Goal 2 did not get a new witness, but the lag-7 bottleneck reading became more explicit and less optimistic

- `research/notes/2026-04-25-k3-lag7-bottleneck-classification.md` is high
  signal because it does not just say "still no lag < 7". It sharpens the
  reason:
  - Baker and non-Baker exact lag-7 routes share the same early sparse
    `4x4` envelope;
  - they do not share a single canonical waypoint bottleneck;
  - and the hard missing bridges are exactly the ones that stop existing as
    low-dimensional graph moves.
- Combined with the exact endpoint witness inventory surface (`4cee43b`), the
  repo now has a better compact view of what exact-endpoint search is actually
  returning, including the explicit reminder that retained meet orientation is
  still missing and therefore worth only the narrow follow-up in `sse-rust-379`.
- The directional effect is negative but real: another blind shortcut or
  guide-pool rerun is less justified than it was yesterday.

### 4. The repo also spent part of the window paying down structural risk around the current search and parity seams

- The structured-proof result surface was generalized (`965e8d0`) and then the
  search and factorisation monoliths lost three bounded seams:
  - start/root observer dispatch (`fda1bc2`);
  - exact-meet retention ownership (`df8693d`);
  - factorisation family registry ownership (`034dc55`).
- These are not solver breakthroughs, but they matter because they reduce
  change pressure in exactly the areas that the Goal 4 parity/report work and
  endpoint inventory/reporting work are now touching.
- `bd` state reflects that this was not a one-off cleanup burst:
  `sse-rust-wwm` remains open and the next readability splits are already live
  as `wwm.4` through `wwm.7`.

## Kept vs. evidence-only conclusions

- Materially direction-changing keeps:
  - the retained Goal 3 reading that the remaining useful `k=4` signal is a
    sparse same-profile `4x4` layout-transfer problem, not another generic
    family-widening problem;
  - the Goal 4 endpoint-local parity descriptor/report path;
  - the narrow structured-proof and search/factorisation refactor seams that
    support those surfaces.
- Kept but still evidence-only:
  - the stuck-state extractor and move-language framing;
  - the exact endpoint witness inventory surface;
  - the report-only concrete-shift proposal surface and boolean-bridge
    restricted family;
  - the same-future/past diversity telemetry and layer-correlation reporting.
- Kept as opt-in candidates, not defaults:
  - `witness_bridge_profile_beam`, because it improved the retained open
    `k=4` lane modestly but only on one frozen evaluation and with mixed
    control results;
  - the approximate-hit parity report surface, because it is useful enough to
    keep but still diagnostic-only.
- Explicitly rejected or downgraded during the window:
  - `concrete_shift_profile_beam` as a promotion candidate;
  - balanced-bridge proposal promotion on the retained `k=4` lane;
  - higher-power in-split promotion on the retained `k=4` lane;
  - stratified beam deferred refill as the next retained `k=4` search mode;
  - active-block switch promotion from proposal diagnostic to solver family;
  - same-future/past saturation as a reason to run a narrower admission
    variant.

## Follow-up work that may be missing or underweighted

### 1. Not clearly owned by a live child bead: the next post-rejection Goal 3 slice under `sse-rust-nw7`

- `sse-rust-nw7` is still the live parent for open Brix-Ruiz `k=4`, but all of
  the concrete children that dominated this window were closed, and `sse-rust-uvwt`
  was closed explicitly as a duplicate of already-rejected active-block work.
- That leaves a gap at the child-bead level. The repo now has a better
  formulation of the remaining problem from
  `2026-04-26-move-language-map-elementary-factorisations.md`, but no live
  child that owns the next bounded non-duplicate test.
- Plausible bounded next step:
  search for a short exact word over the existing move language for Baker
  `A4 -> A5` and the retained rank-4/rank-6 sparse `4x4` near-hit shapes,
  without reusing the already-rejected contingency-switch screen and without
  reopening generic `4x4` enumeration.

### 2. Not clearly owned: calibration of parity action labels as usefulness signals, not just completeness signals

- `sse-rust-mvst` already owns propagation of the parity report into nested
  guided and shortcut segment searches. That work should not be called missed.
- What is still underweighted is a separate question: after propagation lands,
  do `reuse_endpoint_local_parity` and
  `rank_or_propose_inside_coarse_bucket` actually correlate with later exact
  segment success or useful guide extraction on any held-out square control?
- Plausible bounded next step:
  run the propagated report on one exact `k=3` replay control plus one retained
  `k=4` lane, then compare parity labels against which inner segment hits later
  become exact witnesses or accepted guides. That would keep the current
  reporting-only boundary while answering whether the labels are more than
  descriptive.

### 3. Underweighted but already partly framed by notes: decide whether Goal 2 should get another bounded slice soon at all

- The new lag-7 bottleneck classification is stronger than most prior "still no
  shorter witness" notes. It argues that the repo is repeatedly hitting a real
  envelope bottleneck rather than simply failing to sample enough routes.
- No live bead currently owns a new Goal 2-specific experiment downstream of
  that classification, which may be correct. But if another Goal 2 slice is
  attempted, it should start from a concrete segment-replacement hypothesis,
  not from another general shortcut replay, guide-pool rebuild, or multi-meet
  rerun.

## Active seams already covered by beads

- `sse-rust-mvst` already owns the next bounded Goal 4 integration step:
  propagate approximate-hit parity reporting into nested guided and shortcut
  segment searches.
- `sse-rust-tk0l` already owns the next bounded validation step for
  `witness_bridge_profile_beam`; that held-out replication work should not be
  called missed.
- `sse-rust-srl` and `sse-rust-7sd` already own the richer concrete-shift lane:
  profile design plus the lower-bound/correctness guardrail.
- `sse-rust-379` already owns the narrow endpoint witness inventory follow-up:
  record exact-meet orientation if it becomes analytically important.
- `sse-rust-wwm` plus `wwm.4` through `wwm.7` already own the current
  maintainability follow-through after the dispatch/exact-meet/family-registry
  splits.
- `sse-rust-nw7` still owns the open Goal 3 structured-family track at the
  parent level, even though the next concrete child bead is not yet obvious
  from the current evidence.

## Conclusion

The last 24 hours did not produce a new Goal 2 or Goal 3 witness. The biggest
durable effect was sharper triage.

For Goal 3, the repo closed several plausible-looking branches and narrowed the
remaining live theme to sparse same-profile `4x4` layout transfer, not generic
family widening. For Goal 4, it promoted endpoint-local square parity from an
offline idea into a real search-facing report surface with one clear
integration follow-up. For Goal 2, it made the lag-7 plateau look more like a
real current-envelope bottleneck and less like a missing easy rerun.

So the repo ends this window pointed toward:

- one narrower post-rejection Goal 3 child slice,
- one in-progress Goal 4 report-propagation seam,
- and a general preference for bounded report/diagnostic surfaces unless they
  beat the retained controls cleanly.
