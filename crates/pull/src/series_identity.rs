//! Whose bars a minute answer holds: every minute judged against the day bar
//! the SAME series was served. WP-13d; findings DB-B07, DB-B08, DB-B10, DB-B60
//! and DB-B64.
//!
//! # What was found
//!
//! A broker's history is asked for by an instrument token, and the token is
//! today's. Read on the operator's store, 11 Oct 2026, read-only, every
//! Zerodha 1min series against its own 1day:
//!
//! - `IRFC` holds 9,315 minutes on days before its first day bar (2021-01-29)
//!   and `KFINTECH` 254: another instrument's bars, years before the listing;
//! - `PIIND` holds 7,576 minutes that lie wholly outside their own day's
//!   range, every one of them by more than 8%, on 156 days;
//! - `VBL` holds 8 minutes on 2023-06-14 at twice the day's level;
//! - index keys hold another index's minutes: `NIFTYNEXT50` 87,375 minutes on
//!   233 days and `NIFTYPVTBANK` 61,088 on 163, every one by more than 8%.
//!
//! Against that, `NIFTY` holds 6 minutes outside their day by any margin, the
//! largest 29.7 bps, and `BANKNIFTY` none, of 796,027 and 795,979.
//!
//! **The day rung of those index keys is itself mixed** (DB-B60 items 4 and
//! 5): a day bar that spans two indices' levels holds both, so a minute at
//! either passes here. `NIFTY100LIQ15` shows 8 foreign days by this rule
//! against 232 by the audit's fold comparison. That is a defect of the day
//! bar and is NOT caught by this module.
//!
//! **Why the vendor serves them is UNVERIFIED** (`CLAUDE.md` §3 rule 1): the
//! shapes fit a token that named another instrument at the time and a minute
//! store keyed by token, and no vendor document read for this repository says
//! so. What is measured is that the day rung of the same key does not hold
//! them, which is all this module relies on.
//!
//! # The rule
//!
//! The day pass runs first over the whole span and the minute pass second
//! (the operator's rule of 4 Oct 2026, [`crate::daycheck`]). So when a minute
//! arrives, the day it belongs to has already been served, and two things can
//! be said without any table of names, tokens or listing dates:
//!
//! 1. **A minute on a day before the series' first served day bar is refused**
//!    ([`Cause::BeforeFirstDayBar`]). The bars cannot tell a day before the
//!    listing from a day pass that has not reached that far; both are a minute
//!    with no day to belong to, and the refusal says so.
//! 2. **A minute whose whole range lies outside its day's range by more than
//!    [`FOREIGN_BAND_BPS`] is refused** ([`Cause::ForeignLevel`]). Another
//!    instrument's print, another price basis and another index all look the
//!    same from here, and none of them is this series on this day.
//! 3. **A minute outside by more than [`SUSPECT_BAND_BPS`] and no more than
//!    that lands, and is counted and named** ([`Verdict::Suspect`]). Between
//!    the two bands the store holds both kinds — an impostor index trading
//!    within 8% of the key's own, and a series' own minute beside a day bar
//!    the vendor adjusted apart — and the bars cannot tell which.
//!
//! A minute whose day the day rung does not hold, though it holds an earlier
//! one, is NOT refused: the day pass may simply be older than the minute pass
//! (today's minutes, yesterday's day pull). It is counted as unchecked and
//! named ([`Unchecked`]), never passed off as checked.
//!
//! # A screen, not a repair
//!
//! Nothing here writes or changes a bar. The landing door refuses the row by
//! name and the rest of the answer lands (D-3351); the store scan reports what
//! the stored months hold. A month already on disk keeps its bars until a
//! versioned generation without them is published, and `store::repair` adds
//! bars and never drops one (D-3350), so that door does not exist yet.
//!
//! # Cost
//!
//! [`DayBook::judge`] is one binary search over the day bars of the months an
//! answer touches (at most 31 a month) and a fixed number of integer
//! compares. [`audit`] is one such judgement per minute.

use store::format::Bar;

