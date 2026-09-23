//! Generated finite oracles for current-day Fibonacci availability and NOT.

#![allow(
    clippy::expect_used,
    reason = "fixture failures identify the broken premise"
)]

use indicators::Candle;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use vocab::Tolerance;
use vocab::expression::{Expression, Truth};
use vocab::tolerance::Base;

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
const NUMERATORS: [i128; 11] = [0, 236, 382, 500, 618, 786, 1000, 1272, 1618, 2000, 2618];

#[derive(Clone, Copy)]
struct Sample {
    day: i64,
    candle: Candle,
}

fn sample(day: i64, minute: i64, high: i64, low: i64, close: i64) -> Sample {
    Sample {
        day,
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
            pivot: vocab::tolerance::pinned_pivot().expect("pinned pivot width"),
        },
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

fn pinned() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("pinned Fibonacci width")
}

/// Reconstruct the prefix from records, independently of `CurDayFib` and its
/// level/leg methods. A true upward leg means the high changed more recently.
fn anchor(prior: &[Sample], day: i64) -> Option<(i128, i128, bool)> {
    let mut rows = prior.iter().filter(|row| row.day == day);
    let first = rows.next()?;
    let (mut high, mut low) = (first.candle.high, first.candle.low);
    let mut up = None;
    for row in rows {
        let high_changed = row.candle.high > high;
        let low_changed = row.candle.low < low;
        up = match (high_changed, low_changed) {
            (true, false) => Some(true),
            (false, true) => Some(false),
            (true, true) => None,
            (false, false) => up,
        };
        high = high.max(row.candle.high);
        low = low.min(row.candle.low);
    }
    let up = up?;
    let range = i128::from(high) - i128::from(low);
    if range <= 0 || range > i128::from(i64::MAX) {
        return None;
    }
    Some((i128::from(if up { high } else { low }), range, up))
}

fn expected(prior: &[Sample], current: Sample, tolerance: Tolerance) -> [Truth; 11] {
    let Some((anchor, range, up)) = anchor(prior, current.day) else {
        return [Truth::Unknown; 11];
    };
    NUMERATORS.map(|numerator| {
        let distance = numerator * range / 1000;
        let level = if up {
            anchor - distance
        } else {
            anchor + distance
        };
        if i64::try_from(level).is_err() || tolerance.base() != Some(Base::SessionRange) {
            Truth::Unknown
        } else if (i128::from(current.candle.close) - level).abs() * 1000
            <= i128::from(tolerance.milli()) * range
        {
            Truth::True
        } else {
            Truth::False
        }
    })
}

fn verify(samples: &[Sample], tolerance: Tolerance) -> [[usize; 3]; 11] {
    let mut actual = evaluator(tolerance);
    let mut legacy = evaluator(tolerance);
    let negations: Vec<_> = (121..132)
        .map(|bit| Expression::parse(&format!("!{bit}")).expect("each current-day rung is live"))
        .collect();
    let mut counts = [[0; 3]; 11];
    for (index, &current) in samples.iter().enumerate() {
        let (truth, known) = actual
            .step_known(&current.candle)
            .expect("valid generated fold");
        assert_eq!(legacy.step(&current.candle).expect("truth-only API"), truth);
        let answers = expected(
            samples.get(..index).expect("past prefix"),
            current,
            tolerance,
        );
        for (((bit, answer), negation), count) in
            (121..132).zip(answers).zip(&negations).zip(&mut counts)
        {
            assert_eq!(
                truth.get(bit),
                answer == Truth::True,
                "row={index} bit={bit}"
            );
            assert_eq!(
                known.get(bit),
                answer != Truth::Unknown,
                "known row={index} bit={bit}"
            );
            let [false_count, true_count, unknown_count] = count;
            let negated = match answer {
                Truth::False => {
                    *false_count += 1;
                    Truth::True
                }
                Truth::True => {
                    *true_count += 1;
                    Truth::False
                }
                Truth::Unknown => {
                    *unknown_count += 1;
                    Truth::Unknown
                }
            };
            assert_eq!(
                negation.evaluate(truth, known),
                negated,
                "NOT row={index} bit={bit}"
            );
        }
    }
    counts
}

fn seeds(up: bool, range: i64) -> [Sample; 2] {
    let middle = 100_000;
    if up {
        [
            sample(30_000, 0, middle + range / 2, middle, middle),
            sample(30_000, 1, middle + range, middle + 1, middle + range),
        ]
    } else {
        [
            sample(30_000, 0, middle, middle - range / 2, middle),
            sample(30_000, 1, middle - 1, middle - range, middle - range),
        ]
    }
}

