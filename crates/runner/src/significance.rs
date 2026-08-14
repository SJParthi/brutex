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
/// | 3,689 | 4.05 | 3.60 |
/// | 61,125,295 | 5.99 | 5.62 |
/// | 100,000,000,000 | 7.12 | 6.80 |
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
    let a = inverse_normal_cdf(1.0 - 1.0 / n_f);
    let b = inverse_normal_cdf(1.0 - 1.0 / (n_f * core::f64::consts::E));
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
    inverse_normal_cdf(1.0 - FWER / (2.0 * n_f))
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
    let q = if upper {
        (-2.0 * (1.0 - p).ln()).sqrt()
    } else {
        (-2.0 * p.ln()).sqrt()
    };
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
    // The rational form is written for the LOWER tail and returns a negative
    // value there; the upper tail is its mirror. Getting this backwards made
    // `bonferroni_t(316)` return -3.78, which the published-figure test caught
    // on the first run -- the reason that test asserts the sign and not just the
    // magnitude.
    if upper { -(num / den) } else { num / den }
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    reason = "the exception every test module in this workspace takes."
)]
mod tests {
    use super::{bonferroni_t, expected_max_bailey, expected_max_t, inverse_normal_cdf, trials};
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
        assert!((expected_max_bailey(61_125_295) - 5.62).abs() < 0.02);
        assert!((expected_max_bailey(3_689) - 3.60).abs() < 0.02);
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
