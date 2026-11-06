# red-AS3.txt.md

One fenced block: the red-under-hand-change evidence of shard G3 (BASE-COV-API-AS3).

`````text
Red under a hand change: every new test of BASE-COV-API-AS3 not shown red by the -3 session (93 tests).
Tree: a local, never-pushed commit c8a3d979 on top of code tip 2e742873 (this branch), holding all 92 hand
changes at once (one change serves two strict sweeprun tests); built with the repo toolchain (rustc 1.97.1),
CARGO_TARGET_DIR=/var/tmp/as3/K4, cargo test -p api --lib --locked --no-run; run as
api-<hash> --exact <the 93 full names> (TMPDIR a 1777 tmpfs): "test result: FAILED. 0 passed; 93 failed".
Each test below failed at the assertion its change aimed for (the last panic of its run: the failing file and line, then the message's
first words; panics a test raises on purpose come before it). A change inside a test helper is marked: those three tests pin a helper of their own file and
reach no production code. The -3 session's 13 tests are in its report (runs red3, red4).

mastersrun::tests::a_refetch_that_changed_nothing_and_a_reprimed_step_are_each_named
  hand change: crates/api/src/mastersrun.rs: «"the master was refetched and had not changed" }» -> «"the master was refetched, unchanged" }»
  red at crates/api/src/mastersrun.rs:2362: 0 bytes: []
mastersrun::tests::a_reload_that_would_lose_a_readable_feed_is_refused_and_logged
  hand change: crates/api/src/mastersrun.rs: «Err(_) => "the universe could NOT be re-parsed, so the previous one still stands",» -> «Err(_) => "the universe was not re-parsed",»
  red at crates/api/src/mastersrun.rs:2506: []
mastersrun::tests::a_refresh_task_that_does_not_return_is_a_500_that_says_where_to_look
  hand change: crates/api/src/mastersrun.rs: «Err(why) => ( axum::http::StatusCode::INTERNAL_SERVER_ERROR,» -> «Err(why) => ( axum::http::StatusCode::BAD_GATEWAY,»
  red at crates/api/src/mastersrun.rs:2528: assertion `left == right` failed: {"landed":[],"refusal":"the refresh task did not return 
autopilot::tests::a_probe_name_the_host_refuses_keeps_the_hosts_own_words
  hand change: crates/api/src/autopilot.rs: «if why.kind() == std::io::ErrorKind::AlreadyExists {» -> «if why.kind() != std::io::ErrorKind::NotFound {»
  red at crates/api/src/autopilot.rs:7517: not the AlreadyExists sentence: the store refused a write probe at /dev/shm/as3-tmp/brutex
server::optiongreeks::cover_tests::a_contract_date_of_the_wrong_widths_is_not_an_option_segment
  hand change: crates/api/src/optiongreeks.rs: «if y.len() != 4 || m.len() != 2 || d.len() != 2 {» -> «if y.len() != 4 || d.len() != 2 {»
  red at crates/api/src/optiongreeks_cover_tests.rs:168: 2026-8-27-2500000-CE
server::optiongreeks::cover_tests::exchange_segment_and_symbol_are_each_required_by_name
  hand change: crates/api/src/optiongreeks.rs: «"exchange, segment and symbol (the option's own, as the store files it) are all required"» -> «"exchange, segment and the symbol (the option's own, as the store files it) are all requir»
  red at crates/api/src/optiongreeks_cover_tests.rs:191: feed=dhan&exchange=&segment=FNO&symbol=NIFTY: exchange, segment and the symbol (the option
server::optiongreeks::cover_tests::the_plain_answer_is_the_cancellable_one_never_cancelled
  hand change: crates/api/src/optiongreeks.rs: «self.answer_with(store_root, rate, &crate::bars::Cancel::never())» -> «let _ = rate; self.answer_with(store_root, &RateTable::Unset, &crate::bars::Cancel::never(»
  red at crates/api/src/optiongreeks_cover_tests.rs:224: assertion `left == right` failed: [1787197500]
server::optiongreeks::cover_tests::an_option_month_that_is_not_a_bar_file_is_a_named_fault_and_a_206
  hand change: crates/api/src/optiongreeks.rs: «} else { "option_unreadable" };» -> «} else { "option_unopened" };»
  red at crates/api/src/optiongreeks_cover_tests.rs:267: assertion `left == right` failed: 0: {"asof":"exact","basis":"calendar365","carry":0,"carr
server::optiongreeks::cover_tests::an_option_month_a_writer_holds_is_being_written_never_unreadable
  hand change: crates/api/src/optiongreeks.rs: «return (status, json_headers(), body);» -> «return (axum::http::StatusCode::PARTIAL_CONTENT, json_headers(), body.replace(status.as_st»
  red at crates/api/src/optiongreeks_cover_tests.rs:303: assertion `left == right` failed: {"being_written":true,"error":"being written, ask again:
server::optiongreeks::cover_tests::a_record_that_will_not_read_is_a_fault_on_its_own_cell
  hand change: crates/api/src/optiongreeks.rs: «Spot::Unreadable => return Cell::refused("option_unreadable", None),» -> «Spot::Unreadable => return Cell::refused("option_unread", None),»
  red at crates/api/src/optiongreeks_cover_tests.rs:374: assertion `left == right` failed: 0: {"asof":"exact","basis":"calendar365","carry":0,"carr
server::optiongreeks::cover_tests::a_stamp_the_rate_calendar_does_not_hold_is_a_tenor_refusal
  hand change: crates/api/src/optiongreeks.rs: «is not a trade day the rate calendar holds» -> «is not a trade day the rate table holds»
  red at crates/api/src/optiongreeks_cover_tests.rs:415: 0: the stamp 630928740000000 is not a trade day the rate table holds
server::optiongreeks::cover_tests::every_expiry_the_instrument_model_admits_is_settled
  hand change: crates/api/src/optiongreeks.rs: «pull::expiry_alias::settle(symbol, stated, None)» -> «pull::expiry_alias::settle(symbol, stated, stated.succ().ok())»
  red at crates/api/src/optiongreeks_cover_tests.rs:442: NIFTY 1990-01-01
server::optiongreeks::cover_tests::a_greeks_read_the_store_pool_cannot_admit_is_a_429
  hand change: crates/api/src/optiongreeks.rs: «not_admitted("a greeks read", &why)» -> «not_admitted("a greek read", &why)»
  red at crates/api/src/optiongreeks_cover_tests.rs:496: {"error":"a greek read not admitted (Saturated): at most 4 run at once, off the async work
server::optiongreeks::cover_tests::a_rate_table_named_by_another_path_is_read_from_it
  hand change: crates/api/src/optiongreeks.rs: «.map_or(0, |held| held.loads).saturating_add(1);» -> «.map_or(0, |held| held.loads).saturating_add(0);»
  red at crates/api/src/optiongreeks_cover_tests.rs:537: assertion `left == right` failed
indexstopqualificationjson::tests::a_refusal_names_a_value_by_its_own_text
  hand change: crates/api/src/indexstopqualificationjson.rs: «fn display(value: impl std::fmt::Display) -> String { value.to_string() }» -> «fn display(value: impl std::fmt::Display) -> String { format!("{value} ") }»
  red at crates/api/src/indexstopqualificationjson.rs:496: assertion `left == right` failed
server::census_rollup::cover_tests::a_span_the_calendar_cannot_count_is_none_never_zero
  hand change: crates/api/src/census_rollup.rs: «if first > last {» -> «if first > last.saturating_add(1) {»
  red at crates/api/src/census_rollup_cover_tests.rs:367: assertion `left == right` failed: an inverted span
server::census_rollup::cover_tests::an_answer_carries_the_cell_bound_and_is_refused_one_past_it
  hand change: crates/api/src/census_rollup.rs: «if cells.len() > CELLS_MOST {» -> «if cells.len() >= CELLS_MOST {»
  red at crates/api/src/census_rollup_cover_tests.rs:311: assertion `left == right` failed: "the selection holds 4096 (month, rung) cells; one answe
server::census_rollup::cover_tests::every_exchange_word_is_read_resolved_or_refused_by_name
  hand change: crates/api/src/census_rollup.rs: «axum::http::StatusCode::CONFLICT» -> «axum::http::StatusCode::BAD_REQUEST»
  red at crates/api/src/census_rollup_cover_tests.rs:241: assertion `left == right` failed: &underlying=NIFTY: {"error":"exchange_ambiguous","why":"
server::dayflags::cover_tests::a_contract_the_store_cannot_name_is_refused_and_a_shaped_tail_must_parse
  hand change: crates/api/src/dayflags.rs: «format!("contract={contract_text:?} is not a contract segment this store can name")» -> «format!("contract={contract_text:?} is not a contract segment this store can read")»
  red at crates/api/src/dayflags_cover_tests.rs:190: assertion `left == right` failed
server::dayflags::cover_tests::a_day_bar_month_that_will_not_read_is_said_on_each_of_its_days
  hand change: crates/api/src/dayflags.rs: «"it holds {n} records, more than the {MAX_DAY_BARS} a month of days can"» -> «"it holds {n} records, more than the {MAX_DAY_BARS} a month of days holds"»
  red at crates/api/src/dayflags_cover_tests.rs:1234: assertion `left == right` failed
server::dayflags::cover_tests::a_folder_or_a_file_where_a_source_goes_is_named_never_read_as_empty
  hand change: crates/api/src/dayflags.rs: «Err(e) => return Err(PullUnread::Failed(format!("{}: {e}", path.display()))),» -> «Err(e) => return Err(PullUnread::Failed(format!("{} {e}", path.display()))),»
  red at crates/api/src/dayflags_cover_tests.rs:1143: /dev/shm/as3-tmp/brutex-12367-dayflags-cover-folders-zerodha/audit/pull.journal Not a dire
server::dayflags::cover_tests::a_journal_past_one_page_is_read_to_its_last_page
  hand change: crates/api/src/dayflags.rs: «body.get("pages")» -> «body.get("page")»
  red at crates/api/src/dayflags_cover_tests.rs:441: assertion `left == right` failed: 2024-03-11: {
server::dayflags::cover_tests::a_journal_replaced_or_cut_back_under_its_tail_is_walked_again
  hand change: crates/api/src/dayflags.rs: «Via::Walk { ref why } => json!({ "kind": "walk", "why": why }),» -> «Via::Walk { ref why } => json!({ "kind": "walked", "why": why }),»
  red at crates/api/src/dayflags_cover_tests.rs:925: assertion `left == right` failed: {
server::dayflags::cover_tests::a_ledger_is_read_through_its_tail_whatever_is_done_to_it
  hand change: crates/api/src/dayflags.rs: «if tail.identity != identity || len < tail.upto {» -> «if tail.identity != identity {»
  red at crates/api/src/dayflags_cover_tests.rs:595: assertion `left == right` failed
server::dayflags::cover_tests::a_ledger_line_this_reader_cannot_name_is_refused_with_its_reason
  hand change: crates/api/src/dayflags.rs: «"not HH:MM:SS or -"» -> «"not HH:MM:SS or a dash"»
  red at crates/api/src/dayflags_cover_tests.rs:646: assertion `left == right` failed
server::dayflags::cover_tests::a_month_the_store_cannot_name_or_open_is_refused_and_starts_nothing
  hand change: crates/api/src/dayflags.rs: «Err(why) => return Opened::Refused(format!("{} {month:?}: {why}", inst.key())),» -> «Err(why) => return Opened::Refused(format!("{}: {month:?}: {why}", inst.key())),»
  red at crates/api/src/dayflags_cover_tests.rs:1308: NSE-INDEX-lower: YearMonth { year: 2024, month: 3 }: path segment symbol holds byte 0x6c i
server::dayflags::cover_tests::a_pull_record_is_kept_only_where_it_names_a_refusal
  hand change: crates/api/src/dayflags.rs: «"a record kind this reader does not know"» -> «"a record kind this reader cannot name"»
  red at crates/api/src/dayflags_cover_tests.rs:786: assertion `left == right` failed
server::dayflags::cover_tests::a_record_that_cannot_hold_a_note_keeps_what_it_holds
  hand change: crates/api/src/dayflags.rs: «record.as_object_mut() else { continue;» -> «record.as_object_mut() else { return said;»
  red at crates/api/src/dayflags_cover_tests.rs:300: a list beside
server::dayflags::cover_tests::a_walk_stops_where_the_file_stops
  hand change: crates/api/src/dayflags.rs: «let kept = got - got % stride;» -> «let kept = got;»
  red at crates/api/src/dayflags_cover_tests.rs:863: assertion `left == right` failed
server::dayflags::cover_tests::each_odd_day_is_said_for_what_it_is
  hand change: crates/api/src/dayflags.rs: «lead.saturating_mul(2) >= traded» -> «lead.saturating_mul(2) > traded»
  red at crates/api/src/dayflags_cover_tests.rs:1421: assertion `left == right` failed
server::dayflags::cover_tests::each_phrase_a_days_sources_owe_is_said
  hand change: crates/api/src/dayflags.rs: «None => format!("{earlier} refusal(s) logged by an earlier run are history"),» -> «None => format!("{earlier} refusal(s) logged by an earlier run are past"),»
  red at crates/api/src/dayflags_cover_tests.rs:1535: assertion `left == right` failed
server::dayflags::cover_tests::each_refusal_is_placed_by_its_time_counted_once_and_named_to_the_bound
  hand change: crates/api/src/dayflags.rs: «if out.superseded.len() < MAX_REFUSALS_NAMED {» -> «if out.superseded.len() <= MAX_REFUSALS_NAMED {»
  red at crates/api/src/dayflags_cover_tests.rs:526: assertion `left == right` failed
server::dayflags::cover_tests::the_envelope_names_every_source_that_would_not_read
  hand change: crates/api/src/dayflags.rs: «Some(Ok(read)) if read.read_now > FIRST_WALK_RECORDS => vec![format!(» -> «Some(Ok(read)) if read.read_now >= FIRST_WALK_RECORDS => vec![format!(»
  red at crates/api/src/dayflags_cover_tests.rs:1803: the threshold itself is not said
server::dayflags::cover_tests::the_lineage_ledger_is_read_again_when_it_changes_and_a_refused_one_is_said
  hand change: crates/api/src/dayflags.rs: «.map_err(|why| format!("cannot read {}: {why}", path.display()))?;» -> «.map_err(|why| format!("could not read {}: {why}", path.display()))?;»
  red at crates/api/src/dayflags_cover_tests.rs:372: could not read /dev/shm/as3-tmp/brutex-12367-dayflags-cover-lineage-folder/imports/gdfl.li
server::dayflags::cover_tests::through_the_index_a_wrongly_named_record_is_a_fault
  hand change: crates/api/src/dayflags.rs: «"the journal's index names this record under another source or kind",» -> «"the journal's index names this record under another source",»
  red at crates/api/src/dayflags_cover_tests.rs:1021: assertion `left == right` failed
server::dayflags::filter_tests::the_words_for_refusals_no_log_holds_are_the_routes_for_one_and_for_many
  hand change: crates/api/src/dayflags.rs: «let missing = self.failures.saturating_sub(self.seen());» -> «let missing = self.failures.wrapping_sub(self.seen());»
  red at crates/api/src/dayflags_filter_tests.rs:912: assertion `left == right` failed: 2 counted, 3 and 0 seen
server::imports::tests::a_journal_that_cannot_be_read_is_named_never_empty
  hand change: crates/api/src/imports.rs: «Ok(Err(why)) => ( axum::http::StatusCode::INTERNAL_SERVER_ERROR,» -> «Ok(Err(why)) => ( axum::http::StatusCode::SERVICE_UNAVAILABLE,»
  red at crates/api/src/imports.rs:2672: assertion `left == right` failed: {"error":"/dev/shm/as3-tmp/brutex-12367-imports-cover-un
server::imports::tests::the_log_alone_moving_moves_the_summary_and_a_logged_stage_settles_a_day
  hand change: crates/api/src/imports.rs: «&& std::sync::Arc::ptr_eq(refusals, &evidence)» -> «&& !std::sync::Arc::ptr_eq(refusals, &evidence)»
  red at crates/api/src/imports.rs:2740: assertion `left == right` failed: {
server::imports::tests::a_close_a_recount_and_what_no_row_reads_are_each_what_they_are
  hand change: crates/api/src/imports.rs: «3_600_000 / pace.p50» -> «360_000 / pace.p50»
  red at crates/api/src/imports.rs:2889: assertion `left == right` failed: {
server::imports::tests::a_run_found_nowhere_names_a_folder_it_cannot_list_and_passes_over_a_file
  hand change: crates/api/src/imports.rs: «format!("logs/{here} ({error})")» -> «format!("logs/{here}: {error}")»
  red at crates/api/src/imports.rs:2990: no lifecycle event in logs/cli, logs/gdfl/indices, logs/gdfl/stocks, logs/gdfl/options, lo
server::imports::tests::a_folder_whose_name_is_not_utf8_is_counted_never_dropped
  hand change: crates/api/src/imports.rs: «if unlisted.unnamed == 1 {» -> «if unlisted.unnamed == 0 {»
  red at crates/api/src/imports.rs:3022: no lifecycle event in logs/cli, logs/gdfl/indices, logs/gdfl/stocks, logs/gdfl/options, lo
ingest::cover_tests::what_no_form_reaches_still_says_what_it_is
  hand change: crates/api/src/ingest.rs: «a member field this build cannot describe» -> «a member this build cannot describe»
  red at crates/api/src/ingest_cover_tests.rs:73: assertion `left == right` failed
ingest::cover_tests::the_rate_box_is_read_or_refused_by_name_and_names_its_control
  hand change: crates/api/src/ingest.rs: «Refusal::UnreadableRate { .. } => Some("rate"),» -> «Refusal::UnreadableRate { .. } => Some("rates"),»
  red at crates/api/src/ingest_cover_tests.rs:127: assertion `left == right` failed: 9.4617
ingest::cover_tests::a_clock_at_the_epoch_is_refused_without_naming_a_control
  hand change: crates/api/src/ingest.rs: «None => event,» -> «None => event.with("field", telemetry::Value::Str("-")),»
  red at crates/api/src/ingest_cover_tests.rs:177: no control is named
ingest::cover_tests::a_date_in_pieces_is_finished_or_missing_never_guessed
  hand change: crates/api/src/ingest.rs: «if y.is_empty() || m.is_empty() || d.is_empty() {» -> «if y.is_empty() && m.is_empty() && d.is_empty() {»
  red at crates/api/src/ingest_cover_tests.rs:219: assertion `left == right` failed: from_m=08&from_d=06
ingest::cover_tests::one_named_expiry_keeps_its_own_series_of_that_day_and_no_other
  hand change: crates/api/src/ingest.rs: «found.expiry.year() == one.year()» -> «found.expiry.year() <= one.year()»
  red at crates/api/src/ingest_cover_tests.rs:311: assertion `left == right` failed
server::ingest_refusals::tests::a_bound_a_short_day_an_unreadable_journal_and_the_route_each_answer_by_name
  hand change: crates/api/src/ingest_refusals.rs: «if symbol.len() > 64» -> «if symbol.len() >= 64»
  red at crates/api/src/ingest_refusals.rs:618: assertion `left == right` failed
server::month_change::tests::the_look_back_stops_at_the_first_month_and_never_looks_forward
  hand change: crates/api/src/month_change.rs: «break;» -> «return Previous::Gap;»
  red at crates/api/src/month_change.rs:732: assertion `left == right` failed
logs::cover_tests::a_line_in_the_index_and_the_ring_is_shown_once_and_ring_rows_newest_first
  hand change: crates/api/src/logs.rs: «rows.sort_by(|left, right| {» -> «rows.sort_by(|right, left| {»
  red at crates/api/src/logs_cover_tests.rs:435: [1, 2, 3]
logs::cover_tests::a_line_the_keys_do_not_meet_bears_on_nothing
  hand change: crates/api/src/logs.rs: «unknown.join(" and ")» -> «unknown.join(" or ")»
  red at crates/api/src/logs_cover_tests.rs:240: assertion `left == right` failed
logs::cover_tests::a_live_copy_read_under_a_target_or_a_run_counts_no_gap
  hand change: crates/api/src/logs.rs: «|| query.run.is_some()» -> «|| false»
  red at crates/api/src/logs_cover_tests.rs:162: assertion `left == right` failed: limit=5&run=7: a gap means nothing here
logs::cover_tests::a_merge_shows_a_line_once_and_stops_at_a_head_with_no_place
  hand change: crates/api/src/logs.rs: «let (Some(at), Some((pointer, record))) = (chosen.head_at(), chosen.head.take()) else { br» -> «let (Some(at), Some((pointer, record))) = (chosen.head_at(), chosen.head.take()) else { co»
  red at crates/api/src/logs_cover_tests.rs:649: assertion `left == right` failed: [("served", "audit", Event, Record { seq: 10, run: 0, at
logs::cover_tests::a_ring_scan_names_what_it_cannot_read_and_stops_at_its_budget
  hand change: crates/api/src/logs.rs: «if left == 0 { out.hit_cap = true;» -> «if left == 0 { out.hit_cap = false;»
  red at crates/api/src/logs_cover_tests.rs:362: no budget is a capped scan
logs::cover_tests::a_search_stopped_by_a_line_that_will_not_read_or_by_no_budget_still_answers
  hand change: crates/api/src/logs.rs: «Some(false) => high = middle, None => break,» -> «Some(false) => high = middle, None => low = high,»
  red at crates/api/src/logs_cover_tests.rs:519: assertion `left == right` failed: the search stopped at the torn line: {
logs::cover_tests::a_span_over_a_folder_where_its_audit_file_should_be_is_refused_by_name
  hand change: crates/api/src/logs.rs: «.map_err(|e| format!("{}: {e}", path.display()))?;» -> «.map_err(|e| e.to_string())?;»
  red at crates/api/src/logs_cover_tests.rs:467: Is a directory (os error 21)
logs::cover_tests::an_audited_refusal_of_a_day_is_the_importers_error_and_nothing_else
  hand change: crates/api/src/logs.rs: «&& record.level == telemetry::Level::Error» -> «&& record.level.at_least(telemetry::Level::Info)»
  red at crates/api/src/logs_cover_tests.rs:833: assertion `left == right` failed: [Record { seq: 1, run: 0, at_unix_millis: 1793938164515,
logs::cover_tests::each_key_probes_its_own_posting
  hand change: crates/api/src/logs.rs: «out.push(("contract", contract.clone())); if gdfl {» -> «out.push(("contract", contract.clone())); if !gdfl {»
  red at crates/api/src/logs_cover_tests.rs:279: assertion `left == right` failed
logs::cover_tests::evidence_held_in_two_directories_merges_oldest_first_and_names_an_attempt_once
  hand change: crates/api/src/logs.rs: «if !out.contains(id) {» -> «if true {»
  red at crates/api/src/logs_cover_tests.rs:788: assertion `left == right` failed
logs::cover_tests::the_importers_ring_is_folded_and_each_line_filed_where_it_belongs
  hand change: crates/api/src/logs.rs: «&& !attempt.importer_runs.contains(&run)» -> «&& run > 0»
  red at crates/api/src/logs_cover_tests.rs:714: assertion `left == right` failed
logs::cover_tests::the_page_form_keeps_each_span_end_and_the_ring_question
  hand change: crates/api/src/logs.rs: «q.push_str("&ring=1");» -> «q.push_str("&ring=0");»
  red at crates/api/src/logs_cover_tests.rs:107: <!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,ini
server::cloud_mutant_tests_b::a_script_with_no_answer_left_refuses_as_a_transport_failure
  hand change: crates/pull/src/chain.rs: «status: None,» -> «status: Some(0),»
  red at crates/api/src/server_cloud_mutant_tests_b.rs:2272: assertion `left == right` failed
sweepevidence::tests::a_saved_attempt_gone_empty_or_damaged_is_answered_as_such
  hand change: crates/api/src/sweepevidence.rs: «is no longer available; no replacement page was served» -> «is no longer held; no replacement page was served»
  red at crates/api/src/sweepevidence.rs:799: {"schema_version":1,"status":"refused","evidence":null,"rows":[],"refusal":"the requested 
sweepevidence::tests::an_attempt_replaced_between_its_two_reads_is_refused_never_mixed
  hand change: crates/api/src/sweepevidence.rs: «no mixed snapshot was exposed" .to_owned(),» -> «no mixed snapshot was served" .to_owned(),»
  red at crates/api/src/sweepevidence.rs:923: {"schema_version":1,"status":"refused","evidence":null,"rows":[],"refusal":"saved evidence
sweeprun::cover_tests::a_command_this_conductor_does_not_run_and_a_strict_knob_are_answered_by_name
  hand change: crates/api/src/sweeprun.rs: «declared Boolean research requires its prepared audited browser dispatch» -> «declared Boolean research requires its prepared browser dispatch»
  red at crates/api/src/sweeprun_cover_tests.rs:434: assertion `left == right` failed
sweeprun::cover_tests::a_finisher_with_no_audit_runs_bare_and_an_empty_finish_closes_failed
  hand change: crates/api/src/sweeprun.rs: «_ => Phase::Failed,» -> «_ => Phase::Refused,»
  red at crates/api/src/sweeprun_cover_tests.rs:368: assertion `left == right` failed
sweeprun::cover_tests::a_press_no_monitor_could_see_or_that_meets_a_running_sweep_is_refused
  hand change: crates/api/src/sweeprun.rs: «no telemetry sink is installed, so the required» -> «no telemetry sink is open, so the required»
  red at crates/api/src/sweeprun_cover_tests.rs:569: run: {"accepted":false,"refusal":"no telemetry sink is open, so the required sweep-attempt
sweeprun::cover_tests::a_rewalk_that_no_longer_ends_on_the_marker_keeps_the_whole_windows_verdict
  hand change: crates/api/src/sweeprun.rs: «status_tail(dir, "cli.lifecycle", None, index.saturating_add(1))» -> «status_tail(dir, "cli.lifecycle", None, index.saturating_add(2))»
  red at crates/api/src/sweeprun_cover_tests.rs:798: assertion `left == right` failed: premise: narrowed
sweeprun::cover_tests::a_store_that_cannot_be_named_reads_no_cli_log_and_refuses_the_strict_command
  hand change: crates/api/src/sweeprun.rs: «let configuration = cli::audited_range_command::StrictConfig::from_env();» -> «let configuration = cli::audited_range_command::StrictConfig::from_values(None, None, None»
  red at crates/api/src/sweeprun_cover_tests.rs:668: refused: strict input configuration is unavailable; missing [BRUTEX_CHECKSUM_RECEIPTS, BRU
sweeprun::cover_tests::a_strict_command_with_its_configuration_is_conducted_by_the_strict_audit
  hand change: crates/api/src/sweeprun.rs: «let configuration = cli::audited_range_command::StrictConfig::from_env();» -> «let configuration = cli::audited_range_command::StrictConfig::from_values(None, None, None»
  red at crates/api/src/sweeprun_cover_tests.rs:730: refused: strict input configuration is unavailable; missing [BRUTEX_CHECKSUM_RECEIPTS, BRU
sweeprun::cover_tests::an_admission_over_a_store_that_is_not_there_is_refused_by_name
  hand change: crates/api/src/sweeprun.rs: «invocation audit could not start: {why}» -> «invocation audit could not begin: {why}»
  red at crates/api/src/sweeprun_cover_tests.rs:393: required durable invocation audit could not begin: invocation audit: the configured store 
sweeprun::cover_tests::the_status_door_answers_each_observation_by_what_it_can_prove
  hand change: crates/api/src/sweeprun.rs: «Self::WalkNotKept => "walk-not-kept",» -> «Self::WalkNotKept => "walk-unkept",»
  red at crates/api/src/sweeprun_cover_tests.rs:307: assertion `left == right` failed
server::ops_safety_tests::a_build_label_is_unstamped_or_a_whole_commit
  hand change: crates/api/src/ops_safety_tests.rs: «label == "unstamped" || (label.len() == 40 && label.bytes().all(|b| b.is_ascii_hexdigit())» -> «label == "unstamped" || (label.len() >= 39 && label.bytes().all(|b| b.is_ascii_hexdigit())»
  red at crates/api/src/ops_safety_tests.rs:313: assertion `left == right` failed: "0123456789abcdef0123456789abcdef0123456"
server::serve_edge_tests::the_print_scan_judges_a_macro_name_as_a_whole_word
  hand change: crates/api/src/serve_edge_tests.rs: «!line .get(..at)» -> «line .get(..at)»
  red at crates/api/src/serve_edge_tests.rs:379: assertion `left == right` failed: println!("x");
server::cloud_mutant_tests_b::the_visible_part_of_a_receipt_starts_at_its_first_content_section
  hand change: crates/api/src/server_cloud_mutant_tests_b.rs: «.and_then(|at| body.get(at..)) .unwrap_or(body)» -> «.and_then(|at| body.get(at + 1..)) .unwrap_or(body)»
  red at crates/api/src/server_cloud_mutant_tests_b.rs:2002: assertion `left == right` failed: <style>x</style><section class="wrap">kept</section>
server::bars_export::tests::every_page_that_cannot_be_written_whole_cuts_the_stream_by_name
  hand change: crates/api/src/bars_export.rs: «"the page at row {} held {} row(s) of a query keeping {}",» -> «"the page at row {} held {} row(s) of a query that keeps {}",»
  red at crates/api/src/bars_export.rs:1971: assertion `left == right` failed
server::bars_export::tests::a_page_read_past_a_full_pool_is_read_and_a_task_that_will_not_join_cuts_the_stream
  hand change: crates/api/src/bars_export.rs: «rows changed while it was exported: {} when it began» -> «rows changed as it was exported: {} when it began»
  red at crates/api/src/bars_export.rs:2035: assertion `left == right` failed
server::bars_export::tests::a_refusal_before_a_byte_is_the_windows_and_a_name_is_always_a_file_name
  hand change: crates/api/src/bars_export.rs: «refused(StatusCode::GONE, "contract_refused", why)» -> «refused(StatusCode::NOT_FOUND, "contract_refused", why)»
  red at crates/api/src/bars_export.rs:2124: assertion `left == right` failed: {"error":"contract_refused: NSE-FNO-ZZFIXTURE-2019-03-28
server::bars_export::tests::a_refused_read_is_named_and_one_past_the_cheap_budget_is_asked_again_as_a_scan
  hand change: crates/api/src/bars_export.rs: «Ok(Err(why)) if !heavy && bars::needs_scan(&why) => heavy = true,» -> «Ok(Err(why)) if heavy && bars::needs_scan(&why) => heavy = true,»
  red at crates/api/src/bars_export.rs:2259: assertion `left == right` failed: {"code":"bad_query","error":"needs_scan_pool: ordering b
server::bars_export::tests::a_full_scan_pool_refuses_an_export_by_name_and_it_is_written_once_the_pool_frees
  hand change: crates/api/src/bars_export.rs: «Err(why) => return Err(Refused::Pool(why, heavy)),» -> «Err(why) => return Err(Refused::Pool(why, !heavy)),»
  red at crates/api/src/bars_export.rs:2298: {"error":"a bar export not admitted (Saturated): at most 4 run at once, off the async work
server::bars_route::tests::a_time_of_day_of_one_field_or_four_is_refused_and_an_open_end_runs_to_the_last_second
  hand change: crates/api/src/bars_route.rs: «"{text:?} is not an IST time of day HH:MM:SS for {key:?}"» -> «"{text:?} is not an IST time of day HH:MM[:SS] for {key:?}"»
  red at crates/api/src/bars_route.rs:2894: assertion `left == right` failed
server::bars_route::tests::a_value_past_a_browsers_number_is_withheld_by_name_and_a_month_dropped_in_part_is_served
  hand change: crates/api/src/bars_route.rs: «for one in withheld {» -> «for one in withheld.into_iter().skip(1) {»
  red at crates/api/src/bars_route.rs:2846: assertion `left == right` failed: [{"t":1704155400,"o":100,"h":110,"l":90,"c":105,"v":1,"o
server::bars_route::tests::a_window_names_damaged_records_withholds_an_inexact_value_and_counts_coverage
  hand change: crates/api/src/bars_route.rs: «"days_covered":{}» -> «"days_held":{}»
  red at crates/api/src/bars_route.rs:2948: {"total":3,"offset":0,"months_read":1,"months_missing":0,"missing_months":[],"months_outsi
server::bars_route::tests::an_abandoned_scan_stops_and_an_answered_one_cancels_nothing_past_a_full_pool
  hand change: crates/api/src/detail.rs: «guard.disarm();» -> «drop(guard);»
  red at crates/api/src/bars_route.rs:3129: assertion failed: !kept.is_cancelled()
server::bars_window_route_tests::a_window_refused_by_a_full_scan_pool_is_asked_again_and_answered_there
  hand change: crates/api/src/bars_route.rs: «"" | "desc" => true,» -> «"" | "desc" | "up" => true,»
  red at crates/api/src/bars_window_route_tests.rs:2527: assertion `left == right` failed: {"admitted":"store_read","at_index":null,"bars":[{"c":18
bars::series::generation_tests::a_journal_that_cannot_be_read_moves_no_index_life
  hand change: crates/api/src/bars_series.rs: «if let Ok(journal) = crate::server::import_journal::snapshot(root)» -> «if let Some(journal) = Some(crate::server::import_journal::snapshot(root).ok()?)»
  red at crates/api/src/bars_series.rs:2067: assertion `left == right` failed
bars::series::cloud_mutant_tests::a_plan_serves_every_month_it_can_and_names_every_one_it_cannot
  hand change: crates/api/src/bars_series.rs: «"slot {slot} is not a month of this window"» -> «"slot {slot} is not a month of this request"»
  red at crates/api/src/bars_series.rs:3061: assertion `left == right` failed
bars::series::tod_index_tests::a_summary_whose_days_do_not_add_up_is_refused_by_name
  hand change: crates/api/src/bars_series.rs: «"its summary counts {} bars on day {day} and {} held {}"» -> «"its summary names {} bars on day {day} and {} held {}"»
  red at crates/api/src/bars_series.rs:2550: assertion `left == right` failed: a day counting one bar fewer than it holds
bars::series::tod_index_tests::an_instant_before_the_month_and_a_unit_past_the_span_answer_nothing
  hand change: crates/api/src/bars_series.rs: «(held.first(), held.last())» -> «(held.first(), held.iter().max())»
  red at crates/api/src/bars_series.rs:2655: assertion `left == right` failed: the span is units 3 to 100: two words
server::feeds_route::tests::the_held_span_answers_at_every_edge_of_its_census
  hand change: crates/api/src/feeds_route.rs: «Some((_, walks)) => (PartSpan::walk(part), walks.saturating_add(1)),» -> «Some((_, walks)) => (PartSpan::walk(part), walks),»
  red at crates/api/src/feeds_route.rs:3092: assertion `left == right` failed: walked again
server::feeds_route::tests::the_import_reach_reads_every_shape_its_journal_lines_take
  hand change: crates/api/src/feeds_route.rs: «return none(format!("{segment:?} is not a segment"));» -> «return none(format!("{segment:?} names no segment"));»
  red at crates/api/src/feeds_route.rs:3321: assertion `left == right` failed
server::feeds_route::tests::the_archive_evidence_and_the_store_rows_name_every_shape_they_meet
  hand change: crates/api/src/feeds_route.rs: «names {} as its last day, after today {} (IST)» -> «names {} as its last day, past today {} (IST)»
  red at crates/api/src/feeds_route.rs:3409: assertion `left == right` failed
server::feeds_route::tests::the_window_under_degraded_settings_is_served_only_from_a_generation
  hand change: crates/api/src/feeds_route.rs: «(from.origin != "generation" || to.origin != "generation")» -> «(from.origin != "generation" && to.origin != "generation")»
  red at crates/api/src/feeds_route.rs:3543: {"degraded":"1 generation number(s) missing (2)","from":"2019-01-01","label_outside":"outs
server::feeds_route::tests::the_descriptor_facts_behind_five_branches_hold_feed_by_feed
  hand change: crates/pull/src/vendor.rs: «is_fold: false,» -> «is_fold: true,»
  red at crates/api/src/feeds_route.rs:3605: Zerodha measures its day as a fold
`````
