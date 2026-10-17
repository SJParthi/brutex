STATUS: IN PROGRESS

# R3-13 — /db day band and row labels (cloud build report)

**Why the first line does not say DONE.** Every goal and finding of R3-13 is built and tested,
and every gate is green on what R3-13 changes: formatting, clippy, its tests, 100% of its
changed lines and branches but one declared line, no surviving mutant, and the measured target
met on the synthetic store. But `STATUS: DONE` needs every gate green, and two are red on
`origin/feat/db-fix` itself, in files R3-13 may not touch. `cargo deny check` fails five
advisories in `Cargo.lock`, and `cargo test --workspace` fails `store`'s `cited_commits`, which
checks the citations in `docs/06-limits.md` against `main`. The front end's W3 (svelte-check, 7
errors, the base's 7) is red on the base too. This cloud machine adds failures of its own: 25
tests fail as root and pass as an unprivileged user, one `store` test fails on this filesystem's
inode reuse, and gate W1 waits for the Mac to rebuild `web/build`. Nothing is left for R3-13 to
build in the cloud. What remains is in NEEDS THE MAC.

## What is left

- Everything under NEEDS THE MAC: the real-store checks and timing, the gates the cloud could not
  close (the suite as a non-root user, the `web/build` rebuild for W1, the store volume's inode
  behaviour), the docs rows to land, and the two notes for other owners.
- The base's own red gates (`cargo deny`'s advisories, `cited_commits`, W3) belong to their
  owners. R3-13 changes none of the files they read.

Branch `cloud/db-R3-13-20261017`, from `origin/feat/db-fix` 3911d1b9c; the newer
`origin/feat/db-fix` was merged in twice as it moved (62396df0, then 50093fa3: AF-02, AF-07,
R3-01, R3-32, R3-35). No merge touched an R3-13 file except `dayflags.rs` (AF-02/AF-07, merged
before any edit).

## Prerequisites

Both merged into `origin/feat/db-fix`:

- `Merge WP-11c: import data-quality labels: minute-sampled days and halts` (c4f6e522)
- `Merge WP-62: pull runtime: calendar horizon, idle stop, per-instrument gates, stop/restart/storage safety` (72c91a46)

## What was built

Every change is in a file R3-13 owns: `crates/api/src/dayflags.rs`, the new
`crates/api/src/dayflags_rules.rs` (declared from `dayflags.rs` by `#[path]`, so `server.rs` is
untouched) and its tests `crates/api/src/dayflags_rules_tests.rs`, `web/src/lib/db/dayflags.js`,
`web/src/lib/db/notes-labels.js`, `web/src/routes/db/DbDayFlags.svelte`,
`web/src/routes/db/DbProvenance.svelte`, `web/src/routes/db/DbBasisBadge.svelte`,
`web/tests/db-dayflags.test.js`, `web/tests/db-labels.test.js`. And this report.

| Finding / goal | What changed | Test (fails before, passes after) |
|---|---|---|
| Goal 1 (band at the grid's rung), DB-053 | `/dayflags.json?timeframe=` counts `held` at the grid's rung, with the feed's finest beside it (`finest`); a Zerodha `1day` session with minutes and no day bar says the day bar is missing; a held day bar is never "no Zerodha bar". The band sends `db.timeframe`. | `the_band_counts_the_asked_timeframe` (asserts the old reading on the same store first), web `R3-13: the band counts the asked timeframe…` |
| Goal 1 (one contract) | `bandInstrument` moved into `dayflags.js` (now testable) and counts only rows at the grid's rung: one contract asks about that contract; one expiry at any strike asks the family with `expiry=`, so its days after the expiry read expired | web `R3-13: one contract is asked about itself…`, `an_option_family_narrowed_to_an_expiry_is_expired_after_it` |
| Goal 2a, DB-031 (scale switch, far months) | the previous `1day` bar: 12 opens back, then up to 144 more months by `stat`, one open; a history found empty is not probed again by `series_start` | `a_scale_switch_is_found_across_far_months`, `the_month_before_is_found_by_stats_within_its_bound` |
| Goal 2b, DB-031 (zero-volume peer, other names) | a share's peer counted under its other names (`brutex_core::alias::LINEAGE`) where its own name holds no month; `peer.as` and the phrase name it | `a_zero_volume_peer_is_found_under_the_shares_other_name` |
| Goal 2c, DB-B20 (frozen index, value outside the day range) | Zerodha: each index day's minutes scanned once (≤1,440 reads) and kept per (device, inode, first row) with its end row, a page's first scans on one scoped thread a month, each on the month's own handle (`scan_months`); runs of ≥180 s unchanged outside any recorded halt, and the minute extremes against the day bar. GDFL: the day ledger's `flat_max_s`. Band kinds `frozen`, `outside_range` | `a_frozen_index_run_is_flagged_on_both_feeds`, `a_frozen_run_is_three_unchanged_minutes_in_a_row`, `a_ledger_run_inside_the_halt_is_not_frozen`, `a_days_scan_is_kept_once_and_a_grown_day_replaces_it`, `a_pages_first_scans_run_a_month_a_thread_and_are_kept`, web `DB-B20, DB-A15, goal 3…` |
| Goal 2d, DB-029 (late rows on a recounted day) | `LateLedger`, a tail read of the journal's closing lines: a `recounted` day keeps the close-before-recount's `late=` and `max_back_s=` (`late_line`, `late_from`); the phrase says the count is over all of the day's files | `a_recounted_day_keeps_its_late_rows`, `the_late_ledger_keeps_the_last_counting_close` |
| Goal 3 (bar definition 2/3/4) | `bar_definition` on every GDFL record (the journal's deciding line, else the day ledger's), current by `pull::gdfl_import::definition_is_current`; a phrase on every option day and every stale day; band kind `old_definition`; the rows' Source line and the drawer name it | `an_option_day_names_its_bar_definition`, `a_spot_day_names_its_definition_only_when_it_is_not_current`, web `R3-13 goal 3…` |
| Goal 4, DB-G38, DB-G40 (adjusted / quantised Zerodha share runs) | `DbProvenance` asks `/sources/day.json` over the page's share days (≤31) and labels each row's Source line and Details with the run its day belongs to: first and last day, the factor(s) served, the grid, and whether the run may go on past the days read. `DbBasisBadge` says where. Whether a % statistic is refused stays D-3323's | web `R3-13 goal 4…` |
| Goal 5a (unknown quality field) | `parse_quality` keeps a field it has no name for, served under its own name and listed in `unknown`; the band draws them | `a_day_field_the_reader_has_never_seen_is_served_and_drawn` (Rust and web) |
| Goal 5b (one function for peers and rungs) | `rules::feed_rungs`; `peer_of` and `finest` delegate to it | `the_band_rungs_come_from_one_function` |
| SC-CLOSE-8 | phases built with the share's dated master flag (`rules::CasFlags`, read as `bars_change.rs` reads it; a failed load is kept by the request only); `calendar.cas`; a note per unread day | `a_shares_phases_follow_its_dated_closing_auction_flag`, `no_flag_is_read_where_none_applies`, `a_flag_refused_by_one_request_is_read_by_the_next` |
| SC-CLOSE-9 | band `cas` only where the flag is not 0, and its word "closing auction from 15:15" only where it is 1 ("closing-auction window, flag not held" otherwise, so the chart keeps its unverified window); "kept apart" only where the month's `.auc` exists (`auction_sidecar`), in the server's phrase and the band's line | `an_auction_print_is_said_kept_apart_only_where_a_sidecar_is`, web `SC-CLOSE-9…` and the `cas` assertions |
| SC-CLOSE-10 | the session's last print is called the auction's only where it equals the official close (0 bps); no zero-book claim; prints at that same price say which is NOT PROVED | web `SC-CLOSE-10: a last print that does not equal the official close…` |
| SC-CLOSE-11 | already fixed on the base by AF-02 (`Names` in `journal_rows`; tests in `crates/api/src/imports_filter_tests.rs`). Nothing added. | AF-02's |
| SC-CLOSE-12 | a day after a contract's (or a narrowed family's) expiry is `expired`: owed 0, one phrase, band kind `expired` (quiet), never a hole or "not imported yet" | `a_day_after_expiry_is_expired_and_owes_nothing`, web `SC-CLOSE-12…` |
| CMP-06 | a GDFL share's day names the market index's sampling (`of: "market"`, NIFTY) and its own first held second | `a_shares_day_names_the_markets_sampling_and_its_first_second` |
| DB-A15 | a closed day the peer feed traded is said as evidence independent of the series; band kind `peer_on_closed`. The stale comment in `crates/pull/src/calendar.rs` is NOT an R3-13 file and is left for its owner | `a_closed_day_the_peer_traded_is_said_as_independent_evidence` |

