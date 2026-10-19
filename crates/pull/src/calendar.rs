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
//! The open-day set is not typed from a webpage. `nseindia.com` was unreachable
//! from the environment this was built in, so it is derived from the store as
//! described below. There is one deliberately different evidence lane:
//! **2021-02-24's trading windows come from SEBI's primary record**, because all
//! three stored series end at 10:08 while SEBI records trading until 11:40 and
//! again from 15:45 to 17:00. Letting three truncated index series define the
//! exchange session would erase the evening reopening. The exact index-
//! publication denominator is a separate, explicitly unmeasured question in
//! [`crate::gaps::classify_spot_index_against`].
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
//! The baseline now extends through 2026-09-04, and only through that date.
//! NSE/CMTR/71775 (2025-12-12), capital-market TRADING holidays for 2026,
//! https://nsearchives.nseindia.com/content/circulars/CMTR71775.pdf, lists no
//! trading holiday in this extension. The charter's regular weekday schedule
//! supplies the hours, not the observed candles. Independently checked NIFTY
//! and BANKNIFTY minute windows each contain exactly 375 distinct consecutive
//! stamps, 09:15–15:29 IST, on Aug 24–28 and Aug 31–Sep 4. The four intervening
//! dates are ordinary scheduled weekends (Aug 22–23, 29–30); their closure is
//! the bounded schedule classification, not an inference from missing rows.
//! These ten sessions add ten weekdays: 1,765 - 92 + 8 = 1,681 open dates.
//! Earlier bits and exceptional/unknown-length sessions are unchanged. No
//! weekday rule is applied outside this fixed, corroborated extension.
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
//! # Five days traded and have no minute series at all
//!
//! The store holds **1,671** days of daily bars and **1,666** of minute bars.
//! The difference is every Diwali **Muhurat** session before 2025 —
//! 2020-11-14, 2021-11-04, 2022-10-24, 2023-11-12, 2024-11-01 — each with a
//! daily bar and not one minute bar. Zerodha's minute history does not reach
//! them.
//!
//! **That last sentence is wrong, and is kept so the correction can be read
//! beside it (D-3352, WP-13c).** It was measured on a store pulled before the
//! answers that carried those days had landed. Kite returned sixty one-minute
//! rows for NIFTY on each of the five days in the 6–7 Oct 2026 pulls
//! (`audit/pull.journal` records 1421354, 1421380, 1421401, 1421480 and
//! 1421519, each "60 minute row(s) refused";
//! `state/gdfl-board/zerodha-holes.md` §2C). The store lacks them because the
//! landing refused them (D-3344) and each month then held later days; their
//! windows are recorded now (D-3319) and a versioned repair adds them
//! (D-3350).
//!
//! **The count is stale as well, and is kept beside its correction (WP-62,
//! repair 3).** 1,666 was 1,671 less those five Muhurats. Re-measured on
//! 2026-10-11 (`/calendar.json` for Zerodha NIFTY 1min and the bar files
//! themselves, read-only): in 2019-12-02..2026-08-21 the calendar has 1,671
//! sessions and NIFTY holds a minute bar on **1,631** of them. The 40 it does
//! not hold are the five Muhurats and 35 days of last-month holes the landing
//! refused (the first 2020-12-01; DB-A12). Every one of the 1,671 holds a
//! daily bar. 1,631 is what the store holds until those days are pulled again;
//! it is a measurement of the store, not of what NSE traded.
//!
//! They get their own answer, [`DayKind::OpenLengthUnmeasured`], because both
//! neighbours are wrong: a full session would claim 375 owed bars each and
//! report 1,875 losses that were never offered, and a closed day would deny a
//! session the daily bar proves happened.
//!
//! **This was found only after an audit had already reported the store
//! complete.** That audit grouped absences by days that HAD minute bars, so a
//! day with none was invisible to it, and "28 minutes missing" was an
//! undercount by an order of magnitude. The test
//! `a_muhurat_with_no_minute_series_owes_an_unknown_number_and_not_375` exists
//! so the same blind spot cannot return.
//!
//! # The calendar of record: special sessions, halts, phases (D-3319, D-3320)
//!
//! Those five windows, and the two Muhurats of the D-2805 spans, are now
//! recorded — the charter's §3 Muhurat table, whose 2018, 2019 and 2023 rows
//! are the vendor's GDFL ticks measured on 9 Oct 2026 ([`MUHURAT_TICKS`]) — in
//! [`SPECIAL_SESSIONS`], beside the DR drills, the outage day, the Budget
//! weekends and 2024-01-20. Three market-wide halts are in [`HALTS`]:
//! 2021-02-24 sourced from SEBI, and the two March 2020 circuit breakers
//! measured and **suspected**, UNVERIFIED until an NSE circular is recorded.
//! [`exchange_kind_of`] answers them; [`phase_at`] and [`phases_of`] give each
//! minute of a day its phase per segment — pre-open, continuous, halted,
//! reopening auction, the closing auction from 2026-08-03 for an eligible
//! share (never for an index, D-3320), post-close; and
//! [`expiry_day`], [`settlement_on_or_before`] and [`shift_reason`] answer
//! the expiry questions. [`kind_of`] and [`sourced_kind_of`] keep every
//! answer they gave: why is written above [`PRE_OPEN_MINUTE`].
//!
//! # Cost
//!
//! One subtraction, one array index and one bit test — O(1), no hash, no search,
//! no allocation. The bitset is 309 bytes for 2,469 days and lives in
//! `.rodata`. The four irregular sessions and the five length-unmeasured days
//! are fixed tables whose lengths are compile-time constants, so the walks over
//! them are bounded and not scans that grow.
//!
//! **UNVERIFIED as a measurement.** The bound is argued from the
//! shape of the code and no bench in this workspace times it.
//! `CLAUDE.md` §3 rule 6: a structural argument is not a
//! measurement, however sound it is.

/// First day this calendar knows: **2019-12-02**, as days since the epoch.
pub const FIRST_DAY: i64 = 18_232;

/// Last day this calendar knows: **2026-09-04**, as days since the epoch.
pub const LAST_DAY: i64 = 20_700;

/// How many days the bitset covers.
pub const DAYS: usize = 2_469;

// TWO ASSERTIONS AND NO CAST. One `DAYS as i64` would say the same thing and
// would be the workspace's only sign-losing cast on a path that never needs
// one; both sides are compared against the literal instead.
const _: () = assert!(LAST_DAY - FIRST_DAY + 1 == 2_469);
const _: () = assert!(DAYS == 2_469);

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

/// 2021-02-24, the NSE systems-outage day, as days since the epoch.
///
/// Public because the exchange calendar and the spot-index completeness policy
/// must name the same fixed historical event without duplicating its number.
pub const SYSTEMS_OUTAGE_DAY: i64 = 18_682;

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
    /// The exchange traded — a `1day` bar proves it — but **no minute bar
    /// exists for that day anywhere in the store**, so the session's length was
    /// never measured and this build will not guess it.
    ///
    /// # The five days this is, and why they are not a bug in the calendar
    ///
    /// Every Diwali **Muhurat** session before 2025: 2020-11-14, 2021-11-04,
    /// 2022-10-24, 2023-11-12 and 2024-11-01. Each has a daily bar and zero
    /// minute bars, which is Zerodha's minute history not reaching them rather
    /// than a hole in a day it served. (Corrected by D-3352: Kite served sixty
    /// minutes on each; the store refused them. See the module header.)
    ///
    /// **It is a third answer on purpose.** Calling them [`Self::Open`] with a
    /// full session would claim 375 owed bars and report 1,875 losses that were
    /// never offered; calling them [`Self::Closed`] would deny a session the
    /// daily bar proves happened. The 2025 Muhurat measured **60** minutes and
    /// these were probably the same, but "probably" is the invention
    /// `CLAUDE.md` §3 rule 1 bans — so the length is withheld and the day is
    /// flagged for a caller to decide about.
    ///
    /// **Since D-3319 the windows are recorded** ([`SPECIAL_SESSIONS`]) and
    /// [`exchange_kind_of`] answers them; [`kind_of`] and [`sourced_kind_of`]
    /// keep this answer, because the first is hashed into run identity and
    /// the second is the GDFL gate's and the ingest calendar's.
    OpenLengthUnmeasured,
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
/// Three were read off the stored bars. The 2021-02-24 windows instead come
/// from SEBI Settlement Order SO/AB/EFD2/2023-24/6580: halt from 11:40,
/// 15-minute pre-open from 15:30, normal trading 15:45–17:00. Its stored 54-bar
/// index prefix is not the exchange timetable and must not define it. This does
/// not claim an index value existed at every normal-market minute.
/// The names in the other comments are identifications from shape and date and
/// are not vendor-confirmed labels.
///
/// Each window is a named constant shared with [`SPECIAL_SESSIONS`], so the
/// run-identity table and the calendar of record hold ONE copy of every
/// window (D-3319). The values are unchanged: this table, and so `kind_of` and
/// `cli::stored::calendar_policy_digest_v2`, answer exactly what they did.
const IRREGULAR: [(i64, Session); 4] = [
    // 2021-02-24 — normal trading 09:15–11:40 and 15:45–17:00. Bar timestamps
    // are left edges, so the inclusive minute opens are 09:15–11:39 and
    // 15:45–16:59: 145 + 75 = 220.
    (SYSTEMS_OUTAGE_DAY, OUTAGE_SESSION),
    // 2024-03-02 (Sat) — two windows. A disaster-recovery live test.
    (19_784, DR_DRILL_SESSION),
    // 2024-05-18 (Sat) — the same two windows, the same kind of test.
    (19_861, DR_DRILL_SESSION),
    // 2025-10-21 — one hour, in the afternoon, on a weekday the market was
    // otherwise shut. Muhurat.
    (20_382, MUHURAT_1345),
];

/// A session of one window, `from..=to` minute opens.
const fn one_window(from: u16, to: u16) -> Session {
    Session {
        windows: [Window { from, to }, Window { from: 0, to: 0 }],
        count: 1,
    }
}

/// A session of two windows.
const fn two_windows(first: Window, second: Window) -> Session {
    Session {
        windows: [first, second],
        count: 2,
    }
}

/// 2021-02-24: 09:15–11:39 and 15:45–16:59, SEBI's record (see [`IRREGULAR`]).
const OUTAGE_SESSION: Session = two_windows(
    Window { from: 555, to: 699 },
    Window {
        from: 945,
        to: 1_019,
    },
);

/// The disaster-recovery Saturdays: 09:15–09:59 and 11:30–12:29.
const DR_DRILL_SESSION: Session =
    two_windows(Window { from: 555, to: 599 }, Window { from: 690, to: 749 });

/// Muhurat 2025-10-21: 13:45–14:44.
const MUHURAT_1345: Session = one_window(825, 884);

/// Muhurat 2018-11-07: 17:30–18:29.
const MUHURAT_1730: Session = one_window(1_050, 1_109);

/// Muhurat 2024-11-01: 18:00–18:59.
const MUHURAT_1800: Session = one_window(1_080, 1_139);

/// Muhurat 2019-10-27 and 2020-11-14 to 2023-11-12: 18:15–19:14.
const MUHURAT_1815: Session = one_window(1_095, 1_154);

/// Days with a `1day` bar and **no minute bar anywhere in the store**.
///
/// Measured, not chosen: the store held 1,671 days of daily bars and 1,666
/// days of minute bars, and this is the difference (re-measured 2026-10-11:
/// 1,631 minute days, these five and 35 hole days short; see the module
/// note). Every one is a Diwali
/// Muhurat session, and the only Muhurat that IS in the minute series is
/// 2025-10-21 — which is in [`IRREGULAR`] with its measured hour instead.
///
/// See [`DayKind::OpenLengthUnmeasured`] for why these are a third answer
/// rather than folded into either neighbour.
const LENGTH_UNMEASURED: [i64; 5] = [
    18_580, // 2020-11-14 Sat
    18_935, // 2021-11-04 Thu
    19_289, // 2022-10-24 Mon
    19_673, // 2023-11-12 Sun
    20_028, // 2024-11-01 Fri
];

/// Whether each day in the window traded. One bit per day, LSB first.
///
/// Original 307 bytes: daily NIFTY/BANKNIFTY/INDIAVIX agreement through Aug 21.
/// Appended two bytes: the sourced, corroborated extension described above.
static TRADED: [u8; 309] = [
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
    0xF3, 0xF9, 0x7C, 0x3E, 0x1F,
];

const _: () = assert!(TRADED.len() == DAYS.div_ceil(8));

// THE TWO WALK LENGTHS `kind_of`'s cost note states. o1api-39, D-1203.
const _: () = assert!(LENGTH_UNMEASURED.len() == 5 && IRREGULAR.len() == 4);

/// What `epoch_day` was: an open session with its windows, a closed day, or
/// outside what has been measured.
///
/// # Cost
///
/// One compare, one subtract, one index, one shift — and, only for a day that
/// traded, TWO bounded walks: the five-element [`LENGTH_UNMEASURED`] table and
/// then the four-element [`IRREGULAR`] table, at most nine comparisons in all.
/// Both lengths are compile-time constants, pinned by the assertion above so a
/// table that grows cannot leave this sentence behind. O(1), no hash, no
/// allocation.
///
/// This note named only the [`IRREGULAR`] walk until o1api-39 (D-1203); the
/// [`LENGTH_UNMEASURED`] walk ran first and was not mentioned.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[must_use]
pub fn kind_of(epoch_day: i64) -> DayKind {
    if !(FIRST_DAY..=LAST_DAY).contains(&epoch_day) {
        return DayKind::Unmeasured;
    }
    // `try_from` RATHER THAN `as`, and the failure arm is honest rather than a
    // panic. The range check above already makes the difference 0..=2468, so
    // this cannot fail; if a future edit breaks that, saying "I have not
    // measured this day" is the safe answer and `expect` is banned outright.
    let Ok(offset) = usize::try_from(epoch_day - FIRST_DAY) else {
        return DayKind::Unmeasured;
    };
    let byte = TRADED.get(offset / 8).copied().unwrap_or(0);
    if byte & (1_u8 << (offset % 8)) == 0 {
        return DayKind::Closed;
    }
    // THE MUHURAT DAYS WITH NO MINUTE SERIES, CHECKED BEFORE THE IRREGULAR
    // TABLE. Five entries, a compile-time constant, so this is a bounded walk
    // and not a scan that grows.
    let mut m = 0_usize;
    while m < LENGTH_UNMEASURED.len() {
        match LENGTH_UNMEASURED.get(m) {
            Some(&day) if day == epoch_day => return DayKind::OpenLengthUnmeasured,
            _ => m += 1,
        }
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
        // BOTH `None`, AND FOR THE SAME REASON. One is a day outside the range
        // and the other a Muhurat whose length was never measured; in both the
        // honest answer to "how many bars are owed" is that nobody knows, and
        // `Some(0)` would say the exchange owed nothing.
        DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => None,
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
        match kind_of(day) {
            // A Muhurat of unmeasured LENGTH is still a day the exchange
            // traded, so it is a session here (attackdata-6, D-1532). Only its
            // bar count is unknown, which is `expected_bars`'s question.
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured => count = count.saturating_add(1),
            DayKind::Closed => {}
            DayKind::Unmeasured => return None,
        }
        day += 1;
    }
    Some(count)
}

// ══ The circular-sourced spans either side of the measured window (D-2805) ══
//
// NOT folded into `kind_of`. That function is the calendar every stored sweep
// is identified by: `cli::stored::calendar_policy_digest_v2` hashes
// `FIRST_DAY`, `LAST_DAY` and `kind_of` of every day in between into run
// identity, so widening it would re-key every recorded run, which is the
// operator's decision and not an importer's. The GDFL readers ask
// `sourced_kind_of` instead, and so does the ingest calendar `Runtime`
// (D-3346), which no run identity hashes; every other reader is unchanged.

/// First day [`sourced_kind_of`] answers: **2018-01-01**.
pub const SOURCED_FIRST_DAY: i64 = 17_532;

/// Last day [`sourced_kind_of`] answers: **2026-12-31**, the last day
/// NSE/CMTR/71775 (calendar year 2026) covers.
///
/// It was 2026-10-04 (`20_730`), the day the D-2805 spans were read, and from
/// 2026-10-05 every minute pull of a newer day failed for every instrument
/// (DB-B25, DB-B41): the record ended while the circular it cites ran on.
/// [`record_horizon`] is the health check that says so BEFORE the next end.
pub const SOURCED_LAST_DAY: i64 = 20_818;

/// How many days before [`SOURCED_LAST_DAY`] the record is said to end soon:
/// **30** (DB-B41's expected "warns when the record ends within 30 days").
pub const RECORD_ENDS_SOON_DAYS: i64 = 30;

/// Where `today` stands against the end of the calendar of record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RecordHorizon {
    /// The last day the record answers, [`SOURCED_LAST_DAY`].
    pub last_day: i64,
    /// Days from `today` to `last_day`: negative once the record has ended.
    pub days_left: i64,
    /// `days_left` is at most [`RECORD_ENDS_SOON_DAYS`] (ended included).
    pub ends_soon: bool,
    /// `today` is past the record: every day from here is `Unmeasured`.
    pub ended: bool,
}

/// The record's horizon seen from `today` (days since the epoch): one
/// subtraction and two comparisons, no allocation. The status document of a
/// pull run carries it, so a page can warn before the record ends rather than
/// after every pull has failed (DB-B25, DB-B41).
#[must_use]
pub const fn record_horizon(today: i64) -> RecordHorizon {
    let days_left = SOURCED_LAST_DAY.saturating_sub(today);
    RecordHorizon {
        last_day: SOURCED_LAST_DAY,
        days_left,
        ends_soon: days_left <= RECORD_ENDS_SOON_DAYS,
        ended: days_left < 0,
    }
}

/// The days of `first..=last` that lie past the record, as `(first, last,
/// count)`, or `None` when the whole window is inside it. A run reports them
/// in ONE line rather than once per instrument and month (DB-B25).
#[must_use]
pub const fn past_record(first: i64, last: i64) -> Option<(i64, i64, i64)> {
    if last <= SOURCED_LAST_DAY || first > last {
        return None;
    }
    let from = if first > SOURCED_LAST_DAY {
        first
    } else {
        SOURCED_LAST_DAY + 1
    };
    Some((from, last, last - from + 1))
}

/// The leading words of a member's finding about days past the record. One
/// constant, so the run that folds those findings into its single line
/// recognises them by the words the writer used and not by a guess at prose.
pub const PAST_RECORD_FINDING: &str =
    "request minute coverage NOT JUDGED past the calendar of record";

/// Every weekday trading holiday NSE notified for the capital-market segment
/// outside the measured span, as days since the epoch (D-2805). Sources,
/// each fetched and read on 2026-10-04 (recorded in `docs/00-charter.md`):
/// NSE/CMTR/36475 (12 Dec 2017, calendar year 2018), NSE/CMTR/39612 (7 Dec
/// 2018, calendar year 2019), NSE/CMTR/40536 (26 Mar 2019, 29 Apr 2019,
/// Parliamentary Elections in Mumbai), NSE/MSD/42319 (7 Oct 2019, 21 Oct
/// 2019, Assembly Elections in Maharashtra) and NSE/CMTR/71775 (12 Dec 2025,
/// calendar year 2026). Holidays those circulars list on a Saturday or a
/// Sunday are closed by the weekday rule and are not repeated here.
///
/// The four 2026 holidays after 2026-10-04 (20 Oct, 10 Nov, 24 Nov, 25 Dec)
/// are NSE/CMTR/71775's too: the circular was fetched and read again on
/// 2026-10-10 by the audit that found the record expired (DB-B41, two
/// readers; `state/blindspots/time-calendar-edges-r1.md`). No vendor day zip
/// corroborates them yet: they are days that have not happened.
///
/// Cross-checked against the vendor's GDFL capital-market day zips over
/// 2018-09-03..=2019-12-01 and 2026-09-05..=2026-09-30: every weekday that
/// is not in this table has an index and a stock day zip (0 missing), and
/// the only day zips on a day this table or the weekend closes are the two
/// Muhurat sessions of [`SOURCED_SPECIAL`].
pub const SOURCED_HOLIDAYS: [i64; 38] = [
    17_557, // 2018-01-26 Fri Republic Day
    17_575, // 2018-02-13 Tue Mahashivratri
    17_592, // 2018-03-02 Fri Holi
    17_619, // 2018-03-29 Thu Mahavir Jayanti
    17_620, // 2018-03-30 Fri Good Friday
    17_652, // 2018-05-01 Tue Maharashtra Day
    17_758, // 2018-08-15 Wed Independence Day
    17_765, // 2018-08-22 Wed Bakri ID
    17_787, // 2018-09-13 Thu Ganesh Chaturthi
    17_794, // 2018-09-20 Thu Moharram
    17_806, // 2018-10-02 Tue Mahatma Gandhi Jayanti
    17_822, // 2018-10-18 Thu Dasera
    17_842, // 2018-11-07 Wed Diwali-Laxmi Pujan (Muhurat: SOURCED_SPECIAL)
    17_843, // 2018-11-08 Thu Diwali-Balipratipada
    17_858, // 2018-11-23 Fri Gurunanak Jayanti
    17_890, // 2018-12-25 Tue Christmas
    17_959, // 2019-03-04 Mon Mahashivratri
    17_976, // 2019-03-21 Thu Holi
    18_003, // 2019-04-17 Wed Mahavir Jayanti
    18_005, // 2019-04-19 Fri Good Friday
    18_015, // 2019-04-29 Mon Parliamentary Elections (NSE/CMTR/40536)
    18_017, // 2019-05-01 Wed Maharashtra Day
    18_052, // 2019-06-05 Wed Id-Ul-Fitr
    18_120, // 2019-08-12 Mon Bakri Id
    18_123, // 2019-08-15 Thu Independence Day
    18_141, // 2019-09-02 Mon Ganesh Chaturthi
    18_149, // 2019-09-10 Tue Moharram
    18_171, // 2019-10-02 Wed Mahatma Gandhi Jayanti
    18_177, // 2019-10-08 Tue Dasera
    18_190, // 2019-10-21 Mon Assembly Elections (NSE/MSD/42319)
    18_197, // 2019-10-28 Mon Diwali-Balipratipada
    18_212, // 2019-11-12 Tue Gurunanak Jayanti
    20_710, // 2026-09-14 Mon Ganesh Chaturthi
    20_728, // 2026-10-02 Fri Mahatma Gandhi Jayanti
    20_746, // 2026-10-20 Tue Dussehra
    20_767, // 2026-11-10 Tue Diwali-Balipratipada
    20_781, // 2026-11-24 Tue Guru Nanak Jayanti
    20_812, // 2026-12-25 Fri Christmas
];

/// The Muhurat sessions in the sourced spans: each circular names the day
/// and says the timings "shall be notified subsequently", so the LENGTH is
/// not sourced here and the day is [`DayKind::OpenLengthUnmeasured`], never a
/// regular day: 2018-11-07 (a Wednesday holiday) and 2019-10-27 (a Sunday),
/// and 2026-11-08 (a Sunday): NSE/CMTR/71775 says Muhurat trading "will be
/// conducted on Sunday, November 08, 2026" with timings to be notified, so
/// the day is a session of no recorded length until a circular states one
/// (DB-B41). It has no [`SPECIAL_SESSIONS`] row: no window is recorded.
pub const SOURCED_SPECIAL: [i64; 3] = [17_842, 18_196, 20_765];

/// Whether `day` is one of a compile-time table's entries: a walk bounded by
/// the table's own constant length, never by the data (the shape `kind_of`
/// already uses for its two tables).
fn listed(table: &[i64], day: i64) -> bool {
    for held in table {
        if *held == day {
            return true;
        }
    }
    false
}

/// A day of the sourced spans: a Muhurat is open with no length, a weekend
/// or a listed holiday is closed, every other weekday is a regular session.
/// Two bounded walks over compile-time tables (3 and 38 entries) and one
/// remainder, no allocation.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
fn sourced_kind(epoch_day: i64) -> DayKind {
    if listed(&SOURCED_SPECIAL, epoch_day) {
        return DayKind::OpenLengthUnmeasured;
    }
    // 1970-01-01, day 0, was a Thursday: Monday..Friday are 0..=4 here.
    let weekday = (epoch_day + 3).rem_euclid(7) < 5;
    if weekday && !listed(&SOURCED_HOLIDAYS, epoch_day) {
        DayKind::Open(Session::full())
    } else {
        DayKind::Closed
    }
}

