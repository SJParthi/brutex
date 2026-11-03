#![allow(clippy::float_cmp, reason = "exact edge answers are compared exactly")]
//! BASE-COV-LIBS: the incomplete beta's and the quantile's edge answers, and
//! the continued fraction's guard against a zero denominator.

use super::*;

#[test]
fn the_incomplete_beta_answers_its_edges_exactly() {
    assert!(regularized_incomplete_beta(2.0, 3.0, f64::NAN).is_nan());
    assert_eq!(regularized_incomplete_beta(2.0, 3.0, 0.0), 0.0);
    assert_eq!(regularized_incomplete_beta(2.0, 3.0, -1.0), 0.0);
    assert_eq!(
        regularized_incomplete_beta(2.0, 3.0, f64::NEG_INFINITY),
        0.0
    );
    assert_eq!(regularized_incomplete_beta(2.0, 3.0, 1.0), 1.0);
    assert_eq!(regularized_incomplete_beta(2.0, 3.0, f64::INFINITY), 1.0);
    let half = regularized_incomplete_beta(4.0, 4.0, 0.5);
    assert!((half - 0.5).abs() < 1e-12, "{half}");
}

#[test]
fn a_zero_lentz_denominator_is_guarded_never_infinite() {
    // 1 - (a + b) x / (a + 1) is exactly zero at a = b = x = 1.
    let guarded = beta_continued_fraction(1.0, 1.0, 1.0);
    assert!(!guarded.is_infinite(), "{guarded}");
}

#[test]
fn an_upper_tail_quantile_exists_only_strictly_inside_zero_and_one() {
    for alpha in [
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        0.0,
        -0.1,
        1.0,
        1.5,
    ] {
        assert_eq!(upper_tail_quantile(alpha), 0.0, "{alpha}");
    }
    let five = upper_tail_quantile(0.05);
    assert!((five - 1.644_853_626_951).abs() < 1e-6, "{five}");
    let tail = upper_tail_quantile(0.001);
    assert!((tail - 3.090_232_306_168).abs() < 1e-6, "{tail}");
}

/// BASE-COV-LIBS-LS2: A FRACTION THAT NEVER CONVERGES IS NOT A PROBABILITY.
/// A non-finite parameter keeps every Lentz factor away from one, so the
/// fraction exhausts its terms and answers NaN, which every caller reads as
/// "does not clear"; the same parameters at their finite neighbours converge.
#[test]
fn a_fraction_that_never_converges_answers_nan_rather_than_a_number() {
    for (a, b) in [
        (f64::NAN, 3.0),
        (2.0, f64::NAN),
        (f64::INFINITY, 3.0),
        (f64::NAN, f64::NAN),
    ] {
        assert!(beta_continued_fraction(a, b, 0.25).is_nan(), "{a} {b}");
    }
    let converged = beta_continued_fraction(2.0, 3.0, 0.25);
    assert!(converged.is_finite() && converged > 0.0, "{converged}");
}
