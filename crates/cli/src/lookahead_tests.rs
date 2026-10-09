//! What a later bar may and may not move: audit-find-17 #5 and #6. D-3696,
//! D-3697.
#![allow(
    clippy::expect_used,
    clippy::indexing_slicing,
    clippy::panic,
    reason = "a failed fixture must fail its test"
)]

use super::*;

/// Both walk-forward shapes over `bars`, as `audit_bars_work` calls them.
fn walks(
    bars: &[indicators::Candle],
    support: runner::validate::FoldSupport,
) -> (runner::validate::Validated, runner::validate::Validated) {
    let fresh = evaluator().expect("the default evaluator");
    let first = runner::rank::Scored {
        mask: vocab::ConditionMask::default(),
        hits: 1,
        edge: runner::outcome::Edge::default(),
    };
    both_shapes(
        bars,
        Execution {
            bars,
            signal_length_micros: 60_000_000,
        },
        horizon_for(bars, false),
        &first,
        Ladder::with_min_hits(120)
            .with_ceiling(20_000)
            .with_support_lanes(1),
        &fresh,
        None,
        runner::validate::FoldRungs::Fixed(4),
        support,
        &|_| {},
    )
}

/// `bars` with its LAST bar's high doubled: a bar after every fold's training
/// window and after every trade but the last.
fn with_a_later_high(bars: &[indicators::Candle]) -> Vec<indicators::Candle> {
    let mut later = bars.to_vec();
    let last = later.last_mut().expect("a non-empty fixture");
    last.high = last.high.saturating_mul(2);
    later
}

/// #6, D-3697. A stop stated in points is converted at the WHOLE span's
/// reference, so a later bar moves the in-sample stop: measured, 1,999 ppm
/// becomes 1,332 when only the last bar's high doubles. That is the screen's
/// whole-span derivation, in-sample by construction (D-1660), and stated as a
/// limit rather than hidden. What must NOT move is the walk-forward: no fold's
/// grid reads the knob, so setting it leaves both shapes byte-for-byte equal.
#[test]
fn a_stated_stop_is_span_relative_in_sample_and_never_reaches_a_fold() {
    let _serial = crate::knobs::serially();
    crate::knobs::clear_all();
    let bars = runner::synthetic::sessions(16);
    let unset = walks(&bars, runner::validate::FoldSupport::Scaled);

    crate::knobs::set("BRUTEX_MAX_STOP_POINTS", "50");
    let span = stated_stop_ppm(&bars);
    let later = stated_stop_ppm(&with_a_later_high(&bars));
    let set = walks(&bars, runner::validate::FoldSupport::Scaled);
    crate::knobs::clear_all();

    assert_eq!(span, Some(points_to_ppm_at(50, reference_price(&bars))));
    assert_ne!(
        later, span,
        "the in-sample stop is span-relative; if this stops moving, D-3697's limit is stale"
    );
    assert!(!unset.0.folds.is_empty(), "the fixture walks");
    assert_eq!(set, unset, "a stated stop reached the walk-forward");
}

/// #5, D-3696. The range path's support policy is an identity term: its word,
/// then the probe ceiling, or `u64::MAX` for none. Two runs that differ only in
/// how their folds derive support are two runs.
#[test]
fn the_fold_support_policy_is_appended_to_the_span_identity() {
    let policy = core::array::from_fn::<u64, 23, _>(|at| u64::try_from(at).expect("small") + 7);
    let scaled = with_fold_support(policy, runner::validate::FoldSupport::Scaled);
    let probed = with_fold_support(
        policy,
        runner::validate::FoldSupport::FirstTraining {
            probe: Ladder::with_min_hits(1).with_ceiling(5_000),
            floor: 3,
        },
    );
    assert_eq!(scaled[..23], policy, "the policy is kept, in place");
    assert_eq!(probed[..23], policy, "the policy is kept, in place");
    assert_eq!(scaled[23..], [FOLD_SUPPORT_SCALED, u64::MAX]);
    assert_eq!(probed[23..], [FOLD_SUPPORT_FIRST_TRAINING, 5_000]);
    assert_ne!(FOLD_SUPPORT_SCALED, FOLD_SUPPORT_FIRST_TRAINING);
    assert_ne!(FOLD_SUPPORT_SCALED, 0, "zero is never written");
}

/// #5, D-3696. The walk-forward's probe spends the share the whole-span probe
/// spends: one ladder for both, not a second number that can drift.
#[test]
fn the_fold_probe_and_the_whole_span_probe_share_one_ladder() {
    let probe = probe_ladder();
    let floor = whole_machine_ceiling()
        .checked_div(PROBE_SHARE)
        .unwrap_or(SEARCH_CEILING)
        .max(SEARCH_CEILING);
    assert_eq!(probe.ceiling(), floor);
    assert_eq!(probe.min_hits(), 1);
    let body = include_str!("lib.rs");
    let affordable = body
        .split("\nfn affordable_min_hits(")
        .nth(1)
        .and_then(|rest| rest.split("\nfn ").next())
        .expect("affordable_min_hits exists");
    assert!(affordable.contains("let ladder = probe_ladder();"));
    let one_rung = body
        .split("\nfn one_rung_cached(")
        .nth(1)
        .and_then(|rest| rest.split("\nfn ").next())
        .expect("one_rung_cached exists");
    assert!(
        one_rung.contains("probe: probe_ladder(),"),
        "the folds re-probe under the same ladder"
    );
}
