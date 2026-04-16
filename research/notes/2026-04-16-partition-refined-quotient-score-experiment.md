# Partition-Refined Quotient Score Experiment

## Question

For bead `sse-rust-ee8`, can one analysis-only partition-refined extension of the
current same-future/same-past quotient signature add useful evidence without
touching `score_node()` or default beam behavior?

## Slice Landed

- `src/graph_moves.rs`
  added a scalar partition-refined quotient gap that starts from the existing
  duplicate row/column classes, then records the quotient-style block profile of
  each class against the opposite-side partition using
  `(opposite multiplicity, opposite entry_sum, opposite support, block value)`.
- `src/path_scoring.rs`
  exported the new analyzer-only score as `partition_refined_quotient_low` in
  `candidate_score_specs()`.
- `src/bin/compare_graph_move_proposals.rs`
  now reports the refined gap alongside the existing coarse
  same-future/same-past gap for blind successors, proposal shortlists, and
  bounded realization probes.

This round intentionally left `score_node()` and all default frontier ranking
unchanged.

## Evidence

### 1. Observed-layer signal corpus stays sparse but is directionally positive

Command:

```bash
cargo run --quiet --features research-tools --bin analyze_path_signal_corpus -- \
  --search-mode graph-only \
  --max-endpoint-dim 4 \
  --max-gap 2 \
  --max-cases 8
```

Result:

- the bounded run again produced only one rankable observed-layer solution node;
- `partition_refined_quotient_low` ranked that node `1/16` (`6.25%`);
- that matched the current best structural family
  (`beam_default_low`, `row_col_types_low`, `support_types_low`,
  `types_plus_sig_low`, `duplicates_high`);
- it still beat the endpoint-distance style controls
  (`endpoint_sig_low` `37.50%`, `entry_plus_sig_low` `43.75%`,
  `entry_sum_low` `25.00%`, `max_entry_low` `56.25%`).

Interpretation:

- the observer-backed corpus is still too small to prove much;
- on the only nontrivial layer it behaves like the good structure-first signals,
  not like the weaker endpoint-distance scores.

### 2. Replay analysis says the score is plausible, but not a new leader

Command:

```bash
cargo run --quiet --features research-tools --bin replay_graph_path_scores
```

Local-successor summary on the blind endpoint 16-move path:

- `partition_refined_quotient_low`: mean percentile `38.43%`
- `endpoint_sig_low`: `57.14%`
- `entry_plus_sig_low`: `57.77%`
- `entry_sum_low`: `53.14%`
- `beam_default_low`: `39.31%`
- `beam_dim_strict_low`: `39.48%`
- `types_plus_sig_low`: `41.79%`
- stronger coarse structural controls still led:
  `row_col_types_low` `24.97%`,
  `support_types_low` `31.19%`,
  `duplicates_high` `33.22%`,
  `dimension_low` `19.84%`.

Interpretation:

- this refined quotient score is clearly better than the endpoint-distance lane
  on the replay path;
- it is not clearly better than the current strongest coarse structural signals,
  so this round does **not** justify promoting it into default beam weights.

### 3. Proposal comparison: useful sidecar, no current tie-bucket win

Source -> target command:

```bash
cargo run --quiet --features research-tools --bin compare_graph_move_proposals -- \
  --current source --target target --top-k 4
```

Result:

- blind and proposal surfaces still had the same best coarse gap;
- both already had best-gap shortlist size `1`;
- the refined gap therefore added **no** extra shortlist shrinkage here.

Same-dimension waypoint command:

```bash
cargo run --quiet --features research-tools --bin compare_graph_move_proposals -- \
  --current guide:1 --target guide:15 --top-k 6 --probe-lag 3
```

Result:

- best blind coarse-gap candidate: refined gap `137`;
- best proposal coarse-gap candidate: refined gap `42`;
- that best proposal remained the same realized `3x3` zig-zag candidate from the
  earlier slice and was still reachable in `3` graph-only steps;
- however the best coarse proposal shortlist was already size `1`, so the
  refined signal still did **not** break a real same-gap tie on this case.

One notable detail:

- the second listed proposal had a worse coarse gap
  (`dim=0 row=3 col=4 entry_sum=1`) but an even lower refined gap (`38`) than
  the coarse-best proposal (`42`).

Interpretation:

- the refined score is measuring something real inside the same proposal lane;
- on current evidence it looks most appropriate as a future tie-break or
  alternate shortlist experiment, not as a replacement for the existing coarse
  gap.

## Conclusion

This experiment is a **keep as analysis-only sidecar**, not a promotion round.

- Positive:
  the partition-refined quotient signal is better than raw endpoint-distance
  baselines and stays aligned with the current structure-first proposal lane.
- Negative:
  it did not beat the best coarse structural controls on replay, and it did not
  yet collapse any real best-gap tie bucket on the bounded proposal cases.

The next justified use would be a bounded tie-break or alternate-shortlist probe
inside the existing proposal-analysis seam, still without changing default beam
ranking unless it wins more clearly.
