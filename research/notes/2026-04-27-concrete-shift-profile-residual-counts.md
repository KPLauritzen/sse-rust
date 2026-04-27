# Concrete-shift profile residual counts (2026-04-27)

## Scope

Bead: `sse-rust-l2mm`.

This slice adds bounded, report-only residual counts to the existing
low-lag concrete-shift profile telemetry. It does not change solver ranking,
scoring, pruning, move generation, or default search behavior.

The residuals are bounded by the existing profile envelope:

- profile relation: aligned
- profile lag: `1`
- profile entry bound: `3`
- profile concrete-witness budget: `10000`
- proposal search remains separately held at lag `1`, entry `1`

## Command

Bounded report command:

```bash
timeout -k 20s 180s cargo run --features research-tools --bin report_concrete_shift_proposals -- \
  --boolean-bridge-aligned \
  --case identity \
  --case lag_one_shortcut_control \
  --case brix_ruiz_k3_seeded_start_transpose \
  --case brix_ruiz_k3 \
  --case brix_ruiz_k4_probe \
  --max-lag 1 \
  --max-entry 1 \
  --max-witnesses 10000 \
  --profile-max-lag 1 \
  --profile-max-entry 3 \
  --profile-max-witnesses 10000 \
  --bridge-sample-limit 1 \
  > tmp/concrete_shift_profile_l2mm_residuals_entry3.json
```

Inspection query:

```bash
jq '{schema_version, profile_config, cases: [.cases[] | {case_id, status: .profile.status, shift_witnesses: .profile.shift_witnesses, concrete_witness_lag: .profile.concrete_witness_lag, limit_reached: .profile.limit_reached, residuals: .profile.residuals, result_status}]}' \
  tmp/concrete_shift_profile_l2mm_residuals_entry3.json
```

## Artifact

- `tmp/concrete_shift_profile_l2mm_residuals_entry3.json`

`tmp/` is gitignored; the command above regenerates the artifact.

## Residual Table

Report schema: `4`.

| case | profile status | shift witnesses | R intertwiners | R without RS factor | RS factors | concrete checks | relation failures | limit reached |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `identity` | `equivalent` | 2 | 256 | 254 | 2 | 1 | 0 | false |
| `lag_one_shortcut_control` | `equivalent` | 1 | 6 | 5 | 1 | 193 | 192 | false |
| `brix_ruiz_k3_seeded_start_transpose` | `shift_witness_only` | 2 | 8 | 6 | 2 | 10000 | 10000 | true |
| `brix_ruiz_k3` | `bounded_exhausted` | 0 | 4 | 4 | 0 | 0 | 0 | false |
| `brix_ruiz_k4_probe` | `bounded_exhausted` | 0 | 2 | 2 | 0 | 0 | 0 | false |

The fiber-size mismatch residuals were all zero in this bounded run:
`omega_f_fiber_size_mismatch_candidates = 0`,
`omega_f_fiber_size_mismatches = 0`,
`sigma_h_fiber_size_mismatch_candidates = 0`, and
`sigma_h_fiber_size_mismatches = 0` for every case.

## Conclusion

Keep the report-only residual telemetry. It adds a small, explainable signal
inside the existing profile surface: the hard zero-witness endpoints are no
longer identical under the report (`brix_ruiz_k3` has four bounded `R`
intertwiners that fail to complete to `RS=A`, while `brix_ruiz_k4_probe` has
two).

Reject any default ranking, scoring, pruning, or lower-bound promotion from
this slice. The signal is bounded-envelope telemetry only; it is not a
theorem-grade no-witness result.

Next step, if pursued, should stay report-only: compare these residuals on a
small set of already-enumerated low-lag path states before deciding whether a
separate ranking experiment is justified.
