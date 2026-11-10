# EXTREMES-REPORT — extreme and failure cases of the data path

Branch `cloud/extremes-20261110`, cut from `origin/feat/db-fix` at
`beaf2e3c4`, written by a cloud session on 10 Nov 2026. **No product code was
changed.** The branch adds eight test files and this report. Every vendor
value in the tests is invented at run time. No market data, store file or
secret is committed, and nothing under `runtime/` is touched.

## Summary

- **68 new tests in 8 new files.**
  - 56 run in CI and pass.
  - 11 are **findings**: each is kept as an `#[ignore]`d test named `finding_*`
    that fails on `feat/db-fix`.
  - 1 is an on-demand real disk-full sweep. It is `#[ignore]`d because it
    needs a small mount; it was run here and passed.
- **The store held under everything aimed at its write path.** Each of these
  reopened at a whole committed prefix and resumed to byte-identical files:
  - every crash point of three kinds of append, across all five files of a
    month;
  - a writer process killed by `SIGKILL` at seven points;
  - a stocks import killed at seven points;
  - readers racing a writer;
  - a real `ENOSPC` at 41 fill levels.
- **The findings are about WHAT reaches the store, not how it is stored.** The
  two most harmful:
  - A NIFTY minute candle priced at **zero** is stored silently through the
    Zerodha door.
  - Outside the measured calendar (before 2019-12-02, and after 2026-09-04,
    which covers every pull since September), Zerodha candles stamped on a
    **closed day** are written into the minute month. Every derived rung of
    that answer is then withheld.

## How to run

```bash
cargo test -p store --test extremes_month --test extremes_crash --test extremes_disk_full
```

```bash
cargo test -p pull --test extremes_gdfl --test extremes_zerodha --test extremes_calendar --test extremes_gdfl_kill
```

```bash
cargo test -p core --test extremes_symbol
```

Each finding fails by design until it is fixed. To run them, add the
`--ignored` flag with the `finding_` filter after `--`. For example:

```bash
cargo test -p pull --test extremes_zerodha -- --ignored finding_
```

The disk-full sweep needs a small filesystem, for example a 4 MiB tmpfs
mounted as root:

```bash
BRUTEX_SMALL_FS=/path/to/small/mount cargo test -p store --test extremes_disk_full -- --ignored
```

| File | Tests | Findings |
|---|---|---|
| `crates/store/tests/extremes_month.rs` | 16 | 1 |
| `crates/store/tests/extremes_crash.rs` | 8 | 1 |
| `crates/store/tests/extremes_disk_full.rs` | 1 (on demand) | 0 |
| `crates/pull/tests/extremes_gdfl.rs` | 19 | 3 |
| `crates/pull/tests/extremes_gdfl_kill.rs` | 1 | 0 |
| `crates/pull/tests/extremes_zerodha.rs` | 10 | 4 |
| `crates/pull/tests/extremes_calendar.rs` | 8 | 2 |
| `crates/core/tests/extremes_symbol.rs` | 5 | 0 |

## Findings, ranked by harm to data correctness

**Ranking rule:** a wrong value stored silently ranks first, then data lost
silently, then a wrong answer from a second authority, then a loud refusal of
good data, then a broken guarantee that no test caught corrupting anything.

| Rank | Id | Finding | Where | Severity |
|---|---|---|---|---|
| 1 | X-01 | A NIFTY candle priced at zero is stored, and no failure is named | Zerodha door | high |
| 2 | X-02 | Outside the measured calendar, closed-day candles are stored; the derived rungs of the whole answer are then withheld | Zerodha landing | high |
| 3 | X-03 | A day zip with no member closes its day `done`, so the day is skipped for ever | GDFL import | high |
| 4 | X-04 | The store admits a bar priced at zero (the write-boundary half of X-01) | store | medium |
| 5 | X-05 | Two expiry authorities disagree on every Muhurat weekday | calendar / expiry | medium |
| 6 | X-06 | Vendor rows on a day the calendar calls closed leave no trace | GDFL import | medium |
| 7 | X-07 | A lot revised in the middle of a contract is folded into a common divisor (75 to 65 is recorded as 5) | GDFL lot ledger | medium |
| 8 | X-08 | A `null` volume in a Kite candle becomes a traded zero, with no count | Zerodha decode | low–medium |
| 9 | X-09 | Both gap-ledger readers read `2024-+6-+3` as 2024-06-03 | Zerodha ledgers | low |
| 10 | X-10 | One candle sent with and without its OI cell is refused as conflicting | Zerodha door | low |
| 11 | X-11 | A reader of a month with no `.lock` file does not exclude a writer | store | low |

**Line numbers are of `beaf2e3c4`.** Each entry below is also the doc comment
of its test.

