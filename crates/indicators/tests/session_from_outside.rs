//! `SessionState` and the aggregate evaluator, driven from **outside** the crate.
//!
//! Every verdict below is reachable through the public surface alone — `SessionState::new`,
//! `step`, `bits`, `day_open`, `day_extremes`, the two threshold structs and
//! `Evaluator::step` — and that is the whole reason they are asserted here as well as in
//! `session.rs`'s own `mod tests`. A unit test inside the module runs against the crate
//! compiled with `cfg(test)`; a binary in this directory links the crate compiled plain,
//! and that second compilation is the only one a consumer can ever hold.
//!
//! Nothing in this directory had asked for any of it. `DayWindows::default`,
//! `ShapeLimits::default`, both `Option` accessors, the guard at the head of the
//! yesterday block and the whole gap-midpoint band had no caller outside the module, so
//! nothing established that a consumer can reach the same answers without the private
//! fields the module's own tests reach for.

#![allow(
    clippy::expect_used,
    reason = "the same exception every test module in this workspace takes — see \
              tests/extremes.rs beside this file: a test that cannot panic cannot fail. \
              `expect` and not `let ... else { unreachable!() }`, because `unreachable!` \
              expands to a panic inside the crate under test and is therefore a coverage \
              region no green run can ever execute, while `expect` panics inside the \
              standard library and leaves no such region behind."
)]

use indicators::evaluator::{Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::session::{DayWindows, PreviousSession, SessionState, ShapeLimits};
use indicators::vwap::Availability;
use indicators::{Candle, Corrupt};
use vocab::{ConditionMask, Tolerance};

const DAY_MICROS: i64 = 24 * 60 * 60 * 1_000_000;
const MINUTE_MICROS: i64 = 60 * 1_000_000;
/// 09:15 IST expressed in UTC micros — the offset is 05:30 exactly, all year.
const IST_OPEN_UTC_MICROS: i64 = (555 - 330) * 60 * 1_000_000;
/// The IST day every fixture below sits in. Any day works; naming one keeps the
/// timestamps readable.
const DAY: i64 = 40_000;

fn tol() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("the pinned fib width is valid")
}

fn widths() -> Widths {
    Widths::pinned().expect("both pinned widths are valid")
}

/// A bar `minute` minutes after the open of [`DAY`].
fn at(minute: i64, open: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle {
        ts_micros: DAY * DAY_MICROS + IST_OPEN_UTC_MICROS + minute * MINUTE_MICROS,
        open,
        high,
        low,
        close,
        volume: 0,
        open_interest: i64::MIN,
    }
}

/// A zero-range bar: a limit-locked or untraded minute, and the cheapest way to put a
/// session's extreme at an exact price.
fn flat(ts_micros: i64, price: i64) -> Candle {
    Candle {
        ts_micros,
        open: price,
        high: price,
        low: price,
        close: price,
        volume: 0,
        open_interest: i64::MIN,
    }
}

fn step(state: &mut SessionState, bar: &Candle, prev: Option<PreviousSession>) -> ConditionMask {
    state
        .step(bar, prev, tol())
        .expect("this fixture bar is sane")
}

/// Yesterday, as the day-type and gap families see it.
fn yesterday() -> PreviousSession {
    PreviousSession {
        high: 2_520_000,
        low: 2_480_000,
        close: 2_500_000,
    }
}

/// Both threshold sets default to the classical ones, and the defaulted state is the
/// one built from them.
///
/// `SessionState::new` takes both structs, so a consumer wanting the shipped behaviour
/// writes `DayWindows::default()`. If either `Default` drifted from `CLASSICAL`, the
/// nine positions those numbers decide would move with no caller changing a line.
#[test]
fn the_two_threshold_sets_default_to_the_classical_ones() {
    assert_eq!(
        DayWindows::default(),
        DayWindows::CLASSICAL,
        "the window set drifted from CLASSICAL"
    );
    assert_eq!(
        ShapeLimits::default(),
        ShapeLimits::CLASSICAL,
        "the shape limits drifted from CLASSICAL"
    );

    let bar = at(0, 2_499_200, 2_500_000, 2_499_000, 2_499_800);
    let mut defaulted = SessionState::default();
    let mut spelled_out = SessionState::new(DayWindows::default(), ShapeLimits::default());
    assert_eq!(
        step(&mut defaulted, &bar, None).words(),
        step(&mut spelled_out, &bar, None).words(),
        "`SessionState::default()` and the spelled-out constructor disagree"
    );
}

