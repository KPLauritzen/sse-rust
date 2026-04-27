# Concrete-shift lower-bound semantics (2026-04-27)

## Question

When can a failed low-lag concrete-shift search be treated as a true lower
bound on SSE lag, and how should that differ from bounded heuristic or
telemetry use?

## Theorem-grade lower-bound conditions

The safe theorem-level rule is conditional:

> If finite essential square matrices `A` and `B` have an SSE witness of lag
> `m`, then they have an aligned concrete-shift witness of lag `m`.

So an SSE lower bound `lag > L` from concrete-shift failure needs all of the
following:

1. The theorem hypotheses must match the endpoints: finite essential square
   matrices, with the same lag convention used by the SSE search and the
   concrete-shift relation.
2. The checked relation must be theorem-linked to SSE at the same fixed lag.
   Exact aligned no-witness data is enough for the implication above. Exact
   balanced or compatible no-witness data is usable only through the fixed-lag
   equivalence theorem and its hypotheses.
3. For every `m <= L`, the concrete-shift decision procedure must be complete
   for the relevant matrix size and relation. It must cover all possible
   shift-equivalence matrices `R,S` and all path isomorphisms, not just a
   bounded search envelope.
4. Any finite bound used by the implementation, such as a max entry for `R,S`
   or a witness enumeration cap, must itself be justified by a proof that every
   SSE witness of lag `<= L` has a concrete-shift representative inside that
   bound.
5. No search-budget abort may occur. A lower-bound certificate needs exact
   no-witness results for all lags in `1..=L`.

Without those conditions, a no-witness result is evidence about the searched
envelope only. It is not a theorem-grade SSE lower bound.

## Current implementation

The current concrete-shift implementation is a useful bounded positive proof
surface, not a complete negative oracle.

- `src/concrete_shift.rs` is `2x2` only.
- `ConcreteShiftSearchConfig2x2` bounds `max_lag`, `max_entry`, and
  `max_witnesses`.
- `enumerate_shift_equivalence_with_lag_2x2` enumerates only `R,S` candidates
  whose entries are at most `max_entry`.
- `search_concrete_witnesses_for_shift` enumerates fiberwise bijections only
  until `max_witnesses` is reached.
- `try_concrete_shift_shortcut_2x2` is only a late positive fallback for
  essential `2x2` endpoints, and currently runs only when the solver config is
  within the small `max_lag <= 4`, `max_entry <= 6` surface.
- The fallback checks aligned, balanced, and compatible concrete-shift
  relations, but it returns only positive proofs. Exhausted or limited searches
  fall through to `Unknown`.

A found concrete-shift witness is proof-grade positive evidence because the
stored witness is verified against the concrete-shift equations. A missing
witness is not proof-grade negative evidence unless the stronger completeness
obligation above has separately been met.

## `Exhausted` vs `SearchLimitReached`

`ConcreteShiftSearchResult2x2::Exhausted` means:

- the implementation found no concrete-shift witness inside the configured
  `2x2` lag, entry, relation, and witness-budget surface;
- the witness-budget abort did not fire for that surface.

It does not mean:

- no concrete-shift witness exists globally;
- no larger-entry `R,S` witness exists;
- no higher-lag witness exists;
- no witness exists in dimensions other than `2x2`;
- no SSE path of the same lag exists.

`ConcreteShiftSearchResult2x2::SearchLimitReached` is even weaker. It means at
least one bounded probe hit `max_witnesses` before completing the bounded
concrete-bijection search. It is inconclusive even for the configured bounded
surface.

`ConcreteShiftProfileStatus2x2::Exhausted` has the same bounded meaning inside
the profile envelope. With the current default profile config, that envelope is
especially small: `max_lag = 1`, `max_entry = 1`, `max_witnesses = 32`.

## Profile scoring

`src/path_scoring.rs` should continue to treat concrete-shift profile data as a
ranking or telemetry signal only.

The current scoring assigns favorable values to low-lag profile hits and
penalizes `SearchLimitReached` and `Exhausted`, but this is heuristic ordering.
It must not be promoted into rejection, shortcut pruning, segment admission, or
deduplication without a separate complete lower-bound certificate.

This keeps the dependent scoring feature `sse-rust-srl` separated from this
semantics decision.

## Reporting decision

The internal result names can remain as code enums, but negative reporting
should avoid implying theorem-grade no-witness.

This slice changed the report-only proposal status emitted by
`src/bin/report_concrete_shift_proposals.rs` from:

```text
exhausted
```

to:

```text
bounded_exhausted
```

Because `result_status` is part of the emitted JSON report surface, the
proposal report schema was bumped from `schema_version = 1` to
`schema_version = 2`. The rename applies to both the general concrete-shift
proposal report surface and the restricted boolean-bridge aligned
concrete-shift report surface.

The same slice added enum documentation clarifying that concrete-shift
`Exhausted` and profile `Exhausted` are bounded statuses. No solver behavior,
ranking behavior, search ordering, move generation, deduplication, or
canonicalization changed.

## Shortcut pruning and segment admission

Do not use concrete-shift failure to reject, prune, or skip SSE segment
searches under the current implementation.

The current positive fallback is acceptable: after a bounded SSE search fails
to find a path, a verified concrete-shift witness may still prove equivalence.
The negative cases must remain `Unknown` unless an independent complete
lower-bound certificate is available.

Segment admission should likewise ignore no-witness concrete-shift results.
Bounded concrete-shift failure may be logged, ranked, or compared as telemetry,
but it must not decide whether a segment is searchable.
