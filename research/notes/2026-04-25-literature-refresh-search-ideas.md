# Literature refresh: ranked search ideas after re-checking the repo evidence (2026-04-25)

## Scope

This note re-checks the repo's existing literature and research notes with the
current code and experiment history in mind. The goal is not to restate the
paper summaries. The goal is to decide which ideas still survive contact with
the repo's newer evidence and which ones should now be downgraded.

Project-goal priority for this refresh:

1. Goal 2: a new shortest path with lag `< 7` for `k = 3`
2. Goal 3: any path for Brix-Ruiz `k = 4` or higher
3. Goal 4: endpoint-agnostic square solver behavior up to dimension `4`

## Papers And Notes Rechecked

Primary repo docs and notes re-read for this slice:

- `TERMINOLOGY.md`
- `docs/research-ideas.md`
- `docs/aligned-shift-equivalence.md`
- `docs/brix-ruiz-sidecar-log.md`
- `research/notes/2026-04-13-solver-literature-ideas.md`
- `research/notes/2026-04-13-esse-distance-heuristics.md`
- `research/notes/2026-04-25-boolean-bridge-concrete-shift-family.md`
- `research/notes/2026-04-25-boolean-bridge-subsearch.md`
- `research/notes/2026-04-25-concrete-shift-proposal-data.md`
- `research/program.md`

Additional repo evidence checked before ranking:

- `research/notes/2026-04-15-positive-conjugacy-phase2-usefulness.md`
- `research/notes/2026-04-20-brix-ruiz-k4-graph-plus-structured-single-doubled-diagonal-refactorization-4x4-reject.md`
- `research/notes/2026-04-25-baker-lag7-structured-classification.md`
- `research/notes/2026-04-25-concrete-shift-profile-beam-prototype.md`
- `research/notes/2026-04-25-witness-bridge-motif-inventory.md`

Tracker state checked:

- `bd ready --json`
- `bd list --json`
- existing beads `sse-rust-srl`, `sse-rust-7sd`, `sse-rust-nw7.3`,
  `sse-rust-1j1`, `sse-rust-84v`, `sse-rust-132`, and closed bead
  `sse-rust-o6o`

## Revised Assumptions

### 1. Low-lag concrete-shift ideas are still the strongest literature-backed ranking lane, but the old note overstated what is already proved in the repo

What still stands:

- Bilich-Dor-On-Ruiz and Carlsen-Dor-On-Eilers still make aligned or
  compatible concrete-shift data the cleanest theorem-backed surface near short
  SSE lag.
- `docs/aligned-shift-equivalence.md` is right that concrete shift is no longer
  a speculative sidecar.

What must be tightened:

- the repo's current concrete-shift search is bounded and small-case oriented;
- bounded failure is not automatically a pruning theorem;
- `research/notes/2026-04-25-concrete-shift-profile-beam-prototype.md` already
  rejected one cheap cross-layer beam signal built from a very small concrete
  profile.

So the surviving claim is narrower: richer concrete-shift residual data is
still the best literature-backed ranking bet, but only as telemetry or scoring
until `sse-rust-7sd` clarifies the lower-bound conditions.

### 2. Same-future/past quotienting is no longer a fresh top bet; it is already partially landed and partially stress-tested

The April 13 note ranked same-future/past quotient signatures near the top.
That is stale as a planning statement.

Newer repo evidence already changed the status:

- same-future/past signatures and graph representative selection landed;
- path and guide-pool quotient tooling also landed;
- partition-refined quotient ranking stayed useful as analysis, but later
  bounded promotion attempts were rejected on the hard lanes.

So quotient-style structure should be treated as:

- already useful for dedup, representative selection, and analysis;
- not the main new literature refresh follow-up by itself;
- and not evidence for new hard pruning.

### 3. Narrow diagonal-refactorization is no longer a hypothetical missing family

The April 13 note proposed adding a narrow diagonal-refactorization family.
That is already obsolete:

- `src/factorisation.rs` already has `diagonal_refactorization_3x3` and
  `diagonal_refactorization_4x4`;
- the retained Brix-Ruiz `k = 4` lane already shows that the `4x4` family is
  active;
- a concrete single-doubled refinement was tested and rejected.

The remaining literature lesson is therefore not "add diagonal
refactorization." It is "do not overgeneralize from the presence of diagonal
hits into a reusable default same-size `4x4` family."

### 4. Blind split widening should stay downgraded, but compressed complete in-splits remain distinct from the failed sidecar widenings

