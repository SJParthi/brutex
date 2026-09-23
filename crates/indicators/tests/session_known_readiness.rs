//! Generated finite fixtures for the shipped session descriptions. Independent
//! integer oracles check truth and false-answer availability through the public
//! evaluator and search column; they do not validate the market conventions.

#![allow(clippy::expect_used, reason = "fixture failures name their premise")]

use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use indicators::{Candle, Corrupt};
use vocab::Tolerance;
use vocab::expression::{Expression, Truth};
use vocab::tolerance::Base;

const DAY: i64 = 86_400_000_000;
const MINUTE: i64 = 60_000_000;
// Independent owner list, including the six previously available descriptions.
const POSITIONS: [u32; 25] = [
    30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50, 51, 66, 67,
    68,
];

fn bar(day: i64, minute: i64, prices: [i64; 4]) -> Candle {
    let [open, high, low, close] = prices;
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

fn pinned() -> Tolerance {
    vocab::tolerance::pinned_fib().expect("pinned Fibonacci width")
}

fn evaluator(tolerance: Tolerance) -> Evaluator {
    Evaluator::with_calendar(
        Widths {
            fib: tolerance,
            pivot: vocab::tolerance::pinned_pivot().expect("pivot width"),
        },
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

fn answer(value: bool) -> Truth {
    if value { Truth::True } else { Truth::False }
}

#[expect(
    clippy::too_many_lines,
    reason = "one independent position table keeps every oracle visible"
)]
#[expect(
    clippy::manual_midpoint,
    reason = "independent wide oracle: both operands came from positive i64 prices, so their i128 sum cannot overflow"
)]
fn expected(prefix: &[Candle], current: &Candle, tolerance: Tolerance) -> [Truth; 25] {
    let day = current.ts_micros / DAY;
    let today: Vec<_> = prefix.iter().filter(|c| c.ts_micros / DAY == day).collect();
    let previous_day = prefix
        .iter()
        .rev()
        .find(|c| c.ts_micros / DAY < day)
        .map(|c| c.ts_micros / DAY);
    let previous: Vec<_> = prefix
        .iter()
        .filter(|c| Some(c.ts_micros / DAY) == previous_day)
        .collect();
    let open = i128::from(current.open);
    let close = i128::from(current.close);
    let high = i128::from(current.high);
    let low = i128::from(current.low);
    let body = (open - close).abs();
    let range = high - low;
    let day_high = today
        .iter()
        .map(|c| i128::from(c.high))
        .chain([high])
        .max()
        .expect("current high");
    let day_low = today
        .iter()
        .map(|c| i128::from(c.low))
        .chain([low])
        .min()
        .expect("current low");
    let day_range = day_high - day_low;
    let day_open = today.first().map_or(open, |c| i128::from(c.open));
    let prior: Vec<_> = today
        .iter()
        .rev()
        .take(3)
        .map(|c| c.close.cmp(&c.open))
        .collect();
    let minutes = (current.ts_micros % DAY) / MINUTE - 225;
    POSITIONS.map(|position| match position {
        30 => answer(close > open),
        31 => answer(close < open),
        32..=36 if range == 0 => Truth::Unknown,
        32 => answer(body * 1000 <= range * 100),
        33 => answer(body * 1000 >= range * 600),
        34 => answer(body * 1000 <= range * 300),
        35 => answer((high - open.max(close)) * 1000 >= range * 400),
        36 => answer((open.min(close) - low) * 1000 >= range * 400),
        37..=39 if prior.len() != 3 => Truth::Unknown,
        37 => answer(prior.iter().all(|d| d.is_gt())),
        38 => answer(prior.iter().all(|d| d.is_lt())),
        39 => answer(
            prior.iter().all(|d| !d.is_eq()) && prior.windows(2).all(|p| p.first() != p.last()),
        ),
        40 => answer(close == day_high),
        41 => answer(close == day_low),
        42 | 43 if day_range == 0 => Truth::Unknown,
        42 => answer((close - day_low) * 3 >= day_range * 2),
        43 => answer((close - day_low) * 3 <= day_range),
        44 => answer((0..60).contains(&minutes)),
        45 => answer((60..165).contains(&minutes)),
        46 => answer((165..285).contains(&minutes)),
        47 => answer((285..375).contains(&minutes)),
        _ => {
            let Some(last) = previous.last() else {
                return Truth::Unknown;
            };
            let pc = i128::from(last.close);
            match position {
                48 => answer(day_open > pc),
                49 => answer(day_open < pc),
                50 => answer(
                    day_high
                        <= previous
                            .iter()
                            .map(|c| i128::from(c.high))
                            .max()
                            .expect("previous high")
                        && day_low
                            >= previous
                                .iter()
                                .map(|c| i128::from(c.low))
                                .min()
                                .expect("previous low"),
                ),
                51 => answer(
                    day_high
                        > previous
                            .iter()
                            .map(|c| i128::from(c.high))
                            .max()
                            .expect("previous high")
                        && day_low
                            < previous
                                .iter()
                                .map(|c| i128::from(c.low))
                                .min()
                                .expect("previous low"),
                ),
                _ if day_open == pc => Truth::Unknown,
                66 => answer(close > (day_open + pc) / 2),
                67 => answer(close < (day_open + pc) / 2),
                68 if tolerance.base() == Some(Base::SessionRange) => answer(
                    (close - (day_open + pc) / 2).abs() * 1000
                        <= (day_open - pc).abs() * i128::from(tolerance.milli()),
                ),
                _ => Truth::Unknown,
            }
        }
    })
}

