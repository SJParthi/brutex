# cov-BASE-MSUM-BACKFILL-MS1.txt.md

Route B coverage (the kit's `d3-route-b-cloud.sh`, recipe v4 step 4) of the D2 set `deep-BASE-MSUM-BACKFILL-MS1-501651b3b6b5-set.txt.md` (at `ceb28bb9` the same set but for `upkeep_command_tests.rs`'s ranges). Pins checked by the script: `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, `llvm-cov` and `llvm-profdata` `LLVM version 23.1.0-rust-1.99.0-nightly` (`nightly-2026-08-08`, x86_64-unknown-linux-gnu). store and pull measured at `ceb28bb90b017814c70790afdc5e0a46a749a0b4`, whose crates/store and crates/pull are byte-identical at the code tip `501651b3` (`git diff --stat ceb28bb9 501651b3` names only crates/cli/src/upkeep_command_tests.rs); cli measured at `501651b3`. Test filters are the brief's gates: store all its tests, pull `upkeep`, cli `upkeep msum`. Host: root, 4 cores; the permission test re-runs itself as uid 65534 and its profile is counted. One fenced block holds every figure line the script printed, `line <n>: lines 0/0` items (lines outside a function that hold no code) left out.

`````text
ROUTE-B START 2026-11-03 17:55:25 IST tree /home/user/brutex tip ceb28bb90b017814c70790afdc5e0a46a749a0b4 packages store K /home/user/deep P /dev/shm/msum-tmp/prof/ceb28bb90b01-3-store-B (entries before: 0)
versions: rustc 1.99.0-nightly (1a98b1e13 2026-08-07) | llvm-cov  LLVM version 23.1.0-rust-1.99.0-nightly | llvm-profdata  LLVM version 23.1.0-rust-1.99.0-nightly
== build rc=0 END 2026-11-03 17:55:52 IST (0 min); 44 artifacts, 0 reused
== test store rc=101 END 2026-11-03 18:21:25 IST (25 min); tests passed 948 failed 5 ignored 8
  FAILED (a gate matter; its profile still counts): test quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply ... FAILED
  FAILED (a gate matter; its profile still counts): test quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply ... FAILED
  FAILED (a gate matter; its profile still counts): test settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new ... FAILED
  FAILED (a gate matter; its profile still counts): test settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible ... FAILED
  FAILED (a gate matter; its profile still counts): test settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason ... FAILED
COVERAGE crates/store/src/file_fault_tests.rs month_with_no_summary 1042-1051: lines 8/8, branches 0/0
COVERAGE crates/store/src/file_fault_tests.rs a_full_disk_leaves_no_partial_summary_under_its_name 1053-1084: lines 22/22, branches 0/0
COVERAGE crates/store/src/file_fault_tests.rs a_refused_sync_leaves_the_name_free 1086-1109: lines 17/17, branches 0/0
COVERAGE crates/store/src/file_fault_tests.rs TOTAL: lines 47/47, branches 0/0
COVERAGE crates/store/src/month_summary_tests.rs the_door_refuses_a_writer_and_never_a_reader 804-845: lines 26/26, branches 0/0
COVERAGE crates/store/src/month_summary_tests.rs a_second_door_is_refused_by_the_bar_files_own_lock 847-880: lines 22/22, branches 0/0
COVERAGE crates/store/src/month_summary_tests.rs the_door_names_an_empty_and_a_small_month_and_writes_nothing 882-911: lines 19/19, branches 0/0
COVERAGE crates/store/src/month_summary_tests.rs a_name_held_by_a_directory_is_refused_before_a_byte_is_written 913-961: lines 38/38, branches 0/0
COVERAGE crates/store/src/month_summary_tests.rs a_staging_name_the_host_refuses_is_the_doors_refusal 963-1002: lines 29/29, branches 0/0
COVERAGE crates/store/src/month_summary_tests.rs a_month_filed_under_another_name_is_refused_by_the_door 1004-1085: lines 66/66, branches 0/0
COVERAGE crates/store/src/month_summary_tests.rs TOTAL: lines 200/200, branches 0/0
COVERAGE crates/store/src/month_summary_writer.rs BarFile::month_summary_existing 235-310: lines 28/28, branches 4/4
COVERAGE crates/store/src/month_summary_writer.rs write_summary 313-316: lines 3/3, branches 0/0
COVERAGE crates/store/src/month_summary_writer.rs write_image 327-371: lines 32/32, branches 4/4
COVERAGE crates/store/src/month_summary_writer.rs TOTAL: lines 63/63, branches 8/8
COVERAGE crates/store/src/sort_index_writer.rs BarFile::open_to_sort 354-369: lines 7/7, branches 0/0
COVERAGE crates/store/src/sort_index_writer.rs BarFile::open_held 371-431: lines 50/50, branches 6/6
COVERAGE crates/store/src/sort_index_writer.rs TOTAL: lines 57/57, branches 6/6
ROUTE-B DONE 2026-11-03 18:21:29 IST: every package measured and read (store)

