//! How good a result must be before it is better than the best of luck.
//!
//! # Why a sweep needs this and a hand-built strategy does not
//!
//! Test twenty combinations and a t-statistic of 3 is evidence. Test sixty-one
//! million and the *best of pure noise* scores about 6. The bar is not a
//! property of the strategy; it is a property of **how many were tried**, and
//! this crate is built to try an enormous number.
//!
//! Harvey, Liu & Zhu put the floor at **t > 3.0** for a newly discovered factor
//! against the conventional 2.0, and conclude that "most claimed research
//! findings in financial economics are likely false" — calibrated to roughly 316
//! published factors and an estimated 820 total trials. A sweep is five to eight
//! orders of magnitude beyond that, so 3.0 is the floor of the literature and
//! not a number this engine may borrow.
//!
//! # Why THIS engine can compute it and most backtesters cannot
//!
//! The correction needs the number of hypotheses actually tested. Most tools
//! have no idea; this one counts to the integer — `generated`, `duplicates`,
//! `excluded`, `pruned`, `infrequent` and `frequent`, per level, reconciling to
//! the candidate. So the bar is derived from the run's own counters and asks
//! nobody for a number, which is the same standard depth and memory are held to.
//!
//! # What this module does NOT do
//!
//! It does not rank, score, or judge a combination. `Itemset` carries a mask and
//! a hit count and nothing else — the engine has no notion of profit, and
//! `CLAUDE.md` §3 rule 1 forbids inventing one. This module answers only "given
//! that you tested N hypotheses, how large must a t-statistic be before it means
//! anything", which is arithmetic on N.
//!
//! # Sources
//!
//! * Harvey, Liu & Zhu, ". . . and the Cross-Section of Expected Returns",
//!   NBER WP 20592 (2014), *Review of Financial Studies* 29(1), 2016.
//! * Bailey & López de Prado, "The Deflated Sharpe Ratio", *Journal of Portfolio
//!   Management* 40(5), 2014 — the expected-maximum formula this module's
//!   [`expected_max_t`] is the independent-trials case of.

// The one lint exception, and the same one `crates/greeks` takes for the same
// reason. `CLAUDE.md` §7 makes prices integers and says in the same breath that
// "statistical values (Sharpe, p-values, ratios) keep full precision and are
// never rounded" — a t-statistic is exactly that. No price, paisa figure or
// tick ever enters this module; it consumes a COUNT and returns a THRESHOLD.
#![allow(
    clippy::float_arithmetic,
    reason = "CLAUDE.md §7 keeps statistical values at full precision; D-0061's \
              integer rule is about prices, and no price reaches this module."
)]

use engine::Sweep;

/// Family-wise error rate the Bonferroni threshold is computed at.
///
/// 5% is the convention Harvey, Liu & Zhu work in when they report a Bonferroni
/// cutoff rising from 1.96 to 3.78 across 316 factors — reproducing that figure
/// from this constant is what shows the arithmetic here is theirs and not
/// something invented. It is a stated convention rather than a tuned parameter:
/// no run changes it, and no caller is asked for it.
const FWER: f64 = 0.05;

/// How many hypotheses a sweep actually tested against the bars.
///
/// **Not** `generated`. A candidate that was rejected as a duplicate, excluded
/// before k=1, or subset-pruned was never measured against a single bar, so it
/// consumed no chance to look good by accident. The hypotheses that did are
/// exactly those whose support was counted: the ones that came back frequent,
/// plus the ones that came back too rare.
///
/// Counting `generated` instead would inflate the bar and reject real findings;
/// counting only `frequent` would deflate it and accept noise. Both are wrong in
/// the direction that matters, so the sum is taken from the two counters that
/// mean "a support count happened here".
#[must_use]
pub fn trials(sweep: &Sweep) -> u64 {
    sweep.levels.iter().fold(0_u64, |n, level| {
        let kept = u64::try_from(level.frequent.len()).unwrap_or(u64::MAX);
        n.saturating_add(level.infrequent).saturating_add(kept)
    })
}

