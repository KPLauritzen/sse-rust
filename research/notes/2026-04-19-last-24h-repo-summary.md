# Repo activity summary: 2026-04-18 to 2026-04-19

## Question

What were the highest-signal repo changes over roughly the last 24 hours, and
which of them materially changed current solver direction versus merely adding
evidence, reporting, or diagnostics?

## Scope

This note reviews activity in roughly the window from `2026-04-18 05:51:27 UTC`
through `2026-04-19 05:51:27 UTC`, continuing the cadence of
`research/notes/2026-04-18-last-24h-repo-summary.md`.

Primary sources:

- `git log main --since='2026-04-18 05:51:27 UTC' --until='2026-04-19 05:51:27 UTC' --stat --oneline`
- `git log main --since='2026-04-18 05:51:27 UTC' --until='2026-04-19 05:51:27 UTC' --oneline -- research/notes src docs`
- durable notes read directly in this window:
  - `research/notes/2026-04-18-riedel-gap-benchmark-lane.md`
  - `research/notes/2026-04-18-riedel-witness-classification-k4-k6.md`
  - `research/notes/2026-04-18-riedel-k4-retained-step-decomposition.md`
  - `research/notes/2026-04-18-riedel-k4-full-graph-decomposition.md`
  - `research/notes/2026-04-18-riedel-graph-only-elementary-conjugation-promotion.md`
  - `research/notes/2026-04-18-riedel-graph-only-rectangular-endpoint-promotion.md`
  - `research/notes/2026-04-18-riedel-k4-retained-interior-bridge-entry-threshold.md`
  - `research/notes/2026-04-18-positive-conjugacy-root-seed-hint-main-search-integration.md`
  - `research/notes/2026-04-18-graph-only-node-only-successor-fast-path.md`
  - `research/notes/2026-04-18-graph-plus-structured-signature-hotspot-no-op.md`
  - `research/notes/2026-04-18-brix-ruiz-k4-graph-plus-structured-beam-direction-signal-no-op.md`
  - `research/notes/2026-04-18-binary-sparse-4x4-to-3-orbit-representative-seam.md`
  - `research/notes/2026-04-18-binary-sparse-4x4-to-3-bounded-orbit-exhaustion.md`
  - `research/notes/2026-04-18-dynamic-graph-only-observer-layer-events.md`
  - `research/notes/2026-04-18-missed-future-work-avenues.md`
  - `research/notes/2026-04-18-docs-entrypoint-scope.md`
- `sed -n '780,980p' research/log.md`
- `bd list --all --updated-after '2026-04-18T05:51:27Z' --json`
- `bd ready --json`
- `bd list --status=open --json`
- `bd show sse-rust-5yo`
- `bd show sse-rust-5yo.5`
- `bd show sse-rust-2uy`
- `bd show sse-rust-2uy.3`
- `bd show sse-rust-2uy.10`
- `bd show sse-rust-2uy.29`
- `bd show sse-rust-2uy.30`
- `bd show sse-rust-2uy.31`

One important current-state fact from `bd`:

- `bd ready --json` returned `[]`
- `bd list --status=open --json` returned `[]`

So this note treats recently updated and recently closed beads as the active
ownership surface, not a live open backlog.

## Highest-signal changes

### 1. The Riedel/Baker graph-only gap stopped being a vague policy deficit and became a precise retained obstruction

- The strongest durable change in this window was the Riedel/Baker ladder work:
  the repo now has a committed retained benchmark lane
  (`research/notes/2026-04-18-riedel-gap-benchmark-lane.md`), concrete low-rung
  witness classification (`2026-04-18-riedel-witness-classification-k4-k6.md`),
  a retained interior-step decomposition
  (`2026-04-18-riedel-k4-retained-step-decomposition.md`), and a full explicit
  `k = 4` graph-only decomposition under a wider envelope
  (`2026-04-18-riedel-k4-full-graph-decomposition.md`).
