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

To be completed with exact commands at the end.
