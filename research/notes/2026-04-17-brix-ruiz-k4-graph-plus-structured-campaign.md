# Bounded Brix-Ruiz `k=4` graph-plus-structured campaign (2026-04-17)

## Question

Does a bounded dedicated `graph_plus_structured` campaign on the open
Brix-Ruiz `k=4` endpoint produce any useful reach, witness, or frontier signal
worth pursuing further for Goal 3?

This slice stays measurement-first:

- no solver rewrites;
- one dedicated reusable campaign corpus for this branch-local round;
- one local JSON run artifact;
- and explicit comparison against the already-recorded mixed and graph-only
  `k=4` evidence instead of broad reruns.

## Endpoint and sources

Open Brix-Ruiz `k=4` endpoint:

- `A = [[1, 4], [3, 1]]`
- `B = [[1, 12], [1, 1]]`

Durable sources used for schedule selection and comparison:

- `research/notes/2026-04-17-graph-plus-structured-harness-baselines.md`
- `research/notes/2026-04-14-k4-goal3-beam-envelope.md`
- `research/notes/2026-04-17-graph-only-harness-baselines.md`
- `research/cases.json`
- new dedicated campaign corpus:
  `research/brix_ruiz_k4_graph_plus_structured_campaign_corpus_2026-04-17.json`

Local-only run artifact:

- `tmp/brix_ruiz_k4_graph_plus_structured_campaign_run_2026-04-17.json`

Reproduce:

```bash
timeout -k 10s 75s target/dist/research_harness \
  --cases research/brix_ruiz_k4_graph_plus_structured_campaign_corpus_2026-04-17.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_campaign_run_2026-04-17.json
```

## Why this bounded schedule

The earlier graph-plus-structured baseline note explicitly deferred a dedicated
`k=4` round until after the `k=3` baseline and optimization work landed. Fresh
local scouting in this worktree showed:

- `dim5 + beam64 + lag12 + entry10` was already too hot and timed out at
  `12000 ms`;
- `dim5 + beam32 + lag12 + entry10` was barely tractable (`unknown` in about
  `11.7 s`);
- `dim4 + beam64 + entry12` stayed cheap through lag `40`.

So the chosen campaign keeps two distinct surfaces:

1. a cheap `dim4` reach ramp to see whether graph-plus-structured produces any
   bounded frontier progress when widened on a Goal-3-style lag range;
2. a small `dim5` stress pair to check whether the lane carries to the first
   richer surface at all before committing to heavier work.

## Chosen schedule

Exact attempted cases from
`research/brix_ruiz_k4_graph_plus_structured_campaign_corpus_2026-04-17.json`:

| Case | Bound | Purpose | Timeout |
| --- | --- | --- | --- |
| `brix_ruiz_k4__graph_plus_structured__reach_1_lag20_dim4_entry12` | `beam64 + lag20 + dim4 + entry12` | cheap reach ramp | `8000 ms` |
| `brix_ruiz_k4__graph_plus_structured__reach_2_lag30_dim4_entry12` | `beam64 + lag30 + dim4 + entry12` | cheap reach ramp | `8000 ms` |
| `brix_ruiz_k4__graph_plus_structured__reach_3_lag40_dim4_entry12` | `beam64 + lag40 + dim4 + entry12` | cheap reach ramp | `10000 ms` |
| `brix_ruiz_k4__graph_plus_structured__stress_1_lag12_dim5_entry10_beam32` | `beam32 + lag12 + dim5 + entry10` | first richer stress point | `12000 ms` |
| `brix_ruiz_k4__graph_plus_structured__stress_2_lag16_dim5_entry10_beam32` | `beam32 + lag16 + dim5 + entry10` | first richer cliff check | `15000 ms` |

## Results

