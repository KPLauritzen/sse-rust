# Repo activity summary: 2026-04-19 to 2026-04-20

## Question

What were the highest-signal repo changes over roughly the last 24 hours, and
which of them materially changed current solver direction versus mainly adding
evidence, controls, or diagnostics?

## Scope

This note reviews activity in roughly the window from
`2026-04-19 05:29:31 UTC` through `2026-04-20 05:29:31 UTC`, continuing the
cadence of `research/notes/2026-04-19-last-24h-repo-summary.md`.

Primary sources:

- `git log main --since='2026-04-19 05:30 UTC' --stat --oneline`
- `git log --since='2026-04-19 05:30 UTC' -- research/notes src docs research/cases.json research/log.md --oneline --stat`
- `tail -n 140 research/log.md`
- durable notes read directly in or about this window:
  - `research/notes/2026-04-19-k3-lag7-retained-diversity-collapse.md`
  - `research/notes/2026-04-19-endpoint-multi-meet-surface.md`
  - `research/notes/2026-04-19-exact-endpoint-multi-meet-lag7-diversity.md`
  - `research/notes/2026-04-19-riedel-k4-retained-bridge-guided-replay-control.md`
  - `research/notes/2026-04-19-riedel-graph-only-endpoint-ceiling-map.md`
  - `research/notes/2026-04-19-riedel-k4-widened-graph-only-control-surface.md`
  - `research/notes/2026-04-19-bounded-dim-split-positive-control-replacement.md`
  - `research/notes/2026-04-19-brix-ruiz-k4-graph-plus-structured-explicit-4x4-to-3x3-amalgamation-cut-keep.md`
  - `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-4x4-to-3-singular-admission-gate-no-op.md`
  - `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-4x4-to-3-row-relation-admission-gate-no-op.md`
  - `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-weighted-4x4-to-3-family-reject.md`
  - `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-staged-weighted-4x4-to-3-fallback-reject.md`
  - `research/notes/2026-04-20-riedel-k4-full-graph-only-guided-replay-control.md`
- `bd list --all --updated-after '2026-04-19T05:30:00Z' --json`
- `bd ready --json`
- `bd list --status=open --json`
- `bd show sse-rust-ise`
- `bd show sse-rust-nw7`
- `bd show sse-rust-3r3`

Current-state fact from `bd`:

- the only ready/open beads at write time are the feature wrappers
  `sse-rust-ise`, `sse-rust-nw7`, and `sse-rust-3r3`
- there are no live bounded child beads already owning the next concrete slices

## Highest-signal changes

### 1. Endpoint multi-meet moved from a tooling idea to a real Goal 2 diversity win

- The strongest solver-direction change in this window was the multi-meet line:
  `d75daf7` through `0a2521e` landed a bounded endpoint-search surface that
  can retain more than one exact meet, and `d8a5ceb` used it to recover a
  second explicit lag-`7` exact-endpoint witness on the hard Brix-Ruiz `k=3`
  pair.
- That matters because the repo's previous durable reading on this question was
  still collapse:
  `research/notes/2026-04-19-k3-lag7-retained-diversity-collapse.md` concluded
  that retained-only exact replay fell back to the Baker family even when the
  broader guide pool contained additional lag-`7` classes.
- The new note
  `research/notes/2026-04-19-exact-endpoint-multi-meet-lag7-diversity.md`
  changes that reading materially:
  - exact endpoint search on the hard pair retained `4` exact meets,
  - replaying only that retained exact-endpoint pool produced a lag-`7`
    witness that is not the committed Baker artifact,
  - and the bottleneck is therefore no longer "normal search only exposes one
    meet" on this slice.
- This is not just reporting. It changes the current Goal 2 strategy:
  endpoint-level diversity work now has one explicit bounded extraction surface
  that can produce genuinely new short witnesses, not only telemetry counts.

### 2. The open Brix-Ruiz `k=4` lane got narrower, not broader

- The most important search-side keep on the still-open Goal 3 lane was not a
  new witness. It was another bounded moveset cut:
  `5b70585` / `90e44ed` kept the explicit `4x4 -> 3x3` row/column
  amalgamation families out of `GraphPlusStructured` while preserving the
  broader `binary_sparse_rectangular_factorisation_4x3_to_3` surface.
- Together with the earlier same-window `3x3 -> 4x4` explicit split-family cut
  (`d5433e5`), the retained `beam256 + lag40 + dim4 + entry12` lane now has a
  clearer direction:
  - keep trimming explicit duplicate families that do not change frontier or
    progress behavior;
  - keep the broader sparse family that still carries the actual `4x4 -> 3`
    reach signal; and
  - stop spending rounds on same-budget beam-order retunes or cosmetic
    tie-break tweaks once they have timed out or gone flat.
