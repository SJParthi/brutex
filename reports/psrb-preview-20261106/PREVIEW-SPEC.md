# PREVIEW-SPEC: one fixed PSRB backtest across the F&O stocks, every timeframe (a PREVIEW outside the main build)

Written Fri 6 Nov 2026 ~09:xx IST by a workflow step of main session 641cf96a, for the builder and the checker of the
preview. The operator asked (Fri 6 Nov ~08:4x IST) for "one backtest strategy check of just one fixed number", the PSRB
strategy, fixed, across the entire F&O stocks, every timeframe, now. This is a PREVIEW: the real PSRB result still
comes from the checked PSRB parts (PSRB-STRAT is not built), and data parts are still being fixed. Nothing here feeds
any other run. Nothing was built, run, committed or written outside this folder in this step; the store was read only
(listings, and the first/last record of some months read by a node script opened read-only).

**Short names.** W = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`; S = `W/state/strategy-psrb-20261103`;
AD = `S/PARTS-STRAT-ADDENDUM-20261105.md`; PP = `S/PARTS.md`; PWC = `S/PARTS-WILLR-CCI.md`; OI =
`S/operator-inputs-20261103.md`; SS = `S/psrb-search-space-20261105.html`; WU = `S/psrb-strategy-v1.html`; COSTS =
`W/state/fib-strategy-20261030/COSTS.md`; CT = `W/state/fib-strategy-20261030/costs-table.json`; ENG =
`W/state/fib-strategy-20261030/ENGINE.md`; FSPEC = `W/state/fib-strategy-20261030/SPEC.md`; FPARTS =
`W/state/fib-strategy-20261030/PARTS.md`; STORE = `/Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store` (READ ONLY).
Repository files are cited at **B = `c8f7608b5cbc5383f294728e4eeb0d1208a31e97`** (FNO-COSTS's passed tip, a descendant
of `1ad6e24ef8e0`; `check-FNO-COSTS-r2.md`:5, :21) unless a PSRB tip is named: RSI `6d3ae223e5db`, BOLL `820f63a7a0e2`,
STV `7b137fee7014`, WILLR `cb2db489bc9d`, CCI `1222c57a7253`.

---

## 0. The recipe in brief (one value each; AD's defaults, no search)

1. **Rule sets:** VIDEO-ORIGINAL = R1/S1 + Supertrend 10/3 + Bollinger 20/2 + RSI 14 at 70/30 (AD:119, :55-56).
   AUTHOR-V2 = narrow CPR (CPR-20BP) + R1/S1 + Supertrend 10/2 + Bollinger 20/3 + RSI 14 at 70/30 + Williams %R 14
   LEVEL (above -85 / below -15) + CCI 20 at +/-200 + EMA 200 (AD:56-58, :85-94, :119; OI:16, :19). Every condition on
   the same closed candle, long and short mirrored. Stop and exit for both: the Supertrend side band as a trailing
   resting stop plus the flip exit, no entry order from 15:08:00, time exit order at 15:07:59, broker square-off from
   15:10:00 (AD:62-66, :304-357).
2. **Timeframes:** 1, 3, 5, 15, 30 and 60 minutes, each its own run (SS:348).
3. **Window:** September 2018 (first session 2018-09-03) to October 2026 (last session 2026-10-30): 98 months, every
   one judged (nothing is learned). Warm-up: every stored Zerodha minute before a stock's trading start (Jan-Aug 2018,
   167 sessions, for the names with minutes from January 2018).
4. **Candles:** Zerodha 1-minute bars folded by the store's own fold, `pull::fold::derive_minutes` under the rule in
   force (`PADDED_FOLD = Absent`), on the 09:15-anchored grid, session 09:15-15:30 IST (09:15-15:15 for a
   closing-auction-eligible stock-day from 2026-08-03).
5. **Fills:** signal on the candle close T; entry at the first GDFL traded second at or after T, at its worst print
   (buy at its high, sell at its low); stop, flip, time and square-off exits the same way at the second they fire; a
   stock-day whose GDFL seconds cannot be read fills on the next Zerodha minute's high (buy) or low (sell), counted
   and labelled. N4's quote range is not loaded.
6. **Sizing:** account Rs 20,00,000; 20 slots; each trade Rs 1,00,000 margin x 5 = Rs 5,00,000 exposure, quantity =
   floor(5,00,000 / entry fill price); one position per stock; a signal when 20 slots are busy is skipped; ties by
   time, then symbol; slot size never compounds. Three accounts per (rule set, timeframe): LONG alone, SHORT alone,
   BOTH (long and short sharing the 20 slots).
7. **Costs:** every charge of the cost table, per trade, by `costs::equity_clock::price_trade` (both clocks, worst
   readings, headline = max), at B.
8. **Month net %** = month net P&L / account capital at that month's start, integer basis points.
9. **Max drawdown** = the largest fall of account equity from a peak to a later low, equity marked after every trade
   close in time order and at every day's end, in basis points of the peak.
10. **Verdict:** a row passes iff all 98 months are >= +2.5% net (OI:28); else its months below +2.5% with each gap,
    and the highest monthly floor it holds (its worst month). Passers rank by lowest max drawdown first (OI:29, :31).

36 rows: 2 rule sets x 6 timeframes x 3 accounts.

---

## 1. What was read (file:line cited where used below)

AD whole (684 lines); PP sections 0-2 (:25-337) and 6 (:972-1031); PWC 5.3 (:548-618) and 6.3 (:831-884); OI items
1-18 (:3-31); SS (:344-359, the timeframes); WU (:232, the warm-up); COSTS sections 1-9 (:11-200); CT `meta`,
`rows`, `readings` (:1-740); `W/state/fno-data-audit-20261030/universe.json` and `universe.md` (`source` field);
`W/state/fib-strategy-20261030/inputs/fno218.txt` (218 lines); `W/state/scope-fno-stocks-20261030/` (classification
and lists files: scope of parts, not a stock list); ENG sections 0, 2, 3, 4, 5; FSPEC 4 (:239-249, the day mask) and
6.6 (:430-446); FPARTS (:373-399, the session clock; :1303-1304, N4); `W/state/data-build-20261031/PARTS.md` and the
specs of REFOLD, REPLACE, SIDX-1, ZBASIS-1 (heads); `W/state/fib-small-20261024/` (REPORT.md, pit-build.log,
pit-membership/); `W/state/fno-data-audit-20261030/report-evidence/backtest-skip.tsv`; the repository at B with
`git show` and `git grep` only.

Measured read-only in this step (node, `fs.openSync(..., 'r')`, first and last 56-byte record of a month; no lock
taken, nothing written):
- Zerodha 1-minute: RELIANCE 2018-01 first 2018-01-01 09:15, 2018-09 first 2018-09-03 09:15, 2026-10 last 2026-10-30
  15:14 (7,200 records), 2026-11 holds only 2026-11-02 (360 records). Jan-Aug 2018: 62,625 minutes on 167 sessions.
- GDFL 1-second: RELIANCE 2018-09 first 2018-09-03 09:15:01, last 2018-09-28 15:29:59; 2026-10 last 2026-10-30
  15:14:59; no 2026-11 month for any symbol.
- Over the 218 names (ICICILIFE read from the stored ICICIPRULI series): both feeds' 2026-10 months end on 2026-10-30
  for 217 names; ICICIPRULI's GDFL ends 2026-10-21 and its Zerodha has no 2026-10 month. First months: Zerodha
  2018-01 for 174 names, GDFL 2018-09 for 178. IRFC (Zerodha 2018-03, GDFL 2021-01), KFINTECH (2018-05, 2022-12) and
  TATACONSUM (2018-01, 2020-02) start apart; IRFC's 2018-03 minutes sit at Rs 995 and KFINTECH's 2018-05 at Rs 11,
  another instrument before a gap of years. GDFL months missing inside the span: ANANDRATHI, DATAPATTNS, ENRIN,
  KARURVYSYA, SYRMA, UJJIVANSFB 2026-08 and 2026-09; FORCEMOT 2023-11 to 2024-01 in both feeds.

---

## 2. How the store holds the bars, and the Rust that reads them

- **Zerodha 1-minute cash bars:** `STORE/bars/zerodha/NSE/CASH/<SYM>/1min/<YYYY-MM>.bin` (+ `.crc`, `.lock`, `.tix`),
  56-byte records of seven little-endian `i64` (`ts_micros` UTC of the minute start, open, high, low, close in paisa,
  volume, open interest) after a 32,768-byte header region (`docs/02-store-format.md` §1, §3 at B). Zerodha's history
  is corporate-action ADJUSTED (`crates/cli/src/tick_second_check.rs`:82 at B). Vendor-padded minutes exist (flat,
  volume 0; INT-15, R4-27). The stored derived rungs (2/3/5/10/15/30/60min, 1day) are NOT read: many derived months
  are still under the fold before D-7270 (REFOLD, `spec-REFOLD.md` §1.1).
- **GDFL 1-second cash bars:** `STORE/bars/gdfl/NSE/CASH/<SYM>/1s/<YYYY-MM>.bin` (+ `.tix` time index, `.crc`, `.lock`,
  `.auc` closing-auction prints kept out of the bars from 2026-08-03), same record. A second exists only if it traded
  (Rule T, `tick_second_check.rs`:51-54): high and low are the largest and smallest traded price. GDFL is RAW
  (unadjusted). A repaired month is served from `bar-revisions-v1/<n>/` through `bar-generations-v1/` pointers (ENG
  §3.1).
- **Readers (at B):**
  - `store::file::BarFile::open_existing(root, StorePath, symbol_id)` (`crates/store/src/file.rs`:2105): read-only, no
    create, shared lock opened read-only (:2185-2199), the served generation resolved (:2110-2134); `Missing`,
    `Locked`, `Generation` refusals. `symbol_id = brutex_core::universe::fnv1a(symbol) as u32`
    (`crates/cli/src/stored.rs`:2740). `StorePath::new(PathParts { vendor, exchange: "NSE", segment: "CASH", symbol,
    contract: None, timeframe: Timeframe::SECOND_1 | MINUTE_1, month: YearMonth::new(y, m), file: FileKind::Bars })`
    (`crates/store/src/path.rs`:897, :358, :410, :583).
  - `BarFile::first_at_or_after(ts)` (file.rs:3553): the first traded second at or after `ts`, through `.tix`, O(1)
    (ENG §3.2-3.3: 2 reads, measured); `BarFile::read_record(i)` (file.rs:3504), O(1); the next traded second is
    `read_record(i + 1)`.
  - `cli::stored::load_span(root, vendor, underlying, "1min", from, to)` (`crates/cli/src/stored.rs`:3083) ->
    `Span { bars: Vec<Candle>, missing, excluded }` (:2910-2935): whole months, oldest first, charter non-regular
    sessions (Muhurat, disaster-recovery) withheld; `load` (:2587) is one month. Never call it on `1s` (ENG §3.2).
  - `pull::fold::derive_minutes(bars, bucket, venue, cash_schedule, runtime)` (`crates/pull/src/fold.rs`:1022): the
    store's own fold of one month of minutes; `PADDED_FOLD = PaddedFold::Absent` (:1077; a padded minute is a minute
    with no trade, a bucket of only padded minutes has no bar, :1032-1060). `runner::resample::resample`
    (`crates/runner/src/resample.rs`:276) is the runner's restatement of the same 09:15 grid (:40-61) but folds a
    padded minute as a trade, so it is not used.
  - `pull::calendar::Runtime::kind_of(day)` (`crates/pull/src/calendar.rs`:4767) = `sourced_kind_of`
    (:1216): the measured window plus the circular-sourced spans to `SOURCED_LAST_DAY` 2026-12-31 (:982-990).
    `pull::calendar::kind_of` (:654) answers `Unmeasured` outside the measured window and is NOT used for the mask.
  - `pull::fno_membership::Membership::open(dir)` (`crates/pull/src/fno_membership.rs`:2126), `is_member(underlying,
    day)` (:2585).
  - `costs::equity_clock::price_trade(&TradeFills, TradeDay) -> TwoClock` (`crates/costs/src/equity_clock.rs`:287;
    `Fill` :87, `Order` :99 with `by_broker`, `TradeFills` :111, `TwoClock::headline` :204, `net` :241); window
    `check_window` 2018-09-03..=2026-10-30 (`crates/costs/src/equity_dated.rs`:957; CT:5-8); version
    `costs-table/2026-10-30/r1` (equity_dated.rs:56).

---

## 3. The decisions

### 3.1 The two rule sets (decision 1)

Every condition is read on the CLOSED signal candle t of the run's timeframe (AD:81). H, L, C = the previous eligible
session's high, low and close (3.4); T3 = H + L + C. Long and short mirrored (AD:130-141). A candle never yields both
sides (AD:128-129).

| id | condition | reading (constructor and call) | long | short | source |
|---|---|---|---|---|---|
| C1 | narrow CPR, **CPR-20BP** | own exact test, `i128` | `10_000 x abs(2C - H - L) <= 20 x T3` (ties narrow) | the same | AD:87, :148-156; OI:19 |
| C2 | close beyond R1 / S1 | own exact test, `i128` | `3 x close_t > 2 x T3 - 3 x L` | `3 x close_t < 2 x T3 - 3 x H` | AD:88, :175-186 |
| C3 | Supertrend side, settled | `DualBandSuperTrend::new(10, 2_000)` (AUTHOR-V2) / `new(10, 3_000)` (VIDEO-ORIGINAL), `reading()`, `settled == true` | `trend == Up` | `trend == Down` | AD:89; STV tip dual_band_supertrend.rs:110-122, :160, :251 |
| C4 | Bollinger band, population sigma | `Bollinger::new(20)`, `window()`, `w.above_upper(Close::new(close_t)?, Width::from_milli(3_000))` (AUTHOR-V2) / `from_milli(2_000)` (VIDEO-ORIGINAL) | `above_upper` | `below_lower` | AD:90; BOLL tip bollinger.rs:193, :221, :268, :333, :402, :410 |
| C5 | RSI 14, faithful only | `Rsi::new(14)`, `reading()`, `faithful()` | `above(Level::from_ppm(700_000))` | `below(Level::from_ppm(300_000))` | AD:91; OI:19; RSI tip rsi.rs:149, :188, :325, :350, :361 |
| C6 | Williams %R 14, **LEVEL** form | `WilliamsR::new(14)`, `reading()`, not `flat()` | `above(Level::from_ppm(-850_000))`: `100 x (HH - C) < 85 x (HH - LL)` | `below(Level::from_ppm(-150_000))`: `100 x (HH - C) > 15 x (HH - LL)` | AD:92; PWC:571-576; WILLR tip williams_r.rs:150, :197, :340, :378, :387 |
| C7 | CCI 20 | `Cci::new(20)`, `reading()`, not `flat()` | `above(Level::from_milli(200_000))`: `n x dev > 3 x D` | `below(Level::from_milli(-200_000))` | AD:93; PWC:846-852; CCI tip cci.rs:165, :206, :315, :337, :344 |
| C8 | EMA 200 on the signal candles, warm | `trend::Ema::new(200)`, `fold(close_t)` every candle, `warm()`, `scaled()` (the getter of AD:197-211, inserted in the scratch tree only) | `i128::from(close_t) * SCALE > ema.scaled()` | `i128::from(close_t) * SCALE < ema.scaled()` | AD:94, :188-239; trend.rs:90, :169, :184, :207, :258 |

- **VIDEO-ORIGINAL** = C2 + C3(10, 3_000) + C4(20, 2_000) + C5 (AD:119).
- **AUTHOR-V2** = C1 (CPR-20BP) + C2 + C3(10, 2_000) + C4(20, 3_000) + C5 + C6 (LEVEL) + C7 + C8 (AD:119; OI:16).
  ONE CPR definition and ONE Williams %R form, as the operator asked for one fixed recipe: CPR-20BP is the middle of
  the three width defaults (0.10%, 0.20%, 0.30%, OI:19; AD:59-60), chosen by position, not by any result; LEVEL is the
  main form, CROSS its listed variant (OI:19 "above -85 plus a crosses variant"). The other AD rows (CPR-10BP,
  CPR-30BP, CPR-NARROW5, CROSS, the 29 ablations, VIDEO-ORIGINAL-FIXED-STOP; AD:388-397) are not run.
- **A reading decides only when ready** (AD:124-127): every reading the rule set uses is warm; C3 settled; C5 faithful;
  C6 and C7 not flat (a flat reading makes its condition false); C8 warm; and no reading is tainted by a hole (3.4).
  Levels and widths are built once per rule set (AD:96-98).
- **The stock has no open position** when the conditions are read; a signal while a position is open is ignored, not
  queued; re-entry after any exit has no cap and needs no fresh green (AD:120-123, :290-299). After a flip exit, C3
  itself waits for the Supertrend to turn back.
- **Folding** (AD:261-266): every reading is folded per timeframe from the stock's first candle, continuously across
  sessions; ST 10/2 and 10/3 are two states; Bollinger widths 2_000 and 3_000 read one `Bollinger::new(20)` state; RSI,
  W%R, CCI and the EMA one state each. A candle that any reading refuses (`Candle::check_evaluable`, lib.rs:1778, or
  `bollinger::Close::new` above 2^32 paisa) is folded into none and is a hole (AD:268-278).

### 3.2 Timeframes (decision 2)

**1, 3, 5, 15, 30 and 60 minutes**, each its own run and its own rows: SS:348 names "3, 5 and 15 minutes, each reported
(your defaults), and also 1, 30 and 60 minutes". Buckets per full session (375 minutes, AD:260): 375, 125, 75, 25, 13
(12 full + the 15:15-15:30 stub), 7 (6 full + the stub); from 2026-08-03 on a closing-auction-eligible stock-day (360
minutes): 360, 120, 72, 24, 12, 6. A stub candle is folded as any other; it closes after 15:08 and so never enters.

### 3.3 Window, universe, warm-up and day mask (decision 3)

- **Window: 2018-09 to 2026-10, 98 calendar months.** First month: 2018-09, the first month in which both Zerodha
  minutes and GDFL seconds exist (GDFL's first month, measured, section 1; FSPEC M4 :242; AD:422-427 "from September
  2018 on"). Last complete month in both feeds: **2026-10**, whose last session 2026-10-30 is the last record of both
  feeds' October months for 217 names; Zerodha's 2026-11 holds only 2026-11-02 and GDFL has no 2026-11. It is also the
  last day the cost table prices (CT:5-8; equity_dated.rs:957).
- **Universe: the 218 names of `fno218.txt`** (`universe.json` `source`: the 29 Oct 2026 GDFL F&O day zip, equal to
  the 6 Oct Zerodha NFO master after the dated changes), ICICILIFE read from the stored ICICIPRULI series. **Dated
  membership exists and is usable:** `W/state/fib-small-20261024/pit-membership/`, 97 files `2018-09-g000001.fnm` to
  `2026-09-g000001.fnm`, built 24 Oct 2026 by `cli fno-membership build` (pit-build.log: unknown days 0, refusal days
  0), source `inferred_gdfl_options` (a share is a member on D when one of its stock-option contracts' lives covers D,
  ENG §4). It is not in the store (`STORE/reference` does not exist). Rule: a stock-day may trade only when
  `is_member(name, D)` is `Ok(true)`; a refusal for a name-day is "membership unknown", not traded, counted per stock;
  for October 2026 (no file) the answer of 2026-09-30 is used. **Probe first:** open the folder with
  `Membership::open` and ask `is_member("RELIANCE", 2018-09-03)` and `is_member("RELIANCE", 2026-09-30)` (expect
  `Ok(true)` both) and `is_member("SYRMA", 2026-09-30)` (expect anything but `Ok(true)`: SYRMA first traded F&O on 28
  Oct 2026, universe.json `source`). If the folder refuses or any answer differs, the run uses today's list on every
  day instead and every output says "survivorship: today's list". Either way the label "survivorship: only today's 218 names can trade; a share that left F&O before 30 Oct
  2026 has no stored series" stands (CT:17).
- **Each stock's trading span** starts at the later of its first Zerodha month and its first GDFL month (never before
  2018-09-03) and ends 2026-10-30. **Warm-up:** each stock's candles start at the first month of the contiguous run of
  Zerodha months that contains its trading start (so IRFC's 2018-03 and KFINTECH's 2018-05 foreign minutes, before
  gaps of years, are never folded); for the 174 names with minutes from 2018-01 that is Jan-Aug 2018, 167 sessions
  (WU:232; AD:261-262): 62,625 one-minute, 20,875 three-minute, 12,525 five-minute, 4,175 fifteen-minute, 2,171
  thirty-minute and 1,169 sixty-minute candles, more than the largest period, EMA 200 (AD:265-266). No entry before a
  stock's trading start; a later-listed stock enters only once every reading is warm.
- **Day mask (a stock-day may trade only if all hold):** (M1) the name is in the 218; (M2)
  `Runtime::default().kind_of(day) == DayKind::Open(Session::full())` (a regular full session; Muhurat
  `OpenLengthUnmeasured` and closed days never trade; FSPEC:240, :247); (M3) the membership rule above; (M4) inside
  the window and the stock's trading span; (M5) the stock has Zerodha candles that day; and the two market-wide skip
  rows of `backtest-skip.tsv` (lines 3-4): **2021-02-24** (NSE outage, the resumed 15:45-16:59 window absent) never
  trades, its stored candles still fold; **2019-05-30** (GDFL vendor files start 11:57:47) is a minute-fallback day for
  every stock (3.5). AD:300's M1-M11 are otherwise not applied (APPROXIMATE).

### 3.4 Candles, holes and the previous session (decision 4)

- **Candles:** per stock, per month, per timeframe k in {1, 3, 5, 15, 30, 60} minutes:
  `pull::fold::derive_minutes(&minutes, Bucket::of_secs(60 x k), Venue::NseCash, Some(&schedule),
  Runtime::default())` (fold.rs:1022; `Bucket::of_secs` :115), where `minutes` are that month's stored bars from
  `load_span` (charter non-regular sessions withheld, stored.rs:2905-2935). The candle's close instant T = bucket start
  + 60 x k seconds, clipped to the session's continuous close. `schedule` (`pull::cash_auction::Schedule::insert`,
  cash_auction.rs:497) carries, for each stock-day from 2026-08-03 (`pull::vendor::cash_auction_eligibility_required`,
  vendor.rs:1443), eligible = true iff that stock-day's last stored minute is 15:14 or earlier, else false (the 15:15 /
  15:30 continuous close, calendar.rs:1256-1260; FPARTS:399). A month the fold refuses (`FoldError`) gives no candles:
  counted, and its scheduled buckets are holes.
- **Hole:** a candle is a HOLE when a scheduled minute of its bucket (09:15 to the day's continuous close, on an M2 day
  of the stock's candle span) has NO stored 1-minute record, or when the fold withheld its day, or when a reading
  refuses it (3.1). A padded or zero-volume minute is not missing: under the store's rule a bucket of only padded
  minutes simply has no bar, "neither written nor withheld" (fold.rs:1056-1059). A scheduled bucket with no bar and a
  missing minute in it is a hole at its place in the stream. **No decision on a hole candle**; each reading is then
  tainted until it has folded its own period of clean candles after the hole: ATR 10 (C3), Bollinger 20, RSI 14,
  Williams %R 14, CCI 20, EMA 200 (AD:124-127, :267; PP:1002-1004). A hole candle that holds bars is still folded (its
  minutes are real trades); a refused candle is folded into none.
- **Previous session (C1, C2):** the day bar of each M2 day is `derive_minutes` at `Bucket::DAY` over that day's minutes
  (session high, low and last traded minute's close), accepted by `indicators::anchored::DailyReference::new(bar,
  DailyEligibility::Eligible)` (anchored.rs:111). C1 and C2 on day D read the last such accepted bar before D; with no
  previous bar, or a refused one, C1 and C2 are false all day and no earlier bar is read (AD:104-118). (Not the stored
  1day rung: ZBASIS-1 shows RELIANCE's 1day ~95 bp off its minutes for 580 days, `spec-ZBASIS-1.md`:22-28.)

### 3.5 Entry, exits and fills (decision 5)

Prices: Zerodha candles and bands are ADJUSTED, GDFL seconds RAW (2). Decisions read Zerodha; fills and the resting
stop's trigger read GDFL.

- **Entry.** At a signal candle's close T (T < 15:08:00 IST, else refused and counted "cut by 15:08", AD:287-289), a
  market order starts at T and fills at `s = first_at_or_after(T)` of that stock-day's GDFL month, at that second's
  worst print: a long buys at `s.high`, a short sells at `s.low` (AD:284-286; the task's rule). No traded second left
  that day: "entry not filled", counted, the stock stays flat. A fill more than 60 s after T is counted "late fill"
  (diagnostic only).
- **The stop, mapped to raw.** At entry the resting stop is the signal candle's side band (long `Reading.lower`, short
  `Reading.upper`, AD:306-309, :312-313). A Zerodha level L_z decided at instant T is mapped to GDFL by FSPEC 6.6's R1
  (:434-437): `L_g = L_z x G / Z`, with Z = close of the last stored Zerodha minute (volume > 0) ending at or before T
  that day, and G = close of the last GDFL traded second inside that same minute (`first_at_or_after(minute end) - 1`,
  same minute); if that minute has no GDFL trade, the latest earlier minute of the session that has both. Rounded
  against the position (FSPEC:442) to the paisa: a long's stop `floor` (`div_euclid`), a short's stop `ceil`, in
  `i128`. No such minute: the candidate is not taken ("no ratio", counted).
- **Trailing (AD:314-317).** After each later candle t closes at T_t <= 15:07:59 with the trade open: if close_t is
  across its OWN side band (long `close_t < Reading.lower` after folding t; short `close_t > Reading.upper`), the trade
  exits by the **flip** (below); else the stop moves to t's side band, mapped at T_t, if tighter (higher for a long,
  lower for a short), never back. A hole candle moves nothing and fires no flip. No move or flip is decided at or after
  15:08:00.
- **Stop fill (AD:318-319).** The stop is live from the first traded second after the entry's fill second; the stop in
  force from instant T_t applies to seconds at or after T_t. A long's stop fills at the first second whose `low <=
  stop_g`, at that `low`; a short's at the first whose `high >= stop_g`, at that `high` (a gap through is taken in
  full).
- **Flip fill (AD:320-335).** At the first traded second at or after T_t, worst print (a long sells at its low, a short
  buys at its high). If that second also reaches the stop: one exit, at that print, counted as a flip.
- **Time exit and square-off (AD:336-346).** A position still open gets an exit order at **15:07:59**, filled at the
  first traded second at or after 15:07:59 and before 15:10:00, worst print ("time"); none: the broker squares off at
  the first traded second at or after **15:10:00**, worst print, `by_broker = true` ("square-off"); none to the session
  end: closed at the worst print since the entry (a long's lowest low, a short's highest high), `by_broker = true`,
  flagged "unclosable" (D-8127's remainder rule, AD:339-346). An entry that fills at or after 15:07:59 goes straight to
  the square-off rule.
- **Minute fallback.** A stock-day is a FALLBACK day when its GDFL month is missing or `open_existing` refuses it, when
  the day has no traded second at all while Zerodha has minutes, or when it is 2019-05-30 (3.3). On a fallback day
  every fill of that day uses Zerodha minutes, with no mapping (one basis): an order starting at instant X fills on the
  first stored minute starting at or after X with volume > 0, a buy at its high, a sell at its low; the stop is live
  from the minute after the entry minute and fills in the first minute whose low reaches a long's stop (high a
  short's), at that minute's low (high); the time order at 15:07:59 fills on the 15:08 minute onward, the square-off on
  the 15:10 minute onward. Each such stock-day with a trade is counted and every trade on it is marked "minute
  fallback". For slot timing a fallback fill's second is its minute's start + 59 s.
- Exits are counted by kind: stop, flip, time, square-off, unclosable.

### 3.6 Sizing and the account (decision 6)

- Account Rs 20,00,000 (200,000,000 paisa) at 2018-09-03; 20 slots; each trade Rs 1,00,000 margin x 5 = Rs 5,00,000
  exposure (50,000,000 paisa): `qty = 50_000_000.div_euclid(entry_fill_paisa)`; qty 0 is "zero quantity", not taken.
  So margin in use <= Rs 20,00,000 and exposure <= Rs 1,00,00,000 at every instant (OI:13; OI:5 item 1 Rs 1 lakh x
  leverage, 20 open). 5x is the task's flat leverage for every stock. Slot size never compounds.
- One position per stock (long or short). A position holds its slot from its signal instant T to its exit fill second
  (inclusive). At instant T, candidate signals are taken in the order (T, symbol byte order of the 218 spelling); a
  candidate is ignored if its stock has a position (counted "signal while open"), else skipped if 20 positions are
  open (counted "slots full"), else admitted.
- Three accounts per (rule set, timeframe): **LONG** (long candidates only), **SHORT** (short only), **BOTH** (both
  sides in one book, sharing the 20 slots and the one-position-per-stock rule). Each starts at Rs 20,00,000.
- Trades are attributed to the IST day and month of their exit fill (always the entry's day).

### 3.7 Costs (decision 7)

Every trade is priced by `costs::equity_clock::price_trade` at B (equity_clock.rs:287): its `TradeFills` are the entry
order and the exit order, one `Fill { second, quantity, price }` each, `by_broker` true on a square-off or unclosable
exit; `TradeDay::new(y, m, d)` of the trade day. Charged per trade, both clocks, each line at its worst reading, the
headline = max(dated worst, today worst) (COSTS:140; CT:15): brokerage min(Rs 20, 0.03%) per executed order (BRK-1,
COSTS:47; per fill = per order here, one fill each, RD-BRK :129), Dhan square-off Rs 20 + GST 18% on top (BRK-SQ :48,
RD-SQ :135), DP Rs 12.50 + GST (BRK-DP :49, RD-DP :134), NSE transaction charge by date (EX-2009 Rs 325/cr to
2020-12-31, EX-2021 Rs 345/cr, EX-2023 Rs 325/cr, EX-2024A Rs 322/cr, EX-2024B Rs 297/cr, EX-2026 Rs 306.99/cr;
:50-56) and IPFT (IP-OLD Rs 0.01/cr, IP-2023 Rs 10/cr, IP-2026 Rs 0.01/cr; :57-59), SEBI fee (SF-2017 Rs 15/cr, SF-2019
Rs 10/cr, SF-2020 Rs 5/cr, SF-2021 Rs 10/cr; :60-63), STT 0.025% of the sell side (STT-ND :64, RD-STT-BASE :130), stamp
(state levy on both sides to 2020-06-30 at the worst listed state, SD-STATE :65, RD-STATE :136; 0.003% of the buy from
2020-07-01, SD-ND :66), GST 18% on brokerage + exchange + IPFT (GST-1 :67) and on the SEBI fee from 2022-07-18
(GST-SEBI :68), rounding at the worst level (RD-ROUND-TAX, RD-ROUND-EX, RD-GST :131-133). Net = gross - headline
(`TwoClock::net`). Slippage is the worst-print fill itself. A refusal of `price_trade` stops the run by name.

### 3.8 Month net % (decision 8)

`month_bp = (10_000 x month_net_paisa).div_euclid(capital_start_paisa)` in `i128`, with capital_start = 200,000,000 +
the net of every trade closed before the month (the base compounds; the slot does not). The +2.5% test is exact:
`10_000 x month_net >= 250 x capital_start`. A month with no trades is 0 bp and fails (OI:28 supersedes OI:24's flat
pass). A month starting at capital <= 0 is "account at or below zero" and fails. All 98 months count for every row.

### 3.9 Max drawdown (decision 9)

Equity starts at 200,000,000 paisa. It is marked after every trade's exit in time order (exit fill second, then symbol)
and at every day's end. Peak = the running maximum (the start included); `max_dd_bp = max over marks of (10_000 x (peak
- equity)).div_euclid(peak)`; also printed in rupees, with the peak's and the low's dates. Positions never span a night,
so the day-end mark equals the last close mark.

### 3.10 Outputs and the verdict (decision 10)

Per row (rule set, timeframe, account LONG / SHORT / BOTH), all money in paisa integers printed as rupees with two
decimals by integer division (no floating point anywhere):

| column | definition |
|---|---|
| months in window | 98 |
| months >= +2.5% | months with `10_000 x net >= 250 x capital_start` |
| months > 0 | months with net > 0 |
| worst / median / best month % | `month_bp`; median = the lower median, the 49th smallest of 98 (AD:453's form) |
| total net Rs and % | sum of trade nets; `(10_000 x total).div_euclid(200,000,000)` bp |
| max drawdown % | 3.9, bp and Rs |
| trades | admitted trades that filled |
| win rate | wins (net > 0) / trades, permille floored (AD:450) |
| average win / average loss | sum of winners' net / wins; sum of losers' net / losses (net < 0); paisa floored; "no wins" / "no losses" |
| profit factor | gross win / gross loss of trade nets, thousandths floored; "no loss" at gross loss 0 (AD:451) |
| total charges | sum of headlines |
| skipped signals (slots full) | 3.6 |
| days with minute-bar fallback fills | stock-days on which a trade filled by 3.5's fallback |
| also | signals, signals while open, cut by 15:08, entry not filled, zero quantity, no ratio, late fills, exits by kind, holes and decisions blocked by taint per reading |

**Verdict:** PASS iff all 98 months are >= +2.5% (OI:28). Else MISS, printing every month below +2.5% with its gap
`250 - month_bp` bp (exact gap `250 x capital_start - 10_000 x net`, shown in paisa x 10_000 / capital), and the
highest monthly floor it holds = its worst month bp (OI:28 "the nearest bar that IS met"). **Ranking** (OI:29, :31):
passers by lowest max drawdown, then most total net, then win rate; then misses by fewest months below +2.5%, then
highest worst month, then lowest max drawdown, then total net. The report's head: per timeframe and account,
VIDEO-ORIGINAL against AUTHOR-V2 (trades, net, max drawdown, months >= +2.5%, worst month) and AUTHOR-V2 over
VIDEO-ORIGINAL in permille for trades, net and max drawdown, a zero or negative base printed as such (AD:481-489); and
the nearest met floor over all 36 rows.

---

## 4. APPROXIMATE (every output file lists these under its PREVIEW line)

1. **Not the PSRB result.** PSRB-STRAT is not built (PP:989-1023); this is a scratch tool. Indicator code copied from
   part tips: PSRB-RSI and PSRB-BOLL passed check 1 (`check1-PSRB-RSI.md`:1, `check1-PSRB-BOLL.md`:1); PSRB-STV,
   PSRB-WILLR and PSRB-CCI are still in check 1. The EMA getter `scaled()` is inserted in the scratch tree only.
   Readings are the published definitions, UNVERIFIED against TradingView's output (AD:664-665).
2. **One recipe:** CPR-20BP and the LEVEL Williams %R form for AUTHOR-V2; no other CPR definition, no CROSS form, no
   ablation, no VIDEO-ORIGINAL-FIXED-STOP, no top-mover selection (AD:388-400).
3. **No walk-forward needed:** nothing is learned, so every month is unseen (AD:422-427). It is not the FNO-WFT run.
4. **N4's quote range is not loaded** (QRANGE-1 not landed; FPARTS:1304): fills and stop triggers use the traded
   range only, where N4 uses the worse of the traded and the quote range.
5. **Fill walk simplified:** the whole quantity fills in the one second at its worst print. No volume-share cap, no 3%
   market protection, no dynamic price band (FNO-BAND), no latency floor (FNO-CLOCK), no MTM 80% square-off
   (FPARTS:373-399; FNO-SEC-EXEC failed check r2, `check-FNO-SEC-EXEC-r2.md`:3). The quantity is sized from the fill
   price itself (a live order sizes from the last price before it).
6. **Leverage flat 5x** for every stock (FNO-LEVERAGE not used); slots stay Rs 1 lakh after losses (margin not checked
   against equity). Ties by (time, symbol), not SIZE-FIXED-20-SEEDED's seeded order (OI:5); no luck spread.
7. **Universe:** dated membership inferred from GDFL options listings for 2018-09..2026-09, 2026-09-30 carried into
   October 2026 (or today's list if the probe fails); survivorship: only today's 218 names can trade. GDFL renamed
   series are not joined (TATACONSUM trades from its own GDFL months, 2020-02).
8. **Price basis:** stops mapped from adjusted Zerodha to raw GDFL by the in-session ratio (FSPEC 6.6 R1, :434-436),
   not the exact factor (R0 refused until R4-26 is live, FSPEC:440), and rounded to the paisa, not to the raw price
   step FSPEC:442 names (no dated tick table is loaded).
9. **Data parts still being fixed:** REFOLD (stored derived rungs under the old fold; not read here), REPLACE (1day
   snapshots; PIIND's 1-minute foreign-level minutes are read as stored), ZBASIS-1 (RELIANCE's minutes on another basis
   for 580 days, VBL's 8 minutes of 2023-06-14, HINDUNILVR; read as stored), SIDX-1 (read-cost sidecars only), and the
   missing months of section 1 (fallback days or holes).
10. **Day mask partial:** M1, M2, M3, M4, M5 and the two market-wide skip rows only; M7's other skip rows, M8
    (trade-for-trade) and the FNO-DAYMASK part are not applied (FSPEC:239-249).
11. **Closing-auction eligibility** from 2026-08-03 is inferred from where each stock-day's stored minutes end, not
    read from the dated NSE master.
12. **Previous-session close** is the last traded minute's close, not NSE's official closing price.
13. **Costs per trade:** each trade is its own contract note; the client-day aggregation is not applied (pre-2020-07-01
    state stamp capped per trade, DP Rs 14.75 per trade, STT and stamp rounded per trade: each at least the
    client-day figure). FNO-COSTS's open F3 (State stamp per-order rounding, up to 50 paise an order before
    2020-07-01) stands (`part-FNO-COSTS.md`:182).
14. **Drawdown** on closed-trade equity; no mark-to-market inside a trade.
15. **Timeframes 1, 30 and 60 minutes** go beyond AD's 3/5/15 defaults (from SS:348).
16. **Stop live** from the second after the entry's fill second (the order goes in after the fill).

---

## 5. Build and run (for the next step)

- **Tree:** `git -C /Volumes/WD_BLACK/brutex/fresh-20260919/project worktree add --detach
  /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/scratch/psrb-preview-1 c8f7608b5cbc5383f294728e4eeb0d1208a31e97`
  (the next free n if that path exists; none exists now). Nothing is committed; no branch or ref moves.
- **Working-copy additions in that tree only:** `git show <tip>:crates/indicators/src/<m>.rs` and `<m>_tests.rs` into
  the tree for m = rsi (RSI tip), bollinger (BOLL), dual_band_supertrend (STV), williams_r (WILLR), cci (CCI); their
  five `pub mod` lines in `crates/indicators/src/lib.rs`; AD:197-211's `scaled()` after `Ema::value` (trend.rs:242 at
  B, unchanged from 1ad6e24ef8e0); a new binary crate `crates/psrb_preview` added to the root `Cargo.toml` members,
  depending on `core` (as `brutex_core`), `store`, `pull`, `cli`, `indicators`, `costs`, `rayon`, under the workspace
  lints (no float, no `as` casts, no indexing).
- **Build:** `PATH=/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/bin:$PATH cargo build --release --locked -p
  psrb_preview --target-dir /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/tgt/psrb-preview-1` (the queue's
  `*psrb-preview*` priority line, `W/state/cargo-priority.txt`:37); nohup, a log, waits of at most 270 s.
- **Run:** `--store STORE --membership W/state/fib-small-20261024/pit-membership --universe
  W/state/fib-strategy-20261030/inputs/fno218.txt --out W/state/psrb-preview-20261106/out-1`. Opens every store
  file read-only through `open_existing` / `load_span`; installs no telemetry sink under `runtime/`; writes only
  under `--out` (a new folder; never deletes a file). Disk floor checked before writing: free KiB - 52,428,800 >=
  1,074,218,750 (free 2,768,343,852 KiB at 09:xx IST).
- **Phase A (per stock, in parallel):** load the minutes; fold the six timeframes and the day bars; fold every reading;
  mark holes and taint; evaluate both rule sets per candle and side; for every signal candle simulate its would-be
  trade (3.5) and price it (3.7): a candidate `(T, symbol, side, entry second/price/qty, exit second/price/kind,
  fallback, gross, headline, net)`. A candidate's path depends only on its own stock's data, never on the portfolio.
- **Phase B (per row):** merge the candidates by (T, symbol); apply one position per stock and the 20 slots (3.6);
  mark equity, months and drawdown (3.8-3.9); write the outputs.
- **Outputs (`out-1/`), line 1 of each = `PREVIEW: not the PSRB result; one fixed recipe outside the main build;
  approximate as listed below; nothing here feeds any other run`, then the APPROXIMATE list:** `REPORT.md` (the head,
  the ranked 36 rows with verdicts and gaps, the nearest met floor), `rows.tsv`, `months.tsv` (36 x 98: net, bp,
  capital at start, trades, pass), `trades.tsv` (every admitted trade with its fills, kind, charges, net, fallback
  mark), `counts.tsv` (every counter of 3.10), `identity.txt` (B, the five tips, the cost table version, the membership
  folder's `file_digest_hex` or "today's list", the command line).
- **Self-checks the tool runs before Phase A and prints:** `price_trade` on COSTS CN01 (400 at Rs 1,250, sold Rs
  1,262.50, 2018-09-03) gives dated official 313.25 and today official 225.79 (equity_clock.rs:273-284); the membership
  probe (3.3); the CPR tie of AD:536-543 (narrow at 10 bp, not at 9); the EMA fixture of AD:572-578 (199 closes of 101
  then 100: short true, long false); one month of RELIANCE 3-minute candles from `derive_minutes` has 125 buckets per
  full session; on any failure the run stops by name.
