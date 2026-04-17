# Bounded Riedel graph-only reach versus mixed (2026-04-17)

## Question

On the retained Riedel/Baker `2x2` lane, how far does `graph_only` search get
relative to the current bounded `mixed` reference when the comparison stays
measurement-first and policy-comparable within each rung?

This round stays narrow:

- no solver rewrites;
- no broad corpus refactor;
- one staged bounded schedule;
- and one explicit answer for whether `graph_only` exceeds, matches, or falls
  short of the current `mixed` baseline on the Riedel lane.

## Sources and retained lane

Primary sources for this bead:

- `research/notes/2026-04-17-riedel-graph-plus-structured-reach.md`
- `research/notes/2026-04-17-known-pair-policy-coverage.md`
- `research/cases.json`

The retained family is the same Riedel/Baker lane already present in
`research/cases.json`:

```text
A_k = [[k,   2],
       [1,   k]]

B_k = [[k-1, 1],
       [1,   k+1]]
```

The durable retained `cases.json` rungs stop at `k = 12`:

- `riedel_baker_k4`
- `riedel_baker_k6`
- `riedel_baker_k8`
- `riedel_baker_k10`
- `riedel_baker_k12`

From `research/notes/2026-04-17-known-pair-policy-coverage.md`, the existing
`graph_only` vs `mixed` comparison point on current `HEAD` was:

| Rung | `graph_only` | `mixed` |
| --- | --- | --- |
| `k = 4` | `unknown` (`1 ms`) | `equivalent`, lag `5` (`85 ms`) |
| `k = 6` | `unknown` (`1 ms`) | `equivalent`, lag `7` (`256 ms`) |
| `k = 8` | `unknown` (`1 ms`) | `equivalent`, lag `9` (`399 ms`) |
| `k = 10` | `unknown` (`1 ms`) | `equivalent`, lag `11` (`715 ms`) |
| `k = 12` | `unknown` (`1 ms`) | `equivalent`, lag `13` (`2167 ms`) |

So the retained answer before this bead's fresh probes was already:

- `graph_only` had no successful retained Riedel rung;
- `mixed` solved through `k = 12`;
- and the natural next bounded question was whether that gap changes at the
  next few policy-comparable rungs.

## Bounded schedule

The comparison policy stayed constant within each rung:

- `max_lag = k`
- `max_intermediate_dim = 3`
- `max_entry = k`
- vary only `move_family_policy`

The schedule stayed explicit and staged:

### Initial stage

Corpus:

- `research/riedel_graph_only_vs_mixed_initial_corpus_2026-04-17.json`

Rungs:

- `k = 12` with `timeout_ms = 8000`
- `k = 14` with `timeout_ms = 12000`

Run artifact:

- `tmp/riedel_graph_only_vs_mixed_initial_run_2026-04-17.json`

Command:

```bash
timeout -k 10s 28s target/dist/research_harness \
  --cases research/riedel_graph_only_vs_mixed_initial_corpus_2026-04-17.json \
  --format json \
  > tmp/riedel_graph_only_vs_mixed_initial_run_2026-04-17.json
```

### Widened stage

The initial stage stayed controlled:

- total elapsed `8788 ms`;
- `mixed` solved both rungs cleanly;
- `graph_only` stayed at its cheap immediate `unknown` edge.

That justified one widened stage, but no more.

Corpus:

- `research/riedel_graph_only_vs_mixed_widened_corpus_2026-04-17.json`

Rungs:

- `k = 16` with `timeout_ms = 16000`
- `k = 18` with `timeout_ms = 20000`

Run artifact:

- `tmp/riedel_graph_only_vs_mixed_widened_run_2026-04-17.json`

Command:

```bash
timeout -k 10s 46s target/dist/research_harness \
  --cases research/riedel_graph_only_vs_mixed_widened_corpus_2026-04-17.json \
  --format json \
  > tmp/riedel_graph_only_vs_mixed_widened_run_2026-04-17.json
```

The round stopped there because:

- `graph_only` still had no witness at all;
- `mixed` had already hit hard timeouts from `k = 16` onward;
- and pushing beyond `k = 18` would widen the round without answering the
  bead's bounded comparison question any better.

## Fresh results

### Initial stage

| Rung | `graph_only` | `mixed` |
| --- | --- | --- |
| `k = 12` | `unknown` (`1 ms`) | `equivalent`, lag `13` (`2248 ms`) |
| `k = 14` | `unknown` (`1 ms`) | `equivalent`, lag `15` (`6538 ms`) |

### Widened stage

| Rung | `graph_only` | `mixed` |
| --- | --- | --- |
| `k = 16` | `unknown` (`1 ms`) | `timeout` (`16033 ms`) |
| `k = 18` | `unknown` (`1 ms`) | `timeout` (`20033 ms`) |

## Furthest bounded reach

Across the retained ladder plus the fresh staged extension:

| Rung | `graph_only` | `mixed` |
| --- | --- | --- |
| `k = 4` | `unknown` | `equivalent`, lag `5` |
| `k = 6` | `unknown` | `equivalent`, lag `7` |
| `k = 8` | `unknown` | `equivalent`, lag `9` |
| `k = 10` | `unknown` | `equivalent`, lag `11` |
| `k = 12` | `unknown` | `equivalent`, lag `13` |
| `k = 14` | `unknown` | `equivalent`, lag `15` |
| `k = 16` | `unknown` | `timeout` |
| `k = 18` | `unknown` | `timeout` |

So under the tested bounded policy envelope:

- furthest successful `graph_only` reach: none on the tested Riedel lane;
- furthest successful `mixed` reach: `k = 14` with witness lag `15`;
- first `mixed` failure edge in this schedule: `k = 16` at the fixed `16000 ms`
  cap.

## Explicit answer

`graph_only` **falls short of** the current bounded `mixed` baseline on the
Riedel lane.

The precise answer on current `HEAD` is:

- `graph_only` does not exceed `mixed`: it never solves any tested Riedel rung;
- `graph_only` does not match `mixed`: `mixed` solves through `k = 14` under
  the same within-rung bounds;
- `graph_only` falls strictly behind `mixed`: even where `mixed` eventually
  times out (`k = 16` and `k = 18`), `graph_only` is still only immediate
  `unknown`, not a delayed near-edge solve.

The durable conclusion for this bead is therefore:

- the current retained Riedel lane remains a structured-or-mixed family, not a
  graph-only reach family;
- the best bounded `graph_only` reach achieved here is still no solved rung at
  all;
- and no additional bead was opened from this round, because the measurement
  result points to "graph_only lacks the necessary move coverage here" rather
  than to a new bounded probe shape that looks likely to change the answer.
