# Riedel/Baker endpoint-ceiling control-map refresh (2026-04-27)

## Scope

This refresh reruns the retained 2026-04-19 Riedel/Baker endpoint-ceiling
graph-only control map on current `main` and compares it to the committed
2026-04-19 run artifact.

This is a measurement/control-lane refresh only:

- no solver or search behavior was changed;
- the retained corpus was not promoted into the default canonical corpus;
- this is not a Brix-Ruiz Goal 3 result; and
- the result is judged as a Riedel/Baker graph-only tooling control surface.

The worktree tip for the run was `5fa91b2` (`main`, `origin/main`), commit
message `Extract endpoint search tests`.

## Inputs and retained output

Input corpus:

- [`research/riedel_graph_only_endpoint_ceiling_map_2026-04-19.json`](../riedel_graph_only_endpoint_ceiling_map_2026-04-19.json)

Baseline comparator:

- [`research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json`](../riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json)

New retained current-main run, with the 2026-04-19 artifact loaded as history
and elapsed-only best-known improvement flags normalized away:

- [`research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json`](../riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json)

Scratch comparison output, not retained:

- `tmp/3r3-1-riedel-endpoint-map-comparison.tsv`

The retained corpus remains the 12-case non-gating measurement lane from
2026-04-19: `k = 4, 6, 8, 10, 12, 14`, both `graph_only` and
`graph_plus_structured`, with `max_intermediate_dim = 3`, `max_entry = k + 1`,
`max_lag = k + 2`, and three measurement repeats per case.

## Commands

Context and claim:

```bash
bd show sse-rust-3r3.1 --json
bd show sse-rust-3r3 --json
bd update sse-rust-3r3.1 --claim --notes "Claimed for current-main Riedel/Baker endpoint-ceiling control-map refresh. Read prior 2026-04-19 corpus, run artifact, benchmark policy, and program guidance; proceeding with full retained corpus unless runtime proves unexpectedly high." --json
git log --oneline --decorate -n 8 --all
git merge-base --is-ancestor main HEAD && printf 'main_is_ancestor_of_head\n' || printf 'main_not_ancestor_of_head\n'
```

Build and run:

```bash
timeout -k 10s 300s cargo build --profile dist --features research-tools --bin research_harness
timeout -k 10s 300s cargo run --profile dist --features research-tools --bin research_harness -- --cases research/riedel_graph_only_endpoint_ceiling_map_2026-04-19.json --reuse-run research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json --format json > research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json
```

Best-known metadata normalization:

```bash
jq --slurpfile base research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json '
  def endpoint_key($e): [$e.source_dim, $e.target_dim, $e.a, $e.b] | tojson;
  ($base[0].cases
    | map(select(.result_model.witness_lag != null)
      | {key: endpoint_key(.endpoint), value: {lag: .result_model.witness_lag, elapsed_ms: .elapsed_ms, source: "research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json"}})
    | group_by(.key)
    | map({key: .[0].key, value: (map(.value) | sort_by(.lag, .elapsed_ms, .source) | .[0])})
    | from_entries) as $hist |
  def with_historical_best($endpoint):
    if $hist[endpoint_key($endpoint)] then
      .best_known_witness = $hist[endpoint_key($endpoint)]
      | .improved_best_known_witness = false
    else
      .
    end;
  .cases |= map(with_historical_best(.endpoint))
  | (.cases | map({key: .id, value: {best_known_witness: .best_known_witness, improved_best_known_witness: .improved_best_known_witness}}) | from_entries) as $case_best
  | .comparisons |= map(. as $comparison | .variants |= map(with_historical_best($comparison.endpoint)))
  | .campaigns |= map(.scheduled_cases |= map(.best_known_witness = $case_best[.case_id].best_known_witness | .improved_best_known_witness = false))
  | .strategies |= map(.best_known_improvements = 0)
  | .fitness.best_known_improvements = 0
' research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json > tmp/3r3-1-riedel-endpoint-map-run-2026-04-27-normalized.json
mv tmp/3r3-1-riedel-endpoint-map-run-2026-04-27-normalized.json research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json
```