/// [`trials`], charged for the exit grid a finding was also selected through.
///
/// # The axis the bar was not charging for
///
/// [`trials`] counts one thing: how many condition COMBINATIONS had their
/// support measured. That is the whole search only for a report that ranks a
/// combination on its own forward return, which is what [`crate::rank`] does.
///
/// It is **not** the whole search for anything chosen through
/// [`crate::grid`]. There, each surviving combination is evaluated at up to
/// [`crate::grid::variants`] stop/target/trail/arm settings and the best of them
/// is kept — `Grid::sharpest` and `Grid::best` are argmaxes over as many as 325
/// cells at the shipped four rungs. Selecting a maximum over 325 variants is 325
/// more chances to look good by luck, per combination, and none of it entered
/// the bar. An audit
/// measured the omission and named the consequence exactly: the reported
/// Bonferroni and Bailey figures understate the true search size by roughly the
/// grid size, **while the report prints them as the bar every row must clear**.
///
/// A bar that is too low is the dangerous direction. It admits noise while
/// carrying the authority of a family-wise correction, which is worse than
/// printing no bar at all — a reader who sees no bar knows to be careful.
///
/// # Why a separate function rather than folding it into `trials`
///
/// Because the two searches are genuinely different, and charging a report for
/// a grid it never ran would be its own error in the opposite direction.
/// `cli sweep-stored` ranks on forward returns and never builds a grid: for it,
/// `trials` is exact and this function would inflate the bar and reject real
/// findings. `cli audit` does build one. The caller knows which it did; this
/// module does not, and guessing would make one of the two reports wrong no
/// matter which default it picked.
///
/// # This is an UPPER bound, and saying which direction matters
///
/// The product assumes the `variants` settings are independent trials. **They
/// are not.** A grid's cells share one trade walk — `crate::grid` caches the
/// path crossings once per candidate and each variant's exit is then three
/// integer compares — so neighbouring cells differ by one rung and their
/// outcomes are heavily correlated. The effective number of independent trials
/// is somewhere between 1 and `variants`, and **nobody has measured where**.
///
/// So this returns the ceiling: a bar computed from it is harder to clear than
/// the true one. A finding that clears it is safe; a finding that does not is
/// not thereby worthless. That is the same direction, and the same disclaimer,
/// [`expected_max_t`] already carries for its own independence assumption.
///
/// The alternative was to invent a correlation discount, which `CLAUDE.md` §3
/// rule 1 forbids, or to keep charging nothing at all — which was the defect.
/// An upper bound stated as one is the honest third option, per §3 rule 6.
///
/// # Multiplicative, and the saturation is deliberate
///
/// The two selections compose: each of `trials` combinations was examined at
/// `variants` settings, so the family is at most the product. `variants == 0` is
/// treated as 1 — "no grid was run" — rather than collapsing the whole family
/// to zero, which would return a bar of zero and pass everything.
///
/// Saturating rather than wrapping: a product past `u64::MAX` cannot arise from
/// a walk the candidate ceiling bounds, and a wrapped trial count would print a
/// small, plausible, catastrophically wrong bar.
///
/// # It takes a COUNT and not a `&Sweep`, and that signature is the fix
///
/// It was `trials_with_grid(sweep: &Sweep, variants: u64)` and computed
/// `trials(sweep) * variants` internally. Its one caller,
/// `cli::grid_exposure`, printed it beside `effective_trials(sweep)` — the
/// DUPLICATE-DEFLATED count — under the sentence *"with N exit settings each, at
/// most {ceiling}"*, which claims the only difference between the two numbers is
/// the grid.
///
/// It was not. One is deflated and one is raw, so the ratio between the printed
/// pair was **929,577x** on a measured fixture where the sentence promised 325x,
/// and the upper Bonferroni bar was overstated by 1.27 t-units. Both figures
/// were individually correct; the line comparing them was not.
///
/// Taking the count as an argument makes the caller choose which axis it is
/// multiplying, and makes the mismatch unspellable: the number printed on the
/// left is the number multiplied on the right, because it is the same binding.
#[must_use]
pub fn trials_with_grid(combinations: u64, variants: u64) -> u64 {
    combinations.saturating_mul(variants.max(1))
}

/// The t-statistic the **best of `n` worthless strategies** is expected to show.
///
/// `√(2·ln n)` — the standard extreme-value result for the maximum of `n`
/// independent standard normals, and the independent-trials case of the
/// expected-maximum expression in the Deflated Sharpe Ratio paper.
///
/// # This is the OPTIMISTIC direction, and saying which way matters
///
/// It assumes the trials are independent. A sweep's are not: `{A,B}` and
/// `{A,B,C}` overlap heavily, which lowers the *effective* number of independent
/// trials and therefore lowers the true bar. So the figure returned here is an
/// **upper** bound on the noise level — a result that clears it is safe, and a
/// result that does not is not automatically worthless. The exact correction
/// needs the lattice's correlation structure and is not computed here rather
/// than being approximated into something that looks precise.
///
/// Returns 0 for fewer than two trials, where "the maximum of the noise" is not
/// a meaningful quantity.
#[must_use]
pub fn expected_max_t(n: u64) -> f64 {
    if n < 2 {
        return 0.0;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "a trial count above 2^53 cannot arise: the candidate ceiling \
                  and the allocator bound both stop the walk far below it, and \
                  the logarithm is insensitive at that scale regardless."
    )]
    let n_f = n as f64;
    (2.0 * n_f.ln()).sqrt()
}

/// Hypotheses that were **distinct tests**, not merely distinct masks.
///
/// # Two masks with identical support are one hypothesis, not two
///
/// [`trials`] counts every support evaluation. But if `{A,B}` and `{A,B,C}` fire
/// on exactly the same bars, they are not two chances to get lucky — they are
/// the *same* chance, counted twice. Every statistic derived from them is
/// identical, so a multiple-testing correction that treats them as independent
/// charges for a trial nobody ran.
///
/// This is stronger than the usual correlation argument and it is why it can be
/// computed exactly. The literature deflates for strategies that are *similar*,
/// which needs a correlation matrix and an eigenvalue count. A non-closed
/// itemset is not similar to its closed superset; it is **the same test**, and
/// `crate::closed` identifies those by construction.
///
/// # This is still an upper bound, and saying so is the point
///
/// It removes only exact duplicates. Two combinations that overlap on 99% of
/// their bars remain two trials here, and genuinely are two *near*-identical
/// ones — so the true effective count sits below this figure. What has been
/// removed is the part that can be removed **without estimating anything**.
///
/// The direction is safe: a bar computed from this is lower than one computed
/// from [`trials`] and higher than the unknowable truth, so a finding that
/// clears it has cleared a real hurdle.
#[must_use]
pub fn effective_trials(sweep: &Sweep) -> u64 {
    // `redundant_count` rather than `closed(..).redundant()`: the latter built a
    // whole Vec of the kept answer and dropped it, which an audit measured as
    // effectively the entire cost of a report render.
    trials(sweep).saturating_sub(crate::closed::redundant_count(sweep))
}

/// Euler–Mascheroni, which is the constant the deflated-Sharpe expression uses.
const EULER_MASCHERONI: f64 = 0.577_215_664_901_532_9;

