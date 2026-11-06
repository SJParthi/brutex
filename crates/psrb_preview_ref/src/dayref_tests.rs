#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "the same exception every test module in this workspace takes: a test that cannot panic cannot fail."
)]

use super::*;
use crate::generator::day;
use crate::market::Bar;
use crate::time::CLOSE_FULL;

fn minute(i: u32, bar: Bar) -> Minute {
    Minute {
        start: OPEN + 60 * i,
        bar,
    }
}

#[test]
fn t_f11_the_day_bar_is_the_session_high_low_and_last_traded_close() {
    let d = day(2018, 9, 4);
    let minutes = [
        minute(0, Bar::new(100, 105, 99, 101, 5)),
        minute(1, Bar::new(101, 110, 100, 108, 5)),
        minute(2, Bar::new(108, 108, 108, 108, 0)),
    ];
    let bar = day_bar(d, &minutes).unwrap();
    assert_eq!((bar.high(), bar.low(), bar.close()), (110, 99, 108));
    let mut prev = PrevSession::default();
    assert_eq!(prev.reference(), None);
    prev.close_day(Some(bar));
    assert_eq!(prev.reference(), Some(bar));
}

/// X-F11: the previous session at its edges.
#[test]
fn x_f11_prev_session_extremes() {
    let d = day(2018, 9, 4);
    // Q2: only padded minutes: no day bar, C1 and C2 false all day.
    assert_eq!(day_bar(d, &[minute(0, Bar::flat(100, 0))]), None);
    assert_eq!(day_bar(d, &[]), None);
    // oob: an inverted stored bar is refused.
    assert_eq!(
        day_bar(d, &[minute(0, Bar::new(100, 90, 110, 100, 5))]),
        None
    );
    assert_eq!(AcceptedBar::from_hlc(90, 110, 100), None);
    assert_eq!(AcceptedBar::from_hlc(110, 90, 120), None);
    assert_eq!(AcceptedBar::from_hlc(110, 0, 100), None);
    let ok = AcceptedBar::from_hlc(110, 90, 100).unwrap();
    assert_eq!((ok.high(), ok.low(), ok.close()), (110, 90, 100));
    // a refused bar ends the reference: no earlier bar is read
    let mut prev = PrevSession::default();
    prev.close_day(Some(ok));
    prev.close_day(None);
    assert_eq!(prev.reference(), None);
    // combo: a minute outside the session never enters the day bar, and an auction day
    // stops at 15:15
    let outside = [
        minute(0, Bar::new(100, 101, 99, 100, 5)),
        Minute {
            start: CLOSE_FULL,
            bar: Bar::new(100, 500, 1, 100, 5),
        },
    ];
    assert_eq!(day_bar(d, &outside).unwrap().high(), 101);
    // a pre-open minute (09:14) never enters it either: the day opens at 09:15
    let pre_open = [
        Minute {
            start: OPEN - 60,
            bar: Bar::new(100, 900, 1, 100, 5),
        },
        minute(0, Bar::new(100, 101, 99, 100, 5)),
    ];
    let bar = day_bar(d, &pre_open).unwrap();
    assert_eq!((bar.high(), bar.low()), (101, 99));
    let auction = [
        minute(0, Bar::new(100, 101, 99, 100, 5)),
        minute(359, Bar::new(100, 104, 99, 103, 5)),
    ];
    assert_eq!(day_bar(day(2026, 8, 3), &auction).unwrap().close(), 103);
}
