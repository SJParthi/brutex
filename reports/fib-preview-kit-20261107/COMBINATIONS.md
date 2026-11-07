PREVIEW FIB: the operator's real Fibonacci strategy (SPEC.md section 1 with the second-touch entry); not the production fib backtest of Thu 12 Nov; not a Winner Gate verdict
AT LEAST ONE RECIPE MEETS RS 2.5 LAKH EVERY UNSEEN MONTH: PENDING
current F&O members only (survivorship bias not removed)

# COMBINATIONS: the table requirement 17 demands (written Sat 7 Nov 2026, before any run)

Nothing has been run. Every figure here is computed by `combos/count.mjs` with node BigInt from the dial tables in that script; no market data or result was read. Re-run: `node count.mjs`, `COMBOS_FULL=1 node count.mjs`, `COMBOS_W=1 node count.mjs`, `node render.mjs` in `state/fib-preview-20261106/combos/`. Exact figures per stage, account, timeframe and family are in `combos/combos.json`: the search asserts each achieved figure against that file.

**What changed since the plan of 24.9 (guaranteed 145,276,706,220; expected 146,279,509,161; each generated recipe once 23,247,542,760; 84.3 model hours; combos.json 26d3b741cc2662e2, this table 3ee9a5fcd4ddc7ac):** requirements 27, 28, 29 and 30 added, as new stages only: E3 (1,303,923,420), ORB-POS (336,718,080), PATTERN-AT-TOUCH (648,002,700), EU (38,857,440), ceiling B (2,290,617,600), account (A) (47,584,347,840) and account (C) (1,481,760). Every earlier stage keeps its exact count, cell by cell (`check.mjs` compares with `combos.json.bak-final` and `combos.json.bak-sniper`), and every earlier stage 2 and adaptive row too. New table 197,480,655,060; a named data-blind cut of 1,185,746,400 (`W2.singles.level1` on 1, 2, 3, 5 minutes, 13.97 model hours) leaves 196,294,908,660 committed. New per-recipe figure 31,559,317,908; new model hours 98.5, under the operator's 100 by the cut. The definitions are in PREVIEW-SPEC section 25.

**What changed since the fingerprint daf6c43c74b7 of 09:08 IST (guaranteed 131,840,470,560; expected 132,807,836,841; each generated recipe once 21,222,014,340; 79.1 model hours; combos.json 596e5cebf30d, this table 2139d0c8f8f0):** requirement 16b, the Sniper view, added two entry gates as seven new stages `W2.sniper.*` (13,436,235,660 more), six stage-2 rows for the 12 SNIPER cells (at most 10,035,060 more) and one adaptive row (at most 25,401,600 more). Every earlier stage kept its exact count, cell by cell; only the fixed run order changed, each new stage running right before its mirror. The definitions are in `SNIPER.md` and PREVIEW-SPEC section 24.

## The numbers

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

## In plain words: what is varied

"Combos" is the exact number of evaluated combinations of the table in which that dimension takes a value other than its seed, over all three accounts. A combination varies several dimensions at once, so this column does not add up to the total.

