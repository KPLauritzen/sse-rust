# `endpoint_equivalent_fast` investigation after the bounded `4x3 -> 3` orbit-exhaustion sidecar merge (2026-04-18)

## Scope

Bounded slice for bead `sse-rust-d4x`:

- investigate the reported `endpoint_equivalent_fast` slowdown after
  `64384ee` / `fcce03f`
- stay on the exact touched commit seam
- either recover the microbench with a tiny safe fix or reject the regression
  with tighter evidence

## Merge-local diff checked

The touched commits were:

- `64384ee` `Add bounded 4x3-to-3 orbit exhaustion probe`
- `fcce03f` `Guard bounded orbit probe lag budget`

The merge-local diff only touched:

- `src/bin/profile_structured_factorisation_orbits.rs`
- `research/notes/2026-04-18-binary-sparse-4x4-to-3-bounded-orbit-exhaustion.md`

No library search code, benchmark source, or shared runtime path changed in
that window.

## Build-graph seam

Focused check:

```bash
cargo bench --bench search endpoint_equivalent_fast --no-run -v
```

The package-local compilation surface for the benchmark was:

- `src/lib.rs`
- `benches/search.rs`
- `src/bin/search.rs`

The touched research binary
`src/bin/profile_structured_factorisation_orbits.rs` was **not** built for the
bench target because it is gated behind `required-features = ["research-tools"]`
in `Cargo.toml`.

So the reported regression is not attributable to a hot-path logic change in
the landed sidecar. The only remaining causal theory would be a benchmark or
tooling artifact.

## Exact commit validation

Because nested repo-local worktrees can otherwise resolve the outer manifest,
the detached-commit reruns used explicit `--manifest-path` commands.

Commands:

```bash
git worktree add tmp/wt-pre64384 c3730d778d7df82ca0d29fbecf3539bc5761a66e
git worktree add tmp/wt-64384 64384eee4290f54984c60cbc664fa3cb488ee641

cargo bench --manifest-path /home/kasper/dev/sse-rust__worktrees/d4x-endpoint-equivalent-fast-regression/tmp/wt-pre64384/Cargo.toml --bench search endpoint_equivalent_fast -- --noplot

cargo bench --manifest-path /home/kasper/dev/sse-rust__worktrees/d4x-endpoint-equivalent-fast-regression/tmp/wt-64384/Cargo.toml --bench search endpoint_equivalent_fast -- --noplot

rm -rf target/criterion/endpoint_equivalent_fast
cargo bench --bench search endpoint_equivalent_fast -- --noplot
```

Clean reruns on the exact seam:

| commit | role | result |
| --- | --- | --- |
| `c3730d7` | parent of `64384ee` | `time: [2.6423 µs 2.6476 µs 2.6528 µs]` |
| `64384ee` | sidecar commit | `time: [2.6119 µs 2.6169 µs 2.6221 µs]` |
| `fcce03f` | head / guard follow-up | `time: [2.6031 µs 2.6075 µs 2.6122 µs]` |

That first parent run appeared to show the opposite effect, so I reran the same
exact parent and sidecar commits after clearing only the local Criterion sample:

```bash
rm -rf tmp/wt-pre64384/target/criterion/endpoint_equivalent_fast
cargo bench --manifest-path /home/kasper/dev/sse-rust__worktrees/d4x-endpoint-equivalent-fast-regression/tmp/wt-pre64384/Cargo.toml --bench search endpoint_equivalent_fast -- --noplot

rm -rf tmp/wt-64384/target/criterion/endpoint_equivalent_fast
cargo bench --manifest-path /home/kasper/dev/sse-rust__worktrees/d4x-endpoint-equivalent-fast-regression/tmp/wt-64384/Cargo.toml --bench search endpoint_equivalent_fast -- --noplot
```

Follow-up reruns:

| commit | rerun result |
| --- | --- |
| `c3730d7` | `time: [2.5953 µs 2.5986 µs 2.6021 µs]` |
| `64384ee` | `time: [2.6004 µs 2.6040 µs 2.6083 µs]` |

## Read

The parent commit alone moved from `2.6476 µs` median to `2.5986 µs` median
across two clean reruns, a swing of about `1.9%` with no code change at all.
That within-commit drift is larger than the originally reported regression band.

Once measured cleanly on the exact commit seam, the three relevant commits all
cluster in the same `~2.60 µs` range:

- parent rerun median: `2.5986 µs`
- sidecar rerun median: `2.6040 µs`
- head rerun median: `2.6075 µs`

That spread is small, changes sign across runs, and does not support a stable
slowdown caused by `64384ee` or `fcce03f`.

## Conclusion

Reject this as a merge-local runtime regression.

What the bounded investigation does support:

- the landed change did not touch the endpoint search path
- the touched research binary is outside the `cargo bench --bench search`
  build graph
- exact parent/commit reruns do not reproduce a stable slowdown
- within-commit benchmark drift is large enough to explain the earlier signal

No fix was kept because there is no causal seam left to repair inside the
requested commit window.
