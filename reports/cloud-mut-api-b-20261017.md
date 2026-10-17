STATUS: IN PROGRESS

# Cloud mutation and coverage pass B, continued — `crates/api`, 2026-10-17

Branch `cloud/mut-api-b-20261017`, cut from `origin/feat/db-fix` at
`f9cdb584` (2026-10-17 12:16 IST). It carries the 16 Oct pass
(`origin/cloud/mut-api-b-20261016`, cut from `cloud/base-20261016` =
`c9e3472e5`, the snapshot of `feat/db-fix` at `4756cce0`) re-applied with
`git apply -3`, then finishes what that pass left open.

**`origin/feat/db-fix` moved during the pass** (R3-09, R4-06, AG-04, DENY-1,
BASE-GATE10 and the CLOUD-ATTACKO1 merge-back), and its head `dd7868a5` is
merged in at `23bea1db` with no conflict. Afterwards the branch differs
from `feat/db-fix` only in this pass's fourteen files (the scoped sources,
their test files, `credential_law.rs`'s test script, `scratch.rs`,
`emitted.rs` and this report). Every `feat/db-fix` change is kept, and
`Cargo.toml`, `Cargo.lock` and `deny.toml` are byte-identical to it. The
merged `api` lib suite passed 2,501 of 2,501 before any mutation job ran
on it. Mutation jobs that started after the merge run on the merged tree;
earlier jobs ran on `f9cdb584` plus the pass, and phase 2 re-runs every
mutant in code changed since then (see Method).

**This report holds no authority** (`CLAUDE.md` §10): it is evidence of what
was measured on this date, on this machine.

## Scope

The 16 Oct pass's seven files in `crates/api/src`: `audit.rs`,
`autopilot.rs`, `emitted.rs`, `import_journal.rs`, `pullrun.rs`,
`scratch.rs`, `server.rs`, plus the test files it added
(`server_cloud_mutant_tests_{a,b,c}.rs`, two tests in
`census_request_tests.rs`) and the one this pass adds
(`server_cloud_mutant_tests_d.rs`).

`emitted.rs` and `scratch.rs` are `#[cfg(test)]` modules (`lib.rs`), so
cargo-mutants lists **no** mutant in them, and their every line is covered.
Mutants listed on the new base (`cargo mutants --list -p api -f <file>`):

| File | Mutants |
|---|---:|
| `audit.rs` | 371 |
| `autopilot.rs` | 371 |
| `import_journal.rs` | 127 |
| `pullrun.rs` | 466 |
| `server.rs` | 1,447 |
| **total** | **2,782** |