/// How far a minute's whole range may lie outside its served day's range, in
/// basis points of the nearer bound, before it is refused as another series'.
///
/// # Why 800
///
/// It is the audit's own threshold (DB-B60, DB-B64: a minute more than 8% off
/// its day). Measured on the operator's store, 11 Oct 2026, read-only, all
/// 330 Zerodha series holding both rungs, each 1min bar against its own 1day
/// (`state/fix/round2/fix-WP-13d.json` names the method): every minute the
/// audit named lies beyond it — `PIIND` 7,576 of 7,576, `VBL` 8 of 8,
/// `NIFTYNEXT50` 87,375 of 87,375, `NIFTYPVTBANK` 61,088 of 61,088 — and
/// `NIFTY`, `BANKNIFTY` and `RELIANCE` hold none beyond it, of 796,027,
/// 795,979 and 795,320.
///
/// In all, 228,367 stored minutes on 31 series lie beyond it. Sixteen of
/// those series are shares holding exactly ONE such minute, on a day their
/// served day bar does not contain it by more than 8% (`ABB`, `VEDL`, `TMPV`,
/// `JIOFIN`, `HYUNDAI` and eleven more). Whether the vendor's day bar or its
/// minute is the wrong one there is UNVERIFIED; the refusal names the minute
/// either way.
pub const FOREIGN_BAND_BPS: i64 = 800;

/// How far outside before a minute is counted as suspect and named, without
/// being refused.
///
/// # Why 200
///
/// The same measurement: a series' own minutes sit a little outside their day
/// because the vendor adjusts and rounds the two rungs apart, and the largest
/// span-wide breach of that kind is under 100 bps (`RELIANCE`, 96,962 minutes,
/// the largest 93.1; `NIFTY` 29.7). 200 is twice that. Between 200 and 800 the
/// store holds 2,244 minutes on 18 series, among them `NIFTY100LIQ15` 848,
/// `NIFTYGSCOMPSITE` 383, one whole `PATANJALI` session of 375, `NIFTYGS48YR`
/// 150, `NIFTYGS813YR` 108 and `INDIAVIX` 99: some another index's, some the
/// series' own, and nothing here can tell which.
pub const SUSPECT_BAND_BPS: i64 = 200;

/// The IST calendar day a stamp in UTC microseconds falls on, as days since
/// the epoch: the arithmetic [`crate::daycheck`] uses.
#[must_use]
pub const fn ist_day(ts_micros: i64) -> i64 {
    (ts_micros.div_euclid(1_000_000) + crate::session::IST_OFFSET_SECS).div_euclid(86_400)
}

/// One served day: its IST day and its range, in paisa.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct DayRange {
    day: i64,
    low: i64,
    high: i64,
}

/// Whether the series' first served day bar is known.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FirstDay {
    /// The day rung holds no bar for this series at all.
    #[default]
    NoDayRung,
    /// The IST day of the first bar the day rung holds.
    On(i64),
}

/// Why a minute was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cause {
    /// Its day is before the first day bar the series was served.
    BeforeFirstDayBar {
        /// That first day, as IST days since the epoch.
        first: i64,
    },
    /// Its whole range lies outside its day's by more than the band.
    ForeignLevel {
        /// The served day's low, in paisa.
        day_low: i64,
        /// The served day's high, in paisa.
        day_high: i64,
    },
}

impl Cause {
    /// The code a receipt line, a ledger line and a log event carry.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BeforeFirstDayBar { .. } => "before_first_day_bar",
            Self::ForeignLevel { .. } => "foreign_level",
        }
    }
}

/// Why a minute could not be judged. It lands, and it is counted here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unchecked {
    /// The day rung holds nothing for the series: nothing to judge against.
    NoDayRung,
    /// The day rung holds an earlier day and not this one.
    NoDayBar,
    /// The day bar's range is not a range: a bound at or below zero, or a low
    /// above its high. A minute is not refused on the word of such a bar.
    DayBarUnusable,
}

impl Unchecked {
    /// The code a log event carries.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::NoDayRung => "no_day_rung",
            Self::NoDayBar => "no_day_bar",
            Self::DayBarUnusable => "day_bar_unusable",
        }
    }
}