/// [`kind_of`] inside the measured window, the circular-sourced spans of
/// D-2805 from [`SOURCED_FIRST_DAY`] to [`SOURCED_LAST_DAY`] outside it, and
/// `Unmeasured` beyond them. The GDFL readers' calendar, and the ingest
/// calendar [`Runtime`]'s (D-3346).
///
/// **Corroborated by the Zerodha store, 6 Oct 2026 (D-3346).** Over the 726
/// days of the two spans (2018-01-01..=2019-12-01 and 2026-09-05..=2026-09-30)
/// the `1day` files of NIFTY, BANKNIFTY and NIFTYIT hold a bar on exactly the
/// 487 days this answers open (485 regular sessions and the two Muhurats) and
/// on no other; INDIAVIX agrees except that it has no bar on 2019-07-02, a
/// session the other three traded. NIFTY's and BANKNIFTY's minute files hold the
/// full 09:15–15:29 grid on 481 of the 485 regular sessions.
#[must_use]
pub fn sourced_kind_of(epoch_day: i64) -> DayKind {
    let (FIRST_DAY..=LAST_DAY) = epoch_day else {
        let (SOURCED_FIRST_DAY..=SOURCED_LAST_DAY) = epoch_day else {
            return DayKind::Unmeasured;
        };
        return sourced_kind(epoch_day);
    };
    kind_of(epoch_day)
}

// ══ The calendar of record: special sessions, halts, phases, expiry days ══
//                                                  (D-3319, D-3320)
//
// THREE CALENDARS, AND WHY THE FIRST TWO KEEP THEIR ANSWERS.
//
// | function                | answers                                         | read by                                   |
// |-------------------------|-------------------------------------------------|-------------------------------------------|
// | `kind_of`               | the measured window, pre-2025 Muhurats unsized  | run identity, the landing verdict, sweeps |
// | `sourced_kind_of`       | + the D-2805 circular spans 2018-01-01..10-04   | GDFL gates, the ingest `Runtime` (D-3346) |
// | `exchange_kind_of`      | + every special session's recorded window       | phases, halts, expiry days, day records   |
//
// `kind_of` is hashed into every stored run by
// `cli::stored::calendar_policy_digest_v2`: giving a Muhurat its window, or
// widening `LAST_DAY` to 2026-09-30, re-keys every recorded run, which D-2805
// and D-3346 record as the operator's decision and which is still open.
// `sourced_kind_of` is the D-2805 policy the GDFL day gate and the ingest
// calendar read, and its tests pin the Muhurats as unsized; moving it is the
// adopting package's change (WP-12a's admission, WP-13b's fold), made in the
// same commit as the test that proves the new behaviour. So the windows land
// HERE, additively, as one more answer than `sourced_kind_of` gives and never
// a different answer on any other day — a test walks every day of both spans.
// Each window is ONE constant, which `IRREGULAR` names too.

/// Minute-of-day the opening call auction starts on a regular day: **09:00**.
///
/// `docs/00-charter.md` §3, "Pre-open auction 09:00 – 09:15 IST — excluded
/// from bars". For cash and the index computed from it; the charter gives no
/// derivatives pre-open, so none is claimed.
pub const PRE_OPEN_MINUTE: u16 = 9 * 60;

/// Minute-of-day continuous trading ends for a closing-auction-eligible share
/// from 2026-08-03: **15:15**. `docs/00-charter.md` §7 and §9 (NSE/CMTR/73845,
/// NSE/CMTR/74466): "the scheduled continuous close (15:15 eligible, 15:30
/// ineligible)".
pub const CAS_CONTINUOUS_CLOSE: u16 = 15 * 60 + 15;

/// The end of the closing-auction band on the minute grid: **15:30**, the
/// scheduled close of an ineligible share (charter §9). The auction's own
/// internal phases and their timings are not charter rows, so the band is the
/// whole 15:15–15:29 stretch and nothing finer is claimed.
pub const CAS_SESSION_END: u16 = 15 * 60 + 30;

/// Minutes in a day.
pub const MINUTES_PER_DAY: u16 = 24 * 60;

const _: () = assert!(PRE_OPEN_MINUTE < OPEN_MINUTE);
const _: () = assert!(CAS_CONTINUOUS_CLOSE < CAS_SESSION_END && CAS_SESSION_END == LAST_MINUTE + 1);

/// Why a day held a session that is not an ordinary weekday 09:15–15:30.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SpecialKind {
    /// Diwali Muhurat: one hour on a day that is otherwise a holiday or a
    /// weekend.
    Muhurat,
    /// A live trading drill from NSE's disaster-recovery site, on a Saturday.
    DisasterRecoveryDrill,
    /// A regular trading day the exchange's own systems interrupted.
    SystemsOutage,
    /// A Union Budget day on a weekend, traded at full regular hours.
    BudgetWeekend,
    /// A full regular session on a weekend for a reason no source here
    /// records (2024-01-20).
    WeekendFullSession,
}

// NO WIRE CODES HERE. A route that serves these enums names them in its own
// crate: a one-word lower-case literal under crates/pull is segment-shaped,
// and gate 1d would rightly ask for every one to be declared (CLAUDE.md §8).
impl SpecialKind {
    /// A reader's label.
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Muhurat => "Muhurat session",
            Self::DisasterRecoveryDrill => "disaster-recovery drill",
            Self::SystemsOutage => "systems outage",
            Self::BudgetWeekend => "Budget weekend session",
            Self::WeekendFullSession => "weekend full session",
        }
    }

    /// Whether the exchange meant the day to be a regular market day, and so
    /// whether a contract can settle on it.
    ///
    /// The charter's distinction (§3, the 2021-02-24 row): "a Muhurat and a DR
    /// drill are days the exchange never meant to be normal", an outage day is
    /// one it did, and a weekend Budget or full session runs regular hours.
    #[must_use]
    pub const fn is_market_day(self) -> bool {
        matches!(
            self,
            Self::SystemsOutage | Self::BudgetWeekend | Self::WeekendFullSession
        )
    }
}

/// One special session: the day, what it was, the windows it traded and the
/// record those windows come from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpecialSession {
    /// Days since the epoch.
    pub day: i64,
    /// What the session was.
    pub kind: SpecialKind,
    /// The windows it traded, as inclusive minute opens.
    pub session: Session,
    /// Where the windows come from, in one sentence.
    pub evidence: &'static str,
}

/// The Muhurat windows' tick measurement, which the charter's §3 Muhurat table
/// records ("The tick measurement, 9 Oct 2026") and seven entries of
/// [`SPECIAL_SESSIONS`] cite.
///
/// **MEASURED 2026-10-09** from the vendor's GDFL capital-market day zips
/// (read only from the vendor archive, counts and minute spans only; no
/// vendor row is reproduced): on every Muhurat from 2018-11-07 to 2025-10-21
/// the minutes in which each of RELIANCE, INFY, HDFCBANK, SBIN, TCS and
/// ICICIBANK trades at least three times run unbroken from the window's open
/// to its last minute, sixty minutes, and NIFTY 50's value changes in every
/// one of them. Before the window a share trades at most once; after it no
/// minute holds three trades until the post-close session, ten or more
/// minutes later. The same walk over 2024-03-02, 2024-05-18 and 2024-01-20
/// gives the charter's Zerodha windows.
pub const MUHURAT_TICKS: &str = "MEASURED 2026-10-09 from the GDFL capital-market day files \
     (charter §3): the minutes in which each of RELIANCE, INFY, HDFCBANK, SBIN, TCS and ICICIBANK \
     trades at least three times run unbroken from the window's open to its last minute, and \
     NIFTY 50 changes in every one of them";

/// Every special session with a recorded window from 2018-01-01 to
/// 2026-12-31, sorted by day.
///
/// Every window is a row of the charter's §3: the Muhurat table (whose
/// 2018-11-07, 2019-10-27 and 2023-11-12 rows are [`MUHURAT_TICKS`]; the
/// 2023 row supersedes an 18:00 open the charter had marked UNVERIFIED), the
/// disaster-recovery and Budget rows, 2024-01-20, and SEBI's 2021-02-24
/// record. The 2018 and 2019 dates are the D-2805 circulars'. The 2026
/// Muhurat (2026-11-08) has no notified timings and lies past
/// [`SOURCED_LAST_DAY`].
pub const SPECIAL_SESSIONS: [SpecialSession; 15] = [
    SpecialSession {
        day: 17_842, // 2018-11-07 Wed
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1730,
        evidence: "Charter §3 Muhurat table: 17:30-18:30, 60 bars, MEASURED from GDFL ticks \
                   (MUHURAT_TICKS); the date is NSE/CMTR/36475's (Diwali-Laxmi Pujan, \
                   \"Muhurat Trading will be conducted\")",
    },
    SpecialSession {
        day: 18_196, // 2019-10-27 Sun
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1815,
        evidence: "Charter §3 Muhurat table: 18:15-19:15, 60 bars, MEASURED from GDFL ticks \
                   (MUHURAT_TICKS); the date is NSE/CMTR/39612's (Muhurat on Sunday 27 Oct)",
    },
    SpecialSession {
        day: 18_293, // 2020-02-01 Sat
        kind: SpecialKind::BudgetWeekend,
        session: Session::full(),
        evidence: "Charter §3 'Special weekend sessions': Union Budget, full regular hours, 375 \
                   bars MEASURED from Zerodha NIFTY 1min",
    },
    SpecialSession {
        day: 18_580, // 2020-11-14 Sat
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1815,
        evidence: "Charter §3 Muhurat table: 18:15-19:15, 60 bars, confirmed against the lake; \
                   GDFL ticks agree (MUHURAT_TICKS)",
    },
    SpecialSession {
        day: SYSTEMS_OUTAGE_DAY, // 2021-02-24 Wed
        kind: SpecialKind::SystemsOutage,
        session: OUTAGE_SESSION,
        evidence: "SEBI Settlement Order SO/AB/EFD2/2023-24/6580 (charter §3): normal trading \
                   09:15-11:40 and 15:45-17:00; the halt and the 15:30 pre-open are in HALTS",
    },
    SpecialSession {
        day: 18_935, // 2021-11-04 Thu
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1815,
        evidence: "Charter §3 Muhurat table: 18:15-19:15, 60 bars, confirmed against the lake; \
                   GDFL ticks agree (MUHURAT_TICKS)",
    },
    SpecialSession {
        day: 19_289, // 2022-10-24 Mon
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1815,
        evidence: "Charter §3 Muhurat table: 18:15-19:15, 60 bars, confirmed against the lake; \
                   GDFL ticks agree (MUHURAT_TICKS)",
    },
    SpecialSession {
        day: 19_673, // 2023-11-12 Sun
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1815,
        evidence: "Charter §3 Muhurat table: 18:15-19:15, 60 bars, MEASURED from GDFL ticks \
                   (MUHURAT_TICKS); supersedes 18:00-19:00 and 46 bars, whose open the \
                   charter had marked UNVERIFIED",
    },
    SpecialSession {
        day: 19_742, // 2024-01-20 Sat
        kind: SpecialKind::WeekendFullSession,
        session: Session::full(),
        evidence: "Charter §3 '2024-01-20 (Sat)': a full regular session, 375 bars MEASURED from \
                   Zerodha NIFTY 1min; why the exchange traded is UNVERIFIED",
    },
    SpecialSession {
        day: 19_784, // 2024-03-02 Sat
        kind: SpecialKind::DisasterRecoveryDrill,
        session: DR_DRILL_SESSION,
        evidence: "Charter §3 'Disaster-recovery sessions': 09:15-09:59 and 11:30-12:29, 105 \
                   bars MEASURED from Zerodha NIFTY 1min; GDFL ticks agree",
    },
    SpecialSession {
        day: 19_861, // 2024-05-18 Sat
        kind: SpecialKind::DisasterRecoveryDrill,
        session: DR_DRILL_SESSION,
        evidence: "Charter §3 'Disaster-recovery sessions': 09:15-09:59 and 11:30-12:29, 105 \
                   bars MEASURED from Zerodha NIFTY 1min; GDFL ticks agree",
    },
    SpecialSession {
        day: 20_028, // 2024-11-01 Fri
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1800,
        evidence: "Charter §3 Muhurat table: 18:00-19:00, 60 bars; GDFL ticks agree \
                   (MUHURAT_TICKS)",
    },
    SpecialSession {
        day: 20_120, // 2025-02-01 Sat
        kind: SpecialKind::BudgetWeekend,
        session: Session::full(),
        evidence: "Charter §3 'Special weekend sessions': Union Budget, full regular hours, 375 \
                   bars MEASURED from Zerodha NIFTY 1min",
    },
    SpecialSession {
        day: 20_382, // 2025-10-21 Tue
        kind: SpecialKind::Muhurat,
        session: MUHURAT_1345,
        evidence: "Charter §3 Muhurat table: 13:45-14:45, 60 bars, an afternoon session; the \
                   Zerodha minute series and GDFL ticks agree",
    },
    SpecialSession {
        day: 20_485, // 2026-02-01 Sun
        kind: SpecialKind::BudgetWeekend,
        session: Session::full(),
        evidence: "Charter §3 'Special weekend sessions': Union Budget, full regular hours, 375 \
                   bars MEASURED from Zerodha NIFTY 1min",
    },
];

/// The special session on `epoch_day`, if one was recorded.
///
/// # Cost
///
/// One walk of the fifteen-entry [`SPECIAL_SESSIONS`], a compile-time
/// constant: at most fifteen comparisons, no allocation.
#[must_use]
pub fn special_session(epoch_day: i64) -> Option<SpecialSession> {
    // A `for` over the constant table, the shape `listed` uses: gate 11's
    // rule 6 refuses `.iter().find(` because it cannot tell a table from data.
    for special in &SPECIAL_SESSIONS {
        if special.day == epoch_day {
            return Some(*special);
        }
    }
    None
}

/// The calendar of record: [`sourced_kind_of`], except that a special
/// session whose window [`SPECIAL_SESSIONS`] records is
/// [`DayKind::Open`] with that window instead of
/// [`DayKind::OpenLengthUnmeasured`] (D-3319).
///
/// It differs from [`sourced_kind_of`] on exactly the seven Muhurats
/// 2018-11-07 to 2024-11-01, and from [`kind_of`] there and on every day of
/// the sourced spans; `exchange_kind_of_differs_only_where_a_window_was_recorded`
/// walks every day to prove it. Read by [`phase_at`], [`phases_of`],
/// [`expiry_day`] and [`record_of`], and by the dated-master schedule in
/// [`crate::cash_session_cache`] — and by nothing that is hashed.
///
/// **The one an adopter of D-3319 reads.** A day gate that admits a special
/// session inside its windows (WP-12a), a fold or landing that keeps one
/// (WP-13b), and a route that lists the day (WP-35) ask this, never
/// [`sourced_kind_of`], which still answers the Muhurats unsized. An expiry
/// walk asks [`expiry_day`] or [`settlement_on_or_before`]: a Muhurat or a
/// drill is open here and is still no settlement day.
///
/// # Cost
///
/// [`sourced_kind_of`] (at most nine comparisons in the measured window, or
/// the 3- and 38-entry walks in a sourced span) and, only for an unsized
/// session, one walk of [`SPECIAL_SESSIONS`]. O(1), no allocation.
/// Timed by `pull::calendar::record_tests::calendar_of_record_costs`
/// (release, 2026-10-09, three runs: p50 2 ns, p99 13 ns). That test is ignored, so a
/// regression fails no gate: UNVERIFIED as a gated measurement.
#[must_use]
pub fn exchange_kind_of(epoch_day: i64) -> DayKind {
    match sourced_kind_of(epoch_day) {
        DayKind::OpenLengthUnmeasured => special_session(epoch_day)
            .map_or(DayKind::OpenLengthUnmeasured, |special| {
                DayKind::Open(special.session)
            }),
        other => other,
    }
}

/// Where a halt's window comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HaltEvidence {
    /// A primary record states the window.
    Sourced(&'static str),
    /// Measured in the stored data and consistent with NSE's published rule,
    /// but **no NSE circular is recorded**: UNVERIFIED, shown as suspected
    /// until one is (operator answer 8, D-3320).
    Suspected(&'static str),
}

impl HaltEvidence {
    /// The record, in one sentence.
    #[must_use]
    pub const fn text(self) -> &'static str {
        match self {
            Self::Sourced(text) | Self::Suspected(text) => text,
        }
    }

    /// Whether the window is unverified.
    #[must_use]
    pub const fn is_suspected(self) -> bool {
        matches!(self, Self::Suspected(_))
    }
}

/// The seconds one segment's one-second series spends in a halt.
///
/// **Per segment, because the segments did not stop on the same second.**
/// On 2020-03-13 the shares print last at 09:21:08, the index changes last at
/// 09:21:09 and the NIFTY and BANKNIFTY options trade until 09:21:44 (charter,
/// circuit-breaker boundary). One second for all three would shade 35 seconds
/// of real index-option trades as halted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HaltSeconds {
    /// Second of the IST day the halt began for this segment: the second
    /// after its last measured print (a share's or an option's trade, an
    /// index's value change), or the sourced start. Its minute is still owed
    /// when the halt began inside it (09:21 on 2020-03-13).
    pub from: u32,
    /// Second of the IST day continuous trading resumed, exclusive end: the
    /// segment's first measured print after the halt, or the sourced resume.
    pub resume: u32,
}

/// A market-wide halt: the minutes with no continuous trading, and the call
/// auction that reopened the market inside them.
///
/// **A halted minute is neither owed nor a loss.** The charter's
/// circuit-breaker boundary: do not "count every such absence as a minute
/// the provider must supply". A coverage reader shows it as halted —
/// suspected where [`HaltEvidence::Suspected`] — and owes only the
/// [`Phase::Continuous`] minutes around it.
///
/// **A halted band labels, it never removes.** A straggling print inside it
/// stays a bar: on 2020-03-13 five trades of shares outside the F&O list and
/// eight of stock options print in the 09:22 minute (charter).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Halt {
    /// Days since the epoch.
    pub day: i64,
    /// Inclusive minute opens with no continuous trading on any segment:
    /// the whole minutes inside every segment's [`HaltSeconds`]
    /// (`each_halt_names_its_seconds_and_whole_minutes`).
    pub halted: Window,
    /// The reopening call auction, inside `halted`, where it is known: the
    /// cash market's and the index's, never a derivative's.
    pub reopening: Option<Window>,
    /// The shares' seconds.
    pub cash: HaltSeconds,
    /// The index values' seconds.
    pub index: HaltSeconds,
    /// The index options' seconds.
    pub derivatives: HaltSeconds,
    /// Where the window comes from.
    pub evidence: HaltEvidence,
}

impl Halt {
    /// The seconds `venue`'s one-second series spends in this halt.
    #[must_use]
    pub const fn seconds(self, venue: crate::vendor::Venue) -> HaltSeconds {
        match venue {
            crate::vendor::Venue::NseCash => self.cash,
            crate::vendor::Venue::NseIndex => self.index,
            crate::vendor::Venue::NseDerivatives => self.derivatives,
        }
    }
}

/// The seconds `from..resume` of a halt, from `(h, m, s)` stamps.
const fn halt_seconds(from: (u32, u32, u32), resume: (u32, u32, u32)) -> HaltSeconds {
    HaltSeconds {
        from: from.0 * 3_600 + from.1 * 60 + from.2,
        resume: resume.0 * 3_600 + resume.1 * 60 + resume.2,
    }
}

/// Every market-wide halt recorded from 2018-01-01 to 2026-12-31.
///
/// The two March 2020 circuit breakers are **suspected** (D-3320): their
/// windows are the charter's "Historical circuit-breaker evidence boundary" —
/// the /db audit's minute holes in both feeds (DB-028, DB-G47) and the GDFL
/// day files re-measured there on 2026-10-09 — and their split into a halt and
/// a 15-minute reopening auction is the NSE rule's shape read from those
/// times, but the charter holds no NSE circular for either. The 2021-02-24
/// outage is sourced from SEBI, on every segment. 2022-03-07's scattered
/// minute holes are not a halt and are not here (DB-G47 asks for them to be
/// investigated, not classified): they are in [`SHORT_DAYS`], with
/// 2019-08-02, as measured and unexplained.
pub const HALTS: [Halt; 3] = [
    Halt {
        day: 18_334, // 2020-03-13 Fri
        halted: Window { from: 562, to: 620 },
        reopening: Some(Window { from: 606, to: 620 }),
        cash: halt_seconds((9, 21, 9), (10, 21, 0)),
        index: halt_seconds((9, 21, 10), (10, 21, 1)),
        derivatives: halt_seconds((9, 21, 45), (10, 21, 0)),
        evidence: HaltEvidence::Suspected(
            "MEASURED from the GDFL day files, 2026-10-09 (charter, circuit-breaker boundary): \
             RELIANCE, HDFCBANK, INFY, SBIN, TCS and ICICIBANK trade last at 09:21:08 and next \
             at 10:21:00; NIFTY 50 and NIFTY BANK change last at 09:21:09, NIFTY 50 changes \
             10:06-10:13 (the reopening auction's indicative index) and next at 10:21:01; every \
             NIFTY and BANKNIFTY option trades last by 09:21:44 and next at 10:21:00. In the \
             09:22 minute five trades of shares outside the F&O list and eight of stock options \
             print (the last at 09:22:58). The NSE rule's 45-minute halt and 15-minute \
             reopening auction; no NSE circular recorded, UNVERIFIED (D-3320)",
        ),
    },
    Halt {
        day: 18_344, // 2020-03-23 Mon
        halted: Window { from: 599, to: 656 },
        reopening: Some(Window { from: 642, to: 656 }),
        cash: halt_seconds((9, 58, 8), (10, 57, 0)),
        index: halt_seconds((9, 58, 9), (10, 57, 1)),
        derivatives: halt_seconds((9, 58, 1), (10, 57, 0)),
        evidence: HaltEvidence::Suspected(
            "MEASURED from the GDFL day files, 2026-10-09 (charter, circuit-breaker boundary): \
             every share trades last at 09:58:07 and next at 10:57:00; NIFTY 50 changes last at \
             09:58:07 and NIFTY BANK at 09:58:08, both next at 10:57:01, NIFTY 50 changing only \
             10:42-10:49 in between (the reopening auction's indicative index); every NIFTY and \
             BANKNIFTY option trades last at 09:58:00 and next at 10:57:00 (stock options last \
             at 09:58:37). The NSE rule's shape; no NSE circular recorded, UNVERIFIED (D-3320)",
        ),
    },
    Halt {
        day: SYSTEMS_OUTAGE_DAY, // 2021-02-24 Wed
        halted: Window { from: 700, to: 944 },
        reopening: Some(Window { from: 930, to: 944 }),
        cash: halt_seconds((11, 40, 0), (15, 45, 0)),
        index: halt_seconds((11, 40, 0), (15, 45, 0)),
        derivatives: halt_seconds((11, 40, 0), (15, 45, 0)),
        evidence: HaltEvidence::Sourced(
            "SEBI Settlement Order SO/AB/EFD2/2023-24/6580 (charter §3): all segments halted \
             from 11:40, a 15-minute pre-open from 15:30, normal trading 15:45-17:00. GDFL \
             share prints run to 11:43:00, three minutes past the order's 11:40 (charter, \
             circuit-breaker boundary); the regulator's record is kept",
        ),
    },
];

/// A session day on which minutes the session owes are absent and the
/// absence was measured in the vendors' own answers, with no cause recorded
/// (WP-62, repair 3; DB-G47, DB-A12).
///
/// **It classifies nothing.** The minutes stay OWED: no NSE circular is
/// recorded for either day and the cause is UNVERIFIED, so neither is a
/// halt ([`HALTS`] takes only a window with a source; a pattern seen across
/// many instruments is not one by itself). What the entry records is that
/// the hole was looked for in the vendors' answers and is not a landing
/// failure of this store, so a reader can say "the vendor's answer lacks
/// it" instead of "a landing failure", with the evidence. A new day is a
/// new measurement with its source, appended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShortDay {
    /// Days since the epoch.
    pub day: i64,
    /// NIFTY 50's absent minutes as measured, first and last inclusive. Other
    /// instruments may lack other minutes; the evidence says which were
    /// counted.
    pub nifty_absent: Window,
    /// Whether GDFL's own NIFTY 50 file for the day was read. `false`: GDFL
    /// is NOT CHECKED for NIFTY that day, and "both vendors" is not claimed
    /// for it; the evidence names what was checked for other instruments.
    pub gdfl_checked: bool,
    /// The measurement and its sources, in words.
    pub evidence: &'static str,
}

