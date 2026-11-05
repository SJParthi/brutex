# fix-BASE-COV-PULL-PS2.json.md

The FIX report of BASE-COV-PULL-PS2 (CLOUD.md item 10), one json block below.

```json
{
  "name": "BASE-COV-PULL-PS2",
  "branch": "cloud/cov-pull-ps2-20261103",
  "head": "02f268e24f8bbf230099ed53ccdb8656aadf68d2",
  "committed": true,
  "findings": [
    {
      "id": "G2-bar (D2 part A, recipe v4 step 4, group G2)",
      "status": "partial",
      "how": "Route B (kit d3-route-b-cloud.sh unchanged, pinned nightly) on the code tip: G2's 93 files lines 58766/60545 (1779 uncovered), branches 4140/4724 (584 sides). Every one of G2's 2363 uncovered rows is classified: (F) 614 by pattern and 9 by hand, (U) 125 each with its proof, (U, type invariant) 21, IGNORED 1594 (inside the 54 #[ignore] tests, real-data tests and timings run by hand, or the helpers only they reach: L-BCP-3), GAP 0. The bar is held for every row a cloud session can drive; the IGNORED rows are the operator's, so the bar as written is not held.",
      "evidence": "reports/base-cloud-20261103/gaps-PS2.txt.md (per-file table, every G2 row with its class); reports/base-cloud-20261103/fu-PS2.txt.md ((U) proofs, (F) reasons)"
    },
    {
      "id": "G2 FAULT and RACE rows (the earlier session's last NEXT)",
      "status": "fixed",
      "how": "The 60 reachable rows run 1 found (33 by a host fault, 12 by an input or a direct call, 1 by a race, 14 test-code arms) are driven by 18 new tests and 4 test edits, and the 2 one-sided rows of this session's own path-keyed steps by 2 more tests and 1 edit: planted host faults (seam::fail/answer, answer_lock, taken) in census_store, gdfl_run_ledger, landing_run, zerodha_spool, gdfl_rekey, response_keep and gdfl_run, steps keyed by path (response_keep plant_for/at_path, the filing door's plant_before_confirm), D-7336; every seam #[cfg(test)].",
      "evidence": "commits a4494cf7 f275750c 0c51a2c7 027a8cd4 ec06c852 734a49b7 05c56c9c 078db458 596cf383 b6363918 e8ceb492 02f268e2; report sections \"What was done\" and \"Coverage\""
    },
    {
      "id": "red checks of every new test",
      "status": "fixed",
      "how": "234 tests (212 of the earlier session, red-checked here by two helpers in worktrees of their own; 22 of this continuation): each shown red at its own assertion under a hand change, then undone; 234 red, 0 not red.",
      "evidence": "reports/base-cloud-20261103/red-PS2.txt.md"
    },
    {
      "id": "host reds (CLOUD.md item 8)",
      "status": "needs_operator",
      "how": "Six tests of G3's files fail as root on this host and pass as uid 65534 on the same binary (runuser -u nobody): gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte, gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost, gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name, gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice, zerodha_verify::tests::a_refused_open_keeps_the_hosts_words, zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named. None is G2's; the Mac's non-root run is the check.",
      "evidence": "report section \"Gates\""
    },
    {
      "id": "L-BCP-3 ignored tests",
      "status": "needs_operator",
      "how": "1594 of G2's uncovered rows are inside #[ignore] tests or helpers only they reach: G2's files hold 23 of the crate's 54, 11 needing the operator's real data and 12 timings on invented data, ignored for their run time; untouched (none weakened, excluded or removed). The operator chooses synthetic twins, a pass that runs them with --ignored and counts, or another rule.",
      "evidence": "gaps-PS2.txt.md class IGNORED; fu-PS2.txt.md counts by file"
    },
    {
      "id": "D-7336 (decision draft)",
      "status": "fixed",
      "how": "Drafted in the report: an arm only another writer or a failing host reaches is driven through a #[cfg(test)] test seam (a step between two host calls, or a planted host fault), never excused. PSA writes the docs entry.",
      "evidence": "report section \"Decision draft\""
    }
  ],
  "files_changed": [
    "crates/pull/src/alerts.rs",
    "crates/pull/src/alerts_ps2_tests.rs",
    "crates/pull/src/alerts_tests.rs",
    "crates/pull/src/calendar_cov_tests.rs",
    "crates/pull/src/census_store.rs",
    "crates/pull/src/census_store_kept_tests.rs",
    "crates/pull/src/census_store_link_tests.rs",
    "crates/pull/src/census_store_ps2_tests.rs",
    "crates/pull/src/census_store_tests.rs",
    "crates/pull/src/chain.rs",
    "crates/pull/src/chain_ps2_tests.rs",
    "crates/pull/src/csv.rs",
    "crates/pull/src/daycheck.rs",
    "crates/pull/src/emit_sites.rs",
    "crates/pull/src/fno.rs",
    "crates/pull/src/fno_ps2_tests.rs",
    "crates/pull/src/fnowork_futures_tests.rs",
    "crates/pull/src/fold.rs",
    "crates/pull/src/fold_ps2_tests.rs",
    "crates/pull/src/gdfl_backfill_tests.rs",
    "crates/pull/src/gdfl_cm.rs",
    "crates/pull/src/gdfl_cm_ps2_tests.rs",
    "crates/pull/src/gdfl_dayzips.rs",
    "crates/pull/src/gdfl_dayzips_ps2_tests.rs",
    "crates/pull/src/gdfl_format.rs",
    "crates/pull/src/gdfl_format_ps2_tests.rs",
    "crates/pull/src/gdfl_import_census.rs",
    "crates/pull/src/gdfl_import_census_ps2_tests.rs",
    "crates/pull/src/gdfl_import_filing.rs",
    "crates/pull/src/gdfl_import_filing_ps2_tests.rs",
    "crates/pull/src/gdfl_import_quality.rs",
    "crates/pull/src/gdfl_import_quality_ps2_tests.rs",
    "crates/pull/src/gdfl_r2_attack_tests.rs",
    "crates/pull/src/gdfl_rekey.rs",
    "crates/pull/src/gdfl_rekey_ps2_tests.rs",
    "crates/pull/src/gdfl_rekey_tests.rs",
    "crates/pull/src/gdfl_run.rs",
    "crates/pull/src/gdfl_run_cov_tests.rs",
    "crates/pull/src/gdfl_run_ledger.rs",
    "crates/pull/src/gdfl_run_ledger_ps2_tests.rs",
    "crates/pull/src/gdfl_run_tests.rs",
    "crates/pull/src/gdfl_sealed_tests.rs",
    "crates/pull/src/gdfl_seconds_attack_tests.rs",
    "crates/pull/src/gdfl_v3.rs",
    "crates/pull/src/gdfl_v3_ps2_tests.rs",
    "crates/pull/src/landing_run.rs",
    "crates/pull/src/landing_run_tests.rs",
    "crates/pull/src/padded_tests.rs",
    "crates/pull/src/repair_plan.rs",
    "crates/pull/src/repair_plan_ps2_tests.rs",
    "crates/pull/src/response_keep.rs",
    "crates/pull/src/response_keep_tests.rs",
    "crates/pull/src/riskfree.rs",
    "crates/pull/src/zerodha_spool.rs",
    "crates/pull/src/zerodha_spool_cov_tests.rs",
    "crates/pull/tests/alerts_ps2.rs",
    "crates/pull/tests/broker.rs",
    "crates/pull/tests/cloud_mutants_import.rs",
    "crates/pull/tests/store_evidence.rs",
    "crates/pull/tests/zerodha_identity.rs",
    "reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PS2.md",
    "reports/base-cloud-20261103/fix-BASE-COV-PULL-PS2.json.md",
    "reports/base-cloud-20261103/fu-PS2.txt.md",
    "reports/base-cloud-20261103/gaps-PS2.txt.md",
    "reports/base-cloud-20261103/red-PS2.txt.md",
    "reports/base-cloud-20261103/report-BASE-COV-PULL-PS2.md"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "cargo fmt --all --check rc=0 on 02f268e24f8bbf230099ed53ccdb8656aadf68d2",
    "clippy": "cargo clippy -p pull --all-targets --locked -- -D warnings rc=0 on 02f268e24f8bbf230099ed53ccdb8656aadf68d2",
    "tests": "cargo test -p pull --locked --no-fail-fast on 02f268e24f8bbf230099ed53ccdb8656aadf68d2: 33 binaries, 3177 passed, 6 failed, 54 ignored; the 6 failures are the root-only host reds of G3 files, which pass as uid 65534 on the same binary (ok. 6 passed; 0 failed; 0 ignored; 0 measured; 2687 filtered out; finished in 0.01s)",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [
    "D3 coverage pull route B: ROUTE-B START 2026-11-05 10:24:45 IST, build to 10:25:36, tests to 10:29:09, every file read by 2026-11-05 10:53:53 IST; ROUTE-B DONE, exit 0",
    "benches: none run (no G2 step is timed; gate 8 is the Mac's)"
  ],
  "coverage": {
    "route": "B (kit d3-route-b-cloud.sh unchanged; rustc 1.99.0-nightly (1a98b1e13 2026-08-07), LLVM 23.1.0-rust-1.99.0-nightly; RUSTFLAGS -C instrument-coverage -Z coverage-options=branch, opt-level 0)",
    "reading": "whole:pull (222 src/ and tests/ files at the tip)",
    "whole_pull": "lines 159895/166046 (6151 uncovered), branches 11903/14346 (2443 sides)",
    "g2": "lines 58766/60545 (1779 uncovered), branches 4140/4724 (584 sides)",
    "classes": {
      "F": 614,
      "F_by_hand": 9,
      "U": 125,
      "U_type_invariant": 21,
      "IGNORED": 1594,
      "GAP": 0
    },
    "lists": [
      "reports/base-cloud-20261103/gaps-PS2.txt.md",
      "reports/base-cloud-20261103/fu-PS2.txt.md"
    ],
    "measured_on_head": true,
    "proved": false,
    "state": "Measured on the head by the kit's own reading (run 5, ROUTE-B DONE, exit 0). NOT PROVED: the bar as written (100% lines and branches) is not held, because 1594 of G2's rows are inside the #[ignore] tests or the helpers only they reach (L-BCP-3, the operator's). Every other row is driven by a test or classified (F)/(U) with its reason or proof: GAP 0."
  },
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "BASE-COV-PULL-PS2, continued on a new account (TOKEN BASE-COV-PULL-PS2-20261105-2) from f4fe72bf; the earlier token is retired. Reading whole:pull; the bar is answered for group G2 (93 files). Report: reports/base-cloud-20261103/report-BASE-COV-PULL-PS2.md (line 1 STATUS: BLOCKED OUTSIDE: only the operator's L-BCP-3 and the Mac's checks are left). Decision draft D-7336 in the report (PSA writes the docs entry). Host: kernel.pid_max was raised from 32768 to 4194304 before the final Route B run, because the suite wraps the pids within a run and a uid-65534 child given the pid of an earlier root process of the same binary cannot merge into that root-owned profile (runs 3 and 4 lost one child's profile so). Follow-up for the deferred mutants pass: gdfl_run::ledger::ps2_tests::a_claims_line_too_long_for_a_record_is_never_read does not pin claimed()'s body-length guard (gdfl_run_ledger.rs:527-529), redundant for every line its reader is handed. Nothing outside G2's files and reports/ was edited; no test was weakened, deleted or ignored.",
  "held": false
}
```
