# Cloud mutation and coverage pass — `crates/pull`, 2026-10-16

Branch `cloud/mut-pull-20261016`, cut from `cloud/base-20261016` at `c9e3472`
(a snapshot of local `feat/db-fix` at `4756cce`). Draft PR SJParthi/brutex#75.

**Scope, exactly:** `cash_session_cache.rs`, `fold.rs`, `gdfl_archive.rs`,
`gdfl_cm.rs`, `gdfl_cm_auction.rs`, `gdfl_import_scope.rs`, `gdfl_nfo.rs`,
`gdfl_tickstore.rs`, `request_minutes.rs`, `scrub.rs`.
**Not touched:** `calendar.rs`, `gdfl_import.rs`, `alerts.rs`,
`gdfl_import_quality.rs`, every ruzstd call site (raw zstd frames in the new
tests are built by hand; no new ruzstd call), `.github/workflows/`.

**No production line changed.** `git diff c9e3472 -- crates/pull/src` adds
only `#[cfg(test)] mod cloud_mutant_tests` blocks at the end of in-scope
files; public behaviour is tested in the new
`crates/pull/tests/cloud_mutants_import.rs`. Every fixture is synthetic:
invented tickers (`ZZNOISIN`, `Z&Z`, `M&M` as a name shape), prices and
stamps; no vendor byte is committed.

## Method

| | |
|---|---|
| Tools | cargo-mutants 27.1.0 (CI pins 26.2.0), cargo-llvm-cov 0.9.1, rustc/cargo 1.97.1 |
| Mutant build | `--cap-lints true --cargo-arg=--lib --baseline skip`, dev profile overridden to opt-level 0, incremental on, no debuginfo (the workspace dev profile is opt-level 3 and was too slow to rebuild per mutant); `--jobs 2`, per-mutant test timeout 300 s |
| Phase 1 | every mutant of each file, in shards (`--shard k/n`, each shard time-boxed to 25 min; none hit the box), on a pristine worktree of the base, lib tests filtered to the file's own and neighbouring modules, slow `attack_tests` skipped |
| Phase 2 base | every phase-1 survivor (missed + timeout) re-tested on the base with **all** lib tests plus the integration binaries that exercise the file — this is the "before" column: a mutant some existing test kills was never a survivor |
| Phase 2 after | the base run's survivors re-tested on the branch head with the same suite plus `cloud_mutants_import` (the branch holds every base test, so a mutant the base suite kills is killed after too) |
| Skipped tests | the three tests that fail as root (the container runs as root; `chmod` cannot deny root): `a_day_ledger_that_will_not_take_a_line_leaves_the_day_open`, `a_journal_that_cannot_be_written_fails_the_day_by_name`, `a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses` — all three pass as uid 65534 (see Gates). Phase 2 also skipped `dpn_04_every_dated_name_round_trips_or_is_refused_by_name` (it passes in the final run) and, from `request_minutes` on, the load-sensitive timing test `deduplicating_index_links_costs_one_probe_each_rather_than_a_scan` (see Measurement notes) |
| Coverage | `cargo llvm-cov -p pull --lcov`; production lines only (each file cut at its first `#[cfg(test)] mod`). `--branch` needs a nightly toolchain and is **not supported** on the pinned 1.97.1, so branch coverage is not measured; region coverage from the summary is given instead |

## Result

On the branch head, **every mutant that can be killed is killed** in all ten
files. What survives is 18 equivalent mutants (each with its reason below) and
5 that hang a test until cargo-mutants' 300 s timeout (detected, filed as
timeouts). **No real bug was found**, and no production line was changed.

| | Before | After |
|---|---:|---:|
| Mutants (all ten files) | 1,809 | 1,809 |
| Unviable (do not build) | 208 | 208 |
| Caught | 1,490 | 1,578 |
| **Missed** | **104** | **18** (all equivalent) |
| Timeout | 7 | 5 |
| Survivors killed by new tests | | 88 |

99 new tests: 48 in `crates/pull/tests/cloud_mutants_import.rs` (public items)
and 51 in `cloud_mutant_tests` modules appended to eight source files (private
items). 3,463 lines added, 0 deleted, in 27 test commits. The mutation after-run was
taken at `a259437`; the three tests added after it (`98ecff3`, `3f0562d`) only
add assertions (a coverage line and the missing-day input), so every mutant
result stands.

## Mutants per file

"Before" is the base `c9e3472` against its whole existing suite (phase 1 plus
phase 2 base). "After" is the branch head with the new tests. Caught counts
exclude unviable mutants.

