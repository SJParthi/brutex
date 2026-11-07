PREVIEW FIB: the operator's real Fibonacci strategy (SPEC.md section 1 with the second-touch entry); not the production fib backtest of Thu 12 Nov; not a Winner Gate verdict
AT LEAST ONE RECIPE MEETS RS 2.5 LAKH EVERY UNSEEN MONTH: PENDING
current F&O members only (survivorship bias not removed)

# THE SNIPER VIEW: requirement 16b, as a plan written before any run

Written Sat 7 Nov 2026, about 09:25-09:40 IST, by a workflow step of main session d4256142. **Data-blind: no result, trades file or search output of any tool was read, and nothing has been run.** Every figure below is either a labelled seed, a rule that generates a value inside a learning window, or labelled arithmetic. This page is binding beside PREVIEW-SPEC.md; section 24 of that file is its short form and carries the amendments. Repaired about 10:05 IST after the independent check, round 1 (`sniper-check1.md`, `sniper-repair1.md`; 24.10 of the spec is the repair's short form): no count moved. Repaired again about 10:30 IST after round 2 (`sniper-check2.md`, `sniper-repair2.md`; 24.11 of the spec): the branch's twin in 2.3 is now named a partial null, and no count moved.

Sources: `state/operator-requirements-20261106.md` line 25 (16b), with items 1, 2, 16, 16a, 17, 18; PREVIEW-SPEC sections 7-16 and 23; COMBINATIONS.md; `combos/count.mjs`; COSTS.md; `build/INTERFACES.md`; `crates/fib_preview/src/types.rs` (read only).

## 0. In plain words

The operator's picture: **eagle's eyes** watch every F&O stock, timeframe and idea; **cheetah's chase** strikes at full size, with a tight stop, and leaves fast; **the sniper** takes one exact shot, and if that shot is not there, takes no trade. Few trades, tiny losses, big wins.

This plan turns that into four things the search can test:

1. **Two gates at the entry** (section 1): the stop must be small against the stock's own past, and the first target must be far against the stop. The search tries each with every band it generates on learning months. "Any" stays the seed, so the operator's own numbers still run unchanged in wave 0.
2. **A selection scope, SNIPER** (section 2): it ranks only among recipes with few trades a month and a small largest loss, on learning months only, and prints its own chains beside the headline.
3. **New facts on every record, path and series** (section 3), so the results can show how far price went against each trade.
4. **A SNIPER table and a plain check in every report** (sections 4-5).

Nothing here promises the picture exists in the data. A tight stop may simply get hit more often, and the test measures that. The bar still comes first, and it pulls against the few-trades goal: **a month with no trade fails the bar.** Section 5 shows the arithmetic, and it shows one problem that does not depend on any data: see 5.4.

## 1. The entry gates: new dials, dimension "Sniper gates"

Both are readings of the filter library, in new stages (section 6). A filter set takes at most one band from each reading, near-duplicates get no discount, and a generated value counts once per learning window: the existing rules. Neither gate moves a stop, a target or a size: they only keep or drop a signal. Entries stay equal in amount (10.2), so the rupee risk of a trade is its amount times its stop.

### 1.1 What is known at the decision (the look-ahead rule)

**An entry gate reads only what is known at the decision T.** The task's wording is "bp of the sizing price", but the sizing price of 10.2 is the last GDFL print strictly before the order's start, and the start is at least 14 s after T. Reading it would look 14 s ahead. So the gates read **REF, the sizing reference at the decision: the last GDFL print known at T.**

| Family | T | REF |
|---|---|---|
| Close reading (E1C, E2C) | the candle's close | `r1_g`, the last GDFL trade in the minute of the last 1-minute Zerodha candle closed by T, when `R1_OK` |
| Second reading (E1S, E2S) | the trigger second + 1 s | the last print of the trigger second |

No REF means both gate columns hold `ABSENT32`, which passes no band test; "any" holds no clause, so the record stays for the seed.

Stops and targets are the prices 7.3 already fixes from candles closed by T (pattern extreme, level, day extreme known at T, bounce high, the ladder at T), R1-mapped with the record's own `r1_z`, `r1_g`. Band edges come only from that window's learning months (12.1, 14.1). The walk's own `stop_dist`, the first fill, the sizing price and every path field are never read by a gate.

### 1.2 The formula (exact integers, no float, O(1))

For a LONG; a SHORT mirrors every price test. "Never kinder": a stop distance rounds up, a reward rounds down.

```text
stop_Z   = stop reference - offset                       (placement and offset of the walk's stop_ix)
target_Z = the lane's first target (1.4)
map(x)   = floor(x * r1_g / r1_z)  on a long;  ceil on a short     (both make the stop farther, the reward smaller)
D_stop   = REF - map(stop_Z)         on a long;  map(stop_Z) - REF on a short
D_tgt    = max(0, map(target_Z) - REF) on a long; max(0, REF - map(target_Z)) on a short

S-STOP   = ceil(D_stop * 1,000,000 / REF)      ppm of REF; 1 bp = 100 ppm; reports print bp
S-RR     = floor(D_tgt * 1,000 / D_stop)       thousandths: 2.000 is 2,000
```

- `D_stop <= 0`: the price is already at or through the stop at T. Both columns hold `ABSENT32`; every gate clause fails; F4 counts it by name, `sn_stop_through_at_decision`, per unit. On the seed stop it can arise only on the second reading; on the L placement also when a close sits beyond L.
- `D_tgt = 0`: the target is behind REF (a frozen ladder can do this). S-RR = 0, a real value.
- The gate does not round to the tick; the walk's tick-rounded stop can differ by under one tick. The walk's `stop_dist` is printed beside it.
- FADE (COMBINATIONS.md) sets its stop and target at the signal lane's distances, mirrored: it reads the same two distances. Its row stays labelled FADE.
- Mapping R2 walks hold no filter in the plan (`W1.exits1` only), so no gate reads them.

Worked vectors for F2's and F4's tests (Z = `r1_z`, G = `r1_g`, all paise, step 5):

| Case | Z, G, REF | Stop ref | Target ref | map(stop) | D_stop | S-STOP | map(target) | D_tgt | S-RR |
|---|---|---|---|---|---|---:|---|---|---:|
| Long, normal | 100000, 100020, 100020 | 99700 | 100600 + step | 99714 | 306 | 3,060 (30.60 bp) | 100625 | 605 | 1,977: fails "at least 2" |
| Short, normal | 100000, 99980, 99980 | 100300 | 99400 - step | 100285 | 305 | 3,051 | 99376 | 604 | 1,980 |
| Long, second reading, through | 100000, 100000, 99200 | 99700 | 100600 + step | 99695 | -495 | absent | - | - | absent |
| Long, target behind | 100000, 100020, 100020 | 99700 | 99895 + step | 99714 | 306 | 3,060 | 99919 | 0 | 0 |
| Long, tiny stop, offset 0 | 100000, 100000, 100000 | 99999 | 3100000 | 99999 | 1 | 10 | 3100000 | 3,000,000 | saturates by `col_value`, never `ABSENT32` |
| Long, exactly 2 | 100000, 100000, 100000 | 99995 | 100015 + step | 99990 | 10 | 100 | 100020 | 20 | 2,000: passes "at least 2" |

### 1.3 S-STOP: stop distance against the stock's own past

- **Seed:** any (no clause). Wave 0 and wave 1 hold no gate.
- **Generated (12.1):** in each window, per stock, on that timeframe and side, at this walk's stop; below 30 learning records (MIN_REF) the pooled unit's edge.
- **Full bands, 9, the usual set:** at or below own p25, p25-p50, p50-p75, at or above p75, at or below own p50, p25-p75, at or above p25, at or below p75, at or above p50.
- **Coarse, 4:** at or below own p25, at or below own p50, p25-p75, at or above p75. That is one more than the usual 3, because 16b names the small end and own p50 low is the operator's seed reading of "tight" (12.2). The wide-stop bands stay: nothing is pre-judged (REQ item 7).
- **Fine values:** where survivors crowd, inside `W2.fine`'s existing budget. Each one is labelled "generated: <quantile> of the stock's stop distances, window Y".

### 1.4 S-RR: reward to the first target over the stop

- **Seed:** any (no clause).
- **Labelled seeds:** at least 2, 3, 5 times the stop, class S, printed "seed (main session for REQ 16b's k; the operator gave no number)" by `ValueClass::SeedMain("REQ 16b's k")` (section 8). Never by `ValueClass::Seed`, which is the operator's own number and prints "seed (operator, <file:line>)", and never as a plain enumerated value. One series each, offered to all windows, unless something else in the recipe is generated. The label moves no count: COMBINATIONS.md still counts each as one series (E).
- **Generated:** the same 9 full bands, own quantiles, per window. **Coarse, 6:** the 3 seeds + at or below p25, p25-p75, at or above p75.
- **The first target of each lane:**

| Lane | S-RR reads | Note |
|---|---|---|
| XA | the day-extreme break + offset | the operator's first target (O7) |
| XB | the 1.272 rung at T | |
| XC | the 1.618 rung at T | |
| XD2, XD3 | the day-extreme break + offset | the operator's "then": this is where the stop moves to the entry; 1.272 or 1.618 comes second |
| XE | its arm, the day-extreme break + offset | labelled "reward to the arm": the trail then lets it run past, so the arm is a floor on the reward, not the reward |
| XF | its arm rung (seed 1.272) | labelled "reward to the arm" |
| Other rung targets (next rung, 2, 2.618, 3) | that rung at T | |
| Excursion target | REF + x R, x generated on learning months | |
| XJ | the bounce high + offset | E2 returns only; "= 1" has no XJ, as before |
| XA, XB, XC with S65's break-even | the lane's own target | S65 moves the stop later, not the reward |
| **XG** | **refused by name, `s_rr_no_target`** | a time exit has no price at the decision |
| **XH** | **refused by name, `s_rr_no_target`** | a condition exit has no price at the decision |
| **XI** | **refused by name, `s_rr_no_target`** | a Supertrend trail has no target |
| The no-stop comparison lane | both gates refused, `sn_no_stop` | shown chains only, never in a search |

A refused pair is never a combination: it is not evaluated and not counted (S-STOP still runs there). On tier A, S-RR runs on 14 of the 20 lanes: 10 enumerated and 4 generated.

## 2. The selection scope SNIPER (14.2)

**It adds no combination by itself:** same series, another heap, from wave 0 on. Gates add series to it from wave 2. Its candidates go through stage 2 like every scope's (section 6).

### 2.1 The cells

In each window Y, on learning months only, for every series with at least one learning trade:

- **Trades a month:** learning trades / learning months, the same average as the TPM scopes (R3:326). **Caps, labelled seeds: 2, 5, 10, 20** (class S; `Scope::Sniper` holds only the number, so F5 prints each cap through the same printer as `ValueClass::SeedMain("REQ 16b's trade caps")`: "seed (main session for REQ 16b's trade caps; the operator gave no number)"). Exact test: `trades <= cap x months`.
- **Largest loss:** the most negative single-trade net (worse clock, every charge) on the learning months, in rupees; a series with no losing trade has 0.
- **Generated bands:** p25, p50 and p75 of that reading over the window's series at or under that cap and at that series' N seed (one population per window x cap x N seed), so a Rs 5 lakh position is not compared with a Rs 1 crore one. "Below the band" means small for its own size.
- **Data-blind form for F4, O(1) a series:** the edges are unknown until the wave ends, so keep per (window, cap, N seed) a half-octave histogram of the largest loss (bucket from `leading_zeros`; class C, a resolution, never a strategy value) and a top-5 heap per bucket by THE RANKING. At the wave's end each edge is the quantile of the histogram's counts, snapped DOWN to a bucket edge (never kinder), printed "generated: p50 of N largest losses at or under cap c, N seed n, window Y = Rs x, walked at Rs y". A cell's heap is the merge of the buckets at or below the edge. Merges in fixed order, byte-equal at workers 1, 2, 8.

**12 cells: cap x band.** Inside a cell, **THE RANKING as usual** (14.2: passers first, then the miss key, then keys 2-8). Each cell's #1 per window chains over the 82 unseen months, on one carried account: **12 chains, `SNIPER-c-pb`.**

### 2.2 The one chain to read: SNIPER (pre-registered now)

So that no one picks a cell after seeing its months, each window's pick comes from one walk over the same 12 cells, tightest first (cap 2 p25, 2 p50, 2 p75, 5 p25, ... 20 p75). The walk reads learning months only, takes no typed number and adds no series. An empty cell is passed over at every step; two cells with one edge hold the same series, and the walk meets the first.

1. **A passer:** the #1 of the first cell that holds a learning-month passer (14.2's own test; section 9, point 6).
2. **If no cell holds one, a series that could still pass:** a no-trade month fails the bar, so a series with one has already missed it. Take the first cell holding a series with a trade in every learning month, and in it the best such series by the miss key (14.2).
3. **If no cell holds one of those, the loosest cell that holds anything,** walking back from cap 20 p75, and its #1 by the miss key. If every cell is empty, the window has no pick, named `sn_no_cell`, and its unseen months are no-trade months. A `SNIPER-c-pb` chain does the same in a window where its own cell is empty.

Printed as **SNIPER, beside the headline,** with the step and cell of each window ("Y2023: step 2, cell 2-p25"). A window from step 3 also says "no series in any cell traded in every learning month: not a sniper pick".

**Why step 2 exists.** 14.2 calls every candidate a miss today, and a passer needs every learning month at the bar, so the fixed floor of 24.R need not bring passers: steps 2 and 3 may decide many windows, perhaps all, and how many is unknown until the run. The miss key rewards total net, so it cannot favour a tight cell. With step 3 alone this chain would fall to cap 20 p75, the least sniper of the 12, and it is the chain the operator reads, though he said "always ensure" very few trades. So the walk chooses the cell by tightness, and the key only ranks inside it.

**For F4.** Beside each bucket's top-5 heap keep one slot: the best series by THE RANKING with a trade in every learning month of that window (one pass over a series' months gives its first no-trade month; each window's test is then O(1)). A cell's slot is the best over its buckets. In stage 2 a cell's candidates are its top 5, or its top 4 and its slot when the slot is not among them: still at most 5 a cell and 420 in all, so no count moves, and each `SNIPER-c-pb` still takes its cell's #1. The walk runs on the same heaps and slots, after each stage 2 as well, so it needs no pass of its own. MIX-WF-1's perturbation covers it: a change in months from LEARN[Y] on, trades removed included, may not change window Y's step, cell or pick. The twin runs the same walk.

### 2.3 The honest place of this chain

**SNIPER stays a labelled chain. It is not the headline and it never writes line 2.** This plan changes neither 14.2's headline procedure (GLOBAL pick #1 of each window) nor 14.5's verdict; 24.R's one point on rule 2's floor is the main session's ruling, apart from this plan. A switch of headline would be honest only by the operator's own word before wave 0's first print, recorded and sealed, with the old headline printed beside it. After any print, it is a new family judged on fresh months, never on these (19, last line). A chain chosen among chains on its unseen months is printed "chosen among N chains on their unseen months: NOT a pass" (14.5).

**Which bar.** In this section, in 24.2 and in 24.7, "the bar" means one thing: 14.5 rule 2's floor as line 2 reads it at the seal. Today that is the fixed Rs 2,50,000 net in every unseen month, with a trade in each (OP-P11's ruling, 24.R; REQ 1a). This branch reads the same floor function as line 2, so it can never be kinder than line 2, and its sentence names the floor. If the operator overrules 24.R so that line 2 reads the carried floor again (section 9, point 1), a chain that holds only the fixed floor does not get the sentences below. It prints this instead: "met the fixed Rs 2,50,000 in all 82 unseen months (reading c): not line 2's floor".

