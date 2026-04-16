# k=3 guide-pool quotient shrinkage (2026-04-16)

## Question

If I take the current stored `k=3` full-path guides and quotient-normalize the
paths using the new triangle/square telemetry seam, how much of the guide pool
actually disappears before `shortcut_search` would consume it?

This stays bounded to guide-artifact preparation/reporting:

- no new default frontier moves;
- no shortcut-search ranking/admission changes;
- no claim that the local quotient should become search behavior.

## Normalization rule

I used the same research-only quotient as
`analyze_triangle_path_telemetry`, but applied it to whole stored guide paths
instead of only suffix windows.

Rule:

1. Canonicalize each matrix in each guide with `canonical_perm()`.
2. Mine the raw guide pool for lag-1 and lag-2 endpoint-preserving local
   alternatives.
3. Allow only these local rewrites on contiguous 3-matrix windows:
   - triangle collapse: `A -> B -> C` rewrites to `A -> C` when the direct
     lag-1 witness is already present in the corpus;
   - commuting-square normalization: between two distinct lag-2 windows with the
     same endpoints, pick the lexicographically smaller lag-2 representative.
4. Canonicalize each full path by bounded rewrite exploration and choose the
   representative with minimal `(lag, matrix-sequence)`.
5. Retain one representative per distinct canonical full path.

Configuration used for the durable numbers below:

- `max_suffix_lag=4`
- `max_rewrite_states=1024`

I also checked `max_rewrite_states=256` first. Raising the cap to `1024` did
not change the retained-guide count; it only shortened the two longest guides
by one extra step each. Both of those long guides still hit the cap, so the
reported lag for them is still a lower-bound under this bounded run rather than
an exhaustive minimum.

## Commands

Pool only:

```bash
timeout 180 cargo run --quiet --features research-tools --bin analyze_guide_pool_quotient -- \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --max-suffix-lag 4 --max-rewrite-states 1024 --max-samples 12 \
  --json-out research/runs/2026-04-16-k3-guide-pool-quotient.json
```

Current `k=3` artifacts on disk (pool + completed shortcut outputs):

```bash
timeout 180 cargo run --quiet --features research-tools --bin analyze_guide_pool_quotient -- \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --guide-artifacts research/guide_artifacts/k3_shortcut_round1.json \
  --guide-artifacts research/guide_artifacts/k3_shortcut_round2.json \
  --max-suffix-lag 4 --max-rewrite-states 1024 --max-samples 12 \
  --json-out research/runs/2026-04-16-k3-guide-artifacts-quotient.json
```

## Results

### Normalized guide pool only

Artifact:

- `research/runs/2026-04-16-k3-guide-pool-quotient.json`

Raw pool:

- `12` stored guides
- total lag `155`
- total matrices `167`
- unique suffix windows `387`
- duplicate suffix-window occurrences `161`

Quotient-retained pool:

- `5` retained representatives (`58.3%` guide-count reduction)
- total retained lag `76` (`51.0%` reduction)
- total retained matrices `81` (`51.5%` reduction)
- unique suffix windows `263` (`32.0%` reduction)
- duplicate suffix-window occurrences `11` (`93.2%` reduction)

Guide-level effects:

- `10 / 12` guides changed under the quotient
- `10 / 12` guides lost lag
- only `2` guides were unchanged: the Lind-Marcus/Baker lag-7 witness and one
  already-minimal lag-7 sqlite shortcut witness
- `9 / 12` raw guides collapsed into just two lag-7 canonical representatives
- the remaining two retained guides are still long (`lag 27` and `lag 28`) and
  were the only paths that hit the rewrite-state cap

Representative retained classes:

- `k3-sqlite-shortcut-2`, absorbing
  `k3-sqlite-shortcut-2`, `k3-sqlite-shortcut-4`, `k3-sqlite-shortcut-7`
- `k3-sqlite-shortcut-1`, absorbing
  `k3-sqlite-shortcut-1`, `k3-sqlite-shortcut-6`, `k3-sqlite-shortcut-8`,
  `k3-sqlite-shortcut-9`, `k3-sqlite-shortcut-11`, `k3-sqlite-shortcut-12`

Representative collapses:

- `k3-sqlite-shortcut-4`: lag `11 -> 7` via triangle plus commuting-square
  normalization
- `k3-sqlite-shortcut-8`: lag `11 -> 7` via triangle
- `k3-sqlite-shortcut-9`: lag `8 -> 7` via triangle

### Current k=3 artifacts on disk

Artifact:

- `research/runs/2026-04-16-k3-guide-artifacts-quotient.json`

Inputs:

- the `12`-guide normalized pool
- `k3_shortcut_round1.json`
- `k3_shortcut_round2.json`

Outcome:

- `14` source guide artifacts still quotient down to the same `5`
  representatives
- total retained lag stays `76`
- total retained matrices stay `81`
- unique suffix windows stay `263`
- duplicate suffix-window occurrences fall from `205` to `11` (`94.6%`
  reduction)

Interpretation:

- the two completed shortcut outputs add no new quotient class;
- both round outputs land in the same quotient class as the existing
  Lind-Marcus/Baker lag-7 witness.

## Conclusion

The quotient is materially shrinking the current `k=3` guide pool.

On the guide pool that current shortcut runs actually consume,
quotient-normalizing stored full-path guides collapses `12` raw guides to `5`
retained representatives and removes almost all repeated local suffix structure
(`161 -> 11` duplicate suffix-window occurrences). The effect is not just on
window telemetry: most of the stored full guides themselves fold into a small
number of lag-7 representatives.

The next sensible follow-up is still narrow:

- materialize a research-only quotient-retained `k=3` pool and A/B it against
  the current `k3_normalized_guide_pool.json` as an input-preparation change;
- keep that experiment out of default search behavior until it shows an actual
  shortcut-search runtime or admission-quality benefit.