/// What the day rung says about one minute.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    /// Inside its served day's range, or within [`SUSPECT_BAND_BPS`] of it.
    Held,
    /// Outside its day's range by more than [`SUSPECT_BAND_BPS`] and no more
    /// than [`FOREIGN_BAND_BPS`]: it lands, counted and named.
    Suspect {
        /// The served day's low, in paisa.
        day_low: i64,
        /// The served day's high, in paisa.
        day_high: i64,
    },
    /// Not this series on this day.
    Refused(Cause),
    /// Not judged, and why.
    Unchecked(Unchecked),
}

/// The served day bars a minute answer is judged against.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DayBook {
    /// Ascending by day, one entry a day.
    days: Vec<DayRange>,
    first: FirstDay,
}

impl DayBook {
    /// A book holding `days` — the served day bars of every month the answer
    /// touches, in any order — for a series whose first served day is `first`.
    ///
    /// Two bars on one IST day keep the first offered: a day rung holds one
    /// bar a day, and a second is not this module's to choose between.
    #[must_use]
    pub fn new(days: &[Bar], first: FirstDay) -> Self {
        let mut days: Vec<DayRange> = days
            .iter()
            .map(|bar| DayRange {
                day: ist_day(bar.ts_micros),
                low: bar.low,
                high: bar.high,
            })
            .collect();
        days.sort_by_key(|range| range.day);
        days.dedup_by_key(|range| range.day);
        let first = match (first, days.first()) {
            // A held day earlier than the stated first is the first.
            (FirstDay::On(stated), Some(range)) => FirstDay::On(stated.min(range.day)),
            (FirstDay::NoDayRung, Some(range)) => FirstDay::On(range.day),
            (stated, None) => stated,
        };
        Self { days, first }
    }

    /// The series' first served day, as this book knows it.
    #[must_use]
    pub const fn first(&self) -> FirstDay {
        self.first
    }

    /// Whether the book holds a bar for `day`.
    #[must_use]
    pub fn holds(&self, day: i64) -> bool {
        self.days
            .binary_search_by_key(&day, |range| range.day)
            .is_ok()
    }

    /// The verdict on a minute of IST day `day` whose range is `low..=high`,
    /// in paisa.
    #[must_use]
    pub fn judge(&self, day: i64, low: i64, high: i64) -> Verdict {
        let FirstDay::On(first) = self.first else {
            return Verdict::Unchecked(Unchecked::NoDayRung);
        };
        if day < first {
            return Verdict::Refused(Cause::BeforeFirstDayBar { first });
        }
        let Ok(at) = self.days.binary_search_by_key(&day, |range| range.day) else {
            return Verdict::Unchecked(Unchecked::NoDayBar);
        };
        let Some(range) = self.days.get(at) else {
            return Verdict::Unchecked(Unchecked::NoDayBar);
        };
        if range.low <= 0 || range.high < range.low {
            return Verdict::Unchecked(Unchecked::DayBarUnusable);
        }
        // Widened in i128: a paisa bound times a band cannot overflow there.
        let outside = |bps: i64| {
            let band = |bound: i64| i128::from(bound) * i128::from(bps) / 10_000;
            i128::from(low) > i128::from(range.high) + band(range.high)
                || i128::from(high) < i128::from(range.low) - band(range.low)
        };
        let (day_low, day_high) = (range.low, range.high);
        if outside(FOREIGN_BAND_BPS) {
            Verdict::Refused(Cause::ForeignLevel { day_low, day_high })
        } else if outside(SUSPECT_BAND_BPS) {
            Verdict::Suspect { day_low, day_high }
        } else {
            Verdict::Held
        }
    }
}

/// The code a suspect minute is counted and named under.
pub const SUSPECT_CODE: &str = "suspect_level";

/// One named group of a scan: one code on one IST day.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Flagged {
    /// The IST day, as days since the epoch.
    pub day: i64,
    /// [`Cause::code`] for refused minutes, [`SUSPECT_CODE`] for suspect ones.
    pub code: &'static str,
    /// Whether a landing refuses these minutes: false for suspect ones.
    pub refused: bool,
    /// Minutes of this cause on this day.
    pub minutes: u64,
    /// The first such minute's stamp, UTC microseconds.
    pub first_ts_micros: i64,
}

