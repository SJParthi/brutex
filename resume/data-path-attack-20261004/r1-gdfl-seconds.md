# gdfl-seconds (finished) + GDFL monthly-name table, round 1

Worktree `/home/claude/gdfl-wt`, branch `claude/attack-gdfl`. Commits (local, not pushed): **bb197f5** (TASK 1, D-3165) and **566a5a8** (TASK 2, D-3176..D-3178), both on top of the WIP commit e1f70e0.
S = `crates/pull/src/gdfl_seconds_attack_tests.rs`, I = `crates/pull/src/gdfl_import.rs`, IT = `crates/pull/src/gdfl_import_tests.rs`, N = `crates/pull/src/gdfl_nfo.rs`, NA = `crates/pull/src/gdfl_nfo_attack_tests.rs`, NT = `crates/pull/src/gdfl_nfo_tests.rs`, F = `crates/pull/src/gdfl_fixtures.rs`.

"Failures on old code" was measured: I copied S and F into a worktree at 0389b3c (the commit before the WIP). The old code has no `Report::max_back_s` and no `ImportRefusal::FilterName`, so I made those three lines fail by name, ran `cargo test -p pull --lib attack_gdfl_seconds`, and got **13 passed, 8 failed**.

## 1. Attacks

| attack | cases tried | failures on old code (0389b3c) | fixed? | evidence |
|---|---|---|---|---|
| Untraded row's stamp as evidence of time: a trade written after a row stamped later was filed at its own earlier stamp (look-ahead) | 1 hand case + the 100k reference sweep | **failed**: 2 bars where 1 was expected | yes, D-3170 | `an_untraded_rows_stamp_is_evidence_of_time_and_no_trade_lands_before_it` S:357; fix `place` I:298 |
| `convert` vs a slow, independent reference, 100,000 random files of every kind | 100,000 | **failed** at case 8 (placement differed from the reference) | yes, D-3170 | `the_one_second_build_matches_a_naive_reference_over_100k_random_files` S:279 |
| Mixed second: every order of 3 traded rows among untraded ones | 3! x 4 positions | 0 | n/a | `a_mixed_second_uses_only_its_traded_rows_in_every_order` S:307 |
| Extremes: first/last second of the day, largest price, one row, duplicates, every row late | hand-picked set | 0 | n/a | `extreme_stamps_prices_and_duplicates_build_exactly` S:402 |
| Session edges 00:00:00, pre-open, 09:14:59, 09:15:00, 15:29:59, 15:30:00, 15:30:01, 23:59:59 through the whole runtime; counts balance | 8 seconds | 0 | n/a | `session_edges_are_filed_or_counted_and_the_counts_balance` S:445 |
| Forward spike: one 14:00:00 row among 10:00 rows collapses hours into one second with no recorded size | 1 | **failed** (the report had no back-step) | yes, D-3172 (`max_back_s` in Report and journal, Warn per file) | `a_forward_spike_collapses_hours_into_one_second_and_says_how_far` S:498; I:269, I:447 |
| Filter name that is not a filed symbol (`*`, `NIFTY BANK`, lower case, comma, empty) keys the journal as another filter | hostile names + 4 good ones | **failed** (`*` accepted, day journalled) | yes, D-3171 | `a_filter_name_that_is_not_a_filed_symbol_is_refused_before_the_journal` S:543; I:476 |
| Torn journal line (crash mid-write), then 3 later runs | several torn prefixes x 3 runs | **failed**: run 1 refused the journal as foreign, "not a journal line" | yes, D-3173 (closed with ` (torn)`, skipped on load) | `a_torn_journal_line_stays_ignored_on_every_later_run` S:595; I:534, I:570 |
| Capital-market day with one ticker twice (`.csv`/`.CSV`), climbing and subfolder names: file + skip + refusal must equal entries | 6 entries | **failed**: 5 counted out of 6 | yes, D-3174 | `every_capital_market_entry_is_a_file_a_skip_or_a_refusal` S:621; I:961 |
| Reruns and mended partial days: rerun is a no-op; two runs leave byte-identical stores; mended day equals clean | 3 stores | 0 | n/a | `reruns_and_mended_partial_days_converge_on_the_clean_store` S:669 |
| Hostile rows (NaN, sign, exponent, blank, 3rd decimal, sub-tick) and LTP edges | hand set | 0 | n/a | `hostile_rows_refuse_by_name_and_edge_prices_decode_exactly` S:744 |
| Random mutations of a valid file (flip, drop, insert, cut) | 100,000 | 0 | n/a | `a_mutated_file_never_panics_the_reader_and_what_decodes_is_sound` S:860 |
| Tick-store day file cut at every length | every length | 0 | n/a | `a_tick_store_day_cut_anywhere_refuses_by_name` S:949 |
| Random byte flips in a tick-store day file | 100,000 | 0 | n/a | `a_flipped_tick_store_byte_never_reaches_a_row_unnoticed` S:976 |
| Tick-store offsets/lengths at the u64 edge, counts past the index, bomb under a small stated size | hand set | 0 | n/a | `tick_store_offsets_at_the_edge_and_bombs_refuse_without_wrapping` S:1061 |
| Member names that climb (`../`), are absolute, use `\`, sit in a subfolder, or have NUL after the extension | hand set | 0 | n/a | `member_names_that_climb_or_hide_are_never_the_tickers_file` S:1162 |
| Deflate bomb: 16 MiB inflating under a stated 100 bytes | 1 | 0 (read to 101 bytes, then refused) | n/a | `a_deflate_bomb_is_inflated_one_byte_past_its_stated_length_and_no_further` S:1225 |
| Random byte flips in an outer archive | 100,000 | 0 | n/a | `a_flipped_archive_byte_never_reaches_a_row_unnoticed` S:1243 |
| Undecodable options name wanted by prefix: a `NIFTY` run counted `NIFTYNXT50…` refusals (open item iv) | 4 names x 3 filters | **failed**: (refused, skipped) = (2,2), expected (1,3) | yes, D-3175 (longest F&O underlying prefix) | `an_undecodable_option_name_is_wanted_only_by_its_whole_underlying` S:1386; I:1020 |
| Contract renamed at the 2019-02-01 cutover keyed by ticker text, not by the decoded contract (open item i) | 1 measured-day pair + the ACC cutover pair + both cross-era readings + a same-day NIFTY pair | **failed**: `ACC19FEB1260PE` did not decode (MonthlyExpiryUnstated) | yes, D-3176 with D-3165 | `a_renamed_contract_is_keyed_by_its_decoded_contract_never_its_name` S:1465 |
| Two tickers naming one contract on one day (open item ii) | 4 pre-cutover names at `nfo_day` + 3 names on a measured day | WIP version **failed** (it expected MonthlyExpiryUnstated) | yes: now asserts TickerAmbiguous x2, FormsAmbiguous, TickerUnparsed (exact text) | `two_tickers_naming_one_contract_are_both_refused` IT:915 |
| Flaky `NotFound` in `tree` (open item iii) | 20 runs of the rename test alone + 20 runs of the full 226-test gdfl suite in parallel (4,520 test runs) | not reproduced, before or after | hardened, D-3177 | F:196 (`scratch`), S:218 (`tree` names the path) |
| Per-tick build and per-second lookup cost | 4 sizes each | n/a (measurement) | n/a | `per_tick_build_and_per_second_lookup_cost` S:1308 (numbers in §4) |
| **TASK 1:** monthly-form names (`ADANIENT18DEC195CE`) were all refused MonthlyExpiryUnstated; sourced table wired in | 15 per-month tests + sweeps: 790,140 monthly names (dpn_02), 7,918,488 dated names (dpn_04), 10^6 fuzz (dpn_09), absent-month sweep 2015–2026 | before: every monthly name refused (none decoded) | yes, D-3165 | N:156 `MONTHLY_EXPIRIES`, N:177, N:396, N:413, N:485; NA:1473 (dpn_20), NA:1535–1675 (dpn_21 x15), NA:1687 (dpn_22), NA:1733 (dpn_23), NA:224 (dpn_02), NA:499 (dpn_04), NA:843 (dpn_09), NA:1443 (dpn_19), NT:73 |

Cause of the item (iii) flake: **not proven.** I could not reproduce it. Every `scratch` tag is unique and the name already held pid and nanoseconds. The cause I could find is `create_dir_all`. If two calls ever produced the same name, it would accept the existing directory without complaint, and one test's `remove_dir_all` would delete the other's tree while `tree` was walking it. That gives exactly a `NotFound` at `read_dir`.

The hardening is two changes. `scratch` now adds a per-process `AtomicU64` sequence number and creates the leaf with `create_dir`, so a collision panics by name and a tree can never be shared. `tree` now prints the path it failed on, so the next occurrence names itself.

The names attacker's report also says the two attackers ran tests concurrently in one worktree. That is a plausible trigger I cannot confirm.

TASK 1 detail:
- **What changed:** before 2019-02-01, a monthly reading whose month is in the table now decodes to the table's day.
  - If the trade day is after that expiry (or the expiry is past the 2,200-day horizon), the name is `ExpiryRefused`.
  - A month not in the table stays `MonthlyExpiryUnstated`.
- **FormsAmbiguous was re-checked and deliberately not narrowed by the table.** For NIFTY and BANKNIFTY, a name with a live dated reading is still refused whenever its monthly month overlaps the window. That holds even when the table says the month's contract has already expired (e.g. `NIFTY18DEC2311000CE` on 2018-12-28 stays FormsAmbiguous). The reason: a monthly contract traded after its expiry is a vendor fault to refuse, never a reason to take the other reading. The census says this case never occurs, so the strict rule costs nothing on real data.
- **dpn_20 checks every table row against NSE's rule:** each is a Thursday and the last Thursday of its month. All 15 hold.
- **Same-day pair:** `NIFTY19JAN10500CE` and `NIFTY31JAN1910500CE` on 2019-01-15 now decode to one contract, `2019-01-31-1050000-CE`. A day holding both refuses both as `TickerAmbiguous` (pinned in S and IT).

Synthetic sweep counts after the change (misread = 0 in every row):

| sweep | names | misread | detail |
|---|---|---|---|
| dpn_02 monthly names, trade months 2018-01..2019-01 | 790,140 | **0** (and 0 "missed") | 297,062 decoded to the sourced day; 391,860 MonthlyExpiryUnstated; 37,068 ExpiryRefused; 1,750 FormsAmbiguous; 62,400 TickerUnparsed/UnderlyingRefused |
| dpn_04 dated names 2018-09-03..2027-12-31 | 7,918,488 | **0** | before the cutover: 16,880 share names read as the monthly form with the sourced day (checked against an independent reference); 337,600 share names refused; 768 index names FormsAmbiguous |
| dpn_09 random names | 1,000,000 | 0 unsound | ok 7,351, of which 2,027 are monthly with the sourced day; refused 992,649, every one by name |

Previously (round 1b): dpn_02 decoded 0 names and had 725,990 MonthlyExpiryUnstated. dpn_04 refused all 354,480 pre-cutover share names.

## 2. Draft decision entries

**D-3165: Pre-cutover GDFL monthly-form option names take their expiry from a sourced table (2026-10-04).**
Before trade day 2019-02-01, a name `UNDERLYING YY MON STRIKE` (`ADANIENT18DEC195CE`) states no expiry day, and every such name was refused `MonthlyExpiryUnstated` (D-2806). `gdfl_nfo::MONTHLY_EXPIRIES` now lists 15 months and their days: 2018-09-27, 2018-10-25, 2018-11-29, 2018-12-27, 2019-01-31, 2019-02-28, 2019-03-28, and the long-dated index months 2019-06-27, 2019-09-26, 2019-12-26, 2020-06-25, 2020-12-31, 2021-06-24, 2021-12-30, 2022-12-29.
There are two sources. The first is the census of the operator's GDFL tree relayed 2026-10-04: 1,218,362 tickers, each month's expiry equal to the vendor's last trade day and to its dated twin's stated date, 0 disagreements, index and stock months sharing dates. The second is NSE's contract specifications page (OPTIDX: "Last Thursday of the expiry month. If the last Thursday is a trading holiday, then the expiry day is the previous trading day.").
A name traded after its month's day is `ExpiryRefused`, and a month not listed stays `MonthlyExpiryUnstated`. The index two-form refusal (D-3160) is unchanged: a live dated reading plus a monthly month overlapping the window is `FormsAmbiguous`, even when the table says that month has expired.

**D-3176: An option's one-second series is keyed by its decoded contract, and the 2019-02-01 rename joins (2026-10-04).**
`nfo_day` hands the filing path `(underlying, Contract)` decoded from the name, never the ticker text, so one contract under two vendor spellings files into one series. With D-3165, `ACC19FEB1260PE` (to 2019-01-31) and `ACC28FEB191260PE` (from 2019-02-01) both decode to ACC `2019-02-28-126000-PE`. Each name read on the other side of the cutover is refused by name.
Two names of one contract can share a second only on one day, and there both are refused `TickerAmbiguous`, never merged. Those days lie before the calendar's first measured day (2019-12-02), so `run_nfo` refuses them `CalendarUnmeasured` today. The join is therefore proven at `nfo_day`, and the store keying on measured days.

**D-3177: GDFL test scratch directories are unique per call and never reused (2026-10-04).**
`gdfl_fixtures::scratch` named a directory by tag, pid and clock and made it with `create_dir_all`, which accepts an existing directory. Two calls landing on one name would share a tree, and one test's cleanup could delete the other's files mid-walk. That is the shape of the once-seen `NotFound` in `tree`, though it was not reproduced in 4,520 runs. The name now carries a per-process sequence number and the leaf is made with `create_dir`, so a collision panics by name. `tree` names the path it failed on.

**D-3178: `gdfl_import` passes gate 11 (2026-10-04).**
At e1f70e0, gate 11 refused `gdfl_import.rs` four ways: two unsized maps (rule 3), a `sort_unstable` of the filter names (rule 4), and two `.contains(&key)` journal probes (rule 7). The listing-scoped maps are now sized from the listing, the filter key is ordered and deduplicated by a `BTreeSet` with byte-identical output, and the probes take `&str`. `gdfl_import_tests.rs` and `gdfl_nfo_tests.rs` now begin with `#![cfg(test)]`, so the gate reads them as test code, as it already did the two attack files. No behaviour changed.

