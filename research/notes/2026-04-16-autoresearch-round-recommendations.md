# Recommendations for the next autoresearch rounds

## Question

After reviewing repo and backlog activity from roughly `2026-04-15 04:15 UTC`
through `2026-04-16 04:15 UTC`, what should change to make future
autoresearch rounds more effective?

This note is intentionally a recommendation document. It does **not** apply the
suggested `research/program.md` or `research/cases.json` edits yet.

## Executive summary

Yes, `research/program.md` should be updated.

The last day changed the repo's operating reality in three important ways:

- the harness now has a real measurement lane and a first deepening surface;
- the most promising new search-side signal is a bounded graph-proposal
  shortlist seam, not broader frontier widening;
- several surfaces are now clearly diagnostic-only for the moment
  (`beam_bfs_handoff` depth sweeps, current positive-conjugacy waypoint
  proposals, higher-lag `k=4` mixed beam).

If the program text does not say that explicitly, future rounds will keep
repeating low-yield probes.

## Priority 1: update `research/program.md`

### Recommended changes

1. Refresh the "Current Bottlenecks" section.

Suggested direction:

- keep Goal 2 explicit: `brix_ruiz_k3` still stalls at lag `7`, and the
  remaining gap still looks structural rather than merely deeper;
- say plainly that `beam_bfs_handoff` is a measurement surface right now, not a
  promising default frontier strategy on the graph-only `k=3` control;
- add a new bottleneck around waypoint quality:
  graph-proposal shortlists look promising, but current sampled
  positive-conjugacy proposals do not survive exact invariants as waypoint
  targets;
- keep Goal 4 visible: generic square search improved only to power-trace
  prefiltering, so broader endpoint parity is still open.

2. Add an explicit "experiment lanes" split.

Suggested lanes:

- required correctness lane: existing required cases only;
- measurement lane: non-required benchmark-style frontier and staged-search
  probes;
- evidence lane: literature-backed or research-only cases that should inform
  direction but not gate merges.

This matters because the repo now has enough harness features that workers can
burn time on measurement-only surfaces without realizing they are off the main
goal path.

3. Tell workers to use `deepening_schedule` for boundary mapping.

The feature landed, but no corpus case uses it yet. The program should say:

- when mapping lag cliffs, bound ramps, or cap sweeps, prefer a
  `deepening_schedule` case over manual one-off reruns;
- use manual ad hoc commands only when the schedule format cannot express the
  experiment cleanly yet.

4. Expand the "What To Read Before Deep Changes" section.

Add these notes to the recommended read list:

- `research/notes/2026-04-15-graph-move-proposal-slice.md`
- `research/notes/2026-04-15-positive-conjugacy-phase2-usefulness.md`
- `research/notes/2026-04-14-beam-bfs-handoff-graph-only-k3.md`
- `research/notes/2026-04-15-k4-mixed-beam-low-lag-ramp.md`
- `research/notes/2026-04-16-missing-references-and-solver-ideas.md`

Without these, new rounds will miss the latest "don't keep repeating this"
evidence.

### Recommended wording changes

- Replace broad frontier optimism with a sharper statement:
  plain beam is still the cheap graph-only control; `beam_bfs_handoff` remains
  research-only until deferred-cap sweeps show clear value.
- Add a warning on sampled positive conjugacy:
  treat current proposal outputs as seed or reprojection material, not literal
  exact intermediate targets.
- Mention `graph_plus_structured` by name as the preferred intermediate policy
  when a round wants "more than graph-only, less than full mixed."

## Priority 2: tune and expand the harness cases

### Changes that should happen

1. Replace or augment the current `beam_bfs_handoff` measurement case.

Current problem:

- `brix_ruiz_k3_graph_only_beam_bfs_handoff_probe` still measures the known
  depth-`4` losing shape, even though the strongest new evidence says depth-only
  sweeps are exhausted and the next bounded question is deferred-cap sizing.

Recommendation:

- keep the existing case only as a historical baseline if it is still useful;
- add cap-based variants at the same default depth, for example one case each
  for `beam_bfs_handoff_deferred_cap = 5`, `10`, and `20`;
- compare those directly against plain beam in the same campaign.

2. Add a `graph_plus_structured` hard-`k=3` measurement case.

Reason:

- the new intermediate move-family policy is one of the biggest actual search
  surface changes from the day, but the corpus still makes workers choose
  between graph-only and full mixed in the highest-visibility cases.

Recommendation:

- add at least one non-required `brix_ruiz_k3` case using
  `move_family_policy = graph_plus_structured`;
- place it in the same measurement campaign as the frontier comparisons or in a
  nearby campaign with identical bounds.

3. Add a `k=4` boundary-mapping case that uses `deepening_schedule`.

Current problem:

- `brix_ruiz_k4_probe` is still a cheap `dim3` wide probe, while the fresh
  evidence is about the `beam64 + dim5 + entry10` release-binary boundary that
  completes through lag `8` and times out by lag `9`.