**If the SNIPER chain holds line 2's floor in all 82 unseen months, the report says exactly this:**

- **What it shows:** a rule fixed before the run, reading learning months only, chose recipes inside its trades-a-month cap and largest-loss band on those months (each window's step and cell printed, 2.2), and the chain held line 2's floor, named in the sentence ("the fixed Rs 2,50,000 in all 82 unseen months (24.R)"), on months it never read, under lane S fills and every charge. The carried reading (a) prints beside it, as x of 82 months.
- **What it does not show:** that the edge is real and not luck, until the same luck test and a twin of the same search clear it (14.4: the whole pipeline, real budgets, seeds 1-3); that it is the best chain; that it holds live. Line 2 still reads the headline. It stays PREVIEW, with survivorship, flat 5x leverage, the uncalibrated 14 s and every other label of section 20. It is 1 of 13 SNIPER chains and about 52 chains printed in all, and nothing reaches real money (REQ item 9).
- **It counts in the luck test.** Every series with a gate is offered to the one luck stream (14.3) and is in `tried_total`, as is everything the scope reads. The chain's own O (months at Rs 2,50,000 against Rs 20 lakh) is printed against the same L95. **Adding the gates raises the luck bar for every chain, the headline too:** that is the honest cost of 13.4 billion more tries.
- **The null test.** The twin runs the same scope. By 23.5 a twin's wave 2 runs only if the real headline meets the bar, and this adds one branch: if the SNIPER chain holds the fixed Rs 2,50,000 in all 82 unseen months while the headline does not meet line 2's floor, twin seed 1 runs the `W2.sniper.*` stages too (about 5.2 model hours on the model's guess), so the chain gets a partial null, named as one (below). A null test costs hours and writes nothing, so this one branch may fire on the fixed floor whichever reading line 2 takes. **It names the reading that fired it:** today "twin branch fired by line 2's floor, the fixed Rs 2,50,000 (24.R)"; after an overrule, for a chain short of the carried floor, "twin branch fired by the fixed Rs 2,50,000 (reading c): not line 2's floor".
- **One caution on the twin.** It breaks the link between dials and outcomes, including the mechanical one: a small stop gives small losses whether or not the entry is any good. So a real SNIPER chain beats the twin on drawdown for that reason alone. Two lines are therefore printed: the ordinary twin, and, in that same branch, the twin with every column permuted except S-STOP and S-RR (which stay with their records). Beating the second is a first sign that the entries add something beyond a tight stop; on a partial null it is no more than that.
- **What that branch is: a partial null, printed as one.** The real chain picks from every series of every stage (section 2), and 14.4's twin re-runs the whole pipeline with the real run's work budgets, on seeds 1-3. The branch's twin runs seed 1 on waves 0-1 and the seven `W2.sniper.*` stages of section 6 only, with no other wave 2 stage, no stage 2 and no adaptive row: 10.8% of the real options. A smaller search finds less luck, so its best chain is weaker and easier to beat. So both lines above print, word for word: "partial null: twin seed 1 only, waves 0-1 and W2.sniper.* only, 15,715,111,500 of the real 145,276,706,220 options; a smaller search finds less luck, so beating it is easier than 14.4's twin, and it does not clear the chain". The second twin is drawn on seed 1 as well, so those words hold on both lines.
- **The same words wherever the twin's chain is smaller.** Twin seed 1 always runs waves 0-1, so any print of the twin's SNIPER chains (SNIPER and any `SNIPER-c-pb`) from waves 0-1 alone names its stages the same way: "partial null: twin seed 1 only, waves 0-1 only, 2,278,875,840 of the real 145,276,706,220 options; a smaller search finds less luck, so beating it is easier than 14.4's twin, and it does not clear the chain". Wave 1's interim prints it too, though the real chain has searched only those stages so far: the line measures against the full search, and nothing at that interim clears anything. The figures print from the achieved stage counts, which F4 asserts against `combos.json`, so after a named cut (section 7) the real figure is the achieved one. The line drops only for a seed whose whole pipeline ran (by 23.5, once the real headline meets the bar).
- **A complete null** means all of wave 2 on the twin: about 75.6 model hours on top of waves 0-1's 8.75, so about 84.3 for one whole seed, and 14.4 asks for three, about 253 (ESTIMATE, the guaranteed stages only). That is the main session's call if the branch ever fires, and this page does not decide it (section 9, point 7).

## 3. The record fields, each with its owner

Everything is integer, paise and ppm. "Frozen" names a change only the main session can make (INTERFACES section 3); everything else fits inside the builder's own files.

| When | Field | Meaning | Owner |
|---|---|---|---|
| Day facts | (nothing new) | `step_raw`, the data start and the session clock as today | **F1** |
| At the decision | `sn_ref_px` | REF, paise | **F2** |
| At the decision | `sn_stop_paise`, `sn_stop_ppm` | the seed stop's distance (pattern extreme + 1 measured step): paise, and ppm of REF (bp in reports) | **F2** |
| At the decision | `sn_t1_paise`, `sn_t1_ppm` | the distance to XA's first target at the seed offset, paise and ppm of REF | **F2** |
| At the decision, per walk | `gen::sniper::distances(&SignalRec, &WalkKey, &WalkGrids, ..)` | 1.2's formula at the walk's own stop and first target, from the record's head and `sn_ref_px`; `s_rr_no_target` by name; O(1) | **F4** (a test proves it equals F2's four columns at the seed stop and XA) |
| At the walk | `rule::STOP_THROUGH_AT_FILL` (new bit, `1 << 11`) | the first fill's second already printed at or through the stop | **F3**; frozen: `types.rs` |
| On each walk path | `adv_paise: i32` | the adverse move before the exit: from the first fill to the worst print against the trade, first fill's second through the exit's first fill second, paise a share, at least 0 | **F3** |
| On each walk path | `fav_paise: i32` | the favourable move, the same span and form | **F3** |
| On each walk path | `fill_off: u16`, `stop_off: u16` | the first fill's second, and the second the stop's trigger printed (`NO_OFF` if it never did), from 09:15:00 | **F3** |
| Derived from the path | adverse and favourable in bp | `adv x 10,000 x qty / deployed`, bp of the average entry | **F5** |
| Derived from the path | in multiples of the stop | `adv x 1,000 / stop_dist`, thousandths; above 1,000 means price went beyond the planned stop | **F5** |
| Derived from the path | seconds held | `exit_off - fill_off`; a fully unfilled path has none. The search's `held_s` stays `exit_off - start_off` | **F3** gives, **F5** prints |
| Per series, each month | `win_sum`, `worst` (with `MonthCell`'s wins, losses, trades, net) | the sum of winning nets, the most negative net: two more O(1) updates a kept step | **F4**, in its own `SnMonth` beside `Series`, no frozen change |
| Per series, derived | largest loss, average win, average loss, payoff ratio, trades a month, losing months, worst month | on any month range, by prefix: payoff ratio = (win_sum / wins) / (loss_sum / losses) as an exact rational; a series with no loss prints "no loss" | **F4** computes, **F5** prints |
| Per chain | `Scope::Sniper { cap, band }`, `Scope::SniperPrimary` | the 13 chains | **F4**; frozen: `types.rs` |
| Per report | the SNIPER table, the plain check, the chain's lines | sections 4-5 | **F5** |

Notes for the builders:

- **F2:** claim the 5 columns in INTERFACES' reserved table, first come first served (the names bind, not the numbers), and name them in `PV/progress.md`. Fill them on every record. They are raw facts, not dials.
- **F4:** `col_table()` marks reserved columns `Banded`, wave 2. **The generator must skip these 5 by name;** the only Sniper dials are S-STOP and S-RR at the walk's own stop and target. A gate clause is walk-dependent while `kept_of` has no walk parameter, so hold two per-walk arrays in `UnitView` (your own type) and intersect.
- **F3:** the path facts go in a **sibling unit, not in `PathRow`**: `A-<id>-<rank>-<name>-<tf>m-<SIDE>-<FAM>-x<delta>`, `db::commit_unit`, `Format::Blob`, 12 bytes a path, walk-major, same order as the group file. So `PathRow` stays 28 bytes, `spoolfmt.rs` is untouched, and the search's hot path reads nothing more. Only F5 reads it, for picked and shown chains.
- **F4, the bound:** a skip by a proven bound (13.1) may change no counter (R3:525), and 2.1's histogram is a counter: its edges are quantiles over every series at or under a cap, and a skipped series has no known largest loss. So a skip must now also prove that the series enters no SNIPER histogram, heap or slot: **in every window it is offered to (14.1), it has no learning trade, or it is above every cap (more than 20 x learning months trades).** Otherwise it is evaluated. "No learning trade" is proven by no kept record in those months. "Above every cap" needs a proven lower bound on trades; kept records give only an upper one, since slots, the lock and admission can refuse a record, so without such a proof that branch never fires. The byte-equal proof of 13.1 now also compares the histograms, the edges, the cell heaps, the slots and the walk's picks.
- **F4, the S81 tie-break (a look-ahead found while writing this):** `PathRow.stop_dist`'s doc says the S81 (b) tie-break reads it, but that distance runs from the first fill, which is at or after the start second that admission decides at. S81 (b) must read the decision's distance (`sn_stop_paise`, or `distances()` at the walk's stop) instead.
- **Every new function:** 8+ extreme cases, 2 combinations, 1 out of the box (REQ item 17); the shuffle-the-future test on the gate columns; the MIX-WF-1 perturbation on the band edges and the largest-loss edges; flat p50 and p99 benches on `distances()` and one SNIPER offer. An evaluation's work still grows with its kept records, as named before.

## 4. The SNIPER table, in every report (16b)

Part 6 of the report (16.6), one row per chain, unseen months, with the same chain's learning months beside it. **The headline and SNIPER rows always print, each as one row with every column of this table, whatever their net** (REQ 16b (c)). REQ item 6's limit applies to full settings and the per-year and per-month tables: if neither chain nets +100%, those shrink to the one line naming the best (16.3), and these two rows stay whole beside it, marked "below +100%: table row only". The 12 cells print only at net at or above +100%. Closest misses print it too (16.7). Interim reports print it from wave 0.

| Column | Exactly |
|---|---|
| Trades a month | trades / 82, with the months above the chain's cap counted beside it |
| Win rate | net above 0 on the worse clock (SPEC:737) |
| Average win; average loss | Rs and % of the deployed amount |
| Payoff ratio | average win / average loss |
| Largest loss | Rs; % of Rs 20 lakh; % of that month's start capital |
| Max drawdown | as 16.4: tenths of Rs 20 lakh, Rs, bp of peak, dates |
| Worst month | Rs, and which month |
| Losing months | net below 0, with no-trade months counted beside it |
| Months at the bar | at line 2's floor (today the fixed Rs 2,50,000, 24.R), and at the carried reading (a) beside it, never deciding (5.4): both, always |
| Average holding time | average and p90, seconds from the first fill to the exit's last fill |
| Adverse move | average, p50, p90, worst, in bp and in multiples of the stop; winners and losers apart |
| Favourable move | the same; with the average given back before the exit |
| Stop hit share | stop exits / trades; inside the first minute, inside the first 5 minutes, each as a share of all trades and of the stop exits (`stop_off - fill_off` at or below 60 s, 300 s) |
| The plan against the fills | median S-STOP at the decision beside the median `stop_dist` at the first fill; average loss on a stop exit in multiples of the stop; the share with `STOP_THROUGH_AT_FILL` |

Under the table: the plain check (section 5), and the line "SNIPER is a labelled chain, chosen on learning months only; line 2 reads the headline".

## 5. The plain check the table prints

**Labelled arithmetic, not a result.** No market data was read. Charges are COSTS.md's calculator (`costs-evidence/calc/vec.js`), worst reading, worse clock, on a Rs 1,000 example share. F5 prints these from `price_trade`, and a test compares them with `combos/sniper-arith.json`. Run: `node combos/sniper-arith.mjs`.

### 5.1 The charges, the net a trade the bar needs, and the operator's picture

**A. The charges of one round trip at a flat price (Rs, worst reading, worse clock; Rs 1,000 example share; f = fill seconds a side).**

| N | Position | Era | f = 1 | f = 10 | f = 30 | f = 60 |
|---|---|---|---:|---:|---:|---:|
| 1 | Rs 1,00,00,000 | today (from 1 Oct 2024 the clocks agree in rate) | 3,610 | 4,034 | 4,978 | 6,394 |
| 1 | Rs 1,00,00,000 | Rs 345/cr top slab | 3,696 | 4,120 | 5,064 | 6,480 |
| 1 | Rs 1,00,00,000 | state stamp, before 1 Jul 2020 | 4,548 | 4,973 | 5,917 | 7,333 |
| 3 | Rs 33,33,333 | today (from 1 Oct 2024 the clocks agree in rate) | 1,244 | 1,669 | 2,613 | 3,541 |
| 3 | Rs 33,33,333 | Rs 345/cr top slab | 1,273 | 1,697 | 2,641 | 3,570 |
| 3 | Rs 33,33,333 | state stamp, before 1 Jul 2020 | 1,557 | 1,982 | 2,926 | 3,854 |
| 5 | Rs 20,00,000 | today (from 1 Oct 2024 the clocks agree in rate) | 771 | 1,196 | 2,126 | 2,140 |
| 5 | Rs 20,00,000 | Rs 345/cr top slab | 788 | 1,213 | 2,143 | 2,157 |
| 5 | Rs 20,00,000 | state stamp, before 1 Jul 2020 | 959 | 1,384 | 2,314 | 2,328 |
| 10 | Rs 10,00,000 | today (from 1 Oct 2024 the clocks agree in rate) | 416 | 841 | 1,077 | 1,077 |
| 10 | Rs 10,00,000 | Rs 345/cr top slab | 425 | 850 | 1,086 | 1,086 |
| 10 | Rs 10,00,000 | state stamp, before 1 Jul 2020 | 510 | 935 | 1,171 | 1,171 |
| 20 | Rs 5,00,000 | today (from 1 Oct 2024 the clocks agree in rate) | 239 | 546 | 546 | 546 |
| 20 | Rs 5,00,000 | Rs 345/cr top slab | 243 | 550 | 550 | 550 |
| 20 | Rs 5,00,000 | state stamp, before 1 Jul 2020 | 326 | 633 | 633 | 633 |

**B. The net a trade the bar needs, and the move that pays it, if every trade of the month pays alike (one fill a side).** The bar plus the data charge: Rs 2,50,588.82 a month.

| k trades a month | Net a trade (Rs) | N = 1: gross move | N = 3: gross move | N = 5: gross move | N = 10: gross move | N = 20: gross move |
|---:|---:|---:|---:|---:|---:|---:|
| 1 | 2,50,588 | 2.54% (2.55%) | 7.56% (7.57%) | 12.57% (12.58%) | 25.11% (25.12%) | 50.18% (50.20%) |
| 2 | 1,25,294 | 1.29% (1.30%) | 3.80% (3.81%) | 6.31% (6.32%) | 12.57% (12.59%) | 25.11% (25.13%) |
| 5 | 50,117 | 0.54% (0.55%) | 1.54% (1.55%) | 2.55% (2.56%) | 5.05% (5.07%) | 10.07% (10.09%) |
| 10 | 25,058 | 0.29% (0.30%) | 0.79% (0.80%) | 1.29% (1.30%) | 2.55% (2.56%) | 5.06% (5.08%) |
| 20 | 12,529 | 0.16% (0.17%) | 0.41% (0.42%) | 0.67% (0.68%) | 1.29% (1.30%) | 2.56% (2.57%) |

Each cell: the move of the position, entry to exit, on today's charges (in brackets, the dearest stretch, before 1 Jul 2020). N = 1 is Rs 1 crore on one position; N = 20 is Rs 5 lakh a position.

**C. The operator's picture at N = 1, Rs 1 crore: k trades, w winners, the rest stopped at exactly s bp with no slippage, today's charges.** How big the average winner must be, and that winner as a multiple of the stop (compare S-RR's seeds 2, 3, 5).

| Stop s (bp) | k trades | w winners | Each loser costs (Rs) | Each winner must gross (Rs) | Move | Multiple of the stop |
|---:|---:|---:|---:|---:|---:|---:|
| 10 | 2 | 1 | 13,608 | 2,67,900 | 2.68% | 26.8 |
| 10 | 5 | 2 | 13,608 | 1,49,400 | 1.49% | 14.9 |
| 10 | 5 | 3 | 13,608 | 96,300 | 0.96% | 9.6 |
| 10 | 10 | 3 | 13,608 | 1,19,000 | 1.19% | 11.9 |
| 10 | 10 | 5 | 13,608 | 67,400 | 0.67% | 6.7 |
| 10 | 20 | 6 | 13,608 | 77,200 | 0.77% | 7.7 |
| 10 | 20 | 10 | 13,608 | 42,300 | 0.42% | 4.2 |
| 25 | 2 | 1 | 28,606 | 2,82,900 | 2.83% | 11.3 |
| 25 | 5 | 2 | 28,606 | 1,71,900 | 1.72% | 6.9 |
| 25 | 5 | 3 | 28,606 | 1,06,300 | 1.06% | 4.3 |
| 25 | 10 | 3 | 28,606 | 1,54,000 | 1.54% | 6.2 |
| 25 | 10 | 5 | 28,606 | 82,400 | 0.82% | 3.3 |
| 25 | 20 | 6 | 28,606 | 1,12,200 | 1.12% | 4.5 |
| 25 | 20 | 10 | 28,606 | 57,300 | 0.57% | 2.3 |
| 50 | 2 | 1 | 53,602 | 3,07,900 | 3.08% | 6.2 |
| 50 | 5 | 2 | 53,602 | 2,09,400 | 2.09% | 4.2 |
| 50 | 5 | 3 | 53,602 | 1,23,000 | 1.23% | 2.5 |
| 50 | 10 | 3 | 53,602 | 2,12,300 | 2.12% | 4.2 |
| 50 | 10 | 5 | 53,602 | 1,07,400 | 1.07% | 2.1 |
| 50 | 20 | 6 | 53,602 | 1,70,500 | 1.71% | 3.4 |
| 50 | 20 | 10 | 53,602 | 82,300 | 0.82% | 1.6 |

**D. The carried floor, reading (a), printed beside line 2 since the OP-P11 ruling (24.R; it was line 2's floor when 14.5 was written), at its slowest possible growth: every month nets exactly the floor, max(Rs 2,50,000, 12.5% of the carried month-start capital), and the exposure stays capped at Rs 1 crore (10.1).**

| Unseen month | Carried capital (Rs) | Floor that month (Rs) | Floor as % of Rs 1 crore | Net a trade at 2 / 5 / 20 / 210 trades a month, % of Rs 1 crore |
|---:|---:|---:|---:|---|
| 1 | 20,00,000 | 2,50,000 | 2.50% | 1.25 / 0.50 / 0.13 / 0.01 |
| 7 | 40,54,573 | 5,06,821 | 5.07% | 2.53 / 1.01 / 0.25 / 0.02 |
| 13 | 82,19,781 | 10,27,472 | 10.27% | 5.14 / 2.05 / 0.51 / 0.05 |
| 19 | 1,66,63,852 | 20,82,981 | 20.83% | 10.41 / 4.17 / 1.04 / 0.10 |
| 25 | 3,37,82,403 | 42,22,800 | 42.23% | 21.11 / 8.45 / 2.11 / 0.20 |
| 37 | 13,88,41,983 | 1,73,55,247 | 173.55% | 86.78 / 34.71 / 8.68 / 0.83 |
| 49 | 57,06,25,372 | 7,13,28,171 | 713.28% | 356.64 / 142.66 / 35.66 / 3.40 |
| 61 | 2,34,52,07,897 | 29,31,50,987 | 2931.51% | 1465.75 / 586.30 / 146.58 / 13.96 |
| 73 | 9,63,85,48,065 | 1,20,48,18,508 | 12048.18% | 6024.09 / 2409.64 / 602.41 / 57.37 |
| 82 | 27,82,17,42,034 | 3,47,77,17,754 | 34777.18% | 17388.59 / 6955.44 / 1738.86 / 165.61 |

First unseen month in which each trade would have to net 20% of Rs 1 crore: 2 trades a month, month 25; 5 trades a month, month 33; 10 trades a month, month 39; 20 trades a month, month 45; 40 trades a month, month 50; 210 trades a month, month 65.

### 5.2 How to read it

- **At Rs 1 crore on one position (N = 1) and 2 trades a month, each trade must gross about 1.29%;** at 5 trades, about 0.54%; at 20, about 0.16%. If half of them lose, the winners must carry the losers too (table C).
- **With a 25 bp stop and 5 trades, 2 winners must each make about 6.9 times the stop;** S-RR's seed "at least 5" would not be enough there, and with 3 winners about 4.3 times is. These are the exact sums behind "few trades, big wins".
- **Fewer trades at the same bar means a bigger move each, not a smaller one.** Larger N makes it harder too: at N = 20 and 2 trades a month, each trade would need 25%.

### 5.3 The tension, printed on every table

- **A month with no trade fails the bar.** Very few trades and every month pull against each other: at a cap of 2, a recipe with a gap of one quiet month has already failed. The table prints how far each goal got: trades a month beside months at the bar, and the fewest-trades chain among passers or closest misses (16.6).
- **A tight stop meets 14 s of latency and worst-print fills.** Between the decision and the order's start, price moves freely; a stop that looked small at T can be close, or already through, at the first fill. It gets hit more often, and a stop exit fills at the worst print of each second, within 10% of that second's volume, so the loss can exceed the stop. Filling Rs 1 crore at 10% of each second's volume can also take many seconds, and each one can add up to Rs 23.60 of brokerage (table A; COSTS.md RD-BRK), while the stop can be hit with the entry part filled. **The test measures all of this. Nothing is assumed in either direction.** The last row of the table is there to show it.

### 5.4 A problem in the arithmetic itself: the carried floor has no realistic way to say YES

14.5 rule 2 as first written, on reading (a), the carried capital, needs every unseen month at or above max(Rs 2,50,000, 12.5% of the month-start capital). A chain that nets exactly that floor every month compounds at 12.5% a month, while the exposure stays capped at Rs 1 crore (10.1). Table D is that sum. **By unseen month 25 the floor is Rs 42 lakh, 42% of the exposure, in one month. By month 82 it is about Rs 348 crore, on Rs 1 crore of exposure, at any trade count: at 100 entries a day (10.1) and about 21 sessions, every one of those entries would have to net about 17% of Rs 1 crore.**

So on that reading rule 2 gives NO for every realistic chain, whatever the data holds, before the bar of Rs 2,50,000 is even tested. (It is not a theorem: 2,100 entries in one month, each netting 17%, would do it.) Fewer trades only brings the wall sooner: table D's last line. This does not depend on any market data, and it is not new with the Sniper view: R3:60 carries the same words.

**The ruling.** This plan itself changes nothing in 14.5. The main session has since ruled (OP-P11, 24.R; REQ 1a): line 2 judges the fixed Rs 2,50,000 net in every unseen month, the same floor as reading (c), C_m = Rs 20 lakh (DRAFTK:256), told to the operator, who can overrule. The carried reading prints beside it on every chain (section 4), and the luck test's O already reads the fixed Rs 2,50,000 against Rs 20 lakh (14.3).

**What the record holds, so that he can judge it.** His only words on the amount are "per month I need 2.5 lakhs" (U/every-month-positive-main-bar.md:29). "Salary" is an earlier note's paraphrase: at :22 it stands in the note writer's own words, outside the quotation marks. The carried base, capital at each month's start, was the main session's default, told to him and not changed (:18, :23); the "12.5% of a grown month-start capital if higher" at :29 and REQ:4 also stands outside his quotation. So his words settle it neither way, and the ruling replaces a default he did not change with a kinder floor. **So he is asked, with table D beside the question:** "Is Rs 2,50,000 a month the floor for good, or should the floor grow with the account? Table D is what growing means." If it grows, line 2 reads the carried floor again, 2.3's wording applies as written there, and the overrule is a new family in the luck test (24.R).

A related effect sits in the miss key. 14.2 and R3:614-618 rank misses by the total shortfall "to the bar". If that bar grew with each series' own carried capital, **more profit could raise the shortfall:** over 88 learning months from Rs 20 lakh, a series netting a flat Rs 2,50,000 a month owes about Rs 12.0 crore of shortfall, and one netting a flat Rs 5,00,000 a month, double in every month, owes about Rs 21.8 crore, so the worse one would rank first. Since nearly every candidate is a miss, this could decide most picks, SNIPER's and the headline's. **Ruled (OP-P12, 24.R): the miss key's bar is the fixed Rs 2,50,000,** so more net never raises the shortfall. F4 holds it behind one function, so that a later ruling is one line.

## 6. The recount

**The choice: new stages, not new values in the filter library.** Seven stages `W2.sniper.*`, each the same shape as its mirror and holding exactly the filter sets that take at least one gate value: the library with the gates less the library without them. The reasons: every earlier stage keeps its exact count, so what the builders and the 09:08 fingerprint hold stays true (`check.mjs` compares all 448 earlier cells with the backup); a finished unit is never reopened (the plan's "incremental" rule); the new dials' cost stays visible on its own rows; and each new stage can run first, right before its mirror, so the operator's newest ask is answered early and a deadline's cut, which takes whole units from the end, leaves it in.

| Figure | At 09:08 IST (fingerprint daf6c43c74b7) | Now | Change |
|---|---:|---:|---:|
| Guaranteed part | 131,840,470,560 | **145,276,706,220** | +13,436,235,660 |
| Stage 2, at most | 23,878,281 | 33,913,341 | +10,035,060 |
| Adaptive, at most | 943,488,000 | 968,889,600 | +25,401,600 |
| **Expected total** | 132,807,836,841 | **146,279,509,161** | +13,471,672,320 |
| Base-2 logarithm of the expected total | 36.951 | 37.090 | |
| Each generated recipe counted once | 21,222,014,340 | **23,247,542,760** | +2,025,528,420 |
| Wave 0, the operator's seeds | 27,480 | 27,480 | none |
| Wave 1 | 2,278,848,360 | 2,278,848,360 | none |
| Shards before chunking | 170,496 | 177,504 | +7,008 |
| The extension (every stage on every timeframe), not committed | 252,195,849,900 | 277,313,030,280 | |
| Model hours | 79.1 | **84.3** | +5.2 |

| New stage (runs right before its mirror) | Exact count | Model hours |
|---|---:|---:|
| `W2.sniper.singles.seeds` | 11,347,200 | 0.15 |
| `W2.sniper.singles.base` | 23,614,560 | 0.17 |
| `W2.sniper.triples` | 11,594,075,880 | 2.75 |
| `W2.sniper.pairs.base` | 351,371,520 | 0.16 |
| `W2.sniper.singles.level1` | 105,648,300 | 0.67 |
| `W2.sniper.pairs.seeds` | 547,336,440 | 0.84 |
| `W2.sniper.singles.level2` | 802,841,760 | 0.50 |
| **Total: 9.2% of the guaranteed part** | **13,436,235,660** | **5.24** |

The stage-2 rows `S2.sniper.*` are the 12 cells x top 5 x 7 windows, 420 candidates, with the month cap taking the labelled values 2 and 5 as well (9.3), so a cell's cap can also be enforced live, as a labelled lane, in stage 2. Those 2 and 5 are the main session's too, printed by `ValueClass::SeedMain("REQ 16b's stage 2 month cap")`; 9.3's own 10, 20, 40 stay the operator's. A cell's top 5 may hold its every-month slot in fifth place (2.2): still at most 420. `W2.sniper.stacks` is the same top 20 nodes x the gates' 21 values as a 4th to 6th filter. Finer gate bands share `W2.fine`'s budget, so it does not grow.

Requirement 17 stays judged on the guaranteed part, and without the Sniper stages it is still 131,840,470,560. F4 asserts every achieved figure against `combos/combos.json`, new stages included. Script guards: `count.mjs` stops if an earlier stage moves, and stops if the Sniper dimension's count differs from its stages' sum; `check.mjs` walks the new figures by nested loops and passes.

## 7. Can the search still finish?

**Yes. The new dials add little: 5.2 model hours of 84.3, for 10.2% more count, and they need no cut.**

- **The new estimate: about 56.3-84.3 hours of phase B wall time** at the rates measured on this Mac under load (12-13 threads), against 51.1-79.1 before. If a free Mac doubles the rate, about 28-42 hours; that is a condition, not a measurement. All of it is an ESTIMATE until the five-stock test measures it.
- **Not in the model, because nothing is measured:** the new per-series figures and the SNIPER heaps touch every series of every stage, including the old ones: about 3-10% by guess (one more sum and one more minimum a kept step; at most 28 heap checks a series, each with one more comparison for the every-month slot of 2.2; far fewer for most, which sit above every cap). The five-stock test must price this before the full run. **The bound's new proof (section 3) narrows what can be skipped:** a series with a learning trade that is not proven above cap 20 must now be evaluated, so the skip may save little. The count does not move, since a skip counted as tried either way; the hours may, and that too is measured there.
- **Spool:** the 12 bytes a path of F3's sibling unit bring the spool to about 104-424 GB at 40 bytes a path (ESTIMATE), inside the room above the disk floor.

**If the projection does not fit, cut by data-blind cost, the most hours per combination first, in this order; never by a result, and always named in the report.** Wave 0, wave 1 and each filter on the seeds and the base nodes are never cut: they are the first answers, about 16.5 model hours together.

| Order | Cut | Saves (model hours) | Count lost | Why this one |
|---|---|---:|---:|---|
| 1 | `W2.singles.level1` on 3 and 5 minutes | 6.7 | 0.84 B | about 8 hours a billion against 0.23 for `W2.triples`: full-spool lanes on many kept records |
| 2 | the rest of `W2.singles.level1` | 9.5 | 1.79 B | the same reason; its 1 and 2-minute parts cost the most per combination, and go after 3 and 5 only because they are the operator's own charts |
| 3 | `W2.pairs.seeds` | 10.5 | 7.09 B | 1.5 hours a billion |
| 4 | `W2.singles.level2` | 11.7 | 19.96 B | 0.6 hours a billion, the last before the triples |

All four together: **about 38.5 hours saved, about 45.8 model hours left, and 115.6 billion guaranteed, still above 100 billion.** The `W2.sniper.*` stages stay in every cut: they are the operator's newest word and cost only 0.39 hours a billion on average. `W2.pairs.base` and `W2.triples` go last, being the cheapest per combination.

## 8. What each builder does, and what needs the main session

| Who | What |
|---|---|
| F1 | nothing new |
| F2 | the 5 columns of section 3, on every record, with 1.2's vectors as tests; the shuffle-the-future test |
| F3 | the sibling unit (12 bytes a path), `STOP_THROUGH_AT_FILL`, the benches |
| F4 | `distances()`, the two generated gates, the refusals by name, `SnMonth`, the 12 cells, the every-month slots and the SNIPER chain's walk (2.2), the seven stages asserted against `combos.json`, S81 (b)'s repair, the bound's new proof, the twin's branch, the reading that fired it and the second twin on seed 1's draw (2.3) |
| F5 | the table, the plain check, the plain answer's SNIPER line, `S2.sniper.*`, the one printer of `ValueClass` with every Sniper seed through `SeedMain` (a test: no Sniper seed prints "seed (operator"); the partial-null line on every twin SNIPER print from less than 14.4's whole pipeline (2.3; a test: none prints without it); **nothing of this plan changes the verdict function** (24.R's one point on rule 2's floor is the main session's) |
| Main session (frozen `types.rs`) | add `Scope::Sniper { cap: u16, band: u8 }`, `Scope::SniperPrimary`, `rule::STOP_THROUGH_AT_FILL = 1 << 11`; **`ValueClass::SeedMain(&'static str)`**, printing "seed (main session for <what>; the operator gave no number)", doc line "the main session's, never the operator's"; **`SnWalkRow { window: u8, step: u8, cap: u16, band: u8 }` and `WaveOut.sn_walk: Vec<SnWalkRow>`**, the step and cell of 2.2 for F5 (step 0 is `sn_no_cell`, and `Chain.picks` then skips that window; section 4's "the chain's cap" reads it too); correct `PathRow.stop_dist`'s doc line on S81 (b) |

## 9. Open points

1. **Line 2's floor (5.4): ruled, and still his to overrule.** OP-P11's ruling (24.R): line 2 judges the fixed Rs 2,50,000, because the carried floor gives NO for every realistic chain, by arithmetic alone. His words are "per month I need 2.5 lakhs"; "salary" is an earlier note's paraphrase, and the carried base was the main session's default, told to him and not changed. So the question goes to him with table D (5.4). This plan prints both readings either way. Changing line 2 is his decision, and an overrule is a new family (24.R).
2. **The bar inside the miss key (5.4): ruled.** OP-P12 (24.R): the fixed Rs 2,50,000. It is monotone: more net never raises the shortfall. One function in F4.
3. **`Scope`, `rule`, `ValueClass` and `WaveOut` in frozen `types.rs`** (section 8): F3, F4 and F5 need those lines. The main session's note in `build/NOTES-shared.md` (09:5x) names the first three items only, so it needs one more line for `ValueClass::SeedMain` and `sn_walk`, and INTERFACES section 5's sentence on what `ValueClass` prints gains the third form.
4. **The seeds 2, 3, 5, the caps 2, 5, 10, 20 and stage 2's month cap 2, 5 are the main session's numbers,** not the operator's; 16b says "labelled seeds". They print through `ValueClass::SeedMain` (section 8). He can change them before a run; after one, a change is a new family (19).
5. **This plan is not sealed by this page,** and the 09:08 fingerprint no longer covers it. COMBINATIONS.md, `combos/*` and section 24 need a new seal before any run. The video's facts still quote `daf6c43c74b7`.
6. **THE RANKING's passer test (14.2 rule 1): one line from the main session, before F4 seals.** 24.R names line 2, 14.5 rule 2 and the miss key. It does not name 14.2's own test, which still reads max(Rs 2,50,000, 12.5% of its start capital), on learning months when choosing and on unseen months when judging (R3:603), so verdict rule 4 reads it too. Left so, a series at the fixed floor in every learning month is a miss with zero shortfall, ranked by worst month and not by lowest max drawdown, and a chain can hold line 2's floor and still rank as a miss against its twin. 2.2's step 1 reads 14.2's own function, so it follows whichever is ruled. This page does not rule it: it touches the headline's pick.
7. **A complete null, if 2.3's branch ever fires: the main session's call.** The branch gives a partial null only, and that cannot clear the chain. All of wave 2 on the twin is about 75.6 model hours more, about 84.3 for one whole seed, and 14.4 asks for three (ESTIMATE). Nothing needs ruling before the run.

## 10. Not verified

Nothing here was compiled or run. The model hours, the selectivities, the 3-10% and the spool are estimates. The charges are the calculator's, not yet compared with `price_trade` on these sizes (F5's test). The Rs 1,000 share is an example: at other prices, rounding moves a charge by a rupee or two. Table C's losers lose exactly the stop, with no slippage, which the real fills will not honour; that is what section 4's last row measures.

## Rulings by the main session (Sat 7 Nov 2026 ~09:5x IST): see PREVIEW-SPEC.md 24.R
OP-P11: line 2 judges the fixed Rs 2,50,000; the 12.5%-of-capital reading and +2.5% are printed beside. OP-P12: the miss key's bar is the same fixed Rs 2,50,000.

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

## Final pass (Sat 7 Nov 2026 ~13:2x IST; pointer only; no earlier line of this page was edited)

PREVIEW-SPEC.md section 25 is read first and wins over this page. Section 5's tables and 2.1's reason, at Rs 1 crore on the worst reading, are replaced by `combos/sniper-arith.md`, `.json` at Rs 15 lakh on the official reading, with 25.4's sentences. Section 7's figures and cuts are replaced by 25.5. The chain counts are replaced by 25.2.8. The caps 40, 60, 100 of 24.U are not taken (25.7).
