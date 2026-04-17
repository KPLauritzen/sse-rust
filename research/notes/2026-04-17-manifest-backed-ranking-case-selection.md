# Manifest-backed ranking case selection and validation (2026-04-17)

## Question

What is the smallest repo-usable way to make
`research/witness_corpus_manifest.json` active in current ranking analysis
without building a wider witness store?

## Decision

Use the current ranking analyzer as the first active manifest consumer in two
bounded ways:

- let `research/ranking_signal_family_benchmark_v1.json` select endpoint cases
  by `benchmark_role`;
- let `research/witness_corpus_manifest.json` validate that the selected
  endpoint cases still match the durable manifest endpoints.

This keeps the slice inside manifest-driven analysis and avoids inventing a new
storage layer.

## Repo-facing change

`src/bin/analyze_path_signal_corpus.rs` now supports:

- `--benchmark-role ROLE`
  - filters `research/cases.json` to family-benchmark entries with that role
    and a non-null `benchmark_case_id`;
- manifest-vs-benchmark consistency checks at catalog load time;
- manifest-vs-case endpoint validation when loading inline research cases.

The witness manifest was updated so every current `endpoint_case_only` pair now
stores explicit `source` and `target` matrices, not just a `case_id`.

That turns the manifest from passive inventory into an analysis gate:

- the family benchmark chooses which ranking cases to run;
- the witness manifest confirms the selected cases still point at the intended
  durable endpoint pairs.

## Verified current use

Verified command:

```bash
cargo run --profile dist --features research-tools --bin analyze_path_signal_corpus -- \
  --cases research/cases.json \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --benchmark-role heldout_benchmark
```

Observed current-head result:

- `endpoint_cases=7`
- `solved_cases=7`
- `ranked_solution_nodes=38/38`
- selected role: `heldout_benchmark`

So the manifest-backed held-out case slice is immediately usable in the current
ranking analyzer.

## Family coverage now

Current durable pair families in the witness/benchmark manifests:

- `brix_ruiz`
  - `brix_ruiz_k3_lind_marcus_baker_reference`
  - `brix_ruiz_k3_search_witness_pool`
  - status: validated witness
- `riedel_baker`
  - `riedel_baker_k4`
  - `riedel_baker_k6`
  - `riedel_baker_k8`
  - `riedel_baker_k10`
  - `riedel_baker_k12`
  - status: endpoint-case-only
- `lind_marcus`
  - `lind_marcus_a_to_c`
  - status: endpoint-case-only
- `higher_block`
  - `full_2_shift_higher_block_1x1_to_4x4`
  - status: endpoint-case-only
- `fixture_permutation`
  - `generic_guided_permutation_3x3`
  - status: validated witness

That is still the right bounded family slice for ranking work:

- one witness-rich development family (`brix_ruiz`);
- three held-out non-Brix literature families
  (`riedel_baker`, `lind_marcus`, `higher_block`);
- one sanity-only fixture family (`fixture_permutation`).

## Case-only pairs added or confirmed

No new endpoint-case-only pair ids were needed from `research/cases.json` in
this round.

Confirmed as the durable endpoint-case-only family coverage set:

- `riedel_baker_k4`
- `riedel_baker_k6`
- `riedel_baker_k8`
- `riedel_baker_k10`
- `riedel_baker_k12`
- `lind_marcus_a_to_c`
- `full_2_shift_higher_block_1x1_to_4x4`

What changed is that these seven pairs now carry explicit manifest endpoints,
so they can be validated against `research/cases.json` instead of only named.

The remaining `research/cases.json` entries were not promoted because they do
not broaden the durable family benchmark surface:

- smoke and negative correctness cases;
- alternative search-policy probes for already-covered Brix-Ruiz endpoints;
- fixture-backed staged comparison cases for already-covered validated pairs;
- open-family probe cases such as `brix_ruiz_k4_probe`.

## Minimum next-step surface

The next missing seam is not wider storage; it is validated-pair
disambiguation on the path side.

Current manifest detail:

- `brix_ruiz_k3_lind_marcus_baker_reference`
- `brix_ruiz_k3_search_witness_pool`

These two validated entries collapse to the same canonical endpoint pair under
the analyzer's endpoint canonicalization. That means path-derived loading can
still only attach one `pair_id` per canonical endpoint key and does not yet
have a durable way to distinguish the literature-reference slice from the
search-witness slice.

Minimum next step:

- add a bounded path-side manifest disambiguation rule, likely by promoting a
  stable provenance or witness-group label into the active pair catalog for
  validated witness pools that share canonical endpoints.

That is a concrete follow-up, but it does not block the new manifest-backed
held-out endpoint-case analysis surface landed in this slice.
