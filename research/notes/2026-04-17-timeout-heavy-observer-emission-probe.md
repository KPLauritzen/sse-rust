# Timeout-heavy observer emission probe (2026-04-17)

## Question

Can instrumented search keep a cheaper observer payload during expansion
without weakening the partial observability we need from runs that may be
externally killed with `timeout -k ...`?

This round stayed deliberately narrow:

- graph-only endpoint search only;
- exact observer emission durability first;
- no solver refactor unless the timeout story stayed at least as strong as the
  current per-layer commit boundary.

## Search seams under test

The hot graph-only observer path currently builds full `SearchEdgeRecord`
payloads inline in `src/search.rs` before layer emission:

- `SearchEdgeRecord` stores full canonical/orig matrices plus exact `EsseStep`
  (`src/search_observer.rs`);
- graph-only observer mode computes the exact witness inline with
  `find_exact_graph_move_witness_between(...)` for every recorded edge
  (`src/search.rs`);
- `emit_layer` clones the full record vector and hands it to the observer
  (`src/search/dispatch.rs`);
- the sqlite recorder turns each `SearchEvent::Layer` into one transaction and
  commits it immediately (`src/sqlite_graph.rs`).

So the real durability boundary today is:

1. finish building one full layer of exact `SearchEdgeRecord`s;
2. emit that layer;
3. let sqlite commit that layer transaction.

Anything deferred beyond that boundary risks losing exact edge data on killed
runs.

## Probe setup

Control pair:

- source `1,3,2,1`
- target `1,6,1,1`
- `--max-lag 22`
- `--max-intermediate-dim 5`
- `--max-entry 6`
- `--search-mode graph-only`

Bounded commands:

```bash
/usr/bin/time -f '%e\t%M\t%x' \
  target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 --max-intermediate-dim 5 --max-entry 6 \
  --search-mode graph-only --json --telemetry

/usr/bin/time -f '%e\t%M\t%x' \
  timeout -k 2s 3s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 --max-intermediate-dim 5 --max-entry 6 \
  --search-mode graph-only \
  --visited-db tmp/observer_timeout_probe_3s.sqlite \
  --json --telemetry

/usr/bin/time -f '%e\t%M\t%x' \
  timeout -k 2s 10s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 --max-intermediate-dim 5 --max-entry 6 \
  --search-mode graph-only \
  --visited-db tmp/observer_timeout_probe_10s.sqlite \
  --json --telemetry

/usr/bin/time -f '%e\t%M\t%x' \
  timeout -k 2s 20s target/release/search 1,3,2,1 1,6,1,1 \
  --max-lag 22 --max-intermediate-dim 5 --max-entry 6 \
  --search-mode graph-only \
  --visited-db tmp/observer_timeout_probe_20s.sqlite \
  --json --telemetry
```

The sqlite probe surface is the right one here because it is exactly what gets
durably committed before an external kill.

## Measurements

### Uninstrumented control still finishes quickly

Without observer persistence, the exact graph-only search completed in:

- wall `6.82 s`
- max RSS `2,255,188 KB`
- exit `0`
- outcome `equivalent`
- witness lag `17`
- telemetry layers `16`
- `frontier_nodes_expanded = 1,382,998`
- `total_visited_nodes = 1,399,061`

So this endpoint is a real exact positive, not an inherently stuck run.

### Killed observer runs preserve only committed layers

`timeout -k 2s 3s` with `--visited-db` produced:

- wall `3.02 s`
- max RSS `7,712 KB`
- exit `124`
- sqlite rows:
  - `search_runs = 1`
  - `run_nodes = 874`
  - `run_edges = 1,140`
  - committed layers `0 .. 3`
  - `outcome = NULL`
  - `finished_unix_ms = NULL`

Committed layer breakdown:

- layer `0` forward: `14` edges from `1` parent
- layer `1` backward: `18` edges from `1` parent
- layer `2` forward: `408` edges from `14` parents
- layer `3` backward: `700` edges from `18` parents

`timeout -k 2s 10s` produced:

- wall `10.02 s`
- max RSS `7,844 KB`
- exit `124`
- sqlite rows:
  - `search_runs = 1`
  - `run_nodes = 11,363`
  - `run_edges = 16,550`
  - committed layers `0 .. 4`
  - `outcome = NULL`
  - `finished_unix_ms = NULL`

Hot committed layer:

- layer `4` forward: `15,410` edges from `320` parents
- average `48.16` recorded edges per parent in that layer
- `10,489` enqueued edges
- `4,921` seen-collision edges

`timeout -k 2s 20s` produced the **same durable sqlite state** as the 10-second
run:

- still `11,363` nodes
- still `16,550` edges
- still only committed layers `0 .. 4`
- still no `Finished` record

Interpretation:

- externally killed runs absolutely do retain useful partial data today, but
  only up to the last fully emitted layer;
- on this hard control, the observer path spent at least another `10` seconds
  after layer `4` without committing layer `5` or finishing;
- any idea that moves exact witness materialization later than the current
  layer commit boundary would make that already-large no-commit window worse.

## Candidate and rejection

### Safe bounded keep candidate

Keep the current durability boundary exactly where it is:

- still collect per-layer data during search;
- still emit exact `SearchEdgeRecord`s before the next layer starts;
- still let sqlite commit one exact layer at a time.

But make the in-layer payload cheaper on the hot graph-only path by replacing
the full hot-loop `SearchEdgeRecord` build with a layer-local descriptor such
as:

- parent locator (`from_canonical` or parent slot/index),
- exact `to_canonical`,
- exact `to_orig`,
- `move_family`,
- `status`,
- `enqueued`,
- `layer_index` / `direction` / `to_depth`.

Then materialize the full `SearchEdgeRecord` only at `emit_layer`, using:

- `from_orig` from the already-held current-frontier representative;
- `from_depth` from the layer depth;
- exact graph witness from
  `find_exact_graph_move_witness_between(from_orig, to_orig)`.

Why this candidate is conservative:

- it keeps exact committed rows for every emitted layer;
- it does **not** defer witness materialization past the existing sqlite commit
  boundary;
- it directly attacks repeated parent cloning in the hottest observed layer:
  layer `4` recorded `15,410` edges from only `320` parents, so the current
  eager path clones the same parent payload about `48x` per parent before
  flushing.

This is the only keep candidate from this round that looks safe enough to
measure later.

### Durable rejection

Reject any design that defers exact witness materialization until:

- after sqlite edge insertion,
- after a later background pass,
- or after successful search completion.

Concrete adversarial timeout case:

- the `timeout -k 2s 20s` run was externally killed with no `Finished` event,
  yet it still left a useful partial graph through committed layers `0 .. 4`;
- a later-boundary design would have left those same killed-run layers without
  exact edge witnesses, or without edge rows at all if the deferred rebuild had
  not happened before death;
- since the 10-second and 20-second runs had identical durable state, this seam
  already has a very long window with no new commit. Deferring witness work
  past layer emission would only enlarge that risk surface.

So "reconstruct later if the run succeeds" is not acceptable for timeout-heavy
observer work on this control.

## Decision

Current eager observer records are still the right durability tradeoff at the
layer boundary.

What this round supports:

- a later follow-up may prototype a **same-boundary** graph-only descriptor
  compression;
- that prototype must measure killed-run durability first, not just completed
  runtime.

What this round rejects now:

- any observer design that postpones exact edge reconstruction beyond the
  current per-layer emit/commit boundary.
