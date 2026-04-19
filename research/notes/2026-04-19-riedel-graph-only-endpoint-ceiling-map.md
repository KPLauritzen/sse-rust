# Bounded widened `graph_only` endpoint-ceiling map on the retained Riedel/Baker ladder (2026-04-19)

## Goal

Measure one bounded widened `graph_only` control surface on the solved retained
Riedel/Baker ladder and compare it directly against `graph_plus_structured`
without drifting into the open Brix-Ruiz `k = 4` lane or into a general solver
rewrite.

This note keeps one deliberately narrow question:

- if the retained dim-3 Riedel/Baker lane is widened just enough to admit the
  target-side endpoint ceiling, how far does `graph_only` recover; and
- what does that cost relative to `graph_plus_structured` on that same widened
  surface?

## Sources and retained artifacts

Primary source material:

- [`2026-04-18-riedel-gap-benchmark-lane.md`](./2026-04-18-riedel-gap-benchmark-lane.md)
- [`2026-04-18-riedel-k4-full-graph-decomposition.md`](./2026-04-18-riedel-k4-full-graph-decomposition.md)
- [`2026-04-18-riedel-graph-only-rectangular-endpoint-promotion.md`](./2026-04-18-riedel-graph-only-rectangular-endpoint-promotion.md)
- [`2026-04-18-riedel-k4-retained-interior-bridge-entry-threshold.md`](./2026-04-18-riedel-k4-retained-interior-bridge-entry-threshold.md)

New retained campaign inputs and outputs:

- corpus:
  [`research/riedel_graph_only_endpoint_ceiling_map_2026-04-19.json`](../riedel_graph_only_endpoint_ceiling_map_2026-04-19.json)
- measured run:
  [`research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json`](../riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json)

Focused reject-surface probes kept only in this note:

- `tmp/zvg_probe_k4_graph_only_lag8_dim4_entry12.json`
- direct stdout probes for `k = 6, 8, 10, 12, 14` on the chosen dim-3 surface

## Chosen campaign surface

The retained campaign surface is:

- retain the literature-backed ladder `k = 4, 6, 8, 10, 12, 14`;
- retain `max_intermediate_dim = 3`; and
- widen each rung to the target endpoint ceiling with one extra lag layer
  beyond the retained `graph_plus_structured` witness length.

Per rung, the chosen envelope is:

| Rung | Chosen `graph_only` / `graph_plus_structured` envelope |
| --- | --- |
| `k = 4` | `lag6 / dim3 / entry5` |
| `k = 6` | `lag8 / dim3 / entry7` |
| `k = 8` | `lag10 / dim3 / entry9` |
| `k = 10` | `lag12 / dim3 / entry11` |
| `k = 12` | `lag14 / dim3 / entry13` |
| `k = 14` | `lag16 / dim3 / entry15` |

Why this exact surface:

- the 2026-04-18 retained endpoint-promotion note already showed that the
  useful widened `graph_only` seam on the retained lane is the dim-3
  endpoint-admitting surface, not generic higher-dim broadening;
- the retained `k = 4` threshold note showed the first concrete recovery at
  `entry = 5`, which exactly matches the target endpoint ceiling on that rung;
  and
- on the retained lane, a coherent formula `max_entry = k + 1`,
  `max_lag = k + 2` gives `graph_only` one extra layer beyond the retained
  `graph_plus_structured` witness length while keeping runtime cheap enough for
  repeated measurement.

## Rejected dim-4 middle surface

I explicitly rejected `max_intermediate_dim = 4` as the main campaign surface.

Focused validation:

```bash
timeout -k 5s 20s target/dist/search \
  4,2,1,4 \
  3,1,1,5 \
  --max-lag 8 \
  --max-intermediate-dim 4 \
  --max-entry 12 \
  --move-policy graph-only \
  --json \
  > tmp/zvg_probe_k4_graph_only_lag8_dim4_entry12.json
```

Observed result:

- `outcome = unknown`

Why that reject matters:

- the retained rectangular endpoint lifts live only on the dim-3 `graph_only`
  lane from the 2026-04-18 promotion work;
- raising `max_intermediate_dim` above `3` moves onto a different policy
  surface that does **not** preserve that retained endpoint-admitting behavior;
  and
