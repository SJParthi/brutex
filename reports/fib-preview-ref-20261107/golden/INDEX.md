# INDEX: FIB-PREVIEW-REF golden scenarios

PREVIEW: an independent reference build on seeded synthetic data; not a result, says nothing about profit

267 scenarios. Readings other than the headline are named (report section 4).

| Id | Kind | Readings | Outputs | Title |
|---|---|---|---:|---|
| R001 | rank | headline | 6 | competition ranks on a seeded pool with an exact tie |
| R002 | rank | headline | 21 | competition ranks on a seeded pool with an exact tie |
| R003 | rank | headline | 14 | competition ranks on a seeded pool with an exact tie |
| R004 | rank | headline | 4 | competition ranks on a seeded pool with an exact tie |
| R005 | rank | headline | 19 | competition ranks on a seeded pool with an exact tie |
| R006 | rank | headline | 36 | competition ranks on a seeded pool with an exact tie |
| R007 | rank | headline | 18 | competition ranks on a seeded pool with an exact tie |
| R008 | rank | headline | 6 | competition ranks on a seeded pool with an exact tie |
| R009 | rank | headline | 33 | competition ranks on a seeded pool with an exact tie |
| R010 | rank | headline | 7 | competition ranks on a seeded pool with an exact tie |
| R011 | rank | headline | 12 | a falling market: the sign readings differ for the long pool (A01) |
| R012 | rank | headline | 8 | eight equal moves share rank 1 both ways |
| R013 | rank | headline | 1 | one stock is first both ways |
| R014 | rank | headline | 2 | no previous close leaves the pool; a stale price over the limit leaves it |
| R015 | rank | headline | 15 | a twelve-way tie at the edge of top 10 includes all twelve |
| L001 | ladder | headline | 71 | the running ladder on 1 minute candles, a last candle with both extremes |
| L002 | ladder | headline | 71 | the running ladder on 2 minute candles, a last candle with both extremes |
| L003 | ladder | headline | 71 | the running ladder on 3 minute candles, a last candle with both extremes |
| L004 | ladder | headline | 70 | the running ladder on 5 minute candles, a last candle with both extremes |
| L005 | ladder | headline | 71 | the running ladder on 10 minute candles, a last candle with both extremes |
| L006 | ladder | headline | 74 | the running ladder on 15 minute candles, a last candle with both extremes |
| L007 | ladder | headline | 38 | the running ladder on 30 minute candles, a last candle with both extremes |
| L008 | ladder | headline | 20 | the running ladder on 60 minute candles, a last candle with both extremes |
| L009 | ladder | headline | 13 | a candle with both extremes: Undetermined until a later one-sided extreme |
| L010 | ladder | headline | 12 | equal extremes, strict reading (A02 headline) |
| L011 | ladder | A02=latest | 12 | equal extremes, latest reading (A02 other) |
| L012 | ladder | headline | 69 | a Kite hole taints the ladder for the session (A03 headline) |
| L013 | ladder | A03=extreme_candles | 87 | the same hole under the extreme-candles reading (A03 other) |
| L014 | ladder | headline | 24 | a padded minute in which GDFL traded is a hole |
| L015 | ladder | headline | 30 | a padded minute with no GDFL trade is held |
| L016 | ladder | headline | 60 | a falling day: a Down leg and its extensions below the low |
| E1-001 | e1 | headline | 35 | the operator's flag, K = 1, long: the break at candle 16 inside a seeded random day |
| E1-002 | e1 | headline | 30 | the operator's flag, K = 2, long: the break at candle 17 inside a seeded random day |
| E1-003 | e1 | headline | 33 | the operator's flag, K = 3, long: the break at candle 18 inside a seeded random day |
| E1-004 | e1 | headline | 32 | the operator's flag, K = 4, long: the break at candle 19 inside a seeded random day |
| E1-005 | e1 | headline | 35 | the operator's flag, K = 1, short: the break at candle 16 inside a seeded random day |
| E1-006 | e1 | headline | 32 | the operator's flag, K = 2, short: the break at candle 17 inside a seeded random day |
| E1-007 | e1 | headline | 33 | the operator's flag, K = 3, short: the break at candle 18 inside a seeded random day |
| E1-008 | e1 | headline | 20 | the flag on 2 minute candles |
| E1-009 | e1 | headline | 22 | the flag on 3 minute candles |
| E1-010 | e1 | headline | 18 | the flag on 5 minute candles |
| E1-011 | e1 | headline | 20 | the flag on 15 minute candles |
| E1-012 | e1 | headline | 30 | not in top N at the break: refused by the rank clause |
| E1-013 | e1 | headline | 30 | a red break candle: refused by the colour clause |
| E1-014 | e1 | headline | 30 | the same red break with the colour clause off |
| E1-015 | e1 | headline | 30 | a tight clause the box fails (height in ppm of R) |
| E1-016 | e1 | headline | 32 | a deep box under the holding rule 'all lows' |
| E1-017 | e1 | headline | 32 | a deep box under the holding rule 'all closes' |
| E1-018 | e1 | headline | 32 | a deep box under the holding rule 'no close below the first low' |
| E1-019 | e1 | headline | 32 | a deep box under the holding rule 'none' |
| E1-020 | e1 | headline | 30 | frozen ladder mode at the box's first candle |
| E1-021 | e1 | headline | 31 | a Kite hole inside the box voids the scope (incomplete_candle) |
| E1-022 | e1 | headline | 33 | a hole early in the day taints the ladder: tainted_ladder (A03 headline) |
| E1-023 | e1 | A03=extreme_candles | 33 | the same early hole under the extreme-candles taint (A03 other) |
| E1-024 | e1 | headline | 31 | breaks without a new high: consumed until a new extreme (A05 headline) |
| E1-025 | e1 | A05=next_box | 33 | the same breaks re-armed on the next box (A05 other) |
| E1-026 | e1 | headline | 30 | a slow box: the sliding box (A06 headline) |
| E1-027 | e1 | A06=anchored | 28 | the same slow box, anchored after the push (A06 other) |
| E1-028 | e1 | A17=judging | 30 | rung targets on the judging ladder (A17 other) |
| E1-029 | e1 | headline | 26 | E1S, the first second beyond the box, K = 2, long |
| E1-030 | e1 | headline | 27 | E1S, the first second beyond the box, K = 3, short |
| E1-031 | e1 | headline | 26 | E1S with no GDFL price in R1's minute: no_trigger_ratio |
| E2-001 | e2 | headline | 47 | the second touch of 0.786, close reading, long |
| E2-002 | e2 | headline | 47 | the second touch of 0.786, close reading, short |
| E2-003 | e2 | headline | 47 | touch clause = 1, the baseline |
| E2-004 | e2 | headline | 47 | touch clause = 3 |
| E2-005 | e2 | headline | 47 | touch clause >= 2 |
| E2-006 | e2 | headline | 47 | touch clause >= 3 |
| E2-007 | e2 | headline | 41 | bounce m = 2: 0.5 must be reached |
| E2-008 | e2 | headline | 41 | rung 0.618 |
| E2-009 | e2 | headline | 47 | a tolerance of 2 paise |
| E2-010 | e2 | headline | 47 | frozen at the first touch |
| E2-011 | e2 | headline | 47 | a touch candle closing below the level: the seed reading counts on |
| E2-012 | e2 | headline | 41 | the same close below under S29b's labelled reading: the count ends |
| E2-013 | e2 | headline | 47 | the resting reading is refused by name |
| E2-014 | e2 | headline | 47 | out of top N at the return |
| E2-015 | e2 | headline | 44 | a through candle wipes the bounce before the return |
| E2-016 | e2 | headline | 41 | a Kite hole ends the count: touch_count_unknown |
| E2-017 | e2 | headline | 49 | a new high mid-count restarts the running count |
| E2-018 | e2 | headline | 49 | the same new high under the frozen ladder keeps the count |
| E2-019 | e2 | headline | 47 | a re-touch before the bounce passes joins the touch (A07 headline) |
| E2-020 | e2 | A07=ignore | 47 | the same re-touch ignored for the count (A07 other) |
| E2-021 | e2 | headline | 44 | a touch from the unfavourable side is not a first touch |
| E2-022 | e2 | headline | 47 | the touch book on 2 minute candles |
| E2-023 | e2 | headline | 47 | the touch book on 3 minute candles |
| E2-024 | e2 | headline | 45 | E2S armed at the bounce, triggered by a GDFL second, long |
| E2-025 | e2 | headline | 45 | E2S armed at the bounce, triggered by a GDFL second, short |
| E2-026 | e2 | headline | 45 | E2S with GDFL seconds missing until the return candle |
| E2-027 | e2 | headline | 41 | E2S with no GDFL price in R1's minute: no_trigger_ratio |
| E3-001 | e3 | headline | 36 | ORB 1 minutes on 1 minute candles, close reading, long |
| E3-002 | e3 | headline | 37 | ORB 2 minutes on 1 minute candles, close reading, short |
| E3-003 | e3 | headline | 38 | ORB 3 minutes on 1 minute candles, close reading, long |
| E3-004 | e3 | headline | 40 | ORB 5 minutes on 1 minute candles, close reading, short |
| E3-005 | e3 | headline | 49 | ORB 10 minutes on 1 minute candles, close reading, long |
| E3-006 | e3 | headline | 12 | ORB 15 minutes on 5 minute candles, close reading, short |
| E3-007 | e3 | headline | 15 | ORB 30 minutes on 5 minute candles, close reading, long |
| E3-008 | e3 | headline | 21 | ORB 60 minutes on 5 minute candles, close reading, short |
| E3-009 | e3 | headline | 45 | ORB 5, the break ordinal clause 'second' |
| E3-010 | e3 | headline | 45 | ORB 5, the break ordinal clause 'second_or_later' |
| E3-011 | e3 | headline | 45 | ORB 5, the break ordinal clause 'any' |
| E3-012 | e3 | headline | 41 | a hole inside the range: orb_tainted |
| E3-013 | e3 | headline | 41 | a one-minute range with no width: orb_no_width |
| E3-014 | e3 | headline | 30 | E3S, the first second beyond the range, long |
| E3-015 | e3 | headline | 30 | E3S, the first second beyond the range, short |
| E3-016 | e3 | headline | 9 | E3S on 5 minute candles, ORB 5: the boundary candle writes no record (A08 headline) |
| E3-017 | e3 | A08=arms | 11 | the same, the boundary candle arms (A08 other) |
| E3-018 | e3 | headline | 31 | E3S with GDFL seconds only from 09:35 |
| G001 | gate | headline | 1 | SNIPER.md 1.2 long, normal |
| G002 | gate | headline | 1 | SNIPER.md 1.2 short, normal |
| G003 | gate | headline | 1 | SNIPER.md 1.2 long, second reading, through: sn_stop_through_at_decision |
| G004 | gate | headline | 1 | SNIPER.md 1.2 long, target behind |
| G005 | gate | headline | 1 | SNIPER.md 1.2 long, tiny stop, offset 0 |
| G006 | gate | headline | 1 | SNIPER.md 1.2 long, exactly 2 |
| G007 | gate | headline | 1 | 24.13: charges decide, passes nothing |
| G008 | gate | headline | 1 | 24.13: reward below charges (floor, not truncation) |
| G009 | gate | headline | 1 | 24.13: the same record at a smaller amount |
| G010 | gate | headline | 1 | 24.13: the normal vector |
| G011 | gate | headline | 1 | 24.13: target behind, a real negative value |
| G012 | gate | headline | 1 | 24.13: a wide stop |
| G013 | gate | headline | 1 | XG, XH, XI have no price at T: s_rr_no_target (S-STOP still runs) |
| G014 | gate | headline | 1 | REF above the amount: sn_no_quantity |
| G015 | gate | headline | 1 | seeded gate case 1, long |
| G016 | gate | headline | 1 | seeded gate case 2, short |
| G017 | gate | headline | 1 | seeded gate case 3, long |
| G018 | gate | headline | 1 | seeded gate case 4, short |
| G019 | gate | headline | 1 | seeded gate case 5, long |
| G020 | gate | headline | 1 | seeded gate case 6, short |
| G021 | gate | headline | 1 | seeded gate case 7, long |
| G022 | gate | headline | 1 | seeded gate case 8, short |
| G023 | gate | headline | 1 | seeded gate case 9, long |
| G024 | gate | headline | 1 | seeded gate case 10, short |
| G025 | gate | headline | 1 | seeded gate case 11, long |
| G026 | gate | headline | 1 | seeded gate case 12, short |
| W001 | walk | headline | 11 | 14 s latency, 10% of each second's volume, a second with no trade, worst prints, a target |
| W002 | walk | headline | 9 | a stop and a target in one second: the stop |
| W003 | walk | headline | 9 | a stop and a target in one second, short: the stop |
| W004 | walk | headline | 9 | a stop through at the first fill: entry first (A09 headline) |
| W005 | walk | A09=exit_first | 9 | the same, exit first (A09 other) |
| W006 | walk | headline | 10 | protection blocks a far print; the stop's second fills the entry first |
| W007 | walk | A09=exit_first | 9 | the same, exit first: the part-filled entry exits its part |
| W008 | walk | headline | 9 | a short: sells at the low, buys back at the high |
| W009 | walk | headline | 10 | XD: at the day-high break the stop moves to the entry after 14 s |
| W010 | walk | headline | 11 | XE: a trailing stop armed at the break, each move 14 s late |
| W011 | walk | headline | 9 | XG/XH: a market exit decided at a candle close, started 14 s later |
| W012 | walk | headline | 1 | a market exit before any fill cancels the entry: no trade |
| W013 | walk | headline | 9 | a market exit starting at 15:08:04 is rejected; the broker squares off at 15:10 |
| W014 | walk | headline | 9 | no traded second from 15:10: the worst since the entry, unclosable_in_session |
| W015 | walk | headline | 1 | an entry starting at or after 15:08:00 is cancelled before it starts |
| W016 | walk | headline | 9 | a resting stop printing at 15:09 acts (A10 headline) |
| W017 | walk | A10=rejected | 9 | the same stop rejected after 15:08: the broker squares off (A10 other) |
| W018 | walk | headline | 24 | an entry still filling at 15:08:00 has its remainder cancelled |
| W019 | walk | headline | 9 | an auction-era stock-day: the square-off reads 15:10:00 to 15:14:59 only |
| W020 | walk | headline | 1 | an order starting before the GDFL data start: no_fill_data_window |
| W021 | walk | headline | 1 | no print before the start to size it: no_sizing_price |
| W022 | walk | headline | 1 | the sizing price above the amount: price_above_position |
| W023 | walk | headline | 21 | seeded walk 1 on generated seconds, long |
| W024 | walk | headline | 13 | seeded walk 2 on generated seconds, short |
| W025 | walk | headline | 12 | seeded walk 3 on generated seconds, long |
| W026 | walk | headline | 10 | seeded walk 4 on generated seconds, short |
| W027 | walk | headline | 10 | seeded walk 5 on generated seconds, long |
| W028 | walk | headline | 94 | seeded walk 6 on generated seconds, short |
| W029 | walk | headline | 18 | seeded walk 7 on generated seconds, long |
| W030 | walk | headline | 16 | seeded walk 8 on generated seconds, short |
| W031 | walk | headline | 11 | seeded walk 9 on generated seconds, long |
| W032 | walk | headline | 10 | seeded walk 10 on generated seconds, short |
| W033 | walk | headline | 15 | seeded walk 11 on generated seconds, long |
| W034 | walk | headline | 19 | seeded walk 12 on generated seconds, short |
| W035 | walk | headline | 39 | seeded walk 13 on generated seconds, long |
| W036 | walk | headline | 11 | seeded walk 14 on generated seconds, short |
| W037 | walk | headline | 11 | seeded walk 15 on generated seconds, long |
| W038 | walk | headline | 20 | seeded walk 16 on generated seconds, short |
| C001 | charges | headline | 9 | COSTS.md contract note CN01, 2018-09-03 |
| C002 | charges | headline | 9 | COSTS.md contract note CN02, 2019-03-29 |
| C003 | charges | headline | 9 | COSTS.md contract note CN03, 2019-04-01 |
| C004 | charges | headline | 9 | COSTS.md contract note CN04, 2020-05-29 |
| C005 | charges | headline | 9 | COSTS.md contract note CN05, 2020-06-01 |
| C006 | charges | headline | 9 | COSTS.md contract note CN06, 2020-06-30 |
| C007 | charges | headline | 9 | COSTS.md contract note CN07, 2020-07-01 |
| C008 | charges | headline | 9 | COSTS.md contract note CN08, 2020-12-31 |
| C009 | charges | headline | 9 | COSTS.md contract note CN09, 2021-01-01 |
| C010 | charges | headline | 9 | COSTS.md contract note CN10, 2021-03-31 |
| C011 | charges | headline | 9 | COSTS.md contract note CN11, 2021-04-01 |
| C012 | charges | headline | 9 | COSTS.md contract note CN12, 2022-07-15 |
| C013 | charges | headline | 9 | COSTS.md contract note CN13, 2022-07-18 |
| C014 | charges | headline | 9 | COSTS.md contract note CN14, 2023-03-31 |
| C015 | charges | headline | 9 | COSTS.md contract note CN15, 2023-04-03 |
| C016 | charges | headline | 9 | COSTS.md contract note CN16, 2024-03-28 |
| C017 | charges | headline | 9 | COSTS.md contract note CN17, 2024-04-01 |
| C018 | charges | headline | 9 | COSTS.md contract note CN18, 2024-09-30 |
| C019 | charges | headline | 9 | COSTS.md contract note CN19, 2024-10-01 |
| C020 | charges | headline | 9 | COSTS.md contract note CN20, 2026-02-27 |
| C021 | charges | headline | 9 | COSTS.md contract note CN21, 2026-03-02 |
| C022 | charges | headline | 6 | COSTS.md contract note CN22, 2026-10-30 |
| C023 | charges | headline | 6 | COSTS.md contract note CN23, 2026-10-30 |
| C024 | charges | headline | 9 | COSTS.md contract note CN24, 2026-10-30 |
| C025 | charges | headline | 9 | the Adani screen: 547 x Rs 2,742.00, official reading Rs 579.41, Dhan's estimator Rs 578.85 |
| C026 | charges | headline | 4 | the HDFCBANK 770 CE screen: 11,700 x Rs 26.95, Dhan Rs 794.67; official option reading |
| C027 | charges | headline | 4 | a Rs 30,000 premium: flat Rs 20 brokerage (A16 headline) |
| C028 | charges | A16=min20 | 4 | the same premium, min(Rs 20, 0.03%) (A16 other) |
| C029 | charges | headline | 9 | a Rs 2.5 lakh round trip on 2019-06-03: Telangana's stamp (A15 headline) |
| C030 | charges | A15=literal | 9 | the same trade, every listed state: Himachal Pradesh's Rs 50 (A15 other) |
| C031 | charges | headline | 9 | a Rs 15 lakh round trip on 2019-06-03 above Rs 16.67 lakh of turnover: Tamil Nadu |
| C032 | charges | headline | 6 | seeded multi-fill trade 1 on 2020-05-30 |
| C033 | charges | headline | 6 | seeded multi-fill trade 2 on 2026-07-24 |
| C034 | charges | headline | 6 | seeded multi-fill trade 3 on 2023-10-16 |
| C035 | charges | headline | 6 | seeded multi-fill trade 4 on 2022-07-29 |
| C036 | charges | headline | 6 | seeded multi-fill trade 5 on 2023-11-26 |
| C037 | charges | headline | 6 | seeded multi-fill trade 6 on 2021-05-01 |
| A001 | account | headline | 14 | the 6th and 7th trades of a day: day cap |
| A002 | account | headline | 16 | the same signals under ceiling B: ten a day at Rs 10 lakh |
| A003 | account | headline | 227 | the 101st trade of a calendar month: month cap |
| A004 | account | headline | 6 | one position per stock (while_open) and N = 1's slot, inclusive lock (A18 headline) |
| A005 | account | A18=exclusive | 7 | the same, free in the last fill's second (A18 other) |
| A006 | account | headline | 11 | the margin check on Rs 13 lakh: one step down does not fit (margin_cap) |
| A007 | account | headline | 12 | the margin check on Rs 14.01 lakh: one step down to Rs 10 lakh fits |
| A008 | account | headline | 11 | on Rs 14 lakh the month's data charge, debited at the first fill, leaves Rs 13,99,411.18: the step down no longer fits (A14 headline) |
| A009 | account | A14=out_of_curve | 12 | the same on Rs 14 lakh with the data charge out of the curve: it fits (A14 other) |
| A010 | account | headline | 8 | dated leverage 3x: the main account enters Rs 7.5 lakh |
| A011 | account | headline | 8 | dated leverage 3x: ceiling B enters Rs 5 lakh |
| A012 | account | headline | 5 | leverage 2x: Rs 5 lakh, below N = 1's half step, allowed (A12 headline) |
| A013 | account | A12=refused | 3 | the same, refused below the half step (A12 other) |
| A014 | account | headline | 10 | C_VAR1 margins 20%, 35%, 50%, 65%: max(20, M) quantises each |
| A015 | account | headline | 9 | five zero-fill entries release the day's count (A11 headline) |
| A016 | account | A11=kept | 7 | the same, counted for good (A11 other) |
| A017 | account | headline | 9 | months and drawdown, the data charge in the curve, floor tenths (A13, A14 headline) |
| A018 | account | A13=ceil | 9 | the same, ceil tenths (A13 other) |
| A019 | account | A14=out_of_curve | 9 | the same, the data charge out of the curve (A14 other) |
| A020 | account | headline | 5 | a walk refusal (price_above_position) admits nothing |
| A021 | account | headline | 2 | N = 20 at 1x on Rs 1,000 of equity: size_refused |
| A022 | account | headline | 94 | seeded account 1 at N = 1 |
| A023 | account | headline | 45 | seeded account 2 at N = 3 |
| A024 | account | headline | 68 | seeded account 3 at N = 5 |
| A025 | account | headline | 81 | seeded account 4 at N = 10 |
| A026 | account | headline | 72 | seeded account 5 at N = 20 |
| A027 | account | headline | 76 | seeded account 6 at N = 2 |
| K001 | walkforward | headline | 118 | THE RANKING's picks over seven windows, seeded candidates 1 |
| K002 | walkforward | headline | 118 | THE RANKING's picks over seven windows, seeded candidates 2 |
| K003 | walkforward | headline | 118 | THE RANKING's picks over seven windows, seeded candidates 3 |
| K004 | walkforward | headline | 118 | THE RANKING's picks over seven windows, seeded candidates 4 |
| K005 | walkforward | headline | 118 | THE RANKING's picks over seven windows, seeded candidates 5 |
| K006 | walkforward | headline | 118 | THE RANKING's picks over seven windows, seeded candidates 6 |
| K007 | walkforward | headline | 104 | a passer in every month beats a miss with far more net |
| K008 | walkforward | headline | 104 | two passers: the lower max drawdown wins before more net |
| K009 | walkforward | A13=ceil | 104 | the same two passers, drawdown tenths ceiled (A13 other) |
| K010 | walkforward | headline | 104 | two misses: the smaller total shortfall wins (more net in all) |
| K011 | walkforward | A14=out_of_curve | 104 | the passer with the data charge out of the curve (A14 other) |
| K012 | gate | headline | 48 | the high-probability gate's boundaries (R4) on the three seeds |
| K013 | luck | headline | 8 | the luck rule clears with at most 49 of 1,000 at or above O |
| K014 | lots | headline | 8 | whole option lots at or under the premium, rounding down |
| V001 | verdict | headline | 5 | rule 1: no headline chain formed |
| V002 | verdict | headline | 5 | rule 2: one paisa below the bar in one month |
| V003 | verdict | headline | 5 | rule 2: a no-trade month |
| V004 | verdict | headline | 5 | rule 3: the sign test does not clear |
| V005 | verdict | headline | 3 | rule 4: twin seed 1 at or above the headline |
| V006 | verdict | headline | 3 | rule 4: a partial twin 1 at or above still says NO |
| V007 | verdict | headline | 3 | rule 4: a partial twin 1 below clears nothing |
| V008 | verdict | headline | 3 | rule 5: fewer than three twin seeds have run |
| V009 | verdict | headline | 4 | rule 5 before rule 6: twin 2 above, twin 3 missing reads PENDING |
| V010 | verdict | headline | 5 | rule 6: twin seed 2 at or above |
| V011 | verdict | headline | 5 | rule 7: YES |
| V012 | verdict | headline | 5 | an interim never prints YES (A19 headline) |
| V013 | verdict | A19=no_can_print | 5 | an interim's NO prints under A19's other reading |
| V014 | verdict | headline | 5 | an interim's NO stays PENDING under A19's headline |
| X001 | e2e | headline | 14 | a flag day end to end, long, K = 2: every E1C signal walked alone on lanes XA, XB, XC, XD2, XD3, XG (no account) |
| X002 | e2e | headline | 14 | one flag day end to end, short, K = 2 |
| X003 | e2e | headline | 7 | one flag day end to end, long, K = 3 |
| X004 | e2e | headline | 49 | one flag day end to end, short, K = 1 |
