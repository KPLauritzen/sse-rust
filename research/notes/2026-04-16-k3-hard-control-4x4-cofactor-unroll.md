# k=3 runtime round: unrolled `4x4` cofactors on the hard shortcut control (2026-04-16)

## Question
On the current `k=3` shortcut hard control, does a purely low-level rewrite of the hot `4x4` cofactor computation improve throughput without changing guide outcomes or harness fitness?

## Profiling-first setup

I re-profiled rebuilt binaries first and kept the runs bounded:

- mixed endpoint-search `brix_ruiz_k3` on rebuilt `target/dist/search` with `--pprof`
- current hard shortcut family on rebuilt `target/dist/search`, reduced to `shortcut_max_total_segment_attempts=4` so the profile stayed bounded

The profiling surfaces split by workload:

- mixed endpoint-search still spent most factorisation work in `square_factorisation_3x3`, with hot frames in `solve_nonneg_2x3_into`, `DynMatrix::new`, and `canonical_perm`
- the current hard shortcut control pivoted to the structured `4x4 -> 5x5` path: the bounded `--pprof` sample in `tmp/sse-rust-oci-profile-hard-a4.pprof.txt` was dominated by `cofactor_matrix_and_det_4x4`, with repeated stacks through `enumerate_binary_sparse_factorisation_4x4_to_5_family` and `solve_nonneg_4x4_with_cofactors`

I picked that one hard-control hotspot family and avoided any pruning, ranking, or policy changes.

## Change

In `src/factorisation.rs`:

- rewrote `cofactor_matrix_and_det_4x4` from generic per-minor construction to a straight-line computation that reuses precomputed `2x2` minors
- added a regression test that compares the fast path against the old reference-style cofactor expansion on representative `4x4` cases

No solver semantics or search policy were intentionally changed.

## Correctness gate

- `cargo test -q`: pass (`248` passed, `1` ignored, plus `20` bin tests)

## Hard-control A/B

Common config:

- endpoint `1,3,2,1 -> 1,6,1,1`
- `stage=shortcut-search`
- `mixed`, `max_intermediate_dim=5`, `max_entry=5`
- guide pool `research/guide_artifacts/k3_normalized_guide_pool.json`
- `guided_max_shortcut_lag=5`, `guided_min_gap=2`, `guided_max_gap=5`
- `guided_segment_timeout=3`, `guided_rounds=2`
- `shortcut_max_guides=8`, `shortcut_rounds=2`, `shortcut_max_total_segment_attempts=8`
- direct plain `target/dist/search` under `/usr/bin/time` and `timeout -k 5s 45s`

Baseline (`tmp/sse-rust-oci-hard-a8.plain-baseline.json`):

- wall `39.27s`
- outcome `equivalent`, lag `7`
- guided improvements/promoted `0/0`
- factorisations `655,694`
- visited `13,417`

Patched (`tmp/sse-rust-oci-hard-a8.plain-patch.json`):

- wall `6.32s`
- outcome `equivalent`, lag `7`
- guided improvements/promoted `0/0`
- factorisations `1,237,024`
- visited `37,409`

Interpretation:

- useful reach stayed flat on the chosen control (`equivalent`, same lag, same promoted/improved guides)
- work counters increased because this surface uses per-segment wall-clock budgets; the faster solver path lets the same attempt cap spend more of each timeout on productive search instead of `4x4` cofactor bookkeeping

## Aggregate confirmation

Saved harness artifacts:

- baseline: `research/runs/sse-rust-oci-cofactor-unroll-baseline.json`
- patched: `research/runs/sse-rust-oci-cofactor-unroll.json`

Harness fitness stayed identical except for elapsed time:

- baseline: required `23/23`, target hits `22`, points `3795`, telemetry-focus `69,496,257`, elapsed `23,041 ms`
- patched: required `23/23`, target hits `22`, points `3795`, telemetry-focus `69,496,257`, elapsed `22,969 ms`

The aggregate gain is small but non-negative; the patch does not buy speed by cutting away useful reach.

## Decision

Keep.

The hard current control showed a clear profiler-led throughput gain in the targeted hotspot family, while the aggregate harness scorecard stayed flat on reach and improved slightly on elapsed time.
