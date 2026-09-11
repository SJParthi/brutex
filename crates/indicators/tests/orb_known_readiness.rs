//! Finite generated-fixture oracles for opening-range truth and negation.

#![allow(
    clippy::expect_used,
    reason = "fixture failures name the broken premise"
)]

use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use indicators::{Candle, Corrupt};
use vocab::expression::{Expression, Truth};
use vocab::tolerance::Base;
use vocab::{ConditionMask, Tolerance};

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
const WINDOWS: [i64; 4] = [5, 15, 30, 60];

#[derive(Clone, Copy)]
struct Sample {
    day: i64,
    minute: i64,
    candle: Candle,
}

fn sample(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Sample {
    Sample {
        day,
        minute,
        candle: Candle::new(
            day * DAY + (225 + minute) * MINUTE,
            close,
            high,
            low,
            close,
            0,
            i64::MIN,
        ),
    }
}

fn evaluator(fib: Tolerance) -> Evaluator {
    Evaluator::with_calendar(
        Widths {
            fib,
            pivot: vocab::tolerance::pinned_pivot().expect("pinned pivot tolerance"),
        },
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

fn pinned() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("pinned opening-range tolerance")
}

/// This oracle scans prior fixture records; it does not call Orb, its window
/// accessors, the vocabulary setters or `Tolerance::covers`.
fn expected(prior: &[Sample], current: Sample, window: i64, tolerance: Tolerance) -> [Truth; 5] {
    if current.minute < window {
        return [Truth::Unknown; 5];
    }
    let mut bars = prior
        .iter()
        .filter(|row| row.day == current.day && row.minute >= 0 && row.minute < window);
    let Some(first) = bars.next() else {
        return [Truth::Unknown; 5];
    };
    let (high, low) = bars.fold((first.candle.high, first.candle.low), |(high, low), row| {
        (high.max(row.candle.high), low.min(row.candle.low))
    });
    let range = i128::from(high) - i128::from(low);
    if range > i128::from(i64::MAX) {
        return [Truth::Unknown; 5];
    }
    let close = current.candle.close;
    let truth = |value| if value { Truth::True } else { Truth::False };
    let near = |level| {
        if range == 0 || tolerance.base() != Some(Base::SessionRange) {
            Truth::Unknown
        } else {
            truth(
                (i128::from(close) - i128::from(level)).abs() * 1000
                    <= i128::from(tolerance.milli()) * range,
            )
        }
    };
    [
        truth(close > high),
        truth(close < low),
        truth(low <= close && close <= high),
        near(high),
        near(low),
    ]
}

fn assert_answers(
    truth: ConditionMask,
    known: ConditionMask,
    expected: Truth,
    bit: u16,
    negation: &Expression,
) {
    assert_eq!(
        known.get(u32::from(bit)),
        expected != Truth::Unknown,
        "known bit {bit}"
    );
    assert_eq!(
        truth.get(u32::from(bit)),
        expected == Truth::True,
        "truth bit {bit}"
    );
    let opposite = match expected {
        Truth::False => Truth::True,
        Truth::True => Truth::False,
        Truth::Unknown => Truth::Unknown,
    };
    assert_eq!(negation.evaluate(truth, known), opposite, "NOT bit {bit}");
}

fn verify(samples: &[Sample], tolerance: Tolerance) -> [[usize; 3]; 20] {
    let mut actual = evaluator(tolerance);
    let mut legacy = evaluator(tolerance);
    let negations: Vec<_> = (86..106)
        .map(|bit| Expression::parse(&format!("!{bit}")).expect("every ORB predicate is live"))
        .collect();
    let mut counts = [[0; 3]; 20];
    for (index, &current) in samples.iter().enumerate() {
        let (truth, known) = actual
            .step_known(&current.candle)
            .expect("valid fixture fold");
        assert_eq!(
            legacy.step(&current.candle).expect("truth-only fold"),
            truth
        );
        assert_eq!(truth.union(&known), known, "every emitted truth is known");
        let expected = WINDOWS.into_iter().flat_map(|window| {
            expected(
                samples.get(..index).expect("prior prefix"),
                current,
                window,
                tolerance,
            )
        });
        for (((bit, negation), expected), count) in
            (86..106).zip(&negations).zip(expected).zip(&mut counts)
        {
            assert_answers(truth, known, expected, bit, negation);
            let [false_count, true_count, unknown_count] = count;
            match expected {
                Truth::False => *false_count += 1,
                Truth::True => *true_count += 1,
                Truth::Unknown => *unknown_count += 1,
            }
        }
    }
    counts
}

#[test]
fn all_twenty_relations_and_not_match_independent_frozen_range_comparisons() {
    let mut samples = vec![
        sample(30_000, 0, 10_000, 9_000, 9_500),
        sample(30_000, 5, 11_000, 8_000, 10_500),
        sample(30_000, 15, 12_000, 7_000, 7_500),
        sample(30_000, 30, 13_000, 6_000, 12_500),
        // This closing bar's much wider range must not alter any frozen anchor.
        sample(30_000, 60, 20_000, 1, 14_000),
    ];
    let mut minute = 61;
    for (high, low) in [
        (10_000, 9_000),
        (11_000, 8_000),
        (12_000, 7_000),
        (13_000, 6_000),
    ] {
        let band = (high - low) / 100;
        for level in [high, low] {
            for delta in [-band - 1, -band, -band + 1, 0, band - 1, band, band + 1] {
                let close = level + delta;
                samples.push(sample(30_000, minute, close, close, close));
                minute += 1;
            }
        }
    }
    let counts = verify(&samples, pinned());
    assert_eq!(samples.len(), 61);
    for (bit, [false_count, true_count, unknown_count]) in (86..106).zip(counts) {
        assert!(
            false_count > 0 && true_count > 0 && unknown_count > 0,
            "bit {bit} must exercise each truth state"
        );
    }
}

#[test]
fn first_closing_bar_holes_preopen_and_missing_windows_follow_actual_session_evidence() {
    let samples = [
        sample(30_000, -1, 99_999, 1, 100),
        sample(30_000, 7, 110, 90, 100),
        sample(30_000, 14, 120, 80, 100),
        sample(30_000, 15, 130, 70, 125),
        sample(30_000, 30, 140, 60, 65),
        sample(30_000, 60, 150, 50, 145),
        // All four windows elapsed without a single current-session print.
        sample(30_001, 60, 150, 50, 100),
        sample(30_002, 0, 110, 90, 100),
        sample(30_002, 4, 120, 80, 100),
        sample(30_002, 5, 999, 1, 130),
    ];
    let counts = verify(&samples, pinned());
    let [false_count, true_count, unknown_count] = counts.first().copied().expect("orb5 above");
    assert_eq!((false_count, true_count, unknown_count), (0, 1, 9));
}

#[test]
fn zero_span_and_tolerance_family_only_withhold_the_near_answers() {
    let samples = [
        sample(30_000, 0, 1_100, 900, 1_000),
        sample(30_000, 60, 1_102, 1_102, 1_102),
        sample(30_000, 61, 1_000, 1_000, 1_000),
        sample(30_001, 0, 1_000, 1_000, 1_000),
        sample(30_001, 60, 1_000, 1_000, 1_000),
    ];
    for tolerance in [
        pinned(),
        Tolerance::from_milli_on(Base::SessionRange, 0).expect("exact equality tolerance"),
        Tolerance::from_milli_on(Base::SessionRange, i64::MAX).expect("large finite tolerance"),
        vocab::tolerance::pinned_pivot().expect("wrong family"),
        Tolerance::from_milli(10).expect("baseless tolerance"),
    ] {
        let counts = verify(&samples, tolerance);
        for (offset, [false_count, true_count, unknown_count]) in counts.into_iter().enumerate() {
            if offset % 5 < 3 {
                assert_eq!(
                    unknown_count, 2,
                    "exact comparisons only await window closure"
                );
                assert_eq!(false_count + true_count, 3);
            } else if tolerance.base() == Some(Base::SessionRange) {
                assert_eq!(unknown_count, 3, "zero span has no near scale");
                assert_eq!(false_count + true_count, 2);
            } else {
                assert_eq!((false_count, true_count, unknown_count), (0, 0, 5));
            }
        }
    }
}

#[test]
fn representable_positive_extremes_keep_integer_comparisons_and_wide_bands_exact() {
    let samples = [
        sample(30_000, 0, i64::MAX, i64::MAX, i64::MAX),
        sample(30_000, 4, 1, 1, 1),
        sample(30_000, 60, 1, 1, 1),
        sample(30_000, 61, i64::MAX, i64::MAX, i64::MAX),
        sample(30_000, 62, i64::MAX / 2, i64::MAX / 2, i64::MAX / 2),
    ];
    for tolerance in [
        pinned(),
        Tolerance::from_milli_on(Base::SessionRange, i64::MAX).expect("large finite tolerance"),
    ] {
        let counts = verify(&samples, tolerance);
        for [false_count, true_count, unknown_count] in counts {
            assert_eq!(unknown_count, 2);
            assert_eq!(false_count + true_count, 3);
        }
    }
}

#[test]
fn a_refused_bar_cannot_change_opening_range_truth_or_availability() {
    for (bad, refusal) in [
        (
            sample(30_000, 60, i64::MAX, i64::MIN, 1),
            Corrupt::RangeOverflows,
        ),
        (sample(30_000, 60, 0, 0, 0), Corrupt::PriceNotPositive),
        (
            sample(30_000, 0, 110, 90, 100),
            Corrupt::TimestampNotIncreasing,
        ),
    ] {
        let mut actual = evaluator(pinned());
        let mut control = evaluator(pinned());
        let first = sample(30_000, 0, 110, 90, 100);
        assert_eq!(
            actual.step_known(&first.candle),
            control.step_known(&first.candle)
        );
        assert_eq!(actual.step_known(&bad.candle), Err(refusal));
        for minute in [5, 15, 30, 60, 61] {
            let candle = sample(30_000, minute, 200, 50, 150).candle;
            assert_eq!(actual.step_known(&candle), control.step_known(&candle));
        }
    }
}

#[test]
fn anchored_columns_preserve_known_false_opening_range_answers() {
    use indicators::anchored::{AnchoredEvaluator, DailyEligibility, DailyReference};
    use indicators::column::AnchoredColumn;

    let references: Vec<_> = (29_995..30_000)
        .map(|day| {
            DailyReference::new(
                sample(day, 0, 200, 100, 150).candle,
                DailyEligibility::Eligible,
            )
            .expect("eligible generated daily reference")
        })
        .collect();
    let bars: Vec<_> = (0..250)
        .map(|minute| sample(30_000, minute, 200, 100, 150).candle)
        .collect();
    let mut evaluator = AnchoredEvaluator::new(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        &references,
    )
    .expect("ordered prior references");
    let anchored = AnchoredColumn::build_required(&bars, &mut evaluator)
        .expect("anchored column with enough trend history");
    let column = anchored.column();
    assert_eq!(column.len(), 50);
    for ((truth, known), source) in column
        .bits()
        .iter()
        .zip(column.known())
        .zip(column.sources())
    {
        assert!(*source >= 200, "only warm emitted rows are retained");
        for bit in 86..106 {
            let expected = if (bit - 86) % 5 == 2 {
                Truth::True
            } else {
                Truth::False
            };
            let negation = Expression::parse(&format!("!{bit}")).expect("live ORB bit");
            assert_answers(*truth, *known, expected, bit, &negation);
        }
    }
}
