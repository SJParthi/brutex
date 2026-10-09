//! Session-anchored VWAP and its three sigma bands.
//!
//! **20 vocabulary positions**: 52–53, 143–152 and 190–197.
//!
//! # Instrument eligibility is decided before reading bars
//!
//! [`Vwap::for_slice`] receives the caller's volume policy. A spot-index run
//! uses [`Availability::Absent`], even if its vendor supplies constituent-volume
//! aggregates. Eligible cash-stock runs use [`Availability::Present`] and their
//! own observed OHLCV. A futures contract also has its own traded volume, but
//! the CLI's separate sweep-scope gate excludes contracts.
//!
//! Eligibility never changes after scanning later bars. Reference readiness
//! does change causally: a present-volume run needs two contributing bars in
//! the current session before these predicates can be evaluated. Zero-volume
//! bars do not contribute. Unavailable predicates have zero truth bits and
//! zero known bits, so NOT cannot turn missing volume into a signal. Once
//! references exist, exact comparisons are known; near predicates additionally
//! need positive dispersion and the correct tolerance family.
//!
//! This is an OHLCV typical-price VWAP, not a reconstruction of trade-by-trade
//! VWAP: each observed bar contributes `(high + low + close) / 3`, weighted
//! by its own volume. No ticks or replacement volume are introduced.
//!
//! # It is measured, and the measurement went the other way
//!
//! `docs/06-limits.md` records `volume` as "100% literal zero" across 1,706,290
//! bars, and that sentence is **wrong about the data on disk**: 1,237 of 1,239
//! NIFTY daily bars carry non-zero volume. Worse, the two that carry zero moved
//! 573 and 286 points — so a per-bar check would divide by zero on a day that
//! plainly traded. That is exactly the case the per-run decision handles and a
//! per-bar check does not.
//!
//! # The accumulator, and where it overflows
//!
//! `sum(price × volume)` grows without bound over a session and is the one place
//! this crate can genuinely overflow on real data. Both accumulators are `i128`,
//! and the variance accumulator needs `price² × volume`, which is why it is held
//! separately and checked: at a BANKNIFTY price of 5.7 × 10⁶ paisa and 10⁹ volume
//! a single bar's `p²v` is 2.9 × 10²³, and a whole 375-bar session reaches
//! 1.1 × 10²⁶ — eight orders inside `i128` (1.7 × 10³⁸) and far past what `i64`
//! could hold. Computed, not estimated; an earlier draft of this paragraph was an
//! order of magnitude out.
//!
//! [`Vwap::fold`] refuses rather than wraps — and that sentence was **false when
//! first written**. It compared `p2v + price² · volume` to [`ACC_CEILING`] *after*
//! computing it, so an input that overflowed `i128` wrapped to a large negative
//! value, sailed under the ceiling, and returned `Ok(())` from a poisoned
//! accumulator; the same input panicked in debug. `high = low = close = 5 × 10¹⁸`
//! reaches it, every field inside `i64`, and `store::format`'s `ohlc_is_sane`
//! checks field ORDER only — never magnitude — so such a bar is legally writable
//! and readable back. Every multiply on the fold path is now `checked_`, evaluated
//! **before** any comparison, because a ceiling cannot bound a product that has
//! already wrapped. `the_i64_extreme_is_refused_rather_than_wrapped_or_panicked`
//! is the test; it would have gone red on release and panicked on debug.
//!
//! There are **four** `checked_` refusals on that path, and only two of them can be
//! reached by handing [`Vwap::fold`] a legal bar: [`ACC_CEILING`] bounds the state
//! every fold starts from, so the accumulate-into-`p2v`, `pv` and `v` steps cannot
//! be made to overflow from a state `fold` itself produced. The tests build those
//! states from the private fields instead. That is not a shortcut around the type —
//! it is the only way to execute a guard whose precondition the surrounding code
//! makes unreachable, and a guard no test can trip is an argument wearing a
//! mechanism's clothes. The `v` case also pins the fold's atomicity: a refusal on
//! the last accumulator leaves `pv` and `p2v` exactly as they were.
//!
//! # Integer square root, no floats
//!
//! Sigma needs a square root and §7 forbids floating point at any layer. This uses
//! a decreasing Newton iteration on `i128` from a seed at or above the root, whose
//! step count is **bounded** by one compile-time constant — `ITERATION_CEILING` =
//! 16 — for every `i128`. Measured by `C-I-03`, in
//! `crates/indicators/benches/ratio.rs`, which folds the count at 1, `i64::MAX`,
//! `10^30` and `i128::MAX`: 2, 5, 5 and 6 iterations (D-1665; the old loop took
//! 1, 37, 55 and 69, and 128 or 129 at every `k^2 - 1`).
//!
//! **Bounded, not flat.** The count varies with the operand and each iteration is
//! a 128-bit division. The honest limit is in `docs/06-limits.md` and the long
//! version is on [`isqrt_i128`], which also records why the earlier loops were
//! worse than this.
//!
//! # Sigma refuses on one observation
//!
//! A single contributing bar has zero dispersion. Reporting `Some(0)` is
//! arithmetically correct and semantically ruinous: it puts all three bands exactly
//! on VWAP, which makes `close_above_vwap_band3_upper` an alias for
//! `close_above_vwap` and hands the sweep two permanently-identical positions on
//! the first bar of every session. [`Vwap::sigma`] returns `None` below
//! `MIN_FOR_SIGMA` contributing bars instead.

use crate::Candle;
use vocab::{ConditionMask, Tolerance};

/// Whether the caller permits this instrument's own traded-volume reference.
///
/// Decided once, before the first bar, and never revisited. See the module
/// documentation for why per-bar is wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// Eligible volume source; session readiness still requires contributing bars.
    Present,
    /// Volume reference disabled. Every VWAP position stays false and unknown
    /// for the whole run, regardless of the numerical volume supplied in bars.
    Absent,
}

/// Why an accumulator refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// A record that is not a bar.
    Corrupt(crate::Corrupt),
    /// The price-times-volume accumulator would leave the range this module can
    /// prove safe. Refused rather than wrapped.
    AccumulatorTooLarge,
}

/// The band multipliers, in sigma. The design source ships exactly these three.
pub const BAND_SIGMA: [i128; 3] = [1, 2, 3];

/// The ceiling this module will let `sum(p² · v)` reach before refusing.
///
/// `i128::MAX` is 1.7 × 10³⁸. This stops at 10³⁴ — four orders of margin, which
/// leaves room for the centred `p2v − q·(pv + r)` arithmetic sigma does without
/// having to prove a second bound.
const ACC_CEILING: i128 = 10_i128.pow(34);

/// Integer square root by Newton's method, in a **compile-time-bounded** number
/// of steps.
///
/// `i128::isqrt` exists but is not `const`, and an unbounded loop would make
/// sigma's cost depend on the value, which `CLAUDE.md` §3 rule 4 forbids.
///
/// # Decreasing Newton from a seed at or above the root (D-1665)
///
/// The seed is `1 << ceil(bits(v) / 2)`, which is at least `sqrt(v)` and at most
/// twice it. From a seed at or above the root, the integer Newton step
/// `(g + v / g) / 2` strictly decreases until it reaches `floor(sqrt(v))`, and the
/// first step that does NOT decrease is the exit: the textbook rule, which returns
/// the exact floor with no correction afterwards.
///
/// The seed's relative error `e` is at most 1, and each step leaves at most
/// `e^2 / 2`, so six steps take it below `2^-64` -- under one unit for any root an
/// `i128` can have -- and one or two more steps reach the exit. **Measured**
/// (W3-indicators2-0): at most 5 iterations over every `v` in `1..=10^6`, 7 at
/// `isqrt(i128::MAX)^2 - 1`, and 8 as the worst of five million pseudo-random
/// inputs across every bit length. [`NEWTON_STEPS`] = 16 is twice that.
///
/// # What it replaced, and why the old figures were wrong (W3-indicators2-1)
///
/// The previous loop started at `guess = v` and stopped on `guess == previous`.
/// From `v` a 127-bit input spends about sixty-four halvings before Newton even
/// converges -- so 69 iterations at `i128::MAX` -- and every `v = k^2 - 1`
/// (3, 8, 143, ...) never stops at all: Newton oscillates between `k - 1` and `k`,
/// `guess` never equals `previous`, and the loop ran its whole 128-step cap, then
/// a bounded step-down fixed the root. 999 of the inputs in `1..=10^6` hit that
/// cap. The root was right; the documented 1-to-69 iteration range was not.
///
/// # BOUNDED IS NOT FLAT
///
/// The count still depends on the operand -- 2 at `v = 1`, up to 8 measured --
/// and each iteration is a 128-bit division whose own cost rises with its
/// operands. The honest limit is in `docs/06-limits.md` §51; `C-I-03` in
/// `crates/indicators/benches/ratio.rs` asserts the count against
/// [`ITERATION_CEILING`] and prints the cost as context, not as a ceiling.
///
/// VWAP abstains entirely on spot indices, so this function does not execute on
/// those runs. Eligible cash-stock sweeps do execute it; their per-bar cost
/// includes this bounded, operand-dependent work.
#[must_use]
pub fn isqrt_i128(v: i128) -> i128 {
    isqrt_i128_counted(v).0
}

/// [`isqrt_i128`], and how many iterations it took.
///
/// The count exists so the bound can be MEASURED rather than trusted to the
/// constants. `crates/indicators/benches/ratio.rs` asserts it against
/// [`ITERATION_CEILING`] at the extremes of the input range, and divides the
/// measured cost by it to get cost per iteration — which is the quantity that
/// has to be flat for a bounded count to bound anything at all.
///
/// This is `greeks::solver`'s pattern, for the same reason: a function whose
/// iteration count varies cannot be timed end to end against a flat ceiling, and
/// pretending otherwise produces either a false breach or a relaxed ceiling that
/// no longer catches anything.
#[must_use]
pub fn isqrt_i128_counted(v: i128) -> (i128, u32) {
    if v <= 0 {
        return (0, 0);
    }
    // AT OR ABOVE THE ROOT: `bits` is at most 127, so the shift is at most 64
    // and the seed fits. `v >= 2^(bits - 1)` makes `2^ceil(bits / 2)` at least
    // `sqrt(v)`, and `v < 2^bits` makes it at most twice the root.
    let bits = i128::BITS.saturating_sub(v.leading_zeros());
    let mut guess: i128 = 1_i128 << bits.div_ceil(2);
    let mut i: u32 = 0;
    for step in 1..=NEWTON_STEPS {
        i = step;
        let next = guess.midpoint(v / guess);
        // THE TEXTBOOK EXIT: from above, Newton decreases until it reaches the
        // floor, and the first step that does not decrease means it has.
        if next >= guess {
            break;
        }
        guess = next;
    }
    // A RANGE, NOT `while i < NEWTON_STEPS` (R1286-rest-04, D-4152). The same
    // steps, the same exit and the same count, with no comparison on the bound:
    // no input reaches it -- 8 steps is the most any of 41,656,974 measured
    // inputs took -- so `<` and `<=` there were one program, and Gate 18 run
    // 1286 could not kill the second.
    (guess, i)
}

