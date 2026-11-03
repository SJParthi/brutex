```json
{
  "name": "BASE-COV-PULL-PS1",
  "branch": "cloud/cov-pull-ps1-20261103",
  "head": "2e7a0b6d3b568190024e2e9b405e803805f3fdca",
  "committed": true,
  "findings": [
    {
      "id": "PS1-step1-compile",
      "status": "fixed",
      "how": "Repair 2's tests type-check as written (cargo check -p pull --lib --tests --locked: no error, no warning). The faults were three clippy -D warnings errors in its new tests: manual_assert_eq at gdfl_dropbox_cov_tests.rs:56, single_match_else at gdfl_run_start_cov_tests.rs:32 and ssm_cov_tests.rs:195. Fixed without a behaviour change.",
      "evidence": "commit 91de72cb; cargo clippy -p pull --all-targets --locked -- -D warnings rc 0 on 2e7a0b6d"
    },
    {
      "id": "PS1-step1-failures",
      "status": "fixed",
      "how": "cargo test -p pull --lib --locked: 10 red as root, all green as uid 65534. Nine are old tests in files outside the 34 (host reds, listed in notes). The tenth, repair 2's master_snapshots::cov_tests::a_root_that_will_not_list_under_the_lock_is_refused_on_both_counts (a G1 file), chmods its root to 0o300, which root lists anyway; its body now runs through support::where_permission_binds (D-0995) and passes as root.",
      "evidence": "commit 2e7a0b6d; cargo test -p pull --locked --no-fail-fast on 2e7a0b6d: 32 binaries, 2940 passed, 9 failed (root only), 54 ignored; the lib binary as uid 65534: 2416 passed, 0 failed, 48 ignored"
    },
    {
      "id": "PS1-green-tip",
      "status": "fixed",
      "how": "GREEN-TIP 2e7a0b6d3b568190024e2e9b405e803805f3fdca: fmt, clippy -D warnings and the whole pull suite green apart from 9 proved root-only host reds.",
      "evidence": "reports/base-cloud-20261103/report-BASE-COV-PULL-PS1.md line 2 and section Gates"
    },
    {
      "id": "PS1-step2-G1-coverage",
      "status": "not_fixed",
      "how": "In progress: Route B on the pinned nightly over pull, then G1's uncovered lines and sides.",
      "evidence": "report section Step (2)"
    }
  ],
  "files_changed": [
    "crates/pull/src/gdfl_dropbox_cov_tests.rs",
    "crates/pull/src/gdfl_run_start_cov_tests.rs",
    "crates/pull/src/master_snapshots_cov_tests.rs",
    "crates/pull/src/ssm_cov_tests.rs",
    "reports/base-cloud-20261103/OWNER-BASE-COV-PULL-PS1.md",
    "reports/base-cloud-20261103/report-BASE-COV-PULL-PS1.md",
    "reports/base-cloud-20261103/fix-BASE-COV-PULL-PS1.json.md"
  ],
  "files_outside_ownership": [
    "crates/pull/src/gdfl_dropbox_cov_tests.rs (G3's; one of the 34 files step (1) may edit)",
    "crates/pull/src/gdfl_run_start_cov_tests.rs (G3's; one of the 34 files step (1) may edit)"
  ],
  "gates": {
    "fmt": "cargo fmt --all --check rc 0 on 2e7a0b6d",
    "clippy": "cargo clippy -p pull --all-targets --locked -- -D warnings rc 0 on 2e7a0b6d",
    "tests": "cargo test -p pull --locked --no-fail-fast on 2e7a0b6d (root, 4 cores, TMPDIR a 1777 tmpfs): 32 binaries, 2940 passed, 9 failed, 54 ignored; the 9 are root-only and pass as uid 65534",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": "none: no bench or timed step in this change",
  "coverage": "step (2) in progress; no figure yet",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [],
  "notes": "Host reds (root only; proof: the same test passes as uid 65534 on the same binary): alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it; gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte; gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost; gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name; gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice; ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words; ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing; zerodha_verify::tests::a_refused_open_keeps_the_hosts_words; zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named. On a root host TMPDIR must be mode 1777, or where_permission_binds children (uid 65534) cannot make their scratch folders and 13 more lib tests go red. NEEDS THE MAC: Route B whole:pull on its pin, the shadow GET re-check, macOS rows, non-root 14-core tests, the 54 ignored real-data tests (L-BCP-3), the D1 landing.",
  "held": false
}
```