/// Every [`ShortDay`] recorded, oldest first.
pub const SHORT_DAYS: [ShortDay; 2] = [
    ShortDay {
        day: 18_110, // 2019-08-02 Fri
        nifty_absent: Window { from: 839, to: 839 },
        gdfl_checked: true,
        evidence: "MEASURED 2026-10-07..13, cause UNVERIFIED: Kite's NIFTY 1min answer for the \
                   day lacks 13:59 (audit/pull.journal record 1421286, '2019-08-02: \
                   13:59-14:00'), and the GDFL NIFTY day file has no print in 13:59 (prints in \
                   13:56, 13:57, 13:58 and 14:00; state/gdfl-board/zerodha-holes.md; the store's \
                   GDFL NIFTY 1s, read 2026-10-13: 47, 55, 48 and 33 seconds, none in 13:59); \
                   Zerodha NIFTY, BANKNIFTY and RELIANCE each hold 374 of 375 minutes that \
                   day, 13:59 absent, and GDFL's BANKNIFTY 1s has no second in 13:59 either \
                   (bar files read 2026-10-13). The /db audit's holes scan (DB-G47; 47 \
                   stocks scanned) found 13:59 absent from Zerodha in at least 90% of the stocks \
                   held that day, and from GDFL in the two it folded from 1s (AXISBANK, \
                   BAJFINANCE). No NSE circular recorded; the minute stays owed",
    },
    ShortDay {
        day: 19_058, // 2022-03-07 Mon
        nifty_absent: Window { from: 622, to: 624 },
        gdfl_checked: true,
        evidence: "MEASURED 2026-10-07..13, cause UNVERIFIED: Kite's NIFTY 1min answer for the \
                   day lacks 10:22-10:24 (audit/pull.journal record 1421400, '2022-03-07: \
                   10:22-10:25'), and Zerodha NIFTY holds 372 of 375 minutes. GDFL's NIFTY 1s \
                   for the day (its first import was refused, logs/cli/events.ndjson lines 873 \
                   and 1490; imported again 2026-10-11 as bar-revisions-v1 generation 1, 22,220 \
                   seconds) holds no second in 10:22-10:24 (55 in 10:21, 19 in 10:25; read \
                   2026-10-13). BANKNIFTY lacks more, the same minutes in both vendors: \
                   Zerodha holds 325 of 375 and GDFL's BANKNIFTY 1s (generation 1, 19,326 \
                   seconds) has a second in the same 325, both lacking 09:54-09:57 and \
                   10:10-10:55 (read 2026-10-13). Stocks lack other minutes: Zerodha RELIANCE \
                   holds 360 of 375, 15 absent between 09:18 and 09:37, and GDFL's RELIANCE 1s \
                   holds no second in 09:17-09:38 (both read 2026-10-13); the /db audit's holes \
                   scan (DB-G47; 47 stocks scanned) found 09:23-09:38 absent, scattered (11 or \
                   more minutes), from Zerodha in at least 90% of the stocks held that day, and \
                   from GDFL in the two it folded from 1s (AXISBANK, BAJFINANCE). No NSE \
                   circular recorded; the minutes stay owed",
    },
];

/// The [`ShortDay`] on `epoch_day`, if one was recorded. A walk of the
/// two-entry [`SHORT_DAYS`]: O(1), no allocation. Timed inside
/// [`record_of`] by `pull::calendar::record_tests::calendar_of_record_costs`,
/// an ignored test: UNVERIFIED as a gated measurement.
#[must_use]
pub fn short_day_on(epoch_day: i64) -> Option<ShortDay> {
    SHORT_DAYS
        .iter()
        .find(|short| short.day == epoch_day)
        .copied()
}

/// The halt on `epoch_day`, if one was recorded. A walk of the three-entry
/// [`HALTS`]: O(1), no allocation. Timed inside [`record_of`] by
/// `pull::calendar::record_tests::calendar_of_record_costs`, an ignored
/// test: UNVERIFIED as a gated measurement.
#[must_use]
pub fn halt_on(epoch_day: i64) -> Option<Halt> {
    for halt in &HALTS {
        if halt.day == epoch_day {
            return Some(*halt);
        }
    }
    None
}

/// What the exchange was doing in one minute of one day, for one segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Phase {
    /// The exchange did not trade that day.
    Closed,
    /// Before the day's first recorded phase.
    BeforeOpen,
    /// The opening call auction, 09:00–09:14 on a regular day, cash and index
    /// only ([`PRE_OPEN_MINUTE`]).
    PreOpen,
    /// Continuous trading: the only phase whose minutes a coverage reader
    /// owes.
    Continuous,
    /// A market-wide halt; [`halt_on`] says whether it is sourced or
    /// suspected. A derivative's whole halt is this phase: no reopening call
    /// auction for derivatives is in the charter, and no NIFTY or BANKNIFTY
    /// option traded from 09:21:45 to 10:20:59 on 2020-03-13 (charter,
    /// circuit-breaker boundary).
    Halted,
    /// The call auction that reopened the cash market after a halt, and the
    /// index computed from it: cash and index only.
    ReopeningAuction,
    /// Between two windows of a special session (the DR drills' switch-over).
    Break,
    /// A closing-auction-eligible share's closing auction, 15:15–15:29 from
    /// 2026-08-03 (charter §7, §9), on [`crate::vendor::Venue::NseCash`]
    /// only. **Never an index's phase** (D-3320): indices stay
    /// `[09:15, 15:30)`, and whether an index level freezes while its
    /// constituents are in the auction has no source; it is UNVERIFIED and
    /// not modelled.
    ClosingAuction,
    /// Past the day's last recorded phase. NSE's post-close session timings
    /// are not a charter row, so this says only that the close has passed.
    PostClose,
    /// No calendar authority for the day, or a fact the answer needs is not
    /// known: the share's closing-auction eligibility, or the venue's hours.
    Unverified,
}

/// Whether a share is eligible for the closing auction on the day asked
/// about. Read only for [`crate::vendor::Venue::NseCash`]; an index or a
/// derivative ignores it (D-3320).
///
/// From the dated NSE security master's `ElgbltyClsgAuctnSsn` flag (charter
/// §9; [`crate::cash_session_cache::VerifiedLifecycleMaster::cas_eligibility`]).
/// `Unknown` is never read as `NotEligible`: "missing rows must never become
/// flag 0".
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CasEligibility {
    /// Flag 1.
    Eligible,
    /// Flag 0.
    NotEligible,
    /// No dated flag is held.
    Unknown,
}

/// The phase of `minute` (minutes since IST midnight) on `epoch_day` for
/// `venue`, on [`exchange_kind_of`].
///
/// - A closed day is [`Phase::Closed`]; an unmeasured or unsized one
///   [`Phase::Unverified`].
/// - A recorded halt's minutes are [`Phase::Halted`], and its reopening call
///   auction's are [`Phase::ReopeningAuction`] for cash and the index; a
///   derivative is [`Phase::Halted`] for the whole halt.
/// - A special session's windows are continuous; before them is
///   [`Phase::BeforeOpen`] (or [`Phase::PreOpen`] on a market day that opens
///   at 09:15), between them [`Phase::Break`] and after them
///   [`Phase::PostClose`].
/// - A regular day reads the venue's dated hours ([`crate::vendor::Venue::hours_on`],
///   the one hours authority), the charter's 09:00 pre-open for cash and the
///   index, and from 2026-08-03 the closing auction at 15:15–15:29 for a cash
///   share according to `cas` — `Unknown` there is [`Phase::Unverified`],
///   never a guess. An index stays continuous to 15:29 (D-3320).
///
/// # Cost
///
/// [`exchange_kind_of`], one walk of the three-entry [`HALTS`] and one dated
/// hours lookup: O(1), no allocation. Timed by
/// `pull::calendar::record_tests::calendar_of_record_costs` (release,
/// 2026-10-09, three runs: p50 13 ns, p99 23 ns). That test is ignored, so a regression
/// fails no gate: UNVERIFIED as a gated measurement.
#[must_use]
pub fn phase_at(
    venue: crate::vendor::Venue,
    epoch_day: i64,
    minute: u16,
    cas: CasEligibility,
) -> Phase {
    if minute >= MINUTES_PER_DAY {
        return Phase::Unverified;
    }
    let session = match exchange_kind_of(epoch_day) {
        DayKind::Closed => return Phase::Closed,
        DayKind::Unmeasured | DayKind::OpenLengthUnmeasured => return Phase::Unverified,
        DayKind::Open(session) => session,
    };
    if let Some(halt) = halt_on(epoch_day)
        && halt.halted.holds(minute)
    {
        // THE REOPENING CALL AUCTION IS THE CASH MARKET'S. Every recorded
        // auction lies inside its halt (`each_halt_names_its_seconds_and_whole_minutes`),
        // so a derivative's minute there is simply halted.
        let auction = venue != crate::vendor::Venue::NseDerivatives
            && halt.reopening.is_some_and(|w| w.holds(minute));
        return if auction {
            Phase::ReopeningAuction
        } else {
            Phase::Halted
        };
    }
    if session == Session::full() {
        return regular_phase(venue, epoch_day, minute, cas);
    }
    if session.expects(minute) {
        return Phase::Continuous;
    }
    let first = session.windows.first().map_or(0, |w| w.from);
    let last = session
        .windows
        .get(usize::from(session.count).saturating_sub(1))
        .map_or(0, |w| w.to);
    if minute < first {
        let market_day = special_session(epoch_day).is_some_and(|s| s.kind.is_market_day());
        if market_day && pre_opens(venue, first, minute) {
            Phase::PreOpen
        } else {
            Phase::BeforeOpen
        }
    } else if minute > last {
        Phase::PostClose
    } else {
        Phase::Break
    }
}

/// The phase of one SECOND of one IST day for one segment: [`phase_at`] of
/// its minute, refined on a halt day by the segment's own [`HaltSeconds`]
/// (DB-B43, D-3320).
///
/// A halt does not begin on a minute. On 2020-03-13 the index changes last
/// at 09:21:09 and is halted from 09:21:10, so at minute resolution the 50
/// flat seconds 09:21:10–09:21:59 were served as `continuous`, and 10:21:00
/// as well although the index resumes at 10:21:01 (the 2020-03-23 halt the
/// same way: 52 seconds). A second inside the segment's `from..resume` is
/// [`Phase::Halted`], or [`Phase::ReopeningAuction`] where its minute is the
/// auction's; a second outside it keeps its minute's phase. One-second
/// series only: a minute bar is labelled by [`phase_at`], whose whole halted
/// minutes `each_halt_names_its_seconds_and_whole_minutes` binds to these
/// seconds. O(1): one walk of the three-entry [`HALTS`].
///
/// UNVERIFIED: this cost names no test or bench that proves it (docs/06-limits.md,
/// "Cost claims of the /db fix round that name no proof").
#[must_use]
pub fn phase_at_second(
    venue: crate::vendor::Venue,
    epoch_day: i64,
    second: u32,
    cas: CasEligibility,
) -> Phase {
    let Ok(minute) = u16::try_from(second / 60) else {
        return Phase::Unverified;
    };
    let by_minute = phase_at(venue, epoch_day, minute, cas);
    let Some(halt) = halt_on(epoch_day) else {
        return by_minute;
    };
    let seconds = halt.seconds(venue);
    if !(seconds.from..seconds.resume).contains(&second) {
        return by_minute;
    }
    match by_minute {
        Phase::Continuous | Phase::Halted => Phase::Halted,
        other => other,
    }
}

/// Whether `minute` is in the charter's 09:00–09:14 opening auction before a
/// session that opens at `open`: cash and index only, and only before 09:15.
fn pre_opens(venue: crate::vendor::Venue, open: u16, minute: u16) -> bool {
    venue != crate::vendor::Venue::NseDerivatives
        && open == OPEN_MINUTE
        && matches!(minute, PRE_OPEN_MINUTE..OPEN_MINUTE)
}

/// [`phase_at`] on a regular full session.
fn regular_phase(
    venue: crate::vendor::Venue,
    epoch_day: i64,
    minute: u16,
    cas: CasEligibility,
) -> Phase {
    // ONE REFUSAL FOR EVERY MISSING FACT: a day no civil date carries, a
    // dated row without verified hours, or a row that is not continuous
    // trading. No shipped row is either of the last two today (vendor's own
    // tables), and the answer for all three is the same `Unverified`.
    let Some((civil, hours)) = civil_day(epoch_day)
        .and_then(|civil| venue.hours_on(civil).ok().map(|hours| (civil, hours)))
        .filter(|(_, hours)| hours.kind() == crate::vendor::SessionKind::Continuous)
    else {
        return Phase::Unverified;
    };
    let at = u32::from(minute);
    if at < hours.open_minute() {
        let open = u16::try_from(hours.open_minute()).unwrap_or(u16::MAX);
        return if pre_opens(venue, open, minute) {
            Phase::PreOpen
        } else {
            Phase::BeforeOpen
        };
    }
    // THE AUCTION BAND IS DECIDED BEFORE THE VENUE'S CLOSE IS READ, so the
    // cash row's close — whatever a dated row makes it — never turns an
    // ineligible share's 15:15–15:29 into a close or an eligible share's
    // into continuous trading. Cash only: derivatives have no closing
    // auction, and an index stays `[09:15, 15:30)` (D-3320).
    if venue == crate::vendor::Venue::NseCash
        && crate::vendor::cash_auction_eligibility_required(civil)
        && matches!(minute, CAS_CONTINUOUS_CLOSE..CAS_SESSION_END)
    {
        return match cas {
            CasEligibility::Eligible => Phase::ClosingAuction,
            CasEligibility::NotEligible => Phase::Continuous,
            CasEligibility::Unknown => Phase::Unverified,
        };
    }
    if at < hours.close_minute() {
        Phase::Continuous
    } else {
        Phase::PostClose
    }
}

/// `epoch_day` as a civil day, or `None` outside what a [`crate::session::Day`]
/// can carry.
fn civil_day(epoch_day: i64) -> Option<crate::session::Day> {
    u32::try_from(epoch_day)
        .ok()
        .and_then(|days| crate::session::Day::from_days(days).ok())
}

/// The most bands one day's phases split into.
///
/// Fifteen phase boundaries are possible — midnight, the 09:00 pre-open, the
/// 09:15 open, the 15:15 and 15:30 auction bounds, the venue's open and
/// close, two windows' two edges, and a halt's and its auction's two edges —
/// and each starts at most one band.
pub const MAX_BANDS: usize = 16;

/// One stretch of a day in one phase: inclusive minute opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Band {
    /// The phase.
    pub phase: Phase,
    /// First minute.
    pub from: u16,
    /// Last minute, inclusive.
    pub to: u16,
}

impl Band {
    /// Minutes in the band.
    #[must_use]
    pub const fn minutes(self) -> u16 {
        self.to.saturating_sub(self.from).saturating_add(1)
    }
}

/// A whole day of phases for one segment: the bands a page shades and a
/// coverage reader counts, in minute order, covering 00:00–23:59 exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayPhases {
    bands: [Band; MAX_BANDS],
    count: u8,
}

impl DayPhases {
    /// The bands, in minute order.
    #[must_use]
    pub fn bands(&self) -> &[Band] {
        self.bands.get(..usize::from(self.count)).unwrap_or(&[])
    }

    /// The phase of `minute`, read from the bands; [`Phase::Unverified`] past
    /// the day.
    #[must_use]
    pub fn phase_at(&self, minute: u16) -> Phase {
        for band in self.bands() {
            if band.from <= minute && minute <= band.to {
                return band.phase;
            }
        }
        Phase::Unverified
    }

    /// Minutes the day spends in `phase`. `minutes(Phase::Continuous)` is the
    /// one-minute bars a continuous series owes that day.
    #[must_use]
    pub fn minutes(&self, phase: Phase) -> u16 {
        self.bands()
            .iter()
            .filter(|band| band.phase == phase)
            .fold(0_u16, |total, band| total.saturating_add(band.minutes()))
    }

    fn push(&mut self, phase: Phase, from: u16, to: u16) {
        let used = usize::from(self.count);
        if let Some(last) = used.checked_sub(1).and_then(|at| self.bands.get_mut(at))
            && last.phase == phase
        {
            last.to = to;
            return;
        }
        if let Some(slot) = self.bands.get_mut(used) {
            *slot = Band { phase, from, to };
            self.count = self.count.saturating_add(1);
        }
    }
}

/// The phases of `epoch_day` for `venue`, as bands.
///
/// Every minute's phase is [`phase_at`]'s: the day is cut at every minute
/// where any input to [`phase_at`] changes, each piece takes [`phase_at`] of
/// its first minute, and equal neighbours merge.
/// `phases_of_agrees_with_phase_at_on_every_minute` checks all 1,440 minutes
/// of every special, halt and closing-auction case.
///
/// # Cost
///
/// At most [`MAX_BANDS`] calls of [`phase_at`], each beside one pass over at
/// most sixteen cuts: O(1), no allocation, no sort. Timed by
/// `pull::calendar::record_tests::calendar_of_record_costs` (release,
/// 2026-10-09, three runs: p50 109-112 ns, p99 169-174 ns). That test is ignored, so a
/// regression fails no gate: UNVERIFIED as a gated measurement.
#[must_use]
pub fn phases_of(venue: crate::vendor::Venue, epoch_day: i64, cas: CasEligibility) -> DayPhases {
    let mut cuts = [MINUTES_PER_DAY; MAX_BANDS];
    let mut used = 0_usize;
    let mut cut = |minute: u32| {
        if let Ok(minute) = u16::try_from(minute)
            && minute < MINUTES_PER_DAY
            && let Some(slot) = cuts.get_mut(used)
        {
            *slot = minute;
            used += 1;
        }
    };
    for fixed in [
        0,
        PRE_OPEN_MINUTE,
        OPEN_MINUTE,
        CAS_CONTINUOUS_CLOSE,
        CAS_SESSION_END,
    ] {
        cut(u32::from(fixed));
    }
    if let Some(hours) = civil_day(epoch_day).and_then(|civil| venue.hours_on(civil).ok()) {
        cut(hours.open_minute());
        cut(hours.close_minute());
    }
    if let DayKind::Open(session) = exchange_kind_of(epoch_day) {
        for window in session.windows.iter().take(usize::from(session.count)) {
            cut(u32::from(window.from));
            cut(u32::from(window.to) + 1);
        }
    }
    if let Some(halt) = halt_on(epoch_day) {
        for window in [Some(halt.halted), halt.reopening].into_iter().flatten() {
            cut(u32::from(window.from));
            cut(u32::from(window.to) + 1);
        }
    }
    let mut phases = DayPhases {
        bands: [Band {
            phase: Phase::Unverified,
            from: 0,
            to: 0,
        }; MAX_BANDS],
        count: 0,
    };
    // EACH PIECE ENDS AT THE NEAREST LATER CUT, so no sort is needed: at most
    // MAX_BANDS pieces, each one pass over the MAX_BANDS cuts. Unused slots
    // hold MINUTES_PER_DAY, the end of the day, and a cut named twice is one
    // boundary. The first cut is midnight, so the pieces tile the day.
    let mut from = 0_u16;
    for _ in 0..MAX_BANDS {
        let next = cuts
            .iter()
            .copied()
            .filter(|minute| *minute > from)
            .min()
            .unwrap_or(MINUTES_PER_DAY);
        phases.push(phase_at(venue, epoch_day, from, cas), from, next - 1);
        if next >= MINUTES_PER_DAY {
            break;
        }
        from = next;
    }
    phases
}

/// A day of the week.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Weekday {
    /// Monday.
    Monday,
    /// Tuesday.
    Tuesday,
    /// Wednesday.
    Wednesday,
    /// Thursday.
    Thursday,
    /// Friday.
    Friday,
    /// Saturday.
    Saturday,
    /// Sunday.
    Sunday,
}

impl Weekday {
    /// Three letters.
    #[must_use]
    pub const fn short(self) -> &'static str {
        match self {
            Self::Monday => "Mon",
            Self::Tuesday => "Tue",
            Self::Wednesday => "Wed",
            Self::Thursday => "Thu",
            Self::Friday => "Fri",
            Self::Saturday => "Sat",
            Self::Sunday => "Sun",
        }
    }

    /// Saturday or Sunday.
    #[must_use]
    pub const fn is_weekend(self) -> bool {
        matches!(self, Self::Saturday | Self::Sunday)
    }
}

/// The weekday of `epoch_day`: 1970-01-01 was a Thursday. One remainder and
/// one match, and total: the remainder is taken before the shift, so neither
/// end of `i64` overflows
/// (`pull::calendar::record_tests::weekday_of_is_total_and_agrees_with_the_civil_calendar`).
#[must_use]
pub const fn weekday_of(epoch_day: i64) -> Weekday {
    match (epoch_day.rem_euclid(7) + 3) % 7 {
        0 => Weekday::Monday,
        1 => Weekday::Tuesday,
        2 => Weekday::Wednesday,
        3 => Weekday::Thursday,
        4 => Weekday::Friday,
        5 => Weekday::Saturday,
        _ => Weekday::Sunday,
    }
}

/// A trading holiday as the circular that lists it names it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HolidayNote {
    /// Days since the epoch.
    pub day: i64,
    /// The circular's name for it.
    pub name: &'static str,
    /// The circular.
    pub circular: &'static str,
}

// Each circular with its own date, as the charter's D-2805 table records it.
const CMTR_36475: &str = "NSE/CMTR/36475 of 12 Dec 2017";
const CMTR_39612: &str = "NSE/CMTR/39612 of 7 Dec 2018";
const CMTR_40536: &str = "NSE/CMTR/40536 of 26 Mar 2019";
const MSD_42319: &str = "NSE/MSD/42319 of 7 Oct 2019";
const CMTR_71775: &str = "NSE/CMTR/71775 of 12 Dec 2025";

/// [`SOURCED_HOLIDAYS`] with the circular each is listed in and its name
/// there, day for day (`the_holiday_notes_are_the_sourced_holidays_day_for_day`).
/// The measured window's closed weekdays have no entry: they were measured
/// from the store, and no circular naming them is recorded in the charter.
pub const SOURCED_HOLIDAY_NOTES: [HolidayNote; 38] = [
    note(17_557, "Republic Day", CMTR_36475),
    note(17_575, "Mahashivratri", CMTR_36475),
    note(17_592, "Holi", CMTR_36475),
    note(17_619, "Mahavir Jayanti", CMTR_36475),
    note(17_620, "Good Friday", CMTR_36475),
    note(17_652, "Maharashtra Day", CMTR_36475),
    note(17_758, "Independence Day", CMTR_36475),
    note(17_765, "Bakri ID", CMTR_36475),
    note(17_787, "Ganesh Chaturthi", CMTR_36475),
    note(17_794, "Moharram", CMTR_36475),
    note(17_806, "Mahatma Gandhi Jayanti", CMTR_36475),
    note(17_822, "Dasera", CMTR_36475),
    note(17_842, "Diwali-Laxmi Pujan", CMTR_36475),
    note(17_843, "Diwali-Balipratipada", CMTR_36475),
    note(17_858, "Gurunanak Jayanti", CMTR_36475),
    note(17_890, "Christmas", CMTR_36475),
    note(17_959, "Mahashivratri", CMTR_39612),
    note(17_976, "Holi", CMTR_39612),
    note(18_003, "Mahavir Jayanti", CMTR_39612),
    note(18_005, "Good Friday", CMTR_39612),
    note(18_015, "Parliamentary Elections in Mumbai", CMTR_40536),
    note(18_017, "Maharashtra Day", CMTR_39612),
    note(18_052, "Id-Ul-Fitr", CMTR_39612),
    note(18_120, "Bakri Id", CMTR_39612),
    note(18_123, "Independence Day", CMTR_39612),
    note(18_141, "Ganesh Chaturthi", CMTR_39612),
    note(18_149, "Moharram", CMTR_39612),
    note(18_171, "Mahatma Gandhi Jayanti", CMTR_39612),
    note(18_177, "Dasera", CMTR_39612),
    note(18_190, "Assembly Elections in Maharashtra", MSD_42319),
    note(18_197, "Diwali-Balipratipada", CMTR_39612),
    note(18_212, "Gurunanak Jayanti", CMTR_39612),
    note(20_710, "Ganesh Chaturthi", CMTR_71775),
    note(20_728, "Mahatma Gandhi Jayanti", CMTR_71775),
    note(20_746, "Dussehra", CMTR_71775),
    note(20_767, "Diwali-Balipratipada", CMTR_71775),
    note(20_781, "Guru Nanak Jayanti", CMTR_71775),
    note(20_812, "Christmas", CMTR_71775),
];

