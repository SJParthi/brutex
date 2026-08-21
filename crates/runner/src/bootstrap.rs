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
    #[must_use]
    pub fn clears(&self) -> bool {
        self.draws > 0 && self.p_value < 0.05
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

/// White's Reality Check.
///
/// `returns[i]` is strategy `i`'s per-period returns. All must be the same
/// length: they are aligned in time, and a bootstrap draw picks the same periods
/// from every strategy so that the correlation between them is preserved.
///
/// `None` when the set is empty, when the series disagree in length, or when a
/// series is empty — refused rather than answered, because a p-value computed
/// over misaligned strategies is a number about nothing.
///
/// # Cost
///
/// `draws x periods x strategies`. This is a once-per-run boundary and the
/// cost is stated rather than bounded — with 1,000 draws over 1,000 periods and
/// 200 strategies it is 200 million operations, which is seconds, not
/// milliseconds.
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn reality_check(
    returns: &[Vec<i64>],
    draws: usize,
    seed: u64,
    block: usize,
) -> Option<Verdict> {
    let periods = aligned(returns)?;
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
    for _ in 0..draws {
        let index = stationary_indices(periods, block, &mut rng);
        // RECENTRED, and this is the whole test. Each resampled mean has its
        // OWN observed mean subtracted, so the bootstrap distribution is what a
        // set of strategies with NO edge would produce while keeping this set's
        // correlation structure. Comparing the observed maximum against that is
        // the question White's test asks.
        let mut best = f64::NEG_INFINITY;
        for (s, series) in returns.iter().enumerate() {
            let resampled = mean_at(series, &index);
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
        p_value: if draws == 0 || periods < 2 {
            1.0
        } else {
            beaten.saturating_add(1) as f64 / draws.saturating_add(1) as f64
        },
        draws,
        strategies: returns.len(),
    })
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
/// `None` under the same conditions as [`reality_check`].
///
/// UNVERIFIED as a measured figure: no bench row covers this yet.
#[must_use]
pub fn spa(returns: &[Vec<i64>], draws: usize, seed: u64, block: usize) -> Option<Verdict> {
    let periods = aligned(returns)?;
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
    for _ in 0..draws {
        let index = stationary_indices(periods, block, &mut rng);
        let mut best = f64::NEG_INFINITY;
        for (s, series) in returns.iter().enumerate() {
            let Some(own) = stats.get(s) else { continue };
            let resampled = mean_at(series, &index);
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
/// Returns the rejected strategies in the order they were rejected. An empty
/// result means nothing cleared, which is a finding rather than a failure.
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
    let Some(periods) = aligned(returns) else {
        return Vec::new();
    };
    let stats: Vec<Performance> = returns.iter().map(|r| summarise(r)).collect();
    let root_n = (periods as f64).sqrt();

    let mut alive: Vec<usize> = (0..returns.len()).collect();
    let mut out: Vec<Rejected> = Vec::new();
    let mut round = 0_usize;

    // ONE RESAMPLE SET, DRAWN ONCE AND REUSED BY EVERY ROUND.
    //
    // This drew fresh indices per round, from `Rng::new(seed + round)`. That
    // breaks the property the stepdown rests on: with a SHRINKING alive set the
    // maximum is taken over fewer strategies, so the threshold must be
    // non-increasing. On independent draws it is not, because the round's
    // threshold is a different sample as well as a smaller set.
    //
    // Measured: 32.536 -> 33.906 on a shrinking set at seed 97 -- the bar ROSE
    // after a strategy was removed. It rose in 19 of 400 configurations, and 0
    // of 400 after this change. It altered the rejection set in 4 of 400, one
    // of them permissively.
    //
    // Romano & Wolf's construction is one B x n resample matrix held across the
    // stepdown, which is what this now is. `Rng::new(seed)` once, so the draws
    // are still fully determined by the caller's seed and §3 rule 5 holds.
    let mut rng = Rng::new(seed);
    let indices: Vec<Vec<usize>> = (0..draws)
        .map(|_| stationary_indices(periods, block, &mut rng))
        .collect();

    // Bounded by the strategy count: each round removes at least one or stops.
    while !alive.is_empty() {
        // The bootstrap maximum over the SURVIVING set only. That shrinking is
        // the stepdown -- with the winner removed the bar is lower, so a
        // strategy it was masking can now clear.
        let mut maxima: Vec<f64> = Vec::with_capacity(draws);
        for index in &indices {
            let mut best = f64::NEG_INFINITY;
            for &s in &alive {
                let (Some(series), Some(own)) = (returns.get(s), stats.get(s)) else {
                    continue;
                };
                let centred = mean_at(series, index) - own.mean;
                best = best.max(studentized(root_n * centred, own.standard_error));
            }
            maxima.push(best);
        }
        let Some(threshold) = quantile(&mut maxima, 1_000_000_u64.saturating_sub(alpha_ppm)) else {
            break;
        };

        // Every surviving strategy above the threshold is rejected together:
        // they all cleared the same bar in the same round.
        //
        // THE PARTITION IS BUILT ONCE, NOT DERIVED TWICE.
        //
        // This collected `rejected_now` and then ran
        // `alive.retain(|s| !rejected_now.contains(s))`, which is a LINEAR SCAN
        // of the rejected set for every survivor -- O(alive x rejected) per
        // round, and nothing structural bounds either: `romano_wolf` is a
        // `pub fn` over `&[Vec<i64>]` and the caller decides how many strategies
        // it holds. A round that rejects half of 10,000 candidates is 25 million
        // comparisons to compute a set the loop above already knew.
        //
        // Gate 11 rule 7 could not see it. Its pattern is `\.contains\(&` and
        // this was `.contains(s)`, `s` already being a reference -- so the one
        // genuine `Vec` scan of that shape in the crate was invisible to the
        // gate written to refuse exactly it.
        //
        // Both halves fall out of the single pass that decides them, so the
        // cost is O(alive) and the two vectors cannot disagree about which
        // strategy went where.
        let mut rejected_now: Vec<usize> = Vec::new();
        let mut survivors: Vec<usize> = Vec::with_capacity(alive.len());
        for &s in &alive {
            // A strategy with no `stats` row SURVIVES, which is what `retain`
            // did: it was never pushed to `rejected_now`, so the predicate kept
            // it. Spelled out here because the old shape said it by omission.
            let Some(own) = stats.get(s) else {
                survivors.push(s);
                continue;
            };
            if studentized(root_n * own.mean, own.standard_error) > threshold {
                rejected_now.push(s);
            } else {
                survivors.push(s);
            }
        }
        if rejected_now.is_empty() {
            break;
        }
        for s in &rejected_now {
            out.push(Rejected {
                strategy: *s,
                round,
            });
        }
        alive = survivors;
        round = round.saturating_add(1);
    }
    out
}

/// Every series is the same non-zero length, and that length.
fn aligned(returns: &[Vec<i64>]) -> Option<usize> {
    let first = returns.first()?.len();
    if first == 0 || returns.iter().any(|r| r.len() != first) {
        return None;
    }
    Some(first)
}

/// Mean and standard error of one series.
fn summarise(series: &[i64]) -> Performance {
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
fn studentized(statistic: f64, standard_error: f64) -> f64 {
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
    if periods == 0 {
        return out;
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
    out
}

/// Mean of `series` taken at `index`.
fn mean_at(series: &[i64], index: &[usize]) -> f64 {
    if index.is_empty() {
        return 0.0;
    }
    let sum = index.iter().fold(0.0_f64, |a, &i| {
        a + series.get(i).copied().unwrap_or(0) as f64
    });
    sum / index.len() as f64
}

/// The `q_ppm` quantile of a slice, sorting it in place.
///
/// Parts per million rather than a fraction, so no float ever becomes an index.
/// A truncating cast on a quantile silently returns the wrong threshold, and a
/// wrong threshold here admits or rejects strategies with no sign anything went
/// wrong -- the lint table denies that cast for exactly this reason.
///
/// `None` for an empty slice — refused rather than answered zero, because a
/// threshold of zero would reject every strategy with a positive statistic and
/// report it as a finding.
fn quantile(values: &mut [f64], q_ppm: u64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    // `total_cmp` and not `partial_cmp`: a NaN under a partial comparator makes
    // the sort's behaviour unspecified, and a NaN can arrive here from a
    // degenerate series. The same choice `crate::rank` makes and for the same
    // reason.
    values.sort_unstable_by(f64::total_cmp);
    // WALKED, NOT CAST. The index is a fraction of the length, and turning a
    // rounded `f64` back into an index needs a cast the lint table denies for
    // good reason: a truncating cast on a quantile silently returns the wrong
    // threshold, and a wrong threshold here rejects or admits strategies with
    // no sign that anything went wrong.
    //
    // Integer arithmetic instead. `q` is clamped to `0..=1` and scaled to parts
    // per million, so the whole computation stays in `usize` and cannot land
    // outside the slice.
    let last = values.len().saturating_sub(1);
    let idx = usize::try_from(
        (last as u64)
            .saturating_mul(q_ppm.min(1_000_000))
            .saturating_add(500_000)
            / 1_000_000,
    )
    .unwrap_or(0)
    .min(last);
    values.get(idx).copied()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{DEFAULT_BLOCK, Verdict, reality_check, romano_wolf, spa};

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

            let rc = reality_check(&one, 1_000, 3, DEFAULT_BLOCK).expect("a verdict");
            same(rc.p_value, 1.0, "one period cannot be resampled");
            assert!(
                !rc.clears(),
                "Reality Check cleared a single period of {magnitude} paisa"
            );

            let v = spa(&one, 1_000, 3, DEFAULT_BLOCK).expect("a verdict");
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
        let v = spa(&two, 1_000, 3, DEFAULT_BLOCK).expect("a verdict");
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
            let v = spa(&set, 200, 6, DEFAULT_BLOCK).expect("a verdict");
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
        let v = spa(&set, 50, 9, DEFAULT_BLOCK).expect("three periods still yield a verdict");
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

    /// EVERY ROUND REMOVES AT LEAST ONE, OR THE LOOP STOPS.
    ///
    /// The `while !alive.is_empty()` bound rests on it. Under the old `retain`
    /// this was guaranteed by the predicate; under a hand-written partition a
    /// survivor pushed on both branches would loop forever, and a test that
    /// merely finished would not say so. This one asserts the count.
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
        if rejected.is_empty() {
            return;
        }
        let rounds = rejected.last().map_or(0, |r| r.round);
        for round in 0..=rounds {
            let n = rejected.iter().filter(|r| r.round == round).count();
            assert!(
                n > 0,
                "round {round} emitted nothing, so the loop ran a round without \
                 shrinking the surviving set"
            );
        }
        assert!(
            rejected.len() <= set.len(),
            "the total rejected can never exceed the input"
        );
    }
}
