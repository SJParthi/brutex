//! White's Reality Check, Hansen's SPA, and Romano–Wolf — the three tests that
//! ask whether the BEST of many strategies beat luck.
//!
//! # The question the deflated bar cannot ask
//!
//! [`crate::significance`] deflates a threshold by how many hypotheses were
//! tried. That is a correction computed from a COUNT, and it assumes the
//! hypotheses are independent. They are not: two combinations sharing four of
//! five conditions fire on nearly the same bars, and their returns move
//! together. Charging Bonferroni's full price for them is too harsh, and
//! charging nothing is too kind.
//!
//! A bootstrap does not need to assume anything about the dependence, because
//! it **resamples the periods** and lets whatever correlation exists come along
//! for the ride. That is the whole reason these three tests exist.
//!
//! # The three, and what separates them
//!
//! * **White's Reality Check** (2000) — is the best strategy's mean return
//!   distinguishable from the best a set of worthless strategies would produce?
//!   One p-value for the whole set.
//! * **Hansen's SPA** (2005) — the same test, with poor strategies no longer
//!   diluting the null. White's version gets less powerful the more rubbish you
//!   add to the comparison; Hansen's does not.
//! * **Romano–Wolf** (2005) — a stepdown that gives a verdict PER STRATEGY at a
//!   controlled family-wise error rate, rather than one yes/no for the set.
//!
//! They answer three different questions and the first two are often confused.
//! Reality Check says "something here is real". Romano–Wolf says "these
//! specific ones are".
//!
//! # Determinism, which a bootstrap makes hard and `CLAUDE.md` §3 rule 5 requires
//!
//! A resampling test that used a random seed would produce a different p-value
//! on every run, and reruns would stop being safe. The generator here is
//! `SplitMix64` seeded from a value the CALLER supplies, so the same inputs give
//! the same answer byte for byte — and a caller who wants a different draw
//! passes a different seed deliberately rather than getting one by accident.
//!
//! # The stationary bootstrap, and why not a plain one
//!
//! Returns are serially correlated: a trending afternoon is not a sequence of
//! independent minutes. Resampling single periods would destroy that structure
//! and understate the variance, making everything look more significant than it
//! is. Politis & Romano's stationary bootstrap resamples **blocks** of random
//! geometric length instead, so the dependence inside a block survives.
//!
//! # Nothing here reads a bar
//!
//! Every input is a per-period return some earlier stage measured. This module
//! opens no file, reads no store, and contacts nothing — which is why its tests
//! are hand-built sequences with known answers and carry no fixture.

// The exception `crates/greeks`, `crate::significance` and `crate::outcome`
// take, for the reason §7 states in one breath: prices are paisa integers, and
// statistical values keep full precision. Every float here is a mean, a
// standard error or a p-value. No price reaches this module.
#![allow(
    clippy::float_arithmetic,
    reason = "CLAUDE.md §7 keeps statistical values at full precision. Nothing \
              in this module is a price; every float is a test statistic."
)]
// A period count above 2^52 cannot arise: it would be more bars than the store
// can address, and the sweep is bounded far below that. The cast is stated
// rather than hidden because `as` on a count is exactly how a silent truncation
// ships, and this file has 29 of them.
#![allow(
    clippy::cast_precision_loss,
    reason = "a period or strategy count above 2^52 exceeds every bound this \
              engine carries; the store cannot address that many bars."
)]

/// A deterministic generator, so a resampling test is reproducible.
///
/// `SplitMix64`: one multiply-xor-shift chain per draw, no state beyond a `u64`,
/// and a period of 2^64. It is not cryptographic and does not need to be — what
/// it needs is to give the same stream for the same seed on every machine,
/// which `CLAUDE.md` §3 rule 5 requires and a system RNG cannot promise.
#[derive(Clone, Copy, Debug)]
struct Rng(u64);

impl Rng {
    /// A generator from an explicit seed.
    ///
    /// The seed is the caller's, never the clock's. A test that seeded itself
    /// from time would give a different p-value on every run, and a rerun that
    /// disagrees with the run it repeats is the opposite of a check.
    const fn new(seed: u64) -> Self {
        // A zero seed is legal for `SplitMix64` -- the increment carries the
        // state forward -- so it is not special-cased. Noting it because a
        // generator that silently rejected zero would make `seed: 0` mean
        // something other than what the caller wrote.
        Self(seed)
    }

    /// The next value in the stream.
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    }

    /// A value in `0..n`, or zero when `n` is zero.
    ///
    /// Modulo rather than rejection sampling. The bias is `2^64 mod n` out of
    /// `2^64` — for any period count a backtest can hold, that is far below one
    /// part in 10^15, and stating the bound beats implying there is none.
    fn below(&mut self, n: usize) -> usize {
        if n == 0 {
            return 0;
        }
        usize::try_from(self.next_u64() % (n as u64)).unwrap_or(0)
    }

    /// A coin that lands true with probability `numerator / 1_000_000`.
    fn chance(&mut self, numerator: u64) -> bool {
        self.next_u64() % 1_000_000 < numerator
    }
}

/// One strategy's per-period returns, and the summary the tests need.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Performance {
    /// Mean return per period.
    pub mean: f64,
    /// Standard error of that mean.
    ///
    /// Used by [`spa`] to studentize. Zero when a strategy never varied, which
    /// is handled rather than divided by.
    pub standard_error: f64,
    /// Periods contributing.
    pub periods: usize,
}

/// What a Reality Check concluded.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Verdict {
    /// The observed statistic: the best studentized mean across strategies.
    pub statistic: f64,
    /// Fraction of bootstrap draws whose statistic matched or beat the observed
    /// one. Small means the observed best is hard to explain by luck.
    pub p_value: f64,
    /// Bootstrap draws taken.
    pub draws: usize,
    /// Strategies compared.
    pub strategies: usize,
    /// Periods each strategy's return series held.
    ///
    /// # Why the sample size travels WITH the verdict
    ///
    /// It is the number that decides whether `p_value` can be believed, and
    /// until this field a caller could not see it. A p-value carries no record
    /// of how much evidence produced it, so a verdict from three periods and one
    /// from three hundred render identically and mean entirely different things.
    /// [`Self::calibration`] is what turns this into a sentence.
    pub periods: usize,
}

impl Verdict {
    /// Does the best strategy clear a family-wise error rate of 5%?
    ///
    /// A stated convention and not a derivation, the same 5% `FWER` in
    /// [`crate::significance`]. It is a threshold on the p-value and nothing
    /// more — clearing it does not make a strategy profitable, and
    /// [`crate::grid`]'s pessimistic total is a separate question.
    ///
    /// Inclusive, `p <= 0.05`: the same boundary the Romano-Wolf stepdown
    /// rejects on (`(1 + count) / (B + 1) <= alpha`), so a strategy at exactly
    /// 5% is not printed as failing White beside a stepdown that rejects it
    /// (D-1548).
    #[must_use]
    pub fn clears(&self) -> bool {
        self.draws > 0 && self.p_value <= 0.05
    }

    /// What this verdict's sample size is worth, in measured terms.
    ///
    /// # Disclosure rather than a threshold, and that is deliberate
    ///
    /// These tests are badly miscalibrated on short samples. Measured on pure
    /// noise against a nominal 5% (`docs/06-limits.md` §77): **73.5% false
    /// positives at 1 period, 45.9% at 2, 37.1% at 3, 26.1% at 5, 21.2% at 10,
    /// 13.3% at 30, 7.4% at 100**, reaching nominal by about 300.
    ///
    /// The repair a reader expects is a minimum-period floor. **This crate does
    /// not have one**, because *which* floor is a number `CLAUDE.md` §3 rule 1
    /// forbids it inventing and no source in `docs/00-charter.md` supplies. What
    /// §3 rule 6 DOES require is that the limit be stated rather than hidden —
    /// so the verdict names the measured rate for the sample it actually had,
    /// and the operator decides.
    ///
    /// The bands are the measured rows, not interpolation. A sample between two
    /// rows takes the WORSE of them, because rounding a false-positive rate
    /// toward the flattering side is the error the whole module exists to avoid.
    #[must_use]
    pub const fn calibration(&self) -> &'static str {
        match self.periods {
            0 | 1 => "1 period: no resolution at all -- refused, p forced to 1.0",
            2 => "2 periods: measured 45.9% false positives against a nominal 5%",
            3..=4 => "3 periods: measured 37.1% false positives against a nominal 5%",
            5..=9 => "5 periods: measured 26.1% false positives against a nominal 5%",
            10..=29 => "10 periods: measured 21.2% false positives against a nominal 5%",
            30..=99 => "30 periods: measured 13.3% false positives against a nominal 5%",
            100..=299 => "100 periods: measured 7.4% false positives against a nominal 5%",
            _ => "300+ periods: measured at the nominal 5%",
        }
    }
}

/// Exact finite-resample probability for White's Reality Check or Hansen's SPA.
///
/// Both procedures count bootstrap maxima that match or exceed the observed
/// statistic, then apply the finite-resample `+1` correction.  The exact count
/// is retained because recovering it from an `f64` p-value is impossible in
/// general and would turn a rounded projection into invented evidence.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactFamilyTestPValueV1 {
    matched_or_exceeded_plus_one: usize,
    draws_plus_one: usize,
}

impl ExactFamilyTestPValueV1 {
    /// Bootstrap maxima that matched or exceeded the observed statistic, plus one.
    #[must_use]
    pub const fn numerator(self) -> usize {
        self.matched_or_exceeded_plus_one
    }

    /// Bootstrap draws plus one.
    #[must_use]
    pub const fn denominator(self) -> usize {
        self.draws_plus_one
    }

    /// Direct full-precision projection of the retained exact fraction.
    #[must_use]
    pub fn value(self) -> f64 {
        self.matched_or_exceeded_plus_one as f64 / self.draws_plus_one as f64
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ExactFamilyTestFieldsV1 {
    statistic_bits: u64,
    p_value_bits: u64,
    matched_or_exceeded: usize,
    exact_p_value: ExactFamilyTestPValueV1,
    draws: usize,
    strategies: usize,
    periods: usize,
    seed: u64,
    block: usize,
    family_digest: [u8; 32],
}

impl ExactFamilyTestFieldsV1 {
    fn verdict(self) -> Verdict {
        Verdict {
            statistic: f64::from_bits(self.statistic_bits),
            p_value: f64::from_bits(self.p_value_bits),
            draws: self.draws,
            strategies: self.strategies,
            periods: self.periods,
        }
    }
}

/// Exact, identity-bound White Reality Check result.
///
/// This is an in-memory receipt, not durable authority.  It preserves the
/// exact finite-resample count and ordered input/procedure identity needed by a
/// later receipt-last writer without changing the legacy [`reality_check`]
/// result shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WhiteRealityCheckReceiptV1 {
    fields: ExactFamilyTestFieldsV1,
}

/// Exact, identity-bound Hansen SPA result.
///
/// This is an in-memory receipt, not durable authority.  It preserves the
/// exact finite-resample count and ordered input/procedure identity needed by a
/// later receipt-last writer without changing the legacy [`spa`] result shape.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SpaReceiptV1 {
    fields: ExactFamilyTestFieldsV1,
}

macro_rules! impl_exact_family_test_receipt_v1 {
    ($receipt:ident) => {
        impl $receipt {
            /// Legacy verdict reconstructed bit-for-bit from this exact receipt.
            #[must_use]
            pub fn verdict(self) -> Verdict {
                self.fields.verdict()
            }

            /// Full-precision observed statistic.
            #[must_use]
            pub const fn statistic(self) -> f64 {
                f64::from_bits(self.fields.statistic_bits)
            }

            /// Original observed-statistic bits.
            #[must_use]
            pub const fn statistic_bits(self) -> u64 {
                self.fields.statistic_bits
            }

            /// Full-precision p-value projection.
            #[must_use]
            pub const fn p_value(self) -> f64 {
                f64::from_bits(self.fields.p_value_bits)
            }

            /// Original p-value bits.
            #[must_use]
            pub const fn p_value_bits(self) -> u64 {
                self.fields.p_value_bits
            }

            /// Exact finite-resample probability and denominator.
            #[must_use]
            pub const fn exact_p_value(self) -> ExactFamilyTestPValueV1 {
                self.fields.exact_p_value
            }

            /// Draws whose bootstrap maximum matched or exceeded the observation.
            #[must_use]
            pub const fn matched_or_exceeded(self) -> usize {
                self.fields.matched_or_exceeded
            }

            /// Deterministic bootstrap draw count.
            #[must_use]
            pub const fn draws(self) -> usize {
                self.fields.draws
            }

            /// Candidates in the complete ordered family.
            #[must_use]
            pub const fn strategies(self) -> usize {
                self.fields.strategies
            }

            /// Aligned periods in every candidate return series.
            #[must_use]
            pub const fn periods(self) -> usize {
                self.fields.periods
            }

            /// Explicit deterministic `SplitMix64` seed.
            #[must_use]
            pub const fn seed(self) -> u64 {
                self.fields.seed
            }

            /// Stationary-bootstrap average block length.
            #[must_use]
            pub const fn block(self) -> usize {
                self.fields.block
            }

            /// Digest of the ordered return family and all procedure inputs.
            #[must_use]
            pub const fn family_digest(self) -> [u8; 32] {
                self.fields.family_digest
            }
        }
    };
}

impl_exact_family_test_receipt_v1!(WhiteRealityCheckReceiptV1);
impl_exact_family_test_receipt_v1!(SpaReceiptV1);

/// The average block length of the stationary bootstrap, in periods.
///
/// Politis & Romano resample blocks of GEOMETRIC length so that the serial
/// dependence inside a block survives the resampling. Ten periods is a stated
/// assumption: on one-minute bars it is ten minutes, long enough to keep an
/// intraday trend intact and short enough that a 375-bar session contributes
/// many independent blocks.
///
/// It is not derived from anything and `CLAUDE.md` §3 rule 1 will not let this
/// module pretend otherwise. A caller who knows the autocorrelation of their own
/// returns should pass their own.
pub const DEFAULT_BLOCK: usize = 10;

/// The largest average block length the stationary bootstrap can honour.
///
/// The continuation draw is in parts per million: a new block starts with
/// probability `1_000_000 / block` ppm. Above one million that floors to
/// zero, so no draw ever restarts and every draw is one rotation of the series
/// (W3-runner1-2, D-0742). Every entry point refuses a longer block rather
/// than resampling at a block length the caller did not ask for.
pub const MAX_BLOCK: usize = 1_000_000;

/// White's Reality Check.
///
/// `returns[i]` is strategy `i`'s per-period returns. All must be the same
/// length: they are aligned in time, and a bootstrap draw picks the same periods
/// from every strategy so that the correlation between them is preserved.
///
/// `None` when the set is empty, when the series disagree in length, when a
/// series is empty, or when `block` exceeds [`MAX_BLOCK`] — refused rather than
/// answered, because a p-value computed over misaligned strategies, or over
/// draws that are all rotations of the sample, is a number about nothing.
///
/// # Cost
///
/// O(S·N) once to build each series' exact prefix sums, O(B·N) to draw and
/// split every draw into its R runs, and O(B·S·R) to read the resampled
/// means, R ≈ N/`block` expected (D-2316). The O(B·S·R) term holds for every
/// series whose fold is provably exact, `N x max|v| <= 2^53`; a series outside
/// that bound keeps the O(N) fold per draw, so the worst case is still
/// `draws x periods x strategies`. This is a once-per-run boundary and the
/// cost is stated rather than bounded.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn reality_check(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
) -> Option<Verdict> {
    if block > MAX_BLOCK {
        return None;
    }
    let periods = aligned_for(returns, block)?;
    let stats: Vec<Performance> = returns.iter().map(|r| summarise(r)).collect();

    // THE OBSERVED STATISTIC: the best mean across strategies, scaled by the
    // square root of the sample. `sqrt(n) * mean` rather than a raw mean,
    // because a strategy measured over more periods carries more evidence for
    // the same average and the test must reflect that.
    let root_n = (periods as f64).sqrt();
    let observed = stats
        .iter()
        .map(|s| root_n * s.mean)
        .fold(f64::NEG_INFINITY, f64::max);

    let mut rng = Rng::new(seed);
    let mut beaten = 0_usize;
    let mut index = Vec::with_capacity(periods);
    let prefixes = exact_prefixes(returns, periods);
    let mut draw = Resample::default();
    for _ in 0..draws {
        stationary_indices_into(&mut index, periods, block, &mut rng);
        draw.fill(&index);
        // RECENTRED, and this is the whole test. Each resampled mean has its
        // OWN observed mean subtracted, so the bootstrap distribution is what a
        // set of strategies with NO edge would produce while keeping this set's
        // correlation structure. Comparing the observed maximum against that is
        // the question White's test asks.
        let mut best = f64::NEG_INFINITY;
        for (s, series) in returns.iter().enumerate() {
            let resampled = resampled_mean(series, prefixes.get(s).and_then(Option::as_ref), &draw);
            let centred = stats.get(s).map_or(0.0, |o| resampled - o.mean);
            best = best.max(root_n * centred);
        }
        if best >= observed {
            beaten = beaten.saturating_add(1);
        }
    }

    Some(Verdict {
        periods,
        statistic: observed,
        // NOTHING TO COMPARE AGAINST READS AS NO EVIDENCE, NOT AS CERTAINTY.
        //
        // `draws == 0` is the case this rule was written for. `periods < 2` is
        // the same case reached from the other side: the stationary bootstrap
        // can only ever draw index 0 out of a one-period series, so every draw
        // reproduces the sample exactly and the null distribution is a point
        // mass. A p-value computed against a point mass is not a weak result, it
        // is an absent one — and before this guard a single positive period
        // returned p = 0.0 and cleared, for 1 paisa as readily as for 1,000,000.
        //
        // The direction is the one `a_sample_too_short_for_hansens_gate_keeps_every_strategy`
        // already fixes for the recentring gate: where a statistic cannot be
        // computed, the answer falls to the CONSERVATIVE side.
        // ADD ONE TO BOTH, WHICH IS NOT A FUDGE.
        //
        // This was `beaten / draws`, so a statistic no draw beat returned the
        // EXACT double 0.0 -- `to_bits() == 0` -- and the audit printed
        // `0.0000` beside `clears 5% = yes`. A 1,000-draw bootstrap cannot
        // resolve a probability finer than 1/1000; reporting zero claims a
        // certainty the estimator does not have, and it is the dangerous
        // direction because zero passes every threshold.
        //
        // `(beaten + 1) / (draws + 1)` is the standard bootstrap p-value
        // (Davison & Hinkley), and its floor is exactly the resolution the
        // draws bought: 1/1001 at 1,000 draws. It is what a resampling test can
        // honestly say, and it never returns zero.
        //
        // A FAMILY IN WHICH NO ROW EVER VARIED IS THE THIRD WAY IN (D-0972).
        // Every recentred draw of a constant row is exactly zero, so when every
        // row is constant the null distribution is the same point mass the
        // one-period case produces, and a positive constant scored 1/(B+1) --
        // for 1 paisa as readily as for 1,000,000. A family with at least one
        // varying row is untouched: its null has spread, and a riskless mean
        // beating that spread is the answer this test is meant to give.
        p_value: if draws == 0 || periods < 2 || white_null_is_a_point_mass(returns) {
            1.0
        } else {
            beaten.saturating_add(1) as f64 / draws.saturating_add(1) as f64
        },
        draws,
        strategies: returns.len(),
    })
}

