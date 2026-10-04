//! GDFL capital-market tick files: the day, the file name, the ticker and
//! every row, read from a [`CmSource`] and checked before anything folds
//! them. D-0808, D-2800.
//!
//! # What this reads
//!
//! The operator's GDFL capital-market history is the vendor's outer zip of
//! stored day zips, one per tree and trading day (D-2800: read in place; the
//! extracted folders were retired on 2026-10-04),
//! `INDICES/<yyyy>/<MON_yyyy>/GFDLCM_INDICES_TICK_<ddmmyyyy>.zip` and
//! `STOCKS/<yyyy>/<MON_yyyy>/GFDLCM_STOCK_TICK_<ddmmyyyy>.zip`, each holding
//! one day folder of the same name with one CSV per series named `<Ticker>.csv` or, in nine September 2018 index folders,
//! `<Ticker>.CSV`. The
//! ticker carries its own suffix: `NIFTY 50.NSE_IDX` for an index and
//! `RELIANCE.NSE` for a cash equity. Every row is
//!
//! `Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest`
//!
//! with the date `DD/MM/YYYY` and the time a whole second, several rows to a
//! second, pre-open rows first (the first row is stamped 09:07:01 to 09:09:52
//! in 3,978 of the 4,002 NIFTY 50 and NIFTY BANK files) and post-close rows
//! after 15:30. Over those 4,002 files the largest stamp is 15:31:59 or later
//! in all but the four files of the two disaster-recovery Saturdays, which
//! [`session_gate`] refuses first, and 16:00:00 or later in all but 26. Every
//! line ends CRLF except the unterminated last line of the 34 index files of
//! 2018 that have no final newline. Every one of those facts was read off the
//! operator's copy and archive and is recorded, with how it was measured, in
//! `docs/00-charter.md`, the GDFL capital-market section; the one-second fill
//! design (held outside this repository) is cited only for where a rule came
//! from.
//! The tests reproduce each SHAPE with invented values and never quote a row:
//! the repository is public and the licence for the files is UNVERIFIED
//! (design §12, revision 5; `tests::fixtures_are_built_not_pasted`).
//!
//! # What it hands on, and what it does not decide
//!
//! [`decode`] returns every row in FILE ORDER with its second of the IST day,
//! its last traded price in paisa and its traded quantity. It never sorts,
//! never drops a row and never re-stamps one: placing a late row, re-stamping a
//! UTC row and filtering stock rows by LTQ are the fold's rules
//! (`pull::fold`, D-0805: a forward reference, not yet in the ledger), and a reader that did any of them would be a second
//! fold authority. What it does decide, and refuses by name, is whether the
//! bytes are the file they claim to be: the header, the ticker, the date, every
//! field's shape, and whether a single row would reach the session at all.
//!
//! # Prices: two places exactly, and a third decimal refused
//!
//! `CLAUDE.md` §7: prices are paisa integers and the tick grid is two decimal
//! places. Design §3.3 decodes the LTP with `csv::paisa`, the F&O reader's own
//! conversion, so a third decimal REFUSES (`PriceRefused`) rather than being
//! snapped: a third digit is the vendor sending something this build does
//! not understand. Two places or fewer convert exactly, so no snap happens
//! here at all and the two GDFL readers treat a third decimal alike (round-4
//! review; D-0808). Over every NIFTY 50 and NIFTY BANK file of the copy no
//! LTP is anything but a plain decimal of at most two places (charter, GDFL
//! section, measured over the whole copy); INDIA VIX carries four and is
//! never mapped (see [`resolve`]).
//!
//! # Which files are the swept surface
//!
//! [`resolve`] maps exact stem bytes: `NIFTY 50.NSE_IDX` is `NSE-NIFTY`,
//! `NIFTY BANK.NSE_IDX` is `NSE-BANKNIFTY`, and `<SYM>.NSE` is the cash equity
//! `SYM` when `SYM` is one of the F&O shares. Membership is a bounded probe of
//! [`brutex_core::universe::FNO_INDEX`], the table built at compile time from
//! the one list `CLAUDE.md` §1 names, so this module holds no second copy of
//! it. A series file (`M&MFIN.N1.NSE`, `IDEA.BE.NSE`) has a dot inside its
//! symbol and so is never a share's own series.
//!
//! # Cost
//!
//! [`resolve`] is worst-case O(1) in its argument: one suffix comparison;
//! for an index, a match against three fixed literals, then
//! `InstrumentKey::index` and `is_sweepable`; for an equity, a length check
//! that refuses any name longer than `SYMBOL_CAPACITY` before it is read
//! (round-4 review: without it the dot search was O(stem length)), then the
//! dot search and the key build over at most that many bytes, then one
//! bounded probe of `FNO_INDEX`, which
//! `core::universe::tests::a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too`
//! pins at twelve slots. [`stem_slot`] adds [`ticker_slot`]'s `is_sweepable`
//! check and its `FNO_INDEX.position` for an equity stem; still worst-case
//! O(1). [`decode`] is O(bytes): one pass, ten fields, no
//! per-row search. A [`DayListing`] is built in O(entries) once per day by
//! its [`CmSource`], after which [`DayListing::locate`] is one index into a
//! ticker-slot array (design §3.3, revision 8).
//! Those two are UNVERIFIED as measured times: no bench times them, and
//! `docs/06-limits.md` records both as limits, not constant-time claims.

use brutex_core::instrument::{Exchange, InstrumentKey, Kind};
use brutex_core::symbol::SYMBOL_CAPACITY;

use crate::calendar::{DayKind, Session, kind_of};
use crate::session::{Day, SESSION_CLOSE_MINUTE, SESSION_OPEN_MINUTE};

/// Which of the two capital-market trees a folder belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CmKind {
    /// `GFDLCM_INDICES_TICK_<ddmmyyyy>`: index levels, one `.NSE_IDX` file each.
    Indices,
    /// `GFDLCM_STOCK_TICK_<ddmmyyyy>`: cash equities, one `.NSE` file each.
    Stocks,
}

/// The folder prefix and the ticker suffix of one capital-market tree.
///
/// Beside the F&O archive descriptor in `pull::vendor`, not inside it: that
/// table is indexed by feed, and the capital-market files are the same feed
/// laid out differently, so a second `Descriptor` for one feed would break the
/// table's one-row-per-feed rule. D-0808.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmDescriptor {
    /// Which tree.
    pub kind: CmKind,
    /// The day folder's name before its `ddmmyyyy`.
    pub folder_prefix: &'static str,
    /// The ticker's suffix, which is also the file stem's.
    pub stem_suffix: &'static str,
}

/// The index tree, as observed.
pub const INDICES: CmDescriptor = CmDescriptor {
    kind: CmKind::Indices,
    folder_prefix: "GFDLCM_INDICES_TICK_",
    stem_suffix: ".NSE_IDX",
};

/// The cash-equity tree, as observed.
pub const STOCKS: CmDescriptor = CmDescriptor {
    kind: CmKind::Stocks,
    folder_prefix: "GFDLCM_STOCK_TICK_",
    stem_suffix: ".NSE",
};

impl CmKind {
    /// This tree's descriptor.
    #[must_use]
    pub const fn descriptor(self) -> CmDescriptor {
        match self {
            Self::Indices => INDICES,
            Self::Stocks => STOCKS,
        }
    }
}

/// The file extension's spelling, recorded per day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExtVariant {
    /// `.csv`: every folder but those nine.
    Lower,
    /// `.CSV`: every file of nine September 2018 index folders, and no other
    /// (`docs/00-charter.md`, the GDFL capital-market section).
    Upper,
}

/// Which of the two observed headers a file opens with.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeaderVariant {
    /// `...,LTQ,OpenInterest`, the F&O archive's header too.
    OpenInterest,
    /// `...,LTQ,Open Interest`: 17 NIFTY 50 files, 2018-09-03 to 2018-10-04
    /// (`docs/00-charter.md`, the GDFL capital-market section).
    OpenInterestSpaced,
}

/// The modern header, character for character. The same literal the F&O
/// archive descriptor declares, taken from there rather than typed twice.
pub const HEADER_OPEN_INTEREST: &str = crate::vendor::GDFL_HEADER;

/// The September 2018 header: `NIFTY 50.NSE_IDX.CSV` of 2018-09-03, line 1.
pub const HEADER_OPEN_INTEREST_SPACED: &str =
    "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,Open Interest";

/// Why a folder, a file or a row was refused. Every refusal names itself; none
/// is skipped.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CmRefusal {
    /// A folder name that is neither tree's prefix plus a real `ddmmyyyy`, or
    /// a day listing that is not of the instrument's own tree.
    FolderUnknown,
    /// A file whose stem is the one wanted but whose extension is neither
    /// `csv` nor `CSV`, alone or beside another name for the same ticker.
    ExtensionUnknown,
    /// Two names in one folder for one ticker, each spelled `.csv` or `.CSV`:
    /// names that differ only in the case of the extension, or one name twice
    /// in a zip. Cannot happen in a folder on a case-insensitive volume; kept
    /// for a case-sensitive one and for a day zip. A second name with any
    /// other extension is [`CmRefusal::ExtensionUnknown`] instead.
    AmbiguousCaseTwins,
    /// The first line is neither observed header.
    HeaderUnknown,
    /// The Ticker field is not the file's own stem.
    TickerMismatch {
        /// One-based line number.
        line: u32,
    },
    /// The Date field is not the folder's date as `DD/MM/YYYY`.
    DateMismatch {
        /// One-based line number.
        line: u32,
    },
    /// Not ten fields, not UTF-8, a time that is not `HH:MM:SS`, or a quote,
    /// quantity or open-interest field that is not a plain number.
    MalformedRow {
        /// One-based line number.
        line: u32,
    },
    /// The LTP is not a plain decimal of at most two places (design §3.3:
    /// `csv::paisa`, more than two decimals refuses), does not fit, or is
    /// below one tick.
    PriceRefused {
        /// One-based line number.
        line: u32,
    },
    /// More rows than `pull::fetch::MAX_ROWS`.
    RowsOverCap {
        /// The bound.
        cap: usize,
    },
    /// No row a fill could use is stamped inside [09:15:00, 15:30:00): a header
    /// alone, only pre- or post-session rows, or only LTQ = 0 stock rows.
    NoSessionRows,
    /// An index file whose largest stamp is before 15:30:00. Over every NIFTY
    /// 50 and NIFTY BANK file of the copy the largest stamp is 15:31:59 or
    /// later, except on the two disaster-recovery Saturdays that
    /// [`session_gate`] refuses before any decode (charter, GDFL section), so
    /// a regular day's file whose stamps stop inside the session was cut,
    /// whatever its archive says (design §3.3, D-0812, revision 9, C45).
    IndexEndsInSession {
        /// The file's largest stamp, as a second of the IST day.
        max_sod: u32,
    },
    /// The calendar knows the day and it is not a regular full session.
    CalendarNotRegular {
        /// What the calendar says the day was.
        kind: DayKind,
    },
    /// The day is outside the span the calendar has measured.
    CalendarUnmeasured,
    /// The instrument has no GDFL capital-market stem: it is not on the swept
    /// surface.
    NotSwept,
    /// The vendor archive could not be opened or read (D-0812). There is no
    /// fallback to the unverified tree.
    ArchiveUnavailable {
        /// What the operating system said.
        kind: std::io::ErrorKind,
    },
    /// The archive's bytes are not the zip they must be, named by what was
    /// wrong with them.
    ArchiveMalformed {
        /// What was wrong.
        what: &'static str,
    },
    /// Two day zips in the archive for one tree on one day.
    ArchiveDuplicateDay {
        /// The day named twice.
        day: Day,
    },
    /// A day zip the outer archive compresses: its bytes, and so its own
    /// central directory, are not at a known offset.
    ArchiveMemberCompressed {
        /// The member's name in the outer archive.
        member: String,
    },
    /// The bytes a source returned for a file are not as long as the vendor's
    /// archive says the file is (D-0812, D-2800).
    SourceLengthMismatch {
        /// The returned bytes' length; one past the stated length when the
        /// source stopped there.
        file: u64,
        /// The member's uncompressed length.
        member: u64,
    },
    /// The bytes a source returned for a file do not have the CRC-32 the
    /// vendor's archive states (D-0812, D-2800).
    SourceCrcMismatch {
        /// The returned bytes' CRC-32.
        file: u32,
        /// The member's CRC-32.
        member: u32,
    },
    /// A day zip member stored with a method other than stored (0) or
    /// deflated (8), the only two this reader inflates (D-2800).
    ArchiveMethodUnknown {
        /// The member's name in the day zip, folder included.
        member: String,
        /// The method its local header states.
        method: u16,
    },
    /// A deflated day zip member whose stream does not inflate (D-2800).
    ArchiveMemberCorrupt {
        /// The member's name in the day zip, folder included.
        member: String,
    },
    /// A tick-store day file that is not what its format specification v1
    /// (`BRTXTS01`) states, named by the rule it breaks (D-2801).
    TickStoreMalformed {
        /// What was wrong.
        what: &'static str,
    },
    /// A tick-store day file that exists but could not be opened or read
    /// (D-2801). There is no fallback to another source.
    TickStoreUnavailable {
        /// What the operating system said.
        kind: std::io::ErrorKind,
    },
}

impl core::fmt::Display for CmRefusal {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::FolderUnknown => f.write_str(
                "folder is not GFDLCM_INDICES_TICK_ or GFDLCM_STOCK_TICK_ plus a real ddmmyyyy",
            ),
            Self::ExtensionUnknown => f.write_str("extension is neither csv nor CSV"),
            Self::AmbiguousCaseTwins => f.write_str(
                "two names for one ticker, each spelled .csv or .CSV \
                     (a case twin in a folder, or one name twice in a day zip); neither is read",
            ),
            Self::HeaderUnknown => f.write_str("first line is neither observed GDFL CM header"),
            Self::TickerMismatch { line } => {
                write!(f, "line {line}: Ticker is not the file's own stem")
            }
            Self::DateMismatch { line } => {
                write!(f, "line {line}: Date is not the folder's date")
            }
            Self::MalformedRow { line } => write!(f, "line {line}: row is malformed"),
            Self::PriceRefused { line } => {
                write!(
                    f,
                    "line {line}: LTP is not a decimal of at most two decimal places and at least one tick"
                )
            }
            Self::RowsOverCap { cap } => write!(f, "more than {cap} rows"),
            Self::NoSessionRows => {
                f.write_str("no usable row is stamped inside 09:15:00 to 15:30:00")
            }
            Self::IndexEndsInSession { max_sod } => write!(
                f,
                "index file ends inside the session, its largest stamp at second {max_sod} of the day"
            ),
            Self::CalendarNotRegular { kind } => {
                write!(f, "the calendar says {kind:?}, not a regular session")
            }
            Self::CalendarUnmeasured => f.write_str("the calendar has not measured this day"),
            Self::NotSwept => f.write_str("the instrument has no GDFL CM stem"),
            Self::ArchiveUnavailable { kind } => write!(f, "archive unavailable: {kind}"),
            Self::ArchiveMalformed { what } => write!(f, "archive malformed: {what}"),
            Self::ArchiveDuplicateDay { day } => write!(f, "two day zips for {day}"),
            Self::ArchiveMemberCompressed { member } => {
                write!(f, "{member} is compressed in the outer archive")
            }
            Self::SourceLengthMismatch { file, member } => {
                write!(
                    f,
                    "the source returned {file} bytes, the archive states {member}"
                )
            }
            Self::SourceCrcMismatch { file, member } => {
                write!(
                    f,
                    "the source returned CRC-32 {file:08x}, the archive states {member:08x}"
                )
            }
            Self::ArchiveMethodUnknown { member, method } => {
                write!(
                    f,
                    "{member} is stored with method {method}, neither stored nor deflated"
                )
            }
            Self::ArchiveMemberCorrupt { member } => write!(f, "{member} does not inflate"),
            Self::TickStoreMalformed { what } => write!(f, "tick store malformed: {what}"),
            Self::TickStoreUnavailable { kind } => write!(f, "tick store unavailable: {kind}"),
        }
    }
}

impl core::error::Error for CmRefusal {}

/// The tree and the day a day folder's name states.
///
/// # Errors
///
/// [`CmRefusal::FolderUnknown`] for any other name, including a `ddmmyyyy`
/// that is not a real date.
pub fn folder_day(name: &str) -> Result<(CmKind, Day), CmRefusal> {
    for tree in [INDICES, STOCKS] {
        if let Some(date) = name.strip_prefix(tree.folder_prefix) {
            return ddmmyyyy(date)
                .map(|day| (tree.kind, day))
                .ok_or(CmRefusal::FolderUnknown);
        }
    }
    Err(CmRefusal::FolderUnknown)
}

/// `ddmmyyyy` as a real date, or `None`. Day first: `01042024` is 1 April.
fn ddmmyyyy(text: &str) -> Option<Day> {
    let &[d1, d2, m1, m2, y1, y2, y3, y4] = text.as_bytes() else {
        return None;
    };
    let dd = digit(d1)? * 10 + digit(d2)?;
    let mm = digit(m1)? * 10 + digit(m2)?;
    let yyyy = u16::from(digit(y1)?) * 1_000
        + u16::from(digit(y2)?) * 100
        + u16::from(digit(y3)?) * 10
        + u16::from(digit(y4)?);
    Day::new(yyyy, mm, dd).ok()
}

/// One ASCII digit's value, or `None`.
fn digit(byte: u8) -> Option<u8> {
    byte.is_ascii_digit().then(|| byte - b'0')
}

