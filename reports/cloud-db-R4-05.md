STATUS: HELD

# R4-05 (cloud): one reasons dictionary with its gate and route, and the kept alert kinds

Branch `cloud/db-R4-05-20261024`, cut from `origin/feat/db-fix` at `e851da1c` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/R4-05.md`.

## Prerequisites

- IA-16: merged (`Merge IA-16: failure kinds and causes ...` on `origin/feat/db-fix`).
- R3-21: FILES_ONLY link (merge order on `crates/pull/src/alerts.rs` KINDS), not merged; does not block the start.
- IA-12a: dropped (D-6900), not a prerequisite.

## Done so far

- `crates/pull/src/reasons.rs` (new): `TABLE` (a `static`), 184 rows `(code, sentence, action, control)`; `Control`
  = `Repair` ("press Repair"), `RetryRefused` ("Retry refused"), `StoreStep(id)` ("store step <id>"),
  `Operator(what)` ("operator: <what>"), `Nothing` ("no action"); `of(code)` one hash probe; `title(row)`
  = `code: sentence; control`; `body(build)` the `/reasons.json` document and its ETag
  `"<build>.<16 hex of blake3(bytes)>"`, one `pull.reasons` / `reasons table built` event per call.
- `crates/pull/src/reasons_tests.rs` (new): the gate. Sources: `ImportRefusal::code` (parsed),
  `cm_cause`/`nfo_cause`/`import_cause` (parsed), `FailKind::code` (parsed; the store's repair refusals
  reach pages through `FailKind::of_repair`), `RunRefusal::CODES` (called, cross-checked against its
  parsed `code()`), `store::verdict::Code::ALL` and `flags::ALL` (called), `RowCause::code` (parsed,
  `crate::listing::` constants resolved from `listing.rs`) and `HistoryEnds::CODE` (called),
  `verdict_words::REASONS` (called). The parser refuses an unreadable arm, a wildcard arm and any string
  literal it did not take, by name.
- `crates/pull/src/alerts.rs`: KINDS 22 `vendor_format_changed`, 23 `vendor_restated`, 24 `scrub_stale`,
  25 `credential_dead`; one key builder per kind (`vendor_format_changed`, `vendor_restated`,
  `scrub_stale`, `credential_dead` -> `Keyed`), each part checked by `key_part` (alphabet
  `[A-Za-z0-9._&-]`, no token shape), refusal `alert_key_part_refused` never repeating the value.
- `crates/pull/src/alerts_tests.rs`: KINDS pin extended to 25; the old unknown-kind probes moved from 22
  to 26 (`kind_word(26)`, `garbage_alert_parameters_are_refused_by_name`); 7 new tests.
- `crates/pull/src/lib.rs`: one `pub mod reasons;` line (shared line).

## Measured so far (in process, this container: 4 cores, `test` profile; n and method per row)

| What | n | p50 | p99 | max | Method |
|---|---|---|---|---|---|
| `GET /reasons.json`, 1 client, sequential | 200 | 5.2 µs | 24.8 µs | 54.6 µs | handler in process, `Instant` per request (first sample 18.9 µs) |
| `GET /reasons.json`, 60 clients, half revalidating (304) | 12,000 | 2.8 µs | 7.2 µs | 925 µs | 60 tokio tasks x 200 requests, 4 worker threads |
| `pull::reasons::body` (the cold build, once per process) | 200 | 1.03 ms | 1.45 ms | 1.70 ms | 184 rows, 68,971 bytes |
| `pull::reasons::of` | 200 | 64 ns | 183 ns | 445 ns | half known, half unknown codes |
| `titleFor` (browser, Node 22) | 200 | 0.0019 ms | 0.0109 ms | 4.1 ms | 100,000-row table, `performance.now` |

Not measured here (NEEDS THE MAC item 7): the socket's share, the shadow server, an import writing.

## Gates so far (the final run comes after the last edit and the final merge)

- `cargo fmt --all --check`: clean.
- `cargo clippy -p pull -p api --all-targets --locked -- -D warnings`: clean (after fixing 4 findings
  of mine: `format_collect` twice, `bool_assert_comparison`, `borrow_as_ptr`). The workspace run comes
  in the final pass.
- `cargo test -p pull --lib --locked -- reasons:: alerts::`: all pass but one root-only test,
  `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`
  (expects `PermissionDenied` from a 0555 directory; this container runs as uid 0, which the mode does
  not stop). Not caused by this change; to be shown red on the unmodified base in the final pass.
- `cargo test -p api --lib --locked -- reasons_route pool_table operation_audit`: all pass.
- Web: `node --test tests/reasons.test.js tests/import-panel.test.js tests/import-panel-archive.test.js`
  -> 19 pass.
- CI shell gates (run from `ci.yml`'s own `run:` blocks by a local extractor, on this branch and on a
  clean worktree of the base `e851da1c`): PASS on both: 0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 7, 13, 15, 16,
  17, 21, 22, 24, 28, 25, 27, 26, 14, 29, 29b; 6c on the two `.github/*.rs` tools I edited (rustfmt and
  clippy-driver, lib and test). RED on both, the base reds CONTEXT §4.10 names (BASE-REDS-1023):
  1c, 1d, 10, 11, 12, 19, 23, 27b, 30. Inside them this change adds nothing: 1c, 27b, 30 identical;
  1d identical once R4-05's words are declared (`R4_05_LITERAL` in `.github/gates_tree.rs`); 12 the
  same 48 refusals once my cost claims name their tests; 23 lists my two measurement prints as declared
  (in `ci.yml`) and measured; 10, 11, 19 differ only in counts. Not run here: 1e (a whole-workspace
  build and test with `web/` moved aside: the workspace test gate covers its test half).

## Process slips, said plainly

- Three shell commands in this session named or ran the Py-language interpreter by mistake: a version
  probe, an empty no-op script, and a throwaway shell variable whose name held the word. None ran any
  code of that language, nothing it printed was used, and nothing of it is in any file.

## Log
- 2026-10-24 (IST morning): GO seen, spec read, branch cut.
- 2026-10-24: table, gate and alert kinds built; tests green but the root-only one. HOLD seen at this
  save point: committed and pushed, waiting for GO without HOLD.
  NEXT: `crates/api/src/reasons_route.rs` (route, server.rs lines, pool_table_tests and
  operation_audit_tests rows, bench), then the web half, docs rows, final gates.
- 2026-10-24 12:37 IST: HOLD lifted on the base (`55dfb044`); README now carries a per-part GO rule
  and "no typed number outside the Winner Gate" (the title cut is therefore served as `title_cut`,
  the table's own longest title). Built `crates/api/src/reasons_route.rs` (OnceLock body, ETag,
  `no-cache`, 304), its `server.rs` mod and route lines, the `pool_table_tests` and
  `operation_audit_tests` rows, `/reasons.json` in `.github/gates_routes.rs`'s compiled-table owed list;
  `web/src/lib/reasons.js`, `import-panel.js` (`outcomeWord`, `reasonOf`, `cells`), `ImportPanel.svelte`
  (titles, control chip, unknown states); `web/tests/reasons.test.js` and two tests in
  `import-panel.test.js`; `docs/02` §38 rows and key rules; `docs/05` D-TBD-R4-05-1..5.
  `node --test tests/reasons.test.js tests/import-panel.test.js tests/import-panel-archive.test.js`
  -> 19 pass, 0 fail.
  NEXT: run the api tests (route, pool table, audit table), docs/04 and docs/06 rows with the measured
  numbers, then the final merge of feat/db-fix and every gate.
- 2026-10-24 ~13:30 IST: api tests green (route, pool table, audit table); docs/04 rows R405-01..12,
  docs/06 rows L-R405-01..06 with the measurements above; workspace clippy found 4 findings of mine
  (fixed); CI shell gates run on branch and base (results above); `.github/gates_tree.rs` gains
  `R4_05_LITERAL` (gate 1d), `ci.yml` gate 23 declares my two measurement prints, cost claims name
  their tests (gate 12). The base was re-HELD at 12:56 IST for parts without `GO-R4-05`: this save
  point is committed and pushed, and I wait. `origin/feat/db-fix` moved to `1c094c90` (Merge IA-4).
  NEXT: merge `origin/feat/db-fix` (`1c094c90` or newer), then the final gate pass: fmt, workspace
  clippy, workspace test, deny (installed: cargo-deny 0.20.2), llvm-cov and mutants on the touched
  modules, the web gates W1 to W5 (web/build rebuilt), the shell gates again; then STATUS: DONE.
- 2026-10-24 17:34 IST: HOLD lifted. Merged `origin/feat/db-fix` at `7b397bbe` (R4-27, IA-4,
  CLOUD-MUTSTORE, CLOUD-MUTCLI; merge `eb1de065`): the three ledgers conflicted textually only and
  were resolved "both, in merge order" (the base's sections, then R4-05's); KINDS at the head still
  ends at 21 and R3-21 is still not merged. After the merge: the reasons gate 20/20 and alerts 49/51
  (one ignored, the root-only failure). `cargo clippy --workspace --all-targets --locked -- -D warnings`
  -> EXIT 0. `cargo deny check` -> advisories FAILED on RUSTSEC-2026-0386 (tokio `AsyncFd`, unsound),
  red the same on the unmodified base `7b397bbe`; this branch touches no manifest and no lock: NEEDS
  THE MAC. Web: W1 passes once `web/build` is rebuilt and committed (it is, here; a second build
  reproduces it); W2 found one failure of mine, `proxy.test.js` (the dev proxy did not name
  `/reasons.json`: fixed in `web/vite.config.js`), and 5 cancelled timer tests in `ask.test.js` and
  `census-worker.test.js` that are cancelled identically on the base under this container's Node 22
  (CI runs Node 24): NEEDS THE MAC; W3 32 svelte-check errors, the base's own 32 (my one extra, in my
  test, fixed); W4 0 unused selectors; W5 clean. HOLD set again at 18:16 IST: this save point is
  pushed; the detached workspace test run (`cargo test --workspace --locked --no-fail-fast`) finishes
  on its own while I wait.
  NEXT: read the workspace test result; coverage (`cargo llvm-cov --branch`) and `cargo mutants
  --in-diff` on the touched modules; the shell gates once more; then STATUS: DONE.
- 2026-10-24 ~20:00 IST (still HELD): the detached `cargo test --workspace --locked --no-fail-fast` ended
  `EXIT 101`, 7 targets failing, 60 tests (list below). Two tests hung at 0% CPU and I stopped them so
  the run could finish, each named: `api` lib
  `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
  (its child process idle 46 min) and `api --test cloud_mutants_bars`
  `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (idle 18 min; stopping it
  ended that binary, so its later tests did not run). Of the 60, one is in a file this part touched:
  the root-only `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`
  (a test of the base, unchanged). The rest are in files this part does not touch; most are refusals a
  0555 directory gives a non-root user (this container runs as uid 0) and slow-reader deadline tests.
  NEXT (when released): run the 60 by name on the unmodified base `7b397bbe` to show each red there
  too, re-run the rest of `cloud_mutants_bars` with the hung one skipped by name, coverage and
  mutants on the touched modules, the shell gates once more, then STATUS: DONE.
  Failing tests:
  `a_run_whose_sink_is_held_refuses_before_any_lock`
  `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`
  `detail::tests::a_refused_admission_names_what_its_bound_and_why`
  `gdfl_cm::tests::fixtures_are_built_not_pasted`
  `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`
  `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`
  `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`
  `gdfl_import_command::cloud_mutant_tests::the_receipt_names_the_workers_and_durability_it_filed_with`
  `gdfl_import_command::guard::tests::a_change_is_never_accepted_by_a_run_that_skipped_no_day`
  `gdfl_import_command::guard::tests::a_changed_archive_with_every_day_done_is_named_not_ok`
  `gdfl_import_command::guard::tests::a_run_that_read_no_file_is_not_accepted_by_running_it_again`
  `gdfl_import_command::guard::tests::a_source_root_that_vanishes_after_files_were_read_is_named_source_lost`
  `gdfl_import_command::guard::tests::an_empty_range_owes_nothing_and_exits_ok`
  `gdfl_import_command::guard::tests::every_day_already_refused_identically_still_fails_when_the_root_is_gone`
  `gdfl_import_command::guard::tests::one_archive_reached_by_another_spelling_is_the_same_key`
  `gdfl_import_command::tests::a_dangling_day_file_link_ends_the_run_failed_never_a_day_not_held`
  `gdfl_import_command::tests::a_named_failure_or_a_refused_run_exits_failed`
  `gdfl_import_command::tests::a_run_says_a_line_a_day_and_leaves_its_progress_record`
  `gdfl_import_command::tests::a_run_that_read_no_file_never_exits_ok`
  `gdfl_import_command::tests::an_unstamped_build_is_refused_a_store_write_unless_allowed`
  `gdfl_import_command::tests::the_retry_flag_is_never_a_symbol_and_the_run_line_names_the_source`
  `gdfl_import_command::tests::the_stored_flag_narrows_a_capital_market_run_to_what_d3333_adds`
  `gdfl_run::tests::a_clock_step_back_never_aborts_a_run`
  `gdfl_run::tests::a_progress_file_that_cannot_be_written_is_named_and_the_run_goes_on`
  `gdfl_run::tests::the_kinds_run_in_order_on_the_calling_thread`
  `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`
  `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`
  `logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code`
  `pullrun::restart_tests::the_journals_own_extremes`
  `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`
  `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`
  `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
  `server::head_deadline_body_tests::a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole`
  `server::head_deadline_body_tests::a_chunked_body_reaches_a_slow_reader_whole`
  `server::head_deadline_body_tests::a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole`
  `server::head_deadline_body_tests::pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is`
  `server::head_deadline_body_tests::slow_readers_at_the_cap_are_served_whole_and_the_next_waits`
  `settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`
  `settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`
  `settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`
  `sink::tests::a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`
  `sink::tests::a_failed_delete_is_counted_named_and_left_for_the_next_start`
  `sink::tests::a_rename_aside_that_cannot_happen_fails_the_roll_by_name`
  `sink::tests::an_audit_stream_that_cannot_be_looked_for_is_an_error`
  `tick_second_check::tests::a_lock_name_that_is_a_link_a_directory_or_a_second_name_is_refused_and_nothing_goes_through_it`
  `tick_second_check::tests::a_month_held_only_sealed_is_checked_row_for_row_as_its_raw_form_was`
  `tick_second_check::tests::a_new_landing_of_a_checked_day_is_checked_again`
  `tick_second_check::tests::a_pipe_or_a_link_at_one_of_the_checks_own_names_and_a_pipe_archive_are_refused_at_once`
  `tick_second_check::tests::a_share_directory_the_store_refuses_is_one_cell_and_files_nothing_twice`
  `tick_second_check::tests::a_share_whose_months_cannot_be_listed_is_filed_month_by_month_with_or_without_a_year`
  `tick_second_check::tests::a_tick_store_landing_is_checked_against_that_tick_store_day_and_a_changed_day_is_refused`
  `tick_second_check::tests::a_version_whose_rows_are_not_the_ones_its_index_names_is_refused`
  `tick_second_check::tests::an_extra_archive_is_compared_beside_the_landing_and_labelled_not_the_landing_source`
  `tick_second_check::tests::every_status_and_source_is_named_against_the_landing_the_store_recorded`
  `tick_second_check::tests::no_worker_count_and_no_resumption_changes_a_byte_of_the_version`
  `two_cli_kinds_at_once_both_log`
  `watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many`
  `watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`
  `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`
  `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named`
