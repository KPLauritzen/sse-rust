# Graph-plus-structured profiler-first hotspot round: duplicate-class signature micro-optimization rejected (2026-04-18)

## Scope

This was a **profiler-first move-path optimization round for `graph_plus_structured`** on the accepted `k=3` baseline surfaces:

- `brix_ruiz_k3_graph_plus_structured`
- `brix_ruiz_k3_graph_plus_structured_beam_probe`

It is intentionally **distinct from `sse-rust-2uy.31`**. That earlier bead was about
`k=4` ranking-quality tuning on a retained beam corpus and already ended with a retained no-op note.
This round stayed on the current `graph_plus_structured` hot path and allowed at most one bounded implementation attempt.

Hard boundaries held:

- no search-policy rewrite
- no `k=4` beam/ranking retune
- no move-family widening
- no generic mixed-search optimization

## Profiler evidence

Hotspot probe command on the accepted exact surface:

```bash
timeout -k 5s 40s target/debug/search 1,3,2,1 1,6,1,1 \
  --max-lag 8 \
  --max-intermediate-dim 4 \
  --max-entry 5 \
  --move-policy graph-plus-structured \
  --json --telemetry --pprof
```

The pprof stack signal stayed concentrated inside frontier expansion. The clearest
non-policy bookkeeping hotspot was:

- `same_future_past_signature`
- `duplicate_vector_classes_with_examples`

The same profile also still showed work in:

- `DynMatrix::canonical_perm_4x4`
- `visit_binary_sparse_factorisations_3x3_to_4`

That led to one bounded micro-optimization attempt only:

- replace the duplicate-class collection in `duplicate_vector_classes_with_examples`
  with a single hash map instead of two `BTreeMap`s, while preserving the final
  sorted class ordering

## Measurement setup

I used a clean baseline worktree at current `HEAD` before the patch:

- baseline worktree: `tmp/cy9-baseline`
- baseline commit: `d71df14`

I treated the pprof run as hotspot evidence only. Before/after timing used the
optimized `dist` profile so the comparison matched the accepted baseline lane.

### Exact-control runtime

Baseline command:

```bash
scripts/measure-search-runtime-rss.sh \
  --label cy9-baseline-gps-exact-dist \
  --runs 5 \
  --timeout 8s \
  -- /home/kasper/dev/sse-rust__worktrees/cy9-graph-plus-structured-profiler-round/target/dist/search \
    1,3,2,1 1,6,1,1 \
    --max-lag 8 \
    --max-intermediate-dim 4 \
    --max-entry 5 \
    --move-policy graph-plus-structured \
    --telemetry --json
```

After command:

```bash
scripts/measure-search-runtime-rss.sh \
  --label cy9-after-gps-exact-dist \
  --runs 5 \
  --timeout 8s \
  -- target/dist/search \
    1,3,2,1 1,6,1,1 \
    --max-lag 8 \
    --max-intermediate-dim 4 \
    --max-entry 5 \
    --move-policy graph-plus-structured \
    --telemetry --json
```

### Beam-control runtime

Baseline command:

```bash
scripts/measure-search-runtime-rss.sh \
  --label cy9-baseline-gps-beam-dist \
  --runs 5 \
  --timeout 5s \
  -- /home/kasper/dev/sse-rust__worktrees/cy9-graph-plus-structured-profiler-round/target/dist/research_harness \
    --cases research/cases.json \
    --format json \
    --worker-case brix_ruiz_k3_graph_plus_structured_beam_probe
```

After command:

```bash
scripts/measure-search-runtime-rss.sh \
  --label cy9-after-gps-beam-dist \
  --runs 5 \
  --timeout 5s \
  -- target/dist/research_harness \
    --cases research/cases.json \
    --format json \
    --worker-case brix_ruiz_k3_graph_plus_structured_beam_probe
```

### Counter-validation commands

```bash
timeout -k 5s 20s target/dist/research_harness \
  --cases research/cases.json \
  --format json \
  --worker-case brix_ruiz_k3_graph_plus_structured

timeout -k 5s 20s target/dist/research_harness \
  --cases research/cases.json \
  --format json \
  --worker-case brix_ruiz_k3_graph_plus_structured_beam_probe
```

The baseline worktree used the same commands with the shared `target/dist/*` binaries.

## Before / After

### Exact solve control

Direct `target/dist/search` runtime:

- before wall samples: `2.07 / 2.09 / 2.07 / 2.08 / 2.09 s`, median `2.08 s`
- after wall samples: `2.11 / 2.11 / 2.08 / 2.09 / 2.08 s`, median `2.09 s`
- delta: about `+0.01 s` (`+0.5%`)

Exact control outcomes and key counters were unchanged:

- outcome `equivalent`
- witness lag `8`
- `frontier_nodes_expanded = 84,875`
- `total_visited_nodes = 212,170`
- `factorisations_enumerated = 473,882`
- `approximate_other_side_hits = 1,237`

### Beam probe control

Direct `research_harness` runtime on the accepted beam probe:

- before wall samples: `0.09 / 0.07 / 0.07 / 0.07 / 0.07 s`, median `0.07 s`
- after wall samples: `0.07 / 0.07 / 0.07 / 0.06 / 0.07 s`, median `0.07 s`
- delta: no durable change

Beam probe outcomes and key counters were unchanged:

- outcome `unknown`
- `frontier_nodes_expanded = 142`
- `total_visited_nodes = 2,631`
- `factorisations_enumerated = 22,370`
- `approximate_other_side_hits = 10`

## Decision

Decision: **reject**

Reason:

- the attempted duplicate-class bookkeeping optimization did not improve the
  accepted exact `graph_plus_structured` control
- the exact control regressed slightly on median wall time
- the accepted beam control stayed flat
- outcomes and key counters stayed identical, so the change appears to be only
  a code-path reshuffle without a measured win

## Durable conclusion

The current `graph_plus_structured` hotspot signal around
`same_future_past_signature` / `duplicate_vector_classes_with_examples`
is real enough to inspect, but **this exact micro-optimization is not worth keeping**.

This bead therefore ends as an **evidence-only rejected round**:

- no retained search code change
- durable note kept
- accepted baseline surfaces revalidated on the same lane