/// A file name split into its stem and its extension's spelling.
///
/// # Errors
///
/// [`CmRefusal::ExtensionUnknown`] unless the name ends `.csv` or `.CSV` after
/// a non-empty stem.
pub fn split_name(name: &str) -> Result<(&str, ExtVariant), CmRefusal> {
    let (stem, ext) = name.rsplit_once('.').ok_or(CmRefusal::ExtensionUnknown)?;
    let variant = match ext {
        "csv" => ExtVariant::Lower,
        "CSV" => ExtVariant::Upper,
        _ => return Err(CmRefusal::ExtensionUnknown),
    };
    if stem.is_empty() {
        return Err(CmRefusal::ExtensionUnknown);
    }
    Ok((stem, variant))
}

/// What a stem is to the swept surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// One of the swept instruments.
    Swept(InstrumentKey),
    /// INDIA VIX: stored as reference elsewhere, never gridded here.
    ReferenceOnly,
    /// A stock series file (`.BE`, `.SM`, `.N1`, ...): never a share's own series.
    SeriesVariant,
    /// A well-formed stem for this tree that is not on the swept surface.
    NotSwept,
    /// The stem does not carry this tree's suffix.
    ForeignSuffix,
}

/// What `stem` is in tree `kind`, by its exact bytes.
#[must_use]
pub fn resolve(kind: CmKind, stem: &str) -> Resolution {
    let Some(name) = stem.strip_suffix(kind.descriptor().stem_suffix) else {
        return Resolution::ForeignSuffix;
    };
    match kind {
        // Every arm below compares or scans `name`; an index arm compares
        // against three fixed literals, which is bounded by the literals.
        CmKind::Indices => match name {
            "NIFTY 50" => swept(InstrumentKey::index(Exchange::Nse, "NIFTY")),
            "NIFTY BANK" => swept(InstrumentKey::index(Exchange::Nse, "BANKNIFTY")),
            "INDIA VIX" => Resolution::ReferenceOnly,
            _ => Resolution::NotSwept,
        },
        // Longer than any `Symbol` can be, so no share's: refused by its
        // length before anything reads it, which is what keeps this O(1) in
        // the argument (round-4 review; `core::universe::FnoIndex::position`
        // guards the same way). A series file that long is reported
        // `NotSwept`, not `SeriesVariant`: telling them apart would mean
        // searching it.
        CmKind::Stocks if name.len() > SYMBOL_CAPACITY => Resolution::NotSwept,
        // A dot inside the symbol is a series suffix (`.BE`, `.SM`, `.N1`,
        // `.GS`, `.TB`): no F&O share's own symbol carries one.
        CmKind::Stocks if name.contains('.') => Resolution::SeriesVariant,
        // `Symbol::new` uppercases, so the exact-bytes rule is kept here: a
        // stem that is not already its symbol's own spelling is no share's.
        CmKind::Stocks => match InstrumentKey::cash(Exchange::Nse, name) {
            Ok(key) if key.underlying.as_str() == name => swept(Ok(key)),
            _ => Resolution::NotSwept,
        },
    }
}

/// A built key that `is_sweepable` admits, or `NotSwept`.
///
/// `is_sweepable` is the one place the surface is decided: NIFTY and
/// BANKNIFTY by name, a cash equity by one probe of `FNO_INDEX` less the five
/// index underlyings. A symbol the key builder refuses (empty, too long, an
/// illegal byte) is not swept either.
fn swept(key: Result<InstrumentKey, brutex_core::error::InstrumentError>) -> Resolution {
    match key {
        Ok(key) if key.is_sweepable() => Resolution::Swept(key),
        _ => Resolution::NotSwept,
    }
}

/// How many slots the swept spot indices take: one per entry of
/// [`InstrumentKey::SWEPT`], the one list that names them, so the share slots
/// below are counted from that list rather than from a typed 2.
pub const INDEX_SLOTS: usize = InstrumentKey::SWEPT.len();

/// How many slots the ticker map has: the swept spot indices, then one per
/// `FNO_UNDERLYINGS` position. The five index underlyings' positions are never
/// filled (`InstrumentKey::is_sweepable` refuses them as cash), which costs
/// five empty slots and keeps the share slot plain arithmetic on the one list.
pub const TICKER_SLOTS: usize = INDEX_SLOTS + brutex_core::universe::FNO_UNDERLYINGS.len();

/// The slot and the vendor's file name of a swept index symbol, matched
/// exactly; `None` for every other symbol. A symbol added to
/// [`InstrumentKey::SWEPT`] without a row here gets no slot and no stem, so
/// it is never read as another index's file
/// (`tests::every_swept_index_has_its_own_slot_and_stem_and_no_other_index_has_one`).
fn index_ticker(symbol: &str) -> Option<(usize, &'static str)> {
    match symbol {
        "NIFTY" => Some((0, "NIFTY 50")),
        "BANKNIFTY" => Some((1, "NIFTY BANK")),
        _ => None,
    }
}

/// The ticker-map slot a swept instrument occupies, worst-case O(1): two
/// bounded probes of `FNO_INDEX` for a share (`is_sweepable`'s, then
/// `position`'s), whose bound
/// `core::universe::tests::a_miss_probes_further_than_a_hit_and_its_bound_is_measured_too`
/// pins; UNVERIFIED as a measured time (`docs/06-limits.md`).
///
/// NIFTY is slot 0 and BANKNIFTY slot 1, matched exactly by name. A swept
/// cash equity is [`INDEX_SLOTS`] plus its position in `FNO_UNDERLYINGS`,
/// read by one bounded probe of
/// [`brutex_core::universe::FNO_INDEX`], the open-addressed table built at
/// compile time from that list, so the equity keys are the universe's own
/// and no second list of them exists (design §3.3, revision 8, C43).
#[must_use]
pub fn ticker_slot(key: &InstrumentKey) -> Option<usize> {
    if !key.is_sweepable() {
        return None;
    }
    let symbol = key.underlying.as_str();
    match key.kind {
        Kind::Index => index_ticker(symbol).map(|(slot, _)| slot),
        _ => brutex_core::universe::FNO_INDEX
            .position(symbol)
            .map(|at| at + INDEX_SLOTS),
    }
}

/// The ticker-map slot of a stem found in a folder (or under a zip tree) of
/// kind `kind`, by its exact bytes; `None` for every stem off the swept
/// surface.
///
/// Two key spaces, gated by the folder's kind (design §3.3, revision 9): an
/// index key is a whole stem and is probed only for a name of an index
/// folder, an equity key only for a name of a stock folder, and only after
/// the stem has been found to end in exactly `.NSE` (not `.NSE_IDX`) and
/// that suffix removed, which [`resolve`] does. So `NIFTY 50.NSE_IDX` in a
/// stock folder and `RELIANCE.NSE` in an index folder resolve to nothing,
/// and the slot of every resolved key is of the folder's own kind.
#[must_use]
pub fn stem_slot(kind: CmKind, stem: &str) -> Option<usize> {
    match resolve(kind, stem) {
        Resolution::Swept(key) => ticker_slot(&key),
        _ => None,
    }
}

/// The tree and the stem a swept instrument is filed under, the inverse of
/// [`resolve`].
#[must_use]
pub fn stem_of(key: &InstrumentKey) -> Option<(CmKind, String)> {
    if !key.is_sweepable() {
        return None;
    }
    let symbol = key.underlying.as_str();
    match key.kind {
        Kind::Index => index_ticker(symbol)
            .map(|(_, name)| (CmKind::Indices, format!("{name}{}", INDICES.stem_suffix))),
        _ => Some((CmKind::Stocks, format!("{symbol}{}", STOCKS.stem_suffix))),
    }
}

/// What one ticker slot of a listing holds.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Held<T> {
    /// No name for this ticker.
    Empty,
    /// One name, its extension's spelling, and what it carries.
    One(Box<str>, ExtVariant, T),
    /// A name with a third spelling of the extension, alone or beside other
    /// names for the ticker: [`CmRefusal::ExtensionUnknown`] names it.
    OddExtension,
    /// Two or more names for one ticker, each spelled `.csv` or `.CSV`: on a
    /// case-insensitive volume only the two spellings of one extension, in a
    /// zip one name twice. [`CmRefusal::AmbiguousCaseTwins`] names it.
    Twins,
}

/// Names filed by their ticker-map slot: an array of [`TICKER_SLOTS`] cells,
/// so "does ticker X have a name here" is one index. Design §3.3, revision 8.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SlotTable<T> {
    /// The tree the names were found in, which decides the key space their
    /// stems are probed in (design §3.3, revision 9).
    kind: CmKind,
    cells: [Held<T>; TICKER_SLOTS],
    /// Names whose stem is off the swept surface, counted for the census.
    unresolved: usize,
    /// Every name offered.
    names: usize,
}

impl<T> SlotTable<T> {
    /// An empty table for names found in tree `kind`.
    #[must_use]
    pub fn new(kind: CmKind) -> Self {
        Self {
            kind,
            cells: std::array::from_fn(|_| Held::Empty),
            unresolved: 0,
            names: 0,
        }
    }

    /// The tree the names were found in.
    #[must_use]
    pub const fn kind(&self) -> CmKind {
        self.kind
    }

    /// Files `name` (`<stem>.<ext>`) with `value` under its stem's slot, or
    /// counts it when the stem is off the surface: one split of the name, one
    /// [`stem_slot`] and one index, so O(1) per name beyond the name's own
    /// length. The shape (one index, no hash, set, search or sort) is CM-13,
    /// proven by `pull::gdfl_cm::tests::day_folder_lookup_is_one_index_into_a_ticker_slot_array`;
    /// UNVERIFIED as a measured time (`docs/06-limits.md`).
    #[expect(
        clippy::indexing_slicing,
        reason = "`stem_slot` returns a slot of the ticker map, and every such \
                  slot is below `TICKER_SLOTS`, the array's length; \
                  `ticker_map_equity_keys_are_the_universe_shares` indexes an \
                  array of that length with every swept slot"
    )]
    pub fn file(&mut self, name: &str, value: T) {
        let Some((stem, ext)) = name.rsplit_once('.') else {
            self.skip();
            return;
        };
        let Some(slot) = stem_slot(self.kind, stem) else {
            self.skip();
            return;
        };
        self.names += 1;
        let cell = &mut self.cells[slot];
        // A third spelling of the extension anywhere among a ticker's names is
        // named as that; only names all spelled `.csv` or `.CSV` are twins.
        *cell = match (ext, &*cell) {
            ("csv", Held::Empty) => Held::One(name.into(), ExtVariant::Lower, value),
            ("CSV", Held::Empty) => Held::One(name.into(), ExtVariant::Upper, value),
            ("csv" | "CSV", Held::One(..) | Held::Twins) => Held::Twins,
            _ => Held::OddExtension,
        };
    }

    /// Counts a name that is not filed: offered, and off the surface. A day
    /// zip's member outside its day folder is counted here (CM-13).
    pub const fn skip(&mut self) {
        self.names += 1;
        self.unresolved += 1;
    }

    /// How many names were offered.
    #[must_use]
    pub const fn len(&self) -> usize {
        self.names
    }

    /// Whether no name was offered.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.names == 0
    }

    /// How many names resolve to no swept ticker: series files, INDIA VIX,
    /// any unknown stem. Report only, for the census.
    #[must_use]
    pub const fn unresolved(&self) -> usize {
        self.unresolved
    }

    /// The name filed for `stem`, its extension's spelling and its value.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::AmbiguousCaseTwins`] when two names spelled `.csv` or
    /// `.CSV` were filed for the stem, and [`CmRefusal::ExtensionUnknown`] when
    /// any name filed for it has a third spelling of the extension.
    pub fn get(&self, stem: &str) -> Result<Option<(&str, ExtVariant, &T)>, CmRefusal> {
        let held = stem_slot(self.kind, stem).and_then(|slot| self.cells.get(slot));
        match held {
            Some(Held::One(name, ext, value)) => Ok(Some((name, *ext, value))),
            Some(Held::OddExtension) => Err(CmRefusal::ExtensionUnknown),
            Some(Held::Twins) => Err(CmRefusal::AmbiguousCaseTwins),
            _ => Ok(None),
        }
    }
}

/// One file of a day as its source lists it: its name inside the day folder,
/// the length and CRC-32 the vendor's archive states for it, and where the
/// source keeps its bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ListedFile<L> {
    /// The entry's whole name, `<day folder>/<file>`, byte for byte as the
    /// source lists it.
    pub entry: Box<str>,
    /// The original file's length in bytes (the zip central directory's
    /// uncompressed size).
    pub len: u64,
    /// The original file's CRC-32 (the zip central directory's).
    pub crc32: u32,
    /// Where the source keeps the bytes; meaningful to that source only.
    pub locator: L,
}

impl<L> ListedFile<L> {
    /// The file's name inside its day folder: the entry name after the last
    /// `/`.
    #[must_use]
    pub fn name(&self) -> &str {
        self.entry
            .rsplit_once('/')
            .map_or(&*self.entry, |(_, file)| file)
    }
}

/// One day of one tree as a [`CmSource`] lists it: every entry in the
/// source's own order (a day zip's central-directory order), each file of
/// the day folder also filed by ticker slot, so "does ticker X have a file
/// this day" is one index (design §3.3, revision 8; D-2800).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayListing<L> {
    day: Day,
    folder: String,
    entries: Vec<ListedFile<L>>,
    table: SlotTable<usize>,
}

impl<L> DayListing<L> {
    /// An empty listing of tree `kind` for `day`.
    #[must_use]
    pub fn new(kind: CmKind, day: Day) -> Self {
        Self {
            day,
            folder: day_folder_name(kind, day),
            entries: Vec::new(),
            table: SlotTable::new(kind),
        }
    }

    /// Appends one entry in source order. Only `<folder>/<file>` is a file of
    /// the day and is filed by its stem; a name outside the day folder, the
    /// zip's root and the folder entry itself included, is kept in order and
    /// counted, never filed, whatever its bare name (CM-13). O(1) per entry
    /// beyond the name's own length, as [`SlotTable::file`], proven in shape
    /// by `pull::gdfl_cm::tests::day_folder_lookup_is_one_index_into_a_ticker_slot_array`;
    /// UNVERIFIED as a measured time (`docs/06-limits.md`).
    pub fn push(&mut self, entry: &str, len: u64, crc32: u32, locator: L) {
        let at = self.entries.len();
        match entry
            .strip_prefix(self.folder.as_str())
            .and_then(|rest| rest.strip_prefix('/'))
        {
            Some(file) => self.table.file(file, at),
            None => self.table.skip(),
        }
        self.entries.push(ListedFile {
            entry: entry.into(),
            len,
            crc32,
            locator,
        });
    }

    /// The tree listed.
    #[must_use]
    pub const fn kind(&self) -> CmKind {
        self.table.kind()
    }

    /// The day listed.
    #[must_use]
    pub const fn day(&self) -> Day {
        self.day
    }

    /// The day folder every file's entry name starts with:
    /// `GFDLCM_INDICES_TICK_01042024`.
    #[must_use]
    pub fn folder(&self) -> &str {
        &self.folder
    }

    /// Every entry, in the source's order.
    #[must_use]
    pub fn entries(&self) -> &[ListedFile<L>] {
        &self.entries
    }

    /// How many entries name no swept ticker's file: series files, INDIA
    /// VIX, folder entries, names outside the day folder, unknown stems.
    /// Report only, for the census.
    #[must_use]
    pub const fn unresolved(&self) -> usize {
        self.table.unresolved()
    }

    /// The file holding `stem`, if the day has one: one slot index.
    ///
    /// # Errors
    ///
    /// [`CmRefusal::AmbiguousCaseTwins`] when two entries are files of the
    /// stem, and [`CmRefusal::ExtensionUnknown`] when the stem is present
    /// under a third spelling of the extension.
    pub fn locate(&self, stem: &str) -> Result<Option<(&ListedFile<L>, ExtVariant)>, CmRefusal> {
        Ok(self
            .table
            .get(stem)?
            .and_then(|(_, ext, &at)| self.entries.get(at).map(|file| (file, ext))))
    }
}

/// `GFDLCM_INDICES_TICK_01042024`: a tree's day folder for `day`, the inverse
/// of [`folder_day`].
#[must_use]
pub fn day_folder_name(kind: CmKind, day: Day) -> String {
    format!(
        "{}{:02}{:02}{:04}",
        kind.descriptor().folder_prefix,
        day.day(),
        day.month(),
        day.year()
    )
}

/// Where the vendor's capital-market files are read from (D-2800). The
/// vendor's zips are the one implementation today
/// ([`crate::gdfl_archive::Archive`]); the planned second is the verified
/// tick store, whose listing keeps each day zip's entries in zip order with
/// the central directory's size and CRC-32, which is what this trait asks of
/// a source.
///
/// A source hands back bytes, never rows: [`read_listed`] checks every
/// fetched file against the length and CRC-32 its listing states before
/// anything parses it, so no source, present or future, can pass on bytes
/// that are not the vendor's file.
pub trait CmSource {
    /// Where the source keeps one file's bytes.
    type Locator;

    /// The listing of tree `kind` on `day`, read once per day (O(entries)),
    /// or `None` when the source holds no such day.
    ///
    /// # Errors
    ///
    /// The source's own refusals: it cannot be read, or what it holds is not
    /// what it must be.
    fn day(&self, kind: CmKind, day: Day) -> Result<Option<DayListing<Self::Locator>>, CmRefusal>;

