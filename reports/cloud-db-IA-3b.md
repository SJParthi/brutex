STATUS: IN PROGRESS

# IA-3b (cloud): the import preflight refusals IA-3 left

Branch `cloud/db-IA-3b-20261024`, cut from `origin/feat/db-fix` e851da1c (Merge DATA-2). The
newest `origin/feat/db-fix`, 1c094c90 (Merge IA-4, with Merge R4-27 before it), was merged in
before the final gates (merge commit fa6aeef7). The spec is
`reports/db-specs-20261024/IA-3b.md` on `origin/cloud/db-base-20261024`.

HOLD appeared at 5dbfdf3f and the part was held (commit c1849952). It resumed under its own
`GO-IA-3b` (f20c2d35). The per-part test in the README passed at every later save point.

## Why the status is not DONE

PENDING: filled when the final runs finish.

## What is done

Everything the spec's items (1) to (10) ask is built and tested.
`pull::gdfl_run::plan` calls `preflight::run` once, after the scope is resolved and before any
lock, claim or journal line, so `cli gdfl-import` and the api's child plan the same way.

| Item | Refusal or behaviour | Where |
|---|---|---|
| 1 | `year_zip_missing{year, near_miss}`: at most 4 near misses, in name order. `source_unreadable{path, mode}`. `year_zip_empty{year}`: the end record's count, read from the tail alone. A root with no year zip at all is left to AF-01's `source_root_unknown`. | `gdfl_run_preflight.rs` `year_zips`, `year_zip`; `gdfl_archive::stated_entries` |
| 2 | An empty day zip nested in the CM archive or in an options year zip, and an empty tick-store day index, each refuse the day `day_zip_empty{name}` (`Cause::open`; the day stays open). `name` is the vendor day zip, e.g. `GFDLNFO_TICK_01042024.zip`. | `gdfl_archive.rs`, `gdfl_nfo.rs`, `gdfl_tickstore.rs` |
| 3 | `member_too_large{name, size}` (R4-08's code and fields; the one cap, `MEMBER_CAP`) is refused before any byte is read: in `member_bytes` on `len.max(packed)`, and in `rebuild` on `size.max(len)` | `gdfl_archive.rs`, `gdfl_tickstore.rs` |
| 4 | `source_changing{path, before, after}` is checked after every listing and member read: the identity is len, inode, mtime and ctime (never st_dev), from the held handle and from the path. `source_busy{root, holder}` is the tick-store writer's `.lock`, probed with a shared non-blocking flock and never created. | `Watch`, `SourceIdentity`, `writer_lock` |
| 5 | `census_room_short{layout, free_now, free_after, need_first_day}`: two header slot reads, with the v4 compaction modelled by `epoch_capacity`. `census_compaction_no_disk{need_bytes, free_bytes, floor}` goes through R4-07's `floor_check` at the run's own floor. `census.projected_stop` is labelled an extrapolation. | `room`, `census_header`, `projected_stop` |
| 6 | `legacy_run_may_be_live{key}`: on a store with no claim lock file, checked per kind on v4 and across kinds on v3. "Has claimed" is the lock's existence, one `metadata` per name (D-4908). `--legacy-run-not-live` gives the acknowledgement; it is recorded `legacy_ack=1` and written on the request line only when given. | `legacy`; cli `LEGACY_RUN_NOT_LIVE` |
| 7 | `store_read_only{root}` (the sentence ends in IA-16's fix), `store_not_local{root, fs}` and `open_files_short{have, need}`. The volume is read with one `statfs`/`statvfs` and the limit with `getrlimit`, through rustix. `open_files_exhausted` comes from the store error's own EMFILE number. | `Os`, `writable`, `ingest_failkind.rs` |
| 8 | Each source's span is stated on the plan line, and the asked days outside it are counted `outside_source`, never as holes | `Archive::span`, `TickStore::span`, `NfoZips::span`, `NfoTickStore::span` |
| 9 | A `FORMAT.md` digest outside the charter's two is warned on stderr and marked `format_digest_unpinned=1` on the run line, never refused: the magic decides (D-4819) | `Format`, `gdfl_import_journal.rs` |
| 10 | The module doc names R4-11 for loose day zips (D-7005, amending D-4815) | `gdfl_run.rs` |

The plan says one `preflight <kind>: ...` line per kind on the cli's declared stderr writer.

Tests: 35 in `gdfl_run_preflight_tests.rs`, of which 31 run and 4 are ignored. The 4 ignored
are the read-only-volume test (M8), the descriptor measurement and the two real-data tests (M1,
M4). All 21 failing-first names from the spec's section 3.1 exist. One cli test
(`the_legacy_flag_reaches_the_request_and_lets_the_run_past_the_cutover`) and the 10 new codes
in IA-1's code-list test were added.

## Gates

PENDING: filled when the final runs finish.

## Measurements (p50, p99, max)

C-GR-05 (`cargo bench -p pull --locked --bench ratio`, release, 24 Oct, in this container: 4
shared cores, Linux, root). The inputs are invented: day files written by the tick-store writer
itself, and the census as a header over a sparse file. The bench passed: "all ratios within the
ceiling", EXIT 0.

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

Peak open descriptors (ignored test `the_descriptors_a_run_holds_are_measured_at_one_and_four_workers`,
400 invented files, three runs): 32 at W=1, 32 at W=4 and 36 to 37 at W=12, beyond the 3 held
before. Every peak is at or under `files_needed(W)` = 32 + 5W (92 at W=12).

All of it is in `docs/06-limits.md`, in the new section "The import preflight IA-3 left": C-GR-05,
L-IA-35 and L-IA3B-01 to L-IA3B-07.

## Extreme cases (17; 3 combinations, 3 out of the box)

1. A misspelt year zip next to a `.DS_Store` and other years' zips: only names that look like
   that year are near misses (`a_missing_or_misspelt_year_zip_is_refused_with_its_near_misses`).
2. Many look-alikes of one year: capped at 4, in name order
   (`at_most_four_near_misses_are_named_in_name_order`).
3. A range across New Year needs both years' zips (`a_range_across_new_year_needs_both_year_zips`).
4. A year zip of zero bytes, or one whose end record states zero entries
   (`an_empty_year_zip_is_refused_before_any_line`). Its count is read from the tail alone: 1 and
   2,000 members cost the same reads (`the_stated_entry_count_reads_the_tail_alone`).
5. An empty day zip nested in both vendor containers, and an empty tick-store index: the day is
   refused and stays open, and the next day closes
   (`an_empty_nested_day_zip_refuses_its_day_and_stays_open`,
   `an_empty_tick_store_day_refuses_its_day_and_stays_open`).
6. Members stating `MEMBER_CAP + 1`, `u64::MAX` packed and unpacked, and exactly the cap: zero
   bytes are read before the refusal, and the cap itself passes
   (`a_member_stating_more_than_the_cap_is_refused_before_any_read`,
   `a_tick_store_entry_rebuilding_past_the_cap_is_refused`).
7. **Combination**: a zip still being copied while two runs read it. No day read after it grew is
   closed, and once the copy is whole the next run closes both days
   (`a_zip_that_is_still_being_copied_is_never_closed`).
8. A year zip rewritten in place between its listing and a read
   (`a_year_zip_rewritten_between_listing_and_read_is_source_changing`).
9. **Out of the box**: a tick-store day file replaced by rename mid-read, by a file of the same
   length AND the same mtime. Only the inode and ctime tell it apart; the handle still reads the
   old file (`a_tick_store_day_file_replaced_by_rename_mid_read_is_source_changing`,
   `the_source_identity_is_length_inode_and_both_times_only`).
10. A held tick-store writer lock is `source_busy`, and the probe never creates the lock
    (`a_held_tick_store_lock_is_source_busy_and_never_created`).
11. **Out of the box**: a torn census header (the newer slot made durable before its entries) is
    read at its older slot; when neither slot decodes it is refused by name
    (`a_torn_census_header_is_read_at_its_older_slot_or_refused_by_name`).
12. **Combination**: census room and the disk at once. Only the compaction gives room and its
    epoch does not fit above the floor; one byte more free and it plans
    (`a_compaction_the_disk_cannot_hold_is_refused_before_any_line`).
13. **Combination**: read-only AND network AND short of descriptors at once, and a missing year
    zip on such a store. Each is refused in the stated order
    (`refusals_come_in_their_stated_order_when_several_hold_at_once`).
14. The open-file limit at its boundary for every worker count 1 to 12: `need - 1` is refused and
    `need` passes (`a_run_under_an_open_file_limit_of_256_is_refused_open_files_short`).
15. EMFILE from the store's own error number is `open_files_exhausted`, never a general I/O fault
    (`an_emfile_during_a_run_is_named_open_files_exhausted`).
16. The legacy cutover is checked per kind on v4: an indices run is not refused for an options
    key. On v3, where kinds share one claim, it is refused
    (`the_first_claim_asks_about_a_legacy_run`). The probe never creates a lock file, and a store
    that holds one is never asked again (`the_legacy_probe_never_creates_a_lock_file`). The
    acknowledgement crosses the request line only when given, so earlier lines are
    byte-identical (`the_acknowledgement_crosses_the_request_line_only_when_given`).
17. **Out of the box**: a tick store whose `FORMAT.md` digest is neither of the charter's two is
    warned and marked on the run line, never refused
    (`an_unpinned_tick_store_format_digest_warns_and_marks_the_run_line`).

## Failing first

- **Compile red.** Every new test calls this part's new surface: `preflight::*`, the 10
  `RunRefusal` variants, the three `CmRefusal` variants, `Request.legacy_ack`, `Plan.checks`,
  `FailKind::OpenFilesExhausted`, `stated_entries` and the spans. None of these exists at
  e851da1c, so the tests do not compile there.
- **Behaviour on the base.** These are the spec's own facts at e851da1c:
  - An empty archive "holds no day" (`day()` answers `Ok(None)`, `an_empty_archive_holds_no_day`).
  - An empty tick-store index fails as "no zstd frame", not by a code.
  - `member_bytes` and `rebuild` have no size check.
  - `ends_with` sees EMFILE only as an unnamed `Io`.
  - `plan` has no preflight at all.
- **Reds seen while building.** Each was fixed in the code, never in the test:
  - the rename test, red while the watch reported the handle's identity as "after";
  - the growing-zip test, red until the fixture held both indices;
  - the R2 and R3 attack models, red until both sources named an empty day by the same vendor
    file name.
- **Kept unchanged.** AF-01's two FM-060 tests,
  `a_wrong_source_root_is_refused_before_any_day_is_journalled` and
  `a_run_that_read_no_file_never_exits_ok`, are unchanged: the preflight yields to AF-01's door.

## Files outside ownership

| File | Edit | Why |
|---|---|---|
| `crates/pull/src/gdfl_run.rs` | the 10 variants and codes, the `#[path]` mod line, the one call in `plan`, the doc line, `Request.legacy_ack` and `Plan.checks` | the spec's sequenced edit, plus item (6)'s field the spec names |
| `crates/pull/src/gdfl_archive.rs` | `member_bytes` cap, empty nested day zip, `stated_entries`, `Watch`, `span` | sequenced edit (items 2, 3, 4, 8) |
| `crates/pull/src/gdfl_tickstore.rs` | `rebuild` cap, empty index, `open_watched_day` (with its `WatchedDay` alias), spans | sequenced edit (items 2, 3, 4, 8) |
| `crates/pull/src/gdfl_nfo.rs` | empty nested day zip, watches, spans; `read_index` import dropped after the merge | sequenced edit (items 2, 4, 8) |
| `crates/pull/src/gdfl_cm.rs` | 3 `CmRefusal` variants and Display. `SymbolNotInUse`'s arm moved into a free function, word for word, because the new arms pushed `fmt` past `clippy::too_many_lines`; the file's own comment gives this remedy | sequenced edit |
| `crates/pull/src/gdfl_import_causes.rs` | 3 `cm_cause` arms (exhaustive) and `door_said`'s `OpenFilesExhausted` arm | sequenced edit |
| `crates/pull/src/ingest_failkind.rs` | `FailKind::OpenFilesExhausted` from `StoreError::Io { code: Some(EMFILE) }` | item (7); the spec's section 9 says the number must be carried |
| `crates/pull/src/gdfl_import_filing.rs` | the fix sentence shared through `preflight::NOT_WRITABLE_FIX` | so `store_read_only` and `store_not_writable` say one sentence |
| `crates/pull/src/gdfl_import_journal.rs` | `format_digest_unpinned=1` on the run line; `Journal::first_open` | item (9) and the legacy key; not in the spec's list |
| `crates/cli/src/gdfl_import_command.rs` | `--legacy-run-not-live` and `ClaimAsk.legacy_ack` from it, as the spec says. Also `said`, which writes the plan line and the unpinned warning, and an `expect(struct_excessive_bools)` with its reason, as `Request` has | sequenced edit, plus clippy |
| `crates/pull/benches/ratio.rs` | one `#[path]` mod pair and its `ok &=` call printing C-GR-05 | sequenced edit; gate 14's cover line unchanged |
| root `Cargo.toml` | rustix `process` feature, with its reason | sequenced edit (`getrlimit`) |
| `crates/pull/src/gdfl_run_tests.rs` | the 10 new variants in IA-1's code-list test | that test pins every code |
| `crates/pull/src/gdfl_nfo_tests.rs` | WP-16's "indexed once" test proves the one index by `Arc::ptr_eq`. It used to move the zip mid-run, which item (4) now refuses by design | adapted, not weakened |
| `crates/pull/src/gdfl_r2_attack_tests.rs`, `gdfl_r3_attack_tests.rs` | the reference models count an empty day as refused (RA-322) | marked ADAPTED (IA-3b, RA-322) in the files |
| `crates/cli/src/gdfl_import_command_tests.rs` | one new test | the cli half of item (6) |

## D-numbers used

D-4802, D-4819, D-4850, D-4864, D-4908 (the design's), and D-7000 to D-7007 from the assigned
D-7000..D-7009. D-7008 and D-7009 are unused.

## NEEDS THE MAC

These are the spec's M1 to M13. `$P` is this branch's worktree on the Mac. The commands and the
expected answers are the spec's (section "NEEDS THE MAC"); the test names are this branch's.

| # | What | Command | Expected |
|---|---|---|---|
| M1 | Every check's answer and time on the real roots | `BRUTEX_GDFL_TICKSTORE=... BRUTEX_GDFL_CM_ARCHIVE=... BRUTEX_GDFL_NFO_ARCHIVES=... BRUTEX_IA3B_REAL_STORE=<store, read only> cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 ia3b_real_preflight_prints_every_check_and_its_time` | every kind and source plans over 2018-09-03..2026-09-24; one `IA3B-REAL` line per check with its time. The test sets the legacy acknowledgement, because the real journal's `options 2020-09-16 *` ends in `begin` (M5 proves that refusal on its own) |
| M2 | A range wider than the source | M1's lines, or `cli gdfl-import indices zip 2018-01-01 2018-09-07 NIFTY` on a scratch store | `source_span=2018-09-03..2026-09-24` and `outside_source=N` (calendar-open days), never holes; journal unchanged |
| M3 | A year with no zip | `cli gdfl-import options zip 2017-12-01 2018-09-10` on a scratch store | `GDFL IMPORT REFUSED: year_zip_missing ... 2017`, no near miss (`.DS_Store` is not one), before any line, lock or progress record |
| M4 | The largest member of each container against the cap | `cargo test --release -p pull --lib -- --ignored --nocapture --test-threads=1 ia3b_real_largest_member_of_each_container` (the three source variables) | CM about 15,279,399 B, options about 2,359,609 B, the tick store measured, all under 2 GiB; refused days counted and the first one named. Write the numbers into `docs/06-limits.md` L-IA3B-05 |
| M5 | The legacy cutover on the real journal | the spec's M5 command (copy only the journal into a scratch root, then `cli gdfl-import options zip 2020-09-16 2020-09-16`) | `legacy_run_may_be_live` naming `options 2020-09-16 *`, no lock file created; with `--legacy-run-not-live` the claim records `legacy_ack=1`; the time goes into L-IA3B-02 |
| M6 | Census room on the real v4 census | M1's `census=v4 free_now= free_after= need_first_day=` | no kind refused; `census.projected_stop` only when the range may need more, labelled an extrapolation |
| M7 | The open-file limit | `launchctl limit maxfiles`; then, in a fresh Terminal (soft 256), `cli gdfl-import options zip 2024-04-04 2024-04-04` on a scratch store; then measure the cli's own descriptors | refused `open_files_short{have, need}` only when need > 256 (the model needs 92 at 12 workers, so none is refused at 256); the cli's own descriptors measured into L-IA-35 |
| M8 | A read-only store | `hdiutil create -size 64m -fs APFS -volname IA3BRO $TMPDIR/ia3b-ro.dmg && hdiutil attach -readonly $TMPDIR/ia3b-ro.dmg`, then `BRUTEX_IA3B_RO_ROOT=/Volumes/IA3BRO cargo test --release -p pull --lib -- --ignored --nocapture a_read_only_store_is_refused_before_any_line_on_a_real_read_only_volume`, then `hdiutil detach /Volumes/IA3BRO` | `store_read_only{root}` before any line, its sentence ending in IA-16's fix. Also run the two permission tests as the operator (not root): `an_unreadable_year_zip_is_refused_before_the_run`, `a_read_only_store_is_refused_before_any_line` |
| M9 | The import is unchanged when every check passes | IA-3's `real_days_close_with_the_same_listing_from_two_sources`, on feat/db-fix's build and on this branch's | identical close lines (IA-3's measured files=973, seconds=1,869,196, entries=11065 for NIFTY options 2024-04-04) |
| M10 | Bench C-GR-05 on the Mac | `cargo bench -p pull --bench ratio 2>&1 \| grep -A16 C-GR-05` | ratios within 3x; p50, p99, max next to this container's numbers in docs/06 |
| M11 | Shadow-server regression (no route change) | the spec's M11 commands (`stop.sh`, `shadow.sh`, `serve.sh IA-3b 8506 ...`, 200 timed requests to `/imports.json?feed=gdfl` and `/logs`) | as the spec's M11 |
| M12 | Coverage and mutants | `cargo llvm-cov --branch -p pull -p cli`; `cargo mutants --in-diff <diff of this branch against feat/db-fix>` on the touched modules | 100% line and branch on the touched modules; no surviving mutant. **Not run here**: neither tool is installed, and the instrumented build and the mutant builds do not fit the container's ~3 GB of free disk |
| M13 | Nothing touched the real store | `ls -la` of the real store's `imports/` and the tick store, before and after M1 to M11 | identical listings; no new `.lock` anywhere |
| M14 | `store_not_local` on a real network volume | an SMB mount as the store root, then `cli gdfl-import indices zip <day> <day> NIFTY` | `store_not_local{root, fs: "smbfs"}` before any line (L-IA3B-04) |

## Honesty notes

- **Root.** This container runs as root, so the two permission refusals' end-to-end halves
  (`source_unreadable` on a mode-0000 zip, `store_read_only` on a mode-0444 lock) are NOT PROVED
  here. Each test proves the refusal on a name that cannot be opened and is quiet about the root
  case; it is listed at M8 and in L-IA3B-06.
- **12 environment failures.** Proven earlier on the base: on e851da1c, with `crates/pull` of
  the base checked out, the same 12 tests fail identically (0 passed, 12 failed). They are
  root/permission tests and runs refused by the 10 GiB disk floor, which this container (about
  3-5 GB free) is below. The list: `alerts` create_fresh; `gdfl_cm`
  `fixtures_are_built_not_pasted`; `gdfl_dropbox` r1 x2; IA-16's immutable-month test;
  `gdfl_run` a_clock_step_back, progress_file and kinds_in_order (`DiskFull`); `census_store`
  link_tests x2; `zerodha_verify` x2.
- **The forbidden interpreter.** It was invoked twice in this session by accident, each time
  only its version command with the output discarded. No file was read or written by it, and
  nothing of it is in any tracked file (`no_py_language.rs` passes).
- **Gate 1d and gate 23.** This part adds literals and prints in the same classes the base
  already fails on (68 words, all invented, listed above; one test file with 6 `println!` in
  ignored measurement tests). I did not widen `.github/gates_tree.rs`, `.github/gates_runtime.rs`
  or `ci.yml`: those belong to no part's spec here, and the base is red on them without this
  part.
- **No tokio bump.** `cargo deny` is red on RUSTSEC-2026-0386 with a `Cargo.lock` identical to
  `origin/feat/db-fix`. The fix (`cargo update -p tokio --precise 1.53.3`, plus its decision and
  the offline floor in `crates/vocab/tests/workspace_is_rust.rs`, as BASE-REDS-1021 did) is a
  base package, not IA-3b.