/// Newton iterations allowed. Twice the measured worst case — see
/// [`isqrt_i128`] for the proof sketch and the measurement.
pub const NEWTON_STEPS: u32 = 16;

/// The most iterations [`isqrt_i128`] can ever perform.
///
/// A compile-time constant, which is what bounds the function's cost. Measured by
/// `C-I-03`, in `crates/indicators/benches/ratio.rs`, which asserts the real count
/// against this ceiling at both ends of the input range rather than trusting the
/// constant to be large enough.
///
/// It bounds the cost and does **not** make it uniform; the spread is recorded in
/// `docs/06-limits.md`. It equals [`NEWTON_STEPS`]: the decreasing iteration
/// lands exactly on the floor, so the bounded step-down the old oscillating loop
/// needed is gone (D-1665).
pub const ITERATION_CEILING: u32 = NEWTON_STEPS;

/// The running VWAP accumulators for one session.
///
/// Fixed size. It does not grow with the number of bars.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Vwap {
    availability: Availability,
    session_day: i64,
    live: bool,
    /// `sum(hlc3_scaled · volume)`, where `hlc3_scaled = high + low + close`.
    /// Scaled by three so the mean needs one division at the end rather than one
    /// per bar.
    pv: i128,
    /// `sum(volume)`.
    v: i128,
    /// `sum(hlc3_scaled² · volume)`, for the variance.
    p2v: i128,
    /// Bars that actually contributed volume to this session. Not a bar count:
    /// zero-volume bars are skipped and must not make dispersion look measurable.
    contributing: u32,
}

/// Contributing bars required before [`Vwap::sigma`] will answer.
///
/// Two, because one observation has no dispersion. Reported as `None` rather than
/// `Some(0)` so the band positions stay unset instead of collapsing onto VWAP.
const MIN_FOR_SIGMA: u32 = 2;

const _: () = assert!(core::mem::size_of::<Vwap>() <= 80);

impl Vwap {
    /// A new accumulator for a slice whose volume availability is already decided.
    #[must_use]
    pub const fn for_slice(availability: Availability) -> Self {
        Self {
            availability,
            session_day: i64::MIN,
            live: false,
            pv: 0,
            v: 0,
            p2v: 0,
            contributing: 0,
        }
    }

    /// Decide availability by scanning a slice once, before the run.
    ///
    /// O(bars) and paid **once per run**, not per bar — the whole point. A slice
    /// with a single non-zero volume anywhere is [`Availability::Present`], because
    /// a run must not change its mind halfway.
    #[must_use]
    pub fn availability_of(bars: &[Candle]) -> Availability {
        if bars.iter().any(|b| b.volume > 0) {
            Availability::Present
        } else {
            Availability::Absent
        }
    }

    /// Whether this run can compute VWAP at all.
    #[must_use]
    pub const fn availability(&self) -> Availability {
        self.availability
    }

    /// The VWAP so far, in paisa. `None` when there is nothing to divide by.
    #[must_use]
    pub fn value(&self) -> Option<i64> {
        if self.availability == Availability::Absent || self.v <= 0 {
            return None;
        }
        // pv is scaled by three (hlc3 held as high+low+close), so divide by 3v.
        i64::try_from(self.pv.div_euclid(3 * self.v)).ok()
    }

    /// One standard deviation of the volume-weighted price, in paisa. `None` when
    /// VWAP itself is unavailable.
    ///
    /// # Exactly `floor(σ)`, and the formula it replaced was not (D-0940)
    ///
    /// With `P = high + low + close` per bar (the ×3 scale), `V = Σv`,
    /// `pv = ΣPv` and `p2v = ΣP²v`, the weighted variance on the ×3 scale is
    /// `σ₃² = (V·p2v − pv²) / V²`, and this returns `floor(σ₃ / 3)` — the floor of
    /// the exact volume-weighted standard deviation in paisa, nothing coarser.
    ///
    /// This used to compute `isqrt(floor(p2v/V) − floor(pv/V)²)`. Flooring the
    /// mean BEFORE squaring it adds up to `2·M·frac(M)` to the variance
    /// (`M = pv/V`, about three times the price), so two bars at 100.00 and
    /// 100.01 rupees — half a paisa of dispersion — reported 57 paisa, and at
    /// 57,000 rupees 1,378. Every band position (146–152, 190–197) and the near
    /// scale of 145 were decided on that number for every cash-equity run
    /// (`Availability::Present`). F-9A880B, ET-indicators-0, ET-indicators-12.
    ///
    /// **The exact form, in O(1).** Let `q = floor(pv/V)` and `r = pv − qV`,
    /// so `0 <= r < V`. Then
    ///
    /// * `C = p2v − q·(pv + r) = Σ v·(P − q)²`, since `q·(pv + r) = 2q·pv − q²V`;
    ///   so `C >= 0` and it is an integer;
    /// * `σ₃² = C/V − (r/V)²`, since `pv/V = q + r/V`;
    /// * with `C = wV + s`, `0 <= s < V`: `σ₃² = w + s/V − r²/V²`, both fractions
    ///   in `[0, 1)`, so `floor(σ₃²) = w` when `s·V >= r²` and `w − 1` otherwise;
    /// * `floor(sqrt(x)) = isqrt(floor(x))` for real `x >= 0`, and
    ///   `floor(floor(y)/3) = floor(y/3)`, so `isqrt(floor(σ₃²)) / 3` is
    ///   `floor(σ₃/3)` exactly.
    ///
    /// **Overflow.** For any state a fold produces, `p2v <= ACC_CEILING`, every
    /// `P >= 3`, and Cauchy–Schwarz gives `pv² <= V·p2v`; so
    /// `q·pv <= pv²/V <= p2v` and `q·r < q·V <= pv <= p2v/3`, and `q·(pv + r)`
    /// stays below `2 × 10³⁴`, far inside `i128`. `s·V` and `r²` do NOT: `V`
    /// passes `2^64` after two bars at `i64::MAX` volume, so they are compared as
    /// 256-bit products by [`wide_mul`]. Every `i128` step is still `checked_`,
    /// so a hostile private state refuses rather than wraps.
    ///
    /// Proven against an independent big-integer oracle by
    /// `sigma_is_the_exact_floor_whatever_the_remainder_of_the_mean`,
    /// `sigma_is_exact_where_the_remainder_squared_leaves_i128` and
    /// `maximal_volumes_at_minimal_prices_stay_exact`. Those prove exactness,
    /// not cost: the O(1) above is by construction (a fixed number of checked
    /// operations and one `isqrt_i128`, no loop over bars) and is UNVERIFIED as a
    /// bench.
    #[must_use]
    pub fn sigma(&self) -> Option<i64> {
        if self.availability == Availability::Absent || self.v <= 0 {
            return None;
        }
        // A single contributing bar has zero dispersion, which is arithmetically
        // correct and semantically useless: sigma of 0 collapses all three bands
        // onto VWAP, so `close_above_vwap_band3_upper` degenerates into
        // `close_above_vwap` and two vocabulary positions become permanently
        // identical. Identical positions inflate support and hand the sweep a pair
        // whose k=2 support equals both k=1 supports — a discovery that is an
        // artifact. Refuse instead: MIN_FOR_SIGMA is a property of what dispersion
        // means, not a tuning knob, and it is stated here rather than assumed.
        if self.contributing < MIN_FOR_SIGMA {
            return None;
        }
        // The derivation and the overflow bound are in the doc comment above.
        // Every i128 step is checked: the ceiling bounds them for a folded state,
        // but the bound is an argument and this is a mechanism.
        let q = self.pv.div_euclid(self.v);
        let r = self.pv.rem_euclid(self.v);
        let centred = q
            .checked_mul(self.pv.checked_add(r)?)
            .and_then(|t| self.p2v.checked_sub(t))?;
        let whole = centred.div_euclid(self.v);
        let rest = centred.rem_euclid(self.v);
        // floor(C/V − r²/V²): one less than floor(C/V) exactly when s·V < r².
        let borrow = wide_mul(rest.unsigned_abs(), self.v.unsigned_abs())
            < wide_mul(r.unsigned_abs(), r.unsigned_abs());
        // `borrow` needs r > 0, so V >= 2 and |whole| <= |i128::MIN| / 2: the
        // subtraction cannot saturate; saturating only so it cannot panic.
        let variance = whole.saturating_sub(i128::from(borrow));
        // Negative variance is impossible for any folded state (C >= 0); it is
        // reachable only from a hostile private state. Clamp rather than sqrt a
        // negative, and never report a negative sigma.
        let sigma_scaled = isqrt_i128(variance.max(0));
        i64::try_from(sigma_scaled.div_euclid(3)).ok()
    }