/// The expected maximum, by **Bailey & López de Prado's** expression.
///
/// `(1−γ)·Z⁻¹(1 − 1/N)  +  γ·Z⁻¹(1 − 1/(N·e))`
///
/// This is the quantity [`expected_max_t`] approximates. Both answer "what does
/// the best of `n` worthless trials score", and they disagree by enough to
/// matter:
///
/// | trials | `√(2·ln n)` | this |
/// |---|---|---|
/// | 3,689 | 4.05 | 3.61 |
/// | 61,125,295 | 5.99 | 5.63 |
/// | 100,000,000,000 | 7.12 | 6.81 |
///
/// **The simpler form OVERSTATES the noise floor**, which is the safe direction
/// but not the accurate one — it would reject a real finding sitting between the
/// two figures. This is the expression the literature actually publishes, so it
/// is the one reported beside the Bonferroni bar; `expected_max_t` stays because
/// the two together show how much the approximation costs, and a reader who has
/// only seen `√(2 ln n)` quoted elsewhere can find it here.
///
/// Returns 0 for fewer than two trials, where a maximum is not a quantity.
#[must_use]
pub fn expected_max_bailey(n: u64) -> f64 {
    if n < 2 {
        return 0.0;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "see expected_max_t: the walk cannot reach 2^53 candidates."
    )]
    let n_f = n as f64;
    // Both quantiles taken from their TAIL probabilities, for the reason
    // `upper_tail_quantile` records: `1.0 - 1.0/n` rounds to 1.0 near
    // n = 6.6e15, and this function dropped 4.739 across one integer there
    // while `sqrt(2 ln n)` was still rising.
    let a = upper_tail_quantile(1.0 / n_f);
    let b = upper_tail_quantile(1.0 / (n_f * core::f64::consts::E));
    (1.0 - EULER_MASCHERONI) * a + EULER_MASCHERONI * b
}

/// The two-sided Bonferroni t-threshold for `n` tests at [`FWER`].
///
/// Each test is allowed `FWER / n` of the error budget, so the threshold is the
/// standard normal quantile at `1 − FWER/(2n)`.
///
/// # Reproducing the published figure is the check that this is right
///
/// Harvey, Liu & Zhu report a Bonferroni cutoff of **3.78** across 316 factors.
/// `bonferroni_t(316)` returns that, which is the evidence this function
/// implements their arithmetic rather than something that merely looks like it —
/// see `tests::the_published_bonferroni_figure_is_reproduced`.
///
/// Returns 0 for fewer than one trial.
#[must_use]
pub fn bonferroni_t(n: u64) -> f64 {
    if n < 1 {
        return 0.0;
    }
    #[allow(
        clippy::cast_precision_loss,
        reason = "see expected_max_t: the walk cannot reach 2^53 candidates."
    )]
    let n_f = n as f64;
    // THE TAIL PROBABILITY, HANDED OVER DIRECTLY. This read
    // `inverse_normal_cdf(1.0 - FWER / (2.0 * n_f))`, which rounds to
    // `inverse_normal_cdf(1.0)` -- and therefore to 0.0 -- once the tail falls
    // below half an ulp of 1.0. See `upper_tail_quantile` for the bisected
    // cliff and for why a bar of zero is the dangerous direction.
    upper_tail_quantile(FWER / (2.0 * n_f))
}

/// Two-sided p-value for a t-statistic, under the normal approximation.
///
/// `2·(1 − Φ(|t|))`. The normal rather than Student's t because the samples
/// here are hundreds to thousands of bars, where the two are indistinguishable
/// to the precision anything downstream uses — and because a Student's t with
/// per-combination degrees of freedom would need the sample size threaded
/// through every caller for a correction smaller than the approximation already
/// acknowledged in [`expected_max_t`].
#[must_use]
pub fn p_value(t: f64) -> f64 {
    (2.0 * (1.0 - normal_cdf(t.abs()))).clamp(0.0, 1.0)
}

/// How many of `p_values` may be called findings at a false-discovery rate of
/// [`FWER`], by **Benjamini–Hochberg**.
///
/// # Why this and not only Bonferroni
///
/// Bonferroni controls the chance of **even one** false positive. That is the
/// right question for a handful of hypotheses and the wrong one for a sweep: at
/// sixty-one million tests it sets a bar so high that a real effect must be
/// enormous to clear it, and most real effects are not enormous.
///
/// Benjamini–Hochberg controls the **fraction** of the findings that are
/// flukes. "Of these forty, at most two are noise" is both more useful and more
/// powerful than "this one thing is beyond doubt", and it is what a search
/// designed to look at everything actually wants. Harvey, Liu & Zhu report both
/// for the same data, and their BHY cutoff (3.39) sits well below their
/// Bonferroni one (3.78).
///
/// # The procedure, and the step that is easy to get wrong
///
/// Sort ascending, find the LARGEST `k` with `p(k) ≤ (k/m)·α`, and reject every
/// hypothesis up to `k` — **including those whose own p-value exceeds their own
/// threshold**. Stopping at the first failure instead is the common error and
/// it makes the procedure conservative in a way that is no longer Benjamini–
/// Hochberg.
///
/// Returns how many are rejected; the caller holds the ordering and can take
/// that many from the front of its own sorted list.
#[must_use]
pub fn benjamini_hochberg(p_values: &mut [f64]) -> usize {
    if p_values.is_empty() {
        return 0;
    }
    p_values.sort_unstable_by(f64::total_cmp);
    #[allow(
        clippy::cast_precision_loss,
        reason = "a finding count above 2^53 cannot be held in memory, let alone \
                  ranked; the bounded heap caps it far below."
    )]
    let m = p_values.len() as f64;
    let mut largest = 0_usize;
    for (index, p) in p_values.iter().enumerate() {
        let rank = index.saturating_add(1);
        #[allow(
            clippy::cast_precision_loss,
            reason = "see above -- the rank is bounded by the slice length."
        )]
        let k = rank as f64;
        if *p <= k / m * FWER {
            // NOT a break. The largest passing rank is what counts, and a later
            // one can pass after an earlier one fails -- stopping here is the
            // classic misreading and it silently makes the test conservative.
            largest = rank;
        }
    }
    largest
}

