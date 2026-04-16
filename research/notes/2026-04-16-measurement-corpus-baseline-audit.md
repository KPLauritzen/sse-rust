# Measurement-only corpus baseline audit (2026-04-16)

## Question

After the required/measurement/evidence lane split and the new round-scorecard
contract, which retained measurement-only baselines in `research/cases.json`
still justify their cost, and which should be reworded, reclassified, or
retired?

## Evidence

Bounded dist-binary checks on the graph-only `k=3` frontier controls:

- plain beam control on `1,3,2,1 -> 1,6,1,1`:
  - `unknown` in `0.13s`
  - `frontier_nodes_expanded = 182`
  - `candidates_generated = 8,334`
  - `total_visited_nodes = 5,372`
- historical depth-`4` `beam_bfs_handoff` losing control:
  - no JSON under `timeout -k 1s 5s`
- deferred-cap follow-ups at `5`, `10`, and `20` with the same default
  handoff depth:
  - no JSON under `timeout -k 1s 5s`
  - still no JSON under `timeout -k 1s 15s`

Single-case `target/dist/research_harness --worker-case` samples on current
`HEAD 81dc77b`:

- `brix_ruiz_k3_graph_plus_structured_probe`:
  - `equivalent` in `6653 ms`
  - `frontier_nodes_expanded = 159,339`
  - `factorisations_enumerated = 665,421`
  - `total_visited_nodes = 512,032`
- `brix_ruiz_k3_shortcut_normalized_pool_probe`:
  - `equivalent` in `1037 ms`
  - `guided_segments_improved = 2`
  - `shortcut_search.promoted_guides = 1`
  - `best_lag_start = 7`, `best_lag_end = 7`
- `brix_ruiz_k3_wide_probe`:
  - `unknown` in `1560 ms`
- `brix_ruiz_k4_probe`:
  - `unknown` in `1290 ms`

## Decisions

- `brix_ruiz_k3_graph_only_beam_probe`: keep, reword.
  The plain beam graph-only control is still the cheapest frontier baseline on
  the hard `k=3` pair and remains the comparison anchor for any future handoff
  work.
- `brix_ruiz_k3_graph_only_beam_bfs_handoff_probe`: keep, reword.
  This is a historical losing control, not an open optimization target. It
  still belongs as the explicit "handoff still loses here" baseline.
- `brix_ruiz_k3_graph_only_beam_bfs_handoff_cap10_probe`: retire.
  On current dist binaries, `cap10` did not differentiate from the losing
  control and failed to emit JSON even under the widened `15s` bound. Shared
  corpus space is better spent on one stable losing control plus the plain beam
  control than on a non-reproducing cap variant.
- `brix_ruiz_k3_graph_plus_structured_probe`: keep, reword.
  This is still the shared intermediate move-policy surface between
  graph-only and broader mixed widening, so it remains worth paying for.
- `brix_ruiz_k3_shortcut_normalized_pool_probe`: keep, reword.
  It is still cheap enough and exposes productive staged-search telemetry on
  the lag-`7` plateau.
- `brix_ruiz_k3_wide_probe`: keep, reclassify to evidence lane.
  This is useful as a cheap mixed wide smoke probe, but it should not sit in
  the required lane or carry timeout penalties.
- `brix_ruiz_k4_probe`: keep, reclassify to evidence lane.
  Same reasoning as the mixed `k=3` wide probe: keep the smoke test, remove the
  accidental gate behavior.
- `brix_ruiz_k4_boundary_ramp`: keep, reword.
  The shared corpus should keep only the cheap deepening map. The heavier
  `beam64 + dim5 + entry10` boundary envelope remains a dedicated local-corpus
  experiment, not a default harness baseline.

## Follow-up

Do not add a wider shared-corpus deferred-cap sweep in this slice. The open
follow-up bead `sse-rust-oaj` already tracks that branch-local experiment if
the handoff surface becomes worth revisiting again.
