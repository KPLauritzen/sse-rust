# Brix-Ruiz `k=4` graph-plus-structured beam-direction signal retune was a no-op (2026-04-18)

## Question

On the retained Brix-Ruiz `k=4` `graph_plus_structured` measurement surface
(`beam256 + dim4 + entry12`), does a narrow same-depth beam-direction retune
help bounded ranking quality if beam mode consults the same per-side overlap
and factorisation-cost signals that the BFS lane already tracks?

## Scope

This pass stayed inside the requested lane:

- endpoint: open Brix-Ruiz `k=4` pair
- mode: `graph_plus_structured`
- bounded surface: `beam256 + dim4 + entry12`
- comparison corpus:
  `research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json`

Attempted lane-local change:

- only at same beam depth, let beam direction choice consult trained overlap and
  factorisation-cost signals instead of using only frontier-head ordering

The change was measured and then rejected; the worktree ends with the search
code reverted.

## Artifacts

Corpus:

- `research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json`

Fresh local run artifacts:

- before:
  `tmp/brix_ruiz_k4_graph_plus_structured_broad_beam_run_2026-04-18_before.json`
- after attempted retune:
  `tmp/brix_ruiz_k4_graph_plus_structured_broad_beam_run_2026-04-18_after_beam_direction_signal.json`

Reproduce:

```bash
timeout -k 20s 190s target/dist/research_harness \
  --cases research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_broad_beam_run_2026-04-18_before.json

timeout -k 20s 190s target/dist/research_harness \
  --cases research/brix_ruiz_k4_graph_plus_structured_broad_beam_corpus_2026-04-17.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_broad_beam_run_2026-04-18_after_beam_direction_signal.json
```

## Results

The retune produced **no measurable change**. The full before/after corpus was
bit-for-bit identical on the recorded telemetry summary fields that matter for
this bead:

- approximate hits
- visited volume
- frontier expansions
- factorisation volume
- terminal bottleneck
- witness status

Retained `beam256 + dim4 + entry12` surface:

| Case | Before approx. hits | After approx. hits | Before visited | After visited | Before factorisations | After factorisations |
| --- | --- | --- | --- | --- | --- | --- |
| `beam256 + lag20 + dim4 + entry12` | `126` | `126` | `112,521` | `112,521` | `441,907` | `441,907` |
| `beam256 + lag30 + dim4 + entry12` | `165` | `165` | `145,621` | `145,621` | `516,357` | `516,357` |
| `beam256 + lag40 + dim4 + entry12` | `184` | `184` | `176,664` | `176,664` | `592,101` | `592,101` |

Other corpus cases were also unchanged:

- `beam128 + lag20/30/40 + dim4 + entry12`: unchanged
- `beam512 + lag20 + dim4 + entry12`: unchanged
- `beam64 + lag12 + dim5 + entry10 + timeout30s`: unchanged

On the attempted hot case
`brix_ruiz_k4__graph_plus_structured__beam256_lag40_dim4_entry12`, even the
per-layer direction pattern and move-family approximate-hit totals were
unchanged between artifacts. That means this retune did not alter actual beam
frontier evolution on the Brix-Ruiz dim4 lane.

## Validation

Required library gate:

```bash
cargo test -p sse-core --lib -- --test-threads=1
```

Why single-threaded:

- the default parallel `cargo test -p sse-core --lib` hit existing
  `sqlite_graph` temp-file collisions in this worktree setup
- the single-threaded rerun passed cleanly and is the stable validation result

Formatter:

```bash
timeout 20s cargo fmt
```

Observed result:

- timed out without formatter output in this workmux setup
- no code was retained from the attempted retune, so the final worktree only
  keeps this note and the measurement artifacts

Focused tests:

- none retained; the temporary beam-direction unit test was removed when the
  code change was rejected

## Decision

Decision: **reject**

Reason:

- the attempted same-depth beam-direction signal retune is a strict no-op on
  the bounded Brix-Ruiz `k=4` graph-plus-structured corpus
- it does not improve approximate overlap, visited/frontier volume,
  factorisation volume, or witness/lag outcome on the retained
  `beam256 + dim4 + entry12` surface

Durable conclusion:

- this exact direction-choice slice can be ruled out for the open Brix-Ruiz
  `k=4` dim4 lane
- a follow-up bead should look elsewhere in the lane, because this particular
  frontier-ordering idea did not move the measurement surface at all