const fn note(day: i64, name: &'static str, circular: &'static str) -> HolidayNote {
    HolidayNote {
        day,
        name,
        circular,
    }
}

/// The circular's note for `epoch_day`, if it is a sourced holiday. A walk of
/// the 38-entry table: O(1). Timed inside [`record_of`] by
/// `pull::calendar::record_tests::calendar_of_record_costs`, an ignored
/// test: UNVERIFIED as a gated measurement.
#[must_use]
pub fn holiday_note(epoch_day: i64) -> Option<HolidayNote> {
    for holiday in &SOURCED_HOLIDAY_NOTES {
        if holiday.day == epoch_day {
            return Some(*holiday);
        }
    }
    None
}

/// Why a day cannot be the day a contract expires and settles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NoSettlement {
    /// A Saturday or Sunday with no session.
    Weekend,
    /// A weekday trading holiday; `note` where a recorded circular names it.
    Holiday {
        /// The circular's name for it, if one is recorded.
        note: Option<HolidayNote>,
    },
    /// The exchange traded only a special session it never meant to be a
    /// market day ([`SpecialKind::is_market_day`]): a Muhurat or a DR drill.
    SpecialSessionOnly(SpecialKind),
}

impl std::fmt::Display for NoSettlement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Weekend => f.write_str("a weekend with no session"),
            Self::Holiday { note: Some(note) } => {
                write!(f, "a trading holiday, {} ({})", note.name, note.circular)
            }
            Self::Holiday { note: None } => f.write_str(
                "a weekday no instrument traded, measured; no recorded circular names it",
            ),
            Self::SpecialSessionOnly(kind) => {
                write!(f, "a {} only, not a regular market day", kind.label())
            }
        }
    }
}

/// Whether a day can be a contract's expiry day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExpiryDay {
    /// A regular market day: an ordinary session, the outage day, or a weekend
    /// Budget or full session.
    Session,
    /// Not a market day, and why.
    NotASession(NoSettlement),
    /// Outside the calendar of record, or a traded day of unrecorded length
    /// and kind: no answer.
    Unmeasured,
}

/// Whether `epoch_day` is a session a contract can expire on, on
/// [`exchange_kind_of`]. A Muhurat or a DR drill traded but is not one;
/// 2024-01-20, a Saturday full session, is. O(1): timed inside
/// [`record_of`] by `pull::calendar::record_tests::calendar_of_record_costs`,
/// an ignored test: UNVERIFIED as a gated measurement.
#[must_use]
pub fn expiry_day(epoch_day: i64) -> ExpiryDay {
    match exchange_kind_of(epoch_day) {
        // A MUHURAT THE CIRCULAR NAMES AND NO RECORD HAS SIZED (2026-11-08,
        // DB-B41) is still a Muhurat: a session, and no settlement day. Its
        // missing window is a question about minutes, not about expiries.
        DayKind::OpenLengthUnmeasured if listed(&SOURCED_SPECIAL, epoch_day) => {
            ExpiryDay::NotASession(NoSettlement::SpecialSessionOnly(SpecialKind::Muhurat))
        }
        DayKind::Unmeasured | DayKind::OpenLengthUnmeasured => ExpiryDay::Unmeasured,
        DayKind::Closed if weekday_of(epoch_day).is_weekend() => {
            ExpiryDay::NotASession(NoSettlement::Weekend)
        }
        DayKind::Closed => ExpiryDay::NotASession(NoSettlement::Holiday {
            note: holiday_note(epoch_day),
        }),
        DayKind::Open(_) => match special_session(epoch_day) {
            Some(special) if !special.kind.is_market_day() => {
                ExpiryDay::NotASession(NoSettlement::SpecialSessionOnly(special.kind))
            }
            _ => ExpiryDay::Session,
        },
    }
}

/// The most days [`settlement_on_or_before`] walks back.
///
/// The longest run of non-settlement days from 2018-01-01 to 2026-12-31 is
/// four (`no_run_of_non_settlement_days_reaches_the_walk_bound` measures it),
/// so a walk that passes this bound has met a gap no calendar fact explains.
pub const MAX_SETTLEMENT_WALK: i64 = 7;

/// A stated expiry and the session it settles on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Settlement {
    /// The day the contract states.
    pub stated: i64,
    /// The session it settles on: `stated` itself, or the last session before
    /// it.
    pub settles: i64,
    /// Why `stated` is not a session, when it is not.
    pub moved: Option<NoSettlement>,
}

/// Why no settlement day or shift could be given.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettlementRefusal {
    /// The walk reached a day the calendar of record does not answer.
    Unmeasured {
        /// That day.
        day: i64,
    },
    /// No session within [`MAX_SETTLEMENT_WALK`] days before the stated day.
    NoSessionWithin {
        /// The stated day.
        stated: i64,
    },
    /// The day asked about is not a session, so nothing was moved TO it.
    NotASession {
        /// That day.
        day: i64,
        /// Why.
        why: NoSettlement,
    },
}

impl std::fmt::Display for SettlementRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unmeasured { day } => {
                write!(f, "day {day} is outside the calendar of record")
            }
            Self::NoSessionWithin { stated } => write!(
                f,
                "no session within {MAX_SETTLEMENT_WALK} days on or before day {stated}"
            ),
            Self::NotASession { day, why } => {
                write!(f, "day {day} is not a session: {why}")
            }
        }
    }
}

/// The session a contract stated to expire on `stated` settles on: `stated`
/// when it is a session, otherwise the last session before it — NSE's rule
/// that an expiry on a trading holiday moves to the previous trading day
/// (D-2806 item 3, extended from the monthly to every stated day form).
///
/// MIDCPNIFTY 22JAN24 states 2024-01-22, a closed Monday, and settles on the
/// Saturday full session 2024-01-20; 29JUN23 settles on 2023-06-28. A Muhurat
/// or a DR drill is walked past. DB-G51.
///
/// # Errors
///
/// [`SettlementRefusal::Unmeasured`] when the walk reaches a day the calendar
/// of record does not answer, and [`SettlementRefusal::NoSessionWithin`] past
/// [`MAX_SETTLEMENT_WALK`] days.
///
/// # Cost
///
/// At most `MAX_SETTLEMENT_WALK + 1` calls of [`expiry_day`]: O(1). Timed by
/// `pull::calendar::record_tests::calendar_of_record_costs` (release,
/// 2026-10-09, three runs: p50 2 ns, p99 20-23 ns). That test is ignored, so a regression
/// fails no gate: UNVERIFIED as a gated measurement.
pub fn settlement_on_or_before(stated: i64) -> Result<Settlement, SettlementRefusal> {
    let mut moved = None;
    for back in 0..=MAX_SETTLEMENT_WALK {
        let day = stated
            .checked_sub(back)
            .ok_or(SettlementRefusal::Unmeasured { day: stated })?;
        match expiry_day(day) {
            ExpiryDay::Session => {
                return Ok(Settlement {
                    stated,
                    settles: day,
                    moved,
                });
            }
            ExpiryDay::Unmeasured => return Err(SettlementRefusal::Unmeasured { day }),
            ExpiryDay::NotASession(why) => {
                if moved.is_none() {
                    moved = Some(why);
                }
            }
        }
    }
    Err(SettlementRefusal::NoSessionWithin { stated })
}

/// The day an expiry was moved from, and why.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Shift {
    /// The usual expiry day that was not a session.
    pub from: i64,
    /// Why it was not.
    pub why: NoSettlement,
}

/// Why `listed`, an expiry that falls on a session, is not on the
/// underlying's `usual` weekday: `Some` when the usual day after it was not a
/// session and every day between was not one either — an expiry moved back
/// over a holiday — and `None` when `listed` is on its usual weekday or a
/// session lies between (the move is not a holiday's).
///
/// `usual` is the caller's to give — the weekday most of that underlying's
/// stored expiries of the era fall on. NSE's changing expiry weekdays are not
/// encoded here. BANKNIFTY's 2018-09-12 (Wed) is `Some`, moved from
/// Thursday 2018-09-13, Ganesh Chaturthi (NSE/CMTR/36475). DB-076.
///
/// # Errors
///
/// [`SettlementRefusal::NotASession`] when `listed` is not a session (ask
/// [`settlement_on_or_before`] first), and [`SettlementRefusal::Unmeasured`]
/// when a day the walk needs is not answered.
///
/// # Cost
///
/// At most seven calls of [`expiry_day`]: O(1), by the same count
/// [`settlement_on_or_before`]'s measurement in
/// `pull::calendar::record_tests::calendar_of_record_costs` covers; not timed
/// on its own: UNVERIFIED as a gated measurement.
pub fn shift_reason(listed: i64, usual: Weekday) -> Result<Option<Shift>, SettlementRefusal> {
    match expiry_day(listed) {
        ExpiryDay::Session => {}
        ExpiryDay::Unmeasured => return Err(SettlementRefusal::Unmeasured { day: listed }),
        ExpiryDay::NotASession(why) => {
            return Err(SettlementRefusal::NotASession { day: listed, why });
        }
    }
    if weekday_of(listed) == usual {
        return Ok(None);
    }
    for ahead in 1..=6 {
        let day = listed
            .checked_add(ahead)
            .ok_or(SettlementRefusal::Unmeasured { day: listed })?;
        match expiry_day(day) {
            ExpiryDay::Session => return Ok(None),
            ExpiryDay::Unmeasured => return Err(SettlementRefusal::Unmeasured { day }),
            ExpiryDay::NotASession(why) if weekday_of(day) == usual => {
                return Ok(Some(Shift { from: day, why }));
            }
            ExpiryDay::NotASession(_) => {}
        }
    }
    Ok(None)
}

/// Everything the calendar of record says about one day, for a day record
/// on `/calendar.json`, `/dayflags.json` and `/db`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayRecord {
    /// Days since the epoch.
    pub day: i64,
    /// The weekday.
    pub weekday: Weekday,
    /// [`exchange_kind_of`].
    pub kind: DayKind,
    /// The special session, if one was recorded.
    pub special: Option<SpecialSession>,
    /// The halt, if one was recorded.
    pub halt: Option<Halt>,
    /// The measured, unexplained short day, if one was recorded
    /// ([`SHORT_DAYS`]); its minutes are still owed.
    pub short: Option<ShortDay>,
    /// The circular's holiday note, if the day is a sourced holiday.
    pub holiday: Option<HolidayNote>,
    /// Whether a contract can expire on it.
    pub expiry: ExpiryDay,
}

