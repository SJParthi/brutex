# Numeric / look-ahead / O(1) audit, pass 6 (costs, greeks, indicators)

**Verdict at 1f4de71 (origin/final/all-fixes-zero): 2 new findings, both low (p6num-1 doc-false, p6num-2 latent). Nothing new in greeks, and nothing new on look-ahead or per-bar cost in indicators.** This was an audit by reading source. No cargo was run, because no finding is high. Pass-1 probes for this ground (numeric/pass1/cost.md, grk.md, ind1.md, ind2.md) are relied on where cited.

## Theme 1: crates/costs, charge by charge

| charge | rate (BpsX100 = rate x 1e7) | base / side | rounding point and direction | regime / boundary | charter-traceable? |
|---|---|---|---|---|---|
| brokerage | flat Rs 20 per order (rate.rs:120,129), x2 per trip | per order, both legs | none (flat paisa) | undated | no (`COSTS_VERIFIED`, hunt-costs-5, still UNVERIFIED and held) |
| STT, options sell premium | 6,250 / 10,000 / 15,000 (regime.rs:316-345) | sell notional only, at the **sell leg's** day (trip.rs:1063, D-1535) | `statutory_levy`: floor to paisa, then ceil to rupee, once per trip | `row.start <= day` (regime.rs:177), so the boundary is inclusive on the new rate. The anchor applies 0.0625% before 2023-04-01 (hunt-costs-8, documented) | no (hunt-costs-5) |
| STT on exercise | 12,500 / 15,000 | intrinsic value | n/a: no caller sets exercise, and `price` refuses any outcome but a normal close | inclusive | no |
| exchange txn charge | NSE 3,503, BSE 3,250 from 2024-10-01. Before that it refuses (UNVERIFIED anchor) | both legs, each at its own day | ceil to paisa per leg, then summed | inclusive. Before 2024-10-01 the whole trip refuses | no |
| SEBI turnover fee | 10 (Rs 10/crore) | both legs | ceil to paisa per leg | undated | no |
| IPFT | NSE 50; BSE 0 (UNVERIFIED zero, documented) | both legs | ceil to paisa per leg | undated (FA73061 re-split is UNVERIFIED and not encoded) | no |
| stamp duty | 300 (0.003%), buy side only | buy notional, at the buy leg's day | `statutory_levy` once | undated | no |
| GST | 1,800,000 (18%) | rounded brokerage+exchange+SEBI+IPFT; excludes STT and stamp | `statutory_levy` once. No CGST/SGST split. The max of the two legs' rates is taken | undated | no |
| DP charges | **not modelled** | n/a | n/a | n/a | correct as it stands: no equity delivery segment exists (`scope::Segment` = IndexSpot/IndexFuture/IndexOption) |

- **Integer math.** Every amount is paisa i64. Products are formed in i128 with one checked narrowing each. There is no f32/f64 in `costs/src` outside tests. Overflow on a huge notional is refused, never saturated. The 1,000,000-trip fuzz from pass 1 (0 under-charges, worst over-charge 306 paisa) still applies, because `money.rs` and `charge_stack_legs` are unchanged in arithmetic.
- **Equities (CLAUDE.md §1).** `costs` has no equity segment, so no invented equity charge can be applied. The only cost-crate reach on the runner/cli live path is `costs::fill` with `Anchor::PrintedExtreme` (runner grid.rs:1743-1788, trade.rs). That anchor adds **no** tick: fill.rs:449-461 only refuses a sell below 5 paisa. Equity output carries the gross label: `runner::audit::CASH_EQUITY_GROSS`, `cli::EQUITY_RANKING_GROSS` (lib.rs:19198), `pool::EQUITY_TOTALS_GROSS` (pool.rs:665), and the stored banners (stored.rs:2149). runner/audit.rs:302,326 states that the Segment has no equity variant. **Clean.**
- **Dated tables (lot, strike step, expiry).** `dated.rs:142` uses the same inclusive `<=`. The expiry weekday regime re-reads at each crossed row start. The holiday caveat is documented (expiry.rs:40-46, CE-14). The lookups are fixed-array walks, so O(1).

## Theme 2: crates/greeks, domain edges

