# Numeric pass 13 (num13): cost rates against their recorded sources

**Verdict at 1f4de71: 5 new findings, all low (p13num-1 to p13num-5). None is high or medium, so no cargo was run.** This was an audit by reading source in /home/claude/wt/zero3. The web was not consulted: every value was compared with `docs/00-charter.md` only.

**The headline is already filed.** `docs/00-charter.md` records **no source for any cost rate, cap, slab or regime date** (grep stt/ctt/stamp/gst/sebi/ipft/brokerage/charge). Its only cost row is :175 *"Track-1 brokerage | none. Spot indices are not tradable; the sweep is signal-only. Cost modelling belongs to the options translation layer."* So every rate below is UNVERIFIED under CLAUDE.md §3 rule 1. That is audit-20261003 **hunt-costs-5**, held as UNVERIFIED in docs/06-limits.md:14119, D-1535 (05-decisions.md:53723) and 11-findings.md:690. It is not re-filed here. Lot sizes, strike steps and expiry weekdays are P9-02, the Groww cap is P9-03 and the GST-choice mutant is P10-01. None of those is re-reported.

Every `Rate::Verified` row below cites the predecessor's `COSTS_VERIFIED` / `DEC-COST-*` documents, and no charter entry. **No production path multiplies any of these rates.** `costs::trip::price` / `charge_stack` / `Rates::resolve` have zero callers outside `crates/costs` (grep). runner trade.rs:64-80 states that nothing is charged. The live cost-crate reach is `fill`, `venue`, `strike` and `expiry` only (pull pricing.rs:748, rolling.rs:113-160).

## Table: charge by charge

| charge | code value (file:line) | charter entry | value matches charter? | dates in code | rounding in code | verdict |
|---|---|---|---|---|---|---|
| STT, options sell premium | 6,250 anchor (regime.rs:325), 10,000 from 2024-10-01 (:335), 15,000 from 2026-04-01 (:341) | none | no source | inclusive `start <= day` (regime.rs:177). The sell leg's day (trip.rs:1059, D-1535) | `statutory_levy` on the sell notional, **once per trip**: floor to the paisa, then ceil to the rupee (money.rs:247) | UNVERIFIED (hunt-costs-5). The :341 row cites a *Bill* and Wikipedia: see p13num-4 |
| STT on exercise | 12,500 anchor (:358), 15,000 from 2026-04-01 (:364) | none | no source | inclusive | n/a: no caller, and `price` refuses every outcome except a normal close | UNVERIFIED, inert |
| CTT | not encoded | none | n/a | n/a | n/a | correct as absent: no MCX or commodity segment exists |
| exchange txn charge | NSE 3,503 and BSE 3,250 from 2024-10-01 (regime.rs:412, :441). The anchor is `Unverified` (:399-407, :427-436) | none | no source | inclusive. Before 2024-10-01 the whole trip refuses, on **either** leg (trip.rs:1232-1233) | `levy_ceiling`: ceil to the paisa **per leg**, then summed (trip.rs:980-992) | UNVERIFIED. The BSE row admits its notice number "was not independently reconfirmable" (:442-446). BSE is never swept (venue::swept_slot) |
| SEBI fee | 10 (rate.rs:149) | none | no source | undated | ceil to the paisa per leg | UNVERIFIED |
| stamp duty | 300, buy side only (rate.rs:210) | none | no source | undated in code. The prose says "since 2020-07-01" (rate.rs:206), but no row carries that date. This is inert: every pre-2024-10-01 trip refuses on the exchange charge | `statutory_levy` on the buy notional, at the buy leg's day, once | UNVERIFIED |
| GST | 1,800,000 (rate.rs:242) | none | no source | undated. A straddling trip takes the larger of the two legs' rates (trip.rs:1085-1089, P10-01) | levied on the **rounded** brokerage + exchange + SEBI + IPFT. STT and stamp are excluded (trip.rs:1094-1101). `statutory_levy` once, with no CGST/SGST split | UNVERIFIED |
| IPFT | NSE 50 (rate.rs:163). BSE 0, marked UNVERIFIED (:179) | none | no source, and its stated source contradicts the crate: **p13num-1** | undated | ceil to the paisa per leg | UNVERIFIED |
| brokerage | Rs 20 per order for each broker (rate.rs:120, :129), x2 per trip (trip.rs:1060) | :175 says Track-1 brokerage "none" (signal-only). That is consistent with `is_cost_free(IndexSpot)` | no options/F&O source | undated. "since June 2024" is prose only (rate.rs:122), and inert as above | flat paisa | UNVERIFIED |
| DP charges | **not encoded anywhere** (grep "DP charge" / "depository" = 0 in crates and charter) | none | n/a | n/a | n/a | correctly absent for options. The equity labels do not name it: p13num-3 |
| option tick Rs 0.05 | rate.rs:252 | :173 says only "Tick grid 2 decimal places" | no | — | — | UNVERIFIED (inside hunt-costs-5) |
| greeks | no charge, lot or contract fact. Only numeric bounds (bsm.rs:66-101, solver.rs:138-192). The risk-free rate is supplied by the caller and refused without a source (pull pricing.rs:493) | charter §4b | consistent | — | — | clean |

