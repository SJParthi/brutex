//! The NSE trading calendar, **measured from the exchange's own record**.
//!
//! # Why this exists, and why it could not be written before
//!
//! [`crate::session`] says, and still says, that it holds no calendar: *"It does
//! not know a holiday, a Muhurat session or a Saturday budget session"*. That
//! was the honest position while nothing here had a **sourced** list — golden
//! rule 1 forbids inventing one, and a weekend rule would have been actively
//! wrong, because 2020-02-01, 2025-02-01 and 2026-02-01 are a Saturday, a
//! Saturday and a Sunday on which NSE traded a full session.
//!
//! The cost of that absence was measured on 2026-08-22 and it was not small: the
//! `/ingest` page computes an expected bar count arithmetically, knew only 24
//! non-trading days across six and a half years, and therefore reported all six
//! stored series as **SHORT** when every one of them was complete. An operator
//! cannot tell a real hole from a public holiday, so every alarm is noise and a
//! true gap hides among them.
//!
//! # Where this list comes from
//!
//! **Not from a webpage.** `nseindia.com` is unreachable from the environment
//! this was built in — the documentation fetch timed out and the browser refused
//! the host — so nothing here was typed off a published calendar.
//!
//! It is derived from **what actually traded**: every day carrying a `1day` bar
//! in the operator's own Zerodha store over 2019-12-02 … 2026-08-21. Three
//! independent instruments were walked — NIFTY, BANKNIFTY and INDIAVIX — and
//! they agree on the **identical** 1,671-day set, which is the corroboration
//! that makes this a measurement rather than one feed's opinion.
//!
//! It reconciles exactly, and that arithmetic is the proof rather than a
//! reassurance:
//!
//! ```text
//! 1,755  weekdays in the window
//! -  92  days no instrument traded  (13.7/yr, the NSE norm)
//! +   8  weekend days all three DID trade
//! = 1,671  exactly the count on disk
//! ```
//!
//! The 92 are self-validating: Republic Day, Independence Day, Gandhi Jayanti,
//! Christmas, Good Friday, Holi and Mahashivratri all fall where they should,
//! and the list also contains one-offs no rule would generate — 2024-01-22, the
//! Ram Mandir consecration, and 2024-05-20, the Mumbai general-election day.
//!
//! # What it refuses to answer
//!
//! **Anything outside the measured window returns [`DayKind::Unmeasured`].**
//! The operator's next backfill reaches 2015 and this calendar does not, because
//! no bar from 2015 has been seen here. Answering "closed" for an unmeasured day
//! would be the invention rule 1 bans, and answering "open" would manufacture a
//! session. It says it does not know, which is the only honest third answer, and
//! `CLAUDE.md` §4 prefers it loudly over either guess.
//!
//! **Extending it is a measurement, never a typed date.** Pull the earlier
//! years, re-derive from the store, widen the range.
//!
//! # Cost
//!
//! One subtraction, one array index and one bit test — O(1), no hash, no search,
//! no allocation. The bitset is 307 bytes for 2,455 days and lives in
//! `.rodata`. The four non-standard sessions are a fixed four-element table
//! whose length is a compile-time constant, so the walk over it is a bounded
//! constant and not a scan that grows.

/// First day this calendar knows: **2019-12-02**, as days since the epoch.
pub const FIRST_DAY: i64 = 18_232;

/// Last day this calendar knows: **2026-08-21**, as days since the epoch.
pub const LAST_DAY: i64 = 20_686;

/// How many days the bitset covers.
pub const DAYS: usize = 2_455;

// TWO ASSERTIONS AND NO CAST. One `DAYS as i64` would say the same thing and
// would be the workspace's only sign-losing cast on a path that never needs
// one; both sides are compared against the literal instead.
const _: () = assert!(LAST_DAY - FIRST_DAY + 1 == 2_455);
const _: () = assert!(DAYS == 2_455);

/// Minute-of-day a standard NSE equity session opens: **09:15**.
pub const OPEN_MINUTE: u16 = 9 * 60 + 15;

/// Minute-of-day a standard session's LAST bar opens: **15:29**.
///
/// The close is exclusive — 15:30 is `AtOrAfterSessionClose` in
/// [`crate::session`] — so the last bar of a full day opens at 15:29 and a full
/// day is [`FULL_BARS`] long.
pub const LAST_MINUTE: u16 = 15 * 60 + 29;

/// Bars in a standard session: **375**.
pub const FULL_BARS: u16 = LAST_MINUTE - OPEN_MINUTE + 1;

