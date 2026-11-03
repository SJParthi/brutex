# deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt.md

Source: `state/fix/round2/deep-BASE-COV-LIBS-ee65c9f6fcbd-set.txt` on the Mac. One fenced block below holds it byte for byte; its sha256 is in MANIFEST.md.

`````text
reading whole:core,costs,engine,greeks,indicators,lake,runner,store,telemetry,vocab
coverage crates/core/benches/ratio.rs whole
coverage crates/core/src/alias.rs whole
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
coverage crates/costs/src/scope.rs whole
coverage crates/costs/src/stack.rs whole
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
coverage crates/engine/src/resume.rs whole
coverage crates/engine/tests/adversarial.rs whole
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
coverage crates/indicators/src/daily.rs whole
coverage crates/indicators/src/evaluator.rs whole
coverage crates/indicators/src/fib.rs whole
coverage crates/indicators/src/gap.rs whole
coverage crates/indicators/src/lib.rs whole
coverage crates/indicators/src/orb.rs whole
coverage crates/indicators/src/pattern.rs whole
coverage crates/indicators/src/session.rs whole
coverage crates/indicators/src/trend.rs whole
coverage crates/indicators/src/vwap.rs whole
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
coverage crates/runner/src/resample.rs whole
coverage crates/runner/src/research_admission_projection.rs whole
coverage crates/runner/src/research_admission_projection_codec.rs whole
coverage crates/runner/src/research_exit_grid.rs whole
coverage crates/runner/src/research_exit_grid_tests.rs whole
coverage crates/runner/src/research_family.rs whole
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
coverage crates/runner/tests/auto_reporting.rs whole
coverage crates/runner/tests/base_cov_admission_words.rs whole
coverage crates/runner/tests/d_0694_record.rs whole
coverage crates/runner/tests/end_to_end.rs whole
coverage crates/runner/tests/expression_pricing.rs whole
coverage crates/runner/tests/fib_rung_sweep.rs whole
coverage crates/runner/tests/fixed_deadline_readiness.rs whole
coverage crates/runner/tests/foreign_slice_deadline_readiness.rs whole
coverage crates/runner/tests/hole_after_exit.rs whole
coverage crates/runner/tests/join_answer_is_unchanged.rs whole
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
coverage crates/store/src/checksum_audit_tests.rs whole
coverage crates/store/src/crc.rs whole
coverage crates/store/src/emits.rs whole
coverage crates/store/src/file.rs whole
coverage crates/store/src/file_fault_tests.rs whole
coverage crates/store/src/file_sealed.rs whole
coverage crates/store/src/file_sealed_tests.rs whole
coverage crates/store/src/file_write_door_tests.rs whole
coverage crates/store/src/flock.rs whole
coverage crates/store/src/flock_patient_tests.rs whole
coverage crates/store/src/flock_tests.rs whole
coverage crates/store/src/format.rs whole
coverage crates/store/src/generation.rs whole
coverage crates/store/src/generation_cover_tests.rs whole
coverage crates/store/src/generation_tests.rs whole
coverage crates/store/src/header.rs whole
coverage crates/store/src/header_audit_tests.rs whole
coverage crates/store/src/layout.rs whole
coverage crates/store/src/lib.rs whole
coverage crates/store/src/month_summary.rs whole
coverage crates/store/src/month_summary_cover_tests.rs whole
coverage crates/store/src/month_summary_tests.rs whole
coverage crates/store/src/month_summary_writer.rs whole
coverage crates/store/src/open_flags.rs whole
coverage crates/store/src/path.rs whole
coverage crates/store/src/provenance.rs whole
coverage crates/store/src/quarantine.rs whole
coverage crates/store/src/quarantine_cover_tests.rs whole
coverage crates/store/src/quarantine_tests.rs whole
coverage crates/store/src/repair.rs whole
coverage crates/store/src/repair_cover_tests.rs whole
coverage crates/store/src/repair_tests.rs whole
coverage crates/store/src/safe_open.rs whole
coverage crates/store/src/safe_open_tests.rs whole
coverage crates/store/src/sealed.rs whole
coverage crates/store/src/sealed_codec.rs whole
coverage crates/store/src/sealed_codec_tests.rs whole
coverage crates/store/src/sealed_tests.rs whole
coverage crates/store/src/settings.rs whole
coverage crates/store/src/settings_cover_tests.rs whole
coverage crates/store/src/settings_tests.rs whole
coverage crates/store/src/sidecar_checksum_tests.rs whole
coverage crates/store/src/small_month_tests.rs whole
coverage crates/store/src/sort_index.rs whole
coverage crates/store/src/sort_index_cover_tests.rs whole
coverage crates/store/src/sort_index_tests.rs whole
coverage crates/store/src/sort_index_writer.rs whole
coverage crates/store/src/sort_index_writer_tests.rs whole
coverage crates/store/src/staging.rs whole
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
mutants crates/core/src/alias.rs the_const_row_builders_give_the_rows_the_tables_hold 1046-1079
mutants crates/core/src/alias.rs the_shipped_lineage_has_no_cycle_and_no_duplicate 1494-1524
mutants crates/core/src/alias.rs line 1080
mutants crates/core/src/universe.rs worst_probe 4815-4848
mutants crates/core/src/vendor.rs the_series_tables_probe_in_bounded_time 3037-3112
mutants crates/costs/src/dated.rs str_eq_answers_exactly_as_the_operator_would 637-661
mutants crates/costs/src/ledger.rs witness 756-763
mutants crates/costs/src/ledger.rs the_no_default_witness_is_the_every_type_item_for_both_checked_types 1701-1711
mutants crates/costs/src/ledger.rs line 1700
mutants crates/costs/src/lot.rs from_rows 900-967
mutants crates/costs/src/lot.rs compiled_rows 1042-1098
mutants crates/costs/src/lot_table_tests.rs the_wire_words_the_absence_text_and_the_grouped_rows_hold_at_their_extremes 373-469
mutants crates/costs/src/lot_table_tests.rs each_sourced_row_carries_its_own_circulars_digest_and_the_row_cap_is_exact 471-522
mutants crates/costs/src/lot_table_tests.rs line 372
mutants crates/costs/src/lot_table_tests.rs line 470
mutants crates/costs/src/moneyness.rs the_offset_matches_a_nearest_multiple_search_across_four_whole_steps 531-560
mutants crates/costs/src/regime.rs the_validation_rejects_every_way_a_table_can_be_wrong 969-1074
mutants crates/costs/src/stack.rs rounding_to_the_rupee_holds_at_both_ends_of_the_type 2962-3004
mutants crates/costs/src/stack.rs line 2961
mutants crates/costs/src/trip.rs a_printed_extreme_sell_below_zero_is_refused_and_does_not_overflow 3226-3266
mutants crates/costs/tests/adversarial.rs every_extreme_notional_against_every_shipped_rate_holds_the_money_invariants 85-219
mutants crates/costs/tests/adversarial.rs a_larger_notional_is_never_charged_less_than_a_smaller_one 221-280
mutants crates/costs/tests/adversarial.rs rounding_the_top_of_the_type_up_to_a_rupee_refuses_rather_than_wraps 282-329
mutants crates/engine/src/lib.rs line 3889
mutants crates/engine/src/lib.rs line 3890
mutants crates/engine/src/lib.rs line 3891
mutants crates/engine/src/lib.rs line 3892
mutants crates/engine/src/lib.rs line 3893
mutants crates/engine/src/lib.rs line 3894
mutants crates/engine/src/lib.rs line 3895
mutants crates/engine/src/lib.rs line 3896
mutants crates/engine/src/lib.rs line 3897
mutants crates/engine/src/lib.rs line 3898
mutants crates/engine/src/lib.rs line 3899
mutants crates/engine/src/lib.rs line 3900
mutants crates/engine/src/lib.rs line 3901
mutants crates/engine/src/lib.rs line 3902
mutants crates/engine/src/lib.rs line 3903
mutants crates/engine/src/lib.rs line 3904
mutants crates/engine/src/lib.rs line 3905
mutants crates/engine/src/lib.rs line 3906
mutants crates/engine/src/lib.rs line 3907
mutants crates/engine/src/lib.rs line 3908
mutants crates/engine/src/lib.rs line 3909
mutants crates/engine/src/lib.rs line 3910
mutants crates/engine/src/lib.rs line 3911
mutants crates/engine/src/lib.rs line 3912
mutants crates/engine/src/lib.rs line 3913
mutants crates/engine/src/lib.rs line 3914
mutants crates/engine/src/lib.rs line 3915
mutants crates/engine/src/lib.rs line 3916
mutants crates/engine/src/lib.rs line 3917
mutants crates/engine/src/lib.rs line 3918
mutants crates/engine/src/lib.rs line 3919
mutants crates/engine/src/lib.rs line 3920
mutants crates/engine/src/lib.rs line 3921
mutants crates/engine/src/lib.rs line 3922
mutants crates/engine/src/lib.rs line 3923
mutants crates/engine/src/lib.rs line 3924
mutants crates/engine/src/lib.rs line 3925
mutants crates/engine/src/lib.rs line 3926
mutants crates/engine/src/lib.rs line 3927
mutants crates/engine/src/lib.rs line 3928
mutants crates/engine/src/lib.rs line 3929
mutants crates/engine/src/lib.rs line 3930
mutants crates/engine/src/lib.rs line 3931
mutants crates/engine/src/lib.rs line 3932
mutants crates/engine/src/lib.rs line 3933
mutants crates/engine/src/lib.rs line 3934
mutants crates/engine/src/lib.rs line 3935
mutants crates/engine/src/lib.rs line 3936
mutants crates/indicators/src/anchored.rs a_reference_reads_back_its_bar_and_the_census_refuses_each_imbalance 2783-2853
mutants crates/indicators/src/anchored.rs line 2782
mutants crates/indicators/src/column.rs checked_reprojection_records_execution_accumulator_refusal 2295-2356
mutants crates/indicators/src/pattern.rs last_mask 3214-3231
mutants crates/indicators/src/pattern.rs the_known_ladder_grows_with_depth_and_stops_at_the_pattern_block 3233-3259
mutants crates/indicators/src/pattern.rs each_shape_misses_by_its_one_condition_and_fires_without_it 3261-3304
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
mutants crates/indicators/tests/exact_minute_orb_gap_readiness.rs the_bridge_refuses_bad_lengths_empty_unaligned_and_repeated_minutes_and_a_short_signal 415-494
mutants crates/indicators/tests/exact_minute_orb_gap_readiness.rs line 414
mutants crates/lake/src/contract.rs parse_month 306-318
mutants crates/lake/src/contract.rs the_expiry_token_and_the_rendering_hold_at_their_edges 769-827
mutants crates/lake/src/contract.rs line 768
mutants crates/lake/src/reader.rs line 1945
mutants crates/lake/src/reader.rs line 1946
mutants crates/lake/src/reader.rs line 1947
mutants crates/lake/src/reader.rs line 1948
mutants crates/lake/src/reader_cover_tests.rs whole
mutants crates/lake/tests/synthetic.rs a_cash_file_decodes_to_paisa_and_the_layout_is_detected 214-247
mutants crates/lake/tests/synthetic.rs edit_pages 1239-1300
mutants crates/lake/tests/synthetic.rs a_page_header_the_reader_cannot_use_is_refused_by_name 1302-1381
mutants crates/lake/tests/synthetic.rs line 1235
mutants crates/lake/tests/synthetic.rs line 1236
mutants crates/lake/tests/synthetic.rs line 1237
mutants crates/lake/tests/synthetic.rs line 1238
mutants crates/lake/tests/synthetic.rs line 1301
mutants crates/runner/src/admission.rs evidence_joins_refuse_a_prefilled_authority_slot_and_a_foreign_trade_count 9567-9670
mutants crates/runner/src/admission.rs line 9560
mutants crates/runner/src/admission.rs line 9561
mutants crates/runner/src/admission.rs line 9562
mutants crates/runner/src/admission.rs line 9563
mutants crates/runner/src/admission.rs line 9564
mutants crates/runner/src/admission.rs line 9565
mutants crates/runner/src/admission.rs line 9566
mutants crates/runner/src/admission.rs line 9672
mutants crates/runner/src/admission.rs line 9673
mutants crates/runner/src/admission.rs line 9674
mutants crates/runner/src/admission.rs line 9675
mutants crates/runner/src/admission.rs line 9676
mutants crates/runner/src/admission.rs line 9677
mutants crates/runner/src/admission.rs line 9678
mutants crates/runner/src/admission.rs line 9679
mutants crates/runner/src/admission_reader_cover_tests.rs whole
mutants crates/runner/src/admission_statistics_cover_tests.rs whole
mutants crates/runner/src/exit_grid_policy.rs line 7731
mutants crates/runner/src/exit_grid_policy.rs line 7732
mutants crates/runner/src/exit_grid_policy.rs line 7733
mutants crates/runner/src/exit_grid_policy.rs line 7734
mutants crates/runner/src/exit_grid_policy_cover_tests.rs whole
mutants crates/runner/src/grid.rs line 11040
mutants crates/runner/src/grid.rs line 11041
mutants crates/runner/src/grid.rs line 11042
mutants crates/runner/src/grid.rs line 11043
mutants crates/runner/src/grid_cover_tests.rs whole
mutants crates/runner/src/pbo.rs relative 241-266
mutants crates/runner/src/significance.rs line 1390
mutants crates/runner/src/significance.rs line 1391
mutants crates/runner/src/significance.rs line 1392
mutants crates/runner/src/significance.rs line 1393
mutants crates/runner/src/significance_cover_tests.rs whole
mutants crates/runner/tests/base_cov_admission_words.rs whole
mutants crates/store/src/catalog.rs classify 432-509
mutants crates/store/src/catalog.rs line 427
mutants crates/store/src/catalog.rs line 428
mutants crates/store/src/catalog.rs line 429
mutants crates/store/src/catalog.rs line 550
mutants crates/store/src/catalog.rs line 551
mutants crates/store/src/catalog.rs line 552
mutants crates/store/src/catalog.rs line 553
mutants crates/store/src/catalog_cover_tests.rs whole
mutants crates/store/src/file.rs fmt 1247-1367
mutants crates/store/src/file.rs write_sealed 1370-1396
mutants crates/store/src/file.rs write_sealed_refused 1398-1412
mutants crates/store/src/file.rs line 1246
mutants crates/store/src/generation.rs encode 131-149
mutants crates/store/src/generation.rs put_field 189-198
mutants crates/store/src/generation.rs line 199
mutants crates/store/src/generation.rs line 633
mutants crates/store/src/generation.rs line 634
mutants crates/store/src/generation.rs line 635
mutants crates/store/src/generation.rs line 636
mutants crates/store/src/generation_cover_tests.rs whole
mutants crates/store/src/month_summary.rs line 1364
mutants crates/store/src/month_summary.rs line 1365
mutants crates/store/src/month_summary.rs line 1366
mutants crates/store/src/month_summary.rs line 1367
mutants crates/store/src/month_summary_cover_tests.rs whole
mutants crates/store/src/quarantine.rs field 711-717
mutants crates/store/src/quarantine.rs encode_slot 758-778
mutants crates/store/src/quarantine.rs write_at 780-790
mutants crates/store/src/quarantine.rs note_damaged 1397-1412
mutants crates/store/src/quarantine.rs note_cut 1414-1431
mutants crates/store/src/quarantine.rs refuse_damage 1909-1920
mutants crates/store/src/quarantine.rs empty_image 1955-1969
mutants crates/store/src/quarantine.rs seal_header 1971-1974
mutants crates/store/src/quarantine.rs put 2069-2090
mutants crates/store/src/quarantine.rs set_described 2092-2096
mutants crates/store/src/quarantine.rs level 2141-2175
mutants crates/store/src/quarantine.rs load 2335-2391
mutants crates/store/src/quarantine.rs line 791
mutants crates/store/src/quarantine.rs line 2773
mutants crates/store/src/quarantine.rs line 2774
mutants crates/store/src/quarantine.rs line 2775
mutants crates/store/src/quarantine.rs line 2776
mutants crates/store/src/quarantine_cover_tests.rs whole
mutants crates/store/src/quarantine_tests.rs every_quarantine_refusal_renders_its_code_its_path_and_its_figures 2002-2146
mutants crates/store/src/quarantine_tests.rs line 1994
mutants crates/store/src/quarantine_tests.rs line 1995
mutants crates/store/src/quarantine_tests.rs line 1996
mutants crates/store/src/quarantine_tests.rs line 1997
mutants crates/store/src/quarantine_tests.rs line 1998
mutants crates/store/src/quarantine_tests.rs line 1999
mutants crates/store/src/quarantine_tests.rs line 2000
mutants crates/store/src/quarantine_tests.rs line 2001
mutants crates/store/src/repair.rs line 1527
mutants crates/store/src/repair.rs line 1528
mutants crates/store/src/repair.rs line 1529
mutants crates/store/src/repair.rs line 1530
mutants crates/store/src/repair_cover_tests.rs whole
mutants crates/store/src/settings.rs line 1464
mutants crates/store/src/settings.rs line 1465
mutants crates/store/src/settings.rs line 1466
mutants crates/store/src/settings.rs line 1467
mutants crates/store/src/settings_cover_tests.rs whole
mutants crates/store/src/sort_index.rs build_run_as 720-811
mutants crates/store/src/sort_index.rs decode_run_header 865-968
mutants crates/store/src/sort_index.rs take 1512-1520
mutants crates/store/src/sort_index.rs put 1522-1531
mutants crates/store/src/sort_index.rs line 1536
mutants crates/store/src/sort_index.rs line 1537
mutants crates/store/src/sort_index.rs line 1538
mutants crates/store/src/sort_index.rs line 1539
mutants crates/store/src/sort_index_cover_tests.rs whole
mutants crates/store/tests/base_cov_words.rs whole
mutants crates/telemetry/src/clock.rs now_millis 34-38
mutants crates/telemetry/src/clock.rs millis_since_epoch 40-51
mutants crates/telemetry/src/clock.rs an_instant_either_side_of_the_epoch_routes_to_its_signed_millis 188-215
mutants crates/telemetry/src/clock.rs line 39
mutants crates/telemetry/src/clock.rs line 184
mutants crates/telemetry/src/clock.rs line 185
mutants crates/telemetry/src/clock.rs line 216
mutants crates/telemetry/src/sink.rs the_keep_nothing_doubles_accept_every_call_of_the_trait 4783-4806
mutants crates/telemetry/src/sink.rs line 4807
`````
