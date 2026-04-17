# Known-pair policy coverage over current SSE pairs (2026-04-17)

## Question

On current `HEAD`, how do bounded `graph_only`, `graph_plus_structured`, and
`mixed` endpoint-search lanes compare over the repo's current durable known-SSE
pair set?

This round stays measurement-first:

- no solver rewrites;
- one reusable evidence-lane harness corpus;
- one durable JSON run artifact;
- and one current-head answer for the canonical hard `brix_ruiz_k3` pair.

## Sources and run surfaces

- durable pair inventory:
  - `research/witness_corpus_manifest.json`
  - `research/cases.json`
- reusable comparison corpus added for this slice:
  - `research/known_pair_policy_coverage_corpus_2026-04-17.json`
- saved harness artifact for this slice:
  - `research/runs/2026-04-17-known-pair-policy-coverage.json`

Coordinator-referenced post-merge artifacts
`research/runs/2026-04-17-post-merge-2uy19.json` and
`research/runs/2026-04-17-post-merge-2uy23.json` were not present in this
worktree, so the conclusions below come from fresh current-`HEAD` reruns.

## Tested known-pair set

The tested set is the current durable ten-pair manifest slice:

- validated-witness pairs:
  - `brix_ruiz_k3_lind_marcus_baker_reference`
  - `brix_ruiz_k3_search_witness_pool`
  - `generic_guided_permutation_3x3`
- endpoint-case-only pairs:
  - `riedel_baker_k4`
  - `riedel_baker_k6`
  - `riedel_baker_k8`
  - `riedel_baker_k10`
  - `riedel_baker_k12`
  - `lind_marcus_a_to_c`
  - `full_2_shift_higher_block_1x1_to_4x4`

The only pair that did not already have a direct harness case was
`brix_ruiz_k3_search_witness_pool`. For that pair, the comparison corpus uses
the witness-backed envelope implied by the durable `endpoint_16_path`
materialization:

- `max_lag = 16`
- `max_intermediate_dim = 5`
- `max_entry = 6`

## Comparison policy

For each pair, this slice fixed one bounded endpoint-search envelope and varied
only `move_family_policy`:

- `graph-only`
- `graph_plus_structured`
- `mixed`

The envelope stayed constant within each pair. That keeps the comparison
policy-comparable without forcing one global bound that would either trivialize
the small pairs or over-widen the hard Brix-Ruiz surfaces.

Per-pair envelopes:

| Pair | Bound |
| --- | --- |
| `brix_ruiz_k3_lind_marcus_baker_reference` | `lag6 / dim3 / entry6` |
| `brix_ruiz_k3_search_witness_pool` | `lag16 / dim5 / entry6` |
| `generic_guided_permutation_3x3` | `lag1 / dim3 / entry2` |
| `riedel_baker_k4` | `lag5 / dim3 / entry4` |
| `riedel_baker_k6` | `lag6 / dim3 / entry6` |
| `riedel_baker_k8` | `lag8 / dim3 / entry8` |
| `riedel_baker_k10` | `lag10 / dim3 / entry10` |
| `riedel_baker_k12` | `lag12 / dim3 / entry12` |
| `lind_marcus_a_to_c` | `lag2 / dim2 / entry2` |
| `full_2_shift_higher_block_1x1_to_4x4` | `lag4 / dim4 / entry2` |

## Coverage summary

Strategy totals from `research/runs/2026-04-17-known-pair-policy-coverage.json`:

- `graph_only`: `4/10 equivalent`, `6/10 unknown`, `0 timeout`, total elapsed
  `8146 ms`
- `graph_plus_structured`: `8/10 equivalent`, `1/10 unknown`, `1 timeout`,
  total elapsed `14776 ms`
- `mixed`: `8/10 equivalent`, `1/10 unknown`, `1 timeout`, total elapsed
  `16075 ms`

Pair-by-pair outcomes:

| Pair | `graph_only` | `graph_plus_structured` | `mixed` |
| --- | --- | --- | --- |
| `brix_ruiz_k3_lind_marcus_baker_reference` | `unknown` (`2 ms`) | `unknown` (`60 ms`) | `unknown` (`416 ms`) |
| `brix_ruiz_k3_search_witness_pool` | `equivalent`, lag `17` (`8137 ms`) | `timeout` (`12029 ms`) | `timeout` (`12035 ms`) |
| `generic_guided_permutation_3x3` | `equivalent`, lag `1` (`0 ms`) | `equivalent`, lag `1` (`0 ms`) | `equivalent`, lag `1` (`0 ms`) |
| `riedel_baker_k4` | `unknown` (`1 ms`) | `equivalent`, lag `5` (`4 ms`) | `equivalent`, lag `5` (`85 ms`) |
| `riedel_baker_k6` | `unknown` (`1 ms`) | `equivalent`, lag `7` (`29 ms`) | `equivalent`, lag `7` (`256 ms`) |
| `riedel_baker_k8` | `unknown` (`1 ms`) | `equivalent`, lag `9` (`145 ms`) | `equivalent`, lag `9` (`399 ms`) |
| `riedel_baker_k10` | `unknown` (`1 ms`) | `equivalent`, lag `11` (`564 ms`) | `equivalent`, lag `11` (`715 ms`) |
| `riedel_baker_k12` | `unknown` (`1 ms`) | `equivalent`, lag `13` (`1943 ms`) | `equivalent`, lag `13` (`2167 ms`) |
| `lind_marcus_a_to_c` | `equivalent`, lag `2` (`1 ms`) | `equivalent`, lag `2` (`0 ms`) | `equivalent`, lag `2` (`0 ms`) |
| `full_2_shift_higher_block_1x1_to_4x4` | `equivalent`, lag `3` (`1 ms`) | `equivalent`, lag `4` (`2 ms`) | `equivalent`, lag `4` (`2 ms`) |

