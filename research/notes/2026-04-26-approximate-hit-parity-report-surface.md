# Approximate-hit parity report surface for square coarse buckets (2026-04-26)

## Question

For bead `sse-rust-bljn`, add the smallest **opt-in** report or telemetry surface
that annotates square `3x3` / `4x4` `approximate_other_side_hits` with the
three-way hoxd action label:

- `reuse_endpoint_local_parity`
- `rank_or_propose_inside_coarse_bucket`
- `ignore`

Hard boundary: reporting only. No default beam ordering, no hard pruning, no
hard dedup, no canonicalization change.

## Surface Kept

Kept surface:

- `search --approximate-hit-parity-report PATH`

Implementation shape:

- factor the square endpoint-local parity helper into
  `src/endpoint_local_parity.rs`;
- reuse the same coarse signature and trimmed active-window descriptor already
  used by the earlier diagnostic note;
- attach a new **observer-only** report path in `src/bin/search.rs`; and
- write a JSON file only when the flag is requested.

Default search behavior is unchanged:

- no search policy reads this report;
- no new telemetry is emitted unless the flag is present; and
- the core frontier logic still only records the existing
  `approximate_other_side_hit` boolean.

The report now also carries an explicit completeness check:

- `missing_approximate_hits`
- `excess_annotated_hits`
- `report_is_complete`
- `completeness_note` when the observer only saw part of the run

The report annotates each discovered `approximate_other_side_hit` record
against the opposite-side coarse bucket currently visible to the observer:

- if the hit is not a square `3x3` / `4x4` state, label it `ignore`;
- if the coarse bucket exists and the trimmed active windows also match, label
  it `reuse_endpoint_local_parity`;
- if the coarse bucket exists but the trimmed active windows differ, label it
  `rank_or_propose_inside_coarse_bucket`.

Exact meets are intentionally excluded so the report stays aligned with the
existing `approximate_other_side_hits` counter.

## Validation Commands

Focused code validation:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test -q --lib endpoint_local_parity
timeout -k 20s 180s cargo test -q --bin search
timeout -k 20s 180s cargo test -q --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms
```

Bounded `k = 3` endpoint-search control:

```bash
timeout -k 20s 180s cargo run -q --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 8 \
  --max-intermediate-dim 4 \
  --max-entry 10 \
  --frontier-mode beam \
  --move-policy graph-plus-structured \
  --beam-width 64 \
  --approximate-hit-parity-report tmp/sse-rust-bljn-k3-approximate-hit-parity.json \
  --json --telemetry
```

Bounded retained `k = 4` stuck lane:

```bash
timeout -k 20s 240s cargo run -q --bin search -- \
  1,4,3,1 1,12,1,1 \
  --max-lag 40 \
  --max-intermediate-dim 4 \
  --max-entry 12 \
  --frontier-mode beam \
  --move-policy graph-plus-structured \
  --beam-width 256 \
  --approximate-hit-parity-report tmp/sse-rust-bljn-k4-approximate-hit-parity.json \
  --json --telemetry
```

Optional negative-control replay that exposed the next boundary:

```bash
timeout -k 20s 240s cargo run -q --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json \
  --max-intermediate-dim 4 \
  --max-entry 5 \
  --guided-max-shortcut-lag 4 \
  --guided-min-gap 2 \
  --guided-max-gap 6 \
  --guided-segment-timeout 5 \
  --guided-rounds 2 \
  --shortcut-max-guides 4 \
  --shortcut-rounds 2 \
  --shortcut-max-total-segment-attempts 64 \
  --approximate-hit-parity-report tmp/sse-rust-bljn-k3-shortcut-replay-approximate-hit-parity.json \
  --json --telemetry
