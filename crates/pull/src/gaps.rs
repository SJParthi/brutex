//! Why every absent bar is absent — classified, not guessed.
//!
//! # The question this answers
//!
//! On 2026-08-22 an operator asked why bars were missing from a completed pull.
//! Answering took an afternoon of decoding fixed-stride records by hand, and the
//! store-derived answer was that **almost nothing was missing**: of 1,204 absent
//! minutes in a 623,546-bar series, 1,176 were assigned to sessions that were
//! never 375 minutes long and 28 to vendor loss. D-0420 later proved one premise
//! circular: the 2021-02-24 54-bar index prefix was being used as its own
//! exchange timetable. The current classifier keeps the verified 220-minute
//! market session and withholds an unproved spot-index denominator for that day.
//!
//! Every one of those numbers was recoverable from the store. None of them was
//! **reported** by it. That is the gap this module closes: the store knew, and
//! nothing asked it.
//!
//! # The taxonomy is exhaustive by construction
//!
//! An absent minute is in exactly one of five states, and [`Reason`] has exactly
//! five variants because there is no sixth. A minute is either on a day the
//! exchange did not trade, or on a traded day but outside its windows, or inside
//! a window and simply not there, or on a day this build has never measured, or
//! inside a session this series did not owe ([`NotOwed`], R4-19). `match` is
//! therefore total and a new case cannot be added without the compiler naming
//! every site that must handle it.
//!
//! **Only the third is a loss.** Conflating it with the other four is what made
//! six complete series report `SHORT`, and separating them is the whole point.
//!
//! # A session the series did not owe (R4-19; G4, Z3, TC-04, COR-07)
//!
//! The fifth state was a fourth-state answer before it had a name: the exchange
//! traded, the minute was inside the window, no bar was stored, so the minute
//! was `vendor-hole`. Measured on the operator's store, `/gaps.json` answered
//! FORCEMOT CASH 2023-12 `expected 7500, held 0, lost_minutes 7500`, and the
//! repair plan named 2024-02 a versioned repair for the nine sessions before
//! the 14th, while NSE had withdrawn the share from trading for all of them
//! (CML58560; readmitted from 2024-02-14 by the circular CML60618 a company
//! filing carries, `docs/00-charter.md`, "FORCEMOT NSE interruption"). No
//! vendor holds a bar NSE never traded, so no retry or repair can fill one.
//!
//! [`not_owed`] is the ONE authority that says so, for every owed path: each
//! classifier door here ([`classify_owed`]) and the repair plan
//! ([`crate::ingest::repair_plan::plan`]). It answers, in order, NSE's
//! independent record of an interruption, a day before the listing or after
//! the delisting the store's receipted NSE security master states
//! ([`crate::listing`], R4-17), and a day no stored reading covers
//! ([`Coverage`], TC-04). The calendar still decides whether a SESSION
//! happened; [`not_owed`] decides only whether this series owed it, so a
//! closed day stays `closed`, and a day the calendar cannot attest stays
//! `unmeasured` unless the instrument's own sourced fact — withdrawn, not yet
//! listed, delisted — excuses it whatever the exchange did.
//!
//! # Cost
//!
//! **A two-cursor merge, O(1) per minute and per bar, with no lookup structure
//! at all.** Stored bars arrive in timestamp order — the store's own append
//! rule guarantees it — so the expected minutes and the stored bars are walked
//! together and each side advances at most once per step. There is no set, no
//! map and no allocation per minute; the only allocation is the answer itself,
//! and that is bounded by [`MAX_GAPS`].
//!
//! A `HashSet` of stored minutes would have been the obvious shape and is
//! strictly worse: it costs a hash per bar to answer a question the ordering
//! already answers for free.
//!
//! **UNVERIFIED as a measurement.** The bound above is argued from the SHAPE of
//! the walk — two cursors, each advancing at most once per step — and no bench
//! in this workspace times it. `crates/pull/benches/ratio.rs` covers the
//! manifest's census and entry lookup and nothing here. The argument is the
//! stronger kind, because a merge that cannot revisit a position has no input
//! that makes it quadratic, but `CLAUDE.md` §3 rule 6 does not let a structural
//! argument be reported as a measured one. Closing it is a ratio row over
//! `classify` at 1x/10x/100x the minute count, beside the rows that already
//! exist for the manifest.

use brutex_core::isin::Isin;

use crate::calendar::{self, Calendar, DayKind};
use crate::cash_auction::InactiveInterval;
use crate::listing::{Listing, Span};
use crate::session::Day;

/// The most gaps one classification will report.
///
/// **A bound, because an unbounded answer is the failure it is describing.** A
/// store that lost a whole year would otherwise produce a `Vec` of ~94,000
/// entries on a path an operator polls, and a report that cannot be rendered is
/// not a report. Past this the walk stops and says so — see [`Ledger::truncated`]
/// — which is `CLAUDE.md` §4's "degrade loudly and name the reason" rather than
/// a silent `take(N)`.
pub const MAX_GAPS: usize = 4_096;

/// Why a minute is not in the store.
///
/// Five variants and no sixth: see the module header for why the taxonomy is
/// exhaustive by construction. Until R4-19 there were four, and the fifth's
/// minutes were answered as [`Self::VendorHole`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Reason {
    /// The exchange did not trade that day at all — a weekend or a holiday.
    ///
    /// Not a loss. This is the reason that made every complete series read
    /// `SHORT` when it was counted as one.
    Closed,
    /// The day traded, and this minute was outside its windows.
    ///
    /// Muhurat is one hour in the afternoon; a disaster-recovery Saturday has a
    /// two-hour hole in the middle by design. Neither is a loss, and both look
    /// exactly like one to an arithmetic count.
    OutsideWindow,
    /// Inside a window the exchange traded, and the bar is not there.
    ///
    /// An expected minute absent from the supplied store image. This alone
    /// cannot distinguish a missed download from an absent provider record or
    /// an instrument that did not trade. A scoped source replay and identity /
    /// listing evidence are needed before attributing the cause. Another feed
    /// must never silently replace this feed's history.
    VendorHole,
    /// The calendar does not cover this day, so no claim is made.
    ///
    /// Deliberately not folded into [`Self::Closed`]: "the exchange was shut"
    /// and "nobody has looked" are different facts, and reporting the second as
    /// the first is the invention `CLAUDE.md` §3 rule 1 bans.
    Unmeasured,
    /// Inside a session of record that this series did not owe, and why
    /// ([`not_owed`], R4-19): never a loss, never counted in what a ledger
    /// owes, and each carries its evidence.
    NotOwed(NotOwed),
}

impl Reason {
    /// Whether this absence is a loss an operator should act on.
    #[must_use]
    pub const fn is_loss(self) -> bool {
        matches!(self, Self::VendorHole)
    }

    /// A short, stable word for a report or a log line.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Closed => "closed",
            Self::OutsideWindow => "outside-window",
            Self::VendorHole => "vendor-hole",
            Self::Unmeasured => "unmeasured",
            Self::NotOwed(why) => why.word(),
        }
    }
}

/// Why a session of record is not owed by one series: the answer of the ONE
/// authority every owed path asks ([`not_owed`]; R4-19, findings G4, Z3,
/// COR-07 and TC-04).
///
/// **None is a loss and none is a guess.** The first three are facts about the
/// instrument, each from a named source; the fourth is a fact about this
/// store. A session owed by nobody cannot be filled by a retry, a repair or
/// another vendor, and counting it lost is what sent the operator after bars
/// NSE never traded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotOwed {
    /// NSE independently documents that this exact symbol and ISIN did not
    /// trade that day: the interval and its two sources, as
    /// [`crate::cash_auction::independently_inactive`] states them (FORCEMOT
    /// 2023-10-26..=2024-02-13: the withdrawal circular CML58560, and the
    /// company filing carrying the readmission circular CML60618;
    /// `docs/00-charter.md`, "FORCEMOT NSE interruption").
    InstrumentInactive(InactiveInterval),
    /// Before the listing day the store's receipted NSE security master
    /// states for this share ([`crate::listing::Span::before_listing`],
    /// R4-17). A listing day the master does not state excuses nothing.
    BeforeListing {
        /// The listing day the master states.
        listed_on: Day,
    },
    /// Strictly after the removal day that master states, with no
    /// readmission after it: the share's history ended
    /// ([`crate::listing::Span::history_ended`], R4-17). The removal day
    /// itself is still owed.
    AfterDelisting {
        /// The removal day the master states.
        delisted_on: Day,
    },
    /// No stored reading covers the day ([`Coverage`], TC-04): nobody read
    /// it, so its absence says nothing about the vendor, either way.
    Unread,
}

