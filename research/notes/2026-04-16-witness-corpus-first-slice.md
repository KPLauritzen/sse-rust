# First slice for a broader solved SSE witness corpus (2026-04-16)

## Question

What is the smallest durable repo surface that broadens the current `k=3`
path-heavy evidence into something useful for ranking-signal analysis?

## Recommendation

Treat the corpus unit as a **solved endpoint pair**, not as a loose path file
or a loose endpoint-only case.

For the first slice, each corpus entry should carry:

- canonical source and target matrices;
- zero or more **validated witness refs**, initially only `full_path` witnesses;
- enough provenance to say where the witness came from and how much to trust it;
- an explicit status that distinguishes:
  - endpoint pair with validated witness payload already in repo;
  - endpoint pair known from literature / harness cases but not yet materialized
    as a stored witness payload.

This is enough to support ranking-signal analysis, because the current analysis
tooling already knows how to consume:

- full-path guide artifacts,
- the legacy `research/k3-graph-paths.sqlite` path DB,
- and bounded endpoint cases from `research/cases.json`.

## Existing surfaces to reuse directly

### 1. `GuideArtifact` full-path files

`src/guide_artifacts.rs` and `src/types.rs` already give a durable witness
format with:

- endpoints,
- `kind = "full_path"`,
- full matrix/step payload,
- `validation`,
- `compatibility`,
- `quality`,
- provenance fields `source_kind`, `label`, `source_ref`.

This is already enough for the first witness-bearing corpus surface.

### 2. Existing guide pools

`research/guide_artifacts/k3_normalized_guide_pool.json` already stores `12`
validated full-path artifacts across `2` endpoint pairs, with provenance kinds:

- `benchmark_fixture`
- `literature_reference`
- `sqlite_graph`
- `sqlite_hardcoded`
- `sqlite_shortcut`

`research/guide_artifacts/k3_quotient_retained_guide_pool.json` already shows a
useful artifact-plus-metadata pattern: `5` retained artifacts across the same
`2` endpoint pairs plus quotient materialization metadata.

The generic `3x3` fixture guides are also reusable immediately:

- `research/guide_artifacts/generic_guided_permutation_3x3.json`
- `research/guide_artifacts/generic_shortcut_permutation_3x3_pool.json`

They are small, but they provide a non-`k=3` path-bearing sanity pair.

### 3. Existing legacy sqlite path DB

`research/k3-graph-paths.sqlite` is still useful as an ingestion/backfill
source:

- `graph_path_results = 2`
- `shortcut_path_results = 12`

It already stores guide labels, source-kind strings, path signatures, and the
materialized matrices for each recovered path. That is enough to backfill or
cross-check guide artifacts.

It is **not** the best first pair-level corpus surface, because it is
run/result-oriented rather than pair-oriented and it only speaks the current
path-search dialect.

### 4. Existing endpoint case corpus

`research/cases.json` already carries durable endpoint-level evidence for the
first non-Brix literature slice:

- `riedel_baker_k4`
- `riedel_baker_k6`
- `riedel_baker_k8`
- `riedel_baker_k10`
- `riedel_baker_k12`
- `lind_marcus_a_to_c`
- `full_2_shift_higher_block_1x1_to_4x4`

Those are not yet stored as a witness corpus, but they are already durable
pair-level records with ids, descriptions, tags, campaigns, and bounded search
configs.

## Minimum metadata and provenance labels

Reuse existing `GuideArtifact` fields wherever possible, and add only the
manifest-level labels that the current artifact schema cannot express.

### Reuse directly from `GuideArtifact`

- `endpoints.source`
- `endpoints.target`
- `kind`
- `validation`
- `provenance.source_kind`
- `provenance.label`
- `provenance.source_ref`
- `compatibility.supported_stages`
- `compatibility.max_endpoint_dim`
- `quality.lag`
- `quality.cost`

### Add at manifest level

- `pair_id`
  - use the existing case id when a pair already lives in `research/cases.json`;
  - otherwise use a stable repo label for the pair group.
- `evidence_status`
  - `validated_witness`
  - `endpoint_case_only`
- `witness_kind`
  - `full_path` now;
  - later this can expand to `concrete_shift`, `balanced_elementary`, or other
    structured proof payloads.
- `family_tags`
  - e.g. `brix_ruiz`, `riedel_baker`, `lind_marcus`, `higher_block`,
    `golden_mean_split`.
- `evidence_refs`
  - artifact ids, sqlite result ids, or case ids that justify the entry.

That is the minimum durable seam needed to compare ranking signals across
families without inventing a large new database schema.

## First ingestion slice from current repo evidence

### A. Ingest now as validated witness-bearing pairs

1. The current `k=3` guide corpus:
   - primary witness source:
     `research/guide_artifacts/k3_normalized_guide_pool.json`
   - derived retained subset:
     `research/guide_artifacts/k3_quotient_retained_guide_pool.json`
   - raw provenance/backfill source:
     `research/k3-graph-paths.sqlite`

2. The generic `3x3` permutation pair:
   - `research/guide_artifacts/generic_guided_permutation_3x3.json`
   - `research/guide_artifacts/generic_shortcut_permutation_3x3_pool.json`

These already have validated full-path payloads and provenance labels. They are
the right first witness-bearing slice because no new code or schema is needed
to consume them.

### B. Ingest now as endpoint-case-only pairs

Promote the existing literature-backed endpoint pairs from `research/cases.json`
into the manifest immediately, even when no stored witness payload exists yet:

- `riedel_baker_k4`
- `riedel_baker_k6`
- `riedel_baker_k8`
- `riedel_baker_k10`
- `riedel_baker_k12`
- `lind_marcus_a_to_c`
- `full_2_shift_higher_block_1x1_to_4x4`

This matters because ranking-signal work needs pair-level breadth even before
every pair has a materialized durable witness path.

### C. Defer until materialized and validated

- the golden-mean split examples from
  `research/notes/2026-04-15-non-brix-ruiz-sse-pairs.md`
- any toy balanced-elementary or concrete-shift positives that currently exist
  only as unit-test witnesses

They are real evidence, but they are not yet durable corpus payloads with repo
level provenance comparable to the guide pools or `research/cases.json`.

## Is a normalized guide-artifact corpus plus manifest enough before a wider sqlite witness corpus?

Yes, for the first useful experiment.

Reasons:

1. The current witness payload we actually know how to reuse broadly is
   `GuideArtifact(kind = full_path)`.
2. `src/bin/analyze_path_signal_corpus.rs` already consumes guide artifacts,
   sqlite path DBs, and case corpora together.
3. A new sqlite witness corpus would mostly duplicate existing path payloads
   before the repo has settled the broader witness vocabulary.
4. The real missing seam is not storage horsepower; it is a durable **pair-level
   manifest** that says which endpoint pairs exist, which ones already have
   validated witness payloads, and which provenance labels attach to them.

The main pressure that would justify a wider sqlite witness-corpus surface
later is one of:

- multiple witness payload kinds beyond `full_path`,
- cross-run query needs that file manifests cannot support cleanly,
- or large-scale corpus growth where per-file artifact envelopes become awkward.

None of those are blocking the first ranking-signal expansion slice.

## Small repo-facing step taken here

Added `research/witness_corpus_manifest.json` as the first pair-level manifest.
It is intentionally outside `research/guide_artifacts/` because the current
directory loader treats every `*.json` in that directory as a guide-artifact
payload.