fn check(
    evaluator: &mut Evaluator,
    prefix: &mut Vec<Candle>,
    current: Candle,
    tolerance: Tolerance,
) -> [Truth; 25] {
    let expected = expected(prefix, &current, tolerance);
    let mut legacy = *evaluator;
    let (truth, known) = evaluator
        .step_known(&current)
        .expect("valid chronological fixture");
    assert_eq!(
        truth,
        legacy.step(&current).expect("unchanged truth-only API")
    );
    for (position, wanted) in POSITIONS.into_iter().zip(expected) {
        assert_eq!(
            truth.get(position),
            wanted == Truth::True,
            "truth{position} at {current:?}"
        );
        assert_eq!(
            known.get(position),
            wanted != Truth::Unknown,
            "known{position} at {current:?}"
        );
        assert_eq!(
            Expression::parse(&format!("!{position}"))
                .expect("live NOT expression")
                .evaluate(truth, known),
            match wanted {
                Truth::True => Truth::False,
                Truth::False => Truth::True,
                Truth::Unknown => Truth::Unknown,
            },
            "NOT{position} at {current:?}"
        );
    }
    prefix.push(current);
    expected
}

#[test]
fn all_25_session_positions_match_independent_truth_known_and_not_oracles() {
    let mut actual = evaluator(pinned());
    let mut prefix = Vec::new();
    let mut seen = [[false; 2]; 25];
    // Each direction history, shape threshold equality, wick, fresh day type,
    // gap side and clock window is visited using only positive integer prices.
    let shapes = [
        [100, 200, 50, 100],
        [100, 110, 90, 105],
        [100, 110, 90, 105],
        [100, 110, 90, 105],
        [105, 110, 90, 100],
        [105, 110, 90, 100],
        [105, 110, 90, 100],
        [100, 110, 90, 105],
        [105, 110, 90, 100],
        [100, 110, 90, 105],
        [100, 110, 90, 100],
        [100, 200, 100, 110],
        [100, 200, 100, 130],
        [100, 200, 100, 160],
        [100, 200, 100, 200],
        [100, 200, 100, 100],
        [100, 100, 100, 100],
    ];
    let mut fixtures = Vec::new();
    for (i, prices) in shapes.into_iter().enumerate() {
        fixtures.push(bar(
            30_000,
            i64::try_from(i).expect("small fixture"),
            prices,
        ));
    }
    for (day, opening) in [(30_001, 120), (30_002, 100), (30_003, 80)] {
        fixtures.push(bar(day, 0, [opening, opening, opening, opening]));
        for (minute, prices) in [
            (1, [110, 130, 70, 110]),
            (2, [109, 130, 70, 109]),
            (3, [111, 130, 70, 111]),
            (59, [100, 200, 50, 200]),
            (60, [100, 220, 40, 40]),
            (164, [100, 120, 80, 100]),
            (165, [100, 120, 80, 100]),
            (284, [100, 120, 80, 100]),
            (285, [100, 120, 80, 100]),
            (374, [100, 120, 80, 100]),
        ] {
            fixtures.push(bar(day, minute, prices));
        }
    }
    for current in fixtures {
        for (observations, wanted) in
            seen.iter_mut()
                .zip(check(&mut actual, &mut prefix, current, pinned()))
        {
            let [saw_true, saw_false] = observations;
            match wanted {
                Truth::True => *saw_true = true,
                Truth::False => *saw_false = true,
                Truth::Unknown => {}
            }
        }
    }
    for (position, observations) in POSITIONS.into_iter().zip(seen) {
        assert_eq!(
            observations,
            [true, true],
            "both answers actually exercised for position{position}"
        );
    }
}