/// A state before its first bar answers nothing, and emits no day-type verdict even
/// with yesterday in hand.
///
/// `bits` is public and takes `&self`, so `step` is not the only door: a consumer can
/// ask an unstarted state for a mask. The fields behind `day_open` and `day_extremes`
/// hold **zero** until a bar arrives, so without the guard the answer would be a gap the
/// width of the entire price scale, measured off a session that has not begun.
#[test]
fn an_unstarted_session_answers_nothing_and_emits_no_day_type() {
    let state = SessionState::new(DayWindows::CLASSICAL, ShapeLimits::CLASSICAL);
    assert_eq!(state.day_open(), None, "an unstarted session named an open");
    assert_eq!(
        state.day_extremes(),
        None,
        "an unstarted session named its extremes"
    );

    let bar = at(0, 2_510_000, 2_512_000, 2_504_000, 2_511_000);
    let mask = state.bits(&bar, Some(yesterday()), tol());
    for index in [40, 41, 42, 43, 48, 49, 50, 51, 66, 67, 68] {
        assert!(
            !mask.get(index),
            "position {index} was emitted before the session began"
        );
    }
    assert!(
        mask.get(30),
        "the bar's own shape needs no session, and was not emitted either"
    );
}

/// A 60% body is a large body and a 45% body is not.
///
/// The limit is 600 thousandths of the bar's own range and `ShapeLimits` is the only
/// place that says so. The two bars differ by 150 paisa of body over the same
/// 1000-paisa range, so a limit misread as 450 — or a `>=` written `>` — separates them.
#[test]
fn a_sixty_percent_body_is_large_and_a_forty_five_percent_body_is_not() {
    let mut large = SessionState::default();
    let mask = step(
        &mut large,
        &at(0, 2_499_200, 2_500_000, 2_499_000, 2_499_800),
        None,
    );
    assert!(
        mask.get(33),
        "a 600-paisa body over a 1000-paisa range was not called large"
    );
    assert!(!mask.get(34), "a 60% body was also called small");

    let mut middling = SessionState::default();
    let mask = step(
        &mut middling,
        &at(0, 2_499_275, 2_500_000, 2_499_000, 2_499_725),
        None,
    );
    assert!(
        !mask.get(33),
        "a 450-paisa body over the same range was called large"
    );
}

/// The afternoon window is the fourth and last, and 14:00 is its first minute.
///
/// The four windows are half-open and exhaustive, so the boundary is the interesting
/// input: 13:59 belongs to `midday` alone and 14:00 to `afternoon` alone. An overlap
/// would make two positions true for one bar, and a gap would make none.
#[test]
fn the_afternoon_window_starts_where_midday_ends() {
    // 285 minutes after 09:15 is 14:00.
    let mut afternoon = SessionState::default();
    let mask = step(&mut afternoon, &at(285, 100, 110, 95, 108), None);
    assert!(mask.get(47), "14:00 did not match the afternoon window");
    for index in 44..=46 {
        assert!(!mask.get(index), "14:00 also matched window {index}");
    }

    let mut midday = SessionState::default();
    let mask = step(&mut midday, &at(284, 100, 110, 95, 108), None);
    assert!(
        mask.get(46) && !mask.get(47),
        "13:59 did not belong to midday alone"
    );
}

/// An outside day needs **both** extremes beyond yesterday's, and an inside day both
/// within.
///
/// One extreme beyond is neither, and that is the case a `||` written for an `&&` would
/// get wrong while both other rows stayed green.
#[test]
fn an_outside_day_needs_both_extremes_beyond_yesterday() {
    // (name, high, low, inside day, outside day)
    let cases: [(&str, i64, i64, bool, bool); 4] = [
        ("both extremes beyond", 2_530_000, 2_470_000, false, true),
        ("both extremes within", 2_510_000, 2_490_000, true, false),
        ("a higher high alone", 2_530_000, 2_490_000, false, false),
        ("a lower low alone", 2_510_000, 2_470_000, false, false),
    ];
    for (name, high, low, inside, outside) in cases {
        let mut state = SessionState::default();
        let bar = at(0, 2_500_000, high, low, 2_500_000);
        let mask = step(&mut state, &bar, Some(yesterday()));
        assert_eq!(mask.get(50), inside, "{name}: inside day");
        assert_eq!(mask.get(51), outside, "{name}: outside day");
    }
}

/// The gap-midpoint band is exactly the pinned width of the gap, and its edge is
/// inclusive.
///
/// The band is 10/1000 of the gap, so 100 paisa either side of the midpoint of a
/// 10,000-paisa gap. The two closes below sit at 100 and at 101 paisa from it. Position
/// 68 is the one `Near` position this family owns and `vocab::table::set_near` returns
/// the mask unchanged whether it sets a bit or declines, so a wrong index, a wrong kind
/// or a wrong base would be silent rather than loud.
#[test]
fn the_gap_midpoint_band_is_the_pinned_width_of_the_gap() {
    // The gap runs 2_500_000..2_510_000, so the midpoint is 2_505_000 exactly.
    let mut edge = SessionState::default();
    let mask = step(
        &mut edge,
        &at(0, 2_510_000, 2_512_000, 2_504_000, 2_505_100),
        Some(yesterday()),
    );
    assert!(
        mask.get(68),
        "a close 100 paisa from the midpoint, on the band's own edge, did not set it"
    );

    let mut past = SessionState::default();
    let mask = step(
        &mut past,
        &at(0, 2_510_000, 2_512_000, 2_504_000, 2_505_101),
        Some(yesterday()),
    );
    assert!(
        !mask.get(68),
        "a close one paisa past the band's edge set it anyway"
    );
    assert!(
        mask.get(66),
        "that close is still above the midpoint, and was not reported"
    );
}

