# Transfer search policy: witness bridge profile beam (2026-04-25)

## Question

Can a single frozen support-profile ranking rule transferred from solved witness
surfaces improve useful reach on the retained open Brix-Ruiz `k = 4`
`graph_plus_structured` lane, without adding moves or changing default search
behavior?

This is bead `sse-rust-ejj`.

## Training/control surfaces used before freezing

The rule is hand-derived only from documented solved/control evidence:

- `research/notes/2026-04-25-witness-bridge-motif-inventory.md`
- `research/notes/2026-04-25-concrete-shift-profile-beam-prototype.md`
- `research/notes/2026-04-25-same-future-past-diversity-layer-correlation.md`
- `research/notes/2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`

Positive training signal:

- repeated solved `k = 3` bridge profiles:
  - `3x3`: support `7`, row/column supports `1/3/3 x 2/2/3`
    or transpose;
  - `3x3`: support `8`, row/column supports `2/3/3 x 2/3/3`;
  - `4x4`: support `11`, row/column supports `2/2/3/4 x 2/2/3/4`;
  - `4x4`: support `11`, row/column supports `2/3/3/3 x 1/3/3/4`
    or transpose.

Negative controls:

- `concrete_shift_profile_beam` was already rejected for this transfer slice;
- `same_future_past_diversity_beam` is not reopened as a ranking/admission
  policy;
- the Brix-Ruiz `k = 4` stuck-state note is used only as retained lane context,
  not as a tuning surface for this rule.

## Frozen rule

Add opt-in frontier mode:

```text
witness_bridge_profile_beam
```

The mode uses the existing beam executor and default beam score. For each
candidate matrix, compute:

```text
(dimension, nonzero support count, sorted row supports, sorted column supports)
```

If that tuple exactly matches one of the solved `k = 3` bridge/plateau/bounce
profiles listed above, subtract `48.0` from the raw default beam score before
the existing integer beam scaling. Otherwise leave the score unchanged.

This is a ranking/admission preference only. It does not prune, does not add a
move family, and does not change default `beam`, `bfs`, or
`graph_plus_structured` behavior.

The rule is frozen here before evaluating the retained open `k = 4` lane.

## Evaluation cases

Planned A/B cases:

- `brix_ruiz_k3_graph_plus_structured_beam_probe`
- `brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`

Artifacts will be saved under `tmp/`.

## Raw metrics

Artifacts:

- `tmp/ejj_witness_bridge_profile_ab_cases.json`
- `tmp/ejj_witness_bridge_profile_ab_results.json`

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
  tmp/ejj_witness_bridge_profile_ab_results.json
```

`k = 3` control:

| Case | Outcome | Target hits | Approx hits | Max frontier | Visited | Expanded | Factorisations | Kept candidates | Discovered | Elapsed ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline `beam` | `unknown` | `0` | `10` | `10` | `2631` | `142` | `21653` | `3501` | `2629` | `571` |
| `witness_bridge_profile_beam` | `unknown` | `0` | `10` | `10` | `2238` | `142` | `25856` | `2974` | `2236` | `612` |

Retained/open `k = 4` graph-plus-structured lane:

| Case | Outcome | Target hits | Approx hits | Max frontier | Visited | Expanded | Factorisations | Kept candidates | Discovered | Elapsed ms |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| baseline `beam` | `unknown` | `0` | `184` | `256` | `176664` | `19970` | `487699` | `271803` | `176662` | `23572` |
| `witness_bridge_profile_beam` | `unknown` | `0` | `198` | `256` | `167594` | `19970` | `497498` | `259329` | `167592` | `22545` |

## Decision

Keep the implementation as an opt-in research mode. Do not promote it to the
default beam or graph-plus-structured behavior.

Reading:

- no exact target hit was found on either case;
- the `k = 3` control is mixed: approximate hits are unchanged and visited
  nodes are lower, but factorisations and elapsed time are worse;
- the retained/open `k = 4` lane improves useful reach under the fixed budget:
  approximate hits increase from `184` to `198`, visited nodes drop by `9070`,
  kept candidates drop by `12474`, and elapsed time drops by about `1.0s`,
  while factorisations rise by `9799`.

This is not enough evidence for default promotion because the control result is
mixed and the open `k = 4` lane has only one frozen evaluation. It is enough to
keep the gated mode for held-out validation.

## Follow-up

Created bounded follow-up bead `sse-rust-tk0l`:

```text
Validate witness_bridge_profile_beam on held-out graph_plus_structured controls
```

Scope: no retuning; run the frozen mode against at least two held-out
graph-plus-structured controls, including one non-Brix-Ruiz solved or near-solved
lane if available, and decide whether the positive retained-k4 signal
replicates or is noisy.

## Validation

Focused tests:

```bash
timeout -k 20s 180s cargo test -q witness_bridge_profile --features research-tools
timeout -k 20s 180s cargo test --features research-tools frontier_mode
timeout -k 20s 180s cargo test -q parse_cli_accepts_witness_bridge_profile_beam_mode --features research-tools
```

Build and bounded A/B:

```bash
timeout -k 20s 180s cargo build --features research-tools --bin research_harness
timeout -k 20s 140s target/debug/research_harness \
  --cases tmp/ejj_witness_bridge_profile_ab_cases.json \
  --format json \
  > tmp/ejj_witness_bridge_profile_ab_results.json
```

`cargo bench --bench search -- --noplot` was not run for this slice because the
default hot path is unchanged: the new scoring path is only reachable through
the opt-in `witness_bridge_profile_beam` frontier mode.
