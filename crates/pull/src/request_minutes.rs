//! Request-wide spot-minute evidence, separate from per-month derivation.
//! A monotone row cursor and one visit per civil day: O(rows + days + gaps).
//! No expected-minute enumeration, synthetic candles, or disk-history inference.

use super::Plan;
use crate::calendar::{DayKind, Session};
use crate::fetch::RawRow;
use crate::session::{Day, IST_OFFSET_SECS};
use crate::vendor::{Granularity, SessionKind, TimestampEncoding, Venue};
use brutex_core::instrument::{Exchange, Segment};

pub(super) fn audit(rows: &[RawRow], plan: Plan<'_>) -> Vec<String> {
    if plan.request.granularity != Granularity::Minute1
        || plan.contract.is_some()
        || plan.request.listing == crate::vendor::Listing::Derivative
    {
        return Vec::new();
    }
    let venue = match (Exchange::parse(plan.exchange), Segment::parse(plan.segment)) {
        (Ok(exchange), Ok(segment)) => Venue::for_segment(exchange, segment),
        _ => None,
    };
    let Some(venue @ (Venue::NseIndex | Venue::NseCash)) = venue else {
        return Vec::new();
    };
    if rows
        .windows(2)
        .any(|pair| matches!(pair, [a, b] if a.timestamp > b.timestamp))
    {
        return vec!["request minute coverage UNVERIFIED: unordered broker timestamps".to_owned()];
    }
    let mut stamps = rows
        .iter()
        .map(|row| local_seconds(row.timestamp, plan.encoding))
        .peekable();
    let mut failures = Vec::new();
    for number in
        plan.request.window.from().days_from_epoch()..=plan.request.window.to().days_from_epoch()
    {
        let Ok(day) = Day::from_days(number) else {
            failures.push(format!(
                "request minute coverage UNVERIFIED: invalid day {number}"
            ));
            continue;
        };
        match plan.calendar.kind_of(i64::from(number)) {
            DayKind::Closed => continue,
            DayKind::Open(session) if session == Session::full() => {}
            DayKind::Unmeasured => {
                failures.push(format!(
                    "request minute coverage UNVERIFIED on {day}: {}",
                    plan.calendar.unverified_reason(i64::from(number))
                ));
                continue;
            }
            _ => {
                failures.push(format!("request minute coverage UNVERIFIED on {day}: exceptional or unmeasured calendar session"));
                continue;
            }
        }
        let hours = match venue.hours_on(day) {
            Ok(hours) if hours.kind() == SessionKind::Continuous => hours,
            other => {
                failures.push(format!(
                    "request minute coverage UNVERIFIED on {day}: venue hours {other:?}"
                ));
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
                        failures.push(format!(
                            "request minute coverage UNVERIFIED on {day}: {why}"
                        ));
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
                missing(&mut failures, day, base, next, stamp);
            }
            next = stamp + 60;
            stamps.next();
        }
        if next < close {
            missing(&mut failures, day, base, next, close);
        }
    }
    failures
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

fn missing(failures: &mut Vec<String>, day: Day, base: i128, from: i128, to: i128) {
    let first = (from - base) / 60;
    let end = (to - base) / 60;
    failures.push(format!("request minute coverage gap on {day}: {:02}:{:02}–{:02}:{:02} IST (end exclusive), {} missing scheduled minutes; no bars fabricated", first / 60, first % 60, end / 60, end % 60, (to - from) / 60));
}
