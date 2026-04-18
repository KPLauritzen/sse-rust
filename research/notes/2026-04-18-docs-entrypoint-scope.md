# Docs Entrypoint Scope

## Question

Should this pass rename or replace `docs/TODO.md`, or broadly reshuffle the
topic notes under `docs/`?

## Context

The task is a bounded docs-entrypoint refresh, not a repo-wide docs rewrite.
The current confusion is mostly about authority and starting points rather than
missing files.

## Evidence

- `bd` already owns live work tracking.
- `docs/TODO.md` is linked from the top-level README and the docs map.
- The existing topic notes already cover distinct durable subjects; the main
  issue is that the entrypoint language does not consistently explain the
  split.

## Conclusion

Keep `docs/TODO.md` at its current path for continuity, but reframe it
explicitly as durable context under a historical filename. Do not do broader
content moves in this slice.

## Next Steps

- Keep future docs-structure passes focused on authority and navigation first.
- Only rename or merge topic notes when there is a clear payoff beyond wording
  cleanup.