/// Standard normal CDF, from the error function's rational approximation.
///
/// Abramowitz & Stegun 7.1.26 composed into `Φ(x) = ½(1 + erf(x/√2))`. Accurate
/// to about 1.5e-7, which is far beyond what a p-value compared against a
/// threshold needs — and the same order as the quantile below it, so the pair
/// do not disagree at a precision either of them can support.
fn normal_cdf(x: f64) -> f64 {
    // Horner form with named coefficients, for the reason the quantile gives:
    // `clippy::indexing_slicing` is denied and a coefficient table would need an
    // exception this does not deserve.
    let z = x / core::f64::consts::SQRT_2;
    let sign = if z < 0.0 { -1.0 } else { 1.0 };
    let a = z.abs();
    let t = 1.0 / (1.0 + 0.327_591_1 * a);
    let poly = ((((1.061_405_429 * t - 1.453_152_027) * t + 1.421_413_741) * t - 0.284_496_736)
        * t
        + 0.254_829_592)
        * t;
    let erf = sign * (1.0 - poly * (-a * a).exp());
    0.5 * (1.0 + erf)
}

/// Standard normal quantile — Acklam's rational approximation.
///
/// Accurate to about 1.15e-9 across the open interval, which is far beyond what
/// a threshold printed to two decimals needs. Written as explicit Horner
/// evaluation with named coefficients rather than arrays, because
/// `clippy::indexing_slicing` is denied workspace-wide and a table lookup here
/// would need an exception this does not deserve.
///
/// Returns 0 outside `(0, 1)`, which no caller here can reach: `bonferroni_t`
/// passes `1 − 0.05/(2n)`, which lies in `[0.975, 1)` for every `n ≥ 1`.
fn inverse_normal_cdf(p: f64) -> f64 {
    // The boundary between the central rational form and the two tail forms.
    const P_LOW: f64 = 0.02425;
    if p <= 0.0 || p >= 1.0 {
        return 0.0;
    }
    // The central region, where the rational form in `q = p - 0.5` applies.
    if p > P_LOW && p < 1.0 - P_LOW {
        let q = p - 0.5;
        let r = q * q;
        let num = ((((-39.696_830_286_653_76 * r + 220.946_098_424_520_5) * r
            - 275.928_510_446_968_7)
            * r
            + 138.357_751_867_269)
            * r
            - 30.664_798_066_147_16)
            * r
            + 2.506_628_277_459_239;
        let den = ((((-54.476_098_798_224_06 * r + 161.585_836_858_040_9) * r
            - 155.698_979_859_886_6)
            * r
            + 66.801_311_887_719_72)
            * r
            - 13.280_681_552_885_72)
            * r
            + 1.0;
        return q * num / den;
    }
    // The tails. `bonferroni_t` always lands in the upper one.
    let upper = p > 1.0 - P_LOW;
    let tail = if upper { 1.0 - p } else { p };
    // The rational form is written for the LOWER tail and returns a negative
    // value there; the upper tail is its mirror. Getting this backwards made
    // `bonferroni_t(316)` return -3.78, which the published-figure test caught
    // on the first run -- the reason that test asserts the sign and not just the
    // magnitude.
    if upper {
        -tail_rational(tail)
    } else {
        tail_rational(tail)
    }
}

/// The shared tail rational form, in the tail probability itself.
///
/// Extracted from [`inverse_normal_cdf`] so that a caller holding a TAIL
/// probability can reach it without first forming `1 - alpha` and then having
/// this function undo that subtraction. See [`upper_tail_quantile`] for why
/// that round trip is not merely wasteful.
///
/// Returns the LOWER-tail quantile, which is negative. Mirror it for the upper.
fn tail_rational(tail: f64) -> f64 {
    let q = (-2.0 * tail.ln()).sqrt();
    let num = ((((-0.007_784_894_002_430_293 * q - 0.322_396_458_041_136_5) * q
        - 2.400_758_277_161_838)
        * q
        - 2.549_732_539_343_734)
        * q
        + 4.374_664_141_464_968)
        * q
        + 2.938_163_982_698_783;
    let den = (((0.007_784_695_709_041_462 * q + 0.322_467_129_070_039_8) * q
        + 2.445_134_137_142_996)
        * q
        + 3.754_408_661_907_416)
        * q
        + 1.0;
    num / den
}

