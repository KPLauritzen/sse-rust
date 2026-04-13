- Run `cargo fmt` before any commit. 
- Use reasonable timeouts for potentially explosive search/probe commands; start with small bounds and increase only after the telemetry looks manageable.
- Commit your changes to git often. Before you ask the user for feedback or additional input. 
- When messaging other agents via `workmux`, prefer file-based sends (`workmux send -f ...`). In this repo's current setup, inline one-line sends to Codex panes can insert text without submitting it.

## Project Goals

- Goal 1: Find any path for `k = 3`. This is already solved.
- Goal 2: Find a new shortest path with lag `< 7` for `k = 3`.
- Goal 3: Find any path for `k = 4` or above.
- Goal 4: Make the main solver endpoint-agnostic for square matrices up to dimension 4, possibly higher later.

## Beads Trial

- Beads (`bd`) is in trial use for task tracking in this repo.
- During the trial, prefer `bd` for actionable work tracking instead of adding new markdown TODO items.
- Treat existing markdown planning files as the source material to migrate from; do not assume they have already been imported into `bd`.
- Check ready work with `bd ready --json` or `bd list --json`.
- Create work with `bd create "Title" -t task|feature|bug -p 0-4 --json`.
- Update work with `bd update <id> --notes "..." --status ... --priority ... --json`.
- Close finished work with `bd close <id> --reason "..." --json`.
- Avoid `bd edit`; use non-interactive `bd update` flags instead.
- `bd` may be in use by another agent or process. If you hit an embedded-dolt exclusive-lock error, the usual resolution is to wait briefly and try again.
