# k=3 shortcut-search trace(M^3) invariant gate (reverted) — 2026-04-14

## Question

Can adding a dynamic endpoint prefilter `trace(A^3) == trace(B^3)` reduce expensive segment-search work on the hard k=3 shortcut surface?

## Change tested

Temporary code patch (reverted afterward):

- in dynamic endpoint search (`search_sse_with_telemetry_dyn_with_deadline_and_observer`), add a prefilter after existing `trace(M^2)` and before `trace`:
  - reject with `NotEquivalent("trace(M^3) invariant mismatch")` when cube traces differ.

Also added temporary telemetry plumbing and a unit test for the new rejection path.

## Measurements

### Stable shortcut probe (dim4/entry5)

Config:

- `max_intermediate_dim=4`, `max_entry=5`
- `guided_max_shortcut_lag=3`, `guided_min_gap=2`, `guided_max_gap=4`, `guided_rounds=1`
- `shortcut_max_guides=12`, `shortcut_rounds=2`, attempts `96`

Before (`tmp/loop7_guides12_a96_dim4e5.json`):

- lag `7`
- improved `14`
- frontier `623`
- visited `32705`

With patch (`tmp/loop11_tracecube_dim4e5_a96.json`):

- lag `7`
- improved `14`
- frontier `623`
- visited `32705`

No change.

### Hard shortcut probe (dim5/entry6)

Config:

- `max_intermediate_dim=5`, `max_entry=6`
- `guided_max_shortcut_lag=4`, `guided_min_gap=2`, `guided_max_gap=6`, `guided_rounds=2`
- `shortcut_max_guides=12`, `shortcut_rounds=2`, attempts `32`

Before (`tmp/loop10_dim5_entry6_a32_notimeout_180.json`):

- completed under outer `timeout 180`
- lag `7`
- improved `1`
- frontier `4426`
- visited `277582`

With patch (`tmp/loop11_tracecube_dim5e6_a32_t240.json`):

- completed under outer `timeout 240`
- lag `7`
- improved `1`
- frontier `4426`
- visited `277582`

No improvement signal; the first patched run at outer `timeout 180` timed out with an empty JSON artifact (`tmp/loop11_tracecube_dim5e6_a32.json`), so there was no evidence of a runtime win on the hard surface.

## Decision

Reverted. This added complexity without moving the active bottleneck:

- no lag gain,
- no frontier/visited reduction on measured probes,
- no evidence of better hard-surface completion behavior.

## Next hypothesis

Keep invariants unchanged and focus on segment-admission quality before expensive segment searches (cheap ranking/prefiltering of candidate segments).