D-3170 to D-3175 are already cited in the WIP code. My drafts, from the code and tests:
- **D-3170: Placement takes the running maximum over every row, untraded ones included.** An untraded row's stamp is evidence of time. A trade written after a row stamped later lands at the next in-order stamp, never its own earlier stamp (`CLAUDE.md` §3 rule 7). The counts still cover kept rows only.
- **D-3171: A filter name must be a symbol exactly as the store files it.** Otherwise it is refused `FilterName` before the journal is touched. `*`, spaces, commas, lower case and empty names would otherwise journal a day `done` under a key another filter shares.
- **D-3172: A deferral states its size.** `Report::max_back_s` carries the run's largest back-step. Each file with late rows logs at Warn, and the journal's `done` line carries `late`, `late_unresolved` and `max_back_s`. No bound refuses a file, because none is measured.
- **D-3173: A torn journal line is closed with ` (torn)`, never a bare newline,** and skipped on every load. Before, one crash mid-write made every later run refuse the journal as foreign.
- **D-3174: A capital-market entry that is a second name for an already offered ticker is counted as skipped,** so files + skips + refusals equal the entries.
- **D-3175: An undecodable options name is wanted by the longest F&O underlying it starts with,** never by any filter name it merely starts with. `NIFTYNXT50…` is not `NIFTY`'s.
- D-3179 is unused.

