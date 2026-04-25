# Depth-one move-language exposure candidate (2026-04-25)

## Question

For bead `sse-rust-5lz`, test one original computable obstruction candidate
that is not a standard endpoint invariant and is not the finite-depth
support-shadow signature from `sse-rust-5lz.1`.

The candidate in this note is intentionally small: one depth, one move-language
profile, and a keep/reject decision.

## Candidate Definition

Fix an envelope:

- endpoint canonicalization by simultaneous row/column permutation;
- maximum intermediate dimension `D`;
- factor-entry cap `E`;
- move vocabulary `P`, using the families selected by
  `visit_factorisations_with_family_for_policy`; and
- local size/spectrum pruning as used by the endpoint search frontier.

For one endpoint matrix `A`, define the depth-one move-language exposure
profile

```text
L_{D,E,P}(A)[f] = (
  generated candidates in move family f from canonical(A),
  candidates in move family f surviving local pruning
)
```

for each enabled move family `f`.

The pair obstruction candidate is:

```text
if L_{D,E,P}(A) != L_{D,E,P}(B), then A and B are not SSE.
```

The implemented search telemetry already computes this profile at the first
expanded layer:

```text
telemetry.layers[0].move_family_telemetry.<family>.candidates_generated
telemetry.layers[0].move_family_telemetry.<family>.candidates_after_pruning
```

The `discovered_nodes` and `exact_meets` fields were recorded in scratch
outputs but are not part of the candidate, because they depend on the opposite
endpoint already being in the bidirectional seen set.

## Why This Is Not A Known Invariant

This is a move-language/enumeration profile, not a determinant, trace,
spectrum, Bowen-Franks group, dimension group, `GL(2,Z)` class,
concrete-shift certificate, ideal-class profile, or arithmetic similarity
test.

It is also distinct from the finite-depth support-shadow signature:

- it does not project reached matrices to support/mass keys;
- it does not compute source/target reachable shadow overlap;
- it keeps only per-family local expansion counts at depth one; and
- it is explicitly tied to the implemented move vocabulary and pruning surface.

This dependence is exactly why it had to be tested as an obstruction candidate
rather than assumed to be invariant.

## Commands And Artifacts

Scratch artifacts are under `tmp/`:

- `tmp/sse-rust-5lz-move-language-rect-positive-search.json`
- `tmp/sse-rust-5lz-move-language-rect-positive-a.json`
- `tmp/sse-rust-5lz-move-language-rect-positive-b.json`
- `tmp/sse-rust-5lz-move-language-perm-relabeled-source.json`
- `tmp/sse-rust-5lz-move-language-k3-a.json`
- `tmp/sse-rust-5lz-move-language-k3-b.json`
- `tmp/sse-rust-5lz-move-language-k4-a.json`
- `tmp/sse-rust-5lz-move-language-k4-b.json`
- `tmp/sse-rust-5lz-move-language-eilers-kiming-a.json`
- `tmp/sse-rust-5lz-move-language-exposure-summary.tsv`

Equivalent control:

```bash
timeout -k 20s 120s cargo run --quiet --bin search -- \
  3,4,3,4 4,4,3,3 \
  --max-lag 4 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy mixed \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-rect-positive-search.json
```

Depth-one exposure profiles:

```bash
timeout -k 20s 120s cargo run --quiet --bin search -- \
  3,4,3,4 4,4,3,3 \
  --max-lag 1 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy mixed \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-rect-positive-a.json

timeout -k 20s 120s cargo run --quiet --bin search -- \
  4,4,3,3 3,4,3,4 \
  --max-lag 1 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy mixed \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-rect-positive-b.json

timeout -k 20s 120s cargo run --quiet --bin search -- \
  1,2,3,1 1,6,1,1 \
  --max-lag 1 \
  --max-intermediate-dim 4 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-perm-relabeled-source.json

timeout -k 20s 120s cargo run --quiet --bin search -- \
  1,3,2,1 1,6,1,1 \
  --max-lag 1 \
  --max-intermediate-dim 4 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-k3-a.json

timeout -k 20s 120s cargo run --quiet --bin search -- \
  1,6,1,1 1,3,2,1 \
  --max-lag 1 \
  --max-intermediate-dim 4 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-k3-b.json

timeout -k 20s 120s cargo run --quiet --bin search -- \
  1,4,3,1 1,12,1,1 \
  --max-lag 1 \
  --max-intermediate-dim 4 \
  --max-entry 12 \
  --move-policy graph-plus-structured \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-k4-a.json

timeout -k 20s 120s cargo run --quiet --bin search -- \
  1,12,1,1 1,4,3,1 \
  --max-lag 1 \
  --max-intermediate-dim 4 \
  --max-entry 12 \
  --move-policy graph-plus-structured \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-k4-b.json
```

Known non-SSE control attempt:

