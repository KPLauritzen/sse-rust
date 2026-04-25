# Brix-Ruiz `k=4` balanced bridge proposals reject (2026-04-25)

## Question

For bead `sse-rust-nw7.4`, test whether a small balanced-elementary bridge
proposal surface improves the retained open Brix-Ruiz `k=4`
`graph_plus_structured` lane:

`brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`

This is the balanced elementary surface from `src/balanced.rs`, not balanced
concrete shift from `src/concrete_shift.rs`.

## Surface

The retained approximate-hit extractor again reports only `4x4` ranked
approximate hits on this lane. The existing balanced bridge-return helpers are
`3x3 -> 2x2 <-balanced-> 2x2 -> 3x3` seams, so this slice used a report-only
principal-window overlay:

1. Replay the retained fixed-budget lane unchanged.
2. Select the top retained `4x4` approximate-hit states.
3. For each selected `4x4` state, delete one matching row/column to get each
   principal `3x3` window.
4. Generate balanced bridge-return proposals on that `3x3` window with the
   existing out-split and in-split return helpers from `src/balanced.rs`.
5. Embed the proposed `3x3` window back into the original `4x4` state, keeping
   the deleted vertex incidence unchanged.
6. Score the embedded full `4x4` proposal against the retained opposite-side
   visited set and the recorded closest counterpart.

This does not enqueue proposals into the solver and does not claim the embedded
full `4x4` edit is an SSE move. It is an opt-in diagnostic for whether the
balanced elementary sidecar has enough retained-lane signal to justify a real
integration or proof attempt.

New diagnostic:

- `src/bin/evaluate_brix_ruiz_k4_balanced_bridge_proposals.rs`

Primary raw artifacts:

- `tmp/brix_ruiz_k4_balanced_bridge_proposals_top12_cap4_2026-04-25_nw7_4.json`
- `tmp/brix_ruiz_k4_balanced_bridge_proposals_top24_cap4_2026-04-25_nw7_4.json`
- `tmp/brix_ruiz_k4_balanced_bridge_proposals_top12_cap4_m2_2026-04-25_nw7_4.json`
- `tmp/brix_ruiz_k4_balanced_bridge_proposals_metrics_2026-04-25_nw7_4.tsv`
- `tmp/brix_ruiz_k4_balanced_bridge_proposals_top24_examples_2026-04-25_nw7_4.tsv`

## Same-Budget Metrics

Each row replays the same retained fixed-budget lane. The proposal columns are
an overlay evaluated against that same retained visited/counterpart state.
Proposal approximate-opposite hits are non-exact signature hits, and the ranked
source hit list excludes exact-meet edges, matching the baseline telemetry
distinction between exact meets and approximate hits. Deduped proposal counts
use explicit `any`/`min` aggregation per candidate and direction, while the
example list keeps one coherent representative provenance without ranking by
cross-provenance counterpart L1. The retained opposite-frontier maps now also
track only discovered, enqueued states, so proposal hit checks match the
solver's actual retained frontier rather than transient exact meets or other
non-retained edges. Ranked source hits are likewise filtered to retained,
discovered/enqueued `4x4` states before truncation, so the requested `top_hits`
budget is spent entirely on eligible principal-window sources.

