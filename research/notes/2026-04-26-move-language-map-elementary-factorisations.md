# Move-language map for selected elementary SSE factorisations (2026-04-26)

## Goal

Treat the currently implemented graph generators and structured factorisation
families as a small move language, then classify a bounded comparison set of
elementary SSE transitions by the shortest currently known word over that
language.

This is a note/prototype slice only:

- no new solver family;
- no broad factorisation enumeration;
- no beam or policy retuning; and
- no attempt to replace the existing Baker-only graph-expansion note.

## Current move-language view

For this note, the implemented vocabulary is read in three layers.

Graph generators already exposed directly by the search:

- `outsplit`
- `insplit`
- `out_amalgamation`
- `in_amalgamation`
- `permutation_relabeling`

Retained graph-coded factorisation labels still admitted under the
`graph_only` policy at the low-dimensional cap:

- `rectangular_factorisation_2x3`
- `rectangular_factorisation_3x3_to_2`
- `elementary_conjugation_3x3`

Additional `graph_plus_structured` labels relevant to this slice:

- `binary_sparse_rectangular_factorisation_3x3_to_4`
- `binary_sparse_rectangular_factorisation_4x3_to_3`
- `diagonal_refactorization_3x3`
- `diagonal_refactorization_4x4`
- `elementary_conjugation`

So a transition may be:

1. already one word in the current language;
2. not one word, but a short mixed word over existing labels; or
3. outside the current vocabulary at reasonable bounded search, leaving only a
   missing-word pattern.

## Selected transitions and why

The comparison set stays small and deliberately mixed:

- Baker `A1 -> A2`, `A4 -> A5`, and `A5 -> A6`.
  These are the useful Baker controls because the repo already knows short
  graph-only expansions for them, and `A4 -> A5` is the only remaining
  non-one-step Baker literal under `graph_plus_structured`.
- One non-Baker `k = 3` control step: the non-Baker plateau exit
  `N4 -> N5` (`4x4 -> 3x3`).
  This is the cleanest contrast to the hard Baker `4x4 -> 4x4` step.
- One retained Brix-Ruiz `k = 4` structured approximate-hit pair: the rank-4
  `diagonal_refactorization_4x4` near-hit from the retained
  `graph_plus_structured` stuck-state extractor.
  This is the best currently recorded dim-4 structured near-miss rather than a
  synthetic open-ended control.

## Move-language classification table

Notation:

- Baker waypoints use the Lind-Marcus/Baker guide indexing from
  `research/guide_artifacts/k3_shortcut_round1.json`.
- Non-Baker waypoints use
  `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`.
- "Best current word" means the shortest exact expression I could justify from
  current bounded artifacts, not a global optimality claim.

| Transition | Why selected | Best current word over implemented vocabulary | Graph-only reading | Current reading |
| --- | --- | --- | --- | --- |
| Baker `A1 -> A2` (`3x3 -> 4x4`) | Former Baker missing lift control | `binary_sparse_rectangular_factorisation_3x3_to_4` | short graph-only word already known from the waypoint expansion: length `5` | Now clearly expressible as one structured word; graph-only spelling exists but is longer. |
| Baker `A4 -> A5` (`4x4 -> 4x4`) | Hard Baker same-size control | `binary_sparse_rectangular_factorisation_4x3_to_3 -> outsplit -> permutation_relabeling` | short graph-only word already known from the waypoint expansion: length `6`; the dim-5 classifier also re-finds a bounded graph-only path | Not one current one-step family. The best known exact expression is a heterogeneous bridge through `3x3`, not a reusable same-size family label. |
| Baker `A5 -> A6` (`4x4 -> 3x3`) | Former Baker missing contraction control | `binary_sparse_rectangular_factorisation_4x3_to_3` | short graph-only word already known from the waypoint expansion: length `3` | Now clearly expressible as one structured word; graph-only spelling exists but is longer. |
| Non-Baker `N4 -> N5` (`4x4 -> 3x3`) | Non-Baker plateau-exit control | `binary_sparse_rectangular_factorisation_4x3_to_3` | no exact graph-coded one-step label on the witness itself; bounded dim-4 probe does not produce a graph-only witness | The non-Baker witness leaves the `4x4` plateau with a direct structured contraction where Baker still needs a same-size layout change. |
| Retained Brix-Ruiz `k = 4` rank-4 near-hit child vs closest opposite-side counterpart | Best retained structured dim-4 near-hit | source-to-child is `diagonal_refactorization_4x4`; no exact word is known from the child to the opposite-side counterpart under the retained bounded surface | not an exact graph-only transition; this is an approximate-hit pair, not a proved SSE edge | The current language reaches the right sparse signature/profile surface, but not the right active `2 x 4` layout. This is the strongest retained missing-word signal. |