Existing tests changed because the rule they pinned changed on purpose: the late-row phrase
(now "over all of the day's indices files"), WP-11c's "a share names no sampled day" (CMP-06),
the web weekend fixture whose peer "held 375 bars" on a Saturday (now 0: a closed day the peer
traded is the new `peer_on_closed`), the web `cas` assertions (SC-CLOSE-9), and the web auction
text "as it does on this day" (SC-CLOSE-10).

### A second agent's review, and what it changed

A reviewer agent read the branch's diff against the base and reported nine defects. Each was
checked against the code; all nine were real and each is fixed with a test that fails on the
unfixed code:

| # | Defect | Fix | Test |
|---|---|---|---|
| 1 | Goal 4 read `share_basis`, a key `/sources/day.json` does not serve (`sources.rs` serves `basis`), so no run was ever named on a real answer | `basisRunsOf` reads `basis`; a day is measured only where `basis.factor` is a number, so a weekend or a holiday inside a run neither joins nor breaks it, and the run's open edges are judged over measured days | web `R3-13 goal 4…` (rewritten on the served key, with unmeasured days inside and at both edges) |
| 2 | A GDFL ledger run starting at 09:21:09 on 2020-03-13, a second before the index halt, was flagged frozen | only the run's seconds outside `[from, resume)` count (`outside_halt_s`); a run with no stamp on a halt day is not placed, so not flagged; a partial run says "N s of it outside the recorded halt" | `a_ledger_run_inside_the_halt_is_not_frozen` (extended) |
| 3 | A closed day, and a session after today, read the day's master and noted a refusal | `CasFlags::on_session`: a closed day reads nothing; a session after today is `basis: "not_yet"`, never a refusal | `a_flag_is_read_only_on_an_open_session_up_to_today` |
| 4 | "kept apart" was said of an index record, and the sidecar-less wording did not name the month | three wordings: sidecar held, sidecar not held, and "counted by the import" where no share month applies | `an_auction_print_is_said_kept_apart_only_where_a_sidecar_is` |
| 5 | A close written by a filtered run (`BANKNIFTY,NIFTY`) was said to count "all of the day's files" | `filter` and `late_scope` served; the phrase names the files the run read | `a_filtered_runs_late_rows_are_over_the_files_it_read` |
| 6 | A share asked at `1min` on GDFL was judged by `held.rung`, so a day with no trade read as a store gap | trade-driven is decided by the finest rung | web `R3-13 review: a share on a coarser grid…` |
| 7 | Zero-volume minutes were read from `held`, which on a `1day` grid counts day bars | counted from `finest` where the grid's rung is another | same test |
| 8 | The band's "kept apart" count included days whose flag no master holds | `casCounts` in `dayflags.js` counts apart only flag-1 days; the component uses it | web `R3-13 review: a closing-auction day whose flag no master holds…` |
| 9 | `imports::day_band_json` (the L-IA-39 bench's entry) parses `Asked` alone, so it cannot ask for a rung or an expiry | `dayflags::query_of` is now the one parse of the band's query, used by the route; `imports.rs` is not an R3-13 file, so its one-line change is in NEEDS THE MAC | `an_option_family_narrowed_to_an_expiry_is_expired_after_it` (extended) |

## Measured target (synthetic store, this cloud machine)

`the_day_band_is_timed_cold_warm_concurrent_and_growing` (`#[ignore]`; run with `--ignored`):
92-day pages of NIFTY on both feeds, RELIANCE on both and one NIFTY contract, 375 bars a session
at each finest rung. Warm: n = 200 a page, 1,000 over the five pages. Cold: n = 20, each request
on a fresh copy of the store, so every memo is empty while the page cache is warm. 60 clients:
n = 1,200 while an import appends to the journal. Growth: the warm p50 with 10x and 100x the
history that no page reads. The final binary (this branch's head) and the base (the same
harness in a binary of `origin/feat/db-fix` 62396df0) were each run three times, alternately, on
the same 4-core machine with nothing else running. The base's third run stopped at its 100x copy
when the disk filled: its harness predates the fix that makes the harness remove its own
fixtures. **Not a gate: one synthetic store on one machine. The real store's figure is the
Mac's (NEEDS THE MAC section 2).**

| p50 / p99 / max, ms (three runs) | base | branch |
|---|---|---|
| warm, 1 client | 6.76 / 15.93 / 20.91; 6.43 / 15.45 / 17.25; 6.97 / 15.30 / 23.79 | **4.59 / 12.28 / 13.09; 4.44 / 9.71 / 13.17; 4.68 / 8.95 / 10.89** |
| warm, 1 client, import writing | 8.19 / 18.67 / 31.04; 7.83 / 17.93 / 23.07; 7.61 / 18.36 / 24.89 | **5.06 / 11.65 / 12.66; 5.05 / 10.92 / 13.85; 4.87 / 11.22 / 18.64** |
| 60 clients, import writing | 400.0 / 1,262.9 / 1,522.8; 233.6 / 1,135.8 / 1,279.8; 267.4 / 1,161.9 / 1,775.4 | **206.7 / 900.3 / 1,047.5; 165.4 / 1,064.0 / 1,556.9; 73.8 / 1,074.2 / 1,156.6** |
| cold, Zerodha NIFTY page (p99 = max at n = 20) | 5.10 / 23.69; 4.78 / 11.04; 5.30 / 8.55 | **4.03 / 8.45; 4.39 / 8.57; 5.39 / 7.41** |
| cold, at 10x history | 8.04 / 12.24; 5.50 / 7.64; 6.15 / 12.35 | 5.04 / 9.96; 6.01 / 7.42; 5.29 / 10.02 |
| cold, at 100x history | 5.13 / 7.06; 5.18 / 8.63; (not reached) | 4.49 / 9.04; 4.32 / 10.15; 4.68 / 8.29 |
| growth, warm p50 at 10x and 100x over 1x | 0.97 / 1.19; 1.00 / 0.98; (not reached) | 0.93 / 1.04; 0.94 / 0.98; 0.92 / 0.89 (gate ≤ 3.00) |

By the median of three runs, every figure of the target is lower on the branch: warm p50 4.59
against 6.76 ms, p99 9.71 against 15.45, max 13.09 against 20.91; with an import writing 5.05
against 7.83, 11.22 against 18.36, 13.85 against 24.89; 60 clients 165.4 against 267.4, 1,064.0
against 1,161.9, 1,156.6 against 1,522.8; cold 4.39 against 5.10, max 8.45 against 11.04. Run by
run there are three overlaps, all said here: the branch's third cold p50 (5.39 ms) is 0.09 ms
above the base's highest (5.30), its second cold max (8.57 ms) is 0.02 ms above the base's
lowest (8.55), and its second 60-client max (1,556.9 ms) is above the base's lowest (1,279.8). One cold figure beyond the target is higher: at 100x history the branch's cold p99
(8.3 to 10.2 ms) is above the base's two runs (7.1 and 8.6 ms), while its cold p50 is lower. A cold
DEVICE (page cache dropped) is NOT MEASURED.

**How the cold page stopped being the slower figure.** DB-B20's scan reads each index day's
minutes once per process before its memo holds them: 63 sessions × 375 reads on the cold page.
The first build scanned them in order, and the cold p50 was 6.51 ms against the base's 4.40. Three
designs were measured. Lanes of eight days sharing the count's one handle: p50 3.95 to 4.19 ms,
but a slow sample of 16 to 18 ms in every run, because a handle verifies a sealed month's reads
through ONE cached block behind one lock, and lanes on different days evicted each other's
block. Lanes each opening the month again: max 9.4 to 11.3 ms, but p50 6.0 to 6.4 ms, since an
open is a generation lookup, a shared lock and a header check. The design kept is one scoped
thread a month, on the handle the count has already opened (`scan_months`): no shared block and
no extra open, and a page the memo holds spawns no thread.

**And the warm page.** The first measurement had the branch slower than the base on warm pages
too (p50 8.3 ms against 6.4). Callgrind (`the_day_band_pages_for_a_profiler`) showed 43% of
`/dayflags.json`'s instructions in `serde_json` copying values: `json!` serializes every `Value`
it is handed into a new one, so each record's nested objects, and then all 92 records, were
deep-copied, and the base pays this too. The records, the day's events and the answer's envelope
are now built by moving values in (`dayflags::object`).

## Gate results (this cloud machine)

On the final tree: this branch with `origin/feat/db-fix` f9cdb584 (R3-06, R3-14) merged in. The
cloud session runs as root, which matters for one gate below.

- `cargo fmt --all --check`: **clean.**
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: **clean.**
- `cargo test --workspace --locked`: **9,097 passed, 27 failed, 49 ignored over 221 test
  binaries; none of the 27 is in an R3-13 file or reads one.** 25 fail because the cloud session
  runs as root, which ignores file modes: 12 in `api` (`logs::tests` x3, `pullrun` x3,
  `server::feeds_route` x1, `server::tests` x5), 3 in `cli`, 3 in `pull`, 3 in `store`
  (`settings::tests`) and 4 in `telemetry` (`sink::tests`). Every one passes when the same binary is
  run as an unprivileged user (`runuser -u nobody`), and the 12 in `api` fail identically on the
  base's binary as root. `store`'s `staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported`
  asserts that a new file gets a new inode, and this container's filesystem reuses an inode number
  at once, so it fails as `nobody` too. `store`'s `cited_commits` fails because `docs/06-limits.md`,
  which this branch leaves as `origin/feat/db-fix` has it, cites nine `feat/db-fix` commits not yet
  on `main` (72c91a46, the WP-62 merge, among them). The known-slow `cli` pool test was skipped, as
  `CONTEXT.md` directs. That run built `api` before the parallel first scan, so the `api` package
  was run again on the final code: 2,147 passed, 13 failed, 10 ignored. The 13 are
  the 12 root-only tests above, which pass when the same binary is run as `nobody`, and
  `coverage::tests::the_lookup_does_not_grow_with_the_universe`, a nanosecond timing test in a file
  R3-13 does not touch (`coverage.rs`). It measured 1 ns against 8 ns while 2,000 tests ran beside
  it, passed in the two `api` runs before this one, and passes alone three times in three.


- `cargo deny check`: **advisories FAILED; bans, licenses and sources ok.** RUSTSEC-2026-0344,
  -0346, -0347, -0348 and -0349 (`ruzstd` 0.8.2 under `lake` and `pull`; `rustls`). The same five
  fail on `origin/feat/db-fix`; `Cargo.lock` is not an R3-13 file, and this branch leaves it
  unchanged.
- **Coverage, Rust: every changed line but one declared one, and every branch.** Of the production
  lines this branch adds or changes in `dayflags.rs` and `dayflags_rules.rs` (the diff against
  `origin/feat/db-fix`), 1,201 of 1,202 run, and 172 of 172 branches on them are taken. THE ONE
  LINE NOT RUN IS DECLARED, as gate 20 declares the logger's: `open_months`'s
  `Err(why) => Opened::Refused(why.to_string())`, the base's own arm for a day whose month will
  not name a file, moved verbatim out of `counts`; `pull::session::Day::year_month` documents that
  refusal unreachable through a validated `Day`, so no input reaches it. Every other line runs; the 27 changed lines of
  `dayflags.rs`'s own test module run too. Measured with `cargo-llvm-cov` 0.9.1 `--branch` on a
  nightly toolchain (rustc 1.101.0-nightly 2026-10-16; branch coverage needs nightly, and the
  repository's pinned 1.97.1 is untouched), opt-level 0, over the `server::dayflags` tests only,
  so a line counts only if a day-flag test runs it. Reaching 100% took these tests: a month
  before the page that will not open, near and 22 months back; a page in 1970-01; a month cut
  short under an open handle, and the same day scanned in place; a late ledger that will not
  read; a day bar session with no minutes; a peer across three months under two names; the
  sidecar's one-stat-a-month memo; a page's first scans a month a thread. It also took the removal
  of every line that existed only for an impossible case: the answer's
  envelope and each count are built by `object`, with no `as_object_mut` fallback; the month
  step is one `month_before` (`Day::year_month` documents its refusal unreachable); a frozen
  minute is `high == low`, since the store refuses any bar whose open or close lies outside its
  range (`store::file` `survey`). The whole `api` crate is not at 100% on the base either (CI's
  floor is 90% lines, 89% regions over the workspace), and its figure was not taken.
- **Coverage, front end: 100% of the changed lines and branches.** Of the lines this branch adds
  or changes in `web/src/lib/db/dayflags.js` and `notes-labels.js`, 381 of 381 run, and 268 of
  268 branches are taken (`node --test --experimental-test-coverage`, Node 22, the whole web
  suite). Whole files: `dayflags.js` 100% lines; `notes-labels.js`'s uncovered lines are all the
  base's. The `.svelte` components have no coverage tool here; their logic was moved into the
  two modules where it can be measured (`casCounts`, `bandInstrument`, `bandExpiry`).
- `cargo mutants`: **no surviving mutant.** `cargo-mutants` 27.1.0 over every
  mutant of this branch's diff in `dayflags.rs` and `dayflags_rules.rs` (`--in-diff` against
  `origin/feat/db-fix`), each run against the `server::dayflags` tests at opt-level 0. A first
  pass, on 4ecb39e1, was stopped by the session's two-hour limit after 234 of its 337 mutants; it
  missed 15. The final pass ran over the final production code's 346 mutants, in two shards: 287
  caught, 48 unviable (they do not compile), 10 missed, all ten in `dayflags_rules.rs`, which the
  first pass never reached; one mutant was not run because the disk filled. Each of the 25
  survivors now has a test that kills it, except three that could not be killed because they
  changed no answer: an `&&` turned `||` in `PageHeld::count` only added a read that no record of
  that page shows. Those three conditions are now one `match` arm a page, and a deleted arm
  changes an answer. Finally, the 55 mutants of the functions the final pass's survivors were in,
  and the one it did not run, were run again on the final code: 53 caught, 2 unviable, 0 missed.
  Between the final pass and that run only tests changed. The scan was then rebuilt twice for the
  measured target (a handle a lane, then a month a thread), and the mutants of every function
  those two changes touched were run on the code as it now stands: 17, then 19, with 0 missed
  (the last: 14 caught, 5 unviable).

- Web tests, `node --test "web/tests/*.test.js"`: 1,417 tests, 1,407 passed, **1 failed, 9
  cancelled, none in an R3-13 file.** The failure is `db-search.test.js`'s "p99 … inside one
  frame (16 ms)", a timing test, run while two mutant builds and the workspace build held all
  four cores: it fails the same way on a copy of `origin/feat/db-fix` under the same load, and
  passed alone on this branch when the machine was quieter. The 9 cancelled are timer tests in
  `ask.test.js`, `census-worker.test.js` and `store-watch.test.js` ("Promise resolution is still
  pending"), cancelled on the base too.
- `npm run check` (svelte-check): **7 errors, the same 7 as on `origin/feat/db-fix`**, none in an
  R3-13 file (`gap-verdict.js`, which arrived with R3-06; `ingest-months.js` and its test;
  `ingest/+page.svelte`; `settings-ceilings.test.js`). CI's W3 ceiling is 0, so W3 fails on the
  base as well.
- Gate W1 (`web/build` matches `web/src`): **fails on this branch until the Mac rebuilds the
  bundle.** `web/build` is not an R3-13 file. The front end builds from this tree (`npm run build`
  on a copy, exit 0); the rebuild is in NEEDS THE MAC section 1.

## NEEDS THE MAC

Nothing here was run in the cloud: there is no market data store, no `:8080` app and no shadow
server. Every check is read-only; none POSTs and none touches `:8080`. Run from a worktree of
`cloud/db-R3-13-20261017` merged as the runbook merges, with the machine's build queue first on
`PATH` and a target directory of its own:

```sh
export PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH
export CARGO_TARGET_DIR=<this check's own target dir>
STORE=/Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store   # read only, never written
```

`P` is the port of a FRESH shadow server of this branch started with `state/fix/shadow.sh` and
`state/fix/serve.sh` (never `:8080`, never another agent's shadow), and

```sh
GET() { curl -s --noproxy '' "http://127.0.0.1:$P$1"; }
```

### 1. The gates the cloud could not close

```sh
# The whole suite as the operator (not root). In the cloud, as root, 15 tests fail because root
# ignores file modes: 12 in api (logs::tests x3, pullrun x3, server::feeds_route x1, server::tests x5)
# and 3 in cli (scrub_blocks x1, watchdog_command x2). The 12 api ones fail identically on the base
# binary as root, and all 15 pass when the same binaries are run as an unprivileged user.
cargo test --workspace --locked

# cargo deny: the five advisories in the gate results fail on the base too; Cargo.lock is not R3-13's.
cargo deny check

# Gate W1 (web/build matches web/src): web/build is not an R3-13 file, so this branch leaves the
# bundle as feat/db-fix built it, and W1 fails until it is rebuilt from the merged tree. The cloud
# built it from a copy of this tree (`npm run build`, exit 0); the Mac rebuilds and commits it:
npm --prefix web ci && npm --prefix web run build && git add web/build
```

```sh
# The scan memo (DB-B20) and the base's ledger tail readers take a file's (device, inode) for its
# identity. This cloud container's filesystem reuses an inode number at once (store's
# staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported fails here).
# Say what the store volume is, and run that test with its scratch files ON that volume (a new
# directory beside the store, never inside it): green means a replaced month is never taken for
# its old self there.
diskutil info /Volumes/WD_BLACK | grep -E 'File System Personality|Type \(Bundle\)'
mkdir -p /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tmp-r3-13-inode
TMPDIR=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tmp-r3-13-inode \
  cargo test -p store --locked --lib -- staging::tests::a_dead_writers_leftover_loses_its_name_and_its_bytes_are_reported
```

### 2. Real-store tests and the measured target

```sh
# The existing real-days test (read only), with the new rules on:
BRUTEX_DAYFLAGS_STORE="$STORE" cargo test -p api --locked -- --ignored \
  the_real_days_are_labelled_and_the_answer_is_timed --nocapture

# The synthetic timing on the Mac (p50/p99/max and n; cold and warm; 1 and 60 clients; an import
# writing; 1x/10x/100x history; asserts warm 100x/1x <= 3):
cargo test -p api --locked -- --ignored the_day_band_is_timed_cold_warm_concurrent_and_growing --nocapture

# The real-store /dayflags.json figure: run once against a fresh shadow of the current feat/db-fix
# (before) and once against this branch's (after), 92 days, n = 200, one client; repeat each
# line with 60 clients (xargs -P 60). The first request after a restart is the cold one.
for q in 'feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2024-07-01&to=2024-09-30' \
         'feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2024-07-01&to=2024-09-30' \
         'feed=zerodha&instrument=NSE-CASH-RELIANCE&from=2024-07-01&to=2024-09-30' \
         'feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2024-07-01&to=2024-09-30' \
         'feed=gdfl&instrument=NSE-FNO-NIFTY-2019-11-28-1200000-CE&from=2019-09-01&to=2019-11-30'; do
  for i in $(seq 200); do
    curl -s -o /dev/null -w '%{time_total}\n' --noproxy '' "http://127.0.0.1:$P/dayflags.json?$q"
  done | sort -n | awk -v q="$q" '{v[NR]=$1} END {print q, "p50", v[int(NR*0.5)], "p99", v[int(NR*0.99)], "max", v[NR], "n", NR}'
done
```

The figures, with n and the method, go into the `docs/06-limits.md` row in section 4.

### 3. Each finding on the real store (expected answers from the findings' evidence)

```sh
# SC-CLOSE-8: a flag-0 share owes 375, a flag-1 share 360; calendar.cas names the master.
GET '/dayflags.json?feed=zerodha&instrument=NSE-CASH-DALBHARAT&from=2026-08-26&to=2026-08-31'
#   expect 08-28 and 08-31: calendar.cas.eligible false, owed_minutes 375, unverified_minutes 0
# SC-CLOSE-8 (review 3): sessions after today read calendar.cas.basis "not_yet", and no note
#   says "could not be read"; closed days carry calendar.cas null.
GET "/dayflags.json?feed=zerodha&instrument=NSE-CASH-RELIANCE&from=$(date +%F)&to=$(date -v+10d +%F)"
# SC-CLOSE-9: /db, NSE-CASH-DALBHARAT, September 2026: no flag-0 day says "closing auction from
#   15:15"; no line says "kept apart from trades" unless the month holds an .auc sidecar.
# SC-CLOSE-10: /db, GDFL RELIANCE 1s, 2026-08-20, 15:15-15:29 (the window below): the 15:15:01
#   bar is NOT labelled "closing auction print"; Details says it differs from the official
#   close by -16.75 bps; no text anywhere says "carries zero bid and ask".
GET '/bars/window.json?feed=gdfl&key=NSE-CASH-RELIANCE&timeframe=1s&from=2026-08&to=2026-08&from_day=2026-08-20&to_day=2026-08-20&tod_from=15:15:00&tod_to=15:29:59&limit=50&sort=ts&dir=desc'
# SC-CLOSE-11 (AF-02's fix, re-checked): import.outcome is not "none", the line is the
#   "done options 2026-08-03 BANKNIFTY,NIFTY" one; its late phrase, where it has late rows,
#   names "the day's options files its run read (BANKNIFTY,NIFTY)" (review 5).
GET '/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY-2026-08-04-2700000-CE&from=2026-08-03&to=2026-08-04'
# SC-CLOSE-12: expired, owed 0.
GET '/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY-2026-08-04-2700000-CE&from=2026-08-05&to=2026-08-05'
#   expect calendar.expired true, calendar.owed_minutes 0, contract.expired true
# Goal 1: a Zerodha 1day page names a held-minutes day with no day bar as missing.
GET '/dayflags.json?feed=zerodha&instrument=NSE-INDEX-INDIAVIX&from=2019-07-01&to=2019-07-05&timeframe=1day'
#   expect 2019-07-02 held.bars 0 at 1day (DB-053 says INDIAVIX misses it at 1day and 1min:
#   then finest.bars is 0 too, and the phrase is "no Zerodha 1day bar on this session")
# Goal 1 probe (operator's row 131): /db, NIFTY, expiry 28 Mar 2019, any strike, window
#   2024-10-01..2024-10-31: every day of the band reads "after expiry", none a hole. The band asks:
GET '/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY&from=2024-10-01&to=2024-10-31&expiry=2019-03-28'
# DB-B20: both feeds flag 2022-03-07; Zerodha names the 10:07 high above the day bar's.
GET '/dayflags.json?feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2022-03-07&to=2022-03-07'
#   expect index.frozen.flagged true (longest run 10:08-10:21 or the 09:57 run), and
#   index.outside_day_range.above_by 22985, above_at "10:07"
GET '/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2022-03-07&to=2022-03-07'
#   expect index.frozen from the day ledger where its line carries flat_max_s (GDFL's day is
#   refused, DB-N26: then index.frozen is null and says so).
# DB-B20 (review 2): 2020-03-13's run from 09:21:09 is the halt's, never frozen, on both feeds.
GET '/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-03-13&to=2020-03-13'
#   expect index.frozen.flagged false, index.frozen.outside_halt_s below 180
GET '/dayflags.json?feed=zerodha&instrument=NSE-INDEX-NIFTY&from=2020-03-13&to=2020-03-13'
# DB-031 far months: the NIFTYGSCOMPSITE re-scale is impossible_jump.
GET '/dayflags.json?feed=zerodha&instrument=NSE-INDEX-NIFTYGSCOMPSITE&from=2018-01-01&to=2018-01-05'
# DB-031 zero volume: ABB, BPCL, WIPRO 2018-09-05/06 and 2018-10-03 read zero_volume_peer_traded.
GET '/dayflags.json?feed=zerodha&instrument=NSE-CASH-ABB&from=2018-09-05&to=2018-09-06'
#   and a renamed share (ZOMATO, its GDFL seconds filed as ETERNAL): peer.as "ETERNAL"
GET '/dayflags.json?feed=zerodha&instrument=NSE-CASH-ZOMATO&from=2024-10-01&to=2024-10-31'
# DB-029: late rows and the UTC burst, and a recounted options day keeps its late rows.
GET '/dayflags.json?feed=gdfl&instrument=NSE-INDEX-NIFTY&from=2020-10-15&to=2020-10-15'
#   expect "63 late row(s) over all of the day's indices files ... 19800 s ... IST's offset"
#   (or "over the day's indices files its run read (...)" where that close carried a filter)
GET '/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY-2019-11-28-1200000-CE&from=2019-11-14&to=2019-11-14'
#   expect import.outcome "recounted" with import.late from late_line (the close before it)
# Goal 3: a definition-2 option day is labelled.
GET '/dayflags.json?feed=gdfl&instrument=NSE-FNO-NIFTY-2019-03-28-1150000-CE&from=2019-03-01&to=2019-03-28'
#   expect bar_definition.value 2, current false, the phrase "...labelled until it is restated";
#   /db Details on such a row shows "Bar definition".
# Goal 4: /db, Zerodha, NSE-CASH-ASHOKLEY 1min 2019-06 and NSE-CASH-BAJFINANCE 1min 2019-06:
#   each row's Source line carries "adjusted run <first> → <last>" (BAJFINANCE "quantised",
#   the 50-paisa grid), read from each day's `basis` in:
GET '/sources/day.json?exchange=NSE&segment=CASH&symbol=BAJFINANCE&from=2019-06-01&to=2019-06-30'
# CMP-06: a share's 2019-05-30 names the market's sampling and its first second.
GET '/dayflags.json?feed=gdfl&instrument=NSE-CASH-RELIANCE&from=2019-05-30&to=2019-05-30'
#   expect sampled.of "market", sampled.first_held the share's own first second that day
# DB-A15: a day the calendar calls closed on which GDFL holds bars says so (look for one with
#   GET /calendar.json; none may exist on the store, which is the finding's own result).
# Goal 5a: once R3-26 lands a new day-ledger field, it appears under quality.<kind>.<name>
#   and in quality.<kind>.unknown, and /db's opened day lists it.
```

### 4. Docs rows to land (R3-13 owns no file under `docs/`)

`CLAUDE.md` §9 asks for every new invariant in `docs/04-invariants.md` beside its test and a
`docs/05-decisions.md` entry for every locked choice, and the spec asks for a limits row. None
of those files is R3-13's, so the rows are written here, word for word, for the Mac to land
with the merge. Decision numbers are taken from the ledger's head at landing (amendment A6).

#### `docs/05-decisions.md` (one entry, numbered at landing)

**D-NNNN — R3-13: the /db day band counts the grid's rung, reads a share's dated
closing-auction flag, and judges frozen index values, late rows on a recounted day and the bar
definition of a day at read time — 2026-10-17**

1. `/dayflags.json` takes `timeframe=` (a rung `store::path::Timeframe::KNOWN` names) and
   counts `held` at it; the feed's finest rung is counted beside it as `finest` where the two
   differ. Absent, the finest is counted, so every existing caller is unchanged. `Asked` gains
   no field (`sources.rs` builds it by name); the rung travels as `dayflags_rules::Band`.
2. `expiry=` narrows an option family key (`NSE-FNO-NIFTY`) to one expiry; a contract's own
   key names its expiry and a disagreeing `expiry=` is refused, as is one on a spot key. A day
   after the expiry is `expired`: `owed_minutes` and `unverified_minutes` 0, as `/gaps.json`.
3. A share's phases are built with its dated master flag (`pull::cash_session_cache`, read as
   `bars_change::Rules::eligibility` reads it): flag 1 owes 360 continuous minutes, flag 0 375,
   no master 360 and 15 unverified, with the reader's reason in `calendar.cas.why` and the
   notes. A failed load is kept by the request only (AF-03's rule). Nothing is read for a closed
   day; a session after today (IST) is `basis: "not_yet"`, never a refusal.
4. "Kept apart from trades" is said of a closing-auction print only where the GDFL share month
   holds its `.auc` sidecar (one `stat` a month); an index record says the prints are "counted by
   the import", and claims nothing of a sidecar. The page calls the session's last print the
   auction's only where it equals the official day close (0 bps); it never claims the print's
   book.
5. FROZEN: an index value unchanged for **180 s or more** in session, outside any recorded halt.
   Chosen, not sourced (DB-B20's detector). Zerodha: from the day's one-minute bars
   (`O = H = L = C`, equal minute to minute, no minute missing), scanned once per (month file,
   inode, day) and kept with the end row it was read to; a page's first scans run on one
   scoped thread a month, each on the month's own handle (a sealed month verifies reads through
   one cached block a handle, so threads sharing one would evict each other's block), and a
   cold page waits on its longest month, not on every day. GDFL: from the day ledger's
   `flat_max_s`, the import's measure, counting only the run's seconds outside the recorded
   halt (`outside_halt_s`; 2020-03-13's run from 09:21:09 is the halt's), and a run with no stamp
   on a halt day is not flagged. OUTSIDE THE DAY RANGE: a minute's high above the Zerodha
   `1day` bar's high, or its low below its low; not judged on GDFL at read time (its seconds
   are up to 22,500 reads a day). Flags only; the bars are served unchanged.
6. A `recounted` day keeps the `late=` and `max_back_s=` of the last `done`, `incomplete` or
   `refused` close of its kind, day and filter, named by `late_line`. The count is said to be
   over all of the kind-day's files for a `*` run, and over the files its run read, named by
   its filter, otherwise (`filter`, `late_scope`).
7. `bar_definition`: the import journal's deciding line, else the day ledger's line; current by
   `pull::gdfl_import::definition_is_current`. Said on every option day and on any day not
   current.
8. A day-ledger field this reader has no name for is served under its own name on the line's
   object and listed in `unknown`; the names `scope`, `line` and `unknown` are the object's own
   and refuse the line, as does a field written twice. (Replaces "kept out of the record".)
9. A share's peer count is looked up under its other names from `brutex_core::alias::LINEAGE`
   where its own name holds no month (`peer.as`).
10. A GDFL share's `sampled` names the market index `NIFTY` (`of: "market"`) and the share's own
    first held second (CMP-06).
11. The previous held `1day` bar is looked for up to 12 opens back and then up to 144 more months
    by `stat`, one open (DB-031); a history found empty there is not probed again by
    `series_start`.
12. The band's peer feed and each feed's finest rung come from one function,
    `dayflags_rules::feed_rungs` (R3-34 points it at the feed table), and every caller of the band
    parses its query with one function, `dayflags::query_of`.
13. On the page, a share is trade-driven, and its zero-volume minutes are counted, by the feed's
    finest rung, whatever rung the grid counts; the closing-auction line counts "kept apart" only
    on days whose dated flag is 1 (`casCounts`). Goal 4 reads `/sources/day.json`'s `basis`; a day
    whose `basis.factor` is not a number is unmeasured and neither joins nor breaks a run.

#### `docs/04-invariants.md` (rows, numbered at landing)

| Invariant | Test |
|---|---|
| The day band counts the rung the grid shows; a Zerodha 1day session holding minutes and no day bar is the day bar missing, and a held day bar is never "no Zerodha bar" | `api::server::dayflags::rules::tests::the_band_counts_the_asked_timeframe` |
| A share's 15:15-15:29 follows its dated closing-auction flag: 360 owed on flag 1, 375 on flag 0, unverified where no master holds it | `api::server::dayflags::rules::tests::a_shares_phases_follow_its_dated_closing_auction_flag` |
| No print is said kept apart from trades without the month's auction sidecar | `api::server::dayflags::rules::tests::an_auction_print_is_said_kept_apart_only_where_a_sidecar_is` |
| A day after a contract's (or a narrowed family's) expiry owes nothing | `api::server::dayflags::rules::tests::a_day_after_expiry_is_expired_and_owes_nothing`, `…::an_option_family_narrowed_to_an_expiry_is_expired_after_it` |
| A scale switch is found however far back the previous held day bar lies (within 156 months) | `api::server::dayflags::rules::tests::a_scale_switch_is_found_across_far_months` |
| A frozen index run is flagged on both feeds, counting only its seconds outside a recorded halt | `api::server::dayflags::rules::tests::a_frozen_index_run_is_flagged_on_both_feeds`, `…::a_ledger_run_inside_the_halt_is_not_frozen` |
| A recounted day keeps its late rows, named by the close they come from | `api::server::dayflags::rules::tests::a_recounted_day_keeps_its_late_rows` |
| An option day names the bar definition that built it | `api::server::dayflags::rules::tests::an_option_day_names_its_bar_definition` |
| A day-ledger field the reader has never seen is served and drawn | `api::server::dayflags::rules::tests::a_day_field_the_reader_has_never_seen_is_served_and_drawn`, web `R3-13: a day field the reader has never seen is served and drawn` |
| A zero-volume share day is checked against its peer under its other names | `api::server::dayflags::rules::tests::a_zero_volume_peer_is_found_under_the_shares_other_name` |
| The scan memo holds one entry per (file, day); a grown day replaces it | `api::server::dayflags::rules::tests::a_days_scan_is_kept_once_and_a_grown_day_replaces_it` |
| The day band's warm answer does not grow with history no page reads (100x within 3x of 1x) | `api::server::dayflags::rules::tests::the_day_band_is_timed_cold_warm_concurrent_and_growing` (ignored; run with `--ignored`) |
| The session's last closing-auction print is called the auction's only where it equals the official close, and no book is claimed | web `SC-CLOSE-10: a last print that does not equal the official close is never called the auction's` |
| A closed day reads no closing-auction flag, and a session after today is `not_yet`, never a refusal | `api::server::dayflags::rules::tests::a_flag_is_read_only_on_an_open_session_up_to_today` |
| A filtered run's late rows are said over the files it read, never all of the kind-day's | `api::server::dayflags::rules::tests::a_filtered_runs_late_rows_are_over_the_files_it_read` |
| A share is trade-driven, and its fills counted, by the finest rung; a day whose flag no master holds is never counted kept apart | web `R3-13 review: a share on a coarser grid is trade-driven by its finest rung…`, `R3-13 review: a closing-auction day whose flag no master holds is never counted as kept apart` |
| An adjusted or quantised run is read from the served `basis`, and an unmeasured day neither joins nor breaks it | web `R3-13 goal 4: a share run whose prices are adjusted or quantised is named with its first and last day` |
| A page's first scans run once each, a month a thread; a page the memo holds spawns no thread; a day with no bar is never scanned | `api::server::dayflags::rules::tests::a_pages_first_scans_run_a_month_a_thread_and_are_kept` |
| The previous close is the last bar of the nearest held month, and an empty month file is a held month for the series' start | `api::server::dayflags::rules::tests::a_scale_switch_is_found_against_the_month_just_before`, `…::an_empty_month_before_the_page_is_not_an_empty_history` |

#### `docs/06-limits.md` (one row, numbered at landing)

| L-R3-13 | `/dayflags.json` with R3-13's read-time rules | per page of at most 92 days: (a) a Zerodha index page scans each day's one-minute bars once per process — at most 1,440 record reads a day, 92 × 1,440 a page — and keeps the scan per (device, inode, first row) with its end row, one probe a day after; a page's first scans run on one scoped thread a month (at most four for 92 days), each on the handle the count already opened, none where the memo holds every day; the memo takes (device, inode) for the month file's identity, as the base's ledger tail readers do, so on a filesystem that reuses an inode number at once (this cloud container's does: `store`'s own `staging` test measures it) a month replaced by a new generation could be served its old scan for a day whose end row is unchanged, until the process restarts; the memo holds one entry per (month file, day) shown, however often a month grows; (b) the previous `1day` bar is looked for 12 opens back, then up to 144 `stat`s and one open, once a page; a history found empty is not probed back again by `series_start`; (c) a share's closing-auction flag reads that day's NSE master once per process from 2026-08-03 (a refusal once per request), and one `stat` a month for its `.auc` sidecar; (d) the late-row ledger is a tail read of the import journal: the whole journal once per process, then the bytes appended; (e) a GDFL share page counts the market index (NIFTY) beside it, one open a month and two lookups a day, and reads one record a day for its first second; (f) a renamed share's peer is counted under each other name only where its own name holds no month | **Measured** (synthetic store, 4-core cloud machine, three runs each, alternately, `the_day_band_is_timed_cold_warm_concurrent_and_growing`; medians of the three, branch against base): warm 1 client, n = 1,000, p50 4.59 / p99 9.71 / max 13.09 ms (6.76 / 15.45 / 20.91); with an import writing 5.05 / 11.22 / 13.85 (7.83 / 18.36 / 24.89); 60 clients, n = 1,200, 165.4 / 1,064.0 / 1,156.6 (267.4 / 1,161.9 / 1,522.8); cold, memo empty, page cache warm, Zerodha NIFTY, n = 20, p50 4.39 / max 8.45 (5.10 / 11.04); warm p50 at 10x and 100x history 0.93x and 0.98x of 1x. A cold device is NOT MEASURED; the real store's figure is the Mac's. The answer is built by moving values into its maps (`json!` deep-copied every nested value: 43% of the base's instructions, callgrind), and a page's first scans run a month a thread | R3-13 |

### 5. For other owners (files R3-13 does not own)

- `crates/api/src/imports.rs` `day_band_json` (the `logs::dayflags_json_over` bench, L-IA-39)
  parses `Asked` alone, so it cannot ask for `timeframe=` or `expiry=`. The one-line change is
  `let (asked, band) = super::dayflags::query_of(query)?; Ok(super::dayflags::answer_at(root,
  &asked, band).to_string())`. Until it lands the bench measures the finest rung only, which is
  what it measured before.
- `crates/pull/src/calendar.rs`: DB-A15's stale comment. It says 1,666 Zerodha minute days in
  2019-12-02..2026-08-21; the finding measured 1,631 on the store, the difference being the 35
  hole days. The count is the store's, so its owner re-measures it on the Mac by the finding's
  own method (its assurance report, `state/assurance/`). R3-13 serves the independent evidence the
  finding asks for on the record instead: a closed day the other feed traded is `peer_on_closed`.
