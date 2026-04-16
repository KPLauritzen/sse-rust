# k=3 runtime round: borrowed-key move-family telemetry accumulator (2026-04-16)

## Question
Can the current hot move-family telemetry bookkeeping avoid per-update `String`
allocation/cloning inside search expansion while keeping the public JSON
telemetry keyed by `String`, and does that pay for itself on the current hard
control plus the full harness?

## Profiling-first setup

Rebuilt `target/dist/search` with `pprof` support first, then reran the same
bounded controls used in the merged runtime rounds:

1. mixed endpoint-search `brix_ruiz_k3`
2. current hard shortcut control reduced to
   `shortcut_max_total_segment_attempts=4` so the profile stayed bounded

Fresh rebuilt-binary profiles still matched the prior lead:

- mixed `brix_ruiz_k3` remained dominated by `square_factorisation_3x3`, but
  the sample still caught `move_family_telemetry_mut` on the expansion path
- the bounded hard shortcut sample again hit `move_family_telemetry_mut`,
  including a sampled `BTreeMap::entry` frame on current `HEAD`
- the bounded hard sample telemetry itself stayed identical to the prior round:
  factorisations `336,161`, generated `350,249`, visited `14,669`

That kept the task narrow: change only the internal accumulator shape, not
pruning, ranking, or search policy.

## Change

In `src/search.rs` and `src/search/frontier.rs`:

- introduced an internal `AHashMap<&'static str, SearchMoveFamilyTelemetry>`
  accumulator for hot move-family updates
- kept `FrontierExpansionStats` and graph-only layer bookkeeping on borrowed
  family labels during expansion
- converted back to the public `BTreeMap<String, SearchMoveFamilyTelemetry>`
  only when finalizing `SearchLayerTelemetry` or merging into aggregate public
  telemetry

The public JSON/output shape stays unchanged.

## Direct hard-control check

### Bounded `pprof` control (`shortcut_max_total_segment_attempts=4`)

- baseline current `HEAD` sample still showed `move_family_telemetry_mut` via
  `BTreeMap::entry`
- patched sample still hit the helper, but through `hashbrown::rustc_entry`
  instead; the sampled `BTreeMap::entry` frame disappeared
- useful work counters stayed identical before and after:
  - factorisations `336,161`
  - generated `350,249`
  - visited `14,669`

Interpretation: the bookkeeping change stayed off the search-policy surface on
the fixed-work profile control.

### Attempts-8 hard shortcut validation

Command family:

```bash
target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 7 \
  --stage shortcut-search \
  --max-intermediate-dim 5 \
  --max-entry 5 \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --guided-max-shortcut-lag 5 \
  --guided-min-gap 2 --guided-max-gap 5 \
  --guided-segment-timeout 3 \
  --guided-rounds 2 \
  --shortcut-max-guides 8 \
  --shortcut-rounds 2 \
  --shortcut-max-total-segment-attempts 8 \
  --json --telemetry
```

Same-session baseline on the pre-change tree:

- wall `6.83s`
- outcome `equivalent`, lag `7`
- stop reason `max_segment_attempts_reached`
- segment attempts / guide improvements / promoted `8 / 0 / 0`
- factorisations `1,237,024`
- visited `37,409`

Patched:

- wall `6.55s`
- outcome `equivalent`, lag `7`
- stop reason `max_segment_attempts_reached`
- segment attempts / guide improvements / promoted `8 / 0 / 0`
- factorisations `845,877`
- visited `24,168`

The attempts-limited shortcut surface remains wall-clock-sensitive, so work
counters can move even when policy does not. The keep gate here is unchanged
lag/outcome/guide results plus lower elapsed time.

## Aggregate harness confirmation

The saved `research/runs/` baseline artifacts referenced by the earlier notes
were not present in this checkout, so I compared against a clean snapshot of
commit `c70d511` exported into `tmp/5qq-baseline-snapshot/` and built via an
explicit `--manifest-path`.

Baseline snapshot harness reruns:

- `tmp/sse-rust-5qq-harness.baseline.json`: required `23/23`, target hits `22`,
  points `3795`, telemetry-focus `69,496,257`, elapsed `22,722 ms`
- `tmp/sse-rust-5qq-harness.baseline.r2.json`: required `23/23`, target hits
  `22`, points `3795`, telemetry-focus `69,496,257`, elapsed `22,712 ms`

Patched harness reruns:

- `tmp/sse-rust-5qq-harness.patch.r2.json`: required `23/23`, target hits `22`,
  points `3795`, telemetry-focus `69,496,257`, elapsed `22,631 ms`
- `tmp/sse-rust-5qq-harness.patch.r3.json`: required `23/23`, target hits `22`,
  points `3795`, telemetry-focus `69,496,257`, elapsed `22,686 ms`

Relevant case-level timing shape stayed close:

- `brix_ruiz_k3_graph_only`: baseline `8,591 / 8,573 ms`, patched
  `8,527 / 8,533 ms`
- `brix_ruiz_k3_shortcut_seeded`: baseline `45 / 45 ms`, patched `48 / 45 ms`

Measurement note:

- one earlier patched harness run using `--reuse-dir research/runs` landed at
  `24,960 ms`, but this checkout has no local `research/runs/` directory and
  the exact no-reuse reruns above were stable; use the apples-to-apples no-reuse
  comparisons as the keep gate

## Decision

Keep.

This internal accumulator refactor removes hot-path family-label
allocation/cloning, preserves the public telemetry JSON shape, improves the
current hard shortcut control, and stays slightly ahead of the clean same-session
harness baseline without changing useful reach.