## Concrete readings behind the table

### 1. Baker `A4 -> A5` is the only selected exact step that is still not one word

The current dim-4 classifier reports no direct `graph_plus_structured` or
`mixed` family match for this `4x4 -> 4x4` elementary SSE. But the step is not
opaque anymore. Under the existing vocabulary it factors as the short exact
word

```text
binary_sparse_rectangular_factorisation_4x3_to_3
-> outsplit
-> permutation_relabeling
```

via the bridge

```text
4x4:1,2,2,0,1,1,1,1,0,1,0,1,0,2,1,0
-> 3x3:1,1,1,3,0,2,1,1,1
-> 4x4:1,1,1,1,3,0,2,2,1,0,0,0,0,1,1,1
```

So the missing object is not "some generic 4x4 family". It is a missing
same-support `4x4` layout-transfer word that the current language can only
spell by dropping to `3x3` and re-expanding.

### 2. The non-Baker witness exits earlier instead of solving the same internal 4x4 problem

The chosen non-Baker control `N4 -> N5` is already a one-word
`binary_sparse_rectangular_factorisation_4x3_to_3` contraction. That is the
useful contrast: the non-Baker path does not offer a second hard same-size
`4x4 -> 4x4` elementary step that repeats Baker's exact obstruction. It avoids
it by leaving the `4x4` plateau.

So the `k = 3` evidence does **not** support "missing generic 4x4 contraction"
as the next family hypothesis. The hard object is narrower.

### 3. The retained Brix-Ruiz `k = 4` near-hit repeats the same narrow theme

The retained extractor's best structured dim-4 signal is the rank-4 forward
diagonal step

```text
[[1,4,1,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]
->diag(1,1,2,1)-step->
[[1,4,2,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]
```

whose closest opposite-side same-signature counterpart is

```text
[[1,12,0,1],[1,1,4,4],[0,0,0,0],[0,0,0,0]]
```

The retained note already records that these states share sorted row sums
`0/0/10/14`, sorted column sums `2/4/5/13`, and the same sparse support
profile. The miss is the active `2 x 4` layout inside that sparse `4x4`
boundary state.

That is materially similar to the Baker `A4 -> A5` reading: the language can
reach the right coarse sparse `4x4` surface, but not directly perform the
internal active-block transfer as a short reusable word.

## Repeated missing-word pattern

Keep this line of research, but only in a very narrow form.

The repeated missing-word pattern is:

> sparse same-profile `4x4` layout transfer, especially inside a two-active-row
> / two-zero-row boundary state, where the current language can match coarse
> row/column totals and support profile but not the internal active-block
> placement in one short direct word.

Evidence for that pattern comes from two different places:

- Baker `A4 -> A5`: exact elementary SSE exists, but not as one current
  same-size family; the best exact word drops to `3x3` and re-expands.
- Retained Brix-Ruiz `k = 4` rank-4 and rank-6 diagonal near-hits: current
  structured moves reach matching sparse signatures and still miss the
  opposite-side layout.

This is specific enough to keep. It is not the same as reopening generic
`4x4 -> 3` admission gates, broad `4x4` factorisation enumeration, or another
beam retune.

## Keep / reject decision

Keep:

- the move-language framing itself, because it compresses several older results
  into one reusable question: which elementary transitions are one words, short
  words, or missing words over the currently implemented vocabulary;
