# fix-BASE-MSUM-BACKFILL-MS1.json.md

The launcher's FIX object for session BASE-MSUM-BACKFILL-MS1 (CLOUD.md item 10). head is the code tip, pushed; the Mac's open work is listed as findings with how = the Mac's half.

```json
{
  "name": "BASE-MSUM-BACKFILL-MS1",
  "branch": "cloud/msum-ms1-20261103",
  "head": "501651b3b6b56ff30b7635ee9bce7e4d6d8b75f1",
  "committed": true,
  "held": false,
  "findings": [
    {"id": "MSUM-F1", "status": "fixed", "how": "The summary door holds the month SHARED (open_held(.., Hold::Shared), Flock::try_lock_shared_patiently): a writer is refused by name and nothing is written, a reader is neither refused nor made to wait; two doors over one month are kept apart by an exclusive non-blocking flock on the bar file handle the door already opened (D-7431).", "evidence": "store month_summary::tests::the_door_refuses_a_writer_and_never_a_reader and cli upkeep_command::tests::a_run_builds_beside_a_reader_and_refuses_a_writers_month_by_name, both red with Hold::Exclusive; month_summary::tests::a_second_door_is_refused_by_the_bar_files_own_lock red without the bar-file lock (red-BASE-MSUM-BACKFILL-MS1.txt.md, batch A)."},
    {"id": "MSUM-F2", "status": "fixed", "how": "The rebuild door syncs the staged .sum before the rename (Durable::Synced); the writer's per-commit step stays unsynced, its next open repairs a cut (D-7432).", "evidence": "store file::file_fault_tests::a_refused_sync_leaves_the_name_free red with the sync removed; a_full_disk_leaves_no_partial_summary_under_its_name red with the write's refusal ignored (batch A)."},
    {"id": "MSUM-F3", "status": "fixed", "how": "The walk drops the .sum lack of a month whose bar file length proves fewer than INDEX_MIN_ROWS rows (summary_owed_by_len) and counts it in Walked::sum_small; a zero-length or torn-header file stays owed; .tix/.srt unchanged; plan SMALL line; --all takes small months (D-7433).", "evidence": "pull upkeep::tests::the_walk_counts_a_month_too_short_to_owe_a_summary_apart and cli upkeep_command::tests::a_plan_names_the_lacking_months_and_none_after_the_step, both red with the walk's small-month block disabled (batch B)."},
    {"id": "MSUM-F4", "status": "open", "how": "the Mac's half", "evidence": "Gate 30 is red at feat/db-fix f01d7a0e4 on nine merged = false packages (R3-17 among them; BASE-REDS-1023's), so the store-steps rows must not name R3-17 as a prerequisite beside figures. Not proved here; the progress file's plan is in D-7437 of the report."},
    {"id": "MS1-S1", "status": "fixed", "how": "The one-door flock was taken on a second open of the bar file, whose refusal only a race could reach; it is now taken on the handle the door already opened (844d4cba).", "evidence": "month_summary_writer.rs lines 63/63, branches 8/8 by Route B; a_second_door_is_refused_by_the_bar_files_own_lock."},
    {"id": "MS1-S2", "status": "fixed", "how": "store.sum's run-start line logged lacking = plan.len(), which under --all counts months that have a .sum; it now logs items and lacking apart (D-7435, 844d4cba).", "evidence": "cli upkeep_command::tests::a_run_says_on_store_sum_what_it_was_given_and_left checks every field's value; red with lacking = plan.len() (batch C)."},
    {"id": "MS1-S3", "status": "fixed", "how": "The door the summary and the sort index share (open_held) did not refuse a month filed under another name (DB-B65), so a backfill wrote a .sum beside a month every reader refuses; it now calls refuse_misfiled before validated (D-7434, b9d24074). This changes the sort door too.", "evidence": "store month_summary::tests::a_month_filed_under_another_name_is_refused_by_the_door (both holds), red without the check (batch A)."},
    {"id": "MS1-S4", "status": "fixed", "how": "summary_owed_by_len assumed the bar geometry; a test pins it to every bar layout in Layout::KNOWN (D-7433).", "evidence": "pull upkeep::tests::the_length_rule_holds_for_every_bar_layout, red under a hypothetical version-4 bar layout with a 48-byte stride (batch B)."},
    {"id": "MS1-S5", "status": "fixed", "how": "msum_builder's read-back refusals (a read-back refused after the door, or one refusing the summary the door kept or wrote) were reachable only by a race; build now hands the door's answer and a read-back closure to checked(), driven by a new test module (D-7436, b9d24074).", "evidence": "cli msum_builder::read_back_tests::a_read_back_refused_after_the_door_is_said_and_never_clean and ::a_summary_the_reader_refuses_after_the_door_is_not_summarised, red under their hand changes (batch C)."},
    {"id": "MS1-S6", "status": "fixed", "how": "START did not compile (StoreError and summary_owed_by_len not imported in two test modules); three existing walk/plan tests expected small months listed as lacking a .sum; two WIP resume tests expected skip counts only --all can give (9189ac1d).", "evidence": "cargo test -p store --lib, -p pull --lib upkeep, -p cli --lib -- upkeep msum green apart from proved host reds and the red-by-design store-steps test (report, Gates)."},
    {"id": "MAC-REDS-STEPS", "status": "open", "how": "the Mac's half", "evidence": "upkeep_command::tests::the_msum_store_half_is_a_pending_step_for_each_feed is red by design (the spec's test-first test): no config/store-steps.toml row runs cli upkeep run msum gdfl NSE/INDEX --workers 8 or cli upkeep run msum zerodha --workers 8 yet; it turns green with the Mac's rows (adjust its commands to the rows)."},
    {"id": "MAC-RELEASE", "status": "open", "how": "the Mac's half", "evidence": "Release build of api and cli: cargo build --release -p api -p cli."},
    {"id": "MAC-REHEARSAL", "status": "open", "how": "the Mac's half", "evidence": "Rehearse cli upkeep run msum gdfl (NSE/INDEX, NSE/CASH, NSE/FNO) and the zerodha twin on shadow/MSUM-g1 and shadow/MSUM-z1 at 1x/10x/100x months and 1, 4, 8 workers: months a second, bytes, the limiting stage, byte-identical .sum across worker counts, one stop and resume, /gaps.json during the run never a 503, a torn read or a changed answer."},
    {"id": "MAC-STORE-STEPS", "status": "open", "how": "the Mac's half", "evidence": "The store-steps rows and their figures, one per feed, with MSUM-F4 (no R3-17 prerequisite beside figures while gate 30 is red); draft D-7437 in the report."},
    {"id": "MAC-DOCS", "status": "open", "how": "the Mac's half", "evidence": "docs/04 (invariant drafts in the report), docs/05 (D-7430..D-7437 drafts in the report), docs/06 (limits drafts; the rehearsal's rates and bytes; the live time as a labelled extrapolation)."},
    {"id": "MAC-GATES", "status": "open", "how": "the Mac's half", "evidence": "Full gates on the merged tree: workspace fmt, clippy, tests and the CI shell gates (gate 30 is BASE-REDS-1023's)."},
    {"id": "MAC-SHADOW-5", "status": "open", "how": "the Mac's half", "evidence": "Shadow step 5: /gaps.json p50/p99/max at n = 200 over 1, 12 and 96 months before and after, 1 and 16 clients (0 refused), one month at 705, 116k and 517k bars, flat within the ratio gate after the step."},
    {"id": "O1L-04", "status": "open", "how": "the Mac's half", "evidence": "The package's original /gaps.json finding: closes when the store step has run live and shadow step 5 measures it."},
    {"id": "O1L-08", "status": "open", "how": "the Mac's half", "evidence": "As O1L-04."},
    {"id": "o1s-16", "status": "open", "how": "the Mac's half", "evidence": "As O1L-04."},
    {"id": "o1r-N1", "status": "open", "how": "the Mac's half", "evidence": "As O1L-04."},
    {"id": "AL-10", "status": "open", "how": "the Mac's half", "evidence": "As O1L-04."},
    {"id": "AA-07", "status": "open", "how": "the Mac's half", "evidence": "As O1L-04."},
    {"id": "MAC-MACOS", "status": "open", "how": "the Mac's half", "evidence": "macOS-only store lines (crates/store/src/file.rs:7271, :10466; crates/store/src/open_flags.rs:33-122) and the whole D2 set re-measured by the Mac's Route B (these figures are Linux x86_64)."},
    {"id": "MAC-NONROOT", "status": "open", "how": "the Mac's half", "evidence": "Non-root 14-core runs of the store, pull and cli suites: the 17 root reds of this host (store 5, pull 9, cli 3, each passing as uid 65534 here) are expected green."},
    {"id": "MAC-D7434", "status": "open", "how": "the Mac's half", "evidence": "Confirm D-7434's change to the sort door at landing: cli upkeep run srt now refuses a misfiled month by name instead of building a .srt no reader reads."},
    {"id": "MAC-LANDING", "status": "open", "how": "the Mac's half", "evidence": "The D1 landing."}
  ],
  "files_changed": [
    "crates/store/src/month_summary_writer.rs",
    "crates/store/src/sort_index_writer.rs",
    "crates/store/src/month_summary_tests.rs",
    "crates/store/src/file_fault_tests.rs",
    "crates/pull/src/upkeep.rs",
    "crates/pull/src/upkeep_tests.rs",
    "crates/cli/src/upkeep_command.rs",
    "crates/cli/src/upkeep_command_tests.rs",
    "crates/cli/src/msum_builder.rs",
    "crates/cli/src/msum_builder_read_back_tests.rs"
  ],
  "files_outside_ownership": [],
  "gates": {
    "fmt": "pass: cargo fmt --all -- --check rc 0 at 501651b3",
    "clippy": "pass: cargo clippy -p store -p pull -p cli --all-targets --locked -- -D warnings rc 0 at 501651b3 (rustc 1.97.1)",
    "tests": "cargo test -p store --lib --locked: 433 passed, 5 failed (root-only, each green as uid 65534; the binary as uid 65534: 438 passed, 0 failed), 2 ignored; cargo test -p pull --lib --locked upkeep: 39 passed, 0 failed; cargo test -p cli --lib --locked -- upkeep msum: 46 passed, 1 failed (the_msum_store_half_is_a_pending_step_for_each_feed, red by design until the Mac's store-steps rows), 1 ignored; whole pull lib: 2173 passed, 9 failed (root-only, green as uid 65534), 48 ignored; whole cli lib with --skip the_pool_verb_prints_its_whole_page_on_a_generated_store: 2431 passed, 4 failed (3 root-only, green as uid 65534, and the red-by-design test), 12 ignored",
    "web_build": "not applicable (web/ unchanged)",
    "web_test": "not applicable (web/ unchanged)"
  },
  "p50_p99": [],
  "coverage": "Route B on the pinned nightly (rustc 1.99.0-nightly (1a98b1e13 2026-08-07); llvm-cov and llvm-profdata LLVM 23.1.0-rust-1.99.0-nightly), reading narrow, set deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-set.txt.md (152 coverage items, 10 files): lines 1,592/1,596, branches 68/68. Per file: store month_summary_writer.rs 63/63 8/8, sort_index_writer.rs 57/57 6/6, month_summary_tests.rs 200/200, file_fault_tests.rs 47/47; pull upkeep.rs 80/80 16/16, upkeep_tests.rs 310/310; cli upkeep_command.rs 198/198 36/36, upkeep_command_tests.rs 552/552 2/2, msum_builder.rs 50/54 0/0, msum_builder_read_back_tests.rs 35/35. The 4 lines are one (U) item, crates/cli/src/msum_builder.rs:184-187 (the wildcard arm of the #[non_exhaustive] store::file::Summarized, all four variants matched above it; proof in deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-fu.txt.md). No GAP, no (F). store and pull measured at ceb28bb9 (byte-identical at the head), cli at the head; Linux x86_64 figures, the Mac re-measures.",
  "mutants": "deferred (operator 30 Oct)",
  "store_writes_needed": [
    "NOT REHEARSED, not ready to run: cli upkeep run msum gdfl NSE/INDEX --workers 8; then NSE/CASH; then NSE/FNO (the rehearsal and the store-steps rows first; the operator's yes)",
    "NOT REHEARSED, not ready to run: cli upkeep run msum zerodha --workers 8 (the rehearsal and the store-steps row first; the operator's yes)"
  ],
  "notes": "Code-only half of BASE-MSUM-BACKFILL, done in the cloud (session BASE-MSUM-BACKFILL-MS1, token BASE-MSUM-BACKFILL-MS1-20261103-1). START 8bc44c1e (4d7f446f7a81 plus the kit) did not compile; it compiles and its tests pass. Findings (1), (2), (3) fixed and each proved by a test red under a hand change; finding (4) is the Mac's (store-steps rows). The self-check found and fixed: a race-only second open in the one-door flock (S1), a misnamed lacking field under --all (S2), the shared door not refusing a misfiled month (S3, changes the sort door too: D-7434, the Mac to confirm), an unpinned geometry assumption (S4), race-only read-back arms made testable (S5). Every new test (8 store, 4 pull, 15 cli) shown red under a hand change: red-BASE-MSUM-BACKFILL-MS1.txt.md (29 checks, 29 red). One test red by design until the Mac's rows land: upkeep_command::tests::the_msum_store_half_is_a_pending_step_for_each_feed. Host reds (root only, each green as uid 65534): store 5, pull 9, cli 3, named in the report. Decision drafts D-7430..D-7437 in the report (D-7438, D-7439 unused); nothing written to docs/, config/, Cargo.* or any lib.rs. No store, shadow, server or vendor data touched. Reading narrow; mutants deferred. The Mac's open items are listed as findings with how = the Mac's half."
}
```