### X-01 — a zero-priced NIFTY candle is stored (Zerodha door)

- **Test:** `finding_a_nifty_candle_priced_at_zero_is_stored`
  (`crates/pull/tests/extremes_zerodha.rs`).
- **Input:** a NIFTY minute answer for 2024-06-05, sent through
  `pull::ingest::from_window`. Its 201st candle has open, high, low and close
  all `0`.
- **Expected:** the candle is refused by name and the minute is left empty.
  NIFTY never prints zero, and a stored zero reads to every indicator as a
  100% crash in one minute.
- **Actual:** stored. `failures` is empty, and the month holds one bar priced
  at zero.
- **Likely cause:**
  - The identity screen exempts every INDEX from its price floor:
    `crates/pull/src/identity_screen.rs:355-356`, `zero_held`, with the comment
    "NIFTY50DIVPOINT restarts from zero, D-4992".
  - The store then admits the zero (X-04).
  - The GDFL capital-market reader refuses a `0` LTP (`ZeroLtp`), so this path
    is through Zerodha only.
- **Proposed fix:** scope the exemption to the indices that can be zero (the
  dividend-point series), keyed by symbol. Refuse a zero on every other index,
  above all the swept NIFTY and BANKNIFTY.

### X-02 — closed-day candles are stored outside the measured calendar (Zerodha landing)

- **Test:** `finding_candles_on_closed_days_outside_the_measured_calendar_are_stored`
  (`crates/pull/tests/extremes_zerodha.rs`).
- **Input:** one NIFTY answer holding a regular session plus 60 candles stamped
  on a closed day. The sourced calendar (circulars, 2018 to 2026) knows each
  of these days is closed; the measured calendar (2019-12-02 to 2026-09-04)
  does not cover any of them:
  - Saturday 2018-03-03;
  - Saturday 2026-09-12;
  - Gandhi Jayanti, 2026-10-02.
- **Expected:** the closed day's candles are dropped and counted, as they are
  inside the measured span. The passing test
  `candles_on_a_closed_day_inside_the_measured_calendar_are_not_stored` shows
  Saturday 2024-06-01 handled this way.
- **Actual:**
  - In all three cases, all 60 candles are written into the minute month,
    which then holds 435 bars.
  - The derived rungs then report "minute bars on days the calendar measures
    closed … store defect; derived buckets withheld" across both days. So the
    open day's 2min to 60min bars are withheld as well, with 7 or 8 failures
    per answer.
- **Likely cause:**
  - `crates/pull/src/fetch.rs:1071-1075` keeps a row on an "unclassified" day.
  - `crates/pull/src/session.rs:1304` (`on_unclassified_day`) decides
    "unclassified" with the measured `kind_of`.
  - The derivation judges the same day with the sourced calendar.
- **Proposed fix:** judge the row's day at landing with the calendar the
  derivation uses (`sourced_kind_of`, with recorded windows). Keep
  `unclassified_kept` for days that no calendar covers, or refuse the
  closed-day rows by name before the append.
- **Note:** Kite normally sends no candle for a closed day, so the trigger is a
  vendor or stamping fault. That is exactly the case the drop exists for.
  Every Zerodha day after 2026-09-04 is in this window today.

### X-03 — a day zip with no member closes the day `done` (GDFL import)

- **Test:** `finding_a_day_zip_with_no_member_closes_its_day_done`
  (`crates/pull/tests/extremes_gdfl.rs`).
- **Input:** the stocks day zip for 2024-06-06 is a valid zip that holds no
  entry.
- **Expected:** the day is refused or left open, its emptiness is named, and
  the next run reads it again.
- **Actual:**
  - The journal writes
    `done stocks 2024-06-06 RELIANCE … files=0 seconds=0 failures=0 … entries=0`.
  - `days_imported` is 1.
  - Every later run skips the day, and the store holds none of its bars with
    nothing saying why.
- **Likely cause:** `crates/pull/src/gdfl_import.rs:2364-2373`. When there is
  no job, a refusal is made only if `entries > 0 && foreign`. The doc at
  `:2333-2335` names this as limit L-IA16-08.
- **Proposed fix:** close an empty listing as `refused reason=day_zip_empty`,
  an open cause like `day_not_held`. It is then read again and counted in the
  run's refusals.

### X-04 — the store admits a bar priced at zero

- **Test:** `finding_a_bar_priced_at_zero_is_admitted_to_a_spot_month`
  (`crates/store/tests/extremes_month.rs`).
- **Input:** one bar of the NIFTY 2024-06 month with open, high, low and close
  all `0`, sent through `BarFile::append`.
- **Expected:** `StoreError::ImpossibleBar`, which is what a negative price
  gets.
