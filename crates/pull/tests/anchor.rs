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

#[test]
fn missing_and_duplicate_minutes_are_withheld_but_other_buckets_survive() {
    let mut bars = session();
    bars.remove(2);
    let (complete, diagnostics) =
        pull::fold::complete_minutes(&bars, Bucket::of_secs(300).unwrap()).unwrap();
    assert_eq!(complete.len(), 74);
    assert_eq!(complete[0].ts_micros, (OPEN_UTC + 300) * 1_000_000);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("observed 4, scheduled 5"));
    let mut bars = session();
    bars.insert(2, bars[1]);
    let (complete, diagnostics) =
        pull::fold::complete_minutes(&bars, Bucket::of_secs(300).unwrap()).unwrap();
    assert_eq!(complete.len(), 74);
    assert_eq!(diagnostics.len(), 1);
}

#[test]
fn an_open_tail_is_not_a_scheduled_closing_stub() {
    let bars = session();
    let bucket = Bucket::of_secs(3600).unwrap();
    let (complete, diagnostics) = pull::fold::complete_minutes(&bars[..374], bucket).unwrap();
    assert_eq!(complete.len(), 6);
    assert_eq!(diagnostics.len(), 1);
    let (complete, diagnostics) = pull::fold::complete_minutes(&bars, bucket).unwrap();
    assert_eq!(complete.len(), 7);
    assert_eq!(complete.last().unwrap().volume, 15);
    assert!(diagnostics.is_empty());
}

#[test]
fn exceptional_sessions_are_withheld_without_changing_the_stored_grid() {
    for day in [pull::calendar::SYSTEMS_OUTAGE_DAY, 19_784, 19_861, 20_382] {
        let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
            panic!("known session")
        };
        let midnight = day * 86_400 - pull::session::IST_OFFSET_SECS;
        let bars: Vec<_> = (0..1440_u16)
            .filter(|m| session.expects(*m))
            .map(|m| minute(midnight + i64::from(m) * 60))
            .collect();
        let (complete, diagnostics) =
            pull::fold::complete_minutes(&bars, Bucket::of_secs(3600).unwrap()).unwrap();
        assert!(
            complete.is_empty(),
            "{day}: exceptional session must not be certified"
        );
        assert!(
            diagnostics
                .iter()
                .any(|why| why.contains("exceptional session"))
        );
        let (short, diagnostics) =
            pull::fold::complete_minutes(&bars[1..], Bucket::of_secs(3600).unwrap()).unwrap();
        assert!(short.is_empty());
        assert!(!diagnostics.is_empty());
    }
}

#[test]
fn a_wholly_absent_bucket_is_named() {
    let mut bars = session();
    bars.drain(5..10);
    let (complete, diagnostics) =
        pull::fold::complete_minutes(&bars, Bucket::of_secs(300).unwrap()).unwrap();
    assert_eq!(complete.len(), 74);
    assert!(
        diagnostics
            .iter()
            .any(|why| why.contains("absent: observed 0, scheduled 5"))
    );
}

#[test]
fn a_multi_year_gap_does_not_scan_or_diagnose_unobserved_days() {
    let first = minute(OPEN_UTC);
    let last = minute(OPEN_UTC + 100_000 * 86_400);
    let (_, diagnostics) =
        pull::fold::complete_minutes(&[first, last], Bucket::of_secs(300).unwrap()).unwrap();
    assert_eq!(
        diagnostics
            .iter()
            .filter(|why| why.contains("unobserved cross-day range"))
            .count(),
        0
    );
    assert_eq!(
        diagnostics.len(),
        4,
        "two observed buckets, the first observed day's tail, and one unknown-day explanation, regardless of gap length"
    );
    assert!(diagnostics[0].contains("observed 1, scheduled 5"));
    let missing = (OPEN_UTC + 300) * 1_000_000;
    let close = (OPEN_UTC + SESSION_MINUTES * 60) * 1_000_000;
    assert_eq!(
        diagnostics[1],
        format!(
            "bucket range [{missing}, {close}): absent: observed 0, scheduled 370; observed-day tail withheld; historical gap refill requires a versioned store repair"
        )
    );
    assert!(
        diagnostics[2]
            .contains("calendar authority missing; absent observations do not prove closure")
    );
    assert!(diagnostics[3].contains("calendar Unmeasured"));
}