ROUTE-B START 2026-11-03 18:21:29 IST tree /home/user/brutex tip ceb28bb90b017814c70790afdc5e0a46a749a0b4 packages pull K /home/user/deep P /dev/shm/msum-tmp/prof/ceb28bb90b01-3-pull-B (entries before: 0)
versions: rustc 1.99.0-nightly (1a98b1e13 2026-08-07) | llvm-cov  LLVM version 23.1.0-rust-1.99.0-nightly | llvm-profdata  LLVM version 23.1.0-rust-1.99.0-nightly
== build rc=0 END 2026-11-03 18:23:23 IST (1 min); 166 artifacts, 129 reused
== test pull rc=0 END 2026-11-03 18:23:27 IST (0 min); tests passed 39 failed 0 ignored 0
COVERAGE crates/pull/src/upkeep.rs Lacks::without 1430-1434: lines 3/3, branches 0/0
COVERAGE crates/pull/src/upkeep.rs summary_owed_by_len 1731-1742: lines 4/4, branches 0/0
COVERAGE crates/pull/src/upkeep.rs walk_rung 1744-1824: lines 73/73, branches 16/16
COVERAGE crates/pull/src/upkeep.rs TOTAL: lines 80/80, branches 16/16
COVERAGE crates/pull/src/upkeep_tests.rs the_walk_lists_every_month_and_what_it_lacks 1258-1535: lines 209/209, branches 0/0
COVERAGE crates/pull/src/upkeep_tests.rs a_sidecar_name_held_by_anything_is_not_lacking 1537-1587: lines 38/38, branches 0/0
COVERAGE crates/pull/src/upkeep_tests.rs the_length_of_a_bar_file_says_when_no_summary_is_owed 2128-2150: lines 12/12, branches 0/0
COVERAGE crates/pull/src/upkeep_tests.rs the_length_rule_holds_for_every_bar_layout 2152-2173: lines 11/11, branches 0/0
COVERAGE crates/pull/src/upkeep_tests.rs a_set_of_lacks_without_a_kind_keeps_the_rest 2175-2184: lines 6/6, branches 0/0
COVERAGE crates/pull/src/upkeep_tests.rs the_walk_counts_a_month_too_short_to_owe_a_summary_apart 2186-2229: lines 34/34, branches 0/0
COVERAGE crates/pull/src/upkeep_tests.rs TOTAL: lines 310/310, branches 0/0
ROUTE-B DONE 2026-11-03 18:23:45 IST: every package measured and read (pull)

ROUTE-B START 2026-11-03 18:33:55 IST tree /home/user/brutex tip 501651b3b6b56ff30b7635ee9bce7e4d6d8b75f1 packages cli K /home/user/deep P /dev/shm/msum-tmp/prof/501651b3b6b5-4-cli-B (entries before: 0)
versions: rustc 1.99.0-nightly (1a98b1e13 2026-08-07) | llvm-cov  LLVM version 23.1.0-rust-1.99.0-nightly | llvm-profdata  LLVM version 23.1.0-rust-1.99.0-nightly
== build rc=0 END 2026-11-03 18:37:10 IST (3 min); 174 artifacts, 141 reused
== test cli rc=101 END 2026-11-03 18:40:05 IST (2 min); tests passed 46 failed 1 ignored 1
  FAILED (a gate matter; its profile still counts): test upkeep_command::tests::the_msum_store_half_is_a_pending_step_for_each_feed ... FAILED