Per-case comparison command:

```bash
jq -n -r --slurpfile base research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json --slurpfile curr research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json '
  def row($c): {
    outcome: $c.actual_outcome,
    lag: $c.result_model.witness_lag,
    elapsed: $c.elapsed_ms,
    median: ($c.measurement.median_elapsed_ms // $c.elapsed_ms),
    generated: $c.telemetry.candidates_generated,
    visited: $c.telemetry.total_visited_nodes,
    expanded: $c.telemetry.frontier_nodes_expanded
  };
  ($base[0].cases | map({key:.id, value:row(.)}) | from_entries) as $b |
  ($curr[0].cases | map({key:.id, value:row(.)}) | from_entries) as $c |
  (["case","base_outcome","curr_outcome","base_lag","curr_lag","base_elapsed","curr_elapsed","delta_elapsed","base_generated","curr_generated","delta_generated","base_visited","curr_visited","delta_visited","base_expanded","curr_expanded","delta_expanded"] | @tsv),
  ($b | keys[] as $id |
    [$id, $b[$id].outcome, $c[$id].outcome, ($b[$id].lag // ""), ($c[$id].lag // ""), $b[$id].elapsed, $c[$id].elapsed, ($c[$id].elapsed - $b[$id].elapsed), $b[$id].generated, $c[$id].generated, ($c[$id].generated - $b[$id].generated), $b[$id].visited, $c[$id].visited, ($c[$id].visited - $b[$id].visited), $b[$id].expanded, $c[$id].expanded, ($c[$id].expanded - $b[$id].expanded)] | @tsv)' > tmp/3r3-1-riedel-endpoint-map-comparison.tsv
```

Total-by-strategy comparison command:

```bash
jq -n -r --slurpfile base research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-19.json --slurpfile curr research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json '
  def totals($a): $a.cases | group_by(.campaign.strategy) | map({key:.[0].campaign.strategy, value:{elapsed:(map(.elapsed_ms)|add), generated:(map(.telemetry.candidates_generated)|add), visited:(map(.telemetry.total_visited_nodes)|add), expanded:(map(.telemetry.frontier_nodes_expanded)|add)}}) | from_entries;
  (totals($base[0])) as $b | (totals($curr[0])) as $c |
  (["strategy","base_elapsed","curr_elapsed","delta_elapsed","base_generated","curr_generated","delta_generated","base_visited","curr_visited","delta_visited","base_expanded","curr_expanded","delta_expanded"] | @tsv),
  ($b | keys[] as $s | [$s, $b[$s].elapsed, $c[$s].elapsed, ($c[$s].elapsed - $b[$s].elapsed), $b[$s].generated, $c[$s].generated, ($c[$s].generated - $b[$s].generated), $b[$s].visited, $c[$s].visited, ($c[$s].visited - $b[$s].visited), $b[$s].expanded, $c[$s].expanded, ($c[$s].expanded - $b[$s].expanded)] | @tsv)'
```

## Outcome and witness comparison

Outcomes and witness lags are unchanged from 2026-04-19.

| Rung | `graph_only` 2026-04-19 -> 2026-04-27 | `graph_plus_structured` 2026-04-19 -> 2026-04-27 |
| --- | --- | --- |
| `k = 4` | `equivalent`, lag `6` -> `equivalent`, lag `6` | `equivalent`, lag `5` -> `equivalent`, lag `5` |
| `k = 6` | `unknown` -> `unknown` | `equivalent`, lag `7` -> `equivalent`, lag `7` |
| `k = 8` | `unknown` -> `unknown` | `equivalent`, lag `9` -> `equivalent`, lag `9` |
| `k = 10` | `unknown` -> `unknown` | `equivalent`, lag `11` -> `equivalent`, lag `11` |
| `k = 12` | `unknown` -> `unknown` | `equivalent`, lag `13` -> `equivalent`, lag `13` |
| `k = 14` | `unknown` -> `unknown` | `equivalent`, lag `15` -> `equivalent`, lag `15` |