- Baker `A4 -> A5` as the exact dim-4 same-size control; and
- retained sparse-`4x4` diagonal near-hits as the only credible open-ended
  analog currently visible in Goal 3 work.

Reject:

- reopening broad factorisation enumeration for `4x4`;
- treating the non-Baker witness as evidence for a second hard same-size `4x4`
  family;
- generic graph-only widening as the answer, because the exact issue is now the
  shape of a short word, not existence of some long graph expansion; and
- broadening to arbitrary `k >= 4` factorisation mining.

## Concrete justified follow-up

If this line is continued, the next slice should be exactly:

> probe one bounded sparse-`4x4` active-block transfer proposal, seeded only
> from Baker `A4 -> A5` and the retained Brix-Ruiz `k = 4` diagonal near-hit
> pairs, and test whether a tiny contingency-switch-style move can reduce exact
> canonical distance while preserving the observed row/column profile.

That is concrete enough to distinguish from generic enumeration, and it matches
the already-open `sse-rust-nw7.7` direction. No new follow-up bead was opened.

## Commands and artifacts used

Focused builds:

```bash
timeout -k 20s 180s cargo build --features research-tools \
  --bin classify_witness_steps \
  --bin explain_witness_step \
  --bin extract_brix_ruiz_k4_stuck_states
```

Step classification artifacts:

```bash
timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/oc0-baker-dim4.json

timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 5 \
  --graph-probe-max-entry 5 \
  > tmp/oc0-baker-dim5.json

timeout -k 20s 180s target/debug/classify_witness_steps \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --factorisation-max-entry 5 \
  --match-up-to-permutation \
  --graph-probe-max-lag 7 \
  --graph-probe-max-intermediate-dim 4 \
  --graph-probe-max-entry 5 \
  > tmp/oc0-non-baker-dim4.json
```

Bridge decomposition artifacts for the hard Baker step:

```bash
timeout -k 20s 180s target/debug/explain_witness_step \
  --from 4x4:1,2,2,0,1,1,1,1,0,1,0,1,0,2,1,0 \
  --to 3x3:1,1,1,3,0,2,1,1,1 \
  --graph-max-lag 3 \
  --graph-max-intermediate-dim 4 \
  --graph-max-entry 5 \
  --factorisation-max-entry 5 \
  --write-json tmp/oc0-baker-step5-bridge-a4-to-3x3.json \
  > tmp/oc0-baker-step5-bridge-a4-to-3x3.stdout.json

timeout -k 20s 180s target/debug/explain_witness_step \
  --from 3x3:1,1,1,3,0,2,1,1,1 \
  --to 4x4:1,1,1,1,3,0,2,2,1,0,0,0,0,1,1,1 \
  --graph-max-lag 3 \
  --graph-max-intermediate-dim 4 \
  --graph-max-entry 5 \
  --factorisation-max-entry 5 \
  --write-json tmp/oc0-baker-step5-bridge-3x3-to-a5.json \
  > tmp/oc0-baker-step5-bridge-3x3-to-a5.stdout.json
```

Retained Brix-Ruiz `k = 4` stuck-state extraction:

```bash
timeout -k 20s 180s target/debug/extract_brix_ruiz_k4_stuck_states \
  --json-out tmp/oc0-brix-k4-stuck-states.json \
  --top 40
```

Older durable notes/artifacts used for context and the known Baker graph-only
word lengths:

- `docs/brix-ruiz-sidecar-log.md`
- `research/notes/2026-04-25-baker-lag7-structured-classification.md`
- `research/notes/2026-04-20-k3-non-baker-exact-endpoint-lag7-guided-replay-control.md`
- `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`

## Validation

Focused validation run in this session:

- the three research binaries above built successfully;
- the three classifier commands completed successfully and produced the
  `tmp/oc0-*.json` artifacts;
- both `explain_witness_step` bridge probes completed successfully; and
- the retained stuck-state extractor completed successfully with `--top 40`.

Repo-wide formatting and branch review are recorded after this note update in
the session close-out.
