//! Generated fixtures for the existing four clock-window conventions. These
//! checks do not establish exchange-session admission or validate that policy.

#![allow(
    clippy::expect_used,
    reason = "fixture failures must name their premise"
)]

use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use indicators::{Candle, Corrupt};
use vocab::ConditionMask;
use vocab::expression::{Expression, Truth};

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
const IST_OFFSET: i64 = 330 * MINUTE;
// Independent absolute IST clock boundaries for the shipped convention:
// 09:15–10:15, 10:15–12:00, 12:00–14:00, 14:00–15:30.
const WINDOWS: [(u32, i64, i64); 4] = [
    (44, 555, 615),
    (45, 615, 720),
    (46, 720, 840),
    (47, 840, 930),
];

fn evaluator() -> Evaluator {
    Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

fn bar(day: i64, ist_minute: i64) -> Candle {
    Candle::new(
        day * DAY + ist_minute * MINUTE - IST_OFFSET,
        100,
        102,
        99,
        101,
        0,
        i64::MIN,
    )
}

fn nots() -> [Expression; 4] {
    WINDOWS.map(|(bit, _, _)| Expression::parse(&format!("!{bit}")).expect("live clock predicate"))
}

fn assert_clock(
    truth: ConditionMask,
    known: ConditionMask,
    ist_minute: i64,
    negations: &[Expression; 4],
) {
    for ((bit, start, end), negation) in WINDOWS.into_iter().zip(negations) {
        let within = (start..end).contains(&ist_minute);
        assert_eq!(
            truth.get(bit),
            within,
            "clock{bit} at IST minute{ist_minute}"
        );
        assert!(
            known.get(bit),
            "an accepted timestamp supplies clock{bit}, even when false"
        );
        assert_eq!(
            negation.evaluate(truth, known),
            if within { Truth::False } else { Truth::True },
            "NOT clock{bit} at IST minute{ist_minute}",
        );
    }
}

#[test]
fn all_1440_minutes_on_two_days_preserve_half_open_truth_and_make_every_false_clock_known() {
    let mut actual = evaluator();
    let negations = nots();
    let mut counts = [0_u64; 4];
    for day in [30_000, 30_001] {
        for minute in 0..1440 {
            let (truth, known) = actual
                .step_known(&bar(day, minute))
                .expect("valid chronological fixture");
            assert_clock(truth, known, minute, &negations);
            for ((bit, _, _), count) in WINDOWS.into_iter().zip(&mut counts) {
                *count += u64::from(truth.get(bit));
            }
        }
    }
    assert_eq!(
        counts,
        [120, 210, 240, 180],
        "exactly twice each original window width"
    );
}

#[test]
fn outside_the_declared_windows_no_positive_clock_is_substituted_and_rollover_needs_no_history() {
    let mut actual = evaluator();
    let negations = nots();
    // The low-level evaluator already accepts these timestamps. No new
    // OutsideSession refusal or market calendar policy is introduced here.
    for (day, minute) in [(30_000, 554), (30_000, 930), (30_000, 1019), (30_001, 555)] {
        let (truth, known) = actual
            .step_known(&bar(day, minute))
            .expect("accepted timestamp under existing low-level contract");
        assert_clock(truth, known, minute, &negations);
        let positive_count = WINDOWS
            .into_iter()
            .filter(|(bit, _, _)| truth.get(*bit))
            .count();
        assert_eq!(positive_count, usize::from(minute == 555));
    }
}

#[test]
fn clock_negation_composes_with_and_or_while_an_unseeded_price_predicate_stays_unknown() {
    let checks: Vec<_> = [
        ("!44 & 30", Truth::False, Truth::True),
        ("!44 | 31", Truth::False, Truth::True),
        ("!44 & 2", Truth::False, Truth::Unknown),
        ("!44 | 2", Truth::Unknown, Truth::True),
        ("!44 | !2", Truth::Unknown, Truth::True),
        ("44 | !44", Truth::True, Truth::True),
        ("44 & !44", Truth::False, Truth::False),
    ]
    .into_iter()
    .map(|(source, before, after)| {
        (
            Expression::parse(source).expect("valid expression"),
            before,
            after,
        )
    })
    .collect();
    let mut actual = evaluator();
    for (minute, before_boundary) in [(614, true), (615, false)] {
        let (truth, known) = actual
            .step_known(&bar(30_000, minute))
            .expect("valid boundary bar");
        assert!(!known.get(2), "two candles cannot establish EMA200");
        assert_eq!(
            Expression::parse("!2")
                .expect("valid missing-reference NOT")
                .evaluate(truth, known),
            Truth::Unknown
        );
        for (expression, before, after) in &checks {
            assert_eq!(
                expression.evaluate(truth, known),
                if before_boundary { *before } else { *after },
                "{expression} at IST minute{minute}"
            );
        }
    }
}

#[test]
fn actual_search_column_keeps_the_315_not_early_morning_signals() {
    let mut actual = evaluator();
    for day in 30_000..30_006 {
        for minute in 555..595 {
            actual
                .step_known(&bar(day, minute))
                .expect("valid warm-up history");
        }
    }
    assert!(actual.warmed_up());
    let bars: Vec<_> = (555..930).map(|minute| bar(30_006, minute)).collect();
    let column = Column::build(&bars, &mut actual);
    assert_eq!(column.len(), 375);
    assert_eq!(column.census().refused(), 0);
    let negations = nots();
    let expression = Expression::parse("30 & !44").expect("not-early-morning bullish expression");
    let mut signals = 0;
    for ((source, truth), known) in column
        .sources()
        .iter()
        .zip(column.bits())
        .zip(column.known())
    {
        assert_clock(
            *truth,
            *known,
            555 + i64::try_from(*source).expect("bounded index"),
            &negations,
        );
        if expression.evaluate(*truth, *known) == Truth::True {
            signals += 1;
        }
    }
    assert_eq!(
        signals, 315,
        "every accepted minute after10:15 must remain eligible for the expression"
    );
}

#[test]
fn a_refused_boundary_candle_cannot_advance_or_publish_clock_state() {
    let mut actual = evaluator();
    actual
        .step_known(&bar(30_000, 614))
        .expect("last early-morning minute");
    let mut untouched = actual;
    let mut broken = bar(30_000, 615);
    broken.high = 98;
    assert_eq!(actual.step_known(&broken), Err(Corrupt::HighBelowLow));
    assert_eq!(
        actual.step_known(&bar(30_000, 614)),
        Err(Corrupt::TimestampNotIncreasing)
    );
    for (day, minute) in [(30_000, 615), (30_000, 930), (30_001, 555)] {
        let next = bar(day, minute);
        let actual_result = actual
            .step_known(&next)
            .expect("valid continuation after refusals");
        assert_eq!(
            actual_result,
            untouched
                .step_known(&next)
                .expect("same untouched continuation")
        );
        assert_clock(actual_result.0, actual_result.1, minute, &nots());
    }
}
