//! The day an option contract SETTLED, when the vendor's name states another.
//! D-3331 (WP-12b; findings DB-018, DB-G50, DB-G51).
//!
//! # The defect this closes
//!
//! A GDFL day-form name states its expiry (`NIFTY29JUN2318000CE`), and until
//! D-3331 that text was the contract's identity: "an expiry the day form
//! states on an exchange holiday is kept as stated". Two things break that.
//!
//! 1. **The vendor renames a live contract's file to a new expiry.** On
//!    2023-06-28 every one of the 189 underlyings then trading a `29JUN23`
//!    text switched to `28JUN23` (the calendar records 2023-06-29 as closed),
//!    and the vendor did the same to index contracts on eight other days from
//!    2021-10-18 to 2025-12-29, moving their expiries one to six days. Keyed by
//!    the text, one contract became two or three store series, and every piece
//!    but the last carried an expiry the contract never reached. Why each
//!    re-dating happened (an exchange holiday, an expiry-weekday change) is
//!    UNVERIFIED: no circular for any of them is recorded in
//!    `docs/00-charter.md`.
//! 2. **A stated expiry on a closed day.** MIDCPNIFTY `22JAN24` was never
//!    renamed; the calendar records 2024-01-22 as closed and the contract last
//!    traded on the Saturday session of 2024-01-20. Measured to the stated
//!    day, a bar at 15:29:59 on the 20th had 172,801 s left instead of 1 s.
//!
//! [`settle`] answers both, from one table and the calendar, in this order:
//!
//! 1. **A row of [`ALIASES`]** for the underlying and the stated day (a
//!    [`AliasKind::VendorRedated`] row, or the one measured
//!    [`AliasKind::ClosedDay`] row), else
//! 2. **the calendar**: a stated day [`crate::calendar::sourced_kind_of`]
//!    calls closed settles on the PREVIOUS OPEN SESSION
//!    ([`Settlement::ClosedDayRule`]). That is the rule the charter's NSE
//!    contract specification states — "If the last Thursday is a trading
//!    holiday, then the expiry day is the previous trading day", with the same
//!    rule for NIFTY 50 weeklies (`docs/00-charter.md`, read from the
//!    web.archive.org snapshot of 2019-08-16) — and D-3331 decision 1 adopts
//!    it. Every closed stated day the archive holds is ALSO a row, so the rule
//!    only ever answers a day the archive has not shown; where a row and the
//!    rule disagree (BANKNIFTY `26FEB25`, re-dated FORWARD to the 27th by the
//!    2025 weekday change), the row wins, because the vendor's own rename is
//!    the measurement.
//!
//! # Where [`ALIASES`] comes from
//!
//! **Measured, 2026-10-07, from the GDFL options name census**: every option
//! file NAME of every vendor day from 2018-09-03 to 2026-09-24 (1,997 days),
//! decoded by an independent reader outside the repository. No row was read
//! and no vendor row is copied here; the table holds underlyings and dates.
//! The two closed stated days with no successor were MIDCPNIFTY `22JAN24`
//! (first listed 2024-01-03, last traded 2024-01-20) and PVR `29JUN23` (last
//! traded 2023-05-11, covered by the every-underlying row).
//!
//! A row is written for an (underlying, stated expiry) when every name of it
//! stops on one vendor day, before the stated expiry, and on the NEXT vendor
//! day names of the same underlying with the same strikes and sides appear
//! under another expiry (at least half of the strikes still trading on that
//! last day reappear): the NEAREST such expiry, at most six days away. Where
//! that text was itself renamed later (`via`), the row maps to the end of the
//! chain. Every target the vendor's history reaches traded on its own day,
//! which is what "settled" means here; a target after 2026-09-24 (two NIFTY
//! rows) is the expiry the vendor states today.
//!
//! **Re-checked 2026-10-09 against the same zips** by the ignored test
//! `the_real_archive_settles_every_name_and_refuses_only_the_impossible_strikes`
//! (names only, from each day zip's central directory: 1,997 vendor days,
//! 20,825,673 file-days): the table of that day re-keyed exactly 14,835
//! distinct names, its own `contracts` total, in 276,971 file-days (14,860
//! and 277,220 with DB-B01's ten rows, by the name census); no two names of one vendor
//! day settle on one contract; no name is refused as unsettled; and the
//! closed-day rule answers no name of the archive, because every closed
//! stated day it holds is a row.
//!
//! **What the table does not claim.** Exchange circulars for these re-datings
//! are not recorded in `docs/00-charter.md`; the source of every row is the
//! vendor's own rename, measured. A recorded circular would upgrade a row's
//! standing (D-3331); one is read (below), none is recorded in the charter.
//!
//! # The texts that stopped BEFORE the rename day (DB-B01, WP-12e)
//!
//! The measuring rule above writes a row only when every name of a text
//! stops on one vendor day and reappears on the next. A thinly traded text
//! whose last trade fell earlier got no row, so past its rename each piece
//! stayed a separate contract. Ten such texts, 25 names, are rows now, each
//! dated to the rename day of ITS UNDERLYING'S REGIME CHANGE (the day the
//! underlying's other texts moved, already rows above), not to its own last
//! trade:
//!
//! * five NIFTY half-yearly texts, re-dated from 2025-08-01. These are
//!   SOURCED: NSE circular `NSE/FAOP/68747` (25 Jun 2025,
//!   <https://nsearchives.nseindia.com/content/circulars/FAOP68747.pdf>),
//!   annexure, lists `24-Jun-27` to `29-Jun-27`, `29-Jun-28` to `27-Jun-28`,
//!   `28-Dec-28` to `26-Dec-28`, `28-Jun-29` to `26-Jun-29` and `27-Dec-29`
//!   to `24-Dec-29` beside the six already rows. Read by the audit and its
//!   second agent (2026-10-10); recorded in `docs/00-charter.md`, "NSE F&O
//!   circulars read for the options lineage" (D-3358);
//! * four FINNIFTY texts of 2021-10-18 and one MIDCPNIFTY text of 2023-08-17.
//!   These are INFERRED, cause UNVERIFIED: the underlying's whole weekday
//!   moved that day (its other texts are measured rows), and the successor is
//!   the same week's new weekday. Names, last trades and reappearing strikes
//!   are in each row's comment; `FINNIFTY 18NOV21` has one name and no strike
//!   of it traded under `16NOV21`, so that row rests on the weekday move
//!   alone.
//!
//! Re-measured 2026-10-11 from the name census (names only): 5, 2, 8, 2 and
//! 1 NIFTY names; 2, 2, 1 and 1 FINNIFTY names; 1 MIDCPNIFTY name; none of
//! them traded on or after its rename day.
//!
//! # A scheme of arrangement expires contracts EARLY (DB-B03, WP-12e)
//!
//! On a demerger NSE expires every open contract of the security on the
//! cum-date and lists the same expiries again from the ex-date. The stated
//! text cannot tell the two generations apart. [`EARLY_SETTLEMENTS`] holds
//! the SOURCED cases (one: VEDL, `NSE/FAOP/73857`), and [`settle`] answers a
//! trade day before the ex-date with [`Settlement::ExpiredEarly`]: the
//! identity stays the stated expiry (three stated expiries expired on one
//! cum-date; keying them by it would file three contracts under one key),
//! and the row names the day the generation really expired, which is what a
//! days-to-expiry reader must measure to. A demerger with no recorded
//! circular is not a row; `crate::gdfl_lineage::detect` finds its generation
//! break from the names and marks it measured.
//!
//! # What is refused, by name
//!
//! * a closed stated day with no open session in the
//!   [`CLOSED_WALK_DAYS`] before it, or whose walk reaches a day the calendar
//!   has not measured: [`SettleRefusal::NoOpenSessionBefore`];
//! * an old text traded on or after the day the vendor replaced it:
//!   [`SettleRefusal::TextAfterRename`]. The vendor reusing a retired text would
//!   name a different contract;
//! * a trade day after the settled day: [`SettleRefusal::SettledBeforeTrade`].
//!
//! A stated day the calendar has not measured (beyond 2026-10-04) is kept as
//! stated and labelled [`Settlement::AsStatedUnmeasured`]: the calendar
//! cannot say it is open, and refusing every long-dated contract would refuse
//! names that are correct.
//!
//! # Cost
//!
//! O(1): one bounded walk over [`ALIASES`], whose length is a compile-time
//! constant no input can raise, and at most `1 + CLOSED_WALK_DAYS`
//! [`crate::calendar::sourced_kind_of`] calls (each a bit test or bounded
//! walks over compile-time tables). No allocation.
//!
//! **UNVERIFIED as a measurement here.** The bound is argued from the shape of
//! the code; the per-call cost on real days is printed by the ignored test
//! `the_decoders_costs_on_real_days_are_measured`. `CLAUDE.md` §3 rule 6.

