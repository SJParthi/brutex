STATUS: DONE

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

**`feat/db-fix` moved during the pass** (31 commits, to `297996d9`), three
of them inside this scope: `gdfl_cm_auction.rs` (the v2 auction sidecar,
+490/−80), `gdfl_tickstore.rs` (+118) and `gdfl_nfo.rs` (+23/−7). It was
merged in (`c511abfc`). The one conflict was both sides appending a test
module at the end of `gdfl_cm_auction.rs`; both are kept. `auction::parse`
now answers a `Sidecar`, so one old test reads its `.records`, assertions
unchanged. Every result below up to that merge was measured on the
`f9cdb584` base; what the merge brought into scope is measured separately
(Mutants: the code `feat/db-fix` moved during the pass). A second merge at
the end (`f4f4eabb`, `feat/db-fix` `f8c22b1b`, five commits) touches only
`store/tests/cited_commits.rs`, `docs/05-decisions.md` and
`docs/06-limits.md`, nothing in scope, with no conflict.

**No production line changed on this branch.** `git diff origin/feat/db-fix`
touches only test code under `crates/pull`, one test-order override in
`.config/nextest.toml`, one gate-1d declaration in `.github/gates_tree.rs`,
and this report.

## Result

On the branch head, under Gate 18's own flags and as an unprivileged user,
**every mutant that can be killed in the ten files is killed**, and **no
mutant times out**. What survives is 20 mutants proved equivalent (the old
18, re-confirmed, and two on lines `feat/db-fix` added: `gdfl_archive.rs:280:58`
and `gdfl_cm_auction.rs:535:73`). **No real bug was found**; five missing
tests were (a mutant of new `feat/db-fix` code that survived because nothing
pinned that behaviour: `stored_line`'s whole-day answer, and four of the v2
auction sidecar's), and each is written and its mutant now caught. Lines
and branches: five files at 100% of both; every line and branch left in the
other five is listed with why no test can reach it.

| Run (Gate 18 flags, user `mut`) | Mutants | Caught | Missed | Timeout | Unviable |
|---|---:|---:|---:|---:|---:|
| `survivors1` — the old pass's 23 survivors, before this pass's tests | 23 | 1 | 18 | 4 | 0 |
| `timeouts2` — the 4 timeouts and the 1 old timeout caught above, after | 5 | **5** | 0 | **0** | 0 |
| `newcode` — every mutant on a line `feat/db-fix` changed since `c9e3472` | 58 | 53 | 2 | 0 | 3 |
| `rmrest` — the rest of `request_minutes.rs` | 115 | 104 | 2 | 0 | 9 |
| `fixed544` — `stored_line`'s mutant after its new test | 1 | **1** | 0 | 0 | 0 |
| `moved` — every mutant on a line the mid-pass merge changed in `gdfl_cm_auction.rs`, `gdfl_tickstore.rs`, `gdfl_nfo.rs` (5 outcomes voided by a full disk, 1 never run) | 92 | 70 | 4 | 1 | 12 |
| `movedafter` — `moved`'s 5 survivors after this pass's tests, and the 1 never run | 6 | **5** | 1 | **0** | 0 |
| `moveddisk` — the 4 the full disk voided | 4 | **4** | 0 | 0 | 0 |

The eight files whose production code is unchanged since `c9e3472` were not
re-mutated whole: the old pass killed every killable mutant in them against
a suite that is a subset of today's (no test removed, 52 added), and their
survivors and timeouts were re-tested above.

## Done (the old report's NOT DONE list, item by item)

| Old NOT DONE item | Now |
|---|---|
| Branch coverage not measured | **Measured** (nightly `--branch`, opt-level 0, unprivileged): 854/876 branches (97.49%) and 4,721/4,786 lines (98.64%) on the final head; five files 100% of both; each remaining branch explained. |
| Five timeouts remain timeouts | **All five caught** under Gate 18's flags: one already was under nextest, three by the old pass's fold watchdog run first, one by a new options-expiry watchdog run first. |
| 18 equivalent mutants remain missed | **Re-confirmed missed under Gate 18's flags**, each still equivalent for the stated reason; two more found on new code (`gdfl_archive.rs:280:58`, `gdfl_cm_auction.rs:535:73`). 17 of the 20 sit on lines Gate 18 will plan for `feat/db-fix` → `main` (NEEDS THE MAC 1). |
| Uncovered production lines remain | Reduced: `cash_session_cache.rs:184–188` now counted (the uid-65534 child's own run); fourteen provokable refusal and host-error paths given tests (Tests added); every line still unreached listed with why. |
| The container runs as root; three tests fail as root | `gdfl_tickstore`'s test now re-runs itself where permission binds and passes as root. The two in `gdfl_import` (not this part's files) still fail as root and pass as any ordinary user. Every run here was also made as an ordinary user. |
| Not run: `cargo deny`, workspace clippy and tests | **Run** (Gates). |
| CI red on the old draft PR for base-snapshot reasons | No PR is opened from this branch. Gate 1d checked locally: red on the base, this branch adds no undeclared word. |

Also found and fixed in the re-applied tests: one test failed as any
ordinary user whose temporary directory is longer than `/tmp` (the socket
name overflowed `sun_path`; on a Mac it would always fail), and five
gate-1d words the branch would have been the only source of (Gates).

## Left

Nothing in scope. Stated limits, so nothing reads as more than it is:

- The eight files whose production code `feat/db-fix` did not change after
  `c9e3472` (before the mid-pass merge: all but `request_minutes.rs` and
  `gdfl_archive.rs`) were not re-mutated whole. Their kills are the old
  pass's, against a suite every one of whose tests still exists; their
  survivors and timeouts were all re-run here under Gate 18's flags.
  `request_minutes.rs` was re-mutated whole (58 + 115), `gdfl_archive.rs`
  on its changed lines, and the three files the merge changed on theirs.
- Measured against `feat/db-fix` `297996d9`; its later `f8c22b1b`, merged
  too, changes none of the ten files. Anything it gains in them after that
  is unmeasured.
- Branch coverage is the nightly toolchain's (the pinned 1.97.1 cannot
  measure it), at opt-level 0.

