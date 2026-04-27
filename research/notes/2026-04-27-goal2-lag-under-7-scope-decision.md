# Goal 2 lag-under-7 scope decision (2026-04-27)

## Question

Decide whether another bounded Goal 2 slice is justified right now for the
hard Brix-Ruiz `k = 3` exact endpoint pair:

```text
[[1,3],[2,1]] -> [[1,6],[1,1]]
```

This is a scoping decision only. It does not rerun broad shortcut search,
guide-pool rebuilding, endpoint multi-meet replay, or a blind witness hunt.

## Evidence reviewed

- `bd show sse-rust-cu9e --json`
- `bd show sse-rust-7jkd --json`
- `research/notes/2026-04-25-k3-lag7-bottleneck-classification.md`
- `research/notes/2026-04-26-last-24h-repo-summary.md`
- `research/notes/2026-04-25-exact-endpoint-witness-inventory-surface.md`
- `research/notes/2026-04-26-guided-segment-approximate-hit-parity-propagation.md`
- `research/notes/2026-04-26-approximate-hit-parity-label-calibration.md`
- `research/notes/2026-04-25-witness-bridge-motif-inventory.md`
- `research/program.md`
- Focused artifact reads from:
  - `research/guide_artifacts/k3_shortcut_round1.json`
  - `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`

Apart from tracker updates, the only evidence-extraction commands beyond
note/tracker reads were bounded `rg`, `jq`, and `bd` inspection commands with
short `timeout -k` guards.

## Current bottleneck reading

The strongest current reading is still the one from the lag-7 bottleneck
classification: the hard `k = 3` exact witnesses share a support/dimension
envelope, not a single mandatory canonical waypoint.

The repeated envelope is:

```text
2x2 source
-> sparse 3x3
-> sparse support-11 4x4
-> support-11 4x4 plateau
-> 3x3 or 2x2 contraction
-> 2x2 target
```

The exact endpoint inventory adds four retained exact-meet rows with meet lag
`7` and reconstructed path length `8`, but those rows do not exactly match the
pinned Baker or non-Baker lag-7 controls. That supports the same conclusion:
there is diversity inside the lag-7 envelope, not a single hidden waypoint to
attack.

The approximate-hit parity notes do not promote a Goal 2 experiment. The
propagated report is useful as diagnostics, but the calibration note found
`reuse_endpoint_local_parity` only in unknown segment scopes on the bounded
`k = 3` shortcut replay (`0 / 7` reuse hits in exact-success scopes). Treating
those labels as segment selectors would be premature.

## Candidate segment-replacement hypotheses considered

### Keep: direct entry-corridor replacement

Hypothesis: replace the two-edge entry corridor

```text
2x2 source -> 3x3 sparse lift -> first sparse support-11 4x4 envelope state
```

with one direct lag-1 rectangular ESSE step from the common source to the first
`4x4` envelope state in a pinned lag-7 control.

This is the only candidate that is both concrete and capable of moving the
existing lag-7 controls below lag `7` without another replay pass. If a direct
`2x2 -> 4x4` step exists for either pinned entry segment, the remaining suffix
of that same guide would stitch to a lag `<= 6` source-to-target path.

Pinned control segments:

```text
Baker positions 0..2:
2x2: [1,3,2,1]
3x3: [1,2,2,2,1,1,1,0,0]
4x4: [1,2,2,0,1,0,2,0,0,1,1,1,1,1,2,0]

Non-Baker positions 0..2:
2x2: [1,3,2,1]
3x3: [0,1,0,2,1,2,1,2,1]
4x4: [1,0,1,1,2,1,0,2,2,1,0,1,2,1,0,0]
```

This is not a reason to add a default move family. The witness-bridge motif
inventory already says the sparse `k = 3` entry corridor is covered by existing
two-step families and should be rejected as a default new family. The kept slice
is narrower: an exact one-step replacement probe for two pinned entry targets.

### Reject: return-side direct `4x4 -> 2x2` replacement

The return side is less clean. Baker and non-Baker diverge near the end, the
non-Baker path uses a sparse `2x2` bridge before the target, and the bottleneck
classification already says the final return differences do not expose a
one-step shortening. There is no single return segment with the same precision
as the shared entry corridor.

### Reject: same-size `4x4` plateau replacement

The plateau is the real structural bottleneck, but current evidence does not
isolate one reusable replacement. The Baker hard same-size step is
heterogeneous, non-Baker exits earlier through `4x4 -> 3x3`, retained exact
paths remain canonically distinct, and the motif inventory rejects promoting a
default same-size `4x4` family from the observed plateau layouts.

### Reject: parity-label-guided segment choice

The latest calibration keeps approximate-hit parity labels as diagnostics only.
They are not predictive enough to select a new Goal 2 segment. In particular,
the rare `reuse_endpoint_local_parity` hits landed only in unknown child
endpoint-search scopes on the bounded `k = 3` replay.

### Reject: endpoint-orientation inventory follow-up as Goal 2 slice

Recording exact-meet orientation remains useful if later analysis needs it, and
`sse-rust-379` already owns that narrow surface. It is not a segment-replacement
hypothesis and does not itself plausibly move lag below `7`.

## Decision

Keep exactly one bounded follow-up slice: `sse-rust-7jkd`.

I did not open a second bead. `sse-rust-7jkd` already existed as the dependent
Goal 2 follow-up, so I tightened it to this single concrete hypothesis instead:
probe a direct lag-1 `2x2 -> 4x4` entry replacement for the pinned Baker and
non-Baker lag-7 controls.

Updated `sse-rust-7jkd` bounds:

- Replacement lag: exactly `1`; stitched source-to-target path must be lag
  `<= 6` to count.
- Dimensions: only `2x2 -> 4x4` entry targets; `U` shape `2x4`, `V` shape
  `4x2`; no dimension `3` search and no dimension `> 4`.
- Entries: nonnegative integer `U,V` entries `<= 5`; no generated matrix entry
  `> 5`.
- Guide artifacts:
  - `research/guide_artifacts/k3_shortcut_round1.json`
  - `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`
- Timeout: `timeout -k 10s 180s` total.
- Max attempts: at most two target controls and at most `500000` candidate
  factor-pair attempts per target.
- Hard exclusions: no broad guide-pool rebuild, no endpoint multi-meet replay,
  no generic shortcut replay, no beam/ranking/pruning/dedup/canonicalization
  change, and no default solver move-generation change.

## Conditions for reopening Goal 2 beyond this slice

Open more Goal 2 work only if one of the following becomes true:

- `sse-rust-7jkd` finds a direct entry replacement or a near miss that identifies
  one exact smaller subproblem with stricter bounds.
- A future exact endpoint inventory records orientation or retained segment
  data that isolates one concrete plateau replacement, not just another lag-7
  envelope instance.
- Approximate-hit diagnostics gain per-segment accepted-edge linkage and show a
  positive held-out concentration in exact-success scopes.
- A new note names a specific consecutive subpath, a smaller target lag, and
  explicit dimension/entry/attempt bounds before any search is run.

Until then, another broad shortcut replay, guide-pool rebuild, multi-meet run,
or parity-label-driven witness hunt is not justified.
