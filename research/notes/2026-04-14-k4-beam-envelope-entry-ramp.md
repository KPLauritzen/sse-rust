# Goal 3 probe: k4 beam-envelope entry ramp and width boundary (2026-04-14)

## Question
From the tractable k4 envelope (`mixed + beam64 + dim5 + lag14`), does widening arithmetic bound (`max_entry`) or beam width produce any witness signal before timeout?

## Endpoint and base envelope

- endpoint `1,4,3,1 -> 1,12,1,1`
- base tractable point from loop28: `mixed`, `beam64`, `dim5`, `lag14`, `entry10` -> unknown in 119s.

## Entry ramp at fixed frontier

Config: `mixed + beam64 + dim5 + lag14`, timeout 120s.

- `entry=11` (`tmp/loop29_k4_mixed_beam64_dim5_lag14_entry11.json`):
  - outcome `unknown`
  - factorisations `1,731,716`, visited `75,090`, frontier `1,666`
  - wall `114s`
- `entry=12` (`tmp/loop29_k4_mixed_beam64_dim5_lag14_entry12.json`):
  - outcome `unknown`
  - factorisations `1,865,003`, visited `76,817`, frontier `1,666`
  - wall `117s`

No witness (`equivalent`) at higher entry bound.

## Beam width boundary at entry=12

- `beam96`, `lag14` (`tmp/loop29_k4_mixed_beam96_dim5_lag14_entry12.json`): timeout 120s (empty JSON).
- `beam96`, `lag12` (`tmp/loop29_k4_mixed_beam96_dim5_lag12_entry12.json`): timeout 120s (empty JSON).

Wider beam is not tractable under the same timeout cap on this dim5 surface.

## Interpretation

- Raising entry bound from 10 to 12 increases work but does not produce a k4 witness.
- Beam width 64 remains the practical upper frontier in this envelope under 120s; width 96 crosses a timeout cliff even at lower lag.

## Decision

Keep Goal-3 exploration envelope at `mixed + beam64 + dim5 + lag<=14` for bounded sweeps; no positive k4 witness yet.
