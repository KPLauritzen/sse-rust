# Retained Riedel/Baker `k = 4` widened `graph_only` endpoint-ceiling control (2026-04-19)

## Goal

Freeze one durable widened `graph_only` existence control on the solved
Riedel/Baker lane so later workers can cite and replay the retained `k = 4`
result directly, without re-deriving the witness and without framing it as
progress on the still-open Brix-Ruiz `k = 4` target.

## Chosen surface

Keep exactly one bounded surface:

- endpoints `[[4,2],[1,4]] -> [[3,1],[1,5]]`
- `move_family_policy = graph-only`
- `max_lag = 6`
- `max_intermediate_dim = 3`
- `max_entry = 5`

This is the same endpoint-ceiling surface isolated in
[`2026-04-19-riedel-graph-only-endpoint-ceiling-map.md`](./2026-04-19-riedel-graph-only-endpoint-ceiling-map.md),
but frozen here as a single reusable control instead of only as one rung inside
the wider ladder map.

Why this surface was chosen:

- it is the narrowest retained Riedel/Baker endpoint surface on which plain
  `graph_only` endpoint search recovers `k = 4`;
- it stays on the solved Riedel/Baker control lane rather than drifting toward
  the open Brix-Ruiz `k = 4` lane; and
- it already has an exact witness on current head, so future workers do not
  need to reconstruct sidecar decompositions just to cite the existence result.

## Frozen artifacts

The retained widened witness artifacts are:

- exact endpoint-search JSON:
  [`research/riedel_baker_k4_graph_only_endpoint_ceiling_control_2026-04-19.json`](../riedel_baker_k4_graph_only_endpoint_ceiling_control_2026-04-19.json)
- reusable `full_path` guide artifact:
  [`research/riedel_baker_k4_graph_only_endpoint_ceiling_control_guide_2026-04-19.json`](../riedel_baker_k4_graph_only_endpoint_ceiling_control_guide_2026-04-19.json)

Observed retained result on current head:

- `outcome = equivalent`
- witness lag `6`
- guide validation `witness_validated`

The durable harness-facing control is now:

- [`research/cases.json`](../cases.json):
  `riedel_k4_graph_only_widened_endpoint_ceiling_control`

That case intentionally keeps plain `endpoint_search`; it does **not** depend
on the guide artifact to succeed. The guide artifact is retained so later
workers can inspect or replay the exact witness without searching for it again.

## Reproduce

Build the focused binaries:

```bash
cargo build --profile dist --features research-tools --bin search --bin research_harness
```

Regenerate the retained widened witness and guide:

```bash
timeout -k 5s 20s target/dist/search \
  4,2,1,4 \
  3,1,1,5 \
  --max-lag 6 \
  --max-intermediate-dim 3 \
  --max-entry 5 \
  --move-policy graph-only \
  --json \
  --write-guide-artifact \
  research/riedel_baker_k4_graph_only_endpoint_ceiling_control_guide_2026-04-19.json \
  > research/riedel_baker_k4_graph_only_endpoint_ceiling_control_2026-04-19.json
```

Validate the retained harness control:

```bash
timeout -k 5s 20s target/dist/research_harness \
  --cases research/cases.json \
  --worker-case riedel_k4_graph_only_widened_endpoint_ceiling_control
```

## Reuse guidance

Use this control when you need one stable answer to the question:

- "Does plain widened `graph_only` recover the retained Riedel/Baker `k = 4`
  endpoint pair once the lane is widened exactly to the endpoint ceiling?"

For later workers:

- cite the harness case
  `riedel_k4_graph_only_widened_endpoint_ceiling_control` when you want the
  pass/fail control surface;
- cite the retained guide artifact when you need the explicit six-step witness;
  and
- do **not** cite this as evidence that the still-open Brix-Ruiz `k = 4` Goal 3
  target is solved, or as justification for a broader graph-only ladder remap.

## Keep / Reject

Keep:

- one reusable widened `graph_only` existence control for retained
  Riedel/Baker `k = 4`;
- plain `endpoint_search` as the control surface; and
- the committed guide artifact as the witness-carrying sidecar for direct reuse.

Reject:

- presenting this as Brix-Ruiz `k = 4` progress;
- broadening this note into a full retained-ladder map; and
- turning the slice into a generic graph-only tooling rewrite.
