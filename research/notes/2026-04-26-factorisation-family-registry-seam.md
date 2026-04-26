# Factorisation family registry seam (2026-04-26)

## Slice

Kept one bounded maintainability seam in `src/factorisation.rs`:

- extracted factorisation family descriptor ownership, enablement predicates,
  stable family ordering, and enabled-family traversal into
  `src/factorisation/families.rs`;
- kept all enumerator arithmetic and the family-specific enumeration entrypoints
  in `src/factorisation.rs`;
- left family labels, `MoveFamilyPolicy` gating, dispatcher exposure, and test
  coverage behavior unchanged.

## Why this seam

The registry block was internally cohesive and only depended on the local
enumeration wrappers by function pointer. That made it a low-risk place to cut
module ownership without rewriting any factorisation generation logic or
widening the public dispatcher surface.

## Keep decision

Keep this seam.

It removes the policy and registry table from the main arithmetic-heavy module
while preserving the existing family semantics and ordering contracts. A sharp
next follow-up would be to extract one dimension-specific wrapper cluster, such
as the `3x3` same-dimension family entrypoints, only if that can stay as
mechanical as this registry split.
