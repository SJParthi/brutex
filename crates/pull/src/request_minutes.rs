//! Request-wide spot-minute evidence, separate from per-month derivation.
//! A monotone row cursor and one visit per civil day: O(rows + days + gaps).
//! The output is not capped: one `String`, one `Failure` and exactly one
//! telemetry event per gap, emitted by the caller and not here (D-2371), at
//! most about half a session's minutes per day. UNVERIFIED as
//! a measured bound: no bench times this. `docs/06-limits.md`, D-1493.
//! No expected-minute enumeration, synthetic candles, or disk-history inference.

use super::Plan;
use crate::calendar::{DayKind, Session};
use crate::fetch::RawRow;
use crate::session::{Day, IST_OFFSET_SECS};
use crate::vendor::{Granularity, SessionKind, TimestampEncoding, Venue};
use brutex_core::instrument::{Exchange, Segment};

pub(super) fn audit(rows: &[RawRow], plan: Plan<'_>) -> Vec<String> {
    let Some(venue) = audited_venue(plan) else {
        return Vec::new();
    };
    audit_venue(rows, plan, venue)
}

fn audited_venue(plan: Plan<'_>) -> Option<Venue> {
    if plan.request.granularity != Granularity::Minute1
        || plan.contract.is_some()
        || plan.request.listing == crate::vendor::Listing::Derivative
    {
        return None;
    }
    let venue = match (Exchange::parse(plan.exchange), Segment::parse(plan.segment)) {
        (Ok(exchange), Ok(segment)) => Venue::for_segment(exchange, segment),
        _ => None,
    };
    venue.filter(|venue| matches!(venue, Venue::NseIndex | Venue::NseCash))
}

fn audit_venue(rows: &[RawRow], plan: Plan<'_>, venue: Venue) -> Vec<String> {
    if rows
        .windows(2)
        .any(|pair| matches!(pair, [a, b] if a.timestamp > b.timestamp))
    {
        // Logged once by the caller, like every other line here (D-2371).
        return vec!["request minute coverage UNVERIFIED: unordered broker timestamps".to_owned()];
    }
    let mut stamps = rows
        .iter()
        // FLOORED TO ITS MINUTE (c4a-4, D-1493). Every comparison below is
        // in whole minutes: `next` steps by 60 from the open, and `missing`
        // counts `(to - from) / 60`. A stamp such as 09:15:59 compared raw
        // opened a "gap" of 0 minutes before it and moved `next` to 09:16:59,
        // so a real missing 09:16 before a 09:17:00 row was reported as 0
        // missing minutes. `ingest::from_window` refuses an off-grid stamp
        // before calling here, so production input is already aligned; this
        // makes the count right without relying on that order of calls.
        .map(|row| local_seconds(row.timestamp, plan.encoding).div_euclid(60) * 60)
        .peekable();
    let mut gaps = Vec::new();
    for number in
        plan.request.window.from().days_from_epoch()..=plan.request.window.to().days_from_epoch()
    {
        let Ok(day) = Day::from_days(number) else {
            push_gap(
                &mut gaps,
                format!("request minute coverage UNVERIFIED: invalid day {number}"),
            );
            continue;
        };
        match plan.calendar.kind_of(i64::from(number)) {
            DayKind::Closed => continue,
            DayKind::Open(session) if session == Session::full() => {}
            DayKind::Unmeasured => {
                push_gap(
                    &mut gaps,
                    format!(
                        "request minute coverage UNVERIFIED on {day}: {}",
                        plan.calendar.unverified_reason(i64::from(number))
                    ),
                );
                continue;
            }
            _ => {
                push_gap(
                    &mut gaps,
                    format!(
                        "request minute coverage UNVERIFIED on {day}: exceptional or unmeasured calendar session"
                    ),
                );
                continue;
            }
        }
        let hours = match venue.hours_on(day) {
            Ok(hours) if hours.kind() == SessionKind::Continuous => hours,
            other => {
                push_gap(
                    &mut gaps,
                    format!("request minute coverage UNVERIFIED on {day}: venue hours {other:?}"),
                );
                continue;
            }
        };
        let base = i128::from(number) * 86_400;
        let mut next = base + i128::from(hours.open_minute()) * 60;
        let close_minute =
            if venue == Venue::NseCash && crate::vendor::cash_auction_eligibility_required(day) {
                match plan
                    .cash_schedule
                    .ok_or_else(|| "dated cash eligibility missing".to_owned())
                    .and_then(|schedule| schedule.close(day))
                {
                    Ok(close) => u32::from(close),
                    Err(why) => {
                        push_gap(
                            &mut gaps,
                            format!("request minute coverage UNVERIFIED on {day}: {why}"),
                        );
                        continue;
                    }
                }
            } else {
                hours.close_minute()
            };
        let close = base + i128::from(close_minute) * 60;
        while stamps.peek().is_some_and(|stamp| *stamp < next) {
            stamps.next();
        }
        while let Some(&stamp) = stamps.peek() {
            if stamp >= close {
                break;
            }
            if stamp > next {
                missing(&mut gaps, day, base, next, stamp);
            }
            next = stamp + 60;
            stamps.next();
        }
        if next < close {
            missing(&mut gaps, day, base, next, close);
        }
    }
    gaps
}