use brutex_core::symbol::Symbol;

use crate::calendar::{DayKind, sourced_kind_of};
use crate::session::Day;

/// How far back, in calendar days, the previous open session of a closed
/// stated day is looked for. NSE's longest closure in the calendar's sourced
/// span is a holiday run beside a weekend; ten is a bound no real closure
/// reaches, and a walk that reaches it is refused rather than continued.
pub const CLOSED_WALK_DAYS: u32 = 10;

/// Why a row settles a stated expiry elsewhere (D-3331's `kind`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AliasKind {
    /// The stated day is a closed day and the contract settled on the open
    /// session before it: a measured row (MIDCPNIFTY `22JAN24`), or the
    /// charter's rule ([`Settlement::ClosedDayRule`]).
    ClosedDay,
    /// The vendor renamed the contract's files mid-life: on the row's
    /// `first_day` the stated text stopped and the settled text began, with
    /// the same strikes and sides.
    VendorRedated,
}

impl AliasKind {
    /// The wire word (D-3331's `kind=` field).
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ClosedDay => "closed_day",
            Self::VendorRedated => "vendor_redated",
        }
    }

    /// The word back; `None` for any other text.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        [Self::ClosedDay, Self::VendorRedated]
            .into_iter()
            .find(|kind| kind.as_str() == text)
    }
}

/// A civil date as `(year, month, day)`: the table's spelling, readable
/// against the measurement that produced it.
pub type Ymd = (u16, u8, u8);

/// One measured re-dating.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ExpiryAlias {
    /// The underlying, or `None` for every underlying (one row: 2023-06-29,
    /// which every underlying trading it left on one day).
    pub underlying: Option<&'static str>,
    /// The expiry the vendor's text states.
    pub stated: Ymd,
    /// The day the contract settled (or, for a target after the vendor's
    /// last day, the expiry the vendor states today).
    pub settled: Ymd,
    /// For a chain, the intermediate text the contract also carried
    /// (`26MAR26` -> `31MAR26` -> `30MAR26`); the row maps straight to the
    /// end, so one lookup settles.
    pub via: Option<Ymd>,
    /// D-3331's `first_day`. For a [`AliasKind::VendorRedated`] row, the
    /// first vendor day that no longer carried the stated text (the rename
    /// day); a stated text traded on or after it is refused. For a
    /// [`AliasKind::ClosedDay`] row, the first vendor day the stated text was
    /// listed.
    pub first_day: Ymd,
    /// Why.
    pub kind: AliasKind,
    /// D-3331's `contracts`: how many distinct names carried the stated text
    /// in the census (over every underlying, for the every-underlying row).
    pub contracts: u32,
}

/// Shorthand for a re-dated row.
const fn redated(
    underlying: &'static str,
    stated: Ymd,
    settled: Ymd,
    renamed_on: Ymd,
    contracts: u32,
) -> ExpiryAlias {
    ExpiryAlias {
        underlying: Some(underlying),
        stated,
        settled,
        via: None,
        first_day: renamed_on,
        kind: AliasKind::VendorRedated,
        contracts,
    }
}

/// Shorthand for a re-dated row reached through an intermediate text.
const fn chained(
    underlying: &'static str,
    stated: Ymd,
    via: Ymd,
    settled: Ymd,
    renamed_on: Ymd,
    contracts: u32,
) -> ExpiryAlias {
    ExpiryAlias {
        underlying: Some(underlying),
        stated,
        settled,
        via: Some(via),
        first_day: renamed_on,
        kind: AliasKind::VendorRedated,
        contracts,
    }
}

/// How many rows [`ALIASES`] holds: the bound of every walk over it.
pub const ALIAS_ROWS: usize = 49;