/// The standard normal quantile at upper-tail probability `alpha`.
///
/// # THE BAR EVERY FINDING MUST CLEAR USED TO COLLAPSE TO EXACTLY ZERO
///
/// [`bonferroni_t`] read `inverse_normal_cdf(1.0 - FWER / (2n))`, and
/// [`expected_max_bailey`] read `inverse_normal_cdf(1.0 - 1.0 / n)`. Both form
/// `1 - alpha` in `f64`. Once `alpha` falls below half an ulp of 1.0 —
/// `2^-53`, about `1.11e-16` — **`1.0 - alpha` rounds to exactly `1.0`**, and
/// `inverse_normal_cdf` returns `0.0` for `p >= 1.0`.
///
/// So the threshold every finding is judged against became **zero**, and
/// everything passed. Bisected, the cliff is one integer wide:
///
/// ```text
/// bonferroni_t(450_359_962_737_049) = 8.209536
/// bonferroni_t(450_359_962_737_050) = 0.000000
/// ```
///
/// A bar that is too LOW is the dangerous direction — it admits noise while
/// carrying the authority of a family-wise correction, which is worse than
/// printing no bar at all. There was also a quantization plateau before the
/// cliff: the bar froze at 8.209536 across a doubling of the search size.
///
/// # Not reachable at shipped defaults, and that is not the reason to fix it
///
/// `engine::DEFAULT_PAIR_BUDGET` is `1 << 34`, so `trials` is bounded near
/// `1.72e10`; times the 325-cell grid that is `5.58e12`, about **80x below the
/// cliff**. But `Ladder::with_pair_budget` is `pub` and takes any `u64`, and
/// [`trials_with_grid`] is `pub` and takes a caller-supplied `u64`. Both return
/// a wrong answer in the dangerous direction for inputs inside their declared
/// domain, with no refusal — which is the `CLAUDE.md` §4 fallback that hides a
/// failure, not a theoretical concern about an unreachable input.
///
/// # The fix is to never form `1 - alpha`
///
/// `inverse_normal_cdf`'s own tail branch computes `sqrt(-2 * ln(1 - p))`, so a
/// caller holding `alpha` was passing `1 - alpha` in for that branch to
/// subtract back out. Taking `alpha` directly removes both the cancellation and
/// the round trip: the smallest `alpha` a `u64` trial count can produce is
/// `0.05 / (2 * u64::MAX)`, about `1.4e-21`, which `ln` handles with room to
/// spare. **There is no longer a cliff at any `u64`.**
///
/// Above `P_LOW` this delegates rather than duplicating: in that region `1 -
/// alpha` loses nothing and the central rational form is the accurate one.
///
/// Returns 0.0 for a non-positive or non-finite `alpha`, and for `alpha >= 1`,
/// where an upper-tail quantile is not a quantity.
fn upper_tail_quantile(alpha: f64) -> f64 {
    /// The same boundary [`inverse_normal_cdf`] uses between its central
    /// rational form and its tail form.
    const P_LOW: f64 = 0.02425;
    if !alpha.is_finite() || alpha <= 0.0 || alpha >= 1.0 {
        return 0.0;
    }
    if alpha >= P_LOW {
        // No cancellation is possible here -- `1 - alpha` is exact to within an
        // ulp for an alpha this large -- and the central form is the accurate
        // one in that region.
        return inverse_normal_cdf(1.0 - alpha);
    }
    -tail_rational(alpha)
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{
        benjamini_hochberg, bonferroni_t, effective_trials, expected_max_bailey, expected_max_t,
        inverse_normal_cdf, normal_cdf, p_value, trials, trials_with_grid,
    };
    use engine::{Frontier, Itemset, Sweep};
    use vocab::ConditionMask;

    fn level(k: u32, frequent: usize, infrequent: u64) -> Frontier {
        Frontier {
            k,
            frequent: (0..frequent)
                .map(|i| Itemset {
                    mask: ConditionMask::default().with_bit(u32::try_from(i % 300).unwrap_or(0)),
                    hits: 1,
                })
                .collect(),
            generated: 0,
            duplicates: 0,
            excluded: 0,
            pruned: 0,
            infrequent,
        }
    }

    fn sweep(levels: Vec<Frontier>) -> Sweep {
        Sweep {
            levels,
            excluded: Vec::new(),
            bars: 1_000,
            min_hits: 1,
            halted: None,
        }
    }

    /// The exit grid is a second search axis, and the bar has to be charged for it.
    ///
    /// # Why each assertion is here
    ///
    /// The first pins the MULTIPLICATION, which is the whole point: a report
    /// that ran a 325-way grid over every surviving combination searched 325
    /// times as much as `trials` alone reports, and a bar computed from the
    /// smaller number admits noise while looking like a family-wise correction.
    ///
    /// The second and third pin the two arguments that could collapse the
    /// family to nothing. `variants == 0` means "no grid ran", not "no
    /// hypotheses were tested" — collapsing to zero would return a bar of zero
    /// from `bonferroni_t` and pass every row, which is the exact failure the
    /// function exists to prevent, arrived at from the other side. `variants ==
    /// 1` is the same statement said the other way and is the identity.
    ///
    /// The fourth pins that the charged bar is genuinely HIGHER. Without it every
    /// assertion above would pass on a function that returned `trials` unchanged.
    #[test]
    fn the_bar_is_charged_for_the_exit_grid_it_was_selected_through() {
        // 4 kept + 6 infrequent at k=1, 2 kept + 8 infrequent at k=2: 20 support
        // counts, which is what `trials` means.
        let s = sweep(vec![level(1, 4, 6), level(2, 2, 8)]);
        assert_eq!(trials(&s), 20, "the combination axis alone");

        // NOT A LITERAL: the shipped grid width, read from the one function
        // that computes it. Written as `125` until the arming axis made it 325,
        // at which point this assertion went on passing against a number the
        // engine no longer runs -- which is the whole failure mode the constant
        // in `crates/cli/src/lib.rs` is asserted against at compile time.
        let shipped = crate::grid::variants(
            super::super::validate::DEFAULT_RUNGS,
            super::super::validate::DEFAULT_RUNGS,
            super::super::validate::DEFAULT_RUNGS,
        );
        assert_eq!(shipped, 625, "the shipped DEFAULT_RUNGS grid");
        let width = u64::try_from(shipped).expect("a grid width fits a u64");
        assert_eq!(
            trials_with_grid(trials(&s), width),
            20 * width,
            "twenty combinations examined at every exit setting is a family of \
             twenty times the grid, not of 20"
        );

        // NO GRID IS ONE GRID, NOT NO HYPOTHESES.
        assert_eq!(
            trials_with_grid(trials(&s), 0),
            20,
            "a zero variant count means no grid ran; collapsing the family to \
             zero would return a bar of zero and pass every row"
        );
        assert_eq!(
            trials_with_grid(trials(&s), 1),
            20,
            "one variant is the identity"
        );

        // AND THE CHARGE ACTUALLY RAISES THE BAR. Every assertion above would
        // also pass on a function that ignored `variants` and returned `trials`.
        assert!(
            bonferroni_t(trials_with_grid(trials(&s), width)) > bonferroni_t(trials(&s)),
            "charging the grid must make the bar harder to clear, not merely \
             change the number"
        );

        // SATURATES RATHER THAN WRAPS. A wrapped product would print a small,
        // plausible, catastrophically low bar.
        let huge = sweep(vec![level(1, 0, u64::MAX)]);
        assert_eq!(
            trials_with_grid(trials(&huge), u64::MAX),
            u64::MAX,
            "the product saturates at the ceiling instead of wrapping to a \
             small number"
        );
    }

    /// The check that this implements Harvey, Liu & Zhu's arithmetic.
    #[test]
    fn the_published_bonferroni_figure_is_reproduced() {
        // HLZ report 3.78 across 316 factors at a 5% family-wise error rate.
        // Reproducing it from this module's own constant is what distinguishes
        // an implementation of their method from a formula that resembles it.
        let t = bonferroni_t(316);
        assert!(
            (t - 3.78).abs() < 0.01,
            "expected the published 3.78 for 316 tests, computed {t}"
        );
    }

    #[test]
    fn the_bar_rises_with_the_number_of_hypotheses_and_never_falls() {
        // Zero tests have no threshold. The loop below starts at one, so the
        // empty case is stated here rather than left as an arm no run reaches.
        assert!(
            bonferroni_t(0).abs() < f64::EPSILON,
            "no hypotheses means no bar to clear"
        );
        let mut previous = 0.0_f64;
        for n in [
            1_u64,
            10,
            316,
            3_689,
            1_000_000,
            61_125_295,
            100_000_000_000,
        ] {
            let t = bonferroni_t(n);
            assert!(
                t >= previous,
                "the bar fell from {previous} to {t} going to {n} tests, which \
                 would mean testing MORE made a result easier to believe"
            );
            previous = t;
        }
        // And the figures this engine's own runs land on.
        assert!(bonferroni_t(61_125_295) > 6.0, "61M tests needs t > 6");
        assert!(
            bonferroni_t(100_000_000_000) > 7.0,
            "100 billion needs t > 7"
        );
    }

    #[test]
    fn the_expected_maximum_of_noise_matches_the_closed_form() {
        // sqrt(2 ln n), checked against values computed by hand.
        assert!((expected_max_t(61_125_295) - 5.99).abs() < 0.01);
        assert!((expected_max_t(100_000_000_000) - 7.12).abs() < 0.01);
        // Fewer than two trials has no meaningful maximum.
        assert!(expected_max_t(1).abs() < f64::EPSILON);
        assert!(expected_max_t(0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_published_expression_and_the_approximation_disagree_by_enough_to_matter() {
        // Both answer "what does the best of n worthless trials score". The
        // simple form OVERSTATES, which is safe but not accurate -- a real
        // finding sitting between the two would be rejected.
        for n in [3_689_u64, 61_125_295, 100_000_000_000] {
            let simple = expected_max_t(n);
            let published = expected_max_bailey(n);
            assert!(
                published < simple,
                "at {n} trials the published expression must sit BELOW the \
                 sqrt(2 ln n) approximation: {published} vs {simple}"
            );
            assert!(
                simple - published < 0.6,
                "and not wildly below it -- they measure the same thing: \
                 {published} vs {simple} at {n}"
            );
        }
        // The documented figures, to two decimals.
        // Pinned to what the function COMPUTES, not to a rounding of it: an
        // audit found the docstring table and the code disagreeing in the last
        // digit on all three rows.
        assert!((expected_max_bailey(61_125_295) - 5.63).abs() < 0.01);
        assert!((expected_max_bailey(3_689) - 3.61).abs() < 0.01);
        // And it rises with the trial count, never falls.
        let mut previous = 0.0_f64;
        for n in [2_u64, 100, 10_000, 1_000_000, 1_000_000_000] {
            let t = expected_max_bailey(n);
            assert!(t > previous, "the noise floor fell from {previous} to {t}");
            previous = t;
        }
        // Fewer than two trials has no maximum.
        assert!(expected_max_bailey(1).abs() < f64::EPSILON);
        assert!(expected_max_bailey(0).abs() < f64::EPSILON);
    }

    #[test]
    fn the_effective_count_removes_exact_duplicates_and_nothing_else() {
        use crate::{Sweeper, synthetic};
        use engine::Ladder;
        use indicators::evaluator::{Evaluator, Widths};
        use indicators::pattern::Thresholds;
        use indicators::vwap::Availability;

        let bars = synthetic::sessions(8);
        let mut ev = Evaluator::new(
            Widths::pinned().expect("pinned"),
            Availability::Absent,
            Thresholds::CLASSICAL,
        );
        let sweep = Sweeper::new(Ladder::with_min_hits(600).with_ceiling(50_000))
            .run(&bars, &mut ev)
            .sweep;

        let raw = trials(&sweep);
        let effective = effective_trials(&sweep);
        assert!(raw > 1_000, "the fixture must test thousands");
        assert!(
            effective < raw,
            "this fixture has redundant combinations, so the effective count \
             must be strictly smaller: {effective} vs {raw}"
        );
        // Exactly the redundancy closure found, and nothing beyond it.
        assert_eq!(
            raw - effective,
            crate::closed::closed(&sweep).redundant(),
            "the deflation must be exactly the exact-duplicate count -- anything \
             more would be an estimate this function does not make"
        );
        // And the bar moves the right way: fewer trials, a lower hurdle.
        assert!(
            bonferroni_t(effective) < bonferroni_t(raw),
            "removing duplicated tests must LOWER the bar, never raise it"
        );
        // Never below the number of genuinely distinct tests.
        assert!(effective > 0);
    }

    #[test]
    fn the_normal_cdf_and_its_quantile_agree_with_each_other() {
        // Two independent approximations of inverse functions. If they disagree
        // the p-values and the thresholds are on different scales, and every
        // comparison between them is meaningless.
        for p in [0.6_f64, 0.75, 0.9, 0.975, 0.99, 0.999] {
            let q = inverse_normal_cdf(p);
            let back = normal_cdf(q);
            assert!(
                (back - p).abs() < 1e-6,
                "round trip failed at p={p}: quantile {q} came back as {back}"
            );
        }
        // The textbook anchors.
        assert!((normal_cdf(0.0) - 0.5).abs() < 1e-9);
        assert!((normal_cdf(1.96) - 0.975).abs() < 1e-4);
        assert!((normal_cdf(-1.96) - 0.025).abs() < 1e-4);
    }

    #[test]
    fn a_p_value_is_two_sided_and_symmetric_in_the_sign_of_t() {
        // A combination preceding a FALL is as real as one preceding a rise, so
        // the p-value must not care which.
        assert!((p_value(1.96) - 0.05).abs() < 1e-3);
        assert!((p_value(-1.96) - 0.05).abs() < 1e-3);
        // 1e-6, matching the CDF approximation's own 1.5e-7 accuracy -- a tighter
        // tolerance would be asserting precision the function does not claim.
        assert!((p_value(0.0) - 1.0).abs() < 1e-6, "no evidence at all");
        assert!(p_value(6.0) < 1e-8, "six sigma is vanishingly unlikely");
        // Bounded, whatever is handed in.
        assert!((0.0..=1.0).contains(&p_value(f64::INFINITY)));
        assert!((0.0..=1.0).contains(&p_value(-40.0)));
    }

    #[test]
    fn benjamini_hochberg_takes_the_largest_passing_rank_not_the_first_failure() {
        // THE STEP THAT IS EASY TO GET WRONG. p3 fails its own threshold while
        // p4 passes; the procedure rejects FOUR, not two. Stopping at the first
        // failure is the classic misreading and yields two.
        // m = 5, alpha = 0.05, thresholds are 0.01, 0.02, 0.03, 0.04, 0.05.
        let mut p = [0.001, 0.015, 0.035, 0.039, 0.9];
        assert_eq!(
            benjamini_hochberg(&mut p),
            4,
            "0.035 exceeds its own threshold of 0.03, but 0.039 clears 0.04 -- \
             so everything up to rank four is rejected"
        );
    }

    #[test]
    fn benjamini_hochberg_is_more_permissive_than_bonferroni_and_that_is_the_point() {
        // The same p-values under both. BH must reject at least as many, or it
        // is not doing the job it exists for.
        let mut p: Vec<f64> = (1..=100).map(|i| f64::from(i) * 0.0004).collect();
        let bh = benjamini_hochberg(&mut p);
        let bonferroni = p.iter().filter(|x| **x <= 0.05 / 100.0).count();
        assert!(
            bh > bonferroni,
            "BH rejected {bh} and Bonferroni {bonferroni}; controlling the \
             FRACTION of false findings must admit more than controlling the \
             chance of any"
        );
    }

    #[test]
    fn benjamini_hochberg_rejects_nothing_when_nothing_deserves_it() {
        assert_eq!(benjamini_hochberg(&mut []), 0, "no hypotheses, no findings");
        let mut noise = [0.6_f64, 0.7, 0.8, 0.99];
        assert_eq!(benjamini_hochberg(&mut noise), 0, "pure noise yields none");
        // And everything, when everything deserves it.
        let mut strong = [1e-12_f64; 20];
        assert_eq!(benjamini_hochberg(&mut strong), 20);
    }

    #[test]
    fn a_hypothesis_is_one_that_was_measured_against_the_bars() {
        // 3 frequent + 7 infrequent at k=1, 2 + 8 at k=2 -> 20 support counts.
        // The other counters are candidates that never reached a bar.
        let s = sweep(vec![level(1, 3, 7), level(2, 2, 8)]);
        assert_eq!(
            trials(&s),
            20,
            "only the candidates whose support was actually counted are \
             hypotheses; duplicates, exclusions and prunes never saw a bar"
        );
        assert_eq!(trials(&sweep(Vec::new())), 0);
    }

    #[test]
    fn the_quantile_is_symmetric_and_refuses_impossible_probabilities() {
        // The median.
        assert!(inverse_normal_cdf(0.5).abs() < 1e-9);
        // A textbook pair, both tails.
        assert!((inverse_normal_cdf(0.975) - 1.96).abs() < 0.001);
        assert!((inverse_normal_cdf(0.025) + 1.96).abs() < 0.001);
        // The lower tail branch, which `bonferroni_t` never reaches.
        assert!(inverse_normal_cdf(0.001) < -3.0);
        // Outside the open interval there is no quantile.
        for p in [0.0, 1.0, -0.5, 2.0] {
            assert!(
                inverse_normal_cdf(p).abs() < f64::EPSILON,
                "there is no quantile at p = {p}"
            );
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tail_tests {
    use super::{bonferroni_t, expected_max_bailey, expected_max_t};

    /// THE BAR COLLAPSED TO EXACTLY ZERO, AND NO TEST ASKED WHETHER IT COULD.
    ///
    /// # The defect, bisected
    ///
    /// `bonferroni_t` read `inverse_normal_cdf(1.0 - FWER / (2n))`. Once the
    /// tail falls below half an ulp of 1.0 — `2^-53` — `1.0 - alpha` rounds to
    /// exactly `1.0`, and `inverse_normal_cdf` returns `0.0` for `p >= 1.0`. One
    /// integer wide:
    ///
    /// ```text
    /// bonferroni_t(450_359_962_737_049) = 8.209536
    /// bonferroni_t(450_359_962_737_050) = 0.000000
    /// ```
    ///
    /// A bar of zero passes every finding, while still being printed as a
    /// family-wise correction. That is worse than printing no bar at all.
    ///
    /// # Why this test is a property and not the two integers
    ///
    /// Asserting those two values would pin the old cliff's location and say
    /// nothing about a new one. The PROPERTY is that the bar never falls as the
    /// search grows and is never zero for a real search — which fails at any
    /// cliff, anywhere, including one introduced later by a different
    /// approximation.
    #[test]
    fn the_significance_bar_never_falls_as_the_search_grows() {
        // Across the whole u64 range by powers of two, plus the exact integers
        // either side of the old cliff and the largest count that exists.
        let mut probes: Vec<u64> = (0..64).map(|k| 1_u64 << k).collect();
        probes.extend([
            450_359_962_737_049,
            450_359_962_737_050,
            450_359_962_737_051,
            6_627_126_856_707_895,
            6_627_126_856_707_896,
            u64::MAX,
        ]);
        probes.sort_unstable();
        probes.dedup();

        let mut previous = 0.0_f64;
        for n in probes {
            let bar = bonferroni_t(n);
            assert!(
                bar.is_finite(),
                "the bar must be a number at n = {n}, and it was {bar}"
            );
            assert!(
                bar > 0.0,
                "a bar of zero passes every finding while claiming to be a \
                 family-wise correction. At n = {n} it was {bar}"
            );
            assert!(
                bar >= previous,
                "the bar must never FALL as the search grows: n = {n} gives \
                 {bar}, below the {previous} a smaller search demanded"
            );
            previous = bar;
        }
    }

    /// THE SAME CLIFF, IN BAILEY'S EXPECTED MAXIMUM.
    ///
    /// It formed `1.0 - 1.0 / n` and `1.0 - 1.0 / (n * e)`, so it dropped 4.739
    /// across one integer near `n = 6.6e15` — while `sqrt(2 ln n)`, the
    /// approximation it sits beside, was still rising through 8.54. Two
    /// estimates of the same quantity moving in opposite directions is the
    /// shape the defect took.
    #[test]
    fn the_expected_maximum_never_falls_and_tracks_its_own_approximation() {
        let mut previous = 0.0_f64;
        for k in 1..64_u32 {
            let n = 1_u64 << k;
            let exact = expected_max_bailey(n);
            assert!(exact.is_finite() && exact > 0.0, "n = {n} gave {exact}");
            assert!(
                exact >= previous,
                "the expected maximum of noise must rise with the number of \
                 draws: n = {n} gives {exact}, below {previous}"
            );
            previous = exact;

            // AND THE TWO ESTIMATES MUST NOT DIVERGE. `expected_max_t` is the
            // `sqrt(2 ln n)` approximation of the same quantity; the published
            // expression is the more accurate one and sits BELOW it. A gap
            // wider than one whole t-unit means one of them has broken, which
            // is exactly what the cliff did.
            let approx = expected_max_t(n);
            assert!(
                (approx - exact).abs() < 1.0,
                "the two estimates of the expected maximum disagree by {} at \
                 n = {n}: closed form {exact}, approximation {approx}",
                (approx - exact).abs()
            );
        }
    }

    /// THE SMALL END STILL ANSWERS, AND THE PUBLISHED FIGURE IS UNMOVED.
    ///
    /// The fix routes `alpha >= P_LOW` back through the central rational form,
    /// so nothing about the reachable range may shift. Harvey, Liu & Zhu's 316
    /// factors give 3.78, and that is the check that the arithmetic is still
    /// theirs.
    #[test]
    fn the_reachable_range_is_unchanged_by_the_tail_fix() {
        assert!(
            (bonferroni_t(316) - 3.78).abs() < 0.01,
            "the published Bonferroni figure moved: {}",
            bonferroni_t(316)
        );
        // EXACT COMPARISONS, AND DELIBERATELY SO. These three are the sentinel
        // returns the functions promise for a degenerate count -- a literal
        // `return 0.0` taken before any arithmetic -- not a computed value that
        // happens to land near zero. `clippy::float_cmp` is right about computed
        // floats and wrong about a documented sentinel, which is why the
        // exception is taken here and named rather than blanket-allowed.
        #[allow(
            clippy::float_cmp,
            reason = "these are documented sentinel returns taken before any \
                      arithmetic, not computed values."
        )]
        {
            assert_eq!(bonferroni_t(0), 0.0, "no trials is not a search");
            assert_eq!(expected_max_bailey(1), 0.0, "one draw has no maximum");
            assert_eq!(expected_max_bailey(0), 0.0);
        }

        // THE SHIPPED CEILING, WHICH IS WHERE THIS ACTUALLY RUNS.
        // `engine::DEFAULT_PAIR_BUDGET` is `1 << 34`, and the exit grid is 325
        // cells, so the largest family a default run can present is about
        // 5.6e12 -- roughly 80x below where the old cliff sat. The fix is not
        // needed for the default path and is needed because both entry points
        // are `pub` and take any `u64`.
        let shipped = (1_u64 << 34).saturating_mul(325);
        let bar = bonferroni_t(shipped);
        assert!(
            bar > 7.0 && bar < 9.0,
            "the bar at the shipped ceiling should sit near 8, and it is {bar}"
        );
    }
}