| File | Mutants | Unviable | Caught before | Missed before | Timeout before | Missed after | Timeout after | Caught after |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `scrub.rs` | 62 | 2 | 59 | 1 | 0 | 0 | 0 | 60 |
| `gdfl_import_scope.rs` | 39 | 1 | 36 | 2 | 0 | 0 | 0 | 38 |
| `gdfl_cm_auction.rs` | 61 | 7 | 51 | 3 | 0 | 0 | 0 | 54 |
| `cash_session_cache.rs` | 101 | 15 | 59 | 26 | 1 | 1 | 1 | 84 |
| `request_minutes.rs` | 143 | 10 | 116 | 15 | 2 | 2 | 0 | 131 |
| `gdfl_archive.rs` | 142 | 13 | 127 | 2 | 0 | 0 | 0 | 129 |
| `gdfl_tickstore.rs` | 164 | 12 | 151 | 1 | 0 | 0 | 0 | 152 |
| `fold.rs` | 243 | 12 | 206 | 22 | 3 | 8 | 3 | 220 |
| `gdfl_nfo.rs` | 295 | 52 | 240 | 2 | 1 | 0 | 1 | 242 |
| `gdfl_cm.rs` | 559 | 84 | 445 | 30 | 0 | 7 | 0 | 468 |
| **Total** | **1809** | **208** | **1490** | **104** | **7** | **18** | **5** | **1578** |

## Survivors killed, and the tests that kill them

Read from the after run's own logs: for each mutant the base suite missed (or
timed out on) and the branch kills, the new tests that failed on it.

#### `scrub.rs` — 1 killed

| Mutant | Killed by (new tests) |
|---|---|
| `458:5: replace contract_of -> Option<Contract> with None` | `scrub_contract_of_is_the_entrys_own_contract` |

#### `gdfl_import_scope.rs` — 2 killed

| Mutant | Killed by (new tests) |
|---|---|
| `120:62: replace / with * in Record::of` | `a_digest_is_sixteen_hex_digits_of_the_sorted_list`, `a_scope_line_whose_count_or_digest_is_not_its_lists_refuses`, `every_torn_fragment_is_counted_once_and_whole_lines_still_read`, `record_and_examine_write_their_lines_once_and_a_reload_reads_them` |
| `172:25: replace match guard why.kind() == std::io::ErrorKind::NotFound with true in Ledger::load` | `a_ledger_that_cannot_be_read_refuses_and_only_a_missing_one_is_empty` |

#### `gdfl_cm_auction.rs` — 3 killed

| Mutant | Killed by (new tests) |
|---|---|
| `336:60: replace - with + in write_day` | `a_print_at_ist_midnight_is_its_own_days_and_not_the_day_befores` |
| `340:55: replace < with <= in write_day` | `a_print_at_ist_midnight_is_its_own_days_and_not_the_day_befores` |
| `380:5: replace note_filed with ()` | `every_filed_day_says_what_it_filed` |

#### `cash_session_cache.rs` — 25 killed

| Mutant | Killed by (new tests) |
|---|---|
| `29:33: replace * with +` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `a_master_exactly_at_the_compressed_cap_reaches_the_gzip_reader`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused`, `the_caps_are_four_and_thirty_two_mebibytes` |
| `30:32: replace * with +` | `an_expansion_exactly_at_the_cap_reaches_the_csv_reader`, `the_caps_are_four_and_thirty_two_mebibytes` |
| `30:39: replace * with +` | `an_expansion_exactly_at_the_cap_reaches_the_csv_reader`, `the_caps_are_four_and_thirty_two_mebibytes` |
| `226:5: replace prepare -> Result<HashMap<u32, DailyEligibility>, String> with Ok(HashMap::new())` | `prepare_over_a_held_window_answers_every_required_day` |
| `254:5: replace prepare_observed -> Result<(), String> with Ok(())` | `prepare_observed_fills_the_cache_from_held_masters` |
| `576:21: replace match guard why.kind() == ErrorKind::NotFound with true in regular_file` | `an_uninspectable_path_is_named_and_write_new_never_overwrites` |
| `586:28: replace \|\| with && in read_limited` | `read_limited_reads_exactly_its_limit_and_no_further` |
| `586:46: replace > with == in read_limited` | `read_limited_reads_exactly_its_limit_and_no_further` |
| `586:46: replace > with >= in read_limited` | `read_limited_reads_exactly_its_limit_and_no_further` |
| `593:28: replace + with * in read_limited` | `read_limited_reads_exactly_its_limit_and_no_further` |
| `593:28: replace + with - in read_limited` | `read_limited_reads_exactly_its_limit_and_no_further` |
| `596:20: replace > with == in read_limited` | `read_limited_reads_exactly_its_limit_and_no_further` |
| `596:20: replace > with >= in read_limited` | `read_limited_reads_exactly_its_limit_and_no_further` |
| `627:25: replace > with >= in decode` | `a_master_exactly_at_the_compressed_cap_reaches_the_gzip_reader` |
| `639:23: replace > with >= in decode` | `an_expansion_exactly_at_the_cap_reaches_the_csv_reader` |
| `708:5: replace download -> Result<Vec<u8>, String> with Ok(vec![0])` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `a_download_is_the_body_of_a_200_and_nothing_else_is`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `708:5: replace download -> Result<Vec<u8>, String> with Ok(vec![1])` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `a_download_is_the_body_of_a_200_and_nothing_else_is`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `708:5: replace download -> Result<Vec<u8>, String> with Ok(vec![])` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `a_download_is_the_body_of_a_200_and_nothing_else_is`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `731:26: replace != with == in download` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `a_download_is_the_body_of_a_200_and_nothing_else_is`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `739:38: replace > with < in download` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `a_download_is_the_body_of_a_200_and_nothing_else_is` |
| `739:38: replace > with == in download` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `739:38: replace > with >= in download` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `751:52: replace > with < in download` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `a_download_is_the_body_of_a_200_and_nothing_else_is`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `751:52: replace > with == in download` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused` |
| `751:52: replace > with >= in download` | `a_declared_length_of_exactly_the_cap_is_read_and_one_more_is_refused`, `an_undeclared_body_of_exactly_the_cap_is_read_and_one_more_is_refused` |