| edge | where | behaviour | verdict |
|---|---|---|---|
| T = 0 (expiry minute) | bsm.rs:242 `years <= 0.0` | `Expired` refusal | clean (the feed-side T bias is grk-2, already known) |
| T < 0 | same | `Expired` | clean |
| T > 100 y | bsm.rs:247 | `OutOfRange` | clean |
| sigma <= 0, sigma > 10, NaN/inf | `positive_bounded` bsm.rs:183-200, `finite` :174 | named refusal | clean (the vendor unit is grk-1, known) |
| price <= discounted intrinsic | solver.rs:269 | `PriceBelowIntrinsic`, which also covers equality | clean, and loud |
| price >= upper bound | solver.rs:272 | `PriceAboveMaximum` | clean |
| price outside the [1e-6, 5.0] sigma band | solver.rs:281-293 | `OutsideVolatilityRange` | clean |
| discount-factor overflow | bsm.rs:273 | `NotRepresentable` | clean |
| Newton non-convergence | solver.rs:369-399 | breaks on zero vega, on leaving the band, or after 8 steps, then 75 fixed bisections. At most 86 evaluations | clean, bounded, documented in 06-limits §29 |
| vega underflow | solver.rs:327 | uncertainty becomes +inf, then `Indeterminate` | clean |
| float to paisa | none in greeks. Paisa i64 to f64 is exact below 2^53 | n/a | clean |

## Theme 3: crates/indicators

The evaluator copies itself per bar (`stepped(mut self)`, evaluator.rs:760-766/879). A refused bar therefore commits no partial state. A non-increasing timestamp is refused (:938-942). All state below is fixed-size.

| indicator (file) | state per bar | order at bar N | look-ahead verdict | per-bar cost | warm-up |
|---|---|---|---|---|---|
| EMA 20/200 (trend.rs:169-236) | scaled i128, seed_sum, folded | emit vs EMA(0..N-1), then fold N | none | O(1) | SMA seed over the first `period` bars. `warm` = folded >= period |
| ATR 10 (trend.rs:265-330) | scaled, seed_sum, prev close | folded inside SuperTrend after the emit | none | O(1) | SMA seed of TR. warm = folded >= 10 |
| SuperTrend (trend.rs:375-465) | Atr, stop, trend | emit vs stop(0..N-1), then fold | none | O(1) | gated by ATR warm and stop present |
| Swing fractal (trend.rs:494-640) | ring [5], counters | confirmed at the window's RIGHT edge (bar N), using bars N-4..N | none (the pivot is N-2, published at N) | O(RING=5) | 5 bars |
| Structure / BOS (trend.rs:650-750, 1059) | last break, in-force | advanced from the emit before the fold | none | O(1) | needs swings |
| VWAP + sigma bands (vwap.rs:242-616) | pv, v, p2v i128, contributing | fold N, then emit (VWAP includes bar N's own volume, known at N's close) | none | O(1) plus bounded isqrt (06-limits §51, W3-indicators2-0) | day reset. sigma None at n<2 suppresses the bits (F-9A6BC3 known). Rounding is ind1-1/ind1-2 (known, NOT FIXED: vwap.rs:613-616, trend.rs:1041-1048, session.rs:476-482) |
| CurDayFib (lib.rs:480-525) | hi, lo, leg, count | emit vs hi/lo(day 0..N-1), then fold | none | O(1) | first bar of day emits nothing |
| ORB windows (orb.rs:171-203) | 4 windows | close by clock of bar N, emit, then fold | none (coarse-rung over-collect documented, D-0527) | O(4) | per window close |
| GapFib (gap.rs:283-376) | opening candle, yesterday, leg | leg settled when a later 3-min bucket arrives, then emit, then fold | none | O(1) | after the opening candle, regular day |
| Session shape / gap (session.rs:260-305) | day OHLC, prior [3] | fold H/L of N, emit, then push N | none | O(PRIOR_N=3) | prior full after 3 bars |
| Candle patterns (pattern.rs:366-400) | ring [5], depth | push N, then emit (completed-candle patterns) | none | O(LOOKBACK=5) | depth reset per day |
| Prev-day pivots / CPR (daily.rs) | DailyLevels | installed at rollover from the completed day (evaluator.rs:1232-1284) | none | O(1) | one completed regular session |
| Prev-5 range (fib.rs Prev5) | 5 slots | push at rollover | none | O(5) | 5 sessions |
| Prev-day fib (fib.rs) | from DailyLevels | same | none | O(1) | one session |
| Day-open side 276/277, structure 278/279 | running open | emit before the fold, skipped on the first bar | none | O(1) | second bar of day |
| Crossings (evaluator.rs:1130-1208) | last_side[], crossings_seen[] u8 saturating | from this bar's mask vs the last definite side | none | O(CROSSINGS) fixed | first definite side |
| Anchored daily join (anchored.rs:484-509) | cursor into references | installs only `day < signal_day` | none | amortised O(1) (cursor) | reference present |
| Column::build (column.rs:558-580) | per-bar Vec push | warm = pre-step AND post-step `warmed_up` | none | O(1) amortised push | `warmed_up` = prev5>=5, yesterday, previous, EMA200/ATR warm |

