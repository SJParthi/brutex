# Charges oracle — the official rate table, 2018-09-01 to 2026-10-31

Every charge on an NSE **intraday** trade in three segments — **equity cash
intraday, stock options, index options** — for every date from 2018-09-01 to
2026-10-31, from official sources only: NSE and NSE Clearing circulars, SEBI,
the Income-tax department and the Finance Acts (STT), the Indian Stamp Act as
amended by the Finance Act 2019, the GST rate notification, and Dhan's own
published pricing.

* **Machine-readable twin:** `crates/costs/tests/fixtures/charges_oracle_rates.toml`.
  Every table row in §3 is generated from it, cell for cell, and
  `charges_oracle::rates_md_carries_every_fixture_row_exactly` fails the build if
  the two drift apart.
* **The oracle that prices from it:** `crates/costs/tests/charges_oracle.rs`
  (test-only; no product code reads this table).
* **What the comparison with the product found:** `CHARGES-ORACLE-REPORT.md` at
  the repository root.

Written 10 Nov 2026 by an independent cloud session. Clean-room: the rates here
were researched without reading the product's rate tables, and the product's
own citations (`docs/00-charter.md`, CHG-1) are compared only in the report.

## 1. How the sources were read — and what that bounds

* **No official page was fetched.** The session's network policy refused every
  official host (NSE, SEBI, India Code, the e-Gazette, the Budget site, Dhan, the
  Wayback Machine), and two requests to widen it were rejected by the
  environment service. Every quote below is **text a web-search tool returned
  from the official URL named on the row**, with the search restricted to that
  official domain.
