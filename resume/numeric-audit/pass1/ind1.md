# ind1: crates/indicators numeric computations (EMA, ATR, SuperTrend, VWAP/sigma bands, pivots, fib, gap mid)

Audited at origin/final/all-fixes 331b05c. Audit only; no repo file was touched.
Probe crate: `$SCRATCH/probes/ind1` (path deps on crates/indicators and crates/vocab). Raw output: `$SCRATCH/probes/ind1/out.txt`.

**Verdict.** The evaluation path uses no floats. Every accumulator is i128 with `checked_`/`saturating_` arithmetic, and every division is `div_euclid` with its divisor guarded. I found no overflow, no division by zero on flat prices, no NaN surface and no running-sum drift. VWAP sigma is the exact floor (D-0940), and it matched an exact oracle in P1. There are two new findings, both low. They come from one root cause: a level is rounded down to an integer paisa and then compared strictly or offset by a multiple.
(1) The VWAP sigma bands are built as `floor(vwap) ± m·floor(σ)`, so a band edge can be off by up to `1+m` paisa. Against an exact oracle this flips band bits.
(2) A floored level compared with strict `<` loses the case where the true level is fractional and `close == floor(level)`. The answer becomes "neither" instead of "below". This happens for EMA (bits 1/3), VWAP (53/144) and the gap midpoint (67). The "above" side stays exact, so the rounding is asymmetric.

## Findings

### ind1-1 (low): VWAP sigma-band edges compound two floors, so a band can be off by up to 1+m paisa
- `crates/indicators/src/vwap.rs:309` `i64::try_from(self.pv.div_euclid(3 * self.v)).ok()` (vwap floored)
- `crates/indicators/src/vwap.rs:393` `i64::try_from(sigma_scaled.div_euclid(3)).ok()` (σ floored)
- `crates/indicators/src/vwap.rs:615-618` `band_levels`: `let offset = i64::try_from(multiple.checked_mul(i128::from(sigma))?).ok()?; Some((vwap.checked_add(offset)?, vwap.checked_sub(offset)?))`
- **Why it is wrong.** D-0940 made σ exactly `floor(σ)`, "nothing coarser". The band then multiplies that floored σ by m=1/2/3 and adds it to a separately floored VWAP. The band is therefore narrowed by up to m·frac(σ) on each side and shifted down by frac(VWAP). Bits 146-152 and 190-197 (above, below and inside for each band) are decided against an edge up to 4 paisa from the true edge, on every cash-equity run (`Availability::Present`). The exact form is cheap and needs no rounding: test `(3V·close − pv)² > m²·(V·p2v − pv²)` with a sign check, using the 256-bit compare the module already has.
- **Repro (probe ran).** Four bars, open=close: (h,l,c,v) = (1016,1014,1014,602), (1020,1017,1017,37), (1017,1016,1016,470), (1022,1018,1018,481). The exact VWAP is 1016.65 and the exact σ is 1.93, so the band-1 upper edge is 1018.58 and close 1018 is not above it. The code computes vwap=1016 and sigma=1, so the upper edge is 1017. **Bit 146 `close_above_vwap_band1_upper` is set (expected false).** Verbatim:
  ```
  P2 sessions with sigma>=1: 157424; mismatches vs exact [b1 above, b1 below, b2 above, b2 below, b3 above, b3 below] = [9564, 11299, 13103, 4497, 6018, 1925]
    first example: [... (1014,1016,1014,1014,v602) (1017,1020,1017,1017,v37) (1016,1017,1016,1016,v470) (1018,1022,1018,1018,v481)] m=1 computed_above=true exact_above=false vwap=Some(1016) sigma=Some(1)
  P4 realistic (200 sessions x 375 bars, ~1000-rupee stock, 5-paisa grid): row-bands evaluated=224400 mismatching=384 (spurious above/below=268, missed=116)
  ```
  P2 uses deliberately small-σ synthetic sessions, so its rate is inflated. P4 uses a more realistic random walk and gives 0.17% of (bar, band) answers wrong. The effect grows as σ shrinks relative to the paisa grid, which means low-priced F&O stocks and quiet opening minutes.
- **Not known.** I grepped docs/06-limits.md, docs/11-findings.md and audit-20261003-workspace for band and floor terms. F-9A6BC3 (σ=Some(0) at n=2) is a different defect, and P2 and P4 exclude σ=0.

