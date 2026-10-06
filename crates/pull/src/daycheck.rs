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
//! **Differing and contradicting are counted apart (D-3347).** The pulled day
//! differs from its minutes on almost every day, so a difference alone says
//! nothing; a pulled day that its own minutes exceed — in high, low or volume —
//! is the finding. See [`Report`].
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
///
/// # Differing is not contradicting (D-3347)
///
/// Measured on the operator's Zerodha store on 6 Oct 2026, over 1,964 days of
/// 2018-01..=2026-09 held at both rungs: the pulled day bar differs from the
/// fold of its own minutes on almost every day. NIFTY: 1,920 days differ, the
/// close on 1,919 of them and a range wider than the minutes' on 1,269.
/// CGPOWER, ABB and ADANIENT: all 1,964 days differ, the pulled volume above
/// the minutes' sum on 1,956 to 1,958 of them. Why the vendor's day bar takes
/// its close, open, range and volume as it does is not sourced in
/// `docs/00-charter.md`, and this module does not guess. So "differs" is
/// counted and is not a warning.
///
/// What a pulled day CAN be held to is the one thing no convention excuses:
/// it cannot hold less than its own minutes. A minute high above the pulled
/// high, a minute low below the pulled low, or minutes summing to more volume
/// than the pulled day, is a contradiction between the vendor's two series,
/// and only that is a warning. Measured on the same store: NIFTY 9 days, among
/// them a minute high 229.85 points above the pulled high (2022-03-07) and a
/// minute low 79 points below the pulled low (2020-03-13); BANKNIFTY 4;
/// NIFTYIT 2; INDIAVIX 131, 106 of them in 2018 and each of those one or two
/// paisa, the largest 176 paisa; each of the three shares 7 to 9, most of them
/// minute volume above the pulled volume.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Report {
    /// Days present on both sides whose open, high, low, close and volume all
    /// agree.
    pub agreed: usize,
    /// Days present on both sides that differ in at least one field, whether
    /// or not they contradict.
    pub differed: usize,
    /// Of [`Self::differed`], days the pulled bar CONTRADICTS: a minute high
    /// above the pulled high, a minute low below the pulled low, or a minute
    /// volume sum above the pulled volume.
    pub contradicted: usize,
    /// Of [`Self::differed`], days whose pulled close is not the last
    /// minute's close.
    pub close_differs: usize,
    /// Of [`Self::differed`], days whose pulled open is not the first
    /// minute's open.
    pub open_differs: usize,
    /// Of [`Self::differed`], days whose pulled high is above the minutes'
    /// high or whose pulled low is below the minutes' low.
    pub range_wider: usize,
    /// Of [`Self::differed`], days whose pulled volume is above the minutes'
    /// sum.
    pub volume_above: usize,
    /// The largest price breach over the contradicted days, in paisa: how far
    /// a minute high rose above the pulled high, or a minute low fell below
    /// the pulled low. Zero when nothing contradicted on price.
    pub largest_breach: i64,
    /// The first contradiction, in words, or `None`.
    pub first_contradiction: Option<String>,
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

    /// Whether every day the minute bars cover has a pulled day that holds
    /// them: it may differ, it does not contradict (D-3347).
    #[must_use]
    pub const fn consistent(&self) -> bool {
        self.contradicted == 0 && self.day_absent == 0
    }

    /// Count one day present on both sides. O(1): a fixed number of compares.
    /// **UNVERIFIED as a measurement**: no bench times it (`docs/06-limits.md`,
    /// D-3347).
    fn tally(&mut self, day: i64, served: &Bar, made: &Bar) {
        let why = differing(served, made);
        if why.is_empty() {
            self.agreed += 1;
            return;
        }
        if self.first.is_none() {
            self.first = Some(format!("IST day {day}: {why}"));
        }
        self.differed += 1;
        self.close_differs += usize::from(served.close != made.close);
        self.open_differs += usize::from(served.open != made.open);
        self.range_wider += usize::from(served.high > made.high || served.low < made.low);
        self.volume_above += usize::from(served.volume > made.volume);
        let against = contradicting(served, made);
        if !against.is_empty() {
            self.contradicted += 1;
            let breach = made
                .high
                .saturating_sub(served.high)
                .max(served.low.saturating_sub(made.low));
            self.largest_breach = self.largest_breach.max(breach);
            if self.first_contradiction.is_none() {
                self.first_contradiction = Some(format!("IST day {day}: {against}"));
            }
        }
    }
}

