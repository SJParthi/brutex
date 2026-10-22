//! Point-in-time F&O membership: which SHARES had a listed stock derivative on
//! each trading day, derived from our own GDFL options listings, kept as
//! sealed day records in append-only month files. D-5160 to D-5168 (PIT-1).
//!
//! # Why this exists
//!
//! `brutex_core::universe::FNO_UNDERLYINGS` is ONE snapshot (2026-08-01, 213
//! names). Ranking 2018-2026 movers inside it is survivorship plus look-ahead:
//! a share added to F&O in 2024 enters the 2019 denominator, and a share that
//! left in 2020 is missing from it. This module answers, for a day `D`, the
//! sorted list of series that had at least one live stock-option contract on
//! `D`, and refuses, by name, every day it cannot answer. The snapshot
//! constant is not edited: it stays the swept surface `CLAUDE.md` §1 names.
//!
//! # The rule, day by day (D-5161, D-5166)
//!
//! A contract's LIFE is `[the first IST day any record of it is stored, its
//! settled expiry]`: the first day is the first trade-day listing that holds a
//! file of it (an UPPER bound on the exchange's listing day: a listing day with
//! no stored record is invisible, so membership may start late, measured per
//! series as [`Spelling::listing_lag_days`]); the settled expiry is
//! [`crate::gdfl_nfo::decode_ticker`]'s. A share is a MEMBER on `D` when one of
//! its contracts' lives covers `D`, whether or not that contract has a file on
//! `D`. The state one day hands the next is one number per live series, the
//! latest settled expiry seen so far, so a day computes only itself from it.
//!
//! * A contract listed but untraded on `D` still counts (its life covers `D`);
//!   [`Member::filed`] says whether the share listed a file that day.
//! * An expired contract does not: `live_until < D` drops the share.
//! * A rename is ONE series: the decoder files every ticker under the newest
//!   symbol NSE's dated changes reach (`brutex_core::alias::LINEAGE`,
//!   sourced), so `ZOMATO` and `ETERNAL` are one spelling. A rename the table
//!   does not hold (`TATAGLOBAL -> TATACONSUM`, D-3354) is caught as a
//!   MEASURED HANDOVER ([`Handover`]): on a session where a live series lists
//!   no file under any spelling, and a spelling never seen before lists keys
//!   (expiry, strike, side) of which the ones it shares with the series'
//!   previous-session listing are a strict majority of BOTH sides and at
//!   least [`MIN_HANDOVER_KEYS`], the new spelling joins the series (same
//!   [`Member::series_id`]) and becomes its newest spelling. A pair either
//!   side of which matches a second candidate is not joined, and so is every
//!   pair the old-side majority alone would have joined (reported, by name,
//!   with why).
//! * A series is live under ONE spelling, its newest: a file listed later
//!   under a spelling the series moved on from (a stray) is filed under the
//!   newest one and reported by name, never a second member and never a stop
//!   (D-5166).
//! * Index underlyings (`brutex_core::alias::option_underlying`) are not
//!   shares: counted per day, never members.
//!
//! # The span and what is refused (D-5162, D-5168)
//!
//! `known_from` and `known_to` are the first and last IST day whose source
//! listing holds a share option: measured by the build's own pass, never
//! typed, written into every month header (`known_from`) and the last known
//! record (`known_to`). Every day outside them is refused as
//! [`MembershipUnknown`] ("membership unknown"), and so is every day inside
//! them without an answer (a session with no day file, one listing no share
//! option, a closed day). Nothing is ever filled from today's list. Refused
//! sessions inside the span are running sums per year, indexed by day
//! ([`Membership::refused_through`]). A day whose day file the SOURCE refused
//! is never sealed: a refusal may be transient, so the build installs every
//! record before it and stops, and the next build resumes at it (D-5168).
//!
//! # The file (D-5163, `docs/02-store-format.md` "F&O membership month files")
//!
//! `<dir>/<YYYY-MM>-g<GGGGGG>.fnm`, one per month and generation, holding the
//! sealed records of that month's days from its first to its last record. A
//! build resumes after the last sealed record and runs to `known_to`; a sealed
//! record is never rewritten (an overlap is refused). Extending a month writes
//! it as the next generation beside the old one; the generation is in the name
//! only, so one pass and a day-by-day build write the same bytes. Any prefix
//! of a build's installs is a servable, resumable directory: the last month
//! may end on the closed days after its last session (D-5167).
//!
//! # The reader (D-5165)
//!
//! [`Membership::members`] answers a day with its [`Member`]s sorted by a
//! series id stable across renames, each with the spelling filed that day (the
//! store key of its cash series) and whether its series is a share of today's
//! 2026-08-01 snapshot; [`Membership::max_pool`] sizes a rank network and
//! [`Membership::file_digest`] names the membership a run read. This is the
//! API R4-26 adopts unchanged.
//!
//! # Cost (D-5164)
//!
//! [`Membership::members`], [`Membership::pool_size`], [`Membership::row`],
//! [`Membership::refused_through`] and [`Membership::is_member`] do one
//! subtraction into a fixed-stride day index and one slice or row read;
//! `is_member` adds a bounded alias walk, one hash probe and one bit. That
//! they stay flat at 1x, 10x and 100x the held days, on known days and on
//! refused ones, is C-PIT-01, measured by
//! `pull::bench::fno_membership_lookup_is_flat` in
//! `crates/pull/benches/ratio.rs`, which prints p50, p99 and max beside it.
//! Building reads each day's listing once, O(entries of that day) plus
//! O(pool) to fold.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use brutex_core::alias::{LINEAGE, option_underlying};
use brutex_core::instrument::{OptionSide, Segment};
use brutex_core::symbol::{SYMBOL_CAPACITY, Symbol};
use brutex_core::universe::{FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS};

use crate::calendar::{DayKind, SOURCED_FIRST_DAY, SOURCED_LAST_DAY, sourced_kind_of};
use crate::gdfl_nfo::{NfoSource, decode_ticker};
use crate::session::Day;

/// The decision that defines a member (`docs/05-decisions.md`).
pub const DECISION: &str = "D-5160";
/// The label every row derived here carries: inferred from our own GDFL
/// options listings, not read from an NSE circular. R4-26's `source` column.
pub const SOURCE_LABEL: &str = "inferred_gdfl_options";
/// The label of a [`Handover`].
pub const HANDOVER_LABEL: &str = "measured";
/// What a report shows for intraday leverage (D-5160 item 2): no dated,
/// charter-sourced margin source is held, so no figure is shown.
pub const LEVERAGE: &str =
    "leverage unknown (UNVERIFIED: no dated, charter-sourced margin source; D-5160)";
/// The first eight bytes of every membership file.
pub const MAGIC: [u8; 8] = *b"BRXFNM01";
/// The one format version this reader knows.
pub const VERSION: u16 = 1;
/// The file extension, without the dot.
pub const EXTENSION: &str = "fnm";
/// The directory name the real build writes under `<store>/reference/`.
pub const DIR_NAME: &str = "fno-membership-v1";
/// Header bytes.
pub const HEADER_LEN: usize = 72;
/// Bytes of one name: [`SYMBOL_CAPACITY`], zero padded.
pub const NAME_LEN: usize = SYMBOL_CAPACITY;
/// Bytes of one spelling record.
pub const SPELLING_LEN: usize = NAME_LEN + 16;
/// Bytes of one day row.
pub const DAY_ROW_LEN: usize = 52;
/// Bytes of one member entry.
pub const ENTRY_LEN: usize = 4;
/// Bytes of one live-spelling carry record.
pub const LIVE_LEN: usize = 8;
/// Bytes of one expiry-start carry record.
pub const EXPIRY_LEN: usize = 8;
/// Bytes of one handover record.
pub const HANDOVER_LEN: usize = 20;
/// Bytes of the trailing BLAKE3 digest.
pub const DIGEST_LEN: usize = 32;
/// The most spellings one membership may name: a FORMAT bound (a member entry
/// holds a `u16`), refused loudly past it, never truncated.
pub const MAX_SPELLINGS: usize = 4_096;
/// How many refused ticker names one day keeps verbatim for its report; every
/// refusal is still counted.
pub const REFUSAL_SAMPLE: usize = 8;
/// The member entry flag: the share listed at least one file that day.
const FILED: u16 = 1;

/// Why a day has, or has not, an answer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DayStatus {
    /// A session with a valid options day file listing share options.
    Known = 1,
    /// Not a session: the calendar says the exchange was closed.
    NotSession = 2,
    /// A session with no options day file in the source.
    NoSource = 3,
    /// A session whose day file the source refused.
    SourceRefused = 4,
    /// A day the IST calendar does not cover.
    NoCalendar = 5,
    /// A session whose day file lists no share option at all.
    NoShareOptions = 6,
    /// A day file on a day the calendar says was closed.
    SourceOnClosedDay = 7,
}

impl DayStatus {
    /// Every status, in code order.
    pub const ALL: [Self; 7] = [
        Self::Known,
        Self::NotSession,
        Self::NoSource,
        Self::SourceRefused,
        Self::NoCalendar,
        Self::NoShareOptions,
        Self::SourceOnClosedDay,
    ];

    /// The status a byte names.
    #[must_use]
    pub fn from_code(code: u8) -> Option<Self> {
        Self::ALL.into_iter().find(|status| *status as u8 == code)
    }

    /// One word for reports.
    #[must_use]
    pub const fn word(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::NotSession => "not_a_session",
            Self::NoSource => "no_source",
            Self::SourceRefused => "source_refused",
            Self::NoCalendar => "no_calendar",
            Self::NoShareOptions => "no_share_options",
            Self::SourceOnClosedDay => "source_on_closed_day",
        }
    }

    /// Why a day of this status has no membership answer.
    #[must_use]
    pub const fn reason(self) -> &'static str {
        match self {
            Self::Known => "known",
            Self::NotSession => "not a session (the IST calendar says closed)",
            Self::NoSource => "no GDFL options day file for this session",
            Self::SourceRefused => "the GDFL options day file was refused",
            Self::NoCalendar => "the IST calendar does not cover this day",
            Self::NoShareOptions => "the options day file lists no share option",
            Self::SourceOnClosedDay => "an options day file exists on a closed day",
        }
    }

    /// Whether a day of this status is a (possible) session without an
    /// answer: the days the per-year running sums count. A closed day is not
    /// a session, with or without a file on it.
    #[must_use]
    pub const fn is_refused_session(self) -> bool {
        matches!(
            self,
            Self::NoSource | Self::SourceRefused | Self::NoCalendar | Self::NoShareOptions
        )
    }
}

/// One sealed day record's row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DayRow {
    /// Whether the day has an answer.
    pub status: DayStatus,
    /// Live series: the pool size (0 unless known).
    pub pool: u16,
    /// Members that listed at least one file that day.
    pub filed: u16,
    /// Refused sessions of this day's year inside the span, through this day:
    /// the running sum (D-5162).
    pub refused_in_year: u16,
    /// Option names the day listed.
    pub names: u32,
    /// Option names the decoder refused.
    pub refused: u32,
    /// Option names of index underlyings (not shares).
    pub index_names: u32,
    /// BLAKE3 of everything the fold consumed from the day's listing; zero
    /// for a day with no listing.
    pub evidence: [u8; DIGEST_LEN],
}

impl DayRow {
    /// A day with no listing.
    #[must_use]
    pub const fn empty(status: DayStatus) -> Self {
        Self {
            status,
            pool: 0,
            filed: 0,
            refused_in_year: 0,
            names: 0,
            refused: 0,
            index_names: 0,
            evidence: [0; DIGEST_LEN],
        }
    }
}

/// A contract key, for the handover test: settled expiry (epoch day), strike
/// (paisa), side (0 call, 1 put).
type Key = (u32, i64, u8);

/// One share on one listed day.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct SeriesDay {
    /// The latest settled expiry among its files that day.
    live_until: u32,
    /// Every contract key it listed, expiry first.
    keys: BTreeSet<Key>,
}

/// What one day's listing says, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayEvidence {
    day: Day,
    series: BTreeMap<Symbol, SeriesDay>,
    /// Option names listed.
    pub names: u32,
    /// Names of index underlyings (not shares).
    pub index_names: u32,
    /// Names the decoder refused.
    pub refused: u32,
    /// The first [`REFUSAL_SAMPLE`] refusals, verbatim.
    pub refused_sample: Vec<String>,
}

impl DayEvidence {
    /// The day listed.
    #[must_use]
    pub const fn day(&self) -> Day {
        self.day
    }

    /// The shares listed, in name order.
    pub fn shares(&self) -> impl Iterator<Item = &Symbol> {
        self.series.keys()
    }

    /// Whether `share` listed a file.
    #[must_use]
    pub fn lists(&self, share: &Symbol) -> bool {
        self.series.contains_key(share)
    }