### ind1-2 (low): a floored level compared with strict `<` drops "below" when the true level is fractional (EMA 1/3, VWAP 53/144, gap mid 67)
- `crates/indicators/src/trend.rs:228` `i64::try_from(self.scaled.div_euclid(SCALE)).ok()`. Compared in `side` at trend.rs:1017-1025 (`Ordering::Equal => mask`).
- `crates/indicators/src/vwap.rs:309` (floored VWAP), compared at vwap.rs:518-525 (`close > vwap` / `close < vwap`).
- `crates/indicators/src/session.rs:476-482` `let mid = prev.close.midpoint(day_open); if close > i128::from(mid) {66} if close < i128::from(mid) {67}`.
- **Why it is wrong.** For an integer close c and a real level L: `c > L ⇔ c > floor(L)`, so "above" stays exact. But `c < L` also holds when `c == floor(L)` and L is fractional, and the code reports that case as "on the level". So the below bits (1, 3, 53, 144, 67) are under-reported, and the predicate is not symmetric under reflecting price. The EMA case is persistent and not a one-bar event. The integer update `floor(2·delta/(n+1))` stalls up to n scaled units *below* the target when approaching from below. It converges *exactly* when approaching from above, because a negative delta floors to at least −1. So a flat price after a rise reports `close_above_ema` on every bar, and the mirror-image flat price after a fall reports neither, indefinitely. The doc test `a_close_exactly_on_the_average_sets_neither_side` covers only a series that was flat from the start. The fix is either to compare in the scaled domain (`close·SCALE` vs `scaled`; `close·3V` vs `pv`; `2·close` vs `prev.close + day_open`) or to round the level to nearest. It is an integer change either way.
- **Repro (probe ran).** Verbatim:
  ```
  P3 EMA20 after 1000 bars at 20000: from below value=Some(19999), from above value=Some(20000)
    TrendState at flat close 20000 after a rise: bit0 above_ema20=true bit1 below_ema20=false bit2 above_ema200=true bit3 below_ema200=false
    TrendState at flat close 20000 after a fall: bit0 above_ema20=false bit1 below_ema20=false bit2 above_ema200=false bit3 below_ema200=false
  P5a VWAP exact=1000.5 value=Some(1000) close=1000: bit52 above=false bit53 below=false bit143=false bit144=false
  P5b gap mid exact=2412351.5 close=2412351: bit66 above_gap_mid=false bit67 below_gap_mid=false
       mirror close=2412352 (0.5 above): bit66=true bit67=false
  ```
  P3 inputs: 200 bars at 10,000 or 30,000, then 1,000 bars at 20,000. The two runs are mirror images and give different bits. P5a inputs: two bars at 1001 and then 1000 with equal volume; expected bit 53/144 set, actual neither. P5b inputs: previous close 2,412,301, open 2,412,402, close 2,412,351; expected bit 67 set, actual neither, while the mirror case 0.5 above sets 66. On a 5-paisa equity grid, the gap and VWAP half-paisa cases cannot occur but the EMA stall still does. On 1-paisa index levels all three occur.
- **Not known.** F-7236E8 is about `CurDayFib::level` truncating versus flooring, which is different. hunt-indicators-1 is about seed weight at warm-up, also different.

## Checked and clean
- No `f32`/`f64` on the evaluation path. The only `as` casts outside tests are const asserts.
- **EMA** (trend.rs:198-221): i128 at SCALE 1e6 cannot overflow (i64::MAX·1e6 ≈ 9.2e24); `div_euclid` floors negative steps; the `period+1 <= 0` guard is in place; there is no drift on a flat series. The one-bar seed at warm is KNOWN (hunt-indicators-1, docs/06-limits §59).
- **ATR** (trend.rs:275-330): true range is two-sided with no `abs` panic; the `period > 0` guard is in place; Wilder 1/n smoothing; the stall from below is at most period−1 scaled units (1e-5 paisa), negligible.
- **SuperTrend**: band = `atr·mult/1000` uses div_euclid; the i128 midpoint of positive prices floors; an unrepresentable stop becomes None and reseeds. The flip-versus-previous-stop variant is KNOWN (hunt-indicators-7).
- **VWAP accumulators** (vwap.rs:400-460): every product is checked before the ceiling compare; `3*self.v` cannot overflow because p2v ≥ 9v and p2v ≤ 1e34; `v <= 0` gives None, so there is no zero divide; a zero-volume bar is skipped; resets on IST day. σ matched my exact oracle in P1. `wide_mul` is correct for operands below 2^127. The isqrt oscillation and its bounded cost are KNOWN (W3-indicators2-0, docs/06-limits §51).
- **Pivots** (daily.rs:192-240): sum in i128, `div_euclid`, narrowed once with a refusal (no clamp). The CPR width rounding past range/3 is DOCUMENTED in the doc comment. `cpr_class` is cross-multiplied in i128, and range 0 gives None.
- **Fib** (fib.rs:164-195, lib.rs:651) `per_mille` uses i128 and div_euclid; lib.rs:539 `/` agrees for the non-negative CURDAY_RUNGS (F-7236E8 is KNOWN anyway).
- **Pattern and session ratios**: all cross-multiplied in i128 with no division; the `range > 0` guard comes first; equality is handled explicitly.
- **ORB**: compares exact running extremes; `minutes_since_open` uses div_euclid/rem_euclid and saturates.
- **Look-ahead in these families**: every family folds per bar from its own state. VWAP and session fold-then-emit include the current bar by design. Trend emits before fold (`emit` reads the pre-fold swings). None of the numeric code reads a later bar.
- **O(1)**: every fold and emit in these files is fixed work. The only operand-dependent loop is isqrt (KNOWN, documented). `availability_of` is O(n) once per run, documented.