/// [`DayRecord`] for `epoch_day`. O(1): the bounded walks above, once each.
/// Timed by `pull::calendar::record_tests::calendar_of_record_costs`
/// (release, 2026-10-09, three runs: p50 5 ns, p99 26 ns). That test is ignored, so a
/// regression fails no gate: UNVERIFIED as a gated measurement.
#[must_use]
pub fn record_of(epoch_day: i64) -> DayRecord {
    DayRecord {
        day: epoch_day,
        weekday: weekday_of(epoch_day),
        kind: exchange_kind_of(epoch_day),
        special: special_session(epoch_day),
        halt: halt_on(epoch_day),
        short: short_day_on(epoch_day),
        holiday: holiday_note(epoch_day),
        expiry: expiry_day(epoch_day),
    }
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod tests {
    use super::*;

    /// **THE TWO MEASURED SHORT DAYS ARE IN THE CALENDAR OF RECORD** (WP-62,
    /// repair 3; DB-G47): 2019-08-02 and 2022-03-07 were plain sessions with
    /// no word, although the vendors' answers had been read for them. They
    /// are recorded with their sources and classify nothing: each is still a
    /// regular session that owes its whole day, and neither is a halt. Both
    /// say GDFL was read for NIFTY: 2022-03-07's entry said "NOT CHECKED"
    /// from before that day's GDFL indices were imported again (2026-10-11),
    /// and the re-imported seconds lack the same three minutes.
    #[test]
    fn the_measured_short_days_are_recorded_and_still_owe_their_minutes() {
        let at = |y, m, d| {
            i64::from(
                crate::session::Day::new(y, m, d)
                    .expect("a real day")
                    .days_from_epoch(),
            )
        };
        let august = short_day_on(at(2019, 8, 2)).expect("2019-08-02 is recorded");
        assert_eq!(august.nifty_absent, Window { from: 839, to: 839 }, "13:59");
        assert!(august.gdfl_checked);
        assert!(august.evidence.contains("1421286") && august.evidence.contains("UNVERIFIED"));
        let march = short_day_on(at(2022, 3, 7)).expect("2022-03-07 is recorded");
        assert_eq!(
            march.nifty_absent,
            Window { from: 622, to: 624 },
            "10:22-10:24"
        );
        // GDFL's NIFTY day was re-imported on 2026-10-11 and read for these
        // minutes; the entry said "NOT CHECKED" from before that import.
        assert!(march.gdfl_checked, "GDFL's NIFTY 1s for the day was read");
        assert!(march.evidence.contains("1421400") && march.evidence.contains("generation 1"));
        assert!(!march.evidence.contains("NOT CHECKED"));
        // NIFTY's window is the field; another index's, measured the same
        // way in both vendors, is in the words (BANKNIFTY lacks 50 minutes).
        assert!(march.evidence.contains("09:54-09:57 and 10:10-10:55"));
        assert!(august.evidence.contains("BANKNIFTY"));
        for day in [august.day, march.day] {
            let record = record_of(day);
            assert_eq!(record.short.map(|short| short.day), Some(day));
            assert_eq!(record.halt, None, "not a halt");
            assert!(
                matches!(record.kind, DayKind::Open(session) if session.bars() == 375),
                "a regular session that owes 375 minutes: {:?}",
                record.kind
            );
        }
        assert_eq!(short_day_on(at(2019, 8, 1)), None);
        assert_eq!(record_of(at(2022, 3, 8)).short, None);
        assert!(
            SHORT_DAYS
                .windows(2)
                .all(|pair| matches!(pair, [older, newer] if older.day < newer.day)),
            "oldest first"
        );
    }

    /// D-2805: the circular-sourced spans either side of the measured window.
    #[test]
    fn the_sourced_spans_follow_the_circulars_and_name_each_muhurat() {
        let at = |y, m, d| {
            i64::from(
                crate::session::Day::new(y, m, d)
                    .expect("a real day")
                    .days_from_epoch(),
            )
        };
        assert_eq!(
            (SOURCED_FIRST_DAY, SOURCED_LAST_DAY),
            (at(2018, 1, 1), at(2026, 12, 31))
        );
        // DB-B41: the record runs to the last day NSE/CMTR/71775 covers. The
        // four holidays after 2026-10-04 are closed, the weekday after the
        // old end is a regular session, the Sunday Muhurat is a session of
        // no recorded length, and the day after the record is not answered.
        for (y, m, d) in [
            (2026, 10, 20),
            (2026, 11, 10),
            (2026, 11, 24),
            (2026, 12, 25),
        ] {
            assert_eq!(sourced_kind_of(at(y, m, d)), DayKind::Closed, "{y}-{m}-{d}");
            assert_eq!(
                holiday_note(at(y, m, d)).map(|n| n.circular),
                Some(CMTR_71775),
                "{y}-{m}-{d}"
            );
        }
        assert_eq!(
            sourced_kind_of(at(2026, 10, 5)),
            DayKind::Open(Session::full()),
            "the first weekday past the old end, which every pull failed on"
        );
        assert_eq!(
            sourced_kind_of(at(2026, 11, 8)),
            DayKind::OpenLengthUnmeasured,
            "the Sunday Muhurat, timings not notified"
        );
        assert_eq!(
            exchange_kind_of(at(2026, 11, 8)),
            DayKind::OpenLengthUnmeasured,
            "no window is recorded, so none is claimed"
        );
        assert_eq!(
            sourced_kind_of(at(2026, 12, 31)),
            DayKind::Open(Session::full())
        );
        assert_eq!(sourced_kind_of(at(2027, 1, 1)), DayKind::Unmeasured);
        assert_eq!((FIRST_DAY, LAST_DAY), (at(2019, 12, 2), at(2026, 9, 4)));
        // Every listed holiday is a weekday inside a sourced span, and closed.
        for day in SOURCED_HOLIDAYS {
            assert!((day + 3).rem_euclid(7) < 5, "{day} is a weekday");
            assert!(!(FIRST_DAY..=LAST_DAY).contains(&day), "{day}");
            if !SOURCED_SPECIAL.contains(&day) {
                assert_eq!(sourced_kind_of(day), DayKind::Closed, "{day}");
            }
        }
        assert!(SOURCED_HOLIDAYS.windows(2).all(|w| w.first() < w.last()));
        assert_eq!(
            sourced_kind_of(at(2019, 4, 29)),
            DayKind::Closed,
            "election, NSE/CMTR/40536"
        );
        assert_eq!(
            sourced_kind_of(at(2019, 10, 21)),
            DayKind::Closed,
            "election, NSE/MSD/42319"
        );
        assert_eq!(
            sourced_kind_of(at(2018, 11, 7)),
            DayKind::OpenLengthUnmeasured
        );
        assert_eq!(
            sourced_kind_of(at(2019, 10, 27)),
            DayKind::OpenLengthUnmeasured
        );
        assert_eq!(
            sourced_kind_of(at(2018, 9, 3)),
            DayKind::Open(Session::full()),
            "GDFL's first day"
        );
        assert_eq!(
            sourced_kind_of(at(2018, 1, 1)),
            DayKind::Open(Session::full())
        );
        assert_eq!(
            sourced_kind_of(at(2018, 9, 8)),
            DayKind::Closed,
            "a Saturday"
        );
        assert_eq!(sourced_kind_of(at(2018, 9, 9)), DayKind::Closed, "a Sunday");
        assert_eq!(sourced_kind_of(at(2026, 9, 14)), DayKind::Closed);
        assert_eq!(
            sourced_kind_of(at(2026, 9, 30)),
            DayKind::Open(Session::full())
        );
        assert_eq!(sourced_kind_of(at(2026, 10, 2)), DayKind::Closed);
        assert_eq!(sourced_kind_of(at(2026, 10, 4)), DayKind::Closed);
        assert_eq!(sourced_kind_of(at(2017, 12, 29)), DayKind::Unmeasured);
        // 2018-09-03..=2019-12-01: 301 regular sessions and the two Muhurats,
        // the 303 GDFL day zips of that span.
        assert_eq!(sourced_sessions(at(2018, 9, 3), at(2019, 12, 1)), Some(303));
        assert_eq!(sourced_sessions(at(2026, 9, 5), at(2026, 9, 30)), Some(17));
    }

    /// **THE RECORD SAYS WHEN IT ENDS BEFORE IT HAS** (DB-B25, DB-B41). It
    /// expired on 2026-10-04 with no warning and every pull of a newer day
    /// failed; the horizon is what a page reads to warn 30 days ahead, and
    /// `past_record` is the part of a window nothing can judge.
    #[test]
    fn the_record_horizon_warns_before_the_end_and_names_the_days_past_it() {
        let last = SOURCED_LAST_DAY;
        let far = record_horizon(last - RECORD_ENDS_SOON_DAYS - 1);
        assert_eq!(
            (far.last_day, far.days_left, far.ends_soon, far.ended),
            (last, RECORD_ENDS_SOON_DAYS + 1, false, false)
        );
        let soon = record_horizon(last - RECORD_ENDS_SOON_DAYS);
        assert!(soon.ends_soon && !soon.ended, "{soon:?}");
        let on = record_horizon(last);
        assert_eq!((on.days_left, on.ends_soon, on.ended), (0, true, false));
        let past = record_horizon(last + 1);
        assert_eq!(
            (past.days_left, past.ends_soon, past.ended),
            (-1, true, true)
        );
        // Garbage in: no overflow, and a far past day is simply not soon.
        assert!(record_horizon(i64::MAX).ended);
        assert!(!record_horizon(i64::MIN).ends_soon);

        assert_eq!(past_record(SOURCED_FIRST_DAY, last), None, "wholly inside");
        assert_eq!(
            past_record(last, last),
            None,
            "the last day itself is answered"
        );
        assert_eq!(
            past_record(last - 3, last + 2),
            Some((last + 1, last + 2, 2))
        );
        assert_eq!(
            past_record(last + 5, last + 5),
            Some((last + 5, last + 5, 1))
        );
        assert_eq!(
            past_record(last + 5, last + 4),
            None,
            "an inverted window names nothing"
        );
        for day in last + 1..=last + 9 {
            assert_eq!(sourced_kind_of(day), DayKind::Unmeasured, "{day}");
        }
        // The Muhurat no circular has sized is a session and no settlement
        // day, never an unmeasured one: an expiry stated on it settles on
        // the session before it.
        assert_eq!(
            expiry_day(20_765),
            ExpiryDay::NotASession(NoSettlement::SpecialSessionOnly(SpecialKind::Muhurat))
        );
        assert_eq!(
            expiry_day(20_767),
            ExpiryDay::NotASession(NoSettlement::Holiday {
                note: holiday_note(20_767),
            })
        );
        assert_eq!(
            settlement_on_or_before(20_767).map(|s| s.settles),
            Ok(20_766),
            "Tuesday 2026-11-10 is a holiday; Monday the 9th is the session before it"
        );
        assert!(PAST_RECORD_FINDING.starts_with("request minute coverage"));
    }

    /// Sessions `sourced_kind_of` counts in `first..=last`.
    fn sourced_sessions(first: i64, last: i64) -> Option<u32> {
        let open = (first..=last)
            .filter(|&d| {
                matches!(
                    sourced_kind_of(d),
                    DayKind::Open(_) | DayKind::OpenLengthUnmeasured
                )
            })
            .count();
        u32::try_from(open).ok()
    }

    /// The global calendar, which run identity hashes, is untouched by D-2805.
    #[test]
    fn the_global_calendar_is_not_widened_by_the_sourced_spans() {
        assert_eq!(kind_of(FIRST_DAY - 1), DayKind::Unmeasured);
        assert_eq!(kind_of(LAST_DAY + 1), DayKind::Unmeasured);
        for day in [FIRST_DAY, 19_000, LAST_DAY] {
            assert_eq!(sourced_kind_of(day), kind_of(day), "{day}");
        }
        assert_eq!(sourced_kind_of(SOURCED_LAST_DAY + 1), DayKind::Unmeasured);
        assert_eq!(sourced_kind_of(SOURCED_FIRST_DAY - 1), DayKind::Unmeasured);
    }

    /// audit-20261003 attackdata-6 (D-1532). A Muhurat whose LENGTH was never
    /// measured is still a day the exchange traded, so it is a session in the
    /// count. `sessions_between` counted only `Open(_)` and returned `Some(0)`
    /// for 2024-11-01 alone and 11 for a fortnight holding 12 traded days.
    #[test]
    fn a_muhurat_of_unmeasured_length_is_still_a_session() {
        assert_eq!(kind_of(20_028), DayKind::OpenLengthUnmeasured);
        assert_eq!(sessions_between(20_028, 20_028), Some(1));
        let traded = (20_020..=20_035)
            .filter(|day| {
                matches!(
                    kind_of(*day),
                    DayKind::Open(_) | DayKind::OpenLengthUnmeasured
                )
            })
            .count();
        assert_eq!(traded, 12, "the premise: twelve traded days");
        assert_eq!(sessions_between(20_020, 20_035), Some(12));
    }

    /// **THE COST NOTE ON `kind_of` NAMES BOTH WALKS.** o1api-39, D-1203.
    ///
    /// The note described one walk, of [`IRREGULAR`], while the body walked
    /// [`LENGTH_UNMEASURED`] first. The lengths are pinned by a compile-time
    /// assertion; this pins the prose that states them. Every day each table
    /// names is also answered by its own walk, so neither walk is dead.
    #[test]
    fn the_cost_note_names_both_table_walks() {
        let source = include_str!("calendar.rs");
        let signature = format!("{}{}", "pub fn kind_", "of(epoch_day: i64)");
        let at = source.find(&signature).expect("kind_of exists");
        let before = source.get(..at).expect("a char boundary");
        let doc_start = before
            .rfind("/// What `epoch_day` was")
            .expect("kind_of's doc");
        let doc = before.get(doc_start..).expect("a char boundary");
        for needle in [
            "five-element [`LENGTH_UNMEASURED`]",
            "four-element [`IRREGULAR`]",
            "at most nine comparisons",
        ] {
            assert!(doc.contains(needle), "kind_of's cost note lacks {needle:?}");
        }
        assert_eq!(LENGTH_UNMEASURED.len() + IRREGULAR.len(), 9);
        for day in LENGTH_UNMEASURED {
            assert_eq!(kind_of(day), DayKind::OpenLengthUnmeasured, "day {day}");
        }
        for (day, session) in IRREGULAR {
            assert_eq!(kind_of(day), DayKind::Open(session), "day {day}");
        }
    }

    /// **THE ARITHMETIC THAT MAKES THIS A MEASUREMENT AND NOT A LIST.**
    ///
    /// 1,765 weekdays, minus 92 days nothing traded, plus 8 weekend days the
    /// instruments did trade, is 1,681 — the count of `1day` sessions on
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
                // BOTH ARE DAYS THE EXCHANGE TRADED, which is what the
                // reconciliation counts. One has a measured session, the other
                // is a pre-2025 Muhurat whose minute series is absent — but a
                // `1day` bar exists for it either way, and this arithmetic is
                // against the daily bars.
                DayKind::Open(_) | DayKind::OpenLengthUnmeasured => {
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

        assert_eq!(weekday_total, 1_765, "weekdays in 2019-12-02..=2026-09-04");
        assert_eq!(
            closed, 92,
            "weekdays on which nothing traded — NSE holidays"
        );
        assert_eq!(weekend_sessions, 8, "Budget Saturdays, Muhurat, DR tests");
        assert_eq!(
            open, 1_681,
            "and the total is exactly the number of 1day bars the store holds"
        );
        assert_eq!(
            weekday_total - closed + weekend_sessions,
            open,
            "1765 - 92 + 8 = 1681, or this calendar is not the one the bars describe"
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

    #[test]
    fn bounded_extension_has_exactly_ten_open_and_four_closed_dates() {
        let opens = [
            20_689, 20_690, 20_691, 20_692, 20_693, 20_696, 20_697, 20_698, 20_699, 20_700,
        ];
        let closed = [20_687, 20_688, 20_694, 20_695];
        for day in 20_687..=20_700 {
            if opens.contains(&day) {
                assert_eq!(kind_of(day), DayKind::Open(Session::full()));
                assert_eq!(expected_bars(day), Some(375));
            } else {
                assert!(closed.contains(&day));
                assert_eq!(kind_of(day), DayKind::Closed);
                assert_eq!(expected_bars(day), Some(0));
            }
        }
        assert_eq!(sessions_between(20_687, 20_700), Some(10));
        assert_eq!(kind_of(20_686), DayKind::Open(Session::full()));
        for day in [FIRST_DAY - 1, 20_701, 20_702, 20_703, 21_000] {
            assert_eq!(kind_of(day), DayKind::Unmeasured);
            assert_eq!(expected_bars(day), None);
        }
        assert_eq!(sessions_between(20_700, 20_701), None);
    }

    #[test]
    fn extension_preserves_every_original_bit_and_exception() {
        // Fingerprint measured from all 307 baseline bytes before this edit.
        let fingerprint = TRADED[..307]
            .iter()
            .fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
                (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
            });
        assert_eq!(fingerprint, 0x25a6_d5dd_6f29_f488);
        assert_eq!(&TRADED[306..], &[0x7C, 0x3E, 0x1F]);
        for day in FIRST_DAY..=20_686 {
            let offset = usize::try_from(day - FIRST_DAY).expect("baseline offset");
            let was_open =
                TRADED.get(offset / 8).expect("baseline byte") & (1 << (offset % 8)) != 0;
            let expected = if LENGTH_UNMEASURED.contains(&day) {
                DayKind::OpenLengthUnmeasured
            } else if let Some((_, session)) = IRREGULAR.iter().find(|(d, _)| *d == day) {
                DayKind::Open(*session)
            } else if was_open {
                DayKind::Open(Session::full())
            } else {
                DayKind::Closed
            };
            assert_eq!(kind_of(day), expected, "baseline day {day}");
        }
    }

    #[test]
    fn extension_matches_both_measured_index_minute_grids() {
        // Compact measured timestamp summaries from the four Aug/Sep response
        // files, independently checked on 2026-09-06. No prices or private file
        // access in this fixture. Both indices had exactly the same grids;
        // the source walk checked EVERY stamp, uniqueness and 60-second spacing.
        let measured = [
            (20_689, 375, 33_300, 55_740),
            (20_690, 375, 33_300, 55_740),
            (20_691, 375, 33_300, 55_740),
            (20_692, 375, 33_300, 55_740),
            (20_693, 375, 33_300, 55_740),
            (20_696, 375, 33_300, 55_740),
            (20_697, 375, 33_300, 55_740),
            (20_698, 375, 33_300, 55_740),
            (20_699, 375, 33_300, 55_740),
            (20_700, 375, 33_300, 55_740),
        ];
        for index in ["NIFTY", "BANKNIFTY"] {
            let mut total = 0;
            for (day, count, first, last) in measured {
                let DayKind::Open(session) = kind_of(day) else {
                    panic!("{index} {day}")
                };
                let scheduled: Vec<_> = (0..86_400_u32)
                    .step_by(60)
                    .filter(|second| {
                        session.expects(u16::try_from(second / 60).expect("minute of day"))
                    })
                    .collect();
                assert_eq!(scheduled.len(), count, "{index} {day}");
                assert_eq!(scheduled.first(), Some(&first));
                assert_eq!(scheduled.last(), Some(&last));
                assert!(
                    scheduled
                        .windows(2)
                        .all(|pair| matches!(pair, [first, second] if second - first == 60))
                );
                total += count;
            }
            assert_eq!(total, 3_750, "{index}");
        }
    }

    /// **THE FOUR IRREGULAR SESSIONS USE THEIR RECORDED EVIDENCE LANE.**
    ///
    /// Three bar counts were read off stored timestamps. The outage day's
    /// exchange windows are pinned to SEBI's primary record because a stored
    /// index prefix cannot define the market timetable. A calendar that got
    /// either lane wrong would understate a session or report a special session
    /// as hundreds of false gaps.
    #[test]
    fn every_irregular_session_holds_the_bars_its_evidence_proves() {
        // 2021-02-24 — 09:15–11:39 then 15:45–16:59.
        assert_eq!(expected_bars(SYSTEMS_OUTAGE_DAY), Some(220));
        let DayKind::Open(outage) = kind_of(SYSTEMS_OUTAGE_DAY) else {
            panic!("2021-02-24 traded");
        };
        assert_eq!(outage.count, 2);
        assert!(outage.expects(11 * 60 + 39), "last minute before halt");
        assert!(!outage.expects(11 * 60 + 40), "halt starts at 11:40");
        assert!(!outage.expects(15 * 60 + 44), "pre-open is not a bar");
        assert!(outage.expects(15 * 60 + 45), "normal trading resumed");
        assert!(outage.expects(16 * 60 + 59), "last minute before 17:00");
        assert!(!outage.expects(17 * 60), "17:00 is the exclusive close");
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

    /// **THE FIVE MUHURAT SESSIONS WITH A DAILY BAR AND NO MINUTE SERIES.**
    ///
    /// Found by an audit that had already reported "28 minutes missing" and was
    /// wrong: the scan grouped by days that HAD minute bars, so a day with none
    /// at all was invisible to it. The store held 1,671 days of daily bars and
    /// **1,666** of minute bars, and this is the difference — every Diwali
    /// Muhurat before 2025. (Re-measured 2026-10-11: **1,631** minute days,
    /// because 35 days of landing holes are short as well; the module note.)
    ///
    /// Getting this wrong in either direction is expensive. As [`DayKind::Open`]
    /// with a full session they would claim 375 owed bars each and report 1,875
    /// losses that were never offered. As [`DayKind::Closed`] they would deny a
    /// session the daily bar proves happened. The length is withheld because the
    /// 2025 Muhurat measured 60 minutes and "probably the same" is the invention
    /// §3 rule 1 bans.
    #[test]
    fn a_muhurat_with_no_minute_series_owes_an_unknown_number_and_not_375() {
        for (day, label) in [
            (18_580_i64, "2020-11-14"),
            (18_935, "2021-11-04"),
            (19_289, "2022-10-24"),
            (19_673, "2023-11-12"),
            (20_028, "2024-11-01"),
        ] {
            assert_eq!(
                kind_of(day),
                DayKind::OpenLengthUnmeasured,
                "{label} has a 1day bar and no minute bar"
            );
            assert_eq!(
                expected_bars(day),
                None,
                "{label} owes an UNKNOWN number of bars — not 375, and not 0"
            );
        }

        // AND THE ONE MUHURAT THAT IS IN THE MINUTE SERIES IS NOT ONE OF THEM.
        // 2025-10-21 was measured at sixty bars, so it is an ordinary irregular
        // session and this list must not swallow it.
        assert_eq!(expected_bars(20_382), Some(60), "2025-10-21 IS measured");
    }

    /// **A DERIVED CALENDAR NEEDS NO TABLE, AND EXTENDS ITSELF.**
    ///
    /// This is the whole point of [`Calendar`] beside the constants above. Every
    /// hardcoded list in this repository has drifted and each was found by
    /// measuring it against bars; this one cannot drift, because it IS the bars.
    /// A 2015 month arriving in the store widens the answer with no edit.
    #[test]
    fn a_derived_calendar_covers_exactly_the_days_the_store_proves() {
        let cal = Calendar::from_observed(&[
            // Two ordinary sessions, a day apart, with a closed day between.
            Observed::from_runs(100, &[(555, 929)]),
            Observed::from_runs(102, &[(555, 929)]),
        ]);
        assert_eq!(cal.first_day(), 100);
        assert_eq!(cal.last_day(), 102);
        assert_eq!(cal.span(), 3);
        assert_eq!(cal.sessions(), 2);

        assert_eq!(cal.expected_bars(100), Some(375));
        assert_eq!(cal.expected_bars(102), Some(375));
        // THE DAY BETWEEN IS CLOSED, not unmeasured: it is inside the span the
        // store proves, and no bar exists for it.
        assert_eq!(cal.kind_of(101), DayKind::Closed);
        assert_eq!(cal.expected_bars(101), Some(0));
        // OUTSIDE THE SPAN IS UNMEASURED, not closed. The distinction is the
        // one an operator backfilling 2015 depends on.
        assert_eq!(cal.kind_of(99), DayKind::Unmeasured);
        assert_eq!(cal.kind_of(103), DayKind::Unmeasured);
        assert_eq!(cal.expected_bars(99), None);
    }

    /// **A WITHHELD STRETCH IS UNMEASURED, NEVER CLOSED, AND NEVER LOSES A
    /// SESSION.** D-1443: a stretch whose daily rung was not read cannot claim
    /// the exchange was shut on it.
    #[test]
    fn withholding_turns_only_closed_days_inside_the_span_into_unmeasured() {
        let build = || {
            Calendar::from_observed(&[
                Observed::from_runs(100, &[(555, 929)]),
                Observed::from_runs(103, &[]),
                Observed::from_runs(106, &[(555, 929)]),
            ])
        };
        let mut cal = build();
        assert_eq!(cal.kind_of(101), DayKind::Closed, "the premise");
        // Ranges reaching past both ends clamp to the span; the open days in
        // it, sized or not, are left alone.
        assert_eq!(cal.withhold_closed(i64::MIN, i64::MAX), 4);
        for day in [101, 102, 104, 105] {
            assert_eq!(cal.kind_of(day), DayKind::Unmeasured, "day {day}");
            assert_eq!(cal.expected_bars(day), None, "day {day}");
        }
        assert_eq!(cal.kind_of(100), DayKind::Open(Session::full()));
        assert_eq!(cal.kind_of(103), DayKind::OpenLengthUnmeasured);
        assert_eq!(cal.sessions(), 3);
        assert_eq!((cal.first_day(), cal.last_day()), (100, 106), "span kept");
        // A rerun changes nothing: idempotent.
        assert_eq!(cal.withhold_closed(i64::MIN, i64::MAX), 0);

        // One day, exactly; an inverted range; a range wholly outside.
        let mut one = build();
        assert_eq!(one.withhold_closed(102, 102), 1);
        assert_eq!(one.kind_of(101), DayKind::Closed);
        assert_eq!(one.kind_of(102), DayKind::Unmeasured);
        assert_eq!(one.withhold_closed(105, 104), 0);
        assert_eq!(one.withhold_closed(0, 99), 0);
        assert_eq!(one.withhold_closed(107, i64::MAX), 0);
        assert_eq!(one.kind_of(104), DayKind::Closed);

        // An empty calendar has nothing to withhold.
        let mut empty = Calendar::from_observed(&[]);
        assert_eq!(empty.withhold_closed(i64::MIN, i64::MAX), 0);
        assert_eq!(empty.span(), 0);

        // The regulator-proved outage day stays open even when withheld.
        let mut outage = Calendar::from_observed(&[
            Observed::from_runs(SYSTEMS_OUTAGE_DAY - 1, &[(555, 929)]),
            Observed::from_runs(SYSTEMS_OUTAGE_DAY + 1, &[(555, 929)]),
        ]);
        assert_eq!(outage.withhold_closed(i64::MIN, i64::MAX), 0);
        assert!(matches!(
            outage.kind_of(SYSTEMS_OUTAGE_DAY),
            DayKind::Open(_)
        ));
    }

    /// **A DAY WITH A DAILY BAR AND NO MINUTE SERIES KEEPS ITS OWN ANSWER.**
    ///
    /// The five pre-2025 Muhurat sessions arrive here with NO runs at all, and
    /// the derived calendar must reach the same conclusion the hand-built one
    /// was corrected to in D-0264 — without being told about Muhurat at all.
    /// That is the test that the derivation is doing the work rather than the
    /// table.
    #[test]
    fn a_day_the_minute_rung_never_reached_is_open_with_no_length() {
        let cal = Calendar::from_observed(&[
            Observed::from_runs(200, &[(555, 929)]),
            Observed::from_runs(201, &[]),
        ]);
        assert_eq!(cal.kind_of(201), DayKind::OpenLengthUnmeasured);
        assert_eq!(
            cal.expected_bars(201),
            None,
            "not 375, which would claim bars the vendor cannot return, and not \
             0, which would deny a session the daily bar proves happened"
        );
        assert_eq!(cal.sessions(), 2, "it still COUNTS as a session");
    }

    /// **A SHORT SESSION IS MEASURED, NEVER ASSUMED.**
    ///
    /// Muhurat 2025-10-21 traded 13:45–14:44. The derivation is handed those
    /// bounds and must report sixty bars owed — where `sessions × 375` reports
    /// 315 losses that were never offered. Nothing here knows the word Muhurat.
    #[test]
    fn a_short_session_owes_what_it_traded_and_not_a_full_day() {
        let cal = Calendar::from_observed(&[Observed::from_runs(20_382, &[(825, 884)])]);
        assert_eq!(cal.expected_bars(20_382), Some(60));
        let DayKind::Open(session) = cal.kind_of(20_382) else {
            panic!("it traded");
        };
        assert!(
            !session.expects(OPEN_MINUTE),
            "09:15 was not a bar that day"
        );
        assert!(session.expects(825), "13:45 was");
        assert!(!session.expects(885), "and 14:45 was not");
    }

    /// **AN EMPTY STORE PRODUCES A CALENDAR THAT CLAIMS NOTHING.**
    ///
    /// The state the operator was in an hour ago, after deleting every bar. A
    /// calendar that answered `Closed` for every day of an empty store would
    /// report the whole history as holidays — a confident wrong answer, which
    /// is the failure mode this type exists to remove.
    #[test]
    fn an_empty_store_yields_a_calendar_that_answers_unmeasured_everywhere() {
        let cal = Calendar::from_observed(&[]);
        assert_eq!(cal.span(), 0);
        assert_eq!(cal.sessions(), 0);
        assert_eq!(cal.kind_of(20_000), DayKind::Unmeasured);
        assert_eq!(cal.expected_bars(20_000), None);
    }

    /// **PRIMARY EVIDENCE OVERRIDES A TRUNCATED OBSERVED OUTAGE DAY.**
    ///
    /// The baked tables were themselves derived from this store on 2026-08-22,
    /// so a disagreement means the derivation is wrong — this is the bridge that
    /// lets the constants be deleted with evidence rather than with hope.
    #[test]
    fn the_derivation_uses_the_sourced_outage_and_agrees_on_other_irregular_days() {
        // The store holds only 09:15–10:08 on 2021-02-24. That is not the
        // exchange window; whether each absent index minute is a vendor gap is
        // a separate unmeasured question. The other three sessions use their
        // measured store runs, including TWO on each DR Saturday.
        let cal = Calendar::from_observed(&[
            Observed::from_runs(SYSTEMS_OUTAGE_DAY, &[(555, 608)]),
            Observed::from_runs(19_784, &[(555, 599), (690, 749)]),
            Observed::from_runs(19_861, &[(555, 599), (690, 749)]),
            Observed::from_runs(20_382, &[(825, 884)]),
        ]);

        // EVERY ONE AGREES WITH THE CANONICAL TABLE. On the outage day this
        // proves the derived calendar did not turn 166 normal-market slots into
        // a clean 54-minute exchange session merely because every stored index
        // series has the same prefix.
        //
        // An earlier draft of `Observed` carried only a first and a last minute,
        // and this assertion had to be written as a documented DISAGREEMENT:
        // 195 derived against 105 baked, because the 90-minute midday break was
        // invisible to it. Measured against the operator's whole store that
        // error was exactly 180 bars across the two Saturdays, which would have
        // reported NIFTY short by 208 where the truth is 28. Carrying the runs
        // instead of the span removes it, and this line is now an equality
        // rather than an excuse.
        for day in [SYSTEMS_OUTAGE_DAY, 19_784, 19_861, 20_382] {
            assert_eq!(
                cal.expected_bars(day),
                expected_bars(day),
                "derived and baked must agree on {day}"
            );
        }
        assert_eq!(cal.expected_bars(19_784), Some(105), "45 + 60, not 195");
        assert_eq!(
            cal.expected_bars(SYSTEMS_OUTAGE_DAY),
            Some(220),
            "145 + 75, not 54"
        );
    }

    /// **PRIMARY EVIDENCE DOES NOT DEPEND ON THE DAMAGED INPUT MENTIONING
    /// ITSELF.**
    #[test]
    fn a_missing_daily_outage_row_cannot_reclassify_the_verified_session_as_closed() {
        let cal = Calendar::from_observed(&[
            Observed::from_runs(SYSTEMS_OUTAGE_DAY - 1, &[(555, 929)]),
            Observed::from_runs(SYSTEMS_OUTAGE_DAY + 1, &[(555, 929)]),
        ]);
        assert_eq!(
            cal.expected_bars(SYSTEMS_OUTAGE_DAY),
            Some(220),
            "an absent stored daily row cannot erase a primary-sourced open day"
        );
    }
}

/// A calendar **derived from bars on disk** rather than baked into this file.
///
/// # Why this exists beside the constants above
///
/// Every hardcoded table in this repository has drifted, and each one was found
/// by measuring it against bars: the browser's holiday list had 28 dates,
/// started in September 2024 and was wrong on five of them; the Zerodha history
/// floor claimed a rolling ten years against a vendor that holds from 2015; the
/// minute expectation multiplied 375 across nine days that never traded 375.
/// Three irregular constants above were derived from the store. The fourth,
/// 2021-02-24, is fixed by SEBI's primary record because an index prefix cannot
/// define the exchange timetable on precisely the day systems failed.
///
/// This type removes the general date table. The **daily rung is normally the
/// calendar**: a `1day` bar is proof the exchange traded, and the minute runs
/// describe its windows. One primary-sourced override remains for 2021-02-24;
/// without it, three identical index prefixes would unanimously erase 166
/// normal-market slots. The answer still extends itself when earlier months are
/// pulled, while that fixed historical fact cannot drift with vendor bytes.
///
/// # Why it is not circular
///
/// Deriving "which days traded" from the rung you are validating would be: a
/// failed pull would read as a holiday. **Two independent rungs cross-check.**
/// The daily rung says which days traded; the minute rung is measured against
/// it. A day with a daily bar and no minute bars is a real gap — which is
/// exactly how the five pre-2025 Muhurat sessions were found, after an audit
/// had already called the store complete. Cross-rung agreement is not enough
/// when all stored minute series share one truncation; the fixed SEBI override
/// is the independently sourced answer for that one known case.
///
/// # Cost
///
/// O(days) to build, once, and **O(1) to query**: one compare, one subtract and
/// one index into a `Vec` whose length is the observed span. No hash, no
/// search, no allocation per lookup.
///
/// **UNVERIFIED as a measurement.** The bound is argued from the
/// shape of the code and no bench in this workspace times it.
/// `CLAUDE.md` §3 rule 6: a structural argument is not a
/// measurement, however sound it is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Calendar {
    first: i64,
    kinds: Vec<DayKind>,
    /// How many of `kinds` are open, counted once by [`Self::from_observed`]
    /// so [`Self::sessions`] answers without a walk. W1-pull1-4, D-0953.
    sessions: u32,
}

/// Calendar input for ingestion, distinct from the observed display calendar.
/// Observations carry no validated schedule provenance: neither their windows
/// nor their absent rows may override the static calendar or extend its authority.
///
/// **Its authority is [`sourced_kind_of`], not [`kind_of`] (D-3346).** The two
/// agree on every day of the measured window. Outside it, `kind_of` answers
/// `Unmeasured`, so every Zerodha day from 2018-01-01 to 2019-12-01 and from
/// 2026-09-05 was withheld at every derived rung and named UNVERIFIED by the
/// request audit, although NSE's own holiday circulars place every one of them
/// (D-2805). This type is read only by ingestion — the request-minute audit, the
/// derived-rung completeness fold (`fold::complete_minutes_with_calendar`) and
/// `cli::fold_audit`'s reference to that fold — and is hashed into no run
/// identity, so it may answer the sourced spans where `kind_of`, which
/// `cli::stored::calendar_policy_digest_v2` hashes, may not. `kind_of`, the
/// landing verdict (`session::Window::verdict`) and every stored reader are
/// unchanged.
#[derive(Debug, Clone, Copy, Default)]
pub struct Runtime<'a> {
    observed: Option<&'a Calendar>,
}

impl<'a> Runtime<'a> {
    /// Attach observations for diagnostics only. This does not attest a schedule.
    #[must_use]
    pub const fn from_observed(observed: &'a Calendar) -> Self {
        Self {
            observed: Some(observed),
        }
    }

    /// The sourced answer for `day`: [`kind_of`] inside the measured window,
    /// the D-2805 circular-sourced spans outside it, `Unmeasured` beyond them.
    /// Every static answer is preserved, including unknown session lengths;
    /// observations never change it. D-3346.
    ///
    /// # Cost
    ///
    /// [`sourced_kind_of`]: inside the measured window `kind_of`'s at most
    /// nine comparisons; in a sourced span one walk of the three-entry
    /// [`SOURCED_SPECIAL`] and one of the 38-entry [`SOURCED_HOLIDAYS`], both
    /// compile-time constants. O(1), no allocation. **UNVERIFIED as a
    /// gated measurement**: timed once in a throwaway release run
    /// (`docs/06-limits.md`, D-3346) and by no bench, so a regression would
    /// not fail a gate.
    #[must_use]
    pub fn kind_of(self, day: i64) -> DayKind {
        sourced_kind_of(day)
    }

    /// Why an unknown date cannot support a completeness claim.
    #[must_use]
    pub fn unverified_reason(self, day: i64) -> &'static str {
        let observation = self.observed.and_then(|calendar| {
            day.checked_sub(calendar.first)
                .and_then(|at| usize::try_from(at).ok())
                .and_then(|at| calendar.kinds.get(at))
        });
        match observation {
            Some(DayKind::Open(_) | DayKind::OpenLengthUnmeasured) => {
                "observed trading does not attest scheduled session hours; validated calendar provenance missing"
            }
            _ => "calendar authority missing; absent observations do not prove closure",
        }
    }
}

#[cfg(test)]
mod runtime_tests {
    use super::*;

    #[test]
    fn observations_preserve_every_static_answer_and_both_unknown_bounds() {
        let observed = Calendar::from_observed(&[
            Observed::from_runs(FIRST_DAY - 1, &[(555, 929)]),
            Observed::from_runs(LAST_DAY + 3, &[(555, 929)]),
        ]);
        let runtime = Runtime::from_observed(&observed);
        for day in FIRST_DAY - 2..=LAST_DAY + 4 {
            assert_eq!(runtime.kind_of(day), sourced_kind_of(day), "day {day}");
            assert_eq!(Runtime::default().kind_of(day), sourced_kind_of(day));
        }
        assert_eq!(runtime.kind_of(i64::MIN), DayKind::Unmeasured);
        assert_eq!(runtime.kind_of(i64::MAX), DayKind::Unmeasured);
        assert!(
            runtime
                .unverified_reason(i64::MIN)
                .contains("absent observations")
        );
        assert!(
            runtime
                .unverified_reason(i64::MAX)
                .contains("absent observations")
        );
    }