- The same window then consumed a long run of tempting `4x4 -> 3` follow-ups
  and pushed them back into evidence-only status:
  - dimension-gap tie-break reject,
  - partition-refined equal-score tie-break reject,
  - determinant-only singular gate no-op,
  - stronger exact row-relation gate no-op,
  - unconditional weighted-row widening reject,
  - staged weighted fallback reject.
- The weighted-family notes are especially informative because they were not
  strict no-ops. They were real widenings with real continuity signal, but
  only after loosening the retained timeout cap. The durable conclusion from
  `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-weighted-4x4-to-3-family-reject.md`
  and
  `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-staged-weighted-4x4-to-3-fallback-reject.md`
  is that the repo should not reopen that line as unconditional family growth
  on the native retained cap.
- So the active Brix-Ruiz `k=4` direction changed in a disciplined way:
  keep spending the fixed-budget lane better, but stop treating every plausible
  `4x4 -> 3` widening or ranking tweak as equally alive.

### 3. The Riedel/Baker graph-only control lane became much more reusable

- The second big directional change was not on the open witness hunt at all. It
  was the continued conversion of solved Riedel/Baker `k=4` graph-only results
  into durable reusable controls:
  - `5052e88` froze the retained interior bridge as a narrow baseline vs
    guide-backed replay pair;
  - `2765303` and `a3078b3` measured and then froze the dim-`3`
    endpoint-ceiling recovery map and its explicit `k=4` worker-case;
  - `25078fd` added one wider-envelope guide-backed replay control for the full
    graph-only `k=4` existence witness from the 2026-04-18 decomposition note.
- This matters because the graph-only lane now has three distinct reusable
  surfaces instead of mostly sidecar evidence:
  - retained narrow-lane obstruction plus guided replay,
  - widened endpoint-ceiling existence at `lag 6 / dim 3 / entry 5`,
  - and wider full-path replay existence at `lag 19 / dim 5 / entry 12`.
- That changes current graph-only strategy materially. Future graph-only work
  can now compare:
  - "plain retained lane is still blocked,"
  - "guide-backed replay explains the retained bridge,"
  - "plain widened endpoint search recovers the first rung,"
  - and "guide-backed wider replay preserves the stronger solved witness."
- This is still not Brix-Ruiz Goal 3 progress, and the notes are careful about
  that. But it is a real backlog-direction change for graph-only tooling and
  control work on the solved family.

### 4. Control surfaces were hardened rather than left as stale historical evidence

- The day also tightened the quality of the harness/control layer:
  `52bb938` / `3a14d1e` replaced the stale positive-side dim-`2` vs dim-`3`
  control with a same-endpoint `riedel_baker_k4` split, and
  `f7c5546` recorded another bounded exact-orbit exhaustion result rather than
  leaving that line only as an earlier one-off.
- These changes did not redirect main search policy by themselves, but they do
  matter for evaluation discipline:
  - the dim-split control now asserts a genuinely higher-dimensional success on
    the same endpoints rather than relying on an old rectangular story that had
    been invalidated by a lag-`1` witness;
  - the exact-orbit/certificate line remains capable of producing real bounded
    local keep/reject statements instead of only runtime micro-optimizations.
- So this part of the window is best read as control-surface hardening rather
  than solver novelty.

## Kept vs. evidence-only conclusions

- Direction-changing keeps:
  - bounded endpoint multi-meet extraction as a real search/result surface;
  - one new explicit non-Baker lag-`7` exact-endpoint witness on hard
    Brix-Ruiz `k=3`;
  - further `GraphPlusStructured` family cuts on the retained Brix-Ruiz `k=4`
    lane, especially the `4x4 -> 3x3` amalgamation cut;
  - reusable Riedel/Baker graph-only controls across retained, endpoint-ceiling,
    and wider replay envelopes.
- Kept, but mainly as control/reporting hardening:
  - the replacement dim-`2` vs dim-`3` positive control;
  - the additional exact-orbit bounded-exhaustion note;
  - the Riedel bridge replay and wider replay worker-cases as benchmark/control
    surfaces rather than solver breakthroughs.
- Evidence-only or reject conclusions:
  - the older lag-`7` retained-pool collapse inventory;
  - Brix-Ruiz `k=4` dimension-gap and partition-refined beam tie-break probes;
  - determinant-only and row-relation `4x4 -> 3` admission gates;
  - unconditional weighted `4x4 -> 3` widening;
  - staged weighted `4x4 -> 3` fallback.
