# Low-lag concrete-shift profile signal slice (2026-04-27)

## Scope

Bead: `sse-rust-srl`.

This slice added report-only telemetry for the existing bounded
`concrete_shift_profile_2x2` helper. It does not change default solver
ranking, scoring, search ordering, move generation, or pruning.

The measured surface is intentionally small:

- profile relation: aligned
- profile lag: `1`
- profile entry bound: `1` for the strict boolean-bridge pass, then `3` for a
  small Brix-Ruiz seeded-waypoint check
- profile witness budget: `10000`
- proposal search remains bounded separately at lag `1`, entry `1`

## Commands

Formatting:

```bash
cargo fmt --all
```

Focused report tests:

```bash
timeout -k 20s 180s cargo test --features research-tools --bin report_concrete_shift_proposals
```

Focused profile tests:

```bash
timeout -k 20s 180s cargo test --features research-tools concrete_shift_profile
```

Strict boolean-bridge profile/proposal report:

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
  --profile-max-entry 1 \
  --profile-max-witnesses 10000 \
  --bridge-sample-limit 1 \
  > tmp/concrete_shift_profile_srl_report.json
```

Small-entry profile rerun, with the proposal surface still held at lag `1`,
entry `1`:

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
  > tmp/concrete_shift_profile_srl_report_entry3.json
```

Summary query used while inspecting the artifacts:

```bash
jq '{profile_config, cases: [.cases[] | {case_id, profile: .profile, result_status}]}' \
  tmp/concrete_shift_profile_srl_report_entry3.json
```

## Artifacts

- `tmp/concrete_shift_profile_srl_report.json`
- `tmp/concrete_shift_profile_srl_report_entry3.json`

The `tmp/` directory is gitignored scratch space, so these artifacts are not
committed. The report commands above reproduce them.

## Observed Signal

At lag `1`, entry `1`, witness budget `10000`:

| case | profile status | shift witnesses | concrete lag | limit reached |
| --- | --- | ---: | --- | --- |
| `identity` | `equivalent` | 2 | 1 | false |
| `lag_one_shortcut_control` | `equivalent` | 1 | 1 | false |
| `brix_ruiz_k3_seeded_start_transpose` | `bounded_exhausted` | 0 | null | false |
| `brix_ruiz_k3` | `bounded_exhausted` | 0 | null | false |
| `brix_ruiz_k4_probe` | `bounded_exhausted` | 0 | null | false |

At lag `1`, profile entry `3`, witness budget `10000`, with proposal search
still held at entry `1`:

| case | profile status | shift witnesses | concrete lag | limit reached |
| --- | --- | ---: | --- | --- |
| `identity` | `equivalent` | 2 | 1 | false |
| `lag_one_shortcut_control` | `equivalent` | 1 | 1 | false |
| `brix_ruiz_k3_seeded_start_transpose` | `shift_witness_only` | 2 | null | true |
| `brix_ruiz_k3` | `bounded_exhausted` | 0 | null | false |
| `brix_ruiz_k4_probe` | `bounded_exhausted` | 0 | null | false |

This is a real but coarse signal. The report distinguishes direct low-lag
controls from the hard Brix-Ruiz endpoint pairs, and the entry-3 profile also
separates a nearby k=3 seeded-guide 2x2 waypoint from the k=3 endpoint pair.
However, the k=3 seeded waypoint is only `shift_witness_only` under this
budget, not a concrete-shift positive. The k=3 endpoint and open k=4 lane both
remain zero-shift-witness bounded exhaustions on this low-lag surface.

## Conclusion

Keep the report telemetry path. It is cheap, bounded, and makes the low-lag
profile visible alongside the existing proposal report without affecting solver
behavior.

Reject default ranking or scoring promotion from this slice alone. The current
profile status is too coarse for the hard Brix-Ruiz endpoints: it can separate
easy low-lag controls and a nearby seeded waypoint, but it does not yet produce
a graded residual for the k=3/k=4 endpoint lanes.

Next step, if this signal is pursued, should be a richer report-only residual:
fiber-size mismatch counts or relation-failure counts for the enumerated
low-lag shift witnesses. That would stay telemetry-only while testing whether
the profile can grade hard endpoint states instead of only labeling them
bounded-exhausted.