    /// The bytes of `file`, one of `listing`'s entries, unverified: the
    /// caller checks them against `file.len` and `file.crc32`. A source may
    /// stop reading one byte past `file.len`.
    ///
    /// # Errors
    ///
    /// The source's own refusals.
    fn fetch(
        &self,
        listing: &DayListing<Self::Locator>,
        file: &ListedFile<Self::Locator>,
    ) -> Result<Vec<u8>, CmRefusal>;
}

/// The first second of the session, 09:15:00, as a second of the IST day.
const SESSION_OPEN_SOD: u32 = SESSION_OPEN_MINUTE * 60;

/// The first second after the session, 15:30:00, as a second of the IST day.
const SESSION_CLOSE_SOD: u32 = SESSION_CLOSE_MINUTE * 60;

/// What a file must be: the tree it was found in, its own stem and its
/// folder's day.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Expect<'a> {
    /// The tree, which decides whether LTQ = 0 rows count toward the session.
    pub kind: CmKind,
    /// The file's stem, which every row's Ticker must equal.
    pub stem: &'a str,
    /// The folder's day, which every row's Date must equal.
    pub day: Day,
}

/// One row, as the file stated it, in file order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CmRow {
    /// One-based line number in the file.
    pub line: u32,
    /// Second of the IST day the row is stamped at, `0..86_400`.
    pub sod: u32,
    /// Last traded price in paisa, exactly as written to two places, at least
    /// one tick.
    pub ltp: i64,
    /// Last traded quantity. Zero on every NIFTY 50 and NIFTY BANK row (charter).
    pub ltq: u64,
}

/// One decoded file: every row in file order and what was counted on the way.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CmFile {
    /// Which header the file opened with.
    pub header: HeaderVariant,
    /// Whether the last byte was a newline. A complete final row without one
    /// is accepted: 17 of 79 NIFTY 50 files of 2018 end that way.
    pub final_newline: bool,
    /// Every row, in file order, none dropped, none moved.
    pub rows: Vec<CmRow>,
    /// The largest stamp in the file AS WRITTEN, as a second of the IST day:
    /// the running maximum, which a late row never raises (design §3.3,
    /// revision 9, C45). The index end check reads this, never the last row
    /// in file order. The design judges the largest EFFECTIVE stamp, after the
    /// fold's UTC re-stamp, which can land a row up to 2 s above this; the
    /// re-stamp is the fold's (D-0805, pending), so this reader is stricter by up to
    /// 2 s and never more lenient (D-0808).
    pub max_sod: u32,
    /// Rows stamped before 09:15:00.
    pub rows_pre: u32,
    /// Rows stamped in [09:15:00, 15:30:00), whatever their LTQ.
    pub rows_in_session: u32,
    /// Rows stamped at or after 15:30:00.
    pub rows_post: u32,
    /// Rows whose LTQ is zero: every index row, and the stock rows the fold
    /// drops (D-0805, pending).
    pub ltq_zero_rows: u32,
    /// Whether a cut last row stamped after the session, with a complete
    /// post-session row before it, was dropped (design §3.3, revision 4).
    pub tail_truncated_post_session: bool,
    /// The file's length in bytes.
    pub source_len: u64,
    /// CRC-32 (the zip polynomial) of the file's bytes, which the archive
    /// check compares with the vendor's member (`pull::gdfl_archive`).
    pub source_crc32: u32,
    /// BLAKE3 of the file's bytes, unkeyed.
    pub source_digest: [u8; 32],
}

/// Decodes one capital-market file, without the archive check.
///
/// Crate-private since the round-5 review: design §3.3 verifies every file
/// before it is gridded, "never a fallback to the unverified tree", so
/// outside this crate the only way from a source's bytes to rows is
/// [`read_listed`] (or [`read_day`]), which checks the bytes against the
/// length and CRC-32 the vendor's archive states first (D-2800).
///
/// # Errors
///
/// [`CmRefusal::HeaderUnknown`], [`CmRefusal::TickerMismatch`],
/// [`CmRefusal::DateMismatch`], [`CmRefusal::MalformedRow`],
/// [`CmRefusal::PriceRefused`], [`CmRefusal::RowsOverCap`] or
/// [`CmRefusal::NoSessionRows`], each naming the first line that failed.
pub(crate) fn decode(bytes: &[u8], expect: &Expect<'_>) -> Result<CmFile, CmRefusal> {
    decode_capped(bytes, expect, crate::fetch::MAX_ROWS)
}

/// [`decode`] with the row bound as a parameter, so the bound's refusal is
/// testable without a million-row fixture.
fn decode_capped(bytes: &[u8], expect: &Expect<'_>, cap: usize) -> Result<CmFile, CmRefusal> {
    let (body, final_newline) = match bytes.strip_suffix(b"\n") {
        Some(body) => (body, true),
        None => (bytes, false),
    };
    let mut lines = body.split(|&b| b == b'\n').peekable();
    let header = match lines.next().map(strip_cr) {
        Some(h) if h == HEADER_OPEN_INTEREST.as_bytes() => HeaderVariant::OpenInterest,
        Some(h) if h == HEADER_OPEN_INTEREST_SPACED.as_bytes() => HeaderVariant::OpenInterestSpaced,
        _ => return Err(CmRefusal::HeaderUnknown),
    };
    let date = format!(
        "{:02}/{:02}/{:04}",
        expect.day.day(),
        expect.day.month(),
        expect.day.year()
    );
    let mut file = CmFile {
        header,
        final_newline,
        rows: Vec::new(),
        max_sod: 0,
        rows_pre: 0,
        rows_in_session: 0,
        rows_post: 0,
        ltq_zero_rows: 0,
        tail_truncated_post_session: false,
        source_len: u64::try_from(bytes.len()).unwrap_or(u64::MAX),
        source_digest: brutex_core::blake3::hash(bytes),
        source_crc32: crate::gdfl_archive::crc32(bytes),
    };
    let mut usable: u32 = 0;
    let mut index = 0_usize;
    while let Some(raw) = lines.next() {
        // Line 1 is the header, so the row at `index` 0 is line 2.
        let line = u32::try_from(index.saturating_add(2)).unwrap_or(u32::MAX);
        index = index.saturating_add(1);
        let last_without_newline = !final_newline && lines.peek().is_none();
        if last_without_newline {
            // A bare CR is the first half of a CRLF the vendor wrote, so a
            // last line ending in one is a file cut between the CR and its
            // LF, whether the line is a complete row or a short one: refused
            // by name, never a cut tail and never a clean final row (round-4
            // and round-5 reviews).
            if raw.ends_with(b"\r") {
                return Err(CmRefusal::MalformedRow { line });
            }
            // Before the row bound: a dropped cut tail is not a row (round-5
            // review).
            if post_session_tail(raw, expect, &date, file.rows.last()) {
                file.tail_truncated_post_session = true;
                break;
            }
        }
        if file.rows.len() >= cap {
            return Err(CmRefusal::RowsOverCap { cap });
        }
        let row = parse_row(strip_cr(raw), line, expect, &date)?;
        if row.sod < SESSION_OPEN_SOD {
            file.rows_pre = file.rows_pre.saturating_add(1);
        } else if row.sod < SESSION_CLOSE_SOD {
            file.rows_in_session = file.rows_in_session.saturating_add(1);
            if expect.kind == CmKind::Indices || row.ltq > 0 {
                usable = usable.saturating_add(1);
            }
        } else {
            file.rows_post = file.rows_post.saturating_add(1);
        }
        if row.ltq == 0 {
            file.ltq_zero_rows = file.ltq_zero_rows.saturating_add(1);
        }
        // The running maximum: a late row is below it and never raises it, so
        // at the end it is the stamp of the last in-order row.
        file.max_sod = file.max_sod.max(row.sod);
        file.rows.push(row);
    }
    if usable == 0 {
        return Err(CmRefusal::NoSessionRows);
    }
    // Design §3.3, revision 9 (C45): the largest stamp, never the last row in
    // file order, so a file ending on late rows stamped inside the session is
    // handed on whole and the fold places them. `usable > 0` means a row was
    // read, so `max_sod` is a real stamp here.
    if expect.kind == CmKind::Indices && file.max_sod < SESSION_CLOSE_SOD {
        return Err(CmRefusal::IndexEndsInSession {
            max_sod: file.max_sod,
        });
    }
    Ok(file)
}

/// Whether `bytes`, the file's last line with no newline after it, is a cut
/// row that can only be a post-session row (design §3.3, revision 4): fewer
/// than ten fields but at least four, so a comma closed the Time; this file's
/// own Ticker and Date; a real Time at or after 15:30:00; and a complete row
/// before it also stamped at or after 15:30:00. Since the round-3 review the
/// fields after the Time must also be a prefix of a row ([`is_row_prefix`]),
/// so a corrupted row is not recorded as a cut one. A last line ending in a
/// bare CR never reaches this test: [`decode`] refuses it first (round-4 and
/// round-5 reviews). Any other short row is `MalformedRow`, which `parse_row`
/// then says.
fn post_session_tail(
    bytes: &[u8],
    expect: &Expect<'_>,
    date: &str,
    before: Option<&CmRow>,
) -> bool {
    let Ok(text) = std::str::from_utf8(bytes) else {
        return false;
    };
    let fields: Vec<&str> = text.split(',').collect();
    let &[ticker, row_date, time, ..] = fields.as_slice() else {
        return false;
    };
    fields.len() >= 4
        && fields.len() < 10
        && ticker == expect.stem
        && row_date == date
        && second_of_day(time).is_some_and(|sod| sod >= SESSION_CLOSE_SOD)
        && before.is_some_and(|row| row.sod >= SESSION_CLOSE_SOD)
        && fields.get(3..).is_some_and(is_row_prefix)
}

/// Whether a field's text has one column's shape.
type Shape = fn(&str) -> bool;

/// The shape of each column after `Time`, in order (`LTP`, `BuyPrice`,
/// `BuyQty`, `SellPrice`, `SellQty`, `LTQ`, `OpenInterest`): whole, and as a
/// prefix cut short.
const AFTER_TIME: [(Shape, Shape); 7] = [
    (is_decimal, is_decimal_prefix),
    (is_decimal, is_decimal_prefix),
    (is_digits, is_digits_prefix),
    (is_decimal, is_decimal_prefix),
    (is_digits, is_digits_prefix),
    (is_digits, is_digits_prefix),
    (is_digits, is_digits_prefix),
];

/// Whether `after_time`, the fields of a cut row after its Time, is what
/// cutting a well-formed row leaves: every field but the last whole, the
/// last a prefix of its column (empty when the cut fell just after a comma).
fn is_row_prefix(after_time: &[&str]) -> bool {
    let last = after_time.len().saturating_sub(1);
    after_time
        .iter()
        .zip(AFTER_TIME)
        .enumerate()
        .all(|(at, (field, (whole, prefix)))| {
            if at == last {
                prefix(field)
            } else {
                whole(field)
            }
        })
}

/// A prefix of [`is_decimal`]'s shape: digits, then optionally a dot and
/// digits, any part possibly empty except the digits before a dot.
fn is_decimal_prefix(text: &str) -> bool {
    match text.split_once('.') {
        Some((whole, frac)) => is_digits(whole) && is_digits_prefix(frac),
        None => is_digits_prefix(text),
    }
}

/// Zero or more ASCII digits and nothing else.
fn is_digits_prefix(text: &str) -> bool {
    text.bytes().all(|b| b.is_ascii_digit())
}

/// A line without the one `\r` of its CRLF. Stricter than `pull::csv`'s
/// splitter, which trims every trailing `\r` and then whitespace: every
/// measured GDFL line ends in exactly one CRLF or, unterminated, in a digit
/// (charter, "Line endings"; D-0808), so a second CR or a trailing blank is left
/// in the field and refused by name, never trimmed (D-0808, round-5 review).
fn strip_cr(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\r").unwrap_or(line)
}

/// One row's ten fields, checked in column order.
fn parse_row(bytes: &[u8], line: u32, expect: &Expect<'_>, date: &str) -> Result<CmRow, CmRefusal> {
    let malformed = CmRefusal::MalformedRow { line };
    let text = std::str::from_utf8(bytes).map_err(|_| malformed.clone())?;
    let mut fields = [""; 10];
    let mut count = 0_usize;
    for field in text.split(',') {
        if let Some(slot) = fields.get_mut(count) {
            *slot = field;
        }
        count = count.saturating_add(1);
    }
    if count != fields.len() {
        return Err(malformed);
    }
    let [
        ticker,
        row_date,
        time,
        last_price,
        bid,
        bid_qty,
        ask,
        ask_qty,
        last_qty,
        open_interest,
    ] = fields;
    if ticker != expect.stem {
        return Err(CmRefusal::TickerMismatch { line });
    }
    if row_date != date {
        return Err(CmRefusal::DateMismatch { line });
    }
    let sod = second_of_day(time).ok_or_else(|| malformed.clone())?;
    let paisa = ltp_paisa(last_price).ok_or(CmRefusal::PriceRefused { line })?;
    // The quote and the open interest are checked for shape and then dropped:
    // the grid keeps no quote, and every ten-field NIFTY 50 and NIFTY BANK row
    // of the copy has all six of those fields 0 (charter, GDFL section).
    if !(is_decimal(bid)
        && is_decimal(ask)
        && is_digits(bid_qty)
        && is_digits(ask_qty)
        && is_digits(open_interest))
    {
        return Err(malformed);
    }
    let ltq = digits_u64(last_qty).ok_or(malformed)?;
    Ok(CmRow {
        line,
        sod,
        ltp: paisa,
        ltq,
    })
}

/// `HH:MM:SS` as a second of the day, or `None`.
fn second_of_day(time: &str) -> Option<u32> {
    let &[h1, h2, b':', m1, m2, b':', s1, s2] = time.as_bytes() else {
        return None;
    };
    let hours = u32::from(digit(h1)?) * 10 + u32::from(digit(h2)?);
    let minutes = u32::from(digit(m1)?) * 10 + u32::from(digit(m2)?);
    let seconds = u32::from(digit(s1)?) * 10 + u32::from(digit(s2)?);
    (hours <= 23 && minutes <= 59 && seconds <= 59)
        .then_some(hours * 3_600 + minutes * 60 + seconds)
}

/// The LTP in paisa, exactly, or `None` when the text is not a plain decimal
/// of at most two places, does not fit, or is below one tick.
///
/// Design §3.3 (Row checks): decoded with `csv::paisa`, the F&O reader's
/// conversion, so a third decimal REFUSES rather than snapping, for that
/// function's own first reason: the tick grid is two places (`CLAUDE.md` §7),
/// so a third digit is the vendor sending something this build does not
/// understand (round-4 review; D-0808). `is_decimal` is checked first because
/// `csv::paisa` also takes a sign and an empty fraction (`10000.`), neither of
/// which this column carries.
///
/// "One tick" is `costs::rate::TICK`, 5 paisa, as design §3.3 names it. That
/// constant is documented as the NSE and BSE index-OPTION premium tick; here
/// it is used only as a positivity floor, never as the tick of an index level
/// or of a cash equity, neither of which this reader asserts (round-3 review).
fn ltp_paisa(text: &str) -> Option<i64> {
    if !is_decimal(text) {
        return None;
    }
    crate::csv::paisa(text).filter(|&paisa| paisa >= costs::rate::TICK.raw())
}

/// Digits, optionally a dot and more digits: no sign, no space, no exponent.
fn is_decimal(text: &str) -> bool {
    match text.split_once('.') {
        Some((whole, frac)) => is_digits(whole) && is_digits(frac),
        None => is_digits(text),
    }
}

/// One or more ASCII digits and nothing else.
fn is_digits(text: &str) -> bool {
    !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
}

/// A plain unsigned integer, or `None` when it is not one or does not fit.
fn digits_u64(text: &str) -> Option<u64> {
    if is_digits(text) {
        text.parse().ok()
    } else {
        None
    }
}

/// Whether the calendar says `day` was a regular full session, the only kind
/// the one-second grid holds.
///
/// # Errors
///
/// [`CmRefusal::CalendarUnmeasured`] outside the measured span, and
/// [`CmRefusal::CalendarNotRegular`] for a closed day, a special session or a
/// day whose length was never measured.
pub fn session_gate(day: Day) -> Result<(), CmRefusal> {
    match kind_of(i64::from(day.days_from_epoch())) {
        DayKind::Open(session) if session == Session::full() => Ok(()),
        DayKind::Unmeasured => Err(CmRefusal::CalendarUnmeasured),
        kind => Err(CmRefusal::CalendarNotRegular { kind }),
    }
}

/// One instrument-day's file, located, read and decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DayFile {
    /// The file's name in its folder.
    pub name: String,
    /// The spelling of its extension.
    pub ext: ExtVariant,
    /// What it held.
    pub file: CmFile,
}

/// Reads `key`'s file for `day` from `source`: the calendar gate, the day's
/// listing (O(the day's entries), once), then [`read_listed`]. A caller
/// reading many instruments of one day lists the day once with
/// [`CmSource::day`] and calls [`read_listed`] for each.
///
/// `Ok(None)` when the source holds no such day, or the day no file for the
/// instrument: a day the vendor never shipped is the census's to list and
/// the fill's to refuse, never a guess here.
///
/// # Errors
///
/// [`CmRefusal::NotSwept`] for an instrument with no stem, the calendar
/// refusals of [`session_gate`] before anything is read, the source's own
/// refusals, and every refusal of [`read_listed`].
pub fn read_day<S: CmSource>(
    source: &S,
    key: &InstrumentKey,
    day: Day,
) -> Result<Option<DayFile>, CmRefusal> {
    let (kind, _) = stem_of(key).ok_or(CmRefusal::NotSwept)?;
    session_gate(day)?;
    match source.day(kind, day)? {
        Some(listing) => read_listed(source, &listing, key),
        None => Ok(None),
    }
}