#[test]
fn prior_three_excludes_current_and_resets_even_when_the_search_column_is_warm() {
    let mut actual = evaluator(pinned());
    for day in 30_000..30_006 {
        for minute in 0..40 {
            actual
                .step_known(&bar(day, minute, [100, 110, 90, 105]))
                .expect("valid warm-up");
        }
    }
    assert!(actual.warmed_up());
    let bars: Vec<_> = (0..5)
        .map(|minute| bar(30_006, minute, [100, 110, 90, 100]))
        .collect();
    let column = Column::build(&bars, &mut actual);
    assert_eq!(column.len(), 5);
    assert_eq!(column.census().refused(), 0);
    let expression = Expression::parse("!37 & !38 & !39").expect("flat prior-three predicate");
    let mut hits = 0;
    for ((source, truth), known) in column
        .sources()
        .iter()
        .zip(column.bits())
        .zip(column.known())
    {
        for position in 37..=39 {
            assert!(
                !truth.get(position),
                "flat bars are neither a direction nor an alternation"
            );
            assert_eq!(
                known.get(position),
                *source >= 3,
                "exact pre-current history"
            );
        }
        let wanted = if *source >= 3 {
            Truth::True
        } else {
            Truth::Unknown
        };
        assert_eq!(expression.evaluate(*truth, *known), wanted);
        hits += usize::from(wanted == Truth::True);
    }
    assert_eq!(
        hits, 2,
        "the third current bar cannot supply its own third prior bar"
    );
}

#[test]
fn no_previous_session_and_flat_open_keep_gap_predicates_unknown() {
    let mut actual = evaluator(pinned());
    let mut prefix = Vec::new();
    for current in [
        bar(30_000, 0, [100, 100, 100, 100]),
        bar(30_000, 1, [100, 120, 90, 100]),
        bar(30_001, 0, [100, 100, 100, 100]),
        bar(30_001, 1, [100, 130, 80, 110]),
    ] {
        check(&mut actual, &mut prefix, current, pinned());
    }
    let (truth, known) = actual
        .step_known(&bar(30_001, 2, [110, 130, 80, 110]))
        .expect("flat-open continuation");
    for position in 66..=68 {
        assert!(
            !truth.get(position) && !known.get(position),
            "no invented midpoint for a flat open"
        );
    }
    assert!(known.get(48) && known.get(49));
    assert!(!truth.get(48) && !truth.get(49));
}

#[test]
fn gap_midpoint_equality_odd_rounding_wrong_family_and_zero_width_are_exact() {
    for tolerance in [
        pinned(),
        Tolerance::from_milli_on(Base::SessionRange, 0).expect("exact-equality width"),
        vocab::tolerance::pinned_pivot().expect("wrong family"),
        Tolerance::from_milli(10).expect("width with no declared family"),
    ] {
        let mut actual = evaluator(tolerance);
        let mut prefix = Vec::new();
        check(
            &mut actual,
            &mut prefix,
            bar(30_000, 0, [100, 120, 80, 100]),
            tolerance,
        );
        for (minute, prices) in [
            (0, [121, 121, 121, 121]),
            (1, [109, 122, 100, 109]),
            (2, [110, 122, 100, 110]),
            (3, [111, 122, 100, 111]),
        ] {
            check(
                &mut actual,
                &mut prefix,
                bar(30_001, minute, prices),
                tolerance,
            );
        }
    }
}