impl NotOwed {
    /// The code a month or a day carries for it, the four `web/` already
    /// spells (`ingest-months.js`, `REASON_CODES`, R4-01).
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::InstrumentInactive(_) => "instrument_inactive",
            Self::BeforeListing { .. } => crate::listing::BEFORE_LISTING,
            Self::AfterDelisting { .. } => "after_delisting",
            Self::Unread => "unread",
        }
    }

    /// The word a run of its minutes is reported with, in the family of
    /// [`Reason::word`].
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::InstrumentInactive(_) => "instrument-inactive",
            Self::BeforeListing { .. } => "before-listing",
            Self::AfterDelisting { .. } => "after-delisting",
            Self::Unread => "unread",
        }
    }

    /// Its evidence in the words a terminal and a page print, one wording for
    /// every consumer: the interval and both NSE circulars by number beside
    /// the documents that carry them, the day the master states, or that no
    /// reading covers the day.
    #[must_use]
    pub fn evidence(self) -> String {
        match self {
            Self::InstrumentInactive(interval) => format!(
                "NSE documents the share as not trading {}..={}: withdrawal circular {} ({}), \
                 readmission circular {} (in {})",
                interval.from,
                interval.through,
                interval.withdrawal_circular,
                interval.withdrawal_source,
                interval.readmission_circular,
                interval.readmission_source
            ),
            Self::BeforeListing { listed_on } => format!(
                "before its listing on {listed_on}, as the store's receipted NSE security \
                 master states"
            ),
            Self::AfterDelisting { delisted_on } => format!(
                "after its removal on {delisted_on}, as the store's receipted NSE security \
                 master states, with no readmission after it"
            ),
            Self::Unread => "no stored reading covers the day".to_owned(),
        }
    }
}

/// Days a feed's gap ledger records as requested (`audit/<feed>.gaps`), one
/// bit a day from [`Self::from`]: the half of TC-04's evidence a held span
/// does not carry. O(1) a probe, whatever the span: measured as part of
/// `not_owed` per call by `crates/api/benches/ratio.rs` (the gap rows).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Requested<'a> {
    /// The day bit 0 of the first word stands for, days since the epoch.
    pub from: i64,
    /// Bit `i % 64` of word `i / 64` is day `from + i`; a day past the last
    /// word, or before `from`, is not recorded.
    pub bits: &'a [u64],
}

impl Requested<'_> {
    /// No day recorded as requested.
    pub const NONE: Requested<'static> = Requested { from: 0, bits: &[] };

    /// Whether the ledger records `day` (days since the epoch) as requested.
    #[must_use]
    pub fn holds(&self, day: i64) -> bool {
        let Some(offset) = day
            .checked_sub(self.from)
            .and_then(|at| u64::try_from(at).ok())
        else {
            return false;
        };
        usize::try_from(offset / 64)
            .ok()
            .and_then(|word| self.bits.get(word))
            .is_some_and(|word| (word >> (offset % 64)) & 1 == 1)
    }
}

/// Which days a stored reading covers: the evidence [`NotOwed::Unread`]
/// rests on (TC-04), the same `calendar_record::walk` reads beside the
/// calendar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Coverage<'a> {
    /// The caller states nothing about reading, so no day is called unread:
    /// every day is judged as read, which is what every door answered before
    /// R4-19. Never a default that hides a loss: it claims one exactly as
    /// before.
    #[default]
    Unstated,
    /// What reading covers the series: a day is unread when it lies after
    /// `held_through` (or nothing is held) and `requested` does not record
    /// it. A day on or before the last held one was read; a hole there is
    /// the vendor's.
    Stated {
        /// The series' last held day (days since the epoch), or `None` when
        /// its month file is absent.
        held_through: Option<i64>,
        /// The days the feed's gap ledger records as requested.
        requested: Requested<'a>,
    },
}

impl Coverage<'_> {
    /// Whether no stored reading covers `day` (days since the epoch).
    #[must_use]
    pub fn unread(&self, day: i64) -> bool {
        match self {
            Self::Unstated => false,
            Self::Stated {
                held_through,
                requested,
            } => held_through.is_none_or(|last| day > last) && !requested.holds(day),
        }
    }
}

/// The series [`not_owed`] is asked about, and the evidence it reads.
///
/// Built once per series ([`Key::series`]), so the one listing probe is paid
/// once and each day after it is a handful of compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Key<'a> {
    /// The exchange, exactly as the store names it.
    pub exchange: &'a str,
    /// The segment, exactly as the store names it.
    pub segment: &'a str,
    /// The exact symbol, never an alias.
    pub symbol: &'a str,
    /// The share's exact ISIN ([`brutex_core::universe::nse_isin`]), when the
    /// universe holds one.
    pub isin: Option<Isin>,
    /// The span the store's receipted NSE security master states for the
    /// share ([`Listing::span`]), when one does.
    pub span: Option<Span<'a>>,
    /// What stored reading covers which day.
    pub coverage: Coverage<'a>,
}

impl<'a> Key<'a> {
    /// A key that names no series: no instrument fact and no reading
    /// evidence, so [`not_owed`] excuses nothing. The doors that take no key
    /// pass this, and answer exactly what they answered before R4-19.
    pub const NONE: Key<'static> = Key {
        exchange: "",
        segment: "",
        symbol: "",
        isin: None,
        span: None,
        coverage: Coverage::Unstated,
    };

    /// The key of `exchange`/`segment`/`symbol`, as every owed path builds it.
    ///
    /// For an NSE cash share: its exact ISIN from the universe (the table the
    /// closing-auction schedule and the listing rule are keyed by; no second
    /// symbol-to-ISIN map) and the span `listing` states for that identity,
    /// probed once here. Any other series carries its names alone, and the
    /// instrument facts, which are the NSE cash market's, excuse none of its
    /// days. `listing` is `None` for a caller that reads no master.
    #[must_use]
    pub fn series(
        exchange: &'a str,
        segment: &'a str,
        symbol: &'a str,
        listing: Option<&'a Listing>,
    ) -> Self {
        let cash = exchange == "NSE" && segment == "CASH";
        let isin = if cash {
            brutex_core::universe::nse_isin(symbol)
        } else {
            None
        };
        let span = listing
            .filter(|_| cash)
            .and_then(|table| table.span(exchange, symbol, isin.as_ref().map(Isin::as_str)));
        Self {
            exchange,
            segment,
            symbol,
            isin,
            span,
            coverage: Coverage::Unstated,
        }
    }

    /// This key, with `coverage` as its reading evidence (TC-04).
    #[must_use]
    pub const fn read(self, coverage: Coverage<'a>) -> Self {
        Self { coverage, ..self }
    }
}

/// Whether `key`'s series does not owe `day` (days since the epoch), and why:
/// the ONE door every owed path asks (R4-19). Each classifier door here calls
/// it through [`classify_owed`], and the repair plan per session.
///
/// The first that holds, in this order:
///
/// 1. NSE independently records the share as not trading that day
///    ([`NotOwed::InstrumentInactive`]).
/// 2. The day is before the listing day, or strictly after the removal day,
///    that the store's receipted master states ([`NotOwed::BeforeListing`],
///    [`NotOwed::AfterDelisting`]). An unknown listing day (`0`, blank,
///    conflicting), a span inferred from the vendor's own bars, or no master
///    at all excuses nothing: the day is owed exactly as before.
/// 3. No stored reading covers the day ([`NotOwed::Unread`]).
///
/// The first two are the NSE cash market's facts, so any other exchange or
/// segment, and a symbol or ISIN that is not exactly the documented one, gets
/// neither. A day that is no calendar day gets neither either; it can still be
/// unread.
///
/// # Cost
///
/// O(1) and no allocation, no file, no lock: a handful of constant compares
/// ([`crate::cash_auction::independently_inactive`]), at most two compares
/// against the span the key already holds, and one bit probe of the reading
/// evidence. The listing probe is [`Key::series`]'s, once per series.
/// Measured, per call at 1x, 10x and 100x months, by
/// `crates/api/benches/ratio.rs` (its `/gaps.json` rows,
/// `benches/ratio/gaps.rs`).
#[must_use]
pub fn not_owed(key: &Key<'_>, day: i64) -> Option<NotOwed> {
    if key.exchange == "NSE"
        && key.segment == "CASH"
        && let Some(date) = u32::try_from(day)
            .ok()
            .and_then(|number| Day::from_days(number).ok())
    {
        if let Some(interval) = key.isin.and_then(|isin| {
            crate::cash_auction::independently_inactive(key.symbol, isin.as_str(), date)
        }) {
            return Some(NotOwed::InstrumentInactive(interval));
        }
        if let Some(span) = key.span {
            if let (true, Some(listed_on)) = (span.before_listing(date), span.listed_on()) {
                return Some(NotOwed::BeforeListing { listed_on });
            }
            if let Some(delisted_on) = span.history_ended(date) {
                return Some(NotOwed::AfterDelisting { delisted_on });
            }
        }
    }
    key.coverage.unread(day).then_some(NotOwed::Unread)
}

