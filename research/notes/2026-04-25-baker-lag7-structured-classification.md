# Baker/Lind-Marcus `k = 3` lag-7 structured-class classification (2026-04-25)

## Question

Classify whether the Baker/Lind-Marcus lag-7 witness for

```text
[[1,3],[2,1]] -> [[1,6],[1,1]]
```

sits inside a narrower structured move class in the native low-dimensional
envelope `max_intermediate_dim = 4`, `max_entry = 5`, or whether it still needs
targeted nonstandard factorisation steps.

## Decision

Reject adding a default solver move family from this slice.

The current implemented vocabulary, when matched up to simultaneous
row/column relabeling, already covers six of the seven displayed Baker steps:

- rectangular endpoint lifts/returns cover steps 1 and 7;
- binary-sparse rectangular factorisations cover steps 2 and 6;
- elementary conjugation covers steps 3 and 4.

The only remaining literal Baker ESSE not represented by the current structured
families is step 5, the same-size `4x4 -> 4x4` refactorization. It is not a
diagonal refactorization, not an elementary conjugation, and not a direct
binary-sparse rectangular move. Its useful structure is a hidden heterogeneous
bridge: a `4x4 -> 3x3` binary-sparse rectangular factorisation followed by a
`3x3 -> 4x4` graph row-split/relabeling. That is evidence for a targeted
one-off or bridge-guided replay tactic, not a clean small one-step family to
add to the default solver.

## Sources Inspected

- [`docs/brix-ruiz-sidecar-log.md`](../../docs/brix-ruiz-sidecar-log.md), section
  `Lind-Marcus/Baker lag-7 witness`
- [`research/notes/2026-04-19-exact-endpoint-multi-meet-lag7-diversity.md`](2026-04-19-exact-endpoint-multi-meet-lag7-diversity.md)
- [`research/notes/2026-04-20-k3-non-baker-exact-endpoint-lag7-guided-replay-control.md`](2026-04-20-k3-non-baker-exact-endpoint-lag7-guided-replay-control.md)
- [`research/notes/2026-04-19-k3-lag7-retained-diversity-collapse.md`](2026-04-19-k3-lag7-retained-diversity-collapse.md)
- [`research/notes/2026-04-12-baker-k3-factor-shape.md`](2026-04-12-baker-k3-factor-shape.md)
- [`src/bin/check_lind_marcus_path.rs`](../../src/bin/check_lind_marcus_path.rs)
- [`src/bin/verify_lind_marcus_reconstruction.rs`](../../src/bin/verify_lind_marcus_reconstruction.rs)
- [`src/bin/classify_witness_steps.rs`](../../src/bin/classify_witness_steps.rs)
- [`src/bin/assemble_k3_guide_pool.rs`](../../src/bin/assemble_k3_guide_pool.rs)
- Emmanuel Jeandel, `A smaller SSE for the example by Baker`,
  <https://members.loria.fr/EJeandel/research/conjugacy.html>

Jeandel's page records a related 7-step simplification with smaller last
matrices. The in-repo committed Baker guide is the Lind-Marcus/Baker sequence
used by `k3_shortcut_round1.json` and `k3_normalized_guide_pool.json`; the
classification below is for that encoded witness.

## Diagnostic Enhancement

I updated the opt-in classifier only:

- added `--match-up-to-permutation` to
  [`src/bin/classify_witness_steps.rs`](../../src/bin/classify_witness_steps.rs);
- capped permutation-aware matching at `5x5` so the opt-in diagnostic cannot
  silently fan out into an unbounded factorial probe;
- kept the source-side invariant check `U*V == source representative` in
  permutation-aware mode before accepting a factorisation-family match;
- documented in CLI help that `--match-up-to-permutation` applies only to
  factorisation-family matching;
- added a `structured_factorization_match` classification so binary-sparse and
  rectangular matches are not reported as "not represented" merely because a
  graph-only probe at the same bound fails;
- added focused classifier unit tests for permutation-aware matching, guard
  behavior, and structured-match classification precedence;
- refreshed stale `SearchConfig` initializers in a few research binaries after
  `endpoint_multi_meet_cap` was added.

No default solver move family was added.

## Step Classification

Input artifact:

- [`research/guide_artifacts/k3_shortcut_round1.json`](../guide_artifacts/k3_shortcut_round1.json)

Permutation-aware native-envelope classification:

```text
timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/uxm-classify-k3-baker-shortcut-dim4-perm.json
```

