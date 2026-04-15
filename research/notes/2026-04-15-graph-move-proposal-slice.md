# Graph move proposal slice (2026-04-15)

## Question

What is the safest first implementation slice toward proposal-oriented graph
moves, without redesigning `src/search.rs`, and does the existing
same-future/same-past signature machinery actually shrink the candidate set in
a useful way?

## Context

The repo already had the bounded ingredients needed for a proposal seam:

- same-future in-split generators,
- same-past out-split generators,
- bounded `3x3 -> 2x2 -> 3x3` zig-zag neighbors,
- same-future/same-past quotient signatures in `src/search.rs`.

What was missing was an explicit proposal object outside the blind frontier
expander. This slice therefore does **not** change default solver behavior.
Instead it:

- moves the same-future/same-past signature API into `src/graph_moves.rs`,
- adds an explicit `enumerate_graph_proposals(...)` research surface, and
- adds `compare_graph_move_proposals` to compare proposal candidates against
  blind one-step graph successors.

## Evidence

### 1. Endpoint source: `brix_ruiz_k3` source -> target

Command:

```sh
cargo run --quiet --features research-tools --bin compare_graph_move_proposals -- \
  --current source --target target --top-k 4
```

Result:

- blind one-step graph generation: `14` raw / `14` unique canonical successors,
  all `3x3`;
- explicit proposal families: `84` raw / `14` unique canonical proposals,
  entirely overlapping the blind one-step set;
- best quotient-signature gap is identical on both surfaces:
  `dim=1 row=5 col=17 entry_sum=2`;
- the best-gap bucket size is `1` on both surfaces.

Interpretation:

- the raw proposal families are **not** smaller than blind graph generation at
  the source;
- the quotient signature is still useful as a shortlist signal, but here it
  only rediscovers the same best `3x3` candidate already present in the blind
  one-step surface.

### 2. Same-dimension waypoint probe: `guide:1 -> guide:15`

Command:

```sh
cargo run --quiet --features research-tools --bin compare_graph_move_proposals -- \
  --current guide:1 --target guide:15 --top-k 6
```

Current and target are both `3x3` matrices from the seeded `brix_ruiz_k3`
guide path.

Result:

- blind one-step graph generation: `28` raw / `27` unique canonical successors,
  with dimensions only `2x2` and `4x4`;
- best blind quotient-signature gap:
  `dim=1 row=2 col=18 entry_sum=4`;
- explicit proposal families: `644` raw / `33` unique canonical proposals,
  including `6` same-dimension `3x3` zig-zag proposals;
- best proposal quotient-signature gap:
  `dim=0 row=2 col=6 entry_sum=0`;
- best-gap proposal bucket size: `1`;
- best proposal:

```text
[0, 0, 1
 1, 1, 1
 3, 3, 1]
```

Interpretation:

- the raw proposal universe is much larger than blind one-step expansion, so it
  should not be dropped into the main frontier unchanged;
- however, the quotient-signature shortlist is tiny (`1` matrix here), and the
  best shortlisted proposal is a same-dimension zig-zag candidate that the
  blind one-step expander cannot express at all;
- this is the concrete evidence that a real proposal seam exists: proposal
  sources matter once the search can consume something other than the blind
  one-step frontier.

## Conclusion

The safest first slice is an **inert research seam** in `src/graph_moves.rs`,
not a `search.rs` rewrite.

This slice shows:

- proposal candidates can be made explicit and inspectable as concrete graph
  objects with provenance and quotient-signature scores;
- the existing quotient machinery is strong enough to collapse a large raw
  proposal universe to a tiny shortlist;
- on a same-dimension `3x3 -> 3x3` waypoint probe, that shortlist is strictly
  better than any blind one-step graph successor under the same quotient score.

What it does **not** show:

- the raw proposal families are not smaller than blind graph widening on their
  own;
- the current quotient score does not yet prove that the shortlisted proposal
  is globally useful for full endpoint search;
- this is not yet a reason to alter default search expansion.

## Follow-up Slice

The next landed step keeps default search untouched, but starts **consuming**
the best-gap shortlist under explicit bounds instead of only printing it.

Code surface:

- `GraphProposals::best_gap_shortlist(...)` makes the quotient shortlist a
  concrete reusable graph-move surface instead of ad hoc iterator logic;
- `search::probe_graph_proposal_shortlist(...)` evaluates only the best-gap
  shortlist under a bounded graph-only lag cap;
- `compare_graph_move_proposals --probe-lag N` now exercises that probe on the
  existing research fixture surface.

### Waypoint probe with bounded realization

Command:

```sh
cargo run --quiet --features research-tools --bin compare_graph_move_proposals -- \
  --current guide:1 --target guide:15 --top-k 6 --probe-lag 3
```

Result:

- the best-gap shortlist remains size `1`;
- that single shortlisted `3x3` zig-zag proposal is realizable from
  `guide:1` by a bounded graph-only search in `3` steps;
- the bounded realization touched `44` visited states and expanded `2`
  frontier nodes;
- default endpoint search behavior is unchanged because the probe is opt-in and
  lives outside `expand_frontier_layer_dyn(...)`.

Interpretation:

- the quotient shortlist is now actionable rather than purely descriptive;
- the best proposal is not only scored well, it is also reachable under a tiny
  explicit graph-only budget from the current waypoint;
- this is still a waypoint evaluator, not yet a proof that proposal-guided
  restarts improve full endpoint search.

## Next Steps

1. Keep raw proposal generation out of the default frontier.
2. Extend the bounded probe into a restart or continuation evaluator only if it
   continues to pay off on more waypoint or endpoint cases.
3. If the best-gap probe repeatedly produces realizable and helpful waypoints,
   thread only that tiny top-k surface into staged search behind explicit
   bounds.
