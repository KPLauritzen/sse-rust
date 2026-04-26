# Endpoint-neighborhood normal forms for `3x3` / `4x4` square frontier states (2026-04-26)

## Question

Define a small set of candidate local normal forms for square frontier states
near the source/target endpoints, test them on representative known `k = 3`
witness states and retained `k = 4` stuck or near-miss states, and decide
whether any of them is useful without rewriting the solver's global
canonicalization path.

This slice stays bounded:

- no production canonicalization rewrite;
- no beam retune;
- no broad new search run; and
- no claim that any candidate is an SSE invariant.

## Tested sample set

I used one small reproducible corpus:

- `11` endpoint-near square witness states from the Baker lag-7 path and the
  non-Baker lag-7 replay, keeping only `3x3` / `4x4` states within endpoint
  radius `3`;
- `16` retained `k = 4` approximate-hit states from the top `8`
  Brix-Ruiz stuck pairs, recording both the frontier-side `to_matrix` and the
  opposite-side `counterpart_matrix`.

That gives `27` total samples:

- `4` states of dimension `3`;
- `23` states of dimension `4`.

The key controls in this slice are:

- Baker vs non-Baker replay duplicates at steps `1`, `2`, `3`, and `4`;
- retained diagonal hotspot rank `4`:
  - frontier state `[[1,4,2,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]`
  - counterpart `[[1,12,0,1],[1,1,4,4],[0,0,0,0],[0,0,0,0]]`;
- retained cross-family hotspot overlap:
  - rank `2` elementary-conjugation pair;
  - rank `6` diagonal-refactorization pair.

## Candidate normal forms

### Candidate A: `mass_support_signature`

Fields:

- dimension;
- total entry sum;
- sorted row sums;
- sorted column sums;
- sorted row supports; and
- sorted column supports.

This is the same coarse signature already used for approximate opposite-side
hits in `src/search.rs` and the retained stuck-state extractor.

### Candidate B: `trimmed_active_window`

Procedure:

1. take `canonical_perm()` of the square state;
2. remove all-zero rows and all-zero columns; then
3. keep the resulting active rectangular block exactly.

This keeps the local active layout while ignoring zero-padding placement.

### Candidate C: `trimmed_entry_bag_signature`

Fields of the trimmed active window:

- active shape `r x c`;
- sorted row sums;
- sorted column sums;
- sorted row supports;
- sorted column supports; and
- sorted multiset of positive entries.

This is intentionally between A and B: it keeps more than coarse marginals, but
forgets exact placement inside the active block.

## Reproducible helper

Research-only helper added:

- `src/bin/diagnose_endpoint_neighborhood_normal_forms.rs`

Command:

```bash
timeout -k 20s 180s cargo run --features research-tools \
  --bin diagnose_endpoint_neighborhood_normal_forms -- \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --stuck-report tmp/sse-rust-1j1-k4-stuck-top16.json \
  --endpoint-radius 3 \
  --top-stuck 8 \
  --json-out tmp/sse-rust-1j1-endpoint-neighborhood-normal-forms.json
```

The helper output used for this note is:

- `tmp/sse-rust-1j1-endpoint-neighborhood-normal-forms.json`

## Comparison table

| Candidate | Unique forms on 27 samples | Collision buckets | Largest bucket | What it collapses | What it preserves | Risk | Decision |
| --- | ---: | ---: | ---: | --- | --- | --- | --- |
| A. `mass_support_signature` | `14` | `11` | `4` | Baker/non-Baker replay duplicates; every retained stuck/counterpart pair; also the rank `2` elementary-conjugation pair and rank `6` diagonal-refactorization pair collapse into one `4`-way bucket | dimension and coarse endpoint mass/support profile | hides the exact active-block placement that the retained `4x4` hotspot notes identify as the missing local vocabulary | Keep, but only as a soft coarse bucket |
| B. `trimmed_active_window` | `22` | `5` | `2` | Baker/non-Baker replay duplicates; one shared target-side `4x2` window between retained ranks `2` and `6` | exact active-block layout after permutation-canonical square ordering and zero trimming | probably too fine to buy much hard duplicate reduction by itself | Narrow keep |
| C. `trimmed_entry_bag_signature` | `21` | `6` | `2` | everything B collapses, plus the retained rank `5` frontier/counterpart pair | active shape and mass multiset inside the live block | starts hiding layout distinctions even when B still separates them | Reject for deployment |

