//! EVERY INTRADAY RUNG OPENS THE SESSION AT 09:15, and the daily rung does not.
//!
//! The NSE open is 555 minutes past IST midnight, so a grid anchored at
//! midnight lands on 09:15 only where the rung divides 555 — three, five and
//! fifteen do; two, ten, thirty and sixty do not. Under that anchor each of
//! those four opened the day with a bar stamped BEFORE the open holding part of
//! the session, which is why `store_timeframe` first refused thirty and sixty
//! and why the operator's ladder — 2, 3, 5, 10, 15, 30, 60 — could not exist.
//! `store_timeframe` still refuses a VENDOR's bar at thirty and sixty, for a
//! different reason that `store_timeframe_follows_the_fold_anchor` below pins:
//! the vendor's grid at those rungs is UNVERIFIED. D-1447.
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
        3,
        "the first observed bucket, the first observed day's tail, and one unknown-day explanation, regardless of gap length; the unknown day's bucket is not restated (o1api-44, D-1201)"
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
    assert!(
        diagnostics
            .iter()
            .all(|why| !why.contains("calendar Unmeasured")),
        "{diagnostics:?}"
    );
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

/// audit-20261003 hunt-pull-3 (D-1533). An exceptional session is named ONCE
/// per day, not once per bucket. o1api-44 / D-1201 collapsed the per-bucket
/// lines for unmeasured days; an outage day, a DR Saturday or a Muhurat still
/// pushed one identical "exceptional session ... withheld" line per bucket,
/// each a `pull.derive` warning and a clause of the rung's refusal: 54 lines
/// for 2024-03-02 at two minutes.
#[test]
fn an_exceptional_session_is_named_once_per_day_not_once_per_bucket() {
    let day = 19_784; // 2024-03-02, a disaster-recovery Saturday in two windows
    let pull::calendar::DayKind::Open(session) = pull::calendar::kind_of(day) else {
        panic!("known session")
    };
    let midnight = day * 86_400 - pull::session::IST_OFFSET_SECS;
    let bars: Vec<_> = (0..1440_u16)
        .filter(|m| session.expects(*m))
        .map(|m| minute(midnight + i64::from(m) * 60))
        .collect();
    let (complete, diagnostics) =
        pull::fold::complete_minutes(&bars, Bucket::of_secs(120).unwrap()).unwrap();
    assert!(complete.is_empty(), "the session is still withheld");
    let named: Vec<_> = diagnostics
        .iter()
        .filter(|why| why.contains("exceptional session"))
        .collect();
    assert_eq!(named.len(), 1, "one line for the day: {diagnostics:?}");
    assert!(
        named.iter().all(|why| why.contains(&day.to_string())),
        "the line names the day: {named:?}"
    );
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

/// IST midnight of `day`, as a UTC epoch second.
fn midnight_of(day: pull::session::Day) -> i64 {
    i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS
}

/// **A DAY WHOSE SESSION LENGTH IS UNMEASURED IS WITHHELD ONCE, FOR ITS OWN
/// REASON.** GAP12-12, D-0955.
///
/// A pre-2025 Muhurat is `OpenLengthUnmeasured`: the exchange traded and the
/// calendar does not state for how long, so no minute count can complete a
/// bucket. Each bucket used to be reported as "incomplete or invalid minute
/// coverage ... historical gap refill requires a versioned store repair", which
/// `ingest::derive` rewrites into "restore complete minute source" — a repair
/// that does not exist, once per bucket. Checked at every width from one minute
/// to an hour, with a regular day after it whose buckets must still complete.
#[test]
fn an_unmeasured_length_day_is_withheld_once_with_its_own_reason() {
    use pull::calendar::DayKind;
    use pull::session::Day;
    let muhurat = Day::new(2020, 11, 14).unwrap();
    assert_eq!(
        pull::calendar::kind_of(i64::from(muhurat.days_from_epoch())),
        DayKind::OpenLengthUnmeasured,
        "the premise: the calendar withholds this day's length"
    );
    // Sixty minutes from 18:15 IST, which is the shape the 2025 Muhurat had.
    let evening = midnight_of(muhurat) + (18 * 60 + 15) * 60;
    let mut bars: Vec<_> = (0..60).map(|m| minute(evening + m * 60)).collect();
    // A regular session after it: 2020-11-17, a Tuesday.
    let regular = midnight_of(Day::new(2020, 11, 17).unwrap()) + 555 * 60;
    bars.extend((0..SESSION_MINUTES).map(|m| minute(regular + m * 60)));
    for secs in [60_u32, 300, 900, 3600] {
        let (complete, diagnostics) =
            pull::fold::complete_minutes(&bars, Bucket::of_secs(secs).unwrap()).unwrap();
        assert_eq!(
            diagnostics.len(),
            1,
            "{secs}s: one sentence for the day, not one per bucket: {diagnostics:?}"
        );
        let why = &diagnostics[0];
        assert!(why.contains("unmeasured"), "{secs}s: {why}");
        assert!(
            !why.contains("incomplete") && !why.contains("historical gap refill"),
            "{secs}s: not a coverage fault, so nothing for derive to rewrite into a repair: {why}"
        );
        assert!(
            complete
                .iter()
                .all(|bar| bar.ts_micros >= regular * 1_000_000),
            "{secs}s: nothing from the unmeasured day is certified"
        );
        assert_eq!(
            complete.iter().map(|bar| bar.volume).sum::<i64>(),
            SESSION_MINUTES,
            "{secs}s: and the regular day after it still completes whole"
        );
    }
    // A single bar on that day is still one sentence; no bars is none.
    let (_, one) = pull::fold::complete_minutes(&bars[..1], Bucket::of_secs(300).unwrap()).unwrap();
    assert_eq!(one.len(), 1, "{one:?}");
    let (none, nothing) = pull::fold::complete_minutes(&[], Bucket::of_secs(300).unwrap()).unwrap();
    assert!(none.is_empty() && nothing.is_empty());
}

/// **BARS ON A MEASURED CLOSED DAY ARE A STORE DEFECT, NAMED ONCE.** GAP12-12.
///
/// A Sunday with a whole session's minutes on it: the calendar says the
/// exchange did not trade, so the bars are a defect in what was stored, not a
/// hole a refill could close.
#[test]
fn bars_on_a_closed_day_are_named_once_as_a_store_defect() {
    use pull::calendar::DayKind;
    use pull::session::Day;
    let sunday = Day::new(2025, 7, 6).unwrap();
    assert_eq!(
        pull::calendar::kind_of(i64::from(sunday.days_from_epoch())),
        DayKind::Closed,
        "the premise: a measured weekend"
    );
    let open = midnight_of(sunday) + 555 * 60;
    let bars: Vec<_> = (0..SESSION_MINUTES)
        .map(|m| minute(open + m * 60))
        .collect();
    for secs in [60_u32, 300, 3600] {
        let (complete, diagnostics) =
            pull::fold::complete_minutes(&bars, Bucket::of_secs(secs).unwrap()).unwrap();
        assert!(complete.is_empty(), "{secs}s: nothing certified");
        assert_eq!(diagnostics.len(), 1, "{secs}s: {diagnostics:?}");
        assert!(
            diagnostics[0].contains("closed") && diagnostics[0].contains("store defect"),
            "{secs}s: {}",
            diagnostics[0]
        );
        assert!(!diagnostics[0].contains("historical gap refill"));
    }
}

/// audit-20261003 attackdata-5 (D-1532). A snapshot with a NEGATIVE volume is
/// refused by the fold, not netted into a plausible positive sum: 10 + -7 = 3
/// passed every later check, because the store's own count gate sees only the
/// sum. And a bucket wider than one IST day is refused by `Bucket::of_secs`: a
/// `u32::MAX` bucket stamped a 2024 snapshot at 1969-12-31.
#[test]
fn a_negative_snapshot_volume_and_a_bucket_wider_than_a_day_are_refused() {
    let five = Bucket::of_secs(300).unwrap();
    let mut a = minute(OPEN_UTC);
    a.volume = 10;
    let mut b = minute(OPEN_UTC + 60);
    b.volume = -7;
    let pair = vec![a, b];
    assert_eq!(
        fold(&pair, five),
        Err(pull::fold::FoldError::NegativeVolume { at: 1, volume: -7 }),
        "a negative volume is not netted into the bucket's sum"
    );
    assert_eq!(
        pull::fold::fold_from_bars(&pair, five, Bucket::MINUTE),
        Err(pull::fold::FoldError::NegativeVolume { at: 1, volume: -7 }),
        "the minute path refuses too"
    );
    let rendered = pull::fold::FoldError::NegativeVolume { at: 1, volume: -7 }.to_string();
    assert!(
        rendered.contains("-7") && rendered.contains("negative"),
        "{rendered}"
    );

    assert_eq!(Bucket::of_secs(86_400).map(Bucket::secs), Some(86_400));
    assert_eq!(Bucket::of_secs(86_401), None, "wider than a day");
    assert_eq!(Bucket::of_secs(u32::MAX), None, "wider than a day");
    assert_eq!(
        Bucket::of_secs(50).map(Bucket::secs),
        Some(50),
        "widths that divide a day"
    );
}

/// **A BUCKET'S VOLUME THAT LEAVES `i64` IS REFUSED, NOT CAPPED.**
/// ET-bars-candles-store-4, D-0955.
///
/// The fold refused a timestamp it could not shift, on the stated ground that
/// saturating files a wrong bar, and then saturated the volume sum. Both ends of
/// the range, the exact boundary, and two full buckets that must not interact.
#[test]
fn a_bucket_whose_volume_leaves_i64_is_refused_not_capped() {
    let five = Bucket::of_secs(300).unwrap();
    let pair = |first: i64, second: i64| {
        let mut a = minute(OPEN_UTC);
        a.volume = first;
        let mut b = minute(OPEN_UTC + 60);
        b.volume = second;
        vec![a, b]
    };
    // The negative ends are refused as NegativeVolume before any sum (D-1532),
    // so only the positive end can overflow.
    for (first, second) in [(i64::MAX, 1), (1, i64::MAX)] {
        assert_eq!(
            fold(&pair(first, second), five),
            Err(pull::fold::FoldError::VolumeOverflow {
                at: 1,
                bucket: OPEN_UTC * 1_000_000
            }),
            "{first} + {second}"
        );
        assert!(
            pull::fold::fold_from_bars(&pair(first, second), five, Bucket::MINUTE).is_err(),
            "the minute path refuses too"
        );
        assert!(
            matches!(
                pull::fold::complete_minutes(&pair(first, second), five),
                Err(pull::fold::FoldError::VolumeOverflow { .. })
            ),
            "and so does derivation's completeness fold"
        );
    }
    let rendered = pull::fold::FoldError::VolumeOverflow { at: 1, bucket: 7 }.to_string();
    assert!(
        rendered.contains("Refused rather than saturated") && rendered.contains("an i64"),
        "{rendered}"
    );
    // Exactly at either end is a legal sum, and stays exact.
    assert_eq!(
        fold(&pair(i64::MAX - 1, 1), five).unwrap()[0].volume,
        i64::MAX
    );
    // The negative end is no longer a sum at all: a negative minute volume is
    // refused before it is added (D-1532).
    assert_eq!(
        fold(&pair(i64::MIN + 1, -1), five),
        Err(pull::fold::FoldError::NegativeVolume {
            at: 0,
            volume: i64::MIN + 1
        })
    );
    // Two buckets each holding i64::MAX never meet.
    let mut apart = pair(i64::MAX, i64::MAX);
    apart[1].ts_micros = (OPEN_UTC + 300) * 1_000_000;
    let out = fold(&apart, five).unwrap();
    assert_eq!(
        out.iter().map(|bar| bar.volume).collect::<Vec<_>>(),
        [i64::MAX; 2]
    );
}

/// **`store_timeframe` FOLLOWS THE FOLD'S REAL ANCHOR, not a comment about it.**
///
/// The `const` block beside `store_timeframe` asserted the 555-minute
/// arithmetic and kept passing when `crate::fold` moved its intraday anchor
/// from IST midnight to the open, so the refusal of a vendor's 30min and 60min
/// bars went on citing a fold stub that no longer existed. This runs the fold.
///
/// For every intraday rung the store ships a directory for:
///
/// 1. the fold's first bar of a session is stamped at 09:15 — the anchor, read
///    from the fold's output rather than from its constants; and
/// 2. `store_timeframe` files a VENDOR's bar at the rung exactly when the grid
///    counted from IST midnight has an edge at that same first stamp — that
///    is, when a vendor's bar lands on a stored edge whichever of the two
///    grids it was stamped on. Where they differ, the vendor's grid is
///    UNVERIFIED and the bar is refused. D-1447.
///
/// A rung the store ships no directory for must answer `None`, and the daily
/// rung, which is not on an intraday grid, must answer `Some`.
#[test]
fn store_timeframe_follows_the_fold_anchor() {
    use pull::vendor::{Granularity, Grid};
    use store::path::Timeframe;

    let midnight_utc = OPEN_UTC - i64::from(Timeframe::OPEN_MINUTES_PAST_IST_MIDNIGHT) * 60;
    let mut refused = Vec::new();
    let mut intraday_seen = 0_usize;
    for rung in Granularity::ALL {
        let directory = Timeframe::KNOWN
            .iter()
            .any(|known| known.as_str() == rung.dir());
        match rung.grid() {
            Grid::Intraday(secs) if directory => {
                intraday_seen += 1;
                let width = i64::from(secs);
                let bucket = Bucket::of_secs(secs).expect("a real width");
                let out = fold(&session(), bucket).expect("a session in order");
                let first = out.first().expect("a session yields bars").ts_micros / 1_000_000;
                assert_eq!(
                    first, OPEN_UTC,
                    "{rung}: the fold's first bar must begin at the 09:15 open"
                );
                let midnight_edge = midnight_utc + (first - midnight_utc).div_euclid(width) * width;
                let grids_agree = midnight_edge == first;
                assert_eq!(
                    rung.store_timeframe().is_some(),
                    grids_agree,
                    "{rung}: a vendor's bar is filable exactly when the midnight-\
                     counted grid and the fold's open-anchored grid share the \
                     first edge (midnight edge {}s before the open)",
                    first - midnight_edge
                );
                if !grids_agree {
                    refused.push(rung);
                }
            }
            Grid::Intraday(_) | Grid::Event | Grid::Weekly => {
                assert_eq!(
                    rung.store_timeframe(),
                    None,
                    "{rung}: no store directory, so nowhere to file it"
                );
            }
            Grid::Daily => assert!(rung.store_timeframe().is_some(), "{rung}"),
        }
    }
    // Not vacuous: the seven intraday rungs with a directory were all asked,
    // and the refusal is exactly the two whose grids disagree.
    assert_eq!(intraday_seen, 7, "1s, 1, 3, 5, 15, 30 and 60 minutes");
    assert_eq!(refused, [Granularity::Minute30, Granularity::Hour1]);
}

/// **WHICH RUNG ENDS THE DAY SHORT IS A PROPERTY OF THE VENUE, NOT THE RUNG.**
///
/// The docs said only 2, 10, 30 and 60 minutes are ragged. That is true of a
/// 375-minute session and nothing else: the equity-derivatives session from
/// 2026-08-03 runs 385 minutes, where 3 and 15 minutes end with a 1- and a
/// 10-minute bar too, and the continuous session of a cash security eligible
/// for the closing auction runs 360, where nothing is ragged. The length here
/// is read from the venue row (`Venue::hours_on`, and the dated cash close),
/// never written down, so any doc table has to be derived the same way.
/// D-1447.
#[test]
fn derived_rung_stub_minutes_follow_the_venue_session() {
    use pull::fold::complete_minutes_with_cash_schedule;
    use pull::session::Day;
    use pull::vendor::Venue;

    let pre = Day::new(2026, 7, 31).unwrap();
    let post = Day::new(2026, 8, 3).unwrap();
    // (venue, day, cash eligible for the closing auction, session minutes the
    // venue row must produce). The last column is the claim under test; the
    // computation below takes the length from the row and checks it against it.
    let cases = [
        (Venue::NseIndex, pre, false, 375_i64),
        (Venue::NseIndex, post, false, 375),
        (Venue::NseDerivatives, pre, false, 375),
        (Venue::NseDerivatives, post, false, 385),
        (Venue::NseCash, post, false, 375),
        (Venue::NseCash, post, true, 360),
    ];
    for (venue, day, eligible, claimed) in cases {
        let hours = venue.hours_on(day).unwrap();
        let mut schedule = pull::cash_auction::Schedule::default();
        schedule.insert(day, eligible).unwrap();
        let close = if venue == Venue::NseCash {
            i64::from(schedule.close(day).unwrap())
        } else {
            i64::from(hours.close_minute())
        };
        let length = close - i64::from(hours.open_minute());
        assert_eq!(
            length, claimed,
            "{venue} on {day:?}: the venue row's length"
        );
        let midnight = i64::from(day.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS;
        let open = midnight + i64::from(hours.open_minute()) * 60;
        let bars: Vec<_> = (0..length).map(|m| minute(open + m * 60)).collect();
        for width in [1_i64, 2, 3, 5, 10, 15, 30, 60] {
            let bucket = Bucket::of_secs(u32::try_from(width * 60).unwrap()).unwrap();
            let (complete, diagnostics) =
                complete_minutes_with_cash_schedule(&bars, bucket, venue, Some(&schedule)).unwrap();
            assert!(
                diagnostics.is_empty(),
                "{venue} {width}min: {diagnostics:?}"
            );
            let stub = length % width;
            let last_minutes = if stub == 0 { width } else { stub };
            let last = complete.last().unwrap();
            assert_eq!(
                last.volume, last_minutes,
                "{venue} on {day:?} at {width}min: the last bar holds {last_minutes} minute(s)"
            );
            assert_eq!(
                last.ts_micros,
                (open + (length - last_minutes) * 60) * 1_000_000,
                "{venue} on {day:?} at {width}min: the short bar is the LAST, stamped on the grid"
            );
            assert_eq!(
                i64::try_from(complete.len()).unwrap(),
                length / width + i64::from(stub != 0),
                "{venue} at {width}min"
            );
        }
    }

    // THE TWO CASES THE OLD TABLE GOT WRONG, named rather than only computed.
    let hours = Venue::NseDerivatives.hours_on(post).unwrap();
    let midnight = i64::from(post.days_from_epoch()) * 86_400 - pull::session::IST_OFFSET_SECS;
    let open = midnight + i64::from(hours.open_minute()) * 60;
    let bars: Vec<_> = (0..385).map(|m| minute(open + m * 60)).collect();
    for (width, stub, opens_at) in [(3_u32, 1_i64, 15 * 60 + 39), (15, 10, 15 * 60 + 30)] {
        let (complete, _) = pull::fold::complete_minutes_for_venue(
            &bars,
            Bucket::of_secs(width * 60).unwrap(),
            Venue::NseDerivatives,
        )
        .unwrap();
        let last = complete.last().unwrap();
        assert_eq!(last.volume, stub, "{width}min on the 385-minute session");
        assert_eq!(last.ts_micros, (midnight + opens_at * 60) * 1_000_000);
    }
}

/// **A DAY THE VENUE CANNOT ATTEST IS NAMED ONCE, NOT TWICE PER BUCKET.** OD-1,
/// D-2370, invariant AFG-70.
///
/// NSE cash on or after 2026-08-03 needs a dated closing-auction eligibility
/// schedule. With none, `minute_session` refuses the day. Each bucket used to
/// push the refusal AND an "incomplete or invalid minute coverage" line, and
/// re-derive the venue's hours, so `cli::fold_audit`'s `withheld` counted
/// `2 x buckets` for one refused day. Two refused days, one with no schedule
/// and one missing from a schedule, are each one line at every width.
#[test]
fn a_day_the_venue_refuses_is_named_once_not_once_per_bucket() {
    use pull::fold::{complete_minutes_for_venue, complete_minutes_with_cash_schedule};
    use pull::session::Day;
    use pull::vendor::Venue;
    let monday = Day::new(2026, 8, 3).unwrap();
    let tuesday = Day::new(2026, 8, 4).unwrap();
    assert!(pull::vendor::cash_auction_eligibility_required(monday));
    let mut bars: Vec<_> = (0..SESSION_MINUTES)
        .map(|m| minute(midnight_of(monday) + 555 * 60 + m * 60))
        .collect();
    bars.extend((0..SESSION_MINUTES).map(|m| minute(midnight_of(tuesday) + 555 * 60 + m * 60)));
    // A schedule that names a different day: both observed days are missing.
    let mut elsewhere = pull::cash_auction::Schedule::default();
    elsewhere
        .insert(Day::new(2026, 8, 5).unwrap(), false)
        .unwrap();
    for secs in [60_u32, 120, 300, 900, 3600] {
        let bucket = Bucket::of_secs(secs).unwrap();
        let none = complete_minutes_for_venue(&bars, bucket, Venue::NseCash).unwrap();
        let missing =
            complete_minutes_with_cash_schedule(&bars, bucket, Venue::NseCash, Some(&elsewhere))
                .unwrap();
        for (label, (complete, diagnostics)) in [("no schedule", none), ("elsewhere", missing)] {
            assert!(complete.is_empty(), "{label} {secs}s: nothing certified");
            assert_eq!(
                diagnostics.len(),
                2,
                "{label} {secs}s: one line per refused day: {diagnostics:?}"
            );
            for (why, day) in diagnostics.iter().zip([monday, tuesday]) {
                assert!(
                    why.starts_with(&format!("day {}: ", day.days_from_epoch())),
                    "{label} {secs}s: {why}"
                );
                assert!(why.ends_with("derived buckets withheld"), "{why}");
                assert!(!why.contains("incomplete"), "{why}");
            }
        }
    }
}