The sidecar log still strongly rejects blind one-step or two-step split
widening around the Brix-Ruiz family. That downgrade remains correct.

But `docs/research-ideas.md` and the open bead `sse-rust-nw7.3` preserve a
different literature-backed possibility:

- higher-power or complete in-split compression is not the same as replaying
  the old bounded split-sidecar growth;
- it is only worth keeping if it becomes a small proposal slice on the retained
  `k = 4` lane rather than another generic widening pass.

### 5. Sampled positive-conjugacy should be downgraded from waypoint optimism to seed-only optimism

The older ranking note treated proximity to sampled positive-conjugacy
waypoints as a strong candidate signal.

The repo evidence now says:

- the sampled positive-conjugacy surface is still interesting as a source of
  local structure;
- but the tested top-ranked proposals do not survive endpoint invariants as
  literal intermediate targets on `k = 3` or the checked `k = 4` slice.

So the right status is:

- seed or reprojection material only;
- not a good next bounded bead for literal waypoint search;
- not a proof surface.

## Ranked Shortlist

Ranking rule for this note:

- theorem-backed directions beat equally plausible speculation;
- repo-backed positive evidence beats untouched paper optimism;
- Goal 2 and Goal 3 dominate Goal 4 unless the literature idea is primarily a
  solver-architecture seam.

### 1. Richer low-lag concrete-shift profile scoring, with theorem/pruning semantics separated first

Status:

- theorem-backed at the relation level;
- repo-backed as an implemented surface;
- still under-tested in the richer profile form that actually matters.

Why it stays first:

- it is still the cleanest literature-backed signal tied directly to short SSE
  lag rather than generic endpoint similarity;
- it can plausibly affect both Goal 2 and Goal 3 by improving search quality
  under the same budget;
- the cheap prototype that failed in `sse-rust-o6o` was a very small bounded
  result-class signal, not the richer residual or fiber-profile lane described
  in `sse-rust-srl`.

Potential impact:

- Goal 2: high
- Goal 3: medium to high
- Goal 4: low to medium

Main risks:

- bounded no-witness data may be misread as theorem-grade pruning;
- richer residuals may be too expensive unless cached carefully;
- a second beam-only attempt could simply repeat the rejected cheap prototype.

Actionability:

- actionable now, but already tracked by `sse-rust-srl`
- correctness guardrail already tracked by `sse-rust-7sd`
- if cross-layer order is revisited, `sse-rust-132` remains the execution
  prerequisite

No bead opened:

- existing beads already cover the concrete bounded experiment and the required
  proof-conditions note

### 2. Compressed complete-in-split or higher-power proposal slices for open Brix-Ruiz `k = 4`

Status:

- literature-backed, but only at the proposal-family level;
- repo evidence is still neutral because the distinct compressed slice has not
  yet been tested on the retained lane;
- clearly separate from the already rejected blind split widening.

Why it ranks second:

- Goal 3 is the hardest unsolved project goal, and this is the one still-live
  literature direction that targets a missing structured vocabulary rather than
  a generic budget increase;
- the sidecar failures actually make the distinction more important: if the
  idea is tried, it must be tried as compressed targeted proposals, not as
  another local refinement universe.

Potential impact:

- Goal 2: low
- Goal 3: high
- Goal 4: low

Main risks:

- could collapse back into broad split widening in disguise;
- might need powers or compressed witnesses that do not fit the retained
  `dim <= 4` lane cleanly;
- no repo evidence yet says it will beat the stronger current diagonal
  approximate-hit surface.

Actionability:

- actionable now, but already tracked by `sse-rust-nw7.3`

No bead opened:

- the existing bead already states the right guardrails and acceptance shape

### 3. Endpoint-neighborhood normal forms and dynamic square-endpoint parity

Status:

- only partly theorem-backed;
- mostly a repo-architecture and search-quality seam;
- still credible because Goal 4 remains explicitly open in `research/program.md`.

Why it ranks third:

- this is the most coherent literature-refresh direction for Goal 4 after
  downgrading the more speculative move-family ideas;
- it also has some plausible secondary value for ranking and duplicate control
  near hard endpoints.

Potential impact:

- Goal 2: low
- Goal 3: low to medium
- Goal 4: high

Main risks:

- easy to turn into vague canonicalization work with no measured solver benefit;
- could over-compress states that need to stay distinct;
- literature support is weaker here than for concrete shift or graph moves.

Actionability:

- actionable now, already split cleanly between `sse-rust-1j1` and
  `sse-rust-84v`

