# Charges oracle report — `crates/costs` against the official rates, 2018-09-01 to 2026-10-31

Branch `cloud/chg-oracle-20261110`, cut from `origin/feat/db-fix` at
`beaf2e3c4fe39a27f0f4c88e9444440726129f7c`. Written 10 Nov 2026 by an
independent cloud session. **No product code was changed**: this branch adds a
rate table, a test-only oracle and this report. Every finding below is an
`#[ignore]`d test named `finding_*` in `crates/costs/tests/charges_oracle.rs`;
run them with

```bash
cargo test -p costs --locked --test charges_oracle -- --ignored
```

and each one fails with the evidence quoted here in its message. Everything that
matches is a green test that runs by default.

## 1. The answer first

1. **No backtest applies any charge today.** The sweep prices every trade gross
   of brokerage, STT, stamp duty, exchange charges, the SEBI fee, the IPFT, GST
   **and** slippage: `crates/runner/src/trade.rs:59` ("What is lost is the
   slippage allowance ... it is gone"), `:64` and `:85` ("gross of charges AND
   gross of slippage"); `crates/cli/src/pool.rs:705` ("NO COST OF ANY KIND IS
   CHARGED"); `web/src/lib/charge-scope.js:70`. Nothing outside `crates/costs`
   calls its charge stack (`stack`, `ledger`, `dhan_calc`) or `trip::price`.
2. **The charge code that exists is right for today.** From **2026-04-01** every
   rate the crate holds equals the official rate, on every day, for all three
   segments and both rate selectors. The precise contract-note stack
   (`costs::stack`) reproduces the oracle **line by line, unrounded**, on every
   case of the grid from 2026-04-01: all 30 under today's rates, and the 20
   option cases under `RateSelector::Dated`, which refuses equity by design and
   which also matches on every day it prices from 2024-10-01. Dhan's ₹20 /
   0.03% brokerage matches at and around its cap.
3. **The rate set the crate names for backtests is wrong for every earlier
   date.** `RateSelector::CurrentOnEveryDate` — "the rate set backtests use, by
   the operator's choice" (`crates/costs/src/schedule.rs:11-16`, `:585-588`) —
   prices a 2019 trade at 2026 rates. It is labelled on every output, but the
   numbers are not the charges that were in force:
   * **Options are over-charged** before 2026-04-01, mainly because STT is taken
     at 0.15% where it was 0.05% (to 2023-03-31), 0.0625% (to 2024-09-30) and
     0.1% (to 2026-03-31). One NIFTY lot (75 units bought at ₹100 and sold at
     ₹120) is charged **₹5.00 to ₹6.19 too much per round trip** (+8% to +10%).
     On 1,800 lots the excess reached **₹45,101 on one day (+39%)**.
   * **Equity intraday is under-charged** before 2024-10-01, because the exchange
     charge was higher: **₹0.29 to ₹0.89 too little** on a ₹2 lakh round trip.
   * **An index option exercised before 2019-09-01** is charged STT on the
     intrinsic value where the law levied it on the full settlement value:
     **₹1,105 too little** on one NIFTY lot in the example below.
4. **The dated selector is never wrong where it prices**, but it refuses every
   equity day and every option day before 2024-10-01, although official (slab)
   rates exist for them; this session read them (`RATES.md` §3.2).
5. Smaller findings: a latent 0.0625% STT anchor before 2023-04-01; the
   round-trip pricer rounds every levy up, by up to ₹2.98 a trip; the Dhan
   calculator replica leaves the SEBI fee out of the GST base; and the SEBI fee
   on an exercised option's notional value is not priced. Details are in §5.

## 2. What was built

| File | What it is |
|---|---|
| `docs/charges-oracle/RATES.md` | The dated rate table: 42 rows, each with its value, unit, base, effective dates, official URL, the exact quote and its verification status; the timeline per segment; the rounding rules and their sources; the open questions. |
| `crates/costs/tests/fixtures/charges_oracle_rates.toml` | The same 42 rows, machine-readable (`.toml`, the one data format the extension allowlist admits here). The test refuses any key or value it does not understand, and requires `RATES.md` to carry every row, cell for cell. |
| `crates/costs/tests/charges_oracle.rs` | The oracle and the comparison. Test-only, Rust, no new dependency, no print, no product code touched. |
| `CHARGES-ORACLE-REPORT.md` | This report. |

**The oracle's API** (all in the test file):
* `order_charges(table, segment, date, order)` — one executed order (side,
  price, quantity, or `OrderIn::lots(side, lots, lot_size, price)`) on its own:
  brokerage, exchange charge, IPFT, clearing, SEBI fee, STT, stamp duty, GST,
  each **exactly** (rational paisa) and the total.
* `day_charges(table, segment, date, orders)` — one client's day in one
  security, the way a contract note bills it, exact **and** rounded.
* `exercise_stt(table, segment, date, exercise)` — STT on an index option
  exercised at expiry, on the base the law used that day.

**Segments:** equity cash intraday, stock options, index options. **Window:**
2018-09-01 to 2026-10-31, every calendar day covered by exactly one row per
charge, segment and side (a green test walks all 2,983 days).

## 3. How the check was done

* **Clean-room.** The rates were researched by four agents that never read this
  repository, from official domains only: NSE and NSE Clearing, SEBI, the
  Income-tax department, India Code, the Budget site, PIB, the GST Council and
  CBIC, and Dhan's own pages. The product's own citations
  (`docs/00-charter.md`, CHG-1) were compared only after the table existed.
* **No page was fetched.** This session's network policy refused every official
  host, and both requests to widen it were rejected. Every quote is the text a
  web-search tool returned from the named URL. Each row says whether that was
  verbatim or a paraphrase (`RATES.md` §1). The session's search budget, 200
  queries, ran out before every question was settled. Each unsettled point is
  an `UNKNOWN` row or an `assumption`, never an invented number (§7).
* **Rates first, rounding second.** The comparison runs on three layers so a
  rounding choice is never mistaken for a rate error:
  1. **Rate by rate, every calendar day**: 2,983 days × 3 segments × 2
     selectors × 7 components (exchange charge with IPFT and clearing, SEBI fee,
     GST rate, SEBI fee in the GST base, STT on sale, STT on exercise, stamp
     duty). The days are folded into runs, so every mismatch carries its exact
     date range. The regime's own lookups (`costs::regime`) are compared the
     same way.
  2. **Amounts, unrounded**, on a grid of **30 cases** (10 equity intraday, 10
     stock option, 10 index option) on **56 grid days**: every first and last
     day of every rate row and of every product rate change, the day either
     side of each, and both window ends. Cases: tiny (1 share at ₹1; 1 lot at
     ₹0.05), tick edges (₹99.95 / ₹100.05), odd quantities (13 shares; 7 lots of
     65; 3 lots of 35; a 1,800-share stock lot), the brokerage cap (₹33,300 × 2
     just under it; ₹66,650 at the half-paisa edge; ₹1 lakh over it), a
     short-first day, several orders a side, a VWAP half-paisa, STT half-paisa
     edges, and huge (₹50 crore a side; 1,800 lots).
  3. **Rounded lines**, only where rates and bases already agree, so what is
     left is rounding.
* **Reference trades** turn rate differences into rupees: equity — 100 shares
  bought at ₹1,000 and sold at ₹1,010; index option — 75 units (1 lot) at ₹100
  / ₹120; stock option — 500 units (1 lot) at ₹20 / ₹25.

## 4. What matches (green, runs by default)

| Test | What it proves |
|---|---|
| `the_crate_holds_the_official_rate_for_every_component_from_2026_04_01` | From 2026-04-01 to 2026-10-31, under both selectors, every card component equals the official rate for all three segments (the dated selector refuses equity by design and is not counted). The one exception is not a mismatch: no official figure was reached for the STT on a stock option settled by delivery. |
| `the_dated_selector_never_prices_a_rate_that_differs_from_the_official_one` | On all 2,983 days, `RateSelector::Dated` either prices the official rate or refuses. It never prices a wrong one. |
| `the_precise_stack_matches_the_oracle_line_by_line_unrounded_from_2026_04_01` | `stack::ClientSecurityDay` equals the oracle line by line, unrounded (within the crate's 10⁻⁷-paisa unit), on every grid case from 2026-04-01: all 30 under today's rates, and the 20 option cases under `Dated`. The lines checked are brokerage per order with its cap, exchange charge with IPFT, SEBI fee, the intraday STT on the VWAP non-delivery value, stamp duty on the buy value, and GST on the rounded base with the SEBI fee in it. |
| `the_dated_precise_stack_matches_the_oracle_unrounded_on_every_day_it_prices` | The same for options under `Dated` on every grid day it prices (from 2024-10-01); it refuses every grid day before. |
| `the_trip_pricers_dated_rates_match_the_oracle_wherever_it_prices` | `costs::regime`'s STT, exercise STT and exchange + IPFT lookups equal the official rates from 2024-10-01, across both the `FA73061` re-split on 2026-03-01 and the STT change on 2026-04-01. |
| `dhan_brokerage_matches_the_oracle_at_and_around_the_cap` | `schedule::dhan_brokerage` equals `min(₹20, 0.03%)` rounded half-up per order at ₹1, ₹99.95, ₹66,600, ₹66,649.95, ₹66,650 (19.995 → ₹20.00), ₹66,666.65, ₹66,666.67, ₹66,667, ₹10 lakh and ₹50 crore; options are ₹20 per order at any size. |
| the oracle's own tests | The fixture parses strictly and its parser refuses what it does not understand; every charge tiles the window; `RATES.md` carries every row exactly; hand-worked examples (below) reproduce; the per-order oracle sums to the one-order day. |

**Hand-worked examples the oracle reproduces** (from the official rates, by
hand, before the oracle ran):

| Day | Trade | Brokerage | Exchange + IPFT | SEBI | STT | Stamp | GST | Total |
|---|---|---|---|---|---|---|---|---|
| 2026-05-15 | index option, 75 at ₹100 → ₹120 | 40.00 | 5.86 | 0.02 | 14 (13.50 → up) | 0 (0.225) | 8.26 | **68.14** |
| 2026-05-15 | equity intraday, 50 at ₹1,520 → ₹1,530 | 40.00 | 4.68 | 0.15 | 19 (19.0625) | 2 (2.28) | 8.07 | **73.90** |
| 2022-01-10 | equity intraday, 100 at ₹1,000 → ₹1,010 | 40.00 | 6.93 (₹3.45 /lakh) | 0.20 | 25 | 3 | 8.45 (SEBI fee outside the base) | **83.58** |
| 2019-06-27 | NIFTY call 11,800 exercised at 11,850 × 75 | | | | **1,111** (0.125% × ₹8,88,750 settlement value) | | | |
| 2019-09-26 | the same exercise | | | | **5** (0.125% × ₹3,750 intrinsic) | | | |

The 2026-05-15 equity day is the crate's own vector V1 (₹73.90): the oracle
re-derives it from the official sources independently.

## 5. Every mismatch

Signs are **product minus official**. Per-crore figures are rupees per crore of
the value the charge is levied on: the premium for options and the traded value
for equity, on the side the charge falls. All ranges are inclusive.

### F-1 — Options STT at 0.15% on every date (today's rates on every date)

* **Where:** `crates/costs/src/schedule.rs:368` (`STT_OPTIONS`, 0.15%), applied
  to any date by `current_card` (`:926`) under `RateSelector::CurrentOnEveryDate`.
* **Official:** 0.05% to 2023-03-31 (`STT-OPT-1`), 0.0625% from 2023-04-01
  (`STT-OPT-2`, Finance Act 2023), 0.1% from 2024-10-01 (`STT-OPT-3`, Finance
  (No. 2) Act 2024), 0.15% from 2026-04-01 (`STT-OPT-4`, Finance Act 2026).
* **Size**, stock and index options alike:

| From | To | Product | Official | Per crore of sell premium |
|---|---|---|---|---|
| 2018-09-01 | 2023-03-31 | 0.15% | 0.05% | **+₹10,000** |
| 2023-04-01 | 2024-09-30 | 0.15% | 0.0625% | **+₹8,750** |
| 2024-10-01 | 2026-03-31 | 0.15% | 0.1% | **+₹5,000** |

* **Test:** `finding_current_rates_charge_todays_option_stt_on_every_earlier_date`.

### F-2 — Exchange charge at today's ₹307 / ₹3,553 per crore on every date

* **Where:** `crates/costs/src/schedule.rs:262` (`NSE_EQUITY_EXCHANGE`, ₹307 per
  crore) and `:283` (`NSE_OPTIONS_EXCHANGE`, ₹3,553 per crore of premium), via
  `current_card` (`:926`).
* **Official:** the first-slab transaction charge plus the IPFT of each period
  (`EXC-*`, `IPFT-*`). Before 2024-10-01 NSE billed members on a slab ladder;
  the highest per-lakh slab is taken as the client's rate, an `assumption`.
  From 2024-10-01 the official total equals the crate's, both before and after
  the `FA73061` re-split on 2026-03-01.
* **Size:**

| Segment | From | To | Product | Official | Per crore, each side |
|---|---|---|---|---|---|
| equity intraday | 2018-09-01 | 2020-12-31 | 0.00307% | 0.0032501% | **−₹18.01** |
| equity intraday | 2021-01-01 | 2023-03-31 | 0.00307% | 0.0034501% | **−₹38.01** |
| equity intraday | 2023-04-01 | 2024-03-31 | 0.00307% | 0.00335% | **−₹28.00** |
| equity intraday | 2024-04-01 | 2024-09-30 | 0.00307% | 0.00332% | **−₹25.00** |
| stock and index options | 2018-09-01 | 2020-12-31 | 0.03553% | 0.0500001% | **−₹1,447.01** |
| stock and index options | 2021-01-01 | 2023-03-31 | 0.03553% | 0.0530001% | **−₹1,747.01** |
| stock and index options | 2023-04-01 | 2024-03-31 | 0.03553% | 0.0505% | **−₹1,497.00** |
| stock and index options | 2024-04-01 | 2024-09-30 | 0.03553% | 0.05% | **−₹1,447.00** |

* **Test:** `finding_current_rates_charge_todays_exchange_charge_on_every_earlier_date`.

### F-3 — SEBI fee at ₹10 per crore on every date

* **Where:** `crates/costs/src/schedule.rs:311` (`SEBI_FEE`), on every date under
  `CurrentOnEveryDate`.
* **Official:** ₹15 per crore to 2019-03-31 (`SEBI-15`); ₹10 from 2019-04-01
  (`SEBI-10A`, first day an `assumption`); **₹5 from 2020-06-01 to 2021-03-31**
  (`SEBI-5`, SEBI PR 24/2020: "reduced to 50% of the existing fee structure for
  the period June 2020 to March 2021"); ₹10 from 2021-04-01.
* **Size:** **−₹5.00** per crore each side, 2018-09-01 to 2019-03-31; **+₹5.00**
  per crore each side, 2020-06-01 to 2021-03-31; all three segments.
* **Test:** `finding_current_rates_charge_rs_10_sebi_fee_where_it_was_15_or_5`.

### F-4 — GST on the SEBI fee before 2022-07-18

* **Where:** `crates/costs/src/stack.rs:599` puts the SEBI fee in the GST base
  on every date. The crate's own source line knows the date ("on the SEBI fee
  from 2022-07-18 per NSE/FATAX/52993", `schedule.rs:321`), but nothing dates
  it.
* **Official:** SEBI's services were GST-exempt until the exemption was
  withdrawn "with effect from July 18, 2022" (`GST-SEBI-0`, `GST-SEBI-1`, NSE
  `FATAX52993`). That brokers charged none before is inferred from the
  exemption (an `assumption`).
* **Size:** 18% of the SEBI fee — **+₹2.70** per crore (to 2019-03-31),
  **+₹1.80** (2019-04-01 to 2020-05-31 and 2021-04-01 to 2022-07-17), **+₹0.90**
  (2020-06-01 to 2021-03-31); all three segments. Paise on any ordinary trade.
* **Test:** `finding_current_rates_charge_gst_on_the_sebi_fee_before_2022_07_18`.

### F-5 — STT on exercise: today's 0.15% on every date, and the wrong base before 2019-09-01

* **Where:** `crates/costs/src/schedule.rs:377` (`STT_EXERCISE`, 0.15% of the
  intrinsic value) under `CurrentOnEveryDate`; `stack::ClientSecurityDay::exercise`
  (`crates/costs/src/stack.rs:520`) takes the intrinsic value as its base on
  every date.
* **Official:** 0.125% of the **settlement price** (× quantity) to 2019-08-31
  (`STTX-IDX-1`: "the value of taxable securities transaction in respect of
  sale of an option in securities, where option is exercised, shall be, the
  settlement price"); 0.125% of the **intrinsic value** from 2019-09-01
  (`STTX-IDX-2`, Finance (No. 2) Act 2019); 0.15% of the intrinsic value from
  2026-04-01 (`STTX-IDX-3`).
* **Size:** **+₹2,500** per crore of intrinsic value, 2019-09-01 to 2026-03-31.
  Before 2019-09-01 the base itself differs. On a NIFTY call, strike 11,800,
  settled at 11,850, 75 units: official STT **₹1,111**, product **₹6**, so
  **−₹1,105** on 2018-09-27 and 2019-08-29; **+₹1** on 2019-09-26 and
  2026-03-26. `Dated` refuses each of those days (before 2024-10-01), except
  2026-03-26, where it matches. Stock options: the crate charges 0.15% of
  intrinsic value, but no official figure for a delivery-settled stock option
  was reached (`STTX-STK`, `UNKNOWN`), so that cannot be checked.
* **Tests:** `finding_current_rates_charge_todays_exercise_stt_on_every_earlier_date`,
  `finding_exercise_stt_before_2019_09_01_was_on_the_settlement_value`.

### F-6 — Stamp duty before 2020-07-01: a uniform 0.003% where the duty was each State's

* **Where:** `crates/costs/src/schedule.rs:387` and `:416` (`STAMP_*`, 0.003% of
  the buy value) on every date under `CurrentOnEveryDate`.
* **Official:** before 2020-07-01 the duty was the State's, set by the client's
  residence. Maharashtra is assumed: the broker and NSE are in Mumbai, and
  Maharashtra waived its duty for non-residents. Its rates were **not reached**
  (`STAMP-STATE`, `UNKNOWN`).
* **Size:** unknown in both direction and size. The product charges ₹3.00 on
  the equity reference day, and ₹0 after rounding on the two option reference
  days, in a window where the right figure is not known. The oracle leaves
  stamp duty out of both sides of every comparison before 2020-07-01.
* **Test:** `finding_current_rates_charge_uniform_stamp_duty_before_2020_07_01`.

### F-1 to F-6 together: what today's rates on every date cost, period by period

`finding_current_rates_reference_trades_period_by_period`, the rounded totals of
the three reference trades:

| From | To | Equity, 100 at ₹1,000 → ₹1,010 | Index option, 75 at ₹100 → ₹120 | Stock option, 500 at ₹20 → ₹25 |
|---|---|---|---|---|
| 2018-09-01 | 2019-03-31 | official ₹80.21, product ₹79.72: **−₹0.49** † | official ₹61.96, product ₹68.14: **+₹6.18** † | official ₹66.51, product ₹75.65: **+₹9.14** † |
| 2019-04-01 | 2020-05-31 | ₹80.11 vs ₹79.72: **−₹0.39** † | ₹61.96 vs ₹68.14: **+₹6.18** † | ₹66.50 vs ₹75.65: **+₹9.15** † |
| 2020-06-01 | 2020-06-30 | ₹80.01 vs ₹79.72: **−₹0.29** † | ₹61.95 vs ₹68.14: **+₹6.19** † | ₹66.49 vs ₹75.65: **+₹9.16** † |
| 2020-07-01 | 2020-12-31 | ₹83.01 vs ₹82.72: **−₹0.29** | ₹61.95 vs ₹68.14: **+₹6.19** | ₹66.49 vs ₹75.65: **+₹9.16** |
| 2021-01-01 | 2021-03-31 | ₹83.48 vs ₹82.72: **−₹0.76** | ₹62.54 vs ₹68.14: **+₹5.60** | ₹67.29 vs ₹75.65: **+₹8.36** |
| 2021-04-01 | 2022-07-17 | ₹83.58 vs ₹82.72: **−₹0.86** | ₹62.55 vs ₹68.14: **+₹5.59** | ₹67.30 vs ₹75.65: **+₹8.35** |
| 2022-07-18 | 2023-03-31 | ₹83.61 vs ₹82.72: **−₹0.89** | ₹62.55 vs ₹68.14: **+₹5.59** | ₹67.30 vs ₹75.65: **+₹8.35** |
| 2023-04-01 | 2024-03-31 | ₹83.38 vs ₹82.72: **−₹0.66** | ₹63.05 vs ₹68.14: **+₹5.09** | ₹68.63 vs ₹75.65: **+₹7.02** |
| 2024-04-01 | 2024-09-30 | ₹83.31 vs ₹82.72: **−₹0.59** | ₹62.96 vs ₹68.14: **+₹5.18** | ₹68.50 vs ₹75.65: **+₹7.15** |
| 2024-10-01 | 2026-03-31 | ₹82.72 = ₹82.72 | ₹63.14 vs ₹68.14: **+₹5.00** | ₹69.65 vs ₹75.65: **+₹6.00** |
| 2026-04-01 | 2026-10-31 | ₹82.72 = ₹82.72 | ₹68.14 = ₹68.14 | ₹75.65 = ₹75.65 |

† Stamp duty left out of both sides: no official figure before 2020-07-01 (F-6).

On large trades the gap grows with the premium. In
`finding_current_rates_grid_amounts_unrounded`, 1,800 lots of 75 bought at ₹500
and sold at ₹505 (₹13.57 crore of premium) cost **₹1,61,377.08** under today's
rates against **₹1,16,275.95** official on 2020-12-31: **+₹45,101.14 (+39%)**.
On 2023-03-31 the STT alone is +₹68,175.00, partly offset by an exchange charge
₹23,702.56 lower and GST ₹4,266.46 lower. For equity, ₹50 crore a side on
2023-03-31 is **₹4,485.20 under-charged**.

### F-7 — The dated selector refuses dates for which official rates exist

* **Where:** `crates/costs/src/schedule.rs:1000-1011` (`NO_DATED_HISTORY`,
  `dated_card`): every equity day is refused. `crates/costs/src/regime.rs:457-470`
  (`NSE_EXCHANGE_CHARGE_TABLE` anchor, unverified) refuses every option day
  before 2024-10-01, and so do `trip::Rates::resolve` and the dated card, which
  read it.
* **Official:** the slab-era circulars `FA46730` (from 2021-01-01), `FA56129`
  (from 2023-04-01) and `FA61137` (from 2024-04-01), and the 2009 ladder before
  them, give the rates in `RATES.md` §3.2. Their slab values were read through
  the search tool's paraphrase, and the pass-through slab is an `assumption`.
* **Size:** not a wrong number, a missing one. Equity is refused on all 2,983
  days; options on 2018-09-01 to 2024-09-30.
* **Tests:** `finding_dated_selector_refuses_dates_with_official_rates`,
  `finding_regime_lookups_differ_from_or_refuse_official_rates`.

### F-8 — `regime::stt_options_rate` returns 0.0625% before 2023-04-01 (latent)

* **Where:** `crates/costs/src/regime.rs:353-365`: the `STT_OPTIONS` anchor row is
  0.0625% (`6_250`, `:358`) from `TradeDay::MIN` to 2024-09-30. Its note says
  the 0.05% → 0.0625% change of 2023-04-01 is "UNVERIFIED training knowledge
  with no retrieved circular" (`:360-365`).
* **Official:** 0.05% to 2023-03-31: NSE `FATAX56235`, "(up to March 31, 2023 –
  0.05%)" (`STT-OPT-1`).
* **Size:** **+₹1,250** per crore of sell premium, 2018-09-01 to 2023-03-31.
  It is **latent**: every caller that prices a trip hits the exchange-charge
  refusal (F-7) first on those dates. It becomes live the day F-7 is closed.
* **Test:** `finding_regime_lookups_differ_from_or_refuse_official_rates`.

### F-9 — The documented exercise base is wrong before 2019-09-01

* **Where:** `crates/costs/src/regime.rs:392` ("0.125% on intrinsic value before
  2026-04-01"); `crates/costs/src/trip.rs:129-130` (`Outcome::ExpiryExercise`:
  "STT would be charged on intrinsic value").
* **Official:** the settlement value until 2019-08-31 (F-5).
* **Size:** as F-5 when the date is before 2019-09-01. Both places are
  documentation today; no priced path reaches a pre-2024-10-01 exercise under
  `Dated`.
* **Test:** `finding_regime_lookups_differ_from_or_refuse_official_rates`.

### F-10 — The SEBI fee on an exercised option's notional value is not priced

* **Where:** `crates/costs/src/stack.rs:358-384` (`Totals::exercise`): the
  exercised value enters no turnover, so the SEBI line on an exercise day is
  ₹0. `docs/00-charter.md` (CHG-1, row `F-02`) already records this as "not
  priced".
* **Official:** "in case where the option is exercised or assigned, it shall be
  additionally computed on the basis of notional value of option contracts
  exercised or assigned" (SEBI Stock Brokers Regulations schedule, `RATES.md`
  §3.5).
* **Size:** ₹10 per crore of notional value, plus 18% GST. "Notional value" is
  not defined in what was reached. On the settlement value (65 units settled at
  ₹25,000) it is **₹1.625 + GST** per exercise.
* **Test:** `finding_sebi_fee_on_exercised_notional_value_is_not_priced`.

### F-11 — The round-trip pricer rounds every levy up, GST to the whole rupee

* **Where:** `crates/costs/src/trip.rs:1102` (STT), `:1122` (stamp duty) and
  `:1139` (GST) use `money::statutory_levy`: floor to the paisa, then **ceil to
  the rupee**. `:1106` onward ceil the exchange charge, the SEBI fee and the
  IPFT to the paisa on each leg. `crates/costs/src/money.rs:1-30` documents this
  as deliberate pessimism.
* **Official:** STT is rounded to the **nearest** rupee (STT Rules rule 4).
  Stamp duty has no official rounding rule reached. GST has no official rule
  that rounds a note line to the rupee: CGST Act s.170 rounds sums payable to
  the Government.
* **Size:** from 2024-10-01 on the grid, with the trip's own fills (one tick
  adverse each leg): STT **+₹1**, stamp duty **+₹1**, GST **+₹0.42 to +₹0.96**,
  exchange and SEBI **+₹0.01 to +₹0.03**. Totals **+₹1.48 to +₹2.98 per round
  trip** (1 NIFTY lot: ₹69.89 against ₹67.14 from 2026-04-01). The trip pricer
  is not called by any backtest today (§1).
* **Test:** `finding_trip_pricer_rounds_levies_upward`.

### F-12 — The Dhan calculator replica differs from the official method

`crates/costs/src/dhan_calc.rs` reproduces Dhan's public calculator page and
labels every output an estimate (`:1-30`). Against the official contract-note
method on 2026-10-30:

* **GST leaves the SEBI fee out** (`dhan_calc.rs:19`, `:299-301`). That
  contradicts SEBI ("the fees and other charges payable to SEBI shall be subject
  to GST at the rate of 18% with effect from July 18, 2022") and Dhan's own
  pricing page ("18% on brokerage + transaction charges + SEBI Turnover +
  IPFT/Other Tax"). Size: −18% of the SEBI fee, **−₹180.00** on a ₹50 crore
  equity day each side, **−₹24.42** on 1,800 option lots, a few paise on small
  trades.
* **Intraday STT on the sell value** (`dhan_calc.rs:16`), not on the offset
  quantity at the day's VWAP (STT Rules rule 3, NSE).
* **The total adds STT and stamp duty unrounded** (`dhan_calc.rs:20`), and sums
  brokerage before rounding, where a note rounds STT and stamp to the rupee and
  brokerage per order.
* **Totals:** **−₹179.75** on the ₹50 crore equity day, **−₹24.93** on 1,800
  lots, **−₹0.28** on 1 NIFTY lot (₹67.86 against ₹68.14), **−₹0.37 to +₹0.64**
  on the small cases; brokerage ±₹0.01.
* **Test:** `finding_dhan_calculator_replica_differs_from_the_official_method`.

### F-13 — Open question: STT and stamp duty rounded to the paisa before the rupee

* **Where:** `crates/costs/src/stack.rs:933` rounds each tax half-up to the
  paisa; `:903` then rounds to the rupee.
* **Official reached:** STT Rules rule 4 rounds the value and the tax "to the
  nearest rupee". It states no paisa step. The crate cites `NSE/CMTR/5588`
  for a per-security two-decimal step, a circular this session did not reach.
* **Size:** **₹1** when the exact tax falls in the half-paisa just below 50
  paise. Seen on the grid as one lot of 75 sold at ₹4.40: 0.15% × ₹330 =
  ₹0.495, which is ₹1 in the crate and ₹0 by rule 4 alone. Also at ₹6.60 in
  the 0.1% period. Who is right depends on `NSE/CMTR/5588`'s text. Reading it
  would settle this.
* **Test:** `finding_precise_stack_rounds_stt_and_stamp_to_the_paisa_before_the_rupee`.

## 6. Where the product's own citations stand against this research

| Product statement | This research |
|---|---|
| NSE `FA64232` (₹2.97 / ₹1.73 / ₹35.03 per lakh, from 2024-10-01) and `FA73061` (₹306.99 + ₹0.01, ₹182.99 + ₹0.01, ₹3,552.99 + ₹0.01 per crore, from 2026-03-01) | **Agrees.** The total outflow is unchanged across 2026-03-01. |
| IPFT ₹50 per crore of premium 2023-04-01 to 2026-02-28 (`FA56129`); ₹0.01 before, start unknown | **Agrees**, and the start date is unknown here too. |
| STT 0.025% / 0.15% / 0.15% exercise from 2026-04-01 (`FATAX73524`) | **Agrees**; 0.15% on exercise despite two conflicting reads (`RATES.md` §6.10). |
| SEBI ₹10 per crore, in force 2019-04-01 (`FA40632`) | **Agrees**, and adds the ₹15 before it and the ₹5 of June 2020 to March 2021. |
| GST on the SEBI fee from 2022-07-18 (`FATAX52993`) | **Agrees.** The stack does not date it (F-4). |
| Stamp duty 0.003% buy side from 2020-07-01; 0.015% delivery, 0.002% futures | **Agrees.** |
| Dhan: ₹20 or 0.03% intraday, ₹20 per F&O order, ₹20 on an exercised or expired option, 0.10% on physical delivery, ₹20 + GST auto square-off | **Agrees** with the current pages. Dhan's options page also says "₹20 (or 0.03%)" (open question). |
| `regime.rs:360-365`: the 2023-04-01 change to 0.0625% is "UNVERIFIED training knowledge" | Now sourced: NSE `FATAX56235` (F-8). |
| `regime.rs:463-470`: `FA46730`, `FA56129`, `FA61137` identified, none retrieved | Their slab values were read through search (paraphrase), with effective dates 2021-01-01, 2023-04-01 and 2024-04-01 (F-7). |
| `regime.rs:392`, `trip.rs:129-130`: exercise STT on intrinsic value | True from 2019-09-01 only (F-9). |

## 7. Open questions

1. **Stamp duty before 2020-07-01**: the Maharashtra rates (delivery,
   non-delivery, futures, options, any cap) were not reached. The client's
   state of residence decides.
2. **The slab era before 2024-10-01**: which slab a broker passed to clients,
   and the ambiguous options "up to ₹3 crore: ₹2,500" row.
3. **SEBI fee**: the first day of ₹10 (2019-04-01 or 2019-04-03), and whether the
   ₹5 of June 2020 to March 2021 reached contract notes.
4. **STT on a stock option settled by delivery**; exchange charges, stamp duty
   and the SEBI fee's "notional value" on an exercise or a delivery.
5. **STT on exercise from 2026-04-01**: two of six reads of `FATAX73524` said "No
   Change"; four other official sources say 0.15%.
6. **The rounding level of every line except STT**, and the paisa step before
   the rupee in STT and stamp duty (F-13, `NSE/CMTR/5588`).
7. **Dhan**: its tariff at launch (November 2021) and any change since; the
   "₹20 (or 0.03%)" wording on its options page; whether a part-filled order is
   one executed order.
8. **GST**: no source states that broking stayed at 18% after the 22 Sep 2025
   rate rationalisation, and none but Dhan's page puts exchange charges and IPFT
   in the base.
9. **Nothing was fetched byte for byte.** A session whose network allows
   `nsearchives.nseindia.com`, `archives.nseindia.com`, `www.nseindia.com`,
   `www.sebi.gov.in`, `www.incometaxindia.gov.in`, `www.indiacode.nic.in`,
   `www.indiabudget.gov.in`, `dhan.co` and `web.archive.org` could re-read every
   row and settle most of the above. So could more search budget: the
   session's 200 queries ran out.

## 8. What was run, and what was not

* `cargo test -p costs --locked`: green. 292 unit tests, every integration
  file, and 55 doc tests. `charges_oracle`: **13 passed, 15 ignored**; the 15
  ignored are the findings above and fail when run, by design.
* `cargo clippy -p costs --locked --all-targets -- -D warnings`: clean.
  `cargo fmt --all -- --check`: clean.
* The repository's own gate tools, built from `.github/*.rs` and run against
  the tree with the three new files staged: **gate 1** (extension allowlist; the
  new `.rs` is compiled) and **gate 1b**: OK. **Gate 15** (the forbidden word):
  OK. Gate 12's trigger words in the new doc comments and gate 23's print
  macros were checked by hand: none.
* **Not run:** `cargo test --workspace` (the repository records it as running
  for over an hour), coverage, mutation testing and `cargo deny`. This change
  adds no dependency and touches no product module.
* **Not done, on purpose:** no product code, no `docs/04-invariants.md` or
  `docs/05-decisions.md` entry, and no change to `CLAUDE.md`'s document table.
  Those belong to the review lane that takes these findings up.
