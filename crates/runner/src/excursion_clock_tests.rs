#![cfg(test)]
//! Finite clock/membership/price admission cases on generated candles.
#![allow(clippy::expect_used)]

use super::{ActualClock, Crossings, Ladder, Ladders, PathAdmission, admit};
use indicators::Candle;

const MINUTE: i64 = 60_000_000;

fn bar(stamp: i64) -> Candle {
    Candle::new(stamp, 100, 101, 99, 100, 1, indicators::OI_NULL)
}

#[test]
fn clock_accepts_inclusive_whole_minute_endpoints_and_never_duplicate_or_reversed_time() {
    let bars = [bar(-2 * MINUTE), bar(2 * MINUTE)];
    let mut clock = ActualClock::new(&bars, 0, 1);
    for value in [-2 * MINUTE, -MINUTE, 0, MINUTE, 2 * MINUTE] {
        assert!(clock.accepts(value));
    }
    assert!(!clock.accepts(2 * MINUTE), "duplicate exact endpoint");
    assert!(!clock.accepts(MINUTE), "backwards actual timestamp");
    for value in [-3 * MINUTE, 3 * MINUTE, -MINUTE + 1, 1, 30_000_000] {
        assert!(!ActualClock::new(&bars, 0, 1).accepts(value), "{value}");
    }
    assert!(
        ActualClock::new(&bars, 0, 1).accepts(-2 * MINUTE),
        "new path resets prior time"
    );
}

#[test]
fn clock_missing_or_reversed_endpoints_cannot_admit_an_actual_record() {
    let bars = [bar(MINUTE), bar(2 * MINUTE)];
    for (from, to) in [(0, 2), (2, 0), (usize::MAX, usize::MAX), (1, 0)] {
        let mut clock = ActualClock::new(&bars, from, to);
        for value in [0, MINUTE, 2 * MINUTE, 3 * MINUTE] {
            assert!(!clock.accepts(value), "from={from} to={to} value={value}");
        }
    }
    assert!(!ActualClock::new(&[], 0, 0).accepts(0));
}

#[test]
fn admission_keeps_clock_membership_and_candle_refusals_out_of_extremes() {
    let ladder = Ladder::new(vec![100]).expect("positive ladder");
    let ladders = Ladders {
        stops: &ladder,
        targets: &ladder,
        trails: &ladder,
    };
    let bounds = [bar(0), bar(2 * MINUTE)];
    let yes = [true];
    let no = [false];
    let mut corrupt = bar(MINUTE);
    corrupt.high = 98;
    for (record, accepted, index, expected) in [
        (bar(MINUTE), Some(yes.as_slice()), 0, true),
        (bar(MINUTE), Some(no.as_slice()), 0, false),
        (bar(MINUTE), Some(yes.as_slice()), 1, false),
        (bar(MINUTE + 1), Some(yes.as_slice()), 0, false),
        (bar(3 * MINUTE), Some(yes.as_slice()), 0, false),
        (corrupt, Some(yes.as_slice()), 0, false),
        (corrupt, None, 0, false),
        (bar(MINUTE + 1), None, 0, true),
    ] {
        for priced in [false, true] {
            let mut out = Crossings::empty(ladders);
            let mut path = PathAdmission::new(&bounds, 0, 1);
            assert_eq!(
                admit(&mut out, &mut path, &record, (accepted, index), priced),
                expected && priced
            );
            assert_eq!(out.refused, usize::from(!expected && priced));
            assert_eq!(
                out.low_run,
                vec![if expected { record.low } else { i64::MAX }]
            );
            assert_eq!(
                out.high_run,
                vec![if expected { record.high } else { i64::MIN }]
            );
        }
    }
}

#[test]
fn admission_counts_duplicate_clock_once_and_preserves_extremes_offset_alignment() {
    let ladder = Ladder::new(vec![100]).expect("positive ladder");
    let ladders = Ladders {
        stops: &ladder,
        targets: &ladder,
        trails: &ladder,
    };
    let bars = [
        bar(0),
        Candle::new(0, 100, 10_000, 1, 100, 1, indicators::OI_NULL),
        bar(MINUTE),
    ];
    let mut out = Crossings::empty(ladders);
    let mut path = PathAdmission::new(&bars, 0, 2);
    let accepted = [true; 3];
    for (index, record) in bars.iter().enumerate() {
        assert_eq!(
            admit(&mut out, &mut path, record, (Some(&accepted), index), true),
            index != 1
        );
    }
    assert_eq!(out.refused, 1);
    assert_eq!(out.low_run, vec![99; 3]);
    assert_eq!(out.high_run, vec![101; 3]);
    out.refused = usize::MAX;
    assert!(!admit(
        &mut out,
        &mut path,
        &bar(MINUTE),
        (Some(&[]), 0),
        true
    ));
    assert_eq!(
        out.refused,
        usize::MAX,
        "a counter cannot wrap into successful admission"
    );
}
