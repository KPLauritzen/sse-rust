# Concrete-shift profile beam prototype (2026-04-25)

## Question

Can cheap concrete-shift-style profile data improve exploration across layer
boundaries or beam admission on the hard Brix-Ruiz lanes, rather than merely
reordering successors inside one BFS layer?

This was bead `sse-rust-o6o`.

## Implemented Signal

Added an opt-in frontier mode:

```text
concrete_shift_profile_beam
```

It uses the existing beam executor, but changes beam candidate scoring. For
`2x2` candidate/target pairs, the scorer computes a bounded low-lag aligned
concrete-shift profile:

- `max_lag = 1`
- `max_entry = 1`
- `max_witnesses = 32`
- relation: `aligned`

The profile records:

- whether a bounded concrete-shift witness was found;
- whether only bounded shift-equivalence witnesses were seen;
- whether the bounded concrete witness search hit its local limit; and
- how many bounded shift-equivalence witnesses were seen.

This is heuristic ranking side data only. It does not prune, does not certify
absence of witnesses, and does not conflate concrete shift with balanced
elementary equivalence.

## Commands

Build:

```bash
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
```

Single A/B corpus:

```bash
jq -n --slurpfile base research/cases.json \
  --slurpfile k4 research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  '{schema_version:5, cases: [
    ($base[0].cases[] | select(.id == "brix_ruiz_k3_graph_plus_structured_beam_probe")
      | .id="brix_ruiz_k3_graph_plus_structured_beam_probe__baseline"
      | del(.measurement) | .timeout_ms=4000),
    ($base[0].cases[] | select(.id == "brix_ruiz_k3_graph_plus_structured_beam_probe")
      | .id="brix_ruiz_k3_graph_plus_structured_beam_probe__concrete_shift_profile"
      | .description="A/B profile-beam variant for sse-rust-o6o."
      | del(.measurement) | .timeout_ms=4000
      | .config.frontier_mode="concrete_shift_profile_beam"),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__baseline"
      | .timeout_ms=30000),
    ($k4[0].cases[] | select(.id == "brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12")
      | .id="brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12__concrete_shift_profile"
      | .description="A/B profile-beam variant for sse-rust-o6o."
      | .timeout_ms=30000
      | .config.frontier_mode="concrete_shift_profile_beam")
  ]}' > tmp/o6o_concrete_shift_profile_ab_cases.json
```

Run:

```bash
timeout -k 20s 100s target/debug/research_harness \
  --cases tmp/o6o_concrete_shift_profile_ab_cases.json \
  --format json \
  > tmp/o6o_concrete_shift_profile_ab_results.json
```

Metric extraction:

```bash
jq -r '.cases[] |
  [.id,.actual_outcome,(.steps//""),.elapsed_ms,(.telemetry.layers|length),
   .telemetry.collisions_with_other_frontier,
   .telemetry.approximate_other_side_hits,
   .telemetry.max_frontier_size,
   .telemetry.total_visited_nodes,
   .telemetry.frontier_nodes_expanded,
   .telemetry.factorisations_enumerated,
   .telemetry.candidates_after_pruning,
   .telemetry.discovered_nodes,
   .telemetry.enqueued_nodes] | @tsv' \
  tmp/o6o_concrete_shift_profile_ab_results.json
```

## A/B Metrics

`k = 3` control:

| Case | Outcome | Witness lag | Elapsed ms | Exact meets | Approx hits | Max frontier | Total visited | Expanded | Factorisations | Kept candidates | Discovered |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline `beam` | `unknown` | none | `491` | `0` | `10` | `10` | `2631` | `142` | `21653` | `3501` | `2629` |
| `concrete_shift_profile_beam` | `unknown` | none | `880` | `0` | `5` | `10` | `2756` | `142` | `26500` | `3506` | `2754` |

Retained/open `k = 4` graph-plus-structured lane:

| Case | Outcome | Witness lag | Elapsed ms | Exact meets | Approx hits | Max frontier | Total visited | Expanded | Factorisations | Kept candidates | Discovered |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline `beam` | `unknown` | none | `23501` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` |
| `concrete_shift_profile_beam` | `unknown` | none | `23321` | `0` | `178` | `256` | `177205` | `19970` | `485429` | `270921` | `177203` |

## Decision

Reject this signal for promotion. It changed cross-layer beam admission, but it
did not improve outcome, exact meets, approximate hits, or visited count on the
bounded controls. The `k = 3` control got worse on approximate hits and elapsed
time. The retained `k = 4` lane was effectively neutral on runtime, slightly
lower on approximate hits, and slightly higher on total visited nodes.

Keep the opt-in scaffold as a reproducible experiment surface because it is
isolated behind `concrete_shift_profile_beam` and may still be useful for later
profile variants. Do not make it default.

## Follow-up

No new follow-up bead is justified from this slice. The existing
`sse-rust-srl` bead remains the right place for a richer concrete-shift profile:
this prototype only used a very small bounded result-class signal, not cached
fiber residuals or relation mismatch counts.

## Validation

Focused tests:

```bash
timeout -k 20s 180s cargo test -q concrete_shift_profile --features research-tools
timeout -k 20s 180s cargo test -q beam --features research-tools
timeout -k 20s 180s cargo test --features research-tools frontier_mode
```

Required gates:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
timeout -k 20s 600s cargo bench --bench search -- --noplot
```

Results:

- `cargo fmt --all`: passed.
- `cargo test -q concrete_shift_profile --features research-tools`: passed, `6`
  filtered tests.
- `cargo test -q beam --features research-tools`: passed, including `22`
  library beam/search tests and `8` search CLI tests.
- `cargo test --features research-tools frontier_mode`: passed.
- `cargo build --features research-tools --bin research_harness`: passed.
- `cargo bench --bench search -- --noplot`: passed. Criterion samples:
  `endpoint_equivalent_fast` `2.5804..2.5926 us`,
  `endpoint_invariant_reject_fast` `3.6823..3.6969 us`,
  `mixed_k3_lag3_dim3_n2048` `514.54..523.86 ms`, and
  `graph_only_k3_lag8_dim4_n8192` `74.459..75.711 ms`.
