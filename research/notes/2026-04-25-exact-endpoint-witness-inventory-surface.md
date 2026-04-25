# Exact endpoint witness inventory surface (2026-04-25)

## Goal

Turn the retained `endpoint_exact_meets` output into a reusable compact
inventory for the hard Brix-Ruiz `k = 3` exact endpoint pair
`[[1,3],[2,1]] -> [[1,6],[1,1]]`.

This stayed on the bounded exact-endpoint multi-meet surface. It did not reopen
broad path enumeration or shortcut replay search.

## Commands

Build:

```bash
cargo build --features research-tools --bin search --bin research_harness
```

Focused hard endpoint inventory run:

```bash
rm -rf tmp/k3_endpoint_witness_inventory_guides_2026-04-25
timeout -k 10s 180s target/debug/search 1,3,2,1 1,6,1,1 \
  --stage endpoint-search \
  --frontier-mode bfs \
  --move-policy graph-plus-structured \
  --max-lag 8 --max-intermediate-dim 4 --max-entry 5 \
  --endpoint-multi-meet-cap 12 \
  --endpoint-witness-inventory tmp/k3_endpoint_witness_inventory_2026-04-25.json \
  --endpoint-witness-control-guide baker=research/guide_artifacts/k3_normalized_guide_pool.json#k3-lind-marcus-baker-lag7 \
  --endpoint-witness-control-guide non_baker=research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --endpoint-witness-guide-dir tmp/k3_endpoint_witness_inventory_guides_2026-04-25 \
  --endpoint-witness-guide-ranks 1,2 \
  --json --telemetry \
  > tmp/k3_endpoint_witness_inventory_run_2026-04-25.result.json
```

Emitted-guide load validation:

```bash
timeout -k 5s 30s target/debug/search 1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifact-dir tmp/k3_endpoint_witness_inventory_guides_2026-04-25 \
  --max-intermediate-dim 4 --max-entry 5 \
  --guided-max-shortcut-lag 1 --guided-min-gap 2 --guided-max-gap 2 \
  --guided-rounds 1 \
  --shortcut-max-guides 2 --shortcut-rounds 1 \
  --shortcut-max-total-segment-attempts 1 \
  --json --telemetry \
  > tmp/k3_endpoint_witness_inventory_guides_load_2026-04-25.result.json
```

## Observed run summary

- outcome: `equivalent`
- retained exact meets: `4 / 12`
- telemetry: `frontier_nodes_expanded = 84875`,
  `total_visited_nodes = 235450`, `max_frontier_size = 127212`,
  `layers = 7`
- emitted retained guide artifacts: ranks `1` and `2`
- guide load validation: `guide_artifacts_considered = 2`,
  `guide_artifacts_accepted = 2`, shortcut unique guides `2`

## Inventory rows

The inventory records meet lag separately from reconstructed path length. On
this run every retained meet has meet lag `7`, while every reconstructed
source-to-target witness path has length `8`.

| rank | index | meet lag | reconstructed length | orientation | meeting-state signature | full-path hash | control matches |
|---:|---:|---:|---:|---|---|---|---|
| 1 | 0 | 7 | 8 | not_recorded | `4x4:0,0,1,1,1,0,1,2,1,1,2,2,1,1,0,0` | `fnv1a64:1f8ac1c39376a5c9` | none |
| 2 | 1 | 7 | 8 | not_recorded | `4x4:0,0,1,1,2,1,0,2,1,0,0,2,1,1,1,1` | `fnv1a64:cfe5d43ecc73d6a7` | none |
| 3 | 2 | 7 | 8 | not_recorded | `4x4:0,0,1,1,1,0,1,1,1,1,2,0,2,1,2,0` | `fnv1a64:c9ffd25c85117c3c` | none |
| 4 | 3 | 7 | 8 | not_recorded | `4x4:0,0,1,1,0,1,0,1,1,2,0,1,2,2,1,1` | `fnv1a64:7fdff21eda91022f` | none |

Loaded controls:

| class | artifact | reconstructed length | full-path hash |
|---|---|---:|---|
| baker | `k3-lind-marcus-baker-lag7` | 7 | `fnv1a64:8099084adeaf131e` |
| non_baker | `search-shortcut_search-lag-7` | 7 | `fnv1a64:9fc19e9cedcc01c7` |

## Baker / non-Baker distinction

The chosen matrix-sequence signature and stable FNV-1a hash distinguish the two
pinned lag-7 controls cleanly:

- Baker control hash: `fnv1a64:8099084adeaf131e`
- non-Baker control hash: `fnv1a64:9fc19e9cedcc01c7`

The retained exact-meet witnesses from this endpoint run do not exactly match
either pinned lag-7 control. That is expected for this surface: the retained
exact meets reconstruct as length-8 source-to-target witnesses, while both
controls are length-7 guide artifacts.

## Limitations

- Endpoint orientation is not available in the retained telemetry today. The
  inventory therefore reports `not_recorded` instead of inferring the frontier
  side or meet orientation.
- Classification is exact full-path matrix-sequence matching against loaded
  guide controls. It does not classify quotient-equivalent, prefix-related, or
  replay-derived witnesses.
- The stable hash is an inventory key for comparison and reporting, not a
  mathematical invariant.

## Decision

Keep the surface.

It gives a compact, reusable view over the retained exact-endpoint multi-meet
surface without changing default search behavior and without treating meet lag
as reconstructed path length. It also supports selected guide-artifact emission,
and those emitted artifacts load through the existing guide-artifact path.

A follow-up bead is justified if endpoint orientation becomes important for
analysis: `sse-rust-379`. The narrow follow-up is to store the retained exact
meet side or orientation at capture time and expose it in this inventory, not to
broaden endpoint search into enumeration.
