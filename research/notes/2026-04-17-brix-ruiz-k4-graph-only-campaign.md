# Bounded Brix-Ruiz `k=4` graph-only campaign (2026-04-17)

## Question

Now that the fresh graph-only optimization rounds have landed, does a bounded
graph-only rerun on the open Brix-Ruiz `k=4` endpoint show any useful Goal 3
reach or at least a measurement signal worth keeping for later comparison?

This round stayed intentionally narrow:

- graph-only only;
- no solver-feature work or broad refactors;
- one dedicated reusable campaign corpus for this branch-local round;
- one local JSON run artifact in `tmp/`;
- and explicit comparison against the existing durable graph-only baseline
  rather than broad family churn.

## Endpoint and sources

Open Brix-Ruiz `k=4` endpoint:

- `A = [[1, 4], [3, 1]]`
- `B = [[1, 12], [1, 1]]`

Durable sources used for the comparison point and schedule choice:

- `research/notes/2026-04-17-graph-only-harness-baselines.md`
- `research/notes/2026-04-17-graph-only-canonical-5x5-group-perms.md`
- `research/notes/2026-04-14-k4-graph-only-deep-beam.md`
- `research/cases.json`
- new dedicated campaign corpus:
  `research/brix_ruiz_k4_graph_only_campaign_corpus_2026-04-17.json`

Local run artifact:

- `tmp/brix_ruiz_k4_graph_only_campaign_run_2026-04-17.json`

Reproduce:

```bash
timeout -k 10s 90s target/dist/research_harness \
  --cases research/brix_ruiz_k4_graph_only_campaign_corpus_2026-04-17.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_only_campaign_run_2026-04-17.json
```

## Reconfirmed baseline surfaces

The shared graph-only reach baseline in `research/cases.json` is still the open
`k=4` Brix-Ruiz boundary ramp on:

- `beam64 + dim5 + entry12 + graph-only`
- lag `20 / 30 / 40`

Focused reruns on current `HEAD` reproduced the same search shape and improved
the wall time again relative to both the original durable baseline note and the
later `5x5` canonical-permutation optimization note:

| Surface | Durable baseline note | After graph-only 5x5 note | Current rerun |
| --- | --- | --- | --- |
| lag `20` | `2578 ms` | `1769 ms` | `1565 ms` |
| lag `30` | `4359 ms` | `3185 ms` | `2809 ms` |
| lag `40` | `5923 ms` | `4553 ms` | `4090 ms` |

All three current reruns stayed:

- `unknown`
- `factorisations_enumerated = 0`
- with the same baseline counters:
  - lag `20`: `2,354` frontier expansions, `128,118` visited, `18` approximate
    hits
  - lag `30`: `3,634` frontier expansions, `219,284` visited, `86` approximate
    hits
  - lag `40`: `4,914` frontier expansions, `305,954` visited, `232`
    approximate hits

These current-HEAD numbers are the comparison point for the dedicated campaign
below.

## Why this bounded schedule

The fresh graph-only-local rounds bought clear timing headroom on the exact
same `beam64 + dim5 + entry12` surface. The first question was whether that
headroom translated into deeper bounded graph-only reach, so this round kept
one axis fixed and widened lag only.

That choice is deliberate:

- it stays directly comparable to the durable graph-only baseline surface;
- it avoids mixing in new beam-width or policy axes on the same turn;
- the older `2026-04-14-k4-graph-only-deep-beam.md` note had already shown
  that broader beam sweeps were much more expensive and still witness-free.

Scout probes on the same surface stayed controlled at:

- lag `50`: `unknown` in `5630 ms`, `433` approximate hits
- lag `60`: `unknown` in `7165 ms`, `482` approximate hits
- lag `80`: `unknown` in `10341 ms`, `915` approximate hits
- lag `100`: `unknown` in `14993 ms`, `1316` approximate hits

So the kept reusable campaign corpus records one control plus four widened lag
points.

## Chosen schedule

Exact cases from
`research/brix_ruiz_k4_graph_only_campaign_corpus_2026-04-17.json`:

