- Run `cargo fmt` before any commit. 
- Use reasonable timeouts for potentially explosive search/probe commands; start with small bounds and increase only after the telemetry looks manageable.
- Commit your changes to git often. Before you ask the user for feedback or additional input. 
- When messaging other agents via `workmux`, prefer multiline file-based sends (`workmux send -f ...`) and make sure the file contains a real line break. In this repo's current setup, inline one-line sends to Codex panes can insert text without submitting it, while multiline file-based sends submit reliably.
- To rebuild the sandbox image for this repo, use `docker build -t my-sandbox -f Dockerfile.sandbox .`. Do not use `workmux sandbox build`; in this setup it rebuilds workmux's embedded default image instead of the repo's `Dockerfile.sandbox`.
- In this repo's `Dockerfile.sandbox`, the container sets `WORKMUX_SANDBOX=container`. Use that as the positive signal that you are inside the workmux sandbox.
- In this setup, some commands can still be proxied to the host by workmux host-command plumbing. Treat host paths like `/tmp` carefully: a command run via host tooling may write to host `/tmp`, not the sandbox's `/tmp`. When you need artifacts visible from both sides, prefer writing into the worktree.
- Use the repo-local `tmp/` directory for scratch artifacts that need to be visible from both sandboxed commands and host-executed tooling. It is intentionally gitignored except for `tmp/.gitkeep`.

## Project Goals

- Goal 1: Find any path for `k = 3`. This is already solved.
- Goal 2: Find a new shortest path with lag `< 7` for `k = 3`.
- Goal 3: Find any path for `k = 4` or above.
- Goal 4: Make the main solver endpoint-agnostic for square matrices up to dimension 4, possibly higher later.

## Beads

- Beads (`bd`) is the default task tracker in this repo.
- Use `bd` for actionable work tracking instead of adding new markdown TODO items.
- Treat existing markdown planning files as the source material to migrate from; do not assume they have already been imported into `bd`.
- Check ready work with `bd ready --json` or `bd list --json`.
- Create work with `bd create "Title" -t task|feature|bug -p 0-4 --json`.
- Update work with `bd update <id> --notes "..." --status ... --priority ... --json`.
- Close finished work with `bd close <id> --reason "..." --json`.
- Avoid `bd edit`; use non-interactive `bd update` flags instead.
- `bd` may be in use by another agent or process. If you hit an embedded-dolt exclusive-lock error, the usual resolution is to wait briefly and try again.

## Beads Issue Tracker

This project uses `bd` (beads) for issue tracking. Run `bd prime` to see the full workflow context and command reference.

### Quick Reference

```bash
bd ready                 # Find available work
bd show <id>             # View issue details
bd update <id> --claim   # Claim work
bd close <id>            # Complete work
```

### Rules

- Use `bd` for all task tracking instead of markdown TODO lists or ad hoc trackers
- Run `bd prime` when you need the detailed workflow and session-close guidance
- Use `bd remember` for persistent project knowledge instead of `MEMORY.md` files

## Session Completion

When ending a work session:

1. File issues for remaining work that still needs follow-up
2. Run quality gates when code changed
3. Update issue status so finished and in-progress work is reflected in `bd`