The recovery map therefore remains:

- `graph_only` recovers only the retained `k = 4` rung, at witness lag `6`;
- `graph_only` still does not recover `k = 6, 8, 10, 12, 14` on this surface;
- `graph_plus_structured` still solves all six retained rungs; and
- `graph_plus_structured` still gives the better lag on the shared `k = 4`
  endpoint pair (`5` vs `6`).

The retained 2026-04-27 artifact records `reused_history_sources = 1` and
`best_known_improvements = 0`. The history-aware harness run initially marked
some equal-lag `graph_plus_structured` cases as best-known improvements because
their median elapsed samples were lower than the 2026-04-19 samples. For this
control-lane refresh, those elapsed-only changes are treated as timing noise,
so the retained artifact pins best-known witness metadata back to the 2026-04-19
history source while preserving the current run's outcome, lag, elapsed, and
telemetry fields.

## Timing and search-volume comparison

Elapsed values below are the representative median `elapsed_ms` values reported
by the harness measurement block.

| Rung | `graph_only` elapsed delta | `graph_plus_structured` elapsed delta |
| --- | ---: | ---: |
| `k = 4` | `6 -> 7 ms` (`+1`) | `13 -> 12 ms` (`-1`) |
| `k = 6` | `9 -> 9 ms` (`+0`) | `107 -> 104 ms` (`-3`) |
| `k = 8` | `22 -> 27 ms` (`+5`) | `487 -> 487 ms` (`+0`) |
| `k = 10` | `51 -> 55 ms` (`+4`) | `2005 -> 1926 ms` (`-79`) |
| `k = 12` | `113 -> 113 ms` (`+0`) | `6736 -> 6459 ms` (`-277`) |
| `k = 14` | `222 -> 223 ms` (`+1`) | `17744 -> 17488 ms` (`-256`) |

Total elapsed by strategy:

| Strategy | 2026-04-19 | 2026-04-27 | Delta |
| --- | ---: | ---: | ---: |
| `graph_only` | `423 ms` | `434 ms` | `+11 ms` |
| `graph_plus_structured` | `27092 ms` | `26476 ms` | `-616 ms` |
| full campaign | `27515 ms` | `26910 ms` | `-605 ms` |

The search-volume counters are unchanged for every case:

| Strategy | generated | visited | expanded |
| --- | ---: | ---: | ---: |
| `graph_only` | `70224 -> 70224` | `4411 -> 4411` | `4394 -> 4394` |
| `graph_plus_structured` | `1020439 -> 1020439` | `342533 -> 342533` | `341441 -> 341441` |

Because every generated/visited/expanded counter is identical and all outcome
and lag fields are identical, the elapsed changes are ordinary run-to-run timing
noise rather than a search-shape movement.

## Keep / Reject

Keep:

- the dim-3 endpoint-ceiling Riedel/Baker map as a stable graph-only
  control-lane measurement surface;
- the new 2026-04-27 JSON artifact as a useful future comparator; and
- the conclusion that target-ceiling entry widening recovers only the retained
  `k = 4` graph-only rung on this surface.

Reject:

- any interpretation as progress on Brix-Ruiz Goal 3;
- any solver/search behavior change based on this refresh alone; and
- promoting heavier variants into the default canonical corpus from this data.

## Conclusion

The 2026-04-27 current-main refresh reproduces the 2026-04-19 control-lane
result exactly on outcomes, witness lags, and search-volume counters. The
control lane remains useful and stable: keep it as a retained non-gating
Riedel/Baker graph-only measurement surface, with `graph_plus_structured` as
the direct solving-policy comparator.
