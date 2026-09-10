//! Independent finite generated-fixture checks of completed-session Fibonacci
//! truth, false-answer availability and negation. These are not market data.

#![allow(
    clippy::expect_used,
    reason = "fixture failures name the broken premise"
)]

use indicators::Candle;
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use vocab::expression::{Expression, Truth};
use vocab::tolerance::Base;
use vocab::{ConditionMask, Tolerance};

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
// Independent vocabulary-position oracle, deliberately not the implementation's
// rung arrays: (position, numerator, measured down, five-session reference).
const RUNGS: [(u16, i64, bool, bool); 27] = [
    (20, 236, true, false),
    (21, 382, true, false),
    (22, 500, true, false),
    (23, 618, true, false),
    (24, 786, true, false),
    (26, 1272, true, false),
    (27, 1618, true, false),
    (28, 2000, true, false),
    (29, 2618, true, false),
    (69, 236, false, false),
    (70, 786, false, false),
    (106, 1272, false, false),
    (107, 1618, false, false),
    (108, 2000, false, false),
    (109, 2618, false, false),
    (71, 4236, false, false),
    (110, 0, false, true),
    (111, 236, false, true),
    (112, 382, false, true),
    (113, 500, false, true),
    (114, 618, false, true),
    (115, 786, false, true),
    (116, 1000, false, true),
    (117, 1272, false, true),
    (118, 1618, false, true),
    (119, 2000, false, true),
    (120, 2618, false, true),
];

fn candle(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle::new(
        day * DAY + (225 + minute) * MINUTE,
        close,
        high,
        low,
        close,
        0,
        i64::MIN,
    )
}

fn pinned() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("pinned Fibonacci tolerance")
}

fn evaluator(tolerance: Tolerance, calendar: Calendar) -> Evaluator {
    Evaluator::with_calendar(
        Widths {
            fib: tolerance,
            pivot: vocab::tolerance::pinned_pivot().expect("pivot tolerance"),
        },
        Availability::Absent,
        Thresholds::CLASSICAL,
        calendar,
    )
}

fn seeded(tolerance: Tolerance, high: i64, low: i64, close: i64) -> Evaluator {
    let mut result = evaluator(tolerance, Calendar::all_regular());
    for day in 30_000..30_005 {
        result
            .step_known(&candle(day, 0, high, low, close))
            .expect("valid completed-session seed");
    }
    result
}

fn level(high: i64, low: i64, numerator: i64, down: bool) -> Option<i64> {
    let range = i128::from(high) - i128::from(low);
    if range <= 0 || range > i128::from(i64::MAX) {
        return None;
    }
    let step = range * i128::from(numerator) / 1000;
    i64::try_from(if down {
        i128::from(high) - step
    } else {
        i128::from(low) + step
    })
    .ok()
}

fn expected(
    reference: Option<(i64, i64)>,
    numerator: i64,
    down: bool,
    close: i64,
    tolerance: Tolerance,
) -> Truth {
    let Some((high, low)) = reference else {
        return Truth::Unknown;
    };
    if tolerance.base() != Some(Base::SessionRange) {
        return Truth::Unknown;
    }
    let Some(level) = level(high, low, numerator, down) else {
        return Truth::Unknown;
    };
    let range = i128::from(high) - i128::from(low);
    if (i128::from(close) - i128::from(level)).abs() * 1000 <= i128::from(tolerance.milli()) * range
    {
        Truth::True
    } else {
        Truth::False
    }
}

fn assert_answers(
    truth: ConditionMask,
    known: ConditionMask,
    close: i64,
    previous: Option<(i64, i64)>,
    five: Option<(i64, i64)>,
    tolerance: Tolerance,
) -> [Truth; 27] {
    let answers = RUNGS.map(|(bit, numerator, down, is_five)| {
        let answer = expected(
            if is_five { five } else { previous },
            numerator,
            down,
            close,
            tolerance,
        );
        assert_eq!(
            truth.get(u32::from(bit)),
            answer == Truth::True,
            "truth bit {bit}, close {close}"
        );
        assert_eq!(
            known.get(u32::from(bit)),
            answer != Truth::Unknown,
            "known bit {bit}, close {close}"
        );
        let atom = Expression::parse(&bit.to_string()).expect("live Fibonacci position");
        assert_eq!(atom.evaluate(truth, known), answer, "atom {bit}");
        let opposite = match answer {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        };
        assert_eq!(
            Expression::parse(&format!("!{bit}"))
                .expect("valid negation")
                .evaluate(truth, known),
            opposite,
            "NOT {bit}"
        );
        answer
    });
    for retired in [19, 25] {
        assert!(
            !truth.get(retired) && !known.get(retired),
            "retired position remains unavailable"
        );
    }
    assert_eq!(
        truth.union(&known),
        known,
        "every emitted truth has a known answer"
    );
    answers
}

