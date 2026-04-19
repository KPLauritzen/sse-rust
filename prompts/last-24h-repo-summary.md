Write a durable note under `research/notes/` named
`YYYY-MM-DD-last-24h-repo-summary.md`.

Goal:

- summarize the highest-signal repo changes over roughly the last 24 hours
- explain which changes materially changed current solver direction versus
  merely adding evidence, instrumentation, or diagnostics
- use both durable notes and `bd` state as first-class inputs
- suggest plausible follow-up work that may have been missed or underweighted

Keep the note similar in tone and structure to:

- `research/notes/2026-04-18-last-24h-repo-summary.md`
- `research/notes/2026-04-17-last-24h-repo-summary.md`

Required source classes:

- git history for the last-day window
- durable research notes written in or about that window
- `research/log.md`
- current `bd` state, especially recently updated work, ready work, and live
  beads that own the active seams

Use concrete commands as needed, for example:

- `git log main --since='<window-start>' --stat --oneline`
- `git log --since='<window-start>' -- research/notes src docs`
- `ls research/notes/<date-prefix>*`
- `sed -n '1,220p' research/notes/<note>.md`
- `bd list --all --updated-after '<window-start-iso>' --json`
- `bd ready --json`
- `bd list --status=open --json`
- `bd show <id>`

Process:

1. Define the window explicitly with absolute timestamps. Use a roughly
   24-hour slice ending at the time of writing.
2. Read the concrete durable notes in that window first. Treat them as the
   primary evidence for what actually happened.
3. Cross-check against git history so major code, docs, or harness changes are
   not missed.
4. Read the relevant `bd` activity for the same window:
   - which beads were updated or closed
   - which beads remain open or in progress
   - which ready beads already cover obvious follow-up ideas
5. Identify the highest-signal changes. Prefer durable direction changes over
   a long changelog.
6. Separate:
   - changes that materially changed solver direction, baseline surfaces,
     pruning policy, or active search strategy
   - changes that are mainly evidence, reporting, telemetry, or diagnostic
     support
7. Add a follow-up pass that explicitly asks:
   - what promising next work is implied by the notes but not clearly owned by
     a live bead?
   - what work is already covered by an open or ready bead and therefore should
     not be called "missed"?
   - what follow-up should be suggested even if the last 24 hours were mostly
     negative or neutral?
8. If you find a plausible missed avenue, say why it looks worthwhile and name
   the concrete notes or bead gaps behind that judgment.

Output requirements:

- title the note `# Repo activity summary: YYYY-MM-DD to YYYY-MM-DD`
- include sections:
  - `## Question`
  - `## Scope`
  - `## Highest-signal changes`
  - `## Kept vs. evidence-only conclusions` if useful
  - `## Follow-up work that may be missing or underweighted`
  - `## Active seams already covered by beads`
  - `## Conclusion`
- list the primary sources in `## Scope`, including the exact `bd` commands or
  bead ids inspected
- make the follow-up section concrete:
  - recommend bounded next steps
  - tie each suggestion back to specific notes, commits, or bead gaps
  - distinguish "open but already owned" from "not clearly owned"

Guardrails:

- do not turn the note into a commit-by-commit dump
- do not treat summary notes as the only evidence; read the underlying durable
  notes
- do not call something "missed" if an open or ready bead already owns it
- do not invent successes, witnesses, or priority changes that are not grounded
  in the notes, git history, and `bd`
- if the window is quiet, say so plainly and focus on the few things that
  actually mattered

Write in the repo's existing durable-note style: direct, evidence-based, and
oriented toward current search strategy and backlog decisions rather than broad
project history.
