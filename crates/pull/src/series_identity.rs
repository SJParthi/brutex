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

use std::collections::HashSet;
use store::format::Bar;
use store::path::YearMonth;

/// How far a minute's whole range may lie outside its served day's range, in
/// basis points of the nearer bound, before it is refused as another series'.
///
/// # Why 800
///
/// It is the audit's own threshold (DB-B60, DB-B64: a minute more than 8% off
/// its day). Measured on the operator's store, 11 Oct 2026, read-only, all
/// 330 Zerodha series holding both rungs, each 1min bar against its own 1day
/// (the WP-13d package's fix record, kept outside this repository, names the
/// method): every minute the
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

/// How far a CASH minute's open may lie from the close of the minute kept
/// before it, inside one continuous recorded run, before it is refused as
/// another price basis interleaved under the key (R4-17, RT-DATA-02 a).
///
/// Measured in basis points of the SMALLER of the two prices
/// ([`jump_bps`]), so a basis change by a factor trips alike in both
/// directions: measured against the earlier close alone, a halving reads 50%
/// and a doubling 100%, and in an interleave the wrong side would be the one
/// refused.
///
/// # Measured, 17 Oct 2026, on the operator's store
///
/// 3,000 was chosen first, by a session with no store, and then measured
/// against the store before it was trusted: `cli series-identity ALL
/// --segment CASH --feed zerodha --workers 4` over a private shadow whose bars
/// are the operator's (read-only), every stored Zerodha CASH minute — 208
/// shares, 153,548,887 minutes. Per share, the 99th percentile of
/// [`jump_bps`] across continuous minutes is at most 188 bps (median share
/// 11) and the 99.9th at most 270 bps outside PIIND. The largest jump of any
/// share whose minutes nothing else accuses is 2,203 bps (RADICO): 3,000 sits
/// 1.36 times above it. The two larger maxima are interleaves: PIIND 270,496
/// bps and VBL 10,003 bps, both on minutes the day rung already refuses as
/// `foreign_level`. No minute anywhere tripped this rule (0 in-session jumps),
/// so no split or bonus ex-date of the basis-run shares trips it either.
/// The numbers are in `docs/06-limits.md` (R4-17).
pub const JUMP_BPS: i64 = 3_000;

/// The jump from `close` to `open`, in basis points of the smaller of the
/// two, or `None` when either is not a positive price. Widened in `i128`, so no
/// paisa pair overflows, and saturated into `i64`.
#[must_use]
pub fn jump_bps(close: i64, open: i64) -> Option<i64> {
    if close <= 0 || open <= 0 {
        return None;
    }
    let smaller = i128::from(close.min(open));
    let gap = (i128::from(open) - i128::from(close)).abs();
    Some(i64::try_from(gap * 10_000 / smaller).unwrap_or(i64::MAX))
}

/// One continuous recorded run of minutes, and the close of the minute last
/// KEPT in it: the reference a new minute's open is judged against.
///
/// A run breaks wherever two recorded minutes are not exactly one grid step
/// apart — a halt, a missing minute, the night — so a session's first minute
/// and the first after any gap are never judged (09:15 is never a jump). A
/// minute refused for any cause is recorded (it keeps the run continuous) but
/// never becomes the reference, so a block of foreign minutes is refused
/// whole, not only at its edges.
///
/// **What this cannot see.** When a run's FIRST minute is the foreign one, it
/// becomes the reference and the security's own minutes after it are the ones
/// refused, loudly and by name. The day rung's [`Cause::ForeignLevel`] judges
/// the same minutes independently ONLY when it holds their day: with the day
/// month absent or unreadable, that first minute is stored unflagged
/// (D-4993 point 3, `docs/06-limits.md` R4-17).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct JumpRun {
    last_second: Option<i64>,
    reference: Option<i64>,
}

impl JumpRun {
    /// Record the minute stamped `second` (epoch seconds, any fixed offset)
    /// on a grid of `step` seconds, and answer the jump of its `open` from the
    /// run's reference: `Some(bps)` when there is a reference to judge against.
    pub fn judge(&mut self, second: i64, step: i64, open: i64) -> Option<i64> {
        let continuous = self
            .last_second
            .is_some_and(|last| second.checked_sub(last) == Some(step));
        self.last_second = Some(second);
        if !continuous {
            self.reference = None;
        }
        self.reference.and_then(|close| jump_bps(close, open))
    }

