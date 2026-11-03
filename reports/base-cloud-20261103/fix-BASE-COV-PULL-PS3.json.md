# fix-BASE-COV-PULL-PS3.json.md

The launcher FIX shape for session BASE-COV-PULL-PS3; head is the code tip (`git rev-list -1 HEAD -- . ":(exclude)reports"`).

```json
{
  "name": "BASE-COV-PULL-PS3",
  "branch": "cloud/cov-pull-ps3-20261103",
  "head": "a5ac2140fcac02e6b123b5661cab56ecbd731405",
  "committed": true,
  "findings": [
    {
      "id": "PS3-G3-bar",
      "status": "partial",
      "how": "Route B, G3 71 files: start 2e7a0b6d lines 49318/51591 (2273 uncovered), branches 3865/4792 (927 sides); tip a5ac2140 lines 58751/60007 (1256 uncovered), branches 4289/4792 (503 sides). By class at the tip: 753 (F), 898 L-BCP-3, 91 (U) with proofs, 17 gaps (11 git-history-dependent sides of gdfl_import_split_tests, 3 of gdfl_nfo_attack_tests dpn_13, 1 race arm fixed after the tip and not yet measured, tenor.rs 203-204 blocked outside). The bar is not yet held for G3.",
      "evidence": "reports/base-cloud-20261103/gaps-PS3.txt.md, fu-PS3.txt.md, report-BASE-COV-PULL-PS3.md (Coverage)"
    },
    {
      "id": "PS3-host-reds",
      "status": "fixed",
      "how": "G3s six root-only reds of PS1s list run through where_permission_binds (D-0995) and pass as root",
      "evidence": "commit d418a3ec; gate run on a5ac2140: none of the six fails"
    },
    {
      "id": "PS3-defect-staging-lock",
      "status": "fixed",
      "how": "gdfl_dropbox StagingLock Drop calls File::unlock(): the flock outlived its press in a forked childs copy of the description (103/300 refused before, 0/300 after); the cause of a duplicate-refusal test red once under load",
      "evidence": "commit 454a6797; test a_staging_lock_is_released_while_a_copy_of_its_description_lives"
    },
    {
      "id": "PS3-defect-day-lock",
      "status": "fixed",
      "how": "tickstore_write DayLock Drop calls File::unlock() (149/300 refused before, 0/300 after)",
      "evidence": "commit b82b4f02; test the_day_lock_is_released_by_its_run_not_by_the_last_copy_of_its_file"
    },
    {
      "id": "PS3-defect-rolling-words",
      "status": "fixed",
      "how": "rolling step_refused: a roll past the calendars last day (OrdinalOutsideWindow, YearOutsideWindow) says so instead of before the regime",
      "evidence": "commit 194b5150; test an_expiry_past_the_last_measured_day_is_filed_unchecked red on the old code"
    },
    {
      "id": "PS3-test-archive-root",
      "status": "fixed",
      "how": "a http test kept vendor captures under $HOME/.brutex/vendor-data/captures; it now uses an invented base URL and asserts feed == None",
      "evidence": "commit b82b4f02"
    },
    {
      "id": "PS3-tenor-outside",
      "status": "needs_operator",
      "how": "tenor.rs 203-204 (TenorError::HoursUnverified words) need a SessionRefusal, whose fields are private to G1s vendor.rs: one test there (exact text in the report)",
      "evidence": "report, The 17 gaps at the tip"
    },
    {
      "id": "PS3-remaining-test-gaps",
      "status": "not_fixed",
      "how": "gdfl_import_split_tests (11, git-history-dependent sides: needs a reader seam), gdfl_nfo_attack_tests dpn_13 (3), cash_session_cache_lock_tests 232 (fixed in the tree after a5ac2140, not yet measured)",
      "evidence": "gaps-PS3.txt.md GAP lines"
    },
    {
      "id": "L-BCP-3",
      "status": "needs_operator",
      "how": "898 items inside G3s #[ignore] real-data and measurement tests and seven helpers only they call",
      "evidence": "fu-PS3.txt.md IGNORED"
    }
  ],
  "files_changed": [
    "crates/pull/src/archive.rs",
    "crates/pull/src/archive_cov_tests.rs",
    "crates/pull/src/gap_backfill.rs",
    "crates/pull/src/gap_backfill_cov_tests.rs",
    "crates/pull/src/gaps.rs",
    "crates/pull/src/gaps_cov_tests.rs",
    "crates/pull/src/gdfl_archive.rs",
    "crates/pull/src/gdfl_cm_admission_tests.rs",
    "crates/pull/src/gdfl_dropbox.rs",
    "crates/pull/src/gdfl_dropbox_cov_tests.rs",
    "crates/pull/src/gdfl_dropbox_tests.rs",
    "crates/pull/src/gdfl_ia16_tests.rs",
    "crates/pull/src/gdfl_import_journal.rs",
    "crates/pull/src/gdfl_import_journal_cov_tests.rs",
    "crates/pull/src/gdfl_import_lineage.rs",
    "crates/pull/src/gdfl_import_lineage_cov_tests.rs",
    "crates/pull/src/gdfl_import_split_tests.rs",
    "crates/pull/src/gdfl_lineage_tests.rs",
    "crates/pull/src/gdfl_nfo_attack_tests.rs",
    "crates/pull/src/gdfl_run_ia2_tests.rs",
    "crates/pull/src/gdfl_run_scope.rs",
    "crates/pull/src/gdfl_run_scope_cov_tests.rs",
    "crates/pull/src/gdfl_run_start_cov_tests.rs",
    "crates/pull/src/gdfl_tickstore.rs",
    "crates/pull/src/gdfl_tickstore_cov_tests.rs",
    "crates/pull/src/gdfl_wp12e_tests.rs",
    "crates/pull/src/http.rs",
    "crates/pull/src/http_cov_tests.rs",
    "crates/pull/src/ingest.rs",
    "crates/pull/src/ingest_failkind_tests.rs",
    "crates/pull/src/ingest_gap_tests.rs",
    "crates/pull/src/masters.rs",
    "crates/pull/src/masters_cov_tests.rs",
    "crates/pull/src/masters_land_tests.rs",
    "crates/pull/src/resolve.rs",
    "crates/pull/src/rolling.rs",
    "crates/pull/src/session.rs",
    "crates/pull/src/tickstore_write.rs",
    "crates/pull/src/tickstore_write_tests.rs",
    "crates/pull/src/totp.rs",
    "crates/pull/src/zerodha_verify.rs",
    "crates/pull/src/zerodha_verify_cov_store_tests.rs",
    "crates/pull/src/zerodha_verify_cov_tests.rs",
    "crates/pull/src/zerodha_verify_tests.rs",
    "crates/pull/tests/derive.rs",
    "crates/pull/tests/no_scheduler_delete.rs",
    "crates/pull/tests/pipeline.rs"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 on a5ac2140",
    "clippy": "cargo clippy -p pull --all-targets --locked -- -D warnings rc 0, no diagnostic, on a5ac2140 (stable 1.97.1)",
    "tests": "cargo test -p pull --locked --no-fail-fast on a5ac2140 (stable 1.97.1, repo profile): 32 binaries, 3197 passed, 3 failed (root-only reds in G2 files, PS1s proof: they pass as uid 65534), 54 ignored, 208 s",
    "route_b_tests": "instrumented run at a5ac2140: 3197 passed, 3 failed (the same), 54 ignored",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "D3 coverage pull route B (G3 shown): start 2e7a0b6d 11 min (build 1, tests 4, shows 6); tip a5ac2140 13 min (build 1, tests 4, shows 8); this hosts figures, 4 cores",
    "no bench run: no new runtime step (seams pass the live value; two lock guards add one unlock per run)"
  ],
  "coverage": {
    "route": "B (kit d3-route-b-cloud.sh, pin rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0)",
    "reading": "whole:pull, G3s 71 files shown",
    "start": "2e7a0b6d: lines 49318/51591, branches 3865/4792",
    "tip": "a5ac2140: lines 58751/60007, branches 4289/4792",
    "classes_tip": {
      "F": 753,
      "U": 91,
      "IGNORED_L_BCP_3": 898,
      "GAP": 17
    },
    "lists": [
      "reports/base-cloud-20261103/gaps-PS3.txt.md",
      "reports/base-cloud-20261103/fu-PS3.txt.md"
    ],
    "not_proved": 0
  },
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Decision draft D-7337 in the report. Six helper subagents worked in detached worktrees; each diff was checked and its tests, fmt and clippy run here before its commit. Process slips recorded in the report: the forbidden language name appeared in shell commands three times (nothing ran with it, no file holds it) and one helper used perl -pi as an editor. NEEDS THE MAC: Route B whole:pull on its pin, macOS lines, the shadow GET re-check, macOS rows, non-root 14-core tests, L-BCP-3, the D1 landing, plus the platform items listed in the report.",
  "held": false
}
```
