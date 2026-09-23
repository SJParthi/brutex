//! Generated OHLCV fixtures proving exact-minute opening ranges at coarse closes.

#![allow(clippy::expect_used, reason = "a broken fixture must fail explicitly")]

use indicators::Candle;
use indicators::anchored::{
    ExactMinuteGapRefusal, overlay_exact_minute_gapfib, overlay_exact_minute_orb_and_gapfib,
};
use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use vocab::ConditionMask;
use vocab::expression::{Expression, Truth};

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
const FIRST_DAY: i64 = 30_006;
const WINDOWS: [i64; 4] = [5, 15, 30, 60];

fn stamp(day: i64, minute: i64) -> i64 {
    day * DAY + (225 + minute) * MINUTE
}

fn bar(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle::new(stamp(day, minute), close, high, low, close, 0, i64::MIN)
}

fn widths() -> Widths {
    Widths::pinned().expect("pinned widths")
}

fn minutes(day: i64, first: i64) -> Vec<Candle> {
    (first..375)
        .map(|minute| {
            let (high, low, close) = match minute {
                0..=4 => (110, 90, 100),
                // These spikes are OUTSIDE ORB5, ORB15, ORB30 and ORB60,
                // respectively. A coarse bucket must not fold them early.
                5 => (1_000, 90, 100),
                15 => (2_000, 90, 100),
                30 => (3_000, 90, 100),
                60 => (4_000, 90, 100),
                9..=10 => (110, 110, 110),
                11..=13 | 21..=23 | 42..=44 | 81..=83 => (90, 90, 90),
                19..=20 => (1_000, 1_000, 1_000),
                39..=41 => (2_000, 2_000, 2_000),
                79..=80 => (3_000, 3_000, 3_000),
                16..=18 | 24..=29 => (1_100, 1_100, 1_100),
                31..=38 | 45..=59 => (2_100, 2_100, 2_100),
                100..=104 => (3_500, 3_500, 3_500),
                // The true session-final close deliberately differs from its
                // neighbour, making a wrong stub alignment observable.
                374 => (50, 50, 50),
                _ => (150, 150, 150),
            };
            bar(day, minute, high, low, close)
        })
        .collect()
}

fn aggregate(minute: &[Candle], rung: usize) -> Vec<Candle> {
    minute
        .chunks(rung)
        .map(|bucket| {
            let first = bucket.first().expect("nonempty bucket");
            let last = bucket.last().expect("nonempty bucket");
            let (high, low) = bucket
                .iter()
                .fold((first.high, first.low), |(high, low), row| {
                    (high.max(row.high), low.min(row.low))
                });
            Candle::new(
                first.ts_micros,
                first.open,
                high,
                low,
                last.close,
                0,
                i64::MIN,
            )
        })
        .collect()
}