```

## Observed Results

### `k = 3` endpoint-search control

Artifact:

- `tmp/sse-rust-bljn-k3-approximate-hit-parity.json`

Observed summary:

- `telemetry_approximate_other_side_hits = 113`
- `discovered_approximate_hit_records = 113`
- `missing_approximate_hits = 0`
- `excess_annotated_hits = 0`
- `report_is_complete = true`
- `supported_square_hits = 113`
- `hits_by_best_action = { rank_or_propose_inside_coarse_bucket: 113 }`
- `candidate_actions = { rank_or_propose_inside_coarse_bucket: 125 }`

Reading:

- the new surface did annotate the bounded `k = 3` control exactly where the
  existing approximate-hit counter fires;
- on this raw endpoint-search control, every observed square coarse-bucket hit
  stayed in the coarse-only mismatch tier; and
- this run did **not** surface a top-level `reuse_endpoint_local_parity`
  example.

This is a useful negative result to keep. The direct search control does not
automatically inherit the positive replay overlap seen in the earlier
offline diagnostic.

### Retained `k = 4` stuck lane

Artifact:

- `tmp/sse-rust-bljn-k4-approximate-hit-parity.json`

Observed summary:

- `telemetry_approximate_other_side_hits = 184`
- `discovered_approximate_hit_records = 184`
- `missing_approximate_hits = 0`
- `excess_annotated_hits = 0`
- `report_is_complete = true`
- `supported_square_hits = 184`
- `multi_candidate_buckets = 80`
- `hits_by_best_action = { rank_or_propose_inside_coarse_bucket: 184 }`
- `candidate_actions = { rank_or_propose_inside_coarse_bucket: 294 }`

Representative top record:

- coarse signature:
  `d4|sum23|rs3,4,5,11|cs0,0,6,17|rS1,2,2,2|cS0,0,3,4`
- frontier trimmed window:
  `4x2|0,11,2,2,3,2,1,2`
- opposite bucket size: `4`
- every counterpart remained
  `rank_or_propose_inside_coarse_bucket`

Reading:

- this matches the retained `k = 4` story from the earlier hoxd note;
- the signal is useful for annotating the already-matched coarse bucket; but
- it does not create a new exact top tier on this retained lane.

### Shortcut-search replay boundary

Artifact:

- `tmp/sse-rust-bljn-k3-shortcut-replay-approximate-hit-parity.json`

Observed summary:

- top-level telemetry reported `approximate_other_side_hits = 796`
- the opt-in report recorded `discovered_approximate_hit_records = 0`
- `missing_approximate_hits = 796`
- `excess_annotated_hits = 0`
- `report_is_complete = false`

Reading:

- the replay itself succeeded and reproduced the lag-`7` witness; but
- the top-level report only sees events emitted by the outer request; and
- the approximate hits here were produced inside guided shortcut segment
  searches rather than the outer surface.

This is the main integration boundary exposed by the slice.

## Keep / Reject Decision

Decision: **keep the surface narrowly**.

Why it is useful enough to keep:

- it is the smallest search-facing opt-in surface that annotates existing
  `approximate_other_side_hits` without changing default behavior;
- it cleanly covers the retained `k = 4` coarse-bucket lane;
- it preserves the negative result that raw bounded `k = 3` endpoint search
  does not automatically yield `reuse` labels; and
- it keeps the action vocabulary on the actual search CLI instead of only in a
  one-off diagnostic binary.

What it does **not** prove:

- it does not show that top-level bounded `k = 3` endpoint search naturally
  surfaces `reuse_endpoint_local_parity`; and
- it does not yet propagate the annotation surface through nested guided or
  shortcut segment searches.

## Next Integration Boundary

If this work is extended, the next boundary should be:

- thread the same observer/report surface into nested guided-refinement and
  shortcut-search segment searches so the positive `k = 3` replay controls can
  expose their inner approximate-hit stream on the same report surface.

Still do **not**:

- feed the action into default beam ordering;
- use it as hard pruning, hard dedup, or parity filtering; or
- claim it is an SSE invariant or a production canonical form.
