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

## Method

| | |
|---|---|
| Tools | cargo-mutants **26.2.0** and cargo-nextest **0.9.145** (both CI's pins), cargo-deny **0.19.0** (CI's pin), cargo-llvm-cov 0.9.1, rustc 1.97.1 (pinned) and nightly 1.101.0 (2026-10-16) for branch coverage only |
| Mutation flags | exactly Gate 18's: `--test-tool nextest --cargo-test-arg=--max-fail=1:immediate --cargo-test-arg=--test-threads=1 --no-shuffle --cap-lints true --cargo-arg=--locked --minimum-test-timeout 900 --timeout-multiplier 2`, `--jobs 2`; the dev profile overridden to opt-level 0, incremental, no debuginfo, for rebuild time (outcomes do not depend on it; timings do). The old pass ran `cargo test`; under nextest each test is its own process and the run stops at the first failure, which is what decides a hang (see Timeouts) |
| User | every mutation, coverage and workspace-test run is as an **unprivileged user** (`mut`, uid 30033) in a mirror of the tree, so permission-bit tests run their own path as on CI and the operator's machine; the container itself runs as root |
| Coverage | `cargo +nightly llvm-cov -p pull --branch`, production lines and branches only (each file cut at its first `#[cfg(test)]` module) |

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

## Done

- Re-applied the 2026-10-16 tests onto the newest `feat/db-fix`.

## Left

## Gates

## NEEDS THE MAC