    /// The minute last judged was kept: its close is the run's reference.
    pub const fn keep(&mut self, close: i64) {
        self.reference = Some(close);
    }
}

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
    /// An open, high, low or close at or below zero (R4-17, ING-01 c).
    NonPositivePrice,
    /// Its day is before the master's listing day for its exact symbol and
    /// ISIN ([`crate::listing`], D-5839).
    BeforeListing {
        /// The listing day, as IST days since the epoch.
        listed_on: i64,
    },
    /// Its open lies more than [`JUMP_BPS`] from the close of the minute kept
    /// before it in one continuous run ([`JumpRun`]).
    InSessionJump {
        /// That close, in paisa.
        reference: i64,
        /// The jump, in basis points of the smaller price.
        bps: i64,
    },
}

impl Cause {
    /// The code a receipt line, a ledger line and a log event carry.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::BeforeFirstDayBar { .. } => "before_first_day_bar",
            Self::ForeignLevel { .. } => "foreign_level",
            Self::NonPositivePrice => "non_positive_price",
            Self::BeforeListing { .. } => crate::listing::BEFORE_LISTING,
            Self::InSessionJump { .. } => "in_session_jump",
        }
    }
}

/// Whether any of a bar's four prices is at or below zero paisa.
#[must_use]
pub const fn non_positive(open: i64, high: i64, low: i64, close: i64) -> bool {
    open <= 0 || high <= 0 || low <= 0 || close <= 0
}

/// Why a minute could not be judged. It lands, and it is counted here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Unchecked {
    /// The day rung holds nothing for the series, or the day month that
    /// would witness this minute would not open: nothing to judge against.
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
    /// Every day month that would not open; `None` when none (no map is
    /// made for the book of an answer whose months all opened).
    unread: Option<HashSet<YearMonth>>,
    /// The first IST day of the earliest of them, as days since the epoch.
    unread_from: Option<i64>,
}

/// The first IST day of `month`, as days since the epoch; `i64::MIN`, the
/// safe side, for a month no day can name (none, for a [`YearMonth`]).
fn month_start(month: YearMonth) -> i64 {
    crate::session::Day::new(month.year(), month.month(), 1)
        .map_or(i64::MIN, |day| i64::from(day.days_from_epoch()))
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
        Self {
            days,
            first,
            unread: None,
            unread_from: None,
        }
    }

    /// The same book, told which day months would not open (R4-17 repair 2).
    /// A minute of such a month is [`Unchecked::NoDayRung`], never judged by
    /// range; and while one of them lies before the first day this book
    /// read, no minute before that day is refused as
    /// [`Cause::BeforeFirstDayBar`], since the month that would not open may
    /// hold the series' first day. Every other minute is judged as before,
    /// so one torn month costs the range rule of its own days only.
    #[must_use]
    pub fn with_unread(mut self, months: &[YearMonth]) -> Self {
        // COLLECTED FROM AN EXACT-SIZE ITERATOR: the set is sized once.
        self.unread = (!months.is_empty()).then(|| months.iter().copied().collect());
        self.unread_from = months.iter().copied().map(month_start).min();
        self
    }

    /// Whether `day` falls in a day month that would not open: the day's
    /// month by calendar arithmetic and one hash probe, so the same cost with
    /// one torn month or a hundred (C-ZI-03).
    fn unread_holds(&self, day: i64) -> bool {
        self.unread.as_ref().is_some_and(|torn| {
            u32::try_from(day)
                .ok()
                .and_then(|days| crate::session::Day::from_days(days).ok())
                .and_then(|on| on.year_month().ok())
                .is_some_and(|month| torn.contains(&month))
        })
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
            // A MONTH THAT WOULD NOT OPEN, EARLIER THAN THE FIRST DAY READ,
            // may hold the series' first day: nothing before it is refused.
            if self.unread_from.is_some_and(|from| from < first) {
                return Verdict::Unchecked(Unchecked::NoDayRung);
            }
            return Verdict::Refused(Cause::BeforeFirstDayBar { first });
        }
        let Ok(at) = self.days.binary_search_by_key(&day, |range| range.day) else {
            return Verdict::Unchecked(if self.unread_holds(day) {
                Unchecked::NoDayRung
            } else {
                Unchecked::NoDayBar
            });
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
    /// Bars with a price at or below zero.
    pub non_positive_price: u64,
    /// Bars before the master's listing day.
    pub before_listing: u64,
    /// Minutes whose open jumps from the run's reference by more than
    /// [`JUMP_BPS`].
    pub in_session_jump: u64,
    /// One entry per (day, code) refused or suspect, in the order met.
    pub flagged: Vec<Flagged>,
}

