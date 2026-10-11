# 00 — Charter

Scope, verified external facts, and the prohibitions. Written as prohibitions
because a prohibition survives a rewrite better than a goal does.

---

## 1. Scope lock

**The engine sweeps two shapes, both on NSE.** Narrowed from three
instruments by D-0017 (`BSE-SENSEX` is no longer swept and BSE is no longer
pulled), then widened by D-0506 to the cash equities of the F&O universe.

| Symbol | Exchange | Segment | Since |
|---|---|---|---|
| `NSE-NIFTY` | NSE | INDEX | — |
| `NSE-BANKNIFTY` | NSE | INDEX | — |
| the 208 F&O underlyings that are shares, e.g. `NSE-HINDALCO` | NSE | CASH | D-0506, D-0682 |

The third row is not a list here and must not become one: the names are
`core::universe::FNO_UNDERLYINGS`, derived from NSE's own F&O list with an
ISIN beside each, and `InstrumentKey::is_sweepable` probes that index in O(1).
The five F&O underlyings that are indices, and so carry no ISIN, are
`core::universe::FNO_INDEX_UNDERLYINGS` and are refused as cash: an index has no
cash equity (D-0682). NIFTY and BANKNIFTY are swept as the indices above.
The cash equity is the stock's own price series — what the spot level is for an
index. It does not expire.

`NSE-INDIAVIX` is **reference only** — stored, stamped onto observable trades
as `vix_at_entry` / `vix_at_exit`, and never in the condition vocabulary, the
ranking inputs, or run identity.

The store may hold futures, options and single-stock series. Nothing outside
the two symbols above is ever swept.

Widening this list requires an entry in `docs/05-decisions.md`. It does not
happen because a task seemed to need it.

---

## 2. Prohibitions

1. No language other than Rust, in any form, at any layer. See `CLAUDE.md` §2.
2. No writable memory mapping of a store file. Writes go through positional
   syscalls. A writable mapping raises SIGBUS on a full disk and a signal
   cannot be caught in any language.
3. No depth parameter on the sweep. Depth ends at extinction.
4. No ORM, no query planner, no dynamic schema. The path is the index.
5. No float in a price, a cost, or a P&L. Paisa integers only.
6. No fallback that hides the reason it fired. Degrade loudly and name it, or
   refuse. Never silently.
7. No condition bit renumbered or reused. Append only.
8. No token minted by this repository. Credentials are read-only.
9. No claim of a measurement that was not taken.
10. No literal credential path in a tracked file. This repository is public.
    Documents carry the shape `/<org>/<env>/<vendor>/<field>`; the real
    segments are resolved at runtime from an untracked local configuration.
    Enforced by CI gate 1c, not by review. See D-0013.

---

## 3. Verified market facts

Sources are Indian exchange publications and the vendor documentation cited in
§4. Every row here is load-bearing; changing one changes results.

| Fact | Value |
|---|---|
| Regular session | 09:15 – 15:30 IST, NSE and BSE equity |
| Pre-open auction | 09:00 – 09:15 IST — excluded from bars |
| First index tick | 09:15:00 IST |
| Timezone | IST, fixed +05:30, no daylight saving |
| Bars per regular session | 375 at 1-minute granularity |
| Muhurat (Diwali) session | ~1 hour, an evening session on a date that is
  otherwise a holiday. Verified dates: 2020-11-14, 2021-11-04, 2022-10-24,
  2023-11-12, 2024-11-01, 2025-10-21 |