    /// BLAKE3 of what the fold consumes: the day, the counts, each share, its
    /// latest settled expiry and every contract key. A resumed build re-reads
    /// the last sealed session and refuses when this differs from its record.
    #[must_use]
    pub fn digest(&self) -> [u8; DIGEST_LEN] {
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"fno-membership evidence v1");
        hasher.update(&self.day.days_from_epoch().to_le_bytes());
        hasher.update(&self.names.to_le_bytes());
        hasher.update(&self.index_names.to_le_bytes());
        hasher.update(&self.refused.to_le_bytes());
        for (share, day) in &self.series {
            hasher.update(share.as_str().as_bytes());
            hasher.update(&[0]);
            hasher.update(&day.live_until.to_le_bytes());
            for (expiry, strike, side) in &day.keys {
                hasher.update(&expiry.to_le_bytes());
                hasher.update(&strike.to_le_bytes());
                hasher.update(&[*side]);
            }
        }
        hasher.finalize()
    }
}

/// Decodes one day's option tickers (file stems without `.NFO.csv`).
///
/// Each ticker is decoded by [`decode_ticker`] for trade day `day`; a refusal
/// is counted and the first [`REFUSAL_SAMPLE`] are kept with their reason. An
/// index underlying is counted and not a share. O(tickers).
#[must_use]
pub fn evidence<'a>(day: Day, tickers: impl IntoIterator<Item = &'a str>) -> DayEvidence {
    let mut out = DayEvidence {
        day,
        series: BTreeMap::new(),
        names: 0,
        index_names: 0,
        refused: 0,
        refused_sample: Vec::new(),
    };
    for ticker in tickers {
        out.names = out.names.saturating_add(1);
        match decode_ticker(ticker, day) {
            Err(why) => {
                out.refused = out.refused.saturating_add(1);
                if out.refused_sample.len() < REFUSAL_SAMPLE {
                    out.refused_sample.push(format!("{ticker}: {why}"));
                }
            }
            Ok(read) => {
                if option_underlying(read.underlying.as_str()).segment == Segment::Index {
                    out.index_names = out.index_names.saturating_add(1);
                    continue;
                }
                let expiry = read.expiry.days_from_epoch();
                let side = match read.side {
                    OptionSide::Call => 0,
                    OptionSide::Put => 1,
                };
                let entry = out.series.entry(read.underlying).or_default();
                entry.live_until = entry.live_until.max(expiry);
                entry.keys.insert((expiry, read.strike, side));
            }
        }
    }
    out
}

/// What the source says about one day, before folding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DayInput {
    /// A listed session.
    Listed(DayEvidence),
    /// Any day without a listing, and the source's own words where it gave
    /// some.
    Unknown(DayStatus, Option<String>),
}

impl DayInput {
    /// Whether the day will be a known record: a listed session naming at
    /// least one share option.
    #[must_use]
    pub fn is_known(&self) -> bool {
        matches!(self, Self::Listed(listed) if !listed.series.is_empty())
    }
}

/// Reads one day from `source` under the IST calendar: a closed day is
/// probed so a file on it is refused by name; a session is listed and
/// decoded; a day the calendar does not cover is not read.
#[must_use]
pub fn read_day<S: NfoSource>(source: &S, day: Day) -> DayInput {
    match sourced_kind_of(i64::from(day.days_from_epoch())) {
        DayKind::Unmeasured => DayInput::Unknown(DayStatus::NoCalendar, None),
        DayKind::Closed => match source.day(day) {
            Ok(None) => DayInput::Unknown(DayStatus::NotSession, None),
            Ok(Some(_)) => DayInput::Unknown(DayStatus::SourceOnClosedDay, None),
            Err(why) => DayInput::Unknown(DayStatus::SourceRefused, Some(why.to_string())),
        },
        DayKind::Open(_) | DayKind::OpenLengthUnmeasured => match source.day(day) {
            Ok(None) => DayInput::Unknown(DayStatus::NoSource, None),
            Err(why) => DayInput::Unknown(DayStatus::SourceRefused, Some(why.to_string())),
            Ok(Some(listing)) => {
                let tickers = listing
                    .entries()
                    .iter()
                    .filter_map(|file| listing.ticker_of(file));
                DayInput::Listed(evidence(day, tickers))
            }
        },
    }
}

/// `days`, read by `workers` threads (at least one), returned in day order.
/// Each day is independent; the fold is not. A reader thread that panics
/// makes its days refused, named, never absent.
#[must_use]
pub fn read_days<S: NfoSource + Sync>(source: &S, days: &[Day], workers: usize) -> Vec<DayInput> {
    let workers = workers.clamp(1, days.len().max(1));
    let chunk = days.len().div_ceil(workers).max(1);
    let mut out: Vec<DayInput> = Vec::with_capacity(days.len());
    std::thread::scope(|scope| {
        let handles: Vec<_> = days
            .chunks(chunk)
            .map(|part| {
                (
                    part.len(),
                    scope.spawn(move || {
                        part.iter()
                            .map(|&day| read_day(source, day))
                            .collect::<Vec<_>>()
                    }),
                )
            })
            .collect();
        for (len, handle) in handles {
            match handle.join() {
                Ok(part) => out.extend(part),
                Err(_) => out.extend((0..len).map(|_| {
                    DayInput::Unknown(
                        DayStatus::SourceRefused,
                        Some("a reader thread panicked".to_owned()),
                    )
                })),
            }
        }
    });
    out
}

/// One spelling of one series: the name a share's options were filed under,
/// and when it first appeared.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Spelling {
    /// The name.
    pub name: Symbol,
    /// Its series: dense, in first-appearance order; a measured handover's
    /// new spelling carries the old one's.
    pub series_id: u32,
    /// The first day any record of it is stored.
    pub first_day: Day,
    /// The earliest settled expiry (epoch day) among its contracts that day.
    pub first_expiry: u32,
    /// The first day (epoch) any stored contract of ANY share with that
    /// expiry appeared: the start of that expiry's series as stored.
    pub series_start: u32,
}

impl Spelling {
    /// Days between the start of its first contract's expiry series and its
    /// own first stored record: how late its membership may have started
    /// (`docs/06-limits.md` L-PIT-3). Zero for a series present on the span's
    /// first day.
    #[must_use]
    pub const fn listing_lag_days(&self) -> u32 {
        self.first_day
            .days_from_epoch()
            .saturating_sub(self.series_start)
    }
}

/// A measured rename: spelling `new` joined spelling `old`'s series on
/// `day`. Labelled [`HANDOVER_LABEL`]. Names come from the registry
/// ([`State::spelling_named`], [`Membership::spelling`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Handover {
    /// The spelling whose life ended (registry id).
    pub old: u32,
    /// The spelling first listed that day holding its keys (registry id).
    pub new: u32,
    /// The session the new spelling was first listed.
    pub day: Day,
    /// Keys of the old spelling's previous-session listing the new one lists.
    pub shared: u32,
    /// Keys the old spelling listed on the previous session.
    pub old_keys: u32,
}

/// One member entry as stored: a spelling and whether it listed a file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entry {
    /// Spelling id (its position in the registry).
    pub spelling: u16,
    /// Whether it listed at least one file that day.
    pub filed: bool,
}

/// The state one day hands the next across a month boundary, as stored.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Carry {
    /// Each live SERIES and the epoch day its life runs to, ascending by
    /// series (D-5166). A live series is always filed under its newest
    /// spelling, which the registry names, so the series is the whole key.
    pub live: Vec<(u32, u32)>,
    /// Each live expiry and the first epoch day any contract of it was
    /// stored, ascending.
    pub expiry_start: Vec<(u32, u32)>,
    /// The year the running sum belongs to.
    pub year: u16,
    /// Refused sessions of that year so far.
    pub refused_in_year: u16,
}

/// One folded day: its row, its members and what it added.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Record {
    /// The day.
    pub day: Day,
    /// Its row.
    pub row: DayRow,
    /// Its members, ascending by series.
    pub entries: Vec<Entry>,
    /// Spellings first seen this day, in registry order.
    pub added: Vec<Spelling>,
    /// Handovers measured this day.
    pub handovers: Vec<Handover>,
    /// Handover candidates not joined, each with why (a side matched twice,
    /// a minority on the new side, or fewer than [`MIN_HANDOVER_KEYS`]).
    pub ambiguous: Vec<String>,
    /// Files listed under a spelling its series has moved on from, each
    /// named: filed under the series' newest spelling, never a second member
    /// (D-5166).
    pub strays: Vec<String>,
    /// The source's own words for a day without a listing.
    pub why: Option<String>,
    /// The day's refused-name sample.
    pub refused_sample: Vec<String>,
}

/// One live series in the fold: the epoch day its life runs to, and the
/// epoch day it last listed a file (a member is `filed` when that is today).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Live {
    until: u32,
    filed_on: u32,
}

/// The fold's state: the registry, each series' newest spelling, the live
/// series, the expiry starts and the running sum. A build starts from
/// [`State::default`] or from [`Membership::resume_state`].
#[derive(Debug, Clone, Default)]
pub struct State {
    registry: Vec<Spelling>,
    by_name: HashMap<Symbol, u32>,
    series: u32,
    /// Series id -> its newest spelling's registry id: the spelling every
    /// file of the series is filed under from the day that spelling joined.
    newest: Vec<u32>,
    /// Live series, keyed by series id: the key IS the member order, so a
    /// day's members are read in series order without sorting them, and one
    /// series can never be live twice (D-5166).
    live: BTreeMap<u32, Live>,
    expiry_start: BTreeMap<u32, u32>,
    year: u16,
    refused_in_year: u16,
    previous: Option<DayEvidence>,
    known_from: Option<Day>,
    last: Option<Day>,
}

impl State {
    /// The last day folded.
    #[must_use]
    pub const fn last(&self) -> Option<Day> {
        self.last
    }

    /// The first known day folded: the span's start.
    #[must_use]
    pub const fn known_from(&self) -> Option<Day> {
        self.known_from
    }

    /// Spellings registered.
    #[must_use]
    pub fn spellings(&self) -> usize {
        self.registry.len()
    }

    /// Series registered.
    #[must_use]
    pub const fn series(&self) -> u32 {
        self.series
    }

    /// The carry at this point.
    #[must_use]
    pub fn carry(&self) -> Carry {
        Carry {
            live: self
                .live
                .iter()
                .map(|(series, live)| (*series, live.until))
                .collect(),
            expiry_start: self
                .expiry_start
                .iter()
                .map(|(expiry, first)| (*expiry, *first))
                .collect(),
            year: self.year,
            refused_in_year: self.refused_in_year,
        }
    }

    /// The spelling at `id`.
    fn spelling(&self, id: u32) -> Option<&Spelling> {
        self.registry.get(usize::try_from(id).ok()?)
    }

    /// The newest spelling of `series`.
    fn newest_of(&self, series: u32) -> Option<&Spelling> {
        let id = self.newest.get(usize::try_from(series).ok()?)?;
        self.spelling(*id)
    }

    /// Folds one day. `day` must follow the last day folded.
    ///
    /// # Errors
    ///
    /// [`MembershipError::Gap`] for a day that does not follow the last one;
    /// [`MembershipError::TooManySpellings`] past [`MAX_SPELLINGS`];
    /// [`MembershipError::SourceRefused`] for a day whose source refused it:
    /// a refusal may be transient, so it is never sealed (D-5168);
    /// [`MembershipError::Corrupt`] for an input that says "unknown" with the
    /// known status.
    pub fn fold(&mut self, day: Day, input: DayInput) -> Result<Record, MembershipError> {
        if let Some(last) = self.last
            && last.succ().ok() != Some(day)
        {
            return Err(MembershipError::Gap { from: day, last });
        }
        if let DayInput::Unknown(DayStatus::SourceRefused, why) = &input {
            return Err(MembershipError::SourceRefused {
                day,
                why: why.clone().unwrap_or_default(),
            });
        }
        if self.year != day.year() {
            self.year = day.year();
            self.refused_in_year = 0;
        }
        let mut record = Record {
            day,
            row: DayRow::empty(DayStatus::NotSession),
            entries: Vec::new(),
            added: Vec::new(),
            handovers: Vec::new(),
            ambiguous: Vec::new(),
            strays: Vec::new(),
            why: None,
            refused_sample: Vec::new(),
        };
        match input {
            DayInput::Listed(listed) => {
                record.row.names = listed.names;
                record.row.refused = listed.refused;
                record.row.index_names = listed.index_names;
                record.row.evidence = listed.digest();
                record.refused_sample.clone_from(&listed.refused_sample);
                if listed.series.is_empty() {
                    record.row.status = DayStatus::NoShareOptions;
                    self.refused_in_year = self.refused_in_year.saturating_add(1);
                    self.previous = None;
                } else {
                    self.fold_known(day, listed, &mut record)?;
                }
            }
            DayInput::Unknown(DayStatus::Known, _) => {
                return Err(MembershipError::Corrupt {
                    path: PathBuf::new(),
                    why: format!("{day}: an input without a listing says known"),
                });
            }
            DayInput::Unknown(status, why) => {
                record.row.status = status;
                record.why = why;
                if status.is_refused_session() {
                    self.refused_in_year = self.refused_in_year.saturating_add(1);
                }
                if status != DayStatus::NotSession {
                    self.previous = None;
                }
            }
        }
        record.row.refused_in_year = self.refused_in_year;
        self.last = Some(day);
        Ok(record)
    }

