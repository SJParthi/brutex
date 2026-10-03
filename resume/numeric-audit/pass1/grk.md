# grk: crates/greeks and its use in crates/pull (numeric pass 1, commit 331b05c)

**Verdict.** The closed form, the normal CDF and the IV solver hold up. The CDF's absolute error is 2.22e-16 (one ulp) against glibc `erfc`. The solver never needs more than 86 evaluations. Its reported `uncertainty` bounds the actual round-trip error on every point of a 1,694-point grid (worst actual/reported ratio was 0.64). A negative model price did not occur on a 96,000-point scan. Every non-finite input is refused by name.

There are two NEW defects, both in how `pull`/`api` feed the model:
- grk-1: the vendor's implied volatility has no unit check, so a value sent in percent and below 10 is taken as a decimal and stored as the vendor's own figure.
- grk-2: the tenor is measured from the bar's OPEN stamp, but the premium is that bar's CLOSE.

Probe crate: `$SCRATCH/probes/grk/` (path deps on greeks, pull, core, costs; `cargo run --release --offline`).

## Findings

### grk-1 · medium · crates/api/src/server.rs:13368-13370 with crates/pull/src/pricing.rs:760-764 and crates/greeks/src/bsm.rs:101
```rust
// server.rs price_group
// THE VENDOR'S OWN VOLATILITY, IN MILLIONTHS. `125_000` is `0.125`.
if let Some(iv) = row.overlay.iv() {
    sent.insert(ts, millionths_to_decimal(iv));
}
// pricing.rs price
let (volatility, vol_from) = if let Some(sent) = volatility {
    (sent, VolSource::Vendor(quote.vendor))
// bsm.rs
pub const MAX_VOLATILITY: f64 = 10.0;
```
**Why it is wrong.** Dhan's rolling-option `iv` is assumed to be a decimal (0.125). The only source for that is a test fixture written in this repo (rolling.rs:1090 `"iv":[0.125]`). The one real Dhan IV sample the charter records (docs/00-charter.md section 4b) is in **percent**: `IV 11.939337251984934 / 9.789193798280868`. D-0195's follow-up says the rolling path "has never executed" against the vendor. So §3 rule 1 says this unit should be marked UNVERIFIED.