/// Every measured re-dating (module doc). Grouped by the vendor day the
/// rename happened; `contracts` is the count of distinct names carrying the
/// stated text.
pub const ALIASES: [ExpiryAlias; ALIAS_ROWS] = [
    // 2021-10-18: FINNIFTY's Thursday texts become Tuesdays.
    redated(
        "FINNIFTY",
        (2021, 10, 21),
        (2021, 10, 19),
        (2021, 10, 18),
        12,
    ),
    redated(
        "FINNIFTY",
        (2021, 10, 28),
        (2021, 10, 26),
        (2021, 10, 18),
        7,
    ),
    // 2022-12-22: `30MAR23` (a day the calendar records as closed) becomes
    // `29MAR23`.
    redated("NIFTY", (2023, 3, 30), (2023, 3, 29), (2022, 12, 22), 24),
    redated(
        "BANKNIFTY",
        (2023, 3, 30),
        (2023, 3, 29),
        (2022, 12, 22),
        18,
    ),
    // 2023-06-28: `29JUN23` (a day the calendar records as closed) becomes
    // `28JUN23`. Every underlying trading the text on 2023-06-27 (189)
    // switched on 2023-06-28; PVR's last traded on 2023-05-11, before its
    // PVRINOX names. 12,634 names.
    ExpiryAlias {
        underlying: None,
        stated: (2023, 6, 29),
        settled: (2023, 6, 28),
        via: None,
        first_day: (2023, 6, 28),
        kind: AliasKind::VendorRedated,
        contracts: 12_634,
    },
    // 2023-08-17: MIDCPNIFTY's Wednesday texts become Mondays.
    redated(
        "MIDCPNIFTY",
        (2023, 8, 23),
        (2023, 8, 21),
        (2023, 8, 17),
        40,
    ),
    redated("MIDCPNIFTY", (2023, 8, 30), (2023, 8, 28), (2023, 8, 17), 9),
    // 2023-09-04: BANKNIFTY's Thursday weekly texts become Wednesdays.
    redated("BANKNIFTY", (2023, 9, 7), (2023, 9, 6), (2023, 9, 4), 132),
    redated("BANKNIFTY", (2023, 9, 14), (2023, 9, 13), (2023, 9, 4), 107),
    redated("BANKNIFTY", (2023, 9, 21), (2023, 9, 20), (2023, 9, 4), 59),
    redated("BANKNIFTY", (2023, 10, 5), (2023, 10, 4), (2023, 9, 4), 29),
    // MIDCPNIFTY `22JAN24` (a day the calendar records as closed) was never
    // renamed; none of its 145 names traded after the Saturday session of
    // 2024-01-20, the session before the stated day. First listed
    // 2024-01-03. The rule would say the same; the row records that the
    // archive shows it.
    ExpiryAlias {
        underlying: Some("MIDCPNIFTY"),
        stated: (2024, 1, 22),
        settled: (2024, 1, 20),
        via: None,
        first_day: (2024, 1, 3),
        kind: AliasKind::ClosedDay,
        contracts: 145,
    },
    // 2024-03-01: BANKNIFTY's Thursday monthly texts become Wednesdays.
    redated("BANKNIFTY", (2024, 3, 28), (2024, 3, 27), (2024, 3, 1), 223),
    redated("BANKNIFTY", (2024, 4, 25), (2024, 4, 24), (2024, 3, 1), 142),
    redated("BANKNIFTY", (2024, 6, 27), (2024, 6, 26), (2024, 3, 1), 18),
    redated("BANKNIFTY", (2024, 9, 26), (2024, 9, 25), (2024, 3, 1), 11),
    redated("BANKNIFTY", (2024, 12, 26), (2024, 12, 24), (2024, 3, 1), 4),
    // 2025-01-02: the index monthly texts of four underlyings become
    // Thursdays.
    redated(
        "MIDCPNIFTY",
        (2025, 1, 27),
        (2025, 1, 30),
        (2025, 1, 2),
        144,
    ),
    redated("FINNIFTY", (2025, 1, 28), (2025, 1, 30), (2025, 1, 2), 133),
    redated("BANKNIFTY", (2025, 1, 29), (2025, 1, 30), (2025, 1, 2), 214),
    redated("NIFTYNXT50", (2025, 1, 31), (2025, 1, 30), (2025, 1, 2), 36),
    redated("MIDCPNIFTY", (2025, 2, 24), (2025, 2, 27), (2025, 1, 2), 14),
    redated("FINNIFTY", (2025, 2, 25), (2025, 2, 27), (2025, 1, 2), 9),
    // `26FEB25` is a day the calendar records as closed, and the contract
    // settled FORWARD: the previous-trading-day rule would have said the
    // 25th. The row wins (module doc).
    redated("BANKNIFTY", (2025, 2, 26), (2025, 2, 27), (2025, 1, 2), 177),
    redated("NIFTYNXT50", (2025, 2, 28), (2025, 2, 27), (2025, 1, 2), 1),
    redated("BANKNIFTY", (2025, 3, 26), (2025, 3, 27), (2025, 1, 2), 36),
    redated("BANKNIFTY", (2025, 6, 25), (2025, 6, 26), (2025, 1, 2), 10),
    chained(
        "BANKNIFTY",
        (2025, 9, 24),
        (2025, 9, 25),
        (2025, 9, 30),
        (2025, 1, 2),
        5,
    ),
    // 2025-08-01: NIFTY's and BANKNIFTY's Thursday (and Wednesday) texts
    // become Tuesdays.
    redated("NIFTY", (2025, 9, 25), (2025, 9, 30), (2025, 8, 1), 178),
    redated("BANKNIFTY", (2025, 9, 25), (2025, 9, 30), (2025, 8, 1), 104),
    redated("NIFTY", (2025, 12, 24), (2025, 12, 30), (2025, 8, 1), 35),
    redated(
        "BANKNIFTY",
        (2025, 12, 24),
        (2025, 12, 30),
        (2025, 8, 1),
        12,
    ),
    // `26MAR26` (closed in the calendar) became `31MAR26` on 2025-08-01, and
    // that text (closed too) became `30MAR26` on 2025-12-29.
    chained(
        "NIFTY",
        (2026, 3, 26),
        (2026, 3, 31),
        (2026, 3, 30),
        (2025, 8, 1),
        12,
    ),
    chained(
        "BANKNIFTY",
        (2026, 3, 26),
        (2026, 3, 31),
        (2026, 3, 30),
        (2025, 8, 1),
        2,
    ),
    redated("NIFTY", (2026, 6, 25), (2026, 6, 30), (2025, 8, 1), 18),
    redated("NIFTY", (2026, 12, 31), (2026, 12, 29), (2025, 8, 1), 28),
    redated("NIFTY", (2027, 12, 30), (2027, 12, 28), (2025, 8, 1), 13),
    // 2025-12-29: `31MAR26` (a day the calendar records as closed) becomes
    // `30MAR26`.
    redated("NIFTY", (2026, 3, 31), (2026, 3, 30), (2025, 12, 29), 25),
    redated(
        "BANKNIFTY",
        (2026, 3, 31),
        (2026, 3, 30),
        (2025, 12, 29),
        15,
    ),
    // ---- WP-12e: the texts that stopped before their rename day. Appended,
    // so every earlier row keeps its place.
    // DB-B01. Four FINNIFTY texts that stopped before the 2021-10-18 move
    // (last trades 2021-10-04..10-14; module doc). INFERRED from the
    // underlying's weekday move, cause UNVERIFIED. `03NOV21` is the Wednesday
    // the Thursday weekly was stated on; 1 of its 2 strikes, 2 of 2 of
    // `11NOV21` and 1 of 1 of `30DEC21` traded under the successor; `18NOV21`
    // (1 name) has none.
    redated("FINNIFTY", (2021, 11, 3), (2021, 11, 2), (2021, 10, 18), 2),
    redated("FINNIFTY", (2021, 11, 11), (2021, 11, 9), (2021, 10, 18), 2),
    redated(
        "FINNIFTY",
        (2021, 11, 18),
        (2021, 11, 16),
        (2021, 10, 18),
        1,
    ),
    redated(
        "FINNIFTY",
        (2021, 12, 30),
        (2021, 12, 28),
        (2021, 10, 18),
        1,
    ),
    // DB-B01. One name, last traded 2023-08-16, the day before the move; its
    // strike traded under `25SEP23` from 2023-08-23. INFERRED, cause
    // UNVERIFIED.
    redated("MIDCPNIFTY", (2023, 9, 27), (2023, 9, 25), (2023, 8, 17), 1),
    // DB-B01. The five half-yearly texts NSE/FAOP/68747's annexure lists
    // beside the rows above (SOURCED; module doc). None of their names traded
    // on 2025-07-31, which is why the measuring rule wrote no row: last
    // trades 2025-07-30, 2024-06-12, 2025-07-28, 2024-11-14 and 2025-06-26.
    // 4 of 5, 1 of 2, 7 of 8, 2 of 2 and 1 of 1 strikes traded under the
    // successor.
    redated("NIFTY", (2027, 6, 24), (2027, 6, 29), (2025, 8, 1), 5),
    redated("NIFTY", (2028, 6, 29), (2028, 6, 27), (2025, 8, 1), 2),
    redated("NIFTY", (2028, 12, 28), (2028, 12, 26), (2025, 8, 1), 8),
    redated("NIFTY", (2029, 6, 28), (2029, 6, 26), (2025, 8, 1), 2),
    redated("NIFTY", (2029, 12, 27), (2029, 12, 24), (2025, 8, 1), 1),
];

/// One SOURCED early settlement: a scheme of arrangement on which the
/// exchange expired every open contract of the security on the cum-date and
/// listed the same expiries again from the ex-date (module doc, DB-B03).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct EarlySettlement {
    /// The underlying.
    pub underlying: &'static str,
    /// The ex-date: the first day of the re-listed generation.
    pub ex_date: Ymd,
    /// The day every contract of the old generation expired (the cum-date).
    pub settled: Ymd,
    /// The stated expiries the circular names. A stated expiry it does not
    /// name is not answered by this row.
    pub expiries: &'static [Ymd],
    /// The circular's reference.
    pub circular: &'static str,
    /// Where it was read.
    pub url: &'static str,
}

/// Every sourced early settlement. One row: the only circular read that
/// STATES the early expiry day. `NSE/FAOP/71627` (HINDUNILVR, ex 2025-12-05)
/// lists the re-introduced expiries but not the day the old ones expired, so
/// it is not a row: UNVERIFIED, left to the measured generation break.
pub const EARLY_SETTLEMENTS: [EarlySettlement; 1] = [EarlySettlement {
    underlying: "VEDL",
    ex_date: (2026, 4, 30),
    settled: (2026, 4, 29),
    // "All existing contracts with expiry dates May 26, 2026, June 30, 2026
    // and July 28, 2026, will expire on Wednesday, April 29, 2026."
    expiries: &[(2026, 5, 26), (2026, 6, 30), (2026, 7, 28)],
    circular: "NSE/FAOP/73857",
    url: "https://nsearchives.nseindia.com/content/circulars/FAOP73857.pdf",
}];

