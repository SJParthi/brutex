PREVIEW (the real Fibonacci strategy, not the production fib backtest of Thu 12 Nov): specification only, nothing built or run

# PREVIEW SPEC: the operator's real Fibonacci strategy on NSE F&O stocks (top-mover flag continuation with the Fibonacci ladder, plus the second-touch entry), judged against Rs 2,50,000 in every unseen month

Written Fri 6 Nov 2026, 20:57 to about 22:xx IST, by a workflow subagent of main session 641cf96a. The operator's ask, relayed verbatim with the task: "Since psrb doesn't look like really making money dude why can't we check our real Fibonacci define dstartgeu dude okay?" (read: why not check our real, defined Fibonacci strategy).

Status: specification only. Nothing was built, compiled, run, merged or committed; no ref was touched; the store and every other folder were only read. This is not a result and not a Winner Gate verdict. Every output of the preview carries, on its first three lines:
1. `PREVIEW FIB: the operator's real Fibonacci strategy (SPEC.md section 1 with the second-touch entry); not the production fib backtest of Thu 12 Nov; not a Winner Gate verdict`
2. the verdict line `AT LEAST ONE RECIPE MEETS RS 2.5 LAKH EVERY UNSEEN MONTH: <YES | NO | PENDING>` (section 14.5)
3. `current F&O members only (survivorship bias not removed)` (SPEC.md:7, word for word)

## Path keys

- W = `/Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925`; REPO = `/Volumes/WD_BLACK/brutex/fresh-20260919/project` (the git common dir of every scratch tree, `git -C W/scratch/psrb-preview-r3 rev-parse --git-common-dir`). The task text names `W/project`: that folder does not exist (checked 21:0x IST); REPO is the repository meant.
- STORE = `/Volumes/WD_BLACK/brutex/fresh-20260919/runtime/store` (read-only).
- SPEC, PARTS, COSTS = `W/state/fib-strategy-20261030/{SPEC,PARTS,COSTS}.md`; INPUTS = `W/state/fib-strategy-20261030/inputs/`.
- BRIEF = `W/state/owed-20261103/brief-second-touch.md` (after its five repairs, lines 5-13); DRAFTK = `W/state/owed-20261103/bar-2p5-fib-winner-gate-draft.md` (Variant K, DRAFT NOT APPLIED, its line 1).
- REQ = `W/state/operator-requirements-20261106.md`; R3 = `W/state/round3-20261106/ROUND3-SPEC.md`; BP = `W/state/round3-20261106/BUILD-PLAN.md`; DX = `W/state/round3-20261106/design-complete.md`.
- F2 = `W/scratch/psrb-preview-r2-fno/crates/fno_r2/src` (round 2, F&O stocks, audited); P2 = `W/scratch/psrb-preview-r2-search/crates/psrb_r2/src`; RF = `W/scratch/psrb-preview-fno-rigor/crates/fno_search2/src` (the F&O rigor layer).
- U = `/Users/parthi/.claude/projects/-Volumes-WD-BLACK-brutex/memory` (user memory); MEM = `/Users/parthi/.claude/projects/-Volumes-WD-BLACK-brutex-fresh-20260919-project/memory` (project memory, as SPEC.md:10 names it).
- PV = `W/state/fib-preview-20261106` (this folder: the only state folder the preview writes); TREE = `W/scratch/fib-preview-1`.
- Part tips (read with `git -C REPO log` and `git show`, 6 Nov 21:0x IST): FNO-LADDER `9caca9523` (check r4 PASS, `check-FNO-LADDER-r4.md`:3), FNO-RANK `079b33146` (check r2 PASS, `check-FNO-RANK-r2.md`:3), FNO-COSTS `c8f7608b5cbc` (check r2 PASS, merged into the scratch base `a6ba0d5d4`, whose subject is "scratch psrb-preview merge c8f7608b5cbc..."), FNO-SEC-EXEC `d35888c0c` (check r2 FAIL 5 Nov; repair 2 committed 6 Nov 11:46 IST, not re-checked: `progress-FNO-SEC-EXEC-repair2.md`), FNO-PAUSE `f8f6ace56` (check r1 FAIL F1; repair 1 under check r2 since 20:03 IST: `check-FNO-PAUSE-r2.progress.md`), FNO-BAND `2d785d8d3` (no check on record, R3:687), FNO-STRAT `2327b1eda` (built, gates at the tip, not checked; `touch.rs` 1,151 lines). Each of the five part tips LADDER, RANK, SEC-EXEC, PAUSE and BAND sits on FNO-SEAM's passed tip `1ad6e24ef` and changes only its own module, one `pub mod` line under its slot in `lib.rs`, its bench rows and docs 04-07 (`git diff --name-only 1ad6e24ef <tip>`); `1ad6e24ef` is an ancestor of `a6ba0d5d4`, where the slot lines exist and no `fno_*` module does. FNO-STRAT's branch holds merges of other parts, so only its `touch.rs` is taken (7.2 item 13).

---

## 0. The answer in brief

