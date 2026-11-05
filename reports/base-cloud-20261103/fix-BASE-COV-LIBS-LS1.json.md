# fix-BASE-COV-LIBS-LS1.json.md

The session's structured report in the launcher's FIX shape, one json block (crate `store`; second continuation of 5 Nov 2026).

```json
{
  "name": "BASE-COV-LIBS-LS1",
  "branch": "cloud/cov-libs-ls1-20261103",
  "head": "24bd3a62cd51abb1c7471d5e2edf64b5ca677d8d",
  "committed": true,
  "findings": [
    {
      "id": "LS1-ROOT-REDS",
      "status": "fixed",
      "how": "Seven store lib tests failed only as root (two from attempts 2-4, five base tests); each runs its unchanged body through crate::support::where_permission_binds, which re-runs it as nobody under root and in the same uid otherwise (the D-0995 pattern). (3 Nov, unchanged.)",
      "evidence": "Commit c7ccb0f7; at the code tip 24bd3a62, cargo test -p store --locked --no-fail-fast as root: exit 0, 1104 passed, 0 failed, 8 ignored."
    },
    {
      "id": "LS1-HOST-RACE",
      "status": "fixed",
      "how": "The 61 production rows only a host refusal or a race reaches (the main session's answer, D-8561 draft): host refusals through cfg(test) one-shot per-thread seams in file.rs's pattern (file::tests::refusal by operation word and path, consulted by generation/quarantine/repair io_at, provenance write_fully and settings' staged write; flock::tests::take_host_refusal for the registered flock attempt, try_lock_shared, try_lock_noted's fstat and the note's clear); races through file::tests::race, a hook at the moment between two host calls through which the test makes the real change on disk (14 points). quarantine.rs:831, :861 and :2224, filed as host refusals, are reached by real races instead. One test per row, each red under a hand change.",
      "evidence": "Commits 095165bb, 24bd3a62; Route B at 24bd3a62: none of the 61 rows is uncovered (fu-LS1.txt.md lists each with its test). Red checks: batch 1, 31 tests, all FAILED under their hand changes, files restored, git status clean."
    },
    {
      "id": "LS1-NEW-ROWS",
      "status": "fixed",
      "how": "Route B at 095165bb found three more production rows: repair.rs's reservation and receipt create_new (the 5 Nov list had called them flushes and a write), driven by a revision directory that takes no new name (mode 0o555 where the bits bind, D-0995) and by another actor making the receipt first (the race hook); and staging.rs's leftover that vanishes between the look and its removal, which a racing base test hit only sometimes, now driven by a race hook.",
      "evidence": "Commit 24bd3a62; Route B at 24bd3a62: production rows 81, all (U) with their 5 Nov proofs, 0 GAP."
    },
    {
      "id": "LS1-TEST-CODE",
      "status": "fixed",
      "how": "Test-code rows (the answer to Q2): a test's or check module's own bounding, skip, failure-collection, fault-mock-guard or failing arm is (F) by recipe step 4's second clause with its proof (97 rows); regions no run reaches are (U) with proofs (14); macOS-only arms are owed to the Mac (10, tests/catalog.rs); every other arm got a direct test of its helper (the D-0693 parser, the civil-day count, the empty month's probes, the telemetry value readers, a formatter mock, a mock device's flush, the settings check's passing side, the operation seam's own sides).",
      "evidence": "Route B at 24bd3a62: 0 test-code GAPs (fu-LS1.txt.md). Red checks: batch 2, 14 tests, all FAILED under their hand changes, restored."
    },
    {
      "id": "LS1-TEST-FIXES",
      "status": "fixed",
      "how": "tests/docs.rs's D-0790 check was gated on a sentence D-1448 corrected away, so it checked nothing; it now runs on every run (both facts it checks hold). safe_open's denied-file test asserted nothing as root; it now runs where the mode bits bind (where_permission_binds).",
      "evidence": "Commit 24bd3a62; both red under a hand change (the D-0790 text edited locally and restored; the denied refusal named as a kind)."
    },
    {
      "id": "LS1-COV-STORE",
      "status": "fixed",
      "how": "Route B on this host: every uncovered row is a (U) with its proof, an ignored-test body, an (F) item or a macOS row owed to the Mac.",
      "evidence": "Route B, pinned nightly, reading whole:store, tip 24bd3a62: lines 53276/54692 (1416 uncovered), sides 3448/3990 (542 uncovered), 112 files read; production 81 rows, all (U); test code 1877 rows: ignored 1108, (F) by pattern 648, step-4 arms 97, (U) 14, macOS 10, GAP 0. Lists: gaps-LS1.txt.md, fu-LS1.txt.md."
    },
    {
      "id": "LS1-SAFE-OPEN-MAC",
      "status": "not_fixed",
      "how": "safe_open.rs:104:19 False and :107 (Found::of's last arm, a file of none of the seven kinds) is covered on Linux by a real anonymous inode (safe_open::tests::a_file_of_none_of_the_seven_kinds_is_unknown) and cannot be reached by any test on aarch64-apple-darwin (no FileType constructor; the only such kind there, a union mount's whiteout, cannot be made; shm_open needs unsafe code). STATUS: BLOCKED QUESTION: (A) (U) on the Mac's target with that proof (recommended), or (B) restructure Found::of over the mode bits (rejected by D-6017).",
      "evidence": "report-BASE-COV-LIBS-LS1.md, QUESTION."
    }
  ],
  "files_changed": [
    "crates/store/src/catalog_cover_tests.rs",
    "crates/store/src/checksum_audit.rs",
    "crates/store/src/checksum_audit_cover_tests.rs",
    "crates/store/src/file.rs",
    "crates/store/src/file_cover_tests.rs",
    "crates/store/src/file_index_cover_tests.rs",
    "crates/store/src/file_sealed.rs",
    "crates/store/src/file_sealed_cover_tests.rs",
    "crates/store/src/flock.rs",
    "crates/store/src/flock_cover_tests.rs",
    "crates/store/src/flock_tests.rs",
    "crates/store/src/generation.rs",
    "crates/store/src/generation_door_cover_tests.rs",
    "crates/store/src/month_summary.rs",
    "crates/store/src/month_summary_decode_cover_tests.rs",
    "crates/store/src/month_summary_writer.rs",
    "crates/store/src/month_summary_writer_cover_tests.rs",
    "crates/store/src/provenance.rs",
    "crates/store/src/provenance_cover_tests.rs",
    "crates/store/src/quarantine.rs",
    "crates/store/src/quarantine_cover_tests.rs",
    "crates/store/src/quarantine_tests.rs",
    "crates/store/src/repair.rs",
    "crates/store/src/repair_door_cover_tests.rs",
    "crates/store/src/safe_open_tests.rs",
    "crates/store/src/sealed.rs",
    "crates/store/src/sealed_fold_cover_tests.rs",
    "crates/store/src/sealed_tests.rs",
    "crates/store/src/settings.rs",
    "crates/store/src/settings_door_cover_tests.rs",
    "crates/store/src/settings_tests.rs",
    "crates/store/src/staging.rs",
    "crates/store/src/staging_cover_tests.rs",
    "crates/store/tests/cited_commits.rs",
    "crates/store/tests/docs.rs",
    "crates/store/tests/misfiled.rs",
    "crates/store/tests/month_summary_log.rs",
    "crates/store/tests/slot_damaged_log.rs",
    "crates/store/tests/sort_index_log.rs",
    "crates/store/tests/tix.rs",
    "crates/store/tests/tix_rebuilder.rs",
    "crates/store/tests/tix_small_month.rs",
    "crates/store/tests/tix_warn_once.rs",
    "reports/base-cloud-20261103/OWNER-BASE-COV-LIBS-LS1.md",
    "reports/base-cloud-20261103/fix-BASE-COV-LIBS-LS1.json.md",
    "reports/base-cloud-20261103/fu-LS1.txt.md",
    "reports/base-cloud-20261103/gaps-LS1.txt.md",
    "reports/base-cloud-20261103/report-BASE-COV-LIBS-LS1.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "pass (cargo fmt --all --check exit 0 at 24bd3a62)",
    "clippy": "pass (cargo clippy -p store --all-targets --locked -- -D warnings exit 0 at 24bd3a62)",
    "tests": "pass (cargo test -p store --locked --no-fail-fast exit 0 at 24bd3a62: 1104 passed, 0 failed, 8 ignored, 43 result lines, 145 s; Linux x86_64 root host, 4 cores, TMPDIR a mode-1777 tmpfs)",
    "web_build": "not applicable",
    "web_test": "not applicable"
  },
  "p50_p99": "none measured: no production behaviour changed (cfg(test) seams and hooks; four shipped statements reshaped with the same calls in the same order), no bench run. Route B wall at the final tip: tests 22 min (this host).",
  "coverage": "Route B (kit d3-route-b-cloud.sh unchanged, rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0), reading whole:store, tip 24bd3a62cd51: lines 53276/54692 (1416 uncovered), branch sides 3448/3990 (542 uncovered), 112 files read; NOT READ: benches/ratio.rs, benches/ratio/{months,quarantine,safe_files}.rs (test = false) and src/lib.rs (no executable line). Production rows: 81, all (U) with proofs, 0 GAP; test code: 0 GAP. Lists: reports/base-cloud-20261103/gaps-LS1.txt.md, fu-LS1.txt.md. macOS-only lines owed to the Mac's Route B; safe_open.rs:104/107 on the Mac is the question.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Second continuation of 5 Nov 2026 (token BASE-COV-LIBS-LS1-20261105-3), with the main session's answer to the first continuation's question. Line 1 of the report: STATUS: BLOCKED QUESTION, for one row only: safe_open.rs:104:19 False and :107 on aarch64-apple-darwin ((A) (U) with its proof, recommended; (B) restructure, rejected by D-6017). Decision drafts: D-8560 (figures, lists, root re-runs, fresh-process children), D-8561 (the seams and race hooks, the reclassified rows, the test-code classes, two test fixes). NEEDS THE MAC: Route B on aarch64-apple-darwin (macOS-only lines: file.rs:7414 and :10609 at this tip, open_flags.rs:33-122, tests/catalog.rs's 10 APFS rows), a non-root 14-core run, the ignored real-data, timing and full-volume tests (L-BCL-3, L-PREC1-01), the D1 landing.",
  "held": false
}
```