Nothing screens the value for plausibility, and nothing checks it against the premium. The only bound is `MAX_VOLATILITY = 10.0`. As a result:
- A percent IV of 10 or more is refused, which is loud.
- A percent IV below 10 (low-vol regimes, including the charter's own 9.79 sample) is accepted as 979% volatility. The greeks computed from it are filed in `.grk` with `VOL_FROM_VENDOR` provenance. That is silent wrong data.

The risk-free rate has exactly this percent-for-decimal screen (`MAX_PLAUSIBLE_RATE = 1.0`, invariant P-50, "the single likeliest wrong number on this path"). The volatility, which arrives from a vendor rather than from the operator, has none.

**Repro (probe ran).** NIFTY ATM call, spot 25,000.00, strike 25,000.00, premium 250.00, 7 days, r = 0.065, `pull::pricing::price(q, Some(v), …)`:
```
C vendor iv 0.09789193798280868: Ok vol=0.09789193798280868 delta=0.540066 price_model=15444.55 paisa vs premium 25000 paisa
C vendor iv 9.789193798280868: Ok vol=9.789193798280868 delta=0.755402 price_model=1276330.64 paisa vs premium 25000 paisa
C vendor iv 11.939337251984934: Err the model refused this quote: `volatility` is 11.939337251984934; this crate evaluates magnitudes up to 10
C solved from premium: vol=0.16601326328235289
```
- Expected: 9.79 is refused (or converted under a recorded unit).
- Actual: it is accepted. Delta is 0.755 instead of about 0.54, and the model price is 12,763 rupees against a 250-rupee premium. No check fires.

**Fix shape.** Record the vendor's unit with a source, or mark it UNVERIFIED and refuse. Add a cheap screen: refuse when the model price at the sent volatility is outside a tolerance of the premium, or compare against the solved IV.

### grk-2 · low · crates/pull/src/tenor.rs:251-255 with crates/api/src/server.rs:11907/11923 and 13341/13376
```rust
let bar_secs =
    i64::from(at.minute_of_day()) * SECONDS_PER_MINUTE + i64::from(at.second_of_minute());
let seconds = days * SECONDS_PER_DAY + close_secs - bar_secs;
// server.rs: Tenor::between(bar.ts_micros, expiry) … premium: bar.close
```
**Why it is wrong.** Bars are stamped at the **open** of their bucket (session.rs:105, fold.rs:358). The premium priced is `bar.close`, which is observed one bar width later. So every row's time to expiry is overstated by one interval: 60 s on the 1-minute rung, and a whole bucket on higher rungs (`price_chain_month` uses `month_of.timeframe`).

The worst case is the last bar of expiry day. The 15:29 bar's close is printed at 15:30, which is the expiry moment, so T is 0. That row is priced with T = 60 s instead of being refused as `AlreadyExpired`. The bias is systematic: it always lowers the solved IV, it never averages out, and it grows toward expiry, which is where this market trades.

**Repro (probe ran).** Expiry 2025-07-31 (close 15:30). Call, spot 24,800, strike 24,750:
```
D expiry-day bar opened 15:29 -> tenor Ok(60)
D last bar priced: tenor_seconds=60 vol=0.847687203688055 theta/yr=-6.86171292327665e7 gamma=3.098215293115027e-5
D same premium at T measured from 14:30: Ok(0.2656463909749829)
D same premium at T measured from 14:31: Ok(0.2679168965570885)
```
- Expected: the 15:29 one-minute bar (close at 15:30) has zero tenor and is refused. A premium closing at 14:31 is solved at T = 59 min, giving IV 0.26792.
- Actual: the 15:29 bar is priced at T = 60 s, with IV 0.85 and a theta of -6.9e7 per year written to `.grk`. The 14:31 close is solved at T = 60 min, giving IV 0.26565, which is 0.85% low one hour out. The error grows as T shrinks.

**Fix shape.** Measure the tenor from `ts + bar width` (the moment the close is observed), or document the convention and refuse the final bar.

## Checked and clean
- `normal::standard_normal_cdf`: worst absolute error 2.220e-16 over [-37, 8] at 1e-4 steps against glibc `erfc` (probe A). The worst relative error is 8.9e-9 at x = -7.79 in the continued-fraction branch. That is within the module's own asserted 1e-8, so INFO only; it is financially negligible at that depth. The tails saturate at |x| > 37 by design. The NaN input propagates and ±inf map to 0/1 (tested).
- `bsm`: textbook BSM-with-yield price, delta, gamma, vega, theta and rho, confirmed by reading. `Contract::check` refuses NaN/inf, zero/negative spot or strike, T <= 0 (`Expired`), and sigma <= 0 or > 10. Overflowing discount factors give `NotRepresentable`. The output is gated by `is_finite`. Negative model price scan (call, sigma 1e-6..1e0, T 3.2e-8..1e-2 yr, d1 -5..-34.5): 0 negatives (probe B). A deep-OTM put gives 0.0, not a negative.
- `solver::implied_volatility`: the no-arbitrage bounds use the discounted intrinsic value. The bracket ends are checked for finiteness. Newton is capped at 8 steps; bisection runs exactly 75 steps (5/2^75 is below ulp(1e-6)). `MAX_ITERATIONS = 86` was observed as the max (probe E). A NaN mid-bracket price cannot arise because inputs are pre-checked. A vega of 0 gives an infinite uncertainty, which becomes `Indeterminate`, not a silent answer. The `uncertainty` bound was honest on all 1,035 solved grid points (worst actual/reported 0.64; 0 points above 4x).
- Iteration count as a non-O(1) cost: bounded by a constant, not flat. Already documented in 06-limits §29, and the code matches it.
- `moneyness::from_ladder`: an off-grid strike is refused, a step bound is applied before the `as i32` cast, and the cast is in range.
- Float to paisa: no greeks result is converted to paisa anywhere. Greeks are filed as full-precision f64 (`greek_records`), and spot/strike/premium stay i64. The `i64 as f64` widening of paisa is exact below 2^53.
- `millionths_to_decimal` and `shift_six`: integer half-up on the 7th decimal with checked arithmetic, and no float rounding before the single division. The UNIT is the problem (grk-1), not the arithmetic.
- `SpotBook`: an exact-stamp HashMap join with no neighbouring-minute borrow, O(1) per row. The spot is the index close at the same stamp as the option close, so there is no look-ahead.
- `Rate::measured`: the ±1.0 screen and the citation requirement hold.