fn fold(actual: &mut Evaluator, bar: &Candle) -> (ConditionMask, ConditionMask) {
    let mut legacy = *actual;
    let result = actual
        .step_known(bar)
        .expect("valid fixture bar must not be refused");
    assert_eq!(legacy.step(bar).expect("same truth-only fold"), result.0);
    result
}

#[test]
fn all_twenty_seven_predicates_and_not_match_integer_oracle_at_both_band_edges() {
    let (high, low) = (112_345, 100_000);
    let tolerance = pinned();
    let seed = seeded(tolerance, high, low, 105_000);
    let band = tolerance.milli() * (high - low) / 1000;
    let mut counts = [[0_u32; 2]; 27];
    for (_, numerator, down, _) in RUNGS {
        let center = level(high, low, numerator, down).expect("fixture rung fits");
        for delta in [-band - 1, -band, 0, band, band + 1] {
            let close = center + delta;
            let mut actual = seed;
            let (truth, known) = fold(&mut actual, &candle(30_005, 0, close, close, close));
            let answers = assert_answers(
                truth,
                known,
                close,
                Some((high, low)),
                Some((high, low)),
                tolerance,
            );
            for (answer, [trues, falses]) in answers.into_iter().zip(&mut counts) {
                assert_ne!(
                    answer,
                    Truth::Unknown,
                    "all fixture levels have a reference"
                );
                *trues += u32::from(answer == Truth::True);
                *falses += u32::from(answer == Truth::False);
            }
        }
    }
    for ((bit, _, _, _), [trues, falses]) in RUNGS.into_iter().zip(counts) {
        assert!(
            trues > 0 && falses > 0,
            "bit {bit} must exercise true and known false"
        );
    }
    let mut expected_positions = RUNGS.map(|(bit, _, _, _)| bit);
    expected_positions.sort_unstable();
    let mut actual_positions = indicators::fib::positions();
    actual_positions.sort_unstable();
    assert_eq!(actual_positions, expected_positions);
}

#[test]
fn five_completed_sessions_are_required_and_current_extremes_do_not_change_frozen_references() {
    let tolerance = pinned();
    let mut actual = evaluator(tolerance, Calendar::all_regular());
    for day in 30_000..30_006 {
        let (truth, known) = fold(&mut actual, &candle(day, 0, 112_345, 100_000, 105_000));
        assert_answers(
            truth,
            known,
            105_000,
            (day > 30_000).then_some((112_345, 100_000)),
            (day >= 30_005).then_some((112_345, 100_000)),
            tolerance,
        );
        if day == 30_000 {
            // Hundreds of bars in one session cannot warm a five-session reference.
            for minute in 1..375 {
                let (truth, known) =
                    fold(&mut actual, &candle(day, minute, 112_345, 100_000, 105_000));
                assert_answers(truth, known, 105_000, None, None, tolerance);
            }
        }
    }
    let (truth, known) = fold(&mut actual, &candle(30_005, 1, 900_000, 1, 500_000));
    assert_answers(
        truth,
        known,
        500_000,
        Some((112_345, 100_000)),
        Some((112_345, 100_000)),
        tolerance,
    );
    let (truth, known) = fold(&mut actual, &candle(30_006, 0, 500_000, 500_000, 500_000));
    assert_answers(
        truth,
        known,
        500_000,
        Some((900_000, 1)),
        Some((900_000, 1)),
        tolerance,
    );
}

#[test]
fn zero_range_is_unknown_until_positive_completed_reference_exists() {
    let tolerance = pinned();
    let mut actual = seeded(tolerance, 100_000, 100_000, 100_000);
    let (truth, known) = fold(&mut actual, &candle(30_005, 0, 112_345, 100_000, 105_000));
    assert_answers(
        truth,
        known,
        105_000,
        Some((100_000, 100_000)),
        Some((100_000, 100_000)),
        tolerance,
    );
    let (truth, known) = fold(&mut actual, &candle(30_006, 0, 105_000, 105_000, 105_000));
    assert_answers(
        truth,
        known,
        105_000,
        Some((112_345, 100_000)),
        Some((112_345, 100_000)),
        tolerance,
    );
}