#[test]
fn zero_range_and_positive_extremes_preserve_exact_descriptions_without_overflow() {
    let mut actual = evaluator(pinned());
    let mut prefix = Vec::new();
    for current in [
        bar(30_000, 0, [1, 1, 1, 1]),
        bar(30_000, 1, [1, i64::MAX, 1, 1]),
        bar(30_001, 0, [i64::MAX, i64::MAX, 1, i64::MAX]),
        bar(30_001, 1, [i64::MAX / 2, i64::MAX, 1, i64::MAX / 2]),
    ] {
        check(&mut actual, &mut prefix, current, pinned());
    }
}

#[test]
fn refusals_do_not_publish_or_warm_any_session_reference() {
    let mut actual = evaluator(pinned());
    for minute in 0..2 {
        actual
            .step_known(&bar(30_000, minute, [100, 110, 90, 105]))
            .expect("valid prefix");
    }
    let mut untouched = actual;
    for (prices, refusal) in [
        ([100, 80, 90, 100], Corrupt::HighBelowLow),
        ([100, 110, 90, 120], Corrupt::PriceOutsideRange),
        ([0, 0, 0, 0], Corrupt::PriceNotPositive),
        ([1, i64::MAX, i64::MIN, 1], Corrupt::RangeOverflows),
    ] {
        assert_eq!(actual.step_known(&bar(30_000, 2, prices)), Err(refusal));
    }
    assert_eq!(
        actual.step_known(&bar(30_000, 1, [100, 110, 90, 105])),
        Err(Corrupt::TimestampNotIncreasing)
    );
    for (day, minute) in [(30_000, 2), (30_000, 3), (30_001, 0), (30_001, 1)] {
        let current = bar(day, minute, [100, 110, 90, 100]);
        assert_eq!(
            actual.step_known(&current).expect("valid continuation"),
            untouched
                .step_known(&current)
                .expect("untouched continuation")
        );
    }
}

#[test]
fn newly_available_false_descriptions_compose_without_promoting_absent_history() {
    let mut actual = evaluator(pinned());
    let (truth, known) = actual
        .step_known(&bar(30_000, 0, [100, 110, 90, 100]))
        .expect("positive-range flat first bar");
    assert!(known.get(33) && !truth.get(33));
    assert!(!known.get(37) && !known.get(66));
    for (source, wanted) in [
        ("!33 & 32", Truth::True),
        ("!33 | 37", Truth::True),
        ("33 & !37", Truth::False),
        ("!33 & !37", Truth::Unknown),
        ("33 | !66", Truth::Unknown),
        ("!48", Truth::Unknown),
    ] {
        assert_eq!(
            Expression::parse(source)
                .expect("valid Boolean expression")
                .evaluate(truth, known),
            wanted,
            "{source}"
        );
    }
}

#[test]
fn a_non_regular_completed_session_does_not_replace_the_previous_session_reference() {
    let mut actual = Evaluator::with_calendar(
        Widths::pinned().expect("pinned widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::charter(),
    );
    let regular = bar(20_381, 0, [100, 120, 80, 100]);
    actual.step_known(&regular).expect("regular anchor");
    actual
        .step_known(&bar(20_382, 0, [1000, 1200, 800, 1000]))
        .expect("non-regular bars still evaluate");
    let current = bar(20_383, 0, [110, 115, 105, 105]);
    let (truth, known) = actual.step_known(&current).expect("next regular day");
    let oracle = expected(&[regular], &current, pinned());
    for (position, wanted) in POSITIONS
        .into_iter()
        .zip(oracle)
        .filter(|(position, _)| *position >= 48)
    {
        assert_eq!(
            truth.get(position),
            wanted == Truth::True,
            "regular reference truth{position}"
        );
        assert_eq!(
            known.get(position),
            wanted != Truth::Unknown,
            "regular reference known{position}"
        );
    }
    assert!(truth.get(48) && truth.get(50) && truth.get(68));
    assert!(
        !truth.get(49),
        "the skipped high close must not fabricate a gap down"
    );
}
