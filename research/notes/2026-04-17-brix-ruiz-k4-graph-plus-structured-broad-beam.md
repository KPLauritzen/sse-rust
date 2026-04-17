# Broadening the Brix-Ruiz `k=4` graph-plus-structured beam (2026-04-17)

## Question

Was the earlier dedicated Brix-Ruiz `k=4` graph-plus-structured campaign held
back mainly by an overly narrow beam, and does broadening beyond `beam64`
produce useful new bounded signal?

This follow-up stays measurement-first:

- no solver rewrites;
- one reusable broad-beam corpus;
- one local JSON run artifact;
- and direct comparison against the earlier dedicated
  `beam64 + dim4`/`beam32 + dim5` campaign note.

## Sources

- prior dedicated campaign note:
  `research/notes/2026-04-17-brix-ruiz-k4-graph-plus-structured-campaign.md`
- durable broad-beam corpus:
  `research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json`
- local run artifact:
  `tmp/brix_ruiz_k4_graph_plus_structured_broad_beam_run_2026-04-17.json`

Reproduce:

```bash
timeout -k 10s 90s target/dist/research_harness \
  --cases research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_broad_beam_run_2026-04-17.json
```

Endpoint:

- `A = [[1, 4], [3, 1]]`
- `B = [[1, 12], [1, 1]]`

## Chosen broadened schedule

The earlier campaign had already shown:

- `beam64 + dim4 + entry12` stayed cheap through lag `40`, but produced only
  `11/26/55` approximate hits at `lag20/30/40`;
- the first richer `dim5` surface was already cliff-like.

So this follow-up keeps one broad `dim4` width sweep and one `dim5` long-timeout
control:

| Case family | Bounds |
| --- | --- |
| `beam128` dim4 ramp | `lag20/30/40 + dim4 + entry12` |
| `beam256` dim4 ramp | `lag20/30/40 + dim4 + entry12` |
| `beam512` dim4 probe | `lag20 + dim4 + entry12` |
| dim5 control | `beam64 + lag12 + dim5 + entry10 + timeout30s` |

This keeps the first pass bounded while still testing whether wider beams help
before the graph-plus-structured lane falls back into the old `dim5`
factorisation wall.

## Exact attempted cases

From
`research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json`:

| Case | Timeout |
| --- | --- |
| `brix_ruiz_k4__graph_plus_structured__beam128_lag20_dim4_entry12` | `12000 ms` |
| `brix_ruiz_k4__graph_plus_structured__beam128_lag30_dim4_entry12` | `15000 ms` |
| `brix_ruiz_k4__graph_plus_structured__beam128_lag40_dim4_entry12` | `18000 ms` |
| `brix_ruiz_k4__graph_plus_structured__beam256_lag20_dim4_entry12` | `15000 ms` |
| `brix_ruiz_k4__graph_plus_structured__beam256_lag30_dim4_entry12` | `18000 ms` |
| `brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12` | `24000 ms` |
| `brix_ruiz_k4__graph_plus_structured__beam512_lag20_dim4_entry12` | `24000 ms` |
| `brix_ruiz_k4__graph_plus_structured__beam64_lag12_dim5_entry10_timeout30s` | `30000 ms` |

## Results

### Dim4 width sweep

| Bound | Outcome | Time | Frontier nodes | Visited | Factorisations | Approx. hits |
| --- | --- | --- | --- | --- | --- | --- |
| `beam64 + lag20 + dim4 + entry12` | `unknown` | `632 ms` | `2,434` | `37,375` | `144,210` | `11` |
| `beam128 + lag20 + dim4 + entry12` | `unknown` | `1072 ms` | `4,866` | `62,599` | `261,472` | `35` |
| `beam256 + lag20 + dim4 + entry12` | `unknown` | `2065 ms` | `9,730` | `112,521` | `438,783` | `126` |
| `beam512 + lag20 + dim4 + entry12` | `unknown` | `4117 ms` | `19,458` | `225,332` | `749,177` | `172` |

| Bound | Outcome | Time | Frontier nodes | Visited | Factorisations | Approx. hits |
| --- | --- | --- | --- | --- | --- | --- |
| `beam64 + lag30 + dim4 + entry12` | `unknown` | `858 ms` | `3,714` | `44,644` | `162,997` | `26` |
| `beam128 + lag30 + dim4 + entry12` | `unknown` | `1436 ms` | `7,426` | `77,288` | `290,526` | `63` |
| `beam256 + lag30 + dim4 + entry12` | `unknown` | `2921 ms` | `14,850` | `145,621` | `511,832` | `165` |

