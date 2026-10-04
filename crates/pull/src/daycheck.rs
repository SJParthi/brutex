//! The pulled day bar, checked against the days its own minute bars fold to.
//!
//! # The operator's rule, 4 Oct 2026
//!
//! *"zerodha will always pull one day as the first point for the entire dates
//! and only then one min will be pulled … and then entirely rederive internal
//! calculation also should happen."* The day pass runs first over the whole
//! span (`api::autopilot::rung_for`), the minute pass second, and every
//! intraday rung is folded from the minute bars (`crate::ingest::derive_all`).
//!
//! That leaves two answers for one day: the day bar the vendor served, and the
//! day its own minute bars add up to. This module compares them.
//!
//! # A check, never a replacement
//!
//! D-0077 stands: the day is SERVED, never derived, because a vendor's day bar
//! follows the vendor's own convention and folding one from minutes would pick
//! a convention silently. So nothing here writes a bar. A disagreement is
//! reported, field by field, and the pulled day stays exactly as it was served.
//! A missing minute inside a session, a day the vendor corrected after the
//! minutes were served, or a volume convention that differs all show up here
//! rather than nowhere.
//!
//! # Cost
//!
//! One fold of the month's minute bars, `O(minutes)`, and one merge of two
//! ascending day lists, `O(days)`. Both are per instrument-month, run once
//! after the minute bars land, and neither probes or sorts anything.
//! **UNVERIFIED as a measurement**: argued from the shape of the code, and no
//! bench times it (`CLAUDE.md` §3 rule 6). D-3001.

use store::format::Bar;

use crate::fold::{Bucket, FoldError, fold_from_bars};

/// What the comparison found for one instrument-month.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Days present on both sides whose open, high, low, close and volume all
    /// agree.
    pub agreed: usize,
    /// Days present on both sides that differ in at least one field.
    pub differed: usize,
    /// Days the minute bars cover and the pulled day file does not.
    pub day_absent: usize,
    /// Days the pulled day file holds and the minute bars do not reach yet.
    ///
    /// Not a fault: the day pass runs over the whole span first, so the day
    /// file is AHEAD of the minute file for as long as the minute pass is still
    /// climbing.
    pub minute_absent: usize,
    /// The first disagreement, in words, or `None` when every compared day
    /// agreed and no day bar was absent.
    pub first: Option<String>,
}

impl Report {
    /// Whether every day the minute bars cover agreed with the pulled day.
    #[must_use]
    pub const fn clean(&self) -> bool {
        self.differed == 0 && self.day_absent == 0
    }
}

/// The IST calendar day a bar's stamp falls on, as days since the epoch.
const fn ist_day(bar: &Bar) -> i64 {
    (bar.ts_micros.div_euclid(1_000_000) + crate::session::IST_OFFSET_SECS).div_euclid(86_400)
}

