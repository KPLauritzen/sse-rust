# Finite-depth support-shadow signature (2026-04-25)

## Question

For bead `sse-rust-5lz.1`, test one computable finite-depth obstruction
signature for SSE candidate pairs.

This is intentionally weaker than a true invariant. The goal is to see whether
a bounded reachable shadow can be useful as a search-guidance signal after
surviving basic known-equivalence and relabeling controls.

## Signature Definition

Fix an envelope:

- one-sided depth `N`;
- maximum intermediate square dimension `D`;
- factor-entry cap `E`;
- move vocabulary `P`, using the factorisation families selected by
  `visit_factorisations_with_family_for_policy`; and
- simultaneous row/column permutation canonicalization.

For an endpoint matrix `A`, compute exact one-sided BFS layers
`R_d(A)` for `0 <= d <= N`. Each successor is `VU` from a visited state
`UV`, canonicalized by `canonical_perm`, and deduplicated by first depth.

Project each reached canonical matrix `M` to its sorted support-shadow key:

```text
(
  dim(M),
  total entry sum,
  nonzero support count,
  sorted row sums,
  sorted column sums,
  sorted row support counts,
  sorted column support counts
)
```

For a pair `(A, B)`, the diagnostic reports:

- exact canonical overlap between `R_i(A)` and `R_j(B)`;
- shadow-key overlap between their projections;
- minimum exact and projected bridge depths `i + j`;
- whether bridge and overlap summaries are complete or based on a truncated
  shadow;
- per-layer source/target count asymmetry; and
- family-level candidate/discovery/collision counts.

The durable implementation is
[`src/bin/finite_depth_shadow_signature.rs`](../../src/bin/finite_depth_shadow_signature.rs).

## Why This Is Not A Known Invariant

This signature is path-envelope dependent: changing `N`, `D`, `E`, or the move
vocabulary changes the value. It is not determinant, trace, spectrum,
Bowen-Franks, dimension-group, GL(2,Z), concrete-shift, or the existing 2x2
ideal-class check.

It is also deliberately weaker than canonical state reachability. The
support-shadow projection can collide for matrices that are not the same
canonical matrix. Therefore a shadow overlap is only a search hint, and shadow
separation is only a bounded diagnostic unless a separate theorem is supplied.

## Commands And Artifacts

Build and test:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 180s cargo build --features research-tools --bin finite_depth_shadow_signature
timeout -k 20s 180s cargo test --features research-tools --bin finite_depth_shadow_signature
```

Controls and hard candidates:

```bash
timeout -k 20s 120s target/debug/finite_depth_shadow_signature \
  --case-id rectangular_positive_pair_depth1 \
  --source 3,4,3,4 \
  --target 4,4,3,3 \
  --max-depth 1 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json-out tmp/sse-rust-5lz1-rectangular-positive-depth1.json

timeout -k 20s 120s target/debug/finite_depth_shadow_signature \
  --case-id permutation_control_brix_ruiz_k3_source \
  --source 1,3,2,1 \
  --target 1,2,3,1 \
  --max-depth 1 \
  --max-intermediate-dim 3 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json-out tmp/sse-rust-5lz1-permutation-control-depth1.json

timeout -k 20s 120s target/debug/finite_depth_shadow_signature \
  --case-id eilers_kiming_14_2_no_go_depth1 \
  --source 14,2,1,0 \
  --target 13,5,3,1 \
  --max-depth 1 \
  --max-intermediate-dim 2 \
  --max-entry 15 \
  --move-policy mixed \
  --json-out tmp/sse-rust-5lz1-eilers-kiming-no-go-depth1.json

timeout -k 20s 120s target/debug/finite_depth_shadow_signature \
  --case-id brix_ruiz_k3_depth1 \
  --source 1,3,2,1 \
  --target 1,6,1,1 \
  --max-depth 1 \
  --max-intermediate-dim 4 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json-out tmp/sse-rust-5lz1-brix-ruiz-k3-depth1.json

timeout -k 20s 120s target/debug/finite_depth_shadow_signature \
  --case-id brix_ruiz_k3_depth2 \
  --source 1,3,2,1 \
  --target 1,6,1,1 \
  --max-depth 2 \
  --max-intermediate-dim 4 \
  --max-entry 6 \
  --move-policy graph-plus-structured \
  --json-out tmp/sse-rust-5lz1-brix-ruiz-k3-depth2.json

timeout -k 20s 120s target/debug/finite_depth_shadow_signature \
  --case-id brix_ruiz_k4_depth1 \
  --source 1,4,3,1 \
  --target 1,12,1,1 \
  --max-depth 1 \
  --max-intermediate-dim 4 \
  --max-entry 12 \
  --move-policy graph-plus-structured \
  --json-out tmp/sse-rust-5lz1-brix-ruiz-k4-depth1.json
