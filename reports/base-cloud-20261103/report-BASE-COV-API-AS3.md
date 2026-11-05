STATUS: IN PROGRESS

# BASE-COV-API-AS3 (shard G3, repair r3, 5 Nov 2026): working report

This file is rewritten as the session goes; line 1 is the state of the branch now.

- Branch `cloud/cov-api-as3-20261105`, cut from the kit commit `f4111a0132473d8771d8624a9538b1362926f70b`;
  START `6ea46e59c07488eeccff388407976041852aae2c`.
- Session 1 (token `BASE-COV-API-AS3-20261105-1`) pushed up to `7700e2f0`; the main session paused the shard at
  `edd54703` (token `PAUSED-BY-MAIN-20261105-1420`). This continuation holds token `BASE-COV-API-AS3-20261105-3`
  (`a5f009a8`) and resumed from `edd54703`.
- Host: 4 cores, 15 GB, root, Linux x86_64, 30 GB free at start. Pinned nightly installed as `nightly-2026-08-08`:
  `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, `LLVM version 23.1.0-rust-1.99.0-nightly` (llvm-cov and
  llvm-profdata). Dev loop: the api lib tests built with Route B's flags (`-C instrument-coverage -Z
  coverage-options=branch`, opt-level 0) and only the tests of the files at hand run, each kit GAP row then
  checked against that run's uncovered rows. These are dev figures; the measurement of record is Route B.

## Done so far (pushed)

- Session 1: verify r3 item (3) for `bars_series.rs` and `feeds_route.rs`; GAP rows of `bars_series.rs`,
  `feeds_route.rs`, `serve_edge_tests.rs`, `ops_safety_tests.rs`, `server_cloud_mutant_tests_b.rs`, `_c.rs`, `_d.rs`.
- This session, `60603ada` (dev run on a stamped build: 188 passed, 0 failed):
  - `indexstoplaunch.rs` 70 of 71 kit rows closed (open: 345, the `Ok(report)` arm of a completed run);
    `indexstoplaunch_metadata.rs` 24 of 25 (open: 52:20 False, (U)); `indexstoplaunch_tests.rs` 6 of 6.
  - `frontierjson.rs` 24 of 46 (open: 193-196 and 265-274, (U) with proofs; 300:25 False and 310-316, reachable
    only by a change of the frontier path between the preflight and the absence check, so no test can order it
    without a seam).
  - Test code with a branch no run takes, rewritten to assert the same with none left: `gaps_route.rs` (3 of its
    10 rows; the other 7 are (U)), `owed_tests.rs` 3/3, `nse_identity.rs` 4/4, `booleanevidencejson_tests.rs` 9/9,
    `http_admission_tests.rs` 5/5, `pool_table_tests.rs` 27/27, `census_request_tests.rs` 17/17.
  - A host defect in a test of this shard's file, fixed: `gaps_route::cloud_mutant_tests::a_full_calendar_pool_
    refuses_the_census_and_the_audit` held `MAX_CALENDAR_CONCURRENT` (8) slots, but the pool has one slot a core up
    to 8; on this 4-core host four holders were refused and the child waited on its barrier for ever (seen twice,
    gdb showed four holders inside their slots). It now holds `Named::Calendar.pool().max()` slots and waits until
    every holder is inside its slot before it asks the pool.
- `57aa875a` (helper worktree c, cherry-picked): `census_rollup.rs` 34/35, `ingest.rs` 31/37, `ingest_refusals.rs`
  7/7, `imports.rs` 49/54 by the same dev check.

## In progress

- Helpers in their own worktrees: dayflags group; logs and sweeprun group; bars and imports group.
- L-BCA-5 probe: a child test runs the production launch (`sweeprun::command_with`) over a generated NIFTY store
  on a stamped build, then reads the saved qualification back through `/index-stop-qualification.json`.

## Not yet

- Route B whole:api on the code tip, `gaps-AS3.txt.md`, `fu-AS3.txt.md`, the gates on the tip,
  `fix-BASE-COV-API-AS3.json.md`, the red-under-hand-change evidence of this session's new tests.