const _: () = assert!(FULL_BARS == 375);

/// One trading window: the minute its first bar opens, and its last.
///
/// A day has one of these normally and two on a disaster-recovery Saturday,
/// which is why [`Session`] carries an array rather than a pair of fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Window {
    /// Minute-of-day the first bar opens.
    pub from: u16,
    /// Minute-of-day the last bar opens, inclusive.
    pub to: u16,
}

impl Window {
    /// How many one-minute bars this window holds.
    #[must_use]
    pub const fn bars(self) -> u16 {
        self.to.saturating_sub(self.from).saturating_add(1)
    }

    /// Whether `minute` falls inside this window.
    #[must_use]
    pub const fn holds(self, minute: u16) -> bool {
        minute >= self.from && minute <= self.to
    }
}

/// The most windows any measured session has: **two**.
///
/// The disaster-recovery Saturdays of 2024-03-02 and 2024-05-18 traded
/// 09:15–09:59 and again 11:30–12:29. Nothing measured here has three, and a
/// third would need this constant raised and the tables rebuilt from bars —
/// which is the point of pinning it rather than using a `Vec`.
pub const MAX_WINDOWS: usize = 2;

/// What one session actually held.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Session {
    /// The windows that traded, `count` of them used.
    pub windows: [Window; MAX_WINDOWS],
    /// How many of `windows` are real.
    pub count: u8,
}

impl Session {
    /// Every bar this session should hold, across all its windows.
    #[must_use]
    pub fn bars(self) -> u16 {
        let mut total = 0_u16;
        let mut i = 0_usize;
        while i < self.count as usize && i < MAX_WINDOWS {
            if let Some(w) = self.windows.get(i) {
                total = total.saturating_add(w.bars());
            }
            i += 1;
        }
        total
    }

    /// Whether a bar at `minute` is one this session should hold.
    ///
    /// **This is the question a gap classifier asks.** A minute inside a window
    /// and absent from the store is a hole; a minute outside every window was
    /// never expected and is not one.
    #[must_use]
    pub fn expects(self, minute: u16) -> bool {
        let mut i = 0_usize;
        while i < self.count as usize && i < MAX_WINDOWS {
            if self.windows.get(i).is_some_and(|w| w.holds(minute)) {
                return true;
            }
            i += 1;
        }
        false
    }

    /// A standard 09:15–15:29 session.
    #[must_use]
    pub const fn full() -> Self {
        Self {
            windows: [
                Window {
                    from: OPEN_MINUTE,
                    to: LAST_MINUTE,
                },
                Window { from: 0, to: 0 },
            ],
            count: 1,
        }
    }
}

/// What a day was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayKind {
    /// The exchange traded, and this is what it held.
    Open(Session),
    /// The exchange did not trade. A weekend or a holiday — this calendar does
    /// not distinguish them, because the store cannot: both are simply days on
    /// which no instrument produced a bar, and naming one a "holiday" would be
    /// a claim about WHY that no measurement here supports.
    Closed,
    /// Outside [`FIRST_DAY`]…[`LAST_DAY`]. **Not a guess in either direction.**
    Unmeasured,
}

/// The four sessions in the measured window that are not 09:15–15:29.
///
/// Each was read off the stored bars, not off a description of the event. The
/// names in the comments are identifications from shape and date and are NOT
/// vendor-confirmed labels — `CLAUDE.md` §3 rule 1 keeps that distinction, so
/// the DATA here binds and the prose beside it does not.
const IRREGULAR: [(i64, Session); 4] = [
    // 2021-02-24 — traded 09:15 and stopped at 10:08. The shape of an exchange
    // outage rather than a scheduled short day: no announced session ends at
    // 10:08.
    (
        18_682,
        Session {
            windows: [Window { from: 555, to: 608 }, Window { from: 0, to: 0 }],
            count: 1,
        },
    ),
    // 2024-03-02 (Sat) — two windows. A disaster-recovery live test.
    (
        19_784,
        Session {
            windows: [Window { from: 555, to: 599 }, Window { from: 690, to: 749 }],
            count: 2,
        },
    ),
    // 2024-05-18 (Sat) — the same two windows, the same kind of test.
    (
        19_861,
        Session {
            windows: [Window { from: 555, to: 599 }, Window { from: 690, to: 749 }],
            count: 2,
        },
    ),
    // 2025-10-21 — one hour, in the afternoon, on a weekday the market was
    // otherwise shut. Muhurat.
    (
        20_382,
        Session {
            windows: [Window { from: 825, to: 884 }, Window { from: 0, to: 0 }],
            count: 1,
        },
    ),
];