/// One run of consecutive absent minutes sharing a reason.
///
/// Runs rather than minutes, because 1,176 of the 1,204 absences measured were
/// four contiguous events. A per-minute list of the same facts is 43× longer and
/// says less.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Gap {
    /// Days since the epoch.
    pub day: i64,
    /// Minute-of-day the run starts.
    pub from: u16,
    /// Minute-of-day the run ends, inclusive.
    pub to: u16,
    /// Why.
    pub reason: Reason,
}

impl Gap {
    /// How many minutes this run covers.
    #[must_use]
    pub const fn minutes(self) -> u32 {
        (self.to as u32)
            .saturating_sub(self.from as u32)
            .saturating_add(1)
    }
}

/// What one classification found.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Ledger {
    /// The runs, in day then minute order.
    pub gaps: Vec<Gap>,
    /// Bars the calendar says the range owed.
    pub expected: u32,
    /// Bars the store actually held.
    pub held: u32,
    /// Off-grid, duplicate or non-increasing timestamps in the input image.
    /// When nonzero, coverage totals are withheld, not a clean zero verdict.
    pub invalid_timestamps: u32,
    /// Whether [`MAX_GAPS`] stopped the walk before it finished.
    ///
    /// **Never silent.** A truncated ledger that read like a complete one would
    /// tell an operator their store is healthier than it is.
    pub truncated: bool,
}

impl Ledger {
    /// Minutes absent for a reason that is a real loss.
    #[must_use]
    pub fn lost_minutes(&self) -> u32 {
        self.gaps
            .iter()
            .filter(|g| g.reason.is_loss())
            .map(|g| g.minutes())
            .fold(0_u32, u32::saturating_add)
    }

    /// Minutes absent for every reason, loss or not.
    #[must_use]
    pub fn absent_minutes(&self) -> u32 {
        self.gaps
            .iter()
            .map(|gap| gap.minutes())
            .fold(0_u32, u32::saturating_add)
    }

    /// Minutes this build makes **no claim** about, either way.
    ///
    /// [`Reason::Unmeasured`] alone: a day outside `FIRST_DAY..=LAST_DAY`, or a
    /// Muhurat whose length the minute series does not reach. Neither a loss
    /// nor a clean bill.
    ///
    /// # Why this needed its own accessor
    ///
    /// It is the number that explains a `held` LARGER than an `expected`, and
    /// without it that pair reads as an arithmetic bug. Measured on the
    /// operator's store: NIFTY at one minute for 2026-08 answered
    /// `expected 5,625, held 7,500, lost 0` — twenty trading days of bars
    /// against fifteen the calendar could vouch for, because `LAST_DAY` is
    /// 2026-08-21 and the run was read on 2026-08-29. Every day past the
    /// table is `Unmeasured` and adds nothing to `expected`, which is correct
    /// and looks exactly like a defect until this number is beside it.
    #[must_use]
    pub fn unmeasured_minutes(&self) -> u32 {
        self.gaps
            .iter()
            .filter(|g| matches!(g.reason, Reason::Unmeasured))
            .map(|gap| gap.minutes())
            .fold(0_u32, u32::saturating_add)
    }

    /// Minutes inside sessions this series did not owe ([`Reason::NotOwed`],
    /// R4-19): neither a loss nor counted in [`Self::expected`].
    #[must_use]
    pub fn not_owed_minutes(&self) -> u32 {
        self.gaps
            .iter()
            .filter(|g| matches!(g.reason, Reason::NotOwed(_)))
            .map(|gap| gap.minutes())
            .fold(0_u32, u32::saturating_add)
    }

    /// Each distinct not-owed answer the ledger holds, first seen first, with
    /// the days and the minutes it covers: what a page names beside the loss,
    /// so it states every class and its source rather than a bare total.
    ///
    /// One pass over the runs, at most a few distinct answers (one interval
    /// per documented interruption, one listing day, one removal day, unread):
    /// O(runs), bounded by [`MAX_GAPS`].
    #[must_use]
    pub fn not_owed(&self) -> Vec<Excused> {
        let mut out: Vec<Excused> = Vec::new();
        for gap in &self.gaps {
            let Reason::NotOwed(why) = gap.reason else {
                continue;
            };
            if let Some(seen) = out.iter_mut().find(|seen| seen.why == why) {
                if seen.last_day != gap.day {
                    seen.days = seen.days.saturating_add(1);
                    seen.last_day = gap.day;
                }
                seen.minutes = seen.minutes.saturating_add(gap.minutes());
            } else {
                out.push(Excused {
                    why,
                    days: 1,
                    minutes: gap.minutes(),
                    first_day: gap.day,
                    last_day: gap.day,
                });
            }
        }
        out
    }
}

/// One not-owed answer of a [`Ledger`], and how much of it the ledger holds
/// ([`Ledger::not_owed`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Excused {
    /// The answer.
    pub why: NotOwed,
    /// The days it covers.
    pub days: u32,
    /// The minutes it covers.
    pub minutes: u32,
    /// The first day it covers, days since the epoch.
    pub first_day: i64,
    /// The last day it covers, days since the epoch.
    pub last_day: i64,
}

/// Minutes since IST midnight for a micros-since-epoch stamp, and its day.
///
/// IST is UTC+5:30 with no daylight saving, so the shift is a constant and
/// there is no zone table to consult — one add and two divides.
fn ist(ts_micros: i64) -> (i64, u16) {
    const IST_OFFSET_SECS: i64 = 5 * 3600 + 30 * 60;
    let secs = ts_micros.div_euclid(1_000_000) + IST_OFFSET_SECS;
    let day = secs.div_euclid(86_400);
    // `rem_euclid` is non-negative and under 86,400, so this is 0..=1439 and
    // `try_from` cannot fail. `u16::MAX` is the fallback rather than a panic
    // because it is the SAFE direction: 65,535 is not a minute-of-day, so an
    // impossible stamp matches no expected minute and is reported as a hole
    // rather than silently satisfying one.
    let minute = u16::try_from(secs.rem_euclid(86_400) / 60).unwrap_or(u16::MAX);
    (day, minute)
}

/// Classify every minute the calendar owes across `first ..= last`.
///
/// `stored` is the timestamps the store holds for that range, **in ascending
/// order** — which is what the store's append rule already guarantees, and what
/// lets this be a merge rather than a lookup.
///
/// # Cost
///
/// One step per expected minute and one per stored bar, each O(1). No set, no
/// map, no per-minute allocation.
///
/// **UNVERIFIED as a measurement** — see the module's own Cost note. The bound
/// is a property of the merge rather than a timing anyone took.
///
/// # Panics
///
/// Never. Every index is bounds-checked and every arithmetic is saturating.
#[must_use]
pub fn classify(stored: &[i64], first: i64, last: i64) -> Ledger {
    classify_against(stored, first, last, None)
}

/// [`classify`], against a calendar the caller supplies.
///
/// # Why this exists, and it is not a convenience
///
/// There are **two calendars in this workspace**, and this module's free
/// functions read the one that goes stale. `crate::calendar`'s table is typed
/// and bounded by [`crate::calendar::LAST_DAY`]; `api::calendar_of` DERIVES a
/// [`Calendar`] from the store's own daily and minute rungs and widens by
/// itself the moment an earlier month lands. `calendar_of`'s own header names
/// the hazard outright: *"Three tables of one fact is a drift waiting for a
/// date. This is the one answer both should read."*
///
/// Measured on 2026-08-29, eight days past `LAST_DAY = 2026-08-21`: an audit of
/// NIFTY at one minute for 2026-08 answered `expected 5,625, held 7,500,
/// lost 0` — twenty trading days of bars against fifteen the TABLE could vouch
/// for. Nothing was wrong with the store and nothing was wrong with the
/// classification; the denominator was reading a calendar that had run out.
///
/// So the caller with a derived calendar passes it, and the caller without one
/// gets the table. `None` is not a default that hides anything: it is the
/// honest answer for a caller that has no store to derive from, and the
/// `Unmeasured` runs it produces say so per day.
///
/// # Panics
///
/// Never. Same bounds as [`classify`].
#[must_use]
pub fn classify_against(
    stored: &[i64],
    first: i64,
    last: i64,
    against: Option<&Calendar>,
) -> Ledger {
    classify_owed(
        stored,
        first,
        last,
        against,
        Door::ExchangeSession,
        &Key::NONE,
    )
}

/// Classify an NSE equity-derivatives series against that venue's dated hours.
///
/// The exchange calendar decides which days are regular sessions; the venue
/// decides their hours, exactly as `fold::complete_minutes_for_venue` does for
/// the same bars. [`classify_against`] used the calendar's fixed 375-minute
/// session for every series, so after NSE extended derivatives trading to
/// 15:40 from 2026-08-03 a month whose vendor dropped 15:30..=15:39 audited as
/// lost nothing while the fold withheld it (audit-20261003 hunt-pull-1,
/// D-1529). A day whose venue hours cannot be stated is `Unmeasured`, as a cash
/// day without dated eligibility is.
#[must_use]
pub fn classify_derivative_against(
    stored: &[i64],
    first: i64,
    last: i64,
    against: Option<&Calendar>,
) -> Ledger {
    classify_owed(stored, first, last, against, Door::Derivatives, &Key::NONE)
}