    /// The series a registered spelling belongs to.
    fn series_named(&self, name: &Symbol) -> Option<u32> {
        let id = self.by_name.get(name)?;
        self.spelling(*id).map(|spelling| spelling.series_id)
    }

    /// The known-day half of [`Self::fold`]: O(pool + the day's listing),
    /// plus the handover test on a day a never-seen spelling lists.
    fn fold_known(
        &mut self,
        day: Day,
        listed: DayEvidence,
        record: &mut Record,
    ) -> Result<(), MembershipError> {
        let epoch = day.days_from_epoch();
        self.live.retain(|_, live| live.until >= epoch);
        self.expiry_start.retain(|expiry, _| *expiry >= epoch);
        for one in listed.series.values() {
            for (expiry, _, _) in &one.keys {
                self.expiry_start.entry(*expiry).or_insert(epoch);
            }
        }
        // A live series that listed a file today under ANY of its spellings
        // is no handover's old side.
        for name in listed.series.keys() {
            if let Some(series) = self.series_named(name)
                && let Some(live) = self.live.get_mut(&series)
            {
                live.filed_on = epoch;
            }
        }
        let (handovers, ambiguous) = match &self.previous {
            Some(previous) => self.handovers_of(epoch, &listed, previous),
            None => (Vec::new(), Vec::new()),
        };
        record.ambiguous = ambiguous;
        for candidate in handovers {
            let old = self
                .newest
                .get(usize::try_from(candidate.series).unwrap_or(usize::MAX))
                .copied()
                .ok_or_else(|| corrupt(day, "a handover names no series"))?;
            let new = self.register(candidate.new, candidate.series, day, &listed)?;
            record.added.extend(self.spelling(new).copied());
            record.handovers.push(Handover {
                old,
                new,
                day,
                shared: candidate.shared,
                old_keys: candidate.old_keys,
            });
        }
        for (name, one) in &listed.series {
            let series = if let Some(series) = self.series_named(name) {
                series
            } else {
                let series = self.series;
                let id = self.register(*name, series, day, &listed)?;
                self.series = self.series.saturating_add(1);
                record.added.extend(self.spelling(id).copied());
                series
            };
            let newest = self
                .newest_of(series)
                .map(|spelling| spelling.name)
                .ok_or_else(|| corrupt(day, "a series has no newest spelling"))?;
            if newest != *name {
                record.strays.push(format!(
                    "{name} listed {} contract key(s) after its series moved on to {newest}: \
                     filed under {newest}, one member (D-5166)",
                    one.keys.len()
                ));
            }
            let live = self.live.entry(series).or_insert(Live {
                until: one.live_until,
                filed_on: epoch,
            });
            live.until = live.until.max(one.live_until);
            live.filed_on = epoch;
        }
        let mut entries = Vec::with_capacity(self.live.len());
        for (series, live) in &self.live {
            let spelling = self
                .newest
                .get(usize::try_from(*series).unwrap_or(usize::MAX))
                .copied()
                .ok_or_else(|| corrupt(day, "a live series has no spelling"))?;
            entries.push(Entry {
                spelling: u16::try_from(spelling)
                    .map_err(|_| MembershipError::TooManySpellings { day })?,
                filed: live.filed_on == epoch,
            });
        }
        record.row.status = DayStatus::Known;
        record.row.pool =
            u16::try_from(entries.len()).map_err(|_| MembershipError::TooManySpellings { day })?;
        record.row.filed = u16::try_from(entries.iter().filter(|entry| entry.filed).count())
            .map_err(|_| MembershipError::TooManySpellings { day })?;
        record.entries = entries;
        self.known_from.get_or_insert(day);
        self.previous = Some(listed);
        Ok(())
    }

    /// Registers `name` as the newest spelling of `series`: a new series
    /// when `series` is the next unused id, else a handover's new side.
    fn register(
        &mut self,
        name: Symbol,
        series: u32,
        day: Day,
        listed: &DayEvidence,
    ) -> Result<u32, MembershipError> {
        if self.registry.len() >= MAX_SPELLINGS {
            return Err(MembershipError::TooManySpellings { day });
        }
        let epoch = day.days_from_epoch();
        let first_expiry = listed
            .series
            .get(&name)
            .and_then(|one| one.keys.iter().next())
            .map_or(epoch, |(expiry, _, _)| *expiry);
        let series_start = self
            .expiry_start
            .get(&first_expiry)
            .copied()
            .unwrap_or(epoch);
        let id = u32::try_from(self.registry.len())
            .map_err(|_| MembershipError::TooManySpellings { day })?;
        self.registry.push(Spelling {
            name,
            series_id: series,
            first_day: day,
            first_expiry,
            series_start,
        });
        self.by_name.insert(name, id);
        match self
            .newest
            .get_mut(usize::try_from(series).unwrap_or(usize::MAX))
        {
            Some(slot) => *slot = id,
            None => self.newest.push(id),
        }
        Ok(id)
    }

    /// The handovers of `today` against the session before (module doc;
    /// D-5161 as amended by D-5166). A never-seen spelling joins a live
    /// series that listed nothing today when the keys they share are a
    /// strict majority of BOTH the series' previous-session keys and the new
    /// spelling's keys, and at least [`MIN_HANDOVER_KEYS`]; a pair either
    /// side of which matches a second candidate is not joined. Every pair the
    /// old-side majority alone would have joined and these rules do not is
    /// reported by name. Work only on a day a never-seen spelling lists:
    /// O(new spellings x pool x keys) then, nothing otherwise.
    fn handovers_of(
        &self,
        epoch: u32,
        today: &DayEvidence,
        previous: &DayEvidence,
    ) -> (Vec<Candidate>, Vec<String>) {
        let mut pairs: Vec<Candidate> = Vec::new();
        let mut ambiguous = Vec::new();
        let count = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
        for (new, listed) in &today.series {
            if self.by_name.contains_key(new) {
                continue;
            }
            for (series, live) in &self.live {
                if live.filed_on == epoch {
                    continue;
                }
                let Some(old) = self.newest_of(*series) else {
                    continue;
                };
                let Some(before) = previous.series.get(&old.name) else {
                    continue;
                };
                let shared = listed.keys.intersection(&before.keys).count();
                if shared.saturating_mul(2) <= before.keys.len() {
                    continue;
                }
                let candidate = Candidate {
                    series: *series,
                    old: old.name,
                    new: *new,
                    shared: count(shared),
                    old_keys: count(before.keys.len()),
                    new_keys: count(listed.keys.len()),
                };
                if shared.saturating_mul(2) <= listed.keys.len() {
                    ambiguous.push(candidate.refusal("a minority of the new spelling's keys"));
                } else if shared < MIN_HANDOVER_KEYS {
                    ambiguous.push(
                        candidate.refusal(&format!("fewer than {MIN_HANDOVER_KEYS} shared keys")),
                    );
                } else {
                    pairs.push(candidate);
                }
            }
        }
        let mut news: HashMap<Symbol, usize> = HashMap::with_capacity(pairs.len());
        let mut olds: HashMap<u32, usize> = HashMap::with_capacity(pairs.len());
        for pair in &pairs {
            *news.entry(pair.new).or_insert(0) += 1;
            *olds.entry(pair.series).or_insert(0) += 1;
        }
        let mut joined = Vec::with_capacity(pairs.len());
        for pair in pairs {
            if news.get(&pair.new) == Some(&1) && olds.get(&pair.series) == Some(&1) {
                joined.push(pair);
            } else {
                ambiguous.push(pair.refusal("a side matches a second candidate"));
            }
        }
        (joined, ambiguous)
    }

    /// The name of spelling `id`, for reports.
    #[must_use]
    pub fn spelling_named(&self, id: u32) -> Option<Symbol> {
        self.spelling(id).map(|spelling| spelling.name)
    }
}

/// The fewest contract keys a measured handover may rest on (D-5166). A
/// RULE of evidence, not a tuned value: one shared key is a coincidence any
/// two shares listing the same round strike of the same expiry make (verify
/// probe `c5_4`), so a pair resting on it is reported and never joined.
pub const MIN_HANDOVER_KEYS: usize = 2;

/// A handover candidate: the old series and its newest spelling, the new
/// spelling, the keys shared and each side's key count.
#[derive(Debug, Clone, Copy)]
struct Candidate {
    series: u32,
    old: Symbol,
    new: Symbol,
    shared: u32,
    old_keys: u32,
    new_keys: u32,
}

impl Candidate {
    /// The report line of a candidate that was not joined, and why.
    fn refusal(&self, why: &str) -> String {
        format!(
            "{} -> {} ({} of {} old keys, {} of {} new keys) not joined: {why}",
            self.old, self.new, self.shared, self.old_keys, self.shared, self.new_keys
        )
    }
}

/// A corrupt-state refusal naming the day.
fn corrupt(day: Day, why: &str) -> MembershipError {
    MembershipError::Corrupt {
        path: PathBuf::new(),
        why: format!("{day}: {why}"),
    }
}

/// One month's sealed records, as built or as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MonthTable {
    year: u16,
    month: u8,
    first: u8,
    known_from: u32,
    max_pool: u16,
    spellings_before: u32,
    series_before: u32,
    added: Vec<Spelling>,
    rows: Vec<DayRow>,
    entries: Vec<Entry>,
    carry_in: Carry,
    carry_out: Carry,
    handovers: Vec<Handover>,
}

impl MonthTable {
    /// An empty table of `day`'s month whose first record will be `day`,
    /// starting from `state` (the state before `day` is folded).
    #[must_use]
    pub fn begin(day: Day, state: &State) -> Self {
        Self {
            year: day.year(),
            month: day.month(),
            first: day.day(),
            known_from: 0,
            max_pool: 0,
            spellings_before: u32::try_from(state.registry.len()).unwrap_or(u32::MAX),
            series_before: state.series,
            added: Vec::new(),
            rows: Vec::new(),
            entries: Vec::new(),
            carry_in: state.carry(),
            carry_out: state.carry(),
            handovers: Vec::new(),
        }
    }

    /// Appends one record, the day after the last one, and takes the carry
    /// and span start from `state` (the state after the record's fold).
    ///
    /// # Errors
    ///
    /// [`MembershipError::Corrupt`] for a record of another month or out of
    /// order.
    pub fn push(&mut self, record: Record, state: &State) -> Result<(), MembershipError> {
        let next = u8::try_from(self.rows.len())
            .ok()
            .and_then(|held| self.first.checked_add(held));
        if (record.day.year(), record.day.month()) != (self.year, self.month)
            || Some(record.day.day()) != next
        {
            return Err(corrupt(
                record.day,
                "a record out of its month's order was offered",
            ));
        }
        if record.row.status == DayStatus::Known {
            self.max_pool = self.max_pool.max(record.row.pool);
        }
        self.rows.push(record.row);
        self.entries.extend(record.entries);
        self.added.extend(record.added);
        self.handovers.extend(record.handovers);
        self.carry_out = state.carry();
        self.known_from = state.known_from.map_or(0, Day::days_from_epoch);
        Ok(())
    }

    /// The month, `(year, month)`.
    #[must_use]
    pub const fn month(&self) -> (u16, u8) {
        (self.year, self.month)
    }

    /// The first and last day held, or `None` for an empty table.
    #[must_use]
    pub fn held(&self) -> Option<(Day, Day)> {
        let last = u8::try_from(self.rows.len().checked_sub(1)?)
            .ok()
            .and_then(|extra| self.first.checked_add(extra))?;
        Some((
            Day::new(self.year, self.month, self.first).ok()?,
            Day::new(self.year, self.month, last).ok()?,
        ))
    }

    /// Every row, in day order from the first held day.
    #[must_use]
    pub fn rows(&self) -> &[DayRow] {
        &self.rows
    }

    /// Spellings first seen this month.
    #[must_use]
    pub fn added(&self) -> &[Spelling] {
        &self.added
    }

    /// Handovers measured this month.
    #[must_use]
    pub fn handovers(&self) -> &[Handover] {
        &self.handovers
    }

    /// The largest pool of a known day this month.
    #[must_use]
    pub const fn max_pool(&self) -> u16 {
        self.max_pool
    }

    /// The carry the month started from.
    #[must_use]
    pub const fn carry_in(&self) -> &Carry {
        &self.carry_in
    }

    /// The carry after its last record.
    #[must_use]
    pub const fn carry_out(&self) -> &Carry {
        &self.carry_out
    }

    /// Whether `self` holds `held`'s records unchanged and possibly more: the
    /// only way a later generation may differ from an earlier one.
    #[must_use]
    pub fn extends(&self, held: &Self) -> bool {
        (self.year, self.month, self.first, self.known_from)
            == (held.year, held.month, held.first, held.known_from)
            && (self.spellings_before, self.series_before, &self.carry_in)
                == (held.spellings_before, held.series_before, &held.carry_in)
            && self.rows.starts_with(&held.rows)
            && self.entries.starts_with(&held.entries)
            && self.added.starts_with(&held.added)
            && self.handovers.starts_with(&held.handovers)
    }