Everything that needs a decision is under NEEDS THE MAC.

## Method

| | |
|---|---|
| Tools | cargo-mutants **26.2.0** and cargo-nextest **0.9.145** (both CI's pins), cargo-deny **0.19.0** (CI's pin), cargo-llvm-cov 0.9.1, rustc 1.97.1 (pinned) and nightly 1.101.0 (2026-10-16) for branch coverage only |
| Mutation flags | exactly Gate 18's: `--test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle --cap-lints true --cargo-arg=--locked --minimum-test-timeout 900 --timeout-multiplier 2`. One test thread, as Gate 18, in every run that decides a hang (`survivors1`, `timeouts2`, `fixed544`, `movedafter`); two where nothing hangs (`newcode`, `rmrest`, `moved`, `moveddisk`), which changes only how soon a catch is seen. `--jobs 2` (1 for `movedafter` and `moveddisk`, after the disk filled). The dev profile overridden to opt-level 0, incremental, no debuginfo, for rebuild time (outcomes do not depend on it; timings do). The old pass ran `cargo test`; under nextest each test is its own process and the run stops at the first failure, which is what decides a hang (see Timeouts) |
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
  only; gate 1's nextest scan allows exactly the keys used. (A third, the
  auction sidecar's lock-wait watchdog, joined it after the merge; below.)

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
| `request_minutes.rs:544:19` `>` → `>=` in `stored_line` | **A missing test, now written.** `stored_line` answers `None` for a day holding every owed minute, and nothing pinned it: `gap_backfill` hands it only days already short. `a_stored_day_holding_every_owed_minute_has_no_line` asks it directly. Run `fixed544` (Gate 18's flags, one test thread): **caught** by that test, 910th of 1,751; its baseline is also the whole `pull` suite passing as the unprivileged user on the head before the merge below. |
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

### The code `feat/db-fix` moved during the pass

Run `moved`: every mutant on a line `git diff f9cdb584 HEAD` changes in
`gdfl_cm_auction.rs`, `gdfl_tickstore.rs` and `gdfl_nfo.rs` after the merge
(92 mutants: 85, 4 and 3; the same 92 by name as the diff `f9cdb584
297996d9` lists on `feat/db-fix` itself), same flags, two test threads.
Baseline 62 s build, 204 s test, every test passing as the unprivileged user.

The run filled the session's disk allowance at mutant 91 (each job copies
the tree and builds its own target). Four outcomes it reported were the full
disk, not the mutant, and are discarded: three "unviable" were build
failures writing the incremental cache (`234:21` `+` → `-` and `*` in
`take`, `242:5` `record_crc` → 0), and one "caught" (`233:5` `take` →
`[1; N]`) was `alerts::tests::the_older_walk_names_every_record_it_cannot_read`
failing on `ENOSPC`. No other log of this or any earlier run holds a disk
error. Build outputs this session made were removed to free space (the
first coverage build, the workspace-test target, test scratch directories);
nothing tracked.

