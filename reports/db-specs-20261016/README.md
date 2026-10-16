# Ingest + /db parts: cloud build rules (16 Oct 2026)

One file per ingest + /db part that had not been started on the operator's Mac by
16 Oct 2026 22:00 IST, plus `CONTEXT.md` (the background every part shares). Each part is
built by its own cloud session on its own branch and merged back into `feat/db-fix` on the
Mac, one at a time, after a second agent checks it. This folder is planning material: it is
never merged into `feat/db-fix`.

Ingest + /db comes first and alone: nothing else runs until it is finished end to end.

## Before you start

1. Your part's file names its **prerequisites**. Start only when every one of them is merged
   into `origin/feat/db-fix` (`git log origin/feat/db-fix --merges --format=%s | grep "Merge <name>:"`).
   If one is missing, say so in your report and stop: never build on top of an unmerged part.
2. Create your branch `cloud/db-<name>-<date>` from the newest `origin/feat/db-fix`.
3. Read `CLAUDE.md` (law), `CONTEXT.md`, then your part's file in full: the work package,
   the files it owns, the confirmed findings with their evidence, and how it is verified.
   Edit only the files your part owns.

## What changes in the cloud

- **Paths.** The files name folders on the operator's Mac (`/Volumes/WD_BLACK/...`, `wt/`,
  `tgt/`, `state/`, a `bin/cargo` wrapper). None exist here. Work in your checkout and use plain `cargo`.
- **Real data.** There is no market data store, no `:8080` app and no Dropbox link in the cloud.
  Build and test on synthetic fixtures. List every real-data check, store write and `:8080` check
  under **NEEDS THE MAC** in your report, with the exact command, so the Mac runs it at merge-back.
  Never invent a measurement.
- **Private settings.** Never write a Dropbox link, a token, a credential path or a parameter
  path into any file. They live only in the operator's local untracked files.

## Rules

- `CLAUDE.md` is law: Rust only outside `web/`, O(1) per operation measured at p99, no
  look-ahead, append-only, no depth parameter, honest limits, no fallback that hides a failure.
- Never use the Py-language and never write its name anywhere.
- Say "order flow delta" and "order flow cumulative delta", never the bare word.
- Gates before you call a part done: `cargo fmt --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`; `cargo test --workspace --locked`;
  `cargo deny check`; 100% line and branch coverage and no surviving mutant on the modules you touched.
  If a tool cannot be installed here, say so plainly.
- Push only your own branch. Never push `main` or `feat/db-fix`, never open a pull request,
  never delete anything.

## Turn protocol

Work in turns of at most 20 minutes. At the end of every turn: commit, push your branch, update
`reports/cloud-db-<name>.md`, end the turn and wait for a message that says CONTINUE. Its first
line is `STATUS: DONE` only when the part is complete and every gate is green; otherwise
`STATUS: IN PROGRESS`, then what is done, what is left, the gate results and the NEEDS THE MAC list.
A message that says HOLD means: finish the step you are in, commit, push, end the turn and wait.
