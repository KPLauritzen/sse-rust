# Concrete-shift path-state residual counts (2026-04-27)

## Scope

Bead: `sse-rust-41yx`.

This slice compares schema-v4 concrete-shift profile residual counts on a
small set of already-enumerated low-lag `2x2` path-state pairs. It is bounded,
report-only telemetry. It does not change default solver ranking, scoring,
pruning, move generation, or search behavior.

The measured profile envelope matches the prior residual-count slice:

- profile relation: aligned
- profile lag: `1`
- profile entry bound: `3`
- profile concrete-witness budget: `10000`
- proposal search remains separately held at lag `1`, entry `1`

## Path-State Sources

The added path-state cases are committed research artifacts, not new witness
hunts:

- `brix_ruiz_k3_non_baker_near_target`: penultimate `2x2` state
  `[[0,5],[1,2]] -> [[1,6],[1,1]]` from
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`.
- `brix_ruiz_k3_retained_near_target_transpose`: penultimate transposed `2x2`
  state `[[0,1],[5,2]] -> [[1,6],[1,1]]` from retained exact-meet artifact
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json`.

The comparison keeps the earlier useful/control rows:

- `identity`
- `lag_one_shortcut_control`
- `brix_ruiz_k3_seeded_start_transpose`
- `brix_ruiz_k3`
- `brix_ruiz_k4_probe`

## Command

Bounded report command:

```bash
timeout -k 20s 180s cargo run --features research-tools --bin report_concrete_shift_proposals -- \
  --boolean-bridge-aligned \
  --case identity \
  --case lag_one_shortcut_control \
  --case brix_ruiz_k3_seeded_start_transpose \
  --case brix_ruiz_k3_non_baker_near_target \
  --case brix_ruiz_k3_retained_near_target_transpose \
  --case brix_ruiz_k3 \
  --case brix_ruiz_k4_probe \
  --max-lag 1 \
  --max-entry 1 \
  --max-witnesses 10000 \
  --profile-max-lag 1 \
  --profile-max-entry 3 \
  --profile-max-witnesses 10000 \
  --bridge-sample-limit 1 \
  > tmp/concrete_shift_profile_41yx_path_state_residuals.json
```

Inspection query:

```bash
jq -r '.cases[] |
  [.case_id,.profile.status,.profile.shift_witnesses,
   (.profile.concrete_witness_lag//""),.profile.limit_reached,
   .profile.residuals.r_intertwiner_candidates,
   .profile.residuals.r_intertwiners_without_rs_factor,
   .profile.residuals.rs_factor_candidates,
   .profile.residuals.concrete_witnesses_checked,
   .profile.residuals.concrete_relation_failures,
   .result_status] | @tsv' \
  tmp/concrete_shift_profile_41yx_path_state_residuals.json
```

## Artifact

- `tmp/concrete_shift_profile_41yx_path_state_residuals.json`

`tmp/` is gitignored; the command above regenerates the artifact.

## Residual Table

Report schema: `4`.

| case | profile status | shift witnesses | concrete lag | R intertwiners | R without RS factor | RS factors | concrete checks | relation failures | limit reached |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| `identity` | `equivalent` | 2 | 1 | 256 | 254 | 2 | 1 | 0 | false |
| `lag_one_shortcut_control` | `equivalent` | 1 | 1 | 6 | 5 | 1 | 193 | 192 | false |
| `brix_ruiz_k3_seeded_start_transpose` | `shift_witness_only` | 2 |  | 8 | 6 | 2 | 10000 | 10000 | true |
| `brix_ruiz_k3_non_baker_near_target` | `bounded_exhausted` | 0 |  | 2 | 2 | 0 | 0 | 0 | false |
| `brix_ruiz_k3_retained_near_target_transpose` | `bounded_exhausted` | 0 |  | 4 | 4 | 0 | 0 | 0 | false |
| `brix_ruiz_k3` | `bounded_exhausted` | 0 |  | 4 | 4 | 0 | 0 | 0 | false |
| `brix_ruiz_k4_probe` | `bounded_exhausted` | 0 |  | 2 | 2 | 0 | 0 | 0 | false |

The fiber-size mismatch residuals were all zero in this bounded run:
`omega_f_fiber_size_mismatch_candidates = 0`,
`omega_f_fiber_size_mismatches = 0`,
`sigma_h_fiber_size_mismatch_candidates = 0`, and
`sigma_h_fiber_size_mismatches = 0` for every case.

## Conclusion

Keep the schema-v4 report-only residual telemetry. It still separates positive
controls from the hard bounded-exhausted surface, and it distinguishes the
seeded transpose waypoint as `shift_witness_only` with concrete relation
failures under the bounded budget.

Reject promoting this signal into ranking, scoring, pruning, or move generation.
On the already-enumerated path-state slice, the zero-witness residual is too
coarse and non-monotone: one endpoint-adjacent state has the same `R without RS`
count as the open `k=4` probe (`2`), while the transposed retained
endpoint-adjacent state has the same count as the hard `k=3` endpoint (`4`).
That does not justify a separate opt-in ranking experiment by itself.

Next step, if pursued, should wait for a richer endpoint-agnostic profile
surface that can cover the existing `3x3`/`4x4` path states. Do not widen the
current `2x2` low-lag residual into default behavior.