- **Actual:** `Committed`.
- **Likely cause:** `crates/store/src/format.rs:800`. The sign clause is
  `>= 0`, and `crates/pull/src/http.rs` documents zero as "a legal price".
- **Proposed fix:** **needs a decision**, because the zero rule is recorded as
  deliberate. Either refuse a zero for every `Bar`, or refuse it for the INDEX
  and CASH segments, where it can never be real. This is the write boundary
  every path crosses, so it would also stop X-01.

### X-05 — two expiry authorities disagree on every Muhurat weekday

- **Test:** `finding_the_two_expiry_authorities_disagree_on_a_muhurat_weekday`
  (`crates/pull/tests/extremes_calendar.rs`).
- **Input:** a NIFTY contract stated to expire on a Muhurat weekday that the
  calendar records: 2021-11-04, 2022-10-24, 2024-11-01 or 2025-10-21.
  2025-10-21 is a Tuesday, the weekday NIFTY's weekly contracts expire on.
- **Expected:** one settlement day. `calendar::expiry_day` records that a
  Muhurat is no session a contract can expire on.
- **Actual:**

  | Stated expiry | `calendar::settlement_on_or_before` | `expiry_alias::settle` |
  |---|---|---|
  | 2021-11-04 | 2021-11-03 | the Muhurat itself |
  | 2022-10-24 | 2022-10-21 | the Muhurat itself |
  | 2024-11-01 | 2024-10-31 | the Muhurat itself |
  | 2025-10-21 | 2025-10-20 | the Muhurat itself |

- **Likely cause:**
  - `crates/pull/src/expiry_alias.rs:800`: `settle` calls `settle_by` with
    `sourced_kind_of`, which answers a Muhurat as open.
  - `:775`: `previous_open_in` takes `OpenLengthUnmeasured` as an open session.
- **Proposed fix:** use one authority. `settle_by` and `previous_open_in`
  should ask `calendar::expiry_day`, or `settlement_on_or_before` when no
  alias row applies.
- **Harm:** the tenor and greeks of a contract stated for one of those days
  depend on which authority was asked.

### X-06 — vendor rows on a day the calendar calls closed leave no trace (GDFL import)

- **Test:** `finding_vendor_rows_on_a_day_the_calendar_calls_closed_leave_no_trace`
  (`crates/pull/tests/extremes_gdfl.rs`).
- **Input:** stocks day zips for Saturday 2024-06-01 and for the holiday
  2024-08-15, each holding a RELIANCE file of in-session trades.
- **Expected:** a journal line, a failure or a refusal says that the vendor
  shipped rows for a closed day. An unrecorded special session, or a calendar
  error, is then seen rather than lost.
- **Actual:** `days_closed` is 1, and there is no journal line, no failure and
  no entry in any closed or refused list. The same silence was met by
  accident while building fixtures, on Bakri Id (2024-06-17).
- **Likely cause:** `crates/pull/src/gdfl_import_day.rs:614-621` returns before
  the source is read. Both sources still file the day's zip
  (`crates/pull/src/gdfl_dayzips.rs:181-190`).
- **Proposed fix:** on a `Closed` day, ask the source whether it holds the day
  (a listing). When it does, name it with a warning event and a journal note.
  Never import it.

### X-07 — a lot revised mid-contract is folded into a common divisor (GDFL lot ledger)

- **Test:** `finding_a_lot_revised_mid_contract_is_folded_into_a_common_divisor`
  (`crates/pull/tests/extremes_gdfl.rs`).
- **Input:** `pull::gdfl_lineage::LotLedger::observe` is fed two NIFTY
  contracts. The lot for each day is taken from `costs::lot::lot_size_of`,
  which cites the circulars:

  | Contract | Circular | Lot before | Lot after | Ledger's lot after |
  |---|---|---|---|---|
  | quarterly settling 2026-03-30 | NSE/FAOP/70616 | 75 (on 2025-12-29) | 65 (on 2025-12-31) | **5** |
  | far contract of 2025-03-27 | NSE/FAOP/64625 | 25 (on 2024-12-24) | 75 (on 2024-12-27) | **25** |

- **Expected:** after each revision, the ledger holds the revised lot.
- **Actual:** the ledger holds the greatest common divisor of the two lots
  (last column), never the revised lot.
- **Likely cause:** `crates/pull/src/gdfl_lineage.rs:1222-1243`. It keeps "the
  common divisor of every day's" lot, on the premise that a lot "can only fall
  within one lineage". The cited circular revises existing quarterly
  contracts in mid-life.
- **Proposed fix:** when a day's GCD and the held lot are not multiples of one
  another, record a lot change from that day. Keep the common-divisor fold for
  days that only refine the same lot. Alternatively, consult the costs crate's
  sourced revision dates.