/// Whether each day in the window traded. One bit per day, LSB first.
///
/// Derived from the `1day` bars of NIFTY, BANKNIFTY and INDIAVIX, which agree.
static TRADED: [u8; 307] = [
    0x9F, 0xCF, 0x67, 0xF3, 0xF9, 0x7C, 0x3E, 0xBF, 0xCF, 0xE7, 0xF1, 0xF9, 0x74, 0x3E, 0x9F, 0x8B,
    0xA3, 0xF3, 0x79, 0x7C, 0x3E, 0x1F, 0xCF, 0xE7, 0xF3, 0xF9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE7, 0xF3,
    0xF9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE7, 0xF1, 0xF9, 0x7C, 0x3E, 0x9F, 0x9F, 0xE7, 0xE3, 0xF9, 0x7C,
    0x1E, 0x9F, 0xCF, 0xE7, 0xD3, 0xF9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE5, 0xF3, 0x71, 0x7C, 0x36, 0x9B,
    0xCF, 0xE7, 0xF2, 0xF9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE7, 0xF3, 0xD9, 0x7C, 0x3E, 0x9F, 0xCB, 0xE7,
    0xF3, 0xF8, 0x7C, 0x3E, 0x9F, 0xC7, 0xE7, 0xF3, 0xF8, 0x3C, 0x3E, 0x9F, 0xCF, 0xE7, 0xF3, 0xF9,
    0x7C, 0x3E, 0x9B, 0xCF, 0xE7, 0xF3, 0xE9, 0x7C, 0x1E, 0x9F, 0xCF, 0xE7, 0xF0, 0xF9, 0x74, 0x3E,
    0x9F, 0xCF, 0xE7, 0xF3, 0xF9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE7, 0xD3, 0xF1, 0x7C, 0x36, 0x9F, 0xCF,
    0xE7, 0xB3, 0xF9, 0x7C, 0x36, 0x9F, 0xCE, 0xE7, 0xF3, 0xF9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE7, 0x73,
    0xF9, 0x7C, 0x3E, 0x9F, 0x4F, 0xE7, 0xF3, 0xB9, 0x34, 0x1E, 0x9F, 0x8F, 0xE7, 0xF3, 0xF9, 0x7C,
    0x3E, 0x9F, 0xCF, 0xE5, 0xF3, 0xF9, 0x7C, 0x3E, 0x9F, 0xCE, 0xE7, 0xF3, 0xF9, 0x74, 0x3E, 0x9E,
    0xCF, 0xA7, 0xF3, 0xF9, 0x76, 0x3E, 0x9E, 0xCF, 0xE7, 0xE3, 0xF9, 0x7C, 0x7E, 0x8E, 0xCF, 0xE7,
    0xF3, 0xF9, 0x3D, 0x3E, 0x1F, 0xC7, 0xE7, 0xB2, 0xF9, 0x6C, 0x3E, 0x3F, 0xCF, 0xE7, 0xF3, 0xF1,
    0x7C, 0x3E, 0x9F, 0xCD, 0xE7, 0xF3, 0xB9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE7, 0xB3, 0xF9, 0x7C, 0x3E,
    0x9F, 0xCF, 0x63, 0xF3, 0xF9, 0x7C, 0x3E, 0x9B, 0xCF, 0xE7, 0xF3, 0xF9, 0x7D, 0x3E, 0x9F, 0xCD,
    0xE7, 0xF1, 0xF9, 0x78, 0x2E, 0x8E, 0xCF, 0xE5, 0xF3, 0xF9, 0x7C, 0x3E, 0x9F, 0xCF, 0xE7, 0xF3,
    0xF9, 0x7C, 0x3E, 0x9F, 0xC7, 0x67, 0xF3, 0xF9, 0x7C, 0x3E, 0x97, 0xCF, 0x67, 0xF3, 0xD9, 0x7C,
    0x3E, 0x9F, 0xCF, 0xE7, 0x73, 0xF9, 0x7C, 0x2E, 0x1F, 0xEF, 0xE7, 0xF3, 0xF9, 0x74, 0x3E, 0x9F,
    0x4B, 0xE3, 0xD3, 0xF9, 0x3C, 0x3E, 0x9F, 0xCF, 0xE5, 0xF3, 0xF9, 0x3C, 0x3E, 0x9F, 0xCF, 0xE7,
    0xF3, 0xF9, 0x7C,
];

const _: () = assert!(TRADED.len() == DAYS.div_ceil(8));