COVERAGE crates/cli/src/msum_builder.rs reader_view 126-135: lines 6/6, branches 0/0
COVERAGE crates/cli/src/msum_builder.rs build 140-146: lines 5/5, branches 0/0
COVERAGE crates/cli/src/msum_builder.rs checked 148-203: lines 39/43, branches 0/0
  UNCOVERED LINE crates/cli/src/msum_builder.rs:184: other => {
  UNCOVERED LINE crates/cli/src/msum_builder.rs:185: return Line::Refused {
  UNCOVERED LINE crates/cli/src/msum_builder.rs:186: why: format!("the store reported an outcome this build does not know: {other:?}"),
  UNCOVERED LINE crates/cli/src/msum_builder.rs:187: };
COVERAGE crates/cli/src/msum_builder.rs TOTAL: lines 50/54, branches 0/0 (every uncovered line and side above is a GAP unless it is an (F) or (U) item of recipe step 4)
COVERAGE crates/cli/src/msum_builder_read_back_tests.rs whole: lines 35/35, branches 0/0
COVERAGE crates/cli/src/msum_builder_read_back_tests.rs TOTAL: lines 35/35, branches 0/0
COVERAGE crates/cli/src/upkeep_command.rs items 433-495: lines 39/39, branches 8/8
COVERAGE crates/cli/src/upkeep_command.rs plan 778-893: lines 83/83, branches 14/14
COVERAGE crates/cli/src/upkeep_command.rs run 1006-1069: lines 52/52, branches 10/10
COVERAGE crates/cli/src/upkeep_command.rs msum_given 1071-1088: lines 12/12, branches 2/2
COVERAGE crates/cli/src/upkeep_command.rs msum_left 1090-1106: lines 12/12, branches 2/2
COVERAGE crates/cli/src/upkeep_command.rs TOTAL: lines 198/198, branches 36/36
COVERAGE crates/cli/src/upkeep_command_tests.rs a_whole_feed_gets_its_sort_index_in_one_run 783-850: lines 44/44, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs sum 2284-2287: lines 3/3, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs summarised 2289-2293: lines 4/4, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs bare 2295-2300: lines 5/5, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs lacking_sum 2302-2310: lines 8/8, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs step_running 2312-2318: lines 6/6, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs the_msum_store_half_is_a_pending_step_for_each_feed 2320-2367: lines 32/32, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_plan_names_the_lacking_months_and_none_after_the_step 2369-2412: lines 34/34, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_writer_commit_leaves_no_month_without_its_summary 2414-2451: lines 25/25, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_run_builds_beside_a_reader_and_refuses_a_writers_month_by_name 2453-2482: lines 22/22, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs an_all_run_rebuilds_a_spoiled_summary_and_keeps_a_true_one 2484-2525: lines 30/30, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs version_1_image 2527-2549: lines 19/19, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs an_all_run_builds_a_version_1_summary_whole_as_version_2 2551-2598: lines 42/42, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs an_empty_and_a_zero_length_month_are_told_apart 2600-2639: lines 28/28, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_sealed_month_beside_its_raw_form_is_summarised_and_left_as_it_was 2641-2667: lines 21/21, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_month_repaired_mid_run_keeps_its_own_summary 2669-2733: lines 55/55, branches 2/2
COVERAGE crates/cli/src/upkeep_command_tests.rs an_import_during_a_run_stops_it_and_a_resume_finishes_the_same_bytes 2735-2792: lines 47/47, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_refused_month_then_a_resume_at_another_worker_count_builds_only_it 2794-2837: lines 27/27, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_clock_stepped_back_gives_a_resume_its_own_id 2839-2893: lines 34/34, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs a_run_says_on_store_sum_what_it_was_given_and_left 2895-2971: lines 66/66, branches 0/0
COVERAGE crates/cli/src/upkeep_command_tests.rs TOTAL: lines 552/552, branches 2/2
ROUTE-B DONE 2026-11-03 18:40:24 IST: every package measured and read (cli)

`````
