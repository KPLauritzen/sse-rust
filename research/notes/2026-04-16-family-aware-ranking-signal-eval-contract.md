# Family-aware ranking-signal evaluation contract (2026-04-16)

## Question

How do we compare ranking signals without leaking witness family identity or
near-duplicate states across train/test?

## Decision

Use a **family-held-out benchmark** backed by canonical endpoint pairs.

The contract is:

- `pair_id` is the atomic evidence and dedup unit;
- `evaluation_family_id` is the split unit;
- the primary benchmark uses one bounded endpoint-case surface per pair;
- path-segment expansion from stored witness paths is development-only
  diagnostics, not the headline benchmark.

This keeps later ranking-model or heuristic-fitting work from learning one
family's witness vocabulary and then reporting that as generalization.

The reusable repo surface for this decision is
`research/ranking_signal_family_benchmark_v1.json`.

## Family boundaries

Assign each pair exactly one stable `evaluation_family_id`. Pick it from the
**endpoint-pair construction or literature family**, not from witness
provenance.

Current family mapping:

- `brix_ruiz`
  - `brix_ruiz_k3_lind_marcus_baker_reference`
  - `brix_ruiz_k3_search_witness_pool`
- `riedel_baker`
  - `riedel_baker_k4`
  - `riedel_baker_k6`
  - `riedel_baker_k8`
  - `riedel_baker_k10`
  - `riedel_baker_k12`
- `lind_marcus`
  - `lind_marcus_a_to_c`
- `higher_block`
  - `full_2_shift_higher_block_1x1_to_4x4`
- `fixture_permutation`
  - `generic_guided_permutation_3x3`

Important consequence:

- the Baker-reference witness for the `brix_ruiz_k3` pair is still
  `evaluation_family_id = "brix_ruiz"`;
- tags such as `lind_marcus`, `baker`, `graph_search`, `shortcut_search`,
  `literature_reference`, or `benchmark_fixture` are **provenance labels**, not
  benchmark families.

That is stricter than the current `family_tags` list in
`research/witness_corpus_manifest.json`, and that is intentional. The split
contract needs one coarse family label per pair, not a bag of overlapping
source tags.

## Leakage and dedup rules

1. Canonicalize before grouping or deduping.
   Endpoints and interior witness states use `DynMatrix::canonical_perm()`.
2. The atomic split unit is the canonical endpoint pair.
   All guide artifacts, sqlite rows, derived segment cases, and endpoint-case
   records that normalize to the same endpoints stay on the same side.
3. The evaluation split unit is `evaluation_family_id`.
   No family may appear on both train/dev and held-out sides in one experiment.
4. Witness provenance is never a split axis.
   The same pair found via `benchmark_fixture`, `literature_reference`,
   `sqlite_graph`, or `sqlite_shortcut` remains one pair in one family.
5. Exact duplicate witness paths are deduped by canonical path signature before
   deriving training examples or dev summaries.
6. Path-segment expansion from stored full paths is diagnostic-only for the
   primary benchmark.
   Overlapping subsegments from one witness path expose the same interior states
   many times and would otherwise overweight one family or one path lineage.
7. If a future materialized pair shares any canonical interior witness state
   with a pair already assigned to another split, collapse both into one
   leakage group and keep them on the same side until a stronger equivalence
   rule exists.
8. For benchmark scoring, repeated sightings of the same witness state do not
   create extra weight.
   Dedup to one ranked observation per
   `(pair_id, benchmark_case_id, canonical_solution_state, direction)`.

## Primary ranking metrics

Use macro averages, not node-weighted totals.

Primary metric:

1. For each pair, compute mean percentile over its deduped rankable witness
   observations.
2. For each family, compute the unweighted mean of its pair scores.
3. The headline score is the family-macro mean percentile across held-out
   benchmark families.
4. Lower is better.

Secondary metric:

- family-macro top-1 rate across held-out benchmark families.

Mandatory coverage diagnostics:

- held-out families present;
- held-out families rankable;
- rankable held-out pairs;
- ranked held-out witness observations;
- unranked solved held-out pairs.

Node-weighted totals from `analyze_path_signal_corpus` stay useful as telemetry,
but they are not the primary benchmark score because a long ladder like
`riedel_baker_k12` would otherwise dominate the whole comparison.

## Minimum held-out family coverage

The first meaningful held-out benchmark should be the non-Brix literature
families:

- `riedel_baker`
- `lind_marcus`
- `higher_block`

A result is benchmark-meaningful only if:

- all three held-out families are present in the evaluated corpus;
- at least two of the three produce rankable observations;
- at least four held-out pairs are rankable in total;
- at least twenty held-out witness observations are rankable in total.

If fewer than two held-out families are rankable, the run is **exploratory
only**. Report the coverage miss explicitly and do not claim a benchmark win
from the scalar score.

Current status under this contract:

- `riedel_baker` is rankable in the 2026-04-15 analyzer pass;
- `lind_marcus` and `higher_block` solved but produced zero rankable observer
  layers in that pass;
- therefore current non-Brix comparisons are still useful for development, but
  not yet strong enough to support a benchmark headline.

## How this fits current tooling

`src/bin/analyze_path_signal_corpus.rs` already exposes the right raw surfaces:

- path-derived segment cases from guide artifacts or sqlite path DBs;
- bounded endpoint cases from `research/cases.json`;
- percentile summaries, top-1 counts, and case-level coverage.

What changes here is the **evaluation contract**, not the scoring binary:

- endpoint-case runs become the primary held-out benchmark surface;
- path-derived segments remain useful for development-time mining and ablation;
- future fitting work should read the split manifest first and only then decide
  which development-side path or endpoint material is allowed.

## Reusable manifest

Added `research/ranking_signal_family_benchmark_v1.json` as the first durable
split manifest for ranking-signal work. It records:

- the stable family mapping;
- development vs held-out vs sanity-only roles;
- the first coverage gate for a meaningful held-out result;
- the benchmark-facing leakage rules that later fitting code must obey.
