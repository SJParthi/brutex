//! Independent finite boundary oracles for the predicates handed to a sweep.

#![allow(
    clippy::expect_used,
    reason = "fixture failures must state the broken premise"
)]

use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::{Patterns, Thresholds};
use indicators::vwap::Availability;
use indicators::{Candle, Corrupt};
use vocab::ConditionMask;

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;

fn bar(day: i64, minute: i64, open: i64, high: i64, low: i64, close: i64) -> Candle {
    Candle::new(
        day * DAY + (225 + minute) * MINUTE,
        open,
        high,
        low,
        close,
        0,
        i64::MIN,
    )
}

fn fresh() -> Evaluator {
    Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

#[test]
fn every_small_integer_single_candle_matches_independent_shape_ratios() {
    let mut cases = 0;
    let mut hammers = 0;
    for range in 0..=20_i64 {
        for open in 0..=range {
            for close in 0..=range {
                let candle = bar(30_000, 0, 100 + open, 100 + range, 100, 100 + close);
                let mask = Patterns::default()
                    .step(&candle)
                    .expect("positive contained candle");
                let body = (close - open).abs();
                let lower = open.min(close);
                let upper = range - open.max(close);
                let small = range > 0 && 10 * body <= 3 * range;
                let doji = range > 0 && 10 * body <= range;
                let hammer = small && lower >= 2 * body && 20 * upper <= range;
                let inverted = small && upper >= 2 * body && 20 * lower <= range;
                for (position, expected) in [
                    (153, hammer),
                    (154, inverted),
                    (174, doji && lower >= 2 * body && 20 * upper <= range),
                    (175, doji && upper >= 2 * body && 20 * lower <= range),
                    (225, range == 0),
                    (
                        171,
                        small && !doji && 5 * upper >= range && 5 * lower >= range,
                    ),
                ] {
                    assert_eq!(
                        mask.get(position),
                        expected,
                        "range={range} open={open} close={close} position={position}"
                    );
                }
                cases += 1;
                hammers += usize::from(hammer);
            }
        }
    }
    assert_eq!(cases, 3311);
    assert!(
        hammers > 0 && hammers < cases,
        "both hammer outcomes must occur"
    );
}

#[test]
fn every_small_two_body_pair_matches_inclusive_engulfing_and_strict_colour() {
    let mut cases = 0;
    let mut bullish = 0;
    let mut bearish = 0;
    for old_open in 100..=106_i64 {
        for old_close in 100..=106_i64 {
            for open in 100..=106_i64 {
                for close in 100..=106_i64 {
                    let mut detector = Patterns::default();
                    detector
                        .step(&bar(30_000, 0, old_open, 108, 98, old_close))
                        .expect("prior bar");
                    let mask = detector
                        .step(&bar(30_000, 1, open, 108, 98, close))
                        .expect("current bar");
                    let up = old_open > old_close
                        && close > open
                        && open <= old_close
                        && close >= old_open;
                    let down = old_close > old_open
                        && open > close
                        && open >= old_close
                        && close <= old_open;
                    assert_eq!(
                        mask.get(157),
                        up,
                        "prior={old_open}/{old_close} current={open}/{close}"
                    );
                    assert_eq!(
                        mask.get(158),
                        down,
                        "prior={old_open}/{old_close} current={open}/{close}"
                    );
                    cases += 1;
                    bullish += usize::from(up);
                    bearish += usize::from(down);
                }
            }
        }
    }
    assert_eq!(cases, 2401);
    assert!(bullish > 0 && bearish > 0);
    assert_eq!(
        bullish, bearish,
        "the mirrored finite price domain is symmetric"
    );
}

#[test]
fn nonpositive_prices_refuse_without_poisoning_state_and_positive_extremes_survive() {
    for price in [i64::MIN, -1, 0] {
        let mut evaluator = fresh();
        let bad = bar(30_000, 0, price, price, price, price);
        assert_eq!(bad.check(), Ok(()), "the sign guard must be reached");
        assert_eq!(bad.check_evaluable(), Err(Corrupt::PriceNotPositive));
        assert_eq!(evaluator.step_known(&bad), Err(Corrupt::PriceNotPositive));
        let ordinary = bar(30_000, 0, 100, 110, 90, 105);
        assert_eq!(
            evaluator.step_known(&ordinary),
            fresh().step_known(&ordinary)
        );
    }
    for price in [1, i64::MAX] {
        let candle = bar(30_000, 0, price, price, price, price);
        let (truth, known) = fresh().step_known(&candle).expect("positive flat extreme");
        assert!(truth.get(225), "a four-price doji has a concrete answer");
        assert!(known.get(225));
        assert!(!truth.get(30) && !truth.get(31));
        assert!(known.get(30) && known.get(31));
    }
}

#[test]
fn availability_uses_the_emission_boundary_and_keeps_absent_volume_unknown() {
    let mut evaluator = fresh();
    let mut other = fresh();
    for minute in 0..=200 {
        let candle = bar(30_000, minute, 100, 110, 90, 100);
        let (truth, known) = evaluator
            .step_known(&candle)
            .expect("ordinary constant bar");
        assert_eq!(other.step(&candle).expect("truth-only API"), truth);
        assert_eq!(truth.union(&known), known, "every truth must be known");
        assert_eq!(vocab::table::only_live(known), known);
        assert_eq!(
            known.get(0),
            minute >= 20,
            "EMA20 must be seeded before this bar"
        );
        assert_eq!(
            known.get(2),
            minute >= 200,
            "EMA200 must be seeded before this bar"
        );
        assert_eq!(known.get(4), minute >= 200, "both averages must be seeded");
        assert_eq!(
            known.get(64),
            minute >= 10,
            "the trailing stop needs its ATR seed"
        );
        assert!(!truth.get(0) && !truth.get(1) && !truth.get(2) && !truth.get(3));
        assert_eq!(
            known.get(157),
            minute >= 1,
            "engulfing needs two current-session bars"
        );
        assert_eq!(
            known.get(176),
            minute >= 4,
            "rising-three-methods needs five bars"
        );
        for position in indicators::vwap::positions() {
            assert!(
                !known.get(u32::from(position)),
                "absent-volume position {position} is unknown"
            );
        }
    }
    let (_, next_day) = evaluator
        .step_known(&bar(30_001, 0, 100, 110, 90, 100))
        .expect("new session");
    assert!(next_day.get(153));
    assert!(
        !next_day.get(157) && !next_day.get(176),
        "patterns cannot borrow yesterday's lookback"
    );
    assert!(
        !next_day.get(276) && !next_day.get(277),
        "session open is unavailable before its first fold"
    );
    assert!(
        next_day.get(2),
        "EMA continuity must survive a session reset"
    );
    assert!(
        next_day.get(15),
        "a valid completed daily anchor is now available"
    );
    for position in [13, 14, 15, 16] {
        assert!(
            next_day.get(position),
            "both sides of prior high/low are known"
        );
    }
}

#[test]
fn vwap_false_availability_requires_two_contributing_bars_and_resets_each_session() {
    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Present,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    // Zero-volume bars cannot seed either reference. One contributing bar
    // supplies a mean but no dispersion; a second is required by emission.
    for (minute, volume, expected) in [(0, 0, false), (1, 1, false), (2, 0, false), (3, 1, true)] {
        let mut candle = bar(30_000, minute, 100, 100, 100, 100);
        candle.volume = volume;
        let (truth, known) = evaluator
            .step_known(&candle)
            .expect("finite positive volume");
        for position in [52, 53, 143, 144] {
            assert!(!truth.get(position), "flat price equals its VWAP");
            assert_eq!(
                known.get(position),
                expected,
                "minute={minute} bit={position}"
            );
        }
    }
    let mut candle = bar(30_001, 0, 100, 100, 100, 100);
    candle.volume = 1;
    let (truth, known) = evaluator.step_known(&candle).expect("new session");
    for position in [52, 53, 143, 144] {
        assert!(!truth.get(position));
        assert!(
            !known.get(position),
            "yesterday's dispersion is unavailable"
        );
    }
}

#[test]
fn all_vwap_predicates_and_negations_match_independent_integer_levels() {
    use vocab::expression::{Expression, Truth};

    let mut evaluator = Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Present,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    );
    let positions = indicators::vwap::positions();
    let negations = positions.map(|bit| {
        Expression::parse(&format!("!{bit}")).expect("every VWAP position is in the vocabulary")
    });
    for (minute, price) in [(0, 200), (1, 300)] {
        let mut candle = bar(30_000, minute, price, price, price, price);
        candle.volume = 1;
        let (truth, known) = evaluator.step_known(&candle).expect("generated seed");
        if minute == 0 {
            for expression in &negations {
                assert_eq!(expression.evaluate(truth, known), Truth::Unknown);
            }
        }
    }
    // The two equal-volume prices have exact mean250 and sigma50. Later
    // zero-volume observations compare their closes without changing either.
    let width = Widths::pinned()
        .expect("width used by the actual evaluator")
        .fib
        .milli();
    let near = |close: i64, level: i64| (close - level).abs() * 1000 <= width * 50;
    for (offset, close) in (1..=451).step_by(2).enumerate() {
        let minute = i64::try_from(offset).expect("finite generated samples") + 2;
        let candle = bar(30_000, minute, close, close, close, close);
        let (truth, known) = evaluator.step_known(&candle).expect("generated comparison");
        let expected = [
            close > 250,
            close < 250,
            close > 250,
            close < 250,
            near(close, 250),
            close > 300,
            close < 200,
            near(close, 300),
            near(close, 200),
            (200..=300).contains(&close),
            close > 350,
            close < 150,
            near(close, 350),
            near(close, 150),
            (150..=350).contains(&close),
            close > 400,
            close < 100,
            near(close, 400),
            near(close, 100),
            (100..=400).contains(&close),
        ];
        for ((bit, expected), expression) in positions.into_iter().zip(expected).zip(&negations) {
            assert!(
                known.get(u32::from(bit)),
                "close={close} bit={bit} must be known"
            );
            assert_eq!(
                truth.get(u32::from(bit)),
                expected,
                "close={close} bit={bit}"
            );
            assert_eq!(
                expression.evaluate(truth, known),
                if expected { Truth::False } else { Truth::True }
            );
        }
    }
    let mut next = bar(30_001, 0, 250, 250, 250, 250);
    next.volume = 1;
    let (truth, known) = evaluator.step_known(&next).expect("new generated session");
    for (bit, expression) in positions.into_iter().zip(&negations) {
        assert!(!truth.get(u32::from(bit)) && !known.get(u32::from(bit)));
        assert_eq!(expression.evaluate(truth, known), Truth::Unknown);
    }
}

