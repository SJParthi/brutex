STATUS: HELD

# CC-5 — Copies register (cloud build report)

Branch `cloud/db-CC-5-20261024`, cut from `origin/feat/db-fix` at `e851da1ca` (Merge DATA-2).
Spec: `origin/cloud/db-base-20261024:reports/db-specs-20261024/CC-5.md`. Decision block D-7040..D-7049.
Prerequisites: none (spec section 1). FILES_ONLY links: none.

## Done so far

- `crates/store/src/settings.rs`: key 12 `disk.extra_copy_roots` (`Key::ExtraCopyRoots`, slot 11,
  `KEYS` 11 -> 12), `COPY_ROOT_SEPARATOR` `|`, `MAX_COPY_ROOTS_BYTES` (= the settings route's own
  4,096-byte `secret_shape` ceiling, an existing project number; no count of roots is typed).
  `copy_roots()` is the one reader: absolute, no `..`, no empty piece, none twice, none nested,
  checked with an ordered set (O(r log r)). `check` adds that each folder exists; `check_stored`
  holds a stored value to its shape only. Test
  `the_extra_copy_roots_key_is_checked_and_stored_canonically`: passes.
- `crates/api/src/settings_route.rs`: the two required arms (`default_of`: `(None, UNSET_WAIT)`;
  `reader_of`: `Ok(...)` naming `pull::copies_register`, `GET /disk/copies.json`, `cli disk-copies`).
