STATUS: IN PROGRESS

# Cloud mutation and coverage pass — `crates/pull`, 2026-10-17

Branch `cloud/mut-pull-20261017`, cut from `origin/feat/db-fix` at `f9cdb584`.
Continues `cloud/mut-pull-20261016` (report
`reports/cloud-mut-pull-20261016.md` on that branch), whose test work was
re-applied with `git diff c9e3472e5 origin/cloud/mut-pull-20261016 -- .
':(exclude)reports' | git apply -3`: every hunk applied cleanly, no conflict,
no `feat/db-fix` line touched.

**Scope:** the same ten files — `cash_session_cache.rs`, `fold.rs`,
`gdfl_archive.rs`, `gdfl_cm.rs`, `gdfl_cm_auction.rs`, `gdfl_import_scope.rs`,
`gdfl_nfo.rs`, `gdfl_tickstore.rs`, `request_minutes.rs`, `scrub.rs`. Of
these, `feat/db-fix` changed the production code of two since `c9e3472`:
`request_minutes.rs` (R3-18: `tally`, `stored_line`, `ledger_key`,
`append_named_picking`, the `stored_short` reason) and `gdfl_archive.rs`
(`dos_time`, `raw_name`). The other eight are byte-identical in production,
and no test was removed between the two bases (every `#[test]` name of
`c9e3472` is still in `f9cdb584`; 52 were added), so the old pass's kills in
those eight files still stand.

**No production line changed on this branch.** `git diff origin/feat/db-fix`
touches only test code under `crates/pull`, one test-order override in
`.config/nextest.toml`, one gate-1d declaration in `.github/gates_tree.rs`,
and this report.

## Result