#[test]
fn wrong_tolerance_family_is_unknown_but_zero_width_has_exact_known_answers() {
    let wrong = vocab::tolerance::pinned_pivot().expect("valid wrong-family width");
    let mut actual = seeded(wrong, 112_345, 100_000, 105_000);
    let (truth, known) = fold(&mut actual, &candle(30_005, 0, 105_000, 105_000, 105_000));
    assert_answers(
        truth,
        known,
        105_000,
        Some((112_345, 100_000)),
        Some((112_345, 100_000)),
        wrong,
    );

    let zero = Tolerance::from_milli_on(Base::SessionRange, 0).expect("zero width is valid");
    let seed = seeded(zero, 112_345, 100_000, 105_000);
    for (_, numerator, down, _) in RUNGS {
        let center = level(112_345, 100_000, numerator, down).expect("fixture rung fits");
        for close in [center, center + 1] {
            let mut actual = seed;
            let (truth, known) = fold(&mut actual, &candle(30_005, 0, close, close, close));
            assert_answers(
                truth,
                known,
                close,
                Some((112_345, 100_000)),
                Some((112_345, 100_000)),
                zero,
            );
        }
    }
}

#[test]
fn unrepresentable_extension_is_unknown_without_hiding_representable_sibling_rungs() {
    let range = 2_000_000_000_000_000_000;
    let low = i64::MAX - 4236 * (range / 1000) + 1;
    let high = low + range;
    let mut actual = seeded(pinned(), high, low, low);
    let (truth, known) = fold(&mut actual, &candle(30_005, 0, low, low, low));
    assert_answers(
        truth,
        known,
        low,
        Some((high, low)),
        Some((high, low)),
        pinned(),
    );
    assert!(!known.get(71), "4.236 rung exceeds i64 by one paisa");
    assert!(known.get(109), "2.618 sibling still has a checked level");
}

#[test]
fn unusable_daily_reference_clears_previous_day_without_falsifying_five_session_ladder() {
    let mut actual = seeded(pinned(), 112_345, 100_000, 105_000);
    fold(&mut actual, &candle(30_004, 1, 1, 1, 1));
    fold(
        &mut actual,
        &candle(30_004, 2, i64::MAX, i64::MAX, i64::MAX),
    );
    let (truth, known) = fold(&mut actual, &candle(30_005, 0, 105_000, 105_000, 105_000));
    assert!(
        !actual.has_yesterday(),
        "overflowing daily pivot ladder is unusable"
    );
    assert_answers(truth, known, 105_000, None, Some((i64::MAX, 1)), pinned());
    assert_eq!(
        RUNGS
            .into_iter()
            .filter(|(bit, _, _, five)| *five && known.get(u32::from(*bit)))
            .count(),
        7
    );
}

#[test]
fn nonregular_session_is_not_substituted_for_completed_regular_references() {
    let mut actual = evaluator(pinned(), Calendar::charter());
    for day in 18_676..18_682 {
        fold(&mut actual, &candle(day, 0, 112_345, 100_000, 105_000));
    }
    assert!(Calendar::charter().is_non_regular(18_682));
    fold(&mut actual, &candle(18_682, 0, 8_000_000, 1, 2_000_000));
    let (truth, known) = fold(&mut actual, &candle(18_683, 0, 105_000, 105_000, 105_000));
    assert_answers(
        truth,
        known,
        105_000,
        Some((112_345, 100_000)),
        Some((112_345, 100_000)),
        pinned(),
    );
}

#[test]
fn public_column_retains_known_false_fibonacci_answers_for_negated_search() {
    let bars: Vec<_> = (30_000..30_006)
        .flat_map(|day| (0..75).map(move |minute| candle(day, minute, 112_345, 100_000, 105_000)))
        .collect();
    let mut actual = evaluator(pinned(), Calendar::all_regular());
    let column = Column::build(&bars, &mut actual);
    assert!(column.census().reconciles());
    assert_eq!(column.census().refused(), 0);
    assert!(
        !column.is_empty(),
        "warm input reaches the actual search column"
    );
    assert_eq!(
        column.first_swept(),
        Some(376),
        "column requires pre-fold warmth, so the first bar completing the five-session window remains outside the swept column"
    );
    let mut verified = 0;
    for ((source, truth), known) in column
        .sources()
        .iter()
        .zip(column.bits())
        .zip(column.known())
    {
        if *source >= 375 {
            assert_answers(
                *truth,
                *known,
                105_000,
                Some((112_345, 100_000)),
                Some((112_345, 100_000)),
                pinned(),
            );
            assert_eq!(
                Expression::parse("!71")
                    .expect("valid false rung")
                    .evaluate(*truth, *known),
                Truth::True
            );
            verified += 1;
        }
    }
    assert_eq!(
        verified, 74,
        "every admitted bar after the fifth completed session is checked"
    );
}
