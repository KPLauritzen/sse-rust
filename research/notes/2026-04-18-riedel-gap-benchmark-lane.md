# Durable Riedel graph-gap benchmark lane (2026-04-18)

## Goal

Freeze one durable committed Riedel/Baker comparison surface so later
graph-only decomposition work can measure against a stable lane instead of
reconstructing the ladder status from multiple 2026-04-17 notes.

This slice stays narrow:

- no solver rewrites;
- no non-Riedel corpus refactor;
- one retained benchmark corpus;
- one small frontier repeat probe; and
- one current-head note that states the keep/reject decision explicitly.

## Sources and retained artifacts

Primary source material:

- `research/cases.json`
- `research/notes/2026-04-17-known-pair-policy-coverage.md`
- `research/notes/2026-04-17-riedel-graph-only-vs-mixed-bounded-reach.md`
- `research/notes/2026-04-17-riedel-graph-plus-structured-reach.md`

New committed inputs for the durable lane:

- retained stable lane:
  `research/riedel_gap_benchmark_lane_2026-04-18.json`
- retained frontier check:
  `research/riedel_gap_benchmark_k16_frontier_2026-04-18.json`

Fresh current-worktree run artifacts for this note:

- `tmp/riedel_gap_benchmark_lane_run_2026-04-18.json`
- `tmp/riedel_gap_benchmark_k16_frontier_run_2026-04-18.json`

Reproduce:

```bash
cargo build --profile dist --features research-tools --bin research_harness

timeout -k 10s 45s target/dist/research_harness \
  --cases research/riedel_gap_benchmark_lane_2026-04-18.json \
  --format json \
  > tmp/riedel_gap_benchmark_lane_run_2026-04-18.json

timeout -k 10s 110s target/dist/research_harness \
  --cases research/riedel_gap_benchmark_k16_frontier_2026-04-18.json \
  --format json \
  > tmp/riedel_gap_benchmark_k16_frontier_run_2026-04-18.json
```

The direct binary runs above are intentional: an earlier parallel `cargo run`
probe in this worktree introduced enough build/process contention to distort
the boundary timing. The retained numbers below come from the clean direct
reruns.

## Retained lane

The retained durable benchmark lane is:

- `k = 4, 6, 8, 10, 12, 14`
- compare `graph_only`, `graph_plus_structured`, and `mixed`
- keep the endpoint matrices fixed to the Riedel/Baker literature family

```text
A_k = [[k,   2],
       [1,   k]]

B_k = [[k-1, 1],
       [1,   k+1]]
```

- keep the within-rung envelope fixed and vary only `move_family_policy`

Retained bounds:

| Rung | Frontier cap |
| --- | --- |
| `k = 4` | `lag5 / dim3 / entry4` |
| `k = 6` | `lag6 / dim3 / entry6` |
| `k = 8` | `lag8 / dim3 / entry8` |
| `k = 10` | `lag11 / dim3 / entry10` |
| `k = 12` | `lag12 / dim3 / entry12` |
| `k = 14` | `lag14 / dim3 / entry14` |

Lag convention used in this note:

- `config.max_lag` is the frontier-layer cap passed to the solver;
- harness `result_model.witness_lag` is the reported witness lag from the
  resulting path record; and
- on cap-saturating two-by-two frontier witnesses, that reported lag appears as
  `max_lag + 1`, so `max_lag = 14` can legitimately report witness lag `15`.

Why this is the retained lane:

- `k = 4..12` already had durable committed literature-backed endpoints in
  `research/cases.json`;
- `k = 14` reproduced cleanly on current `HEAD` in both the retained-lane run
  and the earlier direct repeat check; and
- later rungs remain frontier-only rather than stable benchmark surface.

## Rejected frontier

`k = 16` is documented but **not retained** as part of the durable benchmark
lane.

Reason:

- the 2026-04-17 extension already treated `k = 16` as a timeout-tight edge;
- the fresh current-head repeat no longer produced a single solving run there
  for either `graph_plus_structured` or `mixed` inside the same `16000 ms`
  cap; and
- `k = 18` was not carried forward because all broader policies had already
  timed out there, so it does not sharpen the stable benchmark lane.

## Current-head answer

The fresh current-head results below are the answer future rounds should cite
for the retained lane and the rejected frontier.

### Retained lane results

| Rung | `graph_only` | `graph_plus_structured` | `mixed` |
| --- | --- | --- | --- |
| `k = 4` | `unknown` (`1 ms`) | `equivalent`, lag `5` (`7 ms`) | `equivalent`, lag `5` (`98 ms`) |
| `k = 6` | `unknown` (`0 ms`) | `equivalent`, lag `7` (`30 ms`) | `equivalent`, lag `7` (`251 ms`) |
| `k = 8` | `unknown` (`0 ms`) | `equivalent`, lag `9` (`145 ms`) | `equivalent`, lag `9` (`403 ms`) |
| `k = 10` | `unknown` (`1 ms`) | `equivalent`, lag `11` (`571 ms`) | `equivalent`, lag `11` (`739 ms`) |
| `k = 12` | `unknown` (`2 ms`) | `equivalent`, lag `13` (`1948 ms`) | `equivalent`, lag `13` (`2144 ms`) |
| `k = 14` | `unknown` (`1 ms`) | `equivalent`, lag `15` (`6274 ms`) | `equivalent`, lag `15` (`6506 ms`) |

Totals across the retained lane:

- `graph_only`: `0/6 equivalent`, `6/6 unknown`, total elapsed `5 ms`
- `graph_plus_structured`: `6/6 equivalent`, total elapsed `8975 ms`
- `mixed`: `6/6 equivalent`, total elapsed `10141 ms`

### `k = 16` frontier repeat

| Policy | Outcome counts over 3 repeats | Samples (ms) | Representative result |
| --- | --- | --- | --- |
| `graph_only` | `3 unknown` | `1, 1, 1` | `unknown` |
| `graph_plus_structured` | `3 timeout` | `16032, 16036, 16043` | `timeout` |
| `mixed` | `3 timeout` | `16031, 16033, 16033` | `timeout` |

## Durable conclusion

The retained benchmark lane for later graph-only work is:

- the literature-backed Riedel/Baker ladder at `k = 4, 6, 8, 10, 12, 14`;
- the explicit three-policy comparison
  `graph_only` vs `graph_plus_structured` vs `mixed`; and
- the bounds recorded in `research/riedel_gap_benchmark_lane_2026-04-18.json`.

The current-head interpretation to keep with that lane is:

- `graph_only` is still the gap policy on the retained lane, with no solved
  rung;
- `graph_plus_structured` and `mixed` both solve the retained lane;
- `graph_plus_structured` remains the cheaper broader policy on every retained
  shared solve; and
- `k = 16` stays a documented frontier edge rather than part of the durable
  benchmark lane.
