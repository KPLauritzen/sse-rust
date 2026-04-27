# endpoint_equivalent_fast nn2r baseline check (2026-04-27)

## Question

Decide whether the reproduced Criterion slowdown on
`endpoint_equivalent_fast` after `1d8046a` is a real search-code regression, a
benchmark-baseline artifact, or environment noise.

## Context

The coordinator observed a full post-merge Criterion regression:

- time `[2.1576 us, 2.1762 us, 2.1979 us]`
- change `+3.8053% / +4.8339% / +5.7750%`

An isolated rerun also reported a regression:

- time `[2.2275 us, 2.2474 us, 2.2693 us]`
- change `+4.1428% / +5.2962% / +6.3388%`

The suspected triggering commit, `1d8046a`, only changed retained research
artifacts:

```bash
git show --stat --oneline --name-only 1d8046a
```

showed:

```text
1d8046a Refresh Riedel endpoint-ceiling control map
research/notes/2026-04-27-riedel-graph-only-endpoint-ceiling-map-refresh.md
research/riedel_graph_only_endpoint_ceiling_map_run_2026-04-27.json
```

`git diff --name-only main...HEAD` was empty before this note was added.

## Evidence

The command shape suggested in the bead:

```bash
cargo bench --bench search endpoint_equivalent_fast -- --save-baseline nn2r-current endpoint_equivalent_fast
```

failed before measurement on this Criterion harness because the extra
post-`--` benchmark filter was rejected as an unexpected argument.

The corrected named-baseline save kept the benchmark filter on Cargo's side:

```bash
timeout -k 10s 20m cargo bench --bench search endpoint_equivalent_fast -- --save-baseline nn2r-current
```

Result:

```text
endpoint_equivalent_fast
                        time:   [2.2005 us 2.2234 us 2.2493 us]
Found 4 outliers among 100 measurements (4.00%)
```

The immediate named-baseline comparison was:

```bash
timeout -k 10s 20m cargo bench --bench search endpoint_equivalent_fast -- --baseline nn2r-current
```

Result:

```text
endpoint_equivalent_fast
                        time:   [2.1622 us 2.1832 us 2.2050 us]
                        change: [-3.0007% -1.8753% -0.8021%] (p = 0.00 < 0.05)
                        Change within noise threshold.
Found 6 outliers among 100 measurements (6.00%)
```

A repeat of the same comparison produced:

```text
endpoint_equivalent_fast
                        time:   [2.1451 us 2.1579 us 2.1721 us]
                        change: [-3.3033% -2.2156% -1.1718%] (p = 0.00 < 0.05)
                        Performance has improved.
Found 7 outliers among 100 measurements (7.00%)
```

## Conclusion

The reproduced `+4%` to `+6%` slowdown did not reproduce against a fresh,
explicit Criterion baseline. The baseline-confirmed data moved in the opposite
direction on both immediate comparisons, while the suspected merge changed no
solver or benchmark code.

Treat this signal as stale-baseline or run-environment noise, not a real
`endpoint_equivalent_fast` code-path regression. No production-code change is
justified from this microbench signal.