| Bound | Outcome | Time | Frontier nodes | Visited | Factorisations | Approx. hits |
| --- | --- | --- | --- | --- | --- | --- |
| `beam64 + lag40 + dim4 + entry12` | `unknown` | `1144 ms` | `4,994` | `51,707` | `175,338` | `55` |
| `beam128 + lag40 + dim4 + entry12` | `unknown` | `2027 ms` | `9,986` | `97,548` | `323,848` | `95` |
| `beam256 + lag40 + dim4 + entry12` | `unknown` | `3734 ms` | `19,970` | `176,664` | `586,393` | `184` |

### Dim5 control

| Bound | Outcome | Time | Frontier nodes | Visited | Factorisations | Approx. hits |
| --- | --- | --- | --- | --- | --- | --- |
| `beam64 + lag12 + dim5 + entry10 + timeout12s` | `timeout` | `12015 ms` | `0` | `0` | `0` | `0` |
| `beam64 + lag12 + dim5 + entry10 + timeout30s` | `unknown` | `20598 ms` | `1,410` | `61,578` | `1,127,025` | `6` |

## What changed when the beam widened

### Beam64 was genuinely narrow on the dim4 lane

The width sweep makes the earlier dim4 result look beam-capped rather than
intrinsically exhausted:

- at `lag20`, approximate overlap rose `11 -> 35 -> 126 -> 172` as the beam
  widened `64 -> 128 -> 256 -> 512`;
- at `lag30`, overlap rose `26 -> 63 -> 165`;
- at `lag40`, overlap rose `55 -> 95 -> 184`.

So the user's complaint was correct: `beam64` was incredibly narrow for this
lane.

### The dim4 ramp remains tractable even after widening

Even `beam256` stayed comfortably bounded on the whole `dim4` ramp:

- `2065 ms` at `lag20`
- `2921 ms` at `lag30`
- `3734 ms` at `lag40`

That is materially heavier than `beam64`, but still cheap enough to serve as a
useful bounded measurement surface.

`beam512` at `lag20` did increase overlap further, but it already doubled the
runtime of `beam256` for a much smaller gain (`126 -> 172` approximate hits),
so the diminishing-returns point is visible.

### The richer dim5 surface is still the real cliff

Widening or relaxing the timeout does not turn the dim5 lane into a healthy
reach surface:

- the old `beam64 + lag12 + dim5 + entry10 + timeout12s` control timed out;
- giving the same case a `30000 ms` timeout let it finish, but only as
  `unknown` after `20598 ms`;
- that longer run paid for `1,127,025` factorisations and still produced only
  `6` approximate hits.

So the broader beam does not rescue the first richer graph-plus-structured
surface. The bottleneck is still factorisation volume, not just beam width.

## Comparison to existing k=4 baselines

Against the retained graph-only baseline from
`research/notes/2026-04-17-graph-only-harness-baselines.md`:

- graph-only `beam64 + dim5 + entry12` still has the stronger open-family reach
  surface overall;
- but broadening graph-plus-structured to `beam256` makes its cheap `dim4`
  overlap signal much less anemic than the earlier `beam64` note suggested;
- at `lag20` and `lag30`, `beam256` graph-plus-structured now exceeds the
  graph-only baseline's approximate hits (`126 > 18`, `165 > 86`), though this
  is still on the cheaper `dim4` surface, not the richer `dim5` one.

So the right reading is not that graph-plus-structured is suddenly the best
Goal-3 lane. It is that the previous dedicated note understated how much dim4
signal wider beams can expose.

## Conclusion

Broadening the beam changes the answer from the earlier campaign in one
important way:

- `beam64` was indeed too narrow on the bounded `dim4` graph-plus-structured
  lane;
- `beam128` and especially `beam256` expose materially stronger frontier signal
  while staying bounded;
- but there is still no witness, and the first richer `dim5` surface remains a
  factorisation-dominated cliff.

The durable conclusion is therefore:

- **useful bounded signal on dim4 once the beam is widened**;
- **no evidence yet that wider beams turn graph-plus-structured into a real
  k=4 witness or dim5 reach lane**.

Recommendation:

- if there is another bounded follow-up, use `beam256 + dim4 + entry12` as the
  graph-plus-structured k=4 measurement surface instead of `beam64`;
- do not yet promote a dim5 graph-plus-structured reach baseline, because the
  richer lane is still too expensive for too little signal.
