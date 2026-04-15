# Goal 3 probe: k4 mixed-beam low-lag reramp on release binary (2026-04-15)

## Question

After the higher-lag recheck timed out, where is the first reproducible
`unknown`-returning region on current `HEAD 8606fcd` if we ramp upward from low
lag on the same mixed-beam surface?

## Fixed search surface

- endpoint `1,4,3,1 -> 1,12,1,1`
- binary `target/release/search`
- move policy `mixed`
- frontier mode `beam`
- beam width `64`
- `max_intermediate_dim=5`
- `max_entry=10`
- outer cap `timeout -k 10s 120s`

## Commands

```sh
/usr/bin/time -f '%e' -o <timefile> \
  timeout -k 10s 120s \
  target/release/search \
  1,4,3,1 1,12,1,1 \
  --max-lag <lag> \
  --max-intermediate-dim 5 \
  --max-entry 10 \
  --frontier-mode beam \
  --move-policy mixed \
  --beam-width 64 \
  --json --telemetry > <jsonfile>
```

## Results

- `lag4` (`tmp/x4v_k4_release_beam64_dim5_lag4_entry10_cap120.json`):
  `unknown` in `35.93s`, factorisations `585,227`, visited `20,978`,
  max frontier `64`
- `lag6` (`tmp/x4v_k4_release_beam64_dim5_lag6_entry10_cap120.json`):
  `unknown` in `61.36s`, factorisations `634,322`, visited `29,312`,
  max frontier `64`
- `lag8` (`tmp/x4v_k4_release_beam64_dim5_lag8_entry10_cap120.json`):
  `unknown` in `103.68s`, factorisations `788,465`, visited `42,465`,
  max frontier `64`
- `lag9` (`tmp/x4v_k4_release_beam64_dim5_lag9_entry10_cap120.json`):
  timeout `120.02s` (`124`, empty JSON)
- `lag10` (`tmp/fl1_6_k4_release_beam64_dim5_lag10_entry10_cap120.json`):
  timeout `120.02s` (`124`, empty JSON; from the immediately preceding
  release-binary recheck)

## Interpretation

- No `equivalent` witness was found.
- Current `HEAD 8606fcd` does still have a reproducible bounded
  `unknown`-returning mixed-beam region on the k4 endpoint.
- On this worker, the practical release-binary boundary for the fixed
  `beam64 + dim5 + entry10` surface is:
  - completes through `lag8`
  - times out by `lag9`
- This is the keepable envelope from the reramp. The earlier 2026-04-15 note
  only established that the higher-lag slice (`lag10+`) no longer reproduced.

## Decision

Keep this as the current bounded boundary map for the k4 mixed-beam branch on
this worker: `unknown` through `lag8`, timeout cliff between `lag8` and `lag9`,
still no Goal-3 witness.