- **Harm:** `imports/gdfl.lots` is wrong for these contracts. Options are
  stored and never swept.

### X-08 — a `null` volume in a Kite candle becomes a traded zero

- **Test:** `finding_a_null_volume_in_a_kite_candle_is_decoded_as_zero`
  (`crates/pull/tests/extremes_zerodha.rs`).
- **Input:** a positional Kite candle for an equity whose volume cell is
  `null`.
- **Expected:** refused, as the same `null` is in the other two decode shapes
  (`decode_objects` and the parallel arrays both go through `one_volume`). At
  the least, it should be counted.
- **Actual:** decoded as volume `0`, for every listing, with no count and no
  event.
- **Likely cause:** `crates/pull/src/http.rs:3810-3811`, the line
  `serde_json::Value::Null => 0` in `decode_positional`, which is the shape
  Kite answers in.
- **Proposed fix:** send the cell through `one_volume`. If a `null` must be
  tolerated, allow it on an index only and count it.

### X-09 — both gap-ledger readers read a signed date as a real day

- **Test:** `finding_the_gap_ledger_reads_a_signed_date_as_a_real_day`
  (`crates/pull/tests/extremes_calendar.rs`).
- **Input:** the gap line the door itself wrote for 2024-06-03, with its date
  re-spelt `2024-+6-+3` (a hand edit or a damaged byte).
- **Expected:** a fault; the line is never read as 2024-06-03.
- **Actual:** both readers read it as 2024-06-03 and name no fault:
  - `zerodha_verify::GapLedger::gap`;
  - `ingest::gap_backfill::ledgered(..).holds`, so a back-fill would be planned
    for the misread day.
- **Likely cause:** `crates/pull/src/zerodha_verify.rs:432-450` and
  `crates/pull/src/gap_backfill.rs:124-131` check the field widths and then
  call `str::parse`, which accepts a leading `+`. This is the class the
  previous cloud round fixed in the `api` routes.
- **Proposed fix:** one strict reader that accepts ASCII digits only.
  `reference_data::parse_day` already is one, and the new
  `the_reference_date_reader_takes_exactly_a_real_iso_date` pins it.

### X-10 — one candle sent with and without its OI cell is refused

- **Test:** `finding_one_index_candle_sent_with_and_without_its_oi_cell_is_refused`
  (`crates/pull/tests/extremes_zerodha.rs`).
- **Input:** the 09:20 NIFTY candle twice in one answer, with the same OHLCV:
  once as a 6-cell row and once as a 7-cell row whose OI cell is `0`.
- **Expected:** one bar. An index's OI is withheld at landing anyway.
- **Actual:** both rows are refused as `conflicting_candles`, so the minute is
  lost. It is named.
- **Likely cause:** the duplicate screen compares whole rows
  (`crates/pull/src/ingest.rs:1489-1528`) before `fetch::land_rows` withholds
  the OI (`crates/pull/src/fetch.rs:1207-1211`).
- **Proposed fix:** compare on the fields the series keeps.
- **Note:** low, because Kite puts a `0` OI cell on every index candle (the
  comment at `crates/pull/src/fetch.rs:1201-1203`), so a mixed answer is
  unlikely.

### X-11 — a reader of a month with no `.lock` does not exclude a writer

- **Test:** `finding_a_reader_of_a_month_without_its_lock_file_is_invisible_to_a_writer`
  (`crates/store/tests/extremes_crash.rs`).
- **Input:** a month of 1,125 bars whose `.lock` is absent (restored from a
  copy, for example). A reader opens it through `open_existing`. A writer then
  calls `open_or_create`.
- **Expected:** the writer is refused `Locked`, as it is when the `.lock`
  exists.
- **Actual:** the writer opens and appends 375 bars under the open reader. The
  reader's committed prefix still read whole, so no wrong bar was observed. The
  exclusion is simply absent, and nothing says so.
- **Likely cause:** `crates/store/src/file.rs:2199-2209`. Its comment says "a
  bar file with no lock beside it has had no writer since it was made", which
  says nothing about a writer that arrives later.
- **Proposed fix:** lock the bar file's own descriptor when no `.lock` exists,
  or refuse the month by name.

## Measured, but needs a decision rather than a fix

These are not called findings. Each is either deliberate in the code, or
depends on a vendor fact the repository does not verify.

1. **A stock file cut at a row boundary mid-session imports as a partial day
   with no flag.**
   - Measured by a throwaway probe (not committed): a RELIANCE file that stops
     at 11:59:59 lands 4,950 bars and closes
     `done stocks 2024-06-13 … files=1 seconds=4950 failures=0`.
   - An index file must reach the close, and
     `an_index_file_cut_at_a_row_boundary_is_refused_then_the_whole_file_lands`
     shows it is refused. A stock file has no such rule.
   - UNVERIFIED: whether a real GDFL stock file always runs to the close. If
     it does, the same rule belongs on stocks.