- That changes the current graph-only question materially. It is no longer
  "which theorem-backed slice might help someday?" The repo now knows:
  - the retained lane is `k = 4, 6, 8, 10, 12, 14`;
  - `graph_only` is still `0/6` there;
  - `graph_plus_structured` and `mixed` both solve the retained lane; and
  - the low-rung mismatch is concrete: retained rectangular endpoint lifts plus
    interior `3x3 -> 3x3` steps that only collapse to graph-only after short
    expansions.
- This is a real solver-direction change. Graph-only follow-up should now be
  judged against the retained Riedel lane and its identified obstruction
  surface, not against a broad "graph-only seems weaker" impression.

### 2. Two smallest theorem-backed graph-only promotions were kept, but both mainly sharpened the remaining blocker

- The repo did promote exactly the next small graph-only slices:
  - retained `elementary_conjugation_3x3` on the dim-3 lane
    (`2026-04-18-riedel-graph-only-elementary-conjugation-promotion.md`);
  - retained `rectangular_factorisation_2x3` and
    `rectangular_factorisation_3x3_to_2` endpoint lifts on the same lane
    (`2026-04-18-riedel-graph-only-rectangular-endpoint-promotion.md`).
- Those are real main-search keeps, but the durable result is still negative on
  the retained benchmark lane: `graph_only` remains `0/6`.
- The key new conclusion is therefore not "graph-only is fixed." It is that
  after these promotions the smallest retained blocker is much clearer:
  the `k = 4` interior `3x3 -> 3x3` bridge stays `unknown` at `max_entry = 4`
  and flips to `equivalent` at `max_entry = 5` because the first admitted
  intermediate matrix `M1` itself has max entry `5`
  (`2026-04-18-riedel-k4-retained-interior-bridge-entry-threshold.md`).
- So the current direction changed from "add missing graph-only theorem slices"
  to "treat the retained interior bridge and its admission threshold as the
  next exact obstruction."

### 3. Sampled positive conjugacy crossed into the main solver, but only as a narrow ordering seam

- `research/notes/2026-04-18-positive-conjugacy-root-seed-hint-main-search-integration.md`
  records the first real main-search integration for sampled positive-conjugacy
  work: bounded seed hints now reorder the root expansion layer in
  `search_sse_2x2`.
- This matters because `sse-rust-2uy.3` is no longer just a sidecar-analysis
  bead. The signal now affects actual search order inside the solver.
- But the bounded result stayed neutral on the hard Brix-Ruiz `k = 3` control,
  and the integration was intentionally kept narrow:
  - `2x2` only;
  - root layer only;
  - exact same-dimension successor reordering only;
  - no prune or admission gate.
- So this is a kept architectural seam, not a broader change in active search
  strategy. The repo's current reading stays conservative: keep the signal as a
  tiny ordering aid until it beats simple controls on bounded cases that matter.

### 4. The day validated one more exact orbit/certificate seam, while ruling out two tempting direction changes

- The underweighted-exact-work survey from
  `research/notes/2026-04-18-missed-future-work-avenues.md` was mostly consumed
  inside the same window:
  - `sse-rust-2uy.29` closed because the preferred exact family-local gates
    were already present in the codebase;
  - `sse-rust-2uy.30` landed the next exact orbit seam on
    `binary_sparse_rectangular_factorisation_4x3_to_3`;
  - `sse-rust-2uy.31` measured the retained Brix-Ruiz `k = 4`
    `graph_plus_structured` dim-4 beam-direction retune and rejected it as a
    strict no-op.
- The orbit work is the lasting exact-method keep:
  `2026-04-18-binary-sparse-4x4-to-3-orbit-representative-seam.md` and
  `2026-04-18-binary-sparse-4x4-to-3-bounded-orbit-exhaustion.md` show a clean
  exact `S3` quotient and a bounded local no-go statement on the fixed mixed
  control (`1409 -> 428 -> 426`, with zero lag-feasible hits).
