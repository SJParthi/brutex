STATUS: BLOCKED

# IA-3b (cloud): the import preflight refusals IA-3 left

Branch `cloud/db-IA-3b-20261024`, cut from `origin/feat/db-fix` e851da1c (Merge DATA-2). The
newest `origin/feat/db-fix`, 1c094c90 (Merge IA-4, with Merge R4-27 before it), was merged in
before the final gates (merge commit fa6aeef7). The spec is
`reports/db-specs-20261024/IA-3b.md` on `origin/cloud/db-base-20261024`.

HOLD appeared at 5dbfdf3f and the part was held (commit c1849952). It resumed under its own
`GO-IA-3b` (f20c2d35). The per-part test in the README passed at every later save point,
including the last.

## Why the status is BLOCKED, not DONE

The part's code, tests, bench and ledger rows are complete and pushed. What is not green is
outside it, or could not be run in this container:

1. **`cargo deny check` is red on the base.** `advisories` fails on RUSTSEC-2026-0386
   (tokio 1.53.1, `AsyncFd` I/O safety, "unsound"); `bans`, `licenses` and `sources` are ok.
   This branch's `Cargo.lock` is byte-identical to `origin/feat/db-fix`, so the base is red the
   same way. The advisory arrived after BASE-REDS-1021. Its fix (`cargo update -p tokio
   --precise 1.53.3`, a decision, and the offline floor in
   `crates/vocab/tests/workspace_is_rust.rs`, as D-7120 did) is a base package, not IA-3b.
2. **Nine CI shell gates are red on the base** with the same exit code here, and this part adds
   no refusal to seven of them: 1c, 10, 11, 12, 19, 27b and 30. Two gain lines in classes the
   base already fails on: gate 1d (466 undeclared literals on the base, 536 here; all 70 new
   words invented) and gate 23 (one test file with 6 `println!` in ignored measurement tests).
3. **`cargo test --workspace` never finished in one run here:**
   - `api` (byte-identical to `origin/feat/db-fix`): 10 failures plus two hangs, each named
     under Gates.
   - The workspace run then hit its timeout.
   - A rerun without `api` died when the 252 GB disk's writable allowance ran out (`target/debug`
     is 27 GB).
   - The touched crates, `pull` and `cli`, were then run in full on a tmpfs: green apart from
     12 environment failures, listed under Gates.
4. **Coverage and mutants were not run:** `cargo llvm-cov` and `cargo mutants` are not
   installed, and their builds do not fit (M12).
5. **The disk.** The container's writable allowance is spent (0 bytes free), and the operator's
   rule forbids deleting target directories to make room. The last commits were therefore made
   from a clone on `/dev/shm` and pushed from there. The local checkout at `/home/user/brutex`
   stays behind origin, with the same code in its working tree. One file was emptied to let the
   shell capture output again: my own byte-identical copy of a test log
   (`scratchpad/wstest-api.log`, a copy of `wstest.log`); nothing else was deleted or truncated.
   For the main session: `/home/user/brutex/target` (27 GB) and the two scratch worktrees under
   the session scratchpad (`base`, `g10wt`) are left in place.

## What is done

Everything the spec's items (1) to (10) ask is built and tested.
`pull::gdfl_run::plan` calls `preflight::run` once, after the scope is resolved and before any
lock, claim or journal line, so `cli gdfl-import` and the api's child plan the same way.