2. **A late trade can fall outside the window it happened in.**
   - A late in-window row whose next in-order row is after the window is
     placed there and dropped with it.
   - It is counted in `late_rows`.
   - This follows the no-look-ahead placement rule (`place_all`,
     `crates/pull/src/gdfl_import.rs:1286`).
   - Pinned by `the_2024_muhurat_keeps_only_its_window_with_rows_out_of_order`.
3. **The strict checksum audit refuses a month a crash left behind.**
   - `open_existing_audited` refuses a month that holds bytes past its commit
     ("non-exact extents") until the next append rewrites them.
   - The reader door serves the month.
   - An operator who audits straight after a crash sees a refusal.
4. **A `.crc` entry torn inside its own four bytes refuses the tail block.**
   - Only storage without atomic sector writes can produce this.
   - The tail block's committed bars, and the resume, are refused loudly with
     `BlockChecksum` until the month is repaired. This is the documented
     D-0688/D-0910 class.
   - Pinned by
     `a_checksum_entry_torn_inside_its_four_bytes_is_refused_by_name_and_never_served`.
5. **`Symbol::new` accepts a symbol of punctuation alone** (`-`, `&`, `_`).
   Nothing in the repository says an NSE symbol must hold a letter or digit,
   so no test asserts that it must.

## What held

These tests pass, and each was the first of its kind here.

### Store

- **Every crash point of an append, across all five month files.**
  - The writes are taken in the writer's order: records, `.crc` seal (torn
    between 4-byte entries), `.tix` entries, the 64-byte header slot (torn at
    every one of its changed bytes), then the `.srt` and `.sum` renames.
  - Three kinds of append are taken apart this way: a month's first append,
    the 1,024-bar index-threshold crossing, and an indexed append.
  - Every state opened through the reader door at exactly its committed
    prefix.
  - Re-offering the batch landed every file byte-identical and passed the full
    checksum audit.
- **A real writer process killed by `SIGKILL`** at seven points of 300
  appends. Each run reopened at a whole committed prefix and resumed to a
  clean month.
- **Two readers racing a writer** that opens, appends and closes per batch:
  - every refusal either side met was `Locked`;
  - every read was a whole prefix;
  - the final month was exact.
- **A real `ENOSPC`**, on demand, measured here on a 4 MiB tmpfs:
  - 0 to 5 free pages: the append is refused with `DiskFull`;
  - 6 free pages or more: it commits;
  - at every level the month read at its committed prefix, and once space was
    freed the resume landed `.bin`, `.crc`, `.tix`, `.srt` and `.sum`
    identical to a month written on a roomy disk.
- **A month with a bar in every slot** of the leap February (41,760 bars):
  - it reads back exactly;
  - it refuses 1 March's first instant;
  - one batch and 29 batches hold the same records and the same checksums.
- **The IST month and year edges hold at the store:**
  - 29 February is February's alone;
  - February 2023 and February 2100 are 28 IST days;
  - 03:00 IST on 1 January (still 31 December in UTC) is January's.
- **Batches with duplicates or late bars are refused whole and write nothing:**
  - in-batch duplicates, whether the copies agree or not;
  - late interleaved bars;
  - a superset whose overlap is one paisa wrong.
- **A resumed superset lands the clean bytes.**
- **Values at the edge of `i64`, including a volume sum past `i64::MAX`,** are
  stored without a panic.
- **The same input twice, or written into two roots,** leaves every file of the
  month byte-identical.

### GDFL import

- **Volume:** a trade in every second (47,250 rows) imports exactly 22,500
  bars, and a file one row past the 1,000,000-row cap is refused by name.
- **Damaged containers are refused and the rerun lands the clean store:**
  - an empty member;
  - a member cut inside its deflate stream;
  - a member whose stated CRC lies.
- **Special sessions:**
  - The Budget Saturday with a duplicate row inside a cut member: the cut file
    is refused, then the whole file lands, with the duplicated second's volume
    summed.
  - The DR drill keeps both of its windows and drops the gap between them.
- **Calendar edges:** the year boundary and the leap day each file their
  sessions in the correct IST month.
- **Losing the import journal:** a rerun after it is deleted re-reads every
  day and commits no second twice.
- **Prices as text:** 26 LTP spellings are either read to the exact paisa or
  refused. The refused include signs, exponents, words, a third decimal and
  non-ASCII digits.
- **Renames:** the chained rename `LTI -> LTIM -> LTM` files and refuses
  correctly on the eve and the day of each change.
- **Kills:** a stocks import killed at seven points resumes to a byte-identical
  store. The existing kill test covers the options tree only.

