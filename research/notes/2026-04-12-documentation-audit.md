# Documentation Audit

## Question

Which documentation areas are currently creating the most friction in this
repo, and what structure should replace them?

## Context

The repo already has substantial written material in `README.md`, `docs/`, and
`research/`, but the boundaries between those areas were not explicit. That
made it harder to tell where new notes should live and which files were meant
to be durable references versus working scratchpads.

## Evidence

- `README.md`, `docs/TODO.md`, `docs/research-ideas.md`, and
  `research/program.md` all contain important orientation material, but there
  was no index describing which file was authoritative for what.
- `docs/aligned-shift-equivalence.md` still described aligned work as blocked on
  a missing source even though the repo already contains the Bilich-Dor-On-Ruiz
  reference and the code now includes concrete-shift validators.
- `docs/research-ideas.md` and `docs/aligned-shift-equivalence.md` still had
  hard-coded links to a different local checkout path.
- `research/log.md` worked as a terse ledger, but there was no tracked place
  for longer experiment writeups, literature summaries, or synthesis notes.

## Conclusion

The main cleanup need is not more prose. It is sharper document ownership:

- top-level project orientation in `README.md`,
- roadmap and topic notes in `docs/`,
- experiment workflow plus lab-notebook material in `research/`,
- active execution tracked in `bd`.

Within `research/`, the right split is:

- `log.md` for short chronological entries,
- `notes/` for longer-form evidence and synthesis,
- `runs/` and `results.tsv` for local artifacts.

## Next Steps

- Keep using `bd` instead of adding new markdown backlog files.
- When a result needs more than a few lines, add or update a note under
  `research/notes/` rather than overloading `research/log.md`.
- Refresh topic notes when code or references materially change, especially the
  aligned/concrete-shift docs.
