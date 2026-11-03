# deep-BASE-COV-LIBS-LS4-bcbc6524bdb2-set.txt.md

BASE-COV-LIBS-LS4's D3 set file at code tip `bcbc6524bdb29351f5d211aed11e4f7b76795b7b` (recipe v4 step 2, SET OVERRIDE: the five crates whole for coverage; the mutants lines are the D2 set of `f01d7a0e4...bcbc6524bdb2` in these crates, for the deferred background pass). One fenced block below holds it byte for byte; sha256 of the fenced content: `724bed6b9bcead559928f20e1d2b8b809d71afc2039e08231d1eaef2e51b8b60`.

`````text
reading whole:core,costs,engine,greeks,telemetry
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
mutants crates/core/src/alias.rs the_const_row_builders_give_the_rows_the_tables_hold 1046-1079
mutants crates/core/src/alias.rs the_shipped_lineage_has_no_cycle_and_no_duplicate 1494-1524
mutants crates/core/src/alias.rs line 1080
mutants crates/core/src/alias.rs line 1901
mutants crates/core/src/alias.rs line 1902
mutants crates/core/src/alias.rs line 1903
mutants crates/core/src/alias.rs line 1904
mutants crates/core/src/alias_cover_tests.rs whole
mutants crates/core/src/secret_shape.rs the_mixed_test_wants_a_letter_and_a_digit_in_one_run 219-234
mutants crates/core/src/secret_shape.rs a_hyphen_or_underscore_after_a_field_is_part_of_its_name 236-243
mutants crates/core/src/secret_shape.rs line 235
mutants crates/core/src/secret_shape.rs line 244
mutants crates/core/src/universe.rs worst_probe 4815-4848
mutants crates/core/src/vendor.rs the_series_tables_probe_in_bounded_time 3037-3112
mutants crates/core/tests/cost_invariants.rs bench_sources 95-103
mutants crates/core/tests/cost_invariants.rs bench_sources_in 105-139
mutants crates/core/tests/cost_invariants.rs every_bench_source 141-156
mutants crates/core/tests/cost_invariants.rs every_bench_source_in 158-191
mutants crates/core/tests/cost_invariants.rs declared_rows 228-250
mutants crates/core/tests/cost_invariants.rs scratch 423-430
mutants crates/core/tests/cost_invariants.rs the_bench_walks_pass_over_what_is_not_a_crates_bench 432-464
mutants crates/core/tests/cost_invariants.rs the_id_row_and_reach_readers_hold_at_their_edges 466-507
mutants crates/core/tests/cost_invariants.rs line 70
mutants crates/core/tests/cost_invariants.rs line 104
mutants crates/core/tests/cost_invariants.rs line 157
mutants crates/core/tests/cost_invariants.rs line 422
mutants crates/core/tests/cost_invariants.rs line 431
mutants crates/core/tests/cost_invariants.rs line 465
mutants crates/core/tests/decision_citations.rs named 258-266
mutants crates/core/tests/decision_citations.rs every_cited_decision_heads_an_entry_or_is_named_pending 288-333
mutants crates/core/tests/decision_citations.rs the_reader_resolves_entries_and_pending_names_and_nothing_else 335-458
mutants crates/core/tests/decision_citations.rs the_walk_skips_build_output_and_hidden_folders 460-513
mutants crates/core/tests/decision_citations.rs line 267
mutants crates/core/tests/decision_citations.rs line 459
mutants crates/core/tests/findings.rs every_killed_finding_carries_its_reason 341-374
mutants crates/core/tests/findings.rs every_refuted_finding_carries_its_reason 440-453
mutants crates/core/tests/findings.rs refuted_without_reason 455-464
mutants crates/core/tests/findings.rs only_a_tree_with_no_git_dir_skips_the_history 511-569
mutants crates/core/tests/findings.rs the_empty_root_is_asked_in_the_working_directory_unfenced 571-616
mutants crates/core/tests/findings.rs row 618-625
mutants crates/core/tests/findings.rs the_history_checks_skip_a_bare_tree_and_the_readers_hold_at_their_edges 627-688
mutants crates/core/tests/findings.rs repository_root 690-697
mutants crates/core/tests/findings.rs fixed_sha 699-708
mutants crates/core/tests/findings.rs fixed_commits 710-716
mutants crates/core/tests/findings.rs every_named_commit_exists 718-734
mutants crates/core/tests/findings.rs named_commits_exist 736-793
mutants crates/core/tests/findings.rs every_named_commit_is_in_this_branchs_history 795-821
mutants crates/core/tests/findings.rs named_commits_in_history 823-881
mutants crates/core/tests/findings.rs every_named_commit_is_on_main 883-904
mutants crates/core/tests/findings.rs named_commits_on_main 906-960
mutants crates/core/tests/findings.rs where_git_cannot_run_the_history_is_skipped_by_name 962-1004
mutants crates/core/tests/findings.rs line 45
mutants crates/core/tests/findings.rs line 454
mutants crates/core/tests/findings.rs line 570
mutants crates/core/tests/findings.rs line 617
mutants crates/core/tests/findings.rs line 626
mutants crates/core/tests/findings.rs line 689
mutants crates/core/tests/findings.rs line 698
mutants crates/core/tests/findings.rs line 709
mutants crates/core/tests/findings.rs line 961
mutants crates/core/tests/graph.rs the_key_and_inline_table_readers_hold_at_shapes_no_manifest_writes 873-928
mutants crates/core/tests/graph.rs line 872
mutants crates/core/tests/lint.rs no_float_in_price 83-183
mutants crates/core/tests/lint.rs floats_off_the_boundary 185-225
mutants crates/core/tests/lint.rs a_float_off_the_boundary_is_named_and_one_on_its_way_is_not 227-251
mutants crates/core/tests/lint.rs line 184
mutants crates/core/tests/lint.rs line 226
mutants crates/costs/src/dated.rs str_eq_answers_exactly_as_the_operator_would 637-661
mutants crates/costs/src/ledger.rs witness 756-763
mutants crates/costs/src/ledger.rs neither_type_has_a_default 771-777
mutants crates/costs/src/ledger.rs the_no_default_witness_is_the_every_type_item_for_both_checked_types 1706-1717
mutants crates/costs/src/ledger.rs line 778
mutants crates/costs/src/ledger.rs line 779
mutants crates/costs/src/ledger.rs line 1705
mutants crates/costs/src/lot.rs from_rows 900-967
mutants crates/costs/src/lot.rs compiled_rows 1042-1098
mutants crates/costs/src/lot_table_tests.rs time_lookups 322-340
mutants crates/costs/src/lot_table_tests.rs picks 342-347
mutants crates/costs/src/lot_table_tests.rs the_lookup_timer_picks_the_named_samples_and_times_a_table 349-378
mutants crates/costs/src/lot_table_tests.rs the_wire_words_the_absence_text_and_the_grouped_rows_hold_at_their_extremes 409-505
mutants crates/costs/src/lot_table_tests.rs each_sourced_row_carries_its_own_circulars_digest_and_the_row_cap_is_exact 507-558
mutants crates/costs/src/lot_table_tests.rs line 341
mutants crates/costs/src/lot_table_tests.rs line 379
mutants crates/costs/src/lot_table_tests.rs line 408
mutants crates/costs/src/lot_table_tests.rs line 506
mutants crates/costs/src/moneyness.rs the_offset_matches_a_nearest_multiple_search_across_four_whole_steps 531-560
mutants crates/costs/src/regime.rs the_validation_rejects_every_way_a_table_can_be_wrong 969-1074
mutants crates/costs/src/schedule.rs line 1830
mutants crates/costs/src/schedule.rs line 1831
mutants crates/costs/src/schedule.rs line 1832
mutants crates/costs/src/schedule.rs line 1833
mutants crates/costs/src/schedule_cover_tests.rs whole
mutants crates/costs/src/stack.rs rounding_to_the_rupee_holds_at_both_ends_of_the_type 2962-3004
mutants crates/costs/src/stack.rs line 2961
mutants crates/costs/src/stack.rs line 3006
mutants crates/costs/src/stack.rs line 3007
mutants crates/costs/src/stack.rs line 3008
mutants crates/costs/src/stack.rs line 3009
mutants crates/costs/src/stack_cover_tests.rs whole
mutants crates/costs/src/trip.rs a_printed_extreme_sell_below_zero_is_refused_and_does_not_overflow 3226-3266
mutants crates/costs/tests/adversarial.rs every_extreme_notional_against_every_shipped_rate_holds_the_money_invariants 82-219
mutants crates/costs/tests/adversarial.rs a_larger_notional_is_never_charged_less_than_a_smaller_one 221-280
mutants crates/costs/tests/adversarial.rs rounding_the_top_of_the_type_up_to_a_rupee_refuses_rather_than_wraps 282-329
mutants crates/costs/tests/dhan_calculator.rs shown 728-738
mutants crates/costs/tests/dhan_calculator.rs every_golden_case_has_a_precise_figure_or_a_named_refusal 740-763
mutants crates/costs/tests/dhan_calculator.rs a_case_with_no_rate_card_is_refused_by_every_precise_helper 1223-1284
mutants crates/costs/tests/dhan_calculator.rs line 739
mutants crates/costs/tests/dhan_calculator.rs line 1222
mutants crates/costs/tests/fa73061.rs the_timing_helpers_pick_the_named_samples_and_call_every_operation 620-650
mutants crates/costs/tests/fa73061.rs line 619
mutants crates/engine/src/column.rs vertical_reference 286-321
mutants crates/engine/src/keep.rs offer 353-407
mutants crates/engine/src/keep.rs the_heap_oracle_refuses_a_broken_heap 589-616
mutants crates/engine/src/keep.rs line 617
mutants crates/engine/src/lib.rs reserved 204-217
mutants crates/engine/src/lib.rs the_manifest_reader_holds_at_the_edges_of_its_string_grammar 3901-3947
mutants crates/engine/src/lib.rs line 2522
mutants crates/engine/src/lib.rs line 2523
mutants crates/engine/src/lib.rs line 2524
mutants crates/engine/src/lib.rs line 2525
mutants crates/engine/src/lib.rs line 3948
mutants crates/engine/src/refusal_cover_tests.rs whole
mutants crates/engine/tests/adversarial.rs one 72-87
mutants crates/engine/tests/adversarial.rs every_one_bit_fixture_holds_one_live_bit 366-389
mutants crates/engine/tests/adversarial.rs line 365
mutants crates/engine/tests/join_probe_screen.rs whole
mutants crates/engine/tests/production_callers.rs rust_files_under 44-58
mutants crates/engine/tests/production_callers.rs sources 60-81
mutants crates/engine/tests/production_callers.rs nothing_after_an_engine_files_first_test_gate_ships 163-183
mutants crates/engine/tests/production_callers.rs ungated_after_the_first_gate 185-223
mutants crates/engine/tests/production_callers.rs an_item_after_the_first_gate_without_its_own_is_named 328-378
mutants crates/engine/tests/production_callers.rs the_source_walk_reads_rust_at_every_depth_and_nothing_else 380-402
mutants crates/engine/tests/production_callers.rs line 59
mutants crates/engine/tests/production_callers.rs line 184
mutants crates/engine/tests/production_callers.rs line 327
mutants crates/engine/tests/production_callers.rs line 379
mutants crates/engine/tests/resume_readiness.rs resumed_identity_column_offers_and_every_configuration_term_must_match 256-307
mutants crates/engine/tests/resume_readiness.rs the_oracle_combines_only_positions_a_ladder_can_keep 1063-1087
mutants crates/engine/tests/resume_readiness.rs the_failing_wrapper_forwards_a_flush 1089-1104
mutants crates/engine/tests/resume_readiness.rs line 5
mutants crates/engine/tests/resume_readiness.rs line 6
mutants crates/engine/tests/resume_readiness.rs line 7
mutants crates/engine/tests/resume_readiness.rs line 1062
mutants crates/engine/tests/resume_readiness.rs line 1088
mutants crates/telemetry/src/clock.rs now_millis 34-38
mutants crates/telemetry/src/clock.rs millis_since_epoch 40-51
mutants crates/telemetry/src/clock.rs push_padded 125-159
mutants crates/telemetry/src/clock.rs an_instant_either_side_of_the_epoch_routes_to_its_signed_millis 184-211
mutants crates/telemetry/src/clock.rs line 39
mutants crates/telemetry/src/clock.rs line 180
mutants crates/telemetry/src/clock.rs line 181
mutants crates/telemetry/src/clock.rs line 212
mutants crates/telemetry/src/lib.rs where_permission_binds 396-426
mutants crates/telemetry/src/record.rs a_record_that_lost_a_different_number_of_fields_does_not_match 397-410
mutants crates/telemetry/src/record.rs line 411
mutants crates/telemetry/src/sink.rs count_code 915-936
mutants crates/telemetry/src/sink.rs seed_aside 3765-3855
mutants crates/telemetry/src/sink.rs seed_state 4014-4115
mutants crates/telemetry/src/sink.rs the_keep_nothing_doubles_accept_every_call_of_the_trait 4803-4826
mutants crates/telemetry/src/sink.rs a_root_that_is_not_there_holds_no_files 8029-8032
mutants crates/telemetry/src/sink.rs reaped 8077-8102
mutants crates/telemetry/src/sink.rs every_line 10265-10280
mutants crates/telemetry/src/sink.rs line 4827
mutants crates/telemetry/src/sink.rs line 8033
mutants crates/telemetry/src/sink.rs line 11695
mutants crates/telemetry/src/sink.rs line 11696
mutants crates/telemetry/src/sink.rs line 11697
mutants crates/telemetry/src/sink.rs line 11698
mutants crates/telemetry/src/sink_recovery_tests.rs whole
mutants crates/telemetry/src/tail.rs walked 365-454
mutants crates/telemetry/tests/global_absent.rs admits_with_no_sink_is_false_and_the_macro_evaluates_nothing 75-119
`````