No bead opened:

- both the exploratory note slice and the direct Goal 4 audit slice already
  exist

### 4. Best-first or cross-layer frontier ordering is only a subordinate tactic now

Status:

- not a primary literature idea on its own anymore;
- worth keeping only as an execution tactic for stronger signals.

Why it ranks below the three ideas above:

- the repo already rejected one concrete cross-layer beam experiment in
  `sse-rust-o6o`;
- the literature does not by itself tell us which score works;
- the main open question is signal quality, not merely heap-versus-queue.

Potential impact:

- Goal 2: medium if paired with a better signal
- Goal 3: medium if paired with a better signal
- Goal 4: low

Main risks:

- can add executor complexity without improving useful reach;
- easy to confuse ordering changes with heuristic pruning.

Actionability:

- do not pursue as a standalone new idea;
- only revisit through `sse-rust-srl` plus `sse-rust-132`

No bead opened:

- existing beads already cover the only sensible bounded follow-up shape

## Explicit Downgrades

### Downgrade: blind split-sidecar widening

Why:

- the sidecar log is now negative across direct balanced search, one-step and
  two-step out-split refinement, mixed out/in refinements, and bounded
  `3x3 -> 2x2 -> 3x3` closure probes;
- newer notes already distinguish the retained compressed-complete-in-split
  hypothesis from this failed broader lane.

Result:

- keep the downgrade;
- do not reopen generic split widening beads from this literature refresh.

### Downgrade: quotient-signature ranking as the main next literature bet

Why:

- quotient-style structure already paid off where it was most credible:
  dedup, representative selection, analysis, and shortlist collapse;
- later partition-refined promotion attempts did not justify default hard-lane
  use.

Result:

- keep quotient signals as local structure tools;
- do not treat them as the main fresh follow-up from the papers.

### Downgrade: generic or slightly retuned diagonal-refactorization as the next missing family

Why:

- diagonal families are already implemented;
- their presence does explain some retained `k = 4` approximate hits;
- but the Baker step-5 classification and the rejected single-doubled `4x4`
  probe both argue against promoting another clean default same-size family
  from this evidence.

Result:

- keep diagonal hits as diagnostic structure;
- do not treat "more diagonal refactorization variants" as the main refreshed
  literature direction.

### Downgrade: sampled positive-conjugacy proposals as literal waypoint candidates

Why:

- the phase-2 usefulness check is negative on the tested `k = 3` and `k = 4`
  slices before real search even starts;
- the proposals fail determinant or Bowen-Franks compatibility too often.

Result:

- keep sampled positive conjugacy as seed or reprojection material only;
- no new waypoint bead from this refresh.

### Downgrade: a clean default family extracted from Baker step 5

Why:

- the updated structured classification shows the remaining uncovered Baker step
  is a heterogeneous hidden bridge, not a clean reusable one-step family in the
  native `dim <= 4` envelope.

Result:

- useful as a diagnostic motif;
- not a general default solver family from this slice.

### Downgrade: arithmetic expansion as a near-term path-to-witness idea

Why:

- the Eilers-Kiming line remains legitimate for endpoint filtering and
  negative-case diagnosis;
- but for the stated project goals, especially the positive Brix-Ruiz targets,
  it is not the most direct next witness-hunting direction.

Result:

- keep it as support work, not a top-ranked follow-up for this bounded refresh

## Follow-Up Beads Opened

No new bead was opened from this slice.

Rationale:

- the top actionable ideas are already covered by bounded existing beads:
  `sse-rust-srl`, `sse-rust-7sd`, `sse-rust-nw7.3`, `sse-rust-1j1`,
  `sse-rust-84v`, and `sse-rust-132`;
- opening duplicate literature beads would add tracker noise without narrowing
  scope further;
- the promising-but-not-actionable directions in this refresh are exactly the
  ones that should stay as note-level downgrades rather than backlog items.

## Bottom Line

The literature refresh changes less about the paper ranking than about the repo
status of each idea.

The strongest surviving bet is still richer low-lag concrete-shift profiling,
but only if theorem-grade no-witness semantics stay separate from heuristic
ranking. The strongest open Goal 3 literature bet is still the compressed
complete-in-split or higher-power slice, but only as a tiny retained-lane
proposal family rather than renewed split widening. For Goal 4, the cleanest
next work is not another move family at all; it is endpoint-neighborhood and
dynamic square-endpoint parity work.

Everything else that looked attractive in the older notes now has enough repo
history to be downgraded explicitly.
