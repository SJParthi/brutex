# deep-BASE-COV-LIBS-LSA-d096f65942d4-set.txt.md

BASE-COV-LIBS-LSA's D3 set file at code tip `d096f65942d4d3cdb98589e01842dff00c945b39` (recipe v4 step 2, SET OVERRIDE): every tracked `.rs` file of the ten crates whole for coverage (362 files), and the mutants lines of the D2 set of `f01d7a0e4...d096f65942d4` in these crates (1,334 lines, 49 of them whole added files) for the deferred background pass. The mutants lines come from a node scanner of the session's (recipe step 2: the outermost enclosing fn item from its first doc or attribute line to its closing brace; a + line outside every fn as its own line; a deletion-only hunk's enclosing fn); run on LS4's tip and crates it prints LS4's 171 lines exactly. One fenced block below holds the set byte for byte; sha256 of the fenced content: `d405c33e92288c6b4cfdff545c30bfcaab04679234ca2f5b8bed6107ef6e1fa4`.

`````text
reading whole:core costs engine greeks indicators lake runner store telemetry vocab
coverage crates/core/benches/ratio.rs whole
coverage crates/core/src/alias.rs whole
coverage crates/core/src/alias_cover_tests.rs whole
coverage crates/core/src/blake3.rs whole
coverage crates/core/src/error.rs whole
coverage crates/core/src/instrument.rs whole
coverage crates/core/src/isin.rs whole
coverage crates/core/src/knob.rs whole
coverage crates/core/src/lib.rs whole
coverage crates/core/src/price.rs whole
coverage crates/core/src/secret_shape.rs whole
coverage crates/core/src/symbol.rs whole
coverage crates/core/src/universe.rs whole
coverage crates/core/src/vendor.rs whole
coverage crates/core/tests/banned_lists.rs whole
coverage crates/core/tests/bound_probe.rs whole
coverage crates/core/tests/claim_reader.rs whole
coverage crates/core/tests/cost_invariants.rs whole
coverage crates/core/tests/decision_citations.rs whole
coverage crates/core/tests/findings.rs whole
coverage crates/core/tests/graph.rs whole
coverage crates/core/tests/lint.rs whole
coverage crates/core/tests/zerodha_index.rs whole
coverage crates/costs/benches/ratio.rs whole
coverage crates/costs/benches/stack.rs whole
coverage crates/costs/src/charge_error.rs whole
coverage crates/costs/src/dated.rs whole
coverage crates/costs/src/day.rs whole
coverage crates/costs/src/dhan_calc.rs whole
coverage crates/costs/src/error.rs whole
coverage crates/costs/src/exact.rs whole
coverage crates/costs/src/expiry.rs whole
coverage crates/costs/src/fill.rs whole
coverage crates/costs/src/ledger.rs whole
coverage crates/costs/src/lib.rs whole
coverage crates/costs/src/lot.rs whole
coverage crates/costs/src/lot_table_tests.rs whole
coverage crates/costs/src/money.rs whole
coverage crates/costs/src/moneyness.rs whole
coverage crates/costs/src/rate.rs whole
coverage crates/costs/src/regime.rs whole
coverage crates/costs/src/schedule.rs whole
coverage crates/costs/src/schedule_cover_tests.rs whole
coverage crates/costs/src/scope.rs whole
coverage crates/costs/src/stack.rs whole
coverage crates/costs/src/stack_cover_tests.rs whole
coverage crates/costs/src/strike.rs whole
coverage crates/costs/src/trip.rs whole
coverage crates/costs/src/venue.rs whole
coverage crates/costs/tests/adversarial.rs whole
coverage crates/costs/tests/dhan_calculator.rs whole
coverage crates/costs/tests/fa73061.rs whole
coverage crates/engine/benches/ratio.rs whole
coverage crates/engine/src/column.rs whole
coverage crates/engine/src/keep.rs whole
coverage crates/engine/src/lib.rs whole
coverage crates/engine/src/manifest.rs whole
coverage crates/engine/src/refusal_cover_tests.rs whole
coverage crates/engine/src/resume.rs whole
coverage crates/engine/tests/adversarial.rs whole
coverage crates/engine/tests/join_probe_screen.rs whole
coverage crates/engine/tests/production_callers.rs whole
coverage crates/engine/tests/resume_readiness.rs whole
coverage crates/engine/tests/sweep_readiness_oracle.rs whole
coverage crates/greeks/benches/ratio.rs whole
coverage crates/greeks/src/bsm.rs whole
coverage crates/greeks/src/error.rs whole
coverage crates/greeks/src/lib.rs whole
coverage crates/greeks/src/moneyness.rs whole
coverage crates/greeks/src/normal.rs whole
coverage crates/greeks/src/solver.rs whole
coverage crates/greeks/tests/standalone.rs whole
coverage crates/greeks/tests/vendor_anchor.rs whole
coverage crates/indicators/benches/ratio.rs whole
coverage crates/indicators/src/anchored.rs whole
coverage crates/indicators/src/column.rs whole
coverage crates/indicators/src/column_cover_tests.rs whole
coverage crates/indicators/src/daily.rs whole
coverage crates/indicators/src/evaluator.rs whole
coverage crates/indicators/src/evaluator_cover_tests.rs whole
coverage crates/indicators/src/fib.rs whole
coverage crates/indicators/src/gap.rs whole
coverage crates/indicators/src/gap_cover_tests.rs whole
coverage crates/indicators/src/lib.rs whole
coverage crates/indicators/src/orb.rs whole
coverage crates/indicators/src/orb_cover_tests.rs whole
coverage crates/indicators/src/pattern.rs whole
coverage crates/indicators/src/session.rs whole
coverage crates/indicators/src/session_cover_tests.rs whole
coverage crates/indicators/src/trend.rs whole
coverage crates/indicators/src/trend_cover_tests.rs whole
coverage crates/indicators/src/vwap.rs whole
coverage crates/indicators/src/vwap_cover_tests.rs whole
coverage crates/indicators/tests/audit_readiness.rs whole
coverage crates/indicators/tests/crossing_known_readiness.rs whole
coverage crates/indicators/tests/current_day_fib_known_readiness.rs whole
coverage crates/indicators/tests/evaluator_position_count.rs whole
coverage crates/indicators/tests/exact_minute_orb_gap_readiness.rs whole
coverage crates/indicators/tests/extremes.rs whole
coverage crates/indicators/tests/fib_known_readiness.rs whole
coverage crates/indicators/tests/fib_rung_rounding.rs whole
coverage crates/indicators/tests/gap_known_readiness.rs whole
coverage crates/indicators/tests/invariants.rs whole
coverage crates/indicators/tests/module_doc_counts.rs whole
coverage crates/indicators/tests/orb_known_readiness.rs whole
coverage crates/indicators/tests/session_from_outside.rs whole
coverage crates/indicators/tests/session_known_readiness.rs whole
coverage crates/indicators/tests/shared_core_doc.rs whole
coverage crates/indicators/tests/sweep_predicate_readiness.rs whole
coverage crates/indicators/tests/time_known_readiness.rs whole
coverage crates/indicators/tests/trend_known_readiness.rs whole
coverage crates/indicators/tests/withheld_cut_fold.rs whole
coverage crates/indicators/tests/withheld_fold.rs whole
coverage crates/lake/benches/ratio.rs whole
coverage crates/lake/src/bar.rs whole
coverage crates/lake/src/batch.rs whole
coverage crates/lake/src/contract.rs whole
coverage crates/lake/src/error.rs whole
coverage crates/lake/src/footer.rs whole
coverage crates/lake/src/footer_tests.rs whole
coverage crates/lake/src/lib.rs whole
coverage crates/lake/src/page.rs whole
coverage crates/lake/src/reader.rs whole
coverage crates/lake/src/reader_cover_tests.rs whole
coverage crates/lake/src/schema.rs whole
coverage crates/lake/src/schema_cover_tests.rs whole
coverage crates/lake/src/zstd_v082_frames.rs whole
coverage crates/lake/tests/events.rs whole
coverage crates/lake/tests/real_lake.rs whole
coverage crates/lake/tests/real_lake_regression.rs whole
coverage crates/lake/tests/refusals.rs whole
coverage crates/lake/tests/synthetic.rs whole
coverage crates/runner/benches/ratio.rs whole
coverage crates/runner/src/admission.rs whole
coverage crates/runner/src/admission_reader_cover_tests.rs whole
coverage crates/runner/src/admission_statistics_cover_tests.rs whole
coverage crates/runner/src/align.rs whole
coverage crates/runner/src/audit.rs whole
coverage crates/runner/src/bootstrap.rs whole
coverage crates/runner/src/bootstrap_family_pass.rs whole
coverage crates/runner/src/bootstrap_family_pass_tests.rs whole
coverage crates/runner/src/bootstrap_zero_v2.rs whole
coverage crates/runner/src/bootstrap_zero_v2_tests.rs whole
coverage crates/runner/src/bound.rs whole
coverage crates/runner/src/closed.rs whole
coverage crates/runner/src/excursion.rs whole
coverage crates/runner/src/excursion_clock_tests.rs whole
coverage crates/runner/src/exit_grid_policy.rs whole
coverage crates/runner/src/exit_grid_policy_cover_tests.rs whole
coverage crates/runner/src/expression.rs whole
coverage crates/runner/src/expression_execution.rs whole
coverage crates/runner/src/expression_execution_tests.rs whole
coverage crates/runner/src/expression_oos.rs whole
coverage crates/runner/src/expression_oos_tests.rs whole
coverage crates/runner/src/expression_validation.rs whole
coverage crates/runner/src/expression_validation_codec.rs whole
coverage crates/runner/src/expression_validation_tests.rs whole
coverage crates/runner/src/family_allocation_v1.rs whole
coverage crates/runner/src/family_allocation_v1_tests.rs whole
coverage crates/runner/src/grid.rs whole
coverage crates/runner/src/grid_cover_tests.rs whole
coverage crates/runner/src/identity.rs whole
coverage crates/runner/src/lib.rs whole
coverage crates/runner/src/outcome.rs whole
coverage crates/runner/src/pbo.rs whole
coverage crates/runner/src/portfolio.rs whole
coverage crates/runner/src/rank.rs whole
coverage crates/runner/src/replay_mask.rs whole
coverage crates/runner/src/report.rs whole
coverage crates/runner/src/report_cover_tests.rs whole
coverage crates/runner/src/resample.rs whole
coverage crates/runner/src/research_admission_projection.rs whole
coverage crates/runner/src/research_admission_projection_codec.rs whole
coverage crates/runner/src/research_exit_grid.rs whole
coverage crates/runner/src/research_exit_grid_tests.rs whole
coverage crates/runner/src/research_family.rs whole
coverage crates/runner/src/reservation_refusal.rs whole
coverage crates/runner/src/resolved_grid_view.rs whole
coverage crates/runner/src/search_allocation_v1.rs whole
coverage crates/runner/src/search_allocation_v1_tests.rs whole
coverage crates/runner/src/signal_candle_stop.rs whole
coverage crates/runner/src/signal_candle_stop_tests.rs whole
coverage crates/runner/src/significance.rs whole
coverage crates/runner/src/significance_cover_tests.rs whole
coverage crates/runner/src/split.rs whole
coverage crates/runner/src/synthetic.rs whole
coverage crates/runner/src/topn.rs whole
coverage crates/runner/src/trade.rs whole
coverage crates/runner/src/trade_clock_tests.rs whole
coverage crates/runner/src/validate.rs whole
coverage crates/runner/src/validate_cover_tests.rs whole
coverage crates/runner/tests/auto_probe_found_nothing.rs whole
coverage crates/runner/tests/auto_probe_reporter_refusal.rs whole
coverage crates/runner/tests/auto_reporting.rs whole
coverage crates/runner/tests/base_cov_admission_words.rs whole
coverage crates/runner/tests/checkpointed_rank_refusals.rs whole
coverage crates/runner/tests/d_0694_record.rs whole
coverage crates/runner/tests/end_to_end.rs whole
coverage crates/runner/tests/expression_pricing.rs whole
coverage crates/runner/tests/fib_rung_sweep.rs whole
coverage crates/runner/tests/fixed_deadline_readiness.rs whole
coverage crates/runner/tests/foreign_slice_deadline_readiness.rs whole
coverage crates/runner/tests/hole_after_exit.rs whole
coverage crates/runner/tests/join_answer_is_unchanged.rs whole
coverage crates/runner/tests/population_door_halt.rs whole
coverage crates/runner/tests/prefix_only_cadence.rs whole
coverage crates/runner/tests/research_family_readiness.rs whole
coverage crates/runner/tests/unwired_validation_record.rs whole
coverage crates/store/benches/ratio.rs whole
coverage crates/store/benches/ratio/months.rs whole
coverage crates/store/benches/ratio/quarantine.rs whole
coverage crates/store/benches/ratio/safe_files.rs whole
coverage crates/store/src/block.rs whole
coverage crates/store/src/catalog.rs whole
coverage crates/store/src/catalog_cover_tests.rs whole
coverage crates/store/src/catalog_tests.rs whole
coverage crates/store/src/checksum_audit.rs whole
coverage crates/store/src/checksum_audit_cover_tests.rs whole
coverage crates/store/src/checksum_audit_tests.rs whole
coverage crates/store/src/crc.rs whole
coverage crates/store/src/emits.rs whole
coverage crates/store/src/file.rs whole
coverage crates/store/src/file_cover_tests.rs whole
coverage crates/store/src/file_fault_tests.rs whole
coverage crates/store/src/file_index_cover_tests.rs whole
coverage crates/store/src/file_sealed.rs whole
coverage crates/store/src/file_sealed_cover_tests.rs whole
coverage crates/store/src/file_sealed_tests.rs whole
coverage crates/store/src/file_write_door_tests.rs whole
coverage crates/store/src/flock.rs whole
coverage crates/store/src/flock_cover_tests.rs whole
coverage crates/store/src/flock_patient_tests.rs whole
coverage crates/store/src/flock_tests.rs whole
coverage crates/store/src/format.rs whole
coverage crates/store/src/generation.rs whole
coverage crates/store/src/generation_cover_tests.rs whole
coverage crates/store/src/generation_door_cover_tests.rs whole
coverage crates/store/src/generation_tests.rs whole
coverage crates/store/src/header.rs whole
coverage crates/store/src/header_audit_tests.rs whole
coverage crates/store/src/layout.rs whole
coverage crates/store/src/lib.rs whole
coverage crates/store/src/month_summary.rs whole
coverage crates/store/src/month_summary_cover_tests.rs whole
coverage crates/store/src/month_summary_decode_cover_tests.rs whole
coverage crates/store/src/month_summary_tests.rs whole
coverage crates/store/src/month_summary_writer.rs whole
coverage crates/store/src/month_summary_writer_cover_tests.rs whole
coverage crates/store/src/open_flags.rs whole
coverage crates/store/src/path.rs whole
coverage crates/store/src/provenance.rs whole
coverage crates/store/src/provenance_cover_tests.rs whole
coverage crates/store/src/quarantine.rs whole
coverage crates/store/src/quarantine_cover_tests.rs whole
coverage crates/store/src/quarantine_tests.rs whole
coverage crates/store/src/repair.rs whole
coverage crates/store/src/repair_cover_tests.rs whole
coverage crates/store/src/repair_door_cover_tests.rs whole
coverage crates/store/src/repair_tests.rs whole
coverage crates/store/src/safe_open.rs whole
coverage crates/store/src/safe_open_tests.rs whole
coverage crates/store/src/sealed.rs whole
coverage crates/store/src/sealed_codec.rs whole
coverage crates/store/src/sealed_codec_tests.rs whole
coverage crates/store/src/sealed_fold_cover_tests.rs whole
coverage crates/store/src/sealed_tests.rs whole
coverage crates/store/src/settings.rs whole
coverage crates/store/src/settings_cover_tests.rs whole
coverage crates/store/src/settings_door_cover_tests.rs whole
coverage crates/store/src/settings_tests.rs whole
coverage crates/store/src/sidecar_checksum_tests.rs whole
coverage crates/store/src/small_month_tests.rs whole
coverage crates/store/src/sort_index.rs whole
coverage crates/store/src/sort_index_cover_tests.rs whole
coverage crates/store/src/sort_index_tests.rs whole
coverage crates/store/src/sort_index_writer.rs whole
coverage crates/store/src/sort_index_writer_tests.rs whole
coverage crates/store/src/staging.rs whole
coverage crates/store/src/staging_cover_tests.rs whole
coverage crates/store/src/staging_tests.rs whole
coverage crates/store/src/time_index.rs whole
coverage crates/store/src/verdict.rs whole
coverage crates/store/src/verdict_tests.rs whole
coverage crates/store/tests/admission.rs whole
coverage crates/store/tests/base_cov_words.rs whole
coverage crates/store/tests/bisect_cost.rs whole
coverage crates/store/tests/catalog.rs whole
coverage crates/store/tests/cited_commits.rs whole
coverage crates/store/tests/cloud_mutants_store.rs whole
coverage crates/store/tests/cold_read.rs whole
coverage crates/store/tests/docs.rs whole
coverage crates/store/tests/durability.rs whole
coverage crates/store/tests/fault.rs whole
coverage crates/store/tests/fifo.rs whole
coverage crates/store/tests/generation.rs whole
coverage crates/store/tests/geometry.rs whole
coverage crates/store/tests/libm_key.rs whole
coverage crates/store/tests/misfiled.rs whole
coverage crates/store/tests/month_repair.rs whole
coverage crates/store/tests/month_summary_log.rs whole
coverage crates/store/tests/open_flags.rs whole
coverage crates/store/tests/provenance.rs whole
coverage crates/store/tests/repair.rs whole
coverage crates/store/tests/roundtrip.rs whole
coverage crates/store/tests/seal_buffer.rs whole
coverage crates/store/tests/sealed_codec_alloc.rs whole
coverage crates/store/tests/sealed_codec_samples.rs whole
coverage crates/store/tests/sealed_door.rs whole
coverage crates/store/tests/sealed_samples.rs whole
coverage crates/store/tests/sealed_write_door.rs whole
coverage crates/store/tests/slot_audit.rs whole
coverage crates/store/tests/slot_audit_sealed.rs whole
coverage crates/store/tests/slot_damaged_log.rs whole
coverage crates/store/tests/sort_index.rs whole
coverage crates/store/tests/sort_index_log.rs whole
coverage crates/store/tests/support/mod.rs whole
coverage crates/store/tests/tail_proof.rs whole
coverage crates/store/tests/tix.rs whole
coverage crates/store/tests/tix_latency.rs whole
coverage crates/store/tests/tix_rebuilder.rs whole
coverage crates/store/tests/tix_small_month.rs whole
coverage crates/store/tests/tix_warn_once.rs whole
coverage crates/store/tests/unit.rs whole
coverage crates/store/tests/verdict.rs whole
coverage crates/store/tests/write.rs whole
coverage crates/telemetry/benches/ratio.rs whole
coverage crates/telemetry/src/clock.rs whole
coverage crates/telemetry/src/encode.rs whole
coverage crates/telemetry/src/event.rs whole
coverage crates/telemetry/src/json.rs whole
coverage crates/telemetry/src/level.rs whole
coverage crates/telemetry/src/lib.rs whole
coverage crates/telemetry/src/record.rs whole
coverage crates/telemetry/src/run_dirs.rs whole
coverage crates/telemetry/src/sink.rs whole
coverage crates/telemetry/src/sink_recovery_tests.rs whole
coverage crates/telemetry/src/state_events.rs whole
coverage crates/telemetry/src/state_events_tests.rs whole
coverage crates/telemetry/src/tail.rs whole
coverage crates/telemetry/src/value.rs whole
coverage crates/telemetry/tests/cloud_mutants_sink.rs whole
coverage crates/telemetry/tests/global_absent.rs whole
coverage crates/vocab/benches/ratio.rs whole
coverage crates/vocab/src/error.rs whole
coverage crates/vocab/src/expression.rs whole
coverage crates/vocab/src/expression_search.rs whole
coverage crates/vocab/src/implication.rs whole
coverage crates/vocab/src/lib.rs whole
coverage crates/vocab/src/mask.rs whole
coverage crates/vocab/src/table.rs whole
coverage crates/vocab/src/tolerance.rs whole
coverage crates/vocab/tests/expression.rs whole
coverage crates/vocab/tests/expression_search.rs whole
coverage crates/vocab/tests/expression_search_readiness.rs whole
coverage crates/vocab/tests/implication_pointers.rs whole
coverage crates/vocab/tests/mask.rs whole
coverage crates/vocab/tests/no_float.rs whole
coverage crates/vocab/tests/no_py_language.rs whole
coverage crates/vocab/tests/no_runtime_spawn.rs whole
coverage crates/vocab/tests/stale_claims.rs whole
coverage crates/vocab/tests/table.rs whole
coverage crates/vocab/tests/workspace_is_rust.rs whole
mutants crates/core/src/alias.rs line 1080
mutants crates/core/src/alias.rs line 1901
mutants crates/core/src/alias.rs line 1902
mutants crates/core/src/alias.rs line 1903
mutants crates/core/src/alias.rs line 1904
mutants crates/core/src/alias.rs the_const_row_builders_give_the_rows_the_tables_hold 1046-1079
mutants crates/core/src/alias.rs the_shipped_lineage_has_no_cycle_and_no_duplicate 1494-1524
mutants crates/core/src/alias_cover_tests.rs whole
mutants crates/core/src/secret_shape.rs a_hyphen_or_underscore_after_a_field_is_part_of_its_name 236-243
mutants crates/core/src/secret_shape.rs line 235
mutants crates/core/src/secret_shape.rs line 244
mutants crates/core/src/secret_shape.rs the_mixed_test_wants_a_letter_and_a_digit_in_one_run 219-234
mutants crates/core/src/universe.rs worst_probe 4815-4848
mutants crates/core/src/vendor.rs the_series_tables_probe_in_bounded_time 3037-3112
mutants crates/core/tests/cost_invariants.rs bench_sources 95-103
mutants crates/core/tests/cost_invariants.rs bench_sources_in 105-139
mutants crates/core/tests/cost_invariants.rs declared_rows 228-250
mutants crates/core/tests/cost_invariants.rs every_bench_source 141-156
mutants crates/core/tests/cost_invariants.rs every_bench_source_in 158-191
mutants crates/core/tests/cost_invariants.rs line 104
mutants crates/core/tests/cost_invariants.rs line 157
mutants crates/core/tests/cost_invariants.rs line 422
mutants crates/core/tests/cost_invariants.rs line 431
mutants crates/core/tests/cost_invariants.rs line 465
mutants crates/core/tests/cost_invariants.rs line 70
mutants crates/core/tests/cost_invariants.rs scratch 423-430
mutants crates/core/tests/cost_invariants.rs the_bench_walks_pass_over_what_is_not_a_crates_bench 432-464
mutants crates/core/tests/cost_invariants.rs the_id_row_and_reach_readers_hold_at_their_edges 466-507
mutants crates/core/tests/decision_citations.rs every_cited_decision_heads_an_entry_or_is_named_pending 288-333
mutants crates/core/tests/decision_citations.rs line 267
mutants crates/core/tests/decision_citations.rs line 459
mutants crates/core/tests/decision_citations.rs named 258-266
mutants crates/core/tests/decision_citations.rs the_reader_resolves_entries_and_pending_names_and_nothing_else 335-458
mutants crates/core/tests/decision_citations.rs the_walk_skips_build_output_and_hidden_folders 460-513
mutants crates/core/tests/findings.rs every_killed_finding_carries_its_reason 341-374
mutants crates/core/tests/findings.rs every_named_commit_exists 718-734
mutants crates/core/tests/findings.rs every_named_commit_is_in_this_branchs_history 795-821
mutants crates/core/tests/findings.rs every_named_commit_is_on_main 883-904
mutants crates/core/tests/findings.rs every_refuted_finding_carries_its_reason 440-453
mutants crates/core/tests/findings.rs fixed_commits 710-716
mutants crates/core/tests/findings.rs fixed_sha 699-708
mutants crates/core/tests/findings.rs line 45
mutants crates/core/tests/findings.rs line 454
mutants crates/core/tests/findings.rs line 570
mutants crates/core/tests/findings.rs line 617
mutants crates/core/tests/findings.rs line 626
mutants crates/core/tests/findings.rs line 689
mutants crates/core/tests/findings.rs line 698
mutants crates/core/tests/findings.rs line 709
mutants crates/core/tests/findings.rs line 961
mutants crates/core/tests/findings.rs named_commits_exist 736-793
mutants crates/core/tests/findings.rs named_commits_in_history 823-881
mutants crates/core/tests/findings.rs named_commits_on_main 906-960
mutants crates/core/tests/findings.rs only_a_tree_with_no_git_dir_skips_the_history 511-569
mutants crates/core/tests/findings.rs refuted_without_reason 455-464
mutants crates/core/tests/findings.rs repository_root 690-697
mutants crates/core/tests/findings.rs row 618-625
mutants crates/core/tests/findings.rs the_empty_root_is_asked_in_the_working_directory_unfenced 571-616
mutants crates/core/tests/findings.rs the_history_checks_skip_a_bare_tree_and_the_readers_hold_at_their_edges 627-688
mutants crates/core/tests/findings.rs where_git_cannot_run_the_history_is_skipped_by_name 962-1004
mutants crates/core/tests/graph.rs line 872
mutants crates/core/tests/graph.rs the_key_and_inline_table_readers_hold_at_shapes_no_manifest_writes 873-928
mutants crates/core/tests/lint.rs a_float_off_the_boundary_is_named_and_one_on_its_way_is_not 227-251
mutants crates/core/tests/lint.rs floats_off_the_boundary 185-225
mutants crates/core/tests/lint.rs line 184
mutants crates/core/tests/lint.rs line 226
mutants crates/core/tests/lint.rs no_float_in_price 83-183
mutants crates/costs/src/dated.rs str_eq_answers_exactly_as_the_operator_would 637-661
mutants crates/costs/src/ledger.rs line 1705
mutants crates/costs/src/ledger.rs line 778
mutants crates/costs/src/ledger.rs line 779
mutants crates/costs/src/ledger.rs neither_type_has_a_default 771-777
mutants crates/costs/src/ledger.rs the_no_default_witness_is_the_every_type_item_for_both_checked_types 1706-1717
mutants crates/costs/src/ledger.rs witness 756-763
mutants crates/costs/src/lot.rs compiled_rows 1042-1098
mutants crates/costs/src/lot.rs from_rows 900-967
mutants crates/costs/src/lot_table_tests.rs each_sourced_row_carries_its_own_circulars_digest_and_the_row_cap_is_exact 507-558
mutants crates/costs/src/lot_table_tests.rs line 341
mutants crates/costs/src/lot_table_tests.rs line 379
mutants crates/costs/src/lot_table_tests.rs line 408
mutants crates/costs/src/lot_table_tests.rs line 506
mutants crates/costs/src/lot_table_tests.rs picks 342-347
mutants crates/costs/src/lot_table_tests.rs the_lookup_timer_picks_the_named_samples_and_times_a_table 349-378
mutants crates/costs/src/lot_table_tests.rs the_wire_words_the_absence_text_and_the_grouped_rows_hold_at_their_extremes 409-505
mutants crates/costs/src/lot_table_tests.rs time_lookups 322-340
mutants crates/costs/src/moneyness.rs the_offset_matches_a_nearest_multiple_search_across_four_whole_steps 531-560
mutants crates/costs/src/regime.rs the_validation_rejects_every_way_a_table_can_be_wrong 969-1074
mutants crates/costs/src/schedule.rs line 1830
mutants crates/costs/src/schedule.rs line 1831
mutants crates/costs/src/schedule.rs line 1832
mutants crates/costs/src/schedule.rs line 1833
mutants crates/costs/src/schedule_cover_tests.rs whole
mutants crates/costs/src/stack.rs line 2961
mutants crates/costs/src/stack.rs line 3006
mutants crates/costs/src/stack.rs line 3007
mutants crates/costs/src/stack.rs line 3008
mutants crates/costs/src/stack.rs line 3009
mutants crates/costs/src/stack.rs rounding_to_the_rupee_holds_at_both_ends_of_the_type 2962-3004
mutants crates/costs/src/stack_cover_tests.rs whole
mutants crates/costs/src/trip.rs a_printed_extreme_sell_below_zero_is_refused_and_does_not_overflow 3226-3266
mutants crates/costs/tests/adversarial.rs a_larger_notional_is_never_charged_less_than_a_smaller_one 221-280
mutants crates/costs/tests/adversarial.rs every_extreme_notional_against_every_shipped_rate_holds_the_money_invariants 82-219
mutants crates/costs/tests/adversarial.rs rounding_the_top_of_the_type_up_to_a_rupee_refuses_rather_than_wraps 282-329
mutants crates/costs/tests/dhan_calculator.rs a_case_with_no_rate_card_is_refused_by_every_precise_helper 1223-1284
mutants crates/costs/tests/dhan_calculator.rs every_golden_case_has_a_precise_figure_or_a_named_refusal 740-763
mutants crates/costs/tests/dhan_calculator.rs line 1222
mutants crates/costs/tests/dhan_calculator.rs line 739
mutants crates/costs/tests/dhan_calculator.rs shown 728-738
mutants crates/costs/tests/fa73061.rs line 619
mutants crates/costs/tests/fa73061.rs the_timing_helpers_pick_the_named_samples_and_call_every_operation 620-650
mutants crates/engine/src/column.rs vertical_reference 286-321
mutants crates/engine/src/keep.rs line 617
mutants crates/engine/src/keep.rs offer 353-407
mutants crates/engine/src/keep.rs the_heap_oracle_refuses_a_broken_heap 589-616
mutants crates/engine/src/lib.rs a_checkpoint_observer_gets_one_terminal_boundary_and_no_later_level 5940-5977
mutants crates/engine/src/lib.rs actual_join_batches_fill_before_handoff_and_keep_budget_tails 5635-5684
mutants crates/engine/src/lib.rs line 2518
mutants crates/engine/src/lib.rs line 2519
mutants crates/engine/src/lib.rs line 2523
mutants crates/engine/src/lib.rs line 2524
mutants crates/engine/src/lib.rs line 2525
mutants crates/engine/src/lib.rs line 2526
mutants crates/engine/src/lib.rs line 3969
mutants crates/engine/src/lib.rs reserved 204-217
mutants crates/engine/src/lib.rs the_bench_primitives_agree_with_the_level_they_stand_for 4172-4210
mutants crates/engine/src/lib.rs the_join_emits_exactly_one_candidate_per_pair 2945-3042
mutants crates/engine/src/lib.rs the_manifest_reader_holds_at_the_edges_of_its_string_grammar 3922-3968
mutants crates/engine/src/lib.rs the_prefix_join_finds_exactly_what_an_exhaustive_join_would 5166-5238
mutants crates/engine/src/lib.rs the_reporter_fires_once_per_level_and_the_halted_one_is_not_skipped 5240-5360
mutants crates/engine/src/lib.rs the_sweep_counts_support_against_the_fixed_width_column 4719-4762
mutants crates/engine/src/lib.rs the_walk_has_exactly_three_early_exits_and_each_is_a_budget 4386-4581
mutants crates/engine/src/refusal_cover_tests.rs whole
mutants crates/engine/tests/adversarial.rs every_one_bit_fixture_holds_one_live_bit 366-389
mutants crates/engine/tests/adversarial.rs line 365
mutants crates/engine/tests/adversarial.rs one 72-87
mutants crates/engine/tests/join_probe_screen.rs whole
mutants crates/engine/tests/production_callers.rs an_item_after_the_first_gate_without_its_own_is_named 328-378
mutants crates/engine/tests/production_callers.rs line 184
mutants crates/engine/tests/production_callers.rs line 327
mutants crates/engine/tests/production_callers.rs line 379
mutants crates/engine/tests/production_callers.rs line 59
mutants crates/engine/tests/production_callers.rs nothing_after_an_engine_files_first_test_gate_ships 163-183
mutants crates/engine/tests/production_callers.rs rust_files_under 44-58
mutants crates/engine/tests/production_callers.rs sources 60-81
mutants crates/engine/tests/production_callers.rs the_source_walk_reads_rust_at_every_depth_and_nothing_else 380-402
mutants crates/engine/tests/production_callers.rs ungated_after_the_first_gate 185-223
mutants crates/engine/tests/resume_readiness.rs line 1062
mutants crates/engine/tests/resume_readiness.rs line 1088
mutants crates/engine/tests/resume_readiness.rs line 5
mutants crates/engine/tests/resume_readiness.rs line 6
mutants crates/engine/tests/resume_readiness.rs line 7
mutants crates/engine/tests/resume_readiness.rs resumed_identity_column_offers_and_every_configuration_term_must_match 256-307
mutants crates/engine/tests/resume_readiness.rs the_failing_wrapper_forwards_a_flush 1089-1104
mutants crates/engine/tests/resume_readiness.rs the_oracle_combines_only_positions_a_ladder_can_keep 1063-1087
mutants crates/indicators/src/anchored.rs a_closing_minute_on_the_next_ist_day_is_refused_even_when_it_exists 1488-1534
mutants crates/indicators/src/anchored.rs a_reference_reads_back_its_bar_and_the_census_refuses_each_imbalance 2865-2935
mutants crates/indicators/src/anchored.rs a_short_session_closes_at_the_callers_close_for_that_day 2341-2421
mutants crates/indicators/src/anchored.rs line 1535
mutants crates/indicators/src/anchored.rs line 2050
mutants crates/indicators/src/anchored.rs line 2864
mutants crates/indicators/src/anchored.rs mixed_refused_and_accepted_bars_match_the_pre_fix_step_exactly 2687-2818
mutants crates/indicators/src/anchored.rs overlay_exact_minute 812-922
mutants crates/indicators/src/anchored.rs the_resampler_gives_a_fully_punched_day_no_bar 2034-2049
mutants crates/indicators/src/column.rs a_map_that_steps_back_or_past_its_series_refuses 2649-2717
mutants crates/indicators/src/column.rs a_zero_priced_bar_is_refused_and_leaves_the_masks_around_it_untouched 2065-2151
mutants crates/indicators/src/column.rs acceptance_bitmap_records_a_duplicate_timestamp_in_constant_time_shape 2263-2287
mutants crates/indicators/src/column.rs checked_reprojection_records_execution_accumulator_refusal 2289-2350
mutants crates/indicators/src/column.rs clearing_before_a_boundary_blanks_the_training_rows_and_renumbers_nothing 1512-1605
mutants crates/indicators/src/column.rs column_across_a_refusal 1460-1510
mutants crates/indicators/src/column.rs line 2719
mutants crates/indicators/src/column.rs line 2720
mutants crates/indicators/src/column.rs line 2721
mutants crates/indicators/src/column.rs line 2722
mutants crates/indicators/src/column.rs sources_maps_a_column_position_to_its_bar_even_across_a_refusal 1397-1458
mutants crates/indicators/src/column.rs two_signals_resolving_to_one_fill_bar_produce_one_row 2536-2613
mutants crates/indicators/src/column_cover_tests.rs whole
mutants crates/indicators/src/evaluator.rs a_non_regular_session_still_emits_its_own_bars 3053-3084
mutants crates/indicators/src/evaluator.rs canonical_v1_changes_for_every_evaluation_spec_field 1714-1804
mutants crates/indicators/src/evaluator.rs crossings_of 1095-1214
mutants crates/indicators/src/evaluator.rs line 3973
mutants crates/indicators/src/evaluator.rs line 3974
mutants crates/indicators/src/evaluator.rs line 3975
mutants crates/indicators/src/evaluator.rs line 3976
mutants crates/indicators/src/evaluator.rs stepped 878-1093
mutants crates/indicators/src/evaluator.rs the_nine_non_regular_days_are_the_charter_dates 3673-3774
mutants crates/indicators/src/evaluator.rs the_structure_direction_bits_are_not_swapped 3776-3847
mutants crates/indicators/src/evaluator.rs the_structure_in_force_is_reported_between_breaks_and_not_before_the_first 3594-3671
mutants crates/indicators/src/evaluator_cover_tests.rs whole
mutants crates/indicators/src/gap.rs line 1538
mutants crates/indicators/src/gap.rs line 1539
mutants crates/indicators/src/gap.rs line 1540
mutants crates/indicators/src/gap.rs line 1541
mutants crates/indicators/src/gap_cover_tests.rs whole
mutants crates/indicators/src/lib.rs the_session_day_floors_for_negative_stamps 879-900
mutants crates/indicators/src/lib.rs the_weekday_map_is_anchored_on_a_real_calendar 2009-2041
mutants crates/indicators/src/orb.rs line 1034
mutants crates/indicators/src/orb.rs line 1035
mutants crates/indicators/src/orb.rs line 1036
mutants crates/indicators/src/orb.rs line 1037
mutants crates/indicators/src/orb_cover_tests.rs whole
mutants crates/indicators/src/pattern.rs each_shape_misses_by_its_one_condition_and_fires_without_it 3261-3304
mutants crates/indicators/src/pattern.rs last_mask 3214-3231
mutants crates/indicators/src/pattern.rs line 3205
mutants crates/indicators/src/pattern.rs line 3206
mutants crates/indicators/src/pattern.rs line 3207
mutants crates/indicators/src/pattern.rs line 3208
mutants crates/indicators/src/pattern.rs line 3209
mutants crates/indicators/src/pattern.rs line 3210
mutants crates/indicators/src/pattern.rs line 3211
mutants crates/indicators/src/pattern.rs line 3212
mutants crates/indicators/src/pattern.rs line 3213
mutants crates/indicators/src/pattern.rs line 3232
mutants crates/indicators/src/pattern.rs line 3260
mutants crates/indicators/src/pattern.rs line 3305
mutants crates/indicators/src/pattern.rs the_known_ladder_grows_with_depth_and_stops_at_the_pattern_block 3233-3259
mutants crates/indicators/src/session.rs line 1333
mutants crates/indicators/src/session.rs line 1334
mutants crates/indicators/src/session.rs line 1335
mutants crates/indicators/src/session.rs line 1336
mutants crates/indicators/src/session_cover_tests.rs whole
mutants crates/indicators/src/trend.rs a_change_of_character_reaches_the_mask 1728-1778
mutants crates/indicators/src/trend.rs a_short_stop_tightens_holds_and_then_flips 2274-2337
mutants crates/indicators/src/trend.rs line 2729
mutants crates/indicators/src/trend.rs line 2730
mutants crates/indicators/src/trend.rs line 2731
mutants crates/indicators/src/trend.rs line 2732
mutants crates/indicators/src/trend_cover_tests.rs whole
mutants crates/indicators/src/vwap.rs an_impossible_state_is_refused_by_the_oracle_and_clamped_by_sigma 1760-1777
mutants crates/indicators/src/vwap.rs into_corrupt 118-129
mutants crates/indicators/src/vwap.rs line 117
mutants crates/indicators/src/vwap.rs line 130
mutants crates/indicators/src/vwap.rs line 131
mutants crates/indicators/src/vwap.rs line 1778
mutants crates/indicators/src/vwap.rs line 2002
mutants crates/indicators/src/vwap.rs line 2003
mutants crates/indicators/src/vwap.rs line 2004
mutants crates/indicators/src/vwap.rs line 2005
mutants crates/indicators/src/vwap.rs sigma_is_exact_where_the_remainder_squared_leaves_i128 1859-1893
mutants crates/indicators/src/vwap_cover_tests.rs whole
mutants crates/indicators/tests/crossing_known_readiness.rs every_crossing_and_ordinal_matches_a_last_definite_side_oracle_including_known_non_events 41-161
mutants crates/indicators/tests/current_day_fib_known_readiness.rs anchor 58-88
mutants crates/indicators/tests/current_day_fib_known_readiness.rs expected 90-114
mutants crates/indicators/tests/exact_minute_orb_gap_readiness.rs a_flat_opening_range_answers_its_exact_relations_and_leaves_near_unknown 496-521
mutants crates/indicators/tests/exact_minute_orb_gap_readiness.rs line 414
mutants crates/indicators/tests/exact_minute_orb_gap_readiness.rs line 495
mutants crates/indicators/tests/exact_minute_orb_gap_readiness.rs the_bridge_refuses_bad_lengths_empty_unaligned_and_repeated_minutes_and_a_short_signal 415-494
mutants crates/indicators/tests/fib_known_readiness.rs expected 108-130
mutants crates/indicators/tests/fib_known_readiness.rs level 91-106
mutants crates/indicators/tests/fib_known_readiness.rs public_column_retains_known_false_fibonacci_answers_for_negated_search 395-444
mutants crates/indicators/tests/fib_rung_rounding.rs closes 109-142
mutants crates/indicators/tests/gap_known_readiness.rs expected 54-141
mutants crates/indicators/tests/module_doc_counts.rs line 335
mutants crates/indicators/tests/module_doc_counts.rs the_header_parsers_read_what_they_should_and_refuse_the_rest 336-391
mutants crates/indicators/tests/orb_known_readiness.rs expected 60-99
mutants crates/indicators/tests/session_known_readiness.rs expected 58-183
mutants crates/indicators/tests/trend_known_readiness.rs a_close_between_a_lower_swing_high_and_a_higher_swing_low_breaks_the_newer 429-475
mutants crates/indicators/tests/trend_known_readiness.rs answer 64-111
mutants crates/indicators/tests/trend_known_readiness.rs confirm 113-139
mutants crates/indicators/tests/trend_known_readiness.rs line 428
mutants crates/indicators/tests/withheld_fold.rs splice_differences 119-165
mutants crates/lake/src/contract.rs line 768
mutants crates/lake/src/contract.rs parse_month 306-318
mutants crates/lake/src/contract.rs the_expiry_token_and_the_rendering_hold_at_their_edges 769-827
mutants crates/lake/src/page.rs a_frame_with_a_byte_after_its_last_block_is_refused_by_the_walk 1124-1132
mutants crates/lake/src/page.rs a_header_length_is_exactly_the_bytes_the_parse_consumed 1506-1542
mutants crates/lake/src/page.rs a_position_past_the_chunk_is_refused_and_one_at_its_end_is_done 1469-1504
mutants crates/lake/src/page.rs a_ruzstd_encoded_compressed_block_decodes_through_the_page_path 955-1018
mutants crates/lake/src/page.rs crafted_frame 1071-1093
mutants crates/lake/src/page.rs line 1094
mutants crates/lake/src/page.rs line 1123
mutants crates/lake/src/page.rs line 1133
mutants crates/lake/src/page.rs line 1461
mutants crates/lake/src/page.rs line 1462
mutants crates/lake/src/page.rs line 1463
mutants crates/lake/src/page.rs line 1464
mutants crates/lake/src/page.rs line 1465
mutants crates/lake/src/page.rs line 1466
mutants crates/lake/src/page.rs line 1467
mutants crates/lake/src/page.rs line 1468
mutants crates/lake/src/page.rs line 1505
mutants crates/lake/src/page.rs line 37
mutants crates/lake/src/page.rs read_header 387-403
mutants crates/lake/src/page.rs the_frame_walk_reads_every_descriptor_shape 1095-1122
mutants crates/lake/src/reader.rs a_column_chunk_running_past_the_end_of_the_file_is_refused_by_name 2120-2168
mutants crates/lake/src/reader.rs a_column_read_as_the_wrong_type_names_the_type_asked_for_and_the_type_held 1987-2045
mutants crates/lake/src/reader.rs a_negative_row_group_count_is_refused_with_the_number_the_file_gave 2047-2083
mutants crates/lake/src/reader.rs a_null_spot_reads_as_none_on_its_own_row_and_the_present_one_as_paisa 2176-2195
mutants crates/lake/src/reader.rs double 741-751
mutants crates/lake/src/reader.rs fno_file_with_provenance 1871-1879
mutants crates/lake/src/reader.rs fno_file_with_spot 2170-2174
mutants crates/lake/src/reader.rs fno_file_written 1881-1952
mutants crates/lake/src/reader.rs int32 731-739
mutants crates/lake/src/reader.rs int64 721-729
mutants crates/lake/src/reader.rs line 1880
mutants crates/lake/src/reader.rs line 1964
mutants crates/lake/src/reader.rs line 1965
mutants crates/lake/src/reader.rs line 1966
mutants crates/lake/src/reader.rs line 1967
mutants crates/lake/src/reader.rs line 1968
mutants crates/lake/src/reader.rs line 1969
mutants crates/lake/src/reader.rs line 1970
mutants crates/lake/src/reader.rs line 1971
mutants crates/lake/src/reader.rs line 1972
mutants crates/lake/src/reader.rs line 1973
mutants crates/lake/src/reader.rs line 1974
mutants crates/lake/src/reader.rs line 1975
mutants crates/lake/src/reader.rs line 1986
mutants crates/lake/src/reader.rs line 2046
mutants crates/lake/src/reader.rs line 2084
mutants crates/lake/src/reader.rs line 2119
mutants crates/lake/src/reader.rs line 2169
mutants crates/lake/src/reader.rs line 2175
mutants crates/lake/src/reader.rs line 2197
mutants crates/lake/src/reader.rs line 2198
mutants crates/lake/src/reader.rs line 2199
mutants crates/lake/src/reader.rs line 2200
mutants crates/lake/src/reader.rs line 35
mutants crates/lake/src/reader.rs mismatch 753-772
mutants crates/lake/src/reader.rs mismatch_of 1976-1985
mutants crates/lake/src/reader.rs timestamp_chunk_ending_past_the_file 2085-2118
mutants crates/lake/src/reader_cover_tests.rs whole
mutants crates/lake/src/schema.rs line 433
mutants crates/lake/src/schema.rs line 434
mutants crates/lake/src/schema.rs line 435
mutants crates/lake/src/schema.rs line 436
mutants crates/lake/src/schema_cover_tests.rs whole
mutants crates/lake/tests/refusals.rs a_column_chunk_whose_bytes_were_cut_is_refused_rather_than_padded_with_nulls 332-458
mutants crates/lake/tests/refusals.rs a_nested_column_is_refused_rather_than_silently_decoding_to_all_null 624-720
mutants crates/lake/tests/refusals.rs an_integer_column_annotated_as_anything_but_a_signed_integer_is_refused_by_name 2055-2140
mutants crates/lake/tests/refusals.rs defects 1081-1090
mutants crates/lake/tests/refusals.rs int64_leaves_file 1022-1076
mutants crates/lake/tests/refusals.rs line 1003
mutants crates/lake/tests/refusals.rs line 1004
mutants crates/lake/tests/refusals.rs line 1005
mutants crates/lake/tests/refusals.rs line 1006
mutants crates/lake/tests/refusals.rs line 1007
mutants crates/lake/tests/refusals.rs line 1008
mutants crates/lake/tests/refusals.rs line 1009
mutants crates/lake/tests/refusals.rs line 1010
mutants crates/lake/tests/refusals.rs line 1011
mutants crates/lake/tests/refusals.rs line 1012
mutants crates/lake/tests/refusals.rs line 1013
mutants crates/lake/tests/refusals.rs line 1014
mutants crates/lake/tests/refusals.rs line 1015
mutants crates/lake/tests/refusals.rs line 1016
mutants crates/lake/tests/refusals.rs line 1017
mutants crates/lake/tests/refusals.rs line 1018
mutants crates/lake/tests/refusals.rs line 1019
mutants crates/lake/tests/refusals.rs line 1020
mutants crates/lake/tests/refusals.rs line 1021
mutants crates/lake/tests/refusals.rs line 1077
mutants crates/lake/tests/refusals.rs line 1078
mutants crates/lake/tests/refusals.rs line 1079
mutants crates/lake/tests/refusals.rs line 1080
mutants crates/lake/tests/refusals.rs line 1091
mutants crates/lake/tests/refusals.rs line 1252
mutants crates/lake/tests/refusals.rs line 1310
mutants crates/lake/tests/refusals.rs line 1495
mutants crates/lake/tests/refusals.rs line 1523
mutants crates/lake/tests/refusals.rs line 58
mutants crates/lake/tests/refusals.rs month_spelling_census 1449-1494
mutants crates/lake/tests/refusals.rs the_month_case_tolerance_is_normalisation_and_can_never_alias_two_contracts 1413-1447
mutants crates/lake/tests/refusals.rs the_month_spelling_census_fails_on_a_name_that_does_not_round_trip 1524-1533
mutants crates/lake/tests/refusals.rs the_month_spelling_census_round_trips_what_it_finds_and_skips_what_is_absent 1496-1522
mutants crates/lake/tests/refusals.rs the_real_lake_scan_finds_each_defect_in_a_file_made_to_carry_it 1092-1251
mutants crates/lake/tests/refusals.rs the_real_lake_walk_finds_month_files_and_samples_by_stride 1253-1309
mutants crates/lake/tests/synthetic.rs a_cash_file_decodes_to_paisa_and_the_layout_is_detected 214-247
mutants crates/lake/tests/synthetic.rs a_page_header_the_reader_cannot_use_is_refused_by_name 1298-1377
mutants crates/lake/tests/synthetic.rs edit_pages 1237-1296
mutants crates/lake/tests/synthetic.rs line 1233
mutants crates/lake/tests/synthetic.rs line 1234
mutants crates/lake/tests/synthetic.rs line 1235
mutants crates/lake/tests/synthetic.rs line 1236
mutants crates/lake/tests/synthetic.rs line 1297
mutants crates/lake/tests/synthetic.rs zstd_copy 890-1003
mutants crates/runner/src/admission.rs an_evidence_record_names_each_malformed_probability_field 9685-9765
mutants crates/runner/src/admission.rs an_evidence_record_whose_authority_slot_is_not_its_projection_is_refused 9767-9793
mutants crates/runner/src/admission.rs evidence_joins_refuse_a_prefilled_authority_slot_and_a_foreign_trade_count 9574-9683
mutants crates/runner/src/admission.rs line 9573
mutants crates/runner/src/admission.rs line 9684
mutants crates/runner/src/admission.rs line 9766
mutants crates/runner/src/admission.rs line 9795
mutants crates/runner/src/admission.rs line 9796
mutants crates/runner/src/admission.rs line 9797
mutants crates/runner/src/admission.rs line 9798
mutants crates/runner/src/admission.rs line 9799
mutants crates/runner/src/admission.rs line 9800
mutants crates/runner/src/admission.rs line 9801
mutants crates/runner/src/admission.rs line 9802
mutants crates/runner/src/admission.rs unavailable_trade_tuple_keeps_every_affected_reason_explicit 7519-7561
mutants crates/runner/src/admission_reader_cover_tests.rs whole
mutants crates/runner/src/admission_statistics_cover_tests.rs whole
mutants crates/runner/src/audit.rs a_completed_bootstrap_prints_each_test_and_whether_it_cleared 2948-2977
mutants crates/runner/src/audit.rs a_grid_with_refused_paths_prints_the_count_and_why 3587-3604
mutants crates/runner/src/audit.rs a_refused_walk_forward_prints_its_reason_and_no_fold 3606-3622
mutants crates/runner/src/audit.rs a_trail_only_cell_is_named_by_its_trail_and_not_as_the_baseline 3575-3585
mutants crates/runner/src/audit.rs an_empty_exit_grid_renders_no_strategy_report 3624-3652
mutants crates/runner/src/audit.rs an_overnight_from_a_close_at_or_below_zero_is_not_measured 3988-4008
mutants crates/runner/src/audit.rs an_upward_traded_overnight_is_signed_with_a_plus 4010-4027
mutants crates/runner/src/audit.rs line 3574
mutants crates/runner/src/audit.rs line 3586
mutants crates/runner/src/audit.rs line 3605
mutants crates/runner/src/audit.rs line 3623
mutants crates/runner/src/audit.rs line 3653
mutants crates/runner/src/audit.rs line 3987
mutants crates/runner/src/audit.rs line 4009
mutants crates/runner/src/audit.rs the_corporate_action_sentence_keeps_the_charge_statement_properties_wherever_it_travels 2591-2662
mutants crates/runner/src/audit.rs the_equity_charge_statement_names_no_rate 2406-2472
mutants crates/runner/src/audit.rs the_integer_renderers_sign_negatives_and_group_lakhs_and_crores 3558-3573
mutants crates/runner/src/audit.rs the_marked_rows_below_the_cut_keep_key_order_when_best_comes_first 3654-3683
mutants crates/runner/src/bootstrap.rs a_draw_below_zero_answers_zero_without_advancing_the_stream 3742-3750
mutants crates/runner/src/bootstrap.rs an_empty_series_summarises_to_nothing 3752-3765
mutants crates/runner/src/bootstrap.rs line 3735
mutants crates/runner/src/bootstrap.rs line 3736
mutants crates/runner/src/bootstrap.rs line 3737
mutants crates/runner/src/bootstrap.rs line 3738
mutants crates/runner/src/bootstrap.rs line 3739
mutants crates/runner/src/bootstrap.rs line 3740
mutants crates/runner/src/bootstrap.rs line 3741
mutants crates/runner/src/bootstrap.rs line 3751
mutants crates/runner/src/bootstrap.rs line 3766
mutants crates/runner/src/bootstrap.rs line 3781
mutants crates/runner/src/bootstrap.rs the_prefix_sum_refuses_a_draw_past_the_exact_bound 2685-2702
mutants crates/runner/src/bootstrap.rs the_stepdown_rejects_exactly_what_the_adjusted_receipt_rejects 3612-3656
mutants crates/runner/src/bootstrap.rs the_surviving_set_strictly_shrinks_every_round_that_rejects 3369-3415
mutants crates/runner/src/bootstrap.rs zero_draws_reject_nothing_at_the_legacy_door 3767-3780
mutants crates/runner/src/bootstrap_family_pass_tests.rs pure_noise_exercises_both_sides_of_every_comparison 172-198
mutants crates/runner/src/bootstrap_zero_v2_tests.rs cold_source_audit_reproduces_full_identity_and_statistics_without_resampling 19-87
mutants crates/runner/src/excursion.rs a_ladder_of_zero_rungs_is_refused_from_any_sample 2658-2663
mutants crates/runner/src/excursion.rs a_stepped_ladder_refuses_a_step_or_count_it_cannot_use 2648-2656
mutants crates/runner/src/excursion.rs a_torn_pair_of_running_extremes_reads_as_nothing_walked 2665-2687
mutants crates/runner/src/excursion.rs line 2635
mutants crates/runner/src/excursion.rs line 2636
mutants crates/runner/src/excursion.rs line 2637
mutants crates/runner/src/excursion.rs line 2638
mutants crates/runner/src/excursion.rs line 2639
mutants crates/runner/src/excursion.rs line 2640
mutants crates/runner/src/excursion.rs line 2641
mutants crates/runner/src/excursion.rs line 2642
mutants crates/runner/src/excursion.rs line 2643
mutants crates/runner/src/excursion.rs line 2644
mutants crates/runner/src/excursion.rs line 2645
mutants crates/runner/src/excursion.rs line 2646
mutants crates/runner/src/excursion.rs line 2647
mutants crates/runner/src/excursion.rs line 2657
mutants crates/runner/src/excursion.rs line 2664
mutants crates/runner/src/excursion.rs line 2688
mutants crates/runner/src/excursion.rs the_ladder_reaches_the_tight_tail_and_never_lands_on_the_maximum 1621-1691
mutants crates/runner/src/excursion.rs the_selected_ladder_is_the_sorted_ladder_rung_for_rung 2557-2586
mutants crates/runner/src/exit_grid_policy.rs a_coordinate_row_width_that_does_not_fit_is_refused_by_its_term 8997-9023
mutants crates/runner/src/exit_grid_policy.rs a_coordinate_visit_stops_where_its_visitor_refuses 5203-5234
mutants crates/runner/src/exit_grid_policy.rs a_grid_view_refuses_what_is_not_its_population 8425-8577
mutants crates/runner/src/exit_grid_policy.rs a_missing_minute_inside_an_oos_hold_refuses_the_one_cell_replay 8787-8825
mutants crates/runner/src/exit_grid_policy.rs a_percentile_over_zero_or_above_one_is_refused_by_name 8579-8599
mutants crates/runner/src/exit_grid_policy.rs a_replay_entering_on_a_bar_below_one_tick_is_refused_by_its_path_classifier 8956-8995
mutants crates/runner/src/exit_grid_policy.rs a_replay_run_of_another_feed_is_refused 8233-8253
mutants crates/runner/src/exit_grid_policy.rs a_resealed_resolution_that_cannot_enumerate_its_grid_is_refused_at_evaluation 8912-8954
mutants crates/runner/src/exit_grid_policy.rs a_resolution_past_its_axis_limit_is_refused_by_name 8383-8424
mutants crates/runner/src/exit_grid_policy.rs a_resolution_resealed_over_a_foreign_policy_digest_is_torn 8622-8641
mutants crates/runner/src/exit_grid_policy.rs a_short_resolution_mints_a_short_witness_whose_direction_is_sealed 8723-8785
mutants crates/runner/src/exit_grid_policy.rs a_training_series_of_another_length_cannot_be_attested 5180-5201
mutants crates/runner/src/exit_grid_policy.rs a_view_into_the_context_is_located_without_a_search 654-702
mutants crates/runner/src/exit_grid_policy.rs admitted_ratio_pairs 3684-3741
mutants crates/runner/src/exit_grid_policy.rs an_attested_replay_prices_and_replays_every_cell_over_one_set_of_slice_facts 6852-7035
mutants crates/runner/src/exit_grid_policy.rs an_attested_training_token_prices_the_same_grid_and_binds_its_resolution 7037-7091
mutants crates/runner/src/exit_grid_policy.rs an_evaluation_and_its_validation_read_back_what_they_bind 8062-8107
mutants crates/runner/src/exit_grid_policy.rs an_evaluation_of_the_other_side_is_refused_by_selection_and_validation 8643-8663
mutants crates/runner/src/exit_grid_policy.rs an_index_segment_key_of_a_non_index_kind_is_refused 8601-8620
mutants crates/runner/src/exit_grid_policy.rs bank_nifty 4722-4724
mutants crates/runner/src/exit_grid_policy.rs candidate_universe_digest_binds_every_path_and_legacy_replay_stays_strict 7359-7444
mutants crates/runner/src/exit_grid_policy.rs canonical_instrument_digest_binds_every_structural_field 4730-4798
mutants crates/runner/src/exit_grid_policy.rs checked_cell_count_matches_every_small_ratio_filtered_enumeration 6041-6082
mutants crates/runner/src/exit_grid_policy.rs corrupt_nonpositive_flat_and_extreme_arithmetic_inputs_never_fabricate_a_level 5922-5982
mutants crates/runner/src/exit_grid_policy.rs empty_duplicate_backward_off_grid_and_subminute_inputs_refuse_loudly 5820-5860
mutants crates/runner/src/exit_grid_policy.rs every_buffer_only_a_refusing_allocator_fails_is_refused_by_name_with_its_size 8284-8381
mutants crates/runner/src/exit_grid_policy.rs every_input_validator_refuses_by_name 8108-8231
mutants crates/runner/src/exit_grid_policy.rs every_replay_door_refusal_names_its_cause 7887-7974
mutants crates/runner/src/exit_grid_policy.rs every_resolved_content_field_is_digest_load_bearing 5652-5750
mutants crates/runner/src/exit_grid_policy.rs every_selector_picks_a_policy_admitted_coordinate 8254-8282
mutants crates/runner/src/exit_grid_policy.rs every_tear_of_a_replayed_universe_fails_its_integrity_gate 7976-8060
mutants crates/runner/src/exit_grid_policy.rs exact_ladders_and_quality_limits_drive_selection_without_oos_derivation 7574-7731
mutants crates/runner/src/exit_grid_policy.rs execution_disposition_reports_each_single_reason_and_combines_reasons 6190-6344
mutants crates/runner/src/exit_grid_policy.rs execution_run_recomputes_data_and_refuses_mismatched_identity_terms 5263-5312
mutants crates/runner/src/exit_grid_policy.rs floor_and_ceiling_resolution_round_a_fractional_ppm_apart 8665-8694
mutants crates/runner/src/exit_grid_policy.rs forced_stop_reason_is_independent_and_legacy_authorization_is_unchanged 6418-6495
mutants crates/runner/src/exit_grid_policy.rs identical_bytes_and_policy_are_byte_identical_and_any_bar_byte_rekeys 5236-5261
mutants crates/runner/src/exit_grid_policy.rs line 5202
mutants crates/runner/src/exit_grid_policy.rs line 5235
mutants crates/runner/src/exit_grid_policy.rs line 7828
mutants crates/runner/src/exit_grid_policy.rs line 7844
mutants crates/runner/src/exit_grid_policy.rs line 7845
mutants crates/runner/src/exit_grid_policy.rs line 7846
mutants crates/runner/src/exit_grid_policy.rs line 7847
mutants crates/runner/src/exit_grid_policy.rs line 7855
mutants crates/runner/src/exit_grid_policy.rs line 7870
mutants crates/runner/src/exit_grid_policy.rs line 7886
mutants crates/runner/src/exit_grid_policy.rs line 7975
mutants crates/runner/src/exit_grid_policy.rs line 8061
mutants crates/runner/src/exit_grid_policy.rs line 8232
mutants crates/runner/src/exit_grid_policy.rs line 8283
mutants crates/runner/src/exit_grid_policy.rs line 8382
mutants crates/runner/src/exit_grid_policy.rs line 8578
mutants crates/runner/src/exit_grid_policy.rs line 8600
mutants crates/runner/src/exit_grid_policy.rs line 8621
mutants crates/runner/src/exit_grid_policy.rs line 8642
mutants crates/runner/src/exit_grid_policy.rs line 8664
mutants crates/runner/src/exit_grid_policy.rs line 8695
mutants crates/runner/src/exit_grid_policy.rs line 8722
mutants crates/runner/src/exit_grid_policy.rs line 8786
mutants crates/runner/src/exit_grid_policy.rs line 8826
mutants crates/runner/src/exit_grid_policy.rs line 8873
mutants crates/runner/src/exit_grid_policy.rs line 8911
mutants crates/runner/src/exit_grid_policy.rs line 8955
mutants crates/runner/src/exit_grid_policy.rs line 8996
mutants crates/runner/src/exit_grid_policy.rs line 9024
mutants crates/runner/src/exit_grid_policy.rs line 9025
mutants crates/runner/src/exit_grid_policy.rs line 9026
mutants crates/runner/src/exit_grid_policy.rs line 9027
mutants crates/runner/src/exit_grid_policy.rs line 9028
mutants crates/runner/src/exit_grid_policy.rs line 9029
mutants crates/runner/src/exit_grid_policy.rs line 9030
mutants crates/runner/src/exit_grid_policy.rs line 9031
mutants crates/runner/src/exit_grid_policy.rs line 9032
mutants crates/runner/src/exit_grid_policy.rs line 9033
mutants crates/runner/src/exit_grid_policy.rs line 9034
mutants crates/runner/src/exit_grid_policy.rs line 9035
mutants crates/runner/src/exit_grid_policy.rs line 9036
mutants crates/runner/src/exit_grid_policy.rs line 9037
mutants crates/runner/src/exit_grid_policy.rs line 9044
mutants crates/runner/src/exit_grid_policy.rs line 9060
mutants crates/runner/src/exit_grid_policy.rs line 9071
mutants crates/runner/src/exit_grid_policy.rs nifty 4718-4720
mutants crates/runner/src/exit_grid_policy.rs observed_ranges 3464-3536
mutants crates/runner/src/exit_grid_policy.rs only_the_two_nse_spot_indices_resolve 4871-4917
mutants crates/runner/src/exit_grid_policy.rs oos_and_run 7871-7885
mutants crates/runner/src/exit_grid_policy.rs oos_replay_refuses_every_non_one_minute_or_corrupt_shape_before_pricing 7227-7357
mutants crates/runner/src/exit_grid_policy.rs p99_is_an_exact_observed_nearest_rank_and_duplicates_collapse 5013-5060
mutants crates/runner/src/exit_grid_policy.rs price 7848-7854
mutants crates/runner/src/exit_grid_policy.rs refusal_identity_binds_coordinate_run_and_the_complete_evaluated_grid 6346-6416
mutants crates/runner/src/exit_grid_policy.rs replay_fixture 7806-7827
mutants crates/runner/src/exit_grid_policy.rs reseal_selected 7829-7843
mutants crates/runner/src/exit_grid_policy.rs resealed 7856-7869
mutants crates/runner/src/exit_grid_policy.rs symbol 4726-4728
mutants crates/runner/src/exit_grid_policy.rs test_column_with_thresholds 4671-4675
mutants crates/runner/src/exit_grid_policy.rs the_column_codec_tags_an_absent_spec_and_acceptance_with_zero 8696-8721
mutants crates/runner/src/exit_grid_policy.rs the_complete_training_grid_has_one_reachable_exact_enumerator 6620-6670
mutants crates/runner/src/exit_grid_policy.rs the_one_cell_replay_refuses_a_stream_past_either_quality_ceiling 8827-8872
mutants crates/runner/src/exit_grid_policy.rs the_per_run_door_refuses_every_foreign_run_term_in_the_fixed_order 7138-7225
mutants crates/runner/src/exit_grid_policy.rs torn 9038-9043
mutants crates/runner/src/exit_grid_policy.rs under_a_required_forced_stop_the_replay_door_admits_only_that_stop 8874-8910
mutants crates/runner/src/exit_grid_policy.rs validated_canonical_ordinals_name_every_authorizable_cell_exactly_once 6084-6167
mutants crates/runner/src/exit_grid_policy.rs whole_minute_gaps_stay_gaps_without_changing_observed_ranges 5862-5901
mutants crates/runner/src/exit_grid_policy.rs with_one_more_cell 9072-9079
mutants crates/runner/src/exit_grid_policy.rs with_training_span 9045-9059
mutants crates/runner/src/exit_grid_policy.rs with_wider_stops 9061-9070
mutants crates/runner/src/exit_grid_policy_cover_tests.rs whole
mutants crates/runner/src/expression.rs malformed_columns_refuse_before_any_evidence_is_published 286-334
mutants crates/runner/src/expression_execution_tests.rs an_expression_over_a_resolution_that_cannot_enumerate_its_grid_is_refused 581-629
mutants crates/runner/src/expression_execution_tests.rs evaluated_fixture 335-364
mutants crates/runner/src/expression_execution_tests.rs every_expression_door_refuses_an_evaluation_that_is_not_its_own 466-579
mutants crates/runner/src/expression_execution_tests.rs every_run_door_and_read_face_answers_what_it_bound 366-464
mutants crates/runner/src/expression_execution_tests.rs line 2
mutants crates/runner/src/expression_execution_tests.rs line 3
mutants crates/runner/src/expression_execution_tests.rs line 330
mutants crates/runner/src/expression_execution_tests.rs line 331
mutants crates/runner/src/expression_execution_tests.rs line 332
mutants crates/runner/src/expression_execution_tests.rs line 333
mutants crates/runner/src/expression_execution_tests.rs line 334
mutants crates/runner/src/expression_execution_tests.rs line 365
mutants crates/runner/src/expression_execution_tests.rs line 4
mutants crates/runner/src/expression_execution_tests.rs line 465
mutants crates/runner/src/expression_execution_tests.rs line 5
mutants crates/runner/src/expression_execution_tests.rs line 580
mutants crates/runner/src/expression_execution_tests.rs programs_and_coordinates_over_one_attestation_derive_the_slice_facts_once 132-178
mutants crates/runner/src/expression_execution_tests.rs same_referenced_bits_do_not_collapse_or_not_or_unknown_programs 180-264
mutants crates/runner/src/expression_execution_tests.rs source_terms_foreign_resolutions_and_incomplete_populations_refuse 266-329
mutants crates/runner/src/expression_oos_tests.rs a_later_timestamp_in_the_same_ist_session_is_not_an_oos_period 299-339
mutants crates/runner/src/expression_oos_tests.rs later_prices_cannot_reresolve_training_levels 241-297
mutants crates/runner/src/expression_oos_tests.rs later_refuses_foreign_frozen_resolution_changed_evaluator_and_entirely_cold_data 341-406
mutants crates/runner/src/expression_oos_tests.rs later_refuses_overlap_program_substitution_foreign_series_and_changed_anchor 170-239
mutants crates/runner/src/expression_oos_tests.rs later_replays_every_frozen_coordinate_and_preserves_zero_unknown_programs 74-169
mutants crates/runner/src/expression_oos_tests.rs line 2
mutants crates/runner/src/expression_oos_tests.rs line 3
mutants crates/runner/src/expression_oos_tests.rs line 4
mutants crates/runner/src/expression_oos_tests.rs line 5
mutants crates/runner/src/expression_oos_tests.rs one_later_slice_prices_every_program_as_the_per_call_path_does 408-486
mutants crates/runner/src/expression_validation_tests.rs a_later_day_whose_every_bar_is_refused_is_not_an_actual_session 774-808
mutants crates/runner/src/expression_validation_tests.rs a_projection_whose_folds_break_the_count_hierarchy_is_refused 810-857
mutants crates/runner/src/expression_validation_tests.rs actual_window_bounds_and_cold_records_follow_sealed_acceptance_authority 486-579
mutants crates/runner/src/expression_validation_tests.rs both_sides_reject_foreign_full_program_anchors_selections_and_ordinals 345-446
mutants crates/runner/src/expression_validation_tests.rs every_fixed_training_authority_refusal_names_its_cause 679-772
mutants crates/runner/src/expression_validation_tests.rs fixed_plan_rejects_training_overlap_missing_dates_empty_actual_folds_and_mapping_ceiling 257-321
mutants crates/runner/src/expression_validation_tests.rs line 2
mutants crates/runner/src/expression_validation_tests.rs line 3
mutants crates/runner/src/expression_validation_tests.rs line 4
mutants crates/runner/src/expression_validation_tests.rs line 5
mutants crates/runner/src/expression_validation_tests.rs line 678
mutants crates/runner/src/expression_validation_tests.rs line 773
mutants crates/runner/src/expression_validation_tests.rs line 809
mutants crates/runner/src/expression_validation_tests.rs one_fold_map_binds_every_program_exactly_as_bind_does 607-677
mutants crates/runner/src/expression_validation_tests.rs with_projection 101-152
mutants crates/runner/src/grid.rs a_walk_built_stop_before_a_hole_is_priced_by_the_cell_and_the_replay_alike 7840-7947
mutants crates/runner/src/grid.rs chosen_materialization_never_contains_a_statefully_refused_path 10440-10540
mutants crates/runner/src/grid.rs line 11120
mutants crates/runner/src/grid.rs line 11121
mutants crates/runner/src/grid.rs line 11122
mutants crates/runner/src/grid.rs line 11123
mutants crates/runner/src/grid.rs materialized_rows_replay_the_selected_variants_own_exclusivity 10939-10994
mutants crates/runner/src/grid.rs replay_universe_from_candidates_v1 3427-3507
mutants crates/runner/src/grid.rs replay_universe_v1 3335-3425
mutants crates/runner/src/grid.rs risk_is_measured_and_a_total_alone_cannot_tell_two_variants_apart 8040-8133
mutants crates/runner/src/grid.rs the_cell_count_is_the_product_of_all_three_ladders_and_nothing_reserves_less 10131-10182
mutants crates/runner/src/grid.rs the_grid_refuses_every_cell_that_cannot_differ 10669-10786
mutants crates/runner/src/grid.rs the_hoisted_replay_doors_equal_the_per_call_doors 6300-6348
mutants crates/runner/src/grid.rs the_indexed_excursion_equals_the_scan_it_replaced 7252-7333
mutants crates/runner/src/grid.rs the_ladders_come_from_the_data_and_a_combination_that_never_trades_has_none 10094-10129
mutants crates/runner/src/grid.rs v1_replay_preserves_refusals_even_when_no_cell_can_be_priced 8616-8687
mutants crates/runner/src/grid_cover_tests.rs whole
mutants crates/runner/src/identity.rs the_identity_renders_as_sixty_four_hex_characters 1454-1462
mutants crates/runner/src/lib.rs line 82
mutants crates/runner/src/lib.rs line 83
mutants crates/runner/src/lib.rs line 84
mutants crates/runner/src/lib.rs line 85
mutants crates/runner/src/lib.rs line 86
mutants crates/runner/src/lib.rs line 87
mutants crates/runner/src/outcome.rs a_bar_the_run_refused_can_never_price_an_outcome 4213-4315
mutants crates/runner/src/outcome.rs a_refused_bar_shrinks_the_sample_rather_than_moving_the_mean 4317-4375
mutants crates/runner/src/outcome.rs a_refused_record_is_never_named_missing 5430-5466
mutants crates/runner/src/outcome.rs duplicate_timestamp_never_prices_a_forward_entry_exit_or_interior 4429-4450
mutants crates/runner/src/outcome.rs every_sum_is_the_exact_sum_converted_once 5115-5162
mutants crates/runner/src/outcome.rs forward_over_hoisted_facts_is_forward 2608-2628
mutants crates/runner/src/outcome.rs line 5545
mutants crates/runner/src/outcome.rs line 5546
mutants crates/runner/src/outcome.rs line 5547
mutants crates/runner/src/outcome.rs line 5548
mutants crates/runner/src/outcome.rs line 5549
mutants crates/runner/src/outcome.rs line 5550
mutants crates/runner/src/outcome.rs line 5551
mutants crates/runner/src/outcome.rs line 5552
mutants crates/runner/src/outcome.rs line 5553
mutants crates/runner/src/outcome.rs line 5554
mutants crates/runner/src/outcome.rs line 5555
mutants crates/runner/src/outcome.rs line 5556
mutants crates/runner/src/outcome.rs line 5567
mutants crates/runner/src/outcome.rs line 5580
mutants crates/runner/src/outcome.rs line 5602
mutants crates/runner/src/outcome.rs line 5632
mutants crates/runner/src/outcome.rs line 5652
mutants crates/runner/src/outcome.rs long 5557-5566
mutants crates/runner/src/outcome.rs no_overlap_pair_crosses_a_session 4861-4885
mutants crates/runner/src/outcome.rs one_paisa_of_genuine_dispersion_still_measures 4107-4152
mutants crates/runner/src/outcome.rs one_window_holding_the_whole_sample_is_no_evidence 4906-4938
mutants crates/runner/src/outcome.rs oversized_accumulator_never_prices_a_forward_entry_exit_or_interior 4452-4478
mutants crates/runner/src/outcome.rs path_ratio_answers_zero_with_no_hits_and_saturates_past_its_bound 5633-5651
mutants crates/runner/src/outcome.rs payoff_names_each_ratio_it_cannot_divide 5603-5631
mutants crates/runner/src/outcome.rs same_day_answers_for_two_existing_bars 5568-5579
mutants crates/runner/src/outcome.rs worst_reward_risk_answers_zero_for_a_ratio_it_cannot_state 5581-5601
mutants crates/runner/src/pbo.rs a_rank_past_the_last_place_has_no_relative_placement 424-439
mutants crates/runner/src/pbo.rs line 440
mutants crates/runner/src/pbo.rs relative 241-266
mutants crates/runner/src/portfolio.rs a_new_scheduler_holds_nothing_and_a_decision_reads_back 1223-1236
mutants crates/runner/src/portfolio.rs every_shuffle_has_the_same_priority_then_digest_answer 746-786
mutants crates/runner/src/portfolio.rs line 1222
mutants crates/runner/src/portfolio.rs line 1237
mutants crates/runner/src/portfolio.rs the_first_bad_intent_is_named_whatever_order_they_came_in 1238-1281
mutants crates/runner/src/rank.rs every_operator_of_a_ranking_wrapper_agrees_with_its_order 1480-1493
mutants crates/runner/src/rank.rs line 1479
mutants crates/runner/src/report.rs an_excluded_position_reaches_the_report_by_name_and_by_reason 1094-1161
mutants crates/runner/src/report.rs line 2021
mutants crates/runner/src/report.rs line 2022
mutants crates/runner/src/report.rs line 2023
mutants crates/runner/src/report.rs line 2024
mutants crates/runner/src/report_cover_tests.rs whole
mutants crates/runner/src/resample.rs open_interest_is_the_last_known_level_and_never_a_sum 548-593
mutants crates/runner/src/research_admission_projection.rs a_search_ceiling_caps_each_family_probability_and_nothing_else 177-217
mutants crates/runner/src/research_admission_projection.rs line 122
mutants crates/runner/src/research_admission_projection.rs line 123
mutants crates/runner/src/research_admission_projection.rs line 124
mutants crates/runner/src/research_admission_projection.rs line 125
mutants crates/runner/src/research_admission_projection.rs line 126
mutants crates/runner/src/research_admission_projection.rs line 127
mutants crates/runner/src/research_admission_projection.rs line 128
mutants crates/runner/src/research_admission_projection.rs line 129
mutants crates/runner/src/research_admission_projection.rs line 130
mutants crates/runner/src/research_admission_projection.rs line 176
mutants crates/runner/src/research_admission_projection.rs line 218
mutants crates/runner/src/research_admission_projection.rs policy 131-175
mutants crates/runner/src/research_exit_grid_tests.rs internally_corrupted_resolution_cannot_reach_ladders_or_training_attestation 15-77
mutants crates/runner/src/research_exit_grid_tests.rs line 2
mutants crates/runner/src/research_exit_grid_tests.rs line 3
mutants crates/runner/src/research_exit_grid_tests.rs line 4
mutants crates/runner/src/research_exit_grid_tests.rs line 5
mutants crates/runner/src/reservation_refusal.rs whole
mutants crates/runner/src/resolved_grid_view.rs coordinate_row_offsets 223-274
mutants crates/runner/src/resolved_grid_view.rs ladders 122-143
mutants crates/runner/src/signal_candle_stop.rs days_before 1441-1471
mutants crates/runner/src/signal_candle_stop_tests.rs a_canonical_word_outside_its_values_is_refused 910-940
mutants crates/runner/src/signal_candle_stop_tests.rs a_forced_close_record_that_breaks_one_rule_is_refused 1062-1103
mutants crates/runner/src/signal_candle_stop_tests.rs a_metrics_record_that_breaks_one_accounting_rule_is_refused 1269-1343
mutants crates/runner/src/signal_candle_stop_tests.rs a_period_record_that_breaks_one_count_rule_is_refused 1181-1267
mutants crates/runner/src/signal_candle_stop_tests.rs a_preparation_whose_day_table_the_allocator_refuses_is_an_allocation_error 1498-1531
mutants crates/runner/src/signal_candle_stop_tests.rs a_priced_trade_reads_back_as_its_statistical_row 879-908
mutants crates/runner/src/signal_candle_stop_tests.rs a_signal_side_that_is_empty_unsorted_or_unlike_its_rows_is_refused 1376-1439
mutants crates/runner/src/signal_candle_stop_tests.rs a_stop_exit_record_that_breaks_one_rule_is_refused 989-1060
mutants crates/runner/src/signal_candle_stop_tests.rs a_tabled_day_window_seals_exactly_what_the_full_row_walk_did 785-821
mutants crates/runner/src/signal_candle_stop_tests.rs an_event_record_that_breaks_one_disposition_rule_is_refused 1105-1179
mutants crates/runner/src/signal_candle_stop_tests.rs an_execution_side_that_is_empty_unsorted_or_not_its_context_is_refused 1441-1496
mutants crates/runner/src/signal_candle_stop_tests.rs an_undirected_run_id_and_an_unknown_timeframe_are_refused_by_scope 1345-1366
mutants crates/runner/src/signal_candle_stop_tests.rs assert_each_trade_refused 978-987
mutants crates/runner/src/signal_candle_stop_tests.rs every_error_reason_and_exit_has_its_own_words 823-864
mutants crates/runner/src/signal_candle_stop_tests.rs forced_long 961-966
mutants crates/runner/src/signal_candle_stop_tests.rs line 1061
mutants crates/runner/src/signal_candle_stop_tests.rs line 1104
mutants crates/runner/src/signal_candle_stop_tests.rs line 1180
mutants crates/runner/src/signal_candle_stop_tests.rs line 1268
mutants crates/runner/src/signal_candle_stop_tests.rs line 1344
mutants crates/runner/src/signal_candle_stop_tests.rs line 1367
mutants crates/runner/src/signal_candle_stop_tests.rs line 1368
mutants crates/runner/src/signal_candle_stop_tests.rs line 1369
mutants crates/runner/src/signal_candle_stop_tests.rs line 1370
mutants crates/runner/src/signal_candle_stop_tests.rs line 1375
mutants crates/runner/src/signal_candle_stop_tests.rs line 1440
mutants crates/runner/src/signal_candle_stop_tests.rs line 1497
mutants crates/runner/src/signal_candle_stop_tests.rs line 822
mutants crates/runner/src/signal_candle_stop_tests.rs line 865
mutants crates/runner/src/signal_candle_stop_tests.rs line 878
mutants crates/runner/src/signal_candle_stop_tests.rs line 909
mutants crates/runner/src/signal_candle_stop_tests.rs line 941
mutants crates/runner/src/signal_candle_stop_tests.rs line 942
mutants crates/runner/src/signal_candle_stop_tests.rs line 943
mutants crates/runner/src/signal_candle_stop_tests.rs line 944
mutants crates/runner/src/signal_candle_stop_tests.rs line 945
mutants crates/runner/src/signal_candle_stop_tests.rs line 960
mutants crates/runner/src/signal_candle_stop_tests.rs line 967
mutants crates/runner/src/signal_candle_stop_tests.rs line 977
mutants crates/runner/src/signal_candle_stop_tests.rs line 988
mutants crates/runner/src/signal_candle_stop_tests.rs ppm_refuses_what_has_no_ratio_and_what_cannot_fit 866-877
mutants crates/runner/src/signal_candle_stop_tests.rs refused_source 1371-1374
mutants crates/runner/src/signal_candle_stop_tests.rs stopped_long 946-959
mutants crates/runner/src/signal_candle_stop_tests.rs with_word 968-976
mutants crates/runner/src/significance.rs line 1390
mutants crates/runner/src/significance.rs line 1391
mutants crates/runner/src/significance.rs line 1392
mutants crates/runner/src/significance.rs line 1393
mutants crates/runner/src/significance_cover_tests.rs whole
mutants crates/runner/src/split.rs an_anchored_fold_never_trains_on_its_own_future 492-522
mutants crates/runner/src/split.rs line 580
mutants crates/runner/src/split.rs the_shapes_name_themselves_and_rolling_refuses_what_it_cannot_cut 581-593
mutants crates/runner/src/synthetic.rs the_two_sided_series_falls_as_readily_as_it_rises 304-352
mutants crates/runner/src/topn.rs a_keeper_asked_for_zero_or_past_its_bound_is_refused 1317-1335
mutants crates/runner/src/topn.rs a_value_outside_the_population_cannot_be_scored 1337-1361
mutants crates/runner/src/topn.rs candidate 830-847
mutants crates/runner/src/topn.rs line 1301
mutants crates/runner/src/topn.rs line 1316
mutants crates/runner/src/topn.rs line 1336
mutants crates/runner/src/topn.rs the_defaults_and_counters_read_what_they_hold 1302-1315
mutants crates/runner/src/trade.rs a_blocked_entry_with_no_square_off_is_counted_while_open_and_recorded_nowhere 3607-3646
mutants crates/runner/src/trade.rs a_path_held_only_by_a_hole_says_where_it_can_still_be_priced 2623-2654
mutants crates/runner/src/trade.rs a_range_or_entry_outside_the_slice_facts_is_answered_no 3739-3779
mutants crates/runner/src/trade.rs a_signal_sourced_cadence_is_not_read_from_bars_after_the_signal 2019-2100
mutants crates/runner/src/trade.rs a_signal_whose_next_minute_is_the_next_day_is_not_entered 3648-3711
mutants crates/runner/src/trade.rs a_slice_that_stops_mid_session_fabricates_no_square_off 2989-3053
mutants crates/runner/src/trade.rs a_sparse_native_slice_cannot_redefine_one_minute_as_its_median_gap 2102-2176
mutants crates/runner/src/trade.rs an_accepted_bar_off_the_whole_minute_is_not_accepted_by_the_slice_facts 3781-3802
mutants crates/runner/src/trade.rs an_accepted_exit_below_one_tick_holds_the_window_unpriced 3517-3574
mutants crates/runner/src/trade.rs an_entry_open_below_one_tick_is_not_held_when_its_exit_cannot_fill 3576-3605
mutants crates/runner/src/trade.rs line 3516
mutants crates/runner/src/trade.rs line 3575
mutants crates/runner/src/trade.rs line 3606
mutants crates/runner/src/trade.rs line 3647
mutants crates/runner/src/trade.rs line 3712
mutants crates/runner/src/trade.rs line 3738
mutants crates/runner/src/trade.rs line 3780
mutants crates/runner/src/trade.rs no_two_trades_overlap_and_every_signal_is_accounted_for 2178-2213
mutants crates/runner/src/trade.rs stateful_refusals_gate_long_and_short_entries_exits_and_interiors 2711-2835
mutants crates/runner/src/trade.rs with_no_measured_cadence_no_signal_is_entered 3713-3737
mutants crates/runner/src/validate.rs a_prepared_fold_builder_refusal_never_falls_back_to_the_ordinary_evaluator 5748-5776
mutants crates/runner/src/validate.rs a_projected_walk_accepts_minute_gaps_but_refuses_malformed_execution_time 5683-5746
mutants crates/runner/src/validate.rs a_rolling_walk_forward_slides_its_window_and_an_anchored_one_grows 6212-6286
mutants crates/runner/src/validate.rs a_walk_forward_never_judges_a_choice_on_a_bar_it_was_chosen_on 6397-6422
mutants crates/runner/src/validate.rs admission_candidate_v2 5574-5586
mutants crates/runner/src/validate.rs admission_capture_v2 5561-5572
mutants crates/runner/src/validate.rs admission_split_v2 5588-5595
mutants crates/runner/src/validate.rs anchored_admission_attack_coordinates_v2 7321-7342
mutants crates/runner/src/validate.rs anchored_admission_fixture_v2 5447-5452
mutants crates/runner/src/validate.rs anchored_admission_v2_refuses_exit_and_ladder_crosswires 7415-7450
mutants crates/runner/src/validate.rs anchored_admission_v2_refuses_mask_and_side_crosswires 7378-7413
mutants crates/runner/src/validate.rs anchored_admission_v2_refuses_zero_rungs_before_running_any_search 5597-5621
mutants crates/runner/src/validate.rs anchored_search_fixture_v3 5480-5485
mutants crates/runner/src/validate.rs anchored_search_v3_projection_is_typed_sealed_and_rekeys 7503-7605
mutants crates/runner/src/validate.rs anchored_search_v4_causal_happy_path_exposes_exact_grid_components 7607-7708
mutants crates/runner/src/validate.rs evaluator 5371-5377
mutants crates/runner/src/validate.rs every_finished_fold_is_reported_once_in_order_with_its_own_counts 5958-6030
mutants crates/runner/src/validate.rs exact_grid_policy_v4 5487-5508
mutants crates/runner/src/validate.rs h 5379-5381
mutants crates/runner/src/validate.rs held_up_judges_the_strategy_that_was_chosen 6577-6642
mutants crates/runner/src/validate.rs line 5391
mutants crates/runner/src/validate.rs line 5399
mutants crates/runner/src/validate.rs line 8177
mutants crates/runner/src/validate.rs line 8178
mutants crates/runner/src/validate.rs line 8179
mutants crates/runner/src/validate.rs line 8180
mutants crates/runner/src/validate.rs opposite 5383-5390
mutants crates/runner/src/validate.rs sweeper 5400-5406
mutants crates/runner/src/validate.rs the_chosen_combination_is_the_best_of_every_candidate_and_not_of_a_prefix 6861-7001
mutants crates/runner/src/validate.rs the_chosen_exit_is_applied_out_of_sample_and_not_merely_recorded 6644-6690
mutants crates/runner/src/validate.rs the_other_side_of_the_other_side_is_the_side 5392-5398
mutants crates/runner/src/validate_cover_tests.rs whole
mutants crates/runner/tests/auto_probe_found_nothing.rs whole
mutants crates/runner/tests/auto_probe_reporter_refusal.rs whole
mutants crates/runner/tests/base_cov_admission_words.rs whole
mutants crates/runner/tests/checkpointed_rank_refusals.rs whole
mutants crates/runner/tests/d_0694_record.rs line 266
mutants crates/runner/tests/d_0694_record.rs the_minute_gaps_walk_reads_only_non_test_sources_at_any_depth 267-295
mutants crates/runner/tests/end_to_end.rs an_equity_audit_is_byte_identical_on_a_rerun_and_differs_from_the_index_one_only_in_its_charge_words 341-421
mutants crates/runner/tests/end_to_end.rs the_ladder_reaches_further_as_support_falls_unless_a_budget_stops_it_first 423-533
mutants crates/runner/tests/end_to_end.rs the_whole_pipeline_runs_and_every_stage_feeds_the_next 58-252
mutants crates/runner/tests/expression_pricing.rs conjunctions_match_all_legacy_trade_and_grid_fields_for_both_sides 36-92
mutants crates/runner/tests/expression_pricing.rs fixture 14-25
mutants crates/runner/tests/expression_pricing.rs levels 26-34
mutants crates/runner/tests/expression_pricing.rs line 2
mutants crates/runner/tests/expression_pricing.rs line 3
mutants crates/runner/tests/expression_pricing.rs line 4
mutants crates/runner/tests/expression_pricing.rs line 5
mutants crates/runner/tests/expression_pricing.rs mixed_operators_price_only_definite_signals_and_replay_exact_selected_cells 94-173
mutants crates/runner/tests/expression_pricing.rs unavailable_negation_and_contradiction_never_create_trades 175-205
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs actual_clock_guard_preserves_earlier_horizons_on_both_public_predicate_paths 147-213
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs checked_grids_refuse_an_actual_late_or_foreign_day_inside_cached_endpoints 269-348
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs every_public_cached_walk_refuses_foreign_facts_that_move_1509_to_1520 60-145
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs line 2
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs line 3
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs line 4
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs line 5
mutants crates/runner/tests/foreign_slice_deadline_readiness.rs matching_forced_boundary_cannot_hide_a_foreign_earlier_horizon_lookup 215-267
mutants crates/runner/tests/hole_after_exit.rs early_stops 92-111
mutants crates/runner/tests/population_door_halt.rs whole
mutants crates/runner/tests/research_family_readiness.rs eligible_cash_prices_and_replays_every_boolean_coordinate_without_index_alias 495-592
mutants crates/runner/tests/research_family_readiness.rs legacy_resolution_identity_matches_the_recorded_pre_extraction_library 355-444
mutants crates/runner/tests/research_family_readiness.rs line 2
mutants crates/runner/tests/research_family_readiness.rs line 3
mutants crates/runner/tests/research_family_readiness.rs line 4
mutants crates/runner/tests/research_family_readiness.rs line 5
mutants crates/runner/tests/research_family_readiness.rs new_resolution_retains_physical_refusals_and_exact_source_identity 446-493
mutants crates/runner/tests/unwired_validation_record.rs line 134
mutants crates/runner/tests/unwired_validation_record.rs the_production_walk_skips_test_directories_test_files_and_non_sources 135-172
mutants crates/store/src/catalog.rs classify 432-509
mutants crates/store/src/catalog.rs line 427
mutants crates/store/src/catalog.rs line 428
mutants crates/store/src/catalog.rs line 429
mutants crates/store/src/catalog.rs line 550
mutants crates/store/src/catalog.rs line 551
mutants crates/store/src/catalog.rs line 552
mutants crates/store/src/catalog.rs line 553
mutants crates/store/src/catalog_cover_tests.rs whole
mutants crates/store/src/checksum_audit.rs line 379
mutants crates/store/src/checksum_audit.rs line 380
mutants crates/store/src/checksum_audit.rs line 381
mutants crates/store/src/checksum_audit.rs line 382
mutants crates/store/src/checksum_audit.rs line 383
mutants crates/store/src/checksum_audit.rs line 384
mutants crates/store/src/checksum_audit_cover_tests.rs whole
mutants crates/store/src/file.rs arm_host_fault 6546-6557
mutants crates/store/src/file.rs arm_race 6679-6682
mutants crates/store/src/file.rs arm_refusal 6629-6640
mutants crates/store/src/file.rs directory_flushes 6795-6798
mutants crates/store/src/file.rs disarm_host_fault 6559-6562
mutants crates/store/src/file.rs fmt 1246-1367
mutants crates/store/src/file.rs host_fault_armed 6564-6567
mutants crates/store/src/file.rs line 10747
mutants crates/store/src/file.rs line 10748
mutants crates/store/src/file.rs line 10749
mutants crates/store/src/file.rs line 10750
mutants crates/store/src/file.rs line 10751
mutants crates/store/src/file.rs line 10752
mutants crates/store/src/file.rs line 10753
mutants crates/store/src/file.rs line 10754
mutants crates/store/src/file.rs line 10755
mutants crates/store/src/file.rs line 10756
mutants crates/store/src/file.rs line 10757
mutants crates/store/src/file.rs line 10758
mutants crates/store/src/file.rs line 6509
mutants crates/store/src/file.rs line 6605
mutants crates/store/src/file.rs line 6606
mutants crates/store/src/file.rs line 6607
mutants crates/store/src/file.rs line 6608
mutants crates/store/src/file.rs line 6609
mutants crates/store/src/file.rs line 6610
mutants crates/store/src/file.rs line 6611
mutants crates/store/src/file.rs line 6612
mutants crates/store/src/file.rs line 6613
mutants crates/store/src/file.rs line 6614
mutants crates/store/src/file.rs line 6615
mutants crates/store/src/file.rs line 6616
mutants crates/store/src/file.rs line 6617
mutants crates/store/src/file.rs line 6618
mutants crates/store/src/file.rs line 6619
mutants crates/store/src/file.rs line 6620
mutants crates/store/src/file.rs line 6621
mutants crates/store/src/file.rs line 6622
mutants crates/store/src/file.rs line 6623
mutants crates/store/src/file.rs line 6624
mutants crates/store/src/file.rs line 6625
mutants crates/store/src/file.rs line 6626
mutants crates/store/src/file.rs line 6627
mutants crates/store/src/file.rs line 6628
mutants crates/store/src/file.rs line 6641
mutants crates/store/src/file.rs line 6646
mutants crates/store/src/file.rs line 6667
mutants crates/store/src/file.rs line 6668
mutants crates/store/src/file.rs line 6669
mutants crates/store/src/file.rs line 6670
mutants crates/store/src/file.rs line 6671
mutants crates/store/src/file.rs line 6672
mutants crates/store/src/file.rs line 6673
mutants crates/store/src/file.rs line 6674
mutants crates/store/src/file.rs line 6675
mutants crates/store/src/file.rs line 6676
mutants crates/store/src/file.rs line 6677
mutants crates/store/src/file.rs line 6678
mutants crates/store/src/file.rs line 6683
mutants crates/store/src/file.rs line 6688
mutants crates/store/src/file.rs line 6708
mutants crates/store/src/file.rs line 6730
mutants crates/store/src/file.rs race 6689-6707
mutants crates/store/src/file.rs race_armed 6684-6687
mutants crates/store/src/file.rs refusal 6647-6666
mutants crates/store/src/file.rs refusal_armed 6642-6645
mutants crates/store/src/file.rs the_operation_refusal_fires_once_on_its_word_and_path_after_its_skips 6709-6729
mutants crates/store/src/file.rs write_sealed 1370-1396
mutants crates/store/src/file.rs write_sealed_refused 1398-1412
mutants crates/store/src/file_cover_tests.rs whole
mutants crates/store/src/file_index_cover_tests.rs whole
mutants crates/store/src/file_sealed.rs audit_sealed 397-446
mutants crates/store/src/file_sealed.rs line 484
mutants crates/store/src/file_sealed.rs line 485
mutants crates/store/src/file_sealed.rs line 486
mutants crates/store/src/file_sealed.rs line 487
mutants crates/store/src/file_sealed.rs line 488
mutants crates/store/src/file_sealed.rs line 489
mutants crates/store/src/file_sealed_cover_tests.rs whole
mutants crates/store/src/flock.rs end 689-715
mutants crates/store/src/flock.rs line 1181
mutants crates/store/src/flock.rs line 1182
mutants crates/store/src/flock.rs line 1183
mutants crates/store/src/flock.rs line 1184
mutants crates/store/src/flock.rs line 1185
mutants crates/store/src/flock.rs take_held 249-292
mutants crates/store/src/flock.rs try_lock_noted 533-596
mutants crates/store/src/flock.rs try_lock_shared 598-614
mutants crates/store/src/flock_cover_tests.rs whole
mutants crates/store/src/flock_tests.rs host_refusal_armed 90-93
mutants crates/store/src/flock_tests.rs line 111
mutants crates/store/src/flock_tests.rs line 52
mutants crates/store/src/flock_tests.rs line 53
mutants crates/store/src/flock_tests.rs line 54
mutants crates/store/src/flock_tests.rs line 55
mutants crates/store/src/flock_tests.rs line 56
mutants crates/store/src/flock_tests.rs line 57
mutants crates/store/src/flock_tests.rs line 58
mutants crates/store/src/flock_tests.rs line 59
mutants crates/store/src/flock_tests.rs line 60
mutants crates/store/src/flock_tests.rs line 61
mutants crates/store/src/flock_tests.rs line 62
mutants crates/store/src/flock_tests.rs line 63
mutants crates/store/src/flock_tests.rs line 64
mutants crates/store/src/flock_tests.rs line 65
mutants crates/store/src/flock_tests.rs line 66
mutants crates/store/src/flock_tests.rs line 67
mutants crates/store/src/flock_tests.rs line 68
mutants crates/store/src/flock_tests.rs line 69
mutants crates/store/src/flock_tests.rs line 70
mutants crates/store/src/flock_tests.rs line 71
mutants crates/store/src/flock_tests.rs line 72
mutants crates/store/src/flock_tests.rs line 73
mutants crates/store/src/flock_tests.rs line 74
mutants crates/store/src/flock_tests.rs line 75
mutants crates/store/src/flock_tests.rs line 76
mutants crates/store/src/flock_tests.rs line 77
mutants crates/store/src/flock_tests.rs line 83
mutants crates/store/src/flock_tests.rs line 89
mutants crates/store/src/flock_tests.rs line 94
mutants crates/store/src/flock_tests.rs refuse_host 84-88
mutants crates/store/src/flock_tests.rs refuse_next_host 78-82
mutants crates/store/src/flock_tests.rs take_host_refusal 95-110
mutants crates/store/src/generation.rs encode 131-149
mutants crates/store/src/generation.rs io_at 600-621
mutants crates/store/src/generation.rs line 199
mutants crates/store/src/generation.rs line 642
mutants crates/store/src/generation.rs line 643
mutants crates/store/src/generation.rs line 644
mutants crates/store/src/generation.rs line 645
mutants crates/store/src/generation.rs line 646
mutants crates/store/src/generation.rs line 647
mutants crates/store/src/generation.rs line 648
mutants crates/store/src/generation.rs line 649
mutants crates/store/src/generation.rs line 650
mutants crates/store/src/generation.rs put_field 189-198
mutants crates/store/src/generation_cover_tests.rs whole
mutants crates/store/src/generation_door_cover_tests.rs whole
mutants crates/store/src/month_summary.rs line 1364
mutants crates/store/src/month_summary.rs line 1365
mutants crates/store/src/month_summary.rs line 1366
mutants crates/store/src/month_summary.rs line 1367
mutants crates/store/src/month_summary.rs line 1368
mutants crates/store/src/month_summary.rs line 1369
mutants crates/store/src/month_summary.rs line 1370
mutants crates/store/src/month_summary.rs line 1371
mutants crates/store/src/month_summary.rs line 1372
mutants crates/store/src/month_summary_cover_tests.rs whole
mutants crates/store/src/month_summary_decode_cover_tests.rs whole
mutants crates/store/src/month_summary_writer.rs line 356
mutants crates/store/src/month_summary_writer.rs line 357
mutants crates/store/src/month_summary_writer.rs line 358
mutants crates/store/src/month_summary_writer.rs line 359
mutants crates/store/src/month_summary_writer.rs line 360
mutants crates/store/src/month_summary_writer.rs line 361
mutants crates/store/src/month_summary_writer_cover_tests.rs whole
mutants crates/store/src/provenance.rs line 1678
mutants crates/store/src/provenance.rs line 1679
mutants crates/store/src/provenance.rs line 1680
mutants crates/store/src/provenance.rs line 1681
mutants crates/store/src/provenance.rs line 1682
mutants crates/store/src/provenance.rs write_fully 1550-1563
mutants crates/store/src/provenance_cover_tests.rs whole
mutants crates/store/src/quarantine.rs audit_dir 1454-1473
mutants crates/store/src/quarantine.rs empty_image 1982-1996
mutants crates/store/src/quarantine.rs encode_slot 767-787
mutants crates/store/src/quarantine.rs field 720-726
mutants crates/store/src/quarantine.rs io_at 641-660
mutants crates/store/src/quarantine.rs level 2168-2202
mutants crates/store/src/quarantine.rs line 2804
mutants crates/store/src/quarantine.rs line 2805
mutants crates/store/src/quarantine.rs line 2806
mutants crates/store/src/quarantine.rs line 2807
mutants crates/store/src/quarantine.rs line 800
mutants crates/store/src/quarantine.rs load 2366-2422
mutants crates/store/src/quarantine.rs note_cut 1435-1452
mutants crates/store/src/quarantine.rs note_damaged 1418-1433
mutants crates/store/src/quarantine.rs open_index 848-883
mutants crates/store/src/quarantine.rs probe_at 818-846
mutants crates/store/src/quarantine.rs publish_image 2228-2261
mutants crates/store/src/quarantine.rs put 2096-2117
mutants crates/store/src/quarantine.rs read_image 1179-1215
mutants crates/store/src/quarantine.rs refuse_damage 1936-1947
mutants crates/store/src/quarantine.rs seal_header 1998-2001
mutants crates/store/src/quarantine.rs set_described 2119-2123
mutants crates/store/src/quarantine.rs write_at 789-799
mutants crates/store/src/quarantine_cover_tests.rs whole
mutants crates/store/src/quarantine_tests.rs a_directory_at_the_lock_or_the_index_name_refuses_the_act_before_any_line 2331-2374
mutants crates/store/src/quarantine_tests.rs a_directory_made_at_the_index_before_its_rename_refuses_the_publication 2620-2645
mutants crates/store/src/quarantine_tests.rs a_ledger_holding_two_series_answers_each_only_its_own_dropped_stamps 2168-2203
mutants crates/store/src/quarantine_tests.rs a_ledger_read_the_host_refuses_is_refused_by_the_ledgers_name 2555-2579
mutants crates/store/src/quarantine_tests.rs a_lock_the_host_will_not_ask_about_is_no_writer_to_a_probe 2710-2754
mutants crates/store/src/quarantine_tests.rs a_quarantine_lock_the_host_refuses_is_refused_before_anything_is_written 2756-2779
mutants crates/store/src/quarantine_tests.rs a_release_the_host_refuses_names_the_lock_and_keeps_the_line 2296-2329
mutants crates/store/src/quarantine_tests.rs a_resume_whose_dropped_line_the_ledger_refuses_is_still_owed 2416-2430
mutants crates/store/src/quarantine_tests.rs a_resume_whose_dropped_line_the_ledger_refuses_is_still_owed_body 2432-2475
mutants crates/store/src/quarantine_tests.rs a_slot_of_the_same_hash_and_another_month_is_passed_over 2376-2414
mutants crates/store/src/quarantine_tests.rs an_audit_directory_made_between_the_look_and_the_creation_is_taken 2686-2708
mutants crates/store/src/quarantine_tests.rs an_index_cut_between_its_measure_and_its_read_is_rebuilt_by_the_writer 2498-2523
mutants crates/store/src/quarantine_tests.rs an_index_cut_under_a_probe_refuses_it_at_the_read_it_cut 2647-2684
mutants crates/store/src/quarantine_tests.rs an_index_image_whose_staged_write_is_refused_is_never_published 2581-2618
mutants crates/store/src/quarantine_tests.rs an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply 1220-1233
mutants crates/store/src/quarantine_tests.rs an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply_body 1235-1277
mutants crates/store/src/quarantine_tests.rs an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply 1279-1291
mutants crates/store/src/quarantine_tests.rs an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply_body 1293-1337
mutants crates/store/src/quarantine_tests.rs an_index_read_the_host_refuses_refuses_the_act_before_its_line 2525-2553
mutants crates/store/src/quarantine_tests.rs every_quarantine_refusal_renders_its_code_its_path_and_its_figures 2015-2166
mutants crates/store/src/quarantine_tests.rs ledger_lines 2490-2496
mutants crates/store/src/quarantine_tests.rs line 1234
mutants crates/store/src/quarantine_tests.rs line 1292
mutants crates/store/src/quarantine_tests.rs line 2014
mutants crates/store/src/quarantine_tests.rs line 2167
mutants crates/store/src/quarantine_tests.rs line 2204
mutants crates/store/src/quarantine_tests.rs line 2280
mutants crates/store/src/quarantine_tests.rs line 2295
mutants crates/store/src/quarantine_tests.rs line 2330
mutants crates/store/src/quarantine_tests.rs line 2375
mutants crates/store/src/quarantine_tests.rs line 2415
mutants crates/store/src/quarantine_tests.rs line 2431
mutants crates/store/src/quarantine_tests.rs line 2476
mutants crates/store/src/quarantine_tests.rs line 2489
mutants crates/store/src/quarantine_tests.rs line 2497
mutants crates/store/src/quarantine_tests.rs line 2524
mutants crates/store/src/quarantine_tests.rs line 2554
mutants crates/store/src/quarantine_tests.rs line 2580
mutants crates/store/src/quarantine_tests.rs line 2619
mutants crates/store/src/quarantine_tests.rs line 2646
mutants crates/store/src/quarantine_tests.rs line 2685
mutants crates/store/src/quarantine_tests.rs line 2709
mutants crates/store/src/quarantine_tests.rs line 2755
mutants crates/store/src/quarantine_tests.rs one_act_recorded 2477-2488
mutants crates/store/src/quarantine_tests.rs two_runs_killed_after_their_promotions_are_both_resumed_and_counted 2205-2279
mutants crates/store/src/quarantine_tests.rs whole_act 2281-2294
mutants crates/store/src/repair.rs io_at 1521-1541
mutants crates/store/src/repair.rs line 1546
mutants crates/store/src/repair.rs line 1547
mutants crates/store/src/repair.rs line 1548
mutants crates/store/src/repair.rs line 1549
mutants crates/store/src/repair.rs line 1550
mutants crates/store/src/repair.rs line 1551
mutants crates/store/src/repair.rs line 1552
mutants crates/store/src/repair.rs line 1553
mutants crates/store/src/repair.rs line 1554
mutants crates/store/src/repair.rs restate_month 1090-1221
mutants crates/store/src/repair.rs write_revision 1318-1413
mutants crates/store/src/repair_cover_tests.rs whole
mutants crates/store/src/repair_door_cover_tests.rs whole
mutants crates/store/src/safe_open_tests.rs a_file_of_none_of_the_seven_kinds_is_unknown 486-523
mutants crates/store/src/safe_open_tests.rs a_regular_file_the_host_refuses_keeps_the_hosts_words 258-269
mutants crates/store/src/safe_open_tests.rs a_regular_file_the_host_refuses_keeps_the_hosts_words_body 271-287
mutants crates/store/src/safe_open_tests.rs line 270
mutants crates/store/src/safe_open_tests.rs line 485
mutants crates/store/src/sealed.rs line 2524
mutants crates/store/src/sealed.rs line 2525
mutants crates/store/src/sealed.rs line 2526
mutants crates/store/src/sealed.rs line 2527
mutants crates/store/src/sealed.rs line 2528
mutants crates/store/src/sealed_fold_cover_tests.rs whole
mutants crates/store/src/sealed_tests.rs a_full_disk_at_any_byte_is_the_writers_refusal_and_the_prefix_is_never_a_month 944-1019
mutants crates/store/src/settings.rs landed 1414-1484
mutants crates/store/src/settings.rs line 1489
mutants crates/store/src/settings.rs line 1490
mutants crates/store/src/settings.rs line 1491
mutants crates/store/src/settings.rs line 1492
mutants crates/store/src/settings.rs line 1493
mutants crates/store/src/settings.rs line 1494
mutants crates/store/src/settings.rs line 1495
mutants crates/store/src/settings.rs line 1496
mutants crates/store/src/settings.rs line 1497
mutants crates/store/src/settings.rs read_one 845-896
mutants crates/store/src/settings.rs save 1277-1412
mutants crates/store/src/settings.rs write_temp 1208-1252
mutants crates/store/src/settings_cover_tests.rs whole
mutants crates/store/src/settings_door_cover_tests.rs whole
mutants crates/store/src/settings_tests.rs a_change_is_checked_as_a_whole_before_any_byte 208-291
mutants crates/store/src/settings_tests.rs a_directory_that_stops_being_readable_is_a_fault_not_nothing_new 605-622
mutants crates/store/src/settings_tests.rs a_directory_that_stops_being_readable_is_a_fault_not_nothing_new_body 624-663
mutants crates/store/src/settings_tests.rs a_disk_that_refuses_the_write_is_refused_with_nothing_visible 486-498
mutants crates/store/src/settings_tests.rs a_disk_that_refuses_the_write_is_refused_with_nothing_visible_body 500-515
mutants crates/store/src/settings_tests.rs a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason 1106-1116
mutants crates/store/src/settings_tests.rs a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason_body 1118-1137
mutants crates/store/src/settings_tests.rs line 1117
mutants crates/store/src/settings_tests.rs line 499
mutants crates/store/src/settings_tests.rs line 623
mutants crates/store/src/sort_index.rs build_run_as 720-811
mutants crates/store/src/sort_index.rs decode_run_header 865-968
mutants crates/store/src/sort_index.rs line 1536
mutants crates/store/src/sort_index.rs line 1537
mutants crates/store/src/sort_index.rs line 1538
mutants crates/store/src/sort_index.rs line 1539
mutants crates/store/src/sort_index.rs put 1522-1531
mutants crates/store/src/sort_index.rs take 1512-1520
mutants crates/store/src/sort_index_cover_tests.rs whole
mutants crates/store/src/staging.rs create 250-323
mutants crates/store/src/staging.rs line 328
mutants crates/store/src/staging.rs line 329
mutants crates/store/src/staging.rs line 330
mutants crates/store/src/staging.rs line 331
mutants crates/store/src/staging.rs line 332
mutants crates/store/src/staging_cover_tests.rs whole
mutants crates/store/tests/base_cov_words.rs whole
mutants crates/store/tests/cited_commits.rs a_site_list_passes_over_words_that_name_no_site 1006-1028
mutants crates/store/tests/cited_commits.rs line 1005
mutants crates/store/tests/docs.rs the_c01_syscall_sentence_is_superseded_where_it_stands 209-243
mutants crates/store/tests/misfiled.rs line 350
mutants crates/store/tests/misfiled.rs the_civil_day_count_is_hinnants_on_known_days 351-360
mutants crates/store/tests/month_summary_log.rs line 159
mutants crates/store/tests/month_summary_log.rs the_field_reader_says_every_value_as_text 160-172
mutants crates/store/tests/slot_damaged_log.rs line 242
mutants crates/store/tests/slot_damaged_log.rs the_count_reader_takes_a_count_past_the_signed_range 243-260
mutants crates/store/tests/sort_index_log.rs line 190
mutants crates/store/tests/sort_index_log.rs the_field_reader_says_every_value_as_text 191-203
mutants crates/store/tests/tix.rs an_empty_month_is_probed_at_its_edges_alone 613-622
mutants crates/store/tests/tix.rs line 612
mutants crates/store/tests/tix_rebuilder.rs an_empty_month_is_probed_at_its_edges_alone 526-535
mutants crates/store/tests/tix_rebuilder.rs line 525
mutants crates/store/tests/tix_small_month.rs an_empty_month_is_probed_at_its_edges_alone 605-614
mutants crates/store/tests/tix_small_month.rs line 604
mutants crates/store/tests/tix_warn_once.rs line 359
mutants crates/store/tests/tix_warn_once.rs the_value_readers_take_what_the_sink_gives_back 360-370
mutants crates/telemetry/src/clock.rs an_instant_either_side_of_the_epoch_routes_to_its_signed_millis 184-211
mutants crates/telemetry/src/clock.rs line 180
mutants crates/telemetry/src/clock.rs line 181
mutants crates/telemetry/src/clock.rs line 212
mutants crates/telemetry/src/clock.rs line 39
mutants crates/telemetry/src/clock.rs millis_since_epoch 40-51
mutants crates/telemetry/src/clock.rs now_millis 34-38
mutants crates/telemetry/src/clock.rs push_padded 125-159
mutants crates/telemetry/src/lib.rs where_permission_binds 396-426
mutants crates/telemetry/src/record.rs a_record_that_lost_a_different_number_of_fields_does_not_match 397-410
mutants crates/telemetry/src/record.rs line 411
mutants crates/telemetry/src/sink.rs a_root_that_is_not_there_holds_no_files 8029-8032
mutants crates/telemetry/src/sink.rs count_code 915-936
mutants crates/telemetry/src/sink.rs every_line 10265-10280
mutants crates/telemetry/src/sink.rs line 11695
mutants crates/telemetry/src/sink.rs line 11696
mutants crates/telemetry/src/sink.rs line 11697
mutants crates/telemetry/src/sink.rs line 11698
mutants crates/telemetry/src/sink.rs line 4827
mutants crates/telemetry/src/sink.rs line 8033
mutants crates/telemetry/src/sink.rs reaped 8077-8102
mutants crates/telemetry/src/sink.rs seed_aside 3765-3855
mutants crates/telemetry/src/sink.rs seed_state 4014-4115
mutants crates/telemetry/src/sink.rs the_keep_nothing_doubles_accept_every_call_of_the_trait 4803-4826
mutants crates/telemetry/src/sink_recovery_tests.rs whole
mutants crates/telemetry/src/tail.rs walked 365-454
mutants crates/telemetry/tests/global_absent.rs admits_with_no_sink_is_false_and_the_macro_evaluates_nothing 75-119
mutants crates/vocab/src/expression.rs a_child_link_back_to_its_own_node_ends_the_render_in_an_error_not_a_loop 900-927
mutants crates/vocab/src/expression.rs encode 352-384
mutants crates/vocab/src/expression.rs line 858
mutants crates/vocab/src/expression.rs line 859
mutants crates/vocab/src/expression.rs line 860
mutants crates/vocab/src/expression.rs line 861
mutants crates/vocab/src/expression.rs line 862
mutants crates/vocab/src/expression.rs line 863
mutants crates/vocab/src/expression.rs line 864
mutants crates/vocab/src/expression.rs line 865
mutants crates/vocab/src/expression.rs line 866
mutants crates/vocab/src/expression.rs line 867
mutants crates/vocab/src/expression.rs line 868
mutants crates/vocab/src/expression.rs line 869
mutants crates/vocab/src/expression.rs line 870
mutants crates/vocab/src/expression.rs line 871
mutants crates/vocab/src/expression.rs line 872
mutants crates/vocab/src/expression.rs line 873
mutants crates/vocab/src/expression.rs line 874
mutants crates/vocab/src/expression.rs line 875
mutants crates/vocab/src/expression.rs line 876
mutants crates/vocab/src/expression.rs line 877
mutants crates/vocab/src/expression.rs line 878
mutants crates/vocab/src/expression.rs line 879
mutants crates/vocab/src/expression.rs line 890
mutants crates/vocab/src/expression.rs line 899
mutants crates/vocab/src/expression.rs line 928
mutants crates/vocab/src/expression.rs program 880-889
mutants crates/vocab/src/expression.rs render_program 399-462
mutants crates/vocab/src/expression.rs rendered 891-898
mutants crates/vocab/src/expression.rs run 282-334
mutants crates/vocab/src/expression_search.rs admits 423-444
mutants crates/vocab/src/expression_search.rs line 467
mutants crates/vocab/src/expression_search.rs the_references_refuse_an_overlong_prefix_and_an_uncarried_length 446-466
mutants crates/vocab/src/expression_search.rs valid_prefix 384-421
mutants crates/vocab/src/implication.rs a_pair_is_informative_exactly_when_neither_restates_the_other_and_both_can_hold 420-436
mutants crates/vocab/src/implication.rs chain 391-398
mutants crates/vocab/src/implication.rs compatibility_is_symmetric_and_refuses_exactly_the_impossible_bands 400-418
mutants crates/vocab/src/implication.rs line 374
mutants crates/vocab/src/implication.rs line 375
mutants crates/vocab/src/implication.rs line 376
mutants crates/vocab/src/implication.rs line 377
mutants crates/vocab/src/implication.rs line 378
mutants crates/vocab/src/implication.rs line 379
mutants crates/vocab/src/implication.rs line 380
mutants crates/vocab/src/implication.rs line 381
mutants crates/vocab/src/implication.rs line 382
mutants crates/vocab/src/implication.rs line 383
mutants crates/vocab/src/implication.rs line 384
mutants crates/vocab/src/implication.rs line 385
mutants crates/vocab/src/implication.rs line 386
mutants crates/vocab/src/implication.rs line 387
mutants crates/vocab/src/implication.rs line 388
mutants crates/vocab/src/implication.rs line 389
mutants crates/vocab/src/implication.rs line 390
mutants crates/vocab/src/implication.rs line 399
mutants crates/vocab/src/implication.rs line 419
mutants crates/vocab/src/implication.rs line 437
mutants crates/vocab/src/table.rs a_name_held_by_two_rows_resolves_to_the_lower_row 1874-1927
mutants crates/vocab/src/table.rs every_name_is_found_by_its_index_and_no_other_token_is 1929-1980
mutants crates/vocab/tests/expression_search.rs grammar_prefix_equals_independent_nested_infix_enumeration_without_duplicates 51-89
mutants crates/vocab/tests/mask.rs adding_a_required_bit_can_only_remove_hits_at_every_position 293-329
mutants crates/vocab/tests/no_float.rs line 183
mutants crates/vocab/tests/no_float.rs line 199
mutants crates/vocab/tests/no_float.rs rs_names 184-198
mutants crates/vocab/tests/no_float.rs the_listing_keeps_rust_sources_only_in_any_case_of_the_extension 200-217
mutants crates/vocab/tests/no_float.rs the_source_list_matches_the_directory 151-182
mutants crates/vocab/tests/no_py_language.rs a_host_that_cannot_run_git_walks_the_tree_and_the_git_proofs_skip_by_name 865-894
mutants crates/vocab/tests/no_py_language.rs a_listed_path_that_cannot_be_inspected_is_a_failure_not_an_absence 853-863
mutants crates/vocab/tests/no_py_language.rs a_planted_tree_is_refused_through_the_same_walk_the_gate_takes 614-652
mutants crates/vocab/tests/no_py_language.rs absent_line 404-419
mutants crates/vocab/tests/no_py_language.rs audit 340-368
mutants crates/vocab/tests/no_py_language.rs extreme_inputs_are_read_whole_through_the_gates_own_walk 755-770
mutants crates/vocab/tests/no_py_language.rs extreme_inputs_under 772-831
mutants crates/vocab/tests/no_py_language.rs files_to_read 257-295
mutants crates/vocab/tests/no_py_language.rs git_as 211-222
mutants crates/vocab/tests/no_py_language.rs line 206
mutants crates/vocab/tests/no_py_language.rs line 207
mutants crates/vocab/tests/no_py_language.rs line 208
mutants crates/vocab/tests/no_py_language.rs line 209
mutants crates/vocab/tests/no_py_language.rs line 210
mutants crates/vocab/tests/no_py_language.rs line 420
mutants crates/vocab/tests/no_py_language.rs line 55
mutants crates/vocab/tests/no_py_language.rs line 653
mutants crates/vocab/tests/no_py_language.rs line 771
mutants crates/vocab/tests/no_py_language.rs line 832
mutants crates/vocab/tests/no_py_language.rs line 852
mutants crates/vocab/tests/no_py_language.rs line 864
mutants crates/vocab/tests/no_py_language.rs no_tracked_file_is_runs_or_names_the_py_language 440-450
mutants crates/vocab/tests/no_py_language.rs the_absent_file_line_counts_every_one_shows_ten_and_masks_them 833-851
mutants crates/vocab/tests/no_py_language.rs the_git_half_of_the_walk 654-696
mutants crates/vocab/tests/no_runtime_spawn.rs a_pass_records_what_the_rarer_placements_hold 4255-4319
mutants crates/vocab/tests/no_runtime_spawn.rs found 4055-4061
mutants crates/vocab/tests/no_runtime_spawn.rs items_and_macro_inputs_end_where_the_model_says 4151-4220
mutants crates/vocab/tests/no_runtime_spawn.rs kinds 4037-4040
mutants crates/vocab/tests/no_runtime_spawn.rs line 4024
mutants crates/vocab/tests/no_runtime_spawn.rs line 4025
mutants crates/vocab/tests/no_runtime_spawn.rs line 4026
mutants crates/vocab/tests/no_runtime_spawn.rs line 4027
mutants crates/vocab/tests/no_runtime_spawn.rs line 4028
mutants crates/vocab/tests/no_runtime_spawn.rs line 4029
mutants crates/vocab/tests/no_runtime_spawn.rs line 4030
mutants crates/vocab/tests/no_runtime_spawn.rs line 4031
mutants crates/vocab/tests/no_runtime_spawn.rs line 4036
mutants crates/vocab/tests/no_runtime_spawn.rs line 4041
mutants crates/vocab/tests/no_runtime_spawn.rs line 4054
mutants crates/vocab/tests/no_runtime_spawn.rs line 4062
mutants crates/vocab/tests/no_runtime_spawn.rs line 4105
mutants crates/vocab/tests/no_runtime_spawn.rs line 4150
mutants crates/vocab/tests/no_runtime_spawn.rs line 4221
mutants crates/vocab/tests/no_runtime_spawn.rs line 4254
mutants crates/vocab/tests/no_runtime_spawn.rs line 4320
mutants crates/vocab/tests/no_runtime_spawn.rs line 4412
mutants crates/vocab/tests/no_runtime_spawn.rs manifests_are_read_as_toml_reads_them 4321-4411
mutants crates/vocab/tests/no_runtime_spawn.rs scanned 4042-4053
mutants crates/vocab/tests/no_runtime_spawn.rs the_cfg_and_attribute_readers_answer_at_their_edges 4106-4149
mutants crates/vocab/tests/no_runtime_spawn.rs the_lexer_reads_the_rarer_spellings_as_rustc_does 4063-4104
mutants crates/vocab/tests/no_runtime_spawn.rs the_needles_readers_answer_at_their_edges 4222-4253
mutants crates/vocab/tests/no_runtime_spawn.rs the_verdict_is_silent_where_the_model_has_nothing_to_refuse 4413-4453
mutants crates/vocab/tests/no_runtime_spawn.rs toks 4032-4035
mutants crates/vocab/tests/workspace_is_rust.rs every_comparator_in_the_table_is_read_and_others_are_refused 1539-1573
mutants crates/vocab/tests/workspace_is_rust.rs line 1574
mutants crates/vocab/tests/workspace_is_rust.rs line 43
mutants crates/vocab/tests/workspace_is_rust.rs reachable_from_workspace 276-328
mutants crates/vocab/tests/workspace_is_rust.rs semver_precedence_is_the_specs 1389-1439
mutants crates/vocab/tests/workspace_is_rust.rs the_lock_parser_keeps_what_it_understands_and_skips_the_rest 1575-1607
`````
