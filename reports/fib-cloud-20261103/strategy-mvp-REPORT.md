F&O STOCKS BACKTEST: HOW MUCH WORK IS LEFT? (review, 30 Oct 2026, about 20:00-20:30 IST)

NOTE: the harness refused to let this step write REPORT.md. This text is the report, to be saved at /Volumes/WD_BLACK/brutex/fresh-20260919/work-20260925/state/strategy-mvp-20261030/REPORT.md.

How this was checked: I re-checked the three inventories (code, data, trial run) and the gap report against the code, the store, the census and the live app, and corrected what was wrong. Everything was read-only: git reads, file reads, and GETs to :8080. I started no server, built nothing, wrote nothing under runtime/ and committed nothing.

1. THE SHORT ANSWER

No, this is not many days of heavy coding. The data is in place and most of the engine works. One piece is missing: filling trades on GDFL seconds. It needs 7 new parts and only 2 of the 110 parts already planned.

- Writing the code once: about 18-34 agent-hours, which is 1 to 1.5 account-days.
- Merged under the repository's rules (full coverage, check rounds, one landing at a time): likely Fri 6 to Sun 8 Nov, worst Mon 16 to Tue 17 Nov.
- Early research numbers on GDFL seconds, from an unmerged branch on a shadow store, for a few stocks: likely Sun 1 to Mon 2 Nov. They would be unchecked and not final.
- The real long pole is computer time, not code. A full brute-force sweep of every timeframe, every F&O stock, 2018 to today, is weeks of machine time even after the code exists. The first run should be scoped down, for example the 15-min timeframe over every stock, which is about 2.5 to 3 days on one store.
- Tonight: no GDFL-second backtest can run, because that code does not exist. What can run is in section 6.

2. WHAT ALREADY WORKS (checked during this review)

Your five points:
- Point 1, F&O stocks only, cash data: works for the 208 stocks on today's list (core/instrument.rs:354-372).
- Point 2, Zerodha signals on the eight 1-60 min timeframes: works. The engine searches combinations of 370 conditions (cli/lib.rs:10213-10215). In the trial it tested 2.56 M combinations in 3.9 s.
- Point 2, the 1-day timeframe: missing. It is refused on purpose, because a daily bar is known only after the close (lib.rs:10219-10225).
- Point 3, fills on GDFL seconds with one worst-price model: missing.
  - Every fill uses Zerodha 1-minute bars of the same feed (lib.rs:7920, 4661-4672, 18779-18789; runner/trade.rs:141).
  - Each trade is priced twice, best and worst.
  - Targets fill at their level (grid.rs:5466-5487).
  - The worst-price function itself exists: costs/fill.rs:336-357, PrintedExtreme.
- Point 4, membership by date: the library exists (pull/fno_membership.rs:2585) but no sweep calls it. The sweep uses today's fixed 208-name list.
- Point 5, 3 Sep 2018 to today: works as arguments; no start date is hard-coded.

Other pieces:
- Results ledger, /backtest.json, POST /backtest/run and the backtest page: work end to end (trial, on a shadow).
- Charges: the library exists (costs/stack.rs, costs/ledger.rs) but nothing calls it. Every result says it is GROSS OF EVERY CHARGE (runner/audit.rs:441-460).

The data:
- Zerodha cash: 208 stocks, all 9 timeframes, every month full from Jan 2018 to Sep 2026. Oct 2026 holds 1-29 Oct on the minute timeframes and 1-30 Oct on the 1-day one.
- GDFL cash seconds: 343 stocks, every month full from Sep 2018 to Sep 2026, plus 1-29 Oct. That is 3.85 B rows, and every F&O stock ever listed has a folder.
- F&O membership by date: builds in 13.7 s with an existing command, but is not in the live store (no runtime/store/reference).
- The price factor and the tick checks: exist only in scratch. /feed-checks.json and /tick-checks.json both answer 404.

The unmerged Fib branches:
- FIB-5 is a GDFL worst-price walker and FIB-8 a reader for Zerodha minutes and GDFL seconds.
- Each report says STATUS: DONE. The builders wrote that themselves, and nobody has run it on this Mac.
- They are 603 and 539 commits behind, with 19 and 23 conflict blocks.
- They are good references for the tests, but cheaper to rewrite small than to merge.

3. WHAT IS MISSING: 7 PARTS, PLUS 2 OF THE 110

Each part below lists the time to write the code once, then the likely / worst number of check rounds.

