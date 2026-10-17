STATUS: IN PROGRESS

# R3-13 — /db day band and row labels (cloud build report)

Branch `cloud/db-R3-13-20261017`, from `origin/feat/db-fix` 3911d1b9c, with `origin/feat/db-fix`
62396df0 merged in (AF-02 and AF-07 changed `dayflags.rs` after the branch was cut).

## Prerequisites

Both merged into `origin/feat/db-fix`:

- `Merge WP-11c: import data-quality labels: minute-sampled days and halts` (c4f6e522)
- `Merge WP-62: pull runtime: calendar horizon, idle stop, per-instrument gates, stop/restart/storage safety` (72c91a46)

## Done

Every change is in a file R3-13 owns. The new module is `crates/api/src/dayflags_rules.rs`
(declared from `dayflags.rs` by `#[path]`, so `server.rs` is untouched), its tests are
`crates/api/src/dayflags_rules_tests.rs`.

| Finding / goal | What changed | Test (fails before, passes after) |
|---|---|---|
| Goal 1 (band at the grid's rung), DB-053 | `/dayflags.json?timeframe=` counts `held` at the grid's rung and the feed's finest beside it (`finest`); a Zerodha `1day` session with minutes and no day bar says the day bar is missing; a held day bar is never "no Zerodha bar". The band sends `db.timeframe`. | `the_band_counts_the_asked_timeframe` (Rust: asserts the old reading on the same store first), `R3-13: the band counts the asked timeframe…` (web) |
| Goal 1 (one contract) | `bandInstrument` moved to `dayflags.js` (testable), counts only rows at the grid's rung; one contract asks about that contract; one expiry at any strike asks the family with `expiry=` | `R3-13: one contract is asked about itself…` (web), `an_option_family_narrowed_to_an_expiry_is_expired_after_it` |
| Goal 2a, DB-031 (scale switch, far months) | `bar_before` falls back to `rules::held_month_before`: up to 144 `stat`s back, one open | `a_scale_switch_is_found_across_far_months`, `the_month_before_is_found_by_stats_within_its_bound` |
| Goal 2b, DB-031 (zero-volume peer, other names) | a share's peer is counted under its other names from `brutex_core::alias::LINEAGE` where its own name holds no month; `peer.as` and the phrase name it | `a_zero_volume_peer_is_found_under_the_shares_other_name` |
| Goal 2c, DB-B20 (frozen index, value outside the day range) | Zerodha: each index day's minutes scanned once (≤1,440 reads, memoised per closed day), runs of ≥180 s unchanged outside any recorded halt, and the minute extremes against the day bar; GDFL: the day ledger's `flat_max_s` | `a_frozen_index_run_is_flagged_on_both_feeds`, `a_frozen_run_is_three_unchanged_minutes_in_a_row`, `a_ledger_run_inside_the_halt_is_not_frozen`, web `DB-B20, DB-A15, goal 3…` |
| Goal 2d, DB-029 (late rows on a recounted day) | `LateLedger`, a tail read of the import journal's closing lines: a `recounted` day keeps the close-before-recount's `late=`/`max_back_s=`, named by `late_line`; the phrase says the count is over all of the day's files | `a_recounted_day_keeps_its_late_rows`, `the_late_ledger_keeps_the_last_counting_close` |
| Goal 3 (bar definition 2/3/4) | `bar_definition` on every GDFL record (journal deciding line, else the day ledger), current or not by `pull::gdfl_import::definition_is_current`; phrase on every option day and on any stale day; band kind `old_definition`; rows' Source line and the drawer name it | `an_option_day_names_its_bar_definition`, `a_spot_day_names_its_definition_only_when_it_is_not_current`, web `R3-13 goal 3…` |
| Goal 4, DB-G38, DB-G40 (adjusted / quantised Zerodha share runs) | `DbProvenance` asks `/sources/day.json` over the page's share days (≤31) and labels each row's Source line and Details with the run its day belongs to (first and last day, factor, grid, whether the run may go on past the days read); `DbBasisBadge` says where | web `R3-13 goal 4…` |
| Goal 5a (unknown quality field) | `parse_quality` keeps a field it has no name for, served under its own name and listed in `unknown`; the band draws them | `a_day_field_the_reader_has_never_seen_is_served_and_drawn` (Rust and web) |
| Goal 5b (one function for peers and rungs) | `rules::feed_rungs`; `peer_of` and `finest` delegate to it | `the_band_rungs_come_from_one_function` |
| SC-CLOSE-8 | phases built with the share's dated master flag (`rules::CasFlags`, read as `bars_change.rs` reads it, failed loads never kept); `calendar.cas` | `a_shares_phases_follow_its_dated_closing_auction_flag`, `no_flag_is_read_where_none_applies`, `a_flag_refused_by_one_request_is_read_by_the_next` |
| SC-CLOSE-9 | band `cas` only where `calendar.cas.eligible` is true; "kept apart" only where the month's `.auc` sidecar exists (`auction_sidecar`), in the server phrase and the band's line | `an_auction_print_is_said_kept_apart_only_where_a_sidecar_is`, web `SC-CLOSE-9…` and the updated `cas` assertions |
| SC-CLOSE-10 | the last print is called the auction's only where it equals the official close (0 bps); no zero-book claim; two prints at that price say which is NOT PROVED | web `SC-CLOSE-10: a last print that does not equal the official close…` |
| SC-CLOSE-11 | already fixed on the base by AF-02 (`Names` in `journal_rows`; tests in `crates/api/src/imports_filter_tests.rs`). Nothing added. | AF-02's |
| SC-CLOSE-12 | a day after a contract's (or a narrowed family's) expiry is `expired`: owed 0, one phrase, band kind `expired` (quiet), never a hole or "not imported yet" | `a_day_after_expiry_is_expired_and_owes_nothing`, web `SC-CLOSE-12…` |
| CMP-06 | a GDFL share's day names the market index's sampling (`of: "market"`, NIFTY) and its own first held second | `a_shares_day_names_the_markets_sampling_and_its_first_second` (and the updated share assertion in WP-11c's sampled test) |
| DB-A15 | a closed day the peer feed traded is said as evidence independent of the series; band kind `peer_on_closed`. The stale comment in `crates/pull/src/calendar.rs` is NOT an owned file and is left for its owner | `a_closed_day_the_peer_traded_is_said_as_independent_evidence` |

Existing tests changed because the rule they pinned changed on purpose: the late-row phrase
(now "over all of the day's indices files"), WP-11c's "a share names no sampled day" (CMP-06),
the web weekend fixture whose peer "held 375 bars" on a Saturday (now 0; a closed day the peer
traded is the new `peer_on_closed`), the web `cas`-on-every-`X`-day assertion (SC-CLOSE-9), and
the web auction text "as it does on this day" (SC-CLOSE-10).

## Left

- Full gate run: `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --locked`, `cargo deny check`, coverage and mutants on the touched
  modules.
- The `/dayflags.json` timing (p50/p99/max, n=200, cold and warm, 1 and 60 clients) and the
  growth ratio at 1x/10x/100x on a synthetic store.
- The web suite as a whole and `svelte-check` on the touched components.

## Gate results

Not run as a whole yet. `node --test web/tests/db-dayflags.test.js` 20/20 and
`web/tests/db-labels.test.js` 38/38 pass. The `api` day-flag tests passed (57) before the last
batch of server changes; the last batch is compiling.

## NEEDS THE MAC

To be completed with exact commands at the end.