- The important methodological change is that the weighted-family line now has
  two bounded negative results. It is no longer just "promising but not yet
  packaged"; it is explicitly below the keep bar on the retained cap.

## Follow-up work that may be missing or underweighted

### 1. Freeze the new non-Baker lag-7 exact-endpoint witness as a durable control surface

- Why it looks worthwhile:
  the repo now has a new explicit lag-`7` exact-endpoint artifact from the
  multi-meet replay line, not just a note that diversity exists.
- Evidence behind that judgment:
  `research/notes/2026-04-19-exact-endpoint-multi-meet-lag7-diversity.md` and
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`.
- Bounded next step:
  add one harness worker-case or one explicit guide-backed replay control that
  validates the non-Baker lag-`7` witness directly on the hard exact endpoints.
- Why this still looks underweighted:
  `sse-rust-28u.1` is closed and `sse-rust-ise` is the generic tooling feature,
  not a durable consumer/control bead for this new witness surface.

### 2. If exact-orbit/certificate work is reopened, take one more family only if it admits a comparably clean exact action

- Why it looks worthwhile:
  the exact-family line now has two durable examples of bounded local payoff:
  the older `4x3 -> 3` orbit seam and this window's `3x3 -> 4` bounded orbit
  exhaustion.
- Evidence behind that judgment:
  `research/notes/2026-04-18-binary-sparse-4x4-to-3-orbit-representative-seam.md`,
  `research/notes/2026-04-18-binary-sparse-4x4-to-3-bounded-orbit-exhaustion.md`,
  and `research/notes/2026-04-19-binary-sparse-3x3-to-4-bounded-orbit-exhaustion.md`.
- Bounded next step:
  choose one additional hot structured family only if it has a clean exact
  symmetry action and run one more bounded fixed-control exhaustion/no-go pass.
- Why this is not clearly owned:
  none of the currently live feature wrappers (`sse-rust-ise`, `sse-rust-nw7`,
  `sse-rust-3r3`) names this exact-method follow-up directly, and there is no
  live child bead for it.

### 3. Do not spend the next Brix-Ruiz `k=4` round on another weighted `4x4 -> 3` variant

- Why this needs to be explicit:
  the repo now has two bounded negative results on that exact line:
  unconditional widening and staged fallback both miss the retained cap.
- Evidence behind that judgment:
  `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-weighted-4x4-to-3-family-reject.md`
  and
  `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-staged-weighted-4x4-to-3-fallback-reject.md`.
- Bounded next step:
  if the open Brix-Ruiz `k=4` lane is revisited, start from a different
  family-local hotspot than weighted `4x4 -> 3` on the retained
  `beam256 + lag40 + dim4 + entry12` surface.
- Why this should **not** be called a missing bead:
  this is already within the broad ownership of `sse-rust-nw7`; the point here
  is prioritization guidance, not an unowned backlog gap.

## Active seams already covered by beads

- `sse-rust-ise` already owns further multi-meet tooling/evolution at the
  feature level. The missing part is not generic tooling ownership; it is the
  lack of a current bounded child bead for freezing the new non-Baker lag-`7`
  witness as a durable control.
- `sse-rust-nw7` already owns continued open Brix-Ruiz `k=4`
  `graph_plus_structured` family work. So general "keep pushing Goal 3 via new
  structured families" should not be described as missed.
- `sse-rust-3r3` already owns the retained Riedel/Baker graph-only benchmark
  and control lane. So broader graph-only control/tooling comparisons on the
  solved family are already owned at the feature level.
- At the same time, there are no ready/live bounded child beads under those
  wrappers at write time. So concrete next steps still need explicit child-bead
  creation rather than assuming the feature wrappers are enough.

## Conclusion

This was not a quiet day.

The biggest durable change was that one tooling line turned into a real solver
win: endpoint multi-meet is now an actual bounded search surface, and it
immediately produced a second explicit lag-`7` exact-endpoint witness on the
hard Brix-Ruiz `k=3` pair.

The open Brix-Ruiz `k=4` story moved in the opposite direction: one more
family cut was kept, but a long run of ranking, admission, and weighted
`4x4 -> 3` follow-ups all failed the retained-cap keep bar. So the lane is now
better pruned and better understood, but not newly solved.

And on the solved Riedel/Baker side, the repo spent the day converting graph-only
evidence into durable controls. That does not change Goal 3, but it does make
the graph-only benchmark lane much more useful for later tooling and
performance work.
