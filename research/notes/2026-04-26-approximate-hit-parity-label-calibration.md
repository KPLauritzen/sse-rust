# Approximate-hit parity label calibration (2026-04-26)

## Question

For bead `sse-rust-wjw8`, calibrate the propagated approximate-hit parity
labels against exact segment outcomes:

- `reuse_endpoint_local_parity`
- `rank_or_propose_inside_coarse_bucket`
- `ignore`

This slice is reporting/evaluation only. It does not change beam ordering,
pruning, deduplication, canonicalization, or move generation, and it does not
claim endpoint-local parity is an SSE invariant.

The specific decision is whether the labels are merely descriptive, or
predictive enough to justify a later opt-in ranking/report surface.

## Exact outcome linkage

The propagated report gives a direct exact outcome per search scope through
`search_scopes[].result`. For nested guided/shortcut segment searches, each
child `endpoint_search` scope is a segment attempt, so I compare annotated hits
inside that scope with the scope result:

- `result == "equivalent"`: exact segment search succeeded;
- `result == "unknown"`: exact segment search did not find a witness under the
  bounded segment config.

This is exact at the segment-scope level, but not at the individual-hit or
final-guide-edge level. The report does not currently record guide gap
start/end indices, a segment-improvement identifier, or which exact segment was
used in the final stitched/promoted guide. Therefore the calibration below is a
bounded scope-level comparison, not a claim that an individual approximate hit
became an exact witness.

The direct endpoint-search `k = 4` stuck lane also exposes a report-shape
limitation after propagation: it writes an aggregate-complete report, but the
scope list contains an unfinished wrapper scope and a finished child
`endpoint_search` scope for the same request. I use the finished scope for
scope/action counts and record the limitation explicitly.

## Commands and artifacts

### Exact `k = 3` shortcut replay control

Command:

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
  --approximate-hit-parity-report tmp/sse-rust-wjw8-k3-shortcut-replay-approximate-hit-parity.json \
  --json --telemetry \
  > tmp/sse-rust-wjw8-k3-shortcut-replay-output.json
```

Artifacts:

- `tmp/sse-rust-wjw8-k3-shortcut-replay-output.json`
- `tmp/sse-rust-wjw8-k3-shortcut-replay-approximate-hit-parity.json`

Observed run result:

- outcome: `equivalent`
- returned lag: `7`
- shortcut guides accepted: `4`
- segment attempts: `64`
- segment cache misses: `63`
- segment cache hits: `1`
- segment improvements: `1`
- promoted guides: `1`
- stop reason: `max_segment_attempts_reached`

Report summary:

- `telemetry_approximate_other_side_hits = 796`
- `discovered_approximate_hit_records = 796`
- `missing_approximate_hits = 0`
- `excess_annotated_hits = 0`
- `report_is_complete = true`
- `search_scopes_observed = 64`
- `nested_search_scopes_observed = 63`
- `complete_search_scopes = 64`
- `incomplete_search_scopes = 0`
- `supported_square_hits = 796`
- `unsupported_hits = 0`
- `multi_candidate_buckets = 196`
- `hits_by_best_action = { rank_or_propose_inside_coarse_bucket: 789, reuse_endpoint_local_parity: 7 }`
- `candidate_actions = { rank_or_propose_inside_coarse_bucket: 1029, reuse_endpoint_local_parity: 7 }`

Scope/stage counts:

| Stage | Scopes | Result | Approximate-hit records | Label counts |
| --- | ---: | --- | ---: | --- |
| `shortcut_search` | 1 | `equivalent` | 0 exclusive, 796 child | none at parent scope |
| `endpoint_search` | 19 | `equivalent` | 20 | `rank_or_propose_inside_coarse_bucket: 20` |
| `endpoint_search` | 44 | `unknown` | 776 | `rank_or_propose_inside_coarse_bucket: 769`, `reuse_endpoint_local_parity: 7` |

Hit-bearing scope counts by exact outcome:

| Exact segment result | Label | Hit-bearing scopes | Hits |
| --- | --- | ---: | ---: |
| `equivalent` | `rank_or_propose_inside_coarse_bucket` | 5 | 20 |
| `unknown` | `rank_or_propose_inside_coarse_bucket` | 14 | 769 |
| `unknown` | `reuse_endpoint_local_parity` | 4 | 7 |

The `reuse_endpoint_local_parity` hits all occurred in unknown segment scopes:

- scope `6`: 1 reuse hit, 84 rank/propose hits, result `unknown`
- scope `30`: 2 reuse hits, 92 rank/propose hits, result `unknown`
- scope `31`: 2 reuse hits, 133 rank/propose hits, result `unknown`
- scope `51`: 2 reuse hits, 92 rank/propose hits, result `unknown`

Representative reuse records were backward `insplit` hits at layer `3` with a
single bucket candidate and exact trimmed-active-window equality. The two
observed reuse coarse signatures were:

- `d4|sum15|rs3,3,4,5|cs0,4,5,6|rS1,2,2,3|cS0,2,3,3`
- `d4|sum15|rs3,3,4,5|cs0,5,5,5|rS1,2,2,2|cS0,1,3,3`

### Retained `k = 4` Brix-Ruiz stuck lane

Command:

```bash
timeout -k 20s 240s cargo run -q --bin search -- \
  1,4,3,1 1,12,1,1 \
  --max-lag 40 \
  --max-intermediate-dim 4 \
  --max-entry 12 \
  --frontier-mode beam \
  --move-policy graph-plus-structured \
  --beam-width 256 \
  --approximate-hit-parity-report tmp/sse-rust-wjw8-k4-retained-stuck-lane-approximate-hit-parity.json \
  --json --telemetry \
  > tmp/sse-rust-wjw8-k4-retained-stuck-lane-output.json