#[test]
fn every_rung_on_both_legs_matches_integer_rounding_band_edges_and_not() {
    let mut total = [[0; 3]; 11];
    let mut cases = 0;
    for up in [false, true] {
        for range in [201, 10_001] {
            let seeds = seeds(up, range);
            let (anchor, span, _) = anchor(&seeds, 30_000).expect("determined fixture leg");
            let band = i64::try_from(span * i128::from(pinned().milli()) / 1000)
                .expect("finite tolerance boundary");
            for numerator in NUMERATORS {
                let distance = numerator * span / 1000;
                let level = if up {
                    anchor - distance
                } else {
                    anchor + distance
                };
                for delta in [-band - 1, -band, 0, band, band + 1] {
                    let close = i64::try_from(level).expect("positive finite fixture rung") + delta;
                    let [first, second] = seeds;
                    let samples = [first, second, sample(30_000, 2, close, close, close)];
                    for (total, counts) in total.iter_mut().zip(verify(&samples, pinned())) {
                        for (sum, count) in total.iter_mut().zip(counts) {
                            *sum += count;
                        }
                    }
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 220);
    for (bit, [false_count, true_count, unknown_count]) in (121..132).zip(total) {
        assert!(
            false_count > 0 && true_count > 0,
            "both measurable outcomes for bit {bit}"
        );
        assert_eq!(unknown_count, 440, "first two bars are cold in every case");
    }
}

#[test]
fn availability_uses_the_old_leg_through_new_extremes_touches_and_session_reset() {
    let samples = [
        sample(30_000, 0, 100_000, 100_000, 100_000),
        sample(30_000, 1, 100_010, 100_010, 100_010),
        sample(30_000, 2, 100_020, 100_010, 100_020),
        // The current bar erases the leg only AFTER its truth and known answers.
        sample(30_000, 3, 100_030, 99_990, 100_015),
        sample(30_000, 4, 100_020, 100_000, 100_010),
        sample(30_000, 5, 100_020, 99_980, 99_980),
        // Touching both exact extremes preserves the previously determined leg.
        sample(30_000, 6, 100_030, 99_980, 100_000),
        sample(30_000, 7, 100_040, 99_985, 100_040),
        sample(30_000, 8, 100_036, 100_000, 100_035),
        sample(30_001, 0, 100_010, 100_000, 100_010),
        sample(30_001, 1, 100_020, 100_010, 100_020),
        sample(30_001, 2, 100_020, 100_010, 100_020),
    ];
    for [false_count, true_count, unknown_count] in verify(&samples, pinned()) {
        assert_eq!((false_count + true_count, unknown_count), (6, 6));
    }
}

#[test]
fn unknown_bands_and_zero_ranges_never_satisfy_negation() {
    let [first, second] = seeds(true, 201);
    let samples = [
        first,
        second,
        sample(30_000, 2, 100_201, 100_201, 100_201),
        sample(30_001, 0, 100_000, 100_000, 100_000),
        sample(30_001, 1, 100_000, 100_000, 100_000),
    ];
    for tolerance in [
        pinned(),
        Tolerance::from_milli_on(Base::SessionRange, 0).expect("equality tolerance"),
        Tolerance::from_milli_on(Base::SessionRange, i64::MAX).expect("finite maximum width"),
        vocab::tolerance::pinned_pivot().expect("wrong base"),
        Tolerance::from_milli(10).expect("missing base"),
    ] {
        for [false_count, true_count, unknown_count] in verify(&samples, tolerance) {
            let expected = if tolerance.base() == Some(Base::SessionRange) {
                (1, 4)
            } else {
                (0, 5)
            };
            assert_eq!((false_count + true_count, unknown_count), expected);
        }
    }
}

#[test]
fn extension_overflow_withholds_only_unrepresentable_rungs_on_the_emitted_leg() {
    for up in [false, true] {
        let (old, new) = if up { (1, i64::MAX) } else { (i64::MAX, 1) };
        let samples = [
            sample(30_000, 0, old, old, old),
            sample(30_000, 1, new, new, new),
            sample(30_000, 2, new, new, new),
        ];
        let counts = verify(&samples, pinned());
        let unavailable = counts
            .iter()
            .filter(|[_, _, unknown]| *unknown == 3)
            .count();
        assert_eq!(unavailable, if up { 1 } else { 4 });
        for [false_count, true_count, unknown_count] in counts {
            assert_eq!(false_count + true_count + unknown_count, 3);
        }
    }
}