/// What the minutes hold that the pulled day cannot, comma separated, or an
/// empty string. A pulled day may close, open, range and count volume by its
/// own convention; it cannot be exceeded by its own minutes (D-3347).
fn contradicting(pulled: &Bar, folded: &Bar) -> String {
    let checks = [
        (
            folded.high > pulled.high,
            "minute high",
            folded.high,
            "above the pulled high",
            pulled.high,
        ),
        (
            folded.low < pulled.low,
            "minute low",
            folded.low,
            "below the pulled low",
            pulled.low,
        ),
        (
            folded.volume > pulled.volume,
            "minute volume",
            folded.volume,
            "above the pulled volume",
            pulled.volume,
        ),
    ];
    checks
        .iter()
        .filter(|(hit, ..)| *hit)
        .map(|(_, name, minutes, relation, served)| format!("{name} {minutes} {relation} {served}"))
        .collect::<Vec<_>>()
        .join(", ")
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
                        report.tally(a, served, made);
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
        assert!(Report::default().consistent());
    }

    /// **A PULLED DAY MAY DIFFER; IT MAY NOT BE EXCEEDED BY ITS MINUTES
    /// (D-3347).**
    ///
    /// A different close or open, a wider range and more volume are each
    /// counted by kind and leave the month consistent. A minute above the
    /// pulled high, below the pulled low, or more minute volume than the pulled
    /// day is a contradiction: counted, the largest price breach kept in paisa,
    /// and the first one named with both values. A minute day with no pulled
    /// day is not consistent either.
    #[test]
    fn a_difference_is_counted_by_kind_and_only_an_exceeded_day_contradicts() {
        // The minutes fold to o100 h120 l90 c98 v60 (see `served`).
        let mut day = served(0);
        day.close = 99;
        day.open = 101;
        day.high = 125;
        day.low = 85;
        day.volume = 70;
        let report = compare(&session(0), &[day]).unwrap();
        assert_eq!(
            report,
            Report {
                differed: 1,
                close_differs: 1,
                open_differs: 1,
                range_wider: 1,
                volume_above: 1,
                first: report.first.clone(),
                ..Report::default()
            }
        );
        assert!(report.consistent() && !report.clean());
        assert!(
            report
                .first
                .as_deref()
                .is_some_and(|f| f.contains("close pulled 99 folded 98"))
        );

        for (field, breach) in [("high", 3), ("low", 4), ("volume", 0)] {
            let mut day = served(0);
            match field {
                "high" => day.high = 117,
                "low" => day.low = 94,
                _ => day.volume = 59,
            }
            let report = compare(&session(0), &[day]).unwrap();
            assert_eq!(report.contradicted, 1, "{field}");
            assert_eq!(report.differed, 1, "{field}");
            assert_eq!(report.largest_breach, breach, "{field}");
            assert!(!report.consistent(), "{field}");
            let named = report
                .first_contradiction
                .expect("the first contradiction is named");
            let want = match field {
                "high" => "minute high 120 above the pulled high 117",
                "low" => "minute low 90 below the pulled low 94",
                _ => "minute volume 60 above the pulled volume 59",
            };
            assert!(named.ends_with(want), "{field}: {named}");
        }

        // The largest breach over a month, and the FIRST contradiction named.
        let mut first = served(0);
        first.high = 119;
        let mut second = served(1);
        second.low = 99;
        let minutes = [session(0), session(1)].concat();
        let report = compare(&minutes, &[first, second]).unwrap();
        assert_eq!((report.contradicted, report.largest_breach), (2, 9));
        assert!(
            report
                .first_contradiction
                .is_some_and(|f| f.contains("minute high 120 above the pulled high 119"))
        );

        // A minute day with no pulled day is not consistent.
        let report = compare(&minutes, &[served(0)]).unwrap();
        assert_eq!(report.contradicted, 0);
        assert!(!report.consistent());
    }
}
