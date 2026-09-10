//! Independent finite generated-fixture evidence for gap truth, NOT and projection.
#![allow(
    clippy::expect_used,
    reason = "fixtures must fail with their broken premise"
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
const RUNGS: [i128; 11] = [0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618];

fn bar(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Candle {
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

fn day_of(bar: &Candle) -> i64 {
    (bar.ts_micros + 330 * MINUTE).div_euclid(DAY)
}

fn evaluator(tolerance: Tolerance, calendar: Calendar) -> Evaluator {
    Evaluator::with_calendar(
        Widths {
            fib: tolerance,
            pivot: vocab::tolerance::pinned_pivot().expect("pivot"),
        },
        Availability::Absent,
        Thresholds::CLASSICAL,
        calendar,
    )
}

// Deliberately scans fixture history. No GapFib accessor, production level
// formula, tolerance predicate or vocabulary setter participates in this oracle.
fn expected(
    prior: &[Candle],
    current: &Candle,
    tolerance: Tolerance,
    calendar: Calendar,
) -> [Truth; 11] {
    let today = day_of(current);
    let first: Vec<_> = prior
        .iter()
        .filter(|bar| day_of(bar) == today)
        .take(3)
        .collect();
    if first.len() < 3 || tolerance.base() != Some(Base::SessionRange) {
        return [Truth::Unknown; 11];
    }
    let Some(yesterday) = prior
        .iter()
        .map(day_of)
        .filter(|day| *day < today && !calendar.is_non_regular(*day))
        .max()
    else {
        return [Truth::Unknown; 11];
    };
    let last: Vec<_> = prior
        .iter()
        .rev()
        .filter(|bar| day_of(bar) == yesterday)
        .take(3)
        .collect();
    let high = first
        .iter()
        .map(|bar| bar.high)
        .max()
        .expect("three opening bars");
    let low = first
        .iter()
        .map(|bar| bar.low)
        .min()
        .expect("three opening bars");
    let y_high = last
        .iter()
        .map(|bar| bar.high)
        .max()
        .expect("previous observed session");
    let y_low = last
        .iter()
        .map(|bar| bar.low)
        .min()
        .expect("previous observed session");
    let (x1, x2) = match (high > y_high, low < y_low) {
        (true, false) => (i128::from(y_high), i128::from(high)),
        (false, true) => (i128::from(y_low), i128::from(low)),
        _ => return [Truth::Unknown; 11],
    };
    let length = (x1 - x2).abs();
    if length == 0 || length > i128::from(i64::MAX) {
        return [Truth::Unknown; 11];
    }
    RUNGS.map(|rung| {
        let level = x2 + (rung * (x1 - x2)).div_euclid(1000);
        if i64::try_from(level).is_err() {
            Truth::Unknown
        } else if (i128::from(current.close) - level).abs() * 1000
            <= i128::from(tolerance.milli()) * length
        {
            Truth::True
        } else {
            Truth::False
        }
    })
}

fn compare(truth: ConditionMask, known: ConditionMask, answers: [Truth; 11]) {
    for (bit, answer) in (132_u16..=142).zip(answers) {
        assert_eq!(
            truth.get(u32::from(bit)),
            answer == Truth::True,
            "truth {bit}"
        );
        assert_eq!(
            known.get(u32::from(bit)),
            answer != Truth::Unknown,
            "known {bit}"
        );
        let opposite = match answer {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        };
        assert_eq!(
            Expression::parse(&format!("!{bit}"))
                .expect("live gap bit")
                .evaluate(truth, known),
            opposite,
            "NOT {bit}"
        );
    }
}

fn verify(bars: &[Candle], tolerance: Tolerance, calendar: Calendar) -> [[usize; 3]; 11] {
    let mut state = evaluator(tolerance, calendar);
    let mut legacy = evaluator(tolerance, calendar);
    let mut counts = [[0; 3]; 11];
    for (index, current) in bars.iter().enumerate() {
        let answers = expected(
            bars.get(..index).expect("prior only"),
            current,
            tolerance,
            calendar,
        );
        let (truth, known) = state.step_known(current).expect("valid generated candle");
        assert_eq!(truth, legacy.step(current).expect("unchanged truth path"));
        compare(truth, known, answers);
        for (counts, answer) in counts.iter_mut().zip(answers) {
            let position = match answer {
                Truth::False => 0,
                Truth::True => 1,
                Truth::Unknown => 2,
            };
            *counts.get_mut(position).expect("three states") += 1;
        }
    }
    counts
}

#[test]
fn all_eleven_gap_predicates_and_negations_match_independent_pre_fold_levels() {
    let tolerance = vocab::tolerance::pinned_fib().expect("fib");
    for (old, opening) in [(100_000, 120_000), (120_000, 100_000)] {
        let mut bars = Vec::new();
        for minute in 0..3 {
            bars.push(bar(30_000, minute, old, old, old));
        }
        for minute in 0..3 {
            bars.push(bar(30_001, minute, opening, opening, opening));
        }
        for (offset, rung) in RUNGS.into_iter().enumerate() {
            let price = i64::try_from(
                i128::from(opening) + (rung * i128::from(old - opening)).div_euclid(1000),
            )
            .expect("ordinary positive levels");
            bars.push(bar(
                30_001,
                3 + i64::try_from(offset).expect("eleven"),
                price,
                price,
                price,
            ));
        }
        bars.push(bar(30_002, 0, 100_000, 100_000, 100_000));
        for [false_count, true_count, unknown_count] in
            verify(&bars, tolerance, Calendar::all_regular())
        {
            assert!(false_count > 0 && true_count > 0);
            assert_eq!(
                unknown_count, 7,
                "first three opening bars and session reset stay unavailable"
            );
        }
    }
}

#[test]
fn absent_ambiguous_and_unrepresentable_gap_references_never_satisfy_not() {
    let mut ordinary = Vec::new();
    for minute in 0..3 {
        ordinary.push(bar(30_000, minute, 110_000, 90_000, 100_000));
    }
    for (high, low) in [(110_000, 90_000), (120_000, 80_000), (109_000, 91_000)] {
        let mut bars = ordinary.clone();
        for minute in 0..5 {
            bars.push(bar(30_001, minute, high, low, 100_000));
        }
        for [f, t, u] in verify(
            &bars,
            vocab::tolerance::pinned_fib().expect("fib"),
            Calendar::all_regular(),
        ) {
            assert_eq!((f, t, u), (0, 0, bars.len()));
        }
    }
    let mut huge = Vec::new();
    for minute in 0..3 {
        huge.push(bar(
            30_000,
            minute,
            i64::MAX - 10,
            i64::MAX - 10,
            i64::MAX - 10,
        ));
    }
    for minute in 0..5 {
        huge.push(bar(30_001, minute, 10, 10, 10));
    }
    let counts = verify(
        &huge,
        vocab::tolerance::pinned_fib().expect("fib"),
        Calendar::all_regular(),
    );
    assert!(counts.iter().any(|[_, _, unknown]| *unknown == huge.len()));
    assert!(counts.iter().any(|[f, t, _]| f + t > 0));
    for wrong in [
        Tolerance::from_milli(10).expect("baseless"),
        vocab::tolerance::pinned_pivot().expect("wrong base"),
    ] {
        for [f, t, u] in verify(&huge, wrong, Calendar::all_regular()) {
            assert_eq!((f, t, u), (0, 0, huge.len()));
        }
    }
    for milli in [0, i64::MAX] {
        verify(
            &huge,
            Tolerance::from_milli_on(Base::SessionRange, milli).expect("finite tolerance"),
            Calendar::all_regular(),
        );
    }
}

#[test]
fn non_regular_session_cannot_replace_the_prior_regular_gap_anchor() {
    let mut bars = Vec::new();
    for (day, price) in [(20_027, 100_000), (20_028, 300_000), (20_029, 120_000)] {
        for minute in 0..5 {
            bars.push(bar(day, minute, price, price, price));
        }
    }
    let counts = verify(
        &bars,
        vocab::tolerance::pinned_fib().expect("fib"),
        Calendar::charter(),
    );
    assert!(counts.iter().all(|[f, t, _]| f + t == 4));
}

#[test]
fn exact_minute_overlay_preserves_known_false_and_is_transactional_on_missing_evidence() {
    let widths = Widths::pinned().expect("widths");
    let calendar = Calendar::all_regular();
    let mut minutes = Vec::new();
    let mut signal = Vec::new();
    for day in 30_000..30_009 {
        let mut today = Vec::new();
        for minute in 0..150 {
            let price = 100_000 + (day - 30_000) * 1000 + minute;
            today.push(bar(day, minute, price, price, price));
        }
        for chunk in today.chunks_exact(5) {
            let first = chunk.first().expect("five minutes");
            let last = chunk.last().expect("five minutes");
            signal.push(Candle::new(
                first.ts_micros,
                first.open,
                last.high,
                first.low,
                last.close,
                0,
                i64::MIN,
            ));
        }
        minutes.extend(today);
    }
    let mut column = Column::build(&signal, &mut evaluator(widths.fib, calendar));
    assert!(!column.is_empty(), "fixture must warm the actual column");
    let before = column.clone();
    indicators::anchored::overlay_exact_minute_gapfib(
        &signal,
        &minutes,
        5 * MINUTE,
        widths,
        calendar,
        &mut column,
    )
    .expect("exact close alignment");
    let mut known_false = 0;
    for ((source, truth), known) in column
        .sources()
        .iter()
        .zip(column.bits())
        .zip(column.known())
    {
        let candle = signal.get(*source).expect("signal source");
        let minute_index = minutes
            .iter()
            .position(|bar| bar.ts_micros == candle.ts_micros + 4 * MINUTE)
            .expect("actual close");
        let exact = minutes.get(minute_index).expect("exact minute");
        let answers = expected(
            minutes.get(..minute_index).expect("prior exact context"),
            exact,
            widths.fib,
            calendar,
        );
        known_false += answers
            .iter()
            .filter(|answer| **answer == Truth::False)
            .count();
        compare(*truth, *known, answers);
    }
    assert!(
        known_false > 0,
        "the overlay must preserve actual false answers"
    );
    for ((old_truth, old_known), (new_truth, new_known)) in before
        .bits()
        .iter()
        .zip(before.known())
        .zip(column.bits().iter().zip(column.known()))
    {
        for bit in (0..132).chain(143..384) {
            assert_eq!(old_truth.get(bit), new_truth.get(bit), "other truth {bit}");
            assert_eq!(
                old_known.get(bit),
                new_known.get(bit),
                "other availability {bit}"
            );
        }
    }
    let first_source = *column.sources().first().expect("warmed source");
    let missing_ts = signal.get(first_source).expect("source").ts_micros + 4 * MINUTE;
    minutes.retain(|bar| bar.ts_micros != missing_ts);
    let intact = column.clone();
    assert!(
        indicators::anchored::overlay_exact_minute_gapfib(
            &signal,
            &minutes,
            5 * MINUTE,
            widths,
            calendar,
            &mut column
        )
        .is_err()
    );
    assert_eq!(
        column, intact,
        "no partial truth or availability overlay on failure"
    );
}
