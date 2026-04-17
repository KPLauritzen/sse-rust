# `square_factorisation_3x3`: quotient/support obstructions on singular sources (2026-04-17)

## Question

For bead `sse-rust-ilu.2`, can quotient signatures, duplicate row/column
classes, support profiles, or related coarse canonical structure become an
**exact** source-only impossibility gate for `square_factorisation_3x3`,
especially on the singular `3x3` sources where determinant-factorization does
not help?

The target was bounded and research-first:

- keep one exact candidate if it survived;
- otherwise reject one idea with a clear counterexample;
- or leave a durable note explaining why the obvious quotient/support
  directions are too weak.

## Local setup

The bounded probes reused the exact enumerator/breakdown machinery in
[src/factorisation.rs](../../src/factorisation.rs) via
[src/bin/profile_sq3_breakdown.rs](../../src/bin/profile_sq3_breakdown.rs).

Saved probe outputs:

- [tmp/2026-04-17-profile-sq3-dup-row-support.txt](../../tmp/2026-04-17-profile-sq3-dup-row-support.txt)
- [tmp/2026-04-17-profile-sq3-singular-grid-cap2.txt](../../tmp/2026-04-17-profile-sq3-singular-grid-cap2.txt)

Commands:

```bash
cargo run --features research-tools --bin profile_sq3_breakdown --quiet -- --scan-dup-row-support --scan-max-entry 6 --sq3-cap 4
cargo run --features research-tools --bin profile_sq3_breakdown --quiet -- --scan-singular-grid --scan-max-entry 2 --sq3-cap 4
```

## Probe 1: hotspot-like duplicate-row support family

I first stayed close to the expensive seam and scanned the singular family

```text
[0,a,0]
[b,c,d]
[0,a,0]
```

with `a,b,c,d in {1, ..., 6}` and the current square-factor cap `4`.

Result:

- total matrices scanned: `1296`
- factorable by `square_factorisation_3x3`: `1218`
- unfactorable by `square_factorisation_3x3`: `78`
- coarse same-future/past collision with different factorability: none found
- partition-refined collision with different factorability: none found

### What this proves

This is already enough to reject **support-profile-only** and
**duplicate-row/duplicate-column-only** hard no-go claims for the current
hotspot shape.

All `1296` matrices share the same support pattern and the same obvious
duplicate-row structure, yet exact family emptiness still splits `1218 / 78`.
So support profile and duplicate classes alone do not determine whether
`square_factorisation_3x3` is empty.

### What it does not prove

Inside this one narrow family, the repo's current same-future/past signature
and the partition-refined quotient signal happened to separate the factorable
from the unfactorable cases. So that family alone was not enough to reject the
stronger quotient-style signatures.

## Probe 2: bounded singular-grid scan

To test the stronger source-only quotient idea directly, I scanned **all**
singular nonzero `3x3` matrices with entries in `{0,1,2}`.

Result:

- total singular matrices scanned: `6890`
- factorable by `square_factorisation_3x3`: `6864`
- unfactorable by `square_factorisation_3x3`: `26`

This scan produced exact collisions for both of the stronger quotient-style
signatures.

### Counterexample A: coarse same-future/past signature is too weak

The scan found a pair with the **same** coarse same-future/past signature but
different exact family behavior:

```text
left  = [0,0,0] [0,0,0] [2,1,0]   emitted_factorisations = 0
right = [2,1,0] [0,0,0] [0,0,0]   emitted_factorisations = 74700
```

So the coarse same-future/past quotient signature cannot be an exact
source-only impossibility certificate for `square_factorisation_3x3`.

### Counterexample B: partition-refined quotient signature is still too weak

The scan also found a pair with zero partition-refined quotient gap but
different exact family behavior:

```text
left  = [0,0,0] [0,0,0] [0,1,2]   emitted_factorisations = 0
right = [0,0,0] [0,1,2] [0,0,0]   emitted_factorisations = 74700
```

So even the stronger partition-refined quotient structure currently used for
proposal scoring is still not an exact source-only impossibility certificate
for `square_factorisation_3x3`.

## Interpretation

The exact `square_factorisation_3x3` witness equations are more rigid than
these quotient signatures.

The breakdown path in [src/factorisation.rs](../../src/factorisation.rs)
solves:

- row-wise admissibility filters for candidate `U` rows,
- column-wise nonnegative integer systems for the top two rows,
- then a final exact completion equation for the remaining row.

Those conditions depend on where the live rows and columns sit in the matrix,
not just on coarse duplicate/support partitions, class sums, or quotient block
profiles. The bounded singular-grid collisions above are the concrete proof.

## Verdict

Reject the obvious quotient/support directions as exact source-only hard gates
for `square_factorisation_3x3`:

- support profile alone: rejected
- duplicate row/column classes alone: rejected
- coarse same-future/past quotient signature: rejected
- one-step partition-refined quotient signature: rejected

What survives:

- these signatures are still useful for exact orbit reduction and proposal
  ordering;
- they are not strong enough to certify family emptiness on their own.

## Best next step

If the square-factorisation seam is revisited, the next exact obstruction needs
to track some witness-equation data that the quotient/support summaries forget:

- per-column solvability constraints,
- row/column placement-sensitive divisibility or completion conditions,
- or a genuinely family-specific singular certificate.

If the goal is a near-term exact gate worth implementation effort, the bounded
split/refactorization families still look cleaner than trying to promote these
coarse quotient/support summaries into a universal `square_factorisation_3x3`
prune.
