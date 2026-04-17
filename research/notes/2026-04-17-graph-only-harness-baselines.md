# Graph-only harness baseline set (2026-04-17)

## Question

Which graph-only harness surfaces should become the durable baseline set before
later graph-only optimization, coverage, Riedel, and `k=4` rounds?

This slice is intentionally measurement-first:

- keep the work in `research/cases.json`, harness reporting, and notes;
- do not redesign the solver or frontier implementation here;
- choose surfaces that are bounded enough for the shared corpus but still say
  something durable about graph-only search quality.

## Chosen baseline set

### 1. `brix_ruiz_k3_graph_only`

Role: graph-only **solve baseline**.

Why keep it:

- it is the current hard exact positive for blind graph-only endpoint search in
  the main harness;
- it protects the only reusable graph-only witness surface already tied to the
  repo's most-studied hard family;
- later graph-only optimization work must not trade away exact witness-finding
  on this pair while chasing cheaper `unknown` results elsewhere.

### 2. `brix_ruiz_k3_graph_only_beam_probe`

Role: graph-only **performance baseline**.

Why keep it:

- plain `beam` is still the cheapest stable frontier control on the hard
  `brix_ruiz_k3` graph-only pair;
- it is cheap enough to repeat (`measurement.repeat_runs = 5`) without turning
  the shared corpus into a large benchmark suite;
- it exposes the right telemetry for later graph-only frontier or ranking work:
  elapsed time, frontier layers, visited nodes, and reach-style counters on the
  same hard positive family as the solve baseline.

### 3. `brix_ruiz_k4_graph_only_boundary_ramp`

Role: graph-only **reach baseline**.

Why this new surface:

- later Goal 3 work needs a durable graph-only reach map on the open `k=4`
  Brix-Ruiz endpoint, not just one-off shell commands;
- the `beam64 + dim5 + entry12` graph-only surface is already known to scale to
  deeper lag quickly without introducing structured-factorization noise;
- encoding the lag `20/30/40` sweep as a `deepening_schedule` keeps the case
  reusable and ordered inside the harness instead of leaving it as ad hoc local
  probes.

Why this is still bounded:

- each point is expected to return quickly on current `HEAD`;
- the surface is neutral-scored and non-required, so it informs direction
  without becoming a correctness gate;
- it is deliberately shallower than the earlier deep stress sweeps, which are
  still better left to dedicated branch-local campaigns.

## Why these surfaces are representative

Together these three surfaces cover the three graph-only questions that later
rounds need to keep distinct:

1. Can graph-only still find the known hard witness?
2. Is the cheap frontier control getting faster or more productive under the
   same budget?
3. How far does pure graph reach scale on the open `k=4` family before a
   heavier campaign is justified?

Using the same Brix-Ruiz family for all three is intentional for this first
graph-only lane:

- it keeps frontier policy, move-family policy, and family effects from getting
  entangled too early;
- it reuses the family that already has the strongest repo-local evidence and
  the clearest follow-up beads;
- it gives later workers one exact positive (`k=3`) and one open reach surface
  (`k=4`) without broadening into a multi-family graph-only campaign yet.

## Why not start with Riedel here

`riedel_baker` should still get its own graph-only follow-up, but not in this
first bounded harness slice.

Reason:

- the immediate follow-on beads are graph-only performance work on the hard
  Brix-Ruiz control and bounded graph-only `k=4` work;
- the shared graph-only lane needed one exact positive and one open reach
  baseline before it needed a second family;
- adding a Riedel graph-only ladder now would broaden the corpus faster than it
  would improve current keep/revert decisions.

The later `sse-rust-2uy.20` round can now add Riedel-specific graph-only reach
surfaces against an already-stable graph-only baseline set.

## Validation

Focused harness validation on current `HEAD`:

- `brix_ruiz_k3_graph_only`
- `brix_ruiz_k3_graph_only_beam_probe`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`

Results are recorded after the validation run in this worktree.

Observed results on current `HEAD` with `target/dist/research_harness`:

- `brix_ruiz_k3_graph_only`:
  - `equivalent` in `8991 ms`
  - witness lag `17`
  - `frontier_nodes_expanded = 1,382,998`
  - `total_visited_nodes = 1,399,061`
  - `factorisations_enumerated = 0`
- `brix_ruiz_k3_graph_only_beam_probe`:
  - `unknown` in `111 ms`
  - `frontier_nodes_expanded = 182`
  - `total_visited_nodes = 5,372`
  - `max_frontier_size = 10`
  - `factorisations_enumerated = 0`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_1_lag20_dim5_entry12`:
  - `unknown` in `2578 ms`
  - `frontier_nodes_expanded = 2,354`
  - `total_visited_nodes = 128,118`
  - `approximate_other_side_hits = 18`
  - `factorisations_enumerated = 0`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_2_lag30_dim5_entry12`:
  - `unknown` in `4359 ms`
  - `frontier_nodes_expanded = 3,634`
  - `total_visited_nodes = 219,284`
  - `approximate_other_side_hits = 86`
  - `factorisations_enumerated = 0`
- `brix_ruiz_k4_graph_only_boundary_ramp__deepening_3_lag40_dim5_entry12`:
  - `unknown` in `5923 ms`
  - `frontier_nodes_expanded = 4,914`
  - `total_visited_nodes = 305,954`
  - `approximate_other_side_hits = 232`
  - `factorisations_enumerated = 0`

These validation numbers are the baseline reference point for later
graph-only-first rounds.