fn local_seconds(timestamp: i64, encoding: TimestampEncoding) -> i128 {
    match encoding {
        TimestampEncoding::EpochMillisUtc => {
            i128::from(timestamp).div_euclid(1_000) + i128::from(IST_OFFSET_SECS)
        }
        TimestampEncoding::EpochSecondsUtc | TimestampEncoding::IsoDateTimeOffset => {
            i128::from(timestamp) + i128::from(IST_OFFSET_SECS)
        }
        TimestampEncoding::IstDateTimeText | TimestampEncoding::IsoDateTimeText => {
            i128::from(timestamp)
        }
    }
}

/// One coverage line, recorded and NOT logged here (OD-2, D-2371).
///
/// This module returns lines; it does not hold the instrument they belong to.
/// Its one caller, `ingest::from_window`, turns each line into a receipt
/// `Failure` and emits exactly one `pull.request_minutes` event at `Error`
/// naming the real instrument. Until D-2371 this helper ALSO emitted a
/// `pull.file` "not filed" event under the placeholder instrument "requested
/// window", so every gap was two log lines and `docs/06-limits.md` said one.
fn push_gap(gaps: &mut Vec<String>, why: String) {
    gaps.push(why);
}

fn missing(gaps: &mut Vec<String>, day: Day, base: i128, from: i128, to: i128) {
    let first = (from - base) / 60;
    let end = (to - base) / 60;
    push_gap(
        gaps,
        format!(
            "request minute coverage gap on {day}: {:02}:{:02}–{:02}:{:02} IST (end exclusive), {} missing scheduled minutes; no bars fabricated",
            first / 60,
            first % 60,
            end / 60,
            end % 60,
            (to - from) / 60
        ),
    );
}

#[cfg(test)]
#[allow(clippy::expect_used, reason = "test-only assertions")]
mod tests {
    use super::*;
    use crate::fetch::BarRequest;
    use crate::session::Window;

    fn row(timestamp: i64) -> RawRow {
        RawRow {
            timestamp,
            open: 100,
            high: 100,
            low: 100,
            close: 100,
            volume: 1,
            open_interest: None,
        }
    }

    /// **A STAMP INSIDE A MINUTE COUNTS AS THAT MINUTE (c4a-4, D-1493).**
    ///
    /// 09:15:59 then 09:17:00 then every minute to the close: one gap, the
    /// 09:16 minute, reported as one missing minute. Before the floor this
    /// produced two "0 missing scheduled minutes" gaps and no correct one.
    #[test]
    fn a_stamp_inside_a_minute_counts_as_that_minute_and_a_gap_counts_right() {
        let day = Day::new(2022, 10, 3).expect("a real day");
        let request = BarRequest {
            instrument_id: String::new(),
            listing: crate::vendor::Listing::Index,
            window: Window::new(day, day).expect("one day"),
            granularity: Granularity::Minute1,
        };
        let plan = Plan {
            calendar: crate::calendar::Runtime::default(),
            cash_schedule: None,
            columns: crate::csv::Columns::TrueDataIndex,
            request: &request,
            encoding: TimestampEncoding::EpochSecondsUtc,
            scale: crate::vendor::PriceScale::Paisa,
            vendor: brutex_core::vendor::Vendor::Groww,
            exchange: "NSE",
            segment: "INDEX",
            contract: None,
        };
        let open = i64::from(day.days_from_epoch()) * 86_400 - IST_OFFSET_SECS + 555 * 60;
        let mut rows = vec![row(open + 59), row(open + 120)];
        rows.extend((3..375).map(|minute| row(open + minute * 60)));
        let failures = audit(&rows, plan);
        assert_eq!(failures.len(), 1, "{failures:?}");
        let only = failures.first().expect("one gap");
        assert!(
            only.contains("09:16–09:17") && only.contains(" 1 missing scheduled minutes"),
            "{only}"
        );

        // A WHOLE SESSION, EACH STAMP 59 SECONDS LATE: no gap at all.
        let late: Vec<RawRow> = (0..375)
            .map(|minute| row(open + minute * 60 + 59))
            .collect();
        assert!(audit(&late, plan).is_empty(), "every minute is held");

        // AND THE ALIGNED SESSION IS UNCHANGED.
        let aligned: Vec<RawRow> = (0..375).map(|minute| row(open + minute * 60)).collect();
        assert!(audit(&aligned, plan).is_empty());

        // AN EMPTY RESPONSE IS STILL THE WHOLE SESSION MISSING.
        let none = audit(&[], plan);
        assert_eq!(none.len(), 1);
        assert!(
            none.first()
                .is_some_and(|line| line.contains("375 missing scheduled minutes")),
            "{none:?}"
        );
    }
}