    /// Fold one bar into the accumulators.
    ///
    /// # Errors
    ///
    /// [`Refused::Corrupt`] for a record that is not a bar — which now includes a
    /// negative volume, because this module is the ninth of nine and detecting it here
    /// left the other eight already folded. See `Candle::check`.
    ///
    /// [`Refused::AccumulatorTooLarge`] when the running sum would leave the range this
    /// module can prove safe. That one cannot move onto `Candle`: it is a property of the
    /// accumulated state, not of the record, so it is the one refusal that can still
    /// arrive after eight modules have folded. It is unreachable from any price a market
    /// prints — it needs a per-field price near 10^10 — and that is the argument for
    /// leaving it here rather than a proof that it cannot happen.
    pub fn fold(&mut self, bar: &Candle) -> Result<(), Refused> {
        // One definition, shared with the other eight modules — see
        // `Candle::check_evaluable`. This said `Candle::check` and the sentence
        // had stopped being true: the zero-price refusal reached the aggregate
        // and not the modules, so this one accepted a bar `Evaluator::stepped`
        // refused. `every_module_refuses_exactly_what_the_evaluator_refuses`
        // named this module last of the seven.
        // Wrapped rather than re-derived, so a new refusal added there reaches here.
        bar.check_evaluable().map_err(Refused::Corrupt)?;
        let day = crate::ist_day(bar.ts_micros);
        if day != self.session_day || !self.live {
            self.session_day = day;
            self.live = true;
            self.pv = 0;
            self.v = 0;
            self.p2v = 0;
            self.contributing = 0;
        }
        if self.availability == Availability::Absent || bar.volume == 0 {
            // A zero-volume bar contributes nothing and is not an error: §7 says
            // zero is a real zero.
            return Ok(());
        }
        let price = i128::from(bar.high) + i128::from(bar.low) + i128::from(bar.close);
        let volume = i128::from(bar.volume);

        // EVERY multiply is checked BEFORE anything is compared. A ceiling cannot
        // bound a product that has already wrapped, and the earlier version
        // compared `self.p2v + price * price * volume` to ACC_CEILING *after*
        // computing it: with high = low = close = 5e18 (each inside i64, and
        // `store::format::ohlc_is_sane` bounds the fields' ORDER and their SIGN,
        // never their MAGNITUDE — 5e18 is positive and well ordered, so it passes)
        // price is 1.5e19 and price² is 2.25e38, past i128::MAX. In release that
        // wrapped to a large NEGATIVE value, which is not > ACC_CEILING, so the
        // guard passed and `fold` returned Ok(()) with a poisoned accumulator —
        // the "fallback that hides a failure" §4 bans. In debug the same input
        // panicked rather than returning the documented `Refused`. Both are wrong;
        // `Refused::AccumulatorTooLarge` was unreachable for the inputs that
        // actually overflow and only fired in the window that was already safe.
        let squared = price
            .checked_mul(price)
            .ok_or(Refused::AccumulatorTooLarge)?;
        let weighted = squared
            .checked_mul(volume)
            .ok_or(Refused::AccumulatorTooLarge)?;
        let squared_total = self
            .p2v
            .checked_add(weighted)
            .ok_or(Refused::AccumulatorTooLarge)?;
        if squared_total > ACC_CEILING {
            return Err(Refused::AccumulatorTooLarge);
        }
        // pv and v were unchecked too. pv stayed inside i128 only through the
        // unstated invariant |price| >= 1, which nothing enforced.
        let price_volume_total = price
            .checked_mul(volume)
            .and_then(|t| self.pv.checked_add(t))
            .ok_or(Refused::AccumulatorTooLarge)?;
        let volume_total = self
            .v
            .checked_add(volume)
            .ok_or(Refused::AccumulatorTooLarge)?;

        self.pv = price_volume_total;
        self.v = volume_total;
        self.p2v = squared_total;
        self.contributing = self.contributing.saturating_add(1);
        Ok(())
    }

    /// The full per-bar step: **fold, then emit.**
    ///
    /// VWAP is a running average *including* the newest bar — a description of
    /// where price has traded, not a reference fixed before it. Same reasoning as
    /// [`crate::session`], opposite of the anchor families.
    ///
    /// # Errors
    ///
    /// As [`Self::fold`].
    pub fn step(&mut self, bar: &Candle, tolerance: Tolerance) -> Result<ConditionMask, Refused> {
        self.fold(bar)?;
        Ok(self.bits(bar.close, tolerance))
    }

    /// The 20 positions for one closing price.
    ///
    /// # Cost
    ///
    /// Two Euclidean divisions for the mean's quotient and remainder, two for the
    /// centred sum's, two fixed-size 256-bit products for the floor correction
    /// (D-0940), one bounded square root, then 20 comparisons. Every count is a
    /// compile-time constant; the square root's cost is bounded, not flat
    /// (`docs/06-limits.md`).
    #[must_use]
    pub fn bits(&self, close: i64, tolerance: Tolerance) -> ConditionMask {
        let mut mask = ConditionMask::ZERO;
        let Some(vwap) = self.value() else {
            return mask;
        };
        let Some(sigma) = self.sigma() else {
            return mask;
        };

        // 52/53 are the shipped pair and 143/144 the appended one. They are the
        // same predicate under the design source, which an audit flagged: both are
        // set, because retiring a shipped position needs a decisions entry and a
        // VOCAB_VERSION bump, and quietly setting only one would make the shipped
        // pair silently dead instead.
        if close > vwap {
            mask = set(mask, 52);
            mask = set(mask, 143);
        }
        if close < vwap {
            mask = set(mask, 53);
            mask = set(mask, 144);
        }
        // `near_vwap_session` is a band on the SIGMA, which is this family's own
        // scale — not a session range and not a CPR width.
        mask = near(mask, 145, tolerance, close, vwap, sigma);

        // Zipped, not indexed. The multiple and the five positions it names are one
        // row of one table, so a band cannot exist in one list and be missing from
        // the other — which is what the `Option` this loop used to unwrap was
        // guarding against, with an `else { continue }` no run could execute.
        for (multiple, (above, below, near_up, near_down, inside)) in
            BAND_SIGMA.iter().zip(BAND_POSITIONS)
        {
            // This one IS reachable, and `a_band_whose_offset_leaves_i64_emits_nothing`
            // reaches it: sigma is an i64 and `3 * sigma` need not be. A band that
            // cannot be expressed on the price scale emits **nothing** rather than a
            // saturated level, because a saturated level is a comparison against a
            // number the market never printed.
            let Some((upper, lower)) = band_levels(vwap, sigma, *multiple) else {
                continue;
            };
            if close > upper {
                mask = set(mask, above);
            }
            if close < lower {
                mask = set(mask, below);
            }
            if lower <= close && close <= upper {
                mask = set(mask, inside);
            }
            mask = near(mask, near_up, tolerance, close, upper, sigma);
            mask = near(mask, near_down, tolerance, close, lower, sigma);
        }
        mask
    }

    /// Which of the same 20 predicates can be evaluated, including false ones.
    /// Near availability uses the same vocabulary check and sigma scale as truth.
    /// Zero sigma leaves exact comparisons known and near comparisons unknown.
    pub(crate) fn known(&self, tolerance: Tolerance) -> ConditionMask {
        let mut known = ConditionMask::ZERO;
        let (Some(vwap), Some(sigma)) = (self.value(), self.sigma()) else {
            return known;
        };
        for position in [52, 53, 143, 144] {
            known = known.with_bit(position);
        }
        known = near(known, 145, tolerance, vwap, vwap, sigma);
        for (multiple, (above, below, near_up, near_down, inside)) in
            BAND_SIGMA.iter().zip(BAND_POSITIONS)
        {
            let Some((upper, lower)) = band_levels(vwap, sigma, *multiple) else {
                continue;
            };
            for position in [above, below, inside] {
                known = known.with_bit(u32::from(position));
            }
            known = near(known, near_up, tolerance, upper, upper, sigma);
            known = near(known, near_down, tolerance, lower, lower, sigma);
        }
        known
    }
}

/// `a · b` as a 256-bit `(high, low)` pair, for operands below `2^127`.
///
/// [`Vwap::sigma`] has to compare `s · V` against `r²`, and with `V` past `2^64`
/// — reachable: two bars at `i64::MAX` volume — either product can leave `i128`.
/// Four 64-bit partial products, no loop, no branch: a fixed cost for every
/// input. The tuple compares lexicographically, high word first, which is the
/// order of the 256-bit values. Exact for every `a, b < 2^128`; the caller's
/// operands are non-negative `i128`s, so below `2^127`.
const fn wide_mul(a: u128, b: u128) -> (u128, u128) {
    const LOW: u128 = u64::MAX as u128;
    let (a_hi, a_lo) = (a >> 64, a & LOW);
    let (b_hi, b_lo) = (b >> 64, b & LOW);
    let low_low = a_lo * b_lo;
    let high_low = a_hi * b_lo;
    let low_high = a_lo * b_hi;
    let high_high = a_hi * b_hi;
    // Each term is below 2^64, so three of them cannot leave u128, and the two
    // halves of `low` occupy disjoint bits, so their sum cannot carry.
    let middle = (low_low >> 64) + (high_low & LOW) + (low_high & LOW);
    let low = (low_low & LOW) + ((middle & LOW) << 64);
    let high = high_high + (high_low >> 64) + (low_high >> 64) + (middle >> 64);
    (high, low)
}

/// Truth and availability share exactly the same representable band bounds.
fn band_levels(vwap: i64, sigma: i64, multiple: i128) -> Option<(i64, i64)> {
    let offset = i64::try_from(multiple.checked_mul(i128::from(sigma))?).ok()?;
    Some((vwap.checked_add(offset)?, vwap.checked_sub(offset)?))
}

/// `(above, below, near_upper, near_lower, inside)` for each entry of
/// [`BAND_SIGMA`], in that order.
///
/// Written out because the shipped table does **not** number them regularly: band
/// 1's five positions are 146, 147, 150, 151, 152, band 2's are 148, 149, 190,
/// 191, 192, and band 3's are 193, 194, 195, 196, 197. Deriving them by arithmetic
/// from a base is one off-by-one away from a mask that means a different band.
///
/// # Why a table and not the `band_positions(band: usize) -> Option<...>` it was
///
/// The function ended in `_ => None`, and that arm could not run: the only caller
/// asked for 0, 1 and 2 because the index came from `BAND_SIGMA.iter().enumerate()`.
/// So did the `else { continue }` that unwrapped its answer. Two branches that
/// cannot execute while the code is correct are two branches no test can cover and
/// no reader can trust — and llvm-cov counts both. Zipping the table against
/// [`BAND_SIGMA`] cannot be asked for a fourth band at all, which is the same
/// guarantee with nothing dead left behind. The length is written as
/// `BAND_SIGMA.len()` so a fourth multiplier added without a fourth row is a
/// compile error rather than a silently skipped band.
const BAND_POSITIONS: [(u16, u16, u16, u16, u16); BAND_SIGMA.len()] = [
    (146, 147, 150, 151, 152),
    (148, 149, 190, 191, 192),
    (193, 194, 195, 196, 197),
];

fn set(mask: ConditionMask, index: u16) -> ConditionMask {
    vocab::table::set_exact(mask, index).unwrap_or(mask)
}

fn near(
    mask: ConditionMask,
    index: u16,
    tolerance: Tolerance,
    close: i64,
    level: i64,
    scale: i64,
) -> ConditionMask {
    vocab::table::set_near(mask, index, tolerance, close, level, scale).unwrap_or(mask)
}