#### `request_minutes.rs` — 15 killed

| Mutant | Killed by (new tests) |
|---|---|
| `130:21: replace += with *= in Cursor<'a>::refused` | `a_minute_refused_by_name_is_a_refused_day_and_no_receipt_finding`, `a_refused_second_is_found_in_any_list_asked_in_order` |
| `142:52: replace < with <= in Cursor<'a>::any_in` | `any_refused_second_in_a_half_open_span_is_found` |
| `142:52: replace < with == in Cursor<'a>::any_in` | `any_refused_second_in_a_half_open_span_is_found` |
| `143:21: replace += with *= in Cursor<'a>::any_in` | `any_refused_second_in_a_half_open_span_is_found` |
| `145:53: replace < with <= in Cursor<'a>::any_in` | `any_refused_second_in_a_half_open_span_is_found` |
| `480:43: replace < with > in audit_refused` | `a_front_day_is_refused_front_and_a_repaired_whole_day_says_repaired` |
| `501:5: replace withheld_day with ()` | `a_withheld_day_counts_only_its_own_rows_and_leaves_the_next_days`, `an_unsized_session_is_withheld_whole_through_the_audit` |
| `503:18: replace >= with < in withheld_day` | `a_withheld_day_counts_only_its_own_rows_and_leaves_the_next_days`, `an_unsized_session_is_withheld_whole_through_the_audit` |
| `503:26: replace + with * in withheld_day` | `a_withheld_day_counts_only_its_own_rows_and_leaves_the_next_days` |
| `503:26: replace + with - in withheld_day` | `a_withheld_day_counts_only_its_own_rows_and_leaves_the_next_days`, `an_unsized_session_is_withheld_whole_through_the_audit` |
| `506:18: replace >= with < in withheld_day` | `a_withheld_day_counts_only_its_own_rows_and_leaves_the_next_days`, `an_unsized_session_is_withheld_whole_through_the_audit` |
| `534:41: replace < with > in short_why` | `a_front_day_is_refused_front_and_a_repaired_whole_day_says_repaired` |
| `548:53: replace + with * in local_seconds` | `every_stamp_encoding_lands_on_the_ist_local_second` |
| `548:53: replace + with - in local_seconds` | `every_stamp_encoding_lands_on_the_ist_local_second` |
| `785:5: replace run_ordinal -> u64 with 0` | `request_minutes_a_gap_line_names_the_sinks_run` |

#### `gdfl_archive.rs` — 2 killed

| Mutant | Killed by (new tests) |
|---|---|
| `446:16: delete field scope from struct Self expression in Archive<B>::with_scope` | `a_one_sided_book_in_the_auction_band_is_a_row_not_a_print`, `an_archive_given_a_scope_lists_its_days_under_it`, `an_eligible_shares_closing_auction_print_is_diverted_from_its_rows` |
| `710:63: replace + with - in stored_span` | `a_stored_members_span_skips_its_local_extra_field` |

#### `gdfl_tickstore.rs` — 1 killed

