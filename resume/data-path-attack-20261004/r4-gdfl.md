# GDFL round 4: report

Round 4 found four new defects as well as fixing the two items round 3 left open. All six changes are in commit **bda0464** on `claude/attack-gdfl` in `/home/claude/gdfl-wt`. The commit is local and has not been pushed.

The new tests are in `crates/pull/src/gdfl_r4_attack_tests.rs` (line 1 is `#![cfg(test)]`), wired into `gdfl_import.rs` the same way round 3's file is.

**Failures on 631022a.** I added the r4 file to a throwaway worktree of 631022a and adapted it with sed only so it would compile there:
- `BAR_DEFINITION` was replaced by `2`;
- the lines asserting the new `restated` field were removed.

All 10 tests failed there, each for the reason it targets. The worktree was then removed.

| attack | cases | failures on 631022a | fixed? | evidence |
|---|---|---|---|---|
| Open item 1: a day closed `incomplete` is imported again but named nowhere | 3 runs; plus one journal holding a begun day, an `incomplete` day, a legacy `done`, a current `done` and a `definition=1` `done` | **failed**: `resumed == []` | yes, D-3190 | `r4_01_…`, `r4_02_…`. Fix: `name_retried` in gdfl_import.rs, and the `Report::resumed` doc |
| Open item 2: a day done under an older bar definition (or a line with no definition) is never rebuilt | 4 older forms × 3 runs; the D-3170 shape with old bars that differ × 3 runs; 9 malformed `definition=` values | **failed**: skipped (`days_skipped == 1`), and a malformed definition was accepted | yes, D-3191 | `r4_03_…`, `r4_04_…`, `r4_05_…`. Fix: `BAR_DEFINITION`, `definition_of`, and the `Journal` `restated`/`incomplete` sets |
| A name with a dated/monthly shape whose strike does not parse (`100.125`, `100.50`, past `i64`, `-100`, `..5`, no strike), or a name past the 64-byte cap | 13 hand-picked names × 5 filters; 300,000 random names (more than 20,000 of them decode) | **failed**: `LTI06APR24100.125CE` was claimed by both `LT` and `LTI` | yes, D-3192 | `r4_06_…`, `r4_07_…`. Fix: `gdfl_nfo::shaped_underlying` now reads the split from the name's last letters, parses no strike and has no cap |
| Columnar block: many columns, each within the size but together far past it | text and numeric variants, 1,000 columns each against a 64 KB size; plus an honest 64-column block at exactly its size | **failed**: 60 MB of columns decoded first, then refused as `COLUMNAR` for a stray byte | yes, D-3193 | `r4_08_…`. Fix: a running sum of header + text lengths in `columnar` (gdfl_tickstore.rs) |
| A `Tick.sod` ≥ 86,400 passed to the public `convert`/`drive` | 3 stamps × 3 kinds, plus one 2-day run per stamp | **failed**: a 1 Apr row stamped 34:00:00 was filed as a bar for 2 Apr 10:00 | yes, D-3194 | `r4_09_…`. Fix: a `StampPastTheDay` check at the top of `convert` |
| A journal append that fails partway through a run: the next line is glued onto the torn bytes | every strict prefix of a `done` line (keep = 0..len) | **failed**: short prefixes left a foreign line that every later run refused; longer ones lost day 2's line (`done … definition=2 begin stocks 2024-04-02 *`) | yes, D-3195 | `r4_10_…`. Fix: `Journal::write` closes a torn last line with ` (torn)` before appending |

## Tried and found sound (no failure; not turned into tests)

- **Shape uniqueness.** I proved by hand that the dated and monthly shapes, with any letter-free strike, can split a name at only one place. r4_07 also checks this.
- **A late row placed at the next in-order row of any kind.** The placed seconds stay non-decreasing, so the fold cannot refuse a back-step.
- **Census when a re-imported file is refused.** `record_held` appends rows; it never replaces them, so a refused file's census row is not dropped.
- **`OverlapDisagrees` covers a changed bar definition.** The store compares a re-offered day record by record, so a definition change can never overwrite held bars (r4_04).
- **Journal state transitions.** Each was checked against the code: done then incomplete, done(old) then begin then crash, incomplete then begin. r3_02's 1,500 crash histories still pass, now with `definition=` lines.

## Not fixed (open; no test was written, because fixing them needs a design or a sourced fact)

1. **A shapeless undecodable name is claimed by two nested filters.** Such a name has no month after two digits. With filters `LT` and `LTI`, both claim `LTIXYZCE`. A fixed owner would need a fixed universe of names, and D-3167 chose against one. This is recorded in docs/11.
2. **Stated decode lengths have no sourced bound.** A tick-store index is decoded up to the footer's `index_raw_len`, and a raw block up to the entry's `size`. Neither has a sourced upper bound, so a hostile `.bts` file can still be a decompression bomb of its own stated size. Recorded in docs/11; no bound is in the charter (UNVERIFIED).

## Changes to existing tests (they encoded the old format)