fn column(signal: &[Candle]) -> Column {
    let mut evaluator = Evaluator::with_calendar(
        widths(),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    for day in 30_000..FIRST_DAY {
        for minute in 0..40 {
            evaluator
                .step_known(&bar(day, minute, 110, 90, 100))
                .expect("valid warm-up bar");
        }
    }
    assert!(evaluator.warmed_up());
    let column = Column::build(signal, &mut evaluator);
    assert_eq!(
        column.len(),
        signal.len(),
        "every coarse fixture bar must reach the bridge"
    );
    assert_eq!(column.census().refused(), 0);
    column
}

/// This oracle scans the real fixture's minute timestamps within each window.
/// It never calls Orb, its accessors, `known()`, or vocabulary near setters.
fn expected(exact: &[Candle], day: i64, final_minute: i64, close: i64, window: i64) -> [Truth; 5] {
    if final_minute < window {
        return [Truth::Unknown; 5];
    }
    let mut input = exact
        .iter()
        .filter(|row| row.ts_micros >= stamp(day, 0) && row.ts_micros < stamp(day, window));
    let Some(first) = input.next() else {
        return [Truth::Unknown; 5];
    };
    let (high, low) = input.fold((first.high, first.low), |(high, low), row| {
        (high.max(row.high), low.min(row.low))
    });
    let truth = |value| if value { Truth::True } else { Truth::False };
    let range = i128::from(high) - i128::from(low);
    let near = |level| {
        if range == 0 {
            Truth::Unknown
        } else {
            truth(
                (i128::from(close) - i128::from(level)).abs() * 1000
                    <= i128::from(widths().fib.milli()) * range,
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

fn assert_oracle(
    signal: &[Candle],
    exact: &[Candle],
    rung: i64,
    column: &Column,
) -> [[u64; 3]; 20] {
    let mut counts = [[0; 3]; 20];
    let negations: Vec<_> = (86..106)
        .map(|bit| Expression::parse(&format!("!{bit}")).expect("live ORB expression"))
        .collect();
    for ((source, truth), known) in column
        .sources()
        .iter()
        .zip(column.bits())
        .zip(column.known())
    {
        let signal = signal.get(*source).expect("actual source mapping");
        let day = signal.ts_micros / DAY;
        let opened = (signal.ts_micros - stamp(day, 0)) / MINUTE;
        let final_minute = (opened + rung - 1).min(374);
        let minute = exact
            .iter()
            .find(|row| row.ts_micros == stamp(day, final_minute))
            .expect("exact final minute");
        assert_eq!(minute.close, signal.close);
        let answers = WINDOWS
            .into_iter()
            .flat_map(|window| expected(exact, day, final_minute, signal.close, window));
        for (((bit, negation), answer), count) in
            (86_u32..106).zip(&negations).zip(answers).zip(&mut counts)
        {
            assert_eq!(
                truth.get(bit),
                answer == Truth::True,
                "truth{bit}, final minute{final_minute}, rung{rung}"
            );
            assert_eq!(
                known.get(bit),
                answer != Truth::Unknown,
                "known{bit}, final minute{final_minute}, rung{rung}"
            );
            let opposite = match answer {
                Truth::Unknown => Truth::Unknown,
                Truth::True => Truth::False,
                Truth::False => Truth::True,
            };
            assert_eq!(negation.evaluate(*truth, *known), opposite);
            let [trues, falses, unknowns] = count;
            match answer {
                Truth::True => *trues += 1,
                Truth::False => *falses += 1,
                Truth::Unknown => *unknowns += 1,
            }
        }
    }
    counts
}

fn overlay(signal: &[Candle], exact: &[Candle], rung: i64, column: &mut Column) {
    let census = overlay_exact_minute_orb_and_gapfib(
        signal,
        exact,
        rung * MINUTE,
        widths(),
        Calendar::all_regular(),
        column,
    )
    .expect("exact aligned minute evidence");
    assert_eq!(
        census.minute_bars,
        u64::try_from(exact.len()).expect("small fixture")
    );
    assert_eq!(
        census.signal_rows,
        u64::try_from(signal.len()).expect("small fixture")
    );
    assert_eq!(census.signal_rows, census.overlaid);
}

#[test]
fn all_eight_rungs_use_exact_window_minutes_including_sixty_and_straddling_two_and_three() {
    let exact = minutes(FIRST_DAY, 0);
    let mut totals = [[0_u64; 3]; 20];
    for rung in [1, 2, 3, 5, 10, 15, 30, 60] {
        let signal = aggregate(&exact, usize::try_from(rung).expect("small rung"));
        let original = column(&signal);
        let mut gap_only = original.clone();
        overlay_exact_minute_gapfib(
            &signal,
            &exact,
            rung * MINUTE,
            widths(),
            Calendar::all_regular(),
            &mut gap_only,
        )
        .expect("compatible GapFib-only bridge");
        let mut combined = original.clone();
        overlay(&signal, &exact, rung, &mut combined);
        for (counts, total) in assert_oracle(&signal, &exact, rung, &combined)
            .into_iter()
            .zip(&mut totals)
        {
            for (count, total) in counts.into_iter().zip(total) {
                *total += count;
            }
        }
        for ((((original_truth, original_known), gap_truth), gap_known), (truth, known)) in original
            .bits()
            .iter()
            .zip(original.known())
            .zip(gap_only.bits())
            .zip(gap_only.known())
            .zip(combined.bits().iter().zip(combined.known()))
        {
            for bit in 0..ConditionMask::BITS {
                if (86..106).contains(&bit) {
                    assert_eq!(
                        gap_truth.get(bit),
                        original_truth.get(bit),
                        "legacy wrapper must preserve ORB truth"
                    );
                    assert_eq!(
                        gap_known.get(bit),
                        original_known.get(bit),
                        "legacy wrapper must preserve ORB known"
                    );
                } else {
                    assert_eq!(
                        truth.get(bit),
                        gap_truth.get(bit),
                        "unrelated truth changed"
                    );
                    assert_eq!(
                        known.get(bit),
                        gap_known.get(bit),
                        "unrelated availability changed"
                    );
                }
            }
        }
        if matches!(rung, 2 | 3 | 60) {
            let opens = if rung == 60 { 60 } else { 6 };
            let row = signal
                .iter()
                .position(|bar| bar.ts_micros == stamp(FIRST_DAY, opens))
                .expect("straddling-window comparison row");
            assert!(
                !original.bits().get(row).expect("coarse row").get(86),
                "fixture exposes the old wrong ORB5 answer"
            );
            assert!(
                combined.bits().get(row).expect("exact row").get(86),
                "minute5 spike cannot enlarge ORB5"
            );
        }
        let last = combined.bits().last().expect("session-final row");
        assert!(
            last.get(87),
            "the real minute374 close50 is below ORB5, unlike minute373 close150"
        );
        assert_eq!(combined.sources(), original.sources());
    }
    for (bit, [trues, falses, unknowns]) in (86..106).zip(totals) {
        assert!(
            trues > 0 && falses > 0 && unknowns > 0,
            "every ORB bit{bit} must exercise true, false and unknown"
        );
    }
}

#[test]
fn new_session_without_opening_window_clears_stale_truth_and_known_including_absent_gap() {
    let first = minutes(FIRST_DAY, 0);
    let second = minutes(FIRST_DAY + 1, 6);
    let mut signal = aggregate(&first, 3);
    signal.extend(aggregate(&second, 3));
    let mut exact = first;
    exact.extend(second);
    let mut column = column(&signal);
    overlay(&signal, &exact, 3, &mut column);
    assert_oracle(&signal, &exact, 3, &column);
    let first_late = signal
        .iter()
        .position(|bar| bar.ts_micros == stamp(FIRST_DAY + 1, 6))
        .expect("late-session opening");
    let truth = *column.bits().get(first_late).expect("late row");
    let known = *column.known().get(first_late).expect("late availability");
    for bit in 86..91 {
        assert!(
            !truth.get(bit) && !known.get(bit),
            "no actual first-five-minute reference exists"
        );
        assert_eq!(
            Expression::parse(&format!("!{bit}"))
                .expect("valid NOT")
                .evaluate(truth, known),
            Truth::Unknown
        );
    }
    for (truth, known) in column.bits().iter().zip(column.known()) {
        for bit in 132..143 {
            assert!(
                !truth.get(bit) && !known.get(bit),
                "no previous minute tail for day1 or exact opening3 for day2 means no GapFib answer"
            );
        }
    }
}

#[test]
fn combined_bridge_rejects_late_missing_mismatched_corrupt_and_unaligned_minutes_transactionally() {
    let exact = minutes(FIRST_DAY, 0);
    let signal = aggregate(&exact, 2);
    let original = column(&signal);
    for fault in 0..4 {
        let mut broken = exact.clone();
        let expected = match fault {
            0 => {
                broken.remove(301);
                ExactMinuteGapRefusal::MissingClosingMinute {
                    source: 150,
                    expected_ts_micros: stamp(FIRST_DAY, 301),
                }
            }
            1 => {
                let last = broken.last_mut().expect("final minute exists");
                last.close += 1;
                last.high = last.close;
                ExactMinuteGapRefusal::SignalCloseMismatch {
                    source: 187,
                    signal_close: 50,
                    minute_close: 51,
                }
            }
            2 => {
                broken.last_mut().expect("final minute exists").high = 49;
                ExactMinuteGapRefusal::CorruptMinute {
                    index: 374,
                    why: indicators::Corrupt::HighBelowLow,
                }
            }
            _ => {
                broken.last_mut().expect("final minute exists").ts_micros += 1;
                ExactMinuteGapRefusal::MalformedMinuteCadence { index: 374 }
            }
        };
        let mut offered = original.clone();
        assert_eq!(
            overlay_exact_minute_orb_and_gapfib(
                &signal,
                &broken,
                2 * MINUTE,
                widths(),
                Calendar::all_regular(),
                &mut offered
            ),
            Err(expected)
        );
        assert_eq!(
            offered, original,
            "no partial replacement after fault{fault}"
        );
    }
}
