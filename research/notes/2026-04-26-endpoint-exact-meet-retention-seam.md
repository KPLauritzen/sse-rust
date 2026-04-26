# Endpoint exact-meet retention seam (2026-04-26)

## Slice

Kept one bounded maintainability seam in `src/search.rs`:

- extracted endpoint exact-meet retention ownership into
  `src/search/exact_meets.rs`;
- moved `ExactMeetRetention`, best-path selection, and retained-surface
  publication helpers out of the main search module;
- left endpoint executor call sites, telemetry schema, CLI JSON shape, and
  witness inventory behavior unchanged.

## Why this seam

The retention block was internally cohesive and already served multiple endpoint
executor paths without needing CLI ownership changes. Moving it reduced
`src/search.rs` surface area while keeping the refactor mechanical and avoiding
any change to retention ordering, timeout handling, or path reconstruction.

## Keep decision

Keep this seam.

It narrows one exact-meet responsibility inside the search hotspot without
widening into broader output or inventory refactors. A sharper follow-up from
here would be a separate CLI-side seam for witness inventory/export helpers, but
that should stay independent from search-loop ownership.