/// Classify a spot-index series without inventing index publication during the
/// 2021-02-24 NSE systems outage.
///
/// SEBI proves the equity market's two normal-trading windows that day, but the
/// same order records NIFTY computation unavailable from 10:06 to 11:43. It
/// does not establish an exact common publication window for NIFTY, BANKNIFTY
/// and INDIA VIX. Therefore a generic spot-index audit makes no loss claim for
/// that entire day. Equity-series callers continue to use [`classify_against`]
/// and its verified 220-minute exchange session.
#[must_use]
pub fn classify_spot_index_against(
    stored: &[i64],
    first: i64,
    last: i64,
    against: Option<&Calendar>,
) -> Ledger {
    classify_owed(stored, first, last, against, Door::SpotIndex, &Key::NONE)
}

/// Audit NSE cash minutes against the independently verified exchange calendar
/// and this exact stock's dated continuous-session eligibility.
///
/// A peer's observed close is not this stock's schedule. Before the CAS change,
/// exceptional exchange windows retain their verified shape. From the change,
/// only full verified sessions with exact dated eligibility get a denominator;
/// missing eligibility and exceptional / unknown sessions remain unmeasured.
/// No metadata is fetched and no source bars are altered by this function.
///
/// **`key` names the share (R4-19).** A session [`not_owed`] excuses for it —
/// NSE withdrew it, it was not yet listed or had been delisted, or no reading
/// covers the day — owes nothing and loses nothing: its minutes read the
/// reason, never `vendor-hole`. [`Key::series`] builds it; [`Key::NONE`]
/// excuses nothing and answers what this door answered before.
#[must_use]
pub fn classify_cash(
    stored: &[i64],
    first: i64,
    last: i64,
    schedule: Option<&crate::cash_auction::Schedule>,
    key: &Key<'_>,
) -> Ledger {
    classify_owed(stored, first, last, None, Door::Cash(schedule), key)
}

/// Which session authority a classification owes a series against.
#[derive(Debug, Clone, Copy)]
pub enum Door<'a> {
    /// The exchange calendar's session ([`classify_against`]).
    ExchangeSession,
    /// The exchange session, less the 2021-02-24 outage day no source sizes
    /// for an index ([`classify_spot_index_against`]).
    SpotIndex,
    /// The derivatives venue's dated hours ([`classify_derivative_against`]).
    Derivatives,
    /// The cash venue's dated hours under this share's dated closing-auction
    /// eligibility ([`classify_cash`]).
    Cash(Option<&'a crate::cash_auction::Schedule>),
}

/// Classify every minute `door` owes across `first ..= last`, excusing each
/// session [`not_owed`] excuses for `key` (R4-19): the one walk every door
/// above is.
///
/// A session of record — open, or open with a length no record states — that
/// `key`'s series does not owe adds nothing to [`Ledger::expected`], and each
/// of its minutes that would have been owed and held no bar reads
/// [`Reason::NotOwed`]; a minute outside its windows is still
/// `outside-window`, and a stored bar on it is still held. A closed day keeps
/// `closed`: [`not_owed`] says what a series owed, never whether the exchange
/// traded. A day the calendar cannot attest keeps `unmeasured` unless the
/// instrument owed nothing on it whatever the exchange did (withdrawn, not
/// yet listed, delisted): then one run of that reason; `unread` is never
/// taken there, as no session was there to read.
///
/// # Cost
///
/// [`classify`]'s two-cursor merge, plus one [`not_owed`] per session: O(1)
/// per minute and per bar, no set, no map, no allocation beyond the answer.
/// Measured per day at 1x, 10x and 100x months, with a key, by
/// `crates/api/benches/ratio.rs` (its `/gaps.json` rows); the merge alone is
/// still UNVERIFIED as a measurement (the module's Cost note).
#[must_use]
pub fn classify_owed(
    stored: &[i64],
    first: i64,
    last: i64,
    against: Option<&Calendar>,
    door: Door<'_>,
    key: &Key<'_>,
) -> Ledger {
    let mut ledger = Ledger {
        held: u32::try_from(stored.len()).unwrap_or(u32::MAX),
        ..Ledger::default()
    };
    let mut previous = None;
    for &stamp in stored {
        if stamp.rem_euclid(60_000_000) != 0 || previous.is_some_and(|last| stamp <= last) {
            ledger.invalid_timestamps = ledger.invalid_timestamps.saturating_add(1);
        }
        previous = Some(stamp);
    }
    if ledger.invalid_timestamps > 0 {
        // Flooring a bad timestamp to a minute invents evidence; retaining it
        // in the merge stalls the cursor and invents hundreds of later holes.
        // Neither interpretation is a coverage result. Report the bad input.
        return ledger;
    }
    // THE ONE CURSOR. It only ever moves forward, which is what makes the whole
    // walk linear in the two inputs rather than quadratic.
    let mut cursor = 0_usize;
    // The run being accumulated, so contiguous minutes of one reason collapse.
    let mut run: Option<Gap> = None;

    let mut day = first;
    while day <= last {
        // THE SUPPLIED CALENDAR WINS. A caller with a store to derive from has
        // a calendar that widens with it; the table is the fallback for one
        // that does not, and it is bounded by `LAST_DAY`.
        let mut kind = against.map_or_else(|| calendar::kind_of(day), |cal| cal.kind_of(day));
        // THE EXCHANGE SESSION IS KNOWN; THE COMMON INDEX-PUBLICATION SESSION
        // IS NOT. SEBI records NIFTY computation unavailable during part of
        // the still-trading morning, and no source read here states the exact
        // BANKNIFTY or VIX interval. Calling absent index bars vendor holes
        // would therefore invent the denominator this audit exists to supply.
        if matches!(door, Door::SpotIndex) && day == calendar::SYSTEMS_OUTAGE_DAY {
            kind = DayKind::OpenLengthUnmeasured;
        }
        // A SESSION THIS SERIES DID NOT OWE (R4-19), asked of the ONE
        // authority once a day and before any venue's hours refine the day:
        // a share NSE withdrew owes none of a session whose closing-auction
        // eligibility nobody recorded, either. A closed day keeps its word.
        // A day no calendar attests keeps `unmeasured` unless the INSTRUMENT
        // owed nothing on it whatever the exchange did — withdrawn, not yet
        // listed, delisted — which is a sourced fact and more than no claim
        // (IRFC's 2018 months, before its 2021 listing, read before-listing,
        // not unmeasured). `unread` needs a session to be read, so it is not
        // taken there.
        let excused = match kind {
            DayKind::Open(_) | DayKind::OpenLengthUnmeasured => not_owed(key, day),
            DayKind::Unmeasured => not_owed(key, day).filter(|why| !matches!(why, NotOwed::Unread)),
            DayKind::Closed => None,
        };
        if let Door::Cash(schedule) = door {
            kind = cash_kind(day, kind, schedule);
        }
        // THE DERIVATIVES VENUE'S DATED HOURS on a regular day, by the one
        // authority the fold uses (D-1529). Exceptional days keep the
        // calendar's own shape, and a day whose hours this build cannot state
        // is unmeasured rather than given the fixed session.
        if matches!(door, Door::Derivatives) {
            kind =
                crate::fold::minute_session(day, kind, crate::vendor::Venue::NseDerivatives, None)
                    .unwrap_or(DayKind::Unmeasured);
        }
        // ADVANCE PAST ANY STORED BAR BEFORE THIS DAY. A bar the caller handed
        // us from outside the range is skipped rather than counted against it.
        while cursor < stored.len() && stored.get(cursor).copied().is_some_and(|t| ist(t).0 < day) {
            cursor = cursor.saturating_add(1);
        }

        match (kind, excused) {
            // NOT OWED, AND NO WINDOWS TO PLACE IT IN: a session no record
            // sizes (a Muhurat before 2025, a cash day without dated
            // eligibility), or a day no calendar attests on which the
            // instrument owed nothing, that the series did not owe at all.
            // One run of the reason, as `Unmeasured` is one run.
            (DayKind::Unmeasured | DayKind::OpenLengthUnmeasured, Some(why)) => {
                push(&mut ledger, &mut run, day, 0, 1_439, Reason::NotOwed(why));
            }
            // BOTH REPORT `Unmeasured`, AND NEITHER ADDS TO `expected`.
            //
            // `Unmeasured` is a day outside the calendar's range.
            // `OpenLengthUnmeasured` is a Muhurat the exchange traded and the
            // minute series does not reach — five days, every Diwali before
            // 2025. Calling those a `VendorHole` would claim a loss whose SIZE
            // this build cannot state, and calling them `Closed` would deny a
            // session the daily bar proves happened. They are flagged, and the
            // caller decides.
            (DayKind::Unmeasured | DayKind::OpenLengthUnmeasured, None) => {
                push(&mut ledger, &mut run, day, 0, 1_439, Reason::Unmeasured);
            }
            (DayKind::Closed, _) => {
                // A closed day owes nothing, so it produces one run and no
                // expected bars. Reported rather than skipped: an operator
                // asking "where did February go" deserves the answer.
                push(&mut ledger, &mut run, day, 0, 1_439, Reason::Closed);
            }
            (DayKind::Open(session), excused) => {
                // A RECORDED HALT IS NOT OWED, whatever calendar sized the
                // day (R3-06, DB-G47): the record carves it already, but the
                // typed table a cash stock reads, the derivatives venue's
                // hours and a peer vote did not, so the same halted minutes
                // were owed and lost on those doors and not on the record's,
                // while `/calendar.json` and `/dayflags.json` owe none of
                // them. One rule here, in the one classifier every door
                // calls. A carved session expects none of them, so nothing is
                // taken twice. A suspected halt (UNVERIFIED, D-3320) is
                // excluded the same way and named apart by the caller.
                let halt = calendar::halt_on(day).map(|halt| halt.halted);
                let halted = |minute: u16| halt.is_some_and(|window| window.holds(minute));
                let carved = halt.map_or(0_u16, |window| {
                    let held = (window.from..=window.to)
                        .filter(|minute| session.expects(*minute))
                        .count();
                    u16::try_from(held).unwrap_or(u16::MAX)
                });
                // A SESSION NOT OWED OWES NO BAR: nothing of it is expected,
                // and a minute of it with no bar reads why (R4-19).
                if excused.is_none() {
                    ledger.expected = ledger
                        .expected
                        .saturating_add(u32::from(session.bars().saturating_sub(carved)));
                }
                let absent = excused.map_or(Reason::VendorHole, Reason::NotOwed);
                let mut minute = 0_u16;
                while minute <= 1_439 {
                    let held = stored
                        .get(cursor)
                        .copied()
                        .map(ist)
                        .is_some_and(|(d, m)| d == day && m == minute);
                    if held {
                        cursor = cursor.saturating_add(1);
                    } else if session.expects(minute) && !halted(minute) {
                        push(&mut ledger, &mut run, day, minute, minute, absent);
                    } else {
                        push(
                            &mut ledger,
                            &mut run,
                            day,
                            minute,
                            minute,
                            Reason::OutsideWindow,
                        );
                    }
                    minute = minute.saturating_add(1);
                }
            }
        }
        // A RUN NEVER SPANS TWO DAYS. 15:29 on Friday and 09:15 on Monday are
        // not contiguous in any sense an operator cares about.
        flush(&mut ledger, &mut run);
        if day == last {
            break;
        }
        day += 1;
    }
    flush(&mut ledger, &mut run);
    ledger
}

