//! Session-anchored VWAP and its three sigma bands.
//!
//! **20 vocabulary positions**: 52–53, 143–152 and 190–197.
//!
//! # The abstention is decided ONCE PER RUN, never per bar
//!
//! This is the rule that matters most here, and getting it wrong is subtle enough
//! that an audit named it before the module existed.
//!
//! A spot index has no traded volume, so on a slice where `volume` is identically
//! zero VWAP is a division by zero and every position must be false. The tempting
//! implementation checks `cumulative_volume == 0` per bar and returns nothing —
//! and it is **wrong**, because a slice whose first hour carries zero volume and
//! whose second hour does not would make bit 52 mean "below VWAP" in the morning
//! and "no opinion" at the open, silently, within one run.
//!
//! So the decision is taken before the first bar: [`Vwap::for_slice`] is handed
//! the verdict, and [`Availability::Absent`] makes every one of the 20 positions
//! false for the whole run. `docs/03-vocabulary.md` §4 is the authority — a bit
//! that cannot be evaluated evaluates false, and it never evaluates to "probably".
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
//! # Integer square root, no floats
//!
//! Sigma needs a square root and §7 forbids floating point at any layer. This uses
//! a Newton iteration on `i128` whose total step count is bounded by two
//! compile-time constants, so the cost is O(1) for **every** `i128` — see
//! [`isqrt_i128`], which records why an earlier 64-step cap was not.
//!
//! # Sigma refuses on one observation
//!
//! A single contributing bar has zero dispersion. Reporting `Some(0)` is
//! arithmetically correct and semantically ruinous: it puts all three bands exactly
//! on VWAP, which makes `close_above_vwap_band3_upper` an alias for
//! `close_above_vwap` and hands the sweep two permanently-identical positions on
//! the first bar of every session. [`Vwap::sigma`] returns `None` below
//! `MIN_FOR_SIGMA` contributing bars instead.

use store::format::Bar;
use vocab::{ConditionMask, Tolerance};

/// Whether this run's slice carries traded volume at all.
///
/// Decided once, before the first bar, and never revisited. See the module
/// documentation for why per-bar is wrong.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Availability {
    /// The slice carries volume. VWAP is computable.
    Present,
    /// The slice carries no volume anywhere. Every VWAP position is false for the
    /// whole run, and the reason is recorded rather than inferred.
    Absent,
}

/// Why an accumulator refused.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// A record that is not a bar.
    Corrupt(crate::Corrupt),
    /// `volume` is negative. Zero is a real zero (§7); negative is corruption.
    NegativeVolume,
    /// The price-times-volume accumulator would leave the range this module can
    /// prove safe. Refused rather than wrapped.
    AccumulatorTooLarge,
}

/// The band multipliers, in sigma. The design source ships exactly these three.
pub const BAND_SIGMA: [i128; 3] = [1, 2, 3];

/// The ceiling this module will let `sum(p² · v)` reach before refusing.
///
/// `i128::MAX` is 1.7 × 10³⁸. This stops at 10³⁴ — four orders of margin, which
/// leaves room for the `sigma² = E[p²] − E[p]²` arithmetic that follows without
/// having to prove a second bound.
const ACC_CEILING: i128 = 10_i128.pow(34);

/// Integer square root by Newton's method, in a **compile-time-bounded** number
/// of steps.
///
/// `i128::isqrt` exists but is not `const`, and an unbounded loop would make
/// sigma's cost depend on the value, which `CLAUDE.md` §3 rule 4 forbids.
///
/// # Why the cap is [`NEWTON_STEPS`] and not 64
///
/// An earlier version capped Newton at 64 and then stepped down without a bound,
/// under a doc comment claiming "64 iterations is past the point of convergence
/// for any `i128`". **That claim was false and was measured false.** Starting from
/// `guess = v`, each iteration roughly halves the guess until it nears `sqrt(v)`,
/// so a 127-bit input needs about 64 halvings *before* quadratic convergence even
/// begins. Above roughly `10^34.3` the 64-step cap returned early and the
/// unbounded step-down absorbed the remainder: at `i128::MAX` it needed
/// **1,638,791,155,897,336,446 decrements**, i.e. `O(sqrt(v))`, in a `pub fn`
/// documented as constant-cost.
///
/// 128 steps covers the ~64 halvings plus the handful of quadratic steps for every
/// `i128`, and the step-down is now bounded at [`STEP_DOWN_STEPS`]. Both are
/// compile-time constants, so the cost is O(1) for every input in the type —
/// including the ones no caller reaches today.
#[must_use]
pub fn isqrt_i128(v: i128) -> i128 {
    if v <= 0 {
        return 0;
    }
    let mut guess = v;
    let mut previous = 0;
    let mut i: u32 = 0;
    while i < NEWTON_STEPS && guess != previous {
        previous = guess;
        guess = guess.midpoint(v / guess);
        i = i.saturating_add(1);
    }
    // Newton can land one above. A bounded step-down keeps the never-over-reports
    // property without letting a pathological input turn this into a scan: with
    // NEWTON_STEPS = 128 the residual is provably at most 1, and the bound of 2
    // leaves a step of margin rather than sitting exactly on the proof.
    let mut down: u32 = 0;
    while down < STEP_DOWN_STEPS && guess > 0 && guess.saturating_mul(guess) > v {
        guess = guess.saturating_sub(1);
        down = down.saturating_add(1);
    }
    guess
}

