# deep-BASE-COV-PULL-PSA-a52270b1f00a-set.txt.md

The recipe v4 step 2 set file BASE-COV-PULL-PSA measured with Route B on the code tip `a52270b1f00a6c0f744a0190d67053b1fdeb6440` (`reading whole:pull`; 255 coverage lines, every tracked `.rs` of `crates/pull` but the 8 bench files; 1950 mutants lines, the D2 set of `git diff f01d7a0e4...a52270b1f00a`, for the deferred background pass). Made by a generator that reproduces the kit's `deep-BASE-COV-PULL-f98a899398f6-set.txt` byte for byte (sha256 `2eafbf44...`). sha256 of the fenced content: `218202d68fb4d3557664d181b738632a7a16c60076f68769a2ca928a645b42ae`. One fenced block below.

`````text
reading whole:pull
coverage crates/pull/src/alerts.rs whole
coverage crates/pull/src/alerts_ps2_tests.rs whole
coverage crates/pull/src/alerts_tests.rs whole
coverage crates/pull/src/archive.rs whole
coverage crates/pull/src/archive_cov_tests.rs whole
coverage crates/pull/src/audit_ledger.rs whole
coverage crates/pull/src/audit_ledger_tests.rs whole
coverage crates/pull/src/base_cov_codes_tests.rs whole
coverage crates/pull/src/base_cov_words_tests.rs whole
coverage crates/pull/src/calendar.rs whole
coverage crates/pull/src/calendar_cov_tests.rs whole
coverage crates/pull/src/calendar_tradable_tests.rs whole
coverage crates/pull/src/capture.rs whole
coverage crates/pull/src/capture_ps1_tests.rs whole
coverage crates/pull/src/cash_auction.rs whole
coverage crates/pull/src/cash_auction_ps1_tests.rs whole
coverage crates/pull/src/cash_session_cache.rs whole
coverage crates/pull/src/cash_session_cache_lock_tests.rs whole
coverage crates/pull/src/cash_session_cache_ps1_tests.rs whole
coverage crates/pull/src/census_store.rs whole
coverage crates/pull/src/census_store_cov_tests.rs whole
coverage crates/pull/src/census_store_kept_tests.rs whole
coverage crates/pull/src/census_store_link_tests.rs whole
coverage crates/pull/src/census_store_ps2_tests.rs whole
coverage crates/pull/src/census_store_tests.rs whole
coverage crates/pull/src/chain.rs whole
coverage crates/pull/src/chain_ps2_tests.rs whole
coverage crates/pull/src/config.rs whole
coverage crates/pull/src/corporate_actions.rs whole
coverage crates/pull/src/corporate_actions_cov_tests.rs whole
coverage crates/pull/src/corporate_actions_tests.rs whole
coverage crates/pull/src/csv.rs whole
coverage crates/pull/src/day_universe.rs whole
coverage crates/pull/src/daycheck.rs whole
coverage crates/pull/src/dhan.rs whole
coverage crates/pull/src/disk_projection.rs whole
coverage crates/pull/src/disk_projection_tests.rs whole
coverage crates/pull/src/emit_sites.rs whole
coverage crates/pull/src/expiry_alias.rs whole
coverage crates/pull/src/fetch.rs whole
coverage crates/pull/src/fetch_ps1_tests.rs whole
coverage crates/pull/src/fno.rs whole
coverage crates/pull/src/fno_membership.rs whole
coverage crates/pull/src/fno_membership_cov_tests.rs whole
coverage crates/pull/src/fno_membership_ps1_tests.rs whole
coverage crates/pull/src/fno_membership_tests.rs whole
coverage crates/pull/src/fno_ps2_tests.rs whole
coverage crates/pull/src/fnowork.rs whole
coverage crates/pull/src/fnowork_cov_tests.rs whole
coverage crates/pull/src/fnowork_futures_tests.rs whole
coverage crates/pull/src/fold.rs whole
coverage crates/pull/src/fold_ps2_tests.rs whole
coverage crates/pull/src/folder.rs whole
coverage crates/pull/src/gap_backfill.rs whole
coverage crates/pull/src/gap_backfill_cov_tests.rs whole
coverage crates/pull/src/gaps.rs whole
coverage crates/pull/src/gaps_cov_tests.rs whole
coverage crates/pull/src/gaps_not_owed_tests.rs whole
coverage crates/pull/src/gdfl_archive.rs whole
coverage crates/pull/src/gdfl_backfill_tests.rs whole
coverage crates/pull/src/gdfl_cm.rs whole
coverage crates/pull/src/gdfl_cm_admission_tests.rs whole
coverage crates/pull/src/gdfl_cm_auction.rs whole
coverage crates/pull/src/gdfl_cm_auction_ps1_tests.rs whole
coverage crates/pull/src/gdfl_cm_auction_v2_tests.rs whole
coverage crates/pull/src/gdfl_cm_ps2_tests.rs whole
coverage crates/pull/src/gdfl_dayzips.rs whole
coverage crates/pull/src/gdfl_dayzips_ps2_tests.rs whole
coverage crates/pull/src/gdfl_dayzips_tests.rs whole
coverage crates/pull/src/gdfl_dropbox.rs whole
coverage crates/pull/src/gdfl_dropbox_cov_tests.rs whole
coverage crates/pull/src/gdfl_dropbox_tests.rs whole
coverage crates/pull/src/gdfl_fixtures.rs whole
coverage crates/pull/src/gdfl_fixtures_ps1_tests.rs whole
coverage crates/pull/src/gdfl_format.rs whole
coverage crates/pull/src/gdfl_format_change_tests.rs whole
coverage crates/pull/src/gdfl_format_ps2_tests.rs whole
coverage crates/pull/src/gdfl_ia16_tests.rs whole
coverage crates/pull/src/gdfl_ia3_tests.rs whole
coverage crates/pull/src/gdfl_ia4_tests.rs whole
coverage crates/pull/src/gdfl_import.rs whole
coverage crates/pull/src/gdfl_import_causes.rs whole
coverage crates/pull/src/gdfl_import_census.rs whole
coverage crates/pull/src/gdfl_import_census_ps2_tests.rs whole
coverage crates/pull/src/gdfl_import_cov_tests.rs whole
coverage crates/pull/src/gdfl_import_day.rs whole
coverage crates/pull/src/gdfl_import_day_ps1_tests.rs whole
coverage crates/pull/src/gdfl_import_filing.rs whole
coverage crates/pull/src/gdfl_import_filing_ps2_tests.rs whole
coverage crates/pull/src/gdfl_import_ia2_tests.rs whole
coverage crates/pull/src/gdfl_import_journal.rs whole
coverage crates/pull/src/gdfl_import_journal_cov_tests.rs whole
coverage crates/pull/src/gdfl_import_lineage.rs whole
coverage crates/pull/src/gdfl_import_lineage_cov_tests.rs whole
coverage crates/pull/src/gdfl_import_ps1_tests.rs whole
coverage crates/pull/src/gdfl_import_quality.rs whole
coverage crates/pull/src/gdfl_import_quality_ps2_tests.rs whole
coverage crates/pull/src/gdfl_import_scope.rs whole
coverage crates/pull/src/gdfl_import_split_tests.rs whole
coverage crates/pull/src/gdfl_import_tests.rs whole
coverage crates/pull/src/gdfl_lineage.rs whole
coverage crates/pull/src/gdfl_lineage_tests.rs whole
coverage crates/pull/src/gdfl_nfo.rs whole
coverage crates/pull/src/gdfl_nfo_attack_tests.rs whole
coverage crates/pull/src/gdfl_nfo_ps1_tests.rs whole
coverage crates/pull/src/gdfl_nfo_tests.rs whole
coverage crates/pull/src/gdfl_r2_attack_tests.rs whole
coverage crates/pull/src/gdfl_r3_attack_tests.rs whole
coverage crates/pull/src/gdfl_r4_attack_tests.rs whole
coverage crates/pull/src/gdfl_rekey.rs whole
coverage crates/pull/src/gdfl_rekey_cov_tests.rs whole
coverage crates/pull/src/gdfl_rekey_ps2_tests.rs whole
coverage crates/pull/src/gdfl_rekey_tests.rs whole
coverage crates/pull/src/gdfl_run.rs whole
coverage crates/pull/src/gdfl_run_cov_tests.rs whole
coverage crates/pull/src/gdfl_run_ia2_tests.rs whole
coverage crates/pull/src/gdfl_run_ledger.rs whole
coverage crates/pull/src/gdfl_run_ledger_cov_tests.rs whole
coverage crates/pull/src/gdfl_run_ledger_ps2_tests.rs whole
coverage crates/pull/src/gdfl_run_locks.rs whole
coverage crates/pull/src/gdfl_run_locks_cov_tests.rs whole
coverage crates/pull/src/gdfl_run_locks_ps1_tests.rs whole
coverage crates/pull/src/gdfl_run_scope.rs whole
coverage crates/pull/src/gdfl_run_scope_cov_tests.rs whole
coverage crates/pull/src/gdfl_run_start.rs whole
coverage crates/pull/src/gdfl_run_start_cov_tests.rs whole
coverage crates/pull/src/gdfl_run_stop.rs whole
coverage crates/pull/src/gdfl_run_stop_cov_tests.rs whole
coverage crates/pull/src/gdfl_run_tests.rs whole
coverage crates/pull/src/gdfl_scope_tests.rs whole
coverage crates/pull/src/gdfl_sealed_tests.rs whole
coverage crates/pull/src/gdfl_seconds_attack_tests.rs whole
coverage crates/pull/src/gdfl_series_code_tests.rs whole
coverage crates/pull/src/gdfl_tickstore.rs whole
coverage crates/pull/src/gdfl_tickstore_cov_tests.rs whole
coverage crates/pull/src/gdfl_v3.rs whole
coverage crates/pull/src/gdfl_v3_ps2_tests.rs whole
coverage crates/pull/src/gdfl_v3_tests.rs whole
coverage crates/pull/src/gdfl_wp11c_tests.rs whole
coverage crates/pull/src/gdfl_wp12e_tests.rs whole
coverage crates/pull/src/groww.rs whole
coverage crates/pull/src/http.rs whole
coverage crates/pull/src/http_cov_tests.rs whole
coverage crates/pull/src/identity_screen.rs whole
coverage crates/pull/src/identity_screen_ps1_tests.rs whole
coverage crates/pull/src/ingest.rs whole
coverage crates/pull/src/ingest_cov_tests.rs whole
coverage crates/pull/src/ingest_failkind.rs whole
coverage crates/pull/src/ingest_failkind_tests.rs whole
coverage crates/pull/src/ingest_gap_tests.rs whole
coverage crates/pull/src/kite.rs whole
coverage crates/pull/src/kite_cov_tests.rs whole
coverage crates/pull/src/landing_run.rs whole
coverage crates/pull/src/landing_run_tests.rs whole
coverage crates/pull/src/lib.rs whole
coverage crates/pull/src/listing.rs whole
coverage crates/pull/src/listing_tests.rs whole
coverage crates/pull/src/manifest.rs whole
coverage crates/pull/src/manifest_cov_tests.rs whole
coverage crates/pull/src/manifest_ps1_tests.rs whole
coverage crates/pull/src/master_snapshots.rs whole
coverage crates/pull/src/master_snapshots_cov_tests.rs whole
coverage crates/pull/src/master_snapshots_first_listed.rs whole
coverage crates/pull/src/master_snapshots_first_listed_ps1_tests.rs whole
coverage crates/pull/src/master_snapshots_first_listed_tests.rs whole
coverage crates/pull/src/master_snapshots_ps1_tests.rs whole
coverage crates/pull/src/master_snapshots_tests.rs whole
coverage crates/pull/src/masters.rs whole
coverage crates/pull/src/masters_cov_tests.rs whole
coverage crates/pull/src/masters_land_tests.rs whole
coverage crates/pull/src/nse.rs whole
coverage crates/pull/src/nse_ps1_tests.rs whole
coverage crates/pull/src/nseindex.rs whole
coverage crates/pull/src/padded.rs whole
coverage crates/pull/src/padded_tests.rs whole
coverage crates/pull/src/pricing.rs whole
coverage crates/pull/src/pricing_ps1_tests.rs whole
coverage crates/pull/src/rate.rs whole
coverage crates/pull/src/reference_data.rs whole
coverage crates/pull/src/reference_data_cov_tests.rs whole
coverage crates/pull/src/reference_data_ps1_tests.rs whole
coverage crates/pull/src/reference_data_tests.rs whole
coverage crates/pull/src/refusal.rs whole
coverage crates/pull/src/refusal_ps1_tests.rs whole
coverage crates/pull/src/rehearsal.rs whole
coverage crates/pull/src/rehearsal_ps1_tests.rs whole
coverage crates/pull/src/rehearsal_tests.rs whole
coverage crates/pull/src/repair_plan.rs whole
coverage crates/pull/src/repair_plan_not_owed_tests.rs whole
coverage crates/pull/src/repair_plan_ps2_tests.rs whole
coverage crates/pull/src/request_minutes.rs whole
coverage crates/pull/src/request_minutes_ps1_tests.rs whole
coverage crates/pull/src/resolve.rs whole
coverage crates/pull/src/response_keep.rs whole
coverage crates/pull/src/response_keep_tests.rs whole
coverage crates/pull/src/riskfree.rs whole
coverage crates/pull/src/rolling.rs whole
coverage crates/pull/src/scrub.rs whole
coverage crates/pull/src/scrub_blocks_tests.rs whole
coverage crates/pull/src/secret.rs whole
coverage crates/pull/src/series_identity.rs whole
coverage crates/pull/src/session.rs whole
coverage crates/pull/src/ssm.rs whole
coverage crates/pull/src/ssm_cov_tests.rs whole
coverage crates/pull/src/ssm_ps1_tests.rs whole
coverage crates/pull/src/tenor.rs whole
coverage crates/pull/src/tickstore_write.rs whole
coverage crates/pull/src/tickstore_write_tests.rs whole
coverage crates/pull/src/totp.rs whole
coverage crates/pull/src/universe.rs whole
coverage crates/pull/src/upkeep.rs whole
coverage crates/pull/src/upkeep_tests.rs whole
coverage crates/pull/src/vendor.rs whole
coverage crates/pull/src/vendor_cov_tests.rs whole
coverage crates/pull/src/verdict_words.rs whole
coverage crates/pull/src/work.rs whole
coverage crates/pull/src/zerodha_spool.rs whole
coverage crates/pull/src/zerodha_spool_cov_tests.rs whole
coverage crates/pull/src/zerodha_verify.rs whole
coverage crates/pull/src/zerodha_verify_cov_store_tests.rs whole
coverage crates/pull/src/zerodha_verify_cov_tests.rs whole
coverage crates/pull/src/zerodha_verify_tests.rs whole
coverage crates/pull/tests/alerts_ps2.rs whole
coverage crates/pull/tests/alerts_seams.rs whole
coverage crates/pull/tests/allocation.rs whole
coverage crates/pull/tests/anchor.rs whole
coverage crates/pull/tests/broker.rs whole
coverage crates/pull/tests/census.rs whole
coverage crates/pull/tests/census_day_pace.rs whole
coverage crates/pull/tests/census_resume.rs whole
coverage crates/pull/tests/census_v4.rs whole
coverage crates/pull/tests/cloud_mutants_import.rs whole
coverage crates/pull/tests/concurrency.rs whole
coverage crates/pull/tests/derive.rs whole
coverage crates/pull/tests/folder.rs whole
coverage crates/pull/tests/futures_events.rs whole
coverage crates/pull/tests/futures_refusal_events.rs whole
coverage crates/pull/tests/gdfl_bad_row_audit.rs whole
coverage crates/pull/tests/gdfl_census_stop.rs whole
coverage crates/pull/tests/gdfl_run_stop.rs whole
coverage crates/pull/tests/integration.rs whole
coverage crates/pull/tests/no_scheduler_delete.rs whole
coverage crates/pull/tests/padded_refold.rs whole
coverage crates/pull/tests/pipeline.rs whole
coverage crates/pull/tests/refusal.rs whole
coverage crates/pull/tests/request_gap_events.rs whole
coverage crates/pull/tests/sealed_month.rs whole
coverage crates/pull/tests/store_evidence.rs whole
coverage crates/pull/tests/support/mod.rs whole
coverage crates/pull/tests/tls.rs whole
coverage crates/pull/tests/token_identity.rs whole
coverage crates/pull/tests/unit.rs whole
coverage crates/pull/tests/zerodha_gaps.rs whole
coverage crates/pull/tests/zerodha_identity.rs whole
coverage crates/pull/tests/zerodha_repair.rs whole
mutants .github/gates_tree.rs gate_1d_declares_the_base_cov_pull_words 4985-5022
mutants .github/gates_tree.rs line 3006
mutants .github/gates_tree.rs line 3007
mutants .github/gates_tree.rs line 3008
mutants .github/gates_tree.rs line 3009
mutants .github/gates_tree.rs line 3010
mutants .github/gates_tree.rs line 3011
mutants .github/gates_tree.rs line 3012
mutants .github/gates_tree.rs line 3013
mutants .github/gates_tree.rs line 3014
mutants .github/gates_tree.rs line 3015
mutants .github/gates_tree.rs line 3016
mutants .github/gates_tree.rs line 3017
mutants .github/gates_tree.rs line 3018
mutants .github/gates_tree.rs line 3019
mutants .github/gates_tree.rs line 3020
mutants .github/gates_tree.rs line 3021
mutants .github/gates_tree.rs line 3022
mutants .github/gates_tree.rs line 3023
mutants .github/gates_tree.rs line 3024
mutants .github/gates_tree.rs line 3025
mutants .github/gates_tree.rs line 3026
mutants .github/gates_tree.rs line 3027
mutants .github/gates_tree.rs line 3028
mutants .github/gates_tree.rs line 3029
mutants .github/gates_tree.rs line 3030
mutants .github/gates_tree.rs line 3031
mutants .github/gates_tree.rs line 3032
mutants .github/gates_tree.rs line 3033
mutants .github/gates_tree.rs line 3034
mutants .github/gates_tree.rs line 3035
mutants .github/gates_tree.rs line 3036
mutants .github/gates_tree.rs line 3037
mutants .github/gates_tree.rs line 3038
mutants .github/gates_tree.rs line 3039
mutants .github/gates_tree.rs line 3040
mutants .github/gates_tree.rs line 3041
mutants .github/gates_tree.rs line 3042
mutants .github/gates_tree.rs line 3043
mutants .github/gates_tree.rs line 3044
mutants .github/gates_tree.rs line 3045
mutants .github/gates_tree.rs line 3046
mutants .github/gates_tree.rs line 3047
mutants .github/gates_tree.rs line 3048
mutants .github/gates_tree.rs line 3049
mutants .github/gates_tree.rs line 3050
mutants .github/gates_tree.rs line 3051
mutants .github/gates_tree.rs line 3052
mutants .github/gates_tree.rs line 3053
mutants .github/gates_tree.rs line 3054
mutants .github/gates_tree.rs line 3055
mutants .github/gates_tree.rs line 3056
mutants .github/gates_tree.rs line 3057
mutants .github/gates_tree.rs line 3058
mutants .github/gates_tree.rs line 3059
mutants .github/gates_tree.rs line 3060
mutants .github/gates_tree.rs line 3061
mutants .github/gates_tree.rs line 3062
mutants .github/gates_tree.rs line 3063
mutants .github/gates_tree.rs line 3064
mutants .github/gates_tree.rs line 3065
mutants .github/gates_tree.rs line 3066
mutants .github/gates_tree.rs line 3067
mutants .github/gates_tree.rs line 3068
mutants .github/gates_tree.rs line 3069
mutants .github/gates_tree.rs line 3070
mutants .github/gates_tree.rs line 3071
mutants .github/gates_tree.rs line 3072
mutants .github/gates_tree.rs line 3073
mutants .github/gates_tree.rs line 3074
mutants .github/gates_tree.rs line 3075
mutants .github/gates_tree.rs line 3076
mutants .github/gates_tree.rs line 3077
mutants .github/gates_tree.rs line 3078
mutants .github/gates_tree.rs line 3079
mutants .github/gates_tree.rs line 3080
mutants .github/gates_tree.rs line 3081
mutants .github/gates_tree.rs line 3082
mutants .github/gates_tree.rs line 3083
mutants .github/gates_tree.rs line 3084
mutants .github/gates_tree.rs line 3085
mutants .github/gates_tree.rs line 3086
mutants .github/gates_tree.rs line 3087
mutants .github/gates_tree.rs line 3088
mutants .github/gates_tree.rs line 3089
mutants .github/gates_tree.rs line 3090
mutants .github/gates_tree.rs line 3091
mutants .github/gates_tree.rs line 3092
mutants .github/gates_tree.rs line 3093
mutants .github/gates_tree.rs line 3094
mutants .github/gates_tree.rs line 3095
mutants .github/gates_tree.rs line 3096
mutants .github/gates_tree.rs line 3097
mutants .github/gates_tree.rs line 3098
mutants .github/gates_tree.rs line 3099
mutants .github/gates_tree.rs line 3100
mutants .github/gates_tree.rs line 3101
mutants .github/gates_tree.rs line 3102
mutants .github/gates_tree.rs line 3103
mutants .github/gates_tree.rs line 3104
mutants .github/gates_tree.rs line 3105
mutants .github/gates_tree.rs line 3106
mutants .github/gates_tree.rs line 3107
mutants .github/gates_tree.rs line 3108
mutants .github/gates_tree.rs line 3109
mutants .github/gates_tree.rs line 3110
mutants .github/gates_tree.rs line 3111
mutants .github/gates_tree.rs line 3112
mutants .github/gates_tree.rs line 3113
mutants .github/gates_tree.rs line 3114
mutants .github/gates_tree.rs line 3115
mutants .github/gates_tree.rs line 3116
mutants .github/gates_tree.rs line 3117
mutants .github/gates_tree.rs line 3118
mutants .github/gates_tree.rs line 3119
mutants .github/gates_tree.rs line 3120
mutants .github/gates_tree.rs line 3121
mutants .github/gates_tree.rs line 3122
mutants .github/gates_tree.rs line 3123
mutants .github/gates_tree.rs line 3124
mutants .github/gates_tree.rs line 3125
mutants .github/gates_tree.rs line 3126
mutants .github/gates_tree.rs line 3127
mutants .github/gates_tree.rs line 3128
mutants .github/gates_tree.rs line 3129
mutants .github/gates_tree.rs line 3130
mutants .github/gates_tree.rs line 3131
mutants .github/gates_tree.rs line 3132
mutants .github/gates_tree.rs line 3133
mutants .github/gates_tree.rs line 3134
mutants .github/gates_tree.rs line 3135
mutants .github/gates_tree.rs line 3136
mutants .github/gates_tree.rs line 3137
mutants .github/gates_tree.rs line 3138
mutants .github/gates_tree.rs line 3139
mutants .github/gates_tree.rs line 3140
mutants .github/gates_tree.rs line 3141
mutants .github/gates_tree.rs line 3142
mutants .github/gates_tree.rs line 3143
mutants .github/gates_tree.rs line 3144
mutants .github/gates_tree.rs line 3145
mutants .github/gates_tree.rs line 3146
mutants .github/gates_tree.rs line 3147
mutants .github/gates_tree.rs line 3148
mutants .github/gates_tree.rs line 3149
mutants .github/gates_tree.rs line 3150
mutants .github/gates_tree.rs line 3151
mutants .github/gates_tree.rs line 3152
mutants .github/gates_tree.rs line 3165
mutants .github/gates_tree.rs line 3166
mutants .github/gates_tree.rs line 3246
mutants .github/gates_tree.rs line 5023
mutants crates/pull/src/alerts.rs keep 1686-1737
mutants crates/pull/src/alerts.rs replace_damaged_at 1739-1855
mutants crates/pull/src/alerts.rs line 1656
mutants crates/pull/src/alerts.rs line 1657
mutants crates/pull/src/alerts.rs line 1658
mutants crates/pull/src/alerts.rs line 1659
mutants crates/pull/src/alerts.rs line 1666
mutants crates/pull/src/alerts.rs line 1667
mutants crates/pull/src/alerts.rs line 1668
mutants crates/pull/src/alerts.rs line 1669
mutants crates/pull/src/alerts.rs line 1670
mutants crates/pull/src/alerts.rs line 1671
mutants crates/pull/src/alerts.rs line 1992
mutants crates/pull/src/alerts.rs line 1993
mutants crates/pull/src/alerts.rs line 1994
mutants crates/pull/src/alerts.rs line 1995
mutants crates/pull/src/alerts.rs line 1996
mutants crates/pull/src/alerts.rs line 1997
mutants crates/pull/src/alerts_ps2_tests.rs whole
mutants crates/pull/src/alerts_tests.rs garbage_alert_parameters_are_refused_by_name 915-977
mutants crates/pull/src/alerts_tests.rs assert_page_empty 983-990
mutants crates/pull/src/alerts_tests.rs mixed_queue 1156-1181
mutants crates/pull/src/alerts_tests.rs a_replacement_holds_the_lock_through_its_first_alert 1496-1560
mutants crates/pull/src/alerts_tests.rs create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it 2333-2339
mutants crates/pull/src/alerts_tests.rs create_fresh_refusal_body 2341-2357
mutants crates/pull/src/alerts_tests.rs line 2330
mutants crates/pull/src/alerts_tests.rs line 2331
mutants crates/pull/src/alerts_tests.rs line 2332
mutants crates/pull/src/alerts_tests.rs line 2340
mutants crates/pull/src/archive.rs descend 690-827
mutants crates/pull/src/archive.rs entry_path 829-845
mutants crates/pull/src/archive.rs line 846
mutants crates/pull/src/archive.rs line 1473
mutants crates/pull/src/archive.rs line 1474
mutants crates/pull/src/archive.rs line 1475
mutants crates/pull/src/archive.rs line 1476
mutants crates/pull/src/archive_cov_tests.rs whole
mutants crates/pull/src/base_cov_codes_tests.rs whole
mutants crates/pull/src/base_cov_words_tests.rs whole
mutants crates/pull/src/calendar.rs line 6616
mutants crates/pull/src/calendar.rs line 6617
mutants crates/pull/src/calendar.rs line 6618
mutants crates/pull/src/calendar.rs line 6619
mutants crates/pull/src/calendar_cov_tests.rs whole
mutants crates/pull/src/calendar_tradable_tests.rs the_rule_is_the_specs_conditions_on_every_day_of_the_record 162-189
mutants crates/pull/src/calendar_tradable_tests.rs an_appended_row_is_a_new_version_and_re_keys_none_before_it 750-821
mutants crates/pull/src/capture.rs write_new_capture_with_stamp 235-245
mutants crates/pull/src/capture.rs write_new_capture_using 247-293
mutants crates/pull/src/capture.rs line 246
mutants crates/pull/src/capture.rs line 809
mutants crates/pull/src/capture.rs line 810
mutants crates/pull/src/capture.rs line 811
mutants crates/pull/src/capture.rs line 812
mutants crates/pull/src/capture.rs line 813
mutants crates/pull/src/capture.rs line 814
mutants crates/pull/src/capture_ps1_tests.rs whole
mutants crates/pull/src/cash_auction.rs the_audit_reader_reads_an_invented_master_of_the_real_width 1421-1446
mutants crates/pull/src/cash_auction.rs line 1447
mutants crates/pull/src/cash_auction.rs line 1520
mutants crates/pull/src/cash_auction.rs line 1521
mutants crates/pull/src/cash_auction.rs line 1522
mutants crates/pull/src/cash_auction.rs line 1523
mutants crates/pull/src/cash_auction.rs line 1524
mutants crates/pull/src/cash_auction_ps1_tests.rs whole
mutants crates/pull/src/cash_session_cache.rs write_new 688-692
mutants crates/pull/src/cash_session_cache.rs write_new_with 694-718
mutants crates/pull/src/cash_session_cache.rs cached_window_never_fetches_and_uses_epoch_keys 1269-1302
mutants crates/pull/src/cash_session_cache.rs observed_dates_sort_dedup_and_reuse_without_a_weekend_filter 1499-1537
mutants crates/pull/src/cash_session_cache.rs observed_unreceipted_memory_refuses_and_late_failure_is_atomic 1592-1639
mutants crates/pull/src/cash_session_cache.rs sample_day 1813-1835
mutants crates/pull/src/cash_session_cache.rs a_sample_name_states_its_day_or_is_refused 1837-1868
mutants crates/pull/src/cash_session_cache.rs serve 2093-2119
mutants crates/pull/src/cash_session_cache.rs line 693
mutants crates/pull/src/cash_session_cache.rs line 1836
mutants crates/pull/src/cash_session_cache.rs line 2408
mutants crates/pull/src/cash_session_cache.rs line 2409
mutants crates/pull/src/cash_session_cache.rs line 2410
mutants crates/pull/src/cash_session_cache.rs line 2411
mutants crates/pull/src/cash_session_cache.rs line 2412
mutants crates/pull/src/cash_session_cache.rs line 2413
mutants crates/pull/src/cash_session_cache_lock_tests.rs an_installer_beside_readers_installs_once 203-255
mutants crates/pull/src/cash_session_cache_ps1_tests.rs whole
mutants crates/pull/src/census_store.rs current_epoch_in 224-286
mutants crates/pull/src/census_store.rs read_epoch 612-678
mutants crates/pull/src/census_store.rs still_current 1950-1990
mutants crates/pull/src/census_store.rs stage 2450-2511
mutants crates/pull/src/census_store.rs reader_refusal 2707-2782
mutants crates/pull/src/census_store.rs require_feature 2830-2912
mutants crates/pull/src/census_store.rs holds_sealed_stub 3029-3063
mutants crates/pull/src/census_store.rs seal_held 3171-3256
mutants crates/pull/src/census_store.rs install_stub 3258-3295
mutants crates/pull/src/census_store.rs settle_seal_receipt 3405-3470
mutants crates/pull/src/census_store.rs plant 4556-4559
mutants crates/pull/src/census_store.rs at 4561-4570
mutants crates/pull/src/census_store.rs fail 4572-4575
mutants crates/pull/src/census_store.rs answer 4577-4589
mutants crates/pull/src/census_store.rs line 4533
mutants crates/pull/src/census_store.rs line 4534
mutants crates/pull/src/census_store.rs line 4535
mutants crates/pull/src/census_store.rs line 4536
mutants crates/pull/src/census_store.rs line 4537
mutants crates/pull/src/census_store.rs line 4538
mutants crates/pull/src/census_store.rs line 4539
mutants crates/pull/src/census_store.rs line 4540
mutants crates/pull/src/census_store.rs line 4541
mutants crates/pull/src/census_store.rs line 4542
mutants crates/pull/src/census_store.rs line 4543
mutants crates/pull/src/census_store.rs line 4544
mutants crates/pull/src/census_store.rs line 4545
mutants crates/pull/src/census_store.rs line 4546
mutants crates/pull/src/census_store.rs line 4547
mutants crates/pull/src/census_store.rs line 4548
mutants crates/pull/src/census_store.rs line 4549
mutants crates/pull/src/census_store.rs line 4550
mutants crates/pull/src/census_store.rs line 4551
mutants crates/pull/src/census_store.rs line 4552
mutants crates/pull/src/census_store.rs line 4553
mutants crates/pull/src/census_store.rs line 4554
mutants crates/pull/src/census_store.rs line 4555
mutants crates/pull/src/census_store.rs line 4560
mutants crates/pull/src/census_store.rs line 4571
mutants crates/pull/src/census_store.rs line 4576
mutants crates/pull/src/census_store.rs line 4590
mutants crates/pull/src/census_store.rs line 4591
mutants crates/pull/src/census_store.rs line 4609
mutants crates/pull/src/census_store.rs line 4610
mutants crates/pull/src/census_store.rs line 4611
mutants crates/pull/src/census_store.rs line 4612
mutants crates/pull/src/census_store.rs line 4613
mutants crates/pull/src/census_store.rs line 4614
mutants crates/pull/src/census_store.rs line 4615
mutants crates/pull/src/census_store.rs line 4616
mutants crates/pull/src/census_store.rs line 4617
mutants crates/pull/src/census_store.rs line 4618
mutants crates/pull/src/census_store.rs line 4619
mutants crates/pull/src/census_store.rs line 4620
mutants crates/pull/src/census_store_cov_tests.rs whole
mutants crates/pull/src/census_store_kept_tests.rs two_runs_at_once_each_see_the_others_days 586-658
mutants crates/pull/src/census_store_kept_tests.rs a_symlinked_segment_directory_is_refused_by_name_on_install 667-728
mutants crates/pull/src/census_store_kept_tests.rs a_file_where_a_segment_directory_belongs_is_refused_by_name 730-763
mutants crates/pull/src/census_store_kept_tests.rs a_fifo_swapped_in_after_the_look_never_blocks_the_stub_check 799-844
mutants crates/pull/src/census_store_kept_tests.rs an_unreadable_file_of_the_stub_s_length_is_no_stub 872-880
mutants crates/pull/src/census_store_kept_tests.rs unreadable_stub_body 882-893
mutants crates/pull/src/census_store_kept_tests.rs the_stub_check_costs_one_open_measured 913-965
mutants crates/pull/src/census_store_kept_tests.rs line 881
mutants crates/pull/src/census_store_kept_tests.rs line 894
mutants crates/pull/src/census_store_link_tests.rs a_read_only_partition_directory_is_refused_with_the_host_s_words 347-356
mutants crates/pull/src/census_store_link_tests.rs read_only_partition_body 358-373
mutants crates/pull/src/census_store_link_tests.rs an_idempotent_requirement_opens_nothing_for_writing 672-686
mutants crates/pull/src/census_store_link_tests.rs idempotent_requirement_body 688-710
mutants crates/pull/src/census_store_link_tests.rs line 357
mutants crates/pull/src/census_store_link_tests.rs line 687
mutants crates/pull/src/census_store_ps2_tests.rs whole
mutants crates/pull/src/census_store_tests.rs a_run_that_meets_a_full_partition_installs_its_next_epoch 120-175
mutants crates/pull/src/census_store_tests.rs a_batch_is_three_flushes_and_one_slot_write_at_a_time 937-980
mutants crates/pull/src/census_store_tests.rs a_torn_write_inside_a_batch_never_leaves_the_census_behind 982-1082
mutants crates/pull/src/census_store_tests.rs a_refused_flush_inside_a_batch_stops_it_at_a_whole_generation 1084-1141
mutants crates/pull/src/census_store_tests.rs a_commit_past_the_batch_is_refused_before_a_byte 1143-1159
mutants crates/pull/src/chain.rs the_expiries_are_asked_first_and_each_contracts_call_is_keyed_on_the_answer 566-638
mutants crates/pull/src/chain.rs a_repeated_expiry_or_name_is_asked_and_filed_once 785-840
mutants crates/pull/src/chain.rs line 865
mutants crates/pull/src/chain.rs line 866
mutants crates/pull/src/chain.rs line 867
mutants crates/pull/src/chain.rs line 868
mutants crates/pull/src/chain.rs line 869
mutants crates/pull/src/chain.rs line 870
mutants crates/pull/src/chain_ps2_tests.rs whole
mutants crates/pull/src/corporate_actions.rs build 692-842
mutants crates/pull/src/corporate_actions.rs build 1051-1167
mutants crates/pull/src/corporate_actions.rs indexed_steps 1613-1643
mutants crates/pull/src/corporate_actions.rs line 2117
mutants crates/pull/src/corporate_actions.rs line 2118
mutants crates/pull/src/corporate_actions.rs line 2119
mutants crates/pull/src/corporate_actions_cov_tests.rs whole
mutants crates/pull/src/corporate_actions_tests.rs day 23-25
mutants crates/pull/src/corporate_actions_tests.rs r 27-29
mutants crates/pull/src/corporate_actions_tests.rs around 31-41
mutants crates/pull/src/corporate_actions_tests.rs runs_draft 57-67
mutants crates/pull/src/corporate_actions_tests.rs actions_draft 69-79
mutants crates/pull/src/corporate_actions_tests.rs nse 81-92
mutants crates/pull/src/corporate_actions_tests.rs load_actions 98-100
mutants crates/pull/src/corporate_actions_tests.rs run 838-849
mutants crates/pull/src/csv.rs rendered 1149-1160
mutants crates/pull/src/csv.rs every_declared_date_format_reads_the_day_it_names 1162-1212
mutants crates/pull/src/csv.rs a_date_whose_bytes_are_not_all_ascii_is_refused_not_sliced 1486-1531
mutants crates/pull/src/daycheck.rs the_first_truncated_day_stays_named_when_another_follows 607-624
mutants crates/pull/src/daycheck.rs line 606
mutants crates/pull/src/emit_sites.rs loopback 435-455
mutants crates/pull/src/emit_sites.rs every_emit_site_in_this_crate_reaches_a_file_body 2096-2180
mutants crates/pull/src/emit_sites.rs drive_census_degraded 2300-2320
mutants crates/pull/src/emit_sites.rs block_the_two_minute_rung 2371-2410
mutants crates/pull/src/expiry_alias.rs the_basis_builders_set_the_basis_and_nothing_else 1056-1067
mutants crates/pull/src/expiry_alias.rs extending_the_calendar_lists_changed_settlements 1704-1825
mutants crates/pull/src/expiry_alias.rs line 1068
mutants crates/pull/src/fetch.rs line 1712
mutants crates/pull/src/fetch.rs line 1713
mutants crates/pull/src/fetch.rs line 1714
mutants crates/pull/src/fetch.rs line 1715
mutants crates/pull/src/fetch.rs line 1716
mutants crates/pull/src/fetch.rs line 1717
mutants crates/pull/src/fetch_ps1_tests.rs whole
mutants crates/pull/src/fno.rs line 1369
mutants crates/pull/src/fno.rs line 1370
mutants crates/pull/src/fno.rs line 1371
mutants crates/pull/src/fno.rs line 1372
mutants crates/pull/src/fno.rs line 1373
mutants crates/pull/src/fno.rs line 1374
mutants crates/pull/src/fno_membership.rs check_rows 1429-1491
mutants crates/pull/src/fno_membership.rs install_with 1920-1986
mutants crates/pull/src/fno_membership.rs publish_then_unlink 1988-2016
mutants crates/pull/src/fno_membership.rs line 2017
mutants crates/pull/src/fno_membership.rs line 3079
mutants crates/pull/src/fno_membership.rs line 3080
mutants crates/pull/src/fno_membership.rs line 3081
mutants crates/pull/src/fno_membership.rs line 3082
mutants crates/pull/src/fno_membership.rs line 3083
mutants crates/pull/src/fno_membership.rs line 3084
mutants crates/pull/src/fno_membership.rs line 3085
mutants crates/pull/src/fno_membership.rs line 3086
mutants crates/pull/src/fno_membership.rs line 3087
mutants crates/pull/src/fno_membership.rs line 3088
mutants crates/pull/src/fno_membership.rs line 3089
mutants crates/pull/src/fno_membership.rs line 3090
mutants crates/pull/src/fno_membership_cov_tests.rs whole
mutants crates/pull/src/fno_membership_ps1_tests.rs whole
mutants crates/pull/src/fno_membership_tests.rs fetch 78-80
mutants crates/pull/src/fno_membership_tests.rs a_failed_write_between_two_month_installs_resumes_to_the_one_pass_bytes 1099-1108
mutants crates/pull/src/fno_membership_tests.rs a_failed_write_between_two_month_installs_resumes_to_the_one_pass_bytes_body 1110-1151
mutants crates/pull/src/fno_membership_tests.rs a_handover_needs_a_majority_of_both_sides_and_two_shared_keys 1153-1215
mutants crates/pull/src/fno_membership_tests.rs two_builds_of_one_directory_at_once_leave_no_temporary_and_the_one_pass_bytes 1433-1453
mutants crates/pull/src/fno_membership_tests.rs one_of_two_builds_won 1455-1497
mutants crates/pull/src/fno_membership_tests.rs a_build_into_a_directory_another_sealed_is_judged_as_the_race_s_loser 1499-1519
mutants crates/pull/src/fno_membership_tests.rs line 1109
mutants crates/pull/src/fno_membership_tests.rs line 1454
mutants crates/pull/src/fno_membership_tests.rs line 1498
mutants crates/pull/src/fno_ps2_tests.rs whole
mutants crates/pull/src/fnowork.rs owed 346-445
mutants crates/pull/src/fnowork.rs the_day_after 673-677
mutants crates/pull/src/fnowork.rs line 678
mutants crates/pull/src/fnowork.rs line 1340
mutants crates/pull/src/fnowork.rs line 1341
mutants crates/pull/src/fnowork.rs line 1342
mutants crates/pull/src/fnowork_cov_tests.rs whole
mutants crates/pull/src/fnowork_futures_tests.rs scratch 29-39
mutants crates/pull/src/fnowork_futures_tests.rs day 41-43
mutants crates/pull/src/fnowork_futures_tests.rs window 45-47
mutants crates/pull/src/fnowork_futures_tests.rs route 49-51
mutants crates/pull/src/fnowork_futures_tests.rs spec 53-59
mutants crates/pull/src/fnowork_futures_tests.rs fut 61-66
mutants crates/pull/src/fnowork_futures_tests.rs snapshot_on 68-76
mutants crates/pull/src/fnowork_futures_tests.rs block_on 120-126
mutants crates/pull/src/fnowork_futures_tests.rs february_reply 158-172
mutants crates/pull/src/fnowork_futures_tests.rs the_plans_warnings_reach_a_sink_with_their_fields 1573-1586
mutants crates/pull/src/fnowork_futures_tests.rs the_plans_warnings_reach_a_sink_with_their_fields_body 1588-1669
mutants crates/pull/src/fnowork_futures_tests.rs line 108
mutants crates/pull/src/fnowork_futures_tests.rs line 109
mutants crates/pull/src/fnowork_futures_tests.rs line 110
mutants crates/pull/src/fnowork_futures_tests.rs line 154
mutants crates/pull/src/fnowork_futures_tests.rs line 155
mutants crates/pull/src/fnowork_futures_tests.rs line 156
mutants crates/pull/src/fnowork_futures_tests.rs line 157
mutants crates/pull/src/fnowork_futures_tests.rs line 1570
mutants crates/pull/src/fnowork_futures_tests.rs line 1571
mutants crates/pull/src/fnowork_futures_tests.rs line 1572
mutants crates/pull/src/fnowork_futures_tests.rs line 1587
mutants crates/pull/src/fold.rs line 2693
mutants crates/pull/src/fold.rs line 2694
mutants crates/pull/src/fold.rs line 2695
mutants crates/pull/src/fold.rs line 2696
mutants crates/pull/src/fold.rs line 2697
mutants crates/pull/src/fold_ps2_tests.rs whole
mutants crates/pull/src/gap_backfill.rs month_lines 308-367
mutants crates/pull/src/gap_backfill.rs line 467
mutants crates/pull/src/gap_backfill.rs line 468
mutants crates/pull/src/gap_backfill.rs line 469
mutants crates/pull/src/gap_backfill.rs line 470
mutants crates/pull/src/gap_backfill_cov_tests.rs whole
mutants crates/pull/src/gaps.rs a_recorded_halt_is_not_owed_by_any_classifier_door 1106-1150
mutants crates/pull/src/gaps.rs a_derivatives_audit_owes_the_venues_dated_minutes 1152-1192
mutants crates/pull/src/gaps.rs cash_auction_flags_are_dated_and_missing_metadata_is_not_a_clean_day 1221-1270
mutants crates/pull/src/gaps.rs cash_ineligible_tail_and_entire_absent_day_remain_expected_gaps 1272-1291
mutants crates/pull/src/gaps.rs the_gap_between_two_windows_is_not_a_loss_but_a_hole_inside_one_is 1414-1448
mutants crates/pull/src/gaps.rs line 1694
mutants crates/pull/src/gaps.rs line 1695
mutants crates/pull/src/gaps.rs line 1696
mutants crates/pull/src/gaps_cov_tests.rs whole
mutants crates/pull/src/gdfl_archive.rs empty_zip64_record 2198-2208
mutants crates/pull/src/gdfl_archive.rs zip64_locator 2210-2217
mutants crates/pull/src/gdfl_archive.rs an_end_record_too_near_the_start_for_a_zip64_record_reads_its_locator_alone 2219-2248
mutants crates/pull/src/gdfl_archive.rs with_extensible_data 2250-2263
mutants crates/pull/src/gdfl_archive.rs a_zip64_record_with_extensible_data_is_read_where_its_locator_points 2265-2283
mutants crates/pull/src/gdfl_archive.rs a_prefixed_zip_whose_zip64_record_carries_extensible_data_is_refused 2285-2300
mutants crates/pull/src/gdfl_archive.rs an_empty_zip_behind_a_prefix_is_refused_by_the_prefix 2302-2324
mutants crates/pull/src/gdfl_archive.rs line 2195
mutants crates/pull/src/gdfl_archive.rs line 2196
mutants crates/pull/src/gdfl_archive.rs line 2197
mutants crates/pull/src/gdfl_archive.rs line 2209
mutants crates/pull/src/gdfl_archive.rs line 2218
mutants crates/pull/src/gdfl_archive.rs line 2249
mutants crates/pull/src/gdfl_archive.rs line 2264
mutants crates/pull/src/gdfl_archive.rs line 2284
mutants crates/pull/src/gdfl_archive.rs line 2301
mutants crates/pull/src/gdfl_archive.rs line 2325
mutants crates/pull/src/gdfl_backfill_tests.rs a_day_a_held_month_lacks_lands_as_a_new_generation_every_reader_opens 187-343
mutants crates/pull/src/gdfl_cm.rs line 6293
mutants crates/pull/src/gdfl_cm.rs line 6294
mutants crates/pull/src/gdfl_cm.rs line 6295
mutants crates/pull/src/gdfl_cm.rs line 6296
mutants crates/pull/src/gdfl_cm.rs line 6297
mutants crates/pull/src/gdfl_cm.rs line 6298
mutants crates/pull/src/gdfl_cm.rs line 6299
mutants crates/pull/src/gdfl_cm_admission_tests.rs only_banknifty_absent_refuses_any_other_failure 1854-1872
mutants crates/pull/src/gdfl_cm_admission_tests.rs line 1853
mutants crates/pull/src/gdfl_cm_auction.rs append 585-610
mutants crates/pull/src/gdfl_cm_auction.rs append_with 621-676
mutants crates/pull/src/gdfl_cm_auction.rs write_day 678-782
mutants crates/pull/src/gdfl_cm_auction.rs line 611
mutants crates/pull/src/gdfl_cm_auction.rs line 612
mutants crates/pull/src/gdfl_cm_auction.rs line 613
mutants crates/pull/src/gdfl_cm_auction.rs line 614
mutants crates/pull/src/gdfl_cm_auction.rs line 615
mutants crates/pull/src/gdfl_cm_auction.rs line 616
mutants crates/pull/src/gdfl_cm_auction.rs line 617
mutants crates/pull/src/gdfl_cm_auction.rs line 618
mutants crates/pull/src/gdfl_cm_auction.rs line 619
mutants crates/pull/src/gdfl_cm_auction.rs line 620
mutants crates/pull/src/gdfl_cm_auction.rs line 1179
mutants crates/pull/src/gdfl_cm_auction.rs line 1180
mutants crates/pull/src/gdfl_cm_auction.rs line 1181
mutants crates/pull/src/gdfl_cm_auction.rs line 1182
mutants crates/pull/src/gdfl_cm_auction.rs line 1183
mutants crates/pull/src/gdfl_cm_auction.rs line 1184
mutants crates/pull/src/gdfl_cm_auction_ps1_tests.rs whole
mutants crates/pull/src/gdfl_cm_auction_v2_tests.rs beside_an_append 405-421
mutants crates/pull/src/gdfl_cm_auction_v2_tests.rs a_reader_beside_a_concurrent_append_sees_whole_days_only 423-462
mutants crates/pull/src/gdfl_cm_auction_v2_tests.rs line 422
mutants crates/pull/src/gdfl_cm_ps2_tests.rs whole
mutants crates/pull/src/gdfl_dayzips.rs line 498
mutants crates/pull/src/gdfl_dayzips.rs line 499
mutants crates/pull/src/gdfl_dayzips.rs line 500
mutants crates/pull/src/gdfl_dayzips.rs line 501
mutants crates/pull/src/gdfl_dayzips.rs line 502
mutants crates/pull/src/gdfl_dayzips.rs line 503
mutants crates/pull/src/gdfl_dayzips_ps2_tests.rs whole
mutants crates/pull/src/gdfl_dropbox.rs proxy_in_use 578-582
mutants crates/pull/src/gdfl_dropbox.rs proxy_named_by 584-596
mutants crates/pull/src/gdfl_dropbox.rs status 1670-1727
mutants crates/pull/src/gdfl_dropbox.rs press 2003-2068
mutants crates/pull/src/gdfl_dropbox.rs drop 2080-2085
mutants crates/pull/src/gdfl_dropbox.rs lock_staging 2088-2139
mutants crates/pull/src/gdfl_dropbox.rs write_body 3223-3277
mutants crates/pull/src/gdfl_dropbox.rs at_step 3313-3323
mutants crates/pull/src/gdfl_dropbox.rs line 583
mutants crates/pull/src/gdfl_dropbox.rs line 2070
mutants crates/pull/src/gdfl_dropbox.rs line 2071
mutants crates/pull/src/gdfl_dropbox.rs line 2072
mutants crates/pull/src/gdfl_dropbox.rs line 2073
mutants crates/pull/src/gdfl_dropbox.rs line 2074
mutants crates/pull/src/gdfl_dropbox.rs line 2075
mutants crates/pull/src/gdfl_dropbox.rs line 2076
mutants crates/pull/src/gdfl_dropbox.rs line 2077
mutants crates/pull/src/gdfl_dropbox.rs line 2078
mutants crates/pull/src/gdfl_dropbox.rs line 2079
mutants crates/pull/src/gdfl_dropbox.rs line 2086
mutants crates/pull/src/gdfl_dropbox.rs line 2087
mutants crates/pull/src/gdfl_dropbox.rs line 3304
mutants crates/pull/src/gdfl_dropbox.rs line 3305
mutants crates/pull/src/gdfl_dropbox.rs line 3306
mutants crates/pull/src/gdfl_dropbox.rs line 3307
mutants crates/pull/src/gdfl_dropbox.rs line 3308
mutants crates/pull/src/gdfl_dropbox.rs line 3309
mutants crates/pull/src/gdfl_dropbox.rs line 3310
mutants crates/pull/src/gdfl_dropbox.rs line 3311
mutants crates/pull/src/gdfl_dropbox.rs line 3312
mutants crates/pull/src/gdfl_dropbox.rs line 3324
mutants crates/pull/src/gdfl_dropbox.rs line 3426
mutants crates/pull/src/gdfl_dropbox.rs line 3427
mutants crates/pull/src/gdfl_dropbox.rs line 3428
mutants crates/pull/src/gdfl_dropbox.rs line 3429
mutants crates/pull/src/gdfl_dropbox.rs line 3430
mutants crates/pull/src/gdfl_dropbox.rs line 3431
mutants crates/pull/src/gdfl_dropbox_cov_tests.rs whole
mutants crates/pull/src/gdfl_dropbox_tests.rs start 308-326
mutants crates/pull/src/gdfl_dropbox_tests.rs s1_enospc_mid_body_leaves_a_named_part_and_resumes 1936-2009
mutants crates/pull/src/gdfl_dropbox_tests.rs s1_a_stop_mid_download_keeps_the_part_and_the_next_press_resumes_it 2072-2165
mutants crates/pull/src/gdfl_dropbox_tests.rs refused_code 2302-2308
mutants crates/pull/src/gdfl_dropbox_tests.rs a_duplicate_refused_by_its_sha256_stays_refused_on_every_later_press 2310-2376
mutants crates/pull/src/gdfl_dropbox_tests.rs an_endless_empty_listing_is_refused_after_a_bounded_number_of_pages 2421-2456
mutants crates/pull/src/gdfl_dropbox_tests.rs a_journal_that_cannot_be_written_stops_the_press_before_any_byte 2764-2776
mutants crates/pull/src/gdfl_dropbox_tests.rs a_journal_that_cannot_be_written_stops_the_press_before_any_byte_body 2778-2807
mutants crates/pull/src/gdfl_dropbox_tests.rs a_read_only_store_root_is_not_writable_not_lost 2872-2881
mutants crates/pull/src/gdfl_dropbox_tests.rs a_read_only_store_root_is_not_writable_not_lost_body 2883-2898
mutants crates/pull/src/gdfl_dropbox_tests.rs a_line_torn_at_a_field_boundary_is_counted_torn_and_never_read 3661-3753
mutants crates/pull/src/gdfl_dropbox_tests.rs serve 3774-3795
mutants crates/pull/src/gdfl_dropbox_tests.rs page 3797-3802
mutants crates/pull/src/gdfl_dropbox_tests.rs file_entry 3804-3806
mutants crates/pull/src/gdfl_dropbox_tests.rs folder_entry 3808-3810
mutants crates/pull/src/gdfl_dropbox_tests.rs status_only 3812-3814
mutants crates/pull/src/gdfl_dropbox_tests.rs api_result 3816-3821
mutants crates/pull/src/gdfl_dropbox_tests.rs whole 3830-3837
mutants crates/pull/src/gdfl_dropbox_tests.rs folder_of 3839-3864
mutants crates/pull/src/gdfl_dropbox_tests.rs press_on 3866-3892
mutants crates/pull/src/gdfl_dropbox_tests.rs go 3894-3904
mutants crates/pull/src/gdfl_dropbox_tests.rs refusal 3906-3911
mutants crates/pull/src/gdfl_dropbox_tests.rs downloads 3913-3920
mutants crates/pull/src/gdfl_dropbox_tests.rs crashed_part 3922-3938
mutants crates/pull/src/gdfl_dropbox_tests.rs space 3950-3958
mutants crates/pull/src/gdfl_dropbox_tests.rs a_second_401_after_a_fresh_token_is_a_dead_token 3963-3987
mutants crates/pull/src/gdfl_dropbox_tests.rs a_5xx_storm_past_the_total_wait_is_server_error_by_the_total 3989-4022
mutants crates/pull/src/gdfl_dropbox_tests.rs a_redirect_chain_past_two_hops_is_refused_by_its_last_location 4024-4062
mutants crates/pull/src/gdfl_dropbox_tests.rs a_first_page_names_the_link_and_every_later_failure_names_the_listing 4066-4124
mutants crates/pull/src/gdfl_dropbox_tests.rs a_folder_deeper_than_eight_is_refused_and_eight_deep_is_listed 4126-4173
mutants crates/pull/src/gdfl_dropbox_tests.rs a_root_of_4096_folders_is_refused_before_its_last_folder 4175-4199
mutants crates/pull/src/gdfl_dropbox_tests.rs pages_that_add_entries_but_keep_their_cursor_stall_and_name_their_folder 4201-4230
mutants crates/pull/src/gdfl_dropbox_tests.rs an_mcx_zip_a_size_past_a_tebibyte_and_a_held_stocks_day_are_named 4234-4282
mutants crates/pull/src/gdfl_dropbox_tests.rs a_held_day_listed_without_a_hash_is_held_not_redelivered 4284-4316
mutants crates/pull/src/gdfl_dropbox_tests.rs a_pending_shrink_at_another_baseline_accepts_nothing 4318-4366
mutants crates/pull/src/gdfl_dropbox_tests.rs a_staging_folder_that_is_a_file_is_store_lost 4370-4398
mutants crates/pull/src/gdfl_dropbox_tests.rs a_journal_under_a_file_is_unreadable_unwritable_and_named_by_status 4400-4438
mutants crates/pull/src/gdfl_dropbox_tests.rs an_import_journal_holding_a_foreign_line_is_journal_unreadable 4440-4461
mutants crates/pull/src/gdfl_dropbox_tests.rs a_tick_store_inside_the_staging_root_is_refused 4463-4483
mutants crates/pull/src/gdfl_dropbox_tests.rs the_drive_unplugged_by_the_last_hand_off_is_store_lost 4485-4519
mutants crates/pull/src/gdfl_dropbox_tests.rs a_stop_read_before_the_first_file_asks_for_nothing 4523-4559
mutants crates/pull/src/gdfl_dropbox_tests.rs a_stop_read_as_a_download_is_sent_keeps_the_empty_part_for_the_next_press 4561-4608
mutants crates/pull/src/gdfl_dropbox_tests.rs a_download_dropped_every_time_is_refused_after_its_resumes 4610-4644
mutants crates/pull/src/gdfl_dropbox_tests.rs a_throttled_download_refuses_the_run_by_its_own_code 4646-4667
mutants crates/pull/src/gdfl_dropbox_tests.rs a_store_lost_between_two_tries_of_one_file_is_store_lost 4669-4694
mutants crates/pull/src/gdfl_dropbox_tests.rs a_part_gone_before_its_body_is_written_is_staging_write_failed 4696-4726
mutants crates/pull/src/gdfl_dropbox_tests.rs a_part_whose_status_or_sync_fails_is_staging_write_failed 4728-4759
mutants crates/pull/src/gdfl_dropbox_tests.rs a_resumed_part_longer_than_the_listing_starts_a_new_part 4761-4793
mutants crates/pull/src/gdfl_dropbox_tests.rs a_verified_part_whose_name_is_taken_is_never_renamed_over_it 4795-4826
mutants crates/pull/src/gdfl_dropbox_tests.rs a_resume_answered_416_or_with_another_total_restarts_into_a_new_part 4828-4885
mutants crates/pull/src/gdfl_dropbox_tests.rs a_download_answered_403_is_refused_by_its_status 4887-4903
mutants crates/pull/src/gdfl_dropbox_tests.rs a_served_revision_is_checked_only_when_both_sides_name_one 4905-4949
mutants crates/pull/src/gdfl_dropbox_tests.rs a_body_past_the_listed_size_is_refused_as_it_comes 4951-4977
mutants crates/pull/src/gdfl_dropbox_tests.rs a_body_that_ends_early_and_cleanly_is_resumed_by_range 4979-5028
mutants crates/pull/src/gdfl_dropbox_tests.rs a_disk_full_exactly_at_the_last_byte_still_takes_the_file 5030-5050
mutants crates/pull/src/gdfl_dropbox_tests.rs a_download_answered_404_is_remote_gone_and_revokes_the_link 5052-5094
mutants crates/pull/src/gdfl_dropbox_tests.rs hand 5105-5116
mutants crates/pull/src/gdfl_dropbox_tests.rs a_file_deleted_upstream_after_its_listing_is_remote_gone 5119-5147
mutants crates/pull/src/gdfl_dropbox_tests.rs line 2777
mutants crates/pull/src/gdfl_dropbox_tests.rs line 2882
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3754
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3755
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3756
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3757
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3758
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3759
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3760
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3761
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3762
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3763
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3764
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3765
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3766
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3767
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3768
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3769
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3770
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3771
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3772
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3773
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3796
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3803
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3807
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3811
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3815
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3822
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3823
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3824
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3825
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3826
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3827
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3828
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3829
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3838
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3865
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3893
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3905
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3912
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3921
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3939
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3940
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3941
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3942
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3943
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3944
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3945
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3946
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3947
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3948
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3949
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3959
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3960
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3961
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3962
mutants crates/pull/src/gdfl_dropbox_tests.rs line 3988
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4023
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4063
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4064
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4065
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4125
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4174
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4200
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4231
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4232
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4233
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4283
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4317
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4367
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4368
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4369
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4399
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4439
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4462
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4484
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4520
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4521
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4522
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4560
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4609
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4645
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4668
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4695
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4727
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4760
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4794
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4827
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4886
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4904
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4950
mutants crates/pull/src/gdfl_dropbox_tests.rs line 4978
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5029
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5051
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5095
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5096
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5097
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5098
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5099
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5100
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5101
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5102
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5103
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5104
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5117
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5118
mutants crates/pull/src/gdfl_dropbox_tests.rs line 5148
mutants crates/pull/src/gdfl_fixtures.rs line 281
mutants crates/pull/src/gdfl_fixtures.rs line 282
mutants crates/pull/src/gdfl_fixtures.rs line 283
mutants crates/pull/src/gdfl_fixtures.rs line 284
mutants crates/pull/src/gdfl_fixtures.rs line 285
mutants crates/pull/src/gdfl_fixtures_ps1_tests.rs whole
mutants crates/pull/src/gdfl_format.rs line 100
mutants crates/pull/src/gdfl_format.rs line 101
mutants crates/pull/src/gdfl_format.rs line 102
mutants crates/pull/src/gdfl_format.rs line 103
mutants crates/pull/src/gdfl_format.rs line 104
mutants crates/pull/src/gdfl_format_ps2_tests.rs whole
mutants crates/pull/src/gdfl_ia16_tests.rs day_passed 1367-1374
mutants crates/pull/src/gdfl_ia16_tests.rs entries_under_counts_every_entry_of_a_tree 1378-1389
mutants crates/pull/src/gdfl_ia16_tests.rs an_option_file_with_rows_refused_one_by_one_is_admitted_not_failed_whole 2066-2151
mutants crates/pull/src/gdfl_ia16_tests.rs drive_nifty 2408-2451
mutants crates/pull/src/gdfl_ia16_tests.rs a_full_held_back_set_ends_the_run_by_name 2453-2495
mutants crates/pull/src/gdfl_ia16_tests.rs an_immutable_month_stops_the_run_by_name 2690-2706
mutants crates/pull/src/gdfl_ia16_tests.rs an_immutable_month_stops_the_run_by_name_body 2708-2776
mutants crates/pull/src/gdfl_ia16_tests.rs line 1390
mutants crates/pull/src/gdfl_ia16_tests.rs line 2707
mutants crates/pull/src/gdfl_import.rs kind 736-738
mutants crates/pull/src/gdfl_import.rs name 739-741
mutants crates/pull/src/gdfl_import.rs location 742-744
mutants crates/pull/src/gdfl_import.rs bytes 745-747
mutants crates/pull/src/gdfl_import.rs yearly_zips 750-767
mutants crates/pull/src/gdfl_import.rs holds_a_live_contract 769-795
mutants crates/pull/src/gdfl_import.rs measure_zips 797-858
mutants crates/pull/src/gdfl_import.rs measure_tick_store 860-865
mutants crates/pull/src/gdfl_import.rs measure_tick_store_within 867-873
mutants crates/pull/src/gdfl_import.rs walk_tick_store 875-963
mutants crates/pull/src/gdfl_import.rs volume_space 1037-1066
mutants crates/pull/src/gdfl_import.rs space_of 1068-1088
mutants crates/pull/src/gdfl_import.rs split_underlyings 1878-1938
mutants crates/pull/src/gdfl_import.rs names_a_split 1940-1944
mutants crates/pull/src/gdfl_import.rs run_cm_from 2203-2296
mutants crates/pull/src/gdfl_import.rs cm_jobs 2521-2617
mutants crates/pull/src/gdfl_import.rs note_unattributed 2686-2694
mutants crates/pull/src/gdfl_import.rs unattributed_event 2696-2716
mutants crates/pull/src/gdfl_import.rs warm_up_begin 3222-3230
mutants crates/pull/src/gdfl_import.rs warm_files_on 3232-3277
mutants crates/pull/src/gdfl_import.rs line 720
mutants crates/pull/src/gdfl_import.rs line 721
mutants crates/pull/src/gdfl_import.rs line 722
mutants crates/pull/src/gdfl_import.rs line 723
mutants crates/pull/src/gdfl_import.rs line 724
mutants crates/pull/src/gdfl_import.rs line 725
mutants crates/pull/src/gdfl_import.rs line 726
mutants crates/pull/src/gdfl_import.rs line 727
mutants crates/pull/src/gdfl_import.rs line 728
mutants crates/pull/src/gdfl_import.rs line 729
mutants crates/pull/src/gdfl_import.rs line 730
mutants crates/pull/src/gdfl_import.rs line 731
mutants crates/pull/src/gdfl_import.rs line 732
mutants crates/pull/src/gdfl_import.rs line 733
mutants crates/pull/src/gdfl_import.rs line 734
mutants crates/pull/src/gdfl_import.rs line 735
mutants crates/pull/src/gdfl_import.rs line 748
mutants crates/pull/src/gdfl_import.rs line 749
mutants crates/pull/src/gdfl_import.rs line 768
mutants crates/pull/src/gdfl_import.rs line 796
mutants crates/pull/src/gdfl_import.rs line 866
mutants crates/pull/src/gdfl_import.rs line 874
mutants crates/pull/src/gdfl_import.rs line 1067
mutants crates/pull/src/gdfl_import.rs line 1945
mutants crates/pull/src/gdfl_import.rs line 2695
mutants crates/pull/src/gdfl_import.rs line 3231
mutants crates/pull/src/gdfl_import.rs line 3771
mutants crates/pull/src/gdfl_import.rs line 3772
mutants crates/pull/src/gdfl_import.rs line 3773
mutants crates/pull/src/gdfl_import.rs line 3774
mutants crates/pull/src/gdfl_import.rs line 3775
mutants crates/pull/src/gdfl_import.rs line 3776
mutants crates/pull/src/gdfl_import.rs line 3806
mutants crates/pull/src/gdfl_import.rs line 3807
mutants crates/pull/src/gdfl_import.rs line 3808
mutants crates/pull/src/gdfl_import.rs line 3809
mutants crates/pull/src/gdfl_import.rs line 3810
mutants crates/pull/src/gdfl_import.rs line 3811
mutants crates/pull/src/gdfl_import.rs line 3812
mutants crates/pull/src/gdfl_import_causes.rs fmt 437-663
mutants crates/pull/src/gdfl_import_causes.rs stop_said 666-699
mutants crates/pull/src/gdfl_import_causes.rs door_said 701-743
mutants crates/pull/src/gdfl_import_census.rs line 284
mutants crates/pull/src/gdfl_import_census.rs line 285
mutants crates/pull/src/gdfl_import_census.rs line 286
mutants crates/pull/src/gdfl_import_census.rs line 287
mutants crates/pull/src/gdfl_import_census.rs line 288
mutants crates/pull/src/gdfl_import_census.rs line 289
mutants crates/pull/src/gdfl_import_census_ps2_tests.rs whole
mutants crates/pull/src/gdfl_import_cov_tests.rs whole
mutants crates/pull/src/gdfl_import_day.rs fresh_root 1541-1553
mutants crates/pull/src/gdfl_import_day.rs fresh_root_with 1555-1596
mutants crates/pull/src/gdfl_import_day.rs line 1554
mutants crates/pull/src/gdfl_import_day.rs line 2510
mutants crates/pull/src/gdfl_import_day.rs line 2511
mutants crates/pull/src/gdfl_import_day.rs line 2512
mutants crates/pull/src/gdfl_import_day.rs line 2513
mutants crates/pull/src/gdfl_import_day.rs line 2514
mutants crates/pull/src/gdfl_import_day.rs line 2515
mutants crates/pull/src/gdfl_import_day_ps1_tests.rs whole
mutants crates/pull/src/gdfl_import_filing.rs on_workers 278-370
mutants crates/pull/src/gdfl_import_filing.rs file 618-772
mutants crates/pull/src/gdfl_import_filing.rs refuse_spawn 1583-1587
mutants crates/pull/src/gdfl_import_filing.rs spawn_refused 1589-1599
mutants crates/pull/src/gdfl_import_filing.rs plant_before_confirm 1669-1679
mutants crates/pull/src/gdfl_import_filing.rs before_confirm 1681-1695
mutants crates/pull/src/gdfl_import_filing.rs line 1576
mutants crates/pull/src/gdfl_import_filing.rs line 1577
mutants crates/pull/src/gdfl_import_filing.rs line 1578
mutants crates/pull/src/gdfl_import_filing.rs line 1579
mutants crates/pull/src/gdfl_import_filing.rs line 1580
mutants crates/pull/src/gdfl_import_filing.rs line 1581
mutants crates/pull/src/gdfl_import_filing.rs line 1582
mutants crates/pull/src/gdfl_import_filing.rs line 1588
mutants crates/pull/src/gdfl_import_filing.rs line 1600
mutants crates/pull/src/gdfl_import_filing.rs line 1662
mutants crates/pull/src/gdfl_import_filing.rs line 1663
mutants crates/pull/src/gdfl_import_filing.rs line 1664
mutants crates/pull/src/gdfl_import_filing.rs line 1665
mutants crates/pull/src/gdfl_import_filing.rs line 1666
mutants crates/pull/src/gdfl_import_filing.rs line 1667
mutants crates/pull/src/gdfl_import_filing.rs line 1668
mutants crates/pull/src/gdfl_import_filing.rs line 1680
mutants crates/pull/src/gdfl_import_filing.rs line 1696
mutants crates/pull/src/gdfl_import_filing.rs line 1725
mutants crates/pull/src/gdfl_import_filing.rs line 1726
mutants crates/pull/src/gdfl_import_filing.rs line 1727
mutants crates/pull/src/gdfl_import_filing.rs line 1728
mutants crates/pull/src/gdfl_import_filing.rs line 1729
mutants crates/pull/src/gdfl_import_filing.rs line 1730
mutants crates/pull/src/gdfl_import_filing_ps2_tests.rs whole
mutants crates/pull/src/gdfl_import_journal.rs current_for 162-172
mutants crates/pull/src/gdfl_import_journal.rs journal_day 174-187
mutants crates/pull/src/gdfl_import_journal.rs make_leaf_dir 1417-1433
mutants crates/pull/src/gdfl_import_journal.rs make_leaf_dir_with 1435-1460
mutants crates/pull/src/gdfl_import_journal.rs line 1434
mutants crates/pull/src/gdfl_import_journal.rs line 1603
mutants crates/pull/src/gdfl_import_journal.rs line 1604
mutants crates/pull/src/gdfl_import_journal.rs line 1605
mutants crates/pull/src/gdfl_import_journal.rs line 1606
mutants crates/pull/src/gdfl_import_journal.rs line 1607
mutants crates/pull/src/gdfl_import_journal.rs line 1608
mutants crates/pull/src/gdfl_import_journal.rs line 1609
mutants crates/pull/src/gdfl_import_journal_cov_tests.rs whole
mutants crates/pull/src/gdfl_import_lineage.rs day 580-679
mutants crates/pull/src/gdfl_import_lineage.rs line 705
mutants crates/pull/src/gdfl_import_lineage.rs line 706
mutants crates/pull/src/gdfl_import_lineage.rs line 707
mutants crates/pull/src/gdfl_import_lineage.rs line 708
mutants crates/pull/src/gdfl_import_lineage.rs line 709
mutants crates/pull/src/gdfl_import_lineage.rs line 710
mutants crates/pull/src/gdfl_import_lineage.rs line 711
mutants crates/pull/src/gdfl_import_lineage_cov_tests.rs whole
mutants crates/pull/src/gdfl_import_ps1_tests.rs whole
mutants crates/pull/src/gdfl_import_quality.rs line 1044
mutants crates/pull/src/gdfl_import_quality.rs line 1045
mutants crates/pull/src/gdfl_import_quality.rs line 1046
mutants crates/pull/src/gdfl_import_quality.rs line 1047
mutants crates/pull/src/gdfl_import_quality.rs line 1048
mutants crates/pull/src/gdfl_import_quality.rs line 1049
mutants crates/pull/src/gdfl_import_quality_ps2_tests.rs whole
mutants crates/pull/src/gdfl_import_split_tests.rs the_split_moves_lines_and_changes_none 891-909
mutants crates/pull/src/gdfl_import_split_tests.rs prove_split 911-958
mutants crates/pull/src/gdfl_import_split_tests.rs split_text 960-973
mutants crates/pull/src/gdfl_import_split_tests.rs lost_and_added 975-986
mutants crates/pull/src/gdfl_import_split_tests.rs the_split_proof_reads_the_tree_the_history_or_nothing 988-1009
mutants crates/pull/src/gdfl_import_split_tests.rs lines_lost_and_added_are_each_named 1011-1022
mutants crates/pull/src/gdfl_import_split_tests.rs the_split_proofs_helpers_hold_at_their_edges 1042-1069
mutants crates/pull/src/gdfl_import_split_tests.rs line 25
mutants crates/pull/src/gdfl_import_split_tests.rs line 910
mutants crates/pull/src/gdfl_import_split_tests.rs line 974
mutants crates/pull/src/gdfl_import_split_tests.rs line 987
mutants crates/pull/src/gdfl_import_split_tests.rs line 1010
mutants crates/pull/src/gdfl_import_split_tests.rs line 1023
mutants crates/pull/src/gdfl_import_split_tests.rs line 1041
mutants crates/pull/src/gdfl_import_tests.rs line_of 761-769
mutants crates/pull/src/gdfl_import_tests.rs reference 2737-2817
mutants crates/pull/src/gdfl_import_tests.rs bars_tree_has_no_day 4428-4446
mutants crates/pull/src/gdfl_import_tests.rs store_bytes 4485-4524
mutants crates/pull/src/gdfl_import_tests.rs a_month_cut_off_in_its_creation_is_counted_unborn 4963-4993
mutants crates/pull/src/gdfl_import_tests.rs a_kill_at_any_point_leaves_no_half_month_and_the_resume_lands_the_same_store 4995-5195
mutants crates/pull/src/gdfl_import_tests.rs a_day_flush_hook_answers_only_the_flushes_of_its_own_root 5284-5330
mutants crates/pull/src/gdfl_import_tests.rs a_day_that_cannot_be_made_durable_stops_the_run_and_stays_open 5468-5580
mutants crates/pull/src/gdfl_import_tests.rs a_back_fill_is_published_outside_the_days_batch_and_leaves_one_workers_store 5915-6041
mutants crates/pull/src/gdfl_import_tests.rs two_runs_into_one_store_at_once_leave_what_one_run_leaves 6118-6166
mutants crates/pull/src/gdfl_import_tests.rs a_run_beside_another_names_the_lock 6168-6191
mutants crates/pull/src/gdfl_import_tests.rs a_run_that_meets_a_month_another_holds_names_the_holder_and_goes_on 6193-6249
mutants crates/pull/src/gdfl_import_tests.rs a_stocks_run_that_meets_a_sidecar_another_holds_names_the_lock 6251-6344
mutants crates/pull/src/gdfl_import_tests.rs a_run_that_meets_the_census_lock_another_holds_is_refused_by_name 6346-6409
mutants crates/pull/src/gdfl_import_tests.rs a_file_the_reader_refuses_is_named_and_the_days_other_files_are_read 6411-6453
mutants crates/pull/src/gdfl_import_tests.rs line 4994
mutants crates/pull/src/gdfl_import_tests.rs line 5331
mutants crates/pull/src/gdfl_import_tests.rs line 6167
mutants crates/pull/src/gdfl_import_tests.rs line 6192
mutants crates/pull/src/gdfl_import_tests.rs line 6250
mutants crates/pull/src/gdfl_import_tests.rs line 6345
mutants crates/pull/src/gdfl_import_tests.rs line 6410
mutants crates/pull/src/gdfl_lineage_tests.rs two_keys 1452-1461
mutants crates/pull/src/gdfl_lineage_tests.rs a_zero_lot_reduces_to_no_factor_and_reads_back_only_as_such 1463-1504
mutants crates/pull/src/gdfl_lineage_tests.rs a_stitched_partner_that_names_another_underlying_is_refused 1506-1540
mutants crates/pull/src/gdfl_lineage_tests.rs a_lot_or_retired_line_of_another_version_is_refused 1542-1576
mutants crates/pull/src/gdfl_lineage_tests.rs a_partner_whose_segment_cannot_be_written_leaves_its_key_unpaired 1578-1609
mutants crates/pull/src/gdfl_lineage_tests.rs an_unwritable_pre_ex_key_is_passed_over_by_the_pairing 1611-1633
mutants crates/pull/src/gdfl_lineage_tests.rs a_sourced_relisting_day_that_relists_nothing_is_no_event 1635-1655
mutants crates/pull/src/gdfl_lineage_tests.rs lives 1661-1671
mutants crates/pull/src/gdfl_lineage_tests.rs the_rename_census_counts_each_key_by_its_symbols_in_life_order 1673-1705
mutants crates/pull/src/gdfl_lineage_tests.rs the_rename_census_refuses_two_lives_of_one_key_that_overlap 1707-1722
mutants crates/pull/src/gdfl_lineage_tests.rs line 1446
mutants crates/pull/src/gdfl_lineage_tests.rs line 1447
mutants crates/pull/src/gdfl_lineage_tests.rs line 1448
mutants crates/pull/src/gdfl_lineage_tests.rs line 1449
mutants crates/pull/src/gdfl_lineage_tests.rs line 1450
mutants crates/pull/src/gdfl_lineage_tests.rs line 1451
mutants crates/pull/src/gdfl_lineage_tests.rs line 1462
mutants crates/pull/src/gdfl_lineage_tests.rs line 1505
mutants crates/pull/src/gdfl_lineage_tests.rs line 1541
mutants crates/pull/src/gdfl_lineage_tests.rs line 1577
mutants crates/pull/src/gdfl_lineage_tests.rs line 1610
mutants crates/pull/src/gdfl_lineage_tests.rs line 1634
mutants crates/pull/src/gdfl_lineage_tests.rs line 1656
mutants crates/pull/src/gdfl_lineage_tests.rs line 1657
mutants crates/pull/src/gdfl_lineage_tests.rs line 1658
mutants crates/pull/src/gdfl_lineage_tests.rs line 1659
mutants crates/pull/src/gdfl_lineage_tests.rs line 1660
mutants crates/pull/src/gdfl_lineage_tests.rs line 1672
mutants crates/pull/src/gdfl_lineage_tests.rs line 1706
mutants crates/pull/src/gdfl_nfo.rs line 2035
mutants crates/pull/src/gdfl_nfo.rs line 2036
mutants crates/pull/src/gdfl_nfo.rs line 2037
mutants crates/pull/src/gdfl_nfo.rs line 2038
mutants crates/pull/src/gdfl_nfo.rs line 2039
mutants crates/pull/src/gdfl_nfo_attack_tests.rs dpn_24_the_oracles_refuse_what_is_not_canonical 429-471
mutants crates/pull/src/gdfl_nfo_attack_tests.rs random_ticker 1019-1061
mutants crates/pull/src/gdfl_nfo_attack_tests.rs dpn_09_a_million_random_names_never_panic_and_every_answer_is_exact_or_named 1063-1145
mutants crates/pull/src/gdfl_nfo_attack_tests.rs dpn_13_a_hundred_thousand_bent_files_never_panic_and_refuse_at_a_line 1358-1460
mutants crates/pull/src/gdfl_nfo_attack_tests.rs line 472
mutants crates/pull/src/gdfl_nfo_ps1_tests.rs whole
mutants crates/pull/src/gdfl_nfo_tests.rs day 314-316
mutants crates/pull/src/gdfl_nfo_tests.rs an_expiry_before_the_trade_day_or_past_the_horizon_is_refused 386-413
mutants crates/pull/src/gdfl_nfo_tests.rs every_fault_of_the_yearly_zip_is_refused_by_name 1142-1201
mutants crates/pull/src/gdfl_nfo_tests.rs a_misfiled_or_unread_options_day_zip_is_never_read_as_not_held 1952-1970
mutants crates/pull/src/gdfl_nfo_tests.rs a_misfiled_or_unread_options_day_zip_is_never_read_as_not_held_body 1972-2177
mutants crates/pull/src/gdfl_nfo_tests.rs decode_by_field_vector 2470-2475
mutants crates/pull/src/gdfl_nfo_tests.rs decode_by_field_vector_capped 2477-2571
mutants crates/pull/src/gdfl_nfo_tests.rs the_oracle_and_the_relation_meet_every_shape_on_invented_files 2657-2792
mutants crates/pull/src/gdfl_nfo_tests.rs line 307
mutants crates/pull/src/gdfl_nfo_tests.rs line 308
mutants crates/pull/src/gdfl_nfo_tests.rs line 1971
mutants crates/pull/src/gdfl_nfo_tests.rs line 2476
mutants crates/pull/src/gdfl_nfo_tests.rs line 2653
mutants crates/pull/src/gdfl_nfo_tests.rs line 2654
mutants crates/pull/src/gdfl_nfo_tests.rs line 2655
mutants crates/pull/src/gdfl_nfo_tests.rs line 2656
mutants crates/pull/src/gdfl_nfo_tests.rs line 2793
mutants crates/pull/src/gdfl_r2_attack_tests.rs reference 227-318
mutants crates/pull/src/gdfl_r2_attack_tests.rs tree 325-344
mutants crates/pull/src/gdfl_r2_attack_tests.rs want_nfo 709-847
mutants crates/pull/src/gdfl_r2_attack_tests.rs r2_03b_a_bent_file_with_no_other_row_is_refused_by_its_bent_row 1060-1131
mutants crates/pull/src/gdfl_r2_attack_tests.rs r2_04_random_stocks_worlds_match_the_reference_through_both_sources 1210-1401
mutants crates/pull/src/gdfl_r2_attack_tests.rs run_stocks_on 1405-1417
mutants crates/pull/src/gdfl_r2_attack_tests.rs r2_06_pre_cutover_listings_key_every_file_by_its_decoded_contract 1486-1675
mutants crates/pull/src/gdfl_r2_attack_tests.rs r2_07_every_table_month_on_every_weekday_around_its_expiry 1677-1736
mutants crates/pull/src/gdfl_r2_attack_tests.rs line 1132
mutants crates/pull/src/gdfl_r3_attack_tests.rs random_rows 123-177
mutants crates/pull/src/gdfl_r3_attack_tests.rs reference 190-259
mutants crates/pull/src/gdfl_r3_attack_tests.rs tree 265-287
mutants crates/pull/src/gdfl_r3_attack_tests.rs run_stocks_on 399-410
mutants crates/pull/src/gdfl_r3_attack_tests.rs r3_04_a_columnar_block_never_builds_past_its_stated_size 751-823
mutants crates/pull/src/gdfl_r3_attack_tests.rs want_cm 1009-1121
mutants crates/pull/src/gdfl_r4_attack_tests.rs bars_tree 79-97
mutants crates/pull/src/gdfl_r4_attack_tests.rs r4_10_a_journal_line_torn_inside_a_run_is_closed_before_the_next_one 617-683
mutants crates/pull/src/gdfl_r4_attack_tests.rs r4_11_a_journal_this_branch_wrote_before_definition_existed_is_resumed_as_its_own 697-821
mutants crates/pull/src/gdfl_rekey.rs run 2095-2149
mutants crates/pull/src/gdfl_rekey.rs apply_within 2189-2264
mutants crates/pull/src/gdfl_rekey.rs plant 2542-2545
mutants crates/pull/src/gdfl_rekey.rs at 2547-2556
mutants crates/pull/src/gdfl_rekey.rs fail 2558-2561
mutants crates/pull/src/gdfl_rekey.rs answer_lock 2563-2579
mutants crates/pull/src/gdfl_rekey.rs line 2519
mutants crates/pull/src/gdfl_rekey.rs line 2520
mutants crates/pull/src/gdfl_rekey.rs line 2521
mutants crates/pull/src/gdfl_rekey.rs line 2522
mutants crates/pull/src/gdfl_rekey.rs line 2523
mutants crates/pull/src/gdfl_rekey.rs line 2524
mutants crates/pull/src/gdfl_rekey.rs line 2525
mutants crates/pull/src/gdfl_rekey.rs line 2526
mutants crates/pull/src/gdfl_rekey.rs line 2527
mutants crates/pull/src/gdfl_rekey.rs line 2528
mutants crates/pull/src/gdfl_rekey.rs line 2529
mutants crates/pull/src/gdfl_rekey.rs line 2530
mutants crates/pull/src/gdfl_rekey.rs line 2531
mutants crates/pull/src/gdfl_rekey.rs line 2532
mutants crates/pull/src/gdfl_rekey.rs line 2533
mutants crates/pull/src/gdfl_rekey.rs line 2534
mutants crates/pull/src/gdfl_rekey.rs line 2535
mutants crates/pull/src/gdfl_rekey.rs line 2536
mutants crates/pull/src/gdfl_rekey.rs line 2537
mutants crates/pull/src/gdfl_rekey.rs line 2538
mutants crates/pull/src/gdfl_rekey.rs line 2539
mutants crates/pull/src/gdfl_rekey.rs line 2540
mutants crates/pull/src/gdfl_rekey.rs line 2541
mutants crates/pull/src/gdfl_rekey.rs line 2546
mutants crates/pull/src/gdfl_rekey.rs line 2557
mutants crates/pull/src/gdfl_rekey.rs line 2562
mutants crates/pull/src/gdfl_rekey.rs line 2580
mutants crates/pull/src/gdfl_rekey.rs line 2581
mutants crates/pull/src/gdfl_rekey.rs line 2585
mutants crates/pull/src/gdfl_rekey.rs line 2586
mutants crates/pull/src/gdfl_rekey.rs line 2587
mutants crates/pull/src/gdfl_rekey.rs line 2588
mutants crates/pull/src/gdfl_rekey.rs line 2589
mutants crates/pull/src/gdfl_rekey.rs line 2590
mutants crates/pull/src/gdfl_rekey_cov_tests.rs whole
mutants crates/pull/src/gdfl_rekey_ps2_tests.rs whole
mutants crates/pull/src/gdfl_rekey_tests.rs two_applies_at_once_leave_a_ledger_that_reads 2352-2405
mutants crates/pull/src/gdfl_rekey_tests.rs a_source_that_grew_after_the_survey_is_refused_not_moved_with_a_stale_count 2407-2442
mutants crates/pull/src/gdfl_rekey_tests.rs a_superseded_line_into_a_retired_key_or_a_cycle_is_refused_by_line 2444-2497
mutants crates/pull/src/gdfl_rekey_tests.rs line 2813
mutants crates/pull/src/gdfl_rekey_tests.rs line 2814
mutants crates/pull/src/gdfl_rekey_tests.rs line 2815
mutants crates/pull/src/gdfl_rekey_tests.rs line 2816
mutants crates/pull/src/gdfl_rekey_tests.rs line 2817
mutants crates/pull/src/gdfl_run.rs day_of 748-765
mutants crates/pull/src/gdfl_run.rs claim_verb 1849-1915
mutants crates/pull/src/gdfl_run.rs write 2743-2800
mutants crates/pull/src/gdfl_run.rs fail 2907-2910
mutants crates/pull/src/gdfl_run.rs answer 2912-2924
mutants crates/pull/src/gdfl_run.rs line 2893
mutants crates/pull/src/gdfl_run.rs line 2894
mutants crates/pull/src/gdfl_run.rs line 2895
mutants crates/pull/src/gdfl_run.rs line 2896
mutants crates/pull/src/gdfl_run.rs line 2897
mutants crates/pull/src/gdfl_run.rs line 2898
mutants crates/pull/src/gdfl_run.rs line 2899
mutants crates/pull/src/gdfl_run.rs line 2900
mutants crates/pull/src/gdfl_run.rs line 2901
mutants crates/pull/src/gdfl_run.rs line 2902
mutants crates/pull/src/gdfl_run.rs line 2903
mutants crates/pull/src/gdfl_run.rs line 2904
mutants crates/pull/src/gdfl_run.rs line 2905
mutants crates/pull/src/gdfl_run.rs line 2906
mutants crates/pull/src/gdfl_run.rs line 2911
mutants crates/pull/src/gdfl_run.rs line 2925
mutants crates/pull/src/gdfl_run.rs line 2926
mutants crates/pull/src/gdfl_run.rs line 2931
mutants crates/pull/src/gdfl_run.rs line 2932
mutants crates/pull/src/gdfl_run.rs line 2933
mutants crates/pull/src/gdfl_run.rs line 2934
mutants crates/pull/src/gdfl_run_cov_tests.rs whole
mutants crates/pull/src/gdfl_run_ia2_tests.rs every_id_the_runs_ledger_gives_is_written_down_twice 247-269
mutants crates/pull/src/gdfl_run_ia2_tests.rs every_id_the_runs_ledger_gives_is_written_down_twice_body 271-378
mutants crates/pull/src/gdfl_run_ia2_tests.rs line 270
mutants crates/pull/src/gdfl_run_ledger.rs allocate 330-476
mutants crates/pull/src/gdfl_run_ledger.rs plant 925-928
mutants crates/pull/src/gdfl_run_ledger.rs at 930-939
mutants crates/pull/src/gdfl_run_ledger.rs fail 941-944
mutants crates/pull/src/gdfl_run_ledger.rs answer 946-958
mutants crates/pull/src/gdfl_run_ledger.rs line 902
mutants crates/pull/src/gdfl_run_ledger.rs line 903
mutants crates/pull/src/gdfl_run_ledger.rs line 904
mutants crates/pull/src/gdfl_run_ledger.rs line 905
mutants crates/pull/src/gdfl_run_ledger.rs line 906
mutants crates/pull/src/gdfl_run_ledger.rs line 907
mutants crates/pull/src/gdfl_run_ledger.rs line 908
mutants crates/pull/src/gdfl_run_ledger.rs line 909
mutants crates/pull/src/gdfl_run_ledger.rs line 910
mutants crates/pull/src/gdfl_run_ledger.rs line 911
mutants crates/pull/src/gdfl_run_ledger.rs line 912
mutants crates/pull/src/gdfl_run_ledger.rs line 913
mutants crates/pull/src/gdfl_run_ledger.rs line 914
mutants crates/pull/src/gdfl_run_ledger.rs line 915
mutants crates/pull/src/gdfl_run_ledger.rs line 916
mutants crates/pull/src/gdfl_run_ledger.rs line 917
mutants crates/pull/src/gdfl_run_ledger.rs line 918
mutants crates/pull/src/gdfl_run_ledger.rs line 919
mutants crates/pull/src/gdfl_run_ledger.rs line 920
mutants crates/pull/src/gdfl_run_ledger.rs line 921
mutants crates/pull/src/gdfl_run_ledger.rs line 922
mutants crates/pull/src/gdfl_run_ledger.rs line 923
mutants crates/pull/src/gdfl_run_ledger.rs line 924
mutants crates/pull/src/gdfl_run_ledger.rs line 929
mutants crates/pull/src/gdfl_run_ledger.rs line 940
mutants crates/pull/src/gdfl_run_ledger.rs line 945
mutants crates/pull/src/gdfl_run_ledger.rs line 959
mutants crates/pull/src/gdfl_run_ledger.rs line 960
mutants crates/pull/src/gdfl_run_ledger.rs line 961
mutants crates/pull/src/gdfl_run_ledger.rs line 962
mutants crates/pull/src/gdfl_run_ledger.rs line 963
mutants crates/pull/src/gdfl_run_ledger.rs line 964
mutants crates/pull/src/gdfl_run_ledger.rs line 965
mutants crates/pull/src/gdfl_run_ledger.rs line 966
mutants crates/pull/src/gdfl_run_ledger.rs line 967
mutants crates/pull/src/gdfl_run_ledger.rs line 968
mutants crates/pull/src/gdfl_run_ledger.rs line 969
mutants crates/pull/src/gdfl_run_ledger.rs line 970
mutants crates/pull/src/gdfl_run_ledger_cov_tests.rs whole
mutants crates/pull/src/gdfl_run_ledger_ps2_tests.rs whole
mutants crates/pull/src/gdfl_run_locks.rs take 387-402
mutants crates/pull/src/gdfl_run_locks.rs take_cloning 404-465
mutants crates/pull/src/gdfl_run_locks.rs unlock_into 801-803
mutants crates/pull/src/gdfl_run_locks.rs unlock_into_with 805-819
mutants crates/pull/src/gdfl_run_locks.rs line 403
mutants crates/pull/src/gdfl_run_locks.rs line 804
mutants crates/pull/src/gdfl_run_locks.rs line 1458
mutants crates/pull/src/gdfl_run_locks.rs line 1459
mutants crates/pull/src/gdfl_run_locks.rs line 1460
mutants crates/pull/src/gdfl_run_locks.rs line 1461
mutants crates/pull/src/gdfl_run_locks.rs line 1462
mutants crates/pull/src/gdfl_run_locks.rs line 1463
mutants crates/pull/src/gdfl_run_locks.rs line 1464
mutants crates/pull/src/gdfl_run_locks.rs line 1465
mutants crates/pull/src/gdfl_run_locks.rs line 1466
mutants crates/pull/src/gdfl_run_locks.rs line 1467
mutants crates/pull/src/gdfl_run_locks.rs line 1468
mutants crates/pull/src/gdfl_run_locks_cov_tests.rs whole
mutants crates/pull/src/gdfl_run_locks_ps1_tests.rs whole
mutants crates/pull/src/gdfl_run_scope.rs line 133
mutants crates/pull/src/gdfl_run_scope.rs line 134
mutants crates/pull/src/gdfl_run_scope.rs line 135
mutants crates/pull/src/gdfl_run_scope.rs line 136
mutants crates/pull/src/gdfl_run_scope.rs line 137
mutants crates/pull/src/gdfl_run_scope.rs line 138
mutants crates/pull/src/gdfl_run_scope_cov_tests.rs whole
mutants crates/pull/src/gdfl_run_start.rs line 183
mutants crates/pull/src/gdfl_run_start.rs line 184
mutants crates/pull/src/gdfl_run_start.rs line 185
mutants crates/pull/src/gdfl_run_start.rs line 186
mutants crates/pull/src/gdfl_run_start.rs line 187
mutants crates/pull/src/gdfl_run_start.rs line 188
mutants crates/pull/src/gdfl_run_start_cov_tests.rs whole
mutants crates/pull/src/gdfl_run_stop.rs line 480
mutants crates/pull/src/gdfl_run_stop.rs line 481
mutants crates/pull/src/gdfl_run_stop.rs line 482
mutants crates/pull/src/gdfl_run_stop.rs line 483
mutants crates/pull/src/gdfl_run_stop_cov_tests.rs whole
mutants crates/pull/src/gdfl_run_tests.rs a_run_whose_sink_was_refused_refuses_before_any_lock 351-390
mutants crates/pull/src/gdfl_run_tests.rs sample 930-976
mutants crates/pull/src/gdfl_sealed_tests.rs a_late_day_for_a_sealed_month_lands_as_its_successor_generation 109-184
mutants crates/pull/src/gdfl_sealed_tests.rs a_day_a_sealed_month_holds_is_already_present_and_writes_nothing 186-230
mutants crates/pull/src/gdfl_sealed_tests.rs revisions_in 302-307
mutants crates/pull/src/gdfl_sealed_tests.rs a_resumed_late_day_at_a_revision_served_month_lands_as_its_next_successor 309-465
mutants crates/pull/src/gdfl_seconds_attack_tests.rs a_flipped_tick_store_byte_never_reaches_a_row_unnoticed 1033-1089
mutants crates/pull/src/gdfl_seconds_attack_tests.rs whole_member 1091-1100
mutants crates/pull/src/gdfl_seconds_attack_tests.rs line 62
mutants crates/pull/src/gdfl_seconds_attack_tests.rs line 1090
mutants crates/pull/src/gdfl_tickstore.rs walk 581-589
mutants crates/pull/src/gdfl_tickstore.rs walk_within 591-650
mutants crates/pull/src/gdfl_tickstore.rs a_dangling_day_file_link_is_refused_by_name_never_a_day_not_held 1652-1742
mutants crates/pull/src/gdfl_tickstore.rs refused 1748-1753
mutants crates/pull/src/gdfl_tickstore.rs a_column_framed_with_a_two_byte_content_size_rebuilds 1897-1918
mutants crates/pull/src/gdfl_tickstore.rs line 542
mutants crates/pull/src/gdfl_tickstore.rs line 543
mutants crates/pull/src/gdfl_tickstore.rs line 544
mutants crates/pull/src/gdfl_tickstore.rs line 590
mutants crates/pull/src/gdfl_tickstore.rs line 868
mutants crates/pull/src/gdfl_tickstore.rs line 869
mutants crates/pull/src/gdfl_tickstore.rs line 870
mutants crates/pull/src/gdfl_tickstore.rs line 871
mutants crates/pull/src/gdfl_tickstore.rs line 872
mutants crates/pull/src/gdfl_tickstore.rs line 1919
mutants crates/pull/src/gdfl_tickstore_cov_tests.rs whole
mutants crates/pull/src/gdfl_v3.rs line 641
mutants crates/pull/src/gdfl_v3.rs line 642
mutants crates/pull/src/gdfl_v3.rs line 643
mutants crates/pull/src/gdfl_v3.rs line 644
mutants crates/pull/src/gdfl_v3.rs line 645
mutants crates/pull/src/gdfl_v3.rs line 646
mutants crates/pull/src/gdfl_v3_ps2_tests.rs whole
mutants crates/pull/src/gdfl_wp12e_tests.rs a_ledger_without_event_records_still_rebuilds_the_retired_keys 353-412
mutants crates/pull/src/gdfl_wp12e_tests.rs tree_names 709-722
mutants crates/pull/src/http.rs new 509-520
mutants crates/pull/src/http.rs built_with 522-586
mutants crates/pull/src/http.rs spec_top 4469-4543
mutants crates/pull/src/http.rs a_refusal_under_a_200_on_the_post_and_discovery_paths_is_never_a_success 5943-6047
mutants crates/pull/src/http.rs a_zone_carrying_stamp_is_converted_once_and_the_two_arms_must_agree 6459-6578
mutants crates/pull/src/http.rs the_zerodha_request_is_the_url_and_header_the_vendor_documents 6633-6688
mutants crates/pull/src/http.rs listener 6741-6774
mutants crates/pull/src/http.rs counted_flood 6802-6847
mutants crates/pull/src/http.rs write 6856-6860
mutants crates/pull/src/http.rs flush 6862-6864
mutants crates/pull/src/http.rs a_vendor_that_hangs_up_without_answering_is_a_named_transport_failure 6867-6892
mutants crates/pull/src/http.rs a_rolling_answer_that_is_not_utf8_is_refused_not_repaired 7224-7268
mutants crates/pull/src/http.rs answer_once 7273-7292
mutants crates/pull/src/http.rs a_rolling_answer_this_build_refuses_earns_the_governor_nothing 7341-7418
mutants crates/pull/src/http.rs a_window_is_fetched_decoded_and_returned_over_a_real_socket 7856-7904
mutants crates/pull/src/http.rs dhan_serves_the_two_rungs_from_two_endpoints_and_only_one_takes_an_interval 8584-8675
mutants crates/pull/src/http.rs line 521
mutants crates/pull/src/http.rs line 6849
mutants crates/pull/src/http.rs line 6850
mutants crates/pull/src/http.rs line 6851
mutants crates/pull/src/http.rs line 6852
mutants crates/pull/src/http.rs line 6853
mutants crates/pull/src/http.rs line 6854
mutants crates/pull/src/http.rs line 6855
mutants crates/pull/src/http.rs line 6861
mutants crates/pull/src/http.rs line 6865
mutants crates/pull/src/http.rs line 6866
mutants crates/pull/src/http.rs line 6893
mutants crates/pull/src/http.rs line 9269
mutants crates/pull/src/http.rs line 9270
mutants crates/pull/src/http.rs line 9271
mutants crates/pull/src/http.rs line 9272
mutants crates/pull/src/http.rs line 9273
mutants crates/pull/src/http.rs line 9274
mutants crates/pull/src/http.rs line 9275
mutants crates/pull/src/http_cov_tests.rs whole
mutants crates/pull/src/identity_screen.rs line 812
mutants crates/pull/src/identity_screen.rs line 813
mutants crates/pull/src/identity_screen.rs line 814
mutants crates/pull/src/identity_screen.rs line 815
mutants crates/pull/src/identity_screen.rs line 816
mutants crates/pull/src/identity_screen_ps1_tests.rs whole
mutants crates/pull/src/ingest.rs rows_mode 1900-2040
mutants crates/pull/src/ingest.rs months_in 3599-3668
mutants crates/pull/src/ingest.rs take_at 5611-5701
mutants crates/pull/src/ingest.rs opened 5703-5728
mutants crates/pull/src/ingest.rs zerodha_minutes_are_checked_against_the_pulled_day_and_no_other_vendor_is 6247-6358
mutants crates/pull/src/ingest.rs a_fifo_at_the_lock_path_is_refused_rather_than_waited_on 6674-6722
mutants crates/pull/src/ingest.rs socket_at 6724-6743
mutants crates/pull/src/ingest.rs socket_through 6745-6775
mutants crates/pull/src/ingest.rs socket_at_binds_where_a_direct_bind_cannot 6777-6829
mutants crates/pull/src/ingest.rs a_census_path_that_is_not_a_regular_file_is_refused_unread_and_never_waits 7071-7119
mutants crates/pull/src/ingest.rs line 5729
mutants crates/pull/src/ingest.rs line 6744
mutants crates/pull/src/ingest.rs line 7258
mutants crates/pull/src/ingest.rs line 7259
mutants crates/pull/src/ingest.rs line 7260
mutants crates/pull/src/ingest.rs line 7261
mutants crates/pull/src/ingest.rs line 7262
mutants crates/pull/src/ingest.rs line 7263
mutants crates/pull/src/ingest.rs line 7264
mutants crates/pull/src/ingest.rs line 7265
mutants crates/pull/src/ingest_cov_tests.rs whole
mutants crates/pull/src/ingest_failkind_tests.rs store_variants 340-352
mutants crates/pull/src/ingest_failkind_tests.rs declares_a_variant 354-365
mutants crates/pull/src/ingest_failkind_tests.rs the_variant_counter_reads_declaration_lines_only 367-379
mutants crates/pull/src/ingest_failkind_tests.rs line 366
mutants crates/pull/src/ingest_failkind_tests.rs line 380
mutants crates/pull/src/ingest_gap_tests.rs whole
mutants crates/pull/src/kite.rs line 251
mutants crates/pull/src/kite.rs line 252
mutants crates/pull/src/kite.rs line 253
mutants crates/pull/src/kite.rs line 254
mutants crates/pull/src/kite.rs line 255
mutants crates/pull/src/kite_cov_tests.rs whole
mutants crates/pull/src/landing_run.rs claim 360-401
mutants crates/pull/src/landing_run.rs fail 763-766
mutants crates/pull/src/landing_run.rs answer 768-780
mutants crates/pull/src/landing_run.rs line 749
mutants crates/pull/src/landing_run.rs line 750
mutants crates/pull/src/landing_run.rs line 751
mutants crates/pull/src/landing_run.rs line 752
mutants crates/pull/src/landing_run.rs line 753
mutants crates/pull/src/landing_run.rs line 754
mutants crates/pull/src/landing_run.rs line 755
mutants crates/pull/src/landing_run.rs line 756
mutants crates/pull/src/landing_run.rs line 757
mutants crates/pull/src/landing_run.rs line 758
mutants crates/pull/src/landing_run.rs line 759
mutants crates/pull/src/landing_run.rs line 760
mutants crates/pull/src/landing_run.rs line 761
mutants crates/pull/src/landing_run.rs line 762
mutants crates/pull/src/landing_run.rs line 767
mutants crates/pull/src/landing_run.rs line 781
mutants crates/pull/src/landing_run.rs line 782
mutants crates/pull/src/landing_run_tests.rs behind_alerts 582-606
mutants crates/pull/src/landing_run_tests.rs a_window_over_differing_held_minutes_is_refused_by_name_and_its_answer_is_kept 1367-1457
mutants crates/pull/src/landing_run_tests.rs read_only_claim 2378-2397
mutants crates/pull/src/landing_run_tests.rs live_notes 2399-2407
mutants crates/pull/src/landing_run_tests.rs a_body_the_pack_could_not_take_is_named_and_not_spooled 2409-2436
mutants crates/pull/src/landing_run_tests.rs a_gap_ledger_that_is_no_file_stops_the_spool_by_name 2438-2462
mutants crates/pull/src/landing_run_tests.rs a_series_whose_daily_folder_is_no_folder_is_verified_with_no_first_day 2464-2505
mutants crates/pull/src/landing_run_tests.rs a_claim_whose_note_the_host_refuses_still_holds_and_remembers_it 2507-2543
mutants crates/pull/src/landing_run_tests.rs a_claim_of_an_unmeasured_lock_file_notes_and_remembers_nothing 2545-2581
mutants crates/pull/src/landing_run_tests.rs every_word_the_hook_and_the_claim_say_reaches_a_file 2583-2595
mutants crates/pull/src/landing_run_tests.rs every_word_body 2597-2669
mutants crates/pull/src/landing_run_tests.rs the_regular_days_leave_out_a_recorded_short_session 2671-2679
mutants crates/pull/src/landing_run_tests.rs a_claim_the_host_cannot_measure_is_taken_but_not_remembered 2681-2692
mutants crates/pull/src/landing_run_tests.rs unmeasured_claim_body 2694-2739
mutants crates/pull/src/landing_run_tests.rs line 2371
mutants crates/pull/src/landing_run_tests.rs line 2372
mutants crates/pull/src/landing_run_tests.rs line 2373
mutants crates/pull/src/landing_run_tests.rs line 2374
mutants crates/pull/src/landing_run_tests.rs line 2375
mutants crates/pull/src/landing_run_tests.rs line 2376
mutants crates/pull/src/landing_run_tests.rs line 2377
mutants crates/pull/src/landing_run_tests.rs line 2398
mutants crates/pull/src/landing_run_tests.rs line 2408
mutants crates/pull/src/landing_run_tests.rs line 2437
mutants crates/pull/src/landing_run_tests.rs line 2463
mutants crates/pull/src/landing_run_tests.rs line 2506
mutants crates/pull/src/landing_run_tests.rs line 2544
mutants crates/pull/src/landing_run_tests.rs line 2582
mutants crates/pull/src/landing_run_tests.rs line 2596
mutants crates/pull/src/landing_run_tests.rs line 2670
mutants crates/pull/src/landing_run_tests.rs line 2680
mutants crates/pull/src/landing_run_tests.rs line 2693
mutants crates/pull/src/lib.rs line 323
mutants crates/pull/src/lib.rs line 324
mutants crates/pull/src/lib.rs line 325
mutants crates/pull/src/lib.rs line 326
mutants crates/pull/src/lib.rs line 327
mutants crates/pull/src/lib.rs line 328
mutants crates/pull/src/lib.rs line 329
mutants crates/pull/src/manifest.rs line 4579
mutants crates/pull/src/manifest.rs line 4580
mutants crates/pull/src/manifest.rs line 4581
mutants crates/pull/src/manifest.rs line 4582
mutants crates/pull/src/manifest.rs line 4583
mutants crates/pull/src/manifest.rs line 4584
mutants crates/pull/src/manifest.rs line 4585
mutants crates/pull/src/manifest.rs line 4586
mutants crates/pull/src/manifest.rs line 4587
mutants crates/pull/src/manifest.rs line 4588
mutants crates/pull/src/manifest.rs line 4589
mutants crates/pull/src/manifest.rs line 4590
mutants crates/pull/src/manifest.rs line 4591
mutants crates/pull/src/manifest_cov_tests.rs whole
mutants crates/pull/src/manifest_ps1_tests.rs whole
mutants crates/pull/src/master_snapshots.rs read 406-528
mutants crates/pull/src/master_snapshots.rs commit_receipt 781-788
mutants crates/pull/src/master_snapshots.rs commit_receipt_renaming 790-815
mutants crates/pull/src/master_snapshots.rs of 1669-1701
mutants crates/pull/src/master_snapshots.rs line 789
mutants crates/pull/src/master_snapshots.rs line 2964
mutants crates/pull/src/master_snapshots.rs line 2965
mutants crates/pull/src/master_snapshots.rs line 2966
mutants crates/pull/src/master_snapshots.rs line 2970
mutants crates/pull/src/master_snapshots.rs line 2971
mutants crates/pull/src/master_snapshots.rs line 2972
mutants crates/pull/src/master_snapshots.rs line 2973
mutants crates/pull/src/master_snapshots.rs line 2974
mutants crates/pull/src/master_snapshots.rs line 2975
mutants crates/pull/src/master_snapshots_cov_tests.rs whole
mutants crates/pull/src/master_snapshots_first_listed.rs line 985
mutants crates/pull/src/master_snapshots_first_listed.rs line 986
mutants crates/pull/src/master_snapshots_first_listed.rs line 987
mutants crates/pull/src/master_snapshots_first_listed.rs line 988
mutants crates/pull/src/master_snapshots_first_listed.rs line 989
mutants crates/pull/src/master_snapshots_first_listed.rs line 990
mutants crates/pull/src/master_snapshots_first_listed_ps1_tests.rs whole
mutants crates/pull/src/master_snapshots_ps1_tests.rs whole
mutants crates/pull/src/master_snapshots_tests.rs scratch 33-43
mutants crates/pull/src/master_snapshots_tests.rs day 45-47
mutants crates/pull/src/master_snapshots_tests.rs stamp 49-51
mutants crates/pull/src/master_snapshots_tests.rs row 53-70
mutants crates/pull/src/master_snapshots_tests.rs fut 72-76
mutants crates/pull/src/master_snapshots_tests.rs share 78-80
mutants crates/pull/src/master_snapshots_tests.rs dump 82-90
mutants crates/pull/src/master_snapshots_tests.rs nse_file 92-99
mutants crates/pull/src/master_snapshots_tests.rs line 23
mutants crates/pull/src/masters.rs planted 974-985
mutants crates/pull/src/masters.rs plant 997-1003
mutants crates/pull/src/masters.rs take 1005-1013
mutants crates/pull/src/masters.rs remove_partial_after 1016-1026
mutants crates/pull/src/masters.rs replace_locked 1028-1089
mutants crates/pull/src/masters.rs holds_exactly 1466-1509
mutants crates/pull/src/masters.rs fetch 1773-1865
mutants crates/pull/src/masters.rs prime_once 1867-1876
mutants crates/pull/src/masters.rs prime_now 1878-1894
mutants crates/pull/src/masters.rs a_source_without_a_vendor_lands_the_catalogues_own_shape 1999-2021
mutants crates/pull/src/masters.rs a_blank_line_between_rows_is_skipped_not_refused 2023-2038
mutants crates/pull/src/masters.rs an_unchanged_master_keeps_its_inode_and_only_its_mtime_moves 2414-2483
mutants crates/pull/src/masters.rs get 2920-2930
mutants crates/pull/src/masters.rs a_master_target_that_is_not_a_regular_file_is_refused_and_never_waits 3704-3742
mutants crates/pull/src/masters.rs line 959
mutants crates/pull/src/masters.rs line 960
mutants crates/pull/src/masters.rs line 961
mutants crates/pull/src/masters.rs line 962
mutants crates/pull/src/masters.rs line 963
mutants crates/pull/src/masters.rs line 964
mutants crates/pull/src/masters.rs line 965
mutants crates/pull/src/masters.rs line 966
mutants crates/pull/src/masters.rs line 967
mutants crates/pull/src/masters.rs line 968
mutants crates/pull/src/masters.rs line 969
mutants crates/pull/src/masters.rs line 970
mutants crates/pull/src/masters.rs line 971
mutants crates/pull/src/masters.rs line 972
mutants crates/pull/src/masters.rs line 973
mutants crates/pull/src/masters.rs line 986
mutants crates/pull/src/masters.rs line 987
mutants crates/pull/src/masters.rs line 988
mutants crates/pull/src/masters.rs line 989
mutants crates/pull/src/masters.rs line 990
mutants crates/pull/src/masters.rs line 991
mutants crates/pull/src/masters.rs line 992
mutants crates/pull/src/masters.rs line 993
mutants crates/pull/src/masters.rs line 994
mutants crates/pull/src/masters.rs line 995
mutants crates/pull/src/masters.rs line 996
mutants crates/pull/src/masters.rs line 1004
mutants crates/pull/src/masters.rs line 1014
mutants crates/pull/src/masters.rs line 1015
mutants crates/pull/src/masters.rs line 2022
mutants crates/pull/src/masters.rs line 2039
mutants crates/pull/src/masters.rs line 3744
mutants crates/pull/src/masters.rs line 3745
mutants crates/pull/src/masters.rs line 3746
mutants crates/pull/src/masters.rs line 3747
mutants crates/pull/src/masters.rs line 3748
mutants crates/pull/src/masters.rs line 3749
mutants crates/pull/src/masters.rs line 3750
mutants crates/pull/src/masters.rs line 3751
mutants crates/pull/src/masters.rs line 3752
mutants crates/pull/src/masters.rs line 3753
mutants crates/pull/src/masters.rs line 3754
mutants crates/pull/src/masters.rs line 3755
mutants crates/pull/src/masters_cov_tests.rs whole
mutants crates/pull/src/masters_land_tests.rs whole
mutants crates/pull/src/nse.rs line 1325
mutants crates/pull/src/nse.rs line 1326
mutants crates/pull/src/nse.rs line 1327
mutants crates/pull/src/nse.rs line 1328
mutants crates/pull/src/nse.rs line 1329
mutants crates/pull/src/nse.rs line 1330
mutants crates/pull/src/nse_ps1_tests.rs whole
mutants crates/pull/src/padded_tests.rs injected_month 472-509
mutants crates/pull/src/padded_tests.rs a_day_that_is_not_an_open_session_has_no_windows 604-624
mutants crates/pull/src/padded_tests.rs line 603
mutants crates/pull/src/pricing.rs line 1934
mutants crates/pull/src/pricing.rs line 1935
mutants crates/pull/src/pricing.rs line 1936
mutants crates/pull/src/pricing.rs line 1937
mutants crates/pull/src/pricing.rs line 1938
mutants crates/pull/src/pricing.rs line 1939
mutants crates/pull/src/pricing_ps1_tests.rs whole
mutants crates/pull/src/reference_data.rs newest 2431-2436
mutants crates/pull/src/reference_data.rs newest_within 2438-2483
mutants crates/pull/src/reference_data.rs write 2839-2848
mutants crates/pull/src/reference_data.rs write_linking 2850-2953
mutants crates/pull/src/reference_data.rs line 2437
mutants crates/pull/src/reference_data.rs line 2849
mutants crates/pull/src/reference_data.rs line 3236
mutants crates/pull/src/reference_data.rs line 3237
mutants crates/pull/src/reference_data.rs line 3238
mutants crates/pull/src/reference_data.rs line 3242
mutants crates/pull/src/reference_data.rs line 3243
mutants crates/pull/src/reference_data.rs line 3244
mutants crates/pull/src/reference_data.rs line 3245
mutants crates/pull/src/reference_data.rs line 3246
mutants crates/pull/src/reference_data.rs line 3247
mutants crates/pull/src/reference_data.rs line 3248
mutants crates/pull/src/reference_data_cov_tests.rs whole
mutants crates/pull/src/reference_data_ps1_tests.rs whole
mutants crates/pull/src/reference_data_tests.rs a_calendar_change_that_closes_a_held_day_is_named_before_it_is_applied 784-836
mutants crates/pull/src/reference_data_tests.rs two_writers_of_one_generation_never_both_land 838-891
mutants crates/pull/src/reference_data_tests.rs a_reused_symbol_is_one_action_item_and_not_a_nightly_incomplete 958-1033
mutants crates/pull/src/reference_data_tests.rs a_seeded_row_keeps_every_citations_checksum 1187-1219
mutants crates/pull/src/reference_data_tests.rs a_loaded_row_cites_its_own_source_and_checksum 1221-1315
mutants crates/pull/src/refusal.rs line 652
mutants crates/pull/src/refusal.rs line 653
mutants crates/pull/src/refusal.rs line 654
mutants crates/pull/src/refusal.rs line 655
mutants crates/pull/src/refusal.rs line 656
mutants crates/pull/src/refusal_ps1_tests.rs whole
mutants crates/pull/src/rehearsal.rs line 1086
mutants crates/pull/src/rehearsal.rs line 1087
mutants crates/pull/src/rehearsal.rs line 1088
mutants crates/pull/src/rehearsal.rs line 1089
mutants crates/pull/src/rehearsal.rs line 1090
mutants crates/pull/src/rehearsal_ps1_tests.rs whole
mutants crates/pull/src/rehearsal_tests.rs a_stamp_is_one_name_and_one_integer 769-780
mutants crates/pull/src/rehearsal_tests.rs each_refusal_names_its_kind_and_what_it_waits_for 782-830
mutants crates/pull/src/rehearsal_tests.rs a_record_field_escapes_every_byte_that_would_make_it_two 832-838
mutants crates/pull/src/rehearsal_tests.rs line 766
mutants crates/pull/src/rehearsal_tests.rs line 767
mutants crates/pull/src/rehearsal_tests.rs line 768
mutants crates/pull/src/rehearsal_tests.rs line 781
mutants crates/pull/src/rehearsal_tests.rs line 831
mutants crates/pull/src/repair_plan.rs plan_month_for 535-642
mutants crates/pull/src/repair_plan.rs plant 1039-1042
mutants crates/pull/src/repair_plan.rs at 1044-1053
mutants crates/pull/src/repair_plan.rs line 1016
mutants crates/pull/src/repair_plan.rs line 1017
mutants crates/pull/src/repair_plan.rs line 1018
mutants crates/pull/src/repair_plan.rs line 1019
mutants crates/pull/src/repair_plan.rs line 1020
mutants crates/pull/src/repair_plan.rs line 1021
mutants crates/pull/src/repair_plan.rs line 1022
mutants crates/pull/src/repair_plan.rs line 1023
mutants crates/pull/src/repair_plan.rs line 1024
mutants crates/pull/src/repair_plan.rs line 1025
mutants crates/pull/src/repair_plan.rs line 1026
mutants crates/pull/src/repair_plan.rs line 1027
mutants crates/pull/src/repair_plan.rs line 1028
mutants crates/pull/src/repair_plan.rs line 1029
mutants crates/pull/src/repair_plan.rs line 1030
mutants crates/pull/src/repair_plan.rs line 1031
mutants crates/pull/src/repair_plan.rs line 1032
mutants crates/pull/src/repair_plan.rs line 1033
mutants crates/pull/src/repair_plan.rs line 1034
mutants crates/pull/src/repair_plan.rs line 1035
mutants crates/pull/src/repair_plan.rs line 1036
mutants crates/pull/src/repair_plan.rs line 1037
mutants crates/pull/src/repair_plan.rs line 1038
mutants crates/pull/src/repair_plan.rs line 1043
mutants crates/pull/src/repair_plan.rs line 1054
mutants crates/pull/src/repair_plan_ps2_tests.rs whole
mutants crates/pull/src/request_minutes.rs an_unsized_session_is_withheld_whole_through_the_audit 1897-1937
mutants crates/pull/src/request_minutes.rs line 2465
mutants crates/pull/src/request_minutes.rs line 2466
mutants crates/pull/src/request_minutes.rs line 2467
mutants crates/pull/src/request_minutes.rs line 2468
mutants crates/pull/src/request_minutes.rs line 2469
mutants crates/pull/src/request_minutes.rs line 2470
mutants crates/pull/src/request_minutes_ps1_tests.rs whole
mutants crates/pull/src/resolve.rs listener 1112-1128
mutants crates/pull/src/resolve.rs counting_flood 1200-1247
mutants crates/pull/src/resolve.rs a_listing_past_the_link_bound_fails_its_category_by_name 1317-1362
mutants crates/pull/src/resolve.rs an_unreached_constituent_file_fails_each_index_by_its_name 1364-1389
mutants crates/pull/src/resolve.rs line 1363
mutants crates/pull/src/resolve.rs line 1390
mutants crates/pull/src/response_keep.rs open_regular 1121-1175
mutants crates/pull/src/response_keep.rs lock_within 1463-1538
mutants crates/pull/src/response_keep.rs abandon 1614-1633
mutants crates/pull/src/response_keep.rs plant 2413-2416
mutants crates/pull/src/response_keep.rs at 2418-2427
mutants crates/pull/src/response_keep.rs plant_for 2429-2435
mutants crates/pull/src/response_keep.rs at_path 2437-2448
mutants crates/pull/src/response_keep.rs fail 2450-2453
mutants crates/pull/src/response_keep.rs taken 2455-2463
mutants crates/pull/src/response_keep.rs answer_lock 2465-2472
mutants crates/pull/src/response_keep.rs line 2383
mutants crates/pull/src/response_keep.rs line 2384
mutants crates/pull/src/response_keep.rs line 2385
mutants crates/pull/src/response_keep.rs line 2386
mutants crates/pull/src/response_keep.rs line 2387
mutants crates/pull/src/response_keep.rs line 2388
mutants crates/pull/src/response_keep.rs line 2389
mutants crates/pull/src/response_keep.rs line 2390
mutants crates/pull/src/response_keep.rs line 2391
mutants crates/pull/src/response_keep.rs line 2392
mutants crates/pull/src/response_keep.rs line 2393
mutants crates/pull/src/response_keep.rs line 2394
mutants crates/pull/src/response_keep.rs line 2395
mutants crates/pull/src/response_keep.rs line 2396
mutants crates/pull/src/response_keep.rs line 2397
mutants crates/pull/src/response_keep.rs line 2398
mutants crates/pull/src/response_keep.rs line 2399
mutants crates/pull/src/response_keep.rs line 2400
mutants crates/pull/src/response_keep.rs line 2401
mutants crates/pull/src/response_keep.rs line 2402
mutants crates/pull/src/response_keep.rs line 2403
mutants crates/pull/src/response_keep.rs line 2404
mutants crates/pull/src/response_keep.rs line 2405
mutants crates/pull/src/response_keep.rs line 2406
mutants crates/pull/src/response_keep.rs line 2407
mutants crates/pull/src/response_keep.rs line 2408
mutants crates/pull/src/response_keep.rs line 2409
mutants crates/pull/src/response_keep.rs line 2410
mutants crates/pull/src/response_keep.rs line 2411
mutants crates/pull/src/response_keep.rs line 2412
mutants crates/pull/src/response_keep.rs line 2417
mutants crates/pull/src/response_keep.rs line 2428
mutants crates/pull/src/response_keep.rs line 2436
mutants crates/pull/src/response_keep.rs line 2449
mutants crates/pull/src/response_keep.rs line 2454
mutants crates/pull/src/response_keep.rs line 2464
mutants crates/pull/src/response_keep.rs line 2473
mutants crates/pull/src/response_keep.rs line 2474
mutants crates/pull/src/response_keep_tests.rs only_an_abandoned_waiter_is_counted_as_parked 1919-1955
mutants crates/pull/src/response_keep_tests.rs kept_in 2488-2500
mutants crates/pull/src/response_keep_tests.rs full_receipts 2502-2510
mutants crates/pull/src/response_keep_tests.rs a_code_byte_no_writer_writes_is_refused_by_each_decoder 2512-2545
mutants crates/pull/src/response_keep_tests.rs a_key_no_store_path_can_hold_is_refused_by_every_door 2547-2616
mutants crates/pull/src/response_keep_tests.rs a_pack_name_the_host_refuses_keeps_the_hosts_kind 2618-2668
mutants crates/pull/src/response_keep_tests.rs a_pack_file_the_host_will_not_create_is_refused_by_name 2670-2704
mutants crates/pull/src/response_keep_tests.rs a_receipts_file_past_its_cap_is_damage_named_at_the_cap 2706-2750
mutants crates/pull/src/response_keep_tests.rs a_kept_body_gone_altered_or_not_text_is_refused_by_what_differs 2752-2797
mutants crates/pull/src/response_keep_tests.rs refused_rows_whose_file_is_gone_are_damage 2799-2832
mutants crates/pull/src/response_keep_tests.rs a_directory_at_the_lock_name_refuses_the_append 2834-2862
mutants crates/pull/src/response_keep_tests.rs the_empty_path_has_no_parent_to_sync_and_nothing_to_create 2864-2876
mutants crates/pull/src/response_keep_tests.rs a_window_journal_at_its_edges_is_read_or_refused_by_name 2878-2934
mutants crates/pull/src/response_keep_tests.rs an_open_window_naming_another_receipt_is_damage 2936-3010
mutants crates/pull/src/response_keep_tests.rs a_finished_window_rolls_its_pointer_past_a_full_later_month 3012-3053
mutants crates/pull/src/response_keep_tests.rs a_month_whose_last_segment_is_full_is_refused_receipts_full 3055-3091
mutants crates/pull/src/response_keep_tests.rs a_body_the_host_refuses_for_another_reason_refuses_the_append 3093-3143
mutants crates/pull/src/response_keep_tests.rs a_window_whose_open_record_is_refused_writes_nothing 3145-3185
mutants crates/pull/src/response_keep_tests.rs a_name_created_between_the_look_and_the_create_is_opened_not_created_over 3187-3206
mutants crates/pull/src/response_keep_tests.rs a_body_no_empty_segment_could_hold_is_not_rolled_for 3208-3231
mutants crates/pull/src/response_keep_tests.rs a_roll_of_a_pack_no_path_can_hold_is_said_by_its_key 3233-3244
mutants crates/pull/src/response_keep_tests.rs roll_said_by_its_key 3246-3286
mutants crates/pull/src/response_keep_tests.rs a_pack_lock_the_host_refuses_is_refused_at_once_by_name 3288-3311
mutants crates/pull/src/response_keep_tests.rs held_pack_lock 3313-3320
mutants crates/pull/src/response_keep_tests.rs answer_after_the_wait 3322-3335
mutants crates/pull/src/response_keep_tests.rs a_waiter_that_answers_just_after_the_wait_ran_out_hands_over_the_lock 3337-3354
mutants crates/pull/src/response_keep_tests.rs a_waiter_whose_flock_the_host_refuses_refuses_its_caller_by_name 3356-3378
mutants crates/pull/src/response_keep_tests.rs a_step_planted_for_a_path_waits_for_its_own_name_and_path 3380-3397
mutants crates/pull/src/response_keep_tests.rs line 2481
mutants crates/pull/src/response_keep_tests.rs line 2482
mutants crates/pull/src/response_keep_tests.rs line 2483
mutants crates/pull/src/response_keep_tests.rs line 2484
mutants crates/pull/src/response_keep_tests.rs line 2485
mutants crates/pull/src/response_keep_tests.rs line 2486
mutants crates/pull/src/response_keep_tests.rs line 2487
mutants crates/pull/src/response_keep_tests.rs line 2501
mutants crates/pull/src/response_keep_tests.rs line 2511
mutants crates/pull/src/response_keep_tests.rs line 2546
mutants crates/pull/src/response_keep_tests.rs line 2617
mutants crates/pull/src/response_keep_tests.rs line 2669
mutants crates/pull/src/response_keep_tests.rs line 2705
mutants crates/pull/src/response_keep_tests.rs line 2751
mutants crates/pull/src/response_keep_tests.rs line 2798
mutants crates/pull/src/response_keep_tests.rs line 2833
mutants crates/pull/src/response_keep_tests.rs line 2863
mutants crates/pull/src/response_keep_tests.rs line 2877
mutants crates/pull/src/response_keep_tests.rs line 2935
mutants crates/pull/src/response_keep_tests.rs line 3011
mutants crates/pull/src/response_keep_tests.rs line 3054
mutants crates/pull/src/response_keep_tests.rs line 3092
mutants crates/pull/src/response_keep_tests.rs line 3144
mutants crates/pull/src/response_keep_tests.rs line 3186
mutants crates/pull/src/response_keep_tests.rs line 3207
mutants crates/pull/src/response_keep_tests.rs line 3232
mutants crates/pull/src/response_keep_tests.rs line 3245
mutants crates/pull/src/response_keep_tests.rs line 3287
mutants crates/pull/src/response_keep_tests.rs line 3312
mutants crates/pull/src/response_keep_tests.rs line 3321
mutants crates/pull/src/response_keep_tests.rs line 3336
mutants crates/pull/src/response_keep_tests.rs line 3355
mutants crates/pull/src/response_keep_tests.rs line 3379
mutants crates/pull/src/riskfree.rs every_day_agrees_with_a_scan_and_none_sees_its_own_or_a_later_release 1074-1117
mutants crates/pull/src/riskfree.rs a_result_row_of_any_other_width_is_refused_by_its_count 2070-2085
mutants crates/pull/src/riskfree.rs line 2069
mutants crates/pull/src/rolling.rs expiry_of 56-192
mutants crates/pull/src/rolling.rs listing_of 211-243
mutants crates/pull/src/rolling.rs step_refused 251-267
mutants crates/pull/src/rolling.rs a_day_before_the_verified_regimes_is_refused_for_either_cadence 1922-1969
mutants crates/pull/src/rolling.rs an_unreal_underlying_or_trade_day_is_refused_by_its_own_step 1971-2028
mutants crates/pull/src/rolling.rs an_expiry_past_the_last_measured_day_is_filed_unchecked 2030-2081
mutants crates/pull/src/rolling.rs a_value_segment_of_a_rolling_path_is_written_as_its_placeholder 2083-2112
mutants crates/pull/src/rolling.rs with_strike 2114-2125
mutants crates/pull/src/rolling.rs a_strike_is_read_beside_every_row_on_either_scale 2127-2198
mutants crates/pull/src/rolling.rs every_refusal_sentence_names_what_it_refused 2200-2230
mutants crates/pull/src/rolling.rs line 244
mutants crates/pull/src/rolling.rs line 245
mutants crates/pull/src/rolling.rs line 246
mutants crates/pull/src/rolling.rs line 247
mutants crates/pull/src/rolling.rs line 248
mutants crates/pull/src/rolling.rs line 249
mutants crates/pull/src/rolling.rs line 250
mutants crates/pull/src/rolling.rs line 1921
mutants crates/pull/src/rolling.rs line 1970
mutants crates/pull/src/rolling.rs line 2029
mutants crates/pull/src/rolling.rs line 2082
mutants crates/pull/src/rolling.rs line 2113
mutants crates/pull/src/rolling.rs line 2126
mutants crates/pull/src/rolling.rs line 2199
mutants crates/pull/src/session.rs whole_months 1535-1578
mutants crates/pull/src/session.rs a_venue_with_no_verified_hours_is_named_with_its_day 1887-1916
mutants crates/pull/src/session.rs the_zerodha_minute_window_never_splits_a_month_between_two_requests 2051-2089
mutants crates/pull/src/session.rs the_walk_of_whole_months_stops_at_the_cap_and_nowhere_else 2091-2140
mutants crates/pull/src/session.rs line 1886
mutants crates/pull/src/session.rs line 1928
mutants crates/pull/src/session.rs line 2141
mutants crates/pull/src/ssm.rs get_parameter 882-972
mutants crates/pull/src/ssm.rs answer_of 974-1061
mutants crates/pull/src/ssm.rs now_stamp 1107-1120
mutants crates/pull/src/ssm.rs stamp_at 1122-1147
mutants crates/pull/src/ssm.rs the_only_action_on_the_wire_is_get_parameter 1815-1846
mutants crates/pull/src/ssm.rs a_fifo_credentials_file_is_refused_without_opening_it 1916-1962
mutants crates/pull/src/ssm.rs line 1121
mutants crates/pull/src/ssm.rs line 2085
mutants crates/pull/src/ssm.rs line 2086
mutants crates/pull/src/ssm.rs line 2087
mutants crates/pull/src/ssm.rs line 2088
mutants crates/pull/src/ssm.rs line 2089
mutants crates/pull/src/ssm.rs line 2090
mutants crates/pull/src/ssm.rs line 2091
mutants crates/pull/src/ssm.rs line 2092
mutants crates/pull/src/ssm.rs line 2093
mutants crates/pull/src/ssm.rs line 2094
mutants crates/pull/src/ssm_cov_tests.rs whole
mutants crates/pull/src/ssm_ps1_tests.rs whole
mutants crates/pull/src/tickstore_write.rs columnar 821-826
mutants crates/pull/src/tickstore_write.rs columnar_under 828-908
mutants crates/pull/src/tickstore_write.rs preflight 1072-1077
mutants crates/pull/src/tickstore_write.rs preflight_under 1079-1155
mutants crates/pull/src/tickstore_write.rs item 1188-1191
mutants crates/pull/src/tickstore_write.rs item_by 1193-1236
mutants crates/pull/src/tickstore_write.rs write 1336-1350
mutants crates/pull/src/tickstore_write.rs write_temp 1358-1425
mutants crates/pull/src/tickstore_write.rs write_day_inner 1721-1825
mutants crates/pull/src/tickstore_write.rs drop 1841-1846
mutants crates/pull/src/tickstore_write.rs lock_root 1849-1869
mutants crates/pull/src/tickstore_write.rs lock_held 1871-1888
mutants crates/pull/src/tickstore_write.rs encode_all 1990-1997
mutants crates/pull/src/tickstore_write.rs encode_all_under 1999-2024
mutants crates/pull/src/tickstore_write.rs line 668
mutants crates/pull/src/tickstore_write.rs line 669
mutants crates/pull/src/tickstore_write.rs line 670
mutants crates/pull/src/tickstore_write.rs line 671
mutants crates/pull/src/tickstore_write.rs line 827
mutants crates/pull/src/tickstore_write.rs line 1078
mutants crates/pull/src/tickstore_write.rs line 1192
mutants crates/pull/src/tickstore_write.rs line 1326
mutants crates/pull/src/tickstore_write.rs line 1327
mutants crates/pull/src/tickstore_write.rs line 1332
mutants crates/pull/src/tickstore_write.rs line 1827
mutants crates/pull/src/tickstore_write.rs line 1828
mutants crates/pull/src/tickstore_write.rs line 1829
mutants crates/pull/src/tickstore_write.rs line 1830
mutants crates/pull/src/tickstore_write.rs line 1831
mutants crates/pull/src/tickstore_write.rs line 1832
mutants crates/pull/src/tickstore_write.rs line 1833
mutants crates/pull/src/tickstore_write.rs line 1834
mutants crates/pull/src/tickstore_write.rs line 1835
mutants crates/pull/src/tickstore_write.rs line 1836
mutants crates/pull/src/tickstore_write.rs line 1837
mutants crates/pull/src/tickstore_write.rs line 1838
mutants crates/pull/src/tickstore_write.rs line 1839
mutants crates/pull/src/tickstore_write.rs line 1840
mutants crates/pull/src/tickstore_write.rs line 1847
mutants crates/pull/src/tickstore_write.rs line 1848
mutants crates/pull/src/tickstore_write.rs line 1870
mutants crates/pull/src/tickstore_write.rs line 1998
mutants crates/pull/src/tickstore_write_tests.rs entries_of 1844-1849
mutants crates/pull/src/tickstore_write_tests.rs bts_of 1851-1868
mutants crates/pull/src/tickstore_write_tests.rs raw_block 1870-1875
mutants crates/pull/src/tickstore_write_tests.rs idx_day 1877-1880
mutants crates/pull/src/tickstore_write_tests.rs fmt 1887-1889
mutants crates/pull/src/tickstore_write_tests.rs a_refusal_the_door_does_not_word_is_its_code_and_a_run_names_its_id 1892-1920
mutants crates/pull/src/tickstore_write_tests.rs a_time_with_a_letter_first_and_sixteen_decimals_are_text 1922-1951
mutants crates/pull/src/tickstore_write_tests.rs a_member_read_back_at_another_length_than_its_record_is_refused 1953-1977
mutants crates/pull/src/tickstore_write_tests.rs failing_encoder 1979-1982
mutants crates/pull/src/tickstore_write_tests.rs lying_encoder 1990-1998
mutants crates/pull/src/tickstore_write_tests.rs mislabelling_encoder 2005-2011
mutants crates/pull/src/tickstore_write_tests.rs a_block_that_does_not_rebuild_to_its_member_is_never_kept 2013-2054
mutants crates/pull/src/tickstore_write_tests.rs a_write_refused_for_another_reason_than_a_full_disk_is_named_write 2056-2095
mutants crates/pull/src/tickstore_write_tests.rs every_helper_on_the_way_to_a_held_day_names_its_step 2097-2157
mutants crates/pull/src/tickstore_write_tests.rs the_publish_and_the_fault_hook_name_their_steps 2159-2205
mutants crates/pull/src/tickstore_write_tests.rs a_lock_the_host_will_not_take_is_named_with_its_kind 2207-2243
mutants crates/pull/src/tickstore_write_tests.rs the_entry_bound_and_the_block_cap_refuse_one_past_and_hold_at_it 2245-2285
mutants crates/pull/src/tickstore_write_tests.rs a_held_day_differing_in_count_name_or_crc_alone_is_named 2287-2327
mutants crates/pull/src/tickstore_write_tests.rs a_held_day_storing_a_folder_as_a_file_or_a_file_as_a_folder_is_named 2329-2373
mutants crates/pull/src/tickstore_write_tests.rs read_at 2380-2382
mutants crates/pull/src/tickstore_write_tests.rs verify_file_names_the_zip_when_the_zip_is_the_damaged_side 2385-2416
mutants crates/pull/src/tickstore_write_tests.rs a_file_where_a_folder_of_the_days_path_should_be_is_refused 2418-2441
mutants crates/pull/src/tickstore_write_tests.rs another_days_files_in_the_month_folder_do_not_stop_this_day 2443-2481
mutants crates/pull/src/tickstore_write_tests.rs a_folder_path_the_host_cannot_make_or_no_path_at_all_is_refused 2483-2511
mutants crates/pull/src/tickstore_write_tests.rs a_folder_the_host_will_not_list_or_a_root_it_will_not_write_is_refused 2513-2562
mutants crates/pull/src/tickstore_write_tests.rs frame_by_hand 2564-2575
mutants crates/pull/src/tickstore_write_tests.rs the_frame_walk_reads_every_content_size_width 2577-2631
mutants crates/pull/src/tickstore_write_tests.rs the_measurement_helpers_answer_by_their_rules 2633-2653
mutants crates/pull/src/tickstore_write_tests.rs a_header_past_what_the_fields_record_is_not_columnar 2655-2681
mutants crates/pull/src/tickstore_write_tests.rs the_day_lock_is_released_by_its_run_not_by_the_last_copy_of_its_file 2683-2709
mutants crates/pull/src/tickstore_write_tests.rs line 1841
mutants crates/pull/src/tickstore_write_tests.rs line 1842
mutants crates/pull/src/tickstore_write_tests.rs line 1843
mutants crates/pull/src/tickstore_write_tests.rs line 1850
mutants crates/pull/src/tickstore_write_tests.rs line 1869
mutants crates/pull/src/tickstore_write_tests.rs line 1876
mutants crates/pull/src/tickstore_write_tests.rs line 1881
mutants crates/pull/src/tickstore_write_tests.rs line 1882
mutants crates/pull/src/tickstore_write_tests.rs line 1883
mutants crates/pull/src/tickstore_write_tests.rs line 1884
mutants crates/pull/src/tickstore_write_tests.rs line 1885
mutants crates/pull/src/tickstore_write_tests.rs line 1886
mutants crates/pull/src/tickstore_write_tests.rs line 1890
mutants crates/pull/src/tickstore_write_tests.rs line 1891
mutants crates/pull/src/tickstore_write_tests.rs line 1921
mutants crates/pull/src/tickstore_write_tests.rs line 1952
mutants crates/pull/src/tickstore_write_tests.rs line 1978
mutants crates/pull/src/tickstore_write_tests.rs line 1983
mutants crates/pull/src/tickstore_write_tests.rs line 1984
mutants crates/pull/src/tickstore_write_tests.rs line 1985
mutants crates/pull/src/tickstore_write_tests.rs line 1986
mutants crates/pull/src/tickstore_write_tests.rs line 1987
mutants crates/pull/src/tickstore_write_tests.rs line 1988
mutants crates/pull/src/tickstore_write_tests.rs line 1989
mutants crates/pull/src/tickstore_write_tests.rs line 1999
mutants crates/pull/src/tickstore_write_tests.rs line 2000
mutants crates/pull/src/tickstore_write_tests.rs line 2001
mutants crates/pull/src/tickstore_write_tests.rs line 2002
mutants crates/pull/src/tickstore_write_tests.rs line 2003
mutants crates/pull/src/tickstore_write_tests.rs line 2004
mutants crates/pull/src/tickstore_write_tests.rs line 2012
mutants crates/pull/src/tickstore_write_tests.rs line 2055
mutants crates/pull/src/tickstore_write_tests.rs line 2096
mutants crates/pull/src/tickstore_write_tests.rs line 2158
mutants crates/pull/src/tickstore_write_tests.rs line 2206
mutants crates/pull/src/tickstore_write_tests.rs line 2244
mutants crates/pull/src/tickstore_write_tests.rs line 2286
mutants crates/pull/src/tickstore_write_tests.rs line 2328
mutants crates/pull/src/tickstore_write_tests.rs line 2374
mutants crates/pull/src/tickstore_write_tests.rs line 2375
mutants crates/pull/src/tickstore_write_tests.rs line 2376
mutants crates/pull/src/tickstore_write_tests.rs line 2377
mutants crates/pull/src/tickstore_write_tests.rs line 2378
mutants crates/pull/src/tickstore_write_tests.rs line 2379
mutants crates/pull/src/tickstore_write_tests.rs line 2383
mutants crates/pull/src/tickstore_write_tests.rs line 2384
mutants crates/pull/src/tickstore_write_tests.rs line 2417
mutants crates/pull/src/tickstore_write_tests.rs line 2442
mutants crates/pull/src/tickstore_write_tests.rs line 2482
mutants crates/pull/src/tickstore_write_tests.rs line 2512
mutants crates/pull/src/tickstore_write_tests.rs line 2563
mutants crates/pull/src/tickstore_write_tests.rs line 2576
mutants crates/pull/src/tickstore_write_tests.rs line 2632
mutants crates/pull/src/tickstore_write_tests.rs line 2654
mutants crates/pull/src/tickstore_write_tests.rs line 2682
mutants crates/pull/src/totp.rs no_path_outside_this_module_computes_a_code 743-821
mutants crates/pull/src/totp.rs every_refusal_reaches_the_log_by_its_word_and_never_its_secret 823-835
mutants crates/pull/src/totp.rs every_refusal_reaches_the_log_body 837-907
mutants crates/pull/src/totp.rs line 822
mutants crates/pull/src/totp.rs line 836
mutants crates/pull/src/vendor.rs a_rung_on_the_wire_needs_a_word_and_the_unrecorded_ones_stay_absent 7017-7202
mutants crates/pull/src/vendor.rs a_tenor_refused_for_unverified_hours_quotes_the_refusal 7892-7952
mutants crates/pull/src/vendor.rs the_table_shape_contract_refuses_a_table_whose_rows_do_not_ascend 7985-8041
mutants crates/pull/src/vendor.rs the_table_shape_contract_refuses_a_row_with_no_window_and_no_citation 8043-8116
mutants crates/pull/src/vendor.rs envelope_of_reads_the_envelope_of_every_shape 8407-8428
mutants crates/pull/src/vendor.rs the_vendor_wide_floor_is_never_later_than_a_rungs_own 9072-9109
mutants crates/pull/src/vendor.rs the_binding_floor_is_never_the_looser_of_the_two 9111-9182
mutants crates/pull/src/vendor.rs judge_binding 9184-9216
mutants crates/pull/src/vendor.rs line 7953
mutants crates/pull/src/vendor.rs line 8429
mutants crates/pull/src/vendor.rs line 9183
mutants crates/pull/src/vendor.rs line 10135
mutants crates/pull/src/vendor.rs line 10136
mutants crates/pull/src/vendor.rs line 10137
mutants crates/pull/src/vendor.rs line 10138
mutants crates/pull/src/vendor_cov_tests.rs whole
mutants crates/pull/src/verdict_words.rs every_number_and_word_is_named_once_in_order 178-221
mutants crates/pull/src/zerodha_spool.rs live_probe 162-182
mutants crates/pull/src/zerodha_spool.rs take_live 815-842
mutants crates/pull/src/zerodha_spool.rs fail 1621-1624
mutants crates/pull/src/zerodha_spool.rs answer_lock 1626-1642
mutants crates/pull/src/zerodha_spool.rs line 1604
mutants crates/pull/src/zerodha_spool.rs line 1605
mutants crates/pull/src/zerodha_spool.rs line 1606
mutants crates/pull/src/zerodha_spool.rs line 1607
mutants crates/pull/src/zerodha_spool.rs line 1608
mutants crates/pull/src/zerodha_spool.rs line 1609
mutants crates/pull/src/zerodha_spool.rs line 1610
mutants crates/pull/src/zerodha_spool.rs line 1611
mutants crates/pull/src/zerodha_spool.rs line 1612
mutants crates/pull/src/zerodha_spool.rs line 1613
mutants crates/pull/src/zerodha_spool.rs line 1614
mutants crates/pull/src/zerodha_spool.rs line 1615
mutants crates/pull/src/zerodha_spool.rs line 1616
mutants crates/pull/src/zerodha_spool.rs line 1617
mutants crates/pull/src/zerodha_spool.rs line 1618
mutants crates/pull/src/zerodha_spool.rs line 1619
mutants crates/pull/src/zerodha_spool.rs line 1620
mutants crates/pull/src/zerodha_spool.rs line 1625
mutants crates/pull/src/zerodha_spool.rs line 1643
mutants crates/pull/src/zerodha_spool.rs line 1644
mutants crates/pull/src/zerodha_spool.rs line 1645
mutants crates/pull/src/zerodha_spool.rs line 1646
mutants crates/pull/src/zerodha_spool.rs line 1647
mutants crates/pull/src/zerodha_spool_cov_tests.rs whole
mutants crates/pull/src/zerodha_verify.rs merge_lock 3713-3727
mutants crates/pull/src/zerodha_verify.rs lock_answer 3729-3745
mutants crates/pull/src/zerodha_verify.rs merge_under 3747-3869
mutants crates/pull/src/zerodha_verify.rs note_left 3871-3887
mutants crates/pull/src/zerodha_verify.rs line 3728
mutants crates/pull/src/zerodha_verify.rs line 3888
mutants crates/pull/src/zerodha_verify.rs line 4595
mutants crates/pull/src/zerodha_verify.rs line 4596
mutants crates/pull/src/zerodha_verify.rs line 4597
mutants crates/pull/src/zerodha_verify.rs line 4598
mutants crates/pull/src/zerodha_verify.rs line 4599
mutants crates/pull/src/zerodha_verify.rs line 4600
mutants crates/pull/src/zerodha_verify.rs line 4601
mutants crates/pull/src/zerodha_verify.rs line 4602
mutants crates/pull/src/zerodha_verify.rs line 4603
mutants crates/pull/src/zerodha_verify.rs line 4604
mutants crates/pull/src/zerodha_verify.rs line 4605
mutants crates/pull/src/zerodha_verify.rs line 4606
mutants crates/pull/src/zerodha_verify_cov_store_tests.rs whole
mutants crates/pull/src/zerodha_verify_cov_tests.rs whole
mutants crates/pull/src/zerodha_verify_tests.rs scratch 35-47
mutants crates/pull/src/zerodha_verify_tests.rs day 49-51
mutants crates/pull/src/zerodha_verify_tests.rs regular 53-56
mutants crates/pull/src/zerodha_verify_tests.rs index 58-65
mutants crates/pull/src/zerodha_verify_tests.rs share 67-74
mutants crates/pull/src/zerodha_verify_tests.rs utc_s 76-78
mutants crates/pull/src/zerodha_verify_tests.rs bar_at 80-90
mutants crates/pull/src/zerodha_verify_tests.rs row_of 92-102
mutants crates/pull/src/zerodha_verify_tests.rs full_day 104-110
mutants crates/pull/src/zerodha_verify_tests.rs whole_day 112-114
mutants crates/pull/src/zerodha_verify_tests.rs write_month 116-125
mutants crates/pull/src/zerodha_verify_tests.rs write_ledger 127-130
mutants crates/pull/src/zerodha_verify_tests.rs answer_of 132-148
mutants crates/pull/src/zerodha_verify_tests.rs minute_record 150-156
mutants crates/pull/src/zerodha_verify_tests.rs reason_word 158-160
mutants crates/pull/src/zerodha_verify_tests.rs the_record 162-187
mutants crates/pull/src/zerodha_verify_tests.rs spooled 189-198
mutants crates/pull/src/zerodha_verify_tests.rs year_generation 200-209
mutants crates/pull/src/zerodha_verify_tests.rs append_raw 2722-2726
mutants crates/pull/src/zerodha_verify_tests.rs flip 2728-2739
mutants crates/pull/src/zerodha_verify_tests.rs kite_body 2743-2767
mutants crates/pull/src/zerodha_verify_tests.rs keep_answer 2782-2821
mutants crates/pull/src/zerodha_verify_tests.rs an_impossible_candle_is_skipped_by_the_oracle_as_by_the_decoder 2935-3068
mutants crates/pull/src/zerodha_verify_tests.rs progress_at 3398-3401
mutants crates/pull/src/zerodha_verify_tests.rs a_write_beside_a_merge_that_fails_is_named 3678-3691
mutants crates/pull/src/zerodha_verify_tests.rs a_write_beside_a_merge_that_fails_is_named_body 3693-3736
mutants crates/pull/src/zerodha_verify_tests.rs a_changed_answer_shape_is_named_and_alerted_once 3738-3844
mutants crates/pull/src/zerodha_verify_tests.rs write_peer 3950-3978
mutants crates/pull/src/zerodha_verify_tests.rs looked_up 4232-4253
mutants crates/pull/src/zerodha_verify_tests.rs a_series_held_at_1day_only_and_a_day_month_outside_the_minutes_are_verified 4568-4739
mutants crates/pull/src/zerodha_verify_tests.rs a_refused_open_keeps_the_hosts_words 5094-5106
mutants crates/pull/src/zerodha_verify_tests.rs a_refused_open_keeps_the_hosts_words_body 5108-5177
mutants crates/pull/src/zerodha_verify_tests.rs line 3692
mutants crates/pull/src/zerodha_verify_tests.rs line 5107
mutants crates/pull/tests/alerts_ps2.rs whole
mutants crates/pull/tests/allocation.rs day 653-659
mutants crates/pull/tests/allocation.rs read 687-694
mutants crates/pull/tests/allocation.rs read_named 700-716
mutants crates/pull/tests/allocation.rs line 646
mutants crates/pull/tests/allocation.rs line 647
mutants crates/pull/tests/broker.rs broker 99-125
mutants crates/pull/tests/broker.rs groww_declares_the_expired_fno_lookup_and_the_others_declare_none 908-979
mutants crates/pull/tests/broker.rs the_store_walks_read_a_missing_folder_as_empty 1399-1411
mutants crates/pull/tests/broker.rs line 1398
mutants crates/pull/tests/cloud_mutants_import.rs socket_at 821-842
mutants crates/pull/tests/cloud_mutants_import.rs socket_at_in 844-872
mutants crates/pull/tests/cloud_mutants_import.rs a_base_that_takes_no_link_or_holds_no_socket_address_is_passed_over 874-902
mutants crates/pull/tests/cloud_mutants_import.rs a_source_that_names_no_scope_lists_under_the_swept_surface 1475-1514
mutants crates/pull/tests/cloud_mutants_import.rs line 843
mutants crates/pull/tests/cloud_mutants_import.rs line 903
mutants crates/pull/tests/derive.rs a_clean_run_writes_one_info_record_naming_what_it_did 291-313
mutants crates/pull/tests/derive.rs a_clean_run_writes_one_info_record_naming_what_it_did_body 315-344
mutants crates/pull/tests/derive.rs a_refused_month_is_that_months_alone_and_the_others_are_counted 1936-1992
mutants crates/pull/tests/derive.rs line 23
mutants crates/pull/tests/derive.rs line 24
mutants crates/pull/tests/derive.rs line 25
mutants crates/pull/tests/derive.rs line 314
mutants crates/pull/tests/gdfl_run_stop.rs whole
mutants crates/pull/tests/no_scheduler_delete.rs the_scanner_blanks_and_finds_at_the_edges_of_its_input 528-559
mutants crates/pull/tests/no_scheduler_delete.rs line 527
mutants crates/pull/tests/pipeline.rs a_member_whose_bars_cross_a_month_boundary_lands_in_both_months 1598-1678
mutants crates/pull/tests/refusal.rs only_the_feed_whose_error_page_was_read_declares_a_contract 519-596
mutants crates/pull/tests/refusal.rs zerodha_is_never_asked_for_an_expired_option_and_its_futures_go_through_a_token 677-738
mutants crates/pull/tests/sealed_month.rs tree 183-197
mutants crates/pull/tests/sealed_month.rs a_held_day_writes_nothing_and_a_restated_minute_is_refused_as_raw 453-519
mutants crates/pull/tests/store_evidence.rs a_dropped_replay_removes_its_scratch 187-202
mutants crates/pull/tests/store_evidence.rs line 203
mutants crates/pull/tests/token_identity.rs check 219-275
mutants crates/pull/tests/token_identity.rs the_module_list_is_closed_over_every_definition_of_a_key 339-384
mutants crates/pull/tests/token_identity.rs a_token_field_in_a_fixture_key_builder_is_refused 402-456
mutants crates/pull/tests/unit.rs a_path_that_is_not_a_regular_file_is_refused_by_name 770-814
mutants crates/pull/tests/unit.rs readonly_credentials 944-989
mutants crates/pull/tests/unit.rs a_flipped_bit_in_any_entry_byte_is_detected 1290-1308
mutants crates/pull/tests/unit.rs a_date_that_does_not_exist_is_refused_with_the_length_that_month_has 4274-4338
mutants crates/pull/tests/unit.rs calendar_filter 4775-4868
mutants crates/pull/tests/unit.rs a_reservation_with_the_cursor_ahead_and_no_shortfall_goes_now 8468-8529
mutants crates/pull/tests/zerodha_identity.rs the_fixture_reads_a_weekend_and_an_unwritten_month_as_empty 2080-2096
mutants crates/pull/tests/zerodha_identity.rs line 2079
`````
