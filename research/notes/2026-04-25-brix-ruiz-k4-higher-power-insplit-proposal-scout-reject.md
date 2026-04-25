# Brix-Ruiz `k=4` higher-power in-split proposal scout rejects on the retained rank-4 pair (2026-04-25)

## Question

For bead `sse-rust-nw7.3`, can one tiny retained-lane-adjacent
complete-in-split / higher-power proposal slice beat a blind shortlist on the
open Brix-Ruiz `k=4` graph-plus-structured lane without reopening generic
split widening?

This round stayed deliberately narrow:

- no default solver behavior change;
- no generic split-sidecar closure or widening pass;
- no reopening of blind one-step or two-step mixed split refinement; and
- no full retained-lane rerank, only a fixed scout surface extracted from the
  retained lane's best structured approximate-hit evidence.

## Hypothesis

Use the smallest higher-power proxy, `M^2`, as a compressed `2`-block shadow of
one-step same-future in-split proposals.

Concrete slice:

1. Start from the retained rank-4 diagonal approximate-hit forward-side `4x4`
   state recorded in
   [`2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md`](2026-04-25-brix-ruiz-k4-graph-plus-structured-stuck-state-inventory.md).
2. Enumerate only one-step same-future in-splits from that state.
3. Canonicalize and deduplicate them.
4. Compare two shortlist strategies over that fixed candidate universe:
   - blind baseline: coarse same-future/past gap to the retained opposite-side
     counterpart;
   - higher-power candidate: coarse same-future/past gap on `proposal^2`
     versus `target^2`, with partition-refined power gap as the tie-break.
5. Run bounded graph-only realization checks from each shortlisted proposal to
   the retained opposite-side counterpart.

This is **not** blind split widening:

- the widened `5x5` states are not injected into the main solver;
- the experiment never closes under repeated splits;
- only a tiny top-k shortlist is realized; and
- the ranking signal comes from a compressed higher-power shadow, not from raw
  split volume.

## Fixed Scout Surface

Source: retained rank-4 diagonal approximate pair from the stuck-state note.

- current:
  `[[1,4,2,7],[3,1,0,6],[0,0,0,0],[0,0,0,0]]`
- opposite-side retained counterpart:
  `[[1,12,0,1],[1,1,4,4],[0,0,0,0],[0,0,0,0]]`

Scout realization cap:

- `graph_only`
- `bfs`
- `max_intermediate_dim = 5`
- `max_entry = 12`
- main measured round: `max_lag = 3`

## Implementation

New bounded research-only sidecar:

- [`src/bin/evaluate_brix_ruiz_k4_higher_power_insplit_proposals.rs`](../../src/bin/evaluate_brix_ruiz_k4_higher_power_insplit_proposals.rs)

The binary:

- materializes the retained scout pair above;
- enumerates canonical one-step same-future in-splits;
- scores them both by direct target gap and by squared-matrix target gap; and
- runs bounded graph-only realization probes on each shortlist.

## Commands And Raw Artifacts

Validation:

```bash
timeout -k 20s 180s cargo test --features research-tools --bin evaluate_brix_ruiz_k4_higher_power_insplit_proposals
timeout -k 20s 180s cargo build --features research-tools --bin evaluate_brix_ruiz_k4_higher_power_insplit_proposals
```

Small safety pass first:

```bash
timeout -k 20s 60s target/debug/evaluate_brix_ruiz_k4_higher_power_insplit_proposals \
  --shortlist-size 4 \
  --probe-lag 2 \
  --json-out tmp/brix_ruiz_k4_higher_power_insplit_proposals_rank4_scout_lag2_2026-04-25_nw7_3.json
```

Measured round:

```bash
timeout -k 20s 90s target/debug/evaluate_brix_ruiz_k4_higher_power_insplit_proposals \
  --shortlist-size 6 \
  --probe-lag 3 \
  --json-out tmp/brix_ruiz_k4_higher_power_insplit_proposals_rank4_lag3_2026-04-25_nw7_3.json
```