Of the 92, `moved` settled 87: **70 caught, 12 unviable** (each asks
`Default`, or a constructor, of a type that has none: `Record`, `Refused`,
`Appended`, `NfoFile`, `Sidecar`, `Flock`), **4 missed and 1 timeout**.
The other five — the four voided and the one never run (`242:5`
`record_crc` → 1) — and the five survivors went to the runs below, after
this pass's tests for the survivors, under Gate 18's flags (`movedafter`
with one test thread, since it decides a hang; `moveddisk` with two):

| Mutant | Run | Outcome |
|---|---|---|
| `gdfl_cm_auction.rs:446:45` `==` → `!=` in `parse` (the header refusal's words by version) | `movedafter` | **caught** by `a_bad_header_is_refused_in_its_own_versions_words` (new) |
| `gdfl_cm_auction.rs:497:21` guard `NotFound` → `true` in `read` | `movedafter` | **caught** by `only_a_missing_sidecar_reads_as_none` (new) |
| `gdfl_cm_auction.rs:612:75` `&&` → `\|\|` in `append`'s directory walk | `movedafter` | **caught** by `a_fresh_sidecar_syncs_no_directory_above_the_one_it_was_made_in` (new) |
| `gdfl_cm_auction.rs:535:55` guard `started.elapsed() < bound` → `true` in `hold` (was a **timeout**: waits forever on a held lock) | `movedafter` | **caught** at test 1 of 1,804, 10.0 s, by `a_sidecar_lock_held_past_the_bound_is_refused_in_time` (new, run first by the nextest override) |
| `gdfl_cm_auction.rs:242:5` `record_crc` → 1 (never run) | `movedafter` | caught by `the_auction_sidecar_is_append_only_and_idempotent` |
| `gdfl_cm_auction.rs:535:73` `<` → `<=` in `hold` | `movedafter` | **missed: equivalent.** `started.elapsed()` and `bound` are `Duration`s; `<` and `<=` differ only when the elapsed time equals the bound to the nanosecond, which no test can arrange (and the arm after either is the same refusal one poll later). |
| `gdfl_cm_auction.rs:233:5` `take` → `[1; N]`, `234:21` `+` → `-` and `*`, `242:5` `record_crc` → 0 (voided by the disk) | `moveddisk` | **all four caught**, by `an_eligible_shares_print_is_filed_in_the_sidecar_never_as_a_bar` and `a_flipped_bit_in_price_quantity_or_time_is_refused_by_name` |

**So the 92 end as 79 caught, 12 unviable, 1 equivalent, 0 timeouts.** No
catch rests on a timing test, and no log of `movedafter` or `moveddisk`
holds a disk error. The `gdfl_nfo` and `gdfl_tickstore` changes' seven
mutants were all caught or unviable in `moved` (the nfo row reservation's
`==` → `!=` by `allocation`'s `a_row_of_half_a_million_fields_is_refused_without_holding_them`).

### Equivalent mutants (20, missed by construction)

The old pass's 18, re-confirmed missed under Gate 18's flags (run
`survivors1`; `request_minutes.rs`'s two again in `rmrest`), and the last two
rows, found in new `feat/db-fix` code. Line numbers are the current file's;
`request_minutes.rs` moved by 8 lines since the old report (`325` → `333`,
`339` → `347`).

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
| `gdfl_archive.rs:280:58` | `\|` → `^` in `central_directory` | The shifted two-byte date and the two-byte time share no bit: OR and XOR agree (new `feat/db-fix` code; run `newcode`). |
| `gdfl_cm_auction.rs:535:73` | `<` → `<=` in `hold` | Two `Duration`s equal to the nanosecond: no test can arrange it, and either arm refuses one poll apart (new `feat/db-fix` code; run `movedafter`). |

## Coverage — lines AND branches, production only

The old pass could not measure branches (the pinned 1.97.1 has no
`--branch`). This pass does, with the nightly toolchain, as the unprivileged
user, at **opt-level 0**. That last setting matters: at the workspace's dev
opt-level 3 a small `pub const fn` (`scrub::contract_of`, the `fold::Ladder`
methods) is inlined across crates into the integration-test binary and the
nightly report loses its counters, so it reads 0 hits though tests call it.
At opt-level 0 nothing is inlined and the count is honest. Measured on the
final head `b86d5a0e` (the merged tree, with every test of this pass); every
test passes in this run.

| File | Lines | Branches |
|---|---:|---:|
| `cash_session_cache.rs` | 435/454 | 47/48 |
| `fold.rs` | 618/629 | 117/126 |
| `gdfl_archive.rs` | **374/374** | **44/44** |
| `gdfl_cm.rs` | 1,143/1,151 | 263/270 |
| `gdfl_cm_auction.rs` (v2, after the merge) | 372/385 | 67/70 |
| `gdfl_import_scope.rs` | **104/104** | **10/10** |
| `gdfl_nfo.rs` | **620/620** | **124/124** |
| `gdfl_tickstore.rs` | **344/344** | **82/82** |
| `request_minutes.rs` | 589/599 | 88/90 |
| `scrub.rs` | 122/126 | **12/12** |
| **Ten files** | **4,721/4,786 (98.64%)** | **854/876 (97.49%)** |

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
| `gdfl_cm_auction.rs:226` (branch), `228` | `put`'s `get_mut` returning `None` | Every call writes a constant range inside a fixed-size array (`header_of`'s 16 bytes, a record's 32). |
| `gdfl_cm_auction.rs:545` | `TryLockError::Error` in `hold` | `flock` refused by the host for a reason other than contention. |
| `gdfl_cm_auction.rs:569` | `YearMonth::new` refusing a `Day`'s month | `Day` and `YearMonth` share the years 1970–9999. |
| `gdfl_cm_auction.rs:610` (branch), `618` | a sidecar path with no parent | A path built under the store root always has one. |
| `gdfl_cm_auction.rs:612` (branch) | the directory walk meeting an empty path | Only a relative store root walks up to `""`; every caller and test passes an absolute one, and a test of a relative root would have to write into the crate's own directory. |
| `gdfl_cm_auction.rs:633–637` | the sidecar lock not released | A host error on `flock` release. |
| `gdfl_cm_auction.rs:659` | reading the open, locked sidecar failing | An I/O error. |
| `gdfl_cm_auction.rs:714–717` | undoing a failed append failing too | The `seam::FAIL_AFTER` hook fails the write; the `set_len` that undoes it failing as well needs a second host error. |
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
| `a_print_at_ist_midnight_is_its_own_days_and_not_the_day_befores` (adapted) | `tests/cloud_mutants_import.rs` | After the merge `auction::parse` answers a `Sidecar`; the test reads its `.records`. Assertions unchanged. |
| `a_bad_header_is_refused_in_its_own_versions_words` | `gdfl_cm_auction.rs` | A version-1 header with a reserved byte set and a version-2 header with its checksum broken are each refused in their own version's words; both unbroken are read with no record. |
| `only_a_missing_sidecar_reads_as_none` | `gdfl_cm_auction.rs` | `read`: an absent sidecar is `None`; one under a regular file (`ENOTDIR`) is refused naming the path. |
| `a_fresh_sidecar_syncs_no_directory_above_the_one_it_was_made_in` | `gdfl_cm_auction.rs` | A fresh sidecar syncs the store root (parent of the first directory it made) and nothing above it, read through `seam::SYNCED`. |
| `a_sidecar_lock_held_past_the_bound_is_refused_in_time` | `gdfl_cm_auction.rs` | `hold` with a 50 ms bound against a held lock refuses naming the bound, inside a ten-second watchdog (run first under nextest). |
| `a_sidecar_directory_that_cannot_be_synced_is_named` | `gdfl_cm_auction.rs` | A fresh sidecar in a writable, unreadable directory is written and synced, then refused naming the directory not synced; the record stays (where permission binds). |

## Gates

| Gate | Result |
|---|---|
| `cargo fmt --all --check` | **pass** |
| `cargo clippy --workspace --all-targets --locked -- -D warnings`, before the merge | **pass** (exit 0) |
| the same, merged head (`--keep-going`) | **exit 101, one error, in `api`, identical on `feat/db-fix` `297996d9`**: `crates/api/src/logs.rs:1606` `note_request` has 104 lines (`too_many_lines`, limit 100). Every other crate, `pull` included, is clean. `crates/api` is byte-identical to `feat/db-fix`. |
| `cargo test --workspace --locked --no-fail-fast`, as the unprivileged user, merged head `873ec2b5` (every test of this pass; later commits are this report) | **exit 101: 159 test runs (unit, integration and doc) ok; 8 tests fail, none in `pull`, all in crates byte-identical to `feat/db-fix` `297996d9`** (below); 49 min with two build jobs. `pull`: lib 1,364 passed, 0 failed, and every integration binary ok. |
| the same, before the merge (`2deb1ec5`) | exit 101: 159 runs ok; 2 failures, the first two below |
| `cargo test -p pull --locked`, as the unprivileged user | **pass**, on the final head: inside the workspace run above, the final coverage run, and the baselines of every mutation run (the last, `moveddisk`, 1,804 tests) |
| `cargo test -p pull --locked`, as root, final head | lib 1,362 passed, 2 failed, every integration binary ok: the two `gdfl_import` permission tests that fail as root on the base too (`a_day_ledger_that_will_not_take_a_line_leaves_the_day_open`, `a_journal_that_cannot_be_written_fails_the_day_by_name`), both passing as any ordinary user. `gdfl_tickstore`'s third one passes as root now (repaired here). |
| `cargo deny check` (0.19.0), merged head `0a636bbb` | **pass**: advisories, bans, licenses and sources ok; two duplicate-crate warnings (`getrandom`, `syn`). Identical on `origin/feat/db-fix` `297996d9`, whose lock carries part DENY-1's fix. `Cargo.toml`, `Cargo.lock` and `deny.toml` are byte-identical to `feat/db-fix`: never touched here. |
| `cargo deny check`, before the merge (`f9cdb584` base) | red, identical to that base: RUSTSEC-2026-0344 (ruzstd) and RUSTSEC-2026-0346, -0347, -0348, -0349 (rustls). |
| Gate 1d (`gates_tree gate-1d`, built from the tree) | **red on the base** (141 undeclared words in `crates/pull` on `f9cdb584`, 226 on `297996d9`); **this branch adds none** (226 on the head, the same words). The re-applied tests brought three (`store-root`, `proc`, `self`): the first is gone from the test, the other two are declared in `CLOUD_MUT_PULL_LITERAL` with their reason (`/proc/self/status`, the procfs file `read_limited`'s test needs). After the merge the new base dropped its own uses of `08` and `msg`, leaving the old tests as their only source (the invented row date `04/08/2026`, a telemetry key); both declared there too. `gates_tree`'s own 48 tests pass. |

The workspace failures, all in code identical on `feat/db-fix` (this branch
changes no file of `api`, `cli` or `store`, and no non-test line any crate
depends on, so these test binaries are the base's own):

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
  (`4bbd37df`). Fails the same in the real checkout. **Fixed by
  `feat/db-fix` `f8c22b1b`** (BASE-CITED): after the second merge it passes
  on the head (6 of 6).
- After the merge only — `api` `server::head_deadline_body_tests` (AH-02,
  just landed on `feat/db-fix`), five tests:
  `a_body_longer_than_the_head_deadline_reaches_a_slow_reader_whole`,
  `a_chunked_body_reaches_a_slow_reader_whole`,
  `a_request_pipelined_mid_body_waits_its_turn_and_both_arrive_whole`,
  `slow_readers_at_the_cap_are_served_whole_and_the_next_waits`,
  `pauses_under_the_stall_deadline_are_never_cut_and_one_over_it_is`: each
  body is cut after about 5.37 MB of 8 MiB ("cut after 5374638 of 8388608",
  `head_deadline_body_tests.rs:552`). Re-run alone, nothing else running:
  the same five fail (7 of the module's 12 pass), as root and as the
  ordinary user alike.
- After the merge only — `store` `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`:
  "it is a new file, not the old one cut", the two inode numbers equal
  (1310754): this filesystem hands the freed inode straight back. Re-run
  alone three times, as either user: fails every time.

## NEEDS THE MAC

1. **Gate 18 will be red on 17 equivalent mutants when `feat/db-fix` is
   planned against `main`, whatever the tests.** Gate 18 plans every line of
   the PR's diff (`main...feat/db-fix` held 6,241 `pull` mutants at
   `f9cdb584`) and "never excludes a mutation or credits a timeout". 17 of
   this report's 20 equivalents are on lines of that diff (all but
   `fold.rs:272:38`, `272:44` and `1527:9`; measured for 16 at `f9cdb584`,
   and `gdfl_cm_auction.rs:535:73` is new `feat/db-fix` code). No test can kill them; only a production
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
   | `gdfl_cm_auction.rs:535:73` | `if started.elapsed() < bound` → `if bound.checked_sub(started.elapsed()).is_some()` (waits while time is left): no comparison operator, so no `<`/`<=` pair; the two differ only at an elapsed time equal to the bound to the nanosecond, as the mutant does. |

2. **`cli` `generated_search_recovers_same_ordinal_and_refuses_missing_ancestry`
   fails as an ordinary user here and passes as root** (Gates). Not this
   part's crate; sources identical to `feat/db-fix`. The Mac runs as an
   ordinary user, so it will show there if it is not this container.
3. ~~`store` `cited_commits` red on `feat/db-fix`~~: fixed there by
   `f8c22b1b` and merged in; passes on the head. Nothing to do.
3a. **Three more base reds arrived with `feat/db-fix` `297996d9`**, none in
   this part's files, each deterministic in this container (Gates):
   `api`'s five `head_deadline_body_tests` (bodies cut at ~5.37 MB of
   8 MiB), `api`'s `logs.rs` `note_request` over clippy's 100-line limit,
   and `store`'s `staging` dead-writer test (an inode reused at once). The
   first and last may be this container's network stack and filesystem;
   the Mac's run decides.
4. **Two root-only test failures in `gdfl_import`** (outside this part's
   files) pass as uid 65534 and as any ordinary user; nothing to do on a Mac.
5. **A D-number, if wanted.** The nextest order override and the gate-1d
   declaration follow the repo's existing patterns (D-0693, D-0694; the
   per-package `*_LITERAL` groups) but cite no decision of their own; numbers
   are the Mac's to allocate, so none was claimed here.