#[test]
fn column_clone_projection_and_clearing_preserve_aligned_availability() {
    let bars: Vec<_> = (30_000..30_008)
        .flat_map(|day| (0..50).map(move |minute| bar(day, minute, 100, 110, 90, 100)))
        .collect();
    let column = Column::build(&bars, &mut fresh());
    assert!(!column.bits().is_empty(), "the fixture must pass warm-up");
    assert_eq!(column.bits().len(), column.known().len());
    let mut cloned = column.try_clone_exact().expect("finite clone");
    assert_eq!(cloned, column);
    let mapping: Vec<_> = column.sources().iter().copied().map(Some).collect();
    let (projected, dropped) = column
        .reproject_checked(&mapping, &bars)
        .expect("parallel projection");
    assert_eq!(dropped, 0);
    assert_eq!(projected.known(), column.known());
    let boundary = *column.sources().last().expect("nonempty source column");
    cloned.clear_before(boundary);
    for ((&source, truth), known) in cloned
        .sources()
        .iter()
        .zip(cloned.bits())
        .zip(cloned.known())
    {
        if source < boundary {
            assert_eq!(*truth, ConditionMask::ZERO);
            assert_eq!(
                *known,
                ConditionMask::ZERO,
                "NOT cannot fire outside the test window"
            );
        } else {
            assert!(known.get(30) && known.get(31));
        }
    }
}