## 3. Invariant rows

| DPN-08 | Before trade day 2019-02-01, a monthly-form option name decodes only to the day `MONTHLY_EXPIRIES` lists for its month, inside `[trade, trade + 2,200]`. Otherwise it is refused by name: `MonthlyExpiryUnstated` (month not listed), `ExpiryRefused` (expired) or `FormsAmbiguous` (an index name whose dated reading is also live). No monthly name decodes to any other contract. | `gdfl_nfo::attack_tests::dpn_20_the_monthly_table_is_ordered_and_each_day_is_its_months_last_thursday` · `dpn_21_*` (15 tests, one per month) · `dpn_22_a_month_absent_from_the_table_is_still_refused_by_name` · `dpn_23_an_index_name_both_forms_read_is_still_refused_when_monthly_names_decode` · `dpn_02_no_monthly_name_before_the_cutover_decodes_to_another_contract` |
| DPT-01 | No kept row is filed at a second earlier than the stamp of any row (traded or not) written before it in its file. | `gdfl_import::attack_gdfl_seconds::an_untraded_rows_stamp_is_evidence_of_time_and_no_trade_lands_before_it` · `the_one_second_build_matches_a_naive_reference_over_100k_random_files` |
| DPT-02 | `convert` equals an independent reference on every bar and every count over 100,000 random files, or both refuse. | `the_one_second_build_matches_a_naive_reference_over_100k_random_files` |
| DPT-03 | A run's largest back-step is reported (`Report::max_back_s`) and journalled; no deferral is silent. | `a_forward_spike_collapses_hours_into_one_second_and_says_how_far` |
| DPT-04 | A filter name that is not a canonical symbol is refused before the journal is read or written. | `a_filter_name_that_is_not_a_filed_symbol_is_refused_before_the_journal` |
| DPT-05 | A torn journal line never stops a later run, on any number of later runs. | `a_torn_journal_line_stays_ignored_on_every_later_run` |
| DPT-06 | Every listed entry of a day is exactly one of: a file, a skip, a refusal. | `every_capital_market_entry_is_a_file_a_skip_or_a_refusal` · `session_edges_are_filed_or_counted_and_the_counts_balance` |
| DPT-07 | An undecodable options name is attributed to the longest F&O underlying it starts with, never to a shorter filter name. | `an_undecodable_option_name_is_wanted_only_by_its_whole_underlying` |
| DPT-08 | An option's series is keyed by its decoded (underlying, expiry, strike, side). Two names of one contract on one day are both refused `TickerAmbiguous`, never merged. | `a_renamed_contract_is_keyed_by_its_decoded_contract_never_its_name` · `gdfl_import::tests::two_tickers_naming_one_contract_are_both_refused` |
| DPT-09 | A rerun writes nothing; two runs over one input leave byte-identical bar files; a mended partial day converges on the clean store. | `reruns_and_mended_partial_days_converge_on_the_clean_store` |
| DPT-10 | No mutated, cut or byte-flipped file, tick-store day or archive panics the reader; any member that passes the size and CRC check is the original. | `a_mutated_file_never_panics_the_reader_and_what_decodes_is_sound` · `a_tick_store_day_cut_anywhere_refuses_by_name` · `a_flipped_tick_store_byte_never_reaches_a_row_unnoticed` · `a_flipped_archive_byte_never_reaches_a_row_unnoticed` · `tick_store_offsets_at_the_edge_and_bombs_refuse_without_wrapping` · `a_deflate_bomb_is_inflated_one_byte_past_its_stated_length_and_no_further` |
| DPT-11 | No member name outside `<folder>\<TICKER>` (climbing, absolute, subfolder, NUL) is read as a ticker's file. | `member_names_that_climb_or_hide_are_never_the_tickers_file` |

