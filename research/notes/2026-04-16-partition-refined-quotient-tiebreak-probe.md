# Partition-Refined Quotient Tie-Break Probe

## Question

For bead `sse-rust-cym`, does the partition-refined quotient gap earn a
stronger proposal-analysis role if it is used only inside a bounded alternate
shortlist order, without touching default solver or beam ranking?

## Slice Landed

- `src/graph_moves.rs`
  now exposes `GraphProposals::refined_shortlist_from_coarse_prefix(...)`,
  which keeps the existing coarse proposal order for a bounded prefix, then
  reorders only that prefix by the partition-refined quotient gap.
- `src/search.rs`
  extends `GraphProposalProbeConfig` with an opt-in shortlist mode so the
  existing proposal probe can evaluate either the original best-gap bucket or
  the new refined alternate order.
- `src/bin/compare_graph_move_proposals.rs`
  adds `--probe-refined-prefix N`, which runs a second bounded probe using
  refined-gap order inside the top `N` coarse-ranked proposals.

This slice leaves `score_node()`, default beam ordering, and normal search
expansion unchanged.

## Evidence

### 1. Endpoint source -> target still does not justify a promotion

Command:

```sh
cargo run --quiet --features research-tools --bin compare_graph_move_proposals -- \
  --current source --target target --top-k 8 \
  --probe-lag 1 --probe-shortlist-k 4 --probe-refined-prefix 4
```

Result:

- the original best-gap shortlist is still size `1`;
- the refined alternate probe reorders the top coarse prefix to start with
  `dim=1 row=7 col=14 entry_sum=1 refined_gap=112`, ahead of the coarse-best
  `dim=1 row=5 col=17 entry_sum=2 refined_gap=116`;
- all four alternate-shortlist proposals are blind-overlap `3x3` successors and
  all are realized by the tiny bounded graph-only probe.

Interpretation:

- the refined signal is active inside the shortlist;
- on the endpoint source, it prefers a structurally nearer-looking proposal that
  is still worse under the existing coarse target gap;
- this is not evidence that refined order improves endpoint proposal quality.

### 2. Same-dimension waypoint: refined order finds easier nearby proposals, not a better target

Command:

```sh
cargo run --quiet --features research-tools --bin compare_graph_move_proposals -- \
  --current guide:1 --target guide:15 --top-k 8 \
  --probe-lag 3 --probe-shortlist-k 4 --probe-refined-prefix 4
```

Result:

- the original best-gap shortlist is still size `1`, headed by the earlier
  `3x3` zig-zag waypoint with coarse gap
  `dim=0 row=2 col=6 entry_sum=0` and refined gap `42`;
- the refined alternate probe reorders the top coarse prefix to place
  `dim=0 row=3 col=4 entry_sum=1 refined_gap=38` first;
- all four alternate-shortlist `3x3` zig-zag proposals are realizable within
  the same bounded lag-`3` graph-only probe;
- the refined-first proposal uses fewer visited states than the coarse-best
  candidate (`30` vs `44`), but it is still worse on the primary coarse target
  gap.

Interpretation:

- the refined quotient gap is measuring something real in this waypoint seam;
- the signal can surface same-dimension proposals that are slightly easier to
  realize under the tiny bounded probe;
- it still does not beat the existing coarse structural target score, so it is
  not yet a better primary shortlist signal.

## Conclusion

This round is another **keep as sidecar-only analysis probe**, not a promotion.

- Positive:
  the refined quotient gap now has one justified bounded use: an alternate
  shortlist order inside proposal analysis.
- Negative:
  on both required bounded cases, the refined-first candidate still loses to the
  coarse-best candidate on the primary target gap.

Decision:

- keep the new alternate-shortlist probe available for future proposal-analysis
  rounds;
- keep the refined quotient signal out of default beam/search ranking;
- do not replace the existing coarse best-gap shortlist unless a later round
  shows the refined-first candidate also wins on bounded target quality rather
  than only on realizability effort.