| Case | Bound | Purpose | Timeout |
| --- | --- | --- | --- |
| `brix_ruiz_k4__graph_only__control_lag40_dim5_entry12_beam64` | `beam64 + lag40 + dim5 + entry12` | control against shared baseline | `8000 ms` |
| `brix_ruiz_k4__graph_only__reach_1_lag50_dim5_entry12_beam64` | `beam64 + lag50 + dim5 + entry12` | first bounded extension | `12000 ms` |
| `brix_ruiz_k4__graph_only__reach_2_lag60_dim5_entry12_beam64` | `beam64 + lag60 + dim5 + entry12` | second bounded extension | `15000 ms` |
| `brix_ruiz_k4__graph_only__reach_3_lag80_dim5_entry12_beam64` | `beam64 + lag80 + dim5 + entry12` | deeper lag check | `18000 ms` |
| `brix_ruiz_k4__graph_only__reach_4_lag100_dim5_entry12_beam64` | `beam64 + lag100 + dim5 + entry12` | capstone bounded extension | `24000 ms` |

## Results

Total elapsed across the five-case campaign artifact:

- `42,451 ms`

Per-case outcomes:

| Case | Outcome | Time | Frontier nodes | Visited | Approx. hits | Frontier layers |
| --- | --- | --- | --- | --- | --- | --- |
| `control_lag40_dim5_entry12_beam64` | `unknown` | `4050 ms` | `4,914` | `305,954` | `232` | `80` |
| `reach_1_lag50_dim5_entry12_beam64` | `unknown` | `5597 ms` | `6,194` | `409,773` | `433` | `100` |
| `reach_2_lag60_dim5_entry12_beam64` | `unknown` | `7049 ms` | `7,474` | `495,649` | `482` | `120` |
| `reach_3_lag80_dim5_entry12_beam64` | `unknown` | `10587 ms` | `10,034` | `685,927` | `915` | `160` |
| `reach_4_lag100_dim5_entry12_beam64` | `unknown` | `15168 ms` | `12,594` | `891,350` | `1,316` | `200` |

Shared telemetry characteristics across the whole campaign:

- `factorisations_enumerated = 0` on every case
- `max_frontier_size = 64` on every case
- no witness and no exact meet surfaced in the retained artifact

## Interpretation

What changed:

- graph-only can now push the same retained `beam64 + dim5 + entry12` surface
  out to lag `100` in about `15 s` without falling off a timeout cliff;
- the frontier counters scale cleanly enough to make this a reusable
  measurement lane;
- approximate overlap is still growing materially (`232 -> 1316`) rather than
  immediately saturating.

What did **not** change:

- every retained point is still `unknown`;
- there is still no exact reach signal, witness, or even a near-witness event
  strong enough to count as Goal 3 progress;
- holding beam width at `64` means the lane is exploring deeper, not wider, so
  this is still a measurement statement rather than a new proof surface.

The lag-only ramp also shows that the overlap growth is not perfectly smooth:

- `lag50 -> lag60` only raised approximate hits from `433` to `482`
- `lag60 -> lag80` then jumped to `915`
- `lag80 -> lag100` rose again to `1316`

So the best durable claim is modest:

- there is useful **measurement signal**
- there is **not** useful `k=4` graph-only reach yet

## Comparison to earlier graph-only evidence

Against the shared graph-only baseline and its later optimization note:

- the same lag `20 / 30 / 40` surface is now clearly cheaper on current `HEAD`
- the counters stayed identical, so the speedup is real but purely operational
- that new headroom was enough to extend the bounded lag map through `100`
  within a still-manageable budget

Against the older `2026-04-14` deep-beam note:

- the older note already showed that `beam128` / `beam256` remained
  witness-free and became much more expensive
- this rerun does not overturn that conclusion
- it does sharpen it: even when the cheaper retained `beam64` lane is now
  tractable all the way to lag `100`, the answer is still `unknown`

So the current picture is:

- graph-only is a stronger bounded **telemetry** lane than it was before
- graph-only is still not a persuasive Goal 3 **reach** lane on this endpoint

## Conclusion

This campaign is a **keep as evidence** round, not a Goal 3 breakthrough.

The durable answer is:

- graph-only now has a better bounded `k=4` measurement lane on the open
  Brix-Ruiz endpoint
- graph-only still does not show useful `k=4` reach
- the new signal is worth preserving for future comparisons, but not strong
  enough by itself to justify prioritizing another graph-only-deep campaign
  over broader mixed or structured Goal 3 work

No new follow-up bead was opened from this round.

Reason:

- the campaign added better bounded reach telemetry,
- but it did not produce a witness or a strong enough qualitative change to
  overturn the earlier “low-yield” graph-only deep-beam conclusion.
