# fix-BASE-COV-LIBS-LS1.json.md

The session's structured report in the launcher's FIX shape, one json block (crate `store`).

```json
{
  "name": "BASE-COV-LIBS-LS1",
  "branch": "cloud/cov-libs-ls1-20261103",
  "head": "fc9b489f0e3ca9207a3f1fe045d913adfa1429fa",
  "committed": true,
  "findings": [
    {
      "id": "LS1-ROOT-REDS",
      "status": "fixed",
      "how": "Seven store lib tests failed only as root (two from attempts 2-4, five base tests); each now runs its unchanged body through crate::support::where_permission_binds, which re-runs it as nobody under root and in the same uid otherwise (the D-0995 pattern).",
      "evidence": "Each passed alone as uid 65534 before the change (setpriv --reuid=65534 ... --exact <name>: 1 passed); after, cargo test -p store --lib --locked --no-fail-fast as root: 475 passed, 0 failed, 2 ignored (commit c7ccb0f7)."
    },
    {
      "id": "LS1-COV-STORE",
      "status": "not_fixed",
      "how": "36 new asserting tests in 9 new modules (each declared in the file it tests) plus one in quarantine_tests.rs, each red under a hand change; 44 (U) items proved from the code; no production line changed.",
      "evidence": "Route B on the pinned nightly, whole crate: baseline tip c7ccb0f7f7e2 lines 50033/51607, sides 3291/3868; final tip fc9b489f0e3c lines 51094/52629, sides 3351/3916; production rows 225 lines + 104 sides down to 169 + 70 (44 of them (U)). Remaining: 195 production GAP rows, 995 ignored-test rows, 213 test-code GAP rows (fu-LS1.txt.md). Bar NOT met."
    },
    {
      "id": "LS1-TEST-FLAW",
      "status": "fixed",
      "how": "My own stale-summary test left n_valid 0 in its crafted summaries, so it passed without reaching the clauses it names; each case now carries its row count.",
      "evidence": "Commit fc9b489f; red when the fold-count clause alone is removed; final Route B shows month_summary_writer.rs 165/165 lines, 34/34 sides."
    }
  ],
  "files_changed": [
    "crates/store/src/catalog_cover_tests.rs",
    "crates/store/src/file.rs",
    "crates/store/src/file_cover_tests.rs",
    "crates/store/src/file_sealed.rs",
    "crates/store/src/file_sealed_cover_tests.rs",
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
    "crates/store/src/settings.rs",
    "crates/store/src/settings_door_cover_tests.rs",
    "crates/store/src/settings_tests.rs",
    "crates/store/src/staging.rs",
    "crates/store/src/staging_cover_tests.rs",
    "reports/base-cloud-20261103/OWNER-BASE-COV-LIBS-LS1.md",
    "reports/base-cloud-20261103/report-BASE-COV-LIBS-LS1.md",
    "reports/base-cloud-20261103/gaps-LS1.txt.md",
    "reports/base-cloud-20261103/fu-LS1.txt.md",
    "reports/base-cloud-20261103/fix-BASE-COV-LIBS-LS1.json.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "pass (cargo fmt --all --check exit 0 at fc9b489f)",
    "clippy": "pass (cargo clippy -p store --all-targets --locked -- -D warnings exit 0 at fc9b489f)",
    "tests": "pass (cargo test -p store --locked --no-fail-fast exit 0 at fc9b489f: 1028 passed, 0 failed, 8 ignored, 43 result lines; Linux x86_64 root host, 4 cores)",
    "web_build": "not applicable",
    "web_test": "not applicable"
  },
  "p50_p99": "none measured: no production line changed, no bench run. Route B wall: baseline tests 26 min, final tests 25 min (this host).",
  "coverage": "Route B (kit d3-route-b-cloud.sh, rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0), reading whole:store, tip fc9b489f0e3c: lines 51094/52629 (1535 uncovered), branch sides 3351/3916 (565 uncovered), 108 files read; NOT READ: benches/ratio.rs, benches/ratio/{months,quarantine,safe_files}.rs (test = false) and src/lib.rs (no executable line). Baseline tip c7ccb0f7f7e2: 50033/51607, 3291/3868. Lists: reports/base-cloud-20261103/gaps-LS1.txt.md, fu-LS1.txt.md. macOS-only lines owed to the Mac's Route B.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Decision drafts in the report: D-8560 (store figures, lists, root re-runs); D-8561 unused. NEEDS THE MAC: Route B on aarch64-apple-darwin (macOS-only lines), a non-root 14-core run, the ignored real-data, timing and full-volume tests (L-BCL-3, L-PREC1-01), the D1 landing. settings.rs 1367-1374 are covered only when the base suite's racing-writers test interleaves that way (covered at baseline, not at final). Line 1 of the report: STATUS: IN PROGRESS (reachable production rows remain; NEXT listed there).",
  "held": false
}
```