impl Tally {
    /// Minutes a landing would refuse.
    #[must_use]
    pub const fn refused(&self) -> u64 {
        self.before_first_day_bar
            + self.foreign_level
            + self.non_positive_price
            + self.before_listing
            + self.in_session_jump
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
                    Cause::NonPositivePrice => self.non_positive_price += 1,
                    Cause::BeforeListing { .. } => self.before_listing += 1,
                    Cause::InSessionJump { .. } => self.in_session_jump += 1,
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
        self.non_positive_price += other.non_positive_price;
        self.before_listing += other.before_listing;
        self.in_session_jump += other.in_session_jump;
        self.flagged.extend(other.flagged);
    }
}

/// Every stored minute of `minutes` judged against `book`: the same verdicts
/// the landing door gives, for bars that are already on disk.
#[must_use]
pub fn audit(minutes: &[Bar], book: &DayBook) -> Tally {
    let mut judge = MinuteJudge::new(book, Rules::default(), 60);
    let mut tally = Tally::default();
    for bar in minutes {
        let day = ist_day(bar.ts_micros);
        let (verdict, _) = judge.judge(bar.ts_micros.div_euclid(1_000_000), day, prices(bar));
        tally.count(verdict, day, bar.ts_micros);
    }
    tally
}

/// A bar's open, high, low and close, in that order.
#[must_use]
pub const fn prices(bar: &Bar) -> [i64; 4] {
    [bar.open, bar.high, bar.low, bar.close]
}

/// What a series' identity rules need beyond its day book (R4-17).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Rules {
    /// The master's listing day, as IST days since the epoch, when a refusal
    /// may rest on it ([`crate::listing::Span::listed_on`]). When it is known,
    /// a minute before the first served day bar but on or after it is the
    /// series' own and is not refused as [`Cause::BeforeFirstDayBar`].
    pub listed_on: Option<i64>,
    /// Whether [`JUMP_BPS`] applies: CASH minutes only, the series it was
    /// measured over.
    pub jumps: bool,
    /// Whether a value of EXACTLY zero is the series' own and is held: an
    /// INDEX. Measured on the operator's store (17 Oct 2026): every
    /// zero-valued Zerodha INDEX day bar, 153 of them, is `NIFTY50DIVPOINT`'s,
    /// in runs that begin the session after a March expiry or in April and
    /// rise from there — an index that restarts from zero. Refusing them
    /// would delete the index's own values. A value BELOW zero is refused on
    /// every series (D-4992).
    pub zero_held: bool,
}

/// Whether any of a bar's four prices is below zero paisa.
#[must_use]
pub const fn negative(open: i64, high: i64, low: i64, close: i64) -> bool {
    open < 0 || high < 0 || low < 0 || close < 0
}

/// A day bar judged by the day-rung arm (R4-17): ONLY a non-positive price
/// or a day before the listing, never by range — the day rung is the witness
/// the minute ranges are judged against, not the accused. On an index
/// ([`Rules::zero_held`]) only a price below zero is refused.
#[must_use]
pub fn judge_day(day: i64, [open, high, low, close]: [i64; 4], rules: Rules) -> Option<Cause> {
    let refused = if rules.zero_held {
        negative(open, high, low, close)
    } else {
        non_positive(open, high, low, close)
    };
    if refused {
        return Some(Cause::NonPositivePrice);
    }
    match rules.listed_on {
        Some(listed_on) if day < listed_on => Some(Cause::BeforeListing { listed_on }),
        _ => None,
    }
}

/// Minutes in time order judged against one day book, with the run state the
/// jump rule needs: the one implementation the landing door and the store
/// scan share.
#[derive(Debug, Clone)]
pub struct MinuteJudge<'a> {
    book: &'a DayBook,
    rules: Rules,
    run: JumpRun,
    step: i64,
}