/// Whether every row of a family is constant, so that every recentred
/// resample of every row is exactly zero and White's bootstrap maximum is a
/// point mass at zero on every draw (D-0972).
///
/// Exact rather than approximate: [`summarise`] and the resampled-mean fold
/// fold the same value the same number of times in the same order over a
/// constant row, so `resampled - mean` is `0.0` bit for bit -- and
/// [`resampled_mean`] is that fold's value bit for bit on either of its paths
/// (D-2316).
fn white_null_is_a_point_mass(returns: &[Vec<i64>]) -> bool {
    returns
        .iter()
        .all(|series| series.iter().all(|value| Some(value) == series.first()))
}

/// Whether White's exact receipt would count its strongest possible
/// probability, `1/(B+1)`, against a point-mass null (D-0972).
///
/// Only a positive observed statistic can: every draw's maximum is `0.0`, so it
/// never matches or exceeds a positive statistic and always matches a
/// nonpositive one, which already counts the conservative `(B+1)/(B+1)`. The
/// receipt refuses the first case rather than restating it: a changed
/// probability under the V1 procedure domain would rename what V1 bytes mean.
fn white_point_mass_would_mint_evidence(returns: &[Vec<i64>], observed: f64) -> bool {
    observed > 0.0 && white_null_is_a_point_mass(returns)
}

/// Hansen's Superior Predictive Ability test.
///
/// # What it fixes about the Reality Check
///
/// White's test recentres EVERY strategy, so adding a hundred hopeless ones
/// raises the bootstrap maximum and makes a genuinely good strategy harder to
/// detect. The test gets weaker the more rubbish you compare against, which is
/// the opposite of what a search over many combinations needs.
///
/// Hansen's fix is to leave a strategy out of the recentring when it is so far
/// below zero that no plausible sample could have produced a positive mean —
/// they cannot be the best under the null, so including them only adds noise.
/// The threshold is the standard one: `-sqrt(2 log log n)` standard errors.
///
/// It is also STUDENTIZED — each mean is divided by its own standard error — so
/// a strategy with a large but wildly variable return does not outrank a steady
/// one purely on size.
///
/// **The standard error is the i.i.d. one, and Hansen's is not.** Hansen
/// (2005) studentizes by a long-run (HAC or bootstrap) estimate; [`summarise`]
/// gives `sqrt(variance / n)`. Both sides of the comparison share it, so the
/// test stays a valid max-type test, but under serial correlation the
/// recentring gate drops a different set of strategies (hunt-runner-5,
/// D-1549, `docs/06-limits.md`).
///
/// `None` under the same conditions as [`reality_check`].
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn spa(returns: &[Vec<i64>], draws: usize, seed: u64, block: usize) -> Option<Verdict> {
    if block > MAX_BLOCK {
        return None;
    }
    let periods = aligned_for(returns, block)?;
    let stats: Vec<Performance> = returns.iter().map(|r| summarise(r)).collect();

    // HANSEN'S STATISTIC IS `sqrt(n) * mean / sigma`, AND THAT IS EXACTLY
    // `mean / standard_error`.
    //
    // [`summarise`] sets `standard_error = sqrt(variance / n)`, which is
    // `sigma / sqrt(n)`. So `mean / standard_error` already carries the
    // `sqrt(n)`, and multiplying by `root_n` first — which this did — produced
    // `n * mean / sigma`: a factor of `sqrt(n)` too large, growing without
    // bound in the sample size.
    //
    // The consequence was one-directional and it was the bad direction. The
    // recentring gate below keeps a strategy when its statistic clears
    // `-sqrt(2 ln ln n)`, so inflating every statistic by `sqrt(n)` pushed the
    // effective keep-threshold toward zero and dropped roughly HALF of the
    // true-null strategies from the maximum, where Hansen's own threshold drops
    // 1.6-2.7%. Dropping a true null lowers the bootstrap maximum, which lowers
    // the p-value, which reports significance that is not there. Measured
    // against a correct implementation on the same draws, the as-shipped
    // p-value was less than or equal to Hansen's in 1000 of 1000 paired
    // replications.
    //
    // `max(·, 0)` is Hansen's floor: `T_SPA = max_k max(sqrt(n) m_k / s_k, 0)`.
    // Without it the reported statistic can print negative, which is not a
    // value the test defines.
    let observed = stats
        .iter()
        .map(|s| studentized(s.mean, s.standard_error).max(0.0))
        .fold(f64::NEG_INFINITY, f64::max);

    // HANSEN'S GATE. `log log n` is undefined below e, so a sample too short to
    // take it keeps every strategy -- refusing to drop any is the conservative
    // direction, and it is the direction a threshold that cannot be computed
    // must fall in.
    let n = periods as f64;
    let gate = if n > 3.0 {
        -(2.0 * n.ln().ln()).sqrt()
    } else {
        f64::NEG_INFINITY
    };

    let mut rng = Rng::new(seed);
    let mut beaten = 0_usize;
    let mut index = Vec::with_capacity(periods);
    let prefixes = exact_prefixes(returns, periods);
    let mut draw = Resample::default();
    for _ in 0..draws {
        stationary_indices_into(&mut index, periods, block, &mut rng);
        draw.fill(&index);
        let mut best = f64::NEG_INFINITY;
        for (s, series) in returns.iter().enumerate() {
            let Some(own) = stats.get(s) else { continue };
            let resampled = resampled_mean(series, prefixes.get(s).and_then(Option::as_ref), &draw);
            // A strategy too far below zero cannot be the best under the null,
            // so it is recentred to nothing rather than dragging the maximum up.
            // Same correction as the observed statistic: no `root_n`, because
            // `standard_error` already carries it. The gate and the draw must
            // be on the same scale as `observed` or the comparison below is
            // between two different statistics.
            let keep = studentized(own.mean, own.standard_error) >= gate;
            let centred = if keep {
                resampled - own.mean
            } else {
                resampled
            };
            best = best.max(studentized(centred, own.standard_error).max(0.0));
        }
        if best >= observed {
            beaten = beaten.saturating_add(1);
        }
    }

    Some(Verdict {
        periods,
        statistic: observed,
        // NOTHING TO COMPARE AGAINST READS AS NO EVIDENCE, NOT AS CERTAINTY.
        //
        // `draws == 0` is the case this rule was written for. `periods < 2` is
        // the same case reached from the other side: the stationary bootstrap
        // can only ever draw index 0 out of a one-period series, so every draw
        // reproduces the sample exactly and the null distribution is a point
        // mass. A p-value computed against a point mass is not a weak result, it
        // is an absent one — and before this guard a single positive period
        // returned p = 0.0 and cleared, for 1 paisa as readily as for 1,000,000.
        //
        // The direction is the one `a_sample_too_short_for_hansens_gate_keeps_every_strategy`
        // already fixes for the recentring gate: where a statistic cannot be
        // computed, the answer falls to the CONSERVATIVE side.
        // ADD ONE TO BOTH, WHICH IS NOT A FUDGE.
        //
        // This was `beaten / draws`, so a statistic no draw beat returned the
        // EXACT double 0.0 -- `to_bits() == 0` -- and the audit printed
        // `0.0000` beside `clears 5% = yes`. A 1,000-draw bootstrap cannot
        // resolve a probability finer than 1/1000; reporting zero claims a
        // certainty the estimator does not have, and it is the dangerous
        // direction because zero passes every threshold.
        //
        // `(beaten + 1) / (draws + 1)` is the standard bootstrap p-value
        // (Davison & Hinkley), and its floor is exactly the resolution the
        // draws bought: 1/1001 at 1,000 draws. It is what a resampling test can
        // honestly say, and it never returns zero.
        p_value: if draws == 0 || periods < 2 {
            1.0
        } else {
            beaten.saturating_add(1) as f64 / draws.saturating_add(1) as f64
        },
        draws,
        strategies: returns.len(),
    })
}

/// Runs White's Reality Check and retains its exact finite-resample evidence.
///
/// Unlike [`reality_check`], this authority-producing API refuses zero draws,
/// a zero block, fewer than two periods and an unrepresentable `draws + 1`
/// denominator.  The legacy API keeps its conservative `p = 1` compatibility
/// behavior for those cases.  Both refuse a block above [`MAX_BLOCK`].  It also
/// refuses a family in which every row is constant and the observed statistic is positive (D-0972): every draw's
/// maximum is then exactly zero, and the count would be the strongest
/// probability the draws can express, earned by no variation at all.  The
/// legacy API answers that family `p = 1`.  The counted comparison remains
/// White's existing
/// `bootstrap maximum >= observed statistic`; changing it to strict `>` would
/// be a different procedure version, not a receipt-only change.
///
/// # Cost
///
/// O(S·N + B·N + B·S·R) time and O(S·N + N + S) temporary space for B
/// draws, N periods, S strategies and R runs per draw (R ≈ N/`block`
/// expected), where every series' fold is provably exact
/// (`N x max|v| <= 2^53`); a series outside that bound keeps the O(N) fold,
/// so the worst case is still O(B·N·S) (D-2316).  Constructing the receipt is
/// not O(1).
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
#[must_use]
pub fn white_reality_check_receipt_v1(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
) -> Option<WhiteRealityCheckReceiptV1> {
    let (periods, stats) = exact_family_test_inputs_v1(returns, draws, block)?;
    let root_n = (periods as f64).sqrt();
    let observed = stats
        .iter()
        .map(|summary| root_n * summary.mean)
        .fold(f64::NEG_INFINITY, f64::max);
    if !observed.is_finite() || white_point_mass_would_mint_evidence(returns, observed) {
        return None;
    }

    let mut rng = Rng::new(seed);
    let mut matched_or_exceeded = 0_usize;
    let mut index = Vec::with_capacity(periods);
    let prefixes = exact_prefixes(returns, periods);
    let mut draw = Resample::default();
    for _ in 0..draws {
        stationary_indices_into(&mut index, periods, block, &mut rng);
        draw.fill(&index);
        let mut best = f64::NEG_INFINITY;
        for (strategy, series) in returns.iter().enumerate() {
            let resampled = resampled_mean(
                series,
                prefixes.get(strategy).and_then(Option::as_ref),
                &draw,
            );
            let centred = stats
                .get(strategy)
                .map_or(0.0, |summary| resampled - summary.mean);
            best = best.max(root_n * centred);
        }
        if best >= observed {
            matched_or_exceeded = matched_or_exceeded.checked_add(1)?;
        }
    }

    Some(WhiteRealityCheckReceiptV1 {
        fields: exact_family_test_fields_v1(
            b"brutex/runner/white-reality-check-exact/v1\0",
            returns,
            draws,
            seed,
            block,
            periods,
            observed,
            matched_or_exceeded,
        )?,
    })
}

/// Runs Hansen's SPA and retains its exact finite-resample evidence.
///
/// Unlike [`spa`], this authority-producing API refuses zero draws, a zero
/// block, fewer than two periods and an unrepresentable `draws + 1`
/// denominator.  Zero-variance candidates retain SPA's existing conservative
/// zero studentized contribution; the receipt does not silently turn them into
/// an infinite statistic or drop them from the ordered family.
///
/// # Cost
///
/// O(S·N + B·N + B·S·R) time and O(S·N + N + S) temporary space for B
/// draws, N periods, S strategies and R runs per draw (R ≈ N/`block`
/// expected), where every series' fold is provably exact
/// (`N x max|v| <= 2^53`); a series outside that bound keeps the O(N) fold,
/// so the worst case is still O(B·N·S) (D-2316).  Constructing the receipt is
/// not O(1).
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
#[must_use]
pub fn spa_receipt_v1(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
) -> Option<SpaReceiptV1> {
    let (periods, stats) = exact_family_test_inputs_v1(returns, draws, block)?;
    let observed = stats
        .iter()
        .map(|summary| studentized(summary.mean, summary.standard_error).max(0.0))
        .fold(f64::NEG_INFINITY, f64::max);
    if !observed.is_finite() {
        return None;
    }

    let n = periods as f64;
    let gate = if n > 3.0 {
        -(2.0 * n.ln().ln()).sqrt()
    } else {
        f64::NEG_INFINITY
    };
    let mut rng = Rng::new(seed);
    let mut matched_or_exceeded = 0_usize;
    let mut index = Vec::with_capacity(periods);
    let prefixes = exact_prefixes(returns, periods);
    let mut draw = Resample::default();
    for _ in 0..draws {
        stationary_indices_into(&mut index, periods, block, &mut rng);
        draw.fill(&index);
        let mut best = f64::NEG_INFINITY;
        for (strategy, series) in returns.iter().enumerate() {
            let own = stats.get(strategy)?;
            let resampled = resampled_mean(
                series,
                prefixes.get(strategy).and_then(Option::as_ref),
                &draw,
            );
            let keep = studentized(own.mean, own.standard_error) >= gate;
            let centred = if keep {
                resampled - own.mean
            } else {
                resampled
            };
            best = best.max(studentized(centred, own.standard_error).max(0.0));
        }
        if best >= observed {
            matched_or_exceeded = matched_or_exceeded.checked_add(1)?;
        }
    }

    Some(SpaReceiptV1 {
        fields: exact_family_test_fields_v1(
            b"brutex/runner/hansen-spa-exact/v1\0",
            returns,
            draws,
            seed,
            block,
            periods,
            observed,
            matched_or_exceeded,
        )?,
    })
}

fn exact_family_test_inputs_v1(
    returns: &[Vec<i64>],
    draws: usize,
    block: usize,
) -> Option<(usize, Vec<Performance>)> {
    if draws == 0 || block == 0 || block > MAX_BLOCK || draws.checked_add(1).is_none() {
        return None;
    }
    let periods = aligned_for(returns, block)?;
    if periods < 2 {
        return None;
    }
    let stats: Vec<Performance> = returns.iter().map(|series| summarise(series)).collect();
    if stats.iter().any(|summary| {
        !summary.mean.is_finite()
            || !summary.standard_error.is_finite()
            || summary.periods != periods
    }) {
        return None;
    }
    Some((periods, stats))
}

#[expect(
    clippy::too_many_arguments,
    reason = "every exact procedure input is independently retained in the receipt"
)]
fn exact_family_test_fields_v1(
    domain: &[u8],
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    periods: usize,
    observed: f64,
    matched_or_exceeded: usize,
) -> Option<ExactFamilyTestFieldsV1> {
    if matched_or_exceeded > draws || !observed.is_finite() {
        return None;
    }
    let exact_p_value = ExactFamilyTestPValueV1 {
        matched_or_exceeded_plus_one: matched_or_exceeded.checked_add(1)?,
        draws_plus_one: draws.checked_add(1)?,
    };
    let p_value = exact_p_value.value();
    if !p_value.is_finite() {
        return None;
    }
    Some(ExactFamilyTestFieldsV1 {
        statistic_bits: observed.to_bits(),
        p_value_bits: p_value.to_bits(),
        matched_or_exceeded,
        exact_p_value,
        draws,
        strategies: returns.len(),
        periods,
        seed,
        block,
        family_digest: ordered_family_test_digest_v1(domain, returns, draws, seed, block)?,
    })
}

fn ordered_family_test_digest_v1(
    domain: &[u8],
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
) -> Option<[u8; 32]> {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(domain);
    hasher.update(&u64::try_from(draws).ok()?.to_le_bytes());
    hasher.update(&seed.to_le_bytes());
    hasher.update(&u64::try_from(block).ok()?.to_le_bytes());
    hasher.update(&u64::try_from(returns.len()).ok()?.to_le_bytes());
    for (strategy, series) in returns.iter().enumerate() {
        hasher.update(&u64::try_from(strategy).ok()?.to_le_bytes());
        hasher.update(&u64::try_from(series.len()).ok()?.to_le_bytes());
        for value in series {
            hasher.update(&value.to_le_bytes());
        }
    }
    Some(hasher.finalize())
}

/// One strategy's Romano–Wolf verdict.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Rejected {
    /// Index into the caller's strategy list.
    pub strategy: usize,
    /// The stepdown round it was rejected in, from zero.
    ///
    /// Round zero is the strongest evidence: it cleared the threshold while
    /// every other strategy was still in the comparison.
    pub round: usize,
}

/// Complete in-memory authority for one Romano--Wolf stepdown.
///
/// An empty rejection set is meaningful only when the aligned family and the
/// procedure denominators are known to have been valid.  Keeping those facts
/// private and constructing this receipt beside the stepdown prevents an
/// invalid input (which the legacy vector API also renders as empty) from being
/// promoted to a measured non-rejection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RomanoWolfReceipt {
    rejected: Vec<Rejected>,
    decisions: Vec<bool>,
    draws: usize,
    strategies: usize,
    periods: usize,
    alpha_ppm: u64,
}

impl RomanoWolfReceipt {
    /// Rejected strategy indices in canonical stepdown order.
    #[must_use]
    pub fn rejected(&self) -> &[Rejected] {
        &self.rejected
    }

    /// Complete candidate decision, or `None` outside this family.
    #[must_use]
    pub fn is_rejected(&self, strategy: usize) -> Option<bool> {
        self.decisions.get(strategy).copied()
    }

    /// Bootstrap draws used by every stepdown round.
    #[must_use]
    pub const fn draws(&self) -> usize {
        self.draws
    }

    /// Strategies in the complete aligned family.
    #[must_use]
    pub const fn strategies(&self) -> usize {
        self.strategies
    }

    /// Aligned periods in every strategy return series.
    #[must_use]
    pub const fn periods(&self) -> usize {
        self.periods
    }

    /// Family-wise error threshold supplied to the stepdown, in ppm.
    #[must_use]
    pub const fn alpha_ppm(&self) -> u64 {
        self.alpha_ppm
    }
}