No rolling window has unbounded width. Every ring is a compile-time array. `vwap::availability_of` (a whole-slice read) has no production caller (pass-1 ind2).

## New findings

### p6num-1 (low, doc-false): `stt_options_rate` still documents entry-day regime selection that D-1535 removed
- `crates/costs/src/regime.rs:475-476`:
  ```
  /// The regime is selected by the trade's **entry** date, per the source's
  /// `DEC-COST-002`. A boundary date is inclusive: ...
  ```
- **Why it is wrong.** `trip::price` (trip.rs:1232-1239) resolves each leg at its own day and charges STT at the **sell** leg's day (trip.rs:1063, D-1535, K-44). trip.rs:336-342 and docs/06-limits.md:1605 say so. A caller reading this public doc would key the tax to entry and reproduce the hunt-costs-1 under-charge. In the same vein, docs/04-invariants.md:1399 K-43 still says "A round trip whose **entry day** lands in an unverified window refuses entirely", but since D-1535 an exit day in such a window also refuses the trip (trip.rs:1233).
- **Repro.** Not run (text check: `grep -n "entry\*\* date" crates/costs/src/regime.rs`).
- **Fix.** Reword regime.rs:475-476 to say "the rate in force on `day`; `trip::price` passes each leg's own day (D-1535)". Make K-43 say "either leg's day".

### p6num-2 (low, latent): `RegimeTable::rate_on` and `refusal_windows` keep the `verified_from` defect that `dated.rs` fixed
- `crates/costs/src/regime.rs:181-184`:
  ```
  } else if verified_from.is_none() {
      verified_from = Some(row.start);
  ```
  and regime.rs:211-226 `refusal_windows` (successor = next row's start, whatever its rate).
- **Why it is wrong.** `Refusal::verified_from` and `RefusalWindow::verified_from` are documented as "the first day a verified rate exists again — the window's exclusive end" (regime.rs:278-281). The sibling `DatedTable::value_on` (dated.rs:146-176) was corrected to skip unverified successors, with this note: "they differ the moment two unverified rows abut, which is a table shape the type permits and `is_shipping_shape` deliberately allows ... a wrong answer wearing the shape of a right one", and test `the_day_a_refusal_names_is_a_day_that_answers` (dated.rs:464). `RegimeTable::is_shipping_shape` (regime.rs:263) likewise permits an unverified `later` row (`is_well_shaped` accepts `Rate::Unverified`, :120-126). The likely next edit is exactly that shape: the UNVERIFIED FA73061 2026-03-01 split, or a pre-2023 STT row, noted at :328 and :382 as candidates. That edit would make the refusal point the caller at a day that refuses again. No shipped table has two abutting unverified rows today, so no wrong output exists at 1f4de71.
- **Repro.** Not run. Reasoning: with a table of anchor Unverified, later[0] Unverified@D1, later[1] Verified@D2, `rate_on(day < D1)` returns `verified_from = Some(D1)`, and `rate_on(D1)` is `Err` again.
- **Fix.** Apply the dated.rs:146 condition (`&& matches!(row.rate, Rate::Verified(_))`) in `rate_on`, and the same successor filter in `refusal_windows`. Port the dated.rs:464 test to regime.rs.

## Verification of prior items in this scope (state at 1f4de71)

| id | status | evidence |
|---|---|---|
| hunt-costs-1 | FIXED | trip.rs:1232-1239 resolves per-leg rates. K-44, D-1535 |
| hunt-costs-5 | NOT FIXED (held, UNVERIFIED) | docs/00-charter.md has no cost-rate source. 06-limits.md:14119, 11-findings.md:690 |
| hunt-costs-6 | FIXED | costs/src/lib.rs:65 cites D-0041. But see p6num-1 for a remaining stale doc |
| hunt-costs-7 | NOT FIXED (info, inert) | fill.rs:536-545 still has `slipped.max(TICK)` and `realized = tick + abs(anchor-fill)`. The comment now describes it. There is no production caller of `AdverseExtreme` |
| hunt-costs-8 | DOCUMENTED | regime.rs:322-329 anchor note. 06-limits §25 |
| ind1-1 | NOT FIXED | vwap.rs:613-616 `band_levels` unchanged |
| ind1-2 | NOT FIXED | trend.rs:1041-1048 `Ordering::Equal => mask` on the floored EMA. session.rs:476-482 is unchanged |
| grk-1, grk-2 | not re-verified here (pull/api scope, already tracked) | — |
