# Graph-plus-structured harness baseline set (2026-04-17)

## Question

Which graph-plus-structured harness surfaces should become the durable baseline
set before later graph-plus-structured optimization and campaign rounds?

This slice is intentionally measurement-first:

- keep the work in `research/cases.json`, harness reporting, and notes;
- do not redesign the solver or search defaults here;
- choose surfaces that are bounded enough for the shared corpus but still say
  something durable about graph-plus-structured search quality.

## Chosen baseline set

### 1. `brix_ruiz_k3_graph_plus_structured`

Role: graph-plus-structured **solve baseline**.

Why keep it:

- it is the current hard exact positive for blind endpoint search on the main
  `brix_ruiz_k3` family under the intermediate `graph_plus_structured` policy;
- it turns the old generic probe into an explicit witness guard rather than a
  one-off measurement case;
- tightening the bound from `max_entry = 6` to `max_entry = 5` keeps the same
  witness lag while cutting the baseline cost enough for routine reuse:
  a bounded current-`HEAD` probe dropped from `5355 ms` to `2163 ms` while
  retaining the lag-`8` witness.

### 2. `brix_ruiz_k3_graph_plus_structured_beam_probe`

Role: graph-plus-structured **performance baseline**.

Why this new surface:

- it uses the same endpoint family, move-family policy, and bounded
  `lag8 + dim4 + entry5` envelope as the solve baseline, so later deltas stay
  directly comparable;
- `frontier_mode = beam` with `beam_width = 10` is cheap enough to repeat
  (`measurement.repeat_runs = 5`) without turning the shared corpus into a
  heavyweight benchmark suite;
- it exposes the right telemetry for later graph-plus-structured work:
  elapsed time, frontier size, visited nodes, factorisation counts, and
  bounded reach signals on the same hard family as the exact positive.

## Why these surfaces are representative

Together these two surfaces cover the first two graph-plus-structured
questions that later rounds need to keep distinct:

1. Can the bounded intermediate move-family policy still find the hard exact
   `brix_ruiz_k3` witness?
2. Under the same family envelope, is the cheap frontier control getting
   faster or more productive before heavier graph-plus-structured campaigns are
   justified?

Keeping both surfaces on the same hard `brix_ruiz_k3` pair is intentional for
this first graph-plus-structured lane:

- `research/program.md` already points workers to `graph_plus_structured` when
  a round wants more than graph-only and less than full mixed widening;
- the hard `k=3` Brix-Ruiz pair is still the repo's clearest shared endpoint
  for comparing exact witness quality against bounded budget controls;
- using one family and one envelope keeps frontier, move-family, and
  family-choice effects from getting entangled too early.

## Why not start with a graph-plus-structured `k=4` ramp here

An open `k=4` graph-plus-structured reach map is still worth a follow-up, but
not in this first bounded harness slice.

Reason:

- the immediate follow-on work is about keeping one exact graph-plus-structured
  witness lane and one cheap performance lane stable before optimization;
- there is already strong repo-local evidence that the hard `k=3` family is
  the main comparison anchor for graph-only, mixed, and shortcut rounds;
- adding a shared-corpus `k=4` graph-plus-structured ramp now would broaden
  the default corpus before there is matching evidence that a bounded `k=4`
  surface is cheap and decision-useful enough to keep.

The next graph-plus-structured reach round can now start from an explicit
solve/performance baseline pair instead of a single generic probe.

## Validation

Focused harness validation on current `HEAD`:

- `brix_ruiz_k3_graph_plus_structured`
- `brix_ruiz_k3_graph_plus_structured_beam_probe`

Results are recorded after the validation run in this worktree.

Observed results on current `HEAD` with `target/dist/research_harness`:

- `brix_ruiz_k3_graph_plus_structured`:
  - `equivalent` in `2163 ms`
  - witness lag `8`
  - `frontier_nodes_expanded = 84,875`
  - `total_visited_nodes = 212,170`
  - `factorisations_enumerated = 470,662`
  - `approximate_other_side_hits = 1,237`
- `brix_ruiz_k3_graph_plus_structured_beam_probe`:
  - `unknown` with measurement `warmups=1 repeats=5`
  - elapsed samples `51 / 53 / 53 / 54 / 62 ms`
  - median `53 ms`, `p90 = 62 ms`
  - `frontier_nodes_expanded = 142`
  - `total_visited_nodes = 2,631`
  - `factorisations_enumerated = 22,301`
  - `approximate_other_side_hits = 10`

These validation numbers are the baseline reference point for later
graph-plus-structured-first rounds.
