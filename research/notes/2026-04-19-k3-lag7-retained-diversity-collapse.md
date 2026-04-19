# `k = 3` lag-7 diversity inventory and retained-only exact replay collapse (2026-04-19)

## Question

On the hard Brix-Ruiz `k = 3` lane, how much explicit lag-7 witness diversity do we
currently have, and does a retained-only exact-endpoint replay preserve any
non-Baker lag-7 family after reanchor/promotion?

This slice stays bounded to:

- explicit lag-7 witness/path-shape inventory;
- one retained-only exact-endpoint control;
- no solver rewrite and no broad parameter sweep.

## Inventory

### Exact hard-endpoint lag-7 witnesses currently committed

Exact endpoints:

- `A = [[1,3],[2,1]]`
- `B = [[1,6],[1,1]]`

Committed explicit lag-7 artifacts on these exact endpoints are all the same
family:

- `research/guide_artifacts/k3_normalized_guide_pool.json#k3-lind-marcus-baker-lag7`
- `research/guide_artifacts/k3_shortcut_round1.json`
- `research/guide_artifacts/k3_shortcut_round2.json`

Their shared path signature is:

```text
2x2:1,3,2,1
-> 3x3:1,2,2,2,1,1,1,0,0
-> 4x4:1,2,2,0,1,0,2,0,0,1,1,1,1,1,2,0
-> 4x4:1,2,1,1,1,0,1,0,1,1,0,1,2,0,0,1
-> 4x4:1,2,2,0,1,1,1,1,0,1,0,1,0,2,1,0
-> 4x4:1,1,1,1,3,0,2,2,1,0,0,0,0,1,1,1
-> 3x3:1,1,1,5,0,5,1,0,1
-> 2x2:1,6,1,1
```

I validated that `k3_shortcut_round1.json` and `k3_shortcut_round2.json`
match this exact matrix sequence.

### Lag-7 classes present in the normalized guide pool after quotient analysis

Running the existing quotient tool on
`research/guide_artifacts/k3_normalized_guide_pool.json` finds **three**
retained lag-7 classes in the broader hard-lane guide surface:

1. Exact Baker class
   Source labels: `k3-lind-marcus-baker-lag7`

```text
2x2:1,3,2,1
-> 3x3:1,2,2,2,1,1,1,0,0
-> 4x4:1,2,2,0,1,0,2,0,0,1,1,1,1,1,2,0
-> 4x4:1,2,1,1,1,0,1,0,1,1,0,1,2,0,0,1
-> 4x4:1,2,2,0,1,1,1,1,0,1,0,1,0,2,1,0
-> 4x4:1,1,1,1,3,0,2,2,1,0,0,0,0,1,1,1
-> 3x3:1,1,1,5,0,5,1,0,1
-> 2x2:1,6,1,1
```

2. Search-pool class A
   Retained representative: `k3-sqlite-shortcut-2`
   Merged source labels: `k3-sqlite-shortcut-2`, `k3-sqlite-shortcut-4`,
   `k3-sqlite-shortcut-7`

```text
2x2:1,2,3,1
-> 3x3:0,0,1,1,1,2,2,2,1
-> 4x4:0,0,1,2,1,0,1,2,2,0,1,2,1,1,0,1
-> 4x4:0,0,1,1,0,1,0,2,1,1,0,1,2,1,1,1
-> 4x4:0,0,1,1,2,1,0,2,1,0,0,2,1,1,1,1
-> 3x3:0,2,3,1,1,1,1,1,1
-> 2x2:0,5,1,2
-> 2x2:1,1,6,1
```

3. Search-pool class B
   Hidden explicit lag-7 class inside longer stored guides.
   Retained label in quotient analysis: `k3-sqlite-shortcut-1`
   Merged source labels: `k3-sqlite-shortcut-1`, `k3-sqlite-shortcut-11`,
   `k3-sqlite-shortcut-12`, `k3-sqlite-shortcut-6`, `k3-sqlite-shortcut-8`,
   `k3-sqlite-shortcut-9`

```text
2x2:1,2,3,1
-> 3x3:0,1,2,0,1,2,1,2,1
-> 4x4:0,0,0,1,1,0,2,1,1,1,1,0,2,2,2,1
-> 4x4:0,0,1,1,1,1,0,1,1,0,0,2,1,2,1,1
-> 4x4:0,0,1,1,1,0,1,1,1,1,2,0,2,1,2,0
-> 3x3:0,0,1,1,2,2,1,2,0
-> 2x2:0,1,5,2
-> 2x2:1,1,6,1
```

Interpretation of the inventory:

- there is only **one explicit lag-7 family on the exact hard endpoints**;
- there are **two additional lag-7 classes** on the broader hard-lane surface,
  but both live on the canonicalized search-witness endpoint pair
  `[[1,2],[3,1]] -> [[1,1],[6,1]]`;
- one of those two extra classes is not explicitly stored at lag 7 in the
  retained artifact pool at all; it only appears after quotient canonicalization
  of longer stored guides.

## One bounded diversity probe

Probe goal:

- use the quotient-retained guide pool as the smallest existing multi-family
  seed set;
- replay it on the exact hard endpoints;
- check whether exact reanchor + shortcut promotion preserves any non-Baker
  lag-7 witness.

Command:

```sh
timeout -k 10s 240s target/release/search 1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_quotient_retained_guide_pool.json \
  --max-intermediate-dim 5 --max-entry 6 \
  --guided-max-shortcut-lag 5 --guided-min-gap 2 --guided-max-gap 6 \
  --guided-segment-timeout 5 --guided-rounds 2 \
  --shortcut-max-guides 5 --shortcut-rounds 2 \
  --shortcut-max-total-segment-attempts 64 \
  --json --telemetry \
  --write-guide-artifact tmp/sse-rust-28u_retained_pool_exact_diversity_probe_2026-04-19.guide.json \
  > tmp/sse-rust-28u_retained_pool_exact_diversity_probe_2026-04-19.result.json
```

Observed telemetry:

- outcome: `equivalent`
- lag: `7`
- guide artifacts considered / accepted: `5 / 5`
- shortcut guides loaded / accepted / unique: `5 / 5 / 5`
- best lag start / end: `7 / 7`
- segment attempts: `64`
- stop reason: `max_segment_attempts_reached`

Observed witness:

- the emitted guide from
  `tmp/sse-rust-28u_retained_pool_exact_diversity_probe_2026-04-19.guide.json`
  is byte-identical to `research/guide_artifacts/k3_shortcut_round1.json`
  (`cmp -s` returned `0`);
- therefore the retained-only exact replay again emitted the same Baker-family
  lag-7 witness shown above, not a new family.

## Conclusion

This slice does **not** add a new explicit exact-endpoint lag-7 witness.

It does leave durable evidence for why current diversity surfaces keep
collapsing:

- the exact hard endpoints currently have only one explicit lag-7 family in
  committed artifacts;
- the broader guide surface does contain two additional lag-7 classes, but they
  are carried by the canonicalized search-witness endpoint pair rather than the
  exact hard endpoints;
- one of those extra classes is only visible after quotient canonicalization of
  longer guides, so it is not even an explicit lag-7 `full_path` artifact yet;
- when the retained multi-family pool is replayed on the exact endpoints, all
  five retained guides survive acceptance and dedup, but the emitted lag-7
  witness still collapses to the exact Baker-family path.

Current read: the bottleneck is not simply guide-count dedup. The missing piece
is a way to turn the non-Baker search-pool classes, especially the quotient-only
class B, into explicit exact-endpoint witnesses rather than letting exact replay
fall back to the existing Baker family.
