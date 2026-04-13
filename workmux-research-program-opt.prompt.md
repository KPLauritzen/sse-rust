Read `research/program.md` and take ownership of a long-running optimization pass based on it.

Primary objective:
- Keep pushing optimization work forward for as long as possible without waiting for user input.

Execution rules:
- Start by reading `research/program.md`, then identify the highest-leverage optimization direction from the current state of the repo.
- Prefer concrete progress over discussion: implement, measure, compare, and iterate.
- Stay aligned with the repo instructions in `AGENTS.md`, especially:
  - use reasonable timeouts for potentially explosive probes,
  - run `cargo fmt` before any commit,
  - commit your changes to git often,
  - use `bd` for actionable work tracking where appropriate.
- If you find multiple plausible optimization paths, choose one, pursue it, and only switch when the data says it is not paying off.
- Keep the work self-directed. Do not stop after one attempt; continue with follow-up experiments, refinements, cleanup, and additional measurements.
- Maintain a short running log in `bd` notes or commit messages so the progression is visible.

Suggested loop:
1. Read `research/program.md` and extract the concrete optimization target.
2. Inspect the relevant solver/search code and identify likely bottlenecks or pruning opportunities.
3. Make one bounded change at a time.
4. Run focused validation/measurement with conservative time bounds first.
5. Keep whichever changes improve the result and continue iterating.
6. Commit meaningful progress frequently before moving to the next experiment.

Deliverables while running:
- Ongoing commits for real progress.
- Terminal updates showing what was tried, what was learned, and what comes next.

Stopping condition:
- Continue until you are genuinely blocked by something external, or you exhaust credible optimization avenues from `research/program.md`.