impl<'a> MinuteJudge<'a> {
    /// A judge over `book` under `rules`, for minutes on a grid of `step`
    /// seconds.
    #[must_use]
    pub const fn new(book: &'a DayBook, rules: Rules, step: i64) -> Self {
        Self {
            book,
            rules,
            run: JumpRun {
                last_second: None,
                reference: None,
            },
            step,
        }
    }

    /// The verdict on the minute stamped `second` (epoch seconds) of IST day
    /// `day`, and its jump from the run's reference when it had one and the
    /// jump rule applies (whether or not it tripped), for the distribution a
    /// scan reports.
    pub fn judge(&mut self, second: i64, day: i64, ohlc: [i64; 4]) -> (Verdict, Option<i64>) {
        let [open, high, low, close] = ohlc;
        let jump = self
            .run
            .judge(second, self.step, open)
            .filter(|_| self.rules.jumps);
        if let Some(cause) = judge_day(day, ohlc, self.rules) {
            return (Verdict::Refused(cause), None);
        }
        let verdict = match self.book.judge(day, low, high) {
            // THE LISTING, NEVER THE FIRST DAY BAR, WHEN A MASTER STATES IT
            // (D-5839): a minute between the two is the series' own.
            Verdict::Refused(Cause::BeforeFirstDayBar { .. }) if self.rules.listed_on.is_some() => {
                Verdict::Unchecked(Unchecked::NoDayBar)
            }
            verdict => verdict,
        };
        if matches!(verdict, Verdict::Refused(_)) {
            return (verdict, jump);
        }
        if let (Some(bps), Some(reference)) = (jump, self.run.reference)
            && bps > JUMP_BPS
        {
            return (
                Verdict::Refused(Cause::InSessionJump { reference, bps }),
                jump,
            );
        }
        self.run.keep(close);
        (verdict, jump)
    }
}

