# Stratified beam deferred-refill prototype (2026-04-25)

## Question

For bead `sse-rust-nw7.8`, test a small opt-in frontier mode for
`graph_plus_structured` search that keeps the existing scored active beam but
retains a bounded deferred spill frontier for later refill. This is not a BFS
recovery path and does not add any structured move family.

## Frontier Design

New opt-in frontier mode:

`frontier_mode = "stratified_beam_refill"`

The active frontier is the normal scored beam:

- `beam_width` is the active cap;
- entries use the existing `score_node` beam score;
- exact meets and approximate-hit scoring remain the same as plain beam.

When active insertion overflows the beam, the overflow entry is admitted to a
bounded deferred spill frontier instead of being discarded. Deferred entries are
stratified by:

- approximate-hit status; and
- depth.

Each bucket is sorted by the same beam comparator. Refill scans buckets in
approximate-hit-first, shallow-depth order and moves the best pending entry per
bucket into active until active is full.

Caps:

- `beam_bfs_handoff_deferred_cap` is reused as the global deferred cap for this
  experimental mode;
- default global cap is `8 * beam_width` when the field is absent;
- per-bucket cap is deliberately internal and small: `max(1, beam_width / 2)`;
- refill threshold is `max(1, beam_width / 4)`;
- refill reason is recorded as either active exhausted or active below
  threshold.

Telemetry is attached under `telemetry.stratified_beam_refill`:

- active admissions;
- deferred admissions;
- drops by bucket cap;
- drops by global cap;
- refill count;
- refill reason counts;
- refill admissions;
- final active/deferred frontier counts for completed runs.

Existing aggregate telemetry continues to report exact meets
(`collisions_with_other_frontier`), approximate hits, frontier nodes expanded,
visited nodes, and elapsed time.

## A/B Commands

Corpus:

`tmp/nw78_stratified_beam_refill_ab_cases.json`

Primary run:

```bash
timeout -k 20s 240s target/debug/research_harness \
  --cases tmp/nw78_stratified_beam_refill_ab_cases.json \
  --format json \
  > tmp/nw78_stratified_beam_refill_ab_run_wide_timeout.json
```

Focused k4 lag20 controls:

```bash
timeout -k 20s 120s target/debug/research_harness \
  --cases tmp/nw78_k4_lag20_beam_handoff_controls.json \
  --format json \
  > tmp/nw78_k4_lag20_beam_handoff_controls_run.json
```

Focused k4 lag20 refill cap16:

```bash
timeout -k 20s 90s target/debug/research_harness \
  --cases tmp/nw78_stratified_beam_refill_k4_lag20_cap16_case.json \
  --format json \
  > tmp/nw78_stratified_beam_refill_k4_lag20_cap16_run.json
```

After fixing deferred-admission and refresh cap accounting, the completed
refill rows were rerun and the tables below use:

- `tmp/nw78_k3_stratified_refill_post_review.json`
- `tmp/nw78_stratified_beam_refill_k4_lag20_cap16_post_review.json`

Timed-out probes retained as negative signal:

- `tmp/nw78_stratified_beam_refill_ab_run.json`
- `tmp/nw78_stratified_beam_refill_k4_cap256_run.json`
- `tmp/nw78_stratified_beam_refill_k4_lag20_cap256_run.json`

## Metrics

### `k=3` graph-plus-structured control

All cases used Brix-Ruiz `k=3`, `max_lag=8`, `dim4`, `entry5`,
`beam_width=10`.

| Mode | Outcome | Time | Frontier | Visited | Exact meets | Approx. hits | Max frontier | Active admissions | Deferred admissions | Drops bucket | Drops global | Refills |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| beam | `unknown` | `522 ms` | `142` | `2631` | `0` | `10` | `10` | `0` | `0` | `0` | `0` | `0` |
| beam_bfs_handoff | `timeout` | `15026 ms` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` |
| stratified_beam_refill cap80 | `unknown` | `5221 ms` | `1375` | `16776` | `0` | `65` | `50` | `4567` | `2671` | `14712` | `0` | `52` |

Refill reasons for the stratified run: `43` exhausted, `9` below-threshold.
It improved approximate-hit count versus beam (`65` vs `10`) but did not find a
witness.

### Retained Brix-Ruiz `k=4` lag40 lane

All cases used `graph_plus_structured`, `beam_width=256`, `dim4`, `entry12`.

| Mode | Outcome | Time | Frontier | Visited | Exact meets | Approx. hits | Max frontier |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| beam lag40 | `unknown` | `23506 ms` | `19970` | `176664` | `0` | `184` | `256` |
| beam_bfs_handoff lag40 cap2048 | `timeout` | `60132 ms` | `0` | `0` | `0` | `0` | `0` |
| stratified_beam_refill lag40 cap2048 | `timeout` | `60027 ms` | `0` | `0` | `0` | `0` | `0` |

The harness kills timed-out workers, so partial telemetry is not available for
the lag40 timeout rows.

### Narrow k4 lag20 probe

To get returned telemetry without broadening the retained envelope, the refill
cap was narrowed to `16` and lag reduced to `20`.

| Mode | Outcome | Time | Frontier | Visited | Exact meets | Approx. hits | Max frontier | Active admissions | Deferred admissions | Drops bucket | Drops global | Refills |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| beam lag20 | `unknown` | `13755 ms` | `9730` | `112521` | `0` | `126` | `256` | `0` | `0` | `0` | `0` | `0` |
| beam_bfs_handoff lag20 cap16 | `timeout` | `60244 ms` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` | `0` |
| stratified_beam_refill lag20 cap16 | `unknown` | `43354 ms` | `41950` | `267739` | `0` | `414` | `272` | `97414` | `2377` | `0` | `222384` | `13` |

The cap16 refill run exposed more approximate overlap (`414` vs `126`) but cost
about `3.1x` the beam runtime and `4.3x` the frontier expansions, with no exact
meet.

## Reading

The prototype does what it was meant to do mechanically: small active beams no
longer die as early, and deferred refill materially increases approximate-hit
surface on the k3 control and the narrow k4 lag20 probe.

The result is still negative for the retained Brix-Ruiz k4 lane:

- no exact meets were found;
- lag40 with a natural `8 * beam_width` deferred cap timed out;
- cap256 still timed out even at lag20;
- cap16 returned at lag20 but was far slower than plain beam for approximate
  hits only.

Recommendation: keep the code as an opt-in research-only frontier surface, but
do not promote it as a default or as the next retained k4 strategy. If this is
continued, narrow the refill policy before spending more runs: refill only
approximate-hit buckets, only late-depth buckets, or only a fixed number of
entries per side after active exhaustion. Do not broaden deferred caps to make
lag40 pass.

No follow-up bead was opened; the next move is not yet strong enough beyond
the narrowing options above.

## Validation

```bash
timeout -k 20s 180s cargo test --features research-tools search --no-run
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools search
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
```