| Mutant | Killed by (new tests) |
|---|---|
| `568:21: replace match guard why.found() == Some(Found::Directory) with false in open_day` | `tickstore_a_directory_at_a_day_name_is_unavailable_as_a_directory` |

#### `fold.rs` — 14 killed

| Mutant | Killed by (new tests) |
|---|---|
| `272:38: replace match guard width > 0 with false in fold` | `a_fold_reserves_exactly_the_bars_it_can_return` |
| `272:44: replace > with < in fold` | `a_fold_reserves_exactly_the_bars_it_can_return` |
| `272:44: replace > with == in fold` | `a_fold_reserves_exactly_the_bars_it_can_return` |
| `414:5: replace bucket_start -> Option<i64> with Some(0)` | `a_sessions_first_bucket_is_on_the_folds_grid` |
| `414:5: replace bucket_start -> Option<i64> with Some(1)` | `a_sessions_first_bucket_is_on_the_folds_grid` |
| `751:9: replace Ladder::finished -> bool with true` | `fold_the_ladder_is_finished_only_after_its_last_rung` |
| `1126:32: replace > with < in fold_month` | `a_withheld_days_buckets_are_counted_under_its_own_sentence` |
| `1126:32: replace > with == in fold_month` | `a_withheld_days_buckets_are_counted_under_its_own_sentence` |
| `1245:42: replace * with + in flush_tail` | `an_index_days_absent_head_and_tail_are_named_and_a_shares_are_not` |
| `1273:60: replace > with >= in session_open` | `a_session_with_no_window_opens_and_closes_nothing` |
| `1277:44: replace * with + in session_open` | `a_sessions_first_bucket_is_on_the_folds_grid` |
| `1277:44: replace * with / in session_open` | `a_sessions_first_bucket_is_on_the_folds_grid` |
| `1317:59: replace > with >= in owed_buckets` | `owed_buckets_stop_at_the_end_of_the_i64_line` |
| `1688:17: delete field ts_micros from struct Bar expression in fold_seconds_with_last_oi` | `a_seconds_bar_is_stamped_at_its_seconds_start` |

#### `gdfl_nfo.rs` — 2 killed

| Mutant | Killed by (new tests) |
|---|---|
| `610:60: replace + with - in readings_at` | `nfo_a_pre_switch_index_weekly_is_dated_only_inside_the_three_month_cycle` |
| `1354:35: replace && with \|\| in decode_capped` | `a_row_is_quoted_when_its_bid_or_its_ask_is_not_zero` |

#### `gdfl_cm.rs` — 23 killed

| Mutant | Killed by (new tests) |
|---|---|
| `965:42: replace > with == in CmScope::extra_slot` | `a_stored_share_of_the_full_symbol_width_is_filed` |
| `965:42: replace > with >= in CmScope::extra_slot` | `a_stored_share_of_the_full_symbol_width_is_filed` |
| `1013:31: replace > with == in CmScope::filing` | `a_stored_share_of_the_full_symbol_width_is_filed` |
| `1013:31: replace > with >= in CmScope::filing` | `a_stored_share_of_the_full_symbol_width_is_filed` |
| `1063:9: replace CmScope::files_series -> bool with false` | `the_stored_scope_skips_an_index_underlying_and_files_series` |
| `1063:9: replace CmScope::files_series -> bool with true` | `the_stored_scope_skips_an_index_underlying_and_files_series` |
| `1096:17: delete match arm Resolution::Swept(key) in CmScope::member_of` | `a_swept_index_is_its_own_member_under_either_scope` |
| `1473:9: replace <impl PartialEq for Cas>::eq -> bool with false` | `listings_compare_by_their_eligibility_source` |
| `1473:9: replace <impl PartialEq for Cas>::eq -> bool with true` | `listings_compare_by_their_eligibility_source` |
| `1474:13: delete match arm (Self::NotRequired, Self::NotRequired) in <impl PartialEq for Cas>::eq` | `listings_compare_by_their_eligibility_source` |
| `1475:13: delete match arm (Self::Master(a), Self::Master(b)) in <impl PartialEq for Cas>::eq` | `listings_compare_by_their_eligibility_source` |
| `1475:66: replace == with != in <impl PartialEq for Cas>::eq` | `listings_compare_by_their_eligibility_source` |
| `1476:13: delete match arm (Self::Unknown(a), Self::Unknown(b)) in <impl PartialEq for Cas>::eq` | `listings_compare_by_their_eligibility_source` |
| `1476:55: replace == with != in <impl PartialEq for Cas>::eq` | `listings_compare_by_their_eligibility_source` |
| `1536:24: delete ! in share_isin` | `an_index_underlying_has_no_isin_to_ask_the_master_with` |
| `1946:19: replace < with <= in SpreadRun::finish` | `a_spread_at_the_threshold_counts_and_the_first_widest_second_is_named` |
| `1951:16: replace > with >= in SpreadRun::finish` | `a_spread_at_the_threshold_counts_and_the_first_widest_second_is_named` |
| `2370:61: replace > with >= in Reading<'_>::finish` | `exactly_one_refused_row_in_a_thousand_keeps_the_file` |
| `2693:35: replace && with \|\| in parse_row` | `a_one_sided_book_in_the_auction_band_is_a_row_not_a_print` |
| `2816:35: replace match guard session == Session::full() \|\| landed(day, session) with true in admitted` | `a_day_is_admitted_only_where_its_windows_land` |
| `2816:43: replace == with != in admitted` | `a_day_is_admitted_only_where_its_windows_land` |
| `2969:5: replace note_refused_rows with ()` | `a_file_with_refused_rows_is_one_warn_line_and_a_clean_file_none` |
| `2969:24: replace == with != in note_refused_rows` | `a_file_with_refused_rows_is_one_warn_line_and_a_clean_file_none` |

