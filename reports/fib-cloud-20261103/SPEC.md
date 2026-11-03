# SPEC: the top-mover flag continuation on NSE F&O stocks

Written 30 Oct 2026, about 22:4x to 23:xx IST, by the spec step of workflow-fib-1b.js (run wf_cf9d340f-28c), side session 2da9f915.
Status: specification only. No code was built, nothing was committed, nothing under runtime/ was read for writing. Every code reference is at feat/db-fix f01d7a0e4 (`git -C wt/DBFIX rev-parse feat/db-fix`, run 30 Oct).

Every result and page this spec governs carries this label, word for word:
**"current F&O members only (survivorship bias not removed)"**

Short names used below:
- MEM = /Users/parthi/.claude/projects/-Volumes-WD-BLACK-brutex-fresh-20260919-project/memory
- RES = /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/fno-winners-20261013
- ST = /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state
- Code paths are relative to the worktree wt/DBFIX.
- "Zerodha" = Zerodha 1-minute candles (back-adjusted) and the 2-60 min rungs folded from them, plus the pulled 1day bars. "GDFL" = GDFL 1-second bars built from raw vendor ticks.
- A "candle" is always a Zerodha candle; a "second" is always a GDFL second.

---

## 0. How to read this spec

- Section 1 quotes the operator's definition from the files, by name and line.
- Section 2 lists the invariants. Every part that touches the named piece proves them with tests.
- Sections 3 to 13 define each piece exactly.
- Section 14 is the settings table: every setting, its full sweep range and how it is generated.
- Section 15 is the record and audit contract. Section 16 maps every piece to existing code or names it missing. Section 17 lists the test obligations. Section 18 lists the open points, each with the choice that keeps the bar and why. Section 19 states the data's limits.
- Words used exactly:
  - **decision time T**: the instant a decision may first be made. For a candle it is the candle's close.
  - **latency λ**: the swept delay between T and the earliest second an order may act in.
  - **traded second**: a GDFL second whose traded volume is above zero.
  - **worse reading**: when the data or a source leaves two readings open, the one that gives the lower net P&L for the trade, or the one that admits less, as each rule says.

---

## 1. The strategy as the operator defined it (quoted, by file and line)

### 1.1 The main strategy

- MEM/fno-winners-research.md line 11: F&O stock winners are "stocks entering the top 10 gainers/losers (% vs yesterday's close)". They pause within 1 to a few minutes at a Fibonacci level of "the day's low-high range so far". The levels listed are 0, 0.236, 0.382 or "0.312", 0.5, 0.618, 0.786 and 1.
- Same line, the operator's 15 Oct narrowing to the MAIN STRATEGY "top-mover flag continuation":
  - no indicators, only today's high/low Fib (so far);
  - top-N gainers (long) and losers (short);
  - a tight consolidation box holding above (below) the 0.382 retracement;
  - entry on the close beyond the box;
  - a stop just beyond the box ("tiny SL / drawdown is the goal");
  - targets: the day-high/low break, then 1.272/1.618.
- Same line: the operator wants EVERY condition tracked for EVERY F&O stock on every candle. Top movers are just a rank filter.
- Line 13: intraday only, every day, trading the CASH shares of the F&O stocks.
- Line 17: two sides of scoring:
  - side 1, the share net of every share charge;
  - side 2a, the same entries and exits with NO charges, labelled "signal only, before charges", still with worst-price fills.
- Line 21: ONE TRADE PER STOCK. Once a trade is open on a stock, no new trade on that stock until it exits, long and short alike.
- Line 25: let it run (trailing stop, trailing take-profit, give-back, time stop, session exit).
- Line 29:
  - (d) STOP-LOSS ALWAYS. No strategy is tested without a stop. No-SL runs only as a labelled comparison.
  - (e) every exit is decided on the underlying share's price.
  - (f) exits work both with the exit grid and from the strategy's own conditions.
- Line 32:
  - REST SHAPE = ANYTHING (box, U, upside-down U), described by measurements.
  - "retracement never inside the high's own candle (rest starts in later candles)".
  - ALL levels (VWAP, opening range, yesterday H/L/C, pivots, CPR, round numbers and more).
  - mover rank top 1/3/5/10/25/50 or single stocks.
  - NOTHING is static: every value is generated from past data by a rule and refined where survivors crowd.
  - Scorecard: win rate, profit factor, avg R, worst loss beyond stop, max drawdown, share of winning days/weeks/months/quarters/half-years/years.
- Line 34: the 25 Oct small check passed nothing; the edge was about 0 gross and costs were about 1R a trade.

### 1.2 Patterns anywhere

MEM/precise-entry-patterns-anywhere.md:
- Line 11: the operator's words, "our main idea is to find that precise entry ... with minimal negatives". A partial U of two or three candles counts.
- Line 15:
  - the distance to the nearest Fib level is a measured feature (touch, approach from below or above, crossed), never a precondition;
  - 2-3 candle micro-structures are described by measurements with generated thresholds;
  - entry is on the close (or break) of the candle that touches the level, with the side's colour and the order flow delta sign;
  - the stop sits just beyond the micro-structure.

### 1.3 Let the data decide; nothing fixed

- MEM/let-data-decide.md line 15: no choice waits on the operator. Every former decision point is tested both ways as a labelled variant.
- Line 17: "NO FIXED VALUES, EVER". Every setting of every idea is swept over its whole range in every combination: every ORB window minute by minute, every Fibonacci level and extension, every range the ladder is drawn on, every pattern, every tolerance.
- Line 19: RUN ORDER = PRICE ACTION FIRST. Exits are their own dimension, never mixed into entry combinations. The first focus run covers F&O stocks only; VIX is only a stamp on each trade.
- MEM/nothing-fixed-everything-generated.md line 18: the ONLY fixed numbers are the requirement (Winner Gate) thresholds. Line 20: exchange and broker facts (the auction, the 15:10 square-off) stay fixed, but the last-entry time is still tested.

### 1.4 Scope, requirement and money (30 Oct)

MEM/scope-fno-stocks-first.md:
- Line 8: "purely NSE equity FnO stocks alone only, not even options".
- Line 23: Zerodha signals on every timeframe; every other decision on GDFL; ONE fill model, the worst price inside the GDFL second; point-in-time membership; 3 Sep 2018 to today.
- Lines 25-27:
  - capital Rs 20 lakh;
  - Rs 1 lakh margin x 5 = about Rs 5 lakh a position;
  - quantity closest to Rs 5 lakh on either side;
  - at most 20 open; up to 100 trades a day.
- Line 34: the 21:19 IST survivorship decision.
- Line 36: the Fibonacci parts merge after BASE-REDS-1023; the full backtest runs only from merged code.

### 1.5 The requirement every recipe must meet (the Winner Gate)

MEM/backtest-acceptance-bar.md:
- Line 15 (16 Oct): win rate >= 51% after real Dhan charges, average win >= 2x average loss, profit factor >= 2, and winning in walk-forward on unseen months. Walk-forward = the unseen months together pass the full gate, >= 70% of unseen months are positive, and no unseen month is worse than the learned loss floor.
- Line 15, 11:13 IST: everything is net of the entire charges, and a trade is a win only if its net P&L is above 0.
- Line 19: leave-one-out (drop the best day, week, fortnight, month or quarter; it must still pass).
- Line 21: no minimum stop.
- Line 23: >= 500 trades a year across all stocks (partial years pro-rated by tradable days).
- Line 29, FINAL:
  - quality per side: each side alone passes 51%, 2x avg, PF 2, 2x largest and is positive on unseen months;
  - quantity per pair: >= 500 trades a year, every year positive, full walk-forward, leave-one-out.
- Line 30: five downside checks, each comparing UNSEEN with LEARNING months: worst day, worst week, max drawdown, losing streaks, recovery time.
- Line 33: walk-forward must be massive, a family of anchored and rolling schemes. A setup must pass under every scheme.
- Line 34: the pair is net positive on unseen months of each NIFTY regime. VIX regimes are information only.
- Line 37: minimum returns, average unseen net return >= 0.3%/day, 1.5%/week, 6%/month, 18%/quarter, 75%/year.
- ST/fib-run-20261016/min-returns-amendment.txt lines 2-10: these are simple (not compounded), averages, partial periods pro-rated.
- ST/fib-run-20261016/wfo-family-amendment.txt lines 3-14: the scheme family, the pass rule under every scheme, every scheme a luck trial.

### 1.6 The research files

- RES/fib-one-level.json lines 3-99:
  - the ladder, running or frozen when the pause starts (lines 3-9);
  - pattern 1, one candle covers exactly one level (lines 11-19);
  - pattern 2, N candles in a row touch the same level (lines 21-27);
  - pattern 3, dip then a green candle reaching the next level (lines 29-37);
  - entry at the green candle's close or the moment it crosses the level (lines 39-45);
  - stop just under the level or at the level below (lines 47-55);
  - exits: the next level, the one after, trailing, the exit grid or a condition exit, shares out by 15:07 (lines 57-63);
  - mirror for falls (lines 65-73);
  - early-day packing (lines 75-81);
  - inside the minute, the order from seconds (lines 83-91).
- RES/fib-and-seconds.json lines 9-20: the range is from 9:15 up to the candle before this one, and a candle is added only after it is judged. The direction comes from the most recent extreme; a candle making both a new high and a new low leaves no direction until the next new extreme.
- RES/fib-and-seconds.json lines 21-26: "near" is a close within 1% of today's range so far (D-0076).
- RES/fib-and-seconds.json lines 33-86: the pause, touch and turn, break, wick touch, previous level, 0.312, near width, range big enough, several timeframes.
- RES/fib-and-seconds.json lines 89-172: seconds give the order inside a minute; worst-price fills; earlier tighter entries; pause quality; data eras; halts and clock steps; share fills wait for a per-stock clock calibration.
- RES/plan.json lines 83-91: run only the pure price-action set first; indicators are a separate track.
- RES/lens-*.json: 248 more idea rows across nine lenses (avoid 16, context 30, events 28, exits 30, levels 29, market 30, opening 26, outside 29, volume 30; counted with jq on 30 Oct). They are features and variants for later waves (section 7.6). None changes a rule in this spec.
- ST/fib-small-20261024/REPORT.md:
  - line 7: nothing passed;
  - line 31: the median stop is about 0.18% of price, so 1R is about Rs 880 on Rs 5 lakh;
  - lines 111-120: costs are about 1R a trade, and the worst-price rule cost Rs 430-707 a trade on minute fills;
  - line 126: frictions should be part of the search, with stops wide against each stock's own measured friction, generated.

---

## 2. Invariants (each needs its own tests; section 17)

**FIB-I1 No look-ahead.**
- A Zerodha candle labelled t on timeframe tf covers [t, t+tf). It is known only at its close t+tf.
- Bars are left-labelled: runner/src/trade.rs lines 21-23 say only the accepted unique 15:09 row can price the 15:10 exit.
- No decision at time T reads a candle whose close is after T, or a second at or after the second it acts in.

**FIB-I2 A ladder never judges its own candles** (operator rule 2).
- Every ladder Λ has a set of defining candles D(Λ) (the candles whose prices set its anchors) and a known-at time K(Λ). K(Λ) is the later of:
  - the close of the last defining candle;
  - the close of any candle that confirms an anchor (a swing confirmed k candles late; a push known to have ended only when the pause began).
