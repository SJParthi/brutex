# cloud-mut-api-a-20261016 — mutation and coverage pass on eight `api` files

Status: **done, except the items under *NOT DONE* at the end**, each with its
reason. Every number below was measured in this session; none is
extrapolated.

Branch `cloud/mut-api-a-20261016`, cut from `origin/cloud/base-20261016`
(`c9e3472e`, the one-commit snapshot of local `feat/db-fix` at `4756cce0`).

## Outcome in one table

| | Before | After |
|---|---:|---:|
| mutants listed | 2,145 | 2,145 |
| unviable (do not build) | 188 | 188 |
| caught, or timed out (detected) | 1,689 | 1,879 |
| **survived** | **268** | **78** |
| of the survivors, equivalent (no test can kill them; reasons below) | — | 60 |
| of the survivors, NOT DONE (killable in principle; reasons below) | — | 18 |
| mutation score, detected ÷ viable | 86.3 % | 96.0 % |
| the same, equivalent mutants left out | — | 99.1 % |

**One real bug was found and fixed** (`bars_series.rs`, *Real bugs found*):
an `at=` instant past the year 9999 was ranked before every row.

## Scope

`crates/api/src/` — `bars.rs`, `bars_change.rs`, `bars_series.rs`,
`bars_sort.rs`, `gaps_route.rs`, `spot.rs`, `render.rs`, `ladder.rs`.
`dayflags.rs` is not touched. Nothing outside `crates/api` is edited.

What changed under `crates/api`:

- **One new test file**, `crates/api/tests/cloud_mutants_bars.rs`, for what
  a test outside the crate can reach.
- **One new `#[cfg(test)] mod cloud_mutant_tests`** at the end of each of
  `bars.rs`, `bars_change.rs`, `bars_sort.rs`, `gaps_route.rs` and
  `render.rs`, for private items.
- **One production change, in `bars_series.rs`, which fixes a real bug**
  (see *Real bugs found*).

No existing test was edited, weakened or deleted.

## Method

| Item | Value |
|---|---|
| toolchain | `1.97.1` (pinned by `rust-toolchain.toml`) |
| cargo-mutants | `27.1.0` |
| cargo-llvm-cov | `0.9.1` |
| machine | 4 cores, 15 GiB, cloud container, runs as `root` |
| mutants listed | `cargo mutants --list --file <f> -p api` |

**Before.** Three workers run the mutants, each in its own full copy of the
tree (`cp -a`, so file times survive and cargo reuses the dependency
build). Each runs `cargo mutants --in-place --shard k/n --sharding slice`
with the stage-1 filter below, and every shard is time-boxed to 25 minutes
by `timeout 1500`. A shard the box cut short is finished afterwards: exactly
its mutants with no outcome are run by name (`--re`), against the original
sources, under the same filter. That is how 4 of `bars.rs`, 71 of
`bars_series.rs` and 153 of `bars_sort.rs` were measured. For `bars_sort.rs`,
488 outcomes came from its 16 shards: four ran to the end, eleven were cut
by the box, and one was lost to a container restart. The other 153 ran by
name, in six runs. Every one of the 641 has exactly one outcome.

**Stage-1 filter.** Each file's mutants run against the `api` library tests
whose path names the module and the routes that call it:

| File | `cargo test --lib -p api -- …` |
|---|---|
| `spot.rs` | `spot optiongreeks chain:: bars_page_tests` |
| `ladder.rs` | `ladder` |
| `render.rs` | `render` |
| `gaps_route.rs` | `gaps gap_peer calendar_route_tests census_request_tests serve_edge_tests` |
| `bars.rs` | `bars spot gaps` |
| `bars_change.rs`, `bars_series.rs`, `bars_sort.rs` | `bars` |

**After.** Every stage-1 survivor is run again under the same filter, now
with the new tests: the new `cloud_mutant_tests` modules (their paths match
the filter) plus `--test cloud_mutants_bars`.

**Stage 2.** A survivor no new test kills is run against the WHOLE `api`
library suite, existing tests only (`--skip cloud_mutant_tests`). That was
done for `spot.rs`, `ladder.rs`, `gaps_route.rs` and `bars.rs`. For
`bars.rs`, `bars_change.rs`, `bars_series.rs`, `render.rs` and
`bars_sort.rs`, the survivors left after the new tests were run against the
final suite: the whole library, the new tests and
`tests/cloud_mutants_bars.rs`. That run skips only
`coverage::tests::the_lookup_does_not_grow_with_the_universe`, a timing test
that fails on unmutated code under this machine's load (see *NOT DONE*). A
mutant reported "left alive" for those five files survives every `api` test.
For `spot.rs` and `ladder.rs` it survives the after-run (its filter, the new
tests and the integration file) and every existing library test.

**A full disk, once, and what was done about it.** At 11:25–11:40 UTC the
container's disk filled while five mutation trees were building. Two
after-runs stopped part-way, and five outcomes in them were the disk's, not
the mutant's: a test that cannot write its fixture fails, and a build that
cannot link is "unviable". Every mutation log of the pass was searched for
the three ways a full disk is reported (`DiskFull`, `StorageFull`, `No space
left`). Those five outcomes, all in the `bars_sort.rs` after-runs, are
discarded, and they and the stopped runs' untested mutants were run again
with space free. No before-run and no other file's run was affected.

Two deviations from a plain `cargo mutants`, both forced and both stated:

1. **The api crate is built at `opt-level = 0`, incrementally, without debug
   info, for the mutation runs only** (`--config profile.{dev,test}.package.api.
   opt-level=0`, `incremental=true`, `debug=0`). The checked-in `[profile.dev]`
   is `opt-level = 3`, `incremental = false`. Under it one mutant of this
   212,892-line crate is a multi-minute rebuild, and 2,145 mutants do not fit
   in a session. Dependencies keep the checked-in profile. Gates and the
   reported coverage use the checked-in profile unchanged.
2. **The copies have their `.git` moved aside.** `crates/cli/build.rs` stamps
   the commit only when the working tree equals the index, and it watches
   every tracked file to decide that. An in-place mutation of an `api` file
   therefore flipped `cli`'s stamp on every mutant, and rebuilt `cli` and all
   of `api` each time (measured: `cli` recompiled for each of the first two
   mutants). With no `.git` the stamp is the constant refusal sentinel and
   only `api` rebuilds (measured: 11 s a mutant).

**`--re` does not filter field deletions.** A `cargo mutants --re …` run also
tests every "delete field … from struct … expression" mutant of the file,
whatever the expression says. Those extra outcomes are ignored. Only the
names asked for are reported.