/// Every position this module can set.
#[must_use]
pub const fn positions() -> [u16; 20] {
    [
        52, 53, // the shipped pair
        143, 144, 145, // session VWAP
        146, 147, 150, 151, 152, // band 1
        148, 149, 190, 191, 192, // band 2
        193, 194, 195, 196, 197, // band 3
    ]
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes — see \
              crates/costs/src/money.rs and crates/indicators/src/orb.rs: a test that \
              cannot panic cannot fail. `expect` and not the \
              `let ... else { unreachable!() }` this module used to spell, because \
              `unreachable!` expands to a panic inside THIS crate and is therefore a \
              coverage region no green run can ever execute, while `expect` panics \
              inside the standard library and leaves no such region behind."
)]
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        vocab::tolerance::pinned_fib().expect("the fib width is pinned")
    }

    /// An accumulator whose fields are set directly, without a fold.
    ///
    /// Every state built with this is one [`Vwap::fold`] cannot produce, because
    /// [`ACC_CEILING`] bounds what a fold leaves behind. That is the point: the four
    /// `checked_` guards on the fold path and the two in [`Vwap::sigma`] are
    /// documented as mechanisms rather than as arguments from that ceiling, and the
    /// only way to execute a mechanism whose precondition the surrounding code makes
    /// unreachable is to build the precondition. `session_day` is set from the
    /// timestamp of the bar that will be folded, and `live` is true, so `fold` does
    /// **not** reset the accumulators before reaching the guard under test.
    /// **`isqrt` of a perfect square is exact, and the step-down must not overrun.**
    ///
    /// The exit is `next >= guess`. Mutated to `next > guess` the loop keeps
    /// stepping at the floor -- the count jumps to the cap -- and the root is
    /// unchanged, so the counts below are what catch it. Mutated so the exit never
    /// fires, or the seed starts below the root, the floor itself moves.
    #[test]
    fn isqrt_is_exact_on_perfect_squares_and_does_not_overshoot_down() {
        for n in [1_i128, 2, 3, 12, 100, 1_000, 46_341, 1_000_000] {
            let (root, steps) = isqrt_i128_counted(n * n);
            assert_eq!(root, n, "isqrt({}) must be exactly {n}", n * n);
            assert!(steps <= 8, "isqrt({}) took {steps}", n * n);
        }
        // And one either side of a square, so the floor is a floor.
        let (below, _) = isqrt_i128_counted(143);
        let (above, _) = isqrt_i128_counted(145);
        assert_eq!(below, 11, "floor below 144");
        assert_eq!(above, 12, "floor above 144");
    }

    /// **W3-indicators2-1: `k^2 - 1` no longer oscillates to the cap.**
    ///
    /// The old loop stopped on `guess == previous`, and at every `v = k^2 - 1`
    /// Newton alternates between `k - 1` and `k`, so it ran all 128 steps and a
    /// step-down repaired the root: 128 or 129 iterations at every one of these,
    /// measured on the old code. Each must now be exact in at most 8.
    #[test]
    fn one_below_a_square_is_exact_in_a_handful_of_steps() {
        for k in [2_i128, 3, 12, 975, 1_000_000_000_000_000, i128::MAX.isqrt()] {
            let v = k * k - 1;
            let (root, steps) = isqrt_i128_counted(v);
            assert_eq!(root, k - 1, "isqrt({v})");
            assert!(steps <= 8, "isqrt({v}) took {steps} iterations");
        }
    }

    /// Every input in `1..=10^6`, against the standard library, and the most
    /// iterations any of them takes -- the figure the docs quote as measured.
    #[test]
    fn every_input_to_a_million_is_exact_and_takes_at_most_five_steps() {
        let mut worst = 0_u32;
        for v in 1..=1_000_000_i128 {
            let (root, steps) = isqrt_i128_counted(v);
            assert_eq!(root, v.isqrt(), "isqrt({v})");
            worst = worst.max(steps);
        }
        assert_eq!(worst, 5, "the measured worst case below 10^6 moved");
        for v in [
            i128::MAX,
            i128::MAX - 1,
            1_i128 << 126,
            (1_i128 << 126) - 1,
            2,
            3,
            4,
        ] {
            let (root, steps) = isqrt_i128_counted(v);
            assert_eq!(root, v.isqrt(), "isqrt({v})");
            assert!(steps <= 8, "isqrt({v}) took {steps}");
        }
    }

    /// **The iteration ceiling is the Newton budget, and nothing else.**
    ///
    /// `ITERATION_CEILING` is what the ratio bench compares a measured step count
    /// against. The decreasing iteration needs no step-down, so the ceiling is the
    /// Newton budget alone: twice the measured worst case of 8.
    #[test]
    fn the_iteration_ceiling_is_the_newton_budget() {
        assert_eq!(ITERATION_CEILING, NEWTON_STEPS);
        assert_eq!(ITERATION_CEILING, 16, "twice the measured worst case");
    }

    /// **A close exactly ON the VWAP is neither above it nor below it.**
    ///
    /// `if close > vwap` and `if close < vwap` are strict, and both mutate to
    /// their non-strict forms. Under either mutation a close sitting exactly on
    /// the VWAP sets a directional bit, so the two positions stop being mutually
    /// exclusive — a bar reported as both above and below.
    #[test]
    fn a_close_exactly_on_the_vwap_sets_neither_direction() {
        // `pv` is held on the ×3 scale — `hlc3` as high+low+close — so the VWAP
        // is `pv / (3v)`. For an exact 100 at v = 10 that is pv = 3000, and a
        // session where every price was 100 has p2v = 3 · v · 100² = 300,000.
        // The first draft of this test used pv = 1000 and asserted a VWAP of
        // 100; the real value was 33, and the test failed for that reason rather
        // than finding a defect. The scale is now written down here.
        let vw = primed(at(0, 100, 1).ts_micros, 3_000, 10, 300_000);
        let bits = vw.bits(100, tol());
        assert!(!bits.get(52) && !bits.get(143), "not above: {bits:?}");
        assert!(!bits.get(53) && !bits.get(144), "not below: {bits:?}");
        // One paisa either side does set exactly one side.
        assert!(vw.bits(101, tol()).get(52), "a paisa above is above");
        assert!(vw.bits(99, tol()).get(53), "a paisa below is below");
    }

    /// **Sigma is absent when availability is absent, whatever the volume.**
    ///
    /// The guard is `availability == Absent || v <= 0`. Mutated to `&&` it
    /// demands BOTH, so an evaluator that was told the session's volume is not
    /// trustworthy would still publish a standard deviation from it — the exact
    /// reading `Availability::Absent` exists to withhold.
    #[test]
    fn sigma_is_withheld_when_availability_is_absent_even_with_volume() {
        let mut vw = primed(at(0, 100, 1).ts_micros, 1_000, 10, 100_000);
        assert!(vw.sigma().is_some(), "present and positive volume answers");
        vw.availability = Availability::Absent;
        assert!(
            vw.sigma().is_none(),
            "absent availability withholds sigma even though v > 0"
        );
    }

    fn primed(ts_micros: i64, pv: i128, v: i128, p2v: i128) -> Vwap {
        Vwap {
            availability: Availability::Present,
            session_day: crate::ist_day(ts_micros),
            live: true,
            pv,
            v,
            p2v,
            contributing: MIN_FOR_SIGMA,
        }
    }

    #[test]
    fn all_twenty_known_positions_follow_their_actual_reference_and_tolerance() {
        let mut v = Vwap::for_slice(Availability::Present);
        assert_eq!(v.known(tol()), ConditionMask::ZERO);
        v.fold(&at(0, 200, 1)).expect("first generated observation");
        assert_eq!(v.known(tol()), ConditionMask::ZERO);
        v.fold(&at(1, 300, 1))
            .expect("second generated observation");
        assert_eq!(v.value(), Some(250));
        assert_eq!(v.sigma(), Some(50));
        let all = positions()
            .into_iter()
            .fold(ConditionMask::ZERO, |mask, bit| {
                mask.with_bit(u32::from(bit))
            });
        assert_eq!(v.known(tol()), all);
        let near_bits = [145, 150, 151, 190, 191, 195, 196];
        let wrong = vocab::tolerance::pinned_pivot().expect("other tolerance family");
        for bit in positions().map(u32::from) {
            assert_eq!(v.known(wrong).get(bit), !near_bits.contains(&bit));
        }
        let mut observed_true = ConditionMask::ZERO;
        let mut observed_false = ConditionMask::ZERO;
        for close in 1..=450 {
            let truth = v.bits(close, tol());
            for bit in positions().map(u32::from) {
                if truth.get(bit) {
                    observed_true = observed_true.with_bit(bit);
                } else {
                    observed_false = observed_false.with_bit(bit);
                }
            }
        }
        assert_eq!(
            observed_true, all,
            "every mapped predicate must be reachable"
        );
        assert_eq!(observed_false, all, "every mapped predicate can be false");
        v.availability = Availability::Absent;
        assert_eq!(v.known(tol()), ConditionMask::ZERO);
    }

    #[test]
    fn zero_dispersion_knows_exact_comparisons_but_cannot_certify_near_false() {
        let mut v = Vwap::for_slice(Availability::Present);
        v.fold(&at(0, 100, 1)).expect("first generated observation");
        v.fold(&at(1, 100, 1))
            .expect("second generated observation");
        let near_bits = [145, 150, 151, 190, 191, 195, 196];
        let known = v.known(tol());
        assert_eq!(known.popcount(), 13);
        for bit in positions().map(u32::from) {
            assert_eq!(known.get(bit), !near_bits.contains(&bit));
        }
    }

    #[test]
    fn unrepresentable_band_levels_cannot_be_truth_or_known_false() {
        assert_eq!(
            band_levels(i64::MAX - 1, 1, 1),
            Some((i64::MAX, i64::MAX - 2))
        );
        assert_eq!(band_levels(i64::MAX, 1, 1), None);
        assert_eq!(
            band_levels(i64::MIN + 1, 1, 1),
            Some((i64::MIN + 2, i64::MIN))
        );
        assert_eq!(band_levels(i64::MIN, 1, 1), None);
        assert_eq!(band_levels(0, i64::MAX, 2), None);
        assert_eq!(band_levels(0, 2, i128::MAX), None);
        // Private hostile state, outside the production accumulator ceiling:
        // first two bands fit, the third offset does not.
        let v = primed(0, 3, 1, 10_i128.pow(38));
        for bit in [193, 194, 195, 196, 197] {
            assert!(!v.bits(1, tol()).get(bit));
            assert!(!v.known(tol()).get(bit));
        }
        assert!(v.known(tol()).get(146) && v.known(tol()).get(148));
    }

    fn at(minute: i64, price: i64, volume: i64) -> Candle {
        let ts = (50_000 * 1_440 + 555 + minute) * 60_000_000 - 19_800 * 1_000_000;
        Candle {
            ts_micros: ts,
            open: price,
            high: price,
            low: price,
            close: price,
            volume,
            open_interest: i64::MIN,
        }
    }

    /// Every position is live and its kind matches how this module sets it.
    #[test]
    fn the_positions_agree_with_the_vocabulary() {
        let near_positions = [145_u16, 150, 151, 190, 191, 195, 196];
        let mut seen = std::collections::BTreeSet::new();
        for index in positions() {
            assert!(seen.insert(index), "position {index} appears twice");
            // Two steps rather than one `expect`, so the failure still names the
            // position: `expect` takes a literal and the index is what a reader needs.
            let entry = vocab::table::definition(index);
            assert!(entry.is_some(), "position {index} is not in the table");
            let def = entry.expect("`is_some` on the line above");
            assert!(vocab::table::is_live(index), "position {index} is not live");
            assert_eq!(
                def.kind == vocab::Kind::Near,
                near_positions.contains(&index),
                "position {index} (`{}`) is {:?}",
                def.name,
                def.kind,
            );
        }
        assert_eq!(seen.len(), 20);
    }

    /// **A zero-volume slice makes every position false for the whole run**, and
    /// the decision is not revisited per bar.
    #[test]
    fn a_zero_volume_slice_disables_the_whole_family() {
        let bars: Vec<Candle> = (0..20).map(|m| at(m, 2_500_000 + m * 100, 0)).collect();
        assert_eq!(Vwap::availability_of(&bars), Availability::Absent);
        let mut v = Vwap::for_slice(Availability::Absent);
        for bar in &bars {
            let mask = v.step(bar, tol()).expect("a zero-volume bar is legal");
            assert!(mask.is_empty(), "an absent-volume run emitted a bit");
        }
        assert_eq!(v.value(), None);
        assert_eq!(v.sigma(), None);
    }

    /// **The case the per-bar check gets wrong.** A slice whose volume starts at
    /// zero and later is not must be `Present` for the WHOLE run, so bit 52 does
    /// not change meaning halfway.
    #[test]
    fn a_slice_that_gains_volume_is_present_from_the_first_bar() {
        let mut bars: Vec<Candle> = (0..5).map(|m| at(m, 2_500_000, 0)).collect();
        bars.extend((5..10).map(|m| at(m, 2_500_100, 1_000)));
        assert_eq!(
            Vwap::availability_of(&bars),
            Availability::Present,
            "a slice with volume anywhere must be Present everywhere",
        );
        // And the zero-volume prefix contributes nothing rather than erroring.
        let mut v = Vwap::for_slice(Availability::Present);
        for bar in bars.iter().take(5) {
            assert_eq!(v.fold(bar), Ok(()));
        }
        assert_eq!(v.value(), None, "no volume yet, so nothing to divide by");
        let sixth = bars.get(5).expect("the slice has ten bars");
        assert_eq!(v.fold(sixth), Ok(()));
        assert_eq!(v.value(), Some(2_500_100));
    }

    /// A single bar's VWAP is its own hlc3 — and its sigma is *unavailable*, not
    /// zero.
    ///
    /// This test asserted `Some(0)` and was wrong in a way the arithmetic hid. One
    /// observation genuinely has zero dispersion, so `Some(0)` is arithmetically
    /// defensible and semantically ruinous: a sigma of zero puts band 1, band 2 and
    /// band 3 all exactly on VWAP, so `close_above_vwap_band3_upper` becomes an
    /// alias for `close_above_vwap`. Two vocabulary positions that are always equal
    /// hand the sweep a k=2 pair whose support equals both k=1 supports — a
    /// "discovery" that is an artifact of a degenerate band, on the very first bar
    /// of every session.
    #[test]
    fn one_bar_has_a_vwap_but_no_measurable_dispersion() {
        let mut v = Vwap::for_slice(Availability::Present);
        assert_eq!(v.fold(&at(0, 2_500_000, 500)), Ok(()));
        assert_eq!(v.value(), Some(2_500_000), "VWAP of one bar is that bar");
        assert_eq!(
            v.sigma(),
            None,
            "one observation has no dispersion to report, and reporting 0 would \
             collapse all three bands onto VWAP"
        );
    }

    /// The second contributing bar is what makes sigma answerable.
    #[test]
    fn sigma_starts_answering_at_the_second_contributing_bar() {
        let mut v = Vwap::for_slice(Availability::Present);
        assert_eq!(v.fold(&at(0, 1_000_000, 100)), Ok(()));
        assert_eq!(v.sigma(), None, "one contributing bar");
        assert_eq!(v.fold(&at(1, 1_000_200, 100)), Ok(()));
        assert!(v.sigma().is_some(), "two contributing bars");
    }

    /// A zero-volume bar is not a contributing bar, so it cannot unlock sigma.
    #[test]
    fn a_zero_volume_bar_does_not_make_dispersion_measurable() {
        let mut v = Vwap::for_slice(Availability::Present);
        assert_eq!(v.fold(&at(0, 1_000_000, 100)), Ok(()));
        assert_eq!(
            v.fold(&at(1, 1_000_500, 0)),
            Ok(()),
            "zero volume is not an error"
        );
        assert_eq!(
            v.sigma(),
            None,
            "the second bar contributed no volume, so there is still one observation"
        );
    }

    /// The accumulator refuses the i64 extreme instead of wrapping or panicking.
    ///
    /// `high = low = close = 5e18` is inside `i64`, and `store::format`'s
    /// `ohlc_is_sane` bounds field ORDER and SIGN — never MAGNITUDE — and 5e18 is
    /// positive and well ordered, so this bar is legally writable to the store and
    /// readable back with a valid CRC. Before the
    /// checked arithmetic went in, `price * price` was 2.25e38, past `i128::MAX`:
    /// release wrapped it to a large negative value that is not `> ACC_CEILING`, so
    /// `fold` returned `Ok(())` and `value()` answered `Some(5000000000000000000)`
    /// from a poisoned accumulator. Debug panicked instead. This test would have
    /// gone red — or panicked — on either profile.
    #[test]
    fn the_i64_extreme_is_refused_rather_than_wrapped_or_panicked() {
        let mut v = Vwap::for_slice(Availability::Present);
        let extreme = Candle {
            ts_micros: 0,
            open: 5_000_000_000_000_000_000,
            high: 5_000_000_000_000_000_000,
            low: 5_000_000_000_000_000_000,
            close: 5_000_000_000_000_000_000,
            volume: 1,
            open_interest: i64::MIN,
        };
        assert_eq!(v.fold(&extreme), Err(Refused::AccumulatorTooLarge));
        assert_eq!(v.value(), None, "a refused bar contributes nothing");
        assert_eq!(v.sigma(), None);
    }

    /// `isqrt_i128` agrees with the standard library across the whole exponent
    /// range, including where the old 64-step cap silently gave up.
    #[test]
    fn the_square_root_is_exact_where_the_old_cap_was_not() {
        for e in 0..38_u32 {
            let v = 10_i128.pow(e);
            assert_eq!(isqrt_i128(v), v.isqrt(), "10^{e}");
        }
        for v in [
            0,
            1,
            2,
            3,
            8,
            ACC_CEILING - 1,
            ACC_CEILING,
            ACC_CEILING + 1,
            i128::MAX - 1,
            i128::MAX,
        ] {
            assert_eq!(isqrt_i128(v), v.max(0).isqrt(), "v = {v}");
        }
    }

    /// Never over-reports: the square of the answer never exceeds the input.
    #[test]
    fn the_square_root_never_over_reports() {
        for v in [1_i128, 2, 3, 8, 99, 10_i128.pow(20), ACC_CEILING, i128::MAX] {
            let r = isqrt_i128(v);
            assert!(r.saturating_mul(r) <= v, "over-reported for {v}");
        }
    }

    /// Two equal-volume bars at different prices average to the midpoint, and
    /// sigma is half the spread.
    #[test]
    fn two_bars_average_and_disperse_as_expected() {
        let mut v = Vwap::for_slice(Availability::Present);
        assert_eq!(v.fold(&at(0, 1_000_000, 100)), Ok(()));
        assert_eq!(v.fold(&at(1, 1_000_200, 100)), Ok(()));
        assert_eq!(v.value(), Some(1_000_100));
        assert_eq!(v.sigma(), Some(100), "sigma of ±100 about the mean");
    }

    /// Volume weighting actually weights: the heavier bar pulls the mean.
    #[test]
    fn volume_weights_the_mean() {
        let mut v = Vwap::for_slice(Availability::Present);
        assert_eq!(v.fold(&at(0, 1_000_000, 1)), Ok(()));
        assert_eq!(v.fold(&at(1, 1_000_300, 999)), Ok(()));
        let value = v.value().expect("volume is present");
        // The exact weighted mean is 1,000,299.7 and the division floors, so
        // 1,000,299 is the correct answer — not 1,000,300. Asserting `> 1_000_299`
        // was off by one against my own arithmetic.
        assert_eq!(value, 1_000_299, "the heavy bar did not dominate");
        assert!(value > 1_000_290, "the light bar dominated instead");
    }

    /// A negative volume is corruption, not a zero.
    #[test]
    fn a_negative_volume_is_refused() {
        let mut v = Vwap::for_slice(Availability::Present);
        // The refusal now arrives through `Candle::check`, wrapped. The dedicated
        // `Refused::NegativeVolume` variant was REMOVED rather than kept: `check` runs
        // first, so nothing could ever construct it, and an error variant that cannot be
        // reached is a door onto nothing.
        assert_eq!(
            v.fold(&at(0, 1_000_000, -1)),
            Err(Refused::Corrupt(crate::Corrupt::NegativeVolume))
        );
    }

    /// The accumulator refuses rather than wraps.
    #[test]
    fn an_oversized_accumulator_refuses() {
        let mut v = Vwap::for_slice(Availability::Present);
        // Reaching the ceiling needs numbers no market produces, and that is the
        // point: my first attempt used a 10^7-paisa price and 10^18 volume, which
        // is 9 x 10^32 — INSIDE the 10^34 ceiling, so the test passed nothing.
        // 10^10 paisa (a hundred-million-point index) with 10^14 volume is
        // 9 x 10^34, past the ceiling and still 10^3 inside i128.
        let huge = Candle {
            ts_micros: 0,
            open: 10_000_000_000,
            high: 10_000_000_000,
            low: 10_000_000_000,
            close: 10_000_000_000,
            volume: 10_i64.pow(14),
            open_interest: i64::MIN,
        };
        assert_eq!(v.fold(&huge), Err(Refused::AccumulatorTooLarge));
        assert_eq!(v.value(), None, "a refused bar was folded in");
    }

    /// A realistic BANKNIFTY session does NOT approach the ceiling — the bound is
    /// not so tight that real data trips it.
    #[test]
    fn a_realistic_session_stays_far_inside_the_ceiling() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..375 {
            // 57,000 index = 5.7e6 paisa, 1e7 volume per minute is generous.
            let bar = at(minute, 5_700_000 + minute * 10, 10_000_000);
            assert_eq!(
                v.fold(&bar),
                Ok(()),
                "minute {minute} refused on real-scale data"
            );
        }
        assert!(v.value().is_some());
        assert!(v.sigma().is_some());
    }

    /// The integer square root is exact on squares and never over-reports.
    #[test]
    fn the_integer_square_root_never_over_reports() {
        for n in [0_i128, 1, 2, 3, 4, 8, 9, 15, 16, 10_000, 999_999, 1_000_000] {
            let r = isqrt_i128(n);
            assert!(r * r <= n, "isqrt({n}) = {r} over-reports");
            assert!((r + 1) * (r + 1) > n, "isqrt({n}) = {r} under-reports");
        }
        assert_eq!(isqrt_i128(-5), 0, "a negative input has no root");
        // And it terminates on a very large value.
        let big = 10_i128.pow(30);
        let r = isqrt_i128(big);
        assert_eq!(r, 10_i128.pow(15));
    }

    /// Exactly one of above / below / inside holds per band.
    #[test]
    fn exactly_one_relation_holds_per_band() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..60 {
            let price = 2_500_000 + (minute * 137) % 4_000;
            assert!(
                v.step(&at(minute, price, 1_000 + minute), tol()).is_ok(),
                "minute {minute} at {price} was refused",
            );
        }
        for close in [2_400_000_i64, 2_500_000, 2_502_000, 2_600_000] {
            let mask = v.bits(close, tol());
            for (band, (above, below, _, _, inside)) in BAND_POSITIONS.into_iter().enumerate() {
                let n = u32::from(mask.get(u32::from(above)))
                    + u32::from(mask.get(u32::from(below)))
                    + u32::from(mask.get(u32::from(inside)));
                assert_eq!(n, 1, "band {band} at close {close} had {n} relations");
            }
        }
    }

    /// **A close exactly ON a band edge is INSIDE it, not outside.**
    ///
    /// `if close > upper` and `if close < lower` are strict, and the inside test
    /// below them is `lower <= close && close <= upper` — inclusive on both
    /// sides. Mutating either strict comparison to its non-strict form makes the
    /// edge belong to **both** relations at once, so a bar sitting exactly one
    /// sigma above the VWAP reports itself as above the band AND inside it.
    ///
    /// `exactly_one_relation_holds_per_band` already asserts the exclusivity,
    /// and it did not catch this: its four closes are round numbers that never
    /// land on a computed edge. The edge has to be COMPUTED from the live VWAP
    /// and sigma to be tested, which is what this does.
    #[test]
    fn a_close_exactly_on_a_band_edge_is_inside_that_band() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..60 {
            let price = 2_500_000 + (minute * 137) % 4_000;
            assert!(v.step(&at(minute, price, 1_000 + minute), tol()).is_ok());
        }
        let vwap = v.value().expect("a primed session has a vwap");
        let sigma = v.sigma().expect("and a sigma");
        assert!(
            sigma > 0,
            "the fixture must have spread, or the edges collapse"
        );

        for (band, (multiple, (above, below, _, _, inside))) in
            BAND_SIGMA.iter().zip(BAND_POSITIONS).enumerate()
        {
            let offset = i64::try_from(multiple * i128::from(sigma)).expect("fits");
            for (edge, name) in [
                (vwap.saturating_add(offset), "upper"),
                (vwap.saturating_sub(offset), "lower"),
            ] {
                let mask = v.bits(edge, tol());
                assert!(
                    mask.get(u32::from(inside)),
                    "band {band}: a close on the {name} edge is inside it"
                );
                assert!(
                    !mask.get(u32::from(above)) && !mask.get(u32::from(below)),
                    "band {band}: a close on the {name} edge is not outside it"
                );
            }
        }
    }

    /// The bands nest: inside band 1 implies inside bands 2 and 3.
    #[test]
    fn the_bands_nest_outward() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..90 {
            let price = 2_500_000 + (minute * 211) % 6_000;
            assert!(
                v.step(&at(minute, price, 5_000), tol()).is_ok(),
                "minute {minute} at {price} was refused",
            );
        }
        for step in -60..60 {
            let close = 2_500_000 + step * 137;
            let mask = v.bits(close, tol());
            if mask.get(152) {
                assert!(mask.get(192), "inside band 1 but not band 2");
                assert!(mask.get(197), "inside band 1 but not band 3");
            }
            if mask.get(192) {
                assert!(mask.get(197), "inside band 2 but not band 3");
            }
        }
    }

    /// Only the 20 positions this module owns are ever set.
    #[test]
    fn nothing_outside_the_twenty_positions_is_set() {
        let owned = positions();
        let mut v = Vwap::for_slice(Availability::Present);
        let mut union = ConditionMask::ZERO;
        for minute in 0..375 {
            let price = 2_500_000 + (minute * 173) % 8_000;
            let mask = v
                .step(&at(minute, price, 1_000 + minute * 3), tol())
                .expect("a sane bar");
            union = union.union(&mask);
        }
        // THE UNION MUST NOT BE EMPTY, AND THIS LINE IS THE WHOLE TEST.
        //
        // Everything below is a SUBSET check: for each set bit, is it ours.
        // `union` starts at `ConditionMask::ZERO`, so an empty union satisfies
        // it vacuously -- the guard never fires and the assertion never runs.
        // Stub this module's emit to `ConditionMask::ZERO` and the test passes,
        // which `CLAUDE.md` §4 bans outright: a test that asserts nothing.
        //
        // `crates/indicators/src/lib.rs` diagnosed exactly this defect and
        // added a positive companion for two modules. Five others, this one
        // among them, were left with the vacuous form.
        assert!(
            union.popcount() > 0,
            "the fixture set NO position, so the subset check below is vacuous \
             and a stub returning ConditionMask::ZERO would pass this test"
        );
        for index in 0..ConditionMask::BITS {
            if union.get(index) {
                let as_u16 = u16::try_from(index).unwrap_or(u16::MAX);
                assert!(
                    owned.contains(&as_u16),
                    "position {index} is not this module's"
                );
            }
        }
    }

    /// A new IST day restarts the accumulators.
    #[test]
    fn a_new_session_restarts_the_accumulators() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..10 {
            assert!(
                v.step(&at(minute, 9_000_000, 1_000), tol()).is_ok(),
                "minute {minute} was refused",
            );
        }
        assert_eq!(v.value(), Some(9_000_000));
        let mut next = at(0, 1_000_000, 1_000);
        next.ts_micros += 1_440 * 60_000_000;
        assert!(
            v.step(&next, tol()).is_ok(),
            "the first bar of the next session was refused",
        );
        assert_eq!(v.value(), Some(1_000_000), "yesterday's volume survived");
    }

    /// Same bars, same bits, byte for byte.
    #[test]
    fn the_same_session_twice_gives_the_same_bits() {
        let run = || {
            let mut v = Vwap::for_slice(Availability::Present);
            (0..300)
                .map(|minute| {
                    let price = 2_500_000 + (minute * 149) % 5_000;
                    v.step(&at(minute, price, 700 + minute), tol())
                        .expect("a sane bar")
                        .words()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run(), "the same bars must give the same bits");
        // The equality above holds for ANY deterministic body -- including
        // `bits() -> ConditionMask::ZERO` and any constant -- so it cannot see the one
        // mutation that matters. An audit applied exactly that mutation and it survived
        // ALL SIX of these idempotence tests, which between them are §3 rule 5's only
        // direct guard in this crate.
        //
        // This closes it without needing to know what the bits SHOULD be, which is what
        // the sibling tests in this module are for. A body that ignores its input emits
        // the same value on every bar, and these fixtures deliberately vary the bars. So:
        // the run must not be constant.
        let observed = run();
        assert!(
            observed
                .iter()
                .skip(1)
                .zip(observed.iter())
                .any(|(later, earlier)| later != earlier),
            "every bar produced an identical result, so this test would pass on a body \
             that ignores its input entirely"
        );
    }

    /// The shipped pair and the appended pair are set together, deliberately.
    #[test]
    fn the_shipped_and_appended_pairs_agree() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..30 {
            assert!(
                v.step(&at(minute, 2_500_000 + minute * 50, 1_000), tol())
                    .is_ok(),
                "minute {minute} was refused",
            );
        }
        for close in [2_400_000_i64, 2_500_700, 2_600_000] {
            let mask = v.bits(close, tol());
            assert_eq!(mask.get(52), mask.get(143), "52 and 143 disagreed");
            assert_eq!(mask.get(53), mask.get(144), "53 and 144 disagreed");
        }
    }

    /// The verdict handed to [`Vwap::for_slice`] is what the accumulator reports,
    /// and folding never revises it.
    ///
    /// Nothing else in this file reads [`Vwap::availability`], so without this the
    /// accessor could return the wrong variant — or, worse, start answering
    /// `Present` the moment a volume-bearing bar arrived — and every other test here
    /// would still be green. The per-run decision in the module documentation is
    /// only a decision if the accumulator keeps it, and this is the only test that
    /// asks the accumulator what it thinks.
    #[test]
    fn the_availability_verdict_is_reported_and_never_revised() {
        let mut present = Vwap::for_slice(Availability::Present);
        assert_eq!(present.availability(), Availability::Present);
        assert_eq!(present.fold(&at(0, 1_000_000, 0)), Ok(()));
        assert_eq!(
            present.availability(),
            Availability::Present,
            "a zero-volume bar must not downgrade a Present run",
        );

        let mut absent = Vwap::for_slice(Availability::Absent);
        assert_eq!(absent.availability(), Availability::Absent);
        assert_eq!(absent.fold(&at(0, 1_000_000, 5_000)), Ok(()));
        assert_eq!(
            absent.availability(),
            Availability::Absent,
            "a volume-bearing bar must not upgrade an Absent run halfway",
        );
        assert_eq!(
            absent.value(),
            None,
            "and the volume it carried must not have been folded",
        );
    }

    /// The **second** multiply on the fold path refuses, not just the first.
    ///
    /// `the_i64_extreme_is_refused_rather_than_wrapped_or_panicked` trips
    /// `price * price`, which means `squared * volume` was never executed with a
    /// product that leaves `i128` — so replacing that second `checked_mul` with a
    /// bare `*` would have wrapped in release and panicked in debug with every test
    /// still green. `4e18` per field is chosen so the first multiply survives and
    /// only the second cannot, and the test asserts that split rather than assuming
    /// it.
    #[test]
    fn the_second_multiply_refuses_when_the_first_survives() {
        let field = 4_000_000_000_000_000_000_i64;
        let price = i128::from(field) * 3;
        assert!(
            price.checked_mul(price).is_some(),
            "price^2 must stay inside i128 or this test exercises the FIRST multiply",
        );
        assert!(
            price
                .checked_mul(price)
                .and_then(|squared| squared.checked_mul(2))
                .is_none(),
            "price^2 * 2 must leave i128 or this test exercises no refusal at all",
        );
        let mut v = Vwap::for_slice(Availability::Present);
        let bar = Candle {
            ts_micros: 0,
            open: field,
            high: field,
            low: field,
            close: field,
            volume: 2,
            open_interest: i64::MIN,
        };
        assert_eq!(v.fold(&bar), Err(Refused::AccumulatorTooLarge));
        assert_eq!(v.p2v, 0, "a refused bar contributed to p2v");
        assert_eq!(v.value(), None, "a refused bar contributed nothing");
    }

    /// A full `p2v` accumulator refuses the next bar instead of wrapping.
    ///
    /// [`ACC_CEILING`] keeps `fold` from ever reaching this state, which is exactly
    /// why the `checked_add` guarding it was never executed: the ceiling is an
    /// argument and the `checked_add` is the mechanism, and only the mechanism runs.
    /// A bare `self.p2v + weighted` here wraps to a plausible number in release and
    /// panics in debug.
    #[test]
    fn a_full_squared_accumulator_refuses_the_next_bar() {
        let bar = at(0, 1_000_000, 1);
        let mut v = primed(bar.ts_micros, 0, 0, i128::MAX);
        assert_eq!(v.fold(&bar), Err(Refused::AccumulatorTooLarge));
        assert_eq!(v.p2v, i128::MAX, "the refused bar was added to p2v anyway");
        assert_eq!(v.v, 0, "the refused bar was added to v anyway");
    }

    /// A full `pv` accumulator refuses the next bar instead of wrapping.
    ///
    /// The price-times-volume sum is checked *after* the ceiling test passes, so it
    /// is the one guard on the fold path that a legal bar reaches with the ceiling
    /// already satisfied. `pv` was unchecked once, under the unstated invariant
    /// `|price| >= 1` that nothing enforced.
    #[test]
    fn a_full_price_volume_accumulator_refuses_the_next_bar() {
        let bar = at(0, 1_000_000, 1);
        let mut v = primed(bar.ts_micros, i128::MAX, 0, 0);
        assert_eq!(v.fold(&bar), Err(Refused::AccumulatorTooLarge));
        assert_eq!(v.pv, i128::MAX, "the refused bar was added to pv anyway");
    }

    /// A full `v` accumulator refuses — **and the fold is all-or-nothing**.
    ///
    /// The volume sum is the last of the four checks, and the three writes happen
    /// after all four. So this is the case that proves the ordering: a bar refused
    /// on the last guard must leave `pv` and `p2v` exactly where they were. Move the
    /// assignments up beside the arithmetic that produced them — the obvious
    /// "simplification" — and this test goes red while every other test in the file
    /// stays green, because a torn accumulator only shows on the refusal path.
    #[test]
    fn a_full_volume_accumulator_refuses_and_commits_nothing() {
        let bar = at(0, 1_000_000, 1);
        let mut v = primed(bar.ts_micros, 0, i128::MAX, 0);
        assert_eq!(v.fold(&bar), Err(Refused::AccumulatorTooLarge));
        assert_eq!(v.v, i128::MAX, "the refused volume was added");
        assert_eq!(v.pv, 0, "pv was committed before the volume check refused");
        assert_eq!(
            v.p2v, 0,
            "p2v was committed before the volume check refused"
        );
    }

    /// Sigma refuses when the mean square leaves `i128`, and says nothing rather
    /// than something wrong.
    ///
    /// `mean * mean` was unchecked once. With `wrapping_mul` this state gives
    /// `mean^2 = 5.97e37`, a variance of `-5.97e37`, a clamp to zero and a confident
    /// `Some(0)` — a sigma of zero, which the module documentation spends a section
    /// explaining is the single most damaging answer this function can give. The
    /// VWAP itself is still answerable here, which is the point: the refusal is the
    /// checked multiply and not a general unavailability.
    ///
    /// Since D-0940 the guarded product is `q·(pv + r)` rather than `mean²`; on this
    /// state `r = 0` and it is the same `4e44`, refused by the same kind of check.
    #[test]
    fn sigma_refuses_when_the_mean_square_leaves_i128() {
        let v = primed(0, 20_000_000_000_000_000_000, 1, 0);
        assert_eq!(
            v.value(),
            Some(6_666_666_666_666_666_666),
            "VWAP itself is inside i64 and must still answer",
        );
        assert_eq!(
            v.sigma(),
            None,
            "the mean square left i128, so sigma must refuse rather than wrap",
        );
    }

    /// Sigma refuses when the variance subtraction leaves `i128`.
    ///
    /// A negative `p2v` is precisely what a wrapped accumulator looks like — the
    /// module's own history is an unchecked `p2v` wrapping to a large negative value
    /// and sailing under the ceiling — so this is the shape sigma has to survive
    /// even after the fold path was fixed. With `wrapping_sub` the subtraction lands
    /// on `+1.61e38`, whose root divides down to a perfectly plausible
    /// `Some(4.23e18)` in paisa: a fabricated dispersion, from a state that has no
    /// dispersion to report.
    #[test]
    fn sigma_refuses_when_the_variance_subtraction_leaves_i128() {
        let v = primed(0, 3_000_000_000_000_000_000, 1, i128::MIN);
        assert_eq!(
            v.value(),
            Some(1_000_000_000_000_000_000),
            "VWAP itself is inside i64 and must still answer",
        );
        assert_eq!(
            v.sigma(),
            None,
            "E[p^2] - E[p]^2 left i128, so sigma must refuse rather than wrap",
        );
    }

    /// A band whose offset does not fit `i64` emits **nothing** for that band, and
    /// does not disturb the bands that do fit.
    ///
    /// `sigma` is an `i64` and `3 * sigma` need not be, so the widest band is the
    /// one that can fall off the price scale. This state gives
    /// `sigma = 3_333_333_333_333_333_333`: bands 1 and 2 are representable, band 3
    /// is `9_999_999_999_999_999_999` and is not. The alternative — saturating the
    /// offset — would compare the close against `i64::MAX`, a number no market ever
    /// printed, and set `close_below_vwap_band3_upper` on every bar forever.
    #[test]
    fn a_band_whose_offset_leaves_i64_emits_nothing() {
        let v = primed(0, 3, 1, 10_i128.pow(38));
        assert_eq!(v.value(), Some(1));
        assert_eq!(
            v.sigma(),
            Some(3_333_333_333_333_333_333),
            "the fixture no longer produces the sigma this test reasons about",
        );
        let mask = v.bits(0, tol());
        assert!(mask.get(53), "the VWAP pair still answers");
        assert!(
            mask.get(152),
            "band 1 fits i64 and the close sits inside it"
        );
        assert!(
            mask.get(192),
            "band 2 fits i64 and the close sits inside it"
        );
        for index in [193_u32, 194, 195, 196, 197] {
            assert!(
                !mask.get(index),
                "band 3's offset leaves i64, so position {index} must stay unset",
            );
        }
    }

    /// The iteration count [`ITERATION_CEILING`] bounds is asserted by `cargo test`,
    /// not only by a bench.
    ///
    /// [`isqrt_i128_counted`] exists so the bound can be measured, and the only
    /// thing measuring it was `benches/ratio.rs` — which `cargo test` does not run,
    /// which coverage does not run, and which lives behind its own CI gate. The four
    /// counts below are the ones that bench prints and the module documentation
    /// quotes; a quoted number rots and an asserted one does not. A change to the
    /// seed or the exit moves these counts (D-1665).
    #[test]
    fn the_documented_iteration_counts_are_the_measured_ones() {
        for (v, want) in [
            (1_i128, 2_u32),
            (i128::from(i64::MAX), 5),
            (10_i128.pow(30), 5),
            (i128::MAX, 6),
        ] {
            let (root, steps) = isqrt_i128_counted(v);
            assert_eq!(root, v.isqrt(), "the root of {v} is not the exact one");
            assert_eq!(steps, want, "the iteration count at {v} moved");
            assert!(
                steps <= ITERATION_CEILING,
                "{v} took {steps} iterations, past the ceiling of {ITERATION_CEILING}",
            );
        }
        assert_eq!(
            isqrt_i128_counted(0),
            (0, 0),
            "zero has a root and needs no iteration to find it",
        );
        assert_eq!(
            isqrt_i128_counted(-1),
            (0, 0),
            "a negative input has no root and must not be iterated on",
        );
    }

    /// An exact, independent oracle for the weighted standard deviation in paisa.
    ///
    /// Arbitrary-precision unsigned arithmetic on 32-bit limbs — multiply, add and
    /// compare, nothing else — so it shares no code and no algebra with
    /// [`Vwap::sigma`]. It answers the largest `s >= 0` with
    /// `(3·s·V)² + pv² <= V·p2v`, i.e. `s = floor(sqrt(V·p2v − pv²) / (3·V))`:
    /// the floor of the exact volume-weighted standard deviation of `hlc3`, in
    /// paisa. `None` when that numerator is negative, which no fold can produce.
    #[allow(
        clippy::indexing_slicing,
        reason = "a limb-array oracle in a test; every index is bounded by the \
                  lengths it is computed from, and a panic is a failed test"
    )]
    mod oracle {
        fn limbs(x: u128) -> Vec<u64> {
            (0..4)
                .map(|i| u64::try_from((x >> (32 * i)) & 0xffff_ffff).expect("32 bits"))
                .collect()
        }

        fn trim(mut x: Vec<u64>) -> Vec<u64> {
            while x.last() == Some(&0) {
                x.pop();
            }
            x
        }

        fn mul(a: &[u64], b: &[u64]) -> Vec<u64> {
            let mut out = vec![0_u64; a.len() + b.len() + 1];
            for (i, &x) in a.iter().enumerate() {
                let mut carry = 0_u64;
                for (j, &y) in b.iter().enumerate() {
                    let t = out[i + j] + x * y + carry;
                    out[i + j] = t & 0xffff_ffff;
                    carry = t >> 32;
                }
                let mut k = i + b.len();
                while carry > 0 {
                    let t = out[k] + carry;
                    out[k] = t & 0xffff_ffff;
                    carry = t >> 32;
                    k += 1;
                }
            }
            trim(out)
        }

        fn add(a: &[u64], b: &[u64]) -> Vec<u64> {
            let n = a.len().max(b.len()) + 1;
            let mut out = vec![0_u64; n];
            let mut carry = 0_u64;
            for (i, slot) in out.iter_mut().enumerate() {
                let t = a.get(i).copied().unwrap_or(0) + b.get(i).copied().unwrap_or(0) + carry;
                *slot = t & 0xffff_ffff;
                carry = t >> 32;
            }
            trim(out)
        }

        fn le(a: &[u64], b: &[u64]) -> bool {
            let (a, b) = (trim(a.to_vec()), trim(b.to_vec()));
            if a.len() != b.len() {
                return a.len() < b.len();
            }
            for (x, y) in a.iter().rev().zip(b.iter().rev()) {
                if x != y {
                    return x < y;
                }
            }
            true
        }

        /// `a·b` as a `(high, low)` pair of 128-bit words, from the limbs.
        pub(super) fn product(a: u128, b: u128) -> (u128, u128) {
            let limbs = mul(&limbs(a), &limbs(b));
            let word = |from: usize| {
                (from..from + 4).rev().fold(0_u128, |acc, i| {
                    (acc << 32) | u128::from(limbs.get(i).copied().unwrap_or(0))
                })
            };
            (word(4), word(0))
        }

        /// `(a·b) <= (c·d)` exactly, for the wide-compare test.
        pub(super) fn product_le(a: u128, b: u128, c: u128, d: u128) -> bool {
            le(&mul(&limbs(a), &limbs(b)), &mul(&limbs(c), &limbs(d)))
        }

        pub(super) fn sigma(pv: i128, v: i128, p2v: i128) -> Option<i64> {
            let pv = u128::try_from(pv).ok()?;
            let v = u128::try_from(v).ok()?;
            let p2v = u128::try_from(p2v).ok()?;
            let rhs = mul(&limbs(v), &limbs(p2v));
            let pv_sq = mul(&limbs(pv), &limbs(pv));
            let fits = |s: u128| {
                let three_s_v = mul(&limbs(3 * s), &limbs(v));
                le(&add(&mul(&three_s_v, &three_s_v), &pv_sq), &rhs)
            };
            if !fits(0) {
                return None;
            }
            // Largest s in [0, 2^63) that fits: plain bisection.
            let (mut lo, mut hi) = (0_u128, 1_u128 << 63);
            while hi - lo > 1 {
                let mid = lo + (hi - lo) / 2;
                if fits(mid) {
                    lo = mid;
                } else {
                    hi = mid;
                }
            }
            i64::try_from(lo).ok()
        }
    }

    /// A deterministic generator, so every fixture is reproducible byte for byte.
    struct Lcg(u64);

    impl Lcg {
        fn next(&mut self) -> u64 {
            self.0 = self
                .0
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            self.0 >> 11
        }

        fn below(&mut self, n: u64) -> u64 {
            self.next() % n
        }
    }

    /// **Sigma is the exact floor of the weighted standard deviation, whatever the
    /// remainder of the mean** (ET-indicators-0, ET-indicators-12, F-9A880B, D-0940).
    ///
    /// Every earlier sigma fixture had a mean that divided exactly — 200/300 at
    /// volume 1, 1,000,000/1,000,200 at volume 100 — so `pv mod v` was zero and the
    /// old formula's two floors cancelled. This folds 600 generated sessions of real
    /// bars (prices from 1 paisa to 5 × 10⁸, volumes from 1 to `i64::MAX`) and checks
    /// `sigma()` after every bar against [`oracle::sigma`], and requires that most of
    /// the states it checked had a non-zero remainder. On the old
    /// `isqrt(floor(p2v/v) − floor(pv/v)²)` it fails on the first such state.
    #[test]
    fn sigma_is_the_exact_floor_whatever_the_remainder_of_the_mean() {
        let mut rng = Lcg(0x9A88_0B00_0940);
        let bases: [i64; 6] = [1, 37, 10_000, 1_000_000, 5_700_000, 500_000_000];
        let volume_caps: [u64; 4] = [1, 20_000, 1_000_000_000, i64::MAX.unsigned_abs()];
        let (mut checked, mut with_remainder, mut nonzero_sigma) = (0_u32, 0_u32, 0_u32);
        for session in 0..600_u64 {
            let mut v = Vwap::for_slice(Availability::Present);
            let base = *bases
                .get(usize::try_from(session % 6).expect("small"))
                .expect("six bases");
            let cap = *volume_caps
                .get(usize::try_from((session / 6) % 4).expect("small"))
                .expect("four caps");
            let bars = 2 + rng.below(40);
            let mut price = base;
            for minute in 0..bars {
                let step = i64::try_from(rng.below(2 * 50 + 1)).expect("small") - 50;
                price = (price + step * (base / 1_000 + 1)).max(1);
                let volume = i64::try_from(1 + rng.below(cap)).expect("below i64::MAX");
                let minute = i64::try_from(minute).expect("small");
                if v.fold(&at(minute, price, volume)).is_err() {
                    break; // past the accumulator ceiling: refused, never wrapped
                }
                if v.contributing < MIN_FOR_SIGMA {
                    continue;
                }
                let want = oracle::sigma(v.pv, v.v, v.p2v);
                assert_eq!(
                    v.sigma(),
                    want,
                    "session {session} bar {minute}: pv {} v {} p2v {}",
                    v.pv,
                    v.v,
                    v.p2v,
                );
                checked += 1;
                with_remainder += u32::from(v.pv.rem_euclid(v.v) != 0);
                nonzero_sigma += u32::from(want.is_some_and(|s| s > 0));
            }
        }
        assert!(checked > 5_000, "only {checked} states checked");
        assert!(
            with_remainder * 2 > checked,
            "only {with_remainder} of {checked} states had pv mod v != 0, \
             so this test no longer exercises the defect it exists for",
        );
        assert!(
            nonzero_sigma > 1_000,
            "only {nonzero_sigma} non-zero sigmas"
        );
    }

    /// The same oracle at the far end of the range a fold can reach: total volume
    /// past `2^64`, so `r²` and `s·V` leave `i128` and only the 256-bit comparison
    /// can decide the floor. Each state is a sum of whole bars of two prices, so it
    /// is one a fold could produce; it is primed because folding 10¹⁵ bars is not a
    /// test.
    #[test]
    fn sigma_is_exact_where_the_remainder_squared_leaves_i128() {
        let mut rng = Lcg(0x0940);
        let mut wide = 0_u32;
        for _ in 0..2_000 {
            // Two bar classes: hlc3·3 prices p1, p2 >= 3, total volumes a, b.
            let p1 = i128::from(3 + rng.below(30));
            let p2 = i128::from(3 + rng.below(3_000_000_000));
            let a = i128::from(rng.next()) * i128::from(rng.next() >> 20) + 1;
            let b_cap = (ACC_CEILING - p1 * p1 * a) / (p2 * p2);
            if b_cap < 1 {
                continue;
            }
            let b = i128::from(rng.next()).rem_euclid(b_cap) + 1;
            let (pv, v, p2v) = (p1 * a + p2 * b, a + b, p1 * p1 * a + p2 * p2 * b);
            assert!(p2v <= ACC_CEILING, "fixture left the fold's own ceiling");
            let r = pv.rem_euclid(v);
            if r.checked_mul(r).is_none() {
                wide += 1;
            }
            assert_eq!(
                primed(0, pv, v, p2v).sigma(),
                oracle::sigma(pv, v, p2v),
                "pv {pv} v {v} p2v {p2v}",
            );
        }
        assert!(wide > 500, "only {wide} states needed the wide comparison");
    }

    /// Real folds, every volume `i64::MAX`, prices one and two paisa: total volume
    /// passes `2^64` within three bars and `r²` leaves `i128` — the extreme a legal
    /// bar sequence reaches without touching [`ACC_CEILING`].
    #[test]
    fn maximal_volumes_at_minimal_prices_stay_exact() {
        let mut v = Vwap::for_slice(Availability::Present);
        let mut seen_wide = false;
        for minute in 0..200_i64 {
            let price = 1 + minute % 2 + i64::from(minute % 7 == 0);
            v.fold(&at(minute, price, i64::MAX - minute))
                .expect("far inside the ceiling");
            if v.contributing < MIN_FOR_SIGMA {
                continue;
            }
            let r = v.pv.rem_euclid(v.v);
            seen_wide |= r.checked_mul(r).is_none();
            assert_eq!(
                v.sigma(),
                oracle::sigma(v.pv, v.v, v.p2v),
                "minute {minute}"
            );
        }
        assert!(seen_wide, "no state needed the 256-bit comparison");
    }

    /// The finding's shape, reproduced from two ordinary bars (F-9A880B, D-0940).
    ///
    /// 100.00 and 100.01 rupees at volume 1 each: the exact dispersion is half a
    /// paisa, so sigma floors to **0** and all three bands sit on the VWAP of
    /// 10,000. The old formula floored the mean before squaring it and reported
    /// **57** paisa. At 57,000.00/57,000.01 it reported **1,378** for the same half
    /// paisa. A close of 10,020 is therefore ABOVE every band (146, 148, 193), where
    /// the old sigma put it INSIDE every band (152, 192, 197).
    #[test]
    fn a_half_paisa_spread_has_no_band_width() {
        let mut v = Vwap::for_slice(Availability::Present);
        v.fold(&at(0, 10_000, 1)).expect("first bar");
        v.fold(&at(1, 10_001, 1)).expect("second bar");
        assert_eq!(v.value(), Some(10_000));
        assert_eq!(v.sigma(), Some(0), "the old formula said 57");
        assert_eq!(v.sigma(), oracle::sigma(v.pv, v.v, v.p2v));
        let mask = v.bits(10_020, tol());
        for above in [146_u32, 148, 193] {
            assert!(mask.get(above), "10,020 is above band position {above}");
        }
        for inside in [152_u32, 192, 197, 147, 149, 194] {
            assert!(!mask.get(inside), "position {inside} must not be set");
        }

        let mut big = Vwap::for_slice(Availability::Present);
        big.fold(&at(0, 5_700_000, 1)).expect("first bar");
        big.fold(&at(1, 5_700_001, 1)).expect("second bar");
        assert_eq!(big.sigma(), Some(0), "the old formula said 1,378");

        // A real dispersion survives untouched, and an odd total volume (r != 0)
        // with zero spread is exactly zero.
        let mut spread = Vwap::for_slice(Availability::Present);
        spread.fold(&at(0, 10_000, 1)).expect("first bar");
        spread.fold(&at(1, 10_101, 2)).expect("second bar");
        assert_eq!(
            spread.sigma(),
            oracle::sigma(spread.pv, spread.v, spread.p2v)
        );
        assert_eq!(spread.sigma(), Some(47), "sqrt(2/9)·101 = 47.6");
        let mut flat = Vwap::for_slice(Availability::Present);
        flat.fold(&at(0, 12_345, 3)).expect("first bar");
        flat.fold(&at(1, 12_345, 4)).expect("second bar");
        assert_eq!(flat.sigma(), Some(0));
    }

    /// The first `checked_` on the new path: `pv + r` leaving `i128` refuses rather
    /// than wraps. Unreachable from a fold, built from the private fields.
    #[test]
    fn sigma_refuses_when_the_remainder_sum_leaves_i128() {
        let v = primed(0, i128::MAX, 2, 0);
        assert_eq!(v.sigma(), None);
    }

    /// The 256-bit product comparison against the limb oracle, at the edges.
    #[test]
    fn the_wide_product_comparison_is_exact() {
        let max = i128::MAX.unsigned_abs();
        let mut rng = Lcg(7);
        let mut cases = vec![
            (0, 0, 0, 0),
            (max, max, max, max),
            (max, max - 1, max - 1, max),
            (max, 1, 1, max),
            (1 << 64, 1 << 64, 1, u128::MAX >> 1),
            ((1 << 64) - 1, (1 << 64) + 1, 1 << 64, 1 << 64),
        ];
        for _ in 0..5_000 {
            let mut n =
                || (u128::from(rng.next()) << 64 | u128::from(rng.next())) >> rng.below(127);
            cases.push((n(), n(), n(), n()));
        }
        for (a, b, c, d) in cases {
            let (a, b, c, d) = (a & max, b & max, c & max, d & max);
            assert_eq!(wide_mul(a, b), oracle::product(a, b), "{a}·{b}");
            assert_eq!(
                wide_mul(a, b) <= wide_mul(c, d),
                oracle::product_le(a, b, c, d),
                "{a}·{b} <= {c}·{d}",
            );
        }
    }
}