- `crates/pull/src/copies_register.rs` (new) and `copies_register_tests.rs` (new, 24 tests, all
  pass): the read-only `Fs` seam (`lstat`, `list`, `lines` through `store::safe_open`; no writing
  method exists), `config()`, `reader_of()` (D-7044), a walk that enters every directory once by
  `(st_dev, st_ino)` on `available_parallelism` workers (no typed entry, depth, list or thread
  count: the operator's 24 Oct rule), `(st_dev, st_ino)` hazards within one walk, streamed receipts
  (`ALL_VERIFIED.txt`, `VERIFIED.tsv` matched by inode, size, mtime and ctime, never `st_dev`;
  `*.prove.out` with `ALL-PROVED`), twins, the retirement list and its text, the events, the cache
  (`answer`: re-walk only when the roots or a root's own stat change; no request waits on
  another's walk), `total()` for R4-29.

- `crates/api/src/copies_route.rs` (new) and `copies_route_tests.rs` (new, 5 tests, all pass):
  `GET /disk/copies.json` in the ops-read pool; unset key = `200 waiting` naming the key; the first
  walk since start = `503 walking` to any other request, never a wait; the register body rendered
  once per walk and joined to each request's own head (`age_ms`, `fresh`, `why`, `rewalking`).
  Wired in `server.rs` (one `mod` line, one `.route`), `operation_audit_tests.rs` (EXEMPT, DATA)
  and `pool_table_tests.rs` (OpsRead row).

- `crates/cli/src/disk_copies_command.rs` (new, 3 tests, all pass): `cli disk-copies`, no arguments,
  always a fresh walk; exit 1 on a hazard, a root not walked whole, the key unset or a refused
  store root; exit 2 on any argument. Wired in `crates/cli/src/lib.rs` (`mod`, one dispatch arm,
  `COMMANDS` 62 -> 63, one usage entry, the zero-argument exemption), `store_write.rs` (reader
  list) and `sweep_wiring_tests.rs` (non-sweep list).
- `crates/pull/tests/no_scheduler_delete.rs` (R4-06): `DECLARED` 12 -> 15 and `LISTED` 4 -> 7 with
  `copies_register.rs`, `copies_route.rs` and `disk_copies_command.rs`; its 5 tests pass.

- The /ingest panel (written by a parallel subagent, reviewed): `web/src/routes/ingest/panels/Copies.svelte`
  (order 95, through R4-01's registry; hazards red; the retirement list downloadable as text),
  `web/src/lib/copies.js`, `web/tests/copies.test.js` (10 tests), and one dev-proxy line in
  `web/vite.config.js` (`web/tests/proxy.test.js` requires every fetched route to have one).
- The bench `crates/api/benches/ratio/copies.rs` (`/disk/copies.json` at 1x/10x/100x files, n = 200,
  after the settings rows) and its three lines in `crates/api/benches/ratio.rs`.
- `docs/02-store-format.md` §42 row, `docs/04-invariants.md` CC5-01..CC5-16,
  `docs/05-decisions.md` D-7040..D-7048.

## Left

- `docs/06-limits.md` L-CC5 rows with the measured figures; the final merge of `origin/feat/db-fix`;
  every gate (fmt, clippy, the workspace tests, deny, coverage, mutants, the CI shell gates).

## Log

### 2026-10-24 — start

- GO present, HOLD absent, spec present. Branch cut from `e851da1ca`.
- Gate tools: `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants` 27.1.0 installed from
  crates.io (GitHub release downloads answer 403 through the proxy).

### 2026-10-24 — save point 1 (HELD)

- Files: `crates/store/src/settings.rs`, `crates/store/src/settings_tests.rs`,
  `crates/api/src/settings_route.rs`, `crates/pull/src/lib.rs`, `crates/pull/src/copies_register.rs`,
  `crates/pull/src/copies_register_tests.rs` (header only).
- `cargo clippy -p pull -p store --locked`: no warning, no error.
- Commit `cca93bc4`, pushed.
- `HOLD` found on `origin/cloud/db-base-20261024` at this save point: stopped here, waiting for
  `GO` with no `HOLD`.
- NEXT: write `crates/pull/src/copies_register_tests.rs` (the eight named tests first, failing
  before the module, then the extreme cases), run `cargo test -p pull -p store --locked`.

### 2026-10-24 — resumed, save points 2 and 3, then HELD again

- 12:37 IST the Mac lifted HOLD ("activate all the sessions"); work resumed from NEXT.
- The README's new section (the operator's words of 24 Oct, "no typed number outside the Winner
  Gate") was applied: removed `MAX_COPY_ROOTS`, the walk's entry and depth caps, the list caps, the
  receipt read cap and the statement cap. Commit `ad16891c`.
- Wrote `crates/pull/src/copies_register_tests.rs`: the eight named tests and 16 extreme cases.
- `cargo test -p pull --lib --locked copies_register`: `test result: ok. 24 passed; 0 failed`.
- `cargo test -p store --lib --locked settings`: `22 passed; 3 failed`. The three
  (`a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`,
  `a_disk_that_refuses_the_write_is_refused_with_nothing_visible`,
  `a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`) are permission-bit tests;
  this container runs as root, which ignores permission bits. To be shown red on the unmodified
  base in the final gate run (not yet shown).
- `cargo clippy -p pull -p store --all-targets --locked`: no warning.
- `HOLD` back on the base at 12:55 IST ("HOLD for parts without GO-<NAME>"); no `GO-CC-5`. Stopped.
- NEXT: the api route `crates/api/src/copies_route.rs` + `copies_route_tests.rs`, its two
  `server.rs` lines, its rows in `operation_audit_tests.rs` and `pool_table_tests.rs`.

### 2026-10-24 — resumed after the 17:30 reset, save point 4, then HELD again

- 17:30 IST HOLD lifted ("every cloud part builds"). Merged the newest `origin/feat/db-fix`
  (`7b397bbe`, Merge CLOUD-MUTCLI) into the branch: clean, merge commit `c03ceb09`.
- Wrote `crates/api/src/copies_route.rs` and `copies_route_tests.rs`; wired `server.rs`,
  `operation_audit_tests.rs`, `pool_table_tests.rs`.
- `cargo test -p api --lib --locked copies_route`: `test result: ok. 5 passed; 0 failed`.
- `cargo test -p api --lib --locked -- pool_table operation_audit settings_route`:
  `test result: ok. 55 passed; 0 failed`.
- `cargo clippy -p api --all-targets --locked`: one test warning (`manual_assert_eq`), fixed.
- `HOLD` back at 18:15 IST ("HOLD for parts without GO-<NAME>"); no `GO-CC-5`. Stopped.
- NEXT: the cli verb `crates/cli/src/disk_copies_command.rs` (zero arguments; a fresh walk; exit 1
  on a hazard, a wait or a root not walked whole), its dispatch arm, `COMMANDS` 60 -> 61, its usage
  line, `store_write.rs` reader list, `sweep_wiring_tests.rs` non-sweep list, the zero-argument
  exemption in `a_known_command_with_the_wrong_arity_is_not_called_unknown`; then R4-06's
  `no_scheduler_delete.rs` rows.

### 2026-10-24 — GO-CC-5 (19:47 IST), save point 5

- `GO-CC-5` appeared on the base: CC-5 builds while HOLD stands for parts without their own GO.
- Wrote `crates/cli/src/disk_copies_command.rs`; wired `cli/src/lib.rs`, `store_write.rs`,
  `sweep_wiring_tests.rs`; added the three files to R4-06's `no_scheduler_delete.rs`.
- `cargo test -p cli --lib --locked -- disk_copies a_known_command every_command_is_listed the_words`:
  `test result: ok. 7 passed; 0 failed`.
- `cargo test -p cli --lib --locked -- disk_copies every_command a_known_command store_write sweep`:
  `test result: ok. 85 passed; 0 failed`.
- `cargo test -p pull --test no_scheduler_delete --locked`: `test result: ok. 5 passed; 0 failed`.
- `cargo clippy -p cli -p pull --all-targets --locked`: no warning.
- NEXT: the /ingest panel (`web/src/routes/ingest/panels/Copies.svelte`, `web/src/lib/copies.js`,
  `web/tests/copies.test.js`, the rebuilt `web/build`), then the bench `crates/api/benches/ratio/copies.rs`.

### 2026-10-24 — save point 6, then HELD (every GO-<NAME> removed, 20:20 IST)

- The /ingest panel by a parallel subagent: `npm ci --prefix web` ok; `npm --prefix web run build`
  exit 0 (the panel compiles); `node --test web/tests/*.test.js` on Node 24 (CI's):
  `tests 1578, pass 1578, fail 0`; on this container's Node 22: `pass 1573, cancelled 5`
  (`ask.test.js` 3, `census-worker.test.js` 2; files the panel does not touch). W4 `0 (ceiling 0)`,
  W5 `No comment swallows a rule`. W3: `svelte-check errors: 32 (ceiling 0)`, all 32 in files this
  part does not touch (0 in the panel's): red before this part.
- **BASE RED, W1: the committed `web/build` is stale on the unmodified base.** Committed
  `version.json` `80659b2a3b706360`; the base's own source builds `fe2953bf00677395` (eight `/db`
  sources changed since R3-03's rebuild of 19 Oct with no rebuild: R3-04 `f68b710f`, R3-30
  `a258ca1c`, among them). The build is reproducible (two builds byte-identical). This part does
  NOT commit `web/build` (rebuilding it would rewrite other parts' output); with the panel the
  bundle builds as `858702fad398a0d9`. NEEDS THE MAC: rebuild `web/build` at merge-back.
- Shell gates (the extracted `run:` blocks of ci.yml's first job, run on this branch and on an
  unmodified worktree of `origin/feat/db-fix` `7b397bbe`): red on BOTH, base reds owned by
  BASE-REDS-1023: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30. On this branch, gates 10, 11, 12, 19, 27b and
  30 add no finding of this part. Gate 1d adds 55 literals of this part's files to the base's 466
  undeclared (declaring them is an edit of `.github/gates_tree.rs`, left with the gate's owner).
  Gate 23: the `println!` of the timing test was removed (no new finding). Gate 28 was red on this
  branch (`"tmp"` in `is_partial`): fixed by reading partial names from the data. Gate 1 was red only
  while the subagent's rebuild had removed tracked bundle files; `git checkout -- web/build`
  restored them. Not yet re-run after these fixes.
- Bench `cargo bench -p api --bench ratio --locked --no-run -j 1`: `Finished bench profile ... in
  19m 58s` (with 4 jobs the release link of `cli` was killed for memory, SIGKILL).
- Commit `da57ae23`, pushed.
- The served bench run started before the hold (`cargo bench -p api --bench ratio --locked --
  --served-only`) recompiled after the last edit and was killed (SIGKILL: two release links of
  `cli` at once ran out of memory): exit 101, NO figure taken.
- NEXT: build the bench with `-j 1`, then read the served bench's copies rows (`cargo bench -p api --bench ratio --locked -j 1 --
  --served-only`, started before the hold) and write `docs/06-limits.md` L-CC5 rows; re-run the
  shell gates; merge the newest `origin/feat/db-fix`; then fmt, clippy, the workspace tests, deny,
  coverage (nightly installed for `--branch`) and mutants on the touched modules.

### 2026-10-25 — resumed (GO, HOLD lifted 13:1x IST)

- New container; `HOLD` found at start, polled; `GO` with no `HOLD` (base `1dcc33e9`, "Lift HOLD: all
  nine cloud parts build") at the first poll.
- Merged the newest `origin/feat/db-fix` (`666ddf7f`, Merge DATA-1w): two both-sides-added conflicts
  (`docs/04-invariants.md`: CC-5's section beside R3-13's; `docs/05-decisions.md`: D-7040..D-7048
  beside D-7300), resolved by keeping both whole.
- After the merge: `cargo build --workspace --locked --all-targets`: no warning, no error.
  `cargo fmt --all --check`: exit 0. `cargo clippy --workspace --all-targets --locked -j 2 -- -D warnings`:
  `Finished`, no warning.
- Shell gates re-run (the `run:` blocks of ci.yml's `language-purity` job, `GITHUB_ENV` emulated so
  gate 0's `SOURCE_SCAN` reaches the rest; gate 1e not run here, it builds a second worktree), on
  this branch and on an unmodified worktree of `origin/feat/db-fix` `666ddf7f`: red on BOTH, the same
  nine, base reds: 1c, 1d, 10, 11, 12, 19, 23, 27b, 30. Findings diffed line by line (paths and
  counts normalised): 1c, 10, 30 identical; 11, 12, 19, 23, 27b differ only in their scanned-line and
  file counts (12's REFUSED list identical modulo line numbers); 1d: 521 undeclared on this branch
  against 466 on the base, the 55 literals of this part's files (left with the gate's owner, as
  recorded at save point 6). Every other gate (0, 1, 1b, 1g, 1f, 2, 9, 9b, 10b, 7, 13, 15, 16, 17, 21,
  22, 24, 28, 25, 27, 26, 14, 29, 29b): exit 0 on this branch.

### 2026-10-25 — new container, save point 7 (GO, no HOLD)

- New container (no target, no gate tools): reinstalled `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1,
  `cargo-mutants` 27.1.0 from crates.io; nightly with `llvm-tools-preview` for `--branch`.
- Merged the newest `origin/feat/db-fix` (`bf9b1c8f`, Merge R4-18): textually clean, merge commit
  `b8f3b34f`. R4-18 added its own verb (`zerodha-quarantine`), so `COMMANDS` is now `[&str; 64]`
  (62 base + R4-18 + CC-5); commit `52718be3`, pushed.
- `cargo build --workspace --locked --all-targets -j 4`: `Finished`, exit 0, no warning.
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`: `Finished`, exit 0.
- `cargo deny check`: `advisories FAILED, bans ok, licenses ok, sources ok`; the one error is
  RUSTSEC-2026-0386 (tokio 1.53.1), the BASE item BASE-DENY-1024 owns (Cargo.lock is the Mac's).
- `cargo test --workspace --locked -j 4 --no-fail-fast`: exit 101, 6 targets red, 31 tests:
  29 failed and 2 hung (killed after 15 min each). EVERY ONE is red on an unmodified worktree of
  `origin/feat/db-fix` `bf9b1c8f` too (its test binaries built and run with the same `--exact`
  names, `--test-threads=1`): the same 29 FAILED, and the 2 hung (`timeout 240`, exit 124). None
  is in a file of this part. They are permission-bit tests (this container runs as root), the
  slow-reader socket tests and two full-pool child tests:
  - api lib (8 + 1 hung): `detail::tests::a_refused_admission_names_what_its_bound_and_why`,
    `pullrun::restart_tests::the_journals_own_extremes`,
    `server::feeds_route::tests::the_window_is_refused_loudly_when_degraded_or_inverted`, the five
    `server::head_deadline_body_tests::*`; HUNG
    `server::gaps_route::cloud_mutant_tests::a_full_calendar_pool_refuses_the_census_and_the_audit`
    (its child waits on a barrier; gdb: the body's `Release` guard waiting in `drop`).
    (`logs::r3_24_tests::a_two_week_old_refusal_is_found_by_run_day_symbol_and_code` failed under
    the full parallel run only, and passes alone on both.)
  - api `--test cloud_mutants_bars` (1 hung):
    `spot_rs::spot_json_is_refused_by_name_when_every_store_read_slot_is_held`.
  - cli lib (3): `scrub_blocks::tests::an_empty_census_a_single_record_and_an_unwritable_ledger`,
    `watchdog_command::tests::a_read_only_state_directory_sends_no_notice_at_all_rather_than_many`,
    `watchdog_command::tests::a_run_never_lists_the_notices_or_reads_the_log`.
  - pull lib (9): `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`,
    `gdfl_cm::tests::fixtures_are_built_not_pasted`, `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`,
    `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`,
    `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`,
    `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`,
    `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`,
    `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`,
    `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named`.
  - store lib (5): `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`,
    `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`,
    and the three `settings::tests::*` permission tests named at save point 3 (now SHOWN red on the base).
  - telemetry lib (4): `sink::tests::a_coverage_file_is_read_line_by_line_and_extended_where_it_stands`,
    `sink::tests::a_failed_delete_is_counted_named_and_left_for_the_next_start`,
    `sink::tests::a_rename_aside_that_cannot_happen_fails_the_roll_by_name`,
    `sink::tests::an_audit_stream_that_cannot_be_looked_for_is_an_error`.
  Every test of this part's files passes in that run (`copies_register::` 25 tests ok, `copies_route` 5,
  `disk_copies_command` 3, `settings::tests::the_extra_copy_roots_key_is_checked_and_stored_canonically`,
  `no_scheduler_delete` 5).
- NEXT: the served bench (`cargo bench -p api --bench ratio --locked -j 2 -- --served-only`, running),
  its copies rows into `docs/06-limits.md` L-CC5; then coverage and mutants on the touched modules
  (disk is 11 GB free: target/ is cleared between them).

### 2026-10-25 — after save point 7 (commits the earlier session pushed without a log entry)

- `9b03bdd3`: `docs/06-limits.md` L-CC5-1..4 with the gate-8 bench's measured figures (the served
  bench, `cargo bench -p api --bench ratio --locked -j 2 -- --served-only`; figures in the rows).
- `7fdc5408`: a coverage run found reachable lines of `copies_register.rs` with no test; 11 tests
  added; `answer()` asks the walk lock (never waiting) before it compares the held answer.
- `532b9fe0`: tests for the unreached lines of `copies_route.rs` and `disk_copies_command.rs`.
- `5233e81b`: `cargo mutants` found `replace > with >= in copy_roots` surviving; a test holds the
  byte ceiling exactly (shown red with the mutant applied by hand, green without).

### 2026-10-25/26 — new container, save point 8 (GO, no HOLD; base `fe6d379e`)

- New container again: reinstalled `cargo-deny` 0.20.2, `cargo-llvm-cov` 0.9.1, `cargo-mutants`
  27.1.0 from crates.io, nightly with `llvm-tools-preview`.
- Merged the newest `origin/feat/db-fix` (`13dcf5e5`, CLOUD-MUTAPIB merge fix): textually clean,
  merge commit `9e6f55fc`, pushed.
- `cargo build --workspace --locked --all-targets -j 3`: `Finished`, no warning.
- `cargo fmt --all --check`: exit 0.
- `cargo clippy --workspace --all-targets --locked -j 4 -- -D warnings`: `Finished`, no warning.
- This part's tests after the merge: `copies_register` 35 passed; `copies_route` 8 passed;
  `cli -- disk_copies every_command a_known_command store_write sweep` 87 passed;
  `store settings::tests` 22 passed, 3 FAILED (the three root-permission tests shown red on the
  base at save point 7; `the_extra_copy_roots_key_is_checked_and_stored_canonically` ok);
  `no_scheduler_delete` 5 passed; `api -- pool_table operation_audit settings_route` 55 passed.
- `cargo test --workspace --locked -j 4 --no-fail-fast -- --skip a_full_calendar_pool_refuses_the_census_and_the_audit --skip spot_json_is_refused_by_name_when_every_store_read_slot_is_held`
  (the two skipped hang on the base, save point 7): `EXIT=101`; red targets: api lib (12 failed of 2,945),
  cli lib (29 of 2,406), cli `--test gdfl_run_logs` (2 of 4), pull lib (12 of 2,108), store lib (5 of 431),
  store `--test sealed_door` (1 of 1), telemetry lib (4 of 225). None is in a file of this part.
  The 24 named at save point 7 recur (shown red on the base there). NEW since the merges of
  `13dcf5e5` (CLOUD-MUTAPIB and the parts before it), NOT YET shown red on an unmodified base:
  - `a_run_whose_sink_is_held_refuses_before_any_lock`
  - `audit::cloud_mutant_tests::an_unreadable_journal_refuses_the_index_build_and_says_so`
  - `both_second_formats_answer_the_same_bars_at_p99`
  - `gdfl_import_command::cloud_mutant_tests::the_receipt_names_the_workers_and_durability_it_filed_with`
  - `gdfl_import_command::guard::tests::a_change_is_never_accepted_by_a_run_that_skipped_no_day`
  - `gdfl_import_command::guard::tests::a_changed_archive_with_every_day_done_is_named_not_ok`
  - `gdfl_import_command::guard::tests::a_run_that_read_no_file_is_not_accepted_by_running_it_again`
  - `gdfl_import_command::guard::tests::a_source_root_that_vanishes_after_files_were_read_is_named_source_lost`
  - `gdfl_import_command::guard::tests::an_empty_range_owes_nothing_and_exits_ok`
  - `gdfl_import_command::guard::tests::every_day_already_refused_identically_still_fails_when_the_root_is_gone`
  - `gdfl_import_command::guard::tests::one_archive_reached_by_another_spelling_is_the_same_key`
  - `gdfl_import_command::tests::a_dangling_day_file_link_ends_the_run_failed_never_a_day_not_held`
  - `gdfl_import_command::tests::a_named_failure_or_a_refused_run_exits_failed`
  - `gdfl_import_command::tests::a_run_says_a_line_a_day_and_leaves_its_progress_record`
  - `gdfl_import_command::tests::a_run_that_read_no_file_never_exits_ok`
  - `gdfl_import_command::tests::an_unstamped_build_is_refused_a_store_write_unless_allowed`
  - `gdfl_import_command::tests::the_retry_flag_is_never_a_symbol_and_the_run_line_names_the_source`
  - `gdfl_import_command::tests::the_stored_flag_narrows_a_capital_market_run_to_what_d3333_adds`
  - `gdfl_run::tests::a_clock_step_back_never_aborts_a_run`
  - `gdfl_run::tests::a_progress_file_that_cannot_be_written_is_named_and_the_run_goes_on`
  - `gdfl_run::tests::the_kinds_run_in_order_on_the_calling_thread`
  - `pullrun::cloud_mutant_tests::newest_records_that_cannot_be_read_leave_the_answer_not_known`
  - `server::cloud_mutant_tests_c::a_store_that_takes_no_write_trips_the_store_breaker`
  - `server::cloud_mutant_tests_c::a_store_whose_audit_dir_cannot_be_made_is_at_fault`
  - `tick_second_check::tests::a_lock_name_that_is_a_link_a_directory_or_a_second_name_is_refused_and_nothing_goes_through_it`
  - `tick_second_check::tests::a_month_held_only_sealed_is_checked_row_for_row_as_its_raw_form_was`
  - `tick_second_check::tests::a_new_landing_of_a_checked_day_is_checked_again`
  - `tick_second_check::tests::a_pipe_or_a_link_at_one_of_the_checks_own_names_and_a_pipe_archive_are_refused_at_once`
  - `tick_second_check::tests::a_share_directory_the_store_refuses_is_one_cell_and_files_nothing_twice`
  - `tick_second_check::tests::a_share_whose_months_cannot_be_listed_is_filed_month_by_month_with_or_without_a_year`
  - `tick_second_check::tests::a_tick_store_landing_is_checked_against_that_tick_store_day_and_a_changed_day_is_refused`
  - `tick_second_check::tests::a_version_whose_rows_are_not_the_ones_its_index_names_is_refused`
  - `tick_second_check::tests::an_extra_archive_is_compared_beside_the_landing_and_labelled_not_the_landing_source`
  - `tick_second_check::tests::every_status_and_source_is_named_against_the_landing_the_store_recorded`
  - `tick_second_check::tests::no_worker_count_and_no_resumption_changes_a_byte_of_the_version`
  - `two_cli_kinds_at_once_both_log`
  Several are permission or read-only tests (this container runs as root); `cli` lib took 724 s
  under the parallel run. To be shown red on an unmodified worktree of `origin/feat/db-fix`.
- `HOLD` on the base with no `GO-CC-5` (fetched at the end of this run): stopped here.
- NEXT: build an unmodified worktree of `origin/feat/db-fix` and run the NEW reds above by
  `--exact` name with `--test-threads=1` (each one red there too, or a fix here); then
  `cargo deny check`; coverage (`cargo +nightly llvm-cov --branch`) and `cargo mutants --in-diff`
  on the touched modules (clear `target/` between, 11 GB free); the CI shell gates; the final report.