```

The command returned the search binary's expected nonzero `unknown` exit code,
while still writing both JSON artifacts.

Artifacts:

- `tmp/sse-rust-wjw8-k4-retained-stuck-lane-output.json`
- `tmp/sse-rust-wjw8-k4-retained-stuck-lane-approximate-hit-parity.json`

Observed run result:

- outcome: `unknown`
- stage: `endpoint_search`
- approximate other-side hits: `184`
- exact meets by every reported move family: `0`

Report summary:

- `telemetry_approximate_other_side_hits = 184`
- `discovered_approximate_hit_records = 184`
- `missing_approximate_hits = 0`
- `excess_annotated_hits = 0`
- `report_is_complete = true`
- `search_scopes_observed = 2`
- `nested_search_scopes_observed = 1`
- `complete_search_scopes = 1`
- `incomplete_search_scopes = 1`
- `supported_square_hits = 184`
- `unsupported_hits = 0`
- `multi_candidate_buckets = 80`
- `hits_by_best_action = { rank_or_propose_inside_coarse_bucket: 184 }`
- `candidate_actions = { rank_or_propose_inside_coarse_bucket: 294 }`

Scope/stage counts:

| Scope | Stage | Result | Approximate-hit records | Label counts | Note |
| ---: | --- | --- | ---: | --- | --- |
| 1 | `endpoint_search` | unfinished | 0 | none | wrapper scope; no finish accounting |
| 2 | `endpoint_search` | `unknown` | 184 | `rank_or_propose_inside_coarse_bucket: 184` | finished child scope used for calibration |

This repeats the earlier retained `k = 4` reading: the label describes a real
coarse-bucket near miss, but there is no exact segment or endpoint success in
this bounded lane.

### Supplemental retained `k = 4` positive replay

Command:

```bash
timeout -k 20s 60s cargo run -q --bin search -- \
  3x3:1,3,1,1,3,0,2,6,4 \
  3x3:4,4,4,1,1,1,0,1,3 \
  --stage guided-refinement \
  --guide-artifacts research/riedel_k4_retained_interior_bridge_entry5_threshold_guide_2026-04-18.json \
  --max-lag 3 \
  --max-intermediate-dim 3 \
  --max-entry 4 \
  --move-policy graph-only \
  --guided-max-shortcut-lag 1 \
  --guided-min-gap 2 \
  --guided-max-gap 3 \
  --guided-rounds 1 \
  --approximate-hit-parity-report tmp/sse-rust-wjw8-riedel-k4-guided-replay-approximate-hit-parity.json \
  --json --telemetry \
  > tmp/sse-rust-wjw8-riedel-k4-guided-replay-output.json
```

Artifacts:

- `tmp/sse-rust-wjw8-riedel-k4-guided-replay-output.json`
- `tmp/sse-rust-wjw8-riedel-k4-guided-replay-approximate-hit-parity.json`

Observed result:

- outcome: `equivalent`
- guided segment attempts: `3`
- guided segment improvements: `0`
- approximate other-side hits: `0`
- report complete: `true`
- label counts: none

This positive replay is not useful for label calibration because there were no
approximate hits to annotate. It does show that exact `k = 4` retained replay
success can occur without any approximate-hit parity label firing.

## Predictiveness reading

On the exact `k = 3` shortcut replay, `reuse_endpoint_local_parity` is rare but
not predictive of exact segment success:

- hit-level exact-scope rate for `reuse_endpoint_local_parity`: `0 / 7`
- hit-level exact-scope rate for `rank_or_propose_inside_coarse_bucket`: `20 / 789`
- every reuse hit was in an `unknown` child `endpoint_search` scope
- no reuse hit appeared in the 19 equivalent child segment scopes

`rank_or_propose_inside_coarse_bucket` is descriptive, not cleanly predictive:

- it appears in exact-success scopes, but most rank/propose hits are in unknown
  scopes;
- on the retained `k = 4` stuck lane it accounts for all `184` hits while the
  exact endpoint result is still `unknown`; and
- its current value is explaining coarse-bucket structure, not identifying
  successful exact segments.

`ignore` cannot be calibrated on this slice:

- no best-action `ignore` hits were emitted in either hit-bearing report;
- all observed approximate hits were supported square states with at least one
  coarse-bucket candidate; and
- therefore there is no positive or negative exact-outcome evidence for
  `ignore` beyond absence.

## Decision

Decision: **reject promotion beyond diagnostics for now**.

The labels are useful descriptive report fields, but this bounded calibration
does not justify a later opt-in ranking or ranking-like report surface yet:

- `reuse_endpoint_local_parity` is rare and landed only in unknown segment
  scopes in the exact `k = 3` replay;
- `rank_or_propose_inside_coarse_bucket` mostly marks coarse-bucket near misses,
  including the retained `k = 4` stuck lane with no exact meets;
- `ignore` was absent, so it has no predictive calibration here; and
- accepted final guide extraction is not linkable per hit in the current report
  schema.

Keep the scoped report and the action vocabulary as diagnostics. Do not create
ranking, pruning, deduplication, canonicalization, or move-generation work from
this slice. A future follow-up would need a more precise per-segment accepted
edge linkage and a positive held-out run where reuse labels concentrate in
exact-success scopes; this slice does not provide that evidence.

## Validation

Local validation:

```bash
timeout -k 20s 120s cargo fmt --all
```

Closeout branch review command:

```bash
roborev review --branch --wait --base main
```
