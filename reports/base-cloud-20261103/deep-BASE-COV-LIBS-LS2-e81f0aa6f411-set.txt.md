# deep-BASE-COV-LIBS-LS2-e81f0aa6f411-set.txt.md

The D3 set file of the final Route B of shard LS2 (crate runner) at code tip e81f0aa6f41159fc9a155c9deb249fa2798daf2e: `reading whole:runner`, every runner .rs file at that tip as `coverage <file> whole`, and the D2 mutants lines of this shard's own diff from START bd04fb299d23 (the functions it added or changed, whole; added lines outside functions; the one added file whole), for the deferred background mutants pass. One fenced block below holds it byte for byte; its sha256 is e535e5a00aaf73f52cead2305d7bc95a19a0aa8cb176e792ae4b2c6e9f5e9a1e.

`````text
reading whole:runner
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
mutants crates/runner/src/admission.rs an_evidence_record_names_each_malformed_probability_field 9677-9752
mutants crates/runner/src/admission.rs an_evidence_record_whose_authority_slot_is_not_its_projection_is_refused 9759-9780
mutants crates/runner/src/admission.rs line 9671
mutants crates/runner/src/admission.rs line 9672
mutants crates/runner/src/admission.rs line 9673
mutants crates/runner/src/admission.rs line 9674
mutants crates/runner/src/admission.rs line 9675
mutants crates/runner/src/admission.rs line 9676
mutants crates/runner/src/admission.rs line 9753
mutants crates/runner/src/admission.rs line 9754
mutants crates/runner/src/admission.rs line 9755
mutants crates/runner/src/admission.rs line 9756
mutants crates/runner/src/admission.rs line 9757
mutants crates/runner/src/admission.rs line 9758
mutants crates/runner/src/audit.rs the_integer_renderers_sign_negatives_and_group_lakhs_and_crores 3562-3573
mutants crates/runner/src/audit.rs a_trail_only_cell_is_named_by_its_trail_and_not_as_the_baseline 3578-3585
mutants crates/runner/src/audit.rs a_grid_with_refused_paths_prints_the_count_and_why 3590-3604
mutants crates/runner/src/audit.rs a_refused_walk_forward_prints_its_reason_and_no_fold 3609-3622
mutants crates/runner/src/audit.rs an_empty_exit_grid_renders_no_strategy_report 3627-3652
mutants crates/runner/src/audit.rs the_marked_rows_below_the_cut_keep_key_order_when_best_comes_first 3659-3683
mutants crates/runner/src/audit.rs an_overnight_from_a_close_at_or_below_zero_is_not_measured 3992-4008
mutants crates/runner/src/audit.rs an_upward_traded_overnight_is_signed_with_a_plus 4012-4027
mutants crates/runner/src/audit.rs line 3558
mutants crates/runner/src/audit.rs line 3559
mutants crates/runner/src/audit.rs line 3560
mutants crates/runner/src/audit.rs line 3561
mutants crates/runner/src/audit.rs line 3574
mutants crates/runner/src/audit.rs line 3575
mutants crates/runner/src/audit.rs line 3576
mutants crates/runner/src/audit.rs line 3577
mutants crates/runner/src/audit.rs line 3586
mutants crates/runner/src/audit.rs line 3587
mutants crates/runner/src/audit.rs line 3588
mutants crates/runner/src/audit.rs line 3589
mutants crates/runner/src/audit.rs line 3605
mutants crates/runner/src/audit.rs line 3606
mutants crates/runner/src/audit.rs line 3607
mutants crates/runner/src/audit.rs line 3608
mutants crates/runner/src/audit.rs line 3623
mutants crates/runner/src/audit.rs line 3624
mutants crates/runner/src/audit.rs line 3625
mutants crates/runner/src/audit.rs line 3626
mutants crates/runner/src/audit.rs line 3653
mutants crates/runner/src/audit.rs line 3654
mutants crates/runner/src/audit.rs line 3655
mutants crates/runner/src/audit.rs line 3656
mutants crates/runner/src/audit.rs line 3657
mutants crates/runner/src/audit.rs line 3658
mutants crates/runner/src/audit.rs line 3987
mutants crates/runner/src/audit.rs line 3988
mutants crates/runner/src/audit.rs line 3989
mutants crates/runner/src/audit.rs line 3990
mutants crates/runner/src/audit.rs line 3991
mutants crates/runner/src/audit.rs line 4009
mutants crates/runner/src/audit.rs line 4010
mutants crates/runner/src/audit.rs line 4011
mutants crates/runner/src/bootstrap.rs a_draw_below_zero_answers_zero_without_advancing_the_stream 3741-3746
mutants crates/runner/src/bootstrap.rs an_empty_series_summarises_to_nothing 3751-3761
mutants crates/runner/src/bootstrap.rs zero_draws_reject_nothing_at_the_legacy_door 3767-3776
mutants crates/runner/src/bootstrap.rs line 3731
mutants crates/runner/src/bootstrap.rs line 3732
mutants crates/runner/src/bootstrap.rs line 3733
mutants crates/runner/src/bootstrap.rs line 3734
mutants crates/runner/src/bootstrap.rs line 3735
mutants crates/runner/src/bootstrap.rs line 3736
mutants crates/runner/src/bootstrap.rs line 3737
mutants crates/runner/src/bootstrap.rs line 3738
mutants crates/runner/src/bootstrap.rs line 3739
mutants crates/runner/src/bootstrap.rs line 3740
mutants crates/runner/src/bootstrap.rs line 3747
mutants crates/runner/src/bootstrap.rs line 3748
mutants crates/runner/src/bootstrap.rs line 3749
mutants crates/runner/src/bootstrap.rs line 3750
mutants crates/runner/src/bootstrap.rs line 3762
mutants crates/runner/src/bootstrap.rs line 3763
mutants crates/runner/src/bootstrap.rs line 3764
mutants crates/runner/src/bootstrap.rs line 3765
mutants crates/runner/src/bootstrap.rs line 3766
mutants crates/runner/src/bootstrap.rs line 3777
mutants crates/runner/src/bootstrap_zero_v2_tests.rs cold_source_audit_reproduces_full_identity_and_statistics_without_resampling 20-87
mutants crates/runner/src/excursion.rs a_stepped_ladder_refuses_a_step_or_count_it_cannot_use 2652-2658
mutants crates/runner/src/excursion.rs a_ladder_of_zero_rungs_is_refused_from_any_sample 2662-2665
mutants crates/runner/src/excursion.rs a_torn_pair_of_running_extremes_reads_as_nothing_walked 2671-2689
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
mutants crates/runner/src/excursion.rs line 2648
mutants crates/runner/src/excursion.rs line 2649
mutants crates/runner/src/excursion.rs line 2650
mutants crates/runner/src/excursion.rs line 2651
mutants crates/runner/src/excursion.rs line 2659
mutants crates/runner/src/excursion.rs line 2660
mutants crates/runner/src/excursion.rs line 2661
mutants crates/runner/src/excursion.rs line 2666
mutants crates/runner/src/excursion.rs line 2667
mutants crates/runner/src/excursion.rs line 2668
mutants crates/runner/src/excursion.rs line 2669
mutants crates/runner/src/excursion.rs line 2670
mutants crates/runner/src/excursion.rs line 2690
mutants crates/runner/src/exit_grid_policy.rs test_column_with_thresholds 4662-4666
mutants crates/runner/src/exit_grid_policy.rs nifty 4709-4711
mutants crates/runner/src/exit_grid_policy.rs bank_nifty 4713-4715
mutants crates/runner/src/exit_grid_policy.rs symbol 4717-4719
mutants crates/runner/src/exit_grid_policy.rs canonical_instrument_digest_binds_every_structural_field 4722-4789
mutants crates/runner/src/exit_grid_policy.rs only_the_two_nse_spot_indices_resolve 4863-4908
mutants crates/runner/src/exit_grid_policy.rs a_training_series_of_another_length_cannot_be_attested 5174-5192
mutants crates/runner/src/exit_grid_policy.rs a_coordinate_visit_stops_where_its_visitor_refuses 5199-5225
mutants crates/runner/src/exit_grid_policy.rs candidate_universe_digest_binds_every_path_and_legacy_replay_stays_strict 7302-7391
mutants crates/runner/src/exit_grid_policy.rs replay_fixture 7751-7768
mutants crates/runner/src/exit_grid_policy.rs reseal_selected 7772-7784
mutants crates/runner/src/exit_grid_policy.rs price 7790-7795
mutants crates/runner/src/exit_grid_policy.rs resealed 7799-7810
mutants crates/runner/src/exit_grid_policy.rs oos_and_run 7813-7826
mutants crates/runner/src/exit_grid_policy.rs every_replay_door_refusal_names_its_cause 7834-7915
mutants crates/runner/src/exit_grid_policy.rs every_tear_of_a_replayed_universe_fails_its_integrity_gate 7921-8001
mutants crates/runner/src/exit_grid_policy.rs an_evaluation_and_its_validation_read_back_what_they_bind 8007-8048
mutants crates/runner/src/exit_grid_policy.rs every_input_validator_refuses_by_name 8059-8172
mutants crates/runner/src/exit_grid_policy.rs a_replay_run_of_another_feed_is_refused 8176-8194
mutants crates/runner/src/exit_grid_policy.rs every_selector_picks_a_policy_admitted_coordinate 8198-8223
mutants crates/runner/src/exit_grid_policy.rs a_resolution_past_its_axis_limit_is_refused_by_name 8230-8266
mutants crates/runner/src/exit_grid_policy.rs a_grid_view_refuses_what_is_not_its_population 8278-8419
mutants crates/runner/src/exit_grid_policy.rs torn 8435-8439
mutants crates/runner/src/exit_grid_policy.rs with_training_span 8443-8455
mutants crates/runner/src/exit_grid_policy.rs with_wider_stops 8459-8466
mutants crates/runner/src/exit_grid_policy.rs line 5171
mutants crates/runner/src/exit_grid_policy.rs line 5172
mutants crates/runner/src/exit_grid_policy.rs line 5173
mutants crates/runner/src/exit_grid_policy.rs line 5193
mutants crates/runner/src/exit_grid_policy.rs line 5194
mutants crates/runner/src/exit_grid_policy.rs line 5195
mutants crates/runner/src/exit_grid_policy.rs line 5196
mutants crates/runner/src/exit_grid_policy.rs line 5197
mutants crates/runner/src/exit_grid_policy.rs line 5198
mutants crates/runner/src/exit_grid_policy.rs line 5226
mutants crates/runner/src/exit_grid_policy.rs line 7747
mutants crates/runner/src/exit_grid_policy.rs line 7748
mutants crates/runner/src/exit_grid_policy.rs line 7749
mutants crates/runner/src/exit_grid_policy.rs line 7750
mutants crates/runner/src/exit_grid_policy.rs line 7769
mutants crates/runner/src/exit_grid_policy.rs line 7770
mutants crates/runner/src/exit_grid_policy.rs line 7771
mutants crates/runner/src/exit_grid_policy.rs line 7785
mutants crates/runner/src/exit_grid_policy.rs line 7786
mutants crates/runner/src/exit_grid_policy.rs line 7787
mutants crates/runner/src/exit_grid_policy.rs line 7788
mutants crates/runner/src/exit_grid_policy.rs line 7789
mutants crates/runner/src/exit_grid_policy.rs line 7796
mutants crates/runner/src/exit_grid_policy.rs line 7797
mutants crates/runner/src/exit_grid_policy.rs line 7798
mutants crates/runner/src/exit_grid_policy.rs line 7811
mutants crates/runner/src/exit_grid_policy.rs line 7812
mutants crates/runner/src/exit_grid_policy.rs line 7827
mutants crates/runner/src/exit_grid_policy.rs line 7828
mutants crates/runner/src/exit_grid_policy.rs line 7829
mutants crates/runner/src/exit_grid_policy.rs line 7830
mutants crates/runner/src/exit_grid_policy.rs line 7831
mutants crates/runner/src/exit_grid_policy.rs line 7832
mutants crates/runner/src/exit_grid_policy.rs line 7833
mutants crates/runner/src/exit_grid_policy.rs line 7916
mutants crates/runner/src/exit_grid_policy.rs line 7917
mutants crates/runner/src/exit_grid_policy.rs line 7918
mutants crates/runner/src/exit_grid_policy.rs line 7919
mutants crates/runner/src/exit_grid_policy.rs line 7920
mutants crates/runner/src/exit_grid_policy.rs line 8002
mutants crates/runner/src/exit_grid_policy.rs line 8003
mutants crates/runner/src/exit_grid_policy.rs line 8004
mutants crates/runner/src/exit_grid_policy.rs line 8005
mutants crates/runner/src/exit_grid_policy.rs line 8006
mutants crates/runner/src/exit_grid_policy.rs line 8049
mutants crates/runner/src/exit_grid_policy.rs line 8050
mutants crates/runner/src/exit_grid_policy.rs line 8051
mutants crates/runner/src/exit_grid_policy.rs line 8052
mutants crates/runner/src/exit_grid_policy.rs line 8053
mutants crates/runner/src/exit_grid_policy.rs line 8054
mutants crates/runner/src/exit_grid_policy.rs line 8055
mutants crates/runner/src/exit_grid_policy.rs line 8056
mutants crates/runner/src/exit_grid_policy.rs line 8057
mutants crates/runner/src/exit_grid_policy.rs line 8058
mutants crates/runner/src/exit_grid_policy.rs line 8173
mutants crates/runner/src/exit_grid_policy.rs line 8174
mutants crates/runner/src/exit_grid_policy.rs line 8175
mutants crates/runner/src/exit_grid_policy.rs line 8195
mutants crates/runner/src/exit_grid_policy.rs line 8196
mutants crates/runner/src/exit_grid_policy.rs line 8197
mutants crates/runner/src/exit_grid_policy.rs line 8224
mutants crates/runner/src/exit_grid_policy.rs line 8225
mutants crates/runner/src/exit_grid_policy.rs line 8226
mutants crates/runner/src/exit_grid_policy.rs line 8227
mutants crates/runner/src/exit_grid_policy.rs line 8228
mutants crates/runner/src/exit_grid_policy.rs line 8229
mutants crates/runner/src/exit_grid_policy.rs line 8267
mutants crates/runner/src/exit_grid_policy.rs line 8268
mutants crates/runner/src/exit_grid_policy.rs line 8269
mutants crates/runner/src/exit_grid_policy.rs line 8270
mutants crates/runner/src/exit_grid_policy.rs line 8271
mutants crates/runner/src/exit_grid_policy.rs line 8272
mutants crates/runner/src/exit_grid_policy.rs line 8273
mutants crates/runner/src/exit_grid_policy.rs line 8274
mutants crates/runner/src/exit_grid_policy.rs line 8275
mutants crates/runner/src/exit_grid_policy.rs line 8276
mutants crates/runner/src/exit_grid_policy.rs line 8277
mutants crates/runner/src/exit_grid_policy.rs line 8425
mutants crates/runner/src/exit_grid_policy.rs line 8426
mutants crates/runner/src/exit_grid_policy.rs line 8427
mutants crates/runner/src/exit_grid_policy.rs line 8428
mutants crates/runner/src/exit_grid_policy.rs line 8429
mutants crates/runner/src/exit_grid_policy.rs line 8430
mutants crates/runner/src/exit_grid_policy.rs line 8431
mutants crates/runner/src/exit_grid_policy.rs line 8432
mutants crates/runner/src/exit_grid_policy.rs line 8433
mutants crates/runner/src/exit_grid_policy.rs line 8434
mutants crates/runner/src/exit_grid_policy.rs line 8440
mutants crates/runner/src/exit_grid_policy.rs line 8441
mutants crates/runner/src/exit_grid_policy.rs line 8442
mutants crates/runner/src/exit_grid_policy.rs line 8456
mutants crates/runner/src/exit_grid_policy.rs line 8457
mutants crates/runner/src/exit_grid_policy.rs line 8458
mutants crates/runner/src/exit_grid_policy.rs line 8467
mutants crates/runner/src/expression.rs malformed_columns_refuse_before_any_evidence_is_published 287-334
mutants crates/runner/src/expression_execution_tests.rs evaluated_fixture 332-358
mutants crates/runner/src/expression_execution_tests.rs every_run_door_and_read_face_answers_what_it_bound 365-457
mutants crates/runner/src/expression_execution_tests.rs every_expression_door_refuses_an_evaluation_that_is_not_its_own 467-572
mutants crates/runner/src/expression_execution_tests.rs line 325
mutants crates/runner/src/expression_execution_tests.rs line 326
mutants crates/runner/src/expression_execution_tests.rs line 327
mutants crates/runner/src/expression_execution_tests.rs line 328
mutants crates/runner/src/expression_execution_tests.rs line 329
mutants crates/runner/src/expression_execution_tests.rs line 330
mutants crates/runner/src/expression_execution_tests.rs line 331
mutants crates/runner/src/expression_execution_tests.rs line 359
mutants crates/runner/src/expression_execution_tests.rs line 360
mutants crates/runner/src/expression_execution_tests.rs line 361
mutants crates/runner/src/expression_execution_tests.rs line 362
mutants crates/runner/src/expression_execution_tests.rs line 363
mutants crates/runner/src/expression_execution_tests.rs line 364
mutants crates/runner/src/expression_execution_tests.rs line 458
mutants crates/runner/src/expression_execution_tests.rs line 459
mutants crates/runner/src/expression_execution_tests.rs line 460
mutants crates/runner/src/expression_execution_tests.rs line 461
mutants crates/runner/src/expression_execution_tests.rs line 462
mutants crates/runner/src/expression_execution_tests.rs line 463
mutants crates/runner/src/expression_execution_tests.rs line 464
mutants crates/runner/src/expression_execution_tests.rs line 465
mutants crates/runner/src/expression_execution_tests.rs line 466
mutants crates/runner/src/expression_validation_tests.rs every_fixed_training_authority_refusal_names_its_cause 655-742
mutants crates/runner/src/expression_validation_tests.rs a_later_day_whose_every_bar_is_refused_is_not_an_actual_session 747-778
mutants crates/runner/src/expression_validation_tests.rs a_projection_whose_folds_break_the_count_hierarchy_is_refused 784-827
mutants crates/runner/src/expression_validation_tests.rs line 649
mutants crates/runner/src/expression_validation_tests.rs line 650
mutants crates/runner/src/expression_validation_tests.rs line 651
mutants crates/runner/src/expression_validation_tests.rs line 652
mutants crates/runner/src/expression_validation_tests.rs line 653
mutants crates/runner/src/expression_validation_tests.rs line 654
mutants crates/runner/src/expression_validation_tests.rs line 743
mutants crates/runner/src/expression_validation_tests.rs line 744
mutants crates/runner/src/expression_validation_tests.rs line 745
mutants crates/runner/src/expression_validation_tests.rs line 746
mutants crates/runner/src/expression_validation_tests.rs line 779
mutants crates/runner/src/expression_validation_tests.rs line 780
mutants crates/runner/src/expression_validation_tests.rs line 781
mutants crates/runner/src/expression_validation_tests.rs line 782
mutants crates/runner/src/expression_validation_tests.rs line 783
mutants crates/runner/src/grid.rs the_indexed_excursion_equals_the_scan_it_replaced 7267-7316
mutants crates/runner/src/grid.rs v1_replay_preserves_refusals_even_when_no_cell_can_be_priced 8569-8639
mutants crates/runner/src/grid.rs chosen_materialization_never_contains_a_statefully_refused_path 10395-10491
mutants crates/runner/src/grid_cover_tests.rs a_target_ladder_past_the_cell_budget_is_thinned_by_even_stride 152-170
mutants crates/runner/src/grid_cover_tests.rs swept 173-182
mutants crates/runner/src/grid_cover_tests.rs the_convenience_doors_give_the_hoisted_doors_answers 188-235
mutants crates/runner/src/grid_cover_tests.rs a_replayed_price_and_candidate_read_back_their_fields 239-275
mutants crates/runner/src/grid_cover_tests.rs four_bars 279-300
mutants crates/runner/src/grid_cover_tests.rs long_candidate 303-327
mutants crates/runner/src/grid_cover_tests.rs every_replay_reconciliation_a_candidate_can_break_is_refused 335-498
mutants crates/runner/src/grid_cover_tests.rs line 5
mutants crates/runner/src/grid_cover_tests.rs line 6
mutants crates/runner/src/grid_cover_tests.rs line 7
mutants crates/runner/src/grid_cover_tests.rs line 8
mutants crates/runner/src/grid_cover_tests.rs line 146
mutants crates/runner/src/grid_cover_tests.rs line 147
mutants crates/runner/src/grid_cover_tests.rs line 148
mutants crates/runner/src/grid_cover_tests.rs line 149
mutants crates/runner/src/grid_cover_tests.rs line 150
mutants crates/runner/src/grid_cover_tests.rs line 151
mutants crates/runner/src/grid_cover_tests.rs line 171
mutants crates/runner/src/grid_cover_tests.rs line 172
mutants crates/runner/src/grid_cover_tests.rs line 183
mutants crates/runner/src/grid_cover_tests.rs line 184
mutants crates/runner/src/grid_cover_tests.rs line 185
mutants crates/runner/src/grid_cover_tests.rs line 186
mutants crates/runner/src/grid_cover_tests.rs line 187
mutants crates/runner/src/grid_cover_tests.rs line 236
mutants crates/runner/src/grid_cover_tests.rs line 237
mutants crates/runner/src/grid_cover_tests.rs line 238
mutants crates/runner/src/grid_cover_tests.rs line 276
mutants crates/runner/src/grid_cover_tests.rs line 277
mutants crates/runner/src/grid_cover_tests.rs line 278
mutants crates/runner/src/grid_cover_tests.rs line 301
mutants crates/runner/src/grid_cover_tests.rs line 302
mutants crates/runner/src/grid_cover_tests.rs line 328
mutants crates/runner/src/grid_cover_tests.rs line 329
mutants crates/runner/src/grid_cover_tests.rs line 330
mutants crates/runner/src/grid_cover_tests.rs line 331
mutants crates/runner/src/grid_cover_tests.rs line 332
mutants crates/runner/src/grid_cover_tests.rs line 333
mutants crates/runner/src/grid_cover_tests.rs line 334
mutants crates/runner/src/outcome.rs long 5537-5545
mutants crates/runner/src/outcome.rs same_day_answers_for_two_existing_bars 5550-5558
mutants crates/runner/src/outcome.rs worst_reward_risk_answers_zero_for_a_ratio_it_cannot_state 5564-5580
mutants crates/runner/src/outcome.rs payoff_names_each_ratio_it_cannot_divide 5586-5610
mutants crates/runner/src/outcome.rs path_ratio_answers_zero_with_no_hits_and_saturates_past_its_bound 5615-5630
mutants crates/runner/src/outcome.rs line 5524
mutants crates/runner/src/outcome.rs line 5525
mutants crates/runner/src/outcome.rs line 5526
mutants crates/runner/src/outcome.rs line 5527
mutants crates/runner/src/outcome.rs line 5528
mutants crates/runner/src/outcome.rs line 5529
mutants crates/runner/src/outcome.rs line 5530
mutants crates/runner/src/outcome.rs line 5531
mutants crates/runner/src/outcome.rs line 5532
mutants crates/runner/src/outcome.rs line 5533
mutants crates/runner/src/outcome.rs line 5534
mutants crates/runner/src/outcome.rs line 5535
mutants crates/runner/src/outcome.rs line 5536
mutants crates/runner/src/outcome.rs line 5546
mutants crates/runner/src/outcome.rs line 5547
mutants crates/runner/src/outcome.rs line 5548
mutants crates/runner/src/outcome.rs line 5549
mutants crates/runner/src/outcome.rs line 5559
mutants crates/runner/src/outcome.rs line 5560
mutants crates/runner/src/outcome.rs line 5561
mutants crates/runner/src/outcome.rs line 5562
mutants crates/runner/src/outcome.rs line 5563
mutants crates/runner/src/outcome.rs line 5581
mutants crates/runner/src/outcome.rs line 5582
mutants crates/runner/src/outcome.rs line 5583
mutants crates/runner/src/outcome.rs line 5584
mutants crates/runner/src/outcome.rs line 5585
mutants crates/runner/src/outcome.rs line 5611
mutants crates/runner/src/outcome.rs line 5612
mutants crates/runner/src/outcome.rs line 5613
mutants crates/runner/src/outcome.rs line 5614
mutants crates/runner/src/outcome.rs line 5631
mutants crates/runner/src/pbo.rs a_rank_past_the_last_place_has_no_relative_placement 427-439
mutants crates/runner/src/pbo.rs line 424
mutants crates/runner/src/pbo.rs line 425
mutants crates/runner/src/pbo.rs line 426
mutants crates/runner/src/pbo.rs line 440
mutants crates/runner/src/rank.rs every_operator_of_a_ranking_wrapper_agrees_with_its_order 1484-1493
mutants crates/runner/src/report.rs an_excluded_position_reaches_the_report_by_name_and_by_reason 1114-1161
mutants crates/runner/src/report_cover_tests.rs plain 248-255
mutants crates/runner/src/report_cover_tests.rs an_unmeasured_exclusion_says_not_measured_and_names_its_reason 262-277
mutants crates/runner/src/report_cover_tests.rs a_plain_run_halted_by_its_workers_names_them 282-293
mutants crates/runner/src/report_cover_tests.rs line 15
mutants crates/runner/src/report_cover_tests.rs line 20
mutants crates/runner/src/report_cover_tests.rs line 246
mutants crates/runner/src/report_cover_tests.rs line 247
mutants crates/runner/src/report_cover_tests.rs line 256
mutants crates/runner/src/report_cover_tests.rs line 257
mutants crates/runner/src/report_cover_tests.rs line 258
mutants crates/runner/src/report_cover_tests.rs line 259
mutants crates/runner/src/report_cover_tests.rs line 260
mutants crates/runner/src/report_cover_tests.rs line 261
mutants crates/runner/src/report_cover_tests.rs line 278
mutants crates/runner/src/report_cover_tests.rs line 279
mutants crates/runner/src/report_cover_tests.rs line 280
mutants crates/runner/src/report_cover_tests.rs line 281
mutants crates/runner/src/research_admission_projection.rs policy 132-175
mutants crates/runner/src/research_admission_projection.rs a_search_ceiling_caps_each_family_probability_and_nothing_else 182-217
mutants crates/runner/src/research_admission_projection.rs line 122
mutants crates/runner/src/research_admission_projection.rs line 123
mutants crates/runner/src/research_admission_projection.rs line 124
mutants crates/runner/src/research_admission_projection.rs line 125
mutants crates/runner/src/research_admission_projection.rs line 126
mutants crates/runner/src/research_admission_projection.rs line 127
mutants crates/runner/src/research_admission_projection.rs line 128
mutants crates/runner/src/research_admission_projection.rs line 129
mutants crates/runner/src/research_admission_projection.rs line 130
mutants crates/runner/src/research_admission_projection.rs line 131
mutants crates/runner/src/research_admission_projection.rs line 176
mutants crates/runner/src/research_admission_projection.rs line 177
mutants crates/runner/src/research_admission_projection.rs line 178
mutants crates/runner/src/research_admission_projection.rs line 179
mutants crates/runner/src/research_admission_projection.rs line 180
mutants crates/runner/src/research_admission_projection.rs line 181
mutants crates/runner/src/research_admission_projection.rs line 218
mutants crates/runner/src/research_exit_grid_tests.rs internally_corrupted_resolution_cannot_reach_ladders_or_training_attestation 12-72
mutants crates/runner/src/signal_candle_stop_tests.rs a_priced_trade_reads_back_as_its_statistical_row 882-908
mutants crates/runner/src/signal_candle_stop_tests.rs stopped_long 948-959
mutants crates/runner/src/signal_candle_stop_tests.rs forced_long 962-966
mutants crates/runner/src/signal_candle_stop_tests.rs assert_each_trade_refused 979-987
mutants crates/runner/src/signal_candle_stop_tests.rs a_stop_exit_record_that_breaks_one_rule_is_refused 994-1060
mutants crates/runner/src/signal_candle_stop_tests.rs a_forced_close_record_that_breaks_one_rule_is_refused 1065-1103
mutants crates/runner/src/signal_candle_stop_tests.rs an_event_record_that_breaks_one_disposition_rule_is_refused 1108-1179
mutants crates/runner/src/signal_candle_stop_tests.rs a_period_record_that_breaks_one_count_rule_is_refused 1184-1267
mutants crates/runner/src/signal_candle_stop_tests.rs a_metrics_record_that_breaks_one_accounting_rule_is_refused 1273-1343
mutants crates/runner/src/signal_candle_stop_tests.rs an_undirected_run_id_and_an_unknown_timeframe_are_refused_by_scope 1348-1366
mutants crates/runner/src/signal_candle_stop_tests.rs refused_source 1372-1374
mutants crates/runner/src/signal_candle_stop_tests.rs a_signal_side_that_is_empty_unsorted_or_unlike_its_rows_is_refused 1381-1439
mutants crates/runner/src/signal_candle_stop_tests.rs an_execution_side_that_is_empty_unsorted_or_not_its_context_is_refused 1445-1496
mutants crates/runner/src/signal_candle_stop_tests.rs line 941
mutants crates/runner/src/signal_candle_stop_tests.rs line 942
mutants crates/runner/src/signal_candle_stop_tests.rs line 943
mutants crates/runner/src/signal_candle_stop_tests.rs line 944
mutants crates/runner/src/signal_candle_stop_tests.rs line 945
mutants crates/runner/src/signal_candle_stop_tests.rs line 946
mutants crates/runner/src/signal_candle_stop_tests.rs line 947
mutants crates/runner/src/signal_candle_stop_tests.rs line 960
mutants crates/runner/src/signal_candle_stop_tests.rs line 961
mutants crates/runner/src/signal_candle_stop_tests.rs line 967
mutants crates/runner/src/signal_candle_stop_tests.rs line 968
mutants crates/runner/src/signal_candle_stop_tests.rs line 969
mutants crates/runner/src/signal_candle_stop_tests.rs line 970
mutants crates/runner/src/signal_candle_stop_tests.rs line 971
mutants crates/runner/src/signal_candle_stop_tests.rs line 972
mutants crates/runner/src/signal_candle_stop_tests.rs line 973
mutants crates/runner/src/signal_candle_stop_tests.rs line 974
mutants crates/runner/src/signal_candle_stop_tests.rs line 975
mutants crates/runner/src/signal_candle_stop_tests.rs line 976
mutants crates/runner/src/signal_candle_stop_tests.rs line 977
mutants crates/runner/src/signal_candle_stop_tests.rs line 978
mutants crates/runner/src/signal_candle_stop_tests.rs line 988
mutants crates/runner/src/signal_candle_stop_tests.rs line 989
mutants crates/runner/src/signal_candle_stop_tests.rs line 990
mutants crates/runner/src/signal_candle_stop_tests.rs line 991
mutants crates/runner/src/signal_candle_stop_tests.rs line 992
mutants crates/runner/src/signal_candle_stop_tests.rs line 993
mutants crates/runner/src/signal_candle_stop_tests.rs line 1061
mutants crates/runner/src/signal_candle_stop_tests.rs line 1062
mutants crates/runner/src/signal_candle_stop_tests.rs line 1063
mutants crates/runner/src/signal_candle_stop_tests.rs line 1064
mutants crates/runner/src/signal_candle_stop_tests.rs line 1104
mutants crates/runner/src/signal_candle_stop_tests.rs line 1105
mutants crates/runner/src/signal_candle_stop_tests.rs line 1106
mutants crates/runner/src/signal_candle_stop_tests.rs line 1107
mutants crates/runner/src/signal_candle_stop_tests.rs line 1180
mutants crates/runner/src/signal_candle_stop_tests.rs line 1181
mutants crates/runner/src/signal_candle_stop_tests.rs line 1182
mutants crates/runner/src/signal_candle_stop_tests.rs line 1183
mutants crates/runner/src/signal_candle_stop_tests.rs line 1268
mutants crates/runner/src/signal_candle_stop_tests.rs line 1269
mutants crates/runner/src/signal_candle_stop_tests.rs line 1270
mutants crates/runner/src/signal_candle_stop_tests.rs line 1271
mutants crates/runner/src/signal_candle_stop_tests.rs line 1272
mutants crates/runner/src/signal_candle_stop_tests.rs line 1344
mutants crates/runner/src/signal_candle_stop_tests.rs line 1345
mutants crates/runner/src/signal_candle_stop_tests.rs line 1346
mutants crates/runner/src/signal_candle_stop_tests.rs line 1347
mutants crates/runner/src/signal_candle_stop_tests.rs line 1367
mutants crates/runner/src/signal_candle_stop_tests.rs line 1368
mutants crates/runner/src/signal_candle_stop_tests.rs line 1369
mutants crates/runner/src/signal_candle_stop_tests.rs line 1370
mutants crates/runner/src/signal_candle_stop_tests.rs line 1371
mutants crates/runner/src/signal_candle_stop_tests.rs line 1375
mutants crates/runner/src/signal_candle_stop_tests.rs line 1376
mutants crates/runner/src/signal_candle_stop_tests.rs line 1377
mutants crates/runner/src/signal_candle_stop_tests.rs line 1378
mutants crates/runner/src/signal_candle_stop_tests.rs line 1379
mutants crates/runner/src/signal_candle_stop_tests.rs line 1380
mutants crates/runner/src/signal_candle_stop_tests.rs line 1440
mutants crates/runner/src/signal_candle_stop_tests.rs line 1441
mutants crates/runner/src/signal_candle_stop_tests.rs line 1442
mutants crates/runner/src/signal_candle_stop_tests.rs line 1443
mutants crates/runner/src/signal_candle_stop_tests.rs line 1444
mutants crates/runner/src/significance_cover_tests.rs a_fraction_that_never_converges_answers_nan_rather_than_a_number 53-64
mutants crates/runner/src/significance_cover_tests.rs line 47
mutants crates/runner/src/significance_cover_tests.rs line 48
mutants crates/runner/src/significance_cover_tests.rs line 49
mutants crates/runner/src/significance_cover_tests.rs line 50
mutants crates/runner/src/significance_cover_tests.rs line 51
mutants crates/runner/src/significance_cover_tests.rs line 52
mutants crates/runner/src/topn.rs candidate 830-847
mutants crates/runner/src/trade.rs an_accepted_exit_below_one_tick_holds_the_window_unpriced 3520-3567
mutants crates/runner/src/trade.rs an_entry_open_below_one_tick_is_not_held_when_its_exit_cannot_fill 3574-3598
mutants crates/runner/src/trade.rs a_range_or_entry_outside_the_slice_facts_is_answered_no 3601-3640
mutants crates/runner/src/trade.rs an_accepted_bar_off_the_whole_minute_is_not_accepted_by_the_slice_facts 3645-3663
mutants crates/runner/src/trade.rs line 3509
mutants crates/runner/src/trade.rs line 3510
mutants crates/runner/src/trade.rs line 3511
mutants crates/runner/src/trade.rs line 3512
mutants crates/runner/src/trade.rs line 3513
mutants crates/runner/src/trade.rs line 3514
mutants crates/runner/src/trade.rs line 3515
mutants crates/runner/src/trade.rs line 3516
mutants crates/runner/src/trade.rs line 3517
mutants crates/runner/src/trade.rs line 3518
mutants crates/runner/src/trade.rs line 3519
mutants crates/runner/src/trade.rs line 3568
mutants crates/runner/src/trade.rs line 3569
mutants crates/runner/src/trade.rs line 3570
mutants crates/runner/src/trade.rs line 3571
mutants crates/runner/src/trade.rs line 3572
mutants crates/runner/src/trade.rs line 3573
mutants crates/runner/src/trade.rs line 3599
mutants crates/runner/src/trade.rs line 3600
mutants crates/runner/src/trade.rs line 3641
mutants crates/runner/src/trade.rs line 3642
mutants crates/runner/src/trade.rs line 3643
mutants crates/runner/src/trade.rs line 3644
mutants crates/runner/src/validate.rs evaluator 5371-5377
mutants crates/runner/src/validate.rs h 5379-5381
mutants crates/runner/src/validate.rs sweeper 5383-5389
mutants crates/runner/src/validate.rs anchored_admission_fixture_v2 5430-5435
mutants crates/runner/src/validate.rs anchored_search_fixture_v3 5463-5468
mutants crates/runner/src/validate.rs exact_grid_policy_v4 5470-5491
mutants crates/runner/src/validate.rs admission_capture_v2 5544-5555
mutants crates/runner/src/validate.rs admission_candidate_v2 5557-5569
mutants crates/runner/src/validate.rs admission_split_v2 5571-5578
mutants crates/runner/src/validate.rs a_rolling_walk_forward_slides_its_window_and_an_anchored_one_grows 6219-6278
mutants crates/runner/src/validate.rs a_walk_forward_never_judges_a_choice_on_a_bar_it_was_chosen_on 6390-6414
mutants crates/runner/src/validate.rs line 8163
mutants crates/runner/src/validate.rs line 8164
mutants crates/runner/src/validate.rs line 8165
mutants crates/runner/src/validate.rs line 8166
mutants crates/runner/src/validate_cover_tests.rs whole
`````