    /// The file bytes. Canonical: no time, host, commit or generation is
    /// written, so the same records give the same bytes.
    #[must_use]
    pub fn encode(&self) -> Vec<u8> {
        let mut out = Vec::with_capacity(self.encoded_len());
        let n = |len: usize| u32::try_from(len).unwrap_or(u32::MAX).to_le_bytes();
        out.extend_from_slice(&MAGIC);
        out.extend_from_slice(&VERSION.to_le_bytes());
        out.extend_from_slice(&u16::try_from(HEADER_LEN).unwrap_or(u16::MAX).to_le_bytes());
        out.extend_from_slice(&self.year.to_le_bytes());
        out.push(self.month);
        out.push(self.first);
        out.push(
            u8::try_from(self.rows.len())
                .ok()
                .and_then(|len| len.checked_sub(1))
                .and_then(|extra| self.first.checked_add(extra))
                .unwrap_or(0),
        );
        out.push(0);
        out.extend_from_slice(&self.max_pool.to_le_bytes());
        out.extend_from_slice(&self.known_from.to_le_bytes());
        out.extend_from_slice(&self.spellings_before.to_le_bytes());
        out.extend_from_slice(&n(self.added.len()));
        out.extend_from_slice(&self.series_before.to_le_bytes());
        out.extend_from_slice(&n(self.entries.len()));
        out.extend_from_slice(&n(self.carry_in.live.len()));
        out.extend_from_slice(&n(self.carry_in.expiry_start.len()));
        out.extend_from_slice(&n(self.carry_out.live.len()));
        out.extend_from_slice(&n(self.carry_out.expiry_start.len()));
        out.extend_from_slice(&n(self.handovers.len()));
        out.extend_from_slice(&self.carry_in.year.to_le_bytes());
        out.extend_from_slice(&self.carry_in.refused_in_year.to_le_bytes());
        out.extend_from_slice(&self.carry_out.year.to_le_bytes());
        out.extend_from_slice(&self.carry_out.refused_in_year.to_le_bytes());
        out.extend_from_slice(&[0, 0, 0, 0]);
        for spelling in &self.added {
            put_name(&mut out, &spelling.name);
            out.extend_from_slice(&spelling.series_id.to_le_bytes());
            out.extend_from_slice(&spelling.first_day.days_from_epoch().to_le_bytes());
            out.extend_from_slice(&spelling.first_expiry.to_le_bytes());
            out.extend_from_slice(&spelling.series_start.to_le_bytes());
        }
        for row in &self.rows {
            out.push(row.status as u8);
            out.push(0);
            out.extend_from_slice(&row.pool.to_le_bytes());
            out.extend_from_slice(&row.filed.to_le_bytes());
            out.extend_from_slice(&row.refused_in_year.to_le_bytes());
            out.extend_from_slice(&row.names.to_le_bytes());
            out.extend_from_slice(&row.refused.to_le_bytes());
            out.extend_from_slice(&row.index_names.to_le_bytes());
            out.extend_from_slice(&row.evidence);
        }
        for entry in &self.entries {
            out.extend_from_slice(&entry.spelling.to_le_bytes());
            out.extend_from_slice(&(if entry.filed { FILED } else { 0 }).to_le_bytes());
        }
        for carry in [&self.carry_in, &self.carry_out] {
            for (a, b) in carry.live.iter().chain(&carry.expiry_start) {
                out.extend_from_slice(&a.to_le_bytes());
                out.extend_from_slice(&b.to_le_bytes());
            }
        }
        for handover in &self.handovers {
            out.extend_from_slice(&handover.old.to_le_bytes());
            out.extend_from_slice(&handover.new.to_le_bytes());
            out.extend_from_slice(&handover.day.days_from_epoch().to_le_bytes());
            out.extend_from_slice(&handover.shared.to_le_bytes());
            out.extend_from_slice(&handover.old_keys.to_le_bytes());
        }
        let digest = brutex_core::blake3::hash(&out);
        out.extend_from_slice(&digest);
        out
    }

    /// The length [`Self::encode`] writes.
    fn encoded_len(&self) -> usize {
        HEADER_LEN
            + self.added.len() * SPELLING_LEN
            + self.rows.len() * DAY_ROW_LEN
            + self.entries.len() * ENTRY_LEN
            + (self.carry_in.live.len() + self.carry_out.live.len()) * LIVE_LEN
            + (self.carry_in.expiry_start.len() + self.carry_out.expiry_start.len()) * EXPIRY_LEN
            + self.handovers.len() * HANDOVER_LEN
            + DIGEST_LEN
    }
}

/// Appends a zero-padded name.
fn put_name(out: &mut Vec<u8>, share: &Symbol) {
    let mut name = [0_u8; NAME_LEN];
    for (slot, byte) in name.iter_mut().zip(share.as_str().bytes()) {
        *slot = byte;
    }
    out.extend_from_slice(&name);
}

/// The checked reader of [`MonthTable::encode`]'s bytes: every check that
/// one file can make alone. The checks that need the months around it (the
/// chain, the registry, the series order of entries) are
/// [`Membership::open`]'s.
mod decode {
    use super::{
        Carry, DAY_ROW_LEN, DIGEST_LEN, DayRow, DayStatus, ENTRY_LEN, EXPIRY_LEN, Entry, FILED,
        HANDOVER_LEN, HEADER_LEN, Handover, LIVE_LEN, MAGIC, MonthTable, NAME_LEN, SPELLING_LEN,
        Spelling, VERSION,
    };
    use crate::session::Day;
    use brutex_core::symbol::Symbol;

    /// A cursor over the bytes that refuses every read past the end.
    struct Cursor<'a> {
        bytes: &'a [u8],
        at: usize,
    }

    impl<'a> Cursor<'a> {
        fn take(&mut self, len: usize) -> Result<&'a [u8], String> {
            let end = self.at.checked_add(len).ok_or("length overflow")?;
            let out = self
                .bytes
                .get(self.at..end)
                .ok_or_else(|| format!("truncated at byte {}", self.at))?;
            self.at = end;
            Ok(out)
        }
        fn array<const N: usize>(&mut self) -> Result<[u8; N], String> {
            let mut out = [0_u8; N];
            out.copy_from_slice(self.take(N)?);
            Ok(out)
        }
        fn u8(&mut self) -> Result<u8, String> {
            Ok(u8::from_le_bytes(self.array()?))
        }
        fn u16(&mut self) -> Result<u16, String> {
            Ok(u16::from_le_bytes(self.array()?))
        }
        fn u32(&mut self) -> Result<u32, String> {
            Ok(u32::from_le_bytes(self.array()?))
        }
        fn count(&mut self) -> Result<usize, String> {
            usize::try_from(self.u32()?).map_err(|_| "a count overflows".to_owned())
        }
        fn day(&mut self) -> Result<Day, String> {
            Day::from_days(self.u32()?).map_err(|why| why.to_string())
        }
        fn name(&mut self) -> Result<Symbol, String> {
            let raw = self.take(NAME_LEN)?;
            let len = raw.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
            let (text, pad) = raw.split_at(len);
            if pad.iter().any(|&b| b != 0) {
                return Err("a name has bytes after its padding".to_owned());
            }
            let text = std::str::from_utf8(text).map_err(|_| "a name is not UTF-8".to_owned())?;
            Symbol::new(text)
                .ok()
                .filter(|share| share.as_str() == text)
                .ok_or_else(|| format!("{text:?} is not a symbol"))
        }
    }

    /// The header's counts.
    struct Counts {
        added: usize,
        entries: usize,
        live_in: usize,
        expiry_in: usize,
        live_out: usize,
        expiry_out: usize,
        handovers: usize,
    }

    impl Counts {
        fn body_len(&self, rows: usize) -> Option<usize> {
            let pairs = self
                .live_in
                .checked_add(self.live_out)?
                .checked_mul(LIVE_LEN)?
                .checked_add(
                    self.expiry_in
                        .checked_add(self.expiry_out)?
                        .checked_mul(EXPIRY_LEN)?,
                )?;
            HEADER_LEN
                .checked_add(self.added.checked_mul(SPELLING_LEN)?)?
                .checked_add(rows.checked_mul(DAY_ROW_LEN)?)?
                .checked_add(self.entries.checked_mul(ENTRY_LEN)?)?
                .checked_add(pairs)?
                .checked_add(self.handovers.checked_mul(HANDOVER_LEN)?)
        }
    }

    /// Every check of `docs/02-store-format.md` one file can make, in file
    /// order.
    pub(super) fn table(bytes: &[u8]) -> Result<MonthTable, String> {
        let body_len = bytes
            .len()
            .checked_sub(DIGEST_LEN)
            .ok_or("shorter than its digest")?;
        let (body, digest) = bytes.split_at(body_len);
        if brutex_core::blake3::hash(body).as_slice() != digest {
            return Err("the BLAKE3 digest does not match the bytes".to_owned());
        }
        let mut cursor = Cursor { bytes: body, at: 0 };
        if cursor.array::<8>()? != MAGIC {
            return Err("not a membership file (magic)".to_owned());
        }
        let version = cursor.u16()?;
        if version != VERSION {
            return Err(format!(
                "version {version} is not one this reader knows ({VERSION})"
            ));
        }
        if usize::from(cursor.u16()?) != HEADER_LEN {
            return Err(format!("header length is not {HEADER_LEN}"));
        }
        let (year, month, first, last) = (cursor.u16()?, cursor.u8()?, cursor.u8()?, cursor.u8()?);
        let first_day =
            Day::new(year, month, first).map_err(|_| "the first held day is not a day")?;
        Day::new(year, month, last).map_err(|_| "the last held day is not a day")?;
        if last < first {
            return Err("the last held day is before the first".to_owned());
        }
        if cursor.u8()? != 0 {
            return Err("the header's reserved byte is not zero".to_owned());
        }
        let max_pool = cursor.u16()?;
        let known_from = cursor.u32()?;
        let spellings_before = cursor.u32()?;
        let added_n = cursor.count()?;
        let series_before = cursor.u32()?;
        let counts = Counts {
            added: added_n,
            entries: cursor.count()?,
            live_in: cursor.count()?,
            expiry_in: cursor.count()?,
            live_out: cursor.count()?,
            expiry_out: cursor.count()?,
            handovers: cursor.count()?,
        };
        let (year_in, refused_in) = (cursor.u16()?, cursor.u16()?);
        let (year_out, refused_out) = (cursor.u16()?, cursor.u16()?);
        if cursor.u32()? != 0 {
            return Err("the header's reserved word is not zero".to_owned());
        }
        if known_from > first_day.days_from_epoch() {
            return Err("the span starts after the month's first record".to_owned());
        }
        let rows_n = usize::from(last - first) + 1;
        if counts.body_len(rows_n) != Some(body.len()) {
            return Err(format!(
                "{} body bytes where the header implies another length",
                body.len()
            ));
        }
        let added = (0..counts.added)
            .map(|_| spelling(&mut cursor))
            .collect::<Result<Vec<_>, _>>()?;
        let rows = (0..rows_n)
            .map(|_| row(&mut cursor))
            .collect::<Result<Vec<_>, _>>()?;
        let entries = (0..counts.entries)
            .map(|_| entry(&mut cursor))
            .collect::<Result<Vec<_>, _>>()?;
        let carry_in = carry(
            &mut cursor,
            (counts.live_in, counts.expiry_in),
            (year_in, refused_in),
        )?;
        let carry_out = carry(
            &mut cursor,
            (counts.live_out, counts.expiry_out),
            (year_out, refused_out),
        )?;
        let handovers = (0..counts.handovers)
            .map(|_| handover(&mut cursor))
            .collect::<Result<Vec<_>, _>>()?;
        let table = MonthTable {
            year,
            month,
            first,
            known_from,
            max_pool,
            spellings_before,
            series_before,
            added,
            rows,
            entries,
            carry_in,
            carry_out,
            handovers,
        };
        check_rows(&table)?;
        Ok(table)
    }

    /// The rows against the entries, the running sum, the pool bound and the
    /// span's edges.
    fn check_rows(table: &MonthTable) -> Result<(), String> {
        let mut at = 0_usize;
        let mut year = table.carry_in.year;
        let mut refused = table.carry_in.refused_in_year;
        let mut max_pool = 0;
        for (offset, row) in table.rows.iter().enumerate() {
            let label = || format!("day row {}", offset + usize::from(table.first));
            if table.year != year {
                year = table.year;
                refused = 0;
            }
            if row.status.is_refused_session() {
                refused = refused.saturating_add(1);
            }
            if row.refused_in_year != refused {
                return Err(format!("{}: its running sum is not the fold's", label()));
            }
            if row.status != DayStatus::Known && (row.pool > 0 || row.filed > 0) {
                return Err(format!("{} is not known and holds members", label()));
            }
            if row.status == DayStatus::Known && row.pool == 0 {
                return Err(format!("{} is known and holds no member", label()));
            }
            if row.status == DayStatus::SourceRefused {
                return Err(format!(
                    "{} is a refused source day, which is never sealed (D-5168)",
                    label()
                ));
            }
            let listed = matches!(row.status, DayStatus::Known | DayStatus::NoShareOptions);
            if !listed && (row.evidence != [0; DIGEST_LEN] || row.names + row.refused > 0) {
                return Err(format!("{} has no listing and holds its counts", label()));
            }
            let end = at
                .checked_add(usize::from(row.pool))
                .ok_or("an entry count overflows")?;
            let day = table
                .entries
                .get(at..end)
                .ok_or_else(|| format!("{}: more members than entries", label()))?;
            if day.iter().filter(|entry| entry.filed).count() != usize::from(row.filed) {
                return Err(format!("{}: filed count differs from its entries", label()));
            }
            at = end;
            if row.status == DayStatus::Known {
                max_pool = max_pool.max(row.pool);
            }
        }
        if at != table.entries.len() {
            return Err("entries past the last day's members".to_owned());
        }
        if (year, refused) != (table.carry_out.year, table.carry_out.refused_in_year) {
            return Err("the carry out's running sum is not the last row's".to_owned());
        }
        if max_pool != table.max_pool {
            return Err("the header's largest pool is not its rows'".to_owned());
        }
        Ok(())
    }

    fn spelling(cursor: &mut Cursor<'_>) -> Result<Spelling, String> {
        Ok(Spelling {
            name: cursor.name()?,
            series_id: cursor.u32()?,
            first_day: cursor.day()?,
            first_expiry: cursor.u32()?,
            series_start: cursor.u32()?,
        })
    }

    fn row(cursor: &mut Cursor<'_>) -> Result<DayRow, String> {
        let code = cursor.u8()?;
        let status =
            DayStatus::from_code(code).ok_or_else(|| format!("day status {code} is unknown"))?;
        if cursor.u8()? != 0 {
            return Err("a day row's reserved byte is not zero".to_owned());
        }
        Ok(DayRow {
            status,
            pool: cursor.u16()?,
            filed: cursor.u16()?,
            refused_in_year: cursor.u16()?,
            names: cursor.u32()?,
            refused: cursor.u32()?,
            index_names: cursor.u32()?,
            evidence: cursor.array()?,
        })
    }

    fn entry(cursor: &mut Cursor<'_>) -> Result<Entry, String> {
        let spelling = cursor.u16()?;
        let flags = cursor.u16()?;
        if flags & !FILED != 0 {
            return Err("a member entry holds an unknown flag".to_owned());
        }
        Ok(Entry {
            spelling,
            filed: flags & FILED != 0,
        })
    }

    /// Two ascending runs of pairs, and the running sum the header gave.
    fn carry(
        cursor: &mut Cursor<'_>,
        (live, expiries): (usize, usize),
        (year, refused_in_year): (u16, u16),
    ) -> Result<Carry, String> {
        let mut run = |n: usize| -> Result<Vec<(u32, u32)>, String> {
            let pairs = (0..n)
                .map(|_| Ok((cursor.u32()?, cursor.u32()?)))
                .collect::<Result<Vec<_>, String>>()?;
            if pairs
                .windows(2)
                .all(|pair| pair.first().map(|a| a.0) < pair.last().map(|b| b.0))
            {
                Ok(pairs)
            } else {
                Err("a carry run is not strictly ascending".to_owned())
            }
        };
        Ok(Carry {
            live: run(live)?,
            expiry_start: run(expiries)?,
            year,
            refused_in_year,
        })
    }

    fn handover(cursor: &mut Cursor<'_>) -> Result<Handover, String> {
        Ok(Handover {
            old: cursor.u32()?,
            new: cursor.u32()?,
            day: cursor.day()?,
            shared: cursor.u32()?,
            old_keys: cursor.u32()?,
        })
    }
}