```

Derived tables:

- `tmp/sse-rust-5lz1-shadow-summary.tsv`
- `tmp/sse-rust-5lz1-brix-ruiz-k3-depth2-layers.tsv`
- `tmp/sse-rust-5lz1-brix-ruiz-k3-depth2-families.tsv`
- `tmp/sse-rust-5lz1-brix-ruiz-k4-depth1-layers.tsv`

## Results

Summary:

| Case | Depth | Policy | Source states | Target states | Source shadows | Target shadows | Exact bridge | Shadow bridge | Exact overlap | Shadow overlap |
| --- | ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| rectangular positive control | `1` | graph_plus_structured | `3607` | `3398` | `3445` | `3205` | `1` | `1` | `296` | `374` |
| permutation control | `1` | graph_plus_structured | `204` | `204` | `181` | `181` | `0` | `0` | `204` | `181` |
| Eilers-Kiming non-SSE control | `1` | mixed | `21` | `10` | `21` | `10` | none | none | `0` | `0` |
| Brix-Ruiz `k=3` | `1` | graph_plus_structured | `204` | `415` | `181` | `414` | none | `2` | `0` | `2` |
| Brix-Ruiz `k=3` | `2` | graph_plus_structured | `1335` | `3342` | `1158` | `3242` | none | `2` | `0` | `16` |
| Brix-Ruiz `k=4` | `1` | graph_plus_structured | `686` | `1415` | `567` | `1411` | none | none | `0` | `0` |

No run truncated under the stated `max_states_per_side = 200000` guard.

### Controls

The known positive rectangular control passes: the exact canonical shadows meet
at bridge depth `1`, and the projected shadow also meets at bridge depth `1`.

The permutation control passes: `[[1,3],[2,1]]` and its row/column relabeling
`[[1,2],[3,1]]` canonicalize to the same endpoint, so exact and projected
bridges both occur at depth `0`. The full depth-1 shadow counts also match.

The Eilers-Kiming non-SSE control remains separated at depth `1` under
`mixed / dim2 / entry15`. This is not new evidence for non-SSE, because the
known ideal-class invariant already separates the pair. It is only a sanity
control showing that the shadow diagnostic does not collapse every
classical-invariant-matching pair at depth `1`.

### Hard Candidates

For Brix-Ruiz `k=3`, the exact canonical shadows do not meet through depth `2`,
but the projected support-shadow already overlaps:

- depth-1 total shadow overlap: `2`;
- depth-2 total shadow overlap: `16`;
- minimum projected bridge depth remains `2`.

The depth-2 layer comparison is:

| Depth | Source states | Target states | Source shadows | Target shadows | Exact overlap | Shadow overlap |
| ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `0` | `1` | `1` | `1` | `1` | `0` | `0` |
| `1` | `203` | `414` | `180` | `413` | `0` | `2` |
| `2` | `1131` | `2927` | `1011` | `2841` | `0` | `12` |

This matches the qualitative lag-7 notes: there is no obvious canonical
waypoint bottleneck, but low-depth aggregate profiles can touch before exact
states meet. The target side has a substantially larger local shadow even under
the same envelope.

For Brix-Ruiz `k=4`, the depth-1 shadow is more separated:

- no exact canonical overlap;
- no projected support-shadow overlap;
- source/target state counts `686` vs `1415`;
- source/target shadow-key counts `567` vs `1411`.

This is too shallow to call an obstruction, but it is a useful ranking contrast:
the open `k=4` lane has no immediate aggregate-profile contact under the same
small local diagnostic, while solved `k=3` has projected contact by total
bridge depth `2`.

## Keep Or Reject

Keep this as a bounded diagnostic and search-guidance signal, not as an SSE
invariant.

Reasons to keep:

- it survives the positive and permutation controls;
- it is cheap at the tested bounds;
- it distinguishes exact canonical meeting from weaker projected contact; and
- it gives a concrete contrast between solved `k=3` and open `k=4` Brix-Ruiz
  lanes under small graph-plus-structured shadows.

Reasons not to promote it:

- it is explicitly envelope-dependent;
- projected overlaps can be false positives;
- projected non-overlap is not a theorem-level obstruction; and
- the no-go control result is weaker than the existing ideal-class screen.

Decision: keep the binary and note as a reusable diagnostic surface. Do not
open a follow-up bead yet. A follow-up is only justified after this shadow score
is tested across a small retained corpus and shown to rank successful or
near-miss lanes better than existing aggregate telemetry.