### Zerodha import

- An empty answer names the day it owed in the gap ledger.
- A positional row of the wrong width or cell type refuses the whole window,
  and a `null` price skips its row alone.
- An out-of-order answer lands none of its bars and is named.
- The year boundary and the leap day file correctly.
- Inside the measured calendar, closed-day candles are dropped.

### Core symbols and calendar

- **Renames:** every applied row of the 27-row rename table files and refuses
  on exactly its own eve and day, for the old stem and the new.
- **The rename table is consistent with itself,** and the swept F&O list names
  no retired symbol.
- **Symbols:** case folds (`m&m` is `M&M`), but no look-alike character does:
  full-width, Cyrillic, zero-width or non-breaking spaces, and control bytes
  are refused.
- **Dates:**
  - every impossible date is refused, and the year bounds hold;
  - `succ`, `end_of_month` and `months_before` cross the leap day and the year
    end correctly;
  - the reference date reader takes only `YYYY-MM-DD`;
  - the expiry leap rule holds at the century years.
- **The special days of record classify as the calendar's own tables say.**
  This includes 2024-05-20 (the election holiday), which nothing asserted
  closed before.
- **F&O membership:** a share that leaves F&O (its last series runs to expiry
  and no new series is issued) and comes back is a member through its last
  expiry, not a member in between, a member again on its return, and two
  spans of one series.

## Coverage: case × component

New tests are named in code. "existing" points at a test that was already on
the branch and was not duplicated. An id in brackets is a finding.