**Regime boundaries.** At 1f4de71 a straddling trip prices each charge as follows:
- STT at the sell leg's day.
- Stamp duty at the buy leg's day.
- Exchange, SEBI and IPFT at each leg's own day.
- GST at the max of the two legs' rates.
- Brokerage from the buy set; it is flat.
- If either leg falls in an Unverified window, the whole trip refuses (trip.rs:1232-1239).

Lot size is the entry day's; this is a stated limit (§27). The verified-from defect is p6num-2 (still NOT FIXED). The boundary is inclusive on the new rate in every table (regime.rs:177, dated.rs:142).

**Equity is gross everywhere checked, and labelled.**
- runner audit.rs:148 `CASH_EQUITY_GROSS`
- cli pool.rs:665 `EQUITY_TOTALS_GROSS`
- cli lib.rs:19198 `EQUITY_RANKING_GROSS`
- cli lib.rs:19212 `SHARE_MEAN_LEGEND`, used by `render_top_record` (lib.rs:8259-8266), and through it by api topjson.rs
- web charge-scope.js:70-72
- `costs::scope::Segment` has no equity variant (scope.rs:51-58), and `is_cost_free` is an exhaustive match (:113-140)

**No equity result can reach Selection V6.** The guard is in the types:
- runner exit_grid_policy.rs:1253-1264 `InstrumentFamilyV1::of` refuses anything that is not `Exchange::Nse` + `Segment::Index` + `Kind::Index`, with underlying NIFTY or BANKNIFTY.
- V6 reads only `ExecutionV4Family::{Nifty, BankNifty}` (selection_v6_source.rs:66-68, :122-124).
- `public_winner` can only name those two (selection_v6.rs:165-175).

## New findings

### p13num-1 (low, provenance contradiction): `NSE_IPFT` cites `NSE/FA/56129`, a circular the same crate records as never retrieved and as superseded before any day on which IPFT is ever charged
- crates/costs/src/rate.rs:155-163:
  ```
  /// NSE IPFT: 0.0005%, ₹50 per crore, on premium both sides.
  ///
  /// Source: `NSE/FA/56129`; `COSTS_VERIFIED` §5.
  pub const NSE_IPFT: BpsX100 = BpsX100::new(50);
  ```