## Concrete examples

### What all three candidates do well

All three candidates collapse the exact replayed `k = 3` endpoint-near states
shared by:

- `research/guide_artifacts/k3_shortcut_round1.json`; and
- `research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json`.

The shared collisions are:

- step `1` (`3x3`);
- step `2` (`4x4`);
- step `3` (`4x4`); and
- step `4` (`4x4`).

So the candidates do recover genuine duplicate reasoning across two distinct
known witness surfaces.

### Why candidate A is too coarse

Candidate A collapses the retained diagonal hotspot rank `4` pair:

- frontier state row sums `0/0/10/14`, column sums `2/4/5/13`;
- counterpart with the same sorted sums and support profile.

That is already risky, because the active `2 x 4` layout differs and the
missing local move appears to live exactly in that layout difference.

More importantly, A also merges two different retained families into one bucket:

- rank `2` elementary-conjugation `to/counterpart`; and
- rank `6` diagonal-refactorization `to/counterpart`.

Those four states share:

- entry sum `23`;
- sorted row sums `3/4/5/11`;
- sorted column sums `0/0/6/17`; and
- the same support pattern.

That is too aggressive for a hard normal form or dedup key.

### Why candidate B is the cleanest local descriptor

Candidate B still separates the retained diagonal hotspot rank `4` pair, because
their trimmed active windows remain visibly different:

- frontier side keeps a `2 x 4` block shaped like `[[1,4,2,7],[3,1,0,6]]`;
- counterpart keeps a different `2 x 4` block shaped like
  `[[1,12,0,1],[1,1,4,4]]`.

But B does collapse one cross-family retained overlap that looks legitimate:

- the target-side counterpart windows of retained rank `2` and rank `6`
  normalize to the same `4 x 2` active block.

So B is fine-grained enough to keep the rank `4` layout distinction and still
recognize one re-used local endpoint surface across different move families.

### Why candidate C is the wrong midpoint

Candidate C keeps the same replay duplicates as B, but it also collapses the
retained rank `5` frontier/counterpart pair even though B still separates them.

That means the entry multiset is already too weak a replacement for exact
active-block layout: it forgets exactly the kind of internal placement
distinction that appears relevant in the `4x4` hotspot region.

## Recommended single application path

Use candidate B, `trimmed_active_window`, only for **Goal 4 endpoint-agnostic
parity**.

Concretely:

- keep global canonicalization unchanged;
- when a frontier state is square of size `3` or `4`, compute the trimmed
  active window after `canonical_perm()`;
- compare forward/backward square frontiers on that local descriptor when
  building parity diagnostics or future endpoint-local proposal surfaces.

Why this path and not duplicate reduction or ranking:

- A is the only form coarse enough to produce large buckets, but it clearly
  hides useful distinctions, so it should remain a soft ranking signal at most;
- B is the cleanest descriptor for comparing endpoint-local square structure
  without pretending two different active layouts are the same state; and
- C already loses useful distinctions with no compensating collapse gain.

## Keep / reject summary

- Keep A only as a coarse soft bucket, not as a hard normal form.
- Narrow-keep B as the candidate local endpoint-parity normal form.
- Reject C as a deployable normal form for now.

## Validation

Focused validation:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo test --features research-tools --bin diagnose_endpoint_neighborhood_normal_forms
timeout -k 20s 180s cargo run --features research-tools \
  --bin extract_brix_ruiz_k4_stuck_states -- \
  --json-out tmp/sse-rust-1j1-k4-stuck-top16.json \
  --top 16
timeout -k 20s 180s cargo run --features research-tools \
  --bin diagnose_endpoint_neighborhood_normal_forms -- \
  --guide-artifact research/guide_artifacts/k3_shortcut_round1.json \
  --guide-artifact research/guide_artifacts/k3_exact_endpoint_multi_meet_replay_lag7_2026-04-19.json \
  --stuck-report tmp/sse-rust-1j1-k4-stuck-top16.json \
  --endpoint-radius 3 \
  --top-stuck 8 \
  --json-out tmp/sse-rust-1j1-endpoint-neighborhood-normal-forms.json
```

Observed result:

- formatting passed;
- helper tests passed (`3` tests);
- the stuck-state extractor replay completed and wrote the retained top-`16`
  report; and
- the endpoint-neighborhood helper wrote the comparison JSON used above.
