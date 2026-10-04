# gdfl-names, round 1 (worktree /home/claude/gdfl-wt, branch claude/attack-gdfl @ 8e6b521)

All tests are in `crates/pull/src/gdfl_nfo_attack_tests.rs` (T below), wired from `crates/pull/src/gdfl_nfo.rs` (N below) the same way `gdfl_nfo_tests.rs` is.

## 1. Attacks

| attack | cases tried | failures on 8e6b521 | fixed? | evidence |
|---|---|---|---|---|
| Monthly-era name whose strike starts with a year (`ADANIENT18DEC195CE` @2018-12-03 -> 2019-12-18 strike 5; `BANKNIFTY18DEC23500CE` -> 2023-12-18 strike 500; `ACC18OCT2150PE`) | 3 hand-picked | 3 of 3 decoded to the WRONG contract, silently | yes | `dpn_01_*` T:128; fix N:112 (const), N:306 (`monthly_reading`), N:382 (rule) |
| Every monthly-form name, all trade months 2018-01..2019-12 (first weekday + month end), contract month +0..+2, yy 00..99 x 5 strike tails x 2 sides x 5 underlyings, plus all 213 F&O underlyings x yy 17..26 | 1,026,720 | **148,716 decoded to a contract** (14.5%), e.g. `ADANIENT18JAN185CE` @2018-01-01 -> 2018-01-18 strike 5 (same year, not just next) | yes, now 0 | `dpn_02_*` T:149 |
| Era cutover: dated name whose DD is also a plausible monthly year, on 2019-12-31 vs 2020-01-01; a dated name with no plausible monthly reading inside the era | 3 | 1 (accepted on 2019-12-31) | yes | `dpn_03_*` T:234 |
| Every dated name: trade days 2018-09-03..2027-12-31 step 11, expiry offsets 0..2200 (weekdays), 213 underlyings, 6 strikes (5 paisa .. 9,999,999,999), CE/PE; must decode exactly and re-encode, or (era only) refuse as FormsAmbiguous | 7,918,488 | 0 | n/a (after fix: 247,932 refused, all in 2018-09..2019-12, all FormsAmbiguous; none misread) | `dpn_04_*` T:262 |
| Leading-zero strike (`0100`, `00.5`, `01`; `ACC18DEC2005CE` -> 2020-12-18 strike 5) | 5 | 4 accepted (non-canonical spelling silently mapped; widens the misread surface) | yes | `dpn_05_*` T:320; fix N:294 |
| Calendar edges: 29FEB leap/non-leap, 30/31FEB, 31APR, 00/32 day, expired, year end, weekend trade day, YY=99, trade 9999-12-31 | 14 | 0 | n/a | `dpn_06_*` T:338 |
| Spelling faults: lower case (each field), whitespace, `.NFO`/`.csv` suffixes, `../`, `/`, `\`, FUT, comma/sign/NUL/non-ASCII, empty, YYMDD weekly `24O10`, YYMMM, 20-digit strike, cap and cap+1 | 33 | 0 (my cap arithmetic was off by one in the first draft, a test bug) | n/a | `dpn_07_*` T:396 |
| Every F&O underlying (incl. `360ONE`, `NIFTYNXT50`, `M&M`, `GVT&D`, `BAJAJ-AUTO`, `NAM-INDIA`) x 10 strikes incl. 0.01, 0.99, 1012.5, 1075.25, 99,999,999.99; 11-digit paisa; zero strike | 4,260 + 213 + 3 | 0 | n/a | `dpn_08_*` T:453 |
| Fuzz 10^6 random names (seed fixed), random trade day 2015..2027: no panic; every Ok re-encodes canonically (or differs only by zero-padded fraction); expiry weekday inside window; in era never a name whose monthly reading also fits; every Err names the ticker | 1,000,000 | before fix: in-era two-form names accepted (e.g. `N1YKAA17MAY189604473.95PE` @2016-10-19) | yes | `dpn_09_*` T:547 (ok 7,400, of which 252 zero-padded fractions; refused 992,600) |
| Exhaustive helpers: `two_digits` all 65,536 byte pairs; `month_of` all 3-byte A..z; `second_of_day` all 10^6 `HH:MM:SS` + 9 malformed | 1,027,123 | 0 | n/a | `dpn_10_*` T:632 |
| Entry names: traversal `..\`, `../`, absolute, drive letter, doubled separators, case of folder/suffix, NUL, trailing `/`; mixed separators; a ticker listed 3 times | 21 | 0 | n/a | `dpn_11_*` T:694 |
| `locate` with 100,000 duplicate tickers in the day | 2,001 probes x 2 | median 57 ns -> 89,714 ns: **O(duplicates) linear scan** behind a "one hash probe" claim | yes (HashSet) | `dpn_12_*` T:751; fix N:459, N:528 (after: 80 ns vs 80 ns) |
| Bent CSV bodies: 100,000 files of 0..5 rows, 9 mutations, CRLF/LF, trailing newline, cap 4 | 100,000 | 0 (no panic; every refusal names a line at or after the first bent line; cap exact) | n/a | `dpn_13_*` T:810 |
| Random entry names (200,000 spliced from a valid one) | 200,000 | 0 (only bare names under the folder are filed) | n/a | `dpn_16_*` T:955 |
| Pre-existing clippy `needless_pass_by_value` on `listing_in(file: Arc<B>)`, `cloned_ref_to_slice_refs` and `panic` in gdfl_nfo_tests.rs; both NFO files not `rustfmt`-clean at 8e6b521 (37 hunks) | — | 3 lint errors + fmt | yes | N:700 (`&Arc<B>`); files formatted |

## 2. Fixes, draft decisions and invariants

### D-3160 — On a trade day of the monthly era, a GDFL option name that reads both as a dated and as a monthly contract is refused, not read as the dated one — 2026-10-04
FORMAT.md §6 as the reader implemented it tried the dated form `DD MON YY STRIKE` first and fell to the monthly form `YY MON STRIKE` only when no dated reading existed. Every monthly name whose strike begins with two digits also reads as a dated name (the month's year becomes the expiry DAY, the strike's first two digits the YEAR), and when that date is a weekday inside the 2,200-day horizon the reader filed it under a wrong contract with no refusal: `ADANIENT18DEC195CE` on 2018-12-03 became 2019-12-18 strike 5, and an enumeration of 1,026,720 monthly-era names found 148,716 such misreads. `decode_ticker` now refuses `FormsAmbiguous` when the trade year is at most `MONTHLY_FORM_LAST_YEAR` (2019, the module's own "monthly 2018–2019 form") and a monthly reading at some split names a month holding a day of `[trade, trade + 2,200]`. After 2019 the dated reading stands alone. The cost is loud: in the dated-name sweep 247,932 of the in-era names were refused rather than read; none was misread. The exact cutover DAY is not recorded and stays unsettled until the real zips are read (below).

| DPN-01 | On a trade day up to `MONTHLY_FORM_LAST_YEAR`, no option name with a monthly reading inside the expiry window decodes to a contract; it is refused as `FormsAmbiguous` | `gdfl_nfo::attack_tests::dpn_01_a_monthly_name_whose_strike_starts_with_a_year_is_never_a_dated_contract` · `dpn_02_no_monthly_era_name_decodes_to_a_contract` · `dpn_03_the_era_cutover_is_the_trade_day_and_nothing_else` |
| DPN-02 | Every dated name `UNDERLYING DD MON YY STRIKE CE/PE` with a weekday expiry in `[trade, trade + 2,200]` decodes to exactly that contract and re-encodes to itself, or (in the monthly era only) is refused as `FormsAmbiguous` | `gdfl_nfo::attack_tests::dpn_04_every_dated_name_round_trips_or_is_refused_by_name` · `dpn_09_a_million_random_names_never_panic_and_every_answer_is_exact_or_named` |

### D-3161 — A GDFL option strike whose whole part begins with 0 (other than 0 itself) is refused — 2026-10-04
`strike_paisa` admitted `0100`, `01` and `00.5`, so a non-canonical spelling filed silently under the canonical contract, and a monthly name's strike `2005` could read as year 20, strike `05`, widening the D-3160 misread surface. The whole part must now be `0` or not start with `0`; `0.5` still reads as 50 paisa. Trailing zeros in the fraction (`100.0`, `107.50`) remain admitted because whether GDFL writes them is not known here; two such spellings of one contract on one day are already refused by `gdfl_import` as `TickerAmbiguous`.

| DPN-03 | No strike with a leading zero in a multi-digit whole part decodes | `gdfl_nfo::attack_tests::dpn_05_a_strike_with_a_leading_zero_is_refused` |

### D-3162 — `NfoDay::locate` keeps duplicate tickers in a set, so a lookup is two hash probes whatever the day holds — 2026-10-04
`locate` was documented as one hash probe but first scanned a `Vec` of every duplicated ticker of the day, so its cost grew linearly with duplicates: a median of 57 ns with one duplicate became 89,714 ns with 100,000. The duplicates are now a `HashSet<Box<str>>`, and the measured medians are 80 ns and 80 ns. The module text now says two probes (the duplicate set and the ticker map).

| DPN-04 | `NfoDay::locate`'s cost does not grow with the number of duplicate tickers a day holds | `gdfl_nfo::attack_tests::dpn_12_locate_does_not_grow_with_the_duplicates_a_day_holds` |

### D-3163 — `gdfl_nfo::listing_in` borrows the year zip's `Arc`, and the NFO files are rustfmt- and clippy-clean — 2026-10-04
At 8e6b521 `cargo clippy -p pull --all-targets -- -D warnings` failed on `listing_in(file: Arc<B>, ..)` (`needless_pass_by_value`) and on two lints in `gdfl_nfo_tests.rs`, and both NFO files had 37 rustfmt hunks. `listing_in` now takes `&Arc<B>`; its two callers are in the NFO files. No behaviour changed.

| DPN-05 | Decoding a bent options CSV never panics, every refusal names a line at or after the first bent line, and the row bound is exact | `gdfl_nfo::attack_tests::dpn_13_a_hundred_thousand_bent_files_never_panic_and_refuse_at_a_line` |

## 3. Commands
- `CARGO_TARGET_DIR=/home/claude/gdfl-target CARGO_BUILD_JOBS=2 cargo test -p pull --locked --lib gdfl_nfo::attack_tests` on unmodified code: 7 passed, 7 failed (dpn_01, 02, 03, 05, 07 [test bug, mine], 09, 12).
- Same filter `gdfl_nfo` after the fix: **37 passed, 0 failed** (21 original + 16 attack).
- `... cargo test -p pull --locked --lib gdfl`: 200 passed, 2 failed — `gdfl_import::tests::a_journal_that_cannot_be_written_fails_the_day_by_name` and `gdfl_tickstore::tests::a_temporary_or_absent_day_is_none_and_an_unreadable_one_refuses`, both permission tests that fail as root (box runs as root), neither in my files.
- `cargo clippy -p pull --all-targets --locked -- -D warnings`: zero errors in gdfl_nfo*.rs; remaining errors are in gdfl_import.rs (4), gdfl_import_tests.rs (1), gdfl_fixtures.rs (19), gdfl_seconds_attack_tests.rs (153) — the other attacker's files.
- `rustfmt --edition 2024 --check` on the three NFO files: clean.

## 4. Cost (debug test profile, this box, measured)
- `decode_ticker`, 20,000 calls per length (half accepted, half refused), underlying length 1/8/24/48: p50 305/330/377/378 ns, p99 362/537/681/596 ns, max 8.0 ms in every row (scheduler outliers; one run earlier showed max 26-70 µs). Flat in name length below the 64-byte cap.
- `NfoDay::locate`, 10,000 probes, listings of 10^3/10^4/10^5/10^6 entries: p50 67/218/497/751 ns, p99 283/688/1,194/1,326 ns, max 26.7 µs/22.5 µs/7.8 ms/36.2 µs. Algorithmically one probe; the p50 growth tracks the map outgrowing cache, not a scan.
- `locate` with 1 vs 100,000 duplicates: before 57 ns vs 89,714 ns; after 80 ns vs 80 ns.

## 5. Needs the real zips (Mac thread)
1. The last trade day on which any GDFL options file name is in the monthly form `YY MON STRIKE`, and the first day carrying a dated `DD MON YY` name, per underlying class (index weeklies vs stock monthlies). If FORMAT.md §6 states a cutover day, it should replace `MONTHLY_FORM_LAST_YEAR` = 2019; until then every 2018–2019 name that reads both ways is refused.
2. Whether a long-dated contract listed in monthly form keeps its monthly name when traded after the cutover (e.g. a 2019-listed NIFTY long-dated option traded in 2020+). After 2019 the reader takes the dated reading alone, so such a name would be misread if one exists.
3. How many 2018–2019 names the new rule refuses on the real tree (count of `FormsAmbiguous` per day), and whether any refused name is a real dated weekly.
4. Whether GDFL ever writes a strike with fraction zero padding (`100.0`, `107.50`) or a leading zero; the reader admits the former and now refuses the latter.
5. Whether any GDFL options file starts with a byte-order mark or carries a header other than the two observed (refused as `HeaderUnknown` today).
6. Whether any stated expiry falls on a weekend (refused today, reported as `MonthlyExpiryUnstated`, a misleading name for that case).

## 6. Files modified
- /home/claude/gdfl-wt/crates/pull/src/gdfl_nfo.rs (fixes, doc, wiring of the attack module, rustfmt)
- /home/claude/gdfl-wt/crates/pull/src/gdfl_nfo_tests.rs (FormsAmbiguous added to the refusal list, `&Arc` call, two clippy fixes, rustfmt)
- /home/claude/gdfl-wt/crates/pull/src/gdfl_nfo_attack_tests.rs (new, 16 tests)

Out-of-area note (not fixed, not mine): `gdfl_import::nfo_day` keeps an undecodable name when `ticker.starts_with(only)`, so `--only NIFTY` also claims undecodable `NIFTYNXT50…`/`NIFTYBANK…` names as refusals for the run (loud, not silent).