## Equivalent mutants (missed after; no test can kill them)

| File:line:col | Mutant | Why it is equivalent |
|---|---|---|
| `fold.rs:272:38` | match guard `width > 0` → `true` in `fold` | `width` is `bucket.secs() * 1_000_000` and `Bucket::of_secs` refuses 0, so `width >= 1_000_000`: `width > 0`, `width >= 0` and `true` agree on every input. |
| `fold.rs:272:44` | `>` → `>=` in `fold` | Same: `width` is never 0. |
| `fold.rs:1126:32` | `>` → `>=` in `fold_month` | `day_note_at` is read only for a withheld day, and every withheld day either had `day_session` push its note (both forms then set the same index) or has `day_note` push one, which overwrites `day_note_at` right after. A day that is not withheld never reads it. |
| `fold.rs:1149:68` | `<` → `<=` in `fold_month` | At `missing == start`, `scheduled_minutes(.., missing, start)` is 0, so the `expected > 0` test after it pushes nothing either way. |
| `fold.rs:1206:42` | `>` → `>=` in `fold_month` (complete's `expected > 0`) | Every candidate bucket is built by `fold_from_bars` from at least one bar in `[start, end)`, and the cursor consumes exactly the bars below `end`, so `count >= 1`; `count == expected` cannot hold at `expected == 0`. |
| `fold.rs:1209:22` | `>` → `>=` in `fold_month` (partial's `count > 0`) | `count >= 1` always (as above). |
| `fold.rs:1210:25` | `>` → `>=` in `fold_month` (partial's `expected > 0`) | `count < expected` cannot hold at `expected == 0` with `count >= 1`. |
| `fold.rs:1527:9` | `\|\|` → `&&` in `minute_session` | `Venue::hours_on` returns only rows of the three shipped venue tables, every one `SessionKind::Continuous` and opening at 09:15, so both operands are false on every reachable input. |
| `request_minutes.rs:325:35` | `>` → `>=` in `day_plan` | The arm is reached only with `DayKind::Unmeasured`. `Runtime::kind_of` is `sourced_kind_of`, which inside `SOURCED_FIRST_DAY..=SOURCED_LAST_DAY` answers Open, Closed or OpenLengthUnmeasured, never Unmeasured, and `with_recorded_window` only rewrites OpenLengthUnmeasured. At `SOURCED_LAST_DAY` itself both forms take the same arm. |
| `request_minutes.rs:339:22` | match guard `hours.kind() == Continuous` → `true` in `regular_plan` | Every row of `NSE_INDEX_SESSIONS`, `NSE_CASH_SESSIONS` and `NSE_DERIVATIVES_SESSIONS` is `Continuous` and `verified` (`ClosingAuction` rows exist only in test fixtures), so an `Ok` from `hours_on` is always continuous. |
| `cash_session_cache.rs:508:5` | `still_regular` → `Ok(())` | `still_regular` re-checks after the open that the lock path is still a regular file; the inspection before the open already refused every other kind of path, so only a swap between the two system calls (a race) reaches the refusal. |
| `gdfl_cm.rs:854:13` | delete field `indices` in `CmScope::stored` | The literal ends `..Self::swept()`, whose `indices` is the same empty `Vec::new()` the deleted field wrote. |
| `gdfl_cm.rs:965:31` | match guard `name.len() > SYMBOL_CAPACITY` → `false` in `CmScope::extra_slot` | `share_at` holds only names `add_share` admitted, each a legal `Symbol` of at most 24 bytes, so a longer name misses the probe and the answer is `None` either way. |
| `gdfl_cm.rs:1010:41` | match guard `self.series` → `true` in `CmScope::filing` | Reached only past `if !self.stored { return }`, and the only two constructors set `stored` and `series` together (`swept`: both false; `stored`: both true; `with_masters` keeps both). |
| `gdfl_cm.rs:1097:22` | match guard `self.stored` → `true` in `CmScope::member_of` | An unstored scope is `swept`, whose `indices` is empty, so `index_row` is `None`, the same answer as the `_ => None` it guards. |
| `gdfl_cm.rs:2306:70` | `>` → `>=` in `Reading::settle_auction` | In the unknown-eligibility arm a zero-book candidate refuses the file at its row, so the band is observed only by booked rows and a one-price band always has `booked >= 1`. |
| `gdfl_cm.rs:2313:34` | match guard `self.booked > 0` → `true` in `Reading::settle_auction` | The guard differs only for a one-price band with `booked == 0`, a band of zero-book prints alone. Then no row in `rows` is in the band, `retain` takes nothing, and the recomputed `max_sod` equals the running maximum: the file is identical. |
| `gdfl_cm.rs:2313:46` | `>` → `>=` in `Reading::settle_auction` | Same as 2313:34. (The base run filed it caught, but only because an unrelated timing test failed under load; see Measurement notes.) |

## Timeouts (after): detected by a hang, not missed

| File:line:col | Mutant | What hangs |
|---|---|---|
| `fold.rs:1083:42` | `*` → `/` in `fold_month` | The bucket width becomes `secs / 1_000_000 == 0`, so `owed_buckets` steps `at += 0` forever; the `fold::guard` derive tests hang. |
| `fold.rs:1191:20` | `+=` → `*=` in `fold_month` | `cursor *= 1` never advances, so the minute loop re-reads row 0 forever. |
| `fold.rs:1245:42` | `*` → `/` in `flush_tail` | The tail's width becomes 0; `owed_buckets` loops on the first absent tail (the new head/tail test fails at its 10 s watchdog, but the existing tail tests hang). |
| `gdfl_nfo.rs:569:22` | `-=` → `/=` in `monthly_expiry` | `day /= 1` never steps back over a holiday; `the_monthly_rule_walks_back_over_holidays_and_refuses_the_unmeasured` hangs. |
| `cash_session_cache.rs:546:19` | `>=` → `<` in `hold` | The new `an_installer_waits_for_readers_who_let_go_inside_the_bound` and the existing `a_reader_beside_an_installer_waits_then_answers_or_names_the_wait` and `an_installer_beside_readers_installs_once` FAIL, but `a_killed_holder_releases_the_day` never returns. |

Two base timeouts became kills: `request_minutes.rs:130:21` and `143:21`
(`Cursor` `+=` → `*=`) now fail the new two-second watchdog tests instead of
hanging.

## Real bugs

**None found.** Every survivor was either a missing test (88, now killed) or
equivalent (18). One behaviour looked like a gap and is deliberate: the last
day of a fold's input never has its absent tail named, which
`tests/anchor.rs::a_final_days_absent_tail_has_no_proven_request_endpoint` pins
("no proven request endpoint").

## Extreme inputs covered

All by the new tests, with invented values:

- **Empty / one-row / maximum:** empty, header-only and cut files refused by
  name; an empty scope digest; one-row tick-store blocks; the maximum at each
  bound — 65,000-row files at the refused-row limit, a symbol of exactly 24
  bytes, every size cap exactly at and one past (lock reads, gzip and CSV
  expansion, declared and chunked HTTP bodies, tick-store columns).
- **Torn / truncated / zero-length:** torn scope-ledger fragments counted
  once, zero-length and one-fragment ledgers, a cut CSV, a record failing its
  checksum, a month born without checksums.
- **Duplicate, out-of-order and late rows:** kept in file order with the
  running maximum unchanged by a late row; a duplicate or off-grid minute
  never completes a bucket.
- **A missing day in the middle:** the request audit names it as its own
  375-minute gap; the fold neither invents nor names it.
- **Special sessions:** the 2024-11-01 Muhurat's first hour bucket from its
  17:15 grid start; a Muhurat's day admitted only where its window lands; an
  unsized special session withheld whole.
- **`&` in names:** `M&M.NSE` read through its own stem; `Z&Z` filed in the
  auction sidecar; `B&C` in a scope digest.
- **Closing auction:** an eligible share's zero-book print diverted from its
  rows, a one-sided book kept as a row, a print at IST midnight filed to its
  own day.
- **Quote-only rows (LTQ = 0):** counted and never usable alone; a quote-only
  options row's bid and ask decide the quoted count; a quote-only row still
  gives its second the open interest it carried.

## Coverage, before → after

`cargo llvm-cov -p pull --locked` on the base `c9e3472` and on the branch head
`3f0562d`. **Production lines** count each file only up to its first
`#[cfg(test)] mod`, so a test module's own lines never inflate them.
**Regions** are llvm-cov's whole-file figure and include in-file test code,
which this pass added to; read them as context, not as a production measure.
Branch coverage: **not supported** (see NOT DONE).

| File | Production lines before | Production lines after | Regions before | Regions after |
|---|---:|---:|---:|---:|
| `cash_session_cache.rs` | 364/453 (80.35%) | 438/453 (96.69%) | 76.88% | 82.13% |
| `fold.rs` | 621/638 (97.34%) | 629/638 (98.59%) | 96.52% | 97.42% |
| `gdfl_archive.rs` | 373/379 (98.42%) | **379/379 (100%)** | 98.60% | 98.73% |
| `gdfl_cm.rs` | 1,135/1,168 (97.17%) | 1,161/1,168 (99.40%) | 98.91% | 99.60% |
| `gdfl_cm_auction.rs` | 193/204 (94.61%) | 198/204 (97.06%) | 86.83% | 91.36% |
| `gdfl_import_scope.rs` | 106/107 (99.07%) | **107/107 (100%)** | 95.00% | 99.83% |
| `gdfl_nfo.rs` | 633/635 (99.69%) | **635/635 (100%)** | 99.33% | 99.43% |
| `gdfl_tickstore.rs` | 345/349 (98.85%) | **349/349 (100%)** | 97.26% | 97.29% |
| `request_minutes.rs` | 442/487 (90.76%) | 477/487 (97.95%) | 94.95% | 97.96% |
| `scrub.rs` | 110/128 (85.94%) | 123/128 (96.09%) | 92.20% | 92.68% |
| **Ten files** | **4,322/4,548 (95.03%)** | **4,496/4,548 (98.86%)** | | |

### The 52 production lines still uncovered, and why no test reaches them

| File:lines | What it is | Why it is not reached |
|---|---|---|
| `cash_session_cache.rs:184–188` | `File::open` of an existing regular lock fails | Only a permission denial gets here, and root opens anything. `a_lock_that_cannot_be_opened_refuses_the_read_by_name` exercises it by re-running as uid 65534 (`where_permission_binds`), and that child's coverage is not collected. |
| `cash_session_cache.rs:511–514` | `still_regular`'s refusal | Race only (see the equivalent mutant `508:5`). |
| `cash_session_cache.rs:538–542` | `TryLockError::Error` | `flock` refused by the host for a reason other than contention (e.g. `ENOLCK`); not provokable on a local filesystem. |
| `cash_session_cache.rs:700–704` | `write_all`/`sync_all` failure | Disk full or an I/O error; not provokable without exhausting the disk. |
| `fold.rs:345–349` | `OutOfOrder` with no bar open | Marked in the source: "Unreachable: `open_at` is only ever Some after a push." |
| `fold.rs:1243` | `flush_tail`'s `expected <= 0` return | `missing < close` with both on the minute grid means `[missing, close)` holds the last window's last minute, so `expected >= 1`. |
| `fold.rs:1529–1531` | `minute_session`'s refusal of non-continuous or non-09:15 hours | Every shipped venue row is continuous from 09:15 (see the equivalent mutant `1527:9`). |
| `gdfl_cm.rs:868` | the false branch of `if change.status.applies()` | Every row of `core::alias::SYMBOL_CHANGES` is built `Status::Sourced`, so `applies()` is always true. |
| `gdfl_cm.rs:1353–1354` | a slot with no cell | `slot_of` only returns slots inside `cells`/`extra`. |
| `gdfl_cm.rs:1522–1524` | `Ok(CasEligibility::Unknown)` from a master | `VerifiedLifecycleMaster::cas_eligibility` answers only `Eligible`, `NotEligible` or `Err`. |
| `gdfl_cm.rs:2837` | `landed`'s `Window::new(day, day)` refusal | `Window::new` refuses only a window that runs backwards. |
| `gdfl_cm_auction.rs:295` | the false branch of `if let Some(parent)` | A sidecar path under the store root always has a parent. |
| `gdfl_cm_auction.rs:310–314` | the sidecar lock not released | A host error on `flock` release. |
| `request_minutes.rs:340–341` | `regular_plan`'s refusal of venue hours | Every venue row is `verified` and continuous (see the equivalent mutant `339:22`). |
| `request_minutes.rs:418–425` | the "invalid day" finding | The loop walks day numbers between two valid `Day`s, each itself a valid day. |
| `scrub.rs:219–223` | an entry naming no legal store path | Every path segment comes from a type that already enforces the store's rules (`Symbol`, `Contract`, the exchange and segment enums: same bytes, at most 24), so `StorePath::new` cannot refuse a manifest entry. |

## Gates, on `3f0562d`

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | **pass** (exit 0) |
| `cargo clippy -p pull --all-targets -- -D warnings` | **pass** (exit 0) |
| `cargo test -p pull --locked` | **exit 101: 1,724 passed, 3 failed, 23 ignored** (run with `--no-fail-fast` to list every failure) |

The three failures are the same three that fail on the base `c9e3472`, and
for the same reason: the container runs as **root**, and each test needs a
file that its own `chmod` makes unreadable or unwritable, which root ignores.

- `gdfl_import::tests::a_day_ledger_that_will_not_take_a_line_leaves_the_day_open`
- `gdfl_import::tests::a_journal_that_cannot_be_written_fails_the_day_by_name`
- `gdfl_tickstore::tests::a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses`

Run from the same test binary as uid 65534
(`setpriv --reuid=65534 --regid=65534 --clear-groups <binary> <test> --exact`),
**all three pass**. No new test fails, as root or otherwise.

Before every commit: `cargo fmt --all --check`, clippy with `--locked`, and
a gate-1d check that the commit adds no undeclared lowercase literal under
`crates/pull` (the base already carries 748).

## Measurement notes

- **A load-sensitive timing test made spurious catches.**
  `nse::tests::deduplicating_index_links_costs_one_probe_each_rather_than_a_scan`
  (a cost test in `nse.rs`, outside scope) failed under the mutation run's
  load on five mutants of the phase-2 base pass (one in `gdfl_cm`, four in
  `fold`), and on no other. When the lib binary fails, `cargo test`
  stops before the integration binaries, so those five mutants were filed
  "caught" without the tests that actually decide them running. From
  `request_minutes` on, phase 2 skips that one test. The five were re-run on
  the base without it: `fold.rs:116:22` and `738:23` are truly caught (by
  `tests/unit.rs`); `fold.rs:272:44` (`==`), `fold.rs:1527:9` and
  `gdfl_cm.rs:2313:46` were missed and are counted missed-before. No phase-1
  catch depended on a timing test alone (checked over every shard's logs).
- cargo-mutants 27.1.0 also runs "delete field" mutants that a `--re` filter
  does not select; such extras were ignored in every count (only phase-1
  survivors are tallied in phase 2).
- The worker restarted once mid-run; the interrupted shard (`fold` 2/3) was
  re-run from scratch.
- The mutant builds used opt-level 0 (the workspace dev profile is opt-level
  3); test outcomes do not depend on the optimisation level, but timings do.

## NOT DONE

- **Branch coverage is not measured.** `cargo llvm-cov --branch` needs a nightly
  toolchain; the workspace pins 1.97.1. Region coverage is given instead.
- **Five timeouts remain timeouts.** Each is a detected infinite loop, but
  cargo-mutants files a hang separately from a kill; turning them into kills
  would mean editing existing tests (to add watchdogs), which this pass does
  not do.
- **18 equivalent mutants remain missed**, by definition. Two of them
  (`request_minutes.rs:339:22`, `fold.rs:1527:9`) would become killable only if
  a venue table gained a non-continuous or non-09:15 row.
- **Uncovered production lines remain** in five files, each listed above with
  why no test reaches it (host I/O failures, a race, and branches the types or
  shipped tables make unreachable). Lines 184–188 of `cash_session_cache.rs` are
  exercised by a test that runs as uid 65534, whose coverage is not collected.
- **The container runs as root.** Three existing tests that rely on file
  permissions fail as root and pass as an unprivileged user (see Gates); they
  were skipped in the mutation runs, as was
  `dpn_04_every_dated_name_round_trips_or_is_refused_by_name` in phase 2.
- **Not run:** `cargo deny check`, and workspace-wide clippy and tests (this
  pass was scoped to `-p pull`). No new invariant or locked choice was
  introduced (the tests pin existing behaviour), so `docs/04-invariants.md`
  and `docs/05-decisions.md` are unchanged.
- **CI on PR SJParthi/brutex#75 is red for base-snapshot reasons**, not this
  diff: Gate W (`web/build` vs `web/src`), Gate 1+2 step 1e
  (`core`'s `banned_lists` test against `main`'s `ci.yml`) and `ci-ok`; gate
  1d's 748 base literals as well. This branch adds no gate-1d literal (checked
  before every commit).
