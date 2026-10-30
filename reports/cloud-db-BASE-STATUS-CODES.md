STATUS: BLOCKED

Blocked only by reds this branch shares with the unmodified `origin/feat/db-fix` (`f01d7a0e4`, each run
on both trees, outputs below) or by this host, and by figures no tool here can produce:

- `cargo test --workspace --locked --no-fail-fast`: 67 tests in 7 binaries, red on the base too by name
  (root on 4 cores: permission tests root defeats, a 4-slot pool, slow-reader timings; two hang at 0%
  CPU on both trees). NOT PROVED here: red on feat/db-fix too (owner: this host); a non-root run on 8 or
  more cores is NEEDS THE MAC.
- CI language-purity gates 1c, 1d, 23, 27b, 30, 19, 10, 11, 12, 29: NOT PROVED here: red on feat/db-fix
  too, with the same refusal lines (owner BASE-REDS-1023, CONTEXT 4.10). Gate 1e: not run here (it moves
  `web/` aside for a full build; red on the base by CONTEXT 4.10, owner BASE-REDS-1023).
- Web gates W1 (the committed `web/build` is not what `web/src` builds), W2 (13 browser-harness tests of
  `web/tests/speed.test.js`, which need a Chrome the harness can start, and 3 of `web/tests/ask.test.js`,
  cancelled under Node 22.22.0) and W3 (58 svelte-check errors, none in this part's files): NOT PROVED
  here: red on feat/db-fix too (owner BASE-REDS-1023 for W1 and W3; this host for W2).
- D2 coverage and mutants: NOT PROVED here (an instrumented build does not fit the disk, no nightly for
  branch figures, 257 mutants at about 5 minutes each); NEEDS THE MAC item 6.

Every red this branch added was fixed: gate 23's 8 `println!` in `status_rule_tests.rs`, clippy's 10, and
`pool_table_tests::every_store_route_names_its_pool` (red here, green on the base, green here since
`520fcdf6`).

# Cloud build report: BASE-STATUS-CODES (30 Oct 2026)

Part: `BASE-STATUS-CODES` — one status rule through one typed classifier: a client's malformed input is
400, a key that names nothing is one not-found answer on every route, 503 only for what exists but cannot
be read (assurance 29 Oct n=31).

- Branch: `cloud/db-BASE-STATUS-CODES-20261030`, cut from `origin/feat/db-fix` at
  `f01d7a0e430e8f9b01e424a299b0bed20738a0ef` (Merge R4-26), the newest head on 30 Oct 12:43 IST.
- Specification: `origin/cloud/db-base-20261030:reports/db-specs-20261030/BASE-STATUS-CODES.md`, read in
  full with `CLAUDE.md` (from `origin/feat/db-fix`), `README.md` and `CONTEXT.md` of that folder.
- Prerequisites: none (the part file, section 1). FILES_ONLY links: none.
- D1: the merge-back waits for `Merge BASE-REDS-1023:` (not on `feat/db-fix` at `f01d7a0e4`: `grep -c` = 0).
- Decision block: D-7550..D-7559 (D-7550 is this part's entry).
- Host: 4 cores, running as root (uid 0), rustc 1.97.1 (the repository's pin), Node 22.22.0, npm 10.9.4.


## What is done

- **One table** (`crates/api/src/status_rule.rs`, `Class::status`): malformed `400`, not found `404`,
  retired `410`, out of range `416`, over the route's bound `422`, a full detail pool `429`, what exists
  and cannot be read `503`. Typed, never words: an `io::ErrorKind`, the store's `PathError`, or the step
  that refused.
- **Malformed before the store** (`status_rule::segment`, `status_rule::identity`): a `symbol=` by the
  store's own `check_segment` (refused, never folded; a dotless `ı` and a Kelvin sign are refused by
  their first byte), an identity — and every 64-hex field read through `candidatejson::hex_param`
  (completion, digest, pin, seal) — exactly 64 lowercase hex digits, never all zeros; the sentence names
  the parameter, the value (quoted to 80 characters) and the breach.
- **Not found is one `404`, decided by ONE probe** (`status_rule::probe`, one `symlink_metadata`, never
  a listing), the sentence naming the configured root; one body on every identity route
  (`Fault::identity_body`: `schema_version`, `status`, `identity`, `refusal`, `rows: []`, `path`,
  `error_kind`, `rule: "D-7550"`).
- **`503` only for what exists and cannot be read** (another `io::ErrorKind`, a reader's refusal of what
  the probe found, no store root configured). `/frontier.json`'s store faults `400` -> `503`.
- **One path** (`status_rule::serve`, `status_rule::page`) for the nine identity routes of the part.
- **Inside a found result, one refusal type** (`status_rule::Refused`, `Refused::fault`, the extent step
  `reach` / `tail` / `beyond`): a stale pin (completion, attempt, digest, snapshot) is the one `404`; a
  page, candidate, setting, tier or rank past the saved extent `416` (D-7551); a reader's own words
  `503`. The three per-route outcome enums are gone.
- **`/sources*`**: `Refusal`'s constructors take the table's class (`Refusal::of`); `Refusal::store` no
  longer reads its sentence for a `410`; `Instrument::parse` checks `symbol=` by the store's rule (a
  function outside the FILES line, named below).
- **Web readers move with it**: `detail-refusal.js` (`notFoundWhy`, all seven class words, read by the
  test from `Class::word`), `sweep-evidence.js`, `expression-search.js`, `candidate-trades.js`,
  `frontier-pages.js`: "nothing saved" only for the exact `404` body, never a pinned page replaced.
- **Ledgers**: docs/05 D-7550 (with 8a) and D-7551; docs/04 STATUS-01..STATUS-13; docs/06 L-STATUS-01..03.

## What is left (not done here, each named)

- The three Boolean evidence routes' stale completion pins and pages past the saved extent are still
  `503` (L-STATUS-03 (1)): their qualification projection shares `base()` with a file outside this part.
- A grid side a Boolean catalog saved no grid for is still `503` (L-STATUS-03 (2)): no fixture here can
  build a one-sided catalog.
- The routes outside this part that answer the rule's inputs differently (L-STATUS-02, each with its
  file) are their owners'.