/// What `epoch_day` was: an open session with its windows, a closed day, or
/// outside what has been measured.
///
/// # Cost
///
/// One compare, one subtract, one index, one shift — and, only for a day that
/// traded, a walk of the four-element [`IRREGULAR`] table whose length is a
/// compile-time constant. O(1), no hash, no allocation.
#[must_use]
pub fn kind_of(epoch_day: i64) -> DayKind {
    if !(FIRST_DAY..=LAST_DAY).contains(&epoch_day) {
        return DayKind::Unmeasured;
    }
    // `try_from` RATHER THAN `as`, and the failure arm is honest rather than a
    // panic. The range check above already makes the difference 0..=2454, so
    // this cannot fail; if a future edit breaks that, saying "I have not
    // measured this day" is the safe answer and `expect` is banned outright.
    let Ok(offset) = usize::try_from(epoch_day - FIRST_DAY) else {
        return DayKind::Unmeasured;
    };
    let byte = TRADED.get(offset / 8).copied().unwrap_or(0);
    if byte & (1_u8 << (offset % 8)) == 0 {
        return DayKind::Closed;
    }
    let mut i = 0_usize;
    while i < IRREGULAR.len() {
        match IRREGULAR.get(i) {
            Some(&(day, session)) if day == epoch_day => return DayKind::Open(session),
            _ => i += 1,
        }
    }
    DayKind::Open(Session::full())
}

/// How many bars `epoch_day` should hold, or `None` where it is unmeasured.
///
/// `Some(0)` for a closed day is deliberate and is not the same answer as
/// `None`: one says the exchange was shut, the other says nobody has looked.
#[must_use]
pub fn expected_bars(epoch_day: i64) -> Option<u16> {
    match kind_of(epoch_day) {
        DayKind::Open(session) => Some(session.bars()),
        DayKind::Closed => Some(0),
        DayKind::Unmeasured => None,
    }
}