## 4. Cost (measured, release profile, this box)

From `per_tick_build_and_per_second_lookup_cost` (S:1308). Build samples are one whole `convert` divided by its ticks, so "max" is the worst whole run, not the worst single tick. Lookup samples are one `BarFile::first_at_or_after` each.

| | n=10^3 | 10^4 | 10^5 | 10^6 |
|---|---|---|---|---|
| build ns/tick p50 / p99 / max | 11.5 / 26.2 / 79.4 | 25.1 / 39.7 / 227.0 | 33.5 / 54.3 / 54.3 | 48.3 / 86.3 / 86.3 |
| lookup ns p50 / p99 / max | 12,568 / 27,319 / 42,830 | 24,303 / 39,559 / 57,305 | 34,034 / 52,127 / 94,660 | 44,665 / 65,996 / 78,142 |

**The per-second lookup is not O(1).** It is the store's binary search (`store::file::first_at_or_after`), one positioned read per probe, so it grows with log2 n (about 10, 13, 17 and 20 probes at these sizes). This is a store property, outside the gdfl files.

Also measured in release:

| measurement | sizes | p50 | p99 | max |
|---|---|---|---|---|
| `decode_ticker` (dpn_14), by underlying length | 1 / 8 / 24 / 48 | 237 / 247 / 272 / 271 ns | 268 / 272 / 289 / 288 ns | 16–60 µs |
| `NfoDay::locate` (dpn_15), by listing size | 10^3 / 10^4 / 10^5 / 10^6 | 55 / 81 / 136 / 431 ns | — | — |