/// The early settlement that expired the contract `underlying` states as
/// `stated`, for a file of trade day `trade`: a row naming that expiry whose
/// ex-date is after the trade day. `None` on or after the ex-date (the
/// re-listed generation settles as stated) and for every other contract.
/// O(rows × expiries), both compile-time constants.
#[must_use]
pub fn early_settlement(
    underlying: &str,
    stated: Day,
    trade: Day,
) -> Option<&'static EarlySettlement> {
    EARLY_SETTLEMENTS.iter().find(|row| {
        row.underlying == underlying
            && row.expiries.iter().any(|&ymd| is(stated, ymd))
            && day_of(row.ex_date).is_ok_and(|ex| trade.days_from_epoch() < ex.days_from_epoch())
    })
}

/// The sourced early settlement of `underlying` whose ex-date is `day`, if
/// any: the day its contracts were listed again.
#[must_use]
pub fn relisted_on(underlying: &str, day: Day) -> Option<&'static EarlySettlement> {
    EARLY_SETTLEMENTS
        .iter()
        .find(|row| row.underlying == underlying && is(day, row.ex_date))
}

/// How a contract's settled expiry was established.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Settlement {
    /// The stated day is a session in the calendar: the contract settles on
    /// the day its name states.
    AsStated,
    /// The calendar has not measured the stated day (after 2026-10-04): kept
    /// as stated, and labelled, because nothing here can say it is open.
    AsStatedUnmeasured,
    /// One measured row settles the stated text on another day; the vendor's
    /// text is the alias.
    Aliased(&'static ExpiryAlias),
    /// No row names the stated day, the calendar records it as closed, and
    /// the charter's rule settles it on the previous open session (module
    /// doc). The vendor's text is the alias.
    ClosedDayRule,
    /// A sourced scheme of arrangement expired this contract EARLY, on the
    /// row's `settled` day; the same stated expiry was listed again from the
    /// row's ex-date as another contract. [`Settled::expiry`] is still the
    /// stated day (the identity); [`Settled::last_day`] is the day it
    /// expired.
    ExpiredEarly(&'static EarlySettlement),
}

impl Settlement {
    /// The wire word.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::AsStated => "as_stated",
            Self::AsStatedUnmeasured => "as_stated_unmeasured",
            Self::Aliased(_) => "aliased",
            Self::ClosedDayRule => "closed_day_rule",
            Self::ExpiredEarly(_) => "expired_early",
        }
    }

    /// D-3331's alias `kind`, or `None` when the contract settled as stated
    /// and there is no alias to record.
    #[must_use]
    pub const fn kind(self) -> Option<AliasKind> {
        match self {
            Self::AsStated | Self::AsStatedUnmeasured | Self::ExpiredEarly(_) => None,
            Self::Aliased(row) => Some(row.kind),
            Self::ClosedDayRule => Some(AliasKind::ClosedDay),
        }
    }
}

/// A stated expiry, settled.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Settled {
    /// The day the contract settled: its identity in the store.
    pub expiry: Day,
    /// How that was established.
    pub settlement: Settlement,
}

impl Settled {
    /// The last day the contract could trade: [`Self::expiry`], except for a
    /// contract a scheme of arrangement expired early
    /// ([`Settlement::ExpiredEarly`]), whose last day is the cum-date. What a
    /// days-to-expiry reader measures to.
    ///
    /// # Errors
    ///
    /// [`SettleRefusal::RowDayInvalid`] for a table date that is not a day.
    pub fn last_day(&self) -> Result<Day, SettleRefusal> {
        match self.settlement {
            Settlement::ExpiredEarly(row) => day_of(row.settled),
            _ => Ok(self.expiry),
        }
    }
}

/// Why a stated expiry could not be settled. Each names what was read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SettleRefusal {
    /// The calendar records the stated day as closed, no row settles it, and
    /// no open session was found in the [`CLOSED_WALK_DAYS`] before it (or
    /// the walk reached a day the calendar has not measured).
    NoOpenSessionBefore {
        /// The stated day.
        stated: Day,
    },
    /// The stated text was traded on or after the day the vendor replaced it.
    TextAfterRename {
        /// The first day without the text.
        renamed_on: Day,
    },
    /// The trade day is after the settled day.
    SettledBeforeTrade {
        /// The settled day.
        settled: Day,
    },
    /// A table date is not a real day. Unreachable: the test
    /// `every_alias_row_names_real_days_in_order_and_settles_on_a_session`
    /// builds every one. Named rather than a panic or a guessed date.
    RowDayInvalid,
}

impl core::fmt::Display for SettleRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NoOpenSessionBefore { stated } => write!(
                f,
                "the stated expiry {stated} is a closed day, no measured re-dating settles it, and no open session the calendar records lies in the {CLOSED_WALK_DAYS} days before it; refused rather than settled on a guessed session"
            ),
            Self::TextAfterRename { renamed_on } => write!(
                f,
                "the stated text was traded on or after {renamed_on}, the day the vendor replaced it; a reused text names another contract"
            ),
            Self::SettledBeforeTrade { settled } => {
                write!(f, "traded after its settled expiry {settled}")
            }
            Self::RowDayInvalid => f.write_str("an alias row names a day that does not exist"),
        }
    }
}

impl core::error::Error for SettleRefusal {}

/// Whether `day` is the table date `ymd`.
const fn is(day: Day, ymd: Ymd) -> bool {
    day.year() == ymd.0 && day.month() == ymd.1 && day.day() == ymd.2
}

/// A table date as a day.
///
/// # Errors
///
/// [`SettleRefusal::RowDayInvalid`] for a date no calendar holds.
pub fn day_of(ymd: Ymd) -> Result<Day, SettleRefusal> {
    Day::new(ymd.0, ymd.1, ymd.2).map_err(|_| SettleRefusal::RowDayInvalid)
}

/// The row that settles `underlying`'s `stated` text, if any: an
/// underlying's own row before the every-underlying one. O(1): two walks
/// bounded by [`ALIAS_ROWS`].
#[must_use]
pub fn alias_of(underlying: &str, stated: Day) -> Option<&'static ExpiryAlias> {
    let own = ALIASES
        .iter()
        .find(|row| row.underlying == Some(underlying) && is(stated, row.stated));
    own.or_else(|| {
        ALIASES
            .iter()
            .find(|row| row.underlying.is_none() && is(stated, row.stated))
    })
}

/// The open session before the closed day `stated`, by the charter's rule:
/// the nearest earlier day the calendar records as open, within
/// [`CLOSED_WALK_DAYS`].
///
/// # Errors
///
/// [`SettleRefusal::NoOpenSessionBefore`] when the walk meets a day the
/// calendar has not measured, or finds no open day within the bound.
pub fn previous_open_session(stated: Day) -> Result<Day, SettleRefusal> {
    previous_open_in(stated, sourced_kind_of)
}

/// [`previous_open_session`] against the calendar `kind_of`, so a test can
/// hand it a closure no real calendar holds.
fn previous_open_in(stated: Day, kind_of: impl Fn(i64) -> DayKind) -> Result<Day, SettleRefusal> {
    let refused = SettleRefusal::NoOpenSessionBefore { stated };
    let at = stated.days_from_epoch();
    for back in 1..=CLOSED_WALK_DAYS {
        let Some(epoch) = at.checked_sub(back) else {
            return Err(refused);
        };
        match kind_of(i64::from(epoch)) {
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured => {
                return Day::from_days(epoch).map_err(|_| refused);
            }
            DayKind::Closed => {}
            DayKind::Unmeasured => return Err(refused),
        }
    }
    Err(refused)
}