/// Newton iterations. Enough for the full `i128` range — see [`isqrt_i128`].
const NEWTON_STEPS: u32 = 128;

/// Bound on the corrective step-down. The residual after [`NEWTON_STEPS`] is at
/// most 1; this leaves one step of margin and makes the total cost constant.
const STEP_DOWN_STEPS: u32 = 2;

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
    pub fn availability_of(bars: &[Bar]) -> Availability {
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
    /// `sigma² = E[p²] − E[p]²`, both weighted. Computed on the ×3 scale and
    /// divided down once, so no per-bar rounding accumulates.
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
        let mean = self.pv.div_euclid(self.v);
        let mean_sq = self.p2v.div_euclid(self.v);
        // `mean * mean` was the third unchecked multiply on this path. With the
        // checked fold above, p2v <= ACC_CEILING bounds it — but the bound is an
        // argument and this is a mechanism.
        let mean_squared = mean.checked_mul(mean)?;
        let variance = mean_sq.checked_sub(mean_squared)?;
        // Negative variance is impossible in exact arithmetic and reachable here
        // only through the two floor divisions above; clamp rather than sqrt a
        // negative, and never report a negative sigma.
        let sigma_scaled = isqrt_i128(variance.max(0));
        i64::try_from(sigma_scaled.div_euclid(3)).ok()
    }

    /// Fold one bar into the accumulators.
    ///
    /// # Errors
    ///
    /// [`Refused::Corrupt`] for a record that is not a bar, [`Refused::NegativeVolume`]
    /// for a negative volume, and [`Refused::AccumulatorTooLarge`] when the running
    /// sum would leave the range this module can prove safe.
    pub fn fold(&mut self, bar: &Bar) -> Result<(), Refused> {
        if bar.high < bar.low {
            return Err(Refused::Corrupt(crate::Corrupt::HighBelowLow));
        }
        if bar.high.checked_sub(bar.low).is_none() {
            return Err(Refused::Corrupt(crate::Corrupt::RangeOverflows));
        }
        if bar.volume < 0 {
            return Err(Refused::NegativeVolume);
        }
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
        // `store::format::ohlc_is_sane` checks field ORDER only, never magnitude)
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
    pub fn step(&mut self, bar: &Bar, tolerance: Tolerance) -> Result<ConditionMask, Refused> {
        self.fold(bar)?;
        Ok(self.bits(bar.close, tolerance))
    }

    /// The 20 positions for one closing price.
    ///
    /// # Cost
    ///
    /// One division for the mean, one for the mean square, one fixed-iteration
    /// square root, then 20 comparisons. Every count is a compile-time constant.
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

        for (band, multiple) in BAND_SIGMA.iter().enumerate() {
            let Some(offset) = i64::try_from(multiple * i128::from(sigma)).ok() else {
                continue;
            };
            let upper = vwap.saturating_add(offset);
            let lower = vwap.saturating_sub(offset);
            let Some((above, below, near_up, near_down, inside)) = band_positions(band) else {
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
}

/// `(above, below, near_upper, near_lower, inside)` for band 0, 1 or 2.
///
/// Written out because the shipped table does **not** number them regularly: band
/// 1's five positions are 146, 147, 150, 151, 152, band 2's are 148, 149, 190,
/// 191, 192, and band 3's are 193, 194, 195, 196, 197. Deriving them by arithmetic
/// from a base is one off-by-one away from a mask that means a different band.
const fn band_positions(band: usize) -> Option<(u16, u16, u16, u16, u16)> {
    match band {
        0 => Some((146, 147, 150, 151, 152)),
        1 => Some((148, 149, 190, 191, 192)),
        2 => Some((193, 194, 195, 196, 197)),
        _ => None,
    }
}

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
mod tests {
    use super::*;

    fn tol() -> Tolerance {
        let Ok(t) = vocab::tolerance::pinned_fib() else {
            unreachable!("the fib width is pinned")
        };
        t
    }

    fn at(minute: i64, price: i64, volume: i64) -> Bar {
        let ts = (50_000 * 1_440 + 555 + minute) * 60_000_000 - 19_800 * 1_000_000;
        Bar {
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
            let Some(def) = vocab::table::definition(index) else {
                unreachable!("position {index} is not in the table")
            };
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
        let bars: Vec<Bar> = (0..20).map(|m| at(m, 2_500_000 + m * 100, 0)).collect();
        assert_eq!(Vwap::availability_of(&bars), Availability::Absent);
        let mut v = Vwap::for_slice(Availability::Absent);
        for bar in &bars {
            let Ok(mask) = v.step(bar, tol()) else {
                unreachable!("a zero-volume bar is legal")
            };
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
        let mut bars: Vec<Bar> = (0..5).map(|m| at(m, 2_500_000, 0)).collect();
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
        let Some(sixth) = bars.get(5) else {
            unreachable!("the slice has ten bars")
        };
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
    /// `ohlc_is_sane` checks field ORDER only — never magnitude — so this bar is
    /// legally writable to the store and readable back with a valid CRC. Before the
    /// checked arithmetic went in, `price * price` was 2.25e38, past `i128::MAX`:
    /// release wrapped it to a large negative value that is not `> ACC_CEILING`, so
    /// `fold` returned `Ok(())` and `value()` answered `Some(5000000000000000000)`
    /// from a poisoned accumulator. Debug panicked instead. This test would have
    /// gone red — or panicked — on either profile.
    #[test]
    fn the_i64_extreme_is_refused_rather_than_wrapped_or_panicked() {
        let mut v = Vwap::for_slice(Availability::Present);
        let extreme = Bar {
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
        let Some(value) = v.value() else {
            unreachable!("volume is present")
        };
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
        assert_eq!(v.fold(&at(0, 1_000_000, -1)), Err(Refused::NegativeVolume));
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
        let huge = Bar {
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
            let Ok(_) = v.step(&at(minute, price, 1_000 + minute), tol()) else {
                unreachable!("sane")
            };
        }
        for close in [2_400_000_i64, 2_500_000, 2_502_000, 2_600_000] {
            let mask = v.bits(close, tol());
            for band in 0..3 {
                let Some((above, below, _, _, inside)) = band_positions(band) else {
                    unreachable!("three bands")
                };
                let n = u32::from(mask.get(u32::from(above)))
                    + u32::from(mask.get(u32::from(below)))
                    + u32::from(mask.get(u32::from(inside)));
                assert_eq!(n, 1, "band {band} at close {close} had {n} relations");
            }
        }
    }

    /// The bands nest: inside band 1 implies inside bands 2 and 3.
    #[test]
    fn the_bands_nest_outward() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..90 {
            let price = 2_500_000 + (minute * 211) % 6_000;
            let Ok(_) = v.step(&at(minute, price, 5_000), tol()) else {
                unreachable!("sane")
            };
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
            let Ok(mask) = v.step(&at(minute, price, 1_000 + minute * 3), tol()) else {
                unreachable!("sane")
            };
            union = union.union(&mask);
        }
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
            let Ok(_) = v.step(&at(minute, 9_000_000, 1_000), tol()) else {
                unreachable!("sane")
            };
        }
        assert_eq!(v.value(), Some(9_000_000));
        let mut next = at(0, 1_000_000, 1_000);
        next.ts_micros += 1_440 * 60_000_000;
        let Ok(_) = v.step(&next, tol()) else {
            unreachable!("sane")
        };
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
                    let Ok(mask) = v.step(&at(minute, price, 700 + minute), tol()) else {
                        unreachable!("sane")
                    };
                    mask.words()
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(run(), run());
    }

    /// The shipped pair and the appended pair are set together, deliberately.
    #[test]
    fn the_shipped_and_appended_pairs_agree() {
        let mut v = Vwap::for_slice(Availability::Present);
        for minute in 0..30 {
            let Ok(_) = v.step(&at(minute, 2_500_000 + minute * 50, 1_000), tol()) else {
                unreachable!("sane")
            };
        }
        for close in [2_400_000_i64, 2_500_700, 2_600_000] {
            let mask = v.bits(close, tol());
            assert_eq!(mask.get(52), mask.get(143), "52 and 143 disagreed");
            assert_eq!(mask.get(53), mask.get(144), "53 and 144 disagreed");
        }
    }
}