1. FNO-LAW (2-4 h, 2/5): change the rules file (CLAUDE.md §1) so the swept stocks are the dated F&O members, GDFL seconds are the fill source, and there is one fill model.
2. FNO-SEC-EXEC (4-8 h, 3/6): the missing heart. It steps second by second: entry at the chosen GDFL second, then stop, target and time exit. A buy pays the second's high and a sell gets its low.
3. FNO-DAYMASK (2-4 h, 3/6): trade a stock only on F&O-member, tradable days with GDFL seconds, and count every skipped day.
4. FNO-SEC-WIRE (4-8 h, 3/7): connect part 2 to the search, the ledger, the API and the page, and record "fills from GDFL" in each run's identity.
5. FNO-LEAN (2-3 h, 2/5): a run mode that does not write millions of evidence files.
6. FNO-NET (2-3 h, 2/5): size each trade at the quantity closest to Rs 5 lakh and subtract real charges using the existing library.
7. FNO-1DAY (2-4 h, 2/5): decide on day D's daily bar and trade day D+1 intraday.
8. FNO-PORTFOLIO, your money rules (4-6 h, 3/6): at most 20 open positions, at most 100 trades a day, Rs 20 lakh capital.

Parts 1-7 cover your five points; part 8 is the money rules.

Of the 110 planned parts, this needs only 2:
- BASE-REDS-1023: nothing merges before it. Its 4th check found a new blocking item at 18:53 tonight.
- BASE-STACK-1028: it has passed and is waiting to land.

Of the rest: 2 help (TESTTIME-1025, BASE-BENCH-1028), 1 overlaps and needs a decision (PREC-15), and 105 are not needed (ingest, /db pages, audits, options and coverage backfills). gap.md names all 110 exactly once; I checked that against lists.json.

4. THE ESTIMATE

These come from measured step times, past check results (22 of 57 merged parts passed their first check) and the landing rule. The calendar uses one account a day and none on Wednesdays.

| Milestone | Likely | Worst |
|---|---|---|
| Your decisions | tonight or Sat 31 morning | Mon 2 Nov |
| BASE-REDS-1023 lands | Sun 1 - Tue 3 Nov | Thu 5 - Fri 6 Nov |
| First research numbers on GDFL seconds (unmerged) | Sun 1 - Mon 2 Nov | Tue 3 Nov |
| Parts 1-6 merged | Fri 6 - Sat 7 Nov | Sat 14 - Mon 16 Nov |
| Part 7 (1 day) merged: all five points done | Sat 7 - Sun 8 Nov | about Tue 17 Nov |
| Money rules merged | Mon 9 - Tue 10 Nov | Thu 19 - Fri 20 Nov |
| Scoped first run done | 1-3 days after deploy | later |
| Full run (every timeframe, every stock, 2018 to today) | weeks after deploy | longer |

Why it is a week and not two days:
- The code itself is about 1 to 1.5 account-days.
- Each part needs about 2-3 check rounds, because only about 1 check in 3 passes the first time.
- Every merge takes a 4-5 h landing slot, one at a time.
- Nothing can merge until BASE-REDS-1023 does, and nothing has merged since 27 Oct 08:03.
- Past estimates for this same goal slipped by a week or more.

Why the full run takes weeks (measured tonight on a shadow):
- RELIANCE on the 15-min timeframe for all of 2025 took about 3 min to search. It then began writing about 3.4 M evidence files, about 96 GB, for that one stock-year, and was stopped.
- The 1-min timeframe was still in its first phase after 26 minutes.
- The 208 stocks Zerodha holds give about 1,156 member stock-years. Every timeframe over all of them is at least about 1,100-2,200 machine-hours, and probably more.
- One store runs only one sweep at a time.

5. TWO THINGS TO KNOW BEFORE TRUSTING RESULTS

(1) GDFL records about one price per second.
- On RELIANCE 28 Mar 2025, only 15,833 of the day's 22,500 seconds have a trade, and only 91 of them (0.57%) have a high different from the low. The trial found 0.51% for all of March 2025.
- In the 2025-26 samples, Zerodha shows minute extremes beyond every GDFL second in 26-47% of minutes.
- So "the worst price inside the second" is nearly always the single price GDFL recorded. It is about 7.7 bp per round trip gentler than today's worst-of-minute reading, and it is optimistic against the true worst.
- This is a limit of the data, not of the code. Every report should say it.

(2) About 22.5% of F&O member-days have no Zerodha data.
- 131 former F&O stocks have none: 82,762 of 368,298 member-days, and 39.8% of them in 2018. Examples are EXIDEIND, IGL, ACC, MRF and HDFC.
- 102 are in today's Kite list and could be pulled. 29 are delisted.
- Using today's 208 stocks only tilts the results towards stocks that survived.