impl MonthTable {
    /// Reads a file's bytes, checking every field one file can check.
    ///
    /// # Errors
    ///
    /// [`MembershipError::Corrupt`] naming the first check that failed.
    pub fn decode(bytes: &[u8]) -> Result<Self, MembershipError> {
        decode::table(bytes).map_err(|why| MembershipError::Corrupt {
            path: PathBuf::new(),
            why,
        })
    }
}

/// Why a day has no membership answer (D-5162).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnknownWhy {
    /// No record is held at all.
    Empty,
    /// Before the span's first record.
    BeforeSpan {
        /// The span's first day.
        known_from: Day,
    },
    /// After the span's last record.
    AfterSpan {
        /// The span's last day.
        known_to: Day,
    },
    /// Inside the span, a sealed record without an answer.
    Refused(DayStatus),
}

/// "membership unknown": the one refusal of every per-day lookup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MembershipUnknown {
    /// The day asked.
    pub day: Day,
    /// Why it has no answer.
    pub why: UnknownWhy,
}

impl fmt::Display for MembershipUnknown {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let day = self.day;
        match self.why {
            UnknownWhy::Empty => write!(f, "membership unknown: {day}: no record is held"),
            UnknownWhy::BeforeSpan { known_from } => write!(
                f,
                "membership unknown: {day}: before the first stored stock-derivative record \
                 ({known_from}); never filled from today's list"
            ),
            UnknownWhy::AfterSpan { known_to } => write!(
                f,
                "membership unknown: {day}: after the last stored stock-derivative record \
                 ({known_to}); never filled from today's list"
            ),
            UnknownWhy::Refused(status) => write!(
                f,
                "membership unknown: {day} {}: {}",
                status.word(),
                status.reason()
            ),
        }
    }
}

impl std::error::Error for MembershipUnknown {}

/// Every refusal of this module.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MembershipError {
    /// The day has no answer.
    Unknown(MembershipUnknown),
    /// The name is not a symbol.
    BadName {
        /// The name asked.
        name: String,
    },
    /// The name is an index underlying, not a share.
    NotAShare {
        /// The name asked.
        name: String,
    },
    /// A day offered to the fold does not follow the last one.
    Gap {
        /// The day offered.
        from: Day,
        /// The last day folded or sealed.
        last: Day,
    },
    /// A build window that starts on or before a sealed record.
    Overlap {
        /// The window's first day.
        from: Day,
        /// The last sealed record.
        last: Day,
    },
    /// A window whose start is after its end, or outside the calendar.
    BadWindow {
        /// The first day.
        from: Day,
        /// The last day.
        to: Day,
    },
    /// More spellings than [`MAX_SPELLINGS`].
    TooManySpellings {
        /// The day it happened.
        day: Day,
    },
    /// The source refused a day's listing. A refusal may be transient (a
    /// failed read, a reader that panicked), so the day is never sealed:
    /// every record before it is installed and a rerun resumes at it
    /// (D-5168).
    SourceRefused {
        /// The day refused.
        day: Day,
        /// The source's own words.
        why: String,
    },
    /// The last sealed session's listing now reads differently.
    SourceChanged {
        /// The session.
        day: Day,
    },
    /// A file failed a check.
    Corrupt {
        /// The file.
        path: PathBuf,
        /// The first check that failed.
        why: String,
    },
    /// A month that does not continue the month before it.
    Stale {
        /// The month's file.
        path: PathBuf,
        /// What does not continue.
        why: String,
    },
    /// A new generation would change a sealed record.
    WouldRewrite {
        /// The month's current file.
        path: PathBuf,
    },
    /// A directory or file operation failed.
    Io {
        /// The path.
        path: PathBuf,
        /// The error.
        why: String,
    },
    /// The final name already exists: another writer got there first.
    Exists {
        /// The path.
        path: PathBuf,
    },
}

impl fmt::Display for MembershipError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unknown(unknown) => write!(f, "{unknown}"),
            Self::BadName { name } => write!(f, "{name:?} is not a symbol"),
            Self::NotAShare { name } => {
                write!(f, "{name:?} is an index underlying, not a share (D-5160)")
            }
            Self::Gap { from, last } => write!(
                f,
                "{from} does not follow the last record {last}: a gap is never left between \
                 sealed records"
            ),
            Self::Overlap { from, last } => write!(
                f,
                "the window starts {from}, on or before the last sealed record {last}: a \
                 sealed record is never rebuilt (a new source is a new file version)"
            ),
            Self::BadWindow { from, to } => write!(
                f,
                "the window {from}..={to} is empty or outside the IST calendar"
            ),
            Self::TooManySpellings { day } => write!(
                f,
                "{day}: more than {MAX_SPELLINGS} spellings, past the format bound"
            ),
            Self::SourceRefused { day, why } => write!(
                f,
                "{day}: the GDFL options day file was refused ({why}); a refusal may be \
                 transient, so the day is not sealed: every record before it is installed and \
                 the next build starts again at it (D-5168)"
            ),
            Self::SourceChanged { day } => write!(
                f,
                "{day}: the source's listing no longer matches its sealed record; the store \
                 is not extended from changed evidence"
            ),
            Self::Corrupt { path, why } | Self::Io { path, why } => {
                write!(f, "{}: {why}", path.display())
            }
            Self::Stale { path, why } => write!(f, "{}: stale: {why}", path.display()),
            Self::WouldRewrite { path } => write!(
                f,
                "the new generation of {} would change its sealed records",
                path.display()
            ),
            Self::Exists { path } => {
                write!(f, "{} already exists; nothing written", path.display())
            }
        }
    }
}

impl std::error::Error for MembershipError {}

impl From<MembershipUnknown> for MembershipError {
    fn from(unknown: MembershipUnknown) -> Self {
        Self::Unknown(unknown)
    }
}

/// What [`install`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// The current generation already holds these bytes: nothing written.
    Unchanged {
        /// The file.
        path: PathBuf,
        /// Its generation.
        generation: u32,
    },
    /// A new generation was written.
    Written {
        /// The file.
        path: PathBuf,
        /// Its generation.
        generation: u32,
        /// The bytes of a dead writer's leftover that `store::staging::create`
        /// cleared from this call's temporary name first, or `None` when the
        /// name was free: reported by the caller, never dropped.
        cleared: Option<u64>,
    },
}

/// `<YYYY-MM>-g<GGGGGG>.fnm`.
#[must_use]
pub fn file_name(year: u16, month: u8, generation: u32) -> String {
    format!("{year:04}-{month:02}-g{generation:06}.{EXTENSION}")
}

/// The month and generation a file name states, or `None` for any other name.
#[must_use]
pub fn parse_file_name(name: &str) -> Option<(u16, u8, u32)> {
    let stem = name.strip_suffix(EXTENSION)?.strip_suffix('.')?;
    let (month_text, generation) = stem.split_once("-g")?;
    let (year, month) = month_text.split_once('-')?;
    let parsed = (
        year.parse().ok()?,
        month.parse().ok()?,
        generation.parse().ok()?,
    );
    (file_name(parsed.0, parsed.1, parsed.2) == name).then_some(parsed)
}

/// Each month (year, month) and its highest generation: number and path.
pub type Generations = BTreeMap<(u16, u8), (u32, PathBuf)>;

/// The highest generation of every month in `dir`, and the hidden temporary
/// files a crashed writer left (never read).
///
/// # Errors
///
/// [`MembershipError::Io`] when the directory cannot be listed, a missing
/// one included: a wrong `--out` or consumer path is named, never served as
/// an empty membership; [`MembershipError::Corrupt`] for a file name that is
/// neither a month file nor a hidden temporary: an unknown file is refused by
/// name.
pub fn generations(dir: &Path) -> Result<(Generations, usize), MembershipError> {
    let io = |why: std::io::Error| MembershipError::Io {
        path: dir.to_path_buf(),
        why: if why.kind() == std::io::ErrorKind::NotFound {
            format!(
                "no such membership directory ({why}); nothing is served from a path that is not there"
            )
        } else {
            why.to_string()
        },
    };
    let mut out: BTreeMap<(u16, u8), (u32, PathBuf)> = BTreeMap::new();
    let mut temporaries = 0;
    for entry in fs::read_dir(dir).map_err(io)? {
        let entry = entry.map_err(io)?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        // A hidden name is never a month file: a writer's temporary (or a
        // desktop's own index file) is counted and never read.
        if name.starts_with('.') {
            temporaries += 1;
            continue;
        }
        let Some((year, month, generation)) = parse_file_name(&name) else {
            return Err(MembershipError::Corrupt {
                path: entry.path(),
                why: "not a membership file name (<YYYY-MM>-g<GGGGGG>.fnm)".to_owned(),
            });
        };
        let slot = out
            .entry((year, month))
            .or_insert((generation, entry.path()));
        if generation > slot.0 {
            *slot = (generation, entry.path());
        }
    }
    Ok((out, temporaries))
}

/// Writes `table` as `dir`'s next generation of its month, or nothing when
/// the current generation already holds the same bytes. A new generation must
/// extend the current one ([`MonthTable::extends`]): a sealed record is never
/// changed. The bytes go to a hidden temporary made by
/// `store::staging::create` (gate 28's one door), flushed, then hard-linked
/// to the final name, which therefore never names a partial file; the
/// directory is flushed after. Every failure after the temporary exists
/// unlinks it before returning, so a refusal that says nothing was written
/// leaves nothing behind; only a kill can leave one (hidden, counted by
/// [`generations`], never read). A dead writer's leftover under this call's
/// own temporary name is cleared by the door first and its size returned in
/// `Installed::Written`'s `cleared`, so it is reported, never dropped unsaid.
///
/// # Errors
///
/// [`MembershipError::Io`] for a failed directory, staging create, write,
/// flush, link or unlink; [`MembershipError::Exists`] when another writer
/// took the name; [`MembershipError::WouldRewrite`] when the new table
/// changes a sealed record; [`MembershipError::Corrupt`] for a current
/// generation that fails its checks.
pub fn install(dir: &Path, table: &MonthTable) -> Result<Installed, MembershipError> {
    // One writer per name, which is what `store::staging::create` asks of its
    // caller: the process id and a per-process count make it this call's own,
    // so two builds of one directory never share a temporary, whether they
    // run in two processes or on two threads of one.
    install_with(dir, table, &|name| {
        dir.join(format!(
            ".{name}.tmp.{}.{}",
            std::process::id(),
            TEMPORARIES.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        ))
    })
}

