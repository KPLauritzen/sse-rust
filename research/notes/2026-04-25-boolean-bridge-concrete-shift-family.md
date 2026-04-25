# Boolean-bridge aligned concrete-shift family (2026-04-25)

## Question

Is there a concrete-shift-adjacent restricted witness class that is narrow
enough to implement cheaply later, but still broad enough to contain the small
positive controls already exposed by the report-only proposal surface?

This note answers that question for one candidate family only.

## Candidate family

Keep a restricted witness class:

- **boolean-bridge aligned concrete shift**

The point is not to define a new equivalence relation. The point is to define a
smaller witness family inside the existing aligned concrete-shift surface.

## Definition

Let `A` and `B` be square nonnegative integer matrices, and let `m >= 1`.

A **boolean-bridge aligned concrete-shift witness of lag `m`** from `A` to `B`
is an aligned concrete-shift witness

```text
W = (R, S, sigma_g, sigma_h, omega_e, omega_f)
```

such that:

1. `(R, S)` is a lag-`m` shift-equivalence witness:

   ```text
   A^m = RS,
   B^m = SR,
   AR = RB,
   BS = SA.
   ```

2. `(sigma_g, sigma_h, omega_e, omega_f)` satisfies the aligned concrete-shift
   equations exactly as in the current concrete-shift surface.

3. Every entry of `R` and `S` is boolean:

   ```text
   R_{ij} in {0,1},   S_{ij} in {0,1}   for all i,j.
   ```

Equivalently, this is the aligned concrete-shift surface with the extra witness
restriction `max_entry(R) <= 1` and `max_entry(S) <= 1`.

## Operational form for later implementation

For the current `2x2` implementation surface in `src/concrete_shift.rs`, this
family is testable without new witness-verification logic:

1. enumerate bounded lag-`m` shift-equivalence witnesses as today;
2. keep only witnesses with boolean `R` and `S`;
3. run the existing aligned concrete-shift validator unchanged.

For the current bounded search code, that means a dedicated restricted pass can
be implemented by fixing:

```text
relation = aligned
max_entry = 1
```

and leaving the aligned witness verification untouched.

## Why this is not already covered

This family is **not** the same as the existing `aligned`, `balanced`, or
`compatible` labels.

- Those labels describe which path-bijection equations a witness satisfies.
- They allow arbitrary nonnegative bridge matrices `R` and `S`.
- The boolean-bridge family adds a combinatorial restriction on the
  shift-equivalence bridge matrices themselves.

So this is a restricted witness class inside aligned concrete shift, not a new
name for the current labels and not a bridge to balanced elementary
equivalence.

## Toy control check

Used the existing report-only binary with a boolean bridge bound:

```bash
mkdir -p tmp
timeout -k 20s 180s cargo run --features research-tools --bin report_concrete_shift_proposals -- \
  --case lag_one_shortcut_control \
  --case identity \
  --max-entry 1 \
  > tmp/4x1-concrete-shift-family-report-max1.json
```

Observed results:

- `lag_one_shortcut_control` stayed positive on the aligned surface with
  `lag = 1`, `R = [[1,0],[1,1]]`, `S = [[0,1],[1,1]]`.
- `identity` also stayed positive with `lag = 1`, but the bounded search found
  a nontrivial boolean bridge witness
  `R = S = [[0,1],[1,0]]` rather than the diagonal identity witness.

This establishes two useful facts:

1. the restricted family is nonempty on the current bounded controls; and
2. the family is not merely the trivial `R = S = I` corner.

What this check does **not** establish:

- it does not prove completeness for aligned concrete shift even in `2x2`;
- it does not show that every short-lag witness can be normalized to boolean
  bridge matrices; and
- it does not justify pruning from failure in this restricted family.

## Expected cost and risk

Implementation cost:

- low for a research-only helper or report/search subpass;
- no new path-bijection equations are needed;
- existing proposal data already reports the relevant `R/S` support summaries
  and max-entry facts.

Risk:

- medium to high as a completeness surface;
- there is no theorem in the current repo notes showing that an arbitrary
  aligned concrete-shift witness can be replaced by one with boolean `R` and
  `S`;
- multiplicity-bearing shift witnesses may be genuinely necessary on harder
  cases.

Practical interpretation:

- good candidate for an early cheap subfamily or report stratification;
- not justified as a rejection rule when the boolean pass fails.

## Decision

**Keep** this family as a bounded restricted witness class worth implementing
later.

Reason:

- it is precise;
- it is operationally cheap;
- it uses terminology already present in the concrete-shift surface;
- and the existing positive controls already land inside it.

The keep decision is narrow: keep it as a research/experimental subfamily, not
as a replacement for unrestricted aligned concrete-shift search.

## Proposed follow-up bead

Title:

- `Implement boolean-bridge aligned concrete-shift subsearch for 2x2`

Bounded acceptance criteria:

1. Add an opt-in research-only helper or report path that searches aligned
   concrete-shift witnesses with `R,S in {0,1}` only.
2. Do not change the default solver mode and do not prune from failure in this
   restricted family.
3. `lag_one_shortcut_control` and `identity` remain positive at lag `1` under
   the restricted helper.
4. Output and docs label the result as a restricted aligned concrete-shift
   witness class, not as full concrete-shift completeness and not as a
   replayable SSE path surface.