Compact summary extraction:

```bash
python - <<'PY'
import json
from pathlib import Path
path = Path('tmp/brix_ruiz_k4_higher_power_insplit_proposals_rank4_lag3_2026-04-25_nw7_3.json')
report = json.loads(path.read_text())
blind = report['blind_strategy']['proposals']
power = report['higher_power_strategy']['proposals']
blind_set = {tuple(p['matrix']['data']) for p in blind}
power_set = {tuple(p['matrix']['data']) for p in power}
out = Path('tmp/brix_ruiz_k4_higher_power_insplit_proposals_rank4_summary_2026-04-25_nw7_3.tsv')
rows = [
    ['strategy','shortlist','admitted','equivalent','approx_hits','max_frontier','max_visited','elapsed_ms','overlap_with_other'],
    ['blind', str(report['blind_strategy']['shortlist_size']), str(report['blind_strategy']['admitted_count']), str(report['blind_strategy']['equivalent_count']), str(report['blind_strategy']['approximate_hit_count']), str(report['blind_strategy']['max_frontier_nodes_expanded']), str(report['blind_strategy']['max_total_visited_nodes']), str(report['blind_strategy']['total_elapsed_ms']), str(len(blind_set & power_set))],
    ['higher_power', str(report['higher_power_strategy']['shortlist_size']), str(report['higher_power_strategy']['admitted_count']), str(report['higher_power_strategy']['equivalent_count']), str(report['higher_power_strategy']['approximate_hit_count']), str(report['higher_power_strategy']['max_frontier_nodes_expanded']), str(report['higher_power_strategy']['max_total_visited_nodes']), str(report['higher_power_strategy']['total_elapsed_ms']), str(len(blind_set & power_set))],
]
out.write_text('\n'.join('\t'.join(r) for r in rows) + '\n')
print(out)
PY
```

Artifacts:

- `tmp/brix_ruiz_k4_higher_power_insplit_proposals_rank4_scout_lag2_2026-04-25_nw7_3.json`
- `tmp/brix_ruiz_k4_higher_power_insplit_proposals_rank4_lag3_2026-04-25_nw7_3.json`
- `tmp/brix_ruiz_k4_higher_power_insplit_proposals_rank4_summary_2026-04-25_nw7_3.tsv`

## Bounded Evidence

Candidate universe:

- raw one-step same-future in-splits: `4620`
- canonical proposals after dedup: `38`

Measured `lag = 3` A/B on the same `38`-proposal universe:

| Strategy | Shortlist | Survive admission | Exact meets | Approx. hits | Max frontier | Max visited | Elapsed |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| blind coarse-gap | `6` | `6` | `0` | `0` | `6` | `846` | `78 ms` |
| higher-power `M^2` gap | `6` | `6` | `0` | `0` | `6` | `859` | `72 ms` |

Additional read:

- the two shortlists overlap in only `1` proposal, so the higher-power signal
  does materially reorder the candidate set;
- despite that reorder, the higher-power shortlist does not produce a better
  bounded realization result;
- all shortlisted proposals survive admission, so the failure is not
  arithmetic-screen rejection;
- the higher-power shortlist also fails the budget-side tie-break, because its
  worst-case visited count is slightly higher (`859` vs `846`).

## Decision

Decision: **reject for now on this scout surface.**

Why:

- the higher-power ranking did change the shortlist, so the slice is not a
  tautology;
- but it produced no exact meet, no approximate hit, and no frontier/visited
  improvement under the fixed scout cap;
- the only positive movement was a small elapsed-time difference (`72 ms` vs
  `78 ms`), which is not enough to justify a follow-up by itself.

## Follow-Up

No follow-up bead opened.

Rationale:

- this bounded slice already answered the narrow question it asked;
- the measured scout surface is negative without ambiguity; and
- opening another bead would just restate "try a different higher-power score"
  without new evidence that this family deserves more retained-lane budget.