/// The settled expiry of `underlying`'s contract whose text states `stated`,
/// traded on `trade` (when a trade day is known; a tenor reader passes
/// `None`).
///
/// # Errors
///
/// Every [`SettleRefusal`] (module doc). None is a fallback.
pub fn settle(underlying: &str, stated: Day, trade: Option<Day>) -> Result<Settled, SettleRefusal> {
    let settled = if let Some(row) = alias_of(underlying, stated) {
        if let (AliasKind::VendorRedated, Some(trade)) = (row.kind, trade) {
            let renamed_on = day_of(row.first_day)?;
            if trade.days_from_epoch() >= renamed_on.days_from_epoch() {
                return Err(SettleRefusal::TextAfterRename { renamed_on });
            }
        }
        Settled {
            expiry: day_of(row.settled)?,
            settlement: Settlement::Aliased(row),
        }
    } else {
        match sourced_kind_of(i64::from(stated.days_from_epoch())) {
            DayKind::Closed => Settled {
                expiry: previous_open_session(stated)?,
                settlement: Settlement::ClosedDayRule,
            },
            DayKind::Unmeasured => Settled {
                expiry: stated,
                settlement: Settlement::AsStatedUnmeasured,
            },
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured => Settled {
                expiry: stated,
                settlement: Settlement::AsStated,
            },
        }
    };
    match trade {
        Some(trade) if trade.days_from_epoch() > settled.expiry.days_from_epoch() => {
            Err(SettleRefusal::SettledBeforeTrade {
                settled: settled.expiry,
            })
        }
        // DB-B03. A file of a day before a sourced scheme's ex-date is the
        // generation that expired on the cum-date. Asked on the STATED day:
        // the circular names the expiries as the exchange stated them.
        Some(trade) => Ok(
            early_settlement(underlying, stated, trade).map_or(settled, |row| Settled {
                expiry: settled.expiry,
                settlement: Settlement::ExpiredEarly(row),
            }),
        ),
        None => Ok(settled),
    }
}

/// One D-3331 `alias` record: the vendor's stated text kept beside the
/// settled contract, in D-3317's line rules, written to
/// `<store>/imports/gdfl.lineage` by the import (WP-11b):
///
/// `alias <underlying|*> stated=<YYYY-MM-DD> settled=<YYYY-MM-DD> kind=<closed_day|vendor_redated> first_day=<YYYY-MM-DD> contracts=<n> v=1`
///
/// `*` is the every-underlying row (2023-06-29). A table row renders through
/// [`Self::of_row`]; a [`Settlement::ClosedDayRule`] the import meets builds
/// one with the first trade day and the contract count it observed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AliasRecord {
    /// The underlying, or `None` for every underlying.
    pub underlying: Option<Symbol>,
    /// The expiry the vendor's text states.
    pub stated: Day,
    /// The day the contract settled.
    pub settled: Day,
    /// Why.
    pub kind: AliasKind,
    /// D-3331's `first_day` ([`ExpiryAlias::first_day`]).
    pub first_day: Day,
    /// D-3331's `contracts`.
    pub contracts: u32,
}

impl AliasRecord {
    /// The record of one table row.
    ///
    /// # Errors
    ///
    /// [`SettleRefusal::RowDayInvalid`] for a row date that is not a day, or
    /// an underlying no `Symbol` holds; unreachable for [`ALIASES`], whose
    /// every row the tests render.
    pub fn of_row(row: &ExpiryAlias) -> Result<Self, SettleRefusal> {
        let underlying = match row.underlying {
            None => None,
            Some(text) => Some(Symbol::new(text).map_err(|_| SettleRefusal::RowDayInvalid)?),
        };
        Ok(Self {
            underlying,
            stated: day_of(row.stated)?,
            settled: day_of(row.settled)?,
            kind: row.kind,
            first_day: day_of(row.first_day)?,
            contracts: row.contracts,
        })
    }

    /// The line, no trailing newline.
    #[must_use]
    pub fn line(&self) -> String {
        format!(
            "alias {} stated={} settled={} kind={} first_day={} contracts={} v=1",
            self.underlying.as_ref().map_or("*", Symbol::as_str),
            self.stated,
            self.settled,
            self.kind.as_str(),
            self.first_day,
            self.contracts,
        )
    }

    /// The record [`Self::line`] wrote, or `None` for any other text. Never
    /// repaired.
    #[must_use]
    pub fn parse(line: &str) -> Option<Self> {
        fn value<'a>(field: &'a str, name: &str) -> Option<&'a str> {
            field.strip_prefix(name)?.strip_prefix('=')
        }
        let fields: Vec<&str> = line.split(' ').collect();
        let &[
            "alias",
            under,
            stated,
            settled,
            kind,
            first_day,
            contracts,
            "v=1",
        ] = fields.as_slice()
        else {
            return None;
        };
        let day = |field: &str, name: &str| crate::gdfl_lineage::parse_day(value(field, name)?);
        let underlying = match under {
            "*" => None,
            text => Some(Symbol::new(text).ok().filter(|s| s.as_str() == text)?),
        };
        let count = value(contracts, "contracts")?;
        let record = Self {
            underlying,
            stated: day(stated, "stated")?,
            settled: day(settled, "settled")?,
            kind: AliasKind::parse(value(kind, "kind")?)?,
            first_day: day(first_day, "first_day")?,
            contracts: (!count.is_empty() && count.bytes().all(|b| b.is_ascii_digit()))
                .then(|| count.parse().ok())
                .flatten()?,
        };
        (record.line() == line).then_some(record)
    }
}

/// Every row of [`ALIASES`] as its `alias` record, in table order: what the
/// import writes once to the lineage ledger so a reader can show the vendor's
/// text beside each settled contract.
///
/// # Errors
///
/// [`AliasRecord::of_row`]'s, unreachable for this table.
pub fn alias_records() -> Result<Vec<AliasRecord>, SettleRefusal> {
    ALIASES.iter().map(AliasRecord::of_row).collect()
}

