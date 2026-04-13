# K=3 Default Beam Score Validation

## Default Score

Use `path_scoring::score_node(node, target)` as the default beam heuristic:

```text
12 * (row_type_count + col_type_count)
+ 6 * (row_support_type_count + col_support_type_count)
- 6 * (duplicate_row_pairs + duplicate_col_pairs)
+ 0.5 * same_future_past_signature_gap(node, target)
+ 0.25 * signature_distance(node, target)
```

Lower is better.

Rationale:

- keep the score structure-first, matching the signal-corpus note;
- reward same-future or same-past style duplicate structure directly;
- keep target-shape distance only as a light tiebreaker;
- avoid raw entry-sum as a primary term.

## Validation Commands

Replay on the known Brix-Ruiz `k=3` endpoint-16 path:

```bash
cargo run --features research-tools --bin replay_graph_path_scores
```

Observer-backed corpus probe:

```bash
cargo run --features research-tools --bin analyze_path_signal_corpus -- \
  --search-mode graph-only \
  --max-endpoint-dim 4 \
  --max-gap 2 \
  --max-cases 8
```

## Results

### Replay: blind endpoint 16-move path

- `beam_default_low`: `mean_pct=39.31%`, `top1=4/16`, `top10%=6/16`
- `types_plus_sig_low`: `mean_pct=41.79%`
- `endpoint_sig_low`: `mean_pct=57.14%`
- `entry_sum_low`: `mean_pct=53.14%`

Selected local-step ranks for `beam_default_low`:

- `step 2`: `4/51` (`7.84%`)
- `step 4`: `5/42` (`11.90%`)
- `step 6`: `5/60` (`8.33%`)
- `step 10`: `6/50` (`12.00%`)
- `step 12`: `4/79` (`5.06%`)
- `step 14`: `1/94` (`1.06%`)
- `step 15`: `1/35` (`2.86%`)

Interpretation:

- the default score cleanly beats naive endpoint distance and raw entry-size;
- it is slightly better than the older `types_plus_sig_low` composite on this replay;
- it still does not beat the very strong `dimension_low` baseline on this path.

### Observer-backed probe

For `sqlite:2:graph_path_result_2_ordinal_1 [6..8]`:

- `beam_default_low`: `rank 1/16` (`6.25%`)
- `endpoint_sig_low`: `37.50%`
- `entry_sum_low`: `25.00%`

This is only one ranked interior solution node, but it matches the same
direction as the replay result: the default structure-first score places the
solution node at the top of its observed layer, while naive distance and entry
size are materially worse.

## Caveat

The current default is a practical structure-first heuristic, not a proved lag
bound. On the known `k=3` replay, pure `dimension_low` remains stronger on
average, so beam-search experiments should still compare against width schedules
or hybrid ranking rules rather than assuming this score is final.

## Retune Follow-Up

Tried one focused hybrid comparison score without changing the beam executor:

```text
beam_dim_strict_low =
128 * abs(dim(node) - dim(endpoint))
+ 6 * (row_type_count + col_type_count)
+ 2 * (row_support_type_count + col_support_type_count)
- 2 * (duplicate_row_pairs + duplicate_col_pairs)
+ 0.25 * same_future_past_signature_gap(node, endpoint)
+ 0.75 * signature_distance(node, endpoint)
```

Replay result on the same blind endpoint 16-move path:

- `beam_default_low`: `mean_pct=39.31%`, `top1=4/16`, `top5%=2/16`, `top10%=6/16`
- `beam_dim_strict_low`: `mean_pct=39.48%`, `top1=4/16`, `top5%=3/16`, `top10%=6/16`

Notable replay detail:

- the hybrid helped some larger BFS next-frontier layers as a tiebreaker, for
  example `segment 0->8 step 3` improved from `1194/1497` (`79.76%`) to
  `919/1497` (`61.39%`);
- but it did not improve the main local-successor average, so `beam_default_low`
  remains the best executor-facing score in this pass.

Observer-backed probe on
`sqlite:2:graph_path_result_2_ordinal_1 [6..8]`:

- `beam_default_low`: `rank 1/16` (`6.25%`)
- `beam_dim_strict_low`: `rank 1/16` (`6.25%`)

Conclusion:

- keep `beam_default_low` as the default beam score for now;
- retain `beam_dim_strict_low` as a small comparison candidate for future replay
  or wider observer-backed probes.
