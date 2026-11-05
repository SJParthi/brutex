# fix-BASE-COV-LIBS-LS1.json.md

The session's structured report in the launcher's FIX shape, one json block (crate `store`; continuation of 5 Nov 2026).

```json
{
  "name": "BASE-COV-LIBS-LS1",
  "branch": "cloud/cov-libs-ls1-20261103",
  "head": "4b3ee558a38fbc65f6c0dfcf59893cff750568c0",
  "committed": true,
  "findings": [
    {
      "id": "LS1-ROOT-REDS",
      "status": "fixed",
      "how": "Seven store lib tests failed only as root (two from attempts 2-4, five base tests); each runs its unchanged body through crate::support::where_permission_binds, which re-runs it as nobody under root and in the same uid otherwise (the D-0995 pattern). (3 Nov, unchanged.)",
      "evidence": "Commit c7ccb0f7; at the code tip 4b3ee558, cargo test -p store --locked --no-fail-fast as root: exit 0, 1064 passed, 0 failed, 8 ignored."
    },
    {
      "id": "LS1-COV-STORE",
      "status": "not_fixed",
      "how": "36 new asserting tests on 5 Nov (72 over both sessions), each red under a hand change; every production row an input, file, folder, variable or direct call reaches now has one (direct calls against the function's own contract where every public path refuses the input first; the file.rs host-fault seam and the flock.rs lock/unlock seams where they reach the call; a fresh child process of the test binary where a process-wide static must be fresh). No production line changed.",
      "evidence": "Route B on the pinned nightly, whole crate: 3 Nov final tip fc9b489f0e3c lines 51094/52629, sides 3351/3916; 5 Nov final tip 4b3ee558a38f lines 52295/53771 (1476 uncovered), sides 3405/3964 (559 uncovered); production rows 239 down to 142 (94 lines + 48 sides): 81 (U) with proofs, 61 reached only by a host refusal (28) or a race (33), 0 unexplained. Test code: 1108 ignored-test rows, 641 (F) by pattern, 144 test-code GAPs. Lists: gaps-LS1.txt.md, fu-LS1.txt.md. Bar NOT met: STATUS BLOCKED QUESTION (seams or limits for the 61 rows; (F) or restructure for the 144)."
    },
    {
      "id": "LS1-TEST-FLAW",
      "status": "fixed",
      "how": "The 3 Nov stale-summary test left n_valid 0 in its crafted summaries; each case carries its row count.",
      "evidence": "Commit fc9b489f (3 Nov)."
    },
    {
      "id": "LS1-TEST-FLAW-2",
      "status": "fixed",
      "how": "The 3 Nov floor test's \"95MB\" case was refused as an unknown unit before the percent clause, so its assertion proved nothing about that clause; it now asserts that 1GB is the floor itself and that MB is refused as no unit, and settings.rs:688:30 False is a (U) with its proof.",
      "evidence": "Commit 4b3ee558; red when the floor comparison < becomes <=."
    },
    {
      "id": "LS1-CLASSIFY",
      "status": "fixed",
      "how": "The 3 Nov lists filed verdict.rs:2278, :2312 and :2478 (production lines; the test modules start at verdict.rs:4178) under test code; the 5 Nov lists file them as production (U) with proofs, and carry the 3 Nov (U) proofs only for rows still uncovered (file_sealed.rs:460-461 are now covered by a direct-call test).",
      "evidence": "fu-LS1.txt.md (U) ITEMS."
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
    "crates/store/src/generation.rs",
    "crates/store/src/generation_door_cover_tests.rs",
    "crates/store/src/month_summary.rs",
    "crates/store/src/month_summary_decode_cover_tests.rs",
    "crates/store/src/month_summary_writer.rs",
    "crates/store/src/month_summary_writer_cover_tests.rs",
    "crates/store/src/provenance.rs",
    "crates/store/src/provenance_cover_tests.rs",
    "crates/store/src/quarantine_cover_tests.rs",
    "crates/store/src/quarantine_tests.rs",
    "crates/store/src/repair.rs",
    "crates/store/src/repair_door_cover_tests.rs",
    "crates/store/src/sealed.rs",
    "crates/store/src/sealed_fold_cover_tests.rs",
    "crates/store/src/settings.rs",
    "crates/store/src/settings_door_cover_tests.rs",
    "crates/store/src/settings_tests.rs",
    "crates/store/src/staging.rs",
    "crates/store/src/staging_cover_tests.rs",
    "reports/base-cloud-20261103/OWNER-BASE-COV-LIBS-LS1.md",
    "reports/base-cloud-20261103/fix-BASE-COV-LIBS-LS1.json.md",
    "reports/base-cloud-20261103/fu-LS1.txt.md",
    "reports/base-cloud-20261103/gaps-LS1.txt.md",
    "reports/base-cloud-20261103/report-BASE-COV-LIBS-LS1.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "pass (cargo fmt --all --check exit 0 at 4b3ee558)",
    "clippy": "pass (cargo clippy -p store --all-targets --locked -- -D warnings exit 0 at 4b3ee558)",
    "tests": "pass (cargo test -p store --locked --no-fail-fast exit 0 at 4b3ee558: 1064 passed, 0 failed, 8 ignored, 43 result lines; Linux x86_64 root host, 4 cores)",
    "web_build": "not applicable",
    "web_test": "not applicable"
  },
  "p50_p99": "none measured: no production line changed, no bench run. Route B wall at the final tip: tests 17 min (this host).",
  "coverage": "Route B (kit d3-route-b-cloud.sh unchanged, rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0), reading whole:store, tip 4b3ee558a38f: lines 52295/53771 (1476 uncovered), branch sides 3405/3964 (559 uncovered), 112 files read; NOT READ: benches/ratio.rs, benches/ratio/{months,quarantine,safe_files}.rs (test = false) and src/lib.rs (no executable line). Production rows: 81 (U), 61 host/race, 0 GAP. Lists: reports/base-cloud-20261103/gaps-LS1.txt.md, fu-LS1.txt.md. macOS-only lines owed to the Mac's Route B.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Continuation of 5 Nov 2026 (token BASE-COV-LIBS-LS1-20261105-2). Line 1 of the report: STATUS: BLOCKED QUESTION: (1) the 61 production rows only a host refusal or a race reaches: cfg(test) seams in the file.rs pattern (A), limits in docs/06 (B), or seams for the 28 host refusals and limits for the 33 races (C, recommended); (2) the 144 test-code rows (timeouts, missing-git skips, macOS-only arms, check-module failure collection): (F) under step 4's second clause, or restructure. Decision drafts: D-8560 (figures, lists, root re-runs, fresh-process children, the cfg(test) seam visibility); D-8561 kept for the answer. NEEDS THE MAC: Route B on aarch64-apple-darwin (macOS-only lines), a non-root 14-core run, the ignored real-data, timing and full-volume tests (L-BCL-3, L-PREC1-01), the (F) classification, the D1 landing.",
  "held": false
}
```
