# Beam Score Check: Non-Brix Literature Campaign

## Goal

Revisit the existing beam-scoring comparisons using the newer non-Brix literature
material, without widening into a new beam-executor design.

The immediate gap after
[2026-04-13-k3-default-beam-score-validation.md](./2026-04-13-k3-default-beam-score-validation.md)
was that the score tooling mostly talked about Brix-Ruiz `k=3` full paths and
did not say much about the newer Riedel/Baker, Lind-Marcus, and higher-block
examples added to `research/cases.json`.

## Tooling change

`src/bin/analyze_path_signal_corpus.rs` now accepts bounded inline endpoint-case
inputs from the research harness corpus:

- `--cases PATH`
- `--case-id ID`
- `--campaign-id ID`

That lets the analyzer rerun the existing bounded search configs from
`research/cases.json` and rank any interior witness nodes that actually appear
inside observed frontier layers, instead of only replaying the older full-path
pool.

## Command

```bash
cargo run --quiet --features research-tools --bin analyze_path_signal_corpus -- \
  --cases research/cases.json \
  --campaign-id non_brix_ruiz_literature
```

## Result

Summary:

- `endpoint_cases=7`
- `solved_cases=7`
- `unranked_solved_cases=2`
- `ranked_solution_nodes=35/38`

Rank summary on the `35` rankable interior nodes:

- `dimension_low`: `mean_pct=2.88%`, `top1=35/35`, `worst_pct=10.00%`
- `duplicates_high`: `mean_pct=9.91%`, `top1=25/35`
- `row_col_types_low`: `mean_pct=9.91%`, `top1=25/35`
- `max_entry_low`: `mean_pct=19.01%`
- `beam_dim_strict_low`: `mean_pct=33.35%`, `top1=0/35`
- `beam_default_low`: `mean_pct=34.09%`, `top1=0/35`

Case coverage:

- `riedel_baker_k4`: `ranked=3/3`, `layers=4`
- `riedel_baker_k6`: `ranked=5/5`, `layers=6`
- `riedel_baker_k8`: `ranked=7/7`, `layers=8`
- `riedel_baker_k10`: `ranked=9/9`, `layers=10`
- `riedel_baker_k12`: `ranked=11/11`, `layers=12`
- `lind_marcus_a_to_c`: `ranked=0/1`, `layers=0`
- `full_2_shift_higher_block_1x1_to_4x4`: `ranked=0/2`, `layers=0`

## Reading

The non-Brix campaign does change the score picture:

- On the rankable Riedel/Baker family nodes, simple dimension pressure is far
  stronger than the current default structure-first blend.
- Duplicate-sensitive structure signals still look useful as tie-breakers.
- The current executor-facing defaults (`beam_default_low`,
  `beam_dim_strict_low`) do not win this campaign.

This does **not** yet justify changing `score_node()`:

- the April 13 Brix-Ruiz replay already showed `dimension_low` beating the
  default on average, but that was not enough on its own to justify switching
  the beam executor;
- the two non-Brix mixed-dimension cases that solved cleanly
  (`lind_marcus_a_to_c`, `full_2_shift_higher_block_1x1_to_4x4`) still did not
  yield rankable observer layers in this analyzer pass, so the expanded corpus
  is not yet uniformly beam-layer-observable;
- there is still no bounded beam-probe validation showing that a dimension-first
  score improves actual beam outcomes instead of only replay/layer ranking.

## Conclusion

This pass should land as an analysis/tooling slice, not as an executor-facing
beam-score retune.

The clearest next scoring step is now narrower:

1. add one explicit dimension-first comparison score in `src/path_scoring.rs`
   rather than another structure-heavy blend;
2. validate it on actual beam cases before changing `score_node()`;
3. separately investigate why the two mixed-dimension literature positives solve
   without producing rankable observer layers here.