/// What a scan of stored minutes found.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tally {
    /// Minutes judged and held.
    pub held: u64,
    /// Minutes before the series' first served day bar.
    pub before_first_day_bar: u64,
    /// Minutes wholly outside their day's range by more than
    /// [`FOREIGN_BAND_BPS`].
    pub foreign_level: u64,
    /// Minutes outside by more than [`SUSPECT_BAND_BPS`] and no more than
    /// [`FOREIGN_BAND_BPS`]: landed, and named.
    pub suspect_level: u64,
    /// Minutes the day rung could not judge because it holds nothing.
    pub no_day_rung: u64,
    /// Minutes on a day the day rung does not hold, though it holds an earlier.
    pub no_day_bar: u64,
    /// Minutes whose day bar is not a usable range.
    pub day_bar_unusable: u64,
    /// One entry per (day, code) refused or suspect, in the order met.
    pub flagged: Vec<Flagged>,
}

impl Tally {
    /// Minutes a landing would refuse.
    #[must_use]
    pub const fn refused(&self) -> u64 {
        self.before_first_day_bar + self.foreign_level
    }

    /// Minutes that were not judged.
    #[must_use]
    pub const fn unchecked(&self) -> u64 {
        self.no_day_rung + self.no_day_bar + self.day_bar_unusable
    }

    /// Every minute seen.
    #[must_use]
    pub const fn minutes(&self) -> u64 {
        self.held + self.suspect_level + self.refused() + self.unchecked()
    }

    /// Count one verdict on the minute stamped `ts_micros` of IST day `day`.
    /// The group is looked for among the last groups of the same day, as
    /// [`crate::fetch::Refused::note`] does: O(1) for minutes in time order.
    ///
    /// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
    /// "Cost claims of the /db fix round that name no proof").
    pub fn count(&mut self, verdict: Verdict, day: i64, ts_micros: i64) {
        match verdict {
            Verdict::Held => self.held += 1,
            Verdict::Unchecked(Unchecked::NoDayRung) => self.no_day_rung += 1,
            Verdict::Unchecked(Unchecked::NoDayBar) => self.no_day_bar += 1,
            Verdict::Unchecked(Unchecked::DayBarUnusable) => self.day_bar_unusable += 1,
            Verdict::Suspect { .. } => {
                self.suspect_level += 1;
                self.flag(SUSPECT_CODE, false, day, ts_micros);
            }
            Verdict::Refused(cause) => {
                match cause {
                    Cause::BeforeFirstDayBar { .. } => self.before_first_day_bar += 1,
                    Cause::ForeignLevel { .. } => self.foreign_level += 1,
                }
                self.flag(cause.code(), true, day, ts_micros);
            }
        }
    }

    fn flag(&mut self, code: &'static str, refused: bool, day: i64, ts_micros: i64) {
        for group in self.flagged.iter_mut().rev() {
            if group.day != day {
                break;
            }
            if group.code == code {
                group.minutes += 1;
                return;
            }
        }
        self.flagged.push(Flagged {
            day,
            code,
            refused,
            minutes: 1,
            first_ts_micros: ts_micros,
        });
    }

    /// Fold another scan's counts into these.
    pub fn absorb(&mut self, other: Self) {
        self.held += other.held;
        self.before_first_day_bar += other.before_first_day_bar;
        self.foreign_level += other.foreign_level;
        self.suspect_level += other.suspect_level;
        self.no_day_rung += other.no_day_rung;
        self.no_day_bar += other.no_day_bar;
        self.day_bar_unusable += other.day_bar_unusable;
        self.flagged.extend(other.flagged);
    }
}

