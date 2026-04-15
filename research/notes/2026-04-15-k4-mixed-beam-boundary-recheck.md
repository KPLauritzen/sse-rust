# Goal 3 probe: k4 mixed-beam boundary recheck on current head (2026-04-15)

## Question

On current `HEAD 8606fcd`, can the previously logged k4 mixed-beam
`unknown`-returning envelope on the open Brix-Ruiz endpoint still complete
under the practical `120s` cap, and does rebuilding with `--release` change
that answer?

## Endpoint and baseline target

- endpoint `1,4,3,1 -> 1,12,1,1`
- historical local `unknown`-returning envelope from 2026-04-14:
  `mixed + beam64 + dim5 + entry10 + lag<=14`

## Commands

Builds:

- `cargo build --profile dist --bin search`
- `cargo build --release --bin search`

Run shape:

```sh
/usr/bin/time -f '%e' -o <timefile> \
  timeout -k 10s 120s <binary> \
  1,4,3,1 1,12,1,1 \
  --max-lag <lag> \
  --max-intermediate-dim 5 \
  --max-entry <entry> \
  --frontier-mode beam \
  --move-policy mixed \
  --beam-width 64 \
  --json --telemetry > <jsonfile>
```

## Results

`target/dist/search`:

- `lag14, entry12` (`tmp/fl1_6_k4_beam64_dim5_lag14_entry12_cap120.json`):
  timeout `120.02s` (`124`, empty JSON)
- `lag14, entry10` (`tmp/fl1_6_k4_beam64_dim5_lag14_entry10_cap120.json`):
  timeout `120.03s` (`124`, empty JSON)
- `lag13, entry10` (`tmp/fl1_6_k4_beam64_dim5_lag13_entry10_cap120.json`):
  timeout `120.02s` (`124`, empty JSON)
- `lag12, entry10` (`tmp/fl1_6_k4_beam64_dim5_lag12_entry10_cap120.json`):
  timeout `120.02s` (`124`, empty JSON)

`target/release/search`:

- `lag12, entry10`
  (`tmp/fl1_6_k4_release_beam64_dim5_lag12_entry10_cap120.json`):
  timeout `120.02s` (`124`, empty JSON)
- `lag10, entry10`
  (`tmp/fl1_6_k4_release_beam64_dim5_lag10_entry10_cap120.json`):
  timeout `120.02s` (`124`, empty JSON)

## Interpretation

- No `equivalent` witness was found.
- The previously logged dim5 mixed-beam `unknown` envelope did **not**
  reproduce on current `HEAD 8606fcd`.
- Rebuilding with `--release` did not recover the historical tractable region.
- In this worker environment, the practical timeout cliff is at or below
  `mixed + beam64 + dim5 + entry10 + lag10` under `120s`.
- No bounded dim5 mixed-beam setting completed in this recheck, so there is no
  new keepable Goal-3 signal and no current envelope improvement to carry
  forward.

## Decision

Treat this as a clean negative result for the refreshed `sse-rust-fl1.6` scope:
stop the k4 mixed-beam branch here unless future work first explains why the
2026-04-14/2026-04-14-followup envelope is not reproducible on the current
head.