- Λ may be tested only against candles whose open is at or after K(Λ), and only against seconds at or after K(Λ).
- The running day ladder for candle i is the ladder of candles 0..i-1 of the session. Candle i is judged against it first; only then does candle i join the range.
  - A candle that sets a new extreme is judged against the ladder as it stood before it.
  - A candle that engulfs the range is judged against the old ladder, then folds (and leaves the leg undetermined, FIB-I3).
- Example (operator): a ladder from the 1-minute candles 09:15..10:04 is first used on the 10:05 candle.
- The same rule holds for every anchor and every timeframe:
  - the push leg;
  - the opening range of any N minutes (first used on the first candle opening at or after 09:15+N);
  - a swing leg;
  - yesterday (first used at today's first candle);
  - the last k sessions;
  - the gap leg;
  - a frozen ladder (first used on the candle after its last defining candle).
- The seconds walk uses at second s only ladders with K(Λ) <= s.
- The existing engine keeps this for the running day ladder by the order reset, emit, fold, fused in `CurDayFib::step` (crates/indicators/src/lib.rs lines 484-503; rule 2 of the module doc, lib.rs lines 39-42).

**FIB-I3 Same-candle extremes** (operator rule 3).
- When the session's high and low so far sit in one candle, the order is unknown and the day ladder has no direction. This covers the first candle alone, and a later candle that makes a new high and a new low at once.
- The ladder is not used until a later candle makes a new extreme on one side only. The ladder that candle produces is first used on the candle after it.
- This is the engine's rule: `Leg::Undetermined` (lib.rs lines 233-245; fold lines 506-529, where (true, true) gives Undetermined; strict > and <).
- Two labelled variants are swept beside it, never silently (section 6.4): GDFL-seconds order, and the both-ways ladder.

**FIB-I4 One fill model.**
- Every fill (entry, exit, stop, target, time exit, square-off) is at the worst price printed inside the GDFL second it fills in: a buy at that second's high, a sell at its low.
- No price is ever a level, an open, a mid or a best case.
- Volume, latency, bands and broker rules only decide WHICH seconds may fill and HOW MUCH. They never change the price rule.

**FIB-I5 Worst-case pins.** Every choice the data or a source leaves open is pinned to the worse reading named in this spec. Both readings are computed where the rule says "both".

**FIB-I6 Nothing static.**
- Every number on a strategy path is a swept setting or is generated from past data by a rule (section 14).
- The exceptions are the operator's requirement thresholds (1.5), the operator's money rules (11), and sourced exchange and broker facts.
- A literal number elsewhere on a strategy path is a defect.

**FIB-I7 Walk-forward always** (operator rule 4). No recipe is reported as passing except on walk-forward results (section 13.4).

**FIB-I8 Feeds are never compared across a corporate action.**
- Zerodha is back-adjusted and GDFL is raw.
- A Zerodha price is mapped to a GDFL price only inside one session, by a ratio measured in that session (section 6.6), or by the corporate-action table once R4-26 is live.
- No level crosses a session boundary between the two feeds.

**FIB-I9 The universe and its label** (section 3). The universe is exactly the 218 names, traded only on days each was a member. Every result carries the survivorship label.

**FIB-I10 Integers.** Prices are paisa i64, money is paisa i64, ratios are exact rationals or integer parts per million, and there are no floats on a decision path (CLAUDE.md section 7).

**FIB-I11 O(1) per step at p99** on hot paths, proven by benches:
- per candle per stock;
- per second per open order or position, with at most 20 positions plus pending orders, a fixed bound;
- per minute for the cross-section, with a fixed-size network over the padded pool.

**FIB-I12 Recorded and auditable.** Every decision, order, partial fill, refusal and skip is written to the store's own records, with the run identity (section 15).

---

## 3. Universe and day mask

### 3.1 The universe: the operator's 21:19 IST decision (binding)

Recorded as given by the operator on 30 Oct 2026 at 21:19 IST, relayed by the main session (MEM/scope-fno-stocks-first.md line 34):

1. The backtest universe is the **218 CURRENT F&O stocks only**. They are listed one per line in ST/fno-data-fix-20261030/gdfl-rehearsal-r1/analysis/fno218.txt, built from ST/fno-data-audit-20261030/universe.json.
   - `wc -l` gives 218 and `sort | uniq -d` gives nothing (run 30 Oct).
   - The audit derives the 218 from GDFL's 29 Oct option zip less the 6 indices, matching the Zerodha master name for name (fno-data-audit REPORT.md line 15).
2. Stocks that left F&O since 2018 are NOT added: not to the traded set and not to the rank pool (section 5.2).
3. Point-in-time membership applies within the 218. A stock is traded only on the days it was an F&O member (`pull::fno_membership::Membership::is_member`, crates/pull/src/fno_membership.rs line 2585).
4. Every backtest result and page carries the label "current F&O members only (survivorship bias not removed)".
5. ICICIPRULI is joined to ICICILIFE as one stock (operator-approved; the join ships in Saturday's build, D-7311). ICICILIFE's history before 23 Oct 2026 is ICICIPRULI's.
   - universe.json for ICICILIFE: ISIN INE726G01019, history name "ICICIPRULI (until 2026-10-22; ICICILIFE from 2026-10-23)", rename held back.
   - crates/core/src/alias.rs lines 549-553 hold the rename back, because `FNO_UNDERLYINGS` names ICICIPRULI.

Difference from the code's list (measured 30 Oct by `comm` of fno218.txt against the 208 shares of `core::universe::FNO_UNDERLYINGS`, crates/core/src/universe.rs line 914 onward, less the 5 index names at line 1159):
- In the 218 and not in the code: ANANDRATHI, ATHERENERG, COROMANDEL, CUB, DATAPATTNS, ENRIN, ICICILIFE, KARURVYSYA, MAHABANK, NAVINFLUOR, SAGILITY, SYRMA, UJJIVANSFB (13).
- In the code and not in the 218: BAJAJHLDNG, DALBHARAT, ICICIPRULI (3). ICICIPRULI is joined, so it is not a leaver.
- CLAUDE.md section 1 names the 208. Section 18 OP-01 covers the law change.

The joined series for the backtest is a read-time map, held as data, never typed in code paths:
- for every date before 2026-10-23, read the store key ICICIPRULI and its membership under ICICIPRULI;
- from 2026-10-23, read ICICILIFE.
- The stock is a member on day D if it is a member under either spelling on D.
- Oct 2026 for ICICILIFE is skipped (audit REPORT.md line 219) until its data is complete.

### 3.2 The day mask (stock s, day D)

A stock-day (s, D) is **tradable** only if every condition below holds. Each refusal is counted by reason, per stock, per year, and written to the records. A refused stock-day may still supply context (a rank pool member or a ladder input) only where section 5 or 6 says so.

| # | Condition | Source of truth | Refusal reason |
|---|---|---|---|
| M1 | s is in the 218 list (joined spelling, 3.1) | fno218.txt, digest in the run identity | `not_in_universe` |
| M2 | D is a session of the calendar of record (CAL-1), a normal session | `pull::calendar::kind_of` (crates/pull/src/calendar.rs line 654) | `not_a_session` |
| M3 | s is a point-in-time F&O member on D; a day the membership refuses (`MembershipUnknown`) is not traded | fno_membership.rs lines 2521-2604 | `not_member`, `membership_unknown` |
| M4 | D is within 2018-09-03 .. the last sealed day of both feeds (the end date is the last day both feeds are proven; provisional 1day bars are excluded) | data proof (main session); audit REPORT.md line 206 | `outside_span`, `provisional` |
| M5 | the Zerodha candles of s on D exist for the timeframe | store | `no_signal_data` |
| M6 | the GDFL seconds of s on D exist | store | `no_fill_data` |
| M7 | (s, D) is not in a skip row of ST/fno-data-audit-20261030/report-evidence/backtest-skip.tsv (79 rows) | the TSV, digest in the run identity | `skip:<row reason>` |
| M8 | s is not on trade-for-trade (BE/BZ) on D: Dhan does not allow T2T scrips intraday | sources-leverage-execution.md line 125 (T01-18) | `trade_for_trade` |
| M9 | D is not a Muhurat or other special session (the square-off clock for those is UNVERIFIED, OP-29) | CAL-1 | `special_session` |
| M10 | s is not suspended on D, and has at least one traded second on D | GDFL | `no_trades` |
| M11 | s has a valid previous close for the % move (section 5.1): it traded on the previous session and that session is not a demerger ex-day | Zerodha | `no_prev_close` |

The partial-session skip rules apply inside a tradable day, never as a silent drop. Each is from the audit (REPORT.md lines 197-221, backtest-skip.tsv):
- **2019-05-30**: GDFL starts at 11:57:47 (182 stocks). An order whose start second would be before 11:57:47 is refused (`no_fill_data_window`). It is never moved later, because that would invent a delay the market did not have. Decisions from the Zerodha morning are allowed; their orders are refused until GDFL begins.
- **2021-02-24**: Zerodha ends at 11:39; GDFL holds 15:45-16:59. No decision after the last Zerodha candle. Any position open at the outage stays open and is closed at the worst GDFL price in the resumed window 15:45 to the session end (worst reading; Dhan's handling that day is UNVERIFIED).
- **The 17 market-wide Kite-hole days** (audit REPORT.md line 119), and every stock-specific Kite-hole minute: a missing Zerodha minute is not a no-trade minute.
  - A candle containing a hole minute is incomplete: no decision is made on it (`incomplete_candle`).
  - Any ladder whose defining candles include an incomplete candle is tainted for the rest of that session and is not used (`tainted_ladder`), because it may have missed an extreme.
  - A labelled variant fills the missing minute from GDFL, ratio-mapped (section 6.6). It is never mixed into the headline.
- **Auction era**:
  - GDFL 2026-08-03..09-24: the closing-auction print served as a 1s trade is dropped from fills.
  - The 44 auction-era stock-days whose 15:15-15:29 trades Zerodha omits: the session end is the last continuous-trading second from the calendar for (s, D) (section 9.8).
- **Per-stock windows** (all from backtest-skip.tsv): PIIND Zerodha 2018-01..2020-01; IRFC before 2021-01-29; KFINTECH before 2022-12-29; MAZDOCK 1day before 2020-10-12 and its Oct 2026 minutes; demerger ex-days NMDC 2022-10-27, ITC 2025-01-06, SIEMENS 2025-04-07, VEDL 2026-04-30, TMPV 2025-10-14; RELIANCE 1day not mixed with intraday through 2020-05-13; VBL 2023-06-14 12:40-12:47; PATANJALI 1day 2023-07-14 and before 2020-01-27; FORCEMOT 2023-10-26..2024-02-13; ICICILIFE Oct 2026.
- **Kept, not skipped**: the 12 stocks with no Zerodha data (ANANDRATHI, ATHERENERG, COROMANDEL, CUB, DATAPATTNS, ENRIN, KARURVYSYA, MAHABANK, NAVINFLUOR, SAGILITY, SYRMA, UJJIVANSFB) cannot signal until the data fix pulls them. Their stock-days fail M5 and are counted. Nothing is filled from GDFL candles in their place, because signals come from Zerodha only.

Halts: a market-wide halt prints frozen GDFL prices with no traded volume (MEM/design-minute-sweep-tick-fills.md line 22). The fill rule "a second with no traded volume fills nothing" (9.3) already refuses fills inside a halt. The exact halt windows of 13 and 23 Mar 2020 are UNVERIFIED (sources-leverage-execution.md line 193, T06-07); those two days are flagged on every trade that touches them.

### 3.3 What the mask reports

For each run, per year and per stock, the report gives:
- member-days in the 218;
- tradable stock-days;
- refusals by reason;
- PIT members on each day that are outside the 218 (`Membership::pool_size` less the 218 members, fno_membership.rs line 2545). This shows the survivorship bias as a number, never hides it.

---

## 4. Clocks: candles, decision time, latency, seconds

### 4.1 Candles

- Timeframes: 1, 2, 3, 5, 10, 15, 30, 60 min and 1 day (operator requirement).
- Intraday rungs are folded from 1-min candles anchored at 09:15 (crates/store/src/path.rs lines 395-430; `pull::fold`). The last candle of a session may be a stub.
- A candle labelled t closes at min(t + tf, session end). Its decision time is T = that close.
- A rung candle is incomplete if any of its 1-min constituents is a hole (3.2).

### 4.2 Decision time and the first second the engine may act in

- For a decision at T, an order may start only in the **first traded second s with s >= T + λ**.
- λ is the swept latency (section 14, S41).
- A second labelled s covers [s, s+1 s). With λ = 0 the earliest second is the one starting at T, which holds no price printed before T.
- **Clock calibration (OP-07).** GDFL's clock can step back 2-14 s, and the share clock offset against Zerodha is not yet measured (MEM/design-minute-sweep-tick-fills.md lines 18 and 21; RES/fib-and-seconds.json lines 166-172).
  - Until a per-stock clock calibration exists, the headline uses only λ >= the measured largest backward step of GDFL's clock for that stock-day.
  - Smaller λ values are run and reported labelled "uncalibrated clock".
- A resting order already at the broker or exchange (a stop, a target, a stop-entry) acts with no latency from the second its trigger prints (section 9).
- Every change to a resting order (a trailing step, a stop moved to a new level) is an order action. It takes effect at the first traded second >= the decision time of the change + λ.

### 4.3 The 1 day timeframe

- A daily candle for D is known after D's session closes. A signal on D's daily candle is traded on the next session D+1 (the next CAL-1 session, which must itself be tradable for s).
- The order starts at the first traded continuous-session second >= 09:15:00 + λ of D+1. A swept variant starts at a later time of day on D+1 (S43).
- The pre-open call auction (09:00-09:15) is not modelled: no pre-open price is in the GDFL seconds the store holds (UNVERIFIED; OP-40).
- The 1day bar of 2018-11-07 (Muhurat), and 1day bars marked provisional, are never signal candles (backtest-skip.tsv).

---

## 5. The top-mover ranking

### 5.1 The % move and its price basis

For stock s at decision time T on day D:

```
m(s, T) = (P(s, T) - C(s, D-1)) / C(s, D-1)
```

- **P(s, T)** = the close of s's latest 1-minute Zerodha candle whose close is <= T.
- **C(s, D-1)** = the previous session's close in Zerodha (back-adjusted). Both are Zerodha, so a split or bonus on D is handled by the adjustment. Demerger ex-days are wrong in Zerodha (audit REPORT.md lines 134-143) and are skipped (M7, M11).
- The basis is a two-reading setting (S10, OP-04):
  - (a) the previous session's pulled 1day close;
  - (b) the close of the previous session's last 1-minute candle.
  Which of these equals NSE's official close is UNVERIFIED, so both run, labelled.
- Known at T: m uses only candles closed by T. It is never the day's closing % (MEM/fib-run-20261016 mover-persistence-amendment.txt lines 1-3: the end-of-day list is look-ahead).
- Arithmetic: m is never materialised as a float. Ranks compare cross-multiplied i128 products, so two moves compare exactly.

### 5.2 The rank pool and the rank

- **Pool(D, T)** = { s in the 218 : (s, D) passes M1-M4 and M11, and s has at least one candle closed by T on D }.
  - Stocks that fail M5-M10 still rank if they have candles. For example a stock whose GDFL day is missing is ranked but not traded.
  - The number of PIT members excluded because they are not in the 218 is reported per day (3.3).
- **Gainer rank** g(s, T) = 1 + #{ x in Pool : m(x, T) > m(s, T) }. This is competition ranking: equal moves share a rank, and a tie at the edge of top N includes all tied stocks.
- **Loser rank** l(s, T) = 1 + #{ x in Pool : m(x, T) < m(s, T) }.
- Long uses gainer ranks and short uses loser ranks. The two are searched separately (MEM/long-short-separate.md line 11).
- **Stale prices.** A stock with no new candle since t_last keeps its last close. Staleness = T - close(t_last) is a measured feature. The setting S15 can exclude stale stocks from the pool beyond a swept limit. The headline never treats a stale price as fresh without saying so.
- **One rank clock for every timeframe.** The rank is a state of the market at T, computed from 1-minute closes. A 15-minute candle closing at T reads the same rank as a 1-minute candle closing at T.
- For the 1 day timeframe, the rank is computed from daily closes (D against D-1) and known after D's close.
- **Rank features** (all measured, settings in section 14):
  - in top N now;
  - entered top N on this candle;
  - entered within the last W candles;
  - the entry's ordinal today (1st, 2nd, ...);
  - minutes in top N;
  - best rank today;
  - rank change over the last W candles;
  - |m| against the stock's own usual move at this time of day (generated);
  - the pool size.

### 5.3 The cross-section's cost

- Per minute: one pass over at most 218 rows, plus a fixed comparison network over 256 padded slots, sized at build time from the 218. This is the K16 design's bitonic network, RES/condition-kit-design.json block K16, "per_bar_work".
- Fixed work every minute, so p99 equals p50 by construction. A bench proves it.

---

## 6. The Fibonacci ladder

### 6.1 Anchors (which range)

Each ladder is an (anchor low, anchor high, leg) triple with its defining candles and its known-at time K (FIB-I2):

| Anchor | Defining candles | Known at | Exists today? |
|---|---|---|---|
| A1 today running (the operator's "range so far") | session candles 0..i-1 | open of candle i | yes, `CurDayFib` (lib.rs lines 400-529), bits 121-131 |
| A2 today frozen at the pause start | candles 0..b-1, where b is the first box candle | open of candle b | missing |
| A3 the push leg (last swing extreme before the pause, to the push extreme) | the push candles | close of the candle that shows the push ended (the first candle not making a new extreme) | missing (K06) |
| A4 a confirmed swing leg | swing candles | close of the confirming candle (k candles later, k swept) | swings exist in `indicators::trend` (bits 72-73); legs as ladders missing |
| A5 the opening range of N minutes | 1-min candles 09:15..09:15+N-1 | 09:15+N | ORB 5/15/30/60 exist as bits 86-105 (no ladder); every N missing |
| A6 yesterday (H, L) | the previous session | today's first candle | previous-day ladders exist (`fib.rs`, bits 20-29, 69-71, 106-109) |
| A7 the last k sessions | the k previous sessions | today's first candle | k = 5 exists (`fib::Prev5`, bits 110-120); other k missing |
| A8 the opening gap leg | previous close to today's open | today's first candle | exists (`gap`, bits 132-142) |
| A9 measured move / pause projection | the push plus the pause | the break candle's close | missing (K07) |

### 6.2 Levels and extensions

- **Retracement rungs, inside the range**: 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1.
  - OPERATOR DECISION (31 Oct 2026 ~08:4x IST): 0.31 (0.312) and 0.65 were typed by mistake and are NOT rungs. Only the standard rungs above, and the extensions, are used.
- **Extensions in the move's direction**: 1.272, 1.618, 2, 2.618, 3, and every ratio in the continuous feature (6.5).
  - For a long on an up leg, the extension x sits at lo + x·R, above the high. For a short on a down leg it sits at hi - x·R, below the low.
- **A defect to name.** The engine's current-day extensions do not lie in the move's direction.
  - `CurDayFib` places rung p at hi - p·R on an Up leg (lib.rs lines 654-660), so its 1.272 rung (bit 128) sits BELOW the low on an up leg.
  - It cannot serve as the long's 1.272/1.618 target. The new ladder block must provide direction-aware extensions.
  - Bits 121-131 stay as they are (append-only, CLAUDE.md section 3 rule 8). OP-35.
- **The continuous position.** For every ladder the engine also measures the exact position p = (close - anchor)/R on the leg's own axis. The named rungs are points on it, and the bucket edges of p are generated (6.5). So "every level and extension" covers more than the listed ratios.

### 6.3 Direction (the leg)

The engine rule is the default and the headline (FIB-I3):
- Up = the high is the later extreme. Retracements are measured down from the high.
- Down = the low is the later extreme. Retracements are measured up from the low.
- Undetermined = no range yet, or one candle made both new extremes. The ladder is not used.

### 6.4 Same-candle extremes: the two labelled variants (operator rule 3)

**V-SEC: GDFL order inside the candle.**
- For the candle (or candles) where the high and the low so far share a candle, read the GDFL seconds inside that candle's span.
- The extreme whose FIRST print is later wins: max later gives Up, min later gives Down. Both first printed in the same second gives Undetermined.
- This is used only where both feeds agree on that candle's high and low. Agreement is tested inside the session by ratio (6.6):

  ```
  | H_z/L_z - H_g/L_g | <= e
  ```

  e is generated from that stock's own distribution of minute disagreements, with e = 0 (exact) in the sweep.
- When the feeds do not agree, the engine rule applies (Undetermined).
- The leg is then known at the candle's close. The ladder is first used on the next candle (FIB-I2).
- A sub-variant, labelled and same-feed: on a timeframe above 1 minute, the 1-minute Zerodha candles inside resolve the order first, and the seconds only when both extremes share one 1-minute candle (OP-09).

**V-BOTH: the both-ways ladder.**
- Two ladders are always live once a positive range exists: up from the low (lo + p·R) and down from the high (hi - p·R).
- Every touch is labelled with which ladder and which rung.
- It needs no order. It is first used on the candle after the last defining candle.

### 6.5 Touch, near and the distance feature

The distance to a level is a FEATURE, never a precondition (MEM/precise-entry-patterns-anywhere.md line 15). For every ladder and rung, on every candle judged against it, the engine records:
- the signed distance from the close, the high and the low to the level:
  - in parts per million of price;
  - as a fraction of R;
  - in measured price steps (tick-denominated measures use the raw price's step, OP-39);
- the relation: wick touch, body through, body inside, close above, close below, between two rungs, spans several rungs;
- whether exactly one rung lies inside the candle's range (RES/fib-one-level.json lines 11-19);
- the count of consecutive candles touching the same rung (lines 21-27);
- the previous rung touched today.

"Near" is a setting, not a constant:
- tolerance t over {exact cover of the candle's range} plus a generated ladder in each unit above, including 0;
- the engine's pinned 1% of range (`TOL_FIB_MILLI` = 10, crates/vocab/src/tolerance.rs line 121, D-0076) is one point of that sweep (OP-36).

Early-day packing: rungs closer together than a generated number of price steps are flagged "packed" (RES/fib-one-level.json lines 75-81). The minimum range so far (R against the stock's usual daily range, generated) is a setting that includes 0 (RES/fib-and-seconds.json lines 76-80).

### 6.6 From a Zerodha level to a GDFL price (FIB-I8)

Patterns and ladders live in Zerodha (adjusted) prices. Stops, targets, breaks and fills live in GDFL (raw) seconds. A level L_z fixed at decision time T maps to a GDFL price by one of three readings (S51):

- **R1, ratio-mapped**: L_g = L_z · G(T) / Z(T).
  - Z(T) is the close of the last Zerodha 1-minute candle closed by T.
  - G(T) is the last traded GDFL price in that same minute.
  - Both are inside one session, so no corporate action lies between them.
- **R2, % of the entry fill**: the stop or target distance d = (L_z - Z(T)) / Z(T) is applied to the entry's volume-weighted fill price (strategy-mvp REPORT.md line 127, which proposes the same).
- **R0, exact factor**: L_g = L_z / f(D), with f from the corporate-action table. Only once R4-26 is live; until then R0 is refused by name.

The mapped price is rounded to the raw price step against the position (a long's stop down, a long's target up), so mapping never helps the trade.

---

## 7. Patterns and features

### 7.1 The operator's seed pattern: the flag (box) after a push

For a long (mirror everything for a short):
- **Push**: from the last confirmed swing low (or the session open) to a new session high. Its size, speed (candles), volume, number of new highs, and rungs crossed are measured (the K06 measurements).
- **Box**: K consecutive closed candles after the push extreme, the first starting no earlier than the candle after the push extreme ("retracement never inside the high's own candle", MEM/fno-winners-research.md line 32).
  - Box high = the max of their highs; box low = the min of their lows; height h = box high - box low.
  - The box height and its relation to a level are measured.
- **Holding rule** (S33): every box candle's low (or close, a variant) stays above level L of the ladder (L = 0.382 is the operator's seed; L is swept over every rung, including "none"). The ladder is judged per FIB-I2, under the ladder mode S21: running (each box candle against the ladder before it), or frozen at the box start (A2).
- **Break** (S40): the first candle after the box whose close > box high (the operator's "entry on the close beyond the box"). Decision time is that candle's close.
  - Variant: the first GDFL second whose printed high exceeds the mapped box high (RES/fib-one-level.json lines 39-45, "or the moment it crosses").
  - Variant: a resting stop-entry at the mapped box high, placed at the box's known-at time + λ.
- **Stop** (S50): below the box low by a generated offset (0 included). Variants: at the next rung below; at L minus an offset; a % of the entry fill (R2).
- **Targets** (S60): the day-high break (first target) and the extensions 1.272/1.618 (operator), plus every option in S60.

### 7.2 Patterns anywhere

The flag is one member of a family. Every member is evaluated at every candle and every level relation, never only at a level:
- P1 one candle covers exactly one rung (fib-one-level lines 11-19);
- P2 K candles in a row touch the same rung (lines 21-27);
- P3 a dip to or below a rung, then a green (long) candle reaching the next rung up, or back to the same rung (lines 29-37), mirrored (lines 65-73);
- P4 partial U: 2 or 3 candles whose lows form a U, measured by the depth, the symmetry and the third candle's close location; mirrored as a partial inverted U (MEM/precise-entry-patterns-anywhere.md line 11);
- P5 touch and turn, confirmed over k candles, k swept (RES/fib-and-seconds.json lines 39-44);
- P6 break through: touched, then closed beyond (lines 45-50);
- P7 wick touch: the high or low touched, the close did not (lines 51-56);
- P8 the box or pause of K candles (7.1), with its rest shape described by measurements (box, U, inverted U, anything; MEM/fno-winners-research.md line 32);
- P0 no pattern: the top-mover filter plus a level relation only. This is the baseline every pattern must beat.

Layer 3 of MEM/miss-no-pattern-three-layers.md (every candle coded from a generated alphabet, code sequences enumerated level-wise with Apriori extinction) is the general form of P1-P8. It is scheduled after the first run (7.6).

### 7.3 Pauses

- A pause is K candles (or N seconds, from GDFL) whose range stays within a generated height, measured against the push, R, the stock's usual candle range at that time of day, and price ppm.
- The pause's ordinal today (1st, 2nd, ...), the staircase (higher box lows), and a failed earlier break are features.
- Seconds-level pause quality (seconds held at the level, how tight, touches; RES/fib-and-seconds.json lines 110-116) is rolled up into the minute that contains it. It is known only at that minute's close (the K18 rule: a window ending after the minute sets its bit at the first candle closing at or after the window's end).

### 7.4 Volume

- Signals come from Zerodha candles, so the volume features are computed from Zerodha candle volume: pause volume against push volume, the candle's volume against the stock's usual volume at that time of day, and relative volume rank.
- GDFL-summed volume per minute is a labelled variant (OP-38). Zerodha 1-minute volumes undercount GDFL by 0.4% to 19.8% (MEM/order-flow-delta-naming.md line 13).
- **Order flow delta** (never bare "delta") per second, summed into the minute, needs a per-second buy/sell classification the store does not have (missing, OF-2). Its variants are refused by name until it exists, never set to zero.

### 7.5 Colour and trigger shape

- Long triggers may require a green candle and shorts a red one (S37, variant "any").
- A flat close is neither colour (D-0109, as bits 30-31 and 276-277 do).

### 7.6 What the first run carries, and what follows

- First run (price action first, MEM/let-data-decide.md line 19): ladders A1-A8 with the rule-3 variants; patterns P0-P8; the rank features; volume from Zerodha; the exits of section 9.
- Later waves, in order: the layer-3 candle-code enumeration; the K18 seconds summaries; order flow delta; the lens-*.json rows; indicators, a separate track (RES/plan.json lines 57-64 and 83-91).
- No later wave changes a rule in this spec. Each adds features and labelled variants.

---

## 8. Signals per timeframe

- A **recipe** is: one direction; one timeframe; a top-mover clause (5); one ladder anchor with its mode and leg rule (6); one pattern with its settings (7); feature clauses (any vocabulary condition, AND-combined by the sweep); one entry trigger; one stop rule; one exit family; the fill settings; one cap tie-break order.
- A signal is a candle (decision time T) on which every entry clause is true. It is evaluated by the existing fold (one candle at a time, CLAUDE.md section 3 rule 7, `Column::build`).
- Every timeframe's candles are folded from the same 1-minute series (4.1). A recipe never mixes two timeframes (strategy-mvp REPORT.md line 134, default (g)); a mixed-timeframe recipe is a later labelled variant.
- The 370 existing conditions (`vocab::table::TABLE`, crates/vocab/src/table.rs line 257) are all usable as feature clauses. Their AND-combinations are what the engine's Apriori sweep enumerates (CLAUDE.md section 6).
- Per the operator's run order, the first run admits only the price-action conditions, and the indicator rows (VWAP, pivots, CPR, EMA, SuperTrend) wait (MEM/let-data-decide.md line 19).

---

## 9. Orders, entries and exits on GDFL seconds

### 9.1 Order kinds

| Order | Starts | Latency | Fills |
|---|---|---|---|
| Entry (decided on a candle) | first traded second >= T + λ | yes | the walk of 9.3 |
| Stop-entry (resting at the mapped box edge) | the first traded second whose printed price crosses the edge, at or after the order's placement (box known-at + λ) | no, once placed | 9.3, starting in the trigger second |
| Stop (resting) | the first traded second whose adverse printed price reaches the mapped stop (long: low <= stop; short: high >= stop) | no | 9.3, starting in the trigger second |
| Target (resting) | the first traded second whose favourable printed price reaches the mapped target | no | 9.3, at the worst price of each second, which may be below a long's target (OP-10) |
| Trailing stop | as a stop. Each move of its level is an order action (4.2) | for each move | 9.3 |
| Time exit (a hold of H, or a clock time) | first traded second >= exit time + λ | yes | 9.3 |
| Condition exit (decided on a candle) | first traded second >= T + λ | yes | 9.3 |
| Broker square-off | 15:10:00 (Dhan RMS for cash intraday; sources-leverage-execution.md line 147, T03-01) | no | 9.3, then the remainder rule of 9.8 |

### 9.2 Entry

- The quantity is fixed at the order's start (section 11).
- Entry orders are market orders. Each carries NSE's market-order price protection, a limit at LTP x (1 + 3%) for a buy (LTP x (1 - 3%) for a sell), or Rs 1.5 for LTP <= Rs 50. The unfilled part rests at that limit as a Day order (sources-leverage-execution.md line 253, T10-05).
- A second whose worst printed price lies beyond the protection limit fills nothing for that order (OP-13).

### 9.3 The ONE fill model and the fill walk

For an order with remaining quantity Q, starting at second s0 and stepping through traded seconds s0, s1, ... in order:

1. A second with zero traded volume fills nothing, under every cap, including no cap.
2. In a traded second s, the order fills
   ```
   q(s) = min(Q, floor(c · v(s)) - used(s))
   ```
   - v(s) is the second's traded volume.
   - c is the swept volume share (S70).
   - used(s) is what this account's other orders on the same stock and side already took from s.
   - floor is the worse rounding. If q(s) <= 0, nothing fills.
   - Under c = no cap, q(s) = Q.
3. Every unit filled in s is at **that second's worst printed price**: a buy at high(s), a sell at low(s) (FIB-I4). This is `costs::fill::Anchor::PrintedExtreme` read per second (crates/costs/src/fill.rs lines 336-357, 449-461).
4. A buy fills nothing in a second whose printed high is at or above the stock's upper price band, and a sell nothing in a second whose printed low is at or below the lower band, until a second prints off the band (section 10.2).
5. Q -= q(s). The walk continues to the next traded second until Q = 0, or until an event of 9.5 or 9.8 ends it.
6. Every partial fill is recorded with (second, quantity, price). The order's price is the volume-weighted average of its fills, kept as an exact rational. Charges are on each fill's value.

Stop or target first in one second: the stop is taken first (the pessimistic reading, as `runner::grid` resolves same-bar ambiguity). For a long both are sells at the same low, so the price is the same and only the label differs.

### 9.4 Stops, targets, trailing and time exits

- **Stop**: always present in the headline. A run without a stop is a labelled comparison only (MEM/fno-winners-research.md line 29). The level is set at the entry decision from the mapped stop rule (6.6), and the trigger is the adverse printed price.
- **Target**: one of S60, or none. It triggers on the favourable printed price and fills at the second's worst price.
- **Trailing stop**: after an arm condition (S61), the level follows the best printed price by a swept distance, or steps to below each new pause's low (stair-step, ST/fno-winners-20261013/pauses.json). Each step is an order action with latency.
- **Trailing take profit**: armed at a target rung; then trails (the existing `grid::Ttp` semantics, crates/runner/src/grid.rs lines 90-110).
- **Break-even move**: after a favourable move of x·R, the stop moves to the entry price; x is swept, including never.
- **Time exit**: a hold of H (candles or seconds), or a clock time. The latest allowed start of a strategy exit is before 15:08:00 (9.8).
- **Condition exit**: on a candle close, e.g. close back under the box low, under the rung, or the rank leaving top N'. Any vocabulary condition may be tried as an exit trigger (MEM/fno-winners-research.md line 29 (f)). A protective stop stays live under a condition exit.
- **Exit family**: whichever comes first among the active exits. Families are enumerated as `runner::grid` does (SL+TP, SL+TSL, SL+TTP, SL+TP+TSL, and so on), with exits as their own dimension, never mixed into entry combinations (MEM/let-data-decide.md line 19).

### 9.5 Events while an entry is still filling

All are pinned to the worse reading, and each is tested:
- **A stop, a target, a time or condition exit, a cap, or the session end** triggers while the entry has filled Q_f > 0 of Q:
  - the entry's unfilled part is cancelled at that second;
  - the exit acts on Q_f only, and starts in the trigger second;
  - the entry and the exit share the one per-second budget (9.3, step 2), because they are one account's orders in one second (OP-11).
- **A stop triggers while Q_f = 0**: the entry is cancelled, and the trade is recorded as "entry cancelled, no fill". It counts toward the 100-trade cap (OP-18) and frees its position slot.
- **The entry's own fill second also triggers the stop**: the stop fires in that second on the quantity filled so far. That is the worst round trip the data can show (buy at the high, sell at the low of one second).

### 9.6 One position per stock

- From the entry order's start to the exit's last fill second, the stock is locked.
- A new entry order on that stock may start only at a second strictly after that last fill second, long and short alike (MEM/fno-winners-research.md line 21).
- A signal while locked is counted as `while_open`, as the runner counts it today (crates/runner/src/trade.rs, rule 4 of the module doc).

### 9.7 Order rate

- Dhan's order API allows 10 orders a second, 250 a minute, 1,000 an hour and 7,000 a day, with 25 modifications per order (sources-leverage-execution.md line 234, T09-07).
- Orders that would exceed a ceiling wait in a queue for the next allowed second.
- Inside one second, exits are sent before entries (risk first), and each group is ordered by the cap tie-break order (11.5).
- A trailing order modified more than 25 times is cancelled and replaced by a new order. That costs nothing extra, but it counts in the per-second budget.

### 9.8 Session end, the broker's square-off and the last self-exit

- Dhan warns that self-exit orders placed in the 2 minutes before the 15:10 square-off may be rejected (sources-leverage-execution.md line 150, T03-04). So a strategy exit order whose start would be at or after 15:08:00 is treated as rejected (worse reading, OP-34), and the position is left to the broker.
- From 15:10:00 the broker squares off every open intraday position (T03-01, line 147). Pending and partial orders are cancelled, and the remainder is walked by 9.3 from 15:10:00.
  - Each broker square-off order pays Dhan's Rs 20 + GST (sources-leverage-execution.md line 151, T03-05).
  - Whether Dhan splits a large position into several square-off orders is UNVERIFIED (line 155, T03-09). The engine counts the larger number of orders under every split rule a source names (the worse reading, OP-27).
- **The remainder rule**: whatever is still open at the last continuous-trading second of (s, D) is closed at the worst printed price over all traded seconds from 15:10:00 to that last second, whatever the volume. This is the operator's "closed at the worst price available".
  - If there is no traded second in that window, the remainder closes at the worst printed price of every traded second after the entry that day, and the trade is flagged `unclosable_in_session`.
- Session ends:
  - before 2026-08-03, 15:29:59 for every stock;
  - from 2026-08-03, 15:14:59 for closing-auction-eligible F&O stocks, because continuous trading ends at 15:15 and the auction runs to 15:35 (sources-leverage-execution.md lines 20 and 276, T11-09);
  - for the 44 non-eligible auction-era stock-days, 15:29:59.
- The closing auction is never used for a fill, and no stop order exists in it.
- **The MTM rule.** Dhan may square off all intraday positions when the MTM loss crosses 80% (T01-17, line 124). What the 80% is of is UNVERIFIED (T01-19, line 126).
  - The engine applies it as 80% of the position's margin, the worse reading of the two named.
  - It closes the position at the next traded second at the walk's prices, plus the square-off charge.

---

## 10. Market rules applied to fills

### 10.1 Price steps

- The exchange tick depends on the price: below Rs 250 it is 0.01, 250-1,000 is 0.05, 1,000-5,000 is 0.10, 5,000-10,000 is 0.50, 10,000-20,000 is 1.00, above 20,000 is 5.00. This holds since 15 Apr 2025, set from the previous month-end close (sources-leverage-execution.md lines 22, 203 and 205; T07-01, T07-03).
- Earlier eras are UNVERIFIED before 2024-02-05 (the security file starts then, line 196).
- So every tick-denominated setting uses the **measured price step** of the stock's own GDFL prints in the learning window (generated, as ST/fib-small-20261024/REPORT.md line 27 did). It never uses `costs::rate::TICK` (crates/costs/src/rate.rs line 259, a fixed 5 paisa).
- Fills are printed prices, so they need no step.

### 10.2 Price bands

- F&O stocks have no fixed circuit. The operating range is 10% of the base price (the previous close), and it flexes 5% at a time in the move's direction after a cooling-off period (sources-leverage-execution.md lines 21 and 189, T06-03).
- The band of (s, D) is taken from NSE's daily security file field PricRg where the file exists, from 2024-02-05 (line 196, T06-10).
- Before that, the band is computed from the base price and the flex rule. The base price in raw terms is C_z(D-1) · ρ(D), where ρ(D) is the session's raw/adjusted ratio at its first traded minute (6.6, R1).
- Rule 4 of 9.3 then applies: a buy fills nothing at or above the upper band, and a sell nothing at or below the lower band.
- **Limit of the data**: the store has no order book, so "at the band" means a printed price at the band limit, not a locked queue. The source-file reading is a missing part (FNO-BAND, section 16).

### 10.3 Short selling

- Same-day shorting of F&O stocks is allowed, and longs and shorts are symmetrical for margin and cost (sources-leverage-execution.md line 38).
- A short that cannot be covered in the session (`unclosable_in_session`) is flagged. Its auction tail risk is described, not priced, because the auction outcome is not in the data (UNVERIFIED T04-08).

---

## 11. Money rules and their tie-breaks

### 11.1 The numbers (operator, fixed; MEM/scope-fno-stocks-first.md lines 25-27)

| Rule | Value |
|---|---|
| Capital | Rs 20,00,000 |
| Margin per position | Rs 1,00,000 |
| Intraday leverage of stock s on D | L(s, D) = min(5, 100 / max(20, M)), M = VaR + ELM % from NSE's C_VAR1 file, batch 1 of D (sources-leverage-execution.md lines 15 and 30; T01-10 line 117; T02-02, T02-03 lines 136-137) |
| Target position value | N(s, D) = Rs 1,00,000 x L(s, D), which is Rs 5 lakh at 5x and smaller below 5x |
| Open positions | at most 20 at once |
| Trades | at most 100 a day |
| Exposure | at most Rs 1,00,00,000 of open position value at any moment |
| Per stock | one trade at a time (9.6) |
| Intraday | everything closed by the session end (9.8) |

### 11.2 Quantity

- The sizing price P_s is the last traded GDFL price at or before the order's start second. It is raw, because shares are counted in raw prices; an adjusted Zerodha price before a later split would mis-size.
- q = the integer closest to N(s, D) / P_s, on either side. It is computed exactly:

  ```
  A/B = N / P_s
  q0  = floor(A/B)
  r   = A - q0·B
  q   = q0 + 1  if 2r > B
        q0      if 2r < B
        q0      if 2r = B   (an exact tie takes the LOWER quantity:
                             less exposure and less margin; OP-14)
  ```

- If q = 0 the trade is refused (`price_above_position`).
- The leverage clocks (operator rule 7) are both read:
  - L_day from D's own file;
  - L_today from today's file;
  - L_extra, the reading with NSE's additional margin (column 9), because whether Dhan counts it is UNVERIFIED (T01-22, line 129).
- The quantity uses the smallest of the readings, so the position is valid under every reading. The walks under each single reading are also run and shown (OP-16, OP-17).

### 11.3 Margin and exposure checks

- **When**: at an entry order's start second, before it is sent.
- **Margin in use** = the sum over open positions and pending entries of value x max(0.20, M/100) (sources-leverage-execution.md line 63). Here value = the quantity times the last traded price. It must stay <= the available capital.
- **Available capital** = Rs 20 lakh plus the carried net P&L (after charges, worse clock) since the start of the forward window (OP-21), less the margin in use.
- **Exposure** = the sum of open and pending position values at their last traded prices, plus this order's q·P_s. It must be <= Rs 1 crore.
- An entry that fails either check is refused (`margin_cap` or `exposure_cap`) and counted.
- Mark-to-market moves never force a reduction, except Dhan's MTM rule (9.8).
- Cost: at most 20 open plus 20 pending rows are summed, a fixed bound. A running sum keeps it O(1).

### 11.4 The 20-position and 100-trade caps

- **A slot is taken** when an entry order is sent. It is released when the exit's last fill completes, or when the entry is cancelled with nothing filled (OP-19).
- **A trade is counted** when its entry order is sent, filled or not (OP-18). The daily count resets at each session.
- **Exactly at a cap**: with 20 slots taken, or 100 entries sent today, every further entry that day (or until a slot frees) is refused (`position_cap` or `trade_cap`) and counted.

### 11.5 Which signal wins when a cap binds (exact tie-break)

1. Earlier order start second first. Entries are queued by the second they may start (T + λ).
2. Within one second, by the recipe's tie-break order, a swept setting picked in the learning window only (S81):
   - (a) the larger |m| (rank strength) first;
   - (b) the smaller stop distance in R first;
   - (c) the earlier first entry into top N today first;
   - (d) symbol order (a neutral, deterministic baseline);
   - (e) a seeded random order, with k seeds generated.
3. A final tie under (a)-(c) falls through to (d).

The report shows the spread of net P&L across the seeded random orders as a luck check, so that a cap tie-break can never manufacture a pass.

### 11.6 Recipe alone, or several together

- Each recipe is first simulated **alone** under these money rules (its own 20 slots, 100 trades and Rs 1 crore).
- A **procedure** (13.4) trades several picked recipes together: the caps and the per-stock lock are shared across them, and one stock's lock blocks every recipe.

---

## 12. Charges (operator rule 7; the table comes from COSTS.md and costs-table.json, written by the costs step in parallel)

- Every trade is charged under two **cost clocks**, both computed, both labelled:
  - **C-DAY**: the official rates in force on the trade's own date, dated, each row citing its official source;
  - **C-TODAY**: Dhan's rates today, as if the trade were placed live today.
- **The headline net, and every pick, use the worse (higher) of the two charges for each trade.** Both columns are shown.
- The charges per trade are the components of RES/sources-charges.md lines 13-23, for R1 cash intraday:
  - Dhan brokerage min(Rs 20, 0.03%) per executed order, both legs;
  - STT 0.025% on the sell side (non-delivery);
  - NSE transaction charge with IPFT on both sides (dated: it changed over time, including 1 Oct 2024; every date is in costs-table.json);
  - SEBI fee Rs 10 a crore on both sides;
  - stamp duty 0.003% on the buy side (unified from 1 Jul 2020);
  - GST 18% on brokerage, exchange charge, IPFT and SEBI fee;
  - the Dhan square-off charge when the broker exits;
  - anything else the official schedules list.
- **Where Dhan's words leave a choice open, each reading is computed and the worse is used** (OP-27):
  - brokerage per executed order vs per fill part (`costs::ledger::ExecutedOrder::with_part` charges once per order; UNVERIFIED for Dhan, D-5127; T12-03 line 285);
  - STT on the day's VWAP for the non-delivery quantity (NSE/CMTR/5588, `costs::stack`, crates/costs/src/stack.rs lines 30-41) vs on the sell value (Dhan's calculator);
  - STT and stamp rounding per security per day (two-step, stack.rs lines 19-28) vs per trade vs per contract note across securities (`ClientDay`);
  - a DP charge on an intraday round trip (not expected; UNVERIFIED, RES/sources-charges.md line 36);
  - auto square-off as one order or several.
- The Dhan data API, Rs 499 + taxes a month (RES/sources-charges.md line 37), is a running cost. It is charged in the account replay, once per calendar month with at least one trade, labelled (OP-28).
- A trade is a win only if its net (worse clock) is above 0 (MEM/backtest-acceptance-bar.md line 15).

---

## 13. Scoring, the requirement and walk-forward

### 13.1 Two scores on one trade list

- **NET**: the volume-weighted fills, less every charge (worse clock, worse reading). This is the headline.
- **SIGNAL ONLY, BEFORE CHARGES**: the same trades and the same worst-price fills, no charges. It is labelled exactly "signal only, before charges" (MEM/fno-winners-research.md line 17).
- The trade list is computed once, with the net equity driving the margin checks. The signal-only score re-scores that list and does not re-run the path (OP-26).

### 13.2 Per-trade record and run statistics

Per trade:
- the stock, direction, timeframe and recipe;
- the signal candle and decision time T;
- every order with its partial fills (second, quantity, price);
- the volume-weighted entry and exit prices;
- the quantity and the leverage readings;
- the stop and target levels in both feeds, and the mapping reading;
- the exit reason;
- seconds to fill entry and exit, and the number of fill seconds;
- the loss beyond the stop;
- MAE and MFE (on printed seconds);
- R and the R-multiple;
- every charge under both clocks and every open reading;
- the net and the signal-only result;
- the INDIA VIX stamp (reference only, CLAUDE.md section 1);
- the data era label.

Per run:
- how often fills spread, and over how many seconds (a distribution, per side and per entry or exit);
- refusals by reason (3.2, 9.5, 11);
- cap hits;
- `unclosable_in_session` count;
- broker square-off count;
- the scorecard of 13.3 per side, per pair, per period.

### 13.3 The requirement (the Winner Gate, 1.5) on net results

Per side (quality):
- win rate >= 51%;
- average win >= 2x average loss;
- profit factor >= 2;
- largest win >= 2x largest loss;
- positive on unseen months.

Per pair (quantity):
- >= 500 trades a year (pro-rated);
- every year positive;
- leave-one-out (best day, week, fortnight, month, quarter dropped, still passing);
- the five downside checks, unseen against learned;
- net positive on unseen months of each NIFTY regime;
- minimum returns of 0.3%/day, 1.5%/week, 6%/month, 18%/quarter, 75%/year (simple, averaged, pro-rated).

The return base is the Rs 20 lakh capital, the newest money rule and the worse reading against the 19 Oct base of peak concurrent positions x Rs 1 lakh (OP-22).

### 13.4 Walk-forward, exactly (operator rule 4)

**Units**: calendar months of tradable days. The span is 2018-09-03 to the last sealed day (M4).
- Before the first learning window, data is used only for causal running features: Zerodha minutes from 2018-01 as warm-up (MEM/backtest-acceptance-bar.md line 36).

**A scheme** is (kind, L, T):
- kind is ANCHORED (expanding) or ROLLING;
- L is the learning length in months (ROLLING only);
- T is the test (forward) length in months.
- The forward windows are W_j = [m_j, m_j + T), with m_{j+1} = m_j + T: they tile the span after the first learning window with no gap and no overlap.
- The learning window of W_j is [span start, m_j) for ANCHORED and [m_j - L, m_j) for ROLLING.

**Generated lengths, not typed** (the scheme family, ST/fib-run-20261016/wfo-family-amendment.txt lines 3-14):
- L doubles from the shortest learning window in which the recipe family reaches the pooled minimum trade count, up to the longest the span allows.
- T doubles from 1 month up to the longest that still leaves at least two forward windows.
- Both are refined by midpoints only where neighbouring lengths give different verdicts.

**In each learning window, using that window's data only:**
1. Generate every fitted setting: bucket edges, thresholds, ladders of stop and target distances, and quantile ladders (section 14, form G).
   - Causal running features (a stock's usual volume at this time of day, its usual daily range) are not fitted. They use all history before each decision, never after it.
   - The runner's excursion ladders (`excursion::Ladder::from_excursions`, crates/runner/src/excursion.rs line 213) are fitted, so they are built from the learning window only (MEM/fno-winners-research.md line 29 notes that today they are calibrated on the same sample).
2. Run every recipe x setting combination over the learning window, under the money rules, net, worse clock.
3. A recipe-setting is **picked** for W_j if it passes the per-side quality gate and its pair passes the quantity checks on the learning window.

**In the forward window W_j:**
- The picked recipe-settings trade with their settings frozen. Results are recorded per W_j.
- A recipe not picked for W_j has zero trades in W_j. That month counts as not positive for the 70%-positive clause (OP-25).

**Recipe verdict.** A recipe is a WALK-FORWARD PASSER only if, under EVERY scheme of the family, the concatenation of its forward windows meets the whole requirement of 13.3. That includes the unseen-month clauses (together they pass, >= 70% positive, none below the learned loss floor) and the downside checks against the learned references.
- Each forward window is reported, and the whole.
- A recipe that passes only some schemes is refused by name, with the schemes it failed.

**Procedure verdict.** A procedure is a selection rule applied in every learning window, traded together in W_j under shared caps (11.6). Examples: "all picked recipes", "the best picked recipe by learning net", "the best k by learning net" with k generated. It is judged by the same rule. Procedures are what a live account would actually run, so both verdicts are reported.

**Luck accounting.**
- Every scheme, recipe, setting and tie-break order tried is a trial. The trial count is reported.
- Deflation and the reality-check family run on all trials (runner `significance`, `pbo`, `bootstrap` modules), so trying more can never manufacture a pass.

**Honest outcome.** "Nothing passed" is reported as plainly as a pass, with the closest misses and by how much (operator rule 5).

**Sealed exam year.** The newest forward windows are reported separately as "newest, never seen at pick time". No year is held back beyond that unless the operator re-affirms the 18 Oct sealed exam (OP-23).

**Implementation.** One visitor pass per scheme. Each learning window's totals are differences of per-month prefix vectors, O(1) per (recipe, month, scheme) (wfo-family-amendment.txt line 14). The part that runs it is FNO-WFT (section 16).

---

## 14. Every setting and its full sweep range

Forms:
- **E** = every value of a finite domain.
- **D** = doubling from the smallest to the largest value of the domain, then midpoints where neighbouring verdicts differ, down to the domain's resolution.
- **G** = generated from the setting's own distribution in the learning window, per stock or pooled as stated: quantiles at 1/2^k from each tail plus the median (k grows until neighbouring quantiles differ by less than the data resolution), plus the domain's end points and the neutral value ("off", 0 or infinity), refined where survivors crowd. This is the rule of MEM/nothing-fixed-everything-generated.md lines 11-16.
- **V** = an enumerated set of labelled readings. All are run, never one silently.

| ID | Setting | Domain | Form | Resolution | Notes |
|---|---|---|---|---|---|
| S01 | direction | long, short | V | - | separate searches; pair gate (1.5) |
| S02 | timeframe | 1, 2, 3, 5, 10, 15, 30, 60 min, 1 day | E | - | operator requirement |
| S10 | % move basis | previous 1day close; previous last 1-min close | V | - | OP-04 |
| S11 | rank edge N | 1 .. pool size on D | D | 1 | also as a pool fraction q = N/pool, D over 1/pool .. 1; "single stock" is N = 1 |
| S12 | rank state | in top N; entered on this candle; entered within W candles; entry ordinal = k | V | - | W over 1 .. candles so far (D), k over 1 .. max seen (E) |
| S13 | minimum abs % move | 0 .. max seen | G | 1 ppm | per stock, same time-of-day slot; 0 = off |
| S15 | stale-price limit for the pool | 0 .. 375 min, plus infinity | D | 1 min | infinity = carry forward |
| S20 | ladder anchor | A1-A9 (6.1) | V | - | with each anchor's own parameters below |
| S20a | opening range N | 1 .. session length in minutes | D | 1 min | every N minute by minute in the limit |
| S20b | last k sessions | 2 .. sessions in the learning window | D | 1 | k = 5 exists |
| S20c | swing confirmation k | 1 .. 2x the median swing length | D | 1 candle | known only after confirmation |
| S21 | ladder mode | running; frozen at the box start; frozen at the push end | V | - | FIB-I2 tests per mode |
| S22 | rung | 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1; extensions 1.272, 1.618, 2, 2.618, 3; and the continuous position p | E + G | p: 1 ppm of R | extensions in the move's direction (6.2) |
| S23 | same-candle rule | engine; V-SEC (with the 1-min sub-variant); V-BOTH | V | - | operator rule 3 |
| S23a | feed agreement e | 0 .. max seen | G | 1 ppm | V-SEC only |
| S24 | touch tolerance | exact cover; distance in fraction of R, price ppm or price steps | V x G | 1 ppm, 1 step | includes 0 and the pinned 1% of R |
| S25 | minimum range so far / usual daily range | 0 .. max seen | G | 1 ppm | 0 = off |
| S26 | earliest decision time | 09:15 + 0 .. last entry | D | 1 min | the "minutes after open" question |
| S26a | packing limit (rung gap in steps) | 0 .. max | G | 1 step | 0 = off |
| S30 | pattern | P0-P8 (7.2) | V | - | P0 is the baseline |
| S31 | box or pause length K | 1 .. candles available | D | 1 candle | also N seconds on GDFL: 1 .. 3,600, D, 1 s |
| S32 | box height | 0 .. max, in fraction of push, R, usual candle range or price ppm | G | 1 ppm | infinity = off |
| S33 | holding rule | all lows >= L; all closes >= L; no close below the first box candle's low; none | V | - | L from S22 |
| S34 | push size, speed, volume, rungs crossed, new highs | each 0 .. max | G | as unit | each includes "any" |
| S35 | pause volume / push volume | 0 .. max | G | 1 ppm | "any" included |
| S36 | partial U shape: candles, depth, symmetry, close location | candles 2, 3; the rest 0 .. max | E + G | 1 ppm | mirrored for shorts |
| S37 | trigger colour | side colour; any | V | - | flat is no colour |
| S38 | order flow delta sign | same as side; any | V | - | refused until OF-2 exists (7.4) |
| S39 | pause ordinal today | 1 .. max seen | E | 1 | |
| S40 | entry trigger | candle close beyond; first GDFL second beyond; resting stop-entry | V | - | 7.1 |
| S41 | latency λ | 0 .. the timeframe's length in seconds (86,400 bounded to the session for 1 day) | D | 1 s | headline only at λ >= the calibrated floor (4.2) |
| S42 | last entry time | the first decision .. 15:07:59 | D | 1 min | facts cap it (9.8) |
| S43 | 1 day entry time on D+1 | 09:15:00 + 0 .. the last entry | D | 1 min | |
| S50 | stop placement | beyond the pattern extreme + offset; the next rung below + offset; L - offset; % of entry fill | V | - | always present in the headline |
| S50a | stop offset | 0 .. max, in price steps, ppm or fraction of R | G | 1 step | 0 included |
| S51 | level mapping | R1; R2; R0 (when live) | V | - | 6.6 |
| S60 | target | day-extreme break + offset; an extension rung; the next rung; an R-multiple; an excursion-quantile ladder; none | V + G | 1 ppm | none = no target |
| S61 | trailing stop: arm and distance | arm 0 .. max R (G), or at a rung; distance G, or stair-step to each new pause low | V + G | 1 ppm | off included |
| S61a | trailing take profit: arm and trail | as `grid::Ttp` | G | 1 ppm | off included |
| S62 | time exit | hold 1 candle .. to 15:07:59; or a clock time | D | 1 candle, 1 min | off included |
| S63 | condition exit | any vocabulary condition or rank clause; none | V | - | a protective stop stays |
| S64 | exit family | every non-empty combination of S50, S60, S61, S61a, S62, S63 that includes a stop; whichever first | E | - | stop-less only as a labelled comparison |
| S65 | break-even move at x·R | 0 .. max, plus never | G | 1 ppm | |
| S70 | volume share c | no cap; 1; halving 1/2, 1/4, ... down to c_min | D | halving, then midpoints | c_min = the smallest share at which that stock's median position still fills within the session, in the learning window (G) |
| S81 | cap tie-break order | (a)-(e) of 11.5 | V | - | picked in learning; seeded spread reported |
| S90 | cost clock | C-DAY, C-TODAY | rule | - | both computed; the worse per trade |
| S91 | open charge readings | each reading of 12 | rule | - | all computed; the worse used |
| S92 | leverage reading | L_day, L_today, L_extra | rule | - | quantity uses the minimum; each single walk shown |
| S100 | walk-forward kind | anchored; rolling | V | - | |
| S101 | learning length L | min-trades length .. span | D | 1 month | |
| S102 | test length T | 1 month .. longest leaving two forward windows | D | 1 month | |
| S104 | selection procedure | each recipe alone; all picked; best k by learning net | V + G | k: 1 | |

Not settings (facts or the operator's numbers):
- the requirement thresholds (1.5);
- the money rules (11.1);
- 15:10 square-off, 15:08 rejection window, continuous-session end times, order-rate ceilings, the 3% market protection, bands, ticks (sourced facts);
- the worst-price rule (FIB-I4).

---

## 15. Records, audit and run identity

- **Run identity.** CLAUDE.md section 3 rule 3 (`blake3(mask ‖ direction ‖ instrument ‖ timeframe ‖ params ‖ data_digest ‖ vocab_version ‖ commit ‖ feed)`, runner `identity::Run`, crates/runner/src/identity.rs line 336 `feed`) gains these terms. Adding a term is a migration (identity.rs lines 327-335), so FNO-SEC-WIRE records a decision entry for it.
  - the signal feed and the fill feed ("zerodha-candles / gdfl-1s-worst-printed");
  - the fill settings (c, λ, mapping reading);
  - the cost-table version (both clocks; `costs::schedule::TODAYS_RATES_VERSION` crates/costs/src/schedule.rs line 599, and the new dated equity table version);
  - the leverage-table version;
  - the universe digest (fno218.txt, the joined-spelling map, the membership file digest `Membership::file_digest_hex`);
  - the skip-table digest;
  - the walk-forward scheme;
  - the pool data digest. A cross-section run spans every stock, so every member's identity includes the pool's digest, per the K16 law.
- **Records.** Every decision, signal, order, partial fill, refusal, skip, cap hit, square-off and charge line is written to the store's own append-only records under a new versioned record family.
  - Tonight and until merge, these go only to a shadow store under work-20260925/shadow/, never runtime/.
  - They are served read-only by api routes for searching, monitoring and the dashboard, with the survivorship label on every page.
- **Logs.** Events go to telemetry at structural boundaries (per run, per stock-month, per stride), never inside the per-second loop (gate 17, CLAUDE.md section 5).
- **Lean mode.** A sweep writes only aggregates per candidate. Full trade records are written only for picked recipes and on request (FNO-LEAN), because one stock-year of evidence files measured about 96 GB (strategy-mvp REPORT.md line 91).

---

## 16. Code map: what exists, and what is missing

| Piece | Exists (file and line) | Missing (part) |
|---|---|---|
| Universe of 218, joined spelling, label | `FNO_UNDERLYINGS` 208 shares (core/src/universe.rs line 914); `is_sweepable` (core/src/instrument.rs lines 354-372); held rename (core/src/alias.rs lines 549-553) | the 218 list as data, the join map, the label: FNO-LAW (CLAUDE.md section 1 change) + FNO-DAYMASK |
| PIT membership | `Membership::is_member/members/pool_size/known_span` (pull/src/fno_membership.rs lines 2585, 2533, 2545, 2391); `day_universe` (pull/src/day_universe.rs line 77) | no sweep calls it (strategy-mvp REPORT.md line 28): FNO-DAYMASK |
| Calendar | `pull::calendar::kind_of` (pull/src/calendar.rs line 654) | special-session and auction-era session ends per stock-day: FNO-DAYMASK |
| Skip rules | backtest-skip.tsv (79 rows, data) | read-time skip table and its digest: FNO-DAYMASK |
| Top-mover rank | none (no rank, % move or volume condition among the 370; `vocab::table::TABLE` line 257) | cross-section block K16: FNO-RANK |
| Running day ladder | `CurDayFib` (indicators/src/lib.rs lines 400-529), rungs (line 90), bits 121-131 (vocab/src/table.rs lines 436-451), near tolerance (vocab/src/tolerance.rs line 121) | direction-aware extensions, 0.312 and 0.65, generated tolerance, frozen mode, the continuous position: FNO-LADDER (K07) |
| Other ladders | previous day and 5 sessions (indicators/src/fib.rs lines 56-120), gap (indicators/src/gap.rs), ORB 5/15/30/60 bits 86-105 | push, swing leg, any-N opening range, any k sessions, measured move: FNO-LADDER |
| Same-candle variants | engine rule only (`Leg::Undetermined`, lib.rs lines 233-245, 506-529) | V-SEC, V-BOTH: FNO-LADDER |
| Pause / box / partial U | none | K06 pause machine: FNO-PAUSE |
| Volume features | VWAP only (bits 52-53, 143-152, 190-197) | K13-style volume features: FNO-PAUSE (pause volume) and later waves |
| Order flow delta | none | OF-2 (later wave) |
| Signals on 1-60 min | engine sweep over 370 conditions (cli/src/lib.rs line 10213, `EVERY_RUNG`), Apriori (CLAUDE.md section 6), `vocab::expression` (MAX_INSTRUCTIONS 1151, vocab/src/expression.rs line 15) | widen `ConditionMask` (6 words, vocab/src/mask.rs line 18; 14 free positions, `NEXT_FREE` 370) for the new blocks: OP-37 |
| 1 day signals | refused on purpose (cli/src/lib.rs lines 10217-10225) | decide D, trade D+1: FNO-1DAY |
| Next-candle alignment | `align::onto_execution` (runner/src/align.rs line 112) | execution on GDFL seconds: FNO-SEC-EXEC |
| Fill price law | `Anchor::PrintedExtreme` (costs/src/fill.rs lines 336-357, 449-461) | per-second use, the volume walk, bands, protection: FNO-SEC-EXEC |
| Exits | `runner::grid` SL/TP/TSL/TTP and families (grid.rs lines 90-140; `variants` line 1396); time exit and 15:10 (`FORCED_EXIT_MINUTE`, runner/src/outcome.rs line 82); the signal-candle stop (runner/src/signal_candle_stop.rs) | seconds-based exits. Note: `grid::level_fill` fills at the resting level or the open (grid.rs lines 5466-5487), which differs from FIB-I4, so it must NOT be reused for the seconds walk. Condition exits: FNO-SEC-EXEC + FNO-EXITS |
| Stop and target ladders | `excursion::Ladder::from_excursions` geometric quantiles (runner/src/excursion.rs line 213, lines 270-300) | fitted in the learning window only: FNO-WFT |
| One position | one global lock `GlobalSinglePositionV1` (runner/src/portfolio.rs line 341) | per-stock lock, 20 slots, 100 trades, exposure, margin, tie-break, order rate: FNO-PORTFOLIO (or inside FNO-NET) |
| Sizing and leverage | none | quantity rule, dated leverage from NSE files: FNO-NET + FNO-LEVERAGE (the margin-file reader) |
| Charges | contract-note stack for equity intraday (costs/src/stack.rs lines 1-60), per-day ledger (costs/src/ledger.rs lines 84-130, 171, 307), Dhan calculator for comparison (costs/src/dhan_calc.rs), rate selector `CurrentOnEveryDate` / `Dated` with Dated refusing all but index options (costs/src/schedule.rs lines 585-592) | dated equity tables, two clocks with the worse used, open readings: FNO-COSTS; wiring: FNO-NET. Today every equity result is gross (runner/src/audit.rs lines 441-460) and `costs::scope::Segment` has no equity variant (costs/src/scope.rs line 50) |
| Walk-forward | anchored folds `split::anchored_folds` / `purged_folds` (runner/src/split.rs lines 85, 276); `validate` (runner/src/validate.rs) runs anchored walk-forward on one series | the scheme family over the pooled universe by month, per-window generation, recipe and procedure verdicts: FNO-WFT |
| Ranking of passers | `runner::topn` keeps at most 25 (`MAX_TOP`, runner/src/topn.rs line 39) | a typed keeper count conflicts with MEM/nothing-fixed-everything-generated.md line 26. The report lists every passer: FNO-WFT |
| Run identity | `identity::Run` with `feed` (runner/src/identity.rs line 336) | new terms (15): FNO-SEC-WIRE |
| Lean run mode | none | FNO-LEAN |
| Price bands | none | FNO-BAND: the PricRg reader from 2024-02-05 + the computed band before |
| Clock calibration | none | FNO-CLOCK (D-0802 phase 2): per-stock GDFL clock offset and back-step |
| GDFL second access | 1s rung in the store (store/src/path.rs lines 395-414) | the cost of reaching one second: ENGINE.md (the engine step, in parallel) |

The seven strategy-mvp parts (REPORT.md lines 51-58) map here:
- FNO-LAW to sections 3 and 16 row 1;
- FNO-SEC-EXEC to 9-10;
- FNO-DAYMASK to 3;
- FNO-SEC-WIRE to 15;
- FNO-LEAN to 15;
- FNO-NET to 11-12;
- FNO-1DAY to 4.3;
- FNO-PORTFOLIO to 11.

The strategy's own blocks add FNO-RANK (K16), FNO-LADDER (K07, rules 2 and 3), FNO-PAUSE (K06), FNO-WFT, FNO-BAND, FNO-CLOCK, FNO-LEVERAGE and FNO-EXITS. The design step sizes them.

---

## 17. Test obligations (every part that touches the piece)

**Ladder (FIB-I2, FIB-I3), on every timeframe and every anchor:**
- the candle that sets a new high is judged against the old ladder;
- a candle that engulfs the range: judged, then leg Undetermined;
- the first candle alone: no ladder;
- (true, true), then (false, false): still Undetermined;
- then a one-sided extreme: the ladder is used from the NEXT candle;
- the frozen ladder never changes after its known-at time;
- the opening range of N is first used at 09:15+N on each timeframe, including non-dividing ones (2, 3 minutes);
- a swing is used only after its confirming candle;
- yesterday is used from today's first candle;
- the seconds walk at second s never reads a ladder with K > s;
- V-SEC: max and min in the same second; feeds disagree, so fall back; the sub-variant on 5 minutes;
- V-BOTH: both ladders labelled;
- a property test: for random sessions, no emitted bit or level depends on any candle at or after the judged one (shuffle the future, the output is unchanged).

**Rank:**
- ties at the top-N edge;
- a stale stock;
- a stock with no candle yet;
- the pool changing on a membership day;
- the 12 no-Zerodha stocks absent;
- a Kite-hole minute;
- a demerger ex-day skip;
- 218 padded to 256 with fixed work;
- a property: the rank at T is unchanged when any candle after T is altered.

**Fill walk:**
- a zero-volume second;
- a no-trade gap of minutes;
- c = no cap, 100%, 50%, 25%, 10%, c_min;
- floor(c·v) = 0;
- the entry and the stop in one second;
- a stop while Q_f = 0;
- a target filled below its level;
- a band-limit second for a buy and for a sell;
- 3% protection exceeded;
- a halt (frozen price, zero volume);
- the 2019-05-30 partial day;
- 2021-02-24;
- auction-era 15:15-15:29 and the dropped auction print;
- the session end with a remainder;
- no trade after 15:10 (`unclosable_in_session`);
- an exit order start at 15:07:59 and at 15:08:00;
- the 10-orders-a-second queue with 20 simultaneous exits;
- 25 modifications;
- the MTM 80% rule.

**Money:**
- a quantity tie exactly at x.5 (the lower is taken);
- q = 0;
- leverage below 5x (for example M = 25% gives 4x, and M = 22.11);
- the three leverage readings;
- the 20-position cap hit exactly (the 21st refused);
- the 100-trade cap hit exactly (the 101st refused), with cancelled entries counted;
- exposure at exactly Rs 1 crore, then +1 paisa;
- margin exhausted by carried losses;
- one trade per stock across recipes;
- every tie-break order;
- the seeded spread.

**Charges:**
- every rate change date in costs-table.json, on both sides of each date (including the NSE transaction charge of 1 Oct 2024 and the unified stamp duty of 1 Jul 2020);
- both clocks;
- every open reading;
- the square-off fee;
- the contract-note vectors of COSTS.md.

**Day mask:**
- every row of backtest-skip.tsv;
- BE/T2T;
- Muhurat;
- renames (ICICIPRULI / ICICILIFE on 2026-10-22 and 23; ZOMATO / ETERNAL);
- membership refused days;
- leavers absent;
- the label on every result and page.

**Walk-forward:**
- every setting is regenerated per learning window (a mutation of a forward-window value changes no learning output);
- the forward windows tile the span;
- not-picked months count as not positive;
- under every scheme;
- the nothing-passed path is reported.

**Benches (p50 and p99):**
- one candle step per stock (ladder, pause, rank features);
- one minute of the cross-section;
- one second of the fill walk with 20 positions and 20 pending orders;
- one charge-ledger add.

Each bench proves p99 is flat across the sizes (1x, 10x, 100x the history).

---

## 18. Open points, each with the choice that keeps the bar

| # | Open point | Choice | Why it keeps the bar |
|---|---|---|---|
| OP-01 | CLAUDE.md section 1 names the 208 shares; the operator's universe is the 218 | FNO-LAW amends section 1 with a decision entry; the list is data with its digest in the run identity | section 3 rule 2: no silent scope change |
| OP-02 | ICICIPRULI to ICICILIFE before D-7311 ships | a read-time join map (3.1); membership under either spelling; Oct 2026 skipped | the operator's join, without editing the store tonight |
| OP-03 | Rank pool: the 218 only, or every PIT member | the 218 that are members on D; the excluded PIT count shown per day | the 21:19 decision says leavers are not added; the bias stays visible |
| OP-04 | % move basis: the 1day close or the last 1-min close | both, labelled (S10) | which equals NSE's official close is UNVERIFIED; nothing silent |
| OP-05 | Rank ties | competition ranking (5.2) | order-independent, deterministic, no arbitrary symbol order |
| OP-06 | Stale prices in the pool | carry forward, flagged; the stale limit swept (S15) | no look-ahead; no silent drop |
| OP-07 | GDFL clock offset against Zerodha (D-0802 phase 2 not built) | the headline only at λ >= the measured backward step per stock-day; smaller λ labelled "uncalibrated clock" | no possible look-ahead through clock skew |
| OP-08 | Mapping Zerodha levels to GDFL | R1 and R2 both; R0 once R4-26 is live | inside one session, so FIB-I8 holds; both readings shown |
| OP-09 | Feed agreement for V-SEC; the finer-Zerodha sub-variant | relative tolerance e generated (0 included); the sub-variant labelled | the operator's "only where both feeds agree"; a third reading, never silent |
| OP-10 | A target filled at the second's worst price can be below a real limit order's price | the operator's one fill model is kept; labelled "harsher than a resting limit" | FIB-I4; never a best case |
| OP-11 | A stop that triggers in the entry's own second | the exit starts in that second, sharing one per-second volume budget | volume never counted twice; the worst round trip the data shows |
| OP-12 | Price bands before 2024-02-05 | computed from the base price and the flex rule (10.2); FNO-BAND | a sourced rule, applied the strict way |
| OP-13 | Market protection: how much fills inside a second that crosses the limit | nothing fills in that second; the order rests | the unknown share is not assumed in the trade's favour |
| OP-14 | Quantity tie at exactly x.5 | the lower quantity | less margin and exposure; deterministic |
| OP-15 | Sizing price | the GDFL LTP at the order's start | raw shares; known at that moment |
| OP-16 | Leverage clocks (the day's file, today's) | the quantity uses the smaller; both single walks shown | valid under both clocks |
| OP-17 | NSE's extra margin in Dhan's MIS limit (T01-22 UNVERIFIED) | a third reading, inside the minimum | the open reading taken the strict way |
| OP-18 | What counts as one of the 100 trades | each entry order sent, filled or not | a live cap counts what is sent; deterministic |
| OP-19 | When a position slot is taken | at the order's send, until the last exit fill or a zero-fill cancel | the broker blocks margin at placement |
| OP-20 | Which signal wins a cap | earliest start second, then the swept tie-break (11.5); seeded spread reported | picked in learning only; a lucky order cannot make a pass |
| OP-21 | Equity: carried from the window start, or reset daily | carried (live-like) as the headline; reset daily shown | a live account carries its losses |
| OP-22 | Minimum-returns base: Rs 20 lakh (30 Oct) or peak concurrent x Rs 1 lakh (19 Oct) | Rs 20 lakh; the 19 Oct base shown | newest rule; it is also the stricter one |
| OP-23 | The 18 Oct sealed exam year against the 30 Oct rule 4 | walk-forward over the whole span; the newest forward windows reported separately; no year held back unless re-affirmed | rule 4 is the newest binding rule; nothing is seen at pick time |
| OP-24 | Recipe-level or procedure-level walk-forward | both; a recipe counts only on its own walk-forward | a live account runs a procedure; a recipe must stand alone |
| OP-25 | Forward windows where the recipe was not picked | zero trades; counted as not positive | stops "positive months" being inflated |
| OP-26 | Does the signal-only score get its own path | no: the same trade list is re-scored | the two scores compare like with like |
| OP-27 | Dhan's open readings (brokerage on pieces, STT basis, rounding, DP, square-off split) | each computed; the worse used per trade until a contract note settles it | operator rule 7 |
| OP-28 | The Dhan data API Rs 499 + taxes a month | charged in the account replay per month with trades, labelled | an official schedule line; the stricter reading |
| OP-29 | Muhurat and special sessions | masked (M9) and counted until Dhan's square-off for them is sourced | no unsourced session clock |
| OP-30 | 2021-02-24 outage | decisions only up to 11:39; open positions closed at the worst price in 15:45-end | the worst reading of an unknown broker action |
| OP-31 | 2019-05-30 morning missing in GDFL | orders before 11:57:47 refused, never delayed | no fabricated delay or price |
| OP-32 | BE/T2T days | masked (M8, T01-18) | intraday not allowed there |
| OP-33 | The MTM 80% rule's base (T01-19) | 80% of the position's margin | the stricter reading of the two named |
| OP-34 | The last self-exit time | an exit started at or after 15:08:00 is treated as rejected | T03-04, the strict way |
| OP-35 | `CurDayFib` extensions lie on the wrong side for a target | new direction-aware ladder (FNO-LADDER); bits 121-131 untouched | append-only vocabulary |
| OP-36 | `TOL_FIB_MILLI` pinned at 1% (D-0076) against "nothing static" | the new block generates tolerances; the pinned bits stay as one point; a decision entry records it | no renumbering, and no fixed value on the new path |
| OP-37 | Mask width: 14 free positions; K06/K07/K16 need about 1,200 | widen `ConditionMask::WORDS` in the same change that appends (the const assert in vocab/src/table.rs lines 1448-1453 requires it), or a typed feature record; the design decides and records it | the build fails rather than silently dropping bits |
| OP-38 | Volume from Zerodha candles or GDFL seconds | Zerodha (requirement) as the headline; GDFL-summed labelled | signals come from Zerodha; the undercount stays visible |
| OP-39 | Tick-denominated settings | the stock's measured raw price step in the learning window | the exchange tick is UNVERIFIED before 2024-02-05; `costs::rate::TICK` is a fixed 5 paisa |
| OP-40 | 1 day entry on D+1: the pre-open | not modelled; the first continuous second + λ; later starts swept | no pre-open data in the GDFL seconds (UNVERIFIED) |
| OP-41 | Kite-hole minutes inside signal candles and ladders | no decision on an incomplete candle; a tainted ladder is unused; the GDFL-filled variant labelled | the audit's rule; a missed extreme cannot leak in |
| OP-42 | Order flow delta | variants refused by name until OF-2 exists | never a silent zero |
| OP-43 | Exits and entries in one second over the order-rate ceiling | exits first, then the tie-break order; queue to the next second | risk first, deterministic |
| OP-44 | The 1day sweep refused by law (cli/src/lib.rs lines 10217-10225) | FNO-1DAY records a decision: allowed only as decide D, trade D+1 | look-ahead stays impossible |
| OP-45 | "Today" as the end date | the last day both feeds are proven and sealed; provisional 1day bars excluded | only proven data enters a verdict |
| OP-46 | GDFL seconds are about one price each (98.2% single-price) | stated on every report (section 19) | honest limits, CLAUDE.md section 3 rule 6 |

---

## 19. The data's limits (stated on every report)

- **GDFL holds trades, not the order book.** The worst printed price in a second is the worst the data can show. 98.2% of GDFL seconds are single-price snapshots (fno-data-audit REPORT.md line 177), and on RELIANCE 28 Mar 2025 only 0.57% of traded seconds had a high different from the low (strategy-mvp REPORT.md line 99).
  - Zerodha shows minute extremes beyond every GDFL second in 26-47% of minutes (same report, line 100).
  - So the worst printed second is optimistic against the true worst, and every report says so.
- **The two feeds are close but not interchangeable.** All four prices agree on 57.1% of minutes, and the median gap is 4.55 bp where they differ (audit REPORT.md lines 176-177).
- **Survivorship.** Current F&O members only (survivorship bias not removed). The count of PIT members outside the 218 is shown per year.
- **Corporate actions.** The table (R4-26) is not live. 107 raw GDFL steps over 80 stocks are unmarked (audit REPORT.md line 149). Cross-feed mapping is in-session only.
- **Unverified rules** used in the worse reading:
  - Dhan's per-stock MIS list (T01-14);
  - the extra margin (T01-22);
  - the MTM base (T01-19);
  - the square-off split (T03-09);
  - brokerage on parts (T12-03);
  - halt windows (T06-07);
  - Muhurat square-off.
- **The data is not proven yet.** The audit verdict on 30 Oct was NOT COMPLETE (REPORT.md line 1). The full backtest starts only after the data is proven (target Sun 1 Nov) and the parts are merged (MEM/scope-fno-stocks-first.md line 36).