/// The fields of `pulled` that differ from `folded`, comma separated, or an
/// empty string when all five agree.
fn differing(pulled: &Bar, folded: &Bar) -> String {
    let fields = [
        ("open", pulled.open, folded.open),
        ("high", pulled.high, folded.high),
        ("low", pulled.low, folded.low),
        ("close", pulled.close, folded.close),
        ("volume", pulled.volume, folded.volume),
    ];
    fields
        .iter()
        .filter(|(_, served, summed)| served != summed)
        .map(|(name, served, summed)| format!("{name} pulled {served} folded {summed}"))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Compares the pulled day bars with the days `minutes` fold to.
///
/// Both inputs are ascending, which is what a store file is. Days are matched
/// on their IST calendar day rather than on the raw stamp, so a vendor that
/// stamps its day bar at midnight and a fold that stamps it at the open still
/// meet on the same day.
///
/// # Errors
///
/// The fold's own refusal, [`FoldError`], when `minutes` is out of order.
pub fn compare(minutes: &[Bar], days: &[Bar]) -> Result<Report, FoldError> {
    let folded = fold_from_bars(minutes, Bucket::DAY, Bucket::MINUTE)?;
    let mut report = Report::default();
    let mut pulled = days.iter().peekable();
    let mut summed = folded.iter().peekable();
    loop {
        match (pulled.peek(), summed.peek()) {
            (None, None) => return Ok(report),
            (Some(_), None) => {
                report.minute_absent += pulled.by_ref().count();
            }
            (None, Some(_)) => {
                let rest: Vec<&Bar> = summed.by_ref().collect();
                if report.first.is_none()
                    && let Some(bar) = rest.first()
                {
                    report.first = Some(format!(
                        "IST day {} has minute bars and no pulled day bar",
                        ist_day(bar)
                    ));
                }
                report.day_absent += rest.len();
            }
            (Some(&served), Some(&made)) => {
                let (a, b) = (ist_day(served), ist_day(made));
                match a.cmp(&b) {
                    std::cmp::Ordering::Less => {
                        report.minute_absent += 1;
                        pulled.next();
                    }
                    std::cmp::Ordering::Greater => {
                        if report.first.is_none() {
                            report.first =
                                Some(format!("IST day {b} has minute bars and no pulled day bar"));
                        }
                        report.day_absent += 1;
                        summed.next();
                    }
                    std::cmp::Ordering::Equal => {
                        let why = differing(served, made);
                        if why.is_empty() {
                            report.agreed += 1;
                        } else {
                            if report.first.is_none() {
                                report.first = Some(format!("IST day {a}: {why}"));
                            }
                            report.differed += 1;
                        }
                        pulled.next();
                        summed.next();
                    }
                }
            }
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::indexing_slicing,
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic
)]
mod tests {
    use super::{Report, compare, differing, ist_day};
    use store::format::Bar;

    /// 09:15 IST on 2026-10-03, in seconds since the epoch.
    const OPEN: i64 = 1_791_000_000 - (1_791_000_000 + 19_800) % 86_400 + 33_300;

    fn bar(secs: i64, open: i64, high: i64, low: i64, close: i64, volume: i64) -> Bar {
        Bar {
            ts_micros: secs * 1_000_000,
            open,
            high,
            low,
            close,
            volume,
            open_interest: i64::MIN,
        }
    }

    /// Three minutes of one session starting `day` days after `OPEN`'s.
    fn session(day: i64) -> Vec<Bar> {
        let at = OPEN + day * 86_400;
        vec![
            bar(at, 100, 110, 95, 105, 10),
            bar(at + 60, 105, 120, 100, 115, 20),
            bar(at + 120, 115, 116, 90, 98, 30),
        ]
    }

    /// The day bar those three minutes add up to, stamped at IST midnight the
    /// way a vendor stamps it.
    fn served(day: i64) -> Bar {
        bar(OPEN - 33_300 + day * 86_400, 100, 120, 90, 98, 60)
    }

    #[test]
    fn the_anchor_is_a_session_open() {
        assert_eq!((OPEN + 19_800).rem_euclid(86_400), 33_300, "09:15 IST");
        assert_eq!(ist_day(&served(0)), ist_day(&session(0)[0]));
        assert_eq!(ist_day(&served(1)), ist_day(&session(0)[0]) + 1);
    }

    #[test]
    fn a_day_its_minutes_add_up_to_agrees() {
        let minutes = [session(0), session(1)].concat();
        let report = compare(&minutes, &[served(0), served(1)]).unwrap();
        assert_eq!(
            report,
            Report {
                agreed: 2,
                ..Report::default()
            }
        );
        assert!(report.clean());
    }

    #[test]
    fn each_differing_field_is_named_with_both_values() {
        let mut day = served(0);
        day.high = 121;
        day.volume = 61;
        let report = compare(&session(0), &[day]).unwrap();
        assert_eq!(report.differed, 1);
        assert_eq!(report.agreed, 0);
        assert!(!report.clean());
        let first = report.first.unwrap();
        assert!(
            first.ends_with("high pulled 121 folded 120, volume pulled 61 folded 60"),
            "{first}"
        );
        for (field, change) in [("open", 0), ("low", 2), ("close", 3)] {
            let mut day = served(0);
            match change {
                0 => day.open += 1,
                2 => day.low += 1,
                _ => day.close += 1,
            }
            let folded = served(0);
            assert_eq!(
                differing(&day, &folded),
                format!(
                    "{field} pulled {} folded {}",
                    match change {
                        0 => day.open,
                        2 => day.low,
                        _ => day.close,
                    },
                    match change {
                        0 => folded.open,
                        2 => folded.low,
                        _ => folded.close,
                    }
                )
            );
        }
        assert_eq!(differing(&served(0), &served(0)), "");
    }

    #[test]
    fn a_day_pass_ahead_of_the_minute_pass_is_not_a_fault() {
        let report = compare(&session(0), &[served(0), served(1), served(2)]).unwrap();
        assert_eq!(report.agreed, 1);
        assert_eq!(report.minute_absent, 2);
        assert!(report.clean(), "the day pass runs first, so it is ahead");
        assert_eq!(report.first, None);
        // Ahead at the START too: a pulled day the minutes have not reached.
        let report = compare(&session(1), &[served(0), served(1)]).unwrap();
        assert_eq!((report.agreed, report.minute_absent), (1, 1));
    }

    #[test]
    fn minutes_with_no_pulled_day_are_reported() {
        let minutes = [session(0), session(1), session(2)].concat();
        // Missing in the middle…
        let report = compare(&minutes, &[served(0), served(2)]).unwrap();
        assert_eq!((report.agreed, report.day_absent), (2, 1));
        assert!(!report.clean());
        let middle = report.first.unwrap();
        assert!(middle.contains("no pulled day bar"), "{middle}");
        // …and at the end, where the first gap is named too.
        let report = compare(&minutes, &[served(0)]).unwrap();
        assert_eq!((report.agreed, report.day_absent), (1, 2));
        assert_eq!(
            report.first.unwrap(),
            format!(
                "IST day {} has minute bars and no pulled day bar",
                ist_day(&served(1))
            )
        );
        // An earlier disagreement is the one named, not a later gap.
        let mut day = served(0);
        day.open = 1;
        let report = compare(&minutes, &[day]).unwrap();
        assert!(report.first.unwrap().contains("open pulled 1"));
        let report = compare(&minutes, &[day, served(2)]).unwrap();
        assert!(report.first.unwrap().contains("open pulled 1"));
        // Nothing pulled at all.
        let report = compare(&minutes, &[]).unwrap();
        assert_eq!(report.day_absent, 3);
    }

    #[test]
    fn out_of_order_minutes_are_refused_by_the_fold() {
        let mut minutes = [session(1), session(0)].concat();
        minutes.truncate(4);
        assert!(compare(&minutes, &[served(0)]).is_err());
    }

    #[test]
    fn nothing_on_either_side_is_clean() {
        assert_eq!(compare(&[], &[]).unwrap(), Report::default());
    }
}
