STATUS: IN PROGRESS

# report-BASE-COV-API-AS2 (BASE-COV-API repair round 3, shard G2, 5 Nov 2026)

Branch `cloud/cov-api-as2-20261105`, cut from the kit commit
`f4111a0132473d8771d8624a9538b1362926f70b`; START `6ea46e59c07488eeccff388407976041852aae2c`.
Owners in order: token `BASE-COV-API-AS2-20261105-1` (commits `dd7f285b`..`f13f4f57`), the main
session's pause `PAUSED-BY-MAIN-20261105-1420` (`0eff0822`), and this continuation,
`BASE-COV-API-AS2-20261105-3` (from `8ed8a90f`). The earlier session wrote no report, so its
commits are its record; this file is the shard's one report from here on.

## Kit check

Every kit file's sha256 matches `MANIFEST.md` (the fenced ones extracted with the manifest's
awk line). Pinned nightly installed as `nightly-2026-08-08`: `rustc -V` prints exactly
`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, its llvm-cov and llvm-profdata print
`LLVM version 23.1.0-rust-1.99.0-nightly`. Host: Linux, root, 4 cores, 15 GB memory, 30 GB
free disk; TMPDIR on a tmpfs folder of mode 1777.

## Done by the earlier session (token -1), read from its commits

- (b) Blocking item 2, `026e8be7`: the crowd child of
  `a_sources_question_it_cannot_read_or_a_full_pool_is_said_by_name` writes `child-ran` into
  a folder its parent names and reads, as `served_routes_tests.rs`' children do; the
  `println!` is gone. Its commit says gate 23's step run by hand lists the same 38 lines as
  f01d7a0e4 and no `api/src/sources_tests.rs` line (re-run by this session: see Gates).
- (c) Blocking item 3, `026e8be7`, `39046efa`, `f13f4f57`: `sources_tests.rs` that test
  13 extremes (10 / 2 / 1); `research_doors_tests.rs` a second combination; the small-size
  timing wrappers of `dayflags_rules_tests.rs` (3 tests), `disk_route.rs` and `server.rs`
  each have a new test beside them with a named `Extremes:` line (counts below, re-checked
  by this session).
- Two `should_panic` tests close the GAP rows `sources_tests.rs:429` and
  `research_doors_tests.rs:35`.

## This session

Work in progress: item (a), G2's GAP rows (1,972 rows at r2: 1,192 / 131 / 110 lines and
412 / 115 / 12 sides, production / test / ignored).

Tests added so far (each with an `Extremes:` doc line of at least 8 cases, 2 combined, 1 out
of the box, each asserted; each run green on the instrumented build and its target rows
checked covered with `llvm-cov show` of that run's profile):

| file | test | G2 rows it reaches (r2 numbering) |
|---|---|---|
| `fault_log.rs` | `a_lost_sidecar_is_one_cause_only_for_the_same_files_and_a_cut_keeps_characters` | 135:14 F, 135:24 F, 320:76 F, 385:11 T, 386-387 |
| `store_wire.rs` | `a_kept_answer_is_its_own_slot_key_and_type_and_an_unheld_row_is_dropped` | 72-74, 154:16 F, 155:20 F, 277:17 F, 278 |
| `instruments_route.rs` | `a_poisoned_body_is_a_503_and_an_untracked_master_row_is_not_listed` | 209-213, 303:17 F |
| `master.rs` | `a_master_path_that_is_a_directory_is_refused_as_one`; the FIFO test's unblock path removed | 336:16 T, 337; test 1075-1076 |
| `verify.rs` | `a_degraded_counter_is_scrubbed_and_said_degraded` | 276:12 T, 277-284 |
| `quarantine_route_tests.rs` | `the_verbs_misuse_and_refusal_and_an_unread_question_are_said_apart`; two test conditions asked exactly | quarantine_route.rs 132, 133, 151-155, 189-193; test 142:12 F, 151, 210:9, 398:12 |
| `livejson.rs` | `a_live_folder_that_cannot_be_listed_refuses_the_census` | 135 |
| `boolean_search_budget.rs`, `boolean_observation_budget.rs` | Extremes lines; 21+ digit values; the let-else is an assert | 20:16 T; 27:16 T; test 89:17, 90 |
| `dayflags_rules_tests.rs` | `a_day_this_request_refused_is_not_read_again_and_a_non_share_reads_nothing` | dayflags_rules.rs 598:21 F, 602:28 F |
| `topjson.rs` | `every_ledger_state_the_top_report_meets_is_answered_by_name` | 59:24 F, 81, 110:25 F, 169:21 F, 169:60 T/F, 170, 176:8 T, 177, 192 |
| `recovery_control.rs` | `every_path_that_is_not_a_stop_file_is_refused_by_what_it_is` | 66:25 F, 67, 68:23 F, 69, 87:25 F, 88, 89:23 F, 90, 131:8 T, 132-134, 166:8 T, 167, 172:67 T, 204:23 F, 214 |
| `calendar_record_tests.rs` | `months_out_of_order_or_twice_and_days_that_are_no_civil_day` | calendar_record.rs 135, 348:21 F, 349, 351:20 F |
| `scrub_route.rs` | `unreadable_lines_absent_ledgers_and_replaced_files_are_answered_apart` | 409:24 F, 427:17 F, 428, 1066:21 T, 1067, 1076:44 F, 1076:69 F, 1321 |
| `provenance_tests.rs` | `every_store_state_between_not_held_and_served_is_said_by_what_it_is` | provenance.rs 445-450, 483-488, 709:8 T, 710-711, 815-820, 857, 860-863 |
| `sources_tests.rs` | `a_root_no_folder_an_unlisted_rung_an_escaped_symbol_and_a_full_memo` | sources.rs 424-428, 426:24 T, 468-470, 485:12 T, 486, 507:21 F, 508, 1300, 1315-1321, 1336:31 F |
| `tests/cloud_mutants_bars.rs` | `spot_json_is_refused_by_name_when_every_store_read_slot_is_held` fixed (see host reds) | (its own run) |
| `operation_audit_tests.rs` | `an_unreadable_question_is_a_400_and_a_panicking_handler_is_said_as_run`; `Counted`/`Parked` | operation_audit.rs 154-160, 260:8 T, 267:8 T, 268, 284:12 T, 285, 297:12 T, 297:32 T, 298, 320-325; test 321:14, 422-425, 451, 453, 483, 517-519 |

Rows read as unreachable so far (proofs to go in `fu-AS2.txt.md`; platform independent):
`fault_log.rs:186:34 False` (`unnamed` > 0 only after 16 named runs, never removed);
`instruments_route.rs:194:51 False` (`MasterState::Unavailable` comes only from `Read::unread`,
which `server::universe` fills only from `master_paths`, the three mastered feeds);
`instruments_route.rs:204:20 False` (the ETag is a quoted hex digest, always a header value);
`dayflags_rules.rs:1190:20 False` (a judged run is open only after a flat judged bar, which
also opens the run through it); `sources.rs:426:24 False` (a nibble is below 16);
`sources.rs:1499:9 False` (the link is built only of unreserved bytes and `%XX`).

Host reds met so far (4-core root host):
- `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
  (G3's file): its child parks `MAX_CALENDAR_CONCURRENT` (8) holders on a barrier of 9
  parties, but the calendar pool has one slot per core (`detail::slots_under`, 4 here), so 4
  holders are refused at admission and the child hangs forever (all 9 threads in
  `futex_do_wait` after 54 min; stopped by hand). The same code as START. Owed to G3 / the
  Mac: the fix is the gate this shard used in `tests/cloud_mutants_bars.rs`.
- `tests/cloud_mutants_bars.rs` `spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
  (G2's file): the same defect on the store-read pool; FIXED in this shard (0.53 s here).

NEXT: close G2 production GAP rows file by file with tests; push after each file.
