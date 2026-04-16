# Graph-Proposal Shortlist Rounds

## Status

Graph-proposal shortlist rounds are a research-tool-driven evidence seam.

- keep them outside `research_harness` for now;
- keep them outside default solver expansion for now;
- only reconsider that boundary after the same bounded round pays off on more
  than one waypoint family.

This follows the current repo guidance from
[`research/notes/2026-04-16-autoresearch-round-recommendations.md`](../research/notes/2026-04-16-autoresearch-round-recommendations.md)
and the original graph-proposal slice note in
[`research/notes/2026-04-15-graph-move-proposal-slice.md`](../research/notes/2026-04-15-graph-move-proposal-slice.md).

## Default Round

Run the default bounded waypoint round with:

```sh
just graph-proposal-shortlist-round
```

If the output should survive across host/sandbox boundaries, save it in the
repo-local scratch area:

```sh
just graph-proposal-shortlist-round | tee tmp/<stamp>-graph-proposal-shortlist.txt
```

The recipe executes `compare_graph_move_proposals` directly through the built
`target/dist/compare_graph_move_proposals` binary under a `20s` timeout rather
than using `timeout cargo run ...`.

## Default Shape

The default round is intentionally narrow and repeatable:

- endpoint fixture: `research/fixtures/brix_ruiz_family.json#brix_ruiz_k3`
- seeded guide: `endpoint_16_path`
- waypoint pair: `guide:1 -> guide:15`
- proposal generation bounds:
  - `max_dim=4`
  - `zigzag_bridge_entry=8`
  - `top_k=6`
- realization probe bounds:
  - shortlist cap `4`
  - graph-only BFS realization
  - `max_lag=3`
  - `max_entry=8`

This is the current documented default because it is the smallest kept round
that already showed a non-blind same-dimension shortlist candidate on the hard
`k=3` family waypoint seam.

## Fields To Compare

Use the same fields on every rerun so the round stays decision-oriented rather
than anecdotal.

From `Blind one-step graph successors`, compare:

- `raw candidates`
- `unique canonical successors`
- `dimension breakdown`
- `family counts`
- `best target signature gap`
- `best-gap shortlist`

From `Targeted graph proposals`, compare:

- `raw proposal candidates`
- `unique canonical proposals`
- `dimension breakdown`
- `family counts`
- `overlap with blind one-step successors`
- `best target signature gap`
- `best-gap shortlist`

From `Best-gap proposal probe`, compare:

- `best-gap shortlist`
- `shortlist cap`
- `probed proposals`
- `realization surface`
- `realization lag bound`
- `realization max_entry`
- per-attempt:
  - outcome
  - `proposal_dim`
  - `blind_overlap`
  - `frontier_nodes`
  - `visited`

The current kept baseline from
[`2026-04-15-graph-move-proposal-slice.md`](../research/notes/2026-04-15-graph-move-proposal-slice.md)
is a best-gap shortlist of `1` whose single shortlisted `3x3` zig-zag proposal
is realizable within lag `3`.

## Keep Or Revert

Keep a change when the default round improves useful waypoint quality under the
same bound. In practice, that means at least one of:

- the `best target signature gap` improves lexicographically;
- the same best gap survives with a tighter `best-gap shortlist`;
- a shortlisted proposal with `blind_overlap=false` becomes realizable;
- realizability stays flat while `frontier_nodes` or `visited` drops.

Revert or do not promote a change when:

- only raw proposal counts moved;
- blind overlap changed but the best gap and realizability did not improve;
- the shortlist got broader or weaker under the same bound;
- realizability regressed at the same bound;
- the change tries to turn this round into a `research_harness` default or a
  main-search default before cross-family evidence exists.

Raw proposal volume is not a success metric here. The point of the round is to
find a tiny, bounded, realizable shortlist that beats blind one-step graph
expansion on waypoint quality, not to maximize proposal generation.