#[test]
fn an_absent_closing_bucket_is_named_before_the_next_observed_day() {
    for secs in [60_u32, 120, 180, 300, 600, 900, 1800, 3600] {
        let bucket = Bucket::of_secs(secs).unwrap();
        let width = i64::from(secs / 60);
        let tail_start = (SESSION_MINUTES - 1) / width * width;
        // Skip an entire civil day too: only the observed day's closing
        // bucket (including a scheduled stub) belongs in this diagnostic.
        let bars: Vec<_> = (0..tail_start)
            .map(|m| minute(OPEN_UTC + m * 60))
            .chain((0..SESSION_MINUTES).map(|m| minute(OPEN_UTC + 2 * 86_400 + m * 60)))
            .collect();
        let (complete, diagnostics) = pull::fold::complete_minutes(&bars, bucket).unwrap();
        assert_eq!(
            complete,
            fold(&bars, bucket).unwrap(),
            "{secs}s: no invented or lost bars"
        );
        assert_eq!(diagnostics.len(), 1, "{secs}s: {diagnostics:?}");
        let missing = (OPEN_UTC + tail_start * 60) * 1_000_000;
        let close = (OPEN_UTC + SESSION_MINUTES * 60) * 1_000_000;
        let expected = SESSION_MINUTES - tail_start;
        assert_eq!(
            diagnostics[0],
            format!(
                "bucket range [{missing}, {close}): absent: observed 0, scheduled {expected}; observed-day tail withheld; historical gap refill requires a versioned store repair"
            ),
            "{secs}s"
        );
    }
}

#[test]
fn a_final_days_absent_tail_has_no_proven_request_endpoint() {
    let bars: Vec<_> = (0..360).map(|m| minute(OPEN_UTC + m * 60)).collect();
    let bucket = Bucket::of_secs(3600).unwrap();
    let (complete, diagnostics) = pull::fold::complete_minutes(&bars, bucket).unwrap();
    assert_eq!(complete, fold(&bars, bucket).unwrap());
    assert_eq!(complete.len(), 6);
    assert!(diagnostics.is_empty());
}

#[test]
fn two_full_consecutive_sessions_have_no_overnight_failure() {
    let mut bars = session();
    bars.extend((0..SESSION_MINUTES).map(|m| minute(OPEN_UTC + 86_400 + m * 60)));
    for secs in [120, 180, 300, 600, 900, 1800, 3600] {
        let bucket = Bucket::of_secs(secs).unwrap();
        let (one_day, _) = pull::fold::complete_minutes(&session(), bucket).unwrap();
        let (complete, diagnostics) = pull::fold::complete_minutes(&bars, bucket).unwrap();
        assert!(diagnostics.is_empty(), "{secs}s: {diagnostics:?}");
        assert_eq!(complete.len(), 2 * one_day.len());
        assert_eq!(
            complete.iter().map(|bar| bar.volume).sum::<i64>(),
            2 * SESSION_MINUTES
        );
    }
}

#[test]
fn a_new_days_missing_open_is_checked_after_reset() {
    let mut bars = session();
    bars.extend((5..SESSION_MINUTES).map(|m| minute(OPEN_UTC + 86_400 + m * 60)));
    let (complete, diagnostics) =
        pull::fold::complete_minutes(&bars, Bucket::of_secs(300).unwrap()).unwrap();
    assert_eq!(complete.len(), 149);
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("absent: observed 0, scheduled 5"));
}

#[test]
fn minute_coverage_refuses_timestamp_extremes_without_overflow() {
    for ts in [i64::MIN, i64::MAX] {
        let mut bar = minute(OPEN_UTC);
        bar.ts_micros = ts;
        assert!(matches!(
            pull::fold::complete_minutes(&[bar], Bucket::of_secs(300).unwrap()),
            Err(pull::fold::FoldError::AnchorOverflow { .. })
        ));
    }
}