- That did not change main search policy, but it did reinforce the repo's
  current exact-pruning method: family-local exact orbit reduction and bounded
  certificates remain real; coarse beam-direction retuning on the retained
  `graph_plus_structured` Goal 3 lane did not.

### 5. Runtime and analysis work stayed disciplined, and the backlog surface is now unusually empty

- One clear runtime keep survived:
  `2026-04-18-graph-only-node-only-successor-fast-path.md` records a bounded
  graph-only hot-path change that preserved outcomes/counters while materially
  improving the accepted graph-only controls.
- Several other changes were deliberately pushed into evidence-only status:
  - `2026-04-18-graph-plus-structured-signature-hotspot-no-op.md` rejected the
    duplicate-class micro-optimization;
  - both `endpoint_equivalent_fast` regression investigations closed as
    non-reproducing noise;
  - `2026-04-18-dynamic-graph-only-observer-layer-events.md` improved
    endpoint-analysis rankability for `graph_only`, but did not change solver
    behavior;
  - `2026-04-18-docs-entrypoint-scope.md` and the docs refresh clarified repo
    navigation, not search direction.
- The `bd` effect is also durable: the day opened, executed, and closed nearly
  every seam it touched. By the time this note was written, both
  `bd ready --json` and `bd list --status=open --json` were empty.

## Kept vs. evidence-only conclusions

- Direction-changing keeps:
  the retained Riedel benchmark/decomposition surface; the bounded graph-only
  promotions that narrowed the retained blocker; the first positive-conjugacy
  main-search seam; and the graph-only node-only hot-path runtime keep.
- Kept but mainly as exact-method support:
  the `binary_sparse_rectangular_factorisation_4x3_to_3` orbit quotient and
  bounded local no-go tooling.
- Evidence or diagnostics, not direction changes:
  the direct wider-envelope `k = 4` graph-only decomposition artifact; the
  retained `max_entry = 4` vs `5` bridge-threshold classification; dynamic
  graph-only observer parity; docs entrypoint refresh; and the two
  `endpoint_equivalent_fast` regression investigations.
- Explicitly rejected in this window:
  the retained `graph_plus_structured` dim-4 beam-direction retune and the
  graph-plus-structured duplicate-signature micro-optimization.

## Follow-up work that may be missing or underweighted

There is no open or ready bead that already owns the suggestions below.

### 1. Reopen the retained `k = 4` interior bridge as its own bounded exact-obstruction task

- Why it looks worthwhile:
  the Riedel ladder work eliminated the broader ambiguity. The retained
  endpoint lifts are now promoted, the retained same-dimension conjugation lift
  is promoted, and the remaining retained blocker is localized to one interior
  `3x3 -> 3x3` bridge with a concrete `max_entry = 4` vs `5` admission split.
- Evidence behind that judgment:
  `2026-04-18-riedel-graph-only-rectangular-endpoint-promotion.md`,
  `2026-04-18-riedel-k4-retained-step-decomposition.md`, and
  `2026-04-18-riedel-k4-retained-interior-bridge-entry-threshold.md`.
- Bounded next step:
  either
  - exhaust the retained bridge at `lag <= 3`, `dim <= 3`, `max_entry <= 4`
    modulo obvious exact symmetries to determine whether the obstruction is
    intrinsic to the retained lane; or
  - consume the committed `entry = 5` guide artifact only in a retained-only
    research seam, without widening general graph-only policy.
- Why this is missing rather than already owned:
  `sse-rust-5yo` and its children are all closed, and current `bd` has no live
  replacement bead for this narrower post-promotion obstruction.

### 2. Preserve one durable graph-only existence control beyond the retained lane

- Why it looks worthwhile:
  `2026-04-18-riedel-k4-full-graph-decomposition.md` is more than a sidecar
  curiosity. It proves a direct `graph_only` `k = 4` witness under the wider
  envelope `lag = 19`, `dim = 5`, `entry = 12`, which is the strongest
  constructive graph-only evidence from the window.