## 5. Commands and results

All with `CARGO_TARGET_DIR=/home/claude/gdfl-target CARGO_BUILD_JOBS=4`, in `/home/claude/gdfl-wt`:
- `cargo test -p pull --locked --lib gdfl` as root: **224 passed, 2 failed**. The two failures are the root-only permission tests, `gdfl_import::tests::a_journal_that_cannot_be_written_fails_the_day_by_name` and `gdfl_tickstore::tests::a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses`.
- The same test binary copied to `/tmp/gdfl-nobody` and run under `setpriv --reuid=65534 --regid=65534 --clear-groups ./pulltest gdfl`: **226 passed, 0 failed**. Both permission tests pass as nobody; repeated 20x, 0 failures.
- Rename test alone (`--exact …a_renamed_contract_is_keyed_by_its_decoded_contract_never_its_name`), 20x: 0 failures.
- `cargo test -p pull --locked --lib gdfl_nfo`: 59 passed, 0 failed (before the final clippy refactors; the full gdfl run above includes them).
- `cargo clippy -p pull --all-targets --locked -- -D warnings`: clean.
- `rustfmt --edition 2024 --check crates/pull/src/gdfl_*.rs`: clean.
- Old-code measurement (0389b3c with the new S and F, three absent-API lines made to fail by name): `attack_gdfl_seconds` 13 passed, 8 failed (§1).
- `cargo test --release … per_tick_build_and_per_second_lookup_cost -- --nocapture` and `… dpn_14 dpn_15`: pass; numbers in §4.