```bash
timeout -k 20s 120s cargo run --quiet --bin search -- \
  14,2,1,0 13,5,3,1 \
  --max-lag 1 \
  --max-intermediate-dim 2 \
  --max-entry 15 \
  --move-policy mixed \
  --json --telemetry \
  > tmp/sse-rust-5lz-move-language-eilers-kiming-a.json
```

Summary extraction:

```bash
for spec in \
  rect_A:tmp/sse-rust-5lz-move-language-rect-positive-a.json \
  rect_B:tmp/sse-rust-5lz-move-language-rect-positive-b.json \
  k3_A:tmp/sse-rust-5lz-move-language-k3-a.json \
  k3_B:tmp/sse-rust-5lz-move-language-k3-b.json \
  k3_perm_A:tmp/sse-rust-5lz-move-language-perm-relabeled-source.json \
  k4_A:tmp/sse-rust-5lz-move-language-k4-a.json \
  k4_B:tmp/sse-rust-5lz-move-language-k4-b.json \
  eilers_kiming:tmp/sse-rust-5lz-move-language-eilers-kiming-a.json
do
  label=${spec%%:*}
  file=${spec#*:}
  jq -r --arg label "$label" '
    if (.telemetry.layers | length) == 0 then
      [$label,"<no expansion>",0,0,0,0]
    else
      .telemetry.layers[0].move_family_telemetry
      | to_entries[]
      | [$label,.key,.value.candidates_generated,.value.candidates_after_pruning,.value.discovered_nodes,.value.exact_meets]
    end
    | @tsv
  ' "$file"
done > tmp/sse-rust-5lz-move-language-exposure-summary.tsv
```

## Results

The positive rectangular control is SSE under the same `mixed / dim3 / entry6`
surface:

- outcome: `equivalent`
- witness: one elementary step with
  `U = [[1,1],[1,1]]` and `V = [[2,2],[1,2]]`
- frontier nodes expanded: `2`
- factorisations enumerated: `51637`

The candidate profile nevertheless separates the two equivalent endpoints:

| Case | Family | Generated | After pruning |
| --- | --- | ---: | ---: |
| rect `A` | `insplit` | `21` | `20` |
| rect `B` | `insplit` | `20` | `20` |
| rect `A` | `outsplit` | `20` | `20` |
| rect `B` | `outsplit` | `21` | `20` |
| rect `A` | `rectangular_factorisation_2x3` | `21604` | `3555` |
| rect `B` | `rectangular_factorisation_2x3` | `29923` | `3335` |
| rect `A` | `square_factorisation_2x2` | `43` | `31` |
| rect `B` | `square_factorisation_2x2` | `67` | `23` |

This is already a decisive false obstruction.

The permutation control passed at this depth. Relabeling
`[[1,3],[2,1]]` to `[[1,2],[3,1]]` and using the same hard target produced the
same profile:

| Case | Family | Generated | After pruning |
| --- | --- | ---: | ---: |
| k3 `A` | `insplit` | `7` | `7` |
| relabeled k3 `A` | `insplit` | `7` | `7` |
| k3 `A` | `outsplit` | `7` | `7` |
| relabeled k3 `A` | `outsplit` | `7` | `7` |
| k3 `A` | `rectangular_factorisation_2x3` | `954` | `189` |
| relabeled k3 `A` | `rectangular_factorisation_2x3` | `954` | `189` |
| k3 `A` | `square_factorisation_2x2` | `2` | `1` |
| relabeled k3 `A` | `square_factorisation_2x2` | `2` | `1` |

The Eilers-Kiming no-go control was not usable through this telemetry front
door: the existing ideal-class invariant fires before any expansion:

```text
outcome = not_equivalent
reason = Eilers-Kiming ideal class mismatch
invariant_filtered = true
layers = []
```

Hard Brix-Ruiz lanes also separate under the candidate:

| Case | Family | Generated | After pruning |
| --- | --- | ---: | ---: |
| k3 `A` | `rectangular_factorisation_2x3` | `954` | `189` |
| k3 `B` | `rectangular_factorisation_2x3` | `1925` | `396` |
| k4 `A` | `rectangular_factorisation_2x3` | `2965` | `670` |
| k4 `B` | `rectangular_factorisation_2x3` | `6107` | `1390` |

For `k=3`, this is another false obstruction because the repo already has
lag-7 witnesses. For `k=4`, the separation is not interpretable after the
positive-control failure.

## Keep Or Reject

Reject as an SSE invariant or obstruction.

The signal is computable and permutation-stable in the tested relabeling
control, but it fails the known equivalent rectangular control and would also
falsely obstruct the solved Brix-Ruiz `k=3` lane. The failure mode is useful:
the implemented move vocabulary has endpoint-local branching asymmetry even
inside one elementary SSE class, so raw local move-language exposure is a
search-telemetry feature rather than an invariant.

No follow-up bead is justified. A reusable checker would only automate a signal
that already fails the acceptance controls.

## Validation

No code was added. Validation consists of the focused commands above and the
required formatting gate:

```bash
timeout -k 20s 120s cargo fmt --all
```