- Hand-written `done` lines in `a_torn_journal_line_is_ignored_and_a_foreign_one_refuses_the_run`, `a_torn_journal_line_stays_ignored_on_every_later_run`, `r2_05_…`, `r3_01_…` and `r3_02_…` now carry `definition=2` (or `definition={BAR_DEFINITION}`).
- `per_tick_build_and_per_second_lookup_cost` built 10^6 ticks at three a second from 09:15, which ran to second 366,633 — past the end of the day. It now builds twelve a second from midnight.

## Docs

- `docs/05-decisions.md`: D-3190..D-3195 appended after D-3199.
- `docs/04-invariants.md`: DPT-23..DPT-28 appended to the GDFL table.
- `docs/11-findings.md`: 7 lines appended to the GDFL dispositions section.
- `.github/gates_tree.rs`: `GDFL_LITERAL` now declares `r4-retried r4-restated r4-stamp r4-torn-append restated definition 7`.
- Journal format doc: `Journal::load`'s doc states the three line forms and `definition=`, and the module doc's "Incremental, idempotent, resumable" section describes `incomplete`, `definition=`, `resumed` and `restated`.

## Commands (at bda0464, all in /home/claude/gdfl-wt)

- **Static gates.** `git add -A` then `bash /tmp/claude-0/r3gates-run.sh /home/claude/gdfl-wt`, using the jobs.language-purity steps extracted in round 3 from the unchanged `ci.yml`: **28 passed, 0 failed, 2 skipped (1e, 7)**.
- **Formatting.** `cargo fmt --all --check`: clean.
- **Lint.** `CARGO_TARGET_DIR=/home/claude/gdfl-target CARGO_BUILD_JOBS=3 cargo clippy -p pull --all-targets --locked -- -D warnings`: clean.
- **Tests as root.** `… cargo test -p pull --locked --lib gdfl`: 250 passed, 2 failed. The two failures are the known root-only tests `a_journal_that_cannot_be_written_fails_the_day_by_name` and `a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses`.
- **Tests as nobody.** The same binary copied to `/tmp/gdfl-nobody-r4/pull` and run under `setpriv --reuid=65534 --regid=65534 --clear-groups ./pull gdfl`: **252 passed, 0 failed**.
- **Base check.** The adapted r4 file on 631022a (separate target dir `/home/claude/gdfl-target-base`): 10 failed, 0 passed.

## Timing

These are test-profile timings measured this round with `--test-threads=1`. They are not bench-gate measurements.

`NfoDay::locate` (dpn_15):

| entries | p50 | p99 | max |
|---|---|---|---|
| 10^3 | 66 ns | 429 ns | 44 µs |
| 10^4 | 112 ns | 457 ns | 124 µs |
| 10^5 | 367 ns | 842 ns | 40 µs |
| 10^6 | 742 ns | 1,295 ns | 79 µs |

- **Duplicates (dpn_12):** the median is 57 ns with 1 duplicate and 58 ns with 100,000, so duplicates do not change the cost.
- **Listing size:** cost is not flat in absolute time — about 11× higher at p50 from 10^3 to 10^6 entries. Each lookup is still one probe by its shape; whether the growth is cache misses is UNVERIFIED.

`convert` per tick (whole-run time ÷ ticks, at the new twelve-a-second workload):

| ticks | p50 | p99 | max |
|---|---|---|---|
| 10^3 | 24.0 ns | 59.7 ns | 104 ns |
| 10^4 | 52.5 ns | 60.7 ns | 88.7 ns |
| 10^5 | 95.6 ns | 124.3 ns | 124.3 ns |
| 10^6 | 111.9 ns | 148.6 ns | 148.6 ns |

This is O(rows) per file, and the per-tick figure is not flat.

The one-second store lookup (`first_at_or_after`, a bisection, O(log n)):

| bars | p50 | p99 | max |
|---|---|---|---|
| 10^3 | 17.8 µs | 60.0 µs | 116 µs |
| 10^6 | 56.4 µs | 110 µs | 466 µs |

The new checks are each bounded by what they read: `shaped_underlying` is O(ticker length), the journal's last-byte check is one seek and one byte per append, the columnar sum is O(columns), and the stamp check is O(rows), the same order as the placement.

## Needs vendor data

These are still open from earlier rounds:
- whether NSE or GDFL ever listed LTI or NIFTYIT options from 2019-12;
- the real tick-store FORMAT.md §4;
- the long-dated months 2022-06, 2023-06 and 2023-12;
- whether monthly names continue after the cutover;
- the missing D-2802..D-2807 entries.

New this round: whether any journal already on the operator's Mac holds `done` lines with no `definition=`. Every such day will now be re-imported once and judged by the store.

## Files modified

- crates/pull/src/gdfl_import.rs
- crates/pull/src/gdfl_nfo.rs
- crates/pull/src/gdfl_tickstore.rs
- crates/pull/src/gdfl_r4_attack_tests.rs (new)
- crates/pull/src/gdfl_import_tests.rs
- crates/pull/src/gdfl_r2_attack_tests.rs
- crates/pull/src/gdfl_r3_attack_tests.rs
- crates/pull/src/gdfl_seconds_attack_tests.rs
- .github/gates_tree.rs
- docs/04-invariants.md
- docs/05-decisions.md
- docs/11-findings.md