#[test]
fn regular_venue_minutes_follow_the_dated_close_exclusively() {
    use pull::fold::{complete_minutes_for_venue, complete_minutes_with_cash_schedule};
    use pull::session::Day;
    use pull::vendor::Venue;
    for (day, derivative_minutes) in [
        (Day::new(2026, 7, 31).unwrap(), 375_i64),
        (Day::new(2026, 8, 3).unwrap(), 385_i64),
    ] {
        let midnight = i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS;
        let open = midnight + 555 * 60;
        let mut cash_schedule = pull::cash_auction::Schedule::default();
        cash_schedule.insert(day, false).unwrap();
        for venue in Venue::ALL {
            let expected = if venue == Venue::NseDerivatives {
                derivative_minutes
            } else {
                375
            };
            let bars: Vec<_> = (0..expected).map(|m| minute(open + m * 60)).collect();
            for secs in [60, 120, 180, 300, 600, 900, 1800, 3600] {
                let bucket = Bucket::of_secs(secs).unwrap();
                let (complete, diagnostics) =
                    complete_minutes_with_cash_schedule(&bars, bucket, venue, Some(&cash_schedule))
                        .unwrap();
                assert!(
                    diagnostics.is_empty(),
                    "{day:?} {venue} {secs}: {diagnostics:?}"
                );
                assert_eq!(complete, fold(&bars, bucket).unwrap());
                assert_eq!(complete.iter().map(|bar| bar.volume).sum::<i64>(), expected);
                if venue == Venue::NseCash && pull::vendor::cash_auction_eligibility_required(day) {
                    let (unverified, warnings) =
                        complete_minutes_for_venue(&bars, bucket, venue).unwrap();
                    assert!(unverified.is_empty());
                    assert!(warnings.iter().any(|why| why.contains("UNVERIFIED")));
                }
            }
            let mut with_close = bars.clone();
            with_close.push(minute(open + expected * 60));
            let (complete, diagnostics) = complete_minutes_with_cash_schedule(
                &with_close,
                Bucket::MINUTE,
                venue,
                Some(&cash_schedule),
            )
            .unwrap();
            assert_eq!(complete, bars);
            let last_minute = if expected == 385 {
                15 * 60 + 39
            } else {
                15 * 60 + 29
            };
            assert_eq!(
                complete.last().unwrap().ts_micros,
                (midnight + last_minute * 60) * 1_000_000
            );
            assert_eq!(diagnostics.len(), 1);
            assert!(diagnostics[0].contains("observed 1, scheduled 0"));
        }
    }
}

#[test]
fn derivative_extension_changes_the_missing_observed_day_tail() {
    use pull::fold::complete_minutes_for_venue;
    use pull::session::Day;
    use pull::vendor::Venue;
    let day = Day::new(2026, 8, 3).unwrap();
    let open =
        i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS + 555 * 60;
    let bars: Vec<_> = (0..375)
        .map(|m| minute(open + m * 60))
        .chain((0..385).map(|m| minute(open + 86_400 + m * 60)))
        .collect();
    let (complete, diagnostics) =
        complete_minutes_for_venue(&bars, Bucket::of_secs(300).unwrap(), Venue::NseDerivatives)
            .unwrap();
    assert_eq!(
        complete,
        fold(&bars, Bucket::of_secs(300).unwrap()).unwrap()
    );
    assert_eq!(diagnostics.len(), 1);
    assert!(diagnostics[0].contains("absent: observed 0, scheduled 10"));
    assert!(diagnostics[0].contains("observed-day tail"));
}

#[test]
fn venue_hours_do_not_override_an_exceptional_calendar_session() {
    for venue in pull::vendor::Venue::ALL {
        let day = pull::calendar::SYSTEMS_OUTAGE_DAY;
        let midnight = day * 86_400 - pull::session::IST_OFFSET_SECS;
        let bars: Vec<_> = (0..385)
            .map(|m| minute(midnight + (555 + m) * 60))
            .collect();
        let (complete, diagnostics) =
            pull::fold::complete_minutes_for_venue(&bars, Bucket::of_secs(3600).unwrap(), venue)
                .unwrap();
        assert!(complete.is_empty());
        assert!(
            diagnostics
                .iter()
                .all(|why| why.contains("exceptional session"))
        );
        assert!(!diagnostics.is_empty());
    }
}