#[cfg(test)]
#[allow(clippy::indexing_slicing, clippy::expect_used, clippy::unwrap_used)]
mod tests {
    use super::{
        Cause, DayBook, FOREIGN_BAND_BPS, FirstDay, JUMP_BPS, JumpRun, MinuteJudge, Rules,
        SUSPECT_BAND_BPS, SUSPECT_CODE, Tally, Unchecked, Verdict, audit, ist_day, judge_day,
        jump_bps, negative, non_positive, prices,
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

    /// R4-17 repair 2 (the check's PROBE-P1): a day month that would not
    /// open costs the range rule of its own days only, and a refusal before
    /// the first day read only while no such month lies before that day.
    #[test]
    fn a_day_month_that_would_not_open_costs_its_own_days_only() {
        use store::path::YearMonth;
        let on = |y: u16, m: u8, d: u8| {
            i64::from(crate::session::Day::new(y, m, d).unwrap().days_from_epoch())
        };
        let midnight = |d: i64| bar((d * 86_400 - 19_800) * 1_000_000, 100_000, 110_000);
        let month = |y: u16, m: u8| YearMonth::new(y, m).unwrap();
        let unread = Verdict::Unchecked(Unchecked::NoDayRung);
        // February 2024 read; January would not open.
        let book = DayBook::new(
            &[midnight(on(2024, 2, 1)), midnight(on(2024, 2, 2))],
            FirstDay::NoDayRung,
        )
        .with_unread(&[month(2024, 1)]);
        assert_eq!(book.first(), FirstDay::On(on(2024, 2, 1)));
        // February still judges its own minutes by range.
        assert_eq!(
            book.judge(on(2024, 2, 1), 10_400, 10_500),
            Verdict::Refused(Cause::ForeignLevel {
                day_low: 100_000,
                day_high: 110_000,
            })
        );
        assert_eq!(book.judge(on(2024, 2, 2), 104_000, 105_000), Verdict::Held);
        // Every day of the torn month, its first and its last, is unjudged.
        for d in [on(2024, 1, 1), on(2024, 1, 30), on(2024, 1, 31)] {
            assert_eq!(book.judge(d, 10_400, 10_500), unread);
        }
        // Before the torn month and before the first day read: that month
        // may hold the series' first day, so nothing is refused on it.
        assert_eq!(book.judge(on(2023, 12, 29), 10_400, 10_500), unread);
        // After the last day read, outside every torn month: as before.
        assert_eq!(
            book.judge(on(2024, 2, 5), 1, 2),
            Verdict::Unchecked(Unchecked::NoDayBar)
        );
        // A torn month AFTER the first day read leaves that refusal standing.
        let later = DayBook::new(&[midnight(on(2024, 2, 1))], FirstDay::NoDayRung)
            .with_unread(&[month(2024, 3)]);
        assert_eq!(
            later.judge(on(2024, 1, 31), 1, 2),
            Verdict::Refused(Cause::BeforeFirstDayBar {
                first: on(2024, 2, 1)
            })
        );
        assert_eq!(later.judge(on(2024, 3, 1), 1, 2), unread);
        assert_eq!(later.judge(on(2024, 3, 31), 1, 2), unread);
        for d in [on(2024, 2, 29), on(2024, 4, 1)] {
            assert_eq!(
                later.judge(d, 1, 2),
                Verdict::Unchecked(Unchecked::NoDayBar)
            );
        }
        // Many torn months, repeated and out of order, December into January
        // across the year: each is its own, and a month between them is not.
        let many = DayBook::new(&[], FirstDay::On(on(2020, 1, 1))).with_unread(&[
            month(2024, 1),
            month(2024, 3),
            month(2023, 12),
            month(2024, 1),
        ]);
        assert_eq!(many.unread_from, Some(on(2023, 12, 1)));
        for d in [
            on(2023, 12, 1),
            on(2024, 1, 31),
            on(2024, 3, 1),
            on(2024, 3, 31),
        ] {
            assert_eq!(many.judge(d, 1, 2), unread);
        }
        for d in [
            on(2023, 11, 30),
            on(2024, 2, 1),
            on(2024, 2, 29),
            on(2024, 4, 1),
        ] {
            assert_eq!(many.judge(d, 1, 2), Verdict::Unchecked(Unchecked::NoDayBar));
        }
        // A torn month before the first day read lifts the refusal before it;
        // one on that very first month's start would not.
        let earliest =
            DayBook::new(&[], FirstDay::On(on(2024, 1, 1))).with_unread(&[month(2023, 12)]);
        assert_eq!(earliest.judge(on(2023, 11, 1), 1, 2), unread);
        let same = DayBook::new(&[], FirstDay::On(on(2024, 1, 1))).with_unread(&[month(2024, 1)]);
        assert_eq!(
            same.judge(on(2023, 12, 31), 1, 2),
            Verdict::Refused(Cause::BeforeFirstDayBar {
                first: on(2024, 1, 1)
            })
        );
        // The first day of a month, and of the last month a store path names.
        assert_eq!(super::month_start(month(2024, 2)), on(2024, 2, 1));
        assert_eq!(super::month_start(month(9_999, 12)), on(9_999, 12, 1));
        // No torn month: the book is the book it was.
        let plain = DayBook::new(&[midnight(on(2024, 2, 1))], FirstDay::NoDayRung);
        assert_eq!(plain.clone().with_unread(&[]), plain);
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

    // R4-17: the jump, the run, the listing and the day arm.

    #[test]
    fn a_jump_is_measured_against_the_smaller_price_either_way() {
        assert_eq!(jump_bps(100, 100), Some(0));
        assert_eq!(jump_bps(100, 200), Some(10_000));
        assert_eq!(
            jump_bps(200, 100),
            Some(10_000),
            "a halving reads as a doubling"
        );
        assert_eq!(jump_bps(82_165, 8_990), Some(81_395), "PIIND's interleave");
        assert_eq!(jump_bps(10_000, 13_000), Some(JUMP_BPS));
        assert_eq!(
            jump_bps(10_000, 13_001),
            Some(JUMP_BPS + 1),
            "the first that trips"
        );
        assert_eq!(
            jump_bps(13_001, 10_000),
            Some(JUMP_BPS + 1),
            "the same, downwards"
        );
        for (close, open) in [(0, 5), (5, 0), (-1, 5), (5, -1)] {
            assert_eq!(jump_bps(close, open), None, "{close} -> {open}");
        }
        assert_eq!(
            jump_bps(1, i64::MAX),
            Some(i64::MAX),
            "saturated, never wrapped"
        );
    }

    #[test]
    fn a_run_breaks_on_any_gap_and_judges_against_the_last_kept_close() {
        let mut run = JumpRun::default();
        assert_eq!(
            run.judge(600, 60, 100),
            None,
            "a run's first minute is never judged"
        );
        run.keep(100);
        assert_eq!(run.judge(660, 60, 150), Some(5_000));
        // Not kept: the reference stays at 100.
        assert_eq!(run.judge(720, 60, 101), Some(100));
        run.keep(101);
        assert_eq!(run.judge(780, 60, 101), Some(0));
        // A missing minute, a stamp out of order and a stamp repeated each break it.
        for second in [900, 840, 840] {
            assert_eq!(run.judge(second, 60, 500), None, "{second}");
        }
        run.keep(500);
        assert_eq!(run.judge(900, 60, 500), Some(0));
        assert_eq!(
            run.judge(i64::MIN, 60, 500),
            None,
            "no overflow on any stamp"
        );
    }

    #[test]
    fn the_day_arm_refuses_a_non_positive_price_and_a_day_before_listing_only() {
        let listed = Rules {
            listed_on: Some(day(2)),
            jumps: false,
            zero_held: false,
        };
        assert_eq!(
            judge_day(day(2), [1, 1, 1, 1], listed),
            None,
            "the listing day itself"
        );
        assert_eq!(
            judge_day(day(1), [5, 6, 4, 5], listed),
            Some(Cause::BeforeListing { listed_on: day(2) })
        );
        assert_eq!(judge_day(day(1), [5, 6, 4, 5], Rules::default()), None);
        for ohlc in [
            [0, 1, 1, 1],
            [1, 0, 1, 1],
            [1, 1, 0, 1],
            [1, 1, 1, 0],
            [1, 1, -3, 1],
        ] {
            assert!(non_positive(ohlc[0], ohlc[1], ohlc[2], ohlc[3]));
            assert_eq!(
                judge_day(day(3), ohlc, listed),
                Some(Cause::NonPositivePrice)
            );
        }
        // Any positive range lands, however wild: never judged by range.
        assert_eq!(judge_day(day(3), [1, i64::MAX, 1, 9], listed), None);
        assert!(!non_positive(1, 1, 1, 1));
    }

    /// An index's zero is its own (`NIFTY50DIVPOINT` restarts from zero) and
    /// is held; below zero is refused on an index as on every series.
    #[test]
    fn an_index_zero_is_held_and_a_price_below_zero_is_refused() {
        let index = Rules {
            zero_held: true,
            ..Rules::default()
        };
        for ohlc in [[0, 0, 0, 0], [0, 1, 0, 1], [1, 1, 1, 0]] {
            assert!(!negative(ohlc[0], ohlc[1], ohlc[2], ohlc[3]));
            assert_eq!(judge_day(day(3), ohlc, index), None, "{ohlc:?}");
            assert_eq!(
                judge_day(day(3), ohlc, Rules::default()),
                Some(Cause::NonPositivePrice),
                "{ohlc:?} off an index"
            );
        }
        for ohlc in [[-1, 1, 1, 1], [1, -1, 1, 1], [1, 1, -1, 1], [1, 1, 1, -1]] {
            assert!(negative(ohlc[0], ohlc[1], ohlc[2], ohlc[3]));
            assert_eq!(
                judge_day(day(3), ohlc, index),
                Some(Cause::NonPositivePrice),
                "{ohlc:?}"
            );
        }
        // The listing still applies to an index's rules as written.
        let listed = Rules {
            listed_on: Some(day(4)),
            ..index
        };
        assert_eq!(
            judge_day(day(3), [0, 0, 0, 0], listed),
            Some(Cause::BeforeListing { listed_on: day(4) })
        );
    }

    #[test]
    fn a_minute_is_judged_price_listing_day_rung_then_jump() {
        let book = DayBook::new(
            &[day_bar(2, 9_000, 11_000), day_bar(3, 9_000, 11_000)],
            FirstDay::NoDayRung,
        );
        let rules = Rules {
            listed_on: Some(day(1)),
            jumps: true,
            zero_held: false,
        };
        let mut judge = MinuteJudge::new(&book, rules, 60);
        let second = |d: i64, m: i64| at(d, m) / 1_000_000;
        let ohlc = |price: i64| [price, price, price, price];
        // Before the listing: refused as before_listing, never before_first_day_bar.
        assert_eq!(
            judge.judge(second(0, 0), day(0), ohlc(10_000)).0,
            Verdict::Refused(Cause::BeforeListing { listed_on: day(1) })
        );
        // On the listing day, before the first day bar: the series' own, unchecked.
        assert_eq!(
            judge.judge(second(1, 0), day(1), ohlc(10_000)).0,
            Verdict::Unchecked(Unchecked::NoDayBar)
        );
        // A zero price before anything else.
        assert_eq!(
            judge
                .judge(second(2, 0), day(2), [0, 10_000, 10_000, 10_000])
                .0,
            Verdict::Refused(Cause::NonPositivePrice)
        );
        assert_eq!(
            judge.judge(second(2, 1), day(2), ohlc(10_000)),
            (Verdict::Held, None)
        );
        assert_eq!(
            judge.judge(second(2, 2), day(2), ohlc(10_100)),
            (Verdict::Held, Some(100))
        );
        // Inside the day's range and beyond JUMP_BPS of the kept close.
        let jumped = judge.judge(second(2, 3), day(2), [13_200, 13_200, 10_500, 10_500]);
        assert_eq!(
            jumped,
            (
                Verdict::Refused(Cause::InSessionJump {
                    reference: 10_100,
                    bps: 3_069
                }),
                Some(3_069)
            )
        );
        // Wholly outside the day: the day rung's refusal, with the jump measured.
        let foreign = judge.judge(second(2, 4), day(2), ohlc(1_000));
        assert_eq!(
            foreign,
            (
                Verdict::Refused(Cause::ForeignLevel {
                    day_low: 9_000,
                    day_high: 11_000
                }),
                Some(91_000)
            )
        );
        assert_eq!(
            judge.judge(second(2, 5), day(2), ohlc(10_100)),
            (Verdict::Held, Some(0))
        );
        // A jump of exactly JUMP_BPS lands: only one beyond it is refused.
        assert_eq!(
            judge
                .judge(second(2, 6), day(2), [10_000, 10_000, 10_000, 10_000])
                .0,
            Verdict::Held
        );
        assert_eq!(
            judge.judge(second(2, 7), day(2), [13_000, 13_000, 10_500, 10_500]),
            (Verdict::Held, Some(JUMP_BPS))
        );
        // Without the jump rule nothing is measured, and without a listing the
        // first-day-bar rule refuses.
        let mut plain = MinuteJudge::new(&book, Rules::default(), 60);
        assert_eq!(
            plain.judge(second(1, 0), day(1), ohlc(10_000)).0,
            Verdict::Refused(Cause::BeforeFirstDayBar { first: day(2) })
        );
        assert_eq!(
            plain.judge(second(2, 0), day(2), ohlc(10_000)),
            (Verdict::Held, None)
        );
        assert_eq!(
            plain.judge(second(2, 1), day(2), ohlc(10_900)),
            (Verdict::Held, None)
        );
    }

    #[test]
    fn every_new_cause_is_counted_named_and_folded() {
        let mut tally = Tally::default();
        let causes = [
            Cause::NonPositivePrice,
            Cause::BeforeListing { listed_on: day(1) },
            Cause::InSessionJump {
                reference: 1,
                bps: 9,
            },
        ];
        for (n, cause) in causes.into_iter().enumerate() {
            tally.count(
                Verdict::Refused(cause),
                day(i64::try_from(n).unwrap()),
                at(0, 0),
            );
        }
        assert_eq!(
            causes.map(Cause::code),
            ["non_positive_price", "before_listing", "in_session_jump"]
        );
        assert_eq!(
            (
                tally.non_positive_price,
                tally.before_listing,
                tally.in_session_jump
            ),
            (1, 1, 1)
        );
        assert_eq!((tally.refused(), tally.minutes()), (3, 3));
        let mut sum = tally.clone();
        sum.absorb(tally);
        assert_eq!(
            (
                sum.non_positive_price,
                sum.before_listing,
                sum.in_session_jump,
                sum.refused()
            ),
            (2, 2, 2, 6)
        );
        assert_eq!(sum.flagged.len(), 6);
        assert_eq!(prices(&bar(0, 3, 4)), [3, 4, 3, 4]);
    }
}