| Run | Outcome | Exact target hits | Approx. hits | Max frontier | Visited | Expanded | Factorisations | Kept candidates | Discovered | Elapsed ms | Proposal raw | Proposal unique | Proposal exact target scope | Proposal exact opposite | Proposal approx opposite | Proposal improved L1 | Best L1 before | Best L1 after |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `top12_cap4_m1` | `unknown` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` | `26889` | `1600` | `1287` | `not_applicable_4x4_overlay_vs_2x2_endpoint` | `0` | `0` | `225` | `14` | `8` |
| `top24_cap4_m1` | `unknown` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` | `26903` | `2978` | `2227` | `not_applicable_4x4_overlay_vs_2x2_endpoint` | `0` | `1` | `343` | `14` | `8` |
| `top12_cap4_m2` | `unknown` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` | `27063` | `9795` | `8904` | `not_applicable_4x4_overlay_vs_2x2_endpoint` | `0` | `0` | `499` | `14` | `8` |

The wider top-24, `m=1` pass has exactly one proposal-level approximate
opposite-side hit. It is not an exact frontier hit and it worsens the recorded
counterpart distance:

| Field | Value |
| --- | --- |
| source rank | `14` |
| direction / proposal depth | `forward` / `20` |
| deleted vertex | `2` |
| seam | `principal_3x3_bridge_return_outsplit` |
| exact opposite-frontier hit | `false` |
| approximate opposite-frontier hit | `true` |
| exact L1 to retained counterpart | `24 -> 31` |

Several off-frontier proposals improve exact L1 to the recorded counterpart,
for example rank `8` improves `34 -> 18` under the out-split return overlay.
Those are ranking-only signals: the improved proposals are not exact endpoint
hits because the overlay is `4x4` while the endpoints are `2x2`; they are also
not exact opposite-frontier hits and not approximate opposite-frontier hits.

## Reading

Reject this balanced bridge proposal surface for solver integration. That
conclusion remains unchanged after restricting proposal hit checks to retained
discovered/enqueued opposite-frontier states.

The useful retained lane evidence remains the sparse `4x4` active-block layout
misses from the stuck-state inventory, but this balanced elementary overlay
does not turn that evidence into a retained-frontier bridge:

- retained-run exact target hits stay at `0`, while proposal endpoint hits are
  not dimension-compatible for this report-only `4x4` overlay;
- exact opposite-frontier proposal hits stay at `0`;
- the only proposal-level approximate hit worsens exact counterpart L1;
- the stronger top-12 `m=2` balanced bound generates `8904` unique proposals
  and still has `0` approximate opposite-frontier hits; and
- the positive L1 improvements are off-frontier projection artifacts, not a
  valid balanced/SSE move family.

This does not collapse to the already rejected active-block `2x2` contingency
switch: the generated proposals come from balanced elementary bridge-return
seams on principal `3x3` windows, not from row/column-sum-preserving switches
inside the active `2x4` or `4x2` block. The conclusion is similar, though:
there is some local distance signal, but no exact or useful approximate bridge
under the retained budget.

No follow-up bead was opened.

## Commands

```bash
timeout -k 20s 180s cargo build --features research-tools \
  --bin extract_brix_ruiz_k4_stuck_states \
  --bin research_harness

timeout -k 20s 120s target/debug/extract_brix_ruiz_k4_stuck_states \
  --json-out tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_stuck_states_2026-04-25_nw7_4.json \
  --top 220

timeout -k 20s 180s cargo build --features research-tools \
  --bin evaluate_brix_ruiz_k4_balanced_bridge_proposals

timeout -k 20s 90s target/debug/evaluate_brix_ruiz_k4_balanced_bridge_proposals \
  --top-hits 4 \
  --bridge-max-entry 2 \
  --max-common-dim 1 \
  --max-entry 2 \
  --json-out tmp/brix_ruiz_k4_balanced_bridge_proposals_small_2026-04-25_nw7_4.json

timeout -k 20s 180s target/debug/evaluate_brix_ruiz_k4_balanced_bridge_proposals \
  --top-hits 12 \
  --bridge-max-entry 4 \
  --max-common-dim 1 \
  --max-entry 4 \
  --json-out tmp/brix_ruiz_k4_balanced_bridge_proposals_top12_cap4_2026-04-25_nw7_4.json

timeout -k 20s 180s target/debug/evaluate_brix_ruiz_k4_balanced_bridge_proposals \
  --top-hits 24 \
  --bridge-max-entry 4 \
  --max-common-dim 1 \
  --max-entry 4 \
  --json-out tmp/brix_ruiz_k4_balanced_bridge_proposals_top24_cap4_2026-04-25_nw7_4.json

timeout -k 20s 180s target/debug/evaluate_brix_ruiz_k4_balanced_bridge_proposals \
  --top-hits 12 \
  --bridge-max-entry 4 \
  --max-common-dim 2 \
  --max-entry 4 \
  --json-out tmp/brix_ruiz_k4_balanced_bridge_proposals_top12_cap4_m2_2026-04-25_nw7_4.json
```

## Validation

```bash
timeout -k 20s 180s cargo build --features research-tools \
  --bin evaluate_brix_ruiz_k4_balanced_bridge_proposals

timeout -k 20s 180s cargo test --features research-tools \
  --bin evaluate_brix_ruiz_k4_balanced_bridge_proposals
```

Both commands passed before this note was written. Final session validation
also ran `cargo fmt --all`.