Recommendation:

- keep `brix_ruiz_k4_probe` as a cheap smoke test;
- add a separate non-required boundary campaign case using the actual mixed
  beam surface and a deepening schedule over something like `lag 4/6/8/9`;
- make the output clearly diagnostic, not target-scored.

4. Do not promote graph-proposal shortlist probes into the main harness yet.

Reason:

- the new graph-proposal result is promising, but it is still waypoint-local
  evidence rather than a stable endpoint-level benchmark surface;
- adding it too early would create more harness complexity than decision value.

Keep it as a research-tool-driven seam until it proves useful across more than
one waypoint family.

## Priority 3: adjust `research/cases.json`

### Cases that should likely be added or promoted

1. Promote one fast non-Brix literature case into the required lane.

Best candidates:

- `lind_marcus_a_to_c`
- `full_2_shift_higher_block_1x1_to_4x4`

Reason:

- both are source-backed,
- both exercise structure that the current required lane underrepresents,
- both are fast enough to be realistic correctness gates,
- neither depends on the Brix-Ruiz family.

If only one is promoted first, prefer `lind_marcus_a_to_c` because it is small,
mixed-dimension, and explicitly "SSE but not ESSE."

2. Keep the Riedel/Baker ladder non-required for now.

Reason:

- it is valuable as a lag-sensitive literature measurement lane;
- it should influence solver direction, but it is too scenario-heavy and too
  ladder-shaped to become part of the required correctness gate immediately.

### Cases that should be reprioritized or rephrased

1. Reframe the existing `beam_bfs_handoff` probe as diagnostic-only.

The case text should no longer imply that depth tuning is still the main open
question.

2. Split "hard target" from "hard measurement" more visibly in the frontier
campaigns.

The current corpus is directionally correct, but the measurement-only cases are
important enough now that their descriptions should say "diagnostic baseline"
or "measurement baseline" more explicitly.

### Cases that should wait

- Do not add `GL(2,Z)` dossier cases to `research/cases.json` yet unless the
  profile/reporting surface is wired into a standard harness output path.
- Do not expand the Riedel/Baker ladder beyond the current retained set until a
  worker actually needs a denser lag-sensitive family.

## Priority 4: update supporting docs around the program

### `research/README.md`

Add short mentions of:

- `measurement` blocks as first-class case metadata for non-required probes;
- `deepening_schedule` as the preferred way to express boundary or ramp
  experiments in the corpus;
- the distinction between required gates, measurement probes, and evidence
  campaigns.

### `docs/research-harness-benchmark-policy.md`

No major rewrite is needed, but one short addition would help:

- state that known-losing frontier configurations may still belong in the
  harness when they serve as stable measurement baselines, but they should be
  culled or replaced once they stop differentiating changes.

That would justify keeping some historical probes without making them feel like
open optimization targets forever.

## New ideas that deserve explicit program-level mention

These ideas are worth elevating into the program or adjacent durable guidance.

1. Triangle-collapsible path telemetry.

Why:

- it is the clearest new path-space idea from the literature refresh;
- it fits the repo's current guide and witness-path storage better than a large
  theorem-engine effort;
- it attacks path redundancy, not just state redundancy.

2. Stronger partition-refinement signatures.

Why:

- same-future/same-past quotienting is already paying off;
- the next plausible quality gain is stronger partition refinement, not just
  more duplicate-row and duplicate-column bookkeeping.

3. `GL(2,Z)` similarity plus Baker/Choe-Shin band reporting.

Why:

- it gives the repo a positive-only exact `2x2` classification surface;
- it is useful for literature-case explanation and later narrow shortcut work;
- it should remain descriptive first, not turn into broad solver widening.

4. Graph-proposal shortlist consumption under explicit bounds.

Why:

- this is the strongest new search-side positive signal from the day;
- it already has one convincing bounded waypoint realization result;
- it is the most plausible way to get new structured path progress without
  dumping a huge proposal universe into the main frontier.

## What future rounds should stop repeating

- Stop spending more depth-only time on `beam_bfs_handoff` for the graph-only
  `k=3` control unless cap-sized follow-ups first show value.
- Stop treating current sampled positive-conjugacy proposals as exact waypoint
  candidates; the invariant failures are already well established.
- Stop using the old broad `k=4` mixed-beam envelope as if it still reproduced
  on the current head; the current bounded boundary on this worker is completion
  through `lag 8`, timeout by `lag 9`.

## Conclusion

The next rounds should become more selective, not more exploratory in every
direction at once.

The program should now steer workers toward:

- explicit required-vs-measurement-vs-evidence lanes,
- `deepening_schedule` for boundary mapping,
- `graph_plus_structured` and graph-proposal shortlists as the main new search
  surfaces,
- and away from repeated frontier or proposal experiments that the last day
  already showed to be low-yield.