/// [`install`] with the temporary named by `temporary` from the final file
/// name: a test plants a dead writer's leftover under a name it knows.
fn install_with(
    dir: &Path,
    table: &MonthTable,
    temporary: &dyn Fn(&str) -> PathBuf,
) -> Result<Installed, MembershipError> {
    let io = |path: &Path| {
        let path = path.to_path_buf();
        move |why: std::io::Error| MembershipError::Io {
            path,
            why: why.to_string(),
        }
    };
    fs::create_dir_all(dir).map_err(io(dir))?;
    let (current, _) = generations(dir)?;
    let (year, month) = table.month();
    let bytes = table.encode();
    let next = match current.get(&(year, month)) {
        Some((generation, path)) => {
            let held = fs::read(path).map_err(io(path))?;
            if held == bytes {
                return Ok(Installed::Unchanged {
                    path: path.clone(),
                    generation: *generation,
                });
            }
            let decoded = decode::table(&held).map_err(|why| MembershipError::Corrupt {
                path: path.clone(),
                why,
            })?;
            if !table.extends(&decoded) {
                return Err(MembershipError::WouldRewrite { path: path.clone() });
            }
            generation
                .checked_add(1)
                .ok_or_else(|| MembershipError::Corrupt {
                    path: path.clone(),
                    why: "the generation counter is exhausted".to_owned(),
                })?
        }
        None => 1,
    };
    let name = file_name(year, month, next);
    let path = dir.join(&name);
    let temporary = temporary(&name);
    let staged = store::staging::create(&temporary).map_err(|why| MembershipError::Io {
        path: temporary.clone(),
        why: why.to_string(),
    })?;
    let cleared = staged.cleared;
    let published = publish(staged.file, &bytes, &temporary, &path);
    // Linked or not, the temporary is this writer's and goes: after a link
    // it is a second name of the final file, before one it is all that
    // would be left of a refused write.
    let unlinked = fs::remove_file(&temporary);
    if let Err(why) = published {
        return Err(match unlinked {
            Ok(()) => why,
            Err(left) => MembershipError::Io {
                path: temporary,
                why: format!("{why}; and its temporary could not be unlinked: {left}"),
            },
        });
    }
    // The final name exists from here: the directory is flushed whether or
    // not the temporary's second name could be unlinked, and a failed unlink
    // is still refused by name.
    fs::File::open(dir)
        .and_then(|handle| handle.sync_all())
        .map_err(io(dir))?;
    unlinked.map_err(io(&temporary))?;
    Ok(Installed::Written {
        path,
        generation: next,
        cleared,
    })
}

/// Temporaries made by this process: the last part of each one's name.
static TEMPORARIES: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Fills `file` (the staged `temporary`) with `bytes`, flushes it and links
/// it to `path`. A name another writer took first is [`MembershipError::Exists`].
fn publish(
    mut file: fs::File,
    bytes: &[u8],
    temporary: &Path,
    path: &Path,
) -> Result<(), MembershipError> {
    let io = |at: &Path, why: std::io::Error| MembershipError::Io {
        path: at.to_path_buf(),
        why: why.to_string(),
    };
    file.write_all(bytes).map_err(|why| io(temporary, why))?;
    file.sync_all().map_err(|why| io(temporary, why))?;
    drop(file);
    match fs::hard_link(temporary, path) {
        Ok(()) => Ok(()),
        Err(why) if why.kind() == std::io::ErrorKind::AlreadyExists => {
            Err(MembershipError::Exists {
                path: path.to_path_buf(),
            })
        }
        Err(why) => Err(io(path, why)),
    }
}

/// One member of one day, as [`Membership::members`] serves it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Member {
    /// The series, stable across renames: every spelling of one series maps
    /// to it. Dense, in first-appearance order; ties in a rank go by it.
    pub series_id: u32,
    /// The spelling the series was filed under on that day: the store key of
    /// its GDFL cash series that day (`TATAGLOBAL` before the measured
    /// rename, `TATACONSUM` after; `ETERNAL` on every day, the sourced rename
    /// filing under its newest symbol).
    pub symbol_on_day: Symbol,
    /// Whether the series' newest spelling is a share of
    /// `brutex_core::universe::FNO_UNDERLYINGS` less `FNO_INDEX_UNDERLYINGS`
    /// (the 2026-08-01 snapshot, `CLAUDE.md` §1's traded surface).
    pub in_current_list: bool,
    /// Whether it listed at least one option file that day (a file may hold
    /// quotes only: "listed a file", not "traded").
    pub filed: bool,
}

/// One day of the fixed-stride index.
#[derive(Debug, Clone, Copy)]
struct Slot {
    row: DayRow,
    start: usize,
}

/// One series' membership over consecutive known days: R4-26's row shape
/// `(underlying, entry_day, exit_day, source)`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Span {
    /// The series.
    pub series_id: u32,
    /// Its newest spelling.
    pub share: Symbol,
    /// The first member day.
    pub entry: Day,
    /// The last member day.
    pub exit: Day,
    /// Refused sessions inside the span, which neither open nor close it.
    pub unknown_inside: u32,
    /// [`SOURCE_LABEL`].
    pub source: &'static str,
}

/// One calendar year's day counts, for reports (D-5162).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct YearCount {
    /// The year.
    pub year: u16,
    /// Known sessions.
    pub known: u32,
    /// Refused sessions inside the span (the running sum at its last record).
    pub refused_inside: u32,
    /// Sessions of the IST calendar outside the span: no record, refused.
    pub outside: u32,
}

/// One served month file.
#[derive(Debug, Clone)]
struct Held {
    table: MonthTable,
    path: PathBuf,
    generation: u32,
    digest: [u8; DIGEST_LEN],
}

/// Every month of a directory, chain-checked, served by day in O(1) per
/// lookup (C-PIT-01, measured by `pull::bench::fno_membership_lookup_is_flat`).
#[derive(Debug, Clone, Default)]
pub struct Membership {
    months: Vec<Held>,
    registry: Vec<Spelling>,
    by_name: HashMap<Symbol, u32>,
    newest: Vec<u32>,
    first_epoch: u32,
    /// Slots up to and including the last KNOWN one: `known_to` is slot
    /// `known_len - 1`. Slots after it are sealed records that are not known
    /// (D-5167).
    known_len: usize,
    slots: Vec<Slot>,
    members: Vec<Member>,
    words: usize,
    bits: Vec<u64>,
    max_pool: u16,
    digest: [u8; DIGEST_LEN],
    handovers: Vec<Handover>,
    temporaries: usize,
}