fn cash_kind(day: i64, kind: DayKind, schedule: Option<&crate::cash_auction::Schedule>) -> DayKind {
    let Some(date) = u32::try_from(day)
        .ok()
        .and_then(|number| crate::session::Day::from_days(number).ok())
    else {
        return DayKind::Unmeasured;
    };
    if !crate::vendor::cash_auction_eligibility_required(date) || kind == DayKind::Closed {
        return kind;
    }
    if kind != DayKind::Open(calendar::Session::full()) {
        return DayKind::Unmeasured;
    }
    crate::fold::minute_session(day, kind, crate::vendor::Venue::NseCash, schedule)
        .unwrap_or(DayKind::Unmeasured)
}

/// Extend the open run, or close it and start another.
fn push(ledger: &mut Ledger, run: &mut Option<Gap>, day: i64, from: u16, to: u16, reason: Reason) {
    match run {
        Some(open)
            if open.day == day && open.reason == reason && open.to.saturating_add(1) == from =>
        {
            open.to = to;
        }
        _ => {
            flush(ledger, run);
            *run = Some(Gap {
                day,
                from,
                to,
                reason,
            });
        }
    }
}

/// Commit the open run, respecting [`MAX_GAPS`].
fn flush(ledger: &mut Ledger, run: &mut Option<Gap>) {
    let Some(gap) = run.take() else {
        return;
    };
    if ledger.gaps.len() >= MAX_GAPS {
        // LOUD, NOT SILENT. See `Ledger::truncated`.
        ledger.truncated = true;
        return;
    }
    ledger.gaps.push(gap);
}

/// Minutes this instrument lacks that a peer on the same feed **did** trade.
///
/// # Why a peer is the only witness that costs nothing
///
/// A calendar derived from one instrument cannot tell a scheduled break from a
/// hole: both are simply minutes with no bar. Before D-0420,
/// [`crate::calendar_of`] measured that exactly — it reported 623,546 owed for
/// NIFTY, which is what NIFTY held, so all 28 candidate holes were invisible.
/// The primary-sourced outage correction changes the total, not this general
/// ambiguity; a peer or independent calendar is still required.
///
/// A peer breaks the tie **for free**, because the bars are already on disk. If
/// BANKNIFTY traded 12:41 on a day NIFTY did not, the exchange was open at 12:41
/// and NIFTY is missing a bar. No vendor request, no calendar, no table.
///
/// # What it finds, and what it cannot — measured
///
/// On the operator's store, of the interior gap-minutes in each series:
///
/// | Series | Gap-minutes | Provable by a peer |
/// |---|---|---|
/// | NIFTY | 1,204 | **3** |
/// | BANKNIFTY | 1,252 | **51** |
/// | INDIAVIX | 1,203 | **2** |
/// | *common to all three* | *1,201* | *0 — see below* |
///
/// **1,201 of them are shared by all three**, and this function is silent about
/// every one. That is not a weakness in the rule; it is what the evidence
/// supports. A minute no instrument on the feed traded is either a scheduled
/// break or a feed-wide outage, and the bars cannot say which — 2023-06-14
/// 12:41–12:47 is missing from all three identically, which is the signature of
/// Zerodha's pipeline stopping rather than of three separate holes.
///
/// **Only another VENDOR settles those.** If Dhan or Groww holds 12:41 that day,
/// the exchange was open and Zerodha lost it.
///
/// So this is a strict lower bound on loss, and it is the right direction: it
/// never reports a hole that is not one. BANKNIFTY's 51 are real bars, absent,
/// and reported by nothing before this.
///
/// # Cost
///
/// One pass over the day's minutes, and one `expects` per peer per minute —
/// each a bounded walk of at most [`crate::calendar::MAX_WINDOWS`]. O(1) per
/// minute per peer, no allocation beyond the answer.
///
/// **UNVERIFIED as a measurement** — see the module's own Cost note. The per-peer
/// factor is real and is stated rather than folded into the O(1): this is
/// constant per minute PER PEER, so three feeds cost three walks. The bound on
/// each walk is `MAX_WINDOWS`, a `const`, which is what makes the inner one
/// constant rather than a search.
#[must_use]
pub fn provable_holes(
    day: i64,
    mine: Option<crate::calendar::Session>,
    peers: &[Option<crate::calendar::Session>],
) -> Vec<Gap> {
    let mut out = Vec::new();
    let mut run: Option<Gap> = None;
    let mut ledger = Ledger::default();
    for minute in 0..=1_439_u16 {
        let i_traded = mine.is_some_and(|s| s.expects(minute));
        // A PEER IS A WITNESS, AND ONE IS ENOUGH. Requiring a majority would
        // discard the case this exists for: a hole in two of three series is
        // still a hole in both.
        let peer_traded = peers
            .iter()
            .any(|peer| peer.is_some_and(|s| s.expects(minute)));
        if !i_traded && peer_traded {
            push(
                &mut ledger,
                &mut run,
                day,
                minute,
                minute,
                Reason::VendorHole,
            );
        } else {
            flush(&mut ledger, &mut run);
        }
    }
    flush(&mut ledger, &mut run);
    out.append(&mut ledger.gaps);
    out
}

#[cfg(test)]
#[allow(clippy::expect_used, clippy::panic, reason = "test-only assertions")]
mod tests {
    use super::*;

    /// Micros-since-epoch for a day and an IST minute-of-day.
    fn at(day: i64, minute: u16) -> i64 {
        const IST_OFFSET_SECS: i64 = 5 * 3600 + 30 * 60;
        (day * 86_400 + i64::from(minute) * 60 - IST_OFFSET_SECS) * 1_000_000
    }

    /// Every bar a full session owes, for one day.
    fn full_day(day: i64) -> Vec<i64> {
        (calendar::OPEN_MINUTE..=calendar::LAST_MINUTE)
            .map(|m| at(day, m))
            .collect()
    }