/// One exact finite-resample probability.
///
/// The fraction is retained rather than only its `f64` projection because a
/// draw count is the resolution the result actually bought.  For the
/// Romano--Wolf adjusted procedure the numerator is the number of strict
/// resample exceedances plus one and the denominator is the number of draws
/// plus one, exactly as Algorithm 4.1 of Romano and Wolf (2016) specifies.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ExactResamplingPValueV1 {
    numerator: usize,
    denominator: usize,
}

impl ExactResamplingPValueV1 {
    /// Exact numerator, including the finite-resample `+1` correction.
    #[must_use]
    pub const fn numerator(&self) -> usize {
        self.numerator
    }

    /// Exact denominator: bootstrap draws plus one.
    #[must_use]
    pub const fn denominator(&self) -> usize {
        self.denominator
    }

    /// Full-precision statistical projection; never rounded to ppm here.
    #[must_use]
    pub fn value(&self) -> f64 {
        self.numerator as f64 / self.denominator as f64
    }

    /// Whether this exact adjusted probability rejects at `alpha_ppm`.
    ///
    /// Integer cross multiplication preserves the exact finite-resample
    /// boundary.  An alpha outside the probability domain is refused.
    #[must_use]
    pub fn rejects_at_ppm(&self, alpha_ppm: u64) -> Option<bool> {
        if alpha_ppm > 1_000_000 {
            return None;
        }
        let lhs = (self.numerator as u128).saturating_mul(1_000_000);
        let rhs = (self.denominator as u128).saturating_mul(u128::from(alpha_ppm));
        Some(lhs <= rhs)
    }
}

/// One candidate in the canonical Romano--Wolf observed-statistic order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RomanoWolfAdjustedCandidateV1 {
    strategy: usize,
    stepdown_rank: usize,
    observed_statistic_bits: u64,
    strict_exceedances: usize,
    initial_p_value: ExactResamplingPValueV1,
    adjusted_p_value: ExactResamplingPValueV1,
}

impl RomanoWolfAdjustedCandidateV1 {
    /// Position in the caller's complete aligned family.
    #[must_use]
    pub const fn strategy(&self) -> usize {
        self.strategy
    }

    /// Canonical rank after descending observed-statistic ordering.
    ///
    /// Exact statistic ties retain ascending caller position.  That rule is
    /// part of the family identity rather than an unstable sort accident.
    #[must_use]
    pub const fn stepdown_rank(&self) -> usize {
        self.stepdown_rank
    }

    /// Observed studentized statistic at full `f64` precision.
    #[must_use]
    pub const fn observed_statistic(&self) -> f64 {
        f64::from_bits(self.observed_statistic_bits)
    }

    /// Draws whose surviving-family maximum strictly exceeded this statistic.
    #[must_use]
    pub const fn strict_exceedances(&self) -> usize {
        self.strict_exceedances
    }

    /// Candidate probability before the required stepdown monotonicity repair.
    #[must_use]
    pub const fn initial_p_value(&self) -> ExactResamplingPValueV1 {
        self.initial_p_value
    }

    /// Family-wise adjusted candidate probability after cumulative maximum.
    #[must_use]
    pub const fn adjusted_p_value(&self) -> ExactResamplingPValueV1 {
        self.adjusted_p_value
    }
}

/// Typed result of Romano--Wolf's adjusted-p-value Algorithm 4.1.
///
/// `candidates` is indexed by the caller's strategy position, so one candidate
/// lookup is O(1).  `stepdown_order` records the exact descending-statistic
/// order used by the suffix maxima.  `family_digest` binds the ordered return
/// family and every resampling input; moving a probability to another position
/// therefore changes the authority rather than silently renaming a strategy.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RomanoWolfAdjustedReceiptV1 {
    candidates: Vec<RomanoWolfAdjustedCandidateV1>,
    stepdown_order: Vec<usize>,
    draws: usize,
    periods: usize,
    seed: u64,
    block: usize,
    family_digest: [u8; 32],
}

impl RomanoWolfAdjustedReceiptV1 {
    /// Candidate result by caller position, O(1).
    ///
    /// **UNVERIFIED as a measured bound.** No bench in this workspace
    /// times this, so the shape above is read from the source rather
    /// than measured. `CLAUDE.md` §3 rule 6.
    #[must_use]
    pub fn candidate(&self, strategy: usize) -> Option<&RomanoWolfAdjustedCandidateV1> {
        self.candidates.get(strategy)
    }

    /// Caller positions in the exact order used by the stepdown.
    #[must_use]
    pub fn stepdown_order(&self) -> &[usize] {
        &self.stepdown_order
    }

    /// Number of bootstrap draws represented by every exact denominator.
    #[must_use]
    pub const fn draws(&self) -> usize {
        self.draws
    }

    /// Number of candidates in the complete aligned family.
    #[must_use]
    pub fn strategies(&self) -> usize {
        self.candidates.len()
    }

    /// Aligned periods in every candidate return series.
    #[must_use]
    pub const fn periods(&self) -> usize {
        self.periods
    }

    /// Explicit deterministic resampling seed.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// Stationary-bootstrap average block length.
    #[must_use]
    pub const fn block(&self) -> usize {
        self.block
    }

    /// Ordered input and procedure identity.
    #[must_use]
    pub const fn family_digest(&self) -> [u8; 32] {
        self.family_digest
    }

    /// P-value for the full-family maximum/intersection test.
    ///
    /// This is the adjusted probability of the first candidate in canonical
    /// stepdown order, whose null resample statistic is the maximum over the
    /// complete family.  It is a named Romano--Wolf family-intersection result,
    /// not an estimate of the realised error rate and not a generic statistic
    /// from some other procedure.
    #[must_use]
    pub fn familywise_p_value(&self) -> Option<ExactResamplingPValueV1> {
        let strategy = *self.stepdown_order.first()?;
        self.candidate(strategy)
            .map(RomanoWolfAdjustedCandidateV1::adjusted_p_value)
    }
}

/// Romano–Wolf stepdown.
///
/// # Why a stepdown rather than one p-value
///
/// [`reality_check`] and [`spa`] answer "is anything here real". Neither says
/// WHICH. Romano–Wolf does: reject the best strategy if it clears the maximum
/// of the bootstrap distribution, then REMOVE it and repeat with the rest. Each
/// round the comparison set shrinks, so the threshold falls and strategies that
/// were masked by the winner get their own chance.
///
/// The family-wise error rate is controlled across the whole procedure, which is
/// what makes this different from testing each strategy at 5% and hoping.
///
/// Returns the rejected strategies in the order they were rejected. This
/// legacy vector shape renders malformed input, a `block` above
/// [`MAX_BLOCK`] and a complete non-rejection as empty; callers that must
/// distinguish them use
/// [`romano_wolf_receipt`]. An `alpha_ppm` above `1_000_000` and zero draws
/// are malformed input here (D-0973).
///
/// A strategy is rejected on the exact `(1 + strict exceedances) / (B + 1) <=
/// alpha` rule, so on a family with no zero-variance row the rejected set is
/// the set whose [`romano_wolf_adjusted_p_values_v1`] probability rejects at the
/// same alpha; a zero-variance strategy is never rejected. Each strategy's
/// null statistics are computed once however many rounds the stepdown takes
/// (D-0973).
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn romano_wolf(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    alpha_ppm: u64,
) -> Vec<Rejected> {
    if block > MAX_BLOCK {
        return Vec::new();
    }
    let Some(periods) = aligned_for(returns, block) else {
        return Vec::new();
    };
    romano_wolf_aligned(returns, periods, draws, seed, block, alpha_ppm)
}

/// Runs Romano--Wolf and preserves the complete procedure denominators.
///
/// Unlike [`romano_wolf`], this distinguishes a valid family that rejected
/// nothing from malformed input.  Zero draws, a zero block length or one above
/// [`MAX_BLOCK`], and an alpha outside the ppm probability domain have no
/// complete procedure receipt and return `None`.  The decisions are
/// [`romano_wolf`]'s, on the exact rule (D-0973): at alpha zero nothing is
/// rejected.
#[must_use]
pub fn romano_wolf_receipt(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
    alpha_ppm: u64,
) -> Option<RomanoWolfReceipt> {
    if draws == 0 || block == 0 || block > MAX_BLOCK || alpha_ppm > 1_000_000 {
        return None;
    }
    let periods = aligned_for(returns, block)?;
    let rejected = romano_wolf_aligned(returns, periods, draws, seed, block, alpha_ppm);
    let mut decisions = vec![false; returns.len()];
    for result in &rejected {
        let decision = decisions.get_mut(result.strategy)?;
        *decision = true;
    }
    Some(RomanoWolfReceipt {
        rejected,
        decisions,
        draws,
        strategies: returns.len(),
        periods,
        alpha_ppm,
    })
}

/// Computes candidate-specific Romano--Wolf adjusted p-values.
///
/// This is Algorithm 4.1 of Romano and Wolf, *Efficient Computation of
/// Adjusted p-Values for Resampling-Based Stepdown Multiple Testing* (2016),
/// applied to the same studentized, recentred stationary-bootstrap statistics
/// as [`romano_wolf`].  The primary algorithm requires all hypotheses to be
/// ordered by decreasing observed statistic, takes the resample maximum over
/// each surviving suffix, and then replaces each initial probability by the
/// cumulative maximum of itself and every stronger candidate.  Omitting that
/// last maximum is explicitly identified by the authors as anti-conservative.
///
/// The one `draws x periods` index matrix is generated once and held across
/// every suffix.  That is enough information: each candidate's null statistic
/// is a deterministic function of one matrix row and its own aligned return
/// series.  No additional random draw, fitted distribution or independence
/// assumption enters this result.
///
/// `None` refuses an empty/misaligned family, fewer than two periods, zero
/// draws, a zero block length or one above [`MAX_BLOCK`], an unrepresentable
/// exact denominator, or any
/// zero-variance candidate.  The last case is structural: studentization has
/// no denominator, and assigning the source paper's strict-exceedance floor to
/// a point mass at zero would manufacture the strongest possible p-value from
/// no variation.
///
/// # Cost
///
/// O(S·N + S log S + B·N + B·S·R) time for S strategies, N periods, B draws
/// and R runs per draw (R ≈ N/`block` expected), where every series' fold is
/// provably exact (`N x max|v| <= 2^53`); a series outside that bound keeps
/// the O(N) fold, so the worst case is still O(B·S·N) (D-2316).  Retained
/// temporary space is O(S·N + B·R + B + S): one prefix per exact series and
/// each held draw's runs, at most two words per period.  Candidate lookup on
/// the completed receipt is O(1); constructing the complete statistical
/// authority is deliberately not claimed constant-time.
///
/// **UNVERIFIED as a measured bound.** No bench in this workspace
/// times this, so the shape above is read from the source rather
/// than measured. `CLAUDE.md` §3 rule 6.
#[must_use]
pub fn romano_wolf_adjusted_p_values_v1(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
) -> Option<RomanoWolfAdjustedReceiptV1> {
    if draws == 0 || block == 0 || block > MAX_BLOCK {
        return None;
    }
    let denominator = draws.checked_add(1)?;
    let periods = aligned_for(returns, block)?;
    if periods < 2 {
        return None;
    }
    let stats: Vec<Performance> = returns.iter().map(|series| summarise(series)).collect();
    if stats
        .iter()
        .any(|stat| stat.standard_error <= 0.0 || !stat.standard_error.is_finite())
    {
        return None;
    }

    // The statistic is sqrt(n)·mean/sigma == mean/standard_error.  Multiplying
    // by sqrt(n) again would inflate both observed and null values by a common
    // factor; comparisons often survive, but the recorded statistic would not
    // be the statistic the procedure names.
    let observed: Vec<f64> = stats
        .iter()
        .map(|stat| studentized(stat.mean, stat.standard_error))
        .collect();
    if observed.iter().any(|value| !value.is_finite()) {
        return None;
    }

    // Romano--Wolf begins with decreasing observed test statistics.  The paper
    // assumes a strict order; integer return series can tie, so caller position
    // is the explicit deterministic tie-breaker.  The monotonic adjustment
    // makes tied candidates share the stronger suffix probability rather than
    // gaining evidence from their arbitrary position.
    let mut stepdown_order: Vec<usize> = (0..returns.len()).collect();
    stepdown_order.sort_unstable_by(|left, right| {
        match (observed.get(*left), observed.get(*right)) {
            (Some(left_value), Some(right_value)) => right_value
                .total_cmp(left_value)
                .then_with(|| left.cmp(right)),
            _ => left.cmp(right),
        }
    });

    let mut rng = Rng::new(seed);
    let held = held_draws(periods, block, draws, &mut rng);
    let prefixes = exact_prefixes(returns, periods);

    // Walking the canonical order backwards grows one surviving suffix at a
    // time.  `maxima[m]` is therefore exactly max(t*_{r_s},...,t*_{r_S}) for
    // resample m when rank s is counted.
    let mut maxima = vec![f64::NEG_INFINITY; draws];
    let mut strict_exceedances_by_rank = vec![0_usize; returns.len()];
    for rank in (0..stepdown_order.len()).rev() {
        let strategy = *stepdown_order.get(rank)?;
        let series = returns.get(strategy)?;
        let own = stats.get(strategy)?;
        let observed_statistic = *observed.get(strategy)?;
        let prefix = prefixes.get(strategy).and_then(Option::as_ref);
        for (maximum, draw) in maxima.iter_mut().zip(&held) {
            let centred = resampled_mean(series, prefix, draw) - own.mean;
            let null_statistic = studentized(centred, own.standard_error);
            *maximum = maximum.max(null_statistic);
        }
        *strict_exceedances_by_rank.get_mut(rank)? = maxima
            .iter()
            .filter(|maximum| **maximum > observed_statistic)
            .count();
    }

    // Algorithm 4.1 step 2(b): monotone adjusted probabilities are the
    // cumulative maximum of the suffix-max probabilities.  Every probability
    // shares denominator B+1, so the exact integer numerators can be compared
    // without rounding through f64.
    let mut adjusted_numerator = 0_usize;
    let mut candidates: Vec<Option<RomanoWolfAdjustedCandidateV1>> = vec![None; returns.len()];
    for (rank, strategy) in stepdown_order.iter().copied().enumerate() {
        let strict_exceedances = *strict_exceedances_by_rank.get(rank)?;
        let initial_numerator = strict_exceedances.checked_add(1)?;
        adjusted_numerator = adjusted_numerator.max(initial_numerator);
        let row = RomanoWolfAdjustedCandidateV1 {
            strategy,
            stepdown_rank: rank,
            observed_statistic_bits: observed.get(strategy)?.to_bits(),
            strict_exceedances,
            initial_p_value: ExactResamplingPValueV1 {
                numerator: initial_numerator,
                denominator,
            },
            adjusted_p_value: ExactResamplingPValueV1 {
                numerator: adjusted_numerator,
                denominator,
            },
        };
        *candidates.get_mut(strategy)? = Some(row);
    }
    let candidates: Vec<RomanoWolfAdjustedCandidateV1> =
        candidates.into_iter().collect::<Option<Vec<_>>>()?;

    Some(RomanoWolfAdjustedReceiptV1 {
        candidates,
        stepdown_order,
        draws,
        periods,
        seed,
        block,
        family_digest: romano_wolf_family_digest_v1(returns, draws, seed, block)?,
    })
}

// ONE WALK OVER THE DRAWS FOR ALL THREE FAMILY-WISE TESTS. A child module, so
// it reads this module's generator, resampler and receipt fields without
// widening the visibility of any of them.
#[path = "bootstrap_family_pass.rs"]
mod family_pass;
pub use family_pass::{FamilyTestsRefusalV1, FamilyTestsV1, family_tests_v1};

/// Ordered Romano--Wolf family and procedure identity.
///
/// Generic over the row type only so a family named by position can be hashed
/// without cloning it; the bytes hashed are unchanged.
pub(crate) fn romano_wolf_family_digest_v1<R: AsRef<[i64]>>(
    returns: &[R],
    draws: usize,
    seed: u64,
    block: usize,
) -> Option<[u8; 32]> {
    let mut hasher = brutex_core::blake3::Hasher::new();
    hasher.update(b"brutex/runner/romano-wolf-adjusted-p-values/v1\0");
    hasher.update(&u64::try_from(draws).ok()?.to_le_bytes());
    hasher.update(&seed.to_le_bytes());
    hasher.update(&u64::try_from(block).ok()?.to_le_bytes());
    hasher.update(&u64::try_from(returns.len()).ok()?.to_le_bytes());
    for (strategy, series) in returns.iter().enumerate() {
        let series = series.as_ref();
        hasher.update(&u64::try_from(strategy).ok()?.to_le_bytes());
        hasher.update(&u64::try_from(series.len()).ok()?.to_le_bytes());
        for value in series {
            hasher.update(&value.to_le_bytes());
        }
    }
    Some(hasher.finalize())
}