| Case | Outcome | Time | Frontier nodes | Visited | Factorisations | Approx. hits |
| --- | --- | --- | --- | --- | --- | --- |
| `reach_1_lag20_dim4_entry12` | `unknown` | `632 ms` | `2,434` | `37,375` | `144,210` | `11` |
| `reach_2_lag30_dim4_entry12` | `unknown` | `858 ms` | `3,714` | `44,644` | `162,997` | `26` |
| `reach_3_lag40_dim4_entry12` | `unknown` | `1144 ms` | `4,994` | `51,707` | `175,338` | `55` |
| `stress_1_lag12_dim5_entry10_beam32` | `unknown` | `11707 ms` | `706` | `30,990` | `484,581` | `5` |
| `stress_2_lag16_dim5_entry10_beam32` | `timeout` | `15015 ms` | `0` | `0` | `0` | `0` |

Additional frontier telemetry:

- every non-timeout run ended with
  `telemetry_summary.terminal_bottleneck = factorisation_volume`;
- no case found an exact meet or witness;
- on the cheap `dim4` ramp, widening lag from `20 -> 30 -> 40` increased
  approximate overlap (`11 -> 26 -> 55`) while staying below `1.2 s`;
- the first richer `dim5` stress point was much heavier:
  `484,581` factorisations for only `706` frontier expansions and still no
  witness;
- the next `dim5` step (`lag16`) fell off the cliff entirely inside the
  `15000 ms` cap.

## Comparison to existing `k=4` evidence

### Against the existing graph-only baseline

From `research/notes/2026-04-17-graph-only-harness-baselines.md`, the current
graph-only reach baseline on the open `k=4` pair is:

- `beam64 + dim5 + entry12 + lag20`: `unknown` in `2578 ms`,
  `approximate_other_side_hits = 18`
- `beam64 + dim5 + entry12 + lag30`: `unknown` in `4359 ms`,
  `approximate_other_side_hits = 86`
- `beam64 + dim5 + entry12 + lag40`: `unknown` in `5923 ms`,
  `approximate_other_side_hits = 232`

Compared with that retained graph-only lane:

- graph-plus-structured does have a cheap bounded ramp, but only on the
  smaller `dim4` surface;
- the `dim4` graph-plus-structured ramp produced far fewer approximate hits
  than the retained graph-only `dim5` ramp;
- once graph-plus-structured moved to `dim5`, it became slower than
  graph-only immediately and timed out by lag `16`.

So graph-plus-structured does **not** currently offer a better Goal-3-style
reach surface than the existing graph-only baseline.

### Against the existing mixed evidence

From `research/notes/2026-04-14-k4-goal3-beam-envelope.md`, the best retained
mixed k=4 envelope was:

- `beam64 + dim5 + entry10 + lag12`: `unknown` in `101 s`
- `beam64 + dim5 + entry10 + lag14`: `unknown` in `119 s`
- `beam64 + dim5 + entry10 + lag16`: timeout at `120 s`

This is not a perfectly identical comparison because the dedicated
graph-plus-structured stress surface used `beam32`, not `beam64`. Even with
that caveat, the current evidence suggests:

- graph-plus-structured is materially cheaper than the earlier mixed `dim5`
  beam envelope on this pair;
- but that cheaper cost does not buy any new witness or exact meet signal.

The safest durable claim is that current `graph_plus_structured` sits between
the existing graph-only and mixed lanes on this `k=4` family:

- cheaper than the previously recorded mixed `dim5` envelope;
- much less scalable than the retained graph-only reach baseline;
- still witness-free on every attempted surface.

## Conclusion

The answer for this bead is **bounded signal only**.

What the campaign established:

- a dedicated `graph_plus_structured` `k=4` lane can produce cheap bounded
  frontier telemetry on `beam64 + dim4 + entry12`, with approximate overlap
  growth through lag `40`;
- that signal is not strong enough to count as useful reach in the Goal-3
  sense, because it never produced a witness and does not survive the first
  richer `dim5` stress step;
- the dominant bottleneck remains structured factorisation volume rather than
  frontier starvation.

Recommendation:

- do **not** promote this corpus into the shared default `research/cases.json`
  set yet;
- keep the dedicated corpus as branch-level evidence;
- if there is a follow-up bead, it should only be justified as a narrow
  profiling or ranking-quality round on the `dim4` bounded ramp, not as a
  broader Goal-3 reach campaign.
