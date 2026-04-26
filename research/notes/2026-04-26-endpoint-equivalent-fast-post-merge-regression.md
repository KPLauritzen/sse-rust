# endpoint_equivalent_fast post-merge regression (2026-04-26)

## Question

Bead `sse-rust-wjaj` investigated the post-merge Criterion regression on
`endpoint_equivalent_fast` after `mvst-nested-parity-report` merged.

The benchmark calls `search_sse_2x2` on a 2x2 permutation-equivalent endpoint
pair:

- source `[[2, 1], [1, 1]]`
- target `[[1, 1], [1, 2]]`
- BFS, mixed policy, lag 4, max intermediate dim 2, max entry 10

## Measurement

The first scratch comparison used an explicit parent worktree at `b066bfb` and
explicit `--manifest-path` because this workmux setup shims `cargo` and a nested
worktree under `tmp/` otherwise resolved to the parent manifest.

Parent baseline:

```bash
timeout -k 20s 900s env CARGO_TARGET_DIR=tmp/wjaj-bench-target \
  cargo bench --manifest-path tmp/wjaj-b066bfb/Cargo.toml \
  --bench search endpoint_equivalent_fast -- --noplot --save-baseline b066bfb
```

Observed parent estimate:

- `endpoint_equivalent_fast`: `[2.7256, 2.7491, 2.7726] us`

Current merge before the fix, built in a fresh target:

```bash
timeout -k 20s 900s env CARGO_TARGET_DIR=tmp/wjaj-current-target \
  cargo bench --manifest-path Cargo.toml \
  --bench search endpoint_equivalent_fast -- --noplot --save-baseline current-fresh
```

Observed merge estimate:

- `endpoint_equivalent_fast`: `[2.8893, 2.9118, 2.9355] us`

This was a real compiled-code-path regression on the isolated benchmark, not
just a stale Criterion baseline comparison.

## Cause

The benchmark reaches the result-only `search_sse_2x2` API, not the CLI or an
observer-enabled search. The reachable outcome is the existing immediate
permutation shortcut after invariant checks.

I did not find new observer event work on the no-observer fast path. The direct
cost was that the result-only API always entered the telemetry/observer-capable
implementation first, paying dynamic endpoint/request/canonical setup before
returning a result that does not expose telemetry. The nested observer/parity
merge made that micro surface slower enough for Criterion to flag, even though
the default solver result was unchanged.

## Change

Kept the same solver semantics and added a result-only fast path inside
`search_sse_2x2` for outcomes that the telemetry implementation already returns
before frontier expansion:

- invalid endpoint multi-meet config -> `Unknown`
- non-supported stratified-beam-refill dim config -> `Unknown`
- identity endpoint -> zero-step equivalent path
- invariant rejection -> `NotEquivalent`
- 2x2 permutation-equivalent endpoint -> one-step permutation witness

All other cases still fall through to `search_sse_2x2_with_telemetry`.

## Validation

Focused correctness checks:

```bash
timeout -k 20s 240s cargo test -q --lib test_elementary_sse_pair
timeout -k 20s 240s cargo test -q --lib test_self_sse
timeout -k 20s 240s cargo test -q --lib test_different_det_not_equivalent
```

Required benchmark after the fix:

```bash
timeout -k 20s 900s cargo bench --bench search endpoint_equivalent_fast -- --noplot
```

Observed fixed estimate:

- `endpoint_equivalent_fast`: `[2.3156, 2.3435, 2.3753] us`
- Criterion change versus local baseline: `[-4.0296%, -2.4231%, -0.8918%]`

Harness check:

```bash
timeout -k 20s 300s just research-json-save 2026-04-26_wjaj_endpoint_equivalent_result_shortcut
```

Observed fitness:

- `required_cases`: `31`
- `passed_required_cases`: `31`
- `target_hits`: `30`
- `total_points`: `5415`
- `total_elapsed_ms`: `36713`

Decision: keep the fix. The slowdown reproduced against a fresh parent build,
the result-only fast path recovered the micro surface, and harness fitness stayed
unchanged.
