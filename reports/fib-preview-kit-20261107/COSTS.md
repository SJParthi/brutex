# COSTS: NSE intraday equity (cash) charges, 3 Sep 2018 to 30 Oct 2026

Dated cost table for the F&O-stock intraday strategy. It covers every charge and every change, the two cost clocks (dated and today's Dhan rates), the worst-case readings, the differences from `crates/costs`, and 24 contract-note test vectors.

- Written 30 Oct 2026 (IST). Every official document below was downloaded today, 2026-10-30, with curl. PDFs were read with pdftotext, zips with bsdtar, and two image PDFs (FA50832, FA53913) visually. The sha256 of each document is in `costs-evidence/sha256-official.txt`, and the text extracts sit beside it.
- Machine-readable twin: `costs-table.json` (version `costs-table/2026-10-30/r1`). It has 25 rows (component, rate, base, side, from, to, source), 10 readings and the 24 vectors.
- The vector calculator is in `costs-evidence/calc/` (`vec.js`, `vectors.js`; node, BigInt paise, no floats). It was checked against the crate's own five golden vectors V1 to V5 (`crates/costs/src/stack.rs:1808-1861`), and all five match to the paisa (`node golden.js`: MATCH ×5). It was also checked by hand with `bc` on CN01 and CN14.
- This is a cost table, not a backtest result. Any backtest priced with it carries the label: **current F&O members only (survivorship bias not removed)**.
- Code read: `crates/costs` at commit `f01d7a0e4` of `feat/db-fix` (worktree `wt/DBFIX`). Nothing in the repository was changed.

## 1. The answer in brief

- **Today (Dhan, 30 Oct 2026)**, one plain round trip of about Rs 5 lakh each side (one buy order, one sell order, one fill each) costs **Rs 225.79** under the official method (CN01, today column). That is about 0.045% of the buy value. STT is Rs 126 of it, brokerage Rs 40, stamp Rs 15, the NSE charge Rs 30.85, GST Rs 12.93 and the SEBI fee Rs 1.01. The worst readings (DP on intraday, CGST/SGST rounding) raise it to **Rs 240.55**.
- **The dated clock is dearer than today on every date before 1 Oct 2024.** The NSE top slab was Rs 325 to 345 per crore, against Rs 307 today. The SEBI fee was Rs 15 per crore to 31 Mar 2019. Before 1 Jul 2020, stamp duty was a state levy on both sides. For a Rs 10 lakh day, the worst listed state (Telangana, 0.01% capped at Rs 100 a day) gives Rs 100 against Rs 15 today. The headline therefore comes from the dated clock: about **Rs 324 to 330** a trade before 1 Jul 2020 and **Rs 242 to 246** from 1 Jul 2020 to 30 Sep 2024. From 1 Oct 2024 the two clocks agree (**Rs 239.49**, CN19).
- **Filling in pieces is what costs most** under the worst reading, if brokerage is charged per fill. Each extra second in which an order fills adds up to Rs 20 + GST. In CN22, an entry over 8 seconds plus 3 exit fills puts brokerage at Rs 210.24 against Rs 60, and the headline at **Rs 466.09**. Until a Dhan contract note shows a part-filled order, the backtest must carry this reading (operator rule 7).
- STT has been 0.025% of the intraday sell side for the whole window (2013-06-01 onwards; unchanged in the 2023, 2024 and 2026 Finance Acts).

## 2. Timeline: every boundary inside 3 Sep 2018 to 30 Oct 2026

| Date (from) | What changed | Dated row | Source |
|---|---|---|---|
| (before window) 2009-10-01 | NSE cash slabs Rs 3.25 .. 3.00 per lakh | EX-2009 | NSE/F&A/13028 |
| (before window) 2017-03-06 | SEBI fee Rs 15/cr | SF-2017 | SEBI GN/2016-17/037 (NSE/FA/34501) |
| (before window) 2017-07-01 | GST 18% on brokerage and exchange charges | GST-1 | Notification 11/2017-CT(Rate) |
| 2018-09-03 | window starts | - | operator requirement |
| 2019-04-01 | SEBI fee Rs 15 -> Rs 10/cr | SF-2019 | SEBI GN/2019/03 (NSE/FA/40632) |
| 2020-06-01 | SEBI fee halved, Rs 5/cr, to 2021-03-31 | SF-2020 | SEBI GN/2020/011 (NSE/FA/44514) |
| 2020-07-01 | stamp duty: state levy on both sides -> 0.003% of buy value, buyer only, collected by NSE Clearing | SD-ND | Finance Act 2019; Stamp Rules 2019; NCL/CMPT/43115, 44032 |
| 2020-08-01 | concession Rs 2.75/lakh for EQ stocks outside NIFTY 50 / Next 50 (to 2023-03-31) | EX-2020C (informational) | NSE/FA/45147 .. 53913 |
| 2021-01-01 | NSE slabs raised to Rs 3.45 .. 3.20 per lakh | EX-2021 | NSE/FA/46730 |
| 2021-04-01 | SEBI fee back to Rs 10/cr | SF-2021 | relief window ends (NSE/FA/44514) |
| 2022-07-18 | GST 18% on the SEBI fee | GST-SEBI | NSE/FATAX/52993 |
| 2023-04-01 | NSE slabs back to Rs 3.25 .. 3.00; IPFT Rs 0.01 -> Rs 10/cr | EX-2023, IP-2023 | NSE/FA/56129 |
| 2024-04-01 | NSE slabs Rs 3.22 .. 2.97 | EX-2024A | NSE/FA/61137 |
| 2024-10-01 | NSE flat Rs 2.97/lakh for every member (true to label) | EX-2024B | NSE/FA/64232 |
| 2026-03-01 | NSE Rs 306.99 + IPFT Rs 0.01 = Rs 307/cr (total unchanged) | EX-2026, IP-2026 | NSE/FA/73061 |
| 2026-10-30 | today: Dhan page read, all figures match EX-2026 + IP-2026 + SF-2021 + STT-ND + SD-ND + GST | today's clock | dhan.co/pricing |

Not changed in the window: STT 0.025% on the intraday sell side, and GST at 18%. Dhan's own fees have no dated history.

## 3. The dated table (one row per component and period)

"Top slab" means the first slab, the highest rate. Before 1 Oct 2024 the NSE charge was set per member by its monthly turnover, and which slab a client was passed is not recorded, so the worst case uses the top slab.

| Id | Component | Rate | Base | Side | From | To | Source (document; URL; read 2026-10-30; quote) | Status |
|---|---|---|---|---|---|---|---|---|
| BRK-1 | Brokerage, Dhan, equity intraday | min(Rs 20, 0.03% of order value) per executed order | each executed order's value | buy and sell orders | UNVERIFIED (page undated; same on Dhan's archived page of 2023-02-01, sources-charges B-01; operator applies it to every date) | open | Dhan pricing, Equity tab; https://dhan.co/pricing/; "₹20 or 0.03% of trade value per executed order, whichever is lower." | PRIMARY (broker) |
| BRK-SQ | Dhan auto square-off fee | Rs 20 per order + GST | each order Dhan places to square off | the square-off order | UNVERIFIED | open | https://dhan.co/pricing/; "₹ 20 / order + GST. For intraday order automatically squared by our team". Time: https://dhan.co/support/general/market-session-status-and-timing/what-are-the-dhan-intraday-auto-square-off-timings/: "Equity cash intraday positions: 3:10 PM." | PRIMARY (broker) |
| BRK-DP | DP transaction charge | Rs 12.50 per instruction per ISIN + GST (Rs 14.75) | scrip-day with a delivery sell | sell | UNVERIFIED | open | https://dhan.co/pricing/; "₹ 12.50 / instruction / ISIN + GST." | PRIMARY; intraday applicability UNVERIFIED (reading RD-DP) |
| EX-2009 | NSE transaction charge, cash | slabs Rs 3.25/3.20/3.15/3.10/3.05/3.00 per lakh; **Rs 325/cr used** | traded value | each side | 2009-10-01 | 2020-12-31 | NSE/F&A/13028 (7 Sep 2009), https://nsearchives.nseindia.com/content/circulars/faac13028.pdf; "Up to First Rs. 1250 cores Rs. 3.25 each side" (typo "cores" is NSE's). Same ladder is FA46730's "Existing" column | PRIMARY |
| EX-2020C | NSE concession, EQ stocks outside NIFTY 50 / Next 50 / GSM | Rs 2.75 per lakh (Rs 1.75 above Rs 7,500 cr a month; from 2022-04-01 Rs 2.25 above Rs 12,500 cr) | traded value of eligible stocks | each side | 2020-08-01 | 2023-03-31 | NSE/FA/45147, 46225, 47844, 49796, 50832, 51855, 53913; e.g. https://nsearchives.nseindia.com/content/circulars/FA53913.pdf; "six months from October 1, 2022 to March 31, 2023" | PRIMARY; informational, lower than the regular rate, never the worst |
| EX-2021 | NSE transaction charge, cash | slabs Rs 3.45 .. 3.20 per lakh; **Rs 345/cr used** | traded value | each side | 2021-01-01 | 2023-03-31 | NSE/FA/46730 (18 Dec 2020), https://nsearchives.nseindia.com/content/circulars/FA46730.pdf; "will be applicable with effect from January 01, 2021." | PRIMARY |
| EX-2023 | NSE transaction charge, cash | slabs Rs 3.25 .. 3.00 per lakh; **Rs 325/cr used** | traded value | each side | 2023-04-01 | 2024-03-31 | NSE/FA/56129 (24 Mar 2023), https://nsearchives.nseindia.com/content/circulars/FA56129.pdf; "will be applicable with effect from April 01, 2023." | PRIMARY |
| EX-2024A | NSE transaction charge, cash | slabs Rs 3.22 .. 2.97 per lakh; **Rs 322/cr used** | traded value | each side | 2024-04-01 | 2024-09-30 | NSE/FA/61137 (14 Mar 2024), https://nsearchives.nseindia.com/content/circulars/FA61137.pdf; "will be applicable with effect from April 01, 2024." | PRIMARY |
| EX-2024B | NSE transaction charge, cash | flat Rs 2.97 per lakh (Rs 297/cr) | traded value | each side | 2024-10-01 | 2026-02-28 | NSE/FA/64232 (27 Sep 2024), https://nsearchives.nseindia.com/content/circulars/FA64232.pdf; "Cash Market: Rs. 2.97 each side per lakh of traded value" | PRIMARY |
| EX-2026 | NSE transaction charge, cash | Rs 306.99/cr | traded value | each side | 2026-03-01 | open | NSE/FA/73061 (27 Feb 2026), https://nsearchives.nseindia.com/content/circulars/FA73061.pdf; "The above revision will be applicable with effect from March 1, 2026." Dhan: "NSE: 0.0030699%" | PRIMARY |
| IP-OLD | Contribution to NSE IPFT, cash | Rs 0.01/cr | traded value | each side | UNVERIFIED start (no earlier circular found) | 2023-03-31 | NSE/FA/56129; "from existing Rs.0.01 per crore of the traded value to Rs.10"; FA73061 calls it the level "levied prior to April 1, 2023" | PRIMARY level, start UNVERIFIED; under one paisa a trade |
| IP-2023 | Contribution to NSE IPFT, cash | Rs 10/cr | traded value | each side | 2023-04-01 | 2026-02-28 | NSE/FA/56129; "Rs.10 per crore of the traded value in Cash Equities Market Segment" | PRIMARY |
| IP-2026 | Contribution to NSE IPFT, cash | Rs 0.01/cr | traded value | each side | 2026-03-01 | open | NSE/FA/73061; Dhan: "IPFT Contribution 0.0000001% of the turnover" | PRIMARY |
| SF-2017 | SEBI turnover fee, cash | 0.00015% (Rs 15/cr) | buy and sell value | each side | 2017-03-06 (in force on Gazette publication) | 2019-03-31 | SEBI/LAD/NRO/GN/2016-17/037 annexed to NSE/FA/34501, https://nsearchives.nseindia.com/content/circulars/FA34501.zip; "0.00015 per cent of the price at which the securities are purchased or sold" | PRIMARY |
| SF-2019 | SEBI turnover fee, cash | 0.00010% (Rs 10/cr) | turnover | each side | 2019-04-01 | 2020-05-31 | SEBI/L.A.D.-N.R.O./G.N./2019/03 annexed to NSE/FA/40632, https://nsearchives.nseindia.com/content/circulars/FA40632.pdf; "They shall come into force with effect from April 01, 2019." | PRIMARY |
| SF-2020 | SEBI turnover fee, relief | 50% of the fee (Rs 5/cr) | turnover | each side | 2020-06-01 | 2021-03-31 | SEBI/LAD-NRO/GN/2020/011 annexed to NSE/FA/44514, https://nsearchives.nseindia.com/content/circulars/FA44514.pdf; "pay only 50% (fifty percent) of fees as calculated therein" | PRIMARY (relief is to the broker; pass-through not recorded) |
| SF-2021 | SEBI turnover fee, cash | Rs 10/cr | turnover | each side | 2021-04-01 | open | relief lapsed; NSE levies page (updated 17/04/2026), https://www.nseindia.com/static/invest/first-time-investor-sebi-turnover-fees-stt-other-levies; "(Rs.10 per crore)"; Dhan: "0.0001% of the turnover" | PRIMARY |
| STT-ND | STT, equity sold otherwise than by delivery | 0.025% | NSE: squared qty x day VWAP of all the client's trades in the stock (VWAP to 2 decimals) | sell (seller) | 2013-06-01 | open | Finance Act 2013 per NSE/FATAX/23500; unchanged in FATAX/56235, 63809, 73524 (https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf, row 3: 0.025 per cent, "No Change", Seller); method NSE/CMTR/5588, https://nsearchives.nseindia.com/content/circulars/cmtr5588.htm | PRIMARY |
| SD-STATE | Stamp duty under State law, on the contract note | state rates (table below) | contract-note value, buy and sell | both | before window | 2020-06-30 | PIB, 30 Jun 2020, https://www.pib.gov.in/PressReleasePage.aspx?PRID=1635399; "stamp duty was payable by both seller and buyer". Rates: SECONDARY (Zerodha archived calculator script) | UNVERIFIED (client state unknown) |
| SD-ND | Stamp duty, non-delivery transfer of a security | 0.003% | FIFO buy value of the non-delivery quantity (squared day: all buys) | buy (buyer) | 2020-07-01 | open | NCL/CMPT/43115 (3 Jan 2020), https://nsearchives.nseindia.com/content/circulars/CMPT43115.zip; "Transfer of Security other than debenture on non-delivery basis 0.003% Buyer"; start per NCL/CMPT/44032 and Gazette of 30 Mar 2020: "will be applicable from trade date July 1, 2020" | PRIMARY |
| GST-1 | GST on brokerage, NSE charge, IPFT (and Dhan's fees) | 18% (9% CGST + 9% SGST, or 18% IGST) | brokerage + transaction charge + IPFT (+ square-off fee, DP when charged) | both | 2017-07-01 | open | Notification 11/2017-Central Tax (Rate), Sl. 15, heading 9971(v), 9% central tax, https://gstcouncil.gov.in/sites/default/files/2024-05/download_4.pdf; "shall come into force with effect from 1st day of July, 2017"; NSE levies page "Stock brokers Services 18.00% (GST)"; Dhan: "18% on brokerage + transaction charges + SEBI Turnover + IPFT/Other Tax" | PRIMARY (SGST/IGST notifications not read) |
| GST-SEBI | GST on the SEBI fee | 18% | SEBI fee | both | 2022-07-18 | open | NSE/FATAX/52993 (18 Jul 2022; Notification 04/2022-CT(Rate)), https://nsearchives.nseindia.com/content/circulars/FATAX52993.pdf; "levy GST @ 18% on the fees payable to SEBI, from their clients" | PRIMARY |
| OT-CLR | Clearing charge | none found | - | - | - | - | FA64232 and FA73061 levy only "transaction charges and contribution to NSE IPFT"; Dhan lists none | UNVERIFIED (absence is not proof) |
| OT-INT | Interest on intraday leverage | none on a squared-off day (inference) | - | - | - | - | Dhan lists interest only "on outstanding non MTF debit" and 50:50 shortfall | UNVERIFIED |
| BRK-CT | Call & trade | Rs 50/order + GST | trade-desk orders | - | - | - | Dhan page | not used (orders by API) |

**Pre-2020 state stamp duty, equity intraday column (SECONDARY).** Read today from Zerodha's archived calculator script `http://web.archive.org/web/20190403072919js_/https://zerodha.com/static/app.js?v=7.0.7` (the archive served the 2019-04-03 copy), with state codes from the 2018-07-14 snapshot of `zerodha.com/brokerage-calculator`. Extracts are in `costs-evidence/zerodha-archived-state-*.txt`. Zerodha's own page says the charge is levied "as per the state of the client's correspondence address".

| State | Intraday rate on the contract note | Cap |
|---|---|---|
| Andhra Pradesh, Odisha | 0.005% | Rs 50 |
| Arunachal Pradesh, Karnataka, Meghalaya, others, Rajasthan, Jammu and Kashmir | 0.003% | - |
| Assam | 0.018% | Rs 49.50 |
| Delhi, Gujarat, Kerala, Maharashtra, Madhya Pradesh, West Bengal | 0.002% | - |
| Goa | 0.005% | - |
| Haryana | 0.002% | Rs 200 |
| Himachal Pradesh | Rs 50 a day | - |
| Tamil Nadu | 0.006% | - |
| Uttar Pradesh | 0.002% | Rs 1,000 a day |
| Telangana | 0.01% | Rs 100 a day |

NSE's own page adds one official fact. Maharashtra waived stamp duty on records of transaction for investors residing outside Maharashtra (Order Mudrank 2006/CR/290/M-1, 23 May 2006; https://www.nseindia.com/static/trade/membership-stamp-duty). **The client's state for 2018-2020 is the open question.** Until the operator names it, the dated clock uses the worst listed state for each day's turnover: Telangana up to Rs 16.67 lakh of turnover a day, Tamil Nadu above it.

## 4. Today's clock (Dhan, read 30 Oct 2026)

From https://dhan.co/pricing/, Equity (Intraday) column, read 2026-10-30 (extract: `costs-evidence/dhan-pricing-equity-20261030.txt`):

| Line | Dhan's words |
|---|---|
| Brokerage | "₹20 or 0.03% of trade value per executed order, whichever is lower." |
| Transaction charges | "NSE: 0.0030699%" |
| IPFT | "0.0000001% of the turnover" |
| SEBI | "0.0001% of the turnover" |
| STT | "0.025% on sell transactions" |
| Stamp | "0.003% on turnover of buy orders" |
| GST | "18% on brokerage + transaction charges + SEBI Turnover + IPFT/Other Tax" |
| Rounding | "Stamp Duty and STT are rounded off to the nearest rupee and all other charges to the nearest 2 decimals." |
| Auto square-off | "₹ 20 / order + GST", at 3:10 PM for equity cash intraday (Dhan support page) |
| DP | "₹ 12.50 / instruction / ISIN + GST." |

These are the same figures `crates/costs` recorded as of 2026-10-16 (`schedule.rs:244`, version `todays-rates/2026-10-16/r1`). No rate changed between the two reads.

## 5. How one contract note is computed: the official method

For one client, one stock and one day, with BQTY/BVAL the day's bought quantity and value and SQTY/SVAL the sold:

1. **Brokerage**: per executed order, min(Rs 20, 0.03% x order value), half-up to the paisa.
2. **NSE charge + IPFT**: (BVAL + SVAL) x rate, half-up to the paisa, as one line.
3. **SEBI fee**: (BVAL + SVAL) x rate, half-up to the paisa.
4. **STT**: VWAP = (BVAL + SVAL) / (BQTY + SQTY) to 2 decimals. Then STT = 0.025% x min(BQTY, SQTY) x VWAP, to 2 decimals. The client's total for the day across stocks is rounded to the nearest rupee, 50 paise up (NSE/CMTR/5588 sections A and D: "rounded off to the nearest rupee").
5. **Stamp** (from 2020-07-01): 0.003% x FIFO buy value of the non-delivery quantity, to 2 decimals per security. The client's day total goes to the nearest rupee (NCL/CMPT/43115 Annexure 1 items 1.2 and 2; Stamp Rules 2019 rule 7(3)).
6. **GST**: 18% x (brokerage + NSE charge incl. IPFT + SEBI fee [only from 2022-07-18 on the dated clock]), to 2 decimals. Plus 18% of any square-off fee or DP charge.
7. **Square-off fee**: Rs 20 per order Dhan placed, plus its GST.

This is exactly what `crates/costs` "precise" mode does for today's rates (`stack.rs:570-690`). The calculator here reproduces the crate's five golden vectors to the paisa.

## 6. Worst-case readings (operator rule 7): every choice Dhan's words leave open

Each reading is computed in the vectors, and the larger figure is used.

| Id | Open choice | Readings computed | Worse used | What settles it |
|---|---|---|---|---|
| RD-BRK | an order that fills in several seconds | per executed order; **per fill** | per fill (up to Rs 20 + GST per extra fill second) | a Dhan contract note for a part-filled order |
| RD-STT-BASE | STT base | NSE VWAP basis; **sell value** (Dhan's page wording and calculator); sell value per sell order | the largest; sell value is larger whenever SVAL > BVAL | a Dhan contract note with different buy and sell prices |
| RD-ROUND-TAX | level of STT and stamp rupee rounding | client-day total (NSE, NCL, Rules 7(3)); per security-day; **per order** | the largest | a contract note with two stocks or two sell orders on one day |
| RD-ROUND-EX | exchange and SEBI rounding | once per day; transaction charge and IPFT as separate lines; **per order** | the largest (at most a paisa or two) | a Dhan contract note |
| RD-GST | GST rounding | 18% once; **9% CGST + 9% SGST each rounded** | the larger (at most one paisa) | a Dhan contract note |
| RD-DP | DP on intraday | none; **Rs 12.50 + GST per scrip-day with a sell** | Rs 14.75 | a Dhan contract note or ledger for an intraday-only day |
| RD-SQ | square-off fee and brokerage | **on top of brokerage**; instead of it | on top | a Dhan contract note with an auto square-off |
| RD-STATE | stamp before 2020-07-01 | every listed state | the highest for that day's turnover | the operator names the client's state |
| RD-SLAB | NSE slab before 2024-10-01 | every slab; the concession | **top slab** | not needed under the worst-case rule |
| RD-TIE | Rs 5 lakh / price = n + 0.5 exactly | n; n + 1 | n + 1 for charges (the spec must pin one) | operator decision |

**Headline per trade** = max(dated clock worst total, today's clock worst total). Both totals are shown on every vector.

## 7. Differences from `crates/costs` (feat/db-fix at f01d7a0e4)

| # | Where | What the crate does | What the table needs | Kind |
|---|---|---|---|---|
| 1 | `schedule.rs:1011-1026` (`dated_card`), message at `:1000` | `RateSelector::Dated` refuses equity on **every** date ("records no dated history") | the dated clock for cash intraday: rows EX-*, IP-*, SF-*, SD-*, GST-SEBI from 2018-09-03 | missing component (whole dated clock) |
| 2 | `regime.rs:90` `MAX_LATER_ROWS = 2`; resolution Rs 1/cr (`regime.rs:435-437`) | a dated table holds an anchor and 2 later rows, at one rupee per crore | NSE cash needs 6 rows, IPFT 3, SEBI 4, and Rs 306.99 and Rs 0.01 per crore are not representable. The dated equity table needs more rows and the exact `Ratio` of `schedule.rs`, not `BpsX100` | structure |
| 3 | `schedule.rs:262` `NSE_EQUITY_EXCHANGE` Rs 307/cr | right for today and for 2024-10-01 onwards | dated Rs 325.01 (to 2020-12-31), 345.01 (2021-01-01 to 2023-03-31), 335 (2023-04-01 to 2024-03-31), 332 (2024-04-01 to 2024-09-30) | wrong rate on earlier dates (only as a dated clock) |
| 4 | `schedule.rs:311` `SEBI_FEE`, `rate.rs:151` `SEBI_TURNOVER_FEE` Rs 10/cr | one rate for all dates | Rs 15 to 2019-03-31; Rs 5 from 2020-06-01 to 2021-03-31 | missing dated rows |
| 5 | `schedule.rs:321` `GST`, `stack.rs:599` GST base = brokerage + exchange + SEBI on every date (also stated as a limit at `docs/06-limits.md:1801`) | GST on the SEBI fee always | dated: SEBI fee outside the GST base before 2022-07-18 (today's clock keeps it) | wrong base on earlier dates (dated clock) |
| 6 | `schedule.rs:387` `STAMP_EQUITY_NON_DELIVERY` 0.003% buy | the unified rate only | state stamp on both sides before 2020-07-01 (SD-STATE), client state needed | missing component |
| 7 | `stack.rs:818-822`, `:896-901` (VWAP basis); `dhan_calc.rs:19` (sell basis, labelled estimate) | the precise mode uses NSE's VWAP basis only; the calculator replica uses the sell value but never decides | worst of the VWAP basis, the sell value and the per-order sell value (RD-STT-BASE) | missing reading |
| 8 | `ledger.rs:84-130` (`ExecutedOrder::with_part` joins fills, "UNVERIFIED for Dhan", D-5127); `schedule.rs:483` | brokerage once per order | the per-fill reading as well, and the worse used (RD-BRK) | missing reading |
| 9 | `stack.rs:593-597` | DP only on a delivery or MTF sell day | worst reading Rs 12.50 + GST on an intraday scrip-day with a sell (RD-DP) | missing reading |
| 10 | `stack.rs:578-579`, `:599-602`, `:903-908` | exchange, SEBI and GST half-up once; STT and stamp half-up to the rupee once per client-day (`ClientDay`) or per security-day priced alone | the per-order and split-line readings and CGST/SGST rounding, worst used (RD-ROUND-*, RD-GST) | missing readings (paise; rupees for STT and stamp) |
| 11 | `stack.rs:1583-1610` (`NetTrade::of_days` refuses two rate sets) and `schedule.rs:585` `RateSelector` | one rate set per trade | the headline is max(dated worst, today worst) per trade, with both shown. That needs a combinator that prices a trade twice and keeps both | missing feature |
| 12 | `dhan_calc.rs:19` | GST base without the SEBI fee | Dhan's page (read today) puts the SEBI fee in the GST base | the replica differs from Dhan's stated rule; it is labelled an estimate and never decides, so this is informational |
| 13 | `schedule.rs:457` `DHAN_SQUARE_OFF_FEE` Rs 20 + GST on top | matches the worst reading RD-SQ | - | agrees |
| 14 | `schedule.rs:335` STT 0.025%, `schedule.rs:430-439` brokerage 0.03% / Rs 20, `schedule.rs:387` stamp 0.003% | today's figures | - | agree with Dhan's page of 2026-10-30 and with every circular |
| 15 | `schedule.rs:599` `TODAYS_RATES_VERSION = "todays-rates/2026-10-16/r1"` | the run identity names today's set only | the run identity should name this table's version (`costs-table/2026-10-30/r1`) for the dated clock and the readings used | missing identity term |
| 16 | `money.rs` (`levy_ceiling`, `statutory_levy`: ceilings) vs `stack.rs` (half-up) | the old options path ceils every levy and the contract-note path rounds half-up | the equity path follows `stack.rs`, and the worst readings are explicit readings, not a ceiling | note (two rounding laws in one crate) |
| 17 | (none) | the concession EX-2020C is not modelled | not needed for the worst case (lower), but a dated "precise" figure for eligible stocks in 2020-08 to 2023-03 would be lower | informational |

Side of each charge: the crate's assignments are all correct. STT is on the sell side, stamp on the buy side, and the NSE charge and SEBI fee on both sides; a short trade's later buy pays the stamp (golden V5). Squared-off detection is by quantities, not product (`stack.rs:875-893`), which is what NSE does.

## 8. What is UNVERIFIED, and what would settle it

1. **The client's state of residence for 2018-09-03 to 2020-06-30** (stamp duty). Until it is known, the worst listed state is used. That adds about Rs 85 a trade to the dated clock in that window. The state rates are from a broker's archived page (secondary). The official text is each State's stamp schedule.
2. **Brokerage on a part-filled order** (per order or per fill). Settled by a Dhan contract note. This is the largest single swing: up to Rs 23.60 per extra fill second.
3. **DP on intraday** (Rs 14.75 per scrip-day). Settled by a Dhan contract note or ledger.
4. **STT basis on Dhan's contract notes** (NSE VWAP or sell value). The crate's D-5111 says the operator's five vectors fit VWAP only, but it does not say they were Dhan contract notes.
5. **Rounding level** (per order, per security or per client-day) for STT and stamp, and how the exact half is rounded (half-up assumed, as in the crate).
6. **Start of the Rs 0.01/cr IPFT** before 2023 (below one paisa a trade).
7. **When Dhan's present brokerage took effect.** Dhan's page is undated. The operator applies it to every date.
8. **Whether brokers passed the 2020-21 SEBI 50% relief to clients.** It is not material: today's clock wins in that window.
9. **No clearing charge, no CAS-specific charge.** A subject search of NSE's circular list, 1 Jan 2008 to 30 Oct 2026 (73,089 entries), found no per-trade clearing charge to clients. The only clearing-fee circular is SEBI's annual fee on clearing members (NCL/CMPL/73443). The closing-auction circulars (NSE/CMTR/76170, 76963; NCL/CMPT/76951) are operational by subject, and their texts were not read for charges. Absence is not proof.
10. **SGST and IGST notifications** were not read. Notification 11/2017-CT(Rate) gives the 9% central half.
11. Not reachable as primary today: sebi.gov.in and incometaxindia.gov.in. Both are reported unreachable in `sources-charges.md`; not retried. SEBI and Finance Act texts were read through NSE's copies (annexures and circulars).

## 9. Test vectors (24 contract notes)

The vectors use these conventions:

- One stock, one day. The quantity is the integer closest to Rs 5,00,000 / the reference price (the first entry fill).
- Long: buy then sell. Short: sell then buy.
- Fills are GDFL-second worst prices (a buy at the high, a sell at the low).
- **Dated, precise**: the official method on the date's rows, with the worst listed state for pre-2020 stamp and the top NSE slab.
- **Dated, worst reading**: every RD-* reading at its worse value.
- **Today**: Dhan's rates of 30 Oct 2026 applied to the same fills.
- Amounts are in rupees. Gross is "signal only, before charges" (still worst-price fills). Net = gross minus the headline.
- Machine-readable copies: `costs-table.json` key `test_vectors`, and `costs-evidence/calc/vectors.json`.

Coverage:

- Both sides of every boundary in section 2: CN02/CN03 SEBI 15->10, CN04/CN05 SEBI relief, CN06/CN07 stamp regime, CN08/CN09 NSE 325->345, CN10/CN11 SEBI 5->10, CN12/CN13 GST on SEBI, CN14/CN15 NSE 345->325+10, CN16/CN17 325->322, CN18/CN19 slabs->flat, CN20/CN21 297+10->306.99+0.01.
- Long and short, winners and losers, prices from 77.85 to 9,875.
- A VWAP that lands on an exact half paisa (CN14/CN15: 77.075 -> 77.08).
- Multi-second fills (CN20 to CN24), an auto square-off (CN22, CN23) and fills under Rs 66,666.67 (CN24).
- The Rs 5 lakh quantity tie, both sides (CN22: 313; CN23: 312).

### CN01 — 2018-09-03 — window start; SEBI Rs 15/cr; NSE Rs 325/cr top slab; state stamp duty

Quantity rule: 5,00,000 / 1250.00 = 400.0000; integers 400 (Rs 5,00,000.00, off by 0.00) and 401 (Rs 5,01,250.00, off by 1250.00); closest = 400. Fills: order 1 BUY: 400 @ 1,250.00; order 2 SELL: 400 @ 1,262.50.
Buy value 5,00,000.00; sell value 5,05,000.00; turnover 10,05,000.00; quantity 400; NSE VWAP (B+S)/(2Q) = 1,256.25; gross P&L (signal only, before charges) **5,000.00**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 15/cr (SEBI GN/2016-17/037); GST on SEBI fee: no; stamp: state rate on turnover 10,05,000.00 (both sides; secondary table): Maharashtra 0.002% = 20.10; Tamil Nadu 0.006% = 60.30; Telangana 0.01% max Rs 100/day = 100.00; worst listed state = TS 100.00 (used).
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.63 -> 126.00, sell-value basis 126.00, per sell order 126.00; exchange line once 32.66 / split 32.66 / per order 32.66 (dated); today's exchange once 30.85 / per order 30.85.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.66 | 32.66 | 30.85 | 30.85 |
| SEBI turnover fee | 1.51 | 1.51 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 126.00 | 126.00 | 126.00 |
| Stamp duty | 100.00 | 100.00 | 15.00 | 15.00 |
| GST 18% | 13.08 | 13.08 | 12.93 | 12.94 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 313.25 | 328.00 | 225.79 | 240.55 |

Headline charge (worse of the two worst-reading totals): **Rs 328.00** (dated clock). Net = 5,000.00 - 328.00 = **Rs 4,672.00**.

### CN02 — 2019-03-29 — last trading day before SEBI 15 -> 10 (short trade)

Quantity rule: 5,00,000 / 812.35 = 615.4982; integers 615 (Rs 4,99,595.25, off by 404.75) and 616 (Rs 5,00,407.60, off by 407.60); closest = 615. Fills: order 1 SELL: 615 @ 812.35; order 2 BUY: 615 @ 805.00.
Buy value 4,95,075.00; sell value 4,99,595.25; turnover 9,94,670.25; quantity 615; NSE VWAP (B+S)/(2Q) = 808.68; gross P&L (signal only, before charges) **4,520.25**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 15/cr (SEBI GN/2016-17/037); GST on SEBI fee: no; stamp: state rate on turnover 9,94,670.25 (both sides; secondary table): Maharashtra 0.002% = 19.89; Tamil Nadu 0.006% = 59.68; Telangana 0.01% max Rs 100/day = 99.47; worst listed state = TS 99.47 (used).
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 124.33 -> 124.00, sell-value basis 125.00, per sell order 125.00; exchange line once 32.33 / split 32.33 / per order 32.33 (dated); today's exchange once 30.54 / per order 30.54.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.33 | 32.33 | 30.54 | 30.54 |
| SEBI turnover fee | 1.49 | 1.49 | 0.99 | 1.00 |
| STT (sell side) | 124.00 | 125.00 | 124.00 | 125.00 |
| Stamp duty | 99.47 | 99.47 | 15.00 | 15.00 |
| GST 18% | 13.02 | 13.02 | 12.88 | 12.88 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 310.31 | 326.06 | 223.41 | 239.17 |

Headline charge (worse of the two worst-reading totals): **Rs 326.06** (dated clock). Net = 4,520.25 - 326.06 = **Rs 4,194.19**.

### CN03 — 2019-04-01 — first day SEBI Rs 10/cr (losing long)

Quantity rule: 5,00,000 / 2345.65 = 213.1605; integers 213 (Rs 4,99,623.45, off by 376.55) and 214 (Rs 5,01,969.10, off by 1969.10); closest = 213. Fills: order 1 BUY: 213 @ 2,345.65; order 2 SELL: 213 @ 2,330.10.
Buy value 4,99,623.45; sell value 4,96,311.30; turnover 9,95,934.75; quantity 213; NSE VWAP (B+S)/(2Q) = 2,337.88; gross P&L (signal only, before charges) **-3,312.15**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 10/cr (SEBI GN/2019/03); GST on SEBI fee: no; stamp: state rate on turnover 9,95,934.75 (both sides; secondary table): Maharashtra 0.002% = 19.92; Tamil Nadu 0.006% = 59.76; Telangana 0.01% max Rs 100/day = 99.59; worst listed state = TS 99.59 (used).
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 124.49 -> 124.00, sell-value basis 124.00, per sell order 124.00; exchange line once 32.37 / split 32.37 / per order 32.37 (dated); today's exchange once 30.58 / per order 30.58.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.37 | 32.37 | 30.58 | 30.58 |
| SEBI turnover fee | 1.00 | 1.00 | 1.00 | 1.00 |
| STT (sell side) | 124.00 | 124.00 | 124.00 | 124.00 |
| Stamp duty | 99.59 | 100.00 | 15.00 | 15.00 |
| GST 18% | 13.03 | 13.03 | 12.88 | 12.88 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 309.99 | 325.15 | 223.46 | 238.21 |

Headline charge (worse of the two worst-reading totals): **Rs 325.15** (dated clock). Net = -3,312.15 - 325.15 = **Rs -3,637.30**.

### CN04 — 2020-05-29 — last day before SEBI 50% relief

Quantity rule: 5,00,000 / 156.40 = 3196.9309; integers 3196 (Rs 4,99,854.40, off by 145.60) and 3197 (Rs 5,00,010.80, off by 10.80); closest = 3197. Fills: order 1 BUY: 3197 @ 156.40; order 2 SELL: 3197 @ 159.85.
Buy value 5,00,010.80; sell value 5,11,040.45; turnover 10,11,051.25; quantity 3197; NSE VWAP (B+S)/(2Q) = 158.13; gross P&L (signal only, before charges) **11,029.65**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 10/cr (SEBI GN/2019/03); GST on SEBI fee: no; stamp: state rate on turnover 10,11,051.25 (both sides; secondary table): Maharashtra 0.002% = 20.22; Tamil Nadu 0.006% = 60.66; Telangana 0.01% max Rs 100/day = 100.00; worst listed state = TS 100.00 (used).
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 126.39 -> 126.00, sell-value basis 128.00, per sell order 128.00; exchange line once 32.86 / split 32.86 / per order 32.86 (dated); today's exchange once 31.04 / per order 31.04.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.86 | 32.86 | 31.04 | 31.04 |
| SEBI turnover fee | 1.01 | 1.01 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 128.00 | 126.00 | 128.00 |
| Stamp duty | 100.00 | 100.00 | 15.00 | 15.00 |
| GST 18% | 13.11 | 13.12 | 12.97 | 12.97 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 312.98 | 329.74 | 226.02 | 242.77 |

Headline charge (worse of the two worst-reading totals): **Rs 329.74** (dated clock). Net = 11,029.65 - 329.74 = **Rs 10,699.91**.

### CN05 — 2020-06-01 — first day SEBI Rs 5/cr (50%), short trade

Quantity rule: 5,00,000 / 4020.00 = 124.3781; integers 124 (Rs 4,98,480.00, off by 1520.00) and 125 (Rs 5,02,500.00, off by 2500.00); closest = 124. Fills: order 1 SELL: 124 @ 4,020.00; order 2 BUY: 124 @ 3,990.55.
Buy value 4,94,828.20; sell value 4,98,480.00; turnover 9,93,308.20; quantity 124; NSE VWAP (B+S)/(2Q) = 4,005.28; gross P&L (signal only, before charges) **3,651.80**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 5/cr = 50% (SEBI GN/2020/011); GST on SEBI fee: no; stamp: state rate on turnover 9,93,308.20 (both sides; secondary table): Maharashtra 0.002% = 19.87; Tamil Nadu 0.006% = 59.60; Telangana 0.01% max Rs 100/day = 99.33; worst listed state = TS 99.33 (used).
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 124.16 -> 124.00, sell-value basis 125.00, per sell order 125.00; exchange line once 32.28 / split 32.28 / per order 32.28 (dated); today's exchange once 30.49 / per order 30.49.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.28 | 32.28 | 30.49 | 30.49 |
| SEBI turnover fee | 0.50 | 0.50 | 0.99 | 0.99 |
| STT (sell side) | 124.00 | 125.00 | 124.00 | 125.00 |
| Stamp duty | 99.33 | 99.33 | 15.00 | 15.00 |
| GST 18% | 13.01 | 13.02 | 12.87 | 12.87 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 309.12 | 324.88 | 223.35 | 239.10 |

Headline charge (worse of the two worst-reading totals): **Rs 324.88** (dated clock). Net = 3,651.80 - 324.88 = **Rs 3,326.92**.

### CN06 — 2020-06-30 — last day of state stamp duty (losing long)

Quantity rule: 5,00,000 / 600.00 = 833.3333; integers 833 (Rs 4,99,800.00, off by 200.00) and 834 (Rs 5,00,400.00, off by 400.00); closest = 833. Fills: order 1 BUY: 833 @ 600.00; order 2 SELL: 833 @ 594.05.
Buy value 4,99,800.00; sell value 4,94,843.65; turnover 9,94,643.65; quantity 833; NSE VWAP (B+S)/(2Q) = 597.03; gross P&L (signal only, before charges) **-4,956.35**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 5/cr = 50% (SEBI GN/2020/011); GST on SEBI fee: no; stamp: state rate on turnover 9,94,643.65 (both sides; secondary table): Maharashtra 0.002% = 19.89; Tamil Nadu 0.006% = 59.68; Telangana 0.01% max Rs 100/day = 99.46; worst listed state = TS 99.46 (used).
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 124.33 -> 124.00, sell-value basis 124.00, per sell order 124.00; exchange line once 32.33 / split 32.33 / per order 32.32 (dated); today's exchange once 30.54 / per order 30.53.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.33 | 32.33 | 30.54 | 30.54 |
| SEBI turnover fee | 0.50 | 0.50 | 0.99 | 0.99 |
| STT (sell side) | 124.00 | 124.00 | 124.00 | 124.00 |
| Stamp duty | 99.46 | 99.46 | 15.00 | 15.00 |
| GST 18% | 13.02 | 13.02 | 12.88 | 12.88 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 309.31 | 324.06 | 223.41 | 238.16 |

Headline charge (worse of the two worst-reading totals): **Rs 324.06** (dated clock). Net = -4,956.35 - 324.06 = **Rs -5,280.41**.

### CN07 — 2020-07-01 — first day unified stamp 0.003% buy

Quantity rule: 5,00,000 / 600.00 = 833.3333; integers 833 (Rs 4,99,800.00, off by 200.00) and 834 (Rs 5,00,400.00, off by 400.00); closest = 833. Fills: order 1 BUY: 833 @ 600.00; order 2 SELL: 833 @ 606.00.
Buy value 4,99,800.00; sell value 5,04,798.00; turnover 10,04,598.00; quantity 833; NSE VWAP (B+S)/(2Q) = 603.00; gross P&L (signal only, before charges) **4,998.00**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 5/cr = 50% (SEBI GN/2020/011); GST on SEBI fee: no; stamp: 0.003% x buy 4,99,800.00 = 14.99 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.57 -> 126.00, sell-value basis 126.00, per sell order 126.00; exchange line once 32.65 / split 32.65 / per order 32.65 (dated); today's exchange once 30.84 / per order 30.84.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.65 | 32.65 | 30.84 | 30.84 |
| SEBI turnover fee | 0.50 | 0.50 | 1.00 | 1.00 |
| STT (sell side) | 126.00 | 126.00 | 126.00 | 126.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.08 | 13.08 | 12.93 | 12.94 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 227.23 | 241.98 | 225.77 | 240.53 |

Headline charge (worse of the two worst-reading totals): **Rs 241.98** (dated clock). Net = 4,998.00 - 241.98 = **Rs 4,756.02**.

### CN08 — 2020-12-31 — last day of NSE Rs 325/cr top slab

Quantity rule: 5,00,000 / 1000.00 = 500.0000; integers 500 (Rs 5,00,000.00, off by 0.00) and 501 (Rs 5,01,000.00, off by 1000.00); closest = 500. Fills: order 1 BUY: 500 @ 1,000.00; order 2 SELL: 500 @ 1,015.00.
Buy value 5,00,000.00; sell value 5,07,500.00; turnover 10,07,500.00; quantity 500; NSE VWAP (B+S)/(2Q) = 1,007.50; gross P&L (signal only, before charges) **7,500.00**.
Dated rows: E-2009 top slab Rs 325/cr (faac13028); IPFT Rs 0.01/cr (start UNVERIFIED); SEBI Rs 5/cr = 50% (SEBI GN/2020/011); GST on SEBI fee: no; stamp: 0.003% x buy 5,00,000.00 = 15.00 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.94 -> 126.00, sell-value basis 127.00, per sell order 127.00; exchange line once 32.74 / split 32.74 / per order 32.74 (dated); today's exchange once 30.93 / per order 30.93.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 32.74 | 32.74 | 30.93 | 30.93 |
| SEBI turnover fee | 0.50 | 0.50 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 127.00 | 126.00 | 127.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.09 | 13.10 | 12.95 | 12.95 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 227.33 | 243.09 | 225.89 | 241.64 |

Headline charge (worse of the two worst-reading totals): **Rs 243.09** (dated clock). Net = 7,500.00 - 243.09 = **Rs 7,256.91**.

### CN09 — 2021-01-01 — first day NSE Rs 345/cr top slab (losing short)

Quantity rule: 5,00,000 / 1000.00 = 500.0000; integers 500 (Rs 5,00,000.00, off by 0.00) and 501 (Rs 5,01,000.00, off by 1000.00); closest = 500. Fills: order 1 SELL: 500 @ 1,000.00; order 2 BUY: 500 @ 1,012.00.
Buy value 5,06,000.00; sell value 5,00,000.00; turnover 10,06,000.00; quantity 500; NSE VWAP (B+S)/(2Q) = 1,006.00; gross P&L (signal only, before charges) **-6,000.00**.
Dated rows: E-2021 top slab Rs 345/cr (FA46730); IPFT Rs 0.01/cr; SEBI Rs 5/cr = 50% (SEBI GN/2020/011); GST on SEBI fee: no; stamp: 0.003% x buy 5,06,000.00 = 15.18 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.75 -> 126.00, sell-value basis 125.00, per sell order 125.00; exchange line once 34.71 / split 34.71 / per order 34.71 (dated); today's exchange once 30.88 / per order 30.88.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 34.71 | 34.71 | 30.88 | 30.88 |
| SEBI turnover fee | 0.50 | 0.50 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 126.00 | 126.00 | 126.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.45 | 13.45 | 12.94 | 12.94 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 229.66 | 244.41 | 225.83 | 240.58 |

Headline charge (worse of the two worst-reading totals): **Rs 244.41** (dated clock). Net = -6,000.00 - 244.41 = **Rs -6,244.41**.

### CN10 — 2021-03-31 — last day SEBI Rs 5/cr

Quantity rule: 5,00,000 / 499.95 = 1000.1000; integers 1000 (Rs 4,99,950.00, off by 50.00) and 1001 (Rs 5,00,449.95, off by 449.95); closest = 1000. Fills: order 1 BUY: 1000 @ 499.95; order 2 SELL: 1000 @ 505.00.
Buy value 4,99,950.00; sell value 5,05,000.00; turnover 10,04,950.00; quantity 1000; NSE VWAP (B+S)/(2Q) = 502.48; gross P&L (signal only, before charges) **5,050.00**.
Dated rows: E-2021 top slab Rs 345/cr (FA46730); IPFT Rs 0.01/cr; SEBI Rs 5/cr = 50% (SEBI GN/2020/011); GST on SEBI fee: no; stamp: 0.003% x buy 4,99,950.00 = 15.00 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.62 -> 126.00, sell-value basis 126.00, per sell order 126.00; exchange line once 34.67 / split 34.67 / per order 34.67 (dated); today's exchange once 30.85 / per order 30.85.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 34.67 | 34.67 | 30.85 | 30.85 |
| SEBI turnover fee | 0.50 | 0.50 | 1.00 | 1.01 |
| STT (sell side) | 126.00 | 126.00 | 126.00 | 126.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.44 | 13.44 | 12.93 | 12.94 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 229.61 | 244.36 | 225.78 | 240.55 |

Headline charge (worse of the two worst-reading totals): **Rs 244.36** (dated clock). Net = 5,050.00 - 244.36 = **Rs 4,805.64**.

### CN11 — 2021-04-01 — first day SEBI back to Rs 10/cr (same trade as CN10)

Quantity rule: 5,00,000 / 499.95 = 1000.1000; integers 1000 (Rs 4,99,950.00, off by 50.00) and 1001 (Rs 5,00,449.95, off by 449.95); closest = 1000. Fills: order 1 BUY: 1000 @ 499.95; order 2 SELL: 1000 @ 505.00.
Buy value 4,99,950.00; sell value 5,05,000.00; turnover 10,04,950.00; quantity 1000; NSE VWAP (B+S)/(2Q) = 502.48; gross P&L (signal only, before charges) **5,050.00**.
Dated rows: E-2021 top slab Rs 345/cr (FA46730); IPFT Rs 0.01/cr; SEBI Rs 10/cr; GST on SEBI fee: no; stamp: 0.003% x buy 4,99,950.00 = 15.00 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.62 -> 126.00, sell-value basis 126.00, per sell order 126.00; exchange line once 34.67 / split 34.67 / per order 34.67 (dated); today's exchange once 30.85 / per order 30.85.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 34.67 | 34.67 | 30.85 | 30.85 |
| SEBI turnover fee | 1.00 | 1.01 | 1.00 | 1.01 |
| STT (sell side) | 126.00 | 126.00 | 126.00 | 126.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.44 | 13.44 | 12.93 | 12.94 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 230.11 | 244.87 | 225.78 | 240.55 |

Headline charge (worse of the two worst-reading totals): **Rs 244.87** (dated clock). Net = 5,050.00 - 244.87 = **Rs 4,805.13**.

### CN12 — 2022-07-15 — last trading day without GST on the SEBI fee

Quantity rule: 5,00,000 / 2500.00 = 200.0000; integers 200 (Rs 5,00,000.00, off by 0.00) and 201 (Rs 5,02,500.00, off by 2500.00); closest = 200. Fills: order 1 BUY: 200 @ 2,500.00; order 2 SELL: 200 @ 2,537.50.
Buy value 5,00,000.00; sell value 5,07,500.00; turnover 10,07,500.00; quantity 200; NSE VWAP (B+S)/(2Q) = 2,518.75; gross P&L (signal only, before charges) **7,500.00**.
Dated rows: E-2021 top slab Rs 345/cr (FA46730); IPFT Rs 0.01/cr; SEBI Rs 10/cr; GST on SEBI fee: no; stamp: 0.003% x buy 5,00,000.00 = 15.00 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.94 -> 126.00, sell-value basis 127.00, per sell order 127.00; exchange line once 34.76 / split 34.76 / per order 34.76 (dated); today's exchange once 30.93 / per order 30.93.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 34.76 | 34.76 | 30.93 | 30.93 |
| SEBI turnover fee | 1.01 | 1.01 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 127.00 | 126.00 | 127.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.46 | 13.46 | 12.95 | 12.95 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 230.23 | 245.98 | 225.89 | 241.64 |

Headline charge (worse of the two worst-reading totals): **Rs 245.98** (dated clock). Net = 7,500.00 - 245.98 = **Rs 7,254.02**.

### CN13 — 2022-07-18 — first day GST 18% on the SEBI fee (same trade as CN12)

Quantity rule: 5,00,000 / 2500.00 = 200.0000; integers 200 (Rs 5,00,000.00, off by 0.00) and 201 (Rs 5,02,500.00, off by 2500.00); closest = 200. Fills: order 1 BUY: 200 @ 2,500.00; order 2 SELL: 200 @ 2,537.50.
Buy value 5,00,000.00; sell value 5,07,500.00; turnover 10,07,500.00; quantity 200; NSE VWAP (B+S)/(2Q) = 2,518.75; gross P&L (signal only, before charges) **7,500.00**.
Dated rows: E-2021 top slab Rs 345/cr (FA46730); IPFT Rs 0.01/cr; SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 5,00,000.00 = 15.00 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.94 -> 126.00, sell-value basis 127.00, per sell order 127.00; exchange line once 34.76 / split 34.76 / per order 34.76 (dated); today's exchange once 30.93 / per order 30.93.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 34.76 | 34.76 | 30.93 | 30.93 |
| SEBI turnover fee | 1.01 | 1.01 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 127.00 | 126.00 | 127.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.64 | 13.64 | 12.95 | 12.95 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 230.41 | 246.16 | 225.89 | 241.64 |

Headline charge (worse of the two worst-reading totals): **Rs 246.16** (dated clock). Net = 7,500.00 - 246.16 = **Rs 7,253.84**.

### CN14 — 2023-03-31 — last day Rs 345 + IPFT 0.01 (short, low price)

Quantity rule: 5,00,000 / 77.85 = 6422.6076; integers 6422 (Rs 4,99,952.70, off by 47.30) and 6423 (Rs 5,00,030.55, off by 30.55); closest = 6423. Fills: order 1 SELL: 6423 @ 77.85; order 2 BUY: 6423 @ 76.30.
Buy value 4,90,074.90; sell value 5,00,030.55; turnover 9,90,105.45; quantity 6423; NSE VWAP (B+S)/(2Q) = 77.08; gross P&L (signal only, before charges) **9,955.65**.
Dated rows: E-2021 top slab Rs 345/cr (FA46730); IPFT Rs 0.01/cr; SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 4,90,074.90 = 14.70 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 123.77 -> 124.00, sell-value basis 125.00, per sell order 125.00; exchange line once 34.16 / split 34.16 / per order 34.16 (dated); today's exchange once 30.40 / per order 30.40.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 34.16 | 34.16 | 30.40 | 30.40 |
| SEBI turnover fee | 0.99 | 0.99 | 0.99 | 0.99 |
| STT (sell side) | 124.00 | 125.00 | 124.00 | 125.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.53 | 13.53 | 12.85 | 12.86 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 227.68 | 243.43 | 223.24 | 239.00 |

Headline charge (worse of the two worst-reading totals): **Rs 243.43** (dated clock). Net = 9,955.65 - 243.43 = **Rs 9,712.22**.

### CN15 — 2023-04-03 — first trading day Rs 325 + IPFT 10 (same trade as CN14)

Quantity rule: 5,00,000 / 77.85 = 6422.6076; integers 6422 (Rs 4,99,952.70, off by 47.30) and 6423 (Rs 5,00,030.55, off by 30.55); closest = 6423. Fills: order 1 SELL: 6423 @ 77.85; order 2 BUY: 6423 @ 76.30.
Buy value 4,90,074.90; sell value 5,00,030.55; turnover 9,90,105.45; quantity 6423; NSE VWAP (B+S)/(2Q) = 77.08; gross P&L (signal only, before charges) **9,955.65**.
Dated rows: E-2023 top slab Rs 325/cr (FA56129); IPFT Rs 10/cr (FA56129); SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 4,90,074.90 = 14.70 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 123.77 -> 124.00, sell-value basis 125.00, per sell order 125.00; exchange line once 33.17 / split 33.17 / per order 33.17 (dated); today's exchange once 30.40 / per order 30.40.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 33.17 | 33.17 | 30.40 | 30.40 |
| SEBI turnover fee | 0.99 | 0.99 | 0.99 | 0.99 |
| STT (sell side) | 124.00 | 125.00 | 124.00 | 125.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.35 | 13.35 | 12.85 | 12.86 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 226.51 | 242.26 | 223.24 | 239.00 |

Headline charge (worse of the two worst-reading totals): **Rs 242.26** (dated clock). Net = 9,955.65 - 242.26 = **Rs 9,713.39**.

### CN16 — 2024-03-28 — last trading day Rs 325 + 10

Quantity rule: 5,00,000 / 1601.00 = 312.3048; integers 312 (Rs 4,99,512.00, off by 488.00) and 313 (Rs 5,01,113.00, off by 1113.00); closest = 312. Fills: order 1 BUY: 312 @ 1,601.00; order 2 SELL: 312 @ 1,640.00.
Buy value 4,99,512.00; sell value 5,11,680.00; turnover 10,11,192.00; quantity 312; NSE VWAP (B+S)/(2Q) = 1,620.50; gross P&L (signal only, before charges) **12,168.00**.
Dated rows: E-2023 top slab Rs 325/cr (FA56129); IPFT Rs 10/cr (FA56129); SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 4,99,512.00 = 14.99 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 126.40 -> 126.00, sell-value basis 128.00, per sell order 128.00; exchange line once 33.87 / split 33.87 / per order 33.87 (dated); today's exchange once 31.04 / per order 31.05.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 33.87 | 33.87 | 31.04 | 31.05 |
| SEBI turnover fee | 1.01 | 1.01 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 128.00 | 126.00 | 128.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.48 | 13.48 | 12.97 | 12.98 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 229.36 | 246.11 | 226.02 | 242.79 |

Headline charge (worse of the two worst-reading totals): **Rs 246.11** (dated clock). Net = 12,168.00 - 246.11 = **Rs 11,921.89**.

### CN17 — 2024-04-01 — first day Rs 322 + 10 (same trade as CN16)

Quantity rule: 5,00,000 / 1601.00 = 312.3048; integers 312 (Rs 4,99,512.00, off by 488.00) and 313 (Rs 5,01,113.00, off by 1113.00); closest = 312. Fills: order 1 BUY: 312 @ 1,601.00; order 2 SELL: 312 @ 1,640.00.
Buy value 4,99,512.00; sell value 5,11,680.00; turnover 10,11,192.00; quantity 312; NSE VWAP (B+S)/(2Q) = 1,620.50; gross P&L (signal only, before charges) **12,168.00**.
Dated rows: E-2024A top slab Rs 322/cr (FA61137); IPFT Rs 10/cr; SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 4,99,512.00 = 14.99 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 126.40 -> 126.00, sell-value basis 128.00, per sell order 128.00; exchange line once 33.57 / split 33.57 / per order 33.57 (dated); today's exchange once 31.04 / per order 31.05.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 33.57 | 33.57 | 31.04 | 31.05 |
| SEBI turnover fee | 1.01 | 1.01 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 128.00 | 126.00 | 128.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.42 | 13.42 | 12.97 | 12.98 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 229.00 | 245.75 | 226.02 | 242.79 |

Headline charge (worse of the two worst-reading totals): **Rs 245.75** (dated clock). Net = 12,168.00 - 245.75 = **Rs 11,922.25**.

### CN18 — 2024-09-30 — last day of slab ladders (losing long, high price, 51 shares)

Quantity rule: 5,00,000 / 9875.00 = 50.6329; integers 50 (Rs 4,93,750.00, off by 6250.00) and 51 (Rs 5,03,625.00, off by 3625.00); closest = 51. Fills: order 1 BUY: 51 @ 9,875.00; order 2 SELL: 51 @ 9,800.00.
Buy value 5,03,625.00; sell value 4,99,800.00; turnover 10,03,425.00; quantity 51; NSE VWAP (B+S)/(2Q) = 9,837.50; gross P&L (signal only, before charges) **-3,825.00**.
Dated rows: E-2024A top slab Rs 322/cr (FA61137); IPFT Rs 10/cr; SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 5,03,625.00 = 15.11 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.43 -> 125.00, sell-value basis 125.00, per sell order 125.00; exchange line once 33.31 / split 33.31 / per order 33.31 (dated); today's exchange once 30.81 / per order 30.80.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 33.31 | 33.31 | 30.81 | 30.81 |
| SEBI turnover fee | 1.00 | 1.00 | 1.00 | 1.00 |
| STT (sell side) | 125.00 | 125.00 | 125.00 | 125.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 13.38 | 13.38 | 12.93 | 12.93 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 227.69 | 242.44 | 224.74 | 239.49 |

Headline charge (worse of the two worst-reading totals): **Rs 242.44** (dated clock). Net = -3,825.00 - 242.44 = **Rs -4,067.44**.

### CN19 — 2024-10-01 — first day flat Rs 297 + 10 (same trade as CN18)

Quantity rule: 5,00,000 / 9875.00 = 50.6329; integers 50 (Rs 4,93,750.00, off by 6250.00) and 51 (Rs 5,03,625.00, off by 3625.00); closest = 51. Fills: order 1 BUY: 51 @ 9,875.00; order 2 SELL: 51 @ 9,800.00.
Buy value 5,03,625.00; sell value 4,99,800.00; turnover 10,03,425.00; quantity 51; NSE VWAP (B+S)/(2Q) = 9,837.50; gross P&L (signal only, before charges) **-3,825.00**.
Dated rows: E-2024B flat Rs 297/cr (FA64232); IPFT Rs 10/cr; SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 5,03,625.00 = 15.11 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 40.00; STT dated VWAP basis 125.43 -> 125.00, sell-value basis 125.00, per sell order 125.00; exchange line once 30.81 / split 30.80 / per order 30.80 (dated); today's exchange once 30.81 / per order 30.80.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 40.00 | 40.00 | 40.00 |
| NSE transaction charge + IPFT | 30.81 | 30.81 | 30.81 | 30.81 |
| SEBI turnover fee | 1.00 | 1.00 | 1.00 | 1.00 |
| STT (sell side) | 125.00 | 125.00 | 125.00 | 125.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 12.93 | 12.93 | 12.93 | 12.93 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 224.74 | 239.49 | 224.74 | 239.49 |

Headline charge (worse of the two worst-reading totals): **Rs 239.49** (both clocks equal). Net = -3,825.00 - 239.49 = **Rs -4,064.49**.

### CN20 — 2026-02-27 — last day Rs 297 + 10; short entry filled in 3 seconds, exit in 2

Quantity rule: 5,00,000 / 5000.00 = 100.0000; integers 100 (Rs 5,00,000.00, off by 0.00) and 101 (Rs 5,05,000.00, off by 5000.00); closest = 100. Fills: order 1 SELL: 40 @ 5,000.00, 35 @ 4,999.50, 25 @ 4,999.00; order 2 BUY: 60 @ 4,950.00, 40 @ 4,951.50.
Buy value 4,95,060.00; sell value 4,99,957.50; turnover 9,95,017.50; quantity 100; NSE VWAP (B+S)/(2Q) = 4,975.09; gross P&L (signal only, before charges) **4,897.50**.
Dated rows: E-2024B flat Rs 297/cr (FA64232); IPFT Rs 10/cr; SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 4,95,060.00 = 14.85 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 100.00; STT dated VWAP basis 124.38 -> 124.00, sell-value basis 125.00, per sell order 125.00; exchange line once 30.55 / split 30.55 / per order 30.55 (dated); today's exchange once 30.55 / per order 30.55.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 100.00 | 40.00 | 100.00 |
| NSE transaction charge + IPFT | 30.55 | 30.55 | 30.55 | 30.55 |
| SEBI turnover fee | 1.00 | 1.00 | 1.00 | 1.00 |
| STT (sell side) | 124.00 | 125.00 | 124.00 | 125.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 12.88 | 23.68 | 12.88 | 23.68 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 223.43 | 309.98 | 223.43 | 309.98 |

Headline charge (worse of the two worst-reading totals): **Rs 309.98** (both clocks equal). Net = 4,897.50 - 309.98 = **Rs 4,587.52**.

### CN21 — 2026-03-02 — first trading day Rs 306.99 + 0.01 (same trade as CN20)

Quantity rule: 5,00,000 / 5000.00 = 100.0000; integers 100 (Rs 5,00,000.00, off by 0.00) and 101 (Rs 5,05,000.00, off by 5000.00); closest = 100. Fills: order 1 SELL: 40 @ 5,000.00, 35 @ 4,999.50, 25 @ 4,999.00; order 2 BUY: 60 @ 4,950.00, 40 @ 4,951.50.
Buy value 4,95,060.00; sell value 4,99,957.50; turnover 9,95,017.50; quantity 100; NSE VWAP (B+S)/(2Q) = 4,975.09; gross P&L (signal only, before charges) **4,897.50**.
Dated rows: E-2026 flat Rs 306.99/cr (FA73061); IPFT Rs 0.01/cr (FA73061); SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 4,95,060.00 = 14.85 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 100.00; STT dated VWAP basis 124.38 -> 124.00, sell-value basis 125.00, per sell order 125.00; exchange line once 30.55 / split 30.55 / per order 30.55 (dated); today's exchange once 30.55 / per order 30.55.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 100.00 | 40.00 | 100.00 |
| NSE transaction charge + IPFT | 30.55 | 30.55 | 30.55 | 30.55 |
| SEBI turnover fee | 1.00 | 1.00 | 1.00 | 1.00 |
| STT (sell side) | 124.00 | 125.00 | 124.00 | 125.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 12.88 | 23.68 | 12.88 | 23.68 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 223.43 | 309.98 | 223.43 | 309.98 |

Headline charge (worse of the two worst-reading totals): **Rs 309.98** (both clocks equal). Net = 4,897.50 - 309.98 = **Rs 4,587.52**.

### CN22 — 2026-10-30 — QUANTITY TIE at Rs 5 lakh: 5,00,000 / 1,600 = 312.5; reading 313 (worse for charges); entry in 8 seconds, exit partly by own order and the rest by Dhan auto square-off after 15:10

Quantity rule: 5,00,000 / 1600.00 = 312.5000; integers 312 (Rs 4,99,200.00, off by 800.00) and 313 (Rs 5,00,800.00, off by 800.00); closest = TIE. Fills: order 1 BUY: 40 @ 1,600.00, 40 @ 1,600.05, 40 @ 1,600.10, 40 @ 1,600.10, 40 @ 1,600.20, 40 @ 1,600.25, 40 @ 1,600.30, 33 @ 1,600.40; order 2 SELL: 120 @ 1,617.00, 80 @ 1,616.50; order 3 SELL (Dhan auto square-off): 113 @ 1,611.05.
Buy value 5,00,853.20; sell value 5,05,408.65; turnover 10,06,261.85; quantity 313; NSE VWAP (B+S)/(2Q) = 1,607.45; gross P&L (signal only, before charges) **4,555.45**.
Dated rows: E-2026 flat Rs 306.99/cr (FA73061); IPFT Rs 0.01/cr (FA73061); SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 5,00,853.20 = 15.03 -> Rs 15.00.
Readings: brokerage per order 60.00 / per fill 210.24; STT dated VWAP basis 125.78 -> 126.00, sell-value basis 126.00, per sell order 127.00; exchange line once 30.89 / split 30.89 / per order 30.90 (dated); today's exchange once 30.89 / per order 30.90.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 60.00 | 210.24 | 60.00 | 210.24 |
| NSE transaction charge + IPFT | 30.89 | 30.90 | 30.89 | 30.90 |
| SEBI turnover fee | 1.01 | 1.01 | 1.01 | 1.01 |
| STT (sell side) | 126.00 | 127.00 | 126.00 | 127.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 16.54 | 43.59 | 16.54 | 43.59 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 20.00 | 20.00 | 20.00 | 20.00 |
| GST on square-off fee | 3.60 | 3.60 | 3.60 | 3.60 |
| **Total charges** | 273.04 | 466.09 | 273.04 | 466.09 |

Headline charge (worse of the two worst-reading totals): **Rs 466.09** (both clocks equal). Net = 4,555.45 - 466.09 = **Rs 4,089.36**.

### CN23 — 2026-10-30 — QUANTITY TIE, the other side: 312 shares, same fills as CN22 less one share on the last entry fill and the square-off

Quantity rule: 5,00,000 / 1600.00 = 312.5000; integers 312 (Rs 4,99,200.00, off by 800.00) and 313 (Rs 5,00,800.00, off by 800.00); closest = TIE. Fills: order 1 BUY: 40 @ 1,600.00, 40 @ 1,600.05, 40 @ 1,600.10, 40 @ 1,600.10, 40 @ 1,600.20, 40 @ 1,600.25, 40 @ 1,600.30, 32 @ 1,600.40; order 2 SELL: 120 @ 1,617.00, 80 @ 1,616.50; order 3 SELL (Dhan auto square-off): 112 @ 1,611.05.
Buy value 4,99,252.80; sell value 5,03,797.60; turnover 10,03,050.40; quantity 312; NSE VWAP (B+S)/(2Q) = 1,607.45; gross P&L (signal only, before charges) **4,544.80**.
Dated rows: E-2026 flat Rs 306.99/cr (FA73061); IPFT Rs 0.01/cr (FA73061); SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 4,99,252.80 = 14.98 -> Rs 15.00.
Readings: brokerage per order 60.00 / per fill 209.76; STT dated VWAP basis 125.38 -> 125.00, sell-value basis 126.00, per sell order 126.00; exchange line once 30.79 / split 30.79 / per order 30.80 (dated); today's exchange once 30.79 / per order 30.80.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 60.00 | 209.76 | 60.00 | 209.76 |
| NSE transaction charge + IPFT | 30.79 | 30.80 | 30.79 | 30.80 |
| SEBI turnover fee | 1.00 | 1.00 | 1.00 | 1.00 |
| STT (sell side) | 125.00 | 126.00 | 125.00 | 126.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 16.52 | 43.48 | 16.52 | 43.48 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 20.00 | 20.00 | 20.00 | 20.00 |
| GST on square-off fee | 3.60 | 3.60 | 3.60 | 3.60 |
| **Total charges** | 271.91 | 464.39 | 271.91 | 464.39 |

Headline charge (worse of the two worst-reading totals): **Rs 464.39** (both clocks equal). Net = 4,544.80 - 464.39 = **Rs 4,080.41**.

### CN24 — 2026-10-30 — fills under Rs 66,666.67 each: entry in 10 seconds of ~Rs 50,000 (brokerage per fill hits the 0.03% leg); one exit fill

Quantity rule: 5,00,000 / 160.00 = 3125.0000; integers 3125 (Rs 5,00,000.00, off by 0.00) and 3126 (Rs 5,00,160.00, off by 160.00); closest = 3125. Fills: order 1 BUY: 313 @ 160.00, 313 @ 160.00, 313 @ 160.05, 312 @ 160.05, 312 @ 160.10, 312 @ 160.10, 312 @ 160.15, 312 @ 160.15, 313 @ 160.20, 313 @ 160.20; order 2 SELL: 3125 @ 158.40.
Buy value 5,00,312.45; sell value 4,95,000.00; turnover 9,95,312.45; quantity 3125; NSE VWAP (B+S)/(2Q) = 159.25; gross P&L (signal only, before charges) **-5,312.45**.
Dated rows: E-2026 flat Rs 306.99/cr (FA73061); IPFT Rs 0.01/cr (FA73061); SEBI Rs 10/cr; GST on SEBI fee: yes; stamp: 0.003% x buy 5,00,312.45 = 15.01 -> Rs 15.00.
Readings: brokerage per order 40.00 / per fill 170.09; STT dated VWAP basis 124.41 -> 124.00, sell-value basis 124.00, per sell order 124.00; exchange line once 30.56 / split 30.56 / per order 30.56 (dated); today's exchange once 30.56 / per order 30.56.

| Component (Rs) | Dated, precise | Dated, worst reading | Today (Dhan 30 Oct 2026), precise | Today, worst reading |
|---|---:|---:|---:|---:|
| Brokerage (Dhan) | 40.00 | 170.09 | 40.00 | 170.09 |
| NSE transaction charge + IPFT | 30.56 | 30.56 | 30.56 | 30.56 |
| SEBI turnover fee | 1.00 | 1.00 | 1.00 | 1.00 |
| STT (sell side) | 124.00 | 124.00 | 124.00 | 124.00 |
| Stamp duty | 15.00 | 15.00 | 15.00 | 15.00 |
| GST 18% | 12.88 | 36.30 | 12.88 | 36.30 |
| DP charge (intraday reading) | 0.00 | 12.50 | 0.00 | 12.50 |
| GST on DP | 0.00 | 2.25 | 0.00 | 2.25 |
| Dhan auto square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| GST on square-off fee | 0.00 | 0.00 | 0.00 | 0.00 |
| **Total charges** | 223.44 | 391.70 | 223.44 | 391.70 |

Headline charge (worse of the two worst-reading totals): **Rs 391.70** (both clocks equal). Net = -5,312.45 - 391.70 = **Rs -5,704.15**.

## Appendix A. Reproduce

```
cd state/fib-strategy-20261030/costs-evidence/calc
node golden.js                              # the crate's five golden vectors, MATCH x5
node vectors.js /tmp/v.md /tmp/v.json       # regenerates section 9 and the vectors JSON
```

`vec.js` holds the dated rows as code: `dated(date)` returns the NSE, IPFT and SEBI rows and the GST-on-SEBI and stamp-regime switches for a date. `today()` returns Dhan's rates of 2026-10-30. `price(vector, clock)` returns the precise (P) and worst-reading (W) components. Money is BigInt paise and rates are exact rationals, half-up to the paisa and half-up to the rupee where a source says so.

## Appendix B. Documents read on 2026-10-30, and how

| Document | URL | Read via |
|---|---|---|
| NSE/F&A/13028 (2009), FA24578 (2013), FA26775 (2014) | nsearchives.nseindia.com/content/circulars/faac13028.pdf, FA24578.pdf, FA26775.zip | curl, pdftotext |
| NSE/FA/45147, 46225, 46730, 47844, 49796, 51855, 56129, 61137, 64232, 73061 | nsearchives.nseindia.com/content/circulars/<id>.pdf | curl, pdftotext |
| NSE/FA/50832, 53913 (image PDFs) | same host | curl, read as images |
| NSE/FA/34501 + SEBI GN/2016-17/037; NSE/FA/40632 + SEBI GN/2019/03; NSE/FA/44514 + SEBI GN/2020/011 | same host | curl, bsdtar, pdftotext |
| NSE/FATAX/23500, 56235, 63809, 73524 (STT); NSE/FATAX/52993 (GST on SEBI fee) | same host | curl, pdftotext |
| NSE/CMTR/5588 (STT method, 2004) | nsearchives.nseindia.com/content/circulars/cmtr5588.htm | curl, tags stripped |
| NCL/CMPT/43115 + Indian Stamp Rules 2019 (G.S.R. 901(E)); NCL/CMPT/44032 + Gazette of 30 Mar 2020 | nsearchives.nseindia.com/content/circulars/CMPT43115.zip, CMPT44032.zip | curl, bsdtar, pdftotext |
| PIB release PRID 1635399 (30 Jun 2020) | www.pib.gov.in/PressReleasePage.aspx?PRID=1635399 | curl |
| Notification 11/2017-Central Tax (Rate) | gstcouncil.gov.in/sites/default/files/2024-05/download_4.pdf | curl, pdftotext |
| NSE levies page (updated 17/04/2026); NSE member stamp-duty page | www.nseindia.com/static/invest/first-time-investor-sebi-turnover-fees-stt-other-levies; www.nseindia.com/static/trade/membership-stamp-duty | curl |
| NSE circular list 2008-01-01 to 2026-10-30 (73,089 entries) | www.nseindia.com/api/circulars?fromDate=..&toDate=.. | curl after a cookie from the circulars page; jq |
| Dhan pricing; Dhan auto square-off timing; Dhan DP FAQ | dhan.co/pricing/; dhan.co/support/general/market-session-status-and-timing/what-are-the-dhan-intraday-auto-square-off-timings/; dhan.co/support/general/pricing/dp-charges-will-be-levied-for-every-delivery-sell-order-per-scrip-per-day-for-any-given-quantity/ | curl |
| Zerodha archived charges page (2019-06-22), calculator (2018-07-14) and script (2019-04-03): SECONDARY | web.archive.org/web/20190622231659id_/https://zerodha.com/charges; web.archive.org/web/20180714193535id_/https://zerodha.com/brokerage-calculator; web.archive.org/web/20190403072919js_/https://zerodha.com/static/app.js?v=7.0.7 | curl |

Web searches (WebSearch) were used only to find pages. No figure here rests on a search summary. The 2019-06-22 Zerodha charges page shows "NSE: 0.00325%" for equity intraday, a secondary cross-check of EX-2009's top slab. Its footnote reads "SEBI charges will be reduced to ₹15 per crore once the regulator announces a date": a broker's note, superseded by SF-2019.

## Addendum, Sat 7 Nov 2026: real Dhan order estimator against this calculator
Operator's screenshot (costs-evidence/dhan-order-estimator-adani-20261107.webp, 10:54 IST): Adani Enterprises, intraday limit buy, 547 x Rs 2,742.00 = Rs 14,99,874.00, margin Rs 3,00,014.80 (5.00x). Buy: brokerage 20.00, exchange 46.05, GST 11.89, SEBI 1.50, stamp 45.00, DP 0.00, total 124.44. Sell estimate 454.41 (= 20.00 + 46.05 + 11.89 + 1.50 + STT 374.97). Round trip 578.85; break-even Rs 2,743.06, +0.0387%.
This calculator on the same order, official method: 579.43, Rs 0.58 above Dhan's: Rs 0.54 is GST on the SEBI fee (NSE/FATAX/52993; the estimator leaves it out) and the rest is STT rounding to the rupee. The worst reading was 594.18: 579.43 + DP Rs 12.50 + its GST Rs 2.25. The estimator's words: "approximate, not actual; refer to contract note".
RD-DP SETTLED for a squared intraday trade: no DP charge. RULING (main session, operator's message, requirement 26): the headline uses the official method; the per-fill-second brokerage reading becomes a labelled stress line, because Dhan's page and estimator both say Rs 20 per order; a real contract note with a partly filled order settles it.

Option estimator, same morning (costs-evidence/dhan-order-estimator-hdfcbank-770ce-20261107.webp, 11:00 IST, market closed): HDFCBANK 23 Nov 770 CALL, 18 lots, 11,700 x Rs 26.95 = Rs 3,15,315, margin 1X. Buy 165.58 (brokerage 20.00, exchange 112.03, GST 23.77, SEBI 0.32, stamp 9.00); sell 629.09 (with STT 472.97); round trip 794.67; break-even Rs 27.02, +0.25%. Quote on the same screen: SELL 26.90 / BUY 27.40, a spread of Rs 0.50 = 1.85% = Rs 5,850 on this quantity, about 7 times the charges (market closed, so a live spread may differ): the store holds traded prices, not this quote, and the option tools' spread lanes stop at 2 ticks.