| Case | Store | GDFL import | Zerodha import | Symbol / calendar |
|---|---|---|---|---|
| Empty file | `an_empty_month_answers_every_lookup_as_empty_through_every_door` | `finding_a_day_zip_with_no_member_closes_its_day_done` [X-03]; `an_empty_member_is_refused_by_name_and_the_rerun_lands_the_clean_store`; `a_header_only_member_stores_nothing_and_names_its_refusal` | `an_empty_answer_lands_no_bar_and_names_the_day_it_owed` | n/a |
| One row | `a_one_bar_month_answers_every_lookup_at_its_edges` | existing reader-level tests in `gdfl_cm.rs`; not added at import level | existing `tests/zerodha_identity.rs:432` | n/a |
| Very large file | `a_month_with_a_bar_in_every_minute_slot_reads_back_whole_and_refuses_one_more` (41,760 bars); `one_huge_batch_and_twenty_nine_daily_batches_hold_the_same_records_and_checksums` | `a_stock_day_with_a_trade_in_every_second_imports_every_second_exactly` (47,250 rows); `a_file_one_row_past_the_row_cap_is_refused_by_name_and_stores_nothing` (1,000,001 rows) | not added | n/a |
| Duplicated rows | `a_batch_repeating_a_stamp_is_refused_whole_whether_or_not_the_copies_agree`; `a_resumed_superset_appends_only_its_tail_and_lands_the_clean_bytes`; `a_superset_whose_overlap_is_one_paisa_wrong_appends_nothing` | `a_budget_saturday_with_a_duplicate_inside_a_cut_member_is_refused_then_lands_whole` | `finding_one_index_candle_sent_with_and_without_its_oi_cell_is_refused` [X-10]; existing `tests/derive.rs:366` | `the_rename_table_is_consistent_with_itself` |
| Rows out of time order | `late_bars_interleaved_with_held_ones_are_refused_and_change_nothing` | `the_2024_muhurat_keeps_only_its_window_with_rows_out_of_order` | `an_out_of_order_answer_lands_none_of_its_bars_and_is_named` | rename chain links in date order |
| Zero-volume seconds | `zero_volume_and_zero_interest_are_stored_as_zero_and_absent_stays_absent` | `a_stock_day_of_quotes_alone_stores_no_bar`; index bars of volume 0 | `finding_a_null_volume_in_a_kite_candle_is_decoded_as_zero` [X-08] | n/a |
| Price 0, negative, non-numeric | `finding_a_bar_priced_at_zero_is_admitted_to_a_spot_month` [X-04]; `the_largest_values_round_trip_and_an_overflowing_volume_sum_is_not_a_panic` | `every_ltp_spelling_is_read_exactly_or_refused` | `a_positional_row_of_the_wrong_shape_refuses_the_window_and_a_null_price_skips_its_row`; `finding_a_nifty_candle_priced_at_zero_is_stored` [X-01] | n/a |
| Lot size changes mid-contract | n/a | `finding_a_lot_revised_mid_contract_is_folded_into_a_common_divisor` [X-07] | n/a | existing `crates/costs/src/lot.rs:1407` |
| Symbol rename | n/a | `a_chained_rename_files_each_stem_under_ltm_only_on_its_own_days` | not added | `every_applied_rename_files_and_refuses_on_exactly_its_own_edges`; `the_swept_list_names_no_retired_symbol`; `a_symbol_folds_case_and_refuses_every_look_alike` |
| Stock leaves and re-enters F&O | n/a | not reachable: the import's scope is the static F&O list | n/a | `a_share_that_leaves_fno_and_comes_back_is_two_spans_and_not_a_member_between` |
| Special sessions and holidays | n/a (by design, the store does not judge sessions) | Budget Saturday, 2024 Muhurat and DR drill tests above; `finding_vendor_rows_on_a_day_the_calendar_calls_closed_leave_no_trace` [X-06] | `candles_on_a_closed_day_inside_the_measured_calendar_are_not_stored`; `finding_candles_on_closed_days_outside_the_measured_calendar_are_stored` [X-02] | `the_special_days_of_record_classify_as_the_calendars_own_tables_say`; `finding_the_two_expiry_authorities_disagree_on_a_muhurat_weekday` [X-05] |
| Leap days and year boundaries | `the_leap_day_belongs_to_february_2024_alone`; `a_february_without_a_twenty_ninth_is_twenty_eight_ist_days_including_2100`; `the_year_boundary_is_drawn_at_ist_midnight_not_utc` | `the_year_boundary_files_each_session_under_its_own_ist_month`; `the_leap_day_session_lands_in_february_and_the_next_in_march` | `an_answer_across_the_year_boundary_files_each_session_in_its_own_month`; `an_answer_across_the_leap_day_files_it_in_february` | `day_new_refuses_every_impossible_date_and_takes_every_real_edge`; `succ_end_of_month_and_months_before_cross_the_leap_day_and_the_year`; `sessions_are_counted_across_the_year_end_and_the_leap_day`; `an_expiry_on_twenty_nine_february_follows_the_century_rule` |
| Truncated or corrupt zip members | n/a | `a_member_cut_inside_its_deflate_stream_is_refused_and_the_rerun_lands_clean`; `a_member_whose_stated_crc_lies_is_refused_and_the_rerun_lands_clean`; `an_index_file_cut_at_a_row_boundary_is_refused_then_the_whole_file_lands` | n/a (no zip) | n/a |
| Crash mid-write, then resume | `every_crash_point_of_a_months_first_append_reopens_and_resumes_clean`; `..._of_the_threshold_crossing_append_...`; `..._of_an_indexed_append_...`; `a_writer_killed_at_seven_points_leaves_a_committed_prefix_and_resumes_clean`; `a_resume_that_skips_the_crashed_day_commits_the_next_and_invents_nothing`; `a_checksum_entry_torn_inside_its_four_bytes_is_refused_by_name_and_never_served` | `a_stocks_import_killed_at_seven_points_resumes_to_the_clean_store`; existing (options) `gdfl_import_tests.rs:4979` | existing `zerodha_verify_tests.rs:1271`, `:1763` | n/a |
| Two readers during a write | `two_readers_racing_a_writer_see_only_whole_committed_prefixes`; `finding_a_reader_of_a_month_without_its_lock_file_is_invisible_to_a_writer` [X-11] | existing `gdfl_ia16_tests.rs:1179`; two runs at once, `gdfl_import_tests.rs:5992` | not added | n/a |
| Disk full | `on_demand_a_disk_that_fills_mid_append_leaves_a_committed_prefix_and_resumes_clean` (run here) | existing `gdfl_ia16_tests.rs:1441` (injected), `:2975` (free-space floor) | not added | n/a |
| Same import twice | `the_same_batches_in_two_fresh_roots_leave_identical_month_files`; `writing_the_whole_input_again_after_a_reopen_changes_no_byte_of_any_file` | `a_rerun_and_a_rerun_after_the_journal_is_lost_change_no_bar_byte` | existing `tests/broker.rs:585` | n/a |
| Permutations | crash point × 3 append kinds × 5 files; kill × resume; full disk × 41 fill levels; skipped crashed day × resume | Budget Saturday × duplicate × cut member; Muhurat × late rows; cut or lying member × resume; journal lost × rerun | closed day × outside the measured span; duplicate × OI cell shape | 27 renames × eve and day × old and new stem; leave × re-enter |

## What could not be covered, and why

- **A real `ENOSPC` in CI.**
  - It needs a small mount, and CI has neither the mount nor the privilege to
    make one.
  - The sweep is `#[ignore]`d with the exact command in its doc, and was run
    here as root.
  - An `ENOSPC` on the 64-byte header-slot write cannot happen on such a mount,
    because the header region is allocated when the month is created. Only the
    store's private fault seam reaches it (`crates/store/src/file_fault_tests.rs`), and no
    existing file was edited to use it.
