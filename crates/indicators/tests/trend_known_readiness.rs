//! Generated finite fixtures; independent delayed-fractal and event oracles.
#![allow(clippy::expect_used, reason = "a broken fixture must fail explicitly")]

use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::trend::{TrendState, TrendThresholds};
use indicators::vwap::Availability;
use indicators::{Candle, Corrupt};
use vocab::expression::{Expression, Truth};
use vocab::tolerance::Base;
use vocab::{ConditionMask, Tolerance};

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
const POSITIONS: [u32; 6] = [56, 57, 58, 59, 72, 73];
const ALL: [u32; 14] = [0, 1, 2, 3, 4, 5, 56, 57, 58, 59, 64, 65, 72, 73];

fn bar(index: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle::new(
        30_000 * DAY + (225 + index) * MINUTE,
        close,
        high,
        low,
        close,
        0,
        i64::MIN,
    )
}

fn evaluator(tolerance: Tolerance) -> Evaluator {
    let mut widths = Widths::pinned().expect("pinned widths");
    widths.fib = tolerance;
    Evaluator::with_calendar(
        widths,
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

fn tolerance(milli: i64) -> Tolerance {
    Tolerance::from_milli_on(Base::SessionRange, milli).expect("nonnegative band")
}

#[derive(Clone, Copy)]
struct Reference {
    price: i64,
    range: i128,
    confirmed: usize,
}

#[derive(Default)]
struct Oracle {
    history: Vec<Candle>,
    high: Option<Reference>,
    low: Option<Reference>,
    prior_up: Option<bool>,
}

impl Oracle {
    fn answer(&mut self, candle: Candle, band: Tolerance) -> [Truth; 6] {
        let mut answer = if self.high.is_some() && self.low.is_some() {
            [Truth::False; 6]
        } else {
            [Truth::Unknown; 6]
        };
        // Select the newest actually broken reference. This is independent of
        // Structure::classify and never accesses the detector under test.
        let broken_high = self.high.filter(|level| candle.close > level.price);
        let broken_low = self.low.filter(|level| candle.close < level.price);
        let direction = match (broken_high, broken_low) {
            (Some(_), None) => Some(true),
            (None, Some(_)) => Some(false),
            (Some(high), Some(low)) if high.confirmed != low.confirmed => {
                Some(high.confirmed > low.confirmed)
            }
            _ => None,
        };
        if let Some(up) = direction {
            let reversal = self.prior_up.is_some_and(|prior| prior != up);
            let index = usize::from(!up) + 2 * usize::from(reversal);
            *answer.get_mut(index).expect("four event positions") = Truth::True;
            self.prior_up = Some(up);
        }
        for (slot, level) in answer.iter_mut().skip(4).zip([self.high, self.low]) {
            *slot = match level {
                Some(level) if band.base() == Some(Base::SessionRange) && level.range > 0 => {
                    let distance = (i128::from(candle.close) - i128::from(level.price)).abs();
                    if distance * 1000 <= i128::from(band.milli()) * level.range {
                        Truth::True
                    } else {
                        Truth::False
                    }
                }
                _ => Truth::Unknown,
            };
        }
        self.history.push(candle);
        self.confirm();
        answer
    }

    fn confirm(&mut self) {
        let length = self.history.len();
        if length < 5 {
            return;
        }
        let window = self
            .history
            .get(length - 5..)
            .expect("last five observed bars");
        let middle = window.get(2).expect("middle");
        let high = window.iter().map(|bar| bar.high).max().expect("nonempty");
        let low = window.iter().map(|bar| bar.low).min().expect("nonempty");
        let range = i128::from(high) - i128::from(low);
        if range > i128::from(i64::MAX) {
            return;
        }
        let reference = |price| Reference {
            price,
            range,
            confirmed: length,
        };
        if window.iter().filter(|bar| bar.high >= middle.high).count() == 1 {
            self.high = Some(reference(middle.high));
        }
        if window.iter().filter(|bar| bar.low <= middle.low).count() == 1 {
            self.low = Some(reference(middle.low));
        }
    }
}

fn assert_answer(truth: ConditionMask, known: ConditionMask, expected: [Truth; 6]) {
    for (position, expected) in POSITIONS.into_iter().zip(expected) {
        assert_eq!(
            truth.get(position),
            expected == Truth::True,
            "truth{position}"
        );
        assert_eq!(
            known.get(position),
            expected != Truth::Unknown,
            "known{position}"
        );
        let negation = Expression::parse(&format!("!{position}")).expect("live trend bit");
        let opposite = match expected {
            Truth::True => Truth::False,
            Truth::False => Truth::True,
            Truth::Unknown => Truth::Unknown,
        };
        assert_eq!(negation.evaluate(truth, known), opposite, "NOT{position}");
    }
}

fn exercise(bars: &[Candle], band: Tolerance) -> [[u64; 3]; 6] {
    let mut evaluator = evaluator(band);
    let mut original = TrendState::new(TrendThresholds::CLASSICAL);
    let mut oracle = Oracle::default();
    let mut counts = [[0; 3]; 6];
    for candle in bars {
        let expected = oracle.answer(*candle, band);
        let (truth, known) = evaluator
            .step_known(candle)
            .expect("valid generated fixture");
        let unchanged = original
            .step(candle, band)
            .expect("same unchanged truth producer");
        for position in ALL {
            assert_eq!(
                truth.get(position),
                unchanged.get(position),
                "existing truth{position}"
            );
            assert!(!truth.get(position) || known.get(position));
        }
        assert_answer(truth, known, expected);
        for (count, expected) in counts.iter_mut().zip(expected) {
            let slot = match expected {
                Truth::True => 0,
                Truth::False => 1,
                Truth::Unknown => 2,
            };
            *count.get_mut(slot).expect("three truth classes") += 1;
        }
    }
    counts
}

fn both_references() -> Vec<Candle> {
    [
        (110, 90, 100),
        (110, 90, 100),
        (130, 70, 100),
        (110, 90, 100),
        (110, 90, 100),
    ]
    .into_iter()
    .zip(0_i64..)
    .map(|((high, low, close), index)| bar(index, high, low, close))
    .collect()
}

#[test]
fn confirmed_swings_become_known_only_after_both_right_hand_bars_have_folded() {
    let band = tolerance(10);
    let mut evaluator = evaluator(band);
    let mut sequence = both_references();
    sequence.push(bar(5, 110, 90, 100));
    for (index, candle) in sequence.iter().enumerate() {
        let (truth, known) = evaluator.step_known(candle).expect("valid candle");
        let answer = if index < 5 {
            Truth::Unknown
        } else {
            Truth::False
        };
        assert_answer(truth, known, [answer; 6]);
    }
    // The same current close is inside both bands only after confirmation;
    // the two future neighbour bars were unavailable on the peak itself.
    exercise(&sequence, band);
}

#[test]
fn first_break_continuation_reversal_quiet_and_equality_follow_the_independent_oracle() {
    let mut bars = both_references();
    for (index, close) in (5_i64..).zip([100, 130, 131, 132, 69, 68, 135, 130, 70, 100, 67]) {
        bars.push(bar(index, close + 1, close - 1, close));
    }
    let mut oracle = Oracle::default();
    let expected: Vec<_> = bars
        .iter()
        .map(|bar| oracle.answer(*bar, tolerance(10)))
        .collect();
    for (row, event) in [(7, 0), (8, 0), (9, 3), (10, 1), (11, 2)] {
        assert_eq!(
            expected.get(row).expect("event row").get(event),
            Some(&Truth::True)
        );
    }
    assert_eq!(expected.get(5).expect("quiet"), &[Truth::False; 6]);
    assert_eq!(
        expected.get(6).expect("equality").get(..4),
        Some(&[Truth::False; 4][..])
    );
    let counts = exercise(&bars, tolerance(10));
    for (position, [trues, falses, unknowns]) in POSITIONS.into_iter().zip(counts) {
        assert!(
            trues > 0 && falses > 0 && unknowns > 0,
            "nonvacuous trend{position}"
        );
    }
}

#[test]
fn a_missing_opposite_swing_stays_unknown_and_cannot_satisfy_negation() {
    for mirrored in [false, true] {
        let bars: Vec<_> = [
            (100, 90, 95),
            (110, 91, 100),
            (130, 92, 110),
            (115, 93, 105),
            (100, 94, 95),
            (130, 129, 130),
            (132, 131, 132),
        ]
        .into_iter()
        .zip(0_i64..)
        .map(|((high, low, close), index)| {
            if mirrored {
                bar(index, 300 - low, 300 - high, 300 - close)
            } else {
                bar(index, high, low, close)
            }
        })
        .collect();
        let counts = exercise(&bars, tolerance(10));
        let missing = if mirrored { 4 } else { 5 };
        assert_eq!(counts.get(missing), Some(&[0, 0, 7]));
        for count in counts.iter().take(4) {
            assert_eq!(count.get(1), Some(&0));
        }
    }
}

#[test]
fn each_swing_band_checks_its_exact_tolerance_and_inclusive_edges() {
    for band in [
        tolerance(0),
        tolerance(100),
        tolerance(i64::MAX),
        Tolerance::from_milli(100).expect("baseless token"),
        Tolerance::from_milli_on(Base::CprWidth, 100).expect("wrong family"),
    ] {
        for close in [63, 64, 70, 76, 77, 123, 124, 130, 136, 137] {
            let mut bars = both_references();
            bars.push(bar(5, close + 1, close - 1, close));
            let counts = exercise(&bars, band);
            if band.base() != Some(Base::SessionRange) {
                assert_eq!(counts.get(4), Some(&[0, 0, 6]));
                assert_eq!(counts.get(5), Some(&[0, 0, 6]));
            }
        }
    }
}

#[test]
fn positive_extremes_plateaus_and_refusals_do_not_fabricate_or_poison_references() {
    let midpoint = i64::MAX / 2;
    let bars = [
        bar(0, midpoint + 1, midpoint - 1, midpoint),
        bar(1, midpoint + 1, midpoint - 1, midpoint),
        bar(2, i64::MAX, 1, midpoint),
        bar(3, midpoint + 1, midpoint - 1, midpoint),
        bar(4, midpoint + 1, midpoint - 1, midpoint),
        bar(5, i64::MAX, i64::MAX - 1, i64::MAX),
        bar(6, 2, 1, 1),
    ];
    for band in [tolerance(0), tolerance(10), tolerance(i64::MAX)] {
        exercise(&bars, band);
    }
    let plateau: Vec<_> = (0..220).map(|index| bar(index, 100, 100, 100)).collect();
    assert_eq!(exercise(&plateau, tolerance(10)), [[0, 0, 220]; 6]);
    let mut evaluator = evaluator(tolerance(10));
    for candle in both_references() {
        evaluator.step_known(&candle).expect("seed");
    }
    let mut saved = evaluator;
    assert_eq!(
        evaluator.step_known(&bar(4, 110, 90, 100)),
        Err(Corrupt::TimestampNotIncreasing)
    );
    assert_eq!(
        evaluator.step_known(&bar(5, i64::MAX, i64::MIN, 100)),
        Err(Corrupt::RangeOverflows)
    );
    assert_eq!(
        evaluator.step_known(&bar(5, 1, 0, 1)),
        Err(Corrupt::PriceNotPositive)
    );
    for index in 5..80 {
        let close = 100 + (index % 11) * 7;
        let candle = bar(index, close + 3, close - 3, close);
        assert_eq!(
            evaluator.step_known(&candle),
            saved.step_known(&candle),
            "refusal changed subsequent state at bar{index}"
        );
    }
}

#[test]
fn average_and_stop_known_gates_use_prior_contribution_counts() {
    let mut evaluator = evaluator(tolerance(10));
    for index in 0..=201 {
        let (truth, known) = evaluator
            .step_known(&bar(index, 100, 100, 100))
            .expect("flat bar");
        for (positions, first) in [
            (&[0, 1][..], 20),
            (&[2, 3, 4, 5][..], 200),
            (&[64, 65][..], 10),
        ] {
            for position in positions {
                assert!(!truth.get(*position), "equal anchors never set a side");
                assert_eq!(
                    known.get(*position),
                    index >= first,
                    "known{position} beforefold{index}"
                );
            }
        }
    }
}

#[test]
fn existing_trend_history_carries_across_days_and_a_fresh_evaluator_resets_it() {
    let mut across = both_references();
    let mut next_day = bar(5, 131, 130, 131);
    next_day.ts_micros += DAY;
    across.push(next_day);
    let counts = exercise(&across, tolerance(10));
    assert_eq!(
        counts.first(),
        Some(&[1, 0, 5]),
        "existing first break after the day boundary"
    );
    let mut fresh = evaluator(tolerance(10));
    let (truth, known) = fresh.step_known(&next_day).expect("fresh valid candle");
    assert_answer(truth, known, [Truth::Unknown; 6]);
}