/// Every stored minute of `minutes` judged against `book`: the same verdicts
/// the landing door gives, for bars that are already on disk.
#[must_use]
pub fn audit(minutes: &[Bar], book: &DayBook) -> Tally {
    let mut tally = Tally::default();
    for bar in minutes {
        let day = ist_day(bar.ts_micros);
        tally.count(book.judge(day, bar.low, bar.high), day, bar.ts_micros);
    }
    tally
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::{
        Cause, DayBook, FOREIGN_BAND_BPS, FirstDay, SUSPECT_BAND_BPS, SUSPECT_CODE, Unchecked,
        Verdict, audit, ist_day,
    };
    use store::format::Bar;

    /// 09:15 IST on a day chosen for the arithmetic, seconds since the epoch.
    const OPEN: i64 = 1_791_000_000 - (1_791_000_000 + 19_800) % 86_400 + 33_300;

    fn at(day: i64, minute: i64) -> i64 {
        (OPEN + day * 86_400 + minute * 60) * 1_000_000
    }

    /// A bar with the invented range `low..=high`.
    fn bar(ts_micros: i64, low: i64, high: i64) -> Bar {
        Bar {
            ts_micros,
            open: low,
            high,
            low,
            close: high,
            volume: 7,
            open_interest: i64::MIN,
        }
    }

    /// A day bar stamped at IST midnight, as a day rung is.
    fn day_bar(day: i64, low: i64, high: i64) -> Bar {
        bar(at(day, 0) - 33_300 * 1_000_000, low, high)
    }

    fn day(n: i64) -> i64 {
        ist_day(at(n, 0))
    }

    #[test]
    fn a_day_bar_at_ist_midnight_and_its_minutes_are_one_day() {
        assert_eq!(ist_day(day_bar(3, 1, 2).ts_micros), day(3));
        assert_eq!(ist_day(at(3, 374)), day(3));
        assert_eq!(day(4), day(3) + 1);
    }

    #[test]
    fn a_minute_inside_its_day_is_held_and_one_at_either_edge_of_the_band_too() {
        let book = DayBook::new(&[day_bar(0, 100_000, 110_000)], FirstDay::NoDayRung);
        assert_eq!(book.judge(day(0), 104_000, 105_000), Verdict::Held);
        // The suspect band is 2% of the nearer bound: 2,200 above, 2,000 below.
        assert_eq!(SUSPECT_BAND_BPS, 200);
        assert_eq!(book.judge(day(0), 112_200, 112_300), Verdict::Held);
        assert_eq!(book.judge(day(0), 97_900, 98_000), Verdict::Held);
        // Straddling a bound is this series: only a WHOLE range outside is not.
        assert_eq!(book.judge(day(0), 109_000, 190_000), Verdict::Held);
        assert_eq!(book.judge(day(0), 10_000, 100_500), Verdict::Held);
    }

    #[test]
    fn a_minute_between_the_two_bands_is_suspect_and_lands() {
        let book = DayBook::new(&[day_bar(0, 100_000, 110_000)], FirstDay::NoDayRung);
        let suspect = Verdict::Suspect {
            day_low: 100_000,
            day_high: 110_000,
        };
        // One paisa past the suspect band, and exactly on the refusing one:
        // 8% of the nearer bound is 8,800 above and 8,000 below.
        assert_eq!(book.judge(day(0), 112_201, 112_300), suspect);
        assert_eq!(book.judge(day(0), 97_000, 97_999), suspect);
        assert_eq!(book.judge(day(0), 118_800, 118_900), suspect);
        assert_eq!(book.judge(day(0), 91_000, 92_000), suspect);
    }

    #[test]
    fn a_minute_one_paisa_past_the_band_is_refused_by_name_on_either_side() {
        let book = DayBook::new(&[day_bar(0, 100_000, 110_000)], FirstDay::NoDayRung);
        let foreign = Verdict::Refused(Cause::ForeignLevel {
            day_low: 100_000,
            day_high: 110_000,
        });
        assert_eq!(FOREIGN_BAND_BPS, 800);
        assert_eq!(book.judge(day(0), 118_801, 118_900), foreign);
        assert_eq!(book.judge(day(0), 91_000, 91_999), foreign);
        // A tenth of the level, twice it, and thirty-seven times it.
        assert_eq!(book.judge(day(0), 10_400, 10_500), foreign);
        assert_eq!(book.judge(day(0), 208_000, 210_000), foreign);
        assert_eq!(book.judge(day(0), 3_900_000, 3_910_000), foreign);
        assert_eq!(
            Cause::ForeignLevel {
                day_low: 1,
                day_high: 2
            }
            .code(),
            "foreign_level"
        );
    }

    #[test]
    fn a_minute_before_the_first_served_day_is_refused_whatever_its_level() {
        let book = DayBook::new(&[day_bar(5, 100_000, 110_000)], FirstDay::NoDayRung);
        assert_eq!(book.first(), FirstDay::On(day(5)));
        let early = Verdict::Refused(Cause::BeforeFirstDayBar { first: day(5) });
        assert_eq!(book.judge(day(4), 104_000, 105_000), early);
        assert_eq!(book.judge(day(-900), 3_900_000, 3_910_000), early);
        assert_eq!(
            Cause::BeforeFirstDayBar { first: 0 }.code(),
            "before_first_day_bar"
        );
        // The first day itself is judged on its level like any other.
        assert_eq!(book.judge(day(5), 104_000, 105_000), Verdict::Held);
    }

    #[test]
    fn the_first_day_may_be_in_a_month_the_answer_does_not_touch() {
        // No day bar of the touched months, and a first day stated from the
        // series' first held month: earlier is refused, later is unchecked.
        let book = DayBook::new(&[], FirstDay::On(day(40)));
        assert_eq!(
            book.judge(day(39), 1, 2),
            Verdict::Refused(Cause::BeforeFirstDayBar { first: day(40) })
        );
        assert_eq!(
            book.judge(day(41), 1, 2),
            Verdict::Unchecked(Unchecked::NoDayBar)
        );
        // A held day earlier than the stated first corrects it.
        let book = DayBook::new(&[day_bar(30, 5, 6)], FirstDay::On(day(40)));
        assert_eq!(book.first(), FirstDay::On(day(30)));
    }

    #[test]
    fn a_series_with_no_day_rung_is_unchecked_never_refused_and_never_held() {
        let book = DayBook::new(&[], FirstDay::NoDayRung);
        assert_eq!(book.first(), FirstDay::NoDayRung);
        assert_eq!(
            book.judge(day(0), 1, 2),
            Verdict::Unchecked(Unchecked::NoDayRung)
        );
        assert_eq!(Unchecked::NoDayRung.code(), "no_day_rung");
        assert!(!book.holds(day(0)));
    }

    #[test]
    fn a_day_the_rung_lacks_after_its_first_is_unchecked_by_name() {
        let book = DayBook::new(
            &[day_bar(0, 100, 110), day_bar(2, 100, 110)],
            FirstDay::NoDayRung,
        );
        assert!(book.holds(day(0)) && book.holds(day(2)) && !book.holds(day(1)));
        // A missing day in the middle, and one after the last held.
        assert_eq!(
            book.judge(day(1), 9_000, 9_100),
            Verdict::Unchecked(Unchecked::NoDayBar)
        );
        assert_eq!(
            book.judge(day(3), 9_000, 9_100),
            Verdict::Unchecked(Unchecked::NoDayBar)
        );
        assert_eq!(Unchecked::NoDayBar.code(), "no_day_bar");
    }

    #[test]
    fn a_day_bar_that_is_not_a_range_refuses_nothing() {
        // Zero prices and an inverted range: both are met in served day rungs.
        let book = DayBook::new(
            &[day_bar(0, 0, 0), day_bar(1, 120, 110), day_bar(2, -5, 110)],
            FirstDay::NoDayRung,
        );
        for n in 0..3 {
            assert_eq!(
                book.judge(day(n), 50_000, 50_100),
                Verdict::Unchecked(Unchecked::DayBarUnusable),
                "day {n}"
            );
        }
        assert_eq!(Unchecked::DayBarUnusable.code(), "day_bar_unusable");
    }

    #[test]
    fn day_bars_out_of_order_and_duplicated_make_the_same_book() {
        let sorted = DayBook::new(
            &[day_bar(0, 100, 110), day_bar(1, 200, 220)],
            FirstDay::NoDayRung,
        );
        let shuffled = DayBook::new(
            &[
                day_bar(1, 200, 220),
                day_bar(0, 100, 110),
                day_bar(1, 200, 220),
            ],
            FirstDay::NoDayRung,
        );
        assert_eq!(sorted, shuffled);
    }

    #[test]
    fn the_widest_prices_do_not_overflow_the_band() {
        let book = DayBook::new(&[day_bar(0, i64::MAX - 1, i64::MAX)], FirstDay::NoDayRung);
        assert_eq!(book.judge(day(0), i64::MAX, i64::MAX), Verdict::Held);
        assert_eq!(
            book.judge(day(0), 1, 2),
            Verdict::Refused(Cause::ForeignLevel {
                day_low: i64::MAX - 1,
                day_high: i64::MAX
            })
        );
    }

    #[test]
    fn a_scan_counts_every_minute_once_and_groups_refusals_by_day_and_cause() {
        let book = DayBook::new(
            &[
                day_bar(2, 100_000, 110_000),
                day_bar(3, 100_000, 110_000),
                day_bar(5, 0, 0),
            ],
            FirstDay::NoDayRung,
        );
        let minutes = [
            bar(at(0, 0), 3_900_000, 3_910_000), // before the first day
            bar(at(0, 1), 3_900_000, 3_910_000), // the same day, the same cause
            bar(at(2, 0), 104_000, 105_000),     // held
            bar(at(2, 1), 10_400, 10_500),       // a tenth of the level
            bar(at(2, 2), 104_000, 105_000),     // held, between two foreign
            bar(at(2, 3), 10_400, 10_500),       // the same day and cause again
            bar(at(3, 0), 208_000, 210_000),     // twice the level, another day
            bar(at(3, 1), 114_000, 115_000),     // 3.6% above: suspect, lands
            bar(at(4, 0), 104_000, 105_000),     // a day the rung lacks
            bar(at(5, 0), 104_000, 105_000),     // a day bar of zeros
        ];
        let tally = audit(&minutes, &book);
        assert_eq!(tally.minutes(), 10);
        assert_eq!(
            (tally.held, tally.before_first_day_bar, tally.foreign_level),
            (2, 2, 3)
        );
        assert_eq!(tally.suspect_level, 1);
        assert_eq!(
            (tally.no_day_rung, tally.no_day_bar, tally.day_bar_unusable),
            (0, 1, 1)
        );
        assert_eq!((tally.refused(), tally.unchecked()), (5, 2));
        let groups: Vec<(i64, &str, bool, u64, i64)> = tally
            .flagged
            .iter()
            .map(|g| (g.day, g.code, g.refused, g.minutes, g.first_ts_micros))
            .collect();
        assert_eq!(
            groups,
            vec![
                (day(0), "before_first_day_bar", true, 2, at(0, 0)),
                (day(2), "foreign_level", true, 2, at(2, 1)),
                (day(3), "foreign_level", true, 1, at(3, 0)),
                (day(3), SUSPECT_CODE, false, 1, at(3, 1)),
            ]
        );
    }

    #[test]
    fn an_empty_scan_and_a_scan_with_no_day_rung_refuse_nothing() {
        let none = audit(&[], &DayBook::new(&[], FirstDay::NoDayRung));
        assert_eq!(none.minutes(), 0);
        assert!(none.flagged.is_empty());
        let unjudged = audit(
            &[bar(at(0, 0), 1, 2)],
            &DayBook::new(&[], FirstDay::NoDayRung),
        );
        assert_eq!((unjudged.no_day_rung, unjudged.refused()), (1, 0));
    }

    #[test]
    fn two_scans_fold_into_one() {
        let book = DayBook::new(&[day_bar(1, 100_000, 110_000)], FirstDay::NoDayRung);
        let mut first = audit(&[bar(at(0, 0), 1, 2)], &book);
        let second = audit(
            &[
                bar(at(1, 0), 104_000, 105_000),
                bar(at(1, 1), 10_400, 10_500),
                bar(at(1, 2), 114_000, 115_000),
            ],
            &book,
        );
        first.absorb(second);
        assert_eq!(
            (first.held, first.before_first_day_bar, first.foreign_level),
            (1, 1, 1)
        );
        assert_eq!((first.suspect_level, first.minutes()), (1, 4));
        assert_eq!(first.flagged.len(), 3);
    }
}
