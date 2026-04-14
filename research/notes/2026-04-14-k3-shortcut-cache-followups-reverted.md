# k=3 shortcut cache follow-ups (reverted) — 2026-04-14

## Question

After landing exact segment-query caching, can two follow-up cache/budget semantics improve lag-search efficiency further?

Candidates tested:

1. charge segment-attempt budget only on cache misses,
2. add endpoint-level cache dominance reuse across lag caps.

## Context

Known-good baseline before these follow-ups:

- commit `166bb33` (`search: cache shortcut segment queries per run`)
- control config (`max_shortcut_lag=4`, `min_gap=2`, `max_gap=6`, attempts `128`) completed and produced lag `7`.

## Evidence

### Follow-up A: miss-only budget accounting

Change idea:

- cache hits should not consume `max_total_segment_attempts`,
- only expensive cache misses should consume budget.

Observed outcome:

- the standard control run at attempts `128` timed out at `300s` (exit `124`),
- no completed JSON artifact produced.

Decision:

- revert.

### Follow-up B: endpoint-level dominance reuse across lag caps

Measurement signal before implementation:

- in top-12 initial guide candidate enumeration (`min_gap=2,max_gap=6`):
  - total segment candidates: `595`
  - unique `(source,target,lag_cap)`: `514`
  - unique `(source,target)` endpoints: `499`
  - endpoint-only duplicate opportunity beyond exact-key duplicates: `15`

Change idea:

- reuse known `Unknown` at higher lag for lower-lag queries on same endpoints,
- reuse known `Equivalent` at lower lag for higher-lag queries on same endpoints.

Observed outcome:

- control run at attempts `128` timed out at `300s` (exit `124`) twice,
- no completed JSON artifact produced.

Decision:

- revert.

## Conclusion

Both follow-up semantic changes regressed practical runtime on the core control run and were reverted. The exact segment-query cache remains the current kept state.

## Next Steps

Prefer experiment-level search policy changes over cache-semantics changes for now, e.g. staged campaign strategy comparisons or richer segment admission ranking from measured utility, while keeping the exact query cache implementation intact.
