STATUS: IN PROGRESS

# report-BASE-COV-API-AS2 (BASE-COV-API repair round 3, shard G2, 5-6 Nov 2026)

Branch `cloud/cov-api-as2-20261105`, cut from the kit commit
`f4111a0132473d8771d8624a9538b1362926f70b`; START `6ea46e59c07488eeccff388407976041852aae2c`.
Owners in order: token `BASE-COV-API-AS2-20261105-1` (commits `dd7f285b`..`f13f4f57`), the main
session's pause `PAUSED-BY-MAIN-20261105-1420` (`0eff0822`), `BASE-COV-API-AS2-20261105-3`
(`8ed8a90f`..`c47e9a06`; the account's weekly limit cut it at about 21:28 IST on 5 Nov, its work
intact), and this continuation, `BASE-COV-API-AS2-20261105-4` (from `725e11d5`, 6 Nov). The first
session wrote no report, so its commits are its record; this file is the shard's one report.

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

## Session -3 (5 Nov)

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

Session -3 ended (weekly limit) with this NEXT: close G2 production GAP rows file by file
with tests; push after each file.

## Session -4 (6 Nov)

### Host and pins

A fresh container: Linux, root, 4 cores, 15 GB memory, 29 GB free disk, TMPDIR
`/dev/shm/as2tmp` (tmpfs, mode 1777). The pinned nightly was installed again as
`nightly-2026-08-08`: `rustc -V` prints exactly `rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`;
its llvm-cov and llvm-profdata print `LLVM version 23.1.0-rust-1.99.0-nightly`. fmt and clippy
use the repository's pinned toolchain, `1.97.1` (`rust-toolchain.toml`). One
`CARGO_TARGET_DIR`; `CARGO_PROFILE_DEV_DEBUG=0` (allowed by CONTEXT for room; no coverage
mapping depends on it).

### Baseline: Route B on the tip this session took over

`d3-route-b-cloud.sh /home/user/rb /home/user/tgt <new profile folder> <new log folder> <set> api
-- --skip a_full_calendar_pool_refuses_the_census_and_the_audit`, tree `/home/user/rb` (a
detached worktree at `725e11d5`, whose code is `c47e9a06`'s), the readout limited to G2's 58
files (a set file of their 58 `coverage ... whole` lines taken from the kit's set). The skipped
test is G3's, the 4-core hang session -3 named (see host reds). The profile folder was opened to
uid 65534 so the children of `isolated::where_permission_binds` write their profiles.

- build rc 0 in 3 min (170 artifacts); `== test api rc=101 END 2026-11-06 08:24:00 IST (5 min);
  tests passed 3103 failed 14 ignored 16`; lib: `test result: FAILED. 3019 passed; 13 failed;
  16 ignored; 0 measured; 1 filtered out; finished in 188.37s`; `ROUTE-B DONE 2026-11-06
  08:29:47 IST: every package measured and read (api)`.
- G2's 58 files on this host: lines 63,325/64,808 (1,483 uncovered), branches 4,493/5,432 (939
  uncovered sides), (F) rows included. Repair r2's figures for the same files (Mac,
  8091e72f7): 61,694/63,260 (1,566) and 4,442/5,438 (996). The two hosts differ in what
  each reaches (macOS-only lines, root, 4 cores), so the comparison is a size, not a proof.
