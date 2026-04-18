# endpoint_equivalent_fast regression investigation after `sse-rust-5yo.4` sidecar merge (2026-04-18)

## Scope

Bounded slice for bead `sse-rust-cr9`:

- investigate the confirmed `endpoint_equivalent_fast` microbenchmark regression
- focus on the `sse-rust-5yo.4` sidecar merge
- avoid reverting unrelated decomposition artifacts, notes, or worker changes

## Merge-local diff checked

The first sidecar commit in the reported regression window was:

- `71f2662` `Add retained k4 step decomposition sidecar`

Relative to its parent `9a04860`, the merge-local diff that could plausibly
touch the benchmark surface was:

- `Cargo.toml`: added an explicit `[[bin]]` entry for `explain_witness_step`
- `src/bin/explain_witness_step.rs`: new research helper binary

No library code, search implementation, or benchmark source changed in that
commit range.

## Focused validation

Primary gate on the investigation branch (`2051e11`):

```bash
cargo bench --bench search endpoint_equivalent_fast -- --noplot
```

Result:

```text
time: [2.5967 µs 2.6045 µs 2.6154 µs]
```

Pre-merge parent in a detached worktree at `9a04860`:

```bash
git worktree add --detach tmp/pre-71f2662 9a04860
cargo bench --bench search endpoint_equivalent_fast -- --noplot
```

Result:

```text
time:   [2.5987 µs 2.6026 µs 2.6066 µs]
change: [-0.3445% -0.0071% +0.3189%] (p = 0.97 > 0.05)
No change in performance detected.
```

Exact sidecar commit in a detached worktree at `71f2662`:

```bash
git worktree add --detach tmp/post-71f2662 71f2662
cargo bench --bench search endpoint_equivalent_fast -- --noplot
```

Result:

```text
time:   [2.6071 µs 2.6124 µs 2.6181 µs]
change: [+0.0413% +0.3005% +0.5621%] (p = 0.03 < 0.05)
Change within noise threshold.
```

Both detached-worktree runs reused the same benchmark executable path
`target/release/deps/search-667e9db4def1f542` and completed compilation in
`0.06-0.08 s`, which is consistent with the benchmark target itself being
unchanged across the parent and sidecar commit.

## Conclusion

I could not confirm a stable `endpoint_equivalent_fast` runtime regression
caused by the `sse-rust-5yo.4` sidecar merge.

What this investigation does confirm:

- the merge-local change set did not modify search-path or benchmark code
- the suspected helper/codegen-layout cause from `explain_witness_step` is not
  supported by the controlled parent-vs-merge benchmark comparison
- the previously reported `3.0590-3.1584 µs` isolated rerun did not reproduce in
  a clean detached-worktree comparison of the exact regression boundary

Within this bounded slice, the earlier regression signal is best treated as
unconfirmed against the actual sidecar merge boundary rather than something that
can be recovered with a safe runtime patch. No code change was applied because
there is no stable merge-local slowdown left to fix without drifting into broad,
speculative benchmark cleanup.