/// Reads `key`'s file from a day `listing` of `source`, with the fetched
/// bytes checked against the length and CRC-32 the listing states BEFORE
/// they are decoded (D-0812, D-2800), so a file the source cut or changed is
/// refused `SourceLengthMismatch` or `SourceCrcMismatch` even where its rows
/// would not decode. The check is here, not in the source, so every source
/// is held to it.
///
/// # Errors
///
/// [`CmRefusal::NotSwept`] for an instrument with no stem,
/// [`CmRefusal::FolderUnknown`] when the listing is not of the instrument's
/// tree, the calendar refusals of [`session_gate`], the listing refusals of
/// [`DayListing::locate`], the source's own refusals from
/// [`CmSource::fetch`], [`CmRefusal::SourceLengthMismatch`] and
/// [`CmRefusal::SourceCrcMismatch`], which come before any decode refusal,
/// and every refusal of [`decode`].
pub fn read_listed<S: CmSource>(
    source: &S,
    listing: &DayListing<S::Locator>,
    key: &InstrumentKey,
) -> Result<Option<DayFile>, CmRefusal> {
    let (kind, stem) = stem_of(key).ok_or(CmRefusal::NotSwept)?;
    // The listing must be of the instrument's own tree: a stock listing
    // resolves no index key (design §3.3, revision 9).
    if listing.kind() != kind {
        return Err(CmRefusal::FolderUnknown);
    }
    session_gate(listing.day())?;
    let Some((file, ext)) = listing.locate(&stem)? else {
        return Ok(None);
    };
    // The whole file is held in memory before `RowsOverCap` can apply: no
    // byte bound precedes the fetch (`docs/06-limits.md`).
    let bytes = source.fetch(listing, file)?;
    verify(file, &bytes)?;
    let decoded = decode(
        &bytes,
        &Expect {
            kind,
            stem: &stem,
            day: listing.day(),
        },
    )?;
    Ok(Some(DayFile {
        name: file.name().to_owned(),
        ext,
        file: decoded,
    }))
}

/// Whether `bytes` are byte for byte the file `listed` names: the length
/// first, and the CRC-32 (one O(bytes) pass) only when the lengths agree.
/// A file cut or changed anywhere fails on one or the other, whether or not
/// it would decode.
///
/// # Errors
///
/// [`CmRefusal::SourceLengthMismatch`] or [`CmRefusal::SourceCrcMismatch`].
pub fn verify<L>(listed: &ListedFile<L>, bytes: &[u8]) -> Result<(), CmRefusal> {
    let len = u64::try_from(bytes.len()).unwrap_or(u64::MAX);
    if len != listed.len {
        return Err(CmRefusal::SourceLengthMismatch {
            file: len,
            member: listed.len,
        });
    }
    let crc = crate::gdfl_archive::crc32(bytes);
    if crc != listed.crc32 {
        return Err(CmRefusal::SourceCrcMismatch {
            file: crc,
            member: listed.crc32,
        });
    }
    Ok(())
}

#[cfg(test)]
#[expect(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::indexing_slicing,
    reason = "a test that cannot panic cannot fail"
)]
mod tests {
    use super::*;
    use std::path::Path;

    fn day(y: u16, m: u8, d: u8) -> Day {
        Day::new(y, m, d).unwrap()
    }

    // ── folder and file names ──────────────────────────────────────────────

    /// Folder names copied from the archive listing:
    /// `INDICES/2024/APR_2024/GFDLCM_INDICES_TICK_01042024` and
    /// `STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024`.
    #[test]
    fn both_observed_folder_names_parse_day_first() {
        assert_eq!(
            folder_day("GFDLCM_INDICES_TICK_01042024"),
            Ok((CmKind::Indices, day(2024, 4, 1))),
            "01042024 is 1 April, not 4 January"
        );
        assert_eq!(
            folder_day("GFDLCM_STOCK_TICK_03092018"),
            Ok((CmKind::Stocks, day(2018, 9, 3)))
        );
        // Every digit place carries its own weight: a two-digit day and
        // month, and a century digit that is not zero.
        assert_eq!(
            folder_day("GFDLCM_STOCK_TICK_31121999"),
            Ok((CmKind::Stocks, day(1999, 12, 31)))
        );
    }

    #[test]
    fn folder_unknown_refuses_every_other_name() {
        for name in [
            "GFDLNFO_TICK_01042024",
            "GFDLCM_INDICES_TICK_0104202",
            "GFDLCM_INDICES_TICK_010420244",
            "GFDLCM_INDICES_TICK_31022024",
            "GFDLCM_INDICES_TICK_0104202x",
            "GFDLCM_STOCK_TICK_+1042024",
            "GFDLCM_INDICES_TICK_",
            "",
        ] {
            assert_eq!(folder_day(name), Err(CmRefusal::FolderUnknown), "{name:?}");
        }
    }

    #[test]
    fn upper_and_lower_extension_split_and_are_recorded() {
        assert_eq!(
            split_name("NIFTY 50.NSE_IDX.csv"),
            Ok(("NIFTY 50.NSE_IDX", ExtVariant::Lower))
        );
        assert_eq!(
            split_name("NIFTY 50.NSE_IDX.CSV"),
            Ok(("NIFTY 50.NSE_IDX", ExtVariant::Upper))
        );
    }

    #[test]
    fn extension_unknown_refuses() {
        for name in [
            "NIFTY 50.NSE_IDX.Csv",
            "RELIANCE.NSE.txt",
            "RELIANCE",
            ".csv",
            "x.csv ",
        ] {
            assert_eq!(
                split_name(name),
                Err(CmRefusal::ExtensionUnknown),
                "{name:?}"
            );
        }
    }

    // ── the day listing ────────────────────────────────────────────────────

    /// A listing of the 2024-04-01 index day holding `names`.
    fn listing(names: &[&str]) -> DayListing<()> {
        listing_in(CmKind::Indices, names)
    }

    /// A listing of the 2024-04-01 day of tree `kind` holding `names`, each
    /// inside the day folder.
    fn listing_in(kind: CmKind, names: &[&str]) -> DayListing<()> {
        let mut listing = DayListing::new(kind, day(2024, 4, 1));
        let folder = listing.folder().to_owned();
        for name in names {
            listing.push(&format!("{folder}/{name}"), 0, 0, ());
        }
        listing
    }

    /// What a listing files for `stem`: the file's name and spelling.
    fn found<L>(
        listing: &DayListing<L>,
        stem: &str,
    ) -> Result<Option<(String, ExtVariant)>, CmRefusal> {
        Ok(listing
            .locate(stem)?
            .map(|(file, ext)| (file.name().to_owned(), ext)))
    }

    /// Design §3.3, revision 9: an equity key is a bare universe symbol,
    /// probed only after the stem has been found to end in exactly `.NSE`
    /// (case-sensitive, and not `.NSE_IDX`) and that suffix removed. A stem
    /// without the exact suffix is never probed: it is counted, not filed.
    #[test]
    fn stem_without_its_exact_suffix_never_resolves_to_an_equity() {
        for stem in [
            "RELIANCE",
            "RELIANCE.nse",
            "RELIANCE.NSE ",
            "RELIANCE.NSE_IDX",
            "RELIANCE.NSEX",
            "RELIANCE.BSE",
        ] {
            assert_eq!(
                resolve(CmKind::Stocks, stem),
                Resolution::ForeignSuffix,
                "{stem}"
            );
            for kind in [CmKind::Indices, CmKind::Stocks] {
                assert_eq!(stem_slot(kind, stem), None, "{stem} in {kind:?}");
            }
        }
        let folder = listing_in(
            CmKind::Stocks,
            &["RELIANCE.csv", "RELIANCE.nse.csv", "RELIANCE.NSE_IDX.csv"],
        );
        assert_eq!(folder.unresolved(), 3, "counted, never filed");
        assert_eq!(found(&folder, "RELIANCE.NSE"), Ok(None));
        let folder = listing_in(CmKind::Stocks, &["RELIANCE.NSE.csv"]);
        assert_eq!(
            found(&folder, "RELIANCE.NSE"),
            Ok(Some(("RELIANCE.NSE.csv".to_owned(), ExtVariant::Lower))),
            "the exact suffix resolves"
        );
    }

    /// Design §3.3, revision 9: an index key is probed only for a file of an
    /// index folder, an equity key only for a file of a stock folder, so a
    /// file filed in the other tree resolves to nothing, whatever its stem.
    #[test]
    fn keys_resolve_only_in_folders_of_their_kind() {
        assert_eq!(stem_slot(CmKind::Indices, "NIFTY 50.NSE_IDX"), Some(0));
        assert_eq!(stem_slot(CmKind::Stocks, "NIFTY 50.NSE_IDX"), None);
        assert_eq!(stem_slot(CmKind::Indices, "NIFTY BANK.NSE_IDX"), Some(1));
        assert_eq!(stem_slot(CmKind::Stocks, "NIFTY BANK.NSE_IDX"), None);
        let reliance = stem_slot(CmKind::Stocks, "RELIANCE.NSE");
        assert!(reliance.is_some_and(|slot| slot >= 2));
        assert_eq!(stem_slot(CmKind::Indices, "RELIANCE.NSE"), None);
        let stocks = listing_in(
            CmKind::Stocks,
            &["NIFTY 50.NSE_IDX.csv", "RELIANCE.NSE.csv"],
        );
        assert_eq!(stocks.kind(), CmKind::Stocks);
        assert_eq!(stocks.unresolved(), 1, "the index file in a stock folder");
        assert_eq!(found(&stocks, "NIFTY 50.NSE_IDX"), Ok(None));
        assert_eq!(
            found(&stocks, "RELIANCE.NSE"),
            Ok(Some(("RELIANCE.NSE.csv".to_owned(), ExtVariant::Lower)))
        );
        let indices = listing(&["NIFTY 50.NSE_IDX.csv", "RELIANCE.NSE.csv"]);
        assert_eq!(indices.kind(), CmKind::Indices);
        assert_eq!(indices.unresolved(), 1, "the stock file in an index folder");
        assert_eq!(found(&indices, "RELIANCE.NSE"), Ok(None));
        assert_eq!(
            found(&indices, "NIFTY 50.NSE_IDX"),
            Ok(Some(("NIFTY 50.NSE_IDX.csv".to_owned(), ExtVariant::Lower)))
        );
    }

    #[test]
    fn a_folder_locates_either_spelling_by_exact_stem() {
        let folder = listing(&[
            "NIFTY 50.NSE_IDX.csv",
            "NIFTY BANK.NSE_IDX.CSV",
            "M&M.NSE.csv",
        ]);
        assert_eq!(folder.entries().len(), 3);
        assert!(!folder.entries().is_empty());
        assert!(listing(&[]).entries().is_empty());
        assert!(SlotTable::<()>::new(CmKind::Stocks).is_empty());
        let mut table = SlotTable::new(CmKind::Indices);
        table.file("NIFTY 50.NSE_IDX.csv", ());
        table.skip();
        assert_eq!(
            (table.len(), table.is_empty(), table.unresolved()),
            (2, false, 1)
        );
        assert_eq!(
            found(&folder, "NIFTY 50.NSE_IDX"),
            Ok(Some(("NIFTY 50.NSE_IDX.csv".to_owned(), ExtVariant::Lower)))
        );
        assert_eq!(
            found(&folder, "NIFTY BANK.NSE_IDX"),
            Ok(Some((
                "NIFTY BANK.NSE_IDX.CSV".to_owned(),
                ExtVariant::Upper
            )))
        );
        assert_eq!(
            found(&folder, "SBIN.NSE"),
            Ok(None),
            "absent is not a refusal"
        );
        assert_eq!(
            found(&folder, "m&m.NSE"),
            Ok(None),
            "a stem differing in case is a different ticker, never this one"
        );
    }

    /// Synthetic listing: the source volume is case-insensitive, so twins can
    /// only be built by hand (design §3.3).
    #[test]
    fn case_twins_refuse() {
        let folder = listing(&["NIFTY 50.NSE_IDX.csv", "NIFTY 50.NSE_IDX.CSV"]);
        assert_eq!(
            found(&folder, "NIFTY 50.NSE_IDX"),
            Err(CmRefusal::AmbiguousCaseTwins)
        );
        let folder = listing(&[
            "NIFTY 50.NSE_IDX.csv",
            "NIFTY 50.NSE_IDX.CSV",
            "NIFTY 50.NSE_IDX.csv",
        ]);
        assert_eq!(
            found(&folder, "NIFTY 50.NSE_IDX"),
            Err(CmRefusal::AmbiguousCaseTwins),
            "a third name spelled csv keeps them twins"
        );
        // A second name whose extension is neither `csv` nor `CSV` is not a
        // case twin: the refusal names the odd extension, in either order and
        // whatever came before it.
        for names in [
            ["NIFTY 50.NSE_IDX.csv", "NIFTY 50.NSE_IDX.Txt"].as_slice(),
            &["NIFTY 50.NSE_IDX.Txt", "NIFTY 50.NSE_IDX.csv"],
            &["NIFTY 50.NSE_IDX.CSV", "NIFTY 50.NSE_IDX.Csv"],
            &[
                "NIFTY 50.NSE_IDX.csv",
                "NIFTY 50.NSE_IDX.CSV",
                "NIFTY 50.NSE_IDX.Bak",
            ],
            &[
                "NIFTY 50.NSE_IDX.Bak",
                "NIFTY 50.NSE_IDX.csv",
                "NIFTY 50.NSE_IDX.CSV",
            ],
        ] {
            assert_eq!(
                found(&listing(names), "NIFTY 50.NSE_IDX"),
                Err(CmRefusal::ExtensionUnknown),
                "{names:?}"
            );
        }
        let stocks = listing_in(CmKind::Stocks, &["RELIANCE.NSE.csv", "RELIANCE.NSE.Bak"]);
        assert_eq!(
            found(&stocks, "RELIANCE.NSE"),
            Err(CmRefusal::ExtensionUnknown)
        );
        // Revision 8: a stem is filed by its exact bytes, so a name whose stem
        // differs in case is not this ticker's file at all. It is counted for
        // the census and the exact name is found.
        let folder = listing(&["NIFTY 50.NSE_IDX.csv", "nifty 50.nse_idx.csv"]);
        assert_eq!(folder.unresolved(), 1);
        assert_eq!(
            found(&folder, "NIFTY 50.NSE_IDX"),
            Ok(Some(("NIFTY 50.NSE_IDX.csv".to_owned(), ExtVariant::Lower)))
        );
    }

    #[test]
    fn a_third_extension_spelling_of_the_wanted_stem_refuses() {
        let folder = listing(&["NIFTY 50.NSE_IDX.Csv"]);
        assert_eq!(
            found(&folder, "NIFTY 50.NSE_IDX"),
            Err(CmRefusal::ExtensionUnknown)
        );
    }

    // ── the ticker map ─────────────────────────────────────────────────────

    #[test]
    fn nifty_50_with_space_maps_to_nifty_and_nifty_bank_to_banknifty() {
        assert_eq!(
            resolve(CmKind::Indices, "NIFTY 50.NSE_IDX"),
            Resolution::Swept(InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap())
        );
        assert_eq!(
            resolve(CmKind::Indices, "NIFTY BANK.NSE_IDX"),
            Resolution::Swept(InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap())
        );
        assert_eq!(
            resolve(CmKind::Indices, "NIFTY50.NSE_IDX"),
            Resolution::NotSwept
        );
        assert_eq!(
            resolve(CmKind::Indices, "nifty 50.NSE_IDX"),
            Resolution::NotSwept
        );
        assert_eq!(
            resolve(CmKind::Indices, "NIFTY 100.NSE_IDX"),
            Resolution::NotSwept
        );
    }

    #[test]
    fn india_vix_is_never_gridded() {
        assert_eq!(
            resolve(CmKind::Indices, "INDIA VIX.NSE_IDX"),
            Resolution::ReferenceOnly
        );
    }

    /// Names from `STOCKS/2024/APR_2024/GFDLCM_STOCK_TICK_01042024`'s listing.
    #[test]
    fn f_and_o_shares_map_with_their_own_spelling() {
        for sym in ["RELIANCE", "M&M", "BAJAJ-AUTO", "SBIN"] {
            assert_eq!(
                resolve(CmKind::Stocks, &format!("{sym}.NSE")),
                Resolution::Swept(InstrumentKey::cash(Exchange::Nse, sym).unwrap()),
                "{sym}"
            );
        }
        // Exact stem bytes: `Symbol::new` uppercases, so the stem itself must
        // already be the symbol's own spelling or it is no share's file.
        for stem in ["m&m.NSE", "Reliance.NSE", "sbin.NSE", "bajaj-auto.NSE"] {
            assert_eq!(
                resolve(CmKind::Stocks, stem),
                Resolution::NotSwept,
                "{stem}"
            );
            assert_eq!(stem_slot(CmKind::Stocks, stem), None, "{stem}");
        }
    }

    /// `M&MFIN.N1.NSE` and `M&MFIN.N2.NSE` sit beside `M&MFIN.NSE` in the
    /// 2024-04-01 listing; `1018GS2026.GS.NSE` and `182D010824.TB.NSE` too.
    #[test]
    fn series_files_never_resolve_to_eq() {
        for stem in [
            "M&MFIN.N1.NSE",
            "M&MFIN.N2.NSE",
            "RELIANCE.BE.NSE",
            "1018GS2026.GS.NSE",
        ] {
            assert_eq!(
                resolve(CmKind::Stocks, stem),
                Resolution::SeriesVariant,
                "{stem}"
            );
        }
    }