`crates/api/src/bars_route.rs`, `facets.rs` and `livejson.rs` are not
touched (the Mac's CLOUD-ATTACKO1 merge-back owns them). `feat/db-fix`
changed other `api` files since the snapshot (`feeds_route.rs`, `speed.rs`,
`settings_route.rs`, `sweeprun.rs` and more); they are outside half B and
not measured here.

## Re-apply

`git diff c9e3472e5 origin/cloud/mut-api-b-20261016 -- . ':(exclude)reports' | git apply -3`
applied to all eleven files with **no textual conflict**. Three semantic
conflicts surfaced when the crate was built and run; each is resolved keeping
both sides' intent:

1. **`pullrun::Interrupted` gained three fields** (AG-01: `open_count`,
   `also_open`, `runs_unread`). The re-applied test now builds the
   one-run-open case, whose sentence AG-01 keeps byte for byte.
2. **`run_in_over` gained a fifth parameter** (R3-14: the `Held` that keeps
   the store and the port for the caller). The four re-applied calls pass
   `&mut Held::default()`, as the existing serve tests do.
3. **R3-14 repair 5 races the start-up load against the stop**, polling the
   stop first, so five re-applied serve tests with an already-fired stop
   ended before the load. They now stop after it
   (`server::tests::after_the_load`, which waits for the `listening` event;
   the child whose log cannot open serves a real port and stops once it
   answers, `once_answering`).

## Defects found

### Fixed, each with a test measured failing before the fix

1. **A `feat/db-fix` test failed on any host with a free store**
   (`19398209`). `server::tests::run_serves_until_the_signal_and_exits_zero`
   served with an already-fired stop; since R3-14 repair 5 that stop wins the
   race with the load, so the serve exited `0` before reading the universe.
   Measured with `BRUTEX_STORE` naming an empty directory: `left: 0, right:
   3`. CI has no store and returns before that branch; the operator's Mac
   passes only while another server holds the store.
2. **A released serve lock kept its key in a poisoned root set**
   (`0bc97f04`). `take_serve_lock` inserts through a poisoned mutex;
   `ServeLock::drop` and `release_root` skipped the removal on poison. Once
   a panic had poisoned the set, a served root stayed in it after its file
   lock was released, so the next serve of that root in this process was
   handed the in-process pass with **no file lock held**, and a second
   process could take the store beside it. Both now remove through the
   poison. Test: `server::cloud_mutant_tests_d::a_released_lock_gives_its_key_back_through_a_poisoned_set`
   (a child process); against the code before the fix its child's second
   take answered `held: None`.
3. **A doc comment called a reachable arm unreachable.** `Journal::page`'s
   doc said its `seek` refusal could not fire. `lseek` takes a signed
   offset, so a start past `i64::MAX` bytes is `EINVAL`; the doc now says
   so and `a_page_past_every_seekable_offset_is_refused_by_name` drives it.
4. **A dropped failure line panicked a debug build and lost the failure**
   (`48bf6bbb`). Three error events in `server.rs` (`member did not land`,
   `vendor refused a window`, `instrument refused`) `debug_assert`ed
   `is_written() || no sink installed`. An error line is above every floor,
   so that assertion could fail only when the sink DROPPED the line: a full
   disk, or a sink already closed (R3-14). In any build with debug
   assertions (the dev profile the Run button builds) the panic came first.
   It fired before the autopilot's page heard of the member, inside the run
   that refused an instrument, and inside the fetch the vendor refused. On
   a full disk, where a member dies at the store and its line cannot be
   written, that is every member. `telemetry::emit` never panics on a drop
   and counts it in `Sink::health`, and the three sites now do what the
   other emits in `server.rs` do. Test:
   `server::cloud_mutant_tests_c::a_dropped_failure_line_loses_nothing_but_the_line`
   (a child process with a closed sink). On the code before the fix, its
   child panicked at `server.rs:5132`.
5. **A count nobody read** (`a01d032f`). `Journal::measured` computed the
   journal's record count beside its identity, and its only caller dropped
   the count. Two mutants on that division survived because no reader
   could see them. It is now `Journal::identity` and returns the identity
   alone.

### Carried from the 16 Oct pass

- Fixed there, kept here: the spot run's vendor-down breaker could never
  trip; a lost line continuation in `price_group`'s refusal.
- **Found there, NOT fixed, needs the operator's call:** a backend failure
  the vendor names (`Disposition::RetryBounded`) under a status outside
  500–599 is counted by neither `with_retry` nor `laddered` (both count
  `server_errors` only for `500..=599`), so its ladder waits `1000·0²` = 0
  ms five times and the refusal says the vendor failed "0 time(s)". Still so
  on this base (server.rs `laddered` and `with_retry`). Whether Dhan or Groww
  send those names under a non-5xx status is **UNVERIFIED**. The proposed
  patch (count a named `RetryBounded` with the 5xx) changes how fast this
  repository re-asks a live vendor, so it is the operator's.

## Method

- cargo-mutants 27.1.0, cargo-nextest 0.9.145 (the CI pin), cargo-llvm-cov
  0.8.4 (the CI pin), cargo-deny 0.19.0 (the CI pin), toolchain 1.97.1.
- Every test runs as the non-root user `mut`: root ignores mode bits.
- Worker copies of the tree without `.git` (so `cli`'s build script stamps a
  constant and only `api` rebuilds per mutant), each bind-mounted at `/work`
  in its own mount namespace so one target directory serves all of them.
- Mutation builds only: `api` at `opt-level = 0`, incremental, debuginfo off
  workspace-wide, from an untracked `.cargo/config.toml` in the worker
  copies. Gates use the checked-in profile.
- **Phase 1 runs the WHOLE `api` lib suite per mutant** (nextest,
  `--max-fail=1:immediate`), with a per-file nextest tool config that only
  ORDERS the file's own tests first (no test is skipped). Under three
  concurrent copies the unmutated suite passed 2,402 of 2,402 in ~85 s, so
  no load-flaky test was found to skip. Per mutant: ~20 s build, 0–4 s to
  the first failure.
- **A killed build is not an unviable mutant.** One build was OOM-killed
  (four concurrent `api` builds); its mutant (`audit.rs:529:14`) was logged
  unviable and is re-run by name. Dev builds now wait for a window with at
  most one mutation build running.
- **The disk filled once, during coverage run 3** (about 19:01–19:04),
  because one profraw per test process came to about 6.5 GB. That run
  failed and is discarded: one test refused with `DiskFull`, and no report
  was written. One mutant build failed for lack of space
  (`autopilot.rs:4114:33`, recorded unviable) and `w1`'s prebuild after
  the merge failed. Every mutant whose result was written from 18:55 to
  19:10 (26 of them) goes back into phase 2, whatever its result. Space
  was recovered by deleting only stale build caches in this pass's own
  scratch copies (`api` incremental directories no build had touched
  since the merge); nothing of the repository's was deleted. Coverage now
  merges profiles online into a pool of eight (`LLVM_PROFILE_FILE_NAME`
  with `%8m`), and a watchdog reports free space below 4 GB.
- Coverage: `cargo llvm-cov nextest -p api --lib --branch` with
  `RUSTC_BOOTSTRAP=1` on the pinned toolchain (branch coverage is a nightly
  flag; this is measurement only, nothing is committed that needs it).

## Results so far

Phase 1 (the whole `api` lib suite per mutant):

| File | Mutants | Caught | Missed | Timeout | Unviable |
|---|---:|---:|---:|---:|---:|
| `import_journal.rs` | 127 | 116 | **0** | 1 | 10 |
| `audit.rs` | 371 | 321 | 10, all equivalent or removed | 0 | 40 |
| `autopilot.rs` | 214 of 371 so far | 199 | 4, all equivalent | 0 | 11 |
| `pullrun.rs`, `server.rs` | | queued | | | |

Phase-1 survivors and why none needs a test (each re-run in phase 2):

- `audit.rs:527:16` and `535:16`, `<` → `==` and `<` → `>` (4 mutants):
  the loop of a `const _: () = { .. }` compile-time assertion. The mutant
  makes the loop run zero times, so the check is skipped and the crate
  still compiles. Nothing at run time differs. **Equivalent.**
- `audit.rs:652:29`, `655:31`, `658:30`, `661:45`, `>` → `>=` in
  `Drops::peak` (4): a running maximum. On a tie, `>=` assigns the value
  `top` already holds. **Equivalent.**
- `audit.rs:2807:50`, `/` → `%` and `/` → `*` in `Journal::measured` (2):
  a count nobody read. **Removed** (defect 5).
- `import_journal.rs:529:11`, delete `!` in `capped` (the timeout):
  **detected**. Without the `!`, the loop walks `end` down to 0, where
  `is_char_boundary(0)` is always true and `saturating_sub` keeps it at 0.
  The cut never returns, so the over-long-value tests hang to the timeout
  rather than pass.
- `autopilot.rs:671:13` (`probe_secs`) and `1527:17`
  (`FeedState::backoff_secs`), `>` → `>=`: when the shifted wait equals the
  ceiling, both arms return the same value. **Equivalent.**
- `autopilot.rs:863:25` and `864:22`, `<` → `<=` in `month_span`: a max
  and a min of two `Day`s, which are the same `Day` on a tie.
  **Equivalent.**

Coverage (`--branch`, the whole `api` lib suite), uncovered production
lines / lines with an unhit branch side:

| File | Run 1 (re-applied base) | Run 2 (17:40, this pass's tests to then) |
|---|---|---|
| `audit.rs` | 5 / 3 | 1 / 0 |
| `import_journal.rs` | 1 / 1 | 1 / 1 |
| `autopilot.rs` | 29 / 14 | 29 / 8 |
| `pullrun.rs` | 34 / 32 | 4 / 8 |
| `server.rs` | 232 / 49 | 205 / 27 |

Run 4 (merged tree, `2b119504`, 2,501 of 2,501 tests passed): `audit` 1 / 0,
`import_journal` 1 / 1, `autopilot` 29 / 8, `pullrun` 3 / 6, `server` 206 / 22,
`emitted` and `scratch` 0 / 0. Since run 2, tests cover `server.rs`'s
right-hand ISIN disqualifier in `spot_vendor_identity_for` and both `let`
guards on `with_retry`'s named-throttle decrease. The three assertion arms
of defect 4 went with the assertions. Every line and branch side still
unexecuted is listed under **Lines not covered, and why**.

## Lines not covered, and why

Coverage run 4, merged tree at `2b119504`; line numbers are that commit's.
"L" is a line never executed; "B" is a line with a branch side never
taken. Nothing below is left unexplained. Each item is either
**unreachable** in this build, with the reason, or **not driven**, with
the rule or the missing seam that stops a test.

**`audit.rs`**
- L 634, `_ => 0`: the wildcard arm over a foreign `#[non_exhaustive]`
  `DropReason`. Every variant this build knows has its own arm.
  Unreachable.

**`import_journal.rs`**
- L 361: a run index past `u32::MAX` lines needs a journal of more than
  4·10⁹ lines in memory. Unreachable in a test.
- B 467, the `None` side of `self.last.get_mut(slot)`: `slot` comes from
  `ImportKindOrder::of` of a known kind, and the table is sized to
  `ImportKind::ALL`. Unreachable.

**`autopilot.rs`**
- L 319, B 2814 / L 2815: `Day` and `YearMonth` both span 1970..=9999, so
  `year_month()` of a `Day` never fails. Unreachable.
- B 928 / L 929–930: `day < to <= 9999-12-31`, so `day` has a successor.
  Unreachable.
- B 2907, L 2909, B/L 2910–2932, B 3553 / L 3554: `yesterday_ist(now)` is
  `None` only for a host clock outside 1970..=9999. There is no clock seam.
  Not driven.
- B 2963 / L 2964–2968, B 3018 / L 3019: `store_timeframe()` is `None` for
  a rung with no store directory. Every rung the autopilot drives has one
  in this build; the arm is a backstop for a rung added without one.
  Unreachable.
- B 3639 / L 3640: `slot` comes from a survey over the same `feeds`.
  Unreachable.

**`pullrun.rs`**
- B 863 / L 864: `unjournalled_feeds()` is never empty. The vendor table
  ships two archive-folder feeds. Unreachable.
- B 1713 / L 1714: `at` is a position the walk recorded while walking the
  same `read`. Unreachable.
- B 2080 / L 2081: the scan runs only while `skip < min(SCAN, records)`,
  so each page asks for at least one record and gets it or an error.
  Unreachable.
- B 3814, 3854, 3859: `nth` enumerates the groups the run's `feeds` and
  `outcomes` were built from. Unreachable.

**`server.rs`, unreachable in this build**
- L 1903–1908: `VendorCensusNow::census()` is always `Some` (D-3327).
- B 1971 / L 1972: `report_from` always ends its text with a newline.
- B 2739: `slot` enumerates `SpotTarget::ALL`, and the array is that long.
- B 4462: a one-day `Window` of a valid `Day` always builds.
- L 5756–5757: `verdict()` reads `kind_of`, and every `LENGTH_UNMEASURED`
  day has a `SPECIAL_SESSIONS` window, so `SessionLengthUnmeasured` never
  reaches `observed_cash_source_days`. The `NseCash` anchor row is
  verified from `EPOCH_DAY`, so `hours_on` never refuses.
- B 6001: the `vendor != Vendor::Zerodha` side. The function's first guard
  already returned for every other vendor under a non-ISIN policy.
- L 6477: `ledger.source()` is `None` only for a run that never opened,
  and every unblocked `BrokerRun` was opened by `RunLedger::open`, which
  always sets it.
- B 9260 / L 9261–9266, L 9674–9680: `Feed::store_vendor()` is `None` for
  no feed: TrueData and GDFL have store prefixes too.
- B 9750 / L 9751–9757: both archive feeds declare an FNO layout.
- B 11194 / L 11195–11196, B 11662 / L 11663, L 12542: a `Day` and a
  `YearMonth` span 1970..=9999, and a window has at least one day.
- B 12612 / L 12613–12618: `window.verdict` converts the same stamp first
  and skips the row on its error, so the second conversion cannot fail.
- L 15280: `contract_param` re-reads a query `locate_series` already
  accepted with the same parser.
- B 17409: `poll_read` only grows `filled`.
- B 35685 / L 35686, B 35692 / L 35693: the vocabulary table is dense and
  far below `u16::MAX`, so `definition()` always answers.

**`server.rs`, not driven, by rule**
- L 8811–8950 (B 8885): `read_credential` and `credentialed_source` read
  AWS Parameter Store. No test reads a credential or reaches AWS
  (`CLAUDE.md` §8, and this pass's network rule).
- B 34885 / L 34902–34988: the `/universe` crawl fetches NSE's index pages
  from `INDEX_HOST`, and no test may reach a non-loopback host. Driving it
  needs a crawl-source seam.
- B 11586 (`fno_report` after a chain discovery): `pull::chain::month`
  builds discovery URLs from the shipped descriptor, not from the wire's
  base URL, so a loopback credential does not redirect it (the 16 Oct
  pass's incident). Driving it needs a discovery-URL seam.
- B 19492 / L 19494–19513, L 20035: `open_in_browser` past its
  suppression. Every test runs with `BRUTEX_NO_OPEN`.
- L 17711–17719 (`end_served`), L 19228–19230 (`run_held`): the binary's
  entry points. `end_served` calls `cli::cancel::request()`, a
  process-wide cancel that would stop every other test in the binary.
- B 19965 / L 19966–19969: the autopilot's start banner needs a writable
  serve with `BRUTEX_AUTOPILOT=run`, a live autopilot over the real
  credential source.

**`server.rs`, not driven: host resources, the wall clock, a race**
- L 19794–19798 (`local_addr()` failing after a bind), L 19839–19844
  (`keep_port`, no descriptor left to duplicate): driving either means
  exhausting descriptors, which would starve every other test.
- L 19918 (`Loading::Failed`): the start-up load's thread panicked, and
  `Site::serving` has no panic a fixture can trigger.
- B 20049 / L 20050 (`warm_at_start`), B 20055 / L 20056
  (`census_cache::start`, arrived with the merge): only a thread that
  cannot be spawned.
- B 6713 / L 6722–6723 (`finished_day_only`: today, with its session not
  yet settled), L 10320–10323 and L 14253–14256 (`ist_day(now)` failing):
  they read the wall clock, so which side runs depends on the hour the
  suite runs. There is no clock seam.
- B 6495 / L 6496 (`recovery_spot` refused after its run closed): a
  member-failure append refused while the closing append right after it
  lands. A notes file that cannot be written does not refuse an append
  (`with_whole_note` keeps the record and logs). Only a busy journal lock
  refuses one append and not the next, a race inside one call that no
  fixture can time from outside. A test built on a blocked notes path was
  tried and the failure was filed anyway, so the arm is not driven.

## Done

- Re-apply, its three conflicts, the latent `feat/db-fix` test failure.
- Three production fixes above, each with its proving test, plus the dead
  count removed and the wrong doc corrected.
- The newer `feat/db-fix` merged in, with the merged `api` lib suite green.
- 54 new tests (17 Oct): `audit` 4, `autopilot` 4, `pullrun` 18,
  `census_request_tests` 1, `server_cloud_mutant_tests_a` 1, `_b` 1, `_c` 7,
  `_d` 18.
- `import_journal.rs` and `audit.rs` phase 1 complete, with every survivor
  classified above.

## Left

- Finish phase 1 (`autopilot`, `pullrun`, `server`), then phase 2: every
  survivor and every mutant in code changed since phase 1's base, on the
  newest source, against the whole suite.
- The final coverage run, with the remaining gaps listed and reasoned.
- Gates, final.

## Gates

- `cargo deny check`: **green, and equal to the base.** After the merge,
  `advisories ok, bans ok, licenses ok, sources ok`, because DENY-1 bumped
  `ruzstd` and `rustls` on `feat/db-fix`. This pass touches no manifest,
  lockfile or `deny.toml`; all three are byte-identical to `feat/db-fix`
  `dd7868a5`. (Before the merge, both this branch and the base failed
  `advisories` alone, on RUSTSEC-2026-0344 and -0346 to -0349.)
- `cargo fmt --check` clean on every commit.
- One wall-clock test, `coverage::tests::the_lookup_does_not_grow_with_the_universe`
  (`coverage.rs`, which this pass does not touch), failed once while three
  mutation workers loaded every core. Its message was `124 ns against
  734 ns, 501 against 1`. Run alone it passed 3 of 3, and it passed in
  both coverage runs. This is recorded, not hidden. Its premise is
  wall-clock timing, which depends on host load.
- clippy and the workspace tests: final run pending.

## NEEDS THE MAC

- Nothing yet in `bars_route.rs`, `facets.rs` or `livejson.rs`.
