- Run `cargo fmt` before any commit. 
- Use reasonable timeouts for potentially explosive search/probe commands; start with small bounds and increase only after the telemetry looks manageable.
- Commit your changes to git often. Before you ask the user for feedback or additional input. 

## Project Goals

- Goal 1: Find any path for `k = 3`. This is already solved.
- Goal 2: Find a new shortest path with lag `< 7` for `k = 3`.
- Goal 3: Find any path for `k = 4` or above.

## Beads Trial

- Beads (`bd`) is in trial use for task tracking in this repo.
- During the trial, prefer `bd` for actionable work tracking instead of adding new markdown TODO items.
- Treat existing markdown planning files as the source material to migrate from; do not assume they have already been imported into `bd`.
- Check ready work with `bd ready --json` or `bd list --json`.
- Create work with `bd create "Title" -t task|feature|bug -p 0-4 --json`.
- Update work with `bd update <id> --notes "..." --status ... --priority ... --json`.
- Close finished work with `bd close <id> --reason "..." --json`.
- Avoid `bd edit`; use non-interactive `bd update` flags instead.
