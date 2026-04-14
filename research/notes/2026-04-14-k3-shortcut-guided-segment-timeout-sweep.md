# k=3 hard-surface guided segment-timeout sweep (2026-04-14)

## Question

Can `guided_segment_timeout_secs` make the hard shortcut-search surface (`max_intermediate_dim=5`, `max_entry=6`) tractable without killing progress toward lag `<7`?

## Hard surface config

Base config:

- endpoints: `A=[[1,3],[2,1]]`, `B=[[1,6],[1,1]]`
- stage: `shortcut-search`
- guides: `research/guide_artifacts/k3_normalized_guide_pool.json`
- `max_intermediate_dim=5`, `max_entry=6`
- guided: `max_shortcut_lag=4`, `min_gap=2`, `max_gap=6`, `rounds=2`
- shortcut: `max_guides=12`, `rounds=2`

## Broad timeout sweep (attempts=64)

Tried `guided_segment_timeout_secs` in `{1,2,3,5}` with outer `timeout 240`.

Outcome:

- all runs exited `124` (outer timeout),
- all output JSON files were empty.

Interpretation:

- simply adding segment timeouts at this budget does not make the hard surface complete reliably.

## Controlled small-budget probes

To isolate timeout impact, I reduced attempts and compared completion/work.

### Attempts=8

No segment timeout (`tmp/loop9_hard_guides12_a8_notimeout.json`):

- lag `7`
- improved `0`
- frontier `811`
- visited `45735`

`guided_segment_timeout=1` (`tmp/loop9_hard_guides12_a8_timeout1.json`):

- lag `7`
- improved `0`
- frontier `124`
- visited `9441`

### Attempts=16

No segment timeout with outer `timeout 120`:

- timed out (`124`), no JSON.

`guided_segment_timeout=1` (`tmp/loop9_hard_guides12_a16_timeout1.json`):

- completed
- lag `7`
- improved `0`
- frontier `456`
- visited `31584`

### Attempts=32

`guided_segment_timeout=1` (`tmp/loop9_hard_guides12_a32_timeout1.json`):

- lag `7`
- improved `0`
- frontier `1940`
- visited `148223`

`guided_segment_timeout=2` (`tmp/loop9_hard_guides12_a32_timeout2.json`):

- lag `7`
- improved `0`
- frontier `2773`
- visited `160174`

## Conclusion

Segment timeouts improve tractability at low attempt budgets, but in this sweep they produced zero segment improvements and did not move best lag below `7`. At higher budget (`64`) they still did not make runs complete under the outer cap.

## Next hypothesis

Use cheap admission/ranking before segment search (rather than only per-segment timeout) so attempt budget is spent on segments likely to shorten the path.
