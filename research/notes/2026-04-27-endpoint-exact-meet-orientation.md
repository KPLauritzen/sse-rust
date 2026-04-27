# Endpoint exact-meet orientation capture (2026-04-27)

## Goal

Record the frontier side that produced each retained endpoint exact meet and
expose it through the retained `endpoint_exact_meets` surface and the endpoint
witness inventory.

This stayed narrowly on capture-time telemetry. It did not broaden endpoint
search into enumeration, change ranking or pruning, change canonicalization, or
alter default search behavior for callers that do not request endpoint
multi-meet inventory.

## Orientation vocabulary

The retained value reuses `SearchDirection`:

- `forward`: the source-side frontier expansion produced the exact meet.
- `backward`: the target-side frontier expansion produced the exact meet.

This is a frontier expansion direction, not a new path class.

## Implementation summary

- Capture point: each `retention.retain(...)` call in `src/search.rs` now
  passes the current layer `SearchDirection`.
- Retention storage: `src/search/exact_meets.rs` stores the direction on each
  retained candidate alongside `canonical`, `path_depth`, and
  `discovery_order`.
- Core surface: `EndpointExactMeetWitness` now carries
  `meet_direction: Option<SearchDirection>`. New retained rows always populate
  it; the option keeps the inventory fallback deliberate for any hand-built
  surface without recorded direction.
- JSON surface: `endpoint_exact_meets.retained[]` now includes
  `meet_direction`.
- Inventory: `rows[].endpoint_orientation` now reports `forward` or
  `backward`; `orientation_status` is `recorded` only when every retained row
  has a recorded direction, otherwise it remains `not_recorded`. Empty
  retained surfaces keep `not_recorded` with an explicit not-applicable note.

Before:

```json
{
  "endpoint_exact_meets": {
    "retained": [
      {
        "path_lag": 7,
        "meeting_canonical": [[0, 0], [0, 1]]
      }
    ]
  },
  "orientation_status": "not_recorded",
  "endpoint_orientation": "not_recorded"
}
```

After:

```json
{
  "endpoint_exact_meets": {
    "retained": [
      {
        "path_lag": 7,
        "meet_direction": "forward",
        "meeting_canonical": [[0, 0], [0, 1]]
      }
    ]
  },
  "orientation_status": "recorded",
  "endpoint_orientation": "forward"
}
```

## Commands

Format:

```bash
cargo fmt --all
```

Focused tests:

```bash
timeout -k 20s 180s cargo test --features research-tools endpoint_exact_meets
timeout -k 20s 180s cargo test --features research-tools endpoint_multi_meet
timeout -k 20s 180s cargo test --features research-tools endpoint_witness
```

Build:

```bash
timeout -k 20s 180s cargo build --features research-tools --bin search
```

Bounded hard `k = 3` retained inventory:

```bash
rm -rf tmp/k3_endpoint_witness_orientation_guides_2026-04-27
timeout -k 20s 180s target/debug/search 1,3,2,1 1,6,1,1 \
  --stage endpoint-search \
  --frontier-mode bfs \
  --move-policy graph-plus-structured \
  --max-lag 8 --max-intermediate-dim 4 --max-entry 5 \
  --endpoint-multi-meet-cap 12 \
  --endpoint-witness-inventory tmp/k3_endpoint_witness_orientation_2026-04-27.json \
  --endpoint-witness-control-guide baker=research/guide_artifacts/k3_normalized_guide_pool.json#k3-lind-marcus-baker-lag7 \
  --endpoint-witness-control-guide non_baker=research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --endpoint-witness-guide-dir tmp/k3_endpoint_witness_orientation_guides_2026-04-27 \
  --endpoint-witness-guide-ranks 1,2 \
  --json --telemetry \
  > tmp/k3_endpoint_witness_orientation_run_2026-04-27.result.json
```

Emitted-guide load validation:

```bash
timeout -k 5s 30s target/debug/search 1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifact-dir tmp/k3_endpoint_witness_orientation_guides_2026-04-27 \
  --max-intermediate-dim 4 --max-entry 5 \
  --guided-max-shortcut-lag 1 --guided-min-gap 2 --guided-max-gap 2 \
  --guided-rounds 1 \
  --shortcut-max-guides 2 --shortcut-rounds 1 \
  --shortcut-max-total-segment-attempts 1 \
  --json --telemetry \
  > tmp/k3_endpoint_witness_orientation_guides_load_2026-04-27.result.json
```

## Observed hard k3 inventory

Artifacts:

- `tmp/k3_endpoint_witness_orientation_2026-04-27.json`
- `tmp/k3_endpoint_witness_orientation_run_2026-04-27.result.json`
- `tmp/k3_endpoint_witness_orientation_guides_2026-04-27/`
- `tmp/k3_endpoint_witness_orientation_guides_load_2026-04-27.result.json`

Run summary:

- outcome: `equivalent`
- retained exact meets: `4 / 12`
- telemetry: `frontier_nodes_expanded = 84875`,
  `total_visited_nodes = 235450`, `max_frontier_size = 127212`,
  `layers = 7`
- emitted retained guide artifacts: ranks `1` and `2`
- guide load validation: `guide_artifacts_considered = 2`,
  `guide_artifacts_accepted = 2`, shortcut unique guides `2`

Inventory rows:

| rank | index | meet lag | reconstructed length | orientation | meeting-state signature | full-path hash | control matches |
|---:|---:|---:|---:|---|---|---|---|
| 1 | 0 | 7 | 8 | forward | `4x4:0,0,1,1,1,0,1,2,1,1,2,2,1,1,0,0` | `fnv1a64:1f8ac1c39376a5c9` | none |
| 2 | 1 | 7 | 8 | forward | `4x4:0,0,1,1,2,1,0,2,1,0,0,2,1,1,1,1` | `fnv1a64:cfe5d43ecc73d6a7` | none |
| 3 | 2 | 7 | 8 | forward | `4x4:0,0,1,1,1,0,1,1,1,1,2,0,2,1,2,0` | `fnv1a64:c9ffd25c85117c3c` | none |
| 4 | 3 | 7 | 8 | forward | `4x4:0,0,1,1,0,1,0,1,1,2,0,1,2,2,1,1` | `fnv1a64:7fdff21eda91022f` | none |

The hard `k = 3` retained inventory now records orientation for every retained
row and reports top-level `orientation_status: "recorded"`.

## Decision

Keep the change. It records the exact-meet producing frontier direction at the
point where the meet is retained and exposes that value without changing the
solver's endpoint search behavior or creating a broader enumeration surface.
