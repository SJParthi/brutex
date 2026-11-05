STATUS: IN PROGRESS

# BASE-COV-API-AS3 (shard G3, repair r3, 5 Nov 2026): working report

This file is rewritten as the session goes; line 1 is the state of the branch now.

- Branch `cloud/cov-api-as3-20261105`, cut from the kit commit `f4111a0132473d8771d8624a9538b1362926f70b`;
  START `6ea46e59c07488eeccff388407976041852aae2c`.
- Session 1 (token `BASE-COV-API-AS3-20261105-1`) pushed up to `7700e2f0`; the main session paused the shard at
  `edd54703` (token `PAUSED-BY-MAIN-20261105-1420`). This continuation holds token `BASE-COV-API-AS3-20261105-3`
  (`a5f009a8`) and resumed from `edd54703`.
- Host: 4 cores, 15 GB, root, Linux x86_64. Pinned nightly installed as `nightly-2026-08-08`:
  `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, `LLVM version 23.1.0-rust-1.99.0-nightly` (llvm-cov and
  llvm-profdata). Dev loop: the api lib tests built with Route B's flags (`-C instrument-coverage -Z
  coverage-options=branch`, opt-level 0), the tests of the files at hand run, and each kit GAP row checked against
  that run's uncovered rows (dev figures; the measurement of record is Route B, still to run). Three helper
  sessions work file groups in worktrees of their own; their commits are cherry-picked here after a combined run.

## Kit GAP rows closed so far by the dev check (closed / kit rows; open rows with their reason)

| file | closed | open, why |
|---|---|---|
| `indexstoplaunch.rs` | 70 / 71 | 345 the `Ok(report)` arm: now run by the saved-qualification probe (to confirm in Route B) |
| `indexstoplaunch_metadata.rs` | 24 / 25 | 52:20 False (U) |
| `indexstoplaunch_tests.rs` | 6 / 6 | |
| `indexstopqualificationjson.rs` | 199 / 216 | (U) 152:9 F, 154:16 F; the 8 MiB page (114-119); the ranking budget (210/215); other verdict statuses (235-237); `display` (allocation failure only); 317 |
| `indexstopcandlesjson.rs` | 94 / 113 | being read |
| `indexstopjson.rs` | 5 / 12 | being read |
| `frontierjson.rs` | 24 / 46 | (U) 193-196 and 265-274; 300:25 F and 310-316 only by a concurrent change of the frontier path |
| `gaps_route.rs` | 3 / 10 | the 7 production rows are (U) |
| `dayflags.rs` | 213 / 232 | 15 (U), 3 race-only (3220:12 F, 3322:16 F, 3323:16 F), 1 now an (F) line |
| `dayflags_filter_tests.rs`, `month_change.rs` | 3 / 3, 2 / 2 | |
| `ladder.rs` | 5 / 8 | 3 production rows (U) |
| `census_rollup.rs`, `ingest.rs`, `ingest_refusals.rs`, `imports.rs` | 34/35, 31/37, 7/7, 49/54 | helper c's report pending |
| `owed_tests.rs`, `nse_identity.rs`, `booleanevidencejson_tests.rs`, `http_admission_tests.rs`, `pool_table_tests.rs`, `census_request_tests.rs` | 3/3, 4/4, 9/9, 5/5, 27/27, 17/17 | |

## L-BCA-5, reached through the production path

`indexstopqualificationjson_saved_tests.rs` (declared at the end of `indexstopqualificationjson.rs`) writes three
generated NIFTY months with the store's own writer, then in a child process with bounded settings runs the real
`/backtest/command` launch (`sweeprun::command_with`) of the native single-stop search: strict source load, search,
qualification and daily assessments all saved by cli's production path in about 12 s at opt-level 0. It then reads
every page back: settings, a kept reader, folds, index days and weeks, the refusals, a saved body changed under the
kept reader (stale, then readmitted when restored), the single-stop source pages and the candle reader. On a build
with no commit stamp (a dirty tree) the launch is refused and the test asserts that refusal: the measured build is
stamped (cli's build.rs stamps a clean tree), so that one arm is (U) for the measured object. A second probe
(`booleanevidencejson_saved_tests.rs`) launches the qualified Boolean search over the same months: in progress.

## Also fixed

- `gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit` held
  `MAX_CALENDAR_CONCURRENT` (8) slots, but the pool has one slot a core up to 8: on this 4-core host the child hung on
  its barrier (gdb: four holders inside their slots). It now holds `Named::Calendar.pool().max()` slots and waits until
  each is inside its slot before it asks the pool (D-8664 draft).
- `feeds_route.rs` test code from session 1: two clippy refusals (`too_many_lines`, `useless_format`).

## Not yet

- Helpers b (logs, sweeprun group) and c (bars, imports group) still working; helper a now on `optiongreeks.rs`,
  `mastersrun.rs`, `autopilot.rs`. `selectionv6json.rs` not started.
- Route B whole:api on the code tip, `gaps-AS3.txt.md`, `fu-AS3.txt.md`, the gates on the tip,
  `fix-BASE-COV-API-AS3.json.md`, the red-under-hand-change evidence of the main session's new tests.
