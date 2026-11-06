#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::market::Bar;
use crate::time::CLOSE_FULL;

fn p(start: u32, high: i64, low: i64) -> Print {
    Print {
        start,
        slot: start,
        high,
        low,
    }
}

#[test]
fn t_f30_f34_entry_worst_print_and_the_stop() {
    let prints = [p(OPEN + 1, 105, 95), p(OPEN + 61, 110, 90)];
    assert_eq!(entry(&prints, OPEN, Side::Long), Some((0, 105)));
    assert_eq!(entry(&prints, OPEN, Side::Short), Some((0, 95)));
    assert_eq!(entry(&prints, OPEN + 2, Side::Long), Some((1, 110)));
    assert_eq!(entry(&prints, OPEN + 62, Side::Long), None);
    assert!(reaches(90, &prints[1], Side::Long) && !reaches(89, &prints[1], Side::Long));
    assert!(reaches(110, &prints[1], Side::Short) && !reaches(111, &prints[1], Side::Short));
    assert_eq!(exit_price(&prints[1], Side::Long), 90);
    assert_eq!(exit_price(&prints[1], Side::Short), 110);
    assert!(entry_buys(Side::Long) && !entry_buys(Side::Short));
    assert_eq!(worst(&prints[0], true), 105);
}

/// X-F34, X-F37, X-F38: the stop scan, the remainder rule and the minute prints at their edges.
#[test]
fn x_fills_extremes() {
    let prints = [
        p(100, 105, 95),
        p(101, 104, 80),
        p(102, 120, 100),
        p(103, 100, 100),
    ];
    let mut scan = StopScan {
        stop: 96,
        side: Side::Long,
        cursor: 1,
    };
    // a gap through: the low far below the stop fills there in full
    assert_eq!(scan.until(&prints, 200), Some(1));
    assert_eq!(scan.cursor, 2);
    // the limit stops the scan before a later print
    let mut short = StopScan {
        stop: 121,
        side: Side::Short,
        cursor: 1,
    };
    assert_eq!(short.until(&prints, 103), None);
    assert_eq!(short.cursor, 3);
    let mut equal = StopScan {
        stop: 120,
        side: Side::Short,
        cursor: 2,
    };
    assert!(equal.step(&prints[2]));
    // worst since the entry includes the entry print (Q5)
    assert_eq!(worst_since(&prints, 0, Side::Long), Some(80));
    assert_eq!(worst_since(&prints, 2, Side::Short), Some(120));
    assert_eq!(worst_since(&prints, 3, Side::Long), Some(100));
    // combo: past the end
    assert_eq!(worst_since(&prints, 9, Side::Long), None);
    assert_eq!(first_at_or_after(&prints, 0), 0);
    assert_eq!(first_at_or_after(&prints, 104), 4);
    // the minute prints of a fallback day: traded, in session, slot = start + 59
    let minutes = [
        Minute {
            start: OPEN - 60,
            bar: Bar::flat(7, 5),
        },
        Minute {
            start: OPEN,
            bar: Bar::new(10, 12, 9, 11, 5),
        },
        Minute {
            start: OPEN + 60,
            bar: Bar::flat(11, 0),
        },
        Minute {
            start: CLOSE_FULL,
            bar: Bar::flat(7, 5),
        },
    ];
    let mp = minute_prints(&minutes, CLOSE_FULL);
    assert_eq!(
        mp,
        vec![Print {
            start: OPEN,
            slot: OPEN + 59,
            high: 12,
            low: 9
        }]
    );
    let seconds = [Second {
        at: OPEN + 3,
        bar: Bar::new(10, 11, 9, 10, 0),
    }];
    assert_eq!(
        second_prints(&seconds),
        vec![Print {
            start: OPEN + 3,
            slot: OPEN + 3,
            high: 11,
            low: 9
        }]
    );
}