/// Trading days in `first ..= last`, or `None` if any part is unmeasured.
///
/// **Refuses a partially-measured range rather than under-counting it.** A
/// caller asking about 2015 gets `None`, not a total that silently begins in
/// December 2019 — which is the shape of wrong answer that made every series
/// read SHORT in the first place.
#[must_use]
pub fn sessions_between(first: i64, last: i64) -> Option<u32> {
    if first < FIRST_DAY || last > LAST_DAY || first > last {
        return None;
    }
    let mut count = 0_u32;
    let mut day = first;
    while day <= last {
        if matches!(kind_of(day), DayKind::Open(_)) {
            count = count.saturating_add(1);
        }
        day += 1;
    }
    Some(count)
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod tests {
    use super::*;

    /// **THE ARITHMETIC THAT MAKES THIS A MEASUREMENT AND NOT A LIST.**
    ///
    /// 1,755 weekdays, minus 92 days nothing traded, plus 8 weekend days all
    /// three instruments did trade, is 1,671 — the exact count of `1day` bars on
    /// disk. A typed calendar can be plausible; only a reconciling one can be
    /// checked, and this is the check.
    #[test]
    fn the_calendar_reconciles_to_the_bars_it_was_derived_from() {
        let mut open = 0_u32;
        let mut closed = 0_u32;
        let mut weekend_sessions = 0_u32;
        let mut weekday_total = 0_u32;

        for day in FIRST_DAY..=LAST_DAY {
            // 1970-01-01 was a Thursday, so day 0 is weekday index 3 and
            // Saturday/Sunday are 2 and 3 in this rotation.
            let dow = (day + 4).rem_euclid(7); // 0 = Sunday
            let weekend = dow == 0 || dow == 6;
            if !weekend {
                weekday_total += 1;
            }
            match kind_of(day) {
                DayKind::Open(_) => {
                    open += 1;
                    if weekend {
                        weekend_sessions += 1;
                    }
                }
                DayKind::Closed => {
                    if !weekend {
                        closed += 1;
                    }
                }
                DayKind::Unmeasured => panic!("day {day} is inside the range and unmeasured"),
            }
        }

        assert_eq!(weekday_total, 1_755, "weekdays in 2019-12-02..=2026-08-21");
        assert_eq!(
            closed, 92,
            "weekdays on which nothing traded — NSE holidays"
        );
        assert_eq!(weekend_sessions, 8, "Budget Saturdays, Muhurat, DR tests");
        assert_eq!(
            open, 1_671,
            "and the total is exactly the number of 1day bars the store holds"
        );
        assert_eq!(
            weekday_total - closed + weekend_sessions,
            open,
            "1755 - 92 + 8 = 1671, or this calendar is not the one the bars describe"
        );
    }

    /// **OUTSIDE THE MEASURED WINDOW IT SAYS SO, RATHER THAN GUESSING EITHER
    /// WAY.**
    ///
    /// The next backfill reaches 2015 and this calendar does not. Answering
    /// "closed" would invent a holiday; answering "open" would invent a session.
    /// `CLAUDE.md` §3 rule 1 leaves exactly one honest answer, and
    /// `expected_bars` must distinguish it from a real zero: `Some(0)` means the
    /// exchange was shut, `None` means nobody has looked.
    #[test]
    fn an_unmeasured_day_is_a_third_answer_and_not_a_closed_one() {
        let before = FIRST_DAY - 1;
        let after = LAST_DAY + 1;
        assert_eq!(kind_of(before), DayKind::Unmeasured);
        assert_eq!(kind_of(after), DayKind::Unmeasured);
        assert_eq!(expected_bars(before), None, "not Some(0)");
        assert_eq!(expected_bars(after), None, "not Some(0)");

        // 2015-01-01 — the floor the operator's next pull reaches.
        assert_eq!(kind_of(16_436), DayKind::Unmeasured);

        // AND A RANGE THAT ONLY PARTLY OVERLAPS IS REFUSED WHOLE. A total that
        // silently began in December 2019 is the shape of wrong answer that
        // made six complete series read SHORT.
        assert_eq!(sessions_between(16_436, LAST_DAY), None);
        assert!(sessions_between(FIRST_DAY, LAST_DAY).is_some());
    }

    /// **THE FOUR IRREGULAR SESSIONS ARE THE ONES THE STORE MEASURED.**
    ///
    /// Each bar count was read off the stored timestamps. A calendar that got
    /// these wrong would report a Muhurat hour as 315 missing bars, which is
    /// precisely the false alarm this module exists to stop.
    #[test]
    fn every_irregular_session_holds_the_bars_the_store_holds() {
        // 2021-02-24 — the outage. 09:15 to 10:08.
        assert_eq!(expected_bars(18_682), Some(54));
        // The two disaster-recovery Saturdays: 45 + 60.
        assert_eq!(expected_bars(19_784), Some(105));
        assert_eq!(expected_bars(19_861), Some(105));
        // Muhurat: one hour, and it does not start at 09:15.
        assert_eq!(expected_bars(20_382), Some(60));

        let DayKind::Open(muhurat) = kind_of(20_382) else {
            panic!("2025-10-21 traded");
        };
        assert!(!muhurat.expects(OPEN_MINUTE), "09:15 was NOT a Muhurat bar");
        assert!(muhurat.expects(13 * 60 + 45), "13:45 was");
        assert!(muhurat.expects(14 * 60 + 44), "and 14:44 was the last");
        assert!(!muhurat.expects(14 * 60 + 45), "14:45 was not");

        // AND THE TWO-WINDOW DAY HAS A REAL HOLE IN THE MIDDLE that is not a
        // gap: 10:30 sits between the windows and was never expected.
        let DayKind::Open(dr) = kind_of(19_784) else {
            panic!("2024-03-02 traded");
        };
        assert_eq!(dr.count, 2);
        assert!(dr.expects(9 * 60 + 59), "end of the first window");
        assert!(!dr.expects(10 * 60 + 30), "between the windows — not owed");
        assert!(dr.expects(11 * 60 + 30), "start of the second");
    }

    /// **AN ORDINARY DAY IS 375 BARS FROM 09:15 TO 15:29, AND 15:30 IS NOT ONE.**
    ///
    /// The close is exclusive — `session::DropReason::AtOrAfterSessionClose`
    /// fires *at* 15:30 — so a calendar that expected a 15:30 bar would report
    /// one missing bar on every single trading day.
    #[test]
    fn a_full_session_ends_at_1529_because_the_close_is_exclusive() {
        // 2026-08-03, an ordinary Monday inside the window.
        let DayKind::Open(full) = kind_of(20_668) else {
            panic!("an ordinary weekday traded");
        };
        assert_eq!(full.bars(), 375);
        assert!(full.expects(OPEN_MINUTE), "09:15 is the first bar");
        assert!(full.expects(LAST_MINUTE), "15:29 is the last");
        assert!(
            !full.expects(LAST_MINUTE + 1),
            "15:30 is NOT owed — expecting it would report a phantom hole on \
             every one of the 1,671 days"
        );
        assert!(!full.expects(OPEN_MINUTE - 1), "09:14 is pre-open");
    }
}