/// A gap wider than `i64` drops the band rather than wrapping.
///
/// The gap is taken in `i128`, because `today_open - prev_close` for two `i64` prices
/// need not fit an `i64`, and `set_near` needs it back as an `i64` base. The
/// explicit low-level previous reference is signed; today's evaluated bar is
/// positive. Negative current candles must refuse before touching state.
#[test]
fn a_gap_wider_than_the_type_drops_the_band() {
    let prev = PreviousSession {
        high: 0,
        low: i64::MIN,
        close: i64::MIN,
    };
    let mut state = SessionState::default();
    assert_eq!(
        state.step(
            &at(0, i64::MIN, i64::MIN, i64::MIN, i64::MIN),
            Some(prev),
            tol()
        ),
        Err(Corrupt::PriceNotPositive)
    );
    assert_eq!(
        state.day_open(),
        None,
        "the refused negative candle must not seed the session"
    );
    let mask = step(
        &mut state,
        &at(0, i64::MAX, i64::MAX, i64::MAX, i64::MAX),
        Some(prev),
    );
    assert!(
        mask.get(48),
        "an open above the previous reference close is a gap up at any width"
    );
    assert!(
        mask.get(66),
        "the close sits above that gap's midpoint and was not reported"
    );
    assert!(
        !mask.get(68),
        "a gap that does not fit i64 still produced a band"
    );
}

/// A completed positive-price session whose derived pivot extensions leave
/// `i64` leaves the pivot ladder absent; an ordinary session still produces one.
/// The legacy test name predates the positive-price admission rule. Its original
/// purpose remains: a completed but unusable ladder cannot be half-installed.
#[test]
fn a_completed_session_wider_than_the_type_leaves_yesterday_absent() {
    let open = DAY * DAY_MICROS + IST_OPEN_UTC_MICROS;

    let mut wide = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
    let _ = wide
        .step(&flat(open, 1))
        .expect("a positive one-paisa candle is evaluable");
    let _ = wide
        .step(&flat(open + MINUTE_MICROS, i64::MAX))
        .expect("a positive maximum-price candle is evaluable");
    // The session span fits, while derived extensions leave the price type.
    let _ = wide
        .step(&flat(open + DAY_MICROS, 2_500_000))
        .expect("a sane bar the next day");
    assert_eq!(
        wide.sessions_completed(),
        1,
        "the completed session was not counted at all"
    );
    assert!(
        !wide.has_yesterday(),
        "a pivot ladder was built despite derived levels leaving i64"
    );

    let mut ordinary = Evaluator::new(widths(), Availability::Absent, Thresholds::CLASSICAL);
    let _ = ordinary.step(&flat(open, 2_500_000)).expect("a sane bar");
    let _ = ordinary
        .step(&flat(open + DAY_MICROS, 2_500_000))
        .expect("a sane bar the next day");
    assert!(
        ordinary.has_yesterday(),
        "an ordinary completed session produced no ladder either"
    );
}

/// An overflowing volume-weighted accumulator reaches an outside caller as a refusal.
///
/// Every other refusal this crate makes is decidable from the record alone and is
/// therefore made by `Candle::check` before the first module runs. This one is not: it
/// depends on what the session has already accumulated, so `Evaluator::step` is the only
/// place a consumer can learn about it. It used to be swallowed — the VWAP result was
/// read through an `if let Ok(v)`, and the bar came back as a successful evaluation with
/// the whole twenty-position family missing.
///
/// `Availability::Present` is load-bearing: on index spot the verdict is `Absent`, the
/// accumulator is never touched, and this refusal cannot happen at all. The same bar with
/// a zero volume is asserted legal, because §7 says zero is a real zero — so the refusal
/// is pinned to the accumulator and not to the price.
#[test]
fn an_overflowing_accumulator_is_refused_by_the_aggregate_evaluator() {
    // (high + low + close) is 1.5e19 here, and its square is about 2.25e38 — past i128.
    let price = 5_000_000_000_000_000_000_i64;
    let open = DAY * DAY_MICROS + IST_OPEN_UTC_MICROS;
    let mut with_volume = Evaluator::new(widths(), Availability::Present, Thresholds::CLASSICAL);
    let mut bar = flat(open, price);
    bar.volume = 1;
    assert_eq!(
        with_volume.step(&bar),
        Err(indicators::Corrupt::AccumulatorTooLarge),
        "a squared price past i128 came back as a successful evaluation"
    );

    let mut untraded = Evaluator::new(widths(), Availability::Present, Thresholds::CLASSICAL);
    assert!(
        untraded.step(&flat(open, price)).is_ok(),
        "the same price with a zero volume contributes nothing and must be accepted"
    );
}
