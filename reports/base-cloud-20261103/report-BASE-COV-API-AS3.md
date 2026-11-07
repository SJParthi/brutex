STATUS: IN PROGRESS

# BASE-COV-API-AS3 (shard G3 of `SHARDS.md`, BASE-COV-API repair r3): report

- Branch `cloud/cov-api-as3-20261105`, cut from the kit commit `f4111a0132473d8771d8624a9538b1362926f70b`; START
  `6ea46e59c07488eeccff388407976041852aae2c`. Code tip `4a86f4085b6ec5d0303b5e4de5a0a99801c868fe` (what
  `git rev-list -1 HEAD -- . ':(exclude)reports'` prints).
- Sessions: token `-1` pushed to `7700e2f0`; the main session paused the shard at `edd54703`; token `-3`
  (`a5f009a8`) worked to `44a0a656` and was cut by the account's weekly limit (Thu 5 Nov, about 21:28 IST): its
  helpers' unpushed work on `mastersrun.rs`, `optiongreeks.rs`, `autopilot.rs`, `selectionv6json.rs` and a saved
  Boolean evidence probe was lost with their container. Token `-4` (`7b6102cb`, 6 Nov) worked the first three of
  those again and was cut by the weekly limit on Fri 6 Nov after `3055774c`, its report at IN PROGRESS. This
  continuation holds `BASE-COV-API-AS3-20261105-5` (`7694e19b`, Sat 7 Nov).
- Host: a fresh cloud container, 4 cores, 15 GB, root, Linux x86_64. Pinned nightly installed as `nightly-2026-08-08`:
  `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`; llvm-cov and llvm-profdata `LLVM version 23.1.0-rust-1.99.0-nightly`.
  Every kit file's sha256 matched `MANIFEST.md`. Every figure here is this host's, the builder's: the Mac re-measures.

## What continuation -5 did (`7694e19b`..`4a86f408`, 7 Nov)

- **NEXT 0, the late red, found and fixed** (`32fbc1ab`). It was the test, not the host's day:
  `an_unpaused_live_autopilot_journals_a_tick_whose_every_instrument_was_refused` read the NEWEST NOT STARTED record
  (`journal_records` reads newest first, a page of at most `audit::MAX_PAGE_RECORDS` = 200), while `autopilot::fly`
  keeps ticking: a Zerodha month no instrument maps to is dry (no reason), so `FeedState::observe` asks it again and
  then passes it with no wait (`DRY_ROUNDS` = 2), month after month from Zerodha's fixed floor (2015-01), until Groww's
  fixed floor month (2020-01) ties it and Groww, earlier in `Feed::ALL`, wins the tie and is refused by the vendor. A
  host fast enough to run those 120 dry ticks between two 250 ms polls handed the test Groww's record (the loaded
  Route B runs of 6 Nov were slow enough to pass). Reproduced alone on this host, 2 of 2, at `:2492` with Groww's
  record (`from_days` 18262). The test now reads the journal's OLDEST page and asserts the first tick's window is
  Zerodha's floor month (2015-01-01..31); green 3 of 3 alone and in both full runs of this round. Production code
  unchanged.
- **The flaky `logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code`, found and fixed**
  (`32fbc1ab`): the refusal and the 200 noise lines after it were stamped inside one millisecond on a tmpfs, so the
  ring's oldest kept line carried the refusal's own stamp and `audited_from < ring_from` (`logs.rs:10249`) failed:
  5 of 12 runs alone before. A 2 ms sleep after the refusal moves the clock first: 20 of 20 after.
- **NEXT 5** (`33fe1754`): an `Extremes:` line on each of the four new tests that lacked one; the feeds_route
  small-size wrapper `the_held_span_bench_runs_at_a_small_size` now runs the bench's own path at the sizes where its
  answer moves (2, 15, 19 keys) and with its scales handed largest first, and names 8 singles, 2 combinations and 1
  out of the box of its own (verify r3 item 3 for that file).
- **NEXT 3** (`8b311fdc`): `bars_export.rs` 974 by
  `an_end_read_that_cannot_open_its_month_refuses_the_export_by_name`: two series over three months, exported once
  so every month's header is memoized by its stamp; one series' July zeroed over its whole header in place with its
  length, inode and modification time kept, so the plan takes the memo's word and the ascending first page reads
  only May and June; the descending end read is the first read that opens that July and the export is refused (400
  naming the month). With the stamp moved, the plan opens the month itself and the first page refuses the export as
  not whole (500): the same damage is a 400 from the end read and a 500 from the plan (a follow-up for the operator,
  not changed here: production behaviour).