- using dim-4 as the main ladder map would therefore answer the wrong question.

## Reproduce

Build the research binaries:

```bash
cargo build --profile dist --features research-tools --bin research_harness --bin search
```

Focused campaign-slice validation:

```bash
target/dist/research_harness \
  --cases research/riedel_graph_only_endpoint_ceiling_map_2026-04-19.json \
  --worker-case riedel_baker_k4__graph_only__endpoint_ceiling_map
```

Full measured campaign:

```bash
target/dist/research_harness \
  --cases research/riedel_graph_only_endpoint_ceiling_map_2026-04-19.json \
  --format json \
  > research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json
```

## Current-head results

### Recovery map

| Rung | `graph_only` | `graph_plus_structured` |
| --- | --- | --- |
| `k = 4` | `equivalent`, lag `6`, median `6 ms` | `equivalent`, lag `5`, median `13 ms` |
| `k = 6` | `unknown`, median `9 ms` | `equivalent`, lag `7`, median `107 ms` |
| `k = 8` | `unknown`, median `22 ms` | `equivalent`, lag `9`, median `487 ms` |
| `k = 10` | `unknown`, median `51 ms` | `equivalent`, lag `11`, median `2005 ms` |
| `k = 12` | `unknown`, median `113 ms` | `equivalent`, lag `13`, median `6736 ms` |
| `k = 14` | `unknown`, median `222 ms` | `equivalent`, lag `15`, median `17744 ms` |

So on this bounded widened surface:

- `graph_only` recovers exactly `1/6` retained rungs, namely `k = 4`; and
- `graph_plus_structured` still solves all `6/6`.

### Search-volume map

Representative median-run search volume from the retained run JSON:

| Rung | `graph_only` generated / visited / expanded | `graph_plus_structured` generated / visited / expanded |
| --- | --- | --- |
| `k = 4` | `3854 / 244 / 237` | `7128 / 1390 / 458` |
| `k = 6` | `6403 / 407 / 404` | `29385 / 6802 / 6776` |
| `k = 8` | `9327 / 590 / 589` | `72043 / 19895 / 19877` |
| `k = 10` | `12753 / 796 / 792` | `150328 / 46870 / 46826` |
| `k = 12` | `16865 / 1052 / 1051` | `280805 / 94883 / 94843` |
| `k = 14` | `21022 / 1322 / 1321` | `480750 / 172693 / 172661` |

Totals across the full retained ladder:

- `graph_only`: median elapsed `423 ms`, generated `70224`, visited `4411`,
  expanded `4394`
- `graph_plus_structured`: median elapsed `27092 ms`, generated `1020439`,
  visited `342533`, expanded `341441`

The result to keep is:

- `graph_only` stays cheap on unsolved higher rungs because the widened dim-3
  frontier still exhausts quickly;
- `graph_plus_structured` remains the solving policy on every retained rung and
  pays between roughly `6x` and `80x` the wall time of `graph_only` on the same
  widened endpoints; and
- even on the single recovered rung `k = 4`, `graph_only` still needs a worse
  witness lag (`6` vs `5`) than `graph_plus_structured` on the same surface.

## Keep / Reject

Keep:

- the dim-3 endpoint-ceiling map as a durable bounded widened `graph_only`
  control surface for the retained Riedel/Baker ladder;
- the interpretation "entry widening to the target ceiling recovers only the
  first retained rung"; and
- direct policy comparison against `graph_plus_structured` on that same widened
  surface.

Reject:

- treating this as progress on the still-open Brix-Ruiz `k = 4` Goal 3 target;
- using dim-4 as the primary ladder map for this bead; and
- treating the single recovered `k = 4` rung as evidence that a general solver
  rewrite is justified.

## Durable conclusion

For future graph-only control-lane work, the reusable answer from this slice is:

- on the retained Riedel/Baker ladder, the cheapest coherent widened
  `graph_only` surface is the dim-3 endpoint-ceiling lane
  `max_lag = k + 2`, `max_entry = k + 1`;
- that surface recovers `k = 4` only, with witness lag `6`;
- it does **not** recover `k = 6, 8, 10, 12, 14`; and
- `graph_plus_structured` remains both broader and better-lagged on the same
  endpoint pairs.