Elapsed: `0:00.38`, exit `0`.

| Step | Dimensions | Graph-plus-structured match | Mixed-only extra | Native dim-4 graph probe | Classification |
| --- | --- | --- | --- | --- | --- |
| 1 | `2x2 -> 3x3` | `rectangular_factorisation_2x3` | none | lag `2` | `structured_factorization_match` |
| 2 | `3x3 -> 4x4` | `binary_sparse_rectangular_factorisation_3x3_to_4` | none | no path within lag 7 | `structured_factorization_match` |
| 3 | `4x4 -> 4x4` | `elementary_conjugation` | none | no path within lag 7 | `structured_factorization_match` |
| 4 | `4x4 -> 4x4` | `elementary_conjugation` | none | no path within lag 7 | `structured_factorization_match` |
| 5 | `4x4 -> 4x4` | none | none | no path within lag 7 | `not_represented_by_current_structured_families` |
| 6 | `4x4 -> 3x3` | `binary_sparse_rectangular_factorisation_4x3_to_3` | none | lag `3` | `structured_factorization_match` |
| 7 | `3x3 -> 2x2` | `rectangular_factorisation_3x3_to_2` | none | lag `4` | `structured_factorization_match` |

Dimension-5 graph expansion comparison:

```text
timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 5 \
  --graph-probe-max-entry 5 \
  > tmp/uxm-classify-k3-baker-shortcut-dim5-perm.json
```

Elapsed: `0:00.48`, exit `0`.

The same report finds a graph-only expansion for step 5 at lag `7` when
dimension `5` is allowed. This agrees with the older graph-only reconstruction
that expands the seven Baker steps into `22` graph moves with block lengths:

```text
[1, 5, 2, 2, 6, 3, 3]
```

That reconstruction was validated by:

```text
timeout -k 20s 180s cargo run --features research-tools --bin verify_lind_marcus_reconstruction \
  > tmp/uxm-verify-lind-marcus-reconstruction.txt
```

Elapsed: `0:00.10`, exit `0`.

## Focus On Former Missing Steps

Step 2 is no longer missing. It is covered directly by
`binary_sparse_rectangular_factorisation_3x3_to_4` inside the native envelope.

Step 6 is no longer missing. It is covered directly by
`binary_sparse_rectangular_factorisation_4x3_to_3` inside the native envelope.

Step 5 is the blocker. Its displayed factors have this shape:

```text
U =
0 1 1 1
1 0 1 1
1 0 0 0
0 1 0 0

V =
0 1 0 1
0 2 1 0
0 0 1 0
1 0 0 0
```

Observed invariants:

- `det(U) = 0`, rank `3`;
- `det(V) = -2`, rank `4`;
- `U` is binary with all column sums `2`;
- the step is not conjugation by a unimodular elementary matrix.

The useful bridge is:

```text
4x4:1,2,2,0,1,1,1,1,0,1,0,1,0,2,1,0
-> 3x3:1,1,1,3,0,2,1,1,1
-> 4x4:1,1,1,1,3,0,2,2,1,0,0,0,0,1,1,1
```

The first half is already a direct
`binary_sparse_rectangular_factorisation_4x3_to_3` match:

```text
timeout -k 20s 180s target/debug/explain_witness_step \
  --from 4x4:1,2,2,0,1,1,1,1,0,1,0,1,0,2,1,0 \
  --to 3x3:1,1,1,3,0,2,1,1,1 \
  --graph-max-lag 3 --graph-max-intermediate-dim 4 --graph-max-entry 5 \
  --factorisation-max-entry 5 \
  --write-json tmp/uxm-step5-hidden-bridge-a4-to-3x3.json
```

Elapsed: `0:00.01`, exit `0`.

The second half is not a current direct structured factorisation match, but it
does have a graph-only explanation as `elementary_row_split_then_graph_isomorphism`:

```text
timeout -k 20s 180s target/debug/explain_witness_step \
  --from 3x3:1,1,1,3,0,2,1,1,1 \
  --to 4x4:1,1,1,1,3,0,2,2,1,0,0,0,0,1,1,1 \
  --graph-max-lag 3 --graph-max-intermediate-dim 4 --graph-max-entry 5 \
  --factorisation-max-entry 5 \
  --write-json tmp/uxm-step5-hidden-bridge-3x3-to-a5.json
```

Elapsed: `0:00.04`, exit `0`.

## Artifact Paths

