# Hard Brix-Ruiz `k = 3` non-Baker lag-7 exact-endpoint guided replay control (2026-04-20)

## Goal

Freeze exactly one durable repo-owned control for the explicit non-Baker lag-7
exact-endpoint witness on the hard Brix-Ruiz `k = 3` pair
`[[1,3],[2,1]] -> [[1,6],[1,1]]`, without reopening endpoint multi-meet search
or broadening the general shortcut tooling surface.

## Chosen surface

Keep one committed harness worker-case only:

- `brix_ruiz_k3_non_baker_exact_endpoint_lag7_guided_replay`

This is a guide-backed replay control, not a new search lane. It reuses the
already promoted exact-endpoint witness artifact:

- [`research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`](../guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json)

on the existing hard endpoint fixture:

- [`research/fixtures/brix_ruiz_family.json#brix_ruiz_k3`](../fixtures/brix_ruiz_family.json)

The harness case runs `guided_refinement` with only that one artifact loaded and
with a deliberately tiny refinement budget (`max_shortcut_lag = 1`, `max_gap =
2`, `rounds = 1`). The case also pins the full expected witness matrix
signature in `research/cases.json`.

On this surface the artifact is first validated directly as a full SSE path on
the requested endpoints; the stage only replaces it if it finds a strictly
shorter replay. The harness then compares the returned path against the pinned
non-Baker signature and fails the case on any mismatch. That keeps this slice
as a direct witness-validation control instead of a broader replay/search
probe.

## Why this is the right bounded control

Keep:

- one durable repo-owned command that validates the explicit non-Baker witness
  directly on the hard exact endpoints;
- the committed `2026-04-19` exact-endpoint replay artifact as the sole witness
  input; and
- one pinned witness signature so the control fails if replay ever collapses to
  Baker or to any other different lag-7 path; and
- the existing `research_harness` control layer instead of introducing new
  replay-specific plumbing.

Reject:

- reopening endpoint multi-meet retention or higher-lag replay probing;
- adding a generic witness-enumeration surface; and
- treating this as a replacement for the already committed Baker-family control.

## Baker vs non-Baker distinction

The control is intentionally distinct from the committed Baker-family witness in
[`research/guide_artifacts/k3_normalized_guide_pool.json`](../guide_artifacts/k3_normalized_guide_pool.json).

Frozen non-Baker lag-7 path starts:

```text
2x2:1,3,2,1
-> 3x3:0,1,0,2,1,2,1,2,1
-> 4x4:1,0,1,1,2,1,0,2,2,1,0,1,2,1,0,0
```

Committed Baker-family lag-7 path starts:

```text
2x2:1,3,2,1
-> 3x3:1,2,2,2,1,1,1,0,0
-> 4x4:1,2,2,0,1,0,2,0,0,1,1,1,1,1,2,0
```

So this control does not merely say "some lag-7 witness exists". It freezes the
alternate exact-endpoint family produced by the bounded 2026-04-19 multi-meet
replay artifact.

## Reproduce / Validate

Build the focused harness binary:

```bash
cargo build --profile dist --features research-tools --bin research_harness
```

Run the single kept control:

```bash
timeout -k 5s 20s target/dist/research_harness \
  --cases research/cases.json \
  --worker-case brix_ruiz_k3_non_baker_exact_endpoint_lag7_guided_replay
```

Observed on current head:

- outcome `equivalent`
- the case accepts exactly one guide artifact, the committed non-Baker lag-7
  replay witness
- the returned path matches the pinned non-Baker witness signature exactly
- the control stays on the hard exact endpoints and validates that witness
  without a fresh multi-meet search round

## Final reading

Final reading: keep the one worker-case replay control.

It is the narrowest durable surface that directly validates the explicit
non-Baker lag-7 exact-endpoint witness on the hard Brix-Ruiz `k = 3` pair while
remaining clearly separate from the Baker-family control already retained in the
normalized guide pool.
