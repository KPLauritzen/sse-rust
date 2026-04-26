# Guided segment approximate-hit parity propagation (2026-04-26)

## Question

For bead `sse-rust-mvst`, extend the existing opt-in
`--approximate-hit-parity-report PATH` surface so nested guided-refinement and
shortcut-search segment searches no longer leave their inner
`approximate_other_side_hits` only as missing/unattributed top-level counts.

Hard boundaries for this slice:

- reporting only;
- no default beam ordering changes;
- no hard pruning, hard dedup, parity filtering, canonicalization, or SSE
  invariant claim; and
- no broad search beyond the bounded replay below.

## Propagated report shape

Kept shape:

- the CLI surface stays opt-in:
  `search --approximate-hit-parity-report PATH`;
- guided-refinement and shortcut-search now forward the optional observer into
  each nested endpoint-search segment attempt; and
- the report now carries explicit per-search-scope attribution instead of
  assuming one flat top-level surface.

New report structure:

- `search_scopes` records the observed search stack:
  - `search_scope_id`
  - `parent_search_scope_id`
  - `nesting_depth`
  - `stage`
  - endpoint/config summary for that scope
  - `inclusive_approximate_other_side_hits`
  - `exclusive_approximate_other_side_hits`
  - `child_approximate_other_side_hits`
  - `discovered_approximate_hit_records`
  - completeness fields for that scope
- each `annotated_hit` now points back to:
  - `search_scope_id`
  - `search_scope_stage`
  - `search_scope_nesting_depth`

Completeness semantics:

- top-level guided/shortcut scopes keep their inclusive telemetry, but their
  per-scope completeness is checked against
  `exclusive_approximate_other_side_hits`, not the inclusive parent total;
- child endpoint-search scopes compare directly against their own inclusive
  count because they have no nested children here; and
- the report summary still checks aggregate completeness against the final
  top-level telemetry total.

This keeps the earlier meaning of `report_is_complete`, but now makes the
parent/child accounting explicit instead of treating nested hits as missing by
construction.

## Validation

Formatting and focused tests:

```bash
timeout -k 20s 120s cargo fmt --all
timeout -k 20s 240s cargo test -q --lib forwards_observer_into_nested_segment_search
timeout -k 20s 240s cargo test -q --bin search approximate_hit_parity_report
```

Bounded `k = 3` shortcut replay:

```bash
timeout -k 20s 240s cargo run -q --bin search -- \
  1,3,2,1 1,6,1,1 \
  --stage shortcut-search \
  --guide-artifacts research/guide_artifacts/k3_exact_endpoint_multi_meet_retained_pool_2026-04-19.json \
  --max-intermediate-dim 4 \
  --max-entry 5 \
  --guided-max-shortcut-lag 4 \
  --guided-min-gap 2 \
  --guided-max-gap 6 \
  --guided-segment-timeout 5 \
  --guided-rounds 2 \
  --shortcut-max-guides 4 \
  --shortcut-rounds 2 \
  --shortcut-max-total-segment-attempts 64 \
  --approximate-hit-parity-report tmp/sse-rust-mvst-k3-shortcut-replay-approximate-hit-parity.json \
  --json --telemetry
```

Artifact:

- `tmp/sse-rust-mvst-k3-shortcut-replay-approximate-hit-parity.json`

## Observed result

Replay outcome:

- the bounded shortcut replay still returns the retained lag-`7` witness;
- default search behavior is unchanged; and
- the parity report is now complete on the replay that was previously partial.

Observed report summary:

- `telemetry_approximate_other_side_hits = 796`
- `discovered_approximate_hit_records = 796`
- `missing_approximate_hits = 0`
- `excess_annotated_hits = 0`
- `report_is_complete = true`
- `search_scopes_observed = 64`
- `nested_search_scopes_observed = 63`
- `complete_search_scopes = 64`
- `incomplete_search_scopes = 0`
- `supported_square_hits = 796`
- `hits_by_best_action = { rank_or_propose_inside_coarse_bucket: 789, reuse_endpoint_local_parity: 7 }`

Important scope-level reading:

- top-level scope `1` is the `shortcut_search` request itself:
  - `inclusive_approximate_other_side_hits = 796`
  - `exclusive_approximate_other_side_hits = 0`
  - `discovered_approximate_hit_records = 0`
  - `report_is_complete = true`
- the remaining `63` child scopes are nested `endpoint_search` segment runs;
- all `796` annotated hits are now attributed to those endpoint-search scopes;
  there are no unattributed leftovers; and
- the largest child scopes carried `135`, `99`, `98`, `98`, `94`, `94`, and
  `85` approximate hits respectively, all with per-scope completeness true.

Representative positive inner reuse hits:

- `7` annotated hits are now explicitly labeled
  `reuse_endpoint_local_parity`;
- they appear inside nested endpoint-search scopes
  `6`, `30`, `31`, and `51`; and
- a representative record is a backward `insplit` hit at layer `3` with
  coarse signature
  `d4|sum15|rs3,3,4,5|cs0,4,5,6|rS1,2,2,3|cS0,2,3,3`
  and trimmed signature
  `4x3|0,0,5,1,2,0,2,1,1,1,2,0`.

This is the missing capability from `sse-rust-bljn`: the replay no longer
reports nested hits only as absent top-level accounting drift.

## Keep / reject decision

Decision: **keep**.

Why it is useful enough to keep:

- it preserves the existing opt-in CLI surface and default solver behavior;
- it makes the previously incomplete shortcut replay report exact and
  attributable;
- it distinguishes parent stage accounting from child endpoint-search evidence
  cleanly enough to avoid false “missing hit” interpretations; and
- it surfaces a small but real `reuse_endpoint_local_parity` subset inside the
  bounded replay instead of hiding it behind top-level aggregation.

What this still does **not** do:

- it does not feed the signal into ranking or pruning;
- it does not claim the parity action is an SSE invariant; and
- it does not attempt to collapse or canonicalize hits across unrelated nested
  segment searches.

## Next integration boundary

The next useful boundary is not more propagation inside search. This slice
already reaches nested guided/shortcut endpoint-search segments.

The next boundary should instead be one of:

- consume the scoped report in downstream diagnostics without flattening child
  scopes back into one pool; or
- decide whether any higher-level summary should deliberately aggregate scoped
  `reuse` hits across segment searches.

Still do **not**:

- use the signal in default beam ordering;
- treat it as hard pruning, hard dedup, or parity filtering; or
- claim that scoped endpoint-local parity is a solver correctness condition.