    /// R3-06, DB-G47: the 2020-03-13 market-wide halt's minutes are owed by
    /// no door. Before, the record carved them and the table (an NSE cash
    /// stock), the derivatives venue's hours and the spot-index door counted
    /// 59 of them lost from a series that held every other minute.
    #[test]
    fn a_recorded_halt_is_not_owed_by_any_classifier_door() {
        let day = 18_334; // 2020-03-13
        let halt = calendar::halt_on(day).expect("2020-03-13 is a recorded halt");
        let held: Vec<i64> = full_day(day)
            .into_iter()
            .filter(|stamp| !halt.halted.holds(ist(*stamp).1))
            .collect();
        let doors: [(&str, Ledger); 4] = [
            ("table", classify(&held, day, day)),
            (
                "spot index",
                classify_spot_index_against(&held, day, day, None),
            ),
            (
                "derivatives",
                classify_derivative_against(&held, day, day, None),
            ),
            ("cash", classify_cash(&held, day, day, None, &Key::NONE)),
        ];
        let owed = 375 - u32::from(halt.halted.bars());
        for (door, ledger) in doors {
            assert_eq!(ledger.lost_minutes(), 0, "{door}: {:?}", ledger.gaps);
            assert_eq!(ledger.expected, owed, "{door}");
            assert!(
                ledger
                    .gaps
                    .iter()
                    .any(|gap| gap.reason == Reason::OutsideWindow
                        && gap.from == halt.halted.from
                        && gap.to == halt.halted.to),
                "{door}: the halt is one run outside the window: {:?}",
                ledger.gaps
            );
        }
        // A minute lost OUTSIDE the halt is still lost, on every door.
        let short: Vec<i64> = held.iter().copied().filter(|s| ist(*s).1 != 900).collect();
        assert_eq!(classify(&short, day, day).lost_minutes(), 1);
        // And a day with no halt owes its whole session as before.
        assert_eq!(classify(&full_day(day - 1), day - 1, day - 1).expected, 375);
    }

    /// audit-20261003 hunt-pull-1 (D-1529). An NSE derivatives series is
    /// audited against the derivatives venue's DATED hours, not the fixed
    /// 375-minute session. From 2026-08-03 the venue closes at 15:40, so a day
    /// owes 385 minutes and a vendor that dropped 15:30..=15:39 lost ten.
    /// `fold::complete_minutes_for_venue` already withheld that day; the gap
    /// audit reported it clean.
    #[test]
    fn a_derivatives_audit_owes_the_venues_dated_minutes() {
        let day = 20_669; // 2026-08-04, after the 2026-08-03 extension
        let whole: Vec<i64> = (555..940).map(|minute| at(day, minute)).collect();
        let full = classify_derivative_against(&whole, day, day, None);
        assert_eq!(full.expected, 385, "the extended session owes 385 minutes");
        assert_eq!(full.held, 385);
        assert_eq!(full.lost_minutes(), 0);

        let short: Vec<i64> = (555..930).map(|minute| at(day, minute)).collect();
        let lost = classify_derivative_against(&short, day, day, None);
        assert_eq!(lost.expected, 385);
        assert_eq!(
            lost.lost_minutes(),
            10,
            "15:30..=15:39 are owed by the venue and were not delivered"
        );
        assert!(
            lost.gaps.iter().any(|gap| gap.day == day
                && gap.from == 930
                && gap.to == 939
                && gap.reason == Reason::VendorHole),
            "the ten minutes are one vendor-hole run: {:?}",
            lost.gaps
        );

        // Before the extension the derivatives session is the 375-minute one.
        let before = 20_663; // 2026-07-29
        let old: Vec<i64> = (555..930).map(|minute| at(before, minute)).collect();
        let pre = classify_derivative_against(&old, before, before, None);
        assert_eq!(pre.expected, 375);
        assert_eq!(pre.lost_minutes(), 0);
    }

    #[test]
    fn stored_minute_audit_rejects_off_grid_duplicate_and_backward_timestamps() {
        let day = 20668;
        let exact = full_day(day);
        let checked = classify(&exact, day, day);
        assert_eq!(checked.expected, 375);
        assert_eq!(checked.invalid_timestamps, 0);
        assert_eq!(checked.lost_minutes(), 0);
        let shifted: Vec<_> = exact.iter().map(|stamp| stamp + 30_000_000).collect();
        let broken = classify(&shifted, day, day);
        assert_eq!(broken.invalid_timestamps, 375);
        assert_eq!(
            broken.expected, 0,
            "coverage withheld, not falsely complete"
        );
        let mut same_minute = exact.clone();
        same_minute.insert(1, at(day, 555) + 30_000_000);
        let repeated = classify(&same_minute, day, day);
        assert_eq!(repeated.invalid_timestamps, 1);
        assert_eq!(repeated.lost_minutes(), 0, "do not invent 374 later gaps");
        *same_minute.get_mut(1).expect("inserted timestamp") = at(day, 555);
        assert_eq!(classify(&same_minute, day, day).invalid_timestamps, 1);
        let mut backward = exact;
        backward.swap(0, 1);
        assert_eq!(classify(&backward, day, day).invalid_timestamps, 1);
    }

    #[test]
    fn cash_auction_flags_are_dated_and_missing_metadata_is_not_a_clean_day() {
        let eligible = crate::session::Day::new(2026, 8, 27).expect("date");
        let ordinary = crate::session::Day::new(2026, 8, 28).expect("date");
        let first = i64::from(eligible.days_from_epoch());
        let last = i64::from(ordinary.days_from_epoch());
        let mut schedule = crate::cash_auction::Schedule::default();
        schedule.insert(eligible, true).expect("exact flag");
        schedule
            .insert(ordinary, false)
            .expect("next day's exact flag");
        let mut bars: Vec<_> = (555..915).map(|minute| at(first, minute)).collect();
        bars.extend(
            (555..930)
                .filter(|minute| *minute != 763)
                .map(|minute| at(last, minute)),
        );
        let checked = classify_cash(&bars, first, last, Some(&schedule), &Key::NONE);
        assert_eq!(checked.held, 734);
        assert_eq!(checked.expected, 735);
        assert_eq!(checked.lost_minutes(), 1);
        assert!(checked.gaps.iter().any(|gap| gap.day == last
            && gap.from == 763
            && gap.to == 763
            && gap.reason == Reason::VendorHole));
        assert!(
            !checked
                .gaps
                .iter()
                .any(|gap| gap.day == first && gap.reason == Reason::VendorHole)
        );
        let unknown = classify_cash(&bars, first, last, None, &Key::NONE);
        assert_eq!(unknown.expected, 0);
        assert_eq!(unknown.lost_minutes(), 0);
        assert_eq!(unknown.held, 734);
        assert_eq!(unknown.unmeasured_minutes(), 2880);
        let mut one_date = crate::cash_auction::Schedule::default();
        one_date
            .insert(eligible, true)
            .expect("only one day verified");
        let partial = classify_cash(&bars, first, last, Some(&one_date), &Key::NONE);
        assert_eq!(partial.expected, 360);
        assert_eq!(
            partial.unmeasured_minutes(),
            1440,
            "never carry yesterday's flag forward"
        );
    }

    #[test]
    fn cash_ineligible_tail_and_entire_absent_day_remain_expected_gaps() {
        let date = crate::session::Day::new(2026, 8, 27).expect("date");
        let day = i64::from(date.days_from_epoch());
        let mut schedule = crate::cash_auction::Schedule::default();
        schedule.insert(date, false).expect("explicit ineligible");
        let prefix: Vec<_> = (555..915).map(|minute| at(day, minute)).collect();
        let tail = classify_cash(&prefix, day, day, Some(&schedule), &Key::NONE);
        assert_eq!(tail.expected, 375);
        assert_eq!(tail.lost_minutes(), 15);
        assert!(
            tail.gaps
                .iter()
                .any(|gap| gap.from == 915 && gap.to == 929 && gap.reason == Reason::VendorHole)
        );
        let absent = classify_cash(&[], day, day, Some(&schedule), &Key::NONE);
        assert_eq!(absent.expected, 375);
        assert_eq!(absent.lost_minutes(), 375);
    }

