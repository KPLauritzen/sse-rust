# Trimmed active-window coarse-bucket ranking experiment for square endpoint-near states (2026-04-26)

## Question

For bead `sse-rust-hoxd`, take the retained `w7e4` three-way signal

- `reuse_endpoint_local_parity`
- `rank_or_propose_inside_coarse_bucket`
- `ignore`

and test it as a **bounded ranking/proposal experiment** against the current
coarse approximate bucket alone for square `3x3` / `4x4` endpoint-near states.

Hard boundaries for this slice:

- no production canonicalization changes;
- no hard dedup key, hard parity filter, or claimed SSE invariant;
- no broad search run; and
- no default search-policy retune.

## Experiment Surface

I extended the existing research-only helper:

- `src/bin/diagnose_endpoint_neighborhood_normal_forms.rs`

The helper still emits the paired parity report from `sse-rust-w7e4`, but it
now also emits a **bucket experiment section** for each paired sample:

1. choose an anchor square endpoint-near sample;
2. compare it to a bounded candidate pool:
   - `k = 3` control: cross-guide witness states of the same endpoint side and
     dimension;
   - retained `k = 4` lane: opposite-frontier retained stuck counterparts of
     the same dimension;
3. record what the current coarse bucket alone would surface; and
4. record how the three-way trimmed signal changes the action inside that
   already-matched bucket.

This is still diagnostic only. The solver search path is unchanged.

## Reproducible Commands

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms
timeout -k 20s 180s cargo run --features research-tools --bin extract_brix_ruiz_k4_stuck_states -- \
  --json-out tmp/sse-rust-hoxd-k4-stuck-top16.json \
  --top 16
timeout -k 20s 180s cargo run --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms -- \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --stuck-report tmp/sse-rust-hoxd-k4-stuck-top16.json \
  --endpoint-radius 3 \
  --top-stuck 8 \
  --json-out tmp/sse-rust-hoxd-trimmed-active-window-ranking.json
```

Artifacts:

- stuck report: `tmp/sse-rust-hoxd-k4-stuck-top16.json`
- bucket experiment report: `tmp/sse-rust-hoxd-trimmed-active-window-ranking.json`

## Overall Result

The paired parity split remains the same as `w7e4`:

| Pair kind | Count | Coarse match | Trimmed match | Three-way action |
| --- | ---: | ---: | ---: | --- |
| `k3_witness_replay_overlap` | `4` | `4/4` | `4/4` | `reuse_endpoint_local_parity` |
| `k4_stuck_vs_counterpart` | `8` | `8/8` | `0/8` | `rank_or_propose_inside_coarse_bucket` |

The new bucket experiment section adds one important limitation:

- the `k = 3` controls do upgrade cleanly from "coarse hit" to explicit
  `reuse_endpoint_local_parity`;
- the retained `k = 4` lanes do **not** gain a new exact-within-bucket ranking
  separation in this bounded sample; they stay in the proposal-only tier.

So the signal is useful for **action labeling inside the coarse bucket**, but
this slice does **not** show evidence that it is already a strong standalone
rank tiebreaker for retained `k = 4` stuck lanes.

## Comparison Against Coarse Bucket Alone

### Control: known `k = 3` witness replay overlap

Representative control:

- anchor:
  `k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19` step `2` / source
- expected candidate:
  `k3_shortcut_round1` step `2` / source
- coarse signature:
  `d4|sum15|rs3,3,4,5|cs1,3,4,7|rS2,3,3,3|cS1,3,3,4`
- trimmed active window:
  `4x4|0,0,1,2,1,0,1,2,2,0,1,2,1,1,0,1`

Observed comparison:

| Surface | Candidate pool | Coarse-bucket candidates | Outcome |
| --- | ---: | ---: | --- |
| coarse bucket alone | `2` | `1` | the replay partner is surfaced only as a coarse hit |
| three-way signal | `2` | `1 reuse`, `0 mismatch`, `1 ignore` | the same candidate is upgraded to `reuse_endpoint_local_parity` |

Observed effect:

- no broader rank movement was needed here because the coarse bucket already had
  one candidate; but
- the three-way signal turns an undifferentiated approximate hit into an
  explicit "same endpoint-local square surface" reuse decision.

That is a real quality improvement, even though it is classification rather
than multi-candidate reranking.

### Retained `k = 4` lane: rank `2` elementary conjugation

Representative retained lane:

- anchor: `k4_stuck_rank2_to`
- expected candidate: `k4_stuck_rank2_counterpart`
- competing coarse-bucket candidate: `k4_stuck_rank6_counterpart`
- coarse signature:
  `d4|sum23|rs3,4,5,11|cs0,0,6,17|rS1,2,2,2|cS0,0,3,4`

Observed comparison:

| Surface | Candidate pool | Coarse-bucket candidates | Outcome |
| --- | ---: | ---: | --- |
| coarse bucket alone | `8` | `2` | rank `2` and rank `6` counterparts are tied as approximate hits |
| three-way signal | `8` | `0 reuse`, `2 mismatch`, `6 ignore` | both coarse-bucket candidates remain `rank_or_propose_inside_coarse_bucket` |

Observed effect:

- the signal does **not** create a new exact top tier inside this retained
  `k = 4` bucket;
- it does prevent a false upgrade to `reuse_endpoint_local_parity`; and
- it preserves the right interpretation: these are proposal/ranking cues inside
  the coarse bucket, not exact endpoint-local reuse.

This is the important negative result in the bounded slice: the signal is
useful for keeping the proposal surface honest, but not yet for confidently
ordering retained `k = 4` coarse neighbors by itself.

## Keep / Reject Decision

Decision: **keep, but only as a coarse-bucket action label and proposal-surface
helper. Reject it as a standalone ranking lift for retained `k = 4` lanes on
current evidence.**

Why:

- the `k = 3` control stays clean:
  coarse hit becomes explicit `reuse_endpoint_local_parity`;
- the retained `k = 4` evidence stays disciplined:
  coarse hits remain coarse-only mismatches, with no false exact reuse;
- the bounded report does **not** show a new within-bucket exact-trimmed winner
  for the retained `k = 4` lane where coarse bucket size exceeds `1`; and
- that means the right use is proposal labeling, not aggressive reranking.

## Exact Next Integration Boundary

If this is integrated further, the next boundary should be:

- an **opt-in report/telemetry helper** on square `3x3` / `4x4`
  `approximate_other_side_hits` that annotates each coarse bucket hit with one
  of:
  - `reuse_endpoint_local_parity`
  - `rank_or_propose_inside_coarse_bucket`
  - `ignore`

Do **not** yet:

- feed it into default beam ordering;
- use it as a hard prune;
- use it as a hard dedup key; or
- claim it breaks ties productively among retained `k = 4` coarse neighbors.

## Validation

Observed results:

- `cargo fmt --all` passed;
- `cargo test --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms` passed (`21` tests);
- the bounded stuck-state extractor wrote
  `tmp/sse-rust-hoxd-k4-stuck-top16.json`; and
- the bounded helper wrote
  `tmp/sse-rust-hoxd-trimmed-active-window-ranking.json`.