    /// `resolve` refuses a stock name longer than any symbol by its length
    /// BEFORE anything reads it (round-4 review; the guard
    /// `core::universe::FnoIndex::position` carries for the same reason),
    /// the guard the module's `# Cost` section rests on. The proof is the
    /// answer: a series file one byte past
    /// the capacity is `NotSwept`, not `SeriesVariant`, which it could only
    /// be called if its dot had been searched for. At the capacity it is
    /// still scanned and still a series file.
    #[test]
    fn a_stock_name_longer_than_any_symbol_is_refused_before_it_is_read() {
        use brutex_core::symbol::SYMBOL_CAPACITY;
        let at_cap = format!("{}.BE", "A".repeat(SYMBOL_CAPACITY - 3));
        assert_eq!(at_cap.len(), SYMBOL_CAPACITY);
        assert_eq!(
            resolve(CmKind::Stocks, &format!("{at_cap}.NSE")),
            Resolution::SeriesVariant
        );
        let past_cap = format!("{}.BE", "A".repeat(SYMBOL_CAPACITY - 2));
        assert_eq!(past_cap.len(), SYMBOL_CAPACITY + 1);
        for stem in [
            format!("{past_cap}.NSE"),
            format!("{}.NSE", "A".repeat(SYMBOL_CAPACITY + 1)),
            format!("{}.NSE", "A.".repeat(1 << 20)),
        ] {
            assert_eq!(resolve(CmKind::Stocks, &stem), Resolution::NotSwept);
            assert_eq!(stem_slot(CmKind::Stocks, &stem), None);
        }
    }

    #[test]
    fn non_f_and_o_and_index_names_in_the_stock_tree_are_not_swept() {
        for stem in [
            "NIFTYBEES.NSE",
            "NIFTY.NSE",
            "BANKNIFTY.NSE",
            "NIFTY1.NSE",
            ".NSE",
        ] {
            assert_eq!(
                resolve(CmKind::Stocks, stem),
                Resolution::NotSwept,
                "{stem}"
            );
        }
    }

    #[test]
    fn a_stem_from_the_other_tree_is_foreign() {
        assert_eq!(
            resolve(CmKind::Stocks, "NIFTY 50.NSE_IDX"),
            Resolution::ForeignSuffix
        );
        assert_eq!(
            resolve(CmKind::Indices, "RELIANCE.NSE"),
            Resolution::ForeignSuffix
        );
        assert_eq!(
            resolve(CmKind::Stocks, "RELIANCE"),
            Resolution::ForeignSuffix
        );
    }

    /// Every swept instrument has exactly one stem and that stem resolves back
    /// to it: 2 indices plus the 208 F&O shares.
    #[test]
    fn every_swept_instrument_round_trips_through_its_stem() {
        let mut keys = vec![
            InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap(),
            InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap(),
        ];
        for sym in brutex_core::universe::FNO_UNDERLYINGS {
            let key = InstrumentKey::cash(Exchange::Nse, sym).unwrap();
            if key.is_sweepable() {
                keys.push(key);
            }
        }
        assert_eq!(keys.len(), 210);
        for key in keys {
            let (kind, stem) = stem_of(&key).expect("a swept key has a stem");
            assert_eq!(resolve(kind, &stem), Resolution::Swept(key), "{stem}");
            assert!(key.is_sweepable());
        }
    }

    /// Design §3.3 (revision 8, C43): the equity keys are the universe's
    /// shares, never a hand-kept list. Every `FNO_UNDERLYINGS` symbol that is
    /// not one of the five `FNO_INDEX_UNDERLYINGS` resolves as `<SYM>.NSE` to
    /// its own cash key and its own slot, the five resolve to nothing, and
    /// the two index keys take the two slots below the shares.
    #[test]
    fn ticker_map_equity_keys_are_the_universe_shares() {
        use brutex_core::universe::{FNO_INDEX_UNDERLYINGS, FNO_UNDERLYINGS};
        let mut taken = [false; TICKER_SLOTS];
        let mut shares = 0;
        for sym in FNO_UNDERLYINGS {
            let stem = format!("{sym}.NSE");
            if FNO_INDEX_UNDERLYINGS.contains(&sym) {
                assert_eq!(resolve(CmKind::Stocks, &stem), Resolution::NotSwept);
                assert_eq!(stem_slot(CmKind::Stocks, &stem), None, "{stem}");
                continue;
            }
            let key = InstrumentKey::cash(Exchange::Nse, sym).unwrap();
            assert_eq!(resolve(CmKind::Stocks, &stem), Resolution::Swept(key));
            let slot = stem_slot(CmKind::Stocks, &stem).expect("a share has a slot");
            assert_eq!(ticker_slot(&key), Some(slot), "{stem}");
            assert!(!taken[slot], "{stem} shares slot {slot}");
            taken[slot] = true;
            shares += 1;
        }
        assert_eq!(shares, FNO_UNDERLYINGS.len() - FNO_INDEX_UNDERLYINGS.len());
        assert_eq!(shares, 208);
        assert_eq!(stem_slot(CmKind::Indices, "NIFTY 50.NSE_IDX"), Some(0));
        assert_eq!(stem_slot(CmKind::Indices, "NIFTY BANK.NSE_IDX"), Some(1));
        assert!(!taken[0], "a share took the NIFTY slot");
        assert!(!taken[1], "a share took the BANKNIFTY slot");
        for off in [
            "INDIA VIX.NSE_IDX",
            "NIFTY IT.NSE_IDX",
            "IDEA.BE.NSE",
            "NIFTYBEES.NSE",
            "RELIANCE.NSE_IDX",
            "NIFTY 50.NSE",
            "RELIANCE",
        ] {
            for kind in [CmKind::Indices, CmKind::Stocks] {
                assert_eq!(stem_slot(kind, off), None, "{off} in {kind:?}");
            }
        }
        for key in [
            InstrumentKey::index(Exchange::Nse, "FINNIFTY").unwrap(),
            InstrumentKey::cash(Exchange::Nse, "NIFTY").unwrap(),
            InstrumentKey::cash(Exchange::Bse, "RELIANCE").unwrap(),
        ] {
            assert_eq!(ticker_slot(&key), None, "{key}");
        }
        assert_eq!(TICKER_SLOTS, INDEX_SLOTS + FNO_UNDERLYINGS.len());
        assert_eq!(INDEX_SLOTS, 2);
    }

    /// The index slots and stems are read off `InstrumentKey::SWEPT`, the one
    /// list of swept indices, and match its symbols exactly: each has its own
    /// slot below the shares and a stem that resolves back to it, and any
    /// other index symbol has neither, so widening `SWEPT` without naming
    /// the vendor's file here fails this test instead of borrowing a slot
    /// (round-3 review).
    #[test]
    fn every_swept_index_has_its_own_slot_and_stem_and_no_other_index_has_one() {
        assert_eq!(INDEX_SLOTS, InstrumentKey::SWEPT.len());
        let mut seen = [false; INDEX_SLOTS];
        for (exchange, symbol) in InstrumentKey::SWEPT {
            let key = InstrumentKey::index(exchange, symbol).unwrap();
            let slot = ticker_slot(&key).unwrap();
            assert!(!seen[slot], "{symbol} shares slot {slot}");
            seen[slot] = true;
            let (kind, stem) = stem_of(&key).unwrap();
            assert_eq!(kind, CmKind::Indices);
            assert_eq!(resolve(kind, &stem), Resolution::Swept(key), "{stem}");
            assert_eq!(stem_slot(kind, &stem), Some(slot), "{stem}");
        }
        for other in [
            "FINNIFTY",
            "MIDCPNIFTY",
            "NIFTYNXT50",
            "SENSEX",
            "nifty",
            "",
        ] {
            assert_eq!(index_ticker(other), None, "{other}");
        }
    }

    /// Design §3.3 (revision 8): "does instrument X have a file today" is one
    /// index into an array the size of the ticker map. Neither this module
    /// nor the archive check reaches for a hash map, a set or a sort outside
    /// its tests, and the only search in either is the archive's bounded
    /// backward scan for its end record (at most 65,557 bytes, once per zip),
    /// which is not a member lookup: every other `.find(` is refused, and that
    /// one is required to be seen exactly once (round-3 review: CM-13 said
    /// "no search" while that scan existed and the test banned only
    /// `.iter().find(`).
    #[test]
    fn day_folder_lookup_is_one_index_into_a_ticker_slot_array() {
        let banned = [
            ["Hash", "Map"].concat(),
            ["Hash", "Set"].concat(),
            ["BTree", "Map"].concat(),
            [".contains", "(&"].concat(),
            [".binary", "_search"].concat(),
            [".sort", "("].concat(),
            [".sort", "_"].concat(),
            [".iter().", "position("].concat(),
            [".find", "("].concat(),
        ];
        let end_record_scan = [".find", "(|&at| {"].concat();
        let mut scans = 0;
        let here = std::path::Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src"));
        for file in ["gdfl_cm.rs", "gdfl_archive.rs"] {
            let text = std::fs::read_to_string(here.join(file)).unwrap();
            let live = text.split("#[cfg(test)]").next().unwrap();
            for (at, line) in live.lines().enumerate() {
                if line.trim_start().starts_with("//") {
                    continue;
                }
                if file == "gdfl_archive.rs" && line.trim_start() == end_record_scan {
                    scans += 1;
                    continue;
                }
                for word in &banned {
                    assert!(!line.contains(word.as_str()), "{file}:{}: {line}", at + 1);
                }
            }
        }
        assert_eq!(scans, 1, "the end-record scan is the one search");
        let folder = listing(&[
            "NIFTY 50.NSE_IDX.csv",
            "INDIA VIX.NSE_IDX.csv",
            "IDEA.BE.NSE.csv",
            "README",
        ]);
        assert_eq!(folder.entries().len(), 4);
        assert_eq!(folder.unresolved(), 3, "counted for the census, not held");
        assert_eq!(
            found(&folder, "NIFTY 50.NSE_IDX"),
            Ok(Some(("NIFTY 50.NSE_IDX.csv".to_owned(), ExtVariant::Lower)))
        );
        assert_eq!(found(&folder, "INDIA VIX.NSE_IDX"), Ok(None));
        assert_eq!(found(&folder, "IDEA.BE.NSE"), Ok(None));
    }

    #[test]
    fn nothing_off_the_surface_has_a_stem() {
        for key in [
            InstrumentKey::index(Exchange::Nse, "FINNIFTY").unwrap(),
            InstrumentKey::index(Exchange::Nse, "INDIAVIX").unwrap(),
            InstrumentKey::index(Exchange::Bse, "NIFTY").unwrap(),
            InstrumentKey::cash(Exchange::Nse, "NIFTY").unwrap(),
            InstrumentKey::cash(Exchange::Nse, "NIFTYBEES").unwrap(),
            InstrumentKey::cash(Exchange::Bse, "RELIANCE").unwrap(),
        ] {
            assert_eq!(stem_of(&key), None, "{key}");
        }
    }

    #[test]
    fn the_two_descriptors_are_the_observed_prefixes_and_suffixes() {
        assert_eq!(CmKind::Indices.descriptor(), INDICES);
        assert_eq!(CmKind::Stocks.descriptor(), STOCKS);
        assert_eq!(
            HEADER_OPEN_INTEREST,
            "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,OpenInterest"
        );
        // `head -1` of `INDICES/2018/SEP_2018/GFDLCM_INDICES_TICK_03092018/
        // NIFTY 50.NSE_IDX.CSV`, its CRLF removed: a header, not a row.
        assert_eq!(
            HEADER_OPEN_INTEREST_SPACED,
            "Ticker,Date,Time,LTP,BuyPrice,BuyQty,SellPrice,SellQty,LTQ,Open Interest"
        );
    }

    // ── rows ───────────────────────────────────────────────────────────────
    //
    // Every fixture below is SYNTHETIC (design §12, revision 5): the field
    // layout and the two header spellings are format facts, and the shapes
    // (several rows to a second, a replay over a hole, a final row without a
    // newline, LTQ = 0 stock rows) are measured ones, but every price, quantity
    // and quote is invented, and so is every stamp other than the session's
    // own edges (09:15:00, 09:15:01, 15:29:59, 15:30:00), which are the
    // boundaries under test. No fixture is a real row shifted by a constant:
    // the round-1 review found the earlier ones were, and they were replaced. No GDFL row is committed: this repository is
    // public and the licence for the files is an open question (D-0802
    // consequence 10; D-0802 is an unlanded draft, not yet in the ledger). `fixtures_are_built_not_pasted` holds that line.

    /// One synthetic row: ten invented fields joined as the vendor joins them.
    fn synthetic_row(fields: [&str; 10]) -> String {
        fields.join(",")
    }

    /// An index row: the quotes, the LTQ and the open interest are all zero,
    /// which is the measured shape of every ten-field NIFTY 50 and NIFTY BANK
    /// row (charter, GDFL section).
    fn index_row(stem: &str, date: &str, time: &str, ltp: &str) -> String {
        synthetic_row([stem, date, time, ltp, "0", "0", "0", "0", "0", "0"])
    }

    /// A NIFTY 50 row on 2024-04-01, a regular `Open` day.
    fn n50(time: &str, ltp: &str) -> String {
        index_row("NIFTY 50.NSE_IDX", "01/04/2024", time, ltp)
    }

    /// A RELIANCE row on 2024-04-01 with an invented quote and a traded
    /// quantity. Quantities are numbers formatted at run time.
    fn rel(time: &str, ltp: &str, ltq: u64) -> String {
        let (bid_qty, ask_qty, ltq) = (40.to_string(), 70.to_string(), ltq.to_string());
        synthetic_row([
            "RELIANCE.NSE",
            "01/04/2024",
            time,
            ltp,
            "731.3",
            &bid_qty,
            "731.55",
            &ask_qty,
            &ltq,
            "0",
        ])
    }

    /// A header and its rows, CRLF as the files are, with or without the
    /// final newline.
    fn synthetic_file(header: &str, rows: &[String], final_newline: bool) -> Vec<u8> {
        let mut text = String::from(header);
        for row in rows {
            text.push_str("\r\n");
            text.push_str(row);
        }
        if final_newline {
            text.push_str("\r\n");
        }
        text.into_bytes()
    }

    /// The modern header, then `rows`, then a final newline.
    fn modern(rows: &[String]) -> Vec<u8> {
        synthetic_file(HEADER_OPEN_INTEREST, rows, true)
    }

    /// An index day of the measured shape: two pre-open rows, the 09:15:00
    /// open, three rows in one second, the last session second, the first
    /// second after it, and two post-close rows.
    fn index_day() -> Vec<u8> {
        modern(&[
            n50("09:08:41", "10250.0"),
            n50("09:08:41", "10250.0"),
            n50("09:15:00", "10251.3"),
            n50("09:15:01", "10253.45"),
            n50("09:15:01", "10249.1"),
            n50("09:15:01", "10260.75"),
            n50("15:29:59", "10301.2"),
            n50("15:30:00", "10298.65"),
            n50("16:12:05", "10300.0"),
            n50("16:12:06", "10300.0"),
        ])
    }

    /// A September-2018-shaped file: the spaced header, and a last row with no
    /// newline after it.
    fn spaced_day_without_final_newline() -> Vec<u8> {
        let row = |time, ltp| index_row("NIFTY 50.NSE_IDX", "03/09/2018", time, ltp);
        synthetic_file(
            HEADER_OPEN_INTEREST_SPACED,
            &[
                row("09:08:03", "7400.6"),
                row("09:08:04", "7400.6"),
                row("09:15:00", "7400.6"),
                row("09:15:00", "7391.25"),
                row("09:15:01", "7392.4"),
                row("16:31:17", "7377.15"),
                row("16:31:18", "7377.15"),
                row("16:31:19", "7377.15"),
            ],
            false,
        )
    }

    /// A stock day: traded rows, LTQ = 0 rows between them (one below the
    /// bid), and rows after the close.
    fn stock_day() -> Vec<u8> {
        modern(&[
            rel("09:16:23", "731.4", 500),
            rel("09:16:23", "731.4", 0),
            rel("09:16:23", "725.0", 0),
            rel("09:16:40", "731.5", 120),
            rel("09:16:40", "732.0", 0),
            rel("15:30:00", "729.85", 0),
            synthetic_row([
                "RELIANCE.NSE",
                "01/04/2024",
                "15:33:10",
                "728.6",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
            ]),
        ])
    }