- The 14 reds: `server::head_deadline_body_tests::` (G2, five: see host reds), and nine of
  other shards' files: `audit::cloud_mutant_tests::an_unreadable_journal_refuses_the_index_build_and_says_so`,
  `detail::tests::a_refused_admission_names_what_its_bound_and_why` (a 4-slot pool),
  `pullrun::cloud_mutant_tests::newest_records_that_cannot_be_read_leave_the_answer_not_known`,
  `pullrun::restart_tests::the_journals_own_extremes`,
  `server::basis_route::tests::a_table_refused_as_unreadable_is_read_again_once_its_mode_is_repaired`
  ("the test needs a reader the mode refuses (not root)"),
  `server::cloud_mutant_tests_c::a_store_that_takes_no_write_trips_the_store_breaker`,
  `server::cloud_mutant_tests_c::a_store_whose_audit_dir_cannot_be_made_is_at_fault`,
  `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted` ("this
  host reads a 000 directory (root?)") and `tests/pool_load.rs`
  `the_feed_list_stays_fast_while_32_clients_loop_each_route` (L-BCA-4). Their files are
  G1's and G3's, so they are named here and not taken up.

### Test-code GAP rows closed by restructure (no check weakened)

Each row was a collector branch, a `match` arm that panics, an `||`/`&&` side inside a closure,
or a guard a run cannot take; each is now a direct `assert!`/`assert_eq!`, a one-line let-else
whose else panics (the (F) forms repair r2's reading kept), a branch-free form of the same
check, or code that runs:

- `tests/logs_r3_24.rs` (274, 355, 399, 403-407; 273:16, 402:24, 402:56): the two collectors
  are per-case `assert_eq!`s; `every_page` panics by name past 200 pages.
- `tests/store_step_forms.rs` (62-63): a loop over a list asserted empty on the line before it
  could never run; removed, with the reason.
- `census_tail_tests.rs` (110, 501), `calendar_of.rs` (1349), `fno_boundary_tests.rs` (42, 220):
  match arms that panic are let-else.
- `fno_boundary_tests.rs` (178): the mock vendor spawns the serve future itself, not a block
  whose tail no run reaches; `head_deadline_body_tests.rs` (138-139): the shutdown future is one
  expression for the same reason.
- `head_deadline_body_tests.rs` (199, 218, 243, 857, 890, 893-895, 903-904): `paced`'s and
  `drain`'s timeouts panic by name (`drain`'s callers all asserted the close, so `drain`
  returns only the text); the pattern check is an `assert_eq!`; the reader's end of socket and
  the write's error are assertions; the waits are recorded and judged after the loop, so which
  kinds a host meets no longer decides which lines run.
- `bars_unknown_series_tests.rs` (455-457): runs through `isolated::where_permission_binds`
  (D-0995), so the refusal is read on every host and no root-only path is left.
- `bars_sort_tests.rs` (499, 955, 1082-1083, 1508-1510): a range `contains`, `min_by_key` /
  `max_by_key` for the tie-break, the one field the holes line names its series by (`key`,
  bars.rs:713), and the foreign-index page asserted as it is (no row served, 360 named faults).
- `bars_sort.rs` (3013, 4196-4197, 4327), `constituents.rs` (1619, 1625): `&&` inside `.any()`
  is `.filter(..).any(..)`; a closure inside an assert's message is a fixed message.
- `render.rs` (3230-3232, 3365, 3371, 4219, 4459): one discovery closure offered to both
  switch positions; the capped note's expectation follows the process's switch in one
  `matches!`; the share-bar and history checks assert the text the fixtures make.
- `server_cloud_mutant_tests_e.rs` (255, 314, 317, 328, 361): a `matches!` assert plus a
  one-line let-else; `ending_on` takes its instant; `[before, after].contains`.

### New tests (each with an `Extremes:` doc line)

| file | test | extremes (single / combined / out of the box) | rows it reaches (baseline numbering) |
|---|---|---|---|
| `sources_tests.rs` | `the_integer_comparisons_month_walks_and_fold_grid_answer_at_their_edges` | 12 (9 / 2 / 1) | sources.rs 196-201, 896:18 T, 908:18 T, 926:5 F, 939:5 F, 1638:16 F, 1653:15 T |
| `sources_tests.rs` | `a_row_is_compared_by_its_feeds_roles_and_tallied_by_what_it_was` | 11 (8 / 2 / 1) | sources.rs 2387:8 T, 2388, 2400:55 F, 2400:72 F, 2403, 2433:35 T, 2461:13 F, 2462-2463, 2534:24 T, 2535, 2539, 2560, 2598 |
| `render.rs` | `the_finder_names_a_handler_by_its_shape_and_lets_prose_be` | 12 (8 / 2 / 2) | render.rs 5133:19 F, 5136:29 T, 5137 |
| `server_cloud_mutant_tests_e.rs` | `the_held_back_model_answers_both_ways_at_fixed_instants` | 11 (8 / 2 / 1) | the file's 314:8 T, 317, 328 |
| `sources_tests.rs` | `the_day_questions_an_unmeasured_basis_and_a_missing_days_reason_are_said_at_their_edges` | 12 (9 / 2 / 1) | sources.rs 1044:29 F, 1062, 1153-1155, 2671:12 T, 2672, 2677, 3804:9 F, 3805, 3812:12 T, 3813, 3819:8 T, 3821:29 T, 3823 |
| `booleanlaunch_tests.rs` | `each_launch_field_at_its_edge_is_refused_in_its_own_words` | 15 (12 / 2 / 1) | booleanlaunch.rs 90:48 F, 109:8 T, 111:12 T, 113:13 T, 150:8 F, 152:12 T, 367-369, 425:8 T, 425:31 T, 426-429, 471:8 T, 472:12 T |
| `booleanlaunch_tests.rs` | `observation_limits_too_small_or_unreadable_are_said_by_the_metadata_route` (seven children, each proving itself by a file) | 11 (8 / 2 / 1) | booleanlaunch.rs 224, 241:8 T, 241:32 T, 242, 408-410, 421:16 T, 422-423 |

Also `booleanlaunch_tests.rs` 439, 460-465: the configuration child's profile path is passed by
`Command::envs` over the `Option`, and its 45 s / 1 MiB watchdog is an assert, with a guard
that kills and reaps the child on every way out (a failing assert included).

Instrumented filtered run of every touched file's tests (`devtest.sh r4`, one `cargo test`):
lib `337 passed; 5 failed; 1 ignored` (the five head-deadline host-red candidates only),
`tests/logs_r3_24.rs` 3 passed, `tests/store_step_forms.rs` 3 passed; each target row read
covered with `llvm-cov show` of that run's profile, the rows left in the edited regions being
(F) forms. `cargo fmt --all --check` rc 0; `cargo clippy -p api --all-targets --locked --
-D warnings` rc 0 (1.97.1).

### Rows proved (U) so far (platform independent; proofs go in `fu-AS2.txt.md`)

- server.rs `let Some(store_vendor) = feed.store_vendor() else` (tip 9463, 9464-9469):
  `Feed::store_vendor` (crates/pull/src/vendor.rs:3891-3906) is Some on every arm.
- server.rs `let Some(layout) = archive.layout(Segment::Fno) else` (tip 9960, 9961-9967): the
  only two archive descriptors (vendor.rs:5670, :5784) each declare an Fno layout (:5707,
  :5801; TRUEDATA_FNO :5647, GDFL_FNO :5756).
- sources.rs 1632:13 F, 1633: `windows(2)` yields only two-element slices.
- sources.rs 2458:16 F: `at` enumerates a four-element array and `equal` is `[false; 4]`.
- booleanlaunch.rs 450:12 F: `booleanevidencejson::search_policy` returns
  `admission_projection::policy`, whose tail is the object literal
  `json!({"digest":..,"values":rows})` (booleanadmission_projection.rs:76-133), so
  `as_object_mut()` is always Some.

### Host reds (G2's files)

- The five `server::head_deadline_body_tests::` slow-reader tests
  (`a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole`,
  `a_chunked_body_reaches_a_slow_reader_whole`,
  `a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole`,
  `slow_readers_at_the_cap_are_served_whole_and_the_next_waits`,
  `pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is`) fail on this host in
  every run so far, in the baseline (the file as at START) and filtered: each response is cut
  at the stall deadline after about 5.4 MB of 16 or 8 MiB ("cut after 5374637 of 16777216
  bytes at 1.633183617s"), and one reader meets a connection reset. PROVED a host red: on
  START `6ea46e59` (the scratch worktree, every tracked file touched, the same instrumented
  build) `cargo test -p api --lib --locked --no-fail-fast -- server::head_deadline_body_tests`
  prints `test result: FAILED. 7 passed; 5 failed` with the same five messages ("cut after
  5374637 of 16777216 bytes at 3.764259572s", "cut after 5484147 framed bytes", "cut after
  5374638 of 8388608", "cut after 3904835 bytes", "cut after 3904834 of 16777216"), and each
  alone (`--exact <name> --test-threads=1`) fails the same way, so it is not load: this
  kernel (Linux 6.18.44; `tcp_wmem` 4096 16384 4194304, `tcp_rmem` 4096 131072 33554432)
  holds 3.9-5.5 MB in flight when the stall deadline cuts the paced reader. Verify r3 ran the
  file green on the Mac. Owed to the Mac's Route B: these tests' lines past the cut.

### Red round 1 (each new test shown red by a hand change, then restored)

Scratch worktree at `c954f932`, four hand changes, instrumented build, `cargo test -p api
--lib --locked --no-fail-fast -- <the four>`: `test result: FAILED. 0 passed; 4 failed`,
each at its own assertion:

- `sources.rs` `price_cmp` without `|| a < 0`: "assertion `left == right` failed: a
  negative price; left: Some((Number(-10001.00), true))".
- `sources.rs` `compare_row` comparing one vendor's two cells unless their rungs differ:
  `left: One` for `RowBasis::NotCompared`.
- `render.rs` `inline_handler` without `b'/'` in attribute position: `"<br/onfocus=z>"`,
  `left: None`.
- `server.rs` `settled_today` with `minute > settles`: "past close", `left: Err("REFUSED · the
  window ends 2025-07-01 and today is 2025-07-01 ...`.

The worktree was restored (`git checkout -- .`) and the main tree's tracked files touched
before its next build (OPS-10).

NEXT: red round 2 for the three tests after round 1; then G2 production rows file by file
(calendar_route, speed, trades, calendar_record, disk_route, calendar_of, scrub_route, topjson,
operation_audit, master, then sources' day table, provenance, bars_sort, census_cache, server).
