# Endpoint start/root dispatch seam (2026-04-25)

## Slice

Kept one bounded maintainability seam in `src/search.rs`:

- extracted `dispatch::emit_started_and_roots`;
- replaced repeated `Started` plus two-root observer bundles in the dynamic
  entrypoint, the 2x2 entrypoint, and the beam-family endpoint executors;
- left search ordering, frontier admission, exact-meet shaping, and result
  semantics unchanged.

## Why this seam

The repeated observer start/root block was duplicated across several endpoint
executor starts with identical structure and only endpoint values changing. It
was a low-risk place to narrow edit surface without rewriting any loop logic.

## Keep decision

Keep this seam.

It reduces one class of copy-paste edits in the main search hotspot while
staying mechanical and small. A sharper next follow-up would be to extract a
similarly bounded finalization/result-shaping seam for exact-meet success paths,
but only if that can be done without widening into algorithm changes.
