# Brix-Ruiz `k=4` graph-plus-structured binary-sparse `3x3 -> 4` orbit-dedup was a no-op (2026-04-19)

## Question

On the retained open Brix-Ruiz `k=4` `graph_plus_structured` lane
(`beam256 + dim4 + entry12`), is the remaining
`binary_sparse_rectangular_factorisation_3x3_to_4` hotspot spending large
budget on family-preserving duplicate witnesses that can be removed earlier
without changing frontier behaviour?

## Hypothesis

One fresh bounded hypothesis was tested:

- the retained dim4 lane still pays heavily for
  `binary_sparse_rectangular_factorisation_3x3_to_4`
- the family already has an exact orbit key
  (`binary_sparse_factorisation_3x3_to_4_orbit_key`)
- if many raw callbacks are duplicate witness-orbits, deduplicating those
  orbits inside the family enumerator should reduce factorisation volume while
  preserving the same frontier/ranking surface

Why this looked plausible on the retained lane baseline:

- `beam256 + lag40 + dim4 + entry12` baseline telemetry showed
  `binary_sparse_rectangular_factorisation_3x3_to_4 = 202,296 generated`
  for only `2,915 kept`, `1,930 discovered`, and `2 approximate hits`
- that made it the single largest remaining family-level factorisation hotspot
  on the kept dim4 surface after the earlier explicit split-family cut

## Attempted Slice

Temporary implementation slice:

- add an internal orbit-dedup gate inside
  `enumerate_binary_sparse_factorisation_3x3_to_4_family`
- use `binary_sparse_factorisation_3x3_to_4_orbit_key` to suppress duplicate
  family-preserving witness presentations before frontier expansion sees them

This code was measured and then reverted. The final worktree keeps only this
note and the measurement artifacts.

## Measurement Surface

Lane and bounds:

- endpoint: open Brix-Ruiz `k=4`
- mode: `graph_plus_structured`
- retained surface: `beam256 + lag40 + dim4 + entry12`
- bounded single-case corpus:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case.json`

Local run artifacts:

- baseline:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_probe.json`
- after temporary orbit-dedup patch:
  `tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_binary_sparse_orbit_dedup.json`

Reproduce:

```bash
cargo build --bin research_harness --features research-tools

timeout -k 20s 40s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_probe.json

# Apply the temporary orbit-dedup patch, then rerun:
timeout -k 20s 40s target/debug/research_harness \
  --cases tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_single_case.json \
  --format json \
  > tmp/brix_ruiz_k4_graph_plus_structured_beam256_lag40_dim4_entry12_after_binary_sparse_orbit_dedup.json
```

## Results

The retained-lane measurement was a strict no-op on the fields that matter for
this bead.

| Field | Before | After |
| --- | --- | --- |
| outcome | `unknown` | `unknown` |
| elapsed | `23866 ms` | `23746 ms` |
| frontier expanded | `19,970` | `19,970` |
| factorisations | `493,407` | `493,407` |
| candidates after pruning | `271,803` | `271,803` |
| discovered nodes | `176,662` | `176,662` |
| approximate hits | `184` | `184` |
| visited | `176,664` | `176,664` |
| terminal bottleneck | `factorisation_volume` | `factorisation_volume` |
| focus progress score | `87,050,000` | `87,050,000` |
| directed progress score | `19,783,000` | `19,783,000` |

The hotspot family itself was also unchanged on retained-lane telemetry:

| Family | Before generated | After generated | Before kept | After kept | Before approx. hits | After approx. hits |
| --- | --- | --- | --- | --- | --- | --- |
| `binary_sparse_rectangular_factorisation_3x3_to_4` | `202,296` | `202,296` | `2,915` | `2,915` | `2` | `2` |

Interpretation:

- the temporary orbit-dedup patch does collapse duplicate orbit-equivalent
  witnesses on a focused unit-test matrix
- but those duplicate witness presentations are not what dominates the retained
  open Brix-Ruiz dim4 lane
- on the actual `beam256 + lag40 + dim4 + entry12` surface, the search evolves
  identically before and after the attempted patch

## Validation

Focused commands run during the temporary slice:

```bash
timeout -k 20s 180s cargo test \
  test_binary_sparse_factorisations_reach_baker_step_2 \
  -- --test-threads=1
```

Observed result:

- passed during the temporary patch, confirming the family still exposed the
  retained Baker witness used as a focused correctness check

Formatter:

```bash
timeout -k 20s 120s cargo fmt --all
```

## Decision

Decision: **reject**

Reason:

- this exact hotspot hypothesis does not improve the retained open Brix-Ruiz
  `k=4` dim4 `graph_plus_structured` surface at all
- the measured before/after lane telemetry is unchanged, including the hotspot
  family's own generated/kept counts

Durable conclusion:

- internal orbit-dedup for
  `binary_sparse_rectangular_factorisation_3x3_to_4` is not the right next
  spend-better lever for the retained `beam256 + dim4 + entry12` lane
- the next fresh hotspot hypothesis should look elsewhere, likely at a
  different family or a ranking-quality seam rather than this particular
  duplicate-witness explanation