**Ten `api` tests fail on this machine before any change, and every one says
why.** Each is a permission test, the session runs as `root`, and `root` is
not bound by mode bits. Each one either panics with its own sentence
(*"mode 000 does not stop this process from searching …: run the tests as a
user the mode bits bind"*) or makes a directory `0o555` and expects the write
to be refused. They are passed to every mutation run as `--skip`, and are run
as uid 65534 for the gates (below):

```
logs::tests::a_directory_that_turns_unreadable_is_never_answered_as_read
logs::tests::a_ring_file_that_stays_unreadable_is_named_once
logs::tests::logs_names_a_directory_it_may_not_search
pullrun::tests::an_absent_index_is_built_by_the_scan_and_not_known_is_said_when_it_cannot_be
pullrun::tests::an_index_that_cannot_be_read_never_leaves_the_answer_clean
server::tests::a_failure_of_the_store_keeps_the_leg_owed_and_is_never_called_standing
server::tests::a_probe_name_that_cannot_be_unlinked_is_a_fault_by_name
server::tests::a_run_cut_short_by_a_restart_is_logged_when_it_is_found
server::tests::a_store_that_cannot_take_a_write_is_found_by_asking_it
server::tests::an_expired_fno_walk_cut_short_is_seen_at_the_next_start
```

**Tests that need binding mode bits re-run themselves.** Two new tests,
`render::cloud_mutant_tests::a_folder_the_walk_may_not_list_is_passed_over`
and `bars::sort::cloud_mutant_tests::a_host_refusal_of_an_index_is_this_requests_alone`,
run their bodies through the crate's existing
`crate::isolated::where_permission_binds`. As `root` that re-runs the test as
uid 65534 in a child process. So they assert under the mutation runs too, where
the existing host-refusal tests beside them stand down as `root`.

**Process-wide pools are filled in a child.** `/gaps.json`'s calendar pool
(8 slots) is shared by every test in the library binary. The test that fills
it, `a_full_calendar_pool_refuses_the_census_and_the_audit`, re-runs itself
in a child process where nothing else needs the pool. The store-read pool
filled by `spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
lives in the integration binary, where no other test reads through it.

## Mutants listed

| File | Mutants |
|---|---:|
| `bars.rs` | 392 |
| `bars_change.rs` | 146 |
| `bars_series.rs` | 330 |
| `bars_sort.rs` | 641 |
| `gaps_route.rs` | 179 |
| `spot.rs` | 123 |
| `render.rs` | 222 |
| `ladder.rs` | 112 |
| **total** | **2,145** |

## Results

The before columns use the stage-1 filter and the original tests. A
*timeout* is a mutant that made the suite hang past cargo-mutants' limit:
detected, and not a survivor. *Killed by a new test* counts stage-1
survivors that a test added by this pass kills; when the new test's input
makes the mutant loop, the timeout is the kill, and those are counted
separately in brackets. *Stage 2* counts stage-1 survivors that no new test
was written for, and that an existing test outside the filter kills. *Left
alive* is after everything: the equivalent mutants and the NOT DONE ones,
each listed below with its reason.

| File | Mutants | Caught | **Missed before** | Timeout | Unviable | Killed by a new test | Stage 2 | **Left alive** |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `spot.rs` | 123 | 105 | **5** | 0 | 13 | 4 | 0 | **1** (NOT DONE) |
| `ladder.rs` | 112 | 76 | **12** | 0 | 24 | 8 ¹ | 0 | **4** (equivalent) |
| `gaps_route.rs` | 179 | 125 | **15** | 1 | 38 | 15 | 0 | **0** |
| `render.rs` | 222 | 189 | **31** | 0 | 2 | 30 | 0 | **1** (equivalent) |
| `bars_change.rs` | 146 | 125 | **12** | 1 | 8 | 8 | 0 | **4** (equivalent) |
| `bars.rs` | 392 | 311 | **44** | 7 | 30 | 30 [+1] | 2 | **11** (9 equivalent, 2 NOT DONE) |
| `bars_series.rs` | 330 | 229 | **56** | 16 | 29 | 48 [+3] | 0 | **5** (3 equivalent, 2 NOT DONE) |
| `bars_sort.rs` | 641 | 448 | **93** | 56 | 44 | 41 | 0 | **52** (39 equivalent, 13 NOT DONE) |
| **total** | **2,145** | **1,608** | **268** | **81** | **188** | **184 [+4]** | **2** | **78** (60 equivalent, 18 NOT DONE) |

¹ Six of `ladder.rs`'s eight are also killed by existing `server::tests`
outside the stage-1 filter. They are counted once, here.

## Survivors killed, file by file

Each row is a stage-1 survivor that is now killed, and the test or tests
that kill it (the first two that fail, by name). A test in
`tests/cloud_mutants_bars.rs` is named by its module there (`spot_rs::…`);
a test in a source file by its path in the library
(`bars::sort::cloud_mutant_tests::…`). The third column names an EXISTING test
outside the stage-1 filter that also kills the mutant, where stage 2 or the
final suite found one. A row with no new test is a stage-2 kill.

### `spot.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `spot.rs:480:9: replace SpotAsk::answer -> Result<(Vec<Spot>, Vec<String>), String> with Ok((vec![], vec!["xyzzy".into()]))` | `spot_rs::spot_answer_reads_the_file_as_answer_with_does` | — |
| `spot.rs:480:9: replace SpotAsk::answer -> Result<(Vec<Spot>, Vec<String>), String> with Ok((vec![], vec![String::new()]))` | `spot_rs::spot_answer_reads_the_file_as_answer_with_does` | — |
| `spot.rs:480:9: replace SpotAsk::answer -> Result<(Vec<Spot>, Vec<String>), String> with Ok((vec![], vec![]))` | `spot_rs::spot_answer_reads_the_file_as_answer_with_does` | — |
| `spot.rs:535:34: replace == with != in SpotAsk::answer_with` | `spot_rs::a_cancelled_spot_read_of_one_instant_is_refused_at_its_first` | — |

### `ladder.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `ladder.rs:153:9: replace OwedWhy::label -> &'static str with ""` | `ladder_rs::ladder_every_owed_reason_is_said_in_its_own_words` | `server::tests::a_minute_run_is_refused_until_the_day_pass_has_landed`, `server::tests::an_instrument_with_a_daily_hole_is_withheld_alone_and_the_rest_are_pulled` |
| `ladder.rs:153:9: replace OwedWhy::label -> &'static str with "xyzzy"` | `ladder_rs::ladder_every_owed_reason_is_said_in_its_own_words` | `server::tests::a_minute_run_is_refused_until_the_day_pass_has_landed`, `server::tests::an_instrument_with_a_daily_hole_is_withheld_alone_and_the_rest_are_pulled` |
| `ladder.rs:184:9: replace Gate::is_open -> bool with true` | `ladder_rs::ladder_only_the_open_gate_is_open` | — |
| `ladder.rs:660:5: replace owing -> Vec<Owed> with vec![]` | `ladder_rs::ladder_owing_names_the_hole_only_where_the_order_applies` | `server::tests::an_instrument_with_a_daily_hole_is_withheld_alone_and_the_rest_are_pulled`, `server::tests::owing_names_every_run_past_the_refusals_cap` |
| `ladder.rs:660:8: delete ! in owing` | `ladder_rs::ladder_owing_names_the_hole_only_where_the_order_applies` | `server::tests::an_instrument_with_a_daily_hole_is_withheld_alone_and_the_rest_are_pulled`, `server::tests::owing_names_every_run_past_the_refusals_cap` |
| `ladder.rs:686:32: replace < with == in owed_runs` | `ladder_rs::ladder_an_unordered_ask_still_owes_the_hole_between_held_months` | — |
| `ladder.rs:822:9: replace <impl HoldsKey for crate::census::Manifests>::holds -> bool with false` | `ladder_rs::ladder_a_census_of_manifests_holds_what_its_manifest_holds` | `server::tests::a_minute_run_is_refused_until_the_day_pass_has_landed`, `server::tests::an_instrument_with_a_daily_hole_is_withheld_alone_and_the_rest_are_pulled` |
| `ladder.rs:822:9: replace <impl HoldsKey for crate::census::Manifests>::holds -> bool with true` | `ladder_rs::ladder_a_census_of_manifests_holds_what_its_manifest_holds` | `server::tests::an_instrument_with_a_daily_hole_is_withheld_alone_and_the_rest_are_pulled` |

### `gaps_route.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `gaps_route.rs:500:17: delete field from from struct Self expression in Denominator::peers` | `server::gaps_route::cloud_mutant_tests::a_vote_with_no_calendar_keeps_its_readings_and_its_unreadable_peers` | — |
| `gaps_route.rs:501:17: delete field unreadable from struct Self expression in Denominator::peers` | `server::gaps_route::cloud_mutant_tests::a_vote_with_no_calendar_keeps_its_readings_and_its_unreadable_peers` | — |
| `gaps_route.rs:576:19: replace > with < in carved` | `server::gaps_route::cloud_mutant_tests::a_halt_that_runs_to_the_close_carves_a_one_window_session` | — |
| `gaps_route.rs:670:24: replace != with == in gaps_body` | `server::gaps_route::cloud_mutant_tests::stale_is_today_past_the_last_day_and_never_a_peer_vote` | — |
| `gaps_route.rs:671:9: replace && with \|\| in gaps_body` | `server::gaps_route::cloud_mutant_tests::stale_is_today_past_the_last_day_and_never_a_peer_vote` | — |
| `gaps_route.rs:671:85: replace > with < in gaps_body` | `server::gaps_route::cloud_mutant_tests::stale_is_today_past_the_last_day_and_never_a_peer_vote` | — |
| `gaps_route.rs:671:85: replace > with == in gaps_body` | `server::gaps_route::cloud_mutant_tests::stale_is_today_past_the_last_day_and_never_a_peer_vote` | — |
| `gaps_route.rs:671:85: replace > with >= in gaps_body` | `server::gaps_route::cloud_mutant_tests::stale_is_today_past_the_last_day_and_never_a_peer_vote` | — |
| `gaps_route.rs:679:59: replace >= with < in gaps_body` | `server::gaps_route::cloud_mutant_tests::a_span_is_covered_only_when_both_its_ends_are_inside_the_source` | — |
| `gaps_route.rs:680:8: replace && with \|\| in gaps_body` | `server::gaps_route::cloud_mutant_tests::a_span_is_covered_only_when_both_its_ends_are_inside_the_source` | — |
| `gaps_route.rs:823:39: replace && with \|\| in peer_vote_reading` | `server::gaps_route::cloud_mutant_tests::a_vote_that_could_not_read_every_census_is_never_kept` | — |
| `gaps_route.rs:1008:9: delete match arm "census" in vote_building` | `server::gaps_route::cloud_mutant_tests::a_vote_held_by_the_start_up_census_names_that_wait` | — |
| `gaps_route.rs:1313:45: replace == with != in audit_one` | `server::gaps_route::cloud_mutant_tests::a_one_second_option_is_audited_on_the_derivatives_hours` | — |
| `gaps_route.rs:1695:55: replace match guard day != pull::calendar::SYSTEMS_OUTAGE_DAY with true in second_audit` | `server::gaps_route::cloud_mutant_tests::the_outage_day_owes_no_second` | — |
| `gaps_route.rs:1763:53: replace > with >= in close_run` | `server::gaps_route::cloud_mutant_tests::of_two_equally_long_runs_the_earlier_is_the_longest` | — |

### `render.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `render.rs:1380:26: replace == with != in share_bar` | `render::cloud_mutant_tests::each_drop_reason_takes_its_own_share_and_only_the_last_the_remainder` | — |
| `render.rs:1386:18: replace > with >= in share_bar` | `render::cloud_mutant_tests::each_drop_reason_takes_its_own_share_and_only_the_last_the_remainder` | — |
| `render.rs:1480:5: replace field_unlabelled -> String with "xyzzy".into()` | `render::cloud_mutant_tests::an_unlabelled_field_wraps_its_control_in_a_div_with_its_caption` | — |
| `render.rs:1480:5: replace field_unlabelled -> String with String::new()` | `render::cloud_mutant_tests::an_unlabelled_field_wraps_its_control_in_a_div_with_its_caption` | — |
| `render.rs:1515:5: replace date_input -> String with "xyzzy".into()` | `render::cloud_mutant_tests::a_date_input_is_the_picker_scoped_to_its_form` | — |
| `render.rs:1515:5: replace date_input -> String with String::new()` | `render::cloud_mutant_tests::a_date_input_is_the_picker_scoped_to_its_form` | — |
| `render.rs:2278:35: replace == with != in store_filter_bar` | `render_rs::render_the_filter_bar_checks_the_kind_the_filter_names` | — |
| `render.rs:2463:25: replace == with != in coverage_table` | `render_rs::render_the_store_page_marks_only_the_feed_it_shows` | — |
| `render.rs:2509:54: replace > with >= in coverage_table` | `render_rs::render_a_cell_counting_no_rows_is_not_a_link_to_them` | — |
| `render.rs:2604:30: replace == with != in store_page` | `render_rs::render_the_store_page_marks_only_the_feed_it_shows` | — |
| `render.rs:2615:52: delete ! in store_page` | `render_rs::render_a_narrowed_held_list_names_the_store_it_narrowed` | — |
| `render.rs:2712:37: replace && with \|\| in bars_page` | `render_rs::render_the_prices_page_tells_held_empty_and_refused_apart` | — |
| `render.rs:2712:40: delete ! in bars_page` | `render_rs::render_the_prices_page_tells_held_empty_and_refused_apart` | — |
| `render.rs:2747:29: replace && with \|\| in bars_page` | `render_rs::render_the_prices_page_tells_held_empty_and_refused_apart` | — |
| `render.rs:3186:5: replace folder_suggestions -> Vec<String> with vec!["xyzzy".into()]` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3186:5: replace folder_suggestions -> Vec<String> with vec![String::new()]` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3186:5: replace folder_suggestions -> Vec<String> with vec![]` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3186:40: replace == with != in folder_suggestions` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3192:5: replace archive_suggestions -> Result<bool, String> with Ok(false)` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3192:5: replace archive_suggestions -> Result<bool, String> with Ok(true)` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3206:5: replace discover_folders -> Vec<String> with vec!["xyzzy".into()]` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3206:5: replace discover_folders -> Vec<String> with vec![String::new()]` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3206:5: replace discover_folders -> Vec<String> with vec![]` | `render::cloud_mutant_tests::folder_suggestions_follow_their_switch_and_walk_home` | — |
| `render.rs:3279:49: replace \|\| with && in collect_csv_dirs` | `render::cloud_mutant_tests::the_folder_walk_offers_only_visible_csv_folders_within_its_depth` | — |
| `render.rs:3288:38: replace && with \|\| in collect_csv_dirs` | `render::cloud_mutant_tests::the_folder_walk_offers_only_visible_csv_folders_within_its_depth` | — |
| `render.rs:3296:36: replace - with + in collect_csv_dirs` | `render::cloud_mutant_tests::the_folder_walk_offers_only_visible_csv_folders_within_its_depth` | — |
| `render.rs:3296:36: replace - with / in collect_csv_dirs` | `render::cloud_mutant_tests::the_folder_walk_offers_only_visible_csv_folders_within_its_depth` | — |
| `render.rs:5536:18: replace match guard (c as u32) < 0x20 with false in json_string` | `render_json_rs::render_json_string_escapes_every_control_character_and_no_space` | — |
| `render.rs:5536:29: replace < with <= in json_string` | `render_json_rs::render_json_string_escapes_every_control_character_and_no_space` | — |
| `render.rs:5536:29: replace < with == in json_string` | `render_json_rs::render_json_string_escapes_every_control_character_and_no_space` | — |

### `bars_change.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `bars_change.rs:226:43: replace < with <= in Rules::judge` | `bars::change::cloud_mutant_tests::a_bar_at_the_ex_dates_midnight_is_already_post_ex` | — |
| `bars_change.rs:280:28: replace > with >= in Rules::oi_unadjusted` | `bars::change::cloud_mutant_tests::an_event_with_no_unscaled_window_labels_no_bar` | — |
| `bars_change.rs:295:66: replace - with + in Rules::halted_between` | `bars::change::cloud_mutant_tests::the_bar_before_the_resume_closes_inside_the_halt` | — |
| `bars_change.rs:295:66: replace - with / in Rules::halted_between` | `bars::change::cloud_mutant_tests::the_bar_before_the_resume_closes_inside_the_halt` | — |
| `bars_change.rs:299:33: replace + with - in Rules::halted_between` | `bars::change::cloud_mutant_tests::two_bars_before_the_halt_began_are_measured` | — |
| `bars_change.rs:299:59: replace * with + in Rules::halted_between` | `bars::change::cloud_mutant_tests::two_bars_before_the_halt_began_are_measured` | — |
| `bars_change.rs:299:59: replace * with / in Rules::halted_between` | `bars::change::cloud_mutant_tests::two_bars_before_the_halt_began_are_measured` | — |
| `bars_change.rs:328:46: replace && with \|\| in Rules::phase_at` | `bars::change::cloud_mutant_tests::a_shares_morning_bar_reads_no_master` | — |

### `bars.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `bars.rs:268:5: replace note_refused with ()` |  | `emitted::every_reachable_emit_site_puts_a_record_in_the_file`, `emitted::the_three_sites_this_binary_cannot_reach_are_named_rather_than_forgotten` |
| `bars.rs:584:34: replace - with + in page_cancellable` | `bars_rs::bars_a_cancellable_page_reads_exactly_its_ask_across_chunks` | — |
| `bars.rs:588:16: replace < with <= in page_cancellable` | `bars_rs::bars_a_cancellable_page_reads_exactly_its_ask_across_chunks` | — |
| `bars.rs:588:16: replace < with == in page_cancellable` | `bars_rs::bars_a_cancellable_page_reads_exactly_its_ask_across_chunks` | — |
| `bars.rs:588:16: replace < with > in page_cancellable` | (times out: the new test's input makes it loop) | — |
| `bars.rs:700:29: replace > with >= in table` | `bars_rs::bars_a_flat_bar_is_unsigned` | — |
| `bars.rs:844:17: replace + with - in SortKey::of` | `bars::cloud_mutant_tests::every_order_reads_its_own_field` | — |
| `bars.rs:977:17: delete field series from struct Extreme expression in Extremes::merge` | `bars::cloud_mutant_tests::a_merged_fold_names_the_series_it_was_merged_under` | — |
| `bars.rs:1196:5: replace retired_refusal -> Option<String> with None` |  | `server::dayflags::tests::a_key_the_rekey_retired_is_refused_by_the_ledgers_sentence`, `server::sources::tests::a_key_the_rekey_retired_is_gone_on_every_route` |
| `bars.rs:1223:31: replace \|\| with && in rekey_status` | `bars_rs::bars_only_a_gdfl_nse_option_is_decoded_for_its_successor` | — |
| `bars.rs:1223:56: replace \|\| with && in rekey_status` | `bars_rs::bars_only_a_gdfl_nse_option_is_decoded_for_its_successor` | — |
| `bars.rs:1523:32: replace \|\| with && in window_cancellable` | `bars_rs::bars_a_window_naming_no_series_or_too_many_is_refused_by_name` | — |
| `bars.rs:1523:54: replace > with == in window_cancellable` | `bars_rs::bars_a_window_naming_no_series_or_too_many_is_refused_by_name`, `bars_rs::bars_exactly_max_series_series_are_read` | — |
| `bars.rs:1523:54: replace > with >= in window_cancellable` | `bars_rs::bars_exactly_max_series_series_are_read` | — |
| `bars.rs:1603:29: replace match guard why.starts_with("cancelled") with true in window_cancellable` | `bars_rs::bars_extremes_a_damaged_month_withholds_leave_the_page_answered` | — |
| `bars.rs:1608:13: delete field column from struct SortReport expression in window_cancellable` | `bars_rs::bars_open_interest_over_an_index_is_time_order_with_its_extremes` | — |
| `bars.rs:1609:13: delete field by_time from struct SortReport expression in window_cancellable` | `bars_rs::bars_open_interest_over_an_index_is_time_order_with_its_extremes` | — |
| `bars.rs:1642:32: replace && with \|\| in window_cancellable` | `bars_rs::bars_a_time_of_day_order_serves_no_extremes_unasked` | — |
| `bars.rs:1642:50: replace == with != in window_cancellable` | `bars_rs::bars_a_time_of_day_order_folds_the_extremes_it_streams`, `bars_rs::bars_a_time_of_day_order_over_an_index_and_a_traded_series` | — |
| `bars.rs:1679:8: delete ! in window_cancellable` | `bars::cloud_mutant_tests::a_window_that_met_an_unreadable_record_says_so_once` | — |
| `bars.rs:1729:31: replace > with < in tally` | `bars_rs::bars_a_hole_over_several_series_names_its_series` | — |
| `bars.rs:1740:41: replace < with <= in tally` | `bars_rs::bars_past_max_named_missing_holes_are_counted_but_not_named` | — |
| `bars.rs:1850:17: replace == with != in behind_of` | `bars_rs::bars_a_row_measures_against_the_row_before_it_in_its_file` | — |
| `bars.rs:1851:31: replace == with != in behind_of` | `bars_rs::bars_a_row_measures_against_the_row_before_it_in_its_file`, `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars.rs:1864:29: replace match guard plan.slot_of(before).is_some() with false in behind_of` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window` | — |
| `bars.rs:2018:14: replace - with / in merged` | `bars_rs::bars_a_merged_page_that_must_rank_a_held_month_is_refused` | — |
| `bars.rs:2018:19: replace > with < in merged` | `bars_rs::bars_a_merged_page_that_must_rank_a_held_month_is_refused` | — |
| `bars.rs:2018:19: replace > with == in merged` | `bars_rs::bars_a_merged_page_that_must_rank_a_held_month_is_refused` | — |
| `bars.rs:2151:22: delete ! in scan` | `bars_rs::bars_a_time_of_day_order_folds_the_extremes_it_streams`, `bars_rs::bars_a_time_of_day_order_over_an_index_and_a_traded_series` | — |
| `bars.rs:2185:29: replace && with \|\| in scan` | `bars_rs::bars_a_time_of_day_order_over_an_index_and_a_traded_series` | — |
| `bars.rs:2197:49: replace - with + in scan` | `bars_rs::bars_a_time_of_day_scan_counts_the_rows_of_each_window` | — |
| `bars.rs:2285:15: replace > with >= in page_of` | `bars::cloud_mutant_tests::a_page_of_every_row_from_the_first_is_only_ordered` | — |
| `bars.rs:2289:14: replace < with <= in page_of` | `bars::cloud_mutant_tests::a_page_of_every_row_from_the_first_is_only_ordered` | — |

### `bars_series.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `bars_series.rs:141:32: replace \|\| with && in SeriesKey::parse` | `bars_series_rs::bars_series_a_key_reads_back_only_when_whole` | — |
| `bars_series.rs:141:54: replace \|\| with && in SeriesKey::parse` | `bars_series_rs::bars_series_a_key_reads_back_only_when_whole` | — |
| `bars_series.rs:171:75: replace > with >= in split_contract` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:190:5: replace contract_shaped -> bool with true` | `bars_series_rs::bars_series_a_key_reads_back_only_when_whole`, `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:192:13: replace && with \|\| in contract_shaped` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:193:13: replace && with \|\| in contract_shaped` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:196:56: replace && with \|\| in contract_shaped` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:196:78: replace && with \|\| in contract_shaped` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:199:27: replace && with \|\| in contract_shaped` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:199:51: replace && with \|\| in contract_shaped` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:201:9: delete match arm [y, m, d, fut] in contract_shaped` | `bars_series_rs::bars_series_a_key_reads_back_only_when_whole` | — |
| `bars_series.rs:201:41: replace && with \|\| in contract_shaped` | `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:201:49: replace == with != in contract_shaped` | `bars_series_rs::bars_series_a_key_reads_back_only_when_whole`, `bars_series_rs::bars_series_a_tail_is_a_contract_only_by_its_shape` | — |
| `bars_series.rs:361:21: replace match guard why.kind() == std::io::ErrorKind::NotFound with true in held_months` | `bars_series_rs::bars_series_a_rung_directory_that_cannot_be_read_is_refused` | — |
| `bars_series.rs:366:15: replace == with != in held_months` | `bars_series_rs::bars_series_a_bound_that_cuts_no_record_opens_no_month`, `bars_series_rs::bars_series_a_month_written_after_the_listing_is_seen` | — |
| `bars_series.rs:496:21: replace match guard why.kind() == std::io::ErrorKind::NotFound with false in header_of` | `bars_series_rs::bars_series_a_gone_month_is_absent_and_an_empty_one_keeps_nothing` | — |
| `bars_series.rs:496:21: replace match guard why.kind() == std::io::ErrorKind::NotFound with true in header_of` | `bars_series_rs::bars_series_a_month_that_will_not_stat_or_be_resolved_is_one_fault` | — |
| `bars_series.rs:496:32: replace == with != in header_of` | `bars_series_rs::bars_series_a_gone_month_is_absent_and_an_empty_one_keeps_nothing`, `bars_series_rs::bars_series_a_month_that_will_not_stat_or_be_resolved_is_one_fault` | — |
| `bars_series.rs:782:25: replace \|\| with && in SeriesPlan::cut_months` | `bars_series_rs::bars_series_a_bound_that_cuts_no_record_opens_no_month` | — |
| `bars_series.rs:788:31: replace > with >= in SeriesPlan::cut_months` | `bars_series_rs::bars_series_a_bound_that_cuts_no_record_opens_no_month` | — |
| `bars_series.rs:798:20: replace > with >= in SeriesPlan::cut_months` | `bars_series_rs::bars_series_an_edge_month_the_bound_empties_is_no_run` | — |
| `bars_series.rs:852:21: delete field slot from struct Run expression in SeriesPlan::cut_days` | `bars_series_rs::bars_series_kept_day_runs_are_read_from_this_requests_slot` | — |
| `bars_series.rs:878:26: replace < with <= in SeriesPlan::day_runs` | `bars_series_rs::bars_series_a_bound_that_cuts_no_record_opens_no_month` | — |
| `bars_series.rs:881:24: replace > with >= in SeriesPlan::day_runs` | `bars_rs::bars_a_row_measures_against_the_row_before_it_in_its_file` | — |
| `bars_series.rs:934:28: delete - in SeriesPlan::bucket_of` | `bars_series_rs::bars_series_an_instant_past_the_calendar_is_after_every_row` | — |
| `bars_series.rs:971:33: replace \|\| with && in SeriesPlan::rank` | `bars_series_rs::bars_series_an_instant_before_its_days_run_is_ranked_unopened` | — |
| `bars_series.rs:988:19: replace > with == in SeriesPlan::rank` | `bars_series_rs::bars_series_at_the_last_record_counts_only_the_rows_before_it` | — |
| `bars_series.rs:988:19: replace > with >= in SeriesPlan::rank` | `bars_series_rs::bars_series_at_the_last_record_counts_only_the_rows_before_it` | — |
| `bars_series.rs:1059:9: replace SeriesPlan::previous_is_kept -> bool with false` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row`, `bars_series_rs::bars_series_without_the_lookback_kept_rows_still_measure` | — |
| `bars_series.rs:1062:19: replace > with < in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_without_the_lookback_kept_rows_still_measure` | — |
| `bars_series.rs:1068:24: replace == with != in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1069:31: replace == with != in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1074:16: replace == with != in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1075:13: replace && with \|\| in SeriesPlan::previous_is_kept` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_without_the_lookback_kept_rows_still_measure` | — |
| `bars_series.rs:1078:55: replace == with != in SeriesPlan::previous_is_kept` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1079:13: replace && with \|\| in SeriesPlan::previous_is_kept` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_without_the_lookback_kept_rows_still_measure` | — |
| `bars_series.rs:1079:28: replace + with * in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1079:28: replace + with - in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1079:32: replace == with != in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1080:13: replace && with \|\| in SeriesPlan::previous_is_kept` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_without_the_lookback_kept_rows_still_measure` | — |
| `bars_series.rs:1083:50: replace == with != in SeriesPlan::previous_is_kept` | `bars_series_rs::bars_series_the_lookback_measures_only_against_a_kept_row` | — |
| `bars_series.rs:1111:9: replace SeriesPlan::slot_of -> Option<usize> with None` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_an_edge_month_the_bound_empties_is_no_run` | — |
| `bars_series.rs:1112:18: replace < with == in SeriesPlan::slot_of` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_an_edge_month_the_bound_empties_is_no_run` | — |
| `bars_series.rs:1112:18: replace < with > in SeriesPlan::slot_of` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_an_edge_month_the_bound_empties_is_no_run` | — |
| `bars_series.rs:1113:26: replace + with * in SeriesPlan::slot_of` | (times out: the new test's input makes it loop) | — |
| `bars_series.rs:1113:32: replace - with + in SeriesPlan::slot_of` | (times out: the new test's input makes it loop) | — |
| `bars_series.rs:1115:17: delete match arm Some(std::cmp::Ordering::Less) in SeriesPlan::slot_of` | `bars_series_rs::bars_series_without_the_lookback_kept_rows_still_measure` | — |
| `bars_series.rs:1115:60: replace + with - in SeriesPlan::slot_of` | (times out: the new test's input makes it loop) | — |
| `bars_series.rs:1116:17: delete match arm Some(std::cmp::Ordering::Equal) in SeriesPlan::slot_of` | `bars_rs::bars_without_lookback_a_month_after_an_unkept_row_is_first_in_window`, `bars_series_rs::bars_series_an_edge_month_the_bound_empties_is_no_run` | — |
| `bars_series.rs:1245:25: replace match guard at.slot == slot with true in SeriesPlan::note_unread` | `bars_series_rs::bars_series_two_months_that_will_not_open_are_two_faults` | — |
| `bars_series.rs:1358:19: replace > with >= in SeriesPlan::days_covered` | `bars_series_rs::bars_series_a_month_covers_only_the_days_that_hold_a_record` | — |

### `bars_sort.rs`

| Survivor | Killed by a new test | Also killed by an existing test outside the filter |
|---|---|---|
| `bars_sort.rs:1584:42: replace - with / in keep_best` | `bars::sort::cloud_mutant_tests::a_read_month_in_any_order_pages_as_the_brute_force` | — |
| `bars_sort.rs:1663:28: replace < with <= in cut_entries` | `bars::sort::cloud_mutant_tests::a_day_bounds_end_is_exclusive_in_a_cut_runs_entries` | — |
| `bars_sort.rs:1727:19: replace - with + in cut_extremes` | `bars::sort::cloud_mutant_tests::a_day_cut_pages_extremes_are_folded_from_the_kept_rows`, `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:1727:19: replace - with / in cut_extremes` | `bars::sort::cloud_mutant_tests::a_day_cut_pages_extremes_are_folded_from_the_kept_rows`, `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:1917:9: delete field column from struct SortReport expression in page_once` | `bars::sort::cloud_mutant_tests::a_page_from_cut_runs_alone_is_the_brute_force` | — |
| `bars_sort.rs:1932:41: replace += with *= in page_once` | `bars::sort::cloud_mutant_tests::a_page_from_cut_runs_alone_is_the_brute_force` | — |
| `bars_sort.rs:1951:49: replace == with != in page_once` | `bars::sort::cloud_mutant_tests::at_most_four_build_commands_are_offered`, `bars::sort::cloud_mutant_tests::under_a_time_of_day_bound_each_month_is_named_once_with_its_rows` | — |
| `bars_sort.rs:1953:57: replace - with + in page_once` | `bars::sort::cloud_mutant_tests::under_a_time_of_day_bound_each_month_is_named_once_with_its_rows` | — |
| `bars_sort.rs:1953:57: replace - with / in page_once` | `bars::sort::cloud_mutant_tests::under_a_time_of_day_bound_each_month_is_named_once_with_its_rows` | — |
| `bars_sort.rs:1957:37: replace - with + in page_once` | `bars::sort::cloud_mutant_tests::under_a_time_of_day_bound_each_month_is_named_once_with_its_rows` | — |
| `bars_sort.rs:1984:35: replace < with <= in page_once` | `bars::sort::cloud_mutant_tests::at_most_four_build_commands_are_offered` | — |
| `bars_sort.rs:2015:41: replace && with \|\| in page_once` | `bars::sort::cloud_mutant_tests::a_mixed_page_is_served_to_the_sort_depth_and_refused_past_it` | — |
| `bars_sort.rs:2022:29: replace > with >= in page_once` | `bars::sort::cloud_mutant_tests::a_mixed_page_is_served_to_the_sort_depth_and_refused_past_it` | — |
| `bars_sort.rs:2063:22: delete ! in page_once` | `bars::sort::cloud_mutant_tests::a_page_of_read_rows_kept_while_they_are_read_is_the_brute_force` | — |
| `bars_sort.rs:2195:25: delete field max from struct ColumnInfo expression in page_once` | `bars::sort::cloud_mutant_tests::a_page_from_cut_runs_alone_is_the_brute_force` | — |
| `bars_sort.rs:2278:48: delete ! in page_once` | `bars::sort::cloud_mutant_tests::a_day_cut_pages_extremes_are_folded_from_the_kept_rows` | — |
| `bars_sort.rs:2519:36: replace == with != in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2519:41: replace && with \|\| in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2519:48: replace == with != in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2544:40: delete ! in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2551:37: replace == with != in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2551:42: replace && with \|\| in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2551:49: replace == with != in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2562:18: replace += with *= in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2562:18: replace += with -= in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2571:14: replace += with *= in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2572:19: replace == with != in extremes_of` | `bars::sort::cloud_mutant_tests::a_time_ordered_pages_extremes_hold_only_a_whole_months_fold` | — |
| `bars_sort.rs:2599:40: replace < with <= in unread_of` | `bars::sort::cloud_mutant_tests::at_most_twelve_unread_months_are_named` | — |
| `bars_sort.rs:298:9: replace Weighed<K, V>::get -> Option<V> with None` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:304:19: replace > with < in Weighed<K, V>::put` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:304:19: replace > with == in Weighed<K, V>::put` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:304:19: replace > with >= in Weighed<K, V>::put` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:304:9: replace Weighed<K, V>::put with ()` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:308:42: replace > with < in Weighed<K, V>::put` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:308:42: replace > with == in Weighed<K, V>::put` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:308:42: replace > with >= in Weighed<K, V>::put` | `bars::sort::cloud_mutant_tests::the_weighed_memo_holds_within_its_budget` | — |
| `bars_sort.rs:406:36: replace == with != in view` | `bars::sort::cloud_mutant_tests::a_host_refusal_of_an_index_is_this_requests_alone` | — |
| `bars_sort.rs:406:54: replace == with != in view` | `bars::sort::cloud_mutant_tests::a_host_refusal_of_an_index_is_this_requests_alone` | — |
| `bars_sort.rs:447:31: replace \|\| with && in view` | `bars::sort::cloud_mutant_tests::a_host_refusal_of_an_index_is_this_requests_alone` | — |
| `bars_sort.rs:449:8: delete ! in view` | `bars::sort::cloud_mutant_tests::a_host_refusal_of_an_index_is_this_requests_alone` | — |
| `bars_sort.rs:937:16: replace += with *= in Arrays::more_than` | `bars::sort::cloud_mutant_tests::a_page_over_read_rows_and_a_wide_run_is_the_brute_force` | — |

## Equivalent survivors, and why no test can kill them (60)

Each was read against the code, and the path a test would need was traced. "Cost only" means the mutant changes how much is read, held or compared, never the answer. No test here measures cost.

| Mutant | Why no test can kill it |
|---|---|
| `ladder.rs:686:32` `<` → `<=` in `owed_runs` | `lo` takes an equal month: the same value. |
| `ladder.rs:689:32` `>` → `>=` in `owed_runs` | `hi` takes an equal month: the same value. |
| `ladder.rs:703:38` `<` → `<=` in `owed_runs` | The month equal to `lo` is held (`lo` is the first held month), so the `held` probe two lines on skips it either way. |
| `ladder.rs:704:38` `>` → `>=` in `owed_runs` | The month equal to `hi` is held, so it is skipped by the same probe before its reason is read. |
| `render.rs:2414:22` `>` → `>=` in `page_peak` | `peak` takes an equal value: the same value. |
| `bars_change.rs:301:59` `<` → `<=` in `Rules::halted_between` | `close(before)` is a whole-second stamp plus a whole-second rung less 1 µs; `resume` is a whole second. They are never equal. |
| `bars_change.rs:333:34` `*` → `+` in `Rules::phase_at` | The test becomes `rung < 1.00006 s` in place of `rung < 60 s`. `Timeframe::from_secs` admits 1 s, then 1 min: no rung lies between, so both tests pick the same rungs. |
| `bars_change.rs:432:21` `-` → `/` in `session_before` | The walk's floor becomes `day / 10` in place of `day − 10`. The two differ only after ten closed days in a row, and `pull::calendar` has no such run in its sourced span; outside it the walk meets an unmeasured day and stops at the same `None`. |
| `bars_change.rs:473:49` guard → `false` in `Lineage::read` | The memo is only a cache: the ledger is read again and parses to the same events. Cost only. |
| `bars.rs:174:49` `>` → `>=` in `rupees` | At `from_end == 3` the first disjunct is already true. |
| `bars.rs:174:66` `-` → `+` in `rupees` | `from_end + 3` and `from_end − 3` have the same parity. |
| `bars.rs:939:66` `<` → `<=` in `Extreme::beats` | Equal `(t, series)` is the same bar: an instant holds one row per series, so the replacement is the identical extreme. |
| `bars.rs:1989:41` `<` → `==`, `>`, `<=` in `merged` (3) | Only the bisection's step changes (1 µs or 1 s). The merge pops rows in its own order and skips to `a`, so the page is the same; only the number of steps moves. Cost only. |
| `bars.rs:2032:47` `<` → `<=` in `merged` | `rank` returns `count == total` only as the position past the last run, where `head` answers `None` anyway. |
| `bars.rs:2185:43` `\|\|` → `&&`, `bars.rs:2185:64` `==` → `!=` in `scan` | `scan` runs only for `SortKey::TimeOfDay` (`OpenInterest` takes its own path), so `matches!(query.sort, OpenInterest)` is false and the right operand is never read. |
| `bars_series.rs:978:42` (974:42 before the fix) `+` → `*` in `SeriesPlan::rank` | Past a run's end, `rank` answers the right count and the WRONG run. `at_index` and the merge's bisection read only the count. The merge's last call, the one that reads the position, ranks an instant at or before the page's first row, so it is inside or before its bucket's run in every series (one filter gives every series the same run ends) and never takes this branch. |
| `bars_series.rs:1115:60` (1111:60 before the fix) `+` → `*` in `SeriesPlan::slot_of` | On "less" the search keeps `lo = mid` instead of `mid + 1`. A month the slots hold is still found, because with `hi = lo + 1` the month at `lo` is the one sought. A month before every slot meets only "greater" steps. Same slot, more steps. |
| `bars_series.rs:1258:9` (1254:9 before the fix) `SeriesPlan::clear_unread` → `()` | Every reader drains the unread ledger with `take_unread` before it answers (`scan` per series, `page_once` through `unread_of`, `extremes_of`). A sort page is asked again only when an index fails inside `select`, after that drain, so the ledger is already empty when the clear would run. (`a_page_asked_again_counts_its_unread_rows_once` pins the count across a retry and does not kill this mutant, as its own comment says.) |
| `bars_sort.rs:132` and `:137` `*` → `+`, `/` (8 mutants, the two budget constants) | The budget decides only how much is memoised. A fence table or cut-entry list not held is read again from the same bytes and decodes to the same value, so every answer is the same; only cost and memory move. (The memo's own contract is tested directly by `the_weighed_memo_holds_within_its_budget`.) |
| `bars_sort.rs:384:21` guard → `true` in `view` | It differs only for a `symlink_metadata` error other than not-found. The `.srt` is the bars file's sibling with a name of the same length, and the bars file has already been opened, so a search-permission, link-loop or name-length error cannot reach the `.srt` alone. The arm after it (line 387) is reached only if the directory changes between that open and this stat, during one request (see *Coverage*, class B). |
| `bars_sort.rs:406:44` `&&` → `\|\|` in `view` | The set-aside list matches a `.srt` by path AND stamp. Two different files never share a stamp (it holds the inode), and the same path under a new stamp needs the file replaced during the one request. |
| `bars_sort.rs:563:49` `\|\|` → `&&` in `cut` | No index run lies both wholly before and wholly after the kept run, so the skip never fires. But a run outside the kept run, let through, gets `from ≥ until` in the cut branch, and nothing is pushed: the same pieces. |
| `bars_sort.rs:584:38` `>` → `>=` in `cut` | The index's runs chain from row 0 with no gap (the walk refuses one), so `at` is the cut run's own start or the kept run's start, and an overlapping run's cut is never empty: `until == from` does not occur. |
| `bars_sort.rs:779:26` `<` → `>`, `bars_sort.rs:780:26` `>` → `<`, `bars_sort.rs:780:50` `+` → `*` in `RunArray::bounds` | The fence bound is replaced by the whole `[lo, hi)`. `partition` then binary-searches a wider range for the first entry where a monotone predicate holds, and finds the same entry; only more entries, and so blocks, are read. Cost only (the module's "one block at most" is a cost bound no test here measures). |
| `bars_sort.rs:822:36` `-` → `+` in `RunArray::count_vkey_le` | It changes only the upper bound of a descending run's fence-bounded count, `valued − a` → `valued + a`, which is still an upper bound, only looser. `more_than` reads it only for the shortcut `ub <= k → false`, which then fires less often, and the exact count decides instead. The exact count (`.0`, `valued − b`) is untouched. In `more_than`'s `a == b` test the mutant's pair is equal only when `a = b = 0`, where `valued − b` is the exact count anyway. Cost only. |
| `bars_sort.rs:894:25` `>` → `>=` in `Arrays::domain` | An all-null run also offers its column ends to the value domain; they are `0`, which (every stored value being at or above `0`) only widens the domain, and the bisection finds the same value. |
| `bars_sort.rs:962:25` `>` → `>=` in `Arrays::count_le` | An all-null run is counted too, and its valued range `[nulls, rows)` is empty: `partition` answers its start without reading, so it adds 0. |
| `bars_sort.rs:1103:27` `\|\|` → `&&` in `Arrays::select_in` | The skip never fires, but a slice whose `[first, last]` does not hold the instant counts the same rows at it and one microsecond before it, so it adds no row. |
| `bars_sort.rs:1299:19` `>` → `>=` in `Arrays::value_slices`; `bars_sort.rs:1349:25`, `:1361:28` `>` → `>=` in `Arrays::null_slices` | Each lets an empty slice into the list, and `select_in` passes over a slice of length 0 before reading it. |
| `bars_sort.rs:1325:16` `>` → `>=` in `Arrays::value_slices`; `bars_sort.rs:1336:30` `>` → `>=` in `Arrays::null_slices` | The same as `:1299:19`: each lets an empty slice into the list, and `select_in` passes over a slice of length 0 before reading it. |
| `bars_sort.rs:1466:36` `>` → `>=` in `cursors_at` | An all-null run (`rows == nulls`) also gets a valued cursor. Ascending, `partition` over `[nulls, rows)` is over an empty range and answers `rows`, so `Up { p: rows }`, and `head` answers `None` at `p >= rows`. Descending, `group` answers `g0 = g1 = rows`, the cursor steps once, `advance` finds `g0 <= nulls` and parks it at `g1`, and `head` answers `None`. No row is added. |
| `bars_sort.rs:1495:26` `>` → `>=` in `cursors_at` | A run with no nulls also gets a null cursor at `p = 0` over `[0, 0)`. `head` answers `None` for `p >= nulls` before reading. |
| `bars_sort.rs:1553:53` `<` → `<=` in `merge` | The keys are unique (an instant holds one row per series, and the key carries both), so two heads never compare equal. |
| `bars_sort.rs:1583:13` `>` → `>=`, `:1583:31` `>` → `>=` in `keep_best` (2) | `keep` is never `0` (it is `.max(1)`), so `keep >= 0` and `keep > 0` are both always true. At `len == keep` the cut keeps every row and the returned bar is ignored (the last call's result is dropped; the call inside the read is made only at `len == capacity`, which is `2 × keep` there). |
| `bars_sort.rs:1583:5` `keep_best` → `None`; `:1583:13` `>` → `<`, `==`; `:1583:31` `>` → `==` in `keep_best` (4) | Each stops the read rows from being cut. `kept` then grows past its room (a `Vec` reallocates), no row is turned away, and every read row is kept in order. The page is chosen from exact counts, and from the end `before` is `0` and the skip is `offset`: the same rows. Memory and cost only. |
| `bars_sort.rs:1595:14` `<` → `<=`, `:1598:56` `+` → `*` in `run_of_slot` | `run_of_slot` is asked only for the slot of a `Found::Run`, which a run of that slot produced, so the slot is always in the list. The search returns on `Equal` before `lo == hi`. On "less" with `lo = mid` the range still holds the slot, and at `hi = lo + 1` the slot is at `lo`. Same answer. |
| `bars_sort.rs:2020:18` `>` → `>=` in `page_once` | At `depth == MAX_SORT_DEPTH` exactly, with every kept row read and more than `read_held()` of them, the page is served from the end instead of the front. Both are exact: the front keeps the `depth` best rows, and the end keeps the `total − offset` last with `before = joined − kept`. Both answer the rows at `[offset, offset + limit)` (unread rows included in `total` shift both alike). |
| `bars_sort.rs:2193:25` delete field `nulls` from `ColumnInfo` in `page_once` | A cut run's null count is carried separately (the array's own `nulls`); only the whole-run path reads `ColumnInfo::nulls`. Nothing reads the deleted field. |
| `bars_sort.rs:2194:25` delete field `base` from `ColumnInfo` in `page_once` | `0` becomes the cut run's lowest value, which only widens the value domain downward: the store refuses a negative price, volume or open interest at its write boundary (`ohlc_is_sane`, `bad_counts`), so every real value is at or above `0`, and the bisection over a wider domain finds the same value. |
| `bars_sort.rs:2571:22` `-` → `+` in `extremes_of` | `whole` means `start == 0`, so `end + start == end − start`. |

**Two whole-suite "kills" that are not kills.** `bars_change.rs:473:49` and
`bars.rs:939:66` each failed one test in a first whole-suite run:
`coverage::tests::the_lookup_does_not_grow_with_the_universe`. That test
times `Coverage::of` against two universes, and it reads neither the lineage
ledger nor an extreme. It failed the same way on an UNMUTATED tree under the
same load, during an interim coverage run. A timing test that fails at four
times oversubscription is measuring the machine, not the mutant. Both
mutants were run again against the final suite, which skips that one test,
and both survived. They are counted as equivalent, not as killed.

## Survivors that are NOT equivalent, and why no test was written (18)

Each can change behaviour, and no test here pins it. There are two kinds.

**Cancellation cadence (5).** These differ only when another thread sets the
cancel flag *while* a loop runs. A flag set before the call is caught by the
check every path makes first, and tests pin that.

**Budgets only a huge store reaches (13).** These differ only past
`MAX_SCAN_BARS` (24,000,000 rows read) or `MAX_EXTREMES_SCAN_BARS`
(64,000,000 rows owed) in one request. Both are `pub const` in `bars.rs`.
Unlike `READ_HELD`, no test can lower them. Lowering one would be a
production change, which this pass does not make.

| Mutant | What it breaks | Why it has no test |
|---|---|---|
| `spot.rs:535:19` `%` → `/` in `SpotAsk::answer_with` | A one-instant-at-a-time spot read checks the flag at every index of its first stride instead of every `CANCEL_EVERY`-th. | A flag set before the call is caught at index 0 by both, and `a_cancelled_spot_read_of_one_instant_is_refused_at_its_first` pins that. Only a flag set mid-loop by another thread tells them apart, which is not a deterministic test. |
| `bars.rs:1603:29`, `bars.rs:1674:29` match guard `why.starts_with("cancelled")` → `false` in `window_cancellable` (2) | A client that goes away while the extremes are folded gets a page with `extremes_why = "cancelled…"` instead of the request ending as cancelled. | The request checks the flag just before the page is read, and nothing between that check and the fold can set it. Only another thread can, mid-fold. (The guard's `true` mutant is killed by `bars_extremes_a_damaged_month_withholds_leave_the_page_answered`.) |
| `bars_series.rs:1222:20` `+=` → `*=`, `bars_series.rs:1223:23` `>=` → `<` in `SeriesPlan::read_stretch` (2) | How often a stretch read checks the flag: never, or every row, instead of every `CANCEL_EVERY` rows. | The same as `spot.rs:535:19`. A flag set before the read is caught by the check every run makes before it starts. |
| `bars_sort.rs:1996:18` `>` → `==`, `>=` in `page_once` (2) | The `scan_budget_exceeded` refusal past 24 M read rows: refused at exactly 24 M (`>=`), or served above it (`==`). | A test would plant and read about 24 M bars, roughly 0.9 GB of fixture, in one request. |
| `bars_sort.rs:2259:45` `delete !`, `:2261:52` `-` → `+`, `/`, `:2265:17` `>` → `==`, `>=` in `page_once`; `bars_sort.rs:2523:39` `&&` → `\|\|`, `:2526:48` `-` → `+`, `:2531:13` `>` → `>=`, `==` in `extremes_of` (9) | The rows the extremes owe, and the refusal past 64 M. | A test would need more than 64 M rows owed in one request. |
| `bars_sort.rs:1680:5` `cut_extremes_held` → `true`, `false` (2) | Every cut stretch counts as already folded, or none does. | It is read in two places. `page_once` reads it only to sum `owed` for the 64 M check. `extremes_of` reads it only to skip that same sum, and pushes the stretch to the read list either way, where `cut_extremes` asks its memo itself. Below 64 M the answer is the same. |

## Real bugs found

**ONE, fixed: an `at=` instant past the year 9999 was ranked before every row.**

- **Where.** `crates/api/src/bars_series.rs`, `SeriesPlan::bucket_of`, commit
  `7540002`.
- **What was wrong.** `bucket_of` answered `-1`, meaning "before the first
  month", for every day it could not name. That covered a day before the
  epoch, and also a day past `pull::session::MAX_YEAR` (9999), which
  `Day::from_days` refuses.
- **How a request reaches it.** `/bars/window.json` takes `at=` as any whole
  number of unix seconds that fits in microseconds (`bars_route::instant`),
  so `at=253402300800` (10000-01-01) gets there.
- **What it did.** That instant was ranked before every row. Ascending, its
  index was 0 instead of the total. Descending, it was the total instead of 0.
  The page served for `at=` was the wrong end of the window.
- **The fix**, three lines at the cause: a day before the epoch is still
  `-1`, and a day past the calendar is `i64::MAX`, after every month. `rank`
  already answers any bucket at or past the last as "past every row".
- **The test.** `bars_series_rs::bars_series_an_instant_past_the_calendar_is_after_every_row`
  (in `tests/cloud_mutants_bars.rs`) failed before the fix with
  `left: Some(0), right: Some(2)` and passes after it.
- **How it was found.** The surviving mutant `bars_series.rs:932:29 delete -
  in SeriesPlan::bucket_of` turned the `-1` into `1` and no test noticed. Asking
  what the `-1` meant showed it meant two different things.

**Seen, outside this pass's scope, not changed.** `Contract::parse`
(`crates/core`) checks a contract's characters and length and nothing else:
it accepts a month of `13`. One fixture here first used such a contract to get
an unparsable tail, and was changed to an over-long strike instead. Whether
`core` should refuse it is for a pass that owns `core`.

## Coverage, before → after

`cargo llvm-cov -p api --lcov`, with the checked-in profile, in the main
repository. "Before" is the base `c9e3472e` and "after" is `6e53f60`, the last commit that changes code.
Lines are **production lines only**: lcov lines outside every top-level
`#[cfg(test)] mod … { … }` of the file. A new test module would otherwise
raise a file's percentage by covering itself.

Two after-figures are given, because the base figure could only be taken one
way:

- **as root, like the baseline.** Measured exactly as the base was: as
  `root`, with the ten permission tests skipped. The children that
  `isolated::where_permission_binds` runs as uid 65534 cannot write their
  profiles into root's directory.
- **full.** The same run, plus those children's profiles (the profile
  directory made writable and the suite run again), plus the ten
  permission tests run as uid 65534.

| File | Production lines | Before | After, as root | After, full | Uncovered, full |
|---|---:|---:|---:|---:|---:|
| `spot.rs` | 324 | 311 (95.99 %) | 324 (100.00 %) | 324 (**100.00 %**) | 0 |
| `render.rs` | 1,240 | 1,217 (98.15 %) | 1,238 (99.84 %) | 1,239 (**99.92 %**) | 1 |
| `bars_change.rs` | 347 | 329 (94.81 %) | 346 (99.71 %) | 346 (**99.71 %**) | 1 |
| `ladder.rs` | 249 | 245 (98.39 %) | 248 (99.60 %) | 248 (**99.60 %**) | 1 |
| `gaps_route.rs` | 1,039 | 964 (92.78 %) | 1,026 (98.75 %) | 1,026 (**98.75 %**) | 13 |
| `bars.rs` | 1,154 | 1,095 (94.89 %) | 1,136 (98.44 %) | 1,136 (**98.44 %**) | 18 |
| `bars_series.rs` | 753 → 751 ¹ | 702 (93.23 %) | 738 (98.27 %) | 738 (**98.27 %**) | 13 |
| `bars_sort.rs` | 1,686 | 1,582 (93.83 %) | 1,607 (95.31 %) | 1,613 (**95.67 %**) | 73 |
| **eight files** | 6,792 → 6,790 | 6,445 (94.89 %) | 6,663 (98.13 %) | 6,670 (**98.23 %**) | **120** |

¹ The `bars_series.rs` fix changed the instrumented lines of `bucket_of`.

**Branch coverage was not measured.** `cargo llvm-cov --branch` needs a
nightly toolchain, and this repository pins stable `1.97.1`.

**100 % line coverage is not reached on seven of the eight files.** The 120
production lines still uncovered are listed below, every one with its
reason. The list was checked against the measurement: the uncovered lines in
`after-full.lcov` and the lines classified below are the same 120, with no
difference either way.

### Every production line still uncovered, and why

Each line below was read in place, and the path a test would need was
traced to its caller. There are four classes.

**A. Defensive arms no caller can reach (77 lines).** Each is an index or a
lookup taken inside its own loop's bounds, a conversion that cannot fail
(`usize` → `u64`, a series count under `MAX_SERIES` → `u32`), or a guard
whose only caller has already decided it.

| File | Lines | Why no input reaches them |
|---|---|---|
| `bars.rs` | 653 | `u64::try_from(usize)` cannot fail on a 64-bit target. |
| `bars.rs` | 1847, 1856, 1899, 1923, 1926, 2051, 2148, 2155, 2159, 2206, 2209, 2243 | Lookups by an index the same function took from the list it now indexes (a run of the plan, a slot of a run, a series of the request), and `u32` conversions of counts bounded by `MAX_SERIES`. |
| `bars.rs` | 2048 | The merge pops `want` rows from heads that hold at least `[a, b)` of the total, so `best` is never `None` before `out` is full. |
| `bars.rs` | 2185 | The open-interest operand inside `scan`. `scan` runs only for `SortKey::TimeOfDay`, since open interest takes its own path. This is the same reason as the equivalent mutants `2185:43` and `2185:64`. |
| `bars_change.rs` | 439 | `session_before`'s answer after ten closed days in a row. The sourced calendar has no such run (the same reason as the equivalent `432:21`). |
| `bars_series.rs` | 202 | `contract_shaped`'s `_` arm. Its only caller hands it exactly 5 or 4 fields. |
| `bars_series.rs` | 668 | A window of no months. `months_of` never returns an empty range. |
| `bars_series.rs` | 740 | `StorePath::new` refusing a month in `admit`. `build` has already addressed the same key for the first month, and refused by name if it could not. Only the month differs, and `YearMonth` is valid by type. |
| `bars_series.rs` | 772, 822 | `cut_months` under day buckets and `cut_days` under month buckets. `build` calls each only under its own kind. |
| `bars_series.rs` | 776, 829, 991, 1060, 1340 | Slot and run lookups inside their own bounds. |
| `bars_series.rs` | 1018, 1213 | `nth` past the total and an empty stretch read. No caller asks either. |
| `bars_sort.rs` | 364, 520, 1293, 1463, 1570, 1603, 1962, 2061, 2066, 2076, 2136, 2211, 2287, 2365, 2368, 2391, 2394–2395, 2440, 2514, 2546 | Lookups and conversions inside their own bounds. `run_of_slot` (1603, 2394–2395) is asked only for a slot a run produced. |
| `bars_sort.rs` | 538 | A view already walked for a `(series, slot)` in this request. A month is one kept run of a series without a time-of-day bound, and `cut` returns before any view under one. |
| `bars_sort.rs` | 1056–1057, 1066, 1072, 1101 | Empty slices in `select_in`. `value_slices` and `null_slices` never push one, and the sum check at 1049 returns before 1066 could. |
| `bars_sort.rs` | 1403 | No value domain while `k < valued`. A valued row gives the domain an end. |
| `bars_sort.rs` | 1855–1857 | `page_once` without a value column. `window_cancellable` calls it only after `column_of` answered one. |
| `bars_sort.rs` | 2163, 2166, 2169, 2207 | An indexed or cut piece whose view, walk or run is missing, or a non-null series' cut run left unloaded. `cut` makes those pieces only from a walked view, and the loader turns an unserved cut into a read. |
| `gaps_route.rs` | 431 | `next_month` past 9999. The loop returns at the request's last month first. |
| `gaps_route.rs` | 1253, 1260–1261, 1264–1266, 1269–1272 | `audit_one`'s answer for a month whose first day the calendar cannot name. `pull::session::Day` and `YearMonth` share the range `1970..=9999`, so every month has one. |
| `gaps_route.rs` | 1300, 1312 | A second-to-minute fold that fails. `fold_from_bars` refuses only a narrower bucket than its source, and minutes are wider than seconds, or a stamp within a bucket of the `i64` ends, which no stored month holds. |
| `ladder.rs` | 464 | `Day::from_days` past 9999-12-31. The loop has returned at the window's last day. |

**B. Cancellation, or the store changing, in the middle of one request
(17 lines).** A cancel flag set before a request is caught by its first
check, and tests pin that. These arms run only when another thread cancels,
or another process removes or locks a file, between two steps of the same
request. No deterministic test reaches them (the same class as the
cancellation-cadence mutants above).

| File | Lines | The step that fails between |
|---|---|---|
| `bars.rs` | 1639, 2196 | A time-of-day scan cancelled mid-read. |
| `bars.rs` | 2221 | A row the scan just read, refused when it is read again for the page. |
| `bars_series.rs` | 512 | A file that passed `stat` and is gone at `open`. |
| `bars_sort.rs` | 387 | A `.srt` `stat` error other than not-found, after its bars file, in the same directory, opened. |
| `bars_sort.rs` | 1833–1835 | `From<String> for Attempt`, reached only by a `?` on a cancellation inside `page_once`. |
| `bars_sort.rs` | 2297, 2561 | A cut stretch's extremes fold cancelled mid-read (on a value page, and on a time-ordered one). |
| `bars_sort.rs` | 2372–2374, 2399–2401 | A row the request read or bound, refused when it is read or located again. |
| `render.rs` | 3284 | `DirEntry::file_type` failing for an entry just listed. On Linux that takes the entry vanishing before the `lstat`. |

**C. The 24 M and 64 M budgets (23 lines).** These are `bars_sort.rs`
1997–2005 (the `scan_budget_exceeded` refusal past `MAX_SCAN_BARS`),
2266–2273 and 2532–2537 (past `MAX_EXTREMES_SCAN_BARS`). They are the lines
of the budget mutants under *NOT DONE*, for the same reason.

**D. An index that disagrees with itself after passing its checksums
(3 lines).** `bars_sort.rs` 1424–1426: the value bisection found a value
that no row of any array holds. A block or fence that fails its checksum is
refused earlier, by name (`past_max_broken_failing_indexes_the_page_is_refused`).
A fixture that passes every checksum and still disagrees would need a
hand-built sort-index file whose fences and blocks contradict each other.
The planting helper builds indexes only from rows, so no test here makes one.


## Gates

Run in the main repository at `6e53f60`, the last commit that changes code, with the checked-in profile
and nothing else running on the machine. The full log is not tracked.

| Gate | Command | Result |
|---|---|---|
| format | `cargo fmt --all --check` | **pass** (exit 0) |
| lint | `cargo clippy -p api --all-targets -- -D warnings` | **pass** (exit 0) |
| tests | `cargo test -p api --locked` (run with `--no-fail-fast`, so every binary reports) | **10 failures, all expected as root** (exit 101). Library: 2,022 passed, 10 failed, 5 ignored. `src/main.rs`: 3 passed. `tests/binary.rs`: 4 passed. `tests/cloud_mutants_bars.rs`: 58 passed. Doc-tests: 1 passed. |
| the 10 as uid 65534 | the same library test binary, `setpriv --reuid=65534 --regid=65534 --clear-groups`, the 10 names, `--test-threads=1` | **pass**: 10 passed, 0 failed (exit 0) |

**The 10 library failures are exactly the ten permission tests named under
*Method*, and nothing else fails.** Each needs mode bits that bind the process
running it, and this session runs as `root`, which no mode bit binds. Run as
uid 65534, all ten pass. With them counted where they pass, the `api` suite
is green: 2,032 library tests pass, plus 3 + 4 + 58 + 1. The gate as worded
exits 101 here, and only because the session is `root`.


## NOT DONE

1. **18 survivors that are not equivalent have no test** (the table above):
   5 differ only when another thread cancels mid-loop, and 13 only past a
   24 M or 64 M row budget that no test can lower without a production
   change.
2. **Line coverage is not 100 % on the eight files.** The lines still
   uncovered are listed and classified under *Coverage*.
3. **Branch coverage was not measured.** `cargo llvm-cov --branch` needs a
   nightly toolchain, and `rust-toolchain.toml` pins stable `1.97.1`.
   Nothing here claims a branch figure.
4. **`cargo test -p api --locked` exits 101 as `root`**, on the ten permission
   tests only. They pass as uid 65534 (*Gates*). The gate was not run as an
   unprivileged user end to end.
5. **One timing test is load-sensitive.**
   `coverage::tests::the_lookup_does_not_grow_with_the_universe` failed on
   unmodified code under four parallel mutation runs. It passes in the gate
   run, which had the machine to itself. It is skipped only in the final
   mutation suite, and the two mutants it seemed to kill are counted as
   equivalent (*Equivalent survivors*).
6. **The PR's CI is red for reasons outside this diff**, each explained on
   the PR, and none is fixed here:
   - Gate W: `web/build` is stale on the base.
   - Gate 1+2: `core`'s `banned_lists` reads a workflow the base did not
     carry. In one run it also stopped at an intermittent `cli` test that
     passes alone.
   - `ci-ok` aggregates both.
7. **`CLAUDE.md` §9 items outside the task's three gates were not run or
   written.** Those are `cargo deny check`, the workspace-wide clippy and
   tests, mutants outside the eight files, a `docs/04-invariants.md` row and
   a `docs/05-decisions.md` entry for the `bars_series.rs` fix. The task
   forbids editing any file outside `crates/api` except new test files. The
   fix's test is its proof here, and the documents are for the change's
   owner.
8. **`dayflags.rs` was not mutated or tested**, as the task requires.
9. **Two slips, with no effect on the repository.** Twice in the session a
   `python3` command was issued by mistake, which `CLAUDE.md` §2 forbids as a
   tool. The first hung on standard input and was stopped before it read
   anything. The second printed the interpreter's version. No script was run,
   and nothing tracked comes from either.
