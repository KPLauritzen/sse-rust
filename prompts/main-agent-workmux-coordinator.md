Main-agent coordinator workflow for this repo:

Goal:
- act as the coordinator for active `workmux` workers
- merge ready work cleanly
- run post-merge regression checks
- keep `main` pushed and clean
- when no worker is active, dispatch the next bounded task

Operating rules:
- use `workmux` for worker lifecycle management
- use file-based prompts when dispatching or following up with workers
- prefer bounded slices over broad rewrites
- do not revert unrelated user or worker changes
- if a benchmark or harness signal looks bad, confirm it before opening a bead
- if a worker is still active, report status/blockers instead of forcing action

Per-turn workflow:

1. Check active workers.
   - run `workmux list`
   - if useful, inspect with `workmux capture <handle> -n ...`

2. If a worker is `done`, take it through coordinator review.
   - run `workmux run <handle> -- roborev review --branch --wait --base main`
   - consider the issues revealed, read the context to validate if it is a real issue
   - if review finds a real issue, send a message to the worker to ask it to fix the issue
   - wait until the issue is fixed
   - rerun `roborev` until it passes cleanly

3. Merge ready work.
   - run `workmux merge <handle>`
   - let `workmux` clean up the branch/worktree/window

4. After each merge, validate on `main`.
   - save a fresh harness artifact with `just research-json-save <stamp>`
   - compare it against the latest previous `research/runs/*post-merge*.json`
   - report:
     - `required_cases`
     - `passed_required_cases`
     - `target_hits`
     - `total_points`
     - `total_elapsed_ms`
     - any changed case outcomes or witness lags

5. Run serial benchmarks after the harness.
   - `cargo bench --bench search -- --noplot`
   - if a regression signal appears, rerun the specific benchmark in isolation
   - only open a follow-up bead if the regression reproduces cleanly

6. Read the worker's durable note and open follow-up work only when justified.
   - always read the note left behind by the finished worker before deciding next steps
   - create/update beads for:
     - confirmed regressions
     - promising next steps that could move the project forward
     - follow-up work needed to refine or narrow the just-finished slice into a better bounded next experiment
   - do not open beads mechanically; only add work when the result suggests a meaningful next move

7. Push `main`.
   - run `git push`

8. If no workers are active, dispatch a new one.
   - check `bd ready --json`
   - if only an epic is ready, inspect current live children with `bd list --json`
   - choose one concrete bounded task
   - gather only enough context to write a good worker prompt
   - write the prompt under `tmp/`
   - dispatch with `workmux add -b -P tmp/<prompt>.md <branch-name>`

9. If a worker is not ready to merge, report:
   - active branch
   - current progress
   - concrete blocker, if any
   - `git push` result

Prompt-writing guidance:
- state the bead id and narrow goal up front
- list the exact repo files/notes to read first
- define the desired bounded slice
- state hard boundaries explicitly
- require a durable note whenever the worker produces measurement, a keep/reject decision, a negative result, or a conclusion likely to guide later work
- include focused validation commands
- tell the worker to update `bd`, commit, and run `roborev`

Default merge-report shape:
- branch merged and commit(s)
- post-merge harness artifact path and comparison baseline
- whether outcomes / witness lags changed
- benchmark outcome
- whether any follow-up bead was opened
- whether a new worker was dispatched
