//! EVERY INTRADAY RUNG OPENS THE SESSION AT 09:15, and the daily rung does not.
//!
//! The NSE open is 555 minutes past IST midnight, so a grid anchored at
//! midnight lands on 09:15 only where the rung divides 555 — three, five and
//! fifteen do; two, ten, thirty and sixty do not. Under that anchor each of
//! those four opened the day with a bar stamped BEFORE the open holding part of
//! the session, which is why `store_timeframe` refused thirty and sixty and why
//! the operator's ladder — 2, 3, 5, 10, 15, 30, 60 — could not exist.
//!
//! `crate::fold` anchors an intraday rung at the OPEN. This asserts it against a
//! real session rather than against the arithmetic that motivated it, because a
//! sign error in that arithmetic is invisible on seven rungs out of eight: the
//! first draft added the offset where it had to subtract it, and only the hour
//! bar moved.

// A test that asserts nothing is banned, and a test that cannot fail loudly is
// a test that asserts nothing.
#![allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing
)]

use pull::fold::{Bucket, fold};
use store::format::Bar;

/// 2025-07-01 09:15:00 IST, as a UTC epoch second. A Tuesday.
const OPEN_UTC: i64 = 1_751_341_500;
/// 09:15 to 15:30 inclusive of the open, exclusive of the close.
const SESSION_MINUTES: i64 = 375;

fn minute(ts_secs: i64) -> Bar {
    Bar {
        ts_micros: ts_secs * 1_000_000,
        open: 100,
        high: 100,
        low: 100,
        close: 100,
        volume: 1,
        open_interest: i64::MIN,
    }
}

/// One one-minute bar for every minute the session holds.
fn session() -> Vec<Bar> {
    (0..SESSION_MINUTES)
        .map(|m| minute(OPEN_UTC + m * 60))
        .collect()
}

/// THE FIRST BAR OF EVERY INTRADAY RUNG STARTS AT THE OPEN. No exceptions, and
/// the four that do not divide 555 are the point of the test.
#[test]
fn every_intraday_rung_opens_the_session_exactly_at_the_open() {
    for (name, secs) in [
        ("1min", 60_u32),
        ("2min", 120),
        ("3min", 180),
        ("5min", 300),
        ("10min", 600),
        ("15min", 900),
        ("30min", 1_800),
        ("60min", 3_600),
    ] {
        let bucket = Bucket::of_secs(secs).expect("a real width");
        let out = fold(&session(), bucket).expect("a session in order");
        let first = out.first().expect("a session yields bars").ts_micros / 1_000_000;
        assert_eq!(
            first,
            OPEN_UTC,
            "{name} opens the day at {}s from the open, not at it — a bar \
             stamped before 09:15 holds part of a session in a record whose \
             header claims a whole one",
            first - OPEN_UTC
        );
    }
}

/// WHAT IS RAGGED IS THE LAST BAR, NOT THE FIRST, and the count says which.
///
/// 375 does not divide 2, 10, 30 or 60, so those four end the session with a
/// short bar. That is a different object from the leading stub they used to
/// have: it is stamped correctly and holds the trades that happened in it.
#[test]
fn a_rung_that_does_not_tile_the_session_is_short_at_the_end_not_the_start() {
    for (name, secs, bars) in [
        // 375 / n is exact: the session tiles.
        ("1min", 60_u32, 375_usize),
        ("3min", 180, 125),
        ("5min", 300, 75),
        ("15min", 900, 25),
        // 375 / n is not: one extra, short, final bar.
        ("2min", 120, 188),
        ("10min", 600, 38),
        ("30min", 1_800, 13),
        ("60min", 3_600, 7),
    ] {
        let bucket = Bucket::of_secs(secs).expect("a real width");
        let out = fold(&session(), bucket).expect("a session in order");
        assert_eq!(out.len(), bars, "{name} folded a 375-minute session wrong");
        let last = out.last().expect("bars").ts_micros / 1_000_000;
        assert!(
            last < OPEN_UTC + SESSION_MINUTES * 60,
            "{name}'s last bar starts after the session ended"
        );
    }
}

/// THE DAILY RUNG KEEPS THE MIDNIGHT ANCHOR, and it is not an oversight.
///
/// A day-wide bucket anchored at the open would run 09:15 to 09:15 — one "day"
/// spanning two calendar dates, which is not what `DAY_1` means anywhere else.
/// Asserted by the bar landing on the IST day's own midnight rather than on the
/// open.
#[test]
fn the_daily_rung_is_anchored_at_ist_midnight_and_not_at_the_open() {
    let out = fold(&session(), Bucket::of_secs(86_400).expect("a day")).expect("in order");
    assert_eq!(out.len(), 1, "a session is one daily bar");
    let at = out[0].ts_micros / 1_000_000;
    assert_ne!(at, OPEN_UTC, "a daily bar is not stamped at the open");
    // IST midnight of the session's own day: the open, less 555 minutes.
    assert_eq!(
        at,
        OPEN_UTC - 555 * 60,
        "a daily bar is stamped at the IST midnight its session fell in"
    );
}