- `tmp/uxm-check-lind-marcus-path.txt`
- `tmp/uxm-verify-lind-marcus-reconstruction.txt`
- `tmp/uxm-classify-k3-baker-shortcut-dim4-perm.json`
- `tmp/uxm-classify-k3-baker-shortcut-dim5-perm.json`
- `tmp/uxm-step5-hidden-bridge-a4-to-3x3.json`
- `tmp/uxm-step5-hidden-bridge-3x3-to-a5.json`

Additional intermediate classifier artifacts kept for comparison:

- `tmp/uxm-classify-k3-normalized-baker.json`
- `tmp/uxm-classify-k3-normalized-baker-dim4.json`
- `tmp/uxm-classify-k3-normalized-baker-dim5.json`

## Validation Commands Run For This Note

```text
timeout -k 20s 180s cargo build --features research-tools \
  --bin classify_witness_steps --bin explain_witness_step \
  --bin check_lind_marcus_path --bin verify_lind_marcus_reconstruction
```

Elapsed: `0:00.09`, exit `0`.

```text
timeout -k 20s 180s cargo test --features research-tools --lib \
  test_binary_sparse_factorisations_reach_
```

Elapsed: `0:00.10`, exit `0`; selected tests:

- `test_binary_sparse_factorisations_reach_baker_step_2`
- `test_binary_sparse_factorisations_reach_baker_step_6`
- `test_binary_sparse_factorisations_reach_hidden_baker_step_5_bridge`

```text
timeout -k 20s 180s cargo run --features research-tools --bin check_lind_marcus_path \
  > tmp/uxm-check-lind-marcus-path.txt
```

Elapsed: `0:00.76`, exit `0`.

```text
timeout -k 20s 180s cargo run --features research-tools --bin verify_lind_marcus_reconstruction \
  > tmp/uxm-verify-lind-marcus-reconstruction.txt
```

Elapsed: `0:00.10`, exit `0`.

## Requested Final Validation

```text
timeout -k 20s 120s cargo fmt --all
```

Elapsed: `0:01.08`, exit `0`.

```text
timeout -k 20s 180s cargo test --features research-tools --bin check_lind_marcus_path
```

Elapsed: `0:00.13`, exit `0`.

```text
timeout -k 20s 180s cargo test --features research-tools --bin verify_lind_marcus_reconstruction
```

Elapsed: `0:00.25`, exit `0`.

```text
timeout -k 20s 180s cargo build --features research-tools --bin classify_witness_steps
```

Elapsed: `0:00.09`, exit `0`.

```text
timeout -k 20s 180s cargo run --features research-tools --bin check_lind_marcus_path
```

Elapsed: `0:00.69`, exit `0`. Output again showed step 5 as the only
`MISSING` Baker transition; steps 2 and 6 were covered by the binary-sparse
rectangular families.

```text
timeout -k 20s 180s cargo run --features research-tools --bin verify_lind_marcus_reconstruction
```

Elapsed: `0:00.09`, exit `0`. Output ended with
`All 7 Baker waypoint transitions are recovered by the 22 graph-only path.`

```text
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
timeout -k 5s 20s target/debug/research_harness \
  --cases research/cases.json \
  --worker-case brix_ruiz_k3_non_baker_exact_endpoint_lag7_guided_replay
```

Build elapsed: `0:02.23`, exit `0`. Harness elapsed: `0:00.15`, exit `0`.
The harness returned `actual_outcome = equivalent`, `steps = 7`, and accepted
one guide artifact.

Post-review focused validation:

```text
timeout -k 20s 180s cargo test --features research-tools --bin classify_witness_steps
```

Elapsed: `0:01.25`, exit `0`. This ran the three new classifier unit tests.

After the first review fix, I reran `cargo fmt --all`, the two requested bin
tests, the two requested `cargo run` validations, both classifier report
commands, and the requested `research_harness` command. All exited `0`; the
regenerated classification artifacts retained the same keep/reject evidence.

After the second review fix, I reran:

```text
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools --bin classify_witness_steps
timeout -k 20s 180s cargo build --features research-tools --bin classify_witness_steps
```

Elapsed: `0:01.04`, `0:00.68`, and `0:00.43`; all exited `0`. I also
regenerated both classifier report artifacts; their keep/reject evidence was
unchanged.

## Follow-Up

No new bead opened.

The evidence is not strong enough for a narrow default move family. A future
task would need to be framed as an explicitly targeted Baker step-5 bridge or
one-off replay diagnostic, not as a general solver move family.