- Everything under NEEDS THE MAC.

## Gates (this host: 4 cores, root, rustc 1.97.1, Node 22.22.0)

Base = the unmodified `origin/feat/db-fix` at `f01d7a0e4`, in a clean worktree of its own.

| Gate | Command | This branch | Base `f01d7a0e4` |
|---|---|---|---|
| fmt | `cargo fmt --all --check` | clean | (not red here) |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | EXIT 0 | (not red here) |
| the touched modules | `cargo test -p api --locked --lib -- status_rule sources booleanjson booleanevidencejson indexstop frontierjson expressionsearchjson sweepevidence candidatejson saved_response_boundary` | 180 passed, 0 failed | (the new tests do not exist there) |
| test | `cargo test --workspace --locked --no-fail-fast` | EXIT 101: 68 failing tests in 7 binaries (67 reported, one inside a binary stopped hung); 67 red on the base too, the 68th this part's, fixed (below) | see the by-name run below |
| deny | `cargo deny check` (cargo-deny 0.19.0) | advisories ok, bans ok, licenses ok, sources ok; EXIT 0 | the same, EXIT 0 |
| CI language-purity gates | the 33 `ci-language-purity-*` lines of `checks_all plan`, each run by `bash -c` as `checks_all script` runs it (1e not run: it moves `web/` away for a full build) | 23 green; red 1c 1d 23 27b 30 19 10 11 12 29 | the same 10 red, the same refusal lines (only file and line counts differ) |
| web W1 | `npm --prefix web run build`, then `gates_jobs w1` | red: the committed `web/build` is not what `web/src` builds (48 files) | red (46 files) |
| web W2 | `node --test web/tests/*.test.js` | 1,802 tests: 1,786 pass, 13 fail, 3 cancelled | 1,796 tests: 1,780 pass, the same 13 fail, the same 3 cancelled |
| web W3 | `npm --prefix web run check`, ceiling 0 | 58 errors, none in a file of this part | 58 errors |
| web W4, W5, W6 | as `ci.yml` | green | green |
| web build | `npm --prefix web run build` (inside W1 and W4) | builds | builds |
| D2 coverage | `cargo llvm-cov` (cargo-llvm-cov 0.8.4 installed here) | NOT PROVED here | — |
| D2 mutants | `cargo mutants` (cargo-mutants 26.2.0 installed here) | NOT PROVED here | — |

**The workspace reds, compared by name.** Disk: 7.4 GB free beside this branch's 19 GB target, so a
second full workspace build does not fit and nothing may be deleted (CONTEXT). The base's test binaries
of the seven failing binaries were built in the same target (`cargo test --locked --no-run -p <crate>
<target>`), and the failing tests were run by name with the same command on both trees:
`<binary> --exact <names> --test-threads=4`, from the crate's folder, bounded at 25 minutes.

