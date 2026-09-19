#![cfg(test)]
//! Generated boundary fixtures for the actual timestamp checks, not market evidence.
#![allow(clippy::expect_used)]

use super::{SliceFacts, SquareOff, actual_ist_day, actual_square_off, horizon_bar};
use indicators::Candle;

const MINUTE: i64 = 60_000_000;
const DAY: i64 = 86_400_000_000;
const OFFSET: i64 = 19_800_000_000;

fn bar(stamp: i64) -> Candle {
    Candle::new(stamp, 100, 101, 99, 100, 1, indicators::OI_NULL)
}

fn stamp(day: i64, minute: i64) -> i64 {
    day * DAY + minute * MINUTE - OFFSET
}

#[test]
fn actual_day_matches_civil_interval_membership_at_signed_extremes_and_midnight() {
    for value in [
        i64::MIN,
        i64::MIN + DAY,
        -OFFSET - 1,
        -OFFSET,
        -OFFSET + 1,
        -1,
        0,
        DAY - OFFSET - 1,
        DAY - OFFSET,
        i64::MAX - DAY,
        i64::MAX,
    ] {
        let day = actual_ist_day(value);
        let first = day * i128::from(DAY) - i128::from(OFFSET);
        assert!(
            first <= i128::from(value) && i128::from(value) < first + i128::from(DAY),
            "{value} must belong to exactly the returned civil-day interval"
        );
    }
    for day in [-10, -1, 0, 1, 10] {
        let midnight = stamp(day, 0);
        assert_eq!(actual_ist_day(midnight - 1), i128::from(day - 1));
        assert_eq!(actual_ist_day(midnight), i128::from(day));
    }
}

#[test]
fn square_off_requires_available_ordered_actual_endpoints_and_a_representable_deadline() {
    let bars = [bar(stamp(0, 600)), bar(stamp(0, 909))];
    let last = SquareOff { bar: 1, real: true };
    assert!(actual_square_off(&bars, 0, last, MINUTE));
    assert!(
        actual_square_off(&bars, 1, last, MINUTE),
        "entry may be the actual final interval"
    );
    assert!(!actual_square_off(&[], 0, last, MINUTE));
    assert!(!actual_square_off(&bars, usize::MAX, last, MINUTE));
    assert!(!actual_square_off(
        &bars,
        0,
        SquareOff {
            bar: usize::MAX,
            real: true
        },
        MINUTE
    ));
    let reverse = [bar(stamp(0, 909)), bar(stamp(0, 600))];
    assert!(
        !actual_square_off(&reverse, 1, SquareOff { bar: 0, real: true }, MINUTE),
        "clock order cannot excuse a backwards source index"
    );
    assert!(
        !actual_square_off(&reverse, 0, last, MINUTE),
        "source-index order cannot excuse backwards clock time"
    );
    let extreme = [bar(i64::MAX - i64::MAX.rem_euclid(MINUTE))];
    assert!(
        !actual_square_off(
            &extreme,
            0,
            SquareOff {
                bar: 0,
                real: false
            },
            MINUTE
        ),
        "a civil cutoff beyond the signed timestamp domain has no receipt"
    );
}

#[test]
fn square_off_exact_intervals_reject_foreign_days_subminutes_and_late_entry() {
    for day in [-1, 0, 20_000] {
        for real in [false, true] {
            for (start, end, step, expected) in [
                (600 * MINUTE, 909 * MINUTE, MINUTE, true),
                (909 * MINUTE, 909 * MINUTE, MINUTE, true),
                (910 * MINUTE, 910 * MINUTE, MINUTE, false),
                (909 * MINUTE + 1, 909 * MINUTE + 1, MINUTE, false),
                (600 * MINUTE + 30_000_000, 909 * MINUTE, MINUTE, false),
                (600 * MINUTE, 908 * MINUTE + 1, MINUTE, false),
                (600 * MINUTE, 909 * MINUTE + 30_000_000, MINUTE, false),
                (600 * MINUTE, DAY + 600 * MINUTE, MINUTE, false),
                (600 * MINUTE, 909 * MINUTE, 0, false),
                (600 * MINUTE, 909 * MINUTE, -MINUTE, false),
                (600 * MINUTE, 909 * MINUTE, i64::MAX, false),
                (600 * MINUTE, 909 * MINUTE, 2 * MINUTE, false),
            ] {
                let origin = stamp(day, 0);
                let bars = [bar(origin + start), bar(origin + end)];
                assert_eq!(
                    actual_square_off(&bars, 0, SquareOff { bar: 1, real }, step),
                    expected,
                    "day={day} start={start} end={end} step={step} real={real}"
                );
            }
        }
    }
}

#[test]
fn truncated_boundaries_allow_earlier_holds_without_forging_forced_one_minute_fills() {
    for step in [MINUTE, 2 * MINUTE, 60 * MINUTE] {
        let bars = [bar(stamp(1, 600)), bar(stamp(1, 700))];
        assert!(actual_square_off(
            &bars,
            0,
            SquareOff {
                bar: 1,
                real: false
            },
            step
        ));
        assert!(!actual_square_off(
            &bars,
            0,
            SquareOff { bar: 1, real: true },
            step
        ));
    }
    let bars = [bar(stamp(1, 600)), bar(stamp(1, 908))];
    assert!(actual_square_off(
        &bars,
        0,
        SquareOff {
            bar: 1,
            real: false
        },
        2 * MINUTE
    ));
    assert!(
        !actual_square_off(&bars, 0, SquareOff { bar: 1, real: true }, 2 * MINUTE),
        "a two-minute record ending at15:10 is not the required15:09 minute"
    );
}

#[test]
fn horizon_lookup_requires_positive_cadence_exact_actual_timestamp_and_present_index() {
    let bars = [bar(stamp(1, 600)), bar(stamp(1, 601)), bar(stamp(1, 602))];
    let mut facts = SliceFacts {
        exits: Vec::new(),
        step_micros: MINUTE,
        accepted: None,
        refused_prefix: Vec::new(),
        broken_prefix: Vec::new(),
        at_timestamp: [(stamp(1, 602), 2)].into_iter().collect(),
    };
    assert_eq!(horizon_bar(&bars, 0, 2, MINUTE, &facts), Some(2));
    for step in [0, -MINUTE] {
        assert_eq!(horizon_bar(&bars, 0, 2, step, &facts), None);
    }
    assert_eq!(horizon_bar(&bars, usize::MAX, 2, MINUTE, &facts), None);
    assert_eq!(horizon_bar(&bars, 0, 1, MINUTE, &facts), None);
    facts.at_timestamp.insert(stamp(1, 602), 1);
    assert_eq!(
        horizon_bar(&bars, 0, 2, MINUTE, &facts),
        None,
        "foreign earlier index"
    );
    facts.at_timestamp.insert(stamp(1, 602), usize::MAX);
    assert_eq!(
        horizon_bar(&bars, 0, 2, MINUTE, &facts),
        None,
        "foreign absent index"
    );
    assert_eq!(horizon_bar(&bars, 0, usize::MAX, MINUTE, &facts), None);
}
