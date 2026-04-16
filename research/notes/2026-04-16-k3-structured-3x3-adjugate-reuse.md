# k=3 runtime round: reuse `3x3` adjugates in structured sparse factorisation loops (2026-04-16)

## Question
After the `4x4` cofactor win, what is the next useful low-level hotspot to target, and can we speed it up without changing search outcomes or useful reach?

## Profiling-first setup

Rebuilt `target/dist/search` with `pprof` support first, then re-profiled two bounded surfaces:

1. mixed endpoint-search `brix_ruiz_k3`
2. current hard shortcut control, reduced to `shortcut_max_total_segment_attempts=4` so the profile stayed bounded

Profile evidence:

- mixed endpoint-search still generated most work in `square_factorisation_3x3`
  - `320,649 / 354,093` generated factorisations
  - sampled stacks repeatedly hit `enumerate_square_factorisation_3x3_family` and `solve_nonneg_2x3_into`
- hard shortcut control shifted after the previous cofactor trim
  - `square_factorisation_3x3`: `148,778` generated
  - `binary_sparse_rectangular_factorisation_4x4_to_5`: `119,640` generated
  - `binary_sparse_rectangular_factorisation_3x3_to_4`: `49,176` generated
  - but sampled stacks now over-weighted the structured `3x3 -> 4x4` sparse path:
    - `enumerate_binary_sparse_factorisation_3x3_to_4_family`: `766` stack hits
    - `solve_nonneg_3x3`: `384`
    - `enumerate_square_factorisation_3x3_family`: `594`

Interpretation:

- raw volume still favors `square_factorisation_3x3`,
- but on the current hard shortcut surface, the next profiler-led CPU target is the repeated nonsingular `3x3` solve work inside the structured sparse family,
- and that family fits the same kind of kept win as the prior `4x4` round: reuse math for repeated RHS solves instead of reallocating and recomputing each time.

## Change

In `src/factorisation.rs`:

- added `adjugate_matrix_and_det_3x3`
- added `solve_nonneg_3x3_with_adjugate` for the nonsingular unique-solution path
- rewired the nonsingular branch of `solve_nonneg_3x3` through that helper
- rewired two structured sparse loops to compute the `3x3` adjugate once per core matrix and solve repeated RHS columns without temporary `Vec` results:
  - `visit_binary_sparse_factorisations_4x4_to_3`
  - `visit_binary_sparse_factorisations_3x3_to_4`
- added a focused unit test comparing the helper against the main `solve_nonneg_3x3` behavior on representative nonsingular cases

No pruning, ranking, or search-policy logic changed.

## Correctness gate

- `cargo test -q`: pass (`249` passed, `1` ignored, plus `20` bin tests)

## Direct controls

### Mixed endpoint-search baseline

Command family:

```bash
target/dist/search 1,3,2,1 1,6,1,1 \
  --max-lag 6 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --search-mode mixed \
  --json --telemetry
```

Baseline (`tmp/sse-rust-t8r-mixed-k3.baseline.json`):

- wall `2.00s`
- outcome `unknown`
- telemetry identical to the patched run:
  - factorisations `354,093`
  - generated `354,221`
  - after pruning `7,478`
  - visited `5,141`

Patched (`tmp/sse-rust-t8r-mixed-k3.patch.json`):

- wall `0.48s`
- outcome `unknown`
- same telemetry as baseline

This is a fixed-work win: same search work, lower runtime.

### Hard shortcut control

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

Baseline (`tmp/sse-rust-t8r-hard-a8.baseline.json`):

- wall `11.62s`
- outcome `equivalent`, lag `7`
- improvements/promoted `0/0`
- factorisations `845,877`
- visited `24,168`

Patched (`tmp/sse-rust-t8r-hard-a8.patch.json`):

- wall `6.49s`
- outcome `equivalent`, lag `7`
- improvements/promoted `0/0`
- factorisations `1,237,024`
- visited `37,409`

Interpretation:

- useful reach stayed flat on the hard control,
- work counters increased because this is a timeout-budgeted shortcut surface, so the faster structured-solver path let the same attempt/timeout budget spend more time on productive search rather than `3x3` solve bookkeeping.

## Aggregate harness confirmation

Current-`HEAD` comparison baseline remained the saved run from the previous kept round:

- baseline artifact: `research/runs/sse-rust-oci-cofactor-unroll.json`
- baseline fitness: required `23/23`, target hits `22`, points `3795`, telemetry-focus `69,496,257`, elapsed `22,969 ms`

Patched harness reruns:

- `research/runs/sse-rust-t8r-3x3-adjugate-r1.json`
  - required `23/23`, target hits `22`, points `3795`, telemetry-focus `69,496,257`, elapsed `23,036 ms`
- `research/runs/sse-rust-t8r-3x3-adjugate-r2.json`
  - required `23/23`, target hits `22`, points `3795`, telemetry-focus `69,496,257`, elapsed `22,951 ms`

Case-level relevant deltas versus the current-`HEAD` baseline:

- `brix_ruiz_k3_shortcut_seeded`: `49 -> 45 -> 44 ms`
- `brix_ruiz_k3_shortcut_normalized_pool_probe`: `393 -> 382 -> 390 ms`
- `brix_ruiz_k3_shortcut_quotient_retained_pool_probe`: `402 -> 398 -> 385 ms`
- `brix_ruiz_k3_graph_only`: `8667 -> 8733 -> 8675 ms`

Interpretation:

- the aggregate harness stayed effectively flat within noise,
- the shortcut-stage cases relevant to the targeted structured-sparse family improved,
- the one noticeable slowdown (`graph_only`) is outside the touched family and disappeared on rerun to near-baseline.

## Decision

Keep.

This round found a real profiler-led follow-up after the `4x4` cofactor trim: reuse `3x3` adjugates inside the structured sparse families that solve several RHS columns against one nonsingular core. The direct hard control and fixed-work mixed endpoint-search surface both improved materially, and the aggregate harness stayed flat-to-slightly-positive with unchanged fitness.
