# Riedel graph-plus-structured reach versus graph-only and mixed (2026-04-17)

## Question

How far does current-`HEAD` `graph_plus_structured` search get on the
Riedel/Baker family compared with the current `graph_only` and `mixed`
references, when the comparison stays on bounded, policy-comparable harness
surfaces?

This round stays measurement-first:

- no solver rewrites;
- durable harness corpora for the new extension sweep and the edge repeat;
- one durable note tying the new results back to the existing `k = 4..12`
  comparison note.

## Sources and run surfaces

Durable sources:

- existing comparison note:
  - `research/notes/2026-04-17-known-pair-policy-coverage.md`
- existing graph-plus-structured baseline note:
  - `research/notes/2026-04-17-graph-plus-structured-harness-baselines.md`
- durable pair inventory:
  - `research/cases.json`
- new extension corpus for this slice:
  - `research/riedel_policy_reach_extension_corpus_2026-04-17.json`
- new repeat corpus for the `k = 16` edge:
  - `research/riedel_policy_reach_k16_repeat_corpus_2026-04-17.json`

Fresh current-worktree run artifacts:

- `tmp/riedel_policy_reach_extension_run_2026-04-17.json`
- `tmp/riedel_policy_reach_k16_repeat_run_2026-04-17.json`

Reproduce:

```bash
timeout -k 10s 95s target/dist/research_harness \
  --cases research/riedel_policy_reach_extension_corpus_2026-04-17.json \
  --format json \
  > tmp/riedel_policy_reach_extension_run_2026-04-17.json

timeout -k 10s 110s target/dist/research_harness \
  --cases research/riedel_policy_reach_k16_repeat_corpus_2026-04-17.json \
  --format json \
  > tmp/riedel_policy_reach_k16_repeat_run_2026-04-17.json
```

## Comparison policy

The family stays the same Riedel/Baker `2x2` lane already retained in
`research/cases.json`:

```text
A_k = [[k,   2],
       [1,   k]]

B_k = [[k-1, 1],
       [1,   k+1]]
```

The comparison envelope is policy-comparable within each rung:

- `max_lag = k`
- `max_intermediate_dim = 3`
- `max_entry = k`
- vary only `move_family_policy`

The existing note already established the lane through `k = 12`. This slice
extends the same surface with one anchor rerun and three higher rungs:

| Rung | Timeout |
| --- | --- |
| `k = 12` | `8000 ms` |
| `k = 14` | `12000 ms` |
| `k = 16` | `16000 ms` |
| `k = 18` | `20000 ms` |

The separate repeat corpus keeps the exact `k = 16` surface fixed and uses
`measurement.repeat_runs = 3` to check whether the new edge is stable.

## Existing durable lane

From `research/notes/2026-04-17-known-pair-policy-coverage.md`, the already
committed Riedel ladder on current `HEAD` was:

| Rung | `graph_only` | `graph_plus_structured` | `mixed` |
| --- | --- | --- | --- |
| `k = 4` | `unknown` (`1 ms`) | `equivalent`, lag `5` (`4 ms`) | `equivalent`, lag `5` (`85 ms`) |
| `k = 6` | `unknown` (`1 ms`) | `equivalent`, lag `7` (`29 ms`) | `equivalent`, lag `7` (`256 ms`) |
| `k = 8` | `unknown` (`1 ms`) | `equivalent`, lag `9` (`145 ms`) | `equivalent`, lag `9` (`399 ms`) |
| `k = 10` | `unknown` (`1 ms`) | `equivalent`, lag `11` (`564 ms`) | `equivalent`, lag `11` (`715 ms`) |
| `k = 12` | `unknown` (`1 ms`) | `equivalent`, lag `13` (`1943 ms`) | `equivalent`, lag `13` (`2167 ms`) |

So before this bead's extension, the durable signal was already:

- `graph_only` failed on every retained rung;
- `graph_plus_structured` matched `mixed` witness lag on every solved rung;
- `graph_plus_structured` was faster than `mixed` on every solved rung.

## Extension sweep results

Fresh rerun from
`research/riedel_policy_reach_extension_corpus_2026-04-17.json`:

| Rung | `graph_only` | `graph_plus_structured` | `mixed` |
| --- | --- | --- | --- |
| `k = 12` | `unknown` (`1 ms`) | `equivalent`, lag `13` (`1950 ms`) | `equivalent`, lag `13` (`2167 ms`) |
| `k = 14` | `unknown` (`1 ms`) | `equivalent`, lag `15` (`6268 ms`) | `equivalent`, lag `15` (`6458 ms`) |
| `k = 16` | `unknown` (`1 ms`) | `equivalent`, lag `17` (`15952 ms`) | `timeout` (`16033 ms`) |
| `k = 18` | `unknown` (`1 ms`) | `timeout` (`20036 ms`) | `timeout` (`20043 ms`) |

What changed:

- the old `k = 12` signal reproduced cleanly;
- `graph_plus_structured` and `mixed` still matched witness lag at `k = 14`,
  with `graph_plus_structured` again slightly faster;
- at `k = 16`, `graph_plus_structured` found a lag-`17` witness inside the
  `16000 ms` cap while `mixed` timed out at the same cap;
- at `k = 18`, both broader policies timed out at `20000 ms`.

## `k = 16` repeat probe

Fresh rerun from
`research/riedel_policy_reach_k16_repeat_corpus_2026-04-17.json`:

| Policy | Outcome counts over 3 repeats | Samples (ms) | Median representative |
| --- | --- | --- | --- |
| `graph_plus_structured` | `2 equivalent`, `1 timeout` | `15937`, `15940`, `16031` | `equivalent`, lag `17` |
| `mixed` | `0 equivalent`, `3 timeout` | `16032`, `16034`, `16043` | `timeout` |

This makes the `k = 16` edge real but narrow:

- `graph_plus_structured` is not coasting; it sits within roughly `60 ms` of
  the cap and can still miss on some runs;
- `mixed` did not produce a single solving run at the same cap in this repeat
  check.

## Furthest reach under the tested bounds

Across the retained old ladder plus this extension:

- `graph_only` stayed `unknown` on every tested rung `k = 4, 6, 8, 10, 12, 14,
  16, 18`;
- `graph_plus_structured` solved through `k = 16` and timed out at `k = 18`;
- `mixed` solved through `k = 14`, timed out at `k = 16` under the single-run
  sweep, and stayed `0/3` on the dedicated `k = 16` repeat probe.

So the current bounded reach answer is:

- furthest successful rung achieved by `graph_plus_structured`: `k = 16`
  with witness lag `17`;
- furthest successful rung achieved by `mixed`: `k = 14`
  with witness lag `15`;
- furthest successful rung achieved by `graph_only`: none in the tested lane.

## Explicit answer

On the current Riedel lane, `graph_plus_structured`:

- **exceeds `graph_only`** clearly: `graph_only` never solves any tested rung;
- **exceeds the current bounded `mixed` baseline** by one rung in this slice:
  `graph_plus_structured` reaches `k = 16` at the fixed `16000 ms` cap, while
  `mixed` does not;
- **does not keep extending indefinitely** under the same style of bound:
  both broader policies time out at `k = 18` with a `20000 ms` cap.

The right durable phrasing is therefore:

- `graph_plus_structured` remains coverage-equal to `mixed` through `k = 14`
  and slightly cheaper on every shared solve;
- the current best bounded Riedel reach belongs to `graph_plus_structured` at
  `k = 16`;
- that `k = 16` edge is real but timeout-tight, so it should be treated as the
  current frontier rather than a comfortably stable baseline.