Gate 11 (`rustc --edition=2024 -O .github/gates_ledger.rs`, then `ledger banned-constructs`):
- Rule 6 (`.iter().position(` in `month_of`) and rule 7 (`.contains(&` in the cutover check) are gone from N.
- All gdfl refusals in I, IT and NT are gone.
- **Still refused, and outside what I changed:** N:602–603, `NfoDay::new` builds `HashMap::new()` and `HashSet::new()` (rule 3). It has no size to pre-size with, and `with_capacity(0)` would only be an evasion. This needs a capacity parameter from its two callers, which do know the entry count.
- The gate still fails overall on other areas' pre-existing entries.

## 6. Open items and honest limits

- The rename pair cannot be imported end to end: `calendar::FIRST_DAY` is 2019-12-02, so every 2018–2019 GDFL day is `CalendarUnmeasured`. The join is proven at `nfo_day`, and the store's keying by contract on measured days.
- `docs/05-decisions.md` and `docs/04-invariants.md` were not edited. The entries above are drafts for the coordinator.
- The share/index split of the long-dated months is not applied: a share name for 2019-06 or later would decode with the table's day. The census says index and stock months share dates, and NSE's rule makes the day a property of the month, but no long-dated share contracts are claimed to exist.
- The scratch-dir fix is hardening. The flake's trigger was not reproduced.

## 7. Files modified

`crates/pull/src/gdfl_nfo.rs`, `gdfl_nfo_tests.rs`, `gdfl_nfo_attack_tests.rs` (bb197f5); `crates/pull/src/gdfl_import.rs`, `gdfl_import_tests.rs`, `gdfl_seconds_attack_tests.rs`, `gdfl_fixtures.rs` (566a5a8). All under `/home/claude/gdfl-wt`.