impl Membership {
    /// Reads every month's highest generation in `dir` and checks the chain.
    ///
    /// # Errors
    ///
    /// [`generations`]' refusals; [`MembershipError::Corrupt`] for a highest
    /// generation that fails a check or names another month (the month is
    /// refused, never served from an older generation);
    /// [`MembershipError::Stale`] for a month that does not continue the one
    /// before it (a gap, a different carry, registry or span start).
    pub fn open(dir: &Path) -> Result<Self, MembershipError> {
        let (current, temporaries) = generations(dir)?;
        let mut months = Vec::with_capacity(current.len());
        for ((year, month), (generation, path)) in current {
            let bytes = fs::read(&path).map_err(|why| MembershipError::Io {
                path: path.clone(),
                why: why.to_string(),
            })?;
            let table = decode::table(&bytes).map_err(|why| MembershipError::Corrupt {
                path: path.clone(),
                why,
            })?;
            if table.month() != (year, month) {
                return Err(MembershipError::Corrupt {
                    path,
                    why: "the header names another month than the file name".to_owned(),
                });
            }
            let mut digest = [0_u8; DIGEST_LEN];
            if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) {
                digest.copy_from_slice(tail);
            }
            months.push(Held {
                table,
                path,
                generation,
                digest,
            });
        }
        let mut out = Self::assemble(months)?;
        out.temporaries = temporaries;
        Ok(out)
    }

    /// Tables as built, in month order, served without files (tests and the
    /// build's own check). Generation 0, digest of their bytes.
    ///
    /// # Errors
    ///
    /// As [`Self::open`]'s chain checks.
    pub fn of(tables: impl IntoIterator<Item = MonthTable>) -> Result<Self, MembershipError> {
        let months = tables
            .into_iter()
            .map(|table| {
                let bytes = table.encode();
                let mut digest = [0_u8; DIGEST_LEN];
                if let Some(tail) = bytes.get(bytes.len().saturating_sub(DIGEST_LEN)..) {
                    digest.copy_from_slice(tail);
                }
                Held {
                    table,
                    path: PathBuf::new(),
                    generation: 0,
                    digest,
                }
            })
            .collect();
        Self::assemble(months)
    }

    /// The chain checks of `docs/02-store-format.md`: the registry every
    /// month continues, the series count and every handover, in month order.
    fn chain(months: &[Held]) -> Result<(Vec<Spelling>, u32, Vec<Handover>), MembershipError> {
        let mut registry: Vec<Spelling> = Vec::new();
        let mut series = 0_u32;
        let mut handovers = Vec::new();
        let stale = |held: &Held, why: &str| MembershipError::Stale {
            path: held.path.clone(),
            why: why.to_owned(),
        };
        for (at, held) in months.iter().enumerate() {
            let table = &held.table;
            let (first, _) = table
                .held()
                .ok_or_else(|| stale(held, "the month holds no record"))?;
            match at.checked_sub(1).and_then(|before| months.get(before)) {
                None => {
                    if table.known_from != first.days_from_epoch()
                        || table.rows.first().map(|row| row.status) != Some(DayStatus::Known)
                    {
                        return Err(stale(held, "the span does not start on its first record"));
                    }
                    if table.carry_in != Carry::default() {
                        return Err(stale(held, "the span's first month starts from a carry"));
                    }
                }
                Some(before) => {
                    let ended = before.table.held().map(|(_, last)| last);
                    if ended.and_then(|last| last.succ().ok()) != Some(first) {
                        return Err(stale(
                            held,
                            "a gap lies between this month and the one before",
                        ));
                    }
                    if before.table.carry_out != table.carry_in {
                        return Err(stale(
                            held,
                            "built from a carry the month before no longer ends with",
                        ));
                    }
                    if before.table.known_from != table.known_from {
                        return Err(stale(held, "it names another span start"));
                    }
                }
            }
            if u32::try_from(registry.len()).ok() != Some(table.spellings_before)
                || table.series_before != series
            {
                return Err(stale(
                    held,
                    "its registry does not continue the month before's",
                ));
            }
            for spelling in &table.added {
                let id = u32::try_from(registry.len()).unwrap_or(u32::MAX);
                let handed = table.handovers.iter().any(|one| one.new == id);
                if spelling.series_id == series && !handed {
                    series = series.saturating_add(1);
                } else if !(handed && spelling.series_id < series) {
                    return Err(stale(held, "a spelling names a series out of order"));
                }
                if registry.len() >= MAX_SPELLINGS {
                    return Err(stale(held, "more spellings than the format bound"));
                }
                registry.push(*spelling);
            }
            for one in &table.handovers {
                let old = registry.get(usize::try_from(one.old).unwrap_or(usize::MAX));
                let new = registry.get(usize::try_from(one.new).unwrap_or(usize::MAX));
                if old
                    .zip(new)
                    .is_none_or(|(old, new)| old.series_id != new.series_id)
                {
                    return Err(stale(held, "a handover joins spellings of two series"));
                }
                // A handover lies on one of this month's KNOWN records: the
                // offset from its first record indexes its rows.
                let on_known = one
                    .day
                    .days_from_epoch()
                    .checked_sub(first.days_from_epoch())
                    .and_then(|at| usize::try_from(at).ok())
                    .and_then(|at| table.rows.get(at))
                    .is_some_and(|row| row.status == DayStatus::Known);
                if !on_known {
                    return Err(stale(
                        held,
                        "a handover lies on no known record of the month",
                    ));
                }
                handovers.push(*one);
            }
            // The LAST month may end on records that are not known (D-5167): a
            // month is installed with the closed days after its last session
            // when the next month's first known day is folded, and a kill or a
            // failed write between that install and the next month's leaves it
            // last. Those records are sealed and true; `known_to` is the last
            // KNOWN record ([`Membership::known_span`]), and a resumed build
            // continues after the last sealed one.
        }
        Ok((registry, series, handovers))
    }

    /// The chain checks and the day index.
    fn assemble(months: Vec<Held>) -> Result<Self, MembershipError> {
        let (registry, series, handovers) = Self::chain(&months)?;
        let stale = |held: &Held, why: &str| MembershipError::Stale {
            path: held.path.clone(),
            why: why.to_owned(),
        };
        let mut by_name = HashMap::with_capacity(registry.len());
        let mut newest = vec![0_u32; usize::try_from(series).unwrap_or(0)];
        for (id, spelling) in (0_u32..).zip(&registry) {
            if by_name.insert(spelling.name, id).is_some() {
                return Err(MembershipError::Corrupt {
                    path: PathBuf::new(),
                    why: format!("{} is registered twice", spelling.name),
                });
            }
            if let Some(slot) = usize::try_from(spelling.series_id)
                .ok()
                .and_then(|at| newest.get_mut(at))
            {
                *slot = id;
            }
        }
        let current: HashSet<&str> = FNO_UNDERLYINGS
            .iter()
            .copied()
            .filter(|name| !FNO_INDEX_UNDERLYINGS.contains(name))
            .collect();
        let in_list: Vec<bool> = newest
            .iter()
            .map(|id| {
                registry
                    .get(usize::try_from(*id).unwrap_or(usize::MAX))
                    .is_some_and(|spelling| current.contains(spelling.name.as_str()))
            })
            .collect();
        let words = newest.len().div_ceil(64).max(1);
        let mut out = Self {
            first_epoch: months.first().map_or(0, |held| held.table.known_from),
            words,
            by_name,
            newest,
            handovers,
            ..Self::default()
        };
        let mut hasher = brutex_core::blake3::Hasher::new();
        hasher.update(b"fno-membership-v1 set");
        for held in &months {
            hasher.update(&held.table.year.to_le_bytes());
            hasher.update(&[held.table.month]);
            hasher.update(&held.digest);
            out.max_pool = out.max_pool.max(held.table.max_pool);
            let mut at = 0_usize;
            for row in &held.table.rows {
                let end = at + usize::from(row.pool);
                let start = out.members.len();
                let day_bits = out.bits.len();
                out.bits.resize(day_bits + words, 0);
                let mut previous: Option<u32> = None;
                // The decoder checks this for a file; a table served without
                // one (`of`) is refused here rather than served short.
                let day =
                    held.table.entries.get(at..end).ok_or_else(|| {
                        stale(held, "a day's members run past the month's entries")
                    })?;
                for entry in day {
                    let spelling = registry
                        .get(usize::from(entry.spelling))
                        .ok_or_else(|| stale(held, "an entry names no spelling"))?;
                    if previous.is_some_and(|previous| previous >= spelling.series_id) {
                        return Err(stale(held, "a day's members are not in series order"));
                    }
                    previous = Some(spelling.series_id);
                    let series_at = usize::try_from(spelling.series_id).unwrap_or(usize::MAX);
                    if let Some(word) = out.bits.get_mut(day_bits + series_at / 64) {
                        *word |= 1_u64 << (series_at % 64);
                    }
                    out.members.push(Member {
                        series_id: spelling.series_id,
                        symbol_on_day: spelling.name,
                        in_current_list: in_list.get(series_at).copied().unwrap_or(false),
                        filed: entry.filed,
                    });
                }
                out.slots.push(Slot { row: *row, start });
                if row.status == DayStatus::Known {
                    out.known_len = out.slots.len();
                }
                at = end;
            }
        }
        out.digest = hasher.finalize();
        out.registry = registry;
        out.months = months;
        Ok(out)
    }

    /// The span: the first and last day of a stored stock-derivative record,
    /// `(known_from, known_to)`. `known_to` is the last KNOWN record, which a
    /// directory a kill interrupted may hold sealed closed days after
    /// ([`Self::sealed_through`], D-5167).
    #[must_use]
    pub fn known_span(&self) -> Option<(Day, Day)> {
        let from = Day::from_days(self.first_epoch).ok()?;
        let len = u32::try_from(self.known_len.checked_sub(1)?).ok()?;
        Some((
            from,
            Day::from_days(self.first_epoch.checked_add(len)?).ok()?,
        ))
    }

    /// The last sealed record, known or not: where a resumed build continues.
    /// It is `known_to` except in a directory whose last month was installed
    /// with the closed days after its last session and whose next month was
    /// not, because a kill or a failed write came between (D-5167).
    #[must_use]
    pub fn sealed_through(&self) -> Option<Day> {
        let len = u32::try_from(self.slots.len().checked_sub(1)?).ok()?;
        Day::from_days(self.first_epoch.checked_add(len)?).ok()
    }

    /// The largest pool of any held day: a rank network sizes itself to the
    /// next power of two at or above it (D-5165 item 2).
    #[must_use]
    pub const fn max_pool(&self) -> u16 {
        self.max_pool
    }

    /// BLAKE3 over every served month's year, month and file digest, in month
    /// order: the membership a run read, for its identity. The generation is
    /// not hashed, so one pass and a day-by-day build give the same digest.
    #[must_use]
    pub const fn file_digest(&self) -> [u8; DIGEST_LEN] {
        self.digest
    }

    /// [`Self::file_digest`] in lower-case hex.
    #[must_use]
    pub fn file_digest_hex(&self) -> String {
        self.digest.iter().fold(String::new(), |mut out, byte| {
            let _ = fmt::Write::write_fmt(&mut out, format_args!("{byte:02x}"));
            out
        })
    }

    /// Hidden temporaries a crashed writer left in the directory.
    #[must_use]
    pub const fn temporaries(&self) -> usize {
        self.temporaries
    }

    /// Each served month: `(year, month)`, generation, path.
    #[must_use]
    pub fn months(&self) -> Vec<((u16, u8), u32, &Path)> {
        self.months
            .iter()
            .map(|held| (held.table.month(), held.generation, held.path.as_path()))
            .collect()
    }

    /// Every spelling, by registry id.
    #[must_use]
    pub fn registry(&self) -> &[Spelling] {
        &self.registry
    }

    /// The spelling at registry id `id`.
    #[must_use]
    pub fn spelling(&self, id: u32) -> Option<&Spelling> {
        self.registry.get(usize::try_from(id).ok()?)
    }

    /// The newest spelling of series `series_id`.
    #[must_use]
    pub fn series(&self, series_id: u32) -> Option<&Spelling> {
        let id = self.newest.get(usize::try_from(series_id).ok()?)?;
        self.spelling(*id)
    }

    /// Series held.
    #[must_use]
    pub fn series_count(&self) -> usize {
        self.newest.len()
    }

    /// Every measured handover, in day order.
    #[must_use]
    pub fn handovers(&self) -> &[Handover] {
        &self.handovers
    }

    /// The fixed-stride slot of `day` and its position, whatever its status,
    /// inside the span. A sealed record after `known_to` answers `AfterSpan`
    /// like a day no record reaches, so an interrupted directory answers
    /// every day exactly as the one-pass directory of the same source does
    /// (D-5167). Constant work: one subtraction and one bounds check.
    fn slot(&self, day: Day) -> Result<(usize, &Slot), MembershipUnknown> {
        let unknown = |why| MembershipUnknown { day, why };
        let Some((known_from, known_to)) = self.known_span() else {
            return Err(unknown(UnknownWhy::Empty));
        };
        let at = day
            .days_from_epoch()
            .checked_sub(self.first_epoch)
            .and_then(|at| usize::try_from(at).ok())
            .ok_or(unknown(UnknownWhy::BeforeSpan { known_from }))?;
        self.slots
            .get(at)
            .filter(|_| at < self.known_len)
            .map(|slot| (at, slot))
            .ok_or(unknown(UnknownWhy::AfterSpan { known_to }))
    }

    /// The known slot of `day`.
    fn known(&self, day: Day) -> Result<(usize, &Slot), MembershipUnknown> {
        let (at, slot) = self.slot(day)?;
        if slot.row.status == DayStatus::Known {
            Ok((at, slot))
        } else {
            Err(MembershipUnknown {
                day,
                why: UnknownWhy::Refused(slot.row.status),
            })
        }
    }

    /// The sealed row of `day`. O(1) (C-PIT-01, measured by
    /// `pull::bench::fno_membership_lookup_is_flat`).
    ///
    /// # Errors
    ///
    /// [`MembershipUnknown`] outside the span.
    pub fn row(&self, day: Day) -> Result<DayRow, MembershipUnknown> {
        self.slot(day).map(|(_, slot)| slot.row)
    }

    /// The members of `day`, ascending by series. O(1): one subtraction into
    /// the fixed-stride day index, one slice (C-PIT-01, measured by
    /// `pull::bench::fno_membership_lookup_is_flat`).
    ///
    /// # Errors
    ///
    /// [`MembershipUnknown`] for a day outside the span, or a sealed record
    /// without an answer.
    pub fn members(&self, day: Day) -> Result<&[Member], MembershipUnknown> {
        let (_, slot) = self.known(day)?;
        let end = slot.start + usize::from(slot.row.pool);
        Ok(self.members.get(slot.start..end).unwrap_or_default())
    }

    /// The pool size of `day`. O(1): one subtraction, one row (C-PIT-01,
    /// measured by `pull::bench::fno_membership_lookup_is_flat`).
    ///
    /// # Errors
    ///
    /// As [`Self::members`].
    pub fn pool_size(&self, day: Day) -> Result<u16, MembershipUnknown> {
        self.known(day).map(|(_, slot)| slot.row.pool)
    }

    /// The series a name belongs to, if it was ever a member: the newest
    /// symbol NSE's dated changes reach, then the registry. `Ok(None)` for a
    /// share never seen. O(1): a bounded alias walk and one hash probe
    /// (C-PIT-01, measured by `pull::bench::fno_membership_lookup_is_flat`, through `is_member`).
    ///
    /// # Errors
    ///
    /// [`MembershipError::BadName`], [`MembershipError::NotAShare`].
    pub fn series_of(&self, underlying: &str) -> Result<Option<u32>, MembershipError> {
        if option_underlying(underlying).segment == Segment::Index {
            return Err(MembershipError::NotAShare {
                name: underlying.to_owned(),
            });
        }
        let current = LINEAGE.current(underlying);
        let symbol = Symbol::new(current)
            .ok()
            .filter(|share| share.as_str() == current)
            .ok_or_else(|| MembershipError::BadName {
                name: underlying.to_owned(),
            })?;
        Ok(self
            .by_name
            .get(&symbol)
            .and_then(|id| self.spelling(*id))
            .map(|spelling| spelling.series_id))
    }

    /// Whether `underlying` (any of its spellings) had a live stock-option
    /// contract on `day`. O(1): one alias walk, one probe, one bit
    /// (C-PIT-01, measured by `pull::bench::fno_membership_lookup_is_flat`).
    ///
    /// # Errors
    ///
    /// [`MembershipError::Unknown`] for a day without an answer, and
    /// [`Self::series_of`]'s refusals.
    pub fn is_member(&self, underlying: &str, day: Day) -> Result<bool, MembershipError> {
        let series = self.series_of(underlying)?;
        let (at, _) = self.known(day)?;
        let Some(series) = series.and_then(|id| usize::try_from(id).ok()) else {
            return Ok(false);
        };
        Ok(self
            .bits
            .get(at * self.words + series / 64)
            .is_some_and(|word| word & (1_u64 << (series % 64)) != 0))
    }

    /// Refused sessions of `day`'s year inside the span, through `day`: the
    /// running sum, O(1) (C-PIT-01, measured by
    /// `pull::bench::fno_membership_lookup_is_flat`).
    ///
    /// # Errors
    ///
    /// [`MembershipUnknown`] outside the span.
    pub fn refused_through(&self, day: Day) -> Result<u16, MembershipUnknown> {
        self.row(day).map(|row| row.refused_in_year)
    }

    /// Every calendar year of the IST calendar, counted (a report: O(days of
    /// the calendar), not a lookup).
    #[must_use]
    pub fn year_counts(&self) -> Vec<YearCount> {
        let mut out: BTreeMap<u16, YearCount> = BTreeMap::new();
        let (Ok(first), Ok(last)) = (
            u32::try_from(SOURCED_FIRST_DAY),
            u32::try_from(SOURCED_LAST_DAY),
        ) else {
            return Vec::new();
        };
        for epoch in first..=last {
            let Ok(day) = Day::from_days(epoch) else {
                continue;
            };
            let count = out.entry(day.year()).or_insert(YearCount {
                year: day.year(),
                known: 0,
                refused_inside: 0,
                outside: 0,
            });
            match self.row(day) {
                Ok(row) => {
                    if row.status == DayStatus::Known {
                        count.known += 1;
                    }
                    count.refused_inside = u32::from(row.refused_in_year);
                }
                Err(_) => {
                    if matches!(
                        sourced_kind_of(i64::from(epoch)),
                        DayKind::Open(_) | DayKind::OpenLengthUnmeasured
                    ) {
                        count.outside += 1;
                    }
                }
            }
        }
        out.into_values().collect()
    }

    /// Every series' spans over the span, in close order: a span opens on a
    /// known member day and closes on the next known day it is not a member;
    /// refused sessions are counted inside it. Not a hot path: O(days ×
    /// pool).
    #[must_use]
    pub fn spans(&self) -> Vec<Span> {
        let mut open: BTreeMap<u32, Span> = BTreeMap::new();
        let mut out = Vec::new();
        for (offset, slot) in (0_u32..).zip(&self.slots) {
            let Ok(day) = Day::from_days(self.first_epoch.saturating_add(offset)) else {
                continue;
            };
            if slot.row.status == DayStatus::Known {
                let today = self.members(day).unwrap_or_default();
                let live: BTreeSet<u32> = today.iter().map(|member| member.series_id).collect();
                let closed: Vec<u32> = open
                    .keys()
                    .filter(|id| !live.contains(id))
                    .copied()
                    .collect();
                for id in closed {
                    out.extend(open.remove(&id));
                }
                for id in live {
                    let Some(share) = self.series(id).map(|spelling| spelling.name) else {
                        continue;
                    };
                    open.entry(id)
                        .and_modify(|span| span.exit = day)
                        .or_insert(Span {
                            series_id: id,
                            share,
                            entry: day,
                            exit: day,
                            unknown_inside: 0,
                            source: SOURCE_LABEL,
                        });
                }
            } else if slot.row.status.is_refused_session() {
                for span in open.values_mut() {
                    span.unknown_inside = span.unknown_inside.saturating_add(1);
                }
            }
        }
        out.extend(open.into_values());
        out
    }

    /// The fold's state after the last sealed record, to resume from. The
    /// last session's listing is re-read from `source` when that session was
    /// known, and must match its sealed evidence.
    ///
    /// # Errors
    ///
    /// [`MembershipError::SourceChanged`] when it does not.
    pub fn resume_state<S: NfoSource>(&self, source: &S) -> Result<State, MembershipError> {
        let Some(last) = self.months.last() else {
            return Ok(State::default());
        };
        let carry = &last.table.carry_out;
        let mut state = State {
            registry: self.registry.clone(),
            by_name: self.by_name.clone(),
            series: u32::try_from(self.newest.len()).unwrap_or(u32::MAX),
            newest: self.newest.clone(),
            live: carry
                .live
                .iter()
                .map(|(series, until)| {
                    (
                        *series,
                        Live {
                            until: *until,
                            filed_on: 0,
                        },
                    )
                })
                .collect(),
            expiry_start: carry.expiry_start.iter().copied().collect(),
            year: carry.year,
            refused_in_year: carry.refused_in_year,
            previous: None,
            known_from: self.known_span().map(|(from, _)| from),
            last: self.sealed_through(),
        };
        let previous = self
            .slots
            .iter()
            .enumerate()
            .rev()
            .find(|(_, slot)| slot.row.status != DayStatus::NotSession);
        if let Some((offset, slot)) = previous
            && slot.row.status == DayStatus::Known
        {
            let day = u32::try_from(offset)
                .ok()
                .and_then(|offset| self.first_epoch.checked_add(offset))
                .and_then(|epoch| Day::from_days(epoch).ok())
                .ok_or_else(|| corrupt_path(&last.path, "the last record is not a day"))?;
            match read_day(source, day) {
                DayInput::Listed(listed) if listed.digest() == slot.row.evidence => {
                    state.previous = Some(listed);
                }
                _ => return Err(MembershipError::SourceChanged { day }),
            }
        }
        Ok(state)
    }

    /// The month table of the last served month, when its records end before
    /// the month does: a resumed build extends it.
    #[must_use]
    pub fn open_tail(&self) -> Option<MonthTable> {
        let held = self.months.last()?;
        let (_, last) = held.table.held()?;
        (last != last.end_of_month()).then(|| held.table.clone())
    }
}