On the branch head, under Gate 18's own flags and as an unprivileged user,
**every mutant that can be killed in the ten files is killed**, and **no
mutant times out**. What survives is 19 mutants proved equivalent (the old
18, re-confirmed, and one on a line `feat/db-fix` added). **No real bug was
found**; one missing test was (`stored_line`'s whole-day answer), and is
written. Lines and branches: five files at 100% of both; every line and
branch left in the other five is listed with why no test can reach it.

| Run (Gate 18 flags, user `mut`) | Mutants | Caught | Missed | Timeout | Unviable |
|---|---:|---:|---:|---:|---:|
| `survivors1` — the old pass's 23 survivors, before this pass's tests | 23 | 1 | 18 | 4 | 0 |
| `timeouts2` — the 4 timeouts and the 1 old timeout caught above, after | 5 | **5** | 0 | **0** | 0 |
| `newcode` — every mutant on a line `feat/db-fix` changed since `c9e3472` | 58 | 53 | 2 | 0 | 3 |
| `rmrest` — the rest of `request_minutes.rs` | 115 | 104 | 2 | 0 | 9 |
| `fixed544` — `stored_line`'s mutant after its new test | 1 | PENDING | | | |

The eight files whose production code is unchanged since `c9e3472` were not
re-mutated whole: the old pass killed every killable mutant in them against
a suite that is a subset of today's (no test removed, 52 added), and their
survivors and timeouts were re-tested above.

## Done (the old report's NOT DONE list, item by item)

| Old NOT DONE item | Now |
|---|---|
| Branch coverage not measured | **Measured** (nightly `--branch`, opt-level 0, unprivileged): 818/842 branches; five files 100%; each remaining branch explained. |
| Five timeouts remain timeouts | **All five caught** under Gate 18's flags: one already was under nextest, three by the old pass's fold watchdog run first, one by a new options-expiry watchdog run first. |
| 18 equivalent mutants remain missed | **Re-confirmed missed under Gate 18's flags**, each still equivalent for the stated reason; one more found on new code (`gdfl_archive.rs:280:58`). 16 of the 19 sit on lines Gate 18 will plan for `feat/db-fix` → `main` (NEEDS THE MAC 1). |
| Uncovered production lines remain | Reduced: `cash_session_cache.rs:184–188` now counted (the uid-65534 child's own run); twelve provokable refusal and host-error paths given tests (Tests added); every line still unreached listed with why. |
| The container runs as root; three tests fail as root | `gdfl_tickstore`'s test now re-runs itself where permission binds and passes as root. The two in `gdfl_import` (not this part's files) still fail as root and pass as any ordinary user. Every run here was also made as an ordinary user. |
| Not run: `cargo deny`, workspace clippy and tests | **Run** (Gates). |
| CI red on the old draft PR for base-snapshot reasons | No PR is opened from this branch. Gate 1d checked locally: red on the base, this branch adds no undeclared word. |

Also found and fixed in the re-applied tests: one test failed as any
ordinary user whose temporary directory is longer than `/tmp` (the socket
name overflowed `sun_path`; on a Mac it would always fail), and three
undeclared gate-1d words.

## Left

- `fixed544`'s result (running at the time of this line).
- Nothing else in scope. Everything that needs a decision is under NEEDS
  THE MAC.

## Method

| | |
|---|---|
| Tools | cargo-mutants **26.2.0** and cargo-nextest **0.9.145** (both CI's pins), cargo-deny **0.19.0** (CI's pin), cargo-llvm-cov 0.9.1, rustc 1.97.1 (pinned) and nightly 1.101.0 (2026-10-16) for branch coverage only |
| Mutation flags | exactly Gate 18's: `--test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle --cap-lints true --cargo-arg=--locked --minimum-test-timeout 900 --timeout-multiplier 2`, `--jobs 2`; the dev profile overridden to opt-level 0, incremental, no debuginfo, for rebuild time (outcomes do not depend on it; timings do). The old pass ran `cargo test`; under nextest each test is its own process and the run stops at the first failure, which is what decides a hang (see Timeouts) |
| User | every mutation, coverage and workspace-test run is as an **unprivileged user** (`mut`, uid 30033) in a mirror of the tree, so permission-bit tests run their own path as on CI and the operator's machine; the container itself runs as root |
| Coverage | `cargo +nightly llvm-cov -p pull --branch`, production lines and branches only (each file cut at its first `#[cfg(test)]` module) |

## Mutants

### The old pass's 23 survivors, re-tested under Gate 18's exact flags

Run `survivors1`, on the re-applied tree before this pass's new tests: the
whole `pull` suite (1,739 tests, lib and every integration binary),
sequential, stopping at the first failure. Baseline: 65 s build, 500 s test.

| Outcome | Count | Mutants |
|---|---:|---|
| Caught | 1 | `cash_session_cache.rs:546:19` (`>=` → `<` in `hold`): filed a timeout by the old `cargo test` run, caught here by `an_installer_waits_for_readers_who_let_go_inside_the_bound`, which runs before the test that hangs |
| Missed | 18 | exactly the 18 the old report proved equivalent (table below) |
| Timeout | 4 | `fold.rs:1083:42`, `fold.rs:1191:20`, `fold.rs:1245:42`, `gdfl_nfo.rs:569:22`, each at 1,002 s |

### Timeouts made catches

Gate 18 never credits a timeout. Under its sequential order an unguarded
test that loops forever on the mutant ran before any test with a watchdog,
so the run hung. Two changes, no existing test edited:

- `gdfl_nfo::cloud_mutant_tests::the_monthly_walk_back_over_a_holiday_answers_within_a_bound`
  (new): the holiday walk-back under a ten-second watchdog. The fold's three
  already had one: `an_index_days_absent_head_and_tail_are_named_and_a_shares_are_not`
  (old pass) runs the fold on its own thread and fails after ten seconds.
- `.config/nextest.toml`: those two tests at `priority = 100`, the repo's
  own pattern for running a fast regression first (D-0693, D-0694). Order
  only; gate 1's nextest scan allows exactly the keys used.

Run `timeouts2`, the same flags on the branch with both changes (baseline
60 s build, 414 s test): **5 mutants, 5 caught, 0 timeouts.**

| Mutant | Caught by | At |
|---|---|---|
| `fold.rs:1083:42` `*` → `/` in `fold_month` | `cloud_mutants_import fold_paths::an_index_days_absent_head_and_tail_are_named_and_a_shares_are_not` (watchdog) | test 3 of 1,750, 10.0 s |
| `fold.rs:1191:20` `+=` → `*=` in `fold_month` | the same | test 3, 10.0 s |
| `fold.rs:1245:42` `*` → `/` in `flush_tail` | the same | test 3, 10.0 s |
| `gdfl_nfo.rs:569:22` `-=` → `/=` in `monthly_expiry` | `gdfl_nfo::cloud_mutant_tests::the_monthly_walk_back_over_a_holiday_answers_within_a_bound` | test 1, 10.0 s |
| `cash_session_cache.rs:546:19` `>=` → `<` in `hold` | `cash_session_cache::cloud_mutant_tests::an_installer_waits_for_readers_who_let_go_inside_the_bound` | test 129, 0.03 s |

### The code `feat/db-fix` changed since the old base

Run `newcode`: every mutant on a line `git diff c9e3472e5 origin/feat/db-fix
-- crates/pull/src/request_minutes.rs crates/pull/src/gdfl_archive.rs`
touches (`--in-diff`), the same flags with two test threads (a thread count
changes only how soon a catch is seen; no mutant here hangs). Baseline 62 s
build, 203 s test.

**58 mutants: 53 caught, 3 unviable, 2 missed.** No catch rests on a timing
test alone: the first failing test of every caught mutant is a behaviour
test (`emit_sites::every_emit_site_in_this_crate_reaches_a_file` 25,
`a_gap_ledger_is_written_for_stored_months_without_a_vendor_call` 13, the
rest request-audit, tick-store and import tests).

| Missed | Outcome |
|---|---|
| `request_minutes.rs:544:19` `>` → `>=` in `stored_line` | **A missing test, now written.** `stored_line` answers `None` for a day holding every owed minute, and nothing pinned it: `gap_backfill` hands it only days already short. `a_stored_day_holding_every_owed_minute_has_no_line` asks it directly; applied by hand, the mutant fails that test (it writes a `missing=0` line for the whole day), and the real code passes. |
| `gdfl_archive.rs:280:58` `\|` → `^` in `central_directory` | **Equivalent.** `dos_time = field(14, 2) << 16 \| field(12, 2)`: each field is two bytes, so the shifted date and the time share no bit and OR, XOR and `+` agree on every input. |

The three unviable mutants ask `Default` of `Entry`, `ZipEntry` and `Group`,
which do not implement it.

### The rest of `request_minutes.rs`

Run `rmrest`: every other mutant of the file (`-f request_minutes.rs`, the
58 above excluded by name), the same flags with two test threads. Baseline
62 s build, 255 s test. **115 mutants: 104 caught, 9 unviable, 2 missed** —
the two old equivalents `333:35` and `347:22`, nothing else. Every catch is
a behaviour test (`emit_sites::every_emit_site_in_this_crate_reaches_a_file`
32, the request-audit tests the rest); none is a timing test.

### Equivalent mutants (18, missed by construction)

Re-confirmed missed under Gate 18's flags (run `survivors1`). Line numbers
are the current file's; `request_minutes.rs` moved by 8 lines since the old
report (`325` → `333`, `339` → `347`).

| File:line:col | Mutant | Why it is equivalent |
|---|---|---|
| `fold.rs:272:38` | guard `width > 0` → `true` in `fold` | `width = secs × 10⁶` and `Bucket::of_secs` refuses 0, so `width ≥ 10⁶`. |
| `fold.rs:272:44` | `>` → `>=` in `fold` | Same: `width` is never 0. |
| `fold.rs:1126:32` | `>` → `>=` in `fold_month` | `day_note_at` is read only for a withheld day, and every withheld day either had `day_session` push its note (both forms set the same index) or has `day_note` push one, which overwrites it right after. |
| `fold.rs:1149:68` | `<` → `<=` in `fold_month` | At `missing == start`, `scheduled_minutes(.., missing, start)` is 0, so nothing is pushed either way. |
| `fold.rs:1206:42` | `>` → `>=` (complete's `expected > 0`) | Every candidate bucket holds a bar, so `count ≥ 1`; `count == expected` cannot hold at 0. |
| `fold.rs:1209:22` | `>` → `>=` (partial's `count > 0`) | `count ≥ 1` always. |
| `fold.rs:1210:25` | `>` → `>=` (partial's `expected > 0`) | `count < expected` cannot hold at `expected == 0` with `count ≥ 1`. |
| `fold.rs:1527:9` | `\|\|` → `&&` in `minute_session` | Every shipped venue row is continuous and opens at 09:15: both operands are false on every reachable input. |
| `request_minutes.rs:333:35` | `>` → `>=` in `day_plan` | The arm is reached only with `Unmeasured`; `sourced_kind_of` never answers that inside `SOURCED_FIRST_DAY..=SOURCED_LAST_DAY`, so at `SOURCED_LAST_DAY` both forms take the same arm. |
| `request_minutes.rs:347:22` | guard `hours.kind() == Continuous` → `true` in `regular_plan` | Every venue row is `Continuous` and verified. |
| `cash_session_cache.rs:508:5` | `still_regular` → `Ok(())` | Only a swap of the lock path between two system calls (a race) reaches the refusal. |
| `gdfl_cm.rs:854:13` | delete field `indices` in `CmScope::stored` | `..Self::swept()` supplies the same empty `Vec`. |
| `gdfl_cm.rs:965:31` | guard `name.len() > SYMBOL_CAPACITY` → `false` | `share_at` holds only legal symbols of at most 24 bytes; a longer name misses the probe either way. |
| `gdfl_cm.rs:1010:41` | guard `self.series` → `true` in `CmScope::filing` | Reached only when `stored`, and every constructor sets `stored` and `series` together. |
| `gdfl_cm.rs:1097:22` | guard `self.stored` → `true` in `CmScope::member_of` | An unstored scope is `swept`, whose `indices` is empty: `index_row` is `None`, the arm's own answer. |
| `gdfl_cm.rs:2306:70` | `>` → `>=` in `settle_auction` | An unknown-eligibility zero-book candidate refuses the file at its row, so a one-price band always has `booked ≥ 1` here. |
| `gdfl_cm.rs:2313:34` | guard `self.booked > 0` → `true` | Differs only for a one-price band of zero-book prints alone: no row is in the band, `retain` takes nothing, the recomputed maximum equals the running one. |
| `gdfl_cm.rs:2313:46` | `>` → `>=` | Same as `2313:34`. |

## Coverage — lines AND branches, production only

The old pass could not measure branches (the pinned 1.97.1 has no
`--branch`). This pass does, with the nightly toolchain, as the unprivileged
user, at **opt-level 0**. That last setting matters: at the workspace's dev
opt-level 3 a small `pub const fn` (`scrub::contract_of`, the `fold::Ladder`
methods) is inlined across crates into the integration-test binary and the
nightly report loses its counters, so it reads 0 hits though tests call it.
At opt-level 0 nothing is inlined and the count is honest. Every test passes
in this run.

| File | Lines | Branches |
|---|---:|---:|
| `cash_session_cache.rs` | 435/454 | 47/48 |
| `fold.rs` | 618/629 | 117/126 |
| `gdfl_archive.rs` | **374/374** | **44/44** |
| `gdfl_cm.rs` | 1,143/1,151 | 263/270 |
| `gdfl_cm_auction.rs` | 188/201 | 33/38 |
| `gdfl_import_scope.rs` | **104/104** | **10/10** |
| `gdfl_nfo.rs` | **611/611** | **122/122** |
| `gdfl_tickstore.rs` | **343/343** | **82/82** |
| `request_minutes.rs` | 589/599 | 88/90 |
| `scrub.rs` | 122/126 | **12/12** |
| **Ten files** | **4,527/4,592 (98.58%)** | **818/842 (97.15%)** |

Line counts differ from the old report's (e.g. `cash_session_cache` 453 →
454): a different compiler's coverage mapping, and the nightly counts a
`map_err` closure's line as its own. **Five files are at 100% of both.**

### Every line and branch still unreached, and why no test can reach it

None is reachable by an input a test can build; each is a host failure, a
race, a defensive arm the types or the shipped tables close, or a guard
another check already decided.

| File:line | What it is | Why no test reaches it |
|---|---|---|
| `cash_session_cache.rs:341`, `387`, `419` | `try_reserve(..).map_err` | An allocation of a few thousand map slots failing: out of memory. |
| `cash_session_cache.rs:508` (branch), `511–514` | `still_regular`'s refusal | The lock path stops being a regular file between the open and the re-check: a race between two system calls (also the equivalent mutant `508:5`). |
| `cash_session_cache.rs:539–542` | `TryLockError::Error` | `flock` refused by the host for a reason other than contention (`ENOLCK` on a filesystem without locks). |
| `cash_session_cache.rs:585`, `595` | `fstat` / `read` of an open regular file failing | An I/O error (`EIO`) on a file already opened. |
| `cash_session_cache.rs:699–704` | `write_all`/`sync_all` failure | Disk full or an I/O error. |
| `cash_session_cache.rs:720` | the HTTP client failing to build | The TLS backend failing to initialise. |
| `fold.rs:343` (branch), `345–349` | `OutOfOrder` with no bar open | Marked in the source: "Unreachable: `open_at` is only ever Some after a push." |
| `fold.rs:272` (branch) | `width > 0` false | `width = secs × 10⁶` and `Bucket::of_secs` refuses 0 (equivalent mutants `272:38`, `272:44`). |
| `fold.rs:1201` (branch), `1203` | a withheld bucket with no day note | Every withheld day has its note: an unmeasured calendar or an unsized/closed session gets one from `day_note`, an unattested one from `day_session` (equivalent mutant `1126:32`). |
| `fold.rs:1206`, `1209`, `1210` (branches) | `expected > 0` false, `count > 0` false | Every candidate bucket holds at least one bar, and a valid one is a scheduled minute, so `count ≥ 1` and `expected ≥ 1` (equivalent mutants `1206:42`, `1209:22`, `1210:25`). |
| `fold.rs:1242` (branch), `1243` | `flush_tail`'s `expected <= 0` | `missing < close`, both on the minute grid, so the window holds the session's last minute. |
| `fold.rs:1526`, `1527` (branches), `1529–1531` | `minute_session`'s refusal of non-continuous or non-09:15 hours | Every row of the three shipped venue tables is continuous from 09:15 (equivalent mutant `1527:9`). |
| `fold.rs:1550` | an exclusive close of minute 0 | `Schedule::close` answers only 915 or 930, the venue tables' closes. |
| `gdfl_cm.rs:860`, `865` (branches), `868` | an index row or a symbol change whose status does not apply | Every row of `core::alias::INDEX_ROWS` and `SYMBOL_CHANGES` is `Status::Sourced`. |
| `gdfl_cm.rs:1010` (branch) | `self.series` false in a stored scope | Both constructors set `stored` and `series` together (equivalent mutant `1010:41`). |
| `gdfl_cm.rs:1352` (branch), `1353–1354` | a slot with no cell | `slot_of` returns only slots inside `cells`/`extra`. |
| `gdfl_cm.rs:1522–1524` | `Ok(CasEligibility::Unknown)` from a master | `VerifiedLifecycleMaster::cas_eligibility` answers only `Eligible`, `NotEligible` or `Err`. |
| `gdfl_cm.rs:2306` (branch) | an unknown-eligibility one-price band with no booked row | A zero-book candidate of unknown eligibility refuses the file at its row, so such a band is only ever observed by booked rows (equivalent mutant `2306:70`). |
| `gdfl_cm.rs:2352` (branch), `2354` | the merge's `next` is `None` | The loop breaks when both runs are empty; any other case takes a row. |
| `gdfl_cm.rs:2836` (branch), `2837` | `Window::new(day, day)` refused | `Window::new` refuses only a window that runs backwards. |
| `gdfl_cm_auction.rs:173`, `201`, `204`, `207` (branches), `175`, `203`, `206`, `209` | `get_mut` of a fixed slice returning `None` | Every range is inside a fixed-size array whose length is a constant (28 of 32 bytes, 12 of 16). |
| `gdfl_cm_auction.rs:255` | `YearMonth::new` refusing a `Day`'s month | `Day` and `YearMonth` share the years 1970–9999. |
| `gdfl_cm_auction.rs:292` (branch), `295` | a sidecar path with no parent | A path built under the store root always has one. |
| `gdfl_cm_auction.rs:310–314` | the sidecar lock not released | A host error on `flock` release. |
| `gdfl_cm_auction.rs:329`, `373` | reading or writing the open sidecar failing | An I/O error or a full disk. |
| `request_minutes.rs:347` (branch), `349` | `regular_plan`'s refusal of venue hours | Every venue row is `verified` and continuous (equivalent mutant `347:22`). |
| `request_minutes.rs:425` (branch), `426–433` | the "invalid day" finding | The loop walks day numbers between two valid `Day`s. |
| `request_minutes.rs:1039` | the ledger append's host-error closure | Used only by `fstat`, `flock`, `pread`, `write` or `sync` failing on an open regular file. |
| `scrub.rs:220–223` | an entry naming no legal store path | Every segment comes from a type that already enforces the store's rules, so `StorePath::new` cannot refuse a manifest entry. |

## Tests added or repaired this pass

All synthetic: invented shares (`ZZAUCTION`, a 25-byte `ZZ…W`), invented
prices and stamps; `RELIANCE` and `HINDALCO` appear only as the name shapes
the old pass already used. No vendor byte is committed.

| Test | File | What it pins |
|---|---|---|
| `a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses` (repaired) | `gdfl_tickstore.rs` | Now runs where permission bits bind (D-0995's `where_permission_binds`): it failed as root on its own premise, a mode-0 day file opening. Body unchanged. |
| `tickstore_a_socket_at_a_day_name_is_refused_by_name` (repaired) | `tests/cloud_mutants_import.rs` | The old test bound its socket at the deep day path, which overflows `sun_path` (108 bytes Linux, 104 macOS) under any longer temporary directory: it failed as an ordinary user and would on a Mac. Bound at a short name and renamed to the day path; it now first asserts the inode is a socket. |
| `the_ledger_lies_beside_the_import_journal` (literal changed) | `gdfl_import_scope.rs` | Root now a temporary directory, not the undeclared gate-1d word `store-root`. |
| `a_cache_root_that_cannot_be_a_directory_refuses_by_name` | `cash_session_cache.rs` | A file at the cache root: the lock refuses by the root's name, the file untouched. |
| `a_lock_that_cannot_be_reopened_for_writing_refuses_by_name` | `cash_session_cache.rs` | A read-only lock file refuses the lock by its own name (where permission binds). |
| `a_cache_directory_that_cannot_be_synced_names_the_unsynced_publication` | `cash_session_cache.rs` | A writable, unreadable cache directory (0o300): payload and receipt are published and kept, then the missing directory sync is named, durability UNVERIFIED (where permission binds). |
| `a_hang_up_before_the_head_or_inside_the_body_is_refused_by_name` | `cash_session_cache.rs` | A server that closes before its head is refused naming the URL; one that closes inside its declared body is refused naming the body. |
| `a_pipe_at_the_ledger_name_is_refused_and_never_waited_on` | `request_minutes.rs` | A FIFO at `audit/<vendor>.gaps` opens at once (`O_NONBLOCK`) and is refused by its own `fstat`, by name, within a one-minute watchdog; `mkfifo` run from the test as `tickstore_write_tests` already does. |
| `a_buckets_ist_day_is_checked_at_both_ends_of_the_line` | `fold.rs` | `ist_day_of`: day 0 and day 1 exact, and `AnchorOverflow` at `i64::MAX` and `i64::MIN`, never a wrapped day. |
| `a_regular_session_on_a_day_no_day_can_name_is_refused` | `fold.rs` | `minute_session` asked for a regular session on day −1 or 3,000,000 refuses for the day, not the hours. |
| `a_header_longer_than_the_size_refuses_before_any_column_is_read` | `gdfl_tickstore.rs` | The size test's second half: a header past the stated size refuses though the rows fit; exactly at it, the columns are read. |
| `a_directory_at_the_sidecar_name_is_refused_by_name` | `gdfl_cm_auction.rs` | A directory where the month's auction sidecar belongs: refused by the sidecar's name, nothing written inside. |
| `a_zero_book_print_before_the_match_band_never_prices_the_band` | `tests/cloud_mutants_import.rs` | An eligible share's zero-book print at 15:20 is diverted as a print but never decides the band's price, so the booked 15:30 row at the band's one price is taken as the auction's and nothing is left unattributed. |
| `a_stored_share_of_the_full_symbol_width_is_filed` (extended) | `tests/cloud_mutants_import.rs` | One byte past the symbol capacity is no slot and no member. |
| `the_monthly_walk_back_over_a_holiday_answers_within_a_bound` | `gdfl_nfo.rs` | The monthly expiry over Thu 29 Mar 2018 (a holiday) answers 28 Mar inside a ten-second watchdog; a walk that stops stepping fails here instead of hanging the run. |
| `a_stored_day_holding_every_owed_minute_has_no_line` | `request_minutes.rs` | `stored_line`: a whole stored day has no `gap` line; a day one minute short has the line, field by field, with `fill=` and `source=store`, and a count of 1. |

## Gates

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | **pass** |
| `cargo clippy --workspace --all-targets --locked -- -D warnings` | **pass** (exit 0) |
| `cargo test --workspace --locked --no-fail-fast`, as the unprivileged user | **exit 101: 159 test runs (unit, integration and doc) ok, 2 tests fail, both outside this branch's diff** (below); 38 min with two build jobs |
| `cargo test -p pull --locked`, as the unprivileged user (inside the coverage run) | **pass**: every `pull` test, lib and integration |
| `cargo test -p pull --locked`, as root | the two `gdfl_import` permission tests still fail as root, as on the base (`a_day_ledger_that_will_not_take_a_line_leaves_the_day_open`, `a_journal_that_cannot_be_written_fails_the_day_by_name`); both pass as uid 65534. `gdfl_tickstore`'s third one passes as root now (repaired here). |
| `cargo deny check` (0.19.0) | **red, identical to the base**: advisories RUSTSEC-2026-0344 (ruzstd) and RUSTSEC-2026-0346, -0347, -0348, -0349 (rustls); bans, licenses and sources ok. Same five errors and two duplicate warnings on `origin/feat/db-fix`. `Cargo.toml`/`Cargo.lock` untouched (part DENY-1). |
| Gate 1d (`gates_tree gate-1d`, built from the tree) | **red on the base** (141 undeclared words in `crates/pull` on `f9cdb584`); **this branch adds none**. The re-applied tests brought three (`store-root`, `proc`, `self`): the first is gone from the test, the other two are declared in `CLOUD_MUT_PULL_LITERAL` with their reason (`/proc/self/status`, the procfs file `read_limited`'s test needs). `gates_tree`'s own 48 tests pass. |

The two workspace failures, both identical on `feat/db-fix` (this branch
changes no file of `cli` or `store`, and no non-test line any crate depends
on):

- `cli` `boolean_search_command::integration_tests::generated_search_recovers_same_ordinal_and_refuses_missing_ancestry`:
  fails as the unprivileged user (`published` 0, expected 1: "only a durably
  readable reservation publishes an identity",
  `crates/cli/src/boolean_search_integration_tests.rs:397`), deterministically
  and alone, with `TMPDIR` either `/tmp` or the user's own; the same binary
  passes as root.
- `store` `tests/cited_commits.rs::every_commit_the_store_records_cite_is_on_main_or_says_why_not`:
  `docs/06-limits.md` on `feat/db-fix` cites nine commits (`023d6ab8`,
  `079ad4af`, `4756cce0`, `521066e6`, `72c91a46`, `91ef0094`, `9cb9de51`,
  `cde73992`, `ff636f58`) that are not ancestors of `origin/main`
  (`4bbd37df`). Fails the same in the real checkout.

## NEEDS THE MAC

1. **Gate 18 will be red on 16 equivalent mutants when `feat/db-fix` is
   planned against `main`, whatever the tests.** Gate 18 plans every line of
   the PR's diff (`main...feat/db-fix` holds 6,241 `pull` mutants) and "never
   excludes a mutation or credits a timeout". 16 of this report's 19
   equivalents are on lines of that diff (all but `fold.rs:272:38`,
   `272:44` and `1527:9`). No test can kill them; only a production
   rewrite that removes the mutation site, or a gate policy, can. Each
   rewrite below keeps behaviour by the same argument that makes the mutant
   equivalent; none was made here (production code of other parts, and a
   tests-only pass):

   | Mutant(s) | Behaviour-preserving rewrite that removes the site |
   |---|---|
   | `gdfl_archive.rs:280:58` | `field(14, 2)? << 16 \| field(12, 2)?` → `… << 16) + field(12, 2)?`. `+`'s mutants (`-`, `*`) differ whenever the time is non-zero, and `a_bts_round_trips_every_entry_byte_identical`'s members carry times `0x7A2F` and `0x6B10`, so both are expected killed (expected, not measured). |
   | `gdfl_cm.rs:854:13` | delete the `indices: Vec::new(),` line; `..Self::swept()` supplies it. |
   | `gdfl_cm.rs:965:31` | delete the `CmKind::Stocks if name.len() > SYMBOL_CAPACITY => None` arm; the probe answers `None` for such a name. |
   | `gdfl_cm.rs:1010:41`, `1097:22` | drop the `if self.series` / `if self.stored` guards they are implied by (or make `series` a function of `stored`). |
   | `gdfl_cm.rs:2306:70`, `2313:34`, `2313:46` | drop `&& self.booked > 0` / `if self.booked > 0` where the band is `One` (a one-price band here always has a booked row, and with none the arm changes nothing). |
   | `fold.rs:1126:32` | have `day_session` return its `Option<Note>` and push it in `fold_month`, setting `day_note_at` from the push: no length comparison left. |
   | `fold.rs:1149:68` | drop `.filter(\|missing\| *missing < start)`: `expected > 0` already refuses `missing >= start`. |
   | `fold.rs:1206:42`, `1209:22`, `1210:25` | drop the `expected > 0` and `count > 0` conjuncts, implied by `count ≥ 1` (every candidate bucket holds a bar). |
   | `request_minutes.rs:333:35` | ask the calendar whether `at` is past the record (one function), instead of comparing with `SOURCED_LAST_DAY` at a boundary the arm never reaches. |
   | `request_minutes.rs:347:22` | none without a venue-table seam a test can fill; the guard is data-equivalent (every shipped row continuous). A gate policy is the only other way. |
   | `cash_session_cache.rs:508:5` | open the lock with `O_NOFOLLOW` and judge the open handle's own `fstat` (as `request_minutes::append_named_picking` now does) instead of re-reading the path: the race the re-check guards no longer exists, and a pipe or device at the name becomes a testable refusal. |

2. **`cli` `generated_search_recovers_same_ordinal_and_refuses_missing_ancestry`
   fails as an ordinary user here and passes as root** (Gates). Not this
   part's crate; sources identical to `feat/db-fix`. The Mac runs as an
   ordinary user, so it will show there if it is not this container.
3. **`store` `cited_commits` is red on `feat/db-fix` itself**: nine commits
   cited in `docs/06-limits.md` are not on `origin/main` and not in
   `NOT_ON_MAIN`.
4. **Two root-only test failures in `gdfl_import`** (outside this part's
   files) pass as uid 65534 and as any ordinary user; nothing to do on a Mac.
5. **A D-number, if wanted.** The nextest order override and the gate-1d
   declaration follow the repo's existing patterns (D-0693, D-0694; the
   per-package `*_LITERAL` groups) but cite no decision of their own; numbers
   are the Mac's to allocate, so none was claimed here.