/// The legacy stepdown, on the exact finite-resample rule and in one walk
/// (D-0973).
///
/// # One boundary, the adjusted receipt's
///
/// A surviving strategy is rejected in a round when one plus the number of
/// bootstrap maxima over the surviving set that STRICTLY exceed its statistic is
/// at most `floor((B + 1) * alpha_ppm / 1_000_000)`: the exact `(1 + count) /
/// (B + 1) <= alpha` test [`ExactResamplingPValueV1::rejects_at_ppm`] applies to
/// [`romano_wolf_adjusted_p_values_v1`]. The statistic, the tie order, the index
/// matrix and the seed are that receipt's too, so on a family with no
/// zero-variance row the rejected set is exactly the set whose adjusted
/// probability rejects at the same alpha. This replaced a rounded `1 - alpha`
/// quantile of the maxima, which rejected at alpha 0 and near 5% disagreed with
/// the exact rule in both directions.
///
/// # One walk
///
/// A round rejects every survivor at or above one bar, so the surviving set is
/// always a suffix of the canonical order (descending statistic, caller position
/// breaking ties). Walking that order from the weakest strategy grows the
/// maxima of every suffix in turn, and each suffix's bar -- the smallest
/// statistic its maxima would reject -- is kept. The rounds then read those
/// bars forwards. Each strategy's null series is computed once, where the
/// former loop recomputed every survivor's on every round.
///
/// # A zero-variance strategy is never rejected
///
/// Its studentized statistic and every null draw of it are `0.0` (see
/// [`studentized`]), so no draw can strictly exceed it; the exact rule would
/// count that as the strongest evidence the draws can express, earned by no
/// variation. It still enters the maxima as it always did, and the stepdown
/// stops at it.
///
/// # Cost
///
/// O(S·N + B·N + S·B·R + S·B + S log S) time for S strategies, B draws, N
/// periods and R runs per draw, R ≈ N/`block` expected (the per-suffix
/// selection is `select_nth_unstable_by`, whose documentation says its
/// fallback "guarantees linear runtime for all inputs"), and
/// O(S·N + B·R + B + S) space. The S·B·R term needs every series' fold to be
/// provably exact (`N x max|v| <= 2^53`); a series outside it keeps the O(N)
/// fold, so the worst case is still O(S·B·N) (D-2316). The former loop was
/// O(rounds·B·S·N). Not measured by a bench; `CLAUDE.md` §3 rule 6.
fn romano_wolf_aligned(
    returns: &[Vec<i64>],
    periods: usize,
    draws: usize,
    seed: u64,
    block: usize,
    alpha_ppm: u64,
) -> Vec<Rejected> {
    // Outside the probability domain there is no procedure to run, and zero
    // draws is no resample: both are refused as empty, which is what this
    // legacy vector renders malformed input as.
    if alpha_ppm > 1_000_000 || draws == 0 {
        return Vec::new();
    }
    // The largest numerator `1 + count` that still rejects.
    let admissible = usize::try_from(
        (draws as u128)
            .saturating_add(1)
            .saturating_mul(u128::from(alpha_ppm))
            / 1_000_000,
    )
    .unwrap_or(usize::MAX);
    if admissible == 0 {
        return Vec::new();
    }

    let stats: Vec<Performance> = returns.iter().map(|r| summarise(r)).collect();
    let observed: Vec<f64> = stats
        .iter()
        .map(|stat| studentized(stat.mean, stat.standard_error))
        .collect();
    let mut order: Vec<usize> = (0..returns.len()).collect();
    order.sort_unstable_by(
        |left, right| match (observed.get(*left), observed.get(*right)) {
            (Some(left_value), Some(right_value)) => right_value
                .total_cmp(left_value)
                .then_with(|| left.cmp(right)),
            _ => left.cmp(right),
        },
    );

    // ONE RESAMPLE SET, DRAWN ONCE AND HELD ACROSS THE STEPDOWN.
    //
    // Fresh indices per round, from `Rng::new(seed + round)`, once let the
    // threshold RISE after a strategy was removed: measured 32.536 -> 33.906 at
    // seed 97, in 19 of 400 configurations. Romano & Wolf's construction is one
    // B x n resample matrix, and `Rng::new(seed)` once keeps §3 rule 5.
    let mut rng = Rng::new(seed);
    let held = held_draws(periods, block, draws, &mut rng);
    let prefixes = exact_prefixes(returns, periods);

    let mut maxima = vec![f64::NEG_INFINITY; draws];
    let mut scratch: Vec<f64> = Vec::with_capacity(draws);
    let mut bars = vec![f64::INFINITY; order.len()];
    for rank in (0..order.len()).rev() {
        let Some(&strategy) = order.get(rank) else {
            continue;
        };
        let (Some(series), Some(own)) = (returns.get(strategy), stats.get(strategy)) else {
            continue;
        };
        let prefix = prefixes.get(strategy).and_then(Option::as_ref);
        for (maximum, draw) in maxima.iter_mut().zip(&held) {
            *maximum = maximum.max(null_statistic(series, prefix, draw, own));
        }
        // A statistic `x` rejects against these maxima iff fewer than
        // `admissible` of them strictly exceed it, i.e. iff `x` is at least
        // the `admissible`-th largest. More admissible than draws rejects
        // anything.
        let bar = match draws.checked_sub(admissible) {
            None => f64::NEG_INFINITY,
            Some(position) => {
                scratch.clear();
                scratch.extend_from_slice(&maxima);
                let (_, kth, _) = scratch.select_nth_unstable_by(position, f64::total_cmp);
                *kth
            }
        };
        if let Some(slot) = bars.get_mut(rank) {
            *slot = bar;
        }
    }

    let mut out: Vec<Rejected> = Vec::new();
    let mut start = 0_usize;
    let mut round = 0_usize;
    while let Some(&bar) = bars.get(start) {
        let mut end = start;
        while let Some(&strategy) = order.get(end) {
            let (Some(&statistic), Some(own)) = (observed.get(strategy), stats.get(strategy))
            else {
                break;
            };
            if own.standard_error > 0.0 && statistic >= bar {
                end = end.saturating_add(1);
            } else {
                break;
            }
        }
        let Some(rejected_now) = order.get(start..end).filter(|now| !now.is_empty()) else {
            break;
        };
        // Within a round, caller position: every one of them cleared the same
        // bar, which is the order this vector has always reported them in.
        let first = out.len();
        out.extend(
            rejected_now
                .iter()
                .map(|&strategy| Rejected { strategy, round }),
        );
        if let Some(this_round) = out.get_mut(first..) {
            this_round.sort_unstable_by_key(|rejected| rejected.strategy);
        }
        start = end;
        round = round.saturating_add(1);
    }
    out
}

#[cfg(test)]
thread_local! {
    /// Null statistics [`romano_wolf_aligned`] has computed on this thread,
    /// so a test can count the stepdown's work rather than time it.
    static NULL_STATISTICS: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
}

/// One strategy's recentred, studentized statistic on one resample.
fn null_statistic(
    series: &[i64],
    prefix: Option<&ExactPrefix>,
    draw: &Resample,
    own: &Performance,
) -> f64 {
    #[cfg(test)]
    NULL_STATISTICS.with(|count| count.set(count.get().saturating_add(1)));
    studentized(
        resampled_mean(series, prefix, draw) - own.mean,
        own.standard_error,
    )
}

/// Every series is the same non-zero length, and that length.
fn aligned(returns: &[Vec<i64>]) -> Option<usize> {
    let first = returns.first()?.len();
    if first == 0 || returns.iter().any(|r| r.len() != first) {
        return None;
    }
    Some(first)
}

/// [`aligned`], refused when the average block is longer than the series.
///
/// A block past the period count resamples almost every draw as one rotation of
/// the sample, so the null distribution collapses onto the observed statistic:
/// measured over 400 periods of pure noise, a block of 40,000 put 36 of 40
/// families under p = 0.05 where a block of 10 put 2 (D-0742's open half,
/// refused by D-1990). Refused rather than clamped: a block the caller did not
/// ask for is an answer to a different question.
fn aligned_for(returns: &[Vec<i64>], block: usize) -> Option<usize> {
    aligned(returns).filter(|&periods| block <= periods)
}

/// Mean and standard error of one series.
pub(crate) fn summarise(series: &[i64]) -> Performance {
    let n = series.len();
    if n == 0 {
        return Performance::default();
    }
    let mean = series.iter().fold(0.0_f64, |a, &x| a + x as f64) / n as f64;
    if n < 2 {
        return Performance {
            mean,
            standard_error: 0.0,
            periods: n,
        };
    }
    let variance = series
        .iter()
        .fold(0.0_f64, |a, &x| a + (x as f64 - mean).powi(2))
        / (n as f64 - 1.0);
    Performance {
        mean,
        standard_error: (variance / n as f64).sqrt(),
        periods: n,
    }
}

/// A statistic divided by its standard error, or **zero** when that error is
/// zero.
///
/// A zero standard error means the series never varied. Dividing would give
/// infinity, which reads as the strongest result ever recorded rather than as a
/// degenerate one — the same trap `crate::outcome` documents about its own
/// t-statistic.
///
/// # It used to return the RAW statistic here, and that was the same trap wearing
/// a different hat
///
/// The guard avoided `inf` and then handed the raw paisa mean into a maximum
/// otherwise taken over t-RATIOS. Nothing could beat it: every recentred draw for
/// a constant series is exactly zero, so the observed value stood unopposed and
/// both [`reality_check`] and [`spa`] returned p = 0.0 for a strategy whose
/// returns never moved. Measured on this tree, `spa` over a constant series of 7
/// paisa beside real noise reported statistic 7.0, p 0.0000, `clears() == true`.
///
/// That it was a UNIT error rather than a defensible reading is settled by two
/// runs of the same shape: a constant of 1 scored p = 0.065 and failed, a
/// constant of 2 scored p = 0.017 and passed. Two qualitatively identical
/// zero-risk strategies, opposite verdicts, decided by paisa magnitude alone —
/// and at 1 the riskless strategy ranked BELOW an insignificant noise series.
///
/// `crate::outcome` had already answered this question for its own t-statistic:
/// "not an infinitely strong result -- it is a degenerate sample, and reporting
/// it as zero refuses to dress one up as the other." This is that same answer.
/// A degenerate sample is not evidence, so it contributes none.
pub(crate) fn studentized(statistic: f64, standard_error: f64) -> f64 {
    if standard_error > 0.0 {
        statistic / standard_error
    } else {
        0.0
    }
}

/// Indices for one stationary-bootstrap draw.
///
/// Each position either continues the previous block or starts a new one at a
/// random place, with the block-continuation probability set so the average
/// block length is `block`. Politis & Romano's construction, and the reason the
/// resample keeps serial dependence that a single-period resample destroys.
fn stationary_indices(periods: usize, block: usize, rng: &mut Rng) -> Vec<usize> {
    let mut out = Vec::with_capacity(periods);
    stationary_indices_into(&mut out, periods, block, rng);
    out
}

/// `draws` stationary-bootstrap draws held as their runs, for a procedure that
/// reads every draw once per stepdown rank.
///
/// One index buffer is reused and each draw keeps only its runs, so the held
/// set is O(B·R) words for R runs per draw -- at most two words per period,
/// about `2·periods/block` words at the expected block length -- rather than
/// B·N indices. The `rng` stream is consumed exactly as the index form
/// consumed it (D-2316).
fn held_draws(periods: usize, block: usize, draws: usize, rng: &mut Rng) -> Vec<Resample> {
    let mut index = Vec::with_capacity(periods);
    (0..draws)
        .map(|_| {
            stationary_indices_into(&mut index, periods, block, rng);
            Resample::of(&index)
        })
        .collect()
}

/// [`stationary_indices`] into a caller's buffer, which is cleared first.
///
/// The draw loops of [`reality_check`], [`spa`] and both receipts hold one
/// buffer for every draw, so B draws allocate once rather than B times (Rust
/// and O(1) sweep OE-2, D-2305, proved by
/// `runner::bootstrap::a_reused_index_buffer_draws_exactly_what_a_fresh_one_does`).
/// The indices are the same: one `rng` stream,
/// consumed in the same order.
fn stationary_indices_into(out: &mut Vec<usize>, periods: usize, block: usize, rng: &mut Rng) {
    out.clear();
    out.reserve(periods);
    if periods == 0 {
        return;
    }
    // Continue-probability as parts per million, so the draw stays integer.
    let carry_on = if block <= 1 {
        0
    } else {
        1_000_000_u64.saturating_sub(1_000_000 / block as u64)
    };
    let mut at = rng.below(periods);
    for _ in 0..periods {
        out.push(at);
        at = if rng.chance(carry_on) {
            // Wrapping, which is what makes it STATIONARY: without the wrap the
            // last periods would be sampled less often than the first and the
            // resample would be biased toward the start of the series.
            at.saturating_add(1) % periods
        } else {
            rng.below(periods)
        };
    }
}

#[cfg(test)]
std::thread_local! {
    /// Resampled means evaluated on this thread, one per strategy per draw,
    /// whichever path [`resampled_mean`] took (D-1197; the name is the
    /// former `mean_at`'s, which D-2316 replaced).
    static MEAN_AT_CALLS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
    /// Terms a resampled mean READ on this thread: one per run on the prefix
    /// path, one per resampled index on the fold path (D-2316).
    static MEAN_TERMS: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

/// f64's contiguous-integer range, `2^53`.
///
/// While every partial sum of the fold is an integer no larger than this in
/// magnitude, every `a + x as f64` is exact, so the fold IS the exact integer
/// sum converted once to f64 (D-2316).
const EXACT_SUM_LIMIT: u128 = 1 << 53;

/// One series' integer prefix sums, padded with zeros out to `periods`.
///
/// Built only when the f64 fold over any `periods` of its values is exact:
/// every `|v| <= 2^53` and `periods x max|v| <= 2^53`, computed in `u128` so
/// the check itself cannot overflow. The zero padding is the fold's
/// `unwrap_or(0)` for a series shorter than `periods`.
#[derive(Debug)]
struct ExactPrefix {
    /// `sums[k]` is the sum of the first `k` padded values; `periods + 1` long.
    sums: Vec<i64>,
    /// The largest `|v|` among the first `periods` values.
    magnitude: u128,
}

impl ExactPrefix {
    /// `None` when the fold over this series is not provably exact -- the
    /// caller then keeps the fold, which gives the same bits more slowly.
    fn new(series: &[i64], periods: usize) -> Option<Self> {
        let magnitude = series
            .iter()
            .take(periods)
            .map(|v| u128::from(v.unsigned_abs()))
            .max()
            .unwrap_or(0);
        let span = u128::try_from(periods).ok()?.checked_mul(magnitude)?;
        if magnitude > EXACT_SUM_LIMIT || span > EXACT_SUM_LIMIT {
            return None;
        }
        let mut sums = Vec::with_capacity(periods.checked_add(1)?);
        let mut total = 0_i64;
        sums.push(total);
        for at in 0..periods {
            total = total.checked_add(series.get(at).copied().unwrap_or(0))?;
            sums.push(total);
        }
        Some(Self { sums, magnitude })
    }

    /// The exact integer sum of the padded series over `draw`, or `None` when
    /// it is not provably the fold's value: the draw is longer than the bound
    /// was checked for, or a run leaves the padded range (where the fold may
    /// read a value past `periods`).
    fn sum(&self, draw: &Resample) -> Option<i64> {
        let span = u128::try_from(draw.len).ok()?.checked_mul(self.magnitude)?;
        if span > EXACT_SUM_LIMIT {
            return None;
        }
        let mut total = 0_i64;
        for &(start, last) in &draw.runs {
            let high = *self.sums.get(last.checked_add(1)?)?;
            let low = *self.sums.get(start)?;
            total = total.checked_add(high.checked_sub(low)?)?;
        }
        #[cfg(test)]
        MEAN_TERMS.with(|n| n.set(n.get() + draw.runs.len() as u64));
        Some(total)
    }
}

/// Each strategy's [`ExactPrefix`], `None` where the fold is kept. Built once
/// per call, not per draw.
fn exact_prefixes(returns: &[Vec<i64>], periods: usize) -> Vec<Option<ExactPrefix>> {
    returns
        .iter()
        .map(|series| ExactPrefix::new(series, periods))
        .collect()
}

/// One draw as its contiguous runs, in draw order.
///
/// A stationary-bootstrap draw continues a block by `at + 1` and jumps
/// otherwise, so it is a sequence of runs of consecutive indices; the wrap
/// from the last period to 0 starts a new run. `(start, last)` is inclusive,
/// so the runs reproduce every index of the draw in its original order.
#[derive(Debug, Default)]
struct Resample {
    runs: Vec<(usize, usize)>,
    /// Indices in the draw: the mean's divisor.
    len: usize,
}

impl Resample {
    /// The runs of `index`.
    fn of(index: &[usize]) -> Self {
        let mut out = Self::default();
        out.fill(index);
        out
    }

    /// Replaces this draw's runs with those of `index`: O(len) once per draw,
    /// shared by every strategy.
    fn fill(&mut self, index: &[usize]) {
        self.runs.clear();
        self.len = index.len();
        for &at in index {
            match self.runs.last_mut() {
                Some((_, last)) if last.checked_add(1) == Some(at) => *last = at,
                _ => self.runs.push((at, at)),
            }
        }
    }

    /// The draw's indices, in their original order.
    fn indices(&self) -> impl Iterator<Item = usize> + '_ {
        self.runs.iter().flat_map(|&(start, last)| start..=last)
    }
}