/// A corrupt-file refusal.
fn corrupt_path(path: &Path, why: &str) -> MembershipError {
    MembershipError::Corrupt {
        path: path.to_path_buf(),
        why: why.to_owned(),
    }
}

/// What a build covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Window {
    /// After the last sealed record (or the IST calendar's first day) to the
    /// calendar's last day: rows end at the last known day, `known_to`.
    Resume,
    /// A typed window, for synthetic stores and scratch copies only: `from`
    /// (`None` = after the last sealed record) to `to`.
    Typed {
        /// The first day, or the resume point.
        from: Option<Day>,
        /// The last day probed.
        to: Day,
    },
}

/// One month a build finished.
#[derive(Debug, Clone)]
pub struct MonthBuilt {
    /// The table installed.
    pub table: MonthTable,
    /// What [`install`] did.
    pub installed: Installed,
    /// Each refused day with the source's own words, where it gave some.
    pub unknown: Vec<(Day, DayStatus, Option<String>)>,
    /// Each listed day's refused-name count and sample.
    pub refusals: Vec<(Day, u32, Vec<String>)>,
    /// Handover candidates not joined, by day, each with why.
    pub ambiguous: Vec<(Day, String)>,
    /// Files listed under a spelling its series moved on from, by day: filed
    /// under the newest spelling (D-5166).
    pub strays: Vec<(Day, String)>,
    /// Handovers, with names.
    pub handovers: Vec<(Handover, Symbol, Symbol)>,
    /// Milliseconds spent reading and folding the month's new days.
    pub millis: u128,
}

/// What a build did.
#[derive(Debug, Clone)]
pub struct Built {
    /// The days probed.
    pub probed: (Day, Day),
    /// Days probed before the span's first record (a fresh build only), with
    /// their status: outside the span, refused, never written.
    pub leading: Vec<(Day, DayStatus, Option<String>)>,
    /// Days probed after the last known day: not written, resumed next time.
    pub trailing: Vec<(Day, DayStatus, Option<String>)>,
    /// Records folded.
    pub folded: usize,
}

/// Builds `dir` from `source` over `window`: resumes after the last sealed
/// record, folds each day once in order (`workers` readers per month), writes
/// each month touched as its next generation, and leaves trailing days
/// without a known record unwritten. `on_month` is called once per month
/// written or found unchanged.
///
/// # Errors
///
/// [`Membership::open`]'s refusals for the directory as held (a directory
/// that does not exist yet is a fresh build);
/// [`MembershipError::Overlap`] for a typed start on or before the last
/// sealed record; [`MembershipError::Gap`] for one after the day following
/// it; [`MembershipError::BadWindow`]; [`MembershipError::SourceRefused`]
/// for a day the source refused, AFTER every record before it is installed
/// (D-5168); the fold's and [`install`]'s.
pub fn build<S: NfoSource + Sync>(
    source: &S,
    dir: &Path,
    window: Window,
    workers: usize,
    on_month: &mut dyn FnMut(&MonthBuilt),
) -> Result<Built, MembershipError> {
    let held = match fs::symlink_metadata(dir) {
        Err(why) if why.kind() == std::io::ErrorKind::NotFound => Membership::default(),
        _ => Membership::open(dir)?,
    };
    let mut state = held.resume_state(source)?;
    let (from, to) = bounds(window, state.last)?;
    let mut open = held.open_tail();
    let mut pending: Vec<(Day, DayInput)> = Vec::new();
    let mut month = MonthWork::default();
    let mut built = Built {
        probed: (from, to),
        leading: Vec::new(),
        trailing: Vec::new(),
        folded: 0,
    };
    let mut cursor = from;
    let mut refused: Option<(Day, String)> = None;
    'months: loop {
        let end = cursor.end_of_month().min(to);
        let days = days_between(cursor, end);
        let clock = std::time::Instant::now();
        let inputs = read_days(source, &days, workers);
        for (day, input) in days.into_iter().zip(inputs) {
            // A refused day is never sealed (D-5168): the build stops before
            // it, installs what it folded, and the next build resumes at it.
            if let DayInput::Unknown(DayStatus::SourceRefused, why) = &input {
                refused = Some((day, why.clone().unwrap_or_default()));
                month.millis += clock.elapsed().as_millis();
                break 'months;
            }
            if !input.is_known() {
                pending.push((day, input));
                continue;
            }
            if state.known_from.is_none() {
                built
                    .leading
                    .extend(pending.drain(..).map(|(day, input)| unknown_of(day, input)));
            }
            for (day, input) in pending.drain(..).chain(std::iter::once((day, input))) {
                if open
                    .as_ref()
                    .is_some_and(|table| table.month() != (day.year(), day.month()))
                    && let Some(table) = open.take()
                {
                    // A held tail this run added nothing to is left as it is.
                    let work = std::mem::take(&mut month);
                    if work.touched {
                        finish(dir, table, work, on_month)?;
                    }
                }
                let table = open.get_or_insert_with(|| MonthTable::begin(day, &state));
                let record = state.fold(day, input)?;
                month.note(&record, &state);
                table.push(record, &state)?;
                built.folded += 1;
            }
        }
        month.millis += clock.elapsed().as_millis();
        if end >= to {
            break;
        }
        cursor = end
            .succ()
            .map_err(|_| MembershipError::BadWindow { from, to })?;
    }
    built.trailing = pending
        .into_iter()
        .map(|(day, input)| unknown_of(day, input))
        .collect();
    if let Some(table) = open
        && month.touched
    {
        finish(dir, table, month, on_month)?;
    }
    match refused {
        Some((day, why)) => Err(MembershipError::SourceRefused { day, why }),
        None => Ok(built),
    }
}

/// The days `window` covers after the last sealed record `last` (`None` for a
/// fresh build), inside the IST calendar's sourced bounds.
///
/// # Errors
///
/// [`MembershipError::Overlap`] for a typed start on or before `last`;
/// [`MembershipError::Gap`] for one after the day following it;
/// [`MembershipError::BadWindow`] for a window that is empty or leaves the
/// calendar.
fn bounds(window: Window, last: Option<Day>) -> Result<(Day, Day), MembershipError> {
    let calendar_first = calendar_day(SOURCED_FIRST_DAY)?;
    let calendar_last = calendar_day(SOURCED_LAST_DAY)?;
    let resume = match last {
        Some(last) => last.succ().map_err(|_| MembershipError::BadWindow {
            from: last,
            to: last,
        })?,
        None => calendar_first,
    };
    let (from, to) = match window {
        Window::Resume => (resume, calendar_last),
        Window::Typed { from, to } => {
            let from = from.unwrap_or(resume);
            if let Some(last) = last {
                if from <= last {
                    return Err(MembershipError::Overlap { from, last });
                }
                if from != resume {
                    return Err(MembershipError::Gap { from, last });
                }
            }
            (from, to)
        }
    };
    if from > to || from < calendar_first || to > calendar_last {
        return Err(MembershipError::BadWindow { from, to });
    }
    Ok((from, to))
}

/// A calendar bound as a day.
fn calendar_day(epoch: i64) -> Result<Day, MembershipError> {
    u32::try_from(epoch)
        .ok()
        .and_then(|epoch| Day::from_days(epoch).ok())
        .ok_or_else(|| corrupt_path(Path::new(""), "the IST calendar's bounds are not days"))
}

/// Every day of `from..=to`.
fn days_between(from: Day, to: Day) -> Vec<Day> {
    (from.days_from_epoch()..=to.days_from_epoch())
        .filter_map(|epoch| Day::from_days(epoch).ok())
        .collect()
}

/// An input without a record, for reports.
fn unknown_of(day: Day, input: DayInput) -> (Day, DayStatus, Option<String>) {
    match input {
        DayInput::Listed(_) => (day, DayStatus::NoShareOptions, None),
        DayInput::Unknown(status, why) => (day, status, why),
    }
}

/// What one month's new records said, for its report.
#[derive(Debug, Default)]
struct MonthWork {
    touched: bool,
    unknown: Vec<(Day, DayStatus, Option<String>)>,
    refusals: Vec<(Day, u32, Vec<String>)>,
    ambiguous: Vec<(Day, String)>,
    strays: Vec<(Day, String)>,
    handovers: Vec<(Handover, Symbol, Symbol)>,
    millis: u128,
}

impl MonthWork {
    fn note(&mut self, record: &Record, state: &State) {
        self.touched = true;
        let status = record.row.status;
        if status != DayStatus::Known && status != DayStatus::NotSession {
            self.unknown.push((record.day, status, record.why.clone()));
        }
        if record.row.refused > 0 {
            self.refusals.push((
                record.day,
                record.row.refused,
                record.refused_sample.clone(),
            ));
        }
        self.ambiguous
            .extend(record.ambiguous.iter().map(|why| (record.day, why.clone())));
        self.strays
            .extend(record.strays.iter().map(|why| (record.day, why.clone())));
        for one in &record.handovers {
            if let (Some(old), Some(new)) =
                (state.spelling_named(one.old), state.spelling_named(one.new))
            {
                self.handovers.push((*one, old, new));
            }
        }
    }
}

/// Installs one finished month and reports it.
fn finish(
    dir: &Path,
    table: MonthTable,
    work: MonthWork,
    on_month: &mut dyn FnMut(&MonthBuilt),
) -> Result<(), MembershipError> {
    let installed = install(dir, &table)?;
    on_month(&MonthBuilt {
        table,
        installed,
        unknown: work.unknown,
        refusals: work.refusals,
        ambiguous: work.ambiguous,
        strays: work.strays,
        handovers: work.handovers,
        millis: work.millis,
    });
    Ok(())
}

#[cfg(test)]
#[path = "fno_membership_tests.rs"]
mod tests;