6. WHAT CAN RUN TONIGHT

Nothing here touches the live store without your yes.
- Answer the decisions in section 7.
- On a fresh shadow store, using app29's cli: build the membership list (13.7 s) and the price factor (about 10 min).
- A read-only re-pricing study with your exact rule for 10-20 stocks over several years, reading /bars/window.json (about 20 s per stock-month). It shows how far the results move; it is not the engine.
- Time the search on the other timeframes on a shadow, stopping before the evidence phase.
- Write the specs for parts 1-8, so the next relaunch builds them first.
- With your yes: start the Zerodha pull of the 102 former F&O stocks. The Kite token is valid until 06:00 on 31 Oct. Its duration is unmeasured.

Not tonight:
- Any GDFL-second backtest. The code does not exist.
- Long range-rung or range-all runs with today's run mode. The evidence files fill the disk.

The planning runs have been paused since 18:13, and the week's account ends around 22:00-23:00. That leaves only a couple of agent-hours tonight; machine jobs can run overnight.

7. DECISIONS ONLY YOU CAN MAKE

- New from this review: stops and targets as a percent of the GDFL entry fill? The engine already measures them from the entry price (grid.rs:4883-4898). If you agree, no Zerodha price has to be converted to a GDFL price, so the price factor is not needed to trade, 3 Sep 2018 does not have to be dropped, and decision h goes away.
- (a) Entry at the first traded GDFL second at or after the Zerodha candle closes? Default: yes, with no clock shift.
- (b) Targets fill at the worst price of the second, not at the target level? Default: yes.
- (c) Exits: stop x target x time, no trailing? Default: yes.
- (d) Intraday only, out before 15:10? Default: yes.
- (e) Does the whole order fill in one second, or only up to that second's traded volume? Default: the whole order, stated in every report.
- (f) Pull the 102 former stocks and name the bias of leaving out the 29 delisted ones? This needs a CLAUDE.md §1 change.
- (g) Should one setup mix two timeframes? Default: no; it is the most expensive option.
- (i) Skip the stock-days with missing GDFL seconds (30 May 2019 morning; 15-16 Jul 2024) and count them? Default: yes.
- (j) Narrow PREC-15, so there are not two fill authorities.
- (k) Is your "proven data first" condition met by the read-only data audit now running (wf_40fc5064-a9d), or must a merged proof gate come first? If a merged gate must come first, every date above moves by weeks.
- The scope of the first run.

8. CORRECTIONS MADE BY THIS REVIEW

1. Part count: it is 7 parts for the five points, not 6 (1 day is in point 2), and 8 with the money rules.
2. Evidence file count: 561 x 242 x 4 = 543,048, not 543,610. The rest are 561 tier files and one catalog.
3. g01 searched to depth 18, not 19, testing 16.66 M combinations.
4. g02: 14 probe attempts finished, not 15.
5. Parallel runs: one store runs one sweep at a time (execution_lease.rs). Running several at once needs one shadow per run; on one store, 15-min over every stock is about 60-77 h.
6. The full-run machine-hours figure is a floor, not a range.
7. Stock-years: 1,156 for the 208 stocks that can be swept now, not 1,490.
8. The price factor is optional if levels are a percent of the entry fill.
9. GDFL's one-price-per-second fidelity is measured and stated (section 5).
10. 131 former members lack Zerodha data, not 135. The other 4 folders are TATAGLOBAL and 3 October entrants.
11. Writing the code takes a few days, but merged under the rules it is about 1 week likely and about 2.5 weeks worst. The two line estimates (2-3k and 10-17k) are unmeasured.
12. The landing time is not yet measured, and the coverage-step time rests on only 2 samples.
13. The 1-day 19/20 against 20/20 difference is two census versions, not an error.

Confirmed as written:
- Every cited code line.
- The data counts.
- The 404s.
- The trial numbers: 2,444.6 s and 2.98 GB; 31 of 31 fills; 7.71 bp; Rs 20.45 rising to Rs 40.85; 78 NIFTY rows with 0 trades on :8080; runs.bin unchanged since 13 Oct.
- The plan facts: last merge 27 Oct 08:03; all 110 parts named once; BASE-STACK-1028 passed; BASE-REDS-1023 blocking item V4-2 at 18:53.
- Nothing from the trial or gap runs is still running: only the operator's :8080 is.

9. NOT PROVEN

- That GDFL holds every exchange trade.
- The real number of check rounds for these parts.
- That the Kite pull of former members works, and how long it takes.
- The search time on the 2, 3, 5, 10, 30 and 60-min timeframes.
- That FIB-5 and FIB-8 pass their tests on this Mac.
