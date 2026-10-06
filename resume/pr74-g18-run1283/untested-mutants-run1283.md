crates/api/src/assets.rs:917:9: replace Assets::shell -> Response with Default::default()
crates/api/src/bars.rs:1257:14: replace < with > in page_of
crates/api/src/ingest.rs:1263:40: replace || with && in parse_day
crates/api/src/recovery.rs:1176:24: replace == with != in reassessed
crates/api/src/server.rs:11441:5: replace detached_pull -> (axum::http::StatusCode, String) with (Default::default(), String::new())
crates/api/src/server.rs:19185:5: replace stopped_over -> u8 with 1
crates/cli/src/lib.rs:1278:30: replace || with && in auto_stored_exit
crates/cli/src/lib.rs:10832:36: replace <= with > in stop_rungs_in_points
crates/cli/src/lib.rs:19537:5: replace record_frontier -> Result<(String, u64), String> with Ok((String::new(), 1))
crates/cli/src/execution_v3.rs:2393:9: replace ExecutionV3Ledger::append_parameter_suffix -> Result<(), ExecutionV3Refusal> with Ok(())
crates/cli/src/and_checkpoint.rs:601:28: replace += with *= in Replay<'_>::load
crates/cli/src/fold_audit.rs:349:36: replace match guard after.ts_micros <= before.ts_micros with true in require_strictly_increasing
crates/cli/src/ordered.rs:85:9: replace Turns::lanes -> MutexGuard<'_, Vec<Lane>> with MutexGuard::from(vec![])
crates/cli/src/pool_oos.rs:397:29: replace * with / in bits_of
crates/cli/src/search_checkpoint.rs:94:9: replace Snapshot::open_through -> Result<Option<Self>, String> with Ok(Some(Default::default()))
crates/cli/src/stored_data_completeness.rs:268:9: replace StoredDataCompletenessReceiptV1::validate -> Result<(), StoredDataCompletenessRefusal> with Ok(())
crates/cli/src/boolean_candidate_persistence.rs:169:32: replace == with != in committed
crates/indicators/src/column.rs:990:9: replace Column::reproject_with -> Option<(Self, u64)> with None
crates/indicators/src/vwap.rs:589:24: replace * with / in wide_mul
crates/lake/src/schema.rs:291:29: replace || with && in misread_timestamp_logical
crates/pull/src/fold.rs:1069:5: replace ist_day_of -> Result<(i64, i64), FoldError> with Ok((0, 1))
crates/pull/src/masters.rs:1247:17: replace && with || in land_validated
crates/pull/src/totp.rs:307:13: replace >= with < in decode_alphabet
crates/runner/src/bootstrap.rs:1257:14: replace == with != in romano_wolf_receipt
crates/runner/src/grid.rs:2370:5: replace evaluate_timed_with_exact_ladders -> Grid with Default::default()
crates/runner/src/outcome.rs:1193:13: replace && with || in prefix_median_steps_over
crates/runner/src/significance.rs:434:5: replace regularized_incomplete_beta -> f64 with 0.0
crates/runner/src/trade.rs:590:9: replace SliceFacts::step_micros -> i64 with 0
crates/store/src/file.rs:2312:34: replace != with == in BarFile::may_append
crates/store/src/layout.rs:646:5: replace degenerate_field -> Option<&'static str> with Some("")
crates/store/src/time_index.rs:618:16: replace += with *= in extend
crates/api/src/autopilot.rs:2472:33: replace || with && in SeriesCache::get
crates/api/src/credential_law.rs:110:9: replace <impl core::fmt::Display for Unreadable>::fmt -> core::fmt::Result with Ok(Default::default())
crates/api/src/mastersrun.rs:459:5: replace credentialed_zerodha -> Result<pull::http::HttpSource, String> with Ok(Default::default())
crates/api/src/server.rs:808:5: replace field_value -> Option<&'a str> with None
crates/api/src/server.rs:16657:5: replace form_key_verdict -> Option<FormKeys<'_>> with None
crates/api/src/sweeprun.rs:2024:51: replace / with % in claim_execution
crates/cli/src/lib.rs:3917:9: replace StoredMonthInputs::withholding -> Withholding<'_> with Default::default()
crates/cli/src/lib.rs:16055:5: replace elite_descend_with_attempt -> String with "xyzzy".into()
crates/cli/src/cancel.rs:58:5: replace requested -> bool with true
crates/cli/src/population_v6.rs:1962:20: delete ! in PopulationV6Ledger::open
crates/cli/src/candidate_universe.rs:1524:9: replace CandidateSearchColumnBuilderV1<'_>::build -> Result<Column, CandidateUniverseRefusal> with Ok(Default::default())
crates/cli/src/institutional_evidence.rs:2125:9: replace BootstrapFraction::ppm -> u64 with 0
crates/cli/src/pool.rs:245:9: replace Pooled::key -> (bool, i64, i128, i128) with (false, 1, 1, -1)
crates/cli/src/population_statistics_v2.rs:2266:9: replace PopulationStatisticsV2Ledger::scan -> Result<(), PopulationStatisticsV2Refusal> with Ok(())
crates/cli/src/step3_comparison.rs:669:5: replace read_execution -> StageProbe<Vec<ExecutionFact>> with StageProbe::from_iter([vec![Default::default()]])
crates/cli/src/population_base_evidence_v2.rs:605:60: replace > with < in BaseEvidenceRecordV2::admission_values
crates/engine/src/lib.rs:185:9: replace primitives::screen_blocks -> (u64, u64) with (1, 1)
crates/indicators/src/pattern.rs:655:13: replace && with || in Patterns::bits
crates/lake/src/footer.rs:236:5: replace zigzag -> i32 with 1
crates/pull/src/csv.rs:599:5: replace note_decoded with ()
crates/pull/src/http.rs:2768:13: replace && with || in HttpSource::refused_status
crates/pull/src/rolling.rs:1017:5: replace micros_of -> Result<i64, RollingError> with Ok(0)
crates/runner/src/audit.rs:235:73: replace * with + in largest_overnight_move
crates/runner/src/excursion.rs:518:9: replace Crossings::empty -> Self with Default::default()
crates/runner/src/identity.rs:557:5: replace data_digest_with_daily_reference -> Result<[u8; OUT_LEN], DailyBindingRefusal> with Ok([0; OUT_LEN])
crates/runner/src/outcome.rs:2142:39: replace - with / in OverlapWindow::sum_squares
crates/runner/src/significance.rs:470:52: replace / with % in beta_continued_fraction
crates/runner/src/expression_execution.rs:460:9: replace ResolvedGridViewV1<'_>::materialize_expression_coordinate -> Result<Vec<TradeRow>, String> with Ok(vec![Default::default()])
crates/store/src/file.rs:3535:77: replace >= with < in Admission::admit_header
crates/store/src/time_index.rs:161:9: replace Geometry::exact -> bool with false
crates/vocab/src/expression_search.rs:246:33: replace match guard below_depth >= 1 with true in Cursor::place
crates/api/src/autopilot.rs:3046:5: replace probe_store_halts -> String with String::new()
crates/api/src/credential_law.rs:430:9: replace Watch::halt -> Reread with Default::default()
crates/api/src/operation_audit.rs:173:5: replace request_audited -> axum::response::Response with Default::default()
crates/api/src/server.rs:1398:58: replace != with == in bars_by_symbol
crates/api/src/server.rs:16753:8: delete ! in one_value_per_form_field
crates/api/src/sweeprun.rs:2603:5: replace tail_fault -> Option<String> with None
crates/cli/src/lib.rs:6073:5: replace grid_rungs -> usize with 1
crates/cli/src/lib.rs:16458:19: replace < with <= in elite_descend_in_points_inner
crates/cli/src/columns.rs:61:5: replace left -> Col with Default::default()
crates/cli/src/admission_store.rs:1012:21: replace != with == in AdmissionAuthorityLedger::complete_orphan_decisions
crates/cli/src/candidate_universe.rs:3883:9: replace CandidateUniverseLedgerV1::reverify_committed_locked -> Result<CandidateUniverseReopenAuditV1, CandidateUniverseRefusal> with Ok(Default::default())
crates/cli/src/institutional_evidence.rs:2167:23: replace || with && in bootstrap_fraction
crates/cli/src/pool.rs:261:5: replace ranked -> i128 with 1
crates/cli/src/population_statistics_v2.rs:4617:41: replace == with != in PopulationStatisticsV2Ledger::append_locked
crates/cli/src/step3_comparison.rs:858:5: replace read_selections -> StageProbe<Vec<SelectionFact>> with StageProbe::from_iter([vec![Default::default()]])
crates/cli/src/boolean_candidate_v1.rs:615:5: replace compute -> Result<Computed, String> with Ok(Default::default())
crates/engine/src/lib.rs:1806:9: replace Ladder::try_next_level_observing -> Result<(Frontier, Option<Halt>, usize, u64), TryReserveError> with Ok((Default::default(), Some(Default::default()), 1, 1))
crates/indicators/src/pattern.rs:672:26: replace < with > in Patterns::bits
crates/lake/src/footer.rs:267:5: replace element -> Result<u8, LakeError> with Ok(1)
crates/pull/src/csv.rs:770:5: replace fields_of -> Result<[&str; MAX_FIELDS], usize> with Ok(["xyzzy"; MAX_FIELDS])
crates/pull/src/http.rs:3321:9: replace HttpSource::weigh_body -> Option<crate::refusal::Disposition> with None
crates/pull/src/scrub.rs:328:62: replace + with - in Tally::seen
crates/runner/src/audit.rs:269:5: replace overnight_line -> String with "xyzzy".into()
crates/runner/src/excursion.rs:1506:24: replace || with && in ppm_ceil_of
crates/runner/src/outcome.rs:500:52: replace != with == in Forward::was_refused
crates/runner/src/outcome.rs:2529:23: replace > with < in newey_west_t
crates/runner/src/significance.rs:470:72: replace * with / in beta_continued_fraction
crates/runner/src/expression_validation.rs:168:16: delete ! in FixedTrainingFoldPlanV1::bind_with
crates/store/src/file.rs:3689:5: replace time_index_path -> Option<PathBuf> with None
crates/store/src/time_index.rs:227:9: replace Geometry::check -> Result<(), Why> with Ok(())
crates/vocab/src/expression_search.rs:314:9: replace Cursor::decode -> Result<Self, Refusal> with Ok(Default::default())
crates/api/src/autopilot.rs:3278:5: replace round -> Pass with Default::default()
crates/api/src/credential_law.rs:449:20: replace == with != in note
crates/api/src/pullrun.rs:62:62: replace * with +
crates/api/src/server.rs:2072:5: replace bars_array -> (String, Vec<String>) with (String::new(), vec!["xyzzy".into()])
crates/api/src/server.rs:16804:21: delete ! in repeated_query_key
crates/api/src/sweeprun.rs:2619:5: replace tail_fault_unless_answered -> Option<String> with Some("xyzzy".into())
crates/cli/src/lib.rs:6130:5: replace walk_forward_rungs -> runner::validate::FoldRungs<'static> with Default::default()
crates/cli/src/lib.rs:16632:19: replace < with == in screen_range_in_points
crates/cli/src/columns.rs:85:5: replace chars -> usize with 1
crates/cli/src/admission_store.rs:1422:5: replace reconcile_all -> Result<(), AdmissionStoreRefusal> with Ok(())
crates/cli/src/candidate_universe.rs:3974:5: replace append_produced_candidate_universe_v1 -> Result<CandidateUniverseProductionCommitV1, CandidateUniverseRefusal> with Ok(Default::default())
crates/cli/src/institutional_statistics.rs:782:9: replace InstitutionalStatisticsAuthorityV1::fwer_p_value_ppm -> u64 with 0
crates/cli/src/pool.rs:463:5: replace head_under -> Result<(String, Vec<String>, String), String> with Ok((String::new(), vec![], String::new()))
crates/cli/src/population_statistics_v2.rs:5530:5: replace ensure_header -> Result<(), PopulationStatisticsV2Refusal> with Ok(())
crates/cli/src/step3_comparison.rs:942:5: replace read_global_replay -> StageProbe<crate::global_replay_v2::PreparedGlobalReplayV2> with StageProbe::new(Default::default())
crates/cli/src/boolean_candidate_v1.rs:895:9: replace Shared<'a>::digests -> Result<ExecutionDigestsV1, String> with Ok(Default::default())
crates/engine/src/lib.rs:2172:5: replace joined_frontier -> Frontier with Default::default()
crates/indicators/src/pattern.rs:772:13: replace && with || in Patterns::bits
crates/lake/src/footer.rs:287:5: replace list -> Result<Frame, LakeError> with Ok(Default::default())
crates/pull/src/csv.rs:799:15: replace == with != in open_interest_of
crates/pull/src/http.rs:3371:5: replace refused_in_body -> String with String::new()
crates/pull/src/scrub.rs:328:36: replace + with * in Tally::seen
crates/runner/src/audit.rs:699:5: replace grid_columns with ()
crates/runner/src/excursion.rs:1512:45: replace == with != in ppm_ceil_of
crates/runner/src/outcome.rs:536:9: replace Forward::exit_at -> Option<usize> with Some(0)
crates/runner/src/report.rs:588:5: replace render_findings_at -> String with "xyzzy".into()
crates/runner/src/significance.rs:470:83: replace + with - in beta_continued_fraction
crates/runner/src/expression_validation.rs:239:47: replace > with < in admit_mapping_bytes
crates/store/src/file.rs:3730:5: replace put_entries -> Result<(), StoreError> with Ok(())
crates/store/src/time_index.rs:275:9: replace Entry::rank -> u64 with 0
crates/vocab/src/table.rs:1607:16: replace > with <
crates/api/src/autopilot.rs:3708:40: replace && with || in outcome_of
crates/api/src/detail.rs:76:9: replace Permit::try_take_from -> Option<Self> with None
crates/api/src/pullrun.rs:62:58: replace * with /
crates/api/src/server.rs:2072:5: replace bars_array -> (String, Vec<String>) with ("xyzzy".into(), vec!["xyzzy".into()])
crates/api/src/server.rs:16850:5: replace route_table -> axum::Router<Loaded> with Router::new(Default::default())
crates/api/src/sweeprun.rs:2626:31: replace && with || in tail_fault_unless_answered
crates/cli/src/lib.rs:6433:5: replace sample_warning -> String with String::new()
crates/cli/src/lib.rs:16716:5: replace descent_table with ()
crates/cli/src/columns.rs:93:44: replace && with || in margin
crates/cli/src/and_checkpoint.rs:38:27: replace * with +
crates/cli/src/candidate_universe.rs:3991:17: replace != with == in append_prepared_and_reverify
crates/cli/src/institutional_statistics.rs:789:9: replace InstitutionalStatisticsAuthorityV1::romano_wolf_p_value_ppm -> u64 with 1
crates/cli/src/pool.rs:463:5: replace head_under -> Result<(String, Vec<String>, String), String> with Ok((String::new(), vec![String::new()], "xyzzy".into()))
crates/cli/src/population_statistics_v3.rs:2195:9: replace PopulationStatisticsV3Ledger::scan -> Result<(), PopulationStatisticsV3Refusal> with Ok(())
crates/cli/src/step3_comparison.rs:1492:5: replace absent_or_uninspectable -> Option<StageProbe<T>> with Some(StageProbe::new())
crates/cli/src/global_replay_v4_lifecycle.rs:15:5: replace plan_identity -> Result<[u8; 32], String> with Ok([0; 32])
crates/engine/src/lib.rs:2380:5: replace drain -> Result<(), Breach> with Ok(())
crates/indicators/src/pattern.rs:769:13: replace && with || in Patterns::bits
crates/lake/src/footer.rs:296:31: replace & with ^ in list
crates/pull/src/csv.rs:808:41: replace < with <= in open_interest_of
crates/pull/src/http.rs:3569:5: replace one_stamp -> Result<i64, FetchError> with Ok(0)
crates/pull/src/scrub.rs:335:9: replace Tally::disagreed -> u64 with 0
crates/runner/src/audit.rs:969:20: replace == with != in strategy_report
crates/runner/src/exit_grid_policy.rs:86:5: replace printed_ohlcv_cost_model_id_v2 -> [u8; 32] with [0; 32]
crates/runner/src/outcome.rs:680:9: replace WindowExtremes::over -> Option<(i64, i64)> with Some((0, 0))
crates/runner/src/report.rs:638:13: replace && with || in render_findings_at
crates/runner/src/significance.rs:470:77: replace + with * in beta_continued_fraction
crates/runner/src/expression_validation.rs:251:5: replace index_folds -> Result<(Vec<usize>, Vec<u64>), String> with Ok((vec![], vec![0]))
crates/store/src/file.rs:3752:22: replace match guard gone.kind() == ErrorKind::NotFound with true in remove_index
crates/store/src/time_index.rs:275:36: replace & with ^ in Entry::rank
crates/vocab/src/table.rs:1610:13: replace += with *=
crates/api/src/autopilot.rs:4115:5: replace stop with ()
crates/api/src/detail.rs:76:9: replace Permit::try_take_from -> Option<Self> with Some(Default::default())
crates/api/src/pullrun.rs:70:18: replace + with -
crates/api/src/server.rs:2111:38: replace - with +
crates/api/src/server.rs:16850:5: replace route_table -> axum::Router<Loaded> with Router::from(Default::default())
crates/api/src/sweeprun.rs:2626:34: delete ! in tail_fault_unless_answered
crates/cli/src/lib.rs:6433:5: replace sample_warning -> String with "xyzzy".into()
crates/cli/src/lib.rs:16793:5: replace descent_line -> String with String::new()
crates/cli/src/columns.rs:93:37: delete ! in margin
crates/cli/src/and_checkpoint.rs:38:27: replace * with /
crates/cli/src/candidate_universe.rs:3991:51: replace != with == in append_prepared_and_reverify
crates/cli/src/institutional_statistics.rs:895:9: replace FileIdentityV1::of -> Self with Default::default()
crates/cli/src/pool.rs:463:5: replace head_under -> Result<(String, Vec<String>, String), String> with Ok((String::new(), vec!["xyzzy".into()], String::new()))
crates/cli/src/population_statistics_v3.rs:2387:9: replace PopulationStatisticsV3Ledger::append_locked -> Result<PopulationStatisticsV3Commit, PopulationStatisticsV3Refusal> with Ok(Default::default())
crates/cli/src/step3_comparison.rs:1492:5: replace absent_or_uninspectable -> Option<StageProbe<T>> with Some(StageProbe::from_iter([Default::default()]))
crates/cli/src/global_replay_v4_lifecycle.rs:15:5: replace plan_identity -> Result<[u8; 32], String> with Ok([1; 32])
crates/engine/src/lib.rs:2402:17: replace >= with < in drain
crates/indicators/src/pattern.rs:769:26: replace > with == in Patterns::bits
crates/lake/src/footer.rs:297:34: replace >> with << in list
crates/pull/src/csv.rs:817:5: replace check_header -> Result<(), CsvError> with Ok(())
crates/pull/src/http.rs:3569:5: replace one_stamp -> Result<i64, FetchError> with Ok(1)
crates/pull/src/scrub.rs:335:9: replace Tally::disagreed -> u64 with 1
crates/runner/src/audit.rs:1027:5: replace mean_row -> (&'static str, String, &'static str) with ("", String::new(), "")
crates/runner/src/exit_grid_policy.rs:86:5: replace printed_ohlcv_cost_model_id_v2 -> [u8; 32] with [1; 32]
crates/runner/src/outcome.rs:680:9: replace WindowExtremes::over -> Option<(i64, i64)> with Some((0, 1))
crates/runner/src/report.rs:638:31: replace >= with < in render_findings_at
crates/runner/src/significance.rs:470:89: replace * with + in beta_continued_fraction
crates/runner/src/expression_validation.rs:251:5: replace index_folds -> Result<(Vec<usize>, Vec<u64>), String> with Ok((vec![], vec![1]))
crates/store/src/file.rs:3752:22: replace match guard gone.kind() == ErrorKind::NotFound with false in remove_index
crates/store/src/time_index.rs:275:44: replace << with >> in Entry::rank
crates/vocab/src/table.rs:1625:5: replace fnv1a -> u64 with 0
crates/api/src/backtest.rs:1297:5: replace limit_asked -> usize with 1
crates/api/src/detail.rs:96:9: replace <impl Drop for Permit>::drop with ()
crates/api/src/pullrun.rs:70:62: replace + with *
crates/api/src/server.rs:2115:26: replace <= with > in browser_exact
crates/api/src/server.rs:17604:9: replace <impl http_body::Body for DeadlineBody>::poll_frame -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> with Poll::from(None)
crates/api/src/sweeprun.rs:2686:9: replace && with || in newest_sweep_marker
crates/cli/src/lib.rs:7448:5: replace unstamped_audit_refusal -> stored::Refusal with Default::default()
crates/cli/src/lib.rs:16922:5: replace descend_in -> String with "xyzzy".into()
crates/cli/src/columns.rs:103:5: replace widths -> Vec<usize> with vec![1]
crates/cli/src/and_checkpoint.rs:42:71: replace * with +
crates/cli/src/candidate_universe.rs:4166:29: replace < with == in session_close_minute_v1
crates/cli/src/institutional_statistics.rs:1478:5: replace snapshot_file -> Result<FileSnapshotV1, String> with Ok(Default::default())
crates/cli/src/pool.rs:463:5: replace head_under -> Result<(String, Vec<String>, String), String> with Ok(("xyzzy".into(), vec![String::new()], "xyzzy".into()))
crates/cli/src/population_statistics_v3.rs:2969:5: replace append_raw -> Result<(), PopulationStatisticsV3Refusal> with Ok(())
crates/cli/src/step3_comparison.rs:1494:32: replace == with != in absent_or_uninspectable
crates/cli/src/selection_v6_read.rs:61:19: replace == with != in selection_v6_family
crates/engine/src/column.rs:162:66: replace << with >> in Column::support_fingerprinted
crates/indicators/src/pattern.rs:770:26: replace < with <= in Patterns::bits
crates/lake/src/footer.rs:317:48: replace & with | in map
crates/pull/src/csv.rs:833:5: replace decode_rows -> Result<Vec<RawRow>, CsvError> with Ok(vec![Default::default()])
crates/pull/src/http.rs:3630:5: replace stated_offset -> Option<i64> with Some(-1)
crates/pull/src/scrub.rs:335:22: replace + with - in Tally::disagreed
crates/runner/src/audit.rs:1027:5: replace mean_row -> (&'static str, String, &'static str) with ("xyzzy", String::new(), "xyzzy")
crates/runner/src/exit_grid_policy.rs:324:9: replace ExecutionDigestsV1::of_daily_reference -> Result<Self, ExitGridErrorV1> with Ok(Default::default())
crates/runner/src/outcome.rs:680:9: replace WindowExtremes::over -> Option<(i64, i64)> with Some((-1, 0))
crates/runner/src/resample.rs:94:29: replace * with +
crates/runner/src/significance.rs:471:33: replace + with * in beta_continued_fraction
crates/runner/src/expression_validation.rs:251:5: replace index_folds -> Result<(Vec<usize>, Vec<u64>), String> with Ok((vec![1], vec![0]))
crates/store/src/file.rs:3972:5: replace is_interrupted_genesis -> bool with true
crates/store/src/time_index.rs:282:9: replace Entry::occupied -> bool with true
crates/vocab/src/table.rs:1628:14: replace ^= with |= in fnv1a
crates/api/src/backtest.rs:1316:5: replace unparseable_limit -> Option<String> with None
crates/api/src/detail.rs:121:5: replace run -> Result<T, RunError> with Ok(Default::default())
crates/api/src/pullrun.rs:84:35: replace + with -
crates/api/src/server.rs:2128:5: replace inexact_bar -> Option<String> with None
crates/api/src/server.rs:17604:9: replace <impl http_body::Body for DeadlineBody>::poll_frame -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> with Poll::from_iter([Some(Ok(Frame::new()))])
crates/api/src/sweeprun.rs:2690:37: replace && with || in newest_sweep_marker
crates/cli/src/lib.rs:7464:5: replace audit_range_inner -> Result<String, stored::Refusal> with Ok(String::new())
crates/cli/src/lib.rs:17062:5: replace return_over_drawdown_cell -> String with String::new()
crates/cli/src/columns.rs:113:47: replace == with != in widths
crates/cli/src/and_checkpoint.rs:42:71: replace * with /
crates/cli/src/candidate_universe.rs:4166:29: replace < with > in session_close_minute_v1
crates/cli/src/institutional_statistics.rs:1589:5: replace ceiling_ppm -> u64 with 0
crates/cli/src/pool.rs:463:5: replace head_under -> Result<(String, Vec<String>, String), String> with Ok(("xyzzy".into(), vec!["xyzzy".into()], String::new()))
crates/cli/src/pre_admission_data.rs:1240:9: replace PreAdmissionDataLedgerV1::page -> Result<PreAdmissionDataPageV1, PreAdmissionDataRefusal> with Ok(Default::default())
crates/cli/src/step3_orchestrator.rs:511:9: replace BoundedStoredContextV1::candidate_swept_v1 -> Result<u64, Step3OrchestratorRefusal> with Ok(0)
crates/cli/src/selection_v6_read.rs:213:5: replace read_stored_selection_v6 -> Vec<(&'static str, StoredSelectionV6Rung)> with vec![]
crates/engine/src/keep.rs:175:9: replace Streamed::depth -> usize with 0
crates/indicators/src/pattern.rs:771:27: replace < with == in Patterns::bits
crates/lake/src/footer.rs:317:48: replace & with ^ in map
crates/pull/src/csv.rs:841:30: replace == with != in decode_rows
crates/pull/src/http.rs:3647:8: delete ! in stated_offset
crates/pull/src/scrub.rs:335:22: replace + with * in Tally::disagreed
crates/runner/src/audit.rs:1027:5: replace mean_row -> (&'static str, String, &'static str) with ("xyzzy", "xyzzy".into(), "")
crates/runner/src/exit_grid_policy.rs:340:9: replace ExecutionDigestsV1::data_digest -> [u8; 32] with [0; 32]
crates/runner/src/outcome.rs:680:9: replace WindowExtremes::over -> Option<(i64, i64)> with Some((-1, 1))
crates/runner/src/resample.rs:94:29: replace * with /
crates/runner/src/significance.rs:471:39: replace * with + in beta_continued_fraction
crates/runner/src/expression_validation.rs:251:5: replace index_folds -> Result<(Vec<usize>, Vec<u64>), String> with Ok((vec![1], vec![1]))
crates/store/src/file.rs:3972:5: replace is_interrupted_genesis -> bool with false
crates/store/src/time_index.rs:282:9: replace Entry::occupied -> bool with false
crates/vocab/src/table.rs:1628:14: replace ^= with &= in fnv1a
crates/api/src/backtest.rs:1316:5: replace unparseable_limit -> Option<String> with Some("xyzzy".into())
crates/api/src/detail.rs:155:5: replace run_store_read -> Result<T, RunError> with Ok(Default::default())
crates/api/src/pullrun.rs:84:56: replace * with +
crates/api/src/server.rs:2128:5: replace inexact_bar -> Option<String> with Some("xyzzy".into())
crates/api/src/server.rs:17604:9: replace <impl http_body::Body for DeadlineBody>::poll_frame -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> with Poll::from(Some(Ok(Frame::new())))
crates/api/src/sweeprun.rs:2701:5: replace observe_elsewhere -> ExternalObservation with Default::default()
crates/cli/src/lib.rs:7550:9: replace AuditCache::inputs -> Result<&AuditInputs, stored::Refusal> with Ok(Box::leak(Box::new(Default::default())))
crates/cli/src/lib.rs:17076:14: replace match guard pessimistic <= 0 with true in return_over_drawdown_cell
crates/cli/src/columns.rs:122:5: replace line -> String with "xyzzy".into()
crates/cli/src/and_checkpoint.rs:146:5: replace walk_within -> Result<Sweep, String> with Ok(Default::default())
crates/cli/src/candidate_universe.rs:4166:85: replace != with == in session_close_minute_v1
crates/cli/src/institutional_statistics.rs:1600:5: replace ceiling_ppm_of -> u64 with 0
crates/cli/src/pool.rs:527:5: replace not_walked with ()
crates/cli/src/pre_admission_data.rs:2544:9: replace PreAdmissionDataLedgerV2::append_complete_locked -> Result<PreAdmissionProductionCommitV2, PreAdmissionDataRefusal> with Ok(Default::default())
crates/cli/src/step3_orchestrator.rs:536:5: replace stored_candidate_swept_v1 -> Result<u64, Step3OrchestratorRefusal> with Ok(0)
crates/cli/src/selection_v6_read.rs:213:5: replace read_stored_selection_v6 -> Vec<(&'static str, StoredSelectionV6Rung)> with vec![("xyzzy", Default::default())]
crates/engine/src/keep.rs:175:74: replace == with != in Streamed::depth
crates/indicators/src/pattern.rs:771:27: replace < with <= in Patterns::bits
crates/lake/src/page.rs:374:5: replace read_header -> PqResult<(PageHeader, usize)> with PqResult::new()
crates/pull/src/csv.rs:858:14: replace == with != in decode_rows
crates/pull/src/http.rs:3650:43: replace + with * in stated_offset
crates/pull/src/scrub.rs:346:9: replace Tally::clean -> bool with false
crates/runner/src/audit.rs:1041:5: replace ratio_or_no_dd -> String with String::new()
crates/runner/src/exit_grid_policy.rs:347:9: replace ExecutionDigestsV1::execution_digest -> [u8; 32] with [0; 32]
crates/runner/src/outcome.rs:694:66: replace || with && in WindowExtremes::over
crates/runner/src/resample.rs:97:31: replace * with /
crates/runner/src/significance.rs:472:27: replace + with - in beta_continued_fraction
crates/runner/src/expression_validation.rs:260:61: replace > with < in index_folds
crates/store/src/file.rs:3973:34: replace == with != in is_interrupted_genesis
crates/store/src/time_index.rs:282:40: replace & with | in Entry::occupied
crates/vocab/src/table.rs:1630:11: replace += with *= in fnv1a
crates/api/src/bars.rs:427:5: replace page -> (Vec<Bar>, Vec<String>) with (vec![], vec![])
crates/api/src/detail.rs:186:5: replace admission_refused -> (axum::http::StatusCode, String) with (Default::default(), "xyzzy".into())
crates/api/src/pullrun.rs:84:51: replace + with *
crates/api/src/server.rs:2147:5: replace inexact_fault -> String with String::new()
crates/api/src/server.rs:17604:9: replace <impl http_body::Body for DeadlineBody>::poll_frame -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> with Poll::from(Some(Ok(Frame::from_iter([Default::default()]))))
crates/api/src/sweeprun.rs:2794:5: replace unterminated_marker -> Option<(u64, String)> with Some((0, "xyzzy".into()))
crates/cli/src/lib.rs:7565:59: replace != with == in AuditCache::raw
crates/cli/src/lib.rs:17192:5: replace rungs_not_cancelled -> Result<(), String> with Ok(())
crates/cli/src/columns.rs:133:23: replace == with != in line
crates/cli/src/and_checkpoint.rs:246:5: replace recover -> Result<Option<(u64, [u8; 32], Boundary)>, String> with Ok(Some((0, [0; 32], Default::default())))
crates/cli/src/candidate_universe.rs:4178:49: replace == with != in session_close_minute_v1
crates/cli/src/institutional_statistics.rs:1600:44: replace * with / in ceiling_ppm_of
crates/cli/src/pool.rs:720:5: replace opening -> String with "xyzzy".into()
crates/cli/src/pre_admission_data.rs:3621:5: replace open_file -> Result<File, PreAdmissionDataRefusal> with Ok(Default::default())
crates/cli/src/step3_orchestrator.rs:830:9: replace CommittedStoredCandidatePreAdmissionV1::memoized_execution_v3_replay -> Result<CandidateExecutionReplayAuthorityV1, Step3OrchestratorRefusal> with Ok(Default::default())
crates/cli/src/selection_v6_read.rs:223:21: replace match guard why.kind() == std::io::ErrorKind::NotFound with false in read_rung
crates/engine/src/keep.rs:350:9: replace Best::repeated -> u64 with 1
crates/indicators/src/pattern.rs:772:27: replace > with >= in Patterns::bits
crates/lake/src/page.rs:374:5: replace read_header -> PqResult<(PageHeader, usize)> with PqResult::from((Default::default(), 0))
crates/pull/src/csv.rs:1013:51: replace && with || in decode_rows
crates/pull/src/http.rs:3650:30: replace - with + in stated_offset
crates/pull/src/scrub.rs:346:44: replace == with != in Tally::clean
crates/runner/src/audit.rs:1087:5: replace grid with ()
crates/runner/src/exit_grid_policy.rs:448:9: replace ExecutionRunV1::new_with_daily_reference -> Result<Self, ExitGridErrorV1> with Ok(Default::default())
crates/runner/src/outcome.rs:694:72: replace < with <= in WindowExtremes::over
crates/runner/src/resample.rs:109:61: replace * with +
crates/runner/src/significance.rs:472:33: replace / with * in beta_continued_fraction
crates/runner/src/expression_validation.rs:261:18: replace += with *= in index_folds
crates/store/src/file.rs:3988:21: replace match guard meta.len() > 0 with false in refuse_if_sealed
crates/store/src/time_index.rs:282:33: replace % with / in Entry::occupied
crates/vocab/src/table.rs:1641:11: replace & with | in home
crates/api/src/bars.rs:427:5: replace page -> (Vec<Bar>, Vec<String>) with (vec![], vec![String::new()])
crates/api/src/detail.rs:205:5: replace admitted -> Result<T, RunError> with Ok(Default::default())
crates/api/src/pullrun.rs:289:9: replace Progress::claimed -> Self with Default::default()
crates/api/src/server.rs:2147:5: replace inexact_fault -> String with "xyzzy".into()
crates/api/src/server.rs:17604:9: replace <impl http_body::Body for DeadlineBody>::poll_frame -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> with Poll::from_iter([Some(Ok(Frame::new(Default::default())))])
crates/api/src/sweeprun.rs:2794:5: replace unterminated_marker -> Option<(u64, String)> with Some((1, String::new()))
crates/cli/src/lib.rs:7586:5: replace audit_range_kernel -> Result<String, stored::Refusal> with Ok(String::new())
crates/cli/src/lib.rs:17241:5: replace sweep_rungs -> Vec<RungRow> with vec![]
crates/cli/src/columns.rs:133:44: replace == with != in line
crates/cli/src/and_checkpoint.rs:246:5: replace recover -> Result<Option<(u64, [u8; 32], Boundary)>, String> with Ok(Some((0, [1; 32], Default::default())))
crates/cli/src/candidate_universe.rs:4178:33: replace % with / in session_close_minute_v1
crates/cli/src/knobs.rs:216:5: replace policy_floor -> Option<i64> with None
crates/cli/src/pool.rs:749:5: replace render_per_symbol with ()
crates/cli/src/pre_admission_data.rs:3633:5: replace file_generation -> Result<FileGenerationV1, PreAdmissionDataRefusal> with Ok(Default::default())
crates/cli/src/step3_orchestrator.rs:832:13: replace && with || in CommittedStoredCandidatePreAdmissionV1::memoized_execution_v3_replay
crates/cli/src/selection_v6_read.rs:223:32: replace == with != in read_rung
crates/engine/src/keep.rs:379:9: replace Best::offer with ()
crates/indicators/src/pattern.rs:783:13: replace && with || in Patterns::bits
crates/lake/src/page.rs:374:5: replace read_header -> PqResult<(PageHeader, usize)> with PqResult::from_iter([(Default::default(), 1)])
crates/pull/src/csv.rs:1013:34: replace == with != in decode_rows
crates/pull/src/http.rs:3650:30: replace - with / in stated_offset
crates/pull/src/session.rs:261:9: replace <impl fmt::Display for SessionError>::fmt -> fmt::Result with Ok(Default::default())
crates/runner/src/audit.rs:1140:18: replace && with || in grid
crates/runner/src/exit_grid_policy.rs:458:9: replace ExecutionRunV1::seal -> Result<Self, ExitGridErrorV1> with Ok(Default::default())
crates/runner/src/outcome.rs:705:25: replace <= with > in WindowExtremes::over
crates/runner/src/resample.rs:109:61: replace * with /
crates/runner/src/significance.rs:473:26: replace * with + in beta_continued_fraction
crates/runner/src/expression_validation.rs:266:19: replace < with == in index_folds
crates/store/src/file.rs:3994:21: replace match guard why.kind() == ErrorKind::NotFound with true in refuse_if_sealed
crates/store/src/time_index.rs:282:33: replace % with + in Entry::occupied
crates/vocab/src/table.rs:1641:11: replace & with ^ in home
crates/api/src/bars.rs:427:5: replace page -> (Vec<Bar>, Vec<String>) with (vec![Default::default()], vec!["xyzzy".into()])
crates/api/src/detail.rs:347:17: replace || with && in Selector::parse
crates/api/src/pullrun.rs:444:9: replace Refusal::why -> String with String::new()
crates/api/src/server.rs:2418:5: replace bars_json -> (axum::http::StatusCode, [(axum::http::HeaderName, &'static str); 1], String,) with (Default::default(), [(Default::default(), "xyzzy"); 1], String::new())
crates/api/src/server.rs:17604:9: replace <impl http_body::Body for DeadlineBody>::poll_frame -> std::task::Poll<Option<Result<http_body::Frame<Self::Data>, Self::Error>>> with Poll::new(Some(Ok(Frame::from(Default::default()))))
crates/api/src/sweeprun.rs:2798:17: replace && with || in unterminated_marker
crates/cli/src/lib.rs:7737:8: delete ! in load_audit_inputs
crates/cli/src/lib.rs:17298:5: replace range_table with ()
crates/cli/src/../commit_stamp.rs:9:5: replace canonical -> bool with false
crates/cli/src/and_checkpoint.rs:250:24: replace match guard magic == BOUNDARY_MAGIC with false in recover
crates/cli/src/candidate_universe.rs:4184:59: replace - with + in session_close_minute_v1
crates/cli/src/knobs.rs:218:9: delete match arm "BRUTEX_MIN_WIN_RATE_BP" in policy_floor
crates/cli/src/pool.rs:860:5: replace union_of -> (Vec<Candidate>, Vec<(String, String)>) with (vec![], vec![("xyzzy".into(), String::new())])
crates/cli/src/pre_admission_data.rs:3749:33: replace != with == in require_metadata_generation
crates/cli/src/step3_orchestrator.rs:3151:5: replace commit_family_from_v6 -> Result<family_v6::StoredFamilyV6, String> with Ok(Default::default())
crates/cli/src/selection_v6_read.rs:247:5: replace read_blocks -> Result<(u64, Vec<StoredSelectionV6Record>), String> with Ok((1, vec![Default::default()]))
crates/engine/src/resume.rs:178:40: replace != with == in Checkpoint::validate_for
crates/indicators/src/pattern.rs:780:26: replace < with == in Patterns::bits
crates/lake/src/page.rs:396:5: replace rowless_extent -> Option<usize> with Some(0)
crates/pull/src/daycheck.rs:61:28: replace && with || in Report::clean
crates/pull/src/http.rs:3651:45: replace + with * in stated_offset
crates/pull/src/session.rs:961:9: replace DropCensus::count with ()
crates/runner/src/audit.rs:1140:27: replace < with == in grid
crates/runner/src/exit_grid_policy.rs:592:5: replace require_exact_execution_subslice -> Result<(), ExitGridErrorV1> with Ok(())
crates/runner/src/outcome.rs:718:60: replace < with > in WindowExtremes::over
crates/runner/src/resample.rs:156:25: replace * with + in Period::micros
crates/runner/src/significance.rs:475:31: replace < with == in beta_continued_fraction
crates/runner/src/expression_validation.rs:270:51: replace != with == in index_folds
crates/store/src/file.rs:3988:32: replace > with >= in refuse_if_sealed
crates/store/src/time_index.rs:288:32: replace + with * in Entry::total
crates/vocab/src/table.rs:1663:5: replace name_index -> [u16; NAME_SLOTS] with [1; NAME_SLOTS]
crates/api/src/bars.rs:440:5: replace slots -> (Vec<Option<Bar>>, Vec<String>) with (vec![None], vec![])
crates/api/src/detail.rs:370:5: replace integer_param -> Result<Option<u64>, String> with Ok(None)
crates/api/src/pullrun.rs:507:23: replace == with != in legs_from
crates/api/src/server.rs:2596:5: replace gaps_json -> (axum::http::StatusCode, [(axum::http::HeaderName, &'static str); 1], String,) with (Default::default(), [(Default::default(), "xyzzy"); 1], String::new())
crates/api/src/server.rs:17626:9: replace <impl http_body::Body for DeadlineBody>::size_hint -> http_body::SizeHint with Default::default()
crates/api/src/sweeprun.rs:2798:31: replace > with < in unterminated_marker
crates/cli/src/lib.rs:8267:5: replace keep_best with ()
crates/cli/src/lib.rs:17663:9: replace && with || in recorded_row
crates/cli/src/fixed_tail.rs:84:5: replace roll_back -> String with "xyzzy".into()
crates/cli/src/and_checkpoint.rs:251:30: replace == with != in recover
crates/cli/src/candidate_universe.rs:4199:58: replace && with || in session_close_minute_v1
crates/cli/src/knobs.rs:305:5: replace var -> Option<String> with Some(String::new())
crates/cli/src/pool.rs:860:5: replace union_of -> (Vec<Candidate>, Vec<(String, String)>) with (vec![Default::default()], vec![(String::new(), "xyzzy".into())])
crates/cli/src/research.rs:164:5: replace render -> String with String::new()
crates/cli/src/step3_orchestrator.rs:3482:5: replace requested_execution_range -> Result<Range<usize>, Step3OrchestratorRefusal> with Ok(Range::from(0))
crates/cli/src/selection_v6_read.rs:253:23: replace % with / in read_blocks
crates/engine/src/resume.rs:253:25: replace > with == in Checkpoint::validate
crates/indicators/src/pattern.rs:781:26: replace > with < in Patterns::bits
crates/lake/src/page.rs:402:43: replace == with != in rowless_extent
crates/pull/src/daycheck.rs:67:5: replace ist_day -> i64 with 1
crates/pull/src/http.rs:3651:32: replace - with / in stated_offset
crates/pull/src/session.rs:998:9: replace DropCensus::unclassified_kept -> u32 with 0
crates/runner/src/audit.rs:1141:50: replace - with / in grid
crates/runner/src/exit_grid_policy.rs:601:26: replace == with != in require_exact_execution_subslice
crates/runner/src/outcome.rs:721:59: replace < with <= in WindowExtremes::over
crates/runner/src/resample.rs:162:9: replace Period::anchor -> i64 with -1
crates/runner/src/significance.rs:475:18: replace - with / in beta_continued_fraction
crates/store/src/catalog.rs:173:9: replace Census::unoffered_report -> String with String::new()
crates/store/src/file.rs:4013:63: replace && with || in missing_below
crates/store/src/time_index.rs:294:27: replace % with + in Entry::through
crates/vocab/src/table.rs:1667:25: replace != with == in name_index
crates/api/src/bars.rs:440:5: replace slots -> (Vec<Option<Bar>>, Vec<String>) with (vec![None], vec![String::new()])
crates/api/src/detail.rs:370:5: replace integer_param -> Result<Option<u64>, String> with Ok(Some(0))
crates/api/src/pullrun.rs:532:12: delete ! in legs_from
crates/api/src/server.rs:2596:5: replace gaps_json -> (axum::http::StatusCode, [(axum::http::HeaderName, &'static str); 1], String,) with (Default::default(), [(Default::default(), "xyzzy"); 1], "xyzzy".into())
crates/api/src/server.rs:17642:5: replace body_deadline -> axum::response::Response with Default::default()
crates/api/src/sweeprun.rs:2798:31: replace > with >= in unterminated_marker
crates/cli/src/lib.rs:8268:9: replace && with || in keep_best
crates/cli/src/lib.rs:17662:9: replace && with || in recorded_row
crates/cli/src/fixed_tail.rs:96:5: replace start -> Result<u64, String> with Ok(0)
crates/cli/src/and_checkpoint.rs:253:25: replace == with != in recover
crates/cli/src/candidate_universe.rs:4199:36: replace <= with > in session_close_minute_v1
crates/cli/src/knobs.rs:305:5: replace var -> Option<String> with Some("xyzzy".into())
crates/cli/src/pool.rs:860:5: replace union_of -> (Vec<Candidate>, Vec<(String, String)>) with (vec![Default::default()], vec![("xyzzy".into(), String::new())])
crates/cli/src/research.rs:164:5: replace render -> String with "xyzzy".into()
crates/cli/src/step3_orchestrator.rs:3482:5: replace requested_execution_range -> Result<Range<usize>, Step3OrchestratorRefusal> with Ok(Range::from_iter([1]))
crates/cli/src/selection_v6_read.rs:253:23: replace % with + in read_blocks
crates/engine/src/resume.rs:253:25: replace > with < in Checkpoint::validate
crates/indicators/src/pattern.rs:781:26: replace > with >= in Patterns::bits
crates/lake/src/page.rs:407:14: replace && with || in rowless_extent
crates/pull/src/daycheck.rs:67:5: replace ist_day -> i64 with -1
crates/pull/src/http.rs:3651:60: replace - with + in stated_offset
crates/pull/src/session.rs:998:9: replace DropCensus::unclassified_kept -> u32 with 1
crates/runner/src/audit.rs:1165:8: delete ! in grid
crates/runner/src/exit_grid_policy.rs:601:19: replace % with / in require_exact_execution_subslice
crates/runner/src/outcome.rs:734:9: replace WindowExtremes::over_blocks -> Option<(i64, i64)> with None
crates/runner/src/resample.rs:162:19: replace >= with < in Period::anchor
crates/runner/src/significance.rs:484:5: replace ln_beta -> f64 with 0.0
crates/store/src/catalog.rs:173:9: replace Census::unoffered_report -> String with "xyzzy".into()
crates/store/src/file.rs:4013:55: replace != with == in missing_below
crates/store/src/time_index.rs:297:38: replace - with + in Entry::through
crates/vocab/src/table.rs:1668:27: replace & with | in name_index
crates/api/src/bars.rs:440:5: replace slots -> (Vec<Option<Bar>>, Vec<String>) with (vec![None], vec!["xyzzy".into()])
crates/api/src/detail.rs:370:5: replace integer_param -> Result<Option<u64>, String> with Ok(Some(1))
crates/api/src/pullrun.rs:534:37: replace == with != in legs_from
crates/api/src/server.rs:2761:5: replace audit_span -> Result<(Vec<AuditedMonth>, PeerCalendar), crate::detail::RunError> with Ok((vec![], Default::default()))
crates/api/src/server.rs:17647:48: replace + with - in body_deadline
crates/api/src/sweeprun.rs:2826:5: replace ended_without_terminal -> Option<u64> with None
crates/cli/src/lib.rs:8270:48: replace >= with < in keep_best
crates/cli/src/lib.rs:17661:9: replace && with || in recorded_row
crates/cli/src/fixed_tail.rs:96:5: replace start -> Result<u64, String> with Ok(1)
crates/cli/src/and_checkpoint.rs:254:26: replace != with == in recover
crates/cli/src/candidate_universe.rs:4199:82: replace >= with < in session_close_minute_v1
crates/cli/src/knobs.rs:328:5: replace resolve -> Option<String> with None
crates/cli/src/pool.rs:860:5: replace union_of -> (Vec<Candidate>, Vec<(String, String)>) with (vec![Default::default()], vec![("xyzzy".into(), "xyzzy".into())])
crates/cli/src/research.rs:236:5: replace command -> u8 with 0
crates/cli/src/step3_orchestrator.rs:3482:5: replace requested_execution_range -> Result<Range<usize>, Step3OrchestratorRefusal> with Ok(Range::new(1))
crates/cli/src/selection_v6_read.rs:260:32: replace / with % in read_blocks
crates/engine/src/resume.rs:253:25: replace > with >= in Checkpoint::validate
crates/indicators/src/pattern.rs:782:27: replace > with == in Patterns::bits
crates/lake/src/page.rs:407:21: replace <= with > in rowless_extent
crates/pull/src/daycheck.rs:67:42: replace + with - in ist_day
crates/pull/src/http.rs:3651:60: replace - with / in stated_offset
crates/pull/src/session.rs:1018:9: replace DropCensus::total -> u32 with 0
crates/runner/src/audit.rs:1184:56: delete ! in grid
crates/runner/src/exit_grid_policy.rs:601:19: replace % with + in require_exact_execution_subslice
crates/runner/src/outcome.rs:734:9: replace WindowExtremes::over_blocks -> Option<(i64, i64)> with Some((0, 0))
crates/runner/src/resample.rs:195:9: replace <impl core::fmt::Display for ResampleError>::fmt -> core::fmt::Result with Ok(Default::default())
crates/runner/src/significance.rs:484:5: replace ln_beta -> f64 with 1.0
crates/store/src/catalog.rs:174:72: replace || with && in Census::unoffered_report
crates/store/src/file.rs:4014:12: delete ! in missing_below
crates/store/src/time_index.rs:297:38: replace - with / in Entry::through
crates/vocab/src/table.rs:1668:27: replace & with ^ in name_index
crates/api/src/bars.rs:440:5: replace slots -> (Vec<Option<Bar>>, Vec<String>) with (vec![Some(Default::default())], vec![String::new()])
crates/api/src/detail.rs:438:5: replace window -> Result<Window, String> with Ok(Default::default())
crates/api/src/pullrun.rs:585:5: replace envelope_disagreement -> Option<String> with Some(String::new())
crates/api/src/server.rs:3223:5: replace audit_one -> AuditedMonth with Default::default()
crates/api/src/server.rs:17691:9: replace Slots::try_take -> bool with true
crates/api/src/sweeprun.rs:2826:5: replace ended_without_terminal -> Option<u64> with Some(1)
crates/cli/src/lib.rs:8344:5: replace rupees -> String with "xyzzy".into()
crates/cli/src/lib.rs:17658:9: replace && with || in recorded_row
crates/cli/src/fixed_tail.rs:146:5: replace append_block -> Result<u64, String> with Ok(0)
crates/cli/src/and_checkpoint.rs:267:17: replace != with == in recover
crates/cli/src/candidate_universe.rs:4786:5: replace replay_execution_side -> Result<(), CandidateUniverseRefusal> with Ok(())
crates/cli/src/knobs.rs:328:5: replace resolve -> Option<String> with Some("xyzzy".into())
crates/cli/src/pool.rs:980:5: replace price_all -> Result<Vec<Priced>, String> with Ok(vec![Default::default()])
crates/cli/src/research_policy.rs:206:5: replace command -> u8 with 0
crates/cli/src/step3_orchestrator.rs:3510:16: replace > with == in requested_execution_range
crates/cli/src/selection_v6_read.rs:262:18: replace > with == in read_blocks
crates/engine/src/resume.rs:254:71: replace > with == in Checkpoint::validate
crates/indicators/src/pattern.rs:782:27: replace > with >= in Patterns::bits
crates/lake/src/page.rs:424:5: replace step_over_rowless -> &[u8] with Vec::leak(vec![0])
crates/pull/src/daycheck.rs:73:5: replace differing -> String with String::new()
crates/pull/src/http.rs:3735:5: replace local_seconds -> Option<i64> with Some(0)
crates/pull/src/session.rs:1163:9: replace Window::verdict -> Result<Option<DropReason>, SessionError> with Ok(None)
crates/runner/src/audit.rs:1187:35: replace match guard key(b) < key(a) with false in grid
crates/runner/src/exit_grid_policy.rs:602:39: replace / with * in require_exact_execution_subslice
crates/runner/src/outcome.rs:734:9: replace WindowExtremes::over_blocks -> Option<(i64, i64)> with Some((0, -1))
crates/runner/src/resample.rs:243:5: replace bucket_start -> Option<i64> with Some(0)
crates/runner/src/significance.rs:484:31: replace - with + in ln_beta
crates/store/src/catalog.rs:174:32: replace || with && in Census::unoffered_report
crates/store/src/file.rs:4051:5: replace no_follow -> &mut fs::OpenOptions with Box::leak(Box::new(Default::default()))
crates/store/src/time_index.rs:297:32: replace + with - in Entry::through
crates/vocab/src/table.rs:1668:22: replace + with * in name_index
crates/api/src/bars.rs:440:5: replace slots -> (Vec<Option<Bar>>, Vec<String>) with (vec![Some(Default::default())], vec!["xyzzy".into()])
crates/api/src/detail.rs:477:5: replace seek_window -> Result<Window, String> with Ok(Default::default())
crates/api/src/pullrun.rs:585:5: replace envelope_disagreement -> Option<String> with Some("xyzzy".into())
crates/api/src/server.rs:3268:39: replace && with || in audit_one
crates/api/src/server.rs:17691:9: replace Slots::try_take -> bool with false
crates/api/src/sweeprun.rs:2829:16: replace match guard record.origin == cli::operation_audit::Origin::Cli && record.label == *command with true in ended_without_terminal
crates/cli/src/lib.rs:8345:26: replace < with == in rupees
crates/cli/src/lib.rs:17657:31: replace == with != in recorded_row
crates/cli/src/fixed_tail.rs:146:5: replace append_block -> Result<u64, String> with Ok(1)
crates/cli/src/and_checkpoint.rs:287:5: replace publish_level -> Result<Vec<Piece>, String> with Ok(vec![])
crates/cli/src/candidate_universe.rs:4955:5: replace hoist_execution_series -> Result<HoistedExecutionV1<'s>, CandidateUniverseRefusal> with Ok(Default::default())
crates/cli/src/ledger_all.rs:242:5: replace exit_policy_with -> Result<ExitGridPolicyV1, String> with Ok(Default::default())
crates/cli/src/pool.rs:992:36: replace > with == in price_all
crates/cli/src/research_policy.rs:206:5: replace command -> u8 with 1
crates/cli/src/step3_orchestrator.rs:3510:16: replace > with < in requested_execution_range
crates/cli/src/selection_v6_read.rs:262:18: replace > with < in read_blocks
crates/engine/src/resume.rs:254:71: replace > with < in Checkpoint::validate
crates/indicators/src/pattern.rs:783:27: replace < with == in Patterns::bits
crates/lake/src/page.rs:424:5: replace step_over_rowless -> &[u8] with Vec::leak(vec![1])
crates/pull/src/daycheck.rs:73:5: replace differing -> String with "xyzzy".into()
crates/pull/src/http.rs:3735:5: replace local_seconds -> Option<i64> with Some(1)
crates/pull/src/session.rs:1163:9: replace Window::verdict -> Result<Option<DropReason>, SessionError> with Ok(Some(Default::default()))
crates/runner/src/audit.rs:1187:42: replace < with == in grid
crates/runner/src/exit_grid_policy.rs:604:65: replace / with % in require_exact_execution_subslice
crates/runner/src/outcome.rs:734:9: replace WindowExtremes::over_blocks -> Option<(i64, i64)> with Some((1, 0))
crates/runner/src/resample.rs:243:5: replace bucket_start -> Option<i64> with Some(1)
crates/runner/src/significance.rs:484:31: replace - with / in ln_beta
crates/store/src/catalog.rs:174:28: replace > with == in Census::unoffered_report
crates/store/src/file.rs:4069:5: replace writer_open -> Result<File, StoreError> with Ok(Default::default())
crates/store/src/time_index.rs:297:32: replace + with * in Entry::through
crates/vocab/src/table.rs:1668:41: replace - with + in name_index
crates/api/src/bars.rs:698:5: replace with_change -> Vec<WindowBar> with vec![]
crates/api/src/detail.rs:479:18: replace > with == in seek_window
crates/api/src/pullrun.rs:587:13: replace != with == in envelope_disagreement
crates/api/src/server.rs:3268:30: replace == with != in audit_one
crates/api/src/server.rs:17694:20: replace < with == in Slots::try_take
crates/api/src/sweeprun.rs:2829:16: replace match guard record.origin == cli::operation_audit::Origin::Cli && record.label == *command with false in ended_without_terminal
crates/cli/src/lib.rs:8345:26: replace < with > in rupees
crates/cli/src/lib.rs:17659:13: replace == with != in recorded_row
crates/cli/src/fixed_tail.rs:170:5: replace sync_or_roll_back -> Result<(), String> with Ok(())
crates/cli/src/and_checkpoint.rs:287:5: replace publish_level -> Result<Vec<Piece>, String> with Ok(vec![Default::default()])
crates/cli/src/candidate_universe.rs:4993:5: replace hoisted_execution_series -> Result<&'h HoistedExecutionV1<'s>, CandidateUniverseRefusal> with Ok(Box::leak(Box::new(Default::default())))
crates/cli/src/ledger_all.rs:825:5: replace ledger_all -> String with String::new()
crates/cli/src/pool.rs:992:36: replace > with < in price_all
crates/cli/src/research_policy.rs:220:5: replace risk_of -> Result<(u64, u64), String> with Ok((0, 0))
crates/cli/src/step3_orchestrator.rs:3510:16: replace > with >= in requested_execution_range
crates/cli/src/selection_v6_read.rs:262:18: replace > with >= in read_blocks
crates/engine/src/resume.rs:254:71: replace > with >= in Checkpoint::validate
crates/indicators/src/pattern.rs:783:27: replace < with > in Patterns::bits
crates/lake/src/page.rs:456:9: replace <impl PageReader for LakePageReader>::get_next_page -> PqResult<Option<Page>> with PqResult::new()
crates/pull/src/daycheck.rs:82:46: replace != with == in differing
crates/pull/src/http.rs:3735:5: replace local_seconds -> Option<i64> with Some(-1)
crates/pull/src/session.rs:1231:20: replace match guard session != crate::calendar::Session::full() with true in Window::verdict
crates/runner/src/audit.rs:1187:42: replace < with > in grid
crates/runner/src/exit_grid_policy.rs:604:65: replace / with * in require_exact_execution_subslice
crates/runner/src/outcome.rs:734:9: replace WindowExtremes::over_blocks -> Option<(i64, i64)> with Some((1, 1))
crates/runner/src/resample.rs:243:5: replace bucket_start -> Option<i64> with Some(-1)
crates/runner/src/significance.rs:484:17: replace + with - in ln_beta
crates/store/src/catalog.rs:174:28: replace > with < in Census::unoffered_report
crates/store/src/file.rs:4077:31: replace == with != in writer_open
crates/store/src/time_index.rs:297:27: replace % with / in Entry::through
crates/vocab/src/table.rs:1668:41: replace - with / in name_index
crates/api/src/bars.rs:724:40: replace match guard before.open_interest == OI_NULL with false in with_change
crates/api/src/detail.rs:643:5: replace lock_through_poison -> std::sync::MutexGuard<'_, V> with MutexGuard::from_iter([Default::default()])
crates/api/src/pullrun.rs:660:5: replace rows_now -> Option<u64> with Some(1)
crates/api/src/server.rs:3471:5: replace bars_window_json -> (axum::http::StatusCode, [(axum::http::HeaderName, &'static str); 1], String,) with (Default::default(), [(Default::default(), ""); 1], String::new())
crates/api/src/server.rs:17729:9: replace <impl axum::serve::Listener for LimitedListener>::accept -> (Self::Io, Self::Addr) with (Default::default(), Default::default())
crates/api/src/sweeprun.rs:2840:5: replace admit_external -> Result<(), Refusal> with Ok(())
crates/cli/src/lib.rs:8388:5: replace div_round_half_away -> i64 with -1
crates/cli/src/lib.rs:17663:28: replace == with != in recorded_row
crates/cli/src/fixed_tail.rs:232:5: replace refuse_after_failed_barrier -> Result<(), String> with Ok(())
crates/cli/src/and_checkpoint.rs:290:35: replace > with < in publish_level
crates/cli/src/candidate_universe.rs:5217:5: replace append_validated_grid_rows -> Result<(), CandidateUniverseRefusal> with Ok(())
crates/cli/src/ledger_all.rs:977:43: replace match guard open == rung with false in render_winners
crates/cli/src/pool.rs:1008:44: replace > with >= in price_all
crates/cli/src/research_policy.rs:232:5: replace explain -> Result<String, String> with Ok(String::new())
crates/cli/src/stored.rs:564:9: replace <impl PartialEq for CashCloses>::eq -> bool with true
crates/cli/src/selection_v6_read.rs:306:9: replace Decoder<'_>::take -> Result<[u8; N], String> with Ok([1; N])
crates/engine/src/resume.rs:435:5: replace write_level -> Result<(), Error> with Ok(())
crates/indicators/src/pattern.rs:811:13: replace && with || in Patterns::bits
crates/lake/src/page.rs:456:9: replace <impl PageReader for LakePageReader>::get_next_page -> PqResult<Option<Page>> with PqResult::from_iter([Some(Default::default())])
crates/pull/src/daycheck.rs:112:21: replace && with || in compare
crates/pull/src/http.rs:3742:9: delete match arm 13 | 16 in local_seconds
crates/pull/src/session.rs:1270:5: replace irregular_verdict -> Option<DropReason> with Some(Default::default())
crates/runner/src/bootstrap.rs:175:9: replace Verdict::clears -> bool with false
crates/runner/src/exit_grid_policy.rs:1430:40: replace != with == in AttestedTrainingV1<'_>::cell_replay
crates/runner/src/outcome.rs:734:9: replace WindowExtremes::over_blocks -> Option<(i64, i64)> with Some((-1, -1))
crates/runner/src/resample.rs:344:5: replace opened -> Candle with Default::default()
crates/runner/src/significance.rs:491:5: replace ln_gamma -> f64 with 0.0
crates/store/src/catalog.rs:174:47: replace > with >= in Census::unoffered_report
crates/store/src/file.rs:4102:5: replace refuse_links_below -> Result<(), StoreError> with Ok(())
crates/store/src/time_index.rs:308:9: replace Entry::encode -> [u8; 16] with [0; 16]
crates/vocab/src/table.rs:1677:13: replace += with *= in name_index
crates/api/src/bars.rs:893:5: replace seek_page -> (Vec<WindowBar>, Vec<String>, Option<&BarFile>) with (vec![], vec!["xyzzy".into()], Some(Box::leak(Box::new(Default::default()))))
crates/api/src/detail.rs:870:5: delete ! in must_admit
crates/api/src/pullrun.rs:941:5: replace run_chain -> PassOutcome with Default::default()
crates/api/src/server.rs:4387:5: replace basis_points -> Result<i64, Unknown> with Ok(1)
crates/api/src/server.rs:17898:9: replace <impl tokio::io::AsyncRead for HeadDeadline>::poll_read -> std::task::Poll<std::io::Result<()>> with Poll::new(Ok(()))
crates/api/src/sweeprun.rs:3424:5: replace command_with_configuration -> (axum::http::StatusCode, JsonHeaders, String) with (Default::default(), Default::default(), "xyzzy".into())
crates/cli/src/lib.rs:8993:5: replace render_top_record -> String with "xyzzy".into()
crates/cli/src/lib.rs:17810:5: replace screen_range_for_attempt -> String with String::new()
crates/cli/src/fixed_tail.rs:371:5: replace heal_torn_header -> Result<Option<TornTail>, String> with Ok(Some(Default::default()))
crates/cli/src/and_checkpoint.rs:412:17: replace != with == in chunk_header
crates/cli/src/execution_capability.rs:1251:5: replace validate_population_capabilities -> Result<(u64, u64), ExecutionCapabilityRefusal> with Ok((1, 0))
crates/cli/src/live.rs:230:9: replace && with || in clears_bar
crates/cli/src/pool.rs:1307:31: replace > with >= in pooled_entries
crates/cli/src/result_set.rs:685:5: replace hash_range -> Result<u64, Refusal> with Ok(0)
crates/cli/src/stored.rs:616:9: replace CashCloses::unverified_reason -> Option<&str> with Some("")
crates/cli/src/selection_v6_read.rs:467:20: replace + with - in decode_block
crates/greeks/src/solver.rs:424:9: replace Checked::bracket -> (f64, f64) with (0.0, 0.0)
crates/indicators/src/pattern.rs:813:27: replace < with > in Patterns::bits
crates/lake/src/reader.rs:285:9: replace LakeFile::read_row_group -> Result<Batch, LakeError> with Ok(Default::default())
crates/pull/src/fno.rs:187:5: replace names -> Result<Vec<String>, FnoError> with Ok(vec![String::new()])
crates/pull/src/ingest.rs:2504:5: replace check_day with ()
crates/pull/src/ssm.rs:432:24: replace > with == in AwsIdentity::from_credentials_file
crates/runner/src/bootstrap.rs:526:5: replace white_null_is_a_point_mass -> bool with true
crates/runner/src/exit_grid_policy.rs:2843:9: replace ResolvedExitGridV1::replay_selected_universe_with -> Result<ReplayedCandidateUniverseV1, ExitGridErrorV1> with Ok(Default::default())
crates/runner/src/outcome.rs:814:24: replace > with == in BlockExtremes::of
crates/runner/src/significance.rs:337:15: replace == with != in clears_bonferroni
crates/runner/src/significance.rs:499:36: replace - with / in ln_gamma
crates/store/src/catalog.rs:376:21: replace match guard why.kind() == std::io::ErrorKind::NotFound with true in bars_present
crates/store/src/format.rs:727:9: replace <impl Row for Greek>::is_sane -> bool with false
crates/store/src/time_index.rs:485:5: replace locate -> Result<u64, Fault> with Ok(0)
crates/vocab/src/tolerance.rs:313:42: replace * with /
crates/api/src/bars.rs:893:5: replace seek_page -> (Vec<WindowBar>, Vec<String>, Option<&BarFile>) with (vec![Default::default()], vec![], None)
crates/api/src/detail.rs:870:23: replace && with || in must_admit
crates/api/src/pullrun.rs:1023:5: replace run_pass with ()
crates/api/src/server.rs:4387:5: replace basis_points -> Result<i64, Unknown> with Ok(-1)
crates/api/src/server.rs:17898:9: replace <impl tokio::io::AsyncRead for HeadDeadline>::poll_read -> std::task::Poll<std::io::Result<()>> with Poll::from(Ok(()))
crates/api/src/sweeprun.rs:3586:5: replace strict_terminal_audit with ()
crates/cli/src/lib.rs:9117:5: replace newest_complete -> Result<Option<crate::results::Record>, crate::results::Refusal> with Ok(None)
crates/cli/src/lib.rs:17810:5: replace screen_range_for_attempt -> String with "xyzzy".into()
crates/cli/src/fixed_tail.rs:377:28: replace > with == in heal_torn_header
crates/cli/src/and_checkpoint.rs:424:5: replace encode_boundary -> Result<Vec<u8>, String> with Ok(vec![])
crates/cli/src/execution_capability.rs:1251:5: replace validate_population_capabilities -> Result<(u64, u64), ExecutionCapabilityRefusal> with Ok((1, 1))
crates/cli/src/live.rs:229:11: replace >= with < in clears_bar
crates/cli/src/pool.rs:1343:5: replace ratio_cell -> String with String::new()
crates/cli/src/result_set.rs:685:5: replace hash_range -> Result<u64, Refusal> with Ok(1)
crates/cli/src/stored.rs:616:9: replace CashCloses::unverified_reason -> Option<&str> with Some("xyzzy")
crates/cli/src/selection_v6_read.rs:467:20: replace + with * in decode_block
crates/greeks/src/solver.rs:424:9: replace Checked::bracket -> (f64, f64) with (0.0, 1.0)
crates/indicators/src/pattern.rs:813:27: replace < with <= in Patterns::bits
crates/lake/src/reader.rs:405:34: replace != with == in LakeFile::read_row_group
crates/pull/src/fno.rs:187:5: replace names -> Result<Vec<String>, FnoError> with Ok(vec!["xyzzy".into()])
crates/pull/src/ingest.rs:2509:23: replace match guard report.clean() with true in check_day
crates/pull/src/ssm.rs:432:24: replace > with < in AwsIdentity::from_credentials_file
crates/runner/src/bootstrap.rs:526:5: replace white_null_is_a_point_mass -> bool with false
crates/runner/src/exit_grid_policy.rs:2905:16: delete ! in ResolvedExitGridV1::replay_selected_universe_with
crates/runner/src/outcome.rs:814:24: replace > with < in BlockExtremes::of
crates/runner/src/significance.rs:338:16: delete ! in clears_bonferroni
crates/runner/src/significance.rs:499:29: replace / with % in ln_gamma
crates/store/src/catalog.rs:376:21: replace match guard why.kind() == std::io::ErrorKind::NotFound with false in bars_present
crates/store/src/format.rs:727:43: replace && with || in <impl Row for Greek>::is_sane
crates/store/src/time_index.rs:485:5: replace locate -> Result<u64, Fault> with Ok(1)
crates/vocab/src/tolerance.rs:315:18: replace + with -