    fn nifty(day: Day) -> Expect<'static> {
        Expect {
            kind: CmKind::Indices,
            stem: "NIFTY 50.NSE_IDX",
            day,
        }
    }

    fn reliance() -> Expect<'static> {
        Expect {
            kind: CmKind::Stocks,
            stem: "RELIANCE.NSE",
            day: day(2024, 4, 1),
        }
    }

    /// A good session row, then `row` on line 3, then a post-close row so an
    /// index file reaches past the session: the shape every refusal test below
    /// edits one field of.
    fn one_bad_row(row: &str) -> Vec<u8> {
        modern(&[
            n50("09:15:00", "10000"),
            row.to_owned(),
            n50("16:00:00", "10001.5"),
        ])
    }

    /// `one_bad_row` with an index row whose LTP is `ltp`.
    fn ltp_row(ltp: &str) -> Vec<u8> {
        one_bad_row(&n50("09:15:01", ltp))
    }

    fn sod(h: u32, m: u32, s: u32) -> u32 {
        h * 3_600 + m * 60 + s
    }

    #[test]
    fn three_rows_at_0915_01_keep_file_order_and_paisa() {
        let bytes = index_day();
        let file = decode(&bytes, &nifty(day(2024, 4, 1))).unwrap();
        assert_eq!(file.header, HeaderVariant::OpenInterest);
        assert!(file.final_newline);
        let seen: Vec<(u32, u32, i64, u64)> = file
            .rows
            .iter()
            .map(|r| (r.line, r.sod, r.ltp, r.ltq))
            .collect();
        assert_eq!(
            seen,
            vec![
                (2, sod(9, 8, 41), 1_025_000, 0),
                (3, sod(9, 8, 41), 1_025_000, 0),
                (4, sod(9, 15, 0), 1_025_130, 0),
                (5, sod(9, 15, 1), 1_025_345, 0),
                (6, sod(9, 15, 1), 1_024_910, 0),
                (7, sod(9, 15, 1), 1_026_075, 0),
                (8, sod(15, 29, 59), 1_030_120, 0),
                (9, sod(15, 30, 0), 1_029_865, 0),
                (10, sod(16, 12, 5), 1_030_000, 0),
                (11, sod(16, 12, 6), 1_030_000, 0),
            ],
            "every row, file order, none dropped or moved"
        );
        assert_eq!(sod(9, 15, 1), 33_301);
        assert_eq!(
            (file.rows_pre, file.rows_in_session, file.rows_post),
            (2, 5, 3),
            "09:15:00 is in the session and 15:30:00 is not"
        );
        assert_eq!(file.ltq_zero_rows, 10, "an index row's LTQ is 0");
        assert!(!file.tail_truncated_post_session);
        assert_eq!(file.source_len, bytes.len() as u64);
        assert_eq!(file.source_digest, brutex_core::blake3::hash(&bytes));
        assert_eq!(file.source_crc32, crate::gdfl_archive::crc32(&bytes));
        assert_ne!(file.source_crc32, 0);
    }

    #[test]
    fn crlf_never_reaches_a_field() {
        let crlf = index_day();
        let lf = String::from_utf8(crlf.clone())
            .unwrap()
            .replace("\r\n", "\n");
        let with = decode(&crlf, &nifty(day(2024, 4, 1))).unwrap();
        let without = decode(lf.as_bytes(), &nifty(day(2024, 4, 1))).unwrap();
        assert_eq!(with.rows, without.rows);
        assert_ne!(
            with.source_digest, without.source_digest,
            "the bytes differ"
        );
        assert_ne!(with.source_crc32, without.source_crc32);
    }

    #[test]
    fn complete_last_row_without_newline_is_accepted() {
        let file = decode(&spaced_day_without_final_newline(), &nifty(day(2018, 9, 3))).unwrap();
        assert!(!file.final_newline);
        assert!(!file.tail_truncated_post_session);
        assert_eq!(file.header, HeaderVariant::OpenInterestSpaced);
        let last = file.rows.last().unwrap();
        assert_eq!(
            (last.line, last.sod, last.ltp),
            (9, sod(16, 31, 19), 737_715)
        );
        assert_eq!(file.rows[0].ltp, 740_060, "7400.6 is sixty paisa");
        assert_eq!(
            (file.rows_pre, file.rows_in_session, file.rows_post),
            (2, 3, 3)
        );
    }

    /// The narrow case of design §3.3 (revision 4): the last row, with no
    /// newline after it, cut short after a complete Time stamped after the
    /// session, behind a complete row also after the session. It can only be a
    /// post-session row, so it is dropped and the drop is recorded.
    #[test]
    fn a_cut_post_session_last_row_is_dropped_and_recorded() {
        for cut_at in 4..10 {
            let whole = n50("16:00:01", "10002.5");
            let cut = whole.split(',').take(cut_at).collect::<Vec<_>>().join(",");
            let bytes = synthetic_file(
                HEADER_OPEN_INTEREST,
                &[n50("09:15:00", "10000"), n50("16:00:00", "10001.5"), cut],
                false,
            );
            let file = decode(&bytes, &nifty(day(2024, 4, 1))).unwrap();
            assert!(file.tail_truncated_post_session, "{cut_at} fields");
            assert_eq!(
                file.rows.len(),
                2,
                "{cut_at} fields: the cut row is not a row"
            );
            assert_eq!(file.rows_post, 1);
            assert_eq!(
                file.source_len,
                bytes.len() as u64,
                "the bytes are all hashed"
            );
        }
        // The same cut with a partial LTP, `1000` of `10002.5`.
        let bytes = synthetic_file(
            HEADER_OPEN_INTEREST,
            &[
                n50("09:15:00", "10000"),
                n50("15:30:00", "10001.5"),
                n50("16:00:01", "1000.2")
                    .split(',')
                    .take(4)
                    .collect::<Vec<_>>()
                    .join(","),
            ],
            false,
        );
        assert!(
            decode(&bytes, &nifty(day(2024, 4, 1)))
                .unwrap()
                .tail_truncated_post_session,
            "15:30:00 itself is after the session"
        );
        let bytes = synthetic_file(
            HEADER_OPEN_INTEREST,
            &[
                n50("09:15:00", "10000"),
                n50("15:30:00", "10001.5"),
                index_row("NIFTY 50.NSE_IDX", "01/04/2024", "15:30:00", "1"),
            ],
            false,
        );
        let cut = bytes.len() - ",0,0,0,0,0,0".len();
        assert!(
            decode(&bytes[..cut], &nifty(day(2024, 4, 1)))
                .unwrap()
                .tail_truncated_post_session,
            "a cut row stamped 15:30:00 is after the session too"
        );
    }

    /// Every other short row still refuses: a cut inside the session, a cut
    /// after a row still inside the session, a cut before the Time is complete,
    /// a cut row followed by a newline, a cut row that is not the file's own,
    /// and a cut row with nothing complete before it.
    #[test]
    fn short_last_row_refuses() {
        let cut =
            |row: String, keep: usize| row.split(',').take(keep).collect::<Vec<_>>().join(",");
        let cases: Vec<(Vec<String>, bool, u32)> = vec![
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("16:00:00", "1"),
                    cut(n50("15:29:59", "1"), 9),
                ],
                false,
                4,
            ),
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("15:29:59", "1"),
                    cut(n50("16:00:01", "1"), 9),
                ],
                false,
                4,
            ),
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("16:00:00", "1"),
                    cut(n50("16:00:01", "1"), 3),
                ],
                false,
                4,
            ),
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("16:00:00", "1"),
                    cut(n50("16:00:01", "1"), 2),
                ],
                false,
                4,
            ),
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("16:00:00", "1"),
                    cut(n50("16:00:01", "1"), 9),
                ],
                true,
                4,
            ),
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("16:00:00", "1"),
                    cut(
                        index_row("NIFTY BANK.NSE_IDX", "01/04/2024", "16:00:01", "1"),
                        9,
                    ),
                ],
                false,
                4,
            ),
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("16:00:00", "1"),
                    cut(
                        index_row("NIFTY 50.NSE_IDX", "02/04/2024", "16:00:01", "1"),
                        9,
                    ),
                ],
                false,
                4,
            ),
            (
                vec![
                    n50("09:15:00", "1"),
                    n50("16:00:00", "1"),
                    cut(n50("16:0x:01", "1"), 9),
                ],
                false,
                4,
            ),
            (vec![cut(n50("16:00:01", "1"), 9)], false, 2),
        ];
        for (rows, final_newline, line) in cases {
            let bytes = synthetic_file(HEADER_OPEN_INTEREST, &rows, final_newline);
            assert_eq!(
                decode(&bytes, &nifty(day(2024, 4, 1))),
                Err(CmRefusal::MalformedRow { line }),
                "{rows:?} final newline {final_newline}"
            );
        }
    }

    /// A short post-session last row followed by a bare LF is not a cut row.
    /// `short_last_row_refuses` builds its rows with `synthetic_file`, which
    /// ends them in CRLF, so there the CR alone refuses; this pins the
    /// newline condition itself with LF line ends (round-3 tests-bite review).
    #[test]
    fn a_short_post_session_last_row_followed_by_a_bare_lf_refuses() {
        let cut: String = n50("16:00:01", "1")
            .split(',')
            .take(9)
            .collect::<Vec<_>>()
            .join(",");
        let mut lf = [
            HEADER_OPEN_INTEREST.to_owned(),
            n50("09:15:00", "1"),
            n50("16:00:00", "1"),
            cut,
        ]
        .join("\n");
        lf.push('\n');
        assert_eq!(
            decode(lf.as_bytes(), &nifty(day(2024, 4, 1))),
            Err(CmRefusal::MalformedRow { line: 4 })
        );
    }

    /// A cut is a PREFIX of a row: every field after the Time that a comma
    /// closed has its column's whole shape, and the field the cut runs into is
    /// a prefix of one. A short post-session last row whose fields are not
    /// that is corruption, not truncation, and refuses rather than being
    /// recorded as a cut (round-3 review; design §3.3 states the rule by field
    /// count alone, and this narrows what counts as a cut row).
    #[test]
    fn a_cut_post_session_row_must_be_a_prefix_of_a_row() {
        let head = n50("16:00:01", "1")
            .split(',')
            .take(3)
            .collect::<Vec<_>>()
            .join(",");
        let rows = |tail: &str| {
            synthetic_file(
                HEADER_OPEN_INTEREST,
                &[
                    n50("09:15:00", "10000"),
                    n50("16:00:00", "10001.5"),
                    format!("{head},{tail}"),
                ],
                false,
            )
        };
        for tail in [
            "abc,xyz",
            "1X",
            ".5",
            "1.2.3",
            "10002.,0",
            "10002.5,-1",
            "10002.5,0,0.5",
            "10002.5,0,0,0,0,0.5",
            "10002.5,0,0,0,0,7 ",
        ] {
            assert_eq!(
                decode(&rows(tail), &nifty(day(2024, 4, 1))),
                Err(CmRefusal::MalformedRow { line: 4 }),
                "{tail}"
            );
        }
        for tail in [
            "",
            "10000",
            "10002.",
            "10002.5,",
            "10002.5,0,",
            "10002.5,0,0,0.",
            "10002.5,0,0,0,0,0",
        ] {
            let file = decode(&rows(tail), &nifty(day(2024, 4, 1))).unwrap();
            assert!(file.tail_truncated_post_session, "{tail}");
            assert_eq!(file.rows.len(), 2, "{tail}");
        }
    }

    /// Design §3.3 (revision 4) drops only a cut last row of FEWER than ten
    /// fields. A last row cut just after its ninth comma has ten fields, the
    /// last one empty, so it is `MalformedRow` and refuses the day, while the
    /// same row cut one byte earlier is dropped. The asymmetry is the
    /// design's rule, kept and recorded in `docs/06-limits.md`.
    #[test]
    fn a_last_row_cut_after_its_ninth_comma_refuses() {
        let whole = n50("16:00:01", "10002.5");
        let ninth = whole.rfind(',').unwrap();
        for (keep, dropped) in [(ninth + 1, false), (ninth, true)] {
            let bytes = synthetic_file(
                HEADER_OPEN_INTEREST,
                &[
                    n50("09:15:00", "10000"),
                    n50("16:00:00", "10001.5"),
                    whole[..keep].to_owned(),
                ],
                false,
            );
            let got = decode(&bytes, &nifty(day(2024, 4, 1)));
            if dropped {
                assert!(got.unwrap().tail_truncated_post_session);
            } else {
                assert_eq!(got, Err(CmRefusal::MalformedRow { line: 4 }));
            }
        }
    }

    /// A last line that ends in a bare CR was terminated by the vendor: the
    /// CR is the first half of its CRLF, so the line is a complete short
    /// row, not a cut one, and CM-05 refuses it `MalformedRow` rather than
    /// dropping it and recording a cut (round-4 review). The same fields
    /// without the CR are still the droppable cut tail.
    #[test]
    fn a_short_last_row_ending_in_a_bare_cr_is_malformed_not_cut() {
        let cut = n50("16:00:01", "100")
            .split(',')
            .take(4)
            .collect::<Vec<_>>()
            .join(",");
        for (tail, dropped) in [(format!("{cut}\r"), false), (cut, true)] {
            let bytes = synthetic_file(
                HEADER_OPEN_INTEREST,
                &[n50("10:00:00", "100"), n50("16:00:00", "100"), tail.clone()],
                false,
            );
            let got = decode(&bytes, &nifty(day(2024, 4, 1)));
            if dropped {
                assert!(got.unwrap().tail_truncated_post_session, "{tail:?}");
            } else {
                assert_eq!(got, Err(CmRefusal::MalformedRow { line: 4 }), "{tail:?}");
            }
        }
    }

    #[test]
    fn a_cut_last_row_that_is_not_utf8_refuses() {
        let cut: String = n50("16:00:01", "1")
            .split(',')
            .take(9)
            .collect::<Vec<_>>()
            .join(",");
        let mut bytes = synthetic_file(
            HEADER_OPEN_INTEREST,
            &[n50("09:15:00", "1"), n50("16:00:00", "1"), cut],
            false,
        );
        let at = bytes.len() - 2;
        bytes[at] = 0xff;
        assert_eq!(
            decode(&bytes, &nifty(day(2024, 4, 1))),
            Err(CmRefusal::MalformedRow { line: 4 }),
            "a cut row that is not UTF-8"
        );
    }

    /// An index file that stops inside the session was cut (design §3.3,
    /// D-0812): on every regular day of the copy, both index files print at
    /// or past 15:31:59 (charter, GDFL section).
    #[test]
    fn index_file_ending_inside_the_session_refuses() {
        for (last, at) in [("15:29:59", sod(15, 29, 59)), ("09:15:00", sod(9, 15, 0))] {
            let bytes = modern(&[
                n50("09:07:00", "10000"),
                n50("09:15:00", "10000"),
                n50(last, "10000"),
            ]);
            assert_eq!(
                decode(&bytes, &nifty(day(2024, 4, 1))),
                Err(CmRefusal::IndexEndsInSession { max_sod: at }),
                "{last}"
            );
        }
        let reaches = modern(&[n50("09:15:00", "10000"), n50("15:30:00", "10000")]);
        assert!(
            decode(&reaches, &nifty(day(2024, 4, 1))).is_ok(),
            "15:30:00 is past the session"
        );
    }

    /// Design §3.3, revision 9 (C45): the end check reads the file's largest
    /// stamp, the stamp of its last in-order row, which a late row never
    /// raises; it never reads the last row in file order. The design names
    /// this test `..._largest_effective_stamp_...`; the reader judges the
    /// stamp AS WRITTEN and leaves the UTC re-stamp to the fold (D-0808), so
    /// the name says what is checked (round-3 review).
    #[test]
    fn index_end_check_reads_the_largest_written_stamp_not_the_last_row() {
        let at = nifty(day(2024, 4, 1));
        // A late row last in the file, stamped inside the session, behind an
        // in-order row past it: the largest stamp decides, so it is read.
        let late_last = modern(&[
            n50("09:15:00", "10000"),
            n50("16:00:00", "10001.5"),
            n50("15:29:58", "10002.5"),
        ]);
        let file = decode(&late_last, &at).unwrap();
        assert_eq!(file.max_sod, sod(16, 0, 0));
        assert_eq!(file.rows.last().map(|r| r.sod), Some(sod(15, 29, 58)));
        // The largest stamp anywhere in the file, not only at either end.
        let middle = modern(&[
            n50("09:15:00", "10000"),
            n50("15:31:00", "10001.5"),
            n50("15:29:00", "10002.5"),
            n50("15:29:59", "10003.5"),
        ]);
        assert_eq!(decode(&middle, &at).unwrap().max_sod, sod(15, 31, 0));
        // A forward spike at or after 15:30:00 with no in-order row after it
        // passes: its own stamp is the largest (design §3.3).
        let spike = modern(&[
            n50("09:15:00", "10000"),
            n50("15:20:00", "10001.5"),
            n50("15:30:00", "10002.5"),
            n50("15:20:01", "10003.5"),
        ]);
        assert_eq!(decode(&spike, &at).unwrap().max_sod, sod(15, 30, 0));
        // A stock file records its largest stamp too, and is never refused
        // for it.
        let stock = modern(&[rel("09:16:23", "731.4", 5), rel("15:10:00", "731.6", 5)]);
        assert_eq!(decode(&stock, &reliance()).unwrap().max_sod, sod(15, 10, 0));
    }

    /// Design §3.3, revision 9 (C45): a file whose last in-order row is past
    /// the session and whose last rows are late rows stamped inside it is
    /// decoded whole, every row in file order, for the fold to place; it is
    /// not refused `IndexEndsInSession`.
    #[test]
    fn file_ending_on_in_session_late_rows_is_gridded_not_refused_index_ends_in_session() {
        let bytes = modern(&[
            n50("09:15:00", "10000"),
            n50("12:00:00", "10001.5"),
            n50("16:05:00", "10002.5"),
            n50("11:00:00", "10003.5"),
            n50("11:00:01", "10004.5"),
        ]);
        let file = decode(&bytes, &nifty(day(2024, 4, 1))).unwrap();
        assert_eq!(file.max_sod, sod(16, 5, 0));
        let stamps: Vec<u32> = file.rows.iter().map(|r| r.sod).collect();
        assert_eq!(
            stamps,
            [
                sod(9, 15, 0),
                sod(12, 0, 0),
                sod(16, 5, 0),
                sod(11, 0, 0),
                sod(11, 0, 1)
            ],
            "every row, in file order, none moved"
        );
        assert_eq!((file.rows_in_session, file.rows_post), (4, 1));
    }

    /// Design §3.3, revision 9 (C45): an index file whose every stamp, late or
    /// not, is inside the session refuses whole, naming its largest stamp,
    /// however its rows are ordered.
    #[test]
    fn index_file_whose_largest_stamp_is_in_session_refuses_whole() {
        let bytes = modern(&[
            n50("09:15:00", "10000"),
            n50("15:29:59", "10001.5"),
            n50("15:29:40", "10002.5"),
            n50("10:00:00", "10003.5"),
        ]);
        assert_eq!(
            decode(&bytes, &nifty(day(2024, 4, 1))),
            Err(CmRefusal::IndexEndsInSession {
                max_sod: sod(15, 29, 59)
            })
        );
    }

    /// Design §3.3 row checks, revision 11 (C55): the reader adds no
    /// plausibility check on an LTP's value. An isolated spike decodes as
    /// written; the fill policy flags it at load from the cells.
    #[test]
    fn an_isolated_spike_is_decoded_as_written_never_refused_here() {
        let bytes = modern(&[
            n50("09:15:00", "10000"),
            n50("09:15:01", "100000.05"),
            n50("09:15:02", "10000.05"),
            n50("16:00:00", "10000"),
        ]);
        let file = decode(&bytes, &nifty(day(2024, 4, 1))).unwrap();
        let ltp: Vec<i64> = file.rows.iter().map(|r| r.ltp).collect();
        assert_eq!(ltp, [1_000_000, 10_000_005, 1_000_005, 1_000_000]);
    }

    /// A stock can stop printing before 15:30, so its last stamp proves
    /// nothing; the archive check is its only completeness evidence.
    #[test]
    fn stock_file_ending_before_1530_is_not_refused_for_its_end() {
        let bytes = modern(&[rel("09:16:23", "731.4", 500), rel("15:10:00", "731.6", 3)]);
        let file = decode(&bytes, &reliance()).unwrap();
        assert_eq!(file.rows.last().unwrap().sod, sod(15, 10, 0));
    }

    #[test]
    fn stock_rows_keep_their_quantity_for_the_fold_to_filter() {
        let file = decode(&stock_day(), &reliance()).unwrap();
        let ltq: Vec<u64> = file.rows.iter().map(|r| r.ltq).collect();
        assert_eq!(ltq, vec![500, 0, 0, 120, 0, 0, 0]);
        assert_eq!(
            file.rows[2].ltp, 72_500,
            "kept although it prints below the bid"
        );
        assert_eq!(file.ltq_zero_rows, 5);
        assert_eq!(
            (file.rows_pre, file.rows_in_session, file.rows_post),
            (0, 5, 2)
        );
    }

    /// The measured shape of a replay: the stamps jump over a 13-second hole
    /// and 12 rows stamped inside the hole follow. The reader keeps that
    /// order; placing the late rows is the fold's rule (D-0805, pending).
    #[test]
    fn late_rows_are_kept_in_file_order_and_never_refused() {
        let mut rows = vec![n50("11:41:20", "10000"), n50("11:41:34", "10010.5")];
        for second in 21..=32 {
            rows.push(n50(&format!("11:41:{second:02}"), "10005.5"));
        }
        rows.push(n50("11:41:35", "10011.5"));
        rows.push(n50("16:00:00", "10011.5"));
        let file = decode(&modern(&rows), &nifty(day(2024, 4, 1))).unwrap();
        let stamps: Vec<u32> = file.rows.iter().map(|r| r.sod - sod(11, 41, 0)).collect();
        assert_eq!(
            stamps,
            vec![
                20, 34, 21, 22, 23, 24, 25, 26, 27, 28, 29, 30, 31, 32, 35, 15_540
            ]
        );
        assert_eq!(file.rows.len(), 16, "12 replayed rows, none dropped");
    }

    /// Design §3.3 (Row checks): the LTP is decoded with `csv::paisa`, and
    /// more than two decimals REFUSES. A third digit is the vendor sending
    /// something this build does not understand, which is `csv::paisa`'s
    /// first reason and applies here as much as to the F&O reader (round-4
    /// review). INDIA VIX is never mapped, but it is the one series that
    /// prints a third decimal (it prints four), so its shape is refused too.
    /// Every price here is invented.
    #[test]
    fn a_third_decimal_is_refused_not_snapped() {
        let vix = |time, ltp| index_row("INDIA VIX.NSE_IDX", "01/04/2024", time, ltp);
        let bytes = modern(&[vix("09:15:01", "13.4175"), vix("16:00:00", "13.40")]);
        let expect = Expect {
            kind: CmKind::Indices,
            stem: "INDIA VIX.NSE_IDX",
            day: day(2024, 4, 1),
        };
        assert_eq!(
            decode(&bytes, &expect),
            Err(CmRefusal::PriceRefused { line: 2 })
        );
        // A third digit refuses whatever it is, a zero included: the rule is
        // the column's shape, not whether a snap would change the value.
        for text in [
            "10000.055",
            "10000.0549",
            "10000.050",
            "1.000",
            "1.00000",
            "0.045",
        ] {
            assert_eq!(
                decode(&ltp_row(text), &nifty(day(2024, 4, 1))),
                Err(CmRefusal::PriceRefused { line: 3 }),
                "{text}"
            );
        }
        // Two places or fewer convert exactly; one place is tenths.
        for (text, paisa) in [
            ("0.05", 5),
            ("7.1", 710),
            ("10000", 1_000_000),
            ("10000.05", 1_000_005),
        ] {
            let file = decode(&ltp_row(text), &nifty(day(2024, 4, 1))).unwrap();
            assert_eq!(file.rows[1].ltp, paisa, "{text}");
        }
    }

    #[test]
    fn header_unknown_refuses() {
        let oi = HEADER_OPEN_INTEREST.replace("OpenInterest", "OI");
        let padded = format!("{HEADER_OPEN_INTEREST} ");
        let row_first = n50("09:15:00", "10000");
        for bytes in [
            b"".as_slice(),
            b"\n",
            oi.as_bytes(),
            padded.as_bytes(),
            row_first.as_bytes(),
        ] {
            assert_eq!(
                decode(bytes, &nifty(day(2024, 4, 1))),
                Err(CmRefusal::HeaderUnknown),
                "{bytes:?}"
            );
        }
    }

    /// A NIFTY BANK row inside a NIFTY 50 file.
    #[test]
    fn ticker_mismatch_refuses() {
        let bytes = one_bad_row(&index_row(
            "NIFTY BANK.NSE_IDX",
            "01/04/2024",
            "09:15:00",
            "20000.5",
        ));
        assert_eq!(
            decode(&bytes, &nifty(day(2024, 4, 1))),
            Err(CmRefusal::TickerMismatch { line: 3 })
        );
    }

    #[test]
    fn date_mismatch_refuses() {
        for date in [
            "02/04/2024",
            "1/04/2024",
            "01.04.2024",
            "2024-04-01",
            "04/01/2024",
        ] {
            let bytes = one_bad_row(&index_row("NIFTY 50.NSE_IDX", date, "09:15:01", "10000.05"));
            assert_eq!(
                decode(&bytes, &nifty(day(2024, 4, 1))),
                Err(CmRefusal::DateMismatch { line: 3 }),
                "{date}"
            );
        }
    }

    #[test]
    fn malformed_row_refuses() {
        let field = |at: usize, text: &str| {
            let mut fields = [
                "NIFTY 50.NSE_IDX",
                "01/04/2024",
                "09:15:01",
                "10000.05",
                "0",
                "0",
                "0",
                "0",
                "0",
                "0",
            ];
            fields[at] = text;
            synthetic_row(fields)
        };
        let whole = field(9, "0");
        let nine = whole.rsplit_once(',').unwrap().0.to_owned();
        let eleven = format!("{whole},0");
        let too_long = "9".repeat(20);
        let mut rows = vec![String::new(), nine, eleven];
        for time in [
            "9:15:01", "24:00:00", "09:60:00", "09:15:60", "09.15.01", "0x:15:01",
        ] {
            rows.push(field(2, time));
        }
        for (at, text) in [
            (4, "-1"),
            (5, "1.5"),
            (6, "x"),
            (7, ""),
            (7, "1.5"),
            (8, "+5"),
            (8, too_long.as_str()),
            (9, ""),
            (9, "-3"),
            (9, "1.5"),
        ] {
            rows.push(field(at, text));
        }
        rows.push(field(3, "10000,05"));
        for row in rows {
            assert_eq!(
                decode(&one_bad_row(&row), &nifty(day(2024, 4, 1))),
                Err(CmRefusal::MalformedRow { line: 3 }),
                "{row:?}"
            );
        }
        let mut bytes = one_bad_row(&field(9, "0"));
        let text = String::from_utf8(bytes.clone()).unwrap();
        let at = text.find("10000.05").unwrap();
        bytes[at] = 0xff;
        assert_eq!(
            decode(&bytes, &nifty(day(2024, 4, 1))),
            Err(CmRefusal::MalformedRow { line: 3 }),
            "a byte that is not UTF-8"
        );
    }

    #[test]
    fn the_last_second_of_the_day_is_a_time_and_the_next_is_not() {
        let bytes = modern(&[n50("09:15:00", "1"), n50("23:59:59", "1")]);
        let file = decode(&bytes, &nifty(day(2024, 4, 1))).unwrap();
        assert_eq!(file.rows[1].sod, 86_399);
    }

    #[test]
    fn price_refused_refuses() {
        // Twenty nines: a decimal that does not fit `i64` paisa.
        let too_big = "9".repeat(20);
        for ltp in [
            "-10000", "+10000", "10000.", ".5", "1E3", "0.04", "0", "", " 10000", &too_big,
        ] {
            assert_eq!(
                decode(&ltp_row(ltp), &nifty(day(2024, 4, 1))),
                Err(CmRefusal::PriceRefused { line: 3 }),
                "{ltp:?}"
            );
        }
    }

    #[test]
    fn rows_over_cap_refuses_and_the_cap_itself_is_allowed() {
        let expect = nifty(day(2024, 4, 1));
        assert_eq!(
            decode_capped(&index_day(), &expect, 10).map(|f| f.rows.len()),
            Ok(10)
        );
        assert_eq!(
            decode_capped(&index_day(), &expect, 9),
            Err(CmRefusal::RowsOverCap { cap: 9 })
        );
    }

    /// The cut-tail rule is applied before the row bound (round-5 review): a
    /// dropped cut tail is not a row, so a file of exactly `cap` complete rows
    /// and a cut post-session tail is accepted with the drop recorded, as the
    /// same file is under any larger bound. One complete row over the bound
    /// still refuses.
    #[test]
    fn a_cut_tail_is_dropped_before_the_row_bound_is_applied() {
        let expect = nifty(day(2024, 4, 1));
        let cut = n50("16:00:01", "1")
            .split(',')
            .take(4)
            .collect::<Vec<_>>()
            .join(",");
        let bytes = synthetic_file(
            HEADER_OPEN_INTEREST,
            &[n50("09:15:00", "100"), n50("16:00:00", "100"), cut],
            false,
        );
        for cap in [2, 3] {
            let got = decode_capped(&bytes, &expect, cap).unwrap();
            assert_eq!(
                (got.rows.len(), got.tail_truncated_post_session),
                (2, true),
                "cap {cap}"
            );
        }
        assert_eq!(
            decode_capped(&bytes, &expect, 1),
            Err(CmRefusal::RowsOverCap { cap: 1 })
        );
    }

    /// A last line that ends in a bare CR with no LF after it is a file cut
    /// between the two bytes of a CRLF: every line the vendor terminated ends
    /// CRLF (charter, "Line endings"), and its measured unterminated last lines
    /// end in a digit (D-0808). Such a file is refused `MalformedRow` naming
    /// that line, even when the line is a complete ten-field row, so the cut
    /// is never passed on as a clean file without a final newline (round-5
    /// review). The same row without the CR is accepted.
    #[test]
    fn a_complete_last_row_ending_in_a_bare_cr_is_malformed() {
        let rows = [n50("10:00:00", "100"), n50("16:00:00", "100")];
        let clean = synthetic_file(HEADER_OPEN_INTEREST, &rows, false);
        let got = decode(&clean, &nifty(day(2024, 4, 1))).unwrap();
        assert_eq!((got.rows.len(), got.final_newline), (2, false));
        let mut cut = clean;
        cut.push(b'\r');
        assert_eq!(
            decode(&cut, &nifty(day(2024, 4, 1))),
            Err(CmRefusal::MalformedRow { line: 3 })
        );
    }

    /// Exactly one CR is stripped from a line, which is stricter than
    /// `pull::csv`, whose splitter trims every trailing CR and then
    /// whitespace. Every measured GDFL line ends in one CRLF or, unterminated,
    /// in a digit (charter, "Line endings"), so a second CR or a trailing
    /// blank is a shape the vendor was never seen to write, refused by name
    /// rather than trimmed (round-5 review, D-0808).
    #[test]
    fn only_one_cr_is_stripped_and_nothing_else_is_trimmed() {
        let expect = nifty(day(2024, 4, 1));
        let row = |tail: &str| {
            let mut bytes = synthetic_file(HEADER_OPEN_INTEREST, &[n50("10:00:00", "100")], false);
            bytes.extend_from_slice(tail.as_bytes());
            bytes.extend_from_slice(b"\r\n");
            bytes.extend_from_slice(n50("16:00:00", "100").as_bytes());
            bytes.extend_from_slice(b"\r\n");
            bytes
        };
        assert_eq!(decode(&row(""), &expect).map(|f| f.rows.len()), Ok(2));
        for tail in ["\r", " ", "\t", " \r"] {
            assert_eq!(
                decode(&row(tail), &expect),
                Err(CmRefusal::MalformedRow { line: 2 }),
                "{tail:?}"
            );
        }
        let header = format!("{HEADER_OPEN_INTEREST}\r\r\n{}\r\n", n50("16:00:00", "100"));
        assert_eq!(
            decode(header.as_bytes(), &expect),
            Err(CmRefusal::HeaderUnknown)
        );
    }

    #[test]
    fn header_only_file_is_not_gridded() {
        assert_eq!(
            decode(&modern(&[]), &nifty(day(2024, 4, 1))),
            Err(CmRefusal::NoSessionRows)
        );
    }

    #[test]
    fn pre_post_only_file_is_not_gridded() {
        let bytes = modern(&[n50("09:07:27", "10000"), n50("15:30:00", "10020.55")]);
        assert_eq!(
            decode(&bytes, &nifty(day(2024, 4, 1))),
            Err(CmRefusal::NoSessionRows)
        );
    }

    #[test]
    fn a_stock_file_with_only_ltq_zero_session_rows_is_not_gridded() {
        let bytes = modern(&[
            rel("09:16:23", "731.4", 0),
            rel("09:16:23", "725.0", 0),
            rel("09:16:40", "732.0", 0),
            rel("15:30:00", "729.85", 0),
        ]);
        assert_eq!(decode(&bytes, &reliance()), Err(CmRefusal::NoSessionRows));
    }

    // ── no vendor row is ever committed ───────────────────────────────────

    /// Whether `text` holds a whole GDFL capital-market row after a ticker's
    /// `.NSE` or `.NSE_IDX` suffix: the comma, a `DD/MM/YYYY` date, an
    /// `HH:MM:SS` time and seven numeric fields.
    fn holds_cm_row(text: &str) -> bool {
        let bytes = text.as_bytes();
        let mut from = 0;
        while let Some(found) = text.get(from..).and_then(|rest| rest.find(".NSE")) {
            let mut at = from + found + ".NSE".len();
            if bytes.get(at..at + 4) == Some(b"_IDX".as_slice()) {
                at += 4;
            }
            if row_shape_at(bytes, at) {
                return true;
            }
            from = from + found + 1;
        }
        false
    }

    /// `,DD/MM/YYYY,HH:MM:SS` then seven comma-led numeric fields at `at`.
    fn row_shape_at(bytes: &[u8], mut at: usize) -> bool {
        for shape in [",dd/dd/dddd", ",dd:dd:dd"] {
            for want in shape.bytes() {
                let ok = bytes.get(at).is_some_and(|&b| match want {
                    b'd' => b.is_ascii_digit(),
                    other => b == other,
                });
                if !ok {
                    return false;
                }
                at += 1;
            }
        }
        for _ in 0..7 {
            if bytes.get(at) != Some(&b',') {
                return false;
            }
            at += 1;
            let start = at;
            while bytes
                .get(at)
                .is_some_and(|b| b.is_ascii_digit() || *b == b'.')
            {
                at += 1;
            }
            if at == start {
                return false;
            }
        }
        true
    }

    fn rust_files(dir: &Path, out: &mut Vec<std::path::PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                rust_files(&path, out);
            } else if path.to_string_lossy().ends_with(".rs") {
                out.push(path);
            }
        }
    }

    /// Design §12 (revision 5): a test fixture in a public repository is
    /// redistribution, so a capital-market row may enter `crates/` only as
    /// values a builder joins at run time, never as one literal.
    #[test]
    fn fixtures_are_built_not_pasted() {
        let pasted = n50("09:15:00", "10000");
        assert!(
            holds_cm_row(&format!("x {pasted}\r\n")),
            "the detector sees a row"
        );
        assert!(holds_cm_row(&rel("09:16:23", "731.4", 500)));
        assert!(!holds_cm_row(&pasted.replacen(",01/", ",x1/", 1)));
        assert!(
            !holds_cm_row(&pasted.replacen(",0,0", ",x,0", 1)),
            "a field that is not a number"
        );
        assert!(!holds_cm_row(&pasted.replace("09:15:00", "09.15.00")));
        assert!(!holds_cm_row(&pasted.replace(".NSE_IDX", ".NFO")));
        assert!(
            !holds_cm_row(pasted.rsplit_once(',').unwrap().0),
            "nine fields are not a row"
        );
        let crates = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut files = Vec::new();
        rust_files(crates, &mut files);
        assert!(
            files.iter().any(|f| f.ends_with("pull/src/gdfl_cm.rs")),
            "the walk reaches this file"
        );
        let pasted: Vec<&std::path::PathBuf> = files
            .iter()
            .filter(|file| holds_cm_row(&std::fs::read_to_string(file).unwrap()))
            .collect();
        assert!(pasted.is_empty(), "pasted GDFL CM rows in {pasted:?}");
    }

    // ── the calendar ───────────────────────────────────────────────────────

    #[test]
    fn a_regular_day_passes_the_session_gate() {
        assert_eq!(session_gate(day(2024, 4, 1)), Ok(()));
    }

    #[test]
    fn closed_day_is_not_gridded() {
        assert!(matches!(
            session_gate(day(2024, 4, 6)),
            Err(CmRefusal::CalendarNotRegular {
                kind: DayKind::Closed
            })
        ));
    }

    /// 2024-03-02 and 2024-05-18, the disaster-recovery Saturdays with two
    /// windows. Their four index files are the only ones of the copy whose
    /// largest stamp is before 15:30:00 (charter, GDFL section), so this
    /// gate, not `IndexEndsInSession`, is what `read_day` refuses them with.
    #[test]
    fn special_session_is_not_gridded() {
        for drill in [day(2024, 3, 2), day(2024, 5, 18)] {
            let got = session_gate(drill);
            assert!(matches!(
                got,
                Err(CmRefusal::CalendarNotRegular {
                    kind: DayKind::Open(_)
                })
            ));
            assert_ne!(
                got,
                Err(CmRefusal::CalendarNotRegular {
                    kind: DayKind::Open(Session::full())
                }),
                "{drill:?}"
            );
        }
    }

    /// 2024-11-01, a Muhurat day with no minute series (`calendar.rs`).
    #[test]
    fn length_unmeasured_day_is_not_gridded() {
        assert_eq!(
            session_gate(day(2024, 11, 1)),
            Err(CmRefusal::CalendarNotRegular {
                kind: DayKind::OpenLengthUnmeasured
            })
        );
    }

    #[test]
    fn unmeasured_day_refuses() {
        assert_eq!(
            session_gate(day(2018, 9, 3)),
            Err(CmRefusal::CalendarUnmeasured)
        );
        assert_eq!(
            session_gate(day(2026, 9, 24)),
            Err(CmRefusal::CalendarUnmeasured)
        );
    }

    // ── one instrument-day, end to end, through a source ──────────────────

    /// One file an in-memory source holds: its entry name, its bytes, and the
    /// length and CRC-32 its listing states.
    struct Held {
        entry: String,
        bytes: Vec<u8>,
        len: u64,
        crc32: u32,
    }

    /// An in-memory [`CmSource`]: every day it holds, with its files in
    /// order. A file's stated length and CRC-32 are its bytes' own unless a
    /// test overrides them, and `fetches` counts the reads.
    struct MemSource {
        days: Vec<(CmKind, Day, Vec<Held>)>,
        fetches: std::cell::Cell<usize>,
    }

    impl MemSource {
        fn new() -> Self {
            Self {
                days: Vec::new(),
                fetches: std::cell::Cell::new(0),
            }
        }

        /// Adds `files` (names inside the day folder) as the day of `kind`.
        fn with(mut self, kind: CmKind, at: Day, files: &[(&str, Vec<u8>)]) -> Self {
            let folder = day_folder_name(kind, at);
            let held = files
                .iter()
                .map(|(name, bytes)| Held {
                    entry: format!("{folder}/{name}"),
                    len: bytes.len() as u64,
                    crc32: crate::gdfl_archive::crc32(bytes),
                    bytes: bytes.clone(),
                })
                .collect();
            self.days.push((kind, at, held));
            self
        }

        fn files(&self, kind: CmKind, at: Day) -> Option<&Vec<Held>> {
            self.days
                .iter()
                .filter(|(k, d, _)| *k == kind && *d == at)
                .map(|(_, _, files)| files)
                .next()
        }
    }

    impl CmSource for MemSource {
        type Locator = usize;

        fn day(&self, kind: CmKind, at: Day) -> Result<Option<DayListing<usize>>, CmRefusal> {
            Ok(self.files(kind, at).map(|files| {
                let mut listing = DayListing::new(kind, at);
                for (i, file) in files.iter().enumerate() {
                    listing.push(&file.entry, file.len, file.crc32, i);
                }
                listing
            }))
        }

        fn fetch(
            &self,
            listing: &DayListing<usize>,
            file: &ListedFile<usize>,
        ) -> Result<Vec<u8>, CmRefusal> {
            self.fetches.set(self.fetches.get() + 1);
            Ok(
                self.files(listing.kind(), listing.day()).unwrap()[file.locator]
                    .bytes
                    .clone(),
            )
        }
    }

    fn nifty_key() -> InstrumentKey {
        InstrumentKey::index(Exchange::Nse, "NIFTY").unwrap()
    }

    #[test]
    fn read_day_reads_the_one_file_its_day_names() {
        let at = day(2024, 4, 1);
        let source = MemSource::new().with(
            CmKind::Indices,
            at,
            &[("NIFTY 50.NSE_IDX.csv", index_day())],
        );
        let read = read_day(&source, &nifty_key(), at).unwrap().unwrap();
        assert_eq!(read.name, "NIFTY 50.NSE_IDX.csv");
        assert_eq!(read.ext, ExtVariant::Lower);
        assert_eq!(read.file, decode(&index_day(), &nifty(at)).unwrap());
        let bank = InstrumentKey::index(Exchange::Nse, "BANKNIFTY").unwrap();
        assert_eq!(
            read_day(&source, &bank, at),
            Ok(None),
            "absent, not refused"
        );
        assert_eq!(
            read_day(&source, &nifty_key(), day(2024, 4, 2)),
            Ok(None),
            "a day the source does not hold"
        );
        let reliance = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
        assert_eq!(
            read_day(&source, &reliance, at),
            Ok(None),
            "the source holds no stock day"
        );
        assert_eq!(
            source.fetches.get(),
            1,
            "only the file that exists is fetched"
        );
    }

    /// CM-14 end to end: a share's file is listed from a stock day and read
    /// with the share's key.
    #[test]
    fn read_day_reads_a_share_from_its_stock_day() {
        let at = day(2024, 4, 1);
        let source =
            MemSource::new().with(CmKind::Stocks, at, &[("RELIANCE.NSE.csv", stock_day())]);
        let key = InstrumentKey::cash(Exchange::Nse, "RELIANCE").unwrap();
        let read = read_day(&source, &key, at).unwrap().unwrap();
        assert_eq!(read.name, "RELIANCE.NSE.csv");
        assert_eq!(read.file, decode(&stock_day(), &reliance()).unwrap());
    }

    #[test]
    fn read_day_refuses_a_key_off_the_surface() {
        let source = MemSource::new();
        let off = InstrumentKey::index(Exchange::Nse, "FINNIFTY").unwrap();
        assert_eq!(
            read_day(&source, &off, day(2024, 4, 1)),
            Err(CmRefusal::NotSwept)
        );
        assert_eq!(
            read_listed(
                &source,
                &DayListing::<usize>::new(CmKind::Indices, day(2024, 4, 1)),
                &off
            ),
            Err(CmRefusal::NotSwept)
        );
    }

    /// A listing of the other tree is not the instrument's day: a stock
    /// listing resolves no index key (design §3.3, revision 9).
    #[test]
    fn read_listed_refuses_a_listing_of_the_other_tree() {
        let at = day(2024, 4, 1);
        let source =
            MemSource::new().with(CmKind::Stocks, at, &[("NIFTY 50.NSE_IDX.csv", index_day())]);
        let stocks = source.day(CmKind::Stocks, at).unwrap().unwrap();
        assert_eq!(
            read_listed(&source, &stocks, &nifty_key()),
            Err(CmRefusal::FolderUnknown)
        );
        assert_eq!(source.fetches.get(), 0);
    }

    #[test]
    fn read_day_gates_the_calendar_before_reading() {
        let at = day(2018, 9, 3);
        let source = MemSource::new().with(
            CmKind::Indices,
            at,
            &[("NIFTY 50.NSE_IDX.CSV", spaced_day_without_final_newline())],
        );
        assert_eq!(
            read_day(&source, &nifty_key(), at),
            Err(CmRefusal::CalendarUnmeasured),
            "the 2018 days are outside the measured calendar and are not gridded"
        );
        let listing = source.day(CmKind::Indices, at).unwrap().unwrap();
        assert_eq!(
            read_listed(&source, &listing, &nifty_key()),
            Err(CmRefusal::CalendarUnmeasured)
        );
        assert_eq!(source.fetches.get(), 0, "gated before any fetch");
    }

    #[test]
    fn read_day_passes_on_a_listing_refusal() {
        let at = day(2024, 4, 1);
        let source = MemSource::new().with(
            CmKind::Indices,
            at,
            &[
                ("NIFTY 50.NSE_IDX.csv", index_day()),
                ("NIFTY 50.NSE_IDX.CSV", index_day()),
            ],
        );
        assert_eq!(
            read_day(&source, &nifty_key(), at),
            Err(CmRefusal::AmbiguousCaseTwins)
        );
    }

    /// A source's own refusal reaches the caller as it is.
    #[test]
    fn read_day_passes_on_a_source_refusal() {
        struct Broken;
        impl CmSource for Broken {
            type Locator = ();
            fn day(&self, _: CmKind, _: Day) -> Result<Option<DayListing<()>>, CmRefusal> {
                Err(CmRefusal::ArchiveUnavailable {
                    kind: std::io::ErrorKind::NotFound,
                })
            }
            fn fetch(&self, _: &DayListing<()>, _: &ListedFile<()>) -> Result<Vec<u8>, CmRefusal> {
                Err(CmRefusal::ArchiveMemberCorrupt {
                    member: "x".to_owned(),
                })
            }
        }
        let at = day(2024, 4, 1);
        assert_eq!(
            read_day(&Broken, &nifty_key(), at),
            Err(CmRefusal::ArchiveUnavailable {
                kind: std::io::ErrorKind::NotFound
            })
        );
        let mut listing = DayListing::new(CmKind::Indices, at);
        listing.push(
            "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv",
            1,
            0,
            (),
        );
        assert_eq!(
            read_listed(&Broken, &listing, &nifty_key()),
            Err(CmRefusal::ArchiveMemberCorrupt {
                member: "x".to_owned()
            })
        );
    }

    #[test]
    fn read_day_passes_on_a_decode_refusal() {
        let at = day(2024, 4, 1);
        let source = MemSource::new().with(
            CmKind::Indices,
            at,
            &[("NIFTY 50.NSE_IDX.csv", modern(&[]))],
        );
        assert_eq!(
            read_day(&source, &nifty_key(), at),
            Err(CmRefusal::NoSessionRows)
        );
    }

    /// Every fetched file is checked against the length and CRC-32 its
    /// listing states BEFORE it is decoded, whatever the source (D-0812,
    /// D-2800): a file cut at a row boundary decodes cleanly and is still
    /// refused by its length, a file cut inside a row is named as cut rather
    /// than as a malformed vendor row, and one changed digit is refused by
    /// its CRC-32.
    #[test]
    fn every_fetched_file_is_checked_before_it_is_decoded() {
        let at = day(2024, 4, 1);
        let whole = index_day();
        let text = String::from_utf8(whole.clone()).unwrap();
        let stated = |bytes: Vec<u8>| {
            let mut source =
                MemSource::new().with(CmKind::Indices, at, &[("NIFTY 50.NSE_IDX.csv", bytes)]);
            source.days[0].2[0].len = whole.len() as u64;
            source.days[0].2[0].crc32 = crate::gdfl_archive::crc32(&whole);
            read_day(&source, &nifty_key(), at)
        };
        assert!(stated(whole.clone()).unwrap().is_some());
        let at_row = text
            .trim_end()
            .rsplit_once("\r\n")
            .unwrap()
            .0
            .as_bytes()
            .to_vec();
        assert!(decode(&at_row, &nifty(at)).is_ok(), "the cut decodes");
        assert_eq!(
            stated(at_row.clone()),
            Err(CmRefusal::SourceLengthMismatch {
                file: at_row.len() as u64,
                member: whole.len() as u64,
            })
        );
        let in_row = whole[..text.find("09:15:01").unwrap() + 4].to_vec();
        assert_eq!(
            stated(in_row.clone()),
            Err(CmRefusal::SourceLengthMismatch {
                file: in_row.len() as u64,
                member: whole.len() as u64,
            })
        );
        let changed = text.replacen("10251.3", "10251.4", 1).into_bytes();
        assert_eq!(changed.len(), whole.len());
        assert_eq!(
            stated(changed.clone()),
            Err(CmRefusal::SourceCrcMismatch {
                file: crate::gdfl_archive::crc32(&changed),
                member: crate::gdfl_archive::crc32(&whole),
            })
        );
    }

    /// The listing keeps every entry in the source's order (a day zip's
    /// central-directory order, which the tick store also keeps), with the
    /// stated length and CRC-32; names outside the day folder and the folder
    /// entry are kept and counted, never filed (CM-13).
    #[test]
    fn a_listing_keeps_the_source_order_and_files_only_the_day_folder() {
        let at = day(2024, 4, 1);
        let mut listing = DayListing::new(CmKind::Indices, at);
        assert_eq!(listing.folder(), "GFDLCM_INDICES_TICK_01042024");
        let names = [
            "GFDLCM_INDICES_TICK_01042024/",
            "GFDLCM_INDICES_TICK_01042024/NIFTY BANK.NSE_IDX.csv",
            "NIFTY 50.NSE_IDX.csv",
            "GFDLCM_INDICES_TICK_02042024/NIFTY 50.NSE_IDX.csv",
            "GFDLCM_INDICES_TICK_01042024x/NIFTY 50.NSE_IDX.csv",
            "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv",
        ];
        for (i, name) in names.iter().enumerate() {
            listing.push(name, i as u64, 7, i);
        }
        assert_eq!((listing.kind(), listing.day()), (CmKind::Indices, at));
        let order: Vec<&str> = listing.entries().iter().map(|e| &*e.entry).collect();
        assert_eq!(order, names);
        assert_eq!(listing.unresolved(), 4);
        let (nifty, ext) = listing.locate("NIFTY 50.NSE_IDX").unwrap().unwrap();
        assert_eq!(
            (nifty.name(), nifty.len, nifty.crc32, nifty.locator, ext),
            ("NIFTY 50.NSE_IDX.csv", 5, 7, 5, ExtVariant::Lower)
        );
        assert_eq!(listing.entries()[2].name(), "NIFTY 50.NSE_IDX.csv");
        let stocks = DayListing::<()>::new(CmKind::Stocks, day(2024, 4, 2));
        assert_eq!(stocks.folder(), "GFDLCM_STOCK_TICK_02042024");
        assert_eq!(
            folder_day(&day_folder_name(CmKind::Stocks, day(2024, 4, 2))),
            Ok((CmKind::Stocks, day(2024, 4, 2)))
        );
    }

    #[test]
    fn every_refusal_says_what_it_refused() {
        for (refusal, words) in [
            (CmRefusal::FolderUnknown, "GFDLCM_INDICES_TICK_"),
            (CmRefusal::ExtensionUnknown, "neither csv nor CSV"),
            (
                CmRefusal::AmbiguousCaseTwins,
                "two names for one ticker, each spelled .csv or .CSV",
            ),
            (CmRefusal::HeaderUnknown, "header"),
            (CmRefusal::TickerMismatch { line: 7 }, "line 7: Ticker"),
            (CmRefusal::DateMismatch { line: 8 }, "line 8: Date"),
            (
                CmRefusal::MalformedRow { line: 9 },
                "line 9: row is malformed",
            ),
            (CmRefusal::PriceRefused { line: 10 }, "line 10: LTP"),
            (CmRefusal::RowsOverCap { cap: 11 }, "more than 11 rows"),
            (CmRefusal::NoSessionRows, "09:15:00 to 15:30:00"),
            (
                CmRefusal::IndexEndsInSession { max_sod: 55_799 },
                "ends inside the session, its largest stamp at second 55799",
            ),
            (
                CmRefusal::CalendarNotRegular {
                    kind: DayKind::Closed,
                },
                "says Closed",
            ),
            (CmRefusal::CalendarUnmeasured, "not measured"),
            (CmRefusal::NotSwept, "no GDFL CM stem"),
            (
                CmRefusal::ArchiveMethodUnknown {
                    member: "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv".to_owned(),
                    method: 12,
                },
                "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv is stored with method 12, \
                 neither stored nor deflated",
            ),
            (
                CmRefusal::ArchiveMemberCorrupt {
                    member: "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv".to_owned(),
                },
                "GFDLCM_INDICES_TICK_01042024/NIFTY 50.NSE_IDX.csv does not inflate",
            ),
            (
                CmRefusal::SourceLengthMismatch { file: 3, member: 4 },
                "the source returned 3 bytes, the archive states 4",
            ),
            (
                CmRefusal::SourceCrcMismatch { file: 1, member: 2 },
                "the source returned CRC-32 00000001, the archive states 00000002",
            ),
            (
                CmRefusal::PriceRefused { line: 3 },
                "at most two decimal places and at least one tick",
            ),
        ] {
            let text = refusal.to_string();
            assert!(text.contains(words), "{text}");
        }
    }
}