1. **What is tested.** Exactly the strategy the operator defined (SPEC.md section 1, quoted in section 1 below): on every NSE F&O stock, cash, intraday, a stock among today's top-N gainers (long) or losers (short) pauses in a tight box that holds above (below) the 0.382 retracement of the day's range so far; the entry is the close beyond the box, the stop just beyond the box, the targets the day-high (low) break, then 1.272 / 1.618. Beside it, the operator's 2 Nov entry: the SECOND touch of the SAME Fibonacci level, entered on the return. Both are one strategy: the same rank, the same ladder, the same fills, charges and bar.
2. **Seeds, then the data.** The operator's own numbers run first, unchanged and labelled "seed" (wave 0). Then every dial is varied around them by values generated from each stock's own past percentiles inside each walk-forward learning window, coarse to fine (waves 1-2). Nothing else is typed.
3. **Judged as live Dhan trading.** 14 s latency, 10% of each GDFL second's traded volume, every slice at that second's worst traded price, 3% market protection, price bands, 15:08 self-exit cut, 15:10 broker square-off with its fee, MTM 80%, one position per stock, Rs 1 crore exposure, equal amount per entry with N generated from the seeds 1 / 3 / 5 / 10 / 20, every charge on two clocks with the worse used.
4. **The bar and the honesty.** Anchored yearly walk-forward, 82 unseen months 2020-01..2026-10 on one account from Rs 20 lakh; THE BAR is Rs 2,50,000 net in every unseen month (and 12.5% of a grown month-start capital), the +2.5% tier beside; a luck test that counts every option tried and a random-entry twin decide whether a pass is real; the Fibonacci Winner Gate Variant K is printed beside the bar for every shown recipe; nothing passes by a kinder lane.
5. **What it reuses.** The passed fib parts as they are (FNO-LADDER, FNO-RANK, FNO-COSTS), the unpassed execution and pattern parts at their tips with labels (FNO-SEC-EXEC, FNO-PAUSE, FNO-BAND, FNO-STRAT's touch book), and round 2's audited F&O tool for the store door, the day mask, the exact accounts, the walk-forward and the report. Every departure from the production engine is listed in section 18 (DV-01..DV-31).
6. **Cost (ESTIMATE, section 17.8).** About 5.5-6.5 builder-days; about 40-200 CPU-hours to the verdict at 8 threads; under 4 GB per process; the first honest interim (the operator's seeds alone) after about 3 builder-days. No calendar date is promised (U/one-date-set-per-day).

---

## 1. The strategy as the operator defined it (quoted by file and line)

### 1.1 The main strategy (SPEC.md section 1.1, lines 36-64)

| # | Operator's definition (SPEC.md line; the memory line it quotes) | Preview rule |
|---|---|---|
| O1 | F&O stock winners are "stocks entering the top 10 gainers/losers (% vs yesterday's close)", pausing "within 1 to a few minutes" at a Fibonacci level of "the day's low-high range so far"; levels 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1 (SPEC:38; MEM/fno-winners-research.md:11). 0.312 and 0.65 are not rungs (operator 31 Oct ~08:4x, SPEC:374, PARTS:1272-1273) | sections 5, 6, 7.2 |
| O2 | The MAIN STRATEGY "top-mover flag continuation" (operator 15 Oct): no indicators, only today's high/low Fib (so far) (SPEC:40) | A1 running day ladder (6.1); the core holds no indicator (7.1); indicators enter only as wave-2 context dials (12.6) |
| O3 | top-N gainers (long) and losers (short) (SPEC:41) | section 5 |
| O4 | a tight consolidation box holding above (below) the 0.382 retracement (SPEC:42) | 7.1 steps 3-4 |
| O5 | entry on the close beyond the box (SPEC:43) | 7.1 step 5 |
| O6 | a stop just beyond the box ("tiny SL / drawdown is the goal") (SPEC:44) | 7.1 step 6 |
| O7 | targets: the day-high/low break, then 1.272/1.618 (SPEC:45) | 7.1 step 7; exit lanes XA-XD (9.1) |
| O8 | EVERY condition tracked for EVERY F&O stock on every candle; top movers are just a rank filter (SPEC:46) | every stock-day of the 218 is folded (17.4 phase A); the rank is a clause, never a pre-filter of the fold |
| O9 | intraday only, every day, the CASH shares of the F&O stocks (SPEC:47; MEM/fno-winners-research.md:13; U/every-strategy-fno-stocks-cash-intraday.md:8) | sections 3, 10 |
| O10 | two scores: net of every share charge; and "signal only, before charges", still worst-price fills (SPEC:48-50) | 11.3 |
| O11 | ONE TRADE PER STOCK, long and short alike (SPEC:51) | 10.4 |
| O12 | let it run: trailing stop, trailing take-profit, give-back, time stop, session exit (SPEC:52) | exit lanes XE-XH (9.1) |
| O13 | STOP-LOSS ALWAYS; no-SL only as a labelled comparison; every exit decided on the share's price; exits from the grid and from the strategy's own conditions (SPEC:53-56) | 9.1, 9.2 |
| O14 | REST SHAPE = ANYTHING, described by measurements; "retracement never inside the high's own candle (rest starts in later candles)"; ALL levels; mover rank top 1/3/5/10/25/50 or single stocks; NOTHING is static; the scorecard (SPEC:57-63; MEM/fno-winners-research.md:32) | the box is measured, never named (7.1); the push-extreme candle is never in the box (7.1 step 2); rank seeds (5.3); section 12; the report (16) |
| O15 | the 25 Oct small check passed nothing; edge about 0 gross, costs about 1R a trade (SPEC:64) | stated on every report as the prior (16.1) |

Further binding words: precise entry with the smallest stop, the level a measured feature never a precondition, partial patterns count (SPEC:68-74; MEM/precise-entry-patterns-anywhere.md:11, :15); NO FIXED VALUES, EVER, every former decision tested both ways (SPEC:78-80; MEM/let-data-decide.md:15, :17); price action first, exits their own dimension (SPEC:80; MEM/let-data-decide.md:19); the only fixed numbers are the requirement thresholds and exchange or broker facts (SPEC:81; MEM/nothing-fixed-everything-generated.md:18, :20); one common primitive path, no named-pattern switches (PARTS:1287-1294).

### 1.2 The second-touch entry (operator 2 Nov 2026 ~19:2x IST)

- The operator, from a Mazagon Dock 1-minute chart of 5 Oct 2026: "when the retracement happens second time, if we take an entry we need to find how much profit or winning strategy it can make"; "at first it touches 0.786 and then in the next few candles it went to 0.618 or even till 0.236 ... then again when it retraces and touches the same 0.786 we need to check the entry and winning criteria" (U/fib-second-touch-entry.md:8).
- It is one new entry condition inside the same strategy: same ranking, ladders, fills, charges and gate; long and short separately; every touch number scored on its own (U/fib-second-touch-entry.md:10, :12; BRIEF:110-125).
- The rule as binding text: BRIEF sections 4 (definitions, :151-265), 5 (the entry, :269-409), 6 (settings, :413-447), 8 (each touch number scored on its own, :474-486) and 9 (counting trials, :490-519).

### 1.3 The requirements every result meets (REQ, items 1-16, 7a, 16a)

REQ:3 says a missing item is BLOCKING. Section 21 traces each item to the section that meets it. The bar (REQ:4; U/every-month-positive-main-bar.md:29), the ranking (REQ:5), the walk-forward and luck test (REQ:6), F&O-only live fills (REQ:7, :15-17; U/backtest-as-dhan-live-trading.md:8), what is shown (REQ:9; U/results-show-net-over-100pct.md:8), nothing fixed (REQ:14), the objective and the operator's picture (REQ:19-20), every dimension (REQ:21).

---

## 2. What the preview is, and what it is not

- **It is** a backtest of the two seed recipes (E1, the flag; E2, the second touch) and of a bounded, fully counted search around them, on the store as it stands, under the 6 Nov requirements, labelled PREVIEW.
- **It is not** the production fib backtest of Thu 12 Nov (U/fib-strategy-deadline-20261112.md): that runs FNO-STRAT, FNO-RUN, FNO-WFT, FNO-RECORDS and FNO-API from merged code, with SPEC 13.4's scheme family. The preview never writes to runtime/, the shadow store, any branch or any part's state; it never delays the production run (17.6 rule 6).
- **It is not** round 3 (R3). Round 3 searches every idea family across four tools; the preview tests one strategy, the operator's, with the families of REQ 7a only as context dials paired with its core (12.6). Its luck count is its own family; a later merge with round 3's union is out of scope (19, item OP-P10).
- **It is not a fix of the data.** It runs on the store as it is; every known data defect is refused by name or labelled (3.4, 19). The data proof is AUDIT-3's; the production backtest still waits for it (U/heart-piece-first-never-move-ahead.md:11, :15). A preview result never reaches real money (REQ:12).
- **Not to be confused** with round 2's FIB2T and TOPMOV cores (F2/common.rs:456; F2/phase_a.rs:714-760, :1170-1215): FIB2T is a second touch with a reversal close on a 15 or 30-minute opening impulse, and TOPMOV is a rank snapshot at fixed instants. Neither is SPEC.md section 1's flag on the day's running ladder, and neither uses FNO-LADDER or FNO-RANK.

---

## 3. Universe, day mask and data

### 3.1 Universe (SPEC 3.1, :208-231)
- The 218 names of `INPUTS/fno218.txt` (digest in the run identity), traded only on the days each was a point-in-time F&O member (`pull::fno_membership::Membership::is_member`, as F2/market.rs:459-487 calls it). Leavers are never added, to the traded set or the rank pool (SPEC:215).
- ICICIPRULI is read as ICICILIFE before 2026-10-23 through `INPUTS/fno-join-v1.tsv` (a member under either spelling is a member); ICICILIFE's Oct 2026 is skipped (SPEC:218-231).
- The 12 names with no Zerodha data cannot signal; their stock-days fail M5 and are counted (SPEC:262).

### 3.2 The day mask, M1-M11 (SPEC 3.2 table, :237-249)
The preview's mask is round 2's (F2/market.rs:459-487: span, membership, unknown membership refused, the 2021-02-24 handling replaced below) plus the rows round 2 lacks, implemented here because FNO-DAYMASK did not pass (r3 FAIL, R3:691; DV-05):
- M7 from the normalised skip table `INPUTS/fno-skip-v2.tsv` (4 Nov, the newest; read as data, digest in the run identity), one rule a line, applied by its action (skip_day, skip_window, end_at, refuse_before, drop_auction_print, no_1day_signal, outage_window; PARTS:474-480);
- M8 trade-for-trade: no dated BE/BZ table exists in STORE or INPUTS, so every result says "T2T unchecked" (PARTS:499; DV-05);
- M9 Muhurat and special sessions masked (SPEC:247; OP-29);
- M10 no traded second on the stock-day;
- M11 no valid previous close (no trade on the previous session, or a demerger ex-day).
Every refusal is counted by reason, per stock and year, and SPEC 3.3's four counts are printed (SPEC:266-272), including the point-in-time members outside the 218 per day.

### 3.3 Data read, all read-only
- Zerodha 1-minute cash candles (STORE/bars/zerodha/NSE/CASH/<stock>/1min, from 2018-01 for warm-up), folded in-process into the 2-60 minute rungs by the store's own fold code (`pull::fold`, SPEC:281); a self-check compares the in-process fold with the store's rung files on the test stocks and stops by name on any difference.
- GDFL 1-second cash bars (STORE/bars/gdfl/NSE/CASH/<stock>/1s, 2018-09..2026-10): trades only. No quote-range files (`.qrg`) exist in the store (`ls`, 21:0x IST), so every trade carries "quote range not loaded" (PARTS:1304; N4).
- Zerodha INDEX daily and minute bars for NIFTY (the regimes of Variant K, 15.2) and for the context dials of 12.6 (NIFTY, BANKNIFTY, FINNIFTY from 2021-01-11, MIDCPNIFTY from 2022-01-24, NIFTY NEXT 50 from 2024-04-24, INDIA VIX; R3:219-221).
- NSE security files `STORE/session-masters/NSE_CM_security_*.csv.gz` (63 sessions 2026-08-03..2026-11-02: PricRg, BidIntrvl, closing-auction eligibility; DX:81-83).
- Dated expiries from the GDFL FNO folder names (DX:91-93), for the days-to-expiry dial.
- Span: 2018-09-03 to 2026-10-30, 98 months; warm-up for causal running features from 2018-01 (SPEC:798). The end is the round-2 tools' end, not a sealed day proven by AUDIT-3 (DV-28).

### 3.4 Known data defects inside a tradable day (each refused by name or labelled, never a silent drop)
- 2019-05-30: GDFL starts at 11:57:47; an order whose start second would be earlier is refused (`no_fill_data_window`), never delayed (SPEC:252); its first row is handled below.
- 2021-02-24: no decision after the last Zerodha candle (11:39); an open position closes at the worst GDFL price of 15:45 to the session end (SPEC:253; OP-30), through FNO-SEC-EXEC's session clock.
- Kite-hole minutes (a minute of the session that Zerodha does not hold, whatever GDFL holds): a candle holding one is incomplete, no decision on it (`incomplete_candle`), and a ladder whose defining candles hold one is tainted for the session (`tainted_ladder`) (SPEC:254-256; FNO-LADDER's own taint, its mod.rs:41-48 at `9caca9523`). The minutes neither feed holds (2022-03-07 and the other market-wide Kite-hole days) are holes by this definition, not no-trade minutes (BRIEF:760-766; DV-29).
- Vendor-padded Zerodha minutes (volume 0, OHLC the close before; `pull::padded`, at `a6ba0d5d4`): the brief leaves their treatment to one ruling (BRIEF:214, :751-754). The preview pins the reading that admits less, split by what GDFL shows: a padded minute in which GDFL printed a trade is treated as a hole minute everywhere (Zerodha's candle is unseen there; census-3 found 3,036 such minutes in one shard, BRIEF:214); a padded minute with no GDFL trade is a minute nobody traded and stays held, as `pull::padded` defines it. A padded minute never prices an R1 ratio, traded or not (BRIEF:331) (DV-29).
- 2019-05-30's first GDFL row (11:57:47, the session's open price with about the whole morning's volume, BRIEF:755-759) is never a fill and never an R1 price: an order that would start in that second, and an R1 ratio whose minute's only GDFL trade is that row, are refused (`bundled_first_row`, new; DV-29).
- The auction era (from 2026-08-03): the closing-auction print is dropped from fills; session end 15:14:59 for eligible stocks, 15:29:59 for the 44 non-eligible stock-days (SPEC:258-260, :606-609).
- Halts: a second with no traded volume fills nothing; 13 and 23 Mar 2020 are flagged on every trade that touches them (SPEC:264).

---

## 4. Clocks and timeframes

- **Timeframes:** 1, 2, 3, 5, 10, 15, 30 and 60 minutes (SPEC:280, S02). A recipe never mixes two timeframes for its core (SPEC:506); a higher timeframe may enter only as a context dial (12.6, multi-timeframe confirmation). The 1 day timeframe is not run: the operator's ladder is today's range so far (O2), which a daily candle does not have before its own close (DV-16).
- **Candles:** a candle labelled t on timeframe tf covers [t, t+tf), closes at min(t+tf, session end) and is known only then (FIB-I1, SPEC:143-146; :282). A rung candle holding a hole minute (3.4: a Kite hole, or a padded minute in which GDFL traded) is incomplete (SPEC:283).
- **Decision and start:** a decision at T starts its order in the first traded second at or after T + λ (SPEC:287). λ = 14 s on every stock-day (lane S, 11.1). A resting stop or target acts with no latency from the second its trigger prints; every move of a resting order is an order action that takes effect at the first traded second at or after its decision + λ (SPEC:293-294).
- **Integers:** prices and money in paise (i64, sums i128), ratios in ppm or as cross-multiplied rationals, percentages in basis points; no float on any decision path (FIB-I10, SPEC:195).

---

## 5. The top-mover clause (FNO-RANK, passed, reused unchanged)

### 5.1 The rule (SPEC 5, :305-349)
- m(s, T) = (P - C) / C: P the close of the stock's latest 1-minute Zerodha candle closed by T; C the previous session's close, under both bases of S10 ((a) the pulled 1day close; (b) the last 1-minute close), each a labelled reading (SPEC:311-320).
- The pool on day D: the 218 passing M1-M4 and M11 with at least one candle closed by T (SPEC:326). Competition ranks: equal moves share a rank, a tie at the edge of top N includes every tied stock (SPEC:329; OP-05). Long reads gainer ranks, short loser ranks (SPEC:331).
- One rank clock for every timeframe: the frame at T from 1-minute closes (SPEC:333). Stale prices carried and flagged (SPEC:332).

### 5.2 Code
`runner::fno_cross_section` copied byte-exact from FNO-RANK's passed tip `079b33146` (17.2): `Pool::begin_day`, `Pool::fold_minute`, `RankFrame::rank`, `Edge::count`, `TopCounter`, `DaySeries` (its mod.rs at that tip, 1,527 lines). The preview adds only an adapter that feeds each minute's closes from the store and the day mask's pool entries.

### 5.3 Seeds and dials
| Dial | Seed (operator) | Generated / varied |
|---|---|---|
| S11 rank edge N | 10 (SPEC:38, "top 10"); the operator's list 1 / 3 / 5 / 10 / 25 / 50 (SPEC:61) | coarse: the six seeds; fine: midpoints between the two best adjacent seeds by learning survival, then halving to resolution 1 (form D, SPEC:845); never above 50, the operator's largest seed (DV-31) |
| S12 rank state | in top N at T (SPEC:41) | also "entered top N within the last W candles" (SPEC:38, "entering"), W over 1, 2, 4, 8, 16 and the session (form D) |
| S10 basis | both, labelled (OP-04) | - |
| S13 minimum abs move | off | |m| against the stock's own usual move at that time of day (a causal running histogram), bands generated (12.2) |
| S15 stale limit | carry forward, flagged | a generated limit (form D, 1 minute .. 375 minutes) |

---

## 6. The Fibonacci ladder (FNO-LADDER, passed, reused unchanged)

### 6.1 The rule (SPEC 6, :353-442)
- **Anchor A1, the running day ladder** ("today's high/low Fib (so far)", O2): `AnchorId::DayRunning`. A ladder never judges its own candles: candle i is judged against the ladder of candles 0..i-1, then folds into it (FIB-I2, SPEC:148-166).
- **Anchor A2, frozen** (S21): for E1 the running ladder frozen at the box's first candle (SPEC:362, 455); for E2 the running ladder frozen at the close of a level's first touch, so the touch count keeps that touch's level (the preview's reading for a touch count, labelled; FNO-STRAT's own freeze point is unchecked). Frozen ladders come from `LadderSet::freeze` and are released when their count or box ends.
- **Direction:** the engine rule is the headline: Up when the high is the later extreme, Down when the low is, Undetermined when one candle holds both, and the ladder unused until a later one-sided extreme (FIB-I3, SPEC:168-172, :383-388). V-BOTH runs as a labelled reading; V-SEC is not run (DV-15).
- **Rungs:** retracements 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1 and extensions 1.272, 1.618, 2, 2.618, 3 in the move's direction (an up leg's extension x at lo + x·R, above the high) (SPEC:373-381; FNO-LADDER rung.rs:82-95 at `9caca9523`). The continuous position p (ppm of R) is a measured feature.
- **Relations** by `relate(candle, level, t)` with a tolerance passed in, never pinned (OP-36): wick touch, body through, body inside, close above, close below (FNO-LADDER relation.rs:151 at `9caca9523`).
- **From a Zerodha level to a GDFL price** (FIB-I8, SPEC:430-442): R1 ratio-mapped inside one session is the headline, L_g = L_z x G(T) / Z(T), Z the close of the last 1-minute Zerodha candle closed by T, G the last GDFL trade in that same minute, rounded against the position to the measured raw price step (SPEC:442). R2 (percent of the entry fill) runs as a labelled reading. R0 is refused by name (R4-26 not live).
- **R1's minute without a price of its own** (a hole, a padded minute, a withheld timeframe, or no GDFL trade): BRIEF section 5 refuses the second-touch trigger there (`no_trigger_ratio`, BRIEF:328-338) and leaves every other recipe's stop and target to one ruling (BRIEF:745-749). The preview takes the reading that admits less for every entry: an entry whose stop or target would be mapped from such a minute is refused by name (`no_level_ratio`, new; DV-29).

### 6.2 Code
`indicators::fno_ladder` copied byte-exact from `9caca9523` (17.2): `LadderSet::{new, open_session, observe_minute, mark_hole, step, freeze, release, usable_at}`, `AnchorSpec`, `Params::new(Tolerance, pack_limit)`, `relate`, `RUNGS`, `level`, `position_ppm` (its mod.rs 1,374 lines). The caller order of its module doc binds (seconds of a minute before the minute, every minute of a candle before the candle, its mod.rs:50-65).

---

## 7. The entry families

Both families are written for a long; a short mirrors every price test (a box below a low, holding below the level, a break below the box low, stops above, targets below).

### 7.1 E1: the flag (box) continuation, the operator's main strategy

1. **Rank and ladder.** At the decision T the stock is in top N by the clause of 5.3, and the ladder of 6.1 is usable (not Undetermined, not tainted).
2. **Push and box** (SPEC 7.1, :451-454; FNO-PAUSE): a push from the last confirmed swing low (or the open) to a new session high; then a box of K consecutive closed candles, the first no earlier than the candle after the push extreme ("retracement never inside the high's own candle", O14). Box high = the max of their highs, box low = the min of their lows, height h = box high - box low. A new high inside the box ends it and restarts the push; an incomplete candle voids the scope (FNO-PAUSE's module doc at `f8f6ace56`, D-8171-D-8177).
3. **Tight** (O4): the box height measured in ppm of R and as a fraction of the push, banded by the stock's own past boxes (12.2). The seed reading is the tight end of the stock's own distribution; "any" is a value.
4. **Holding rule** (S33): every box candle's low stays above level L of the ladder, judged per FIB-I2 under the ladder mode (running: each box candle against the ladder before it; frozen at the box start). Seed L = 0.382 (O4); swept over every rung and "none"; readings "all lows above L" (seed), "all closes above L", "no close below the first box candle's low", "none" (SPEC:455; S33).
5. **Break and entry** (S40; O5): the first candle after the box whose close is above the box high; T = that close; a market entry at the first traded second at or after T + λ, as `EntryOrder::Market { decided_at: T }` (FNO-SEC-EXEC walk.rs:17-35 at `d35888c0c`). This is the seed. The labelled reading "second": the first GDFL second whose printed high exceeds the R1-mapped box high, decided at that second + 1 s, then a market entry + λ (SPEC:457). The resting stop-entry (SPEC:458) is not run (DV-18).
6. **Stop** (S50; O6): below the box low by an offset; seed offset = 1 measured raw price step ("just beyond"); other placements: the next rung below + offset, L - offset, a percent of the entry fill (R2) (SPEC:459). The offset is generated (12.3). Always present.
7. **Targets** (S60; O7): seed T1 = the day-high break (the session high known at T plus 1 measured step), seed T2 = the 1.272 extension, seed T3 = the 1.618 extension, and the operator's "then" as a two-stage lane (9.1, XD). Every other S60 value is a generated lane (9.1).
8. **Touch clause on L** (S27, optional): the box's touch of its holding level may be required to be that level's n-th touch on this signature (BRIEF 4.8). Seed: no clause. Values = 1, = 2, >= 2 (counted as trials).
9. **Colour** (S37): the break candle green (seed: the side's colour, MEM/precise-entry-patterns-anywhere.md:15) or any; a flat candle has no colour (SPEC:491-492).
10. **Baseline P0:** rank + ladder + a level relation only, no box (SPEC:473). In the preview P0 is E2's "= 1" row (the first touch of a rung from the favourable side, entered at that candle's close), which the fold already emits, so the baseline costs no extra record; every E1 result prints it beside.

### 7.2 E2: the second touch (BRIEF sections 4-5, binding)

1. **The level** is one rung of one ladder, `(slot, lo, hi, leg, rung)`; the SAME level is the same rung on the same `(lo, hi, leg)`; when the signature changes the count restarts (BRIEF:157-162).
2. **Touch, clear, through** at tolerance t: touch `low - t <= L <= high + t`; clear `low - t > L`; through `high + t < L` (BRIEF:174-178). The touch number belongs to (candle, tolerance, bounce reading) (BRIEF:168).
3. **One touch** = consecutive touching candles; the first touch must come from the favourable side (BRIEF:182-184).
4. **The bounce** = the best favourable price of clear candles since the touch's last candle; the touch's last candle and the next touching candle never count; a through candle wipes it; it passes by S28: m rungs further, reached at `bounce >= rung` with no tolerance, or a generated size (BRIEF:188-192).
5. **The return** = the first candle of a later touch when the bounce before it passes; the touch number n advances by one; gap = candles strictly between (BRIEF:194-204). n is a fact of the market: it advances whether a trade was open, taken, refused or capped (BRIEF:204).
6. **When the count stops**: a signature change or a new session restarts it; a hole or an incomplete candle ends it for the signature (`touch_count_unknown`); S29b's labelled reading ends it on a close beyond the level by more than t (BRIEF:208-215). Under 3.4, a padded minute in which GDFL traded ends it too.
7. **The state table** Idle / At / Away / Ended, one candle at a time, arming at T_a when the bounce passes (BRIEF:219-228); the short side mirrored (BRIEF:232-244).
8. **Entry readings** (S40 for a return, BRIEF:273-277): **close** (seed: "that return is the entry", the return candle's close, market + λ); **second** (armed at T_a; the first traded second s from P + λ whose print reaches the trigger L_g + t_g, decided at s + 1 s, market + λ); **resting** refused by name, `touch_order_unsourced`. Every other clause of a second-reading recipe is read on the arming candle (BRIEF:281); S29d's two readings (held, or re-read at each later close) both run.
9. **The second reading's refusals at P** (BRIEF:321-340): `no_fill_data_window` when P + λ is before the stock-day's GDFL data start; `no_trigger_ratio` when R1's minute lacks a price. **Its ends** (BRIEF:342-356): a signature change; Ended (acting at the close of the first hole minute from P on); a through candle's close; the close of the touch candle it is armed for; S29; S42; the outage's start; S29d's re-read false; at a tie the end wins.
10. **Stop and target** (BRIEF:316-319): S50's pattern extreme = the lowest low (long) of this level's touches so far on this signature (`no_pattern_extreme` at n = 1 on the second reading); S60 gains the bounce high + offset (generated), walked only on returns of a recipe with a touch clause. The second reading fixes its stop and target at s + 1 s from candles closed by then.
11. **An entry with no fill** is never a trade row and never one of the 500 a year; it holds its slot, margin and count while it lives and is reported by end reason (BRIEF:374-388). Variant K's 51% check is printed under both readings (left out; counted as a non-win) and the worse decides it (15.1; DV-29).
12. **Seeds** (U/fib-second-touch-entry.md:8; BRIEF fixture :527-575): rung 0.786; touch number = 2 ("second time"); bounce m = 1 ("went to 0.618 or even till 0.236": the next rung, and m >= 1 as values); tolerance 0 (the chart's touch); the running ladder (the chart's A1 ladder 2,075.00-2,093.90 in force 09:19-10:26); entry reading close; S29c "in the next few candles" has no number and is generated (form D). Everything else of BRIEF section 6 is generated (12.4).
13. **Code**: the touch book `fno_strategy/touch.rs` copied from FNO-STRAT's tip read at build start (`2327b1eda` when this was written: built, not checked; DV-04), with the three items it takes from its parent module (`ABSENT`, `PPM`, `Refusal`) copied beside it as a shim (it also reads `indicators::fno_pause::Side`, vendored with FNO-PAUSE), and its brief-named tests copied with it; the second reading's trigger scan (BRIEF:362-372, FNO-RUN's job) is preview code over FNO-SEC-EXEC's `SecondSource` and `Second::traded()`.
14. **Acceptance fixture**: the BRIEF's MAZDOCK 5 Oct 2026 table (BRIEF:527-575) is replayed from the store: every touch, gap, bounce, arming time, trigger second (09:35:43), decision (09:35:44), start (09:35:46 at λ = 0; 09:35:54 at λ = 10 s) and refusal must match, or the build stops by name. The day stays in the data and is labelled as the source of the idea (BRIEF:516).

### 7.3 Both families
- A **signal** is one candle with one decision time on which every clause of the recipe is true (SPEC:505); for E2's second reading that candle is the arming candle (BRIEF:281).
- **Sides**: LONG (gainer ranks), SHORT (loser ranks), each searched with its own clauses and its own generated numbers (MEM/long-short-separate.md:11, SPEC:331); BOTH = a long recipe and its mirrored short in one account; PAIR = the window's best long and best short recipes traded together under shared caps (a procedure, SPEC:827). FADE sides are not run (DV-26).
- **Look-ahead**: every clause, level, stop and target of a signal is fixed from candles closed by its decision time, and every ladder handed to the seconds walk has K <= s (SPEC:146, :165; BRIEF:246-261).

---

## 8. Signals per timeframe and the record

For every stock-day that passes the mask, every timeframe and both sides, the fold of 17.4 phase A emits one RECORD per candle that is a signal of the loosest clause set walked:
- E1: a break of a K-box for at least one K of the walked K set (12.3), while the stock is inside the loosest rank edge walked (top 50 by either state);
- E2: a return (n >= 2) or a first touch (n = 1, the baseline row) at some (rung, ladder mode, t, bounce reading) walked, and for the second reading every arming event (BRIEF:466-468), while inside the loosest rank edge.

A record holds its raw feature columns (every dial of section 12 as a raw measured value, never a band), its decision, arming and placement times, its stop and target references in Zerodha prices, its R1 ratio terms, and its walks (11.4). Bands are computed per learning window at search time (12.1), so one record serves every window.

---

## 9. Exits, the session end and account controls

### 9.1 Exit lanes (exits are their own dimension, never mixed into entry combinations: SPEC:80, :573)
Every lane has a stop (O13). "Whichever comes first" among the lane's exits, the stop taken first when a stop and a target trigger in one second (SPEC:562).

| Lane | Exits | Seed | Generated |
|---|---|---|---|
| XA | stop + target T1 (the day-extreme break + offset) | E1 seed (O7) | the offset (12.3) |
| XB | stop + target T2 (1.272) | E1 seed (O7) | - |
| XC | stop + target T3 (1.618) | E1 seed (O7) | - |
| XD | stop + at the day-extreme break the stop moves to the entry price and the target becomes 1.272 (XD2) or 1.618 (XD3) | the operator's "then" (O7), labelled as a reading | - |
| XE | stop + a trailing stop armed at the day-extreme break: trail the best print by a distance, or step to below each new pause's low (stair-step, `W/state/fno-winners-20261013/pauses.json`) | "let it run" (O12) | arm and distance (12.3) |
| XF | stop + trailing take-profit armed at a rung (the `grid::Ttp` semantics, SPEC:569) | O12 | arm rung and trail (12.3) |
| XG | stop + time exit after H candles | "time stop" (O12) | H (form D, 1 candle .. 15:07:59) |
| XH | stop + condition exit on a candle close: close back below the box low (E1) or the level (E2), or the rank leaving top N' (SPEC:572) | O13 | N' (as S11) |
| XI | stop + a Supertrend trail with the box stop as its floor (REQ:20, "a tight Supertrend stop") | labelled: REQ 16a's picture, not the operator's fib definition | period from the stock's own p25 / p50 / p75 swing duration, multiplier from its p50 / p75 / p90 pullback depth in ATR units (R3:457, G-EXIT) |
| XJ | stop + target at the bounce high + offset (E2 returns only) | BRIEF:318 | the offset (12.4) |
| (beside) | the break-even move after a favourable x·R (S65) on XA-XC | - | x, including never (form G) |
| (comparison) | no stop | never in the search; printed for shown chains only (SPEC:54) | - |

The lanes run through FNO-SEC-EXEC's `WalkSpec { stop, target, time_exit }` and its `Amend::{MoveStop, MoveTarget, TimeExit, ConditionExit, CancelEntry}` (walk.rs:44-105 at `d35888c0c`): each move is an order action at its decision + λ; a trailing order modified more than 25 times is cancelled and replaced (SPEC:596). FNO-EXITS has no branch, so the lane logic is preview code (DV-20).

### 9.2 The session end (facts, class X; SPEC 9.8, :598-613)
- A strategy exit order whose start would be at or after 15:08:00 is treated as rejected and the position is left to the broker (OP-34); the entry's unfilled remainder is cancelled at the exit decision and at 15:08:00, whichever is first (R3:674).
- From 15:10:00 the broker squares off every open position, Rs 20 + GST per square-off order, counting the larger number of orders under every split rule a source names (SPEC:600-603).
- The remainder rule: whatever is open at the last continuous-trading second closes at the worst printed price over all traded seconds from 15:10:00 to that second; with no traded second there, at the worst since the entry, flagged `unclosable_in_session` (SPEC:604-605).
- The last entry S42 is at most 15:07:59 and is itself a generated dial (SPEC:889).
- Dhan's MTM rule: 80% of the position's margin, the worse of the two readings named (SPEC:611-613; OP-33).

### 9.3 Account controls (they only block new entries; they never move a stop or an exit; R3:286-302)
- Seeds, labelled (the operator's risk addendum, as R3:289-293 lists them): day loss limit 0.5 / 1.0 / 1.5% of the month-start capital; month loss limit 2 / 4 / 6%; month target lock at the bar (Rs 2,50,000 and 12.5%), 15%, 20%, never below the bar; caps of one trade per stock-day, k trades a day, and 10 / 20 / 40 a month admitted in ranked order.
- Generated levels from the candidate's own learning-month trace: the day loss at its own p10 / p25 / p50 losing day, the month loss at its own p10 / p25 month running minimum, a lock at its own p50 / p75 month above the bar, caps at its own p25 / p50 / p75 trades a day and a month (R3:294-297).
- Where they run: the seed grid on every walk-forward candidate of each window, then a coordinate ascent over the generated levels; every evaluation goes to the heaps and to the luck stream (R3:298-302).

---

## 10. Money and sizing

### 10.1 The numbers (class B, the operator's; REQ:7; SPEC 11.1, :643-655)
Capital Rs 20,00,000 of margin; at most Rs 1,00,00,000 of open position value at any moment; at most 20 positions open (SPEC:651); at most 100 entries a day (SPEC:652; each entry order sent counts, OP-18); one trade per stock at a time; everything closed inside its session.

### 10.2 Equal amount per entry, N generated (REQ:18; R3:262-275; U/sizing-when-many-signals.md:12, :14)
- **N**, the most positions open at once, is a walk-forward choice: seeds 1, 3, 5, 10, 20, refined between the two best adjacent seeds (2, 4, 7, 15; then 6, 8, 12, 17) by walks at the new N (G-POS, R3:459).
- **The amount** of an entry is min(Rs 1 crore, closed equity x 5) / N, quantised DOWN to the largest walked step whose margin and exposure checks pass; below the half step the entry is refused and counted (`size_refused`). Two steps are walked, 8/8 and 4/8 of Rs 1 crore / N, labelled "approximate step" (R3:268-270, round 3's own interim rule; DV-08).
- **The quantity** is the whole shares of the amount at the sizing price, never above it: the last traded GDFL price strictly before the order's start (FNO-SEC-EXEC D-8122, walk.rs:37-42 at its tip; SPEC 11.2's "closest on either side" would breach the Rs 1 crore cap at N = 1 and is a deviation, DV-08). q = 0 is refused (`price_above_position`).
- **Leverage** flat 5x (margin = value / 5): NSE's C_VAR1 files are not in the store, so every result says "dated leverage UNAVAILABLE (needs C_VAR1)" (DV-09); an L-SENS 3x lane is printed beside the shown chains.

### 10.3 Margin and exposure checks (SPEC 11.3, :679-687)
At an entry order's start second: margin in use (open plus pending positions) plus this order's <= the closed equity carried from Rs 20 lakh; exposure in use plus this order's <= Rs 1 crore. Both are valued at each position's deployed entry amount, not at its last traded price (DV-13). Failures are refused (`margin_cap`, `exposure_cap`) and counted.

### 10.4 Slots, the lock and admission (SPEC 9.6, 11.4, 11.5)
- A slot is taken when an entry order is sent and released at the exit's last fill or at a zero-fill cancel; a pending entry holds its slot and its stock's lock until its cancel second (R3:673).
- One position per stock anywhere in the account, long and short alike, from the entry's start to the exit's last fill; a signal while locked is `while_open` (SPEC:585-589).
- Admission when a cap binds: earlier start second first; inside one second by the recipe's tie-break order S81, chosen on learning months: (a) the larger |m|; (b) the smaller stop distance in R; (c) the earlier first entry into top N today; (d) symbol order; a final tie falls to (d) (SPEC:695-704). (e) 19 seeded random orders run on every shown chain and the spread of their net is printed as a luck check (SPEC:706).
- Order rate: 10 orders a second and 250 a minute, exits before entries inside a second (SPEC:591-596), queued and counted.

---

## 11. Fills and charges

### 11.1 Lane S, the headline (R3:647-651; REQ:15-16)
| Rule | Lane S | Source |
|---|---|---|
| latency | 14 s on every stock-day, labelled "UNCALIBRATED, strict" (the largest measured GDFL backward clock step) | OP-07; R3:647 |
| volume | each entry and exit walks second by second, at most 10% of each second's traded volume; a zero-volume second fills nothing | SPEC 9.3, :545-560; R3:649 |
| price | every slice at that second's worst traded price: a buy at its high, a sell at its low; never a level, an open or a mid | FIB-I4, SPEC:174-177 |
| protection | every market order carries a limit at LTP x 1.03 (Rs 1.5 at or below Rs 50); a second whose worst print lies beyond it fills nothing for that order | SPEC 9.2, :540-541 |
| bands | the lower net of R-BAND-1 and R-BAND-2 per trade, flex 5% then 3% then 2% observed; NSE's PricRg on the 63 dated sessions | PARTS:709-720; R3:650; DV-12 |
| ticks | NSE BidIntrvl where the security file exists, the 15 Apr 2025 table from then, before that the stock's own measured step from its previous month's GDFL prints | R3:669; SPEC 10.1 |
| one budget a second | the entry and the exit share one per-second volume budget | OP-11 |
| events while filling | stop, target, exit, cap or session end while the entry is part filled: the remainder cancelled, the exit on the filled part | SPEC 9.5, :575-583 |
| session end, MTM, square-off | 9.2 | SPEC 9.8 |
| special days | FNO-SEC-EXEC's `SessionClock` (data start, outage, auction print) | 3.4 |

The walk is `runner::fno_exec` copied byte-exact from `d35888c0c` (17.2): `PositionWalk`, `WalkSpec`, `EntryOrder`, `Lane { share_ppm, latency_s }`, `SessionClock`, `Bands`, `reference_ltp`, `walk_stock` (its mod.rs 689 lines, walk.rs 1,522). Its check r2 failed (5 Nov, N1 and N2) and repair 2 is not re-checked, so every result says "FNO-SEC-EXEC at d35888c0c: check r2 FAILED, repair 2 not re-checked" (DV-02). Round 3's rule for an unpassed execution part applies on the shown chains (R3:694-696): every shown trade is re-walked by round 2's audited F&O walk (F2/exec.rs:617-812, `fill_second` at :617, `path_at` at :652) at lane S; each trade where FNO-SEC-EXEC's net is kinder is counted, printed with its rupee gap, and booked at the stricter of the two in the shown figures.

### 11.2 Labelled comparison lanes (never chosen on results; shown chains only; R3:652-655)
25% of volume; no cap; one-second whole-quantity fills (REQ:15's approximation label); S70's halving sweep of the volume share; L-SENS 3x; the stricter-only cross-check of 11.1. Each prints its rupee difference against lane S. A pass that appears only in a comparison lane is printed "passes only in lane X: NO" (R3:572).

### 11.3 Charges (FNO-COSTS, passed, reused unchanged; COSTS.md)
- `costs::equity_clock::price_trade` at the scratch base `a6ba0d5d4` (FNO-COSTS passed r2, `check-FNO-COSTS-r2.md`:5): every trade priced on both clocks, its own day's official rates (dated) and Dhan's rates of today, each under the official method and every worst reading; the headline per trade is max(dated worst, today worst), both shown (COSTS:123-140; SPEC 12, :715-737).
- Worst readings in force until a Dhan contract note settles them: brokerage per fill second (up to Rs 20 + GST each), STT on the larger base, rounding per order, DP on an intraday sell day, the square-off fee on top of brokerage, the worst listed state's stamp before 2020-07-01, the NSE top slab before 2024-10-01 (COSTS:127-138; PARTS:1268).
- The Dhan data API, Rs 499 + GST = Rs 588.82 a calendar month with at least one trade, charged in the account replay (OP-28; DRAFTK:244).
- A trade is a win only if its net on the worse clock is above 0 (SPEC:737).
- Two scores on one trade list: NET (the headline) and "signal only, before charges" (the same trades and the same worst-price fills, no charges); the second re-scores the list and never re-runs it (SPEC 13.1, :743-747).

### 11.4 The walks a record carries
Per record: per exit lane walked x stop grid value x target grid value (12.3) x mapping reading (R1; R2 labelled) x amount lane (5 N seeds x 2 steps), the net (worse clock), gross, charges, exit second, exit reason, fill seconds, the partial-fill flag and the per-rule bits (latency-delayed, protection-limited, band-limited, band-rejected, volume-walked over 1 s, 15:08-rejected, broker square-off, MTM, tick-rounded stop, uncalibrated latency; R3:701-704). Walks for refined values (12.1) are delta walks over the records that need them, never a scaling.

---

## 12. Seeds and generated values (what the search varies, and how)

### 12.1 The generator (REQ:14; MEM/nothing-fixed-everything-generated.md:11-16; SPEC form G, :846; R3 4.2)
- **Learning months only.** In window w (14.1) every generated value is computed from that window's learning months (2018-09 to the end of the year before), and only from them (SPEC:812-815). Causal running features (a stock's usual range, volume or move at this time of day) are not fitted: they use all history before each decision.
- **Per stock first.** Each band edge is a quantile of the stock's own learning-month values of that reading, on that timeframe and side; a stock with fewer than 30 learning records (MIN_REF, class C, R3:428) takes the pooled unit's edge.
- **Coarse to fine.** Coarse: the quartiles p25 / p50 / p75 (bands below, between and above, and their unions), plus the domain's end points and the neutral value ("off", 0 or infinity). Fine: quantiles at 1/2^k from each tail inside the band where survivors crowd, k growing while neighbouring quantiles differ by more than the data's resolution (SPEC:846).
- **The guard.** A fine split is kept only when its statistic beats all 19 seeded shuffles of the band's learning nets (seed 20261107), and adjacent bands merge when their union does not beat the median of its shuffles (R3:436-441). The profile only proposes; the exact account decides (R3:444-445).
- **Path-dependent values** (stop offsets, target offsets, trailing distances, time holds) need walks. They are walked on a computation grid (class C, data-blind: a resolution, never a strategy value), and each generated value is snapped to the nearest walked grid point and printed as "generated: <quantile> of <population> = <value>, walked at <grid point>". Where survivors crowd, the grid is refined between its two points by a delta walk (coarse to fine). The grid is form D with no typed end: in measured raw steps 0, then 1, 2, 4, 8, ... with their midpoints 3, 6, 12, ..., up to the first point at or above the largest value any window generated; in fractions of R 0, then 1, 1/2, 1/4, ... down to the stock's one measured step; holds in candles 1, 2, 4, 8, ... up to 15:07:59.
- **Labels.** Every printed number names its class (B, X, E, S "generated: <rule>", C; R3:420-430) and every seed value says "seed (operator, <file:line>)". A printed number without its class stops the report (R3:430).

### 12.2 E1 and rank dials
| Dial | Seed (labelled; source) | Population for the generator (learning months only) | Coarse | Fine |
|---|---|---|---|---|
| S11 N | 10; list 1, 3, 5, 10, 25, 50 (SPEC:38, :61) | (enumerated) | the six seeds | midpoints, then halving (form D) |
| S12 state | in top N (SPEC:41) | (enumerated) | in top N; entered within W | W by doubling |
| S13 abs move vs own usual | off | the stock's |m| at that time-of-day slot | quartiles | 1/2^k |
| S21 ladder mode | running (SPEC:40) | (enumerated) | running; frozen at box start | - |
| S23 same-candle rule | engine | (enumerated) | engine; V-BOTH labelled | - |
| S31 K | 1 to "a few" candles (SPEC:38) | (form D, fixed per run because the box machine is per K) | 1, 2, 4, 8, 16 | 3, 6, 12, all walked from the start |
| S32 box height | "tight" = the stock's own low end (SPEC:42) | the stock's own K-boxes (broken or not), height in ppm of R and of the push | quartiles; any | 1/2^k |
| S33 holding | all lows above 0.382 (SPEC:42) | (enumerated) | 4 readings x 7 rungs + none | - |
| S34 push size, candles, new highs | any | the stock's own pushes: size in ppm of its usual daily range and of R | quartiles; any | 1/2^k |
| S35 pause volume / push volume (Zerodha volume) | any | the stock's own boxes | quartiles; any | 1/2^k |
| S37 colour | the side's colour (MEM/precise-entry-patterns-anywhere.md:15) | (enumerated) | side colour; any | - |
| S40 entry reading | close (SPEC:43) | (enumerated) | close; second | - |
| S26 earliest decision, S42 last entry | none given (open question in MEM/fno-winners-research.md:32) | the recipe's learning signals by minute of the session | halves, quarters, eighths of the session from 09:15 | one candle, where survivors crowd |
| S27 on L (E1) | no clause | (enumerated) | none; = 1; = 2; >= 2 | - |

### 12.3 Stops, targets and exits
| Dial | Seed | Population | Coarse | Fine |
|---|---|---|---|---|
| S50 placement | beyond the box low (E1), the pattern extreme (E2) (SPEC:44; BRIEF:317) | (enumerated) | pattern extreme; next rung below; L | R2 labelled |
| S50a stop offset | 1 measured step ("just beyond") | the learning window's adverse excursions beyond the pattern extreme of the recipe's own signals (the excursion ladder of SPEC:815), in measured steps and ppm of R | 0, 1 step, quartiles | 1/2^k (grid of 12.1) |
| S60 day-extreme break offset | 1 measured step | the learning window's favourable excursions beyond the day extreme | quartiles | 1/2^k |
| S60 rung targets | 1.272, 1.618 (SPEC:45) | (enumerated) | next rung; 1.272; 1.618; 2; 2.618; 3 | - |
| S60 R-multiple and excursion-quantile ladder | none | the learning window's favourable excursions in R | quartiles | 1/2^k |
| S61 trailing arm and distance | arm at the day-extreme break | the learning window's pullbacks after the arm | quartiles; stair-step | 1/2^k |
| S61a trailing take-profit | arm at 1.272 | as `grid::Ttp`, from the learning window's give-backs | quartiles | 1/2^k |
| S62 time exit | none | the learning window's holding times of winners | the hold grid's coarse points | its fine points |
| S63 condition exit | close back below the box low (E1) or the level (E2) | (enumerated) | 3 conditions; N' as S11 | - |
| S65 break-even move | never | the learning window's favourable excursions in R | never; quartiles | 1/2^k |

### 12.4 E2 dials (BRIEF section 6, :417-447; every size generated)
| Dial | Seed | Generated |
|---|---|---|
| rung | 0.786 (the chart) | every one of the 12 rungs (E) |
| S24 tolerance t | 0 (the chart) | fraction of R and ppm of price, form G from the learning window's distance distribution of candles to the rung; its upper end that window's own largest (BRIEF:254, :430); measured steps are not used for S24 (its split-factor ruling is open, BRIEF:768; DV-29) |
| S27 touch number | = 2 ("second time") | = n for n over 1 .. that window's largest seen; >= n for n over 2 .. (E) |
| S28 bounce | m = 1 ("another level") | m over 1 .. rungs on that side; the size reading `bounce - L` in R or ppm, form G over the away runs ending in a touch, at t = 0 (BRIEF:431) |
| S29, S29a gap at most / at least | off | form D over 1 .. candles in the session (BRIEF:432) |
| S29b close beyond the level | count goes on (the page's reading) | ends (labelled) |
| S29c candles to the bounce | "in the next few candles": no number | form D, infinity = off |
| S29d (second reading) | held at T_a | re-read (labelled) |
| S40 | close | second; resting refused by name |
| S60 bounce high offset | 0 | form G over the distance from each return's bounce high to the best favourable price to the session's end (BRIEF:318) |

### 12.5 Money and admission dials
N (10.2), S81 (10.4), the account controls (9.3), the admission order FIRST (signal time, then symbol) or RANKED (the stated score's causal percentile from the recipe's own earlier years, frozen every 1 January; R3:316-318).

### 12.6 Context dials for REQ 7a (wave 2, pair partners of the core; never in the seed recipes)
Each is a group of generated bands on the record's own columns, paired with every E1 and E2 core node of level 1-2 (13.1):
- time of day (as S26 / S42), day of week, day of month, sessions to the stock's monthly expiry and the expiry week (dated from the store's expiries);
- the stock's 1st, 2nd and 3rd+ trade today, and the account's 1st..5th and 6th+ trade, and after the stock's last win or loss (account-level rules in the exact account);
- trigger-candle anatomy and the two candles before it: body, upper and lower wick, close location, range against the stock's own ATR14 (generated bands);
- index context: NIFTY and BANKNIFTY day move and their own EMA-20 side at T, FINNIFTY, MIDCPNIFTY and NIFTY NEXT 50 only from their F&O starts, INDIA VIX level and change, each as generated bands (REQ:17; R3:242);
- universe: liquidity, volatility and price bands among the dated F&O members, NSE industry baskets labelled "survivorship: today's map" (DX:75-80);
- multi-timeframe confirmation: the last closed candle of each higher timeframe (its colour and its position p on its own A1 ladder, banded);
- indicators: round 2's 42 generated readings (F2/pct.rs:17-66, each ranked against the stock's own past) on the timeframes round 2 computes them (3, 5, 15, 30, 60 min); on 1, 2 and 10 min the indicator group is printed UNAVAILABLE (DV-27). The operator's core holds no indicator (O2); here an indicator is only a filter on that core, labelled so.
Days-to-expiry as a calendar fact, VIX and index values are read only as known at the decision second, never in the rank or the run identity (U/fib-step2-context-filters.md, the 31 Oct approval, governs the production engine; DV-27).

---

## 13. The search: levels, waves and counting

### 13.1 Levels (per unit = timeframe x side x family x exit lane)
- **Level 0, the operator's strategy as typed:** E1-seed (top 10, in top N, running A1, K = 1, 2 and 3, "tight" read two ways as the box height at or below the stock's own p25 and at or below its own p50 with "any" beside, all lows above 0.382, the side's colour, close beyond the box, stop 1 step below the box low, lanes XA / XB / XC / XD) and E2-seed (top 10, running A1, rung 0.786, t = 0, n = 2, m = 1, close reading, stop 1 step below the touches' low, lanes XA / XB / XC / XJ), each at every N seed, FIRST admission, controls off.
- **Level 1:** one dial at a time over its coarse values, every other dial at its seed.
- **Level 2:** every pair of dials of different groups over their coarse values, exhaustively.
- **Level 3+:** refinement by extinction: a further dial or a finer value while the learning key strictly improves and the work budget allows; no depth cap (CLAUDE.md section 6; R3:378-380). The depth reached is printed per unit.
- **Stage 2 on each window's candidates:** the N refinement, S81, FIRST / RANKED, the account controls (9.3), and in wave 2 the context dials (12.6) paired with every level 1-2 node.
- **Exactness:** every evaluation is an exact account over all 98 months (10, 11). Skips only by round 3's exact bounds, which prove a series can enter no heap and raise no luck draw, proven byte-equal with the bound on and off on real units before use (R3:522-532); a skipped series is counted as tried.

### 13.2 Waves (fixed now, data-blind; a deadline cuts work units, never by results; R3:384-393)
- **Wave 0, the operator's seeds:** level 0 only, every timeframe, side and N seed; walk-forward among the seeds only. Interim report "PENDING: interim after wave 0 (the operator's seeds only)". This answers the operator's question first.
- **Wave 1:** levels 1-2 on coarse values for E1 and E2, the exit lanes' coarse values, the N seeds; twin seed 1 (14.4); interim report 1.
- **Wave 2:** fine values and level 3+, the N refinement, stage 2 with the controls, the context dials of 12.6, the comparison lanes and the cross-check of 11.1 on the shown chains, Variant K (15); the verdict (14.5).
- **Twin seeds 2-3** only if the headline meets the bar in every unseen month (R3:544-545).

### 13.3 Counting every option tried (BRIEF 9, :504-519; R3:515-518)
- `tried_total` = evaluated + bound-skipped series, per wave, family, unit and stage, with its base-2 logarithm; waves 0-2 are one family (BRIEF:515); count at evaluation, not at survival; per distinct learning window; no discount for near-duplicates, only exact duplicates by the digest of the whole order list (each order's signal time, decision time or none, and end time and reason) at every latency walked (BRIEF:508-512); refinement points counted as tried (BRIEF:513).
- The idea came from inside the span: the flag from the operator's study of Oct 2026 winners (MEM/fno-winners-research.md:11), the second touch from MAZDOCK on 5 Oct 2026 (BRIEF:516). Every report prints the 2026 window apart, labelled "2026: the idea's own months", beside the 2020-2025 figures.

---

## 14. Walk-forward, selection, luck test, random-entry twin and verdict

### 14.1 Windows (REQ:6; R3:480-489)
- Seven windows Y2020..Y2026, anchored: window Y learns on 2018-09 to (Y-1)-12 (learning prefixes of 16, 28, 40, 52, 64, 76 and 88 months) and is judged on year Y's months. The 82 unseen months 2020-01..2026-10 chain on ONE account from Rs 20 lakh with capital and peak carried across years.
- Every choice of window Y (recipe, every generated number, exit lane, N, S81, admission order, controls) reads months before Y only. The months from Y on are read only by the luck stream and the hindsight counters.
- Search reach: a series whose numbers were generated on window Y's learning months is offered only to windows with a learning prefix at least as long (R3:493-495).
- Look-ahead tests on every stage: perturb every net in months >= LEARN[Y]; no choice of window Y may change (MIX-WF-1, R3:499-502); and each of BRIEF 4.12 item 8's five shuffle properties (BRIEF:255-260).

### 14.2 Selection on learning months (THE RANKING; REQ:4-5; R3 6.1-6.2)
- Passers first, in this order: (1) every learning month has trades and nets at least max(Rs 2,50,000, 12.5% of its start capital); (2) the lowest max drawdown (closed-trade fall from the running high, in tenths of a percent of Rs 20 lakh; no cap: U/every-month-positive-main-bar.md:27); (3) the most net; (4) the highest entry accuracy (win rate; zero trades rank below every rate); (5) fewer trades; (6) shorter average holding; (7) higher net per trade; (8) the stable id (R3:603-607).
- Misses (every candidate today): the pre-registered shortfall key, the least total rupee shortfall to the bar over the learning months, a no-trade month owing the full bar; then the highest worst month; then keys 2-8 (R3:614-618, decision D1's default). Round 2's count key's chain is printed beside.
- Scopes printed beside the headline, never writing the verdict: per family (E1, E2), per timeframe, per side, per N, FEWEST, TPM10 / TPM20 / TPM40 (at most 10 / 20 / 40 trades a month on learning months), LOWEST-DD, and per touch number (15.3) (R3:326-329, :625-632).
- The headline procedure, fixed now: the GLOBAL pick #1 of each window over every family, timeframe, side and N, chained over the 82 months.

### 14.3 The luck test (R3 5.3, :504-532)
- One mask set: psrb's 1,000 sign-flip masks over the 82 unseen months (`flips()` at P2/engine.rs:904-914, splitmix64 of seed 20261106), copied bit-identical, so the preview's luck figures compare with round 3's (R3:506-509).
- Every series evaluated anywhere (every level, wave, scope, stage-2 and control evaluation, comparison lanes of shown chains) is offered: its 98-month row, `pos` = an unseen month with trades and net >= Rs 2,50,000, `neg` = net <= -Rs 2,50,000, both against C0 = Rs 20 lakh; best_b = the largest count over all series under flip b (R3:510-511).
- Clears when (1 + #{b : best_b >= O}) x 20 <= 1,001, O = the headline chain's months at Rs 2,50,000 against Rs 20 lakh (R3:519). Printed as the run grows: L95, the median best-by-luck, O, tried_total. The same test at the +2.5% tier is printed beside.

### 14.4 The random-entry twin (R3 5.4, :534-547)
- Form: for seed S in {1, 2, 3}, inside every unit and every year, the assignment of records to their feature vectors is jointly permuted (every raw feature column moves together), drawn by splitmix of (S, unit, year). Set sizes stay exact; trade outcomes, signal times, stocks, sides, families, exit lanes and timeframes stay real; only the link between the dials and the outcomes breaks.
- The WHOLE pipeline re-runs on the permuted records: the generator, levels 0-3+, stage 2, controls, the walk-forward, with the real run's work budgets. Seed 1 always runs (in wave 1); seeds 2-3 only once the headline meets the bar.
- Printed per seed beside the headline: the twin's GLOBAL chain (months at the bar and at the tier, worst month, max drawdown, net, trades, holding) and its hindsight best beside the real hindsight best.

### 14.5 The verdict (one function, the only writer of line 2; R3 5.6, :555-584)
1. no headline chain formed: NO ("no headline");
2. any of the 82 unseen months with no trade or below max(Rs 2,50,000, 12.5% of its start capital): NO, with the closest misses;
3. the sign test does not clear: NO;
4. twin seed 1's GLOBAL chain ranks at or above the headline by THE RANKING on the 82 unseen months: NO;
5. fewer than three twin seeds have run: PENDING;
6. twin seed 2 or 3 ranks at or above the headline: NO;
7. otherwise YES.
Interim reports print "PENDING: interim after wave k" and never YES. A chain chosen among many on its unseen months is printed "chosen among N chains on their unseen months: NOT a pass". Hindsight rows never touch line 2.

---

## 15. The Fibonacci Winner Gate, Variant K, reported beside the bar

The operator chose Variant K on Thu 5 Nov ~19:24 IST, "Keep all, add 2.5%": every existing Winner Gate check stays and the monthly floor is added FIRST as one pair check, M0; no drawdown cap, "lowest wins" (U/every-month-positive-main-bar.md:25, :27; DRAFTK:64, :234-265). The draft is not applied to SPEC.md (DRAFTK:1), so the preview computes it itself; it never decides a pick or line 2 (DV-24).

### 15.1 The 41 clauses, printed for every shown recipe and every closest miss, each with its measured value against its required value
- **M0, the floor (pair):** every unseen month at least max(Rs 2,50,000, 12.5% of the month-start capital), the newest bar (U/every-month-positive-main-bar.md:29), on the carried capital reading (a), with DRAFTK BAR-1's +2.5% floor (:240-262) and its readings (b) window and (c) fixed Rs 20 lakh beside; month net N_m = the pair's net of trades closed in m less the data charge of a month with trades (DRAFTK:243-245). Exact i128: month m passes iff N_m >= Rs 2,50,000 and 10,000 x N_m >= 1,250 x C_m; the +2.5% reading iff 10,000 x N_m >= 250 x C_m; a month with no trade fails both.
- **Per side (x2), quality** (SPEC:778-783): win rate >= 51% (printed with the no-fill entries left out and counted as non-wins; the worse decides); average win >= 2x average loss; profit factor >= 2; largest win >= 2x largest loss; positive on the unseen months.
- **Per side (x2) and per pair, the five downside checks**, unseen against the same chain's learning months (MEM/backtest-acceptance-bar.md:30; `W/state/fib-run-20261016/downside-amendment.txt`:8-12): no unseen day worse than the worst learning day; no unseen week worse than the worst learning week; unseen max drawdown no deeper than learned; the longest run of losing trades, days and weeks no longer than learned; recovery time no longer than learned. Computed at the chain's own equal-amount sizing, not at Rs 1 lakh and Rs 5 lakh (DV-24).
- **Per pair, quantity:** >= 500 trades a year, partial years pro-rated by tradable days (MEM/backtest-acceptance-bar.md:23); every year positive; leave-one-out x5 (drop the best day, week, fortnight, month, quarter of the unseen record and judge again, M0 included; DRAFTK:264); net positive on the unseen months of each NIFTY regime (falling, flat, rising: terciles of the learning months' NIFTY monthly returns; MEM/backtest-acceptance-bar.md:34).
- **Minimum returns x5** (average unseen net, simple, on Rs 20 lakh, partial periods pro-rated): 0.3% a tradable day, 1.5% a week, 6% a month, 18% a quarter, 75% a year (`W/state/fib-run-20261016/min-returns-amendment.txt`:4-10; OP-22).
- **The two walk-forward clauses:** at least 70% of the unseen months positive; no unseen month worse than the learned loss floor (MEM/backtest-acceptance-bar.md:15).
- The count printed: checks passed of 39 counted (M0 + the 38 of PARTS:1357), and both walk-forward clauses.

### 15.2 What "pair" means here
For a BOTH or PAIR chain the per-side checks read its long and short trades separately and the pair checks read the whole. A LONG-only or SHORT-only chain fails the other side's per-side checks by name ("no trades on this side"), shown as such, never hidden. One scheme only (anchored yearly): every Variant K line says "one scheme, not SPEC 13.4's family" (DV-06).

### 15.3 Each touch number on its own (BRIEF section 8, :474-486)
Long and short apart, one row per number clause (= 1, = 2, = 3, >= 2, >= 3): the best recipe by learning net inside that clause, picked in each window from learning months only, traded forward, judged whole; each row with trades a year, win rate, average win over average loss, profit factor, largest win over largest loss, the five downside checks, NET beside "signal only, before charges", checks passed of 39 and its trade count. = 1 is the baseline. Comparing rows describes the data; only the bar and the verdict judge.

---

## 16. The report (PV/run/out-<seq>/REPORT.md and the database)

1. Lines 1-3 as at the top of this file; then the APPROXIMATE list: every DV of section 18 that touches a figure, each part's tip and check state, survivorship first, "quote range not loaded", "dated leverage UNAVAILABLE", "T2T unchecked", "ban days UNAVAILABLE (traded)", "14 s UNCALIBRATED", "band computed; part not checked", the prior of SPEC:64 (the 25 Oct check passed nothing), and the data limits of SPEC 19 (:1132-1147).
2. **The plain answer:** the bar (Rs 2,50,000) and the +2.5% tier, both for the headline chain; the verdict's parts (bar, luck, each twin, the lane); the yardstick line "needed X bp a trade at your n and N (D measured); this chain makes Y" (R3:149-161); the operator's seeds' own line (wave 0's best seed chain, whatever its net, because it answers the ask).
3. **Shown in full only when the chain nets >= +100% on its unseen months**; else one line naming the best with its net (U/results-show-net-over-100pct.md:12; REQ:9).
4. **Each shown chain:** its FULL settings in plain words, year by year (family, timeframe, side, every dial with its seed or generated value and rule, exit lane, N, sizing, admission, controls); months at the bar and at the tier; worst month; max drawdown (tenths of Rs 20 lakh, Rs, bp of peak, dates); net (Rs and %); gross; charges; charges per trade and their share of gross; trades; trades a month; wins; losses; win rate; average win and loss; largest win and loss; profit factor; net per trade; holding (average, p90, longest); deployed amount against target; partly filled trades, fill seconds, the difference against one-second fills; the per-rule counts (R3:701-704); refusals by reason, the four second-touch reasons included (SPEC:770); the INDIA VIX stamp on every trade, reference only (SPEC:765); a per-year table and a per-month table inside each year (month, net Rs and %, gross, charges, trades, wins, win rate, worst trade, month-end equity, drawdown from the running peak, at the bar yes/no, at the tier yes/no), also written as `months-<chain>.tsv` (R3:799-812).
5. **Variant K** (15.1) for every shown chain and every closest miss; the per-touch-number rows (15.3); E1 beside its P0 baseline.
6. **Item 16 (REQ:19):** the positive side (net, win rate, average win, profit factor, net per trade) and the negative side (max drawdown, losing months, worst month, average loss, largest loss, losing trades) on every row; the Pareto set on (unseen net, max drawdown, win rate, trades a month) over the chains formed, display only, labelled "chosen among N chains on their unseen months: not a pass"; the fewest-trades chain among the passers, else among the closest misses, named (R3:330-332, :813-815).
7. **When nothing passes** (U/closest-miss-never-just-failed.md:12): the 10 closest chains by the shortfall key of 14.2 on the unseen months, each failing month with its rupee gap to the bar, the luck gap (O against L95), the twin gap, the checks passed of 39; and the nearest bar that IS met, F, the highest monthly floor some chain holds in every unseen month, with F_min over chains at or above the 31 Oct minimum trade count beside it (DRAFTK:287-290).
8. **Luck and nulls:** tried_total per wave, family and stage with log2; L95; the median best-by-luck; O; each twin seed's chain or "seeds 2-3: pending"; the S81 seeded-order spread.
9. **The operator's chart days, replayed (description only):** MAZDOCK 2026-10-05 (E2, the BRIEF fixture) and TECHM 2026-10-15 (E1, MEM/fno-winners-research.md:32): what each seed recipe did on that day under lane S, labelled "one day proves nothing" (U/fib-second-touch-entry.md:10).
10. **The database** (requirement 8, REQ:11): every table and figure committed as units in PV/run/ with the audit log; `query` reads any row; a static dashboard page in each out-<seq>/; `verify` re-hashes everything (17.5).

---

## 17. Build

### 17.1 The tree (BP rule 1, :33-35, adapted to this task's law)
- `git -C REPO worktree add --detach W/scratch/fib-preview-1 a6ba0d5d4`. This writes git's own worktree metadata under REPO's `.git/worktrees/`, as every scratch tree has; nothing else outside TREE and PV is written.
- **No commit, no branch, no merge, no push, no ref moved.** The vendored modules and the preview crate stay uncommitted in TREE. The code identity of each build is the sha256 of every source file of the crates it compiles, written to `PV/identity/<seq>.tsv`; `git -C REPO for-each-ref | shasum -a 256` is recorded before and after every build step and must be equal.
- Cargo only through `W/bin/cargo` with `PATH=W/bin:$PATH`; target folders `W/tgt/fib-preview-build`, `-test`, `-rel`, `-cov`, `-bench`; one build or test of this tree at a time; before each step every tracked file of TREE is touched (OPS-10: a shared lane once read another tree's artifacts as fresh, `part-FNO-STRAT.progress.md`, 18:41 entry).

### 17.2 Reuse, never a rewrite (BP:71-94)
| Piece | Source | Into TREE | Label |
|---|---|---|---|
| Fibonacci ladder | FNO-LADDER `9caca9523`, `crates/indicators/src/fno_ladder/` | same path, `pub mod fno_ladder;` under its slot | passed (r4) |
| top-mover rank | FNO-RANK `079b33146`, `crates/runner/src/fno_cross_section/` | same path, under its slot | passed (r2) |
| charges | FNO-COSTS `c8f7608b5cbc`, already in `a6ba0d5d4` (`costs::equity_clock`, `equity_dated`, `equity_readings`) | unchanged | passed (r2) |
| fill walk | FNO-SEC-EXEC `d35888c0c`, `crates/runner/src/fno_exec/` | same path, under its slot | check r2 FAILED, repair 2 not re-checked |
| push, box, break | FNO-PAUSE, newest tip at build start (`f8f6ace56` now), `crates/indicators/src/fno_pause/` | same path, under its slot | r1 FAILED; repair 1 under check r2 (the label changes to "passed" only if that check passes before the build) |
| price bands | FNO-BAND `2d785d8d3`, `crates/runner/src/fno_band.rs` | same path, under its slot | no check on record |
| touch book | FNO-STRAT tip at build start (`2327b1eda` now), `crates/runner/src/fno_strategy/touch.rs` + its tests | `crates/fib_preview/src/touch.rs` with a three-item shim | built, not checked |
| store door, calendar, day mask, index series | F2/store_io.rs (151 lines), F2/market.rs (calendar, `mask`, `Index`), F2/market2.rs (F&O starts, baskets) | `crates/fib_preview/src/` with their sha256 named | round 2, audited |
| exact accounts, units, walk-forward, union picks | F2/acct2.rs, F2/search.rs (`Unit2`, `kept_of`), F2/wf2.rs (`union_picks` and its unit test) | adapted to fib units | round 2, audited, with R3's account rules (10) |
| luck masks | P2/engine.rs:904-914 `flips()` | bit-identical | round 2 |
| readings for the indicator dial | F2/pct.rs, F2/feat.rs | wave 2 only | round 2 |
| reference walk for the cross-check | F2/exec.rs | report stage only | round 2, audited |
| rigor layer | RF db.rs, runner.rs, units.rs, cmd.rs, cli.rs, selfcheck.rs, seams.rs, monitor.rs, benchkit.rs | `crates/fib_preview/src/rigor/` | the F&O rigor tree |

Each vendored module is copied with `git -C REPO archive <tip> <path> | tar -x -C TREE` (read-only on REPO); its own test suite runs green in TREE before any preview code depends on it, which proves the copy is the code that was checked. A copy that conflicts outside its slot line stops the build by name.

### 17.3 The preview crate `crates/fib_preview`
Binary `fib_preview` with commands `phase-r`, `phase-a`, `search`, `report`, `run` (resume), `status`, `verify`, `query`, `monitor`. Modules: `day` (mask, session clock, holes, padded, data start, measured step), `rank` (FNO-RANK adapter), `fold` (LadderSet, PauseSet, touch book per stock-day and timeframe), `e1`, `e2` (signals and records), `scan` (the second reading's trigger scan), `exits` (lanes XA-XJ on `Amend`), `walk` (WalkSpec builder, `price_trade`), `spool`, `gen` (generator and guard), `search` (levels, exact accounts), `wf` (windows, picks, chain), `luck`, `twin`, `gatek` (Variant K), `report`, `rigor`.

### 17.4 Phases and data flow
- **Phase R (market, once, 4 threads, <= 1.5 GB):** every session's minutes of the 218 through FNO-RANK (both bases), the per-stock-day rank series (375 u16 per side and basis), and the per-stock-day facts: hole and padded minutes, GDFL data start, session clock, measured raw step, both band bases, the NIFTY and context series. Written once, read-only memory-mapped after.
- **Phase A (per stock, up to 8 threads, one stock a thread):** for each tradable stock-day and timeframe, in time order: open the session; report every minute (holes, padded) before its candle; read the day's GDFL seconds once; step the ladder set, the box machines and the touch books; emit records (8); run the walks of 11.4 through FNO-SEC-EXEC; price each with `price_trade`; append to the stock's spool file (sha256 per file). A stock-day with no record reads no GDFL second.
- **Phase B (search, up to 8 threads, one unit resident per thread):** per window the generator (12.1), then levels 0-3+ (13.1) as exact accounts over 98 months, every series offered to the window heaps and to the luck stream; stage 2; the twin on permuted records.
- **Phase V (report, 4 threads, <= 2 GB):** the chain, the verdict, Variant K, re-walks of the shown chains in the comparison lanes and the cross-check, the report and the dashboard.

### 17.5 Records, identity and the database (REQ:11; SPEC 15)
- The run identity: the data inputs' digests (fno218.txt, fno-join-v1.tsv, fno-skip-v2.tsv, the membership file digest, the cost table version `costs-table/2026-10-30/r1`, the store months read) and the code identity (17.1). Units carry their stage's code identity, so a fix recomputes only its stage and those after it (R3:750-752).
- RF's database form in PV/run/: an exclusive lock, runs/<run id>/manifest.tsv (sha256 and CRC per unit), a hash-chained audit.log, progress.log, staging with O_EXCL and hard links (never an overwrite or a deletion), one out-<seq>/ per finishing process with MANIFEST.tsv last (R3:745-749). Telemetry only at unit boundaries, never inside a per-second loop (SPEC:933).
- Lean mode: aggregates for every candidate; full trade records only for picked and shown chains (SPEC:934).

### 17.6 Machine rules (the task's law; BP rules 3-8)
1. At most 8 threads in total for this tree; every process under 4 GB, with an RSS guard at 3.5 GB in every binary, test binaries included (save state, halve the cache, else stop by name); end-to-end tests in release with `--test-threads 1`.
2. Never two builds, or two full runs, of this tree at once.
3. Heavy runs (phase A or phase B on more than the 5 test stocks) start only after the round-2 PSRB sweep has ended: no `psrb_r2` process in `ps` (it was running at 21:04 IST as `psrb_r2-r9 --phase b`; its end was planned for about 23:15 IST on 6 Nov).
4. nice 15, below every running search; drain files (DRAIN, DRAIN-WF, DRAIN-FIB) honoured at unit boundaries only, so a drain never cuts a running unit (U/drain-stops-running-stages).
5. Every foreground wait at most 270 s; long work in the background with logs in PV/logs/; the Mac on mains (`pmset -g batt`) and the disk floor (1.1 TB + 50 GiB) checked before every write-heavy step; kill-and-resume byte-equal at every unit boundary.
6. The production fib backtest has priority (U/fib-strategy-deadline-20261112.md): when it starts, the preview finishes its running unit and pauses.

### 17.7 Tests and benches (REQ:11; BP rule 6, :50-54)
- Every new function: at least 8 extreme cases, 2 combinations and 1 out-of-the-box case; line and branch coverage of `crates/fib_preview` measured by Route B and printed as achieved (100% is the bar; anything short is listed, never rounded up).
- The vendored parts' own suites green in TREE; the in-process fold equal to the store's rungs on the test stocks.
- Acceptance: the operator's flag on synthetic sessions gives exactly one signal at the break candle's close, and its short mirror (PARTS:886); the BRIEF's MAZDOCK 5 Oct 2026 fixture (7.2 item 14); the BRIEF's named refusal cases (2019-05-30 INFY, HDFCBANK 2024-04-23 and 2022-03-07, POWERGRID 2022-03-07, ALKEM 2023-07-06; BRIEF:555-572); every row of SPEC 17's fill-walk, money and day-mask lists that the preview's own code touches (SPEC:1012-1060).
- Properties: no look-ahead (shuffle the future: every output before the shuffled point unchanged); the walk-forward perturbation test (14.1); byte-equal results at workers 1 / 2 / 8; the luck masks bit-identical to P2's; a twin whose permutation is the identity reproduces the real run.
- Benches, p50 and p99 flat from 10^3 to 10^7 (p99 <= 3 x p50; the p99 at the largest size <= 1.5 x at the smallest): one candle step of the fold (ladder + boxes + touch books), one record's exact-account step, one luck offer, one generator split. FNO-SEC-EXEC's and FNO-RANK's own bench rows run as their parts define them.
- Test spools: the 5 test stocks of round 2 (`W/state/fno-search-preview-20261106/round2/test-3`) before any full data.

### 17.8 Size and cost (every figure an ESTIMATE; re-measured by the test run before the full run)
| Stage | Content | Builder-days |
|---|---|---|
| F0 | tree, vendoring, the parts' suites green, the store door and day mask | 0.5 |
| F1 | phase R: rank adapter, day facts, index series | 0.75 |
| F2 | phase A: the fold, E1 and E2 records, the second-reading scan, the MAZDOCK fixture | 1.25 |
| F3 | walks, exit lanes, charges, spool | 1.0 |
| F4 | phase B: generator and guard, levels, exact accounts, walk-forward, luck, twin | 1.25 |
| F5 | stage 2 and controls, Variant K, report, verdict, rigor database, dashboard | 1.0 |
| | tests and benches inside each stage | - |
| **Total** | | **about 5.75 (range 5.5-6.5)** |

- Wave 0 can run after F0-F3 and a reduced F4 (seeds only): about 3 builder-days from the build's start.
- CPU to the verdict: phase R about 1 CPU-hour; phase A about 10-40 CPU-hours (round 2's F&O phase A wrote 3.5 billion live-model paths in 4,562 s on 12 threads at load 100-200, `W/state/fno-search-preview-20261106/round2/progress.md`:83-84; the preview adds the 1, 2 and 10-minute timeframes and E2's rungs); phase B about 10-80 CPU-hours (round 2's stage 1 ran 10.3 billion exact accounts in 14,235 unit-seconds, its RESULTS.md, "The count tried"); twin seed 1 about phase B again; phase V 1-3. Total about 40-200 CPU-hours, about 5-25 hours of wall time at 8 threads at today's load (209-221 on 14 cores at 21:04 IST).
- Disk: records 5-15 million, about 150 paths each in waves 0-1, about 10-30 GB of spool, plus wave-2 delta walks; far above the floor (about 2.2 TB free today).
- If the test run's projection does not fit the budget, waves are cut by data-blind work units only and the operator is told what was cut; nothing is trimmed silently (BRIEF:723).

---

## 18. Every deviation from the production engine (labelled on every result it touches)

| # | Production (SPEC / PARTS) | Preview | Why |
|---|---|---|---|
| DV-01 | FNO-STRAT, FNO-RUN, FNO-WFT, FNO-RECORDS, FNO-API from merged `feat/db-fix` | a scratch crate on vendored tips, uncommitted | the production parts are not all built or passed; this is a preview (REQ:12) |
| DV-02 | the walk from FNO-SEC-EXEC's passed tip | `d35888c0c`, check r2 FAILED, repair 2 not re-checked; stricter-only cross-check on shown chains | no passed tip exists (R3:686, :694-696) |
| DV-03 | FNO-PAUSE passed | its newest tip, r1 FAILED, repair 1 under check r2 | E1's box needs it; no rewrite |
| DV-04 | FNO-STRAT's touch book, checked | `touch.rs` at FNO-STRAT's tip, built, not checked | no rewrite of the brief's state table |
| DV-05 | FNO-DAYMASK (`DayPlan`, M1-M11, T2T from a table) | round 2's mask + the skip table as data + M8-M11 here; "T2T unchecked" | FNO-DAYMASK r3 FAILED (R3:691); no dated BE/BZ table |
| DV-06 | SPEC 13.4's scheme family (anchored and rolling, generated L and T, pass under every scheme) | one scheme: anchored yearly, T = 12 months, 7 windows | the 6 Nov task and round 3's windows; every Variant K line says "one scheme" |
| DV-07 | a recipe is picked when it passes the per-side gate and the pair checks on the learning window (SPEC:817) | picked by THE RANKING of 6 Nov (REQ:4-5) with the D1 miss key; Variant K beside, never deciding | the operator's newest bar and ranking |
| DV-08 | Rs 1 lakh margin x L(s, D) per position, 20 slots, quantity closest on either side (SPEC 11.1-11.2); FNO-SIZE's four policies | equal amount Rs 1 crore / N, N from seeds 1 / 3 / 5 / 10 / 20, approximate equity steps (8/8, 4/8), whole shares within the amount | REQ:7, :18; ADDENDUM 14 (equal amount; R3:265); closest-either-side breaks the Rs 1 crore cap at N = 1 |
| DV-09 | dated leverage from NSE's C_VAR1 files (FNO-LEVERAGE), three readings | flat 5x, labelled; L-SENS 3x beside | no margin files in the store |
| DV-10 | latency at FNO-CLOCK's calibrated floor; smaller λ labelled "uncalibrated" (SPEC:290-292) | 14 s strict on every stock-day | no calibration table exists (R3:663) |
| DV-11 | S70 swept from no cap to c_min | 10% headline; 25%, no cap and the S70 sweep on shown chains | round 3's lane S |
| DV-12 | FNO-BAND passed, then FNO-BAND-FILES | FNO-BAND's tip, both readings, the lower net; PricRg on 63 sessions | no check on record |
| DV-13 | margin and exposure at the last traded price (SPEC:682-684) | at each position's deployed entry amount | no cross-stock price matrix in the preview |
| DV-14 | anchors A1-A9 (PARTS:1358) | A1 running and A2 frozen only | the operator's "today's high/low Fib (so far)" (SPEC:40) |
| DV-15 | engine, V-SEC (with the 1-minute sub-variant) and V-BOTH | engine headline, V-BOTH labelled; V-SEC not run | V-SEC needs the feed-agreement machinery of SPEC 6.4 |
| DV-16 | timeframes 1-60 min and 1 day (S02) | 1-60 min | the day's running ladder has no daily reading; FNO-DAILY's D+1 path is not in the preview |
| DV-17 | one common primitive path; the search finds patterns (PARTS:1287-1294) | the operator's two seed recipes and the search around them; P0 beside | the ask is the operator's own definition; the dials are the same measured dials |
| DV-18 | S40's resting stop-entry for a break | not run; close and second only | its sizing clock is an open ruling (BRIEF:742) |
| DV-19 | S64: every non-empty combination of S50, S60, S61, S61a, S62 and S63 that holds a stop (SPEC:899) | lanes XA-XJ: one exit beside the stop, XD's two stages, the break-even move on XA-XC | the walks per record stay bounded; every lane is counted as tried |
| DV-20 | FNO-EXITS | exit lanes XA-XJ as preview code on FNO-SEC-EXEC's `Amend` | FNO-EXITS has no branch |
| DV-21 | SPEC 13.4's luck family (deflation, PBO, reality check) | round 3's sign-flip test and the random-entry twin | the 6 Nov requirement (REQ:6) and round 3's one luck module |
| DV-22 | records in the store's own families, served by api routes (SPEC:930-932) | the preview's rigor database in PV/run/ | read-only on the store |
| DV-23 | the 38 checks in `gate.rs` (FNO-WFT) | computed by the preview, one scheme | FNO-WFT not passed |
| DV-24 | Variant K as DRAFTK applies it (M0 at +2.5%, downside at Rs 1 lakh and Rs 5 lakh) | M0 at the newest bar (Rs 2,50,000 / 12.5%) with +2.5% beside; downside at the chain's own sizing | the 6 Nov raise; DRAFTK is not applied |
| DV-25 | price action only in the first run (PARTS:1269, :1298) | context dials of REQ 7a in wave 2, labelled | REQ:21, "every dimension" |
| DV-26 | (no FADE in SPEC) | FADE not run | not the operator's definition |
| DV-27 | VIX only a stamp until the law change is built (U/fib-step2-context-filters.md) | VIX and index bands as wave-2 filters; the indicator group only on 3, 5, 15, 30, 60 min | REQ:17, :21; round 2's readings exist on those timeframes |
| DV-28 | the end date = the last sealed day both feeds prove (M4, OP-45) | 2026-10-30, the round-2 tools' end | AUDIT-3's proof is not read here |
| DV-29 | open rulings left to the main session (BRIEF:741-768) | pinned to the reading that admits less: a padded minute in which GDFL traded, and a minute Zerodha does not hold, as holes; `no_level_ratio` for any R1 minute without a price of its own; `bundled_first_row` on 2019-05-30; no-fill entries never trade rows, the 51% check under both readings; S24 without measured steps; sizing at the start (D-8122) | FIB-I5 (SPEC:179); never a kinder reading |
| DV-30 | (production commits on part branches) | no commit at all; sha256 code identity | the task's law: never move a ref (the repository is public: U/github-repo-is-public) |
| DV-31 | S11's domain 1 .. the pool size (SPEC:854) | N walked 1 .. 50; records only inside top 50 | 50 is the operator's largest seed (SPEC:61); it bounds the records; printed on every rank line |

---

## 19. Open points, each with the choice that keeps the bar

| # | Open point | Choice in the preview | Why it keeps the bar |
|---|---|---|---|
| OP-P1 | M0's bar after the 6 Nov raise (DRAFTK names +2.5%) | Rs 2,50,000 and 12.5% decide; +2.5% printed beside | the newest word, and the stricter one |
| OP-P2 | E2's frozen freeze point | at the close of the level's first touch | keeps "the SAME level" of the first touch; labelled |
| OP-P3 | XD, the operator's "then" | a labelled reading among the lanes, chosen only by the data | the words name an order of targets, not a mechanism |
| OP-P4 | "tight" box | the stock's own lower height bands as the seed reading, "any" as a value | no number was given; generated |
| OP-P5 | K on timeframes above 1 minute | the same K candle counts, all walked | the operator's minutes are on 1-minute charts; labelled |
| OP-P6 | the twin's unit | (timeframe, side, family, exit lane) x year | as round 3's unit x year |
| OP-P7 | the 2026 window holds the idea's own months | the verdict reads all 82 months; 2026 printed apart | nothing hidden, nothing dropped |
| OP-P8 | one scheme only | every Variant K line names it | never claimed as SPEC 13.4's verdict |
| OP-P9 | the luck family of the preview vs round 3's | separate; printed as the preview's own count | no merge of families after the fact (BRIEF:515, :517) |
| OP-P10 | a later merge into round 3's union pool | out of scope; would carry both counts | BRIEF:515 |

No point here waits on the operator; each can be changed by him before a run, and a change is a new family counted in the luck test (`min-returns-amendment.txt`:12).

---

## 20. Not verified, and the data's limits (printed on every report)

- Nothing here was compiled or run; every API named was read from `git show` at the named tip on 6 Nov; the vendored copies' fit at `a6ba0d5d4` is UNVERIFIED until F0 builds them (each tip touches only its module, its slot line, benches and docs, so the copy is expected to be clean).
- The cost of each wave is an ESTIMATE until the test run measures records a second, paths a record and bytes a record.
- GDFL holds trades, not the book: the worst printed price of a second is the worst the data can show; 98.2% of GDFL seconds hold one price, and Zerodha shows minute extremes beyond every GDFL second in 26-47% of minutes (SPEC:1134-1136).
- The feeds agree on all four prices in 57.1% of minutes; mapping is in-session only; 107 raw GDFL steps over 80 stocks are unmarked corporate actions (SPEC:1137-1139).
- Survivorship: current F&O members only; the point-in-time members outside the 218 printed per year (SPEC:1138).
- UNVERIFIED rules taken the worse way: Dhan's per-stock MIS list, the extra margin, the MTM base, the square-off split, brokerage on parts, halt windows, the Muhurat square-off (SPEC:1140-1147); ban days UNAVAILABLE (traded); T2T unchecked; the quote range not loaded.
- The parts used at unpassed tips (DV-02, DV-03, DV-04, DV-12) may change before they pass; a later passed tip makes these results stale, and the report names the tips it ran.

---

## 21. Requirement trace (REQ items 1-16, 7a, 16a; a missing item is blocking, REQ:3)

| Item | Where |
|---|---|
| 1 the bar, Rs 2,50,000 every unseen month, no-trade month fails, +2.5% beside | 14.2, 14.5, 15.1, 16.2 |
| 2 ranking among passers | 14.2 |
| 3 walk-forward always, luck test counting every option, no look-ahead | 13.3, 14.1, 14.3, 17.7 |
| 4 F&O stocks, cash, intraday, Rs 20 lakh margin, Rs 1 crore exposure, worst traded price of the second, every charge | 3, 10, 11 |
| 5 PSRB's Supertrend stop | not applicable (PSRB only); the fib stop is the operator's box stop (O6); a Supertrend lane XI is tested, labelled (9.1) |
| 6 shown results: net >= +100%, full settings, per-year and per-month tables and every figure named | 16.3-16.4 |
| 7 fewer, shorter, bigger trades; every idea tested, never pre-judged | 13, 14.2 (FEWEST, TPM scopes), 12.6 |
| 7a every dimension, nothing missed | 12.2-12.6, 13.1 (context dials paired with every core node) |
| 8 standing principles: coverage, extreme cases, O(1) at p99, parallel, database, audit, search, monitoring, dashboard | 17.5, 17.7 |
| 9 honesty, PREVIEW labels, nothing to real money | the top of this file, 2, 16.1 |
| 10 never stop until the bar is met or what blocks it is shown | 16.7 (closest misses, the nearest bar met, the yardstick) |
| 11 nothing fixed: every number generated from each stock's own past percentiles inside the training months, coarse to fine, labelled | 12 |
| 12 fills by volume, second by second, each slice at its second's worst price; one-second fills labelled | 11.1, 11.2 |
| 13 live Dhan execution, every rule | 9.2, 10, 11.1 |
| 14 F&O only; index context from F&O indices and VIX only | 3.1, 12.6 |
| 15 positions generated from seeds (3/5/10/20; with 1, as the task and R3:262 say) | 10.2 |
| 16 maximise the positive side, minimise the negative side, the fewest trades, the Pareto set, the fewest-trades recipe | 14.2, 16.6 |
| 16a eagle's eyes, cheetah's chase | every stock, timeframe and dial watched (O8, 13); strikes only on the strongest learning-month conditions (small N, ranked admission, TPM scopes); full equal size (8/8 step), lane XI's tight Supertrend, shorter holding a ranking key (14.2) |

---

## 22. Files read for this spec (all read-only, 6 Nov 2026 20:57-21:5x IST)

SPEC.md (1-1148); PARTS.md (365-466, 546-615, 682-803, 861-903, 1266-1365); COSTS.md (1-178); BRIEF (1-769); REQ (1-21); R3 (1-835); BP (1-95); DX (61-118, 251-273); DRAFTK (1-12, 234-315); `W/state/fib-run-20261016/downside-amendment.txt` (1-40) and `min-returns-amendment.txt` (1-12); `check-FNO-LADDER-r4.md`, `check-FNO-RANK-r2.md`, `check-FNO-PAUSE-r1.md`, `check-FNO-COSTS-r2.md` (headers), `check-FNO-PAUSE-r2.progress.md`, `progress-FNO-SEC-EXEC-repair2.md`, `base-spec-FNO-STRAT.md`, `part-FNO-STRAT.progress.md`; the module docs and public items of `fno_ladder` (`9caca9523`), `fno_cross_section` (`079b33146`), `fno_exec` (`d35888c0c`), `fno_pause` (`f8f6ace56`), `fno_strategy/touch.rs` (`2327b1eda`), `costs::equity_clock` (`a6ba0d5d4`); F2 (store_io.rs 1-60, market.rs 360-490, acct2.rs 1-30, wf2.rs 1-40, select.rs 150-200, phase_a.rs 700-760 and 1165-1215, exec.rs and common.rs item lists); P2/engine.rs 895-975; `W/state/fno-search-preview-20261106/round2/{BUILT.md, RESULTS.md (head), progress.md}`; `W/state/psrb-preview-20261106/search/SEARCH-SPEC.md` 8.2-8.3; the memory notes U/{fib-second-touch-entry, every-month-positive-main-bar, results-show-net-over-100pct, backtest-as-dhan-live-trading, sizing-when-many-signals, closest-miss-never-just-failed, every-strategy-fno-stocks-cash-intraday, heart-piece-first-never-move-ahead, fib-strategy-deadline-20261112, fib-step2-context-filters}.md and MEM/{fno-winners-research, precise-entry-patterns-anywhere, scope-fno-stocks-first, backtest-acceptance-bar, nothing-fixed-everything-generated, let-data-decide}.md; STORE folder listings (no file opened).

## 23. COMBINATIONS (requirement 17; appended Sat 7 Nov 2026, before any run)

The full table, in plain words and exact, is `PV/COMBINATIONS.md`; its figures come from `PV/combos/count.mjs` (node BigInt) and `PV/combos/combos.json`, which the search asserts against. Nothing here was chosen from a result: nothing has been run.

### 23.0 The numbers

- **Guaranteed part (exact, fixed now, data-blind): 131,840,470,560 evaluated combinations; base-2 logarithm 36.940.** It is at least 100,000,000,000 whatever the data holds, because every band count is fixed in advance and a skip by a proven bound counts as tried.
- At most 23,878,281 more in stage 2 and at most 943,488,000 more in the adaptive drill-down (finer bands, a 4th to 6th filter).
- **Expected total: 132,807,836,841; base-2 logarithm 36.951.**
- The same table with each generated recipe counted once, not once per learning window: 21,222,014,340. Requirement 17 is judged on the first figure, the one the luck test uses (BRIEF section 9 rule 2 counts per distinct learning window, and near-duplicates get no discount), and every report prints the second beside it.
- 50.0% of the guaranteed part (65,920,235,280) is FADE sides.

| Wave | Evaluated combinations |
|---|---|
| Wave 0: the operator's seeds | 27,480 |
| Wave 1: levels 1-2 and the exits | 2,278,848,360 |
| Wave 2: every context filter, singles, pairs, triples | 129,561,594,720 |
| Stage 2 (at most): money, controls, exit drill on each window's candidates | 23,878,281 |
| Adaptive drill-down (at most) | 943,488,000 |
| **Expected total** | **132,807,836,841** |

### 23.1 Stages

| Stage | Exact count |
|---|---|
| `W0.seeds` | 27,480 |
| `W1.base` | 456,360 |
| `W1.level1` | 118,918,080 |
| `W1.exits1` | 3,560,040 |
| `W1.exits2` | 4,520,040 |
| `W1.level2` | 2,151,393,840 |
| `W2.singles.seeds` | 330,259,200 |
| `W2.singles.base` | 495,900,840 |
| `W2.codes` | 166,164,480 |
| `W2.triples` | 95,027,264,640 |
| `W2.pairs.base` | 3,849,768,360 |
| `W2.singles.level1` | 2,633,575,320 |
| `W2.pairs.seeds` | 7,093,779,600 |
| `W2.singles.level2` | 19,964,882,280 |
| **Guaranteed part** | **131,840,470,560** |

Timeframes: 1 min 1.2 B, 2 min 1.1 B, 3 min 4.0 B, 5 min 11.1 B, 10 min 4.3 B, 15 min 35.0 B, 30 min 38.5 B, 60 min 36.7 B. 83.6% sits on 15, 30 and 60 minutes, fixed by cost only. Extension, not committed: 120,355,379,340 more.

### 23.2 What counts

One evaluated combination = one exact account over all 98 months for one recipe identity (timeframe, account side, family, core node, filter set, exit config, N, and the learning window when any value is generated), or a skip by a proven exact bound, counted as tried. Near-duplicates get no discount. `distinct_total`, "with at least one trade" and "each generated recipe once" are printed beside `tried_total`. Twins are not counted.

### 23.3 Shards

One shard = timeframe x side x family x exit lane x wave unit; resumable; 170,496 before chunking; run in the fixed order of COMBINATIONS.md; a deadline cuts whole units, never by results, and an achieved total under 100,000,000,000 is reported as requirement 17 NOT MET.

### 23.4 Estimate (ESTIMATE; 17.8's 40-200 CPU-hours no longer holds)

About 51.1-79.1 hours of phase B wall time at the rates measured on this Mac under load (12-13 threads); the five-stock test re-measures before the full run. Dominant stage: `W2.triples` (72.1% of the count, 22.0 of 79.1 model hours).

### 23.5 Amendments to the sections above (no earlier line was edited; these rows win where they differ)

| Section | Amendment | Why |
|---|---|---|
| 7.3, DV-26 | FADE runs: FADE-LONG, FADE-SHORT, FADE-BOTH on every level, defined in COMBINATIONS.md | REQ 17 names FADE and binds |
| 12.2 | S33 has 16 distinct values, not 29 | the other 13 repeat a value |
| 12.2, 12.6, 5.3 | new values and dials: S12 clocks and ordinals, S25, S37a, S39, seven anchors as position bands, trigger volumes, candle codes | REQ 17, REQ 7a |
| 12.6, 13.1 | the level lattice and the filter pairing follow COMBINATIONS.md's stage table; level 2 x filters on 1, 2, 3, 5 minutes is the extension | a data-blind cost choice; the literal reading could not finish on this Mac |
| 14.4 | twin seed 1 always runs waves 0-1; its wave 2 runs only if the real headline meets the bar | rule 2 of the verdict already gives NO otherwise, and no YES is possible without it |
| 17.6, 17.8 | the main session's 7 Nov limits: 12 threads, normal nice, 8 GB per process (RSS guard 7.5 GB), per-stage target dirs; the wall time above replaces 17.8's | the round-2 PSRB sweep has ended |
| 3.3 | REQ 18: no futures column anywhere; days to expiry is the calendar only | operator, 7 Nov |
| 21 | requirement 17: this section and COMBINATIONS.md; 18: 3.3 as amended; 19: downloads replace their labels only after READY.md = PASS | REQ:22-24 |

## 24. THE SNIPER VIEW (requirement 16b; appended Sat 7 Nov 2026, before any run)

The full text, with every table, the worked vectors and the labelled arithmetic, is `PV/SNIPER.md`; this section is its binding short form. The figures come from `PV/combos/count.mjs` (node BigInt) and `PV/combos/combos.json`. Nothing here was chosen from a result: nothing has been run, and no result, trades file or search output was read to write it.

### 24.1 The entry gates, dimension "Sniper gates"

A gate reads only what is known at the decision T. The sizing price of 10.2 is at least 14 s later, so the gates read REF, the last GDFL print known at T (`r1_g` on the close reading, the trigger second's last print on the second reading), and the stop and first target as 7.3 fixes them, R1-mapped, rounded never kinder. Formula: SNIPER.md 1.2. Neither gate moves a stop, a target or a size.

- **S-STOP:** the stop's distance from REF, ppm of REF (reports print bp), at the walk's own stop. Seed "any" (no clause), so wave 0 and wave 1 are unchanged. Generated per stock, per window, on learning months (12.1): the 9 full bands; coarse 4, the usual 3 + at or below own p50.
- **S-RR:** the reward to the lane's first target over the stop, in thousandths. Seed "any". Labelled seeds at least 2, 3, 5 ("seed (main session for REQ 16b's k)"). Generated: the same 9 bands; coarse 6, the 3 seeds + the usual 3.
- **First target by lane:** XA, XD2, XD3 the day-extreme break + offset; XB 1.272; XC 1.618; XE its arm, the day-extreme break + offset, and XF its arm rung, both labelled "reward to the arm"; other rung targets that rung; the excursion target REF + x R; XJ the bounce high + offset; the break-even move changes nothing. **XG, XH and XI are refused by name, `s_rr_no_target`:** they have no price at the decision, and a refused pair is neither evaluated nor counted. S-STOP runs on every lane. The no-stop comparison lane refuses both, `sn_no_stop`.
- **Both count where filters count,** singles, pairs and triples, under 23's rules, in the new stages of 24.6. `D_stop <= 0` at T holds an absent value, which fails every gate clause and is counted, `sn_stop_through_at_decision`. FADE reads the same two distances, mirrored.

### 24.2 The selection scope SNIPER (14.2)

No new series: another heap over every series, from wave 0 on, chosen on learning months only. **Cells, 12:** trades averaging at most 2, 5, 10 or 20 a month on the learning months (labelled seeds; the same average as TPM, R3:326) x a largest single-trade loss at or below p25, p50 or p75 of that reading over the window's series at that cap and N seed (generated, class S; one population per window x cap x N seed; histogram edges snapped down, class C). Inside a cell, THE RANKING as usual. Each cell chains over the 82 unseen months, `SNIPER-c-pb`. **The one chain to read, SNIPER, fixed now:** in each window the #1 of the first cell, tightest first, that holds a learning-month passer; if none, the #1 of the loosest cell by the miss key.

**SNIPER stays a labelled chain, printed beside the headline. It is not the headline and never writes line 2; 14.5 does not change.** A switch would be honest only by the operator's word before wave 0's first print, recorded and sealed; after any print it is a new family judged on fresh months.

**If the SNIPER chain meets the bar in all 82 unseen months:** that shows a rule fixed before the run, reading learning months only, gave few trades and small losses and met the bar on months it never read, under lane S and every charge. It does not show the edge is real until the same luck test and twin clear for it, nor that it holds live; line 2 still reads the headline, and every label of section 20 stays. **It counts in the luck test:** every gated series is in `tried_total` and the one luck stream (14.3), its O is printed against the same L95, and the gates raise the luck bar for every chain, the headline too.

### 24.3 The fields and their owners

- **F1:** nothing new.
- **F2, on every record (5 reserved columns, raw facts, not dials):** `sn_ref_px`; `sn_stop_paise`, `sn_stop_ppm`; `sn_t1_paise`, `sn_t1_ppm`.
- **F3, on each walk path (a sibling unit, 12 bytes a path, so `PathRow` stays 28 bytes and the search's hot path reads nothing more):** `adv_paise`, `fav_paise`, `fill_off`, `stop_off`; and `rule::STOP_THROUGH_AT_FILL`.
- **F4:** `gen::sniper::distances` at each walk's stop and first target (O(1), equal to F2's columns at the seed), the two gates, per series `win_sum` and `worst` by month, hence largest loss, average win, average loss, payoff ratio, trades a month, losing months, worst month; the cells, the chain, the seven stages asserted against `combos.json`. The generator skips F2's 5 columns by name. A bound that skips a series must also prove it can enter no SNIPER heap.
- **F5:** adverse and favourable in bp and in multiples of the stop, seconds held; the table, the plain check, `S2.sniper.*`. No change to the verdict function.

### 24.4 The SNIPER table, in every report

Trades a month; win rate; average win; average loss; payoff ratio; largest loss in Rs and % of account; max drawdown; worst month; losing months and no-trade months; months at the bar on both the fixed Rs 2,50,000 and line 2's carried floor; average holding time; adverse and favourable move in bp and in multiples of the stop; stop hit share within the first minute and the first 5 minutes; the plan against the fills. The headline and SNIPER rows always print, as the one line naming the best if neither nets +100% (REQ item 6); the cells print at +100% or more. It sits in part 6 of 16 and in the closest misses (part 7), from wave 0's interim on.

### 24.5 The plain check, printed under the table

Labelled arithmetic, not a result, from `combos/sniper-arith.mjs` (COSTS.md's calculator, worst reading, worse clock): the charges of a round trip at each N; the net a trade the bar needs at each N and k trades a month; the operator's picture at N = 1 (at Rs 1 crore and 2 trades a month, each trade must gross about 1.29%; with a 25 bp stop and 5 trades, 2 winners must each make about 6.9 times the stop). And the tension, on every table: a month with no trade fails the bar; a tight stop with 14 s of latency and worst-print fills gets hit more often and fills worse. The test measures it; nothing is assumed.

### 24.6 The recount and the estimate

Seven new stages `W2.sniper.*`, each its mirror's shape, holding exactly the filter sets with at least one gate value; each runs right before its mirror. New stages, not new library values, so that every earlier stage keeps its exact count (448 earlier cells checked against the backup), no finished unit reopens, the cost stays on its own rows, and the operator's newest ask runs early. Six stage-2 rows `S2.sniper.*` (420 candidates) and one adaptive row. The new figures are in 24.9.

**About 56.3-84.3 hours of phase B wall time** at the rates measured under load, against 51.1-79.1: the new stages add 5.2 of 84.3 model hours, so **no cut is needed.** ESTIMATE; about 3-10% more on every series for the new figures and heaps is not in the model, and the five-stock test prices it. If the projection does not fit, cut by data-blind cost, most hours per combination first, whole units, named in the report: `W2.singles.level1` on 3 and 5 minutes; the rest of `W2.singles.level1`; `W2.pairs.seeds`; `W2.singles.level2`. All four: about 38.5 model hours saved, 115.6 billion still guaranteed.

### 24.7 Amendments to the sections above (no earlier line was edited; these rows win where they differ)

| Section | Amendment | Why |
|---|---|---|
| 12.6, 13.1, 23 | the two gates and the stages of 24.6; the fixed run order takes each new stage before its mirror | REQ 16b |
| 14.2, 16 | SNIPER's 13 chains are printed beside the headline; part 2 gains the chain's line, part 6 the table and the plain check | REQ 16b |
| 14.5 | no change | the pre-registered headline judges a pass |
| 10.4 | S81 (b) reads the stop distance at the decision, not `PathRow.stop_dist`, which runs from the first fill, at or after the start second that admission decides at | no look-ahead (REQ item 3) |
| 14.4, 23.5 | if the SNIPER chain meets the bar and the headline does not, twin seed 1 runs the `W2.sniper.*` stages, and a second twin keeps S-STOP and S-RR with their records, because a small stop gives small losses whether or not the entry is good | the chain's null test, without counting a tight stop as skill |
| 11.4, 17.5 | F3's sibling unit of path facts | REQ 16b's adverse move |
| 21 | requirement 16b: this section and SNIPER.md | REQ:25 |
| 23.0-23.4 | those figures are the plan of 09:01 IST, kept as written; 24.9 is the current plan | the spec is append only |

### 24.8 Open points (these two wait on a ruling, unlike section 19's)

| # | Open point | State | What it needs |
|---|---|---|---|
| OP-P11 | **Line 2's carried floor.** 14.5 rule 2 reads max(Rs 2,50,000, 12.5% of the carried capital), and the exposure stays capped at Rs 1 crore. At the slowest possible growth the floor is about Rs 42 lakh in unseen month 25 and about Rs 348 crore in month 82, so no realistic chain can say YES, by arithmetic alone (SNIPER.md 5.4, table D). | 14.5 unchanged; both readings printed on every chain; O already reads the fixed Rs 2,50,000 | **The operator's word before the seal:** a salary on Rs 20 lakh means the fixed floor, reading (c) |
| OP-P12 | **The bar inside the miss key (14.2).** If it grows with each series' own carried capital, more profit can raise the shortfall: over 88 months, a flat Rs 5,00,000 a month owes about Rs 21.8 crore, a flat Rs 2,50,000 about Rs 12.0 crore, so the worse one ranks first. | one function in F4, so the ruling is one line | The main session's ruling before F4 seals; the fixed bar is monotone |

The main session also adds to frozen `types.rs`: `Scope::Sniper { cap, band }`, `Scope::SniperPrimary`, `rule::STOP_THROUGH_AT_FILL`. The 09:08 fingerprint `daf6c43c74b7` does not cover this section: the plan needs a new seal before any run.

### 24.9 The numbers

- **Guaranteed part (exact, fixed now, data-blind): 145,276,706,220 evaluated combinations; base-2 logarithm 37.080.** It is at least 100,000,000,000 whatever the data holds, because every band count is fixed in advance and a skip by a proven bound counts as tried.
- At most 33,913,341 more in stage 2 and at most 968,889,600 more in the adaptive drill-down (finer bands, a 4th to 6th filter).
- **Expected total: 146,279,509,161; base-2 logarithm 37.090.**
- The same table with each generated recipe counted once, not once per learning window: 23,247,542,760. Requirement 17 is judged on the first figure, the one the luck test uses (BRIEF section 9 rule 2 counts per distinct learning window, and near-duplicates get no discount), and every report prints the second beside it.
- 50.0% of the guaranteed part (72,638,353,110) is FADE sides.
- 9.2% of the guaranteed part (13,436,235,660) holds a Sniper gate (requirement 16b); without those stages it is 131,840,470,560, which alone stays above 100,000,000,000.

| Wave | Evaluated combinations |
|---|---|
| Wave 0: the operator's seeds | 27,480 |
| Wave 1: levels 1-2 and the exits | 2,278,848,360 |
| Wave 2: every context filter, singles, pairs, triples | 142,997,830,380 |
| Stage 2 (at most): money, controls, exit drill on each window's candidates | 33,913,341 |
| Adaptive drill-down (at most) | 968,889,600 |
| **Expected total** | **146,279,509,161** |

| Stage | Exact count |
|---|---|
| `W0.seeds` | 27,480 |
| `W1.base` | 456,360 |
| `W1.level1` | 118,918,080 |
| `W1.exits1` | 3,560,040 |
| `W1.exits2` | 4,520,040 |
| `W1.level2` | 2,151,393,840 |
| `W2.sniper.singles.seeds` | 11,347,200 |
| `W2.singles.seeds` | 330,259,200 |
| `W2.sniper.singles.base` | 23,614,560 |
| `W2.singles.base` | 495,900,840 |
| `W2.codes` | 166,164,480 |
| `W2.sniper.triples` | 11,594,075,880 |
| `W2.triples` | 95,027,264,640 |
| `W2.sniper.pairs.base` | 351,371,520 |
| `W2.pairs.base` | 3,849,768,360 |
| `W2.sniper.singles.level1` | 105,648,300 |
| `W2.singles.level1` | 2,633,575,320 |
| `W2.sniper.pairs.seeds` | 547,336,440 |
| `W2.pairs.seeds` | 7,093,779,600 |
| `W2.sniper.singles.level2` | 802,841,760 |
| `W2.singles.level2` | 19,964,882,280 |
| **Guaranteed part** | **145,276,706,220** |

### 24.R Main session rulings on OP-P11 and OP-P12 (Sat 7 Nov 2026 ~09:5x IST; told to the operator, who can overrule; an overrule is a new family in the luck test)
- **OP-P11:** line 2 of every report and the verdict function (14.5 rule 2) judge the FIXED Rs 2,50,000 net in every unseen month, the operator's own words ("per month I need 2.5 lakhs"). Why: 12.5% of a carried capital grows to about Rs 42 lakh by unseen month 25 and about Rs 348 crore by month 82 while exposure stays capped at Rs 1 crore, so no chain could ever say YES, by arithmetic alone. The 12.5%-of-capital reading and the +2.5% tier are printed beside every chain, labelled, and never decide line 2. This section overrides 14.5 rule 2 on that one point; every other rule of 14.5 stands.
- **OP-P12:** the miss key's bar is the same fixed Rs 2,50,000 (a bar growing with each series' own capital would rank a flat Rs 5 lakh a month below a flat Rs 2.5 lakh a month).
- **Seeds:** the S-RR seeds 2, 3, 5 and the trade-cap seeds 2, 5, 10, 20 stand, labelled as the main session's seeds, not the operator's numbers.

### 24.10 Repairs after the independent check, round 1 (Sat 7 Nov 2026 ~10:0x IST, before any run; no earlier line was edited; these rows win where they differ)

`PV/sniper-check1.md` found 6 blocking points in the words of this section, none in a number. `PV/sniper-repair1.md` lists each fix, and SNIPER.md carries the full text. **No count, stage, charge or estimate figure moves: 24.9 stands.** The new seal must cover this section too.

| Section | Amendment | Why |
|---|---|---|
| 24.2 (its paragraph "If the SNIPER chain meets the bar"), 24.7 (its row 14.4, 23.5) | "The bar" there means 14.5 rule 2's floor as line 2 reads it at the seal: today the fixed Rs 2,50,000 (24.R). The branch reads line 2's own floor function, and its sentence names the floor. If line 2 is overruled back to the carried floor, a chain that holds only the fixed floor prints "met the fixed Rs 2,50,000 in all 82 unseen months (reading c): not line 2's floor" and gets none of the prepared sentences | two bars exist (SNIPER.md 5.4), so "the bar" alone was a kinder door by wording |
| 24.7 (its row 14.4, 23.5) | The twin branch fires when the SNIPER chain holds the fixed Rs 2,50,000 in all 82 unseen months and the headline does not meet line 2's floor. It prints the reading that fired it: "twin branch fired by line 2's floor, the fixed Rs 2,50,000 (24.R)", or after an overrule, for a chain short of the carried floor, "twin branch fired by the fixed Rs 2,50,000 (reading c): not line 2's floor" | a null test costs hours and writes nothing, and the reader must see which floor ran it |
| 24.4, SNIPER.md section 4 | Months at the bar: at line 2's floor (today the fixed Rs 2,50,000) and at the carried reading (a) beside it, never deciding | after 24.R the carried floor is not line 2's |
| 24.7 (its row 14.5) | "No change" means no change by this plan; 24.R's one point on rule 2's floor is the main session's, apart from it | 24.R came later |
| 24.8 OP-P11 | Ruled by 24.R. "A salary on Rs 20 lakh" is struck as a reason. The operator's words on record are "per month I need 2.5 lakhs"; "salary" is an earlier note's paraphrase (U/every-month-positive-main-bar.md:22, outside its quotation marks), and the carried base was the main session's default, told to him and not changed (:18, :23). So the question goes to him with table D: is Rs 2,50,000 a month the floor for good, or should the floor grow with the account? | a paraphrase must not carry the ask for a kinder reading |
| 12.1, 24.1, 24.2, COMBINATIONS.md's stage 2 line | Every seed value says "seed (operator, <file:line>)", except the main session's own, which say "seed (main session for <what>; the operator gave no number)": S-RR's 2, 3, 5 (<what> = REQ 16b's k), the SNIPER caps 2, 5, 10, 20 (REQ 16b's trade caps, printed for every `Scope::Sniper`), stage 2's month cap 2, 5 (REQ 16b's stage 2 month cap). Both forms are class labels under R3:430. S-RR's seeds still count as one series (E) | frozen `ValueClass::Seed` means the operator's number, and these are not his |
| 24.8 (its last paragraph) | The additions to frozen `types.rs` also hold `ValueClass::SeedMain(&'static str)`, and `SnWalkRow { window: u8, step: u8, cap: u16, band: u8 }` with `WaveOut.sn_walk: Vec<SnWalkRow>` (the walk's step and cell for F5; step 0 is `sn_no_cell`, and `Chain.picks` then skips that window) | the frozen types cannot print the label or carry the step; the table's "the chain's cap" needs the cell too |
| 13.1, 24.3, COMBINATIONS.md "What counts as one evaluated combination" | A skip by a proven bound must also prove the series enters no SNIPER histogram, heap or slot: in every window it is offered to, it has no learning trade, or it is above every cap (more than 20 x learning months trades). Otherwise it is evaluated. "Above every cap" needs a proven lower bound on trades; kept records give only an upper one, so without such a proof that branch never fires. The byte-equal proof (13.1, R3:531) also compares the histograms, the edges, the cell heaps, the slots and the walk's picks | 24.2's edges are quantiles over every series at or under a cap, and a skipped series has no known largest loss, so a skip would move the edges; R3:525 says a skip may change no counter |
| 24.2, 14.2 | **The one chain to read, SNIPER:** in each window, over the 12 cells tightest first, an empty cell passed over: (1) the #1 of the first cell holding a learning-month passer; (2) else the first cell holding a series with a trade in every learning month, and in it the best such series by the miss key; (3) else the loosest cell holding anything, walking back from cap 20 p75, its #1 by the miss key; if every cell is empty, no pick (`sn_no_cell`), and that window's unseen months are no-trade months. Each window prints its step and cell. A cell's stage 2 candidates are its top 5, or its top 4 and its every-month slot: at most 5, so the 420 stands. It reads learning months only, takes no typed number and adds no series; MIX-WF-1 covers it | falling straight to cap 20 p75 would make the chain the operator reads the least sniper of the 12; the miss key rewards total net and cannot favour a tight cell; 14.2 calls every candidate a miss today, so a fixed floor need not bring passers |
| 24.4, 16.3 | The headline and SNIPER rows always print, each as one row with every column of the SNIPER table, whatever their net, marked "below +100%: table row only" under +100%. REQ item 6's limit applies to full settings and the per-year and per-month tables, which shrink to the one line naming the best | REQ 16b (c) asks for the table in every result; a name and a net alone would hide the adverse move, the stop hit share and the largest loss of the chain he asked about |

**One more open point, OP-P13: THE RANKING's passer test (14.2 rule 1).** 24.R names line 2, 14.5 rule 2 and the miss key. It does not name 14.2's own test, which still reads max(Rs 2,50,000, 12.5% of its start capital), on learning months when choosing and on unseen months when judging (R3:603), so verdict rule 4 reads it too. Left so, a series at the fixed floor in every learning month is a miss with zero shortfall, ranked by worst month and not by lowest max drawdown, and a chain can hold line 2's floor and still rank as a miss against its twin. Step 1 of the walk reads 14.2's own function, so it follows whichever is ruled. This section does not rule it, because it touches the headline's pick: it needs one line from the main session before F4 seals.

### 24.11 Repair after the independent check, round 2 (Sat 7 Nov 2026 ~10:3x IST, before any run; no earlier line was edited; these rows win where they differ)

`PV/sniper-check2.md` found 1 blocking point in the words of this section, none in a number, and all 6 repairs of 24.10 held. `PV/sniper-repair2.md` lists the fix, and SNIPER.md 2.3 carries the full text. **No count, stage, charge or estimate figure moves: 24.9 stands, and twins are not counted.** The new seal must cover this section too.

| Section | Amendment | Why |
|---|---|---|
| 24.7 (its row 14.4, 23.5), 24.10 (its second row) | **The branch's twin is a partial null and prints as one.** It runs twin seed 1 on waves 0-1 and the seven `W2.sniper.*` stages of 24.6 only, with no other wave 2 stage, no stage 2 and no adaptive row. Both twin lines of the branch print: "partial null: twin seed 1 only, waves 0-1 and W2.sniper.* only, 15,715,111,500 of the real 145,276,706,220 options; a smaller search finds less luck, so beating it is easier than 14.4's twin, and it does not clear the chain". The second twin is drawn on seed 1 as well | the real chain picks from every series of every stage, 14.4 asks for the whole pipeline with the real run's work budgets, and 23.5 narrows that only for the headline's case; a search with 10.8% of the options finds less luck, so "twin cleared" would read as a full null: a kinder door by wording |
| 24.2 (its paragraph "If the SNIPER chain meets the bar"), SNIPER.md 2.3 | "Until the same luck test and twin clear for it" reads "until the same luck test and a twin of the same search clear it (14.4: the whole pipeline, real budgets, seeds 1-3)" | the branch's twin is not the same search |
| 24.7 (its row 14.4, 23.5, last column), SNIPER.md 2.3 | "The chain's null test" reads "the chain's partial null". What beating the second twin says: "Beating the second is a first sign that the entries add something beyond a tight stop; on a partial null it is no more than that." | the second twin has the same gap as the first |
| 14.4, 16.6, 16.7, 16.8 | Twin seed 1 always runs waves 0-1, so any print of the twin's SNIPER chains (SNIPER and any `SNIPER-c-pb`) from waves 0-1 alone names its stages the same way: "partial null: twin seed 1 only, waves 0-1 only, 2,278,875,840 of the real 145,276,706,220 options; a smaller search finds less luck, so beating it is easier than 14.4's twin, and it does not clear the chain". Wave 1's interim prints it too, though the real chain has searched only those stages so far: the line measures against the full search, and nothing at that interim clears anything. The figures print from the achieved stage counts, which F4 asserts against `combos.json`, so after a named cut the real figure is the achieved one. The line drops only for a seed whose whole pipeline ran (by 23.5, once the real headline meets the bar). F5's test: no twin SNIPER print from less than the whole pipeline goes without it | this is the print a reader meets most often, and 1.57% of the options is further still from 14.4's twin |
| 14.4, 23.5 | A complete null means all of wave 2 on the twin: about 75.6 model hours on top of waves 0-1's 8.75, so about 84.3 for one whole seed, and 14.4 asks for three, about 253 (ESTIMATE, the guaranteed stages only). It is the main session's call if the branch ever fires; this section does not decide it | the cost of the honest test is named and not chosen here |

### 24.T The day cap (operator, Sat 7 Nov 2026 ~10:3x IST; requirement 22)
Every exact account, in every wave and stage including wave 0 and the twins, takes at most 10 trades a session, counted at entry on the account across all its recipes and routes. The 11th signal is refused by name ("day cap") and counted; the check is one integer per account-day, O(1). This overrides 13.1's "FIRST admission and no account controls" on this one point. Any account-control value above 10 a day is dropped; values below 10 stay searchable. The combination counts do not change, and evaluation does. No net or count is computed without it.

### 24.S Max Rs 2 lakh a trade (operator, Sat 7 Nov 2026 ~10:4x IST; requirement 23)
The amount per entry is min(Rs 10,00,000 of exposure, Rs 1 crore / N), with margin at most Rs 2,00,000 at the stock's leverage. So N = 1, 3, 5, 10 all enter Rs 10 lakh and differ only in how many may be open at once; N = 20 enters Rs 5 lakh. Stage 2's refined N follow the same formula. The walks' distinct amounts fall from 8 (Rs 1 crore, 50, 33.33, 20, 16.67, 10, 5, 2.5 lakh) to 3 (Rs 10, 5, 2.5 lakh: the N seeds x steps 8/8 and 4/8), which overrides COMBINATIONS.md's walk line and shrinks phase A. The combination counts do not change. SNIPER.md's plain check is re-read at Rs 10 lakh a trade, as requirement 23 prints it. This overrides 10.2 and 16a's "full equal size" on this one point: full size now means Rs 10 lakh.

### 24.12 Net is the centre (requirement 21a; operator, Sat 7 Nov 2026 ~10:2x IST; before any run; no earlier line was edited; these rows win where they differ)

No gross P&L field, key, column, table, chart, filter, ranking or sentence exists anywhere in this tool, the spool, the database and the ledger included. A trade is its fills and its charge items; its net = money in - money out - charges, on the worse clock (`TwoClock::net`; charges are `TwoClock::headline`; `TwoClock::gross` is never called). Every decision and every figure reads net, and charges are said in rupees per trade and per month, never as a share. The "signal only, before charges" score is retired, and there is no second score. **No key, count, stage, estimate or verdict moves, because none read gross.** The new seal must cover this section too.

| Line | Reads now |
|---|---|
| 52 (O10) | one score: net of every share charge |
| 57 (O15) | "the 25 Oct small check passed nothing; net about -1R a trade, charges about 1R a trade (about Rs 880 at Rs 5 lakh notional)" |
| 299 | deleted: the charges rule above it stands, and there is one score |
| 302 | "the net (worse clock), charges, exit second ..." |
| 453 | "NET" alone |
| 462 | "net (Rs and %); charges; charges per trade and per month, in rupees; trades; ..."; the month table and `months-<chain>.tsv` read "month, net Rs and %, charges, trades, ..." |
| 748; SNIPER.md:291; combos/section-24.md:37; combos/section-24-head.md:37 | "each trade's price must move X% in its favour to net the Rs Y it needs", the numbers from 24.S's re-reading |
| SNIPER.md:236; combos/sniper-arith.md:23 | column heads "N = k: price move" |
| SNIPER.md:248; combos/sniper-arith.md:35 | column "Each winner must net (Rs)" = (the bar + the data charge + (k - w) x each loser's cost) / w |
| combos/sniper-arith.mjs:56, 62, 72, 98, 107, 108 and its JSON | the same words; key `grossToday` dropped, `winGross` becomes `winNet` |
| build/INTERFACES.md:173; `PathRow` and `MonthCell` in `types.rs` | `gross` becomes `charges` at the same place; `PathRow` stays 28 bytes; `charges` is `TwoClock::headline` on the row |
| 17.8's ledger, PROOF-SPEC.md 2.2 | no `gross` column in the `T`, `A` or `M` row |
| the row above (correction, append only) | "17.8's ledger" is a slip: 17.8 is size and cost. The ledger meant is the one `build/NOTES-shared.md` 09:5x gives F3 and F5, PROOF-SPEC.md 2.2 |

### 24.U The newest ceilings (operator, Sat 7 Nov 2026 ~10:5x IST; requirement 25; replaces the numbers of 24.T and 24.S, whose rules stand)
Every exact account, in every wave and stage including wave 0 and the twins, takes at most 5 trades a session and at most 100 trades a calendar month, each one integer per account, O(1); refusals are counted as "day cap" and "month cap". The amount per entry is min(Rs 15,00,000 of exposure, Rs 1 crore / N), with margin at most Rs 3,00,000 at the stock's leverage: N = 1, 3, 5 enter Rs 15 lakh and differ only in how many may be open at once, N = 10 enters Rs 10 lakh, N = 20 enters Rs 5 lakh; stage 2's refined N follow the same formula. The walks' distinct amounts become 5 (Rs 15, 10, 7.5, 5, 2.5 lakh: the N seeds x steps 8/8 and 4/8). Fewer trades stay searchable: day-cap seeds 1, 2, 3, 5 and month-cap seeds 5, 10, 20, 40, 60, 100 replace any higher control values (labelled main-session seeds), and the SNIPER cap cells gain 40, 60 and 100 only if the chain count stays small. The combination counts do not change unless the final pass says so. The plain check and every other Rs 1 crore or Rs 10 lakh line are re-read at Rs 15 lakh.

### 24.13 Rulings on the net-only sweep (main session, Sat 7 Nov 2026 ~10:5x IST; requirement 21a; before any run; no earlier line was edited; these rows win over 24.1, 24.12, SNIPER.md 1.2 and 1.4 where they differ, and each row says what it overrides)

| Ruling | Reads now | Overrides |
|---|---|---|
| N1 | `PathRow.gross: i32` becomes `charges: i32` at the same offset, charges in paise on the worse clock (`TwoClock::headline`); `PathRow` stays 28 bytes. `MonthCell.gross` becomes `charges` | "drop it" in `build/NOTES-shared.md`:9; sweep choice 1 |
| N2 | F0's vendored text: `common.rs` `Path` and `Rec` are cut with their impls, and `VENDORED.tsv`'s `how` column says so. `rigor/rf/runner.rs`:818 stays: it is byte-exact and not compiled. No line may declare `mod rf`, because `rigor/rf/cmd.rs` imports the cut types | sweep choice 2 |
| N6 | S-RR uses planned NET, defined below. S-STOP, S60, S65, S81 (b) and the excursion fields (`PathFact` adv and fav, `fav_ppm_r`) stay price distances, each labelled "price distance, not P&L" wherever it is printed | 24.1's S-RR line; SNIPER.md:53, :57 and the S-RR column of its worked vectors |
| N7 | Line 57 (O15), printed on every report as the prior (16.1): "the 25 Oct small check passed nothing; every family lost on both sides; worst-price fills Rs 430 to 707 plus charges Rs 224 to 231, together about 1R a trade (1R is about Rs 880 at Rs 5 lakh notional)". Round 3's R-02: "Round 1's net of 1.77 bp is 33 times short of the needed net of 58.66 bp; charges were 4.94 bp a trade" | 24.12's row for line 57; `build/NOTES-shared.md`:32; sweep F-16 and R-02 |
| N8 | Charges are never a results column beside net. A report prints them only in a cost section (charges in Rs per trade and per month), to explain, never as a P&L figure. Line 462's chain figures read "net (Rs and %); trades; ...", with "charges" and "charges per trade and per month" moved to that chain's cost section. Its month table and `months-<chain>.tsv` read "month, net Rs and %, trades, wins, ..." with no charges column. `MonthCell.charges` and `PathRow.charges` stay stored, for the cost section only; they decide nothing | 24.12's row for line 462; sweep F-20, F-21 and F-22 |

**S-RR on planned net (N6).** Exact integers, no float, O(1) per record and walk. Net is the centre, and charges matter most on a tiny stop:

```text
qty     = floor(amount / REF)        whole shares; amount is the walk's own (WalkKey.amount), REF as 24.1
charges = TwoClock::headline of one round trip: buy qty at REF, sell qty at REF, on the decision's date (FNO-COSTS, the same calculator as the walk)
S-RR    = floor(1000 x (D_tgt x qty - charges) / (D_stop x qty + charges))
```

- **No look-ahead:** REF is the last print known at T. The sizing price, the first fill and `stop_dist` are never read. Both legs are priced at REF because no other price is known at T (this file's reading of "on the last print known at the decision").
- **Per walked amount:** charges are not proportional to quantity, so one record reads a different S-RR at each amount. The walk's key already holds the amount, so the per-walk arrays of SNIPER.md section 3 hold it with no new dimension, and generated bands read each walk's own amount.
- **Floor, not truncation:** the numerator can be negative, so use `div_euclid` on `i128`. With `D_stop > 0` and `qty >= 1`, S-RR is above -1,000, so it cannot reach `ABSENT32`.
- **`D_tgt = 0`:** S-RR = floor(-1000 x charges / (D_stop x qty + charges)), a real negative value that fails every "at least" clause. This overrides SNIPER.md:57.
- **`qty = 0`** (REF above the amount): S-RR holds `ABSENT32`, every S-RR clause fails, and it is counted by name, `sn_no_quantity`, per unit.
- **Unchanged:** `D_stop <= 0` stays absent as `sn_stop_through_at_decision`; XG, XH and XI stay refused by name; FADE reads the same distances, mirrored; neither gate moves a stop, a target or a size. Seeds at least 2, 3, 5 are 2,000, 3,000, 5,000 on this reading.
- **No count moves; the gate's values do.** `combos/combos.json` and every stage count stand, and `COMBINATIONS.md`:50 and :211's "reward to the first target over the stop" now means the planned net reward over the planned net risk.

**The required test: a stop so small that charges decide the ratio.** One function of (D_tgt, D_stop, qty, charges), with these exact vectors, each worked by hand and by BigInt from `costs-evidence/calc/vec.js` at 2026-09-09. A second test asserts that the `charges` it is given equal `TwoClock::headline` on that round trip; `part-FNO-COSTS.md` documents a 1-paisa DP-GST rounding difference from `vec.js`, so these vectors take charges as an input.

| Case | REF, amount | qty | D_stop | D_tgt | charges | Old reading (price only) | S-RR | Shows |
|---|---|---:|---:|---:|---:|---:|---:|---|
| Charges decide, passes nothing | Rs 1,000.00, Rs 10 lakh | 1,000 | 10 | 50 | 41,677 | 5,000 | 161 | "at least 5" on price; below every seed on net |
| Reward below charges | Rs 1,000.00, Rs 10 lakh | 1,000 | 10 | 20 | 41,677 | 2,000 | -420 | floor: truncation would give -419 |
| Same record, smaller amount | Rs 1,000.00, Rs 2.5 lakh | 250 | 10 | 50 | 15,167 | 5,000 | -151 | one record, another amount, another value |
| The normal vector of SNIPER.md 1.2 | Rs 1,000.20, Rs 10 lakh | 999 | 306 | 605 | 41,671 | 1,977 | 1,619 | charges still move a normal stop |
| Target behind | Rs 1,000.20, Rs 10 lakh | 999 | 306 | 0 | 41,671 | 0 | -120 | a real negative value, never absent |
| Wide stop | Rs 1,000.00, Rs 10 lakh | 1,000 | 2,500 | 12,500 | 41,677 | 5,000 | 4,901 | charges matter less and still fail "at least 5" |

The usual cases apply (8+ extremes, 2 combinations, 1 out of the box; requirement 17): `qty = 0`; `D_stop = 1`; charges 0 equals the old reading; the dated and today clocks swapping which is worse; the shuffle-the-future test; flat p50 and p99 benches.

**Notes to the builders are in `build/NOTES-shared.md`, 10:5x lines: F2, F3, F4 and F5.** The new seal must cover this section too.

### 24.V Charges as real Dhan (operator, Sat 7 Nov 2026 ~10:5x IST; requirement 26; COSTS.md addendum)
Every headline figure uses the official reading of the charge calculator (`P`, Rs 20 per order, no DP charge on a squared day), on the worse of the two clocks. The worst reading `W` (per-fill brokerage, DP) prints only as a labelled stress line and decides nothing. This overrides 11.3 and every line that names the worst reading as the headline.

### 24.P The main plan: high probability, few trades (operator, Sat 7 Nov 2026; before any run)

The operator, Sat 7 Nov 2026 ~10:4x IST, gave the main plan (REQ 24) and then, told of the tensions and that the main session had made the sniper chain the headline, said "Let the data decide this" (REQ 24a). These are the main session's rulings on it, written about 10:5x IST by a workflow step of main session d4256142. **Data-blind: no result, trades file or search output was read, and this step ran nothing on market data; by file names only, no report exists yet.** Every figure is a labelled seed or labelled arithmetic (`PV/combos/hp-gate-arith.mjs`, its `.md` and `.json`; node BigInt). The builders' source was read only, for names. This block is byte-equal at the end of `PV/PREVIEW-SPEC.md` and `PV/SNIPER.md`. No earlier line was edited; these rows win where they differ. **No combination count moves: 24.9 stands and COMBINATIONS.md is unchanged.** The new seal must cover this section.

**In plain words.** One headline, picked each year by the same fixed ranking from every recipe tried, sniper recipes included, reading earlier months only. So the data says each year what kind of recipe wins, and every report shows what kind it was. The sniper chain now also has to clear a win-rate test on its learning months, and it prints beside the headline without ever deciding line 2. Every shown recipe prints its win rate with a range, so a high rate on few trades shows how little it may mean.

#### R1. OP-P13, the passer test: ruled

14.2 rule 1 reads: **every month of the range has trades and nets at least the fixed Rs 2,50,000 (REQ 1a).** It reads learning months when choosing and unseen months when judging, so verdict rules 4 and 6 follow through THE RANKING. A series is a passer exactly when its shortfall is 0 and it traded in every month of the range. The 12.5%-of-start-capital reading prints beside, as months at that reading, on learning and unseen months, and decides nothing.

| Earlier line | Now |
|---|---|
| 14.2 rule 1; 14.5 rules 4 and 6; 16.4's "at the bar" column | the fixed floor; the month table gains "at 12.5% of start capital", beside |
| 24.10's last paragraph; SNIPER.md section 9 point 6 | closed by this ruling |
| SNIPER.md 2.2 step 1 | follows, as it said it would |
| 9.3's seeds "Rs 2,50,000 and 12.5%" | stay as two searchable lock values; "never below the bar" reads the fixed floor. A dial, not the bar |
| 15.1's M0, OP-P1, DV-24 | stand: Variant K prints beside and decides nothing |

#### R2. The headline: let the data decide

**Line 2 stays with one chain: GLOBAL, each window's #1 by THE RANKING, chosen on learning months only, over every series evaluated.** No chain is favoured by hand. There is no switch: SNIPER.md 2.3's switch and 24.2's sentence on it are closed, and REQ 24a replaced REQ 24's ruling, so 14.2's procedure never changed. Each test year the data decides, on earlier months only, whether a sniper recipe or any other recipe wins. It is one pre-registered headline, not two: no kinder door.

**What stays hand-made, said plainly:** THE RANKING itself, fixed before any run. Among passers the lowest max drawdown decides first. Among misses the least shortfall decides, then the highest worst month, so drawdown decides only exact ties there and more net can beat fewer trades. If no recipe passes on learning months, the headline may be a many-trades recipe. The year table below shows that the moment it happens.

**Does the union really hold every cell? Checked: at the series level yes, after stage 2 not yet.**

- **At the series level it holds, in the rule and in the source.** GLOBAL takes the best of every series offered, and a SNIPER or high-probability cell is a subset of those. `wf::Heaps::offer_series` and `fold_shard` keep GLOBAL exact (a global top 5 sits inside the N lists' top 5s), and both offer sites, `search/shard.rs` and `search/adaptive.rs`, offer each series to the lists, the SNIPER table and the luck stream side by side (read at about 11:00 IST, mid-build).
- **The words were only implied for stage 2.** 9.3 says every evaluation goes to the heaps, SNIPER.md section 2 says a cell's candidates go through stage 2 "like every scope's", and no line says a cell's stage 2 variant reaches GLOBAL.
- **After stage 2 the source cannot yet carry it.** `search::adaptive::apply_stage2` and `stage2::run_stage2` are still stubs, and `wave_out` forms the chains before the seam. `wf::Rid` holds N and the exit but no admission, tie-break or controls, so a stage 2 variant has no id in GLOBAL's list.

**So it is wired here:**

1. Every evaluated series, `W2.sniper.*` and stage 2 included, goes to the lists, the SNIPER table and the luck stream together. Nothing offers to one alone.
2. After each stage 2, window Y's GLOBAL pick is the best by THE RANKING over GLOBAL's list and every stage 2 variant of every candidate of the 39 scopes and the 12 SNIPER cells, the high-probability candidates of R4 included. Pair and mix evaluations are offered the same way (R3:352).
3. Every stage 2 series gets a stable id, whether an added `Rid` field or a per-window list compared by the same key. Two variants differing only in controls are two series, and a pair or mix holds two recipes, which one `Candidate` cannot name.
4. The twin builds its GLOBAL the same way, or verdict rule 4 compares unlike chains.

**SNIPER prints beside it and never writes line 2.** It has its own labelled line under line 2's answer in part 2. `report::verdict` reads `Scope::Global`'s chain only, and `VerdictInputs` gains no SNIPER field.

**Line 3 (a ruling that needs one reconciling).** Lines 1-3 are fixed, and line 3 is the survivorship label, word for word (SPEC.md:7; this file's head; INTERFACES section 7's "3-line head"). The ruling asks for one sentence on line 3. So the head stays three lines, the label stays first and unchanged, and when SNIPER holds line 2's floor in all 82 unseen months and the headline does not, line 3 reads:

`current F&O members only (survivorship bias not removed). THE SNIPER CHAIN MET THE FIXED RS 2,50,000 IN ALL 82 UNSEEN MONTHS AND THE HEADLINE DID NOT (<x> OF 82); LINE 2 READS THE HEADLINE, AND THIS IS NOT A PASS`

Interim reports put "interim after wave k: " before "THE SNIPER CHAIN". The floor's words come from line 2's floor function (24.10). In every other case line 3 is the label alone. 2.3's fuller sentences stay in parts 2 and 6.

**The year table, in every report, interims included, in part 2: what kind of recipe the data picked.** One row per test year for GLOBAL, one beside it for SNIPER. It prints no dial settings, so REQ item 6's limit does not reach it. Learning figures read what the choice saw; that year's are what then happened.

| Column | Exactly |
|---|---|
| Trades a month | learning / that year |
| Stop size | median S-STOP in bp at the decision, at the walk's own stop (`gen::sniper::distances`), learning / that year, absent ones counted beside; labelled "price distance, not P&L" (24.13) |
| Net win rate | with its 95% range (R5), learning / that year |
| SNIPER cell | the tightest cell holding the pick on its learning months, by 24.2's test at its N seed's edges (a stage 2 variant reads its parent's), or "no cell" |
| Gate on the recipe | its S-STOP and S-RR clauses, or "none" (closes check 2's nonblocking 5) |
| Clears | the highest seed of R4 it clears on learning months, or "none" |
| Also SNIPER's pick | yes or no; a window with no pick says so |

Under it: "the data picked a recipe in a SNIPER cell in x of 7 test years."

#### R3. The two points left open in sniper-repair2.md: closed

- **Every print of any twin's chain from less than the whole pipeline carries the partial-null words.** 24.11's last-but-one row named SNIPER chains; it now covers every chain: GLOBAL, the other 38 scopes and the SNIPER family, at 14.4's per-seed line and in 16.6, 16.7 and 16.8. From waves 0-1 the words are 24.11's: "partial null: twin seed 1 only, waves 0-1 only, 2,278,875,840 of the real 145,276,706,220 options; a smaller search finds less luck, so beating it is easier than 14.4's twin, and it does not clear the chain". The figures print from achieved counts.
- **"It does not clear" binds the verdict too.** Rule 4's "below the headline" counts only from a seed whose whole pipeline ran. A partial twin at or above the headline still gives NO. A partial twin below it clears nothing. So `twin1_below` is `Some(true)` only from a whole pipeline.
- **The second twin runs: it costs only CPU, and it writes nothing.** It fires in 2.3's branch, on seed 1's draw, running the seven `W2.sniper.*` stages with S-STOP and S-RR left with their records, merged into a copy of seed 1's lists after waves 0-1 (which read no gate column). The branch is **about 10.5 model hours, not 5.2** (5.24 for each twin, ESTIMATE); this replaces 2.3's figure, and both lines print the partial-null words.
- **It also runs when the headline meets line 2's floor,** because a headline can now hold a gate, and 2.3's caution (a small stop gives small losses whatever the entry) then applies to it. There it runs its own `W2.sniper.*`, stage 2 and adaptive rows and shares seed 1's other stages, so its whole pipeline has run and the words drop. It prints beside and is not a verdict rule.
- **For the builders (check 1's nonblocking 7, check 3's nonblocking 4):** the per-walk S-STOP and S-RR arrays are computed on real records. The ordinary twin then permutes them with the other columns, and the second twin does not. Otherwise the two twins come out the same in code, so a test proves they differ on a fixture.

#### R4. The high-probability gate

A selection rule on learning months only, no new evaluations. A series clears seed Zsq in window Y when, on Y's learning months:

`W > L and (W - L)^2 x 1000 >= Zsq x (W + L)`

- **W and L** are net wins and net losses: a win is a trade whose net after its own charges is above 0, a loss at or below 0 (REQ 21a). Both are already kept: `Key.wins`, and L = `Key.trades` - `Key.wins`.
- **Exact integers, O(1), `u128`:** no overflow at `u32::MAX`. W <= L fails first, so an unsigned square cannot pass a losing series. With 0 trades it does not clear.
- **Seeds, labelled "seed (main session for REQ 24's confidence test; the operator gave no number)":** Zsq 2706, 5412, 9548, a one-sided sign test against a coin on the normal reading, at z = 1.645, 2.326, 3.09. Exactly, 5412 holds z = 2.3264; 2.326 squared would be 5410. The seed stands as ruled and prints so.
- **Boundaries:** fewest wins clearing with no loss: 3, 6, 10. Equality clears: W 677,853, L 675,147 clears 5412 and W 677,852, L 675,148 does not. All 34 vectors are in `combos/hp-gate-arith.json`.

**The SNIPER primary chain requires Zsq 5412 on top of its cap and band.** 2.2's walk now reads only series clearing 5412 on that window's learning months, over the same 12 cells, tightest first, an empty cell passed over. The edges keep 24.2's population, every series at or under the cap, so the gate does not move a band.

1. The #1 of the first cell holding a learning-month passer that clears.
2. Else the first cell holding a series that clears with a trade in every learning month; the best such series by the miss key.
3. Else the loosest cell holding a series that clears, walking back from cap 20 p75; its #1 by the miss key. Its window prints "no series in any cell cleared the gate and traded in every learning month: not a sniper pick".
4. Else no pick, `sn_no_cell`, printed "no series in any cell cleared the gate". That window's unseen months are no-trade months, and they fail the bar. Few trades plus a stated test can leave the chain with nothing, and the report says so.

**The other 12 cells do not gain the seeds: the count would not stay small.** 12 x 3 is 36 more chains, 88 in all and 49 in the SNIPER family, nearly four times as many SNIPER chains to look at once the unseen months are in, and up to 84 more list checks on a series at cap 2. **Instead, three chains, `SNIPER-HP-2706`, `SNIPER-HP-5412`, `SNIPER-HP-9548`:** each window's #1 by THE RANKING among series with at most 20 trades a month on learning months, clearing that seed, with no loss band. They give the other two seeds the one use that measures them: TPM20 is the same population without the test, so the report shows what a stricter test does ("searched both ways", REQ 24a). They hold no stage 2 candidates of their own, so no count moves, and any stage 2 variant that qualifies enters them. Staying at or under cap 20 keeps 24.10's bound proof true as written.

**The exact chain count: 55, against 52 before.** `WaveOut.chains` holds at most 55, and each report prints "chains formed: x of 55". This holds until the choice 24.U leaves open, named below the table.

| Chains | Count |
|---|---:|
| GLOBAL, the headline | 1 |
| Family (E1C, E2C, E1S, E2S) | 4 |
| Timeframe | 8 |
| Side (the 6 of 23.5) | 6 |
| N seed | 5 |
| FEWEST | 1 |
| TPM10, TPM20, TPM40 | 3 |
| LOWEST-DD | 1 |
| Touch-number rows (5 rows x long, short) | 10 |
| **The 39 scopes (`wf::N_SCOPES`, `SCOPES` of count.mjs)** | **39** |
| SNIPER cells `SNIPER-c-pb` (no gate) | 12 |
| SNIPER, the primary chain (gate 5412) | 1 |
| `SNIPER-HP-z` (gate only) | 3 |
| **The SNIPER family** | **16** |
| **All chains** | **55** |

**24.U's caps 40, 60, 100 are not taken here.** 24.U adds them only if the chain count stays small, and moves no count unless the final pass says so. They would be 9 more cells: 64 chains, 25 in the SNIPER family, and 315 more stage 2 candidates, so 7,526,295 more at most in stage 2 (33,913,341 becomes 41,439,636, the expected total 146,287,035,456, and COMBINATIONS.md changes; the guaranteed part does not). The walk would then run 21 cells, tightest first, with the gate on every step. Against that, cap 20 asks 0.833% net a trade and cap 100 asks 0.167% at Rs 15 lakh (REQ 25), so the caps of 24.2 sit far from the bar (check 3's nonblocking 6). That choice is the final pass's.

**It counts in the luck test as 24.2 does.** The gate adds no series. Every series is already in `tried_total` and the one luck stream (14.3), the 16 chains' O print against the same L95, and "chosen among N chains" reads the chains formed. The twin runs the same gate and walk.

**Stage 2 stays inside the 420.** A cell's at most 5 candidates are, in this order and without repeats: the cell's #1; its #1 clearing 5412; its best clearing 5412 with a trade in every learning month; its every-month slot; then the next by THE RANKING. This replaces "top 5, or top 4 and its slot" (24.10, SNIPER.md 2.2). Which variants run can change, inside the same "at most".

**A stage 2 variant in a cell (closes check 3's blocking 2, check 2's nonblocking 1):** it stays in its parent's cell and reads that cell's edge from the wave's end at the parent's N seed, leaves if its own learning trades pass the cap or its largest loss passes the edge, and clears the gate or not on its own W and L.

**What the gate does not show, printed wherever it is named:** "Clearing it on learning months is a selection rule, not proof. Among this many options many series clear it by luck, and it tests against a coin, not against a win rate that pays. The luck test and the twin judge that." Two limits of the normal reading, measured in `combos/hp-gate-arith.md`:

- **At tiny counts it is kinder than its z says.** 3 wins and no loss clears 2706, and a coin does that 12.5% of the time, not 5%; 6 and none clears 5412 at 1.56%, not 1%. The formula stands as ruled, and R5's exact range prints beside it (3 of 3: 29.2% to 100%).
- **It asks less of more trades.** On 16 learning months at 2 trades a month, 5412 asks 71.9%; on 88 months at 40 a month, 52.0%.

#### R5. The table

The SNIPER table (24.4, SNIPER.md section 4), every shown recipe (16.4), the closest misses (16.7) and the headline and SNIPER rows also print these, from each trade's net after its charges. Unseen months, with the chain's learning months beside. **Net only (24.12).**

| Column | Exactly |
|---|---|
| Net win rate | W / (W + L); "no trades" at 0 |
| Its 95% range | the exact binomial range (Clopper-Pearson) from W and L on integer thousandths, so it rounds outward: lower = the largest t with P(X >= W at t/1000) <= 1/40, 0 when W = 0; upper = the smallest t with P(X <= W at t/1000) <= 1/40, 1000 when L = 0. Class C, display only |
| Profit factor on net | sum of winning nets / (size of the sum of losing nets + the range's data charges), so it is above 1 exactly when net is above 0; "no losing trade" beside when L = 0 |
| Average net win | sum of winning nets / W, Rs and % of the deployed amount |
| Average net loss | size of the sum of losing nets / L, Rs and %; "no loss" at L = 0. The month's data charge is not in it (closes check 1's nonblocking 6) |
| Payoff ratio | average net win / average net loss, rounded down; "no loss" at L = 0 |
| Trades a month | trades / months of the range, with the most in one month (never above 100, 24.U) and the months at that cap |
| Trades a day | trades / sessions of the range, with the most in one day (never above 5, 24.U) and the days at that cap |
| Net Rs; net % | the sum of the months' nets; % of Rs 20 lakh, one fixed base |
| The gate | per window, the highest seed the pick clears on learning months; beside it, the same test on the chain's unseen months, which never decides |

**Why exact and not a shorter formula:** the range exists for few trades, and that is where the short ones flatter. Over all counts to 300 trades, plain Wilson is up to 18.4 points narrower than exact (2 of 2: 34.2% against 15.8%), and Wilson with continuity correction up to 4.0 points. Exact is not O(1), so this is named plainly (REQ 8): about 20 tail sums of at most n terms for each printed row, stopping when terms vanish. It runs only on printed rows, never in the search. Vectors: `combos/hp-gate-arith.json`. Printed under the table: "The range treats trades as independent. Trades on one day often move together, so the true range may be wider."

**Closed with the table (check 3's blocking 3 and 4, check 2's nonblocking 2 and 3):** 60 s and 300 s are class C, display only, and select nothing. When `stop_dist` is at or below 0, "multiples of the stop" prints "through at fill" for that trade, it enters no average of multiples, and each such average prints beside it "leaves out n trades already through the stop at the first fill". A test proves no zero or negative divisor.

#### R6. The plain tensions, printed under the table, every table

"A tiny stop lowers the win rate, because noise hits it; a wide stop raises the win rate and the size of each loss. Very few trades leave months with no trade, and those months fail the bar. A high win rate on few trades can be luck. The search measures these and assumes nothing."

These sit beside 24.5 and SNIPER.md 5.3, which stay.

#### What this overrides, and what stays

| Earlier line | Now | By |
|---|---|---|
| 14.2's headline procedure, line 412 | "over every family, timeframe, side and N" means every series evaluated, as wired above | R2 |
| 14.2's scopes, line 411; 24.7's row "SNIPER's 13 chains"; SNIPER.md section 3's "the 13 chains"; 2.3's "1 of 13 SNIPER chains and about 52 chains" | 16 SNIPER chains, 55 in all | R4 |
| 24.2's and 24.10's walk; SNIPER.md 2.2; `Scope::SniperPrimary` | the gate on every step | R4 |
| SNIPER.md 2.3 "What it shows" | "chose recipes inside its cap and largest-loss band that cleared the high-probability gate at Zsq 5412" | R4 |
| 24.2's and 2.3's switch; check 1's 16 and check 2's 6 (the headline question) | closed: the operator answered | R2 |
| The head's line 3; 16.1; the head test | as above; the test reads `starts_with` the label | R2 |
| 16.2 | gains SNIPER's line and the year table | R2 |
| 24.11's row on prints; 2.3's "about 5.2 model hours"; repair 2's two points | as R3 | R3 |
| 24.4, 16.4, 16.6, 16.7 | gain R5's columns | R5 |
| 24.5, 16.6 | gain R6's words | R6 |
| 21 | requirements 24 and 24a: this section | REQ 24a |
| SNIPER.md section 7's and COMBINATIONS.md:244's "at most 28 heap checks" | at most 28 list checks, up to 2 more comparisons on each, and 21 for the three chains: still O(1). The 3-10% stays an unmeasured guess, and the five-stock test prices it | R4 |

**What stays:** 24.9, every stage, band and seed; the 420; the luck masks; 14.5 apart from 24.R's floor and R1; and the headline and SNIPER rows printing always (24.10). **COMBINATIONS.md is unchanged (3ee9a5fcd4ddc7ac): no combination count changes,** because the gate is a selection rule, the three chains hold no stage 2 row, and `S2.shown` is fixed at 12 chains. Its "28 heap checks" is a cost note, amended above.

**Open after this:** the independent check of this section; **check 3's blocking 1, which this section does not close: the plain check is still at Rs 1 crore, and its re-read now means Rs 15 lakh and the official charges (24.U, 24.V)**; the choice on 24.U's caps; the new seal; the floor question to the operator with table D (R1 rests on 24.R, which he can overrule); round 3's own line, since R3:603 is outside this folder. Nothing here waits on the operator before a run.

### 24.W ORB on every timeframe (operator, Sat 7 Nov 2026 ~11:1x IST; requirement 27; the final plan pass defines and counts it before any run)
Until now the plan held the opening range only as two position bands (15 and 30 minutes, COMBINATIONS.md "Ladder anchors"). New: opening-range durations 1, 2, 3, 5, 10, 15, 30, 60 minutes. Family E3 (E3C on the close beyond the range, E3S on the first second beyond), a third entry family on the top-mover pools, stop just beyond the range or the seeds of 12.3, exits as the other families; a decision only after its range has closed, never inside it. Filter dial ORB-POS on E1 and E2: above, inside, below each range, plus generated distance bands. New stages run after their E1 and E2 mirrors, so no earlier count moves. Built as stage F2b after F2 and F4 return.

### 24.X One strategy, every way in (operator, Sat 7 Nov 2026 ~11:1x IST; requirement 28; the final plan pass defines and counts it before any run)
The plan holds E1 flag, E2 touches (clause = 1, = 2, = 3, >= 2, >= 3) and E3 ORB as families in one union search: that part stands. Gaps closed here: (a) a PATTERN-AT-TOUCH group: named candlestick patterns (W/state/fib-run-20261016 patterns amendment and SPEC section 7), anatomy and candle codes on the touch candle and the one before it, crossed with the touch clauses, as stages with their own exact counts; (b) family EU, the union recipe: entries of two or more kinds on one account, first signal wins, one position per stock, the day and month ceilings shared; stage 2's mix stays beside it.

### 24.Y What the data check means for this tool (state/proof-20261107/data/RECONCILE.md and DATA-EVIDENCE.md, Sat 7 Nov 2026; check 1 FAILED on 10 wording and class items, repair owed; the final plan pass reads them, data quality only, no result)
Candles come from Zerodha minutes, fills from GDFL seconds (3.3). Zerodha is back-adjusted on about 36% of stock-days; a plain ratio leaves percent readings right, a ratio plus a rupee offset (about 25,661 stock-days on 32 names; IOC Jan 2023: 1% reads about 1.23%) leaves percent readings, ranks, ladders in rupees and stop distances wrong. Owed in the plan: (a) every offset span named and either refused or read from raw prices, with the count printed; (b) a labelled lane on shown chains: signals re-derived from candles folded from GDFL seconds, printing how many trades and how much net change; (c) rank moves against the raw previous close where a raw source exists; (d) 2024-07-15 and 2024-07-16 (half captured, in no tool's labels) refused by name; (e) TATACONSUM read through its old key TATAGLOBAL (365 stock-days) and the 6 names' gaps named; (f) every report prints that 0 of 17 data fixes are on the store and AUDIT-3 has not run on it.

### 24.Z Ceiling B beside the main ceilings (operator's question, Sat 7 Nov 2026 ~11:2x IST; requirement 29; counted in the final plan pass)
Account ceilings are data, not constants: (day cap, month cap, amount). Main: (5, 100, Rs 15 lakh). Ceiling B: (10, 100, Rs 10 lakh), run on waves 0 and 1 and on every shown chain, as a labelled account that never writes line 2. Rs 10 lakh is already a walked amount, so phase A does not grow. Its series count in tried_total. Estimate: about 9 more model hours (wave 1's size) until the test run measures it.

### 24.AA Three accounts (operator, Sat 7 Nov 2026 ~11:3x IST; requirement 30; the final plan pass defines and counts it before any run)
(A) INDEX: E1, E2, E3 and EU on the NIFTY and BANKNIFTY spot on every timeframe, the top-mover clause off because two names cannot rank (labelled), each signal traded in that index's option under the option rules (current expiry, top volume, then moneyness, then price; at most Rs 2,00,000 premium, whole lots), reusing the fills and charges of the index-options tool; run first. (B) STOCKS: this preview as specified. (C) BOTH: one account across (A) and (B), the ceilings shared. Each is its own walk-forward chain, net Rs and net % of Rs 20 lakh, side by side; line 2 stays one pre-registered reading (24a's rule over the union of scopes; the final pass states how the three accounts enter it without opening a kinder door). A new stage F6 builds (A) and (C) after F5.

### 24.P1 Repair of 24.P after its independent check, round 1 (Sat 7 Nov 2026 ~11:3x IST; before any run; no earlier line was edited; these rows win where they differ)

`PV/highprob-check1.md` found 7 blocking points in the words of 24.P, none in a number. `PV/highprob-repair1.md` lists each fix and every place this block goes past the check's words. **Data-blind: no result, trades file or search output was read, and nothing ran on market data. No count, stage, seed, charge or estimate figure moves: 24.9 and COMBINATIONS.md stand, the chains stay 55, and `combos/hp-gate-arith.*` is not regenerated.** This block is byte-equal at the end of `PV/PREVIEW-SPEC.md` and `PV/SNIPER.md`, and in `PV/combos/section-24-P1.md`. The builders' lines are in `build/NOTES-shared.md`, the 24.P1 lines. The new seal must cover this block.

**In plain words.** A recipe that exists only because it did well up to 2022 can no longer reach the 2020 pick. A comparison lane can no longer be picked at all. Every variant has one rule for its SNIPER cell, judged against positions no bigger than its own. A year with no wins, no losses or no trades prints a word, never a division by zero.

#### Fix 1. Reach: no look-ahead through what exists (R2 items 1-2; 14.1's search reach; R3:495 as this preview reads it)

"A stage 2 variant, pair or mix of a candidate of window Y is offered only to windows from Y on, with or without a generated number; a candidate in several windows counts from the earliest. R3:495's every-window offer does not reach it. The same holds for the cells and the three chains."

That line reads with these four:

- **A pair or mix** counts from the earliest window that formed it, the first whose own candidates held both its recipes. One recipe's earlier window does not widen it: a pair formed only in Y2023 is not offered to Y2020 because one of its recipes was also a Y2020 candidate.
- **14.1 still binds on top.** A number generated on window Z keeps the series out of every window before Z, whatever its parent's earliest window.
- **The adaptive rows are the same case** (`W2.fine`, `W2.stacks`, `W2.sniper.stacks`). A series drilled from window Y's top nodes is offered only from Y on, with or without a generated number, and the same series drilled from several windows counts from the earliest. So it also stays out of earlier windows' largest-loss histograms.
- **Only the offer to the lists, the cells and the three chains is limited.** The luck stream and `tried_total` still take every one (14.3), because they choose nothing.

R2 item 2 now reads "over GLOBAL's list and every such series offered to window Y". The twin does the same. A fixture proves it: a Y2023 candidate's variant that would beat Y2020's pick on Y2020's learning months leaves Y2020's pick unchanged.

Why: such a series exists because of 2020-2022, which are Y2020's unseen months. Perturb them and Y2020's pick can change, so MIX-WF-1 (14.1) fails.

#### Fix 2. Comparison lanes stay out of every pick (R2 item 1; 11.2; 14.3)

R2 item 1 gains: "except `S2.shown` and every other labelled comparison lane: those go to the luck stream alone (14.3, 11.2) and enter no list, cell or SNIPER-HP chain." "Nothing offers to one alone" holds for every other series.

- **The lanes:** 11.2's list, the no-stop lane and `S2.shown`'s seeded orders. 24.Z's ceiling B keeps its own labelled chains and enters none of the main account's lists, cells or SNIPER-HP chains. 24.AA's accounts stay the final pass's to state.
- **Not a lane:** the month caps 2 and 5 of `S2.sniper.controls`, though SNIPER.md:341 calls them "a labelled lane". They are searched seed values (9.3) and stay in the lists and cells.

Test: a comparison series that beats everything changes no pick.

Why: a chain is shown when it nets +100% on its unseen months (16.3), so which lanes run reads unseen months, and 25% of volume or one-second fills would be a kinder lane into the headline.

#### Fix 3. One rule for a variant in the SNIPER table (R4's "A stage 2 variant in a cell"; 24.2; SNIPER.md 2.1; the year table's "SNIPER cell")

This replaces R4's paragraph:

**A stage 2 variant of one recipe, of a scope's candidate or a cell's, enters the table by 2.1's own test: on its own learning trades and largest loss, against the wave's-end edges, at its own N when that is an N seed, else at the smallest N seed above its own N; it adds to no histogram.** It sits in every cell it passes, so a cell's variant stays in its parent's cell exactly while it passes that cell's test. With no edge at that cap and N seed, that cell does not hold it. It clears the gate or not on its own W and L.

**A pair or mix enters no cell unless it came from `S2.sniper.pair` or `S2.sniper.mix`, where it stays in that row's cell by the same test; elsewhere the year table prints "pair" or "mix" and no cell.** The same test reads its own learning trades and largest loss at its own N seed. The three chains have no cell and no band, so any variant, pair or mix enters them on its own learning trades, W and L, as R4 says.

The year table's "SNIPER cell" reads "the tightest cell holding the pick by this test; a pair or mix as above; or 'no cell'", and "the data picked a recipe in a SNIPER cell" counts the rows that print a cell.

**One point departs from the check's words and from R4: "at its parent's N seed".** `S2.money` runs a candidate at all 13 N values, so a parent at N = 1 has a variant at N = 20. A Rs 5 lakh position read against Rs 15 lakh's edges lands in a tight cell by size alone, and the walk goes tightest first. 2.1 forbids that ("small for its own size"). A seed's own amount is never above the variant's (labelled arithmetic from 24.U's formula, not a result), so this is never kinder:

| N | Its amount | Reads N seed | That seed's amount |
|---:|---:|---:|---:|
| 1, 3, 5, 10, 20 | Rs 15, 15, 15, 10, 5 lakh | its own | the same |
| 2 | Rs 15 lakh | 3 | Rs 15 lakh |
| 4 | Rs 15 lakh | 5 | Rs 15 lakh |
| 6 | Rs 15 lakh | 10 | Rs 10 lakh |
| 7 | Rs 14.29 lakh | 10 | Rs 10 lakh |
| 8 | Rs 12.5 lakh | 10 | Rs 10 lakh |
| 12 | Rs 8.33 lakh | 20 | Rs 5 lakh |
| 15 | Rs 6.67 lakh | 20 | Rs 5 lakh |
| 17 | Rs 5.88 lakh | 20 | Rs 5 lakh |

#### Fix 4. The year table prints two gate clauses, and says so (R2; REQ item 6; 24.10's row 24.4, 16.3)

Check 1 left two choices to the main session. This block takes (a), the one it recommended, which closes check 2's nonblocking 5.

- R2's sentence reads: "It prints no dial settings but the two gate clauses."
- Row "Gate on the recipe" reads: its S-STOP and S-RR clauses in words, each with its class label ("generated" or the seed's label), both recipes' for a pair or mix, or "none".
- It prints below +100% as well. That is one narrow exception to REQ item 6's one-line rule: this table and those two clauses, no other setting and no per-month table.

#### Fix 5. R5 never divides by zero

| Column | Adds to R5 |
|---|---|
| Its 95% range | "no trades" at no trades, and no figure |
| Profit factor on net | "no trades" at no trades. With trades and a divisor of 0 (no net on any loss and no data charge in the range): "no net loss" and no ratio. "no losing trade" beside at L = 0 stays |
| Average net win | "no win" at W = 0, in Rs and in % |
| Payoff ratio | "no trades" at W = L = 0; "no win" at W = 0; "no loss" at L = 0; "no net loss" when W > 0, L > 0 and every loss nets exactly 0, since a loss is net at or below 0 (REQ 21a) and the average net loss is then 0 |
| Trades a month; trades a day | "no months" and "no sessions" if the range holds none |
| The year table and 24.4's averages and shares | "no trades" for any figure that reads trades in a range with none, the median stop size included |

R5's test now covers every ratio and average of R5's table, the year table and 24.4, at W = 0, at L = 0, at no trades, and with every loss at net exactly 0.

#### Fix 6. "What stays" follows R3 (14.5 rule 4)

"What stays" reads "14.5 apart from 24.R's floor, R1 and R3's reading of rule 4"; the rest of that sentence stands.

#### Fix 7. The gate's function (R4; `build/NOTES-shared.md` P-02)

`clears(wins: u32, losses: u32, zsq: u32)`, with W + L taken in `u64` inside; the caller passes `key.trades - key.wins`. R4's words and the 34 vectors stand, and no file is regenerated. Why: (4294967295, 4294967294) has W + L = 8,589,934,589, which does not fit `trades: u32`.

#### What this block adds to 24.P's override table

| Earlier line | Now | By |
|---|---|---|
| 14.1's search reach, line 405; R3:495 as this preview reads it | Fix 1 | R2 |
| 14.5 rule 4; 24.7's row 14.5 | "below the headline" counts only from a seed whose whole pipeline ran | R3 |
| 24.10's row 24.4, 16.3 | the year table prints below +100% (REQ 24a) | R2 |
| R4's "A stage 2 variant in a cell"; the year table's "a stage 2 variant reads its parent's"; check 3's blocking 2's "at the parent's N seed" | Fix 3 | R4 |
| 11.2, 14.3 against R2 item 1 | Fix 2: 11.2 and 14.3 win | R2 |

**Open after this:** the independent check of this block; the main session's word on the two choices named above (Fix 3's N seed, Fix 4's (a)); R3:495, which holds the same hole for round 3 and is outside this folder; check 1's 19 nonblocking points, none repaired here; and everything 24.P's own "Open after this" names. Nothing here waits on the operator before a run.

## 25. THE FINAL PLAN (Sat 7 Nov 2026, before any run)

Written Sat 7 Nov 2026, about 12:40-13:40 IST, by a workflow step of main session d4256142. **This is the one place a builder or a checker reads first.** No earlier line of this file was edited. Where this section differs from an earlier line of this file, `SNIPER.md`, `build/INTERFACES.md` or `build/NOTES-shared.md`, this section wins; 25.0 names what it overrides. `COMBINATIONS.md` and `combos/*` were regenerated with it (backups `*.bak-final`).

**Data-blind.** Nothing here was chosen from a result. Every figure is a labelled seed, a rule that generates a value on learning months, or labelled arithmetic (`combos/count.mjs`, `combos/sniper-arith.mjs`; node BigInt). Nothing ran on market data, `test-run/` was not opened, and `logs/FULL-RUN-OK` does not exist. **Two slips, named so a checker can weigh them:** looking up the state of the two option tools, I read `index-options-preview-20261106/check2.md`'s blocking 1, which quotes one old trade's rupees, and a `tail` of that folder's `progress.md` printed its run 4 headline line. Both are old tools' figures, on other recipes and without today's ceilings. The stage table was fixed and counted before I saw them, and no figure or rule here reads them.

**In plain words.** The operator's top-mover Fibonacci strategy stays. It gains the opening range as a third way in and a filter, patterns at the touch, one recipe that takes any way in, a second ceiling beside the first, and three accounts: indices in options, stocks in cash, both together. The indices run first. One fixed ranking, on earlier months only, picks each year's recipe from all three accounts, so the data decides what kind of recipe wins and which account it trades. The highest net chain prints on every report. Data rules found today bind by name, each with its owner. Every open finding is closed below or given an owner. The search grows to about 196 billion combinations, and one named cut keeps it under 100 model hours.

### 25.0 What this overrides

| Earlier line | Now | Where |
|---|---|---|
| 10.1, 10.2, 24.S, 24.T, 24.U's numbers | the ceilings as data; no equity term in the size step | 25.1 |
| 10.2's "Leverage flat 5x", DV-09 | dated leverage from C_VAR1 | 25.3 D5 |
| 3.1's `Membership::is_member`, 3.3's security-file line, 11.1's bands and ticks rows | the official dated list; the file before the session | 25.3 D6, D8 |
| 3.3's "Zerodha 1-minute cash candles" on offset spans; 5.1's C on those days; `col::PRICE`, `col::LIQ_LAKH` | read raw | 25.3 D3, D4 |
| 3.4 | gains 2024-07-15 and 2024-07-16; "half captured" (24.Y) reads "half the rows and seconds, all the shares" (data check 1, finding 10) | 25.3 D1 |
| 7, 7.3, 8, 12.2-12.4, 13.1, 13.2 | E3, ORB-POS, PATTERN-AT-TOUCH, EU; run order A, B, C | 25.2, 25.5 |
| 14.2's scopes; 24.P's "55 chains"; 24.U's caps 40, 60, 100 | 44 scopes + 16 SNIPER family on (B), 127 chains in all; the three caps are not taken | 25.2.8, 25.7 |
| 14.2's headline, 14.5's chain, `Scope::Global` in the verdict | line 2 reads GLOBAL-ALL | 25.2.6 |
| 14.4, 23.5, 24.11, 24.P R3 | the twin runs on all three accounts; the partial-null figures print from achieved counts | 25.2.6 |
| 16.1, 16.2, 16.6 | the three accounts side by side, MAX-NET, the data lines | 25.2.8, 25.3 D12 |
| 23, 24.9, 24.6's estimate | the numbers of 25.5 | 25.5 |
| 24.5, line 748, SNIPER.md section 5 and 2.1's reason | the plain check at Rs 15 lakh on the official reading | 25.4 |
| 24.AA's "E1, E2, E3 and EU" and "final pass states" | defined | 25.2.5, 25.2.6 |
| 17.3-17.8 | stages F1b, F2b, F3b, F4b, F5, F6, INTEGRATE | 25.6 |

### 25.1 The rules as they stand

Account ceilings are data, `Ceilings { day_cap, month_cap, amount, margin }`, never constants (24.Z, requirement 29). Fewer and smaller are searched, more never.

| Rule | Main, account (B) | Ceiling B, labelled | Account (A) | Account (C) | Source |
|---|---|---|---|---|---|
| Trades a session, across every strategy, timeframe and route | 5 | 10 | 5 (10 under ceiling B) | 5, shared by both universes | REQ 25, 29, 30 |
| Trades a calendar month | 100 | 100 | 100 | 100, shared | REQ 25, 29, 30 |
| Largest position | Rs 15,00,000, less where dated leverage is lower | Rs 10,00,000, less where dated leverage is lower | - | cash as main, options as (A) | REQ 25, 29 |
| Largest margin or premium a trade | Rs 3,00,000 margin at the stock's dated leverage | Rs 2,00,000 margin at the stock's dated leverage | Rs 2,00,000 premium, whole lots, rounded down; labelled seeds Rs 30,000, 50,000, 1 lakh, 2 lakh | as each route | REQ 25, 25a, 29 |
| Where it runs | every wave, stage, chain and twin | waves 0-1 and every shown chain; never writes line 2 | every wave, stage, chain and twin | its own stages (25.5) | REQ 29, 30 |
| Refusals, counted by name, no look-ahead | `day cap`, `month cap`, `margin_cap`, `exposure_cap`, `size_refused` | the same | the same, + `lot_above_premium` | the same | REQ 25; 24.U |

| Rule | As it stands | Source |
|---|---|---|
| The bar | the FIXED Rs 2,50,000 net in every unseen month, a no-trade month failing. +2.5% (Rs 50,000) and 12.5% of the carried month-start capital print beside and never decide | REQ 1, 1a; 24.R; 24.P R1 |
| Capital | Rs 20,00,000 of margin; at most Rs 1,00,00,000 of open position value; every account on the same Rs 20 lakh, never added together | REQ 4, 30 |
| The amount | **Ruling, confirming 24.U as written:** full step = min(account amount, Rs 1 crore / N), so N = 1, 3, 5 enter Rs 15 lakh, N = 10 Rs 10 lakh, N = 20 Rs 5 lakh (ceiling B: Rs 10, 10, 10, 10, 5 lakh). Quantised DOWN to the largest of the 5 walked amounts whose margin at that stock-day's leverage is at most the ceiling's margin; none: `margin_cap`. At the order's start, 10.3's margin and exposure checks may take it down one more walked step; below the half step `size_refused`. **10.2's "min(Rs 1 crore, closed equity x 5) / N" no longer scales the step:** read that way, the first month's Rs 588.82 data charge halved every position at N = 10 and 20 from exactly Rs 20 lakh (F4's 12:19 question), which no broker does. A smaller account meets the margin check instead | REQ 25; 24.U; 10.3 |
| Net only | every figure, ranking, table and field reads net in Rs and % of Rs 20 lakh. A win is net above 0, a loss net at or below 0. Charges only in a cost section, in Rs per trade and per month | REQ 21, 21a; 24.12; 24.13 N8 |
| Charges | as real Dhan: the official reading P (Rs 20 per order, no DP on a squared day), the dearer of the dated and today's rates. The worst reading W is one labelled stress line and decides nothing. Test figure: 547 x Rs 2,742 = Rs 579.41 in the passed calculator (COSTS.md's addendum says Rs 579.43; F3's 11:38 note), Dhan's estimator Rs 578.85 | REQ 26; 24.V |
| One route per signal | decisions always from the underlying's spot; one signal trades cash or its option, never both; cash and options are separate accounts on the same Rs 20 lakh | REQ 20 |
| No futures | not read, not searched, not a lane, not counted; a column derived from them is a defect | REQ 18 |
| Fills | lane S unchanged: 14 s, 10% of each second's traded volume, the worst traded price of the second, 3% protection, bands, 15:08 cut, 15:10 square-off | REQ 12, 13; 11.1 |
| The headline | the data decides: one pre-registered pick by THE RANKING over the union of every scope and account, on earlier months only (25.2.6). SNIPER and the highest net chain print beside it, labelled | REQ 24a |
| The aim | "our aim is to achieve the maximum profit": the highest net chain prints on every report (25.2.8) | operator, 7 Nov |
| Fewest trades | item 16's order stands: the bar, then lowest max drawdown, net, accuracy, fewer trades, shorter holding, net per trade | REQ 2, 16, 16a, 16b |

### 25.2 The new entry families and groups

Written for a long; a short mirrors every price test. Every rule of 6, 7.3, 9, 11, 12.1 and 14 applies unless a line here says otherwise.

#### 25.2.1 E3C and E3S: the opening range on top movers (requirement 27 (a))

| Item | Exactly |
|---|---|
| The range | For duration d in {1, 2, 3, 5, 10, 15, 30, 60} minutes: `H_d`, `L_d` = the highest high and lowest low of the session's first d 1-minute candles, from the session's own first minute (`door::session_minute`). It closes at open + d minutes. All durations run on every candle size |
| Rank and ladder | Inside top N by 5.3, same pools (long gainers, short losers). The ladder of 6.1 is read for targets and context, and is not required for the entry |
| E3C, close reading | T = the close of a candle of the recipe's timeframe, at or after the range's close, whose close is above `H_d`, while the last close at or after the range's close was not. Market entry at the first traded second at or after T + 14 s |
| E3S, second reading | Armed at each candle close at or after the range's close whose close is at or inside `H_d`. T = the first GDFL second from that close + 14 s to the next close whose printed high exceeds the R1-mapped `H_d`, + 1 s; market entry + 14 s. Every clause is read on the arming candle, and the trigger consumes the arming, as E1S's |
| No look-ahead | **A decision only after its range has closed, never inside it.** A candle whose close falls on or before the range's close writes no record. A candle that began inside the range and closes after it may decide, because every minute it reads has closed by T. The second reading reads no candle after the arming close. Test: shuffling every minute after T, and everything after the range's close at a T inside it, changes no record |
| Stop (S50), always present | Seed: "just beyond the range": `L_d` - 1 measured step (`pattern_extreme` = `L_d`). Placements: the far edge, the next rung below, L = the broken edge `H_d`; offsets 0, 1 step, own p25, p50, p75, generated as 12.3: 15 stops |
| Exits | E1's lane table. XH closes back below `H_d` (level) or `L_d` (box low), or the rank leaves top N'. No XJ. XE's stair-step K = 2 (class C, as `exits::STAIR_K_E2`) |
| Seeds, wave 0 | Every duration, labelled "seed (operator, REQ:38)"; top 10, in top N, running A1, the side's colour, first break, E1's seed stop rule and lanes XA, XB, XC, XD2, XD3, FIRST admission, controls off. First break is printed "seed (main session for REQ 27's break; the operator gave no number)". Central seed d = 1, the operator's chart (09:15-09:16) |
| Dials, level 1-2 | The rank dials of 5.3, S13, S15, S21 (frozen at the range's close), S23, S25, TIME, S37, as 12.2. S37a: the close beyond the range, ppm of the range's width. **ORBW** the range's width: ppm of price; against the stock's own usual range at that duration (a causal running histogram, as S13); 9 generated bands each. **ORBB** which break of the range today: first (seed), second, second or later, any. **ORBG** the break within g candles of the range's close, doubling points, seed off |
| Generated | ORBW, S37a, S13, S25, stops and exits, per stock, per window, on learning months (12.1). Filters as 12.6, on the stages of 25.5 |
| Refusals, by name | `orb_tainted` (a hole or padded-traded minute inside the range: no E3 on that d that day); `orb_no_width` (`H_d` = `L_d`); `incomplete_candle`; `no_level_ratio`; `no_trigger_ratio`, `no_fill_data_window` (E3S); per lane `no_ladder_target` when a rung target has no usable ladder at T |
| Record | `family` E3C or E3S (`Family` codes 4, 5). Head: `pattern_extreme` = the far edge, `level` = the broken edge, `rung` 255, `bounce_high` absent. New columns (reserved table, INTERFACES section 5): 198 `ORB_D`, 199 `ORB_WIDTH_PPM_PRICE`, 200 `ORB_WIDTH_VS_USUAL_PPM`, 201 `ORB_BREAK_ORD`, 202 `ORB_CLOSE_BEYOND_PPM`, 203 `ORB_GAP_CANDLES`. One record per (break, d, reading), while inside the loosest rank edge. `SignalRec::BYTES` stays 990 |

#### 25.2.2 ORB-POS: a filter on E1 and E2 (requirement 27 (b))

Eight readings, one per duration, columns 206-213 `ORB_POS_PPM_d`: the signed distance of the last close at T beyond the range, in the side's frame, ppm of price; 0 inside. Values: above, inside, below (E, one series each, labelled "seed (operator, REQ:38)"), and 9 generated bands of that distance, own quantiles, learning months; coarse 3 + 3. **A reading is `ABSENT32` until its range has closed, and an absent reading passes no clause, "inside" included.** Second reading: read at the arming close (BRIEF:281). A hole inside the range holds it absent, counted `orb_tainted`. It replaces nothing: the opening-range position bands A5 (15, 30) stay. It enters wave 2's new stages only, so waves 0-1 and every earlier stage are unchanged.

#### 25.2.3 PATTERN-AT-TOUCH (requirement 28 (a))

The group is read on the touch candle T and the one before it. It is crossed with E2's rung x clause = 1, = 2, = 3, >= 2, >= 3 (the stages of 25.5), and it is also read on the flag's and the opening range's break candle.

- **Named patterns,** columns 204, 205 `PAT_BITS_0`, `PAT_BITS_1` (31 bits each, never the sign bit), one clause "bit k set", E, one series each. Source: `W/state/fib-run-20261016/patterns-amendment.txt` Layer 2 and its level-touch block, the NR forms of Layer 1, SPEC.md 7.2's P1-P7. A name needing a candle after T cannot run at the touch and is not run here: P5 "touch and turn" and any confirmation candle, reached by the candle-code sequences instead.
- **Thresholds are never typed.** `PatBook`, per (stock, timeframe): a causal histogram of body, upper wick and lower wick as permille of the candle's range, frozen each 1 January from everything before it (class C, as round 2's `pct.rs`). `q(p)` reads it: "generated: causal p of the stock's own candles". Below MIN_REF = 30 values a name that needs the book is not set, counted `pat_book_warming`.
- **Primitives, in the side's frame** (a SHORT signal's candle is mirrored first, so every name reads as a long): green close > open, red close < open, flat neither. b, u, d = body, upper wick, lower wick; r = range. small: b/r <= q(25); big: b/r >= q(75); doji: b/r <= q(12.5); longdn, longup: d/r, u/r >= q(75); shortdn, shortup: d/r, u/r <= q(25); mid = (open + close) / 2. prior_down: close(T-1) < close(T-3); prior_up its mirror. L = the record's rung (E2), holding level (E1) or broken edge (E3), at T's ladder per FIB-I2.

| # | Name | Rule | # | Name | Rule |
|---:|---|---|---:|---|---|
| 1 | doji | doji(T) | 28 | outside_bar | high(T) >= high(T-1), low(T) <= low(T-1), not both equal |
| 2 | doji_long_legged | doji; u/r, d/r >= q(50) | 29 | morning_star | red big(T-2); small(T-1) with its body below close(T-2); green(T), close(T) > mid(T-2) |
| 3 | doji_dragonfly | doji; shortup; longdn | 30 | evening_star | green big(T-2); small(T-1) above close(T-2); red(T), close(T) < mid(T-2) |
| 4 | doji_gravestone | doji; longup; shortdn | 31 | morning_doji_star | 29 with doji(T-1) |
| 5 | doji_four_price | r = 0 | 32 | evening_doji_star | 30 with doji(T-1) |
| 6 | hammer | small; longdn; shortup; prior_down | 33 | abandoned_baby_with | red(T-2); doji(T-1), high(T-1) < low(T-2) and < low(T); green(T) |
| 7 | hanging_man | 6's shape; prior_up | 34 | abandoned_baby_against | green(T-2); doji(T-1), low(T-1) > high(T-2) and > high(T); red(T) |
| 8 | inverted_hammer | small; longup; shortdn; prior_down | 35 | three_white_soldiers | green T-2, T-1, T; closes rising; each open inside the body before; shortup(T) |
| 9 | shooting_star | 8's shape; prior_up | 36 | three_black_crows | 35 mirrored |
| 10 | marubozu_with | green; big; shortup; shortdn | 37 | three_inside_up | 17 at (T-2, T-1); green(T); close(T) > close(T-1) |
| 11 | marubozu_against | red; big; shortup; shortdn | 38 | three_inside_down | 18 at (T-2, T-1); red(T); close(T) < close(T-1) |
| 12 | spinning_top | small, not doji; u > b; d > b | 39 | three_outside_up | 15 at (T-2, T-1); green(T); close(T) > close(T-1) |
| 13 | belt_hold_with | green; big; shortdn; prior_down | 40 | three_outside_down | 16 at (T-2, T-1); red(T); close(T) < close(T-1) |
| 14 | belt_hold_against | red; big; shortup; prior_up | 41 | rising_three_methods | green big(T-4); T-3..T-1 small, inside T-4's range; green(T), close(T) > close(T-4) |
| 15 | engulfing_with | red(T-1); green(T); open(T) <= close(T-1); close(T) >= open(T-1); b(T) > b(T-1) | 42 | falling_three_methods | 41 mirrored |
| 16 | engulfing_against | 15 mirrored | 43 | nr4 | r(T) < every r of T-3..T-1 |
| 17 | harami_with | red big(T-1); green(T), its body inside T-1's body | 44 | nr7 | r(T) < every r of T-6..T-1 |
| 18 | harami_against | 17 mirrored | 45 | one_rung_covered | T's range holds L and no other rung (SPEC P1) |
| 19 | harami_cross_with | red big(T-1); doji(T), its body inside | 46 | two_at_rung | T-1 and T both touch L (P2) |
| 20 | harami_cross_against | green big(T-1); doji(T), its body inside | 47 | three_at_rung | T-2, T-1, T all touch L (P2) |
| 21 | piercing_line | red big(T-1); green(T); open(T) < close(T-1); mid(T-1) < close(T) < open(T-1) | 48 | dip_then_side | low(T-1) <= L; green(T); close(T) >= L (P3) |
| 22 | dark_cloud_cover | 21 mirrored | 49 | partial_u | low(T-1) < low(T-2) and < low(T); T touches L (P4) |
| 23 | tweezer_bottom | lows of T-1, T within one measured step; red(T-1); green(T) | 50 | body_through | open(T), close(T) on two sides of L (P6) |
| 24 | tweezer_top | 23 mirrored on highs | 51 | wick_touch | low(T) <= L < min(open, close)(T) (P7) |
| 25 | kicker_with | red(T-1); green(T); open(T) >= open(T-1) | 52 | body_inside | open(T), close(T) both within the record's tolerance of L |
| 26 | kicker_against | 25 mirrored | 53 | pin_rejection | wick into L as a share of r >= q(75), close back away (s19) |
| 27 | inside_bar | high(T) <= high(T-1), low(T) >= low(T-1), not both equal | 54 | rejection_pair | T-1 touched L in the against colour; 53 at the same L and signature (the operator's 19 Oct example) |

- **Anatomy and codes.** 405 pairs of the 10 anatomy readings of T and T-1 (5 x 2; generated coarse bands, at most one band a reading), and the 600 candle codes of lengths 1 and 2 of round 2's alphabet. Anatomy singles on the touch nodes are already in `W2.singles.base`, and XA pairs from 5 minutes in `W2.pairs.base`: no series is counted twice.
- **No look-ahead.** Every candle read has closed by T. **On a second reading the touch candle has not closed at the decision, so the group reads the arming candle and the one before it,** printed "arming candle, not the touch candle" (BRIEF:281). On a ladder change the signature rule of 7.2 item 1 holds: "same rung" means the same (slot, lo, hi, leg, rung).
- **Refusals:** `pat_book_warming`; a name needing T-k before the session's k-th candle is simply not set. F2b counts bits per stock and year in the fold's tally.

#### 25.2.4 EU: one recipe, every way in (requirement 28 (b))

| Item | Exactly |
|---|---|
| A recipe | One timeframe, one account side, one N, one exit lane, and at most one flag member (K 1, 2, 3 at own p50 low), one touch member (rung 0.786 x = 1, = 2, = 3, >= 2, >= 3, any) and one opening-range member (d, 8), at least two members: 234 nodes, 48 with no generated value. EUC: every member on its close reading. EUS: every member on its second reading. A mixed reading is left to stage 2's mix |
| One account | First signal wins: admission reads the merged list in start-second order, then S81, then flag, touch, range. One position per stock, long and short alike, `while_open` for a later signal. The day and month ceilings are shared, counted at entry on the account. A trade keeps the stop, target and exit lane of the member that fired |
| Seeds | F + T, F + O, T + O, F + T + O at K 2 own p50 low, 0.786 = 2, d = 1, lanes XA, XB, XC |
| Dials | Level 1 and 2 on the 4 seed nodes (level 2 below 15 minutes on T + O alone): the rank dials, S37, S37a, on all members together. Filters: coarse singles on the 4 seeds. A touch member under a named pattern: `W2.eu.pat` |
| No new record | EU reads its members' group files; no walk of its own, and `Family` codes 6, 7 hold no group file. `Candidate` gains its member list; two recipes differing in one member are two series. Every series goes to the lists, the SNIPER table, the three chains and the luck stream, as any other. A member's generated numbers keep 14.1's reach |
| No look-ahead | Every member's record is decided as its family decides, and merging reads start seconds only. Test: an identity twin reproduces the merged list; perturbing months >= LEARN[Y] changes no pick of Y |
| Beside it | `S2.mix` (top 5 E1 x top 5 E2) stays: EU is searched and generated on all windows, the mix pairs candidates |

#### 25.2.5 The three accounts (requirement 30)

**(B) STOCKS** is this preview as specified, under 25.1 and 25.3.

**(A) INDEX, run first.** NIFTY and BANKNIFTY spot signals, traded in that index's options only.

| Item | Exactly |
|---|---|
| Families | E1C, E1S, E2C, E2S, E3C, E3S, EUC, EUS on every timeframe, 6 sides. A FADE of a long signal buys the put |
| Top-mover clause | **Off, labelled on every line: "top-mover clause off: two names cannot rank".** S10, S11, S12, S15, the rank exits XH and S81 (c) do not exist on (A). S13 stays, the index's own move against its usual |
| Volume | An index has none: S35, the 2 trigger volumes and the 3 indicators that read volume are refused by name, `no_volume`, never set to 0 |
| Signal data | The index tool's: Zerodha index minutes, a minute it lacks folded from the GDFL index seconds of the same series (`spot::fold_missing`), a minute neither holds a hole; stops, targets and triggers on GDFL index seconds through R1. Label: "NIFTY and INDIA VIX minutes: IDXGAP-1 not landed" (DATA-EVIDENCE section 5) |
| Library | COMBINATIONS.md "Account (A)'s library": no cross-section universe, no industry, no trigger volume, 33 of the 42 indicators, + NIFTY only, BANKNIFTY only |
| The route | At each decision, for that index: **the current expiry; CE on a long, PE on a short; top traded volume in the window before the order's start (seeds 5, 15, 30 minutes; centre 15), then moneyness (seeds ATM, 1 ITM, 1 OTM; centre ATM), then premium: the largest whole-lot count at or under the amount.** The contract rule's 9 values run on the seeds in `A.W1.contract`; every other stage at the centre. Labelled seeds by OPERATOR NOTES 5-7; a contract whose single lot exceeds the amount is `lot_above_premium`, skipped and counted |
| Size | Premium seeds Rs 30,000, 50,000, 1 lakh, 2 lakh (`WalkKey.amount`). N = 1 or 2: one position per name always (NOTE 7). Premium in use never above free cash. Pairs and triples run at the Rs 2 lakh ceiling only, by cost |
| Fills and charges | The index tool's `options.rs`: dated lots, freeze-quantity slices, the buy limit from the print before the decision, the volume-capped worst print second by second, half a tick of spread per slice, `mark_holes`, "option data incomplete" refused; `rates_on` and `charges`, official reading, the operator's Dhan screen within Rs 1 (REQ 25a). Exit decisions come from the spot; the exit order is a market order on the option at decision + 14 s, as REQ 20 says |
| Data limits, printed | Options end 2026-10-29 (30 Oct not imported; DATA-EVIDENCE section 5: 30 Sep lacks 09:16-09:21; 2021-03-30 unchecked; 158 missing contract-days, 94 of them BANKNIFTY 2020-06-04; OI on 9 of 20 samples, and no OI dial is read). A refused day is never filled. Spread: **NOTE 9's lane is owed (F6):** measure real spreads from the tick store's quotes where they exist, add a wider labelled lane than 2 ticks, and label what is unknown. Labels: "index-options walk: check 3 not finished" |
| Stocks' rules that do not apply | ban list, C_VAR1 leverage, security-file bands: none on an index. Option execution range as the tool holds it |

**(C) BOTH.** One account across (A) and (B): index signals in options, stock signals in cash, the 5 a day and 100 a month shared, one integer each, premium and margin together within free cash. Its series are (i) the same recipe on both universes: `C.W0.*`, `C.W1.base`, level 1 on lane XA, the option at the Rs 2 lakh ceiling, top-mover clause on for stocks and off for the two indices, labelled; (ii) `C.mix`: in each window, each scope the two accounts share and each SNIPER cell, the top 5 of (A) x the top 5 of (B), at each N seed, from that window on (24.P1 Fix 1). Its series need both spools, so (C) runs last.

**Never added together.** Every report prints the three accounts side by side, net Rs and net % of Rs 20 lakh, with no total row. Each is its own walk-forward chain.

#### 25.2.6 How the accounts enter line 2: one reading, no kinder door

**Line 2 reads one chain, GLOBAL-ALL.** In each window Y, the #1 by THE RANKING, on Y's learning months only, over every series offered to Y from all three accounts at the main ceilings, stage 2's included (REQ 24a; 24.P R2; reach as 24.P1 Fix 1). The chain carries one account from Rs 20 lakh over the 82 unseen months (14.1). The kind of recipe and the account it trades change only at a year's start, and only by earlier months, so the data decides. `report::verdict` reads this chain and no other; `VerdictInputs` gains no account field. Rules 1-7 of 14.5 apply to it as amended by 24.R, 24.P R1 and R3: the fixed floor; O of GLOBAL-ALL against the L95 of one luck stream; the twin's GLOBAL-ALL. `Scope::Global` keeps its meaning inside one account's run and prints as GLOBAL-A, GLOBAL-B, GLOBAL-C.

| Door | Why it stays shut |
|---|---|
| Three chances | Three lines would be three tries at the bar. There is one line. GLOBAL-A, -B, -C print beside it, labelled, and never write it |
| Picking the account after the fact | Any account chain read on its unseen months prints "chosen among N chains on their unseen months: NOT a pass". If one holds the fixed floor in all 82 months and the headline does not, line 3 gains, after the label, as 24.P's sentence does: `THE <INDEX\|STOCKS\|BOTH> ACCOUNT'S CHAIN MET THE FIXED RS 2,50,000 IN ALL 82 UNSEEN MONTHS AND THE HEADLINE DID NOT (<x> OF 82); LINE 2 READS THE HEADLINE, AND THIS IS NOT A PASS` |
| Fewer tries in the count | Every series of every account, ceiling B and every comparison lane included, is offered to the one luck stream and is in one `tried_total`. Printed per account beside the whole. The bar rises for every chain: the honest cost of 51 billion more tries |
| A kinder null | The twin runs on all three accounts and builds its GLOBAL-ALL the same way. Twin seed 1 on waves 0-1 of (A), (B) and (C) always runs, about 13.2 model hours (ESTIMATE). From less than a whole pipeline it prints 24.11's words with achieved figures, today "partial null: twin seed 1 only, waves 0-1 only, 4,681,284,192 of the real 196,294,908,660 options; a smaller search finds less luck, so beating it is easier than 14.4's twin, and it does not clear the chain" |
| A kinder ceiling | Ceiling B enters no list, cell or chain of the main ceilings (24.P1 Fix 2) and never writes line 2. A switch counts only on the operator's word before the first print of any wave 0, and (A) prints first |
| An early look | (A) runs first and prints PENDING with its own chains. Nothing it prints can change a rule: every rule here is sealed before (A) starts. After any print a change of line 2, account or scope is a new family judged on fresh months (24.R; 19) |
| Options' missing days | (A) has no session 2026-10-30, and a refused option day is not filled. It nets what it nets: October 2026 still reads the month's trades |

**Scores across accounts compare like for like:** every key is net on the same Rs 20 lakh, the same fixed bar, the same months; ranking reads no route. Option charges and fills are real for that route, so nothing needs scaling.

#### 25.2.7 The option-route replay of (B)'s chains (requirement 20)

After (B)'s shown chains exist, each replays in the stock's own option as a separate account, beside cash, never added. It reads the chain's own decision times, stops and exits from the spot, the same admission and the 5 a day and 100 a month; a signal trades cash in one account and its option in the other, and no signal is both inside one account.

- **Which chains:** at most 12 shown chains, in a data-blind order: GLOBAL-ALL, GLOBAL-B, SNIPER, MAX-NET, then the other chains at or above +100% in the fixed scope order (closes highprob-check1's nonblocking 15).
- **Rules:** REQ 25a's Rs 2 lakh ceiling and its 4 labelled seeds; the current expiry; the contract rule of 25.2.5 with its 9 values, and NOTE 5's moneyness-first and price-first rules as comparisons: 11 rules x 4 premiums, `S2.shown.replay`, 528 at most.
- **Code:** `psrb-preview-opt-1`'s `choose.rs`, `walk.rs`, `accounts.rs`, vendored as read, with 24.13's net-only rules. Overlay check 4's B1 binds: the session-end remainder gets the volume cap, or the line is labelled.
- **Refusals, counted, never filled:** `no_option_folder` (13 renamed series' option folders stop early, fno-history READY section 9), `option_data_incomplete`, `lot_above_premium`, `ban_day`.
- **It decides nothing:** it goes to the luck stream alone and enters no list, cell or chain (24.P1 Fix 2), because which chains are shown reads unseen months. It prints net Rs and % of Rs 20 lakh, its own R5 columns, and its own month table. The gate's W and L read the cash records (closes highprob-check1's nonblocking 18).

#### 25.2.8 The chains, MAX-NET and the side-by-side table

| Account | Scopes | SNIPER family | Chains |
|---|---:|---:|---:|
| (B): the 39 + E3C, E3S, EUC, EUS + MAX-NET | 44 | 16 | 60 |
| (A): GLOBAL, 8 families, 8 timeframes, 6 sides, 8 money cells (N 1, 2 x 4 premiums), FEWEST, TPM x 3, LOWEST-DD, 10 touch rows, MAX-NET | 47 | 16 | 63 |
| (C): GLOBAL-C, SNIPER-C, MAX-NET-C | 2 | 1 | 3 |
| GLOBAL-ALL, line 2 | 1 | | 1 |
| **All, "chains formed: x of 127"** | | | **127** |

Ceiling B's chains print apart: its own GLOBAL and SNIPER from waves 0-1, and every shown chain re-run at it (`WaveOut.ceiling_b`).

**MAX-NET ("our aim is to achieve the maximum profit").** A scope: in each window, the series with the highest net on learning months, ties by THE RANKING, chained over the unseen months; per account and over all. It is walk-forward and printed beside the headline. Beside it, on every report, interims included, in part 2: **"The highest net of any chain formed: <name>, Rs x, y% of Rs 20 lakh, chosen among N chains on their unseen months: NOT a pass".** If it nets +100% or more it prints in full (REQ item 6), else as one line. It never writes line 2.

**New seeds, by label.** The operator's, printed "seed (operator, <file:line>)": the 8 durations; above, inside, below; the premium seeds; the contract rule's 5, 15, 30 minutes and ATM, 1 ITM, 1 OTM (OPERATOR-NOTE). The main session's, through `ValueClass::SeedMain`: E3's first break and its central d = 1's reading of the chart; EU's member lists; the contract rule's centre, 15 minutes and ATM. Class C, generated causally: `PatBook`'s quantile points. Nothing else is typed.

### 25.3 The data rules, each with its owner

Sources: 24.Y, `proof-20261107/data/RECONCILE.md` section 8, DATA-EVIDENCE.md, `downloads-20261107/*/READY.md` (all PASS). RECONCILE's check 1 found 10 wording items and held every core number; its repair is the proof step's, and nothing here rests on the words it faulted. D = `state/downloads-20261107`.

| # | Rule | Exactly | Owner |
|---|---|---|---|
| D1 | **2024-07-15 and 2024-07-16 refused by name** | Half the rows and seconds, all the shares: second-by-second fills see half the prints, in no later list. `Reason::HalfRows20240715` (`half_rows_20240715_16`), no trade on (A), (B) or (C) on either day; Zerodha minutes still feed rank, ladders and causal state. Counted in the months unit | F1b; F6 for (A) |
| D2 | **TATACONSUM read through TATAGLOBAL** | Sessions to 2020-02-26 read GDFL seconds from the folder TATAGLOBAL: 365 stock-days, additive in `door::DayReader`. Named, not filled: ANANDRATHI, DATAPATTNS, ENRIN, KARURVYSYA, SYRMA, UJJIVANSFB, 42 sessions each of Aug-Sep 2026 (252; AUC-1), refused under M10; ICICILIFE's 7 October sessions (3.1) | F1b |
| D3 | **Ratio-plus-offset spans read raw, count printed** | `proof-20261107/data/zerodha-factor-spans.tsv`, read as data, digest in the run identity: kind `ratio_plus_rupee_offset` (119 stable spans, 25,661 stock-days, 32 names; 29 isolated spans, 63 stock-days) and reading `no_stable_factor` (297 spans, 4,229 stock-days): **29,953 named stock-days. On each, the minutes are folded from that day's GDFL seconds.** O first print, H, L, C last, stamped at the minute's start; a minute with no print is a hole by 3.4. Why read, not refuse: a table built from later adjustments may choose which raw source to read, since raw prices hold no future, but refusing by it would choose which days exist, which is look-ahead. No GDFL seconds that day: refused, `offset_span_no_raw`. Level-based causal readings (EMA, ATR, anchors A6-A7, CPR, pivot) restart at each change of basis, `ABSENT32` until warmed, never a guess. RECONCILE's check 1 (finding 2) notes that some no-stable-factor days are unadjusted days with short extremes; reading those raw is harmless. Every report prints "read raw: x of 29,953; refused y; plain-ratio days read Zerodha, whose percent readings stay right". Span table stops 2026-09-30; October 2026 reads Zerodha, labelled | F1b, F2b |
| D4 | **Rank and rupee readings on raw** | On D3's days the rank's P is the GDFL-folded close and C the NSE cash bhavcopy CLOSE of the session before (basis a) or the last raw print before (basis b); a basis with no raw close leaves that basis NOT_RANKED. `col::PRICE` and `col::LIQ_LAKH` read raw on every stock-day: R1's G and GDFL volume (372 of 1,594 stock-years change price tier otherwise) | F1b, F2b |
| D5 | **Dated leverage from C_VAR1** | Session D reads file D, no shift. M = col 7 + col 8, L = min(5, 100 / max(20, M)). Filed name joined by READY's 19 rows, the last two KPIT > BSOFT, TATAGLOBAL > TATACONSUM. The duplicate SUBCAPCITY row of `C_VAR1_12012023_1.DAT` is taken by name; no other file is skipped. No row: refused by name, never 5x. Amount: 25.1. Printed: "L is NSE's file with Dhan's published formula, never Dhan's own number"; column 9 and the L-SENS 3x lane stay comparisons. Lifts "dated leverage UNAVAILABLE" for 2018-09-03..2026-10-30 | F1 done (10:5x, 11:4x); F4 done; F5 lifts the label; F4b ceiling B's margin |
| D6 | **Bands and ticks** | **Session S reads the security file of the session before S,** the look-ahead found today, by column NAME. `PricRg`, `BidIntrvl` for 2024-02-06..2026-10-30; base = PricRg's midpoint. Before 2024-02-06 computed, labelled "band computed" and the measured tick. Flex stays observed. Labels stay: the 6 demerger sessions; 2024-02-06, 02-07's web date; closing-auction eligibility only from file 2026-07-31. A test: S never reads file S | F1 done; F3 reads `pric_rg_ppm`; F5 lifts labels by span |
| D7 | **Ban days** | Session S reads S's OWN file (no shift), by READY's parser, refusing any other shape. `Reason::BanDay`. **2025-05-06:** the file is not in the set. Main session, no web request: copy `D/secban-verify-probe/raw/fo_secban_06052025.csv` into `D/secban/raw` and replace the 404 manifest row as READY section 8 says, before the seal; then MANAPPURAM and RBLBANK are refused that day. Until then that day stays "ban days: UNAVAILABLE (traded)". **2020-03-23** reads NIL, labelled "ban list doubtful: file re-issued after the session". 168 other late stamps: "public before the open shown by OI, not by stamp". On the option replay a ban day is refused too | F1 done; F1b the 2025-05-06 read; main session the copy |
| D8 | **The official dated F&O list** | D/fno-history READY section 4: **session S reads the newest file dated before S; X is in the list when that file holds a stock option row for X with expiry >= S.** Option rows only, instrument word first. Per-day, never first..last. `D/tools/fno-history/verify/out/stock-option-by-day.tsv`, `old` on the 3 two-form days. Replaces `Membership::is_member` for trading and the pool. Cases: **2021-03-30 reads 2021-03-26 and 2021-03-31 also reads 2021-03-26** (NSE's file is a 404, nothing filled); 2018-09-03 has no earlier file: the GDFL list under its own label; 2021-08-30's 10 re-issued names carry the note; 168 first days not traded. Reading file S for S is a defect | F1b (F1 "reads no fno-history" today) |
| D9 | **The departed names: a labelled survivorship lane** | On shown chains only, a comparison lane, luck stream alone, printing "how many trades and how much net change". **Facts:** PLAN section 6: 125 series, 81,228 option days, 75,460 with GDFL seconds (92.9%), **5,768 unbuilt days**; READY's corrected count with 19 joins: 123 series, 80,880, 75,150, **5,730**; months 2018-11 1,535, 2019-10 646, 2020-11 437, 2021-02 438, 2021-11 533, 2022-10 541, 2023-11 521, 2024-03 451, 2024-05 526, 2024-11 1, 2025-10 101. **It needs:** D8's list with the departed in the traded set and the pool; every name's candles folded from GDFL seconds (121 of the 123 have no Zerodha folder), the 218's too inside the lane, so ranks compare; READY's joins including KPIT, TATAGLOBAL's two folders; C_VAR1, bans and bands as D5-D7 (their rows hold departed names); one Sym table per test year, because `Sym::rank` is u8 and 341 series do not fit, a year above 255 names stopping by name. The unbuilt days print refused, `departed_day_unbuilt`, until a shadow build from `tickstore-data/cm/STOCKS` fills them (a separate step, never into the store). No fno-skip rule exists for departed names: labelled. Survivorship stays first in every APPROXIMATE list | F6 (F6-SURV), after shown chains exist; F5 prints |
| D10 | **The labelled lane of shown chains** | Each shown chain's signals re-derived from candles folded from GDFL seconds, every stock-day; prints how many trades and how much net change. `S2.shown.data` with D9 | F5, with F2b's source switch |
| D11 | **Index minutes** | 25.2.5's signal data; `tainted` rules as 3.4 | F6 |
| D12 | **Every report prints** | "0 of 17 data fixes are on the store; AUDIT-3 has never run on it." "The fill feed's high and low fall short of NSE's on about half the days (RECONCILE section 1); the signal feed is on another basis on 35.99% of stock-days; nothing here is live." In the APPROXIMATE list, part 1 of 16 and the head of every interim. Test: no report without them | F5 |

### 25.4 Every open finding, closed by name

**`sniper-check3.md`'s 4 blocking points:**

| # | Finding | Closed by | Row |
|---:|---|---|---|
| 1 | The plain check is still at Rs 1 crore, and the ceiling has moved | **Closed here.** `combos/sniper-arith.mjs` re-run at the ceilings of 24.U on the official reading of 24.V, words as 24.12, W a stress column: `combos/sniper-arith.md`, `.json` (F5 prints from them, and a test compares them with `price_trade`). See below | 25.4 |
| 2 | Stage 2 variants' cell rule | 24.P's R4, replaced by **24.P1 Fix 3**: a variant enters by 2.1's own test, at its own N when that is an N seed, else the smallest N seed above; built by F4 (12:19 note, `n_seed_at_or_above`) | 24.P1 Fix 3 |
| 3 | 60 s and 300 s as class C display | **24.P R5's closing paragraph:** class C, display only, select nothing. F5 prints the label | 24.P R5 |
| 4 | Trades through the stop at first fill | **24.P R5's closing paragraph:** "through at fill", no average of multiples, "leaves out n trades already through the stop at the first fill"; F5's test: no zero or negative divisor | 24.P R5 |

**The plain check, as it now reads (labelled arithmetic, not a result; Rs 1,000 example share, official reading, worse clock).** A round trip at Rs 15 lakh costs Rs 579 today and Rs 720 in the dearest stretch, flat, however many fill seconds (the worst reading's stress column: Rs 594 to Rs 1,608).

| k trades a month | Net a trade (Rs) | Rs 15 lakh: price move | Rs 10 lakh: price move | Rs 5 lakh: price move |
|---:|---:|---:|---:|---:|
| 1 | 2,50,588 | 16.75% (16.76%) | 25.10% (25.11%) | 50.17% (50.19%) |
| 2 | 1,25,294 | 8.39% (8.40%) | 12.57% (12.58%) | 25.11% (25.13%) |
| 5 | 50,117 | 3.38% (3.39%) | 5.05% (5.06%) | 10.07% (10.09%) |
| 10 | 25,058 | 1.71% (1.72%) | 2.55% (2.56%) | 5.06% (5.08%) |
| 20 | 12,529 | 0.88% (0.88%) | 1.29% (1.30%) | 2.55% (2.57%) |
| 40 | 6,264 | 0.46% (0.47%) | 0.67% (0.68%) | 1.30% (1.32%) |
| 60 | 4,176 | 0.32% (0.33%) | 0.46% (0.47%) | 0.88% (0.90%) |
| 100 | 2,505 | 0.21% (0.22%) | 0.29% (0.30%) | 0.55% (0.56%) |

- **Replacing line 748 and SNIPER.md 5.2:** "At Rs 15 lakh and 2 trades a month, each trade's price must move about 8.39% in its favour to net the Rs 1,25,294 it needs; at 5 trades about 3.38%; at 20, about 0.88%; at the ceiling of 100, about 0.21%. With a 25 bp stop and 5 trades, 2 winners must each make about 35.3 times the stop; S-RR's seed at least 5 is not enough there. At 100 trades with half winning and a 25 bp stop, about 2.6 times is." Fewer trades at the same bar means a bigger move each.
- **Replacing SNIPER.md 2.1's reason:** "so a Rs 5 lakh position is not compared with a Rs 15 lakh one".
- **Replacing SNIPER.md 5.3's "Filling Rs 1 crore":** "Filling Rs 15 lakh at 10% of each second's volume can take many seconds, while the stop can be hit with the entry part filled. Brokerage stays Rs 20 an order on the official reading; a per-fill reading is the stress line, until a real contract note settles it."
- **Replacing SNIPER.md 5.4's "100 entries a day" and "2,100 entries":** "at the ceiling of 100 trades a month, month 82's Rs 348 crore is about Rs 3.48 crore a trade, 2,318% of a Rs 15 lakh position." First unseen month in which each trade would have to net 20% of Rs 15 lakh: 2 trades a month, month 9; 5, month 17; 10, month 23; 20, month 28; 40, month 34; 100, month 42. It stays table D, printed beside line 2 and never deciding.
- **The option route:** at Rs 2 lakh of premium and about Rs 521 a round trip, the premium must move about 1.51% a trade at 100 trades a month, 6.53% at 20, 25.32% at 5.
- Net-only sweep F-30 to F-39 are closed by this re-run. F-28, F-29 (`section-24*.md`) are copies of appended text and stay as they are.

**`highprob-check1.md`'s 7 blocking points: all closed by `highprob-repair1.md`,** rows of 24.P1. The repair's own independent check is still owed, with this section's.

| # | Finding | Row | Built |
|---:|---|---|---|
| 1 | A stage 2 variant reaches windows before its parent's | 24.P1 Fix 1; P-08, P-13, P-21 | F4 12:19 (`Rid.reach`); F5 owes the pair and mix entry |
| 2 | Comparison lanes would enter GLOBAL's list | 24.P1 Fix 2; P-08 | F4 12:19 (`adaptive::offer_variant`, `is_comparison`); F5 labels its lanes |
| 3 | No table rule for a scope's variant, a pair or a mix | 24.P1 Fix 3; P-10, P-11 | F4 12:19; F5 sets `PickRow.combo` |
| 4 | The year table prints two dials it says it does not | 24.P1 Fix 4 (a); P-11, P-16. **Ruled: (a) stands** | F4 12:19 (`gate_clauses`); F5 |
| 5 | R5 divides by zero at W = 0 and no trades | 24.P1 Fix 5; P-17 | F5 |
| 6 | "What stays" contradicts R3 on rule 4 | 24.P1 Fix 6 | words |
| 7 | One vector cannot reach P-02's function | 24.P1 Fix 7; P-02 | F4 12:19 (`wf::clears(wins, losses, zsq)`) |

**Its nonblocking points, ruled so no builder guesses:** 2: this section. 3: "top 5" reads "at most 5" (COMBINATIONS.md regenerated; SNIPER.md:134, :341 overridden). 4: line 31's "(and 12.5% ...)" is overridden by 24.P R1. 5: R6's words print under every print of R5's table: parts 6 and 7 and the interims. 6: O prints for every chain formed. 7: `wave_out` re-forms O, `luck_bar`, `luck_tier`, `closest` and `sn_walk` after the seam, not only the picks. 8: a SNIPER-HP window with no clearing series is no-trade months, named per window. 9: printed under the year table: "the walk's cell and the tightest cell holding the pick may differ". 10: reads "12.5% of that month's start capital, carried". 11: Wilson's two figures stand as the check recomputed them; no file regenerates. 12: P-16 and P-17 also print step 3's and step 0's sentences of R4. 13: line 3's sentences print in REPORT.md and the dashboard; every other output keeps the label alone. 14: the range is exact integers in a small big-integer module of F5's own, no new dependency, printed rows only; at most 8,200 trades by the ceiling. 15: 25.2.7. 16: about 5.24 model hours + its own stage 2 and adaptive rows, ESTIMATE. 17: REQ 24's own wording stays. 18: 25.2.7. 19: payoff in thousandths; a pair or mix in `Chain.picks` is F5's entry (F4 12:19).

**`sniper-check3.md`'s nonblocking points worth a line:** 1, 2, 3, 4 closed by 24.P R3 and P-12. 6: 25.7's ruling on the caps. 7: the headline question is closed by REQ 24a; "the floor" is 25.7's. 8: 25.5's line on duplicates. 9: the official reading has no fill split; the stress column keeps the remainder in the last fill, said here. 10: figures print from achieved counts. 11: 25.6's spool.

**The net-only sweep's 11 class A items** (`sat-20261107/net-only-sweep.md`, "Counts, as corrected"):

| Id | What decides | State | Owner |
|---|---|---|---|
| F-42 | S-RR read money before charges | Landed: planned net per walked amount, official charges (24.13; 24.V) | F4 (12:19), with F3's `walk::planned_charges` and F2's facts |
| P-05 | The verifier's PORT check | Text repaired, ADDENDUM 2 confirms | `bt_verify` builder (proof) |
| P-06 | L05 | Text repaired | `bt_verify` builder |
| P-07 | L08: net = sell_value - buy_value - charges | Text repaired | `bt_verify` builder |
| P-08 | A05 | Text repaired | `bt_verify` builder |
| P-09 | `bt_attack score`'s allowed list | Void: the sentence no longer exists (ADDENDUM 2, correction) | none |
| P-10 | P47, net arithmetic off by 1 paisa | Text repaired | `bt_attack` builder |
| P-14 | `live-gap/spot-sheet.mjs` | Applied | proof, done |
| P-17 | A10: results file, table and board | Text repaired; new results fail if the removed term prints | `bt_verify` builder |
| R-08 | `port_r3/src/lib.rs`:387-391 | Noted to B6 (round 3's notes, 10:5x) | B6, round 3 |
| R-09 | `opt_r3/src/main.rs`:1081, `inputs.rs`:126 | Noted to B10b | B10b, round 3 |

Class B items of this tool: F-01 to F-13, F-18, F-27, F-40, F-41 landed (F3 10:58, F4 10:47). F-15 to F-17, F-19 to F-23, F-26 are F5's, as N7 and N8 amend.

### 25.5 The recount

Run: `node count.mjs`, `COMBOS_FULL=1 node count.mjs`, `COMBOS_W=1 node count.mjs`, `COMBOS_W=1 COMBOS_FULL=1 node count.mjs`, `node check.mjs`, `node render.mjs` in `combos/`. `check.mjs`, all equal: each new family and account by nested loops, and all 672 cells, the 13 stage 2 rows and the 3 adaptive rows of 24.9 equal to `combos.json.bak-final`. Written by the same hand as `count.mjs`, so an independent recount is still owed. F4's `search::combos_cell` still reads all 672 cells of its plan to the unit from the new file (checked with its own parsing rule), so its tests stay green until F4b.

**What changed since 24.9** (guaranteed 145,276,706,220; expected 146,279,509,161; each generated recipe once 23,247,542,760; 84.3 model hours): requirements 27, 28, 29 and 30 added E3, ORB-POS, PATTERN-AT-TOUCH, EU, ceiling B, accounts (A) and (C), as new stages only. No earlier count moved. Table 197,480,655,060; named cut 1,185,746,400; committed 196,294,908,660; expected after the cut 198,475,377,556; each generated recipe once 31,559,317,908; model hours 98.5, under 100 by the cut. Requirement 17 holds: at least 100 billion, on the committed figure, and on account (B) alone. **The accounts run A, B, C. Inside an account each new stage runs after its mirror** (each gate stage before its mirror, as 24.6 fixed; ceiling B after its wave 1). That reconciles "after their mirrors" with requirement 30's "the indices run first": a stage's mirror is inside its own account.

#### The numbers

- **Guaranteed part of the table (exact, fixed now, data-blind): 197,480,655,060 evaluated combinations; base-2 logarithm 37.523.**
- **A named data-blind cut (by cost only; PREVIEW-SPEC 25.5): 1,185,746,400, `W2.singles.level1` on 1, 2, 3, 5 minutes, 13.97 model hours. Committed: 196,294,908,660; base-2 logarithm 37.514.** It is at least 100,000,000,000 whatever the data holds, because every band count is fixed in advance and a skip by a proven bound counts as tried.
- At most 57,620,896 more in stage 2 and at most 2,122,848,000 more in the adaptive drill-down (finer bands, a 4th to 6th filter).
- **Expected total, after the cut: 198,475,377,556; base-2 logarithm 37.530.**
- By account, before the cut: (A) NIFTY and BANKNIFTY in options 47,584,347,840; (B) F&O stocks 149,894,825,460; (C) both 1,481,760. Account (B) alone, and its 21 stages of 24.9 alone (145,276,706,220), each stay above 100,000,000,000.
- The same table with each generated recipe counted once, not once per learning window, after the cut: 31,559,317,908. Requirement 17 is judged on the committed figure above, the one the luck test uses (BRIEF section 9 rule 2 counts per distinct learning window, and near-duplicates get no discount), and every report prints the second beside it.
- 50.0% of the table (98,740,327,530) is FADE sides.
- 8.9% of the table (17,532,091,716) holds a Sniper gate (requirement 16b).
- Many of these hold no trade or repeat another: N = 1, 3, 5 enter one amount (24.U), so a recipe where N never binds gives three exact duplicates, and a dial value F2 keeps no record for keeps nothing (its 11:19 note). They stay in `tried_total`, which is never kinder; `distinct_total` and "with at least one trade" print beside it.

| Group | Evaluated combinations |
|---|---|
| Wave 0, account (B): the operator's seeds | 40,200 |
| Wave 1, account (B): levels 1-2 and the exits | 2,400,444,600 |
| Wave 2, account (B): every context filter, singles, pairs, triples | 145,203,723,060 |
| Ceiling B, account (B): waves 0-1 again, a labelled account | 2,290,617,600 |
| Account (A): NIFTY and BANKNIFTY, options only, run first | 47,584,347,840 |
| Account (C): both, one account | 1,481,760 |
| **Table** | **197,480,655,060** |
| Named cut | -1,185,746,400 |
| Stage 2 (at most): money, controls, exit drill on each window's candidates, the accounts' mix | 57,620,896 |
| Adaptive drill-down (at most) | 2,122,848,000 |
| **Expected total, after the cut** | **198,475,377,556** |

| Stage | Exact count | Model hours |
|---|---:|---:|
| `A.W0.seeds` | 43,968 | 0.00 |
| `A.W0.e3.seeds` | 7,680 | 0.00 |
| `A.W0.eu.seeds` | 12,672 | 0.00 |
| `A.W1.base` | 730,176 | 0.00 |
| `A.W1.e3.base` | 29,184 | 0.00 |
| `A.W1.eu.base` | 4,134,528 | 0.04 |
| `A.W1.level1` | 156,980,736 | 0.55 |
| `A.W1.e3.level1` | 13,349,376 | 0.05 |
| `A.W1.eu.level1` | 3,667,968 | 0.03 |
| `A.W1.contract` | 703,488 | 0.00 |
| `A.W1.e3.contract` | 122,880 | 0.00 |
| `A.W1.exits1` | 5,484,096 | 0.03 |
| `A.W1.e3.exits1` | 3,484,416 | 0.02 |
| `A.W1.exits2` | 6,467,136 | 0.03 |
| `A.W1.e3.exits2` | 2,943,360 | 0.02 |
| `A.W1.level2` | 1,986,894,720 | 1.26 |
| `A.W1.e3.level2` | 79,028,064 | 0.21 |
| `A.W1.eu.level2` | 15,233,184 | 0.06 |
| `A.CB.W0.seeds` | 43,968 | 0.00 |
| `A.CB.W0.e3.seeds` | 7,680 | 0.00 |
| `A.CB.W0.eu.seeds` | 12,672 | 0.00 |
| `A.CB.W1.base` | 730,176 | 0.00 |
| `A.CB.W1.e3.base` | 29,184 | 0.00 |
| `A.CB.W1.eu.base` | 4,134,528 | 0.04 |
| `A.CB.W1.level1` | 156,980,736 | 0.55 |
| `A.CB.W1.e3.level1` | 13,349,376 | 0.05 |
| `A.W2.sniper.singles.seeds` | 17,671,680 | 0.05 |
| `A.W2.singles.seeds` | 423,299,520 | 1.11 |
| `A.W2.e3.sniper.singles.seeds` | 2,377,728 | 0.01 |
| `A.W2.e3.singles.seeds` | 40,746,240 | 0.09 |
| `A.W2.orb.singles.seeds` | 91,865,088 | 0.25 |
| `A.W2.sniper.singles.base` | 37,783,296 | 0.05 |
| `A.W2.singles.base` | 678,365,280 | 1.02 |
| `A.W2.orb.singles.base` | 157,888,512 | 0.23 |
| `A.W2.codes` | 265,863,168 | 0.09 |
| `A.W2.e3.codes` | 33,232,896 | 0.01 |
| `A.W2.pat.named` | 11,943,936 | 0.00 |
| `A.W2.pat.codes` | 81,561,600 | 0.02 |
| `A.W2.pat.anatomy` | 305,091,360 | 0.13 |
| `A.W2.pat.named.anatomy` | 522,547,200 | 0.04 |
| `A.W2.pat.named.context` | 32,322,240 | 0.00 |
| `A.W2.pat.level1` | 41,539,392 | 0.01 |
| `A.W2.pat.trigger` | 31,850,496 | 0.01 |
| `A.W2.eu.singles` | 11,200,320 | 0.05 |
| `A.W2.eu.pat` | 1,866,240 | 0.00 |
| `A.W2.sniper.triples` | 2,951,677,920 | 0.27 |
| `A.W2.triples` | 19,324,468,968 | 1.74 |
| `A.W2.e3.triples` | 217,209,840 | 0.04 |
| `A.W2.sniper.pairs.base` | 113,050,272 | 0.02 |
| `A.W2.pairs.base` | 1,009,358,064 | 0.14 |
| `A.W2.sniper.singles.level1` | 51,544,224 | 0.05 |
| `A.W2.singles.level1` | 1,059,985,776 | 1.04 |
| `A.W2.e3.sniper.singles.level1` | 1,581,120 | 0.00 |
| `A.W2.e3.singles.level1` | 29,708,736 | 0.05 |
| `A.W2.orb.singles.level1` | 20,141,568 | 0.01 |
| `A.W2.sniper.pairs.seeds` | 181,222,272 | 0.06 |
| `A.W2.pairs.seeds` | 1,940,312,208 | 0.67 |
| `A.W2.e3.sniper.pairs.seeds` | 1,495,248 | 0.00 |
| `A.W2.e3.pairs.seeds` | 12,981,816 | 0.01 |
| `A.W2.orb.pairs.seeds` | 52,275,456 | 0.01 |
| `A.W2.sniper.singles.level2` | 730,020,096 | 0.12 |
| `A.W2.singles.level2` | 14,643,692,112 | 2.33 |
| `W0.seeds` | 27,480 | 0.00 |
| `W0.e3.seeds` | 4,800 | 0.00 |
| `W0.eu.seeds` | 7,920 | 0.00 |
| `W1.base` | 456,360 | 0.01 |
| `W1.e3.base` | 18,240 | 0.00 |
| `W1.eu.base` | 2,584,080 | 0.11 |
| `W1.level1` | 118,918,080 | 2.11 |
| `W1.e3.level1` | 9,126,720 | 0.18 |
| `W1.eu.level1` | 3,369,600 | 0.13 |
| `W1.exits1` | 3,560,040 | 0.11 |
| `W1.e3.exits1` | 2,200,800 | 0.06 |
| `W1.exits2` | 4,520,040 | 0.11 |
| `W1.e3.exits2` | 2,035,440 | 0.06 |
| `W1.level2` | 2,151,393,840 | 6.40 |
| `W1.e3.level2` | 79,178,400 | 1.12 |
| `W1.eu.level2` | 23,082,960 | 0.42 |
| `CB.W0.seeds` | 27,480 | 0.00 |
| `CB.W0.e3.seeds` | 4,800 | 0.00 |
| `CB.W0.eu.seeds` | 7,920 | 0.00 |
| `CB.W1.base` | 456,360 | 0.01 |
| `CB.W1.e3.base` | 18,240 | 0.00 |
| `CB.W1.eu.base` | 2,584,080 | 0.11 |
| `CB.W1.level1` | 118,918,080 | 2.11 |
| `CB.W1.e3.level1` | 9,126,720 | 0.18 |
| `CB.W1.exits1` | 3,560,040 | 0.11 |
| `CB.W1.exits2` | 4,520,040 | 0.11 |
| `CB.W1.level2` | 2,151,393,840 | 6.40 |
| `W2.sniper.singles.seeds` | 11,347,200 | 0.15 |
| `W2.singles.seeds` | 330,259,200 | 4.18 |
| `W2.e3.sniper.singles.seeds` | 1,486,080 | 0.02 |
| `W2.e3.singles.seeds` | 29,603,520 | 0.37 |
| `W2.orb.singles.seeds` | 60,364,800 | 0.81 |
| `W2.sniper.singles.base` | 23,614,560 | 0.17 |
| `W2.singles.base` | 495,900,840 | 3.57 |
| `W2.orb.singles.base` | 98,680,320 | 0.71 |
| `W2.codes` | 166,164,480 | 0.26 |
| `W2.e3.codes` | 20,770,560 | 0.03 |
| `W2.pat.named` | 7,464,960 | 0.01 |
| `W2.pat.codes` | 50,976,000 | 0.04 |
| `W2.pat.anatomy` | 190,682,100 | 0.37 |
| `W2.pat.named.anatomy` | 326,592,000 | 0.07 |
| `W2.pat.named.context` | 23,775,120 | 0.01 |
| `W2.pat.level1` | 28,605,960 | 0.02 |
| `W2.pat.trigger` | 19,906,560 | 0.03 |
| `W2.eu.singles` | 8,646,480 | 0.20 |
| `W2.eu.pat` | 1,166,400 | 0.00 |
| `W2.sniper.triples` | 11,594,075,880 | 2.75 |
| `W2.triples` | 95,027,264,640 | 22.02 |
| `W2.e3.triples` | 1,075,825,800 | 0.88 |
| `W2.sniper.pairs.base` | 351,371,520 | 0.16 |
| `W2.pairs.base` | 3,849,768,360 | 1.82 |
| `W2.sniper.singles.level1` | 105,648,300 | 0.67 |
| `W2.singles.level1` | 2,633,575,320 | 16.24 |
| `W2.e3.sniper.singles.level1` | 1,306,440 | 0.01 |
| `W2.e3.singles.level1` | 28,606,200 | 0.25 |
| `W2.orb.singles.level1` | 16,364,160 | 0.02 |
| `W2.sniper.pairs.seeds` | 547,336,440 | 0.84 |
| `W2.pairs.seeds` | 7,093,779,600 | 10.53 |
| `W2.e3.sniper.pairs.seeds` | 4,639,680 | 0.03 |
| `W2.e3.pairs.seeds` | 49,120,740 | 0.28 |
| `W2.orb.pairs.seeds` | 161,308,800 | 0.15 |
| `W2.sniper.singles.level2` | 802,841,760 | 0.50 |
| `W2.singles.level2` | 19,964,882,280 | 11.69 |
| `C.W0.seeds` | 27,480 | 0.00 |
| `C.W0.e3.seeds` | 4,800 | 0.00 |
| `C.W1.base` | 456,360 | 0.01 |
| `C.W1.e3.base` | 18,240 | 0.00 |
| `C.W1.level1` | 784,740 | 0.02 |
| `C.W1.e3.level1` | 190,140 | 0.00 |
| **Table** | **197,480,655,060** | **112.5** |
| Named cut | -1,185,746,400 | -13.97 |
| **Committed** | **196,294,908,660** | **98.5** |

**The plain-words table and every stage row, with nodes, lanes and timeframes, are in `COMBINATIONS.md`, regenerated.** The per-recipe figure prints beside the luck test's on every report.

**The estimate (ESTIMATE; nothing measured on fib records).**

| Basis, at COMBINATIONS.md's rates | Hours for 198,475,377,556 |
|---|---:|
| Round 2 F&O, 722,285 accounts a second | 76.3 |
| PSRB round 2, 701,751 a second | 78.6 |
| The model, 261.6 million account steps a second, kept records guessed, after the cut | **98.5** |

- **Planning figure: about 76-98.5 hours of phase B on 12-13 threads under load.** The table without the cut models 112.5. If a free Mac doubles the rate, about 38-49 hours: a condition, not a measurement. The two-index and five-stock tests price it.
- **In run order, model hours:** (A)'s waves 0-1 end about 2.9 in, all of (A) about 12.7; (B)'s wave 1 interim about 23.5; ceiling B about 32.6; 100,000,000,000 crossed inside `W2.triples` on 30 minutes, about 52 in; the end about 98.5. Twin seed 1, not counted: about 13.2 more.
- **Not in the model:** about 3-10% for the per-series figures and slots (guess); an option path against a cash path; EU's merges; the O(1) month-full jump of the 12:4x note, which can only cut steps.

**The named cut, data-blind, by cost only, most model hours per combination first (SNIPER.md section 7's first two rows, narrowed): `W2.singles.level1` on 1, 2, 3 and 5 minutes: saves 13.97 model hours, loses 1,185,746,400 combinations, 0.6% of the table.** What it costs, plainly: on those four timeframes no context filter is crossed with a one-dial variant of the central seed at full bands. The operator's own charts are 1 and 2 minutes, and they lose this crossing, while keeping every filter on the seeds and base nodes, pairs and triples, and the gate mirror `W2.sniper.singles.level1`; 10 minutes and up keep it (2.28 hours); and `W2.fine` and the stacks can still drill it from a window's top nodes. A recipe whose edge lives only in a level-1 dial x a filter on 1-5 minutes may be missed by the committed search. The units come back in the extension below. Every report names the cut with its count.

**If the measured projection is still above about 100 hours,** the next whole units, in this order, never by a result, always named: `CB.W1.level2` below 15 minutes (2.9 h, 0.14 B); the rest of `W2.singles.level1` (2.3 h, 1.45 B); `W2.pairs.seeds` (10.5 h, 7.09 B); `A.W2.singles.level2` and its gate stage (2.5 h, 15.37 B); `W2.singles.level2` (11.7 h, 19.96 B). All of them: 29.9 h saved, 152.3 billion still committed. **Never cut:** waves 0-1 of every account at the main ceilings, so every new family's seeds and level 1-2; each filter on the seeds and base nodes; the gate stages.

**The extension (requirement 17a; committed search first).** The committed search reaches 100 billion on the luck test's count and prints its verdict. Then, while the CPU is free, CPU only, the same resumable shards run on in the fixed order of `combos/extension-order.tsv`: 305 units, one stage on one timeframe, every stage on every timeframe (588,858,155,736 in all), the named cut's units, then finer drill-down `X.triples.base` (coarse triples on the other base nodes, lane XA), cheapest model hours per once-counted combination first, by cost only. The whole table counted once is 93,348,437,100, so the finer units are needed. **By the model the once-counted figure passes 100,000,000,000 at unit 39, about 74 model hours after the committed search, with about 606 billion on the luck test's count.** ESTIMATE. **What line 2 reads meanwhile:** each report prints both figures and the verdict at its own checkpoint. Every new series goes to the lists and the luck stream, so picks can move and the luck bar rises for every chain. The stop rule is fixed and reads no result: the once-counted figure past 100 billion, or the operator's word, or the production fib backtest's priority (17.6). Line 2 reads the latest finished checkpoint before the stop, never an earlier kinder one.

### 25.6 Build

**Rules.** F1b, F2b, F3b, F4b reopen their stage's own files, feature and target dir under INTERFACES section 1, and each says DONE again in `PV/progress.md`. This section is the main session's word for the additive edits it names in F0's frozen files (`types.rs`, `contract.rs`, `door.rs`, `lib.rs`, `Cargo.toml`: no new dependency, `--locked`). Every new function: 8+ extreme cases, 2 combinations, 1 out of the box; flat p50 and p99 benches; achieved coverage printed, never rounded up; nothing read from `test-run/` by a checker that judges results. A full-data process before `logs/FULL-RUN-OK` exists is a defect. Exact lines go to `build/NOTES-shared.md` with this section.

**The order.** (0) Now: INTEGRATE finishes its 5-stock test and projection, then stops; F5 builds on, reading the new lines. (1) The independent check of section 25 and of 24.P1, then the new seal over 23, 24, 25, `COMBINATIONS.md`, `combos/*`. (2) F1b, then F2b, F3b, F4b, then F5's additions, then F6. (3) INTEGRATE-2: the two-index test, the seven-stock test, the projection, `logs/FULL-RUN-OK` written by the main session only. (4) **Account (A) first, because two names are quick:** phases R and A for NIFTY and BANKNIFTY, wave 0, its print, waves 1-2, stage 2. (5) **Then stocks:** account (B) once the disk fits (below): phases R, A, wave 0's print, wave 1, interim 1, ceiling B, wave 2, stage 2. (6) Account (C), then GLOBAL-ALL, the luck line, the twin, the verdict. (7) The extension.

| Stage | Work list |
|---|---|
| **F1b** (`day.rs`, `day/**`, `rank.rs`; additive `door.rs`) | (1) D1: `Reason::HalfRows20240715`, verdict refuses trading, minutes stay in the pool and causal state. (2) D2: `DayReader::seconds` reads TATAGLOBAL before 2020-02-27; a test on 2019-06-03 sees seconds. (3) D8: the official list replaces `Membership::is_member`, from `stock-option-by-day.tsv` by READY section 4's rule, with its cases; a test that S never reads file S; the pool follows. (4) D3: the span table as data, digest in the run identity; `StockDay.minute_source`; `DayReader::minutes` folds GDFL seconds on those days; `offset_span_no_raw`. (5) D4: the rank's raw P and C on those days. (6) D7's 2025-05-06 read once the main session's copy is there; 2020-03-23's label. (7) D1, D2, D3, D8 counts in `R-<id>-months`. (8) F2's named gap stays: readings 30-34 absent, no source |
| **F2b** (`fold.rs`, new `e3.rs`, `fold/orb.rs`, `fold/pat.rs`; `types.rs` additive) | (1) `Family::{E3C, E3S, EUC, EUS}`, codes 4-7, `is_flag` false, `reading` as named; `N_GROUPS` from the 6 families with a group file; `is_range()`. (2) E3 as 25.2.1: range state per stock-day and d, break ordinal, arming, trigger scan over `SecondsDoor`, records with 25.2.1's head and columns 198-203, refusals by name. (3) ORB-POS columns 206-213 on every E1 and E2 record, as 25.2.2. (4) PAT bits on every E1, E2, E3 record, `PatBook` per (stock, tf), as 25.2.3; arming candle on second readings; a by-hand vector for each of the 54 names, long and short. (5) D3: causal restart at a change of basis. (6) D4: `col::PRICE`, `col::LIQ_LAKH` raw. (7) `FoldCfg.rank_gate: bool` (off for (A): the top-50 gate and rank columns ABSENT32) and `FoldCfg.minute_source` (for F5's GDFL-fold lane, D10); volume columns ABSENT32 on an index. (8) Rows in `col_table()`; the reserved table of INTERFACES section 5 filled as 25.2. (9) Measured records a stock-day on the seven test names, in the notes |
| **F3b** (`exits.rs`, `walk.rs`, `spool.rs`) | (1) `full_spool_lanes(E3*)` = E1's; XH close back and L placement read `level` as the broken edge. (2) **Spool cut (a):** the sibling path-facts unit holds only the walks F4's `exit_populations` reads (confirm with F4b), and F5 re-walks picked and shown chains for the rest: `spool::load_path_facts` stays the reader. (3) **Spool cut (b):** phase A's plan is Rs 15, 10, 5 lakh x WITH and FADE; Rs 7.5 and 2.5 lakh are a delta file for records on stock-days below 5x only. A half step on a 5x stock-day has no row, so F4b refuses it by name, `half_step_not_walked`: fewer trades in a deep drawdown, never kinder by size. (4) A plan per family, no typed 60 and 50. (5) Print bytes a record at phase A |
| **F4b** (`gen/**`, `search/**`, `wf/**`, `luck.rs`, `twin.rs`) | (1) **`Ceilings::B.margin` is Rs 2,00,000** (REQ 29), today `MAX_MARGIN_PAISE`; a test at leverage 3: main Rs 7.5 lakh, B Rs 5 lakh. (2) The size step of 25.1: no equity term; tests at N = 10, 20 from Rs 19.99 lakh. (3) `Stage` from `combos.json`'s `ORDER`, every row; `stage_shape` for each; the plan equals `perCell` on every key (`tf\|fam`, `A\|tf\|fam`, `C\|tf\|fam`), total 197,480,655,060; the two typed checks in `search_tests.rs` follow. (4) The named cut: skip by `combos.json`'s `cuts`, logged, achieved figure reported. (5) `gen/nodes.rs`: E3's table and A's (no rank, no S35); `gen/library.rs`: ORB-POS, PAT, A's library. (6) EU: `search/eu.rs`, a merged view over member group files, `Candidate` members, `Rid`; a test that first signal wins, one position per stock, ceilings shared. (7) Scopes: 44 and 47, `Scope::{MaxNet, GlobalAll}`, chains 127, per-account `WaveOut`. (8) `WaveMode.acct`; unit keys carry the account; `Ceilings::A` with `WalkKey.amount` = premium, no half step, margin = premium. (9) One luck stream and `tried_total` across accounts and runs; `best_b` merges by maximum, byte-equal in any order; the twin the same. (10) GLOBAL-ALL's fold by `cmp_ranking`, reach as 24.P1 Fix 1; MIX-WF-1 on it. (11) `combos/extension-order.tsv` as the later order; `X.triples.base`. (12) The month-full jump of the 12:4x note |
| **F5** (`stage2.rs`, `gatek.rs`, `report.rs`, `wiring.rs`), building now | As assigned: P-08 to P-19, 24.P1, N7, N8, F-15 to F-26, R5, R6, 24.4-24.5, ledger. **Adds:** (1) the verdict reads GLOBAL-ALL only; line 3's account sentence; tests of each case. (2) Part 2: the three accounts side by side, no total row; MAX-NET and the highest net line, on every report; ceiling B beside; the year table per account. (3) D12's lines; lifted labels by span (D5, D6, D7, D8); survivorship stays first. (4) The plain check from the new `combos/sniper-arith.json`. (5) Stage 2's new rows `S2.new.*`, `S2.controls.24U`, the control seeds of 24.U; `S2.shown` in 25.2.7's data-blind order; `S2.shown.data` (D9, D10). (6) highprob-check1's nonblocking rulings of 25.4. (7) Counts per account; "chains formed: x of 127"; both figures of requirement 17a at every checkpoint. Option columns print as (B)'s R5 columns |
| **F6, new** (`src/index.rs`, `src/index/{spot,options,route,replay,surv}.rs`, feature `f6`, target `W/tgt/fib-preview-F6`, `tests/f6.rs`, `benches/f6.rs`) | **Account (A):** (1) vendor by copy, with sha256 in `VENDORED.tsv`, from `W/scratch/psrb-preview-idx-1/crates/idx_opt_preview/src`: `spot.rs` (`fold_missing`, `DaySecs`), `options.rs` (`load_chain`, `load_day`, `mark_holes`, `select`, `walk`, `rates_on`, `charges`, `freeze_of`, lots): the tree is uncommitted, so the digest is the identity; label "index-options walk: check 3 not finished". (2) A phase A that writes ordinary group files: `Sym` for the two names, F2b's fold with `rank_gate` off, each path an option round trip through `walk`, `PathRow` in the same 28 bytes (`deployed` = premium paid, `var_margin_bp` = 10,000), `WalkKey.amount` = premium; so F4's search reads (A) as any unit. (3) The contract rule's 9 values as delta walks on the seeds' records. (4) `rates_on` to the official reading; a test on REQ 25a's Rs 3,15,315 screen within Rs 1. (5) NOTE 9's spreads: measure from the tick store's quotes where they exist; one labelled wider lane on shown chains; label what is unknown. (6) D1 on (A). **Account (C):** (7) a merged view of (A)'s and (B)'s files, ceilings shared, premium and margin within free cash; `C.W0.*`, `C.W1.*`, `C.mix` through F5's seam. **The replay:** (8) 25.2.7, vendoring `psrb-preview-opt-1`'s `choose.rs`, `walk.rs`, with overlay check 4's B1 answered and 24.13's rules; it reads F3's `DayWalker::detail` decision times. **F6-SURV:** (9) D9's lane, after shown chains exist; never blocks a run. **Tests:** 2 indices end to end; no rank clause; options only; whole lots; premium at or under the seed; no decision before a contract's print; a trade count across accounts on (C); no cash route on an index |
| **INTEGRATE** | Now: as told at 12:3x. **INTEGRATE-2, after F6:** seven test names (ANANDRATHI, FORCEMOT, ICICILIFE, IRFC, RELIANCE + TATACONSUM, IOC) and NIFTY, BANKNIFTY, wave 0 only. It proves: each ceiling on each account (5th taken, 6th refused; 100th, 101st; ceiling B's Rs 2 lakh margin at low leverage); no E3 decision inside its range; ORB-POS absent before close; PAT on the arming candle; EU's first signal wins; (A)'s options only; D1, D2, D3, D8 by name; S never reads file S; the verdict's chain is GLOBAL-ALL; chains at most 127; achieved counts against `combos.json`; no removed term and no banned column anywhere. It prints per stage records, kept steps, mean and p99 steps an evaluation, rate, bytes a record, and the projection in hours and GB, with what still grows with kept records named |

**The disk (a point below).** F2's measure and E3's guess give about 150 + 30 + 2 million records. As written, at 990 bytes a record and 60 paths x 40 bytes, about 590 GB. With F3b's two cuts about 390 GB. Free 1,409 GiB at 13:10 IST, the floor `FLOOR_KIB` + `MARGIN_KIB` is 1,074 GiB, so the room is about 335 GiB, 360 GB, and falling. (A) needs about 7 GB and does not wait. (B) does not fit by a small margin, and its figure is a guess until INTEGRATE-2 prints bytes.

### 25.7 Rulings taken here, and what stays open

**Rulings (the main session's to overrule before the seal; full automation: none waits on the operator):**

| # | Ruling | Why |
|---:|---|---|
| 1 | The size step has no equity term (25.1) | 24.U's own formula; the cliff at N = 10, 20 is no broker's rule |
| 2 | **SNIPER caps 40, 60, 100: not taken** | 24.U adds them "only if the chain count stays small", and it no longer does: 21 cells and more per account. GLOBAL, MAX-NET and TPM40 already range over denser recipes; the SNIPER family stays about few trades. What it costs: no SNIPER chain holds a recipe above 20 trades a month, so steps 2-3 of the walk may decide most windows (check 3's nonblocking 6). The count would move only stage 2's at-most figure |
| 3 | 24.P1 Fix 4 (a) and Fix 3's N seed stand | the repair's reasons hold; (a) is one narrow exception to REQ item 6 |
| 4 | Read raw, not refuse, on offset spans (D3) | no look-ahead by a table of later adjustments |
| 5 | D1 refuses on all three accounts | one rule; the option seconds of those days are unmeasured |
| 6 | Line 2 reads GLOBAL-ALL (25.2.6) | REQ 24a over REQ 30's accounts: one reading |
| 7 | The named cut, and its order | cost only (25.5) |
| 8 | Ceiling B on (A): seeds, base and level 1 only | one premium ceiling there, only the day cap differs; 0.6 hours against 2 |
| 9 | Spool cuts (a) and (b) | no count moves; (b) is never kinder by size |

**Needs the operator, one point: the disk.** Phase A of (B) needs about 390 GB after the spool's own cuts against about 360 GB of room above the 1.1 TB + 50 GiB floor, and falling. `W/tgt` holds 328 GiB of build folders, and Claude never deletes. **Recommendation: the main session hands him a delete list of dead build folders to type DELETE, as on 31 Oct.** If he does not, (A) still runs, and (B) waits. Nothing else here waits on him. The floor question with table D (24.R, SNIPER.md 5.4) stays his to overrule at any time, and it is not needed before a run.

**Owed by the main session before the seal:** the independent check of this section and of 24.P1 (an independent recount with it); D7's one-file copy; telling F5 and INTEGRATE the new lines (appended to `build/NOTES-shared.md`, which they read); the new seal: spec first 1,208 lines 89a00be888ae5f89 are unchanged, and the seal covers sections 23-25, `COMBINATIONS.md`, `combos/*`.

**Outside this folder, unchanged:** R3:495 and R3:603 (round 3's own lines); RECONCILE.md's and DATA-EVIDENCE.md's 10 wording repairs (the proof step's); round 3's own readers for the same data rules.

**Not verified.** Nothing new here was compiled or run. E3, ORB-POS, PATTERN-AT-TOUCH, EU, accounts (A) and (C) exist only as words and counts. The model hours, the selectivities, E3's 20,000 and (A)'s 2 / 10, the 30 million records and the 390 GB are guesses until INTEGRATE-2 prints them. The plain check is at a Rs 1,000 example share with no slippage. `check.mjs` shares an author with `count.mjs`. The index tool's check 3 is unfinished and overlay check 4 failed on 7 points, so (A)'s and the replay's fills carry those labels until repaired or re-checked. Nothing here is a result, and nothing reaches real money without the operator.

### 25.8 Requirement trace (the operator's words as they stand; the newest word wins)

| Requirement | Where it binds now |
|---|---|
| 1a, the fixed Rs 2,50,000 | 25.1; 24.R; 24.P R1 |
| 16, positive side up, negative side down, fewest trades, the Pareto set | 25.1; 16.6, printed per account and over all; R5 |
| 16a, 16b, eagle, cheetah, sniper | 24.1-24.5, 24.P; on (A) too, S-RR reading spot price distance there, labelled "price distance, not P&L", since an option's net at T is unknown |
| 17, at least 100 billion; 17a, both ways of counting | 25.5: 196,294,908,660 committed; the extension's fixed order to 100 billion counted once |
| 18, no futures | 25.1; `col_table()` holds no such column; F2b's and F6's tests |
| 19, resources | the four downloads, READY = PASS: 25.3 D5-D9. No credentials, no bot-check bypass, no deletions, no trades |
| 20, one route per signal | 25.1; 25.2.5; 25.2.7 |
| 21a, net is the centre | 25.1; 25.4's table; `combos/sniper-arith.*` regenerated with net words |
| 22, 23, 23a | numbers replaced by 25 and 25a; their rules stand: one trade = one entry through its exit; the 6th signal and the 101st refused by name; no look-ahead |
| 24, 24a, "Let the data decide this" | 25.2.6: the union now spans the accounts; 24.P's gate, SNIPER and SNIPER-HP stay labelled beside it |
| 25, 25a, the ceilings | 25.1 |
| 26, charges as real Dhan | 25.1; 24.V; 25.4's plain check |
| 27, ORB | 25.2.1, 25.2.2 |
| 28, one strategy, every way in | 25.2.3, 25.2.4 |
| 29, the second ceiling | 25.1; 25.5's `CB.*`; F4b's margin line |
| 30, three accounts | 25.2.5, 25.2.6, 25.6's order |
| "our aim is to achieve the maximum profit" | 25.2.8 |

### 25.9 Small corrections to this section, found on its read-through (same pass; append only; these rows win)

| Line | Reads now |
|---|---|
| 25.2.1, E3C | "while the last close at or after the range's close was not" reads "while no close since the range's close was above `H_d`, or since the last close at or below it" |
| 25.2.5, (A)'s seeds | Wave 0 on (A) is 25.2.1's and 13.1's seeds with the top-mover clause off and printed so; no other seed changes |
| 25.1, 25.5, `CB.S2.shown` | F4's `WaveOut.ceiling_b` re-runs every chain formed at ceiling B. `CB.S2.shown`'s 12 are the shown ones; the others print as achieved, are in `tried_total` and the luck stream, and move no pick |
| 25.3 D8, labels | The official list holds for sessions 2018-09-04..2026-10-30 (fno-history READY section 8); F5 lifts S05's label for those only |