## What the comparison says

### Structured moves help

The clear win for structured moves is the full tested `riedel_baker` ladder:

- `graph_only` stayed `unknown` on `k = 4, 6, 8, 10, 12`;
- both `graph_plus_structured` and `mixed` solved all five rungs;
- `graph_plus_structured` matched `mixed` witness lag on every rung and was
  faster on every rung.

This is the strongest current-head evidence that the explicit structured lane is
actually broadening bounded proof coverage on at least one durable family.

### Structured moves stay neutral

Coverage-neutral families in this slice:

- `generic_guided_permutation_3x3`
- `lind_marcus_a_to_c`
- `full_2_shift_higher_block_1x1_to_4x4`

All three policies solved these pairs under the chosen bounds.

`graph_plus_structured` is also coverage-neutral relative to `mixed` over the
full ten-pair set:

- both solved `8/10`;
- both missed the same two Brix-Ruiz pairs in the pair-sweep corpus;
- but `graph_plus_structured` was still cheaper overall (`14776 ms` vs
  `16075 ms`) and faster on every nontrivial solved `riedel_baker` rung.

### Structured moves hurt

The current counterexample is
`brix_ruiz_k3_search_witness_pool`:

- `graph_only` found a lag-`17` witness in `8137 ms`;
- `graph_plus_structured` timed out at `12029 ms`;
- `mixed` timed out at `12035 ms`.

So the broader move vocabularies are not dominating graph-only on all current
known Brix-Ruiz surfaces. The validated search-witness-pool pair remains a real
graph-only advantage case on current `HEAD`.

### No rescue on the tight literature-reference Brix bound

On the tighter comparable literature-reference envelope
`lag6 / dim3 / entry6`, none of the three policies solved
`brix_ruiz_k3_lind_marcus_baker_reference`.

The broader policies therefore do not automatically rescue the canonical hard
pair when the bound stays at the old mixed-baseline scale.

## Explicit current-head answer for `brix_ruiz_k3`

For the canonical existing `research/cases.json` lanes on current `HEAD`:

- `brix_ruiz_k3_graph_only` (`lag22 / dim5 / entry6`, `graph-only`):
  `equivalent` in `9317 ms`, witness lag `17`
- `brix_ruiz_k3_graph_plus_structured` (`lag8 / dim4 / entry5`,
  `graph_plus_structured`):
  `equivalent` in `3312 ms`, witness lag `8`
- `brix_ruiz_k3` mixed baseline (`lag6 / dim3 / entry6`, `mixed`):
  `unknown` in `1235 ms`
- `brix_ruiz_k3_wide_probe` mixed wider bounded lane
  (`lag8 / dim3 / entry8`, `mixed`):
  `unknown` in `1580 ms`

So the current-head answer is:

- `graph_only` still solves the canonical hard pair on its durable witness lane;
- `graph_plus_structured` also solves it, and does so on a tighter bounded
  envelope;
- the current durable bounded `mixed` lanes still do not solve it.

That means the answer is **not** "graph_only still solves while broader
policies do not." The current more precise statement is:

- `graph_only` is no longer the only solving lane for the canonical
  `brix_ruiz_k3` pair, because `graph_plus_structured` also solves;
- but graph-only still uniquely wins on the separate validated
  `brix_ruiz_k3_search_witness_pool` pair within the tested known-pair corpus.

## Conclusion

Current `HEAD` gives a split answer rather than one global winner:

- `graph_plus_structured` is the best bounded intermediate policy on the tested
  literature/non-Brix ladder: it broadens coverage relative to `graph_only`
  and matches `mixed` coverage more cheaply.
- `graph_only` still owns one important Brix-Ruiz surface: the validated
  search-witness-pool pair.
- `mixed` does not add coverage over `graph_plus_structured` on this ten-pair
  set, and it is slightly slower overall.

For this bead's measurement-first question, the durable answer is that
structured moves **help on the Riedel family, stay neutral on the easy and
small non-Brix pairs, and hurt on at least one current Brix-Ruiz witness pair**.