#[test]
fn anchored_columns_certify_real_prior_daily_predicates_after_trend_warmup() {
    use indicators::anchored::{AnchoredEvaluator, DailyEligibility, DailyReference};
    use indicators::column::AnchoredColumn;

    let references: Vec<_> = (29_995..30_000)
        .map(|day| {
            DailyReference::new(bar(day, 0, 100, 110, 90, 105), DailyEligibility::Eligible)
                .expect("positive daily reference")
        })
        .collect();
    let bars: Vec<_> = (0..250)
        .map(|minute| bar(30_000, minute, 100, 110, 90, 100))
        .collect();
    let mut evaluator = AnchoredEvaluator::new(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        &references,
    )
    .expect("ordered daily references");
    let anchored =
        AnchoredColumn::build_required(&bars, &mut evaluator).expect("prior daily anchors");
    let column = anchored.column();
    assert_eq!(column.len(), 50, "EMA200 warmth is read before emission");
    assert_eq!(column.bits().len(), column.known().len());
    for (truth, known) in column.bits().iter().zip(column.known()) {
        assert_eq!(truth.union(known), *known);
        assert!(
            known.get(15),
            "valid prior daily pivot predicate is available"
        );
        assert!(!truth.get(13) && !truth.get(16));
        assert!(
            known.get(13) && known.get(16),
            "false daily sides are available too"
        );
        assert!(known.get(176), "five same-session candles are available");
        assert!(
            !known.get(52),
            "absent VWAP remains unknown under anchoring"
        );
    }
}
