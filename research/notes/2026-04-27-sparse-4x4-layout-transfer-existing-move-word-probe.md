# Sparse `4x4` layout-transfer existing-move word probe (2026-04-27)

## Question

For bead `sse-rust-8dav`, run one bounded move-language probe over already
implemented moves only. The goal is to test whether the sparse same-profile
`4x4` layout-transfer hotspots already have a short exact word under the
current `graph_plus_structured` vocabulary, or whether the retained evidence
still points to a missing local move.

This is a diagnostic/probe only:

- no new default solver move family;
- no default solver behavior change;
- no generic `4x4` factorisation enumeration;
- no active-block contingency-switch reuse as the main experiment;
- no beam order retune; and
- no broad mixed search.

## Vocabulary And Bounds

Probe binary:

```bash
target/debug/probe_sparse_4x4_layout_transfer_words
```

Implementation:

- `src/bin/probe_sparse_4x4_layout_transfer_words.rs`
- registered as a research-only binary in `Cargo.toml`

The probe uses only:

- `MoveFamilyPolicy::GraphPlusStructured`;
- `FrontierMode::Bfs`;
- graph moves already emitted by the current frontier expansion; and
- currently selected `graph_plus_structured` factorisation families from
  `visit_factorisations_with_family_for_policy`.

Evidence bounds:

| Bound | Value |
| --- | ---: |
| max word depth | `3` |
| max intermediate dimension | `4` |
| max factorisation entry | `12` |
| cases | exactly the three controls below |

The `max_entry = 12` bound is deliberate: the retained Brix rank-4 counterpart
contains entry `12`, while the Baker control still stays inside its known
smaller witness entries.

## Exact Comparison Cases

### 1. Baker `A4 -> A5`

Source, Baker `A4`:

```text
[[1,2,2,0],
 [1,1,1,1],
 [0,1,0,1],
 [0,2,1,0]]
```

Target, Baker `A5`:

```text
[[1,1,1,1],
 [3,0,2,2],
 [1,0,0,0],
 [0,1,1,1]]
```

### 2. Brix-Ruiz `k = 4` retained rank-4 sparse `2x4` near-hit

Source, rank-4 diagonal-refactorization frontier child:

```text
[[1,4,2,7],
 [3,1,0,6],
 [0,0,0,0],
 [0,0,0,0]]
```

Target, rank-4 closest opposite-side counterpart:

```text
[[1,12,0,1],
 [1, 1,4,4],
 [0, 0,0,0],
 [0, 0,0,0]]
```

### 3. Brix-Ruiz `k = 4` retained rank-6 sparse `4x2` near-hit

Source, rank-6 diagonal-refactorization frontier child:

```text
[[0, 2,3,0],
 [0, 2,1,0],
 [0,11,0,0],
 [0, 2,2,0]]
```

Target, rank-6 closest opposite-side counterpart:

```text
[[0, 2,1,0],
 [0, 1,4,0],
 [0, 3,1,0],
 [0,11,0,0]]
```

## Commands And Artifact

Focused build/test:

```bash
timeout -k 20s 240s cargo test --features research-tools \
  --bin probe_sparse_4x4_layout_transfer_words -- --test-threads=1

timeout -k 20s 180s cargo build --features research-tools \
  --bin probe_sparse_4x4_layout_transfer_words
```

Evidence regeneration:

```bash
timeout -k 20s 180s target/debug/probe_sparse_4x4_layout_transfer_words \
  --max-depth 3 \
  --max-intermediate-dim 4 \
  --max-entry 12 \
  --json-out tmp/sse-rust-8dav-sparse-4x4-layout-transfer-words-depth3.json
```

Artifact:

- `tmp/sse-rust-8dav-sparse-4x4-layout-transfer-words-depth3.json`

## Results

| Case | Outcome | Direct one-step families | Word / negative result | Expanded | Factorisations | Candidates kept | Approx hits |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| Baker `A4 -> A5` | `equivalent`, lag `3` | none | `binary_sparse_rectangular_factorisation_4x3_to_3 -> permutation_relabeling -> insplit` | `2` | `33` | `19` | `0` |
| Brix rank-4 frontier child -> counterpart | `unknown` | none | no exact word within depth `3`, dim `4`, entry `12` | `10` | `222` | `272` | `1` |
| Brix rank-6 frontier child -> counterpart | `unknown` | none | no exact word within depth `3`, dim `4`, entry `12` | `7` | `85` | `131` | `1` |

Baker path matrices from the generated artifact:

```text
[[1,2,2,0],[1,1,1,1],[0,1,0,1],[0,2,1,0]]
-> [[1,1,1],[2,0,3],[1,1,1]]
-> [[1,1,1],[3,0,2],[1,1,1]]
-> [[1,1,1,1],[3,0,2,2],[1,0,0,0],[0,1,1,1]]
```

This is the same short-word reading as the earlier Baker bridge evidence, but
with the relabeling placed before the final split in the regenerated path.

## Reading

The exact control behaves as expected: Baker `A4 -> A5` is not a one-step
current family, but it does have a short length-3 word over implemented moves.
The word still drops through a `3x3` bridge and re-expands, so it does not
provide a direct same-size sparse `4x4` layout-transfer family.

The retained Brix-Ruiz rank-4 and rank-6 sparse near-hit shapes do not have a
depth-3 exact word under the current selected `graph_plus_structured`
vocabulary, even though each probe sees one approximate opposite-side hit. That
preserves the previous missing-word interpretation: the current vocabulary can
reach the coarse sparse profile surface, but still does not spell the active
layout transfer exactly in this bounded slice.

## Keep / Reject / Next Step

Keep:

- the research-only probe binary as a small regeneration hook for this negative
  result;
- Baker `A4 -> A5` as the exact length-3 control; and
- the retained Brix rank-4/rank-6 pair definitions as the sparse `4x4`
  frontier/counterpart controls.

Reject from this slice:

- promoting any new solver move;
- reopening generic `4x4` factorisation enumeration;
- reusing the already rejected contingency-switch diagnostic as the answer; and
- retuning beam order.

Next step: no new bead is opened from this result. The evidence does not
identify a non-duplicate next family hypothesis; it only confirms the already
recorded vocabulary gap: a narrow, SSE-valid same-profile sparse `4x4`
layout-transfer move remains missing from the selected implemented vocabulary.
