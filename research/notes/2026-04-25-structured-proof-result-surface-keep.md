# Structured-proof result surface keep (2026-04-25)

Kept the bounded `sse-rust-cpd5` slice as a result-surface abstraction only:

- introduced `StructuredProofResult` as the shared structured-proof payload at
  the `SearchRunResult` boundary;
- kept concrete-shift as the only implemented structured proof via
  `StructuredProofResult::ConcreteShift2x2`;
- preserved the existing external concrete-shift reporting surface:
  `equivalent_by_concrete_shift` remains the outcome label and relation details
  remain present in CLI JSON and sqlite `result_json`.

Why this was kept narrow:

- it removes the direct `ConcreteShiftProof2x2` leak from shared
  request/result/event/persistence plumbing without changing proof search
  semantics;
- it does not broaden dispatch policy, invariant filtering, or any theorem-bound
  `2x2` proof family;
- future Goal 4 same-dimension square proof families can now extend the shared
  boundary without adding another proof-type-specific `SearchRunResult` variant.

Validation note:

- generated `research/runs/20260425T233009Z.json` via
  `just research-json-save 20260425T233009Z`;
- no earlier `research/runs/` artifact existed in this worktree, so there was
  nothing local to diff against.