* The `note` column says whether the tool returned the string **verbatim** (as
  the source's own text) or **paraphrased** it — slab tables in particular were
  read through the tool's paraphrase. A row is only as strong as its note.
* The search budget for the session (200 queries across four research agents)
  ran out before every question was answered. What is still open is in §6, and
  every such gap is a row with status `UNKNOWN` or an `assumption`, never an
  invented number.
* Brokerage, the exchange charges, IPFT, clearing and the SEBI fee are levied
  **each side** (buy and sell). STT is the **seller's** (on a sale) or the
  **holder's** (on an exercise). Stamp duty is the **buyer's**. GST is on the
  fee base.

**Status values.** `verified`: the figure is quoted from the official source.
`assumption`: the figure is official but applying it to one client order needs
a stated choice (which slab, which first day, a pass-through). `counterfactual`:
the figure did not exist on those dates (Dhan had not launched) and is applied
as a labelled what-if. `UNKNOWN`: no official figure was reached; the oracle
prices nothing for that line and says so.

## 2. The timeline at a glance

Rates are each side unless the column says otherwise. "lakh" is ₹1,00,000;
"crore" is ₹1,00,00,000.

### Equity cash, intraday (non-delivery)

| From | To | Exchange txn charge | IPFT | SEBI fee | STT (seller, non-delivery value) | Stamp duty (buyer) | SEBI fee in GST base |
|---|---|---|---|---|---|---|---|
| 2018-09-01 | 2019-03-31 | ₹3.25 /lakh (first slab) | ₹0.01 /cr | ₹15 /cr | 0.025% | state duty — UNKNOWN | no |
| 2019-04-01 | 2020-05-31 | ₹3.25 /lakh | ₹0.01 /cr | ₹10 /cr | 0.025% | state duty — UNKNOWN | no |
| 2020-06-01 | 2020-06-30 | ₹3.25 /lakh | ₹0.01 /cr | ₹5 /cr | 0.025% | state duty — UNKNOWN | no |
| 2020-07-01 | 2020-12-31 | ₹3.25 /lakh | ₹0.01 /cr | ₹5 /cr | 0.025% | 0.003% | no |
| 2021-01-01 | 2021-03-31 | ₹3.45 /lakh (first slab) | ₹0.01 /cr | ₹5 /cr | 0.025% | 0.003% | no |
| 2021-04-01 | 2022-07-17 | ₹3.45 /lakh | ₹0.01 /cr | ₹10 /cr | 0.025% | 0.003% | no |
| 2022-07-18 | 2023-03-31 | ₹3.45 /lakh | ₹0.01 /cr | ₹10 /cr | 0.025% | 0.003% | yes |
| 2023-04-01 | 2024-03-31 | ₹3.25 /lakh (first slab) | ₹10 /cr | ₹10 /cr | 0.025% | 0.003% | yes |
| 2024-04-01 | 2024-09-30 | ₹3.22 /lakh (first slab) | ₹10 /cr | ₹10 /cr | 0.025% | 0.003% | yes |
| 2024-10-01 | 2026-02-28 | ₹2.97 /lakh (one rate for all) | ₹10 /cr | ₹10 /cr | 0.025% | 0.003% | yes |
| 2026-03-01 | 2026-10-31 | ₹306.99 /cr | ₹0.01 /cr | ₹10 /cr | 0.025% | 0.003% | yes |

### Stock options and index options (premium)

NSE has one "Equity Options" line for index and stock options alike. Only the
exercise differs: index options settle in cash; stock options settle by
delivery, whose STT was not reached (`STTX-STK`, UNKNOWN).

| From | To | Exchange txn charge | IPFT | SEBI fee | STT on sale (seller, premium) | STT on exercise (holder, index) | Stamp duty (buyer, premium) | SEBI fee in GST base |
|---|---|---|---|---|---|---|---|---|
| 2018-09-01 | 2019-03-31 | ₹50.00 /lakh (slab) | ₹0.01 /cr | ₹15 /cr | 0.05% | 0.125% of the **settlement value** | state duty — UNKNOWN | no |
| 2019-04-01 | 2019-08-31 | ₹50.00 /lakh | ₹0.01 /cr | ₹10 /cr | 0.05% | 0.125% of the **settlement value** | state duty — UNKNOWN | no |
| 2019-09-01 | 2020-05-31 | ₹50.00 /lakh | ₹0.01 /cr | ₹10 /cr | 0.05% | 0.125% of the intrinsic value | state duty — UNKNOWN | no |
| 2020-06-01 | 2020-06-30 | ₹50.00 /lakh | ₹0.01 /cr | ₹5 /cr | 0.05% | 0.125% of intrinsic | state duty — UNKNOWN | no |
| 2020-07-01 | 2020-12-31 | ₹50.00 /lakh | ₹0.01 /cr | ₹5 /cr | 0.05% | 0.125% of intrinsic | 0.003% | no |
| 2021-01-01 | 2021-03-31 | ₹53.00 /lakh (slab) | ₹0.01 /cr | ₹5 /cr | 0.05% | 0.125% of intrinsic | 0.003% | no |
| 2021-04-01 | 2022-07-17 | ₹53.00 /lakh | ₹0.01 /cr | ₹10 /cr | 0.05% | 0.125% of intrinsic | 0.003% | no |
| 2022-07-18 | 2023-03-31 | ₹53.00 /lakh | ₹0.01 /cr | ₹10 /cr | 0.05% | 0.125% of intrinsic | 0.003% | yes |
| 2023-04-01 | 2024-03-31 | ₹50.00 /lakh (slab) | ₹50 /cr | ₹10 /cr | 0.0625% | 0.125% of intrinsic | 0.003% | yes |
| 2024-04-01 | 2024-09-30 | ₹49.50 /lakh (slab) | ₹50 /cr | ₹10 /cr | 0.0625% | 0.125% of intrinsic | 0.003% | yes |
| 2024-10-01 | 2026-02-28 | ₹35.03 /lakh (one rate for all) | ₹50 /cr | ₹10 /cr | 0.1% | 0.125% of intrinsic | 0.003% | yes |
| 2026-03-01 | 2026-03-31 | ₹3,552.99 /cr | ₹0.01 /cr | ₹10 /cr | 0.1% | 0.125% of intrinsic | 0.003% | yes |
| 2026-04-01 | 2026-10-31 | ₹3,552.99 /cr | ₹0.01 /cr | ₹10 /cr | 0.15% | 0.15% of intrinsic | 0.003% | yes |

**Throughout:** GST 18% (9% CGST + 9% SGST, or 18% IGST); clearing charge nil
to the client; Dhan brokerage the lower of ₹20 and 0.03% per executed order for
equity intraday and ₹20 per executed order for options — Dhan's public launch
was November 2021, so before 2021-11-04 that schedule is a `counterfactual`.

**On 2026-03-01 the total exchange outflow did not change.** NSE `FA73061`
moved money from the IPFT contribution into the transaction charge: cash
₹297 + ₹10 = ₹306.99 + ₹0.01 = ₹307 per crore; options ₹3,503 + ₹50 =
₹3,552.99 + ₹0.01 = ₹3,553 per crore of premium.

## 3. The rate table

Every row below is generated from the fixture. `rate` is the exact fraction the
oracle computes with.

### 3.1 Brokerage — Dhan (Raise Securities Pvt Ltd)

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| BRK-EQ-RATE-0 | equity_intraday | both sides | 0.03% (3/10000) | 0.03% of the order value (what-if: Dhan had not launched; its launch tariff applied backwards) | order value, per executed order | 2018-09-01 | 2021-11-03 | counterfactual | https://dhan.co/ai-info/ | "Founded: 2021 · Launched: November 2021" | verbatim via search. Dhan published no price before its public launch (Muhurat trading, 4 Nov 2021, community.dhan.co, paraphrase via search). |
| BRK-EQ-RATE-1 | equity_intraday | both sides | 0.03% (3/10000) | the lower of Rs 20 and 0.03% of the order value, per executed order | order value, per executed order | 2021-11-04 | 2026-10-31 | assumption | https://knowledge.dhan.co/support/solutions/articles/82000830571-what-are-the-trading-investing-charges-for-dhan- ; https://dhan.co/pricing/ | "Equity Intraday & ETF Equity Intraday: Rs. 20/- or 0.03%, whichever is lower on per executed order." | verbatim via search (knowledge base). The figure is the current page's; that it held unchanged since launch is assumed — no dated snapshot was reachable (web.archive.org refused). |
| BRK-EQ-CAP-0 | equity_intraday | both sides | Rs 20.00 | Rs 20 per executed order (what-if, as BRK-EQ-RATE-0) | paisa per executed order | 2018-09-01 | 2021-11-03 | counterfactual | https://dhan.co/ai-info/ | "Founded: 2021 · Launched: November 2021" | verbatim via search. |
| BRK-EQ-CAP-1 | equity_intraday | both sides | Rs 20.00 | Rs 20 per executed order, the cap on the 0.03% | paisa per executed order | 2021-11-04 | 2026-10-31 | assumption | https://dhan.co/pricing/ | "Brokerage \| ₹ 0 (Zero Brokerage) \| ₹ 20 or 0.03% per executed order whichever is lower" | verbatim via search. Constancy since launch assumed, as BRK-EQ-RATE-1. |
| BRK-OPT-0 | stock_option, index_option | both sides | Rs 20.00 | Rs 20 per executed order (what-if, as BRK-EQ-RATE-0) | paisa per executed order | 2018-09-01 | 2021-11-03 | counterfactual | https://dhan.co/ai-info/ | "Founded: 2021 · Launched: November 2021" | verbatim via search. |
| BRK-OPT-1 | stock_option, index_option | both sides | Rs 20.00 | Rs 20 per executed order, whatever the size | paisa per executed order | 2021-11-04 | 2026-10-31 | assumption | https://knowledge.dhan.co/support/solutions/articles/82000830571-what-are-the-trading-investing-charges-for-dhan- ; https://dhan.co/pricing/ | "Options (Equity, Commodities, Currency): Flat Rs. 20/- per executed order." | verbatim via search. CONFLICT: https://dhan.co/options/ says "For options trading, there is a ₹20 (or 0.03%) charge per order executed in any segment's options." The knowledge base and pricing page say a fixed Rs 20; that is used. Constancy since launch assumed. |

### 3.2 NSE exchange transaction charge

Before 2024-10-01 NSE billed each **member** on a monthly-turnover slab ladder
(cash: up to ₹1,250 crore a month at the first rate, falling in steps of 5 paise
per lakh; options: an "up to ₹3 crore" row, then per-lakh slabs). No per-client
rate existed. The rows take the ladder's **highest per-lakh rate**, the one a
member pays on its first slab — what brokers commonly passed to clients — and
are marked `assumption` for that reason. SEBI's true-to-label circular says why
this ended: "The MII charges which are to be recovered by the trading members
from the end client should be True to Label where in the amount recovered from
end clients is the same as amount paid by the trading members to the MIIs."
(NSE `FA64232`, verbatim via search.)

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| EXC-EQ-2009 | equity_intraday | both sides | 0.00325% (13/400000) | Rs 3.25 per lakh each side (first slab, up to Rs 1,250 crore a month) | traded value | 2018-09-01 | 2020-12-31 | assumption | https://archives.nseindia.com/content/circulars/faac13028.pdf ; https://archives.nseindia.com/content/circulars/FA46730.pdf | "Rs. 3.25 each side per lakh" | fragment verbatim via search (NSE/F&A/13028, 7 Sep 2009, effective 1 Oct 2009); FA46730's 'existing' column confirms 3.25 as at Dec 2020 (paraphrase). No circular dated 2018-09 to 2020-12 surfaced. |
| EXC-EQ-2021 | equity_intraday | both sides | 0.00345% (69/2000000) | Rs 3.45 per lakh each side (first slab) | traded value | 2021-01-01 | 2023-03-31 | assumption | https://archives.nseindia.com/content/circulars/FA46730.pdf | "Trading members may be aware that there have been certain market exigencies due to broker member defaults." | quote verbatim via search; the slab values (3.45 / 3.40 / 3.35 / 3.30 / 3.25 / 3.20) are the tool's paraphrase of the table, repeated as FA56129's 'existing' column. FA46730 is dated 18 Dec 2020. |
| EXC-EQ-2023 | equity_intraday | both sides | 0.00325% (13/400000) | Rs 3.25 per lakh each side (first slab) | traded value | 2023-04-01 | 2024-03-31 | assumption | https://archives.nseindia.com/content/circulars/FA56129.pdf | "The above revision in transaction charges will be applicable with effect from April 01, 2023." | date verbatim via search; values paraphrase. FA56129 is dated 24 Mar 2023. |
| EXC-EQ-2024A | equity_intraday | both sides | 0.00322% (161/5000000) | Rs 3.22 per lakh each side (first slab) | traded value | 2024-04-01 | 2024-09-30 | assumption | https://nsearchives.nseindia.com/content/circulars/FA61137.pdf | "The above revision in transaction charges will be applicable with effect from April 01, 2024." | quote verbatim via search; 3.22 (first slab) and 2.97 (last) read, the middle slabs derived. FA61137 is dated 14 Mar 2024. |
| EXC-EQ-2024B | equity_intraday | both sides | 0.00297% (297/10000000) | Rs 2.97 per lakh each side, the same for every member | traded value | 2024-10-01 | 2026-02-28 | verified | https://nsearchives.nseindia.com/content/circulars/FA64232.pdf | "the Exchange has now decided to levy the following transaction charges to the Trading members: Cash Market: Rs. 2.97 each side per lakh of traded value" | verbatim via search. FA64232 is dated 27 Sep 2024, effective 1 Oct 2024, under SEBI/HO/MRD/TPD-1/P/CIR/2024/92 (true to label). |
| EXC-EQ-2026 | equity_intraday | both sides | 0.0030699% (30699/1000000000) | Rs 306.99 per crore each side | traded value | 2026-03-01 | 2026-10-31 | verified | https://nsearchives.nseindia.com/content/circulars/FA73061.pdf | "The above revision will be applicable with effect from March 1, 2026." | date verbatim via search; 297 -> 306.99 is the tool's reading of the table, the same in more than five searches. FA73061 is dated 27 Feb 2026. No later circular found to 2026-10-31. |
| EXC-OPT-2009 | stock_option, index_option | both sides | 0.05% (1/2000) | Rs 50.00 per lakh of premium (slab above Rs 3 crore up to Rs 100 crore a month) | premium value | 2018-09-01 | 2020-12-31 | assumption | https://archives.nseindia.com/content/circulars/FA46730.pdf | "Up to Rs 3 Crores, Rs. 2,500/-" | fragment verbatim via search; the per-lakh slab values are FA46730's 'existing' column (paraphrase). The 'up to Rs 3 crore: Rs 2,500' first row is unclear (a fixed monthly amount, or an extraction artefact); the highest per-lakh rate is used. |
| EXC-OPT-2021 | stock_option, index_option | both sides | 0.053% (53/100000) | Rs 53.00 per lakh of premium (highest per-lakh slab) | premium value | 2021-01-01 | 2023-03-31 | assumption | https://archives.nseindia.com/content/circulars/FA46730.pdf | "Trading members may be aware that there have been certain market exigencies due to broker member defaults." | quote verbatim via search; 53.00 / 50.50 / 45.50 paraphrase, repeated as FA56129's 'existing' column. |
| EXC-OPT-2023 | stock_option, index_option | both sides | 0.05% (1/2000) | Rs 50.00 per lakh of premium (highest per-lakh slab) | premium value | 2023-04-01 | 2024-03-31 | assumption | https://archives.nseindia.com/content/circulars/FA56129.pdf | "The above revision in transaction charges will be applicable with effect from April 01, 2023." | date verbatim via search; values paraphrase. |
| EXC-OPT-2024A | stock_option, index_option | both sides | 0.0495% (99/200000) | Rs 49.50 per lakh of premium (highest per-lakh slab) | premium value | 2024-04-01 | 2024-09-30 | assumption | https://nsearchives.nseindia.com/content/circulars/FA61137.pdf | "In order to optimise the overall cost of trading, the Exchange has now decided to reduce the transaction charges levied to the Trading members." | quote verbatim via search; 49.50 / 47.00 paraphrase. |
| EXC-OPT-2024B | stock_option, index_option | both sides | 0.03503% (3503/10000000) | Rs 35.03 per lakh of premium each side, the same for every member | premium value | 2024-10-01 | 2026-02-28 | verified | https://nsearchives.nseindia.com/content/circulars/FA64232.pdf | "Equity Options: Rs. 35.03 each side per lakh of premium value" | verbatim via search. One 'Equity Options' line: no separate index or stock option rate. |
| EXC-OPT-2026 | stock_option, index_option | both sides | 0.0355299% (355299/1000000000) | Rs 3,552.99 per crore of premium each side | premium value | 2026-03-01 | 2026-10-31 | verified | https://nsearchives.nseindia.com/content/circulars/FA73061.pdf | "The above revision will be applicable with effect from March 1, 2026." | date verbatim via search; 3,503 -> 3,552.99 is the tool's reading of the table (one search said the cell was garbled; the 3,553 total with IPFT is consistent). |

### 3.3 NSE Investor Protection Fund Trust (IPFT) contribution

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| IPFT-ALL-0 | equity_intraday, stock_option, index_option | both sides | 0.0000001% (1/1000000000) | Rs 0.01 per crore each side | traded value (premium value for options) | 2018-09-01 | 2023-03-31 | assumption | https://archives.nseindia.com/content/circulars/FA56129.pdf | "recalibrated the contribution from the existing Rs.0.01 per crore of traded value to Rs.10 per crore of the traded value" | near-verbatim via search (the tool set it inside its own sentence). Rs 0.01 was 'existing' in March 2023; that it applied from 2018-09-01 is assumed. |
| IPFT-EQ-2023 | equity_intraday | both sides | 0.0001% (1/1000000) | Rs 10 per crore each side, charged on top of the transaction charge | traded value | 2023-04-01 | 2026-02-28 | verified | https://archives.nseindia.com/content/circulars/FA56129.pdf ; https://nsearchives.nseindia.com/content/circulars/FA73061.pdf | "recalibrated the contribution from the existing Rs.0.01 per crore of traded value to Rs.10 per crore of the traded value" | near-verbatim via search. |
| IPFT-OPT-2023 | stock_option, index_option | both sides | 0.0005% (1/200000) | Rs 50 per crore of premium each side | premium value | 2023-04-01 | 2026-02-28 | verified | https://archives.nseindia.com/content/circulars/FA56129.pdf ; https://nsearchives.nseindia.com/content/circulars/FA73061.pdf | "recalibrated the contribution from the existing Rs.0.01 per crore of traded value to Rs.10 per crore of the traded value" | the Rs 50 per crore of premium figure is paraphrase via search ('raised ... to Rs 50 per crore of traded premium value, up from Rs 0.01'); FA73061's table shows the same Rs 50 as the figure it replaced. |
| IPFT-ALL-2026 | equity_intraday, stock_option, index_option | both sides | 0.0000001% (1/1000000000) | Rs 0.01 per crore each side | traded value (premium value for options) | 2026-03-01 | 2026-10-31 | verified | https://nsearchives.nseindia.com/content/circulars/FA73061.pdf | "the Exchange has now decided to roll back the increase made in the Trading Member's contributions towards NSE IPFT at the level levied prior to April 1, 2023" | verbatim via search; the figure is the pre-April-2023 level (IPFT-ALL-0). |

### 3.4 Clearing charge

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| CLR-ALL | equity_intraday, stock_option, index_option | both sides | 0.0% (0/1) | nil to the client: the exchange pays NSE Clearing out of its transaction charge | traded value | 2018-09-01 | 2026-10-31 | assumption | https://nsearchives.nseindia.com/web/sites/default/files/inline-files/NSE%20Earnings%2006May%202024%20final.pdf ; https://nsearchives.nseindia.com/corporate/BSE_19112024165318_NSEIntimation.pdf | "The clearing and settlement charges paid to our subsidiary company, NSE Clearing Limited and Indian Clearing Corporation Limited owned by BSE" | verbatim via search (NSE Q4 FY24 call). No NSE or NSE Clearing circular stating nil was found; nil is an inference. |

### 3.5 SEBI turnover fee

For options the fee is on the premium: "In case of options contracts,
'turnover' shall be computed on the basis of premium traded for the option
contracts and in case where the option is exercised or assigned, it shall be
additionally computed on the basis of notional value of option contracts
exercised or assigned." (SEBI Stock Brokers Regulations schedule, verbatim via
search; https://www.sebi.gov.in/acts/stockreg.pdf.)

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| SEBI-15 | equity_intraday, stock_option, index_option | both sides | 0.00015% (3/2000000) | Rs 15 per crore (0.00015%) | traded value (premium value for options) | 2018-09-01 | 2019-03-31 | verified | https://www.sebi.gov.in/sebi_data/meetingfiles/mar-2019/1553244864312_1.pdf | "reducing the broker turnover fee (other than Debt Securities & Agri Commodity Derivative) from current Rs.15/- per crore to Rs. 10/-per crore" | verbatim via search (SEBI board memo, March 2019). |
| SEBI-10A | equity_intraday, stock_option, index_option | both sides | 0.0001% (1/1000000) | Rs 10 per crore (0.0001%) | traded value (premium value for options) | 2019-04-01 | 2020-05-31 | assumption | https://www.sebi.gov.in/sebiweb/home/HomeAction.do?doListingAll=yes&search=Fees | "Securities and Exchange Board of India (Payment of Fees)(Amendment) Regulations, 2019" | listing title verbatim via search, dated March 22, 2019. The first day is UNVERIFIED: 2019-04-01 is used; one non-official notice says 3 Apr 2019. |
| SEBI-5 | equity_intraday, stock_option, index_option | both sides | 0.00005% (1/2000000) | Rs 5 per crore: 50% of Rs 10, June 2020 to March 2021 | traded value (premium value for options) | 2020-06-01 | 2021-03-31 | verified | https://www.sebi.gov.in/media/press-releases/apr-2020/sebi-reduces-broker-turnover-fees-and-filing-fees-for-issuers_46571.html | "The broker turnover fee will be reduced to 50% of the existing fee structure for the period June 2020 to March 2021." | verbatim via search (PR No. 24/2020, 27 Apr 2020), which adds: "The benefit of the above reduction in fees will automatically be passed on to the investors as well." No implementing regulation or exchange circular was read. |
| SEBI-10B | equity_intraday, stock_option, index_option | both sides | 0.0001% (1/1000000) | Rs 10 per crore (0.0001%) | traded value (premium value for options) | 2021-04-01 | 2026-10-31 | verified | https://www.nseindia.com/static/invest/first-time-investor-sebi-turnover-fees-stt-other-levies | "0.0001 per cent of the price at which the securities are purchased or sold (Rs.10 per crore)" | verbatim via search (a page updated after April 2026). The return to Rs 10 on 2021-04-01 follows from SEBI-5's end date. Options: 'turnover shall be computed on the basis of premium traded' (SEBI Stock Brokers Regulations schedule, verbatim via search). |

### 3.6 Securities transaction tax (Chapter VII, Finance (No. 2) Act 2004)

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| STT-EQ | equity_intraday | sell | 0.025% (1/4000) | 0.025%, seller | non-delivery value: offset quantity x the client's VWAP for the scrip and day | 2018-09-01 | 2026-10-31 | verified | https://www.incometaxindia.gov.in/w/section-98-55 ; https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf | "Sale of an equity share in a company or a unit of an equity oriented fund [or a unit of a business trust], where— 0.025 per cent Seller (a) the transaction of such sale is entered into in a recognised stock exchange; and (b) the contract for the sale of such share or unit is settled otherwise than by the actual delivery or transfer of such share or unit." | verbatim via search; FATAX73524 (31 Mar 2026): "0.025 per cent (No Change)". |
| STT-OPT-1 | stock_option, index_option | sell | 0.05% (1/2000) | 0.05% of the premium, seller | option premium value | 2018-09-01 | 2023-03-31 | verified | https://archives.nseindia.com/content/circulars/FATAX56235.pdf | "(up to March 31, 2023 – 0.05%)" | source fragment via search (NSE circular 2/2023). That no Finance Act of 2020-2022 changed it was not searched directly; none surfaced. |
| STT-OPT-2 | stock_option, index_option | sell | 0.0625% (1/1600) | 0.0625% of the premium, seller (Finance Act 2023) | option premium value | 2023-04-01 | 2024-09-30 | verified | https://archives.nseindia.com/content/circulars/FATAX56235.pdf ; https://nsearchives.nseindia.com/content/circulars/FATAX63809.pdf | "0.0625 per cent of the option premium" | verbatim via search (Budget 2024-25 memorandum describing the rate in force). |
| STT-OPT-3 | stock_option, index_option | sell | 0.1% (1/1000) | 0.1% of the premium, seller (Finance (No. 2) Act 2024) | option premium value | 2024-10-01 | 2026-03-31 | verified | https://nsearchives.nseindia.com/content/circulars/FATAX63809.pdf | "The Finance Act, 2024 (No. 2 of 2024), which received the Presidential assent on August 16, 2024, has revised the rate of levy of STT with effect from 1st day of October, 2024." | verbatim via search; FATAX73524: "(upto March 31, 2026 – 0.10%)". |
| STT-OPT-4 | stock_option, index_option | sell | 0.15% (3/2000) | 0.15% of the premium, seller (Finance Act 2026) | option premium value | 2026-04-01 | 2026-10-31 | verified | https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf ; https://www.indiabudget.gov.in/doc/budget_speech.pdf | "The Finance Act, 2026, which received the Presidential assent on March 30, 2026, has revised the following rates of levy of STT with effect from 1st day of April, 2026" | verbatim via search (identical across four queries). Budget speech: "STT on options premium and exercise of options are both proposed to be raised to 0.15 percent from the present rate of 0.1 percent and 0.125 percent respectively". No later change found to 2026-10-31. |
| STTX-IDX-1 | index_option | exercise | 0.125% (1/800) | 0.125% of the SETTLEMENT PRICE, purchaser (holder who exercises) | settlement value: settlement price x quantity | 2018-09-01 | 2019-08-31 | verified | https://www.indiabudget.gov.in/budget2019-20/doc/memo.pdf ; https://www.nseindia.com/static/invest/first-time-investor-sebi-turnover-fees-stt-other-levies | "As per the existing provisions section 99 of the Finance (No.2) Act, 2004, the value of taxable securities transaction in respect of sale of an option in securities, where option is exercised, shall be, the settlement price." | verbatim via search (identical in two queries); NSE: "(prior to Septemebr 1, 2019 -on the settlement price)" (its typo). |
| STTX-IDX-2 | index_option | exercise | 0.125% (1/800) | 0.125% of the intrinsic value, purchaser | intrinsic value: \|settlement price - strike price\| x quantity | 2019-09-01 | 2026-03-31 | verified | https://www.indiabudget.gov.in/budget2019-20/doc/memo.pdf ; https://www.nseindia.com/static/invest/first-time-investor-sebi-turnover-fees-stt-other-levies | "It is proposed to amend the said section so as to provide that value of taxable securities transaction in respect of sale of an option in securities, where option is exercised, shall be the difference between the strike price and the settlement price. This amendment will take effect from 1st September, 2019." | verbatim via search (the Bill memorandum; NSE confirms "Purchaser - On Intrinsic value w.e.f. 01.09.2019"). |
| STTX-IDX-3 | index_option | exercise | 0.15% (3/2000) | 0.15% of the intrinsic value, purchaser (Finance Act 2026) | intrinsic value: \|settlement price - strike price\| x quantity | 2026-04-01 | 2026-10-31 | verified | https://www.nseindia.com/static/products-services/equity-derivatives-securities-transaction-tax ; https://nsearchives.nseindia.com/content/circulars/FATAX73524.pdf | "RATES W.E.F. APRIL 1, 2026 ... Sale of an option in securities, where option is exercised \| 0.15 per cent \| 0.125 per cent \| Purchaser" | table cells verbatim via search. CONFLICT: two of six reads of FATAX73524 said 'No Change' for this row; the NSE STT page, the budget speech, the Key Features document and the s.98 page all say 0.15%. |
| STTX-STK | stock_option | exercise | UNKNOWN | UNVERIFIED: the STT on a stock option settled by delivery was not reached | physically settled since SEBI moved stock derivatives to delivery | 2018-09-01 | 2026-10-31 | UNKNOWN | none reached | "none" | the search budget ran out before this was searched. The oracle prices no stock-option exercise. |

### 3.7 Stamp duty

From 2020-07-01 the Indian Stamp Act, as amended by the Finance Act 2019, levies
one duty through the exchange, on the buyer, credited to the State of the
buyer's residence: "Collecting agents shall within three weeks of the end of
each month transfer the stamp-duty collected to the State Government where the
residence of the buyer is located." (PIB, 30 Jun 2020, verbatim via search;
https://www.pib.gov.in/PressReleasePage.aspx?PRID=1635399.) The start date was
postponed twice: "9th January, 2020, which was later extended to 1st April,
2020 vide notifications dated 8th January, 2020" and then to "1st July, 2020
vide notifications dated 30th March, 2020" (same release).

**Before 2020-07-01, which state.** Stamp duty on a broker's note was the
State's, set by the client's state of residence. The state assumed is
**Maharashtra**: Dhan (Raise Securities) and NSE are in Mumbai, and Maharashtra
waived its duty for non-residents (row `STAMP-STATE`), so a Maharashtra
resident is the client its rate governs. Its rate for delivery, non-delivery,
futures and options, and any cap, **was not reached**; the oracle prices no
stamp duty before 2020-07-01 and flags every such day.

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| STAMP-STATE | equity_intraday, stock_option, index_option | buy | UNKNOWN | UNVERIFIED: state stamp duty. Maharashtra is the state assumed (the broker and NSE are in Mumbai, and Maharashtra waived its duty for non-residents, so a Maharashtra resident is the case its rate governs) — its rate was not reached | set by the client's state of residence | 2018-09-01 | 2020-06-30 | UNKNOWN | http://www.nseindia.com/regulations/member-compliance-stamp-duty ; https://www.indiacode.nic.in/bitstream/123456789/22026/1/the_maharashtra_stamp_act,_1958.pdf | "The Govt. of Maharashtra vide its Order No. Mudrank 2006/CR/290/M-1 dated May 23, 2006 had in public interest waived the stamp duty on the instruments of record of transaction relating to sale and purchase of securities by Trading Members and investors who are residing outside the State of Maharashtra." | verbatim via search. The Maharashtra Stamp Act text reached is 'as on 8th April 2025' and its rate sub-item was cut off; no delivery, non-delivery, futures or options rate, and no cap, was reached. |
| STAMP-EQ | equity_intraday | buy | 0.003% (3/100000) | 0.003%, buyer only | buy value (market value of the security bought) | 2020-07-01 | 2026-10-31 | verified | https://www.indiacode.nic.in/ViewFileUploaded?path=AC_CEN_2_2_00036_189902_1523339055436%2Frulesindividualfile%2F&file=stamp_act%2C_2019.pdf ; https://www.pib.gov.in/PressReleasePage.aspx?PRID=1635399 | "transfer of security other than debenture on non-delivery basis" | Schedule I entry verbatim via search; 0.003% is the tool's reading of the same Gazette PDF. Liability: "in the case of sale of security through stock exchange, by the buyer of such security". PIB: "... will come into effect from tomorrow, i.e. 1st July, 2020 vide notifications dated 30th March, 2020." |
| STAMP-OPT | stock_option, index_option | buy | 0.003% (3/100000) | 0.003% of the premium, buyer only | premium paid by the buyer | 2020-07-01 | 2026-10-31 | verified | https://www.indiacode.nic.in/bitstream/123456789/15510/5/A1899-2%20.pdf | "the market value for calculating the stamp-duty shall be, in the case of— (i) options in any securities, the premium paid by the buyer" | verbatim via search (s.21 proviso); Schedule entry "options (equity and commodity)"; 0.003% is the tool's reading. Index options read as 'options in any securities'. |

### 3.8 GST

| id | segments | applies | rate | value, as the source states it | base | from | to | status | source | exact quote (as the search tool returned it) | note |
|---|---|---|---|---|---|---|---|---|---|---|---|
| GST-18 | equity_intraday, stock_option, index_option | both sides | 18.0% (9/50) | 18%: 9% CGST + 9% SGST, or 18% IGST | brokerage + exchange transaction charge + IPFT + clearing (+ SEBI fee from 2022-07-18) | 2018-09-01 | 2026-10-31 | verified | https://gstcouncil.gov.in/sites/default/files/2024-02/gstservicerates.pdf ; https://cbic-gst.gov.in/hindi/pdf/central-tax-rate/Notification11-CGST.pdf | "15 Heading 9971 (vii) Financial and related services 9 9 18" | verbatim via search (Notification 11/2017-CT(Rate), from 2017-07-01). No change from the 22 Sep 2025 rate rationalisation surfaced; that broking stayed at 18% is not stated by any source reached. Which lines bear GST: Dhan's formula "18% on brokerage + transaction charges + SEBI Turnover + IPFT/Other Tax" (verbatim via search, dhan.co/pricing); no NSE or CBIC text was reached for the exchange-charge part. |
| GST-SEBI-0 | equity_intraday, stock_option, index_option | both sides | excluded | no GST on the SEBI fee: services by SEBI were exempt | flag: 0 = the SEBI fee is outside the GST base | 2018-09-01 | 2022-07-17 | assumption | https://archives.nseindia.com/content/circulars/FATAX52993.pdf | "the GST exemption granted to the services provided by SEBI is being withdrawn with effect from July 18, 2022." | verbatim via search (NSE circular 2/2022, 18 Jul 2022). That brokers levied none before is inferred from the exemption; no source says so directly. |
| GST-SEBI-1 | equity_intraday, stock_option, index_option | both sides | included | GST at 18% on the SEBI fee, collected from clients | flag: 1 = the SEBI fee is inside the GST base | 2022-07-18 | 2026-10-31 | verified | https://www.sebi.gov.in/sebi_data/faqfiles/mar-2023/1678853311186.pdf ; https://archives.nseindia.com/content/circulars/FATAX52993.pdf | "the fees and other charges payable to SEBI shall be subject to GST at the rate of 18% with effect from July 18, 2022." | verbatim via search. NSE: "the SEBI has advised the Exchange to direct the trading members to levy GST @ 18% on the fees payable to SEBI, from their clients". |

## 4. Context — rates outside the three segments

Read in the same research, kept here for anyone extending the oracle. They are
**not** in the fixture and the oracle does not price them.

| Charge | Segment | Value | From | To | Source | Status |
|---|---|---|---|---|---|---|
| STT, sale of futures (seller) | equity futures | 0.01% → 0.0125% → 0.02% → 0.05% | before 2018-09-01 / 2023-04-01 / 2024-10-01 / 2026-04-01 | 2026-10-31 | `FATAX56235`, `FATAX63809`, `FATAX73524`: "Sale of a futures in securities has been revised to 0.05% (upto March 31, 2026 – 0.02%)" (verbatim via search) | verified |
| STT, delivery (buyer and seller) | equity delivery | 0.1% each side | before 2018-09-01 | 2026-10-31 | NSE investor levies page; `FATAX73524` "No Change" | verified (paraphrase) |
| NSE transaction charge | equity futures | ₹1.90 /lakh (first slab) → ₹2.00 → ₹1.90 → ₹1.88 → ₹1.73 one rate → ₹182.99 /cr | as the cash rows | 2026-10-31 | `FA46730`, `FA56129`, `FA61137`; `FA64232` "Rs. 1.73 each side per lakh of traded value" (verbatim); `FA73061` | slab values paraphrase |
| IPFT | equity futures | ₹0.01 → ₹10 → ₹0.01 /cr | as the cash rows | 2026-10-31 | `FA56129`, `FA73061` | as the cash rows |
| Stamp duty (buyer) | futures | 0.002% | 2020-07-01 | 2026-10-31 | Indian Stamp Act Schedule I: "(d) derivatives—(i) futures (equity and commodity) 0.002%" (verbatim via search) | verified |
| Stamp duty (buyer) | equity delivery | 0.015% | 2020-07-01 | 2026-10-31 | Schedule I: "(b) transfer of security other than debenture on delivery basis; 0.015%" (verbatim via search) | verified |
| Dhan brokerage | equity delivery | ₹0 | launch | 2026-10-31 | dhan.co/pricing: "₹ 0 (Zero Brokerage)" (verbatim) | verified (current page) |
| Dhan brokerage | derivatives settled by physical delivery | 0.10% of the contract value + exchange charges | not dated | current page | dhan.co/pricing: "A brokerage fee of 0.10% of the contract value is applicable on all derivative contracts (futures and options) that result in physical delivery, along with the relevant Exchange charges." (verbatim) | verified (current page; date unknown) |
| Dhan brokerage | options expired, exercised or assigned | ₹20 per executed order | not dated | current page | dhan.co/pricing: "A brokerage charge of ₹20 per executed order is also applicable to Options Contracts that are expired, exercised, or assigned." (verbatim) | verified (current page) |
| NSE Nifty Next 50 F&O transaction charge | out of scope | nil | 2024-04-24 | 2024-10-31 | `FA61696` (paraphrase) | does not touch NIFTY, BANKNIFTY or stock options |

## 5. Rounding and aggregation

No official source reached fixes how a broker rounds each line of a contract
note, except for STT. Every step the oracle takes is listed with its source, and
those without one are `assumption`. The comparison in the report checks **rates
and bases unrounded first**, so a rounding choice can never be mistaken for a
rate mismatch.

| Step | Rule the oracle uses | Source | Status |
|---|---|---|---|
| Equity intraday STT: the price | the client's volume-weighted average price for the scrip and day, buys and sells together, "rounded off to the nearest paisa" (an exact half paisa up) | STT Rules 2004, rule 3 (verbatim fragment via search); NSE/CMTR/5459: "The value of taxable securities transaction shall be determined with respect to the trade executed under a particular trading client code." | verified (the half-paisa direction: assumption) |
| Equity intraday STT: the value | offset (squared-off) quantity × that price | rule 3; NSE method (paraphrase via search) | verified (paraphrase) |
| Options STT: the value | the day's sell premium value: "each futures trade is valued at the actual traded price and option trade is valued at premium" | NSE STT page (verbatim via search) | verified |
| STT: rounding | the value **and** the tax each to the nearest rupee, fifty paise and above up: "The value of taxable securities transaction and the amount of securities transaction tax, interest and penalty payable, and the amount of refund due, under the provisions of Chapter VII of the Act shall be rounded off to the nearest rupee"; NSE's copy: "if such part is fifty paise or more, it shall be increased to one rupee and if such part is less than fifty paise it shall be ignored" | STT Rules 2004, rule 4 (India Code, verbatim via search); NSE `cmtr5459` | verified (rule); the LEVEL — per client per security per day — is the oracle's reading |
| Stamp duty: rounding | to the nearest rupee, per client per security per day | no official text reached; the Act says only "on each such sale in the clearance list" (s.9A(1)(a)) | assumption |
| Exchange charge + IPFT + clearing | on the day's turnover in the security, one line, half-up to the paisa | none: NSE bills members monthly; how a broker allocates and rounds per client is its own | assumption |
| SEBI fee | on the day's turnover, half-up to the paisa | none | assumption |
| Brokerage | each executed order, half-up to the paisa; equity intraday is `min(₹20, 0.03% × order value)` | Dhan: "per executed order" (verbatim); its calculator says "for exact transaction related charges, please refer to your contract note." | assumption (rounding) |
| GST | 18% of the note's rounded base (brokerage + exchange line + SEBI fee when in the base), one line, half-up to the paisa | CGST Act s.170 rounds sums payable to the Government to the rupee, not note lines; NSE: decimals on a note are "the choice / arrangement between client/member" | assumption |
| CGST + SGST shown as two halves | presentation only; the oracle computes 18% once | IGST Act s.12(12): place of supply is the client's location on the broker's records (verbatim via search) | assumption |

## 6. Choices made, and what is still open

1. **Pre-2024-10-01 exchange charge** — the slab ladder's highest per-lakh rate
   is assumed to be what a client paid. A broker could have passed through less.
   The options "up to ₹3 crore: ₹2,500" row was ambiguous in the extraction.
2. **Stamp duty before 2020-07-01** — Maharashtra assumed; its rates were not
   reached. `UNKNOWN`: the oracle prices none and flags every such day.
3. **Dhan before 2021-11-04** — Dhan had not launched; its launch tariff is
   applied as a labelled `counterfactual`. That the tariff held unchanged from
   launch to today is an `assumption`: no dated snapshot of the pricing page
   could be reached (the archive was refused).
4. **Dhan options brokerage wording** — the knowledge base and the pricing page
   say a fixed ₹20 per order; `https://dhan.co/options/` says "₹20 (or 0.03%)".
   The fixed ₹20 is used. A Dhan contract note would settle it.
5. **SEBI fee, first day of ₹10** — 2019-04-01 is used; a non-official notice
   says 2019-04-03.
6. **SEBI fee, ₹5 from June 2020 to March 2021** — SEBI's press release says the
   cut is passed to investors; no exchange circular showing it on contract notes
   was read.
7. **GST on the SEBI fee before 2022-07-18** — taken as none, from SEBI's
   exemption being withdrawn on that date. No source says brokers levied none.
8. **IPFT at ₹0.01 per crore before 2023-04-01** — "existing" in March 2023; its
   start date was not read.
9. **STT on a stock option settled by delivery** — not reached (`STTX-STK`,
   `UNKNOWN`). Stamp duty, exchange charges and the SEBI fee on an exercise or
   a delivery were not researched beyond the SEBI fee's "notional value" rule.
10. **STT on exercise from 2026-04-01** — two of six reads of NSE `FATAX73524`
    said "No Change"; the NSE STT page, the budget speech, the Key Features
    document and the s.98 page all say 0.15%. 0.15% is used.
11. **GST after the 22 Sep 2025 rate rationalisation** — no change to broking
    surfaced, but no source states that it stayed at 18%.
12. **GST on the exchange charge and IPFT** — only Dhan's published formula says
    so; no NSE or CBIC text was reached. STT and stamp duty are outside the base
    on the same authority.
13. **Clearing charge nil** — inferred from NSE paying NSE Clearing; no circular
    stating nil was found.
14. **Nothing was fetched byte for byte.** A session whose network allows
    `nsearchives.nseindia.com`, `archives.nseindia.com`, `www.nseindia.com`,
    `www.sebi.gov.in`, `www.incometaxindia.gov.in`, `www.indiacode.nic.in`,
    `www.indiabudget.gov.in`, `dhan.co` and `web.archive.org` could re-read every
    row and settle items 1–13.
