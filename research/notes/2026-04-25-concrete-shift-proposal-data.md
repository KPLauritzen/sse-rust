# Concrete-shift proposal data (2026-04-25)

## Question

Can we surface useful bounded concrete-shift witness metadata as proposal data
without mislabeling it as a replayable SSE path or duplicating endpoint
multi-meet path inventory work?

This was bead `sse-rust-5ly`.

## Decision

Keep a report-only proposal surface. The new `research-tools` binary emits
bounded concrete-shift-positive control data with:

- explicit non-path semantics (`artifact_kind = "concrete_shift_proposal_data"`
  and `replayable_as_full_path = false`);
- relation validation plus bijection-shape validation before bridge sampling,
  plus bound-consistency checks on reported `max_lag` / `max_entry`, so
  malformed or misdescribed witnesses return `Err` instead of panicking or
  being mislabeled;
- relation, lag, search bounds, and a stable `fnv1a64` witness signature;
- compact `R` / `S` matrix support summaries;
- lag-power support summaries for `A^m` and `B^m`;
- per-map fiber cardinalities; and
- bounded bridge samples linking `R/S` edge pairs to `A^m` / `B^m` path
  signatures.

Do not export these witnesses as `full_path` guide artifacts. The search CLI
restriction stays correct.

## Command

Build:

```bash
timeout -k 20s 180s cargo build --features research-tools --bin report_concrete_shift_proposals
```

Generate bounded proposal JSON for two positive controls:

```bash
mkdir -p tmp
timeout -k 20s 180s cargo run --features research-tools --bin report_concrete_shift_proposals -- \
  --case lag_one_shortcut_control \
  --case identity \
  > tmp/2026-04-25-concrete-shift-proposal-report.json
```

## Raw artifact

- `tmp/2026-04-25-concrete-shift-proposal-report.json`

## Example output fields

The nontrivial `lag_one_shortcut_control` produced:

- `result_status = "equivalent_by_concrete_shift"`
- `proposal.relation = "aligned"`
- `proposal.lag = 1`
- `proposal.shift_signature = "lag=1|R=2x2:1,0,1,1|S=2x2:0,1,1,1"`
- `proposal.witness_signature = "fnv1a64:bbf20e73627dd3f5"`
- `proposal.bridge_r.nonzero_positions = [[0,0],[1,0],[1,1]]`
- `proposal.bridge_s.nonzero_positions = [[0,1],[1,0],[1,1]]`
- `proposal.fiber_cardinalities.omega_e = [0,1,1,2]`
- `proposal.rs_to_a_path_fibers[3].samples[0].mapped_path.signature = "1>1#0"`

The `identity` control also stays positive on the same surface and shows that
the witness signature is tied to the actual bounded witness, not merely to the
endpoint pair.

## Keep / Reject

Keep:

- relation, lag, and explicit search bounds;
- stable witness signature plus compact `lag|R|S` shift signature;
- raw `R` / `S` matrices for `2x2`, because they stay small and interpretable;
- support summaries (`nonzero_positions`, row sums, column sums, entry sum,
  max entry) for `A`, `B`, `R`, `S`, `A^m`, and `B^m`;
- fiber cardinalities as a compact proxy for where the witness mass sits; and
- bounded bridge samples with edge-pair to path-signature links, since they are
  the smallest concrete hint toward later ranking features.

Reject:

- `full_path` guide-artifact emission or any `full_path`-shaped payload;
- full raw `sigma_*` / `omega_*` permutation arrays in the report surface;
- unbounded edge/path inventories for all fibers; and
- treating concrete-shift-positive data as endpoint multi-meet inventory.

## Follow-up

No new bead was opened from this slice. If later ranking work wants to consume
the kept fields, existing bead `sse-rust-srl` is still the right home for that
integration step.

## Validation

Focused tests:

```bash
timeout -k 20s 240s cargo test -q concrete_shift_proposal_data --features research-tools
timeout -k 20s 240s cargo test -q --bin report_concrete_shift_proposals --features research-tools
```

Required formatting/build gate:

```bash
cargo fmt --all
timeout -k 20s 180s cargo build --features research-tools --bin report_concrete_shift_proposals
```

Bench note:

- `cargo bench --bench search -- --noplot` was not necessary here because the
  default solver hot path was not changed. The new code is a report-only helper
  plus a `research-tools` sidecar binary, and the default solver behavior and
  guide-artifact path handling stayed unchanged.