| Binary | Failing on this branch | This branch, by name | Base, by name |
|---|---|---|---|
| `api` lib | 15 | 15 failed (the pool table among them, fixed in `520fcdf6`: 174 passed, 0 failed at 16:14 IST with it) | 14 failed, 1 passed (the pool table: this part's red alone) |
| `api` `tests/cloud_mutants_bars.rs` | 1, hung | stopped at 20 min, 0% CPU (workspace run) | hung the same, stopped at 20 min, 0% CPU |
| `cli` lib | 29 | 29 failed | 29 failed |
| `cli` `tests/gdfl_run_logs.rs` | 2 | 2 failed | 2 failed |
| `pull` lib | 12 | 12 failed | 12 failed |
| `store` lib | 5 | 5 failed | 5 failed |
| `telemetry` lib | 4 | 4 failed | 4 failed |

`server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit` hung at
0% CPU on both trees (its child holds the calendar pool's slots) and was stopped at 20 minutes on each, so
it is red on both. Where a message says why, it is this host: "this host reads a 000 directory (root?)",
"the test needs a reader the mode refuses (not root)", "a read-only root takes no audit dir", a 4-slot
pool on 4 cores ("at most 4 run at once"), slow readers cut at 1.28 s (`head_deadline_body_tests`). Each is
**NOT PROVED here: red on feat/db-fix too (owner: this host, root on 4 cores)**, listed under NEEDS THE
MAC for a non-root run on 8 or more cores (CONTEXT 4.10). The `store` and `telemetry` binaries are one
build on both trees (this part changes neither crate nor any crate they depend on). By name:

- `api`: `audit::cloud_mutant_tests::an_unreadable_journal_refuses_the_index_build_and_says_so`, `detail::tests::a_refused_admission_names_what_its_bound_and_why`, `pullrun::cloud_mutant_tests::newest_records_that_cannot_be_read_leave_the_answer_not_known`, `pullrun::restart_tests::the_journals_own_extremes`, `server::basis_route::tests::a_table_refused_as_unreadable_is_read_again_once_its_mode_is_repaired`, `server::cloud_mutant_tests_c::a_store_that_takes_no_write_trips_the_store_breaker`, `server::cloud_mutant_tests_c::a_store_whose_audit_dir_cannot_be_made_is_at_fault`, `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`, `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`, `server::head_deadline_body_tests::a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole`, `server::head_deadline_body_tests::a_chunked_body_reaches_a_slow_reader_whole`, `server::head_deadline_body_tests::a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole`, `server::head_deadline_body_tests::pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is`, `server::head_deadline_body_tests::slow_readers_at_the_cap_are_served_whole_and_the_next_waits`
- `cli`: `gdfl_import_command::cloud_mutant_tests::the_receipt_names_the_workers_and_durability_it_filed_with`, `gdfl_import_command::guard::tests::a_change_is_never_accepted_by_a_run_that_skipped_no_day`, `gdfl_import_command::guard::tests::a_changed_archive_with_every_day_done_is_named_not_ok`, `gdfl_import_command::guard::tests::a_run_that_read_no_file_is_not_accepted_by_running_it_again`, `gdfl_import_command::guard::tests::a_source_root_that_vanishes_after_files_were_read_is_named_source_lost`, `gdfl_import_command::guard::tests::an_empty_range_owes_nothing_and_exits_ok`, `gdfl_import_command::guard::tests::every_day_already_refused_identically_still_fails_when_the_root_is_gone`, `gdfl_import_command::guard::tests::one_archive_reached_by_another_spelling_is_the_same_key`, `gdfl_import_command::tests::a_dangling_day_file_link_ends_the_run_failed_never_a_day_not_held`, `gdfl_import_command::tests::a_named_failure_or_a_refused_run_exits_failed`, `gdfl_import_command::tests::a_run_says_a_line_a_day_and_leaves_its_progress_record`, `gdfl_import_command::tests::a_run_that_read_no_file_never_exits_ok`, `gdfl_import_command::tests::an_unstamped_build_is_refused_a_store_write_unless_allowed`, `gdfl_import_command::tests::the_retry_flag_is_never_a_symbol_and_the_run_line_names_the_source`, `gdfl_import_command::tests::the_stored_flag_narrows_a_capital_market_run_to_what_d3333_adds`, `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`, `tick_second_check::tests::a_lock_name_that_is_a_link_a_directory_or_a_second_name_is_refused_and_nothing_goes_through_it`, `tick_second_check::tests::a_month_held_only_sealed_is_checked_row_for_row_as_its_raw_form_was`, `tick_second_check::tests::a_new_landing_of_a_checked_day_is_checked_again`, `tick_second_check::tests::a_pipe_or_a_link_at_one_of_the_checks_own_names_and_a_pipe_archive_are_refused_at_once`, `tick_second_check::tests::a_share_directory_the_store_refuses_is_one_cell_and_files_nothing_twice`, `tick_second_check::tests::a_share_whose_months_cannot_be_listed_is_filed_month_by_month_with_or_without_a_year`, `tick_second_check::tests::a_tick_store_landing_is_checked_against_that_tick_store_day_and_a_changed_day_is_refused`, `tick_second_check::tests::a_version_whose_rows_are_not_the_ones_its_index_names_is_refused`, `tick_second_check::tests::an_extra_archive_is_compared_beside_the_landing_and_labelled_not_the_landing_source`, `tick_second_check::tests::every_status_and_source_is_named_against_the_landing_the_store_recorded`, `tick_second_check::tests::no_worker_count_and_no_resumption_changes_a_byte_of_the_version`, `watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many`, `watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`
- `gdfl_run_logs`: `a_run_whose_sink_is_held_refuses_before_any_lock`, `two_cli_kinds_at_once_both_log`
- `pull`: `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`, `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`, `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`, `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`, `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice`, `gdfl_run::tests::a_clock_step_back_never_aborts_a_run`, `gdfl_run::tests::a_progress_file_that_cannot_be_written_is_named_and_the_run_goes_on`, `gdfl_run::tests::the_kinds_run_in_order_on_the_calling_thread`, `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`, `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`, `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`, `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named`
- `store`: `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`, `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`, `settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`, `settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`, `settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`
- `telemetry`: `sink::tests::a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`, `sink::tests::a_failed_delete_is_counted_named_and_left_for_the_next_start`, `sink::tests::a_rename_aside_that_cannot_happen_fails_the_roll_by_name`, `sink::tests::an_audit_stream_that_cannot_be_looked_for_is_an_error`

The one red this part added: `server::pool_table_tests::every_store_route_names_its_pool` read each
handler for `crate::detail::run(`, which now runs inside `status_rule::serve`; fixed in `520fcdf6` (the
table holds the nine routes to `crate::status_rule::serve(` and `serve` to `crate::detail::run(`),
now passing (`cargo test -p api --locked --lib -- pool_table status_rule ...`: 174 passed, 0 failed).

**Coverage and mutants, NOT PROVED here.** cargo-llvm-cov 0.8.4 and cargo-mutants 26.2.0 were installed
(`cargo install --locked`). Coverage: an instrumented build of the `api` graph is a second full target,
which does not fit in the 7.4 GB left, and nothing may be deleted; branch figures also need a nightly
this host does not have. Mutants: 257 of the 1,436 mutants of the touched files fall in the D2 set (the
listing below); each is a rebuild of `api` at `opt-level = 3` (about 5 minutes on this host), some 20
hours. Both are NEEDS THE MAC item 6, by recipe v4.

## Red before, green after (TEST FIRST)

`status_rule::tests::every_symbol_and_identity_route_answers_the_one_status_rule` run over the base's
own routes: a clean worktree of `f01d7a0e4` with only `status_rule.rs` as a shim of the constants the
table names, `status_rule_tests.rs`'s table section and the `pub mod` line added (never committed),
`CARGO_TARGET_DIR=/home/user/brutex/target cargo test --locked -p api --lib --
status_rule::tests::every_symbol_and_identity_route_answers_the_one_status_rule
status_rule::tests::every_route_of_the_route_table_has_a_row`: `1 passed; 1 failed`, EXIT 101;
the empty-store child: `28 of 206 answers differ`:

```text
/boolean-admission.json [all zeros] answered 503, expected 400
/boolean-admission.json [unknown identity] answered 503, expected 404
/boolean-campaign.json [all zeros] answered 503, expected 400 (outside: booleancampaignjson.rs)
/boolean-candidates.json [all zeros] answered 503, expected 400
/boolean-candidates.json [unknown identity] answered 503, expected 404
/boolean-oos.json [all zeros] answered 503, expected 400 (outside: booleanoosjson.rs)
/boolean-qualification.json [all zeros] answered 503, expected 400
/boolean-qualification.json [unknown identity] answered 503, expected 404
/boolean-qualified-campaign.json [all zeros] answered 503, expected 400 (outside: booleancampaignjson.rs)
/boolean-qualified-search.json [all zeros] answered 503, expected 400 (outside: booleansearchjson.rs)
/boolean-statistics.json [all zeros] answered 503, expected 400
/boolean-statistics.json [unknown identity] answered 503, expected 404
/candidate-trades.json [all zeros] answered 200, expected 400
/candidate-trades.json [unknown identity] answered 200, expected 404
/expression-search.json [all zeros] answered 200, expected 400
/expression-search.json [unknown identity] answered 200, expected 404
/frontier.json [all zeros] answered 200, expected 400
/frontier.json [unknown identity] answered 200, expected 404
/frontier.json [upper-case hex] answered 200, expected 400
/index-stop.json [unknown identity] answered 503, expected 404
/sources.json [lower-case symbol] answered 200, expected 400
/sources.json [mixed-case symbol] answered 200, expected 400
/sources/bars.json [lower-case symbol] answered 503, expected 400
/sources/bars.json [mixed-case symbol] answered 503, expected 400
/sources/day.json [lower-case symbol] answered 503, expected 400
/sources/day.json [mixed-case symbol] answered 503, expected 400
/sweep-evidence.json [all zeros] answered 200, expected 400
/sweep-evidence.json [unknown identity] answered 200, expected 404
```

(The four marked `outside` are routes outside this part: their identity is read through
`candidatejson::hex_param`, so this branch's one identity rule refuses their all-zero identity `400`;
D-7550 item 2, L-STATUS-02.)
On this branch the same test passes over both roots (the empty directory and the regular file).
The web reader tests (`web/tests/status-rule-readers.test.js`) were red 6 of 6 on the base's readers
and are green here (log of 13:35 IST). The new extent tests of the Boolean, single-stop and candidate
routes call this branch's typed `answer` functions, which the base does not have, so they cannot run there;
on the base every one of those inputs was the read's `String` refusal, a `503`. That their assertions
kill the old behaviour is for the D3 mutants (NEEDS THE MAC item 6).

## Measurements (p50 / p99 / max)

From `crates/api/src/status_rule_tests.rs`, written by the tests to
`<temp>/brutex-<pid>-status-rule-figures.txt` (never printed: gate 23), run of 14:19 IST, this host
(4 cores, root, `dev` profile at `opt-level = 3`), invented stores only. docs/06 L-STATUS-01 holds the same.

| What | n | p50 | p99 | max |
|---|---|---|---|---|
| a `400` on each of the nine identity routes, at 16 / 160 / 1,600 entries per namespace | 200 per route and size | 4 to 6 µs at every size | 7 to 48 µs | 8 to 50 µs |
| the one not-found on each identity route but `/frontier.json`, same three sizes | 200 per route and size | 37 to 43 µs | 54 to 171 µs | 60 to 345 µs, two stalls 2,327 and 3,356 µs |
| the one not-found on `/frontier.json` (receipt-index lookup after its fixed preflight) | 200 per size | 63 to 64 µs | 130 to 323 µs | 152 to 619 µs |
| a `400` on `/sources.json`, `/sources/bars.json`, `/sources/day.json`, same three sizes | 200 per route and size | 168 to 226 µs | 265 to 545 µs | 288 to 2,009 µs |
| 64 malformed questions over TCP (`router_serving`), one at a time | 64 | 197 µs | 418 µs | 418 µs |
| the same 64 at once, every identity route and kind (each its exact `400`, no `5xx`, no `429`) | 64 | 6,265 µs | 33,787 µs | 33,787 µs |

Flat: the largest hundredfold-to-onefold median ratio is 1.07 (`/sources.json`, 226 / 212 µs); the
test asserts each within three times plus 200 µs, and the burst's p99 within 64 times the series median
plus 50 ms. The run of 16:14 IST on the final code (after the last fixes) measured the 64 over TCP at p50
152 / p99 677 / max 677 µs one at a time and p50 16,483 / p99 23,445 / max 23,445 µs at once (still
every answer its exact `400`). The real-store figures (n >= 200 sequential GETs on a shadow) are NEEDS
THE MAC item 5.

## Extreme cases (at least 8 per file tested, 2 combinations, 1 out of the box)

- `status_rule_tests.rs` (the classifier and the route table): `nifty`, `NiFtY`, a trailing blank, an
  empty symbol, `..`, 25 bytes, a wrong-case segment, `M&M` encoded (one symbol) and not (reads as `M`),
  63 and 65 hex digits, upper-case hex at the first and the last digit, all zeros, a NUL after 64
  digits, a multi-byte character that makes the length right, a value over 80 characters quoted to a
  bound; a root that is missing, a file where the folder should be, a dangling link at the entry,
  mode 000 (re-run as uid 65534, where the bits bind), no root configured; a regular file as the root of
  the whole table. COMBINATIONS: a symbol too long AND lower case (its length first); a lower-case symbol
  beside a malformed date (the symbol's refusal, the route's first step); 64 parallel clients each a
  different malformed input across every identity route. OUT OF THE BOX: a dotless `ı` and a Kelvin sign,
  whose Unicode case folds are ASCII, refused and never folded into `NIFTY` / `BANKNIFTY`.
- `booleanjson_tests.rs` (13 cases): offset AT the extent (200, empty), one past, `usize::MAX`;
  past the programs, trades, sessions and grid levels; a candidate past the count; a stale completion.
  COMBINATIONS: a stale pin beside a page past the extent (the pin's 404); a candidate and a page both
  past (the candidate's 416). OUT OF THE BOX: the largest candidate a client can name. The held catalog
  is not evicted by any of them (no cold admission).
- `indexstop_projection_tests.rs` (12 cases): the same shape over settings, trades, events and days;
  COMBINATIONS: a stale pin beside a setting past the count; a setting and a page both past. OUT OF THE
  BOX: `setting=usize::MAX`.
- `candidatejson.rs` (12 + 3 + 5): a page at and past the candidate sides, `u64::MAX`; a tier at and past
  the count, `u64::MAX`; a rank past the tier; a no-cell candidate's empty trade page and one past it; a
  stale digest; an empty catalog's other tier, offset and rank; the extent step's five edges.
  COMBINATIONS: a stale digest beside a tier past the count; a tier, a rank and a page all past (the
  tier's). OUT OF THE BOX: `rank=u64::MAX` (no side-ordinal overflow).
- `web/tests/status-rule-readers.test.js`: another run's identity, a 200 carrying the words, a 503 with
  them, rows beside the refusal, rows `null`, an empty, a blank and a 4,097-character sentence, schema 2,
  the old `missing` word, an array, `null`, no response at all, exactly 4,096 characters (kept).
  COMBINATIONS: a pinned snapshot with the not-found (refused by name, never "missing"); a pinned digest
  and an offset with the not-found. OUT OF THE BOX: the old `200 "missing"` body, refused as invalid, so
  a server that regressed to it is seen.
- A writer holding the month (still the being-written 503 with `Retry-After`): the existing
  `pool_table_tests::the_fold_day_flags_and_day_table_beside_a_writer_say_being_written` over
  `Refusal::answer`, unchanged by this part, in the workspace run.

## The D2 set (operator decisions D2 and D3)

No `Merge BASE-COV-<G>:` is on `feat/db-fix` at `f01d7a0e4`, so the narrow reading holds: the Rust files
this part added, whole (`crates/api/src/status_rule.rs`, `crates/api/src/status_rule_tests.rs`), and every
function the diff adds or changes, whole, in `booleanevidencejson.rs` (`Model::namespace`, `serve`,
`answer`), `booleanjson.rs` (`boolean_json`, `answer`, `render`, `render_with_budget`, `project`, `page`,
`window`, `grid_page`), `candidatejson.rs` (`hex_param`, `candidate_json`, `answer`, `render`,
`empty_catalog`, `past`, `first_row`, `trade_page`, `page_through`),
`expressionsearchjson.rs` (`anchor`, `expression_search_json`, `answer`, `unpinned`, `render`),
`frontierjson.rs` (`frontier_json`, `selected`, `respond`, `missing_file_response`, `absent_response`),
`indexstopjson.rs` (`index_stop_json`, `answer`, `render`, `project`, `window`), `sources.rs`
(`Refusal::of`, `bad`, `over`, `store`, `gone`, `Instrument::parse`), `sweepevidence.rs` (`Asked::parse`,
`evidence_json`, `respond`, `answer`, `render`), the changed tests of `booleanjson_tests.rs`,
`indexstop_projection_tests.rs`, `saved_response_boundary_tests.rs`, `pool_table_tests.rs`,
`candidatejson.rs`, `expressionsearchjson.rs`, `frontierjson.rs` and `sweepevidence.rs`, and the added
lines outside a function (the `pub mod` line and its doc in `lib.rs`, `const WHAT` and the `use` lines).
Computed from `git diff -U0 origin/feat/db-fix` hunk by hunk against the enclosing `fn`; recipe v4 on the
Mac recomputes it and decides.

**Coverage: NOT PROVED here** (see Gates). **Mutants: NOT PROVED here**: `cargo mutants --list --json
--file <each touched file>` lists 1,436 mutants; the 257 whose function meets a changed line (all 59 of
`status_rule.rs`) are the D2 set:

- `booleanevidencejson.rs` (4): `Model::namespace` 2, `answer` 1, `serve` 1
- `booleanjson.rs` (38): `answer` 1, `boolean_json` 1, `grid_page` 11, `page` 14, `project` 2, `render` 1, `render_with_budget` 6, `window` 2
- `candidatejson.rs` (46): `answer` 2, `candidate_json` 1, `empty_catalog` 6, `first_row` 4, `hex_param` 3, `page_through` 13, `past` 1, `render` 14, `trade_page` 2
- `expressionsearchjson.rs` (27): `anchor` 8, `answer` 2, `expression_search_json` 1, `render` 15, `unpinned` 1
- `frontierjson.rs` (22): `absent_response` 1, `frontier_json` 2, `missing_file_response` 8, `respond` 10, `selected` 1
- `indexstopjson.rs` (28): `answer` 1, `index_stop_json` 1, `project` 14, `render` 10, `window` 2
- `sources.rs` (8): `Instrument::parse` 3, `Refusal::bad` 1, `Refusal::gone` 1, `Refusal::of` 1, `Refusal::over` 1, `Refusal::store` 1
- `status_rule.rs` (59): `<impl From<&str> for Refused>::from` 1, `<impl From<String> for Refused>::from` 1, `<impl std::fmt::Display for Malformed>::fmt` 2, `<impl std::fmt::Display for Refused>::fmt` 1, `<impl std::fmt::Display for Rule>::fmt` 1, `Class::status` 1, `Class::word` 2, `Fault::identity_body` 1, `Fault::malformed` 1, `Fault::refused` 1, `Fault::saturated` 1, `Fault::status` 1, `Fault::unreadable` 1, `Refused::fault` 1, `absent` 1, `beyond` 1, `configured` 1, `identity` 3, `identity_of` 20, `page` 4, `probe` 4, `probe_identity` 1, `reach` 4, `segment` 1, `serve` 1, `tail` 2
- `sweepevidence.rs` (25): `Asked::parse` 12, `answer` 2, `evidence_json` 1, `render` 9, `respond` 1

## Files outside ownership (the FILES line), each named

| File | What, and why |
|---|---|
| `crates/api/src/lib.rs` | `pub mod status_rule;` and its doc line (spec 4.2) |
| `crates/api/src/sources.rs` `Instrument::parse` | the symbol check by `status_rule::segment`: the GOAL's "decided before any store file is opened" needs it before the store; the rest of the file untouched beyond `Refusal` |
| `crates/api/src/booleanjson_tests.rs` | one line (`.to_string()` on the typed refusal) and the new extent test of the owned route |
| `crates/api/src/indexstop_projection_tests.rs` | one line (`.to_string()`) and the new extent test of the owned route |
| `crates/api/src/saved_response_boundary_tests.rs` | `/frontier.json`'s expectation moved with D-7550 (the identity rule's sentence, generated; no store root `503`); `/trades.json`'s bytes unchanged |
| `crates/api/src/pool_table_tests.rs` | the nine routes of this part are held to `crate::status_rule::serve(` (`SERVE`), and `status_rule::serve` itself to `crate::detail::run(`, so the route-to-pool chain is still checked whole; `every_store_route_names_its_pool` was red on this branch until then |
| `web/src/lib/detail-refusal.js` | the shared refusal reader every changed reader uses (`notFoundWhy`, the class words) |
| `web/tests/sweep-evidence.test.js`, `frontier-pages.test.js`, `candidate-trades.test.js`, `expression-search.test.js` | the old `200 "missing"` cases moved to the `404` answer (spec 4.2: "and their web/tests/*.test.js") |

Never edited: `.github/**`, the tests BASE-REDS-1023's spec and LOAD-FLAKE note name,
`web/tests/{database-pages,read-pools,speed}.test.js`, `crates/api/benches/**`, `ingest.rs`
`parse_vendor`/`parse_feed`, `logs.rs`, the refusal-line helper, `server.rs` (read only), `web/build/**`.

## D-numbers

D-7550 (the rule, the not-found status and why, item 8a the one refusal type) and D-7551 (a page past
a found result's extent is `416`). D-7552 to D-7559 are unused.

## NEEDS THE MAC

The part file's section 6, items 1 to 10, all still owed, with what this run adds:

1. **Build and stage** the release `api` and `cli` from the merged tree (pool and lock as section 6
   item 1 says; the merge-back prompt's own pool wins).
2. **The proof replay** on a shadow over a copy of the store, GET only, never `:8080`, on the port the
   merge-back prompt gives (never 9748): the GET rows of `api-attack/17-routes.tsv` (1,416),
   `18-repro-503.tsv` (3) and `29-research.tsv` (66); expected `5xx` 0 against the eight of
   `90-totals.txt`, each row the decided status (the table test's rows of this part are the cloud's
   statement of each decided status). Logs under `state/fix/round2/logs-STATUS/`.
3. **Logged once and searchable**: each replayed answer once in the request line and found on the
   shadow's `/logs`; the refused value never an audit-index key. Note: the line logs `5xx` at `error` and
   `4xx` at `warn` (`logs.rs`, BASE-LOG-SEARCH's), not the specification's "5xx warn, 4xx info"
   (L-STATUS-03 (4)).
4. **Mode 000 as the normal user**: `a_root_whose_mode_is_000_is_unreadable_never_not_found` re-runs
   itself as uid 65534 when the process is root (`isolated::where_permission_binds`), so it passed here
   with the bits binding; on the Mac it runs as the user directly.
5. **Timings on the Mac**: p50 / p99 / max, n >= 200 sequential GETs per touched route on the shadow,
   and the 1x / 10x / 100x flatness on the real store's sizes (the cloud's are in L-STATUS-01).
6. **D3 deep tests by recipe v4** on the D2 set below: Route B coverage, 100% line AND branch, on the
   recipe's nightly (`rustc 1.99.0-nightly (1a98b1e13 2026-08-07)`, LLVM 23.1.0); cargo-mutants 26.2.0
   on the 257 mutants listed below, sharded. NOT PROVED here (see Gates).
7. **The CI shell gates and the landing gate** (`d1Land`, groups 1 to 4) on the merged tree. Gate 1e
   (web moved aside) was not run here (it moves `web/` away during a full build); red on the base by
   CONTEXT 4.10.
8. **D1**: merge only after `Merge BASE-REDS-1023:` is on `feat/db-fix`.
9. **Web**: `(cd web && npm run build && npm test)` with the Mac's `node_modules`; and gate W1: rebuild
   `web/build` on the merged tree. The `web/src` files this part changed:
   `web/src/lib/detail-refusal.js`, `web/src/lib/sweep-evidence.js`, `web/src/lib/expression-search.js`,
   `web/src/lib/candidate-trades.js`, `web/src/lib/frontier-pages.js`. The 13 browser-harness tests of
   `web/tests/speed.test.js` need a Chrome the harness can start (none here), and the 3 tests of
   `web/tests/ask.test.js` cancel under Node 22.22.0 here; both are red on the base too.
10. **Store writes: none.** This part writes no store file; prove the shadow's real-store link was only
    read.

Also owed: the routes outside this part that answer the rule's inputs differently (docs/06 L-STATUS-02,
each with its file) are for their owners; the Boolean evidence routes' stale pins and extents and the
one-sided grid (L-STATUS-03 (1), (2)) are this part's follow-up with the owner of
`booleanqualification_projection.rs`.

Scratch worktrees in this container, local to it and never pushed (nothing was deleted):
`/tmp/claude-0/-home-user-brutex/60d3652a-6139-5053-8fa2-865faba04e64/scratchpad/{base-wt,base-clean,mine-web}`:
`mine-web`'s `web/build` was rebuilt by gates W1 and W4; `base-clean` holds the red-before shim (its
`web/build` restored, the build's new files moved beside it); `base-wt` an earlier shim.

## Log

### 2026-10-30 12:43 IST

- Waited on the switches from 10:28 IST with one blocking 9-minute poll at a time (README step 1); the
  test printed `OPEN 12:43:15 IST` (HOLD removed by `b9e9ed39`, GO and GO-BASE-STATUS-CODES present).
- Branch cut from `origin/feat/db-fix` `f01d7a0e4`.
- NEXT: read the code the part owns (`sources.rs` `Refusal`, the seven identity routes' refusal paths,
  `server.rs` `route_table`), then write `status_rule.rs` and `status_rule_tests.rs` test first.

### 2026-10-30 13:35 IST

- Done (commits `6829bded`, `9c9d3c52`, `a635684a`, all pushed):
  - `crates/api/src/status_rule.rs` (new): `Class` (400/404/410/416/422/429/503), `Rule`/`Malformed`
    (the store's own `check_segment` for a symbol; 64 lowercase hex, never all zeros, for an
    identity), `probe`/`probe_identity`/`absent` (one `symlink_metadata`, never a listing),
    `Fault::identity_body` (one body), `serve`/`page` (one path for every identity route of the part).
  - `sources.rs`: `Refusal`'s constructors take the table's status; `Refusal::store` no longer reads
    its sentence for a `410`; `Instrument::parse` checks `symbol=` by the store's rule (400 before
    the store) — a function outside the FILES line, see "Files outside ownership".
  - The nine identity routes of the seven owned files answer through `serve`/`page`: probe before
    any reader, typed absences as the one 404, frontier's store faults 400 -> 503, sweep-evidence's
    page past the extent 503 -> 416 (D-7551).
  - Web readers: `detail-refusal.js` (`REFUSAL_STATUSES`, `notFoundWhy`), `sweep-evidence.js`,
    `expression-search.js`, `candidate-trades.js`, `frontier-pages.js`; tests moved in
    `web/tests/sweep-evidence.test.js` and `frontier-pages.test.js`, new
    `web/tests/status-rule-readers.test.js` (6 tests: red 6/6 on the base readers, green 6/6 here).
  - Ledgers: docs/05 D-7550, D-7551; docs/04 STATUS-01..12.
- Slip, recorded plainly: at 13:1x IST one shell command started the Py-language interpreter by
  mistake (a typed command, not a script); it read nothing, ran no code and wrote no file, and was
  stopped at once (`TaskStop`). No file of this branch names or uses the language.
- NEXT: run the api tests of the touched modules (in progress), fill the table's outside-route
  answers from the measured run, write docs/06 L-STATUS-01..03 with the measured timings, then the
  full gates.

### 2026-10-30 13:50 IST

- Switches checked at 13:41 IST: `HOLD` absent, `GO` and `GO-BASE-STATUS-CODES` present;
  `origin/feat/db-fix` still `f01d7a0e4`.
- The first full `npm test` (1,802 tests) ended `fail 15, cancelled 3`. Two were this part's: the
  old 200 "missing" cases of `web/tests/candidate-trades.test.js` and `expression-search.test.js`,
  moved to the 404 answer (both readers' own tests). `detail-refusal.js` now quotes all seven class
  words, read by the reader test from `Class::word` in `status_rule.rs` (never typed twice).
  `tests/ask.test.js` (3 cancelled) is red on the unmodified base too (shown below under Gates);
  the 12 browser-harness tests are being compared with the base the same way.
- `frontierjson.rs`'s unknown-identity test now asserts the one 404; the table test prints what
  every outside route answers so its expectation is copied from a run.

### 2026-10-30 14:20 IST

- Switches checked at 14:20 IST: `HOLD` absent, `GO` and `GO-BASE-STATUS-CODES` present;
  `origin/feat/db-fix` still `f01d7a0e4`.
- Refusals inside a found result are typed by their step (`status_rule::Refused`, `reach`, `tail`,
  `beyond`; D-7550 item 8a, D-7551): a stale completion pin on `/boolean-candidates.json` and
  `/index-stop.json` is the one 404; a page, candidate, setting, tier or rank past the saved extent
  is 416 there and on `/candidate-trades.json` (all were 503). The per-route outcome enums of
  `sweepevidence.rs`, `candidatejson.rs` and `expressionsearchjson.rs` are gone: one
  `Refused::fault`.
- The outside rows of the table hold their measured answers (both roots); the quarantine row's
  `cause` was not a valid cause and is now `zero_price`.
- Gate 23 refused the 8 `println!` of `status_rule_tests.rs` (the print surface is declared in
  `.github/workflows/ci.yml`, a file this part never edits): every child now says what it found
  through a file, and the figures go to `<temp>/brutex-<pid>-status-rule-figures.txt`.
- `saved_response_boundary_tests.rs` moved with D-7550 for `/frontier.json` only (the identity
  rule's sentence, generated; a missing store root 503); `/trades.json`'s bytes unchanged.
- `cargo test -p api --locked --lib -- status_rule sources booleanjson booleanevidencejson
  indexstop frontierjson expressionsearchjson sweepevidence candidatejson saved_response_boundary`:
  `180 passed; 0 failed` (14:19 IST). docs/06 L-STATUS-01..03 written with the measured figures.
- Tools installed here: cargo-deny 0.19.0, cargo-mutants 26.2.0, cargo-llvm-cov 0.8.4.
  `cargo deny check`: advisories, bans, licenses, sources ok, EXIT 0 (this branch and the base).
- NEXT: clippy, the full workspace test on both trees, the web gates on both trees, the table's
  red-before on the base, coverage of the D2 set.

### 2026-10-30 15:41 IST

- Switches checked at 15:41 IST: `HOLD` absent, `GO` and `GO-BASE-STATUS-CODES` present;
  `origin/feat/db-fix` still `f01d7a0e4`.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: EXIT 0 (after fixing 10 of this
  part's: `candidatejson::render` over 100 lines, a 113-line test split in two, 7 `Value[...]`
  indexings in tests, one complex type named).
- The 33 language-purity gates (1e not run) on both trees: the same 10 red on each (1c 1d 23 27b 30 19
  10 11 12 29), the same refusal lines, counts aside; gate 23 had also refused this part's 8 `println!`,
  now gone. The six web gates on both trees: W4 W5 W6 green; W1 W2 W3 red on each, W2 the same 13
  browser-harness and 3 `ask.test.js` tests, W3 the same 58 svelte-check errors, none in this part's files.
- `cargo test --workspace --locked --no-fail-fast` on this branch (`6a3cec6a`): EXIT 101, 67 failing
  tests in 7 binaries. One is this part's: `pool_table_tests::every_store_route_names_its_pool` read
  each handler for `crate::detail::run(`, which now runs inside `status_rule::serve`; the table now holds
  the nine routes to `crate::status_rule::serve(` and `serve` to `crate::detail::run(` (to be re-run).
  Two tests hung at 0% CPU past 20 minutes and were stopped (CONTEXT's 20-minute rule):
  `gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit` and
  `cloud_mutants_bars` `spot_json_is_refused_by_name_when_every_store_read_slot_is_held`.
- Disk: 7.4 GB free beside the 19 GB target, so the base's comparison builds the failing crates' test
  binaries only, in the same target (registry dependencies shared), and runs the 67 tests by name there.
- NEXT: the base's run of the 67, the pool table re-run, the table's red-before on the base.

### 2026-10-30 16:25 IST

- The 68 by name on the base's own binaries (same target, same command; the 15:41 count of 67 left out
  the spot test inside the binary stopped hung): every one but the pool table
  red there too, the calendar-pool child hung at 0% CPU on the base as here and was stopped at 20 minutes
  on both; the base's `spot_json_is_refused_by_name_when_every_store_read_slot_is_held` hung at 0% CPU
  the same way and was stopped at 20 minutes (16:38 IST), as here.
- The table test over the base's routes (shim, never committed): `28 of 206 answers differ`, the
  attack's failures exactly, and four outside routes whose all-zero identity this branch now refuses
  `400` through `hex_param` (named in D-7550 item 2 and L-STATUS-02, `f38ec883`).
- `cargo test -p api --locked --lib -- pool_table status_rule booleanjson indexstop candidatejson
  sweepevidence expressionsearchjson frontierjson saved_response_boundary sources`: 174 passed, 0 failed
  (the pool table green).
- The 33 language-purity gates on the final tree: the same 10 red as the base, refusal lines identical
  (gate 23 no longer names this part). `origin/feat/db-fix` is still `f01d7a0e4` and already in this
  branch: nothing to merge. Switches at 16:15 IST: `HOLD` absent, `GO` and `GO-BASE-STATUS-CODES`
  present.