#[cfg(test)]
#[allow(
    clippy::expect_used,
    clippy::unwrap_used,
    clippy::panic,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;

    fn d(y: u16, m: u8, dd: u8) -> Day {
        Day::new(y, m, dd).unwrap()
    }

    fn kind(day: Day) -> DayKind {
        sourced_kind_of(i64::from(day.days_from_epoch()))
    }

    /// The stated days the table's comments call closed.
    const CLOSED_STATED: [Ymd; 6] = [
        (2023, 3, 30),
        (2023, 6, 29),
        (2024, 1, 22),
        (2025, 2, 26),
        (2026, 3, 26),
        (2026, 3, 31),
    ];

    /// Every row's dates are real, the settled day is a session the calendar
    /// records (or one it has not measured), never a closed day, and it lies
    /// within six days of the stated one; a re-dated row's rename day lies
    /// before both, and a closed-day row's first listing before its
    /// settlement.
    #[test]
    fn every_alias_row_names_real_days_in_order_and_settles_on_a_session() {
        for row in &ALIASES {
            let stated = day_of(row.stated).unwrap();
            let settled = day_of(row.settled).unwrap();
            let first = day_of(row.first_day).unwrap();
            assert!(
                matches!(
                    kind(settled),
                    DayKind::Open(_) | DayKind::OpenLengthUnmeasured | DayKind::Unmeasured
                ),
                "{row:?}: settles on a closed day"
            );
            let gap = i64::from(stated.days_from_epoch()) - i64::from(settled.days_from_epoch());
            assert!(gap != 0 && gap.abs() <= 6, "{row:?}: {gap} days apart");
            assert!(first <= settled && first < stated, "{row:?}: first day");
            if let Some(via) = row.via {
                day_of(via).unwrap();
            }
            assert!(row.contracts > 0);
            if row.kind == AliasKind::ClosedDay {
                // A closed-day row is a measurement the rule agrees with.
                assert_eq!(kind(stated), DayKind::Closed, "{row:?}");
                assert_eq!(row.via, None, "{row:?}");
                assert_eq!(previous_open_session(stated), Ok(settled), "{row:?}");
            }
        }
        assert_eq!(
            ALIASES
                .iter()
                .filter(|row| row.kind == AliasKind::ClosedDay)
                .count(),
            1,
            "the one closed-day row is MIDCPNIFTY 22JAN24"
        );
        for (y, m, dd) in CLOSED_STATED {
            assert_eq!(kind(d(y, m, dd)), DayKind::Closed, "{y}-{m}-{dd}");
        }
        // Every other stated day is a session: those rows are re-datings of
        // a day the exchange did trade.
        for row in &ALIASES {
            if !CLOSED_STATED.contains(&row.stated) {
                assert_ne!(
                    kind(day_of(row.stated).unwrap()),
                    DayKind::Closed,
                    "{row:?}"
                );
            }
        }
        assert_eq!(ALIASES.len(), ALIAS_ROWS);
        assert_eq!(
            ALIASES.iter().map(|row| row.contracts).sum::<u32>(),
            14_860,
            "the census re-keys exactly this many names (14,835 measured by the every-name-stops \
             rule, and the 25 of DB-B01's ten texts that stopped before their rename day)"
        );
    }

    /// The two row builders are what the table is written with: called at
    /// run time here, they build exactly the rows the `const` holds.
    #[test]
    fn the_row_builders_build_the_rows_the_table_holds() {
        assert_eq!(
            redated("NIFTY", (2023, 3, 30), (2023, 3, 29), (2022, 12, 22), 24),
            ALIASES[2]
        );
        let chain = chained(
            "NIFTY",
            (2026, 3, 26),
            (2026, 3, 31),
            (2026, 3, 30),
            (2025, 8, 1),
            12,
        );
        assert!(ALIASES.contains(&chain));
        assert_eq!(chain.via, Some((2026, 3, 31)));
        assert_eq!(chain.kind, AliasKind::VendorRedated);
        assert_eq!(chain.first_day, (2025, 8, 1));
    }

    /// No (underlying, stated) pair has two rows, and no settled day is
    /// itself a stated text of the same underlying: every chain is resolved
    /// to its end, so one lookup settles.
    #[test]
    fn the_table_is_unique_and_every_chain_is_resolved() {
        for (i, a) in ALIASES.iter().enumerate() {
            for b in ALIASES.iter().skip(i + 1) {
                assert!(
                    !(a.underlying == b.underlying && a.stated == b.stated),
                    "{a:?} and {b:?}"
                );
            }
            let end = day_of(a.settled).unwrap();
            let next = alias_of(a.underlying.unwrap_or("ANY"), end);
            assert!(next.is_none(), "{a:?} settles on a re-dated text {next:?}");
        }
    }

    /// The wire words are D-3331's, and read back.
    #[test]
    fn the_kind_words_are_the_decisions_and_read_back() {
        assert_eq!(AliasKind::ClosedDay.as_str(), "closed_day");
        assert_eq!(AliasKind::VendorRedated.as_str(), "vendor_redated");
        for kind in [AliasKind::ClosedDay, AliasKind::VendorRedated] {
            assert_eq!(AliasKind::parse(kind.as_str()), Some(kind));
        }
        assert_eq!(AliasKind::parse("closed_day_last_session"), None);
        assert_eq!(AliasKind::parse("vendor_renamed"), None);
        assert_eq!(Settlement::AsStated.kind(), None);
        assert_eq!(Settlement::AsStatedUnmeasured.kind(), None);
        assert_eq!(Settlement::ClosedDayRule.kind(), Some(AliasKind::ClosedDay));
        assert_eq!(
            Settlement::Aliased(&ALIASES[0]).kind(),
            Some(AliasKind::VendorRedated)
        );
        assert_eq!(Settlement::ClosedDayRule.as_str(), "closed_day_rule");
    }

    /// DB-018 / DB-G50: one contract, one series. Both the old and the new
    /// text of a re-dated contract settle on one day, and the vendor's text is
    /// kept as the alias.
    #[test]
    fn a_renamed_contract_settles_on_its_final_day_under_both_texts() {
        // June 2023: every underlying, through the one every-underlying row.
        for under in ["ACC", "NIFTY", "PVRINOX", "ANYSTOCK"] {
            let got = settle(under, d(2023, 6, 29), Some(d(2023, 6, 27))).unwrap();
            assert_eq!(got.expiry, d(2023, 6, 28), "{under}");
            assert_eq!(got.settlement.as_str(), "aliased");
            let new = settle(under, d(2023, 6, 28), Some(d(2023, 6, 28))).unwrap();
            assert_eq!(
                (new.expiry, new.settlement),
                (d(2023, 6, 28), Settlement::AsStated)
            );
        }
        // March 2023, a stated holiday re-dated months ahead.
        let got = settle("NIFTY", d(2023, 3, 30), Some(d(2022, 4, 26))).unwrap();
        assert_eq!(got.expiry, d(2023, 3, 29));
        // A regime change on an OPEN stated day: only the underlying named.
        assert_eq!(
            settle("BANKNIFTY", d(2023, 9, 7), Some(d(2023, 9, 1)))
                .unwrap()
                .expiry,
            d(2023, 9, 6)
        );
        assert_eq!(
            settle("NIFTY", d(2023, 9, 7), Some(d(2023, 9, 1))).unwrap(),
            Settled {
                expiry: d(2023, 9, 7),
                settlement: Settlement::AsStated
            },
            "NIFTY kept Thursday in September 2023"
        );
        // A chain settles at its end in one lookup: 26MAR26 -> 31MAR26 -> 30MAR26.
        let chain = settle("NIFTY", d(2026, 3, 26), Some(d(2025, 7, 31))).unwrap();
        assert_eq!(chain.expiry, d(2026, 3, 30));
        let Settlement::Aliased(row) = chain.settlement else {
            panic!("aliased")
        };
        assert_eq!(row.via, Some((2026, 3, 31)));
        // FORWARD, not back: the row beats the rule. 26 Feb 2025 was a
        // holiday, the rule says the 25th, and the vendor re-dated to the 27th.
        assert_eq!(previous_open_session(d(2025, 2, 26)), Ok(d(2025, 2, 25)));
        assert_eq!(
            settle("BANKNIFTY", d(2025, 2, 26), Some(d(2024, 12, 2)))
                .unwrap()
                .expiry,
            d(2025, 2, 27)
        );
    }

    /// DB-G51: MIDCPNIFTY `22JAN24` settles on the Saturday session it last
    /// traded on, which the calendar records as open. The measured row and the
    /// rule agree.
    #[test]
    fn a_closed_stated_day_with_no_rename_settles_on_its_last_session() {
        assert_eq!(kind(d(2024, 1, 22)), DayKind::Closed);
        assert!(matches!(kind(d(2024, 1, 20)), DayKind::Open(_)));
        let got = settle("MIDCPNIFTY", d(2024, 1, 22), Some(d(2024, 1, 20))).unwrap();
        assert_eq!(got.expiry, d(2024, 1, 20));
        let Settlement::Aliased(row) = got.settlement else {
            panic!("aliased")
        };
        assert_eq!(row.kind, AliasKind::ClosedDay);
        assert_eq!(row.first_day, (2024, 1, 3));
        assert_eq!(previous_open_session(d(2024, 1, 22)), Ok(d(2024, 1, 20)));
        // Another underlying's 22JAN24 has no row: the rule settles it on the
        // same Saturday.
        let rule = settle("NIFTY", d(2024, 1, 22), Some(d(2024, 1, 19))).unwrap();
        assert_eq!(
            (rule.expiry, rule.settlement),
            (d(2024, 1, 20), Settlement::ClosedDayRule)
        );
    }

    /// D-3331 decision 1: a closed stated day no row names settles on the
    /// previous open session, the charter's rule, walking over weekends and
    /// holidays.
    #[test]
    fn a_closed_stated_day_no_row_names_settles_on_the_previous_session() {
        // Good Friday 2024 (Friday) -> Thursday 28 March.
        let got = settle("NIFTY", d(2024, 3, 29), Some(d(2024, 3, 27))).unwrap();
        assert_eq!(
            got,
            Settled {
                expiry: d(2024, 3, 28),
                settlement: Settlement::ClosedDayRule
            }
        );
        // A Saturday (2024-04-06) walks back to Friday 5 April.
        assert_eq!(
            settle("ACC", d(2024, 4, 6), None).unwrap().expiry,
            d(2024, 4, 5)
        );
        // A Monday holiday after a weekend (2024-01-22, a stock with no row)
        // walks over Sunday to the Saturday session.
        assert_eq!(
            settle("ACC", d(2024, 1, 22), None).unwrap().expiry,
            d(2024, 1, 20)
        );
        // The rule's settlement is still checked against the trade day.
        assert_eq!(
            settle("NIFTY", d(2024, 3, 29), Some(d(2024, 3, 29))),
            Err(SettleRefusal::SettledBeforeTrade {
                settled: d(2024, 3, 28)
            })
        );
    }

    /// Every refusal names itself; none is a fallback.
    #[test]
    fn what_cannot_be_settled_is_refused_by_name() {
        // The walk stops at a day the calendar has not measured: the day
        // before the calendar's sourced span.
        let edge =
            Day::from_days(u32::try_from(crate::calendar::SOURCED_FIRST_DAY).unwrap()).unwrap();
        let refused = previous_open_session(edge);
        assert_eq!(
            refused,
            Err(SettleRefusal::NoOpenSessionBefore { stated: edge })
        );
        assert!(
            refused.unwrap_err().to_string().contains("closed day"),
            "named"
        );
        // A closure longer than the bound, and a walk off the epoch, are
        // refused too: neither is a calendar this crate holds, so a calendar
        // is handed in.
        let closed = |_: i64| DayKind::Closed;
        let far = d(2024, 3, 29);
        assert_eq!(
            previous_open_in(far, closed),
            Err(SettleRefusal::NoOpenSessionBefore { stated: far })
        );
        let dawn = Day::from_days(3).unwrap();
        assert_eq!(
            previous_open_in(dawn, closed),
            Err(SettleRefusal::NoOpenSessionBefore { stated: dawn })
        );
        // Exactly the bound back is still found; one further is not.
        let open_at = |target: u32| {
            move |epoch: i64| {
                if epoch == i64::from(target) {
                    DayKind::OpenLengthUnmeasured
                } else {
                    DayKind::Closed
                }
            }
        };
        let at = far.days_from_epoch();
        assert_eq!(
            previous_open_in(far, open_at(at - CLOSED_WALK_DAYS)),
            Ok(Day::from_days(at - CLOSED_WALK_DAYS).unwrap())
        );
        assert!(previous_open_in(far, open_at(at - CLOSED_WALK_DAYS - 1)).is_err());
        // A retired text traded on its rename day, or later.
        for on in [d(2023, 6, 28), d(2023, 7, 3)] {
            let got = settle("ACC", d(2023, 6, 29), Some(on)).unwrap_err();
            assert_eq!(
                got,
                SettleRefusal::TextAfterRename {
                    renamed_on: d(2023, 6, 28)
                }
            );
            assert!(got.to_string().contains("2023-06-28"), "{got}");
        }
        // The day before the rename is still the old text's.
        assert!(settle("ACC", d(2023, 6, 29), Some(d(2023, 6, 27))).is_ok());
        // A trade after the settled day (a closed-day row has no rename day).
        let late = settle("MIDCPNIFTY", d(2024, 1, 22), Some(d(2024, 1, 21))).unwrap_err();
        assert_eq!(
            late,
            SettleRefusal::SettledBeforeTrade {
                settled: d(2024, 1, 20)
            }
        );
        assert!(late.to_string().contains("2024-01-20"), "{late}");
        // A closed-day row is never a rename: a trade on its first day or
        // after it, before the settlement, is its own.
        assert!(settle("MIDCPNIFTY", d(2024, 1, 22), Some(d(2024, 1, 3))).is_ok());
        // An ordinary stated day traded after itself.
        assert!(matches!(
            settle("ACC", d(2024, 4, 25), Some(d(2024, 4, 26))),
            Err(SettleRefusal::SettledBeforeTrade { .. })
        ));
        assert_eq!(day_of((2024, 2, 30)), Err(SettleRefusal::RowDayInvalid));
        assert!(
            SettleRefusal::RowDayInvalid
                .to_string()
                .contains("does not exist")
        );
    }

    /// The decoder asks `settle()` with the underlying an option is FILED
    /// under (D-3354): the newest symbol its dated changes reach. A row keyed
    /// by an OLD symbol would never be found, silently. Every row's
    /// underlying is therefore a symbol with no later change.
    #[test]
    fn every_row_is_keyed_by_a_symbol_the_decoder_files_under() {
        let head = |symbol: &str| brutex_core::alias::LINEAGE.current(symbol) == symbol;
        for row in &ALIASES {
            if let Some(underlying) = row.underlying {
                assert!(
                    head(underlying),
                    "ALIASES names {underlying}, filed elsewhere"
                );
            }
        }
        for row in &EARLY_SETTLEMENTS {
            assert!(
                head(row.underlying),
                "EARLY_SETTLEMENTS names {}, filed elsewhere",
                row.underlying
            );
        }
        // The probe is not vacuous: an old symbol is not a head.
        assert!(!head("NIITTECH") && head("COFORGE") && head("NIFTY"));
    }

    /// D-3331 decision 3: every table row is one `alias` line that reads
    /// back exactly; anything else is refused, never repaired.
    #[test]
    fn every_row_is_one_alias_line_that_reads_back_exactly() {
        let records = alias_records().unwrap();
        assert_eq!(records.len(), ALIAS_ROWS);
        for record in &records {
            assert_eq!(
                AliasRecord::parse(&record.line()),
                Some(*record),
                "{record:?}"
            );
        }
        let june = records
            .iter()
            .find(|r| r.underlying.is_none())
            .unwrap()
            .line();
        assert_eq!(
            june,
            "alias * stated=2023-06-29 settled=2023-06-28 kind=vendor_redated first_day=2023-06-28 contracts=12634 v=1"
        );
        let mid = records
            .iter()
            .find(|r| r.kind == AliasKind::ClosedDay)
            .unwrap()
            .line();
        assert_eq!(
            mid,
            "alias MIDCPNIFTY stated=2024-01-22 settled=2024-01-20 kind=closed_day first_day=2024-01-03 contracts=145 v=1"
        );
        for bad in [
            mid.replace("alias ", "aliases "),
            mid.replace("MIDCPNIFTY", "midcpnifty"),
            mid.replace("kind=closed_day", "kind=closed"),
            mid.replace("stated=2024-01-22", "stated=2024-1-22"),
            mid.replace("contracts=145", "contracts=+145"),
            mid.replace("contracts=145", "contracts="),
            mid.replace(" v=1", " v=2"),
            mid.replace("first_day=", "first="),
            mid.replace(' ', "  "),
            format!("{mid} x=1"),
            String::new(),
        ] {
            assert_eq!(AliasRecord::parse(&bad), None, "{bad:?}");
        }
        // A row that names no real day, or no symbol, is refused by name.
        let mut broken = ALIASES[0];
        broken.settled = (2023, 2, 30);
        assert_eq!(
            AliasRecord::of_row(&broken),
            Err(SettleRefusal::RowDayInvalid)
        );
        broken = ALIASES[0];
        broken.underlying = Some("NO SPACE");
        assert_eq!(
            AliasRecord::of_row(&broken),
            Err(SettleRefusal::RowDayInvalid)
        );
    }

    /// A stated day past the calendar is kept, and says so; a tenor reader
    /// with no trade day still settles.
    #[test]
    fn an_unmeasured_stated_day_is_kept_and_labelled() {
        let got = settle("NIFTY", d(2028, 12, 26), None).unwrap();
        assert_eq!(got.expiry, d(2028, 12, 26));
        assert_eq!(got.settlement, Settlement::AsStatedUnmeasured);
        assert_eq!(got.settlement.as_str(), "as_stated_unmeasured");
        assert_eq!(Settlement::AsStated.as_str(), "as_stated");
        // A table target after the calendar: the row still applies.
        assert_eq!(
            settle("NIFTY", d(2027, 12, 30), None).unwrap().expiry,
            d(2027, 12, 28)
        );
        // A trade day of None skips the rename check: a tenor of a bar is
        // measured to the settled day whatever day asked.
        assert_eq!(
            settle("ACC", d(2023, 6, 29), None).unwrap().expiry,
            d(2023, 6, 28)
        );
    }

    /// DB-B01's ten texts: underlying, stated, settled, last trade day (name
    /// census), rename day.
    const STOPPED_EARLY: [(&str, Ymd, Ymd, Ymd, Ymd); 10] = [
        (
            "NIFTY",
            (2027, 6, 24),
            (2027, 6, 29),
            (2025, 7, 30),
            (2025, 8, 1),
        ),
        (
            "NIFTY",
            (2028, 6, 29),
            (2028, 6, 27),
            (2024, 6, 12),
            (2025, 8, 1),
        ),
        (
            "NIFTY",
            (2028, 12, 28),
            (2028, 12, 26),
            (2025, 7, 28),
            (2025, 8, 1),
        ),
        (
            "NIFTY",
            (2029, 6, 28),
            (2029, 6, 26),
            (2024, 11, 14),
            (2025, 8, 1),
        ),
        (
            "NIFTY",
            (2029, 12, 27),
            (2029, 12, 24),
            (2025, 6, 26),
            (2025, 8, 1),
        ),
        (
            "FINNIFTY",
            (2021, 11, 3),
            (2021, 11, 2),
            (2021, 10, 13),
            (2021, 10, 18),
        ),
        (
            "FINNIFTY",
            (2021, 11, 11),
            (2021, 11, 9),
            (2021, 10, 14),
            (2021, 10, 18),
        ),
        (
            "FINNIFTY",
            (2021, 11, 18),
            (2021, 11, 16),
            (2021, 10, 14),
            (2021, 10, 18),
        ),
        (
            "FINNIFTY",
            (2021, 12, 30),
            (2021, 12, 28),
            (2021, 10, 4),
            (2021, 10, 18),
        ),
        (
            "MIDCPNIFTY",
            (2023, 9, 27),
            (2023, 9, 25),
            (2023, 8, 16),
            (2023, 8, 17),
        ),
    ];

    /// DB-B01. Each of the ten texts that stopped before its underlying's
    /// rename day settles on the same contract as its successor text, on
    /// every day it could have traded, and is refused from the rename day on.
    /// Before the rows existed each answered its own stated day
    /// (`AsStated`, or `AsStatedUnmeasured` past 2026-10-04): two store
    /// series for one exchange contract.
    #[test]
    fn a_text_that_stopped_before_its_rename_day_is_the_contract_its_successor_names() {
        let mut names = 0;
        for (under, stated, settled, last_trade, renamed) in STOPPED_EARLY {
            let (stated, settled) = (day_of(stated).unwrap(), day_of(settled).unwrap());
            let (last_trade, renamed) = (day_of(last_trade).unwrap(), day_of(renamed).unwrap());
            let old = settle(under, stated, Some(last_trade)).unwrap();
            let new = settle(under, settled, Some(renamed)).unwrap();
            assert_eq!(old.expiry, settled, "{under} {stated}");
            assert_eq!(old.expiry, new.expiry, "{under} {stated}: one contract");
            let Settlement::Aliased(row) = old.settlement else {
                panic!("{under} {stated}: {:?}", old.settlement);
            };
            assert_eq!(row.kind, AliasKind::VendorRedated);
            names += row.contracts;
            // The tenor reader's question (no trade day) has the same answer.
            assert_eq!(settle(under, stated, None).unwrap().expiry, settled);
            // The retired text on or after the rename day names another
            // contract: refused, never filed.
            assert_eq!(
                settle(under, stated, Some(renamed)),
                Err(SettleRefusal::TextAfterRename {
                    renamed_on: renamed
                }),
                "{under} {stated}"
            );
            // Another underlying's same stated day is not this row's.
            assert_ne!(
                settle("RELIANCE", stated, None).map(|s| s.expiry),
                Ok(settled),
                "{under} {stated}"
            );
        }
        assert_eq!(names, 25, "the audit's 18 + 6 + 1 names");
        // NSE/FAOP/68747's annexure names eleven NIFTY re-datings; every one
        // is a row.
        let nifty = ALIASES
            .iter()
            .filter(|row| row.underlying == Some("NIFTY") && row.first_day == (2025, 8, 1))
            .count();
        assert_eq!(
            nifty, 11,
            "9 half-yearly/quarterly texts and the two 2025 monthlies"
        );
    }

    /// DB-B03. The sourced VEDL scheme (NSE/FAOP/73857): a file of a day
    /// before the ex-date is the generation that expired on the cum-date; the
    /// same text from the ex-date on is the re-listed contract. The identity
    /// is the stated expiry for both (three stated expiries expired on one
    /// cum-date), and only the last day differs.
    #[test]
    fn a_contract_a_scheme_expired_early_names_the_day_it_really_expired() {
        let cum = d(2026, 4, 29);
        let ex = d(2026, 4, 30);
        for stated in [d(2026, 5, 26), d(2026, 6, 30), d(2026, 7, 28)] {
            for trade in [d(2026, 3, 2), cum] {
                let old = settle("VEDL", stated, Some(trade)).unwrap();
                assert_eq!(old.expiry, stated, "the identity is the stated expiry");
                assert_eq!(old.last_day(), Ok(cum), "{stated} on {trade}");
                assert_eq!(old.settlement.as_str(), "expired_early");
                assert_eq!(old.settlement.kind(), None, "no alias: the key is the text");
                let Settlement::ExpiredEarly(row) = old.settlement else {
                    panic!("{:?}", old.settlement);
                };
                assert_eq!(row.circular, "NSE/FAOP/73857");
                assert!(row.url.ends_with("FAOP73857.pdf"));
            }
            for trade in [ex, d(2026, 5, 4), stated] {
                let new = settle("VEDL", stated, Some(trade)).unwrap();
                assert_eq!(new.settlement, Settlement::AsStated, "{stated} on {trade}");
                assert_eq!(new.last_day(), Ok(stated));
            }
            // No trade day, no generation: the stated day, as before.
            let unknown = settle("VEDL", stated, None).unwrap();
            assert_eq!(unknown.settlement, Settlement::AsStated);
            // Another underlying, and an expiry the circular does not name.
            assert_eq!(
                settle("HINDUNILVR", stated, Some(cum)).unwrap().settlement,
                Settlement::AsStated
            );
            assert!(early_settlement("VEDL", stated, cum).is_some());
            assert!(early_settlement("VEDL", stated, ex).is_none());
        }
        assert!(early_settlement("VEDL", d(2026, 8, 25), cum).is_none());
        assert_eq!(
            settle("VEDL", d(2026, 8, 25), Some(cum))
                .unwrap()
                .settlement,
            Settlement::AsStated
        );
        // The ex-date is the day the contracts were listed again, and only it.
        assert!(relisted_on("VEDL", ex).is_some());
        assert!(relisted_on("VEDL", cum).is_none());
        assert!(relisted_on("HINDUNILVR", ex).is_none());
        // Every row is real days in order: cum-date before ex-date before
        // every expiry it names, and the cum-date is a session.
        for row in &EARLY_SETTLEMENTS {
            let (settled, ex) = (day_of(row.settled).unwrap(), day_of(row.ex_date).unwrap());
            assert!(settled.days_from_epoch() < ex.days_from_epoch(), "{row:?}");
            assert!(matches!(kind(settled), DayKind::Open(_)), "{row:?}");
            assert!(!row.expiries.is_empty() && row.circular.starts_with("NSE/FAOP/"));
            for &expiry in row.expiries {
                assert!(day_of(expiry).unwrap().days_from_epoch() > ex.days_from_epoch());
            }
        }
    }
}