    #[test]
    fn cash_audit_preserves_pre_change_sessions_and_never_promotes_unknown_days() {
        let pre = i64::from(
            crate::session::Day::new(2026, 7, 31)
                .expect("date")
                .days_from_epoch(),
        );
        let normal = classify_cash(&full_day(pre), pre, pre, None, &Key::NONE);
        assert_eq!(normal.expected, 375);
        assert_eq!(normal.lost_minutes(), 0);
        let holiday = classify_cash(&[], 20479, 20479, None, &Key::NONE);
        assert_eq!(holiday.expected, 0);
        assert_eq!(holiday.unmeasured_minutes(), 0);
        let future = calendar::LAST_DAY + 1;
        let mut schedule = crate::cash_auction::Schedule::default();
        schedule
            .insert(
                crate::session::Day::from_days(u32::try_from(future).expect("day")).expect("date"),
                false,
            )
            .expect("flag");
        let unknown = classify_cash(
            &full_day(future),
            future,
            future,
            Some(&schedule),
            &Key::NONE,
        );
        assert_eq!(
            unknown.expected, 0,
            "a master is not a trading-calendar extension"
        );
        assert_eq!(unknown.unmeasured_minutes(), 1440);
        let dr = classify_cash(&[], 19784, 19784, None, &Key::NONE);
        assert_eq!(
            dr.expected, 105,
            "verified pre-change exceptional windows remain intact"
        );
        assert_eq!(dr.lost_minutes(), 105);
        for outside in [i64::MIN, -1, i64::MAX] {
            let invalid_day = classify_cash(&[], outside, outside, None, &Key::NONE);
            assert_eq!(invalid_day.expected, 0);
            assert_eq!(invalid_day.unmeasured_minutes(), 1440);
        }
    }

    /// **A COMPLETE DAY REPORTS NOTHING MISSING — WHICH IS THE WHOLE POINT.**
    ///
    /// The defect this module exists to fix is a report that cried wolf on
    /// complete data. If a full 375-bar session produced a single gap, every
    /// alarm downstream would be noise again.
    #[test]
    fn a_complete_session_reports_no_loss_and_no_outside_window_noise() {
        // 2026-08-03, an ordinary Monday.
        let day = 20_668;
        let ledger = classify(&full_day(day), day, day);
        assert_eq!(ledger.held, 375);
        assert_eq!(ledger.expected, 375);
        assert_eq!(ledger.lost_minutes(), 0, "nothing was lost");
        assert!(
            ledger
                .gaps
                .iter()
                .all(|g| g.reason == Reason::OutsideWindow),
            "the only absences on a complete day are the 1,065 minutes outside \
             09:15-15:29, which are not losses"
        );
        assert!(!ledger.truncated);
    }

    /// **THE 2023-06-14 HOLE, REPRODUCED FROM THE OPERATOR'S OWN STORE.**
    ///
    /// Seven consecutive minutes at 12:41 were absent from a 368-bar day. They
    /// are mid-session, so nothing dropped them — the vendor did not send them —
    /// and they must be reported as the one reason that is a loss, collapsed
    /// into a single run rather than seven rows.
    #[test]
    fn a_mid_session_hole_is_one_run_of_vendor_hole_and_not_seven_rows() {
        let day = 19_522; // 2023-06-14
        let missing = 12 * 60 + 41;
        let bars: Vec<i64> = (calendar::OPEN_MINUTE..=calendar::LAST_MINUTE)
            .filter(|m| !(missing..missing + 7).contains(m))
            .map(|m| at(day, m))
            .collect();
        assert_eq!(bars.len(), 368, "the count the store actually holds");

        let ledger = classify(&bars, day, day);
        assert_eq!(ledger.held, 368);
        assert_eq!(ledger.expected, 375);
        assert_eq!(ledger.lost_minutes(), 7);

        let losses: Vec<&Gap> = ledger.gaps.iter().filter(|g| g.reason.is_loss()).collect();
        assert_eq!(losses.len(), 1, "ONE run, not seven rows");
        let hole = losses.first().expect("the run");
        assert_eq!(hole.from, missing, "12:41");
        assert_eq!(hole.to, missing + 6, "through 12:47");
        assert_eq!(hole.minutes(), 7);
    }

    /// **A MUHURAT HOUR IS NOT 315 MISSING BARS.**
    ///
    /// 2025-10-21 traded 13:45–14:44 and nothing else. An arithmetic count
    /// expecting 375 reports 315 losses; this must report **zero**, because the
    /// exchange owed sixty bars and delivered sixty.
    #[test]
    fn a_muhurat_session_owes_sixty_bars_and_loses_none() {
        let day = 20_382;
        let bars: Vec<i64> = (13 * 60 + 45..=14 * 60 + 44).map(|m| at(day, m)).collect();
        assert_eq!(bars.len(), 60);

        let ledger = classify(&bars, day, day);
        assert_eq!(ledger.expected, 60, "sixty owed, not 375");
        assert_eq!(ledger.held, 60);
        assert_eq!(
            ledger.lost_minutes(),
            0,
            "a short session is not a short store — this is the false alarm the \
             module exists to stop"
        );
    }

    /// **THE TWO-WINDOW SATURDAY HAS A HOLE THAT IS NOT A GAP.**
    ///
    /// 2024-03-02 traded 09:15–09:59 and 11:30–12:29. The ninety minutes
    /// between are `OutsideWindow` — never owed — while a minute absent from
    /// either window would be a loss. Both must be distinguishable on one day.
    #[test]
    fn the_gap_between_two_windows_is_not_a_loss_but_a_hole_inside_one_is() {
        let day = 19_784;
        let mut bars: Vec<i64> = (555..=599).map(|m| at(day, m)).collect();
        bars.extend((690..=749).map(|m| at(day, m)));
        assert_eq!(bars.len(), 105);

        let clean = classify(&bars, day, day);
        assert_eq!(clean.expected, 105);
        assert_eq!(clean.lost_minutes(), 0, "the midday break is not a loss");
        assert!(
            clean
                .gaps
                .iter()
                .any(|g| g.reason == Reason::OutsideWindow && g.from == 600),
            "and it IS reported, as outside-window: {:?}",
            clean.gaps
        );

        // NOW REMOVE ONE BAR FROM INSIDE THE SECOND WINDOW.
        let holed: Vec<i64> = bars
            .iter()
            .copied()
            .filter(|t| *t != at(day, 700))
            .collect();
        let ledger = classify(&holed, day, day);
        assert_eq!(ledger.lost_minutes(), 1, "and that one IS a loss");
    }

    /// **A CLOSED DAY AND AN UNMEASURED DAY ARE DIFFERENT ANSWERS.**
    ///
    /// Reporting "nobody has looked" as "the exchange was shut" is the
    /// invention `CLAUDE.md` §3 rule 1 bans, and it is exactly what an operator
    /// backfilling 2015 would be told by a calendar that guessed.
    #[test]
    fn a_holiday_and_an_unmeasured_day_do_not_share_a_reason() {
        // 2026-01-26, Republic Day — inside the calendar, closed.
        let holiday = classify(&[], 20_479, 20_479);
        assert_eq!(holiday.expected, 0, "a closed day owes nothing");
        assert_eq!(holiday.lost_minutes(), 0);
        assert_eq!(holiday.gaps.first().map(|g| g.reason), Some(Reason::Closed));

        // 2015-01-01 — before anything was measured.
        let unmeasured = classify(&[], 16_436, 16_436);
        assert_eq!(unmeasured.expected, 0);
        assert_eq!(unmeasured.lost_minutes(), 0, "not a loss — a non-claim");
        assert_eq!(
            unmeasured.gaps.first().map(|g| g.reason),
            Some(Reason::Unmeasured),
            "and NOT Closed"
        );
    }

    /// **AN EXCHANGE SESSION DOES NOT INVENT A SPOT-INDEX PUBLICATION WINDOW.**
    ///
    /// SEBI proves 220 normal-market minutes on 2021-02-24 and also records
    /// NIFTY computation unavailable during part of the morning. BANKNIFTY and
    /// VIX publication windows are not established by that record. The equity
    /// session can therefore be sized while a common index-bar denominator
    /// must remain unmeasured; the store's shared 54-bar prefix is neither
    /// allowed to call itself complete nor labelled as 166 vendor losses.
    #[test]
    fn the_outage_day_separates_market_minutes_from_unverified_index_bars() {
        let day = calendar::SYSTEMS_OUTAGE_DAY;
        let stored: Vec<i64> = (555..=608).map(|minute| at(day, minute)).collect();
        let observed = calendar::Observed::from_runs(day, &[(555, 608)]);
        let derived = calendar::Calendar::from_observed(&[observed]);

        let exchange = classify_against(&stored, day, day, Some(&derived));
        assert_eq!(exchange.expected, 220, "the SEBI-recorded market session");
        assert_eq!(exchange.lost_minutes(), 166, "market minutes not stored");

        let index = classify_spot_index_against(&stored, day, day, Some(&derived));
        assert_eq!(index.held, 54, "stored evidence remains visible");
        assert_eq!(index.expected, 0, "no invented common index denominator");
        assert_eq!(index.lost_minutes(), 0, "no invented vendor-hole count");
        assert_eq!(index.unmeasured_minutes(), 1_440, "the whole day is named");
        assert_eq!(
            index.gaps.first().map(|gap| gap.reason),
            Some(Reason::Unmeasured)
        );
    }

