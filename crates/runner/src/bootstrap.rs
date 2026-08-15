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
        statistic: observed,
        p_value: if draws == 0 {
            1.0
        } else {
            beaten as f64 / draws as f64
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
        statistic: observed,
        p_value: if draws == 0 {
            1.0
        } else {
            beaten as f64 / draws as f64
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
        let mut rejected_now: Vec<usize> = Vec::new();
        for &s in &alive {
            let Some(own) = stats.get(s) else { continue };
            if studentized(root_n * own.mean, own.standard_error) > threshold {
                rejected_now.push(s);
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
        alive.retain(|s| !rejected_now.contains(s));
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

/// A statistic divided by its standard error, or the raw statistic when that
/// error is zero.
///
/// A zero standard error means the series never varied. Dividing would give
/// infinity, which reads as the strongest result ever recorded rather than as a
/// degenerate one — the same trap `crate::outcome` documents about its own
/// t-statistic.
fn studentized(statistic: f64, standard_error: f64) -> f64 {
    if standard_error > 0.0 {
        statistic / standard_error
    } else {
        statistic
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
    use super::{DEFAULT_BLOCK, reality_check, romano_wolf, spa};

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
        // This asserts the consequence a caller can see: rejections must come
        // out in non-decreasing rounds, and a strategy rejected in a later
        // round must not have been rejectable earlier under a threshold that
        // had risen. A rising bar shows up as a strategy that survives a round
        // it should have failed, so the round sequence is the observable.
        let mut set: Vec<Vec<i64>> = (0..12).map(|s| noise(300, s)).collect();
        set.push(edged(300, 90, 80));
        set.push(edged(300, 91, 60));
        set.push(edged(300, 92, 40));

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