- **NEXT 1 in part** (`042384a3`, `4a86f408`): `indexstopqualificationjson.rs` 236 (`rank`'s Rejected arm) by
  `a_forty_session_training_window_is_measured_whole_and_ranked_rejected`, a second production launch beside the
  saved probe: April and May 2025 for training (forty open sessions: every split value measured, where May alone,
  21 sessions, leaves values unmeasured), June later, March generated for the launch's first read (without it the
  launch is refused "No such file or directory"). The two launch tests share one parent half.
  - Admitted (235) not reached: a policy file with every floor at its least the policy admits (1 for the structural
    counts, 0 elsewhere, every ceiling at its domain's top, both rejection requirements false) still fails
    `worst_reward_risk` and `worst_trade_loss` on the generated minutes (signal-candle stops lose more than the risk).
  - Refused (237) not reached: a minute left out of a training session is refused by the calendar receipt before
    any metric ("offered 7874, expected 7875, missing 1"), so a Refused verdict needs an execution the strict load
    admits and the runner still marks incomplete (`metrics::execution_complete`, or a day the calendar assessment
    counts unmeasured).
- **NEXT 2 not done**: `indexstopcandlesjson.rs` 117 True, 118, 120 need a source page over 8 MiB. No production
  writer makes one: every source context it seals takes its condition names from `vocab::table::TABLE`, its
  expression from at most `vocab::expression::MAX_INSTRUCTIONS` (1,151) instructions, and its candles from a window
  `Asked::parse` bounds below `detail::MAX_RESULT_ROWS` (4,096), so its page stays under about 2 MB. The reader takes
  the names from the saved image (`index_stop_source_context::selection`), so a test reaches it only by re-sealing a
  launch's context, link and catalog with the keyless digests the reader recomputes, by hand: the earlier plan in
  its `gaps-AS3.txt.md` row stands, and the rows stay "reachable, not done" (not (U): a file reaches them).
- **NEXT 4 tried, not landed**: the withdrawn Boolean evidence probe (`050fd625^`) run again in a local scratch tree
  with the forty-session window: the production launch writes its saved Boolean qualification about four minutes in
  at opt-level 3 (statistics, admission, OOS, qualification, index consistency all saved), then was still running at
  100% of a core past 13 minutes and was stopped. At Route B's opt-level 0 that is hours: a probe needs a lighter
  configuration first. `selectionv6json.rs` has a public way in, `cli::run(["ledger-v6", ...])`, which sweeps both
  indices at every rung: not tried.
- Red under a hand change for every new assert of this continuation (run red5, `red-AS3.txt.md`); clippy, fmt, the
  stable suite and Route B of record on the code tip `4a86f408`; `gaps-AS3.txt.md` and `fu-AS3.txt.md` rewritten from
  that measurement (each earlier row carried over through a line map from `git diff 26003a03 4a86f408`, its text
  checked; every row that did not carry over classified by hand).

## Why IN PROGRESS

Every uncovered row of a G3 file is now (F), (U) with its proof, (H), or a GAP row that names what it is owed to,
except the rows still G3's to reach (`gaps-AS3.txt.md`, "reachable, not done" and the L-BCA-5 renders):
`indexstopqualificationjson.rs` 235 and 237 (`rank`'s Admitted and Refused arms: the launches tried this round
reach Rejected only), `indexstopcandlesjson.rs` 117 True, 118, 120 (a hand re-sealed source context), and the two
saved-artifact renders `booleanevidencejson.rs` (184 lines 13 sides: the production launch writes its artifact but
ran past 13 minutes at opt-level 3) and `selectionv6json.rs` (9 lines 2 sides: the public `cli::run` ledger-v6 verb,
not tried). NEXT names each with its way in. Every gate is green on the code tip except the 13 host reds, each
proved; the 8-extremes rule holds for every named file.

## What continuation -4 did (`7b6102cb`..`26003a03`, 6 Nov)

- `mastersrun.rs`: 3 new tests (a refetch that changed nothing and a reprimed step, each named; a reload that would
  lose a readable feed is refused and logged; a refresh task that does not return is a 500 that says where to
  look); the refresh child reads its summary record against its page and drives `refresh` itself; its test code
  rewritten branch-free.
- `autopilot.rs`: 2 new tests (a probe name the host refuses keeps the host's own words: 8 singles, 2 combinations,
  1 out of the box; a live loop over a loopback vendor that answers no bars refuses the empty answer by name, backs
  the month off and keeps passing); the switch child reads a case table; the probe-halt test now asserts the store
  root stays empty (its listing closure ran only for an entry, and there is none).
- `optiongreeks.rs`: new test file `optiongreeks_cover_tests.rs`, 10 tests.
- `frontierjson.rs`: a frontier whose absence cannot be checked (a file root, a `results` file, a NUL root, an empty
  reason) is refused by name; `every_edge_of_a_page_answers_or_refuses_by_name` kept as verify r3 found it.
- `ingest.rs`: `discovery_ask` (L-BCA-7: nothing calls it) asked directly in seven forms, among them a named expiry a
  month past its window's end and a January expiry past a December window, so a reading of the window's end is red.
- `logs_cover_tests.rs`: a window after kept-out server errors opens with their count (WP-33c), in a child.
- `indexstopqualificationjson_saved_tests.rs` (the `-3` session's production-path launch probe): the kept
  qualification reader answers only its own root; the ranking refuses a budget its admitted bytes already fill, and
  none; the kept candle reader asked at another root refuses by its code, through `indexstopcandlesjson::render_at`,
  a `#[cfg(test)]` door (the handler always passes the process's one store root).
- `indexstopjson_cover_tests.rs` and `indexstopqualificationjson.rs`: the page-kind loop and the value-text refusal
  widened to 8 extremes, 2 of them at once, 1 out of the box.
- `ops_safety_tests.rs`: `month_get`'s retry, reached before only while another test held the store-read pool,
  driven in a child that holds every slot, waits until the refusal is counted, then frees them.
- `pool_table_tests.rs`: the wait for the reader that left uses `server_cloud_mutant_tests_c::eventually` (now
  `pub(super)`), not a loop whose not-yet side ran only on a slow host.
- `bars_route.rs`: `past_saturation`'s refusal is an assert. Clippy refusals fixed in `census_request_tests.rs`,
  `indexstoplaunch_tests.rs`, `http_admission_tests.rs` and 19 rows of the earlier sessions' code.
- Red under a hand change for every new test, the (U) proofs and the owed reasons, the root host reds proved as uid
  65534, Route B of record, `gaps-AS3.txt.md`, `fu-AS3.txt.md`, `red-AS3.txt.md`.

No production behaviour changed on the branch: outside test modules and test files, the diff from START adds only
`#[cfg(test)]` module declarations, doc comments and the test-only `render_at`.

## Files

All G3's: the 48 `SHARDS.md` gives G3 and 8 new test files, each declared in the production file it tests (new:
`census_rollup_cover_tests.rs`, `dayflags_cover_tests.rs`, `indexstopjson_cover_tests.rs`,
`indexstopqualificationjson_saved_tests.rs`, `ingest_cover_tests.rs`, `logs_cover_tests.rs`,
`optiongreeks_cover_tests.rs`, `sweeprun_cover_tests.rs`). From START, all under `crates/api/src/`:
`autopilot.rs` +187 −36, `bars_export.rs` +708 −100, `bars_route.rs` +574 −93, `bars_series.rs` +517 −16,
`bars_window_route_tests.rs` +130 −30, `booleanevidencejson_tests.rs` +42 −7, `census_request_tests.rs` +64 −61,
`census_rollup.rs` +12 −0, `census_rollup_cover_tests.rs` +393 −0, `dayflags.rs` +17 −13, `dayflags_cover_tests.rs`
+1812 −0, `dayflags_filter_tests.rs` +45 −3, `feeds_route.rs` +574 −23, `frontierjson.rs` +234 −0, `gaps_route.rs`
+24 −13, `http_admission_tests.rs` +102 −0, `imports.rs` +407 −0, `indexstopcandlesjson.rs` +9 −0, `indexstopjson.rs`
+4 −0, `indexstopjson_cover_tests.rs` +53 −0, `indexstoplaunch_tests.rs` +556 −38, `indexstopqualificationjson.rs`
+54 −2, `indexstopqualificationjson_saved_tests.rs` +619 −0, `ingest.rs` +107 −25, `ingest_cover_tests.rs` +321 −0,
`ingest_refusals.rs` +86 −0, `ladder.rs` +47 −31, `logs.rs` +116 −48, `logs_cover_tests.rs` +922 −0,
`logs_units_tests.rs` +7 −4, `mastersrun.rs` +316 −27, `month_change.rs` +52 −0, `nse_identity.rs` +8 −8,
`ops_safety_tests.rs` +144 −19, `optiongreeks.rs` +4 −0, `optiongreeks_cover_tests.rs` +556 −0, `owed_tests.rs`
+9 −7, `pool_table_tests.rs` +51 −6, `serve_edge_tests.rs` +53 −13, `server_cloud_mutant_tests_b.rs` +109 −38,
`server_cloud_mutant_tests_c.rs` +128 −58, `server_cloud_mutant_tests_d.rs` +4 −5, `strict_sweep_tests.rs` +25 −26,
`sweepevidence.rs` +227 −0, `sweeprun.rs` +65 −66, `sweeprun_cover_tests.rs` +821 −0,
`sweeprun_status_cache_tests.rs` +8 −8 (47 files). Outside ownership: none. Never touched: `crates/api/src/lib.rs`,
`crates/api/benches/`, `Cargo.*`, `docs/`, `.github/`, another group's file. Reports: this file,
`fix-BASE-COV-API-AS3.json.md`, `OWNER-BASE-COV-API-AS3.md`, `gaps-AS3.txt.md`, `fu-AS3.txt.md`, `red-AS3.txt.md`.

## Tests by name (111 new since START, by file under `crates/api/src/`)

- `autopilot.rs` (2): `a_live_loop_over_an_empty_vendor_refuses_it_by_name_and_keeps_passing`, `a_probe_name_the_host_refuses_keeps_the_hosts_own_words`
- `bars_export.rs` (6): `a_full_scan_pool_refuses_an_export_by_name_and_it_is_written_once_the_pool_frees`, `an_end_read_that_cannot_open_its_month_refuses_the_export_by_name`, `a_page_read_past_a_full_pool_is_read_and_a_task_that_will_not_join_cuts_the_stream`, `a_refusal_before_a_byte_is_the_windows_and_a_name_is_always_a_file_name`, `a_refused_read_is_named_and_one_past_the_cheap_budget_is_asked_again_as_a_scan`, `every_page_that_cannot_be_written_whole_cuts_the_stream_by_name`
- `bars_route.rs` (4): `a_time_of_day_of_one_field_or_four_is_refused_and_an_open_end_runs_to_the_last_second`, `a_value_past_a_browsers_number_is_withheld_by_name_and_a_month_dropped_in_part_is_served`, `a_window_names_damaged_records_withholds_an_inexact_value_and_counts_coverage`, `an_abandoned_scan_stops_and_an_answered_one_cancels_nothing_past_a_full_pool`
- `bars_series.rs` (4): `a_journal_that_cannot_be_read_moves_no_index_life`, `a_plan_serves_every_month_it_can_and_names_every_one_it_cannot`, `a_summary_whose_days_do_not_add_up_is_refused_by_name`, `an_instant_before_the_month_and_a_unit_past_the_span_answer_nothing`
- `bars_window_route_tests.rs` (1): `a_window_refused_by_a_full_scan_pool_is_asked_again_and_answered_there`
- `booleanevidencejson_tests.rs` (1): `the_scanner_reads_unclosed_and_look_alike_tokens_as_the_rule_needs`
- `census_rollup_cover_tests.rs` (3): `a_span_the_calendar_cannot_count_is_none_never_zero`, `an_answer_carries_the_cell_bound_and_is_refused_one_past_it`, `every_exchange_word_is_read_resolved_or_refused_by_name`
- `dayflags_cover_tests.rs` (17): `a_contract_the_store_cannot_name_is_refused_and_a_shaped_tail_must_parse`, `a_day_bar_month_that_will_not_read_is_said_on_each_of_its_days`, `a_folder_or_a_file_where_a_source_goes_is_named_never_read_as_empty`, `a_journal_past_one_page_is_read_to_its_last_page`, `a_journal_replaced_or_cut_back_under_its_tail_is_walked_again`, `a_ledger_is_read_through_its_tail_whatever_is_done_to_it`, `a_ledger_line_this_reader_cannot_name_is_refused_with_its_reason`, `a_month_the_store_cannot_name_or_open_is_refused_and_starts_nothing`, `a_pull_record_is_kept_only_where_it_names_a_refusal`, `a_record_that_cannot_hold_a_note_keeps_what_it_holds`, `a_walk_stops_where_the_file_stops`, `each_odd_day_is_said_for_what_it_is`, `each_phrase_a_days_sources_owe_is_said`, `each_refusal_is_placed_by_its_time_counted_once_and_named_to_the_bound`, `the_envelope_names_every_source_that_would_not_read`, `the_lineage_ledger_is_read_again_when_it_changes_and_a_refused_one_is_said`, `through_the_index_a_wrongly_named_record_is_a_fault`
- `dayflags_filter_tests.rs` (1): `the_words_for_refusals_no_log_holds_are_the_routes_for_one_and_for_many`
- `feeds_route.rs` (5): `the_archive_evidence_and_the_store_rows_name_every_shape_they_meet`, `the_descriptor_facts_behind_five_branches_hold_feed_by_feed`, `the_held_span_answers_at_every_edge_of_its_census`, `the_import_reach_reads_every_shape_its_journal_lines_take`, `the_window_under_degraded_settings_is_served_only_from_a_generation`
- `frontierjson.rs` (2): `a_frontier_whose_absence_cannot_be_checked_is_refused_by_name`, `every_unasked_frontier_answer_is_given_or_refused_by_name`
- `http_admission_tests.rs` (1): `a_journaled_read_asked_while_every_slot_is_held_waits_for_one`
- `imports.rs` (5): `a_close_a_recount_and_what_no_row_reads_are_each_what_they_are`, `a_folder_whose_name_is_not_utf8_is_counted_never_dropped`, `a_journal_that_cannot_be_read_is_named_never_empty`, `a_run_found_nowhere_names_a_folder_it_cannot_list_and_passes_over_a_file`, `the_log_alone_moving_moves_the_summary_and_a_logged_stage_settles_a_day`
- `indexstopjson_cover_tests.rs` (1): `a_second_root_reopens_and_a_page_kind_no_query_names_is_refused_by_name`
- `indexstoplaunch_tests.rs` (5): `a_foreign_command_and_a_count_of_no_or_too_many_digits_are_refused_before_any_read`, `a_launch_whose_slot_moved_and_a_preview_that_cannot_run_are_refused_by_name`, `a_preview_past_its_observation_records_or_store_bounds_names_each_one`, `every_incoherent_observation_is_refused_by_name_and_its_coherent_edges_pass`, `the_configuration_preview_door_answers_json_when_nothing_is_configured`
- `indexstopqualificationjson.rs` (1): `a_refusal_names_a_value_by_its_own_text`
- `indexstopqualificationjson_saved_tests.rs` (2): `a_forty_session_training_window_is_measured_whole_and_ranked_rejected`, `a_saved_qualification_is_served_by_every_page_that_reads_it`
- `ingest.rs` (1): `a_discovery_asks_for_the_named_expirys_month_or_the_windows`
- `ingest_cover_tests.rs` (5): `a_clock_at_the_epoch_is_refused_without_naming_a_control`, `a_date_in_pieces_is_finished_or_missing_never_guessed`, `one_named_expiry_keeps_its_own_series_of_that_day_and_no_other`, `the_rate_box_is_read_or_refused_by_name_and_names_its_control`, `what_no_form_reaches_still_says_what_it_is`
- `ingest_refusals.rs` (1): `a_bound_a_short_day_an_unreadable_journal_and_the_route_each_answer_by_name`
- `logs_cover_tests.rs` (13): `a_line_in_the_index_and_the_ring_is_shown_once_and_ring_rows_newest_first`, `a_line_the_keys_do_not_meet_bears_on_nothing`, `a_live_copy_read_under_a_target_or_a_run_counts_no_gap`, `a_merge_shows_a_line_once_and_stops_at_a_head_with_no_place`, `a_ring_scan_names_what_it_cannot_read_and_stops_at_its_budget`, `a_search_stopped_by_a_line_that_will_not_read_or_by_no_budget_still_answers`, `a_span_over_a_folder_where_its_audit_file_should_be_is_refused_by_name`, `a_window_after_kept_out_server_errors_opens_with_their_count`, `an_audited_refusal_of_a_day_is_the_importers_error_and_nothing_else`, `each_key_probes_its_own_posting`, `evidence_held_in_two_directories_merges_oldest_first_and_names_an_attempt_once`, `the_importers_ring_is_folded_and_each_line_filed_where_it_belongs`, `the_page_form_keeps_each_span_end_and_the_ring_question`
- `mastersrun.rs` (3): `a_refetch_that_changed_nothing_and_a_reprimed_step_are_each_named`, `a_refresh_task_that_does_not_return_is_a_500_that_says_where_to_look`, `a_reload_that_would_lose_a_readable_feed_is_refused_and_logged`
- `month_change.rs` (1): `the_look_back_stops_at_the_first_month_and_never_looks_forward`
- `ops_safety_tests.rs` (2): `a_build_label_is_unstamped_or_a_whole_commit`, `a_month_asked_while_every_store_read_slot_is_held_is_asked_again`
- `optiongreeks_cover_tests.rs` (10): `a_contract_date_of_the_wrong_widths_is_not_an_option_segment`, `a_greeks_read_the_store_pool_cannot_admit_is_a_429`, `a_rate_table_named_by_another_path_is_read_from_it`, `a_record_that_will_not_read_is_a_fault_on_its_own_cell`, `a_stamp_the_rate_calendar_does_not_hold_is_a_tenor_refusal`, `an_option_month_a_writer_holds_is_being_written_never_unreadable`, `an_option_month_that_is_not_a_bar_file_is_a_named_fault_and_a_206`, `every_expiry_the_instrument_model_admits_is_settled`, `exchange_segment_and_symbol_are_each_required_by_name`, `the_plain_answer_is_the_cancellable_one_never_cancelled`
- `pool_table_tests.rs` (1): `the_table_readers_read_every_literal_shape_and_the_row_builder_runs`
- `serve_edge_tests.rs` (1): `the_print_scan_judges_a_macro_name_as_a_whole_word`
- `server_cloud_mutant_tests_b.rs` (2): `a_script_with_no_answer_left_refuses_as_a_transport_failure`, `the_visible_part_of_a_receipt_starts_at_its_first_content_section`
- `sweepevidence.rs` (2): `a_saved_attempt_gone_empty_or_damaged_is_answered_as_such`, `an_attempt_replaced_between_its_two_reads_is_refused_never_mixed`
- `sweeprun_cover_tests.rs` (8): `a_command_this_conductor_does_not_run_and_a_strict_knob_are_answered_by_name`, `a_finisher_with_no_audit_runs_bare_and_an_empty_finish_closes_failed`, `a_press_no_monitor_could_see_or_that_meets_a_running_sweep_is_refused`, `a_rewalk_that_no_longer_ends_on_the_marker_keeps_the_whole_windows_verdict`, `a_store_that_cannot_be_named_reads_no_cli_log_and_refuses_the_strict_command`, `a_strict_command_with_its_configuration_is_conducted_by_the_strict_audit`, `an_admission_over_a_store_that_is_not_there_is_refused_by_name`, `the_status_door_answers_each_observation_by_what_it_can_prove`

Removed: none. Test helpers are not counted. Existing tests changed: in continuation -5,
`server::cloud_mutant_tests_c::an_unpaused_live_autopilot_journals_a_tick_whose_every_instrument_was_refused` (reads
the journal's oldest page; a new first-tick window assert), `logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code`
(a 2 ms sleep before the noise), `server::feeds_route::tests::the_held_span_bench_runs_at_a_small_size` (two more
runs of the bench's path and four asserts), and doc-only `Extremes:` lines on the two bars_series tests, the
indexstoplaunch preview door and the saved qualification probe (whose parent half is now the shared
`in_a_launch_child`); in continuation -4 the probe-halt test of `autopilot.rs` and
`pool_table_tests::a_read_stops_when_its_reader_leaves` (its wait); in the earlier sessions, among others, the
bars_series summary test,
`indexstopqualificationjson::tests::pins_scope_period_and_bounded_paging_are_required_before_any_read` and
`indexstopcandlesjson::tests::original_source_requests_are_pinned_exact_and_bounded_before_file_reads`.

## Red under a hand change

`red-AS3.txt.md` holds each new test's hand change and the assertion it failed at, all from local commits that were
never pushed: run 1 (93 tests, tree `c8a3d979` over `2e742873`, "0 passed; 93 failed"), run 2 (the 4 tests added
after it and the new asserts of 2 widened tests: trees `532b881e` and `a9118762` over `d1981ec9`), run 3
(`month_get`'s retry and the candle reader's new assert: tree `fca3d025` over `7d7d243f`), run red5 (continuation
-5: the 2 new tests and the new asserts of 2 changed tests, four hand changes at once: tree `dfc51d90` over
`4a86f408`, "0 passed; 4 failed"). The `-3` session's 13 are in its report (`red3`, `red4`), 11 of them new tests.
93 + 4 + 1 + 11 + 2 = all 111 new tests, each red at the assertion its change aimed for.

## Gates on the code tip `4a86f408`

- `cargo fmt --all --check`: clean (checked before every code commit of this continuation).
- `cargo clippy -p api --all-targets --locked -- -D warnings` (stable 1.97.1, `CARGO_TARGET_DIR=/var/tmp/as3/K2`, on
  the tree `4a86f408` commits): clean, exit 0. Its first run refused two rows of this continuation's own code (an
  unused generator after the launch tests' refactor, and `similar_names`), fixed in `4a86f408`.
- `cargo test -p api --locked --no-fail-fast -- --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
  (stable 1.97.1, `CARGO_TARGET_DIR=/var/tmp/as3/K2`, TMPDIR a 1777 tmpfs folder, a stamped build of the committed
  tree): exit 101, passed 3189 failed 13 ignored 16 over 9 result lines: the lib target 3105 / 13 / 16; the main
  target, the 6 integration targets (the load-bound `pool_load` among them) and the doc tests green. The 13 are the
  host reds below (G3's three root reds and ten of other shards); no G3 test fails for a G3 reason.
- The same tests under Route B (instrumented, nightly, opt-level 0): `tests passed 3189 failed 13 ignored 16` (7 min),
  the same 13.
- Benches: none run this round (no p50 or p99 to give). The feeds_route small-size wrapper runs in the suite with
  its ratio gate lifted.
- Mutants: deferred (operator, 30 Oct): not run; each new test was shown red under a hand change instead.

## Coverage: Route B of record, whole:api

The kit's `d3-route-b-cloud.sh`, unchanged (sha256 checked against `MANIFEST.md`), with `D3_W=/var/tmp/as3`, the
pinned nightly and its own llvm tools, on a clean worktree checked out at the code tip:
`d3-route-b-cloud.sh /var/tmp/as3/wt-rb5 /var/tmp/as3/K1 /var/tmp/as3/P8 /var/tmp/as3/L8 <set> api -- --skip
spot_json_is_refused_by_name_when_every_store_read_slot_is_held`. The set is the kit's
`deep-BASE-COV-API-6ea46e59c074-set.txt` plus one `coverage <file> whole` line for each of the 8 files the branch
added (whole:api names every file; the kit's set predates them): 213 coverage lines. TMPDIR was a 1777 tmpfs folder,
and the profile folder was made 1777 and the script run under `umask 0000`, so the children
`isolated::where_permission_binds` re-runs as uid 65534 write their profiles (CLOUD.md item 10). Started 15:38:02 IST
7 Nov; build 3 min (cold target), tests 7 min, readout done 16:04:29 IST; exit 3 for NOT PROVED: the 22 benches and
`lib.rs` only (L-BCA-2). No test process ended by a signal.

- Whole api (the 190 files read): lines 179147/183008 (3861 uncovered), branches 13021/15386 (2365 uncovered sides).
- G3 (56 files): lines 63260/63962 (702 uncovered), branches 4726/5124 (398 uncovered sides). Classified strictly
  by recipe step 4:
  - (F) 235 lines 312 sides; (U) 97 lines 52 sides, each with its proof; (H) 44 lines 2 sides (`fu-AS3.txt.md`);
  - GAP 326 lines 32 sides (`gaps-AS3.txt.md`): in `#[ignore]`d tests 86 lines (L-BCA-3, the Mac); L-BCA-5 193 lines
    15 sides (`booleanevidencejson.rs` 184/13, `selectionv6json.rs` 9/2); L-BCA-6 18 lines 7 sides
    (`mastersrun.rs`); the host clock 18 lines 4 sides (`autopilot.rs`); a race 4 lines 4 sides (`dayflags.rs`
    3220, 3322, 3323; `indexstopcandlesjson.rs` 188-191; `bars_export.rs` 979); a failing medium or a cut append 3
    lines 1 side (`imports.rs` 1342-1344, `optiongreeks.rs` 558); reachable and not done 4 lines 1 side
    (`indexstopqualificationjson.rs` 235, 237; `indexstopcandlesjson.rs` 117 True, 118, 120); test code: none.
- Against `26003a03`: 2 production lines closed (`bars_export.rs` 974, `indexstopqualificationjson.rs` 236), and the
  4 rows the new code added are (F) (2 sides of a new assert) or (U) (the new launch test's unstamped arm, the proof
  the saved probe's carries). Every other row carried over through a line map from `git diff 26003a03 4a86f408`, its
  source text checked (1,100 rows); two GAP reasons this round's evidence corrected were rewritten (235, 237).
- The (H) rows: the three G3 root reds failed as root in this run and passed as uid 65534
  (`setpriv --reuid=65534 --regid=65534 --clear-groups`) with this run's instrumented binary
  (`api-bbc8861d417950e8`): `test result: ok. 1 passed` each.

The `--skip`: `cloud_mutants_bars.rs` `spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (G2's file,
unchanged from START) hangs on this host: it fills `detail::MAX_STORE_READ_CONCURRENT` holders behind a barrier the
pool can never fill here (the pool has one slot a core: 4 on this host). Its rows are G2's.

## The 8-extremes count of each named file (singles / combined / out of the box, from the `Extremes:` lines)

| file | test | count | verify r3 |
|---|---|---|---|
| `bars_series.rs` | `a_summary_that_does_not_describe_its_file_is_refused_field_by_field` | 11 / 3 / 2 | 9 / 1 / 0 |
| `feeds_route.rs` | `the_held_span_bench_runs_at_a_small_size`, on its own `Extremes:` line (the bench path at 20/30/40, 2/15/19 and 40/30/20 keys); `the_held_span_answers_at_every_edge_of_its_census` beside it adds 6 / 2 / 3 | 8 / 2 / 1 | none declared |
| `frontierjson.rs` | `every_edge_of_a_page_answers_or_refuses_by_name` | 10 / 2 / 2, kept | 10 / 2 / 2 |
| `mastersrun.rs` | `a_refresh_with_no_credential_lands_the_public_masters_and_says_it_is_not_whole` | 9 configurations / 2 / 1, kept | 9 / 2 / 1 |

Of the 111 new tests, all 111 name their extremes on an `Extremes:` line; of the four lines continuation -5 added, the three whose own count is short name the test that carries their file's count.

## Host reds (each with its proof)

- Root only, G3's three: `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`,
  `server::cloud_mutant_tests_c::a_store_that_takes_no_write_trips_the_store_breaker`,
  `server::cloud_mutant_tests_c::a_store_whose_audit_dir_cannot_be_made_is_at_fault`. Each failed as root in both
  runs of this round and passed as uid 65534 (`setpriv --reuid=65534 --regid=65534 --clear-groups`) with this
  round's instrumented binary (`api-bbc8861d417950e8`, 7 Nov; as with `api-7b94fb89521800a1` on 6 Nov):
  `test result: ok. 1 passed` each. The rows only their later lines reach are (H): covered as non-root, owed to the
  Mac's Route B.
- 4-core hang: the G2 test above (skipped).
- Failing as root here and not in G3's files (not proved by this session; other shards' and the Mac's):
  `audit::cloud_mutant_tests::an_unreadable_journal_refuses_the_index_build_and_says_so`,
  `detail::tests::a_refused_admission_names_what_its_bound_and_why`,
  `pullrun::cloud_mutant_tests::newest_records_that_cannot_be_read_leave_the_answer_not_known`,
  `pullrun::restart_tests::the_journals_own_extremes`,
  `server::basis_route::tests::a_table_refused_as_unreadable_is_read_again_once_its_mode_is_repaired`, five
  `server::head_deadline_body_tests` slow-reader tests (these ten all failed again in this round's Route B run), and the integration test
  `the_feed_list_stays_fast_while_32_clients_loop_each_route` (it passed in this round's Route B run).
- Late reds of 6 Nov, both in G3 files: fixed by continuation -5 (`32fbc1ab`), neither a host red. The autopilot
  tick test's red was the test's own read (the journal's newest page while the loop kept ticking dry Zerodha months
  up to Groww's floor; continuation -5 above); the `logs::r3_24_tests` flake was its refusal and its noise sharing one
  millisecond on a tmpfs. Both green in the Route B run and in the stable run of record of this round.

## Decision drafts (ids D-8664 and D-8665 only; ASA writes the docs entries)

- **D-8664 (draft): shard G3's new tests, and the defects they fixed, outside `feeds_route.rs`.** The 111 test
  functions this round added in G3's files (listed by file above), each driving the input of a row of
  `base-cov-api-gaps-r2.txt.md` or of a G3 row Route B left uncovered on the way; each shown red under a hand change
  (`red-AS3.txt.md`, and the `-3` session's report for its 13). Cited as "D-8664" in `bars_series.rs` by session 1.
  It records the test fixes made under this id: `gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
  holds `Named::Calendar.pool().max()` slots, not `MAX_CALENDAR_CONCURRENT`, and waits until each holder is inside
  its slot before it asks (the pool has one slot a core up to 8, so on a 4-core host the old test hung on its
  barrier: gdb showed four holders inside their slots); `ops_safety_tests::month_get`'s retry is driven by a child
  that holds the store-read pool instead of only by another test's chance contention; `pool_table_tests` waits
  through the shared `eventually`; the discovery test holds an expiry past its window's month and year; the
  probe-halt test asserts the store root stays empty; the autopilot tick test reads the journal's oldest page and asserts the first tick's window (the loop runs on through dry Zerodha months, and a fast host ran them up to Groww's floor between two polls); the r3_24 refusal is stamped a millisecond before the ring's noise (continuation -5).
- **D-8665 (draft): `feeds_route.rs`'s verify-r3 item (3) and its new tests; G3's test code rewritten so no row of
  it is a GAP; every (U) row of G3 with its proof; the rows G3 leaves owed outside.** The descriptor facts behind five
  `feeds_route.rs` branches are compile-time (`pull::vendor` descriptors) and are pinned feed by feed by
  `the_descriptor_facts_behind_five_branches_hold_feed_by_feed`: those sides are (U) on that test, and the day it
  stops holding names the branch that then needs its own test. Every other (U) row's proof is in `fu-AS3.txt.md`
  beside the row; none rests on "hard to reach" or "never happens". At this tip no G3 test-code row is a GAP. Cited
  as "D-8665" by session 1 in `feeds_route.rs`, `ops_safety_tests.rs`, `serve_edge_tests.rs` and
  `server_cloud_mutant_tests_b.rs`.
- **Limit drafts for docs/06 (ASA assigns the rows; D-8665 names them):**
  1. L-BCA-5 widened: `selectionv6json.rs`'s saved arm (a sealed Selection V6 record only the production `ledger-v6`
     chain writes; cli's `commit_stored_selection_v6` is `pub(crate)`, though the public `cli::run` runs that chain as
     the `ledger-v6` verb, not tried this round), beside `booleanevidencejson.rs`, which the
     row already names. And narrowed: `indexstopqualificationjson.rs`, `indexstopcandlesjson.rs` and
     `indexstopjson.rs` are reached through the production launch (`indexstopqualificationjson_saved_tests.rs`), so
     the row's "only cli's private test fixtures can write" no longer holds for them, except one side the production
     writer cannot reach: `indexstopcandlesjson.rs` 117 True, 118, 120, a source page past 8 MiB (the production page
     stays under about 2 MB; only a hand-sealed artifact chain could carry an 8 MiB condition name). A production
     launch can write a saved Boolean qualification too (continuation -5: about four minutes in at opt-level 3), but
     the launch ran on past 13 minutes, so the row stands for `booleanevidencejson.rs` until a lighter launch is found.
  2. L-BCA-6 widened: `mastersrun::refresh_work` past `masters_dir` (it fetches from the real public hosts), the
     whole-refresh 200 (it needs the credentialed Zerodha dump), and `pull::masters::PublicFetch::new`'s refusal
     (`mastersrun.rs` 595-603: its roots and TLS provider are compiled in), beside the credentialed success arm the
     row names.
  3. New: the host clock. `autopilot`'s backfill clock wait and `round`'s clock check run only on a clock before the
     epoch or past 9999 (`yesterday_ist(SystemTime::now())` is None); no test may set the machine clock.
  4. New: race-only sides. `dayflags::pull_walked` and `warm_pull_journal` re-check the held tail after the walk (its
     identity or extent changed by a concurrent walk between the two lock holds); `indexstopcandlesjson::render`'s
     second `require_current` (a file replaced between the projection and its recheck); `bars_export`'s end-day read
     (a month appended or replaced between the first page and the end reads). Each is reachable only by a race; no
     hook orders it without restructuring the path (D-6017).
  5. New: a failing medium. `imports.rs` 1342-1344 (a `ReadDir` entry error after the directory opened: only a
     failing disk or a FUSE or network mount gives one) and `optiongreeks.rs` 558 (a premium record stamped 0 or out
     of order inside the committed records: the writer refuses both and the block checksum refuses a damaged one;
     only a cut append's zero-filled extent could hold one).
  6. New (host): `cloud_mutants_bars.rs` `spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (G2's
     file) hangs on a host with fewer store-read slots than `MAX_STORE_READ_CONCURRENT`.

## NEEDS THE MAC

- Route B whole:api on its pin, over the merged tree (this host's figures are the builder's).
- The ignored real-data tests (L-BCA-3): the `GAP ignored` rows of `gaps-AS3.txt.md` (86 lines).
- The (H) rows: G3's three root host reds run as a non-root user there, so their later lines are covered there.
- macOS rows, and the non-root 14-core tests; the load-bound census warm test (L-BCA-4); the G2 test this host skips.
- The shadow D rows and attacks on real data; the D1 landing.

## NEXT (not done; G3's)

1. `indexstopqualificationjson.rs` 235 and 237, `rank`'s Admitted and Refused arms, each a launch beside the two
   launch tests (`in_a_launch_child` takes the months; `launch_body_over` the windows):
   - Admitted (235): every check must pass with measured evidence. A policy file at the least the policy admits
     still fails `worst_reward_risk` (the smallest win must be at least three times the largest loss: the operator's
     own rule, which no policy file sets) and `worst_trade_loss` (the largest loss against the risk) on the generated
     minutes, so the minutes must change: signal candles whose stops lose at most the risk and whose winning
     trades each gain at least three times it
     (for example one strong one-way session pattern a day, the risk raised with `max_loss_points`).
   - Refused (237): an execution the strict load admits and `metrics::execution_complete` still refuses (an entry,
     path or closing refusal, an unreachable trade, an unaccepted minute) or a training day the calendar assessment
     counts unmeasured (a day whose pessimistic and optimistic readings disagree in sign). A missing minute does not
     do it: the calendar receipt refuses the month first.
2. `indexstopcandlesjson.rs` 117 True, 118, 120: the hand re-seal its `gaps-AS3.txt.md` rows spell out (a launch's
   source context with one referenced name made about 9 MiB long, then its context identity and receipt, the
   BRISCL01 relation and the contextual catalog identity recomputed with the keyless digests the reader checks), in a
   child with a large `BRUTEX_BOOLEAN_OBSERVATION_BYTES`.
3. `booleanevidencejson.rs` (184 lines 13 sides): the withdrawn probe (`050fd625^`) with the forty-session window
   (March to June generated, April and May training) writes its saved Boolean qualification about four minutes in
   at opt-level 3 and was still running past 13 minutes; find a lighter launch (fewer bits, a lower node allowance,
   a shorter horizon, any draw count the Boolean statistics take from the environment) before it can be a suite test.
4. `selectionv6json.rs` (9 lines 2 sides): `cli::run(["ledger-v6", VENDOR, FROM_Y, FROM_M, TO_Y, TO_M, SUPPORT_PPM,
   MAX_POINTS, ROOT])` (public) over generated NIFTY and BANKNIFTY months at every rung it reads (the verb pairs the
   two indices per rung), then `render_page` over its root.
5. Mutants stay deferred (operator, 30 Oct): not run.

## Slips

- The `-3` session recorded, in its report: twice a shell command named the Py-language's interpreter (once as a
  version check whose output was discarded); its helper b reported one such slip in its own shell. Nothing was
  written to any file, tracked or not. None in continuation -4.
- Continuation -5, 7 Nov: one shell command (an experiment edit in a local scratch worktree, never pushed) began
  with a version check of the Py-language's interpreter whose output was discarded; nothing ran under it and nothing
  was written by it. The same command made a one-line edit in that scratch tree with perl; nothing of it is tracked.
  Every other edit of this continuation was made with the editor or sed.
- Continuation -5's first build of the revived Boolean probe in the scratch tree was unstamped (its file was still
  untracked), so the probe took its unstamped arm; it was rebuilt from a committed tree. Every launch result in this
  report is from a stamped build of a committed tree.
- The `-3` session's helpers left work unpushed when it was cut; it was lost with their container. `mastersrun.rs`,
  `optiongreeks.rs` and `autopilot.rs` were redone here; the Selection V6 and Boolean evidence probes were not
  (NEXT 4).
- This continuation's first measurement (`7b6102cb`) ended the hanging G2 test with `exit(0)` from gdb so its binary
  wrote its profile; the measurement of record skips that test instead.
- This continuation's classifier first read every line of a multi-line check after its first as a failing arm; the
  rows read by hand showed a closure inside a condition is none, and the rule was narrowed to the message arguments
  before the measurement of record (which is how the probe-halt test's empty listing was found).