    /// **A CALENDAR ENDING ON `i64::MAX` NAMES `i64::MAX` AS ITS LAST DAY.**
    ///
    /// `last_day` computed `first.saturating_add(len).saturating_sub(1)`. The
    /// add saturated first and the subtraction then stepped back one, so a
    /// one-day calendar on `i64::MAX` reported `i64::MAX - 1`: a last day
    /// BEFORE the only day it holds, contradicting its own `kind_of`. D-1390.
    #[test]
    fn a_calendar_ending_on_the_last_i64_names_it_as_its_last_day() {
        let high = Calendar::from_observed(&[Observed::from_runs(i64::MAX, &[(555, 929)])]);
        assert_eq!(high.kind_of(i64::MAX), DayKind::Open(Session::full()));
        assert_eq!(high.first_day(), i64::MAX);
        assert_eq!(
            high.last_day(),
            i64::MAX,
            "the day it holds, not one before"
        );

        let two = Calendar::from_observed(&[
            Observed::from_runs(i64::MAX - 1, &[(555, 929)]),
            Observed::from_runs(i64::MAX, &[(555, 929)]),
        ]);
        assert_eq!(two.last_day(), i64::MAX);
        assert_eq!(two.span(), 2);

        // Unchanged away from the edge, and for the empty calendar.
        assert_eq!(
            Calendar::from_observed(&[Observed::from_runs(100, &[(555, 929)])]).last_day(),
            100
        );
        let empty = Calendar::from_observed(&[]);
        assert_eq!(empty.last_day(), empty.first_day() - 1);
    }

    /// **ANY RUN PAST [`MAX_WINDOWS`] IS DROPPED, NOT MERGED.**
    ///
    /// `Observed::from_runs` named this test as the pin on its behaviour, and
    /// no test of this name existed. Merging would reinstate the 180-bar
    /// midday-break over-count; dropping under-counts. The returned session
    /// carries no marker that a run was dropped — `count` is simply
    /// [`MAX_WINDOWS`] — so the caller that walked the day is the only place
    /// a third run is visible. D-1391.
    #[test]
    fn a_third_window_is_dropped_rather_than_merged() {
        let three = Observed::from_runs(19_784, &[(555, 599), (690, 749), (800, 809)]).session;
        assert_eq!(
            three.map(|s| usize::from(s.count)),
            Some(MAX_WINDOWS),
            "three runs are a session of MAX_WINDOWS windows"
        );
        assert_eq!(
            three.map(Session::bars),
            Some(45 + 60),
            "the third run's 10 bars are dropped"
        );
        assert_eq!(
            three.map(|s| s.expects(805)),
            Some(false),
            "the dropped run is not owed"
        );
        assert_eq!(
            three.map(|s| s.expects(650)),
            Some(false),
            "and the gap between runs is not merged in"
        );
        assert_eq!(
            Observed::from_runs(19_784, &[(555, 599), (690, 749)]).session,
            three,
            "identical to the two-run session: nothing marks the drop"
        );
    }

    /// **THE INGEST CALENDAR ANSWERS THE SOURCED SPANS, AND ONLY THEM (D-3346).**
    ///
    /// Inside the measured window it is `kind_of`, day for day. Outside it, it
    /// is `sourced_kind_of`: a regular weekday, a listed holiday, a weekend and
    /// a sourced Muhurat each get the circulars' answer, and a day past either
    /// sourced end is still `Unmeasured`. `kind_of` itself, which run identity
    /// hashes, still answers `Unmeasured` on every one of those days.
    #[test]
    fn the_ingest_calendar_answers_the_sourced_spans_and_kind_of_is_unchanged() {
        let runtime = Runtime::default();
        for day in FIRST_DAY..=LAST_DAY {
            assert_eq!(runtime.kind_of(day), kind_of(day), "day {day}");
        }
        for (day, want) in [
            (17_532, DayKind::Open(Session::full())), // 2018-01-01 Mon
            (17_557, DayKind::Closed),                // 2018-01-26 Republic Day
            (17_538, DayKind::Closed),                // 2018-01-07 Sun
            (17_842, DayKind::OpenLengthUnmeasured),  // 2018-11-07 Muhurat
            (18_231, DayKind::Closed),                // 2019-12-01 Sun
            (20_703, DayKind::Open(Session::full())), // 2026-09-07 Mon
            (20_710, DayKind::Closed),                // 2026-09-14 holiday
            (SOURCED_LAST_DAY + 1, DayKind::Unmeasured),
            (SOURCED_FIRST_DAY - 1, DayKind::Unmeasured),
        ] {
            assert_eq!(runtime.kind_of(day), want, "day {day}");
            assert_eq!(kind_of(day), DayKind::Unmeasured, "kind_of {day}");
        }
    }

    #[test]
    fn unknown_observed_lengths_and_empty_calendars_carry_no_authority() {
        let past = SOURCED_LAST_DAY + 1;
        let observed = Calendar::from_observed(&[Observed::from_runs(past, &[])]);
        let runtime = Runtime::from_observed(&observed);
        assert_eq!(runtime.kind_of(past), DayKind::Unmeasured);
        assert!(runtime.unverified_reason(past).contains("observed trading"));
        let empty = Calendar::from_observed(&[]);
        assert!(
            Runtime::from_observed(&empty)
                .unverified_reason(0)
                .contains("absent observations")
        );
        assert!(
            Runtime::default()
                .unverified_reason(past)
                .contains("calendar authority missing")
        );
    }
}

/// One day as the store observed it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Observed {
    /// Days since the epoch.
    pub day: i64,
    /// The contiguous runs of minute bars the caller actually found.
    ///
    /// `None` where the daily rung has a bar and the minute rung has none — the
    /// shape that becomes [`DayKind::OpenLengthUnmeasured`], and the one the
    /// five pre-2025 Muhurat sessions arrive in.
    pub session: Option<Session>,
}

impl Observed {
    /// Build a session from contiguous runs, as a caller walking a day finds
    /// them.
    ///
    /// **Runs past [`MAX_WINDOWS`] are dropped** rather than merged into the
    /// span. Merging would silently reinstate the 180-bar over-count this
    /// signature exists to remove; dropping under-counts instead, which reports
    /// fewer owed bars and therefore never invents a hole.
    ///
    /// **Nothing in the returned value marks the drop.** `count` is
    /// [`MAX_WINDOWS`] whether two runs or ten were given, and the session is
    /// identical to the two-run one, so a caller that must notice a third
    /// window has to compare `runs.len()` with [`MAX_WINDOWS`] itself — the
    /// count does not "say so". Nothing in the operator's store needs a third
    /// window, and `a_third_window_is_dropped_rather_than_merged` pins the
    /// behaviour. D-1391.
    #[must_use]
    pub fn from_runs(day: i64, runs: &[(u16, u16)]) -> Self {
        if runs.is_empty() {
            return Self { day, session: None };
        }
        let mut windows = [Window { from: 0, to: 0 }; MAX_WINDOWS];
        let mut count = 0_u8;
        for (i, &(from, to)) in runs.iter().enumerate() {
            let Some(slot) = windows.get_mut(i) else {
                break;
            };
            *slot = Window { from, to };
            count = count.saturating_add(1);
        }
        Self {
            day,
            session: Some(Session { windows, count }),
        }
    }
}

impl Calendar {
    /// Build from the days the daily rung proves traded.
    ///
    /// `observed` need not be sorted and may hold duplicates; the last entry for
    /// a day wins. Days between the first and last that are absent from
    /// `observed` are [`DayKind::Closed`] — the exchange did not trade — and
    /// days outside that span are [`DayKind::Unmeasured`].
    ///
    /// **The windows come from the bars, except where a primary regulator's
    /// record proves those bars incomplete.** The caller normally hands over
    /// the contiguous runs it found, so a disaster-recovery Saturday arrives
    /// as its two real windows. The fixed 2021-02-24 SEBI override prevents the
    /// stored 54-bar prefix from declaring itself complete.
    ///
    /// An earlier draft inferred a single window from the first and last minute.
    /// It was measured against the operator's store and over-counted by exactly
    /// **180 bars** — the two midday breaks of 2024-03-02 and 2024-05-18 — which
    /// would have reported NIFTY short by 208 where the true figure is 28. The
    /// coarse reading was documented rather than hidden, and it was still wrong
    /// enough to matter, so the caller does the walk it was already doing.
    #[must_use]
    pub fn from_observed(observed: &[Observed]) -> Self {
        let Some(first) = observed.iter().map(|o| o.day).min() else {
            return Self {
                first: 0,
                kinds: Vec::new(),
                sessions: 0,
            };
        };
        let last = observed.iter().map(|o| o.day).max().unwrap_or(first);
        // The span is bounded by the store's own extent, so this cannot be
        // unbounded; a `usize` that would not fit is a span of 5 billion days.
        //
        // CHECKED, because `last - first` overflows `i64` when the two sit near
        // opposite ends of it, and the workspace builds with `overflow-checks`
        // on in release too. A span that does not fit is treated exactly as
        // the `try_from` already treated one: the calendar holds the first day
        // alone and every other day reads `Unmeasured`, not known, never
        // `Closed`. ET-bars-candles-store-5, D-0953.
        let span = last
            .checked_sub(first)
            .and_then(|gap| usize::try_from(gap).ok())
            .unwrap_or(0)
            .saturating_add(1);
        let mut kinds = vec![DayKind::Closed; span];
        for entry in observed {
            let Some(at) = entry
                .day
                .checked_sub(first)
                .and_then(|gap| usize::try_from(gap).ok())
            else {
                continue;
            };
            let Some(slot) = kinds.get_mut(at) else {
                continue;
            };
            *slot = match entry.session {
                None => DayKind::OpenLengthUnmeasured,
                Some(session) => DayKind::Open(session),
            };
        }
        // ONE PRIMARY-SOURCED EXCEPTION. Apply it AFTER the observed fold, not
        // only while visiting a stored row. A missing DAILY row on this date is
        // damage in the evidence being audited; it cannot turn a regulator-
        // verified open session into `Closed` merely by omitting itself. SEBI
        // records a halt from 11:40, a 15-minute pre-open from 15:30, and normal
        // trading 15:45–17:00. Index-bar availability is separately refused by
        // `classify_spot_index_against`.
        if let Some(at) = SYSTEMS_OUTAGE_DAY
            .checked_sub(first)
            .and_then(|gap| usize::try_from(gap).ok())
            && let Some(slot) = kinds.get_mut(at)
        {
            *slot = DayKind::Open(OUTAGE_SESSION);
        }
        // THE ONE WALK `sessions` USED TO REPEAT ON EVERY CALL, done once here
        // and after the override, so the SEBI day counts as the open day it is.
        let sessions = kinds
            .iter()
            .filter(|k| matches!(k, DayKind::Open(_) | DayKind::OpenLengthUnmeasured))
            .count()
            .try_into()
            .unwrap_or(u32::MAX);
        Self {
            first,
            kinds,
            sessions,
        }
    }

    /// The first day this calendar knows.
    #[must_use]
    pub const fn first_day(&self) -> i64 {
        self.first
    }

    /// The last day this calendar knows.
    ///
    /// Equal to [`Self::first_day`] minus one for an empty calendar, which is
    /// the honest reading of a span that holds nothing: the range is empty
    /// rather than one day wide.
    #[must_use]
    pub fn last_day(&self) -> i64 {
        // `try_from` RATHER THAN `as`. The length is bounded by the store's own
        // extent, so it always fits; a length past `i64::MAX` would be a span of
        // nine quintillion days, and saturating there is the safe direction
        // because it can only ever make the range look SMALLER, never claim a
        // day nobody measured.
        //
        // ADD `span - 1`, NOT `span` THEN SUBTRACT ONE. The second order
        // saturated at `i64::MAX` first and then stepped back, so a one-day
        // calendar on `i64::MAX` reported its last day as `i64::MAX - 1` —
        // a day before the only day it holds. D-1390.
        let span = i64::try_from(self.kinds.len()).unwrap_or(i64::MAX);
        match span.checked_sub(1) {
            Some(tail) if tail >= 0 => self.first.saturating_add(tail),
            _ => self.first.saturating_sub(1),
        }
    }

    /// How many days it covers.
    #[must_use]
    pub fn span(&self) -> usize {
        self.kinds.len()
    }

    /// What `epoch_day` was — O(1).
    ///
    /// **UNVERIFIED as a measurement.** The bound is argued from the
    /// shape of the code and no bench in this workspace times it.
    /// `CLAUDE.md` §3 rule 6: a structural argument is not a
    /// measurement, however sound it is.
    #[must_use]
    pub fn kind_of(&self, epoch_day: i64) -> DayKind {
        // CHECKED, as `Runtime::unverified_reason` already was: a day near
        // either end of `i64` is one this calendar cannot index, which is
        // `Unmeasured`, not a panic. ET-bars-candles-store-5, D-0953.
        let Some(at) = epoch_day
            .checked_sub(self.first)
            .and_then(|gap| usize::try_from(gap).ok())
        else {
            return DayKind::Unmeasured;
        };
        self.kinds.get(at).copied().unwrap_or(DayKind::Unmeasured)
    }

    /// Bars `epoch_day` owes, or `None` where that is not known.
    #[must_use]
    pub fn expected_bars(&self, epoch_day: i64) -> Option<u16> {
        match self.kind_of(epoch_day) {
            DayKind::Open(session) => Some(session.bars()),
            DayKind::Closed => Some(0),
            DayKind::OpenLengthUnmeasured | DayKind::Unmeasured => None,
        }
    }

    /// Turn every [`DayKind::Closed`] day in `from..=to` that lies inside this
    /// calendar's span into [`DayKind::Unmeasured`], and answer how many changed.
    ///
    /// **`Closed` is a claim, and only a daily rung that was read can make it.**
    /// [`Self::from_observed`] fills every day between the first and last
    /// observed day with `Closed`, which is right only for a stretch whose daily
    /// rung was read whole. A caller that knows a stretch was NOT read (its daily
    /// file is absent, or one of its records failed its checks) withholds that
    /// stretch here, so the gap reads as "not measured" rather than as a run of
    /// holidays that never happened. D-1443.
    ///
    /// Open days are left alone: a bar is proof, wherever it came from, and the
    /// 2021-02-24 regulator override stays open. Days outside the span are
    /// already `Unmeasured`. An empty or inverted range changes nothing.
    ///
    /// # Cost
    ///
    /// O(days in `from..=to` ∩ span): one slot visited per day, no allocation.
    pub fn withhold_closed(&mut self, from: i64, to: i64) -> u32 {
        let last = self.last_day();
        let (from, to) = (from.max(self.first), to.min(last));
        let mut changed = 0_u32;
        if from > to {
            return changed;
        }
        let (Ok(start), Ok(end)) = (
            usize::try_from(from.saturating_sub(self.first)),
            usize::try_from(to.saturating_sub(self.first)),
        ) else {
            return changed;
        };
        for slot in self
            .kinds
            .iter_mut()
            .take(end.saturating_add(1))
            .skip(start)
        {
            if *slot == DayKind::Closed {
                *slot = DayKind::Unmeasured;
                changed = changed.saturating_add(1);
            }
        }
        changed
    }

    /// Days the exchange traded, counting both open kinds.
    ///
    /// O(1): the count is taken once by [`Self::from_observed`] and held. It
    /// used to filter every day of the span on each call. W1-pull1-4, D-0953.
    /// Proved by `pull::calendar::sessions_is_counted_once_at_construction_not_per_call`.
    #[must_use]
    pub const fn sessions(&self) -> u32 {
        self.sessions
    }
}

#[cfg(test)]
mod extreme_day_tests {
    use super::*;

    /// **AN EXTREME DAY IS UNMEASURED, NEVER A PANIC.**
    ///
    /// ET-bars-candles-store-5. `kind_of` subtracted `self.first` from the day
    /// unchecked, and `from_observed` did the same three times, so a day near
    /// either end of `i64` panicked with "attempt to subtract with overflow",
    /// in release too, because the workspace sets `overflow-checks = true`.
    #[test]
    fn an_extreme_day_is_unmeasured_rather_than_a_panic() {
        let from_one = Calendar::from_observed(&[Observed::from_runs(1, &[(555, 929)])]);
        assert_eq!(from_one.kind_of(i64::MIN), DayKind::Unmeasured);
        assert_eq!(from_one.kind_of(i64::MAX), DayKind::Unmeasured);
        assert_eq!(from_one.expected_bars(1), Some(375));

        let from_minus_one = Calendar::from_observed(&[Observed::from_runs(-1, &[(555, 929)])]);
        assert_eq!(from_minus_one.kind_of(i64::MAX), DayKind::Unmeasured);
        assert_eq!(from_minus_one.kind_of(i64::MIN), DayKind::Unmeasured);
        assert_eq!(from_minus_one.expected_bars(-1), Some(375));

        // BOTH ENDS AT ONCE: the span does not fit, so the calendar holds the
        // first day alone and every other day reads as not known, never as
        // `Closed`, which would be a claim about a day it cannot index.
        let both = Calendar::from_observed(&[
            Observed::from_runs(i64::MIN, &[]),
            Observed::from_runs(i64::MAX, &[]),
        ]);
        assert_eq!(both.first_day(), i64::MIN);
        assert_eq!(both.span(), 1);
        assert_eq!(both.kind_of(i64::MIN), DayKind::OpenLengthUnmeasured);
        assert_eq!(both.kind_of(i64::MAX), DayKind::Unmeasured);
        assert_eq!(both.kind_of(0), DayKind::Unmeasured);
        assert_eq!(both.sessions(), 1);

        // THE SEBI OVERRIDE DAY is past the end of this one-day calendar, so it
        // stays unmeasured rather than panicking on `SYSTEMS_OUTAGE_DAY - first`.
        let lowest = Calendar::from_observed(&[Observed::from_runs(i64::MIN, &[])]);
        assert_eq!(lowest.span(), 1);
        assert_eq!(lowest.kind_of(SYSTEMS_OUTAGE_DAY), DayKind::Unmeasured);
        assert_eq!(lowest.kind_of(i64::MIN), DayKind::OpenLengthUnmeasured);
    }

    /// **`sessions` IS COUNTED ONCE, AT CONSTRUCTION, NOT PER CALL.**
    ///
    /// W1-pull1-4. It filtered the whole `kinds` vector on every call, so its
    /// cost grew with the observed span. The proof it no longer scans: empty
    /// `kinds` after construction and the answer does not move. A scan would
    /// answer zero.
    #[test]
    fn sessions_is_counted_once_at_construction_not_per_call() {
        let mut cal = Calendar::from_observed(&[
            Observed::from_runs(100, &[(555, 929)]),
            Observed::from_runs(102, &[]),
            Observed::from_runs(104, &[(555, 929)]),
        ]);
        assert_eq!(
            cal.sessions(),
            3,
            "two measured sessions and one unmeasured"
        );
        cal.kinds.clear();
        assert_eq!(cal.sessions(), 3, "the count is held, not recomputed");

        // The SEBI override is counted too: it is written after the observed
        // fold, onto a day the store left `Closed`.
        let around_outage = Calendar::from_observed(&[
            Observed::from_runs(SYSTEMS_OUTAGE_DAY - 1, &[(555, 929)]),
            Observed::from_runs(SYSTEMS_OUTAGE_DAY + 1, &[(555, 929)]),
        ]);
        assert_eq!(around_outage.sessions(), 3);
        // And overwriting an open day with the override is not counted twice.
        let on_outage =
            Calendar::from_observed(&[Observed::from_runs(SYSTEMS_OUTAGE_DAY, &[(555, 929)])]);
        assert_eq!(on_outage.sessions(), 1);
        assert_eq!(Calendar::from_observed(&[]).sessions(), 0);
    }
}

