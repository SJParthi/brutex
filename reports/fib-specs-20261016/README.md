# Fib Flag Breakout engine: cloud build rules (16 Oct 2026)

These 14 files are the specifications for the Fib Flag Breakout engine, one per part.
Each part is built by its own cloud session on its own branch, and merged back into
`feat/db-fix` on the operator's Mac. This folder is planning material: it is never
merged into `feat/db-fix`.

## Order (dependencies)

| Wave | Parts | Starts from |
|---|---|---|
| 1 | FIB-1 | `cloud/fib-base-20261016` |
| 2 | FIB-2, FIB-3, FIB-5, FIB-6, FIB-8 | `cloud/fib1-20261016` once FIB-1's report says `STATUS: DONE` |
| 3 | FIB-4 (after FIB-3), FIB-7 (after FIB-6) | the branch of the part it depends on |
| 4 | FIB-9 | FIB-2, FIB-4, FIB-5, FIB-7 and FIB-8 merged together |
| later | FIB-10 .. FIB-14 | on the Mac: they need the stored data and the new-work parts |

## What changes in the cloud

1. **Paths.** The specs name folders on the operator's Mac (`/Volumes/WD_BLACK/...`,
   `wt/DBFIX`, `tgt/`, `state/`, a `bin/cargo` wrapper). None exist here. Work in your
   own checkout, use plain `cargo`, and let the target folder default.
2. **Real data.** There is no market data store in the cloud. Every step marked
   `REAL DATA` is built and tested on synthetic fixtures here; list each real-data check
   in your report under **NEEDS THE MAC**, with the exact command, so the Mac runs it
   at merge-back. Never invent a measurement.
3. **Benches.** Run the benches here and report p50, p99 and max at every size, naming
   this host. The Mac re-measures at merge-back.

## Rules

- `CLAUDE.md` is law: Rust only outside `web/`, O(1) per operation and measured at p99,
  no look-ahead, append-only, no depth parameter, honest limits, no fallback that hides a
  failure.
- Never use the Py-language and never write its name anywhere (files, commits, reports).
- Say "order flow delta" and "order flow cumulative delta", never the bare word.
- Only the Winner Gate numbers are fixed; every other value is generated from the data.
- Decision numbers: use the block your specification gives, in order, append-only.
- Gates before you call a part done: `cargo fmt --check`;
  `cargo clippy --workspace --all-targets -- -D warnings`;
  `cargo test --workspace --locked`; `cargo deny check`; 100% line and branch coverage
  and no surviving mutant on the modules you touched. If a tool cannot be installed here,
  say so plainly in the report.
- Push only your own `cloud/fib<N>-20261016` branch. Never push `main` or `feat/db-fix`,
  never open a pull request, never delete anything.

## Turn protocol

Work in turns of at most 20 minutes. At the end of every turn: commit, push your branch,
update `reports/cloud-fib<N>-20261016.md`, end the turn, and wait for a message that
says CONTINUE. The report's first line is `STATUS: DONE` only when the part is complete
and every gate is green; otherwise `STATUS: IN PROGRESS`, followed by what is done, what
is left, the gate results and the NEEDS THE MAC list. A message that says HOLD means:
finish the step you are in, commit, push, end the turn and wait.