| Item | Refusal or behaviour | Where |
|---|---|---|
| 1 | `year_zip_missing{year, near_miss}` (at most 4, in name order), `source_unreadable{path, mode}`, and `year_zip_empty{year}` (the end record's count, read from the tail alone). A root with no year zip at all is AF-01's `source_root_unknown`. A root whose zips are all of other years, with no name like the range's, is AF-01's `no_file_read` (FM-060). See D-7000. | `year_zips`, `year_zip`; `gdfl_archive::stated_entries` |
| 2 | An empty day zip nested in the CM archive or in an options year zip, and an empty tick-store day index, refuse the day `day_zip_empty{name}` (`Cause::open`; the day stays open). `name` is the vendor day zip, e.g. `GFDLNFO_TICK_01042024.zip` | `gdfl_archive.rs`, `gdfl_nfo.rs`, `gdfl_tickstore.rs` |
| 3 | `member_too_large{name, size}` (R4-08's code; the one `MEMBER_CAP`) before any byte is read: `member_bytes` on `len.max(packed)`, `rebuild` on `size.max(len)` | `gdfl_archive.rs`, `gdfl_tickstore.rs` |
| 4 | `source_changing{path, before, after}` after every listing and member read. The identity is len, inode, mtime and ctime, never st_dev, from the held handle and the path. `source_busy{root, holder}` comes from the tick-store writer's `.lock`, probed shared and non-blocking and never created | `Watch`, `SourceIdentity`, `writer_lock` |
| 5 | `census_room_short{layout, free_now, free_after, need_first_day}` from two header slot reads, with the v4 compaction modelled by `epoch_capacity`. `census_compaction_no_disk{need_bytes, free_bytes, floor}` through R4-07's `floor_check` at the run's floor. `census.projected_stop` is labelled an extrapolation. An unreadable census layout is the door's run-lock refusal (D-7000) | `room`, `census_header`, `projected_stop` |
| 6 | `legacy_run_may_be_live{key}`: per kind on v4, any kind on v3. "Has claimed" is the lock's existence (D-4908). `--legacy-run-not-live` reaches the request and the claim, which records `legacy_ack=1`; the request line carries it only when given | `legacy`; cli `LEGACY_RUN_NOT_LIVE` |
| 7 | `store_read_only{root}` (ends in IA-16's fix), `store_not_local{root, fs}`, `open_files_short{have, need}`; one `statfs`/`statvfs` and `getrlimit` through rustix. `open_files_exhausted` comes from the store error's EMFILE number | `Os`, `writable`, `ingest_failkind.rs` |
| 8 | Each source's span is on the plan line; the asked days outside it are counted `outside_source`, never as holes | the four `span`s |
| 9 | A `FORMAT.md` digest outside the charter's two is never refused (the magic decides, D-4819). It is a Warn event `format_digest_unpinned` on the log, `format=unpinned` on the plan line, and `format_digest_unpinned=1` on the run line | `Format`, `check_with`, `gdfl_import_journal.rs` |
| 10 | The module doc names R4-11 for loose day zips (D-7005, amending D-4815) | `gdfl_run.rs` |

The plan's lines (one per kind) go to the log as one info event, `preflight passed`, or as an
error event, `preflight refused`. The cli prints none of them: the spec allows only the flag and
`ClaimAsk` there.

Tests: 35 in `gdfl_run_preflight_tests.rs`, 31 run and 4 ignored. The ignored four are the
read-only-volume test (M8), the descriptor measurement and the two real-data tests (M1, M4). All
21 failing-first names of the spec's section 3.1 exist and pass. There is also one cli test
(`the_legacy_flag_reaches_the_request_and_lets_the_run_past_the_cutover`), and the 10 new codes
are in IA-1's code-list test.

## Gates

| Gate | Command | Result |
|---|---|---|
| fmt | `cargo fmt --all --check` | green on the final tree |
| clippy | `cargo clippy --workspace --all-targets --locked -- -D warnings` | green on the merged tree (fa6aeef7). After the last two commits, which changed only `pull` and `cli`: `cargo clippy -p pull -p cli --all-targets --locked -- -D warnings` green |
| test, `pull` | `cargo test -p pull --locked` (on tmpfs, final tree) | lib 2013 passed, 9 failed; every other target green. The 9 are the environment set: `alerts` create_fresh; `gdfl_cm::tests::fixtures_are_built_not_pasted` (it flags a cli file); `gdfl_dropbox` repair_r1 x2; IA-16's immutable month; `census_store` link_tests x2; `zerodha_verify` x2. All 9 are among the 12 that fail identically with the base's `crates/pull` (0 passed, 12 failed); the other 3 are disk-floor tests, which pass on the 16 GB tmpfs |
| test, `cli` | `cargo test -p cli --locked` (tmpfs, final tree) | lib 2355 passed, 3 failed; every other target green. The 3 are permission tests that root defeats, in modules this part does not touch (`scrub_blocks` an unwritable ledger; `watchdog_command` a read-only state directory, an unreadable log) |
| test, other crates | `cargo test -p vocab -p core -p store -p telemetry -p costs -p greeks -p indicators -p engine -p lake -p runner --locked` (tmpfs, final tree) | 3394 passed, 13 failed (126 test binaries). 5 are git-history tests that refuse a shallow clone in their own words (`core` `every_named_commit_exists`, `..._is_on_main`, `..._is_in_this_branchs_history`, `d_0693_lists_each_bracket_site_at_a_commit_where_its_line_takes_a_lock`, `every_commit_the_store_records_cite_is_on_main_or_says_why_not`): this tree was a `--depth 30` clone. 7 are permission fault-injection tests that root defeats ("this host reads a 000 directory (root?)"), in `store` (`settings` x3) and `telemetry` (`sink` x4), which this part does not touch. 1 was this part's: `core::decision_citations` found D-7009 cited in two doc comments and never written (only D-7000 to D-7007 are). Fixed, and that test now passes |
| test, `api` | `cargo test -p api --locked` (merged tree; `crates/api` byte-identical to `origin/feat/db-fix`) | lib 2461 passed, 10 failed. Environment: a root process reads a mode-000 directory (`feeds_route`); five slow-reader timing tests cut early (`head_deadline_body_tests`); a 4-slot admission pool saturated by parallel tests (`detail`); the audit-history clock (`logs::r3_24_tests`). Not shown to be environment: `pullrun::restart_tests::the_journals_own_extremes` (`interrupted` against `not_known`); it is the vendor pull's journal, which this part does not touch, and NOT PROVED either way here. Two tests hang here: `gaps_route::...a_full_calendar_pool_refuses_the_census_and_the_audit` (its child sat 47 minutes at 0% CPU and was killed) and `tests/spot_rs.rs` `spot_json_is_refused_by_name_when_every_store_read_slot_is_held` (ran into the timeout). Re-run on the Mac (M12) |
| deny | `cargo deny check` (cargo-deny 0.20.2, installed here) | red: `advisories` (RUSTSEC-2026-0386, tokio 1.53.1); bans, licenses, sources ok. The same `Cargo.lock` as the base |
| CI shell gates | the 33 `language-purity` gates of `.github/workflows/`, run by `checks_all` on the final tree and on `origin/feat/db-fix` 1c094c90 | 24 green; the same 9 red on both (1c, 1d, 10, 11, 12, 19, 23, 27b, 30), with no new refusal from this part except gate 1d's literals and gate 23's prints, both above. Gate 10 stops at R4-27's row R427-11 on both. With that one row set aside in a scratch copy, every row of this part resolves; only HA-02 and AG05-13 (not this part's) remain missing |
| bench | `cargo bench -p pull --locked --bench ratio` | green: "all ratios within the ceiling", EXIT 0 (C-GR-05 below) |
| coverage | `cargo llvm-cov --branch` | NOT RUN: not installed, and the build does not fit (M12) |
| mutants | `cargo mutants --in-diff` | NOT RUN: not installed, and the build does not fit (M12) |

## Measurements (p50, p99, max)

C-GR-05 (`cargo bench -p pull --locked --bench ratio`, release, 24 Oct, in this container: 4
shared cores, Linux, root). The inputs are invented: day files written by the tick-store writer
itself, and the census as a header over a sparse file.

| What | n | p50 | p99 | max |
|---|---|---|---|---|
| the checks alone, 10,000 entries | 200 | 31.9 µs | 53.6 µs | 62.3 µs |
| the checks alone, 100,000 entries | 200 | 32.0 µs | 53.3 µs | 70.0 µs |
| the checks alone, 1,000,000 entries | 200 | 32.1 µs | 73.4 µs | 99.1 µs |
| repeated plan, 10,000 / 100,000 / 1,000,000 | 200 each | 1.03 / 1.02 / 1.01 ms | 3.31 / 1.21 / 1.16 ms | 4.97 / 1.44 / 1.24 ms |
| first (cold) plan, timed once apart | 1 each | 1.49 / 1.30 / 1.25 ms | - | - |
| ratio of the checks at 10x / 100x (ceiling 3x) | - | 0.999x / 1.002x | - | - |
| the one listing read, 26,820-entry options day (O(entries), L-IA3B-03) | 200 | 11.8 ms | 15.2 ms | 18.9 ms |
| the `source_changing` check, two stats (L-IA3B-01) | 2,000 | 721 ns | 939 ns | 23.3 µs |
| the legacy refusal path, 100,000 journal lines (O(lines), L-IA3B-02) | 20 | 50.7 ms | 66.3 ms | 82.2 ms |

Peak open descriptors (the ignored test
`the_descriptors_a_run_holds_are_measured_at_one_and_four_workers`, 400 invented files, three
runs): 32 at W=1, 32 at W=4 and 36 to 37 at W=12, beyond the 3 held before. Every peak is at or
under `files_needed(W)` = 32 + 5W (92 at W=12).

All of this is in `docs/06-limits.md`, section "The import preflight IA-3 left": C-GR-05,
L-IA-35 and L-IA3B-01 to L-IA3B-07.

## Extreme cases (18; 3 combinations, 3 out of the box)

1. A misspelt year zip beside a `.DS_Store` and other years' zips: only names that look like the
   year are near misses (`a_missing_or_misspelt_year_zip_is_refused_with_its_near_misses`).
2. Many look-alikes of one year: capped at 4, in name order
   (`at_most_four_near_misses_are_named_in_name_order`).
3. A range across New Year needs both years' zips (`a_range_across_new_year_needs_both_year_zips`).
4. A root holding only other years' zips is left to AF-01's `no_file_read`, and one name like
   the year is enough to refuse it (`a_root_with_no_year_zip_at_all_is_left_to_the_door`).
5. A year zip of zero bytes, or one stating zero entries
   (`an_empty_year_zip_is_refused_before_any_line`). Its count comes from the tail alone: 1 and
   2,000 members cost the same reads (`the_stated_entry_count_reads_the_tail_alone`).
6. An empty day zip in both vendor containers, and an empty tick-store index: the day is refused
   and stays open, and the next day closes (`an_empty_nested_day_zip_refuses_its_day_and_stays_open`,
   `an_empty_tick_store_day_refuses_its_day_and_stays_open`).
7. Members stating `MEMBER_CAP + 1`, `u64::MAX` packed and unpacked, and exactly the cap: zero
   bytes read before the refusal, and the cap itself passes
   (`a_member_stating_more_than_the_cap_is_refused_before_any_read`,
   `a_tick_store_entry_rebuilding_past_the_cap_is_refused`).
8. **Combination**: a zip still being copied while two runs read it. No day read after it grew
   is closed; once whole, the next run closes both (`a_zip_that_is_still_being_copied_is_never_closed`).
9. A year zip rewritten in place between listing and read
   (`a_year_zip_rewritten_between_listing_and_read_is_source_changing`).
10. **Out of the box**: a tick-store day file replaced by rename mid-read with the same length
    AND mtime. Only the inode and ctime tell it apart
    (`a_tick_store_day_file_replaced_by_rename_mid_read_is_source_changing`,
    `the_source_identity_is_length_inode_and_both_times_only`).
11. A held tick-store writer lock is `source_busy`, and the probe never creates it
    (`a_held_tick_store_lock_is_source_busy_and_never_created`).
12. **Out of the box**: a torn census header is read at its older slot; when neither decodes it
    is refused by name (`a_torn_census_header_is_read_at_its_older_slot_or_refused_by_name`).
13. **Combination**: census room and the disk at once. Only the compaction gives room, and its
    epoch does not fit above the floor; one byte more and it plans
    (`a_compaction_the_disk_cannot_hold_is_refused_before_any_line`).
14. **Combination**: read-only AND network AND short of descriptors at once, and a missing year
    zip on such a store, are refused in the stated order
    (`refusals_come_in_their_stated_order_when_several_hold_at_once`).
15. The open-file limit at its boundary for every worker count 1 to 12: `need - 1` is refused,
    `need` passes (`a_run_under_an_open_file_limit_of_256_is_refused_open_files_short`).
16. EMFILE from the store's own error number is `open_files_exhausted`
    (`an_emfile_during_a_run_is_named_open_files_exhausted`).
17. The legacy cutover is per kind on v4 and across kinds on v3
    (`the_first_claim_asks_about_a_legacy_run`). The probe never creates a lock, and a store that
    holds one is never asked again (`the_legacy_probe_never_creates_a_lock_file`). The
    acknowledgement crosses the request line only when given
    (`the_acknowledgement_crosses_the_request_line_only_when_given`).
18. **Out of the box**: a `FORMAT.md` digest outside the charter's two is warned and marked,
    never refused (`an_unpinned_tick_store_format_digest_warns_and_marks_the_run_line`).

## Failing first

- **Compile red.** Every new test calls this part's new surface: `preflight::*`, the 10
  `RunRefusal` variants, the 3 `CmRefusal` variants, `Request.legacy_ack`, `Plan.checks`,
  `FailKind::OpenFilesExhausted`, `stated_entries` and the spans. None exists at e851da1c.
- **The base's behaviour** (the spec's facts at e851da1c):
  - an empty archive "holds no day" (`Ok(None)`);
  - an empty tick-store index fails as "no zstd frame";
  - `member_bytes` and `rebuild` have no size check;
  - EMFILE is an unnamed `Io`;
  - `plan` has no preflight.
- **Reds seen while building**, each fixed in the code:
  - the rename test, while the watch reported the handle's identity;
  - the growing-zip test, until its fixture held both indices;
  - the R2 and R3 attack models, until both sources named an empty day alike.
- **The cli suite, first run in full late** (my miss: it had been compiled, never run before).
  Fourteen `gdfl_import_command` tests went red on this part's changes. All are green now:
  - The year-zip rule had refused AF-01's FM-060 fixture, a root with only `2019.zip` asked for
    2024. D-7000 now leaves that root to the door.
  - The cli printed the plan's lines, beyond the spec's allowed edit. Removed.
  - An unreadable census layout was refused before the door's own refusal. It is now left to the
    door.
  - Four of AF-01's door tests used empty day and year zips, RA-322's silent case. Their fixtures
    now hold one invented file, marked ADAPTED.
- **Unchanged.** AF-01's two FM-060 tests, `a_wrong_source_root_is_refused_before_any_day_is_journalled`
  and `a_run_that_read_no_file_never_exits_ok`, are unchanged and pass.

## Files outside ownership

| File | Edit | Why |
|---|---|---|
| `crates/pull/src/gdfl_run.rs` | the 10 variants and codes, the `#[path]` mod line, the one call in `plan`, the doc line, `Request.legacy_ack`, `Plan.checks` | sequenced edit, plus item (6)'s field the spec names |
| `crates/pull/src/gdfl_archive.rs` | `member_bytes` cap, empty nested day zip, `stated_entries`, `Watch`, `span` | sequenced edit (items 2, 3, 4, 8) |
| `crates/pull/src/gdfl_tickstore.rs` | `rebuild` cap, empty index, `open_watched_day` (alias `WatchedDay`), spans | sequenced edit |
| `crates/pull/src/gdfl_nfo.rs` | empty nested day zip, watches, spans; the `read_index` import dropped after the merge | sequenced edit |
| `crates/pull/src/gdfl_cm.rs` | 3 `CmRefusal` variants and Display. `SymbolNotInUse`'s arm moved word for word into a free function: the new arms pushed `fmt` past `clippy::too_many_lines`, and the file's own comment gives that remedy | sequenced edit |
| `crates/pull/src/gdfl_import_causes.rs` | 3 `cm_cause` arms (exhaustive), `door_said`'s `OpenFilesExhausted` arm | sequenced edit |
| `crates/pull/src/ingest_failkind.rs` | `FailKind::OpenFilesExhausted` from `StoreError::Io { code: Some(EMFILE) }` | item (7) |
| `crates/pull/src/gdfl_import_filing.rs` | the fix sentence shared through `preflight::NOT_WRITABLE_FIX` | one sentence for `store_read_only` and `store_not_writable` |
| `crates/pull/src/gdfl_import_journal.rs` | `format_digest_unpinned=1` on the run line; `Journal::first_open` | items (9) and (6); not in the spec's list |
| `crates/cli/src/gdfl_import_command.rs` | `--legacy-run-not-live`, `ClaimAsk.legacy_ack` from it, and an `expect(struct_excessive_bools)` with its reason, as `Request` has | sequenced edit, plus clippy |
| `crates/cli/src/gdfl_import_guard_tests.rs` (AF-01's tests) | `held_zip()` instead of an empty zip in four door tests whose subject is the door | ADAPTED (IA-3b, RA-322): the empty containers they used now refuse by design |
| `crates/pull/benches/ratio.rs` | one `#[path]` mod pair and its `ok &=` call printing C-GR-05 | sequenced edit; gate 14's cover line unchanged |
| root `Cargo.toml` | rustix `process` feature, with its reason | sequenced edit (`getrlimit`) |
| `crates/pull/src/gdfl_run_tests.rs` | the 10 new variants in IA-1's code-list test | that test pins every code |
| `crates/pull/src/gdfl_nfo_tests.rs` | WP-16's "indexed once" test proves the one index by `Arc::ptr_eq`; it had moved the zip mid-run, which item (4) refuses by design | adapted, not weakened |
| `crates/pull/src/gdfl_r2_attack_tests.rs`, `gdfl_r3_attack_tests.rs` | the reference models count an empty day as refused (RA-322) | marked ADAPTED in the files |
| `crates/cli/src/gdfl_import_command_tests.rs` | one new test | the cli half of item (6) |

## D-numbers used

D-4802, D-4819, D-4850, D-4864, D-4908 (the design's), and D-7000 to D-7007 from the assigned
D-7000..D-7009. D-7008 and D-7009 are unused.

## NEEDS THE MAC

The spec's M1 to M13, plus M14. Commands and expected answers are the spec's; the test names are
this branch's.

| # | What | Command | Expected |
|---|---|---|---|
| M1 | Every check's answer and time on the real roots | `BRUTEX_GDFL_TICKSTORE=... BRUTEX_GDFL_CM_ARCHIVE=... BRUTEX_GDFL_NFO_ARCHIVES=... BRUTEX_IA3B_REAL_STORE=<store, read only> cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 ia3b_real_preflight_prints_every_check_and_its_time` | every kind and source plans over 2018-09-03..2026-09-24, one `IA3B-REAL` line per check with its time. The test gives the legacy acknowledgement (the real journal's `options 2020-09-16 *` ends in `begin`; M5 proves that refusal) |
| M2 | A range wider than the source | M1's lines, or `cli gdfl-import indices zip 2018-01-01 2018-09-07 NIFTY` on a scratch store | `source_span=2018-09-03..2026-09-24`, `outside_source=N`, never holes; journal unchanged |
| M3 | A year with no zip | `cli gdfl-import options zip 2017-12-01 2018-09-10` on a scratch store | `year_zip_missing` for 2017, no near miss, before any line, lock or progress record |
| M4 | The largest member of each container | `cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 ia3b_real_largest_member_of_each_container` | CM about 15,279,399 B, options about 2,359,609 B, the tick store measured, all under 2 GiB; refused days counted and the first named. Into L-IA3B-05 |
| M5 | The legacy cutover on the real journal | the spec's M5 command | `legacy_run_may_be_live` naming `options 2020-09-16 *`, no lock created; with `--legacy-run-not-live` the claim records `legacy_ack=1`; the time into L-IA3B-02 |
| M6 | Census room on the real v4 census | M1's `census=v4 free_now= free_after= need_first_day=` | no kind refused |
| M7 | The open-file limit | `launchctl limit maxfiles`; a fresh Terminal (soft 256), `cli gdfl-import options zip 2024-04-04 2024-04-04` on a scratch store; measure the cli's own descriptors | `open_files_short` only when need > 256 (92 at 12 workers); the cli's descriptors into L-IA-35 |
| M8 | A read-only store, and the two permission tests as a non-root user | `hdiutil create -size 64m -fs APFS -volname IA3BRO $TMPDIR/ia3b-ro.dmg && hdiutil attach -readonly $TMPDIR/ia3b-ro.dmg`; `BRUTEX_IA3B_RO_ROOT=/Volumes/IA3BRO cargo test --release -p pull --lib -- --ignored --nocapture a_read_only_store_is_refused_before_any_line_on_a_real_read_only_volume`; `hdiutil detach /Volumes/IA3BRO`; `cargo test -p pull --lib an_unreadable_year_zip_is_refused_before_the_run a_read_only_store_is_refused_before_any_line` | `store_read_only{root}` before any line; `source_unreadable{path, mode: "0000"}` |
| M9 | The import unchanged when every check passes | IA-3's `real_days_close_with_the_same_listing_from_two_sources` on feat/db-fix's build and this branch's | identical close lines |
| M10 | Bench C-GR-05 on the Mac | `cargo bench -p pull --bench ratio 2>&1 \| grep -A16 C-GR-05` | ratios within 3x; numbers beside this container's in docs/06 |
| M11 | Shadow-server regression | the spec's M11 commands | as the spec's M11 |
| M12 | The full gate set | `cargo test --workspace --locked` (here `api` hung twice and the disk ran out); `cargo llvm-cov --branch -p pull -p cli`; `cargo mutants --in-diff <this branch against feat/db-fix>` | green; 100% line and branch on the touched modules; no surviving mutant. One mutant expected to survive in the cli: `ClaimAsk.legacy_ack: plan.request.legacy_ack` set to `false`, since the claim is truncated at release and no cli test can read it after the run. Pull's `the_first_claim_asks_about_a_legacy_run` proves the request-to-claim path |
| M13 | Nothing touched the real store | `ls -la` of the real `imports/` and the tick store before and after M1 to M11 | identical; no new `.lock` |
| M14 | `store_not_local` on a real network volume | an SMB mount as the store root, `cli gdfl-import indices zip <day> <day> NIFTY` | `store_not_local{root, fs: "smbfs"}` before any line (L-IA3B-04) |

## Honesty notes

- **Root.** The container runs as root, so the permission halves of `source_unreadable` and
  `store_read_only` are NOT PROVED end to end here (M8, L-IA3B-06).
- **The forbidden interpreter.** It was invoked twice in this session by accident, each time
  only its version command, with the output discarded. No file was read or written by it, and
  no tracked file names it (gate 15 and `no_py_language.rs` pass).
- **Gates 1d and 23.** I did not widen `.github/gates_tree.rs`, `.github/gates_runtime.rs` or
  `ci.yml`: no part's spec gives them to this part, and the base is red on both without it.
- **No tokio bump** (above).
- **The local checkout.** Its last local commit is 2c3a93b9; origin holds 08a44755 and later,
  pushed from `/dev/shm` clones. A worker restart wiped the first clone after its push, which
  briefly left the local branch pointing at an object it no longer had. The ref was set back to
  2c3a93b9 and the index rebuilt from it; nothing was lost, because origin has every commit.