/// Mean of `series` over one draw.
///
/// The value is the former `mean_at`'s, bit for bit: the fold
/// `index.iter().fold(0.0, |a, &i| a + series.get(i).copied().unwrap_or(0) as f64)`
/// divided by `index.len() as f64`, which the tests keep as their reference.
/// O(runs) through `prefix` when it is present and provably exact for this
/// draw; otherwise that same fold over the same indices in the same order,
/// O(len) (D-2316).
fn resampled_mean(series: &[i64], prefix: Option<&ExactPrefix>, draw: &Resample) -> f64 {
    #[cfg(test)]
    MEAN_AT_CALLS.with(|n| n.set(n.get() + 1));
    if draw.len == 0 {
        return 0.0;
    }
    let sum = if let Some(exact) = prefix.and_then(|exact| exact.sum(draw)) {
        exact as f64
    } else {
        #[cfg(test)]
        MEAN_TERMS.with(|n| n.set(n.get() + draw.len as u64));
        draw.indices()
            .fold(0.0, |a, i| a + series.get(i).copied().unwrap_or(0) as f64)
    };
    sum / draw.len as f64
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{DEFAULT_BLOCK, Verdict, reality_check, romano_wolf, romano_wolf_receipt, spa};

    /// Two f64 statistics that must be the same number.
    ///
    /// `assert_eq!` on a float is denied workspace-wide, and rightly -- a
    /// computed float rarely lands on an exact value. These comparisons are
    /// different: both sides come from the SAME computation over the SAME
    /// input, so bit equality is the property being asserted rather than an
    /// approximation being hoped for. That is what determinism MEANS.
    fn same(a: f64, b: f64, what: &str) {
        assert!(
            a.to_bits() == b.to_bits(),
            "{what}: {a} and {b} differ bit for bit"
        );
    }

    /// A series that is pure noise around zero, deterministic per `seed`.
    fn noise(n: usize, seed: u64) -> Vec<i64> {
        let mut r = super::Rng::new(seed);
        (0..n)
            .map(|_| i64::try_from(r.next_u64() % 200).unwrap_or(0) - 100)
            .collect()
    }

    /// A series with a genuine positive edge on top of the same noise.
    fn edged(n: usize, seed: u64, edge: i64) -> Vec<i64> {
        noise(n, seed).into_iter().map(|x| x + edge).collect()
    }

    /// A STRATEGY THAT NEVER VARIED IS DEGENERATE, NOT UNBEATABLE — IN THE
    /// STUDENTIZED TESTS.
    ///
    /// `studentized` used to return the RAW statistic when the standard error was
    /// zero. [`spa`] and [`romano_wolf`] take a maximum over t-RATIOS, so that fed
    /// a paisa mean into a comparison of dimensionless quantities; and since every
    /// recentred draw for a constant series is exactly zero, the observed value
    /// could not be beaten. Measured before the fix: `spa` over a constant series
    /// of 7 paisa beside real noise reported statistic 7.0, p 0.0000, and cleared.
    ///
    /// The give-away that it was a UNIT error rather than a defensible "a riskless
    /// edge really is significant" reading is that the verdict moved with
    /// MAGNITUDE: a constant of 1 failed at p = 0.065 while a constant of 2 passed
    /// at p = 0.017 — same strategy shape, opposite answers — and at 1 the riskless
    /// series ranked below insignificant noise. Several magnitudes are asserted
    /// here for exactly that reason: a test at one magnitude would have passed
    /// against the broken function at another.
    ///
    /// # Why [`reality_check`] is deliberately NOT asserted here
    ///
    /// It never called `studentized`. White's statistic is `sqrt(n) * mean`,
    /// un-studentized, so both the observed value and every bootstrap draw are in
    /// paisa and the comparison is already in one unit. A riskless positive mean
    /// genuinely does beat what resampling noise produces, and rejecting the null
    /// there is the right answer rather than a scale artefact. Asserting the
    /// opposite would pin a bug into the suite.
    #[test]
    fn a_zero_variance_strategy_does_not_clear_the_studentized_tests() {
        for constant in [1_i64, 2, 7, 500] {
            let set = vec![vec![constant; 200], noise(200, 1)];

            let v = spa(&set, 1_000, 3, DEFAULT_BLOCK).expect("a verdict");
            assert!(
                !v.clears(),
                "SPA cleared a constant series of {constant} paisa at statistic \
                 {}: a sample with no spread carries no t-ratio, whatever its mean",
                v.statistic
            );

            // Romano–Wolf reaches the same `studentized` and decides PER strategy,
            // so the constant series must not be among those it names.
            // 50_000 ppm = 5% FWER, the alpha every other row here uses.
            let named = romano_wolf(&set, 1_000, 3, DEFAULT_BLOCK, 50_000);
            assert!(
                !named.iter().any(|r| r.strategy == 0),
                "Romano-Wolf named the zero-variance strategy at {constant} paisa; \
                 it reaches the same `studentized` and must reach the same answer"
            );
        }
    }

    /// The block these short-sample tests resample with. D-1990 refuses a
    /// block longer than the sample, and each of them has fewer periods than
    /// `DEFAULT_BLOCK`, so they draw one period at a time; what they pin is the
    /// period count, not the block (D-1934).
    const SHORT_SAMPLE_BLOCK: usize = 1;

    /// A ONE-PERIOD SERIES HAS NOTHING TO RESAMPLE, SO IT CARRIES NO EVIDENCE.
    ///
    /// `aligned` refuses an empty series and a length mismatch and nothing else,
    /// so a single period reached the full machinery. There the stationary
    /// bootstrap can only draw index 0, every draw reproduces the sample, the
    /// null distribution is a point mass at zero and **any** positive value
    /// scored p = 0.0 and cleared — measured at 1, 7, 500 and 1,000,000 paisa
    /// alike, which is the tell that the number was not being tested at all.
    ///
    /// Measured false-positive rate on pure noise against a nominal 5%, before
    /// the guard: 73.5% at 1 period, 45.9% at 2, 37.1% at 3, 21.2% at 10, 13.3%
    /// at 30, 7.4% at 100. This guard closes only the structural case at the top
    /// of that table — a sample that cannot be resampled at all. The remaining
    /// small-sample miscalibration is a real limit and is recorded in
    /// `docs/06-limits.md` rather than fixed by a threshold this crate would have
    /// to invent.
    #[test]
    fn a_single_period_carries_no_evidence_rather_than_certainty() {
        for magnitude in [1_i64, 7, 500, 1_000_000] {
            let one = vec![vec![magnitude]];

            let rc = reality_check(&one, 1_000, 3, SHORT_SAMPLE_BLOCK).expect("a verdict");
            same(rc.p_value, 1.0, "one period cannot be resampled");
            assert!(
                !rc.clears(),
                "Reality Check cleared a single period of {magnitude} paisa"
            );

            let v = spa(&one, 1_000, 3, SHORT_SAMPLE_BLOCK).expect("a verdict");
            same(v.p_value, 1.0, "one period cannot be resampled");
            assert!(
                !v.clears(),
                "SPA cleared a single period of {magnitude} paisa"
            );
        }

        // TWO periods is the shortest series the bootstrap can actually vary, so
        // it is NOT refused here. The guard is structural, not a calibration
        // threshold — asserting otherwise would smuggle in the number this crate
        // declined to invent.
        let two = vec![vec![10_i64, 20]];
        let v = spa(&two, 1_000, 3, SHORT_SAMPLE_BLOCK).expect("a verdict");
        assert!(
            v.p_value <= 1.0,
            "two periods still produce a computed p-value, not the guard's 1.0"
        );
    }

    /// THE VERDICT CARRIES WHAT ITS SAMPLE SIZE IS WORTH.
    ///
    /// A p-value keeps no record of how much evidence produced it, so a verdict
    /// from three periods and one from three hundred render identically and mean
    /// entirely different things. `docs/06-limits.md` §77 measures the gap:
    /// 37.1% false positives at three periods against a nominal 5%.
    ///
    /// This crate does not pick a minimum-period floor — that number is one
    /// `CLAUDE.md` §3 rule 1 forbids it inventing, with no charter source to take
    /// it from. What §3 rule 6 requires instead is that the limit be STATED, and
    /// this is where it is stated.
    #[test]
    fn a_verdict_reports_the_sample_size_it_had_and_what_that_is_worth() {
        for periods in [2_usize, 3, 5, 10, 30, 100, 400] {
            let set = vec![noise(periods, 4), noise(periods, 5)];
            let v = spa(&set, 200, 6, SHORT_SAMPLE_BLOCK).expect("a verdict");
            assert_eq!(v.periods, periods, "the sample size travels with it");
            assert!(
                v.calibration().contains("period"),
                "and is described in measured terms: {}",
                v.calibration()
            );
        }

        // The bands take the WORSE of two measured rows rather than
        // interpolating, because rounding a false-positive rate toward the
        // flattering side is the error this whole module exists to avoid.
        let band = |n: usize| {
            Verdict {
                statistic: 0.0,
                p_value: 1.0,
                draws: 1,
                strategies: 1,
                periods: n,
            }
            .calibration()
        };
        assert!(band(29).contains("21.2%"), "29 takes the 10-period row");
        assert!(band(30).contains("13.3%"), "30 takes its own row");
        assert!(band(99).contains("13.3%"), "99 still takes the 30 row");
        assert!(band(100).contains("7.4%"), "100 takes its own row");
        assert!(
            band(1).contains("refused"),
            "one period is refused outright"
        );
        assert!(band(0).contains("refused"), "and so is none");
        assert!(band(10_000).contains("nominal"), "a long sample is nominal");
    }

    /// The fix must not have made the tests unable to find a REAL edge.
    ///
    /// A guard that refuses everything passes the test above and is worthless.
    /// This is the other side of it: genuine signal still clears.
    #[test]
    fn a_real_edge_still_clears_after_the_degenerate_guard() {
        let set = vec![edged(200, 2, 60), noise(200, 3)];
        assert!(
            spa(&set, 1_000, 5, DEFAULT_BLOCK)
                .expect("a verdict")
                .clears(),
            "a genuine edge over real noise must still be found"
        );
    }

    #[test]
    fn the_spa_statistic_is_hansens_scale_and_not_root_n_times_it() {
        // THE FIX FOR THIS HAD NO DETECTING COVERAGE. An independent audit
        // reverted the whole SPA correction -- `root_n` restored, both `max(.,0)`
        // floors deleted -- and the suite stayed at 185 passed, 0 failed. A fix
        // nothing can fail is not a fix, it is a hope with a commit message.
        //
        // Hansen's statistic is `sqrt(n) * mean / sigma`, and `summarise` sets
        // `standard_error = sqrt(variance / n)` = `sigma / sqrt(n)`. So the
        // statistic IS `mean / standard_error` and multiplying by `root_n`
        // first gives `n * mean / sigma` -- too large by exactly `sqrt(n)`,
        // growing without bound in the sample size.
        //
        // That is the scale this pins. One strategy, so the max is that
        // strategy, and the expected value is computed here from the series
        // rather than copied from a run.
        let series = edged(400, 11, 40);
        let n = series.len();
        let mean = series.iter().fold(0.0_f64, |a, &x| a + x as f64) / n as f64;
        let variance = series
            .iter()
            .fold(0.0_f64, |a, &x| a + (x as f64 - mean).powi(2))
            / (n as f64 - 1.0);
        let expected = mean / (variance / n as f64).sqrt();
        assert!(
            expected > 1.0,
            "the fixture must have a real edge or the scale test is degenerate"
        );

        let v = spa(&[series], 200, 5, DEFAULT_BLOCK).expect("a verdict");
        let ratio = v.statistic / expected;
        assert!(
            (ratio - 1.0).abs() < 1e-9,
            "SPA reported {} where Hansen's statistic is {expected}; ratio {ratio}. \
             A ratio near sqrt(n) = {} is the root_n inflation returning",
            v.statistic,
            (n as f64).sqrt()
        );
    }

    #[test]
    fn the_spa_statistic_is_floored_at_zero_on_a_hopeless_set() {
        // The other half of the same revert: Hansen's `T_SPA = max_k max(., 0)`.
        // Without the floor a set where every strategy loses reports a NEGATIVE
        // statistic, which the test does not define. A verdict printing a
        // negative statistic beside a p-value is a number that looks like a
        // measurement and is not one.
        let losing: Vec<Vec<i64>> = (0..4).map(|s| edged(300, 40 + s, -80)).collect();
        let v = spa(&losing, 200, 5, DEFAULT_BLOCK).expect("a verdict");
        assert!(
            v.statistic >= 0.0,
            "SPA reported a negative statistic {}; Hansen's T_SPA is floored at zero",
            v.statistic
        );
        // And the floor must not have been applied to the observed statistic
        // alone: if the draws were left unfloored their maximum would sit below
        // an observed zero on nearly every draw, driving the p-value to zero and
        // calling a set of pure losers significant.
        assert!(
            v.p_value > 0.05,
            "a set in which every strategy loses cleared 5%: p = {}. The floor is \
             on the observed statistic but not on the bootstrap draws",
            v.p_value
        );
    }

    #[test]
    fn the_same_seed_gives_the_same_p_value_every_time() {
        // CLAUDE.md section 3 rule 5. A resampling test seeded from the clock
        // would make a rerun disagree with the run it repeats, which is the
        // opposite of a check.
        let set = vec![noise(200, 1), noise(200, 2), edged(200, 3, 5)];
        let a = reality_check(&set, 200, 42, DEFAULT_BLOCK).expect("a verdict");
        for _ in 0..4 {
            let b = reality_check(&set, 200, 42, DEFAULT_BLOCK).expect("a verdict");
            assert_eq!(a, b, "two runs of one seed disagreed");
        }
        // A different seed must be ABLE to give a different draw. Asserted as
        // "the seed reached the generator" rather than "the answer changed":
        // two seeds can legitimately agree on a p-value, and asserting they
        // differ would be a test that fails on a correct implementation.
        let different = reality_check(&set, 200, 43, DEFAULT_BLOCK).expect("a verdict");
        same(
            a.statistic,
            different.statistic,
            "the observed statistic does not depend on the seed at all",
        );
    }

    #[test]
    fn a_set_of_pure_noise_does_not_clear_the_reality_check() {
        // The whole point. Twenty worthless strategies, and the best of them
        // must not look real -- if this clears, the test is not testing.
        let set: Vec<Vec<i64>> = (0..20).map(|s| noise(300, s)).collect();
        let v = reality_check(&set, 300, 7, DEFAULT_BLOCK).expect("a verdict");
        assert!(
            !v.clears(),
            "the best of twenty noise series cleared at p = {}",
            v.p_value
        );
    }

    /// WHITE, SPA AND ROMANO-WOLF JUDGE ONE 5% WITH ONE BOUNDARY.
    /// hunt-runner-2, D-1548.
    ///
    /// Romano-Wolf rejects when `(1 + count) / (B + 1) <= alpha`, the usual
    /// "reject when p <= alpha". `Verdict::clears` was `p < 0.05`, so at 19
    /// draws a strategy no draw beat had p = 1/20 = 0.05 exactly: the stepdown
    /// rejected it and White and SPA, printed beside it "at the same 5%",
    /// said it did not clear.
    #[test]
    fn an_exact_five_percent_p_value_clears_where_the_stepdown_rejects() {
        let set = vec![edged(300, 5, 500)];
        let white = reality_check(&set, 19, 7, DEFAULT_BLOCK).expect("a verdict");
        let hansen = spa(&set, 19, 7, DEFAULT_BLOCK).expect("a verdict");
        let stepdown = romano_wolf(&set, 19, 7, DEFAULT_BLOCK, 50_000);
        // Bit-exact on purpose: the boundary case is p EQUAL to 0.05.
        let five = 0.05_f64.to_bits();
        assert_eq!(
            white.p_value.to_bits(),
            five,
            "premise: no draw beat the edge"
        );
        assert_eq!(
            hansen.p_value.to_bits(),
            five,
            "premise: no draw beat the edge"
        );
        assert_eq!(stepdown.len(), 1, "premise: the stepdown rejects at 5%");
        assert!(white.clears(), "White at p = 0.05");
        assert!(hansen.clears(), "SPA at p = 0.05");
    }

    #[test]
    fn a_large_genuine_edge_clears_where_noise_does_not() {
        // One strategy with a real edge among nineteen worthless ones.
        let mut set: Vec<Vec<i64>> = (0..19).map(|s| noise(300, s)).collect();
        set.push(edged(300, 99, 60));
        let v = reality_check(&set, 300, 7, DEFAULT_BLOCK).expect("a verdict");
        assert!(
            v.clears(),
            "a 60-unit edge over 300 periods did not clear: p = {}",
            v.p_value
        );
    }

    #[test]
    fn spa_is_not_weakened_by_adding_hopeless_strategies() {
        // THE DIFFERENCE FROM WHITE'S TEST. Adding rubbish raises White's
        // bootstrap maximum and makes a real edge harder to see. Hansen's gate
        // drops strategies that cannot be best under the null, so it should not
        // degrade nearly as much.
        let mut small: Vec<Vec<i64>> = vec![edged(300, 99, 45)];
        small.extend((0..4).map(|s| noise(300, s)));

        let mut padded = small.clone();
        // Forty strategies that are badly negative -- they cannot be the best
        // under any plausible sample.
        padded.extend((100..140).map(|s| edged(300, s, -400)));

        let s_small = spa(&small, 300, 11, DEFAULT_BLOCK).expect("a verdict");
        let s_padded = spa(&padded, 300, 11, DEFAULT_BLOCK).expect("a verdict");
        let w_padded = reality_check(&padded, 300, 11, DEFAULT_BLOCK).expect("a verdict");

        assert!(
            s_padded.p_value <= w_padded.p_value + 1e-9,
            "SPA must not be weaker than the Reality Check on a padded set: \
             SPA {} vs RC {}",
            s_padded.p_value,
            w_padded.p_value
        );
        assert_eq!(s_small.strategies, 5);
        assert_eq!(s_padded.strategies, 45);
    }

    #[test]
    fn the_stepdown_threshold_never_rises_as_the_surviving_set_shrinks() {
        // THE PROPERTY THE STEPDOWN RESTS ON. Each round takes the bootstrap
        // maximum over the SURVIVING strategies only. Removing a strategy can
        // only lower a maximum, so the threshold must be non-increasing --
        // that monotonicity is why a strategy masked by a stronger one can
        // clear in a later round rather than being lost.
        //
        // It did not hold. `Rng::new(seed + round)` drew FRESH indices every
        // round, so each threshold was a different sample as well as a smaller
        // set. Measured: 32.536 -> 33.906 at seed 97, the bar RISING after a
        // removal; it rose in 19 of 400 configurations. Romano & Wolf hold one
        // B x n resample matrix across the whole stepdown, which is what the
        // implementation now does.
        //
        // # WHAT THIS TEST DOES NOT PROVE, measured rather than assumed
        //
        // It does not exercise monotonicity, because on every fixture tried the
        // stepdown finishes in ONE round: `rejected.round` is `[0, 0]`, so the
        // non-decreasing check below compares `0 >= 0` and holds vacuously.
        //
        // That is a property of the algorithm, not of the fixture. A round
        // rejects EVERY alive strategy above the threshold at once, so a second
        // round needs a strategy that was below the old bar and is above the
        // new one. The bar is the maximum over CENTRED bootstrap draws, so a
        // large mean does not raise it — only a large spread does. Three
        // attempts to force a second round failed: a 300-edge strategy, a
        // marginal 30/34/38 edge beneath it, and a 12x-variance strategy
        // designed to dominate the maximum. All gave `[0, 0]`.
        //
        // So what this DOES hold is the two properties that survive one round:
        // rounds are non-decreasing, and the answer is reproducible from the
        // seed — the latter being what the single-resample-matrix change could
        // most easily have broken.
        //
        // The monotonicity itself is UNVERIFIED by any test. It was verified by
        // instrumentation during the fix — 32.536 rising to 33.906 at seed 97,
        // 19 of 400 configurations before, 0 of 400 after — and closing it
        // properly needs `romano_wolf` to expose its per-round thresholds, which
        // is an API change. Recorded in `docs/06-limits.md`.
        let mut set: Vec<Vec<i64>> = (0..12).map(|s| noise(300, s)).collect();
        // A HIGH-VARIANCE strategy dominates the bootstrap maximum, because the
        // draws are centred on each series own mean -- a large mean does not
        // raise the bar, a large spread does.
        let wide: Vec<i64> = noise(300, 90).into_iter().map(|x| x * 12 + 60).collect();
        set.push(wide);
        set.push(edged(300, 91, 34));

        let rejected = romano_wolf(&set, 300, 11, DEFAULT_BLOCK, 50_000);
        assert!(
            !rejected.is_empty(),
            "nothing was rejected, so the stepdown never stepped"
        );

        // Rounds are assigned in order, so they must be non-decreasing.
        let mut previous = 0_usize;
        for r in &rejected {
            assert!(
                r.round >= previous,
                "rejection rounds went backwards: {} after {}",
                r.round,
                previous
            );
            previous = r.round;
        }

        // Determinism, which the single-matrix change must not have cost:
        // the same seed gives the same answer, a different seed may not.
        let again = romano_wolf(&set, 300, 11, DEFAULT_BLOCK, 50_000);
        assert_eq!(rejected, again, "the same seed gave two different answers");
    }

    /// **One buffer reused across draws yields the draws a fresh vector
    /// does.** Rust and O(1) sweep OE-2, D-2305. Proved by this test,
    /// `runner::bootstrap::a_reused_index_buffer_draws_exactly_what_a_fresh_one_does`.
    ///
    /// Two generators from one seed: one fills a single buffer for every draw,
    /// the other allocates per draw. Every draw's indices are equal, at block
    /// lengths 1, 3 and 50 and at 1 and 97 periods, and the buffer is cleared,
    /// not appended to.
    #[test]
    fn a_reused_index_buffer_draws_exactly_what_a_fresh_one_does() {
        for (periods, block) in [(1_usize, 1_usize), (97, 1), (97, 3), (97, 50)] {
            let (mut reused, mut fresh) = (super::Rng::new(41), super::Rng::new(41));
            let mut buffer = Vec::new();
            for _ in 0..200 {
                super::stationary_indices_into(&mut buffer, periods, block, &mut reused);
                assert_eq!(
                    buffer,
                    super::stationary_indices(periods, block, &mut fresh)
                );
                assert_eq!(buffer.len(), periods);
            }
        }
    }

    #[test]
    fn romano_wolf_names_which_strategies_rather_than_whether_any() {
        // Two genuine edges among eighteen noise series. A stepdown must be able
        // to reject BOTH -- the second is masked by the first until the first is
        // removed, which is the entire mechanism.
        let mut set: Vec<Vec<i64>> = (0..18).map(|s| noise(300, s)).collect();
        set.push(edged(300, 90, 70));
        set.push(edged(300, 91, 65));

        let rejected = romano_wolf(&set, 300, 5, DEFAULT_BLOCK, 50_000);
        let names: Vec<usize> = rejected.iter().map(|r| r.strategy).collect();
        assert!(
            names.contains(&18) && names.contains(&19),
            "both genuine edges must be named, got {names:?}"
        );
        assert!(
            names.len() < 10,
            "a stepdown that rejects half a noise set is not controlling \
             anything: {names:?}"
        );
    }

    #[test]
    fn misaligned_or_empty_input_is_refused_rather_than_answered() {
        // A p-value computed over strategies whose periods do not line up is a
        // number about nothing, and truncating to the shorter would compare one
        // strategy's period against another's.
        assert!(reality_check(&[], 10, 1, DEFAULT_BLOCK).is_none());
        assert!(reality_check(&[vec![]], 10, 1, DEFAULT_BLOCK).is_none());
        assert!(
            reality_check(&[vec![1, 2, 3], vec![1, 2]], 10, 1, DEFAULT_BLOCK).is_none(),
            "series of different lengths must be refused"
        );
        assert!(romano_wolf(&[vec![1, 2], vec![1]], 10, 1, DEFAULT_BLOCK, 50_000).is_empty());
    }

    #[test]
    fn romano_wolf_receipt_distinguishes_complete_non_rejection_from_absence() {
        let complete = romano_wolf_receipt(
            &[vec![-1, 1, -1, 1], vec![1, -1, 1, -1]],
            20,
            7,
            SHORT_SAMPLE_BLOCK,
            50_000,
        )
        .expect("an aligned family with real draws has a receipt");
        assert!(complete.rejected().is_empty());
        assert_eq!(complete.is_rejected(0), Some(false));
        assert_eq!(complete.is_rejected(1), Some(false));
        assert_eq!(complete.is_rejected(2), None);
        assert_eq!(complete.draws(), 20);
        assert_eq!(complete.strategies(), 2);
        assert_eq!(complete.periods(), 4);
        assert_eq!(complete.alpha_ppm(), 50_000);

        assert!(
            romano_wolf_receipt(&[vec![1, 2], vec![1]], 20, 7, SHORT_SAMPLE_BLOCK, 50_000)
                .is_none()
        );
        assert!(romano_wolf_receipt(&[vec![1, 2]], 0, 7, SHORT_SAMPLE_BLOCK, 50_000).is_none());
        assert!(romano_wolf_receipt(&[vec![1, 2]], 20, 7, 0, 50_000).is_none());
        assert!(romano_wolf_receipt(&[vec![1, 2]], 20, 7, SHORT_SAMPLE_BLOCK, 1_000_001).is_none());
    }

    #[test]
    fn zero_draws_reports_no_evidence_rather_than_certainty() {
        // With no bootstrap draws there is nothing to compare against. A
        // p-value of 0 would read as overwhelming evidence; 1 reads as none,
        // which is the truth.
        let set = vec![edged(100, 1, 500)];
        let v = reality_check(&set, 0, 1, DEFAULT_BLOCK).expect("a verdict");
        same(v.p_value, 1.0, "no draws means no evidence");
        assert!(!v.clears(), "no draws cannot clear anything");
    }

    #[test]
    fn a_sample_too_short_for_hansens_gate_keeps_every_strategy() {
        // Hansen's threshold is `-sqrt(2 log log n)`, and `log log n` is
        // undefined below e. A sample too short to take it cannot drop any
        // strategy from the recentring, and the direction that failure falls in
        // matters: keeping everything is CONSERVATIVE, which is where a
        // threshold that cannot be computed has to land.
        //
        // Dropping a strategy on an uncomputable gate would make the test more
        // powerful on exactly the samples that justify it least.
        let set = vec![vec![10_i64, 20, 30], vec![-5_i64, -5, -5]];
        let v = spa(&set, 50, 9, SHORT_SAMPLE_BLOCK).expect("three periods still yield a verdict");
        assert_eq!(v.strategies, 2);
        assert!(
            v.statistic.is_finite(),
            "a three-period sample must still produce a finite statistic"
        );
    }

    #[test]
    fn spa_with_no_draws_reports_no_evidence_rather_than_certainty() {
        // The same rule the Reality Check follows: with nothing to compare
        // against, a p-value of 0 would read as overwhelming evidence. One
        // reads as none, which is the truth.
        let set = vec![edged(100, 1, 500), noise(100, 2)];
        let v = spa(&set, 0, 1, DEFAULT_BLOCK).expect("a verdict");
        same(v.p_value, 1.0, "no draws means no evidence");
        assert!(!v.clears(), "no draws cannot clear anything");
    }

    #[test]
    fn a_series_that_never_varies_does_not_produce_an_infinite_statistic() {
        // Zero standard error would divide to infinity, which reads as the
        // strongest result ever found rather than as a degenerate one.
        let set = vec![vec![7_i64; 200], noise(200, 1)];
        let v = spa(&set, 100, 3, DEFAULT_BLOCK).expect("a verdict");
        assert!(
            v.statistic.is_finite(),
            "a constant series produced a non-finite statistic"
        );
    }

    /// TWO PERIODS IS COMPUTED, NOT REFUSED, by White's test too.
    ///
    /// The `periods < 2` guard is structural: one period cannot be resampled.
    /// Two can, so the guard must not fire at exactly two. With returns of
    /// `[10, 20]` the observed statistic is `sqrt(2) * 15`; the largest
    /// recentred resample is both indices drawing 20, `sqrt(2) * (20 - 15)`,
    /// which never reaches it. So no draw beats the observation and the
    /// p-value is exactly the bootstrap floor `1 / (draws + 1)` -- not the
    /// guard's `1.0`.
    #[test]
    fn two_periods_are_tested_by_the_reality_check_rather_than_refused() {
        let two = vec![vec![10_i64, 20]];
        let rc = reality_check(&two, 1_000, 3, SHORT_SAMPLE_BLOCK).expect("a verdict");
        assert_eq!(rc.periods, 2);
        same(rc.p_value, 1.0 / 1_001.0, "no draw can beat sqrt(2) * 15");
        assert!(rc.clears(), "the floor 1/1001 clears 5%");
    }

    /// The reference fold: `mean_at` exactly as production computed every
    /// resampled mean before D-2316, kept here so the prefix/run path is
    /// compared against the code it replaced rather than against itself.
    fn mean_at(series: &[i64], index: &[usize]) -> f64 {
        if index.is_empty() {
            return 0.0;
        }
        let sum = index.iter().fold(0.0_f64, |a, &i| {
            a + series.get(i).copied().unwrap_or(0) as f64
        });
        sum / index.len() as f64
    }

    /// The largest `|v|` for which `periods x |v| <= 2^53`.
    fn exact_bound(periods: usize) -> i64 {
        let periods = u64::try_from(periods.max(1)).expect("a period count fits u64");
        i64::try_from((1_u64 << 53) / periods).expect("2^53 fits i64")
    }

    /// One draw's mean both ways must be the same f64, bit for bit.
    fn prefix_matches_fold(series: &[i64], periods: usize, index: &[usize], what: &str) {
        let prefix = super::ExactPrefix::new(series, periods);
        let draw = super::Resample::of(index);
        assert_eq!(
            draw.indices().collect::<Vec<usize>>(),
            index,
            "{what}: the runs must reproduce the draw in order"
        );
        assert_eq!(draw.len, index.len(), "{what}: divisor");
        same(
            super::resampled_mean(series, prefix.as_ref(), &draw),
            mean_at(series, index),
            what,
        );
    }

    /// OE-1 / D-2316: the run/prefix mean is `mean_at`'s fold bit for bit over
    /// many seeds, blocks and period counts, at the exactness boundary on both
    /// sides, on short, negative and extreme series, and wherever the prefix
    /// path must decline and the fold answers.
    #[test]
    fn the_run_prefix_mean_is_the_fold_bit_for_bit() {
        let mut fast = 0_usize;
        let mut kept = 0_usize;
        for periods in [0_usize, 1, 2, 3, 7, 64, 225] {
            let bound = exact_bound(periods);
            let alternating = |edge: i64| -> Vec<i64> {
                (0..periods)
                    .map(|t| if t % 3 == 1 { -edge } else { edge })
                    .collect()
            };
            let mut over = alternating(bound);
            if let Some(first) = over.first_mut() {
                *first = bound + 1;
            }
            let families: Vec<(&str, Vec<i64>, bool)> = vec![
                ("noise", noise(periods, 11), true),
                (
                    "negative",
                    noise(periods, 12).iter().map(|v| v - 500).collect(),
                    true,
                ),
                ("short", noise(periods / 2, 13), true),
                ("at the bound", alternating(bound), true),
                ("one past the bound", over, periods == 0),
                (
                    "extreme",
                    (0..periods)
                        .map(|t| if t % 2 == 0 { i64::MIN } else { i64::MAX })
                        .collect(),
                    periods == 0,
                ),
            ];
            for (name, series, exact) in &families {
                assert_eq!(
                    super::ExactPrefix::new(series, periods).is_some(),
                    *exact,
                    "{name} at {periods} periods: exactness"
                );
                if *exact {
                    fast += 1;
                } else {
                    kept += 1;
                }
                for block in [0_usize, 1, 2, DEFAULT_BLOCK, periods + 5] {
                    for seed in 0..24_u64 {
                        let mut rng = super::Rng::new(seed);
                        let index = super::stationary_indices(periods, block, &mut rng);
                        let what = format!("{name} periods {periods} block {block} seed {seed}");
                        prefix_matches_fold(series, periods, &index, &what);
                    }
                }
            }
        }
        assert!(fast > 0 && kept > 0, "both paths must be exercised");

        // Indices past `periods` over a longer series, and a draw longer than
        // the bound was checked for: the prefix path declines and the fold's
        // own reads answer.
        let long: Vec<i64> = (1..=10).collect();
        prefix_matches_fold(&long, 5, &[3, 4, 5, 6], "past the padded range");
        let edge = vec![exact_bound(4); 4];
        prefix_matches_fold(
            &edge,
            4,
            &[0, 1, 2, 3, 0, 1, 2, 3],
            "a draw twice the bound",
        );
        prefix_matches_fold(
            &[7, -3],
            2,
            &[usize::MAX, 0, 1],
            "an index that cannot extend",
        );

        // The guard is load-bearing: here the f64 fold rounds and the exact
        // integer sum does not, and the prefix is refused rather than used.
        let rounding = [1_i64 << 53, 1, 1];
        let exact_sum = ((1_i64 << 53) + 2) as f64;
        assert!(
            mean_at(&rounding, &[0, 1, 2]).to_bits() != (exact_sum / 3.0).to_bits(),
            "the fold must round on this series"
        );
        assert!(super::ExactPrefix::new(&rounding, 3).is_none());
        prefix_matches_fold(&rounding, 3, &[0, 1, 2], "a fold that rounds");
    }

    /// OE-1 / D-2316: the prefix path reads one term per run, not one per
    /// period, both for one draw and across a whole Romano--Wolf stepdown, and
    /// the fold it keeps for an inexact series reads every period.
    #[test]
    fn the_prefix_path_reads_one_term_per_run_not_per_period() {
        let terms = || super::MEAN_TERMS.with(std::cell::Cell::get);
        let periods = 4_000;
        let series = noise(periods, 3);
        let prefix = super::ExactPrefix::new(&series, periods).expect("noise is exact");
        let mut rng = super::Rng::new(9);
        let draw = super::Resample::of(&super::stationary_indices(periods, 50, &mut rng));
        let before = terms();
        let _ = super::resampled_mean(&series, Some(&prefix), &draw);
        let read = terms() - before;
        assert_eq!(read, draw.runs.len() as u64, "one term per run");
        assert!(
            draw.runs.len() * 10 < periods,
            "runs {} must be far fewer than periods {periods}",
            draw.runs.len()
        );

        let extreme = vec![i64::MAX; periods];
        assert!(super::ExactPrefix::new(&extreme, periods).is_none());
        let before = terms();
        let _ = super::resampled_mean(&extreme, None, &draw);
        assert_eq!(
            terms() - before,
            periods as u64,
            "the fold reads every period"
        );

        // Whole stepdown: S strategies x the runs of every held draw.
        let set: Vec<Vec<i64>> = (0..5).map(|s| noise(300, 40 + s)).collect();
        let (draws, seed, block) = (120, 17, DEFAULT_BLOCK);
        let mut rng = super::Rng::new(seed);
        let runs: usize = super::held_draws(300, block, draws, &mut rng)
            .iter()
            .map(|draw| draw.runs.len())
            .sum();
        let before = terms();
        let _ = romano_wolf(&set, draws, seed, block, 50_000);
        let read = terms() - before;
        assert_eq!(read, (set.len() * runs) as u64, "S x total runs");
        assert!(read * 5 < (set.len() * draws * 300) as u64, "not S x B x N");
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod exact_family_test_receipt_tests {
    use super::{
        DEFAULT_BLOCK, Rng, reality_check, spa, spa_receipt_v1, white_reality_check_receipt_v1,
    };

    fn noise(periods: usize, seed: u64) -> Vec<i64> {
        let mut rng = Rng::new(seed);
        (0..periods)
            .map(|_| i64::try_from(rng.next_u64() % 200).unwrap_or(0) - 100)
            .collect()
    }

    fn family() -> Vec<Vec<i64>> {
        vec![
            noise(64, 1),
            noise(64, 2).into_iter().map(|value| value + 15).collect(),
            noise(64, 3),
        ]
    }

    #[test]
    fn exact_counts_bits_and_legacy_verdicts_are_identical() {
        let family = family();
        let white = white_reality_check_receipt_v1(&family, 37, 0, 7)
            .expect("a variable aligned White family");
        let white_legacy = reality_check(&family, 37, 0, 7).expect("legacy White verdict");
        assert_eq!(white.verdict(), white_legacy);
        assert_eq!(white.statistic_bits(), white_legacy.statistic.to_bits());
        assert_eq!(white.p_value_bits(), white_legacy.p_value.to_bits());
        assert_eq!(white.exact_p_value().denominator(), 38);
        assert_eq!(
            white.exact_p_value().numerator(),
            white.matched_or_exceeded() + 1
        );
        assert_eq!(
            white.exact_p_value().value().to_bits(),
            white.p_value_bits()
        );
        assert_eq!(white.draws(), 37);
        assert_eq!(white.strategies(), family.len());
        assert_eq!(white.periods(), 64);
        assert_eq!(white.seed(), 0, "zero remains an explicit legal seed");
        assert_eq!(white.block(), 7);
        assert_eq!(white.statistic().to_bits(), white.statistic_bits());
        assert_eq!(white.p_value().to_bits(), white.p_value_bits());

        let spa_exact = spa_receipt_v1(&family, 37, 0, 7).expect("a variable aligned SPA family");
        let spa_legacy = spa(&family, 37, 0, 7).expect("legacy SPA verdict");
        assert_eq!(spa_exact.verdict(), spa_legacy);
        assert_eq!(spa_exact.statistic_bits(), spa_legacy.statistic.to_bits());
        assert_eq!(spa_exact.p_value_bits(), spa_legacy.p_value.to_bits());
        assert_eq!(spa_exact.exact_p_value().denominator(), 38);
        assert_eq!(
            spa_exact.exact_p_value().numerator(),
            spa_exact.matched_or_exceeded() + 1
        );
        assert_eq!(
            spa_exact.exact_p_value().value().to_bits(),
            spa_exact.p_value_bits()
        );
        assert_eq!(spa_exact.draws(), 37);
        assert_eq!(spa_exact.strategies(), family.len());
        assert_eq!(spa_exact.periods(), 64);
        assert_eq!(spa_exact.seed(), 0);
        assert_eq!(spa_exact.block(), 7);
        assert_eq!(spa_exact.statistic().to_bits(), spa_exact.statistic_bits());
        assert_eq!(spa_exact.p_value().to_bits(), spa_exact.p_value_bits());
        assert_ne!(
            white.family_digest(),
            spa_exact.family_digest(),
            "procedure identity separates White from SPA over identical bytes"
        );
    }

    #[test]
    fn repeated_inputs_are_identical_and_every_identity_term_rekeys() {
        let input_family = family();
        let first =
            white_reality_check_receipt_v1(&input_family, 31, 11, 5).expect("first receipt");
        let again = white_reality_check_receipt_v1(&input_family, 31, 11, 5).expect("same receipt");
        assert_eq!(first, again, "an exact rerun must be byte-identical");

        let changed_draws = white_reality_check_receipt_v1(&input_family, 32, 11, 5)
            .expect("changed draw count remains valid");
        let changed_seed = white_reality_check_receipt_v1(&input_family, 31, 12, 5)
            .expect("changed seed remains valid");
        let changed_block = white_reality_check_receipt_v1(&input_family, 31, 11, 6)
            .expect("changed block remains valid");
        assert_ne!(first.family_digest(), changed_draws.family_digest());
        assert_ne!(first.family_digest(), changed_seed.family_digest());
        assert_ne!(first.family_digest(), changed_block.family_digest());

        let mut changed_value = input_family.clone();
        *changed_value
            .first_mut()
            .and_then(|series| series.first_mut())
            .expect("fixture has a first return") += 1;
        let changed_value = white_reality_check_receipt_v1(&changed_value, 31, 11, 5)
            .expect("changed family remains valid");
        assert_ne!(first.family_digest(), changed_value.family_digest());

        let mut reordered = input_family;
        reordered.swap(0, 2);
        let reordered = white_reality_check_receipt_v1(&reordered, 31, 11, 5)
            .expect("reordered family remains valid");
        assert_ne!(
            first.family_digest(),
            reordered.family_digest(),
            "caller order is part of the family identity"
        );

        let mut extra_candidate = family();
        extra_candidate.push(noise(64, 4));
        let extra_candidate = white_reality_check_receipt_v1(&extra_candidate, 31, 11, 5)
            .expect("a wider family remains valid");
        assert_eq!(extra_candidate.strategies(), 4);
        assert_ne!(first.family_digest(), extra_candidate.family_digest());

        let longer_periods = vec![noise(65, 1), noise(65, 2), noise(65, 3)];
        let longer_periods = white_reality_check_receipt_v1(&longer_periods, 31, 11, 5)
            .expect("a longer aligned family remains valid");
        assert_eq!(longer_periods.periods(), 65);
        assert_ne!(first.family_digest(), longer_periods.family_digest());

        let spa_family = family();
        let spa_first = spa_receipt_v1(&spa_family, 31, 11, 5).expect("first SPA receipt");
        let spa_again = spa_receipt_v1(&spa_family, 31, 11, 5).expect("same SPA receipt");
        assert_eq!(spa_first, spa_again);
        let spa_changed_seed =
            spa_receipt_v1(&spa_family, 31, 12, 5).expect("changed SPA seed remains valid");
        assert_ne!(spa_first.family_digest(), spa_changed_seed.family_digest());
    }

    #[test]
    fn malformed_or_unresampleable_inputs_never_gain_exact_authority() {
        for white in [
            white_reality_check_receipt_v1(&[], 10, 1, DEFAULT_BLOCK),
            white_reality_check_receipt_v1(&[vec![]], 10, 1, DEFAULT_BLOCK),
            white_reality_check_receipt_v1(&[vec![1, 2, 3], vec![1, 2]], 10, 1, DEFAULT_BLOCK),
            white_reality_check_receipt_v1(&[vec![1, 2]], 0, 1, DEFAULT_BLOCK),
            white_reality_check_receipt_v1(&[vec![1, 2]], 10, 1, 0),
            white_reality_check_receipt_v1(&[vec![1]], 10, 1, DEFAULT_BLOCK),
            white_reality_check_receipt_v1(&[vec![1, 2]], usize::MAX, 1, DEFAULT_BLOCK),
        ] {
            assert!(white.is_none());
        }
        for spa_exact in [
            spa_receipt_v1(&[], 10, 1, DEFAULT_BLOCK),
            spa_receipt_v1(&[vec![]], 10, 1, DEFAULT_BLOCK),
            spa_receipt_v1(&[vec![1, 2, 3], vec![1, 2]], 10, 1, DEFAULT_BLOCK),
            spa_receipt_v1(&[vec![1, 2]], 0, 1, DEFAULT_BLOCK),
            spa_receipt_v1(&[vec![1, 2]], 10, 1, 0),
            spa_receipt_v1(&[vec![1]], 10, 1, DEFAULT_BLOCK),
            spa_receipt_v1(&[vec![1, 2]], usize::MAX, 1, DEFAULT_BLOCK),
        ] {
            assert!(spa_exact.is_none());
        }
    }

    #[test]
    fn handled_zero_variance_keeps_named_procedure_compatibility() {
        let constant = vec![vec![7_i64; 32]];
        // White still permits a deterministic nonzero mean BESIDE A ROW THAT
        // VARIES: its null then has spread. A family with none is refused
        // instead (D-0972), which
        // `a_family_that_never_varied_mints_no_white_evidence` pins.
        let mixed = vec![vec![7_i64; 32], noise(32, 5)];
        let white = white_reality_check_receipt_v1(&mixed, 19, 3, 4)
            .expect("White permits a deterministic nonzero mean beside variation");
        assert_eq!(
            white.verdict(),
            reality_check(&mixed, 19, 3, 4).expect("legacy White verdict")
        );
        assert_eq!(white.exact_p_value().denominator(), 20);

        let spa_exact = spa_receipt_v1(&constant, 19, 3, 4)
            .expect("SPA conservatively maps zero standard error to zero");
        assert_eq!(
            spa_exact.verdict(),
            spa(&constant, 19, 3, 4).expect("legacy SPA verdict")
        );
        assert_eq!(spa_exact.matched_or_exceeded(), 19);
        assert_eq!(spa_exact.exact_p_value().numerator(), 20);
        assert_eq!(spa_exact.exact_p_value().denominator(), 20);
    }

    /// A FAMILY IN WHICH NO ROW EVER VARIED EARNS NO WHITE EVIDENCE (D-0972).
    ///
    /// Every recentred draw of a constant row is exactly zero, so with no
    /// varying row White's bootstrap maximum is a point mass at zero. Before
    /// D-0972 a positive constant then counted the strongest probability the
    /// draws can express, 1/1001 at 1,000 draws, for 1 paisa as readily as for
    /// 1,000,000, while SPA answered 1 and Romano--Wolf refused. Several
    /// magnitudes are asserted because the magnitude is the tell.
    #[test]
    fn a_family_that_never_varied_mints_no_white_evidence() {
        for constant in [1_i64, 7, 1_000_000] {
            for family in [
                vec![vec![constant; 100]],
                vec![vec![constant; 100], vec![0; 100], vec![-constant; 100]],
            ] {
                assert_eq!(
                    white_reality_check_receipt_v1(&family, 1_000, 7, DEFAULT_BLOCK),
                    None,
                    "a point-mass null minted exact White evidence at {constant}"
                );
                let legacy = reality_check(&family, 1_000, 7, DEFAULT_BLOCK).expect("a verdict");
                assert!(
                    legacy.p_value.to_bits() == 1.0_f64.to_bits(),
                    "a point-mass null is no evidence, got p = {}",
                    legacy.p_value
                );
                assert!(!legacy.clears(), "a point-mass null cleared at {constant}");
            }
        }

        // A nonpositive statistic against the same point mass already counts
        // every draw, so those receipts are unchanged and conservative.
        for family in [
            vec![vec![0_i64; 100]],
            vec![vec![-5_i64; 100], vec![0; 100]],
        ] {
            let white = white_reality_check_receipt_v1(&family, 1_000, 7, DEFAULT_BLOCK)
                .expect("a nonpositive point mass keeps its conservative receipt");
            assert_eq!(white.matched_or_exceeded(), 1_000);
            assert_eq!(white.exact_p_value().numerator(), 1_001);
            assert_eq!(white.exact_p_value().denominator(), 1_001);
            assert_eq!(
                white.verdict(),
                reality_check(&family, 1_000, 7, DEFAULT_BLOCK).expect("legacy verdict")
            );
        }

        // One varying row gives the null spread, and the constant keeps the
        // measured White answer the named-procedure test above relies on.
        let mixed = vec![vec![1_i64; 100], noise(100, 3)];
        let white = white_reality_check_receipt_v1(&mixed, 1_000, 7, DEFAULT_BLOCK)
            .expect("a family with variation is measured");
        assert!(white.exact_p_value().numerator() < white.exact_p_value().denominator());
        assert_eq!(
            white.verdict(),
            reality_check(&mixed, 1_000, 7, DEFAULT_BLOCK).expect("legacy verdict")
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod adjusted_p_value_tests {
    use super::{Rng, romano_wolf_adjusted_p_values_v1};

    fn noise(periods: usize, seed: u64) -> Vec<i64> {
        let mut rng = Rng::new(seed);
        (0..periods)
            .map(|_| i64::try_from(rng.next_u64() % 200).unwrap_or(0) - 100)
            .collect()
    }

    fn edged(periods: usize, seed: u64, edge: i64) -> Vec<i64> {
        noise(periods, seed)
            .into_iter()
            .map(|value| value + edge)
            .collect()
    }

    #[test]
    fn exact_denominators_seed_and_full_precision_survive_the_receipt() {
        let family = vec![noise(80, 1), edged(80, 2, 15), noise(80, 3)];
        let receipt = romano_wolf_adjusted_p_values_v1(&family, 37, 0, 7)
            .expect("a variable aligned family yields adjusted probabilities");

        assert_eq!(receipt.draws(), 37);
        assert_eq!(receipt.strategies(), 3);
        assert_eq!(receipt.periods(), 80);
        assert_eq!(receipt.seed(), 0, "zero is an explicit legal SplitMix seed");
        assert_eq!(receipt.block(), 7);
        assert!(receipt.family_digest().iter().any(|byte| *byte != 0));
        for strategy in 0..family.len() {
            let row = receipt
                .candidate(strategy)
                .expect("every candidate is present");
            assert_eq!(row.strategy(), strategy);
            assert_eq!(row.adjusted_p_value().denominator(), 38);
            assert_eq!(
                row.initial_p_value().numerator(),
                row.strict_exceedances() + 1
            );
            let projected = row.adjusted_p_value().value();
            let exact_projection = row.adjusted_p_value().numerator() as f64
                / row.adjusted_p_value().denominator() as f64;
            assert_eq!(
                projected.to_bits(),
                exact_projection.to_bits(),
                "the public f64 is the direct exact-fraction projection"
            );
            assert!(row.observed_statistic().is_finite());
        }
    }

    #[test]
    fn canonical_stepdown_probabilities_are_monotone_and_familywise_is_named() {
        let family = vec![
            edged(120, 7, 45),
            noise(120, 8),
            edged(120, 9, 20),
            noise(120, 10),
        ];
        let receipt =
            romano_wolf_adjusted_p_values_v1(&family, 99, 11, 10).expect("a complete family");

        let mut previous = 0_usize;
        for (rank, strategy) in receipt.stepdown_order().iter().copied().enumerate() {
            let row = receipt.candidate(strategy).expect("ordered candidate");
            assert_eq!(row.stepdown_rank(), rank);
            assert!(
                row.adjusted_p_value().numerator() >= previous,
                "Algorithm 4.1 requires nondecreasing adjusted p-values"
            );
            previous = row.adjusted_p_value().numerator();
        }

        let leading = receipt
            .stepdown_order()
            .first()
            .copied()
            .expect("a complete family has a leading candidate");
        let leading_p_value = receipt
            .candidate(leading)
            .expect("the leading candidate is present")
            .adjusted_p_value();
        assert_eq!(
            receipt.familywise_p_value(),
            Some(leading_p_value),
            "the named family-intersection probability is the complete-family maximum test"
        );
    }

    #[test]
    fn monotonicity_repair_is_load_bearing_and_not_a_sorted_output_decoration() {
        // Romano and Wolf (2016) Remark 4.1 says the cumulative maximum is
        // essential because an unadjusted later suffix can otherwise look too
        // optimistic.  Search a fixed, bounded deterministic fixture family
        // for exactly that shape, then require at least one row whose adjusted
        // numerator is strictly larger than its initial numerator.  Replacing
        // the repair with `adjusted = initial` makes this test fail.
        let witness = (0_u64..128).find_map(|seed| {
            let family: Vec<Vec<i64>> = (0_u64..7)
                .map(|candidate| {
                    let edge = i64::try_from(candidate).unwrap_or(0) - 3;
                    edged(72, seed.saturating_mul(17).saturating_add(candidate), edge)
                })
                .collect();
            let receipt = romano_wolf_adjusted_p_values_v1(&family, 63, seed, 6)?;
            receipt
                .stepdown_order()
                .iter()
                .copied()
                .find_map(|strategy| {
                    let row = receipt.candidate(strategy)?;
                    (row.adjusted_p_value().numerator() > row.initial_p_value().numerator())
                        .then_some((seed, strategy))
                })
        });
        assert!(
            witness.is_some(),
            "the suite must contain a row where the mandatory cumulative maximum changes the result"
        );
    }

    #[test]
    fn no_rejection_and_all_rejection_are_both_complete_results() {
        let alternating: Vec<i64> = (0..200)
            .map(|index| if index % 2 == 0 { -20 } else { 20 })
            .collect();
        let inverse: Vec<i64> = alternating.iter().map(|value| -*value).collect();
        let no_edge =
            romano_wolf_adjusted_p_values_v1(&[alternating.clone(), inverse.clone()], 199, 17, 10)
                .expect("a variable zero-mean family is a complete non-rejection");
        assert!(
            (0..2).all(|strategy| {
                !no_edge
                    .candidate(strategy)
                    .expect("candidate")
                    .adjusted_p_value()
                    .rejects_at_ppm(50_000)
                    .expect("valid alpha")
            }),
            "zero-mean alternating candidates must not clear 5%"
        );

        let strong: Vec<Vec<i64>> = [80_i64, 70, 60]
            .into_iter()
            .map(|edge| alternating.iter().map(|value| value + edge).collect())
            .collect();
        let all_edge = romano_wolf_adjusted_p_values_v1(&strong, 199, 17, 10)
            .expect("a variable strong-edge family is complete");
        assert!(
            (0..strong.len()).all(|strategy| {
                all_edge
                    .candidate(strategy)
                    .expect("candidate")
                    .adjusted_p_value()
                    .rejects_at_ppm(50_000)
                    .expect("valid alpha")
            }),
            "every deliberately overwhelming edge must clear the exact 5% boundary"
        );
        assert_eq!(
            all_edge
                .candidate(0)
                .expect("candidate")
                .adjusted_p_value()
                .rejects_at_ppm(1_000_001),
            None,
            "an alpha outside the probability domain is refused"
        );
    }

    #[test]
    fn ties_and_input_permutations_cannot_silently_rename_probabilities() {
        let tied_a: Vec<i64> = (0..120)
            .map(|index| if index % 3 == 0 { 30 } else { -15 })
            .collect();
        let tied_b: Vec<i64> = (0..120)
            .map(|index| match index % 4 {
                0 => 30,
                1 => -30,
                _ => 0,
            })
            .collect();
        let leader = edged(120, 51, 40);
        let original = vec![tied_a.clone(), tied_b.clone(), leader.clone()];
        let permuted = vec![leader, tied_b, tied_a];
        let a = romano_wolf_adjusted_p_values_v1(&original, 79, 23, 8).expect("original family");
        let b = romano_wolf_adjusted_p_values_v1(&permuted, 79, 23, 8).expect("permuted family");

        assert_eq!(
            a.candidate(0)
                .expect("first tied candidate")
                .observed_statistic()
                .to_bits(),
            a.candidate(1)
                .expect("second tied candidate")
                .observed_statistic()
                .to_bits(),
            "the fixture must exercise an exact observed-statistic tie"
        );
        let tied_ranks: Vec<usize> = a
            .stepdown_order()
            .iter()
            .copied()
            .filter(|strategy| *strategy < 2)
            .collect();
        assert_eq!(
            tied_ranks,
            vec![0, 1],
            "caller position is the explicit tie-breaker"
        );

        // Map by series identity after the deliberate permutation: original
        // [A,B,L] becomes [L,B,A].  The probabilities travel with the series,
        // while the ordered-family digest changes so a positional swap cannot
        // masquerade as the same authority.
        for (original_index, permuted_index) in [(0_usize, 2_usize), (1, 1), (2, 0)] {
            assert_eq!(
                a.candidate(original_index)
                    .expect("original candidate")
                    .adjusted_p_value(),
                b.candidate(permuted_index)
                    .expect("permuted candidate")
                    .adjusted_p_value(),
                "adjusted evidence must be permutation-equivariant by candidate identity"
            );
        }
        assert_ne!(
            a.family_digest(),
            b.family_digest(),
            "the receipt binds caller order and cannot be relabelled after computation"
        );
    }

    #[test]
    fn repeated_seed_is_byte_identical_and_any_input_change_rekeys_the_family() {
        let family = vec![noise(96, 4), edged(96, 5, 18), noise(96, 6)];
        let first = romano_wolf_adjusted_p_values_v1(&family, 61, 29, 9).expect("first receipt");
        let again = romano_wolf_adjusted_p_values_v1(&family, 61, 29, 9).expect("repeated receipt");
        assert_eq!(first, again, "same inputs and seed must be byte-identical");

        let another_seed = romano_wolf_adjusted_p_values_v1(&family, 61, 30, 9)
            .expect("different explicit seed remains a valid receipt");
        assert_ne!(
            first.family_digest(),
            another_seed.family_digest(),
            "the seed is part of procedure identity even when two estimates coincide"
        );
        let mut changed = family;
        let first_series = changed
            .first_mut()
            .expect("the constructed family is nonempty");
        let first_value = first_series
            .first_mut()
            .expect("the constructed series is nonempty");
        *first_value += 1;
        let changed =
            romano_wolf_adjusted_p_values_v1(&changed, 61, 29, 9).expect("changed input family");
        assert_ne!(first.family_digest(), changed.family_digest());
    }

    #[test]
    fn zero_or_malformed_or_degenerate_input_is_refused_not_numeric() {
        assert!(romano_wolf_adjusted_p_values_v1(&[], 20, 1, 5).is_none());
        assert!(romano_wolf_adjusted_p_values_v1(&[vec![]], 20, 1, 5).is_none());
        assert!(romano_wolf_adjusted_p_values_v1(&[vec![1, 2], vec![1]], 20, 1, 5).is_none());
        assert!(romano_wolf_adjusted_p_values_v1(&[vec![1, 2]], 0, 1, 5).is_none());
        assert!(romano_wolf_adjusted_p_values_v1(&[vec![1, 2]], 20, 1, 0).is_none());
        assert!(romano_wolf_adjusted_p_values_v1(&[vec![1]], 20, 1, 5).is_none());
        assert!(
            romano_wolf_adjusted_p_values_v1(&[vec![7; 20], noise(20, 4)], 20, 1, 5).is_none(),
            "a zero-variance point mass cannot receive the strict-exceedance floor"
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod stepdown_partition_tests {
    use super::{DEFAULT_BLOCK, romano_wolf};

    /// A LINEAR STEPDOWN, PROVED BY THE SHAPE OF WHAT IT RETURNS.
    ///
    /// # What this replaces, and why the old shape was invisible
    ///
    /// The round's survivors came from
    /// `alive.retain(|s| !rejected_now.contains(s))` — a scan of the rejected
    /// set per survivor, so O(alive × rejected) per round with nothing
    /// structural bounding either. Gate 11 rule 7 exists to refuse exactly that
    /// and could not see it: its pattern is a `.contains(&` and this was
    /// `.contains(s)`, `s` already being a reference.
    ///
    /// # What is actually asserted
    ///
    /// A complexity claim is not testable from inside the crate, so this pins
    /// the two OBSERVABLE properties the rewrite had to preserve, either of
    /// which a careless partition would break:
    ///
    /// 1. **No strategy is rejected twice.** The old predicate removed every
    ///    rejected index from `alive` in one go; a partition that pushed a
    ///    rejected strategy to the survivors would re-reject it next round and
    ///    emit a second row.
    /// 2. **Survivors keep their original order**, which the stepdown depends
    ///    on: `maxima` is taken over `alive` in order, and the resample matrix
    ///    is held across rounds precisely so the threshold is comparable
    ///    between them.
    ///
    /// Thirty-two strategies with clearly separated means, so several rounds
    /// genuinely run rather than the whole set falling in one.
    #[test]
    fn the_stepdown_partition_rejects_each_strategy_at_most_once() {
        let set: Vec<Vec<i64>> = (0..32)
            .map(|s| {
                let level = i64::from(s).saturating_mul(7);
                (0..64)
                    .map(|t| level.saturating_add(if t % 3 == 0 { 4 } else { -1 }))
                    .collect()
            })
            .collect();
        let rejected = romano_wolf(&set, 200, 9, DEFAULT_BLOCK, 50_000);

        let mut seen: Vec<usize> = rejected.iter().map(|r| r.strategy).collect();
        let before = seen.len();
        seen.sort_unstable();
        seen.dedup();
        assert_eq!(
            before,
            seen.len(),
            "a strategy rejected twice means the partition returned it to the \
             surviving set, which `retain` could not do"
        );
        assert!(
            rejected.iter().all(|r| r.strategy < set.len()),
            "every named strategy indexes the input"
        );

        // ROUNDS NEVER GO BACKWARDS. `out` is appended per round, so a
        // partition that left `alive` unordered would produce a round number
        // that dips -- which is also how a survivor set silently growing would
        // announce itself.
        let mut last = 0_usize;
        for r in &rejected {
            assert!(
                r.round >= last,
                "rounds are appended in order: {} after {last}",
                r.round
            );
            last = r.round;
        }

        // AND IT IS STILL DETERMINISTIC. §3 rule 5 is the reason the resample
        // matrix is drawn once; a rewrite of the survivor set must not disturb
        // it.
        let again = romano_wolf(&set, 200, 9, DEFAULT_BLOCK, 50_000);
        assert_eq!(
            rejected, again,
            "same inputs, same rejections, byte for byte"
        );
    }

    /// EVERY ROUND REMOVES AT LEAST ONE, OR THE LOOP STOPS, and no strategy is
    /// rejected twice.
    ///
    /// Termination is STRUCTURAL: a round runs only when `end > start`, and
    /// `start = end` after it, so the walk over the ordered strategies cannot
    /// revisit one. A round number exists only after a non-empty round, so a
    /// per-round count could not fail (P7-02, D-2667). What this asserts is
    /// what a broken partition would change: the fixture must reject something,
    /// each strategy appears at most once, and the rounds are numbered in order
    /// without a gap.
    #[test]
    fn the_surviving_set_strictly_shrinks_every_round_that_rejects() {
        let set: Vec<Vec<i64>> = (0..12)
            .map(|s| {
                let level = i64::from(s).saturating_mul(11);
                (0..48)
                    .map(|t| level + if t % 4 == 0 { 9 } else { -2 })
                    .collect()
            })
            .collect();
        let rejected = romano_wolf(&set, 150, 5, DEFAULT_BLOCK, 50_000);
        assert!(
            !rejected.is_empty(),
            "means from 0.75 to 121.75 against an SE near 0.7 must reject"
        );
        let mut seen = vec![false; set.len()];
        let mut expected_round = 0_usize;
        for r in &rejected {
            let slot = seen.get_mut(r.strategy).expect("a strategy of the input");
            assert!(!*slot, "strategy {} rejected twice", r.strategy);
            *slot = true;
            assert!(
                r.round == expected_round || r.round == expected_round + 1,
                "round {} after round {expected_round}: a gap or a step back",
                r.round
            );
            expected_round = r.round;
        }
        let rounds = rejected.last().map_or(0, |r| r.round);
        assert!(
            rounds < rejected.len() && rejected.len() <= set.len(),
            "{} rounds for {} rejections of {} strategies",
            rounds + 1,
            rejected.len(),
            set.len()
        );
    }

    /// o1runner-7 / D-1197: the stepdown computes each strategy's resampled
    /// mean once per draw, not once per draw per ROUND, counted at `mean_at`
    /// itself, and answers the same set on every rerun -- the set its receipt
    /// carries. On the merged tree that property is held by D-0973's one-walk
    /// stepdown; the null table D-1197 introduced was folded into it.
    #[test]
    fn the_stepdown_computes_each_null_statistic_once_and_answers_unchanged() {
        // Ten strong strategies clear round zero; a moderate one, masked while
        // they are in the maximum, clears a later round; one null never does.
        let noise = |t: i64| ((t * 7_919) % 23) - 11;
        let mut set: Vec<Vec<i64>> = (0..10)
            .map(|s| (0..64).map(|t| 40 + s + noise(t + s)).collect())
            .collect();
        set.push((0..64).map(|t| 55 + 60 * noise(t * 3)).collect());
        set.push((0..64).map(|t| noise(t * 5)).collect());
        let draws = 150;
        let before = super::MEAN_AT_CALLS.with(std::cell::Cell::get);
        let rejected = romano_wolf(&set, draws, 5, DEFAULT_BLOCK, 50_000);
        let calls = super::MEAN_AT_CALLS.with(std::cell::Cell::get) - before;
        let rounds = rejected.last().map_or(0, |r| r.round) + 1;
        assert!(
            rounds >= 2,
            "the fixture must take several rounds: {rejected:?}"
        );
        assert_eq!(
            calls,
            (set.len() * draws) as u64,
            "one mean per strategy per draw"
        );

        // The per-round recomputation this test first compared against is
        // gone: D-0973 replaced the rounded `1 - alpha` quantile with the exact
        // `(1 + count) / (B + 1)` rule, so the old loop is no longer the
        // reference. `exact_stepdown_tests` holds the new oracle (the adjusted
        // receipt); here the answer is idempotent and is the receipt's set.
        for seed in [1_u64, 5, 97, 2_026] {
            let once = romano_wolf(&set, draws, seed, DEFAULT_BLOCK, 50_000);
            assert_eq!(
                once,
                romano_wolf(&set, draws, seed, DEFAULT_BLOCK, 50_000),
                "seed {seed}"
            );
            let receipt = super::romano_wolf_receipt(&set, draws, seed, DEFAULT_BLOCK, 50_000)
                .expect("a valid family has a receipt");
            assert_eq!(receipt.rejected(), once.as_slice(), "seed {seed}");
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod block_ceiling_tests {
    use super::{
        FamilyTestsRefusalV1, Rng, family_tests_v1, reality_check, romano_wolf,
        romano_wolf_adjusted_p_values_v1, romano_wolf_receipt, spa, spa_receipt_v1,
        stationary_indices, white_reality_check_receipt_v1,
    };

    /// Deterministic noise around zero, plus `edge` paisa per period.
    fn edged(periods: usize, seed: u64, edge: i64) -> Vec<i64> {
        let mut r = Rng::new(seed);
        (0..periods)
            .map(|_| i64::try_from(r.next_u64() % 200).unwrap_or(0) - 100 + edge)
            .collect()
    }

    /// W3-runner1-2 (D-0742): the continuation draw is in parts per million,
    /// so above one million periods the restart probability floors to zero
    /// and every draw is one rotation of the series. Every entry point
    /// refuses that block instead of resampling rotations; one million itself
    /// is under the ceiling, and a block is also refused past the series length (D-1990).
    #[test]
    fn a_block_the_ppm_draw_cannot_restart_is_refused_by_every_entry_point() {
        let ceiling = 1_000_000_usize;
        assert_eq!(
            super::MAX_BLOCK,
            ceiling,
            "the ppm draw resolves one million"
        );
        let beyond = ceiling + 1;
        let periods = 200_usize;

        // Why it is refused: at `beyond` no draw ever restarts.
        let mut rng = Rng::new(5);
        for _ in 0..50 {
            let index = stationary_indices(periods, beyond, &mut rng);
            assert!(
                index
                    .windows(2)
                    .all(|pair| pair.get(1) == pair.first().map(|at| (at + 1) % periods).as_ref()),
                "above the ppm resolution every draw is one rotation"
            );
        }

        // A small positive mean on noise, beside plain noise.
        let set = vec![edged(periods, 11, 3), edged(periods, 12, 0)];
        let rc = reality_check(&set, 999, 3, beyond);
        assert!(
            rc.is_none(),
            "Reality Check answered over rotations: {:?}",
            rc.map(|v| v.p_value)
        );
        let v = spa(&set, 999, 3, beyond);
        assert!(
            v.is_none(),
            "SPA answered over rotations: {:?}",
            v.map(|v| (v.p_value, v.clears()))
        );
        assert!(white_reality_check_receipt_v1(&set, 999, 3, beyond).is_none());
        assert!(spa_receipt_v1(&set, 999, 3, beyond).is_none());
        assert!(romano_wolf_receipt(&set, 999, 3, beyond, 50_000).is_none());
        assert!(romano_wolf_adjusted_p_values_v1(&set, 999, 3, beyond).is_none());
        assert_eq!(
            family_tests_v1(&set, &[0], 999, 3, beyond).err(),
            Some(FamilyTestsRefusalV1::RomanoWolf)
        );
        assert_eq!(
            family_tests_v1(&set, &[], 999, 3, beyond).err(),
            Some(FamilyTestsRefusalV1::White)
        );

        // A strong edge the stepdown names at the series length is not named
        // beyond it.
        let strong = vec![edged(periods, 21, 60), edged(periods, 22, 0)];
        assert!(!romano_wolf(&strong, 999, 3, periods, 50_000).is_empty());
        assert!(romano_wolf(&strong, 999, 3, beyond, 50_000).is_empty());

        // A block as long as the series is accepted.
        assert!(reality_check(&set, 999, 3, periods).is_some());
        assert!(spa(&set, 999, 3, periods).is_some());
        assert!(white_reality_check_receipt_v1(&set, 999, 3, periods).is_some());
        assert!(spa_receipt_v1(&set, 999, 3, periods).is_some());
        assert!(romano_wolf_receipt(&set, 999, 3, periods, 50_000).is_some());
        assert!(romano_wolf_adjusted_p_values_v1(&set, 999, 3, periods).is_some());
        assert!(family_tests_v1(&set, &[0], 999, 3, periods).is_ok());
    }

    /// D-0742's open half, D-1990: a block longer than the series is refused
    /// by every entry point, even under the ppm ceiling. Measured before the
    /// fix: over 400 periods of pure noise, a block of 40,000 put 36 of 40
    /// families under p = 0.05.
    #[test]
    fn a_block_longer_than_the_series_is_refused_by_every_entry_point() {
        let periods = 200_usize;
        let longer = periods + 1;
        assert!(longer <= super::MAX_BLOCK, "under the ppm ceiling");
        let set = vec![edged(periods, 11, 3), edged(periods, 12, 0)];
        assert!(reality_check(&set, 999, 3, longer).is_none());
        assert!(spa(&set, 999, 3, longer).is_none());
        assert!(white_reality_check_receipt_v1(&set, 999, 3, longer).is_none());
        assert!(spa_receipt_v1(&set, 999, 3, longer).is_none());
        assert!(romano_wolf_receipt(&set, 999, 3, longer, 50_000).is_none());
        assert!(romano_wolf_adjusted_p_values_v1(&set, 999, 3, longer).is_none());
        let strong = vec![edged(periods, 21, 60), edged(periods, 22, 0)];
        assert!(romano_wolf(&strong, 999, 3, longer, 50_000).is_empty());
        assert_eq!(
            family_tests_v1(&set, &[0], 999, 3, longer).err(),
            Some(FamilyTestsRefusalV1::RomanoWolf)
        );
        assert_eq!(
            family_tests_v1(&set, &[], 999, 3, longer).err(),
            Some(FamilyTestsRefusalV1::White)
        );
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod exact_stepdown_tests {
    use super::{
        DEFAULT_BLOCK, NULL_STATISTICS, Rng, romano_wolf, romano_wolf_adjusted_p_values_v1,
        romano_wolf_receipt,
    };

    fn noise(periods: usize, seed: u64, shift: i64) -> Vec<i64> {
        let mut rng = Rng::new(seed);
        (0..periods)
            .map(|_| i64::try_from(rng.next_u64() % 21).unwrap_or(0) - 10 + shift)
            .collect()
    }

    fn family(seed: u64, shift: i64) -> Vec<Vec<i64>> {
        vec![
            noise(60, seed, shift),
            noise(60, seed.wrapping_add(10_000), 0),
            noise(60, seed.wrapping_add(20_000), shift / 2),
        ]
    }

    /// ONE PROCEDURE AND ONE BOUNDARY (D-0973).
    ///
    /// On a family with no zero-variance row, the legacy stepdown rejects a
    /// strategy exactly when its Romano--Wolf adjusted probability rejects at
    /// the same alpha under the exact `(1 + count) / (B + 1)` rule, and the
    /// receipt carries the same set. Before D-0973 it read a rounded `1 - alpha`
    /// quantile: at alpha 0 it rejected the strongest strategy, and near 5% it
    /// disagreed with the exact rule in both directions.
    #[test]
    fn the_stepdown_rejects_exactly_what_the_adjusted_receipt_rejects() {
        let mut compared = 0_usize;
        let mut disagreements: Vec<(u64, i64, u64, usize)> = Vec::new();
        for seed in 0..40_u64 {
            for shift in 1..=5_i64 {
                let returns = family(seed, shift);
                let adjusted = romano_wolf_adjusted_p_values_v1(&returns, 400, seed, DEFAULT_BLOCK)
                    .expect("a varying family has adjusted probabilities");
                for alpha in [0_u64, 1, 50_000, 100_000, 1_000_000] {
                    let legacy = romano_wolf(&returns, 400, seed, DEFAULT_BLOCK, alpha);
                    let receipt = romano_wolf_receipt(&returns, 400, seed, DEFAULT_BLOCK, alpha)
                        .expect("a valid family has a receipt");
                    assert_eq!(receipt.rejected(), legacy.as_slice());
                    for strategy in 0..returns.len() {
                        let exact = adjusted
                            .candidate(strategy)
                            .and_then(|row| row.adjusted_p_value().rejects_at_ppm(alpha))
                            .expect("every candidate has a probability");
                        if legacy.iter().any(|row| row.strategy == strategy) != exact
                            || receipt.is_rejected(strategy) != Some(exact)
                        {
                            disagreements.push((seed, shift, alpha, strategy));
                        }
                        compared = compared.saturating_add(1);
                    }
                }
            }
        }
        assert_eq!(compared, 40 * 5 * 5 * 3);
        assert!(
            disagreements.is_empty(),
            "{} disagreements (seed, shift, alpha, strategy): {disagreements:?}",
            disagreements.len()
        );
    }

    /// At alpha zero nothing is rejected: `(1 + count) / (B + 1)` is never 0.
    #[test]
    fn alpha_zero_rejects_nothing() {
        let returns = family(3, 8);
        assert!(
            !romano_wolf(&returns, 400, 3, DEFAULT_BLOCK, 1_000_000).is_empty(),
            "the control: the same family does reject at alpha 1"
        );
        assert!(romano_wolf(&returns, 400, 3, DEFAULT_BLOCK, 0).is_empty());
        let receipt =
            romano_wolf_receipt(&returns, 400, 3, DEFAULT_BLOCK, 0).expect("a valid receipt");
        assert!(receipt.rejected().is_empty());
    }

    /// AN ALPHA OUTSIDE THE PROBABILITY DOMAIN RUNS NO PROCEDURE (D-0973).
    ///
    /// `romano_wolf_receipt` already refused it; the legacy vector saturated
    /// `1_000_000 - alpha` to the minimum bootstrap maximum and rejected almost
    /// every strategy. It now renders it as it renders every malformed input:
    /// empty.
    #[test]
    fn an_alpha_above_one_rejects_nothing() {
        let returns = family(3, 8);
        let at_one = romano_wolf(&returns, 400, 3, DEFAULT_BLOCK, 1_000_000);
        assert_eq!(at_one.len(), returns.len(), "alpha 1 rejects every row");
        for alpha in [1_000_001_u64, 2_000_000, u64::MAX] {
            assert!(
                romano_wolf(&returns, 400, 3, DEFAULT_BLOCK, alpha).is_empty(),
                "alpha {alpha} is not a probability"
            );
            assert!(romano_wolf_receipt(&returns, 400, 3, DEFAULT_BLOCK, alpha).is_none());
        }
    }

    /// EACH NULL STATISTIC IS COMPUTED ONCE, HOWEVER MANY ROUNDS (D-0973).
    ///
    /// The former loop recomputed every survivor's resampled mean on every
    /// round, B·Σ|alive| statistics over the stepdown. The walk computes each
    /// strategy's B statistics once, whatever the round count.
    #[test]
    fn each_null_statistic_is_computed_once_however_many_rounds() {
        let returns: Vec<Vec<i64>> = (0..8_u64)
            .map(|strategy| {
                let edge = 8_i64.saturating_sub(i64::try_from(strategy).unwrap_or(8));
                noise(120, strategy.wrapping_add(500), edge)
            })
            .collect();
        NULL_STATISTICS.with(|count| count.set(0));
        let rejected = romano_wolf(&returns, 300, 11, DEFAULT_BLOCK, 50_000);
        let computed = NULL_STATISTICS.with(std::cell::Cell::get);
        let rounds = rejected.last().map_or(0, |row| row.round.saturating_add(1));
        assert!(rounds >= 2, "the family must step down; rounds {rounds}");
        assert_eq!(computed, 300 * returns.len(), "rounds {rounds}");
    }

    /// A ZERO-VARIANCE STRATEGY IS NEVER REJECTED (D-0973).
    ///
    /// Its statistic and every null draw of it are `0.0`, so no draw strictly
    /// exceeds it and the exact rule alone would call it the strongest result
    /// the draws can express.
    #[test]
    fn a_strategy_that_never_varied_is_never_rejected() {
        for alpha in [50_000_u64, 1_000_000] {
            assert!(
                romano_wolf(&[vec![0; 60], vec![0; 60]], 400, 5, DEFAULT_BLOCK, alpha).is_empty()
            );
            assert!(romano_wolf(&[vec![9; 60]], 400, 5, DEFAULT_BLOCK, alpha).is_empty());
        }
        let beside = vec![noise(60, 7, 8), vec![3; 60], noise(60, 8, 0)];
        let rejected = romano_wolf(&beside, 400, 5, DEFAULT_BLOCK, 50_000);
        assert!(
            rejected.iter().any(|row| row.strategy == 0),
            "the control: the edged row beside it is still rejected"
        );
        assert!(rejected.iter().all(|row| row.strategy != 1));
    }
}