- crates/costs/src/regime.rs:401-404 (the NSE anchor's refusal text): *"Three superseded circulars were IDENTIFIED and NONE was officially retrieved: `NSE/FA/46730` (roughly 2021-01 to 2023-03), `FA56129` (2023-04 to 2024-03) and `FA61137` ..."*. docs/06-limits.md:1136-1140 says the same: *"none of the five was ever officially retrieved"*.
- **Why it is wrong.**
  - The one figure in the crate that cites FA56129 as its source is priced as fact. The regime table refuses every rate that rests on that very circular.
  - The crate dates FA56129's window as 2023-04 to 2024-03. IPFT is undated, but `price` can only reach it on days from 2024-10-01 on, because the exchange charge refuses everything earlier. So **every day on which the 50 is actually multiplied falls after the cited circular was superseded**: by FA61137, and then by FA64232, which the crate itself says re-set the charges.
  - The crate also records that FA73061 reportedly moves IPFT to ₹1/crore from 2026-03-01 (rate.rs:159-162, regime.rs:377-381). So the cited source covers no in-force day.
  - There is no charter row either (hunt-costs-5). This is a second, internal defect: the citation given is one the crate disowns.
- **Repro.** Not run (text check): `grep -n 56129 crates/costs/src/rate.rs crates/costs/src/regime.rs`.
- **Fix.**
  - Re-cite NSE_IPFT to the circular in force from 2024-10-01 (FA64232, if it states IPFT). Otherwise mark it UNVERIFIED in its doc and in 06-limits §25 beside the BSE zero.
  - Move it into a dated `RegimeTable` so that the 2026-03-01 split, once sourced, is one row.

### p13num-2 (low, doc-false in the charter): the charter says the NIFTY/BANKNIFTY strike interval is "Assumed nowhere in code", but production code assumes it
- docs/00-charter.md:494: `| **NIFTY and BANKNIFTY strike intervals** | nowhere — \`Moneyness::from_ladder\` takes the interval as an argument | No source states them. | **UNVERIFIED.** Assumed nowhere in code |`
- **Why it is wrong.** `costs::strike::strike_step_on` encodes 50 and 100 (strike.rs:118-121, :139-187). pull pricing.rs:748 calls it on the production path:
  ```
  let step = costs::strike::strike_step_on(quote.slot, quote.on).map_err(|why| {
  ```
  The caller is api server.rs:11669-12196 (`pull::pricing::slot_of`, `price`), so the API's moneyness and greeks figures rest on that interval.
  - P9-02 said the charter has "no strike row". It has one, and that row positively asserts the opposite of the code.
  - Under CLAUDE.md §10, a reader is told the charter is authoritative over verified facts, so this row tells them there is nothing to audit.
- **Repro.** Not run (text check): `grep -n "Assumed nowhere" docs/00-charter.md`. `grep -n strike_step_on crates/pull/src/pricing.rs` gives :748 outside `#[cfg(test)]` (:910).
- **Fix.** Rewrite the row: "Where it lives: `costs::strike::strike_step_on` (dated 50/100 rows citing `TRACK2_OPTIONS_SPEC` §5), used by `pull::pricing::price`. UNVERIFIED, encoded." Add a D-entry.

### p13num-3 (low, rule 1): every equity gross label names the charges a share trade pays, a cost claim with no charter source, and the list omits IPFT and DP charges
- runner audit.rs:148-153: `"CASH EQUITY. EVERY TOTAL BELOW IS GROSS OF EVERY CHARGE: brokerage,\n STT, stamp duty, exchange charges, the SEBI fee and GST apply to a\n share trade and none is subtracted. ..."`
- The same list appears in pool.rs:665-670, lib.rs:19198-19203, lib.rs:19212-19216 and web charge-scope.js:70-72. They are held identical by `sweep_wiring_tests::every_equity_charge_statement_is_the_audit_headers_own_and_names_no_rate`.
- **Why it is wrong.**
  - D-0696 and the doc at audit.rs:146-147 say the label "names no rate ... `docs/00-charter.md` records no source for one". But the label still asserts which charges apply to a cash-equity trade, introduced by a colon after "EVERY CHARGE". That is a claim about a cost under CLAUDE.md §3 rule 1. The charter has no equity cost row at all, and the label is not marked UNVERIFIED.
  - The enumeration also leaves out IPFT, which this crate treats as its own charge component (trip.rs:1074-1079). It leaves out DP charges, which no file in the repository mentions.
  - A reader takes "every charge: A, B, C" as the set, so the omission understates the gap that the word "gross" is warning about.
  - "Gross" itself is true: nothing is subtracted.
- **Repro.** Not run (text check): `grep -n "GROSS OF EVERY CHARGE" crates/runner/src/audit.rs crates/cli/src/*.rs`. `grep -rni "dp charge\|depository" crates docs/00-charter.md` gives 0 hits.
- **Fix.** Either drop the enumeration ("GROSS OF EVERY CHARGE A SHARE TRADE PAYS; no charge stack is charter-sourced") or mark it "(an UNVERIFIED, non-exhaustive list)". Update the pinned test text.

### p13num-4 (low, citation inconsistency on the one live STT row): the 2026-04-01 rows cite a Finance **Bill** plus Wikipedia as VERIFIED, and their own sibling row and doc call the same change the Finance **Act**
- regime.rs:340-343: `APR_2026, 15_000, "0.15% — Finance Bill 2026 (DEC-COST-003), VERIFIED via \`TaxGuru\`, the Wikipedia STT page, and \`1Finance\` citing the Finance Bill on \`indiabudget.gov.in\`."`
- regime.rs:316: `/// The day the Finance Act 2026 rates take effect.`
- regime.rs:365: `"0.15% on intrinsic value from 2026-04-01 — Finance Act 2026 (DEC-COST-004)."`
- **Why it is wrong.**
  - This row covers 2026-04-01 onward. It is the only STT row that any trip priced today (2026-10-04) uses, and it is typed `Rate::Verified`, documented as "A citation-grounded rate" (regime.rs:86-87).
  - Its citation is a bill as introduced, plus secondary and tertiary sites. Two lines away, the same date is attributed to an enacted Act with no document named.
  - Neither has a charter row (hunt-costs-5). But the row's own text does not support the word VERIFIED that it carries.
- **Repro.** Not run (text check: `sed -n 314,366p crates/costs/src/regime.rs`).
- **Fix.** Cite the enacted Finance Act 2026 section (with the gazette) in both rows and in the charter. Until then, describe the row's source as "Bill, secondary sources" and add it to 06-limits §25's UNVERIFIED list.

### p13num-5 (low, doc-false limit): 06-limits §25 says pre-2023-04-01 trades are "currently priced 25% too high on STT", but every such trade refuses
- docs/06-limits.md:1177-1179: *"If true, `costs`'s 1990-01-01 anchor row is two windows and every trade before 2023-04-01 is currently priced 25% too high on STT."*
- **Why it is wrong.** The same section (:1127-1131) says that on every day before 2024-10-01 the repository "cannot price an options trade at all". `trip::price` refuses any trip with either leg before 2024-10-01 (trip.rs:1232-1233 through `Rates::resolve`). So no trade before 2023-04-01 is priced at any STT rate. Only a direct caller of the public `stt_options_rate` would see 0.0625%, and none exists outside `costs`.
  - The sentence overstates a live harm and contradicts its own section.
  - That breaks CLAUDE.md §3 rule 6 in the opposite direction: it is not honest about which limits are live.
- **Repro.** Not run. Reasoning: `Rates::resolve` calls `exchange_charge_rate`, whose anchor is `Rate::Unverified` (regime.rs:399), and that refuses before 2024-10-01.
- **Fix.** Reword: "...the anchor's 0.0625% would be 25% high for those days. No trade there is priced today, because the exchange-charge refusal covers them. Only `stt_options_rate` callers would see it."

## Verification of prior items in this scope (state at 1f4de71)

| id | status | evidence |
|---|---|---|
| hunt-costs-5 | NOT FIXED (held as UNVERIFIED) | charter has no cost row except :175. 06-limits.md:14119 |
| hunt-costs-8 | DOCUMENTED | regime.rs:326-330. But see p13num-5 for the overstated limit text |
| p6num-1 | NOT FIXED | regime.rs:475-476 still says "selected by the trade's **entry** date" |
| p6num-2 | NOT FIXED | regime.rs:181-184 `verified_from` still takes an unverified successor |
| P9-02, P9-03, P10-01 | not re-reported (filed) | — |
| §1 equity gross labelling (D-0509, D-0525, D-0681, D-0696) | FIXED (present on every surface sampled) | audit.rs:148, pool.rs:665, lib.rs:19198/19212/8264, web charge-scope.js:70 |
| §1 no equity reaches Selection V6 | FIXED (by type) | exit_grid_policy.rs:1253-1264, selection_v6_source.rs:66-68, selection_v6.rs:165-175 |