- **Power loss, as opposed to a process kill.**
  - Losing unflushed pages needs a VM or a fault-injecting block device.
  - The crash-state enumeration models it at write granularity, under the
    store's own barrier order.
  - Byte-torn writes inside a 4-byte checksum entry are modelled separately
    (item 4 above).
- **Readers in other processes.** The races use threads. `flock` locks belong
  to the open file description, so the semantics are the same, but a
  cross-process race was not run.
- **A real full disk during an import.** The existing injected test and the
  free-space floor cover it; it was not re-run on a small mount.
- **A one-row file at the import level, and Zerodha disk-full or reader
  races.** These were not added; existing tests cover the nearest shapes (see
  the table).
- **`reference_data::FnoTable::member_on`.** It has no test anywhere, and
  constructing it needs the reference-data file format. It was reviewed by
  reading only: its span walk is correct for a share that leaves and comes
  back.
- **The 2026-11-08 Muhurat.** No window is recorded for it, so only its
  classification (open, length unmeasured) is asserted.
- **macOS durability (`F_FULLFSYNC`).** This is a Linux container.
- **Real vendor bytes.** Excluded by instruction. The fixture shapes follow
  `docs/08-vendor-samples.md` and the parsers' own constants.

## Checks run here

All of these ran as uid 0 on Linux, on `cloud/extremes-20261110`.

- **New tests.** `cargo test` on the eight files: 56 passed, with 12 ignored.
  Each finding was run with `--include-ignored` and fails as described.
- **`cargo fmt`.** rustfmt was run on every new file.
- **`cargo clippy -p store -p pull -p core --tests --locked -- -D warnings`:**
  clean.
- **CI gate 1d** (segment-shaped literals in `crates/pull`), run locally from
  `.github/gates_tree.rs`: OK. Gate 0's spawn scan: OK; every child process is
  this test binary re-run.
- **Tests that walk the source tree** passed with the new files present:
  - `vocab`: `no_py_language`, `no_runtime_spawn`, `stale_claims`, `no_float`;
  - `store`: `open_flags`, `libm_key`;
  - `core`: `decision_citations`, `cost_invariants`;
  - `engine`: `production_callers`;
  - `pull`: `token_identity`;
  - `gdfl_cm`'s pasted-row scan, `fixtures_are_built_not_pasted`.
- **The full suites of the touched crates**
  (`cargo test --locked --no-fail-fast -p store -p core -p pull`):
  - 93 test binaries: 3,988 passed, 74 ignored, 14 failed.
  - **None of the 14 is in a file this branch touches.** This branch adds only
    files under `tests/`, so the two failing library test binaries are
    byte-for-byte the base's.
  - **All 14 fail only as root.** Each passed when re-run alone as uid 65534
    (`runuser -u nobody`). They are permission checks that root bypasses, the
    class D-0995's `where_permission_binds` exists for. Noted here for the
    local lanes; they are not data findings.
  - The nine in `pull` (library tests):
    - `alerts::tests::create_fresh_returns_a_refusal_that_is_not_already_exists_as_the_host_gave_it`
    - `gdfl_dropbox::tests::repair_r1::a_journal_that_cannot_be_written_stops_the_press_before_any_byte`
    - `gdfl_dropbox::tests::repair_r1::a_read_only_store_root_is_not_writable_not_lost`
    - `gdfl_import::tests::ia16::an_immutable_month_stops_the_run_by_name`
    - `gdfl_run::ia2_tests::every_id_the_runs_ledger_gives_is_written_down_twice`
    - `ingest::census_store::link_tests::a_read_only_partition_directory_is_refused_with_the_host_s_words`
    - `ingest::census_store::link_tests::an_idempotent_requirement_opens_nothing_for_writing`
    - `zerodha_verify::tests::a_refused_open_keeps_the_hosts_words`
    - `zerodha_verify::tests::a_write_beside_a_merge_that_fails_is_named`
  - The five in `store` (library tests):
    - `quarantine::tests::an_index_publish_refused_after_a_whole_line_is_finished_by_the_next_apply`
    - `quarantine::tests::an_index_publish_refused_after_a_dropped_line_is_finished_by_the_next_apply`
    - `settings::tests::a_directory_that_stops_being_readable_is_a_fault_not_nothing_new`
    - `settings::tests::a_disk_that_refuses_the_write_is_refused_with_nothing_visible`
    - `settings::tests::a_landed_save_whose_read_back_cannot_look_is_saved_with_the_reason`
  - Every integration test binary of the three crates passed, the eight new
    ones included.
- **Not run here:** `cargo deny`, line and branch coverage, mutation testing
  and the other crates' full suites. Those tools are not installed in this
  container: UNVERIFIED.
