//! Finite generated crossing/ordinal oracles. These do not attest market data.

#![allow(clippy::expect_used, reason = "fixture premises must fail explicitly")]

use indicators::column::Column;
use indicators::evaluator::{Calendar, Evaluator, Widths};
use indicators::pattern::Thresholds;
use indicators::vwap::Availability;
use indicators::{Candle, Corrupt};
use vocab::expression::{Expression, Truth};
use vocab::table::{CROSSINGS, LevelCrossing};

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;

fn evaluator() -> Evaluator {
    Evaluator::with_calendar(
        Widths::pinned().expect("usable widths"),
        Availability::Absent,
        Thresholds::CLASSICAL,
        Calendar::all_regular(),
    )
}

fn candle(day: i64, minute: i64, open: i64, close: i64) -> Candle {
    Candle::new(
        day * DAY + (225 + minute) * MINUTE,
        open,
        open.max(close) + 100,
        open.min(close) - 100,
        close,
        0,
        i64::MIN,
    )
}

fn positions(level: &LevelCrossing) -> [u16; 5] {
    [level.up, level.down, level.first, level.second, level.later]
}

#[test]
#[allow(
    clippy::too_many_lines,
    reason = "one independent stateful oracle spans all mapped levels"
)]
fn every_crossing_and_ordinal_matches_a_last_definite_side_oracle_including_known_non_events() {
    let mut actual = evaluator();
    let mut prior = [None; CROSSINGS.len()];
    let mut counts = [0_u64; CROSSINGS.len()];
    let mut known_false = [0_u64; CROSSINGS.len()];
    let negations: Vec<_> = CROSSINGS
        .iter()
        .map(|level| {
            positions(level)
                .map(|bit| Expression::parse(&format!("!{bit}")).expect("live crossing or ordinal"))
        })
        .collect();
    let mut event_count = 0_u64;
    let mut unavailable_count = 0_u64;
    for day in 0..8 {
        prior.fill(None);
        counts.fill(0);
        for minute in 0..375 {
            let centre = 2_000_000 + day * 2_000_000;
            let close = centre + ((minute * 73) % 401 - 200) * 3_000;
            let open = if minute == 0 {
                centre + 800_000
            } else {
                close - 10
            };
            let (truth, known) = actual
                .step_known(&candle(30_000 + day, minute, open, close))
                .expect("positive ordered fixture");
            for ((((level, prior), count), known_false), negations) in CROSSINGS
                .iter()
                .zip(&mut prior)
                .zip(&mut counts)
                .zip(&mut known_false)
                .zip(&negations)
            {
                let above = truth.get(u32::from(level.above));
                let below = truth.get(u32::from(level.below));
                assert!(
                    !(above && below),
                    "a real two-sided level cannot be on both sides"
                );
                let now = if above {
                    Some(true)
                } else if below {
                    Some(false)
                } else {
                    None
                };
                let event = now.zip(*prior).filter(|(a, b)| a != b).map(|(a, _)| a);
                let available = prior.is_some()
                    && known.get(u32::from(level.above))
                    && known.get(u32::from(level.below));
                if event.is_some() {
                    *count += 1;
                    event_count += 1;
                }
                let expected = [
                    event == Some(true),
                    event == Some(false),
                    event.is_some() && *count == 1,
                    event.is_some() && *count == 2,
                    event.is_some() && *count >= 3,
                ];
                for ((bit, expected_truth), negation) in
                    positions(level).into_iter().zip(expected).zip(negations)
                {
                    let bit = u32::from(bit);
                    assert_eq!(
                        truth.get(bit),
                        expected_truth,
                        "bit{bit} day{day} minute{minute}"
                    );
                    let expected_known = expected_truth || available;
                    assert_eq!(
                        known.get(bit),
                        expected_known,
                        "known bit{bit} day{day} minute{minute}"
                    );
                    assert_eq!(
                        negation.evaluate(truth, known),
                        if expected_truth {
                            Truth::False
                        } else if available {
                            Truth::True
                        } else {
                            Truth::Unknown
                        },
                        "NOT bit{bit} day{day} minute{minute}"
                    );
                    *known_false += u64::from(available && !expected_truth);
                    unavailable_count += u64::from(!expected_known);
                }
                if now.is_some() {
                    *prior = now;
                }
            }
        }
    }
    assert!(
        known_false.into_iter().all(|count| count > 0),
        "every mapped level has usable non-events"
    );
    assert!(
        event_count > 100,
        "the fixture exercises actual transitions"
    );
    assert!(
        unavailable_count > 100,
        "missing prerequisites must remain unknown"
    );
}

