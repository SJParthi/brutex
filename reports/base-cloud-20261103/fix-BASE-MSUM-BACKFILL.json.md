# fix-BASE-MSUM-BACKFILL.json.md

Source: `state/fix/round2/fix-BASE-MSUM-BACKFILL.json` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````json
{
  "name": "BASE-MSUM-BACKFILL",
  "branch": "fix/db-BASE-MSUM-BACKFILL",
  "worktree": "/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/wt/P-BASE-MSUM-BACKFILL",
  "head": "4d7f446f7a816364e4dc55fd51b3580f5c3dca4a",
  "committed": false,
  "held": false,
  "nothing_to_do": false,
  "findings": [
    {"id": "O1L-04", "status": "not_fixed", "how": "NOT PROVED. No store step exists yet and nothing was compiled or rehearsed. Code for the run is on the branch as WIP."},
    {"id": "O1L-08", "status": "not_fixed", "how": "NOT PROVED, same cause."},
    {"id": "o1s-16", "status": "not_fixed", "how": "NOT PROVED, same cause."},
    {"id": "o1r-N1", "status": "not_fixed", "how": "NOT PROVED, same cause."},
    {"id": "AL-10", "status": "not_fixed", "how": "NOT PROVED, same cause."},
    {"id": "AA-07", "status": "not_fixed", "how": "NOT PROVED, same cause."}
  ],
  "gates": {
    "fmt": "cargo fmt --all ran clean at 65725cae1; --check not re-run at the tip",
    "clippy": "NOT RUN",
    "tests": "NOT RUN: nothing on this branch has been compiled",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "coverage": "NOT MEASURED",
  "mutants": "deferred (operator 30 Oct)",
  "p50_p99": [],
  "files_changed": [
    "crates/cli/src/msum_builder.rs",
    "crates/cli/src/upkeep_command.rs",
    "crates/cli/src/upkeep_command_tests.rs",
    "crates/pull/src/upkeep.rs",
    "crates/pull/src/upkeep_tests.rs",
    "crates/store/src/file_fault_tests.rs",
    "crates/store/src/month_summary_tests.rs",
    "crates/store/src/month_summary_writer.rs",
    "crates/store/src/sort_index_writer.rs"
  ],
  "files_outside_ownership": [
    "crates/store/src/month_summary_writer.rs",
    "crates/store/src/sort_index_writer.rs",
    "crates/store/src/month_summary_tests.rs",
    "crates/store/src/file_fault_tests.rs"
  ],
  "store_writes_needed": [
    "NOT REHEARSED, not ready to run: cli upkeep run msum gdfl NSE/INDEX --workers 8; then NSE/CASH; then NSE/FNO",
    "NOT REHEARSED, not ready to run: cli upkeep run msum zerodha --workers 8"
  ],
  "notes": "PACKAGE NOT DONE. Stopped by the build queue after about 4 h: tgt/pool-5.lock was held from 19:07 to at least 22:43 IST by BASE-COV-LIBS, whose cargo calls queue for hours (its progress file: 'lottery of 5 low slots, 27 callers, not on the priority file'; about 60 bin/cargo callers were waiting), and tgt/pool-5 does not exist, so the first build is cold. Each cargo call here would cost about 2 h more, so polling on was dearer than a relaunch. Nothing on this branch has been compiled or run, so no result in this report is a proof. Commits 4d7f446f7 are WIP and must not merge as they stand. NEEDS FROM THE MAIN SESSION: add this package to state/cargo-priority.txt and give it a free pool, then relaunch; the resume reads state/fix/round2/fix-BASE-MSUM-BACKFILL-progress.md, whose last NEXT line holds the plan. DEFECTS FOUND BY READING, each with a test written but not run: (1) month_summary_existing held the month lock exclusive, so a reader refused the build and a reader waited past READ_PATIENCE_MS on the biggest months, which is the 503 the spec forbids; changed to a shared hold plus a flock on the bar file between doors, which touches crates/store (outside ownership, needed for that NEVER). (2) the door renamed with no sync; it now syncs first. (3) a small month was listed as lacking forever, so a plan could never read 0; the walk now reads the bar file's length, for .sum only. GOAL (3) NEEDS NARROWING: a month under 1,024 bars keeps no .sum by D-4933 and an index month with sparse seconds still reads its bars, so 'every held month' can only mean every month of 1,024 bars or more with a usable summary; how many sparse months exist is NOT MEASURED. GATE 30 is already red at feat/db-fix f01d7a0e4 on nine packages marked merged = false (R3-17 among them), which is BASE-REDS-1023's; a step row with figures cannot name R3-17 as a prerequisite until that is fixed, so the planned rows name R3-06 and an operator yes. NOT DONE: compiling and running every test; config/store-steps.toml rows; docs/04, docs/05 D-7430, docs/06; all rehearsal; every /gaps.json measurement; all gates; coverage; shadow step 5. An early regex substitution mangled the edited sources; they were restored from feat/db-fix and redone by hand before 65725cae1, and b5a4ff2a4 still holds the mangled text in history (the merge squash folds it). SCRATCH LEFT IN PLACE: shadow/MSUM-g1 (26,208 bar files, 0 .sum), shadow/MSUM-z1 (113,204 bar files, 0 .sum), state/fix/round2/logs-MSUM/. No lock of this package is held and no process of it is left running. Second-agent self-check 5b not done, because there is no build to check. The real store was only read; no server was started."
}
`````