| Disaster-recovery sessions | NSE runs a **live-trading DR drill on a
  Saturday**, out of the secondary site, to prove the site works. Two are in
  the store: **2024-03-02 (Sat)** and **2024-05-18 (Sat)**, **105 bars each**,
  09:15–09:59 then 11:30–12:29 with a 90-minute closure between. MEASURED by
  decoding `zerodha/NSE/INDEX/NIFTY/1min/2024-03.bin` and `2024-05.bin`; also
  recorded in `crates/pull/src/calendar.rs`. **A drill is not a market day**
  and its OHLC never becomes the previous-day anchor — it is in
  `CHARTER_NON_REGULAR_IST_DAYS` for exactly the reason the Muhurats are.
  Both land squarely inside the pull's 09:15–15:30 window, so the pull keeps
  them, as it keeps any in-hours bar of a Muhurat day (cloud audit GAP12-10
  reported 43 bars dated 2021-11-04, 14:47–15:29, in one vendor's store; D-1442).
  Only this list keeps them out of the anchor. What this
  cost while they were absent from that list, measured on the store's own
  bars: for Monday 2024-03-04 the anchor was the 105-bar Saturday rather than
  the 375-bar Friday, moving the pivot **144.5 index points** and the CPR width
  **6.2×** — enough alone to flip `cpr_class`, which is a vocabulary position.
  See `docs/05-decisions.md` and `docs/11-findings.md`. |
| Special weekend sessions | Union Budget sessions falling on a weekend, run at
  full regular hours. **2020-02-01 (Sat), 2025-02-01 (Sat), 2026-02-01 (Sun)**
  — 375 bars each, confirmed in the lake and re-MEASURED by decoding
  `zerodha/NSE/INDEX/NIFTY/1min/2020-02.bin`, `2025-02.bin` and `2026-02.bin`:
  in each the session's first record is stamped 09:15, its 375th 15:29, and its
  376th is the following trading day. This is the complete set OF BUDGET
  weekend sessions: 1 February fell on a weekend in no other year from 2020 to
  date. Supersedes an earlier claim of 2021-01-30 and 2021-02-01; see D-0014. |
| 2024-01-20 (Sat) | A **full regular session on a Saturday**, and it is neither
  a Budget day nor a disaster-recovery drill. MEASURED by decoding
  `zerodha/NSE/INDEX/NIFTY/1min/2024-01.bin`: the month holds 8,250 minute bars
  = 22 × 375, record 5,625 is stamped 09:15 on 2024-01-20, record 5,999 is
  15:29, and record 6,000 is 2024-01-23 09:15. **375 bars, 09:15–15:29.**
  **WHY the exchange traded that Saturday is UNVERIFIED** — no circular is
  cited here and none is guessed; what is recorded is the shape on disk.
  Because it is a full regular session it correctly REMAINS an ordinary
  previous-day anchor and is deliberately **not** in
  `CHARTER_NON_REGULAR_IST_DAYS`: what disqualifies a day from that list is that
  its OHLC does not describe a regular session, and this one's does. It was
  absent from this table while the calendar bitset already held it, which made
  the derived count below wrong; that is a documentation defect and was never an
  engine one. |
| Weekend sessions on disk | **Six, totalling 1,710 one-minute bars**, all
  MEASURED in the operator's own store rather than counted from this table:
  three Budget sessions at 375 each (1,125), two disaster-recovery Saturdays at
  105 each (210), and 2024-01-20 at 375. Against **618,296** `1min` NIFTY bars
  — the `zerodha` feed's 81 month files, counted as `(length − 32768) / 56` —
  that is **0.28%** of the series. The same denominator puts the superseded
  1,335 at 0.216%, which is where the previously stated 0.21% came from; the
  denominator was right and the numerator was not. The earlier figure of five
  sessions and 1,335 bars omitted 2024-01-20 entirely. NSE trades on a
  weekend more often than "weekday" implies, and `indicators::weekday_bit`
  therefore leaves all five weekday positions false on every one of these bars —
  honest under `docs/03-vocabulary.md` §4, and not free: those bars sit in a
  weekday-conditioned candidate's support DENOMINATOR and never in its
  numerator. |
| 2021-02-24 NSE outage | NSE halted all segments from **11:40 IST**, ran a
  15-minute pre-open from **15:30**, resumed normal trading at **15:45**, and
  closed the extended session at **17:00**. Therefore the equity normal-market
  windows are **09:15–11:39** and **15:45–16:59**, totalling **220 minute
  slots**. This does **not** prove that 220 spot-index bars were published: the
  same order records NIFTY computation unavailable from 10:06 to 11:43 and
  does not give one exact availability window for NIFTY, BANKNIFTY and VIX.
  Their shared 54-bar prefixes ending 10:08 cannot define their own denominator;
  a generic index completeness audit marks the day unmeasured rather than
  inventing either a clean session or a vendor-loss count. Primary source:
  [SEBI Settlement Order SO/AB/EFD2/2023-24/6580](https://www.sebi.gov.in/sebi_data/attachdocs/jun-2023/1687270559560.pdf),
  paragraphs 1–2; contemporaneous corroboration:
  [SEBI PR No. 9/2021](https://www.sebi.gov.in/sebi_data/attachdocs/feb-2021/1614256948318.pdf).
  **What is on disk is 54 bars, 09:15–10:08, and it is IST day 18,682.**
  MEASURED by decoding `zerodha/NSE/INDEX/NIFTY/1min/2021-02.bin`: `n_valid` is
  7,179, which is 19 × 375 + 54; record 6,375 is stamped 09:15, record 6,428 is
  10:08, and record 6,429 is 2021-02-25 09:15. The 15:45–17:00 reopening is
  entirely outside the pull's `[09:15, 15:30)` window, so ingest drops it — and
  correctly: re-pulling can never recover a bar the window excludes.
  **That 54-minute stub is therefore in `CHARTER_NON_REGULAR_IST_DAYS`, for the
  identical reason the two DR Saturdays are.** It is a SHORTER session than the
  Muhurat hour the list already refuses, so treating it as regular was strictly
  the larger error: without the entry it became the previous-day anchor for
  2021-02-25, moved the whole 44-position pivot ladder and both previous-day
  Fibonacci ladders, and stayed inside `Prev5` for five sessions. Note the
  distinction the list turns on — a Muhurat and a DR drill are days the exchange
  never meant to be normal, this is a day it did. What disqualifies a session is
  that its OHLC does not describe a regular day, never why it does not. |
| Bar timestamp | the **OPEN** (left edge) of its minute. A bar covers the
  half-open window `[t, t + tf)`, left-closed and left-labelled. VERIFIED |
| Last regular 1-minute bar | **15:29:00 IST**, not 15:30. 09:15 through 15:29
  inclusive is exactly 375 bars, which is the arithmetic that closes it. |
| Forced exit | **Fixed 15:10 IST product policy for every swept intraday
  execution, not an exchange-closing-time fact.** Execution bars are stored at
  their left edge, so the unique accepted `1min` bar stamped **15:09** covers
  `[15:09, 15:10)` and is the only stored OHLCV record that may price the
  forced fill. The 15:10 bar contains post-deadline prices and may not be used.
  A missing, refused, duplicated or otherwise ambiguous 15:09 record has no
  substitute: no 15:08/15:10 neighbour, delayed next-available minute,
  interpolation or fabricated tick. A non-regular session without that exact
  record cannot price a hold that needs forced liquidation, although an
  earlier exact horizon may still be measured. D-0436 deliberately supersedes
  the former dynamic `session_close − 10 min` / 15:20 policy. |
| Tick grid | 2 decimal places |
| Price storage | paisa integers, `i64` |
| Track-1 brokerage | none. Spot indices are not tradable; the sweep is
  signal-only. Cost modelling belongs to the options translation layer. |

**Muhurat is not a normal session.** A previous-day anchor must never roll
across it — the one-hour evening OHLC is not the prior regular day. This was a
real defect that silently poisoned six years of daily anchors.

Stated exactly, because "never roll across it" is not implementable:

> For a bar on any day after a Muhurat session, the previous-day anchor is the
> OHLC of the **last regular trading session strictly before the Muhurat
> date**. The Muhurat day's own OHLC never enters the previous-day anchor, the
> multi-day rolling history, or the previous-session edge.

Worked case: the anchor for 2024-11-04 (Mon) is **2024-10-31 (Thu)** — not
2024-11-01, which was the Muhurat Friday. The mechanism is that every Muhurat
date is also a non-trading date, so an anchor walk restricted to trading days
skips it structurally rather than by a special case that can be forgotten.

Muhurat sessions, confirmed bar-for-bar against the lake:

| Date | Session (IST) | Bars |
|---|---|---|
| 2018-11-07 (Wed) | 17:30 – 18:30 | 60 — MEASURED from GDFL ticks (below); the date is NSE/CMTR/36475's |
| 2019-10-27 (Sun) | 18:15 – 19:15 | 60 — MEASURED from GDFL ticks (below); the date is NSE/CMTR/39612's |
| 2020-11-14 (Sat) | 18:15 – 19:15 | 60 |
| 2021-11-04 (Thu) | 18:15 – 19:15 | 60 |
| 2022-10-24 (Mon) | 18:15 – 19:15 | 60 |
| 2023-11-12 (Sun) | 18:15 – 19:15 | 60 — MEASURED from GDFL ticks (below). Supersedes 18:00 – 19:00 and 46 bars, a row that marked its own open UNVERIFIED (first bar 18:07) |
| 2024-11-01 (Fri) | 18:00 – 19:00 | 60 |
| 2025-10-21 (Tue) | 13:45 – 14:45 | 60 — an afternoon session, not an evening one |

**The tick measurement, 9 Oct 2026 (D-3319).** Read only from the vendor's
GDFL capital-market day zips (`STOCKS/<yyyy>/<MON_yyyy>/GFDLCM_STOCK_TICK_<ddmmyyyy>.zip`
and the `INDICES` twin) in the vendor archive named under "GDFL
capital-market tick files" below, with `unzip -p` and `awk`, counts and minute
spans only; no row is reproduced. A trade is a row whose `LTQ` is positive. On
every date in the table the minutes in which each of RELIANCE, INFY, HDFCBANK,
SBIN, TCS and ICICIBANK trades at least three times run unbroken for sixty
minutes, from the window's open to its last minute (17:30–18:29 in 2018,
18:15–19:14 in 2019 to 2023, 18:00–18:59 in 2024, 13:45–14:44 in 2025), and
NIFTY 50's value changes in every one of those minutes. Before the window a
share trades at most once, and NIFTY 50 prints its first value at the
first-row stamp under "GDFL capital-market tick files" below and does not
change again until the window opens (in 2024 it also changes in the 17:59
minute). After the window no minute holds three trades of any of the six
until the post-close session, ten or more minutes later (18:40 in 2018, 19:25
or 19:26 in 2019 to 2023, 19:19 in 2024, 14:55 in 2025). NIFTY 50 also changes in the minute after
the window's last in 2019 to 2023 (19:15) and in the two after it in 2025
(14:45–14:46), and in one isolated later minute in 2018 to 2024 (18:31, 19:17
or 19:18, and 19:10). The same walk gives 09:15–09:59 and 11:30–12:29 for
all six on both drill Saturdays and 09:15–15:29 for RELIANCE and INFY on
2024-01-20, as measured above from Zerodha. No exchange notice of these timings is recorded
here.

2026 has an announced Muhurat date (2026-11-08) with **no notified timings**.
It is therefore absent rather than guessed.

---

## 4. Vendor facts

Evidence lane is recorded per row and is never promoted while copying.

**Corporate-action adjustment is MEASURED for Zerodha and GDFL, and
UNVERIFIED for every other feed.** Zerodha's share prices are back-adjusted by
the vendor as of the day each month was pulled, with volume on a basis of its
own; GDFL's are as traded (§4z, §4g, D-3323). This paragraph said every feed
was UNVERIFIED until 7 Oct 2026 (D-3348). Groww, Dhan and TrueData (which has a
descriptor in `pull::vendor` and no section here) stay UNVERIFIED; the
operator excluded all three from use on 6 Oct 2026 (D-3338). The store keeps
the vendor's prices as served (`docs/02-store-format.md` §3); the basis is a
descriptor fact (D-3323), and the day each month was pulled is its
provenance (D-3334). numeric-pass16 p16num-2, D-1969.

### Groww — primary

| Fact | Value | Lane |
|---|---|---|
| Transport | official SDK, injected client | verified |
| History endpoint | one method for spot and derivatives | verified |
| Granularity fetched | `1minute` only. Every other timeframe is derived — **at the write boundary**, by `pull::fold`, into the rung the bars are filed under. See D-0055. | decided |
| History depth, daily | **2020-01-01, and the vendor claims more.** This row read "from 2020 / documented" and the lane was wrong: no vendor page says 2020. The operator stated it on **11 Aug 2026** and again on **12 Aug 2026**, and the vendor's own interval table gives its `1 day` row **"Full history"** (`Groww Docs / 08-historical-data.md`). Two sources, disagreeing. The operator's is both the LATER day and the higher-standing claim, so it binds on either rule and the vendor's is carried beside it — `pull::vendor::Descriptor::history`, D-0113, D-0131. | operator-stated 11 Aug 2026, restated 12 Aug 2026, contested by vendor docs |
| History depth, 1-minute | **2020-01-01 — the same day as the daily rung, and this row was REVERSED on 12 Aug 2026.** It read "a rolling 3 months — NOT 2020", binding the vendor's published `1 min` row **"Last 3 months"** (`Groww Docs / 08-historical-data.md`) on the rule that the stricter claim wins. The operator restated the floor on **12 Aug 2026**, having watched this build refuse January 2020 through May 2026 on his own account: *"GROWW — data is available from JANUARY 2020. A fixed floor, not a rolling one."* That is a report of what his entitlement answers, not a competing reading of what Groww published, so it outranks rather than merely out-stricts — see `pull::vendor::ClaimStanding`. The vendor's quarter is carried beside it as the contested claim. **UNVERIFIED:** he stated the figure against the vendor, not against a rung, so it is applied to both rungs unchanged; whether this rung truly reaches 2020 is unmeasured, and no request was made to find out. D-0113, D-0131. | operator-stated 12 Aug 2026, contested by vendor docs; the per-rung split UNVERIFIED |
| Window cap, 1-minute | 30 days per request at 1-minute granularity | documented |
| Window cap, 1-minute — the vendor's own table says 7 | **UNRESOLVED, and both figures are written down.** The 30 above is what this repository has carried and is what `pull::vendor::HttpSpec::window_caps` encodes. `Groww Docs / 08-historical-data.md`'s interval table gives the `1 min` row a **"Max Duration per Request" of 7 days**, and its `1 day` row 1,080. Nothing was changed on the strength of this reading: the 30 is operator-facing history and a narrower cap only costs requests, while a wrong one loses bars. Named here so it is not discovered a third time. | conflicting sources |
| Window cap, daily | **180 days per request, read from the vendor's own backtesting limits table, and a second vendor table disagrees.** `Groww Docs / 11-backtesting.md`, "Backtesting Data Limits", gives `1 hour, 4 hours, 1 day, 1 week, ...` **180 days**; `pull::vendor::HttpSpec::window_caps` encodes `(Day1, 180)` and cites that table. `Groww Docs / 08-historical-data.md`'s interval table gives its `1 day` row 1,080 (row above). The narrower 180 is the one encoded: a narrower cap only costs requests, a wider wrong one loses bars. This row read "UNVERIFIED ... no day-level figure is published in any source this repository has read ... encoded as absent" while the descriptor already carried 180 (W1-pull4-2 left it for a charter edit); corrected by tests-docs-security-pass17 P17-05, D-1963. The vendor page was not re-read in that change; the citation is the descriptor's. | documented, two vendor tables disagreeing |
| Daily interval word | **`1day`.** The vendor's own annexure, *Candle Interval*, gives `GrowwAPI.CANDLE_INTERVAL_DAY` the value **`1day`** — the same table that gives `CANDLE_INTERVAL_MIN_1` the value `1minute` this repository already used. The full table also carries `2minute`…`4hour`, `1week` and `1month`; none is recorded here, because `store::path::Timeframe` has a directory for two rungs and a token for a rung the store cannot file is a request whose answer has nowhere to go. Was UNVERIFIED until the docs were read; D-0076. | verified from vendor annexure |
| Index segment word | **`CASH`** — the same word an equity takes. The vendor's live-data page states it: *"Use the segment value FNO for derivatives and CASH for stocks and index."* Before this was read, `Listing::Index` was absent from the descriptor and a live index pull refused by name with `FetchError::ListingNotSpellable`. D-0076. | verified from vendor docs |
| Instrument-type word | **Not applicable to this request.** The annexure carries an instrument-type alphabet (`EQ`, `IDX`, `FUT`, `CE`, `PE`), but the historical-candles request schema is `exchange`, `segment`, `trading_symbol`, `start_time`, `end_time`, `interval_in_minutes` and nothing else — there is no field for a kind, so no word is written into one. Contrast Dhan, whose request carries `instrument`. | verified from vendor docs |
| Response shape | row arrays: `[ts, o, h, l, c, v, oi]`, `oi` null off-derivatives | verified |
| Timestamp | native IST string, or epoch seconds defensively | verified |
| Price unit | rupees as float on the wire; converted to paisa at the boundary | verified |
| Cash-equity candles adjusted for splits, bonuses or other corporate actions | **UNVERIFIED** for 1min and 1day alike. No page read states whether historical candles are back-adjusted after an action, and none was measured. If they are, months pulled before and after an action sit in the store on different bases, and nothing records the pull date's basis. numeric-pass16 p16num-2, D-1969. | unverified |
| Rate limit | 500 requests per minute. **No daily quota.** | operator-confirmed, not published |
| Per-second cap | **UNVERIFIED.** The published 10/s applies to a different endpoint group. Production ceiling is 8/s, chosen not measured. | unverified |
| Auth | TOTP-derived daily token, reset 06:00 IST | verified |
| Credentials | `/<org>/<env>/<vendor>/<field>` — SecureStrings, region `ap-south-1`, read-only. Fields: `api-key`, `totp-secret`, `access-token`. Real segments resolved at runtime; see D-0013. | verified |

### Dhan — secondary, spot indices only

| Fact | Value | Lane |
|---|---|---|
| Endpoint | intraday charts, 1/5/15/25/60 min — we fetch 1 only | verified from SDK |
| Response shape | **parallel column arrays**, not rows. Unequal lengths reject the chunk. | documented |
| Timestamp | epoch seconds, UTC | verified from SDK |
| Window cap, 1-minute | 90 days per request. Documented against the **intraday charts** endpoint named one row above, so it is recorded at the one-minute rung and not promoted past it. | documented |
| Window cap, daily | **UNVERIFIED.** Whether the 90 above applies to a day-level request is not stated anywhere read; nor is whether `/v2/charts/historical` serves daily at all — the endpoint row calls it "intraday charts" and the descriptor's path string does not say. Encoded as **absent**, which the store's month boundary already bounds: every chunk is ≤ 31 days and therefore inside the 90 regardless. | unverified |
| Daily interval word | **Not applicable — and that is itself the fact.** This vendor's request carries no interval parameter at all (five fields: `securityId`, `exchangeSegment`, `instrument`, `fromDate`, `toDate`), so this build cannot vary the rung on its wire. Bars are folded into whatever rung they are filed under, which is correct for any vendor cadence no coarser than the target. | verified from SDK |
| History depth, daily | **A rolling ~5 years, and the vendor claims more.** Operator, **11 Aug 2026**: a rolling last 5 years — **not a fixed floor**, it moves every day. `Dhan Docs / 12-historical-data.md`, *Get Daily Historical Data*: *"The data for any scrip is available back upto the date of its inception."* Two sources, disagreeing; the operator's is the later day, so it binds and the vendor's is carried beside it. A stated absence of a floor cannot widen a stated one. **Restated unchanged on 12 Aug 2026** — *"DHAN — a ROLLING 5 YEARS"* — in the same breath as the Groww correction, so this row is confirmed rather than merely unrevisited. D-0113, D-0131. | operator-stated 11 Aug 2026, restated 12 Aug 2026, contested by vendor docs |
| History depth, 1-minute | **A rolling 5 years, and here the two sources AGREE.** Same page, *Get Intraday Historical Data*: *"…for last 5 years."* Recorded although this build does not serve the rung — the row is a vendor fact, and `/feeds.json` emits it marked `served: false`. D-0113. | documented |
| `toDate` inclusivity | **NON-INCLUSIVE on the daily endpoint, and NOT STATED on the intraday one.** `Dhan Docs / 12-historical-data.md`: the daily request table describes `toDate` as *"End date (YYYY-MM-DD, non-inclusive)"*; the intraday table one section below describes the same field as *"End date (YYYY-MM-DD)"*, with no qualifier. The expired-options endpoint (`14-expired-options-data.md`) repeats *non-inclusive*. `pull::vendor::HttpSpec::range_end` is per **vendor**, and the served path is the daily one, so the encoded `Exclusive` is right for what is sent today — and it is **UNVERIFIED** for the intraday path, which cannot be enabled without settling it. D-0113. | documented (daily) · unverified (intraday) |
| Rate limit | 5/s, 100,000/day, no per-minute governor | documented |
| Subscription | paid data plan; enforcement surfaces as a specific error code | documented |
| Credentials | `/<org>/<env>/<vendor>/<field>` — read-only. Fields: `client-id`, `access-token`. Real segments resolved at runtime; see D-0013. | verified |
| Security ids | NIFTY = 13 (verified from the SDK's own example). BANKNIFTY 25, SENSEX 51, INDIA VIX 21 — **community sources only, unverified.** | mixed |
| India VIX candle availability | **UNVERIFIED.** No documentation states it. Treat as a hard gate before relying on it. | unverified |
| Cash-equity candles adjusted for corporate actions | **Not applicable to this build's use, and UNVERIFIED as a vendor fact.** This feed is used for spot indices only, so no cash equity is pulled from it; whether its equity candles are split- or bonus-adjusted is not stated in anything read. numeric-pass16 p16num-2, D-1969. | unverified |

### 4z. Zerodha — recorded, and carried nowhere

The operator stated on **11 Aug 2026** that Zerodha serves **a rolling 10 years**
of history. It is written here because §3 rule 1 wants a stated fact traceable.

**IT IS NOW CARRIED IN A DESCRIPTOR, and this paragraph said the opposite until
14 Aug 2026.** It read: *"it is carried in no descriptor: `pull::vendor::Feed`
has four rows and none of them is this vendor. There is no transport, no
credential field and no wire name for it in this repository."* Every clause of
that is now false, and leaving it standing made §4z — the traceability anchor
every Zerodha fact in the code cites — assert that the thing citing it does not
exist.

| What shipped | Where |
|---|---|
| `Feed::Zerodha`, the fifth row | `pull::vendor::DESCRIPTORS` |
| Store prefix `zerodha` | `core::vendor::Vendor::Zerodha` |
| Transport | HTTP, `https://api.kite.trade` |
| Credential fields | `api-key` and `access-token`, the second named by `Auth::key_field` |
| Rungs asked for | `minute` and `day` |

**The history-depth lane below is unchanged and still true**: one source,
operator-stated, with no vendor page confirming it. A descriptor existing does
not make an unverified figure verified, and the row still says so.

| Fact | Value | Lane |
|---|---|---|
| History depth | rolling 10 years | operator-stated 11 Aug 2026, restated 14 Aug 2026, no vendor page states it |
| Descriptor | **`Feed::Zerodha`, shipped 14 Aug 2026.** Was "none exists"; see the paragraph above for what that claim cost while it stood. | verified from source |

**The vendor page has now been read.** `https://kite.trade/docs/connect/v3/historical/`,
read 14 Aug 2026. Every row below is quoted from it, so the "no vendor page read"
lane above applies only to the history depth, which that page still does not state.

| Fact | Value | Lane |
|---|---|---|
| Base URL | `https://api.kite.trade` | documented |
| Endpoint | `GET /instruments/historical/:instrument_token/:interval` | documented |
| Auth | header `Authorization: token api_key:access_token`, plus `X-Kite-Version: 3` | documented |
| Instrument identity | numeric `instrument_token`, from the instruments API — **not a symbol** | documented |
| Intervals published | `minute` `3minute` `5minute` `10minute` `15minute` `30minute` `60minute` `day` | documented |
| Intervals THIS BUILD WILL ASK FOR | **`minute` and `day` only** — the same two rungs Groww and Dhan serve. The operator narrowed it on 14 Aug 2026: "one and only one day pull and one min pull". The other six are real and are not wired, so `Descriptor::granularities` carries two and the remaining six refuse by name like every other unfetched rung. | operator-stated 14 Aug 2026 |
| Window params | `from` / `to`, `yyyy-mm-dd hh:mm:ss` | documented |
| `to` inclusivity | **INCLUSIVE — read from the vendor's own example, because the page never states it in words.** `from=2017-12-15 09:15:00&to=2017-12-15 09:20:00` is answered with **six** candles, stamped 09:15, 09:16, 09:17, 09:18, 09:19 and **09:20**. Both endpoints are present. The OI example one section below repeats the same window and returns the same six. This is the third vendor and the third answer — Dhan's daily `toDate` is non-inclusive, Groww's is inclusive, and Kite's is inclusive by demonstration rather than by assertion. `pull::vendor::HttpSpec::range_end` therefore takes `Inclusive` with the example as its citation. Re-read 14 Aug 2026. | documented by example, not by prose |
| Extra params | `continuous` (0/1), `oi` (0/1) | documented |
| Response | `{status, data:{candles:[[ts,o,h,l,c,volume(,oi)]]}}` — an array of ARRAYS, positional | documented |
| Timestamp | `2017-12-15T09:15:00+0530` — ISO **carrying an offset** | documented |
| Prices | decimal rupees (`1704.5`) | documented |
| Cash-equity candles adjusted for splits, bonuses or other corporate actions | **MEASURED: back-adjusted by the vendor, as of the day the month was pulled, for `minute` and `day` alike.** The historical page still does not say so; this is a measurement, not a vendor statement. **101 of 208** F&O shares have at least one run of three or more common sessions in which the GDFL raw first print ÷ the Zerodha `1day` open differs from 1 by more than 0.2%: a scan of every common day 2018-09-03..2026-09-24, run read-only on **6 Oct 2026** by the /db audit (its result file is held outside this repository) over the operator's store, `bars/zerodha/NSE/CASH/<SYM>/1day/` against `bars/gdfl/NSE/CASH/<SYM>/1s/`, and the count re-derived from that file on 7 Oct 2026. Re-measured on **7 Oct 2026** through a read-only shadow of the same store: HDFCBANK 2019-06-03 open ₹606.80 (Zerodha) against ₹2,427.00 (GDFL); RELIANCE 2019-05-30 close ₹633.75 against the last print ₹1,330.00; BRITANNIA 2018-11-28, the day before its bonus, close ₹2,980.87, off the 5-paisa tick, against ₹5,991.00. Examples of a run's factor (GDFL ÷ Zerodha): RELIANCE 2.0981 to 2023-07-19, then 2.0 to 2024-10-25, then 1; EICHERMOT 10.0; NESTLEIND 20.0; INFY 1.0216 to 2026-06-09. Which corporate actions the vendor applied is not stated anywhere read and is UNVERIFIED. D-3323. | measured 6–7 Oct 2026 |
| The adjustment is not always proportional | **MEASURED.** ASHOKLEY's minute close is GDFL ÷ 2 − ₹2.48: on 2019-06-10, over 375 minutes of `bars/zerodha/NSE/CASH/ASHOKLEY/1min/2019-06.bin` against the GDFL `1s` fold, the offset's p10/p50/p90 are 245/247.5/250 paisa (7 Oct 2026; the audit found the same on seven sampled days 2018–2024). Other factors change from run to run instead of stepping once: CANBK 359 runs between 1.03 and 6.74 over 1,913 sessions, BANKBARODA 357 runs between 1.03 and 1.34 over 1,908, BPCL 152 runs between 1.03 and 2.36 over 1,826 (6 Oct 2026 scan, 2018-09-03 onward). On such a run a percentage move is not the traded one: the audit measured CANBK 2020-05-11 open-to-close −2.008% on Zerodha against −1.561% on GDFL. D-3323. | measured 6–7 Oct 2026 |
| Adjusted prices are quantised coarser than the tick | **MEASURED.** Every BAJFINANCE Zerodha `1min` open, high, low and close on 2019-06-03 and 2023-06-15 (375 bars each) is a multiple of 50 paisa, while GDFL's raw prices those days are on the 5-paisa grid, consistent with the raw price ÷ 10 rounded to 50 paisa (the audit's example, 2019-06-03 09:15: GDFL high ₹3,478.35, Zerodha ₹348.00). From the 2025-06-16 ex-date the two feeds agree; GDFL's own raw prices were on a 50-paisa grid on 2025-06-13 and 2025-06-16 and on 5 paisa again by 2025-07-01. Read 7 Oct 2026 through the shadow store, `bars/zerodha/NSE/CASH/BAJFINANCE/1min/` and `bars/gdfl/NSE/CASH/BAJFINANCE/1s/`. The audit found BAJAJ-AUTO and BSE on the same 50-paisa grid. D-3323. | measured 7 Oct 2026 |
| Volume basis of share candles | **MEASURED: on its own basis, not scaled with price.** In **45 of the 101** adjusted shares, one probe day in some run shows the Zerodha `1day` volume ÷ the sum of GDFL `1s` volume differing from that run's price factor by more than 2% (one probe day in each of up to eight runs per share, 7 Oct 2026, through the shadow store: `bars/zerodha/NSE/CASH/<SYM>/1day/` against `bars/gdfl/NSE/CASH/<SYM>/1s/`; 100 of the 101 had a common day to probe). Examples: EICHERMOT 2019-08-28 price ×10.00, volume ×1.0004; CHOLAFIN 2019-03-12 price ×4.98, volume ×1.0003; CANBK 2020-05-11 price ×6.73, volume ×5.00. Where neither side is adjusted the ratio is about 1.0002 (RELIANCE 2026-09-22: 10,684,376 against 10,681,733). So price × volume on an adjusted span is not a traded value. D-3323. | measured 7 Oct 2026 |
| `day` close | **MEASURED: the official close, not the last print.** NIFTY `day` close differs from the last `minute` close of the same day on **2,081 of 2,126** common days of 2018-01..2026-09 (median 4.69 bps, p99 33.4, maximum 109.0 on 2021-02-24, the outage day of §3, whose minutes stop at 10:08); BANKNIFTY 2,083 of 2,126 (median 6.44 bps, p99 44.1, maximum 231.8 the same day). From 2026-08-03 the two are equal on all 42 common days for both. Read 7 Oct 2026 through the shadow store, the `1day` and `1min` files under `bars/zerodha/NSE/INDEX/NIFTY/` and `.../BANKNIFTY/`. How the exchange forms its official close is not sourced here. The `day` bar is not a fold of its minutes (D-3001, D-3347). D-3324. | measured 7 Oct 2026 |
| `minute` candles of a CAS-eligible share from 2026-08-03 | **MEASURED: they end at 15:14.** RELIANCE holds 360 `minute` bars on 2026-09-24 (09:15–15:14) and 375 on 2026-07-31 (09:15–15:29), `bars/zerodha/NSE/CASH/RELIANCE/1min/`, read 7 Oct 2026. The landing drops bars from 15:15 for an eligible share (`fetch::cash_close_verdict`, §9), so this is the store's rule as much as the vendor's. The `day` close that day, ₹1,219.20, is the closing-auction price (§9 below). D-3320. | measured 7 Oct 2026 |
| Expired F&O | `continuous=1` returns **day** candles for expired contracts of a live token's underlying, NFO and MCX futures | documented |
| Window cap | **NOW STATED, AND NOT BY THE PAGE.** The historical page still states no span limit at any interval — that half of the old row was and remains true. The figures come from Zerodha's own Kite Connect developer forum, thread `kite.trade/forum/discussion/7756`, posted by staff member `rakeshr` in May 2020, which other forum threads and Zerodha staff cite as the canonical reference. Max days per single request: `minute` **60** · `3minute` **100** · `5minute` **100** · `10minute` **100** · `15minute` **200** · `30minute` **200** · `60minute` **400** · `day` **2000**. A request spanning more than the permitted days for its interval FAILS, so a backfill must chunk to them. Captured 19 Aug 2026. | forum-sourced, authoritative-but-unversioned |
| Rate limit | **3 requests/second** on the historical candle endpoint | documented |
| Rate-limit refusal | HTTP **429** | documented |
| Session expiry | **`TokenException`, HTTP 403.** Caused by logout, natural expiry, **or the user logging into another Kite instance.** Clear the session and re-login. | documented |
| Other exceptions | **NINE ON THE PAGE, AND A TENTH THAT IS NOT.** This row carried four. The exceptions page publishes nine, in this order: `TokenException` · `UserException` · `OrderException` · `InputException` · `MarginException` · `HoldingException` · `NetworkException` · `DataException` · `GeneralException`. The page also states the handling rule per group, which is what `pull::kite` encodes: `TokenException` clears the session and re-logins; `NetworkException` and `DataException` are *"candidates for a bounded retry"*; `InputException`, `MarginException` and `HoldingException` are *"deterministic rejections of the request as sent"*. Read 19 Aug 2026. | documented |
| `PermissionException` — the tenth, and the one that costs | **NOT ON THE EXCEPTIONS PAGE. Live in the vendor's own SDK.** `github.com/zerodha/pykiteconnect`, `kiteconnect/exceptions.py`: *"Represents permission denied exceptions for certain calls. Default code is 403."* It is what an API key with no historical-data subscription is answered with. **It shares HTTP 403 with `TokenException`**, so on the status axis alone the two are one event — and this build reported the second as the first, telling an operator with no subscription that the next pull would read a refreshed credential. Nothing about that key is refreshable. The pair is why `pull::refusal` exists and why `HttpSpec::error_names` is a field. The vendor's own page warns of exactly this: the nine names are *"the complete set documented on this page, not the complete set of `error_type` strings the API can return"*, and an unrecognised `error_type` must be logged raw rather than folded into `GeneralException`. | vendor SDK, not the documentation page |
| Other HTTP codes | 400 bad params · **405 method not allowed** · 404 not found · 410 gone permanently · 500 · 502 OMS down · 503 · 504 | documented |
| Per-day quota | **NONE on this endpoint, and the only daily cap on the page is not ours.** `kite.trade/docs/connect/v3/exceptions/` publishes one daily figure — *"a single user/API key will not be able to place more than 5000 orders per day"* — and it is stated against ORDER PLACEMENT, alongside 400 orders/minute and 25 modifications per order. This build places no order and never will, so the figure is recorded as read and is **not** carried into `Budget::per_day`, which stays `None`. A `None` here means "no published bound on that span", which is the honest reading; copying the order cap across would be promoting a figure past the endpoint it was measured against, the same error the Dhan window-cap row names. Re-read 14 Aug 2026. | documented, and not applicable |
| History DEPTH per interval | **THERE IS NOW A SECOND CLAIM, AND THIS ROW SAID THERE WAS NOT.** It read: *"it stands **uncontested** rather than contested: a vendor page that declines to state a depth is not a second claim, so `ClaimStanding` has one row to weigh here and no tie to break. Contrast Groww and Dhan, where a vendor table gives a competing figure."* The page is still silent — data *"spanning back several years"*, a phrase and not a figure — so that clause holds of the PAGE. It does not hold of the vendor. The same forum thread `kite.trade/forum/discussion/7756`, posted by the `kiteapi` staff account in August 2020, states: every intraday rung (`minute` through `60minute`) **up to 3 years**; `day` **since 1990 for some NSE instruments, and since January 2008 for BSE**. Captured 19 Aug 2026. So Kite now has exactly the tie Groww and Dhan have. **AND THEN A THIRD CLAIM ARRIVED, FROM THE SAME VENDOR, AND IT SETTLES IT THE OTHER WAY.** `kite.trade/forum/discussion/14149`, Zerodha staff `sujith`, **June 2024**: *"There is no fixed date for day candle data but for minute level data it starts somewhere around early 2015"*, and *"For some NSE stocks, day candles are back filled till late 1990s as well"*. Captured 22 Aug 2026. That is **four years later than the 2020 post**, on the same forum, at the same staff standing — so the contest was never operator-versus-vendor at all. It is **vendor versus vendor**, and the later word puts the minute rung at roughly eleven and a half years: LONGER than the ten the operator claimed. The rolling floor was not the risky widening this row feared; it was an **undershoot** that refused nineteen months of one-minute history the vendor holds. The "seven years of empty windows" cost is therefore **withdrawn, not restated** — it was sound only while the 2020 figure was the vendor's latest word. Both rungs are now `HistoryFloor::Fixed { 2015-01-01 }`, and `Fixed` rather than `Rolling` is a shape correction as much as a number one: Zerodha's minute history starts at a fixed point and grows forward, so a floor moving with the clock would refuse 2015–2020 by the year 2030. What remains UNVERIFIED is much narrower and is kept: *"somewhere around early 2015"* is the vendor's own hedge, so a window opening on 1 January 2015 may spend a few weeks on empty answers — the safe direction, since asking for slightly more than exists costs empty windows while asking for less loses data silently. D-0258. | vendor-sourced, and the vendor's later word now agrees with the operator |
| Gap-filling / completeness | **UNVERIFIED, and no page claims it.** Nothing read states that a candle exists for every session minute, nor what a missing session returns. | unverified |

**`TokenException` is the row to read twice.** It fires when the user logs into
ANOTHER Kite instance — so a human opening kite.zerodha.com while a backfill runs
kills that backfill's session. `CLAUDE.md` §8's rule that this repository never
mints a token is not a limitation here, it is the correct posture: a mint would
invalidate whatever else holds the session.

At 3 req/s the arithmetic for a 2020→yesterday backfill is 80 month-windows ×
~800 instruments ÷ 3 = **~5.9 hours of wall clock at the cap, per rung**, before
any retry. That is arithmetic from a published limit, not a measurement.

#### The instruments API — where a numeric token comes from, read 14 Aug 2026

`https://kite.trade/docs/connect/v3/market-quotes/`. The historical endpoint is
addressed by `instrument_token` and nothing else, so this page is the whole of
how an instrument this repository already knows becomes a Kite request. It was
not read when §4z was first written, and the fourth obstacle recorded below —
"the instrument is a NUMERIC TOKEN … so the (exchange, symbol) key this
repository joins on does not address a Kite request at all" — is **half right
and the half that is wrong changes the design**.

| Fact | Value | Lane |
|---|---|---|
| Endpoint | `GET /instruments` for every exchange, `GET /instruments/:exchange` for one | documented |
| Response | a **gzipped CSV dump**, not JSON — the only endpoint on this vendor that is not JSON | documented |
| Freshness | *"The dump is generated once everyday"*, and the page recommends requesting it once a day at around 08:30 AM and storing it | documented |
| Columns, in order | `instrument_token`, `exchange_token`, `tradingsymbol`, `name`, `last_price`, `expiry`, `strike`, `tick_size`, `lot_size`, `instrument_type`, `segment`, `exchange` — twelve | documented |
| **ISIN** | **ABSENT. There is no ISIN column.** D-0125 keyed the constituent join on NSE's own ISIN at both ends and D-0117 keys a tier to a vendor's ids on `(exchange, ISIN)`. Neither addresses a Kite row, because a Kite row does not carry one. | documented by absence |
| The key the VENDOR names | *"For storage, it is recommended to use a combination of **exchange and tradingsymbol** as the unique key, **not** the numeric instrument token."* | documented |
| Why not the token | *"Exchanges may reuse instrument tokens for different derivative instruments after each expiry."* A token is therefore stable for a spot index or a cash equity, which do not expire, and **unstable for F&O across an expiry boundary**. The engine surface is NSE spot (§1), so the reuse hazard does not reach it — and it binds the moment an F&O row is stored. | documented |
| `instrument_type` alphabet | `EQ`, `FUT`, `CE`, `PE` — four words, and **no index word among them**. How an index row spells this column is **UNVERIFIED**; the page's own quote examples address one as `NSE:NIFTY 50`, so the tradingsymbol is known and the type and segment words are not. | documented (four) · unverified (index) |
| An index tradingsymbol carries a SPACE | `NSE:NIFTY 50`, from this page's `/quote/ohlc` and `/quote/ltp` examples. Recorded because a value with a space cannot sit in a URL path segment unescaped, and `pull::http` refuses such a value by name rather than encoding it — see D-0134. | documented |
| Rate limit | not the historical 3/s. The page's own table puts everything outside quote, historical and orders at **10 req/second**, and this call is one request per day. | documented |
| `exchange` column values in the dump | **`NSE`, `NFO`, `MCX`** appear in this page's own sample rows — a NIFTY future and a NIFTY call filed under `NFO` (segments `NFO-FUT`, `NFO-OPT`), a silver future under `MCX` (segment `MCX`). Re-read 6 Oct 2026. So the column is **not a venue alone**: NSE's own index derivatives carry `NFO`, not `NSE`. | documented by sample |
| Exchange codes the vendor names | **`NSE BSE NFO CDS BCD MCX`** — `kite.trade/docs/connect/v3/orders/`, regular order parameters: *"Name of the exchange (NSE, BSE, NFO, CDS, BCD, MCX)"*. The page does not expand the abbreviations. **`BFO`** — not on that page; the vendor's own SDK, `github.com/zerodha/pykiteconnect` `kiteconnect/connect.py`, defines `EXCHANGE_BFO = "BFO"` beside the six. **`NCO`** — on neither; Zerodha staff `rakeshr`, `kite.trade/forum/discussion/13953`, April 2024: *"it's recently added the NSE-Commodity exchange"*. All read 6 Oct 2026. | documented (six) · vendor SDK (`BFO`) · forum staff (`NCO`) |
| What each code MEANS beyond the samples | **UNVERIFIED** for `CDS`, `BCD`, `BFO`: no page read expands them. None is needed: every one is declined by code (D-3310), so no stored row depends on its meaning. | unverified, and unused |
| Whether the dump carries any code beyond these eight | **Superseded in part by the next row.** The 6 Oct 2026 refusal (`line 2: unrecognised exchange`) did not say which code line 2 held, and the dump cannot be re-read without spending the session token. A code outside the list still refuses the master, now by name (D-3310). | unverified |
| `NSEIX` in the dump | **Observed in the live dump.** The operator's app, built with D-3310, refreshed `https://api.kite.trade/instruments` (HTTP 200) on **6 Oct 2026 at about 10:36 IST** and refused it at *"line 25963: unrecognised exchange `NSEIX`"*. So line 2 now passes and `NSEIX` is the first code past the eight. Zerodha staff `sujith`, `kite.trade/forum/discussion/14874`, March 2025: *"I think it is `NSEIX` for GIFT NIFTY"*. The staff answer is hedged, so what `NSEIX` covers beyond GIFT NIFTY is **UNVERIFIED**. It is declined by code (D-3311), so nothing stored depends on that. | operator/app-observed · forum staff |
| Any code after line 25963, and `GLOBAL` | **Superseded by the next two rows.** Forum threads about global indices (`kite.trade/forum/discussion/14817`, `/13841`), read 6 Oct 2026, show no dump row or staff statement giving `GLOBAL` or any other code as an `exchange` value. A refusal now keeps the refused body (D-3311), so the next refusal shows every code the dump holds. | unverified, then observed |
| `GLOBAL` in the dump | **Observed in the live dump.** The operator's app refreshed `api.kite.trade/instruments` on **6 Oct 2026 at 11:07 IST**. It refused at *"line 25979: unrecognised exchange `GLOBAL`"* and kept the body (D-3311). Counted from that kept file: **12** `GLOBAL` rows, **all on segment `INDICES`**. That is all that is known about `GLOBAL`. What it covers beyond "12 INDICES rows in the dump" is **UNVERIFIED**. It is declined by code at the exchange gate, before any segment is read (D-3312). | operator/app-observed |
| Every `exchange` value in one complete dump | Counted by the operator from the same kept body (11:07 IST, 6 Oct 2026; header `instrument_token,…,segment,exchange`): **`BFO` 5,016 · `BSE` 12,992 · `CDS` 7,902 · `GLOBAL` 12 · `MCX` 15,330 · `NCO` 21,557 · `NFO` 34,928 · `NSE` 10,318 · `NSEIX` 1**. That is nine codes and no others. `BCD` is in the vendor's list but not in this dump. `NSE` rows carry only two segments: **`NSE` 10,182** and **`INDICES` 136**. The 136 matches the `INDICES` count measured on 19 Aug 2026 (`segment_of`'s comment). This is evidence from one day's dump; it does not promise what later dumps will hold. A new code still refuses the master by name and keeps the body. | operator/app-observed |
| `MCX` in the Groww and Dhan masters | Operator-measured 6 Oct 2026 on the two public files: Groww `exchange` and Dhan `EXCH_ID` each carry **15,319** `MCX` rows, beside `NSE` and `BSE`. Recorded for D-3310's shared `MCX` arm; neither vendor is in use. | operator-measured |

**So the token map is not a new kind of thing.** It is this vendor's instrument
master, decoded into the `core::vendor::MasterRow` the other two brokers already
decode into, keyed on `(exchange, tradingsymbol)` — which is the key **the vendor
itself names**, and which `MasterRow::trading_symbol` already carries because
Groww's CASH rows have always needed it. `core::vendor::VendorId` holds an opaque
per-vendor id of up to 48 bytes and an `instrument_token` is at most eight
digits, so the numeric token is a `VendorId` like any other and no second
identity type is required.

What is genuinely new is the **transport of the master**: gzipped CSV over HTTP,
where the two existing masters are files on disk. That is a `pull::manifest`
question and not a `pull::vendor::Descriptor` one.

#### Why this vendor cannot be described by the current descriptor, and it is all three

`docs/07-plan.md` §5 measured three fields that "cannot express an arbitrary
broker at all" and predicted the vendor that would prove it. Zerodha is that
vendor, and it trips **every one**:

1. **`HttpSpec::bars_path` is a fixed string concatenated with `base_url`.**
   Kite carries the instrument AND the interval as PATH SEGMENTS
   (`/instruments/historical/5633/minute`). §5's exact words: "A vendor whose
   instrument and granularity are *path segments* cannot be described."
2. **`AuthScheme` is `Raw | Bearer`.** Kite's is `token api_key:access_token` —
   a prefix AND a second secret. §5: "A scheme carrying a prefix and a **second**
   secret cannot be described."
3. **`TimestampEncoding::IsoDateTimeText` documents itself as carrying no zone.**
   Kite returns `+0530`. §5: "A vendor returning `+0530` cannot be described."

A fourth, not in §5 and found here: the instrument is a NUMERIC TOKEN from a
separate instruments call, not a tradingsymbol, so the (exchange, symbol) key
this repository joins on does not address a Kite **request** at all.

**That fourth row is half right, and the half that is wrong was corrected on 14
Aug 2026 by reading the instruments page** (above). The REQUEST is addressed by
the numeric token, which is the true half. The **master row** is keyed by
`(exchange, tradingsymbol)` — and that is not this repository's choice, it is the
vendor's own instruction: *"it is recommended to use a combination of exchange
and tradingsymbol as the unique key, not the numeric instrument token."* So the
key this repository joins on **does** address a Kite row; what it does not do is
address a Kite request, and the master is exactly the table that carries you from
one to the other. The `(exchange, ISIN)` key of D-0117 and D-0125 is the one that
genuinely cannot be used here, because the CSV has no ISIN column.

**So adding Zerodha is not a descriptor row.** It is three `pull::vendor` type
changes plus this vendor's instrument master, and each needs its own decision
entry. The prediction in §5 was right and this is the evidence that closed it.
The "instrument-token map" this line used to call for is **not** a new structure:
it is `core::vendor::MasterRow` decoded from a gzipped CSV, with the numeric
token living in the `VendorId` the other two brokers already populate.

### 4a. Instrument facts transcribed into source

Golden rule 1: every claim about an instrument names the route that produced
it. These are the instrument-level facts compiled into this repository as Rust
data rather than fetched, so the route has to be recorded here.

| Fact | Where it lives | Route | Lane |
|---|---|---|---|
| **NIFTY Total Market — 750 constituents** | `core::universe::NIFTY_TOTAL_MARKET` | niftyindices.com states the index holds 750 stocks: "all stocks that are part of Nifty 500 and Nifty Microcap 250". The names were transcribed from the local lake's NSE/CASH directories and matched the primary broker's equity list 750/750 with zero misses. Re-checked against both real masters 2026-08-01: 750/750 resolve as a kept equity in **both** vendors, ISINs agreeing. | derived + cross-checked; **UNVERIFIED against an NSE constituent circular** |
| **F&O underlyings — 213 names** | `core::universe::FNO_UNDERLYINGS` | Derived from the derivative rows of both vendor masters, excluding `*NSETEST`. The two brokers agree exactly: 213 each, zero on either side only. | derived + cross-checked; **UNVERIFIED against an NSE circular** |
| **NSE board series — 128 codes** | `core::vendor::EQUITY_BOARD_SERIES`, `SME_BOARD_SERIES`, `NON_EQUITY_SERIES` | The measured union of every distinct `series` on a Groww `NSE/CASH/EQ` row and every distinct `SERIES` on a Dhan `NSE/E/EQUITY` row, 2026-08-01. 6 equity-board, 2 SME, 120 debt and fund. Board verdicts agree on 4,080 of 4,080 shared ISINs. | measured from both masters; **UNVERIFIED against an NSE series circular** |

Both index lists are **snapshots** of rebalanced indices, and none of the three
has been checked against an exchange publication. `docs/06-limits.md` §11
carries what that costs. D-0025 and D-0029.

**The Total Market row above is now partly contradicted, and the contradiction
is recorded rather than repaired.** On 2026-08-11 the exchange's own file was
fetched for the first time (§4c). Excluding two placeholder scrips it holds 750
real names, the same count — but not the same names. The exchange lists
`GRINDWELL`, which `NIFTY_TOTAL_MARKET` does not hold; `NIFTY_TOTAL_MARKET`
holds `AGL`, which the exchange does not list. One name in 750, and the
constant's 750 is therefore **not a subset** of the exchange's 752. The array
is deliberately left as it stands: it was measured 750/750 against two real
vendor masters under the D-0025 gate, and that measurement cannot be redone
from a downloaded CSV. `AGL` is **UNVERIFIED** — this repository does not know
what instrument it is, and will not guess. D-0089 states the resolution and
`docs/06-limits.md` §11 carries the cost.

### 4d. The NSE index directory — read 14 Aug 2026, and it settles a claim this repository asserted the opposite of

**`crates/api/src/ingest.rs` stated, as the justification for the whole shape of
`SpotTarget::Indices`:** *"NSE publishes no file naming that set, so a hardcoded
one would be invention and would go stale the day a vendor added a series."*

**That is false, and it was read as false on 14 Aug 2026.** NSE Indices Limited
— the company that computes the indices — publishes a categorised directory of
every equity index it maintains, and each index's own page carries a link to its
constituent CSV.

| Category | Indices | Route |
|---|---|---|
| Broad-based | **22** | `niftyindices.com/indices/equity/broad-based-indices` |
| Sectoral | **34** | `niftyindices.com/indices/equity/sectoral-indices` |
| Strategy | **48** | `niftyindices.com/indices/equity/strategy-indices` |
| Thematic | **44** | `niftyindices.com/indices/equity/thematic-indices` |
| **Total** | **148** | four category pages, each listing its members |

| Fact | Value | Lane |
|---|---|---|
| Directory exists | **Yes.** Four category pages, 148 equity indices between them, each linking its own page. | verified 14 Aug 2026 |
| Constituent file | On each index's page, as `/IndexConstituent/ind_<name>list.csv` | verified |
| CSV header | `Company Name,Industry,Symbol,Series,ISIN Code` — **identical to the five files §4c already reads**, and every row carries an ISIN | verified from a live body |
| Measured example | `ind_niftybanklist.csv`, 916 bytes, header as above, first row `AU Small Finance Bank Ltd.,Financial Services,AUBANK,EQ,INE949L01017` | verified |
| **The URL is NOT derivable from the index name** | `ind_niftybanklist.csv` against `ind_niftytotalmarket_list.csv` — one word-joined, one underscore-separated. There is no rule that turns "Nifty Bank" into the first and "Nifty Total Market" into the second. | verified, and it is the load-bearing row |

**Why the last row decides the design.** A resolver that *derives* a CSV URL from
an index's name is inventing a filename convention the exchange has not stated,
and §3 rule 1 forbids exactly that — the two examples above already disagree
with each other. So the directory must be **crawled**: category page → index
page → the constituent link that page carries. Every step reads a URL the
exchange published rather than one this repository composed.

**What this does NOT settle.** Whether all 148 are instruments any vendor
serves, and whether a vendor's "index series" set is a subset of these 148, is
the cross-check the resolver exists to perform and has not performed. The count
above is what the exchange publishes, not what is reachable. Nothing here
widened `CLAUDE.md` §1 at the time it was written: the sweep surface was
`NSE-NIFTY` and `NSE-BANKNIFTY`, and every one of the other 146 was **stored,
never swept**. D-0506 has since widened the surface to the cash equities of the
213 F&O underlyings; the 146 here are index constituents, a different question,
and this paragraph is left as the record it was.

### 4b. Option-greek facts, measured from a live chain

Golden rule 1 again. `crates/greeks` makes claims about what a vendor's option
chain *means*, and a unit convention nobody states is exactly the kind of claim
that has to name its route. Every row below was measured from **one live Dhan
option-chain response, one strike, both sides**, captured 2026-08-01:
`IV 11.939337251984934 / 9.789193798280868` (percent), `delta 0.53871 /
−0.46732`, `gamma 0.00132 / 0.00109`, `theta −15.1539 / −10.61131`,
`vega 12.2025 / 12.18593`. The response carried **no rho, no spot, no strike,
no timestamp and no expiry.**

| Fact | Where it lives | Route | Lane |
|---|---|---|---|
| **Dhan publishes vega per one percentage point** | `greeks::bsm` module docs; `greeks::vendor_anchor::vega_is_published_per_percentage_point_and_the_index_level_proves_it` | The raw scaling implies an index level of **258.51**; the per-percent scaling implies **25,851.19**, which is where NIFTY trades. | measured; the vendor documents no unit |
| **Dhan publishes theta per a CALENDAR day, not a trading day** | same, and `greeks::vendor_anchor::the_trading_day_divisor_is_excluded_and_the_calendar_one_is_not_selected` | The two sides of one strike must agree on `r`. They agree to **0.9999** points under 365 and **23.2598** under 252 — a factor of 23.3. | measured — but only as an **exclusion of 252** |
| **The divisor is exactly 365** | `greeks::vendor_anchor::the_rate_criterion_has_its_root_at_370_and_365_is_a_convention_near_it` | The same criterion is affine in the divisor and has its root at **`D* = 370.0757`**, where the two sides agree to 2.8e-15 points; it also prefers **375** (spread 0.9700) to 365 (0.9999). `365` is the nearest ordinary calendar convention to that root, not a measurement of it. | **UNVERIFIED.** Withdrawn from "measured" by D-0046 |
| **Dhan's two published IVs are transposed relative to its delta/gamma/vega block** | `greeks::vendor_anchor::the_two_published_volatilities_are_transposed_and_the_identity_says_so` | The scale-free identity `vega·gamma·sigma == n(d1)^2` — no spot, strike, maturity or rate in it — gives 1.21979 / 0.82249 as published and 1.00012 / 1.00315 swapped, against gamma's own printed slack of 0.38% / 0.46%. | measured, **from one strike only.** Whether every strike is transposed the same way is **UNVERIFIED** |
| **Dhan uses standard spot BSM** | `greeks::vendor_anchor::our_greeks_reproduce_the_captured_dhan_chain` | Gamma and vega are near-identical between the two sides at one strike, which is BSM. | measured; **forward Black-76 is not excluded — UNVERIFIED** |
| **Dhan's carry is zero** | `greeks::vendor_anchor::the_carry_is_consistent_with_zero_and_the_sample_cannot_pin_it` | `q = 0` reproduces the chain, and a single volatility would need `q = −42.54%`, which is not a rate. But `q = 1%` and `q = 2%` reproduce **all eight** published fields too, gammas included, at `T = 4.085` and `3.428` calendar days. | **UNVERIFIED.** `q = 0` is *consistent*, not measured. Withdrawn from "measured" by D-0046 |
| **Dhan publishes no rho; Groww publishes all five** | `crates/greeks` ships rho regardless | The captured response has no `rho` field. Groww documents a dedicated Greeks section at `groww.in/trade-api/docs/curl/live-data`. | read from the response and from the vendor documentation |
| **Dhan's risk-free rate** | nowhere — nothing in this repository hardcodes one | Solved at **9.4619%** (call side) and **10.4618%** (put side), each ±0.55 at 95% from the printed precision of delta alone, with a 1.00-point side-to-side residual outside both intervals. | **UNVERIFIED.** A hardcoded 10.0% fits the sample and so does a market rate near 7%; this sample cannot separate them |
| **NIFTY and BANKNIFTY strike intervals** | nowhere — `Moneyness::from_ladder` takes the interval as an argument | No source states them. | **UNVERIFIED before 2021-01-01.** `costs::strike::strike_step_on` encodes 50 and 100 rupees from 2021-01-01, sourced to the predecessor's `STRIKE_STEP` over its 2021-2026 backfill window, and refuses every earlier day; `Moneyness::from_ladder` itself still assumes none. (This said "assumed nowhere in code". D-1779.) |

The maturity is the best-conditioned parameter the sample carries:
`T = 0.0141324716` years = **5.15835 calendar days**, ±0.079 at 95% over the
rounding box of the printed fields. That interval is a **rounding-box width,
not an identification result**, and it is conditional on `q = 0` and on the
transposition — at `q = 2%` the same eight fields give `T = 3.428` days. The
**day count that generates it** is UNVERIFIED without the capture timestamp.

**And the sample is internally inconsistent with spot BSM at one instant.**
Matching both published deltas forces the model's vega ratio to `0.998641`
against the vendor's `1.001360`, in a quantity containing no spot, strike,
maturity or rate. The best possible *single* contract is off by **884× the
vendor's own display precision** — measured. Everything above is fitted with
**two** mutually inconsistent contracts, one per side, which is a diagnostic
and not an agreement. D-0046, and `docs/06-limits.md` §18.

### 4c. NSE index constituents, read from the exchange's own files

Golden rule 1 again, and this time the rule caught something that had already
shipped. The web pages offered NIFTY 50 / 100 / 200 / 500 and produced them by
slicing `core::universe::NIFTY_TOTAL_MARKET`, which is stored alphabetically —
so `slice(0, 50)` yielded `360ONE, 3MINDIA, AADHARHFC, …` and the page called
that the NIFTY 50. No source said any of it. The four tiers did not exist in
this repository at all. D-0089 replaces the slice with the exchange's own
published constituent files, which are the route recorded here.

**Fetched 2026-08-11**, over HTTPS, from `nsearchives.nseindia.com` — the
archive host of the exchange that computes the indices. Each file is a CSV with
the header `Company Name, Industry, Symbol, Series, ISIN Code`; **every row
carries an ISIN**, which is what makes a row checkable against a vendor master
rather than merely readable.

| Index | URL | Rows | Where it lives |
|---|---|---|---|
| NIFTY 50 | `https://nsearchives.nseindia.com/content/indices/ind_nifty50list.csv` | 50 | `core::universe::NIFTY_50` |
| NIFTY 100 | `https://nsearchives.nseindia.com/content/indices/ind_nifty100list.csv` | 100 | `core::universe::NIFTY_100` |
| NIFTY 200 | `https://nsearchives.nseindia.com/content/indices/ind_nifty200list.csv` | 200 | `core::universe::NIFTY_200` |
| NIFTY 500 | `https://nsearchives.nseindia.com/content/indices/ind_nifty500list.csv` | 500 | `core::universe::NIFTY_500` |
| NIFTY Total Market | `https://nsearchives.nseindia.com/content/indices/ind_niftytotalmarket_list.csv` | **752** | `core::universe::NIFTY_TOTAL_MARKET` holds **750** — see below |

**Symbols only are transcribed.** `CLAUDE.md` §2 allows no `.csv` in the
tracked tree, so the files are input and the data becomes Rust. The company
name, industry, series and ISIN are read, used for the checks in this section,
and not carried: nothing in `crates/core` reads them, and a field carried
without a reader is a field that goes stale unnoticed.

**Verified on the fetched files, not asserted:**

- The tiers **nest**: all 50 NIFTY 50 symbols are in the 100, all 100 in the
  200, all 200 in the 500, and all 500 in `NIFTY_TOTAL_MARKET` — zero outside
  in every direction. Proved for every symbol by
  `core::universe::the_published_tiers_nest_one_inside_the_next`, which is why
  `of_equity` may set several bits for one name.
- The Total Market file is exactly the union of the NIFTY 500 file and the
  NIFTY Microcap 250 file — 752 rows, row for row, nothing on either side
  alone. That matches niftyindices.com's own definition, "all stocks that are
  part of Nifty 500 and Nifty Microcap 250".

**The 752 that is 750.** Two of the 752 rows are NSE placeholder scrips, not
constituents: `DUMMYINXGN` ("Dummy Inox Green Ltd.") and `DUMMYTRVN` ("Dummy
Triveni Ltd."). Their identifiers are not ISINs — they read `DUM510W01014` and
`DUM256C01024`, the real ISINs of `INOXGREEN` and `TRIVENI` with `INE` replaced
by `DUM` — and both of those real constituents are separately present in the
same file. Dropping the two placeholders leaves **750 real names**, which is
the count niftyindices.com states and the count `NIFTY_TOTAL_MARKET` declares.
The agreement of the counts is a coincidence of arithmetic, not of membership:
see §4a's row, `docs/06-limits.md` §11, and D-0089, which records that the
repository's 750 and the exchange's 750 differ by one name.

**What NSE publishes and this repository does NOT carry.** The exchange's own
index directory (`https://www.nseindia.com/api/allIndices`, same fetch) lists
**139 indices**. This repository carries **five** of them — the four tiers above
plus NIFTY Total Market — alongside the derived F&O underlying list of §4a. The
other 134, sectoral and thematic and strategy indices among them — NIFTY BANK,
NIFTY IT, NIFTY MIDCAP 150, NIFTY MICROCAP 250 and the rest — are **not
carried**, are not a universe, and no part of this repository may claim
membership in one. Two of them appear in this section only as evidence:
Microcap 250 was read to check the Total Market's composition, and it was not
transcribed.

These are **snapshots.** NSE rebalances these indices semi-annually and none of
the five has been checked against a constituent circular. `docs/06-limits.md`
§11 carries what that costs.

---

## 4e. Statistical procedure sources

These sources govern the named procedure; they do not turn its assumptions
into a guarantee about this dataset.

| Procedure fact | Primary source | What is and is not carried |
|---|---|---|
| CSCV begins with one synchronous `T × N` performance matrix, partitions its rows into an even number `S` of equal-sized disjoint blocks, visits every canonical half-block training set with its exact complement as test, chooses the in-sample maximum under one fixed performance measure, and estimates PBO from how often that winner ranks below the out-of-sample median | Bailey, Borwein, López de Prado and Zhu (2015/2017), *The Probability of Backtest Overfitting*, [author-hosted PDF](https://www.davidhbailey.com/dhbpapers/backtest-prob.pdf), Algorithm 2.3 and §3.1; [journal abstract](https://www.risk.net/journal-of-computational-finance/2471206/the-probability-of-backtest-overfitting) | The synchronous complete-family construction, the equal-block partition and the rank event are carried. **The enumeration is not.** Algorithm 2.3 forms every one of the `C(S,S/2)` half-block training sets, so each complementary pair is visited in both orientations. This repository's CSCV (D-0476, `cli::population_statistics_v2::canonical_split_count`) keeps segment 0 on the test side and visits exactly `C(S-1,S/2)` = `C(S,S/2)/2` splits, one orientation per pair: the swapped orientation, whose in-sample winner can differ, is never scored. That is a deviation from the paper, not a reading of it, and it bites hardest at small `S` — at `S = 2` the paper scores two splits and this scores one, so the estimate can only be 0 or 1. D-1448. The paper says different trading frequencies must be aggregated to one common index, but it does **not** choose this repository's IST observation unit, empty-session treatment, segment count, tie policy, pessimistic-return score, bootstrap seed/draw count or block length. Those remain explicit versioned implementation decisions and cannot be smuggled in as facts from the paper. |
| Romano--Wolf stepdown controls family-wise error by testing maxima over successively smaller surviving hypothesis sets; a fixed bootstrap distribution makes those critical values monotone | Romano and Wolf (2005), *Exact and Approximate Stepdown Methods for Multiple Hypothesis Testing*, JASA 100(469), 94--108, [publisher DOI](https://doi.org/10.1198/016214504000000539), [author-hosted PDF](https://www.econ.uzh.ch/dam/jcr:ffffffff-935a-b0d6-ffff-ffffd823d949/jasa.pdf), especially §4.2 and Theorem 6 | The construction and its stated asymptotic conditions are carried. It is **not** a distribution-free finite-sample promise for arbitrary strategy returns. The paper explicitly directs dependent data to block bootstrap methods; it does not select this repository's block length. |
| Candidate adjusted p-values order observed statistics from largest to smallest, count strict exceedances of each surviving-suffix resample maximum with the finite-resample `(+1)/(M+1)` correction, then apply a cumulative maximum; that monotonicity step is essential | Romano and Wolf (2016), *Efficient Computation of Adjusted p-Values for Resampling-Based Stepdown Multiple Testing*, [University of Zurich Working Paper 219](https://www.econ.uzh.ch/apps/workingpapers/wp/econwp219.pdf), Algorithms 3.1 and 4.1 and Remark 4.1 | `runner::bootstrap::romano_wolf_adjusted_p_values_v1` implements that exact-count algorithm over one shared stationary-bootstrap matrix. Exact observed-statistic ties use ascending caller position, and any zero-variance candidate refuses the complete adjusted receipt rather than receiving the strict-exceedance floor. Those two discrete-data rules are D-0460 implementation choices, not claims made by the paper. |

---

## 4f. Research policy explanation sources — reviewed 2026-09-07

| Method or interpretation | Primary source | Boundary |
|---|---|---|
| Wilson intervals incorporate sample size when estimating a binomial proportion | [NIST confidence intervals](https://www.itl.nist.gov/div898/handbook/prc/section2/prc241.htm) | Does not establish independent trading outcomes or choose a minimum sample size for this repository. |
| White's Reality Check addresses data snooping; Hansen's SPA uses studentization and a sample-dependent null distribution | [White (2000)](https://users.ssc.wisc.edu/~behansen/718/White2000.pdf), [Hansen (2005), primary publisher abstract](https://www.tandfonline.com/doi/abs/10.1198/073500105000000063) | These methods motivate complete-population evidence. Their assumptions remain material; they do not select this repository's acceptance thresholds. |
| P-values do not measure the probability a hypothesis is true; decisions should not rest solely on a threshold | [American Statistical Association statement](https://www.amstat.org/asa/files/pdfs/p-valuestatement.pdf) | The delegated37-field profile contains explicit research choices. No cited source prescribes its5%p-value cutoff,20%PBO limit, sample floors or loss caps. See [the explanation](22-research-policy.md). |

## 4g. GDFL — what D-0802 depends on, and how little of it is recorded

D-0802 (1 October 2026) records the operator's design that every entry, exit
and exit combination, and the ranking that reads those fills, is to be priced
from GDFL one-second data, at each second's worst-case high or low, on the
underlying spot. Nothing of it is built. **This section records no GDFL fact
from a vendor source.** It records one operator statement, one day measured
from GDFL files outside this repository, and what must still be read from a
source and written here, with its lane, before any code relies on it.

**Operator statement.**

| Fact | Value | Lane |
|---|---|---|
| GDFL coverage for this design | GDFL second-level snapshot data covers everything D-0802 needs. Excerpt of message 5, whose whole text D-0802 quotes: *"... for gdfl entilrey we have seconds level snapshot dude so you don't need to worry we have all the data ..."* | operator-stated 1 Oct 2026, no vendor page states it |

**Measured on one day only.** Read on 1 October 2026 (IST) from an extracted
GDFL copy outside this repository, a directory on the operator's own storage
named `NSE_Tick_2018-09-01_to_2026-09-24` (where it is mounted is machine
layout and is not recorded; its name ends at 24 September 2026, though its
folder names run to 30 September 2026; why is not recorded), by `awk` over the
files named. Anyone
with that copy can repeat each figure. Each row is a
measurement of those files on 26 May 2026 and no more; it is not a vendor
statement and does not say what any other day holds.

| Fact | Value | Lane |
|---|---|---|
| Index file path | `INDICES/<yyyy>/<MON_yyyy>/GFDLCM_INDICES_TICK_<ddmmyyyy>/<NAME>.NSE_IDX.csv`; 141 files on 26 May 2026, among them `NIFTY 50.NSE_IDX.csv` and `NIFTY BANK.NSE_IDX.csv` | measured, one day |
| Index columns | `Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest`, header present, date `dd/mm/yyyy`, time `hh:mm:ss` with no sub-second field, in every row | measured, one day, two files |
| Index fields beyond the level | every column after `LTP` is zero in every row, so the volume columns carry nothing | measured, one day, two files |
| Rows per session second (09:15:00 to 15:29:59) | NIFTY 50 about 4.0 (90,360 rows over 22,499 seconds, up to 10 in one); NIFTY BANK about 2.0 (45,272 over 22,498, up to 5); no tiebreaker among rows sharing a second | measured, one day |
| Session seconds with no row | NIFTY 50 one (14:16:23); NIFTY BANK two (10:48:06, 14:16:23) | measured, one day |
| Rows outside the session (26 May 2026 is a regular 09:15–15:29 session in `pull::calendar`) | present: first rows 09:07:03 and 09:07:04, last 16:09:13; 1,908 and 955 rows before 09:15:00, 9,418 and 4,709 from 15:30:00 on | measured, one day |
| Row order | the time column steps **backward** 25 times in each index file, by one or two seconds, roughly every fifteen minutes; `pull::fold` refuses such a step rather than sorting (D-0802 consequence 5) | measured, one day; what a backward step means UNVERIFIED |
| Range, minute against second | mean `LTP` high minus low over the session: NIFTY 50 8.68 points per minute, 1.03 per second; NIFTY BANK 29.74 per minute, 2.27 per second | measured, one day |
| One cash-equity file | `STOCKS/2026/MAY_2026/GFDLCM_STOCK_TICK_26052026/RELIANCE.NSE.csv` (3,429 files in that folder): the same ten-name header, 17,626 rows, a nonzero `BuyPrice` or `SellPrice` on all but one row, a nonzero `LTQ` on 15,816 | measured, one day, one file |
| Folder names present | 2,001 `GFDLCM_INDICES_TICK_<ddmmyyyy>` folders whose names run from 3 Sep 2018 to 30 Sep 2026, each with both index files, and 2,001 `GFDLCM_STOCK_TICK_<ddmmyyyy>` folders | counted by name only; contents of every other day unread |

**Still UNVERIFIED.**

| Needed fact | Lane |
|---|---|
| Whether the layout, density, gaps and backward steps above hold on every other day, for every needed series | UNVERIFIED — one day read |
| Whether every needed day is present and non-empty across the tested history | operator-stated (above); per day UNVERIFIED, so a window with no GDFL seconds still refuses at run time |
| Clock basis of the CSV `Time` column (exchange time, vendor receive time or other), and whether `hh:mm:ss` means `[ss, ss+1)` | UNVERIFIED — D-0015 records GDFL epoch-second fields `LastTradeTime` and `ServerTime` (documented); neither name is in the CSV |
| Alignment of GDFL seconds with Zerodha minutes and folded rungs | **Partly measured since (7 Oct 2026, D-3324).** NIFTY on 2026-09-24: Zerodha `1day`, the last Zerodha `1min` bar and the last GDFL `1s` bar all close at 23,063.10, and open, high and low agree; on 2019-06-14 open, high and low agree and the close does not (official 11,823.30, last GDFL second 11,809.50). Read through a read-only shadow of the operator's store, `bars/{zerodha,gdfl}/NSE/INDEX/NIFTY/`. Minute-by-minute alignment over the whole history is still unmeasured |
| GDFL's naming for every F&O underlying's cash series, and its mapping to `InstrumentKey` | UNVERIFIED here; measured over the whole copy in "GDFL capital-market tick files" below, and the index names sourced in "Index aliases and symbol changes" below (D-3332) |
| Whether GDFL's and Zerodha's price series agree in level, including corporate-action adjustment for the cash equities | **MEASURED (6–7 Oct 2026, D-3323, D-3324): an index level is not adjusted, and 101 of 208 shares differ by the vendor's back-adjustment.** §4z gives the share counts, store paths and examples. Index levels: NIFTY open, high and low agree on all 19 days of 2019-03 and the close on none (official against last print), and the open agrees on 12 of 17 common days of 2026-09 (the /db audit); every field agrees on 2026-09-24 (row above) |
| Whether GDFL's own cash-equity series is adjusted for splits, bonuses or other corporate actions | **MEASURED: raw, as traded (7 Oct 2026, D-3323).** GDFL's price steps by the action's factor on the ex-date (BRITANNIA 2018-11-28 last print ₹5,991.00 before a 1:1 bonus; BAJFINANCE first print ₹9,150.00 on 2025-06-13 and ₹956.00 on 2025-06-16, the ex-date), its option strikes are renamed to adjusted values on the ex-date (D-3331), and its volume is the sum of traded quantities. Read through the shadow store, `bars/gdfl/NSE/CASH/<SYM>/1s/`. No vendor page states it |
| The price of the data | not recorded; the operator states it is in hand |
| Licence terms for this use | UNVERIFIED |

Two untracked files in the main checkout, `GFDLNFO_TICK_06012019.zip` and
`GFDLNFO_TICK_07012019.zip`, were seen on 1 October 2026 at 450 bytes each with
one 18-byte two-column `ts,ltp` member. They are outside version control, so no
reader of this repository can re-check them, and they may no longer exist.
Nothing records where they came from, and they are not evidence for any row
above. D-0045 first recorded that GDFL appears nowhere in this charter, and
D-0118 named the same gap for GDFL's granularity floor; the operator statement
and the one measured day above narrow that gap and do not close it.

## 4h. Risk-free rate — the RBI 91-day Treasury Bill cut-off yield (D-2810)

**Operator choice, 4 October 2026.** For GDFL option IV and greeks, the rate is
the RBI 91-day Treasury Bill yield, taken as a dated series and cited here.
Before this section, no page of this charter recorded a rate.
`pull::pricing::RateSource::Charter` therefore had nothing it could cite, and
the only rate in use was one supplied by the operator with each request.

**Where the values come from.** Each weekly value comes from RBI's own press
release *"91-Day ... T-Bill Auction Result: Cut-off"*, on
`https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=<id>`. The
value is the line *"Cut-off Price and Implicit Yield at Cut-Off Price"* for
the 91-day column. Each value is cross-checked against the same date's
*"Treasury Bills: Full Auction Result"* release.

**The values are not in this repository.** The operator keeps them in an
eight-column tab-separated table outside the tree, and it is read at run time
from a path the caller passes (`pull::riskfree::Series::read`, no default). The
table's header is `pull::riskfree::HEADER`. Every row must name its RBI release
in `source_url`. The table's SHA-256 travels with every rate
(`pull::riskfree::Series::sha256`). Three sample rows are quoted only so a
reader can check the reading:

| Release | Cut-off price | Implicit yield | Source |
|---|---|---|---|
| 2018-08-01 | ₹98.35 | 6.7292% | [prid 44632](https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=44632) |
| 2020-03-24 | ₹98.7000 | 5.2830% | [prid 49565](https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=49565) |
| 2026-09-30 | ₹98.6425 | 5.5199% | [prid 63694](https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=63694) |

| Fact | Value | Lane |
|---|---|---|
| What the implicit yield is | A simple yield on price, annualised by 365/91: `(100 − P)/P × 365/91`. It is not a discount yield. Every one of the operator's 419 rows from 2018-08-01 to 2026-09-30 matches this to within one unit in the fourth decimal, and the parser refuses any row that does not. | measured against RBI's published pairs, 4 Oct 2026 |
| When a result becomes known | Results are announced on the auction day itself, after competitive bidding closes. The source says: *"Results will be announced on the same day"* ([prid 44674](https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=44674)). Bidding closes at 12:00 noon in 2018 and at 11:30 am later. The time of the announcement is **not** stated. | documented; the announcement time is UNVERIFIED |
| Weeks with no 91-day value | Some weeks have no value: auctions cancelled (2024-09-18 and 2024-09-25, [prid 58686](https://www.rbi.org.in/Scripts/BS_PressReleaseDisplay.aspx?prid=58686)), 91-day cut-off "NA" (2023-03-29, 2025-02-20, 2026-03-25), and three weeks with no auction announced. | documented per release |
| Continuous compounding | A simple rate `y` over `τ = 91/365` years becomes `r = ln(1 + y·τ)/τ`. This is the rule `R_c = m·ln(1 + R_m/m)` from J. C. Hull, *Options, Futures, and Other Derivatives*, chapter 4 "Interest Rates", subsection "Continuous Compounding" (the section number varies by edition), with `m = 365/91`. | textbook identity |
| Year basis | 365 calendar days. This is the same basis as `pull::tenor::YearBasis::Calendar365`, and the same basis RBI uses to annualise the yield. | follows from the row above |

**The as-of rule, and why it is "strictly before".** A trade on day `D` uses
the latest result whose release is dated **strictly before** `D`. A result
dated `D` is announced during `D`'s session, at a time the release does not
state, so using it on `D` would be look-ahead (`CLAUDE.md` §3 rule 7). A
release more than 14 days old is refused as stale, not carried forward
(`pull::riskfree::MAX_AGE_DAYS`). A day on or before the first release is
refused as before the series. D-2810.

## 5. Run identity

```
blake3(
  strategy_mask ‖ direction ‖ instrument ‖ timeframe ‖ mode ‖
  canonical_params ‖ data_digest ‖ vocab_version ‖ commit_sha
)
```

`data_digest` is a streamed digest over **every field of every loaded bar** —
not a sample, not a count-and-endpoints fingerprint. One differing bar re-keys
the identity. That is the point.

No computation runs without that identity recorded first.

---

## 6. The sweep, in one paragraph

Load a contiguous slice of 1-minute bars for one instrument. Compute the 74
condition bits per bar, once. Walk the combination ladder from k=1: at each
level, join the previous frequent frontier with itself, prune any candidate
whose (k−1)-subsets are not all frequent, evaluate the survivors against the
bar-bit column, keep those with at least `min_hits` hits, and recurse. Stop
when a level produces nothing. Rank the survivors, persist a bounded top set,
and record the identity.

Nothing about that paragraph has a tunable depth.

## 7. Dated regular-session completeness — verified 2026-09-05

The NSE [CAS session rules](https://www.nseindia.com/static/products-services/closing-auction-session)
distinguish derivatives (continuous through 15:40), non-CAS cash (through
15:30), and CAS-eligible cash (continuous trading ends 15:15, followed by
auction phases). The dated derivatives schedule already recorded in
`pull::vendor` applies the extension from 2026-08-03, also confirmed by
[Zerodha's dated implementation notice](https://zerodha.com/z-connect/general/everything-you-need-to-know-about-closing-auction-session-cas).
Opening-stamped
one-minute derivatives bars therefore end at 15:29 before the change and
15:39 afterward on regular trading days. These are expected session bounds,
not proof of vendor coverage. Exceptional sessions retain their own calendar
classification. Generic cash venue hours do not prove per-instrument CAS
eligibility; treating auction observations as continuous candles remains
UNVERIFIED and must not be represented as resolved.

## 8. Zerodha identity evidence — verified 2026-09-05

The [Kite instruments specification](https://kite.trade/docs/connect/v3/market-quotes/)
lists twelve CSV columns, without ISIN. It recommends exchange plus trading
symbol as a storage key and warns that derivative numeric tokens may be reused
after expiry. A valid parsed token therefore does not establish the
vendor-specific cash ISIN proof required by D-0511. Refreshing the same schema
does not supply that proof. This is a local assurance-policy limitation, not a
claim that the vendor's historical API cannot serve cash candles.

D-0517 keeps the two assertions separate: the exact Zerodha NSE cash token,
and the exchange ISIN corroborated by an exact, unambiguous ISIN-bearing
independent master listing. That cross-check is current-snapshot evidence;
it does not claim that Zerodha supplied the ISIN or prove historical symbol
changes or historical F&O membership. No ISIN is synthesized in its crawl.

D-3314 adds a third Zerodha-only assurance level that reads no other
vendor's master. Its one source is the exchange's own file, already recorded
in §4c:

| Assurance level | Evidence it requires | Source | What it does not claim |
|---|---|---|---|
| `zerodha_nse_checked` | Exactly one Zerodha NSE cash EQ row for the symbol, with a token no other row shares; NSE's `ind_niftytotalmarket_list.csv` landed on the same IST day as the Zerodha dump; that file lists the symbol in series `EQ`; the ISIN it prints beside the symbol equals the member's ISIN in `core::universe` | `https://nsearchives.nseindia.com/content/indices/ind_niftytotalmarket_list.csv` (§4c, fetched 2026-08-11 with header `Company Name,Industry,Symbol,Series,ISIN Code`) and the Kite instruments specification above | That Zerodha supplied an ISIN (none is written into Zerodha's row, D-0517); historical renames, corporate actions or membership; when NSE last changed the file, which carries no date of its own |

Whether NSE updates this constituent file when a constituent's symbol or
ISIN changes between rebalances is **UNVERIFIED**. The landing day says when
the file was fetched, not how current its rows are.

## 9. Dated cash closing-auction eligibility — verified 2026-09-06

[NSE/CMTR/73845](https://nsearchives.nseindia.com/content/circulars/CMTR73845.zip)
(22 April 2026), Annexure A, names the MII CSV column
`ElgbltyClsgAuctnSsn`: 0 is not eligible and 1 is eligible, effective
3 August 2026. The circular payload is a ZIP, not a PDF at the same basename.
[NSE/CMTR/74466](https://nsearchives.nseindia.com/content/circulars/CMTR74466.zip)
(29 May 2026), page 6, directs members to the dated
`NSE_CM_security_ddmmyyyy.csv.gz` files. The file date, not its `UpdDt` cell,
binds this evidence to a trading date.

All 25 weekday masters from 2026-08-03 through 2026-09-04 were retrieved from
NSE archives, with HTTP 200 and valid gzip streams. Actual endpoint examples:
[3 August](https://nsearchives.nseindia.com//content/cm/NSE_CM_security_03082026.csv.gz)
and [4 September](https://nsearchives.nseindia.com//content/cm/NSE_CM_security_04092026.csv.gz).
Both contain exact EQ entries HINDALCO/INE038A01020 with flag 1 and
20MICRONS/INE144J01027 with flag 0. Runtime joins must check date, series,
symbol and ISIN; missing rows must never become flag 0.

This verifies the scheduled continuous close (15:15 eligible, 15:30
ineligible), not that every scheduled minute traded. Circular 74466 §A
also names scheme-of-arrangement special-preopen and circuit-halt exceptions.
Their occurrence remains UNVERIFIED; gaps or exceptional sessions must stay
visible, with no fabricated bars. Auction execution-report absence does not
establish non-eligibility. The cash policy does not change index hours or the
separately cited derivatives schedule.

**What both feeds hold from 2026-08-03 — measured 7 Oct 2026 (D-3320).** Read
through a read-only shadow of the operator's store. Zerodha: RELIANCE `1min`
holds 360 bars on 2026-09-24, 09:15 to 15:14, and 375 on 2026-07-31, 09:15 to
15:29 (`bars/zerodha/NSE/CASH/RELIANCE/1min/`); the landing drops an eligible
share's bars from 15:15 (`fetch::cash_close_verdict`). GDFL: RELIANCE `1s` on
2026-09-24 holds its last continuous bar at 15:14:59, one bar at 15:15:00
(volume 65), and one at 15:28:07 (volume 346,818, close ₹1,219.20)
(`bars/gdfl/NSE/CASH/RELIANCE/1s/`). ₹1,219.20 is Zerodha's `1day` close of
that day. The /db audit found the same single late print, with zero bid and
ask and equal to the Zerodha day close, for AXISBANK and BAJFINANCE that day,
on every session from 2026-08-03 and on none before. Calling that print the
closing auction's is an inference from those measurements and §9's rule; no
vendor page labels it. It is kept as an auction observation, never as a
continuous trade (D-3320). Whether an index level freezes during the auction
is UNVERIFIED.

### FORCEMOT NSE interruption — verified 2026-09-06

[NSE circular CML58560](https://archives.nseindia.com/content/circulars/CML58560.pdf)
withdraws FORCEMOT (INE451A01017) from the permitted-to-trade category with
effect from 2023-10-26, after the close on 2023-10-25. The company's
[filing including NSE circular CML60618](https://www.forcemotors.com/wp-content/uploads/2025/02/Announcement-under-Regulation-30-for-Listing-on-NSE.pdf)
admits the same symbol and ISIN as an EQ listing from 2024-02-14. The missing
November 2023, December 2023 and January 2024 daily files therefore overlap
a documented NSE trading interruption, not evidence that another identical
vendor retry will produce bars. Acquisition may request the two available
periods separately, retaining this interval as explicitly not applicable to
NSE trading. No BSE substitution or synthetic bars is authorized. This dated
evidence is not yet wired into general instrument-calendar classification;
the whole-window application prerequisite still refuses the interior holes.

## Calendar extension evidence reviewed 2026-09-06

Primary capital-market trading-holiday authority:
[NSE/CMTR/71775, 12 December 2025](https://nsearchives.nseindia.com/content/circulars/CMTR71775.pdf).
Its trading-holiday list has no holiday in 2026-08-22 through 2026-09-04.
Do not substitute settlement-holiday tables: those answer a different question.
The regular equity trading-week and hours rule is published at
[NSE market timings](https://www.nseindia.com/resources/exchange-communication-holidays/).
This evidence is used only for the bounded measured interval, not a perpetual
weekday fallback or permission to infer future special sessions.

Local Zerodha NIFTY and BANKNIFTY minute records were independently read for
August 22 through September 4. Each index has 3,750 unique minute-aligned
records across ten dates: August 24–28, August 31, September 1–4. Every date
has exactly 375 records spanning 09:15–15:29 IST. Four read-only response
snapshots are retained in the local audit calendar-extension directory.
This corroborates the regular index windows for this interval only. It does
not prove broker history universally complete, independently sourced feeds,
historical stock identity, or stock-specific cash-auction eligibility; the
existing dated eligibility authority remains necessary for cash stocks.

## Historical circuit-breaker evidence boundary — 2026-09-06

[Zerodha's contemporaneous March 13, 2020 bulletin](https://zerodha.com/marketintel/bulletin/249418/trading-halted-at-the-exchange)
reports a market halt followed by a pre-open session. Its 09:26 publication
timestamp is not proof of the exact halt's start minute. The general
[NSE circuit-breaker rule](https://www.nse.in/products-services/equity-market-circuit-breakers)
describes a 45-minute halt plus 15-minute reopening auction for a 10% trigger
before 13:00. A rule is not a dated event log.

Exact historical source-minute windows on March 13 and March 23, 2020 remain
**UNVERIFIED** in this reconciliation. The ABB stored-grid audit names gaps on
those dates under the existing regular-day model; this does not establish
provider fault. Do not infer precise exchange windows from one stock's missing
timestamps, or count every such absence as a minute the provider must supply.
No calendar override is made from publication timestamps or news summaries.

**Searched again on 7 Oct 2026, and still UNVERIFIED (D-3320).** A search of
nseindia.com, nsearchives.nseindia.com, archives.nseindia.com and
sebi.gov.in for a dated NSE circular or press release of the 13 March or
23 March 2020 halt found none; only the general circuit-breaker rule page
above and news reports, which this charter does not take as a source. What
the store holds is measured, not sourced: on 2020-03-13 GDFL NIFTY `1s` holds
3,567 bars from 09:21:30 to 10:20:59 with volume 0, frozen at 8,624.05 to
10:05:59, then varying values to 10:13, and no stock in either feed has a bar
from 09:21:08 to 10:21:00; on 2020-03-23 every stock is absent from 09:59 to
10:56 in both feeds (the /db audit of 6 Oct 2026, over
`bars/{gdfl,zerodha}/NSE/{INDEX,CASH}/`). D-3320 carries both days in
`pull::calendar` as **suspected** halts with those measured windows (WP-14);
they stay suspected until a circular is recorded here, and a suspected halt's
minutes are neither owed nor excused (D-3336).

**Re-measured on 9 Oct 2026 from the GDFL day files (D-3320)**, counts and
times only, read as the Muhurat measurement in §3 was, from the capital-market
day zips and the options day zips (`<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.zip`)
of the vendor archives. On 2020-03-13 RELIANCE, HDFCBANK, INFY, SBIN, TCS and
ICICIBANK trade last at 09:21:08 and next at 10:21:00; across the whole stock
day file 25 trades print after 09:21:08, the last at 09:22:03, five of them in
the 09:22 minute and all five in names outside `core::universe::FNO_UNDERLYINGS`,
and the next is at 10:21:00. NIFTY 50 and NIFTY BANK change last at 09:21:09 and next, after
the auction, at 10:21:01; NIFTY 50 changes in every minute 10:06–10:13 and in
no other minute from 09:22 to 10:20. Every NIFTY and BANKNIFTY option trades
last by 09:21:44 and next at 10:21:00; eight trades of stock options print in
the 09:22 minute, the last at 09:22:58. On 2020-03-23 every share in the stock
day file trades last at 09:58:07 and next at 10:57:00; NIFTY 50 changes last
at 09:58:07 and NIFTY BANK at 09:58:08, both next at 10:57:01, and NIFTY 50
changes in no minute from 09:59 to 10:56 except 10:42–10:49; every NIFTY and
BANKNIFTY option trades last at 09:58:00 and next at 10:57:00, and stock
options last at 09:58:37. Each is a 45-minute halt and a 15-minute reopening
call auction, `[10:06, 10:21)` and `[10:42, 10:57)`, in the shape of the NSE
rule above; that shape is an inference from these times, not a record. On
2021-02-24 RELIANCE, HDFCBANK, INFY, SBIN and TCS trade last at 11:43:00 and
ICICIBANK at 11:42:59, three minutes past SEBI's 11:40, and all six next at
15:45:00; the SEBI order stays the record.

## VWAP applicability evidence — reviewed 2026-09-06

[NSE Clearing's equity-derivatives settlement mechanism](https://www.nseclearing.in/clearing-settlement/equity-derivatives/settlement-mechanism)
explicitly uses a futures contract's volume-weighted traded price. This
establishes that futures VWAP is applicable to contract trades; it does not
authorize futures sweeps here or prescribe this engine's session indicator.
[NSE capital-market circular CMTR5588](https://nsearchives.nseindia.com/content/circulars/cmtr5588.htm),
dated 2004-11-10, gives traded-value divided by traded-quantity VWAP for a
cash security. Only that arithmetic/applicability is cited; its historical
tax rates and settlement rules are not adopted as current rules.

The engine's OHLCV typical-price approximation, sigma conventions and
spot-index abstention remain the explicit implementation choices in D-0507
and D-0526, not exchange certifications of the engine's calculated values.
Prices and traded volumes must belong to the same stock or exact contract;
no constituent-volume or futures-volume replacement grants a spot index VWAP.

## Finite qualification statistical sources — reviewed 2026-09-07

[Romano and Wolf, Efficient Computation of Adjusted p-Values for
Resampling-Based Stepdown Multiple Testing](https://www.econ.uzh.ch/apps/workingpapers/wp/econwp219.pdf)
provides the strict-exceedance plus-one probability and monotone stepdown
construction used by the shared procedure. Its validity conditions depend on
the testing and resampling setting. It does not establish universal validity
for financial return sequences or certify this implementation.

[Tian and Ramdas, Online control of the familywise error rate](https://arxiv.org/html/1910.04900)
discusses the distinction between fixed and sequential testing families and
allocation of testing levels. This implementation adopts a finite, predeclared
eight-timeframe scope, not an unbounded online acceptance guarantee. Equal
allocation and the zero-conservative wrapper are explicit repository choices
in D-0542 and D-0544; resampling assumptions remain necessary.

## GitHub Actions workflow size — reviewed 2026-09-19

[GitHub Actions limits](https://docs.github.com/en/actions/reference/limits#workflow-file-size)
states that each workflow file must be 500 KB or smaller to trigger runs.
The 515,779-byte CI workflow at `19187a4e` exceeded that limit. D-0608 moves
explanatory YAML comments into a document, leaving the executable workflow
unchanged at 497,119 bytes. This restores eligibility to run; it does not
establish that the workflow's checks pass.

### BLAKE3 full compression reference — verified 2026-09-20

The upstream [BLAKE3 test vectors](https://raw.githubusercontent.com/BLAKE3-team/BLAKE3/master/test_vectors/test_vectors.json)
record extended unkeyed output for empty input. Core's
`complete_compression_output_matches_the_official_empty_vector` compares the
first 64 bytes, including the second half unused by its public 32-byte digest.
The expected bytes come from upstream, not this implementation.

## GDFL capital-market tick files — measured 2026-10-03 (D-0808, D-0812)

`pull::gdfl_cm` and `pull::gdfl_archive` name GDFL folder layouts, file names,
headers and an archive layout in source. `CLAUDE.md` §3 rule 1 requires each
such fact to trace to a source recorded here, and until this section none did:
D-0808 and the module docs cited the one-second fill design, which is held
outside this repository. **No row of this section is a vendor statement.**
Each was read on 2 and 3 October 2026 (IST), read-only, from the operator's
extracted GDFL copy (a directory named `NSE_Tick_2018-09-01_to_2026-09-24`)
and from the vendor archive it was extracted from (a file named
`NSE (Stock+Indices)_01.09.2018 to 24.09.2026_Tick.zip`), with `ls`, `find`,
`head`, `tail -c`, `xxd`, `od`, `unzip -v`, `grep -c` and `awk`. Where they are mounted is
machine layout and is not recorded. Anyone with those two copies can repeat
each figure. **No GDFL row is quoted here or anywhere in this repository**:
the repository is public and the licence below is UNVERIFIED.

The D-0802 draft (branch `docs/d-0802-tick-precise-fills`, not landed when
this was written) adds a section 4g recording one measured day and listing
"GDFL's naming for every F&O underlying's cash series" as UNVERIFIED. The
rows below measure that naming over the whole copy for the files the reader
maps. Whichever lands second must reconcile the two (`docs/07-plan.md`, the
GDFL capital-market rows).

| Fact | Value | How measured | Lane |
|---|---|---|---|
| Day folders | `INDICES/<yyyy>/<MON_yyyy>/GFDLCM_INDICES_TICK_<ddmmyyyy>/` and `STOCKS/<yyyy>/<MON_yyyy>/GFDLCM_STOCK_TICK_<ddmmyyyy>/`, day first; 2,001 folders in each tree, first `03092018`, last `30092026`; no folder of either tree has any other name | `find` of the third level, `grep -v -E` of the exact pattern: 0 | measured, whole copy |
| Index file names | every one of 159,023 index files is `<NAME>.NSE_IDX.csv` or `<NAME>.NSE_IDX.CSV` | `find INDICES -type f`, excluding the two spellings: 0 left | measured, whole copy |
| Stock file names | every one of 4,886,181 stock files is `<SYM>.NSE.csv`; none is spelled `.CSV` | `find STOCKS -type f`, excluding `*.NSE.csv`: 0 left | measured, whole copy |
| `.CSV` spelling | 474 files, all in nine September 2018 index folders (3, 5, 6, 7, 11, 12, 14, 17 and 18 September), every file of those folders; every other folder spells `.csv` | `find -name '*.CSV'` (case-sensitive), grouped by folder | measured, whole copy |
| Case on the volume | the copy's volume is case-insensitive: a `.CSV` glob matches `.csv` files | `test -e` of `NIFTY 50.NSE_IDX.CSV` in the `.csv`-only folder of 2024-04-01 succeeds | measured |
| The two index tickers the sweep reads | `NIFTY 50.NSE_IDX` and `NIFTY BANK.NSE_IDX`, present in every index folder; `INDIA VIX.NSE_IDX` beside them | `ls` per folder | measured, whole copy |
| A share's file | `<SYM>.NSE.csv`, `SYM` the NSE trading symbol; of the 208 F&O shares in `core::universe` under today's symbols, 208 have one on 2026-09-24, 195 on 2024-04-01, 175 on 2021-04-01, 159 on 2019-01-01. The rest are later listings and renames | `grep -qxF "<SYM>.NSE.csv"` against each folder's `ls` | measured, four days; a rename's earlier symbol UNVERIFIED |
| Headers | over the 2,001 `NIFTY 50` index files: 1,984 open with `Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest` and 17 with the same names but `Open Interest` (with a space) last; those 17 are dated 2018-09-03 to 2018-10-04 (14 September and 3 October folders), and every NIFTY 50 file outside that span opens with the first | `head -1`, `tr -d '\r'`, counted | measured, NIFTY 50 files only |
| Final newline | 17 of the 79 `NIFTY 50` files of 2018 do not end in a newline: the same 17 files as the spaced header | `tail -c1 \| xxd -p` | measured, NIFTY 50 files of 2018 |
| Third decimal in the LTP | INDIA VIX 2024-04-01: 21,870 rows with three or more decimals. NIFTY BANK 2020-10-01 and SBIN 2022-04-01: 0 LTPs that are not a plain decimal of at most two places. Every NIFTY 50 and NIFTY BANK file of the copy (4,002 files, 147,837,877 rows after their headers; 2026-10-03): 0 LTPs that are not a plain decimal of at most two places. Equities beyond SBIN 2022-04-01 are not measured | `awk -F,` on field 4; the whole-copy pass `find INDICES -name 'NIFTY 50.NSE_IDX.*' -o -name 'NIFTY BANK.NSE_IDX.*'` into one `awk -F, 'FNR>1'` testing `^[0-9]+(\.[0-9][0-9]?)?$` | measured: three files, then both swept indices over the whole copy |
| An LTP of zero | over every `NIFTY 50` and `NIFTY BANK` file: only 2022-03-07 holds one. NIFTY 50: 4 rows (lines 4,820 and 4,821 at 10:25:41, the first second after a gap in stamps from 10:21:54; lines 44,168 and 44,169 at 22:27:05); NIFTY BANK: 2 rows (lines 3,495 and 3,496 at 10:56:51, the first second after a gap from 10:09:55). Each zero row shares its second with a non-zero row before it | `grep -c -E` for a zero LTP field, then `grep -n` | measured, whole copy; what a zero means UNVERIFIED |
| Largest files | NIFTY BANK 2026-09-24: 6,141,980 bytes, 101,788 rows; largest stock file that day `SSRETAIL.NSE.csv`, 1,430,976 bytes; `GFDLCM_STOCK_TICK_01042024` holds 2,746 names | `ls -l`, `wc -l`, `ls \| wc -l` | measured, two days |
| Archive layout | the archive is 128,918,864,518 bytes. Its end record names 4,209 entries and a 493,028-byte central directory and saturates the directory offset (`FFFFFFFF`); a ZIP64 locator and ZIP64 end record precede it and carry the offset; the last central record carries the local-header offset in an 8-byte ZIP64 extra block (id 1) followed by a `UT` block | `tail -c 120 \| xxd` | measured, one archive |
| Archive directory, walked | the 4,209 central records, walked from the directory's first byte by their own name, extra and comment lengths, end exactly at its 493,028th byte. The first 256 carry a plain 32-bit local-header offset and a 9-byte `UT` extra block only; the other 3,953 saturate the local-header offset alone and carry it in a 21-byte extra field, an 8-byte ZIP64 block (id 1) then the `UT` block. No record saturates either size, and every record's method is 0 (stored) | the directory's bytes dumped with `od -An -tu1 -v` and walked with `awk` | measured, one archive (round-3 review) |
| Archive entries | 4,209: 215 folder entries (names ending `/`), 1,997 index day zips `INDICES/<yyyy>/<MON_yyyy>/GFDLCM_INDICES_TICK_<ddmmyyyy>.zip` and 1,997 stock day zips `STOCKS/<yyyy>/<MON_yyyy>/GFDLCM_STOCK_TICK_<ddmmyyyy>.zip`; every one `Stored`, its length equal to its size. The copy's four folders of each tree after 2026-09-24 have no day zip | `unzip -v` of the archive, `grep -c -E` of the two day-zip patterns | measured, one archive |
| Files per day folder | 50 to 141 in an index folder and 1,764 to 3,710 in a stock folder, over the 2,001 folders of each tree | `ls \| wc -l` per folder | measured, whole copy |
| Line endings | every line of the 4,002 `NIFTY 50` and `NIFTY BANK` files ends CRLF, header included, except the unterminated last line of 34 files: the 17 `NIFTY 50` files of 2018 with no final newline and the `NIFTY BANK` file of each of the same 17 days. 147,837,877 rows after the headers | `awk` counting lines that end `\r`, `tail -c1 \| xxd -p` over the 158 index files of 2018 | measured, both swept indices, whole copy |
| First row of an index file | stamped 09:07:01 to 09:09:52 in 3,978 of the 4,002 `NIFTY 50` and `NIFTY BANK` files. The other 24: the Muhurat evenings 2018-11-07 (17:22:56), 2019-10-27, 2020-11-14, 2021-11-04, 2022-10-24 and 2023-11-12 (18:07:09 to 18:07:39) and 2024-11-01 (17:52:46); 2025-10-21 (13:37:56); and 2018-10-08 to 2018-10-11 (07:45:22 to 08:20:05) | `awk`, field 3 of the first row | measured, both swept indices, whole copy |
| Largest stamp of an index file | 13:23:27 to 22:27:05 over the 4,002 files, and in every one the last row in file order holds it. Four are before 15:30:00, the files of the two disaster-recovery Saturdays 2024-03-02 and 2024-05-18 (13:23:27 to 13:36:30; that section above), which `pull::gdfl_cm::session_gate` refuses before any decode. 22 more are before 16:00:00, both files of 2018-10-17, 2018-11-21, 2019-02-05, -11, -18, -19, -25, -27, 2019-05-20, 2019-05-30 and 2025-01-07, from 15:31:59 (2019-05-30, whose files hold only 1,568 and 1,557 rows) to 15:59:17. Every other is 16:00:17 or later. So on every regular day both files print at or past 15:31:59, which is the evidence for refusing one that stops inside the session (`IndexEndsInSession`) | `awk`, the running maximum of field 3 | measured, both swept indices, whole copy |
| Quote, LTQ and open-interest fields of an index row | 0 in all six fields (`BuyPrice` to `OpenInterest`) on every ten-field row of the 4,002 `NIFTY 50` and `NIFTY BANK` files: no row has any of them otherwise | `awk -F,`, fields 5 to 10 | measured, both swept indices, whole copy |
| Rows stamped in UTC | 227 rows of the 4,002 files are stamped more than 3,000 s below the running maximum, all in October 2020 (113 `NIFTY 50`, 114 `NIFTY BANK`) on 13, 15, 16, 19, 20, 21, 22, 26, 28, 29 and 30 October. Their back-step from the running maximum, taken over re-stamped (+19,800 s) stamps: 19,798 s for 4, 19,799 for 159, 19,800 for 58, 19,801 for 6. No row elsewhere steps back that far | `awk` over all 4,002 files, then over the 22 files of those days | measured, both swept indices, whole copy |
| Member methods | every day zip is STORED (method 0) in the outer archive, so each is one contiguous byte range; the members inside a day zip are DEFLATED (method 8), e.g. `GFDLCM_STOCK_TICK_19012023/<NAME>.NSE.csv` in `STOCKS/2023/JAN_2023/GFDLCM_STOCK_TICK_19012023.zip`, and `INDICES/2026/JUN_2026/GFDLCM_INDICES_TICK_11062026.zip` is stored | the GDFL build thread's read of the central directories, relayed 2026-10-04 (D-2800); the stored day zips were already relied on by D-0812 | measured (relayed) |
| The verified tick store's format | version 1 (`BRTXTS01`, frozen 2026-10-04): one `.bts` file per vendor day zip at `cm/<TREE>/<yyyy>/<MON_yyyy>/<day folder>.bts` under the store root, holding every entry of that zip in its central-directory order with the zip's own size, CRC-32 and DOS time per entry; file layout, footer, index and block encodings (directory, columnar, raw zstd) as its §3 and §4 state; `*.bts.tmp` incomplete and ignored. Specified in `/Volumes/WD_BLACK/brutex/tickstore-data/FORMAT.md`, sha256 `6edc62a772b257a2aa1b45c0785a23936d9bff47b87648bbb1d9a84ab7063259` | read 2026-10-04, `shasum -a 256`; the operator's specification, not a vendor statement (D-2800, D-2801) | specification |
| Tick store against the zip | the 21 index day files of August 2026 (`cm/INDICES/2026/AUG_2026/`): all 2,982 entries carry the zip's names, order, sizes and CRC-32s, and all 2,961 files (11,048,377,844 bytes) rebuild byte for byte to what the zip source inflates | a read-only probe through `pull::gdfl_tickstore::TickStore` and `pull::gdfl_archive::Archive`, both `CmSource`s, 2026-10-04; the probe was not committed and no byte of either copy is | measured, August 2026 index days (D-2801) |
| What the store holds from the archive, and at which rung | **One second only (D-2802), 2018-09-03 to 2026-09-24.** GDFL NIFTY's first stored bar is 2018-09-03 09:15:00 and its last 2026-09-24 15:29:59, in 97 month files each for NIFTY and BANKNIFTY (`bars/gdfl/NSE/INDEX/NIFTY/1s/` and `.../BANKNIFTY/1s/`); the import journal closes 1,985 index days and 1,986 stock days `done`; the store holds 208 share series and option series for 219 underlyings. The tick store also holds 2026-09-25, 28, 29 and 30, which the zip archive lacks (DB-G56); the store's months come from the zip and stop at 2026-09-24. Read on 7 Oct 2026, read-only, from the operator's store and `imports/gdfl.journal` | `ls` and `wc` of the month files; one-row reads through a read-only shadow; `awk` over the journal | measured, 7 Oct 2026 |
| The options import's state when the operator stopped it | **Stopped at 23:23 IST on 6 Oct 2026, inside 2020-09-16** (the journal's last line is that day's `begin`, unclosed). 288 options days are `done` (2018-09-03 to 2019-11-13, and 2019-12-27); 212 are `incomplete`: 2018-11-06 and 2018-11-09 (vendor refusals) and every options day from 2019-11-14 to 2020-09-15 (210), whose bars are on disk and whose census append was refused at the manifest's ceiling (D-3316, D-3317) | `awk` over `imports/gdfl.journal`, 7 Oct 2026 | measured, 7 Oct 2026 |
| Licence terms for keeping a converted copy, and for fixtures | UNVERIFIED (D-0802 consequence 10, on its unlanded branch). So no GDFL row is committed (D-0808 item 9) | — | UNVERIFIED |

## NSE trading holidays 2018-2019 and September-October 2026 — read 2026-10-04 (D-2805)

Each circular was fetched from `nsearchives.nseindia.com` and read on
2026-10-04; the dates below are copied from it.

| Circular | Date | Covers | Weekday trading holidays it lists (in the calendar's span) |
|---|---|---|---|
| [NSE/CMTR/36475](https://nsearchives.nseindia.com/content/circulars/CMTR36475.pdf), Circular Ref. 71/2017 | 12 Dec 2017 | calendar year 2018 | 26 Jan, 13 Feb, 2 Mar, 29 Mar, 30 Mar, 1 May, 15 Aug, 22 Aug, 13 Sep, 20 Sep, 2 Oct, 18 Oct, 7 Nov (Diwali-Laxmi Pujan, "Muhurat Trading will be conducted"), 8 Nov, 23 Nov, 25 Dec |
| [NSE/CMTR/39612](https://nsearchives.nseindia.com/content/circulars/CMTR39612.pdf), Circular Ref. 115/2018 | 7 Dec 2018 | calendar year 2019 | 4 Mar, 21 Mar, 17 Apr, 19 Apr, 1 May, 5 Jun, 12 Aug, 15 Aug, 2 Sep, 10 Sep, 2 Oct, 8 Oct, 28 Oct, 12 Nov, 25 Dec; Muhurat on Sunday 27 Oct |
| [NSE/CMTR/40536](https://nsearchives.nseindia.com/content/circulars/CMTR40536.pdf), Circular Ref. 30/2019 | 26 Mar 2019 | partial modification of 39612 | 29 Apr 2019, Parliamentary Elections in Mumbai |
| [NSE/MSD/42319](https://nsearchives.nseindia.com/content/circulars/MSD42319.pdf), Circular Ref. 53/2019 | 7 Oct 2019 | CM, FO and CD segments | 21 Oct 2019, Assembly Elections in Maharashtra |
| [NSE/CMTR/71775](https://nsearchives.nseindia.com/content/circulars/CMTR71775.pdf), Circular Ref. 172/2025 | 12 Dec 2025 | calendar year 2026 | 14 Sep (Ganesh Chaturthi), 2 Oct (Mahatma Gandhi Jayanti), 20 Oct (Dussehra), 10 Nov (Diwali-Balipratipada), 24 Nov (Guru Nanak Jayanti), 25 Dec (Christmas) in 2026-09-05..2026-12-31; Muhurat trading on Sunday 8 Nov 2026, timings to be notified subsequently |

The 2026 row was widened from 2026-10-04 to 2026-12-31 on 2026-10-12 (audit
findings DB-B25 and DB-B41: the calendar of record had ended while this
circular ran on, and every pull of a newer day failed). Its four holidays
after 4 October and its Muhurat note were read from the same circular on
2026-10-10 by the audit's two readers
(`state/blindspots/time-calendar-edges-r1.md` in the operator's work folder);
the package that widened the row did not fetch the circular again. No vendor
day zip corroborates those four days: they had not happened when the row was
widened. The Muhurat's timings were not notified in that circular, so no
window is recorded for 2026-11-08.

Cross-check against the vendor's GDFL capital-market day zips (names of the
outer archive, read with `unzip -Z1`): over 2018-09-03..2019-12-01 every
weekday not listed above has an index and a stock day zip (301), and the only
day zips on a weekend or a listed holiday are the Muhurat sessions 2018-11-07
and 2019-10-27; over 2026-09-05..2026-09-30 every such weekday has one (17,
the last four from the standalone 25-30 Sep day zips). January-August 2018 is
sourced from 36475 alone; no local data corroborates it.

## GDFL NSE options tick files — measured 2026-10-04 and 2026-10-05 (D-2806)

- **Layout.** One outer zip per year (`<yyyy>.zip`), each day a STORED day zip
  `<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.zip` (2026: 293,014,548 bytes for
  2026-08-03, read with `unzip -v`); inside, entries
  `GFDLNFO_TICK_<ddmmyyyy>\Options\<TICKER>.NFO.csv` with the Windows separator,
  deflated. The verified tick store keeps every entry byte for byte under
  `options/<yyyy>/<MON_yyyy>/GFDLNFO_TICK_<ddmmyyyy>.bts` (FORMAT.md v1, §2).
- **Only options.** The day zips hold only an `Options` folder (2018-11-30:
  4,669 entries; 2022-06-03: 8,988; 2026-09-24: 16,466; GDFL build thread,
  read-only). There are no futures in the operator's GDFL data.
- **Rows.** The capital-market header
  `Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest`,
  the Ticker `<TICKER>.NFO`, CRLF. Traded rows of options on low-priced shares
  print at 0.01 to 0.04 (2026-08-03: 20 files, IDEA, YESBANK, IRFC and
  others), so one paisa is a price.
- **Names.** Tick store FORMAT.md §6 as revised 2026-10-04 (sha256
  `d21139f55789315c5a8280432f8074ea613f17efe8d1a838b325d193adec5be9`): day form
  from 2019-02-01; before it the day form only for NIFTY and BANKNIFTY
  weeklies and the monthly form for every other option. Underlyings carry
  digits and punctuation (`TV18BRDCST`, `360ONE`, `M&M`, `NAM-INDIA`).
- **Monthly expiry.** "Last Thursday of the expiry month. If the last Thursday
  is a trading holiday, then the expiry day is the previous trading day." NSE
  equity derivatives contract specifications,
  `https://www.nseindia.com/products/content/derivatives/equities/contract_specifitns.htm`,
  read from the web.archive.org snapshot of 2019-08-16 21:57:03 (sha256 of the
  fetched page `883f5e4de7256a3262d55e5d1790946b9adaf6c15579f65671c2f20081e22bc1`);
  the live page no longer serves it. The same snapshot states NIFTY 50 weekly
  options expire every Thursday, on the previous trading day when it is a
  holiday. The name census of the GDFL build thread found this rule equal to
  the vendor's last-trade day and its dated twins for every month 2018-09 to
  2022-12.

## Index aliases and symbol changes — read 7 Oct 2026 (D-3332)

`CLAUDE.md` §3 rule 1 requires every name mapping `core::alias` applies to
trace to a source recorded here. These were read on 7 Oct 2026 (IST) over
HTTPS, at the operator's instruction to look up the exchange's own sources
(D-3332). The digests are of the bytes read; a live page may serve different
bytes later.

**F&O index symbols.** Each row joins three sources and one measurement: the
NSE derivatives product page names the F&O symbol and the index it is based
on; NSE's own index list (`https://www.nseindia.com/api/allIndices`, 139
indices, its timestamp `06-Oct-2026 15:30`, sha256
`563bf499d9815fb97198c50ffc1b8fe78c17ac0e879d389b5e1f8bcf460b25cd`) gives the
index's `index` name and its `indexSymbol`; and the `indexSymbol` is the GDFL
index file stem (`<indexSymbol>.NSE_IDX.csv`, measured by the /db audit in
the vendor's capital-market zips, DB-G26, DB-G53) and, with its spaces
removed, the Zerodha store key (`bars/zerodha/NSE/INDEX/<key>/`, present on
7 Oct 2026).

| F&O symbol | Product page (sha256 of the page read) | allIndices `index` | `indexSymbol` = GDFL stem | Zerodha store key |
|---|---|---|---|---|
| `FINNIFTY` | `https://www.nseindia.com/static/products-services/equity-derivatives-finnifty` (`60cd6ddf540a2771c4af76353ad2c0ea60fcada964ee77a0c427c8d8dc6aef44`): "Underlying symbol denotes the underlying index, which is FINNIFTY"; "The underlying index is Nifty Financial Services Index" | NIFTY FINANCIAL SERVICES | `NIFTY FIN SERVICE` | `NIFTYFINSERVICE` |
| `MIDCPNIFTY` | `.../equity-derivatives-nifty-midcap-select` (`d3930bfad0f56cba53e19935e05edbef000b96644be0f2c5be83e5e789ada710`): "futures contracts on MIDCPNIFTY"; "The underlying index is NIFTY Midcap Select Index" | NIFTY MIDCAP SELECT | `NIFTY MID SELECT` | `NIFTYMIDSELECT` |
| `NIFTYNXT50` | `.../equity-derivatives-nifty-next50` (`ca5f1c41a3856ce808921c30e602fb9262dcde62855caa19ee9b5fdfd599a2f0`): "Symbol : NIFTYNXT50 Underlying: Nifty Next 50 Index" | NIFTY NEXT 50 | `NIFTY NEXT 50` | `NIFTYNEXT50` |
| `NIFTYIT` | `.../equity-derivatives-niftyit` (`3c99c7b3a40fc47ded2316bb38aab584fbbba87ce33c6392f7b3e21bd1ddb6e6`): "Underlying : NIFTYIT"; "The underlying index is Nifty IT" | NIFTY IT | `NIFTY IT` | `NIFTYIT` |
| `NIFTYFPI` | `.../equity-derivatives-nifty-india-fpi` (`ee0c9e1f89df9cb48af432a17be04a5298f7d4e095fb1826f8358e413e9a070b`): "the underlying index, which is NIFTYFPI"; "The underlying index is Nifty India FPI 150 Index" | NIFTY INDIA FPI 150 | `NIFTY FPI 150` | `NIFTYFPI150` |
| `NIFTYCPSE` | `.../equity-derivatives-niftycpse` (`81c632e8fcc201d6dc7dccec1b4a434482212e42fdd017edd64ef00656bc9e8e`): "Underlying : NIFTYCPSE"; "The underlying index is NIFTY CPSE" | NIFTY CPSE | `NIFTY CPSE` | `NIFTYCPSE` |
| `NIFTY`, `BANKNIFTY` | already aliased (`NIFTY 50`, `NIFTY BANK`) | NIFTY 50, NIFTY BANK | `NIFTY 50`, `NIFTY BANK` | `NIFTY`, `BANKNIFTY` |
| — (reference only, `CLAUDE.md` §1) | none: VIX has no F&O symbol in this table | INDIA VIX | `INDIA VIX` | `INDIAVIX` |

The product pages' `...` stands for `https://www.nseindia.com/static/products-services/`.
allIndices also lists `NIFTY FINANCIAL SERVICES 25/50` (`NIFTY FINSRV25 50`)
and `NIFTY FINANCIAL SERVICES EX-BANK` (`NIFTY FINSEREXBNK`): they are other
indices, which is why an abbreviation match alone could not decide FINNIFTY's
index (DB-G25), and why this table rests on the product page.

**Symbol changes.** NSE's `symbolchange.csv`,
`https://nsearchives.nseindia.com/content/equities/symbolchange.csv` (69,052
bytes, 1,065 lines, sha256
`948ccc56f46555bc6553538047d7b967e99a9250c58b7bf0a1d66a7e1f033e10`), one row
per change: company, old symbol, new symbol, date. The rows below are those
whose old or new symbol the store holds as a GDFL share or option series
(checked on 7 Oct 2026). One was also read in its own circular:
[NSE/CML/67420](https://nsearchives.nseindia.com/content/circulars/CML67420.pdf),
Circular Ref. 0518/2025, 3 April 2025 (sha256
`f0ee1ddfa019753b0c04718e3e55761e74dafef34c5763be3d28000e9759afd4`): ZOMATO to
ETERNAL, Zomato Limited to ETERNAL LIMITED, effective 9 April 2025.

| Old symbol | New symbol | Date in the file |
|---|---|---|
| `IDFCBANK` | `IDFCFIRSTB` | 2019-01-16 |
| `NIITTECH` | `COFORGE` | 2020-08-20 |
| `INFRATEL` | `INDUSTOWER` | 2020-12-18 |
| `CADILAHC` | `ZYDUSLIFE` | 2022-03-07 |
| `MOTHERSUMI` | `MOTHERSON` | 2022-06-09 |
| `LTI` | `LTIM` | 2022-12-05 |
| `SRTRANSFIN` | `SHRIRAMFIN` | 2022-12-20 |
| `IIFLWAM` | `360ONE` | 2023-01-23 |
| `PVR` | `PVRINOX` | 2023-05-12 |
| `ADANITRANS` | `ADANIENSOL` | 2023-08-24 |
| `AMARAJABAT` | `ARE&M` | 2023-10-27 |
| `L&TFH` | `LTF` | 2024-04-23 |
| `MCDOWELL-N` | `UNITDSPR` | 2024-06-07 |
| `IBULHSGFIN` | `SAMMAANCAP` | 2024-07-26 |
| `GMRINFRA` | `GMRAIRPORT` | 2024-12-11 |
| `ZOMATO` | `ETERNAL` | 2025-04-09 |
| `TATAMOTORS` | `TMPV` | 2025-10-24 |
| `LTIM` | `LTM` | 2026-02-27 |

The GDFL share series of each new symbol starts at its date (360ONE 2023-01,
ETERNAL 2025-04, TMPV 2025-10, LTM 2026-02, UNITDSPR 2024-06, SHRIRAMFIN
2022-12, ZYDUSLIFE 2022-03, COFORGE 2020-08, MOTHERSON 2022-06; the /db audit,
DB-071), which agrees with the file. A symbol change does not say a series is
economically continuous: a demerger or a merger can sit on the same day, and
the change refusals of D-3325 still apply. **A symbol with no row in the file
is not aliased**: HDFC and MINDTREE, whose options the store holds, have none;
their share files are filed under their own keys (D-3333), and why each
stopped trading is not recorded here (UNVERIFIED). Rows of the file not listed here are not
needed by any stored series today; `core::alias` may transcribe them later
from the same file and digest.

## NSE F&O circulars read for the options lineage — read 10 and 11 Oct 2026 (D-3354 to D-3358)

`CLAUDE.md` §3 rule 1 requires every exchange claim the code applies to trace
to a source recorded here. Each circular below was fetched over HTTPS from
`nsearchives.nseindia.com` on 10 Oct 2026 (IST) by the /db audit and its
second agent, and read again by WP-12e on 11 Oct 2026 from the bytes fetched.
The digest is of those bytes; the exchange may serve other bytes later.

| Circular | Date | URL | sha256 of the bytes read | What it says, as far as the code relies on it | Where it is applied |
|---|---|---|---|---|---|
| `NSE/FAOP/68747` | 25 Jun 2025 | `https://nsearchives.nseindia.com/content/circulars/FAOP68747.pdf` | `e1b56024a511135ffd5c6c3c097881dd0ac2c37b15b5f61e94fe84e51cf66762` | "Revision in Expiry Day of Index and Stock Derivatives Contracts - Update", in partial modification of circular 68685 of 23 Jun 2025. Its annexure lists each existing NIFTY long-dated expiry with its revised day; the code uses five rows of it: `24-Jun-27` to `29-Jun-27`, `29-Jun-28` to `27-Jun-28`, `28-Dec-28` to `26-Dec-28`, `28-Jun-29` to `26-Jun-29` and `27-Dec-29` to `24-Dec-29`. | `pull::expiry_alias::ALIASES`, the five NIFTY half-yearly rows (D-3358) |
| `NSE/FAOP/73857` | 22 Apr 2026 | `https://nsearchives.nseindia.com/content/circulars/FAOP73857.pdf` | `7d567d564732742bbc81bf5008752fa66a27ce6a3b87e2f2fb4847ddfc525005` | "Scheme of Arrangement: Adjustment of Futures and Options contracts of Vedanta Limited (VEDL)": a demerger, ex-date and effective date Thursday 30 Apr 2026; "All existing contracts with expiry dates May 26, 2026, June 30, 2026 and July 28, 2026, will expire on Wednesday, April 29, 2026." | `pull::expiry_alias::EARLY_SETTLEMENTS`, its one row (D-3356) |
| `NSE/FAOP/64625` | 18 Oct 2024 | `https://nsearchives.nseindia.com/content/circulars/FAOP64625.pdf` | `ee1169239eed9cc06519537c6af53751e031903fe709b2bf296283412dc028f8` | The index lot revision "effective for all new index derivatives contracts ... introduced from November 20, 2024"; "the existing weekly and monthly expiry contracts will continue with the existing lot size till its respective expiry date"; existing quarterly and half-yearly contracts move "on December 24, 2024, end of the day for BANKNIFTY and December 26, 2024, end of the day for NIFTY". Annexure: NIFTY 30-Jan-2025 and BANKNIFTY 29-Jan-2025 are the last monthly expiries with the existing lot; 27-Mar-2025 and 26-Mar-2025 the first existing quarterly ones. | `costs::lot::CONTRACT_RULES`, the first rule of each underlying (D-3357) |
| `NSE/FAOP/70616` | 3 Oct 2025 | `https://nsearchives.nseindia.com/content/circulars/FAOP70616.pdf` | `0718cf0cc7e6c74105dc946497369270fca807f59912586560c9552cad8050ef` | "This circular shall come into effect from October 28, 2025 (EOD). The existing lot size will be applicable for weekly and monthly contracts till December 30, 2025 expiry date. Lot size of existing quarterly and half yearly contracts will be revised on 30-Dec-2025 EOD." Annexure: first monthly expiry with the revised lot 27-Jan-2026; first existing quarterly expiry 31-Mar-2026. | `costs::lot::CONTRACT_RULES`, the second rule of each underlying (D-3357) |
| `NSE/FAOP/53079` | 25 Jul 2022 | `https://nsearchives.nseindia.com/content/circulars/FAOP53079.pdf` | `d46f719c942df40de6507ec3c3e0d6fc83d615c52a731f3790129c9ef8322304` | "Adjustment of Futures and Options contracts in the security VEDL": an interim dividend of Rs 19.50 per share, ex-date 26 Jul 2022. "The full value of dividend i.e. Rs. 19.50 /- would be deducted from all the cum-dividend strike prices on the ex-dividend date." The circular states no lot change (the word "lot" does not occur in it). | The SHAPE `pull::gdfl_lineage::EventKind::StrikeOffset` names (D-3355). No record states a cause: the detector measures the shift and never writes "dividend". |
| `NSE/FAOP/71627` | 4 Dec 2025 | `https://nsearchives.nseindia.com/content/circulars/FAOP71627.pdf` | `c6007698f03bd83c197848176b5d59e7e4457505cceae6844c344769c6e4f0a1` | "Scheme of Arrangement: Adjustment of Futures and Options contracts of Hindustan Unilever Limited (HINDUNILVR)", a demerger. It lists the re-introduced expiries and does NOT state the day the existing contracts expired. | Applied nowhere: no `EARLY_SETTLEMENTS` row is written from it. The early expiry of HINDUNILVR's contracts before 2025-12-05 stays UNVERIFIED. |

**What is NOT sourced here, and so is labelled in the code.** The four
FINNIFTY alias rows of 2021-10-18 and the MIDCPNIFTY row of 2023-08-17 are
INFERRED from the underlying's measured weekday move; their cause is
UNVERIFIED. No circular was read for the lot revisions of July 2021, June
2023, April 2024 and July 2025, so `costs::lot::lot_size_of` refuses a
contract's lot near each of them (D-3357). No circular was read for any
measured unchanged-lot event of 2019 to 2020 (OIL, NATIONALUM, RECLTD, ONGC,
BEL, GAIL, TATACHEM, CENTURYTEX and the others the sweep lists): each `event`
record says what moved and never why. `TATAGLOBAL -> TATACONSUM` and
`KPIT -> BSOFT` have no row in NSE's `symbolchange.csv` as read on 7 Oct 2026
(above), so their options keep two keys (D-3354).