#[test]
fn a_touch_is_known_false_but_cannot_erase_the_side_or_count_across_it() {
    let mut actual = evaluator();
    // Day-open comparison: the first bar does not define its own prior anchor.
    // The second seeds a side, then quiet and touch bars are usable non-events.
    let closes = [1_000, 1_010, 1_020, 1_000, 990, 1_000, 1_010, 990, 990];
    let expected_cross = [
        None,
        None,
        None,
        None,
        Some(false),
        None,
        Some(true),
        Some(false),
        None,
    ];
    let expected_count = [0, 0, 0, 0, 1, 1, 2, 3, 3];
    let level = CROSSINGS.last().expect("session-open crossing");
    assert_eq!((level.above, level.below), (276, 277));
    for (index, ((close, event), count)) in closes
        .into_iter()
        .zip(expected_cross)
        .zip(expected_count)
        .enumerate()
    {
        let (truth, known) = actual
            .step_known(&candle(
                30_000,
                i64::try_from(index).expect("nine rows"),
                1_000,
                close,
            ))
            .expect("valid touch walk");
        let expected = [
            event == Some(true),
            event == Some(false),
            event.is_some() && count == 1,
            event.is_some() && count == 2,
            event.is_some() && count >= 3,
        ];
        for (bit, wanted) in positions(level).into_iter().zip(expected) {
            assert_eq!(truth.get(u32::from(bit)), wanted);
            assert_eq!(known.get(u32::from(bit)), index >= 2);
        }
    }
    let (truth, known) = actual
        .step_known(&candle(30_001, 0, 1_000, 990))
        .expect("new day");
    for bit in positions(level) {
        assert!(!truth.get(u32::from(bit)));
        assert!(
            !known.get(u32::from(bit)),
            "yesterday cannot establish today's prior side"
        );
    }
}

#[test]
fn available_crossing_negation_survives_column_handoff_and_refusal_is_transactional() {
    let mut actual = evaluator();
    for day in 30_000..30_006 {
        for minute in 0..50 {
            actual
                .step_known(&candle(day, minute, 1_000, 1_010))
                .expect("warm-up");
        }
    }
    assert!(actual.warmed_up());
    let bars: Vec<_> = (0..10)
        .map(|minute| candle(30_006, minute, 1_000, 1_010))
        .collect();
    let column = Column::build(&bars, &mut actual);
    assert_eq!(column.len(), 10);
    let not_cross = Expression::parse("!312 & !313").expect("no day-open crossing");
    for (row, (truth, known)) in column.bits().iter().zip(column.known()).enumerate() {
        assert_eq!(
            not_cross.evaluate(*truth, *known),
            if row >= 2 {
                Truth::True
            } else {
                Truth::Unknown
            }
        );
    }
    let mut untouched = actual;
    let mut broken = candle(30_006, 10, 1_000, 990);
    broken.low = 1_200;
    assert_eq!(actual.step_known(&broken), Err(Corrupt::HighBelowLow));
    for next in [
        candle(30_006, 10, 1_000, 990),
        candle(30_007, 0, 1_000, 1_010),
    ] {
        assert_eq!(actual.step_known(&next), untouched.step_known(&next));
    }
}
