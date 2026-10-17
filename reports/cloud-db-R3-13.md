STATUS: IN PROGRESS

# R3-13 — /db day band and row labels (cloud build report)

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
| Goal 2c, DB-B20 (frozen index, value outside the day range) | Zerodha: each index day's minutes scanned once (≤1,440 reads) and kept per (device, inode, first row) with its end row; runs of ≥180 s unchanged outside any recorded halt, and the minute extremes against the day bar. GDFL: the day ledger's `flat_max_s`. Band kinds `frozen`, `outside_range` | `a_frozen_index_run_is_flagged_on_both_feeds`, `a_frozen_run_is_three_unchanged_minutes_in_a_row`, `a_ledger_run_inside_the_halt_is_not_frozen`, `a_days_scan_is_kept_once_and_a_grown_day_replaces_it`, web `DB-B20, DB-A15, goal 3…` |
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

### A cost found and removed on the way

The first measurement had the branch SLOWER than the base (warm p50 8.3 ms against 6.4 ms).
Callgrind (`the_day_band_pages_for_a_profiler`) showed 43% of `/dayflags.json`'s instructions in
`serde_json` copying values: `json!` serializes every `Value` it is handed into a new one, so
each record's nested objects, and then all 92 records, were deep-copied — the base pays this
too. The record, the day's events and the answer's envelope are now built by moving values in
(`dayflags::object`), and the branch is faster than the base on every page (below).

## Measured target (synthetic store, this cloud machine)

`the_day_band_is_timed_cold_warm_concurrent_and_growing` (`#[ignore]`; run with `--ignored`):
92-day pages of NIFTY on both feeds, RELIANCE on both, one NIFTY contract; 375 bars a session at
each finest rung; n = 200 a page. Run on the base (the same harness pasted into a worktree of
`origin/feat/db-fix` 62396df0) and on the branch, back to back, 4 cores, idle. **Not a gate: one
machine, one run each; the real store's figure is NEEDS THE MAC below.**

| measure (p50 / p99 / max) | base | branch |
|---|---|---|
| warm, 1 client, all 5 pages, n = 1,000 | 6.37 / 16.30 / 21.73 ms | **4.35 / 8.67 / 10.86 ms** |
| warm, 1 client, import writing | 6.38 / 18.12 / 19.86 ms | **4.48 / 8.90 / 13.37 ms** |
| 60 clients at once, import writing, n = 1,200 | 214.4 / 958.5 / 1,150.5 ms | **66.7 / 725.0 / 861.4 ms** |
| cold (a fresh copy of the store: every memo empty, page cache warm), Zerodha NIFTY page, n = 20 | 4.40 / 8.99 ms | **6.51 / 9.29 ms** — slower |
| GDFL NIFTY, warm | 9.47 / 17.94 ms | 5.39 / 10.77 ms |
| Zerodha NIFTY, warm | 4.01 / 8.44 ms | 2.79 / 5.40 ms |
| Zerodha RELIANCE, warm | 6.57 / 11.58 ms | 4.63 / 5.84 ms |
| GDFL RELIANCE, warm | 6.40 / 7.16 ms | 5.12 / 7.41 ms |
| GDFL NIFTY contract, warm | 6.92 / 16.95 ms | 4.69 / 9.06 ms |
| growth, warm p50 at 10x / 100x history no page reads, over 1x | 1.02 / 1.01 | 0.98 / 1.01 (gate: ≤ 3.00) |

**The one figure that is slower is the cold Zerodha index page** (+2.1 ms at p50): DB-B20's scan
reads each index day's minutes once per process (63 sessions × 375 reads here) before its memo
holds them. Every later answer is faster than the base. A cold DEVICE (page cache dropped) is
NOT MEASURED.

## Gate results (this cloud machine)

- `cargo fmt --all --check`: clean.
- `cargo clippy --workspace --all-targets --locked -- -D warnings`: clean.
- `cargo test --workspace --locked`: RUNNING (skipping only
  `the_pool_verb_prints_its_whole_page_on_a_generated_store`, the known slow one `CONTEXT.md`
  names). `api`'s day-flag tests: 70 passed, 3 ignored (the real-store test, the timing, the
  profiler target).
- `cargo deny check`: **advisories FAILED, bans ok, licenses ok, sources ok** — RUSTSEC-2026-0344,
  -0346, -0347, -0348 and -0349 (`ruzstd` 0.8.2 under `lake` and `pull`; `rustls`). The same five
  fail on the base `origin/feat/db-fix`; `Cargo.lock` is not an R3-13 file and is unchanged here.
- Web: `node --test "web/tests/*.test.js"` 1,407 tests, 0 failed; 5 timer tests in
  `ask.test.js` and `census-worker.test.js` are cancelled ("Promise resolution is still pending")
  exactly as on the base, and `store-watch.test.js`'s 4 are cancelled in some full runs and pass
  alone on both trees. `npm run check` (svelte-check): 6 errors, the same 6 as on the base, none
  in an R3-13 file (`ingest-months.js`, `ingest/+page.svelte`, `settings-ceilings.test.js`).
- Coverage and mutants: NOT RUN YET.

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
   inode, day) and kept with the end row it was read to. GDFL: from the day ledger's
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

#### `docs/06-limits.md` (one row, numbered at landing)

| L-R3-13 | `/dayflags.json` with R3-13's read-time rules | per page of at most 92 days: (a) a Zerodha index page scans each day's one-minute bars once per process — at most 1,440 record reads a day, 92 × 1,440 a page — and keeps the scan per (device, inode, first row) with its end row, one probe a day after; the memo holds one entry per (month file, day) shown, however often a month grows; (b) the previous `1day` bar is looked for 12 opens back, then up to 144 `stat`s and one open, once a page; a history found empty is not probed back again by `series_start`; (c) a share's closing-auction flag reads that day's NSE master once per process from 2026-08-03 (a refusal once per request), and one `stat` a month for its `.auc` sidecar; (d) the late-row ledger is a tail read of the import journal: the whole journal once per process, then the bytes appended; (e) a GDFL share page counts the market index (NIFTY) beside it, one open a month and two lookups a day, and reads one record a day for its first second; (f) a renamed share's peer is counted under each other name only where its own name holds no month | **Measured** (synthetic store, 4-core cloud machine, one run each, `the_day_band_is_timed_cold_warm_concurrent_and_growing`, n = 200 a page over 5 pages): warm 1 client p50 4.35 / p99 8.67 / max 10.86 ms (the base 6.37 / 16.30 / 21.73); with an import writing 4.48 / 8.90 / 13.37 ms (base 6.38 / 18.12 / 19.86); 60 clients 66.7 / 725.0 / 861.4 ms (base 214.4 / 958.5 / 1,150.5); cold, memo empty, page cache warm, Zerodha NIFTY page, n = 20: 6.51 / 9.29 ms (base 4.40 / 8.99) — the one figure slower than the base, DB-B20's first scan; warm p50 at 10x and 100x history 0.98x and 1.01x of 1x. A cold device is NOT MEASURED; the real store's figure is the Mac's. The answer is built by moving values into its maps: `json!` deep-copied every nested value, 43% of the base's instructions (callgrind) | R3-13 |

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
