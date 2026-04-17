# Validated path-pair disambiguation from manifest evidence refs (2026-04-17)

## Question

How should `analyze_path_signal_corpus` distinguish validated manifest pair
groups that collapse to the same canonical endpoint pair, without building a
new witness store?

Current concrete ambiguity:

- `brix_ruiz_k3_lind_marcus_baker_reference`
- `brix_ruiz_k3_search_witness_pool`

These are different validated witness pools, but their endpoints normalize to
the same canonical pair under `canonical_perm()`.

## Decision

Use `research/witness_corpus_manifest.json` `validated_pairs[].evidence_refs`
as the active path-side disambiguation contract.

Resolution rule:

1. For guide artifacts, resolve `pair_id` by matching the loaded
   `(validated_witness_source.id, artifact_id)` against `evidence_refs`.
2. For legacy sqlite path DB rows, resolve `pair_id` by matching the loaded
   `(validated_witness_source.id, result table, result id)` against
   `evidence_refs.result_refs`.
3. Treat the manifest pair endpoints as a guard, not as the primary selector:
   the evidence-ref match is accepted only if the loaded path's canonical
   endpoints still match the manifest pair endpoints.
4. If no evidence-ref rule matches, fall back to endpoint-only path resolution
   only when the canonical endpoint pair is unique in the validated manifest
   slice.
5. If multiple validated pairs share the canonical endpoints and no
   evidence-ref rule matches, leave path-side `pair_id` unresolved instead of
   arbitrarily collapsing to one pair.

This keeps the change entirely inside the existing manifest/analyzer seam.

## Why This Is Bounded

- No new witness store or secondary index was added.
- No new benchmark manifest was needed.
- The existing manifest already contained the durable discriminators:
  `artifact_ids` for guide pools and `result_refs` for the legacy sqlite backfill.

The only behavioral change is that the analyzer now treats those refs as active
metadata instead of passive documentation.

## Pair Metadata Contract

### `pair_id`

For endpoint-case loading, `pair_id` keeps its existing meaning: the durable
manifest pair chosen by `benchmark_case_id`.

For validated path-side loading, `pair_id` now means:

- the validated manifest witness-pool id selected by `evidence_refs`, when a
  matching evidence ref exists;
- otherwise the unique validated endpoint pair, if the canonical endpoints are
  unambiguous;
- otherwise unresolved (`null` on the exported artifact).

Important consequence:

- path-derived analysis may now carry multiple `pair_id` values for the same
  canonical endpoint pair;
- that is intentional and bounded to validated witness-pool disambiguation on
  the development path side.

To preserve that distinction, path dedup now keys exact duplicate full paths
and derived segment endpoints by `pair_id` when present, instead of collapsing
across pair groups that merely share canonical endpoints.

### `evaluation_family_id`

`evaluation_family_id` is unchanged.

Both ambiguous Brix-Ruiz validated pairs still map to:

- `evaluation_family_id = "brix_ruiz"`

So the new rule does **not** create new evaluation families and does **not**
turn provenance labels into a benchmark split axis.

### Benchmark-facing family splits

Benchmark-facing family splits are unchanged because the primary benchmark path
still runs on bounded endpoint cases selected by `benchmark_role` and
`benchmark_case_id`.

That means:

- held-out family selection still happens in
  `research/ranking_signal_family_benchmark_v1.json`;
- the current headline benchmark remains one bounded endpoint case per pair;
- path-derived segments remain development-only diagnostics.

So the new path-side disambiguation rule broadens development analysis fidelity
without changing the benchmark split contract.

## Verified current-head behavior

Bounded path-side validation:

```bash
target/dist/analyze_path_signal_corpus \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --min-gap 2 \
  --max-gap 4 \
  --max-cases 24 \
  --max-endpoint-dim 4 \
  --max-intermediate-dim 4 \
  --max-entry 6 \
  --emit-layer-contrasts tmp/4ue-path-disambiguation-wide.json
```

Observed exported path-side `pair_id`s:

- `brix_ruiz_k3_lind_marcus_baker_reference`
- `brix_ruiz_k3_search_witness_pool`

Example emitted labels:

- `k3-lind-marcus-baker-lag7 [1..3]` ->
  `brix_ruiz_k3_lind_marcus_baker_reference`
- `k3-sqlite-shortcut-7 [1..3]` ->
  `brix_ruiz_k3_search_witness_pool`

Held-out benchmark-role non-regression:

```bash
target/dist/analyze_path_signal_corpus \
  --cases research/cases.json \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --benchmark-role heldout_benchmark
```

Observed current-head result stayed:

- `endpoint_cases=7`
- `solved_cases=7`
- `ranked_solution_nodes=38/38`

So the new rule fixes the concrete validated-pair ambiguity on the path side
without breaking the newer manifest-backed benchmark-role endpoint path.
