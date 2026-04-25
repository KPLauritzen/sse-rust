# Boolean-bridge aligned concrete-shift subsearch (2026-04-25)

## Question

Can the existing `2x2` aligned concrete-shift report surface expose the
restricted boolean-bridge witness class cheaply, without changing the default
solver and without mislabeling the result as a replayable SSE path?

This note records bead `sse-rust-2fwp`.

## Decision

Keep a small report-only opt-in surface:

- `report_concrete_shift_proposals --boolean-bridge-aligned`

This is a restricted aligned concrete-shift witness class, not a completeness
claim and not a replayable `full_path` surface.

Implementation shape:

- reuse the existing aligned concrete-shift search and verification logic;
- force `relation = aligned`;
- force `max_entry = 1`, which restricts `R` and `S` to boolean entries in the
  bounded `2x2` search space; and
- label the report artifact and equivalent status explicitly as the restricted
  boolean-bridge aligned witness class.

No default solver mode changed, and failure in this restricted class is still
non-pruning.

## Command

```bash
mkdir -p tmp
timeout -k 20s 180s cargo run --features research-tools --bin report_concrete_shift_proposals -- \
  --case lag_one_shortcut_control \
  --case identity \
  --boolean-bridge-aligned \
  > tmp/2fwp-boolean-bridge-report.json
```

## Observed result

Artifact:

- `tmp/2fwp-boolean-bridge-report.json`

Top-level labels:

- `artifact_kind = "boolean_bridge_aligned_concrete_shift_proposal_report"`
- `witness_class = "restricted boolean-bridge aligned concrete-shift witness class"`
- `search_restriction = "relation=aligned and bridge matrices R,S are boolean"`

Case statuses:

- `lag_one_shortcut_control`:
  `result_status = "equivalent_by_boolean_bridge_aligned_concrete_shift"`,
  `lag = 1`, `R = 2x2:1,0,1,1`, `S = 2x2:0,1,1,1`
- `identity`:
  `result_status = "equivalent_by_boolean_bridge_aligned_concrete_shift"`,
  `lag = 1`, `R = 2x2:0,1,1,0`, `S = 2x2:0,1,1,0`

Both required positive controls stayed positive at lag `1` under the restricted
surface.

## Validation

```bash
cargo fmt --all
timeout -k 20s 240s cargo test -q concrete_shift --features research-tools
timeout -k 20s 240s cargo test -q --bin report_concrete_shift_proposals --features research-tools
```

## Follow-up

No follow-up bead is justified from this slice alone. The bounded report-only
surface is now explicit enough for later experiments, and there is still no
evidence that failure in the boolean-bridge class should be used as a rejection
rule.