/// The calendar of record (D-3319, D-3320): one test for every listed day,
/// and the proofs that the two older calendars keep their answers.
#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod record_tests {
    use super::*;
    use crate::vendor::Venue;

    const NIFTY_VENUES: [Venue; 3] = [Venue::NseIndex, Venue::NseCash, Venue::NseDerivatives];

    fn at(y: u16, m: u8, d: u8) -> i64 {
        i64::from(
            crate::session::Day::new(y, m, d)
                .expect("a real day")
                .days_from_epoch(),
        )
    }

    fn hm(hour: u16, minute: u16) -> u16 {
        hour * 60 + minute
    }

    fn bands(phases: &DayPhases) -> Vec<(Phase, u16, u16)> {
        phases
            .bands()
            .iter()
            .map(|band| (band.phase, band.from, band.to))
            .collect()
    }

    /// One listed day: the day, its windows as inclusive minute opens, the
    /// bars it owes and its kind.
    type Listed = (i64, Vec<(u16, u16)>, u16, SpecialKind);

    /// Every listed special day.
    fn listed_special_days() -> Vec<Listed> {
        use SpecialKind::{
            BudgetWeekend, DisasterRecoveryDrill, Muhurat, SystemsOutage, WeekendFullSession,
        };
        let evening = || vec![(hm(18, 15), hm(19, 14))];
        let drill = || vec![(555, 599), (690, 749)];
        let full = || vec![(555, 929)];
        vec![
            (at(2018, 11, 7), vec![(hm(17, 30), hm(18, 29))], 60, Muhurat),
            (at(2019, 10, 27), evening(), 60, Muhurat),
            (at(2020, 11, 14), evening(), 60, Muhurat),
            (at(2021, 11, 4), evening(), 60, Muhurat),
            (at(2022, 10, 24), evening(), 60, Muhurat),
            (at(2023, 11, 12), evening(), 60, Muhurat),
            (at(2024, 11, 1), vec![(hm(18, 0), hm(18, 59))], 60, Muhurat),
            (
                at(2025, 10, 21),
                vec![(hm(13, 45), hm(14, 44))],
                60,
                Muhurat,
            ),
            (at(2024, 3, 2), drill(), 105, DisasterRecoveryDrill),
            (at(2024, 5, 18), drill(), 105, DisasterRecoveryDrill),
            (
                at(2021, 2, 24),
                vec![(555, 699), (945, 1_019)],
                220,
                SystemsOutage,
            ),
            (at(2020, 2, 1), full(), 375, BudgetWeekend),
            (at(2025, 2, 1), full(), 375, BudgetWeekend),
            (at(2026, 2, 1), full(), 375, BudgetWeekend),
            (at(2024, 1, 20), full(), 375, WeekendFullSession),
        ]
    }

    /// **EVERY SPECIAL SESSION HOLDS ITS RECORDED WINDOW, ON THE CALENDAR OF
    /// RECORD, AND THE OLDER TWO KEEP THEIRS.** DB-G33, DB-072, DB-G55.
    ///
    /// Each Muhurat from 2018-11-07 to 2025-10-21 opens and closes where the
    /// charter's table or the GDFL ticks put it, and owes sixty bars — not
    /// `None`, which made five Zerodha minute answers and nine GDFL vendor days
    /// unadmittable, and not 375. `kind_of` and `sourced_kind_of` still answer
    /// `OpenLengthUnmeasured` (or `Unmeasured`) on them, because one is hashed
    /// into run identity and the other is the GDFL gate's and the ingest
    /// calendar's until their packages adopt this one.
    #[test]
    fn every_special_session_holds_its_recorded_window() {
        let listed = listed_special_days();
        assert_eq!(listed.len(), SPECIAL_SESSIONS.len(), "one row per entry");
        for (day, windows, owed, kind) in listed {
            let label = format!("day {day}");
            let label = label.as_str();
            let special = special_session(day).expect(label);
            assert_eq!(special.kind, kind, "{label}");
            let DayKind::Open(session) = exchange_kind_of(day) else {
                panic!("{label} is open on the calendar of record")
            };
            assert_eq!(session, special.session, "{label}");
            let held: Vec<_> = session
                .windows
                .iter()
                .take(usize::from(session.count))
                .map(|w| (w.from, w.to))
                .collect();
            assert_eq!(held, windows, "{label}");
            assert_eq!(session.bars(), owed, "{label}");
            assert!(!special.evidence.is_empty(), "{label}");
            // The two older calendars: never a different OPEN answer, and
            // unsized exactly where they always were.
            match sourced_kind_of(day) {
                DayKind::Open(older) => assert_eq!(older, session, "{label}"),
                DayKind::OpenLengthUnmeasured => assert_eq!(kind, SpecialKind::Muhurat),
                other => panic!("{label}: sourced_kind_of answered {other:?}"),
            }
        }
        // The five unsized Muhurats of the identity calendar stay unsized there.
        for day in LENGTH_UNMEASURED {
            assert_eq!(kind_of(day), DayKind::OpenLengthUnmeasured, "{day}");
            assert!(matches!(exchange_kind_of(day), DayKind::Open(_)), "{day}");
        }
        for day in SOURCED_SPECIAL {
            assert_eq!(sourced_kind_of(day), DayKind::OpenLengthUnmeasured);
            assert_eq!(kind_of(day), DayKind::Unmeasured);
        }
    }

    /// **THE CALENDAR OF RECORD DIFFERS FROM `sourced_kind_of` ON SEVEN DAYS
    /// AND NO OTHER.** Every day of both sourced spans and the measured window
    /// between, and two past each end: a window was recorded, so an unsized
    /// Muhurat became open; nothing else moved, and nothing in the range is
    /// unsized any more.
    #[test]
    fn exchange_kind_of_differs_only_where_a_window_was_recorded() {
        let mut moved = Vec::new();
        for day in SOURCED_FIRST_DAY - 2..=SOURCED_LAST_DAY + 2 {
            let (older, record) = (sourced_kind_of(day), exchange_kind_of(day));
            // The one session no circular has sized yet: the 2026-11-08
            // Muhurat, "timings to be notified" (NSE/CMTR/71775, DB-B41).
            if day == 20_765 {
                assert_eq!(record, DayKind::OpenLengthUnmeasured);
                assert_eq!(special_session(day), None);
                continue;
            }
            assert_ne!(record, DayKind::OpenLengthUnmeasured, "day {day}");
            if older != record {
                assert_eq!(older, DayKind::OpenLengthUnmeasured, "day {day}");
                assert_eq!(
                    special_session(day).map(|s| (s.kind, DayKind::Open(s.session))),
                    Some((SpecialKind::Muhurat, record)),
                    "day {day}"
                );
                moved.push(day);
            }
        }
        assert_eq!(
            moved,
            [17_842, 18_196, 18_580, 18_935, 19_289, 19_673, 20_028]
        );
        assert_eq!(exchange_kind_of(i64::MIN), DayKind::Unmeasured);
        assert_eq!(exchange_kind_of(i64::MAX), DayKind::Unmeasured);
    }

    /// The table is sorted, every day traded on both older calendars' own
    /// evidence, and each window `IRREGULAR` names is the same constant.
    #[test]
    fn the_special_table_is_sorted_and_shares_irregulars_windows() {
        assert!(SPECIAL_SESSIONS.windows(2).all(|w| match w {
            [a, b] => a.day < b.day,
            _ => false,
        }));
        for special in SPECIAL_SESSIONS {
            assert!(
                matches!(
                    sourced_kind_of(special.day),
                    DayKind::Open(_) | DayKind::OpenLengthUnmeasured
                ),
                "{}",
                special.day
            );
        }
        for (day, session) in IRREGULAR {
            assert_eq!(special_session(day).map(|s| s.session), Some(session));
        }
        assert_eq!(
            special_session(at(2020, 3, 13)),
            None,
            "a halt day is no special session"
        );
    }

    /// **EVERY WEEKEND SESSION IS A RECORDED SPECIAL SESSION.** The charter
    /// counts eight weekend sessions in the measured window; the sourced spans
    /// add the Sunday Muhurat of 2019-10-27. A weekend that traded and is not
    /// in the table would be a session nobody labelled.
    #[test]
    fn every_weekend_session_is_a_recorded_special_session() {
        let weekend_sessions: Vec<_> = (SOURCED_FIRST_DAY..=SOURCED_LAST_DAY)
            .filter(|day| weekday_of(*day).is_weekend())
            .filter(|day| matches!(exchange_kind_of(*day), DayKind::Open(_)))
            .collect();
        assert_eq!(weekend_sessions.len(), 9);
        for day in weekend_sessions {
            assert!(special_session(day).is_some(), "{day}");
        }
    }

    /// **THE MARCH 2020 CIRCUIT BREAKERS ARE SUSPECTED HALTS, THE OUTAGE A
    /// SOURCED ONE, AND NO HALTED MINUTE IS OWED.** DB-028, DB-G47.
    ///
    /// 2020-03-13: continuous to the 09:21 minute, halted 09:22–10:05, the
    /// reopening auction 10:06–10:20, continuous from 10:21 — 316 owed
    /// minutes, the count Zerodha's RELIANCE 1min holds that day (read on a
    /// shadow of the operator's store, 2026-10-09), where `/calendar.json`
    /// owed 375. 2020-03-23: halted 09:59–10:41, reopening 10:42–10:56, 317
    /// owed. A derivative has no reopening auction: its whole halt is halted.
    #[test]
    fn the_march_2020_halts_are_suspected_and_the_outage_is_sourced() {
        let march_13 = halt_on(at(2020, 3, 13)).expect("2020-03-13 halt");
        assert_eq!(
            march_13.halted,
            Window {
                from: hm(9, 22),
                to: hm(10, 20)
            }
        );
        assert_eq!(march_13.halted.bars(), 59);
        assert_eq!(
            march_13.reopening,
            Some(Window {
                from: hm(10, 6),
                to: hm(10, 20)
            })
        );
        assert!(march_13.evidence.is_suspected());
        assert!(march_13.evidence.text().contains("UNVERIFIED"));

        let march_23 = halt_on(at(2020, 3, 23)).expect("2020-03-23 halt");
        assert_eq!(
            march_23.halted,
            Window {
                from: hm(9, 59),
                to: hm(10, 56)
            }
        );
        assert_eq!(march_23.halted.bars(), 58);
        assert_eq!(
            march_23.reopening,
            Some(Window {
                from: hm(10, 42),
                to: hm(10, 56)
            })
        );
        assert!(march_23.evidence.is_suspected());

        let outage = halt_on(SYSTEMS_OUTAGE_DAY).expect("2021-02-24 halt");
        assert!(!outage.evidence.is_suspected());
        assert!(matches!(outage.evidence, HaltEvidence::Sourced(_)));
        assert!(outage.evidence.text().contains("SEBI"));
        assert_eq!(
            outage.halted,
            Window {
                from: hm(11, 40),
                to: hm(15, 44)
            }
        );

        assert_eq!(halt_on(at(2020, 3, 12)), None);
        assert_eq!(
            halt_on(at(2022, 3, 7)),
            None,
            "scattered holes are not a halt"
        );

        for venue in NIFTY_VENUES {
            // Cash and the index reopen by call auction; a derivative does not.
            let (auction, halted_13, halted_23) = if venue == Venue::NseDerivatives {
                (Phase::Halted, 59, 58)
            } else {
                (Phase::ReopeningAuction, 44, 43)
            };
            let day = at(2020, 3, 13);
            let phase = |m| phase_at(venue, day, m, CasEligibility::Unknown);
            assert_eq!(phase(hm(9, 21)), Phase::Continuous, "{venue}");
            assert_eq!(phase(hm(9, 22)), Phase::Halted, "{venue}");
            assert_eq!(phase(hm(10, 5)), Phase::Halted, "{venue}");
            assert_eq!(phase(hm(10, 6)), auction, "{venue}");
            assert_eq!(phase(hm(10, 20)), auction, "{venue}");
            assert_eq!(phase(hm(10, 21)), Phase::Continuous, "{venue}");
            let thirteenth = phases_of(venue, day, CasEligibility::Unknown);
            assert_eq!(thirteenth.minutes(Phase::Continuous), 316, "{venue}");
            assert_eq!(thirteenth.minutes(Phase::Halted), halted_13, "{venue}");
            assert_eq!(
                thirteenth.minutes(Phase::ReopeningAuction),
                59 - halted_13,
                "{venue}"
            );
            let twenty_third = phases_of(venue, at(2020, 3, 23), CasEligibility::Unknown);
            assert_eq!(twenty_third.minutes(Phase::Continuous), 317, "{venue}");
            assert_eq!(twenty_third.minutes(Phase::Halted), halted_23, "{venue}");
            assert_eq!(
                twenty_third.phase_at(hm(10, 42)),
                auction,
                "{venue}: 2020-03-23's auction"
            );
        }
        // The halt changes no calendar answer: the day is still a full session.
        assert_eq!(
            exchange_kind_of(at(2020, 3, 13)),
            DayKind::Open(Session::full())
        );
        assert_eq!(kind_of(at(2020, 3, 13)), DayKind::Open(Session::full()));
    }

    /// **EACH HALT NAMES THE SECONDS EACH SEGMENT'S ONE-SECOND SERIES SHADES**,
    /// as the GDFL day files measure them (charter, circuit-breaker boundary,
    /// 2026-10-09). On 2020-03-13 the shares stop after 09:21:08, the index
    /// after 09:21:09 and the index options after 09:21:44 — one shared
    /// second would shade 35 seconds of NIFTY option trades. Every segment's
    /// seconds hold exactly the same whole minutes, `halted`, and every
    /// reopening auction ends where its halt does.
    #[test]
    fn each_halt_names_its_seconds_and_whole_minutes() {
        let second = |h: u32, m: u32, s: u32| h * 3_600 + m * 60 + s;
        let span = |from, resume| HaltSeconds { from, resume };
        assert_eq!(
            HALTS.map(|h| (h.cash, h.index, h.derivatives)),
            [
                (
                    span(second(9, 21, 9), second(10, 21, 0)),
                    span(second(9, 21, 10), second(10, 21, 1)),
                    span(second(9, 21, 45), second(10, 21, 0)),
                ),
                (
                    span(second(9, 58, 8), second(10, 57, 0)),
                    span(second(9, 58, 9), second(10, 57, 1)),
                    span(second(9, 58, 1), second(10, 57, 0)),
                ),
                (
                    span(second(11, 40, 0), second(15, 45, 0)),
                    span(second(11, 40, 0), second(15, 45, 0)),
                    span(second(11, 40, 0), second(15, 45, 0)),
                ),
            ]
        );
        for halt in HALTS {
            for venue in NIFTY_VENUES {
                let seconds = halt.seconds(venue);
                assert!(seconds.from < seconds.resume, "{} {venue}", halt.day);
                assert_eq!(
                    u32::from(halt.halted.from),
                    seconds.from.div_ceil(60),
                    "{} {venue}",
                    halt.day
                );
                assert_eq!(
                    u32::from(halt.halted.to) + 1,
                    seconds.resume / 60,
                    "{} {venue}",
                    halt.day
                );
            }
            let reopening = halt
                .reopening
                .expect("each recorded halt reopened by auction");
            assert_eq!(reopening.to, halt.halted.to);
            assert!(reopening.from > halt.halted.from);
        }
        // The segment each answer belongs to, and not another's.
        let march_13 = halt_on(at(2020, 3, 13)).expect("2020-03-13 halt");
        assert_eq!(march_13.seconds(Venue::NseCash), march_13.cash);
        assert_eq!(march_13.seconds(Venue::NseIndex), march_13.index);
        assert_eq!(
            march_13.seconds(Venue::NseDerivatives),
            march_13.derivatives
        );
        assert_ne!(march_13.cash, march_13.derivatives);
        assert_ne!(march_13.index, march_13.derivatives);
        assert_ne!(march_13.cash, march_13.index);
    }

    /// **A HALT IS LABELLED TO THE SECOND ON A ONE-SECOND SERIES** (DB-B43).
    /// The index is halted from 09:21:10 on 2020-03-13 and 09:58:09 on
    /// 2020-03-23, and resumes at 10:21:01 and 10:57:01; a minute grid called
    /// the rest of the halt-start minute and the resume second `continuous`.
    #[test]
    fn a_halt_is_labelled_to_the_second() {
        let second = |h: u32, m: u32, s: u32| h * 3_600 + m * 60 + s;
        let unknown = CasEligibility::Unknown;
        let (march_13, march_23) = (at(2020, 3, 13), at(2020, 3, 23));
        let idx = Venue::NseIndex;
        // The second before the halt is still continuous; the next is halted.
        assert_eq!(
            phase_at_second(idx, march_13, second(9, 21, 9), unknown),
            Phase::Continuous
        );
        assert_eq!(
            phase_at_second(idx, march_13, second(9, 21, 10), unknown),
            Phase::Halted
        );
        assert_eq!(
            phase_at_second(idx, march_13, second(9, 21, 59), unknown),
            Phase::Halted
        );
        // The minute grid alone called 09:21:10-09:21:59 continuous.
        assert_eq!(
            phase_at(idx, march_13, 9 * 60 + 21, unknown),
            Phase::Continuous
        );
        // The reopening auction keeps its own phase; the resume second is the index's.
        assert_eq!(
            phase_at_second(idx, march_13, second(10, 10, 0), unknown),
            Phase::ReopeningAuction
        );
        assert_eq!(
            phase_at_second(idx, march_13, second(10, 21, 0), unknown),
            Phase::Halted
        );
        assert_eq!(
            phase_at_second(idx, march_13, second(10, 21, 1), unknown),
            Phase::Continuous
        );
        // 2020-03-23: halted from 09:58:09, back at 10:57:01.
        assert_eq!(
            phase_at_second(idx, march_23, second(9, 58, 8), unknown),
            Phase::Continuous
        );
        assert_eq!(
            phase_at_second(idx, march_23, second(9, 58, 9), unknown),
            Phase::Halted
        );
        assert_eq!(
            phase_at_second(idx, march_23, second(10, 57, 0), unknown),
            Phase::Halted
        );
        assert_eq!(
            phase_at_second(idx, march_23, second(10, 57, 1), unknown),
            Phase::Continuous
        );
        // Each segment's own seconds: index options trade until 09:21:44.
        let fno = Venue::NseDerivatives;
        assert_eq!(
            phase_at_second(fno, march_13, second(9, 21, 44), unknown),
            Phase::Continuous
        );
        assert_eq!(
            phase_at_second(fno, march_13, second(9, 21, 45), unknown),
            Phase::Halted
        );
        assert_eq!(
            phase_at_second(Venue::NseCash, march_13, second(9, 21, 9), unknown),
            Phase::Halted
        );
        // A day with no halt is its minute's phase, second for second.
        let ordinary = at(2024, 1, 2);
        assert_eq!(
            phase_at_second(idx, ordinary, second(9, 21, 30), unknown),
            Phase::Continuous
        );
        assert_eq!(
            phase_at_second(idx, ordinary, second(9, 5, 0), unknown),
            Phase::PreOpen
        );
        assert_eq!(
            phase_at_second(idx, ordinary, 86_400 * 2, unknown),
            Phase::Unverified
        );
    }

    /// **A REGULAR DAY'S PHASES, BEFORE AND AFTER 2026-08-03.** DB-034,
    /// DB-G42. Before: the 09:00 pre-open for cash and the index, continuous
    /// 09:15–15:29 on all three. From 2026-08-03: an eligible share's
    /// continuous trading ends at 15:14 and 15:15–15:29 is its closing
    /// auction — 360 owed minutes, Zerodha RELIANCE's count on 2026-09-24 —
    /// an ineligible share keeps 375, an unknown flag is `Unverified` there,
    /// an index stays continuous to 15:29 whatever the flag (D-3320), and
    /// derivatives run to 15:39 (NSE/FAOP/74467).
    #[test]
    fn a_regular_days_phases_follow_the_venue_and_the_closing_auction() {
        let july_31 = at(2026, 7, 31);
        let sept_24 = at(2026, 9, 24);
        let regular = [
            (Phase::BeforeOpen, 0, 539),
            (Phase::PreOpen, 540, 554),
            (Phase::Continuous, 555, 929),
            (Phase::PostClose, 930, 1_439),
        ];
        for cas in [
            CasEligibility::Eligible,
            CasEligibility::NotEligible,
            CasEligibility::Unknown,
        ] {
            for venue in [Venue::NseIndex, Venue::NseCash] {
                assert_eq!(
                    bands(&phases_of(venue, july_31, cas)),
                    regular,
                    "{venue} {cas:?}: no auction before 2026-08-03"
                );
            }
            // NEVER AN INDEX'S AUCTION: indices stay [09:15, 15:30) and an
            // index freeze is UNVERIFIED and not modelled (D-3320).
            assert_eq!(
                bands(&phases_of(Venue::NseIndex, sept_24, cas)),
                regular,
                "index {cas:?}"
            );
        }
        assert_eq!(
            bands(&phases_of(
                Venue::NseCash,
                sept_24,
                CasEligibility::Eligible
            )),
            [
                (Phase::BeforeOpen, 0, 539),
                (Phase::PreOpen, 540, 554),
                (Phase::Continuous, 555, 914),
                (Phase::ClosingAuction, 915, 929),
                (Phase::PostClose, 930, 1_439),
            ]
        );
        assert_eq!(
            bands(&phases_of(
                Venue::NseCash,
                sept_24,
                CasEligibility::NotEligible
            )),
            regular
        );
        let unknown = phases_of(Venue::NseCash, sept_24, CasEligibility::Unknown);
        assert_eq!(unknown.minutes(Phase::Unverified), 15);
        assert_eq!(unknown.phase_at(hm(15, 15)), Phase::Unverified);
        assert_eq!(unknown.phase_at(hm(15, 14)), Phase::Continuous);
        assert_eq!(unknown.phase_at(hm(15, 30)), Phase::PostClose);
        let eligible = phases_of(Venue::NseCash, sept_24, CasEligibility::Eligible);
        assert_eq!(eligible.minutes(Phase::Continuous), 360);
        assert_eq!(eligible.minutes(Phase::ClosingAuction), 15);
        // The first day of the auction, and the last day before it.
        assert_eq!(
            phase_at(
                Venue::NseCash,
                at(2026, 8, 3),
                hm(15, 15),
                CasEligibility::Eligible
            ),
            Phase::ClosingAuction
        );
        assert_eq!(
            phase_at(
                Venue::NseCash,
                at(2026, 7, 31),
                hm(15, 15),
                CasEligibility::Eligible
            ),
            Phase::Continuous
        );
    }

    /// **A DERIVATIVES DAY HAS NO PRE-OPEN AND NO AUCTION, AND THE OPERATOR'S
    /// LAST DAY IS ANSWERED.** The charter gives derivatives no pre-open, so
    /// none is claimed; their close is the venue's dated row, 15:39 from
    /// NSE/FAOP/74467. 2026-09-30 is answered on the calendar of record
    /// (DB-G35) while the run-identity `kind_of` is unchanged.
    #[test]
    fn a_derivatives_day_has_no_auction_and_the_operators_last_day_is_answered() {
        let july_31 = at(2026, 7, 31);
        let sept_24 = at(2026, 9, 24);
        for cas in [CasEligibility::Eligible, CasEligibility::Unknown] {
            assert_eq!(
                bands(&phases_of(Venue::NseDerivatives, july_31, cas)),
                [
                    (Phase::BeforeOpen, 0, 554),
                    (Phase::Continuous, 555, 929),
                    (Phase::PostClose, 930, 1_439),
                ]
            );
            assert_eq!(
                bands(&phases_of(Venue::NseDerivatives, sept_24, cas)),
                [
                    (Phase::BeforeOpen, 0, 554),
                    (Phase::Continuous, 555, 939),
                    (Phase::PostClose, 940, 1_439),
                ]
            );
        }
        // 2026-09-30 is the operator's last day and is answered (DB-G35).
        assert_eq!(
            phases_of(Venue::NseCash, at(2026, 9, 30), CasEligibility::Eligible)
                .minutes(Phase::Continuous),
            360
        );
        assert_eq!(
            kind_of(at(2026, 9, 30)),
            DayKind::Unmeasured,
            "identity unchanged"
        );
    }

    /// **A SPECIAL SESSION'S PHASES ARE ITS WINDOWS.** The Muhurat hour is the
    /// only continuous stretch of its day; a DR drill has a break between its
    /// windows; the outage day keeps its morning pre-open, halts at 11:40,
    /// re-opens by auction at 15:30 and trades 15:45–16:59 — 220 owed.
    #[test]
    fn a_special_sessions_phases_are_its_windows() {
        for venue in NIFTY_VENUES {
            assert_eq!(
                bands(&phases_of(venue, at(2020, 11, 14), CasEligibility::Unknown)),
                [
                    (Phase::BeforeOpen, 0, hm(18, 14)),
                    (Phase::Continuous, hm(18, 15), hm(19, 14)),
                    (Phase::PostClose, hm(19, 15), 1_439),
                ],
                "{venue}"
            );
            assert_eq!(
                bands(&phases_of(venue, at(2024, 3, 2), CasEligibility::Unknown)),
                [
                    (Phase::BeforeOpen, 0, 554),
                    (Phase::Continuous, 555, 599),
                    (Phase::Break, 600, 689),
                    (Phase::Continuous, 690, 749),
                    (Phase::PostClose, 750, 1_439),
                ],
                "{venue}"
            );
            let outage = phases_of(venue, SYSTEMS_OUTAGE_DAY, CasEligibility::Unknown);
            assert_eq!(outage.minutes(Phase::Continuous), 220, "{venue}");
            assert_eq!(outage.phase_at(hm(11, 39)), Phase::Continuous, "{venue}");
            assert_eq!(outage.phase_at(hm(11, 40)), Phase::Halted, "{venue}");
            let reopening = if venue == Venue::NseDerivatives {
                Phase::Halted
            } else {
                Phase::ReopeningAuction
            };
            assert_eq!(outage.phase_at(hm(15, 30)), reopening, "{venue}");
            assert_eq!(outage.phase_at(hm(15, 44)), reopening, "{venue}");
            assert_eq!(outage.phase_at(hm(15, 45)), Phase::Continuous, "{venue}");
            assert_eq!(outage.phase_at(hm(17, 0)), Phase::PostClose, "{venue}");
        }
        let outage_cash = phases_of(Venue::NseCash, SYSTEMS_OUTAGE_DAY, CasEligibility::Unknown);
        assert_eq!(
            outage_cash.phase_at(hm(9, 0)),
            Phase::PreOpen,
            "a market day"
        );
        let outage_fno = phases_of(
            Venue::NseDerivatives,
            SYSTEMS_OUTAGE_DAY,
            CasEligibility::Unknown,
        );
        assert_eq!(outage_fno.phase_at(hm(9, 0)), Phase::BeforeOpen);
        let drill = phases_of(Venue::NseCash, at(2024, 3, 2), CasEligibility::Unknown);
        assert_eq!(
            drill.phase_at(hm(9, 0)),
            Phase::BeforeOpen,
            "a drill is no market day"
        );
    }

    /// A closed day is closed all day, an unmeasured one unverified all day,
    /// and a minute past the day is no phase at all.
    #[test]
    fn closed_and_unmeasured_days_are_one_band() {
        for venue in NIFTY_VENUES {
            assert_eq!(
                bands(&phases_of(venue, at(2026, 9, 14), CasEligibility::Eligible)),
                [(Phase::Closed, 0, 1_439)]
            );
            for day in [
                SOURCED_LAST_DAY + 1,
                SOURCED_FIRST_DAY - 1,
                i64::MIN,
                i64::MAX,
            ] {
                assert_eq!(
                    bands(&phases_of(venue, day, CasEligibility::Eligible)),
                    [(Phase::Unverified, 0, 1_439)]
                );
            }
            assert_eq!(
                phase_at(
                    venue,
                    at(2026, 9, 24),
                    MINUTES_PER_DAY,
                    CasEligibility::Eligible
                ),
                Phase::Unverified
            );
        }
        let phases = phases_of(Venue::NseCash, at(2026, 9, 24), CasEligibility::Eligible);
        assert_eq!(phases.phase_at(MINUTES_PER_DAY), Phase::Unverified);
        assert_eq!(
            phases
                .bands()
                .iter()
                .map(|band| band.minutes())
                .sum::<u16>(),
            MINUTES_PER_DAY
        );
        // The regular-day arm refuses a day no civil date carries rather
        // than read hours for it; `phase_at` never reaches it with one,
        // because `exchange_kind_of` answers `Unmeasured` first.
        for day in [-1, i64::MAX] {
            assert_eq!(
                regular_phase(Venue::NseCash, day, 600, CasEligibility::Eligible),
                Phase::Unverified
            );
        }
        assert_eq!(civil_day(-1), None);
    }

    /// **THE BANDS ARE `phase_at`, MINUTE FOR MINUTE.** All 1,440 minutes of
    /// every special session, every halt, a regular day either side of
    /// 2026-08-03, a closed and an unmeasured day, on every venue and every
    /// eligibility; the bands tile the day with no two neighbours equal.
    #[test]
    fn phases_of_agrees_with_phase_at_on_every_minute() {
        let mut days: Vec<i64> = SPECIAL_SESSIONS.iter().map(|s| s.day).collect();
        days.extend(HALTS.iter().map(|h| h.day));
        days.extend([
            at(2026, 7, 31),
            at(2026, 8, 3),
            at(2026, 9, 24),
            at(2026, 9, 14),
            at(2018, 9, 3),
            SOURCED_LAST_DAY + 1,
        ]);
        for day in days {
            for venue in NIFTY_VENUES {
                for cas in [
                    CasEligibility::Eligible,
                    CasEligibility::NotEligible,
                    CasEligibility::Unknown,
                ] {
                    let phases = phases_of(venue, day, cas);
                    let mut next = 0_u16;
                    let mut previous = None;
                    for band in phases.bands() {
                        assert_eq!(band.from, next, "{day} {venue} {cas:?}");
                        assert!(band.to >= band.from);
                        assert_ne!(Some(band.phase), previous, "{day} {venue} {cas:?}");
                        previous = Some(band.phase);
                        next = band.to + 1;
                    }
                    assert_eq!(next, MINUTES_PER_DAY, "{day} {venue} {cas:?}");
                    for minute in 0..MINUTES_PER_DAY {
                        assert_eq!(
                            phases.phase_at(minute),
                            phase_at(venue, day, minute, cas),
                            "{day} {venue} {cas:?} minute {minute}"
                        );
                    }
                }
            }
        }
    }

    /// **A STATED EXPIRY ON A CLOSED DAY SETTLES ON THE SESSION BEFORE IT.**
    /// DB-G51: every day the audit found 13,052 tickers stating, each with
    /// the session their last trades fell on, plus a Muhurat Thursday and the
    /// 2018 Diwali pair, which walks past a Muhurat-only day.
    #[test]
    fn every_stated_expiry_on_a_closed_day_settles_on_the_session_before_it() {
        let holiday = NoSettlement::Holiday { note: None };
        for (stated, settles, moved) in [
            (at(2023, 3, 30), at(2023, 3, 29), Some(holiday)),
            (at(2023, 6, 29), at(2023, 6, 28), Some(holiday)),
            (at(2024, 1, 22), at(2024, 1, 20), Some(holiday)),
            (at(2025, 2, 26), at(2025, 2, 25), Some(holiday)),
            (at(2026, 3, 26), at(2026, 3, 25), Some(holiday)),
            (at(2026, 3, 31), at(2026, 3, 30), Some(holiday)),
            (
                at(2021, 11, 4),
                at(2021, 11, 3),
                Some(NoSettlement::SpecialSessionOnly(SpecialKind::Muhurat)),
            ),
            (
                at(2018, 11, 8),
                at(2018, 11, 6),
                Some(NoSettlement::Holiday {
                    note: holiday_note(at(2018, 11, 8)),
                }),
            ),
            (at(2026, 9, 24), at(2026, 9, 24), None),
            (at(2024, 1, 20), at(2024, 1, 20), None),
        ] {
            assert_eq!(
                settlement_on_or_before(stated),
                Ok(Settlement {
                    stated,
                    settles,
                    moved
                }),
                "{stated}"
            );
        }
        assert_eq!(
            holiday_note(at(2018, 11, 8)).map(|n| n.name),
            Some("Diwali-Balipratipada")
        );
        assert_eq!(
            settlement_on_or_before(SOURCED_LAST_DAY + 1),
            Err(SettlementRefusal::Unmeasured {
                day: SOURCED_LAST_DAY + 1
            })
        );
        // 2026-12-31, the record's last day, is a Thursday session and
        // settles on itself.
        assert_eq!(
            settlement_on_or_before(SOURCED_LAST_DAY).map(|s| s.settles),
            Ok(SOURCED_LAST_DAY)
        );
        // 2026-10-04 is a Sunday and 2026-10-02 a holiday: the walk crosses
        // them to Thursday 2026-10-01.
        assert_eq!(
            settlement_on_or_before(at(2026, 10, 4)).map(|s| s.settles),
            Ok(at(2026, 10, 1))
        );
        // The walk reaches the calendar's first day and refuses past it.
        assert_eq!(
            settlement_on_or_before(SOURCED_FIRST_DAY).map(|s| s.settles),
            Ok(SOURCED_FIRST_DAY)
        );
        assert_eq!(
            settlement_on_or_before(i64::MIN),
            Err(SettlementRefusal::Unmeasured { day: i64::MIN })
        );
        assert!(
            SettlementRefusal::NoSessionWithin { stated: 5 }
                .to_string()
                .contains("no session within 7 days")
        );
        assert!(
            SettlementRefusal::Unmeasured { day: 5 }
                .to_string()
                .contains("outside")
        );
    }

    /// **EVERY KIND OF NON-SESSION IS NAMED, AND A TRADED SPECIAL SESSION IS
    /// AN EXPIRY DAY ONLY WHEN IT WAS A MARKET DAY.**
    #[test]
    fn expiry_day_names_every_kind_of_non_session() {
        assert_eq!(
            expiry_day(at(2026, 9, 26)),
            ExpiryDay::NotASession(NoSettlement::Weekend)
        );
        let ganesh = holiday_note(at(2018, 9, 13)).expect("a sourced holiday");
        assert_eq!(
            (ganesh.name, ganesh.circular),
            ("Ganesh Chaturthi", CMTR_36475)
        );
        assert!(CMTR_36475.starts_with("NSE/CMTR/36475 "));
        assert_eq!(
            expiry_day(at(2018, 9, 13)),
            ExpiryDay::NotASession(NoSettlement::Holiday { note: Some(ganesh) })
        );
        assert_eq!(
            expiry_day(at(2024, 1, 22)),
            ExpiryDay::NotASession(NoSettlement::Holiday { note: None }),
            "a measured closed weekday no recorded circular names"
        );
        for (day, kind) in [
            (at(2020, 11, 14), SpecialKind::Muhurat),
            (at(2018, 11, 7), SpecialKind::Muhurat),
            (at(2025, 10, 21), SpecialKind::Muhurat),
            (at(2024, 3, 2), SpecialKind::DisasterRecoveryDrill),
        ] {
            assert_eq!(
                expiry_day(day),
                ExpiryDay::NotASession(NoSettlement::SpecialSessionOnly(kind)),
                "{day}"
            );
        }
        for day in [
            SYSTEMS_OUTAGE_DAY,
            at(2024, 1, 20),
            at(2025, 2, 1),
            at(2020, 3, 13),
            at(2026, 9, 30),
        ] {
            assert_eq!(expiry_day(day), ExpiryDay::Session, "{day}");
        }
        assert_eq!(expiry_day(SOURCED_LAST_DAY + 1), ExpiryDay::Unmeasured);
        for (why, says) in [
            (NoSettlement::Weekend, "a weekend with no session"),
            (
                NoSettlement::Holiday { note: None },
                "no recorded circular names it",
            ),
            (
                NoSettlement::Holiday { note: Some(ganesh) },
                "a trading holiday, Ganesh Chaturthi (NSE/CMTR/36475 of 12 Dec 2017)",
            ),
            (
                NoSettlement::SpecialSessionOnly(SpecialKind::Muhurat),
                "a Muhurat session only, not a regular market day",
            ),
        ] {
            assert!(why.to_string().contains(says), "{why}");
        }
    }

    /// **A MOVED EXPIRY NAMES THE HOLIDAY IT WAS MOVED OVER.** DB-076:
    /// BANKNIFTY's Wednesday 2018-09-12 and Tuesday 2018-11-06 and the
    /// Wednesday 2019-03-20 against its Thursday weeklies, and the Wednesday
    /// 2021-11-03 before a Muhurat Thursday.
    #[test]
    fn shift_reason_names_the_day_an_expiry_moved_back_over() {
        let note = |y, m, d| NoSettlement::Holiday {
            note: holiday_note(at(y, m, d)),
        };
        for (listed, from, why) in [
            (at(2018, 9, 12), at(2018, 9, 13), note(2018, 9, 13)),
            (at(2018, 11, 6), at(2018, 11, 8), note(2018, 11, 8)),
            (at(2019, 3, 20), at(2019, 3, 21), note(2019, 3, 21)),
            (
                at(2021, 11, 3),
                at(2021, 11, 4),
                NoSettlement::SpecialSessionOnly(SpecialKind::Muhurat),
            ),
        ] {
            assert_eq!(
                shift_reason(listed, Weekday::Thursday),
                Ok(Some(Shift { from, why })),
                "{listed}"
            );
        }
        assert_eq!(
            holiday_note(at(2019, 3, 21)).map(|n| (n.name, n.circular)),
            Some(("Holi", CMTR_39612))
        );
        // On its usual weekday, or with a session between: not a shift.
        assert_eq!(shift_reason(at(2018, 9, 27), Weekday::Thursday), Ok(None));
        assert_eq!(shift_reason(at(2018, 9, 26), Weekday::Thursday), Ok(None));
        // A Friday is not explained by the Thursday after the weekend.
        assert_eq!(shift_reason(at(2018, 9, 28), Weekday::Thursday), Ok(None));
        assert_eq!(
            shift_reason(at(2024, 1, 22), Weekday::Monday),
            Err(SettlementRefusal::NotASession {
                day: at(2024, 1, 22),
                why: NoSettlement::Holiday { note: None }
            })
        );
        assert_eq!(
            shift_reason(SOURCED_LAST_DAY + 1, Weekday::Thursday),
            Err(SettlementRefusal::Unmeasured {
                day: SOURCED_LAST_DAY + 1
            })
        );
        // 2026-12-31 is the last day answered; the Friday after is not.
        assert_eq!(
            shift_reason(at(2026, 12, 31), Weekday::Friday),
            Err(SettlementRefusal::Unmeasured {
                day: SOURCED_LAST_DAY + 1
            })
        );
        assert!(
            SettlementRefusal::NotASession {
                day: 1,
                why: NoSettlement::Weekend
            }
            .to_string()
            .contains("is not a session: a weekend with no session")
        );
    }

    /// The NIFTY option expiries the GDFL census held on 9 Oct 2026, read
    /// through a shadow of the operator's store
    /// (`/store.json?feed=gdfl&encoding=census-tuples-v1`): dates only.
    const NIFTY_EXPIRIES: [(u16, u8, u8); 67] = [
        (2018, 9, 27),
        (2018, 10, 25),
        (2018, 11, 29),
        (2018, 12, 27),
        (2019, 1, 31),
        (2019, 2, 14),
        (2019, 2, 21),
        (2019, 2, 28),
        (2019, 3, 7),
        (2019, 3, 14),
        (2019, 3, 20),
        (2019, 3, 28),
        (2019, 4, 4),
        (2019, 4, 11),
        (2019, 4, 18),
        (2019, 4, 25),
        (2019, 5, 2),
        (2019, 5, 9),
        (2019, 5, 16),
        (2019, 5, 23),
        (2019, 5, 30),
        (2019, 6, 6),
        (2019, 6, 13),
        (2019, 6, 20),
        (2019, 6, 27),
        (2019, 7, 4),
        (2019, 7, 11),
        (2019, 7, 18),
        (2019, 7, 25),
        (2019, 8, 1),
        (2019, 8, 8),
        (2019, 8, 14),
        (2019, 8, 22),
        (2019, 8, 29),
        (2019, 9, 5),
        (2019, 9, 12),
        (2019, 9, 19),
        (2019, 9, 26),
        (2019, 10, 3),
        (2019, 10, 10),
        (2019, 10, 17),
        (2019, 10, 24),
        (2019, 10, 31),
        (2019, 11, 7),
        (2019, 11, 14),
        (2019, 11, 21),
        (2019, 11, 28),
        (2019, 12, 5),
        (2019, 12, 19),
        (2019, 12, 26),
        (2020, 1, 2),
        (2020, 1, 9),
        (2020, 1, 16),
        (2020, 1, 23),
        (2020, 1, 30),
        (2020, 2, 6),
        (2020, 2, 27),
        (2020, 3, 26),
        (2020, 6, 25),
        (2020, 9, 24),
        (2020, 12, 31),
        (2021, 6, 24),
        (2021, 12, 30),
        (2022, 6, 30),
        (2022, 12, 29),
        (2023, 6, 29),
        (2023, 12, 28),
    ];

    /// RELIANCE's option expiries in the same census, 9 Oct 2026.
    const RELIANCE_EXPIRIES: [(u16, u8, u8); 18] = [
        (2018, 9, 27),
        (2018, 10, 25),
        (2018, 11, 29),
        (2018, 12, 27),
        (2019, 1, 31),
        (2019, 2, 28),
        (2019, 3, 28),
        (2019, 4, 25),
        (2019, 5, 30),
        (2019, 6, 27),
        (2019, 7, 25),
        (2019, 8, 29),
        (2019, 9, 26),
        (2019, 10, 31),
        (2019, 11, 28),
        (2019, 12, 26),
        (2020, 1, 30),
        (2020, 2, 27),
    ];

    /// The shift over the sourced holiday `y-m-d`, after checking that its
    /// circular names it `name`.
    fn named(y: u16, m: u8, d: u8, name: &str, circular: &str) -> Shift {
        let holiday = at(y, m, d);
        let note = holiday_note(holiday).expect("a sourced holiday");
        assert_eq!((note.name, note.circular), (name, circular));
        Shift {
            from: holiday,
            why: NoSettlement::Holiday { note: Some(note) },
        }
    }

    /// **NIFTY FIRST: EVERY STORED NIFTY EXPIRY IS A SESSION OR NAMES THE ONE
    /// IT SETTLED ON, AND EACH ONE OFF ITS THURSDAY NAMES THE HOLIDAY IT MOVED
    /// OVER.** DB-076, DB-G51. The 67 NIFTY expiries the GDFL census held on
    /// 9 Oct 2026 (read through a shadow of the operator's store,
    /// `/store.json?feed=gdfl&encoding=census-tuples-v1`): 66 settle on the
    /// day they state, and 29JUN23 states a closed Thursday and settles on
    /// 2023-06-28. Then the same check on RELIANCE's eighteen monthlies, every
    /// one a Thursday session.
    #[test]
    fn every_stored_nifty_expiry_is_a_session_and_each_moved_one_names_its_holiday() {
        let mut moved_off_thursday = Vec::new();
        for (y, m, d) in NIFTY_EXPIRIES.into_iter().chain(RELIANCE_EXPIRIES) {
            let stated = at(y, m, d);
            let settlement = settlement_on_or_before(stated).expect("answered");
            if (y, m, d) == (2023, 6, 29) {
                assert_eq!(settlement.settles, at(2023, 6, 28));
                assert_eq!(
                    settlement.moved,
                    Some(NoSettlement::Holiday { note: None }),
                    "a measured closed Thursday no recorded circular names"
                );
                continue;
            }
            assert_eq!(
                settlement,
                Settlement {
                    stated,
                    settles: stated,
                    moved: None
                },
                "{y}-{m}-{d}"
            );
            let shift = shift_reason(stated, Weekday::Thursday).expect("a session");
            if weekday_of(stated) == Weekday::Thursday {
                assert_eq!(shift, None, "{y}-{m}-{d}");
            } else {
                moved_off_thursday.push((stated, shift.expect("moved over a holiday")));
            }
        }
        assert_eq!(
            moved_off_thursday,
            [
                (at(2019, 3, 20), named(2019, 3, 21, "Holi", CMTR_39612)),
                (
                    at(2019, 8, 14),
                    named(2019, 8, 15, "Independence Day", CMTR_39612)
                ),
            ],
            "NIFTY's two Wednesdays; RELIANCE has none"
        );
    }

    /// **ONE BANKNIFTY CHECK: EACH OF ITS SIX STORED EXPIRIES OFF A THURSDAY
    /// NAMES THE HOLIDAY IT MOVED OVER** (the same census, 9 Oct 2026),
    /// 2018-11-06 over a Muhurat-only Wednesday to the Thursday holiday after
    /// it. DB-076.
    #[test]
    fn each_banknifty_expiry_off_its_thursday_names_its_holiday() {
        for (listed, shift) in [
            (
                at(2018, 9, 12),
                named(2018, 9, 13, "Ganesh Chaturthi", CMTR_36475),
            ),
            (at(2018, 9, 19), named(2018, 9, 20, "Moharram", CMTR_36475)),
            (at(2018, 10, 17), named(2018, 10, 18, "Dasera", CMTR_36475)),
            (
                at(2018, 11, 6),
                named(2018, 11, 8, "Diwali-Balipratipada", CMTR_36475),
            ),
            (at(2019, 3, 20), named(2019, 3, 21, "Holi", CMTR_39612)),
            (
                at(2019, 8, 14),
                named(2019, 8, 15, "Independence Day", CMTR_39612),
            ),
        ] {
            assert_eq!(
                shift_reason(listed, Weekday::Thursday),
                Ok(Some(shift)),
                "{listed}"
            );
            assert_eq!(
                settlement_on_or_before(listed).map(|s| s.moved),
                Ok(None),
                "{listed}"
            );
        }
    }

    /// **NO RUN OF NON-SETTLEMENT DAYS REACHES THE WALK BOUND.** Measured over
    /// every day the calendar of record answers, so `NoSessionWithin` means a
    /// gap no calendar fact explains, never a long weekend.
    #[test]
    fn no_run_of_non_settlement_days_reaches_the_walk_bound() {
        let mut longest = 0_i64;
        let mut run = 0_i64;
        for day in SOURCED_FIRST_DAY..=SOURCED_LAST_DAY {
            match expiry_day(day) {
                ExpiryDay::Session => run = 0,
                ExpiryDay::NotASession(_) => {
                    run += 1;
                    longest = longest.max(run);
                }
                ExpiryDay::Unmeasured => panic!("day {day} is answered"),
            }
        }
        assert_eq!(longest, 4);
        assert!(longest < MAX_SETTLEMENT_WALK);
        for day in SOURCED_FIRST_DAY + 7..=SOURCED_LAST_DAY {
            assert!(settlement_on_or_before(day).is_ok(), "{day}");
        }
    }

    /// The holiday notes are [`SOURCED_HOLIDAYS`], day for day, so a name is
    /// never attached to a day the circulars do not list.
    #[test]
    fn the_holiday_notes_are_the_sourced_holidays_day_for_day() {
        assert_eq!(SOURCED_HOLIDAY_NOTES.map(|n| n.day), SOURCED_HOLIDAYS);
        for holiday in SOURCED_HOLIDAY_NOTES {
            assert!(!holiday.name.is_empty() && holiday.circular.starts_with("NSE/"));
        }
        assert_eq!(holiday_note(at(2018, 9, 12)), None);
        assert_eq!(
            holiday_note(at(2026, 9, 14)).map(|n| n.circular),
            Some(CMTR_71775)
        );
        assert_eq!(
            [CMTR_36475, CMTR_39612, CMTR_40536, MSD_42319, CMTR_71775].map(|c| c.len() > 20),
            [true; 5],
            "each circular carries its own date"
        );
    }

    /// The weekday agrees with the civil calendar and is total.
    #[test]
    fn weekday_of_is_total_and_agrees_with_the_civil_calendar() {
        assert_eq!(weekday_of(0), Weekday::Thursday);
        assert_eq!(weekday_of(-1), Weekday::Wednesday);
        assert_eq!(weekday_of(at(2024, 1, 20)), Weekday::Saturday);
        assert_eq!(weekday_of(at(2019, 10, 27)), Weekday::Sunday);
        assert_eq!(weekday_of(at(2024, 1, 22)), Weekday::Monday);
        assert_eq!(weekday_of(at(2025, 10, 21)), Weekday::Tuesday);
        assert_eq!(weekday_of(at(2018, 9, 12)), Weekday::Wednesday);
        assert_eq!(weekday_of(at(2018, 9, 14)), Weekday::Friday);
        let _ = (weekday_of(i64::MIN), weekday_of(i64::MAX));
        for day in SOURCED_FIRST_DAY..=SOURCED_LAST_DAY {
            assert_eq!(
                weekday_of(day).is_weekend(),
                (day + 3).rem_euclid(7) >= 5,
                "{day}"
            );
        }
        assert_eq!(
            [
                Weekday::Monday,
                Weekday::Tuesday,
                Weekday::Wednesday,
                Weekday::Thursday,
                Weekday::Friday,
                Weekday::Saturday,
                Weekday::Sunday
            ]
            .map(Weekday::short),
            ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"]
        );
    }

    /// One day's whole record, for the routes that serve it.
    #[test]
    fn a_day_record_gathers_the_day() {
        let muhurat = record_of(at(2018, 11, 7));
        assert_eq!(muhurat.weekday, Weekday::Wednesday);
        assert_eq!(muhurat.kind, DayKind::Open(MUHURAT_1730));
        assert_eq!(muhurat.special.map(|s| s.kind), Some(SpecialKind::Muhurat));
        assert_eq!(muhurat.holiday.map(|n| n.name), Some("Diwali-Laxmi Pujan"));
        assert_eq!(muhurat.halt, None);
        assert_eq!(
            muhurat.expiry,
            ExpiryDay::NotASession(NoSettlement::SpecialSessionOnly(SpecialKind::Muhurat))
        );
        let halted = record_of(at(2020, 3, 13));
        assert!(halted.halt.is_some_and(|h| h.evidence.is_suspected()));
        assert_eq!(halted.special, None);
        assert_eq!(halted.expiry, ExpiryDay::Session);
        let outside = record_of(SOURCED_LAST_DAY + 1);
        assert_eq!(outside.kind, DayKind::Unmeasured);
        assert_eq!(outside.expiry, ExpiryDay::Unmeasured);
    }

    /// Each kind reads differently, and only the outage, Budget and weekend
    /// full sessions are market days.
    #[test]
    fn special_kinds_have_distinct_labels_and_one_market_day_rule() {
        let kinds = [
            SpecialKind::Muhurat,
            SpecialKind::DisasterRecoveryDrill,
            SpecialKind::SystemsOutage,
            SpecialKind::BudgetWeekend,
            SpecialKind::WeekendFullSession,
        ];
        let mut labels: Vec<_> = kinds.iter().map(|k| k.label()).collect();
        labels.sort_unstable();
        labels.dedup();
        assert_eq!(labels.len(), 5);
        assert_eq!(
            kinds.map(SpecialKind::is_market_day),
            [false, false, true, true, true]
        );
    }

    /// **MEASURED p50/p99, release build, this machine (D-3319).** Every
    /// answer of the calendar of record is a fixed number of bounded walks;
    /// this times each over a mixed set of days. Run with
    /// `cargo test --release -p pull --lib -- --ignored calendar_of_record_costs --nocapture`.
    #[test]
    #[ignore = "a timing measurement: run in release by hand, numbers are printed"]
    fn calendar_of_record_costs() {
        let mut days: Vec<i64> = SPECIAL_SESSIONS.iter().map(|s| s.day).collect();
        days.extend(HALTS.iter().map(|h| h.day));
        days.extend((SOURCED_FIRST_DAY..=SOURCED_LAST_DAY).step_by(97));
        days.push(SOURCED_LAST_DAY + 1);
        measure("exchange kind of a day", &days, exchange_kind_of);
        measure("phase at one minute", &days, |d| {
            phase_at(Venue::NseCash, d, 915, CasEligibility::Eligible)
        });
        measure("phases of a whole day", &days, |d| {
            phases_of(Venue::NseCash, d, CasEligibility::Eligible)
        });
        measure("settlement on or before", &days, settlement_on_or_before);
        measure("record of a day", &days, record_of);
    }

    /// p50 and p99 of `op` over `days`, each sample the mean of sixteen
    /// calls (one call is below the clock's own cost), the WHOLE answer kept
    /// alive with `black_box` so no field of it is optimised away.
    fn measure<T>(name: &str, days: &[i64], op: impl Fn(i64) -> T) {
        use std::hint::black_box;
        use std::time::Instant;
        let mut samples = Vec::new();
        for _ in 0..200 {
            for &day in days {
                let started = Instant::now();
                for _ in 0..16 {
                    black_box(op(black_box(day)));
                }
                samples.push(started.elapsed().as_nanos() / 16);
            }
        }
        samples.sort_unstable();
        let pick = |permille: usize| {
            samples
                .get(samples.len() * permille / 1_000)
                .copied()
                .unwrap_or(u128::MAX)
        };
        let (p50, p99) = (pick(500), pick(990));
        eprintln!("{name}: p50 {p50} ns, p99 {p99} ns, n = {}", samples.len());
        assert!(p99 < 50_000, "{name}: p99 {p99} ns");
    }
}
