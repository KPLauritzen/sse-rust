# Layer-contrast ranking labels from solved searches (2026-04-16)

## Question

Among sibling candidates in one observed search layer, how do we label
"better continuation" without collapsing back to trivial SSE reachability or
cross-family leakage?

## Decision

Use **same-layer continuation labels** extracted from bounded reruns of solved
cases.

For one observed layer:

- `best_continuation`
  - candidate lies on the returned solved path for that search direction;
  - among matched witness siblings, it has the smallest
    `remaining_witness_lag`.
- `supporting_continuation`
  - candidate lies on the returned solved path for that direction;
  - but another matched sibling in the same layer has smaller
    `remaining_witness_lag`.
- `non_continuation`
  - candidate is a sibling in the same observed layer but is not on the
    returned solved path for that direction.

This is a **within-layer continuation-quality** label, not a reachability
label:

- every sibling already belongs to an endpoint pair that this bounded rerun
  solved;
- the label asks which sibling stays on the recovered continuation, not whether
  the endpoint pair is solvable in principle;
- path-segment cases remain development-only under the family-aware evaluation
  contract and do not become headline benchmark cases.

## Extraction method

1. Load either:
   - full-path witness sources from guide artifacts, then derive bounded segment
     cases; or
   - bounded endpoint cases from `research/cases.json`.
2. Re-run each case with `SearchObserver` enabled.
3. For each observed layer, canonicalize candidates and intersect them with the
   interior states of the returned solved path.
4. Compute `remaining_witness_lag` in the search direction:
   - forward layer: steps remaining from the candidate to the target;
   - backward layer: steps remaining from the candidate to the source.
5. Emit a rankable layer only when at least one sibling matches the returned
   solved path.
6. Inside that layer, label the matched sibling with minimum remaining lag as
   `best_continuation`; other matched siblings become
   `supporting_continuation`; every other sibling is implicitly
   `non_continuation`.

## Dedup and leakage rules

Align the labels with
`research/notes/2026-04-16-family-aware-ranking-signal-eval-contract.md`:

- canonicalize endpoints and candidate states with `canonical_perm()` before
  matching or manifest lookup;
- resolve `pair_id` for witness-derived `k3` path cases from
  `research/witness_corpus_manifest.json`;
- resolve `evaluation_family_id` and `benchmark_role` from
  `research/ranking_signal_family_benchmark_v1.json`;
- keep `path_segment` examples development-only by leaving
  `benchmark_case_id = null`;
- keep endpoint-case examples on their existing `benchmark_case_id = pair_id`;
- dedup repeated witness-state sightings to the first observed layer per
  `(pair_id, benchmark_case_id-or-source-case, direction, canonical_candidate_state)`;
- do not use artifact ids, sqlite row ids, or path-segment labels as split
  axes.

Important consequence:

- later pairwise training rows should be derived from these layer sets on read;
- the durable repo artifact should store the sibling set once and avoid
  materializing unstable `O(n^2)` pairwise expansions.

## Storage shape

Added bounded export support to `src/bin/analyze_path_signal_corpus.rs` via:

- `--witness-manifest`
- `--family-benchmark`
- `--emit-layer-contrasts`

The first artifact is:

- `research/layer_contrast_signal_corpus_first_pass.json`

Shape:

- top-level `config` and `summary`
- `cases[*]`
  - case label, pair metadata, and `contrast_source_kind`
- `rankable_layers[*]`
  - direction, `layer_size`, dedup scope key, and only the matched witness
    candidates with their continuation labels and remaining witness lag

`non_continuation` remains implicit at the manifest level:

- the durable artifact records which sibling won inside a rankable layer and how
  large the sibling set was;
- when a later experiment needs the full negative sibling pool, it should rerun
  `analyze_path_signal_corpus` on the same bounded case surface and recover that
  exact layer by `(case label, direction, layer_index)`.

This is intentionally a small **anchor manifest** over the existing analyzer
surface rather than a fully materialized training table.

## First bounded extraction pass

Command:

```bash
cargo run --quiet --features research-tools --bin analyze_path_signal_corpus -- \
  --guide-artifacts research/guide_artifacts/k3_normalized_guide_pool.json \
  --cases research/cases.json \
  --case-id riedel_baker_k4 \
  --case-id riedel_baker_k6 \
  --case-id riedel_baker_k8 \
  --case-id riedel_baker_k10 \
  --case-id riedel_baker_k12 \
  --max-gap 2 \
  --max-cases 12 \
  --max-endpoint-dim 4 \
  --search-mode graph-only \
  --witness-manifest research/witness_corpus_manifest.json \
  --family-benchmark research/ranking_signal_family_benchmark_v1.json \
  --emit-layer-contrasts research/layer_contrast_signal_corpus_first_pass.json
```

Observed summary:

- `source_paths = 12`
- `path_segment_cases = 12`
- `endpoint_cases = 5`
- `solved_cases = 12`
- `unsolved_cases = 5`
- `ranked_solution_nodes = 37 / 40`
- exported families:
  - `brix_ruiz`
  - `riedel_baker`
- exported rankable cases: `7`
- exported rankable layers: `44`
- exported matched witness candidates: `44`

Coverage by surface:

- `brix_ruiz` development-only path segments:
  - `2` rankable `k3` cases;
  - `4` rankable layers total.
- `riedel_baker` held-out endpoint cases:
  - all `5` requested cases rankable;
  - `40` rankable layers total.

Current limitation of this first pass:

- every rankable layer contained exactly one matched witness sibling, so the
  artifact currently instantiates only `best_continuation` vs implicit
  `non_continuation`;
- `supporting_continuation` remains part of the schema, but it did not occur in
  this bounded pass;
- the other non-Brix held-out families (`lind_marcus`, `higher_block`) remain
  excluded from this artifact because the current analyzer surface still does
  not produce rankable observer layers for them.

## Consequence

This gives the repo a first durable within-layer ranking-label surface that is:

- sibling-local rather than endpoint-reachability-local;
- aligned with the family-held-out split contract;
- small enough to keep as one repo artifact without introducing a broader data
  pipeline.

It is enough for development-side ranking-signal ablations and later derived
pairwise sampling, but it is not yet a benchmark-complete held-out corpus.