- Bounded next step:
  freeze that witness as one explicit reusable control in the harness or in a
  committed guide-backed replay surface, so later graph-only work can compare
  "retained dim-3 obstruction" against "known wider-envelope existence" without
  reconstructing the sidecar evidence again.
- Why this still looks underweighted:
  the note records the artifact, but no live bead now owns turning that
  existence proof into a durable benchmark or guide surface.

### 3. If exact-orbit/certificate work is reopened, keep it family-local and tie it to another concrete bounded hotspot

- Why it looks worthwhile:
  the new `4x3 -> 3` orbit seam was strong enough to support a real bounded
  local no-go result, not just a local runtime cleanup.
- Evidence behind that judgment:
  `2026-04-18-binary-sparse-4x4-to-3-orbit-representative-seam.md` and
  `2026-04-18-binary-sparse-4x4-to-3-bounded-orbit-exhaustion.md`.
- Bounded next step:
  choose one additional hot structured family only if it admits a comparably
  clean exact orbit action and run the same local bounded-exhaustion test. Do
  not reopen this as a generic certificate framework.
- Why this is not already owned:
  `sse-rust-2uy.30` closed after landing the one seam; there is no current
  successor bead for the next family.

### 4. Do not reopen the same Goal 3 beam-direction retune; if the dim-4 lane is revisited, start from a new hotspot signal

- Why this needs to be said explicitly:
  the strongest "underweighted Goal 3 follow-up" from the earlier survey was
  immediately tried and ruled out in
  `2026-04-18-brix-ruiz-k4-graph-plus-structured-beam-direction-signal-no-op.md`.
- Bounded next step:
  if the retained `graph_plus_structured` `beam256 + dim4 + entry12` lane is
  reopened, start from a fresh pprof or family-hotspot signal and allow at most
  one family-local implementation attempt. Do not spend another round on
  same-depth beam-direction policy changes.
- Why this is a backlog gap:
  `sse-rust-2uy.31` is closed and no live bead replaces it with a different
  lane-local hypothesis.

## Active seams already covered by beads

- None are currently covered by live beads:
  `bd ready --json` returned `[]`, and `bd list --status=open --json` returned
  `[]`.
- The day's major seams were owned and then closed:
  - `sse-rust-5yo` and `sse-rust-5yo.1` through `.5` covered the retained
    Riedel graph-gap lane, witness classification, sidecar decomposition, first
    explicit `k = 4` graph-only decomposition, and bounded graph-only
    promotions;
  - `sse-rust-2uy.3` covered the narrow positive-conjugacy main-search seam;
  - `sse-rust-2uy.29` closed as already satisfied by existing exact family
    gates;
  - `sse-rust-2uy.30` covered the next exact structured-family orbit seam;
  - `sse-rust-2uy.31` covered and closed the retained dim-4 Goal 3 beam
    retune as a no-op;
  - `sse-rust-2sp` covered graph-only observer layer-event parity;
  - `sse-rust-csl` covered the graph-only runtime round;
  - `sse-rust-cr9` and `sse-rust-d4x` covered the two bounded
    `endpoint_equivalent_fast` regression investigations.
- So nothing below should be described as "already owned by a live bead." The
  repo currently has durable evidence and closed work items, but no active
  bead surface for the remaining solver questions.

## Conclusion

The biggest durable change in this window was not a new open-family witness. It
was the repo turning the Riedel/Baker graph-only gap into a precise retained
obstruction with committed benchmarks, explicit low-rung witness
classification, and bounded graph-only promotions that still fail for concrete
reasons.

The other enduring changes were narrower:

- sampled positive conjugacy now touches real main-search order, but only as a
  root-layer hint;
- graph-only got one more real runtime keep;
- exact orbit/certificate work gained another strong local seam; and
- several tempting Goal 3 or hotspot directions were explicitly ruled out.

The backlog implication is unusually stark: there are no ready or open beads at
all. So the next meaningful work should probably be reopened explicitly around
the retained `k = 4` interior bridge first, not around another broad graph-only
or Goal 3 widening pass.
