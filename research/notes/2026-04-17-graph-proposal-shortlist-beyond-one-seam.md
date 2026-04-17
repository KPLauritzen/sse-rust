# Graph-proposal shortlist beyond one seam (2026-04-17)

## Question

Does the bounded graph-proposal shortlist signal from
`brix_ruiz_k3` `guide:1 -> guide:15` survive on any additional comparable seam,
or is that first win just a narrow curiosity?

## Round Shape

This round kept the established comparison surface from
[`docs/graph-proposal-shortlist-rounds.md`](../../docs/graph-proposal-shortlist-rounds.md):

- use `compare_graph_move_proposals`, not `research_harness`;
- compare blind one-step graph successors against targeted graph proposals on
  the same quotient-gap fields;
- keep the realization probe graph-only and bounded;
- leave default search behavior unchanged.

All captured outputs were saved in repo-local scratch files under `tmp/`:

- `tmp/2026-04-17-brix-guide1-guide15.txt`
- `tmp/2026-04-17-brix-guide2-guide14.txt`
- `tmp/2026-04-17-brix-guide1-target.txt`
- `tmp/2026-04-17-brix-source-guide15.txt`
- `tmp/2026-04-17-guided-permutation-3x3-source-target.txt`
- `tmp/2026-04-17-guided-permutation-4x4-source-target.txt`

## Evidence

### 1. Anchor: `brix_ruiz_k3` `guide:1 -> guide:15`

Command:

```sh
timeout -k 5s 20s target/dist/compare_graph_move_proposals \
  --fixture-ref research/fixtures/brix_ruiz_family.json#brix_ruiz_k3 \
  --seeded-guide-id endpoint_16_path \
  --current guide:1 \
  --target guide:15 \
  --max-dim 4 \
  --zigzag-bridge-entry 8 \
  --top-k 6 \
  --probe-lag 3 \
  --probe-shortlist-k 4
```

Result:

- blind successors: `28` raw / `27` unique, dimensions `2x2:1, 4x4:26`;
- blind best gap: `dim=1 row=2 col=18 entry_sum=4`, shortlist `1`;
- targeted proposals: `644` raw / `33` unique, dimensions
  `2x2:1, 3x3:6, 4x4:26`, overlap with blind `27`;
- proposal best gap: `dim=0 row=2 col=6 entry_sum=0`, shortlist `1`;
- bounded realization: the shortlisted proposal is a non-blind `3x3`
  zig-zag candidate, realized in `3` graph-only steps with
  `frontier_nodes=2` and `visited=44`.

Verdict:

- the original seam still holds up as a real shortlist win;
- this remains the only tested seam where the best proposal is both
  `blind_overlap=false` and strictly better than the best blind one-step gap.

### 2. Additional Brix-Ruiz waypoint seam: `guide:2 -> guide:14`

Command:

```sh
timeout -k 5s 15s target/dist/compare_graph_move_proposals \
  --fixture-ref research/fixtures/brix_ruiz_family.json#brix_ruiz_k3 \
  --seeded-guide-id endpoint_16_path \
  --current guide:2 \
  --target guide:14 \
  --max-dim 5 \
  --zigzag-bridge-entry 8 \
  --top-k 6 \
  --probe-lag 3 \
  --probe-shortlist-k 4
```

Result:

- blind successors: `53` raw / `51` unique, dimensions `3x3:2, 5x5:49`;
- blind best gap: `dim=1 row=8 col=9 entry_sum=2`, shortlist `1`;
- targeted proposals: `6122` raw / `51` unique, same dimension breakdown,
  overlap with blind `51`;
- proposal best gap: exactly the same
  `dim=1 row=8 col=9 entry_sum=2`, shortlist `1`;
- bounded realization: the single shortlisted proposal is realized in `2`
  steps, but it is `blind_overlap=true`.

Verdict:

- no shortlist advantage survived on this second Brix-Ruiz waypoint seam;
- the proposal lane only re-described the blind one-step surface while paying a
  much larger raw-candidate cost.

### 3. Additional Brix-Ruiz endpoint-derived seam: `guide:1 -> target`

Command:

```sh
timeout -k 5s 15s target/dist/compare_graph_move_proposals \
  --fixture-ref research/fixtures/brix_ruiz_family.json#brix_ruiz_k3 \
  --seeded-guide-id endpoint_16_path \
  --current guide:1 \
  --target target \
  --max-dim 4 \
  --zigzag-bridge-entry 8 \
  --top-k 6 \
  --probe-lag 3 \
  --probe-shortlist-k 4
```

Result:

- blind successors: `28` raw / `27` unique, dimensions `2x2:1, 4x4:26`;
- blind best gap: `dim=0 row=4 col=4 entry_sum=2`, shortlist `1`;
- targeted proposals: `644` raw / `33` unique, dimensions
  `2x2:1, 3x3:6, 4x4:26`, overlap with blind `27`;
- proposal best gap: the same `dim=0 row=4 col=4 entry_sum=2`, shortlist `1`;
- bounded realization: the best proposal is the same blind-overlapping `2x2`
  out-amalgamation, realized in `2` steps with `visited=28`.

Verdict:

- the extra `3x3` zig-zag lane still exists here, but it loses to the blind
  endpoint-facing `2x2` candidate;
- this seam therefore does **not** reproduce the original shortlist win.

### 4. Extra Brix-Ruiz endpoint control: `source -> guide:15`

Result:

- blind successors: `14` raw / `14` unique, all `3x3`;
- targeted proposals: `84` raw / `14` unique, overlap with blind `14`;
- blind and proposal best gap are identical:
  `dim=0 row=1 col=4 entry_sum=1`, shortlist `1`;
- the bounded realization is `2` steps and `blind_overlap=true`.

Verdict:

- when the current endpoint is `2x2`, the proposal surface is fully collapsed
  to the blind one-step `3x3` neighbors.

### 5. Second family: `guided_permutation_3x3` `source -> target`

Command:

```sh
timeout -k 5s 12s target/dist/compare_graph_move_proposals \
  --fixture-ref research/fixtures/generic_guides.json#guided_permutation_3x3 \
  --seeded-guide-id two_hop_permutation \
  --current source \
  --target target \
  --max-dim 4 \
  --zigzag-bridge-entry 8 \
  --top-k 6 \
  --probe-lag 3 \
  --probe-shortlist-k 4
```

Result:

- blind successors: `16` raw / `16` unique, all `4x4`;
- targeted proposals: `384` raw / `16` unique, overlap with blind `16`;
- blind and proposal best gap are identical:
  `dim=1 row=1 col=10 entry_sum=3`, shortlist `1`;
- the bounded realization is `2` steps and `blind_overlap=true`.

Verdict:

- no cross-family shortlist edge appeared on the `3x3` generic control;
- raw proposal generation expanded by `24x` with no best-gap or shortlist gain.

### 6. Second family companion: `guided_permutation_4x4` `source -> target`

Result:

- blind successors: `40` raw / `40` unique, all `5x5`;
- targeted proposals: `4800` raw / `40` unique, overlap with blind `40`;
- blind and proposal best gap are identical:
  `dim=1 row=1 col=10 entry_sum=4`, shortlist `1`;
- the bounded realization is `2` steps and `blind_overlap=true`.

Verdict:

- the negative cross-family pattern persists at `4x4`;
- this is another case where proposal volume grows sharply but the best useful
  shortlist candidate is still just a blind successor.

## Partition-Refined Sidecar

The partition-refined quotient sidecar stayed analysis-only and did not change
the conclusion on any tested seam.

- every measured best coarse-gap bucket was already size `1`;
- the reported `refined_shortlist` therefore also stayed `1` on every case;
- on the anchor seam, refined ordering still says something real inside the
  broader proposal list, but it did not overturn or shrink the kept coarse
  shortlist.

## Conclusion

The shortlist signal does **not** currently generalize beyond the original
`guide:1 -> guide:15` seam.

- one Brix-Ruiz seam still shows a real non-blind win:
  `guide:1 -> guide:15`;
- the additional Brix-Ruiz waypoint and endpoint-derived controls collapse back
  to blind one-step behavior;
- the second-family controls also collapse back to blind one-step behavior.

## Recommendation

Recommendation: **one-off curiosity**.

The current shortlist seam is worth keeping as a documented example, but it is
not yet evidence for a stable measurement surface and it does not justify
another bounded rollout with the same scoring lane alone. If this topic is
reopened later, it should be because a new ranking or proposal family changes
the cross-seam picture, not because the current coarse shortlist already looks
generally predictive.