    /// **A SUPPLIED CALENDAR OVERRIDES THE TABLE, AND THAT IS THE WHOLE POINT
    /// OF `classify_against`.**
    ///
    /// The table is bounded by [`calendar::LAST_DAY`], and on 2026-08-29 — eight
    /// days past it — an audit of NIFTY at one minute for 2026-08 answered
    /// `expected 5,625, held 7,500, lost 0`: twenty trading days of bars against
    /// fifteen the table could vouch for. Nothing was wrong with the store and
    /// nothing was wrong with the classification; the DENOMINATOR was reading a
    /// calendar that had run out.
    ///
    /// `api::calendar_of` derives this same type from the store's own rungs and
    /// widens by itself, so a caller with a store passes one. Driven here on a
    /// day the TABLE calls `Unmeasured`, because that is the case where the two
    /// sources give different answers and the supplied one must win — a test on
    /// a day they agree about would pass with the parameter ignored entirely.
    #[test]
    fn a_supplied_calendar_answers_for_a_day_the_table_has_never_heard_of() {
        // One day past the table's last, so the table alone can only shrug.
        let past = calendar::LAST_DAY + 1;
        let table_only = classify(&[], past, past);
        assert_eq!(
            table_only.gaps.first().map(|g| g.reason),
            Some(Reason::Unmeasured),
            "the table has nothing to say about a day it does not cover"
        );
        assert_eq!(
            table_only.expected, 0,
            "so it owes nothing, which is what makes `held` exceed `expected`"
        );

        // A derived calendar that DID see that day trade a full session.
        let observed =
            calendar::Observed::from_runs(past, &[(calendar::OPEN_MINUTE, calendar::LAST_MINUTE)]);
        let derived = calendar::Calendar::from_observed(&[observed]);
        let against = classify_against(&[], past, past, Some(&derived));
        assert_eq!(
            against.expected,
            u32::from(calendar::FULL_BARS),
            "the supplied calendar owes a full session where the table owed nothing"
        );
        assert_eq!(
            against.lost_minutes(),
            u32::from(calendar::FULL_BARS),
            "and an empty store against a day that traded is a real loss, which \
             the table could not have called"
        );
        assert_eq!(
            against.unmeasured_minutes(),
            0,
            "nothing is unclaimed once a calendar can speak for the day"
        );
    }

    /// **TRUNCATION IS ANNOUNCED, NEVER SILENT.**
    ///
    /// A ledger cut short that read like a complete one would tell an operator
    /// their store is healthier than it is — the §4 failure wearing a success's
    /// clothes, on the very report built to prevent it.
    #[test]
    fn a_truncated_ledger_says_so() {
        // Every day of the calendar, holding nothing: far past MAX_GAPS.
        let ledger = classify(&[], calendar::FIRST_DAY, calendar::LAST_DAY);
        assert!(ledger.truncated, "the walk hit the bound");
        assert_eq!(ledger.gaps.len(), MAX_GAPS, "and stopped exactly there");
    }

    /// **A PEER THAT TRADED IS A WITNESS, AND ONE IS ENOUGH.**
    ///
    /// The case this exists for: BANKNIFTY holds 51 gap-minutes that NIFTY and
    /// INDIAVIX do not share, measured on the operator's store. Every one is a
    /// bar a peer offered evidence for and this series did not deliver, and
    /// nothing reported them before. A single-instrument calendar remains blind
    /// to its own holes by construction; D-0420's one primary-sourced day does
    /// not remove that general ambiguity.
    #[test]
    fn a_minute_a_peer_traded_and_this_one_did_not_is_a_provable_hole() {
        let full = crate::calendar::Session::full();
        // MINE IS MISSING 12:41-12:47; the peer holds the whole session.
        let mine = crate::calendar::Observed::from_runs(19_522, &[(555, 760), (768, 929)]).session;

        let holes = provable_holes(19_522, mine, &[Some(full)]);
        assert_eq!(holes.len(), 1, "ONE run, not seven rows: {holes:?}");
        let hole = holes.first().expect("the run");
        assert_eq!(hole.from, 761, "12:41");
        assert_eq!(hole.to, 767, "through 12:47");
        assert_eq!(hole.minutes(), 7);
        assert_eq!(hole.reason, Reason::VendorHole);
        assert!(hole.reason.is_loss());
    }

    /// **A MINUTE NO PEER TRADED IS NOT REPORTED, AND THAT IS THE POINT.**
    ///
    /// 1,201 of the operator's gap-minutes are shared by all three series —
    /// 2023-06-14 12:41–12:47 is missing from every one of them identically,
    /// which is Zerodha's pipeline stopping rather than three separate holes.
    /// The bars cannot distinguish that from a scheduled break, so this
    /// function says nothing about it.
    ///
    /// Silence is the safe direction: it never reports a hole that is not one.
    /// Only another VENDOR settles those minutes.
    #[test]
    fn a_minute_no_peer_traded_is_silence_and_not_a_guess() {
        let broken =
            crate::calendar::Observed::from_runs(19_522, &[(555, 760), (768, 929)]).session;

        // EVERY PEER HAS THE SAME GAP — a feed-wide outage.
        let holes = provable_holes(19_522, broken, &[broken, broken]);
        assert!(
            holes.is_empty(),
            "a gap every series shares is not provable from bars alone: {holes:?}"
        );

        // AND THE MIDDAY BREAK OF A DR SATURDAY IS THE SAME SHAPE. All three
        // stop at 09:59 and resume at 11:30, so nothing is claimed.
        let dr = crate::calendar::Observed::from_runs(19_784, &[(555, 599), (690, 749)]).session;
        assert!(provable_holes(19_784, dr, &[dr, dr]).is_empty());
    }

    /// **A DAY THIS SERIES HAS NO MINUTE BARS FOR IS EVERY PEER MINUTE.**
    ///
    /// The five pre-2025 Muhurats arrive as `None`. Where a peer DID trade that
    /// hour, every minute of it is a provable hole; where no peer did — which is
    /// the real case, since Zerodha's minute history reaches none of them — the
    /// answer is silence.
    #[test]
    fn a_day_with_no_minute_series_is_all_hole_or_all_silence() {
        let hour = crate::calendar::Observed::from_runs(20_028, &[(825, 884)]).session;

        // A peer traded the Muhurat hour and this series holds nothing.
        let holes = provable_holes(20_028, None, &[hour]);
        assert_eq!(holes.len(), 1);
        assert_eq!(holes.first().map(|g| g.minutes()), Some(60));

        // No peer traded it either — the operator's actual case.
        assert!(provable_holes(20_028, None, &[None, None]).is_empty());
    }

    /// **A STORED BAR OUTSIDE THE DAY BEING AUDITED IS STEPPED PAST, NOT HELD
    /// AGAINST IT.**
    ///
    /// Written because `cargo mutants` showed that replacing the cursor's
    /// `cursor < stored.len()` with `cursor == stored.len()` survived the whole
    /// suite. Under that mutant one bar before the range, or one bar on a closed
    /// day, stalls the cursor, and every minute of the next open session is
    /// lost. GAP14-60.
    #[test]
    fn a_bar_before_the_range_or_on_a_closed_day_does_not_stall_the_next_session() {
        // 2026-01-26 is Republic Day (closed, see the test above); 2026-01-27
        // is an ordinary session.
        let holiday = 20_479;
        let next = 20_480;
        let mut stored = vec![at(holiday, 600)];
        stored.extend(full_day(next));
        let ledger = classify(&stored, holiday, next);
        assert_eq!(ledger.invalid_timestamps, 0);
        assert_eq!(ledger.held, 376, "the holiday bar is still counted as held");
        assert_eq!(ledger.expected, 375, "the holiday owes nothing");
        assert_eq!(ledger.lost_minutes(), 0, "the next session lost nothing");
        assert!(
            ledger.gaps.iter().all(|g| g.reason != Reason::VendorHole),
            "no minute of the open session is a hole"
        );
        assert_eq!(
            ledger.gaps.first().map(|g| (g.day, g.reason)),
            Some((holiday, Reason::Closed))
        );

        // One bar the day before `first`, then the whole session of `first`.
        let day = 20_668; // 2026-08-03, an ordinary Monday.
        let mut early = vec![at(day - 1, 600)];
        early.extend(full_day(day));
        let ledger = classify(&early, day, day);
        assert_eq!(ledger.invalid_timestamps, 0);
        assert_eq!(ledger.held, 376);
        assert_eq!(ledger.expected, 375);
        assert_eq!(
            ledger.lost_minutes(),
            0,
            "a bar outside the range is skipped"
        );
        assert!(
            ledger
                .gaps
                .iter()
                .all(|g| g.reason == Reason::OutsideWindow),
            "the only absences are the minutes outside the session"
        );
    }
}

#[cfg(test)]
#[path = "gaps_not_owed_tests.rs"]
mod not_owed_tests;