| Dimension | What it means | How many values | Combos |
|---|---|---|---|
| Timeframes | the candle size the recipe reads | 8: 1, 2, 3, 5, 10, 15, 30, 60 minutes | all; per timeframe below |
| Accounts | which names, which route | (A) NIFTY and BANKNIFTY spot signals in index options only; (B) F&O stocks, cash; (C) both in one account, ceilings shared; never added together | (A) 47,584,347,840; (C) 1,481,760 |
| Top-mover pools and times | how high the stock ranks among F&O stocks today, and when | top N 6 (1, 3, 5, 10, 25, 50); state and clock 19 (in top N; entered within 1, 2, 4, 8, 16 candles or the session; 1st, 2nd, 3rd+ entry; in top N at 15, 30 minutes and at the seven eighths of the session); move basis 2; move size off + 9 generated bands; stale limit carry + 10; (A): off, labelled, except the move size against the index's own usual | 15,720,726,324 |
| Ladder anchors | which range the Fibonacci levels are drawn on | entries: running day range, frozen day range (2). Beside them, position bands on 7 other ladders (swing k 2, 4; opening range 15, 30; yesterday; last 5 sessions; opening gap), 9 generated bands each | 39,491,550,384 |
| Levels and extensions | which Fibonacci rung | 12: 0, 0.236, 0.382, 0.5, 0.618, 0.786, 1, 1.272, 1.618, 2, 2.618, 3 | 52,039,261,368 |
| Direction and same-candle | how the leg is read when one candle holds both extremes | 2: engine, V-BOTH (V-SEC not run, DV-15) | 628,074,348 |
| Touch number and near distance | first, second or later touch, and how close counts as a touch | clause 5 (= 1, = 2, = 3, >= 2, >= 3; higher numbers in the fine stage); on a flag 4 (none, = 1, = 2, >= 2); distance 0 + 6 generated | 10,480,988,796 |
| Bounce and gaps | how far price must leave the level, and how many candles may pass between touches | bounce 5 rungs + 6 generated; gap at most off + up to 9; gap at least off + up to 8; candles to the bounce off + up to 9; close beyond 2; re-read 2 | 5,022,951,468 |
| Flag and box | the pause shape | candles in the box 8 (1, 2, 3, 4, 6, 8, 12, 16); box height any + 18 generated; holding rule 16 | 59,086,752,096 |
| Pauses and push | how big the move before the pause was, which pause of the day | push 4 measures x 9 generated + any; pause number 4; day range so far off + 9 generated | 18,584,736,840 |
| Volume | is volume drying up or arriving | pause against push, off + 9 generated; trigger volume 2 readings x 9 generated; (A): none, an index has no volume, refused by name | 10,796,873,220 |
| Trigger shape | what the entry candle looks like | colour 2; strength off + 9 generated; body, wicks, close location, range on the last 3 candles 15 x 9 generated; candle-code sequences 14,424 | 83,485,344,192 |
| Entry rule, including the second touch | flag, touch or opening range, on the close or the first second beyond; or several in one recipe | 8 families: E1C, E1S, E2C, E2S, E3C, E3S, EUC, EUS | per family below |
| Opening range (E3) | the break of the first 1, 2, 3, 5, 10, 15, 30 or 60 minutes' high or low (requirement 27), a decision only after the range has closed | durations 8, all seeds; width off + 18 generated; which break 4 (first, second, second or later, any); within g candles of the range's close off + up to 9 | 256,308,732 |
| Opening-range position (ORB-POS) | a filter on the flag and the touch: where the decision sits against each range | 8 durations x (above, inside, below + 9 generated distance bands); coarse 3 + 3 | 658,888,704 |
| Pattern at the touch | named candlestick and at-level patterns, anatomy and candle codes on the touch candle and the one before it, crossed with touch = 1, = 2, = 3, >= 2, >= 3 (requirement 28 (a)) | 54 names; 405 anatomy pairs; 600 codes | 1,677,891,564 |
| One recipe, every way in (EU) | at most one flag, one touch and one opening-range member on one account, first signal wins (requirement 28 (b)) | 234 member sets; dials on all members together | 81,711,552 |
| Second ceiling (ceiling B) | 10 a day, 100 a month, Rs 10 lakh, a labelled account (requirement 29) | waves 0-1 of (B); seeds, base and level 1 of (A) | 2,465,905,920 |
| Contract rule, account (A) | which option contract a signal buys (OPERATOR NOTES 5-7) | current expiry; top volume over 5, 15, 30 minutes; then moneyness ATM, 1 ITM, 1 OTM; then premium, whole lots | 826,368 |
| Exits and targets | where the stop sits and how the trade ends | 15 stops x 51 to 60 lanes (target rungs, day-high break, trailing, time, condition, Supertrend, break-even, bounce high) | 117,345,472,860 (not at XA with the seed stop) |
| Sides, including FADE | long, short, both, and trading against the signal | 6: LONG, SHORT, BOTH, FADE-LONG, FADE-SHORT, FADE-BOTH | 98,740,327,530 (FADE) |
| Filters: singles, pairs, triples, deeper | how many context conditions are stacked, the plan of 24.9, account (B) | up to 640 single filters (161 to 297 coarse); deeper by forward selection | singles 23,590,782,120; pairs 10,943,547,960; triples 95,027,264,640; deeper at most 774,144,000 |
| Sniper gates | a small stop and a big first target, read at the decision; no shot if they are not there | stop distance (S-STOP): any (seed) + 9 generated bands of the stock's own past, own p25 low and own p50 low among them; reward to the first target over the stop (S-RR): any (seed) + labelled at least 2, 3, 5 + 9 generated bands, refused by name on XG, XH, XI; both labelled, both learning months only | 17,532,091,716: of the plan of 24.9, singles 943,451,820; pairs 898,707,960; triples 11,594,075,880; deeper at most 25,401,600 |
| Time windows | when in the day an entry may happen | 36 windows on the session's eighths; one candle where winners crowd (fine stage) | 17,775,856,980 |
| Day of week and month | calendar | 5; 9 | 13,856,992,740 |
| Days to expiry | sessions to the stock's monthly expiry; on (A) the index's current expiry | 20 bands of 0, 1, 2, 3, 4, 5+; expiry week 2 | 8,371,109,292 |
| Trade order | the stock's and the account's k-th trade, after a win or loss | 11 | 12,361,250,292 |
| Index context | what the market is doing | 5 F&O index moves x 9 generated; NIFTY, BANKNIFTY EMA-20 side 2 each; INDIA VIX level, change x 9 generated | 44,101,928,472 |
| Multi-timeframe | the last closed higher-timeframe candle | per higher timeframe: colour 2 + position 9 generated (0 on 60 minutes, 77 on 1 minute) | 13,483,065,972 |
| Indicators (as filters only) | round 2's 42 readings, ranked against the stock's own past; 33 on (A), which have no volume, rank, basket or breadth | 42 x 5 bands on 3, 5, 15, 30, 60 minutes; UNAVAILABLE on 1, 2, 10 | 124,344,903,492 |
| Universes | which stocks | liquidity, volatility, price x 9 generated; 18 NSE industries (survivorship: today's map); on (A) NIFTY only, BANKNIFTY only; index members UNAVAILABLE (no dated list), never guessed | 32,219,637,672 |
| Positions | most positions open at once | (B) 5 seeds (1, 3, 5, 10, 20), 8 refined (2, 4, 7, 15, 6, 8, 12, 17) in stage 2; (A) 1 or 2, one position per name, x 4 premium seeds | (B) every combination carries one: exactly 29,978,965,092 at each seed |
| Account controls | day loss, month loss, month lock, trade caps | 3,072 seed grid (day cap 1, 2, 3, 5; month cap 5, 10, 20, 40, 60, 100) + 13 generated levels; tie-break 4; admission 2 | 36,774,675 at most |

## Where the count sits

| Timeframe | Table | Share | Every timeframe, extension in | Model hours |
|---|---|---|---|---|
| 1 min | 2,254,836,468 | 1.1% | 44,195,594,532 | 21.2 |
| 2 min | 2,142,545,106 | 1.1% | 41,768,606,742 | 10.5 |
| 3 min | 5,871,741,060 | 3.0% | 104,243,786,652 | 13.2 |
| 5 min | 15,535,106,118 | 7.9% | 100,134,047,058 | 14.2 |
| 10 min | 7,452,440,670 | 3.8% | 35,240,307,342 | 6.7 |
| 15 min | 50,519,866,074 | 25.6% | 91,753,312,962 | 21.1 |
| 30 min | 57,562,885,458 | 29.1% | 87,718,205,022 | 15.4 |
| 60 min | 56,141,234,106 | 28.4% | 83,804,295,426 | 10.1 |

**The count is uneven on purpose, and this matters:** 83.2% of it sits on 15, 30 and 60-minute candles; 1 and 2-minute candles, the operator's own charts, get about 2.1-2.3 billion each. An evaluation's work grows with the records it keeps, and by the model's guess a 1-minute unit holds about 60 times the records of a 60-minute unit, so the deep stages run where they cost least. This was fixed by cost only, before any run. The rest is the extension below.

| Account | Family | What it is | Table |
|---|---|---|---|
| (B) | E1C | E1 flag, close beyond the box | 39,806,551,470 |
| (B) | E1S | E1 flag, first second beyond the box | 39,806,551,470 |
| (B) | E2C | E2 touch, return candle's close | 34,436,158,320 |
| (B) | E2S | E2 touch, second reading | 34,489,797,420 |
| (B) | E3C | E3 opening range, close beyond it | 657,158,670 |
| (B) | E3S | E3 opening range, first second beyond it | 657,158,670 |
| (B) | EUC | EU one recipe, every way in, close readings | 20,724,720 |
| (B) | EUS | EU one recipe, every way in, second readings | 20,724,720 |
| (A) | E1C | E1 flag, close beyond the box | 13,164,450,420 |
| (A) | E1S | E1 flag, first second beyond the box | 13,163,759,220 |
| (A) | E2C | E2 touch, return candle's close | 10,354,794,720 |
| (A) | E2S | E2 touch, second reading | 10,407,405,888 |
| (A) | E3C | E3 opening range, close beyond it | 226,899,180 |
| (A) | E3S | E3 opening range, first second beyond it | 226,776,300 |
| (A) | EUC | EU one recipe, every way in, close readings | 20,131,056 |
| (A) | EUS | EU one recipe, every way in, second readings | 20,131,056 |
| (C) | E1C | E1 flag, close beyond the box | 476,880 |
| (C) | E1S | E1 flag, first second beyond the box | 476,880 |
| (C) | E2C | E2 touch, return candle's close | 157,290 |
| (C) | E2S | E2 touch, second reading | 157,530 |
| (C) | E3C | E3 opening range, close beyond it | 106,590 |
| (C) | E3S | E3 opening range, first second beyond it | 106,590 |

| Group | Combinations | Model hours (before the cut) |
|---|---|---|
| the plan of 24.9: E1, E2, gates, account (B) | 145,276,706,220 | 84.3 |
| E3, account (B) | 1,303,923,420 | 3.3 |
| ORB-POS, account (B) | 336,718,080 | 1.7 |
| PATTERN-AT-TOUCH, account (B) | 648,002,700 | 0.5 |
| EU, account (B) | 38,857,440 | 0.9 |
| ceiling B, account (B) | 2,290,617,600 | 9.0 |
| account (A), the whole lattice | 47,584,347,840 | 12.7 |
| account (C) | 1,481,760 | 0.0 |

## The stages, in run order

The accounts run (A), then (B), then (C): "the indices run first because two names are quick" (requirement 30). Inside an account each new stage runs right after its mirror, each gate stage right before its mirror as 24.6 fixed, and ceiling B after its wave 1. Inside a stage: timeframes 60 down to 1, families E1C, E2C, E1S, E2S, E3C, E3S, EUC, EUS. Stage 2 runs on each window's candidates after wave 1 (interim 1) and again after wave 2. Model hours: see the estimate below.

| Group | Stage | Plain words | Nodes and filters | Lanes | Timeframes | Exact count | Share | Shards | Model hours |
|---|---|---|---|---|---|---|---|---|---|
| (A) wave 0 | `A.W0.seeds` | (A) the operator's seeds, as typed | level 0: E1 K 1, 2, 3 x tight (own p25 low, own p50 low, any); E2 rung 0.786, = 2 | the seed lanes (E1 XA XB XC XD2 XD3; E2 XA XB XC XJ0) | every timeframe; LONG, SHORT, BOTH | 43,968 | 0.0% | 216 | 0.0 |
| (A) wave 0 | `A.W0.e3.seeds` | (A) E3: every range duration, as seeds | E3C: 8 durations, first break | E3 seed lanes (XA XB XC XD2 XD3) | every timeframe; LONG, SHORT, BOTH | 7,680 | 0.0% | 120 | 0.0 |
| (A) wave 0 | `A.W0.eu.seeds` | (A) EU: the operator's seeds in one recipe | F + T, F + O, T + O, F + T + O at the seeds | XA XB XC | every timeframe; LONG, SHORT, BOTH | 12,672 | 0.0% | 72 | 0.0 |
| (A) wave 1 | `A.W1.base` | (A) every base node | as (B), less wave 0's | full-spool lanes (seed lanes + XE stair-step) | every timeframe, 6 sides | 730,176 | 0.0% | 1,056 | 0.0 |
| (A) wave 1 | `A.W1.e3.base` | (A) E3: every duration, every side | 8 durations | full-spool lanes | every timeframe, 6 sides | 29,184 | 0.0% | 576 | 0.0 |
| (A) wave 1 | `A.W1.eu.base` | (A) EU: every member set | 234 nodes: flag 3, touch 6, ORB 8, at least two members | XA XB XC XE | every timeframe, 6 sides | 4,134,528 | 0.0% | 384 | 0.0 |
| (A) wave 1 | `A.W1.level1` | (A) one more dial on every base node | base x one dial | full-spool lanes | every timeframe | 156,980,736 | 0.1% | 1,056 | 0.6 |
| (A) wave 1 | `A.W1.e3.level1` | (A) E3: one more dial | 8 durations x one dial | full-spool lanes | every timeframe | 13,349,376 | 0.0% | 576 | 0.0 |
| (A) wave 1 | `A.W1.eu.level1` | (A) EU: one more dial, on all members | 4 seed nodes x one dial | core lanes | every timeframe | 3,667,968 | 0.0% | 288 | 0.0 |
| (A) wave 1 | `A.W1.contract` | (A) the contract rule: volume window x moneyness | level 0 seeds x 8 other rules | seed lanes | every timeframe | 703,488 | 0.0% | 432 | 0.0 |
| (A) wave 1 | `A.W1.e3.contract` | (A) the contract rule on E3 | 8 durations x 8 other rules | seed lanes | every timeframe | 122,880 | 0.0% | 240 | 0.0 |
| (A) wave 1 | `A.W1.exits1` | (A) every exit on the seeds | level 0 seeds | every stop on the seed lanes + every lane at the seed stop + mapping R2 on the seed lanes | every timeframe | 5,484,096 | 0.0% | 21,432 | 0.0 |
| (A) wave 1 | `A.W1.e3.exits1` | (A) E3: every exit on the seeds | 8 durations | every stop on the seed lanes + every lane at the seed stop | every timeframe | 3,484,416 | 0.0% | 11,244 | 0.0 |
| (A) wave 1 | `A.W1.exits2` | (A) every other stop x every other lane | the central seeds | the rest of the 15 stops x 51-60 lanes | every timeframe | 6,467,136 | 0.0% | 121,296 | 0.0 |
| (A) wave 1 | `A.W1.e3.exits2` | (A) E3: every other stop x every other lane | d = 1 | the rest of the 15 stops x the lanes | every timeframe | 2,943,360 | 0.0% | 57,960 | 0.0 |
| (A) wave 1 | `A.W1.level2` | (A) two more dials | timeframes 15, 30, 60: every base node x two dials; below 15: the central seed x two dials | core lanes (XA, XC, XE stair-step) | every timeframe | 1,986,894,720 | 1.0% | 576 | 1.3 |
| (A) wave 1 | `A.W1.e3.level2` | (A) E3: two more dials | timeframes 15, 30, 60: 8 durations x two dials; below 15: d = 1 x two dials + another duration x one dial | XA on 15, 30, 60; core below | every timeframe | 79,028,064 | 0.0% | 216 | 0.2 |
| (A) wave 1 | `A.W1.eu.level2` | (A) EU: two more dials | 15, 30, 60 minutes: 4 seed nodes; below: T + O | XA | every timeframe | 15,233,184 | 0.0% | 96 | 0.1 |
| (A) ceiling B | `A.CB.W0.seeds` | (A) at 10 a day: the operator's seeds, as typed | level 0: E1 K 1, 2, 3 x tight (own p25 low, own p50 low, any); E2 rung 0.786, = 2 | the seed lanes (E1 XA XB XC XD2 XD3; E2 XA XB XC XJ0) | every timeframe; LONG, SHORT, BOTH | 43,968 | 0.0% | 216 | 0.0 |
| (A) ceiling B | `A.CB.W0.e3.seeds` | (A) at 10 a day: E3: every range duration, as seeds | E3C: 8 durations, first break | E3 seed lanes (XA XB XC XD2 XD3) | every timeframe; LONG, SHORT, BOTH | 7,680 | 0.0% | 120 | 0.0 |
| (A) ceiling B | `A.CB.W0.eu.seeds` | (A) at 10 a day: EU: the operator's seeds in one recipe | F + T, F + O, T + O, F + T + O at the seeds | XA XB XC | every timeframe; LONG, SHORT, BOTH | 12,672 | 0.0% | 72 | 0.0 |
| (A) ceiling B | `A.CB.W1.base` | (A) at 10 a day: every base node | 24 E1 (K 8 x tight 3) + 60 E2 (rung 12 x clause 5), less wave 0's | full-spool lanes (seed lanes + XE stair-step) | every timeframe, 6 sides | 730,176 | 0.0% | 1,056 | 0.0 |
| (A) ceiling B | `A.CB.W1.e3.base` | (A) at 10 a day: E3: every duration, every side | 8 durations | full-spool lanes | every timeframe, 6 sides | 29,184 | 0.0% | 576 | 0.0 |
| (A) ceiling B | `A.CB.W1.eu.base` | (A) at 10 a day: EU: every member set | 234 nodes: flag 3, touch 6, ORB 8, at least two members | XA XB XC XE | every timeframe, 6 sides | 4,134,528 | 0.0% | 384 | 0.0 |
| (A) ceiling B | `A.CB.W1.level1` | (A) at 10 a day: one more dial on every base node | base x one dial | full-spool lanes | every timeframe | 156,980,736 | 0.1% | 1,056 | 0.6 |
| (A) ceiling B | `A.CB.W1.e3.level1` | (A) at 10 a day: E3: one more dial | 8 durations x one dial | full-spool lanes | every timeframe | 13,349,376 | 0.0% | 576 | 0.0 |
| (A) wave 2 | `A.W2.sniper.singles.seeds` | (A) Sniper: each gate value on the seeds | level 0 seeds x 21 gate values (9 where S-RR is refused) | tier A (20 lanes) | every timeframe | 17,671,680 | 0.0% | 3,648 | 0.0 |
| (A) wave 2 | `A.W2.singles.seeds` | (A) each context filter on the seeds | level 0 seeds x every filter (full bands) | tier A (20 lanes) | every timeframe | 423,299,520 | 0.2% | 3,648 | 1.1 |
| (A) wave 2 | `A.W2.e3.sniper.singles.seeds` | (A) E3: each gate value | 8 durations x 21 gate values | core lanes | every timeframe | 2,377,728 | 0.0% | 288 | 0.0 |
| (A) wave 2 | `A.W2.e3.singles.seeds` | (A) E3: each context filter | 8 durations x every filter (full bands) | core lanes | every timeframe | 40,746,240 | 0.0% | 288 | 0.1 |
| (A) wave 2 | `A.W2.orb.singles.seeds` | (A) ORB-POS: each position value on the seeds | level 0 seeds x 8 durations x (3 states + 9 bands) | tier A | every timeframe | 91,865,088 | 0.0% | 3,648 | 0.3 |
| (A) wave 2 | `A.W2.sniper.singles.base` | (A) Sniper: each gate value on the other base nodes | other base nodes x 21 gate values | full-spool lanes | every timeframe | 37,783,296 | 0.0% | 1,056 | 0.1 |
| (A) wave 2 | `A.W2.singles.base` | (A) each context filter on the other base nodes | other base nodes x every filter (full bands) | full-spool lanes | every timeframe | 678,365,280 | 0.3% | 1,056 | 1.0 |
| (A) wave 2 | `A.W2.orb.singles.base` | (A) ORB-POS: on the other base nodes | other base nodes x 96 values | full-spool lanes | every timeframe | 157,888,512 | 0.1% | 1,056 | 0.2 |
| (A) wave 2 | `A.W2.codes` | (A) candle-code sequences | central seeds x 14,424 codes | core lanes | every timeframe | 265,863,168 | 0.1% | 576 | 0.1 |
| (A) wave 2 | `A.W2.e3.codes` | (A) E3: candle-code sequences | d = 1 x 14,424 codes | core lanes | every timeframe | 33,232,896 | 0.0% | 288 | 0.0 |
| (A) wave 2 | `A.W2.pat.named` | (A) pattern at the touch: each name on each touch node | 60 nodes (rung x clause) x 54 names | full-spool lanes, no XJ on = 1 | every timeframe | 11,943,936 | 0.0% | 480 | 0.0 |
| (A) wave 2 | `A.W2.pat.codes` | (A) pattern at the touch: candle codes on T and T-1 | 59 nodes x 600 codes of lengths 1, 2 | core lanes | every timeframe | 81,561,600 | 0.0% | 288 | 0.0 |
| (A) wave 2 | `A.W2.pat.anatomy` | (A) pattern at the touch: two anatomy readings on T and T-1 | 59 nodes x 405 coarse pairs | XC, XE from 5 minutes (XA is in `W2.pairs.base`); core below | every timeframe | 305,091,360 | 0.2% | 228 | 0.1 |
| (A) wave 2 | `A.W2.pat.named.anatomy` | (A) a name x an anatomy band | 20 nodes (4 deep rungs x clause) x 54 x 30 | core lanes | every timeframe | 522,547,200 | 0.3% | 288 | 0.0 |
| (A) wave 2 | `A.W2.pat.named.context` | (A) a name x a context filter | central seed x 54 x every coarse filter | XA | every timeframe | 32,322,240 | 0.0% | 96 | 0.0 |
| (A) wave 2 | `A.W2.pat.level1` | (A) a name on level 1 | central seed x one dial x 54 | core lanes | every timeframe | 41,539,392 | 0.0% | 288 | 0.0 |
| (A) wave 2 | `A.W2.pat.trigger` | (A) a name on the flag's or the ORB's break candle | E1 24 nodes, E3 8 nodes x 54 | full-spool lanes | every timeframe | 31,850,496 | 0.0% | 1,152 | 0.0 |
| (A) wave 2 | `A.W2.eu.singles` | (A) EU: each context filter | 4 seed nodes x every coarse filter | core lanes | every timeframe | 11,200,320 | 0.0% | 288 | 0.1 |
| (A) wave 2 | `A.W2.eu.pat` | (A) EU: the touch member under a name | 3 seed nodes x 54 | core lanes | every timeframe | 1,866,240 | 0.0% | 288 | 0.0 |
| (A) wave 2 | `A.W2.sniper.triples` | (A) Sniper: triples holding a gate, at the Rs 2 lakh ceiling | as `W2.triples`, sets with at least one gate value | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 2,951,677,920 | 1.5% | 432 | 0.3 |
| (A) wave 2 | `A.W2.triples` | (A) three filters at once, at the Rs 2 lakh ceiling | timeframes 15, 30, 60: deep nodes (E1 K 1, 2, 3 at p50 low; E2 rungs 0.382, 0.5, 0.618, 0.786 at = 2); below 15: the central seed x every coarse triple | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 19,324,468,968 | 9.8% | 432 | 1.7 |
| (A) wave 2 | `A.W2.e3.triples` | (A) E3: three filters at once, at the Rs 2 lakh ceiling | d = 1 x every coarse triple | XA | 60 minutes (extension below) | 217,209,840 | 0.1% | 12 | 0.0 |
| (A) wave 2 | `A.W2.sniper.pairs.base` | (A) Sniper: coarse pairs holding a gate, at the Rs 2 lakh ceiling | other base nodes, sets with at least one gate value | XA | 5 minutes and up | 113,050,272 | 0.1% | 120 | 0.0 |
| (A) wave 2 | `A.W2.pairs.base` | (A) two coarse filters on the other base nodes, at the Rs 2 lakh ceiling | other base nodes x every coarse pair | XA | 5 minutes and up | 1,009,358,064 | 0.5% | 120 | 0.1 |
| (A) wave 2 | `A.W2.sniper.singles.level1` | (A) Sniper: each gate value on level 1 | central seed x one dial x 21 gate values | core lanes from 10 minutes; XA below | every timeframe | 51,544,224 | 0.0% | 384 | 0.1 |
| (A) wave 2 | `A.W2.singles.level1` | (A) each filter on level 1 | central seed x one dial x every filter (full bands) | core lanes from 10 minutes; XA below | every timeframe | 1,059,985,776 | 0.5% | 384 | 1.0 |
| (A) wave 2 | `A.W2.e3.sniper.singles.level1` | (A) E3: each coarse gate value on level 1 | d = 1 x one dial x 10 coarse gate values | XA | 15 minutes and up (extension below) | 1,581,120 | 0.0% | 36 | 0.0 |
| (A) wave 2 | `A.W2.e3.singles.level1` | (A) E3: each coarse filter on level 1 | d = 1 x one dial x every filter (coarse) | XA | 15 minutes and up (extension below) | 29,708,736 | 0.0% | 36 | 0.0 |
| (A) wave 2 | `A.W2.orb.singles.level1` | (A) ORB-POS: on level 1 | central seed x one dial x 48 coarse values | core lanes from 10 minutes; XA below | 15 minutes and up (extension below) | 20,141,568 | 0.0% | 72 | 0.0 |
| (A) wave 2 | `A.W2.sniper.pairs.seeds` | (A) Sniper: pairs holding a gate (full bands), at the Rs 2 lakh ceiling | as `W2.pairs.seeds`, sets with at least one gate value | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 181,222,272 | 0.1% | 432 | 0.1 |
| (A) wave 2 | `A.W2.pairs.seeds` | (A) two filters at once (full bands), at the Rs 2 lakh ceiling | timeframes 15, 30, 60: level 0 seeds; below 15: the central seed | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 1,940,312,208 | 1.0% | 432 | 0.7 |
| (A) wave 2 | `A.W2.e3.sniper.pairs.seeds` | (A) E3: coarse pairs holding a gate, at the Rs 2 lakh ceiling | d = 1 | XA | 5 minutes and up (extension below) | 1,495,248 | 0.0% | 60 | 0.0 |
| (A) wave 2 | `A.W2.e3.pairs.seeds` | (A) E3: two coarse filters, at the Rs 2 lakh ceiling | d = 1 | XA | 5 minutes and up (extension below) | 12,981,816 | 0.0% | 60 | 0.0 |
| (A) wave 2 | `A.W2.orb.pairs.seeds` | (A) ORB-POS: coarse pairs holding a position, at the Rs 2 lakh ceiling | level 0 seeds | XA | 15 minutes and up (extension below) | 52,275,456 | 0.0% | 72 | 0.0 |
| (A) wave 2 | `A.W2.sniper.singles.level2` | (A) Sniper: each coarse gate value on level 2 | central seed x two dials x 10 coarse gate values | core lanes on 30, 60 minutes; XA on 10, 15 | 10 minutes and up (extension below 10) | 730,020,096 | 0.4% | 192 | 0.1 |
| (A) wave 2 | `A.W2.singles.level2` | (A) each coarse filter on level 2 (12.6's pairing) | central seed x two dials x every filter (coarse bands) | core lanes on 30, 60 minutes; XA on 10, 15 | 10 minutes and up (extension below 10) | 14,643,692,112 | 7.4% | 192 | 2.3 |
| Wave 0 | `W0.seeds` | the operator's seeds, as typed | level 0: E1 K 1, 2, 3 x tight (own p25 low, own p50 low, any); E2 rung 0.786, = 2 | the seed lanes (E1 XA XB XC XD2 XD3; E2 XA XB XC XJ0) | every timeframe; LONG, SHORT, BOTH | 27,480 | 0.0% | 216 | 0.0 |
| Wave 0 | `W0.e3.seeds` | E3: every range duration, as seeds | E3C: 8 durations, first break | E3 seed lanes (XA XB XC XD2 XD3) | every timeframe; LONG, SHORT, BOTH | 4,800 | 0.0% | 120 | 0.0 |
| Wave 0 | `W0.eu.seeds` | EU: the operator's seeds in one recipe | F + T, F + O, T + O, F + T + O at the seeds | XA XB XC | every timeframe; LONG, SHORT, BOTH | 7,920 | 0.0% | 72 | 0.0 |
| Wave 1 | `W1.base` | every base node | 24 E1 (K 8 x tight 3) + 60 E2 (rung 12 x clause 5), less wave 0's | full-spool lanes (seed lanes + XE stair-step) | every timeframe, 6 sides | 456,360 | 0.0% | 1,056 | 0.0 |
| Wave 1 | `W1.e3.base` | E3: every duration, every side | 8 durations | full-spool lanes | every timeframe, 6 sides | 18,240 | 0.0% | 576 | 0.0 |
| Wave 1 | `W1.eu.base` | EU: every member set | 234 nodes: flag 3, touch 6, ORB 8, at least two members | XA XB XC XE | every timeframe, 6 sides | 2,584,080 | 0.0% | 384 | 0.1 |
| Wave 1 | `W1.level1` | one more dial on every base node | base x one dial | full-spool lanes | every timeframe | 118,918,080 | 0.1% | 1,056 | 2.1 |
| Wave 1 | `W1.e3.level1` | E3: one more dial | 8 durations x one dial | full-spool lanes | every timeframe | 9,126,720 | 0.0% | 576 | 0.2 |
| Wave 1 | `W1.eu.level1` | EU: one more dial, on all members | 4 seed nodes x one dial | core lanes | every timeframe | 3,369,600 | 0.0% | 288 | 0.1 |
| Wave 1 | `W1.exits1` | every exit on the seeds | level 0 seeds | every stop on the seed lanes + every lane at the seed stop + mapping R2 on the seed lanes | every timeframe | 3,560,040 | 0.0% | 22,584 | 0.1 |
| Wave 1 | `W1.e3.exits1` | E3: every exit on the seeds | 8 durations | every stop on the seed lanes + every lane at the seed stop | every timeframe | 2,200,800 | 0.0% | 11,820 | 0.1 |
| Wave 1 | `W1.exits2` | every other stop x every other lane | the central seeds | the rest of the 15 stops x 51-60 lanes | every timeframe | 4,520,040 | 0.0% | 137,424 | 0.1 |
| Wave 1 | `W1.e3.exits2` | E3: every other stop x every other lane | d = 1 | the rest of the 15 stops x the lanes | every timeframe | 2,035,440 | 0.0% | 66,024 | 0.1 |
| Wave 1 | `W1.level2` | two more dials | timeframes 15, 30, 60: every base node x two dials; below 15: the central seed x two dials | core lanes (XA, XC, XE stair-step) | every timeframe | 2,151,393,840 | 1.1% | 576 | 6.4 |
| Wave 1 | `W1.e3.level2` | E3: two more dials | timeframes 15, 30, 60: 8 durations x two dials; below 15: d = 1 x two dials + another duration x one dial | XA on 15, 30, 60; core below | every timeframe | 79,178,400 | 0.0% | 216 | 1.1 |
| Wave 1 | `W1.eu.level2` | EU: two more dials | 15, 30, 60 minutes: 4 seed nodes; below: T + O | XA | every timeframe | 23,082,960 | 0.0% | 96 | 0.4 |
| (B) ceiling B | `CB.W0.seeds` | 10 a day, Rs 10 lakh: the operator's seeds, as typed | level 0: E1 K 1, 2, 3 x tight (own p25 low, own p50 low, any); E2 rung 0.786, = 2 | the seed lanes (E1 XA XB XC XD2 XD3; E2 XA XB XC XJ0) | every timeframe; LONG, SHORT, BOTH | 27,480 | 0.0% | 216 | 0.0 |
| (B) ceiling B | `CB.W0.e3.seeds` | 10 a day, Rs 10 lakh: E3: every range duration, as seeds | E3C: 8 durations, first break | E3 seed lanes (XA XB XC XD2 XD3) | every timeframe; LONG, SHORT, BOTH | 4,800 | 0.0% | 120 | 0.0 |
| (B) ceiling B | `CB.W0.eu.seeds` | 10 a day, Rs 10 lakh: EU: the operator's seeds in one recipe | F + T, F + O, T + O, F + T + O at the seeds | XA XB XC | every timeframe; LONG, SHORT, BOTH | 7,920 | 0.0% | 72 | 0.0 |
| (B) ceiling B | `CB.W1.base` | 10 a day, Rs 10 lakh: every base node | 24 E1 (K 8 x tight 3) + 60 E2 (rung 12 x clause 5), less wave 0's | full-spool lanes (seed lanes + XE stair-step) | every timeframe, 6 sides | 456,360 | 0.0% | 1,056 | 0.0 |
| (B) ceiling B | `CB.W1.e3.base` | 10 a day, Rs 10 lakh: E3: every duration, every side | 8 durations | full-spool lanes | every timeframe, 6 sides | 18,240 | 0.0% | 576 | 0.0 |
| (B) ceiling B | `CB.W1.eu.base` | 10 a day, Rs 10 lakh: EU: every member set | 234 nodes: flag 3, touch 6, ORB 8, at least two members | XA XB XC XE | every timeframe, 6 sides | 2,584,080 | 0.0% | 384 | 0.1 |
| (B) ceiling B | `CB.W1.level1` | 10 a day, Rs 10 lakh: one more dial on every base node | base x one dial | full-spool lanes | every timeframe | 118,918,080 | 0.1% | 1,056 | 2.1 |
| (B) ceiling B | `CB.W1.e3.level1` | 10 a day, Rs 10 lakh: E3: one more dial | 8 durations x one dial | full-spool lanes | every timeframe | 9,126,720 | 0.0% | 576 | 0.2 |
| (B) ceiling B | `CB.W1.exits1` | 10 a day, Rs 10 lakh: every exit on the seeds | level 0 seeds | every stop on the seed lanes + every lane at the seed stop + mapping R2 on the seed lanes | every timeframe | 3,560,040 | 0.0% | 22,584 | 0.1 |
| (B) ceiling B | `CB.W1.exits2` | 10 a day, Rs 10 lakh: every other stop x every other lane | the central seeds | the rest of the 15 stops x 51-60 lanes | every timeframe | 4,520,040 | 0.0% | 137,424 | 0.1 |
| (B) ceiling B | `CB.W1.level2` | 10 a day, Rs 10 lakh: two more dials | timeframes 15, 30, 60: every base node x two dials; below 15: the central seed x two dials | core lanes (XA, XC, XE stair-step) | every timeframe | 2,151,393,840 | 1.1% | 576 | 6.4 |
| Wave 2 | `W2.sniper.singles.seeds` | Sniper: each gate value on the seeds | level 0 seeds x 21 gate values (9 where S-RR is refused) | tier A (20 lanes) | every timeframe | 11,347,200 | 0.0% | 3,840 | 0.2 |
| Wave 2 | `W2.singles.seeds` | each context filter on the seeds | level 0 seeds x every filter (full bands) | tier A (20 lanes) | every timeframe | 330,259,200 | 0.2% | 3,840 | 4.2 |
| Wave 2 | `W2.e3.sniper.singles.seeds` | E3: each gate value | 8 durations x 21 gate values | core lanes | every timeframe | 1,486,080 | 0.0% | 288 | 0.0 |
| Wave 2 | `W2.e3.singles.seeds` | E3: each context filter | 8 durations x every filter (full bands) | core lanes | every timeframe | 29,603,520 | 0.0% | 288 | 0.4 |
| Wave 2 | `W2.orb.singles.seeds` | ORB-POS: each position value on the seeds | level 0 seeds x 8 durations x (3 states + 9 bands) | tier A | every timeframe | 60,364,800 | 0.0% | 3,840 | 0.8 |
| Wave 2 | `W2.sniper.singles.base` | Sniper: each gate value on the other base nodes | other base nodes x 21 gate values | full-spool lanes | every timeframe | 23,614,560 | 0.0% | 1,056 | 0.2 |
| Wave 2 | `W2.singles.base` | each context filter on the other base nodes | other base nodes x every filter (full bands) | full-spool lanes | every timeframe | 495,900,840 | 0.3% | 1,056 | 3.6 |
| Wave 2 | `W2.orb.singles.base` | ORB-POS: on the other base nodes | other base nodes x 96 values | full-spool lanes | every timeframe | 98,680,320 | 0.0% | 1,056 | 0.7 |
| Wave 2 | `W2.codes` | candle-code sequences | central seeds x 14,424 codes | core lanes | every timeframe | 166,164,480 | 0.1% | 576 | 0.3 |
| Wave 2 | `W2.e3.codes` | E3: candle-code sequences | d = 1 x 14,424 codes | core lanes | every timeframe | 20,770,560 | 0.0% | 288 | 0.0 |
| Wave 2 | `W2.pat.named` | pattern at the touch: each name on each touch node | 60 nodes (rung x clause) x 54 names | full-spool lanes, no XJ on = 1 | every timeframe | 7,464,960 | 0.0% | 480 | 0.0 |
| Wave 2 | `W2.pat.codes` | pattern at the touch: candle codes on T and T-1 | 59 nodes x 600 codes of lengths 1, 2 | core lanes | every timeframe | 50,976,000 | 0.0% | 288 | 0.0 |
| Wave 2 | `W2.pat.anatomy` | pattern at the touch: two anatomy readings on T and T-1 | 59 nodes x 405 coarse pairs | XC, XE from 5 minutes (XA is in `W2.pairs.base`); core below | every timeframe | 190,682,100 | 0.1% | 228 | 0.4 |
| Wave 2 | `W2.pat.named.anatomy` | a name x an anatomy band | 20 nodes (4 deep rungs x clause) x 54 x 30 | core lanes | every timeframe | 326,592,000 | 0.2% | 288 | 0.1 |
| Wave 2 | `W2.pat.named.context` | a name x a context filter | central seed x 54 x every coarse filter | XA | every timeframe | 23,775,120 | 0.0% | 96 | 0.0 |
| Wave 2 | `W2.pat.level1` | a name on level 1 | central seed x one dial x 54 | core lanes | every timeframe | 28,605,960 | 0.0% | 288 | 0.0 |
| Wave 2 | `W2.pat.trigger` | a name on the flag's or the ORB's break candle | E1 24 nodes, E3 8 nodes x 54 | full-spool lanes | every timeframe | 19,906,560 | 0.0% | 1,152 | 0.0 |
| Wave 2 | `W2.eu.singles` | EU: each context filter | 4 seed nodes x every coarse filter | core lanes | every timeframe | 8,646,480 | 0.0% | 288 | 0.2 |
| Wave 2 | `W2.eu.pat` | EU: the touch member under a name | 3 seed nodes x 54 | core lanes | every timeframe | 1,166,400 | 0.0% | 288 | 0.0 |
| Wave 2 | `W2.sniper.triples` | Sniper: triples holding a gate | as `W2.triples`, sets with at least one gate value | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 11,594,075,880 | 5.9% | 432 | 2.8 |
| Wave 2 | `W2.triples` | three filters at once | timeframes 15, 30, 60: deep nodes (E1 K 1, 2, 3 at p50 low; E2 rungs 0.382, 0.5, 0.618, 0.786 at = 2); below 15: the central seed x every coarse triple | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 95,027,264,640 | 48.1% | 432 | 22.0 |
| Wave 2 | `W2.e3.triples` | E3: three filters at once | d = 1 x every coarse triple | XA | 60 minutes (extension below) | 1,075,825,800 | 0.5% | 12 | 0.9 |
| Wave 2 | `W2.sniper.pairs.base` | Sniper: coarse pairs holding a gate | other base nodes, sets with at least one gate value | XA | 5 minutes and up | 351,371,520 | 0.2% | 120 | 0.2 |
| Wave 2 | `W2.pairs.base` | two coarse filters on the other base nodes | other base nodes x every coarse pair | XA | 5 minutes and up | 3,849,768,360 | 1.9% | 120 | 1.8 |
| Wave 2 | `W2.sniper.singles.level1` | Sniper: each gate value on level 1 | central seed x one dial x 21 gate values | full-spool lanes from 3 minutes; core on 1, 2 minutes | every timeframe | 105,648,300 | 0.1% | 936 | 0.7 |
| Wave 2 | `W2.singles.level1` | each filter on level 1 (**named cut on 1, 2, 3, 5 minutes**) | central seed x one dial x every filter (full bands) | full-spool lanes from 3 minutes; core on 1, 2 minutes | every timeframe | 2,633,575,320 | 1.3% | 936 | 16.2 |
| Wave 2 | `W2.e3.sniper.singles.level1` | E3: each coarse gate value on level 1 | d = 1 x one dial x 10 coarse gate values | XA | 15 minutes and up (extension below) | 1,306,440 | 0.0% | 36 | 0.0 |
| Wave 2 | `W2.e3.singles.level1` | E3: each coarse filter on level 1 | d = 1 x one dial x every filter (coarse) | XA | 15 minutes and up (extension below) | 28,606,200 | 0.0% | 36 | 0.3 |
| Wave 2 | `W2.orb.singles.level1` | ORB-POS: on level 1 | central seed x one dial x 48 coarse values | XA | 15 minutes and up (extension below) | 16,364,160 | 0.0% | 72 | 0.0 |
| Wave 2 | `W2.sniper.pairs.seeds` | Sniper: pairs holding a gate (full bands) | as `W2.pairs.seeds`, sets with at least one gate value | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 547,336,440 | 0.3% | 432 | 0.8 |
| Wave 2 | `W2.pairs.seeds` | two filters at once (full bands) | timeframes 15, 30, 60: level 0 seeds; below 15: the central seed | core lanes from 5 minutes; XA on 1, 2, 3 minutes | every timeframe | 7,093,779,600 | 3.6% | 432 | 10.5 |
| Wave 2 | `W2.e3.sniper.pairs.seeds` | E3: coarse pairs holding a gate | d = 1 | XA | 5 minutes and up (extension below) | 4,639,680 | 0.0% | 60 | 0.0 |
| Wave 2 | `W2.e3.pairs.seeds` | E3: two coarse filters | d = 1 | XA | 5 minutes and up (extension below) | 49,120,740 | 0.0% | 60 | 0.3 |
| Wave 2 | `W2.orb.pairs.seeds` | ORB-POS: coarse pairs holding a position | level 0 seeds | XA | 15 minutes and up (extension below) | 161,308,800 | 0.1% | 72 | 0.1 |
| Wave 2 | `W2.sniper.singles.level2` | Sniper: each coarse gate value on level 2 | central seed x two dials x 10 coarse gate values | core lanes on 30, 60 minutes; XA on 10, 15 | 10 minutes and up (extension below 10) | 802,841,760 | 0.4% | 192 | 0.5 |
| Wave 2 | `W2.singles.level2` | each coarse filter on level 2 (12.6's pairing) | central seed x two dials x every filter (coarse bands) | core lanes on 30, 60 minutes; XA on 10, 15 | 10 minutes and up (extension below 10) | 19,964,882,280 | 10.1% | 192 | 11.7 |
| (C) | `C.W0.seeds` | (C) both universes, one account: the operator's seeds, as typed | level 0: E1 K 1, 2, 3 x tight (own p25 low, own p50 low, any); E2 rung 0.786, = 2 | the seed lanes (E1 XA XB XC XD2 XD3; E2 XA XB XC XJ0) | every timeframe; LONG, SHORT, BOTH | 27,480 | 0.0% | 216 | 0.0 |
| (C) | `C.W0.e3.seeds` | (C) both universes, one account: E3: every range duration, as seeds | E3C: 8 durations, first break | E3 seed lanes (XA XB XC XD2 XD3) | every timeframe; LONG, SHORT, BOTH | 4,800 | 0.0% | 120 | 0.0 |
| (C) | `C.W1.base` | (C) both universes, one account: every base node | 24 E1 (K 8 x tight 3) + 60 E2 (rung 12 x clause 5), less wave 0's | full-spool lanes (seed lanes + XE stair-step) | every timeframe, 6 sides | 456,360 | 0.0% | 1,056 | 0.0 |
| (C) | `C.W1.e3.base` | (C) both universes, one account: E3: every duration, every side | 8 durations | full-spool lanes | every timeframe, 6 sides | 18,240 | 0.0% | 576 | 0.0 |
| (C) | `C.W1.level1` | (C) one more dial, both universes, one account | central seed x one dial | XA | every timeframe | 784,740 | 0.0% | 192 | 0.0 |
| (C) | `C.W1.e3.level1` | (C) one more dial, both universes, one account | central seed x one dial | XA | every timeframe | 190,140 | 0.0% | 96 | 0.0 |
| | **Table** | | | | | **197,480,655,060** | | **678,384** | **112.5** |
| | **Named cut: `W2.singles.level1` on 1, 2, 3, 5 minutes** | | | | | **-1,185,746,400** | | | **-13.97** |
| | **Committed** | | | | | **196,294,908,660** | | | **98.5** |

Stage 2, an exact formula over at most 1,365 candidates (39 scopes x at most 5 x 7 windows), plus 175 for the 5 new scopes of account (B) (E3C, E3S, EUC, EUS, MAX-NET), plus 420 for the SNIPER cells (12 cells x at most 5 x 7 windows; their month cap also takes the labelled values 2 and 5), plus 2,065 for account (A) (47 scopes + 12 cells), fewer where scopes share a candidate:

| Stage | At most |
|---|---|
| `S2.money` | 141,960 |
| `S2.controls` | 17,472,000 |
| `S2.ascent` | 53,235 |
| `S2.exitdrill` | 6,142,500 |
| `S2.pair` | 34,125 |
| `S2.mix` | 34,125 |
| `S2.shown` | 336 |
| `S2.sniper.money` | 43,680 |
| `S2.sniper.controls` | 8,064,000 |
| `S2.sniper.ascent` | 16,380 |
| `S2.sniper.exitdrill` | 1,890,000 |
| `S2.sniper.pair` | 10,500 |
| `S2.sniper.mix` | 10,500 |
| `S2.new.money` | 18,200 |
| `S2.new.controls` | 2,688,000 |
| `S2.new.ascent` | 6,825 |
| `S2.new.exitdrill` | 787,500 |
| `S2.new.pair` | 4,375 |
| `S2.new.mix` | 4,375 |
| `S2.controls.24U` | 3,494,400 |
| `S2.shown.data` | 24 |
| `S2.shown.replay` | 528 |
| `A.S2.money` | 132,160 |
| `A.S2.controls` | 12,687,360 |
| `A.S2.ascent` | 80,535 |
| `A.S2.exitdrill` | 3,717,000 |
| `A.S2.pair` | 20,650 |
| `A.S2.mix` | 20,650 |
| `A.S2.shown` | 336 |
| `C.mix` | 44,625 |
| `CB.S2.shown` | 12 |

Adaptive drill-down, level 3+ of PREVIEW-SPEC 13.1, chosen on learning months only; each is a budget, the achieved figure is printed, and neither counts toward the guaranteed part:

| Stage | At most |
|---|---|
| `W2.fine` | 169,344,000 |
| `W2.stacks` | 774,144,000 |
| `W2.sniper.stacks` | 25,401,600 |
| `W2.e3.fine` | 84,672,000 |
| `W2.e3.stacks` | 387,072,000 |
| `W2.orb.stacks` | 116,121,600 |
| `A.W2.fine` | 101,606,400 |
| `A.W2.stacks` | 464,486,400 |

## What counts as one evaluated combination

- One exact account over all 98 months (2018-09..2026-10) for one recipe identity: account, timeframe, account side, family, core node, filter set, exit config, N (and on (A) the premium seed and contract rule) and, when any value in it is generated, the learning window that generated it. FIRST admission and no account controls in waves 0-2, with the ceilings of 24.U always on; stage 2 adds the controls.
- Or a skip by a proven exact bound, counted as tried: an empty kept set (no trade in any month), or a series the R3:522-532 bound proves can enter no heap and raise no luck draw, proven byte-equal with the bound on and off on real units before use.
- Every one is offered to the one luck test (14.3), or is proven not to raise it. `tried_total` = evaluated + bound-skipped, per account, wave, family, unit and stage, with its base-2 logarithm.
- **Near-duplicates get no discount.** An exact duplicate (equal digest of the whole order list) stays in `tried_total`, which can only raise the bar; `distinct_total` is printed beside it, as are "with at least one trade" and "each generated recipe once". Round 2 F&O's share with a trade was 47%: **many of these combinations will hold no trade, and the report says how many.**
- A unit the named cut removes is not tried and not counted: the report prints the achieved figure beside this table, cut by cut.
- Twins are not counted: they are a null test, not options tried on the real data.

## The exact enumeration, for the stage builders

**Generated or not.** E = enumerated, one series offered to all windows. G = generated from a window's learning months, so one series per window (7). C = round 2's causal readings, frozen each 1 January, one series. A combination with any G value counts 7 times; the script counts this exactly.

**Band lists.** Full bands of a G reading, 9: at or below p25, p25-p50, p50-p75, at or above p75, at or below p50, p25-p75, at or above p25, at or below p75, at or above p50 (the contiguous unions of the quartile atoms, less the whole range, which is the neutral value). Coarse bands, 3: at or below p25, p25-p75, at or above p75. C readings: round 2's five coarse bands (F2/pct.rs BANDS), coarse = its three thirds. Every coarse band is a full band. A filter set takes at most one band from each reading.

**Base nodes.** E1: K in {1, 2, 3, 4, 6, 8, 12, 16} x tight in {own p25 low, own p50 low, any}, 24 nodes. E2: rung (12) x clause (= 1, = 2, = 3, >= 2, >= 3), 60 nodes. E3: range duration d in {1, 2, 3, 5, 10, 15, 30, 60}, 8 nodes. EU: at most one flag member (K 1, 2, 3 at own p50 low), one touch member (rung 0.786 x = 1, = 2, = 3, >= 2, >= 3, any) and one ORB member (8 durations), at least two: 234 nodes, of which the 48 with no flag hold no generated value. **Level 0 seeds:** E1 K 1, 2, 3 x the three tight readings (9); E2 0.786, = 2 (1); E3 every duration (8; requirement 27 lists them all); EU the four member sets at the seeds K 2 own p50 low, 0.786 = 2, d = 1. **Central seed:** E1 K 2, own p50 low; E2 0.786, = 2; E3 d = 1, the operator's chart (09:15-09:16). **Deep nodes:** E1 K 1, 2, 3 at own p50 low; E2 rungs 0.382, 0.5, 0.618, 0.786 at = 2.

**Dials beyond the base (non-seed values; h = doubling points up to the session: 9, 8, 7, 7, 6, 5, 4, 3 on 1, 2, 3, 5, 10, 15, 30, 60 minutes).** "All" below means E1, E2 and E3; EU's dials are the rank dials, S37 and S37a, on all its members together.

| Dial | Applies | E values | G values | Seed |
|---|---|---|---|---|
| S10 move basis | all, not (A) | 1 (b) | | (a) |
| S11 top N | all, not (A) | 5 (1, 3, 5, 25, 50) | | 10 |
| S12 rank state and clock | all, not (A) | 18 (entered within 1, 2, 4, 8, 16, session; ordinal = 1, = 2, >= 3; in top N at minute 15, 30, 46, 93, 140, 187, 234, 281, 328; false before its clock) | | in top N at T |
| S13 move vs own usual | all | | 9 | off |
| S15 stale limit | all, not (A) | 10 (1, 2, 4, ..., 256, 375) | | carry |
| S21 ladder mode | all | 1 (frozen; on E3 at the range's close) | | running |
| S23 same candle | all | 1 (V-BOTH) | | engine |
| S25 range so far vs usual | all | | 9 | off |
| TIME S26 x S42 | all | 35 (i < j on the eighths, less the whole session) | | whole session |
| S32 box height | E1 | | 16 (2 measures x 9, less the 2 seeds), with tight = any only | base |
| S33 holding | E1 | 15 (2 readings x 7 rungs + first-candle + none, less the seed) | | lows above 0.382 |
| S34 push | E1 | | 36 (4 measures x 9) | any |
| S35 pause vs push volume | E1, not (A) | | 9 | any |
| S37 colour | all | 1 | | E1, E3 side colour; E2 any |
| S37a close beyond box, level or range | all | | 9 (ppm of R; on E3 ppm of the range's width) | any |
| S39 pause number | E1 | 3 | | any |
| S27 on the holding level | E1 | 3 | | none |
| S24 near distance | E2 | | 6 (2 measures x 3 quartiles) | 0 |
| S28 bounce | E2 | 4 (m = 2..5) | 6 (2 measures x 3 quartiles) | m = 1 |
| S29 gap at most | E2 | h | | off |
| S29a gap at least | E2 | h - 1 | | off |
| S29b close beyond | E2 | 1 | | goes on |
| S29c candles to the bounce | E2 | h | | off |
| S29d re-read | E2 second only | 1 | | held |
| ORBW range width | E3 | | 18 (ppm of price; against the stock's own usual range at that duration; 9 each) | any |
| ORBB which break | E3 | 3 (= 2, >= 2, any) | | first |
| ORBG within g candles of the range's close | E3 | h | | off |

**Levels.** Level 1 is one dial; level 2 is any two different dials. Left out because they can only hold nothing or repeat a value: returns-only dials (S28, S29, S29a, S29b, S29c, S29d) and XJ lanes with clause = 1; S27 on the level with holding "none" or "first candle"; S29a above S29; the base pairs K x tight and rung x clause in the central lattice (they are base nodes).

**Lanes (exit configs).** Seed stop = pattern extreme + 1 measured step (on E3 the range's far edge + 1 step).
- Seed lanes: E1, E3 XA, XB, XC, XD2, XD3; E2 XA, XB, XC, XJ offset 0; EU XA, XB, XC.
- Full-spool lanes: the seed lanes + XE stair-step.
- Core lanes: XA, XC, XE stair-step.
- Tier A, 20: XA; XB; XC; XD2; XD3; XE stair-step; XE trail own p50 (G); XF arm 1.272 trail own p50 (G); XG H = 2, 4, 8 candles (1, 2, 4 on 60 minutes); XH close back; XH rank leaves top 50; XI period p50 x multiplier p75 (G); E1 next-rung target, E2 XJ offset 0; E1 excursion p50 in R (G), E2 XJ own p50 (G); and with target XA the stops next rung + 1 step, L - 1 step, pattern extreme + own p50 (G), pattern extreme + 0.
- Every lane, 51 to 60: XA offset 1 step + 3 G; 6 rung targets; excursion target 3 G; XD 2; XE stair-step + 3 G; XF 3 G; XG h; XH E1 8 (box low, level, rank leaves top N' for 6 N'), E2 7; XI 9 G; break-even on XA, XB, XC x 3 G; E2 XJ 0 + 3 G.
- Every stop, 15: placement (pattern extreme, next rung, L) x offset (0, 1 step, own p25, p50, p75). On E3 L is the broken edge of the range.
- Account (A): no rank-leaves lane: tier A 19, every lane 6 fewer.

**Walks these lanes need (for F3).** Every record: its full-spool lanes x the 5 distinct amounts of 24.U (Rs 15, 10, 7.5, 5, 2.5 lakh) x WITH and FADE, 60 paths on E1 and E3 and 50 on E2 (round 2 held 156). EU reads the records of its members and needs no walk of its own. Account (A): full-spool lanes x the 4 premium seeds x CE and PE at the central contract rule. Delta walks, only on the records the stage keeps: tier A and every stop and lane on the seeds' records, the cross on the central seeds' records, stage 2's drill on each candidate's records; each generated value snapped to its walked grid point (12.1).

**FADE (REQ 17; replaces DV-26).** A FADE entry takes the opposite side at the same decision second, under every fill rule of lane S. Its stop and target sit at the signal lane's stop and target distances, mirrored about the entry's sizing price and mapped by R1. Time and condition exits keep their clocks. FADE sides are labelled in every row. The side's rank pool stays the signal's. On (A) a FADE of a long signal buys the put.

**Filter library per reading (E unless marked).** Day of week 5 / 5 (full / coarse); day of month 9 / 4; days to expiry 20 / 3; expiry week 2 / 2; stock's k-th trade 3 / 3; account's k-th trade 6 / 3; after a win or loss 2 / 2; anatomy 15 readings G 9 / 3; five index moves G 9 / 3; NIFTY and BANKNIFTY EMA-20 side 2 / 2; VIX level and change G 9 / 3; liquidity, volatility, price G 9 / 3; 18 industries 18 / 18; per higher timeframe colour 2 / 2 and position G 9 / 3; 7 anchor positions G 9 / 3; 2 trigger volumes G 9 / 3; 42 indicators C 5 / 3 on 3, 5, 15, 30, 60 minutes. Candle codes (24 + 576 + 13,824, round 2's alphabet) run only as singles in `W2.codes` and on lengths 1 and 2 in `W2.pat.codes`. **No futures column anywhere (REQ 18):** days to expiry is the expiry calendar only, from the GDFL FNO folder names (3.3); no futures price, open interest or volume is read or counted.

| Timeframe | Singles, full | Pairs, full | Singles, coarse | Pairs, coarse | Triples, coarse |
|---|---|---|---|---|---|
| 1 min | 452 | 100,032 | 181 | 15,978 | 917,870 |
| 2 min | 441 | 95,163 | 176 | 15,092 | 841,354 |
| 3 min | 640 | 202,240 | 297 | 43,526 | 4,197,318 |
| 5 min | 629 | 195,303 | 292 | 42,060 | 3,985,266 |
| 10 min | 408 | 81,282 | 161 | 12,584 | 637,316 |
| 15 min | 607 | 181,792 | 282 | 39,203 | 3,582,742 |
| 30 min | 596 | 175,218 | 277 | 37,812 | 3,392,020 |
| 60 min | 585 | 168,765 | 272 | 36,446 | 3,208,158 |

**Account (A)'s library:** the same, less the 3 generated universe readings, the 18 industries and the 2 trigger volumes (no cross-section, no volume), plus NIFTY only and BANKNIFTY only (one reading, 2 / 2), with 33 of the 42 indicators (not volume vs its mean, VWAP, relative volume, rank, the three sector basket readings and the two breadth readings). Pairs and triples on (A) run at the ceiling premium only (N 1, 2 x Rs 2 lakh x 6 sides).

| Timeframe | Singles, full | Pairs, full | Singles, coarse | Pairs, coarse | Triples, coarse |
|---|---|---|---|---|---|
| 1 min | 391 | 74,683 | 150 | 11,030 | 529,946 |
| 2 min | 380 | 70,485 | 145 | 10,299 | 477,581 |
| 3 min | 534 | 140,493 | 239 | 28,205 | 2,191,075 |
| 5 min | 523 | 134,722 | 234 | 27,029 | 2,054,526 |
| 10 min | 347 | 58,617 | 130 | 8,256 | 341,346 |
| 15 min | 501 | 123,543 | 224 | 24,752 | 1,798,658 |
| 30 min | 490 | 118,135 | 219 | 23,651 | 1,679,089 |
| 60 min | 479 | 112,848 | 214 | 22,575 | 1,564,930 |

**Sniper gates (requirement 16b; full definitions: `SNIPER.md`, PREVIEW-SPEC section 24).** Two more readings, in the `W2.sniper.*` stages only, each stage holding exactly the filter sets of its mirror that take at least one gate value; a filter set still takes at most one band from each reading. S-STOP, G: the 9 full bands; coarse 4 (the usual 3 + at or below own p50). S-RR, mixed: labelled seeds at least 2, 3, 5 (E, one series) + the 9 full bands (G); coarse 6 (the 3 seeds + the usual 3). "Any" is the seed of both and holds no clause, so waves 0 and 1 and every earlier stage are unchanged. S-RR is refused by name (`s_rr_no_target`) on XG, XH and XI, which have no first target at the decision: on tier A it runs on 14 lanes (10 E + 4 G), S-STOP on all 20. Added by the gates, per timeframe:

| Timeframe | Singles, full | Pairs, full | Singles, coarse | Pairs, coarse | Triples, coarse |
|---|---|---|---|---|---|
| 1 min | 21 | 9,600 | 10 | 1,834 | 164,124 |
| 2 min | 21 | 9,369 | 10 | 1,784 | 155,144 |
| 3 min | 21 | 13,548 | 10 | 2,994 | 442,388 |
| 5 min | 21 | 13,317 | 10 | 2,944 | 427,608 |
| 10 min | 21 | 8,676 | 10 | 1,634 | 129,704 |
| 15 min | 21 | 12,855 | 10 | 2,844 | 398,798 |
| 30 min | 21 | 12,624 | 10 | 2,794 | 384,768 |
| 60 min | 21 | 12,393 | 10 | 2,744 | 370,988 |

**ORB-POS (requirement 27 (b)).** Eight more readings, one per range duration, in the `W2.orb.*` stages only, each stage holding exactly the filter sets that take at least one of them: above, inside, below (E, one series each), and 9 generated distance bands; coarse 3 + the usual 3. A reading is absent until its range has closed, and an absent reading passes no clause.

**PATTERN-AT-TOUCH (requirement 28 (a)) and EU (requirement 28 (b)):** PREVIEW-SPEC 25.2.

## Shards

- One shard = account x timeframe x account side x family x exit lane x wave unit (the stage, then a fixed chunk of the filter-set order: chunks = the shard's exact count / 2^28, rounded up). 678,384 shards before chunking. The exit-lattice stages hold most of them, as small shards that may be grouped per lane bundle; the count does not change.
- Resumable: a shard is one unit in the rigor database (PREVIEW-SPEC 17.5), with its achieved count checked against `combos.json`. Kill and resume are byte-equal at every unit boundary. Workers claim the next shard in the fixed order.
- Incremental: a new dial adds shards and never reopens a finished one, because each unit carries its stage's code identity.
- **A deadline cuts whole units in the fixed order above, never by results.** Whatever is cut is named with its count in the report, and the achieved total is printed beside this table's total. A cut below 100,000,000,000 achieved is reported as requirement 17 NOT MET.

## Wall time (ESTIMATE; nothing measured on fib records yet)

The two measured rates agree closely: both are per wall second at 12-13 threads, under heavy load (80-220 on 14 cores).

| Basis | Figure | Hours for 198,475,377,556 |
|---|---|---|
| Round 2 F&O: 10,281,723,360 accounts in 14,235 unit-seconds (units ran one after another on 13 threads) | 722,285 a second | 76.3 |
| PSRB round 2: 701,751 series a second at 12 threads (293,384 simulated, the rest proven without a simulation) | 701,751 a second | 78.6 |
| Model: round 2 F&O's 261.6 million account steps a second, with kept records guessed at 80,000 / timeframe (E1), 40,000 / timeframe (E2), 20,000 flat (E3: at most about one first break a stock-day in the pool) a side and 2 / 10 of that on (A), x each stage's filter selectivity (the SEL tables in count.mjs), + 50 steps a series | mean 536 kept steps (round 2: 362) | **98.5** |

- **Planning figure: about 76.3-98.5 hours of phase B wall time at those rates, after the cut.** The table without the cut models 112.5 hours. The Mac is free now, so the rate may well be higher; if it doubles, about 38-49 hours. That is a condition, not a measurement: the five-stock and two-index tests measure records, kept steps and rate per stage, and print the projection before the full run. The kept-record guess can be off several times either way.
- **In the fixed run order, in model hours:** account (A)'s waves 0-1 end about 2.94 in, account (A) about 12.68 in; account (B)'s wave 0 about 12.68 in; its wave 1's interim about 23.52 in; ceiling B about 32.57 in; 100,000,000,000 is crossed inside `W2.triples` at 30 minutes, about 52.02 model hours in; the end of the table about 98.5 after the cut.
- **The dominant stage is `W2.triples`:** 48.1% of the table and 22.0 of the 98.5 model hours, the largest on both. Next by time: `W2.singles.level2` 11.7 h, `W2.pairs.seeds` 10.5 h, `W1.level2` and `CB.W1.level2` 6.4 h each. By timeframe: 1 minute 21.2 h against 60 minutes 10.1 h, before the cut.
- **What the new stages cost, by the model:** E3 3.3 h, ORB-POS 1.7 h, PATTERN-AT-TOUCH 0.5 h, EU 0.9 h, ceiling B 9.0 h, account (A) 12.7 h, account (C) 0.0 h. Not in the model, because nothing is measured: the search's per-series figures and the SNIPER slots add work to every series of every stage, about 3-10% by guess; an option path costs more than a cash path, and account (A)'s option walks are priced by the two-index test; EU's merged lists may cost more than its members' sum. If the projection does not fit, the next data-blind cuts are in PREVIEW-SPEC 25.5.
- **Phase A before it:** F2 measured 433 records a stock-day after its data-blind cuts, about 150 million records (11:19 note); E3 adds about 30 million by guess; (A) about 2 million. At 990 bytes a record and 60 paths x 40 bytes that is about 590 GB, **and the room above the disk floor is about 360 GB today (1,409 GiB free at 13:10 IST against the floor's 1,074 GiB). It does not fit as written: PREVIEW-SPEC 25.6 names the spool's own cuts and the decision it needs.** Walks: about 1-4 hours at round 2's 766,000 paths a second, plus delta walks.
- Twin seed 1, not counted: waves 0-1 of (A) and (B) on permuted records, about 13.1 model hours. Its wave 2 runs only if the real headline meets the bar (see section 23); no YES is possible without it.
- **This is far above the spec's 40-200 CPU-hours (17.8):** 98.5 hours x 12 threads is about 1182 CPU-hours. Requirement 17 was added after that estimate. The production fib backtest still has priority (17.6 rule 6): when it starts, the preview finishes its running unit and pauses.

## What I widened, and what I narrowed (all fixed before any run; none from a result)

The spec's levels read literally already exceed 100 billion, but in a form this Mac could not finish. So both directions apply.

**Widened beyond the spec's words:**
1. The level 1-2 lattice runs on every base node (K x tight, rung x clause), not only around the operator's seed.
2. The second reading is its own family crossing the whole lattice, not one dial.
3. Exhaustive pairs and triples of context filters over the library, not only singles paired with core nodes; deep nodes for triples on 15, 30, 60 minutes.
4. New dial values: S12's clocks and entry ordinals (REQ 17 "times"; SPEC S12), S25 and S39 (SPEC section 14), S37a (REQ 17 "trigger shape").
5. New context: seven other ladders as position bands (REQ 17 "ladder anchors"; DV-14 stays for entries), two trigger-volume readings (REQ 17 "volume"), round 2's candle-code sequences (REQ 7a), industries as universe values.
6. FADE on every level (REQ 17; DV-26 replaced).
7. The two Sniper gates, S-STOP and S-RR, as new stages (REQ 16b).
8. Requirements 27-30: E3, ORB-POS, PATTERN-AT-TOUCH, EU, ceiling B, accounts (A) and (C), as new stages (PREVIEW-SPEC section 25).

**Narrowed from the spec's words, by cost only:**
- 12.6's pairing with level 2 runs on coarse bands, and on 10 minutes and up. On 1, 2, 3 and 5 minutes level 2 x filters is the extension, not committed.
- Context filters on level 2 use the core lanes, not every lane. Exits get their full lattice on the seeds and in stage 2's drill (exits are their own dimension, SPEC:80).
- Each dial's fine values and "n up to the largest seen" are adaptive, not in the guaranteed part.
- 12.2's holding row "4 readings x 7 rungs + none" names 29; only 16 are distinct, and 16 are run.
- **The named cut:** `W2.singles.level1` on 1, 2, 3, 5 minutes, 1,185,746,400 combinations, 13.97 model hours (PREVIEW-SPEC 25.5).
- E3's deep filter stages run on one duration and lane XA, on the timeframes named in the stage table; EU's level 2 runs on one lane; ORB-POS's pairs start at 15 minutes; (A)'s pairs and triples run at the ceiling premium. Ceiling B on (A) holds seeds, base and level 1.

**The extension (requirement 17a; not committed).** After the committed search has reached 100 billion on the luck test's count and printed its verdict, while the CPU is free, the same resumable shards run on, CPU only: every stage on every timeframe (588,858,155,736 in all, about 834 model hours; each generated recipe once, 93,348,437,100), the named cut's units, then finer drill-down (`X.triples.base`, coarse triples on the other base nodes, lane XA), until each generated recipe counted once also passes 100,000,000,000. **The fixed order is by cost only: model hours per once-counted combination, cheapest first, one unit a stage on one timeframe: `combos/extension-order.tsv`, 305 units. By the model the once-counted figure passes 100 billion at unit 39 (`W2.orb.triples` on 10 minutes), about 74 model hours after the committed search, with 606,057,977,412 on the luck test's count.** ESTIMATE. Every series raises the luck bar for every chain, each report prints both figures, and PREVIEW-SPEC 25.5 says what line 2 reads while it runs.

| Stage | Committed table | Every timeframe | Extension |
|---|---|---|---|
| `A.W1.level2` | 1,986,894,720 | 5,177,021,184 | 3,190,126,464 |
| `A.W1.e3.level2` | 79,028,064 | 376,390,656 | 297,362,592 |
| `A.W1.eu.level2` | 15,233,184 | 29,113,344 | 13,880,160 |
| `A.W2.sniper.triples` | 2,951,677,920 | 5,397,121,872 | 2,445,443,952 |
| `A.W2.triples` | 19,324,468,968 | 34,403,700,024 | 15,079,231,056 |
| `A.W2.e3.sniper.triples` | 0 | 746,260,848 | 746,260,848 |
| `A.W2.e3.triples` | 217,209,840 | 4,580,081,928 | 4,362,872,088 |
| `A.W2.orb.triples` | 0 | 30,047,742,720 | 30,047,742,720 |
| `A.W2.sniper.pairs.base` | 113,050,272 | 174,277,344 | 61,227,072 |
| `A.W2.pairs.base` | 1,009,358,064 | 1,526,092,752 | 516,734,688 |
| `A.W2.sniper.singles.level1` | 51,544,224 | 78,040,800 | 26,496,576 |
| `A.W2.singles.level1` | 1,059,985,776 | 1,611,984,240 | 551,998,464 |
| `A.W2.e3.sniper.singles.level1` | 1,581,120 | 29,247,264 | 27,666,144 |
| `A.W2.e3.singles.level1` | 29,708,736 | 571,121,280 | 541,412,544 |
| `A.W2.orb.singles.level1` | 20,141,568 | 54,650,880 | 34,509,312 |
| `A.W2.sniper.pairs.seeds` | 181,222,272 | 383,376,240 | 202,153,968 |
| `A.W2.pairs.seeds` | 1,940,312,208 | 3,983,538,528 | 2,043,226,320 |
| `A.W2.e3.sniper.pairs.seeds` | 1,495,248 | 37,322,856 | 35,827,608 |
| `A.W2.e3.pairs.seeds` | 12,981,816 | 365,738,976 | 352,757,160 |
| `A.W2.orb.pairs.seeds` | 52,275,456 | 66,929,472 | 14,654,016 |
| `A.W2.sniper.singles.level2` | 730,020,096 | 2,280,919,680 | 1,550,899,584 |
| `A.W2.singles.level2` | 14,643,692,112 | 43,156,037,088 | 28,512,344,976 |
| `A.W2.e3.sniper.singles.level2` | 0 | 606,596,544 | 606,596,544 |
| `A.W2.e3.singles.level2` | 0 | 11,341,401,120 | 11,341,401,120 |
| `W1.level2` | 2,151,393,840 | 5,529,777,120 | 3,378,383,280 |
| `W1.e3.level2` | 79,178,400 | 381,147,840 | 301,969,440 |
| `W1.eu.level2` | 23,082,960 | 47,904,960 | 24,822,000 |
| `CB.W1.level2` | 2,151,393,840 | 5,529,777,120 | 3,378,383,280 |
| `W2.sniper.triples` | 11,594,075,880 | 20,801,838,600 | 9,207,762,720 |
| `W2.triples` | 95,027,264,640 | 165,085,971,120 | 70,058,706,480 |
| `W2.e3.sniper.triples` | 0 | 2,862,981,360 | 2,862,981,360 |
| `W2.e3.triples` | 1,075,825,800 | 21,651,361,200 | 20,575,535,400 |
| `W2.orb.triples` | 0 | 111,534,027,840 | 111,534,027,840 |
| `W2.sniper.pairs.base` | 351,371,520 | 536,551,200 | 185,179,680 |
| `W2.pairs.base` | 3,849,768,360 | 5,739,368,520 | 1,889,600,160 |
| `W2.sniper.singles.level1` | 105,648,300 | 119,760,660 | 14,112,360 |
| `W2.singles.level1` | 2,633,575,320 | 2,930,967,360 | 297,392,040 |
| `W2.e3.sniper.singles.level1` | 1,306,440 | 24,595,380 | 23,288,940 |
| `W2.e3.singles.level1` | 28,606,200 | 547,525,620 | 518,919,420 |
| `W2.orb.singles.level1` | 16,364,160 | 44,225,280 | 27,861,120 |
| `W2.sniper.pairs.seeds` | 547,336,440 | 1,142,525,520 | 595,189,080 |
| `W2.pairs.seeds` | 7,093,779,600 | 14,233,186,440 | 7,139,406,840 |
| `W2.e3.sniper.pairs.seeds` | 4,639,680 | 111,029,400 | 106,389,720 |
| `W2.e3.pairs.seeds` | 49,120,740 | 1,290,684,060 | 1,241,563,320 |
| `W2.orb.pairs.seeds` | 161,308,800 | 205,212,960 | 43,904,160 |
| `W2.sniper.singles.level2` | 802,841,760 | 2,481,542,640 | 1,678,700,880 |
| `W2.singles.level2` | 19,964,882,280 | 57,556,772,820 | 37,591,890,540 |
| `W2.e3.sniper.singles.level2` | 0 | 697,577,400 | 697,577,400 |
| `W2.e3.singles.level2` | 0 | 15,401,127,240 | 15,401,127,240 |
| **Total** | **197,480,655,060** | **588,858,155,736** | **391,377,500,676** |

## Not settled by this table

- O(1) at p99 (REQ 17) is benched per record step and per luck offer (17.7). An evaluation's whole work is O(its kept records), so it is not O(1) as a whole. That is why the model above weighs kept records, and why the mean kept steps are printed. The operator may read "per evaluation" more strictly than this.
- The kept-record guess and the selectivities in the model are mine, not measured. The gate and ORB-POS stages copy their mirrors' selectivities, also a guess: a gate such as S-RR at least 5 may keep far fewer records, which would make them cheaper, not dearer. E3's flat 20,000 records a side and account (A)'s 2 / 10 are guesses too.
- "Combos" here follows BRIEF 9 in counting each learning window. Counted once per recipe, the committed figure is 31,559,317,908, below 100 billion. The report prints both.
